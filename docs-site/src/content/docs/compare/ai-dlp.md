---
title: "AI DLP: endpoint DLP vs browser extension vs LLM proxy"
seoTitle: "AI DLP: AI data loss prevention approaches compared"
description: "AI DLP compared: endpoint DLP, browser extensions and LLM proxies. What each sees, whether it blocks, redacts or masks reversibly, and when to pick Shadow."
sidebar:
  label: AI DLP compared
  order: 2
keywords:
  - ai dlp
  - ai data loss prevention
  - AI DLP tools
  - GenAI DLP
  - LLM DLP
  - ChatGPT DLP
  - block vs redact vs pseudonymise
  - LLM proxy
faq:
  - q: "What is AI DLP?"
    a: "AI data loss prevention (AI DLP) stops sensitive data such as personal data, customer records, source code and secrets from reaching AI tools and model providers. Products do this by blocking the request, redacting the sensitive values, or replacing them with reversible pseudonyms."
  - q: "What are the main types of AI DLP tool?"
    a: "Three: endpoint DLP agents that watch activity on the device, browser extensions that act on what people type, paste and upload in AI web apps, and LLM proxies such as Shadow that sit between apps and AI services and change the request itself."
  - q: "Should an AI DLP tool block or redact?"
    a: "Blocking is simplest for data that must never reach an external AI, but it stops the task. Redaction lets the request through without the value but can weaken the answer. Reversible pseudonymisation sends realistic stand-ins and restores the real values in the response."
  - q: "Is there an open-source AI DLP tool?"
    a: "Shadow Core is open source (Apache-2.0). It masks PII and secrets in requests to AI services with reversible fakes on each laptop or server. Shadow Hub, the commercial team console, has a 14-day free trial and is free for up to 3 users."
  - q: "Does AI DLP make us compliant?"
    a: "No tool makes an organisation compliant on its own. AI DLP helps meet requirements such as data minimisation and confidentiality as part of a wider programme."
---

**Choose endpoint DLP or a browser extension if employees pasting into AI web apps is your main risk and you want one policy across SaaS, files and the browser. Choose an LLM proxy such as Shadow if AI traffic comes from developers, agents and scripts and answers must keep working. Core is free; Shadow Hub is free for up to 3 users.**

[Shadow Hub for teams](/shadow/hub/) · [Install Core free](/shadow/getting-started/quick-start/)

## At a glance

The three approaches below are categories. The vendor examples in each column come from the sources at the end of this page.

| | Endpoint DLP | Browser-extension AI DLP | Shadow Core (LLM proxy) | Shadow Hub |
|---|---|---|---|---|
| **Deployment model** | Agent on each managed device, with a central console (for example Microsoft Purview Endpoint DLP, Nightfall) | Extension pushed to managed browsers, with a central console (for example LayerX, now Akamai Workforce Protector; Harmonic Security) | Free local proxy, a single binary on each laptop or server | Hosted console with a 14-day free trial; members run one command that installs Core and enrols the machine |
| **Where it runs** | On the device: browser activity, clipboard, uploads, file movement | Inside the browser: text input, copy and paste, uploads and downloads | Between apps and AI services: model APIs, coding agents, and with the system proxy on, ChatGPT, Claude.ai, Gemini, Perplexity, DeepSeek and Grok in a browser | Masking stays on each machine; the hub receives what the organisation chooses to share: counts only, sanitised prompts or full detail |
| **Masking vs blocking** | Mostly warn and block; some vendors also redact | Warn, coach, block; some vendors also redact | Masks PII and secrets with realistic fakes; blocks file uploads by policy | Same as Core, plus a view of what was masked or blocked per person, machine, app and AI service |
| **Reversible pseudonymisation** | Not described by the vendors cited | Not described by the vendors cited | Yes, real values restored in answers | Yes, through Core |
| **Open source** | No, for the vendors cited | No, for the vendors cited | Yes, Apache-2.0 | No, commercial hosted service |
| **Published pricing** | Varies; Nightfall's amounts are not published (as of 2 Nov 2026) | Varies; Harmonic and LayerX: not published (as of 2 Nov 2026) | Free | Provisional: free for 1-3 users, then $3 per user per month |
| **Best for** | One DLP policy across files, SaaS and AI on managed devices | Employees using AI and SaaS apps in the browser | Developers, coding agents and scripts calling AI services | Teams that want a view of what Core masked or blocked, without collecting real values unless they choose to |

## Three things a tool can do with sensitive data

| Approach | What the model sees | User experience | Main risk |
|---|---|---|---|
| **Block** | Nothing; the request is refused | Task fails; user may be coached | People route around it (personal accounts, phones) |
| **Redact** | `[REDACTED]`, `<PERSON>` or a masked value | Task runs with less information | Answers degrade; values cannot be restored |
| **Pseudonymise (reversible)** | A realistic stand-in: `hui.zhao@zhilan.cn` | Task runs normally; the answer shows the real values | Depends on detection recall and how securely the mapping is kept |

None of these is always right. Blocking suits data that must never reach an external AI. Redaction suits logs and analytics. Reversible pseudonymisation suits everyday work where the answer has to mention the real customer, colleague or account.

## What vendors say they do

| Tool | Type | Actions stated on the vendor's own pages |
|---|---|---|
| [Nightfall](/shadow/compare/nightfall/) | Browser and endpoint DLP, MCP Gateway, SaaS DLP | Coach, redact or block |
| [Harmonic Security](/shadow/compare/harmonic-security/) | Browser extension, endpoint, MCP Gateway | Silent logging, warnings, coaching, blocking |
| [LayerX (Akamai Workforce Protector)](/shadow/compare/layerx/) | Browser extension | Warn, redact, block or guide |
| [Prompt Security](/shadow/compare/prompt-security/) | AI security platform (SentinelOne) | Redact, coach, block unsanctioned tools and adversarial prompts |
| [Microsoft Purview](/shadow/compare/microsoft-purview/) | Endpoint DLP on onboarded Windows devices | Warn or block sharing with third-party generative AI sites in a browser |
| [Netskope One](/shadow/compare/netskope/) | Inline (SASE) | Redact by mask, format-preserving encryption or hash; file content, SSE streaming responses and MCP traffic not supported for redaction |
| **Shadow** | Local LLM proxy | Mask with reversible fakes; block file uploads by policy |

More vendor comparisons: [Zscaler](/shadow/compare/zscaler/), [Palo Alto AI Access](/shadow/compare/palo-alto-ai-access/), [Lasso Security](/shadow/compare/lasso-security/), [Strac](/shadow/compare/strac/), and the [full overview](/shadow/compare/).

## When to choose endpoint DLP or a browser extension

- **Employees in AI web apps are the main risk.** Extensions and endpoint agents see what people type, paste and upload in the browser, often across many AI sites.
- **You want one policy for more than AI.** Endpoint DLP and several extensions also cover SaaS apps, file movement, clipboard, USB and printing.
- **You need coaching.** A prompt that explains why a paste is risky changes behaviour over time.
- **You already own the platform.** If you run Microsoft 365 with Purview, an SSE or SASE product, or SentinelOne, adding AI controls may cost little extra.

## When to choose Shadow

- **AI traffic comes from outside the browser.** Developers, coding agents (Claude Code, Codex, Gemini CLI, GitHub Copilot), scripts and services call AI services directly. Core sits in front of them and recognises 288 AI services.
- **Answers must keep working.** Shadow sends realistic, consistent fakes instead of placeholders and restores the real values in the answer. See [how it works](/shadow/concepts/how-it-works/).
- **You want to start free and bottom-up.** Core is open source under Apache-2.0, needs no account and runs on the machine. See the [quick start](/shadow/getting-started/quick-start/).
- **You want a team view without collecting real values.** With [Shadow Hub](/shadow/hub/) (14-day free trial, free for up to 3 users), admins invite members by link and see what was masked or blocked per person, machine, app and AI service. Each organisation chooses whether machines share counts only, sanitised prompts or full detail.

Know Shadow's limits. It does not cover phone apps, apps that pin certificates, WebSocket features, or apps that ignore the system proxy unless pointed at it. Cursor, Windsurf, Amazon Q and Kiro use binary protocols Shadow cannot mask; it shows them as "seen, not protected". Shadow is not a browser extension, has no SaaS app connectors, no DLP for files on disk and no SOC 2 or other certification.

## Using both

Many organisations run an extension or endpoint DLP for web chat and SaaS, and Shadow on developer machines and servers. If your SSE or SASE product inspects TLS, exclude the AI hosts Shadow handles or chain Shadow's outbound traffic through your corporate proxy with `upstream_proxy`, so the two do not compete for the same connection. See [proxy mode](/shadow/integrate/proxy-mode/).

## Sources

Vendor facts on this page come from each vendor's own public pages:

- [Nightfall AI security](https://www.nightfall.ai/products/ai-security) (fetched 2 Nov 2026): coach, redact or block; MCP Gateway.
- [Nightfall browser DLP and endpoint DLP](https://www.nightfall.ai/integrations/browser-dlp-and-endpoint-dlp) (fetched 2 Nov 2026): browser extensions; uploads, clipboard, USB, printing.
- [Nightfall pricing](https://www.nightfall.ai/pricing) (fetched 2 Nov 2026): per user per year, amounts not shown.
- [Harmonic Guide](https://www.harmonic.security/products/guide) (fetched 2 Nov 2026): browser extension, MCP Gateway, logging, warnings, coaching, blocking.
- [Harmonic pricing](https://www.harmonic.security/pricing) (fetched 2 Nov 2026): plans without prices.
- [Akamai Workforce Protector](https://www.akamai.com/products/akamai-workforce-protector) (fetched 2 Nov 2026): LayerX is now Akamai Workforce Protector; browser extension; warn, redact, block or guide; contact sales.
- [SentinelOne: Prompt Security AI Usage Control](https://www.sentinelone.com/platform/prompt-security-ai-usage-control/) (fetched 2 Nov 2026): redaction, coaching, blocking unsanctioned tools.
- [SentinelOne: Securing AI with Prompt Security](https://www.sentinelone.com/platform/securing-ai-prompt/) (fetched 2 Nov 2026): AI firewall blocking adversarial prompts.
- [Microsoft Purview protections for generative AI apps](https://learn.microsoft.com/purview/ai-microsoft-purview) (fetched 2 Nov 2026): Endpoint DLP on onboarded Windows devices can warn or block sharing with third-party generative AI sites in a browser.
- [Netskope: redact sensitive data](https://docs.netskope.com/en/redact-sensitive-data/) (fetched 2 Nov 2026): mask, FPE or hash; file content, SSE streaming responses and MCP traffic not supported.

---

*Last reviewed: 2 Nov 2026. All product names are trademarks of their owners; this guide is based on public vendor pages as of the review date.*
