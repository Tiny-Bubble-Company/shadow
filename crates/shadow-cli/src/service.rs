//! `shadow install` / `shadow uninstall`: OS service + CA trust, as an explicit plan.
//!
//! The plan is always printed. It is only executed with `--apply`.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::Context;

use crate::ui;

const PLIST: &str = include_str!("../../../packaging/macos/dev.shadow.proxy.plist");
const TRAY_PLIST: &str = include_str!("../../../packaging/macos/dev.shadow.tray.plist");
const UNIT: &str = include_str!("../../../packaging/linux/shadow.service");

/// One step of an install/uninstall plan.
pub enum Action {
    Write { path: PathBuf, contents: String },
    Remove(PathBuf),
    Run { argv: Vec<String>, allow_fail: bool },
}

fn run(argv: &[&str]) -> Action {
    Action::Run {
        argv: argv.iter().map(|s| s.to_string()).collect(),
        allow_fail: false,
    }
}
fn run_lenient(argv: &[&str]) -> Action {
    Action::Run {
        argv: argv.iter().map(|s| s.to_string()).collect(),
        allow_fail: true,
    }
}

/// Inputs for plan generation.
pub struct Ctx {
    pub exe: PathBuf,
    pub data_dir: PathBuf,
    pub listen: String,
    pub ca_pem: PathBuf,
    pub bundle: PathBuf,
    /// The menu-bar / tray app (`shadow-tray`), when it is installed.
    pub tray: Option<PathBuf>,
}

/// Find `shadow-tray` next to the `shadow` binary, else on PATH.
pub fn find_tray(exe: &Path) -> Option<PathBuf> {
    let name = if cfg!(windows) {
        "shadow-tray.exe"
    } else {
        "shadow-tray"
    };
    let sibling = exe.parent().map(|d| d.join(name));
    sibling.filter(|p| p.is_file()).or_else(|| {
        std::env::var_os("PATH").and_then(|paths| {
            std::env::split_paths(&paths)
                .map(|d| d.join(name))
                .find(|p| p.is_file())
        })
    })
}

fn fill(tpl: &str, c: &Ctx) -> String {
    tpl.replace("{{EXE}}", &c.exe.display().to_string())
        .replace("{{TRAY}}", &c.tray.as_deref().map(s).unwrap_or_default())
        .replace("{{DATA_DIR}}", &c.data_dir.display().to_string())
        .replace("{{LISTEN}}", &c.listen)
}

fn home() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn s(p: &Path) -> String {
    p.display().to_string()
}

/// Steps to install the background service and trust the CA on this OS.
pub fn install_plan(c: &Ctx) -> Vec<Action> {
    let ca = s(&c.ca_pem);
    if cfg!(target_os = "macos") {
        let plist = home().join("Library/LaunchAgents/dev.shadow.proxy.plist");
        let uid = std::env::var("UID").ok().or_else(|| {
            Command::new("id")
                .arg("-u")
                .output()
                .ok()
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        });
        let domain = format!("gui/{}", uid.unwrap_or_else(|| "501".into()));
        let keychain = s(&home().join("Library/Keychains/login.keychain-db"));
        let mut v = vec![
            Action::Write {
                path: plist.clone(),
                contents: fill(PLIST, c),
            },
            run_lenient(&["launchctl", "bootout", &domain, &s(&plist)]),
            run(&["launchctl", "bootstrap", &domain, &s(&plist)]),
            run(&[
                "security",
                "add-trusted-cert",
                "-r",
                "trustRoot",
                "-k",
                &keychain,
                &ca,
            ]),
        ];
        if c.tray.is_some() {
            let tray = home().join("Library/LaunchAgents/dev.shadow.tray.plist");
            v.extend([
                Action::Write {
                    path: tray.clone(),
                    contents: fill(TRAY_PLIST, c),
                },
                run_lenient(&["launchctl", "bootout", &domain, &s(&tray)]),
                run(&["launchctl", "bootstrap", &domain, &s(&tray)]),
            ]);
        }
        v
    } else if cfg!(target_os = "linux") {
        let unit = home().join(".config/systemd/user/shadow.service");
        let debian = Path::new("/usr/local/share/ca-certificates").exists();
        let mut v = vec![
            Action::Write {
                path: unit,
                contents: fill(UNIT, c),
            },
            run(&["systemctl", "--user", "daemon-reload"]),
            run(&["systemctl", "--user", "enable", "--now", "shadow.service"]),
        ];
        if debian {
            v.push(run(&[
                "sudo",
                "cp",
                &ca,
                "/usr/local/share/ca-certificates/shadow-local-ca.crt",
            ]));
            v.push(run(&["sudo", "update-ca-certificates"]));
        } else {
            v.push(run(&[
                "sudo",
                "cp",
                &ca,
                "/etc/pki/ca-trust/source/anchors/shadow-local-ca.pem",
            ]));
            v.push(run(&["sudo", "update-ca-trust"]));
        }
        if let Some(tray) = &c.tray {
            v.push(Action::Write {
                path: home().join(".config/autostart/shadow-tray.desktop"),
                contents: format!(
                    "[Desktop Entry]\nType=Application\nName=Shadow\nComment=Shadow privacy activity\nExec=\"{}\" --url http://{}\nX-GNOME-Autostart-enabled=true\n",
                    s(tray),
                    c.listen
                ),
            });
        }
        v
    } else if cfg!(windows) {
        let tr = format!("\"{}\" run --listen {}", s(&c.exe), c.listen);
        let mut v = vec![
            run(&[
                "schtasks", "/Create", "/TN", "Shadow", "/SC", "ONLOGON", "/RL", "LIMITED", "/F",
                "/TR", &tr,
            ]),
            run(&["schtasks", "/Run", "/TN", "Shadow"]),
            run(&["certutil", "-user", "-addstore", "Root", &ca]),
            run(&["setx", "SHADOW_HOME", &s(&c.data_dir)]),
        ];
        if let Some(tray) = &c.tray {
            let tr = format!("\"{}\" --url http://{}", s(tray), c.listen);
            v.extend([
                run(&[
                    "schtasks",
                    "/Create",
                    "/TN",
                    "Shadow Tray",
                    "/SC",
                    "ONLOGON",
                    "/RL",
                    "LIMITED",
                    "/F",
                    "/TR",
                    &tr,
                ]),
                run(&["schtasks", "/Run", "/TN", "Shadow Tray"]),
            ]);
        }
        v
    } else {
        vec![]
    }
}

/// Steps to remove the service and CA trust.
pub fn uninstall_plan(c: &Ctx) -> Vec<Action> {
    if cfg!(target_os = "macos") {
        let plist = home().join("Library/LaunchAgents/dev.shadow.proxy.plist");
        let uid = Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
        let domain = format!("gui/{}", uid.unwrap_or_else(|| "501".into()));
        let tray = home().join("Library/LaunchAgents/dev.shadow.tray.plist");
        vec![
            run_lenient(&["launchctl", "bootout", &domain, &s(&tray)]),
            Action::Remove(tray),
            run_lenient(&["launchctl", "bootout", &domain, &s(&plist)]),
            Action::Remove(plist),
            run_lenient(&["security", "delete-certificate", "-c", "Shadow Local CA"]),
        ]
    } else if cfg!(target_os = "linux") {
        vec![
            run_lenient(&["systemctl", "--user", "disable", "--now", "shadow.service"]),
            Action::Remove(home().join(".config/systemd/user/shadow.service")),
            Action::Remove(home().join(".config/autostart/shadow-tray.desktop")),
            run_lenient(&[
                "sudo",
                "rm",
                "-f",
                "/usr/local/share/ca-certificates/shadow-local-ca.crt",
                "/etc/pki/ca-trust/source/anchors/shadow-local-ca.pem",
            ]),
            run_lenient(&[
                "sudo",
                "sh",
                "-c",
                "update-ca-certificates --fresh 2>/dev/null || update-ca-trust",
            ]),
        ]
    } else if cfg!(windows) {
        let _ = c;
        vec![
            run_lenient(&["schtasks", "/End", "/TN", "Shadow"]),
            run_lenient(&["schtasks", "/Delete", "/TN", "Shadow", "/F"]),
            run_lenient(&["schtasks", "/End", "/TN", "Shadow Tray"]),
            run_lenient(&["schtasks", "/Delete", "/TN", "Shadow Tray", "/F"]),
            run_lenient(&["certutil", "-user", "-delstore", "Root", "Shadow Local CA"]),
        ]
    } else {
        vec![]
    }
}

/// Print a plan (and execute it when `apply`).
pub fn execute(plan: &[Action], apply: bool) -> anyhow::Result<()> {
    for a in plan {
        match a {
            Action::Write { path, contents } => {
                ui::step(&format!("write {}", path.display()));
                if apply {
                    if let Some(p) = path.parent() {
                        std::fs::create_dir_all(p)?;
                    }
                    std::fs::write(path, contents)
                        .with_context(|| format!("writing {}", path.display()))?;
                }
            }
            Action::Remove(path) => {
                ui::step(&format!("remove {}", path.display()));
                if apply && path.exists() {
                    std::fs::remove_file(path)?;
                }
            }
            Action::Run { argv, allow_fail } => {
                let shown = argv
                    .iter()
                    .map(|a| {
                        if a.contains(' ') {
                            format!("\"{a}\"")
                        } else {
                            a.clone()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                ui::step(&format!("run   {shown}"));
                if apply {
                    let mut cmd = Command::new(&argv[0]);
                    cmd.args(&argv[1..]);
                    if *allow_fail {
                        // Clean-up steps that may find nothing to do ("not loaded"): keep quiet.
                        cmd.stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null());
                    }
                    let status = cmd.status();
                    match status {
                        Ok(st) if st.success() => {}
                        Ok(_) if *allow_fail => {}
                        Ok(st) => anyhow::bail!("{shown} failed with {st}"),
                        Err(e) if *allow_fail => ui::warn(&format!("{}: {e} (ignored)", argv[0])),
                        Err(e) => return Err(e).with_context(|| format!("running {}", argv[0])),
                    }
                }
            }
        }
    }
    Ok(())
}

/// Shell snippet that points runtimes at Shadow.
pub fn env_snippet(c: &Ctx) -> String {
    let proxy = format!("http://{}", c.listen);
    if cfg!(windows) {
        format!(
            "$env:HTTPS_PROXY = \"{proxy}\"\n$env:NO_PROXY = \"localhost,127.0.0.1\"\n$env:NODE_EXTRA_CA_CERTS = \"{ca}\"\n$env:REQUESTS_CA_BUNDLE = \"{b}\"\n$env:SSL_CERT_FILE = \"{b}\"\n$env:CURL_CA_BUNDLE = \"{b}\"",
            ca = s(&c.ca_pem),
            b = s(&c.bundle)
        )
    } else {
        format!(
            "export HTTPS_PROXY={proxy}\nexport NO_PROXY=localhost,127.0.0.1\nexport NODE_EXTRA_CA_CERTS=\"{ca}\"\nexport REQUESTS_CA_BUNDLE=\"{b}\"\nexport SSL_CERT_FILE=\"{b}\"\nexport CURL_CA_BUNDLE=\"{b}\"",
            ca = s(&c.ca_pem),
            b = s(&c.bundle)
        )
    }
}

/// Write `<ca dir>/bundle.pem` = system roots + Shadow CA (for runtimes that replace,
/// rather than extend, their trust store via `SSL_CERT_FILE` / `REQUESTS_CA_BUNDLE`).
pub fn write_bundle(ca_pem: &str, bundle: &Path) -> anyhow::Result<bool> {
    let mut roots = String::new();
    for p in [
        "/etc/ssl/certs/ca-certificates.crt",
        "/etc/pki/tls/certs/ca-bundle.crt",
        "/etc/ssl/cert.pem",
        "/usr/local/etc/openssl/cert.pem",
    ] {
        if let Ok(s) = std::fs::read_to_string(p) {
            roots = s;
            break;
        }
    }
    if roots.is_empty() && cfg!(target_os = "macos") {
        if let Ok(o) = Command::new("security")
            .args([
                "find-certificate",
                "-a",
                "-p",
                "/System/Library/Keychains/SystemRootCertificates.keychain",
            ])
            .output()
        {
            roots = String::from_utf8_lossy(&o.stdout).to_string();
        }
    }
    let found = !roots.is_empty();
    std::fs::write(bundle, format!("{roots}\n# Shadow Local CA\n{ca_pem}"))?;
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(tray: Option<PathBuf>) -> Ctx {
        Ctx {
            exe: PathBuf::from("/opt/shadow/shadow"),
            data_dir: PathBuf::from("/tmp/shadow-data"),
            listen: "127.0.0.1:8877".into(),
            ca_pem: PathBuf::from("/tmp/shadow-data/ca/ca.pem"),
            bundle: PathBuf::from("/tmp/shadow-data/ca/bundle.pem"),
            tray,
        }
    }

    fn written(plan: &[Action]) -> Vec<(String, String)> {
        plan.iter()
            .filter_map(|a| match a {
                Action::Write { path, contents } => Some((s(path), contents.clone())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn the_menu_bar_app_starts_at_login_only_when_installed() {
        if cfg!(windows) {
            return;
        }
        let without = written(&install_plan(&ctx(None)));
        assert!(!without.iter().any(|(p, _)| p.contains("tray")));
        let with = written(&install_plan(&ctx(Some(PathBuf::from(
            "/opt/shadow/shadow-tray",
        )))));
        let (_, contents) = with
            .iter()
            .find(|(p, _)| p.contains("tray"))
            .expect("tray start-up entry");
        assert!(contents.contains("/opt/shadow/shadow-tray"), "{contents}");
        assert!(contents.contains("http://127.0.0.1:8877"), "{contents}");
    }
}
