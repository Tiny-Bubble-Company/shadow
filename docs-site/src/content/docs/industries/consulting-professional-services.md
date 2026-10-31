---
title: "Secure LLM use in consulting and professional services: keep client data out of AI prompts"
description: "Mask client names, engagement codenames and contacts in prompts, decks and spreadsheets before consultants use OpenAI, Anthropic or Gemini. Self-hosted."
keywords:
  - consulting firm chatgpt client confidentiality
  - professional services ai data protection
  - mask client names llm
  - nda generative ai
  - accounting firm ai privacy
  - consultant ai pptx xlsx redaction
faq:
  - q: "Can consultants use ChatGPT or Claude on client work without breaching NDAs?"
    a: "That depends on the NDA and the engagement terms, which often restrict disclosure of a client's identity and information to third parties. Shadow replaces client names, engagement codenames, people and contact details with fakes before requests reach the provider API, which reduces what is disclosed. The substance of the work still reaches the model, so check the engagement terms first."
  - q: "Can Shadow mask client decks and models, not just prompts?"
    a: "Yes. PPTX, DOCX and XLSX uploads are masked in place and stay valid files: slide text runs, document text and spreadsheet shared strings are replaced, while numbers and formulas are untouched. PDF text is extracted and sent masked. Images and embedded charts are not masked."
  - q: "How do we keep one client's data separate from another's?"
    a: "Run a separate Shadow data directory per client with SHADOW_HOME or --data-dir. Each directory has its own persona, vault and key, so mappings never mix. Each running instance needs its own listen address."
  - q: "Will the client name appear in the AI's answer?"
    a: "Yes, on your screen. The provider sees a consistent fake company name, and Shadow restores the real name in the response before it reaches you, including in streamed answers."
  - q: "Is Shadow enough for client data protection in an AI policy?"
    a: "No single control is. Shadow is a technical safeguard for detected identifiers. Firms still need an AI policy, approved tools, client consent where required, and review of outputs."
sidebar:
  order: 15
---

Shadow is for management consultancies, accounting and audit firms, advisory, engineering and research firms whose staff use large language models on client work. It runs on consultants' laptops or on firm servers, replaces client names, engagement codenames, people and contact details with consistent fakes before a request reaches the AI provider, and restores them in the answer. Decks, models and reports are masked in place, so they come back as the same kind of file.

## What data leaks into AI tools in professional services

| Workflow | What typically ends up in the prompt |
|---|---|
| Drafting proposals and deliverables | Client name, engagement codename, scope, fees, stakeholder names |
| Summarising interviews and workshops | Interviewee names, roles, candid opinions, emails |
| Building and checking models | XLSX with client entities, cost centres, employee lists, salaries |
| Reworking slide decks | PPTX with client names, logos, org charts, strategy |
| Due diligence and M&A | Target company names, deal codenames, financials, management names |
| Audit and tax | Client entity names, bank details, tax IDs, transaction listings |

Professional services firms hold many clients' confidential information at once, often under NDAs that forbid naming the client to third parties.

## How Shadow protects it

Shadow detects sensitive values in every AI request and uploaded file, replaces them with fakes from a persistent [persona](/shadow/concepts/personas/), records the mapping in an [encrypted local vault](/shadow/concepts/vault/), and restores the answer.

| Data | How Shadow handles it |
|---|---|
| Client and target company names | `org:` terms; names with a legal suffix are recognised as organisations, and the bare name is protected too |
| Engagement and deal codenames | Dictionary `terms` |
| Stakeholders, interviewees, client staff | Name detector, `person:` terms, NER sidecar or Presidio |
| Emails, phones, addresses | Built-in detectors |
| IBANs, card numbers, national IDs | Checksum-validated detectors with checksum-valid fakes |
| Decks, documents, models | PPTX, DOCX, XLSX masked in place; PDF text masked |
| Interview transcripts and listings over many people | `report` intent tokenises people and contact details |

A per-client configuration:

```toml
# ~/clients/globex/shadow.toml
organization = ["Bright Path Advisory LLP"]       # your firm
terms = [
  "org:Globex Corporation", "org:Initech",        # client and target
  "Project Kestrel",                              # engagement codename
  "person:Laura Chen", "person:Ravi Menon",       # client stakeholders
]
internal_domains = ["globex.example", "brightpath.example"]

[policy.always]
secret = "redact"

[files]
pdf = "text"
images = "block"
```

### Keeping clients apart

Each Shadow data directory has its own persona, vault and key. Give each sensitive client its own directory so mappings never mix:

```sh
SHADOW_HOME=~/clients/globex shadow init --org "Bright Path Advisory LLP"
SHADOW_HOME=~/clients/globex shadow run --listen 127.0.0.1:8877

SHADOW_HOME=~/clients/umbrella shadow init --org "Bright Path Advisory LLP"
SHADOW_HOME=~/clients/umbrella shadow run --listen 127.0.0.1:8879
```

Point each client workspace's base URL at its own port. For most firms one persona per machine with all client names as terms is enough; separate directories are for engagements with stricter walls.

:::caution[What masking does not hide]
- **Substance.** Strategy, numbers and findings reach the model as written. A distinctive situation can identify a client.
- **Logos, images and embedded charts** in decks are not masked. Charts' cached values and embedded workbooks inside Office files are not masked either.
- **Client-specific IDs** (cost centres, entity codes) are not detected unless listed as terms.
:::

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| NDAs and engagement letters | Reduces disclosure of client identity and personal data to AI vendors. Does not make a prohibited disclosure permitted |
| Professional confidentiality rules (for example accountancy ethics codes) | A safeguard that supports confidentiality when AI use is permitted |
| GDPR Art. 5(1)(c), 25, 28 and 32 | Data minimisation and pseudonymisation when acting as a processor for client data. Pseudonymised data remains personal data for whoever holds the key (Recital 26) |
| EU AI Act | Supports data governance; most consulting use is not high-risk, but client deliverables built on AI may be |
| ISO/IEC 27001, ISO/IEC 42001, SOC 2 | Evidence of a technical control on AI data flows for client security questionnaires |

:::caution[Not legal advice]
This page describes technical capabilities, not legal or professional conclusions. Shadow does not satisfy NDAs, professional standards or data protection law on its own. Check engagement terms and your firm's AI policy.
:::

## Example workflows

### Reworking a steering committee deck

A consultant uploads a PPTX and asks for a tighter storyline. What one slide says (all data fictional):

```text
Project Kestrel: Globex Corporation acquisition of Initech. Sponsor: Laura Chen (CFO).
```

What the provider sees (your fakes will differ):

```text
Velora Quilent: Zhilan Technology Co., Ltd. acquisition of Lumeth Systems. Sponsor: Ying Chen (CFO).
```

The suggestions and any rewritten slide text come back with the real names, and the deck is still a valid presentation.

### Synthesising stakeholder interviews

A consultant pastes notes from 25 interviews and asks for themes. With many names and emails in one request, Shadow classifies it as `report` and uses typed tokens, so the model attributes quotes to `[PERSON_0007]` rather than inventing biographies. The synthesis comes back with real interviewee names.

### Checking a cost model

An analyst uploads an XLSX of 2,000 rows by cost centre and supplier. Supplier and client names in shared strings are replaced; numbers and formulas are untouched. The model explains anomalies and the answer is restored.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** on consultant laptops: `shadow install --apply` | API-backed chat clients, assistants and scripts; terms per client in `shadow.toml` |
| **Per-client data directories** | Engagements with strict information barriers |
| **Gateway** for firm knowledge tools: `http://127.0.0.1:8877/gw/<provider>/...` | Proposal, research and document assistants you build |
| **Sidecar** in Kubernetes | Firm-wide AI services. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |

There is no central admin console or shared team vault yet; both team identities and an optional audit log are on the [roadmap](/shadow/project/roadmap/). Consumer web chat apps such as chatgpt.com are not built-in hosts, so provide an approved API-backed client.

## Related

- [Redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/) and [stop PII leaks to ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/)
- [Legal](/shadow/industries/legal/) and [financial services](/shadow/industries/financial-services/)
- [File masking](/shadow/concepts/file-masking/), [the vault](/shadow/concepts/vault/), [personas](/shadow/concepts/personas/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
