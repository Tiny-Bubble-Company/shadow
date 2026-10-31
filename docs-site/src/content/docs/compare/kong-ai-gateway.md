---
title: "Shadow vs Kong AI Gateway — open-source alternative to Kong ai-sanitizer"
seoTitle: "Shadow vs Kong AI Gateway: an ai-sanitizer alternative"
description: Shadow vs Kong AI Gateway's ai-sanitizer plugin. Both can use synthetic values and restore; Shadow is open source, local and streams restores.
sidebar:
  label: Kong AI Gateway
  order: 17
keywords:
  - kong ai gateway alternative
  - kong ai sanitizer alternative
  - kong vs shadow
  - kong ai-sanitizer
  - kong ai gateway pii
  - kong enterprise pricing
  - LLM gateway PII masking
  - synthetic data PII LLM
faq:
  - q: "Is Shadow an alternative to Kong's ai-sanitizer plugin?"
    a: "For PII masking, yes. Kong's ai-sanitizer is an Enterprise plugin that replaces PII with placeholders or synthetic values and can restore them. Shadow does the same job as an open-source endpoint proxy, gateway or Rust library, with streaming and tool-call restoration. Kong AI Gateway also provides routing, rate limiting and other gateway features that Shadow does not."
  - q: "Is Kong ai-sanitizer free?"
    a: "No. The ai-sanitizer plugin is available in Kong Enterprise and depends on a private kong/ai-pii-service container image. Shadow is free under Apache-2.0."
  - q: "Can Kong ai-sanitizer restore the original values?"
    a: "Yes. With the restore option, values sanitized in the request are put back in the response. Kong's docs require Gateway 3.10 or later, and sanitizing responses needs 3.12 or later."
  - q: "Does Kong ai-sanitizer work with streaming?"
    a: "Kong's plugin documentation does not describe restoring sanitized values in streamed responses. Shadow restores SSE and NDJSON streams, including fakes split across events."
  - q: "Which languages does Kong ai-sanitizer support?"
    a: "The PII service ships language-specific images for English, Spanish, French, German, Italian, Japanese, Korean, Portuguese and Turkish. Shadow's multilingual recall depends on its detectors and is measured on the benchmarks page."
---

**Kong AI Gateway is Kong's set of AI plugins for its API gateway, and its Enterprise `ai-sanitizer` plugin replaces PII in prompts with placeholders or synthetic values and can restore them. Shadow is an open-source privacy layer that does the same kind of reversible masking with realistic, format-preserving fakes, on the endpoint or as a sidecar, and restores values in streams and tool calls.** Kong's approach is the closest commercial analogue to Shadow's; the main differences are where it runs, the licence, and documented streaming support.

:::note[TL;DR]
If you already run Kong Enterprise and want PII handling centrally in the gateway, `ai-sanitizer` fits naturally. If you want reversible synthetic masking without an Enterprise licence, on developer machines as well as servers, with documented streaming and tool-call restoration, Shadow fits. Shadow can also sit in front of Kong.
:::

## At a glance

| | Shadow | Kong AI Gateway (`ai-sanitizer`) |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Kong Gateway OSS exists; `ai-sanitizer` is Enterprise-only |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Self-hosted Kong Gateway plugin plus a private `kong/ai-pii-service` container |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Placeholder or synthetic mode; credentials replaced with `#` |
| **Restores responses** | Yes, automatically | Yes, with `restore` (requests sanitized on input) |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Not documented |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Not documented |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Not documented |
| **Self-hosted** | Yes, single binary, no account | Yes |
| **Price model** | Free | Kong Enterprise licence |

## What Kong AI Gateway does well

- **A full API gateway.** Kong brings routing, authentication, rate limiting, AI-specific plugins (prompt guard, prompt templates, semantic caching, multi-LLM routing) and a mature plugin ecosystem.
- **Synthetic mode with restore.** The [`ai-sanitizer`](https://developer.konghq.com/plugins/ai-sanitizer/) plugin can replace values with synthetic data rather than placeholders and restore them afterwards, the same principle Shadow uses.
- **Multilingual images.** The PII service ships images for nine languages: English, Spanish, French, German, Italian, Japanese, Korean, Portuguese and Turkish.
- **Central policy.** Platform teams configure sanitization once in the gateway for every consumer.
- **Vault option.** A separate [Skyflow plugin](https://developer.konghq.com/plugins/skyflow-ai-data-control/) adds vault-backed tokenisation and re-identification for teams that want a managed vault.

## Where Shadow is different

### Open source and no Enterprise licence

`ai-sanitizer` requires Kong Enterprise, Gateway 3.10 or later (3.12 for response sanitization) and a private PII-service image that Kong documents as needing at least 600 MB of RAM. Shadow is Apache-2.0, a single binary, with no account.

### Does Kong restore values in streams and tool calls?

Kong's plugin docs do not describe restoring sanitized values in streamed responses or tool-call arguments. Shadow restores fakes in [SSE and NDJSON streams](/shadow/concepts/how-it-works/#streaming-hold-back) for OpenAI, Anthropic, Gemini and other formats, even when a fake is split across events, and restores [tool-call arguments](/shadow/concepts/tool-calls/) before your code runs the tool.

### Format-preserving, persona-consistent fakes

Shadow's fakes are drawn from a persistent [persona](/shadow/concepts/personas/): card numbers keep a valid Luhn digit, IBANs pass mod-97, emails use the persona's domain, and the same value gets the same fake across sessions. Kong's documentation describes synthetic values but does not document checksum validity or cross-session consistency.

### Endpoint coverage

A gateway only sees traffic routed to it. Shadow also runs on laptops as an [HTTPS proxy](/shadow/integrate/proxy-mode/), so IDE agents and desktop apps calling a [supported provider API](/shadow/reference/providers/) directly are covered. It masks [Office uploads](/shadow/concepts/file-masking/) and applies an [intent policy](/shadow/concepts/intent-policy/) per request.

## When should you choose Kong AI Gateway?

- You already run Kong Enterprise and want PII handling in the same control plane as routing and auth.
- You need language-specific detection in one of Kong's nine supported languages today.
- Central gateway policy matters more than endpoint coverage.

## When should you choose Shadow?

- You want reversible synthetic masking without an Enterprise licence.
- Streaming and agent tool calls must be restored.
- Developer machines and tools that call providers directly need coverage.

## Can you use Kong with Shadow?

Yes. Run Shadow as a sidecar in front of Kong, or point Kong's AI upstreams at Shadow's gateway routes so Shadow masks just before traffic leaves your network. Kong keeps routing, auth and rate limits. Enable PII sanitization in only one of the two layers, otherwise the second sees synthetic values it did not create. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/).

## Migrating from Kong ai-sanitizer

1. Keep Kong for routing and other AI plugins.
2. Disable `ai-sanitizer` on the routes Shadow will cover.
3. Deploy Shadow as a sidecar and point the AI upstream URLs at `http://127.0.0.1:8877/gw/<provider>/...`, with custom routes for Azure or Bedrock ([gateway mode](/shadow/integrate/gateway-mode/)).
4. Add your organisation names and codenames as [dictionary terms](/shadow/concepts/detection/#dictionary-terms).
5. Test streaming, tool calls and non-English prompts with `shadow scan` before switching.

## How do they compare on accuracy?

Kong does not publish detection accuracy for its PII service. Shadow's [benchmark](/shadow/benchmarks/) publishes its methodology, code and results; Kong's Enterprise-only plugin was not available for independent testing.

## Sources

- [Kong ai-sanitizer plugin](https://developer.konghq.com/plugins/ai-sanitizer/)
- [Kong Skyflow AI data control plugin](https://developer.konghq.com/plugins/skyflow-ai-data-control/)
- [Kong AI Gateway](https://konghq.com/products/kong-ai-gateway)
- [Shadow file masking](/shadow/concepts/file-masking/)

---

*Last reviewed: 2026-10-31. Kong is a trademark of its owner; this comparison is based on public documentation as of the review date.*
