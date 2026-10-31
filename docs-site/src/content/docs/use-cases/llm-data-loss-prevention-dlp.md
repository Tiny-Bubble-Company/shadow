---
title: "LLM data loss prevention: DLP for ChatGPT, Claude and AI APIs without blocking"
description: "Open-source DLP for LLM traffic: Shadow masks PII, customer data and secrets in AI API requests with reversible fakes instead of blocking them."
keywords:
  - llm dlp
  - ai data loss prevention
  - dlp for chatgpt
  - genai dlp open source
  - prevent data leakage llm
  - ai dlp api traffic
faq:
  - q: "How is Shadow different from a traditional DLP tool?"
    a: "Traditional DLP mostly blocks, warns or redacts, which breaks the request or the answer. Shadow translates: sensitive values are replaced with realistic fakes, the request still works, and the answer is restored. Blocking is used only where translation cannot work, such as images or PDFs if you choose."
  - q: "Does Shadow replace Netskope, Zscaler or Microsoft Purview?"
    a: "No. Those products cover browsers, SaaS apps and the network edge, with central consoles and reporting. Shadow covers AI API traffic from endpoints, servers and containers, and transforms rather than blocks. Many teams will want both."
  - q: "Does Shadow have a central DLP console or incident reporting?"
    a: "Not today. Each instance logs per-request intent and counts by treatment, never values. An optional local audit log is a roadmap item; central fleet management is not available."
  - q: "Can Shadow stop secrets from being sent to AI tools?"
    a: "Yes, with opt-in redaction. By default secrets become fakes of the same shape. Set [policy.always] secret = \"redact\" to send [REDACTED_SECRET] instead, which cannot be reversed."
  - q: "Does Shadow inspect non-AI traffic?"
    a: "No. In proxy mode it decrypts only known AI hosts and tunnels everything else byte for byte, so banking, email and package downloads are never decrypted."
sidebar:
  order: 12
---

Data loss prevention for LLMs means keeping personal data, customer records and secrets out of what you send to AI providers. Shadow does it by translation rather than blocking: it runs on endpoints, servers or as a sidecar, replaces sensitive values in AI API requests with realistic, reversible fakes, and restores them in the response. The tool keeps working, so people have no reason to route around it.

## Why does blocking-style DLP fail for AI?

DLP was built for files and email: find a sensitive pattern, stop the transfer. Applied to AI assistants, that approach has three problems:

- **It breaks the task.** A blocked prompt gets no answer. People retry with the data slightly altered, switch to a personal account, or use their phone.
- **Placeholder redaction degrades answers.** `<PERSON>` and `<EMAIL>` give the model nothing to work with. It cannot tell two people apart or write to the right address, and you have to map placeholders back by hand.
- **Streaming and agents need restoration.** Assistants stream answers and call tools. A DLP layer that only inspects outbound traffic cannot give the agent back the real values it needs to act.

Shadow keeps the request flowing with data that has the same shape as the original:

| Real value | What the provider sees |
|---|---|
| `jane.doe@acme.example` | `xiu.wang@lingxi.cn` |
| `Acme GmbH` | `Lingxi Technology Co., Ltd.` |
| `4111 1111 1111 1111` | `4265 3052 9922 6576` (valid Luhn) |
| `DE89 3704 0044 0532 0130 00` | `DE46 0256 5725 3509 8212 53` (valid mod-97) |
| `build01.acme.internal`, `10.20.4.17` | `velond.draix.internal`, `10.227.176.110` |
| `sk-proj-…` | a fake key of the same shape, or `[REDACTED_SECRET]` if you opt in |

Fakes come from one machine's [persona](/shadow/concepts/personas/); yours will differ.

## Where Shadow fits next to existing DLP

| | Browser, SASE and SaaS DLP (Netskope, Zscaler, Purview and similar) | Shadow |
|---|---|---|
| Covers | Web chat UIs, SaaS apps, email, the network edge | AI API traffic from SDKs, agents, IDE assistants, internal apps |
| Main action | Block, warn, coach; some can redact | Translate with reversible fakes; block only images or PDFs if configured |
| Response handling | Usually none | Restores answers, streams and tool-call arguments |
| Runs | Vendor cloud, browser extension, endpoint agent | Single binary on the endpoint, server or pod |
| Management | Central console, policies, reporting | Local `shadow.toml` per machine; no console yet |
| Cost | Commercial | Free, Apache-2.0 |

A practical split: let your existing DLP or CASB govern consumer web apps such as chatgpt.com, which Shadow does not intercept, and use Shadow for the API-based tools and applications your teams are encouraged to use.

## How to roll out Shadow as LLM DLP

### 1. Install on the machines that call AI APIs

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme GmbH"
shadow install            # dry run: prints every change
shadow install --apply    # background service, CA trust, env file
```

Proxy mode decrypts only [known AI hosts](/shadow/reference/providers/). Add your internal LLM gateway with `extra_hosts`.

### 2. Write the policy

```toml
organization = ["Acme GmbH"]
terms = ["Project Falcon", "org:Globex Ltd"]
internal_domains = ["acme.example", "acme.internal"]

[policy.act]
targets = "surrogate"               # optional: fake message recipients too (default "keep" sends them, logged)
remote_tool_targets = "surrogate"   # never send action targets to provider-hosted tools in clear

[policy.always]
secret = "redact"                   # detected secrets never leave in any form

[files]
pdf = "text"
images = "block"                    # no OCR yet
block_oversize = true               # refuse files Shadow cannot mask
```

### 3. Improve recall where it matters

Built-in detectors cover emails, phones, names, addresses, IBANs, cards, national IDs (US SSN and ITIN, UK NINO and NHS number, Aadhaar, Indian PAN, Australian TFN and ABN), IPs, MACs, hostnames, secrets and home-directory usernames. For free-text names, enable the open-weight [NER sidecar](/shadow/concepts/detection/#local-ner-sidecar) or [Presidio](/shadow/concepts/detection/#presidio).

### 4. Test with real-looking content

```sh
shadow scan sample-ticket.txt
shadow scan sample-ticket.txt --json
```

### 5. Distribute with your own tooling

There is no central admin console or fleet management yet. Use MDM or configuration management to place `shadow.toml` in the data directory, run `shadow install --apply`, and use `shadow status --quiet` as a health check.

### 6. Monitor

With `shadow run` in the foreground or the service logs, each request shows its intent and counts per treatment (surrogates, tokens, kept, redacted). Every `keep` is logged with kind and reason. Values are never logged. On each machine, the local [activity dashboard](/shadow/concepts/activity-dashboard/) (`shadow dashboard`) shows the same counts by data type, provider and app.

## What Shadow does not do

- **No discovery or inventory** of which AI tools people use.
- **No web chat coverage.** chatgpt.com, claude.ai and similar consumer apps are not intercepted; vendor backends such as GitHub Copilot and Cursor's hosted service are not built in either.
- **No OCR**, WebSocket/Realtime or gRPC processing.
- **No central reporting.** A local, append-only audit log of per-request decisions is on the [roadmap](/shadow/project/roadmap/). Blocking-only DLP features are explicitly not planned.
- **Not perfect detection.** Values that are not detected are sent as is. See [benchmarks](/shadow/benchmarks/).

## Related

- [Shadow AI governance](/shadow/use-cases/shadow-ai-governance/) and [stopping PII leaks to ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/)
- [Protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/)
- [Enterprise](/shadow/company-size/enterprise/) and [regulated enterprise](/shadow/company-size/regulated-enterprise/) rollouts
- [Security model](/shadow/reference/security/) and [how Shadow compares](/shadow/compare/)
