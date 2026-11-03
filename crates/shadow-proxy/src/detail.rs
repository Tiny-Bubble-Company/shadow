//! Incident details for the local dashboard: the request text as written and as sent, the
//! model, conversation headers and the local process that sent it. Built only for masked
//! or blocked requests, only when `activity.details` is on; saved encrypted across restarts
//! with `activity.persist_details`.

use std::net::SocketAddr;
use std::process::Command;

use http::HeaderMap;
use serde_json::Value;
use shadow_core::activity::{Process, TextPart};

/// Address of the client connection, stored in request extensions.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ClientAddr(pub SocketAddr);

/// Most text parts kept per incident.
const MAX_PARTS: usize = 40;
/// Longest text kept per part, in bytes.
const MAX_TEXT: usize = 20_000;
/// Longest header value kept.
const MAX_HEADER: usize = 200;

/// Keys whose string values are prompt text even when nothing in them was masked.
const TEXT_KEYS: &[&str] = &[
    "content",
    "text",
    "input",
    "prompt",
    "instructions",
    "system",
    "query",
    "arguments",
    "input_text",
];

/// Subtrees that are configuration, not conversation.
const SKIP_KEYS: &[&str] = &[
    "tools",
    "functions",
    "tool_choice",
    "response_format",
    "generationConfig",
    "safetySettings",
    "stop",
];

/// Model named in a request body.
pub(crate) fn model(v: &Value) -> Option<String> {
    v.get("model")
        .and_then(Value::as_str)
        .map(|m| cut(m, 120).0)
}

/// Session, conversation or thread ids sent as headers.
pub(crate) fn conversation(headers: &HeaderMap) -> std::collections::BTreeMap<String, String> {
    headers
        .iter()
        .filter(|(k, _)| {
            let k = k.as_str();
            ["session", "conversation", "thread"]
                .iter()
                .any(|n| k.contains(n))
                && !["token", "key", "auth", "secret", "cookie"]
                    .iter()
                    .any(|n| k.contains(n))
        })
        .filter_map(|(k, v)| Some((k.as_str().to_string(), cut(v.to_str().ok()?, MAX_HEADER).0)))
        .take(8)
        .collect()
}

/// Text parts of a JSON body: `orig` as the app wrote it, `sent` as the model received it
/// (`None` when blocked). Returns the parts and whether anything was cut.
pub(crate) fn json_parts(orig: &Value, sent: Option<&Value>) -> (Vec<TextPart>, bool) {
    let mut all = vec![];
    walk(orig, sent, String::new(), None, None, &mut all);
    let truncated_count = all.len() > MAX_PARTS;
    let parts = keep(all);
    let mut truncated = truncated_count;
    let parts = parts
        .into_iter()
        .map(|(p, _)| {
            let (original, a) = cut(&p.original, MAX_TEXT);
            let sent = p.sent.map(|s| {
                let (s, b) = cut(&s, MAX_TEXT);
                truncated |= b;
                s
            });
            truncated |= a;
            TextPart {
                original,
                sent,
                ..p
            }
        })
        .collect();
    (parts, truncated)
}

/// A whole non-JSON body as one part.
pub(crate) fn text_part(orig: &str, sent: Option<&str>) -> (Vec<TextPart>, bool) {
    let (original, a) = cut(orig, MAX_TEXT);
    let (sent, b) = match sent {
        Some(s) => {
            let (s, b) = cut(s, MAX_TEXT);
            (Some(s), b)
        }
        None => (None, false),
    };
    (
        vec![TextPart {
            role: None,
            field: "body".into(),
            original,
            sent,
        }],
        a || b,
    )
}

/// Keep every changed part and as much recent context as fits, in body order.
fn keep(all: Vec<(TextPart, bool)>) -> Vec<(TextPart, bool)> {
    if all.len() <= MAX_PARTS {
        return all;
    }
    let changed = all.iter().filter(|(_, c)| *c).count().min(MAX_PARTS);
    let mut context = MAX_PARTS - changed;
    let mut picked = vec![false; all.len()];
    let mut budget = changed;
    for (i, (_, c)) in all.iter().enumerate() {
        if *c && budget > 0 {
            picked[i] = true;
            budget -= 1;
        }
    }
    for i in (0..all.len()).rev() {
        if context == 0 {
            break;
        }
        if !picked[i] {
            picked[i] = true;
            context -= 1;
        }
    }
    all.into_iter()
        .zip(picked)
        .filter_map(|(p, k)| k.then_some(p))
        .collect()
}

fn walk(
    o: &Value,
    s: Option<&Value>,
    path: String,
    key: Option<&str>,
    role: Option<&str>,
    out: &mut Vec<(TextPart, bool)>,
) {
    match o {
        Value::Object(map) => {
            let role = map.get("role").and_then(Value::as_str).or(role);
            for (k, v) in map {
                if SKIP_KEYS.contains(&k.as_str()) || k == "model" {
                    continue;
                }
                let sv = s.and_then(|s| s.get(k));
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                let r = match (k.as_str(), role) {
                    (
                        "system" | "instructions" | "system_instruction" | "systemInstruction",
                        None,
                    ) => Some("system"),
                    ("input" | "prompt", None) => Some("user"),
                    _ => role,
                };
                walk(v, sv, p, Some(k), r, out);
            }
        }
        Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                let sv = s.and_then(|s| s.get(i));
                walk(v, sv, format!("{path}[{i}]"), key, role, out);
            }
        }
        Value::String(text) => {
            if text.trim().is_empty() || looks_binary(text) {
                return;
            }
            let sent = s.and_then(Value::as_str);
            let changed = sent.is_some_and(|t| t != text);
            let texty = key.is_some_and(|k| TEXT_KEYS.contains(&k));
            if !changed && !texty {
                return;
            }
            out.push((
                TextPart {
                    role: role.map(str::to_string),
                    field: path,
                    original: text.clone(),
                    sent: s.map(|_| sent.unwrap_or(text).to_string()),
                },
                changed,
            ));
        }
        _ => {}
    }
}

/// Data URLs and long unbroken base64 runs are file payloads, not prompt text.
fn looks_binary(s: &str) -> bool {
    s.starts_with("data:") || (s.len() > 256 && !s.bytes().any(|b| b.is_ascii_whitespace()))
}

/// At most `max` bytes of `s`, on a char boundary; `true` when cut.
fn cut(s: &str, max: usize) -> (String, bool) {
    if s.len() <= max {
        return (s.to_string(), false);
    }
    let mut end = max;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    (s[..end].to_string(), true)
}

/// The local process that owns the client end of a loopback connection from `peer`.
/// Best effort: `None` for remote peers or when the platform tool is missing or slow.
pub(crate) fn process_for(peer: SocketAddr) -> Option<Process> {
    if !crate::activity::is_loopback(&peer) {
        return None;
    }
    let port = peer.port();
    let own = std::process::id();
    if cfg!(windows) {
        windows_process(port, own)
    } else {
        lsof_process(port, own)
    }
}

fn lsof_process(port: u16, own: u32) -> Option<Process> {
    let out = Command::new("lsof")
        .args(["-nP", "+c", "0", "-S", "2", "-sTCP:ESTABLISHED", "-Fpcn"])
        .arg(format!("-iTCP:{port}"))
        .output()
        .ok()?;
    parse_lsof(&String::from_utf8_lossy(&out.stdout), port, own)
}

/// Parse `lsof -Fpcn` output: the process whose socket has `port` as its local end.
fn parse_lsof(out: &str, port: u16, own: u32) -> Option<Process> {
    let (mut pid, mut name) = (0u32, String::new());
    let local = format!(":{port}");
    for line in out.lines() {
        let (tag, rest) = line.split_at(line.len().min(1));
        match tag {
            "p" => pid = rest.parse().unwrap_or(0),
            "c" => name = rest.to_string(),
            "n" => {
                let src = rest.split("->").next().unwrap_or("");
                if pid != 0 && pid != own && src.ends_with(&local) {
                    return Some(Process {
                        pid,
                        name: name.clone(),
                    });
                }
            }
            _ => {}
        }
    }
    None
}

fn windows_process(port: u16, own: u32) -> Option<Process> {
    let out = Command::new("netstat")
        .args(["-ano", "-p", "TCP"])
        .output()
        .ok()?;
    let local = format!(":{port}");
    let pid = String::from_utf8_lossy(&out.stdout).lines().find_map(|l| {
        let cols: Vec<&str> = l.split_whitespace().collect();
        (cols.len() >= 5 && cols[1].ends_with(&local) && cols[3] == "ESTABLISHED")
            .then(|| cols[4].parse::<u32>().ok())
            .flatten()
    })?;
    if pid == own {
        return None;
    }
    let out = Command::new("tasklist")
        .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let name = text.split(',').next()?.trim().trim_matches('"').to_string();
    Some(Process { pid, name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn changed_and_prompt_text_is_kept_config_is_not() {
        let orig = json!({
            "model": "gpt-4o-mini",
            "tools": [{"function": {"description": "Look up jane@example.org"}}],
            "messages": [
                {"role": "system", "content": "You are helpful."},
                {"role": "user", "content": [{"type": "text", "text": "Email jane@example.org"}]},
                {"role": "user", "content": [{"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAA"}}]}
            ]
        });
        let mut sent = orig.clone();
        sent["messages"][1]["content"][0]["text"] = json!("Email mei@lin.example");
        let (parts, cut) = json_parts(&orig, Some(&sent));
        assert!(!cut);
        assert_eq!(parts.len(), 2, "{parts:?}");
        assert_eq!(parts[0].role.as_deref(), Some("system"));
        assert_eq!(parts[1].field, "messages[1].content[0].text");
        assert_eq!(parts[1].role.as_deref(), Some("user"));
        assert_eq!(parts[1].sent.as_deref(), Some("Email mei@lin.example"));
        assert_eq!(model(&orig).as_deref(), Some("gpt-4o-mini"));
    }

    #[test]
    fn blocked_requests_have_no_sent_text() {
        let orig = json!({"system": "Be brief.", "messages": [{"role": "user", "content": "hi"}]});
        let (parts, _) = json_parts(&orig, None);
        assert_eq!(parts[0].role.as_deref(), Some("system"));
        assert!(parts.iter().all(|p| p.sent.is_none()));
    }

    #[test]
    fn long_conversations_keep_every_change() {
        let msgs: Vec<Value> = (0..100)
            .map(|i| json!({"role": "user", "content": format!("message {i}")}))
            .collect();
        let orig = json!({ "messages": msgs });
        let mut sent = orig.clone();
        sent["messages"][3]["content"] = json!("changed");
        let (parts, cut) = json_parts(&orig, Some(&sent));
        assert!(cut);
        assert_eq!(parts.len(), MAX_PARTS);
        assert_eq!(parts[0].field, "messages[3].content");
        assert_eq!(parts.last().unwrap().field, "messages[99].content");
    }

    #[test]
    fn text_is_cut_on_a_char_boundary() {
        let (s, cut) = cut("ééé", 3);
        assert!(cut);
        assert_eq!(s, "é");
    }

    #[test]
    fn lsof_output_names_the_client_end() {
        let out = "p656\ncshadow\nf10\nn127.0.0.1:8877->127.0.0.1:54604\np900\ncclaude\nf3\nn127.0.0.1:54604->127.0.0.1:8877\n";
        let p = parse_lsof(out, 54604, 656).unwrap();
        assert_eq!((p.pid, p.name.as_str()), (900, "claude"));
        assert!(parse_lsof(out, 54605, 656).is_none());
    }

    #[test]
    fn conversation_headers_are_picked() {
        let mut h = HeaderMap::new();
        h.insert("x-claude-code-session-id", "abc".parse().unwrap());
        h.insert("authorization", "Bearer x".parse().unwrap());
        h.insert("x-session-token", "secret".parse().unwrap());
        let c = conversation(&h);
        assert_eq!(c.len(), 1);
        assert_eq!(c["x-claude-code-session-id"], "abc");
    }
}
