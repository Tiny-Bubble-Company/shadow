//! Configuration (`shadow.toml`).
//!
//! Every field has a sensible default, so an empty file (or
//! [`ShadowConfig::from_dir`]) is a valid configuration.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Top-level Shadow configuration.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ShadowConfig {
    /// Directory holding `persona.json`, `vault.bin`, `vault.key`, `terms.json` and the CA.
    pub data_dir: PathBuf,
    /// Persona (fake identity) settings.
    pub persona: PersonaConfig,
    /// Organisation names (legal names). The first one maps to the persona company.
    pub organization: Vec<String>,
    /// Extra protected dictionary terms: product codenames, customers, projects, people.
    pub terms: Vec<String>,
    /// Domains that are always treated as internal and masked (e.g. `acme.com`, `corp.acme.io`).
    pub internal_domains: Vec<String>,
    /// Public domains that are never masked (added to the built-in allow list).
    pub allow_domains: Vec<String>,
    /// Values that are never masked, even when detected (case-insensitive).
    pub allow_terms: Vec<String>,
    /// Toggle individual built-in detectors.
    pub detectors: DetectorToggles,
    /// Optional local NER sidecar (see `ml/`).
    pub model_detector: ModelDetectorConfig,
    /// Proxy / gateway settings used by `shadow-proxy`.
    pub proxy: ProxyConfig,
    /// Intent-aware treatment of detected values (see [`crate::policy`]).
    pub policy: PolicyConfig,
    /// Masking the contents of uploaded files (see [`crate::files`]).
    pub files: FilesConfig,
    /// Optional Microsoft Presidio Analyzer as an extra detector.
    pub presidio: PresidioConfig,
    /// The privacy-safe activity feed behind the dashboard, `shadow top` and the tray app.
    pub activity: ActivityConfig,
}

/// Activity feed (see [`crate::activity`]): counts and categories only, never values.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ActivityConfig {
    /// Record activity events at all.
    pub enabled: bool,
    /// Append events to `<data_dir>/activity/activity.jsonl` (0600) so history survives
    /// restarts. Off = in memory only.
    pub persist: bool,
    /// Drop events older than this many days.
    pub retention_days: u32,
    /// Rotate the log file when it grows past this size (one previous file is kept).
    pub max_file_mb: u64,
    /// Serve `/shadow/activity*` and `/shadow/ui` to non-loopback peers. Off by default;
    /// only turn it on behind your own access control (e.g. a Docker port bound to localhost).
    pub allow_remote: bool,
}

impl Default for ActivityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            persist: true,
            retention_days: 30,
            max_file_mb: 16,
            allow_remote: false,
        }
    }
}

/// Persona settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PersonaConfig {
    /// ISO country code of the fake region (`CN`, `JP`, `BR`, ...). `None`/`"auto"` picks a
    /// region far from the machine's real timezone.
    pub region: Option<String>,
    /// Override the generated company name.
    pub company: Option<String>,
    /// Override the generated company domain.
    pub domain: Option<String>,
}

/// Built-in detector switches. All enabled by default.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DetectorToggles {
    pub email: bool,
    pub phone: bool,
    pub person: bool,
    pub address: bool,
    pub iban: bool,
    pub credit_card: bool,
    pub national_id: bool,
    pub ip: bool,
    pub mac: bool,
    pub hostname: bool,
    pub secret: bool,
    pub path: bool,
    /// Organisation names with a legal form (GmbH, Ltd, S.A., ...).
    pub org: bool,
    /// Usernames from context (`ssh user@host`, `username=`, basic-auth tuples, URL userinfo).
    pub username: bool,
    /// Dates in a birth context ("born", "DOB", "Geburtsdatum", ...).
    pub date_of_birth: bool,
    /// Labelled fields (`Name:`, `Address:`, `Tel:`), CSV columns and signature names.
    pub fields: bool,
    /// Keep well-known public names (companies, places, public figures, standards) that
    /// name-like detectors (ML sidecars, rule-based names) would otherwise mask.
    pub public_allowlist: bool,
    /// Protect local machine facts (OS user, hostname, git identity, local domain).
    pub local_facts: bool,
}

impl Default for DetectorToggles {
    fn default() -> Self {
        Self {
            email: true,
            phone: true,
            person: true,
            address: true,
            iban: true,
            credit_card: true,
            national_id: true,
            ip: true,
            mac: true,
            hostname: true,
            secret: true,
            path: true,
            org: true,
            username: true,
            date_of_birth: true,
            fields: true,
            public_allowlist: true,
            local_facts: true,
        }
    }
}

/// Settings for the optional HTTP NER sidecar.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ModelDetectorConfig {
    pub enabled: bool,
    /// Must be a loopback URL, e.g. `http://127.0.0.1:8878/detect`.
    pub url: String,
    pub timeout_ms: u64,
    pub min_score: f32,
}

impl Default for ModelDetectorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "http://127.0.0.1:8878/detect".into(),
            timeout_ms: 300,
            min_score: 0.6,
        }
    }
}

/// How a detected value is sent to the model.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Treatment {
    /// A realistic, format-preserving fake from the persona; restored in responses and tool calls.
    Surrogate,
    /// A typed placeholder such as `[PERSON_0007]`; restored in responses and tool calls.
    Token,
    /// The real value. Only for action targets the provider itself acts on; always logged.
    Keep,
    /// Removed for good (`[REDACTED_SECRET]`); never restored.
    Redact,
}

/// Intent-aware policy: which [`Treatment`] each kind of value gets, per request intent.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PolicyConfig {
    /// Classify each request's intent (act / report / code / converse). Off = surrogate everything.
    pub intent: bool,
    pub act: ActPolicy,
    /// Treatment per kind (`person`, `email`, ...) when the request is a report over data;
    /// `default` covers the rest.
    pub report: BTreeMap<String, Treatment>,
    /// Treatment per kind that applies whatever the intent, e.g. `secret = "redact"`.
    pub always: BTreeMap<String, Treatment>,
}

/// Values an action or message is aimed at (the recipient of an email, the number to call).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ActPolicy {
    /// The recipient of a message or action the model writes or prepares (an email to Jane,
    /// a call to a number). `keep` (default) shows the model the real recipient so it writes
    /// to the right person; `surrogate` hides it and relies on restoring it in the answer.
    pub targets: Treatment,
    /// When the action runs in the provider's cloud (hosted MCP servers, server-side tools),
    /// Shadow can't restore values there; `keep` sends the real target so the action works.
    pub remote_tool_targets: Treatment,
}

impl Default for ActPolicy {
    fn default() -> Self {
        Self {
            targets: Treatment::Keep,
            remote_tool_targets: Treatment::Keep,
        }
    }
}

impl Default for PolicyConfig {
    fn default() -> Self {
        let report = [
            "person",
            "email",
            "phone",
            "address",
            "national_id",
            "iban",
            "credit_card",
            "date",
        ]
        .iter()
        .map(|k| (k.to_string(), Treatment::Token))
        .chain([("default".to_string(), Treatment::Surrogate)])
        .collect();
        // Secrets get fakes like everything else, so "fix this config" comes back with your
        // real key restored locally; set `secret = "redact"` to strip them instead.
        Self {
            intent: true,
            act: ActPolicy::default(),
            report,
            always: BTreeMap::new(),
        }
    }
}

/// What to do with PDFs and images inside requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileAction {
    /// PDFs: extract the text, mask it, send it as text.
    Text,
    /// Refuse the request with an error explaining why.
    Block,
    /// Forward untouched.
    Pass,
}

/// File content masking.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct FilesConfig {
    pub enabled: bool,
    pub pdf: FileAction,
    /// Images can't be read without OCR; pass them through or block them.
    pub images: FileAction,
    /// Files larger than this are forwarded untouched (with a warning) or blocked if `block_oversize`.
    pub max_bytes: usize,
    pub block_oversize: bool,
}

impl Default for FilesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            pdf: FileAction::Text,
            images: FileAction::Pass,
            max_bytes: 50 * 1024 * 1024,
            block_oversize: false,
        }
    }
}

/// Microsoft Presidio Analyzer (https://microsoft.github.io/presidio/) as an extra detector.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct PresidioConfig {
    pub enabled: bool,
    /// The analyzer's `/analyze` endpoint.
    pub url: String,
    pub language: String,
    pub min_score: f32,
    /// Presidio entity names to ask for; empty means all.
    pub entities: Vec<String>,
    pub timeout_ms: u64,
}

impl Default for PresidioConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "http://127.0.0.1:5002/analyze".into(),
            language: "en".into(),
            min_score: 0.5,
            entities: vec![],
            timeout_ms: 800,
        }
    }
}

/// Proxy settings (consumed by `shadow-proxy`).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct ProxyConfig {
    /// Listen address for forward-proxy + gateway mode.
    pub listen: String,
    /// Extra AI hosts to intercept (glob `*` allowed), added to the built-in list.
    pub extra_hosts: Vec<String>,
    /// Hosts never intercepted even if they match the AI list.
    pub bypass_hosts: Vec<String>,
    /// Optional egress relay for intercepted traffic: `http://host:port`, `socks5://host:port`
    /// or `socks5h://host:port`.
    pub upstream_proxy: Option<String>,
    /// Request headers removed before forwarding (case-insensitive).
    pub strip_headers: Vec<String>,
    /// Request headers whose value is replaced with a persona value. Map header -> template
    /// (`{user_id}`, `{hostname}`, `{company}`, `{domain}`, `{username}`).
    pub persona_headers: BTreeMap<String, String>,
    /// Extra gateway routes: `/gw/<name>/...` -> `<base>/...`. Overrides built-ins.
    pub routes: BTreeMap<String, String>,
}

impl Default for ProxyConfig {
    fn default() -> Self {
        let persona_headers = [
            ("x-request-origin", "{hostname}"),
            ("x-user-id", "{user_id}"),
            ("x-client-hostname", "{hostname}"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        Self {
            listen: "127.0.0.1:8877".into(),
            extra_hosts: vec![],
            bypass_hosts: vec![],
            upstream_proxy: None,
            strip_headers: [
                "openai-organization",
                "x-forwarded-for",
                "x-real-ip",
                "forwarded",
                "via",
                "x-client-ip",
                "x-stainless-os",
                "x-stainless-arch",
                "x-stainless-runtime-version",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
            persona_headers,
            routes: BTreeMap::new(),
        }
    }
}

impl Default for ShadowConfig {
    fn default() -> Self {
        Self::from_dir(default_data_dir())
    }
}

/// Default data directory: `$SHADOW_HOME`, else the platform data dir + `shadow`.
pub fn default_data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("SHADOW_HOME") {
        return PathBuf::from(p);
    }
    dirs::data_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("."))
        .join("shadow")
}

impl ShadowConfig {
    /// Defaults rooted at `data_dir`.
    pub fn from_dir(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            persona: PersonaConfig::default(),
            organization: vec![],
            terms: vec![],
            internal_domains: vec![],
            allow_domains: vec![],
            allow_terms: vec![],
            detectors: DetectorToggles::default(),
            model_detector: ModelDetectorConfig::default(),
            proxy: ProxyConfig::default(),
            policy: PolicyConfig::default(),
            files: FilesConfig::default(),
            presidio: PresidioConfig::default(),
            activity: ActivityConfig::default(),
        }
    }

    /// Load a `shadow.toml`. A missing or relative `data_dir` is resolved against the
    /// config file's directory.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading config {}", path.display()))?;
        let value: toml::Value =
            toml::from_str(&raw).with_context(|| format!("parsing {}", path.display()))?;
        let has_dir = value.get("data_dir").is_some();
        let mut cfg: ShadowConfig = value
            .try_into()
            .with_context(|| format!("invalid config {}", path.display()))?;
        let base = path.parent().map(Path::to_path_buf).unwrap_or_default();
        if !has_dir {
            cfg.data_dir = base;
        } else if cfg.data_dir.is_relative() {
            cfg.data_dir = base.join(&cfg.data_dir);
        }
        Ok(cfg)
    }

    /// Add extra protected terms (e.g. an organisation name).
    pub fn with_terms(mut self, terms: Vec<String>) -> Self {
        for t in terms {
            let t = t.trim().to_string();
            if !t.is_empty() && !self.terms.contains(&t) {
                self.terms.push(t);
            }
        }
        self
    }

    /// Set the persona region (ISO code such as `CN`).
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.persona.region = Some(region.into());
        self
    }

    /// Add organisation names (first maps to the persona company).
    pub fn with_organization(mut self, orgs: Vec<String>) -> Self {
        self.organization.extend(orgs);
        self
    }

    /// Path of the persona file.
    pub fn persona_path(&self) -> PathBuf {
        self.data_dir.join("persona.json")
    }
    /// Path of the encrypted vault.
    pub fn vault_path(&self) -> PathBuf {
        self.data_dir.join("vault.bin")
    }
    /// Path of the vault key (0600).
    pub fn key_path(&self) -> PathBuf {
        self.data_dir.join("vault.key")
    }
    /// Directory of the activity log.
    pub fn activity_dir(&self) -> PathBuf {
        self.data_dir.join("activity")
    }
    /// Path of runtime-added terms.
    pub fn terms_path(&self) -> PathBuf {
        self.data_dir.join("terms.json")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_config_parses() {
        let raw = include_str!("../../../shadow.example.toml");
        let cfg: ShadowConfig = toml::from_str(raw).unwrap();
        assert_eq!(cfg.organization, vec!["Acme GmbH".to_string()]);
        assert_eq!(cfg.proxy.persona_headers.len(), 3);
    }

    #[test]
    fn load_resolves_relative_data_dir() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("shadow.toml");
        std::fs::write(
            &p,
            "terms = [\"Falcon\"]\n[persona]\nregion = \"JP\"\n[proxy]\nlisten = \"127.0.0.1:9999\"\n",
        )
        .unwrap();
        let cfg = ShadowConfig::load(&p).unwrap();
        assert_eq!(cfg.data_dir, dir.path());
        assert_eq!(cfg.persona.region.as_deref(), Some("JP"));
        assert_eq!(cfg.proxy.listen, "127.0.0.1:9999");
        assert!(cfg.detectors.email);
        let cfg = cfg.with_terms(vec!["Acme GmbH".into(), "Falcon".into()]);
        assert_eq!(
            cfg.terms,
            vec!["Falcon".to_string(), "Acme GmbH".to_string()]
        );
    }
}
