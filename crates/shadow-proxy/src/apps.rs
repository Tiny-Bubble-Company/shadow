//! AI chat apps used in a browser or desktop app (ChatGPT, Claude.ai, Gemini, ...).
//!
//! Their web traffic carries much more than prompts: sign-in, settings, history, analytics.
//! Shadow masks only the requests that send a prompt, and only the fields that hold the
//! prompt text, so ids, tokens and the account itself are left alone. Every JSON or event
//! stream answer from the app is restored, so answers and reloaded history read normally.
//! Request formats are the apps' own and change without notice; anything unrecognised is
//! forwarded as it is.

use std::sync::Arc;

use serde_json::Value;
use shadow_core::{RequestContext, Session, Shadow};

use crate::hosts::glob_match;

/// How an app's prompt request is encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Format {
    /// JSON body; the prompt is in the string leaves under these keys.
    Json(&'static [&'static str]),
    /// Gemini web: form body with the prompt nested in JSON inside the `f.req` field.
    GeminiForm,
}

/// One supported chat app.
#[derive(Debug)]
pub(crate) struct ChatApp {
    pub id: &'static str,
    pub hosts: &'static [&'static str],
    /// Paths (glob) of `POST` requests that carry a prompt.
    pub prompts: &'static [&'static str],
    pub format: Format,
}

pub(crate) const CHAT_APPS: &[ChatApp] = &[
    ChatApp {
        id: "chatgpt",
        hosts: &["chatgpt.com", "chat.openai.com"],
        prompts: &[
            "/backend-api/conversation",
            "/backend-api/f/conversation*",
            "/backend-anon/conversation",
            "/backend-anon/f/conversation*",
        ],
        format: Format::Json(&["parts", "partial_query"]),
    },
    ChatApp {
        id: "claude",
        hosts: &["claude.ai"],
        prompts: &["/api/organizations/*/chat_conversations/*completion"],
        format: Format::Json(&["prompt", "extracted_content"]),
    },
    ChatApp {
        id: "gemini_app",
        hosts: &["gemini.google.com"],
        prompts: &["/_/BardChatUi/data/*StreamGenerate*"],
        format: Format::GeminiForm,
    },
    ChatApp {
        id: "perplexity_app",
        hosts: &["perplexity.ai", "www.perplexity.ai"],
        prompts: &["/rest/sse/perplexity_ask*"],
        format: Format::Json(&["query_str"]),
    },
    ChatApp {
        id: "deepseek_app",
        hosts: &["chat.deepseek.com"],
        prompts: &["/api/v0/chat/completion"],
        format: Format::Json(&["prompt"]),
    },
    ChatApp {
        id: "grok",
        hosts: &["grok.com"],
        prompts: &["/rest/app-chat/conversations/*"],
        format: Format::Json(&["message"]),
    },
];

/// Every chat-app host, for the interception list.
pub(crate) fn hosts() -> impl Iterator<Item = &'static str> {
    CHAT_APPS.iter().flat_map(|a| a.hosts.iter().copied())
}

/// The chat app served from `host`, if any.
pub(crate) fn find(host: &str) -> Option<&'static ChatApp> {
    let host = host.trim_end_matches('.').to_ascii_lowercase();
    CHAT_APPS
        .iter()
        .find(|a| a.hosts.iter().any(|h| glob_match(h, &host)))
}

impl ChatApp {
    /// Does this request send a prompt?
    pub fn is_prompt(&self, method: &str, path: &str) -> bool {
        method.eq_ignore_ascii_case("POST") && self.prompts.iter().any(|p| glob_match(p, path))
    }
}

/// Mask the prompt fields of a JSON prompt request in place, and set its timezone fields to
/// the persona's. Returns the session (for the answer) and the number of values replaced.
pub(crate) fn mask_json(shadow: &Shadow, keys: &[&str], body: &mut Value) -> (Arc<Session>, usize) {
    let mut texts = vec![];
    visit(body, keys, false, &mut |s: &mut String| {
        texts.push(s.clone())
    });
    // Fakes, not tokens: the app shows its history again later, outside this request.
    let session = Arc::new(
        shadow
            .session(RequestContext {
                instruction: texts.last().cloned().unwrap_or_default(),
                tools: vec![],
            })
            .surrogates_only(),
    );
    let mut n = 0;
    visit(body, keys, false, &mut |s: &mut String| {
        let m = session.mask(s);
        if m.replacements > 0 {
            n += m.replacements;
            *s = m.text;
        }
    });
    flip_timezone(shadow, body);
    (session, n)
}

/// Visit string leaves that sit under one of `keys`.
fn visit(v: &mut Value, keys: &[&str], inside: bool, f: &mut dyn FnMut(&mut String)) {
    match v {
        Value::String(s) if inside => {
            if !s.starts_with("data:") && !s.contains("://") {
                f(s)
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| visit(x, keys, inside, f)),
        Value::Object(o) => {
            for (k, x) in o.iter_mut() {
                let under = inside || keys.contains(&k.as_str());
                // Ids and pointers inside a prompt part are not text.
                if under && (k == "id" || k.ends_with("_id") || k == "content_type") {
                    continue;
                }
                visit(x, keys, under, f);
            }
        }
        _ => {}
    }
}

/// Make the request's timezone match the persona: `timezone` (IANA name) and
/// `timezone_offset_min` (JavaScript sign: minutes *behind* UTC).
fn flip_timezone(shadow: &Shadow, body: &mut Value) {
    let tz = shadow.persona().timezone.clone();
    let Some(obj) = body.as_object_mut() else {
        return;
    };
    if obj.get("timezone").is_some_and(Value::is_string) {
        obj.insert("timezone".into(), Value::String(tz.clone()));
    }
    if obj.get("timezone_offset_min").is_some_and(Value::is_number) {
        if let Some(off) = shadow_core::persona::standard_offset_minutes(&tz) {
            obj.insert("timezone_offset_min".into(), Value::from(-off));
        }
    }
}

/// Mask a Gemini web prompt: a form body whose `f.req` field is JSON `[null, "<inner>"]`,
/// where `<inner>` is JSON again and `inner[0][0]` is the prompt. Other fields (the `at`
/// anti-forgery token) are left exactly as they were. Returns the new body, the session and
/// the number of values replaced and the prompt before and after, or `None` when the body is
/// not in that shape.
pub(crate) fn mask_gemini_form(shadow: &Shadow, body: &str) -> Option<GeminiMasked> {
    let mut found = false;
    let mut out = vec![];
    let mut result = None;
    for pair in body.split('&') {
        let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
        if k != "f.req" {
            out.push(pair.to_string());
            continue;
        }
        let decoded = decode(v)?;
        let mut outer: Value = serde_json::from_str(&decoded).ok()?;
        let inner_text = outer.get(1)?.as_str()?.to_string();
        let mut inner: Value = serde_json::from_str(&inner_text).ok()?;
        let prompt = inner.pointer("/0/0")?.as_str()?.to_string();
        let session = Arc::new(
            shadow
                .session(RequestContext {
                    instruction: prompt.clone(),
                    tools: vec![],
                })
                .surrogates_only(),
        );
        let m = session.mask(&prompt);
        *inner.pointer_mut("/0/0")? = Value::String(m.text.clone());
        outer[1] = Value::String(inner.to_string());
        out.push(format!("f.req={}", encode(&outer.to_string())));
        result = Some((session, m.replacements, prompt, m.text));
        found = true;
    }
    if !found {
        return None;
    }
    let (session, replaced, prompt, sent) = result?;
    Some(GeminiMasked {
        body: out.join("&"),
        session,
        replaced,
        prompt,
        sent,
    })
}

/// A masked Gemini web prompt request.
pub(crate) struct GeminiMasked {
    pub body: String,
    pub session: Arc<Session>,
    pub replaced: usize,
    pub prompt: String,
    pub sent: String,
}

/// `application/x-www-form-urlencoded` value decoding (`+` is a space).
fn decode(v: &str) -> Option<String> {
    let bytes = v.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => out.push(b' '),
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 2;
            }
            b => out.push(b),
        }
        i += 1;
    }
    String::from_utf8(out).ok()
}

/// Form encoding: unreserved characters as they are, everything else percent-encoded.
fn encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'*') {
            out.push(b as char);
        } else if b == b' ' {
            out.push('+');
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn shadow() -> (tempfile::TempDir, Shadow) {
        let dir = tempfile::tempdir().unwrap();
        let mut cfg = shadow_core::ShadowConfig::from_dir(dir.path()).with_region("CN");
        cfg.detectors.local_facts = false;
        (dir, Shadow::open(cfg).unwrap())
    }

    #[test]
    fn finds_apps_and_prompt_requests() {
        let a = find("chatgpt.com").unwrap();
        assert!(a.is_prompt("POST", "/backend-api/f/conversation"));
        assert!(a.is_prompt("POST", "/backend-api/conversation"));
        assert!(!a.is_prompt("GET", "/backend-api/conversation"));
        assert!(!a.is_prompt("POST", "/backend-api/me"));
        let c = find("claude.ai").unwrap();
        assert!(c.is_prompt(
            "POST",
            "/api/organizations/o1/chat_conversations/c1/completion"
        ));
        assert!(c.is_prompt(
            "POST",
            "/api/organizations/o1/chat_conversations/c1/retry_completion"
        ));
        assert!(!c.is_prompt("POST", "/api/organizations/o1/chat_conversations"));
        assert!(find("gemini.google.com").unwrap().is_prompt(
            "POST",
            "/_/BardChatUi/data/assistant.lamda.BardFrontendService/StreamGenerate"
        ));
        assert!(find("api.openai.com").is_none());
    }

    #[test]
    fn chatgpt_prompt_is_masked_and_ids_and_timezone_handled() {
        let (_d, s) = shadow();
        let mut body = json!({
            "action": "next",
            "messages": [{
                "id": "aaa-111",
                "author": {"role": "user"},
                "content": {"content_type": "text", "parts": ["Card 4111 1111 1111 1111 for Maria Lopez, maria.lopez@example.com"]},
                "metadata": {}
            }],
            "conversation_id": "c0ffee-1234",
            "parent_message_id": "client-created-root",
            "model": "auto",
            "timezone_offset_min": -60,
            "timezone": "Europe/Berlin"
        });
        let (session, n) = mask_json(&s, &["parts", "partial_query"], &mut body);
        assert!(n >= 3, "{body}");
        let text = body
            .pointer("/messages/0/content/parts/0")
            .unwrap()
            .as_str()
            .unwrap();
        assert!(!text.contains("maria.lopez@example.com") && !text.contains("4111 1111 1111 1111"));
        assert_eq!(body["conversation_id"], "c0ffee-1234");
        assert_eq!(body["messages"][0]["id"], "aaa-111");
        assert_eq!(body["timezone"], s.persona().timezone.as_str());
        assert_eq!(body["timezone_offset_min"], -480);
        assert!(session.unmask(text).contains("maria.lopez@example.com"));
    }

    #[test]
    fn gemini_form_masks_only_the_prompt() {
        let (_d, s) = shadow();
        let inner = json!([
            [
                "Email maria.lopez@example.com the results for card 4111 1111 1111 1111",
                0,
                null
            ],
            ["en"],
            ["c_1", "r_2", "rc_3"]
        ]);
        let outer = json!([null, inner.to_string()]);
        let body = format!(
            "f.req={}&at=AJvLN6M%3A1730000000000&",
            encode(&outer.to_string())
        );
        let g = mask_gemini_form(&s, &body).unwrap();
        let (out, n) = (g.body, g.replaced);
        assert!(n >= 1);
        assert!(
            g.prompt.contains("4111 1111 1111 1111") && !g.sent.contains("4111 1111 1111 1111")
        );
        assert!(out.contains("&at=AJvLN6M%3A1730000000000&"));
        let freq = decode(out.split('&').next().unwrap().trim_start_matches("f.req=")).unwrap();
        let o: Value = serde_json::from_str(&freq).unwrap();
        let i: Value = serde_json::from_str(o[1].as_str().unwrap()).unwrap();
        assert!(!i[0][0].as_str().unwrap().contains("4111 1111 1111 1111"));
        assert_eq!(i[2], json!(["c_1", "r_2", "rc_3"]));
        assert!(mask_gemini_form(&s, "at=x&other=1").is_none());
    }

    #[test]
    fn form_encoding_round_trips() {
        let s = "[null,\"[[\\\"héllo wörld & co\\\"]]\"]";
        assert_eq!(decode(&encode(s)).unwrap(), s);
        assert_eq!(decode("a+b%20c").unwrap(), "a b c");
    }
}
