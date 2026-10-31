---
title: "Shadow vs Presidio — open-source Presidio alternative for LLM traffic"
seoTitle: "Shadow vs Presidio: open-source alternative for LLM traffic"
description: Shadow vs Microsoft Presidio for LLM prompts. Presidio detects and redacts PII; Shadow adds reversible fakes, streaming restore and a proxy.
sidebar:
  label: Presidio
  order: 30
keywords:
  - presidio alternative
  - presidio vs shadow
  - microsoft presidio alternative
  - presidio llm
  - presidio deanonymize
  - open source PII redaction
  - PII redaction for LLM
  - presidio streaming
faq:
  - q: Is Shadow an alternative to Microsoft Presidio?
    a: "Partly. Presidio is a PII detection and anonymisation library; Shadow is a privacy layer that sits in front of AI providers, replaces values with reversible realistic fakes and restores them in responses. Shadow can use Presidio as one of its detectors, so many teams use both."
  - q: Can Presidio restore the original values in an LLM response?
    a: "Only with the encrypt operator, which produces ciphertext that the decrypt operator reverses. The default replace operator produces placeholders such as <PERSON> that cannot be mapped back. Presidio has no built-in logic for restoring values in streamed LLM responses or tool calls."
  - q: Does Presidio work with streaming LLM responses?
    a: "Presidio processes text you hand it. It has no built-in handling for server-sent event streams, so you would need to buffer or reassemble the stream yourself. Shadow restores SSE and NDJSON streams on the fly."
  - q: Is Presidio still maintained by Microsoft?
    a: "As of October 2026 the repository redirects from microsoft/presidio to data-privacy-stack/presidio, and documentation lives at presidio.dataprivacystack.org. It is still MIT-licensed and actively released. Container images moved to ghcr.io/data-privacy-stack."
  - q: Which is more accurate, Presidio or Shadow?
    a: "Accuracy depends on the dataset and how entity labels are mapped, and published Presidio numbers vary widely. Shadow can run Presidio as an extra detector. On Shadow's held-out benchmark, Presidio alone leaked 20.74% of sensitive values, Shadow's built-in rules 3.08%, and Shadow with Presidio as an extra detector 1.33%. See the Shadow benchmarks page."
  - q: Is Presidio free?
    a: Yes. Presidio is open source under the MIT licence. Shadow is open source under Apache-2.0. Both are free to self-host.
---

**Presidio is the reference open-source library for detecting and anonymising PII. Shadow is a privacy layer for AI traffic that uses detection as one step, then replaces values with realistic, reversible fakes and restores them in the model's response.** If you need a detector to build your own pipeline, Presidio is an excellent choice. If you need prompts to OpenAI, Anthropic or Gemini masked and answers restored without writing that pipeline, Shadow does it, and can call Presidio for extra recall.

:::note[TL;DR]
Presidio finds PII and replaces it, usually one way, with placeholders like `<PERSON>`. Shadow translates PII into consistent, format-preserving fakes, sends them to the provider, and restores the real values in responses, streams and tool calls. They are complementary: Shadow can run Presidio as a detector.
:::

## At a glance

| | Shadow | Presidio |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Yes, MIT |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Python library; Analyzer and Anonymizer REST containers |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Replace (placeholder), redact, mask, hash, encrypt, custom, keep, `surrogate_ahds` |
| **Restores responses** | Yes, automatically | Only values protected with `encrypt` (via `decrypt`); you wire it up |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | No built-in stream handling |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | No |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Image redactor with OCR; DICOM; structured data module |
| **Self-hosted** | Yes, single binary, no account | Yes |
| **Price model** | Free | Free |

## What Presidio does well

- **Detection breadth.** The Analyzer ships dozens of recognizers, including many national ID formats, and can use spaCy, Stanza or Hugging Face transformer models for named-entity recognition. You can add custom recognizers with regex, deny lists or code.
- **Language flexibility.** Because the NLP engine is pluggable, you can bring a model for your language rather than waiting for a vendor to support it.
- **Images and medical imaging.** The image redactor runs OCR and blacks out PII in screenshots and scans; there is DICOM support for medical images. Shadow does not do OCR.
- **Ecosystem.** Presidio is the default PII backend for many other tools, including LiteLLM, NeMo Guardrails and several gateways. There is a large community, many examples, and an evaluation toolkit (`presidio-research`) for measuring recall on your own data.
- **Operator choice.** Replace, mask, hash, encrypt and custom operators let you pick the trade-off per entity type, which is useful for logs, analytics and dataset preparation.

## Where Shadow is different

### Does Presidio keep LLM answers useful?

Presidio's default `replace` operator turns `Jane Doe` into `<PERSON>`. That is ideal for logs, but in a prompt it removes meaning: the model can't tell two people apart, can't write an email to the right address, and may refuse to complete a table. Shadow replaces `Jane Doe` with a realistic name such as `Hui Zhao`, and `jane@acme.com` with `hui.zhao@zhilan.cn`, drawn from a persistent [persona](/shadow/concepts/personas/). Card numbers stay Luhn-valid, IBANs pass mod-97, paths still look like paths. The model reasons normally, and the answer stays useful.

### Can Presidio reverse anonymisation in a response?

Presidio's only reversible operator is `encrypt`, which produces AES ciphertext. The model sees random-looking strings, and you must find them in the response and decrypt them yourself. Shadow keeps a real-to-fake mapping in an [encrypted local vault](/shadow/concepts/vault/) and restores its fakes in JSON responses, [server-sent event streams](/shadow/concepts/how-it-works/#streaming-hold-back) (even when a fake is split across events), and [tool-call arguments](/shadow/concepts/tool-calls/).

### Intent-aware handling

Shadow classifies each request as act, report, code or converse. A report over hundreds of customer rows gets typed tokens such as `[PERSON_0007]`; an agent action that runs in the provider's cloud keeps real the contact details it needs, so the action works, and each keep is logged. See [intent-aware policy](/shadow/concepts/intent-policy/). Presidio leaves this logic to you.

### A proxy, not just a library

Presidio is something you call from code. Shadow intercepts traffic: set `HTTPS_PROXY` or a base URL and every tool on the machine that calls a [supported AI API](/shadow/reference/providers/), including ones you didn't write, is covered. See [proxy mode](/shadow/integrate/proxy-mode/) and [gateway mode](/shadow/integrate/gateway-mode/).

### Office files in uploads

Shadow masks DOCX, XLSX and PPTX uploads in place and keeps them valid documents, plus base64 and multipart payloads. See [file masking](/shadow/concepts/file-masking/). Presidio has no Office-file handler for LLM uploads.

## When should you choose Presidio?

- You are building a data pipeline (logs, analytics, dataset preparation) and need one-way anonymisation.
- You need OCR on images or DICOM redaction.
- You need detection in a language where you can bring your own spaCy or transformer model.
- You want a library to embed in a Python application and you'll write the LLM-specific parts yourself.

## When should you choose Shadow?

- You want prompts to AI providers masked and responses restored, with no code changes.
- Your users or agents rely on streaming and tool calls.
- You need consistent fakes across requests and sessions, so the same customer has the same fake name tomorrow.
- You want Office uploads masked before they leave.
- You prefer a single Rust binary with no Python runtime on every machine.

## Can you use Presidio with Shadow?

Yes, and it is a common setup. Shadow can call a local Presidio Analyzer as an additional detector. Presidio's findings are mapped to Shadow kinds (`PERSON` to person, `IBAN_CODE` to iban, and so on), merged with Shadow's own detectors, and then translated, stored and restored by Shadow. If Presidio is down, Shadow fails open to its built-in detectors.

```toml
[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"
language = "en"
min_score = 0.5
```

See [Detection: Presidio](/shadow/concepts/detection/#presidio) for setup.

## Migrating from a Presidio-based LLM pipeline

1. Keep your Presidio Analyzer, including custom recognizers, and point Shadow's `[presidio]` section at it.
2. Remove your own anonymise-before-call and deanonymise-after-call code.
3. Route model calls through Shadow: set the SDK base URL to `http://127.0.0.1:8877/gw/<provider>/...` ([gateway mode](/shadow/integrate/gateway-mode/)) or set `HTTPS_PROXY` ([proxy mode](/shadow/integrate/proxy-mode/)).
4. Add your organisation names and codenames as [dictionary terms](/shadow/concepts/detection/#dictionary-terms).
5. Run `shadow scan` on representative prompts to compare what is caught before you switch traffic over.

## How do they compare on accuracy?

Published Presidio results vary a lot with the dataset and label mapping, so a single number is misleading. Shadow's [benchmark](/shadow/benchmarks/) measures leak rate, over-masking, round-trip exactness and latency, with Presidio as a baseline and as a Shadow detector, and publishes its code and mappings. On the held-out set, Presidio alone leaked 20.74% of sensitive values, Shadow's built-in rules 3.08% (about 8 ms per document), and Shadow with Presidio as an extra detector 1.33%. On the third-party Gretel finance set every tool leaks more: Presidio alone 43.39%, Shadow's rules 30.94% and Shadow with Presidio 23.34%. GLiNER alone did best on that set (17.61%), so Shadow's rules are not the strongest detector everywhere.

## Sources

- [Presidio repository (data-privacy-stack/presidio)](https://github.com/data-privacy-stack/presidio)
- [Presidio Anonymizer operators](https://presidio.dataprivacystack.org/anonymizer/)
- [Presidio installation and container images](https://presidio.dataprivacystack.org/installation/)
- [Shadow detection: Presidio](/shadow/concepts/detection/#presidio)

---

*Last reviewed: 2026-10-31. Presidio is a trademark of its owner; this comparison is based on public documentation as of the review date.*
