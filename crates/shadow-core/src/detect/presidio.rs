//! Microsoft Presidio Analyzer (<https://microsoft.github.io/presidio/>) as an extra detector.
//!
//! Contract: `POST {url}` with
//! `{"text": "...", "language": "en", "score_threshold": 0.5, "entities": [...]}` returns
//! `[{"entity_type":"PERSON","start":0,"end":8,"score":0.85}, ...]` where offsets are
//! Unicode code points (Python string indices).
//!
//! Presidio only *finds* spans; Shadow still generates the fakes, owns the vault and keeps
//! the persona consistent. Unlike the model sidecar, the analyzer may run on another host
//! (e.g. a docker service `http://presidio:3000/analyze`), but only plain `http://` is
//! supported. The detector uses a short timeout and fails open: on any error it logs a
//! warning once and returns no findings.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::{Detector, Finding, Kind};
use crate::config::{PresidioConfig, ShadowConfig};

/// Upper bound on a response body, to protect against a misbehaving server.
const MAX_RESPONSE: usize = 16 * 1024 * 1024;

/// HTTP client for a Presidio Analyzer `/analyze` endpoint.
pub struct PresidioDetector {
    /// `host:port` used for DNS resolution (and the `Host` header).
    authority: String,
    path: String,
    language: String,
    min_score: f32,
    entities: Vec<String>,
    timeout: Duration,
    /// Resolved addresses, cached after the first successful lookup.
    addrs: Mutex<Option<Vec<SocketAddr>>>,
    warned: AtomicBool,
}

#[derive(Debug, Deserialize)]
struct Entity {
    entity_type: String,
    start: usize,
    end: usize,
    #[serde(default = "one")]
    score: f32,
}

fn one() -> f32 {
    1.0
}

/// Map a Presidio entity type to a Shadow [`Kind`].
///
/// `explicit` is true when the user listed the entity in `presidio.entities`; a few
/// entities that are noisy or not identifying on their own (`URL`, `NRP`) are only mapped
/// then. `DATE_TIME` and unknown entities are always skipped.
fn entity_kind(entity: &str, explicit: bool) -> Option<Kind> {
    Some(match entity {
        "PERSON" | "PER" => Kind::Person,
        "EMAIL_ADDRESS" | "EMAIL" => Kind::Email,
        "PHONE_NUMBER" => Kind::Phone,
        "LOCATION" | "ADDRESS" | "LOC" | "GPE" => Kind::Address,
        "IBAN_CODE" => Kind::Iban,
        "CREDIT_CARD" => Kind::CreditCard,
        "IP_ADDRESS" => Kind::Ip,
        "ORGANIZATION" | "ORG" => Kind::Org,
        "CRYPTO" => Kind::Secret,
        "US_SSN"
        | "US_ITIN"
        | "US_PASSPORT"
        | "US_DRIVER_LICENSE"
        | "US_BANK_NUMBER"
        | "US_MBI"
        | "UK_NHS"
        | "UK_NINO"
        | "UK_PASSPORT"
        | "ES_NIF"
        | "ES_NIE"
        | "IT_FISCAL_CODE"
        | "IT_DRIVER_LICENSE"
        | "IT_PASSPORT"
        | "IT_IDENTITY_CARD"
        | "IT_VAT_CODE"
        | "PL_PESEL"
        | "SG_NRIC_FIN"
        | "SG_UEN"
        | "AU_ABN"
        | "AU_ACN"
        | "AU_TFN"
        | "AU_MEDICARE"
        | "IN_PAN"
        | "IN_AADHAAR"
        | "IN_VEHICLE_REGISTRATION"
        | "IN_VOTER"
        | "IN_PASSPORT"
        | "FI_PERSONAL_IDENTITY_CODE"
        | "KR_RRN"
        | "TH_TNIN"
        | "MEDICAL_LICENSE" => Kind::NationalId,
        "URL" if explicit => Kind::Hostname,
        "NRP" if explicit => Kind::Org,
        _ => return None,
    })
}

/// The host part of a URL span `[start,end)` (byte offsets), so the Hostname faker sees
/// just the host rather than a full URL.
fn url_host(text: &str, start: usize, end: usize) -> Option<(usize, usize)> {
    let span = &text[start..end];
    let mut host_start = span.find("://").map(|i| i + 3).unwrap_or(0);
    if let Some(at) = span[host_start..].find('@') {
        let slash = span[host_start..].find('/').unwrap_or(usize::MAX);
        if at < slash {
            host_start += at + 1;
        }
    }
    let rest = &span[host_start..];
    let len = rest.find(['/', ':', '?', '#']).unwrap_or(rest.len());
    let host = &rest[..len];
    (host.contains('.') && !host.is_empty())
        .then_some((start + host_start, start + host_start + len))
}

impl PresidioDetector {
    /// Create from config. Fails if the URL is not `http://host[:port][/path]`.
    pub fn new(cfg: &PresidioConfig) -> anyhow::Result<Self> {
        let rest = cfg
            .url
            .trim()
            .strip_prefix("http://")
            .ok_or_else(|| anyhow::anyhow!("presidio.url must start with http://"))?;
        let (hostport, path) = match rest.find('/') {
            Some(i) => (&rest[..i], rest[i..].to_string()),
            None => (rest, "/analyze".to_string()),
        };
        anyhow::ensure!(
            !hostport.is_empty() && !hostport.contains('@'),
            "presidio.url has an invalid host"
        );
        // Append the default port unless one is given (`[v6]:p` or `host:p`).
        let has_port = match hostport.rfind(']') {
            Some(i) => hostport[i..].contains(':'),
            None => hostport.contains(':'),
        };
        let authority = if has_port {
            hostport.to_string()
        } else {
            format!("{hostport}:80")
        };
        let port_ok = authority
            .rsplit_once(':')
            .is_some_and(|(_, p)| p.parse::<u16>().is_ok_and(|p| p > 0));
        anyhow::ensure!(port_ok, "presidio.url has an invalid port");
        anyhow::ensure!(
            (0.0..=1.0).contains(&cfg.min_score),
            "presidio.min_score must be within 0.0..=1.0"
        );
        anyhow::ensure!(
            !cfg.language.trim().is_empty(),
            "presidio.language is empty"
        );
        Ok(Self {
            authority,
            path,
            language: cfg.language.trim().to_string(),
            min_score: cfg.min_score,
            entities: cfg
                .entities
                .iter()
                .map(|e| e.trim().to_ascii_uppercase())
                .filter(|e| !e.is_empty())
                .collect(),
            timeout: Duration::from_millis(cfg.timeout_ms.max(10)),
            addrs: Mutex::new(None),
            warned: AtomicBool::new(false),
        })
    }

    fn resolve(&self) -> anyhow::Result<Vec<SocketAddr>> {
        let mut cache = self.addrs.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(a) = cache.as_ref() {
            return Ok(a.clone());
        }
        let addrs: Vec<SocketAddr> = self.authority.to_socket_addrs()?.collect();
        anyhow::ensure!(!addrs.is_empty(), "cannot resolve {}", self.authority);
        *cache = Some(addrs.clone());
        Ok(addrs)
    }

    fn connect(&self, deadline: Instant) -> anyhow::Result<TcpStream> {
        let addrs = self.resolve()?;
        let mut last = None;
        for a in &addrs {
            let left = remaining(deadline)?;
            match TcpStream::connect_timeout(a, left) {
                Ok(s) => return Ok(s),
                Err(e) => last = Some(e),
            }
        }
        // Addresses may have moved (e.g. container restarted): re-resolve next time.
        *self.addrs.lock().unwrap_or_else(|p| p.into_inner()) = None;
        Err(last
            .map(anyhow::Error::from)
            .unwrap_or_else(|| anyhow::anyhow!("no address for {}", self.authority)))
    }

    fn request_body(&self, text: &str) -> serde_json::Result<Vec<u8>> {
        let mut req = serde_json::json!({
            "text": text,
            "language": self.language,
            "score_threshold": self.min_score,
        });
        if !self.entities.is_empty() {
            req["entities"] = serde_json::json!(self.entities);
        }
        serde_json::to_vec(&req)
    }

    fn call(&self, text: &str) -> anyhow::Result<Vec<Entity>> {
        let deadline = Instant::now() + self.timeout;
        let body = self.request_body(text)?;
        let mut s = self.connect(deadline)?;
        s.set_nodelay(true).ok();
        s.set_write_timeout(Some(remaining(deadline)?))?;
        let head = format!(
            "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nAccept: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.path,
            self.authority,
            body.len()
        );
        s.write_all(head.as_bytes())?;
        s.write_all(&body)?;
        s.flush()?;
        let raw = read_response(&mut s, deadline)?;
        Ok(serde_json::from_slice::<Vec<Entity>>(&raw)?)
    }
}

fn remaining(deadline: Instant) -> anyhow::Result<Duration> {
    let left = deadline.saturating_duration_since(Instant::now());
    anyhow::ensure!(!left.is_zero(), "presidio request timed out");
    Ok(left)
}

/// Read one HTTP/1.1 response and return its (de-chunked) body; errors on non-200.
fn read_response(s: &mut TcpStream, deadline: Instant) -> anyhow::Result<Vec<u8>> {
    let mut raw = Vec::new();
    let mut buf = [0u8; 8192];
    let mut read_more = |s: &mut TcpStream, raw: &mut Vec<u8>| -> anyhow::Result<bool> {
        s.set_read_timeout(Some(remaining(deadline)?))?;
        let n = s.read(&mut buf)?;
        raw.extend_from_slice(&buf[..n]);
        anyhow::ensure!(raw.len() <= MAX_RESPONSE, "presidio response too large");
        Ok(n > 0)
    };
    let split = loop {
        if let Some(i) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
            break i;
        }
        anyhow::ensure!(read_more(s, &mut raw)?, "malformed presidio response");
    };
    let head = String::from_utf8_lossy(&raw[..split]).to_ascii_lowercase();
    let status = head.lines().next().unwrap_or("");
    anyhow::ensure!(
        status.starts_with("http/1.") && status.split_whitespace().nth(1) == Some("200"),
        "presidio returned {}",
        String::from_utf8_lossy(&raw[..split])
            .lines()
            .next()
            .unwrap_or("")
    );
    let header = |name: &str| {
        head.lines().skip(1).find_map(|l| {
            let (k, v) = l.split_once(':')?;
            (k.trim() == name).then(|| v.trim().to_string())
        })
    };
    let chunked = header("transfer-encoding").is_some_and(|v| v.contains("chunked"));
    let length = header("content-length").and_then(|v| v.parse::<usize>().ok());
    let body_start = split + 4;
    if chunked {
        loop {
            if let Some(body) = dechunk(&raw[body_start..])? {
                return Ok(body);
            }
            anyhow::ensure!(read_more(s, &mut raw)?, "truncated chunked response");
        }
    }
    if let Some(n) = length {
        anyhow::ensure!(n <= MAX_RESPONSE, "presidio response too large");
        while raw.len() - body_start < n {
            anyhow::ensure!(read_more(s, &mut raw)?, "truncated presidio response");
        }
        return Ok(raw[body_start..body_start + n].to_vec());
    }
    while read_more(s, &mut raw)? {}
    Ok(raw[body_start..].to_vec())
}

/// Decode a chunked body. `Ok(None)` means more bytes are needed.
fn dechunk(mut b: &[u8]) -> anyhow::Result<Option<Vec<u8>>> {
    let mut out = vec![];
    loop {
        let Some(i) = b.windows(2).position(|w| w == b"\r\n") else {
            return Ok(None);
        };
        let size_line = String::from_utf8_lossy(&b[..i]);
        let size_hex = size_line.split(';').next().unwrap_or("").trim();
        let n = usize::from_str_radix(size_hex, 16)
            .map_err(|_| anyhow::anyhow!("bad chunk size {size_hex:?}"))?;
        if n == 0 {
            return Ok(Some(out));
        }
        if b.len() < i + 2 + n + 2 {
            return Ok(None);
        }
        out.extend_from_slice(&b[i + 2..i + 2 + n]);
        b = &b[i + 2 + n + 2..];
    }
}

impl Detector for PresidioDetector {
    fn name(&self) -> &'static str {
        "presidio"
    }

    fn detect(&self, text: &str) -> Vec<Finding> {
        if text.trim().is_empty() {
            return vec![];
        }
        let ents = match self.call(text) {
            Ok(e) => e,
            Err(e) => {
                if !self.warned.swap(true, Ordering::Relaxed) {
                    tracing::warn!("presidio analyzer unavailable, continuing without it: {e:#}");
                }
                return vec![];
            }
        };
        // Code-point offsets -> byte offsets.
        let mut idx: Vec<usize> = text.char_indices().map(|(i, _)| i).collect();
        idx.push(text.len());
        ents.into_iter()
            .filter(|e| {
                e.score.is_finite()
                    && e.score >= self.min_score
                    && e.start < e.end
                    && e.end < idx.len()
            })
            .filter_map(|e| {
                let ty = e.entity_type.to_ascii_uppercase();
                let explicit = self.entities.contains(&ty);
                let kind = entity_kind(&ty, explicit)?;
                let (mut start, mut end) = (idx[e.start], idx[e.end]);
                if ty == "URL" {
                    (start, end) = url_host(text, start, end)?;
                }
                // Presidio's NER spans sometimes include surrounding whitespace.
                let span = &text[start..end];
                start += span.len() - span.trim_start().len();
                end -= span.len() - span.trim_end().len();
                (start < end).then_some(Finding {
                    start,
                    end,
                    kind,
                    score: e.score.min(1.0),
                    detector: "presidio",
                })
            })
            .collect()
    }
}

/// Build the Presidio detector from `cfg.presidio`, or `None` when it is disabled or
/// misconfigured (a warning is logged in the latter case).
pub fn presidio_detector(cfg: &ShadowConfig) -> Option<Box<dyn Detector>> {
    if !cfg.presidio.enabled {
        return None;
    }
    match PresidioDetector::new(&cfg.presidio) {
        Ok(d) => Some(Box::new(d)),
        Err(e) => {
            tracing::warn!("presidio detector disabled: {e:#}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::sync::mpsc;

    /// Serve one request with a canned response; send the request body back over `tx`.
    fn serve_once(response: String) -> (u16, mpsc::Receiver<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut c, _) = l.accept().unwrap();
            let mut got = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                if let Some(i) = got.windows(4).position(|w| w == b"\r\n\r\n") {
                    let head = String::from_utf8_lossy(&got[..i]).to_ascii_lowercase();
                    let len: usize = head
                        .lines()
                        .find_map(|l| l.strip_prefix("content-length:"))
                        .map(|v| v.trim().parse().unwrap())
                        .unwrap_or(0);
                    if got.len() >= i + 4 + len {
                        tx.send(String::from_utf8_lossy(&got[i + 4..i + 4 + len]).into())
                            .ok();
                        break;
                    }
                }
                let n = c.read(&mut buf).unwrap();
                if n == 0 {
                    break;
                }
                got.extend_from_slice(&buf[..n]);
            }
            c.write_all(response.as_bytes()).unwrap();
        });
        (port, rx)
    }

    fn json_response(body: &str) -> String {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    }

    fn cfg(port: u16) -> PresidioConfig {
        PresidioConfig {
            enabled: true,
            url: format!("http://127.0.0.1:{port}/analyze"),
            timeout_ms: 2000,
            ..Default::default()
        }
    }

    /// Non-ASCII text before the entities, so code-point and byte offsets differ.
    const TEXT: &str = "Grüße — Jürgen Kos, mail jurgen@acme.io, 1990, Catholic, ssn 123-45-6789";

    fn cp(text: &str, needle: &str) -> (usize, usize) {
        let b = text.find(needle).unwrap();
        let s = text[..b].chars().count();
        (s, s + needle.chars().count())
    }

    fn canned() -> String {
        let ent = |ty: &str, needle: &str, score: f32| {
            let (s, e) = cp(TEXT, needle);
            format!(
                r#"{{"entity_type":"{ty}","start":{s},"end":{e},"score":{score},"analysis_explanation":null,"recognition_metadata":{{"recognizer_name":"X"}}}}"#
            )
        };
        format!(
            "[{}]",
            [
                ent("PERSON", "Jürgen Kos", 0.85),
                ent("EMAIL_ADDRESS", "jurgen@acme.io", 1.0),
                ent("URL", "acme.io", 0.5),
                ent("DATE_TIME", "1990", 0.85),
                ent("NRP", "Catholic", 0.85),
                ent("US_SSN", "123-45-6789", 0.3),
                ent("SOMETHING_NEW", "ssn", 0.9),
            ]
            .join(",")
        )
    }

    #[test]
    fn maps_entities_and_codepoints() {
        let (port, rx) = serve_once(json_response(&canned()));
        let d = PresidioDetector::new(&cfg(port)).unwrap();
        let f = d.detect(TEXT);
        let got: Vec<(&str, Kind)> = f.iter().map(|f| (&TEXT[f.start..f.end], f.kind)).collect();
        // URL / NRP skipped (not explicitly requested), DATE_TIME and unknowns skipped,
        // the SSN is below min_score 0.5.
        assert_eq!(
            got,
            vec![
                ("Jürgen Kos", Kind::Person),
                ("jurgen@acme.io", Kind::Email)
            ]
        );
        assert!(f.iter().all(|f| f.detector == "presidio"));
        let req: serde_json::Value = serde_json::from_str(&rx.recv().unwrap()).unwrap();
        assert_eq!(req["text"], TEXT);
        assert_eq!(req["language"], "en");
        assert!((req["score_threshold"].as_f64().unwrap() - 0.5).abs() < 1e-6);
        assert!(req.get("entities").is_none());
    }

    #[test]
    fn explicit_entities_and_min_score() {
        let (port, rx) = serve_once(json_response(&canned()));
        let d = PresidioDetector::new(&PresidioConfig {
            min_score: 0.2,
            entities: vec!["person".into(), "URL".into(), "NRP".into(), "US_SSN".into()],
            ..cfg(port)
        })
        .unwrap();
        let f = d.detect(TEXT);
        let got: Vec<(&str, Kind)> = f.iter().map(|f| (&TEXT[f.start..f.end], f.kind)).collect();
        assert_eq!(
            got,
            vec![
                ("Jürgen Kos", Kind::Person),
                ("jurgen@acme.io", Kind::Email),
                ("acme.io", Kind::Hostname),
                ("Catholic", Kind::Org),
                ("123-45-6789", Kind::NationalId),
            ]
        );
        let req: serde_json::Value = serde_json::from_str(&rx.recv().unwrap()).unwrap();
        assert_eq!(
            req["entities"],
            serde_json::json!(["PERSON", "URL", "NRP", "US_SSN"])
        );
    }

    #[test]
    fn chunked_response_and_bad_offsets() {
        let text = "日本 Alice Smith";
        let body = r#"[{"entity_type":"PERSON","start":3,"end":14,"score":0.9},{"entity_type":"PERSON","start":5,"end":99,"score":0.9},{"entity_type":"PERSON","start":4,"end":4,"score":0.9}]"#;
        let (a, b) = body.split_at(20);
        let resp = format!(
            "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{a}\r\n{:x}\r\n{b}\r\n0\r\n\r\n",
            a.len(),
            b.len()
        );
        let (port, _rx) = serve_once(resp);
        let d = PresidioDetector::new(&cfg(port)).unwrap();
        let f = d.detect(text);
        assert_eq!(f.len(), 1);
        assert_eq!(&text[f[0].start..f[0].end], "Alice Smith");
    }

    #[test]
    fn fails_open() {
        // A port that was just released: nothing is listening.
        let port = TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let d = PresidioDetector::new(&cfg(port)).unwrap();
        assert!(d.detect("Alice Smith").is_empty());
        assert!(d.warned.load(Ordering::Relaxed));
        assert!(d.detect("Alice Smith").is_empty());

        // Server error status.
        let (port, _rx) =
            serve_once("HTTP/1.1 500 INTERNAL SERVER ERROR\r\nContent-Length: 2\r\n\r\n{}".into());
        assert!(PresidioDetector::new(&cfg(port))
            .unwrap()
            .detect("Alice Smith")
            .is_empty());

        // Garbage body.
        let (port, _rx) = serve_once(json_response("not json"));
        assert!(PresidioDetector::new(&cfg(port))
            .unwrap()
            .detect("Alice Smith")
            .is_empty());
    }

    #[test]
    fn times_out() {
        // Accepts but never answers.
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let hold = std::thread::spawn(move || {
            let (c, _) = l.accept().unwrap();
            std::thread::sleep(Duration::from_millis(600));
            drop(c);
        });
        let d = PresidioDetector::new(&PresidioConfig {
            timeout_ms: 150,
            ..cfg(port)
        })
        .unwrap();
        let t = Instant::now();
        assert!(d.detect("Alice Smith").is_empty());
        assert!(t.elapsed() < Duration::from_millis(500));
        hold.join().unwrap();
    }

    #[test]
    fn config_validation_and_factory() {
        let ok = |url: &str| {
            PresidioDetector::new(&PresidioConfig {
                url: url.into(),
                ..Default::default()
            })
        };
        // Non-loopback hosts are allowed (docker service names); resolution is lazy.
        let d = ok("http://presidio:3000/analyze").unwrap();
        assert_eq!(d.authority, "presidio:3000");
        assert_eq!(d.path, "/analyze");
        let d = ok("http://10.0.0.5").unwrap();
        assert_eq!(d.authority, "10.0.0.5:80");
        assert_eq!(d.path, "/analyze");
        assert_eq!(
            ok("http://[::1]:5002/analyze").unwrap().authority,
            "[::1]:5002"
        );
        assert!(ok("https://presidio:3000/analyze").is_err());
        assert!(ok("http://presidio:notaport/analyze").is_err());
        assert!(ok("http:///analyze").is_err());

        let mut sc = ShadowConfig::from_dir("/tmp");
        assert!(presidio_detector(&sc).is_none());
        sc.presidio.enabled = true;
        assert_eq!(presidio_detector(&sc).unwrap().name(), "presidio");
        sc.presidio.url = "ftp://x".into();
        assert!(presidio_detector(&sc).is_none());
    }

    #[test]
    fn entity_mapping() {
        assert_eq!(entity_kind("IN_PAN", false), Some(Kind::NationalId));
        assert_eq!(entity_kind("CRYPTO", false), Some(Kind::Secret));
        assert_eq!(entity_kind("LOCATION", false), Some(Kind::Address));
        assert_eq!(entity_kind("IBAN_CODE", false), Some(Kind::Iban));
        assert_eq!(entity_kind("DATE_TIME", true), None);
        assert_eq!(entity_kind("URL", false), None);
        assert_eq!(entity_kind("NRP", false), None);
        assert_eq!(
            url_host("go https://u:p@Host.example.com:8443/x", 3, 38),
            Some((15, 31))
        );
    }
}
