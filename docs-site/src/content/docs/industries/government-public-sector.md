---
title: "Secure generative AI in government and the public sector: mask citizen data with Shadow"
description: "Self-hosted, no-telemetry masking of citizen and case data before it reaches an AI provider. Runs inside your boundary; restores answers locally."
keywords:
  - government generative ai data protection
  - public sector llm privacy
  - fedramp ai tools
  - citizen data chatgpt
  - data residency llm
  - cjis ai
faq:
  - q: "Is Shadow FedRAMP authorised?"
    a: "FedRAMP authorises cloud services, and Shadow is not one. It is open-source software you run on your own endpoints, servers or clusters, with no hosted component or telemetry. The AI provider you send masked requests to is a cloud service and still needs whatever authorisation your agency requires."
  - q: "Does Shadow solve data residency for AI?"
    a: "Partly. Detection, masking and the vault all stay on your infrastructure, so real identifiers never leave it. The masked text still goes to the provider's region, so residency requirements for the remaining content, and for metadata, still apply to the provider."
  - q: "Can Shadow run in an air-gapped or restricted network?"
    a: "Shadow itself needs no internet access: it builds from source, has no licence server and sends nothing except the masked requests you route through it. It is useful in restricted networks that allow egress only to an approved model endpoint, which you can add with a custom gateway route or extra_hosts."
  - q: "Can caseworkers use ChatGPT with Shadow?"
    a: "Not the consumer chatgpt.com web app, which is not a built-in intercepted host. Shadow covers API-based tools: internal assistants, SDK apps, agents and approved clients that call Azure OpenAI, OpenAI, Anthropic, Gemini or a self-hosted endpoint."
  - q: "Is masked data safe to send for criminal justice information?"
    a: "CJIS Security Policy governs criminal justice information and the systems that handle it. Shadow can reduce identifiers in prompts, but it does not determine whether a workflow is permitted. Treat that as a decision for your CJIS security officer."
sidebar:
  order: 5
---

Shadow is for government departments, agencies, local authorities and public bodies that want to use LLMs on casework, correspondence and policy work without sending citizen identities to a commercial AI provider. It is open-source software that runs inside your own boundary, on endpoints, servers or Kubernetes, with no hosted component and no telemetry. It replaces names, contact details, national IDs and internal hostnames with realistic fakes before a request leaves and restores them locally in the answer.

## What data leaks into AI tools in the public sector

| Workflow | What ends up in the prompt |
|---|---|
| Summarising case files and benefit claims | Citizen names, addresses, national insurance or social security numbers, family members |
| Drafting replies to correspondence and FOI requests | Requester names, emails, phone numbers, officials' names |
| Policy analysis on consultation responses | Thousands of responses with names and email addresses |
| Inspection and enforcement reports | Business names, owners, site addresses |
| Digital service development | Sample records, internal hostnames, IP ranges, API keys |
| Procurement | Supplier names, contract values, evaluation notes |

## How Shadow protects it

| Data | Treatment |
|---|---|
| Citizen names, emails, phones, addresses | Built-in detectors with persona-consistent fakes |
| SSN, ITIN, UK NINO and NHS number, Aadhaar, PAN, Australian TFN and ABN | Built in; [Presidio](/shadow/concepts/detection/#presidio) adds passports, driver licences and many national IDs |
| Case and reference numbers | Not detected by built-ins. Known values as terms; the NER sidecar for account-style numbers; a custom Rust detector when embedding |
| Department, programme and system names | `organization` and dictionary `terms` |
| Internal hostnames and IPs | `internal_domains`, plus internal TLDs such as `.internal`, `.corp`, `.local` |
| Bulk consultation data | The `report` intent: typed tokens for people and contact details |
| Request metadata | OpenAI `user`, Anthropic `metadata.user_id`, org and OS headers replaced or stripped |

```toml
organization = ["Department of Civic Services"]
terms = ["Programme Meridale", "term:Case Hub", "org:Northshire Council"]
internal_domains = ["civic.gov.example", "corp.civic.internal"]

[persona]
region = "AU"          # pick a region that keeps answers relevant; see Personas

[policy.always]
secret = "redact"
national_id = "token"

[files]
pdf = "text"
images = "block"

[proxy]
upstream_proxy = "http://egress.corp.civic.internal:3128"   # route AI egress via your gateway

[proxy.routes]
llm = "https://llm.civic.gov.example"                        # a self-hosted or sovereign endpoint
```

The persona's region affects answers about holidays, phone formats and legal context. Pin a region that suits your work, as described in [Personas](/shadow/concepts/personas/).

### Data residency, in practice

| Stays on your infrastructure | Still reaches the provider |
|---|---|
| Real identifiers, the vault and its key, the persona, logs | The masked prompt and its non-identifying content |
| Detection, including the optional NER and Presidio sidecars on loopback | Your API key (identifying the account) and your egress IP, unless you use a relay |
| Restored answers | Any value a detector missed |

For the strongest position, combine Shadow with a provider region or sovereign endpoint that meets your residency rules, or a self-hosted model added through `[proxy.routes]` or `extra_hosts`.

## Regulations and frameworks

| Framework | How Shadow helps |
|---|---|
| FedRAMP | Not applicable to Shadow itself, which is software you operate, not a cloud service. It runs inside your authorised boundary and reduces what reaches an authorised (or unauthorised) AI service |
| CJIS Security Policy | Reduces criminal justice identifiers in prompts; it does not make an unapproved service acceptable for CJI |
| GDPR / UK GDPR Art. 5(1)(c), 25, 32, and Chapter V transfers | Minimisation and pseudonymisation before data reaches a processor, which also reduces what is transferred abroad. Pseudonymised data is still personal data for you (Recital 26) |
| EU AI Act | Public-sector uses such as eligibility for essential public benefits are Annex III high-risk, with obligations now due from 2 December 2027. Shadow supports data minimisation only |
| NIS2, ISO/IEC 27001 | A local technical control with a documented [security model](/shadow/reference/security/), memory-safe implementation and open source for review |

:::caution[Not legal advice]
This page describes technical capabilities, not an authorisation decision. Your accreditation, security and data protection officers decide what is permitted.
:::

## Example workflows

### Correspondence reply

Sent (fictional):

```text
Draft a reply to Sam Okoro, 7 Mill Lane, Northshire, sam.okoro@mail.example,
NINO AB 12 34 56 C, about the delayed housing benefit decision.
```

Seen by the provider (persona varies):

```text
Draft a reply to Liam Walker, 52 Banksia Street, Northshire, liam.walker@coralbay.com.au,
NINO JK 48 21 77 B, about the delayed housing benefit decision.
```

The draft is restored with Sam Okoro's real details before an officer reviews it.

### Consultation analysis

A policy team attaches a CSV of 5,000 responses and asks for themes and counts. Respondent names and emails become typed tokens, the free text is masked by the detectors, and the analysis comes back with real names where quotes are attributed.

### Service team debugging

A developer on a benefits platform uses Claude Code through Shadow. Internal hostnames under `corp.civic.internal`, home-directory usernames and secrets in configs are replaced before the code reaches the provider.

## Deployment patterns

| Pattern | Use |
|---|---|
| **Endpoint** on managed devices | Policy and casework staff using approved API-backed clients; distribute `shadow.toml` and run `shadow install --apply` with your device management |
| **Gateway** on a hardened VM | Shared internal assistants: systemd unit with a dedicated user, gateway on loopback, app on the same host |
| **Sidecar** in Kubernetes | Citizen-facing services; `NetworkPolicy` so only the sidecar can reach the model endpoint |
| **Restricted network** | Build from source (`cargo build --release -p shadow-cli`) on an approved build host; no runtime downloads |

Signed release binaries and a published container image are on the [roadmap](/shadow/project/roadmap/); today the install script falls back to building with `cargo`, and the container image is built with `docker build -f packaging/Dockerfile`. There is no central admin console or SIEM export yet.

## Related

- [Regulated enterprise](/shadow/company-size/regulated-enterprise/), [education](/shadow/industries/education/)
- [Self-hosted AI gateway with PII masking](/shadow/use-cases/self-hosted-ai-gateway-with-pii-masking/), [GDPR-compliant LLM usage](/shadow/use-cases/gdpr-compliant-llm-usage/)
- [Integrating any endpoint](/shadow/integrate/any-endpoint/), [proxy mode](/shadow/integrate/proxy-mode/), [benchmarks](/shadow/benchmarks/), [compare](/shadow/compare/)
