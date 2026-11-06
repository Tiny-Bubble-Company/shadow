---
title: "Does Cursor train on my code? Privacy Mode, retention and what still leaves"
seoTitle: "Does Cursor train on my code? Privacy Mode explained"
description: "Only if Privacy Mode is off. With it on, Cursor and its model providers do not train on your code, but every prompt still goes through Cursor's servers."
publishedDate: 2026-10-15
updatedDate: 2026-11-06
topics:
  - AI Coding Tools
  - Privacy Controls
  - Developer Tools
author: Shadow Research Team
readingTime: 7
keywords:
  - does cursor train on my code
  - cursor privacy mode
  - cursor data retention
  - is cursor safe for company code
  - cursor enterprise privacy
  - ai coding assistant privacy
faq:
  - q: "Does Cursor train on my code?"
    a: "Only when Privacy Mode is off. Cursor's data-use page says that with Privacy Mode off it may use and store codebase data, prompts and code snippets to improve its AI features and train its models. With Privacy Mode on, Cursor does not train on your code and its model providers keep no data under zero-data-retention agreements."
  - q: "Is Cursor Privacy Mode on by default?"
    a: "For Teams and Enterprise plans, yes: Cursor enables it for every member. Individual Free and Pro users choose it in Settings, so check that it is on."
  - q: "Does Privacy Mode stop my code leaving my machine?"
    a: "No. Privacy Mode is a promise about training and retention, not about transmission. Prompts, open files, codebase context and terminal output still travel to Cursor's servers and on to model providers such as OpenAI and Anthropic."
  - q: "Does using my own OpenAI or Anthropic API key keep data away from Cursor?"
    a: "No. Cursor says requests still go through its backend, where the final prompt is built, even with your own key. Cursor also notes that its zero-data-retention agreements do not apply when you use your own API keys."
  - q: "Can Shadow mask what Cursor sends?"
    a: "Not today. Cursor talks to its backend in a binary protocol that Shadow cannot rewrite, so Shadow recognises Cursor's traffic and shows it as seen, not protected. Shadow masks Claude Code, Codex CLI, Gemini CLI, GitHub Copilot, Aider, Cline, Continue and any tool that calls a model API directly."
  - q: "How do I stop secrets in .env files reaching the model?"
    a: "Keep real secrets out of the workspace, list `.env` files in `.cursorignore`, use short-lived credentials, and rotate any key an agent may have read. For agents that call model APIs directly, a masking proxy such as Shadow replaces secrets before the request leaves the laptop."
cta:
  label: "Install Shadow Core free"
  href: /getting-started/quick-start/
related:
  - is-chatgpt-safe-for-company-data
  - ai-leak-report-1
  - what-is-an-llm-proxy
---

**Cursor trains on your code only if Privacy Mode is off.** With Privacy Mode on (the default for Teams and Enterprise), Cursor does not use your code for training and its model providers keep no data under zero-data-retention agreements. Neither setting stops your code from leaving the machine: prompts, open files, codebase context and terminal output still go to Cursor's servers and on to model providers such as OpenAI and Anthropic.

## What does Cursor's Privacy Mode change?

| Setting | Training | Retention at model providers | What it does not change |
|---|---|---|---|
| **Privacy Mode off** | Cursor may use and store codebase data, prompts and code snippets to improve its features and train models | Provider terms apply | Data is sent and may be kept |
| **Privacy Mode on** | No training by Cursor | Zero data retention with Cursor's providers, with exceptions Cursor lists (abuse review, some models that require retention) | Data still leaves your machine and passes through Cursor's backend |
| **Teams / Enterprise** | Privacy Mode on by default for every member | As above, plus admin controls | Same as Privacy Mode on |
| **Your own API key** | Your provider's terms | Cursor's ZDR agreements do not apply | Requests still go through Cursor's backend |

Sources: Cursor's [data use](https://cursor.com/data-use), [privacy policy](https://cursor.com/privacy) and [security and privacy help](https://cursor.com/help/security-and-privacy/privacy), read on 6 November 2026. Cursor's policy pages differ in wording; the data-use page is the most specific about training.

## What still leaves your machine?

Privacy Mode governs what happens to data after it arrives. It does not scan, redact or reduce what is sent. In a normal session that includes:

| What leaves | Example |
|---|---|
| Prompt text | "Refactor this function to be async" |
| Files and snippets the agent reads | Source files, config files, READMEs |
| Terminal output and errors | Stack traces with internal hostnames, IPs and usernames |
| Secrets anywhere in the workspace | `.env` values, API keys in config, tokens in logs |
| Git metadata | Commit author names and emails |
| Data in fixtures and exports | Customer emails and order numbers in test data |

A secret that reaches a provider should be treated as exposed and rotated, whatever the provider's terms say. Our [AI Leak Report #1](/shadow/research/ai-leak-report-1/) counted 287 secrets in 30 days of AI traffic from one developer laptop.

## Is Cursor safe for company code?

It can be, with three conditions: Privacy Mode on (enforced at team level), no real secrets or customer data in the workspace, and a clear policy for which repositories may be opened in it. For regulated code or code under strict client confidentiality, treat Cursor like any cloud service that receives your source: get the agreement reviewed, and keep the most sensitive repositories in tools you can control.

## How do I protect code in Cursor?

1. **Turn on Privacy Mode**, and on a Teams or Enterprise plan confirm it is enforced for everyone.
2. **Keep secrets out of the workspace.** Use environment variables or a secrets manager, never commit real keys, and remember the agent can read git history.
3. **Exclude sensitive paths** with a `.cursorignore` file at the project root:

   ```text
   .env*
   config/production.yml
   test/fixtures/customer_exports/
   ```

4. **Use short-lived, scoped credentials** so a leaked key expires and can do little.
5. **Audit fixtures and logs** for real customer data before an agent reads them.

## Can a masking proxy protect Cursor?

Not Shadow, today. Cursor sends prompts to its own backend in a binary protocol and builds the final prompt there, so a local proxy cannot read or rewrite the content. Shadow recognises Cursor's hosts and shows its requests on the dashboard as **seen, not protected**, which tells you how much Cursor traffic a machine sends, but it does not mask them. See the [provider catalog](/shadow/reference/providers/).

Shadow does mask coding agents that call model APIs directly: Claude Code, Codex CLI, Gemini CLI, GitHub Copilot, Aider, Cline, Continue and OpenCode. For those, names, emails, secrets, hostnames and your own terms are replaced with realistic fakes before the request leaves the laptop and restored in the answer and in tool calls. Setup: [protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/).

| Approach | Training risk | Data leaves the machine? | Secrets protected? |
|---|---|---|---|
| Cursor, Privacy Mode off | Possible | Yes | No |
| Cursor, Privacy Mode on | None by contract | Yes, through Cursor's backend | No |
| Agent calling a model API through Shadow | Per provider terms | Yes, with fakes in place of real values | Yes |
| Local model (Ollama, vLLM) with a local agent | None | No | Yes |

## What about GitHub Copilot?

Copilot has the same shape: prompts and code context reach GitHub's servers, and settings control retention and training rather than transmission. Unlike Cursor, Copilot's requests use an OpenAI-style JSON format, so Shadow can mask them when the editor's traffic goes through Shadow (the IDE's HTTP proxy setting and Shadow's trusted CA, or the system proxy).

## About this research

Written by the Shadow Research Team from Cursor's published policies and Shadow's own provider catalog. [Shadow](/shadow/) is an open-source (Apache-2.0) privacy layer for AI from [Tiny Bubble Company](https://github.com/Tiny-Bubble-Company): it runs on your machine, masks personal data and secrets in prompts to the AI APIs and chat apps it supports, and restores the real values in the answer. It is not affiliated with Cursor or Anysphere.

**Disclaimer:** this article describes publicly stated policies and technical controls as of the date above. It is not legal advice. Policies change; check Cursor's current terms before relying on them.
