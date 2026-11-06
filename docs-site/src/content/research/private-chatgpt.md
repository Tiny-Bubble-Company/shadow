---
title: "Private ChatGPT: How to Use AI Without Sharing Your Data"
seoTitle: "Private ChatGPT: How to Use AI Without Sharing Your Data"
description: "A private ChatGPT keeps prompts out of training data and away from the provider. Compare settings, enterprise plans, self-hosted models and privacy proxies."
publishedDate: 2026-09-20
updatedDate: 2026-11-06
topics:
  - AI Privacy
  - Enterprise AI
  - ChatGPT
author: Shadow Research Team
readingTime: 12
keywords:
  - private chatgpt
  - private ai
  - private chatgpt for business
  - chatgpt without sharing data
  - chatgpt enterprise privacy
  - turn off chatgpt training
faq:
  - q: "Is there a private version of ChatGPT?"
    a: "There is no single product called private ChatGPT. You can get privacy by turning off training in settings (limited), using ChatGPT Enterprise or the API with zero-data-retention terms, running an open-weight model locally, or routing prompts through a privacy proxy like Shadow that masks your data before it reaches any provider."
  - q: "Does ChatGPT train on my data?"
    a: 'On personal Free, Plus and Pro accounts, yes by default. You can turn off "Improve the model for everyone" in Settings, Data controls, but prompts are still sent and retained for abuse monitoring. ChatGPT Business, Enterprise and the API do not train on your data by default.'
  - q: "Is ChatGPT Enterprise private?"
    a: "ChatGPT Enterprise does not train on your data by default, has admin-controlled retention, SOC 2 Type 2 and SSO. Prompts still reach OpenAI, and some content may be reviewed for abuse or safety, per OpenAI's terms. Whether that meets your definition of private depends on your contracts and data classification."
  - q: "Can I run ChatGPT locally?"
    a: "You cannot run ChatGPT itself locally because it is a proprietary OpenAI service. You can run open-weight models such as Llama, Mistral or Qwen locally with tools like Ollama or vLLM. Quality and speed depend on your hardware; most are weaker than GPT-4o or Claude on complex reasoning."
  - q: "How do I stop employees pasting customer data into ChatGPT?"
    a: "Blocking AI sites rarely holds and pushes use to personal devices. The approach that works is to give staff an approved tool that masks sensitive data before prompts leave, so there is no reason to route around. Shadow replaces names, emails, IDs and secrets with realistic fakes and restores them in the answer."
  - q: "Is a private ChatGPT GDPR compliant?"
    a: "No single tool makes you GDPR compliant. GDPR asks for a lawful basis, data minimisation, a DPA with processors and more. A privacy proxy supports data minimisation by pseudonymising personal data before it leaves, but you still need the organisational controls."
cta:
  label: "Try Shadow free"
  href: /getting-started/quick-start/
related:
  - is-chatgpt-safe-for-company-data
  - does-cursor-train-on-my-code
  - what-is-an-llm-proxy
  - what-is-shadow-ai
---

**A private ChatGPT is any setup that lets you use a powerful AI assistant without your prompts being stored, trained on, or seen by the provider.** There is no single product with that name: you achieve it by combining the right plan, settings, architecture, or tools. The safest approach is to keep sensitive data out of the prompt entirely, or mask it before it leaves your machine.

This guide explains what "private" realistically means, compares your options honestly, and shows how to set up a private ChatGPT for your team in an afternoon.

## What does "private ChatGPT" actually mean?

People searching for a private ChatGPT usually want one or more of these:

1. **No training.** Prompts are not used to improve the model.
2. **No retention.** Prompts are not stored after the request completes.
3. **No access.** No one at the provider can read the prompts.
4. **Data stays local.** Nothing sensitive ever leaves your network.

No cloud AI service offers all four, because even zero-retention APIs log something for abuse monitoring and safety review. The practical question is how much risk you accept, and for which data.

## Your options

### 1. Consumer ChatGPT with training turned off

On personal Free, Plus and Pro accounts, you can disable "Improve the model for everyone" in Settings, Data controls. You can also use temporary chats, which are not saved to history and not used for training.

**Pros:** Free or cheap, no setup, the full GPT-4o model.

**Cons:** Settings can change. Users have reported the training toggle being silently re-enabled after product updates, including reports around Codex in 2025 and Gemini personal accounts training on saved chats. Even with training off, prompts are still sent to OpenAI and retained for up to 30 days for abuse monitoring. The company cannot see or delete what employees send from personal accounts. This is not suitable for customer data, credentials or regulated information.

### 2. ChatGPT Team/Enterprise or Claude Team/Enterprise

Business plans from OpenAI and Anthropic come with zero-data-retention (ZDR) agreements, admin controls, SSO and SOC 2 attestations. The OpenAI API and Anthropic API also offer ZDR for eligible use cases.

**Pros:** Prompts are not used for training by default, retention is admin-controlled, and you have a contract and a DPA.

**Cons:** Prompts still reach the provider and may be reviewed by staff for abuse, safety or legal reasons. You pay per seat or per token, and you must trust the provider's terms and infrastructure. See [Is ChatGPT safe for company data?](/shadow/research/is-chatgpt-safe-for-company-data/) for plan-by-plan retention details.

### 3. Self-hosted open-weight models (Ollama, vLLM)

Run Llama, Mistral, Qwen, DeepSeek or another open-weight model on your own hardware with [Ollama](https://ollama.com/) (single machine) or [vLLM](https://docs.vllm.ai/) (multi-GPU inference). Nothing leaves your network.

**Pros:** Full data control. No per-token cost after hardware.

**Cons:** Quality trails frontier models on complex reasoning and coding. You need capable hardware (a good GPU for interactive use). Setup, maintenance and model updates are your responsibility. No hosted safety filters or RLHF unless you add them.

### 4. A privacy proxy that masks data before it leaves

A privacy proxy sits between your apps and the AI provider, detects personal data and secrets in prompts, replaces them with realistic fakes, and restores the real values in the response. You keep using frontier models, and the provider sees a prompt about fictional people.

**Pros:** Full model quality. Works with ChatGPT, Claude, Gemini and coding assistants. Data is masked on your machine, not a third-party server. Streaming and tool calls are supported.

**Cons:** Detection is best effort: unusual names or formats can be missed. The proxy adds a small amount of latency. You need to install and configure it.

[Shadow](/shadow/) is an open-source (Apache-2.0) privacy proxy that does exactly this. It runs on `127.0.0.1:8877`, masks prompts to [288 AI services](/shadow/reference/providers/), and restores the real values in answers, including streamed text and tool calls.

## Comparison table

| Approach | Data leaves your network? | Provider sees real PII? | Model quality | Setup effort | Cost |
|---|---|---|---|---|---|
| Consumer ChatGPT, training off | Yes | Yes | Frontier | None | Free or subscription |
| ChatGPT Enterprise / API ZDR | Yes | Yes (under contract) | Frontier | Procurement | Per seat or token |
| Self-hosted (Ollama/vLLM) | No | No | Open-weight, weaker | Medium-high | Hardware |
| Privacy proxy (Shadow) | Yes, masked | No (fakes only) | Frontier | Low | Free (Shadow Core) |

## How to set up a private ChatGPT for your team in an afternoon

### Step 1: Pick a sanctioned tool

Decide which AI tools your team may use. Options include:

- ChatGPT Business or Enterprise (prompts reach OpenAI under a DPA)
- The OpenAI or Anthropic API via an internal chat UI
- ChatGPT, Claude.ai or Gemini in a browser, with Shadow's [system proxy](/shadow/integrate/proxy-mode/#system-wide-with-the-system-proxy) masking prompts
- A self-hosted model for the most sensitive work

Most teams pick a hosted option for quality and convenience, then mask the prompts.

### Step 2: Write a one-page acceptable use policy

Tell people what tools are approved, what data may never be shared (customer PII, credentials, unreleased financials), and what happens if they use unapproved tools. Start from our [free AI acceptable use policy template](/shadow/research/ai-acceptable-use-policy-template/).

### Step 3: Route traffic through Shadow

Install Shadow on each laptop so prompts are masked before they leave:

```sh
curl -fsSL https://hub.getshadow.dev/core/install.sh | sh
shadow init --org "Your Company"
shadow install --apply
```

For ChatGPT, Claude.ai and other browser-based chat, turn on the system proxy:

```sh
shadow install --system-proxy --apply
```

For API-based tools, point them at Shadow's [gateway](/shadow/integrate/gateway-mode/) or set `HTTPS_PROXY=http://127.0.0.1:8877`. See the [quick-start guide](/shadow/getting-started/quick-start/) and [proxy mode](/shadow/integrate/proxy-mode/) for detailed setup.

### Step 4: Get visibility with Shadow Hub

[Shadow Hub](/shadow/hub/) shows which AI tools your team uses and what Shadow masked or blocked, per user and per tool, without collecting prompt text by default. It is free for up to 3 users, then $3 per user per month up to 500 users. See [Pricing](/shadow/pricing/).

Connect a laptop to Hub with one command from your invite link:

```sh
curl -fsSL https://hub.getshadow.dev/core/install.sh | SHADOW_ENROLL='<invite-link>' sh
```

## Example: before and after masking

What you type into ChatGPT:

```text
Draft a reply to Jane Doe (jane.doe@acme.example, +1 415 555 0199) about
her order #A-29104 that shipped to 221 Baker Street. She says Tom from
support promised a refund.
```

What ChatGPT sees (persona values differ per machine):

```text
Draft a reply to Yuki Tanaka (yuki.tanaka@sakura.jp, +81 3 5412 7780) about
her order #A-29104 that shipped to 12 Sakura-dori. She says Kenji from
support promised a refund.
```

ChatGPT writes a polite reply about Yuki Tanaka. Shadow restores the real names and details in the answer you see, so the reply addresses Jane Doe at her real address. The order number passed through because it is not a detected format; add such references as [terms](/shadow/concepts/detection/#custom-terms) if they matter.

## Limits to be honest about

- **Detection is best effort.** Unusual names, internal ID formats and context that uniquely identifies a person can be missed. Test with `shadow scan`.
- **Dates are not masked.** A date of birth combined with other context can identify someone.
- **Images are not OCR'd.** Screenshots pass through unless you block them with `[files] images = "block"`.
- **Phone apps are not covered.** Shadow intercepts traffic on the laptop; mobile ChatGPT, WhatsApp and SMS are out of scope.

---

**Disclaimer**: This guide describes risks and technical controls. It does not make your AI use compliant with any law or standard. Involve your security, privacy and legal teams.
