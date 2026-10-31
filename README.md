<p align="center">
  <img src="assets/banner.svg" alt="Shadow: privacy layer for AI traffic. Your data leaves under a different name." width="100%">
</p>

<p align="center">
  <strong>Shadow masks names, emails, customers and secrets with realistic, reversible fakes before a request reaches an AI provider, then restores them in the answer.</strong>
</p>

<p align="center">
  <a href="LICENSE"><img alt="License: Apache-2.0" src="https://img.shields.io/badge/license-Apache--2.0-7C6CFF?style=flat-square"></a>
  <img alt="Rust 1.88+" src="https://img.shields.io/badge/rust-1.88%2B-B7410E?style=flat-square&logo=rust&logoColor=white">
  <img alt="Platforms: macOS, Linux, Windows" src="https://img.shields.io/badge/platforms-macOS%20%7C%20Linux%20%7C%20Windows-3A3660?style=flat-square">
  <a href="https://tiny-bubble-company.github.io/shadow/"><img alt="Docs" src="https://img.shields.io/badge/docs-tiny--bubble--company.github.io%2Fshadow-5A48E8?style=flat-square"></a>
</p>

<p align="center">
  <a href="https://tiny-bubble-company.github.io/shadow/"><b>Docs</b></a> ·
  <a href="#quick-start"><b>Quick start</b></a> ·
  <a href="#how-it-works"><b>How it works</b></a> ·
  <a href="#use-it-from-your-app"><b>Integrations</b></a> ·
  <a href="CONTRIBUTING.md"><b>Contributing</b></a>
</p>

---

Shadow is an endpoint privacy layer between your apps and AI clouds: OpenAI, Anthropic, Gemini, Azure OpenAI, OpenRouter, Mistral, Groq and more. It runs on your machine as a local proxy or gateway, or inside your Rust service as a library. It finds sensitive values in every outbound request, swaps them for format-preserving fakes from a persistent **persona**, keeps the mapping in an encrypted local vault, and translates responses back, including streamed responses and tool calls.

```text
you      ──"Email jane@acme.com about Project Falcon for Acme GmbH"──────────────────────▶ Shadow
Shadow   ──"Email hui.zhao@zhilan.cn about Velora Quilent for Zhilan Technology Co., Ltd."──▶ provider
provider ──"Sure, drafting a note to hui.zhao@zhilan.cn…"────────────────────────────────▶ Shadow
Shadow   ──"Sure, drafting a note to jane@acme.com…"─────────────────────────────────────▶ you
```

Single Rust binary · Apache-2.0 · no cloud, no account, no telemetry.

## Why

Prompts carry far more than the question: colleagues' names, customer lists, invoices with IBANs, internal hostnames, file paths with your username, API keys pasted by accident. Policies say "don't paste that", and people paste it anyway because the tool is useful.

Most DLP gateways **block**, which breaks the tool, so people route around them. Plain redaction (`<PERSON>`) keeps the request flowing but destroys meaning: the model can't tell two people apart or write to the right address.

Shadow **translates**. A fake email is still an email. A fake card still passes Luhn. A fake IBAN still passes mod-97. The model gets a coherent prompt about a company that does not exist, and you get a useful answer about yours.

## Before and after

| What you send | What the provider sees |
|---|---|
| `jane@acme.com` | `hui.zhao@zhilan.cn` |
| `Acme GmbH` | `Zhilan Technology Co., Ltd.` |
| `Project Falcon` | `Velora Quilent` |
| `4111 1111 1111 1111` | `4837 2191 9471 3480` (valid Luhn) |
| `DE89 3704 0044 0532 0130 00` | another German IBAN (valid mod-97) |
| `/Users/jdoe/src/billing` | `/Users/huizhao/src/billing` |
| `sk-proj-…` (an API key) | a fake key of the same shape, or `[REDACTED_SECRET]` if you opt in |
| 400 customer rows in a report | `[PERSON_0001]`, `[EMAIL_0001]`, … restored in the answer |

The same real value always gets the same fake, and no two real values share one, so restoring is exact.

## Features

| | |
|---|---|
| **Reversible, format-preserving fakes** | Emails, phones, names, addresses, IBANs, cards, national IDs, IPs, MACs, hostnames, secrets, paths, from one coherent persona |
| **Three integration modes** | HTTPS proxy for every tool, `/gw/<provider>` gateway with no CA, or the `shadow-core` Rust library |
| **Stream-aware** | Restores SSE and NDJSON streams on the fly, even when a fake is split across events |
| **Agent-safe** | Tool-call arguments are restored on your machine before your tools run |
| **Intent-aware policy** | Classifies requests as `act`, `report`, `code` or `converse` and picks a surrogate, token, keep or redact per value |
| **File masking** | Text, CSV, JSON, HTML, XML; DOCX, XLSX, PPTX masked in place; PDF text extracted and masked |
| **Pluggable detection** | Regex, checksums, your dictionary, local machine facts, an optional local NER model, and Microsoft Presidio |
| **Local and encrypted** | ChaCha20-Poly1305 vault, `0600` key, atomic multi-process writes; nothing leaves except the masked request |
| **Minimal interception** | Only known AI hosts are decrypted; all other traffic is tunnelled byte for byte |
| **Identity scrubbing** | OpenAI `user`, Anthropic `metadata.user_id`, org and OS headers replaced or stripped |

## Quick start

**Install**

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh

# Windows (PowerShell)
irm https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.ps1 | iex

# From source (Rust 1.88+)
cargo install --locked --git https://github.com/Tiny-Bubble-Company/shadow shadow-cli

# Docker
docker build -f packaging/Dockerfile -t shadow .
docker run -d --name shadow -p 127.0.0.1:8877:8877 -v shadow-data:/data shadow
```

The install scripts use a release binary when one is published for your platform and otherwise build with `cargo`. They never change system settings.

**Set up and try it offline**

```sh
shadow init --org "Acme GmbH"          # persona + vault key + local CA
echo "Mail jane@acme.com re Acme GmbH, card 4111 1111 1111 1111" | shadow mask
# Mail tsun@xinghe.cn re Xinghe Technology Co., Ltd., card 4261 8002 0172 9224
```

**Run it and send a protected request**

```sh
shadow run                             # foreground, 127.0.0.1:8877

# Gateway mode: no certificate needed
export OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1
export ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic
```

**Cover every tool on the machine (proxy mode)**

```sh
shadow install            # dry run: prints exactly what would change
shadow install --apply    # background service + trust the local CA + env file
export HTTPS_PROXY=http://127.0.0.1:8877
```

**Verify**

```sh
shadow status             # proxy health, persona, vault, CA fingerprint
shadow identity           # the persona providers see
shadow scan prompt.txt    # what would be masked, without changing anything
```

Full walkthrough: [Quick start](https://tiny-bubble-company.github.io/shadow/getting-started/quick-start/).

## Use it from your app

**OpenAI SDK (Python)**

```python
from openai import OpenAI

client = OpenAI(base_url="http://127.0.0.1:8877/gw/openai/v1")
reply = client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[{"role": "user", "content": "Draft a renewal reminder to jane@acme.com"}],
)
print(reply.choices[0].message.content)   # mentions jane@acme.com; the provider never saw it
```

**OpenAI SDK (Node.js)**

```ts
import OpenAI from "openai";
const client = new OpenAI({ baseURL: "http://127.0.0.1:8877/gw/openai/v1" });
```

**Anthropic SDK**

```python
import anthropic
client = anthropic.Anthropic(base_url="http://127.0.0.1:8877/gw/anthropic")
```

**Any tool that honours a proxy**

```sh
HTTPS_PROXY=http://127.0.0.1:8877 your-tool
```

**Embedded in Rust**

```toml
[dependencies]
shadow-core = { git = "https://github.com/Tiny-Bubble-Company/shadow" }
```

```rust
use shadow_core::{Shadow, ShadowConfig};

let shadow = Shadow::open(
    ShadowConfig::from_dir("/var/lib/myapp/shadow").with_organization(vec!["Acme GmbH".into()]),
)?;

// Simple: realistic surrogates for everything.
let masked = shadow.mask("Ping jane@acme.com about Acme GmbH");
let answer = shadow.unmask(&call_llm(&masked.text)?);

// Per request, with the intent-aware policy and tool calls.
let session = shadow.session_for(&request_json);
session.mask_json(&mut request_json);
let mut response = send(&request_json)?;
session.unmask_json(&mut response);           // tool-call arguments come back real
```

Gateway routes are built in for `openai`, `anthropic`, `openrouter`, `gemini`, `mistral`, `groq`, `together`, `deepseek`, `xai`, `cohere`, `perplexity`, `fireworks` and `cerebras`; add Azure, Bedrock or your own under `[proxy.routes]`. See [Integrations](https://tiny-bubble-company.github.io/shadow/integrate/overview/).

## How it works

```mermaid
flowchart LR
  A[App · SDK · CLI · agent] -- "HTTPS_PROXY or /gw/…" --> D
  subgraph S["Shadow (on your machine)"]
    D[Detect<br/>regex · checksum · dictionary<br/>NER · Presidio] --> P[Policy<br/>act · report · code · converse]
    P --> T[Translate<br/>surrogate · token · keep · redact]
    T <--> V[(Vault<br/>encrypted)]
    R[Restore<br/>JSON · SSE · tool calls] <--> V
    PE[Persona] --> T
  end
  T -- masked request --> C[(AI provider)]
  C -- response / stream --> R --> A
  A -. other hosts: blind tunnel .-> X[(Internet)]
```

1. **Intercept.** Shadow listens on `127.0.0.1:8877`. As an HTTPS proxy, it terminates TLS only for known AI hosts, using a local CA you trust once; every other host is tunnelled untouched. As a gateway (`/gw/openai/...`) it needs no CA.
2. **Classify.** Each request gets an intent: `act`, `report`, `code` or `converse`.
3. **Detect.** Built-in detectors, your dictionary, local machine facts and optional NER or Presidio find sensitive spans in every JSON string and uploaded file.
4. **Translate.** Each value gets the treatment the policy picks, by default a stable fake from the persona, recorded in the vault.
5. **Scrub.** Identifying fields and headers are replaced with persona values or removed.
6. **Restore.** Responses, SSE streams and tool-call arguments are translated back before they reach your app.

Details: [How it works](https://tiny-bubble-company.github.io/shadow/concepts/how-it-works/).

### Intent-aware policy

"Email jane@acme.com about the renewal": with a **local** tool, the model sees a fake address and writes the tool call to the fake; Shadow restores `jane@acme.com` in the tool arguments on your machine before the tool runs. The action works and the provider never sees the address. With a **provider-hosted** tool (hosted MCP, server-side tools) Shadow can't restore values in the provider's cloud, so the target is kept by default and logged.

"Build a churn report from this customer list": **report** intent. Names, emails and phones become typed tokens such as `[PERSON_0001]` that the model treats as opaque IDs, restored in the answer.

```toml
[policy]
intent = true
[policy.act]
targets = "surrogate"          # local tools: mask, restore in the tool call
remote_tool_targets = "keep"   # provider-hosted tools
[policy.report]
person = "token"
email = "token"
default = "surrogate"
[policy.always]
# secret = "redact"            # opt in to irreversible redaction
```

[Read more](https://tiny-bubble-company.github.io/shadow/concepts/intent-policy/)

### Files

Multipart uploads, base64 file payloads in JSON (OpenAI `input_file`, Anthropic `document`, Gemini `inline_data`) and `data:` URIs are masked before they leave. Office files stay valid documents; PDF text is sent masked; images pass through (no OCR yet) or can be blocked. [Read more](https://tiny-bubble-company.github.io/shadow/concepts/file-masking/)

### Presidio

Shadow can call a [Microsoft Presidio](https://microsoft.github.io/presidio/) Analyzer as an extra detector and merge its findings with its own. Presidio detects; Shadow keeps the fakes, vault and persona. It fails open if Presidio is down.

```sh
docker run -d -p 127.0.0.1:5002:3000 mcr.microsoft.com/presidio-analyzer
```

```toml
[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"
```

[Read more](https://tiny-bubble-company.github.io/shadow/concepts/detection/#presidio)

## How Shadow compares

| | Blocking DLP gateway | Redaction only (e.g. Presidio anonymizer) | **Shadow** |
|---|---|---|---|
| Request still goes through | No, blocked | Yes | Yes |
| Model sees coherent data | n/a | No, placeholders | Yes, realistic fakes from one persona |
| Answer refers to your real data | n/a | No | Yes, restored |
| Streaming responses | n/a | Usually not handled | Restored on the fly |
| Agents and tool calls | Blocked | Arguments contain placeholders | Arguments restored locally |
| Runs where | Network or cloud | Library or service | On the endpoint, or in-process |
| Non-AI traffic | Often inspected | n/a | Never decrypted |

## Security model

Shadow protects the **content** of AI requests from the provider, its subprocessors, logs, training sets and breaches. It trusts the endpoint it runs on. It does not hide your IP address or the account behind your API key (use `upstream_proxy` to route AI traffic through a relay). Detection is best effort: add your own terms and test with `shadow scan`. Images are not OCR'd; WebSocket/Realtime APIs and gRPC are not processed; SigV4-signed requests are forwarded unmasked with a warning; certificate-pinning apps reject the local CA in proxy mode.

Read the full [security model](https://tiny-bubble-company.github.io/shadow/reference/security/). Report vulnerabilities privately as described in [SECURITY.md](SECURITY.md).

## Documentation

- [Introduction](https://tiny-bubble-company.github.io/shadow/getting-started/introduction/) · [Installation](https://tiny-bubble-company.github.io/shadow/getting-started/installation/)
- [Proxy mode](https://tiny-bubble-company.github.io/shadow/integrate/proxy-mode/) · [Gateway mode](https://tiny-bubble-company.github.io/shadow/integrate/gateway-mode/) · [Embed in Rust](https://tiny-bubble-company.github.io/shadow/integrate/rust/) · [Docker and Kubernetes](https://tiny-bubble-company.github.io/shadow/integrate/docker-kubernetes/)
- [Configuration](https://tiny-bubble-company.github.io/shadow/reference/configuration/) · [CLI](https://tiny-bubble-company.github.io/shadow/reference/cli/) · [Providers](https://tiny-bubble-company.github.io/shadow/reference/providers/) · [FAQ](https://tiny-bubble-company.github.io/shadow/reference/faq/)

The source of the docs is in [`docs-site/`](docs-site/).

## Roadmap

Release binaries and a published container image, local OCR for images and scanned PDFs, WebSocket/Realtime support, an optional local audit log, more persona regions, and language bindings for `shadow-core`. See the [roadmap](https://tiny-bubble-company.github.io/shadow/project/roadmap/).

## Repository layout

```text
crates/shadow-core    detection, persona, vault, policy, translation
crates/shadow-proxy   CA, forward proxy, gateway, files, SSE translation
crates/shadow-cli     the `shadow` binary
ml/                   optional PyTorch NER sidecar
packaging/            install scripts, launchd / systemd / Windows service, Dockerfile
docs-site/            documentation site (Starlight)
```

## Contributing

Contributions are welcome: detectors, provider formats, docs, tests. See [CONTRIBUTING.md](CONTRIBUTING.md). In short:

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
```

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).

## License

Apache-2.0. See [LICENSE](LICENSE).
