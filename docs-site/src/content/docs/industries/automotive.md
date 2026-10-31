---
title: "Secure LLM use in automotive: protect connected-car, driver and supplier data in AI prompts"
seoTitle: "Secure LLM use in automotive: protect driver data · Shadow"
description: "Mask driver names, contact details, internal hosts, secrets and programme codenames before automotive teams use OpenAI, Anthropic or Gemini. Self-hosted."
keywords:
  - automotive generative ai data protection
  - connected car data llm privacy
  - iso 21434 ai tools
  - unece r155 chatgpt
  - oem supplier ip ai
  - mask vin ai
faq:
  - q: "Does Shadow mask VINs and vehicle identifiers?"
    a: "Not with the built-in detectors, and there is no custom regex setting in shadow.toml. List known VINs or fleet IDs as terms, use the NER sidecar or Presidio for some identifier types, or write a custom detector in Rust if you embed Shadow. Treat VINs in prompts as unmasked unless you have verified otherwise with shadow scan."
  - q: "Can connected-car location data be sent to an LLM through Shadow?"
    a: "Shadow masks street addresses in English and German styles, and the NER sidecar or Presidio finds more locations. GPS coordinates and timestamped trip data are not masked. Location traces are highly identifying, so aggregate or remove them before using an external model."
  - q: "Can engineers use AI coding assistants on ECU or backend code?"
    a: "Yes for API-based assistants such as Claude Code, Codex CLI, Aider, Continue or Cline. Shadow masks internal hostnames, IPs, usernames in paths and recognised secrets in code and logs. The code itself still reaches the provider; whether that is acceptable for safety-critical or supplier-owned code is a policy decision."
  - q: "Does Shadow help with UNECE R155 or ISO/SAE 21434?"
    a: "It can be a documented control in your cybersecurity management system for one risk: sensitive data and infrastructure details sent to AI services. It does not cover vehicle security engineering or type approval."
  - q: "Can we protect supplier and programme names in AI prompts?"
    a: "Yes. Add vehicle programme codenames as terms and suppliers as org: terms. Each gets a consistent fake, so supplier comparisons and programme reviews still make sense to the model, and the answer comes back with real names."
sidebar:
  order: 16
---

Shadow is for vehicle manufacturers, suppliers, mobility and fleet companies whose engineering, after-sales, connected-services and business teams use large language models. It runs on laptops, build servers or pods, replaces driver and customer names, contact details, internal hostnames, secrets, supplier names and programme codenames with consistent fakes before a request reaches the AI provider, and restores them in the answer. Some automotive identifiers, notably VINs and location traces, are not masked, and this page says how to handle them.

## What data leaks into AI tools in automotive

| Workflow | What typically ends up in the prompt |
|---|---|
| Connected-services support and analytics | Driver names, emails, phone numbers, VINs, trip logs, GPS coordinates |
| After-sales and warranty claims | Customer and dealer names, addresses, VINs, fault descriptions |
| Software engineering for ECUs and cloud backends | Source code, build logs, internal hostnames, signing keys, API tokens |
| Supplier management and sourcing | Supplier names, quotes, volumes, programme codenames, contacts |
| Homologation and requirements work | Programme codenames, requirement text, test reports |
| Fleet and mobility operations | Driver records, licence numbers, booking histories |

Two risks overlap: personal data from drivers and customers, and intellectual property from OEMs and suppliers, often covered by strict confidentiality terms.

## How Shadow protects it

Shadow masks values in AI requests and uploaded files with fakes from a persistent [persona](/shadow/concepts/personas/), keeps the mapping in an [encrypted local vault](/shadow/concepts/vault/), and restores responses, including streams and tool-call arguments.

| Data | How Shadow handles it |
|---|---|
| Driver, customer and dealer names | Name detector, `person:` terms, NER sidecar or Presidio |
| Emails, phones, street addresses | Built-in detectors (addresses in English and German styles) |
| Payment details | Card numbers (Luhn) and IBANs (mod-97) with checksum-valid fakes |
| Programme and model codenames | Dictionary `terms` |
| Suppliers, dealers, partners | `org:` terms |
| Backend and build hostnames, IPs, MACs | `hostname`, `ip`, `mac` detectors and `internal_domains` |
| Signing keys, tokens, API keys | `secret` detector: PEM private keys, JWTs, bearer tokens and known key formats; redaction opt-in |
| Usernames in paths, git identity | `path` and `local_facts` detectors |
| Warranty exports | `report` intent with typed tokens |

```toml
organization = ["Nordwerk Fahrzeugbau AG"]
terms = [
  "Project Kestrel", "term:NW-E7 Platform",     # programme codenames
  "org:Globex Electronics", "org:Initech Seating", # suppliers
]
internal_domains = ["nordwerk.example", "ota.nordwerk.internal", "build.nordwerk.internal"]

[policy.always]
secret = "redact"        # signing keys and tokens never leave in any form

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"

[files]
images = "block"
```

:::caution[Known gaps for automotive data]
- **VINs, licence plates and fleet IDs** are not detected by the built-in detectors. There is no custom regex setting; list known values as terms or write a custom Rust detector when embedding Shadow.
- **GPS coordinates and timestamps** are not masked. Dates are never masked.
- **Code and technical content** reach the model as written. Masking hostnames and keys does not hide algorithms or calibration data.
- **Passwords inside connection URLs** are not caught; **images and CAD** are not masked.
:::

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| UNECE R155 (cyber security management system) and R156 (software updates) | A documented control for one data flow: development and operational data sent to AI services. Not a CSMS or SUMS |
| ISO/SAE 21434 | Supports protection of development assets such as keys, internal hosts and codenames in AI-assisted work |
| GDPR Art. 5(1)(c), 25 and 32, and EDPB guidance on connected vehicles | Data minimisation and pseudonymisation for driver data before it reaches an AI vendor. Pseudonymised data is still personal data for you (Recital 26) |
| EU Data Act (connected product data) | Does not address data access or sharing obligations; only limits what reaches AI vendors |
| Supplier NDAs, TISAX | Keeps supplier and programme names out of third-party AI services; evidence of a technical control |
| ISO/IEC 27001, ISO/IEC 42001 | Evidence of a control on AI data flows |

:::caution[Not legal advice]
This page describes technical capabilities, not legal or homologation conclusions. Shadow does not make vehicles or processes compliant with UNECE regulations, ISO/SAE 21434 or GDPR. Work with your cybersecurity, legal and privacy teams.
:::

## Example workflows

### Warranty claim summary

What the after-sales analyst sends (all data fictional):

```text
Summarise claim for Lena Fischer (lena.fischer@mail.example, +49 30 1234567),
Hauptstraße 5, Berlin. Dealer: Initech Seating. Fault: seat heater on NW-E7 Platform.
```

What the provider sees (your fakes will differ):

```text
Summarise claim for Ying Chen (ying.chen@xinghe.cn, +86 28 6512 0417),
Renmin Road 21, Berlin. Dealer: Lumeth Interiors Co., Ltd. Fault: seat heater on Velora Quilent.
```

The summary comes back about Lena Fischer and the NW-E7 Platform. If the claim text contained a VIN, it would have been sent unmasked unless you added it as a term.

### Debugging an OTA backend

An engineer pastes a stack trace from `ota-api-03.ota.nordwerk.internal` and a config with a PEM private key into Claude Code. The hostname becomes a fake, the key becomes `[REDACTED_SECRET]`, and `/Users/lfischer/...` becomes the persona's username. The suggested fix comes back with real hostnames restored.

### Supplier quote comparison

A buyer uploads an XLSX of quotes from six suppliers. Supplier names in shared strings are replaced, prices and volumes are left as numbers, and the comparison comes back with real names.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** on engineering laptops: `shadow install --apply` | API-backed coding assistants, chat clients and scripts |
| **CI runners and build servers** | Systemd unit with gateway URLs or `HTTPS_PROXY`, so AI steps in pipelines are masked |
| **Gateway** for internal tools: custom route to your Azure OpenAI or Bedrock (API-key auth) resource | Warranty, service and requirements assistants. SigV4-signed Bedrock requests are forwarded unmasked, so use API keys |
| **Sidecar** in Kubernetes | Connected-services backends calling LLMs. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |

GitHub Copilot and Cursor's hosted backends are not built-in intercepted hosts. There is no central admin console yet; see the [roadmap](/shadow/project/roadmap/).

## Related

- [Protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/) and [LLM data loss prevention](/shadow/use-cases/llm-data-loss-prevention-dlp/)
- [Manufacturing](/shadow/industries/manufacturing/) and [telecommunications](/shadow/industries/telecommunications/)
- [Detection](/shadow/concepts/detection/), [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/), [security model](/shadow/reference/security/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
