---
title: "122,255 Sensitive Values Protected on One Developer Laptop: AI Leak Report #1"
seoTitle: "AI Leak Report #1: 122k sensitive values from one laptop"
description: "In 30 days Shadow masked 122,255 sensitive values in AI traffic from one developer laptop, including 287 secrets and 1,405 emails. What it found and why."
publishedDate: 2026-11-06
topics:
  - AI Security
  - Data Privacy
  - Developer Tools
  - Research
author: Shadow Research Team
readingTime: 8
series: AI Leak Report
seriesNumber: 1
keywords:
  - ai data leak
  - llm privacy
  - cursor privacy
  - claude code privacy
  - chatgpt data exposure
  - ai coding assistant security
  - api key leak prevention
  - pii in prompts
keyFindings:
  - "122,255 total sensitive values detected and masked in 30 days on one laptop"
  - "287 secrets and API keys caught before reaching model providers"
  - "1,405 email addresses and 2,759 names masked in outgoing prompts"
  - "184 national IDs and 87 card numbers protected"
  - "12 dangerous tool calls blocked by intent policy"
  - "1,669 total requests observed across AI coding assistants and chat UIs"
methodology: |
  These aggregates come from one developer laptop running Shadow over a 30-day window ending approximately November 5, 2026. Data was collected via `shadow activity --window 30d --json --no-hints`.

  **What the numbers represent:**
  - Detections of sensitive-looking strings in outgoing prompts, context windows, and related AI traffic
  - Values that were masked (replaced with realistic fakes) before reaching model providers
  - The laptop ran typical developer AI workflows: coding assistants, chat UIs, and agents

  **Important caveats:**
  - This is not a count of confirmed data breaches or verified leaks
  - Some values are duplicates (the same key resent with every agent turn)
  - Some detections may be false positives (pattern-matched but not actually sensitive)
  - One machine's usage patterns do not represent industry-wide data
  - AI tools in the mix included Cursor, Claude Code, ChatGPT, and other common developer tools

  We label these as "potential leaks" or "protected values" rather than breaches. The goal is to show what *could* leave a developer machine, not to claim definite exposure.
faq:
  - q: "What counts as a 'sensitive value' in this report?"
    a: "Any string that matches Shadow's detection patterns for PII, secrets, or sensitive business data: API keys and tokens, email addresses, names, national ID numbers (SSN, passport, etc.), payment card numbers, and custom terms configured by the user."
  - q: "Were any actual data breaches confirmed?"
    a: "No. These are detections of sensitive-looking data that Shadow intercepted and masked *before* it reached model providers. We cannot know what would have happened without the masking, and some detections may be false positives."
  - q: "Why are the numbers so high for one laptop?"
    a: "AI coding assistants and agents read files for context—potentially entire repositories. Each turn can include secrets from .env files, names from test fixtures, and emails from logs. The agent doesn't paste them; it reads them automatically. Over 30 days of active development, these add up."
  - q: "Does this mean Cursor, Claude Code, or ChatGPT are unsafe?"
    a: "These tools follow their stated data-handling policies. The issue isn't policy violations—it's that prompts and context reach providers at all. Even with training disabled and zero-data-retention agreements, the data travels over the network and is processed by provider infrastructure."
  - q: "How can I protect my own development environment?"
    a: "Keep secrets out of your workspace tree or in `.cursorignore`. Use short-lived credentials. Check what context your AI tools actually send (verbose/debug modes). Consider a masking proxy like Shadow that replaces sensitive values before the request leaves your machine."
cta:
  label: "Try Shadow free"
  href: /getting-started/quick-start/
related:
  - does-cursor-train-on-my-code
  - what-is-shadow-ai
  - what-is-an-llm-proxy
---

Developer AI tools—coding assistants, chat UIs, and agents—can accelerate work dramatically. They can also send more data to model providers than most developers realize.

Over a 30-day period ending around November 5, 2026, we tracked what Shadow detected and masked on one developer laptop. The results: **122,255 sensitive values** in outgoing AI traffic, including API keys, customer data, and internal identifiers—masked before they could reach any provider.

This is not an industry survey. It's what one active developer workstation looks like. The numbers show the *scope* of potential exposure that can accumulate silently during normal AI-assisted development.

## What the laptop revealed

| Category | Count | What gets caught |
|---|---|---|
| **Names** | 2,759 | Customer names in logs, test fixtures, commit metadata |
| **Email addresses** | 1,405 | User emails in config, support tickets in context |
| **Secrets / API keys** | 287 | Keys in .env, tokens in logs, connection strings |
| **National IDs** | 184 | SSNs, passport numbers, government identifiers |
| **Card numbers** | 87 | Test cards, production cards in debugging sessions |
| **All protected values** | **122,255** | Everything above plus addresses, IBANs, custom terms |

*Additionally: 1,669 requests observed, 12 dangerous tool calls blocked*

The 122,255 figure is large because of how AI coding tools work: they build context by reading files. Every turn of an agent conversation can include material from your entire repository. Secrets in `.env` files, customer data in test fixtures, and API keys in logs can all become prompt content.

## Why this matters now

This week, Claude Code was found putting account emails into outbound User-Agent headers because the email address was already in model context ([GitHub #107757](https://github.com/anthropics/claude-code/issues/107757)). The same class of problem applies to any data in the context window: if the agent can see it, a tool call can emit it.

**What AI coding assistants can expose:**

| Data type | How it enters context | Why it matters |
|---|---|---|
| API keys and secrets | Agent reads .env, config files, or error logs | Credential theft, unauthorized access |
| Customer emails and names | Test fixtures, support tickets, or logs in the repo | Privacy violations, GDPR exposure |
| Internal hostnames | Stack traces, config, environment variables | Network reconnaissance information |
| Database credentials | Connection strings in config or .env | Direct database access risk |
| Commit author info | Git metadata the agent reads for context | Employee identification, social engineering |

## A synthetic example

What Shadow catches and masks (values are synthetic examples, not real data):

```
Before:  DATABASE_URL=postgres://admin:hunter2@db.example.com
         OPENAI_KEY=sk-test-EXAMPLE0000000000000000000000000000

After:   DATABASE_URL=postgres://admin:<SECRET_1>@db.example.com
         OPENAI_KEY=<API_KEY_1>
```

The model sees a working prompt. The provider never sees your real credentials. When the response arrives, Shadow restores the original values so your code runs correctly.

## What helps, regardless of tooling

Based on what we've observed:

1. **Keep secrets out of the repo tree.** Store credentials in environment variables, a secrets manager, or a `.env` file listed in `.gitignore` *and* `.cursorignore` (or equivalent for your tool).

2. **Use short-lived, scoped credentials.** Temporary tokens limit damage if one leaks. Scoped permissions mean a leaked key can't do everything.

3. **Check what your tools actually send.** Most AI tools have verbose or debug modes that show the full request. Inspect the context, headers, and tool arguments—not just the chat box.

4. **Audit test fixtures and logs.** Real customer data often lingers in test data, support exports, and error logs. The agent will read them if they're in your workspace.

5. **Mask at the network boundary.** A privacy proxy detects sensitive values and replaces them before the request leaves your machine. This works even for files you didn't know contained sensitive data.

## Series introduction

This is the first **AI Leak Report**, a recurring publication tracking what sensitive data developer AI tools can send to model providers and practical ways to reduce it.

Future reports will cover specific tools, different work patterns, and what the community reports. If you've measured this in your own setup, we'd like to hear about it.

## About Shadow

[Shadow](/shadow/) is an open-source (Apache-2.0) privacy layer for AI. It runs locally, masks PII and secrets in prompts before they reach ChatGPT, Claude, Cursor, and other LLM APIs, and restores real values in responses. The mapping stays in an encrypted vault on your machine—no cloud service, no account, no telemetry.

For teams, [Shadow Hub](/shadow/hub/) shows which AI tools your people use and what was masked or blocked, per user and per tool, without collecting prompt text.

---

**Disclosure**: This report was produced by the Shadow team. We build the masking proxy that collected these statistics. We've tried to be precise about what the numbers represent and what they don't.
