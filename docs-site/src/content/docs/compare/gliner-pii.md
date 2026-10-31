---
title: "Shadow vs GLiNER PII — zero-shot PII models inside an LLM privacy proxy"
description: Shadow vs GLiNER PII models (urchade, NVIDIA, Knowledgator). GLiNER detects PII spans; Shadow runs it as a detector and adds reversible masking.
sidebar:
  label: GLiNER PII
  order: 14
keywords:
  - gliner pii alternative
  - gliner pii vs shadow
  - gliner_multi_pii
  - nvidia gliner-pii
  - gliner pii license
  - open source PII detection model
  - multilingual PII detection
  - PII redaction for LLM
faq:
  - q: "What is GLiNER PII?"
    a: "GLiNER is a family of compact named-entity models that find entities for labels you supply at run time (zero-shot). Several checkpoints are tuned for PII, including urchade/gliner_multi_pii-v1 (Apache-2.0), knowledgator/gliner-pii-base-v1.0 (Apache-2.0) and nvidia/gliner-PII (NVIDIA Open Model License)."
  - q: "Is Shadow an alternative to GLiNER?"
    a: "Not directly. GLiNER is a detector; it returns spans. Shadow is a privacy layer that intercepts AI traffic, replaces values with reversible fakes and restores them in responses. Shadow's optional NER sidecar runs urchade/gliner_multi_pii-v1 as one of its detectors."
  - q: "Can I use GLiNER PII commercially?"
    a: "It depends on the checkpoint. urchade/gliner_multi_pii-v1 and knowledgator/gliner-pii-base-v1.0 are Apache-2.0. nvidia/gliner-PII uses the NVIDIA Open Model License, which has its own terms. Read the model card and licence of the exact checkpoint you deploy."
  - q: "How do I use GLiNER with Shadow?"
    a: "Run Shadow's sidecar (ml/serve_hf.py) with the gliner or ensemble backend on 127.0.0.1:8878 and enable [model_detector]. Shadow merges GLiNER's spans with its own detectors and handles masking and restoration."
  - q: "How accurate is GLiNER for PII?"
    a: "It varies a lot by checkpoint and dataset. NVIDIA reports strict F1 of 0.70, 0.64 and 0.87 for gliner-PII on three datasets, the last being its own training distribution. Other benchmarks report much lower figures. See the Shadow benchmarks page."
---

**GLiNER PII models are small, open-weight named-entity models that detect personal data for whatever labels you ask for. Shadow is an open-source privacy layer for AI traffic that detects sensitive values, replaces them with realistic, reversible fakes, and restores them in the response.** They are complementary: Shadow's optional NER sidecar runs `urchade/gliner_multi_pii-v1` (Apache-2.0) to improve recall on names, organisations and addresses, especially outside English.

:::note[TL;DR]
GLiNER finds spans; it doesn't mask traffic or restore answers. Shadow does the masking, vault, intent policy, streaming and tool-call restoration, and can use GLiNER as a detector. Pick the GLiNER checkpoint by licence as much as by accuracy.
:::

## At a glance

| | Shadow | GLiNER PII models |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | urchade and Knowledgator checkpoints Apache-2.0; `nvidia/gliner-PII` NVIDIA Open Model License |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Python library (`gliner`) and model weights; NVIDIA also offers a NIM |
| **Protection method** | Reversible realistic fakes; typed tokens for reports; optional redaction | Detector only: returns labelled spans |
| **Restores responses** | Yes, automatically | n/a |
| **Streaming** | Yes, SSE and NDJSON | n/a |
| **Tool calls** | Yes, restored locally; intent-aware | n/a |
| **Files** | DOCX, XLSX, PPTX in place; PDF to text | n/a |
| **Self-hosted** | Yes | Yes |
| **Price model** | Free | Free (NIM access needs an NVIDIA NGC key) |

## What GLiNER does well

- **Zero-shot labels.** You pass the entity labels at inference time ("person", "organization", "passport number"), so you can add a new type without retraining.
- **Multilingual checkpoints.** `urchade/gliner_multi_pii-v1` covers several languages, which helps where English-centric detectors miss names.
- **Small and fast.** Compact encoders run on a CPU, and much faster on a GPU, without a large language model.
- **Organisations and free text.** NER models catch names and companies that regex and dictionaries can't know about.
- **Ecosystem.** GLiNER is used by NVIDIA NeMo Guardrails (via the GLiNER-PII NIM), DataFog and others, and there are several PII-tuned checkpoints to choose from.

## Where Shadow is different

### What happens after detection?

GLiNER returns spans and stops. To protect an LLM prompt you still need replacements, a mapping to restore the answer, stream handling for values split across events, tool-call restoration, and interception of tools you didn't write. Shadow provides all of these: consistent, format-preserving fakes from a [persona](/shadow/concepts/personas/), an [encrypted vault](/shadow/concepts/vault/), [streaming restore](/shadow/concepts/how-it-works/#streaming-hold-back) and [tool-call](/shadow/concepts/tool-calls/) handling.

### Structured identifiers need more than NER

Card numbers, IBANs, IPs and API keys are better caught by patterns with checksums. Shadow's built-in detectors handle those, a dictionary covers your own codenames and customers, and local machine facts cover usernames and hostnames. GLiNER adds recall on free-text entities. See [detection](/shadow/concepts/detection/).

### Policy and files

Shadow's [intent policy](/shadow/concepts/intent-policy/) decides between a fake, a typed token like `[PERSON_0007]`, keeping a value, or redaction. [File masking](/shadow/concepts/file-masking/) applies the same detectors to DOCX, XLSX and PPTX uploads.

### Public names

Zero-shot NER tends to flag well-known names such as "Microsoft" or "London". Shadow applies a public-name allow list after the sidecar, so these are not masked unnecessarily.

## When should you use GLiNER on its own?

- You are building a custom pipeline (logs, datasets, search indexing) and need flexible entity types.
- You need a detector inside a Python app with labels that change often.
- You only need to find entities, not mask live AI traffic.

## When should you choose Shadow?

- You want AI requests masked and responses restored with no code changes.
- You use streaming, agents and file uploads.
- You want structured identifiers, your own terms and NER combined in one place.

## How do you run GLiNER inside Shadow?

Shadow's open-weight sidecar, `ml/serve_hf.py`, has three backends: `gliner` (`urchade/gliner_multi_pii-v1`), `privacy-filter` (OpenAI Privacy Filter) and `ensemble` (both, the default). All bundled models are Apache-2.0.

```sh
cd ml
pip install -r requirements-hf.txt
python serve_hf.py --backend gliner --host 127.0.0.1 --port 8878
```

```toml
[model_detector]
enabled = true
url = "http://127.0.0.1:8878/detect"   # must be loopback
timeout_ms = 300
min_score = 0.6
```

The sidecar maps GLiNER labels to Shadow kinds (person, org, address, email, phone, national ID, IBAN, credit card and others). If it is slow or down, Shadow fails open to its built-in detectors. See [local NER sidecar](/shadow/concepts/detection/#local-ner-sidecar) and [Shadow vs OpenAI Privacy Filter](/shadow/compare/openai-privacy-filter/).

:::caution[Check the checkpoint licence]
The bundled sidecar uses Apache-2.0 weights. If you swap in `nvidia/gliner-PII` or a model trained on restricted ai4privacy datasets, review its licence first; some PII models carry non-commercial terms.
:::

## Migrating from a GLiNER-based redaction script

1. Start the sidecar with `--backend gliner` or `ensemble`, and enable `[model_detector]`.
2. Remove your own replacement and restoration code around model calls.
3. Route model traffic through Shadow's [gateway](/shadow/integrate/gateway-mode/) or [proxy](/shadow/integrate/proxy-mode/).
4. Move any custom labels you relied on into the sidecar's label map or Shadow [dictionary terms](/shadow/concepts/detection/#dictionary-terms).
5. Compare findings with `shadow scan --json`.

## How do they compare on accuracy?

Published GLiNER figures vary widely. NVIDIA reports strict F1 of 0.70 (Argilla PII), 0.64 (AI4Privacy) and 0.87 (Nemotron-PII, its own training distribution) for `gliner-PII`, while a vendor-run Tonic benchmark scored a generic GLiNER setup at 0.32 aggregate F1. Shadow's [benchmark](/shadow/benchmarks/) evaluates GLiNER as a baseline and as part of Shadow, and flags results on datasets a model was trained on.

## Sources

- [urchade/gliner_multi_pii-v1](https://huggingface.co/urchade/gliner_multi_pii-v1)
- [nvidia/gliner-PII](https://huggingface.co/nvidia/gliner-PII)
- [knowledgator/gliner-pii-base-v1.0](https://huggingface.co/knowledgator/gliner-pii-base-v1.0)
- [GLiNER on GitHub](https://github.com/urchade/GLiNER)
- [Tonic Textual benchmark (vendor-run)](https://www.tonic.ai/ai-model-benchmarks/textual-benchmark)

---

*Last reviewed: 2026-10-31. GLiNER and NVIDIA are trademarks of their respective owners; this comparison is based on public documentation as of the review date.*
