---
title: "Shadow vs DataFog — open-source DataFog alternative for LLM privacy"
description: Shadow vs DataFog for LLM PII protection. DataFog redacts, replaces or hashes PII; Shadow uses reversible fakes and restores streamed responses.
sidebar:
  label: DataFog
  order: 13
keywords:
  - datafog alternative
  - datafog vs shadow
  - datafog litellm guardrail
  - datafog claude code hook
  - open source PII redaction
  - PII redaction for LLM
  - python PII library
faq:
  - q: "Is Shadow an alternative to DataFog?"
    a: "For protecting LLM traffic, yes. DataFog is an MIT-licensed Python library that detects PII with regex, spaCy or GLiNER and redacts, replaces or hashes it, with a LiteLLM guardrail and a Claude Code hook. Shadow is a Rust privacy layer that replaces values with reversible fakes and restores them in responses."
  - q: "Can DataFog restore redacted values in the response?"
    a: "Public DataFog documentation does not describe restoring values in model responses. Its modes are redact, replace and hash. Shadow restores values automatically, including in streams and tool calls."
  - q: "Does DataFog work with Claude Code?"
    a: "Yes, DataFog ships a Claude Code hook. Shadow covers Claude Code through its HTTPS proxy or gateway, so the whole request, including file contents the agent reads, is masked and the response restored."
  - q: "Is DataFog free?"
    a: "Yes. DataFog is open source under the MIT licence. Shadow is open source under Apache-2.0. Both are free to self-host."
  - q: "Which is faster, DataFog or Shadow?"
    a: "DataFog publishes its own latency figures for its regex mode and hook. Shadow's built-in rules take about 7.5 ms per document on the held-out benchmark. DataFog itself is not in that benchmark."
---

**DataFog is an open-source (MIT) Python library for detecting and redacting PII, with integrations for LiteLLM and Claude Code. Shadow is an open-source (Apache-2.0) privacy layer for AI traffic that replaces sensitive values with realistic, reversible fakes and restores them in the model's response.** DataFog is a light, flexible choice when one-way redaction is enough. Shadow is built for the full round trip, where the answer has to come back with the real names, emails and IDs.

:::note[TL;DR]
DataFog redacts, replaces or hashes PII before it reaches a model, with a choice of regex, spaCy or GLiNER engines. Shadow translates values into consistent fakes and restores them in responses, streams and tool calls, as a proxy, gateway or Rust library.
:::

## At a glance

| | Shadow | DataFog |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Yes, MIT |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Python library; LiteLLM guardrail; Claude Code hook |
| **Protection method** | Reversible realistic fakes; typed tokens for reports; optional redaction | Redact (`[EMAIL_1]`-style), replace, hash, block |
| **Restores responses** | Yes, automatically | Not documented |
| **Streaming** | Yes, SSE and NDJSON | Not documented |
| **Tool calls** | Yes, restored locally; intent-aware | Not documented |
| **Files** | DOCX, XLSX, PPTX in place; PDF to text | Not documented for LLM uploads |
| **Self-hosted** | Yes | Yes |
| **Price model** | Free | Free |

## What DataFog does well

- **Engine choice.** Regex for speed, spaCy or GLiNER for named entities, and a "smart" cascade that combines them. You pick the trade-off between latency and recall.
- **Fast regex path.** DataFog publishes low vendor latency figures for its regex mode and its Claude Code hook.
- **Integrations where developers are.** A LiteLLM guardrail and a Claude Code hook make it easy to add PII checks to an existing gateway or coding agent.
- **Simple Python API.** Easy to drop into scripts, notebooks and data pipelines.
- **Permissive licence.** MIT, active releases (v4.9.0 at the time of review).

## Where Shadow is different

### Does DataFog give you the real values back?

Its documented modes are redact, replace and hash. A prompt about `jane@acme.com` reaches the model as `[EMAIL_1]` or a replacement, and the answer refers to that. Shadow restores the real value automatically using an [encrypted local vault](/shadow/concepts/vault/), in JSON responses, [server-sent event streams](/shadow/concepts/how-it-works/#streaming-hold-back) and [tool-call arguments](/shadow/concepts/tool-calls/).

### Realistic, consistent fakes

Shadow replaces values with format-preserving fakes from a persistent [persona](/shadow/concepts/personas/): a plausible name, a valid email at a consistent fake domain, a Luhn-valid card, a mod-97-valid IBAN. The model reads natural text, and the same customer gets the same fake across sessions.

### Covering many tools on a machine

DataFog integrates through LiteLLM or a Claude Code hook. Shadow intercepts at the network layer, so any tool that honours `HTTPS_PROXY` or accepts a base URL is covered when it calls a [supported AI API host](/shadow/reference/providers/), including desktop apps and SDKs you didn't configure individually. Web chat in a browser (chatgpt.com, claude.ai) is not covered. See [proxy mode](/shadow/integrate/proxy-mode/) and [gateway mode](/shadow/integrate/gateway-mode/).

### Intent policy and files

Shadow's [intent policy](/shadow/concepts/intent-policy/) chooses fakes for conversation, typed tokens such as `[PERSON_0007]` for bulk reports, and keep for targets of provider-hosted actions. [File masking](/shadow/concepts/file-masking/) covers DOCX, XLSX and PPTX uploads in place.

### No Python runtime

Shadow is a single Rust binary for macOS, Linux and Windows. That matters when you roll it out to many developer machines or CI runners.

## When should you choose DataFog?

- One-way redaction is enough, for example for logs, analytics or a compliance pre-check.
- You want a Python library to call from your own code.
- You already run LiteLLM and want a lightweight guardrail there.
- You want to experiment with regex, spaCy and GLiNER engines side by side.

## When should you choose Shadow?

- The model's answer must contain the real values.
- You use streaming, agents and tool calls.
- You want one control that covers every tool on a machine that calls a supported AI API.
- You want Office uploads masked before they leave.

## Can you use them together?

There is no built-in integration. You can run both on the same traffic, for example DataFog in a LiteLLM gateway and Shadow on developer machines, but masking twice is rarely useful. If you mainly like DataFog's GLiNER engine, Shadow's [NER sidecar](/shadow/concepts/detection/#local-ner-sidecar) runs GLiNER and OpenAI Privacy Filter as detectors.

## Migrating from DataFog

1. Route model calls through Shadow's [gateway](/shadow/integrate/gateway-mode/) or [proxy](/shadow/integrate/proxy-mode/), and remove the DataFog guardrail or hook for that traffic.
2. Port any custom regex patterns to [dictionary terms](/shadow/concepts/detection/#dictionary-terms) or a Rust [custom detector](/shadow/integrate/rust/#custom-detectors).
3. Enable the NER sidecar if you relied on DataFog's spaCy or GLiNER engines.
4. If you used hashing for values that must never leave, set those kinds to `redact` under `[policy.always]`.
5. Compare coverage with `shadow scan` on representative prompts.

## How do they compare on accuracy and speed?

DataFog's published figures are latency claims from the vendor, not accuracy results on public datasets. DataFog is not in Shadow's [benchmark](/shadow/benchmarks/), which measures leak rate, over-masking, round-trip exactness and latency for Shadow and other open-source tools, with code published. Its GLiNER engine is represented there by the GLiNER baseline.

## Sources

- [DataFog on GitHub](https://github.com/DataFog/datafog-python)
- [DataFog on PyPI](https://pypi.org/project/datafog/)
- [Shadow detection](/shadow/concepts/detection/)

---

*Last reviewed: 2026-10-31. DataFog is a trademark of its owner; this comparison is based on public documentation as of the review date.*
