---
title: "Shadow vs Cloudflare AI Gateway DLP — alternative that masks instead of blocking"
description: Shadow vs Cloudflare AI Gateway DLP. Cloudflare flags or blocks prompts with sensitive data; Shadow masks with reversible fakes and restores answers.
sidebar:
  label: Cloudflare AI Gateway
  order: 12
keywords:
  - cloudflare ai gateway alternative
  - cloudflare ai gateway dlp
  - cloudflare ai gateway vs shadow
  - cloudflare ai gateway pii
  - cloudflare dlp pricing
  - LLM gateway PII masking
  - AI DLP
  - self-hosted AI gateway
faq:
  - q: "Can Cloudflare AI Gateway redact PII?"
    a: "Cloudflare AI Gateway's DLP feature can flag or block requests and responses that match DLP profiles. Its documentation describes no redaction or replacement action. Shadow replaces sensitive values with reversible fakes so the request still goes through."
  - q: "Is Shadow an alternative to Cloudflare AI Gateway?"
    a: "For data protection, yes; as a gateway, no. Cloudflare AI Gateway also provides caching, rate limiting, analytics and logging at Cloudflare's edge. Shadow focuses on masking and restoring sensitive values and runs on your own machines."
  - q: "Does Cloudflare AI Gateway DLP slow down streaming?"
    a: "Cloudflare's documentation notes that scanning responses requires buffering the full response, which increases time to first token. Shadow restores streams incrementally, holding back only a few characters."
  - q: "How much does Cloudflare AI Gateway DLP cost?"
    a: "AI Gateway's core features are available on Cloudflare plans; DLP profiles come from Cloudflare One. Which plans include DLP for AI Gateway was not verified at review time, so check Cloudflare's pricing. Shadow is free."
  - q: "Can I use Shadow with Cloudflare AI Gateway?"
    a: "Yes. Route requests through Shadow first, then to your Cloudflare AI Gateway URL. Cloudflare then sees already-masked prompts, and its DLP profiles act as a second check."
---

**Cloudflare AI Gateway is a hosted gateway that sits between your application and model providers, adding caching, rate limiting, analytics and logging, plus a DLP feature that can flag or block prompts and responses containing sensitive data. Shadow is an open-source privacy layer that replaces sensitive values with realistic, reversible fakes before requests leave your machine, and restores them in the answer.** Cloudflare's DLP decides whether a request may pass; Shadow changes the request so it can pass safely.

:::note[TL;DR]
Cloudflare AI Gateway DLP uses Cloudflare One DLP profiles to flag or block, and buffers responses to scan them. Shadow masks values with consistent fakes, restores them in responses, streams and tool calls, and runs locally with no account. Use Cloudflare for edge analytics, caching and a blocking backstop; use Shadow so useful requests are not blocked in the first place.
:::

## At a glance

| | Shadow | Cloudflare AI Gateway DLP |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, proprietary service |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Hosted on Cloudflare's network |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Flag or block, based on DLP profiles |
| **Restores responses** | Yes, automatically | No (nothing is replaced) |
| **Streaming** | Yes, SSE and NDJSON, incremental | Response scanning buffers the stream |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | No |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Not documented |
| **Self-hosted** | Yes, single binary, no account | No |
| **Price model** | Free | Cloudflare plan-based (DLP plan requirements not verified) |

## What Cloudflare AI Gateway does well

- **Zero infrastructure.** Change a base URL and traffic flows through Cloudflare's network, with no servers to run.
- **Gateway features.** Caching, rate limiting, retries, model fallbacks, analytics, cost tracking and request logs in one dashboard.
- **Shared DLP profiles.** [AI Gateway DLP](https://developers.cloudflare.com/ai-gateway/features/dlp/) reuses Cloudflare One DLP detection, so organisations already using Cloudflare One get consistent policies across web, SaaS and AI traffic. A "PII Record" profile was [added in April 2026](https://developers.cloudflare.com/changelog/post/2026-04-28-pii-record-profile/).
- **Both directions.** Prompts and responses can be checked, and matches can be flagged for review rather than blocked.
- **Scale.** Cloudflare's edge network handles high volume without capacity planning.

## Where Shadow is different

### Does blocking stop the leak or stop the work?

A blocked request protects the data but leaves the user without an answer, and people tend to work around tools that block often. Shadow translates the request: `jane@acme.com` becomes `hui.zhao@zhilan.cn`, the provider answers about the fake, and Shadow restores the real address in the response. See [introduction](/shadow/getting-started/introduction/) for the translate-don't-block approach.

### Where is sensitive data processed?

With a hosted gateway, prompts travel to the gateway in clear text before they are inspected. Shadow masks on the endpoint or in the same pod, so the provider and any gateway downstream only see fakes. No account, no telemetry.

### Streaming without buffering

Cloudflare's docs note that scanning responses buffers the full stream, which raises time to first token. Shadow restores [streams incrementally](/shadow/concepts/how-it-works/#streaming-hold-back), holding back at most the length of the longest fake.

### Agents, intent and files

Shadow restores [tool-call arguments](/shadow/concepts/tool-calls/) locally, uses an [intent policy](/shadow/concepts/intent-policy/) to choose fakes, tokens, keep or redact per value, and masks [uploaded Office documents](/shadow/concepts/file-masking/) in place.

## When should you choose Cloudflare AI Gateway?

- You want a hosted gateway with caching, analytics and rate limits and no infrastructure.
- Flagging or blocking is the right response to sensitive data in your traffic.
- You already use Cloudflare One DLP and want one policy set across channels.

## When should you choose Shadow?

- Requests containing personal data should still succeed, with answers restored to real values.
- Sensitive data must be masked before it leaves your machine or network.
- You stream responses and want no added time to first token from buffering.
- You need agent tool calls and file uploads handled.

## Can you use Cloudflare AI Gateway with Shadow?

Yes, and the combination is natural. Add a [custom route](/shadow/integrate/gateway-mode/#custom-routes) in Shadow pointing at your Cloudflare AI Gateway URL, or run your client behind Shadow's [proxy](/shadow/integrate/proxy-mode/) with the gateway host added to `extra_hosts`. Shadow masks first; Cloudflare caches, logs and analyses masked traffic, and its DLP profiles act as a backstop for anything Shadow's detectors miss. Logs stored at the gateway then contain fakes, not real values.

## How do they compare on accuracy?

Cloudflare's DLP profiles are not benchmarked publicly for AI Gateway. Shadow's [benchmark](/shadow/benchmarks/) publishes its leak-rate and round-trip methodology and code; hosted services without independent access are compared on documented capabilities only.

## Sources

- [Cloudflare AI Gateway DLP](https://developers.cloudflare.com/ai-gateway/features/dlp/)
- [PII Record DLP profile changelog](https://developers.cloudflare.com/changelog/post/2026-04-28-pii-record-profile/)
- [Cloudflare AI Gateway overview](https://developers.cloudflare.com/ai-gateway/)
- [Shadow streaming hold-back](/shadow/concepts/how-it-works/#streaming-hold-back)

---

*Last reviewed: 2026-10-31. Cloudflare is a trademark of its owner; this comparison is based on public documentation as of the review date.*
