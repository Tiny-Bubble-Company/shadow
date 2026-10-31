---
title: "Secure LLM use in banking and financial services: mask customer and card data with Shadow"
description: "Keep IBANs, card numbers, customer names and account data out of AI prompts. Shadow masks them locally with checksum-valid fakes and restores the answer."
keywords:
  - chatgpt for banks data privacy
  - pci dss llm
  - mask iban in ai prompts
  - dora ai third party risk
  - glba generative ai
  - financial services llm data protection
faq:
  - q: "Can bank staff use ChatGPT or Claude without exposing customer data?"
    a: "They can reduce exposure substantially by routing API-based AI tools through Shadow, which replaces customer names, emails, IBANs, card numbers and national IDs with fakes before the request leaves. Consumer web apps such as chatgpt.com are not covered, so give staff an approved API-backed client."
  - q: "Does Shadow take an LLM workflow out of PCI DSS scope?"
    a: "It can help keep primary account numbers out of the data sent to an AI provider, because card numbers are detected with a Luhn check and replaced. Scope is decided by your QSA and depends on every system that handles PAN, including the machine running Shadow."
  - q: "Will the model still work with fake IBANs and card numbers?"
    a: "Yes. Fake IBANs pass the mod-97 check and fake card numbers pass Luhn, so validation logic, formatting questions and code that processes them keep working. The real values are restored in the answer."
  - q: "Does Shadow work with AWS Bedrock?"
    a: "With Bedrock API keys (bearer tokens), yes. Requests signed with AWS SigV4 cannot be modified without breaking the signature, so Shadow forwards them unmasked with a warning. Use API-key auth or another route for masked traffic."
  - q: "How does Shadow help with DORA?"
    a: "DORA asks financial entities to manage ICT third-party risk. Shadow is self-hosted software with no telemetry, and it limits what a third-party AI provider receives, which can form part of your risk treatment for AI services. It does not replace contract, register or exit-strategy obligations."
sidebar:
  order: 2
---

Shadow is for banks, payment companies, asset managers, fintechs and credit providers that want to use LLMs on customer-facing and back-office work without handing customer data to the model provider. It runs on your endpoints or next to your services, replaces names, emails, IBANs, card numbers, national IDs and internal hostnames with realistic, checksum-valid fakes, and restores the real values in the answer. It is a control on one data flow, not a compliance programme.

## What data leaks into AI tools in financial services

| Workflow | What ends up in the prompt |
|---|---|
| Drafting replies to complaints and disputes | Customer name, email, phone, account and card numbers, transaction details |
| KYC and onboarding reviews | Passport and ID numbers, addresses, dates of birth, employer names |
| Credit memos and relationship notes | Client company names, counterparties, deal codenames, exposures |
| Fraud and AML investigation summaries | Account numbers, IBANs, IPs, device identifiers, linked parties |
| Analytics on exported customer lists | Hundreds of rows of names, emails and balances |
| Engineering on payment systems | Sample payloads with PANs, internal hostnames, API keys, connection strings |

The data is often regulated several times over: by banking secrecy, by GLBA or GDPR, and, for cards, by PCI DSS.

## How Shadow protects it

| Data | Treatment |
|---|---|
| Card numbers | Detected with a **Luhn** check, replaced by a fake that also passes Luhn |
| IBANs | Detected with the **mod-97** check, replaced by a valid IBAN of the same country |
| Customer names, emails, phones, addresses | Built-in detectors and persona-consistent fakes; known clients as `org:` and `person:` terms |
| SSNs and national IDs | Built in for US SSN and ITIN, UK NINO, Aadhaar and Indian PAN; [Presidio](/shadow/concepts/detection/#presidio) adds `US_BANK_NUMBER`, `US_ITIN`, `US_PASSPORT` and many others |
| Account numbers in free text | Not detected by built-ins. The open-weight NER sidecar labels account numbers, or add known values as terms |
| Deal codenames, counterparties | Dictionary `terms` |
| Bulk customer exports | The `report` intent turns people, emails, phones and addresses into typed tokens |
| Secrets in code and configs | Faked by default; redact with `[policy.always] secret = "redact"` |

```toml
organization = ["Northwind Bank plc"]
terms = ["Project Harbour", "org:Globex Capital", "org:Initech Holdings"]
internal_domains = ["northwind.example", "corp.northwind.internal"]

[model_detector]
enabled = true                      # open-weight NER sidecar on loopback
url = "http://127.0.0.1:8878/detect"

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
national_id = "token"
default = "surrogate"

[policy.always]
secret = "redact"
credit_card = "token"               # never send even a fake PAN-shaped value, if you prefer

[files]
images = "block"
```

Start the sidecar with `python serve_hf.py --backend ensemble --host 127.0.0.1 --port 8878` from the `ml/` directory after `pip install -r requirements-hf.txt`. It combines OpenAI Privacy Filter and GLiNER (both Apache-2.0) and runs locally.

:::caution[Check these before rollout]
- **AWS Bedrock with SigV4** is forwarded unmasked with a warning, because changing the body breaks the signature. Bedrock API keys work normally.
- **Dates of birth and transaction dates** are not masked.
- **Images and scanned statements** are not OCR'd. Block images if they may contain account data.
- **Detection is best effort.** Run `shadow scan` on representative, approved test files, and see [benchmarks](/shadow/benchmarks/).
:::

## Regulations and frameworks

| Framework | How Shadow helps |
|---|---|
| PCI DSS v4.0.1 | Keeps PAN out of what reaches the AI provider, supporting scope reduction and Requirement 3's limits on stored and transmitted PAN. The endpoint running Shadow still sees PAN and stays in scope |
| GLBA Safeguards Rule (16 CFR 314) | A technical safeguard on a data flow to a service provider, under your control |
| DORA (applies since 17 January 2025) | Reduces what a third-party AI provider receives; useful in ICT third-party risk treatment. No telemetry, no hosted component |
| GDPR Art. 5(1)(c), 25, 32 | Data minimisation and pseudonymisation by design before transfer. Pseudonymised data remains personal data for you (Recital 26) |
| EU AI Act | Credit scoring of natural persons is an Annex III high-risk use, with obligations now due from 2 December 2027. Shadow is not a conformity tool, but it limits personal data in prompts during development and use |
| SOC 2, ISO/IEC 27001, ISO/IEC 42001 | Evidence of a technical control for AI data flows |

:::caution[Not legal advice]
This page describes technical capabilities. Shadow does not make you PCI DSS, GLBA, DORA or GDPR compliant, and scope decisions belong to your assessors and counsel.
:::

## Example workflows

### Dispute reply

Sent (fictional data):

```text
Draft a reply to Jane Doe (jane.doe@acme.example) about the disputed charge on card
4111 1111 1111 1111. Refund goes to DE89 3704 0044 0532 0130 00.
```

Seen by the provider:

```text
Draft a reply to Hui Zhao (hui.zhao@zhilan.cn) about the disputed charge on card
4837 2191 9471 3480. Refund goes to DE44 5001 0517 5407 3249 31.
```

The draft comes back addressed to Jane Doe with the real card and IBAN.

### Churn analysis over a customer export

An analyst attaches a CSV of 800 customers and asks for churn drivers by segment. As a `report`, every customer becomes `[PERSON_0001]` and `[EMAIL_0001]`; product names and balances stay readable. The model ranks segments without making up facts about fake people, and the result is restored locally.

### Payment-service debugging

An engineer pastes a failing ISO 20022 payload and a config into Claude Code. IBANs and names become valid fakes, `payments.corp.northwind.internal` becomes a persona hostname, and the database password is redacted.

## Deployment patterns

| Pattern | Use |
|---|---|
| **Endpoint** (`shadow install --apply`) | Analysts, operations and engineers using API-backed assistants and scripts |
| **Gateway** with a custom `azure` route | Internal copilots on Azure OpenAI: `[proxy.routes] azure = "https://<resource>.openai.azure.com"` |
| **Sidecar** with a `NetworkPolicy` | Customer-facing AI features; only the sidecar may reach provider hosts |
| **Rust library** | Payment or risk services written in Rust that call models in-process |

Use `upstream_proxy` if AI egress must go through a corporate proxy. There is no central admin console or SIEM export yet; per-request logs (intent and counts, never values) go to stderr and can be collected by your existing agents. A local audit log is on the [roadmap](/shadow/project/roadmap/).

## Related

- [Insurance](/shadow/industries/insurance/) and [e-commerce and retail](/shadow/industries/ecommerce-retail/)
- [Redact PII before sending to the OpenAI API](/shadow/use-cases/redact-pii-before-sending-to-openai-api/), [LLM data loss prevention](/shadow/use-cases/llm-data-loss-prevention-dlp/)
- [Regulated enterprise](/shadow/company-size/regulated-enterprise/)
- [Intent-aware policy](/shadow/concepts/intent-policy/), [providers](/shadow/reference/providers/), [compare](/shadow/compare/)
