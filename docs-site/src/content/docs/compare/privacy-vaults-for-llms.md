---
title: "Privacy vaults for LLMs: Skyflow, Protecto, Private AI and alternatives"
description: "Privacy vaults for LLMs compared: Skyflow, Protecto, Private AI, Tonic Textual, Wald.ai and Shadow's local encrypted vault. Tokens, fakes and re-identification."
sidebar:
  label: Privacy vaults for LLMs
  order: 4
keywords:
  - LLM privacy vault
  - data privacy vault for AI
  - Skyflow alternative
  - Protecto alternative
  - Private AI alternative
  - reversible pseudonymization LLM
  - tokenization for LLM
faq:
  - q: What is a privacy vault for LLMs?
    a: "A privacy vault stores sensitive values and gives the rest of the system tokens or pseudonyms instead. For LLMs, the vault replaces personal data in prompts before they reach the model and re-identifies the values in responses for authorised users."
  - q: Do I need a hosted privacy vault?
    a: "A hosted vault suits organisations that want central policy, shared tokens across many systems and a managed service. If you only need to protect AI traffic from specific machines or services, a local vault such as Shadow's keeps the mapping on your own infrastructure with no extra service."
  - q: What is the difference between vault tokens and realistic fakes?
    a: "Vault tokens are opaque or semi-opaque strings that clearly are not real data. Realistic fakes look like real data of the same type, such as a valid email or a Luhn-valid card number. Fakes keep prompts natural for the model; tokens make it obvious which values are protected. Shadow uses fakes by default and typed tokens for reports."
  - q: Is there an open-source privacy vault for LLMs?
    a: "Shadow is open source (Apache-2.0) and keeps its mapping in a ChaCha20-Poly1305 encrypted vault on the local machine. LLM Guard and Rehydra also keep reversible mappings. Skyflow, Protecto, Private AI, Tonic Textual and Wald.ai are commercial."
  - q: Can a privacy vault be shared across machines?
    a: "Hosted vaults are shared by design. Shadow's vault can be shared by copying persona.json, vault.bin and vault.key, or by pointing several processes at one data directory."
---

**A privacy vault for LLMs swaps sensitive values for tokens or pseudonyms before a prompt reaches the model, and re-identifies them in the response.** Skyflow, Protecto, Private AI, Tonic Textual and Wald.ai offer this as commercial platforms or containers. Shadow offers it as open-source software with a local encrypted vault and realistic, format-preserving fakes instead of opaque tokens.

## Why a vault and not plain redaction?

Redaction throws information away. A vault keeps a mapping, so the model works on stand-ins and your users still see real values in the answer. The design questions are:

- **Where does the mapping live?** In a vendor's cloud, in your VPC, or on each endpoint?
- **What does the model see?** Opaque tokens, labelled placeholders, or realistic fakes?
- **Who can re-identify?** Policy-governed per user, or whoever runs the process?
- **What else does it cover?** Files, databases, analytics, non-AI systems?

## Comparison

| Product | Deployment | Licence | What the model sees | Re-identification | Streaming | Files | Price model |
|---|---|---|---|---|---|---|---|
| [Skyflow](/shadow/compare/skyflow/) | Vault SaaS; Kong plugin | Proprietary | Vault tokens | Policy-governed | Not documented | Images, PDF, audio, Office (Detect) | Quote |
| [Protecto](/shadow/compare/protecto/) | SaaS or VPC | Proprietary | Consistent "semantic" tokens; FPE for databases | Unmask API under policy | Not documented | Not documented | Quote |
| [Private AI](/shadow/compare/private-ai/) | Self-hosted container or cloud | Proprietary | Placeholders, masks or synthetic values | Re-identify (PrivateGPT Headless) | Not documented | Many types (vendor) | Quote; limited free tier (third-party report) |
| [Tonic Textual](/shadow/compare/tonic-textual/) | SaaS or self-hosted | Proprietary | Redaction or consistent synthesis | Not as a live proxy | No | Files in original format | Pay-as-you-go or quote |
| [Wald.ai](/shadow/compare/wald-ai/) | SaaS chat and API | Proprietary | Contextual substitution | Yes | Not documented | Not documented | Per user |
| **Shadow** | Endpoint proxy, gateway, Rust library | Apache-2.0 | Realistic, format-preserving fakes; typed tokens for reports | Automatic for the local process | Yes (SSE, NDJSON) | DOCX/XLSX/PPTX in place, PDF to text | Free |

## The vendors in brief

### Skyflow

A data privacy vault used well beyond AI: payments, PII storage and analytics, with an LLM Privacy Vault and Detect APIs for files across images, PDF, audio and Office formats. Re-identification is governed by fine-grained policy. Strong if you want one vault for all systems. [Shadow vs Skyflow](/shadow/compare/skyflow/).

### Protecto

Consistent tokens across systems, an unmask API under policy, and format-preserving encryption for databases; available as SaaS or in your VPC. Protecto publishes a vendor-run benchmark white paper against Presidio and Comprehend. [Shadow vs Protecto](/shadow/compare/protecto/).

### Private AI

A self-hosted container with broad multilingual detection (the vendor cites 50+ entity types in 52 languages), placeholders, masks or synthetic values, and re-identification via PrivateGPT Headless. The strongest choice on documented language coverage. [Shadow vs Private AI](/shadow/compare/private-ai/).

### Tonic Textual

Primarily batch de-identification and synthesis for datasets, with consistent synthetic replacement and files redacted in their original format. Tonic also publishes PrivacyBench, which scores replacement quality. A better fit for training and analytics data than for live traffic. [Shadow vs Tonic Textual](/shadow/compare/tonic-textual/).

### Wald.ai

A hosted secure-AI chat and API that substitutes sensitive values contextually and restores them, with a consistent mapping across a conversation. Conceptually close to Shadow, but cloud-hosted. [Shadow vs Wald.ai](/shadow/compare/wald-ai/).

## Shadow's local vault

Shadow's [vault](/shadow/concepts/vault/) is a ChaCha20-Poly1305 encrypted file on the machine that runs Shadow, with atomic writes and safe concurrent use by several processes. Each value maps to a realistic fake from a persistent [persona](/shadow/concepts/personas/): the same customer gets the same fake name tomorrow, and no two real values share a fake. Restoration covers JSON responses, [streams and tool calls](/shadow/concepts/tool-calls/). The [intent policy](/shadow/concepts/intent-policy/) switches to typed tokens like `[PERSON_0007]` for bulk reports.

**What it does not have:** per-user re-identification policy, a central shared vault service, database or analytics integrations, OCR, or an admin console. Anyone who can read the data directory can decrypt the mapping, so protect it like any credential. See the [security model](/shadow/reference/security/).

## How to choose

| If you need... | Consider |
|---|---|
| One governed vault for AI, payments and analytics | Skyflow |
| Consistent tokens across many data systems, in your VPC | Protecto |
| Broad multilingual detection in a self-hosted container | Private AI |
| De-identified datasets for training or analytics | Tonic Textual |
| A hosted secure chat for non-technical staff | Wald.ai |
| Free, open-source, local masking of API, agent and file traffic with streaming and tool-call restoration | Shadow |

These can coexist. For example, Tonic Textual for training data and Shadow for live developer and agent traffic.

## Accuracy

Vendors in this category mostly publish their own benchmarks; treat those as vendor-run. The Shadow [benchmark](/shadow/benchmarks/) measures leak rate, over-masking and round-trip exactness with published code; the commercial vaults above are not included. On its held-out set, Shadow's built-in rules leaked 3.08% of sensitive values and round-tripped 100% of documents exactly.

## Sources

- [Skyflow Detect SDK sample](https://raw.githubusercontent.com/skyflowapi/skyflow-node-sdk/HEAD/samples/detect-api/get-detect-run.ts)
- [Protecto: technology](https://protecto.ai/tech-behind-protecto/)
- [Private AI: PrivateGPT Headless](https://docs.private-ai.com/pgpt-headless)
- [Tonic Textual benchmark](https://www.tonic.ai/ai-model-benchmarks/textual-benchmark) and [PrivacyBench](https://www.tonic.ai/ai-model-benchmarks/privacy-bench)
- [Wald.ai data security for AI](https://wald.ai/datasecurity-for-ai)

---

*Last reviewed: 2026-10-31. All product names are trademarks of their respective owners; this guide is based on public documentation as of the review date.*
