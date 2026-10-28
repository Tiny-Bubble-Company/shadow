//! Facts about the local machine that are protected automatically: OS username,
//! hostname, home directory, git identity and local domain.

use std::path::PathBuf;

use serde::Serialize;

/// Discovered local identity facts (all optional).
#[derive(Clone, Debug, Default, Serialize)]
pub struct LocalFacts {
    pub username: Option<String>,
    /// Hostname without `.local`.
    pub hostname: Option<String>,
    pub home: Option<PathBuf>,
    pub git_name: Option<String>,
    pub git_email: Option<String>,
    /// DNS domain part of the FQDN, if any (e.g. `corp.acme.com`).
    pub local_domain: Option<String>,
}

const GENERIC_USERS: &[&str] = &[
    "root", "admin", "user", "dev", "ubuntu", "runner", "ec2-user", "debian", "pi", "app", "node",
    "guest",
];

impl LocalFacts {
    /// Discover facts from the environment. Never fails.
    pub fn discover() -> Self {
        let username = whoami::fallible::username()
            .ok()
            .filter(|u| u.len() >= 3 && !GENERIC_USERS.contains(&u.to_ascii_lowercase().as_str()));
        let fqdn = whoami::fallible::hostname()
            .ok()
            .filter(|h| h.len() >= 3 && h != "localhost");
        let (hostname, local_domain) = match fqdn {
            Some(h) => {
                let h = h
                    .trim_end_matches(".local")
                    .trim_end_matches(".lan")
                    .to_string();
                match h.split_once('.') {
                    Some((short, dom)) => (Some(short.to_string()), Some(dom.to_string())),
                    None => (Some(h), None),
                }
            }
            None => (None, None),
        };
        let home = dirs::home_dir();
        let (git_name, git_email) = home
            .as_ref()
            .map(|h| read_git_user(&h.join(".gitconfig")))
            .unwrap_or_default();
        Self {
            username,
            hostname,
            home,
            git_name,
            git_email,
            local_domain,
        }
    }
}

/// Minimal `[user]` section reader for a gitconfig file.
fn read_git_user(path: &std::path::Path) -> (Option<String>, Option<String>) {
    let Ok(raw) = std::fs::read_to_string(path) else {
        return (None, None);
    };
    let (mut in_user, mut name, mut email) = (false, None, None);
    for line in raw.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_user = l.eq_ignore_ascii_case("[user]");
            continue;
        }
        if !in_user {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            let v = v.trim().trim_matches('"').to_string();
            match k.trim().to_ascii_lowercase().as_str() {
                "name" if !v.is_empty() => name = Some(v),
                "email" if !v.is_empty() => email = Some(v),
                _ => {}
            }
        }
    }
    (name, email)
}

#[cfg(test)]
mod tests {
    #[test]
    fn parses_gitconfig() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join(".gitconfig");
        std::fs::write(
            &p,
            "[core]\n name = x\n[user]\n\tname = Jane Doe\n\temail = jane@acme.io\n",
        )
        .unwrap();
        let (n, e) = super::read_git_user(&p);
        assert_eq!(n.as_deref(), Some("Jane Doe"));
        assert_eq!(e.as_deref(), Some("jane@acme.io"));
    }
}
