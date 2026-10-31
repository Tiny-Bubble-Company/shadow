---
title: "Shadow vs Tonic Textual — open-source Tonic Textual alternative for live LLM traffic"
description: Shadow vs Tonic Textual. Tonic redacts and synthesises data for datasets; Shadow masks live AI traffic with reversible fakes. Compare features and pricing.
sidebar:
  label: Tonic Textual
  order: 37
keywords:
  - tonic textual alternative
  - tonic textual vs shadow
  - tonic ai alternative
  - open source tonic textual
  - tonic textual pricing
  - synthetic PII replacement
  - PII redaction for LLM
  - unstructured data de-identification
faq:
  - q: "Is Shadow an alternative to Tonic Textual?"
    a: "For live AI traffic, yes; for building de-identified datasets, usually not. Tonic Textual redacts or synthesises sensitive data in documents and datasets, often for training and RAG. Shadow sits in front of AI providers and translates live requests into realistic fakes, then restores the response."
  - q: "Does Tonic Textual restore original values?"
    a: "Tonic Textual's focus is one-way de-identification and consistent synthesis for datasets. It is not documented as a request-and-response proxy that restores values in LLM answers. Shadow restores values automatically, including in streams and tool calls."
  - q: "How much does Tonic Textual cost?"
    a: "Tonic Textual offers pay-as-you-go cloud usage and quoted enterprise plans, including self-hosting. Shadow is free and open source under Apache-2.0."
  - q: "How accurate is Tonic Textual?"
    a: "Tonic publishes a vendor-run benchmark (October 2025) reporting an aggregate F1 of 0.96 for Textual on legal, medical and call-transcript data. Shadow publishes its own methodology and results on the benchmarks page."
  - q: "Can Tonic Textual run on-premises?"
    a: "Yes. Tonic Textual is available as SaaS or self-hosted with Docker or Helm. Shadow is self-hosted only, as a single binary or container."
---

**Tonic Textual is a commercial platform for redacting and synthesising sensitive data in unstructured text and files, built mainly for preparing datasets for training, RAG and analytics. Shadow is an open-source (Apache-2.0) privacy layer that masks live AI traffic with realistic, reversible fakes and restores them in responses, streams and tool calls.** If you are de-identifying a document corpus, Tonic Textual is built for that. If you are protecting prompts on their way to OpenAI, Anthropic or Gemini, Shadow is built for that.

:::note[TL;DR]
Tonic Textual and Shadow both replace sensitive values with realistic synthetic ones rather than `[REDACTED]`. Tonic applies this to datasets and files in batch, with strong published detection results and file-format fidelity. Shadow applies it to live requests on the endpoint, reversibly, with streaming and tool-call restoration. They are different jobs and often complementary.
:::

## At a glance

| | Shadow | Tonic Textual |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, proprietary |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | SaaS or self-hosted (Docker, Helm); SDK and API |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Redaction or consistent synthesis |
| **Restores responses** | Yes, automatically | Not as a proxy |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | No |
| **Tool calls** | Yes, arguments restored locally; intent-aware | No |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Files redacted in their original format; PDF and images |
| **Self-hosted** | Yes, single binary, no account | Yes (Docker, Helm) |
| **Price model** | Free | Pay-as-you-go cloud or quote |

## What Tonic Textual does well

- **Detection quality with published numbers.** Tonic's [benchmark](https://www.tonic.ai/ai-model-benchmarks/textual-benchmark) (vendor-run, October 2025, legal, medical and call-transcript data) reports an aggregate F1 of 0.96 for Textual, with AWS Comprehend at 0.88 and Presidio at 0.57 on the same data.
- **Consistent synthesis.** Textual can replace values with realistic synthetic ones and keep them consistent across documents, so a de-identified dataset still reads naturally.
- **Measuring replacement quality.** Tonic released [PrivacyBench](https://www.tonic.ai/ai-model-benchmarks/privacy-bench), a public benchmark (CC-BY-4.0) that scores not only detection but replacement quality and cross-document consistency. That is a useful contribution to the whole field.
- **Files in original format.** Documents are redacted in place, including PDFs and images, which matters for dataset pipelines.
- **Flexible deployment.** Cloud or self-hosted with Docker or Helm.

## Where Shadow is different

### Is Shadow a free alternative to Tonic Textual?

For live AI traffic, yes. Shadow is Apache-2.0, needs no account, and runs as one binary on macOS, Linux or Windows. It is not a dataset de-identification platform.

### Reversible by design

Tonic's synthesis is designed to produce a de-identified copy. Shadow's fakes are designed to come back: every real-to-fake pair is stored in an [encrypted local vault](/shadow/concepts/vault/), and the model's answer is translated back to real values before you see it.

### Live traffic, streams and agents

Shadow intercepts requests as an [HTTPS proxy](/shadow/integrate/proxy-mode/) or [gateway](/shadow/integrate/gateway-mode/), restores [streamed responses](/shadow/concepts/how-it-works/#streaming-hold-back) as they arrive and restores [tool-call arguments](/shadow/concepts/tool-calls/) before your tools run. Tonic Textual is called on content, not placed in the request path.

### Persistent persona

Shadow's fakes come from one persistent [persona](/shadow/concepts/personas/) per machine, so the same customer has the same fake across days and sessions, and fakes are coherent with each other (names, domains, phone dial codes from the same region).

### Intent-aware policy

Shadow classifies requests as act, report, code or converse and chooses fakes, typed tokens, a logged keep or redaction per value. See [intent-aware policy](/shadow/concepts/intent-policy/).

### Where Tonic is ahead

Tonic handles images and scanned PDFs; Shadow has no OCR and converts PDFs to masked text. Tonic also publishes detection results; Shadow's are on its [benchmark page](/shadow/benchmarks/).

## When should you choose Tonic Textual?

- You need de-identified datasets for model training, fine-tuning, RAG indexes or analytics.
- You need files, PDFs and images redacted in their original format.
- You want a vendor-supported platform with published accuracy figures.

## When should you choose Shadow?

- You want prompts to AI providers masked and responses restored, in real time.
- You use streaming, tool calls and agents.
- You need a free, open-source, local component.

## Using them together

A natural split: use Tonic Textual to de-identify the corpus you index for RAG or use for fine-tuning, and run Shadow on the request path so that whatever users paste in at query time, and whatever the retriever pulls in, is also masked before it reaches the provider. PrivacyBench is also a natural dataset for scoring Shadow's replacement quality; see the [benchmark](/shadow/benchmarks/).

## How do they compare on accuracy?

Tonic's numbers are vendor-run on Tonic-selected data. Shadow does not quote its own numbers here; see the [benchmark](/shadow/benchmarks/) for methodology, datasets and results.

## Sources

- [Tonic Textual](https://www.tonic.ai/)
- [Tonic Textual benchmark (vendor-run)](https://www.tonic.ai/ai-model-benchmarks/textual-benchmark)
- [Tonic PrivacyBench](https://www.tonic.ai/ai-model-benchmarks/privacy-bench)
- [PrivacyBench scoring code](https://github.com/TonicAI/privacy_bench_metrics)
- [Shadow personas](/shadow/concepts/personas/)

---

*Last reviewed: 2026-10-31. Tonic, Tonic Textual and PrivacyBench are trademarks of their owner; this comparison is based on public documentation as of the review date.*
