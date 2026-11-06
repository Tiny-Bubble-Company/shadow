---
title: "Does Cursor Train on My Code? Privacy, Data Retention and Residual Risks"
seoTitle: "Does Cursor train on my code?"
description: "Cursor does not train on your code by default, but prompts, file context and terminal output still reach model providers. What leaves, and how to mask it."
publishedDate: 2026-10-15
updatedDate: 2026-11-06
topics:
  - AI Coding Tools
  - Privacy Controls
  - Developer Security
author: Shadow Research Team
readingTime: 10
keywords:
  - does cursor train on my code
  - cursor privacy
  - cursor ai data privacy
  - is cursor safe for company code
  - cursor enterprise privacy
  - cursor privacy mode
  - ai coding assistant privacy
  - cursor data retention
faq:
  - q: "Does Cursor train on my code?"
    a: "No. Cursor states it does not train models on your code by default. Enabling Privacy Mode adds a contractual guarantee that your data is never used for training. Enterprise plans include the same guarantee and additional controls such as zero-data-retention (ZDR) with providers."
  - q: "Is Cursor safe for company code?"
    a: "That depends on your risk tolerance. Cursor does not train on code, but prompts, file snippets, terminal output and any secrets in your workspace still reach model providers such as OpenAI or Anthropic over the internet. For regulated or highly sensitive code, combine Privacy Mode with a local masking proxy that removes secrets and identifiers before they leave."
  - q: "What is Cursor Privacy Mode?"
    a: "A setting in Cursor's preferences that makes code and prompts ineligible for training and restricts their use to providing the service. It does not stop data from leaving your machine; it is a contractual commitment about how that data is handled."
  - q: "Does Cursor send my code to OpenAI or Anthropic?"
    a: "Yes. When you use a model like GPT-4 or Claude in Cursor, your prompts and any file context the agent reads are sent to the provider's API. Cursor routes the traffic, but the provider's servers process it. Privacy Mode and enterprise ZDR terms control retention and training, not transmission."
  - q: "How do I stop secrets in .env files reaching the model?"
    a: "Do not rely on Cursor settings alone. Keep .env files outside your workspace or in .cursorignore, and route API traffic through a local masking proxy like Shadow that detects and replaces secrets before they leave your machine."
  - q: "Can my company use Cursor without data leaving the network?"
    a: "Not with Cursor's hosted models. You can point Cursor at a local OpenAI-compatible endpoint (such as Ollama or vLLM) so prompts never leave, but you lose frontier model quality. For cloud models with privacy, use a masking proxy to strip sensitive values before egress."
cta:
  label: "Try Shadow free"
  href: /getting-started/quick-start/
related:
  - is-chatgpt-safe-for-company-data
  - private-chatgpt
  - what-is-an-llm-proxy
---

**Cursor does not train models on your code by default.** Enabling Privacy Mode adds a contractual guarantee that prompts and codebase snippets are never used for training and are retained only as long as needed for the service. Enterprise customers can negotiate zero-data-retention (ZDR) terms with the underlying providers. None of these settings stop your code from leaving your machine: prompts, file context, terminal output and anything the agent reads are still sent over the internet to model providers such as OpenAI and Anthropic.

## What Cursor's privacy controls actually do

| Control | Who has it | What it does | What it does not do |
|---|---|---|---|
| **Default (no Privacy Mode)** | Everyone | Cursor states it does not train on your code. Provider terms apply. | Does not prevent prompts reaching provider APIs or being logged for abuse monitoring. |
| **Privacy Mode** | Everyone (Settings → Privacy) | Adds a contractual commitment that your data is never used for training and is used only to provide the service. | Does not stop data leaving your machine or being processed by provider infrastructure. |
| **Enterprise / Business** | Paid orgs | Adds SSO, admin controls, audit logs, and the option to negotiate ZDR agreements with providers. | Still routes traffic through provider APIs unless you bring your own endpoint. |
| **Local model endpoint** | Self-configured | Points Cursor at a local OpenAI-compatible server; prompts never leave. | You lose GPT-4o/Claude-class quality and take on model hosting. |

Sources: [Cursor Privacy Policy](https://www.cursor.com/privacy) and [Cursor Security](https://www.cursor.com/security).

## The residual risk: what still leaves your machine

Even with Privacy Mode on and an enterprise contract signed, every prompt you send and every file the agent opens travels to a model provider. That includes:

| What leaves | Example |
|---|---|
| Prompt text | "Refactor this function to be async" |
| File snippets the agent reads for context | Entire source files, config files, READMEs |
| Terminal output and error logs | Stack traces with internal hostnames, IPs, usernames |
| Secrets if they are anywhere in the workspace | `.env` values, API keys in config, tokens in logs |
| Git metadata | Commit author names and emails |
| Internal identifiers | Database IDs, order numbers, customer emails in test fixtures |

### Secrets deserve special attention

AI coding assistants read files to build context. If your `.env`, `config/production.yml` or test fixtures contain real API keys, database passwords or customer data, the agent can include them in a prompt without you noticing. Cursor's Privacy Mode does not scan or redact content; it only promises not to train on it after it arrives. A secret that reaches the provider is a secret you should rotate, regardless of what the provider's terms say.

## Comparison: settings vs contracts vs masking

| Approach | Training risk | Data leaves your machine? | Secrets protected? | Effort |
|---|---|---|---|---|
| Default Cursor (no Privacy Mode) | Low (Cursor says no training) | Yes | No | None |
| Privacy Mode on | Contractually zero | Yes | No | One toggle |
| Enterprise ZDR agreement | Contractually zero, auditable | Yes | No | Procurement |
| Local masking proxy ([Shadow](/shadow/)) | Contractually zero + values replaced | Yes, but with fakes | Yes | Install and configure |
| Local model (Ollama, vLLM) | Zero (nothing leaves) | No | Yes | Host your own model |

A masking proxy like Shadow sits between Cursor and the provider API. It detects names, emails, phone numbers, card numbers, secrets, internal hostnames and your own terms in the request, replaces them with realistic fakes, and restores the real values in the response. The provider never sees the originals. You keep frontier model quality and add a technical control instead of relying solely on contracts.

## Practical steps to protect code in Cursor

### 1. Turn on Privacy Mode

In Cursor, go to **Settings → Privacy → Privacy Mode** and enable it. This is the minimum baseline for work code.

### 2. Keep secrets out of the workspace

- Store API keys and credentials in environment variables, a secrets manager or a `.env` file listed in `.cursorignore`.
- Never commit real secrets to version control; the agent can read git history.
- Audit test fixtures and config files for leftover production values.

### 3. Exclude sensitive directories

Add paths you never want the agent to read to `.cursorignore` at the project root:

```text
.env*
config/production.yml
test/fixtures/customer_exports/
```

### 4. Point Cursor at a local masking proxy

[Shadow](/shadow/) intercepts requests to OpenAI, Anthropic and other providers and masks sensitive values before they leave. Install it on your laptop:

```sh
curl -fsSL https://hub.getshadow.dev/core/install.sh | sh
shadow init --org "Your Company"
```

Configure Shadow for code (in `shadow.toml`):

```toml
organization = ["Your Company"]
internal_domains = ["corp.example.com", "prod.internal"]

[policy.always]
secret = "redact"
```

Start Shadow and configure Cursor to route traffic through the proxy:

```sh
shadow install --apply
```

Then set Cursor's HTTP proxy to `http://127.0.0.1:8877` in your IDE settings, or export `HTTPS_PROXY` for terminal-based agent commands. See the [protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/) guide for detailed setup.

### 5. Alternatively: use a local model

If nothing may leave your network, point Cursor at a local OpenAI-compatible endpoint:

1. Install [Ollama](https://ollama.com/) and pull a coding model (`ollama pull codellama` or `ollama pull deepseek-coder`).
2. In Cursor, add a custom model with base URL `http://localhost:11434/v1`.

You get full air-gap privacy but accept weaker model quality for complex reasoning and large codebases.

## What about GitHub Copilot?

GitHub Copilot has similar dynamics: prompts reach GitHub/OpenAI servers, and settings control training opt-out and telemetry but not transmission. Shadow does not intercept Copilot's proprietary backend today (see [which assistants are covered](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/#which-assistants-are-covered)). For Copilot privacy, rely on the business plan's telemetry controls and keep secrets out of the workspace.

## About Shadow

[Shadow](/shadow/) is an open-source (Apache-2.0) privacy proxy for AI, developed by the [Tiny Bubble Company](https://github.com/Tiny-Bubble-Company). It runs locally, masks PII and secrets in prompts to ChatGPT, Claude, Cursor (when using provider APIs) and other LLMs, and restores the real values in responses. Nothing is sent to a third-party service; the real-to-fake mapping stays in an encrypted vault on your machine.

For teams, [Shadow Hub](/shadow/hub/) shows which AI tools your people use and what was masked or blocked, per user and per tool, without collecting prompt text. Hub is free for up to 3 users, then $3 per user per month up to 500 users. See [Pricing](/shadow/pricing/).

---

**Disclaimer**: This guide describes technical controls and publicly stated policies. It does not constitute legal advice or guarantee compliance with any regulation. Involve your security, legal and privacy teams before using AI coding assistants on sensitive codebases.
