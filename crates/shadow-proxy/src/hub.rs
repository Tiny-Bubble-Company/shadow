//! Reporting to a Shadow Hub, for machines an organisation has enrolled
//! (`shadow enroll <join link>`).
//!
//! Every few seconds the uploader sends new activity events to the hub, with incident
//! details trimmed to the organisation's sharing level **before** they leave the machine:
//! - `full`: events plus the prompt as written and as sent, values caught and replacements;
//! - `sanitised`: events plus only what the AI received (fakes), never real values;
//! - `counts`: events only.
//!
//! The enrolment lives in `<data_dir>/hub.json` (mode 0600). Without it nothing is sent.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use shadow_core::activity::{ActivityLog, Detail, Event};

/// File name of the enrolment inside the data directory.
pub const HUB_FILE: &str = "hub.json";
/// Events per upload (the hub accepts up to 500).
const BATCH: usize = 200;
const INTERVAL: Duration = Duration::from_secs(10);

/// This machine's enrolment with a hub.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Enrollment {
    /// The hub's public origin, e.g. `https://hub.example.com`.
    pub hub_url: String,
    pub device_id: String,
    pub device_token: String,
    pub org_name: String,
    /// `full`, `sanitised` or `counts`; refreshed from the hub on every upload.
    pub share_level: String,
    pub user_email: String,
    /// Highest local event id the hub has accepted.
    #[serde(default)]
    pub cursor: u64,
    /// When this machine was enrolled (unix ms). Activity from before it is never sent.
    #[serde(default)]
    pub enrolled_at: i64,
}

impl Enrollment {
    pub fn path(data_dir: &Path) -> PathBuf {
        data_dir.join(HUB_FILE)
    }

    pub fn load(data_dir: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(Self::path(data_dir)).ok()?;
        serde_json::from_str(&text).ok()
    }

    pub fn save(&self, data_dir: &Path) -> anyhow::Result<()> {
        let path = Self::path(data_dir);
        let text = serde_json::to_string_pretty(self)?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))?;
        }
        std::fs::rename(tmp, path)?;
        Ok(())
    }

    /// What the people using this machine are told: who manages it and what is shared.
    pub fn notice(&self) -> String {
        let what = match self.share_level.as_str() {
            "full" => "incidents, including prompts and the values caught,",
            "sanitised" => "incidents with prompts as the AI received them (no real values)",
            _ => "counts of what was protected",
        };
        format!(
            "Managed by {}: {what} are shared with its admins.",
            self.org_name
        )
    }
}

/// Split a join link (`https://hub.example.com/join/inv_...`) into hub origin and token.
pub fn parse_join_link(link: &str) -> Option<(String, String)> {
    let link = link.trim().trim_end_matches('/');
    let (base, token) = link.rsplit_once("/join/")?;
    if token.is_empty()
        || token.contains('/')
        || !(base.starts_with("https://") || base.starts_with("http://"))
    {
        return None;
    }
    Some((base.to_string(), token.to_string()))
}

/// What a join link offers, shown before anything is sent: `(org name, sharing level, invited email)`.
pub async fn join_info(join_link: &str) -> anyhow::Result<(String, String, Option<String>)> {
    let (hub_url, token) = parse_join_link(join_link).ok_or_else(|| {
        anyhow::anyhow!("that doesn't look like a join link (https://<hub>/join/<token>)")
    })?;
    let r = client()?
        .get(format!("{hub_url}/api/join/{token}"))
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("could not reach the hub at {hub_url}: {e}"))?;
    let v: Value = serde_json::from_str(&r.text().await.unwrap_or_default()).unwrap_or_default();
    anyhow::ensure!(
        v.get("valid").and_then(Value::as_bool) == Some(true),
        "this invite link has expired, been used or been revoked"
    );
    let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    Ok((
        s("org_name").unwrap_or_default(),
        s("share_level").unwrap_or_default(),
        s("email"),
    ))
}

/// Enrol this machine with the hub behind `join_link` and save the enrolment.
pub async fn enroll(
    join_link: &str,
    data_dir: &Path,
    email: Option<&str>,
    name: Option<&str>,
) -> anyhow::Result<Enrollment> {
    let (hub_url, token) = parse_join_link(join_link).ok_or_else(|| {
        anyhow::anyhow!("that doesn't look like a join link (https://<hub>/join/<token>)")
    })?;
    let hostname = std::process::Command::new("hostname")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|h| !h.is_empty())
        .unwrap_or_else(|| "unknown".into());
    let os_user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "unknown".into());
    let body = json!({
        "token": token, "hostname": hostname, "os": std::env::consts::OS, "os_user": os_user,
        "shadow_version": env!("CARGO_PKG_VERSION"), "email": email, "name": name,
    });
    let r = client()?
        .post(format!("{hub_url}/api/enroll"))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| anyhow::anyhow!("could not reach the hub at {hub_url}: {e}"))?;
    let status = r.status();
    let reply: Value =
        serde_json::from_str(&r.text().await.unwrap_or_default()).unwrap_or_default();
    if !status.is_success() {
        let msg = reply
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("enrolment refused");
        anyhow::bail!("{msg} (HTTP {status})");
    }
    let s = |p: &str| {
        reply
            .pointer(p)
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    };
    let en = Enrollment {
        hub_url,
        device_id: s("/device_id"),
        device_token: s("/device_token"),
        org_name: s("/org/name"),
        share_level: s("/org/share_level"),
        user_email: s("/user/email"),
        cursor: 0,
        enrolled_at: shadow_core::activity::now_ms(),
    };
    anyhow::ensure!(!en.device_token.is_empty(), "the hub sent no device token");
    en.save(data_dir)?;
    Ok(en)
}

/// Trim an incident detail to the sharing level.
pub fn share_detail(d: &Detail, level: &str) -> Option<Value> {
    match level {
        "full" => serde_json::to_value(d).ok(),
        "sanitised" => {
            let mut v = serde_json::to_value(d).ok()?;
            if let Some(parts) = v.get_mut("parts").and_then(Value::as_array_mut) {
                parts.retain(|p| p.get("sent").is_some());
                for p in parts.iter_mut().filter_map(Value::as_object_mut) {
                    p.remove("original");
                }
            }
            if let Some(changes) = v.get_mut("changes").and_then(Value::as_array_mut) {
                for c in changes.iter_mut().filter_map(Value::as_object_mut) {
                    c.remove("original");
                }
            }
            if let Some(o) = v.as_object_mut() {
                o.remove("conversation");
                // Only opens the commitment to the original text, which is not shared.
                o.remove("receipt_salt");
                // A dangerous command can hold real values.
                for g in o
                    .get_mut("guard")
                    .and_then(Value::as_array_mut)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_object_mut)
                {
                    g.remove("snippet");
                }
            }
            Some(v)
        }
        _ => None,
    }
}

/// One upload: events (with their scan receipts) and the key that signed them.
fn payload(log: &ActivityLog, events: &[Event], level: &str) -> Value {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let list: Vec<Value> = events
        .iter()
        .map(|e| {
            let mut v = serde_json::to_value(e).unwrap_or_default();
            if let Some(o) = v.as_object_mut() {
                let id = o.remove("id").unwrap_or_default();
                o.insert("local_id".into(), id);
                if let Some(d) = log.detail(e.id).and_then(|d| share_detail(&d, level)) {
                    o.insert("detail".into(), d);
                }
                if let Some(r) = log.receipt(e.id) {
                    o.insert("receipt".into(), b64.encode(r.as_slice()).into());
                }
            }
            v
        })
        .collect();
    let mut body = json!({ "events": list });
    if let Some(key) = log.receipt_key() {
        body["receipt_key"] = b64.encode(key).into();
    }
    body
}

fn client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("shadow/", env!("CARGO_PKG_VERSION")))
        .build()
}

/// Send every event the hub hasn't accepted yet. Returns how many were sent; does nothing
/// when this machine is not enrolled.
pub async fn upload_pending(
    client: &reqwest::Client,
    log: &ActivityLog,
    data_dir: &Path,
) -> anyhow::Result<usize> {
    let Some(mut en) = Enrollment::load(data_dir) else {
        return Ok(0);
    };
    // The local log was reset (for example `persist = false` and a restart).
    if log.last_id() < en.cursor {
        en.cursor = 0;
    }
    let mut sent = 0;
    loop {
        let mut batch = log.recent(Some(en.cursor), usize::MAX, &[]);
        if batch.is_empty() {
            return Ok(sent);
        }
        batch.reverse();
        batch.truncate(BATCH);
        let last = batch.last().map(|e| e.id).unwrap_or(en.cursor);
        // Never report what happened before this machine joined the organisation.
        batch.retain(|e| e.ts >= en.enrolled_at);
        if batch.is_empty() {
            en.cursor = last;
            en.save(data_dir)?;
            continue;
        }
        let body = payload(log, &batch, &en.share_level);
        let r = client
            .post(format!("{}/api/ingest", en.hub_url))
            .bearer_auth(&en.device_token)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .body(body.to_string())
            .send()
            .await?;
        anyhow::ensure!(
            r.status().is_success(),
            "hub refused the upload: HTTP {}",
            r.status()
        );
        let reply: Value =
            serde_json::from_str(&r.text().await.unwrap_or_default()).unwrap_or_default();
        if let Some(level) = reply.get("share_level").and_then(Value::as_str) {
            en.share_level = level.to_string();
        }
        sent += batch.len();
        en.cursor = last;
        en.save(data_dir)?;
    }
}

/// Tell the hub this machine is running, and refresh the sharing level.
async fn heartbeat(client: &reqwest::Client, data_dir: &Path) {
    let Some(mut en) = Enrollment::load(data_dir) else {
        return;
    };
    let Ok(r) = client
        .get(format!("{}/api/device/policy", en.hub_url))
        .bearer_auth(&en.device_token)
        .send()
        .await
    else {
        return;
    };
    let reply: Value =
        serde_json::from_str(&r.text().await.unwrap_or_default()).unwrap_or_default();
    if let Some(level) = reply.get("share_level").and_then(Value::as_str) {
        if level != en.share_level {
            en.share_level = level.to_string();
            let _ = en.save(data_dir);
        }
    }
}

/// Upload new events until the process exits. Errors are retried on the next tick.
pub async fn run_uploader(log: Arc<ActivityLog>, data_dir: PathBuf) {
    let client = match client() {
        Ok(c) => c,
        Err(e) => return tracing::warn!("hub uploader unavailable: {e}"),
    };
    let mut warned = false;
    let mut idle = 0u32;
    loop {
        tokio::time::sleep(INTERVAL).await;
        match upload_pending(&client, &log, &data_dir).await {
            Ok(0) => {
                warned = false;
                idle += 1;
                // About once a minute with nothing to send: a heartbeat, which also picks up
                // a changed sharing level.
                if idle >= 6 {
                    idle = 0;
                    heartbeat(&client, &data_dir).await;
                }
            }
            Ok(_) => {
                warned = false;
                idle = 0;
            }
            Err(e) if !warned => {
                tracing::warn!("hub upload failed, will retry: {e:#}");
                warned = true;
            }
            Err(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shadow_core::activity::TextPart;
    use shadow_core::Change;

    #[test]
    fn join_links_are_parsed() {
        assert_eq!(
            parse_join_link("https://hub.example.com/join/inv_abc"),
            Some(("https://hub.example.com".into(), "inv_abc".into()))
        );
        assert_eq!(
            parse_join_link("http://127.0.0.1:3000/join/inv_x/"),
            Some(("http://127.0.0.1:3000".into(), "inv_x".into()))
        );
        assert!(parse_join_link("hub.example.com/join/x").is_none());
        assert!(parse_join_link("https://hub.example.com/other").is_none());
    }

    #[test]
    fn details_follow_the_sharing_level() {
        let d = Detail {
            parts: vec![
                TextPart {
                    role: Some("user".into()),
                    field: "m".into(),
                    original: "mail jane@acme.com".into(),
                    sent: Some("mail mei@lin.cn".into()),
                },
                TextPart {
                    role: Some("user".into()),
                    field: "blocked".into(),
                    original: "secret text".into(),
                    sent: None,
                },
            ],
            changes: vec![Change {
                kind: "email".into(),
                detector: "email".into(),
                original: "jane@acme.com".into(),
                sent: "mei@lin.cn".into(),
            }],
            ..Detail::default()
        };
        let full = share_detail(&d, "full").unwrap().to_string();
        assert!(full.contains("jane@acme.com"));
        let san = share_detail(&d, "sanitised").unwrap().to_string();
        assert!(
            !san.contains("jane@acme.com") && !san.contains("secret text"),
            "{san}"
        );
        assert!(san.contains("mei@lin.cn"));
        assert!(share_detail(&d, "counts").is_none());
    }
}
