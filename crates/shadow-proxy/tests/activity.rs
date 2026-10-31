//! Activity feed end to end: events recorded on mask / block / clean, the summary API, the
//! dashboard, the loopback guard, and that no real value or fake ever lands in an event.

use std::sync::Arc;
use std::time::Duration;

use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use serde_json::{json, Value};
use shadow_core::config::FileAction;
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
    "Maria Garcia",
    "Project Falcon",
    "acme-industries",
];

async fn echo(body: String) -> Response {
    let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let content = v["messages"][0]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();
    let reply = format!("Sure. Summary: {content}");
    if v["stream"].as_bool() == Some(true) {
        let mut sse = String::new();
        let chars: Vec<char> = reply.chars().collect();
        for piece in chars.chunks(4) {
            let p: String = piece.iter().collect();
            let ev = json!({"choices":[{"index":0,"delta":{"content":p},"finish_reason":null}]});
            sse.push_str(&format!("data: {ev}\n\n"));
        }
        sse.push_str("data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\ndata: [DONE]\n\n");
        ([(header::CONTENT_TYPE, "text/event-stream")], sse).into_response()
    } else {
        axum::Json(json!({"choices":[{"index":0,"message":{"role":"assistant","content":reply}}]}))
            .into_response()
    }
}

struct Env {
    dir: tempfile::TempDir,
    url: String,
    shadow: Shadow,
    ca_pem: String,
}

async fn setup(tweak: impl FnOnce(&mut ShadowConfig)) -> Env {
    let app = Router::new()
        .route("/v1/chat/completions", post(echo))
        .route("/v1/embeddings", post(echo));
    let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let up = upstream.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(upstream, app).await.unwrap() });

    let dir = tempfile::tempdir().unwrap();
    let mut cfg = ShadowConfig::from_dir(dir.path())
        .with_region("CN")
        .with_terms(vec!["Acme GmbH".into(), "Project Falcon".into()]);
    cfg.detectors.local_facts = false;
    cfg.files.images = FileAction::Block;
    cfg.proxy
        .routes
        .insert("mock".into(), format!("http://{up}"));
    tweak(&mut cfg);
    let shadow = Shadow::open(cfg).unwrap();
    let proxy = Proxy::new(shadow.clone())
        .unwrap()
        .with_upstream_override("api.openai.com", &format!("http://{up}"));
    let ca_pem = proxy.ca().pem();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    tokio::spawn(Arc::new(proxy).serve(listener));
    Env {
        dir,
        url,
        shadow,
        ca_pem,
    }
}

fn client() -> reqwest::Client {
    reqwest::Client::builder().no_proxy().build().unwrap()
}

async fn get_json(url: &str) -> Value {
    client()
        .get(url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap()
}

/// Events arrive when a response finishes; poll briefly for `n` of them.
async fn events(env: &Env, n: usize) -> Vec<Value> {
    for _ in 0..50 {
        let v = get_json(&format!("{}/shadow/activity?limit=50", env.url)).await;
        let evs = v["events"].as_array().cloned().unwrap_or_default();
        if evs.len() >= n {
            return evs;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    panic!("expected {n} activity events");
}

#[tokio::test]
async fn records_masked_streamed_blocked_and_clean_requests() {
    let env = setup(|_| {}).await;
    let c = client();
    let body = json!({"model":"gpt-4o-mini","user":"john@acme-industries.com","messages":[{"role":"user","content":PROMPT}]});

    // 1. masked, JSON response
    let r = c
        .post(format!("{}/gw/mock/v1/chat/completions", env.url))
        .header(header::USER_AGENT, "OpenAI/Python 1.51.0")
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    r.text().await.unwrap();

    // 2. masked, streamed
    let mut sbody = body.clone();
    sbody["stream"] = json!(true);
    let r = c
        .post(format!("{}/gw/mock/v1/chat/completions", env.url))
        .header(header::USER_AGENT, "claude-cli/2.0.14 (external, cli)")
        .json(&sbody)
        .send()
        .await
        .unwrap();
    let text = r.text().await.unwrap();
    assert!(text.contains("John Smith"), "stream restored");

    // 3. blocked by file policy
    let img = json!({"model":"m","messages":[{"role":"user","content":[{"type":"image_url","image_url":{"url":"data:image/png;base64,iVBORw0KGgoAAAANSUhEUg=="}}]}]});
    let r = c
        .post(format!("{}/gw/mock/v1/chat/completions", env.url))
        .header(header::USER_AGENT, "curl/8.7.1")
        .json(&img)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 403);

    // 4. clean, through the MITM forward proxy to a known provider
    let mitm = reqwest::Client::builder()
        .proxy(reqwest::Proxy::https(&env.url).unwrap())
        .add_root_certificate(reqwest::Certificate::from_pem(env.ca_pem.as_bytes()).unwrap())
        .user_agent("Cursor/0.42.3")
        .build()
        .unwrap();
    let r = mitm
        .post("https://api.openai.com/v1/embeddings")
        .json(&json!({"model":"text-embedding-3-small","input":"the weather is nice"}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);

    let evs = events(&env, 4).await;
    let find = |client: &str| {
        evs.iter()
            .find(|e| e["client"] == client)
            .unwrap_or_else(|| panic!("no event for {client}: {evs:?}"))
    };

    let e = find("OpenAI Python SDK");
    assert_eq!(e["action"], "masked");
    assert_eq!(e["mode"], "gateway");
    assert_eq!(e["provider"], "local");
    assert_eq!(e["endpoint"], "chat");
    assert_eq!(e["status"], 200);
    assert_eq!(e["streamed"], false);
    assert!(e["kinds"]["email"].as_u64().unwrap() >= 1);
    assert!(e["kinds"]["credit_card"].as_u64().unwrap() >= 1);
    assert!(e["kinds"]["secret"].as_u64().unwrap() >= 1);
    assert_eq!(e["kinds"]["identifier"], 1, "the `user` field");
    assert!(
        e["restored"].as_u64().unwrap() >= 5,
        "response restored: {e}"
    );
    assert!(!e["detectors"].as_array().unwrap().is_empty());

    let e = find("Claude Code");
    assert_eq!(e["action"], "masked");
    assert_eq!(e["streamed"], true);
    assert!(e["restored"].as_u64().unwrap() >= 5, "stream restored: {e}");
    assert!(e["bytes_out"].as_u64().unwrap() > 0);

    let e = find("curl");
    assert_eq!(e["action"], "blocked");
    assert_eq!(e["block_reason"], "image_blocked");
    assert_eq!(e["status"], 403);

    let e = find("Cursor");
    assert_eq!(e["action"], "clean");
    assert_eq!(e["provider"], "openai");
    assert_eq!(e["host"], "api.openai.com");
    assert_eq!(e["mode"], "proxy");
    assert_eq!(e["endpoint"], "embeddings");

    // Summary.
    let s = get_json(&format!("{}/shadow/activity/summary?window=1h", env.url)).await;
    assert_eq!(s["enabled"], true);
    assert_eq!(s["window"], "1h");
    assert_eq!(s["totals"]["requests"], 4);
    assert_eq!(s["totals"]["masked_requests"], 2);
    assert_eq!(s["totals"]["blocked"], 1);
    assert_eq!(s["totals"]["clean"], 1);
    assert!(s["totals"]["values_protected"].as_u64().unwrap() >= 10);
    assert_eq!(s["timeline"].as_array().unwrap().len(), 60);
    assert!(s["providers"]
        .as_array()
        .unwrap()
        .iter()
        .any(|p| p["label"] == "OpenAI"));
    assert_eq!(s["block_reasons"][0]["key"], "image_blocked");
    assert!(s["kinds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|k| k["key"] == "email"));

    // Incidents filter.
    let inc = get_json(&format!("{}/shadow/activity?action=incidents", env.url)).await;
    assert_eq!(inc["events"].as_array().unwrap().len(), 3);
    let bad = client()
        .get(format!("{}/shadow/activity/summary?window=2y", env.url))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 400);

    // Nothing sensitive anywhere: API responses and the file on disk.
    let api = format!(
        "{}{}",
        get_json(&format!("{}/shadow/activity?limit=100", env.url)).await,
        s
    );
    let disk_path = env.dir.path().join("activity/activity.jsonl");
    let mut disk = String::new();
    for _ in 0..50 {
        disk = std::fs::read_to_string(&disk_path).unwrap_or_default();
        if disk.lines().count() >= 4 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    assert_eq!(disk.lines().count(), 4);
    let fakes: Vec<String> = env
        .shadow
        .export()
        .into_iter()
        .map(|e| e.fake)
        .filter(|f| f.len() >= 5)
        .collect();
    assert!(!fakes.is_empty());
    for hay in [&api, &disk] {
        for r in REALS {
            assert!(!hay.contains(r), "real value {r:?} leaked into activity");
        }
        for f in &fakes {
            assert!(!hay.contains(f.as_str()), "fake {f:?} leaked into activity");
        }
        assert!(!hay.contains("_0001]"), "token leaked into activity");
        assert!(!hay.contains("weather"), "prompt text leaked into activity");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let m = std::fs::metadata(&disk_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(m, 0o600);
    }
}

#[tokio::test]
async fn dashboard_is_served_with_strict_csp() {
    let env = setup(|_| {}).await;
    let r = client()
        .get(format!("{}/shadow/ui", env.url))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let csp = r.headers()["content-security-policy"]
        .to_str()
        .unwrap()
        .to_string();
    assert!(csp.contains("default-src 'none'") && csp.contains("script-src 'self'"));
    assert!(!csp.contains("unsafe"));
    let html = r.text().await.unwrap();
    assert!(html.contains("/shadow/ui/app.js"));
    assert!(
        !html.contains("src=\"http") && !html.contains("stylesheet\" href=\"http"),
        "no external scripts, styles or images"
    );
    for (path, ct) in [
        ("app.js", "text/javascript"),
        ("app.css", "text/css"),
        ("logo.svg", "image/svg+xml"),
    ] {
        let r = client()
            .get(format!("{}/shadow/ui/{path}", env.url))
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 200);
        assert!(r.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with(ct));
    }
}

#[tokio::test]
async fn dashboard_rejects_foreign_host_headers_unless_allowed() {
    let env = setup(|_| {}).await;
    for path in ["/shadow/ui", "/shadow/activity", "/shadow/activity/summary"] {
        let r = client()
            .get(format!("{}{path}", env.url))
            .header(header::HOST, "attacker.example.com")
            .send()
            .await
            .unwrap();
        assert_eq!(r.status(), 403, "{path}");
    }
    let open = setup(|c| c.activity.allow_remote = true).await;
    let r = client()
        .get(format!("{}/shadow/activity/summary", open.url))
        .header(header::HOST, "shadow.internal:8877")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
}

#[tokio::test]
async fn disabled_activity_records_nothing() {
    let env = setup(|c| c.activity.enabled = false).await;
    let r = client()
        .post(format!("{}/gw/mock/v1/chat/completions", env.url))
        .json(&json!({"model":"m","messages":[{"role":"user","content":PROMPT}]}))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    let s = get_json(&format!("{}/shadow/activity/summary", env.url)).await;
    assert_eq!(s["enabled"], false);
    assert_eq!(s["totals"]["requests"], 0);
    assert!(!env.dir.path().join("activity").exists());
}

#[tokio::test]
async fn activity_stream_pushes_new_events() {
    let env = setup(|_| {}).await;
    let mut stream = client()
        .get(format!("{}/shadow/activity/stream", env.url))
        .send()
        .await
        .unwrap();
    assert_eq!(stream.headers()["content-type"], "text/event-stream");
    client()
        .post(format!("{}/gw/mock/v1/chat/completions", env.url))
        .json(&json!({"model":"m","messages":[{"role":"user","content":"summarise the ticket from jane@example.org"}]}))
        .send()
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let chunk = tokio::time::timeout(Duration::from_secs(5), stream.chunk())
        .await
        .expect("stream event in time")
        .unwrap()
        .unwrap();
    let text = String::from_utf8_lossy(&chunk);
    assert!(text.starts_with("event: activity"), "{text}");
    assert!(text.contains("\"action\":\"masked\""));
    assert!(!text.contains("jane@example.org"));
}
