//! Translation primitives: the masked result type, fake->real replacement (including
//! streaming with hold-back), and JSON tree walking.

use std::sync::Arc;

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use serde::Serialize;
use serde_json::Value;

/// Result of [`crate::Shadow::mask`].
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct Masked {
    /// Text with every sensitive span replaced by its fake.
    pub text: String,
    /// Number of spans replaced.
    pub replacements: usize,
    /// Distinct kinds replaced (sorted), e.g. `["email","person"]`.
    pub kinds: Vec<String>,
}

/// Immutable fake->real replacer built from a vault snapshot.
pub struct Unmasker {
    ac: Option<AhoCorasick>,
    reals: Vec<String>,
    /// Longest fake in bytes.
    pub max_len: usize,
}

impl Unmasker {
    /// Build from `(fake, real)` pairs.
    pub fn new(pairs: Vec<(String, String)>) -> Self {
        let max_len = pairs.iter().map(|(f, _)| f.len()).max().unwrap_or(0);
        let (fakes, reals): (Vec<String>, Vec<String>) = pairs.into_iter().unzip();
        let ac = if fakes.is_empty() {
            None
        } else {
            AhoCorasickBuilder::new()
                .match_kind(MatchKind::LeftmostLongest)
                .build(&fakes)
                .ok()
        };
        Self { ac, reals, max_len }
    }

    /// Replace every fake in `text` with its real value.
    pub fn apply(&self, text: &str) -> String {
        let Some(ac) = &self.ac else {
            return text.to_string();
        };
        let mut out = String::with_capacity(text.len());
        let mut last = 0;
        for m in ac.find_iter(text) {
            out.push_str(&text[last..m.start()]);
            out.push_str(&self.reals[m.pattern().as_usize()]);
            last = m.end();
        }
        out.push_str(&text[last..]);
        out
    }

    /// True if `text` contains any fake.
    pub fn contains_fake(&self, text: &str) -> bool {
        self.ac.as_ref().is_some_and(|ac| ac.is_match(text))
    }
}

/// Incremental unmasker for streamed text. Holds back up to `max_len - 1` bytes so a
/// fake split across chunks is still restored. Output of `push`* + `finish` equals
/// `unmask(whole text)`.
pub struct StreamUnmasker {
    un: Arc<Unmasker>,
    buf: String,
}

impl StreamUnmasker {
    /// Create from an unmasker snapshot.
    pub fn new(un: Arc<Unmasker>) -> Self {
        Self {
            un,
            buf: String::new(),
        }
    }

    /// Feed a chunk; returns the text that is safe to emit now.
    pub fn push(&mut self, chunk: &str) -> String {
        self.buf.push_str(chunk);
        let Some(ac) = &self.un.ac else {
            return std::mem::take(&mut self.buf);
        };
        let hold = self.un.max_len.saturating_sub(1);
        if self.buf.len() <= hold {
            return String::new();
        }
        let mut cut = self.buf.len() - hold;
        while !self.buf.is_char_boundary(cut) {
            cut -= 1;
        }
        let mut out = String::new();
        let mut last = 0;
        for m in ac.find_iter(&self.buf) {
            if m.start() >= cut {
                break;
            }
            out.push_str(&self.buf[last..m.start()]);
            out.push_str(&self.un.reals[m.pattern().as_usize()]);
            last = m.end();
        }
        if last < cut {
            out.push_str(&self.buf[last..cut]);
            last = cut;
        }
        self.buf.drain(..last);
        out
    }

    /// Flush everything still held back.
    pub fn finish(&mut self) -> String {
        let rest = std::mem::take(&mut self.buf);
        self.un.apply(&rest)
    }

    /// Bytes currently held back.
    pub fn pending(&self) -> usize {
        self.buf.len()
    }
}

/// JSON keys whose string values are protocol fields and never masked.
pub const SKIP_KEYS: &[&str] = &[
    "model",
    "role",
    "type",
    "object",
    "id",
    "tool_call_id",
    "tool_use_id",
    "finish_reason",
    "stop_reason",
    "encoding_format",
    "media_type",
    "mime_type",
    "detail",
    "service_tier",
    "reasoning_effort",
    "voice",
    "format",
    "tool_choice",
    "stop_sequence",
];

/// Visit every string leaf that is eligible for masking.
pub fn walk_strings_mut(v: &mut Value, key: Option<&str>, f: &mut dyn FnMut(&mut String)) {
    match v {
        Value::String(s) => {
            if key.is_some_and(|k| SKIP_KEYS.contains(&k)) || s.starts_with("data:") {
                return;
            }
            f(s)
        }
        Value::Array(a) => a.iter_mut().for_each(|x| walk_strings_mut(x, key, f)),
        Value::Object(o) => o
            .iter_mut()
            .for_each(|(k, x)| walk_strings_mut(x, Some(k), f)),
        _ => {}
    }
}

/// Visit every string leaf (no skipping), used for unmasking.
pub fn walk_all_strings_mut(v: &mut Value, f: &mut dyn FnMut(&mut String)) {
    match v {
        Value::String(s) => f(s),
        Value::Array(a) => a.iter_mut().for_each(|x| walk_all_strings_mut(x, f)),
        Value::Object(o) => o.values_mut().for_each(|x| walk_all_strings_mut(x, f)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn un() -> Arc<Unmasker> {
        Arc::new(Unmasker::new(vec![
            ("Wei Zhang".into(), "John Smith".into()),
            ("Wei".into(), "Bob".into()),
            ("velora.cn".into(), "acme.com".into()),
        ]))
    }

    #[test]
    fn stream_equals_batch_at_every_split() {
        let text = "Hi Wei Zhang, mail wei@velora.cn — Wei! ünïcode velora.cn end";
        let expected = un().apply(text);
        assert!(expected.contains("John Smith") && expected.contains("acme.com"));
        for i in 0..=text.len() {
            if !text.is_char_boundary(i) {
                continue;
            }
            for j in i..=text.len() {
                if !text.is_char_boundary(j) {
                    continue;
                }
                let mut s = StreamUnmasker::new(un());
                let mut out = s.push(&text[..i]);
                out += &s.push(&text[i..j]);
                out += &s.push(&text[j..]);
                out += &s.finish();
                assert_eq!(out, expected, "split at {i},{j}");
            }
        }
    }

    #[test]
    fn json_walk_skips_protocol_keys() {
        let mut v = serde_json::json!({"model":"m","messages":[{"role":"user","content":"x"}]});
        let mut n = 0;
        walk_strings_mut(&mut v, None, &mut |_| n += 1);
        assert_eq!(n, 1);
    }
}
