---
title: "Secure ChatGPT and LLM use in healthcare: protect PHI with Shadow"
description: "Keep patient names, contact details and identifiers out of OpenAI, Anthropic and Gemini prompts. Shadow masks PHI locally and restores it in the answer."
keywords:
  - hipaa chatgpt
  - phi in llm prompts
  - healthcare ai privacy
  - hipaa compliant llm
  - de-identify clinical notes for ai
  - mask patient data openai
faq:
  - q: "Does Shadow make ChatGPT or the OpenAI API HIPAA compliant?"
    a: "No tool makes you compliant on its own. Shadow is a technical safeguard that keeps detected identifiers out of AI requests, which supports the minimum necessary standard. You still need a risk analysis, policies and, where PHI may reach a vendor, a business associate agreement."
  - q: "Is text masked by Shadow de-identified under HIPAA Safe Harbor?"
    a: "Not by itself. Safe Harbor requires removing all 18 identifier types, including dates and record numbers. Shadow does not mask dates, and it does not detect medical record numbers unless you add them or a detector that finds them, so treat masked text as reduced-risk PHI rather than de-identified data."
  - q: "Can clinicians use Shadow with the ChatGPT website?"
    a: "Not as a built-in integration. Shadow intercepts provider APIs such as api.openai.com and Azure OpenAI, not consumer web apps like chatgpt.com. Give clinicians an approved API-backed client or internal tool and route it through Shadow."
  - q: "Where does patient data go when Shadow masks it?"
    a: "The real values stay in an encrypted vault on the machine or pod running Shadow. The provider receives realistic fakes, and the answer is translated back locally before anyone sees it."
  - q: "Can Shadow mask patient data in uploaded PDFs and Word documents?"
    a: "Yes for text. DOCX, XLSX and PPTX are masked in place and PDF text is extracted and sent masked. Scanned PDFs and images are not OCR'd, so block images if they may contain PHI."
sidebar:
  order: 1
---

Shadow is for hospitals, clinics, health tech vendors and digital health teams that want to use large language models without sending protected health information (PHI) to the model provider. It runs on your own laptops, servers or pods, replaces patient names, emails, phone numbers, addresses and national IDs with realistic fakes before a request leaves, and restores the real values in the response. It is a technical safeguard that reduces what reaches the provider; it does not replace your HIPAA programme or a business associate agreement.

## What data leaks into AI tools in healthcare

Clinical and administrative staff use LLMs for the same reason everyone else does: they save time on writing. The text they paste is rarely clean.

| Workflow | What typically ends up in the prompt |
|---|---|
| Summarising a discharge letter or referral | Patient name, date of birth, address, NHS or Medicare number, GP name, ward |
| Drafting a reply to a patient complaint | Patient and family names, email, phone number, clinician names |
| Ambient scribe or note-cleanup tools | Full consultation transcript: symptoms, medications, family history, names |
| Coding and billing support | Encounter details, insurer member IDs, claim references |
| Analytics on exported reports | Spreadsheets with hundreds of patient rows |
| Engineering on EHR integrations | HL7 or FHIR samples with real identifiers, internal hostnames, API keys |

Each of these sends PHI to a third party, often outside any agreement your compliance team has reviewed.

## How Shadow protects it

Shadow sits between the tool and the provider. For every outbound request it detects sensitive values, swaps them for format-preserving fakes from a persistent [persona](/shadow/concepts/personas/), keeps the mapping in an [encrypted local vault](/shadow/concepts/vault/), and translates the answer back.

| PHI element | How Shadow handles it |
|---|---|
| Patient, family and clinician names | Built-in name detector; add known names with `person:` terms; the optional NER sidecar or Presidio improves recall on free text |
| Emails, phone numbers, street addresses | Built-in detectors, replaced with persona-consistent fakes |
| SSN, UK NHS number, Medicare and similar IDs | US SSN and UK NHS number (checksum-validated) are built in; enable [Presidio](/shadow/concepts/detection/#presidio) for `AU_MEDICARE`, `US_MBI`, `MEDICAL_LICENSE` and more |
| Hospital, practice and partner names | `organization` and `org:` terms |
| Internal hostnames, IPs, API keys | `hostname`, `ip` and `secret` detectors, plus `internal_domains` |
| Bulk patient lists in reports | The `report` intent swaps people, emails, phones and addresses for typed tokens such as `[PERSON_0004]` |
| Uploaded DOCX, XLSX, PPTX, PDF text | [File masking](/shadow/concepts/file-masking/) |

A starting configuration for a health system:

```toml
organization = ["St Mary's Health Partners"]
terms = ["org:Northside Imaging", "term:Ward 7B", "person:Dr Amara Okafor"]
internal_domains = ["stmarys.example", "ehr.stmarys.internal"]

[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"
entities = []          # all Presidio recognizers, including AU_MEDICARE, US_MBI and MEDICAL_LICENSE

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
national_id = "token"
default = "surrogate"

[policy.always]
secret = "redact"

[files]
pdf = "text"
images = "block"       # no OCR yet: stop screenshots and scans from leaving unmasked
```

:::caution[Known gaps for PHI]
- **Dates are not masked.** Dates of birth, admission and discharge dates pass through. Remove or generalise them before sending, or keep them out of the workflow.
- **Medical record numbers are not detected by the built-in detectors.** There is no custom regex setting in `shadow.toml`. Add known values as terms, enable the open-weight NER sidecar (which finds account-style numbers), or write a custom detector in Rust if you embed Shadow.
- **No OCR.** Scanned PDFs and images are not masked.
- **Context can identify.** A rare condition in a small town can identify a patient even with every name replaced.
:::

Run `shadow scan` on representative, already approved test documents to see what is found before relying on it.

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| HIPAA minimum necessary (45 CFR 164.502(b)) | Limits the identifiers that reach an AI vendor to what the task needs: usually none |
| HIPAA Security Rule safeguards | A technical control on a known data flow, running on infrastructure you control, with no telemetry |
| HIPAA de-identification (45 CFR 164.514) | Removes many of the 18 Safe Harbor identifiers, but not dates or unrecognised record numbers. Not a de-identification method on its own; an expert determination would need to assess it |
| GDPR Art. 9 health data, Art. 25 and 32 | Pseudonymisation by design before transfer. Pseudonymised data is still personal data for you, because you hold the key (Recital 26) |
| ISO/IEC 27001, ISO/IEC 42001 | Evidence of a technical control for AI data flows in your ISMS or AI management system |

Where PHI may still reach a vendor (missed detections, dates, free text), keep a business associate agreement with that vendor. Shadow reduces exposure; it does not change who is a business associate.

:::caution[Not legal advice]
This page describes technical capabilities, not legal conclusions. Using Shadow does not make an organisation HIPAA or GDPR compliant. Work with your privacy officer and counsel.
:::

## Example workflows

### Summarising a discharge letter

What the clinician sends (all data fictional):

```text
Summarise for the GP: Maria Lopez (NHS 943 476 5919), 12 Elm Road, Leeds,
phone +44 113 496 0123, admitted under Dr Amara Okafor for community-acquired pneumonia.
```

What the provider sees with this machine's persona (your fakes will differ):

```text
Summarise for the GP: Ying Chen (NHS 485 771 2046), 88 Tianfu Road, Leeds,
phone +86 28 6512 0417, admitted under Dr Tao Zhu for community-acquired pneumonia.
```

The summary comes back with Maria Lopez, Dr Okafor and the real NHS number restored. The clinical content is unchanged, which is why the answer stays useful.

### Patient-experience report over a spreadsheet

An analyst uploads an XLSX of 600 complaint records and asks for themes by department. Shadow classifies this as `report`, so names, emails and phones in the workbook become `[PERSON_0001]`, `[EMAIL_0001]` and so on. The model groups and counts them without inventing details, and the final report lists real patients only on the analyst's screen.

### EHR integration debugging

A developer pastes a FHIR bundle and a stack trace into Claude Code. Patient names, the `ehr.stmarys.internal` hostname, the developer's home path and the API key in the config are all replaced. With `secret = "redact"`, the key leaves as `[REDACTED_SECRET]`.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** on clinician and analyst machines: `shadow install --apply` | Staff using API-backed chat clients, scripts or coding assistants. Covers any tool that honours `HTTPS_PROXY` |
| **Gateway** for internal apps: base URL `http://127.0.0.1:8877/gw/openai/v1` or a custom `azure` route | Your own scribe, triage or summarisation services |
| **Sidecar** in Kubernetes | Product features in a health app; one Shadow per pod, egress locked to the sidecar. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |

Shadow has no central admin console yet, so fleet rollout uses your existing tooling (MDM, configuration management) to place `shadow.toml` and run `shadow install --apply`. An optional local audit log is on the [roadmap](/shadow/project/roadmap/). Today, each request logs its intent and counts per treatment, never values.

## Related

- [HIPAA-safe LLM usage](/shadow/use-cases/hipaa-safe-llm-usage/) and [redacting documents before upload](/shadow/use-cases/redact-documents-before-uploading-to-ai/)
- [Pharma and life sciences](/shadow/industries/pharma-life-sciences/) and [insurance](/shadow/industries/insurance/)
- [Detection](/shadow/concepts/detection/), [intent-aware policy](/shadow/concepts/intent-policy/), [security model](/shadow/reference/security/)
- Detection quality on public datasets: [benchmarks](/shadow/benchmarks/). How Shadow differs from other tools: [compare](/shadow/compare/)
