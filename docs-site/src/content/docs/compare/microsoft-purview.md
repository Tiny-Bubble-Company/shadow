---
title: "Shadow vs Microsoft Purview — open-source Purview DSPM for AI alternative"
description: Shadow vs Microsoft Purview DSPM for AI. Purview warns, blocks and audits Copilot and AI sites; Shadow masks API prompts and restores responses.
sidebar:
  label: Microsoft Purview
  order: 23
keywords:
  - microsoft purview alternative
  - purview dspm for ai alternative
  - purview vs shadow
  - purview dlp chatgpt
  - purview pricing
  - copilot dlp
  - AI DLP
  - GenAI DLP
  - PII redaction for LLM
faq:
  - q: "Is Shadow an alternative to Microsoft Purview for AI?"
    a: "They cover different ground. Purview DSPM for AI governs Microsoft 365 Copilot and employee use of third-party AI sites in Microsoft-managed browsers and endpoints, mainly by warning, blocking and auditing. Shadow masks AI API traffic with reversible fakes on the machines where it runs. Shadow does not cover Copilot inside Microsoft 365."
  - q: "Can Purview scan files uploaded into ChatGPT prompts?"
    a: "Microsoft's documentation states that Purview cannot scan files uploaded into prompts on third-party AI sites. Shadow masks DOCX, XLSX and PPTX uploads in place and turns PDFs into masked text before they leave."
  - q: "Does Purview redact or block AI prompts?"
    a: "Purview's controls for AI are block, warn and audit, via Endpoint DLP and the browser extension for Edge, Chrome and Firefox. DLP for Copilot prompts was announced with a general-availability target of mid-2026 in a Message Center post; check Microsoft's current status. Reversible pseudonymisation is not documented."
  - q: "How much does Purview DSPM for AI cost?"
    a: "It is part of Microsoft 365 E5-class or Purview add-on licensing; exact entitlements vary by plan. Shadow is free under Apache-2.0."
  - q: "Does Shadow make us compliant with GDPR?"
    a: "No tool does that alone. Shadow helps meet data-minimisation requirements by keeping personal data out of requests to AI providers. Purview helps with discovery, classification and audit. Compliance depends on your whole programme."
---

**Microsoft Purview Data Security Posture Management (DSPM) for AI gives Microsoft 365 organisations visibility into AI use and applies DLP to Copilot and to third-party AI sites through Endpoint DLP and a browser extension, mainly by warning, blocking and auditing. Shadow is an open-source privacy layer that translates sensitive values in AI API requests into realistic, reversible fakes and restores them in the response.** If your AI use centres on Copilot and Microsoft-managed devices, Purview is the natural control. If developers, agents and services call model APIs directly, Shadow keeps that content private while answers stay useful.

:::note[TL;DR]
Purview is block-and-audit governance woven into Microsoft 365, with strong classification and reporting; its docs say it cannot scan files uploaded into prompts. Shadow masks prompts and files on the endpoint with reversible fakes and restores them in streams and tool calls. They are complementary.
:::

## At a glance

| | Shadow | Microsoft Purview DSPM for AI |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, commercial |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Microsoft 365 service, Endpoint DLP, browser extension (Edge, Chrome, Firefox) |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Block, warn, audit |
| **Restores responses** | Yes, automatically | No (block and audit model) |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Not applicable |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Not applicable |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Cannot scan files uploaded into prompts (per docs) |
| **Self-hosted** | Yes, single binary, no account | No, Microsoft cloud |
| **Price model** | Free | Microsoft 365 E5-class or add-on licensing |

## What Purview does well

- **Native Microsoft 365 coverage.** Purview understands Copilot interactions, sensitivity labels and Microsoft 365 content, which no third-party proxy can match.
- **Classification and labels.** Sensitive information types, trainable classifiers and sensitivity labels are reused for AI policies, so classification work done for email and SharePoint pays off.
- **Visibility into AI use.** DSPM for AI reports which AI apps are used and what kinds of sensitive data are involved, which helps prioritise risk.
- **Endpoint and browser enforcement.** Endpoint DLP and the extension for Edge, Chrome and Firefox warn or block when users paste sensitive data into third-party AI sites.
- **Audit, eDiscovery and compliance tooling.** Interactions can be audited and investigated with the same Purview tools used for the rest of Microsoft 365.
- **Little extra infrastructure** for organisations already licensed and managed through Intune.

## Where Shadow is different

### Does Purview keep AI useful when data is sensitive?

Purview's actions are block, warn and audit. That stops leaks but also stops the task. Shadow lets the request through with sensitive values translated: `jane@acme.com` becomes `hui.zhao@zhilan.cn`, a card number stays Luhn-valid, from a persistent [persona](/shadow/concepts/personas/). The model answers, and Shadow restores the real values.

### Files uploaded into prompts

Microsoft's documentation says Purview cannot scan files uploaded into prompts. Shadow masks DOCX, XLSX and PPTX uploads in place, keeps them valid documents, and turns PDFs into masked text. See [file masking](/shadow/concepts/file-masking/).

### API, agent and developer traffic

Purview's AI controls focus on Copilot and browser use. Shadow intercepts model API calls from any tool via [proxy mode](/shadow/integrate/proxy-mode/) or [gateway mode](/shadow/integrate/gateway-mode/), restores [streams](/shadow/concepts/how-it-works/#streaming-hold-back) and [tool-call arguments](/shadow/concepts/tool-calls/), and applies an [intent-aware policy](/shadow/concepts/intent-policy/).

### Not tied to one vendor's stack

Shadow runs on macOS, Linux and Windows, in containers and inside Rust services, with no account and no telemetry.

## When should you choose Purview?

- Your organisation is standardised on Microsoft 365 and Copilot.
- You need classification, labels, audit and eDiscovery that span AI and the rest of your content.
- You manage devices with Intune and want warn-or-block on third-party AI sites.
- You already hold the licences.

## When should you choose Shadow?

- Developers, agents and services call OpenAI, Anthropic, Gemini or other APIs directly.
- Users must keep working with sensitive data, with real values restored in answers.
- Files attached to prompts must be masked, not just blocked.
- You need an open-source, self-hosted control that works outside Microsoft-managed endpoints.

## Using Purview and Shadow together

1. Keep Purview for Copilot governance, classification, labels and audit.
2. Use Purview's Endpoint DLP and browser extension for web-chat AI on managed devices.
3. Install Shadow on developer machines, CI runners and services that call model APIs.
4. Reuse your Purview classification thinking: add organisation names, codenames and customer names to Shadow's [dictionary terms](/shadow/concepts/detection/#dictionary-terms).

## How do they compare on accuracy?

Microsoft does not publish Purview AI detection results on public PII datasets. Shadow publishes its methodology on the [benchmarks page](/shadow/benchmarks/).

## Sources

- [Microsoft Purview DSPM for AI](https://learn.microsoft.com/purview/ai-microsoft-purview)
- [Message Center RM561892 (DLP for Copilot prompts)](https://mc.merill.net/message/RM561892)
- [Shadow: file masking](/shadow/concepts/file-masking/)
- [Shadow: security model](/shadow/reference/security/)

---

*Last reviewed: 2026-10-31. Microsoft, Microsoft 365, Copilot and Purview are trademarks of their owner; this comparison is based on public documentation as of the review date.*
