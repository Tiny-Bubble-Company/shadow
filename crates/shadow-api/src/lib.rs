//! # shadow-api
//!
//! Shadow Connect: Shadow's privacy layer as an HTTP API and an MCP server, so other
//! platforms and agents can use it without running the proxy.
//!
//! * `POST /v1/scan`: report sensitive values without changing anything.
//! * `POST /v1/protect`: replace them with consistent fakes (text or JSON).
//! * `POST /v1/restore`: translate an answer back to the real values.
//! * `POST /mcp`: the same three operations as MCP tools (streamable HTTP, JSON responses).
//!
//! The API runs next to the caller, on the customer's side: real values never leave it.
//! On a non-loopback address a bearer token is required (see [`ApiConfig::token`]).

mod error;
mod mcp;
mod privacy;
mod sessions;

use std::sync::Arc;
use std::time::Duration;

use axum::extract::{DefaultBodyLimit, Request, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use shadow_core::Shadow;
use subtle::ConstantTimeEq;
use tokio::net::TcpListener;

pub use error::ApiError;
pub use sessions::SessionStore;

/// Default listen address of `shadow serve`.
pub const DEFAULT_LISTEN: &str = "127.0.0.1:8878";

/// Largest request body accepted (bytes).
pub const MAX_BODY: usize = 4 * 1024 * 1024;

/// The OpenAPI 3.1 description of the HTTP API, served at `/v1/openapi.json`.
pub const OPENAPI: &str = include_str!("openapi.json");

/// API settings.
#[derive(Clone, Debug)]
pub struct ApiConfig {
    /// Bearer token every call except `/v1/health` must present. Required when listening
    /// on a non-loopback address.
    pub token: Option<String>,
    /// How long an unused session (and the tokens it handed out) is kept.
    pub session_ttl: Duration,
    /// Most sessions kept at once; the least recently used are dropped first.
    pub max_sessions: usize,
}

impl Default for ApiConfig {
    fn default() -> Self {
        ApiConfig {
            token: None,
            session_ttl: Duration::from_secs(3600),
            max_sessions: 10_000,
        }
    }
}

pub(crate) struct AppState {
    pub shadow: Shadow,
    pub sessions: SessionStore,
    token_hash: Option<[u8; 32]>,
}

pub(crate) type Shared = Arc<AppState>;

/// The API's routes, ready to serve or to mount inside another axum app.
pub fn router(shadow: Shadow, cfg: ApiConfig) -> Router {
    let state: Shared = Arc::new(AppState {
        shadow,
        sessions: SessionStore::new(cfg.session_ttl, cfg.max_sessions),
        token_hash: cfg.token.as_deref().map(hash),
    });
    let protected = Router::new()
        .route("/v1", get(index))
        .route("/v1/info", get(info))
        .route("/v1/scan", post(privacy::scan))
        .route("/v1/protect", post(privacy::protect))
        .route("/v1/restore", post(privacy::restore))
        .route("/v1/sessions/:id", delete(privacy::end_session))
        .route("/mcp", post(mcp::handle).get(mcp::not_allowed))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth));
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/openapi.json", get(openapi))
        .merge(protected)
        .fallback(|| async { ApiError::not_found("no such endpoint; see /v1/openapi.json") })
        .layer(DefaultBodyLimit::max(MAX_BODY))
        .with_state(state)
}

/// Serve the API on `listener` until the task is cancelled.
pub async fn serve(listener: TcpListener, shadow: Shadow, cfg: ApiConfig) -> anyhow::Result<()> {
    let addr = listener.local_addr()?;
    if !addr.ip().is_loopback() && cfg.token.is_none() {
        anyhow::bail!("{addr} is reachable from other machines: set an API token first");
    }
    tracing::info!("Shadow Connect API on http://{addr}/v1 (MCP at /mcp)");
    axum::serve(listener, router(shadow, cfg)).await?;
    Ok(())
}

fn hash(token: &str) -> [u8; 32] {
    Sha256::digest(token.as_bytes()).into()
}

async fn auth(State(st): State<Shared>, req: Request, next: Next) -> Response {
    if let Some(want) = &st.token_hash {
        let given = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|t| hash(t.trim()));
        let ok = given.is_some_and(|g| bool::from(g.ct_eq(want)));
        if !ok {
            let mut r = ApiError::new(
                StatusCode::UNAUTHORIZED,
                "unauthorized",
                "missing or wrong bearer token",
            )
            .into_response();
            r.headers_mut()
                .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
            return r;
        }
    }
    next.run(req).await
}

async fn health() -> Json<Value> {
    Json(json!({"status": "ok", "version": env!("CARGO_PKG_VERSION")}))
}

async fn openapi() -> impl IntoResponse {
    ([(header::CONTENT_TYPE, "application/json")], OPENAPI)
}

async fn index() -> Json<Value> {
    Json(json!({
        "name": "Shadow Connect API",
        "version": env!("CARGO_PKG_VERSION"),
        "endpoints": {
            "GET /v1/health": "Liveness (no token needed)",
            "GET /v1/info": "Persona, vault and session counts",
            "POST /v1/scan": "Find sensitive values without changing anything",
            "POST /v1/protect": "Replace sensitive values with consistent fakes",
            "POST /v1/restore": "Translate fakes back to the real values",
            "DELETE /v1/sessions/{id}": "Forget a session and its tokens",
            "POST /mcp": "The same operations as MCP tools",
            "GET /v1/openapi.json": "OpenAPI 3.1 description",
        },
        "docs": "https://getshadow.dev/connect/",
    }))
}

async fn info(State(st): State<Shared>) -> Json<Value> {
    let p = st.shadow.persona();
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "persona": {"company": p.company, "full_name": p.full_name, "region": p.region, "domain": p.domain},
        "vault": st.shadow.stats(),
        "sessions": st.sessions.len(),
    }))
}
