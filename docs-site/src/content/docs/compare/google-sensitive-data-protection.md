---
title: "Shadow vs Google Sensitive Data Protection — Cloud DLP alternative for LLMs"
description: Shadow vs Google Cloud Sensitive Data Protection (Cloud DLP). Google offers reversible FPE; Shadow adds realistic fakes, a local proxy and streaming restore.
sidebar:
  label: Google Sensitive Data Protection
  order: 15
keywords:
  - google cloud dlp alternative
  - sensitive data protection alternative
  - google dlp vs shadow
  - cloud dlp format preserving encryption
  - google dlp pricing
  - reidentifyContent
  - PII redaction for LLM
  - reversible pseudonymization LLM
faq:
  - q: "Is Shadow an alternative to Google Cloud DLP for LLM prompts?"
    a: "Yes for live LLM traffic. Google Sensitive Data Protection (formerly Cloud DLP) is a cloud API for inspecting and de-identifying data, including reversible format-preserving encryption. Shadow runs locally in front of AI providers, replaces values with realistic fakes and restores them in responses automatically."
  - q: "Can Google Sensitive Data Protection re-identify data?"
    a: "Yes. With format-preserving encryption (FF1) or deterministic encryption with a surrogate infoType, the reidentifyContent method can restore the original values. Google is the only hyperscaler PII service in this comparison with documented reversible, format-preserving transforms."
  - q: "What is the difference between Google FPE and Shadow's fakes?"
    a: "Google FPE output keeps the alphabet and length but looks random, for example a name becomes a string of random characters. Shadow produces realistic values, such as a plausible name or a valid email at a consistent fake domain, so the model reads natural text."
  - q: "How much does Google Sensitive Data Protection cost?"
    a: "Google's pricing page lists content inspection at $3 per GB beyond the first 1 GB per month, and transformation at about $2 per GB. Shadow is free and open source."
  - q: "Which is more accurate, Google DLP or Shadow?"
    a: "Google DLP is not in the Shadow benchmark, because commercial cloud APIs are impractical to run publicly and reproducibly. Vendor-run benchmarks such as Tonic's include it, but results vary by dataset and label mapping."
---

**Google Cloud Sensitive Data Protection (formerly Cloud DLP) is a managed API that inspects, classifies and de-identifies sensitive data, and it is the only hyperscaler PII service with documented reversible, format-preserving transforms. Shadow is an open-source privacy layer that sits in front of AI providers, replaces values with realistic, reversible fakes, and restores them in responses, streams and tool calls.** Google is a strong choice for data pipelines and storage scanning on GCP. Shadow is built specifically for live LLM traffic and runs on your own machines.

:::note[TL;DR]
Google SDP can de-identify text with format-preserving encryption and re-identify it later with `reidentifyContent`, which is genuinely reversible. It is a cloud API you call, not an LLM proxy, and FPE output looks random rather than realistic. Shadow does the round trip automatically, with realistic fakes, on the endpoint.
:::

## At a glance

| | Shadow | Google Sensitive Data Protection |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, proprietary Google Cloud service |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Google Cloud API |
| **Protection method** | Reversible realistic fakes; typed tokens for reports; optional redaction | Mask, replace, bucketing, FPE (FF1), deterministic encryption with surrogate, crypto hash |
| **Restores responses** | Yes, automatically | Yes, via `reidentifyContent`, which you call |
| **Streaming** | Yes, SSE and NDJSON | No, request/response API |
| **Tool calls** | Yes, restored locally; intent-aware | No |
| **Files** | DOCX, XLSX, PPTX in place; PDF to text | Image redaction; inspection jobs over Cloud Storage, BigQuery and Datastore |
| **Self-hosted** | Yes | No |
| **Price model** | Free | Per GB inspected and transformed |

## What Google Sensitive Data Protection does well

- **True reversibility.** `CryptoReplaceFfxFpeConfig` (FF1) and deterministic encryption with a surrogate infoType can be reversed with `reidentifyContent`, using keys you manage in Cloud KMS. Among AWS, Azure and Google PII services, only Google documents this.
- **Format preservation.** FPE keeps the length and character set, so a 16-digit number stays 16 digits, which keeps schemas and validators happy.
- **Scale and storage scanning.** Inspection jobs over Cloud Storage, BigQuery and Datastore, data profiling, and discovery make it a strong data-governance tool, not just a text API.
- **Many infoTypes.** A large built-in catalogue of detectors across many countries, plus custom infoTypes, dictionaries and regex.
- **Images.** Image redaction for text found in images, which Shadow does not do (no OCR).
- **Clear pricing.** Published per-GB prices, with the first GB of inspection each month free.

## Where Shadow is different

### Realistic fakes versus encrypted surrogates

Google FPE turns `Jane Doe` into a same-alphabet string that looks random, often prefixed with a surrogate infoType so it can be found again. A model reading it sees noise, which can hurt answers that depend on names, emails or addresses. Shadow sends a realistic value from a persistent [persona](/shadow/concepts/personas/): `Hui Zhao`, `hui.zhao@zhilan.cn`, a Luhn-valid card, a mod-97-valid IBAN. The model writes naturally, and Shadow maps it back.

### Is Google DLP an LLM proxy?

No. You call `deidentifyContent` before the model and `reidentifyContent` after it, and handle the plumbing yourself. That plumbing includes [streamed responses](/shadow/concepts/how-it-works/#streaming-hold-back) where a value can be split across events, and [tool-call arguments](/shadow/concepts/tool-calls/) that must be restored before your code acts. Shadow does all of this as a [proxy](/shadow/integrate/proxy-mode/) or [gateway](/shadow/integrate/gateway-mode/) with no code changes.

### Where detection runs

With Google, plaintext is sent to a Google Cloud API for inspection. Shadow detects locally before anything leaves the machine. If your data already lives in GCP, this matters less.

### Intent-aware treatment

Shadow's [intent policy](/shadow/concepts/intent-policy/) uses realistic fakes for conversation, typed tokens like `[PERSON_0007]` for bulk reports, and keeps real the recipient of a message and what an action run by a provider-hosted tool needs, logging each keep. Google gives you the transforms; choosing per request is up to you.

### Office files inside requests

Shadow masks DOCX, XLSX and PPTX uploads in place inside AI requests. See [file masking](/shadow/concepts/file-masking/).

### Cost

Shadow is free. Google charges per GB, which is modest for prompts but adds up for large document uploads, and adds a network round trip per call.

## When should you choose Google Sensitive Data Protection?

- You need to discover and de-identify data across Cloud Storage, BigQuery or Datastore.
- You need cryptographic, key-managed reversibility with Cloud KMS for audit reasons.
- You need image redaction.
- Your data and models already run on Google Cloud and you want a managed service.

## When should you choose Shadow?

- You want live LLM traffic masked and restored with no code changes.
- Answer quality matters and you want realistic fakes rather than encrypted strings.
- You use streaming, agents and tool calls.
- You want detection to happen on the endpoint, with a free, open-source tool.

## Can you use them together?

Yes, for different jobs. Use Google SDP to scan and de-identify data at rest in GCP, and Shadow for live traffic from developer machines, agents and services to Gemini, Vertex AI or any other provider. Shadow intercepts the Gemini API and Vertex AI hosts in proxy mode. A Rust [custom detector](/shadow/integrate/rust/#custom-detectors) could call Google's inspection API, but that sends plaintext to the cloud, which most Shadow users want to avoid.

## Migrating from a Cloud DLP de-identify / re-identify wrapper

1. Route your model client through Shadow by base URL or `HTTPS_PROXY`. For Vertex AI, add your regional endpoint under `[proxy.routes]` or rely on the intercepted `*-aiplatform.googleapis.com` hosts.
2. Remove the `deidentifyContent` and `reidentifyContent` calls around the model.
3. Port custom infoTypes and dictionaries to Shadow [dictionary terms](/shadow/concepts/detection/#dictionary-terms) or a custom detector.
4. Keep Google SDP for storage scanning if you use it.
5. Check coverage with `shadow scan` on representative prompts.

## How do they compare on accuracy?

Tonic's vendor-run benchmark (October 2025) includes Google DLP. As with every tool here, results depend heavily on dataset and label mapping. Shadow's [benchmark](/shadow/benchmarks/) publishes its code and mappings; commercial cloud APIs such as Google SDP are not included, because their terms and pricing make public, reproducible runs impractical.

## Sources

- [Re-identify free text with FPE using a surrogate (Google sample)](https://cloud.google.com/sensitive-data-protection/docs/samples/dlp-reidentify-free-text-with-fpe-using-surrogate)
- [Sensitive Data Protection pricing](https://cloud.google.com/sensitive-data-protection/pricing)
- [Sensitive Data Protection documentation](https://cloud.google.com/sensitive-data-protection/docs)
- [Tonic Textual benchmark (vendor-run)](https://www.tonic.ai/ai-model-benchmarks/textual-benchmark)

---

*Last reviewed: 2026-10-31. Google Cloud and Sensitive Data Protection are trademarks of their owner; this comparison is based on public documentation as of the review date.*
