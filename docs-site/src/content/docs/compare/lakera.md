---
title: "Shadow vs Lakera Guard — Lakera alternative for PII masking in LLM prompts"
description: Shadow vs Lakera Guard (Check Point AI Guardrails). Lakera detects PII and prompt attacks; Shadow masks PII with reversible fakes and restores answers.
sidebar:
  label: Lakera Guard
  order: 18
keywords:
  - lakera alternative
  - lakera guard alternative
  - lakera vs shadow
  - lakera guard pricing
  - check point ai guardrails
  - open source lakera
  - PII redaction for LLM
  - AI DLP
faq:
  - q: "Is Shadow an alternative to Lakera Guard?"
    a: "For PII, partly. Lakera Guard is an API that screens prompts and responses for prompt injection, jailbreaks, PII, secrets and unsafe content, and returns flags your application acts on. Shadow does not detect prompt injection; it masks sensitive values with reversible fakes and restores them. They complement each other."
  - q: "Does Lakera Guard redact PII?"
    a: "Lakera Guard's public documentation describes detecting and flagging PII, secrets and custom patterns. It does not describe replacing values and restoring them in the response. Shadow does both."
  - q: "Who owns Lakera now?"
    a: "Check Point announced the acquisition of Lakera in September 2025. Lakera Guard is being offered as part of Check Point's AI security portfolio, referred to as Check Point AI Guardrails."
  - q: "How much does Lakera Guard cost?"
    a: "Third-party sources describe a free community tier of around 10,000 requests per month, with Enterprise plans on quote. Check Lakera's or Check Point's site for current pricing. Shadow is free and open source."
  - q: "Can Lakera Guard be self-hosted?"
    a: "Lakera describes a self-hosted option for Enterprise customers. The standard offering is a SaaS API. Shadow is always self-hosted."
---

**Lakera Guard, now part of Check Point, is an API that screens LLM inputs and outputs for prompt injection, jailbreaks, PII, secrets and other risks, and returns flags your application uses to block or allow. Shadow is an open-source privacy layer that replaces sensitive values with realistic, reversible fakes before requests reach the provider, and restores them in responses, streams and tool calls.** Lakera answers "is this request dangerous?"; Shadow answers "how do we send this request without exposing the data in it?".

:::note[TL;DR]
Lakera Guard is strongest at attack detection (prompt injection, jailbreaks) and also flags PII and secrets. It does not document masking-and-restore. Shadow masks PII and secrets with consistent fakes and restores answers locally, but does not detect prompt injection. Many teams will want both.
:::

## At a glance

| | Shadow | Lakera Guard |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, proprietary |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | SaaS API; self-hosted for Enterprise |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Detect and flag (PII, secrets, custom patterns, prompt attacks); your app blocks |
| **Restores responses** | Yes, automatically | No (nothing is replaced) |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Not documented |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Agent-security features for attack detection |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Not documented |
| **Self-hosted** | Yes, single binary, no account | Enterprise only |
| **Price model** | Free | Free community tier (per third-party sources); Enterprise quote |

## What Lakera Guard does well

- **Prompt-injection and jailbreak detection.** This is Lakera's core strength, backed by a large stream of adversarial data from its public red-teaming games. Shadow does not attempt it.
- **One API for many risks.** PII, secrets, custom regex, content moderation and malicious links are screened in one call, with results your app can act on.
- **Low integration effort.** A single HTTP call before and after the model, with SDKs and policy configuration in a dashboard.
- **Enterprise backing.** Following Check Point's [acquisition](https://www.itpro.com/business/acquisition/check-point-buys-lakera-to-secure-the-full-enterprise-ai-lifecycle), Lakera fits into a larger security portfolio, which suits organisations standardising on Check Point.
- **Agent security.** Lakera positions its product for agentic applications, screening tool inputs and outputs for attacks.

## Where Shadow is different

### Flagging versus masking

When Lakera flags PII, your application has to decide what to do, usually block or ask the user to edit. Shadow transforms the request instead: names, emails, IDs and secrets become consistent fakes from a persistent [persona](/shadow/concepts/personas/), the request goes through, and the response comes back with the real values. The user gets an answer and the provider never sees the values Shadow detected.

### Where does the prompt go?

Screening with a SaaS API means sending the unmasked prompt to that API. Shadow detects locally, with no account and no telemetry, and only the masked request leaves the machine.

### Restoration in streams and tool calls

Shadow restores fakes in [SSE and NDJSON streams](/shadow/concepts/how-it-works/#streaming-hold-back) and in [tool-call arguments](/shadow/concepts/tool-calls/), and applies an [intent policy](/shadow/concepts/intent-policy/) so bulk reports get typed tokens and provider-hosted actions are handled explicitly.

### Coverage without code

Lakera is called from your code. Shadow can run as an [HTTPS proxy](/shadow/integrate/proxy-mode/) that covers every tool on a machine that calls a [supported AI API](/shadow/reference/providers/), including ones you didn't write, and masks [uploaded Office files](/shadow/concepts/file-masking/).

## When should you choose Lakera Guard?

- Your main risk is prompt injection, jailbreaks or harmful content in a customer-facing LLM app.
- You want a managed API with a dashboard and vendor support.
- Flagging PII for your app to block or review is enough.

## When should you choose Shadow?

- You need to send prompts that contain personal or confidential data and still get useful answers.
- Data must not leave your machine or network unmasked, including to a screening API.
- You need open source, self-hosting and no per-request cost.

## Can you use Lakera Guard with Shadow?

Yes. A layered setup works well: Shadow masks the request first, then your application (or gateway) calls Lakera Guard on the masked text for prompt-injection and content checks. Lakera then never receives the personal data Shadow detected, and Shadow restores values only after Lakera has screened the response. Expect Lakera's PII detector to fire less often on masked text, though realistic fakes may still look like PII to it; you may want to rely on Shadow for PII and on Lakera for attacks.

## How do they compare on accuracy?

They mostly measure different things: Lakera focuses on attack detection, Shadow on keeping sensitive values out of requests. Shadow's [benchmark](/shadow/benchmarks/) covers PII leak rate, over-masking and round-trip exactness with published code. Lakera was not independently benchmarked for PII.

## Sources

- [Lakera Guard](https://www.lakera.ai/lakera-guard)
- [Lakera documentation](https://docs.lakera.ai/)
- [Check Point acquires Lakera (ITPro)](https://www.itpro.com/business/acquisition/check-point-buys-lakera-to-secure-the-full-enterprise-ai-lifecycle)
- [Shadow security model](/shadow/reference/security/)

---

*Last reviewed: 2026-10-31. Lakera and Check Point are trademarks of their owners; this comparison is based on public documentation as of the review date.*
