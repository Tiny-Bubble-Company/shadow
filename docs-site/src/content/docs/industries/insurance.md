---
title: "Secure generative AI in insurance: protect policyholder and claims data with Shadow"
seoTitle: "Secure generative AI in insurance: protect claims data"
description: "Use LLMs on claims notes, adjuster reports and policyholder letters without sending personal data to the provider. Shadow masks it locally and restores it."
keywords:
  - insurance generative ai data privacy
  - claims processing llm pii
  - naic ai model bulletin
  - policyholder data chatgpt
  - insurance ai governance
faq:
  - q: "Can claims adjusters use ChatGPT on claim files safely?"
    a: "Adjusters can reduce what leaves by using an API-backed assistant routed through Shadow, which replaces claimant names, contact details, IDs and addresses with fakes and restores them in the answer. Consumer web apps such as chatgpt.com are not covered, and claim and policy numbers need extra configuration to be detected."
  - q: "Does Shadow detect policy and claim numbers?"
    a: "Not with its built-in detectors, and shadow.toml has no custom regex setting. Add known numbers as terms, enable the open-weight NER sidecar, which recognises account-style numbers, or implement a custom detector when embedding Shadow in Rust."
  - q: "What does the NAIC Model Bulletin on AI expect from insurers?"
    a: "The 2023 bulletin, adopted by many states, expects insurers to run a written AI programme with governance, risk management and controls over third-party AI systems and data. Shadow is one possible technical control for data sent to model providers; it does not cover fairness testing or model governance."
  - q: "Is insurance pricing with AI high-risk under the EU AI Act?"
    a: "AI used for risk assessment and pricing of natural persons in life and health insurance is listed as high-risk in Annex III. Under the 2026 Digital Omnibus those obligations apply from 2 December 2027. Shadow helps with data minimisation but is not a conformity tool."
  - q: "Can Shadow mask medical reports attached to a claim?"
    a: "Text-based DOCX, XLSX, PPTX and PDF attachments are masked; PDF text is extracted and sent masked. Scanned reports and photos are images, which Shadow does not OCR, so block images for these workflows."
sidebar:
  order: 3
---

Shadow is for insurers, brokers, MGAs, third-party administrators and claims teams that want LLM help with claims notes, correspondence and analysis without sending policyholder data to the AI provider. It runs on your machines or beside your services, replaces names, contact details, addresses, national IDs, bank details and internal names with realistic fakes, and restores them in the answer. Health information in claims gets the same treatment, with the gaps listed below.

## What data leaks into AI tools in insurance

| Workflow | What ends up in the prompt |
|---|---|
| Summarising a claim file | Claimant and witness names, addresses, phone numbers, injury and medical details, policy and claim numbers |
| Drafting declinature or settlement letters | Policyholder name, address, bank details for payment, broker names |
| Subrogation and fraud review | Third parties, vehicle registrations, IPs, linked claims |
| Underwriting submissions | Commercial client names, locations, loss histories, employee schedules |
| Portfolio and loss analytics | Spreadsheets of policyholders with contact details and claim amounts |
| Building claims automation | Sample FNOL payloads, internal hostnames, API keys |

## How Shadow protects it

| Data | Treatment |
|---|---|
| Claimant, witness, adjuster names | Built-in name detector, `person:` terms, optional NER sidecar or Presidio for free text |
| Emails, phones, street addresses | Built-in detectors, persona-consistent fakes |
| SSN, NINO and other national IDs | Built in for US SSN and ITIN, UK NINO and NHS number, Aadhaar; [Presidio](/shadow/concepts/detection/#presidio) adds driver licence, passport and many national formats |
| IBANs and card numbers for payouts | Checksum-validated detection and checksum-valid fakes |
| Policy and claim numbers | **Not detected by built-ins.** Use terms for known values, the NER sidecar, or a custom Rust detector |
| Brokers, reinsurers, commercial clients | `org:` terms |
| Bordereaux and loss runs | The `report` intent swaps people and contact details for typed tokens |
| Claim attachments | DOCX, XLSX, PPTX masked in place; PDF text extracted and masked; images blocked if you choose |

```toml
organization = ["Harbourline Mutual Insurance Ltd"]
terms = ["org:Globex Re", "org:Brightside Brokers", "term:Operation Tidewater"]
internal_domains = ["harbourline.example", "claims.harbourline.internal"]

[model_detector]
enabled = true                       # open-weight NER: names, addresses, account-style numbers

[presidio]
enabled = true
entities = []                        # all recognizers, incl. US_DRIVER_LICENSE, UK_NINO, US_SSN

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
national_id = "token"
default = "surrogate"

[files]
pdf = "text"
images = "block"
```

:::note[What Shadow does not change]
Shadow masks identifiers, not facts. Injury descriptions, diagnoses and loss narratives reach the model as written, and dates (date of loss, date of birth) are not masked. A distinctive event in a small community can still identify someone. Decide which claim types are suitable for AI assistance at all.
:::

## Regulations and frameworks

| Framework | How Shadow helps |
|---|---|
| NAIC Model Bulletin on the Use of AI Systems by Insurers (2023) | A documented technical control over data sent to third-party AI systems, which the bulletin expects insurers to govern |
| NYDFS Part 500 | Supports limiting nonpublic information sent to third-party service providers; runs on infrastructure you control |
| GLBA and state privacy laws | Reduces nonpublic personal information disclosed to an AI vendor |
| GDPR Art. 5(1)(c), 9, 25, 32 | Minimisation and pseudonymisation of personal and health data before transfer. Still personal data for you (Recital 26) |
| EU AI Act Annex III | Life and health insurance risk assessment and pricing for individuals is high-risk, with obligations from 2 December 2027. Shadow supports data minimisation, not conformity assessment |
| HIPAA (US health plans) | Supports minimum necessary when plans use AI on member data. See [healthcare](/shadow/industries/healthcare/) |

:::caution[Not legal advice]
This page describes what the software does. It is not a compliance determination. Confirm obligations with your compliance team and counsel.
:::

## Example workflows

### Claim summary for a supervisor

Sent (fictional):

```text
Summarise claim notes: insured Tom Becker, 41 Harbour Street, Bristol, +44 117 496 0555,
tom.becker@acme.example. Third party Priya Patel admits fault. Pay to GB82 WEST 1234 5698 7654 32.
```

Seen by the provider (your persona will differ):

```text
Summarise claim notes: insured Lei Li, 17 Jinjiang Street, Bristol, +86 28 6513 2290,
lei.li@zhilan.cn. Third party Yong Xu admits fault. Pay to GB33 BUKB 2020 1555 5555 55.
```

The summary is restored with Tom Becker, Priya Patel and the real account. The liability reasoning is unaffected, because it does not depend on who the people are.

### Loss-run analysis

An underwriter attaches an XLSX of 1,200 claims and asks for a breakdown of the claim records by peril, with frequency and severity. The request reads as a report, so Shadow masks the workbook's shared strings in place: claimant names become typed tokens, amounts and formulas are untouched. The model's tables come back with real names wherever it quotes them.

### Letter drafting with an agent

A claims assistant built on the OpenAI SDK offers a local `send_letter` tool. When the adjuster names the policyholder ("Send the settlement letter to Tom Becker, 9 High Street, Leeds"), that recipient passes through as written and is logged, so the model writes the tool call to the real address. Other people and details in the claim are masked and restored in the tool-call arguments before your code sends the letter. With `[policy.act] targets = "surrogate"`, the recipient is a fake too and Shadow restores the real address the same way. See [tool calls and agents](/shadow/concepts/tool-calls/).

## Deployment patterns

| Pattern | Use |
|---|---|
| **Endpoint** with `shadow install --apply` | Adjusters, underwriters and analysts using API-backed assistants |
| **Gateway** | Claims platforms and internal copilots: point the SDK base URL at `/gw/openai/v1`, `/gw/anthropic` or a custom route |
| **Sidecar** | Customer-facing chat and FNOL automation in Kubernetes, with egress limited to the sidecar |

Shadow has no central console. Roll out `shadow.toml` with your configuration management or MDM, and keep a shared persona across a team by copying `persona.json`, `vault.bin` and `vault.key` (treat the key as a secret). Shared team identities are a [roadmap](/shadow/project/roadmap/) item.

## Related

- [Financial services](/shadow/industries/financial-services/) and [healthcare](/shadow/industries/healthcare/)
- [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/), [redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/)
- [File masking](/shadow/concepts/file-masking/), [intent-aware policy](/shadow/concepts/intent-policy/), [benchmarks](/shadow/benchmarks/), [compare](/shadow/compare/)
