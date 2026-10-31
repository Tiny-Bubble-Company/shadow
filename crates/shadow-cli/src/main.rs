//! `shadow` — endpoint privacy layer for AI clouds.

mod activity;
mod service;
mod top;
mod ui;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use clap::{Args, Parser, Subcommand};
use shadow_core::config::default_data_dir;
use shadow_core::{Shadow, ShadowConfig};
use shadow_proxy::{CertAuthority, Proxy};

#[derive(Parser)]
#[command(
    name = "shadow",
    version,
    about = "Endpoint privacy layer for AI clouds: masks what leaves, restores what returns."
)]
struct Cli {
    /// Path to shadow.toml (default: <data dir>/shadow.toml).
    #[arg(long, global = true, env = "SHADOW_CONFIG")]
    config: Option<PathBuf>,
    /// Data directory (default: $SHADOW_HOME or the platform data dir).
    #[arg(long, global = true)]
    data_dir: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create the persona, vault key and local CA.
    Init {
        /// Fake region (ISO code, e.g. CN, JP, BR). Default: far from this machine.
        #[arg(long)]
        region: Option<String>,
        /// Organisation name(s) to protect; the first maps to the persona company.
        #[arg(long = "org")]
        orgs: Vec<String>,
        /// Discard the existing persona and vault and start over.
        #[arg(long)]
        force: bool,
    },
    /// Run the proxy in the foreground (forward proxy + /gw gateway).
    Run {
        /// Listen address (default from config: 127.0.0.1:8877).
        #[arg(long)]
        listen: Option<String>,
    },
    /// Install the background service and trust the CA (dry run unless --apply).
    Install(InstallArgs),
    /// Remove the background service and CA trust (dry run unless --apply).
    Uninstall(InstallArgs),
    /// Show configuration, persona, vault and proxy health.
    Status {
        /// No output; exit code 0 only if the proxy answers.
        #[arg(long)]
        quiet: bool,
    },
    /// Show the persona (fake identity) of this machine.
    Identity {
        #[arg(long)]
        json: bool,
    },
    /// Mask stdin to stdout.
    Mask {
        /// Treat input as JSON and mask string values only.
        #[arg(long)]
        json: bool,
    },
    /// Unmask stdin to stdout.
    Unmask {
        #[arg(long)]
        json: bool,
    },
    /// Report sensitive values in a file without changing anything.
    Scan {
        file: PathBuf,
        /// Print full values instead of redacted previews.
        #[arg(long)]
        reveal: bool,
        #[arg(long)]
        json: bool,
    },
    /// Inspect or edit the vault.
    #[command(subcommand)]
    Vault(VaultCmd),
    /// Manage protected dictionary terms.
    #[command(subcommand)]
    Terms(TermsCmd),
    /// List intercepted AI hosts and gateway routes.
    Hosts,
    /// Live terminal dashboard: traffic, protected data, destinations, apps, incidents.
    Top {
        /// Show synthetic demo data instead of the running proxy.
        #[arg(long)]
        demo: bool,
    },
    /// Print an activity summary (for servers and scripts).
    Activity {
        /// 1h, 24h, 7d, 30d or today.
        #[arg(long, short, default_value = "24h")]
        window: String,
        /// Print the summary as JSON.
        #[arg(long)]
        json: bool,
        /// Summarise synthetic demo data.
        #[arg(long)]
        demo: bool,
    },
    /// Open the web dashboard in your browser.
    Dashboard {
        /// Open it with synthetic demo data.
        #[arg(long)]
        demo: bool,
        /// Print the URL instead of opening a browser.
        #[arg(long)]
        print: bool,
    },
}

#[derive(Args)]
struct InstallArgs {
    /// Actually perform the steps (default: print them).
    #[arg(long)]
    apply: bool,
    #[arg(long)]
    listen: Option<String>,
}

#[derive(Subcommand)]
enum VaultCmd {
    /// Entry counts by kind.
    Stats,
    /// Print every real<->fake pair as JSON (contains real values!).
    Export {
        #[arg(long = "i-understand")]
        i_understand: bool,
    },
    /// Remove the mapping for a real or fake value.
    Forget { value: String },
}

#[derive(Subcommand)]
enum TermsCmd {
    /// Protect one or more terms (prefix with person:, org:, hostname: ... to set the kind).
    Add { terms: Vec<String> },
    /// List protected terms.
    List,
}

fn main() {
    let cli = Cli::parse();
    let level = if matches!(cli.cmd, Cmd::Run { .. }) {
        "info"
    } else {
        "warn"
    };
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| level.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
        .with_target(false)
        .init();
    if let Err(e) = real_main(cli) {
        eprintln!("{} {e:#}", ui::red("error:"));
        std::process::exit(1);
    }
}

fn load_config(cli: &Cli) -> anyhow::Result<ShadowConfig> {
    let dir = cli.data_dir.clone().unwrap_or_else(default_data_dir);
    let path = cli
        .config
        .clone()
        .unwrap_or_else(|| dir.join("shadow.toml"));
    let mut cfg = if path.exists() {
        ShadowConfig::load(&path)?
    } else if cli.config.is_some() {
        anyhow::bail!("config file {} not found", path.display());
    } else {
        ShadowConfig::from_dir(&dir)
    };
    if let Some(d) = &cli.data_dir {
        cfg.data_dir = d.clone();
    }
    Ok(cfg)
}

fn open(cli: &Cli) -> anyhow::Result<Shadow> {
    let cfg = load_config(cli)?;
    Shadow::open(cfg)
}

fn read_stdin() -> anyhow::Result<String> {
    let mut s = String::new();
    std::io::stdin().read_to_string(&mut s)?;
    Ok(s)
}

fn real_main(cli: Cli) -> anyhow::Result<()> {
    match &cli.cmd {
        Cmd::Init {
            region,
            orgs,
            force,
        } => init(&cli, region.clone(), orgs.clone(), *force),
        Cmd::Run { listen } => run(&cli, listen.clone()),
        Cmd::Install(a) => install(&cli, a, true),
        Cmd::Uninstall(a) => install(&cli, a, false),
        Cmd::Status { quiet } => status(&cli, *quiet),
        Cmd::Identity { json } => identity(&open(&cli)?, *json),
        Cmd::Mask { json } => {
            let s = open(&cli)?;
            let input = read_stdin()?;
            let out = if *json {
                let mut v: serde_json::Value =
                    serde_json::from_str(&input).context("stdin is not JSON")?;
                s.mask_json(&mut v);
                serde_json::to_string_pretty(&v)?
            } else {
                s.mask(&input).text
            };
            std::io::stdout().write_all(out.as_bytes())?;
            Ok(())
        }
        Cmd::Unmask { json } => {
            let s = open(&cli)?;
            let input = read_stdin()?;
            let out = if *json {
                let mut v: serde_json::Value =
                    serde_json::from_str(&input).context("stdin is not JSON")?;
                s.unmask_json(&mut v);
                serde_json::to_string_pretty(&v)?
            } else {
                s.unmask(&input)
            };
            std::io::stdout().write_all(out.as_bytes())?;
            Ok(())
        }
        Cmd::Scan { file, reveal, json } => scan(&open(&cli)?, file, *reveal, *json),
        Cmd::Vault(v) => vault(&open(&cli)?, v),
        Cmd::Terms(t) => terms(&open(&cli)?, t),
        Cmd::Hosts => hosts(&cli),
        Cmd::Top { demo } => {
            let source = if *demo {
                activity::Source::demo()
            } else {
                activity::Source::live(&load_config(&cli)?)
            };
            top::run(source)
        }
        Cmd::Activity { window, json, demo } => {
            activity::run(&load_config(&cli)?, window, *json, *demo)
        }
        Cmd::Dashboard { demo, print } => dashboard(&cli, *demo, *print),
    }
}

fn init(cli: &Cli, region: Option<String>, orgs: Vec<String>, force: bool) -> anyhow::Result<()> {
    let mut cfg = load_config(cli)?;
    shadow_core::vault::create_private_dir(&cfg.data_dir)?;
    let toml_path = cli
        .config
        .clone()
        .unwrap_or_else(|| cfg.data_dir.join("shadow.toml"));
    if force {
        for f in [cfg.persona_path(), cfg.vault_path()] {
            if f.exists() {
                std::fs::remove_file(&f)?;
                ui::warn(&format!("removed {}", f.display()));
            }
        }
    }
    if let Some(r) = &region {
        anyhow::ensure!(
            r.eq_ignore_ascii_case("auto") || shadow_core::persona::region(r).is_some(),
            "unknown region {r:?}; known: {}",
            shadow_core::persona::REGIONS
                .iter()
                .map(|r| r.code)
                .collect::<Vec<_>>()
                .join(", ")
        );
        if cfg.persona_path().exists() {
            ui::warn("a persona already exists; --region is ignored (use --force to regenerate)");
        }
        cfg.persona.region = Some(r.clone());
    }
    if !toml_path.exists() {
        let orgs_toml = orgs
            .iter()
            .map(|o| format!("{o:?}"))
            .collect::<Vec<_>>()
            .join(", ");
        let body = format!(
            "# Shadow configuration. See shadow.example.toml for every option.\n\norganization = [{orgs_toml}]\nterms = []\ninternal_domains = []\n\n[persona]\nregion = {:?}\n\n[proxy]\nlisten = {:?}\n",
            cfg.persona.region.clone().unwrap_or_else(|| "auto".into()),
            cfg.proxy.listen
        );
        std::fs::write(&toml_path, body)?;
        cfg.organization = orgs.clone();
        ui::ok(&format!("wrote {}", toml_path.display()));
    }
    let shadow = Shadow::open(cfg.clone())?;
    if !orgs.is_empty() {
        shadow.add_terms(&orgs);
    }
    let ca = CertAuthority::load_or_create(&cfg.data_dir.join("ca"))?;
    let bundle = cfg.data_dir.join("ca/bundle.pem");
    if !service::write_bundle(&ca.pem(), &bundle)? {
        ui::warn("could not find system root certificates; bundle.pem contains only the Shadow CA");
    }
    println!();
    identity(&shadow, false)?;
    println!();
    ui::header("Files");
    ui::kv("data dir", &cfg.data_dir.display().to_string());
    ui::kv(
        "vault",
        &format!("{} (encrypted)", cfg.vault_path().display()),
    );
    ui::kv("key", &format!("{} (0600)", cfg.key_path().display()));
    ui::kv("CA", &ca.pem_path().display().to_string());
    println!();
    ui::header("Next");
    println!(
        "  {}   foreground proxy on {}",
        ui::bold("shadow run"),
        cfg.proxy.listen
    );
    println!(
        "  {}       see what installing as a service would do",
        ui::bold("shadow install")
    );
    println!(
        "  {}  try it: masks stdin",
        ui::bold("echo 'mail jane@acme.com' | shadow mask")
    );
    Ok(())
}

fn run(cli: &Cli, listen: Option<String>) -> anyhow::Result<()> {
    let shadow = open(cli)?;
    let addr_s = listen.unwrap_or_else(|| shadow.config().proxy.listen.clone());
    let addr: std::net::SocketAddr = addr_s
        .parse()
        .with_context(|| format!("bad listen address {addr_s}"))?;
    if !addr.ip().is_loopback() {
        ui::warn(&format!(
            "listening on {addr}: anyone who can reach this port can use your proxy"
        ));
    }
    let proxy = Proxy::new(shadow.clone())?;
    let p = shadow.persona();
    ui::header(&format!(
        "Shadow {} listening on http://{addr}",
        env!("CARGO_PKG_VERSION")
    ));
    ui::kv(
        "persona",
        &format!("{} · {}, {} · {}", p.company, p.city, p.country, p.hostname),
    );
    ui::kv(
        "gateway",
        &format!("http://{addr}/gw/<provider>/...  e.g. /gw/openai/v1/chat/completions"),
    );
    ui::kv(
        "proxy",
        &format!(
            "HTTPS_PROXY=http://{addr}  (trust {})",
            proxy.ca().pem_path().display()
        ),
    );
    if let Some(up) = &shadow.config().proxy.upstream_proxy {
        ui::kv("egress via", up);
    }
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async move {
        let server = Arc::new(proxy).run(addr);
        tokio::select! {
            r = server => r,
            _ = tokio::signal::ctrl_c() => { eprintln!("\nshutting down"); Ok(()) }
        }
    })
}

fn svc_ctx(cli: &Cli, listen: Option<String>) -> anyhow::Result<(ShadowConfig, service::Ctx)> {
    let cfg = load_config(cli)?;
    let ca_dir = cfg.data_dir.join("ca");
    let ctx = service::Ctx {
        exe: std::env::current_exe()?,
        data_dir: cfg.data_dir.clone(),
        listen: listen.unwrap_or_else(|| cfg.proxy.listen.clone()),
        ca_pem: ca_dir.join("ca.pem"),
        bundle: ca_dir.join("bundle.pem"),
        tray: None,
    };
    let ctx = service::Ctx {
        tray: service::find_tray(&ctx.exe),
        ..ctx
    };
    Ok((cfg, ctx))
}

fn install(cli: &Cli, a: &InstallArgs, installing: bool) -> anyhow::Result<()> {
    let (cfg, ctx) = svc_ctx(cli, a.listen.clone())?;
    if installing {
        // Make sure persona, key and CA exist before the service starts.
        Shadow::open(cfg.clone())?;
        let ca = CertAuthority::load_or_create(&cfg.data_dir.join("ca"))?;
        if !ctx.bundle.exists() {
            service::write_bundle(&ca.pem(), &ctx.bundle)?;
        }
    }
    let plan = if installing {
        service::install_plan(&ctx)
    } else {
        service::uninstall_plan(&ctx)
    };
    ui::header(if a.apply {
        "Applying"
    } else {
        "Plan (dry run — re-run with --apply to perform)"
    });
    if plan.is_empty() {
        ui::warn(
            "no service integration for this OS; run `shadow run` under your supervisor of choice",
        );
    }
    service::execute(&plan, a.apply)?;
    if installing && ctx.tray.is_none() && !cfg!(target_os = "linux") {
        println!();
        ui::warn(
            "the menu-bar app (shadow-tray) is not installed, so there is no menu-bar icon. Install it with:",
        );
        println!(
            "    cargo install --locked --git https://github.com/Tiny-Bubble-Company/shadow shadow-tray --root ~/.local"
        );
        println!("  {}", ui::dim("then run `shadow install --apply` again."));
    }
    if installing {
        println!();
        ui::header("Point your tools at Shadow");
        println!(
            "  {}",
            ui::dim("Gateway mode (no CA needed): use a base URL such as")
        );
        println!("    OPENAI_BASE_URL=http://{}/gw/openai/v1", ctx.listen);
        println!("    ANTHROPIC_BASE_URL=http://{}/gw/anthropic", ctx.listen);
        println!(
            "  {}",
            ui::dim("Proxy mode (all tools, needs the CA): add to your shell profile")
        );
        for l in service::env_snippet(&ctx).lines() {
            println!("    {l}");
        }
        println!(
            "  {}",
            ui::dim("System-wide (macOS, only AI hosts go through Shadow):")
        );
        println!(
            "    networksetup -setautoproxyurl \"Wi-Fi\" http://{}/proxy.pac",
            ctx.listen
        );
        if a.apply {
            let env_path = cfg
                .data_dir
                .join(if cfg!(windows) { "env.ps1" } else { "env.sh" });
            std::fs::write(&env_path, service::env_snippet(&ctx) + "\n")?;
            ui::ok(&format!(
                "wrote {} (source it from your shell profile)",
                env_path.display()
            ));
        }
    }
    Ok(())
}

async fn probe(listen: &str) -> Option<String> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let fut = async {
        let mut s = tokio::net::TcpStream::connect(listen).await.ok()?;
        s.write_all(b"GET /shadow/status HTTP/1.1\r\nHost: shadow\r\nConnection: close\r\n\r\n")
            .await
            .ok()?;
        let mut buf = Vec::new();
        s.read_to_end(&mut buf).await.ok()?;
        let text = String::from_utf8_lossy(&buf).to_string();
        text.starts_with("HTTP/1.1 200").then_some(text)
    };
    tokio::time::timeout(std::time::Duration::from_millis(800), fut)
        .await
        .ok()
        .flatten()
}

fn status(cli: &Cli, quiet: bool) -> anyhow::Result<()> {
    let cfg = load_config(cli)?;
    let listen = cfg.proxy.listen.clone();
    let up = tokio::runtime::Runtime::new()?
        .block_on(probe(&listen))
        .is_some();
    if quiet {
        std::process::exit(if up { 0 } else { 1 });
    }
    let shadow = Shadow::open(cfg.clone())?;
    let p = shadow.persona();
    let st = shadow.stats();
    ui::header("Shadow status");
    ui::kv(
        "proxy",
        &if up {
            ui::green(&format!("running on http://{listen}"))
        } else {
            ui::yellow(&format!("not reachable on {listen}"))
        },
    );
    ui::kv("data dir", &cfg.data_dir.display().to_string());
    ui::kv(
        "persona",
        &format!("{} ({}, {})", p.company, p.city, p.region),
    );
    ui::kv(
        "vault",
        &format!("{} entries · created {}", st.entries, st.created_at),
    );
    let ca_path = cfg.data_dir.join("ca");
    if ca_path.join("ca.pem").exists() {
        let ca = CertAuthority::load_or_create(&ca_path)?;
        ui::kv(
            "CA",
            &format!(
                "{} · SHA256 {}…",
                ca.pem_path().display(),
                &ca.fingerprint()[..23]
            ),
        );
    } else {
        ui::kv("CA", &ui::yellow("not created (run `shadow init`)"));
    }
    ui::kv(
        "model detector",
        if cfg.model_detector.enabled {
            &cfg.model_detector.url
        } else {
            "off"
        },
    );
    ui::kv(
        "upstream proxy",
        cfg.proxy
            .upstream_proxy
            .as_deref()
            .unwrap_or("none (source IP is visible to providers)"),
    );
    Ok(())
}

fn identity(shadow: &Shadow, json: bool) -> anyhow::Result<()> {
    let p = shadow.persona();
    if json {
        println!("{}", serde_json::to_string_pretty(&p)?);
        return Ok(());
    }
    ui::header("Persona — what AI providers see");
    ui::kv(
        "company",
        &format!("{} ({})", ui::bold(&p.company), p.domain),
    );
    ui::kv("person", &format!("{} <{}>", p.full_name, p.email));
    ui::kv(
        "location",
        &format!("{}, {} · {} · {}", p.city, p.country, p.timezone, p.locale),
    );
    ui::kv("user id", &p.user_id);
    ui::kv("username", &p.username);
    ui::kv("hostname", &p.hostname);
    ui::kv("colleagues", &p.people.join(", "));
    Ok(())
}

fn redact(v: &str) -> String {
    let n = v.chars().count();
    if n <= 4 {
        return "*".repeat(n);
    }
    let head: String = v.chars().take(2).collect();
    let tail: String = v.chars().skip(n - 2).collect();
    format!("{head}{}{tail}", "*".repeat((n - 4).min(12)))
}

fn scan(shadow: &Shadow, file: &Path, reveal: bool, json: bool) -> anyhow::Result<()> {
    let text =
        std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let findings = shadow.scan(&text);
    let pos = |off: usize| {
        let before = &text[..off];
        let line = before.matches('\n').count() + 1;
        let col = before
            .rsplit('\n')
            .next()
            .map(|l| l.chars().count())
            .unwrap_or(0)
            + 1;
        (line, col)
    };
    if json {
        let items: Vec<_> = findings
            .iter()
            .map(|f| {
                let (l, c) = pos(f.start);
                let v = &text[f.start..f.end];
                serde_json::json!({"kind": f.kind.as_str(), "line": l, "column": c, "detector": f.detector,
                    "score": f.score, "value": if reveal { v.to_string() } else { redact(v) }})
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&items)?);
        return Ok(());
    }
    ui::header(&format!(
        "{} — {} finding(s)",
        file.display(),
        findings.len()
    ));
    let rows: Vec<Vec<String>> = findings
        .iter()
        .map(|f| {
            let (l, c) = pos(f.start);
            let v = &text[f.start..f.end];
            let v = if reveal {
                v.replace('\n', "⏎")
            } else {
                redact(v)
            };
            vec![
                f.kind.as_str().to_string(),
                format!("{l}:{c}"),
                v,
                f.detector.to_string(),
            ]
        })
        .collect();
    if !rows.is_empty() {
        ui::table(&["KIND", "AT", "VALUE", "DETECTOR"], &rows);
    }
    Ok(())
}

fn vault(shadow: &Shadow, cmd: &VaultCmd) -> anyhow::Result<()> {
    match cmd {
        VaultCmd::Stats => {
            let st = shadow.stats();
            ui::header(&format!(
                "Vault — {} entries (created {})",
                st.entries, st.created_at
            ));
            let rows: Vec<Vec<String>> = st
                .by_kind
                .iter()
                .map(|(k, n)| vec![k.clone(), n.to_string()])
                .collect();
            if !rows.is_empty() {
                ui::table(&["KIND", "ENTRIES"], &rows);
            }
        }
        VaultCmd::Export { i_understand } => {
            anyhow::ensure!(
                *i_understand,
                "export prints REAL values in clear text; re-run with --i-understand"
            );
            println!("{}", serde_json::to_string_pretty(&shadow.export())?);
        }
        VaultCmd::Forget { value } => {
            let n = shadow.forget(value)?;
            if n == 0 {
                ui::warn("no matching entry");
            } else {
                ui::ok(&format!(
                    "forgot {n} entr{}",
                    if n == 1 { "y" } else { "ies" }
                ));
            }
        }
    }
    Ok(())
}

fn terms(shadow: &Shadow, cmd: &TermsCmd) -> anyhow::Result<()> {
    match cmd {
        TermsCmd::Add { terms } => {
            anyhow::ensure!(!terms.is_empty(), "give at least one term");
            shadow.add_terms(terms);
            for t in terms {
                let m = shadow.mask(
                    t.split_once(':')
                        .filter(|(k, _)| shadow_core::Kind::parse(k).is_some())
                        .map(|(_, v)| v)
                        .unwrap_or(t),
                );
                ui::ok(&format!("{t}  →  {}", ui::accent(&m.text)));
            }
        }
        TermsCmd::List => {
            let rows: Vec<Vec<String>> = shadow
                .terms()
                .into_iter()
                .map(|(t, k)| vec![k.as_str().to_string(), t])
                .collect();
            ui::header(&format!("{} protected term(s)", rows.len()));
            if !rows.is_empty() {
                ui::table(&["KIND", "TERM"], &rows);
            }
        }
    }
    Ok(())
}

fn dashboard(cli: &Cli, demo: bool, print: bool) -> anyhow::Result<()> {
    let cfg = load_config(cli)?;
    let port = cfg
        .proxy
        .listen
        .rsplit_once(':')
        .map(|(_, p)| p.to_string())
        .unwrap_or_else(|| "8877".into());
    let url = format!(
        "http://127.0.0.1:{port}/shadow/ui{}",
        if demo { "?demo=1" } else { "" }
    );
    if print {
        println!("{url}");
        return Ok(());
    }
    if activity::http_get(&format!("127.0.0.1:{port}"), "/healthz").is_err() {
        ui::warn("Shadow is not running; start it with `shadow run` (the page retries on its own)");
    }
    let status = if cfg!(target_os = "macos") {
        std::process::Command::new("open").arg(&url).status()
    } else if cfg!(windows) {
        std::process::Command::new("cmd")
            .args(["/C", "start", "", &url])
            .status()
    } else {
        std::process::Command::new("xdg-open").arg(&url).status()
    };
    match status {
        Ok(s) if s.success() => ui::ok(&format!("opened {url}")),
        _ => println!("Open {url} in your browser."),
    }
    Ok(())
}

fn hosts(cli: &Cli) -> anyhow::Result<()> {
    let cfg = load_config(cli)?;
    let m = shadow_proxy::HostMatcher::new(&cfg.proxy.extra_hosts, &cfg.proxy.bypass_hosts);
    ui::header("Intercepted hosts (everything else is tunnelled untouched)");
    for h in m.patterns() {
        let mark = if m.intercept(&h.replace('*', "x")) {
            ui::green("●")
        } else {
            ui::dim("○ bypassed")
        };
        println!("  {mark} {h}");
    }
    println!();
    ui::header(&format!(
        "Gateway routes  http://{}/gw/<name>/...",
        cfg.proxy.listen
    ));
    let rows: Vec<Vec<String>> = shadow_proxy::hosts::routes(&cfg.proxy.routes)
        .into_iter()
        .map(|(k, v)| vec![k, v])
        .collect();
    ui::table(&["NAME", "UPSTREAM"], &rows);
    Ok(())
}
