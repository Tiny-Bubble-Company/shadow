---
title: Configuration
description: Every key in shadow.toml, with types, defaults and examples.
---

Shadow reads `shadow.toml` from the data directory, or from the path given by `--config` or `SHADOW_CONFIG`. **Every key is optional**: an empty file, or no file at all, is a valid configuration. `shadow init` writes a minimal one. [`shadow.example.toml`](https://github.com/Tiny-Bubble-Company/shadow/blob/main/shadow.example.toml) in the repository is an annotated template.

## Locations

| | |
|---|---|
| Config file | `--config <path>`, else `$SHADOW_CONFIG`, else `<data dir>/shadow.toml` |
| Data directory | `--data-dir`, else `data_dir` in the config, else `$SHADOW_HOME`, else the platform default |
| macOS default | `~/Library/Application Support/shadow` |
| Linux default | `~/.local/share/shadow` |
| Windows default | `%APPDATA%\shadow` |

When the config file has no `data_dir`, the data directory is the folder that contains the file. A relative `data_dir` is resolved against that folder.

## Top level

| Key | Type | Default | Description |
|---|---|---|---|
| `data_dir` | path | see above | Where `persona.json`, `vault.bin`, `vault.key`, `terms.json` and `ca/` live |
| `organization` | list of strings | `[]` | Legal names of your organisation(s). The first maps to the persona company. The bare name without its legal suffix is protected too |
| `terms` | list of strings | `[]` | Dictionary terms: codenames, customers, projects, people. Optional kind prefix: `person:`, `org:`, `term:`, `hostname:`, `email:`, `username:` |
| `internal_domains` | list of strings | `[]` | Domains always treated as internal and masked in URLs, hostnames and emails, including subdomains |
| `allow_domains` | list of strings | `[]` | Public domains never masked, added to the built-in allow list |
| `allow_terms` | list of strings | `[]` | Values never masked, even when detected (case-insensitive) |

```toml
organization = ["Acme GmbH"]
terms = ["Project Falcon", "Nightjar", "org:Globex", "person:Jane Doe"]
internal_domains = ["acme.com", "acme.internal"]
allow_domains = ["mycompany-public-docs.io"]
allow_terms = []
```

## `[persona]`

| Key | Type | Default | Description |
|---|---|---|---|
| `region` | string | `"auto"` | ISO code of the fake region: `CN`, `JP`, `KR`, `BR`, `US`, `DE`, `IN`, `AU`, `MX`, `ZA`, `CA`. `auto` picks a region far from the machine's timezone |
| `company` | string | generated | Override the persona company name |
| `domain` | string | generated | Override the persona company domain |

The persona is generated once. Changing `region` later has no effect until you run `shadow init --force`.

## `[detectors]`

All `true` by default. See [Detection](/shadow/concepts/detection/).

| Key | Detects |
|---|---|
| `email` | Email addresses |
| `phone` | Phone numbers |
| `person` | Person names |
| `address` | Street addresses |
| `iban` | IBANs (mod-97 checked) |
| `credit_card` | Card numbers (Luhn checked) |
| `national_id` | SSN, UK NINO, Aadhaar |
| `ip` | IPv4 and IPv6 addresses |
| `mac` | MAC addresses |
| `hostname` | Non-public hostnames and internal domains |
| `secret` | API keys, tokens, private keys, password assignments |
| `path` | Usernames in home-directory paths |
| `local_facts` | OS user, hostname, git `user.name` / `user.email`, local domain |

## `[model_detector]`

The optional local NER sidecar in `ml/`.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Call the sidecar for every scanned string |
| `url` | string | `"http://127.0.0.1:8878/detect"` | Must be a loopback URL |
| `timeout_ms` | integer | `300` | Per-call timeout. On timeout or error Shadow continues without model results |
| `min_score` | float | `0.6` | Minimum entity score to accept |

## `[presidio]`

A [Microsoft Presidio](/shadow/concepts/detection/#presidio) Analyzer as an extra detector.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `false` | Call Presidio for every scanned string |
| `url` | string | `"http://127.0.0.1:5002/analyze"` | The Analyzer's `/analyze` endpoint |
| `language` | string | `"en"` | Language passed to Presidio |
| `min_score` | float | `0.5` | Minimum score to accept a Presidio finding |
| `entities` | list of strings | `[]` | Presidio entity types to request; empty means all |

Shadow fails open if Presidio is unreachable: it logs a warning and uses its built-in detectors.

## `[policy]`

The [intent-aware policy](/shadow/concepts/intent-policy/). Treatments are `surrogate`, `token`, `keep` and `redact`.

| Key | Type | Default | Description |
|---|---|---|---|
| `intent` | bool | `true` | Classify each request as `act`, `report`, `code` or `converse`. When `false`, every request is handled as `converse` |

### `[policy.act]`

| Key | Type | Default | Description |
|---|---|---|---|
| `targets` | `surrogate` \| `keep` | `"surrogate"` | Treatment of the values an action is aimed at when the tool runs locally |
| `remote_tool_targets` | `surrogate` \| `keep` | `"keep"` | Treatment of action targets when the tool runs in the provider's cloud (hosted MCP, server-side tools). Each keep is logged |

### `[policy.report]`

Per-kind treatments for `report` requests. Keys are detector kinds (`email`, `phone`, `person`, `org`, `term`, `address`, `iban`, `credit_card`, `national_id`, `ip`, `mac`, `hostname`, `secret`, `username`) plus `default`.

| Key | Default |
|---|---|
| `person` | `"token"` |
| `email` | `"token"` |
| `phone` | `"token"` |
| `address` | `"token"` |
| `default` | `"surrogate"` |

### `[policy.always]`

Per-kind treatments that apply to every request, regardless of intent. Empty by default. Example:

```toml
[policy.always]
secret = "redact"   # never send secrets in any form; irreversible [REDACTED_SECRET]
```

## `[files]`

[File masking](/shadow/concepts/file-masking/) for uploads and embedded files.

| Key | Type | Default | Description |
|---|---|---|---|
| `enabled` | bool | `true` | Mask multipart uploads, base64 file payloads and `data:` URIs |
| `pdf` | `text` \| `block` \| `pass` | `"text"` | Send extracted, masked text; refuse PDFs; or pass them unmasked |
| `images` | `pass` \| `block` | `"pass"` | Images are not OCR'd. `block` refuses requests that carry images |
| `max_bytes` | integer | `52428800` | Largest file Shadow will process (50 MiB) |

## `[proxy]`

| Key | Type | Default | Description |
|---|---|---|---|
| `listen` | string | `"127.0.0.1:8877"` | Address for the forward proxy and the gateway. Shadow warns if it is not loopback |
| `extra_hosts` | list of globs | `[]` | Extra hosts to intercept in proxy mode, e.g. a self-hosted LLM gateway |
| `bypass_hosts` | list of globs | `[]` | Hosts never intercepted (blind tunnel), even if built in |
| `upstream_proxy` | string | none | Egress relay for intercepted traffic: `http://`, `socks5://` or `socks5h://` |
| `strip_headers` | list of strings | see below | Request headers removed before forwarding (case-insensitive) |

Default `strip_headers`:

```toml
strip_headers = [
  "openai-organization", "x-forwarded-for", "x-real-ip", "forwarded", "via", "x-client-ip",
  "x-stainless-os", "x-stainless-arch", "x-stainless-runtime-version",
]
```

Remove `"openai-organization"` from the list if you rely on a non-default OpenAI organisation. Setting `strip_headers` replaces the default list.

### `[proxy.persona_headers]`

Request headers whose value is replaced with a persona value, **when the header is present**. Placeholders: `{user_id}`, `{hostname}`, `{company}`, `{domain}`, `{username}`.

```toml
[proxy.persona_headers]
"x-request-origin" = "{hostname}"
"x-user-id" = "{user_id}"
"x-client-hostname" = "{hostname}"
```

These three are the defaults.

### `[proxy.routes]`

Extra gateway routes: `http://127.0.0.1:8877/gw/<name>/<path>` → `<base>/<path>`. A route with a built-in name overrides it. See [Gateway mode](/shadow/integrate/gateway-mode/#built-in-routes).

```toml
[proxy.routes]
azure = "https://my-resource.openai.azure.com"
bedrock = "https://bedrock-runtime.eu-central-1.amazonaws.com"
```

## Environment variables

| Variable | Effect |
|---|---|
| `SHADOW_HOME` | Data directory |
| `SHADOW_CONFIG` | Config file path (same as `--config`) |
| `RUST_LOG` | Log filter, e.g. `info`, `debug`, `shadow=debug`. `shadow run` defaults to `info`, other commands to `warn` |

## A complete example

```toml
organization = ["Acme GmbH"]
terms = ["Project Falcon", "org:Globex", "person:Jane Doe"]
internal_domains = ["acme.com", "acme.internal"]

[persona]
region = "JP"

[detectors]
mac = false

[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"

[policy]
intent = true
[policy.act]
remote_tool_targets = "keep"
[policy.report]
person = "token"
email = "token"
default = "surrogate"
[policy.always]
secret = "redact"

[files]
pdf = "text"
images = "block"

[proxy]
listen = "127.0.0.1:8877"
bypass_hosts = ["api.cohere.com"]
[proxy.routes]
azure = "https://my-resource.openai.azure.com"
```
