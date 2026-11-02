//! The catalog of AI services Shadow recognises: model makers' APIs, inference clouds,
//! routers, coding agents' backends and AI chat apps, each with its API hosts.
//!
//! The catalog lives in `catalog.json` at the root of this crate and is the single source
//! for which hosts the proxy masks (`mask: true`), which it only observes (recognised and
//! recorded as "seen, not protected", never decrypted), and how the dashboard names and
//! groups them. The docs' providers page is generated from the same file.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

/// One AI service.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provider {
    /// Stable id used in activity events, e.g. `openai`.
    pub id: String,
    /// Display name, e.g. `OpenAI`.
    pub name: String,
    /// `model_api`, `inference`, `router`, `coding_agent`, `chat_app` or `local`.
    pub category: String,
    /// Request format on its hosts: `openai`, `anthropic`, `gemini`, `cohere`, `custom` or
    /// `binary` (protobuf, gRPC, AWS event streams).
    pub api: String,
    /// API hosts; `*` matches any run of characters.
    pub hosts: Vec<String>,
    /// Shadow masks this service's traffic. Otherwise it only recognises and records it.
    #[serde(default)]
    pub mask: bool,
    /// OpenAI-compatible base URL, when there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// Official page where the hosts are documented.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    /// Where the company is based (`global`, `cn`, `eu`, ...).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

static CATALOG: LazyLock<Vec<Provider>> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../catalog.json")).expect("catalog.json is valid")
});

/// Every provider in the catalog.
pub fn providers() -> &'static [Provider] {
    &CATALOG
}

/// Glob match where `*` matches any run of characters (case-insensitive).
pub fn glob_match(pattern: &str, host: &str) -> bool {
    let (p, h) = (pattern.to_ascii_lowercase(), host.to_ascii_lowercase());
    let parts: Vec<&str> = p.split('*').collect();
    if parts.len() == 1 {
        return p == h;
    }
    let mut rest = h.as_str();
    for (i, part) in parts.iter().enumerate() {
        if i == 0 {
            match rest.strip_prefix(part) {
                Some(r) => rest = r,
                None => return false,
            }
        } else if i == parts.len() - 1 {
            return rest.ends_with(part);
        } else {
            match rest.find(part) {
                Some(idx) => rest = &rest[idx + part.len()..],
                None => return false,
            }
        }
    }
    true
}

/// The provider serving `host`: an exact host wins over a glob, a longer glob over a shorter.
pub fn find(host: &str) -> Option<&'static Provider> {
    let host = host.trim_end_matches('.').trim_matches(['[', ']']);
    let mut best: Option<(&'static Provider, usize)> = None;
    for p in providers() {
        for pat in &p.hosts {
            if !glob_match(pat, host) {
                continue;
            }
            let score = if pat.contains('*') {
                pat.len()
            } else {
                usize::MAX
            };
            if best.is_none_or(|(_, s)| score > s) {
                best = Some((p, score));
            }
        }
    }
    best.map(|(p, _)| p)
}

/// A provider by id.
pub fn by_id(id: &str) -> Option<&'static Provider> {
    providers().iter().find(|p| p.id == id)
}

/// Hosts Shadow masks as API traffic (chat apps are handled separately by the proxy).
pub fn masked_hosts() -> impl Iterator<Item = &'static str> {
    providers()
        .iter()
        .filter(|p| p.mask && p.category != "chat_app")
        .flat_map(|p| p.hosts.iter().map(String::as_str))
}

/// Hosts Shadow recognises but does not decrypt: their traffic is recorded as seen.
pub fn observed_hosts() -> impl Iterator<Item = &'static str> {
    providers()
        .iter()
        .filter(|p| !p.mask)
        .flat_map(|p| p.hosts.iter().map(String::as_str))
}

/// Display name of a category.
pub fn category_label(category: &str) -> &'static str {
    match category {
        "model_api" => "Model API",
        "inference" => "Inference cloud",
        "router" => "Model router",
        "coding_agent" => "Coding agent",
        "chat_app" => "AI chat app",
        "local" => "Local model",
        _ => "Other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_is_well_formed() {
        let mut ids = HashSet::new();
        for p in providers() {
            assert!(ids.insert(p.id.as_str()), "duplicate id {}", p.id);
            assert!(
                p.id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "bad id {}",
                p.id
            );
            assert!(!p.name.is_empty() && !p.hosts.is_empty(), "{}", p.id);
            assert!(
                [
                    "model_api",
                    "inference",
                    "router",
                    "coding_agent",
                    "chat_app"
                ]
                .contains(&p.category.as_str()),
                "{}: category {}",
                p.id,
                p.category
            );
            assert!(
                [
                    "openai",
                    "anthropic",
                    "gemini",
                    "cohere",
                    "custom",
                    "binary"
                ]
                .contains(&p.api.as_str()),
                "{}: api {}",
                p.id,
                p.api
            );
            assert!(!(p.mask && p.api == "binary"), "{} cannot be masked", p.id);
            for h in &p.hosts {
                assert_eq!(h, &h.to_ascii_lowercase(), "{h}");
                assert!(!h.contains('/') && !h.contains(':'), "host only: {h}");
                // Never a whole company or cloud.
                for broad in [
                    "*.amazonaws.com",
                    "*.google.com",
                    "*.googleapis.com",
                    "*.microsoft.com",
                    "*.azure.com",
                    "*.cloudflare.com",
                    "*",
                ] {
                    assert_ne!(h, broad, "{}: glob too broad", p.id);
                }
            }
        }
    }

    #[test]
    fn hosts_resolve_to_providers() {
        assert_eq!(find("api.openai.com").unwrap().id, "openai");
        assert_eq!(find("API.Anthropic.com").unwrap().id, "anthropic");
        assert_eq!(find("myres.openai.azure.com").unwrap().id, "azure");
        assert_eq!(
            find("bedrock-runtime.eu-west-1.amazonaws.com").unwrap().id,
            "bedrock"
        );
        assert_eq!(
            find("europe-west4-aiplatform.googleapis.com").unwrap().id,
            "vertex"
        );
        assert!(find("openai.azure.com.evil.com").is_none());
        assert!(find("github.com").is_none());
        assert!(masked_hosts().any(|h| h == "api.openai.com"));
        assert!(!masked_hosts().any(|h| h == "chatgpt.com"));
    }
}
