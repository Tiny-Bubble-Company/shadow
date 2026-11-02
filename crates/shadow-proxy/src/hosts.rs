//! Which hosts are intercepted, and the gateway route table.

use std::collections::BTreeMap;

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
    ("huggingface", "https://router.huggingface.co"),
    ("deepinfra", "https://api.deepinfra.com/v1/openai"),
    ("nvidia", "https://integrate.api.nvidia.com"),
    ("sambanova", "https://api.sambanova.ai"),
    (
        "qwen",
        "https://dashscope-intl.aliyuncs.com/compatible-mode",
    ),
    ("moonshot", "https://api.moonshot.ai"),
];

pub use shadow_core::catalog::glob_match;

/// Decides whether a host is intercepted (decrypted and masked) or only observed.
#[derive(Clone, Debug)]
pub struct HostMatcher {
    include: Vec<String>,
    bypass: Vec<String>,
    observe: Vec<String>,
}

impl HostMatcher {
    /// The catalog's masked hosts plus `extra`, minus `bypass`.
    pub fn new(extra: &[String], bypass: &[String]) -> Self {
        let mut include: Vec<String> = shadow_core::catalog::masked_hosts()
            .map(str::to_string)
            .collect();
        include.extend(extra.iter().cloned());
        Self {
            include,
            bypass: bypass.to_vec(),
            observe: shadow_core::catalog::observed_hosts()
                .map(str::to_string)
                .collect(),
        }
    }

    /// A known AI service that is not intercepted: its traffic is tunnelled untouched and
    /// recorded as seen but not protected.
    pub fn observed(&self, host: &str) -> bool {
        let host = host.trim_end_matches('.');
        !self.intercept(host) && self.observe.iter().any(|p| glob_match(p, host))
    }

    /// Also intercept the supported AI chat apps (ChatGPT, Claude.ai, Gemini, ...).
    pub fn with_chat_apps(mut self) -> Self {
        self.include
            .extend(crate::apps::hosts().map(str::to_string));
        self
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

/// A PAC file that sends all traffic through Shadow, for the system proxy. Local and private
/// addresses go direct. If Shadow is not answering, clients fall back to a direct connection,
/// so stopping Shadow never cuts the machine off. Shadow decrypts only AI hosts; everything
/// else is tunnelled byte for byte and not recorded.
pub fn pac_all(proxy: &str) -> String {
    format!(
        r#"function FindProxyForURL(url, host) {{
  if (isPlainHostName(host) ||
      host == "localhost" || dnsDomainIs(host, ".localhost") || dnsDomainIs(host, ".local") ||
      shExpMatch(host, "127.*") || shExpMatch(host, "10.*") || shExpMatch(host, "192.168.*") ||
      shExpMatch(host, "169.254.*") || shExpMatch(host, "172.1[6-9].*") ||
      shExpMatch(host, "172.2[0-9].*") || shExpMatch(host, "172.3[0-1].*") ||
      host == "::1" || host == "[::1]") {{
    return "DIRECT";
  }}
  return "PROXY {proxy}; DIRECT";
}}
"#
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
        assert!(!m.intercept("chatgpt.com"));
        let m = m.with_chat_apps();
        assert!(m.intercept("chatgpt.com") && m.intercept("claude.ai"));
    }

    #[test]
    fn system_pac_falls_back_to_direct() {
        let pac = pac_all("127.0.0.1:8877");
        assert!(pac.contains("return \"PROXY 127.0.0.1:8877; DIRECT\";"));
        assert!(pac.contains("shExpMatch(host, \"192.168.*\")"));
    }
}
