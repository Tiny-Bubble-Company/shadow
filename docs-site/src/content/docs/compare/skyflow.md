---
title: "Shadow vs Skyflow — open-source Skyflow LLM Privacy Vault alternative"
seoTitle: "Shadow vs Skyflow LLM Privacy Vault: open-source option"
description: Shadow vs Skyflow LLM Privacy Vault. Compare vault tokens vs realistic fakes, re-identification, files, streaming, deployment and pricing.
sidebar:
  label: Skyflow
  order: 35
keywords:
  - skyflow alternative
  - skyflow vs shadow
  - skyflow llm privacy vault alternative
  - open source skyflow
  - skyflow pricing
  - privacy vault for LLM
  - PII tokenization for LLM
  - AI DLP
faq:
  - q: "Is Shadow an alternative to Skyflow's LLM Privacy Vault?"
    a: "For protecting LLM traffic, yes. Both replace sensitive values before the model sees them and restore them afterwards. Skyflow is a managed data privacy vault with governed re-identification across your systems; Shadow is a free, Apache-2.0 layer that runs locally as a proxy, gateway or Rust library, with its encrypted vault on the machine."
  - q: "Is Skyflow open source?"
    a: "No. Skyflow is a commercial SaaS platform with open-source SDKs. Shadow is fully open source under Apache-2.0."
  - q: "How much does Skyflow cost?"
    a: "Skyflow pricing is by quote; public pages we reviewed do not list prices. Shadow is free."
  - q: "Can Skyflow de-identify files?"
    a: "Yes. Skyflow Detect de-identifies files including images, PDFs, audio and Office documents through its SaaS API. Shadow masks DOCX, XLSX and PPTX in place and converts PDFs to masked text, but does not do OCR or audio."
  - q: "Where is the data stored with Skyflow versus Shadow?"
    a: "Skyflow stores sensitive values in its vault service and governs who can re-identify them. Shadow keeps the real-to-fake mapping in a ChaCha20-Poly1305 encrypted file on the machine running it, with no hosted component."
---

**Skyflow is a commercial data privacy vault: sensitive values are stored in Skyflow, replaced with tokens, and re-identified under fine-grained policy. Its LLM Privacy Vault and Detect APIs apply this to AI prompts and files. Shadow is an open-source (Apache-2.0) privacy layer that replaces values with realistic, reversible fakes on the endpoint and restores them in responses, streams and tool calls.** Choose Skyflow when you want a central, governed vault shared across many systems. Choose Shadow when you want a free, local layer in front of AI providers with no external service.

:::note[TL;DR]
Skyflow centralises sensitive data in a managed vault and governs re-identification, with strong file de-identification (images, PDF, audio, Office). Shadow keeps the mapping in a local encrypted vault and focuses on the live AI traffic path: persona-consistent fakes, streaming and tool-call restoration, and an endpoint proxy. Skyflow is quote-priced SaaS; Shadow is free and self-hosted.
:::

## At a glance

| | Shadow | Skyflow |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, proprietary (SDKs are public) |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Vault SaaS; LLM Privacy Vault and Detect APIs; Kong plugin |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Vault tokens, polymorphic encryption |
| **Restores responses** | Yes, automatically | Yes, policy-governed re-identification |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Not documented in the docs we reviewed |
| **Tool calls** | Yes, arguments restored locally; intent-aware | Not documented |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Images, PDF, audio, Office documents (Detect API) |
| **Self-hosted** | Yes, single binary, no account | No, SaaS |
| **Price model** | Free | Quote |

## What Skyflow does well

- **Governed re-identification.** Skyflow's core idea is that sensitive data lives in one vault, and policies decide who can see what, in clear, masked or tokenised form. That is a strong model for organisations where many services and people need different views of the same data.
- **One vault across systems.** The same customer record can be tokenised consistently across your databases, analytics and LLM workflows, not only in prompts.
- **File de-identification beyond text.** Skyflow Detect handles images, PDFs, audio and Office documents. Shadow does not do OCR or audio.
- **Gateway integration.** A [Kong plugin](https://developer.konghq.com/plugins/skyflow-ai-data-control/) applies Skyflow's vault-backed de-identification and re-identification at an AI gateway.
- **Enterprise product.** Vendor support, a managed service and the governance features that procurement and audit teams look for.

## Where Shadow is different

### Is Shadow a free alternative to Skyflow?

Yes. Shadow is Apache-2.0, free, and runs without an account. There is no external vault to call, so prompts are masked on the machine before the request leaves it, and the mapping is never stored with a third party.

### Realistic fakes instead of tokens

Vault tokens are safe and reversible, but to a model they look like tokens. Shadow's default is a realistic, format-preserving fake from a persistent [persona](/shadow/concepts/personas/): a valid email at a coherent fake company, a Luhn-valid card, a mod-97-valid IBAN. Models write naturally about realistic data. When opaque IDs are better, for example bulk reports, Shadow's [intent policy](/shadow/concepts/intent-policy/) switches to typed tokens such as `[PERSON_0007]`.

### Streams and agents

Shadow restores fakes in [server-sent event streams](/shadow/concepts/how-it-works/#streaming-hold-back) as they arrive, including fakes split across events, and in [tool-call arguments](/shadow/concepts/tool-calls/) before your code runs the tool. Public Skyflow docs we reviewed do not describe streaming re-identification or tool-call handling in detail.

### Endpoint coverage without code changes

Skyflow is integrated through APIs, SDKs or a gateway plugin. Shadow can also intercept traffic as an HTTPS proxy, so desktop tools, IDE assistants and CLIs that you don't control are covered when they call a [supported AI API](/shadow/reference/providers/). See [proxy mode](/shadow/integrate/proxy-mode/).

### Local vault

Shadow's [vault](/shadow/concepts/vault/) is a ChaCha20-Poly1305 encrypted file in Shadow's data directory. That is simpler, but it also means Shadow does not offer Skyflow's central, role-based governance across an organisation.

## When should you choose Skyflow?

- You want a central privacy vault for customer data across applications, not only AI traffic.
- You need role-based, audited re-identification for many teams.
- You need de-identification of images, audio or scanned documents.
- You prefer a managed service with vendor support.

## When should you choose Shadow?

- You want AI prompts masked on the endpoint, with no third-party service in the path.
- You rely on streaming responses and agent tool calls.
- You want realistic fakes that keep model answers useful.
- You need a free, open-source component you can audit and run anywhere.

## Using them together

They solve adjacent problems. A team might keep Skyflow as the system of record for customer PII in its applications, and run Shadow on developer laptops and CI runners so code assistants and ad-hoc scripts send fakes instead of the values Shadow detects. If the data reaching a prompt is already Skyflow-tokenised, Shadow will treat the tokens as ordinary text and mask anything else it detects.

## Migrating LLM traffic from Skyflow

1. Identify which LLM calls go through Skyflow's de-identify and re-identify APIs.
2. Run Shadow as a [gateway](/shadow/integrate/gateway-mode/) sidecar and point those calls at `http://127.0.0.1:8877/gw/<provider>/...`.
3. Add your organisation names, customers and codenames as [dictionary terms](/shadow/concepts/detection/#dictionary-terms), and enable the optional NER sidecar for free-text names.
4. Test with `shadow scan` on representative prompts and attachments.
5. Keep Skyflow for data at rest and non-AI systems if you use it there.

## How do they compare on accuracy?

Skyflow does not publish detection benchmarks that we found, and we could not test it without a commercial account. Shadow's [benchmark](/shadow/benchmarks/) publishes methodology, datasets and code so you can run it against your own tools.

## Sources

- [Skyflow](https://www.skyflow.com/)
- [Skyflow Detect API sample (Node SDK)](https://raw.githubusercontent.com/skyflowapi/skyflow-node-sdk/HEAD/samples/detect-api/get-detect-run.ts)
- [Kong Skyflow AI Data Control plugin](https://developer.konghq.com/plugins/skyflow-ai-data-control/)
- [Shadow vault](/shadow/concepts/vault/)

---

*Last reviewed: 2026-10-31. Skyflow is a trademark of its owner; this comparison is based on public documentation as of the review date.*
