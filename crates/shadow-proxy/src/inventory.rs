//! Keeps the extension / MCP / plugin inventory current (see `shadow_core::inventory`) and
//! sends it, with what changed, to Shadow Hub when this machine is connected.

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{json, Value};
use shadow_core::activity::now_ms;
use shadow_core::inventory::{self, Policy, Roots, State};

const INTERVAL: Duration = Duration::from_secs(600);

/// Scan now and save. Returns the new state.
pub fn rescan(path: &Path) -> anyhow::Result<State> {
    let roots = Roots::detect().ok_or_else(|| anyhow::anyhow!("no home directory"))?;
    let mut st = State::load(path).unwrap_or_default();
    let now = now_ms();
    let items = inventory::scan(&roots, &st.cache, now);
    let changes = st.update(items, now);
    for c in &changes {
        tracing::info!(target: "shadow::inventory", "{} {} {} ({})", c.change, c.kind, c.id, c.app);
    }
    for (i, p) in st.blocked() {
        tracing::warn!(target: "shadow::inventory", "{} {} is installed but blocked by your organisation{}", i.kind, i.id,
            p.note.as_deref().map(|n| format!(": {n}")).unwrap_or_default());
    }
    st.save(path)?;
    Ok(st)
}

/// The inventory without local paths, for the dashboard and Hub.
pub fn public_items(st: &State) -> Vec<Value> {
    st.items
        .iter()
        .map(|i| {
            let mut v = serde_json::to_value(i).unwrap_or_default();
            if let Some(o) = v.as_object_mut() {
                o.remove("path");
            }
            v
        })
        .collect()
}

async fn upload(client: &reqwest::Client, data_dir: &Path, path: &Path) -> anyhow::Result<()> {
    let Some(en) = crate::hub::Enrollment::load(data_dir) else {
        return Ok(());
    };
    let Some(st) = State::load(path) else {
        return Ok(());
    };
    let sent = st.unsent.len();
    let body = json!({"items": public_items(&st), "changes": st.unsent});
    let r = client
        .post(format!("{}/api/device/inventory", en.hub_url))
        .bearer_auth(&en.device_token)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body.to_string())
        .send()
        .await?;
    anyhow::ensure!(
        r.status().is_success(),
        "hub refused the inventory: HTTP {}",
        r.status()
    );
    let reply: Value =
        serde_json::from_str(&r.text().await.unwrap_or_default()).unwrap_or_default();
    let policy: Vec<Policy> = reply
        .get("policy")
        .cloned()
        .and_then(|p| serde_json::from_value(p).ok())
        .unwrap_or_default();
    // Re-read: a scan may have run meanwhile; drop only what was sent.
    let mut st = State::load(path).unwrap_or(st);
    st.unsent.drain(..sent.min(st.unsent.len()));
    st.policy = policy;
    st.save(path)
}

/// Scan every ten minutes, and upload to the hub after each scan.
pub async fn run(data_dir: PathBuf, path: PathBuf) {
    let client = match reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(30))
        .user_agent(concat!("shadow/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => return tracing::warn!("inventory uploader unavailable: {e}"),
    };
    tokio::time::sleep(Duration::from_secs(5)).await;
    loop {
        let p = path.clone();
        match tokio::task::spawn_blocking(move || rescan(&p)).await {
            Ok(Ok(_)) => {
                if let Err(e) = upload(&client, &data_dir, &path).await {
                    tracing::debug!("inventory upload failed, will retry: {e:#}");
                }
            }
            Ok(Err(e)) => tracing::warn!("inventory scan failed: {e:#}"),
            Err(e) => tracing::warn!("inventory scan stopped: {e}"),
        }
        tokio::time::sleep(INTERVAL).await;
    }
}
