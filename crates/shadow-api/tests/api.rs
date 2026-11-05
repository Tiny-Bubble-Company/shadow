use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::{json, Value};
use shadow_api::{router, ApiConfig};
use shadow_core::{Shadow, ShadowConfig};
use tower::ServiceExt;

const PROMPT: &str = "Email jane@acme.com about the Acme GmbH contract";

fn app(dir: &std::path::Path, token: Option<&str>) -> Router {
    let shadow =
        Shadow::open(ShadowConfig::from_dir(dir).with_terms(vec!["Acme GmbH".into()])).unwrap();
    router(
        shadow,
        ApiConfig {
            token: token.map(str::to_string),
            ..ApiConfig::default()
        },
    )
}

async fn call(
    app: &Router,
    method: &str,
    path: &str,
    body: Option<Value>,
    token: Option<&str>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(path);
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_string())),
        None => req.body(Body::empty()),
    }
    .unwrap();
    let res = app.clone().oneshot(req).await.unwrap();
    let status = res.status();
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn protect_then_restore_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), None);
    let (s, p) = call(
        &app,
        "POST",
        "/v1/protect",
        Some(json!({"text": PROMPT})),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{p}");
    let masked = p["text"].as_str().unwrap();
    assert!(
        !masked.contains("jane@acme.com") && !masked.contains("Acme GmbH"),
        "{masked}"
    );
    assert!(p["replacements"].as_u64().unwrap() >= 2);
    let id = p["session_id"].as_str().unwrap();
    assert!(id.starts_with("ses_"));

    let answer = format!("Done: {masked}");
    let (s, r) = call(
        &app,
        "POST",
        "/v1/restore",
        Some(json!({"text": answer, "session_id": id})),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(r["text"], format!("Done: {PROMPT}"));

    // Fakes are in the vault: they restore without the session too.
    let (_, r) = call(
        &app,
        "POST",
        "/v1/restore",
        Some(json!({"text": masked})),
        None,
    )
    .await;
    assert_eq!(r["text"], PROMPT);
}

#[tokio::test]
async fn protects_every_string_in_json() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), None);
    let body = json!({"messages": [{"role": "user", "content": PROMPT}], "model": "x"});
    let (s, p) = call(
        &app,
        "POST",
        "/v1/protect",
        Some(json!({"json": body})),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let sent = p["json"].to_string();
    assert!(!sent.contains("jane@acme.com"), "{sent}");
    let (_, r) = call(
        &app,
        "POST",
        "/v1/restore",
        Some(json!({"json": p["json"], "session_id": p["session_id"]})),
        None,
    )
    .await;
    assert_eq!(r["json"], body);
}

#[tokio::test]
async fn scan_shortens_values_unless_revealed() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), None);
    let (_, r) = call(
        &app,
        "POST",
        "/v1/scan",
        Some(json!({"text": PROMPT})),
        None,
    )
    .await;
    assert_eq!(r["counts"]["email"], 1);
    assert!(!r.to_string().contains("jane@acme.com"));
    let (_, r) = call(
        &app,
        "POST",
        "/v1/scan",
        Some(json!({"text": PROMPT, "reveal": true})),
        None,
    )
    .await;
    assert!(r.to_string().contains("jane@acme.com"));
}

#[tokio::test]
async fn rejects_bad_requests() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), None);
    let (s, e) = call(
        &app,
        "POST",
        "/v1/protect",
        Some(json!({"text": "a", "json": {}})),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::BAD_REQUEST);
    assert_eq!(e["error"]["code"], "invalid_request");
    let (s, e) = call(
        &app,
        "POST",
        "/v1/restore",
        Some(json!({"text": "a", "session_id": "ses_nope"})),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    assert_eq!(e["error"]["code"], "session_not_found");
    let (s, _) = call(&app, "POST", "/v1/scan", Some(json!({"txt": "a"})), None).await;
    assert_eq!(s, StatusCode::UNPROCESSABLE_ENTITY);
    let (s, _) = call(&app, "GET", "/v1/nothing", None, None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn sessions_can_be_ended() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), None);
    let (_, p) = call(
        &app,
        "POST",
        "/v1/protect",
        Some(json!({"text": PROMPT})),
        None,
    )
    .await;
    let path = format!("/v1/sessions/{}", p["session_id"].as_str().unwrap());
    assert_eq!(
        call(&app, "DELETE", &path, None, None).await.0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        call(&app, "DELETE", &path, None, None).await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn token_is_required_when_set() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), Some("s3cret"));
    assert_eq!(
        call(&app, "GET", "/v1/health", None, None).await.0,
        StatusCode::OK
    );
    assert_eq!(
        call(&app, "GET", "/v1/openapi.json", None, None).await.0,
        StatusCode::OK
    );
    let (s, e) = call(
        &app,
        "POST",
        "/v1/scan",
        Some(json!({"text": PROMPT})),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    assert_eq!(e["error"]["code"], "unauthorized");
    assert_eq!(
        call(
            &app,
            "POST",
            "/v1/scan",
            Some(json!({"text": PROMPT})),
            Some("wrong")
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/v1/scan",
            Some(json!({"text": PROMPT})),
            Some("s3cret")
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        call(
            &app,
            "POST",
            "/mcp",
            Some(json!({"jsonrpc": "2.0", "id": 1, "method": "ping"})),
            None
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn mcp_tools_round_trip() {
    let dir = tempfile::tempdir().unwrap();
    let app = app(dir.path(), None);
    let rpc = |id: u64, method: &str, params: Value| json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});

    let (_, r) = call(
        &app,
        "POST",
        "/mcp",
        Some(rpc(
            1,
            "initialize",
            json!({"protocolVersion": "2025-03-26"}),
        )),
        None,
    )
    .await;
    assert_eq!(r["result"]["protocolVersion"], "2025-03-26");
    assert_eq!(r["result"]["serverInfo"]["name"], "shadow");

    let (s, _) = call(
        &app,
        "POST",
        "/mcp",
        Some(json!({"jsonrpc": "2.0", "method": "notifications/initialized"})),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::ACCEPTED);

    let (_, r) = call(
        &app,
        "POST",
        "/mcp",
        Some(rpc(2, "tools/list", json!({}))),
        None,
    )
    .await;
    let names: Vec<_> = r["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["shadow_scan", "shadow_protect", "shadow_restore"]);

    let (_, r) = call(
        &app,
        "POST",
        "/mcp",
        Some(rpc(
            3,
            "tools/call",
            json!({"name": "shadow_protect", "arguments": {"text": PROMPT}}),
        )),
        None,
    )
    .await;
    let out = &r["result"]["structuredContent"];
    assert_eq!(r["result"]["isError"], false);
    let masked = out["text"].as_str().unwrap();
    assert!(!masked.contains("jane@acme.com"));

    let (_, r) = call(&app, "POST", "/mcp", Some(rpc(4, "tools/call", json!({"name": "shadow_restore", "arguments": {"text": masked, "session_id": out["session_id"]}}))), None).await;
    assert_eq!(r["result"]["structuredContent"]["text"], PROMPT);

    let (_, r) = call(
        &app,
        "POST",
        "/mcp",
        Some(rpc(
            5,
            "tools/call",
            json!({"name": "shadow_restore", "arguments": {"text": "x", "session_id": "ses_nope"}}),
        )),
        None,
    )
    .await;
    assert_eq!(r["result"]["isError"], true);

    let (_, r) = call(
        &app,
        "POST",
        "/mcp",
        Some(rpc(
            6,
            "tools/call",
            json!({"name": "nope", "arguments": {}}),
        )),
        None,
    )
    .await;
    assert_eq!(r["error"]["code"], -32602);
    let (_, r) = call(
        &app,
        "POST",
        "/mcp",
        Some(rpc(7, "resources/list", json!({}))),
        None,
    )
    .await;
    assert_eq!(r["error"]["code"], -32601);
}

#[test]
fn openapi_is_valid_json_and_lists_every_route() {
    let v: Value = serde_json::from_str(shadow_api::OPENAPI).unwrap();
    for p in [
        "/v1/health",
        "/v1/info",
        "/v1/scan",
        "/v1/protect",
        "/v1/restore",
        "/v1/sessions/{id}",
        "/mcp",
    ] {
        assert!(v["paths"].get(p).is_some(), "{p} missing from openapi.json");
    }
}
