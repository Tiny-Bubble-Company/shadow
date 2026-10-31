---
title: "LLM gateways with PII masking compared (2026)"
description: "Which LLM and AI gateways mask PII, and which restore it? LiteLLM, Portkey, Kong, Cloudflare, Bedrock Guardrails and Shadow compared."
sidebar:
  label: LLM gateways with PII masking
  order: 3
keywords:
  - LLM gateway PII masking
  - AI gateway PII redaction
  - LiteLLM PII masking
  - Kong AI gateway PII
  - Portkey PII redaction
  - Cloudflare AI Gateway DLP
  - PII redaction for LLM
faq:
  - q: Which AI gateways can mask PII in prompts?
    a: "LiteLLM (via Presidio, Lakera, Bedrock and other guardrails), Portkey (via partner guardrails), Kong AI Gateway (ai-sanitizer, Enterprise), Amazon Bedrock Guardrails (anonymize) and Shadow mask PII. Cloudflare AI Gateway DLP flags or blocks rather than redacts."
  - q: Which gateways restore PII in the response?
    a: "Kong's ai-sanitizer has a restore option, LiteLLM's Presidio guardrail can restore placeholders with output_parse_pii, and Shadow restores realistic fakes in JSON, streams and tool calls. Portkey's docs describe its PII redaction as one way, and Bedrock Guardrails and Cloudflare do not restore."
  - q: Do gateway PII features work with streaming?
    a: "It varies. Cloudflare buffers the full response to scan it. A LiteLLM issue reported missing unmasking on Anthropic's native streaming path. Kong and Portkey do not document streaming behaviour for PII. Shadow restores SSE and NDJSON streams with a hold-back of a few characters."
  - q: Can I use Shadow with my existing LLM gateway?
    a: "Yes. Run Shadow on the client or as a sidecar so traffic is masked before it reaches the gateway, or point the gateway's upstream at Shadow's /gw/ routes. Keep the gateway for routing, keys, budgets and observability."
  - q: Is there a free open-source gateway with reversible PII masking?
    a: "LiteLLM's open-source proxy with the Presidio guardrail restores placeholders. Shadow is Apache-2.0 and restores realistic fakes, but it is a privacy layer rather than a full routing gateway."
---

**Most LLM gateways can redact or block PII, but few restore it in the response, and fewer handle streaming and tool calls.** LiteLLM and Kong can restore placeholders or synthetic values; Portkey, Bedrock Guardrails and Cloudflare are one way. Shadow is not a routing gateway, but it sits in the same position for privacy: it masks with realistic fakes and restores responses, streams and tool calls, and it works alongside any gateway.

## What an AI gateway does, and where PII fits

AI gateways centralise model access: one endpoint for many providers, API keys, rate limits, budgets, caching, fallbacks, logging and guardrails. PII handling is usually one guardrail among many, often delegated to Presidio or a cloud API.

That matters because the gateway is typically a **shared service**. Prompts travel in clear text from every client to the gateway, and the gateway's logs may hold them. Masking at the gateway protects the provider hop, not the hop before it.

## Comparison

| Gateway | Licence | PII action | Restores | Streaming restore | Tool calls | Self-hosted |
|---|---|---|---|---|---|---|
| [LiteLLM](/shadow/compare/litellm/) | MIT core; enterprise features paid | Block or redact via Presidio, Lakera, Bedrock and others | Placeholders (`output_parse_pii`) | Reported gaps on some paths ([#22821](https://github.com/BerriAI/litellm/issues/22821)) | No | Yes |
| [Portkey](/shadow/compare/portkey/) | Gateway MIT; guardrails in cloud | Redact via partner guardrails | No (docs: one way) | Not documented | No | Gateway only |
| [Kong AI Gateway](/shadow/compare/kong-ai-gateway/) | `ai-sanitizer` is Enterprise-only | Placeholder or synthetic | Yes (`restore`) | Not documented | No | Yes |
| [Cloudflare AI Gateway](/shadow/compare/cloudflare-ai-gateway/) | Proprietary | Flag or block | No | Buffers response to scan | No | No |
| [Bedrock Guardrails](/shadow/compare/aws-comprehend-bedrock-guardrails/) | Proprietary | Block or `{NAME}` tags | No | Not verified | No | No |
| Agent Router (formerly Envoy AI Gateway) | Apache-2.0 | Guardrail hooks only in OSS; PII in Tetrate's enterprise offering | n/a | n/a | MCP routing | Yes |
| **Shadow** | Apache-2.0 | Realistic reversible fakes; tokens for reports; optional redact | Yes | Yes | Yes, intent-aware | Yes |

## Gateway by gateway

### LiteLLM

The most popular open-source gateway. Its Presidio guardrail masks prompts and, with `output_parse_pii`, restores placeholders. An issue reported that unmasking was missing on Anthropic's native path and in SSE streaming; it was closed in June 2026, and the status of the replacement fixes is not verified. Great for routing and budgets. [Shadow vs LiteLLM](/shadow/compare/litellm/).

### Portkey

A popular gateway with an MIT core. PII redaction comes from partner guardrails (Pangea, Bedrock, Azure, Patronus), and Portkey's own docs describe the transformation as one way. Palo Alto Networks announced its acquisition in April 2026. [Shadow vs Portkey](/shadow/compare/portkey/).

### Kong AI Gateway

Kong's Enterprise `ai-sanitizer` plugin is the closest commercial analogue to Shadow: it can replace PII with placeholders or synthetic values and restore them. It requires Gateway 3.10 or later (3.12 for response sanitization) and a private PII service image supporting nine languages. Streaming behaviour is not documented. Kong also offers a Skyflow plugin for vault-backed tokens. [Shadow vs Kong](/shadow/compare/kong-ai-gateway/).

### Cloudflare AI Gateway

Cloudflare's DLP for AI Gateway can pass, flag or block based on Cloudflare One DLP profiles; it does not redact. Scanning responses buffers the stream, which adds time to first token. Ideal if you already use Cloudflare and want visibility and blocking. [Shadow vs Cloudflare](/shadow/compare/cloudflare-ai-gateway/).

### Amazon Bedrock Guardrails

Native to Bedrock: block or anonymise PII into type tags, plus custom regex. No restoration. Note that Shadow forwards SigV4-signed Bedrock requests unmasked because changing the body would break the signature, so Guardrails remain the right control for IAM-signed Bedrock traffic. [Shadow vs AWS](/shadow/compare/aws-comprehend-bedrock-guardrails/).

## How does Shadow fit with a gateway?

Shadow is not a router: it has no budgets, caching, fallbacks or multi-tenant key management. It is a privacy layer that can sit **in front of** a gateway on each client, or **between** the gateway and providers.

| Pattern | How | Benefit |
|---|---|---|
| Shadow on clients, gateway central | Clients use Shadow's [proxy](/shadow/integrate/proxy-mode/) or [gateway mode](/shadow/integrate/gateway-mode/) with a custom route to your gateway | Detected values are masked before they leave the endpoint, so gateway logs hold fakes instead |
| Shadow as a sidecar next to the gateway | Gateway's upstream points at Shadow's `/gw/<provider>/` routes in the same pod | One masking point for all services; see [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |
| Shadow inside your service | Embed `shadow-core` in a [Rust service](/shadow/integrate/rust/) | No extra hop |

In the first pattern, the gateway's own PII guardrail becomes a second line of defence.

## How to choose

- **You need routing, budgets and observability first:** pick a gateway (LiteLLM or Kong self-hosted, Portkey or Cloudflare managed), then decide on PII.
- **You need answers that mention the real values, including streams and agents:** add Shadow, or use Kong's `ai-sanitizer` if you are on Kong Enterprise and do not need streaming guarantees.
- **You only need to block known-bad data:** Cloudflare DLP or Bedrock Guardrails may be enough.

Detection accuracy and round-trip exactness are measured on the [benchmarks page](/shadow/benchmarks/): on the held-out set, Shadow's built-in rules leaked 3.08% of sensitive values and round-tripped 100% of documents exactly. The gateways themselves are not benchmarked there.

## Sources

- [LiteLLM Presidio PII masking tutorial](https://docs.litellm.ai/docs/tutorials/presidio_pii_masking)
- [Portkey PII redaction](https://portkey.ai/docs/product/guardrails/pii-redaction)
- [Kong ai-sanitizer plugin](https://developer.konghq.com/plugins/ai-sanitizer/)
- [Cloudflare AI Gateway DLP](https://developers.cloudflare.com/ai-gateway/features/dlp/)
- [Amazon Bedrock Guardrails components](https://docs.aws.amazon.com/bedrock/latest/userguide/guardrails-components.html)
- [Agent Router](https://github.com/theagentrouter/agent-router)

---

*Last reviewed: 2026-10-31. All product names are trademarks of their respective owners; this guide is based on public documentation as of the review date.*
