//! `shadow receipts`: check this machine's evidence receipts (see `shadow_core::receipt`),
//! show one, or verify a downloaded receipt offline.

use anyhow::Context;
use base64::Engine;
use serde_json::Value;
use shadow_core::{receipt, ShadowConfig};

use crate::activity::{http_get, Source};
use crate::ui;

fn addr(cfg: &ShadowConfig) -> String {
    Source::live(cfg).addr().to_string()
}

/// Check every receipt of `window` in chain order.
pub fn chain(cfg: &ShadowConfig, window: &str, json: bool) -> anyhow::Result<()> {
    let body = http_get(
        &addr(cfg),
        &format!("/shadow/activity/receipts?window={window}"),
    )?;
    if json {
        println!("{body}");
        return Ok(());
    }
    let r: Value = serde_json::from_str(&body)?;
    let n = |k: &str| r[k].as_u64().unwrap_or(0);
    ui::header("Evidence receipts");
    ui::kv("Signing key", r["key_id"].as_str().unwrap_or(""));
    if n("checked") == 0 {
        ui::kv("Receipts", "none yet in this period");
        return Ok(());
    }
    ui::kv(
        "Chain",
        &format!(
            "{} to {}{}",
            n("first_sequence"),
            n("last_sequence"),
            if r["starts_mid_chain"] == true {
                " (earlier receipts are outside this period or expired)"
            } else {
                ""
            }
        ),
    );
    let problems = r["problems"].as_array().cloned().unwrap_or_default();
    if n("valid") == n("checked") {
        ui::ok(&format!(
            "{} receipts checked, every signature valid",
            n("checked")
        ));
    } else {
        ui::warn(&format!(
            "{} of {} receipts failed the signature check",
            n("checked") - n("valid"),
            n("checked")
        ));
    }
    if problems.is_empty() {
        ui::ok("every receipt links to the one before: none missing or changed");
        return Ok(());
    }
    if n("missing") > 0 {
        ui::warn(&format!("{} receipts missing from the chain", n("missing")));
    }
    for p in &problems {
        println!(
            "  {} request #{}: {}",
            ui::red("✕"),
            p["event_id"],
            p["error"]["message"].as_str().unwrap_or("")
        );
    }
    anyhow::bail!("the receipt chain has {} problem(s)", problems.len())
}

/// Show the receipt of request `id`, as checked by the running proxy.
pub fn show(cfg: &ShadowConfig, id: u64, json: bool) -> anyhow::Result<()> {
    let body = http_get(&addr(cfg), &format!("/shadow/activity/receipts/{id}"))
        .with_context(|| format!("no receipt for request #{id}"))?;
    let r: Value = serde_json::from_str(&body)?;
    if json {
        let out = serde_json::json!({
            "profile": r["profile"], "receipt": r["receipt"], "public_key": r["public_key"],
        });
        println!("{}", serde_json::to_string_pretty(&out)?);
        return Ok(());
    }
    let c = &r["check"];
    ui::header(&format!("Receipt for request #{id}"));
    print_claims(&c["claims"]);
    ui::kv("Receipt hash", r["hash"].as_str().unwrap_or(""));
    if c["valid"] == true {
        ui::ok(&format!(
            "signature valid (key {})",
            r["key_id"].as_str().unwrap_or("")
        ));
    }
    match c["chained"].as_bool() {
        Some(true) => ui::ok("links to the previous receipt"),
        Some(false) => ui::warn("does not link to the previous receipt"),
        None => println!(
            "  {}",
            ui::dim("previous receipt expired: link not checked")
        ),
    }
    if c["content"]["sent_text"] == true {
        ui::ok("the saved prompt matches the signed hashes");
    }
    if let Some(m) = c["error"]["message"].as_str() {
        anyhow::bail!("receipt check failed: {m}");
    }
    Ok(())
}

/// Verify a `{receipt, public_key}` file offline (layers 1-3).
pub fn verify_file(path: &std::path::Path) -> anyhow::Result<()> {
    let v: Value = serde_json::from_str(
        &std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .context("not a receipt JSON file")?;
    let b64 = base64::engine::general_purpose::STANDARD;
    let bytes = b64
        .decode(v["receipt"].as_str().context("no `receipt` field")?)
        .context("`receipt` is not base64")?;
    let key: [u8; 32] = b64
        .decode(v["public_key"].as_str().context("no `public_key` field")?)
        .ok()
        .and_then(|k| k.try_into().ok())
        .context("`public_key` must be 32 bytes of base64")?;
    ui::header("Receipt");
    match receipt::verify(&bytes, &key) {
        Ok(c) => {
            print_claims(&serde_json::to_value(&c)?);
            ui::kv("Receipt hash", &hex(&receipt::receipt_hash(&bytes)));
            ui::ok(&format!(
                "valid: signed by key {}, unchanged since",
                receipt::key_id(&key)
            ));
            println!(
                "  {}",
                ui::dim("To check it belongs to a machine, compare the key id with the one Shadow Hub shows for it.")
            );
            Ok(())
        }
        Err(e) => anyhow::bail!("not valid ({:?} check): {}", e.layer, e.message),
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn print_claims(c: &Value) {
    let s = |k: &str| c[k].as_str().unwrap_or("").to_string();
    let when = c["event_ts_ms"]
        .as_i64()
        .and_then(chrono::DateTime::from_timestamp_millis)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%Y-%m-%d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default();
    ui::kv("Sequence", &c["sequence"].to_string());
    ui::kv("When", &when);
    ui::kv("Signed by", &s("iss"));
    ui::kv("Action", &s("action"));
    ui::kv("To", &format!("{} ({})", s("host"), s("provider")));
    ui::kv("App", &s("client"));
    let findings = c["findings"]
        .as_object()
        .map(|o| {
            o.iter()
                .map(|(k, v)| format!("{v} {k}"))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    if !findings.is_empty() {
        ui::kv("Found", &findings);
    }
    for (label, k) in [
        ("Sent to AI", "sent_body_hash"),
        ("AI answer", "response_hash"),
        ("Given to app", "delivered_hash"),
        ("Prompt sent", "sent_text_hash"),
        ("Settings", "policy_hash"),
    ] {
        if let Some(h) = c[k].as_str() {
            ui::kv(label, h);
        }
    }
    ui::kv("Shadow", &s("shadow_version"));
}
