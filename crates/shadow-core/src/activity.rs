//! Privacy-safe activity feed: what Shadow did, never what it saw.
//!
//! Every request that goes through the proxy can produce one [`Event`]: when it happened,
//! which provider it went to, which app sent it, what kind of endpoint it hit, whether it
//! was masked, blocked or clean, and *how many* values of each kind were protected or
//! restored. Events never contain real values, fakes, tokens, prompts, file names, headers
//! or request paths. The only free-form strings are the provider host (an AI API host from
//! your own config) and a client name derived from the `User-Agent` product token.
//!
//! [`ActivityLog`] keeps a ring buffer of recent events, rolled-up per-minute / per-hour
//! counters for summaries, and (optionally) an append-only JSONL file with rotation and
//! retention.

use std::collections::{BTreeMap, VecDeque};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::config::ActivityConfig;

/// What Shadow did with a request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Inspected; nothing sensitive found.
    #[default]
    Clean,
    /// At least one value was replaced before the request left the machine.
    Masked,
    /// Refused by policy; nothing was sent upstream.
    Blocked,
}

impl Action {
    pub fn as_str(self) -> &'static str {
        match self {
            Action::Clean => "clean",
            Action::Masked => "masked",
            Action::Blocked => "blocked",
        }
    }
}

/// Coarse kind of API endpoint.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Endpoint {
    Chat,
    Embeddings,
    Files,
    #[default]
    Other,
}

impl Endpoint {
    pub fn as_str(self) -> &'static str {
        match self {
            Endpoint::Chat => "chat",
            Endpoint::Embeddings => "embeddings",
            Endpoint::Files => "files",
            Endpoint::Other => "other",
        }
    }
}

/// One proxied request. Counts and categories only.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Event {
    /// Monotonic id (unique per data dir).
    pub id: u64,
    /// Unix time in milliseconds.
    pub ts: i64,
    /// Provider id, e.g. `openai`, `anthropic`, `gemini` (see [`provider_label`]).
    pub provider: String,
    /// Upstream API host, e.g. `api.openai.com`.
    pub host: String,
    /// Friendly client name, e.g. `Claude Code`, `OpenAI Python SDK`, `curl`.
    pub client: String,
    /// `proxy` (HTTPS_PROXY / MITM) or `gateway` (`/gw/...`).
    pub mode: String,
    pub method: String,
    pub endpoint: Endpoint,
    pub action: Action,
    /// Values protected per kind, e.g. `{"email": 2, "secret": 1}`.
    pub kinds: BTreeMap<String, u32>,
    /// Detectors that found them, e.g. `["dictionary", "email"]`.
    pub detectors: Vec<String>,
    /// Values translated back to their real form in the response.
    pub restored: u32,
    /// Request intent (`act`, `report`, `code`, `converse`) when classified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    /// Response was a stream (SSE / NDJSON).
    pub streamed: bool,
    /// Request body size as received from the client.
    pub bytes_in: u64,
    /// Response body size as sent to the client.
    pub bytes_out: u64,
    /// Time from receiving the request to the end of the response.
    pub latency_ms: u64,
    /// HTTP status returned to the client.
    pub status: u16,
    /// Why it was blocked: a fixed category such as `image_blocked`, never a message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_reason: Option<String>,
}

impl Event {
    /// Total values protected in this request.
    pub fn values(&self) -> u64 {
        self.kinds.values().map(|n| *n as u64).sum()
    }
}

// ----------------------------------------------------------------------------------------
// Providers, clients, endpoints

/// `(host glob, provider id, label)`; first match wins.
const PROVIDERS: &[(&str, &str, &str)] = &[
    ("api.openai.com", "openai", "OpenAI"),
    ("chatgpt.com", "chatgpt", "ChatGPT"),
    ("*.chatgpt.com", "chatgpt", "ChatGPT"),
    ("chat.openai.com", "chatgpt", "ChatGPT"),
    ("api.anthropic.com", "anthropic", "Anthropic"),
    ("claude.ai", "claude", "Claude.ai"),
    ("*.claude.ai", "claude", "Claude.ai"),
    (
        "generativelanguage.googleapis.com",
        "gemini",
        "Google Gemini",
    ),
    ("*aiplatform.googleapis.com", "vertex", "Google Vertex AI"),
    ("openrouter.ai", "openrouter", "OpenRouter"),
    ("api.mistral.ai", "mistral", "Mistral"),
    ("api.groq.com", "groq", "Groq"),
    ("api.together.xyz", "together", "Together AI"),
    ("api.together.ai", "together", "Together AI"),
    ("api.deepseek.com", "deepseek", "DeepSeek"),
    ("api.x.ai", "xai", "xAI"),
    ("*.openai.azure.com", "azure", "Azure OpenAI"),
    ("*.services.ai.azure.com", "azure", "Azure OpenAI"),
    ("*.cognitiveservices.azure.com", "azure", "Azure OpenAI"),
    ("bedrock-runtime.*.amazonaws.com", "bedrock", "AWS Bedrock"),
    ("api.cohere.com", "cohere", "Cohere"),
    ("api.cohere.ai", "cohere", "Cohere"),
    ("api.perplexity.ai", "perplexity", "Perplexity"),
    ("api.fireworks.ai", "fireworks", "Fireworks AI"),
    ("api.cerebras.ai", "cerebras", "Cerebras"),
    ("api.voyageai.com", "voyage", "Voyage AI"),
    ("api.githubcopilot.com", "copilot", "GitHub Copilot"),
    ("*.githubcopilot.com", "copilot", "GitHub Copilot"),
    ("localhost", "local", "Local model"),
    ("127.0.0.1", "local", "Local model"),
    ("::1", "local", "Local model"),
];

fn glob(pattern: &str, host: &str) -> bool {
    match pattern.split_once('*') {
        None => pattern == host,
        Some((pre, post)) => {
            host.len() >= pre.len() + post.len() && host.starts_with(pre) && host.ends_with(post)
        }
    }
}

/// Provider id for an upstream host (`api.openai.com` -> `openai`); `other` if unknown.
pub fn provider_for_host(host: &str) -> &'static str {
    let h = host.trim_matches(['[', ']']).to_ascii_lowercase();
    PROVIDERS
        .iter()
        .find(|(p, _, _)| glob(p, &h))
        .map(|(_, id, _)| *id)
        .unwrap_or("other")
}

/// Display name of a provider id (unknown ids are returned as-is).
pub fn provider_label(id: &str) -> String {
    PROVIDERS
        .iter()
        .find(|(_, p, _)| *p == id)
        .map(|(_, _, l)| l.to_string())
        .unwrap_or_else(|| match id {
            "other" => "Other".into(),
            _ => id.to_string(),
        })
}

/// `(lowercase needle, client name)`; first match wins.
const CLIENTS: &[(&str, &str)] = &[
    ("claude-cli", "Claude Code"),
    ("claude-code", "Claude Code"),
    ("claude-desktop", "Claude Desktop"),
    ("codex_cli", "Codex CLI"),
    ("codex-cli", "Codex CLI"),
    ("chatgpt", "ChatGPT Desktop"),
    ("cursor", "Cursor"),
    ("windsurf", "Windsurf"),
    ("codeium", "Windsurf"),
    ("githubcopilot", "GitHub Copilot"),
    ("copilot", "GitHub Copilot"),
    ("gemini-cli", "Gemini CLI"),
    ("geminicli", "Gemini CLI"),
    ("aider", "Aider"),
    ("cline", "Cline"),
    ("zed/", "Zed"),
    ("jetbrains", "JetBrains AI"),
    ("raycast", "Raycast"),
    ("langchain", "LangChain"),
    ("litellm", "LiteLLM"),
    ("llama-index", "LlamaIndex"),
    ("llamaindex", "LlamaIndex"),
    ("openai/python", "OpenAI Python SDK"),
    ("openai/js", "OpenAI Node SDK"),
    ("openai/node", "OpenAI Node SDK"),
    ("openai/go", "OpenAI Go SDK"),
    ("openai/java", "OpenAI Java SDK"),
    ("anthropic/python", "Anthropic Python SDK"),
    ("anthropic/js", "Anthropic Node SDK"),
    ("anthropic/node", "Anthropic Node SDK"),
    ("anthropic-sdk-go", "Anthropic Go SDK"),
    ("google-genai-sdk", "Google GenAI SDK"),
    ("google-genai", "Google GenAI SDK"),
    ("google-api-", "Google API client"),
    ("vscode", "VS Code"),
    ("curl/", "curl"),
    ("wget", "wget"),
    ("httpie", "HTTPie"),
    ("postmanruntime", "Postman"),
    ("insomnia", "Insomnia"),
    ("python-requests", "Python requests"),
    ("python-httpx", "Python httpx"),
    ("aiohttp", "Python aiohttp"),
    ("python-urllib", "Python urllib"),
    ("axios", "Node.js (axios)"),
    ("node-fetch", "Node.js"),
    ("undici", "Node.js"),
    ("bun/", "Bun"),
    ("deno/", "Deno"),
    ("go-http-client", "Go"),
    ("reqwest", "Rust (reqwest)"),
    ("okhttp", "Java (OkHttp)"),
    ("java/", "Java"),
    ("electron", "Electron app"),
    ("mozilla/", "Web browser"),
];

/// Friendly client name from a `User-Agent` header. Unknown agents fall back to their first
/// product token (`MyTool/1.2 (...)` -> `MyTool`), sanitised and truncated; never the
/// comment part, which can carry hostnames or user names.
pub fn client_from_user_agent(ua: Option<&str>) -> String {
    let Some(ua) = ua.map(str::trim).filter(|u| !u.is_empty()) else {
        return "Unknown".into();
    };
    let lower = ua.to_ascii_lowercase();
    if lower == "node" || lower.starts_with("node/") {
        return "Node.js".into();
    }
    if let Some((_, name)) = CLIENTS.iter().find(|(n, _)| lower.contains(n)) {
        return name.to_string();
    }
    let token: String = ua
        .split(['/', ' ', '(', ';'])
        .next()
        .unwrap_or("")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        .take(32)
        .collect();
    if token.is_empty() {
        "Unknown".into()
    } else {
        token
    }
}

/// Endpoint kind from a request path.
pub fn endpoint_for_path(path: &str) -> Endpoint {
    let p = path.to_ascii_lowercase();
    if p.contains("embed") {
        Endpoint::Embeddings
    } else if p.contains("/files") || p.contains("/uploads") || p.contains("/upload/") {
        Endpoint::Files
    } else if [
        "/chat",
        "/completions",
        "/messages",
        "/responses",
        "generatecontent",
        "/api/generate",
        "/converse",
        "/invoke",
        "/conversation",
        "/rawpredict",
        "/predict",
    ]
    .iter()
    .any(|m| p.contains(m))
    {
        Endpoint::Chat
    } else {
        Endpoint::Other
    }
}

/// Display name of a data kind.
pub fn kind_label(kind: &str) -> String {
    match kind {
        "email" => "Email addresses",
        "phone" => "Phone numbers",
        "person" => "Names",
        "org" => "Organisations",
        "term" => "Protected terms",
        "address" => "Postal addresses",
        "iban" => "IBANs",
        "credit_card" => "Card numbers",
        "national_id" => "National IDs",
        "ip" => "IP addresses",
        "mac" => "MAC addresses",
        "hostname" => "Hostnames",
        "secret" => "Secrets & API keys",
        "username" => "Usernames",
        "identifier" => "User identifiers",
        other => other,
    }
    .to_string()
}

/// Display name of a block reason category.
pub fn block_reason_label(reason: &str) -> String {
    match reason {
        "image_blocked" => "Image blocked by policy",
        "pdf_blocked" => "PDF blocked by policy",
        "unreadable_pdf" => "PDF could not be read",
        "file_too_large" => "File over size limit",
        "file_policy" => "File policy",
        other => other,
    }
    .to_string()
}

fn endpoint_label(e: &str) -> String {
    match e {
        "chat" => "Chat & completions",
        "embeddings" => "Embeddings",
        "files" => "File uploads",
        _ => "Other API calls",
    }
    .to_string()
}

// ----------------------------------------------------------------------------------------
// Windows and summaries

/// Summary window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Window {
    Hour,
    Day,
    Week,
    Month,
    /// Since local midnight.
    Today,
}

impl Window {
    pub const ALL: [Window; 4] = [Window::Hour, Window::Day, Window::Week, Window::Month];

    pub fn parse(s: &str) -> Option<Window> {
        Some(match s.trim().to_ascii_lowercase().as_str() {
            "1h" | "hour" => Window::Hour,
            "24h" | "1d" | "day" => Window::Day,
            "7d" | "week" => Window::Week,
            "30d" | "month" => Window::Month,
            "today" => Window::Today,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Window::Hour => "1h",
            Window::Day => "24h",
            Window::Week => "7d",
            Window::Month => "30d",
            Window::Today => "today",
        }
    }

    /// `(from, bucket, points)` in seconds for a window ending at `now` (seconds).
    pub fn range(self, now: i64) -> (i64, i64, usize) {
        let aligned = |bucket: i64, points: usize| {
            let from = now - now.rem_euclid(bucket) - (points as i64 - 1) * bucket;
            (from, bucket, points)
        };
        match self {
            Window::Hour => aligned(60, 60),
            Window::Day => aligned(1800, 48),
            Window::Week => aligned(3 * 3600, 56),
            Window::Month => aligned(12 * 3600, 60),
            Window::Today => {
                use chrono::TimeZone;
                let local = chrono::Local
                    .timestamp_opt(now, 0)
                    .single()
                    .unwrap_or_else(chrono::Local::now);
                let midnight = local
                    .date_naive()
                    .and_hms_opt(0, 0, 0)
                    .and_then(|m| m.and_local_timezone(chrono::Local).earliest())
                    .map(|m| m.timestamp())
                    .unwrap_or(now - now.rem_euclid(86400));
                let points = ((now - midnight) / 3600 + 1).clamp(1, 25) as usize;
                (midnight, 3600, points)
            }
        }
    }
}

/// Requests / values / blocks for one breakdown row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tally {
    pub requests: u64,
    pub values: u64,
    pub blocked: u64,
}

impl Tally {
    fn add(&mut self, o: &Tally) {
        self.requests += o.requests;
        self.values += o.values;
        self.blocked += o.blocked;
    }
}

#[derive(Clone, Debug, Default)]
struct Agg {
    requests: u64,
    masked: u64,
    blocked: u64,
    values: u64,
    restored: u64,
    bytes_in: u64,
    bytes_out: u64,
    streamed: u64,
    latency_ms: u64,
    kinds: BTreeMap<String, Tally>,
    providers: BTreeMap<String, Tally>,
    clients: BTreeMap<String, Tally>,
    endpoints: BTreeMap<String, Tally>,
    block_reasons: BTreeMap<String, Tally>,
}

fn bump(map: &mut BTreeMap<String, Tally>, key: &str, t: &Tally) {
    match map.get_mut(key) {
        Some(x) => x.add(t),
        None => {
            map.insert(key.to_string(), *t);
        }
    }
}

impl Agg {
    fn add(&mut self, e: &Event) {
        let values = e.values();
        let blocked = (e.action == Action::Blocked) as u64;
        self.requests += 1;
        self.masked += (e.action == Action::Masked) as u64;
        self.blocked += blocked;
        self.values += values;
        self.restored += e.restored as u64;
        self.bytes_in += e.bytes_in;
        self.bytes_out += e.bytes_out;
        self.streamed += e.streamed as u64;
        self.latency_ms += e.latency_ms;
        let t = Tally {
            requests: 1,
            values,
            blocked,
        };
        bump(&mut self.providers, &e.provider, &t);
        bump(&mut self.clients, &e.client, &t);
        bump(&mut self.endpoints, e.endpoint.as_str(), &t);
        for (k, n) in &e.kinds {
            let kt = Tally {
                requests: 1,
                values: *n as u64,
                blocked: 0,
            };
            bump(&mut self.kinds, k, &kt);
        }
        if let Some(r) = &e.block_reason {
            bump(&mut self.block_reasons, r, &t);
        }
    }

    fn merge(&mut self, o: &Agg) {
        self.requests += o.requests;
        self.masked += o.masked;
        self.blocked += o.blocked;
        self.values += o.values;
        self.restored += o.restored;
        self.bytes_in += o.bytes_in;
        self.bytes_out += o.bytes_out;
        self.streamed += o.streamed;
        self.latency_ms += o.latency_ms;
        for (dst, src) in [
            (&mut self.kinds, &o.kinds),
            (&mut self.providers, &o.providers),
            (&mut self.clients, &o.clients),
            (&mut self.endpoints, &o.endpoints),
            (&mut self.block_reasons, &o.block_reasons),
        ] {
            for (k, t) in src {
                bump(dst, k, t);
            }
        }
    }
}

/// Headline numbers for a window.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Totals {
    /// Requests that went through Shadow's translation.
    pub requests: u64,
    /// Requests in which at least one value was masked.
    pub masked_requests: u64,
    /// Requests refused by policy.
    pub blocked: u64,
    /// Requests with nothing to protect.
    pub clean: u64,
    /// Values replaced before leaving the machine.
    pub values_protected: u64,
    /// Values translated back in responses.
    pub values_restored: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub streamed: u64,
    /// Distinct providers / clients seen.
    pub providers: u64,
    pub clients: u64,
    pub avg_latency_ms: u64,
}

/// One timeline bucket.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Point {
    /// Bucket start, unix ms.
    pub t: i64,
    pub requests: u64,
    pub masked: u64,
    pub values: u64,
    pub blocked: u64,
}

/// One breakdown row.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Breakdown {
    pub key: String,
    pub label: String,
    pub requests: u64,
    pub values: u64,
    pub blocked: u64,
}

/// Aggregated activity for a window (`GET /shadow/activity/summary`).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Summary {
    pub enabled: bool,
    pub window: String,
    /// Window start / end, unix ms.
    pub from: i64,
    pub to: i64,
    pub bucket_ms: i64,
    pub totals: Totals,
    pub timeline: Vec<Point>,
    /// By data kind, sorted by values.
    pub kinds: Vec<Breakdown>,
    /// By destination provider, sorted by requests.
    pub providers: Vec<Breakdown>,
    /// By client app, sorted by requests.
    pub clients: Vec<Breakdown>,
    pub endpoints: Vec<Breakdown>,
    pub block_reasons: Vec<Breakdown>,
    /// Id of the newest event (for incremental polling).
    pub last_id: u64,
    /// When the proxy started recording (unix ms).
    pub since: i64,
}

fn rows(
    map: &BTreeMap<String, Tally>,
    label: impl Fn(&str) -> String,
    by_values: bool,
) -> Vec<Breakdown> {
    let mut v: Vec<Breakdown> = map
        .iter()
        .map(|(k, t)| Breakdown {
            key: k.clone(),
            label: label(k),
            requests: t.requests,
            values: t.values,
            blocked: t.blocked,
        })
        .collect();
    if by_values {
        v.sort_by(|a, b| b.values.cmp(&a.values).then(b.requests.cmp(&a.requests)));
    } else {
        v.sort_by(|a, b| b.requests.cmp(&a.requests).then(b.values.cmp(&a.values)));
    }
    v
}

/// Per-minute counters for the last 48 hours, per-hour counters before that.
#[derive(Default)]
struct Rollup {
    minutes: BTreeMap<i64, Agg>,
    hours: BTreeMap<i64, Agg>,
    compacted_at: i64,
}

const MINUTE_HORIZON: i64 = 48 * 3600;

impl Rollup {
    fn add(&mut self, e: &Event, now: i64, retention: i64) {
        let s = e.ts.div_euclid(1000);
        if s < now - retention {
            return;
        }
        let slot = if s >= now - MINUTE_HORIZON {
            self.minutes.entry(s - s.rem_euclid(60)).or_default()
        } else {
            self.hours.entry(s - s.rem_euclid(3600)).or_default()
        };
        slot.add(e);
        if now - self.compacted_at >= 60 {
            self.compact(now, retention);
        }
    }

    fn compact(&mut self, now: i64, retention: i64) {
        self.compacted_at = now;
        let cut = now - MINUTE_HORIZON;
        let keep = self.minutes.split_off(&cut);
        let old = std::mem::replace(&mut self.minutes, keep);
        for (t, a) in old {
            self.hours
                .entry(t - t.rem_euclid(3600))
                .or_default()
                .merge(&a);
        }
        let floor = now - retention;
        self.hours = self.hours.split_off(&(floor - floor.rem_euclid(3600)));
    }

    fn summary(&self, from: i64, bucket: i64, points: usize) -> (Agg, Vec<Point>) {
        let mut total = Agg::default();
        let mut tl: Vec<Point> = (0..points)
            .map(|i| Point {
                t: (from + i as i64 * bucket) * 1000,
                ..Point::default()
            })
            .collect();
        for (t, a) in self.hours.range(from..).chain(self.minutes.range(from..)) {
            total.merge(a);
            let i = ((t - from) / bucket) as usize;
            if let Some(p) = tl.get_mut(i) {
                p.requests += a.requests;
                p.masked += a.masked;
                p.values += a.values;
                p.blocked += a.blocked;
            }
        }
        (total, tl)
    }
}

// ----------------------------------------------------------------------------------------
// The log

/// Recent events kept in memory for `GET /shadow/activity`.
pub const RECENT_CAPACITY: usize = 5000;

/// File name of the activity log inside [`crate::ShadowConfig::activity_dir`].
pub const LOG_FILE: &str = "activity.jsonl";

enum Msg {
    Line(String),
    Flush(mpsc::Sender<()>),
}

struct State {
    recent: VecDeque<Event>,
    rollup: Rollup,
    next_id: u64,
}

/// In-memory activity store with optional JSONL persistence. Cheap to call from the hot
/// path: one short mutex section and a channel send; file I/O happens on its own thread.
pub struct ActivityLog {
    state: Mutex<State>,
    writer: Option<mpsc::Sender<Msg>>,
    retention: i64,
    since: i64,
}

/// Current unix time in milliseconds.
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

impl ActivityLog {
    /// Memory-only log.
    pub fn in_memory(retention_days: u32) -> Self {
        Self {
            state: Mutex::new(State {
                recent: VecDeque::new(),
                rollup: Rollup::default(),
                next_id: 1,
            }),
            writer: None,
            retention: retention_days.max(1) as i64 * 86400,
            since: now_ms(),
        }
    }

    /// Open per `cfg`: with `persist`, load `<dir>/activity.jsonl` (dropping expired
    /// events) and append new events to it from a background thread.
    pub fn open(cfg: &ActivityConfig, dir: &Path) -> anyhow::Result<Self> {
        let mut log = Self::in_memory(cfg.retention_days);
        if !cfg.persist {
            return Ok(log);
        }
        crate::vault::create_private_dir(dir)?;
        let path = dir.join(LOG_FILE);
        log.load(&path, true);
        log.writer = Some(spawn_writer(path, cfg.max_file_mb.max(1) * 1024 * 1024));
        Ok(log)
    }

    /// Load the persisted log without writing anything (for offline `shadow activity`).
    pub fn read_only(cfg: &ActivityConfig, dir: &Path) -> Self {
        let mut log = Self::in_memory(cfg.retention_days);
        log.load(&dir.join(LOG_FILE), false);
        log
    }

    fn load(&mut self, path: &Path, prune: bool) {
        let now = now_ms();
        let floor = now - self.retention * 1000;
        let mut events = vec![];
        for p in [rotated(path), path.to_path_buf()] {
            let Ok(f) = std::fs::File::open(&p) else {
                continue;
            };
            let mut kept = vec![];
            let mut dropped = 0usize;
            for line in std::io::BufReader::new(f).lines().map_while(Result::ok) {
                match serde_json::from_str::<Event>(&line) {
                    Ok(e) if e.ts >= floor => {
                        kept.push(line);
                        events.push(e);
                    }
                    _ => dropped += 1,
                }
            }
            if prune && dropped > 0 {
                let res = if kept.is_empty() {
                    std::fs::remove_file(&p).map_err(anyhow::Error::from)
                } else {
                    let mut body = kept.join("\n");
                    body.push('\n');
                    crate::vault::write_atomic(&p, body.as_bytes(), true)
                };
                if let Err(e) = res {
                    tracing::warn!("could not prune {}: {e:#}", p.display());
                }
            }
        }
        events.sort_by_key(|e| (e.ts, e.id));
        if let Some(first) = events.first() {
            self.since = self.since.min(first.ts);
        }
        let st = self.state.get_mut().unwrap_or_else(|e| e.into_inner());
        let now_s = now / 1000;
        for e in events {
            st.next_id = st.next_id.max(e.id + 1);
            st.rollup.add(&e, now_s, self.retention);
            push_recent(&mut st.recent, e);
        }
        st.rollup.compact(now_s, self.retention);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Record an event; assigns `id` (and `ts` if zero). Returns the id.
    pub fn record(&self, mut e: Event) -> u64 {
        if e.ts == 0 {
            e.ts = now_ms();
        }
        let now_s = now_ms() / 1000;
        let id = {
            let mut st = self.lock();
            e.id = st.next_id;
            st.next_id += 1;
            st.rollup.add(&e, now_s, self.retention);
            let line = self
                .writer
                .as_ref()
                .and_then(|_| serde_json::to_string(&e).ok());
            let id = e.id;
            push_recent(&mut st.recent, e);
            if let (Some(w), Some(line)) = (&self.writer, line) {
                let _ = w.send(Msg::Line(line));
            }
            id
        };
        id
    }

    /// Wait until every recorded event has been written to disk.
    pub fn flush(&self) {
        if let Some(w) = &self.writer {
            let (tx, rx) = mpsc::channel();
            if w.send(Msg::Flush(tx)).is_ok() {
                let _ = rx.recv_timeout(Duration::from_secs(5));
            }
        }
    }

    /// Recent events, newest first: ids greater than `since`, at most `limit`, optionally
    /// only those whose action is in `actions`.
    pub fn recent(&self, since: Option<u64>, limit: usize, actions: &[Action]) -> Vec<Event> {
        let st = self.lock();
        st.recent
            .iter()
            .rev()
            .take_while(|e| since.is_none_or(|s| e.id > s))
            .filter(|e| actions.is_empty() || actions.contains(&e.action))
            .take(limit)
            .cloned()
            .collect()
    }

    /// Id of the newest event (0 if none).
    pub fn last_id(&self) -> u64 {
        self.lock().next_id.saturating_sub(1)
    }

    /// Summary of `window` ending now.
    pub fn summary(&self, window: Window) -> Summary {
        self.summary_at(window, now_ms())
    }

    /// Summary of `window` ending at `now_ms`.
    pub fn summary_at(&self, window: Window, now_ms: i64) -> Summary {
        let now = now_ms.div_euclid(1000);
        let (from, bucket, points) = window.range(now);
        let st = self.lock();
        let (a, timeline) = st.rollup.summary(from, bucket, points);
        let last_id = st.next_id.saturating_sub(1);
        drop(st);
        Summary {
            enabled: true,
            window: window.as_str().into(),
            from: from * 1000,
            to: now_ms,
            bucket_ms: bucket * 1000,
            totals: Totals {
                requests: a.requests,
                masked_requests: a.masked,
                blocked: a.blocked,
                clean: a.requests - a.masked - a.blocked,
                values_protected: a.values,
                values_restored: a.restored,
                bytes_in: a.bytes_in,
                bytes_out: a.bytes_out,
                streamed: a.streamed,
                providers: a.providers.len() as u64,
                clients: a.clients.len() as u64,
                avg_latency_ms: a.latency_ms.checked_div(a.requests).unwrap_or(0),
            },
            timeline,
            kinds: rows(&a.kinds, kind_label, true),
            providers: rows(&a.providers, provider_label, false),
            clients: rows(&a.clients, str::to_string, false),
            endpoints: rows(&a.endpoints, endpoint_label, false),
            block_reasons: rows(&a.block_reasons, block_reason_label, false),
            last_id,
            since: self.since,
        }
    }
}

fn push_recent(q: &mut VecDeque<Event>, e: Event) {
    if q.len() >= RECENT_CAPACITY {
        q.pop_front();
    }
    q.push_back(e);
}

fn rotated(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".1");
    PathBuf::from(s)
}

fn open_append(path: &Path) -> std::io::Result<std::fs::File> {
    let mut o = std::fs::OpenOptions::new();
    o.create(true).append(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        o.mode(0o600);
    }
    o.open(path)
}

fn spawn_writer(path: PathBuf, max_bytes: u64) -> mpsc::Sender<Msg> {
    let (tx, rx) = mpsc::channel::<Msg>();
    let spawned = std::thread::Builder::new()
        .name("shadow-activity".into())
        .spawn(move || {
            let mut file: Option<std::fs::File> = None;
            let mut size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            for msg in rx {
                let line = match msg {
                    Msg::Flush(done) => {
                        if let Some(f) = file.as_mut() {
                            let _ = f.flush();
                        }
                        let _ = done.send(());
                        continue;
                    }
                    Msg::Line(l) => l,
                };
                if size + line.len() as u64 + 1 > max_bytes && size > 0 {
                    file = None;
                    if let Err(e) = std::fs::rename(&path, rotated(&path)) {
                        tracing::warn!("could not rotate activity log: {e}");
                    }
                    size = 0;
                }
                if file.is_none() {
                    match open_append(&path) {
                        Ok(f) => file = Some(f),
                        Err(e) => {
                            tracing::warn!("activity log {} unavailable: {e}", path.display());
                            continue;
                        }
                    }
                }
                if let Some(f) = file.as_mut() {
                    if f.write_all(format!("{line}\n").as_bytes()).is_ok() {
                        size += line.len() as u64 + 1;
                    }
                }
            }
        });
    if let Err(e) = spawned {
        tracing::warn!("activity writer thread failed to start: {e}");
    }
    tx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(ts: i64, provider: &str, client: &str, action: Action, kinds: &[(&str, u32)]) -> Event {
        Event {
            ts,
            provider: provider.into(),
            host: format!("api.{provider}.com"),
            client: client.into(),
            mode: "proxy".into(),
            method: "POST".into(),
            endpoint: Endpoint::Chat,
            action,
            kinds: kinds.iter().map(|(k, n)| (k.to_string(), *n)).collect(),
            block_reason: (action == Action::Blocked).then(|| "image_blocked".into()),
            restored: 1,
            status: 200,
            ..Event::default()
        }
    }

    #[test]
    fn providers_map_from_hosts() {
        assert_eq!(provider_for_host("api.openai.com"), "openai");
        assert_eq!(provider_for_host("API.Anthropic.com"), "anthropic");
        assert_eq!(
            provider_for_host("generativelanguage.googleapis.com"),
            "gemini"
        );
        assert_eq!(
            provider_for_host("us-central1-aiplatform.googleapis.com"),
            "vertex"
        );
        assert_eq!(provider_for_host("chatgpt.com"), "chatgpt");
        assert_eq!(provider_for_host("my-res.openai.azure.com"), "azure");
        assert_eq!(
            provider_for_host("bedrock-runtime.eu-central-1.amazonaws.com"),
            "bedrock"
        );
        assert_eq!(provider_for_host("127.0.0.1"), "local");
        assert_eq!(provider_for_host("[::1]"), "local");
        assert_eq!(provider_for_host("llm.example.net"), "other");
        assert_eq!(provider_label("openai"), "OpenAI");
        assert_eq!(provider_label("anthropic"), "Anthropic");
        assert_eq!(provider_label("gemini"), "Google Gemini");
        assert_eq!(provider_label("mycorp"), "mycorp");
    }

    #[test]
    fn clients_from_user_agents() {
        let c = |ua: &str| client_from_user_agent(Some(ua));
        assert_eq!(c("claude-cli/2.0.14 (external, cli)"), "Claude Code");
        assert_eq!(c("OpenAI/Python 1.51.0"), "OpenAI Python SDK");
        assert_eq!(c("OpenAI/JS 4.67.1"), "OpenAI Node SDK");
        assert_eq!(c("Anthropic/Python 0.39.0"), "Anthropic Python SDK");
        assert_eq!(c("curl/8.7.1"), "curl");
        assert_eq!(c("python-requests/2.32.3"), "Python requests");
        assert_eq!(c("python-httpx/0.27.2"), "Python httpx");
        assert_eq!(
            c("Mozilla/5.0 (Macintosh) AppleWebKit/537.36 Cursor/0.42.3 Chrome/124 Electron/30"),
            "Cursor"
        );
        assert_eq!(
            c("ChatGPT/1.2025.084 (Mac OS X 14.4; arm64; build 1)"),
            "ChatGPT Desktop"
        );
        assert_eq!(c("codex_cli_rs/0.1.0"), "Codex CLI");
        assert_eq!(c("node"), "Node.js");
        assert_eq!(c("Go-http-client/1.1"), "Go");
        assert_eq!(
            c("Mozilla/5.0 (Windows NT 10.0) Chrome/120 Safari/537"),
            "Web browser"
        );
        // Unknown agents: product token only, never the comment.
        assert_eq!(c("AcmeBot/1.0 (jane@acme.com; build-host-7)"), "AcmeBot");
        assert_eq!(c("weird<script>/1"), "weirdscript");
        assert_eq!(client_from_user_agent(None), "Unknown");
        assert_eq!(client_from_user_agent(Some("  ")), "Unknown");
    }

    #[test]
    fn endpoints_from_paths() {
        assert_eq!(endpoint_for_path("/v1/chat/completions"), Endpoint::Chat);
        assert_eq!(endpoint_for_path("/v1/messages"), Endpoint::Chat);
        assert_eq!(endpoint_for_path("/v1/responses"), Endpoint::Chat);
        assert_eq!(
            endpoint_for_path("/v1beta/models/gemini-2.0-flash:streamGenerateContent"),
            Endpoint::Chat
        );
        assert_eq!(endpoint_for_path("/v1/embeddings"), Endpoint::Embeddings);
        assert_eq!(
            endpoint_for_path("/v1beta/models/x:batchEmbedContents"),
            Endpoint::Embeddings
        );
        assert_eq!(endpoint_for_path("/v1/files"), Endpoint::Files);
        assert_eq!(endpoint_for_path("/v1/models"), Endpoint::Other);
    }

    #[test]
    fn summary_aggregates_by_kind_provider_client_and_time() {
        let log = ActivityLog::in_memory(30);
        let now = now_ms();
        log.record(ev(
            now - 10_000,
            "openai",
            "curl",
            Action::Masked,
            &[("email", 2), ("person", 1)],
        ));
        log.record(ev(
            now - 20_000,
            "anthropic",
            "Claude Code",
            Action::Masked,
            &[("email", 1)],
        ));
        log.record(ev(
            now - 30_000,
            "anthropic",
            "Claude Code",
            Action::Clean,
            &[],
        ));
        log.record(ev(now - 40_000, "openai", "curl", Action::Blocked, &[]));
        // Outside the 1h window but inside 24h.
        log.record(ev(
            now - 3 * 3600 * 1000,
            "gemini",
            "Cursor",
            Action::Masked,
            &[("secret", 1)],
        ));

        let s = log.summary_at(Window::Hour, now);
        assert_eq!(s.totals.requests, 4);
        assert_eq!(s.totals.masked_requests, 2);
        assert_eq!(s.totals.blocked, 1);
        assert_eq!(s.totals.clean, 1);
        assert_eq!(s.totals.values_protected, 4);
        assert_eq!(s.totals.values_restored, 4);
        assert_eq!(s.totals.providers, 2);
        assert_eq!(s.kinds[0].key, "email");
        assert_eq!(s.kinds[0].values, 3);
        assert_eq!(s.kinds[0].requests, 2);
        assert_eq!(s.kinds[0].label, "Email addresses");
        assert_eq!(s.providers.len(), 2);
        assert!(s
            .providers
            .iter()
            .any(|p| p.label == "Anthropic" && p.requests == 2));
        assert_eq!(s.block_reasons[0].key, "image_blocked");
        assert_eq!(s.timeline.len(), 60);
        assert_eq!(s.timeline.iter().map(|p| p.requests).sum::<u64>(), 4);
        assert_eq!(s.timeline.iter().map(|p| p.values).sum::<u64>(), 4);

        let d = log.summary_at(Window::Day, now);
        assert_eq!(d.totals.requests, 5);
        assert_eq!(d.timeline.len(), 48);
        assert!(d.clients.iter().any(|c| c.key == "Cursor"));

        let recent = log.recent(None, 10, &[]);
        assert_eq!(recent.len(), 5);
        assert!(recent[0].id > recent[1].id, "newest first");
        let incidents = log.recent(None, 10, &[Action::Masked, Action::Blocked]);
        assert_eq!(incidents.len(), 4);
        assert_eq!(log.recent(Some(log.last_id()), 10, &[]).len(), 0);
    }

    #[test]
    fn rollup_compacts_old_minutes_into_hours() {
        let log = ActivityLog::in_memory(30);
        let now = now_ms();
        for d in [3, 5, 10] {
            log.record(ev(
                now - d * 86400 * 1000,
                "openai",
                "curl",
                Action::Masked,
                &[("ip", 1)],
            ));
        }
        log.record(ev(
            now - 40 * 86400 * 1000,
            "openai",
            "curl",
            Action::Masked,
            &[("ip", 1)],
        ));
        let w = log.summary_at(Window::Week, now);
        assert_eq!(w.totals.requests, 2);
        let m = log.summary_at(Window::Month, now);
        assert_eq!(m.totals.requests, 3, "older than retention is dropped");
    }

    #[test]
    fn persists_privately_and_reloads_with_retention() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = ActivityConfig::default();
        let adir = dir.path().join("activity");
        {
            let log = ActivityLog::open(&cfg, &adir).unwrap();
            log.record(ev(0, "openai", "curl", Action::Masked, &[("email", 1)]));
            log.record(ev(0, "anthropic", "curl", Action::Clean, &[]));
            log.flush();
        }
        let path = adir.join(LOG_FILE);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let m = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(m, 0o600);
            let d = std::fs::metadata(&adir).unwrap().permissions().mode() & 0o777;
            assert_eq!(d, 0o700);
        }
        // An expired line is pruned on the next open.
        let old = ev(
            now_ms() - 90 * 86400 * 1000,
            "openai",
            "curl",
            Action::Clean,
            &[],
        );
        let mut f = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(
            f,
            "{}",
            serde_json::to_string(&Event { id: 99, ..old }).unwrap()
        )
        .unwrap();
        drop(f);
        let log = ActivityLog::open(&cfg, &adir).unwrap();
        assert_eq!(log.summary(Window::Hour).totals.requests, 2);
        assert_eq!(log.last_id(), 2);
        let id = log.record(ev(0, "openai", "curl", Action::Clean, &[]));
        assert_eq!(id, 3);
        log.flush();
        let lines = std::fs::read_to_string(&path).unwrap();
        assert_eq!(lines.lines().count(), 3);
        let ro = ActivityLog::read_only(&cfg, &adir);
        assert_eq!(ro.summary(Window::Day).totals.requests, 3);
    }

    #[test]
    fn rotates_at_size_cap() {
        let dir = tempfile::tempdir().unwrap();
        let cfg = ActivityConfig {
            max_file_mb: 1,
            ..ActivityConfig::default()
        };
        let log = ActivityLog::open(&cfg, dir.path()).unwrap();
        for _ in 0..6000 {
            log.record(ev(0, "openai", "curl", Action::Masked, &[("email", 1)]));
        }
        log.flush();
        assert!(dir.path().join("activity.jsonl.1").exists());
        assert!(std::fs::metadata(dir.path().join(LOG_FILE)).unwrap().len() <= 1024 * 1024);
    }

    #[test]
    fn window_ranges_are_aligned() {
        let now = 1_700_000_000;
        for w in Window::ALL {
            let (from, bucket, points) = w.range(now);
            assert_eq!(from % bucket, 0);
            assert!(from + bucket * points as i64 > now);
            assert!(from + bucket * (points as i64 - 1) <= now);
        }
        let (from, bucket, points) = Window::Today.range(now);
        assert_eq!(bucket, 3600);
        assert!(from <= now && (now - from) < 86400 + 3600);
        assert!(points >= 1);
        assert_eq!(Window::parse("7d"), Some(Window::Week));
        assert_eq!(Window::parse("nope"), None);
    }
}
