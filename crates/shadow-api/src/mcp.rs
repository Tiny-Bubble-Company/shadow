//! MCP server over streamable HTTP: one JSON-RPC message per POST, answered with JSON.
//! Tools: `shadow_scan`, `shadow_protect`, `shadow_restore`.

use axum::extract::State;
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::{json, Value};

use crate::privacy::{self, ProtectRequest, RestoreRequest, ScanRequest};
use crate::{ApiError, Shared};

/// Protocol versions this server speaks; the client's is echoed when known.
const VERSIONS: &[&str] = &["2025-06-18", "2025-03-26", "2024-11-05"];

pub async fn not_allowed() -> Response {
    (
        StatusCode::METHOD_NOT_ALLOWED,
        [(header::ALLOW, "POST")],
        "this MCP server answers POST requests only",
    )
        .into_response()
}

pub async fn handle(State(st): State<Shared>, body: axum::body::Bytes) -> Response {
    let msg: Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return rpc_error(Value::Null, -32700, &format!("parse error: {e}")),
    };
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    let Some(id) = msg.get("id").cloned() else {
        // Notifications (`notifications/initialized` and the like) and responses: accepted.
        return StatusCode::ACCEPTED.into_response();
    };
    let params = msg.get("params").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => {
            let asked = params.get("protocolVersion").and_then(Value::as_str);
            let version = asked
                .filter(|v| VERSIONS.contains(v))
                .unwrap_or(VERSIONS[0]);
            rpc_result(
                id,
                json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {"listChanged": false}},
                    "serverInfo": {"name": "shadow", "title": "Shadow", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": "Call shadow_protect on text before sending it to another AI service or tool, and shadow_restore on the answer with the same session_id. shadow_scan reports sensitive values without changing anything.",
                }),
            )
        }
        "ping" => rpc_result(id, json!({})),
        "tools/list" => rpc_result(id, json!({"tools": tools()})),
        "tools/call" => {
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            match call(&st, name, args) {
                Ok(v) => rpc_result(id, tool_result(v, false)),
                Err(CallError::Unknown) => rpc_error(id, -32602, &format!("unknown tool: {name}")),
                Err(CallError::Failed(e)) => rpc_result(id, tool_result(json!({"error": e}), true)),
            }
        }
        _ => rpc_error(id, -32601, &format!("method not found: {method}")),
    }
}

enum CallError {
    Unknown,
    Failed(String),
}

fn call(st: &Shared, name: &str, args: Value) -> Result<Value, CallError> {
    let bad = |e: serde_json::Error| CallError::Failed(format!("invalid arguments: {e}"));
    let failed = |e: ApiError| CallError::Failed(e.message);
    match name {
        "shadow_scan" => Ok(privacy::do_scan(
            st,
            serde_json::from_value::<ScanRequest>(args).map_err(bad)?,
        )),
        "shadow_protect" => privacy::do_protect(
            st,
            serde_json::from_value::<ProtectRequest>(args).map_err(bad)?,
        )
        .map_err(failed),
        "shadow_restore" => privacy::do_restore(
            st,
            serde_json::from_value::<RestoreRequest>(args).map_err(bad)?,
        )
        .map_err(failed),
        _ => Err(CallError::Unknown),
    }
}

fn tool_result(v: Value, is_error: bool) -> Value {
    json!({
        "content": [{"type": "text", "text": v.to_string()}],
        "structuredContent": v,
        "isError": is_error,
    })
}

fn tools() -> Value {
    let session =
        json!({"type": "string", "description": "session_id from an earlier shadow_protect call"});
    json!([
        {
            "name": "shadow_scan",
            "title": "Find sensitive data",
            "description": "Report personal data, secrets and company terms in text without changing it. Values come back shortened unless reveal is true.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "reveal": {"type": "boolean", "default": false},
                },
                "required": ["text"],
            },
            "annotations": {"readOnlyHint": true, "openWorldHint": false},
        },
        {
            "name": "shadow_protect",
            "title": "Mask sensitive data",
            "description": "Replace personal data, secrets and company terms with consistent fakes before the text goes to another AI service or tool. Returns the masked text and a session_id for shadow_restore.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "session_id": session,
                    "instruction": {"type": "string", "description": "The user's request, if text is a document rather than the request itself"},
                    "policy": {"type": "string", "enum": ["fakes", "intent"], "default": "fakes"},
                },
                "required": ["text"],
            },
            "annotations": {"readOnlyHint": false, "destructiveHint": false, "idempotentHint": true, "openWorldHint": false},
        },
        {
            "name": "shadow_restore",
            "title": "Restore real values",
            "description": "Translate fakes in an answer back to the real values. Pass the session_id from shadow_protect.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string"},
                    "session_id": session,
                },
                "required": ["text"],
            },
            "annotations": {"readOnlyHint": true, "openWorldHint": false},
        },
    ])
}

fn rpc_result(id: Value, result: Value) -> Response {
    Json(json!({"jsonrpc": "2.0", "id": id, "result": result})).into_response()
}

fn rpc_error(id: Value, code: i64, message: &str) -> Response {
    Json(json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}))
        .into_response()
}
