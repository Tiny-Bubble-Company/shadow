---
title: "Shadow vs LiteLLM — LiteLLM PII masking alternative with streaming restore"
description: Shadow vs LiteLLM proxy for PII masking. LiteLLM routes models and runs guardrails; Shadow adds reversible fakes and streaming, tool-call restore.
sidebar:
  label: LiteLLM
  order: 21
keywords:
  - litellm alternative
  - litellm vs shadow
  - litellm pii masking
  - litellm presidio
  - litellm output_parse_pii
  - litellm guardrails
  - LLM gateway PII redaction
  - PII redaction for LLM
faq:
  - q: "Is Shadow an alternative to LiteLLM?"
    a: "Not as a gateway. LiteLLM routes requests to 100+ model providers with keys, budgets, fallbacks and logging. Shadow does not route or budget; it masks sensitive values with reversible fakes and restores them. For PII masking specifically, Shadow can replace LiteLLM's Presidio guardrail, or sit in front of LiteLLM."
  - q: "Can LiteLLM unmask PII in responses?"
    a: "Yes, partly. With the Presidio guardrail and output_parse_pii enabled, LiteLLM replaces placeholders in the response with the original values. A GitHub issue (#22821) reported that unmasking was missing on the Anthropic native path and in SSE streaming; it was closed in June 2026, and whether a fix has shipped for every path was not verified."
  - q: "Does Shadow work with LiteLLM?"
    a: "Yes. Point LiteLLM's provider base URLs at Shadow's gateway, or run LiteLLM behind Shadow's HTTPS proxy. Shadow masks the request before it leaves the host and restores the response before LiteLLM returns it."
  - q: "Is LiteLLM free?"
    a: "The LiteLLM proxy core is open source under MIT. Some features live in an enterprise directory under a commercial licence. Shadow is free and open source under Apache-2.0."
  - q: "Which guardrails does LiteLLM support for PII?"
    a: "LiteLLM can call several guardrail providers, including Presidio, Lakera, Bedrock Guardrails and others. Most of them block or redact; restoration is only available with the Presidio integration."
---

**LiteLLM is an open-source AI gateway and SDK that gives one OpenAI-compatible API over many model providers, with keys, budgets, fallbacks and guardrail hooks. Shadow is an open-source privacy layer that replaces sensitive values with realistic, reversible fakes before a request leaves, and restores them in responses, streams and tool calls.** They are different products that overlap on PII masking. Many teams will keep LiteLLM for routing and use Shadow for masking.

:::note[TL;DR]
LiteLLM masks PII through guardrail integrations, most often Presidio, with placeholders and optional unmasking via `output_parse_pii`. Shadow uses consistent, format-preserving fakes and restores them in JSON, SSE streams and tool-call arguments. Keep LiteLLM as your gateway and put Shadow in front of it, or use Shadow's own gateway if you don't need routing.
:::

## At a glance

| | Shadow | LiteLLM |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Core MIT; enterprise directory under a commercial licence |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Self-hosted Python proxy server; Python SDK |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Block or redact via guardrail providers (Presidio, Lakera, Bedrock and others) |
| **Restores responses** | Yes, automatically | With Presidio and `output_parse_pii` (placeholders) |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Reported gaps on some paths ([#22821](https://github.com/BerriAI/litellm/issues/22821)); current status not verified |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Not documented for PII |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Not documented for PII |
| **Self-hosted** | Yes, single binary, no account | Yes |
| **Price model** | Free | Free core; Enterprise paid |

## What LiteLLM does well

- **Unified API over many providers.** One OpenAI-compatible endpoint for OpenAI, Anthropic, Gemini, Bedrock, Azure, Vertex, local models and many more. Shadow does not translate between APIs.
- **Platform features.** Virtual keys, per-team budgets, rate limits, fallbacks, retries, load balancing, caching and spend tracking. These are what most teams adopt LiteLLM for.
- **Guardrail plug-ins.** LiteLLM can chain [Presidio](https://docs.litellm.ai/docs/tutorials/presidio_pii_masking), Lakera, Bedrock Guardrails, Aporia and others, configured per model or per key.
- **Observability.** Callbacks to Langfuse, Helicone, OpenTelemetry and other tools.
- **Large community.** It is one of the most widely deployed open-source AI gateways and moves quickly.

## Where Shadow is different

### Does LiteLLM restore PII in streamed responses?

LiteLLM's Presidio integration can unmask placeholders in responses when `output_parse_pii` is enabled. Issue [#22821](https://github.com/BerriAI/litellm/issues/22821) reported that this did not happen on the Anthropic native path and in SSE streaming. It was closed in June 2026; whether every path is fixed was not verified at the review date, so test your own clients. Shadow restores fakes in [SSE and NDJSON streams](/shadow/concepts/how-it-works/#streaming-hold-back) for OpenAI, Anthropic, Gemini, Cohere and Ollama-style formats, including fakes split across events.

### Placeholders or realistic fakes?

With Presidio, the model sees `<PERSON>` or similar placeholders. Shadow sends realistic, consistent fakes from a [persona](/shadow/concepts/personas/): a valid email at a fake company domain, a Luhn-valid card, a German IBAN that passes mod-97. The model reasons about plausible data, so answers stay useful, and the same customer keeps the same fake across sessions.

### Tool calls and intent

Shadow restores [tool-call arguments](/shadow/concepts/tool-calls/) locally, and its [intent policy](/shadow/concepts/intent-policy/) handles bulk reports with typed tokens and provider-hosted actions with an explicit, logged `keep`.

### Where masking happens

LiteLLM is usually a shared server, so prompts travel to it in clear text before masking. Shadow can run on the endpoint itself, so data is masked before it leaves the machine, or as a sidecar next to a service. It also masks [Office uploads](/shadow/concepts/file-masking/) in place.

## When should you choose LiteLLM?

- You need one API over many model providers, with budgets, keys, fallbacks and spend tracking.
- You want to chain several guardrail vendors centrally.
- Block or placeholder redaction is enough for your traffic.

## When should you choose Shadow?

- Masked values must be restored reliably, including in streams and tool calls.
- You want realistic fakes so answer quality stays high.
- Data should be masked on the endpoint, before it reaches any shared server.
- You don't need multi-provider routing and want a single binary with no database.

## Can you use LiteLLM with Shadow?

Yes. Two common layouts:

1. **Shadow behind LiteLLM.** Set each LiteLLM provider's `api_base` to Shadow's gateway (`http://127.0.0.1:8877/gw/openai/v1`, `/gw/anthropic`, and so on) or run the LiteLLM container with `HTTPS_PROXY` pointing at a Shadow sidecar. LiteLLM keeps routing, keys and budgets; Shadow masks what leaves and restores what comes back.
2. **Shadow on the client.** Developers run Shadow locally and point their tools at LiteLLM through Shadow, so prompts are masked before they reach the shared gateway.

In either case, disable LiteLLM's Presidio masking for those routes so the two layers don't fight. See [integrating any endpoint](/shadow/integrate/any-endpoint/) and [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/).

## Migrating from LiteLLM's Presidio guardrail

1. Keep LiteLLM for routing.
2. Remove the Presidio guardrail and `output_parse_pii` from the affected models.
3. Point those providers' `api_base` at Shadow's gateway routes, or add custom routes for Azure, Bedrock and self-hosted endpoints ([gateway mode](/shadow/integrate/gateway-mode/)).
4. If you had custom Presidio recognizers, point Shadow's `[presidio]` section at your Analyzer.
5. Test streaming and tool-call flows end to end before switching production traffic.

## How do they compare on accuracy?

With the Presidio guardrail, LiteLLM's detection accuracy is Presidio's. Shadow's [benchmark](/shadow/benchmarks/) includes LiteLLM with Presidio as a round-trip and streaming baseline, with published code.

## Sources

- [LiteLLM Presidio PII masking tutorial](https://docs.litellm.ai/docs/tutorials/presidio_pii_masking)
- [LiteLLM issue #22821](https://github.com/BerriAI/litellm/issues/22821)
- [LiteLLM on GitHub](https://github.com/BerriAI/litellm)
- [Shadow gateway mode](/shadow/integrate/gateway-mode/)

---

*Last reviewed: 2026-10-31. LiteLLM is a trademark of its owner; this comparison is based on public documentation as of the review date.*
