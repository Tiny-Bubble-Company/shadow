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
