---
title: "Shadow vs Azure AI Language PII — open-source Azure PII alternative for LLMs"
seoTitle: "Shadow vs Azure AI Language PII: open-source alternative"
description: Shadow vs Azure AI Language PII detection. Azure masks or synthesises PII one way; Shadow uses reversible fakes and restores LLM responses locally.
sidebar:
  label: Azure AI Language PII
  order: 11
keywords:
  - azure pii detection alternative
  - azure ai language pii vs shadow
  - azure text analytics pii alternative
  - azure syntheticReplacement
  - azure pii redaction pricing
  - PII redaction for LLM
  - open source PII redaction
faq:
  - q: "Is Shadow an alternative to Azure AI Language PII detection?"
    a: "For LLM traffic, yes. Azure AI Language detects PII and returns redacted text using a character mask, an entity mask or synthetic replacement (preview). It does not map values back. Shadow masks prompts with reversible fakes and restores the real values in responses."
  - q: "Can Azure PII redaction be reversed?"
    a: "Public Azure documentation does not describe re-identification for the PII redaction policies. characterMask, entityMask and syntheticReplacement are one-way. If you need the originals back, you keep your own mapping."
  - q: "Does Azure syntheticReplacement work like Shadow's fakes?"
    a: "Both produce realistic-looking values instead of placeholders. Azure's synthetic replacement is a preview feature and is not documented as reversible or consistent across requests. Shadow's fakes come from a persistent persona, stay consistent across sessions and are restored in the response."
  - q: "Does Shadow work with Azure OpenAI?"
    a: "Yes. Shadow intercepts *.openai.azure.com and Azure AI services hosts in proxy mode, and you can add your Azure endpoint as a custom gateway route."
  - q: "Which is more accurate, Azure PII or Shadow?"
    a: "Azure AI Language is not in the Shadow benchmark, because commercial cloud APIs are impractical to run publicly and reproducibly. Vendor-run benchmarks such as Tonic's include it, but results vary with dataset and label mapping."
---

**Azure AI Language PII detection is a managed Microsoft API that finds personal data in text and returns a redacted version. Shadow is an open-source privacy layer that replaces sensitive values in AI requests with realistic, reversible fakes on your own machine, then restores them in the model's answer.** Azure suits teams that want a managed, multilingual detector inside Azure. Shadow suits teams that want prompts to any provider masked and answers restored without building that pipeline.

:::note[TL;DR]
Azure detects PII and redacts it one way, with masks, entity labels or (in preview) synthetic values. Shadow detects locally, uses consistent reversible fakes from a persona, and restores them in responses, streams and tool calls. Use Azure for managed batch redaction; use Shadow in front of live LLM traffic.
:::

## At a glance

| | Shadow | Azure AI Language PII |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, proprietary Azure service |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Azure cloud API; container availability not verified here |
| **Protection method** | Reversible realistic fakes; typed tokens for reports; optional redaction | `characterMask`, `entityMask`, `syntheticReplacement` (preview) |
| **Restores responses** | Yes, automatically | No |
| **Streaming** | Yes, SSE and NDJSON | No, request/response API |
| **Tool calls** | Yes, restored locally; intent-aware | No |
| **Files** | DOCX, XLSX, PPTX in place; PDF to text | Document PII API |
| **Self-hosted** | Yes | Not verified (Azure documents containers for some Language features) |
| **Price model** | Free | Usage-based, per text record |

## What Azure AI Language PII does well

- **Managed and multilingual.** No models to host, many entity categories and languages, and Microsoft-run updates.
- **Several redaction styles.** Character masking (`****`), entity masking (`[PERSON]`) and, since the `2025-11-15-preview` API version, synthetic replacement with realistic values, which is closer to how Shadow works.
- **Document support.** A document PII API handles whole files, which suits batch redaction workflows.
- **Azure governance.** It fits into Azure identity, networking, monitoring and regional data residency choices that many enterprises already use.
- **Conversation PII.** Azure also offers PII detection for conversation transcripts, useful in contact-centre scenarios.

## Where Shadow is different

### Can Azure put the real values back in the answer?

No re-identification is documented for the redaction policies. If you send a redacted prompt to a model, the answer refers to `[PERSON]` or a synthetic name, and connecting it back to the original is your job. Shadow keeps an [encrypted vault](/shadow/concepts/vault/) of real-to-fake pairs and restores them in JSON, [streamed responses](/shadow/concepts/how-it-works/#streaming-hold-back) and [tool-call arguments](/shadow/concepts/tool-calls/).

### Consistent fakes across requests

Synthetic replacement in a one-off API call is not documented as consistent between calls. Shadow's [persona](/shadow/concepts/personas/) gives the same customer the same fake name today and next week, so long conversations and agents stay coherent. Fakes are format-preserving: emails stay emails at a consistent domain, cards pass Luhn, IBANs pass mod-97.

### Detection happens on your machine

With Azure, the plaintext is sent to an Azure endpoint to be scanned. Shadow detects locally before the request leaves. If Azure is already your trusted processor, this difference matters less.

### One step, not two round trips

A pipeline built on Azure PII calls the Language API, then the model, then your own restore code. Shadow is a single local hop as a [proxy](/shadow/integrate/proxy-mode/) or [gateway](/shadow/integrate/gateway-mode/), and adds an [intent policy](/shadow/concepts/intent-policy/) that picks fakes, typed tokens or keep per value.

### Office files in uploads

Shadow masks DOCX, XLSX and PPTX attachments in place inside requests, plus base64 and multipart uploads. See [file masking](/shadow/concepts/file-masking/).

## When should you choose Azure AI Language PII?

- You want a managed, multilingual detector inside Azure.
- You redact documents or transcripts in batch and don't need the originals back.
- You need Azure's compliance attestations and regional hosting for the detector itself.
- Your team prefers to buy rather than run software.

## When should you choose Shadow?

- You want live LLM traffic masked and responses restored automatically.
- You use streaming, agents or tool calls.
- You want consistent fakes across sessions.
- You want detection to happen locally, with a free, open-source tool.

## Can you use them together?

Shadow doesn't call Azure AI Language as a detector out of the box. You can implement Shadow's `Detector` trait in Rust to call any detection API ([custom detectors](/shadow/integrate/rust/#custom-detectors)), but sending plaintext to a cloud detector undoes part of the point of local masking. A more common split: Azure PII for batch redaction of stored documents, Shadow for live traffic to Azure OpenAI and other providers.

## Migrating from an Azure PII pipeline in front of an LLM

1. Route your model client through Shadow, by base URL ([gateway mode](/shadow/integrate/gateway-mode/)) or `HTTPS_PROXY` ([proxy mode](/shadow/integrate/proxy-mode/)). Add Azure OpenAI deployments under `[proxy.routes]`.
2. Remove the pre-call redaction and any post-call substitution code.
3. Add organisation names, customers and codenames as [dictionary terms](/shadow/concepts/detection/#dictionary-terms).
4. For extra recall on names, enable the [NER sidecar or Presidio](/shadow/concepts/detection/).
5. Compare with `shadow scan` on representative content before switching.

## How do they compare on accuracy?

Tonic's vendor-run benchmark (October 2025) includes Azure. Numbers swing widely with datasets and label mapping, so treat any single figure with care. Shadow's [benchmark](/shadow/benchmarks/) publishes its code and mappings; commercial cloud APIs such as Azure are not included, because their terms and pricing make public, reproducible runs impractical.

## Sources

- [Azure AI Language: redact text PII](https://docs.azure.cn/en-us/ai-services/language-service/personally-identifiable-information/how-to/redact-text-pii)
- [Azure AI Language PII overview](https://learn.microsoft.com/azure/ai-services/language-service/personally-identifiable-information/overview)
- [Azure AI Language pricing](https://azure.microsoft.com/pricing/details/cognitive-services/language-service/)
- [Tonic Textual benchmark (vendor-run)](https://www.tonic.ai/ai-model-benchmarks/textual-benchmark)

---

*Last reviewed: 2026-10-31. Azure and Microsoft are trademarks of their owner; this comparison is based on public documentation as of the review date.*
