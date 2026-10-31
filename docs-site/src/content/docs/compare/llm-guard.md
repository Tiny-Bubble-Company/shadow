---
title: "Shadow vs LLM Guard — open-source LLM Guard alternative for PII masking"
description: Shadow vs LLM Guard (Protect AI). Both reverse PII in LLM responses; Shadow adds realistic fakes, streaming restore, tool calls and a proxy.
sidebar:
  label: LLM Guard
  order: 22
keywords:
  - llm guard alternative
  - llm guard vs shadow
  - protect ai llm guard
  - llm guard anonymize deanonymize
  - llm guard streaming
  - open source PII redaction
  - PII redaction for LLM
  - LLM guardrails
faq:
  - q: "Is Shadow an alternative to LLM Guard?"
    a: "For PII protection, yes. LLM Guard is a Python toolkit of input and output scanners, including Anonymize and Deanonymize. Shadow focuses only on sensitive data: it masks prompts with realistic, reversible fakes and restores them in responses, streams and tool calls, as a proxy, gateway or Rust library. LLM Guard also covers prompt injection, toxicity and other risks that Shadow does not."
  - q: "Can LLM Guard restore anonymised values in the response?"
    a: "Yes. The Anonymize input scanner stores originals in a Vault and the Deanonymize output scanner puts them back. By default the model sees placeholders; an optional use_faker setting substitutes fake values instead."
  - q: "Does LLM Guard support streaming responses?"
    a: "Public documentation does not describe deanonymising streamed responses, and streaming has appeared as a roadmap item. Shadow restores SSE and NDJSON streams on the fly, including fakes split across events."
  - q: "Is LLM Guard free for commercial use?"
    a: "LLM Guard itself is MIT-licensed. Check the licences of the models it downloads: one optional recognizer model used by the Anonymize scanner is published under CC-BY-NC-4.0, which restricts commercial use."
  - q: "Which is faster, LLM Guard or Shadow?"
    a: "LLM Guard publishes latency figures for its scanners, which depend on hardware and whether ONNX or a GPU is used. Shadow's built-in detectors are in-process Rust; optional NER models add their own time. See the Shadow benchmarks page for measured, reproducible numbers."
---

**LLM Guard, from Protect AI, is an open-source Python toolkit of scanners that check LLM inputs and outputs, including an Anonymize/Deanonymize pair for PII. Shadow is an open-source privacy layer that does one job: it replaces sensitive values with realistic, reversible fakes before requests reach the provider and restores them in responses, streams and tool calls.** If you want a broad guardrail toolkit in Python, LLM Guard is a strong option. If you want PII masking with restoration across every tool on a machine, without changing application code, Shadow is built for that.

:::note[TL;DR]
LLM Guard and Shadow share the same core idea for PII: replace on the way out, restore on the way back. LLM Guard does it inside your Python app with placeholders by default and also scans for prompt injection, toxicity and more. Shadow does it at the network layer with persona-based fakes, restores streamed responses and tool-call arguments, and masks Office uploads.
:::

## At a glance

| | Shadow | LLM Guard |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Yes, MIT (check licences of downloaded models) |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Python library; API container (`laiyer/llm-guard-api`) |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Placeholders by default; optional Faker values (`use_faker`) |
| **Restores responses** | Yes, automatically | Yes, with the Deanonymize output scanner and a shared Vault |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Not documented |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Not documented |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | Not documented |
| **Self-hosted** | Yes, single binary, no account | Yes |
| **Price model** | Free | Free |

## What LLM Guard does well

- **Breadth of scanners.** Beyond PII, LLM Guard has input scanners for prompt injection, secrets, banned topics, toxicity, token limits and more, and output scanners for things like relevance, bias and malicious URLs. Shadow covers only sensitive data.
- **Reversible anonymisation in the same library.** The [Anonymize](https://protectai.github.io/llm-guard/input_scanners/anonymize/) and [Deanonymize](https://protectai.github.io/llm-guard/output_scanners/deanonymize/) scanners share a `Vault`, so a Python app can mask a prompt and restore the answer with a few lines of code.
- **Transformer-based NER.** The Anonymize scanner combines regex patterns with NER models, which helps with free-text names that pattern matching misses.
- **Published latency.** The project documents per-scanner latency on CPU and GPU, with ONNX options to speed things up.
- **Easy to try.** `pip install llm-guard` or run the API container, and you have a working scanner pipeline.

## Where Shadow is different

### Placeholders or realistic fakes?

LLM Guard's default replaces a name with a placeholder like `[REDACTED_PERSON_1]`. Its optional `use_faker` mode substitutes a fake value, but each value is generated independently. Shadow draws every fake from a persistent [persona](/shadow/concepts/personas/): your company maps to one fake company, emails at your domain map to its domain, card numbers stay Luhn-valid and IBANs pass mod-97. Prompts stay coherent, and the same customer gets the same fake tomorrow.

### Does it restore streamed responses and tool calls?

Most agent and chat clients stream. Shadow parses SSE and NDJSON events for OpenAI, Anthropic, Gemini, Cohere and Ollama-style APIs and restores fakes even when they are split across events, with a hold-back of only a few characters. See [streaming hold-back](/shadow/concepts/how-it-works/#streaming-hold-back). Tool-call arguments are restored before your code runs the tool ([tool calls](/shadow/concepts/tool-calls/)). LLM Guard's public docs do not describe either case.

### Intent-aware policy

Shadow classifies each request as act, report, code or converse and picks a treatment per value: fakes for conversation, typed tokens such as `[PERSON_0007]` for bulk reports, and an explicit, logged `keep` for the target of a provider-hosted action. See [intent-aware policy](/shadow/concepts/intent-policy/).

### No code changes

LLM Guard runs where you call it. Shadow runs as an [HTTPS proxy](/shadow/integrate/proxy-mode/) or [gateway](/shadow/integrate/gateway-mode/), so IDE assistants, CLIs and desktop apps you don't control are covered too.

### Files and runtime

Shadow masks DOCX, XLSX and PPTX uploads in place ([file masking](/shadow/concepts/file-masking/)) and ships as a single Rust binary with an [encrypted vault](/shadow/concepts/vault/). There is no Python runtime or model download required for the built-in detectors.

## When should you choose LLM Guard?

- You need prompt-injection, toxicity or topic scanning as well as PII handling, in one Python toolkit.
- You are building a Python application and want masking inside your own code path.
- You want transformer NER for names without running a separate sidecar.

## When should you choose Shadow?

- You need masking for tools you didn't write: IDE agents, CLIs, desktop apps, other teams' services.
- Responses are streamed or agents call tools, and the answers must come back with real values.
- You want realistic, consistent fakes rather than placeholders.
- You need Office uploads masked before they leave.

## Can you use LLM Guard with Shadow?

Yes. They solve overlapping but different problems. A common split is LLM Guard inside your application for prompt-injection and output-safety checks, with Shadow at the network edge handling PII masking and restoration for every outbound model call. If both anonymise, let only one of them do it, otherwise the second sees already-masked text. Shadow skips values that are already its own fakes, but it does not know LLM Guard's placeholders.

## Migrating from LLM Guard's Anonymize/Deanonymize

1. Keep any non-PII LLM Guard scanners you rely on.
2. Remove the `Anonymize` and `Deanonymize` scanners and their `Vault` from your pipeline.
3. Point your SDK at Shadow's gateway (`http://127.0.0.1:8877/gw/<provider>/...`) or set `HTTPS_PROXY` ([integration modes](/shadow/integrate/overview/)).
4. Add organisation names, customers and codenames as [dictionary terms](/shadow/concepts/detection/#dictionary-terms); optionally enable the NER sidecar or Presidio for free-text names.
5. Compare coverage with `shadow scan` on a sample of real prompts before switching.

## How do they compare on accuracy?

Both tools' detection depends on configuration and models. Shadow's [benchmark](/shadow/benchmarks/) measures leak rate, over-masking, round-trip exactness and latency on public datasets with published code and label mappings. LLM Guard's own documentation publishes latency figures; check them against your hardware.

## Sources

- [LLM Guard documentation](https://protectai.github.io/llm-guard/)
- [Anonymize input scanner](https://protectai.github.io/llm-guard/input_scanners/anonymize/)
- [Deanonymize output scanner](https://protectai.github.io/llm-guard/output_scanners/deanonymize/)
- [LLM Guard on GitHub](https://github.com/protectai/llm-guard)
- [Shadow intent-aware policy](/shadow/concepts/intent-policy/)

---

*Last reviewed: 2026-10-31. LLM Guard and Protect AI are trademarks of their owner; this comparison is based on public documentation as of the review date.*
