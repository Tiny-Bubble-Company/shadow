//! Tiny terminal styling helpers (ANSI colours, honours `NO_COLOR` and non-TTY output).

use std::io::IsTerminal;
use std::sync::OnceLock;

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal())
}

fn paint(code: &str, s: &str) -> String {
    if enabled() {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
    }
}

pub fn bold(s: &str) -> String {
    paint("1", s)
}
pub fn dim(s: &str) -> String {
    paint("2", s)
}
pub fn green(s: &str) -> String {
    paint("32", s)
}
pub fn yellow(s: &str) -> String {
    paint("33", s)
}
pub fn red(s: &str) -> String {
    paint("31", s)
}
pub fn accent(s: &str) -> String {
    paint("38;2;94;106;210", s)
}

/// Section header.
pub fn header(title: &str) {
    println!("{} {}", accent("◆"), bold(title));
}

/// `key  value` line with an aligned, dimmed key.
pub fn kv(key: &str, value: &str) {
    println!("  {}  {}", dim(&format!("{key:<14}")), value);
}

pub fn ok(msg: &str) {
    println!("{} {msg}", green("✓"));
}
pub fn warn(msg: &str) {
    eprintln!("{} {msg}", yellow("!"));
}
pub fn step(msg: &str) {
    println!("{} {msg}", accent("→"));
}

/// Print a compact table; the header row is dimmed, columns padded to fit.
pub fn table(headers: &[&str], rows: &[Vec<String>]) {
    let mut w: Vec<usize> = headers.iter().map(|h| h.chars().count()).collect();
    for r in rows {
        for (i, c) in r.iter().enumerate() {
            if i < w.len() {
                w[i] = w[i].max(c.chars().count());
            }
        }
    }
    let line = |cells: Vec<String>| {
        cells
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{c:<width$}", width = w[i]))
            .collect::<Vec<_>>()
            .join("  ")
    };
    println!(
        "  {}",
        dim(&line(headers.iter().map(|s| s.to_string()).collect()))
    );
    for r in rows {
        println!("  {}", line(r.clone()));
    }
}
