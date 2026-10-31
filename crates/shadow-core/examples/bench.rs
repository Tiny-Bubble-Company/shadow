//! Benchmark driver: reads JSONL samples (`{"id": "...", "text": "..."}`) on stdin and writes one
//! JSON line per sample: detected spans (Unicode code-point offsets, like Python), the masked
//! text, whether unmasking restores the original exactly, and the time taken.
//!
//!     cargo run --release -p shadow-core --example bench -- [--terms "Acme GmbH,Falcon"] < samples.jsonl
//!
//! Used by `bench/` to compare Shadow with other tools on the same data.

use std::io::{BufRead, Write};
use std::time::Instant;

use serde_json::{json, Value};
use shadow_core::{Shadow, ShadowConfig};

fn char_offset(text: &str, byte: usize) -> usize {
    text[..byte].chars().count()
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let terms: Vec<String> = args
        .iter()
        .position(|a| a == "--terms")
        .and_then(|i| args.get(i + 1))
        .map(|t| {
            t.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let dir = tempfile::tempdir()?;
    let mut cfg = ShadowConfig::from_dir(dir.path()).with_terms(terms);
    // The benchmark measures the engine on the text itself, not this machine's identity.
    cfg.detectors.local_facts = false;
    if let Ok(url) = std::env::var("SHADOW_BENCH_PRESIDIO") {
        cfg.presidio.enabled = true;
        cfg.presidio.url = url;
    }
    if let Ok(url) = std::env::var("SHADOW_BENCH_MODEL") {
        cfg.model_detector.enabled = true;
        cfg.model_detector.url = url;
        cfg.model_detector.timeout_ms = 20_000;
    }
    let shadow = Shadow::open(cfg)?;
    let lines: Vec<String> = std::io::stdin().lock().lines().collect::<Result<_, _>>()?;
    // Measure a running service, not process start-up: compile the detectors' patterns first.
    // `scan` stores nothing, so this doesn't change any result.
    for line in lines.iter().filter(|l| !l.trim().is_empty()).take(5) {
        let v: Value = serde_json::from_str(line)?;
        shadow.scan(v["text"].as_str().unwrap_or_default());
    }
    let mut out = std::io::BufWriter::new(std::io::stdout());
    for line in &lines {
        if line.trim().is_empty() {
            continue;
        }
        let v: Value = serde_json::from_str(line)?;
        let text = v["text"].as_str().unwrap_or_default();
        let t0 = Instant::now();
        let findings = shadow.scan(text);
        let masked = shadow.mask(text).text;
        let micros = t0.elapsed().as_micros();
        let restored = shadow.unmask(&masked);
        let spans: Vec<Value> = findings
            .iter()
            .map(|f| json!({ "start": char_offset(text, f.start), "end": char_offset(text, f.end), "label": f.kind.as_str(), "detector": f.detector }))
            .collect();
        let mut row = json!({ "id": v["id"], "spans": spans, "masked": masked, "roundtrip": restored == text, "micros": micros });
        if restored != text {
            row["restored"] = restored.into();
        }
        writeln!(out, "{row}")?;
    }
    Ok(())
}
