---
title: "Shadow vs Prompt Security — open-source Prompt Security alternative for LLM data"
description: Shadow vs Prompt Security (SentinelOne). Prompt Security filters or blocks AI prompts; Shadow masks them with reversible fakes and restores answers.
sidebar:
  label: Prompt Security
  order: 32
keywords:
  - prompt security alternative
  - prompt security vs shadow
  - prompt security sentinelone
  - prompt security pricing
  - open source prompt security alternative
  - AI DLP
  - GenAI DLP
  - PII redaction for LLM
faq:
  - q: "Is Shadow a free alternative to Prompt Security?"
    a: "For AI API data protection, partly. Shadow is free and open source and masks prompts with reversible fakes on the machines where it runs. Prompt Security, now part of SentinelOne, is a commercial platform with a browser extension and gateway that also addresses prompt injection, shadow AI and other AI security risks that Shadow does not cover."
  - q: "Did SentinelOne acquire Prompt Security?"
    a: "Yes. SentinelOne announced the acquisition of Prompt Security in August 2025."
  - q: "Does Prompt Security redact sensitive data?"
    a: "Its public material describes filtering and blocking prompts that contain sensitive data. We could not verify detailed documentation of redaction or of restoring values in responses, so check with the vendor. Shadow replaces values with realistic fakes and restores them."
  - q: "How much does Prompt Security cost?"
    a: "Pricing is by quote through SentinelOne. Shadow is free under Apache-2.0."
  - q: "Can Prompt Security and Shadow run together?"
    a: "Yes. Prompt Security can govern employee AI use and screen for AI threats while Shadow masks the content of API, agent and developer traffic on the endpoint."
---

**Prompt Security, acquired by SentinelOne in 2025, offers a browser extension and a gateway that give organisations visibility into AI use and filter or block risky prompts. Shadow is an open-source privacy layer that translates sensitive values in AI API requests into realistic, reversible fakes and restores them in the response.** Prompt Security is a broad AI security platform; Shadow does one job, keeping data out of provider requests while tools keep working.

:::note[TL;DR]
Prompt Security covers AI security broadly: shadow-AI discovery, prompt filtering and blocking, and protection against AI-specific threats, now within SentinelOne. Its redaction behaviour is not documented in detail publicly. Shadow masks values with consistent, format-preserving fakes and restores them in streams, tool calls and files, locally and for free.
:::

## At a glance

| | Shadow | Prompt Security (SentinelOne) |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, commercial |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Browser extension, gateway, SaaS console |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Filter or block; redaction not verified |
| **Restores responses** | Yes, automatically | Not documented |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Not documented |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Not documented |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Not documented |
| **Self-hosted** | Yes, single binary, no account | Not documented |
| **Price model** | Free | Quote |

## What Prompt Security does well

- **Broad AI security scope.** Beyond data leakage, it addresses risks such as prompt injection and unsafe AI use, which Shadow does not attempt.
- **Shadow-AI discovery.** The extension gives security teams visibility into which AI tools employees use.
- **Two enforcement points.** A browser extension for employee use and a gateway for applications cover both people and apps.
- **Central policy and reporting.** Policies are managed in one console, with visibility for security teams.
- **SentinelOne integration.** As part of SentinelOne, it can sit alongside endpoint and XDR tooling that many security teams already run.

## Where Shadow is different

### Filter and block, or translate?

Blocking or filtering a prompt prevents a leak but also stops or degrades the work. Shadow sends the request with every sensitive value it detects translated into a realistic fake from a persistent [persona](/shadow/concepts/personas/): `Acme GmbH` becomes `Zhilan Technology Co., Ltd.`, emails and phone numbers keep their shape, card numbers stay Luhn-valid. The model answers normally and Shadow restores the real values.

### Streams and tool calls

Shadow restores fakes in [SSE and NDJSON streams](/shadow/concepts/how-it-works/#streaming-hold-back), even when split across events, and in [tool-call arguments](/shadow/concepts/tool-calls/) before local tools run. Public Prompt Security documentation does not describe restoration.

### Intent-aware policy

Shadow picks a treatment per value based on what the request is for: fakes in conversation, typed tokens for bulk reports, kept targets for provider-hosted actions. See [intent-aware policy](/shadow/concepts/intent-policy/).

### Files

Shadow masks DOCX, XLSX and PPTX uploads in place and turns PDFs into masked text. See [file masking](/shadow/concepts/file-masking/).

### Open source and local

No account, no telemetry, auditable code, and no plaintext sent to a vendor cloud.

## When should you choose Prompt Security?

- You want one platform for AI security: discovery, data protection and AI threat protection.
- You already use SentinelOne and want AI controls in the same ecosystem.
- You need central visibility over employee AI use.
- Blocking or filtering fits your risk appetite.

## When should you choose Shadow?

- Your priority is keeping sensitive values out of model API requests while answers stay useful.
- You rely on streaming, agents and tool calls.
- You want Office files masked before upload.
- You need an open-source, self-hosted control with no licence cost.

## Using Prompt Security and Shadow together

1. Use Prompt Security for AI discovery, employee browser use and AI threat screening.
2. Install Shadow on developer machines, CI runners and services that call model APIs, in [proxy mode](/shadow/integrate/proxy-mode/) or [gateway mode](/shadow/integrate/gateway-mode/).
3. If both sit in the API path, put Shadow closest to the application so that the gateway only sees masked content. Check that any prompt-injection scanning still works on masked text.

## How do they compare on accuracy?

Prompt Security does not publish PII detection results on public datasets that we could find. Shadow publishes its methodology on the [benchmarks page](/shadow/benchmarks/).

## Sources

- [SentinelOne acquires Prompt Security (SiliconANGLE)](https://siliconangle.com/2025/08/05/sentinelone-acquires-ai-security-startup-prompt-security/)
- [Prompt Security](https://www.prompt.security/)
- [Shadow: how it works](/shadow/concepts/how-it-works/)

---

*Last reviewed: 2026-10-31. Prompt Security and SentinelOne are trademarks of their owners; this comparison is based on public documentation as of the review date.*
