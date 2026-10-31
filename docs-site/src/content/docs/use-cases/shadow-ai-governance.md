---
title: "Shadow AI governance: how to manage unsanctioned AI tools without banning AI"
description: "Shadow AI is employees using unapproved AI tools. Learn how to govern it, and how the open-source Shadow privacy layer makes a sanctioned path safe to use."
keywords:
  - shadow ai
  - shadow ai governance
  - unsanctioned ai tools employees
  - shadow ai risk
  - ai acceptable use policy
  - manage employee use of chatgpt
faq:
  - q: "What is shadow AI?"
    a: "Shadow AI is the use of AI tools that IT and security have not approved, such as personal ChatGPT accounts, browser extensions or AI features in SaaS apps. The main risk is company and customer data reaching providers outside any contract or review."
  - q: "Is the Shadow privacy layer a shadow AI discovery tool?"
    a: "No. Shadow, the open-source product these docs describe, does not discover or inventory AI tools. It masks sensitive data in AI API traffic from the tools you sanction. For discovery, use network logs, a CASB or SASE product, or your identity provider's app catalogue."
  - q: "Should we just block ChatGPT and other AI tools?"
    a: "Outright bans tend to push usage onto personal devices and accounts, where you have no visibility at all. Most guidance favours offering an approved tool that is safe enough to use for real work, plus a clear policy and monitoring."
  - q: "How does Shadow reduce shadow AI?"
    a: "It removes the main reason people go around policy: the approved tool was either blocked or too restricted to be useful. With Shadow, teams can use API-based assistants and agents on real work while the names, customer data and secrets Shadow detects leave as reversible fakes (or redacted, for secrets, if you opt in)."
  - q: "Can Shadow show which employees sent sensitive data to AI?"
    a: "No. Shadow logs per-request intent and counts by treatment, never values, and has no central console. An optional local audit log of decisions is on the roadmap. It is a protection layer, not a monitoring tool."
sidebar:
  order: 13
---

Shadow AI is the use of AI tools that your organisation has not approved: personal ChatGPT accounts, unvetted browser extensions, AI features switched on inside SaaS apps. You govern it by finding out what is used, offering an approved alternative that is genuinely useful, and making that alternative safe for real data. Shadow, the open-source privacy layer documented here, helps with the third part: it masks sensitive data in AI API traffic from sanctioned tools, so saying yes to AI does not mean sending detected customer data to a provider in clear.

:::note[Two different things called "shadow"]
**Shadow AI** (lower case, the phenomenon) is unsanctioned AI use. **Shadow** (the product) is a self-hosted, Apache-2.0 privacy layer that replaces sensitive values in AI requests with reversible fakes. The product does not detect or block shadow AI. It makes an approved path safe enough that people stop needing an unapproved one.
:::

## Why does shadow AI happen?

People use AI because it saves them hours. They use *unapproved* AI when the approved route is missing, slow to get, or so locked down that it is useless for their actual work. A DLP rule that blocks any prompt containing a customer name pushes that work to a personal phone.

The data at risk is ordinary and constant: customer emails pasted for a reply, a spreadsheet uploaded for analysis, a stack trace with an internal hostname and an API key, a contract for summarising.

## A practical governance model

| Step | What it involves | Where Shadow fits |
|---|---|---|
| 1. Discover | Find which AI services are used: proxy and DNS logs, CASB or SASE reports, SSO app catalogues, expense reports | Not a Shadow feature |
| 2. Decide | Pick approved providers and clients; sign agreements; write an acceptable-use policy | Shadow widens what is safe to approve |
| 3. Provide | Give people approved tools that work on real tasks | Shadow masks data in those tools' API traffic |
| 4. Protect | Technical controls on what leaves | Reversible masking, secret redaction, file masking |
| 5. Monitor and review | Track usage and decisions; revisit the policy | Per-request counts in logs today; audit log on the [roadmap](/shadow/project/roadmap/) |

## How to give teams a safe, sanctioned AI path with Shadow

### 1. Choose API-based tools for the approved list

Shadow intercepts provider APIs (OpenAI, Anthropic, Azure OpenAI, Gemini and Vertex AI, Bedrock runtime, OpenRouter, Mistral and others; see [providers](/shadow/reference/providers/)). Good candidates for the approved list:

- coding assistants that call the provider with your key: Claude Code, Codex CLI, Aider, Continue, Cline;
- desktop chat clients that use your own API key;
- internal assistants and agents your teams build on the OpenAI or Anthropic SDKs.

Consumer web apps such as chatgpt.com and claude.ai, and vendor backends such as GitHub Copilot or Cursor's hosted service, are not built in. Govern those with your existing web controls.

### 2. Install Shadow on endpoints

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme GmbH"
shadow install --apply    # background service, CA trust, proxy env file
```

Run `shadow install` without `--apply` first to see every change it will make.

### 3. Set an organisation-wide `shadow.toml`

```toml
organization = ["Acme GmbH"]
terms = ["Project Falcon", "org:Globex Ltd", "org:Initech"]
internal_domains = ["acme.example", "acme.internal"]

[policy.always]
secret = "redact"

[files]
images = "block"
```

Distribute it with MDM or configuration management. There is no central console yet, so treat the file like any other managed configuration.

### 4. Cover internal apps with the gateway

```sh
export OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1
export ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic
```

For services in Kubernetes, use the [sidecar pattern](/shadow/integrate/docker-kubernetes/).

### 5. Write it into the policy

Make the acceptable-use policy concrete: which tools are approved, that they must run with Shadow, what still must not be pasted (for example scanned documents, since there is no OCR), and how to request a new tool. Tell people what Shadow does; it works better when they know the approved tool is safe to use with real data.

### 6. Check it is working

```sh
shadow status
shadow scan sample-prompt.txt
```

The service log shows each request's intent and treatment counts, and the local [activity dashboard](/shadow/concepts/activity-dashboard/) (`shadow dashboard`) shows them by data type, provider and app.

## What Shadow does not cover

- **Discovery.** It cannot tell you which AI tools are in use.
- **Personal devices and accounts.** It protects the machines it is installed on.
- **Web chat UIs and pinned apps.** Browser chat and certificate-pinning apps are outside its scope.
- **Phones and Chromebooks.** Shadow runs on macOS, Linux, Windows, servers and containers.
- **User-level monitoring.** No per-user reporting; logs never contain values.

## Regulations and frameworks

ISO/IEC 42001 (AI management systems) expects organisations to know how AI is used and to manage the risk, and the EU AI Act adds AI literacy and deployer expectations on top (check the current text: the 2026 Digital Omnibus amended several provisions and timelines). GDPR Articles 25 and 32 expect data protection by design and appropriate security. Shadow supports these as one technical control on sanctioned AI data flows; it is not a governance programme by itself.

:::caution[Not legal advice]
This page describes technical capabilities. It does not make your AI use compliant with any law or standard. Involve your security, privacy and legal teams.
:::

## Related

- [LLM data loss prevention](/shadow/use-cases/llm-data-loss-prevention-dlp/) and [stopping PII leaks to ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/)
- Rollout by size: [small business](/shadow/company-size/small-business/), [mid-market](/shadow/company-size/mid-market/), [enterprise](/shadow/company-size/enterprise/)
- [Proxy mode](/shadow/integrate/proxy-mode/), [security model](/shadow/reference/security/), [how Shadow compares](/shadow/compare/)
