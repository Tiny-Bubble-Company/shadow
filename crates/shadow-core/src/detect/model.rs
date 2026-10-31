//! Optional ML detector: asks a local NER sidecar (see `ml/`) for entities.
//!
//! Contract: `POST {url}` with `{"text": "..."}` returns
//! `{"entities":[{"start":0,"end":4,"label":"PERSON","score":0.93}]}` where offsets are
//! Unicode code points (Python string indices). The detector only talks to loopback
//! addresses, uses a short timeout and fails open (returns no findings) on error.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde::Deserialize;

use super::{Detector, Finding, Kind};
use crate::config::ModelDetectorConfig;

/// HTTP client for the NER sidecar.
pub struct ModelDetector {
    addr: SocketAddr,
    host: String,
    path: String,
    timeout: Duration,
    min_score: f32,
    warned: AtomicBool,
}

#[derive(Deserialize)]
struct Resp {
    entities: Vec<Entity>,
}

#[derive(Deserialize)]
struct Entity {
    start: usize,
    end: usize,
    label: String,
    #[serde(default = "one")]
    score: f32,
}

fn one() -> f32 {
    1.0
}

impl ModelDetector {
    /// Create from config. Fails if the URL is not `http://` on a loopback address.
    pub fn new(cfg: &ModelDetectorConfig) -> anyhow::Result<Self> {
        let rest = cfg
            .url
            .strip_prefix("http://")
            .ok_or_else(|| anyhow::anyhow!("model_detector.url must start with http://"))?;
        let (hostport, path) = rest
            .split_once('/')
            .map(|(h, p)| (h, format!("/{p}")))
            .unwrap_or((rest, "/".into()));
        let addr = hostport
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow::anyhow!("cannot resolve {hostport}"))?;
        anyhow::ensure!(
            addr.ip().is_loopback(),
            "model_detector.url must point at a loopback address"
        );
        Ok(Self {
            addr,
            host: hostport.to_string(),
            path,
            timeout: Duration::from_millis(cfg.timeout_ms.max(10)),
            min_score: cfg.min_score,
            warned: AtomicBool::new(false),
        })
    }

    fn call(&self, text: &str) -> anyhow::Result<Vec<Entity>> {
        let body = serde_json::to_vec(&serde_json::json!({ "text": text }))?;
        let mut s = TcpStream::connect_timeout(&self.addr, self.timeout)?;
        s.set_read_timeout(Some(self.timeout))?;
        s.set_write_timeout(Some(self.timeout))?;
        write!(
            s,
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.path,
            self.host,
            body.len()
        )?;
        s.write_all(&body)?;
        let mut raw = Vec::new();
        s.read_to_end(&mut raw)?;
        let split = raw
            .windows(4)
            .position(|w| w == b"\r\n\r\n")
            .ok_or_else(|| anyhow::anyhow!("malformed sidecar response"))?;
        let head = String::from_utf8_lossy(&raw[..split]);
        anyhow::ensure!(
            head.starts_with("HTTP/1.") && head.contains(" 200"),
            "sidecar returned {}",
            head.lines().next().unwrap_or("")
        );
        let mut body = &raw[split + 4..];
        let dechunked;
        if head
            .to_ascii_lowercase()
            .contains("transfer-encoding: chunked")
        {
            dechunked = dechunk(body);
            body = &dechunked;
        }
        Ok(serde_json::from_slice::<Resp>(body)?.entities)
    }
}

fn dechunk(mut b: &[u8]) -> Vec<u8> {
    let mut out = vec![];
    while let Some(i) = b.windows(2).position(|w| w == b"\r\n") {
        let n = usize::from_str_radix(String::from_utf8_lossy(&b[..i]).trim(), 16).unwrap_or(0);
        if n == 0 || b.len() < i + 2 + n {
            break;
        }
        out.extend_from_slice(&b[i + 2..i + 2 + n]);
        b = &b[(i + 4 + n).min(b.len())..];
    }
    out
}

/// The Shadow kind for a sidecar label; `None` for labels Shadow leaves to its own detectors
/// (URLs), plain dates, or labels it doesn't handle.
fn label_kind(label: &str) -> Option<Kind> {
    Some(match label.to_ascii_uppercase().as_str() {
        "PERSON" | "PER" => Kind::Person,
        "ORG" | "ORGANIZATION" => Kind::Org,
        "LOC" | "LOCATION" | "ADDRESS" | "GPE" => Kind::Address,
        "EMAIL" => Kind::Email,
        "PHONE" => Kind::Phone,
        "SECRET" | "PASSWORD" | "API_KEY" => Kind::Secret,
        "USERNAME" => Kind::Username,
        "IP" | "IP_ADDRESS" => Kind::Ip,
        "IBAN" => Kind::Iban,
        "CREDIT_CARD" => Kind::CreditCard,
        "NATIONAL_ID" | "SSN" | "ACCOUNT" | "ACCOUNT_NUMBER" => Kind::NationalId,
        // Only birth dates: other dates ("the meeting on 3 May") carry meaning, not identity.
        "DOB" | "DATE_OF_BIRTH" => Kind::Date,
        "PROJECT" | "TERM" | "MISC" => Kind::Term,
        _ => return None,
    })
}

impl Detector for ModelDetector {
    fn name(&self) -> &'static str {
        "model"
    }
    fn detect(&self, text: &str) -> Vec<Finding> {
        let ents = match self.call(text) {
            Ok(e) => e,
            Err(e) => {
                if !self.warned.swap(true, Ordering::Relaxed) {
                    tracing::warn!("model detector unavailable, continuing without it: {e:#}");
                }
                return vec![];
            }
        };
        // Map code-point offsets to byte offsets.
        let mut idx: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        idx.push(text.len());
        ents.into_iter()
            .filter(|e| e.score >= self.min_score && e.start < e.end && e.end < idx.len())
            .filter_map(|e| {
                Some(Finding {
                    start: idx[e.start],
                    end: idx[e.end],
                    kind: label_kind(&e.label)?,
                    score: e.score,
                    detector: "model",
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    #[test]
    fn talks_to_sidecar_and_maps_codepoints() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            while !got.ends_with(b"}") {
                let n = c.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
            }
            let body = r#"{"entities":[{"start":2,"end":12,"label":"PERSON","score":0.9}]}"#;
            write!(
                c,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let d = ModelDetector::new(&ModelDetectorConfig {
            enabled: true,
            url: format!("http://127.0.0.1:{port}/detect"),
            timeout_ms: 2000,
            min_score: 0.5,
        })
        .unwrap();
        let text = "é Jürgen Kos was here";
        let f = d.detect(text);
        assert_eq!(f.len(), 1);
        assert_eq!(&text[f[0].start..f[0].end], "Jürgen Kos");
        assert!(ModelDetector::new(&ModelDetectorConfig {
            url: "http://8.8.8.8/x".into(),
            ..Default::default()
        })
        .is_err());
    }
}
