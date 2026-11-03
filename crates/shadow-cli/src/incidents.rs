//! `shadow incidents`: list masked or blocked requests, or show one with its prompt, the
//! values caught and the app that sent it (details come from the running proxy's memory).

use anyhow::Context;
use serde_json::Value;
use shadow_core::activity::{kind_label, provider_label, Detail, Event};
use shadow_core::ShadowConfig;

use crate::activity::{describe_kinds, http_get, Source};
use crate::{redact, ui};

pub fn run(
    cfg: &ShadowConfig,
    id: Option<u64>,
    limit: usize,
    reveal: bool,
    json: bool,
) -> anyhow::Result<()> {
    let source = Source::live(cfg);
    let addr = source.addr().to_string();
    match id {
        None => list(&addr, limit, json),
        Some(id) => show(&addr, id, reveal, json),
    }
}

fn when(ts: i64) -> String {
    chrono::DateTime::from_timestamp_millis(ts)
        .map(|d| {
            d.with_timezone(&chrono::Local)
                .format("%b %d %H:%M:%S")
                .to_string()
        })
        .unwrap_or_default()
}

fn list(addr: &str, limit: usize, json: bool) -> anyhow::Result<()> {
    let body = http_get(
        addr,
        &format!("/shadow/activity?action=incidents&limit={limit}"),
    )?;
    if json {
        println!("{body}");
        return Ok(());
    }
    let v: Value = serde_json::from_str(&body).context("parsing incidents")?;
    let rows: Vec<Vec<String>> = v["events"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|e| {
            let has = e["has_detail"].as_bool().unwrap_or(false);
            let e: Event = serde_json::from_value(e).ok()?;
            let what = if e.action.as_str() == "blocked" {
                e.block_reason.clone().unwrap_or_else(|| "policy".into())
            } else {
                describe_kinds(&e, 3)
            };
            Some(vec![
                e.id.to_string(),
                when(e.ts),
                e.client.clone(),
                provider_label(&e.provider),
                e.action.as_str().to_string(),
                what,
                if has { "yes" } else { "—" }.to_string(),
            ])
        })
        .collect();
    ui::header("Shadow incidents");
    if rows.is_empty() {
        println!("  No incidents yet. When Shadow masks or blocks something, it shows up here.");
        return Ok(());
    }
    ui::table(
        &["ID", "WHEN", "APP", "AI", "ACTION", "WHAT", "PROMPT"],
        &rows,
    );
    println!();
    println!(
        "  {}",
        ui::dim("Run `shadow incidents <ID>` to see the prompt, the values caught and the app that sent it.")
    );
    Ok(())
}

fn show(addr: &str, id: u64, reveal: bool, json: bool) -> anyhow::Result<()> {
    let body = http_get(addr, &format!("/shadow/activity/events/{id}"))
        .with_context(|| format!("no incident {id} (see `shadow incidents`)"))?;
    if json {
        println!("{body}");
        return Ok(());
    }
    let v: Value = serde_json::from_str(&body).context("parsing incident")?;
    let e: Event = serde_json::from_value(v["event"].clone()).context("parsing event")?;
    let detail: Option<Detail> = serde_json::from_value(v["detail"].clone()).ok();
    let hide = |s: &str| if reveal { s.to_string() } else { redact(s) };

    let action = match e.action.as_str() {
        "blocked" => ui::red("Blocked"),
        "masked" => ui::accent("Masked"),
        a => a.to_string(),
    };
    ui::header(&format!("Incident #{id} · {}", when(e.ts)));
    ui::kv("action", &action);
    ui::kv("app", &e.client);
    if let Some(d) = &detail {
        if let Some(p) = &d.process {
            ui::kv("process", &format!("{} (pid {})", p.name, p.pid));
        }
        for (k, val) in &d.conversation {
            ui::kv("conversation", &format!("{k}: {val}"));
        }
        if let Some(peer) = &d.peer {
            ui::kv("from", peer);
        }
    }
    ui::kv(
        "ai",
        &format!("{} ({})", provider_label(&e.provider), e.host),
    );
    if let Some(d) = &detail {
        if let Some(m) = &d.model {
            ui::kv("model", m);
        }
        ui::kv("path", &format!("{} {}", e.method, d.path));
    }
    ui::kv(
        "response",
        &format!("HTTP {} · {} ms", e.status, e.latency_ms),
    );
    if let Some(r) = &e.block_reason {
        let msg = detail.as_ref().and_then(|d| d.message.clone());
        ui::kv("reason", &msg.unwrap_or_else(|| r.clone()));
    }
    if let Some(i) = &e.intent {
        ui::kv("intent", i);
    }

    let Some(d) = detail else {
        println!();
        if v["details_enabled"] == false {
            ui::warn("incident details are off; set [activity] details = true and restart Shadow");
        } else {
            ui::warn("the prompt of this incident was not kept (it happened before Shadow saved prompts to disk or while details were off, or was dropped to stay under activity.details_max_mb)");
        }
        return Ok(());
    };

    if !d.changes.is_empty() {
        println!();
        ui::header("What was caught");
        let rows: Vec<Vec<String>> = d
            .changes
            .iter()
            .map(|c| {
                vec![
                    kind_label(&c.kind),
                    c.detector.clone(),
                    hide(&c.original),
                    if c.kept() {
                        "sent unchanged (needed for the task)".into()
                    } else {
                        c.sent.clone()
                    },
                ]
            })
            .collect();
        ui::table(
            &["TYPE", "FOUND BY", "YOUR VALUE", "SENT TO THE AI AS"],
            &rows,
        );
    }

    println!();
    ui::header(if e.action.as_str() == "blocked" {
        "Prompt that was stopped"
    } else {
        "Prompt"
    });
    for p in &d.parts {
        let role = p.role.as_deref().unwrap_or("text").to_uppercase();
        let changed = p.sent.as_ref().is_some_and(|s| s != &p.original);
        println!("  {} {}", ui::bold(&role), ui::dim(&p.field));
        if changed || p.sent.is_none() {
            let mut written = p.original.clone();
            for c in &d.changes {
                if !c.original.is_empty() {
                    written = written.replace(&c.original, &ui::red(&hide(&c.original)));
                }
            }
            println!("  {} {}", ui::dim("written:"), indent(&written));
            if let Some(sent) = &p.sent {
                let mut s = sent.clone();
                for c in d.changes.iter().filter(|c| !c.sent.is_empty()) {
                    // Values kept on purpose are real: shorten them like the written side.
                    let shown = if c.kept() {
                        ui::accent(&hide(&c.sent))
                    } else {
                        ui::green(&c.sent)
                    };
                    s = s.replace(&c.sent, &shown);
                }
                println!("  {} {}", ui::dim("sent:   "), indent(&s));
            }
        } else {
            let line: String = p.original.chars().take(100).collect();
            let more = if p.original.chars().count() > 100 {
                "…"
            } else {
                ""
            };
            println!(
                "  {}",
                ui::dim(&format!("{}{more}", line.replace('\n', " ")))
            );
        }
    }
    if d.truncated {
        println!("  {}", ui::dim("(long text shortened)"));
    }
    if !reveal {
        println!();
        println!(
            "  {}",
            ui::dim("Values are shortened. Add --reveal to print them in full.")
        );
    }
    Ok(())
}

fn indent(s: &str) -> String {
    s.replace('\n', "\n            ")
}
