---
title: "Secure ChatGPT and LLM use for nonprofits: protect donor and beneficiary data for free"
description: "Free, open-source way for charities and NGOs to mask donor, volunteer and beneficiary data before staff use OpenAI, Anthropic or Gemini. No account needed."
keywords:
  - nonprofit chatgpt data privacy
  - charity ai gdpr
  - protect donor data ai
  - free pii masking tool
  - ngo generative ai safeguarding
  - beneficiary data llm
faq:
  - q: "Is Shadow free for nonprofits?"
    a: "Yes. Shadow is open source under Apache-2.0 and free for everyone. There is no account, no licence key, no hosted service and no telemetry. Your only costs are the AI provider you already use and the time to set it up."
  - q: "Can charity staff paste donor lists into ChatGPT safely?"
    a: "Not into the consumer chatgpt.com website through Shadow, because that is not a built-in intercepted host. With an approved API-backed client, Shadow turns donor names, emails, phones and addresses in bulk lists into typed tokens such as [PERSON_0001] before the request leaves, and restores them in the answer."
  - q: "We support vulnerable people. Is masking enough?"
    a: "No. Masking replaces names and contact details, but case notes describe situations that can identify someone, and Shadow does not mask dates or images. For safeguarding cases, keep the most sensitive material out of external AI and use Shadow for the work you do allow."
  - q: "Do we need technical staff to run it?"
    a: "Someone comfortable installing software and editing a short text file can set it up on a laptop in under an hour. Running it for a whole organisation without a central console takes more care; see the small business guide."
  - q: "Does Shadow help with GDPR for charities?"
    a: "It supports data minimisation and pseudonymisation when staff use AI, which are GDPR principles and safeguards. It does not provide a lawful basis, privacy notices or a data protection impact assessment."
sidebar:
  order: 17
---

Shadow is for charities, NGOs, foundations, community groups and social enterprises whose staff and volunteers use large language models to write appeals, summarise case notes and analyse supporter data. It is free, open source and runs on your own computers. It replaces donor, volunteer and beneficiary names, emails, phone numbers, addresses and bank details with realistic fakes before a request reaches the AI provider, and puts the real values back in the answer. It is a safeguard that fits a small budget, not a replacement for a data protection policy.

## What data leaks into AI tools at nonprofits

| Workflow | What typically ends up in the prompt |
|---|---|
| Fundraising appeals and thank-you letters | Donor names, addresses, gift amounts, giving history |
| Donor and supporter analysis | CRM exports with names, emails, postcodes, gift aid or tax status |
| Case notes and referrals | Beneficiary names, family details, health, immigration or housing situations |
| Grant applications and reports | Partner organisations, beneficiary stories, staff names |
| Volunteer coordination | Volunteer names, phone numbers, availability, background-check status |
| Finance | Bank details, IBANs, payment card numbers from donation records |

Nonprofits often hold data on people in vulnerable situations, under tight budgets, with staff using whatever AI tool is free.

## How Shadow protects it

Shadow sits between the AI tool and the provider. It masks values with fakes from a persistent [persona](/shadow/concepts/personas/), keeps the mapping in an [encrypted vault](/shadow/concepts/vault/) on your computer, and restores the response.

| Data | How Shadow handles it |
|---|---|
| Donor, volunteer and beneficiary names | Name detector, `person:` terms, optional NER sidecar for better recall |
| Emails, phones, street addresses | Built-in detectors (addresses in English and German styles) |
| Bank details and card numbers | IBAN (mod-97) and card (Luhn) detectors with valid-looking fakes |
| National IDs | US SSN and ITIN, UK National Insurance and NHS numbers, Indian Aadhaar and PAN, Australian TFN and ABN |
| Partner organisations and programme names | `org:` and plain terms |
| CRM exports, spreadsheets, letters | CSV, XLSX and DOCX masked; donor lists get typed tokens in reports |

A simple configuration for a charity:

```toml
organization = ["Riverside Community Trust"]
terms = ["org:Northside Food Bank", "term:Warm Homes Appeal"]
internal_domains = ["riversidetrust.example"]

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"

[files]
images = "block"     # photos and scans are not masked
```

### Setting it up on one laptop

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Riverside Community Trust"
shadow install            # shows what it will change
shadow install --apply    # runs in the background and trusts the local certificate
```

Then point your AI tool at Shadow, either with its base URL setting (`http://127.0.0.1:8877/gw/openai/v1`) or the proxy setting. The [quick start](/shadow/getting-started/quick-start/) walks through it. Release binaries are on the [roadmap](/shadow/project/roadmap/); until they exist for your platform, the installer builds from source, which needs Rust and takes a few minutes.

:::caution[What masking does not hide]
- **Situations.** A case note describing a rare circumstance in a small community can identify someone without any name.
- **Dates** are not masked, and **images and scanned forms** are not masked (no OCR).
- **The ChatGPT website** is not covered. Shadow protects tools that call the provider APIs.
:::

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| GDPR / UK GDPR Art. 5(1)(c), 25 and 32 | Data minimisation and pseudonymisation before supporter or beneficiary data reaches an AI vendor. Pseudonymised data is still personal data for you (Recital 26) |
| GDPR Art. 9 special categories (health, religion, ethnicity) | Removes identifiers attached to sensitive content; the content itself still reaches the model, so a lawful basis is needed |
| HIPAA, for health nonprofits that are covered entities or business associates | Supports the minimum necessary standard; does not de-identify data (dates are not masked) and does not replace a BAA |
| PCI DSS, for donation card data | Card numbers are replaced with Luhn-valid fakes, keeping real numbers out of AI requests. It does not change the scope of your payment systems |
| Fundraising regulator codes and donor promises | Helps honour commitments not to share supporter data with third parties beyond what is necessary |

:::caution[Not legal advice]
This page describes technical capabilities, not legal conclusions. Using Shadow does not make a charity compliant with data protection, health privacy or fundraising rules. Ask your data protection lead or a pro bono adviser.
:::

## Example workflows

### Thank-you letter for a major donor

What the fundraiser sends (all data fictional):

```text
Write a warm thank-you letter to Margaret Ellis, 4 Church Lane, York, margaret.ellis@mail.example,
for her gift to the Warm Homes Appeal. Mention Northside Food Bank, our partner.
```

What the provider sees (your fakes will differ):

```text
Write a warm thank-you letter to Margaret Ellis, 4 Church Lane, York, margaret.ellis@mail.example,
for her gift to the Lumeth Draane. Mention Zhilan Food Co., Ltd., our partner.
```

Margaret Ellis is the recipient, so her name, address and email pass through as written and each keep is logged; the model thanks the right donor. The appeal and the partner are not the recipient, so they are masked and the letter comes back about the Warm Homes Appeal and Northside Food Bank. Set `[policy.act] targets = "surrogate"` if donor details must not reach the provider.

### Segmenting a donor export

A CRM export of 3,000 supporters is pasted in as CSV with the question "Which segments lapsed this year?". Shadow classifies it as a report, so names, emails, phones and addresses become `[PERSON_0001]`, `[EMAIL_0001]` and so on. Gift amounts and dates are left alone so the analysis works. The answer is restored on your screen.

### Grant report from case summaries

A programme manager pastes five anonymised-by-hand case summaries. Shadow still catches the names and phone numbers someone forgot to remove. It is a second line of defence, not the first.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **One laptop**: `shadow install --apply` | A small team or a single fundraiser or analyst |
| **A few laptops** with the same `shadow.toml` | Small charities; copy the config file and run the installer on each |
| **Server or container** with the gateway | A CRM integration or internal tool you build. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |

There is no central console to manage many machines yet. See [small businesses](/shadow/company-size/small-business/) for a practical rollout checklist.

## Related

- [Stop PII leaks to ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/) and [GDPR-compliant LLM usage](/shadow/use-cases/gdpr-compliant-llm-usage/)
- [Healthcare](/shadow/industries/healthcare/) and [education](/shadow/industries/education/)
- [Quick start](/shadow/getting-started/quick-start/), [intent-aware policy](/shadow/concepts/intent-policy/), [security model](/shadow/reference/security/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
