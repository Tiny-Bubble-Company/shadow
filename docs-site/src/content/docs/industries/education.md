---
title: "Secure ChatGPT and AI use in education: protect student data under FERPA with Shadow"
description: "Let staff and edtech apps use LLMs without sending student names, IDs and records to the provider. Shadow masks them locally and restores the answer."
keywords:
  - ferpa chatgpt
  - student data privacy ai
  - ai in schools data protection
  - edtech llm pii
  - coppa generative ai
  - university ai student records
faq:
  - q: "Is it FERPA compliant for teachers to use ChatGPT with student work?"
    a: "FERPA limits disclosure of personally identifiable information from education records. Shadow reduces what reaches an AI provider by replacing student names, emails, IDs and contact details with fakes, but it does not decide whether a disclosure is permitted. Your institution's FERPA policy and vendor agreements still govern."
  - q: "Does Shadow detect student ID numbers?"
    a: "Not with its built-in detectors, because student ID formats vary by institution and shadow.toml has no custom regex setting. Add known IDs as terms, enable the open-weight NER sidecar for account-style numbers, or embed Shadow in Rust with a custom detector."
  - q: "Can Shadow protect students using AI tools directly?"
    a: "Shadow runs on managed laptops and servers, not phones or Chromebooks, and it covers API-based tools rather than consumer web apps like chatgpt.com. It suits staff devices and the institution's own AI services more than student-owned devices."
  - q: "Can an LMS or edtech product use Shadow?"
    a: "Yes. Point the product's model client at Shadow's gateway (for example http://127.0.0.1:8877/gw/openai/v1) or run Shadow as a sidecar. Student data is masked before it leaves and restored in responses, including streamed ones."
  - q: "Does Shadow help with COPPA?"
    a: "It can reduce children's personal information sent to an AI provider by an edtech operator. COPPA obligations such as notice, parental consent and retention limits still apply to the operator."
sidebar:
  order: 6
---

Shadow is for schools, districts, universities and edtech companies that want staff and products to use LLMs without sending student identities to the model provider. It runs on staff machines, servers or alongside your application, replaces student and parent names, emails, phone numbers, addresses and known identifiers with realistic fakes, and restores the real values in the answer. It supports your FERPA and data protection obligations; it does not replace them.

## What data leaks into AI tools in education

| Workflow | What ends up in the prompt |
|---|---|
| Writing feedback on essays | Student names, sometimes personal stories in the essay itself |
| Drafting emails to parents | Student and parent names, emails, phone numbers |
| Summarising IEPs, 504 plans or support plans | Disability information, diagnoses, names, dates of birth, staff names |
| Admissions and financial aid review | Applicant names, addresses, national IDs, family income details |
| Analysing grade exports | Spreadsheets of names, student IDs and grades |
| Building edtech features | Real student records in test fixtures, LMS hostnames, API keys |

## How Shadow protects it

| Data | Treatment |
|---|---|
| Student, parent and staff names | Built-in name detector, `person:` terms for known names, optional NER sidecar for better recall on free text |
| Emails, phones, home addresses | Built-in detectors, persona-consistent fakes |
| SSNs and national IDs in admissions or aid files | Built-in national ID detector; [Presidio](/shadow/concepts/detection/#presidio) for more formats |
| Student ID numbers | Not detected by built-ins. Use terms for known IDs, the NER sidecar, or a custom detector in Rust |
| School and district names | `organization` and `org:` terms |
| Grade exports and rosters | The `report` intent: typed tokens such as `[PERSON_0012]` for names and contact details |
| Uploaded documents | DOCX, XLSX and PPTX masked in place; PDF text masked; images blocked if you choose |

```toml
organization = ["Riverside Unified School District"]
terms = ["org:Lincoln Middle School", "org:Riverside High School", "term:Bridge Program"]
internal_domains = ["riverside-usd.example", "sis.riverside.internal"]

[model_detector]
enabled = true          # open-weight NER on loopback: better recall on names in free text

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"

[files]
pdf = "text"
images = "block"        # handwritten work and scans are images; no OCR yet
```

:::caution[Limits to plan for]
Dates of birth are not masked. Student IDs need configuration. Disability, health and behavioural details in IEPs are content, not identifiers, so they reach the model as written. In a small class, context alone can identify a student.
:::

## Regulations and frameworks

| Framework | How Shadow helps |
|---|---|
| FERPA (US) | Reduces personally identifiable information from education records disclosed to an AI vendor. Whether a vendor is a "school official" under your agreement is a separate question |
| COPPA (US, under-13s) | Helps an operator minimise children's personal information sent to AI providers; notice and consent duties remain |
| State student privacy laws (for example California SOPIPA) | Supports contractual limits on how student data is shared with subprocessors |
| GDPR / UK GDPR, including Art. 8 for children | Minimisation and pseudonymisation by design (Art. 5(1)(c), 25, 32). Pseudonymised data remains personal data for you (Recital 26) |
| EU AI Act | AI used for admission, evaluation of learning outcomes, or proctoring is Annex III high-risk, with obligations due from 2 December 2027. Shadow supports data minimisation, not conformity |

:::caution[Not legal advice]
This page explains what the software does. It is not a FERPA or GDPR determination. Check with your privacy office and counsel.
:::

## Example workflows

### Email to a parent

Sent (fictional):

```text
Write a supportive email to Ana Silva (ana.silva@mail.example, +1 415 555 0147) about
her son Leo Silva's missed assignments at Lincoln Middle School.
```

Seen by the provider (persona varies):

```text
Write a supportive email to Ana Silva (ana.silva@mail.example, +1 415 555 0147) about
her son Ren Tanaka's missed assignments at Hoshino Gakuen.
```

Ana Silva is the recipient, so her name, email and phone pass through as written and each keep is logged; the model writes to the right parent. The student and the school come after "about", so the provider sees fakes, and the draft comes back about Leo Silva at Lincoln Middle School ("Ren" on its own is restored too, because fake first and last names are restored on their own). Set `[policy.act] targets = "surrogate"` if parent contact details must not reach the provider.

### Grade analysis

A head of department attaches an XLSX gradebook and asks "Analyse these records: which students are at risk?". The wording reads as a report over records, so student names become typed tokens inside the workbook; grades and formulas are untouched. The model lists `[PERSON_0007]` and `[PERSON_0019]`, which the teacher sees as the real names.

### Edtech feedback feature

A writing-feedback product sends essays to Anthropic through a Shadow sidecar. Names and emails in the essay are masked, the feedback streams back, and any fake split across stream events is reassembled and restored before it reaches the student's screen.

## Deployment patterns

| Pattern | Use |
|---|---|
| **Endpoint** on staff laptops (`shadow install --apply`) | Teachers and administrators using approved API-backed assistants on macOS or Windows |
| **Gateway** on a district or campus server | A shared internal assistant or LMS integration; keep Shadow on loopback next to the app |
| **Sidecar** in Kubernetes | Edtech products with AI features, with egress limited to the sidecar |

Shadow does not run on phones, tablets or Chromebooks, and it does not cover consumer web chat sites. The practical model for schools is an approved, API-backed assistant routed through Shadow. Distribute `shadow.toml` with your device management; there is no central console yet (see the [roadmap](/shadow/project/roadmap/)).

## Related

- [Nonprofit](/shadow/industries/nonprofit/) and [government and public sector](/shadow/industries/government-public-sector/)
- [Stop employees pasting personal data into ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/), [shadow AI governance](/shadow/use-cases/shadow-ai-governance/)
- [Gateway mode](/shadow/integrate/gateway-mode/), [intent-aware policy](/shadow/concepts/intent-policy/), [benchmarks](/shadow/benchmarks/), [compare](/shadow/compare/)
