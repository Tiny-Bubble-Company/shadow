---
title: Introduction
description: "What Shadow is: an open-source privacy layer that masks PII and secrets in prompts to ChatGPT, Claude and other LLMs with reversible fakes, unlike blocking DLP."
---

Shadow is an endpoint privacy layer for AI traffic. It sits between your apps and AI providers such as OpenAI, Anthropic, Google Gemini, Azure OpenAI, OpenRouter, Mistral and Groq. On the way out, it finds sensitive values in each request and replaces them with realistic, format-preserving fakes. On the way back, it turns the fakes into the real values again, streamed responses and tool calls included.

```text
you      ── "Email jane@acme.com about Project Falcon for Acme GmbH" ──▶  Shadow
Shadow   ── "Email hui.zhao@zhilan.cn about Velora Quilent for Zhilan Technology Co., Ltd." ──▶  provider
provider ── "Sure, drafting a note to hui.zhao@zhilan.cn…" ──▶  Shadow
Shadow   ── "Sure, drafting a note to jane@acme.com…" ──▶  you
```

The provider gets a coherent prompt about a company that does not exist. You get an answer about yours.

## The problem

Prompts carry far more than the question:

- names, emails and phone numbers of colleagues and customers;
- customer lists, invoices with IBANs, contracts with addresses;
- internal hostnames, IPs and file paths that contain your username;
- API keys and tokens pasted along with a config file;
- the name of your company and its unreleased projects.

Once a request reaches a provider, that content can end up in logs, abuse-monitoring queues, support tooling, training sets or a breach. Policies tell people not to paste sensitive data, and people paste it anyway, because the tool is useful.

## The approach: translate, don't block

Most data-loss-prevention gateways **block** a request when they see something sensitive. Blocking breaks the tool, so people route around it. Plain redaction replaces values with `[REDACTED]` or `<PERSON>`, which loses meaning: the model can no longer tell two people apart, write an email to the right address, or keep a table consistent.

Shadow **translates**. Every sensitive value maps to a stable fake of the same shape, drawn from a persistent fake identity for your machine called the [persona](/shadow/concepts/personas/):

| Real | What the provider sees | Property kept |
|---|---|---|
| `jane@acme.com` | `hui.zhao@zhilan.cn` | valid email, persona domain |
| `4111 1111 1111 1111` | `4837 2191 9471 3480` | 16 digits, valid Luhn check |
| `DE89 3704 0044 0532 0130 00` | another German IBAN | passes mod-97 |
| `Acme GmbH` | `Zhilan Technology Co., Ltd.` | a company name |
| `/Users/jdoe/src/app` | `/Users/huizhao/src/app` | a path that still works in code |

The same real value always gets the same fake, and no two real values share a fake, so the reverse translation is unambiguous. The mapping lives in an [encrypted vault](/shadow/concepts/vault/) on your machine and never leaves it.

## What Shadow is not

- **Not an anonymiser.** Providers still see your IP address and the account behind your API key. Shadow protects the *content* of requests. See the [security model](/shadow/reference/security/).
- **Not a perfect detector.** Detection combines regular expressions, checksums, a dictionary of your own terms, local machine facts and optional NER or [Presidio](/shadow/concepts/detection/#presidio). It is best effort, and you can test it with `shadow scan`.
- **Not a cloud service.** There is no account, no hosted component and no telemetry.

## Three ways to run it

| Mode | Use it when | Setup |
|---|---|---|
| [Proxy](/shadow/integrate/proxy-mode/) | You want every tool on a machine covered | `HTTPS_PROXY=http://127.0.0.1:8877` and trust a local CA |
| [Gateway](/shadow/integrate/gateway-mode/) | The tool lets you set a base URL | `base_url=http://127.0.0.1:8877/gw/openai/v1`, no CA |
| [Library](/shadow/integrate/rust/) | You build a Rust service that calls models | `shadow-core` crate, masking in-process |

Meridian, a compliance platform, embeds `shadow-core` so every model call it makes is masked in-process.

## Next steps

- [Quick start](/shadow/getting-started/quick-start/): protect your first request in five minutes.
- [How it works](/shadow/concepts/how-it-works/): the request and response lifecycle.
- [Intent-aware policy](/shadow/concepts/intent-policy/): how actions, reports and agents are handled.
