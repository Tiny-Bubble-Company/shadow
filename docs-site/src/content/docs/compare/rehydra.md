---
title: "Shadow vs Rehydra — open-source local PII proxies for LLMs compared"
seoTitle: "Shadow vs Rehydra: local PII proxies for LLMs compared"
description: Shadow vs Rehydra, two open-source local proxies that mask PII before LLM calls and restore it. Compare fakes vs placeholders, streaming and files.
sidebar:
  label: Rehydra
  order: 34
keywords:
  - rehydra alternative
  - rehydra vs shadow
  - rehydra pii
  - local pii proxy for llm
  - claude code pii proxy
  - open source PII redaction
  - PII masking for LLM
  - reversible pseudonymization
faq:
  - q: "What is the difference between Shadow and Rehydra?"
    a: "Both are open-source tools that mask PII locally before an LLM call and restore it afterwards, including in streams and tool calls. Rehydra is a TypeScript SDK and proxy (MIT) that uses XML-style placeholders; Shadow is a Rust binary and library (Apache-2.0) that uses realistic, format-preserving fakes from a persistent persona, plus intent policy and Office file masking."
  - q: "Is Rehydra free?"
    a: "Yes. Rehydra is MIT-licensed and published on npm. Shadow is Apache-2.0. Both are free."
  - q: "Which one runs in the browser?"
    a: "Rehydra runs on Node, Bun and in the browser. Shadow runs on macOS, Linux and Windows hosts, servers and containers, not inside a browser page. A browser can send traffic through it as the system proxy, but only calls to supported AI API hosts are masked, not web chat such as chatgpt.com or claude.ai."
  - q: "Do both work with Claude Code?"
    a: "Yes. Rehydra documents a local proxy for Claude Code and OpenAI and Anthropic clients. Shadow covers Claude Code through proxy mode or by setting the Anthropic base URL to its gateway."
  - q: "Which one is more accurate?"
    a: "Rehydra does not publish detection benchmarks at the time of review, and it is not in Shadow's benchmark. Shadow publishes its methodology and results: on the held-out set its built-in rules leaked 3.08% of sensitive values (1.35% with the Presidio detector) and round-tripped 100% of documents exactly."
---

**Rehydra and Shadow are the two open-source projects closest to each other in this space: both mask PII locally before a request reaches an LLM provider, and both restore the original values in responses, streams and tool calls.** Rehydra is a TypeScript SDK and local proxy under MIT that replaces values with XML-style placeholders. Shadow is a Rust binary and library under Apache-2.0 that replaces values with realistic, format-preserving fakes from a persistent persona, adds an intent policy and masks Office files. Pick by language ecosystem and by whether you want placeholders or realistic fakes.

:::note[TL;DR]
Same model, different choices. Rehydra: TypeScript, runs in Node, Bun and browsers, regex plus ONNX NER, placeholders like `<PII type id>`, AES-256-GCM session map, sentence-buffered streaming. Shadow: Rust, runs as a proxy, gateway or crate, realistic checksum-valid fakes, persistent encrypted vault, character-level stream hold-back, intent policy, DOCX/XLSX/PPTX masking.
:::

## At a glance

| | Shadow | Rehydra |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Yes, MIT |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | TypeScript SDK (npm) and local proxy; Node, Bun, browser |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Reversible XML-style placeholders |
| **Restores responses** | Yes, automatically | Yes |
| **Streaming** | Yes, SSE and NDJSON, holds back at most the longest fake minus one byte | Yes, sentence-buffered with NER overlap |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Yes (`onToolCall` argument rehydration) |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | `.env` scrubbing |
| **Self-hosted** | Yes, single binary, no account | Yes |
| **Price model** | Free | Free |

## What Rehydra does well

- **TypeScript-native.** If your application is in Node or Bun, Rehydra is an npm package you import. It also runs in the browser, which Shadow does not.
- **Local proxy for coding agents.** Rehydra documents a proxy for Claude Code and OpenAI and Anthropic clients.
- **Regex plus ONNX NER.** Detection combines patterns with an ONNX NER model in-process, without a separate Python service.
- **Encrypted session map.** The placeholder mapping is protected with AES-256-GCM.
- **Tool calls handled.** Arguments are rehydrated through an `onToolCall` hook, so agents work.
- **Clear placeholders.** XML-style tags make it obvious to the model, and to anyone reading logs, which values were masked.

## Where Shadow is different

### Realistic fakes or placeholders?

Placeholders are explicit but read as placeholders, which can make some answers awkward (drafting an email, filling a table, writing code that parses an address). Shadow sends realistic values instead: `jane@acme.com` becomes `hui.zhao@zhilan.cn`, card numbers stay Luhn-valid, IBANs pass mod-97, paths keep working. For bulk data, Shadow's [intent policy](/shadow/concepts/intent-policy/) switches to typed tokens like `[PERSON_0007]`, which are similar in spirit to Rehydra's placeholders.

### Persistent persona and vault

Shadow's mapping persists in an [encrypted vault](/shadow/concepts/vault/) (ChaCha20-Poly1305), and fakes come from a per-machine [persona](/shadow/concepts/personas/). The same customer gets the same fake across sessions and days, and several processes on a machine share one mapping.

### Streaming granularity

Shadow holds back at most the length of the longest fake minus one byte per stream channel, so tokens keep flowing with a delay of a few characters. See [streaming hold-back](/shadow/concepts/how-it-works/#streaming-hold-back). Rehydra buffers by sentence so that NER can see context. Both approaches are valid trade-offs.

### Provider-hosted tools

Shadow's intent policy distinguishes tools your code runs from tools the provider runs (hosted MCP, server-side tools) and decides, per configuration, whether the action target is faked or kept and logged. By default the recipient of a message or action is kept, so the model writes to the right person. See [tool calls](/shadow/concepts/tool-calls/).

### Files

Shadow masks DOCX, XLSX and PPTX uploads in place, extracts PDF text, and handles base64 and multipart payloads. See [file masking](/shadow/concepts/file-masking/).

### Whole-machine coverage

Shadow's [HTTPS proxy](/shadow/integrate/proxy-mode/) with a local CA decrypts only [known AI API hosts](/shadow/reference/providers/) and tunnels everything else, so any tool that honours proxy settings is covered when it calls one of those APIs.

## When should you choose Rehydra?

- Your app is TypeScript and you want masking in-process via npm.
- You need masking in a browser.
- You prefer explicit placeholders the model can't mistake for real data.

## When should you choose Shadow?

- You want realistic fakes that keep answers natural.
- You want one install to cover every tool on a machine that calls a supported AI API.
- You need Office file masking or an intent policy.
- You want a single native binary, or a Rust crate.

## Using them together

Usually you would pick one per path to avoid double masking. A reasonable split: Rehydra inside a browser or TypeScript app, Shadow on developer machines and servers for everything else. If both are in the same path, Shadow will see Rehydra's placeholders as ordinary text and leave them alone unless a detector matches.

## Migrating from Rehydra

1. Install Shadow ([quick start](/shadow/getting-started/quick-start/)).
2. Replace the Rehydra proxy URL in your client with Shadow's gateway (`http://127.0.0.1:8877/gw/<provider>/...`), or use proxy mode.
3. Move custom terms into Shadow's [dictionary](/shadow/concepts/detection/#dictionary-terms).
4. Enable the optional NER sidecar (OpenAI Privacy Filter plus GLiNER) if you relied on Rehydra's NER.
5. Compare with `shadow scan` on representative prompts.

## How do they compare on accuracy?

Rehydra does not publish benchmarks at the time of review and is not in Shadow's [benchmark](/shadow/benchmarks/). There, Shadow's built-in rules leaked 3.08% of sensitive values and round-tripped 100% of documents exactly.

## Sources

- [Rehydra SDK on GitHub](https://github.com/rehydra-ai/rehydra-sdk)
- [rehydra on npm](https://www.npmjs.com/package/rehydra)
- [Shadow streaming hold-back](/shadow/concepts/how-it-works/#streaming-hold-back)

---

*Last reviewed: 2026-10-31. Rehydra is a trademark of its owner; this comparison is based on public documentation as of the review date.*
