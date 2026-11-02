---
title: "Shadow Hub vs Prompt Security: AI data protection for teams"
seoTitle: "Prompt Security vs Shadow Hub: AI data protection"
description: "Prompt Security (SentinelOne) vs Shadow. Prompt Security redacts and enforces AI policy; Shadow masks prompts with reversible fakes and restores answers."
sidebar:
  label: Prompt Security
  order: 32
keywords:
  - prompt security
  - prompt security alternative
  - prompt security vs shadow
  - prompt security sentinelone
  - prompt security pricing
  - open source prompt security alternative
  - AI DLP
  - GenAI DLP
faq:
  - q: "Is Shadow an alternative to Prompt Security?"
    a: "For keeping sensitive data out of AI prompts, partly. Prompt Security, from SentinelOne, is a broad AI security platform covering employee AI use, developer tools, homegrown AI apps and agents. Shadow does one job: Core masks PII and secrets with reversible fakes on each machine, and Shadow Hub (early access) shows teams what was masked or blocked."
  - q: "Is Prompt Security part of SentinelOne?"
    a: "Yes. Prompt Security brands itself as Prompt Security from SentinelOne, and its platform pages are on sentinelone.com as part of the Singularity Platform."
  - q: "Does Prompt Security redact sensitive data?"
    a: "Yes. SentinelOne says Prompt Security redacts sensitive data and enforces policies across 15,000+ AI services, and that its AI Usage Control redacts data before a prompt leaves the device. Its pages do not describe restoring the original values in answers; Shadow does."
  - q: "How much does Prompt Security cost?"
    a: "Not published (as of 2 Nov 2026). SentinelOne's packages page lists prices for Singularity endpoint packages but none for Prompt Security. Shadow Core is free. Shadow Hub's planned pricing is free for 1-5 users, then from $8 per user per month; these prices are provisional."
  - q: "Is Shadow Hub available now?"
    a: "Not generally. Shadow Hub is in early access with a waitlist. Shadow Core is available today and free."
---

**Choose Prompt Security if you want a broad AI security platform from SentinelOne that governs employee AI use, developer tools, homegrown AI apps and agents, including prompt-injection defence. Choose Shadow if your goal is narrower: mask sensitive data in prompts with reversible fakes so answers stay useful. Core is free; Shadow Hub is in early access.**

[Shadow Hub for teams](/shadow/hub/) · [Install Core free](/shadow/getting-started/quick-start/)

## At a glance

| | Prompt Security | Shadow Core | Shadow Hub |
|---|---|---|---|
| **Deployment model** | Commercial platform, part of SentinelOne's Singularity Platform | Free local proxy, a single binary on each laptop or server | Hosted console in early access (waitlist); members run one command that installs Core and enrols the machine |
| **Where it runs** | Browser, desktop and embedded copilots for employees; developer tools such as GitHub Copilot, Cursor and Claude Code; homegrown AI apps; agents and MCP servers | On the device, between apps and AI services: model APIs, coding agents, and with the system proxy on, ChatGPT, Claude.ai, Gemini, Perplexity, DeepSeek and Grok in a browser | Masking stays on each machine; the hub receives what the organisation chooses to share: counts only, sanitised prompts or full detail |
| **Masking vs blocking** | Redacts sensitive data, coaches users, blocks unsanctioned tools and adversarial prompts | Masks PII and secrets with realistic fakes; blocks file uploads by policy | Same as Core, plus a view of what was masked or blocked per person, machine, app and AI service |
| **Reversible pseudonymisation** | Not described on its public pages | Yes, real values restored in answers | Yes, through Core |
| **Open source** | Platform: no. Some separate tools (ps-fuzz, ClawSec) are on GitHub | Yes, Apache-2.0 | No, commercial hosted service |
| **Published pricing** | No, not published (as of 2 Nov 2026) | Free | Provisional: free for 1-5 users, then from $8 per user per month |
| **Best for** | Organisations that want AI usage control, AI app security and agent security from one vendor, especially SentinelOne customers | Developers and engineers using model APIs and coding agents | Teams that want to see what Core masked or blocked across members, without collecting real values unless they choose to |

## When to choose Prompt Security

- **You need more than data protection.** Prompt Security covers prompt injection and jailbreak testing before deployment, runtime protection for homegrown AI apps, and an AI firewall that blocks adversarial prompts.
- **You govern agents and MCP.** It maps agents and MCP servers, discovers shadow MCP servers and enforces least-privilege access.
- **You want wide AI service coverage.** SentinelOne says policies apply across 15,000+ AI services, and that unsanctioned tools can be blocked.
- **You already run SentinelOne.** Prompt Security is part of the Singularity Platform, so it fits an existing SentinelOne estate.
- **You want coaching.** Employees are coached in the moment rather than simply blocked.

## When to choose Shadow

- **Answers must keep working.** Shadow replaces names, emails, card numbers and secrets with realistic, consistent fakes, so the model can still reason about them, and restores the real values in the answer. See [how it works](/shadow/concepts/how-it-works/).
- **You want a focused, inspectable tool.** Core is open source under Apache-2.0, needs no account and runs on the machine. It recognises 288 AI services and sits in front of model APIs and coding agents such as Claude Code, Codex, Gemini CLI and GitHub Copilot. See the [quick start](/shadow/getting-started/quick-start/).
- **You want to start bottom-up.** Developers can install Core today; the team can add Shadow Hub later.
- **You want a team view without collecting real values.** With [Shadow Hub](/shadow/hub/) (early access, waitlist), admins invite members by link and see what was masked or blocked per person, machine, app and AI service. Each organisation chooses whether machines share counts only, sanitised prompts or full detail.

Know Shadow's limits before choosing it. It does not cover phone apps, apps that pin certificates, WebSocket features, or apps that ignore the system proxy unless pointed at it. Cursor, Windsurf, Amazon Q and Kiro use binary protocols Shadow cannot mask; it shows them as "seen, not protected". Shadow does not detect prompt injection, is not a browser extension, has no SaaS app connectors, no DLP for files on disk and no SOC 2 or other certification.

## Sources

Prompt Security facts on this page come from Prompt Security's and SentinelOne's own public pages:

- [Prompt Security home page](https://www.prompt.security/) (fetched 2 Nov 2026): "Prompt Security from SentinelOne" branding; open-source tools ps-fuzz and ClawSec on GitHub.
- [SentinelOne: Securing AI with Prompt Security](https://www.sentinelone.com/platform/securing-ai-prompt/) (fetched 2 Nov 2026): Singularity Platform; redaction across 15,000+ AI services; AI firewall; developer tools; homegrown apps; agents and MCP.
- [SentinelOne: Prompt Security AI Usage Control](https://www.sentinelone.com/platform/prompt-security-ai-usage-control/) (fetched 2 Nov 2026): redaction before a prompt leaves the device; coaching; blocking unsanctioned tools; browser, desktop and embedded copilots.
- [SentinelOne platform packages](https://www.sentinelone.com/platform-packages/) (fetched 2 Nov 2026): no published price for Prompt Security.

Shadow facts: [how it works](/shadow/concepts/how-it-works/), [supported providers](/shadow/reference/providers/), [proxy mode](/shadow/integrate/proxy-mode/).

---

*Last reviewed: 2 Nov 2026. Prompt Security and SentinelOne are trademarks of their owners; this comparison is based on their public pages as of the review date. See also [AI DLP compared](/shadow/compare/ai-dlp/).*
