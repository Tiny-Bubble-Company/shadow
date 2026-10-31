//! Reading the activity feed: from the running proxy's API, from the log file when the
//! proxy is down, or synthetic demo data. Also `shadow activity`.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use anyhow::Context;
use shadow_core::activity::{
    now_ms, provider_label, Action, ActivityLog, Endpoint, Event, Summary, Window,
};
use shadow_core::ShadowConfig;

use crate::ui;

/// Where the numbers come from.
pub enum Source {
    /// The proxy's HTTP API (falls back to the log file while it is unreachable).
    Live {
        addr: String,
        cfg: Box<ShadowConfig>,
    },
    /// Synthetic data for screenshots and trying the UI.
    Demo(Box<ActivityLog>),
}

/// One fetch: what to show and how it was obtained.
pub struct Snapshot {
    pub summary: Summary,
    pub incidents: Vec<Event>,
    pub state: Feed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Feed {
    /// Live from the proxy.
    Online,
    /// Proxy unreachable; showing the saved log (may be empty).
    Offline(String),
    Demo,
}

impl Source {
    pub fn live(cfg: &ShadowConfig) -> Self {
        Source::Live {
            addr: connect_addr(&cfg.proxy.listen),
            cfg: Box::new(cfg.clone()),
        }
    }

    pub fn demo() -> Self {
        Source::Demo(Box::new(demo_log()))
    }

    /// Address shown to the user.
    pub fn addr(&self) -> &str {
        match self {
            Source::Live { addr, .. } => addr,
            Source::Demo(_) => "demo",
        }
    }

    pub fn fetch(&self, window: Window, incidents: usize) -> Snapshot {
        match self {
            Source::Demo(log) => Snapshot {
                summary: log.summary(window),
                incidents: log.recent(None, incidents, &[Action::Masked, Action::Blocked]),
                state: Feed::Demo,
            },
            Source::Live { addr, cfg } => match fetch_live(addr, window, incidents) {
                Ok((summary, incidents)) => Snapshot {
                    summary,
                    incidents,
                    state: Feed::Online,
                },
                Err(e) => {
                    let log = ActivityLog::read_only(&cfg.activity, &cfg.activity_dir());
                    Snapshot {
                        summary: log.summary(window),
                        incidents: log.recent(None, incidents, &[Action::Masked, Action::Blocked]),
                        state: Feed::Offline(format!("{e:#}")),
                    }
                }
            },
        }
    }
}

/// `0.0.0.0:8877` is not connectable everywhere; talk to loopback on the same port.
fn connect_addr(listen: &str) -> String {
    match listen.parse::<SocketAddr>() {
        Ok(a) if a.ip().is_unspecified() => {
            if a.is_ipv6() {
                format!("[::1]:{}", a.port())
            } else {
                format!("127.0.0.1:{}", a.port())
            }
        }
        _ => listen.to_string(),
    }
}

/// Minimal HTTP/1.1 GET against the local proxy (no TLS, no redirects, no proxies).
pub fn http_get(addr: &str, path: &str) -> anyhow::Result<String> {
    let sock = addr
        .to_socket_addrs()?
        .next()
        .context("no address to connect to")?;
    let mut s = TcpStream::connect_timeout(&sock, Duration::from_millis(600))
        .with_context(|| format!("Shadow is not running on {addr}"))?;
    s.set_read_timeout(Some(Duration::from_secs(3)))?;
    s.set_write_timeout(Some(Duration::from_secs(3)))?;
    write!(
        s,
        "GET {path} HTTP/1.1\r\nHost: {addr}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    )?;
    let mut buf = Vec::new();
    s.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf);
    let (head, body) = text.split_once("\r\n\r\n").context("bad HTTP response")?;
    let status = head.split_whitespace().nth(1).unwrap_or("");
    anyhow::ensure!(status == "200", "proxy answered HTTP {status}");
    if head
        .to_ascii_lowercase()
        .contains("transfer-encoding: chunked")
    {
        return Ok(dechunk(body));
    }
    Ok(body.to_string())
}

fn dechunk(mut body: &str) -> String {
    let mut out = String::new();
    while let Some((size, rest)) = body.split_once("\r\n") {
        let n = usize::from_str_radix(size.trim(), 16).unwrap_or(0);
        if n == 0 || rest.len() < n {
            break;
        }
        out.push_str(&rest[..n]);
        body = rest[n..].trim_start_matches("\r\n");
    }
    out
}

fn fetch_live(addr: &str, window: Window, n: usize) -> anyhow::Result<(Summary, Vec<Event>)> {
    let summary: Summary = serde_json::from_str(&http_get(
        addr,
        &format!("/shadow/activity/summary?window={}", window.as_str()),
    )?)
    .context("parsing summary")?;
    let v: serde_json::Value = serde_json::from_str(&http_get(
        addr,
        &format!("/shadow/activity?action=incidents&limit={n}"),
    )?)?;
    let incidents = serde_json::from_value(v["events"].clone()).unwrap_or_default();
    Ok((summary, incidents))
}

// ----------------------------------------------------------------------------------------
// Demo data

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn pick<'a, T>(&mut self, items: &'a [(T, f64)]) -> &'a T {
        let total: f64 = items.iter().map(|(_, w)| w).sum();
        let mut x = self.next() * total;
        for (t, w) in items {
            x -= w;
            if x <= 0.0 {
                return t;
            }
        }
        &items[items.len() - 1].0
    }
}

/// A month of realistic, clearly synthetic traffic; "now" is always a busy afternoon.
pub fn demo_log() -> ActivityLog {
    type Client = (&'static str, &'static [(&'static str, f64)]);
    const CLIENTS: &[(Client, f64)] = &[
        (("Claude Code", &[("anthropic", 1.0)]), 30.0),
        (("Codex CLI", &[("openai", 1.0)]), 20.0),
        (
            (
                "OpenAI Python SDK",
                &[("openai", 0.7), ("azure", 0.2), ("openrouter", 0.1)],
            ),
            14.0,
        ),
        (
            (
                "Aider",
                &[("anthropic", 0.5), ("openai", 0.3), ("openrouter", 0.2)],
            ),
            9.0,
        ),
        (("Google GenAI SDK", &[("gemini", 1.0)]), 7.0),
        (("Anthropic Node SDK", &[("anthropic", 1.0)]), 6.0),
        (("LangChain", &[("openai", 0.5), ("mistral", 0.5)]), 5.0),
        (("Anthropic Python SDK", &[("anthropic", 1.0)]), 5.0),
        (("curl", &[("openai", 0.6), ("anthropic", 0.4)]), 4.0),
    ];
    const KINDS: &[(&str, f64)] = &[
        ("email", 26.0),
        ("person", 22.0),
        ("secret", 12.0),
        ("hostname", 11.0),
        ("org", 8.0),
        ("term", 8.0),
        ("phone", 6.0),
        ("ip", 6.0),
        ("username", 5.0),
        ("address", 3.0),
        ("credit_card", 2.0),
        ("iban", 1.5),
        ("national_id", 1.0),
    ];
    let host = |p: &str| match p {
        "openai" => "api.openai.com",
        "anthropic" => "api.anthropic.com",
        "gemini" => "generativelanguage.googleapis.com",
        "azure" => "acme-eu.openai.azure.com",
        "openrouter" => "openrouter.ai",
        "mistral" => "api.mistral.ai",
        _ => "",
    };
    let log = ActivityLog::in_memory(30);
    let mut r = Rng(0x5eed_cafe_f00d_1234);
    let now = now_ms() / 1000;
    let end = now - now % 60;
    let mut m = end - 30 * 86400;
    while m <= end {
        let hours_ago = (end - m) as f64 / 3600.0;
        let hour = (16.0 - hours_ago).rem_euclid(24.0);
        let day = ((hours_ago + 8.0) / 24.0).floor() as i64 % 7;
        let busy = if (9.0..19.0).contains(&hour) {
            1.0
        } else if (7.0..22.0).contains(&hour) {
            0.35
        } else {
            0.05
        };
        let weekend = if day == 5 || day == 6 { 0.18 } else { 1.0 };
        let wave = 0.75 + 0.5 * (m as f64 / 5400.0).sin() * (m as f64 / 21000.0 + 1.0).sin();
        let lam: f64 = (weekend * busy * 1.1 * wave).max(0.0);
        // Poisson draw
        let (mut n, mut p, x) = (0, (-lam).exp(), r.next());
        let mut c = p;
        while x > c && n < 12 {
            n += 1;
            p *= lam / n as f64;
            c += p;
        }
        for _ in 0..n {
            let (client, provs) = *r.pick(CLIENTS);
            let provider = *r.pick(provs);
            let roll = r.next();
            let action = if roll < 0.022 {
                Action::Blocked
            } else if roll < 0.6 {
                Action::Masked
            } else {
                Action::Clean
            };
            let mut ev = Event {
                ts: (m * 1000) + (r.next() * 59_000.0) as i64,
                provider: provider.into(),
                host: host(provider).into(),
                client: client.into(),
                mode: if r.next() < 0.7 { "proxy" } else { "gateway" }.into(),
                method: "POST".into(),
                endpoint: *r.pick(&[
                    (Endpoint::Chat, 88.0),
                    (Endpoint::Embeddings, 8.0),
                    (Endpoint::Files, 4.0),
                ]),
                action,
                status: 200,
                bytes_in: 800 + (r.next() * 24000.0) as u64,
                bytes_out: 400 + (r.next() * 9000.0) as u64,
                latency_ms: 380 + (r.next() * 4000.0) as u64,
                ..Event::default()
            };
            ev.streamed = ev.endpoint == Endpoint::Chat && r.next() < 0.7;
            match action {
                Action::Masked => {
                    let k = 1 + (r.next() * r.next() * 4.0) as usize;
                    for _ in 0..k {
                        let kind = *r.pick(KINDS);
                        *ev.kinds.entry(kind.into()).or_default() +=
                            1 + (r.next() * r.next() * 5.0) as u32;
                    }
                    ev.restored = (ev.values() as f64 * (0.4 + r.next() * 0.6)).round() as u32;
                    ev.intent = Some("code".into());
                }
                Action::Blocked => {
                    ev.status = 403;
                    ev.bytes_out = 160;
                    ev.latency_ms = 3;
                    ev.block_reason = Some(
                        (*r.pick(&[
                            ("image_blocked", 5.0),
                            ("pdf_blocked", 2.0),
                            ("file_too_large", 2.0),
                            ("unreadable_pdf", 1.0),
                        ]))
                        .into(),
                    );
                }
                Action::Clean => {}
            }
            log.record(ev);
        }
        m += 60;
    }
    log
}

// ----------------------------------------------------------------------------------------
// `shadow activity`

pub fn fmt_n(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn pct(a: u64, b: u64) -> String {
    if b == 0 {
        "0%".into()
    } else {
        format!("{}%", (a * 100 + b / 2) / b)
    }
}

/// Short "2 email · 1 secret" description of an event's protected kinds.
pub fn describe_kinds(e: &Event, limit: usize) -> String {
    let mut kinds: Vec<(&String, &u32)> = e.kinds.iter().collect();
    kinds.sort_by(|a, b| b.1.cmp(a.1));
    let mut parts: Vec<String> = kinds
        .iter()
        .take(limit)
        .map(|(k, n)| format!("{n} {}", k.replace('_', " ")))
        .collect();
    if kinds.len() > limit {
        parts.push(format!("+{}", kinds.len() - limit));
    }
    parts.join(" · ")
}

pub fn run(cfg: &ShadowConfig, window: &str, json: bool, demo: bool) -> anyhow::Result<()> {
    let window = Window::parse(window)
        .with_context(|| format!("unknown window {window:?}; use 1h, 24h, 7d, 30d or today"))?;
    let source = if demo {
        Source::demo()
    } else {
        Source::live(cfg)
    };
    let snap = source.fetch(window, 10);
    if json {
        println!("{}", serde_json::to_string_pretty(&snap.summary)?);
        if let Feed::Offline(e) = &snap.state {
            eprintln!("note: {e}; summarised the saved activity log instead");
        }
        return Ok(());
    }
    let s = &snap.summary;
    let t = &s.totals;
    let label = match window {
        Window::Hour => "last hour",
        Window::Day => "last 24 hours",
        Window::Week => "last 7 days",
        Window::Month => "last 30 days",
        Window::Today => "today",
    };
    ui::header(&format!("Shadow activity · {label}"));
    match &snap.state {
        Feed::Online => ui::kv(
            "proxy",
            &ui::green(&format!("running on {}", source.addr())),
        ),
        Feed::Offline(_) => ui::kv(
            "proxy",
            &ui::yellow(&format!(
                "not running on {} (showing the saved log)",
                source.addr()
            )),
        ),
        Feed::Demo => ui::kv("data", &ui::yellow("DEMO DATA (synthetic)")),
    }
    if !s.enabled && snap.state == Feed::Online {
        ui::warn("activity recording is off ([activity] enabled = false)");
    }
    ui::kv(
        "requests",
        &format!(
            "{} monitored · {} masked ({}) · {} blocked · {} clean",
            ui::bold(&fmt_n(t.requests)),
            fmt_n(t.masked_requests),
            pct(t.masked_requests, t.requests),
            fmt_n(t.blocked),
            fmt_n(t.clean)
        ),
    );
    ui::kv(
        "values",
        &format!(
            "{} protected · {} restored in responses",
            ui::accent(&fmt_n(t.values_protected)),
            fmt_n(t.values_restored)
        ),
    );
    ui::kv(
        "reach",
        &format!("{} destination(s) · {} app(s)", t.providers, t.clients),
    );
    let section = |title: &str, rows: Vec<Vec<String>>, headers: &[&str]| {
        if rows.is_empty() {
            return;
        }
        println!();
        ui::header(title);
        ui::table(headers, &rows);
    };
    section(
        "By data type",
        s.kinds
            .iter()
            .map(|k| {
                vec![
                    k.label.clone(),
                    fmt_n(k.values),
                    format!("{} req", fmt_n(k.requests)),
                ]
            })
            .collect(),
        &["TYPE", "VALUES", "IN"],
    );
    section(
        "By destination",
        s.providers
            .iter()
            .map(|p| {
                vec![
                    provider_label(&p.key),
                    fmt_n(p.requests),
                    pct(p.requests, t.requests),
                    fmt_n(p.values),
                ]
            })
            .collect(),
        &["DESTINATION", "REQUESTS", "SHARE", "VALUES"],
    );
    section(
        "By app",
        s.clients
            .iter()
            .map(|c| {
                vec![
                    c.label.clone(),
                    fmt_n(c.requests),
                    fmt_n(c.values),
                    fmt_n(c.blocked),
                ]
            })
            .collect(),
        &["APP", "REQUESTS", "VALUES", "BLOCKED"],
    );
    section(
        "Blocked",
        s.block_reasons
            .iter()
            .map(|b| vec![b.label.clone(), fmt_n(b.requests)])
            .collect(),
        &["REASON", "REQUESTS"],
    );
    section(
        "Recent incidents",
        snap.incidents
            .iter()
            .map(|e| {
                let when = chrono::DateTime::from_timestamp_millis(e.ts)
                    .map(|d| {
                        d.with_timezone(&chrono::Local)
                            .format("%b %d %H:%M")
                            .to_string()
                    })
                    .unwrap_or_default();
                vec![
                    when,
                    format!("{} → {}", e.client, provider_label(&e.provider)),
                    e.action.as_str().to_string(),
                    if e.action == Action::Blocked {
                        shadow_core::activity::block_reason_label(
                            e.block_reason.as_deref().unwrap_or("file_policy"),
                        )
                    } else {
                        describe_kinds(e, 3)
                    },
                ]
            })
            .collect(),
        &["WHEN", "SOURCE → DESTINATION", "ACTION", "WHAT"],
    );
    if t.requests == 0 {
        println!();
        println!(
            "  {}",
            ui::dim("No AI traffic in this window yet. Dashboard: http://127.0.0.1:8877/shadow/ui")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn demo_log_is_realistic() {
        let log = demo_log();
        let d = log.summary(Window::Day);
        assert!(d.totals.requests > 100, "{}", d.totals.requests);
        assert!(d.totals.values_protected > d.totals.masked_requests);
        assert!(d.providers.len() >= 5);
        assert!(log.summary(Window::Month).totals.requests > d.totals.requests);
    }

    #[test]
    fn formatting() {
        assert_eq!(fmt_n(0), "0");
        assert_eq!(fmt_n(1234567), "1,234,567");
        assert_eq!(pct(1, 3), "33%");
        assert_eq!(pct(0, 0), "0%");
        assert_eq!(connect_addr("0.0.0.0:8877"), "127.0.0.1:8877");
        assert_eq!(connect_addr("127.0.0.1:9000"), "127.0.0.1:9000");
        assert_eq!(
            dechunk("5\r\nhello\r\n6\r\n world\r\n0\r\n\r\n"),
            "hello world"
        );
    }
}
