---
title: "Secure ChatGPT and LLM use in real estate: protect tenant and buyer data in AI prompts"
description: "Mask tenant, buyer and landlord names, contact details, bank data and IDs in leases and listings before agents use OpenAI, Anthropic or Gemini."
keywords:
  - real estate chatgpt privacy
  - property management ai tenant data
  - mortgage ai glba
  - mask pii in leases ai
  - realtor generative ai data protection
  - fair housing ai
faq:
  - q: "Can property managers use ChatGPT on tenant data?"
    a: "Not through the consumer chatgpt.com website with Shadow, because that is not a built-in intercepted host. With an API-backed client or an internal tool routed through Shadow, tenant names, emails, phone numbers, addresses, bank details and recognised IDs are replaced with fakes before the request leaves, and restored in the answer."
  - q: "Does Shadow mask property addresses?"
    a: "It detects street addresses written in English and German styles, and the optional NER sidecar or Presidio finds more locations. Addresses in other formats may not be detected. In listings the property address is often the point of the prompt, so decide per workflow whether it should be masked."
  - q: "Can AI help with leases and contracts without exposing the parties?"
    a: "Yes. DOCX leases are masked in place, so party names, contact details and bank details are replaced inside the Word file and the document stays valid. PDF text is extracted, masked and sent as text. Scanned leases are not OCR'd."
  - q: "Does Shadow make AI tenant screening compliant with fair housing rules?"
    a: "No. Masking names does not remove bias that can come from addresses, income sources, language or history, and Shadow does not evaluate model decisions. AI-assisted screening and advertising still need human review and legal guidance."
  - q: "Does it help mortgage brokers with GLBA?"
    a: "It supports the Safeguards Rule by keeping detected customer identifiers, such as names, account-style numbers, SSNs and bank details, out of requests to AI vendors. It does not replace an information security programme or vendor oversight."
sidebar:
  order: 18
---

Shadow is for estate agents, brokers, property managers, landlords, mortgage brokers and proptech companies whose teams use large language models on listings, leases, applications and correspondence. It runs on your own computers or servers, replaces tenant, buyer, seller and landlord names, emails, phone numbers, addresses, bank details and recognised national IDs with realistic fakes before a request reaches the AI provider, and restores them in the answer. It reduces the personal data you share with AI vendors; it does not review AI decisions for fairness.

## What data leaks into AI tools in real estate

| Workflow | What typically ends up in the prompt |
|---|---|
| Writing listings and marketing copy | Property address, seller name, sometimes access codes and occupancy details |
| Tenant and buyer correspondence | Names, emails, phone numbers, viewing times, complaints |
| Lease drafting and review | Party names, addresses, rent, deposit, bank details, guarantor details |
| Rental applications and screening | Income, employer, references, national IDs, credit details |
| Mortgage and financing | SSNs, bank statements, account numbers, loan details |
| Maintenance and portfolio analysis | Tenant lists, unit addresses, arrears, contractor names |

Rental and mortgage files are dense with personal and financial data, and agents often use free AI tools on their phones and laptops between viewings.

## How Shadow protects it

Shadow masks values in every AI request and uploaded file with fakes from a persistent [persona](/shadow/concepts/personas/), records the mapping in an [encrypted local vault](/shadow/concepts/vault/), and restores the response.

| Data | How Shadow handles it |
|---|---|
| Tenant, buyer, seller, landlord, guarantor names | Name detector, `person:` terms, NER sidecar or Presidio |
| Emails and phone numbers | Built-in detectors |
| Street addresses | English and German styles built in; NER sidecar or Presidio for other formats |
| Bank details and cards | IBAN (mod-97) and card (Luhn) detectors with valid-looking fakes |
| National IDs | US SSN and ITIN, UK National Insurance number, Indian Aadhaar and PAN, Australian TFN and ABN; more with Presidio |
| Agency, developer and contractor names | `organization` and `org:` terms |
| Leases, applications, rent rolls | DOCX and XLSX masked in place; PDF text masked |
| Tenant lists and arrears reports | `report` intent with typed tokens |

```toml
organization = ["Harbour Lettings Ltd"]
terms = ["org:Initech Property Services", "term:Riverside Quarter"]
internal_domains = ["harbourlettings.example"]

[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"
entities = ["PERSON", "LOCATION", "US_SSN", "US_BANK_NUMBER", "US_DRIVER_LICENSE", "UK_NINO"]

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
national_id = "token"
default = "surrogate"

[files]
pdf = "text"
images = "block"     # ID scans, payslips as photos and floor plans are not OCR'd
```

:::caution[Known gaps for property data]
- **Addresses in other formats** (outside English and German styles) need the NER sidecar or Presidio, and detection is best effort.
- **Dates** (dates of birth, move-in dates) are not masked.
- **Account and loan numbers** without a recognised format are not detected by the built-in detectors; there is no custom regex setting. Presidio's `US_BANK_NUMBER` and the NER sidecar's account-number detection help.
- **Images and scanned documents** are not masked. Many rental applications include ID photos and scanned payslips.
:::

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| GLBA Safeguards Rule (16 CFR 314), for mortgage brokers and lenders | A technical safeguard that keeps detected customer information out of AI vendor requests. It does not replace your written information security programme or service provider oversight |
| GDPR / UK GDPR Art. 5(1)(c), 25 and 32 | Data minimisation and pseudonymisation for tenant and buyer data. Pseudonymised data is still personal data for you (Recital 26) |
| US state privacy laws (for example CCPA/CPRA) | Reduces personal information shared with AI service providers |
| Fair Housing Act and equivalent equality laws | **Not addressed by masking.** AI-written ads and AI-assisted screening still need human review for discriminatory content or outcomes |
| AML and KYC rules for estate agents | Masks identifiers in AI-assisted KYC work; it does not perform checks or keep records |

:::caution[Not legal advice]
This page describes technical capabilities, not legal conclusions. Using Shadow does not make a business compliant with GLBA, GDPR, fair housing or AML rules. Work with your compliance adviser.
:::

## Example workflows

### Drafting an arrears letter

What the property manager sends (all data fictional):

```text
Draft a polite arrears reminder to Tom Barker, Flat 3, 221 Baker Street, London,
tom.barker@mail.example, +44 20 7946 0018. Two months owed; pay to IBAN
GB33 BUKB 2020 1555 5555 55.
```

What the provider sees (your fakes will differ):

```text
Draft a polite arrears reminder to Hui Zhao, Flat 3, 88 Tianfu Road, London,
hui.zhao@zhilan.cn, +86 28 6512 0417. Two months owed; pay to IBAN
GB71 WEST 1234 5698 7654 32.
```

The letter comes back with Tom Barker's real name, address and the real IBAN. The fake IBAN still passes mod-97, so the model treats it like a real one.

### Reviewing a lease

An agent uploads a DOCX lease and asks for a plain-English summary of the break clause and the tenant's obligations. Names of landlord, tenant and guarantor, addresses and bank details are replaced inside the Word document. The summary comes back with the real parties.

### Portfolio arrears analysis

A finance manager uploads an XLSX rent roll of 450 units and asks which buildings have rising arrears. The `report` intent turns tenant names, emails and phones into typed tokens; amounts stay as numbers. The analysis is restored locally.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** on office laptops: `shadow install --apply` | Agents and property managers using API-backed chat clients and desktop apps |
| **Gateway** for a CRM or property management integration: `http://127.0.0.1:8877/gw/openai/v1` | Letter generation, enquiry triage and listing tools you build |
| **Sidecar** in Kubernetes | Proptech products calling LLMs for many customers. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |

Shadow does not run on phones, so mobile AI apps are not covered; steer sensitive work to laptops with Shadow installed. There is no central admin console yet; see [small businesses](/shadow/company-size/small-business/) for a rollout checklist.

## Related

- [Redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/) and [privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/)
- [Financial services](/shadow/industries/financial-services/) and [insurance](/shadow/industries/insurance/)
- [File masking](/shadow/concepts/file-masking/), [detection](/shadow/concepts/detection/), [intent-aware policy](/shadow/concepts/intent-policy/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
