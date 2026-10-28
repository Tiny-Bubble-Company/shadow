//! Request/response translation and upstream forwarding.

use std::io::Read;

use bytes::Bytes;
use futures_util::StreamExt;
use http::header::{self, HeaderMap, HeaderName, HeaderValue};
use http::{Request, Response, StatusCode};
use http_body_util::{BodyExt, Full, StreamBody};
use hyper::body::{Frame, Incoming};
use serde_json::Value;

use crate::sse::{EventStreamUnmasker, Framing};
use crate::{Body, Proxy};

const HOP_BY_HOP: &[&str] = &[
    "connection",
    "keep-alive",
    "proxy-connection",
    "proxy-authorization",
    "proxy-authenticate",
    "te",
    "trailer",
    "transfer-encoding",
    "upgrade",
    "host",
    "content-length",
];

/// A complete body.
pub fn full(b: impl Into<Bytes>) -> Body {
    Full::new(b.into())
        .map_err(|never| match never {})
        .boxed_unsync()
}

/// JSON error response in an OpenAI-compatible shape.
pub fn error_response(status: StatusCode, msg: &str) -> Response<Body> {
    let body = serde_json::json!({"error": {"message": msg, "type": "shadow_error"}});
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(full(body.to_string()))
        .expect("valid response")
}

fn is_json(ct: &str, body: &[u8]) -> bool {
    ct.contains("json")
        || matches!(
            body.iter().find(|b| !b.is_ascii_whitespace()),
            Some(b'{') | Some(b'[')
        )
}

fn decode(encoding: &str, body: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let ok = match encoding {
        "gzip" | "x-gzip" => flate2::read::GzDecoder::new(body)
            .read_to_end(&mut out)
            .is_ok(),
        "deflate" => flate2::read::ZlibDecoder::new(body)
            .read_to_end(&mut out)
            .is_ok(),
        "identity" | "" => {
            out = body.to_vec();
            true
        }
        _ => false,
    };
    ok.then_some(out)
}

impl Proxy {
    fn outbound_headers(&self, mut h: HeaderMap) -> HeaderMap {
        for name in HOP_BY_HOP.iter().chain(["accept-encoding"].iter()) {
            h.remove(*name);
        }
        for name in &self.cfg.strip_headers {
            if let Ok(n) = HeaderName::from_bytes(name.to_ascii_lowercase().as_bytes()) {
                h.remove(n);
            }
        }
        let p = self.shadow.persona();
        for (name, tpl) in &self.cfg.persona_headers {
            let Ok(n) = HeaderName::from_bytes(name.to_ascii_lowercase().as_bytes()) else {
                continue;
            };
            if h.contains_key(&n) {
                let v = tpl
                    .replace("{user_id}", &p.user_id)
                    .replace("{hostname}", &p.hostname)
                    .replace("{company}", &p.company)
                    .replace("{domain}", &p.domain)
                    .replace("{username}", &p.username);
                if let Ok(v) = HeaderValue::from_str(&v) {
                    h.insert(n, v);
                }
            }
        }
        h
    }

    /// Mask a request body. Returns the new body and replacement count, or `None` to
    /// forward the original bytes untouched.
    fn mask_body(&self, headers: &mut HeaderMap, body: &Bytes) -> Option<(Bytes, usize)> {
        if body.is_empty() {
            return None;
        }
        if headers
            .get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.starts_with("AWS4-HMAC-SHA256"))
        {
            tracing::warn!("SigV4-signed request: body cannot be modified without breaking the signature; forwarded unmasked");
            return None;
        }
        let enc = headers
            .get(header::CONTENT_ENCODING)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let Some(plain) = decode(enc.trim(), body) else {
            tracing::warn!("unsupported request content-encoding {enc:?}; forwarded unmasked");
            return None;
        };
        let ct = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let (out, n) = if is_json(&ct, &plain) {
            match serde_json::from_slice::<Value>(&plain) {
                Ok(mut v) => {
                    let n = self.shadow.mask_json(&mut v) + self.shadow.scrub_identifiers(&mut v);
                    (serde_json::to_vec(&v).ok()?, n)
                }
                Err(_) => {
                    let text = std::str::from_utf8(&plain).ok()?;
                    let m = self.shadow.mask(text);
                    (m.text.into_bytes(), m.replacements)
                }
            }
        } else if ct.starts_with("text/") || ct.contains("x-www-form-urlencoded") {
            let text = std::str::from_utf8(&plain).ok()?;
            let m = self.shadow.mask(text);
            (m.text.into_bytes(), m.replacements)
        } else {
            // multipart, audio, images, protobuf...: not scanned (documented limitation)
            return None;
        };
        headers.remove(header::CONTENT_ENCODING);
        Some((Bytes::from(out), n))
    }

    /// Forward `req` to `url`, translating both directions when `translate` is set.
    pub async fn forward(
        &self,
        req: Request<Incoming>,
        url: String,
        translate: bool,
    ) -> Response<Body> {
        let (parts, body) = req.into_parts();
        let body = match body.collect().await {
            Ok(b) => b.to_bytes(),
            Err(e) => {
                return error_response(
                    StatusCode::BAD_REQUEST,
                    &format!("reading request body: {e}"),
                )
            }
        };
        let mut headers = parts.headers.clone();
        let mut out_body = body.clone();
        if translate {
            headers = self.outbound_headers(headers);
            if let Some((b, n)) = self.mask_body(&mut headers, &body) {
                tracing::info!(target: "shadow::proxy", "{} {} masked {n} value(s)", parts.method, url);
                out_body = b;
            }
        } else {
            for name in HOP_BY_HOP {
                headers.remove(*name);
            }
        }
        let resp = match self
            .client
            .request(parts.method.clone(), &url)
            .headers(headers)
            .body(out_body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("upstream request failed: {e}"),
                )
            }
        };
        self.translate_response(resp, translate).await
    }

    async fn translate_response(&self, resp: reqwest::Response, translate: bool) -> Response<Body> {
        let status = resp.status();
        let mut headers = resp.headers().clone();
        for name in HOP_BY_HOP {
            headers.remove(*name);
        }
        let encoded = headers.contains_key(header::CONTENT_ENCODING);
        let ct = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();
        let framing = if ct.starts_with("text/event-stream") {
            Some(Framing::Sse)
        } else if ct.contains("ndjson") || ct.contains("jsonl") || ct.contains("json-seq") {
            Some(Framing::Lines)
        } else {
            None
        };
        let mut builder = Response::builder().status(status);
        if let Some(h) = builder.headers_mut() {
            *h = headers;
        }
        if !translate || encoded {
            let stream = resp
                .bytes_stream()
                .map(|r| r.map(Frame::data).map_err(std::io::Error::other));
            return builder
                .body(BodyExt::boxed_unsync(StreamBody::new(stream)))
                .expect("valid response");
        }
        if let Some(framing) = framing {
            let unmasker = EventStreamUnmasker::new(self.shadow.clone(), framing);
            let upstream = resp.bytes_stream().boxed();
            let stream = futures_util::stream::unfold(
                (upstream, unmasker, false),
                |(mut up, mut un, done)| async move {
                    if done {
                        return None;
                    }
                    loop {
                        match up.next().await {
                            Some(Ok(chunk)) => {
                                let out = un.push(&chunk);
                                if !out.is_empty() {
                                    return Some((
                                        Ok(Frame::data(Bytes::from(out))),
                                        (up, un, false),
                                    ));
                                }
                            }
                            Some(Err(e)) => {
                                return Some((Err(std::io::Error::other(e)), (up, un, true)))
                            }
                            None => {
                                return Some((
                                    Ok(Frame::data(Bytes::from(un.finish()))),
                                    (up, un, true),
                                ))
                            }
                        }
                    }
                },
            );
            return builder
                .body(BodyExt::boxed_unsync(StreamBody::new(stream)))
                .expect("valid response");
        }
        let bytes = match resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return error_response(
                    StatusCode::BAD_GATEWAY,
                    &format!("reading upstream body: {e}"),
                )
            }
        };
        let out = match std::str::from_utf8(&bytes) {
            Ok(text) if self.shadow.contains_fake(text) => {
                if is_json(&ct, &bytes) {
                    match serde_json::from_str::<Value>(text) {
                        Ok(mut v) => {
                            self.shadow.unmask_json(&mut v);
                            Bytes::from(serde_json::to_vec(&v).unwrap_or_default())
                        }
                        Err(_) => Bytes::from(self.shadow.unmask(text)),
                    }
                } else {
                    Bytes::from(self.shadow.unmask(text))
                }
            }
            _ => bytes,
        };
        builder.body(full(out)).expect("valid response")
    }
}
