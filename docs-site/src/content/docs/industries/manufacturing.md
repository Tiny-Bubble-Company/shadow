---
title: "Secure LLM use in manufacturing: protect trade secrets and supplier data in AI prompts"
seoTitle: "Secure LLM use in manufacturing: protect trade secrets"
description: "Mask programme codenames, part names, supplier and customer names before engineers send prompts to OpenAI, Anthropic or Gemini. Self-hosted, open source."
keywords:
  - manufacturing ai trade secrets
  - chatgpt intellectual property protection
  - llm data protection manufacturing
  - mask supplier names ai
  - itar ear generative ai
  - engineering ai assistant privacy
faq:
  - q: "Can Shadow stop trade secrets reaching ChatGPT or the OpenAI API?"
    a: "It replaces the names that make content attributable: programme codenames, part names, suppliers, customers, plants, people and internal hostnames, using terms you define. It does not hide technical content such as tolerances, process parameters or chemistry. If the technical content itself is the secret, it should not go to an external model."
  - q: "Is Shadow an ITAR or EAR export-control solution?"
    a: "No. Masking names does not remove technical data, so export-controlled technical data should not be sent to external AI services at all. Shadow can protect identifiers in the AI use you do allow, alongside your export-control process."
  - q: "Does Shadow mask CAD files, drawings or PLM data?"
    a: "No. Shadow masks text: prompts, CSV, JSON, DOCX, XLSX, PPTX and PDF text. CAD formats and images are not masked; block images with images = \"block\" if drawings or screenshots may be uploaded."
  - q: "How do we protect our part numbers and project names?"
    a: "Add them as dictionary terms in shadow.toml or with shadow terms add. Each term gets a consistent fake, so the model can still reason about Project Falcon versus Project Kestrel without seeing either name. Patterned part numbers are not detected automatically, because there is no custom regex setting."
  - q: "Does it work on air-gapped or restricted plant networks?"
    a: "Shadow itself has no telemetry and needs no account, so it runs anywhere you can install a binary, and it can be built from source. It only helps when the AI provider is reachable; for fully air-gapped sites you would point a custom route at a self-hosted model."
sidebar:
  order: 11
---

Shadow is for manufacturers whose engineering, quality, procurement and operations teams use large language models on documents that carry intellectual property. It runs on laptops, servers or pods, replaces programme codenames, part and product names, supplier and customer names, people, plant hostnames and secrets with consistent fakes before a request reaches the AI provider, and restores them in the answer. It hides who and what a document is about; it does not hide the engineering in it.

## What data leaks into AI tools in manufacturing

| Workflow | What typically ends up in the prompt |
|---|---|
| Writing or translating work instructions and SOPs | Product and programme names, plant names, process steps, operator names |
| Quality: 8D reports, CAPA, non-conformance summaries | Customer names, part numbers, supplier names, defect details, inspector names |
| Procurement and supplier negotiation | Supplier names, contract prices, volumes, contact details |
| Engineering change and design reviews | Programme codenames, requirement text, test results |
| Maintenance and OT troubleshooting | PLC and MES hostnames, IP addresses, alarm logs, service account credentials |
| Sales and RFQ responses | Customer names, target prices, delivery schedules |

The damage from a leak here is less about privacy law and more about competitive position, customer contracts and NDAs, which often forbid sharing a customer's name or programme with third parties.

## How Shadow protects it

Shadow sits between the tool and the provider. Detected values are swapped for fakes from a persistent [persona](/shadow/concepts/personas/) and recorded in an [encrypted local vault](/shadow/concepts/vault/), so the same codename always maps to the same fake and answers come back with your real names.

| Data | How Shadow handles it |
|---|---|
| Programme and product codenames, part names | Dictionary `terms` (plain or `term:` prefix), consistent fakes |
| Suppliers, customers, partners | `org:` terms; organisation names with a legal suffix are recognised as organisations automatically |
| Engineers, operators, supplier contacts | Name detector, `person:` terms, the NER sidecar or Presidio for free text |
| Emails, phones, addresses | Built-in detectors |
| Plant hostnames and OT IP addresses | `hostname`, `ip`, `mac` detectors and `internal_domains` |
| Credentials in configs and scripts | `secret` detector; redaction is opt-in |
| Supplier price lists and quality exports | XLSX masked in place (shared strings); numbers and formulas untouched |
| SOPs and 8D reports | DOCX and PPTX masked in place, so they remain valid documents |

```toml
organization = ["Kestrel Precision Components GmbH"]
terms = [
  "Project Falcon", "Project Osprey",            # programme codenames
  "term:KX-400 Actuator",                         # product names
  "org:Globex Automotive", "org:Initech Castings", # customers and suppliers
  "hostname:mes-plant2-01",
]
internal_domains = ["kestrel.example", "plant2.kestrel.internal"]

[policy.always]
secret = "redact"

[files]
pdf = "text"
images = "block"     # drawings, photos and screenshots are not OCR'd
```

Add terms at runtime as new programmes start:

```sh
shadow terms add "Project Kestrel" "org:Umbrella Tooling"
shadow terms list
```

:::caution[What masking does not hide]
- **Technical content.** Tolerances, materials, process parameters, test results and design rationale reach the model as written. If that content is the trade secret, keep it out of external AI.
- **Patterned part numbers** such as `KX-400-0193-B` are not detected automatically; there is no custom regex setting. List the important ones as terms, or write a custom detector in Rust when embedding Shadow.
- **CAD, drawings and images** are not masked. No OCR.
:::

## Regulations and frameworks

| Requirement | How Shadow helps |
|---|---|
| Trade secret law (for example the EU Trade Secrets Directive, the US DTSA) | Supports "reasonable steps" to keep information secret by limiting identifying details sent to AI vendors. It is one control among many |
| Customer NDAs and quality agreements | Keeps customer and programme names out of third-party AI services where NDAs forbid disclosure |
| ITAR and EAR | **Not an export-control solution.** Masking names leaves technical data intact, so controlled technical data should not go to external AI at all |
| GDPR Art. 5(1)(c), 25 and 32 | Minimises and pseudonymises employee and supplier-contact data before it leaves |
| NIS2 (manufacturing of critical products) and IEC 62443 programmes | Keeps OT hostnames, IP ranges and credentials local |
| ISO/IEC 27001, ISO/IEC 42001, TISAX | Evidence of a technical control on AI data flows |

:::caution[Not legal advice]
This page describes technical capabilities, not legal conclusions. Shadow does not make AI use export-compliant or guarantee trade secret protection. Work with your legal, export-control and security teams.
:::

## Example workflows

### Summarising an 8D report

What the quality engineer sends (all data fictional):

```text
Summarise this 8D for Globex Automotive: KX-400 Actuator housings from Initech Castings
failed porosity checks at Plant 2. Containment led by Jonas Weber (jonas.weber@kestrel.example).
```

What the provider sees (your fakes will differ):

```text
Summarise this 8D for Zhilan Motors: Velora Quilent housings from Lumeth Casting Co., Ltd.
failed porosity checks at Plant 2. Containment led by Ying Chen (ying.chen@xinghe.cn).
```

The summary comes back about Globex Automotive, the KX-400 Actuator and Jonas Weber. The porosity issue is still visible to the provider, because it is the substance of the question.

### Supplier comparison in a spreadsheet

A buyer uploads an XLSX with quotes from eight suppliers and asks for a ranked shortlist. Supplier names in the shared strings are replaced; prices and lead times, which are numbers, are untouched. The ranked list comes back with real supplier names.

### PLC script troubleshooting

A controls engineer pastes a Python script that polls `mes-plant2-01.plant2.kestrel.internal` at `10.40.2.15` with `password="Plant2!"`. The hostname and IP become fakes, the password assignment becomes `[REDACTED_SECRET]`, and the fix comes back with your real host and IP restored.

## Deployment patterns

| Pattern | When to use it |
|---|---|
| **Endpoint install** for engineers: `shadow install --apply` | API-backed chat clients, coding assistants and scripts on engineering laptops |
| **Gateway** for internal tools: `http://127.0.0.1:8877/gw/<provider>/...` or a custom route to your Azure OpenAI resource | Quality, SOP and procurement assistants you build |
| **Sidecar** in Kubernetes | Shared AI services; restrict provider egress to the sidecar. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |
| **Server install on a restricted network** | Build from source (`cargo install`) and use `upstream_proxy` to send AI traffic through the approved egress |

There is no central admin console yet: distribute `shadow.toml` with your term list through existing configuration management. Shared personas and vaults for groups of machines are on the [roadmap](/shadow/project/roadmap/); today you copy `persona.json`, `vault.bin` and `vault.key`.

## Related

- [Redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/) and [LLM data loss prevention](/shadow/use-cases/llm-data-loss-prevention-dlp/)
- [Automotive](/shadow/industries/automotive/) and [energy and utilities](/shadow/industries/energy-utilities/)
- [Detection and dictionary terms](/shadow/concepts/detection/), [file masking](/shadow/concepts/file-masking/), [security model](/shadow/reference/security/)
- Detection quality: [benchmarks](/shadow/benchmarks/). Other approaches: [compare](/shadow/compare/)
