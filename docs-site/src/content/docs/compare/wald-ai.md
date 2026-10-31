---
title: "Shadow vs Wald.ai — open-source, self-hosted Wald.ai alternative"
description: Shadow vs Wald.ai for safe use of ChatGPT and Claude. Compare contextual substitution, restoration, deployment and pricing with a free local proxy.
sidebar:
  label: Wald.ai
  order: 38
keywords:
  - wald ai alternative
  - wald.ai vs shadow
  - open source wald ai
  - wald ai pricing
  - self-hosted wald ai alternative
  - secure chatgpt for business
  - PII masking for LLM
  - reversible pseudonymization LLM
faq:
  - q: "Is Shadow an alternative to Wald.ai?"
    a: "Yes, conceptually they are close. Both substitute sensitive values before a model sees them and restore them in the answer. Wald.ai is a cloud-hosted secure chat and API service; Shadow is a free, open-source layer that runs on your own machines as a proxy, gateway or Rust library."
  - q: "How much does Wald.ai cost?"
    a: "Wald.ai lists a consumer plan at about $19.99 per user per month; business pricing may differ. Check Wald.ai for current prices. Shadow is free and open source under Apache-2.0."
  - q: "Is Wald.ai self-hosted?"
    a: "Wald.ai is offered as SaaS. Public material we reviewed does not describe a self-hosted option. Shadow is self-hosted only, with no hosted component and no telemetry."
  - q: "Which languages does Wald.ai support?"
    a: "Wald.ai's API documentation states English only, with a 128K-token limit. Shadow's pattern detectors are language-independent for structured values, and its optional NER sidecar adds multilingual names via GLiNER; multilingual recall is measured on the benchmarks page."
  - q: "Does Wald.ai work with agents and tool calls?"
    a: "Public docs we reviewed do not describe tool-call or streaming restoration. Shadow restores both."
---

**Wald.ai is a commercial secure-AI service: it replaces sensitive values with context-aware substitutes before prompts reach ChatGPT, Claude or other models, keeps the substitutions consistent through a conversation, and restores them in the answer. Shadow does the same job as open-source (Apache-2.0) software you run yourself, as an endpoint proxy, gateway or Rust library, and adds streaming and tool-call restoration, an intent policy and Office file masking.** Wald.ai suits teams that want a hosted chat product. Shadow suits teams that want the mapping and the traffic to stay on their own machines.

:::note[TL;DR]
Wald.ai and Shadow share the idea that substitution beats blocking. Wald.ai delivers it as a hosted chat and API, priced per user. Shadow delivers it as free, self-hosted software that sits in front of AI tools and SDKs that call provider APIs, with realistic persona-based fakes and restoration in streams and tool calls.
:::

## At a glance

| | Shadow | Wald.ai |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, proprietary |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | SaaS secure chat and API |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Contextual substitution with encrypted obfuscation maps |
| **Restores responses** | Yes, automatically | Yes |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Not documented |
| **Tool calls** | Yes, arguments restored locally; intent-aware | Not documented |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Not documented in the docs we reviewed |
| **Self-hosted** | Yes, single binary, no account | No, SaaS |
| **Price model** | Free | Per user (consumer plan about $19.99/month) |

## What Wald.ai does well

- **Substitution, not blocking.** Wald.ai replaces values in context rather than refusing the prompt, so users keep getting useful answers.
- **Conversation consistency.** It describes encrypted obfuscation maps that keep substitutions consistent across a conversation.
- **Ready-made product.** A secure chat interface means non-technical users can start immediately, without configuring proxies or base URLs.
- **Contextual detection.** Wald.ai positions its redaction as context-aware rather than purely pattern-based.

## Where Shadow is different

### Is Shadow a free, self-hosted alternative to Wald.ai?

Yes. Shadow is Apache-2.0, runs on macOS, Linux, Windows, servers and containers, and has no hosted component. The real-to-fake mapping lives in an [encrypted vault](/shadow/concepts/vault/) on your machine. With a hosted service, prompts pass through the vendor before reaching the model; with Shadow, they are masked before they leave the machine.

### Works with the tools you already use

Wald.ai is a destination: users chat in Wald's interface or call its API. Shadow is in the path: set `HTTPS_PROXY` or a base URL and existing tools, IDE assistants, CLIs and SDKs that call a [supported AI API](/shadow/reference/providers/) keep working, now masked. Web chat such as chatgpt.com or claude.ai in a browser is not covered. See [proxy mode](/shadow/integrate/proxy-mode/) and [gateway mode](/shadow/integrate/gateway-mode/).

### Persistent persona

Shadow's fakes come from a persistent [persona](/shadow/concepts/personas/), so consistency extends across conversations and days, not just one chat. Fakes keep their format: valid emails, Luhn-valid cards, mod-97 IBANs.

### Streams, tools and intent

Shadow restores [streamed responses](/shadow/concepts/how-it-works/#streaming-hold-back) and [tool-call arguments](/shadow/concepts/tool-calls/), and its [intent policy](/shadow/concepts/intent-policy/) uses typed tokens for reports and handles provider-hosted tools explicitly.

### Files

DOCX, XLSX and PPTX uploads are masked in place. See [file masking](/shadow/concepts/file-masking/).

### What Shadow lacks

No chat UI, no hosted service, no admin console. Someone has to install and configure it.

## When should you choose Wald.ai?

- You want a ready-made secure chat for business users with no installation.
- You prefer a managed service and per-user billing.
- Your use is primarily English chat.

## When should you choose Shadow?

- You want data masked before it leaves your machines, with no third party in the path.
- You want to keep using existing tools, SDKs and agents.
- You use streaming, tool calls or Office uploads.
- You need open-source code you can audit, at no cost.

## Migrating from Wald.ai

1. Install Shadow on the machines or as a sidecar for the services that used Wald.ai's API ([quick start](/shadow/getting-started/quick-start/)).
2. Point SDKs at `http://127.0.0.1:8877/gw/<provider>/...`, or set `HTTPS_PROXY` for whole-machine coverage.
3. Add your organisation names, customers and codenames as [dictionary terms](/shadow/concepts/detection/#dictionary-terms).
4. Test with `shadow scan` on typical prompts.
5. Give chat users an API-based AI client of your choice that goes through Shadow; the chatgpt.com and claude.ai websites are not masked.

## How do they compare on accuracy?

Wald.ai does not publish detection benchmarks that we found. Shadow's [benchmark](/shadow/benchmarks/) publishes methodology and code.

## Sources

- [Wald.ai data security for AI](https://wald.ai/datasecurity-for-ai)
- [Dark Reading: Wald launches data loss protection for AI platforms](https://darkreading.com/data-privacy/wald-launches-data-loss-protection-for-ai-platforms)
- [Shadow security model](/shadow/reference/security/)

---

*Last reviewed: 2026-10-31. Wald.ai is a trademark of its owner; this comparison is based on public documentation as of the review date.*
