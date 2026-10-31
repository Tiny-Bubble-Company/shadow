---
title: "How to stop employees pasting customer data into ChatGPT"
description: "Stop customer names, emails and IDs leaking into ChatGPT and other LLMs without banning AI: give staff an approved client that masks data through Shadow."
keywords:
  - stop employees pasting data into chatgpt
  - prevent pii leaks to chatgpt
  - chatgpt data leakage prevention
  - customer data in chatgpt
  - mask pii before chatgpt
  - safe chatgpt for employees
faq:
  - q: "Can Shadow mask what employees type into the chatgpt.com website?"
    a: "Not as a built-in, supported integration. Shadow intercepts the provider APIs such as api.openai.com, Azure OpenAI and api.anthropic.com, not consumer web apps such as chatgpt.com or claude.ai. The reliable pattern is an approved API-backed client that runs through Shadow."
  - q: "Why not just block ChatGPT?"
    a: "Blocking rarely holds. People switch to a phone, a personal laptop or another AI site, and you lose all visibility. Shadow keeps an approved tool useful while the customer data in it is replaced with fakes, so there is less reason to route around it."
  - q: "Will the answers be worse if customer data is masked?"
    a: "Usually not noticeably. Shadow uses realistic, format-preserving fakes from one coherent persona, so the model still sees a name, an email and a company, and the real values are restored in the answer."
  - q: "Does Shadow tell me who pasted what?"
    a: "No. Shadow is not a monitoring tool. It logs, per request, the intent and how many values of each kind were masked, never the values themselves. A local audit log of decisions is on the roadmap."
  - q: "Does Shadow cost anything?"
    a: "No. Shadow is open source under Apache-2.0, self-hosted, with no account and no telemetry. You pay only for the model API you already use."
sidebar:
  order: 1
---

You can't reliably stop people pasting customer data into a chatbot by asking them not to, and blocking AI sites mostly pushes the habit somewhere you can't see. The approach that holds up is to give staff an approved, API-backed AI client and route it through Shadow, which replaces customer names, emails, phone numbers, IDs and secrets with realistic fakes before the request leaves the machine and restores them in the answer. Pair that with a short policy and a block or warning on the consumer sites you don't sanction.

## Why do employees paste customer data into ChatGPT?

Because it works. Drafting a reply to an angry customer, summarising a ticket thread or cleaning up a spreadsheet is faster with a model, and the fastest way to give the model context is to paste the whole thing. A typical paste looks like this (all data fictional):

```text
Reply to this complaint politely. Customer: Jane Doe <jane.doe@acme.example>,
account 4111 1111 1111 1111, phone (415) 555-0134, order #A-20931 shipped to
221 Baker Street. She says our agent Tom Reyes promised a refund.
```

That single prompt sends a name, an email, a card number, a phone number, an address and an employee's name to a third party. Multiply by every support agent, account manager and analyst.

## What are the options?

| Approach | What happens | Typical outcome |
|---|---|---|
| Policy only ("don't paste customer data") | Nothing technical changes | People paste anyway, because the tool is useful |
| Block AI sites | Requests fail | Use moves to phones and personal accounts; no visibility |
| Blocking DLP on prompts | Requests with PII are refused | The tool breaks for exactly the tasks people need it for |
| Placeholder redaction (`<PERSON>`) | Request goes through with placeholders | Model can't tell people apart or write to the right address |
| **Translate with Shadow** | Values replaced with consistent, realistic fakes and restored in the answer | The tool keeps working; the provider sees fakes for every detected value except the person a message is addressed to |

Shadow is the last row. It does not replace a policy or your blocking controls for unsanctioned sites; it makes the sanctioned path good enough that people use it. See [LLM data loss prevention](/shadow/use-cases/llm-data-loss-prevention-dlp/) for how this fits with an existing DLP stack.

:::caution[What Shadow covers, and what it does not]
Shadow intercepts the **provider APIs**: `api.openai.com`, Azure OpenAI, `api.anthropic.com`, Gemini, Vertex AI, Bedrock runtime, OpenRouter, Mistral and [others](/shadow/reference/providers/). It does **not** intercept the consumer web apps `chatgpt.com` or `claude.ai`, which use their own undocumented protocols. You can add hosts with `[proxy] extra_hosts`, but that is untested and unsupported for consumer web apps. The dependable setup is an approved client that calls the API.
:::

## How to set it up

### 1. Pick an approved, API-backed client

Choose something that calls a provider API with your organisation's key, not a consumer web app:

- an internal chat UI your team already runs (Open WebUI, LibreChat or a home-grown one) pointed at OpenAI, Azure OpenAI or Anthropic;
- a bring-your-own-key desktop chat client that lets you set the API base URL or honours the system proxy;
- task-specific internal tools, for example a "draft reply" button in your helpdesk.

For a shared internal chat UI, run Shadow next to the server and point the UI at the [gateway](/shadow/integrate/gateway-mode/). For desktop clients, install Shadow on each machine (step 2).

### 2. Install Shadow on each machine

```sh
# macOS / Linux
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh

shadow init --org "Acme Ltd"     # persona, vault key, local CA; protects "Acme Ltd" and "Acme"
shadow install                   # dry run: prints every step
shadow install --apply           # background service, CA trust, env file
```

On Windows use `install.ps1` (see [Installation](/shadow/getting-started/installation/)). Then either set the client's base URL to `http://127.0.0.1:8877/gw/openai/v1`, or let it use the proxy through `HTTPS_PROXY=http://127.0.0.1:8877` or the [PAC file](/shadow/integrate/proxy-mode/#system-wide-with-a-pac-file).

### 3. Teach Shadow your vocabulary

Detectors find structured values (emails, phones, cards, IBANs, national IDs, IPs, hostnames, secrets) and common names. They cannot know your customers, products or codenames:

```toml
organization = ["Acme Ltd"]
terms = ["org:Globex", "org:Initech", "person:Tom Reyes", "Project Falcon"]
internal_domains = ["acme.example", "acme.internal"]

[policy.always]
secret = "redact"        # detected keys and passwords never leave, even as fakes

[files]
images = "block"         # screenshots are not OCR'd, so stop them leaving unmasked
```

Put this in `shadow.toml` in the data directory, or add terms at runtime with `shadow terms add "org:Globex"`.

### 4. Check what gets caught

Save a representative, approved sample prompt to a file and scan it. Nothing is sent:

```sh
shadow scan sample-ticket.txt
shadow scan sample-ticket.txt --reveal   # full values instead of previews
```

Anything missing? Add it as a term, or enable the [open-weight NER sidecar or Presidio](/shadow/concepts/detection/) for better recall on free-text names.

### 5. Roll out, then restrict the unsanctioned path

Tell people which client to use and why. Then use the controls you already have (secure web gateway, DNS filtering, browser policy) to warn on or block the consumer AI sites you don't sanction. Because the approved client is just as useful, the block is far less likely to be routed around.

## What the provider sees

The complaint above, through Shadow on one machine (your persona differs):

```text
Reply to this complaint politely. Customer: Hui Zhao <hui.zhao@zhilan.cn>,
account 4837 2191 9471 3480, phone +86 28 6512 0417, order #A-20931 shipped to
88 Tianfu Road. She says our agent Lei Li promised a refund.
```

The card number still passes Luhn and the email is still an email, so the model writes a normal reply. Wherever it uses the full fakes (Hui Zhao, 88 Tianfu Road, Lei Li), the answer arrives with Jane Doe, her address and Tom Reyes restored. A greeting with the first name alone ("Dear Hui") is restored too; other fragments, such as the last four digits of the fake card, stay fake. Order `#A-20931` is not a detected format; add such references as terms if they matter.

Here the reply is addressed to "this complaint", so nobody in the prompt is the recipient. If the request names the person it writes to ("Reply to Jane Doe at jane.doe@acme.example about her refund"), Shadow passes that recipient through as written and logs it, so the model greets the right person; the card, phone, address and Tom Reyes are still masked. Set `[policy.act] targets = "surrogate"` to fake the recipient too.

## Rolling out to a team

| Team size | Suggested pattern |
|---|---|
| One person or a small team | Endpoint install, BYOK client, shared `shadow.toml` in a repo |
| Dozens to hundreds | Push `shadow.toml` and run `shadow install --apply` through MDM or configuration management |
| Internal chat UI for everyone | One Shadow next to the chat server in [gateway mode](/shadow/integrate/gateway-mode/) or as a [sidecar](/shadow/integrate/docker-kubernetes/) |

There is no central admin console or fleet management yet, and an optional local audit log is a [roadmap](/shadow/project/roadmap/) item. See [Shadow by company size](/shadow/company-size/) for rollout checklists.

## Limits to be honest about

- **Consumer web apps** such as `chatgpt.com` are not covered (see above).
- **Detection is best effort.** Unusual names and internal ID formats can be missed. Test with `shadow scan`.
- **No OCR.** Screenshots and scanned PDFs pass through unless you block them.
- **Dates are not masked**, and **context can identify**: a unique incident described in detail can point to a customer even with every name replaced.

## Related

- [Shadow AI governance](/shadow/use-cases/shadow-ai-governance/): unsanctioned AI tools and what to do about them
- [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/)
- [Redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/)
- [Intent-aware policy](/shadow/concepts/intent-policy/), [Detection](/shadow/concepts/detection/), [Security model](/shadow/reference/security/)
- How Shadow compares with DLP and redaction tools: [compare](/shadow/compare/). Detection results: [benchmarks](/shadow/benchmarks/)
