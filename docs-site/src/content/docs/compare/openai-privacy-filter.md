---
title: "Shadow vs OpenAI Privacy Filter — using the open-weight PII model in a proxy"
description: Shadow vs OpenAI Privacy Filter. Privacy Filter is a free PII detection model; Shadow can run it as a detector and adds reversible masking and restore.
sidebar:
  label: OpenAI Privacy Filter
  order: 27
keywords:
  - openai privacy filter alternative
  - openai privacy filter vs shadow
  - openai privacy filter proxy
  - openai/privacy-filter
  - open source PII detection model
  - PII redaction for LLM
  - local PII detection
faq:
  - q: "What is OpenAI Privacy Filter?"
    a: "An open-weight PII detection model released by OpenAI in April 2026 under Apache-2.0. It labels eight kinds of span (account numbers, addresses, emails, people, phones, URLs, dates and secrets) and runs locally with Hugging Face Transformers or Transformers.js."
  - q: "Is Shadow an alternative to OpenAI Privacy Filter?"
    a: "They do different jobs. Privacy Filter is a detector: it finds spans and stops there. Shadow is a privacy layer that intercepts AI traffic, replaces values with reversible fakes and restores them in responses. Shadow's optional NER sidecar can run Privacy Filter as one of its detectors."
  - q: "How do I use OpenAI Privacy Filter with Shadow?"
    a: "Run Shadow's open-weight sidecar (ml/serve_hf.py) with the privacy-filter or ensemble backend on 127.0.0.1:8878, then enable [model_detector] in Shadow's configuration. Shadow merges its findings with the built-in detectors."
  - q: "Does OpenAI Privacy Filter send data to OpenAI?"
    a: "No. The weights are downloaded once and the model runs on your hardware. Inference does not call an OpenAI API."
  - q: "How accurate is OpenAI Privacy Filter?"
    a: "OpenAI publishes its own self-reported scores. On Shadow's held-out benchmark, Privacy Filter alone leaked 11.01% of sensitive values, against 3.08% for Shadow's built-in rules and 0.08% for Shadow with the open-weight NER sidecar, which includes Privacy Filter. It is primarily English."
---

**OpenAI Privacy Filter is a free, open-weight model (Apache-2.0) that detects personal data in text. Shadow is an open-source privacy layer for AI traffic that detects sensitive values, replaces them with realistic, reversible fakes, and restores them in responses.** The two fit together: Shadow's optional NER sidecar can run Privacy Filter, so you get its recall on names and addresses plus Shadow's masking, vault, policy and restoration.

:::note[TL;DR]
Privacy Filter answers "where is the PII in this text?" Shadow answers "how do I send this to a model without exposing it, and get a useful answer back?" Use Privacy Filter inside Shadow, not instead of it, unless you only need detection.
:::

## At a glance

| | Shadow | OpenAI Privacy Filter |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Yes, Apache-2.0 (model weights) |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Model you run with Transformers (Python) or Transformers.js |
| **Protection method** | Reversible realistic fakes; typed tokens for reports; optional redaction | Detector only: returns labelled spans |
| **Restores responses** | Yes, automatically | n/a |
| **Streaming** | Yes, SSE and NDJSON | n/a |
| **Tool calls** | Yes, restored locally; intent-aware | n/a |
| **Files** | DOCX, XLSX, PPTX in place; PDF to text | n/a (text in, spans out) |
| **Self-hosted** | Yes | Yes |
| **Price model** | Free | Free |

## What OpenAI Privacy Filter does well

- **Strong reported accuracy.** OpenAI publishes high self-reported scores on the pii-masking-300k dataset family. On Shadow's held-out benchmark it was the strongest standalone detector tested, leaking 11.01% of sensitive values.
- **Efficient architecture.** About 1.5B total parameters with roughly 50M active per token, so it is practical on a CPU for moderate volumes and fast on a GPU.
- **Permissive licence.** Apache-2.0 weights, usable commercially without the non-commercial terms that affect some PII models trained on restricted datasets.
- **Runs anywhere.** Python with Transformers, or in the browser and Node with Transformers.js.
- **Useful labels.** Its eight labels include `secret` and `account_number`, not only classic named entities, which suits developer and finance content.

## Where Shadow is different

### Is a detector enough to protect LLM prompts?

No. After detection you still need to decide what to replace each span with, keep a mapping so the answer can be restored, handle values split across streamed events, restore tool-call arguments before your code acts, and intercept traffic from tools you didn't write. Shadow does all of that. Privacy Filter is one possible input to it.

### Realistic, reversible replacements

Shadow turns detected values into consistent, format-preserving fakes from a persistent [persona](/shadow/concepts/personas/) and stores the pairs in an [encrypted vault](/shadow/concepts/vault/). Responses, [streams](/shadow/concepts/how-it-works/#streaming-hold-back) and [tool calls](/shadow/concepts/tool-calls/) are restored automatically.

### Structured values and your own terms

Regex and checksum detectors are more reliable than a language model for IBANs (mod-97), card numbers (Luhn), IPs, MACs and API key formats. Shadow's built-in detectors cover these, plus a dictionary for company names, customers and codenames no public model knows, and local machine facts like your OS username and hostname. See [detection](/shadow/concepts/detection/).

### Intent and files

Shadow's [intent policy](/shadow/concepts/intent-policy/) picks fakes, typed tokens or keep per value; [file masking](/shadow/concepts/file-masking/) covers DOCX, XLSX and PPTX uploads.

### Fail-open design

If the sidecar is slow or down, Shadow keeps masking with its built-in detectors, capped by `timeout_ms`, so a model hiccup does not stop traffic or bypass masking entirely.

## When should you use OpenAI Privacy Filter on its own?

- You are building your own redaction pipeline, for example for logs or datasets.
- You need detection in a browser or Node app via Transformers.js.
- You only need to flag or count PII, not mask live traffic.

## When should you choose Shadow?

- You want prompts to AI providers masked and answers restored without writing that pipeline.
- You need streaming, tool calls and file uploads handled.
- You want structured identifiers and your own terms covered alongside NER.

## How do you run Privacy Filter inside Shadow?

Shadow ships an optional open-weight sidecar, `ml/serve_hf.py`. Its backends are `privacy-filter` (OpenAI Privacy Filter), `gliner` (`urchade/gliner_multi_pii-v1`, Apache-2.0) and `ensemble` (both, the default). Models download on first run.

```sh
cd ml
pip install -r requirements-hf.txt
python serve_hf.py --backend ensemble --host 127.0.0.1 --port 8878
```

```toml
[model_detector]
enabled = true
url = "http://127.0.0.1:8878/detect"   # must be loopback
timeout_ms = 300
min_score = 0.6
```

Shadow merges the sidecar's spans with its own, resolves overlaps and applies its public-name allow list, so over-eager spans on well-known names are dropped. The sidecar binds to loopback only. See [detection](/shadow/concepts/detection/#local-ner-sidecar).

:::tip
Use `ensemble` when your content includes non-English names or organisation names: Privacy Filter is strongest on English, and GLiNER adds multilingual and organisation coverage. See [Shadow vs GLiNER PII](/shadow/compare/gliner-pii/).
:::

## Migrating from a Privacy Filter-based redaction script

1. Start the sidecar with `--backend privacy-filter` (or `ensemble`) and enable `[model_detector]`.
2. Remove your own span-replacement and post-processing code around model calls.
3. Route model calls through Shadow's [gateway](/shadow/integrate/gateway-mode/) or [proxy](/shadow/integrate/proxy-mode/).
4. Add your organisation's terms to the [dictionary](/shadow/concepts/detection/#dictionary-terms).
5. Check results with `shadow scan --json`, which shows which detector found each span.

## How do they compare on accuracy?

Privacy Filter's published numbers are self-reported on a dataset family whose licence restricts redistribution, and it is primarily English. Shadow's [benchmark](/shadow/benchmarks/) evaluates Privacy Filter as a baseline and Shadow with and without the sidecar on a licence-clean held-out set, with code and label mappings published: Privacy Filter alone leaked 11.01% of sensitive values, Shadow's built-in rules 3.08%, and Shadow with the sidecar 0.08%.

## Sources

- [openai/privacy-filter on Hugging Face](https://huggingface.co/openai/privacy-filter)
- [Introducing OpenAI Privacy Filter](https://openai.com/blog/introducing-openai-privacy-filter)
- [Shadow detection: local NER sidecar](/shadow/concepts/detection/#local-ner-sidecar)

---

*Last reviewed: 2026-10-31. OpenAI is a trademark of its owner; this comparison is based on public documentation as of the review date.*
