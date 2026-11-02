//! AI chat apps through the HTTPS proxy: only prompt requests are masked (only their prompt
//! fields), answers and history come back restored, pages and other requests are untouched.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::State;
use axum::http::header;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use serde_json::{json, Value};
use shadow_core::{Shadow, ShadowConfig};
use shadow_proxy::Proxy;

const PROMPT: &str = "Summarise this referral: Maria Lopez, NHS number 943 476 5919, \
email maria.lopez@example.com, card 4111 1111 1111 1111 12/29 123.";
/// The prompt as the answer shows it: everything restored except the card's expiry and
/// security code, which are sent as fixed labels and never come back.
const RESTORED: &str = "Summarise this referral: Maria Lopez, NHS number 943 476 5919, \
email maria.lopez@example.com, card 4111 1111 1111 1111 [CARD_EXPIRY] [CARD_CVV].";

/// What the fake chatgpt.com received.
#[derive(Default)]
struct Seen {
    prompt: Option<Value>,
    other: Option<String>,
}
type Shared = Arc<Mutex<Seen>>;

/// Echo the prompt back in ChatGPT's patch stream, three bytes per append.
async fn conversation(State(seen): State<Shared>, body: String) -> Response {
    let v: Value = serde_json::from_str(&body).unwrap();
    let text = v
        .pointer("/messages/0/content/parts/0")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    seen.lock().unwrap().prompt = Some(v);
    let mut s = String::from("event: delta_encoding\ndata: \"v1\"\n\n");
    s.push_str("event: delta\ndata: {\"p\":\"\",\"o\":\"add\",\"v\":{\"message\":{\"content\":{\"parts\":[\"\"]}}},\"c\":0}\n\n");
    let chars: Vec<char> = text.chars().collect();
    for (i, c) in chars.chunks(3).enumerate() {
        let piece: String = c.iter().collect();
        let ev = if i == 0 {
            json!({"p": "/message/content/parts/0", "o": "append", "v": piece})
        } else {
            json!({"v": piece})
        };
        s.push_str(&format!("event: delta\ndata: {ev}\n\n"));
    }
    s.push_str("data: {\"type\":\"message_stream_complete\"}\n\ndata: [DONE]\n\n");
    ([(header::CONTENT_TYPE, "text/event-stream")], s).into_response()
}

/// History: the last prompt as stored by the provider.
async fn history(State(seen): State<Shared>) -> Response {
    let text = seen
        .lock()
        .unwrap()
        .prompt
        .as_ref()
        .and_then(|v| v.pointer("/messages/0/content/parts/0").cloned())
        .unwrap_or(Value::Null);
    axum::Json(json!({"mapping": {"m1": {"message": {"content": {"parts": [text]}}}}}))
        .into_response()
}

async fn other(State(seen): State<Shared>, body: String) -> &'static str {
    seen.lock().unwrap().other = Some(body);
    "{}"
}

struct Env {
    url: String,
    ca_pem: String,
    seen: Shared,
    fake_email: String,
}

async fn setup() -> Env {
    let seen: Shared = Arc::default();
    let app = Router::new()
        .route("/backend-api/f/conversation", post(conversation))
        .route("/backend-api/conversation/c1", get(history))
        .route("/backend-api/settings", post(other))
        .route(
            "/",
            get(|| async { Html("<p>Hello maria.lopez@example.com</p>") }),
        )
        .with_state(seen.clone());
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up = upstream.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(upstream, app).await.unwrap() });

    let dir = tempfile::tempdir().unwrap();
    let mut cfg = ShadowConfig::from_dir(dir.path()).with_region("CN");
    cfg.detectors.local_facts = false;
    let shadow = Shadow::open(cfg).unwrap();
    let proxy = Proxy::new(shadow.clone())
        .unwrap()
        .with_upstream_override("chatgpt.com", &format!("http://{up}"));
    let ca_pem = proxy.ca().pem();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(proxy).serve(listener));
    let fake_email = shadow.mask("maria.lopez@example.com").text;
    std::mem::forget(dir);
    Env {
        url,
        ca_pem,
        seen,
        fake_email,
    }
}

fn browser(env: &Env) -> reqwest::Client {
    reqwest::Client::builder()
        .proxy(reqwest::Proxy::https(&env.url).unwrap())
        .add_root_certificate(reqwest::Certificate::from_pem(env.ca_pem.as_bytes()).unwrap())
        .user_agent("Mozilla/5.0 (Macintosh) AppleWebKit/537.36 Chrome/131.0 Safari/537.36")
        .build()
        .unwrap()
}

/// Replay the appends of a ChatGPT stream.
fn answer(stream: &str) -> String {
    let mut out = String::new();
    for d in stream.lines().filter_map(|l| l.strip_prefix("data: ")) {
        let Ok(v) = serde_json::from_str::<Value>(d) else {
            continue;
        };
        if v.get("p").is_some_and(|p| p != "") || v.get("o").is_none() {
            if let Some(t) = v.get("v").and_then(Value::as_str) {
                out.push_str(t);
            }
        }
    }
    out
}

#[tokio::test]
async fn chatgpt_prompt_is_masked_and_answer_and_history_restored() {
    let env = setup().await;
    let c = browser(&env);
    let r = c
        .post("https://chatgpt.com/backend-api/f/conversation")
        .json(&json!({
            "action": "next",
            "messages": [{"id": "u1", "author": {"role": "user"},
                "content": {"content_type": "text", "parts": [PROMPT]}}],
            "conversation_id": "c1",
            "parent_message_id": "client-created-root",
            "model": "auto",
            "timezone_offset_min": -60,
            "timezone": "Europe/Berlin"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let stream = r.text().await.unwrap();
    assert_eq!(answer(&stream), RESTORED, "{stream}");

    // What left the machine.
    let sent = env.seen.lock().unwrap().prompt.clone().unwrap();
    let text = sent
        .pointer("/messages/0/content/parts/0")
        .unwrap()
        .as_str()
        .unwrap();
    for real in [
        "Maria Lopez",
        "943 476 5919",
        "maria.lopez@example.com",
        "4111 1111 1111 1111",
        "12/29",
    ] {
        assert!(!text.contains(real), "{real} leaked: {text}");
    }
    assert_eq!(sent["conversation_id"], "c1");
    assert_eq!(sent["messages"][0]["id"], "u1");
    assert_eq!(sent["timezone"], "Asia/Shanghai");
    assert_eq!(sent["timezone_offset_min"], -480);

    // Reloaded history reads normally.
    let h: Value = c
        .get("https://chatgpt.com/backend-api/conversation/c1")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        h.pointer("/mapping/m1/message/content/parts/0").unwrap(),
        RESTORED
    );

    // Other requests and pages are not touched.
    c.post("https://chatgpt.com/backend-api/settings")
        .body("{\"email\":\"maria.lopez@example.com\"}")
        .send()
        .await
        .unwrap();
    assert_eq!(
        env.seen.lock().unwrap().other.as_deref(),
        Some("{\"email\":\"maria.lopez@example.com\"}")
    );
    let page = c
        .get("https://chatgpt.com/")
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert_eq!(page, "<p>Hello maria.lopez@example.com</p>");

    // One incident, from ChatGPT in a browser; the settings call and page load are not recorded.
    let mut evs = vec![];
    for _ in 0..50 {
        let v: Value = reqwest::Client::builder()
            .no_proxy()
            .build()
            .unwrap()
            .get(format!("{}/shadow/activity?limit=10", env.url))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        evs = v["events"].as_array().cloned().unwrap_or_default();
        if !evs.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(evs.len(), 1, "{evs:?}");
    assert_eq!(evs[0]["provider"], "chatgpt");
    assert_eq!(evs[0]["action"], "masked");
    assert_eq!(evs[0]["client"], "Google Chrome");
    assert!(!env.fake_email.is_empty());
}
