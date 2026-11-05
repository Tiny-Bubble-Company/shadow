//! Scan, protect and restore: the HTTP handlers and the operations behind them (the MCP
//! tools call the same operations).

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use shadow_core::{RequestContext, Session, ToolInfo};

use crate::{ApiError, AppState, Shared};

type ApiJson<T> = Result<Json<T>, JsonRejection>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScanRequest {
    pub text: String,
    /// Return each value in full instead of a shortened preview.
    #[serde(default)]
    pub reveal: bool,
}

/// How a new session replaces values.
#[derive(Deserialize, Default, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    /// Persona fakes everywhere: consistent across calls and restorable without a session.
    #[default]
    Fakes,
    /// Shadow's intent policy: fakes, typed tokens (restorable only within the session)
    /// or kept values, depending on what the instruction asks for.
    Intent,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Tool {
    pub name: String,
    #[serde(default)]
    pub remote: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectRequest {
    pub text: Option<String>,
    pub json: Option<Value>,
    /// Continue a session from an earlier call.
    pub session_id: Option<String>,
    /// The user's instruction, for the intent policy (defaults to `text` with `policy: intent`).
    pub instruction: Option<String>,
    /// Tools the model may call, for the intent policy.
    #[serde(default)]
    pub tools: Vec<Tool>,
    #[serde(default)]
    pub policy: Policy,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreRequest {
    pub text: Option<String>,
    pub json: Option<Value>,
    pub session_id: Option<String>,
}

/// Exactly one of `text` and `json`.
enum Payload {
    Text(String),
    Json(Value),
}

fn payload(text: Option<String>, json: Option<Value>) -> Result<Payload, ApiError> {
    match (text, json) {
        (Some(t), None) => Ok(Payload::Text(t)),
        (None, Some(j)) => Ok(Payload::Json(j)),
        _ => Err(ApiError::bad_request(
            "send exactly one of `text` and `json`",
        )),
    }
}

/// A shortened preview of a sensitive value (`jo********om`).
pub fn redact(v: &str) -> String {
    let n = v.chars().count();
    if n <= 4 {
        return "*".repeat(n);
    }
    let head: String = v.chars().take(2).collect();
    let tail: String = v.chars().skip(n - 2).collect();
    format!("{head}{}{tail}", "*".repeat((n - 4).min(12)))
}

pub fn do_scan(st: &AppState, req: ScanRequest) -> Value {
    let found = st.shadow.scan(&req.text);
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    let findings: Vec<Value> = found
        .iter()
        .map(|f| {
            *counts.entry(f.kind.as_str()).or_default() += 1;
            let v = &req.text[f.start..f.end];
            json!({
                "kind": f.kind.as_str(),
                "detector": f.detector,
                "start": f.start,
                "end": f.end,
                "score": f.score,
                "value": if req.reveal { v.to_string() } else { redact(v) },
            })
        })
        .collect();
    json!({"findings": findings, "counts": counts})
}

fn session_for(st: &AppState, req: &ProtectRequest) -> Result<(String, Arc<Session>), ApiError> {
    if let Some(id) = &req.session_id {
        return st
            .sessions
            .get(id)
            .map(|s| (id.clone(), s))
            .ok_or_else(ApiError::session_not_found);
    }
    // Only the intent policy reads the text as the instruction: it keeps the recipient of
    // "email X" readable, which a plain masking call should not.
    let instruction = req
        .instruction
        .clone()
        .or_else(|| {
            (req.policy == Policy::Intent)
                .then(|| req.text.clone())
                .flatten()
        })
        .unwrap_or_default();
    let ctx = RequestContext {
        instruction,
        tools: req
            .tools
            .iter()
            .map(|t| ToolInfo {
                name: t.name.clone(),
                remote: t.remote,
            })
            .collect(),
    };
    let mut session = st.shadow.session(ctx);
    if req.policy == Policy::Fakes {
        session = session.surrogates_only();
    }
    Ok(st.sessions.insert(session))
}

pub fn do_protect(st: &AppState, req: ProtectRequest) -> Result<Value, ApiError> {
    let (id, session) = session_for(st, &req)?;
    let mut out = json!({"session_id": id});
    match payload(req.text, req.json)? {
        Payload::Text(t) => {
            let m = session.mask(&t);
            out["text"] = Value::String(m.text);
            out["replacements"] = json!(m.replacements);
            out["kinds"] = json!(m.kinds);
        }
        Payload::Json(mut v) => {
            let n = session.mask_json(&mut v);
            out["json"] = v;
            out["replacements"] = json!(n);
        }
    }
    out["intent"] = json!(session.classification().intent.as_str());
    out["summary"] = json!(session.summary());
    Ok(out)
}

pub fn do_restore(st: &AppState, req: RestoreRequest) -> Result<Value, ApiError> {
    let session = match &req.session_id {
        Some(id) => Some(
            st.sessions
                .get(id)
                .ok_or_else(ApiError::session_not_found)?,
        ),
        None => None,
    };
    Ok(match payload(req.text, req.json)? {
        Payload::Text(t) => {
            let (text, n) = match &session {
                Some(s) => s.unmask_counted(&t),
                None => st.shadow.unmask_counted(&t),
            };
            json!({"text": text, "restored": n})
        }
        Payload::Json(mut v) => {
            let n = match &session {
                Some(s) => s.unmask_json(&mut v),
                None => st.shadow.unmask_json(&mut v),
            };
            json!({"json": v, "restored": n})
        }
    })
}

pub async fn scan(
    State(st): State<Shared>,
    body: ApiJson<ScanRequest>,
) -> Result<Json<Value>, ApiError> {
    let Json(req) = body?;
    Ok(Json(do_scan(&st, req)))
}

pub async fn protect(
    State(st): State<Shared>,
    body: ApiJson<ProtectRequest>,
) -> Result<Json<Value>, ApiError> {
    let Json(req) = body?;
    do_protect(&st, req).map(Json)
}

pub async fn restore(
    State(st): State<Shared>,
    body: ApiJson<RestoreRequest>,
) -> Result<Json<Value>, ApiError> {
    let Json(req) = body?;
    do_restore(&st, req).map(Json)
}

pub async fn end_session(
    State(st): State<Shared>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    if st.sessions.remove(&id) {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::session_not_found())
    }
}
