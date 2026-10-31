---
title: "Shadow vs Portkey — Portkey PII redaction alternative that restores responses"
seoTitle: "Shadow vs Portkey: PII redaction that restores responses"
description: Shadow vs Portkey AI gateway for PII. Portkey redacts one way via partner guardrails; Shadow uses reversible fakes and restores streams and tool calls.
sidebar:
  label: Portkey
  order: 29
keywords:
  - portkey alternative
  - portkey vs shadow
  - portkey pii redaction
  - portkey guardrails
  - portkey pricing
  - open source portkey
  - LLM gateway PII masking
  - AI DLP
faq:
  - q: "Is Shadow an alternative to Portkey?"
    a: "Only for data protection. Portkey is an AI gateway and control plane with routing, observability, prompt management and guardrails. Shadow does not route or observe; it masks sensitive values with reversible fakes and restores them in responses. You can run Shadow alongside Portkey."
  - q: "Does Portkey restore redacted PII in the response?"
    a: "Portkey's PII redaction documentation describes the transformation as one-way: values are redacted before the request reaches the model and are not put back. Shadow restores its fakes in the response."
  - q: "Is Portkey open source?"
    a: "The Portkey AI Gateway is open source under MIT. The PII redaction guardrails run through Portkey's cloud and partner integrations such as Pangea, AWS Bedrock Guardrails, Azure and Patronus. Shadow is fully open source under Apache-2.0."
  - q: "Was Portkey acquired by Palo Alto Networks?"
    a: "Palo Alto Networks announced an agreement to acquire Portkey on 30 April 2026, with Portkey positioned as the Prisma AIRS AI Gateway. The closing date was not verified at the time of review."
  - q: "How much does Portkey cost?"
    a: "Portkey offers SaaS tiers and enterprise plans; check Portkey's pricing page for current figures. Shadow is free to self-host."
---

**Portkey is an AI gateway and control plane: an open-source gateway for routing to many model providers, plus a hosted platform for observability, prompt management and guardrails. Shadow is an open-source privacy layer that replaces sensitive values with realistic, reversible fakes before requests leave, and restores them in responses, streams and tool calls.** Portkey's PII redaction is one-way and comes from partner guardrails; Shadow's masking is reversible and runs locally.

:::note[TL;DR]
Choose Portkey for multi-provider routing, observability and a managed control plane. Choose Shadow when prompts must be masked before they leave your machine and the answer must come back with real values. They can run together: Shadow masks, Portkey routes and observes.
:::

## At a glance

| | Shadow | Portkey |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Gateway MIT; guardrails and platform in Portkey's cloud |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | SaaS, hybrid, or self-hosted OSS gateway |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Redaction via partner guardrails (Pangea, Bedrock, Azure, Patronus) and regex checks |
| **Restores responses** | Yes, automatically | No (docs describe redaction as one-way) |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Not documented for PII redaction |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Not documented for PII |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Not documented |
| **Self-hosted** | Yes, single binary, no account | Gateway yes; partner guardrails are hosted services |
| **Price model** | Free | SaaS tiers and enterprise plans |

## What Portkey does well

- **Routing and reliability.** One API over many providers with fallbacks, retries, load balancing, caching and conditional routing.
- **Observability.** Request logs, traces, cost and latency analytics, and feedback in a hosted dashboard.
- **Prompt management.** Versioned prompt templates and a playground for teams.
- **Guardrail marketplace.** Portkey can call many guardrail partners, so you can combine PII, safety and content checks without integrating each vendor yourself.
- **Open-source gateway.** The gateway itself is MIT-licensed and widely used, and can run in your own environment.
- **Enterprise direction.** Palo Alto Networks [announced](https://pulse2.com/palo-alto-networks-to-acquire-portkey-to-establish-ai-gateway-as-control-plane-for-autonomous-enterprise-agents/amp/) its intent to acquire Portkey in April 2026 as part of Prisma AIRS, which may matter to buyers standardising on Palo Alto's platform. Closing was not verified at review time.

## Where Shadow is different

### Can Portkey put redacted values back in the answer?

Portkey's [PII redaction docs](https://portkey.ai/docs/product/guardrails/pii-redaction) describe the transformation as one-way. That is safe, but the answer then contains placeholders, and an agent cannot act on a redacted email address. Shadow keeps a real-to-fake mapping in an [encrypted local vault](/shadow/concepts/vault/) and restores its fakes in JSON, [streamed responses](/shadow/concepts/how-it-works/#streaming-hold-back) and [tool-call arguments](/shadow/concepts/tool-calls/).

### Where does the data go before it is masked?

With hosted partner guardrails, the unmasked prompt is sent to a guardrail service to be checked. Shadow detects locally with built-in detectors and, optionally, local NER models or Presidio on loopback. Plaintext is never sent to a guardrail service; only the masked request leaves the host, apart from message recipients and the values a provider-hosted action needs, which are kept real by default and logged. No account, no telemetry.

### Realistic fakes and intent

Shadow's fakes come from a persistent [persona](/shadow/concepts/personas/), so prompts stay coherent and consistent across sessions. The [intent policy](/shadow/concepts/intent-policy/) uses typed tokens for bulk reports and a logged `keep` for message recipients and provider-hosted action targets.

### Files

Shadow masks DOCX, XLSX and PPTX uploads in place and PDF text before upload ([file masking](/shadow/concepts/file-masking/)).

## When should you choose Portkey?

- You need a managed control plane: routing, observability, prompt management and governance for many teams.
- One-way redaction is acceptable and you want to buy guardrails from several vendors through one integration.
- You are standardising on Palo Alto Networks' AI security stack.

## When should you choose Shadow?

- Answers and agent actions need the real values back.
- Data must be masked before it leaves the machine or the pod, without a hosted guardrail call.
- You want an open-source, self-hosted tool with no account.

## Can you use Portkey with Shadow?

Yes. Run Shadow as a sidecar or on developer machines and send traffic through Shadow before it reaches Portkey, or configure the self-hosted Portkey gateway to call providers through Shadow's HTTPS proxy. Portkey still sees request metadata, cost and latency, while the values Shadow detected are already masked in its logs, which also keeps them out of observability storage. Disable Portkey's PII redaction guardrail on those routes so values reach Shadow intact when Shadow sits downstream. See [integration modes](/shadow/integrate/overview/).

## Migrating Portkey PII redaction to Shadow

1. Keep Portkey for routing and observability.
2. Turn off the PII redaction guardrail on the routes Shadow will cover.
3. Install Shadow next to the client or the self-hosted gateway ([Docker and Kubernetes](/shadow/integrate/docker-kubernetes/)).
4. Route provider traffic through Shadow's [proxy](/shadow/integrate/proxy-mode/) or [gateway](/shadow/integrate/gateway-mode/).
5. Add company names and codenames as [dictionary terms](/shadow/concepts/detection/#dictionary-terms) and check coverage with `shadow scan`.

## How do they compare on accuracy?

Portkey's detection accuracy is whichever partner guardrail you choose. Shadow's [benchmark](/shadow/benchmarks/) publishes leak rate, over-masking and round-trip results with code, against open-source baselines; Portkey and its partner guardrails are not included.

## Sources

- [Portkey PII redaction](https://portkey.ai/docs/product/guardrails/pii-redaction)
- [Portkey AI Gateway on GitHub](https://github.com/Portkey-AI/gateway)
- [Portkey pricing](https://portkey.ai/pricing)
- [Palo Alto Networks to acquire Portkey (Pulse 2.0)](https://pulse2.com/palo-alto-networks-to-acquire-portkey-to-establish-ai-gateway-as-control-plane-for-autonomous-enterprise-agents/amp/)

---

*Last reviewed: 2026-10-31. Portkey is a trademark of its owner; this comparison is based on public documentation as of the review date.*
