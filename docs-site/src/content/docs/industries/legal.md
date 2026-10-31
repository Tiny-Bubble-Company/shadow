---
title: "Secure ChatGPT and LLM use for law firms: protect client confidentiality with Shadow"
seoTitle: "Secure LLM use for law firms: protect client confidentiality"
description: "Keep client names, matter codenames and counterparties out of AI prompts and uploaded contracts. Shadow masks them locally and restores the answer."
keywords:
  - law firm chatgpt confidentiality
  - attorney client privilege ai
  - aba formal opinion 512
  - legal ai data privacy
  - redact contracts before ai
  - generative ai for lawyers client data
faq:
  - q: "Can lawyers put client information into ChatGPT?"
    a: "ABA Formal Opinion 512 says lawyers must assess the risk of disclosing client information to generative AI tools and, for self-learning tools, generally need informed client consent. Shadow lowers the risk by replacing client names, parties, contacts and matter codenames before the request leaves, but it does not remove the need for that judgement."
  - q: "Does masking with Shadow preserve attorney-client privilege?"
    a: "Privilege is a legal question that depends on jurisdiction and facts, and no tool settles it. Shadow reduces what a third party receives: names and identifiers are replaced, but the substance of the facts and legal questions still reaches the provider."
  - q: "Can Shadow mask a contract I upload to an AI tool?"
    a: "Yes, when the upload goes through an API that Shadow intercepts. DOCX files are masked in place and remain valid Word documents; PDF text is extracted and sent masked. Scanned PDFs are not OCR'd."
  - q: "How do I protect matter numbers and codenames?"
    a: "Add them as dictionary terms in shadow.toml or with shadow terms add. Terms are matched case-insensitively on word boundaries and replaced with consistent fakes. Unlisted matter numbers are not detected by the built-in detectors."
  - q: "Does Shadow work with legal AI products like Harvey or CoCounsel?"
    a: "Only if the product calls a model API from your machine through a host Shadow intercepts, which hosted legal AI products generally do not. Shadow is aimed at API-based tools: SDK apps, internal assistants, agents and bring-your-own-key clients."
sidebar:
  order: 4
---

Shadow is for law firms, in-house legal teams and legal service providers that want to use LLMs for drafting, review and research without sending client identities to the AI provider. It runs on your machines or servers, replaces client and party names, contact details, matter codenames and internal hostnames with consistent fakes, masks uploaded Word documents in place, and restores everything in the answer. It narrows what a third party learns; it does not make disclosure risk disappear.

## What data leaks into AI tools in legal work

| Task | What ends up in the prompt |
|---|---|
| Summarising or redlining a contract | Party names, signatories, addresses, deal values, bank details |
| Drafting a letter before action | Client and opponent names, contact details, factual narrative |
| Preparing a chronology from emails | Every name, email address and phone number in the thread |
| Due diligence on a data room export | Employee schedules, customer lists, IP registers |
| Time narratives and billing | Matter names, client names, codenames for deals |
| Litigation support scripts and tools | Internal hostnames, document IDs, API keys |

Even when a firm has an enterprise AI agreement, fee earners often use whatever tool is at hand. Identifiers are the part of that text a firm most often has a duty to protect.

## How Shadow protects it

| Data | Treatment |
|---|---|
| Client, opponent and counterparty organisations | `organization` for the firm, `org:` terms for clients and counterparties |
| Individuals: clients, witnesses, signatories | Built-in name detector plus `person:` terms; optional NER sidecar or Presidio for recall on free text |
| Matter names and deal codenames | Dictionary `terms`, replaced with consistent invented names |
| Emails, phones, addresses, IBANs | Built-in detectors with format-preserving fakes |
| Contracts and memos | DOCX masked in place (styles and tables survive), PDF text extracted and masked |
| Bulk schedules in due diligence | The `report` intent: typed tokens for people and contact details |

```toml
organization = ["Hartley & Quinn LLP"]
terms = [
  "org:Globex Holdings", "org:Initech Ltd",        # clients and counterparties
  "Project Lighthouse", "term:HQ-2026-0457",       # deal codename and a matter number
  "person:Jane Doe",
]
internal_domains = ["hartleyquinn.example", "dms.hq.internal"]

[policy.always]
secret = "redact"

[files]
pdf = "text"
images = "block"
```

Add new matters as they open:

```sh
shadow terms add "org:Umbrella Pharma" "Project Northgate" "term:HQ-2026-0512"
shadow terms list
```

:::caution[Masking is not anonymity]
Shadow replaces identifiers. The facts, the legal issue and the structure of a deal still reach the provider. A unique transaction described in detail can be recognisable with every name changed. Matter numbers that are not listed as terms are not detected, and dates are not masked.
:::

## Regulations and professional rules

| Rule or framework | How Shadow helps |
|---|---|
| ABA Model Rule 1.6 and Formal Opinion 512 (2024) | Supports "reasonable efforts" to prevent unauthorised disclosure when using generative AI by keeping client identities out of prompts. The opinion's duties of competence, consent assessment, supervision and fee reasonableness still apply |
| Model Rules 5.1 and 5.3 (supervision) | A firm-configured control that applies to every API-based tool on a machine that calls a supported provider API, not just the ones lawyers remember to sanitise |
| GDPR Art. 5(1)(c), 25, 32 | Minimisation and pseudonymisation of personal data in matters before transfer to a processor |
| Outside counsel guidelines and client security questionnaires | A concrete, inspectable answer to "what client data reaches your AI vendors": self-hosted, Apache-2.0, no telemetry |
| ISO/IEC 27001 | Evidence of a technical control on AI data flows |

:::caution[Not legal advice]
This page describes technical behaviour. It is not an ethics opinion. Check your jurisdiction's rules and bar guidance, and client engagement terms.
:::

## Example workflows

### Contract review

An associate uploads `Globex-Initech-SPA.docx` to an internal review tool built on the Anthropic API. Shadow masks the document's text runs inside the DOCX: Globex Holdings and Initech Ltd become persona companies, signatories and notice addresses become fakes, and the escrow IBAN becomes a valid fake IBAN. The review comes back with the real party names.

### Letter before action

Sent (fictional):

```text
Draft a letter before action from Hartley & Quinn LLP for Globex Holdings to
Initech Ltd, attn. Jane Doe (jane.doe@initech.example), re unpaid invoices on Project Lighthouse.
```

Seen by the provider (persona varies):

```text
Draft a letter before action from Zhilan Technology Co., Ltd. for Globex Holdings to
Initech Ltd, attn. Jane Doe (jane.doe@initech.example), re unpaid invoices on Velora Quilent.
```

The letter is addressed to Initech Ltd and Jane Doe, so the recipient phrase (everything after "for" up to "re") passes through as written and each keep is logged: the model writes to the real opponent and does not research a made-up company. Your firm's name and the matter codename are not the recipient, so the provider sees fakes and the draft comes back with Hartley & Quinn LLP and Project Lighthouse restored. If party identities must not reach the provider, set `[policy.act] targets = "surrogate"`; the model then writes to stand-ins that Shadow restores in the draft.

### Chronology from an email export

A paralegal pastes 40 emails and asks for a dated chronology. Names and addresses are replaced consistently across all 40, so the model can still follow who wrote to whom. Dates pass through, which is what a chronology needs, and is also why the text is not anonymous.

## Deployment patterns

| Pattern | Use |
|---|---|
| **Endpoint** (`shadow install --apply`) | Fee earners and knowledge lawyers using API-backed assistants, coding tools for legal engineering, or scripts |
| **Gateway** | Firm-built drafting and review tools: set the SDK base URL to `/gw/anthropic`, `/gw/openai/v1` or an Azure route |
| **Sidecar** | Client-facing portals and document automation in containers |

Consumer web apps such as chatgpt.com and hosted legal AI platforms are not covered by Shadow's built-in host list. Pair Shadow with a policy that directs staff to an approved API-backed client. A firm-wide terms list can be distributed with `shadow.toml`; there is no central console yet (see [roadmap](/shadow/project/roadmap/)).

## Related

- [Consulting and professional services](/shadow/industries/consulting-professional-services/)
- [Redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/), [shadow AI governance](/shadow/use-cases/shadow-ai-governance/)
- [Detection and dictionary terms](/shadow/concepts/detection/#dictionary-terms), [file masking](/shadow/concepts/file-masking/), [security model](/shadow/reference/security/), [compare](/shadow/compare/)
