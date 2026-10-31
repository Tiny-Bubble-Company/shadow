---
title: "Secure LLM use in HR and recruiting: protect candidate and employee data in AI prompts"
description: "Mask candidate and employee names, contact details and IDs in CVs and HR documents before they reach OpenAI, Anthropic or Gemini. Self-hosted, reversible."
keywords:
  - chatgpt candidate data privacy
  - hr ai gdpr
  - anonymize cv for ai screening
  - eu ai act recruitment high risk
  - employee data llm
  - recruiting ai pii masking
faq:
  - q: "Can recruiters paste CVs into ChatGPT?"
    a: "Sending CVs to a consumer AI service puts candidate personal data with a third party, often without a clear lawful basis or agreement. Shadow masks names, emails, phones, addresses and recognised national IDs in CVs and documents before they reach a provider API. Consumer web apps such as chatgpt.com are not built-in hosts, so give recruiters an approved API-backed client."
  - q: "Does masking candidate names make AI screening fair or unbiased?"
    a: "No. Removing names and contact details can reduce some signals a model might react to, but bias can come from schools, employers, gaps, language and many other features that Shadow does not mask. Fairness needs testing, oversight and, where required, a bias audit."
  - q: "Is AI-assisted hiring high-risk under the EU AI Act?"
    a: "AI used for recruitment, selection, promotion, termination and task allocation is listed as high-risk in Annex III. Under the 2026 Digital Omnibus, those obligations now apply from 2 December 2027. Shadow is a data protection control; it does not perform conformity assessment or provide human oversight."
  - q: "Can Shadow mask CVs uploaded as PDF or Word files?"
    a: "Yes for text. DOCX CVs are masked in place and stay valid documents. PDF text is extracted, masked and sent as text, so layout is lost. Scanned CVs and photos are not OCR'd; block images if they may be uploaded."
  - q: "Can we compare a shortlist without the model seeing who the candidates are?"
    a: "Yes. A request that pastes many candidates in at once, such as a table with their names and contact details, is classified as a report, so names, emails, phones and addresses become typed tokens such as [PERSON_0003]. The model ranks and compares tokens, and the answer is restored with real names on your screen."
sidebar:
  order: 14
---

Shadow is for HR teams, recruiters, staffing agencies and HR tech vendors that use large language models on CVs, employee records and people-related documents. It runs on your own laptops, servers or pods, replaces candidate and employee names, emails, phone numbers, addresses and recognised national IDs with fakes before a request reaches the AI provider, and restores them in the answer. It reduces personal data sent to AI vendors; it does not make automated hiring decisions fair or lawful by itself.

## What data leaks into AI tools in HR and recruiting

| Workflow | What typically ends up in the prompt |
|---|---|
| CV screening and summarising | Full CVs: names, contact details, addresses, dates of birth, photos, employment history |
| Writing job offers and rejection emails | Candidate names, salaries, start dates, personal emails |
| Interview notes and scorecards | Candidate and interviewer names, opinions, sometimes health or family details |
| Employee relations and investigations | Names, allegations, sick leave, disciplinary history |
| Payroll and benefits questions | Salaries, bank details, national insurance or social security numbers |
| People analytics | Exports of the workforce with names, demographics and performance ratings |

Much of this is sensitive by any standard, and some of it falls into GDPR special categories: health, ethnicity, trade union membership, religion.

## How Shadow protects it

Shadow masks values in every AI request and uploaded file with fakes from a persistent [persona](/shadow/concepts/personas/), keeps the mapping in an [encrypted local vault](/shadow/concepts/vault/), and restores responses, including streams and tool calls.

| Data | How Shadow handles it |
|---|---|
| Candidate, employee and referee names | Name detector, `person:` terms, the NER sidecar or Presidio for multilingual free text |
| Emails, phones, street addresses | Built-in detectors (addresses in English and German styles; others need the NER sidecar or Presidio) |
| National IDs | US SSN and ITIN, UK National Insurance and NHS numbers, Indian Aadhaar and PAN, Australian TFN built in; more with Presidio |
| Bank details | IBAN detector with mod-97-valid fakes |
| Employer and client company names | `org:` terms, organisations with a legal suffix detected automatically |
| CVs and HR documents | DOCX masked in place; PDF text extracted and masked |
| Candidate lists and workforce exports | `report` intent: typed tokens for people and contact details |

```toml
organization = ["Acme Staffing Ltd"]
terms = ["org:Globex", "org:Initech", "term:Requisition Falcon"]
internal_domains = ["acme-staffing.example"]

[model_detector]
enabled = true                # ml/serve_hf.py: multilingual names and addresses
url = "http://127.0.0.1:8878/detect"

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
national_id = "token"
default = "surrogate"

[files]
pdf = "text"
images = "block"              # CV photos and scanned documents are not OCR'd
```

:::caution[What masking does not hide]
- **Dates of birth and other dates** are not masked.
- **Special-category details** written in prose (a diagnosis, a religion, a union role) are content, not identifiers, and reach the model.
- **Employers, schools and job titles** can identify someone, especially senior or unusual roles. Add key employers as terms if needed.
- **Photos** are not masked. No OCR.
:::

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| GDPR Art. 5(1)(c), 25 and 32 | Data minimisation and pseudonymisation by design before candidate and employee data reaches an AI vendor. Pseudonymised data remains personal data for you (Recital 26) |
| GDPR Art. 9 special categories | Reduces identifiers attached to sensitive content; it does not remove the sensitive content, so a lawful basis is still needed |
| GDPR Art. 22 automated decisions | Not addressed by masking. Keep humans in the decision |
| EU AI Act, Annex III employment use cases | High-risk obligations apply from 2 December 2027 under the 2026 Digital Omnibus. Shadow supports data governance and security; it is not a risk management system, conformity assessment or human oversight measure |
| NYC Local Law 144 (automated employment decision tools) | Unrelated to masking: a bias audit and notices are still required where the law applies |
| ISO/IEC 27001, ISO/IEC 42001, SOC 2 | Evidence of a technical control on AI data flows |

:::caution[Not legal advice]
This page describes technical capabilities, not legal conclusions. Using Shadow does not make AI-assisted hiring lawful, fair or compliant. Work with your data protection officer, employment counsel and works council where relevant.
:::

## Example workflows

### Summarising a CV

What the recruiter sends (all data fictional):

```text
Summarise this CV for the hiring manager: Daniel Okoye, daniel.okoye@mail.example,
+44 7700 900456, 7 Station Road, Bristol. NI number AB 12 34 56 C. Five years at Globex
as a data engineer.
```

What the provider sees (your fakes will differ):

```text
Summarise this CV for the hiring manager: Tao Zhu, tao.zhu@xinghe.cn,
+86 139 2245 7710, 88 Tianfu Road, Bristol. NI number CE 48 21 07 B. Five years at Zhilan
Technology as a data engineer.
```

The summary comes back about Daniel Okoye. His experience, which is what the hiring manager needs, is unchanged.

### Comparing a shortlist

A recruiter pastes a CSV of 40 shortlisted candidates with notes and asks for a comparison against the job description. With that many names and contact details in tabular form, Shadow classifies this as `report`: candidate names, emails and phones become `[PERSON_0001]`, `[EMAIL_0001]` and so on. The model compares tokens and the ranking comes back with real names. A human still makes the decision.

### Offer letter from a template

An HR partner uploads a DOCX offer template filled in for a candidate. Names, address and bank details are masked inside the Word document, it stays valid, and the edited letter comes back with the real details restored.

### Scheduling interviews with an agent

If an assistant emails or schedules candidates itself, they still get the message. With a tool your code runs, the model writes the call with a fake email and Shadow restores the real one in the tool-call arguments before your code sends it. With a provider-hosted tool, Shadow cannot restore inside the provider, so by default the contact details the action needs are kept real and each keep is logged. See [intent-aware policy](/shadow/concepts/intent-policy/).

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** for recruiters and HR partners: `shadow install --apply` | API-backed chat clients and desktop apps on HR laptops |
| **Gateway** for an ATS or HR tool integration: `http://127.0.0.1:8877/gw/openai/v1` | Screening, scheduling and letter-writing features you build |
| **Sidecar** in Kubernetes | HR tech products calling LLMs for many customers. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |

For agencies serving several clients, a separate data directory per client (`SHADOW_HOME` or `--data-dir`) keeps vaults apart. There is no central admin console yet; an optional local audit log is on the [roadmap](/shadow/project/roadmap/).

## Related

- [GDPR-compliant LLM usage](/shadow/use-cases/gdpr-compliant-llm-usage/) and [redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/)
- [Consulting and professional services](/shadow/industries/consulting-professional-services/) and [education](/shadow/industries/education/)
- [Intent-aware policy](/shadow/concepts/intent-policy/), [file masking](/shadow/concepts/file-masking/), [detection](/shadow/concepts/detection/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
