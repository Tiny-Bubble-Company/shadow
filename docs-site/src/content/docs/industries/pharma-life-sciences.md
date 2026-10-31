---
title: "Secure generative AI in pharma and life sciences: protect clinical and R&D data with Shadow"
description: "Mask investigator, patient and partner names, compound codenames and internal systems before AI requests leave. Self-hosted, reversible, no telemetry."
keywords:
  - pharma generative ai data privacy
  - clinical trial data llm
  - gxp ai tools
  - protect drug codenames ai
  - life sciences chatgpt confidentiality
  - pharmacovigilance llm pii
faq:
  - q: "Can we use LLMs on clinical trial documents without exposing subject data?"
    a: "Shadow reduces exposure by replacing names, contact details, investigator names, sites and known codenames before text reaches the provider. Trial subject IDs and dates are not detected by default, so add known IDs as terms or a detector that finds them, and review what remains with shadow scan."
  - q: "How do we protect compound and programme codenames?"
    a: "List them as dictionary terms in shadow.toml or with shadow terms add. Each codename gets a consistent invented name, so the model can still reason about it, and the real codename is restored in the answer."
  - q: "Is Shadow GxP validated?"
    a: "No. Shadow is open-source software without a vendor validation package. If an AI-assisted process falls under GxP, validate the process, including Shadow's configuration, under your computer system validation approach."
  - q: "Can Shadow handle pharmacovigilance case narratives?"
    a: "It masks reporter and patient names, contact details, addresses and national IDs in narratives. Ages, dates, drugs and adverse events are content and are not masked, which keeps the narrative useful but means the text is not anonymised."
  - q: "Does Shadow mask data in protocol PDFs and Excel listings?"
    a: "Yes for text. XLSX shared strings, DOCX and PPTX text runs are masked in place, and PDF text is extracted and sent masked. Scanned pages and images are not OCR'd."
sidebar:
  order: 7
---

Shadow is for pharmaceutical, biotech, CRO and medical device teams that want to use LLMs on clinical, regulatory and research documents without handing confidential identifiers to the model provider. It runs on your machines or beside your services, replaces patient, investigator and partner names, contact details, compound and programme codenames and internal hostnames with consistent fakes, and restores them in the answer. Clinical facts are not masked, so plan which workflows are suitable.

## What data leaks into AI tools in pharma and life sciences

| Workflow | What ends up in the prompt |
|---|---|
| Drafting clinical study reports and narratives | Subject IDs, ages, dates, adverse events, investigator and site names |
| Pharmacovigilance case intake | Reporter and patient names, phone numbers, addresses, free-text narratives |
| Medical information replies | Healthcare professional names, emails, institutions |
| Competitive and portfolio analysis | Compound codenames, licensing partners, programme names |
| Regulatory submissions | Draft module text, internal document IDs, unreleased results |
| Lab and data engineering | ELN exports, LIMS hostnames, service account keys |

Two kinds of risk mix here: personal data of patients and HCPs, and trade secrets about pipeline and partners.

## How Shadow protects it

| Data | Treatment |
|---|---|
| Patient, reporter, investigator names | Built-in name detector, `person:` terms, optional NER sidecar or Presidio |
| Emails, phones, addresses | Built-in detectors, persona-consistent fakes |
| Compound codes, programme names, partners | Dictionary `terms` and `org:` terms |
| Sites and institutions | `org:` terms for known sites |
| Trial subject IDs, document IDs | **Not detected by built-ins.** Terms for known values, the NER sidecar for account-style numbers, or a custom Rust detector |
| Internal hostnames, keys | `internal_domains`, `hostname` and `secret` detectors |
| Listings and line listings in XLSX | Masked in place; with the `report` intent, people become typed tokens |

```toml
organization = ["Helix Therapeutics AG"]
terms = [
  "HLX-4417", "HLX-5520",                    # compound codes
  "Project Aurora",                          # programme codename
  "org:Globex Biologics",                    # licensing partner
  "org:St Mary's Research Hospital",         # trial site
]
internal_domains = ["helix.example", "lims.helix.internal"]

[model_detector]
enabled = true

[policy.always]
secret = "redact"

[files]
pdf = "text"
images = "block"
```

Add new codenames as programmes start:

```sh
shadow terms add "HLX-6031" "Project Borealis"
```

:::caution[What stays visible]
Ages, dates, doses, diagnoses and adverse events are not masked. Rare-disease trials with few subjects can be identifying through context alone. Dates are not detected, so a narrative with an event date, a site and a rare indication is not anonymous even after masking.
:::

## Regulations and frameworks

| Framework | How Shadow helps |
|---|---|
| GDPR Art. 9 (health data), 5(1)(c), 25, 32 | Pseudonymises identifiers before data reaches a processor. Key-coded trial data is already pseudonymised; Shadow keeps additional direct identifiers out of AI requests. Still personal data for you (Recital 26) |
| HIPAA (US covered entities and business associates) | Supports minimum necessary when PHI enters AI workflows; see [healthcare](/shadow/industries/healthcare/) |
| ICH E6(R3) Good Clinical Practice | Supports confidentiality of trial subjects when AI tools are used on trial records |
| 21 CFR Part 11 and EU GMP Annex 11 | Shadow is not a validated system. If AI output feeds a GxP record, validate the process and configuration, and keep the human review step |
| Trade secret protection | Codenames and partners as terms reduce what an AI provider learns about your pipeline |
| ISO/IEC 27001, ISO/IEC 42001 | Evidence of a technical control on AI data flows |

:::caution[Not legal advice]
This page describes technical capabilities. It is not a regulatory or validation opinion. Involve your quality, privacy and legal functions.
:::

## Example workflows

### Medical information reply

Sent (fictional):

```text
Draft a reply to Dr Amara Okafor (a.okafor@stmarys.example) at St Mary's Research Hospital
asking whether HLX-4417 can be co-administered with warfarin.
```

Seen by the provider (persona varies):

```text
Draft a reply to Dr Amara Okafor (a.okafor@stmarys.example) at St Mary's Research Hospital
asking whether Velora Quilent can be co-administered with warfarin.
```

Dr Okafor, her email and her hospital are the recipient, so they pass through as written and each keep is logged; the model writes to the right physician. The unreleased compound is not, so the provider sees a codename and the reply comes back about HLX-4417. Warfarin stays as written because it is public. Set `[policy.act] targets = "surrogate"` if the enquirer's identity must stay hidden from the provider.

### Partnering memo

A business development lead asks for a summary of term-sheet differences between two DOCX drafts. Partner names, the programme codename and signatories are masked inside both documents consistently, so the comparison still lines up.

### PV case triage tool

A triage service built on the OpenAI SDK sends case narratives through the Shadow gateway. Reporter names, phones and addresses are faked; tool calls that file the case into the safety database are restored to real values before your code runs them. See [tool calls](/shadow/concepts/tool-calls/).

## Deployment patterns

| Pattern | Use |
|---|---|
| **Endpoint** (`shadow install --apply`) | Medical writers, regulatory and BD staff using approved API-backed assistants |
| **Gateway** with a custom route | Internal copilots on Azure OpenAI or a private endpoint: `[proxy.routes] azure = "https://<resource>.openai.azure.com"` |
| **Sidecar** | PV, medical information and data platforms in Kubernetes |
| **Rust library** | Data pipelines written in Rust that call models in-process |

Keep a shared terms list for codenames under change control and distribute it with `shadow.toml`. There is no central console yet; an optional local audit log is on the [roadmap](/shadow/project/roadmap/).

## Related

- [Healthcare](/shadow/industries/healthcare/), [manufacturing](/shadow/industries/manufacturing/)
- [Redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/), [protect customer data in RAG pipelines](/shadow/use-cases/protect-customer-data-in-rag-pipelines/)
- [Detection](/shadow/concepts/detection/), [file masking](/shadow/concepts/file-masking/), [benchmarks](/shadow/benchmarks/), [compare](/shadow/compare/)
