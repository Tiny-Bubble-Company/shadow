---
title: "Secure ChatGPT and LLM use by industry: protect regulated data with Shadow"
description: "How healthcare, finance, legal, public sector and 14 more industries use ChatGPT, Claude and LLM APIs without sending personal or confidential data."
keywords:
  - secure llm use by industry
  - chatgpt for regulated industries
  - ai data privacy by sector
  - pii masking for llms
  - generative ai compliance support
  - self-hosted ai privacy layer
faq:
  - q: "Which industries is Shadow built for?"
    a: "Any organisation whose prompts contain personal data, customer data, secrets or confidential names. It is most useful in regulated sectors such as healthcare, finance, insurance, legal and the public sector, where sending that data to an AI provider is restricted."
  - q: "Does Shadow make us compliant with HIPAA, GDPR or PCI DSS?"
    a: "No tool does that on its own. Shadow is a technical control that reduces what reaches AI providers, which supports requirements such as data minimisation, pseudonymisation and minimum necessary. Compliance depends on your whole programme and your legal advice."
  - q: "What does Shadow cost for a regulated organisation?"
    a: "Nothing to license. Shadow is open source under Apache-2.0, self-hosted, with no account and no telemetry. Your costs are the time to roll it out and any optional detector infrastructure, such as a Presidio container."
  - q: "Does the AI provider still receive our data?"
    a: "It receives a masked version: realistic, format-preserving fakes instead of names, emails, IDs, card numbers, hostnames and secrets. The only real values sent on purpose are those a provider-hosted action (such as a hosted MCP tool sending an email) needs to reach the right person, and each one is logged. Detection is best effort and context is not masked, so keep the provider under your normal vendor and contract controls."
  - q: "Which AI tools does Shadow cover?"
    a: "Anything that calls a model API through Shadow's proxy or gateway: SDKs, internal apps, agents, and coding assistants that use your own API key, for OpenAI, Anthropic, Azure OpenAI, Gemini, Bedrock, OpenRouter, Mistral and more. Consumer web apps such as chatgpt.com are not built-in hosts."
sidebar:
  label: Overview
  order: 0
---

Every industry has its own version of the same problem: people get real value from ChatGPT, Claude and LLM APIs, and the prompts that get the best answers are full of data that should not leave the organisation. Shadow is an open-source privacy layer that sits between your tools and the AI provider. It replaces names, emails, customer records, IDs, internal hostnames and secrets with realistic, reversible fakes before a request leaves, and restores the real values in the answer.

The pages below explain, sector by sector, what leaks, how Shadow handles it, which regulations it helps with, and how to deploy it.

## Choose your industry

### Health and life sciences

| Industry | Main concern | Key frameworks |
|---|---|---|
| [Healthcare](/shadow/industries/healthcare/) | Patient data (PHI) in clinical notes, letters, coding and EHR work | HIPAA, GDPR, NHS guidance |
| [Pharma and life sciences](/shadow/industries/pharma-life-sciences/) | Clinical trial data, compound codenames, pharmacovigilance narratives | GDPR, ICH E6(R3), Part 11, Annex 11 |

### Financial services

| Industry | Main concern | Key frameworks |
|---|---|---|
| [Banking and financial services](/shadow/industries/financial-services/) | Customer records, IBANs, card numbers, KYC files | PCI DSS, GLBA, DORA, GDPR |
| [Insurance](/shadow/industries/insurance/) | Policyholder and claims data, adjuster notes | NAIC AI bulletin, NYDFS Part 500, EU AI Act |
| [Real estate](/shadow/industries/real-estate/) | Tenant, buyer and mortgage data, leases | GLBA, Fair Housing, state privacy laws |

### Legal, professional and public services

| Industry | Main concern | Key frameworks |
|---|---|---|
| [Legal](/shadow/industries/legal/) | Client confidentiality, privilege, matter names | ABA Opinion 512, Model Rule 1.6, GDPR |
| [Consulting and professional services](/shadow/industries/consulting-professional-services/) | Client names, engagement codenames, deliverables | NDAs, GDPR, ISO 27001 |
| [Government and public sector](/shadow/industries/government-public-sector/) | Citizen data, case files, data residency | FedRAMP context, CJIS, GDPR |
| [Education](/shadow/industries/education/) | Student records, IEPs, grades | FERPA, COPPA, GDPR |
| [Nonprofit](/shadow/industries/nonprofit/) | Donor and beneficiary data on a small budget | GDPR, HIPAA (health charities) |
| [HR and recruiting](/shadow/industries/hr-recruiting/) | Candidate CVs, employee records | GDPR, EU AI Act, NYC Local Law 144 |

### Technology, commerce and media

| Industry | Main concern | Key frameworks |
|---|---|---|
| [SaaS and technology](/shadow/industries/saas-technology/) | Customer data in support and engineering, product AI features | SOC 2, GDPR, customer DPAs |
| [E-commerce and retail](/shadow/industries/ecommerce-retail/) | Order data, addresses, payment details | PCI DSS, GDPR, CCPA |
| [Telecommunications](/shadow/industries/telecommunications/) | Subscriber data, network configs | CPNI rules, ePrivacy, NIS2 |
| [Media and entertainment](/shadow/industries/media-entertainment/) | Unreleased titles, talent names, scripts | TPN, NDAs, GDPR |

### Industrial and infrastructure

| Industry | Main concern | Key frameworks |
|---|---|---|
| [Manufacturing](/shadow/industries/manufacturing/) | Trade secrets, supplier names, part codenames | Trade secret law, ITAR/EAR context, ISO 27001 |
| [Energy and utilities](/shadow/industries/energy-utilities/) | OT hostnames and IPs, customer billing data | NERC CIP-011, NIS2 |
| [Automotive](/shadow/industries/automotive/) | Connected-car and driver data, supplier IP | UNECE R155/R156, ISO/SAE 21434, GDPR |

## What is the same in every industry

The specifics change from sector to sector. The mechanism does not.

| Step | What happens |
|---|---|
| Detect | Built-in detectors find emails, phones, names, addresses, IBANs, cards, national IDs, IPs, hostnames, secrets and usernames. Your dictionary adds customers, matters, codenames and colleagues. Optional [Presidio](/shadow/concepts/detection/#presidio) or a local NER model raise recall |
| Decide | The [intent-aware policy](/shadow/concepts/intent-policy/) picks a treatment per value: realistic fake, typed token, kept or redacted |
| Translate | Values become fakes from one coherent [persona](/shadow/concepts/personas/), recorded in an encrypted local [vault](/shadow/concepts/vault/) |
| Restore | The answer, including streamed text and [tool-call arguments](/shadow/concepts/tool-calls/), comes back with your real values |

Uploaded files go through the same pipeline: DOCX, XLSX and PPTX are masked in place, and PDF text is extracted and masked. See [File masking](/shadow/concepts/file-masking/).

## What no industry should assume

:::caution[Know the limits before you rely on it]
- **Detection is best effort.** Unusual formats and internal IDs (MRNs, claim, policy or matter numbers) are not detected by the built-in detectors. Add terms, enable Presidio or the NER sidecar, and test with `shadow scan`.
- **Dates are not masked.** This matters for HIPAA Safe Harbor and for any re-identification analysis.
- **No OCR.** Text in images and scanned PDFs is not masked. Block images where they may contain personal data.
- **Context is not masked.** A unique situation can identify a person or company even with every name replaced.
- **Consumer web apps are not covered.** Shadow protects API traffic. chatgpt.com and claude.ai are not built-in hosts.
- **No central console yet.** Fleet rollout uses your existing device management. See [Shadow by company size](/shadow/company-size/).
:::

Read the full [security model](/shadow/reference/security/) and the [benchmarks](/shadow/benchmarks/) before you make claims about Shadow in a risk assessment.

## Where to go next

- **Try it in five minutes:** [Quick start](/shadow/getting-started/quick-start/)
- **Pick a deployment:** [Integration modes](/shadow/integrate/overview/)
- **Find your task:** [Use cases](/shadow/use-cases/)
- **Plan a rollout:** [Shadow by company size](/shadow/company-size/)
- **See the alternatives:** [How Shadow compares](/shadow/compare/)
