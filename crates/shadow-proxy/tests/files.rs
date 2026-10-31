//! End-to-end file masking and per-request sessions through the gateway.

use std::sync::{Arc, Mutex};

use axum::body::Bytes;
use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use serde_json::{json, Value};
use shadow_core::config::FileAction;
use shadow_core::{Shadow, ShadowConfig};
use shadow_proxy::Proxy;

#[derive(Clone, Default)]
struct Seen(Arc<Mutex<Vec<(HeaderMap, Bytes)>>>);

async fn upload(State(seen): State<Seen>, headers: HeaderMap, body: Bytes) -> Response {
    seen.0.lock().unwrap().push((headers, body));
    axum::Json(json!({"id":"file-1","object":"file"})).into_response()
}

/// Echoes the user message back, as an LLM summarising it would.
async fn chat(State(seen): State<Seen>, headers: HeaderMap, body: Bytes) -> Response {
    seen.0.lock().unwrap().push((headers, body.clone()));
    let v: Value = serde_json::from_slice(&body).unwrap();
    let content = v["messages"][0]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    axum::Json(json!({"choices":[{"index":0,"message":{"role":"assistant","content":format!("Summary: {content}")}}]}))
        .into_response()
}

struct Env {
    _dir: tempfile::TempDir,
    proxy_url: String,
    seen: Seen,
}

async fn setup(images: FileAction) -> Env {
    let seen = Seen::default();
    let app = Router::new()
        .route("/v1/files", post(upload))
        .route("/v1/chat/completions", post(chat))
        .with_state(seen.clone());
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up_addr = upstream.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(upstream, app).await.unwrap() });

    let dir = tempfile::tempdir().unwrap();
    let mut cfg = ShadowConfig::from_dir(dir.path()).with_region("CN");
    cfg.detectors.local_facts = false;
    cfg.files.images = images;
    cfg.proxy
        .routes
        .insert("mock".into(), format!("http://{up_addr}"));
    let shadow = Shadow::open(cfg).unwrap();
    let proxy = Proxy::new(shadow).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(proxy).serve(listener));
    Env {
        _dir: dir,
        proxy_url,
        seen,
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

fn multipart(boundary: &str, filename: &str, ct: &str, file: &[u8]) -> Vec<u8> {
    let mut b = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"purpose\"\r\n\r\nassistants\r\n\
--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {ct}\r\n\r\n"
    )
    .into_bytes();
    b.extend_from_slice(file);
    b.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    b
}

#[tokio::test]
async fn multipart_upload_reaches_upstream_masked() {
    let env = setup(FileAction::Pass).await;
    let secret = "Customer: john.smith@acme-industries.com, call +49 30 1234 5678";
    let body = multipart("shadowtest", "notes.txt", "text/plain", secret.as_bytes());
    let resp = client()
        .post(format!("{}/gw/mock/v1/files", env.proxy_url))
        .bearer_auth("sk-test")
        .header(
            "content-type",
            "multipart/form-data; boundary=\"shadowtest\"",
        )
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let seen = env.seen.0.lock().unwrap();
    let (_, up) = seen.last().expect("upstream called");
    let up = String::from_utf8_lossy(up);
    assert!(!up.contains("john.smith@acme-industries.com"), "{up}");
    assert!(!up.contains("1234 5678"), "{up}");
    assert!(
        up.contains("filename=\"notes.txt\"") && up.contains("Customer: "),
        "{up}"
    );
    assert!(up.ends_with("\r\n--shadowtest--\r\n"), "{up}");
}

#[tokio::test]
async fn blocked_image_returns_error() {
    let env = setup(FileAction::Block).await;
    let png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR";
    let body = multipart("b", "shot.png", "image/png", png);
    let resp = client()
        .post(format!("{}/gw/mock/v1/files", env.proxy_url))
        .header("content-type", "multipart/form-data; boundary=b")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
    let v: Value = resp.json().await.unwrap();
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("images are blocked"),
        "{v}"
    );
    assert!(
        env.seen.0.lock().unwrap().is_empty(),
        "upstream never called"
    );

    // Same for an image embedded in a JSON chat request.
    let data = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUg==";
    let resp = client()
        .post(format!("{}/gw/mock/v1/chat/completions", env.proxy_url))
        .json(&json!({"model":"m","messages":[{"role":"user","content":[{"type":"image_url","image_url":{"url":data}}]}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);
    assert!(env.seen.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn report_intent_uses_request_tokens_and_reverses_them() {
    let env = setup(FileAction::Pass).await;
    let emails = [
        "anna.berg@nordic-mail.se",
        "li.wei@sinotrade.cn",
        "omar.haddad@gulfnet.ae",
    ];
    let prompt = format!(
        "Summarise these customers: {}, {}, {}",
        emails[0], emails[1], emails[2]
    );
    let resp = client()
        .post(format!("{}/gw/mock/v1/chat/completions", env.proxy_url))
        .bearer_auth("sk-test")
        .json(&json!({"model":"m","messages":[{"role":"user","content":prompt}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    {
        let seen = env.seen.0.lock().unwrap();
        let (_, up) = seen.last().unwrap();
        let up = String::from_utf8_lossy(up);
        for e in emails {
            assert!(!up.contains(e), "upstream saw {e}: {up}");
        }
        assert!(
            up.contains("[EMAIL_0001]") && up.contains("[EMAIL_0003]"),
            "{up}"
        );
    }
    let v: Value = resp.json().await.unwrap();
    let answer = v["choices"][0]["message"]["content"].as_str().unwrap();
    assert_eq!(answer, format!("Summary: {prompt}"));
}
