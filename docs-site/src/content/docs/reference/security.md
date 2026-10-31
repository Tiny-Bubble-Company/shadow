---
title: Security model
description: "Shadow's security model: what it protects from AI providers, what it trusts, how the vault and local CA are secured, and its known limitations."
---

Shadow's job is narrow and specific: **keep the content of AI requests away from AI providers**, and from everyone downstream of them, while keeping the tools that send those requests useful.

## What Shadow protects

| Asset | Protected from |
|---|---|
| Personal data in prompts, history, tool results and uploaded files | The provider, its subprocessors, its logs, abuse review, training pipelines, and breaches of any of them |
| Your organisation's names, codenames, customers | Same |
| Secrets pasted into prompts | Same (as fakes by default, or [redacted](/shadow/concepts/intent-policy/#example-a-secret-with-redaction-turned-on)) |
| Machine identity: OS user, hostname, git identity, paths | Same |
| Request metadata: OpenAI `user`, Anthropic `metadata.user_id`, organisation and OS headers | Same |

## Trust boundaries

**Trusted**: the endpoint running Shadow, the user account it runs as, and anything that can read its data directory.

**Not trusted**: the network between Shadow and the provider (TLS is used), the provider itself, and anything that receives data from the provider.

**Out of scope**: an attacker who already controls your machine or user account. They can read the vault key, the vault and the CA key, and they can read your prompts before Shadow does.

## Design properties

- **Local only.** No hosted component, no account, no telemetry. Shadow sends nothing except the masked requests you were already sending, to the hosts you were already sending them to.
- **Encrypted vault.** ChaCha20-Poly1305 with a key in `vault.key` (mode `0600`), atomic writes. See [The vault](/shadow/concepts/vault/).
- **Minimal interception.** In proxy mode only listed AI hosts are decrypted; everything else is a byte-for-byte tunnel. Bypassed hosts are never decrypted.
- **Constrained CA.** The local CA has path length 0, its key is created `0600`, it is unique per machine, and `shadow uninstall --apply` removes its trust.
- **Loopback by default.** Proxy and gateway listen on `127.0.0.1`. Shadow warns when you bind anything else. The NER sidecar URL must be loopback, and the sidecar refuses non-loopback binds unless forced.
- **No values in logs.** Logs contain methods, URLs, counts, intents and kinds, never real or fake values.
- **Fail closed on transport, fail open on optional detectors.** If Shadow is down, proxied tools fail to connect rather than bypass it. If the NER sidecar or Presidio is down, the built-in detectors still run.
- **Explicit decisions.** Values sent in clear text through `keep` are logged with kind and reason. Installation steps are printed before they run.
- **Memory-safe implementation.** Rust, rustls for TLS, no OpenSSL.

## Limitations

These are inherent to the approach or not yet addressed. Read them before relying on Shadow.

- **Your IP address and account are visible.** Translation hides content, not who you are to the provider. The API key identifies your account; the source IP identifies your network. `upstream_proxy` can route AI traffic through a relay, but the key still identifies the account.
- **Detection is best effort.** Regexes, checksums and dictionaries miss unusual formats and names they don't know. Add your own terms, enable the NER sidecar or Presidio, and test with `shadow scan`.
- **Context can identify.** Masking values does not mask meaning. A prompt that describes a unique event, product or situation can identify a company even with every name replaced.
- **Not scanned:** text inside images (no OCR), audio, WebSocket and Realtime APIs, gRPC, files above `max_bytes`, files referenced by an earlier upload id.
- **`keep` sends real values.** By default, the target of an action executed by a provider-hosted tool is sent in clear text so the action works. Set `remote_tool_targets = "surrogate"` if that is not acceptable.
- **Signed requests** (AWS SigV4) are forwarded unmasked with a warning.
- **Certificate-pinning apps** reject the local CA in proxy mode.
- **Meaning can shift.** The model reasons about the persona: a persona in Shenzhen gets answers about Chinese holidays. Pick a region that suits your work.
- **Coincidental fakes.** If a model produces text that happens to equal a fake on its own, it is translated back too. This is rare, and harmless in practice.
- **Surrogate secrets look real.** A fake API key has the same shape as a real one. That keeps code and configs coherent, but if you need a guarantee that no secret-shaped value is sent, use `[policy.always] secret = "redact"`.

## Operational guidance

- Use full-disk encryption on machines that run Shadow.
- Never share the data directory. It contains the vault key and the CA key.
- On servers, run Shadow as a dedicated user with the systemd unit's hardening, and restrict the data directory to that user.
- Keep the proxy and gateway on loopback or inside the pod. Anyone who can reach the port can use your persona and have responses unmasked with your vault.
- Review the per-request log during rollout and run `shadow scan` on representative content.

## Reporting a vulnerability

Please report security issues privately. See [`SECURITY.md`](https://github.com/Tiny-Bubble-Company/shadow/blob/main/SECURITY.md).
