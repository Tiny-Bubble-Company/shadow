---
title: "Shadow Hub vs LayerX (Akamai Workforce Protector)"
seoTitle: "LayerX Security vs Shadow Hub: AI data protection"
description: "LayerX Security (now Akamai Workforce Protector) vs Shadow: a browser extension that warns, redacts or blocks vs a local proxy with reversible fakes."
sidebar:
  label: LayerX
  order: 24
keywords:
  - layerx security
  - layerx
  - layerx alternative
  - layerx vs shadow
  - akamai workforce protector
  - browser extension AI DLP
  - AI DLP
  - GenAI DLP
faq:
  - q: "What happened to LayerX Security?"
    a: "Akamai acquired LayerX. Akamai's product page says LayerX is now Akamai Workforce Protector and is part of Akamai's security portfolio, and layerxsecurity.com redirects there."
  - q: "Is Shadow an alternative to LayerX?"
    a: "For AI prompts, partly. LayerX is a browser extension that secures AI, SaaS, web and private apps in the browser. Shadow is a local proxy: Core masks PII and secrets in requests to AI services with reversible fakes, including from coding agents and scripts outside the browser. Shadow Hub shows teams what was masked or blocked."
  - q: "Does LayerX redact data before it reaches AI apps?"
    a: "Akamai's page says Workforce Protector applies adaptive controls that warn, redact, block or guide user actions, and protects sensitive data before it reaches AI applications. It does not describe restoring the original values in answers; Shadow does."
  - q: "How much does LayerX cost?"
    a: "Not published (as of 2 Nov 2026). Akamai's product page offers contact sales. Shadow Core is free. Shadow Hub's planned pricing is free for 1-3 users, then $3 per user per month; these prices are provisional."
  - q: "Is Shadow Hub available now?"
    a: "Yes. Shadow Hub has a 14-day free trial with no card, then stays free for up to 3 users. Shadow Core is free and open source."
  - q: "Can LayerX and Shadow run together?"
    a: "Yes. LayerX works inside the browser and Shadow works as a local proxy, so LayerX can govern browser and SaaS activity while Shadow masks prompts from developer tools, coding agents and scripts."
---

**Choose LayerX, now Akamai Workforce Protector, if you want a browser extension that governs AI, SaaS and web apps in the browsers you already run, with warn, redact, block and guide. Choose Shadow if you want AI prompts masked with reversible fakes, including from coding agents and scripts outside the browser. Core is free; Shadow Hub is free for up to 3 users.**

[Shadow Hub for teams](/shadow/hub/) · [Install Core free](/shadow/getting-started/quick-start/)

## At a glance

| | LayerX (Akamai Workforce Protector) | Shadow Core | Shadow Hub |
|---|---|---|---|
| **Deployment model** | Commercial browser extension with a management console and cloud intelligence; no proxies or network changes | Free local proxy, a single binary on each laptop or server | Hosted console with a 14-day free trial; members run one command that installs Core and enrols the machine |
| **Where it runs** | In Chrome, Edge, Firefox and Safari on Windows, macOS and Linux, with iOS and Android deployment; covers AI apps, agentic browsers and PWA desktop apps | On the device, between apps and AI services: model APIs, coding agents, and with the system proxy on, ChatGPT, Claude.ai, Gemini, Perplexity, DeepSeek and Grok in a browser | Masking stays on each machine; the hub receives what the organisation chooses to share: counts only, sanitised prompts or full detail |
| **Masking vs blocking** | Warn, redact, block or guide; DLP for text input, copy and paste, uploads and downloads | Masks PII and secrets with realistic fakes; blocks file uploads by policy | Same as Core, plus a view of what was masked or blocked per person, machine, app and AI service |
| **Reversible pseudonymisation** | Not described on its public pages | Yes, real values restored in answers | Yes, through Core |
| **Open source** | No | Yes, Apache-2.0 | No, commercial hosted service |
| **Published pricing** | No, not published (as of 2 Nov 2026) | Free | Provisional: free for 1-3 users, then $3 per user per month |
| **Best for** | Organisations that want browser-level control of AI, SaaS and web apps, including on BYOD and contractor devices | Developers and engineers using model APIs and coding agents | Teams that want to see what Core masked or blocked across members, without collecting real values unless they choose to |

## When to choose LayerX

- **Most AI use happens in the browser.** LayerX installs as an extension on the browsers your organisation already runs, with no browser swap and no network changes.
- **You need more than AI.** It secures SaaS, web and private apps as well, with DLP on text input, copy and paste, uploads and downloads.
- **You cover agentic browsers and phones.** Akamai lists agentic browsers such as ChatGPT Atlas, Perplexity Comet and Dia, PWA desktop apps, and iOS and Android deployment.
- **You have BYOD or contractors.** Akamai describes read-only sessions and watermarking for BYOD and third-party contractors.
- **You buy through Akamai or a cloud marketplace.** It is part of Akamai's security portfolio and listed on the AWS and Google Cloud marketplaces.

## When to choose Shadow

- **Answers must keep working.** Shadow replaces names, emails, card numbers and secrets with realistic, consistent fakes, so the model can still reason about them, and restores the real values in the answer. See [how it works](/shadow/concepts/how-it-works/).
- **AI traffic leaves from outside the browser.** Core sits in front of model APIs, inference clouds and coding agents such as Claude Code, Codex, Gemini CLI and GitHub Copilot, and recognises 288 AI services. With the system proxy on, it also covers ChatGPT, Claude.ai, Gemini, Perplexity, DeepSeek and Grok in a browser.
- **You want to start free and bottom-up.** Core is open source under Apache-2.0, needs no account and runs on the machine. See the [quick start](/shadow/getting-started/quick-start/).
- **You want a team view without collecting real values.** With [Shadow Hub](/shadow/hub/) (14-day free trial, free for up to 3 users), admins invite members by link and see what was masked or blocked per person, machine, app and AI service. Each organisation chooses whether machines share counts only, sanitised prompts or full detail.

Know Shadow's limits before choosing it. It does not cover phone apps, apps that pin certificates, WebSocket features, or apps that ignore the system proxy unless pointed at it. Cursor, Windsurf, Amazon Q and Kiro use binary protocols Shadow cannot mask; it shows them as "seen, not protected". Shadow is not a browser extension, has no SaaS app connectors, no DLP for files on disk and no SOC 2 or other certification.

## Sources

LayerX facts on this page come from LayerX's and Akamai's own public pages:

- [layerxsecurity.com](https://layerxsecurity.com/) (fetched 2 Nov 2026): redirects (301) to Akamai Workforce Protector.
- [Akamai Workforce Protector](https://www.akamai.com/products/akamai-workforce-protector) (fetched 2 Nov 2026): "LayerX is now Akamai Workforce Protector"; browser extension, management console and cloud intelligence; warn, redact, block or guide; supported browsers, platforms and AI apps; AWS and Google Cloud marketplace listings; contact sales, no prices.
- [Akamai Workforce Protector solution brief](https://www.akamai.com/resources/solution-brief/akamai-workforce-protector-solution-brief) (fetched 2 Nov 2026): "formerly LayerX"; extension-based; BYOD and contractor support with read-only sessions and watermarking.

Shadow facts: [how it works](/shadow/concepts/how-it-works/), [supported providers](/shadow/reference/providers/), [proxy mode](/shadow/integrate/proxy-mode/).

---

*Last reviewed: 2 Nov 2026. LayerX and Akamai are trademarks of their owners; this comparison is based on their public pages as of the review date. See also [AI DLP compared](/shadow/compare/ai-dlp/).*
