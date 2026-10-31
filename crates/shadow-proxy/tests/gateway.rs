//! End-to-end: client -> Shadow (gateway / MITM) -> mock upstream -> Shadow -> client.

use std::sync::{Arc, Mutex};

use axum::extract::State;
use axum::http::{header, HeaderMap};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use serde_json::{json, Value};
use shadow_core::{Shadow, ShadowConfig};
use shadow_proxy::Proxy;

const PROMPT: &str = "I'm John Smith (john.smith@acme-industries.com, +49 30 1234 5678) at Acme GmbH. \
Card 4111 1111 1111 1111, key sk-proj-Abcdefghijklmnopqrstuvwx12345. Draft a reply to Maria Garcia about Project Falcon.";
const REALS: &[&str] = &[
    "John Smith",
    "john.smith@acme-industries.com",
    "+49 30 1234 5678",
    "Acme",
    "4111 1111 1111 1111",
    "sk-proj-Abcdefghijklmnopqrstuvwx12345",
    "Project Falcon",
];
/// Who the prompt's reply is for: the model has to see her to write to the right person.
const RECIPIENT: &str = "Maria Garcia";

#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<(HeaderMap, String)>>>);

async fn mock(State(seen): State<Seen>, headers: HeaderMap, body: String) -> Response {
    seen.0.lock().unwrap().push((headers, body.clone()));
    let v: Value = serde_json::from_str(&body).unwrap();
    let content = v["messages"][0]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let reply = format!("Sure. Summary: {content}");
    if v["stream"].as_bool() == Some(true) {
        let mut sse = String::new();
        let chars: Vec<char> = reply.chars().collect();
        for piece in chars.chunks(3) {
            let p: String = piece.iter().collect();
            let ev = json!({"id":"x","object":"chat.completion.chunk","choices":[{"index":0,"delta":{"content":p},"finish_reason":null}]});
            sse.push_str(&format!("data: {ev}\n\n"));
        }
        sse.push_str("data: {\"id\":\"x\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n");
        ([(header::CONTENT_TYPE, "text/event-stream")], sse).into_response()
    } else {
        axum::Json(json!({"id":"x","object":"chat.completion","choices":[{"index":0,"message":{"role":"assistant","content":reply},"finish_reason":"stop"}],"user_echo": v["user"]})).into_response()
    }
}

struct Env {
    _dir: tempfile::TempDir,
    proxy_url: String,
    seen: Seen,
    shadow: Shadow,
    ca_pem: String,
}

async fn setup() -> Env {
    let seen = Seen::default();
    let app = Router::new()
        .route("/v1/chat/completions", post(mock))
        .with_state(seen.clone());
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_addr = upstream.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(upstream, app).await.unwrap() });

    let dir = tempfile::tempdir().unwrap();
    let mut cfg = ShadowConfig::from_dir(dir.path())
        .with_region("CN")
        .with_terms(vec!["Acme GmbH".into(), "Project Falcon".into()]);
    cfg.detectors.local_facts = false;
    cfg.proxy
        .routes
        .insert("mock".into(), format!("http://{up_addr}"));
    let shadow = Shadow::open(cfg).unwrap();
    let proxy = Proxy::new(shadow.clone())
        .unwrap()
        .with_upstream_override("api.openai.com", &format!("http://{up_addr}"));
    let ca_pem = proxy.ca().pem();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(proxy).serve(listener));
    Env {
        _dir: dir,
        proxy_url,
        seen,
        shadow,
        ca_pem,
    }
}

fn assert_upstream_clean(env: &Env) {
    let seen = env.seen.0.lock().unwrap();
    let (headers, body) = seen.last().expect("upstream was called");
    for r in REALS {
        assert!(!body.contains(r), "upstream saw real value {r:?}: {body}");
    }
    assert!(body.contains(RECIPIENT), "the recipient is passed through: {body}");
    let v: Value = serde_json::from_str(body).unwrap();
    assert_eq!(v["model"], "gpt-4o-mini", "protocol fields untouched");
    assert_eq!(
        v["user"],
        env.shadow.persona().user_id,
        "user field replaced"
    );
    assert!(
        headers.get("openai-organization").is_none(),
        "org header stripped"
    );
    assert_eq!(
        headers.get("x-request-origin").unwrap(),
        env.shadow.persona().hostname.as_str()
    );
    assert!(
        headers.get(header::AUTHORIZATION).is_some(),
        "auth header forwarded"
    );
}

fn request_body(stream: bool) -> Value {
    json!({"model":"gpt-4o-mini","user":"john@acme-industries.com","stream":stream,"messages":[{"role":"user","content":PROMPT}]})
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

#[tokio::test]
async fn gateway_masks_request_and_unmasks_json_response() {
    let env = setup().await;
    let resp = client()
        .post(format!("{}/gw/mock/v1/chat/completions", env.proxy_url))
        .bearer_auth("sk-test")
        .header("OpenAI-Organization", "org-real")
        .header("X-Request-Origin", "laptop-of-john")
        .json(&request_body(false))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: Value = resp.json().await.unwrap();
    assert_eq!(
        v["choices"][0]["message"]["content"],
        format!("Sure. Summary: {PROMPT}")
    );
    assert_upstream_clean(&env);
}

#[tokio::test]
async fn gateway_unmasks_sse_stream() {
    let env = setup().await;
    let resp = client()
        .post(format!("{}/gw/mock/v1/chat/completions", env.proxy_url))
        .bearer_auth("sk-test")
        .header("OpenAI-Organization", "org-real")
        .header("X-Request-Origin", "laptop-of-john")
        .json(&request_body(true))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.headers()[header::CONTENT_TYPE], "text/event-stream");
    let text = resp.text().await.unwrap();
    let content: String = text
        .split("\n\n")
        .filter_map(|e| e.strip_prefix("data: "))
        .filter(|d| *d != "[DONE]")
        .filter_map(|d| serde_json::from_str::<Value>(d).ok())
        .filter_map(|v| {
            v["choices"][0]["delta"]["content"]
                .as_str()
                .map(str::to_string)
        })
        .collect();
    assert_eq!(content, format!("Sure. Summary: {PROMPT}"));
    assert!(text.trim_end().ends_with("data: [DONE]"));
    assert_upstream_clean(&env);
}

#[tokio::test]
async fn forward_proxy_mitm_with_local_ca() {
    let env = setup().await;
    let client = reqwest::Client::builder()
        .proxy(reqwest::Proxy::https(&env.proxy_url).unwrap())
        .add_root_certificate(reqwest::Certificate::from_pem(env.ca_pem.as_bytes()).unwrap())
        .build()
        .unwrap();
    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth("sk-test")
        .header("OpenAI-Organization", "org-real")
        .header("X-Request-Origin", "laptop-of-john")
        .json(&request_body(false))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: Value = resp.json().await.unwrap();
    assert_eq!(
        v["choices"][0]["message"]["content"],
        format!("Sure. Summary: {PROMPT}")
    );
    assert_upstream_clean(&env);
}

#[tokio::test]
async fn status_and_unknown_routes() {
    let env = setup().await;
    let s: Value = client()
        .get(format!("{}/shadow/status", env.proxy_url))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(s["persona"]["region"], "CN");
    let r = client()
        .post(format!("{}/gw/nope/x", env.proxy_url))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 404);
    let pac = client()
        .get(format!("{}/proxy.pac", env.proxy_url))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    assert!(pac.contains("api.openai.com"));
}
