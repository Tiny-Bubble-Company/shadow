//! # shadow-proxy
//!
//! A local HTTP(S) proxy that runs every AI API call through [`shadow_core::Shadow`].
//!
//! * **Forward proxy** (`HTTPS_PROXY=http://127.0.0.1:8877`): `CONNECT` tunnels to AI
//!   hosts are terminated with a leaf certificate from the local [`ca::CertAuthority`]
//!   and translated; every other host is tunnelled blindly, byte for byte.
//! * **Gateway** (`http://127.0.0.1:8877/gw/openai/v1/...`): explicit reverse proxy, no
//!   CA trust needed.
//!
//! Requests are masked (JSON-aware), identifying headers/fields are replaced with persona
//! values, and responses (including SSE streams) are unmasked before reaching the caller.
//!
//! Each translated request also produces a privacy-safe [`shadow_core::activity::Event`]
//! (counts only), served on loopback at `/shadow/activity*` and visualised at `/shadow/ui`.

mod activity;
mod apps;
pub mod ca;
mod detail;
mod forward;
pub mod hosts;
pub mod hub;
pub mod sse;

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http::{header, Method, Request, Response, StatusCode};
use http_body_util::combinators::UnsyncBoxBody;
use hyper::body::Incoming;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use shadow_core::activity::ActivityLog;
use shadow_core::config::ProxyConfig;
use shadow_core::Shadow;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

pub use activity::{is_loopback, UI_CSP};
pub use ca::CertAuthority;
pub use forward::{error_response, full};
pub use hosts::HostMatcher;

/// Response body type used by the proxy.
pub type Body = UnsyncBoxBody<Bytes, std::io::Error>;

/// The proxy: shared state for all connections.
pub struct Proxy {
    shadow: Shadow,
    ca: Arc<CertAuthority>,
    client: reqwest::Client,
    /// HTTP/1.1-only client for web apps (browser traffic arrives as HTTP/1.1, and some
    /// sites send responses over HTTP/2 that the client rejects, such as early hints).
    web_client: reqwest::Client,
    hosts: HostMatcher,
    routes: BTreeMap<String, String>,
    overrides: HashMap<String, String>,
    cfg: ProxyConfig,
    activity: Option<Arc<ActivityLog>>,
}

impl Proxy {
    /// Build a proxy from the Shadow instance's `[proxy]` config. Creates the CA in
    /// `<data_dir>/ca` if needed.
    pub fn new(shadow: Shadow) -> anyhow::Result<Self> {
        let cfg = shadow.config().proxy.clone();
        let ca = CertAuthority::load_or_create(&shadow.config().data_dir.join("ca"))?;
        let builder = || -> anyhow::Result<reqwest::ClientBuilder> {
            let b = reqwest::Client::builder()
                .use_rustls_tls()
                .redirect(reqwest::redirect::Policy::none())
                .pool_idle_timeout(std::time::Duration::from_secs(60))
                .connect_timeout(std::time::Duration::from_secs(15));
            Ok(match &cfg.upstream_proxy {
                Some(up) if !up.trim().is_empty() => b.proxy(reqwest::Proxy::all(up.as_str())?),
                _ => b.no_proxy(),
            })
        };
        let act = &shadow.config().activity;
        let activity = if act.enabled {
            let opened = ActivityLog::open(act, &shadow.config().activity_dir())
                .and_then(|log| {
                    if act.persist_details {
                        log.with_saved_details(&shadow.config().key_path(), act.details_max_mb)
                    } else {
                        Ok(log)
                    }
                })
                .and_then(|log| {
                    if act.receipts {
                        let c = shadow.config();
                        log.with_receipts(&c.receipt_key_path(), c.policy_hash())
                    } else {
                        Ok(log)
                    }
                });
            match opened {
                Ok(log) => Some(Arc::new(log)),
                Err(e) => {
                    tracing::warn!("activity log unavailable, keeping it in memory: {e:#}");
                    Some(Arc::new(ActivityLog::in_memory(act.retention_days)))
                }
            }
        } else {
            None
        };
        Ok(Self {
            activity,
            hosts: {
                let m = HostMatcher::new(&cfg.extra_hosts, &cfg.bypass_hosts);
                if cfg.chat_apps {
                    m.with_chat_apps()
                } else {
                    m
                }
            },
            routes: hosts::routes(&cfg.routes),
            shadow,
            ca: Arc::new(ca),
            client: builder()?.build()?,
            web_client: builder()?.http1_only().build()?,
            overrides: HashMap::new(),
            cfg,
        })
    }

    /// Send intercepted traffic for `host` to `base_url` instead (testing, private
    /// deployments). Example: `("api.openai.com", "http://127.0.0.1:9000")`.
    pub fn with_upstream_override(mut self, host: &str, base_url: &str) -> Self {
        self.overrides.insert(
            host.to_ascii_lowercase(),
            base_url.trim_end_matches('/').to_string(),
        );
        self
    }

    /// The activity log (`None` when `activity.enabled = false`).
    pub fn activity(&self) -> Option<&Arc<ActivityLog>> {
        self.activity.as_ref()
    }

    /// The local CA.
    pub fn ca(&self) -> &CertAuthority {
        &self.ca
    }

    /// Host matcher in use.
    pub fn hosts(&self) -> &HostMatcher {
        &self.hosts
    }

    /// Gateway routes in use.
    pub fn routes(&self) -> &BTreeMap<String, String> {
        &self.routes
    }

    /// Bind `addr` and serve until the task is dropped.
    pub async fn run(self: Arc<Self>, addr: SocketAddr) -> anyhow::Result<()> {
        let listener = TcpListener::bind(addr).await?;
        self.serve(listener).await
    }

    /// Serve connections from an already-bound listener.
    pub async fn serve(self: Arc<Self>, listener: TcpListener) -> anyhow::Result<()> {
        if let Some(log) = &self.activity {
            tokio::spawn(hub::run_uploader(
                log.clone(),
                self.shadow.config().data_dir.clone(),
            ));
        }
        loop {
            let (stream, peer) = listener.accept().await?;
            let me = self.clone();
            tokio::spawn(async move {
                if let Err(e) = me.serve_conn(stream, peer).await {
                    tracing::debug!("connection from {peer}: {e}");
                }
            });
        }
    }

    async fn serve_conn(
        self: Arc<Self>,
        stream: TcpStream,
        peer: SocketAddr,
    ) -> Result<(), hyper::Error> {
        let _ = stream.set_nodelay(true);
        let me = self.clone();
        hyper::server::conn::http1::Builder::new()
            .preserve_header_case(true)
            .title_case_headers(true)
            .serve_connection(
                TokioIo::new(stream),
                service_fn(move |req| me.clone().handle(req, peer)),
            )
            .with_upgrades()
            .await
    }

    async fn handle(
        self: Arc<Self>,
        mut req: Request<Incoming>,
        peer: SocketAddr,
    ) -> Result<Response<Body>, Infallible> {
        req.extensions_mut().insert(detail::ClientAddr(peer));
        if req.method() == Method::CONNECT {
            return Ok(self.connect(req, peer));
        }
        if req.uri().scheme().is_some() {
            // Plain-HTTP forward proxy request (absolute-form URI).
            let host = req.uri().host().unwrap_or("").to_string();
            let url = req.uri().to_string();
            let mode = self.mode_for(&host, req.method().as_str(), req.uri().path());
            return Ok(self
                .forward_via(req, url, mode, activity::Via::Proxy(None))
                .await);
        }
        let path = req.uri().path().to_string();
        if let Some(rest) = path.strip_prefix("/gw/") {
            return Ok(self.gateway(req, rest.to_string()).await);
        }
        if path == "/shadow/ui"
            || path.starts_with("/shadow/ui/")
            || path == "/shadow/activity"
            || path.starts_with("/shadow/activity/")
        {
            return Ok(self.dashboard(peer, req.headers(), &path, req.uri().query()));
        }
        Ok(match path.as_str() {
            "/" | "/shadow/status" => self.status(),
            "/healthz" => Response::new(full("ok")),
            "/proxy.pac" => {
                let addr = req
                    .headers()
                    .get(header::HOST)
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("127.0.0.1:8877")
                    .to_string();
                let all = req
                    .uri()
                    .query()
                    .is_some_and(|q| q.split('&').any(|kv| kv == "scope=all"));
                let pac = if all {
                    hosts::pac_all(&addr)
                } else {
                    hosts::pac_file(&self.hosts, &addr)
                };
                Response::builder()
                    .header(header::CONTENT_TYPE, "application/x-ns-proxy-autoconfig")
                    .header(header::CACHE_CONTROL, "no-cache")
                    .body(full(pac))
                    .expect("valid response")
            }
            "/shadow/ca.pem" => Response::builder()
                .header(header::CONTENT_TYPE, "application/x-pem-file")
                .body(full(self.ca.pem()))
                .expect("valid response"),
            _ => error_response(
                StatusCode::NOT_FOUND,
                "unknown path; use /gw/<provider>/... or configure Shadow as HTTPS proxy",
            ),
        })
    }

    /// Record a connection to a known AI service that Shadow cannot mask. Counts only, as
    /// for every event: host, provider and app, never content.
    fn record_passed(&self, host: &str, user_agent: Option<&str>) {
        let Some(log) = &self.activity else {
            return;
        };
        let host = host.to_ascii_lowercase();
        log.record(shadow_core::activity::Event {
            provider: shadow_core::activity::provider_for_host(&host).to_string(),
            client: shadow_core::activity::client_from_user_agent(user_agent),
            host,
            mode: "proxy".into(),
            method: "CONNECT".into(),
            action: shadow_core::activity::Action::Passed,
            status: 200,
            ..Default::default()
        });
    }

    /// How a request to `host` is handled: chat apps mask only their prompt requests and
    /// restore everything else; other intercepted hosts are masked as API calls.
    fn mode_for(&self, host: &str, method: &str, path: &str) -> forward::Mode {
        if !self.hosts.intercept(host) {
            return forward::Mode::Pass;
        }
        match apps::find(host).filter(|_| self.shadow.config().proxy.chat_apps) {
            Some(app) if app.is_api(path) => forward::Mode::Api,
            Some(app) if app.is_prompt(method, path) => forward::Mode::App(app),
            Some(_) => forward::Mode::Restore,
            None => forward::Mode::Api,
        }
    }

    fn status(&self) -> Response<Body> {
        let p = self.shadow.persona();
        let body = serde_json::json!({
            "shadow": env!("CARGO_PKG_VERSION"),
            "persona": {"region": p.region, "country": p.country, "company": p.company, "hostname": p.hostname},
            "vault": self.shadow.stats(),
            "routes": self.routes,
            "ca_fingerprint": self.ca.fingerprint(),
            "activity": {"enabled": self.activity.is_some(), "dashboard": "/shadow/ui"},
        });
        Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .body(full(body.to_string()))
            .expect("valid response")
    }

    async fn gateway(&self, req: Request<Incoming>, rest: String) -> Response<Body> {
        let (name, tail) = rest.split_once('/').unwrap_or((rest.as_str(), ""));
        let Some(base) = self.routes.get(&name.to_ascii_lowercase()) else {
            let known: Vec<&String> = self.routes.keys().collect();
            return error_response(
                StatusCode::NOT_FOUND,
                &format!("unknown gateway route {name:?}; known: {known:?}"),
            );
        };
        let mut url = format!("{base}/{tail}");
        if let Some(q) = req.uri().query() {
            url.push('?');
            url.push_str(q);
        }
        self.forward_via(req, url, forward::Mode::Api, activity::Via::Gateway(name))
            .await
    }

    fn connect(self: Arc<Self>, req: Request<Incoming>, peer: SocketAddr) -> Response<Body> {
        let Some(authority) = req.uri().authority().map(|a| a.to_string()) else {
            return error_response(StatusCode::BAD_REQUEST, "CONNECT needs host:port");
        };
        let host = authority
            .rsplit_once(':')
            .map(|(h, _)| h)
            .unwrap_or(&authority)
            .trim_matches(['[', ']'])
            .to_string();
        let intercept = self.hosts.intercept(&host);
        if !intercept && self.hosts.observed(&host) {
            let ua = req
                .headers()
                .get(header::USER_AGENT)
                .and_then(|v| v.to_str().ok());
            self.record_passed(&host, ua);
        }
        tokio::spawn(async move {
            let upgraded = match hyper::upgrade::on(req).await {
                Ok(u) => u,
                Err(e) => return tracing::debug!("upgrade failed: {e}"),
            };
            let io = TokioIo::new(upgraded);
            if intercept {
                if let Err(e) = self.mitm(io, host, authority, peer).await {
                    tracing::warn!("intercept failed: {e:#}");
                }
            } else {
                match TcpStream::connect(&authority).await {
                    Ok(mut upstream) => {
                        let mut io = io;
                        let _ = tokio::io::copy_bidirectional(&mut io, &mut upstream).await;
                    }
                    Err(e) => tracing::debug!("tunnel to {authority} failed: {e}"),
                }
            }
        });
        Response::new(full(Bytes::new()))
    }

    async fn mitm(
        self: Arc<Self>,
        io: TokioIo<hyper::upgrade::Upgraded>,
        host: String,
        authority: String,
        peer: SocketAddr,
    ) -> anyhow::Result<()> {
        let acceptor = TlsAcceptor::from(self.ca.server_config(&host)?);
        let tls = acceptor.accept(io).await?;
        let base = self
            .overrides
            .get(&host.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| {
                if authority.ends_with(":443") {
                    format!("https://{host}")
                } else {
                    format!("https://{authority}")
                }
            });
        let me = self.clone();
        hyper::server::conn::http1::Builder::new()
            .serve_connection(
                TokioIo::new(tls),
                service_fn(move |mut req: Request<Incoming>| {
                    req.extensions_mut().insert(detail::ClientAddr(peer));
                    let me = me.clone();
                    let base = base.clone();
                    let host = host.clone();
                    async move {
                        let pq = req
                            .uri()
                            .path_and_query()
                            .map(|p| p.as_str().to_string())
                            .unwrap_or_else(|| "/".into());
                        let via = activity::Via::Proxy(Some(&host));
                        let mode = me.mode_for(&host, req.method().as_str(), req.uri().path());
                        Ok::<_, Infallible>(
                            me.forward_via(req, format!("{base}{pq}"), mode, via).await,
                        )
                    }
                }),
            )
            .await?;
        Ok(())
    }
}
