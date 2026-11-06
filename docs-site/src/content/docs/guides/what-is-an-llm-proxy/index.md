---
title: "What is an LLM proxy? How it works and when you need one"
seoTitle: "What is an LLM proxy?"
description: "An LLM proxy sits between your apps and AI providers to route, log, limit or protect requests. Learn the types (gateway, forward proxy, privacy proxy), how they differ, and how to pick one."
editUrl: false
keywords:
  - llm proxy
  - llm gateway
  - ai gateway
  - llm privacy proxy
  - ai api proxy
faq:
  - q: "What is the difference between an LLM proxy and an LLM gateway?"
    a: 'The terms overlap. "Gateway" usually means a central server that apps call by base URL, focused on routing, keys and cost. "Proxy" is broader and also covers forward proxies that intercept traffic on the device.'
  - q: "Does an LLM proxy add latency?"
    a: "Some. A local proxy adds a network hop on loopback plus inspection time; a remote gateway adds a round trip to its server. Measure it with your own prompts."
  - q: "Can an LLM proxy work with streaming responses?"
    a: "Good ones do. Shadow restores values inside streamed responses and tool calls."
  - q: "Is LiteLLM an LLM proxy?"
    a: "Yes, LiteLLM offers a proxy server that gives many providers one OpenAI-style endpoint. A privacy proxy like Shadow can run alongside it."
---

**An LLM proxy is a service that sits between an application and one or more large language model APIs (OpenAI, Anthropic, Google and others).** Every request passes through it, so it can route traffic between providers, enforce keys and rate limits, log usage and cost, cache responses, or inspect and change the content, for example masking personal data before it reaches the model. Apps usually use it by changing one setting: the API base URL or the HTTPS proxy.

## Why teams put a proxy in front of LLMs

- **One endpoint for many providers:** switch models without changing application code.
- **Keys and access control:** apps get a proxy key; the real provider keys stay in one place.
- **Cost and usage tracking:** per team, per app, per model.
- **Reliability:** retries, fallbacks and load balancing across providers.
- **Caching:** reuse answers to identical requests.
- **Privacy and security:** detect personal data, secrets and confidential terms, then block, redact or mask them before the request leaves.
- **Audit:** a record of which tool sent what kind of data where.

## The three kinds of LLM proxy

| Type | Where it runs | How apps connect | Best for |
|---|---|---|---|
| **LLM gateway** (reverse proxy) | A server or managed service | Change the API base URL | Routing, keys, cost, fallbacks across a whole platform |
| **Forward proxy** | On the device or network edge | `HTTPS_PROXY` setting | Covering tools you cannot reconfigure, such as CLIs and desktop apps |
| **Privacy proxy** | Usually on the device | Base URL or `HTTPS_PROXY` | Keeping sensitive data from ever reaching the provider |

Many products combine these. A gateway can add guardrails; a privacy proxy can also offer a gateway route.

## How an LLM proxy works, step by step

Using a privacy proxy as the example ([Shadow's request lifecycle](/shadow/concepts/how-it-works/)):

1. **Intercept:** the app sends its request to the proxy, either to a local gateway route such as `/gw/openai/v1/chat/completions` or through `HTTPS_PROXY`.
2. **Inspect:** the proxy parses the JSON body and scans each text field with detectors for emails, phone numbers, card numbers, IBANs, secrets, names, addresses and custom terms.
3. **Transform:** sensitive values are replaced, for example with format-preserving fakes, so the model still gets a coherent prompt.
4. **Forward:** the masked request goes to the real provider over TLS.
5. **Restore:** the response, including streamed tokens and tool calls, is translated back so the user and the app see the real values.

## LLM proxy vs API gateway vs AI firewall

- An **API gateway** handles generic HTTP concerns (auth, rate limits) and does not understand prompts or tokens.
- An **LLM gateway** understands model APIs: providers, models, token cost, streaming.
- An **AI firewall** or guardrail focuses on blocking: prompt injection, unsafe output, data leakage.
- A **masking proxy** focuses on letting the request through safely: it changes the sensitive parts and restores them in the answer, instead of only blocking.

## Self-hosted or managed?

A managed gateway is quick to set up, but every prompt passes through one more third party. A self-hosted or on-device proxy keeps plaintext on your infrastructure. For privacy, the closer to the user the masking happens, the fewer systems ever see the real data.

## How to choose an LLM proxy

1. Which problem comes first: routing and cost, or data protection?
2. Which tools must it cover? SDK-based apps can change a base URL; CLIs, IDEs and chat apps often need a forward proxy.
3. Does it handle streaming and tool calls, or only simple completions?
4. Where does plaintext live, and who can read the logs?
5. Is it open source, so your security team can read what it does?
6. Can it sit next to an existing gateway (for example LiteLLM) instead of replacing it?

## Shadow: an open-source privacy proxy for LLMs

[Shadow](/) runs locally on `127.0.0.1:8877`, written in Rust and licensed Apache-2.0. It works as a gateway (change the base URL) or as a forward proxy (set `HTTPS_PROXY`), so it covers SDKs, Claude Code, Cursor and the ChatGPT app. It masks personal data and secrets with reversible fakes and restores them in responses, streams and tool calls included. For hosts that are not AI providers it only tunnels bytes and never sees the content ([proxy mode](/shadow/integrate/proxy-mode/)).

On a Mac or Linux laptop:

```sh
curl -fsSL https://hub.getshadow.dev/core/install.sh | sh
```

Running it across a team? [Shadow Hub](/shadow/hub/) shows what was masked or blocked per user and tool. Hub is free for up to 3 users, then $3 per user per month up to 500 users. See [Pricing](/shadow/pricing/).

## Related

- [Private ChatGPT: how to use AI without sharing your data](/shadow/guides/private-chatgpt/)
- [Does Cursor train on my code?](/shadow/guides/does-cursor-train-on-my-code/)
- [How Shadow works](/shadow/concepts/how-it-works/)
- [Proxy mode](/shadow/integrate/proxy-mode/)
- [Gateway mode](/shadow/integrate/gateway-mode/)
- [LLM gateways with PII masking compared](/shadow/compare/llm-gateways-with-pii-masking/)
- [Shadow vs LiteLLM](/shadow/compare/litellm/)
