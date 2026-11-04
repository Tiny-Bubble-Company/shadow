//! Scan receipts end to end: every request through the proxy gets a signed receipt whose
//! hashes match the bytes the provider received and the bytes the app got back.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::routing::post;
use axum::Router;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use shadow_core::{receipt, Shadow, ShadowConfig};
use shadow_proxy::Proxy;

#[tokio::test]
async fn receipts_bind_the_bytes_sent_and_received() {
    let seen: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
    let sink = seen.clone();
    let app = Router::new().route(
        "/v1/chat/completions",
        post(move |body: axum::body::Bytes| {
            let sink = sink.clone();
            async move {
                sink.lock().unwrap().push(body.to_vec());
                let v: Value = serde_json::from_slice(&body).unwrap_or_default();
                let text = v["messages"][0]["content"].as_str().unwrap_or("").to_string();
                axum::Json(json!({"choices":[{"message":{"role":"assistant","content":format!("Re: {text}")}}]}))
            }
        }),
    );
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up = upstream.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(upstream, app).await.unwrap() });

    let dir = tempfile::tempdir().unwrap();
    let mut cfg = ShadowConfig::from_dir(dir.path()).with_region("CN");
    cfg.detectors.local_facts = false;
    cfg.proxy
        .routes
        .insert("mock".into(), format!("http://{up}"));
    let shadow = Shadow::open(cfg).unwrap();
    let proxy = Proxy::new(shadow).unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(proxy).serve(listener));
    let c = reqwest::Client::builder().no_proxy().build().unwrap();

    let prompt = "Write to ann.miller@acme-industries.com about the invoice.";
    let r = c
        .post(format!("{url}/gw/mock/v1/chat/completions"))
        .json(&json!({"model":"m","messages":[{"role":"user","content":prompt}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let delivered = r.bytes().await.unwrap();
    assert!(String::from_utf8_lossy(&delivered).contains("ann.miller@acme-industries.com"));
    let r = c
        .post(format!("{url}/gw/mock/v1/chat/completions"))
        .json(&json!({"model":"m","messages":[{"role":"user","content":"What is 2 + 2?"}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    r.bytes().await.unwrap();

    let mut events = vec![];
    for _ in 0..50 {
        let v: Value = c
            .get(format!("{url}/shadow/activity?limit=10"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        events = v["events"].as_array().cloned().unwrap_or_default();
        if events.len() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    let masked = events.iter().find(|e| e["action"] == "masked").unwrap();
    let clean = events.iter().find(|e| e["action"] == "clean").unwrap();

    let get = |id: &Value| {
        let c = c.clone();
        let url = format!("{url}/shadow/activity/receipts/{id}");
        async move {
            c.get(url)
                .send()
                .await
                .unwrap()
                .json::<Value>()
                .await
                .unwrap()
        }
    };
    let m = get(&masked["id"]).await;
    assert_eq!(m["check"]["valid"], true, "{m}");
    assert_eq!(m["check"]["chained"], true, "{m}");
    assert_eq!(m["check"]["content"]["sent_text"], true, "{m}");
    assert_eq!(m["check"]["content"]["original_text"], true, "{m}");

    // Verify independently from the bytes and the public key.
    let b64 = base64::engine::general_purpose::STANDARD;
    let bytes = b64.decode(m["receipt"].as_str().unwrap()).unwrap();
    let key: [u8; 32] = b64
        .decode(m["public_key"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    let claims = receipt::verify(&bytes, &key).unwrap();
    let hex = |b: &[u8]| hex::encode(Sha256::digest(b));
    let sent = seen.lock().unwrap()[0].clone();
    assert!(!String::from_utf8_lossy(&sent).contains("ann.miller"));
    assert_eq!(hex::encode(claims.sent_body_hash.unwrap()), hex(&sent));
    assert_eq!(hex::encode(claims.delivered_hash.unwrap()), hex(&delivered));
    assert_eq!(claims.findings.get("email"), Some(&1));
    assert!(!String::from_utf8_lossy(&bytes).contains("ann.miller"));

    // The clean request is the next link in the chain.
    let cl = get(&clean["id"]).await;
    assert_eq!(cl["check"]["valid"], true, "{cl}");
    assert_eq!(cl["check"]["chained"], true, "{cl}");
    let next = receipt::peek(&b64.decode(cl["receipt"].as_str().unwrap()).unwrap()).unwrap();
    assert_eq!(next.sequence, claims.sequence + 1);
    assert_eq!(next.prev_receipt, receipt::receipt_hash(&bytes));
    assert_eq!(next.action, "clean");
    assert!(next.findings.is_empty() && next.sent_text_hash.is_none());
}
