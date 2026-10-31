//! Streaming response translation for Server-Sent Events and NDJSON.
//!
//! LLM APIs stream text as many small JSON events, so a fake such as `Wei Zhang` may
//! arrive as `"Wei"` in one event and `" Zhang"` in the next. Byte-level replacement
//! cannot see across the JSON framing, so this module parses each event, routes the
//! *delta text fields* of known formats (OpenAI chat + Responses, Anthropic, Gemini,
//! Cohere, Ollama-style NDJSON) through a per-channel [`StreamUnmasker`], and unmasks
//! every other string leaf directly. Held-back text is flushed into the event that ends
//! the channel or, if needed, into a synthesized copy of the channel's last event.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde_json::Value;
use shadow_core::{Session, Shadow, StreamUnmasker};

/// What translates a response back: the engine (fakes only) or one request's session
/// (fakes plus that request's tokens such as `[EMAIL_0001]`).
#[derive(Clone)]
pub enum Translator {
    Shadow(Shadow),
    Session(Arc<Session>),
}

impl From<Shadow> for Translator {
    fn from(s: Shadow) -> Self {
        Translator::Shadow(s)
    }
}

impl From<Arc<Session>> for Translator {
    fn from(s: Arc<Session>) -> Self {
        Translator::Session(s)
    }
}

impl Translator {
    pub fn unmask(&self, text: &str) -> String {
        match self {
            Translator::Shadow(s) => s.unmask(text),
            Translator::Session(s) => s.unmask(text),
        }
    }

    /// Like [`Translator::unmask`], also returning how many values were restored.
    pub fn unmask_counted(&self, text: &str) -> (String, usize) {
        match self {
            Translator::Shadow(s) => s.unmask_counted(text),
            Translator::Session(s) => s.unmask_counted(text),
        }
    }

    /// Unmask every string leaf in place; returns values restored.
    pub fn unmask_json(&self, v: &mut Value) -> usize {
        match self {
            Translator::Shadow(s) => s.unmask_json(v),
            Translator::Session(s) => s.unmask_json(v),
        }
    }

    pub fn stream_unmasker(&self) -> StreamUnmasker {
        match self {
            Translator::Shadow(s) => s.stream_unmasker(),
            Translator::Session(s) => s.stream_unmasker(),
        }
    }
}

/// How events are framed on the wire.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    /// `text/event-stream`: events separated by blank lines, payload in `data:` lines.
    Sse,
    /// NDJSON / JSON lines: one JSON document per line.
    Lines,
}

#[derive(Clone, Debug)]
enum Seg {
    Key(String),
    Idx(usize),
}

struct Channel {
    un: StreamUnmasker,
    template: Value,
    path: Vec<Seg>,
    fields: Vec<String>,
}

/// Incremental event-stream unmasker.
pub struct EventStreamUnmasker {
    shadow: Translator,
    framing: Framing,
    line_buf: Vec<u8>,
    fields: Vec<String>,
    data: Vec<String>,
    channels: BTreeMap<String, Channel>,
    /// Values restored by finished channels and direct unmasking.
    restored: usize,
}

fn get_mut<'a>(v: &'a mut Value, path: &[Seg]) -> Option<&'a mut Value> {
    let mut cur = v;
    for s in path {
        cur = match s {
            Seg::Key(k) => cur.get_mut(k.as_str())?,
            Seg::Idx(i) => cur.get_mut(*i)?,
        };
    }
    Some(cur)
}

fn k(s: &str) -> Seg {
    Seg::Key(s.to_string())
}

fn idx_of(v: &Value, fallback: usize) -> String {
    v.get("index")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(fallback)
        .to_string()
}

/// Delta text fields of an event: `(channel key, path)`.
fn deltas(v: &Value) -> Vec<(String, Vec<Seg>)> {
    let mut out = vec![];
    let ty = v.get("type").and_then(Value::as_str).unwrap_or("");
    if let Some(choices) = v.get("choices").and_then(Value::as_array) {
        for (i, c) in choices.iter().enumerate() {
            let ci = idx_of(c, i);
            if let Some(d) = c.get("delta") {
                for f in ["content", "reasoning_content", "reasoning", "refusal"] {
                    if d.get(f).is_some_and(Value::is_string) {
                        out.push((
                            format!("oa:{ci}:{f}"),
                            vec![k("choices"), Seg::Idx(i), k("delta"), k(f)],
                        ));
                    }
                }
                if let Some(tools) = d.get("tool_calls").and_then(Value::as_array) {
                    for (j, t) in tools.iter().enumerate() {
                        if t.pointer("/function/arguments")
                            .is_some_and(Value::is_string)
                        {
                            out.push((
                                format!("oa:{ci}:tool:{}", idx_of(t, j)),
                                vec![
                                    k("choices"),
                                    Seg::Idx(i),
                                    k("delta"),
                                    k("tool_calls"),
                                    Seg::Idx(j),
                                    k("function"),
                                    k("arguments"),
                                ],
                            ));
                        }
                    }
                }
            } else if c.get("text").is_some_and(Value::is_string) {
                out.push((
                    format!("oa:{ci}:text"),
                    vec![k("choices"), Seg::Idx(i), k("text")],
                ));
            }
        }
    }
    if ty == "content_block_delta" {
        let ix = idx_of(v, 0);
        for f in ["text", "partial_json", "thinking"] {
            if v.pointer(&format!("/delta/{f}"))
                .is_some_and(Value::is_string)
            {
                out.push((format!("an:{ix}:{f}"), vec![k("delta"), k(f)]));
            }
        }
    }
    if ty.starts_with("response.")
        && ty.ends_with(".delta")
        && v.get("delta").is_some_and(Value::is_string)
    {
        let item = v.get("item_id").and_then(Value::as_str).unwrap_or("");
        let ci = v.get("content_index").and_then(Value::as_u64).unwrap_or(0);
        out.push((format!("rs:{item}:{ci}:{ty}"), vec![k("delta")]));
    }
    if let Some(cands) = v.get("candidates").and_then(Value::as_array) {
        for (i, c) in cands.iter().enumerate() {
            if let Some(parts) = c.pointer("/content/parts").and_then(Value::as_array) {
                for (j, p) in parts.iter().enumerate() {
                    if p.get("text").is_some_and(Value::is_string) {
                        out.push((
                            format!("gm:{i}:{j}"),
                            vec![
                                k("candidates"),
                                Seg::Idx(i),
                                k("content"),
                                k("parts"),
                                Seg::Idx(j),
                                k("text"),
                            ],
                        ));
                    }
                }
            }
        }
    }
    if ty == "content-delta"
        && v.pointer("/delta/message/content/text")
            .is_some_and(Value::is_string)
    {
        out.push((
            format!("co:{}", idx_of(v, 0)),
            vec![k("delta"), k("message"), k("content"), k("text")],
        ));
    }
    if v.get("done").is_some() {
        if v.pointer("/message/content").is_some_and(Value::is_string) {
            out.push(("ol:content".into(), vec![k("message"), k("content")]));
        }
        if v.get("response").is_some_and(Value::is_string) {
            out.push(("ol:response".into(), vec![k("response")]));
        }
    }
    out
}

/// Channel-key prefixes that end with this event (`""` = all channels).
fn terminal(v: &Value) -> Vec<String> {
    let mut out = vec![];
    let ty = v.get("type").and_then(Value::as_str).unwrap_or("");
    if let Some(choices) = v.get("choices").and_then(Value::as_array) {
        for (i, c) in choices.iter().enumerate() {
            if c.get("finish_reason").is_some_and(|f| !f.is_null()) {
                out.push(format!("oa:{}:", idx_of(c, i)));
            }
        }
    }
    match ty {
        "content_block_stop" => out.push(format!("an:{}:", idx_of(v, 0))),
        "message_delta"
        | "message_stop"
        | "message-end"
        | "response.completed"
        | "response.failed"
        | "response.incomplete"
        | "error" => out.push(String::new()),
        "content-end" => out.push(format!("co:{}", idx_of(v, 0))),
        t if t.starts_with("response.") && t.ends_with(".done") => {
            let item = v.get("item_id").and_then(Value::as_str).unwrap_or("");
            out.push(format!("rs:{item}:"));
        }
        _ => {}
    }
    if let Some(cands) = v.get("candidates").and_then(Value::as_array) {
        for (i, c) in cands.iter().enumerate() {
            if c.get("finishReason").is_some_and(|f| !f.is_null()) {
                out.push(format!("gm:{i}:"));
            }
        }
    }
    if v.get("done").and_then(Value::as_bool) == Some(true) {
        out.push("ol:".into());
    }
    out
}

impl EventStreamUnmasker {
    /// New unmasker for one response stream.
    pub fn new(translator: impl Into<Translator>, framing: Framing) -> Self {
        Self {
            shadow: translator.into(),
            framing,
            line_buf: vec![],
            fields: vec![],
            data: vec![],
            channels: BTreeMap::new(),
            restored: 0,
        }
    }

    /// Values restored so far (counts only).
    pub fn restored(&self) -> usize {
        self.restored
            + self
                .channels
                .values()
                .map(|c| c.un.restored())
                .sum::<usize>()
    }

    /// Feed raw bytes; returns translated bytes ready to send.
    pub fn push(&mut self, bytes: &[u8]) -> Vec<u8> {
        self.line_buf.extend_from_slice(bytes);
        let mut out = String::new();
        while let Some(pos) = self.line_buf.iter().position(|b| *b == b'\n') {
            let line: Vec<u8> = self.line_buf.drain(..=pos).collect();
            let line = String::from_utf8_lossy(&line);
            let line = line.trim_end_matches('\n').trim_end_matches('\r');
            self.on_line(line, &mut out);
        }
        out.into_bytes()
    }

    /// End of stream: flush partial lines, pending events and held-back text.
    pub fn finish(&mut self) -> Vec<u8> {
        let mut out = String::new();
        if !self.line_buf.is_empty() {
            let rest = String::from_utf8_lossy(&std::mem::take(&mut self.line_buf)).to_string();
            self.on_line(rest.trim_end_matches('\r'), &mut out);
        }
        if !self.fields.is_empty() || !self.data.is_empty() {
            self.dispatch(&mut out);
        }
        self.flush_channels("", &mut out);
        out.into_bytes()
    }

    fn on_line(&mut self, line: &str, out: &mut String) {
        match self.framing {
            Framing::Lines => {
                if line.trim().is_empty() {
                    out.push_str(line);
                    out.push('\n');
                } else {
                    self.data.push(line.to_string());
                    self.dispatch(out);
                }
            }
            Framing::Sse => {
                if line.is_empty() {
                    self.dispatch(out);
                } else if let Some(d) = line.strip_prefix("data:") {
                    self.data.push(d.strip_prefix(' ').unwrap_or(d).to_string());
                } else {
                    self.fields.push(line.to_string());
                }
            }
        }
    }

    fn emit(&self, fields: &[String], data: &str, out: &mut String) {
        match self.framing {
            Framing::Lines => {
                out.push_str(data);
                out.push('\n');
            }
            Framing::Sse => {
                for f in fields {
                    out.push_str(f);
                    out.push('\n');
                }
                for l in data.split('\n') {
                    out.push_str("data: ");
                    out.push_str(l);
                    out.push('\n');
                }
                out.push('\n');
            }
        }
    }

    /// Flush channels whose key starts with `prefix`, synthesizing events for any tail.
    fn flush_channels(&mut self, prefix: &str, out: &mut String) {
        let keys: Vec<String> = self
            .channels
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        for key in keys {
            let Some(mut ch) = self.channels.remove(&key) else {
                continue;
            };
            let tail = ch.un.finish();
            self.restored += ch.un.restored();
            if tail.is_empty() {
                continue;
            }
            if let Some(slot) = get_mut(&mut ch.template, &ch.path) {
                *slot = Value::String(tail);
            }
            let data = ch.template.to_string();
            self.emit(&ch.fields, &data, out);
        }
    }

    fn dispatch(&mut self, out: &mut String) {
        let fields = std::mem::take(&mut self.fields);
        let data_lines = std::mem::take(&mut self.data);
        if data_lines.is_empty() {
            // comment / keep-alive / bare fields
            for f in &fields {
                out.push_str(f);
                out.push('\n');
            }
            out.push('\n');
            return;
        }
        let data = data_lines.join("\n");
        if data.trim() == "[DONE]" {
            self.flush_channels("", out);
            self.emit(&fields, &data, out);
            return;
        }
        let Ok(mut v) = serde_json::from_str::<Value>(&data) else {
            let (un, n) = self.shadow.unmask_counted(&data);
            self.restored += n;
            self.emit(&fields, &un, out);
            return;
        };
        let original = v.clone();
        let found = deltas(&v);
        let ends = terminal(&v);
        // Take delta texts out so the generic pass does not touch them.
        let mut texts = Vec::with_capacity(found.len());
        for (_, path) in &found {
            let t = get_mut(&mut v, path)
                .and_then(|s| s.as_str().map(str::to_string).map(|t| (t, s)))
                .map(|(t, s)| {
                    *s = Value::String(String::new());
                    t
                })
                .unwrap_or_default();
            texts.push(t);
        }
        self.restored += self.shadow.unmask_json(&mut v);
        let template = v.clone();
        for ((key, path), text) in found.iter().zip(texts) {
            let shadow = &self.shadow;
            let ch = self.channels.entry(key.clone()).or_insert_with(|| Channel {
                un: shadow.stream_unmasker(),
                template: Value::Null,
                path: path.clone(),
                fields: fields.clone(),
            });
            let mut emitted = ch.un.push(&text);
            if ends.iter().any(|p| key.starts_with(p.as_str())) {
                emitted.push_str(&ch.un.finish());
            }
            ch.template = template.clone();
            ch.path = path.clone();
            ch.fields = fields.clone();
            if let Some(slot) = get_mut(&mut v, path) {
                *slot = Value::String(emitted);
            }
        }
        // Channels ending here without text in this event get a synthesized flush first.
        for p in &ends {
            let pending: Vec<String> = self
                .channels
                .keys()
                .filter(|k| k.starts_with(p.as_str()) && !found.iter().any(|(fk, _)| fk == *k))
                .cloned()
                .collect();
            for key in pending {
                self.flush_channels(&key, out);
            }
        }
        for (key, _) in &found {
            if ends.iter().any(|p| key.starts_with(p.as_str())) {
                if let Some(ch) = self.channels.remove(key) {
                    self.restored += ch.un.restored();
                }
            }
        }
        if v == original {
            self.emit(&fields, &data, out);
        } else {
            self.emit(&fields, &v.to_string(), out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shadow_core::ShadowConfig;

    fn shadow() -> (tempfile::TempDir, Shadow) {
        let d = tempfile::tempdir().unwrap();
        let mut c = ShadowConfig::from_dir(d.path()).with_region("JP");
        c.detectors.local_facts = false;
        (d, Shadow::open(c).unwrap())
    }

    fn openai_stream(pieces: &[&str]) -> String {
        let mut s = String::new();
        s.push_str("data: {\"id\":\"c1\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\"},\"finish_reason\":null}]}\n\n");
        for p in pieces {
            let ev = serde_json::json!({"id":"c1","choices":[{"index":0,"delta":{"content":p},"finish_reason":null}]});
            s.push_str(&format!("data: {ev}\n\n"));
        }
        s.push_str("data: {\"id\":\"c1\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n");
        s.push_str("data: [DONE]\n\n");
        s
    }

    fn collect_openai(out: &str) -> String {
        out.split("\n\n")
            .filter_map(|e| e.strip_prefix("data: "))
            .filter(|d| *d != "[DONE]")
            .filter_map(|d| serde_json::from_str::<Value>(d).ok())
            .filter_map(|v| {
                v.pointer("/choices/0/delta/content")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect()
    }

    #[test]
    fn openai_fake_split_across_events_at_every_offset() {
        let (_d, s) = shadow();
        let real = "Ask John Smith at john.smith@acme-intern.io";
        let fake = s.mask(real).text;
        assert_ne!(fake, real);
        for cut in 1..fake.len() {
            if !fake.is_char_boundary(cut) {
                continue;
            }
            let stream = openai_stream(&[&fake[..cut], &fake[cut..]]);
            // also split the raw bytes at an arbitrary point
            let bytes = stream.as_bytes();
            let mid = (cut * 7) % bytes.len();
            let mut u = EventStreamUnmasker::new(s.clone(), Framing::Sse);
            let mut out = u.push(&bytes[..mid]);
            out.extend(u.push(&bytes[mid..]));
            out.extend(u.finish());
            let out = String::from_utf8(out).unwrap();
            assert_eq!(collect_openai(&out), real, "cut {cut}: {out}");
            assert!(out.ends_with("data: [DONE]\n\n"));
        }
    }

    #[test]
    fn anthropic_events_flush_before_block_stop() {
        let (_d, s) = shadow();
        let real = "Contact Maria Garcia";
        let fake = s.mask(real).text;
        let (a, b) = fake.split_at(fake.len() - 3);
        let mut stream = String::from("event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"m\"}}\n\n");
        for p in [a, b] {
            let ev = serde_json::json!({"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":p}});
            stream.push_str(&format!("event: content_block_delta\ndata: {ev}\n\n"));
        }
        stream.push_str(
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        );
        let mut u = EventStreamUnmasker::new(s.clone(), Framing::Sse);
        let mut out = u.push(stream.as_bytes());
        out.extend(u.finish());
        let out = String::from_utf8(out).unwrap();
        let text: String = out
            .split("\n\n")
            .filter_map(|e| e.lines().find_map(|l| l.strip_prefix("data: ")))
            .filter_map(|d| serde_json::from_str::<Value>(d).ok())
            .filter_map(|v| {
                v.pointer("/delta/text")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .collect();
        assert_eq!(text, real);
        let stop = out.find("content_block_stop").unwrap();
        assert!(
            out[..stop].contains("Garcia"),
            "tail flushed before stop: {out}"
        );
    }
}
