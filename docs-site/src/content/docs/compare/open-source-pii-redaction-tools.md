---
title: "Best open-source PII redaction tools for LLMs in 2026"
description: An honest guide to open-source PII redaction tools for LLM prompts in 2026, including Presidio, LLM Guard, GLiNER, OpenAI Privacy Filter, Rehydra and Shadow.
sidebar:
  label: Open-source PII redaction tools
  order: 1
keywords:
  - open source PII redaction
  - best PII redaction tools 2026
  - PII redaction for LLM
  - open source PII detection
  - presidio alternative
  - LLM PII masking open source
  - self-hosted PII anonymization
faq:
  - q: What is the best open-source PII redaction tool for LLMs?
    a: "There is no single best tool. Presidio is the most established detector library. OpenAI Privacy Filter and GLiNER PII models are strong free detection models. LLM Guard and Rehydra add reversible placeholders for LLM calls. Shadow adds realistic reversible fakes, streaming and tool-call restoration, file masking and a local proxy, and can use Presidio and the open models as detectors."
  - q: Which open-source tools can restore PII in the LLM response?
    a: "LLM Guard (Anonymize and Deanonymize with a shared vault), Rehydra (placeholders with an encrypted session map) and Shadow (realistic fakes with an encrypted vault) restore values. Presidio can reverse only its encrypt operator. NeMo Guardrails, LangChain PIIMiddleware and DataFog do not document restoration."
  - q: Are open-source PII models licensed for commercial use?
    a: "Check each checkpoint. OpenAI Privacy Filter and urchade/gliner_multi_pii-v1 are Apache-2.0. nvidia/gliner-PII uses the NVIDIA Open Model License. Some community models trained on older ai4privacy datasets carry non-commercial licences, including one optional LLM Guard recognizer that is CC-BY-NC-4.0."
  - q: Do open-source PII tools support languages other than English?
    a: "Some do. Presidio supports any language you have a spaCy, Stanza or transformer model for. GLiNER multi-PII is multilingual. OpenAI Privacy Filter and LLM Guard are primarily English. Shadow's multilingual recall is being measured on the benchmarks page."
  - q: Is Shadow free?
    a: Yes. Shadow is Apache-2.0, self-hosted, with no account and no telemetry.
---

**The strongest open-source options for keeping PII out of LLM prompts in 2026 are Presidio for detection, OpenAI Privacy Filter and GLiNER for model-based detection, LLM Guard and Rehydra for reversible placeholders, and Shadow for reversible realistic fakes with streaming, tool-call and file support.** They solve different parts of the problem, and the best setups often combine a detector with a layer that handles the round trip.

This guide is written by the Shadow maintainers, so read it with that in mind. We have tried to be fair, and Shadow is not the right answer in every row.

## What to look for

Detection is only half the job when the text goes to an LLM. Ask:

1. **Recall and precision.** Does it find what matters in your data, without masking public names the model needs?
2. **What replaces the value?** `[REDACTED]`, a placeholder like `<PERSON_1>`, or a realistic fake? Placeholders are safe but can degrade answers.
3. **Is it reversible?** Can the response, streamed or not, be turned back into real values?
4. **Agents and tools.** Are tool-call arguments restored before tools run?
5. **Files.** Are uploaded documents covered?
6. **Licence.** Is the code *and* every model checkpoint usable commercially?
7. **Operational fit.** Python library, container, Node SDK, or a single binary?

## The shortlist

| Tool | Type | Licence | Replacement | Reversible | Streaming | Tool calls | Best for |
|---|---|---|---|---|---|---|---|
| [Presidio](/shadow/compare/presidio/) | Detector + anonymizer library | MIT | Placeholder, mask, hash, encrypt | Only `encrypt` | No | No | Pipelines, OCR, custom recognizers |
| [OpenAI Privacy Filter](/shadow/compare/openai-privacy-filter/) | Detection model | Apache-2.0 | Detector only | n/a | n/a | n/a | Strong English detection, local |
| [GLiNER PII](/shadow/compare/gliner-pii/) | Detection models | Apache-2.0 or NVIDIA Open Model License | Detector only | n/a | n/a | n/a | Zero-shot labels, multilingual |
| [LLM Guard](/shadow/compare/llm-guard/) | Scanner library | MIT | Placeholders, optional Faker | Yes | Not documented | No | Python apps wanting many scanners |
| [NeMo Guardrails](/shadow/compare/nemo-guardrails/) | Guardrail framework | Apache-2.0 | Label mask or block | No | Output rails | Retrieval rails | Conversational rails |
| [LangChain PIIMiddleware](/shadow/compare/langchain-pii-middleware/) | Agent middleware | MIT | Redact, mask, hash, block | No | n/a | Tool results | LangChain agents |
| [DataFog](/shadow/compare/datafog/) | Library + hooks | MIT | Redact, replace, hash | Not documented | Not documented | No | Fast regex-first scrubbing |
| [Rehydra](/shadow/compare/rehydra/) | SDK + local proxy | MIT | Reversible placeholders | Yes | Yes | Yes | TypeScript and browser apps |
| **Shadow** | Proxy, gateway, Rust library | Apache-2.0 | Reversible realistic fakes; tokens for reports | Yes | Yes | Yes, intent-aware | Endpoint and service traffic to any provider |

## 1. Presidio: the reference detector

Presidio has the broadest recognizer set, a pluggable NLP engine (spaCy, Stanza, transformers), an image redactor with OCR, and a large ecosystem; many gateways use it as their PII backend. Its anonymizer is built for one-way transformations; only `encrypt` can be reversed, and there is no LLM-specific streaming logic. The repository now lives at `data-privacy-stack/presidio`, still MIT.

**Choose it for** data pipelines and as a detector inside other tools. **Pair it with** Shadow if you need the round trip: Shadow can call Presidio's Analyzer as an extra detector. [Shadow vs Presidio](/shadow/compare/presidio/).

## 2. OpenAI Privacy Filter: a strong free model

Released in April 2026 under Apache-2.0, it is a token-classification model with eight labels (person, address, email, phone, URL, date, account number, secret). OpenAI reports an F1 of 96% on pii-masking-300k (self-reported). It is primarily English and is a detector only. **Shadow's optional NER sidecar can run it**, alone or in an ensemble with GLiNER. [Shadow vs OpenAI Privacy Filter](/shadow/compare/openai-privacy-filter/).

## 3. GLiNER PII models: zero-shot and multilingual

GLiNER models take entity labels at inference time, so you can ask for "passport number" or "IBAN" without retraining. `urchade/gliner_multi_pii-v1` is Apache-2.0 and multilingual; `nvidia/gliner-PII` uses the NVIDIA Open Model License. Published scores vary a lot by dataset. Shadow's sidecar uses the Apache-2.0 multilingual checkpoint. [Shadow vs GLiNER](/shadow/compare/gliner-pii/).

## 4. LLM Guard: scanners with a reversible vault

LLM Guard's Anonymize and Deanonymize scanners share a `Vault`, so placeholders can be restored. It also offers many non-PII scanners such as prompt-injection detection. Streaming is not documented, detection is English-focused, and one optional recognizer model is CC-BY-NC-4.0, so check licences before commercial use. [Shadow vs LLM Guard](/shadow/compare/llm-guard/).

## 5. NeMo Guardrails: rails, not round trips

NVIDIA's framework runs input, output and retrieval rails using Presidio or the GLiNER-PII NIM. It masks with labels or blocks, and does not restore. Excellent for conversational policy; less so when answers must contain the real values. [Shadow vs NeMo Guardrails](/shadow/compare/nemo-guardrails/).

## 6. LangChain PIIMiddleware: native to LangChain agents

It applies redact, mask, hash or block strategies to inputs, outputs and tool results inside a LangChain agent. It is convenient if you are already on LangChain, but it is not reversible. [Shadow vs LangChain PIIMiddleware](/shadow/compare/langchain-pii-middleware/).

## 7. DataFog: lightweight scrubbing

DataFog offers regex, spaCy and GLiNER engines, a LiteLLM guardrail and a Claude Code hook. Its modes are redact, replace and hash; reversibility is not documented. [Shadow vs DataFog](/shadow/compare/datafog/).

## 8. Rehydra: the closest open-source analogue

Rehydra is a TypeScript SDK and local proxy with regex plus ONNX NER, an AES-256-GCM session map, sentence-buffered streaming rehydration and tool-call argument rehydration. It uses XML-style placeholders rather than realistic fakes and is English and UK-centric. If you work in Node, Bun or the browser, it is a natural fit. [Shadow vs Rehydra](/shadow/compare/rehydra/).

## 9. Shadow: reversible realistic fakes at the endpoint

Shadow replaces values with realistic, format-preserving fakes from a persistent [persona](/shadow/concepts/personas/), stores the mapping in an [encrypted local vault](/shadow/concepts/vault/), and restores responses including [SSE streams and tool calls](/shadow/concepts/tool-calls/). Its [intent policy](/shadow/concepts/intent-policy/) switches to typed tokens for reports and handles provider-hosted tools explicitly. It masks [DOCX, XLSX and PPTX uploads in place](/shadow/concepts/file-masking/). It runs as an [HTTPS proxy](/shadow/integrate/proxy-mode/), a [gateway](/shadow/integrate/gateway-mode/) or a [Rust library](/shadow/integrate/rust/).

**Where it is weaker:** no OCR, no Python API, multilingual recall still being measured, and detection that depends on regex, dictionaries and optional models. For the most recall, enable [Presidio and the NER sidecar](/shadow/concepts/detection/).

## Recommended combinations

| Situation | Suggested stack |
|---|---|
| Python data pipeline, one-way anonymisation | Presidio (+ GLiNER or Privacy Filter recognizers) |
| Live LLM traffic, answers must stay useful | Shadow, with Presidio and/or the NER sidecar for recall |
| TypeScript app or browser extension | Rehydra |
| LangChain agent, values never needed back | LangChain PIIMiddleware |
| Need many safety scanners in one Python library | LLM Guard |
| Conversational policy and topical rails | NeMo Guardrails, with Shadow in front for the round trip |

## How accurate are they?

Published numbers for the same tool vary by up to five times between datasets because of label mapping and annotation style, so headline F1 scores rarely transfer. The Shadow [benchmark](/shadow/benchmarks/) runs these open-source tools side by side on licence-clean datasets and measures leak rate, over-masking, round-trip exactness and latency, with the mapping and code published.

## Sources

- [Presidio](https://github.com/data-privacy-stack/presidio)
- [OpenAI Privacy Filter model card](https://huggingface.co/openai/privacy-filter)
- [urchade/gliner_multi_pii-v1](https://huggingface.co/urchade/gliner_multi_pii-v1), [nvidia/gliner-PII](https://huggingface.co/nvidia/gliner-PII)
- [LLM Guard Anonymize](https://protectai.github.io/llm-guard/input_scanners/anonymize/)
- [NeMo Guardrails PII detection](https://docs.nvidia.com/nemo/guardrails/latest/configure-guardrails/guardrail-catalog/pii-detection)
- [LangChain PIIMiddleware](https://reference.langchain.com/python/langchain/agents/middleware/pii)
- [DataFog](https://github.com/DataFog/datafog-python)
- [Rehydra SDK](https://github.com/rehydra-ai/rehydra-sdk)

---

*Last reviewed: 2026-10-31. All product names are trademarks of their respective owners; this guide is based on public documentation as of the review date.*
