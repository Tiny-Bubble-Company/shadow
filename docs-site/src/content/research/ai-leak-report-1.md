---
title: "AI Leak Report #1: 122,255 sensitive values caught on one developer laptop"
seoTitle: "AI Leak Report #1: 122k sensitive values from one laptop"
description: "In 30 days Shadow masked 122,255 sensitive values in AI traffic from one developer laptop, including 287 secrets and 1,405 emails. What it found and why."
publishedDate: 2026-11-06
topics:
  - AI Security
  - Data Privacy
  - Developer Tools
author: Shadow Research Team
readingTime: 6
series: AI Leak Report
seriesNumber: 1
keywords:
  - ai data leak
  - llm privacy
  - claude code privacy
  - ai coding assistant security
  - api key leak prevention
  - pii in prompts
keyFindings:
  - "<strong>122,255</strong> sensitive values detected and masked in 30 days on one laptop"
  - "<strong>287</strong> secrets and API keys replaced before they reached a model provider"
  - "<strong>1,405</strong> email addresses and <strong>2,759</strong> names masked in outgoing prompts"
  - "<strong>184</strong> national IDs and <strong>87</strong> card numbers protected"
  - "<strong>12</strong> dangerous tool calls blocked by the intent policy"
  - "<strong>1,669</strong> AI requests observed from coding agents and chat apps"
methodology: |
  The numbers come from one developer laptop running Shadow Core for the 30 days ending 5 November 2026, read with `shadow activity --window 30d --json --no-hints`. Shadow's activity log holds counts and categories only, never the values themselves.

  **What the numbers are:**
  - Sensitive values Shadow detected in outgoing AI requests (prompts, context and tool results) and replaced with realistic fakes before the request left the machine
  - Counted per request: a value resent on every turn of an agent conversation is counted each time
  - Traffic from AI tools whose requests Shadow can read: coding agents that call model APIs and supported chat apps

  **What they are not:**
  - Confirmed breaches or verified leaks: the values were masked, so none of them reached a provider
  - Free of false positives: some pattern matches are not truly sensitive
  - Representative of every developer or company: this is one machine's work pattern
  - Complete: tools whose protocol Shadow cannot read, such as Cursor's, are recognised but their contents are not inspected, so they add no values to these counts
faq:
  - q: "What counts as a sensitive value in this report?"
    a: "Any string Shadow's detectors classed as personal data, a secret or a custom term: API keys and tokens, email addresses, names, national ID numbers, payment card numbers, hostnames, IP addresses, usernames, addresses and the organisation's own terms."
  - q: "Were any data breaches confirmed?"
    a: "No. These are sensitive-looking values Shadow intercepted and replaced *before* they reached a model provider. Some detections may be false positives, and nobody can say what would have happened without the masking."
  - q: "Why are the numbers so high for one laptop?"
    a: "Coding agents read files for context and resend that context on every turn. One secret in a config file, or a page of names in a test fixture, is counted each time it is sent. Over a month of active development the counts add up quickly."
  - q: "Does this mean Claude Code or ChatGPT are unsafe?"
    a: "No. These tools follow their published data-handling policies. The point is that prompts and context reach providers at all: even with training off and zero-data-retention terms, the data travels over the network and is processed on the provider's infrastructure."
  - q: "How can I protect my own development environment?"
    a: "Keep secrets out of the workspace, list `.env` files in your tool's ignore file (such as `.cursorignore`), use short-lived credentials, check what your tools send with their debug modes, and route agents that call model APIs through a masking proxy such as Shadow."
cta:
  label: "Install Shadow Core free"
  href: /getting-started/quick-start/
related:
  - does-cursor-train-on-my-code
  - what-is-shadow-ai
  - what-is-an-llm-proxy
---

**In 30 days, Shadow caught and masked 122,255 sensitive values in the AI traffic of one developer laptop: 287 secrets and API keys, 1,405 email addresses, 2,759 names, 184 national IDs and 87 card numbers, across 1,669 requests.** Each value was replaced with a realistic fake before the request left the machine. The total is high because coding agents resend the files they have read on every turn, so one secret in a config file is counted each time it is sent.

This is not an industry survey. It is what one active developer workstation sends over a month of normal AI-assisted work, published to show how much exposure builds up without anyone noticing.

## What did the laptop send to AI tools?

| Category | Count | Where it usually comes from |
|---|---|---|
| **Names** | 2,759 | Customer names in logs, test fixtures and commit metadata |
| **Email addresses** | 1,405 | User emails in config, support tickets read into context |
| **Secrets and API keys** | 287 | Keys in `.env`, tokens in logs, connection strings |
| **National IDs** | 184 | Social security, passport and other government numbers in test data |
| **Card numbers** | 87 | Test cards and card numbers in debugging sessions |
| **Other types** | 117,533 | The remaining detected types, such as hostnames, IP addresses, usernames, addresses and custom terms |
| **All protected values** | **122,255** | Every value replaced before it left the laptop |

In the same window Shadow observed 1,669 AI requests and blocked 12 dangerous tool calls with its [intent policy](/shadow/concepts/intent-policy/).

## Why are the numbers so high?

Coding agents build context by reading files, and every turn of a conversation resends that context. Secrets in `.env` files, customer data in test fixtures and API keys in logs all become prompt content, again and again. The other 117,533 values are the remaining types Shadow detects, such as hostnames, IP addresses, usernames and the organisation's own terms; this report does not break them down further.

## Why does this matter now?

This week, Claude Code was reported to put the account's email address into outbound User-Agent headers because the address was already in the model's context ([anthropics/claude-code#107757](https://github.com/anthropics/claude-code/issues/107757)). The same applies to any data in the context window: if the agent can see it, a tool call can send it somewhere.

| Data type | How it enters context | Why it matters |
|---|---|---|
| API keys and secrets | The agent reads `.env`, config files or error logs | Credential theft and unauthorised access |
| Customer emails and names | Test fixtures, support tickets or logs in the repository | Privacy breaches and GDPR exposure |
| Internal hostnames | Stack traces, config, environment variables | A map of your internal network |
| Database credentials | Connection strings in config or `.env` | Direct database access |
| Commit author details | Git metadata the agent reads | Identifies employees for phishing |

## What does masking look like?

An illustrative example, not data from the laptop. Shadow replaces each value with a fake of the same shape, so the prompt still makes sense to the model:

```text
Sent by the agent:   DATABASE_URL=postgres://admin:skobayashi2@db.yamamoto-corp.internal
                     OPENAI_API_KEY=sk-proj-Q7wKIHXZ4097704176058327

Received by the AI:  DATABASE_URL=postgres://admin:Lq0vT8d2xW@db.nexora-dev.internal
                     OPENAI_API_KEY=sk-proj-mB2rTnYp8814620093715542
```

The provider never sees the real credentials or hostname. When the answer comes back, Shadow puts the real values back, including inside tool-call arguments, so the code the agent writes still works. Teams that never want a secret sent in any form can set `secret = "redact"` under [`[policy.always]`](/shadow/reference/configuration/#policyalways).

## How can developers reduce the exposure?

From what this laptop sent, five habits matter most:

1. **Keep secrets out of the repository tree.** Store credentials in environment variables, a secrets manager, or a `.env` file listed in `.gitignore` and in your AI tool's ignore file.
2. **Use short-lived, scoped credentials.** A leaked temporary token expires; a scoped one can do little.
3. **Check what your tools actually send.** Most AI tools have a verbose or debug mode that shows the full request. Look at the context, headers and tool arguments, not just the chat box.
4. **Audit test fixtures and logs.** Real customer data lingers in test data, support exports and error logs, and agents read them.
5. **Mask on the machine.** A privacy proxy replaces sensitive values before the request leaves the laptop, including in files you did not know held sensitive data. It works for tools whose traffic it can read; see [which tools Shadow masks](/shadow/reference/providers/).

## About the AI Leak Report

The **AI Leak Report** is a recurring Shadow Research series on what sensitive data developer AI tools send to model providers, and how to reduce it. Future reports will cover specific tools, team-wide numbers and other ways of working. If you have measured this in your own setup, write to [support@getshadow.dev](mailto:support@getshadow.dev).

## About this research

Produced by the Shadow Research Team from Shadow Core's activity log on one laptop. [Shadow](/shadow/) is an open-source (Apache-2.0) privacy layer for AI: it runs on your machine, masks personal data and secrets in prompts to the AI APIs and chat apps it supports, and restores the real values in the answer. The real-to-fake mapping stays in an encrypted vault on the machine, with no account and no telemetry. For teams, [Shadow Hub](/shadow/hub/) shows these numbers per person, app and AI service, at the [sharing level](/shadow/trust/) the organisation chooses.

**Disclosure:** we build the tool that collected these numbers, and have tried to be precise about what they represent and what they do not.
