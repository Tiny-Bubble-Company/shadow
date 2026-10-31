---
title: Integration modes
description: "Choose how to run Shadow: HTTPS proxy for every tool, gateway base URL for SDKs, or the embedded Rust library. How each mode works and when to use it."
---

Shadow runs the same masking engine in three ways. You can mix them on one machine: the proxy and the gateway are served from the same port, and they share the persona and vault.

| | Proxy | Gateway | Library |
|---|---|---|---|
| **How you connect** | `HTTPS_PROXY=http://127.0.0.1:8877`, or a PAC file | Base URL `http://127.0.0.1:8877/gw/<provider>/...` | `shadow-core` crate in your Rust process |
| **CA trust needed** | Yes, once | No | No |
| **Covers** | Any tool that honours proxy settings | Tools and SDKs that accept a base URL | Your own code |
| **Non-AI traffic** | Tunnelled without decryption | Not involved | Not involved |
| **Typical users** | Developer laptops, IDEs, desktop apps, CI runners | SDK-based apps, services, containers | Platforms that call models from Rust |
| **Guide** | [Proxy mode](/shadow/integrate/proxy-mode/) | [Gateway mode](/shadow/integrate/gateway-mode/) | [Embed in Rust](/shadow/integrate/rust/) |

## Choosing

**Start with the gateway** if the tool lets you set a base URL. It is explicit, it needs no certificate, and you can see in the configuration that requests go through Shadow.

**Use the proxy** when you want a whole machine covered, including tools you don't control. Shadow decrypts only the [known AI hosts](/shadow/reference/providers/) and tunnels everything else byte for byte, so banking, email and package downloads are never touched.

**Embed the library** when you are building the application yourself in Rust. There is no extra hop, you control exactly which strings are masked, and you can use [sessions](/shadow/integrate/rust/#sessions-and-intent) to apply the intent policy per request.

## Deployment patterns

| Where | Pattern |
|---|---|
| Laptop or workstation | `shadow install --apply`: background service, CA trusted, proxy env in your shell |
| Server or VM | systemd unit, gateway URLs in the app's environment |
| Container | Sidecar container on `localhost:8877`, gateway mode. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |
| Rust service | `shadow-core` in-process, data directory on a persistent volume |
| Anything else that talks to an LLM | See [Integrating any endpoint](/shadow/integrate/any-endpoint/) |

## What every mode does

Whatever the mode, a request goes through the same steps:

1. **Classify** the request's intent and pick a treatment per value ([intent policy](/shadow/concepts/intent-policy/)).
2. **Detect** sensitive values in every string of the JSON body, plus uploaded [files](/shadow/concepts/file-masking/).
3. **Translate** each value to its fake or token, recorded in the vault.
4. **Scrub** identifying fields and headers (proxy and gateway only).
5. **Restore** the response: JSON, server-sent events, NDJSON streams and tool-call arguments.

See [How it works](/shadow/concepts/how-it-works/) for the full lifecycle.
