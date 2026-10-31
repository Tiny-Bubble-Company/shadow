//! Activity recording on the request path, the `/shadow/activity*` API and the embedded
//! dashboard at `/shadow/ui`.

use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::Bytes;
use futures_util::StreamExt;
use http::{header, HeaderMap, Response, StatusCode};
use http_body_util::{BodyExt, StreamBody};
use hyper::body::Frame;
use shadow_core::activity::{
    self, client_from_user_agent, endpoint_for_path, provider_for_host, Action, ActivityLog, Event,
    Summary, Window,
};

use crate::forward::{error_response, full};
use crate::{Body, Proxy};

const INDEX_HTML: &str = include_str!("../ui/index.html");
const APP_JS: &str = include_str!("../ui/app.js");
const APP_CSS: &str = include_str!("../ui/app.css");
const LOGO_SVG: &str = include_str!("../ui/logo.svg");

/// Content-Security-Policy of the dashboard: nothing but this origin, no inline code.
pub const UI_CSP: &str = "default-src 'none'; script-src 'self'; style-src 'self'; \
img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'; \
frame-ancestors 'none'";

/// How a request reached the proxy.
#[derive(Clone, Copy)]
pub(crate) enum Via<'a> {
    /// Forward proxy; for intercepted TLS, the original host (the URL may point at an
    /// upstream override).
    Proxy(Option<&'a str>),
    /// `/gw/<route>/...`
    Gateway(&'a str),
}

/// An event being filled in while a request is in flight; recorded when dropped, so
/// aborted streams and early returns are still counted.
pub(crate) struct Pending {
    log: Arc<ActivityLog>,
    pub ev: Event,
    start: Instant,
}

impl Pending {
    pub fn new(
        log: Arc<ActivityLog>,
        url: &str,
        via: Via<'_>,
        method: &str,
        headers: &HeaderMap,
        bytes_in: usize,
    ) -> Self {
        let parsed = reqwest::Url::parse(url).ok();
        let (route, original) = match via {
            Via::Proxy(h) => (None, h),
            Via::Gateway(r) => (Some(r), None),
        };
        let host = original
            .or_else(|| parsed.as_ref().and_then(|u| u.host_str()))
            .unwrap_or("")
            .trim_matches(['[', ']'])
            .to_ascii_lowercase();
        let path = parsed.as_ref().map(|u| u.path()).unwrap_or("");
        let mut provider = provider_for_host(&host).to_string();
        if provider == "other" {
            if let Some(r) = route {
                provider = r
                    .chars()
                    .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
                    .take(32)
                    .collect::<String>()
                    .to_ascii_lowercase();
            }
        }
        let ua = headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok());
        Self {
            log,
            ev: Event {
                provider,
                host,
                client: client_from_user_agent(ua),
                mode: if route.is_some() { "gateway" } else { "proxy" }.into(),
                method: method.to_string(),
                endpoint: endpoint_for_path(path),
                bytes_in: bytes_in as u64,
                ..Event::default()
            },
            start: Instant::now(),
        }
    }
}

impl Drop for Pending {
    fn drop(&mut self) {
        self.ev.latency_ms = self.start.elapsed().as_millis() as u64;
        self.log.record(std::mem::take(&mut self.ev));
    }
}

/// Is `peer` the local machine (including IPv4-mapped IPv6 loopback)?
pub fn is_loopback(peer: &SocketAddr) -> bool {
    match peer.ip() {
        IpAddr::V4(v4) => v4.is_loopback(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.to_ipv4_mapped().is_some_and(|v| v.is_loopback()),
    }
}

/// `Host` header names a loopback address (defeats DNS rebinding from web pages).
fn host_is_local(headers: &HeaderMap) -> bool {
    let Some(h) = headers.get(header::HOST).and_then(|v| v.to_str().ok()) else {
        return true;
    };
    let h = h.trim().to_ascii_lowercase();
    let name = if let Some(rest) = h.strip_prefix('[') {
        rest.split(']').next().unwrap_or("")
    } else {
        h.rsplit_once(':').map(|(n, _)| n).unwrap_or(&h)
    };
    name == "localhost"
        || name.ends_with(".localhost")
        || name
            .parse::<IpAddr>()
            .is_ok_and(|ip| ip.is_loopback() || ip.is_unspecified())
}

fn query<'a>(q: Option<&'a str>, key: &str) -> Option<&'a str> {
    q?.split('&')
        .filter_map(|kv| kv.split_once('=').or(Some((kv, ""))))
        .find(|(k, _)| *k == key)
        .map(|(_, v)| v)
}

fn json_response(status: StatusCode, body: String) -> Response<Body> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::CACHE_CONTROL, "no-store")
        .header("x-content-type-options", "nosniff")
        .body(full(body))
        .expect("valid response")
}

fn asset(ct: &str, body: &'static str) -> Response<Body> {
    Response::builder()
        .header(header::CONTENT_TYPE, ct)
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONTENT_SECURITY_POLICY, UI_CSP)
        .header("x-content-type-options", "nosniff")
        .header("x-frame-options", "DENY")
        .header(header::REFERRER_POLICY, "no-referrer")
        .body(full(body))
        .expect("valid response")
}

impl Proxy {
    /// Start an activity event for a translated request (`None` when recording is off).
    pub(crate) fn pending(
        &self,
        url: &str,
        via: Via<'_>,
        method: &str,
        headers: &HeaderMap,
        bytes_in: usize,
    ) -> Option<Pending> {
        let log = self.activity.clone()?;
        Some(Pending::new(log, url, via, method, headers, bytes_in))
    }

    /// `/shadow/ui*` and `/shadow/activity*`, local peers only unless allowed by config.
    pub(crate) fn dashboard(
        &self,
        peer: SocketAddr,
        headers: &HeaderMap,
        path: &str,
        q: Option<&str>,
    ) -> Response<Body> {
        if !self.shadow.config().activity.allow_remote
            && (!is_loopback(&peer) || !host_is_local(headers))
        {
            return error_response(
                StatusCode::FORBIDDEN,
                "the Shadow dashboard only answers on loopback (set activity.allow_remote to change)",
            );
        }
        match path {
            "/shadow/ui" | "/shadow/ui/" | "/shadow/ui/index.html" => {
                asset("text/html; charset=utf-8", INDEX_HTML)
            }
            "/shadow/ui/app.js" => asset("text/javascript; charset=utf-8", APP_JS),
            "/shadow/ui/app.css" => asset("text/css; charset=utf-8", APP_CSS),
            "/shadow/ui/logo.svg" => asset("image/svg+xml", LOGO_SVG),
            "/shadow/activity" | "/shadow/activity/" => self.activity_events(q),
            "/shadow/activity/summary" => self.activity_summary(q),
            "/shadow/activity/stream" => self.activity_stream(q),
            _ => error_response(StatusCode::NOT_FOUND, "unknown dashboard path"),
        }
    }

    fn activity_events(&self, q: Option<&str>) -> Response<Body> {
        let Some(log) = &self.activity else {
            return json_response(
                StatusCode::OK,
                serde_json::json!({"enabled": false, "events": [], "last_id": 0}).to_string(),
            );
        };
        let since = query(q, "since").and_then(|s| s.parse().ok());
        let limit = query(q, "limit")
            .and_then(|s| s.parse().ok())
            .unwrap_or(100usize)
            .clamp(1, activity::RECENT_CAPACITY);
        let mut actions = vec![];
        for a in query(q, "action").unwrap_or("").split(',') {
            match a.trim() {
                "masked" => actions.push(Action::Masked),
                "blocked" => actions.push(Action::Blocked),
                "clean" => actions.push(Action::Clean),
                "incidents" => actions.extend([Action::Masked, Action::Blocked]),
                _ => {}
            }
        }
        let events = log.recent(since, limit, &actions);
        let body = serde_json::json!({
            "enabled": true,
            "last_id": log.last_id(),
            "events": events,
        });
        json_response(StatusCode::OK, body.to_string())
    }

    fn activity_summary(&self, q: Option<&str>) -> Response<Body> {
        let raw = query(q, "window").unwrap_or("24h");
        let Some(window) = Window::parse(raw) else {
            return error_response(
                StatusCode::BAD_REQUEST,
                "window must be one of 1h, 24h, 7d, 30d, today",
            );
        };
        let summary = match &self.activity {
            Some(log) => log.summary(window),
            None => Summary {
                enabled: false,
                window: window.as_str().into(),
                ..Summary::default()
            },
        };
        json_response(
            StatusCode::OK,
            serde_json::to_string(&summary).unwrap_or_default(),
        )
    }

    /// Server-Sent Events: one `activity` event per new [`Event`], keep-alives in between.
    fn activity_stream(&self, q: Option<&str>) -> Response<Body> {
        let Some(log) = self.activity.clone() else {
            return error_response(StatusCode::NOT_FOUND, "activity recording is disabled");
        };
        let start = query(q, "since")
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| log.last_id());
        let ticker = tokio::time::interval(Duration::from_millis(750));
        let stream = futures_util::stream::unfold(
            (log, start, ticker, 0u32),
            |(log, mut last, mut ticker, mut idle)| async move {
                loop {
                    ticker.tick().await;
                    let mut fresh = log.recent(Some(last), 500, &[]);
                    if fresh.is_empty() {
                        idle += 1;
                        if idle >= 20 {
                            return Some((
                                Ok(Frame::data(Bytes::from_static(b": keep-alive\n\n"))),
                                (log, last, ticker, 0),
                            ));
                        }
                        continue;
                    }
                    fresh.reverse();
                    let mut out = String::new();
                    for e in &fresh {
                        last = last.max(e.id);
                        out.push_str("event: activity\nid: ");
                        out.push_str(&e.id.to_string());
                        out.push_str("\ndata: ");
                        out.push_str(&serde_json::to_string(e).unwrap_or_default());
                        out.push_str("\n\n");
                    }
                    return Some((Ok(Frame::data(Bytes::from(out))), (log, last, ticker, 0)));
                }
            },
        );
        Response::builder()
            .header(header::CONTENT_TYPE, "text/event-stream")
            .header(header::CACHE_CONTROL, "no-store")
            .body(BodyExt::boxed_unsync(StreamBody::new(stream.boxed())))
            .expect("valid response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_detection() {
        assert!(is_loopback(&"127.0.0.1:5000".parse().unwrap()));
        assert!(is_loopback(&"[::1]:5000".parse().unwrap()));
        assert!(is_loopback(&"[::ffff:127.0.0.1]:5000".parse().unwrap()));
        assert!(!is_loopback(&"192.168.1.20:5000".parse().unwrap()));
        assert!(!is_loopback(&"172.17.0.1:5000".parse().unwrap()));
    }

    #[test]
    fn host_header_must_be_local() {
        let mut h = HeaderMap::new();
        for (v, ok) in [
            ("127.0.0.1:8877", true),
            ("localhost:8877", true),
            ("[::1]:8877", true),
            ("shadow.localhost", true),
            ("evil.example.com", false),
            ("evil.example.com:8877", false),
            ("10.0.0.5:8877", false),
        ] {
            h.insert(header::HOST, v.parse().unwrap());
            assert_eq!(host_is_local(&h), ok, "{v}");
        }
    }

    #[test]
    fn query_params() {
        assert_eq!(query(Some("a=1&window=7d"), "window"), Some("7d"));
        assert_eq!(query(Some("a=1"), "window"), None);
        assert_eq!(query(None, "window"), None);
    }
}
