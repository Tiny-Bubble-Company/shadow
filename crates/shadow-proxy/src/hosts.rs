//! Which hosts are intercepted, and the gateway route table.

use std::collections::BTreeMap;

/// Built-in AI API hosts that are intercepted (glob `*` allowed).
pub const AI_HOSTS: &[&str] = &[
    "api.openai.com",
    "api.anthropic.com",
    "openrouter.ai",
    "generativelanguage.googleapis.com",
    "*-aiplatform.googleapis.com",
    "aiplatform.googleapis.com",
    "api.mistral.ai",
    "api.groq.com",
    "api.together.xyz",
    "api.together.ai",
    "api.deepseek.com",
    "api.x.ai",
    "*.openai.azure.com",
    "*.services.ai.azure.com",
    "*.cognitiveservices.azure.com",
    "bedrock-runtime.*.amazonaws.com",
    "api.cohere.com",
    "api.cohere.ai",
    "api.perplexity.ai",
    "api.fireworks.ai",
    "api.cerebras.ai",
    "api.voyageai.com",
];

/// Built-in gateway routes: `/gw/<name>/<path>` -> `<base>/<path>`.
pub const ROUTES: &[(&str, &str)] = &[
    ("openai", "https://api.openai.com"),
    ("anthropic", "https://api.anthropic.com"),
    ("openrouter", "https://openrouter.ai/api"),
    ("gemini", "https://generativelanguage.googleapis.com"),
    ("google", "https://generativelanguage.googleapis.com"),
    ("mistral", "https://api.mistral.ai"),
    ("groq", "https://api.groq.com/openai"),
    ("together", "https://api.together.xyz"),
    ("deepseek", "https://api.deepseek.com"),
    ("xai", "https://api.x.ai"),
    ("cohere", "https://api.cohere.com"),
    ("perplexity", "https://api.perplexity.ai"),
    ("fireworks", "https://api.fireworks.ai/inference"),
    ("cerebras", "https://api.cerebras.ai"),
];

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

/// Decides whether a host is intercepted.
#[derive(Clone, Debug)]
pub struct HostMatcher {
    include: Vec<String>,
    bypass: Vec<String>,
}

impl HostMatcher {
    /// Built-ins plus `extra`, minus `bypass`.
    pub fn new(extra: &[String], bypass: &[String]) -> Self {
        let mut include: Vec<String> = AI_HOSTS.iter().map(|s| s.to_string()).collect();
        include.extend(extra.iter().cloned());
        Self {
            include,
            bypass: bypass.to_vec(),
        }
    }

    /// True if traffic to `host` should be translated.
    pub fn intercept(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.');
        !self.bypass.iter().any(|p| glob_match(p, host))
            && self.include.iter().any(|p| glob_match(p, host))
    }

    /// All intercept patterns.
    pub fn patterns(&self) -> &[String] {
        &self.include
    }
}

/// Built-in routes merged with config overrides.
pub fn routes(overrides: &BTreeMap<String, String>) -> BTreeMap<String, String> {
    let mut r: BTreeMap<String, String> = ROUTES
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    for (k, v) in overrides {
        r.insert(k.to_ascii_lowercase(), v.trim_end_matches('/').to_string());
    }
    r
}

/// A PAC file that sends only intercepted hosts through Shadow.
pub fn pac_file(matcher: &HostMatcher, proxy: &str) -> String {
    let conds: Vec<String> = matcher
        .patterns()
        .iter()
        .map(|p| format!("shExpMatch(host, \"{p}\")"))
        .collect();
    format!(
        "function FindProxyForURL(url, host) {{\n  if ({}) {{\n    return \"PROXY {proxy}\";\n  }}\n  return \"DIRECT\";\n}}\n",
        conds.join(" ||\n      ")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn globbing() {
        let m = HostMatcher::new(&["llm.internal.corp".into()], &["api.cohere.com".into()]);
        assert!(m.intercept("api.openai.com"));
        assert!(m.intercept("myres.openai.azure.com"));
        assert!(m.intercept("bedrock-runtime.eu-west-1.amazonaws.com"));
        assert!(m.intercept("llm.internal.corp"));
        assert!(!m.intercept("api.cohere.com"));
        assert!(!m.intercept("github.com"));
        assert!(!m.intercept("openai.azure.com.evil.com"));
    }
}
