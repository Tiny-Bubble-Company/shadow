---
title: "Secure LLM use in energy and utilities: keep OT details and customer data out of AI prompts"
description: "Mask OT hostnames, IPs, credentials and customer billing data before utility teams use OpenAI, Anthropic or Gemini. Self-hosted, no telemetry, open source."
keywords:
  - utilities ai cybersecurity
  - nerc cip chatgpt
  - critical infrastructure llm security
  - scada config ai assistant
  - energy company generative ai data protection
  - nis2 ai tools energy
faq:
  - q: "Can utility engineers use AI assistants on SCADA or OT configurations?"
    a: "Shadow replaces internal hostnames, IP and MAC addresses and recognised secrets with fakes before a config leaves the machine, so the provider does not learn your addressing scheme or credentials. Device models, protocols and logic remain visible. Whether OT configurations may go to an external AI service at all is a decision for your security and compliance teams."
  - q: "Does Shadow help with NERC CIP-011 BES Cyber System Information?"
    a: "It can reduce the BCSI in an AI request by masking hostnames, IPs and credentials, and it runs entirely on infrastructure you control. It is not a CIP-011 information protection programme and does not decide whether information is BCSI. Treat it as one technical control."
  - q: "Should secrets be faked or redacted?"
    a: "For operational technology, redact. Set [policy.always] secret = \"redact\" and recognised keys, tokens and password assignments leave as [REDACTED_SECRET], which cannot be reversed. The default fake has the same shape as a real secret, which is convenient for code but gives a weaker guarantee."
  - q: "Can Shadow send AI traffic through our approved egress?"
    a: "Yes. Set [proxy] upstream_proxy to an http://, socks5:// or socks5h:// relay and intercepted AI traffic leaves through it. Shadow sends nothing else: no telemetry, no account, no update checks."
  - q: "Does Shadow mask customer billing data?"
    a: "Yes for detected identifiers: names, emails, phone numbers, street addresses in English and German styles, IBANs and card numbers. Meter and account numbers are not detected by the built-in detectors; use the NER sidecar, terms or a custom Rust detector."
sidebar:
  order: 12
---

Shadow is for electricity, gas, water and renewable energy companies whose engineering, operations, customer service and IT teams use large language models. It runs on your own machines, replaces OT hostnames, IP and MAC addresses, credentials, site codenames and customer identifiers with fakes before a request reaches the AI provider, and restores them in the answer. For critical infrastructure, the point is simple: the provider should not learn your network layout, your credentials or your customers.

## What data leaks into AI tools in energy and utilities

| Workflow | What typically ends up in the prompt |
|---|---|
| OT and SCADA troubleshooting | RTU and PLC configs, historian queries, alarm logs, device hostnames, IP ranges, service accounts |
| IT and security operations | Firewall rules, VPN configs, incident timelines, usernames |
| Engineering studies and reports | Substation and plant names, project codenames, contractor names |
| Customer service and billing | Customer names, service addresses, phone numbers, IBANs, payment arrangements, vulnerability flags |
| Regulatory filings and correspondence | Site names, incident details, staff names |
| Field work orders | Addresses, access codes, site contacts |

Several of these sit inside regulated categories, such as BES Cyber System Information in North America, and almost all are useful reconnaissance for an attacker.

## How Shadow protects it

Shadow decrypts only known AI hosts and tunnels every other connection untouched, so it never inspects SCADA, historian or business traffic. For AI requests, it masks values with fakes from a persistent [persona](/shadow/concepts/personas/) and keeps the mapping in an [encrypted local vault](/shadow/concepts/vault/).

| Data | How Shadow handles it |
|---|---|
| OT and IT hostnames | `hostname` detector; `internal_domains` covers every subdomain; `.internal`, `.corp`, `.lan`, `.local` always count as internal |
| IPv4, IPv6, MAC addresses | `ip` and `mac` detectors, replaced with valid-looking addresses |
| API keys, tokens, PEM keys, `password=` assignments | `secret` detector; redact with `[policy.always]` |
| Substation, plant and project names | Dictionary `terms` |
| Contractors and partners | `org:` terms |
| Customer names, emails, phones, addresses | Built-in detectors; `report` intent tokens for bulk exports |
| IBANs and card numbers | Checksum-validated detectors with checksum-valid fakes |

```toml
organization = ["Riverside Energy Ltd"]
terms = [
  "term:Hollow Creek Substation", "term:Project Beacon",
  "org:Contoso Grid Services",
  "hostname:hmi-hc-01", "hostname:hist-core-02",
]
internal_domains = ["riverside.example", "ot.riverside.internal"]

[policy.always]
secret = "redact"          # never send credentials in any form

[proxy]
upstream_proxy = "http://egress.riverside.internal:3128"

[files]
images = "block"           # no OCR: one-line diagrams and HMI screenshots are not masked
```

:::caution[Known gaps for OT and customer data]
- **Passwords inside connection URLs** (`mqtt://ops:pw@host`) are not caught. Use `password=`-style assignments or remove them before sending.
- **Device models, protocols, setpoints and logic are content.** They reach the model as written.
- **Meter, account and asset IDs** are not detected by the built-in detectors, and there is no custom regex setting. List known identifiers as terms or use the NER sidecar.
- **No OCR, no WebSocket or Realtime APIs.**
:::

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| NERC CIP-011 (information protection for BCSI) | Masks hostnames, IPs and credentials in AI requests and runs inside your environment. It does not identify BCSI for you or replace your information protection programme |
| NIS2 (energy, drinking water, wastewater) | Supports supply-chain security and data-handling measures for AI services: infrastructure details stay local |
| TSA security directives, national critical infrastructure guidance | Keeps network details out of third-party services; no telemetry or outbound connections of its own |
| GDPR Art. 5(1)(c), 25 and 32 | Minimises and pseudonymises customer data before it reaches the provider. Pseudonymised data remains personal data for you (Recital 26) |
| ISO/IEC 27001, ISO/IEC 27019, IEC 62443 programmes | A documented technical control for AI data flows |

:::caution[Not legal advice]
This page describes technical capabilities, not legal or regulatory conclusions. Using Shadow does not make an entity compliant with NERC CIP, NIS2 or other requirements. Work with your compliance and security teams, and decide which data classes may reach external AI at all.
:::

## Example workflows

### Explaining an RTU configuration

What the engineer sends (all data fictional):

```text
Why does polling fail? RTU hc-rtu-03.ot.riverside.internal at 172.16.40.12,
master hmi-hc-01, DNP3 outstation 10, token: api_key=rv-ot-88213
```

What the provider sees (your fakes will differ):

```text
Why does polling fail? RTU xinghe-srv-07.xinghe.cn at 172.16.91.204,
master xinghe-srv-11, DNP3 outstation 10, token: [REDACTED_SECRET]
```

The protocol details stay, so the model can still diagnose the problem, and the answer comes back with your real hostnames and IP.

### Customer payment-plan letter

A customer service agent asks to draft a letter to Sarah Murphy at 21 Mill Lane, phone 555-0142, IBAN `DE89 3704 0044 0532 0130 00`, about her new payment plan. Sarah is the recipient, so her name, address, phone and IBAN pass through as written and each keep is logged; the model writes to the right customer. Anything after "about", such as a colleague's name or a meter account holder, is masked and restored in the letter. Set `[policy.act] targets = "surrogate"` to send a stand-in customer and a mod-97-valid fake IBAN instead, restored in the letter.

### Outage report over a work-order export

An analyst pastes a CSV of 900 work orders and asks for a summary. The rows of personal data make it a `report`, so Shadow turns customer names, phones and addresses into typed tokens; site terms become consistent fakes. The summary by cause and region is restored locally.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** on IT and engineering workstations: `shadow install --apply` | API-backed assistants, scripts and chat clients. Keep AI tools off OT-network hosts entirely if policy requires |
| **Gateway** for internal tools, with `upstream_proxy` | Customer service and engineering assistants you build, using approved egress |
| **Sidecar** in Kubernetes | Shared AI services; a `NetworkPolicy` so only the sidecar reaches providers. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |
| **Hardened server install** | Systemd unit with `NoNewPrivileges` and `ProtectSystem=strict`, dedicated user, data directory restricted to it |

Shadow has no central admin console. Distribute configuration with your existing tooling. Logs show intent and counts per request, never values; an optional local audit log is on the [roadmap](/shadow/project/roadmap/).

## Related

- [Protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/) and [self-hosted AI gateway with PII masking](/shadow/use-cases/self-hosted-ai-gateway-with-pii-masking/)
- [Government and public sector](/shadow/industries/government-public-sector/) and [telecommunications](/shadow/industries/telecommunications/)
- [Proxy mode](/shadow/integrate/proxy-mode/), [intent-aware policy](/shadow/concepts/intent-policy/), [security model](/shadow/reference/security/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
