---
title: Troubleshooting
description: Fixes for common problems with certificates, proxies, gateways, detection and streaming.
---

Start with:

```sh
shadow status
RUST_LOG=debug shadow run
```

`shadow status` tells you whether the proxy answers, where the data directory is, and which CA is in use. Debug logs show each connection and whether it was intercepted or tunnelled.

## Certificates

### `certificate verify failed`, `self signed certificate in certificate chain`, `UNABLE_TO_GET_ISSUER_CERT_LOCALLY`

The client does not trust Shadow's CA.

- Run `shadow install --apply`, or trust `ca/ca.pem` by hand ([how](/shadow/integrate/proxy-mode/#2-trust-the-local-ca)).
- Node.js: set `NODE_EXTRA_CA_CERTS` to `ca/ca.pem`.
- Python `requests`: set `REQUESTS_CA_BUNDLE` to `ca/bundle.pem`. `httpx` and OpenSSL-based tools: `SSL_CERT_FILE`.
- curl: `CURL_CA_BUNDLE`.
- Java: import `ca.pem` into the JVM's truststore with `keytool -importcert`.
- Or switch the tool to [gateway mode](/shadow/integrate/gateway-mode/), which needs no CA.

### Other HTTPS sites broke after setting the proxy

They should not: non-AI hosts are tunnelled without TLS interception. If one did, check that its host is not matched by an `extra_hosts` glob, and add it to `bypass_hosts`.

### An app still fails with the CA trusted

It probably pins certificates. Add its host to `bypass_hosts` (it will not be masked) or use a client that supports a base URL.

### I regenerated the CA and things broke

`shadow init --force` does not recreate the CA, but deleting `ca/` does. Run `shadow install --apply` again to trust the new one, and re-point `NODE_EXTRA_CA_CERTS` and friends.

## Proxy and gateway

### `Connection refused` on 127.0.0.1:8877

Shadow is not running. Start it with `shadow run`, or check the service: `launchctl print gui/$(id -u)/dev.shadow.proxy` (macOS), `systemctl --user status shadow` (Linux), `schtasks /Query /TN Shadow` (Windows).

### `address already in use`

Another process (often a second Shadow) holds the port. Stop it, or use `--listen 127.0.0.1:8878` and update your settings.

### `unknown gateway route "..."`

The first path segment after `/gw/` must be a route name. Check `shadow hosts` and add custom routes under `[proxy.routes]`.

### 404 from the provider through the gateway

The path after the route is appended to the route's base URL as is. Compare with the [route table](/shadow/integrate/gateway-mode/#built-in-routes): for example the OpenAI SDK needs `/gw/openai/v1`, while the Anthropic SDK needs `/gw/anthropic` because it adds `/v1` itself.

### Requests bypass Shadow

- The tool ignores `HTTPS_PROXY`. Use its own proxy setting, the system proxy / PAC file, or gateway mode.
- `NO_PROXY` matches the provider host. Remove it from `NO_PROXY`.
- The host is not on the intercept list. Add it with `extra_hosts`.

## Detection

### A value was not masked

1. Put the text in a file and run `shadow scan file.txt --reveal`.
2. If it is a name, codename or customer, add it: `shadow terms add "person:Jane Doe"`.
3. If it is a domain of yours, add it to `internal_domains`.
4. Check it is not on `allow_terms` or `allow_domains`, and that its detector is enabled.
5. For free-text names, enable the NER sidecar or Presidio.

### Something was masked that shouldn't be

Add it to `allow_terms` (exact values) or `allow_domains` (public domains), or disable the detector under `[detectors]`.

### The answer contains a fake instead of the real value

The response contained a variant of the fake that is not in the vault, for example a changed case, a fake email split across lines, or a fake that the model abbreviated. Exact and case variants are restored; paraphrases cannot be. Typed tokens for reports avoid most of this.

### Presidio or the NER sidecar seems ignored

Both fail open with a warning in the log. Check the URL, that the service is running (`curl http://127.0.0.1:5002/health` for Presidio, `curl http://127.0.0.1:8878/healthz` for the sidecar), and the `min_score`.

## Streaming

### Streams arrive in larger chunks than before

Shadow holds back at most the length of the longest fake, so a fake split across events can be restored. That is a few characters, not a buffering of the whole response. If a stream stalls entirely, run with `RUST_LOG=debug` and report it.

## Vault

### `vault.key` permission or decryption errors

The key does not match `vault.bin`, or the file was modified. Restore both from the same backup. If neither can be recovered, move them away and Shadow starts a new vault; old fakes will no longer be restored.

## Getting help

Open an issue with the output of `shadow --version`, `shadow status`, your OS, the client you used, and a `RUST_LOG=debug` log with any sensitive content removed. Security issues go through [`SECURITY.md`](https://github.com/Tiny-Bubble-Company/shadow/blob/main/SECURITY.md).
