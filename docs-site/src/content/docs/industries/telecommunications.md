---
title: "Secure LLM use in telecommunications: keep subscriber data and network details out of AI prompts"
description: "Mask subscriber names, phone numbers, IPs, MACs and internal hostnames before telecom teams send prompts to OpenAI, Anthropic or Gemini. Self-hosted."
keywords:
  - telecom ai privacy
  - cpni chatgpt
  - mask subscriber data llm
  - network config ai assistant security
  - telco generative ai data protection
  - nis2 ai tools
faq:
  - q: "Can telecom staff use LLMs on subscriber records without exposing CPNI?"
    a: "Shadow reduces what reaches the provider: subscriber names, phone numbers, emails, addresses and account-style identifiers it detects are replaced with fakes before the request leaves. Call detail records and usage patterns are content, not identifiers, so they still reach the model. Decide which CPNI workflows are acceptable at all, and use Shadow as a safeguard on the ones you allow."
  - q: "Does Shadow detect IMSI, IMEI or ICCID numbers?"
    a: "Not with the built-in detectors. There is no custom regex setting in shadow.toml today. Use the open-weight NER sidecar or Presidio for account-style numbers, add known values as terms, or write a custom detector in Rust if you embed Shadow."
  - q: "Is it safe to paste router and firewall configs into an AI assistant through Shadow?"
    a: "Shadow replaces IP addresses, MAC addresses, internal hostnames and recognised secrets with format-preserving fakes, so the config still makes sense to the model. Topology and vendor details are not masked, and passwords embedded in connection URLs are not caught, so review configs before sending and turn on secret redaction."
  - q: "Will masking phone numbers break troubleshooting answers?"
    a: "Usually not. Fakes keep the format of the original, so a phone number stays a valid-looking number from the persona's region and an IP stays an IP. If the model's answer depends on the real number plan or country, keep that detail out of the prompt or pin the persona region."
  - q: "Does Shadow work for voice AI in the contact centre?"
    a: "Not today. WebSocket and Realtime APIs, which most voice products use, are passed through unprocessed. Shadow covers text APIs, including streaming responses and tool calls."
sidebar:
  order: 10
---

Shadow is for telecom operators, MVNOs, ISPs and network service providers whose care, network operations and engineering teams use large language models. It runs on your own laptops, servers or pods, replaces subscriber names, phone numbers, emails, IP and MAC addresses, internal hostnames and secrets with realistic fakes before a request reaches the AI provider, and restores the real values in the answer. It is a technical safeguard on AI data flows, not a CPNI compliance programme.

## What data leaks into AI tools in telecommunications

Telecoms hold two kinds of sensitive data that both end up in prompts: subscriber data and network data.

| Workflow | What typically ends up in the prompt |
|---|---|
| Customer care: drafting replies, summarising complaints | Subscriber name, MSISDN, email, billing address, account number, plan details |
| Retention and billing analysis | Exported spreadsheets of subscribers, usage and churn flags |
| NOC and incident troubleshooting | Router and switch configs, syslog lines, IP ranges, MAC addresses, device hostnames |
| Provisioning and OSS/BSS engineering | API payloads with subscriber identifiers, internal service hostnames, API keys |
| Fraud and abuse investigation | Call patterns, numbers involved, SIM swap histories |
| Field service | Customer addresses, site names, access notes |

Network data is a security problem as much as a privacy one: an internal hostname scheme, management IP ranges and device credentials are useful to an attacker.

## How Shadow protects it

Shadow intercepts AI API traffic, detects sensitive values in every JSON string and uploaded file, swaps them for fakes from a persistent [persona](/shadow/concepts/personas/), keeps the mapping in an [encrypted local vault](/shadow/concepts/vault/), and translates responses back, including streams and tool calls.

| Data | How Shadow handles it |
|---|---|
| Subscriber names, emails, addresses | Built-in detectors; the open-weight NER sidecar or Presidio improves recall on free text |
| Phone numbers (MSISDNs) | `phone` detector for international (`+44 20 7946 0000`) and US formats; replaced with numbers in the persona's region |
| IPv4, IPv6, MAC addresses | `ip` and `mac` detectors, replaced with valid-looking addresses |
| Device and service hostnames | `hostname` detector plus `internal_domains`, which covers every subdomain |
| API keys, tokens, `password=` assignments | `secret` detector; opt in to irreversible redaction |
| Account numbers | Not built in; the NER sidecar maps account-style numbers to `national_id` |
| Bulk subscriber exports | The `report` intent turns people, emails, phones and addresses into typed tokens such as `[PHONE_0012]` |
| Partner and wholesale carrier names | `org:` terms |

A starting configuration for a network operator:

```toml
organization = ["Northwind Telecom plc"]
terms = ["org:Contoso Wholesale", "term:Project Lighthouse", "hostname:core-rtr-lon-01"]
internal_domains = ["northwind.example", "noc.northwind.internal", "oss.northwind.internal"]

[model_detector]
enabled = true                       # ml/serve_hf.py: account-style numbers, free-text names
url = "http://127.0.0.1:8878/detect"

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"

[policy.always]
secret = "redact"

[files]
images = "block"                     # no OCR: network diagrams and screenshots are not masked
```

:::caution[Known gaps for telecom data]
- **IMSI, IMEI and ICCID are not detected** by the built-in detectors, and there is no custom regex setting in `shadow.toml`. Add known values as terms, rely on the NER sidecar for account-style numbers, or write a custom Rust detector when embedding Shadow.
- **Passwords inside connection URLs** (`ssh://admin:pw@host`) are not caught. The `secret` detector finds known key formats, JWTs, PEM keys, bearer tokens and `password=`-style assignments.
- **Topology and usage patterns are content.** Masking identifiers does not hide that a cell site failed at 02:14 or that a number made 400 calls.
- **No voice.** WebSocket and Realtime APIs are not processed.
:::

Run `shadow scan` on a sanitised sample config and a sample ticket export before rollout to see what is found.

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| US CPNI rules (47 CFR 64.2001 and following) | Reduces subscriber identifiers in AI requests, so fewer records are tied to a person if a vendor is breached. It does not decide whether a use of CPNI is permitted |
| GDPR Art. 5(1)(c), 25 and 32 | Data minimisation and pseudonymisation by design before data leaves your infrastructure. Pseudonymised data is still personal data for you, because you hold the key (Recital 26) |
| EU ePrivacy rules on traffic and location data | Masks identifiers attached to traffic data; the traffic data itself still needs a lawful basis to be processed by AI at all |
| NIS2 (telecom and digital infrastructure) | Supports supply-chain and data-handling measures for AI vendors: internal hostnames, IPs and secrets stay local |
| ISO/IEC 27001, ISO/IEC 42001 | A documented technical control for AI data flows in your ISMS or AI management system |

:::caution[Not legal advice]
This page describes technical capabilities, not legal conclusions. Using Shadow does not make an operator compliant with CPNI, GDPR, ePrivacy or NIS2 obligations. Work with your privacy, security and regulatory teams.
:::

## Example workflows

### Drafting a care reply

What the agent sends (all data fictional):

```text
Draft an apology to Priya Shah (+44 7700 900123, priya.shah@mail.example), account
holder at 14 Kingsway, Manchester, for the outage on her fibre line last weekend.
```

What the provider sees with this machine's persona (your fakes will differ):

```text
Draft an apology to Hui Zhao (+86 138 0571 4492, hui.zhao@zhilan.cn), account
holder at 88 Tianfu Road, Manchester, for the outage on her fibre line last weekend.
```

The draft comes back addressed to Priya Shah with the real number.

### NOC troubleshooting with a config

An engineer pastes a BGP neighbour config and log lines into a terminal assistant:

```text
router bgp 64512
 neighbor 10.20.30.1 remote-as 64513
 neighbor 10.20.30.1 description core-rtr-lon-01.noc.northwind.internal
 snmp-server community api_key=Nw-Prod-7781 RO
```

The provider sees different private IPs, a fake hostname under the persona's domain, and `[REDACTED_SECRET]` for the assignment. The AS numbers and the BGP logic are unchanged, which is what the model needs. The explanation comes back referring to your real neighbour and hostname.

### Churn analysis over an export

An analyst pastes a CSV of 1,200 subscribers and asks for churn drivers by plan. Shadow classifies this as `report`, so subscriber names, numbers and emails become `[PERSON_0001]`, `[PHONE_0001]` and so on. Plan names and usage figures are left alone so the analysis still works. The answer is restored locally.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** for NOC engineers and analysts: `shadow install --apply` | API-backed chat clients, terminal assistants and scripts that honour `HTTPS_PROXY` |
| **Gateway** for internal care and OSS tools: `http://127.0.0.1:8877/gw/openai/v1`, or a custom `azure` route | Agent-assist, ticket summarisation and knowledge tools you build |
| **Sidecar** in Kubernetes | Customer-facing chatbots and BSS integrations; egress to providers only from the sidecar. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |

Shadow has no central console or fleet management today. Roll it out with your existing configuration management, and use `shadow status --quiet` as a health check. Each request logs its intent and counts per treatment, never values; a local append-only audit log is on the [roadmap](/shadow/project/roadmap/).

## Related

- [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/) and [protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/)
- [Energy and utilities](/shadow/industries/energy-utilities/) and [SaaS and technology](/shadow/industries/saas-technology/)
- [Detection](/shadow/concepts/detection/), [intent-aware policy](/shadow/concepts/intent-policy/), [security model](/shadow/reference/security/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
