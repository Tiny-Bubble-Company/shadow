---
title: "Shadow vs NeMo Guardrails — NeMo Guardrails alternative for PII masking"
description: Shadow vs NVIDIA NeMo Guardrails for PII. NeMo masks or blocks with labels; Shadow uses reversible fakes and restores responses and tool calls.
sidebar:
  label: NeMo Guardrails
  order: 24
keywords:
  - nemo guardrails alternative
  - nemo guardrails vs shadow
  - nvidia nemo guardrails pii
  - nemo guardrails pii masking
  - gliner pii nim
  - LLM guardrails
  - PII redaction for LLM
  - open source AI DLP
faq:
  - q: "Is Shadow an alternative to NeMo Guardrails?"
    a: "For sensitive-data protection, partly. NeMo Guardrails is a general guardrail framework for dialogue, topical, safety and retrieval rails, with PII detection as one rail. Shadow does only data protection: it replaces sensitive values with reversible fakes and restores them in responses. Many teams could use both."
  - q: "Does NeMo Guardrails restore masked PII in the response?"
    a: "Its PII rails detect and either mask values with labels such as [EMAIL] or block the message. Public documentation does not describe restoring the original values in the model's answer."
  - q: "Which PII detectors does NeMo Guardrails use?"
    a: "The documented PII rails can use Microsoft Presidio or the NVIDIA GLiNER-PII model served as a NIM. Shadow can also use Presidio, and its optional open-weight sidecar uses OpenAI Privacy Filter and a GLiNER multilingual PII model."
  - q: "Is NeMo Guardrails free?"
    a: "The framework is open source and free to self-host. Using the hosted GLiNER-PII NIM requires an NVIDIA NGC key. Shadow is free and open source under Apache-2.0."
  - q: "Can NeMo Guardrails mask PII in streaming responses?"
    a: "NeMo Guardrails supports output rails on streamed responses, but those rails check or mask output; they do not restore earlier masked inputs. Shadow restores streamed responses on the fly."
---

**NeMo Guardrails, from NVIDIA, is an open-source framework for adding programmable rails to LLM applications: topical, safety, dialogue, retrieval and PII rails. Shadow is an open-source privacy layer that replaces sensitive values with realistic, reversible fakes before requests leave, and restores them in responses, streams and tool calls.** NeMo is the broader framework; Shadow goes deeper on one problem, keeping data out of provider requests without breaking answers.

:::note[TL;DR]
NeMo Guardrails detects PII (via Presidio or GLiNER-PII) and masks it with labels like `[EMAIL]` or blocks the message, with no restoration. Shadow translates PII into consistent fakes and turns them back into real values in the answer, including streamed text and tool-call arguments. Use NeMo for conversation and safety rails; use Shadow for reversible data masking.
:::

## At a glance

| | Shadow | NeMo Guardrails |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Yes, Apache-2.0 |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Python framework or guardrails server in front of your app |
| **Protection method** | Reversible realistic fakes from a persona; typed tokens for reports; optional redaction | Block, or mask with entity labels (`[EMAIL]`) |
| **Restores responses** | Yes, automatically | No |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | Output rails on streams; no unmasking |
| **Tool calls** | Yes, arguments restored locally; intent-aware for provider-hosted tools | Not for PII; retrieval rails cover RAG chunks |
| **Files** | DOCX, XLSX, PPTX masked in place; PDF to text; CSV, JSON, HTML | No |
| **Self-hosted** | Yes, single binary, no account | Yes (GLiNER-PII NIM route needs an NGC key) |
| **Price model** | Free | Free |

## What NeMo Guardrails does well

- **A full guardrail framework.** Colang lets you define dialogue flows, topical boundaries, jailbreak and moderation checks, fact-checking and hallucination rails. PII is one rail among many.
- **Three places to apply PII rails.** The [PII detection rail](https://docs.nvidia.com/nemo/guardrails/latest/configure-guardrails/guardrail-catalog/pii-detection) can run on user input, on model output and on retrieved chunks, which covers RAG leaks that a pure prompt proxy may not see in the same way.
- **Pluggable detection.** You can choose Presidio or NVIDIA's GLiNER-PII model, and swap configuration per application.
- **NVIDIA ecosystem.** Integrates with NIM microservices and NVIDIA's model catalog, which suits teams already standardised there.
- **Open source and active.** The project lives at `NVIDIA-NeMo/Guardrails` on GitHub.

## Where Shadow is different

### What happens to masked values in the answer?

NeMo replaces `jane@acme.com` with `[EMAIL]`. The model can no longer tell two addresses apart, and the answer comes back with labels instead of your data. Shadow replaces it with `hui.zhao@zhilan.cn`, a fake drawn from a persistent [persona](/shadow/concepts/personas/), and restores the original in the response. Answers mention your real customers, not placeholders.

### Streaming and tool calls

Shadow restores fakes in [server-sent event streams](/shadow/concepts/how-it-works/#streaming-hold-back) even when they are split across events, and restores [tool-call arguments](/shadow/concepts/tool-calls/) before your code runs the tool, so an agent's own `send_email` tool reaches the real address while the provider only saw a fake.

### Intent-aware policy

Shadow decides per request whether values become fakes, typed tokens (`[PERSON_0007]` for bulk reports), a logged `keep` for message recipients and provider-hosted action targets, or a redaction. See [intent-aware policy](/shadow/concepts/intent-policy/).

### Covers tools you don't control

NeMo Guardrails wraps an application you build. Shadow sits at the network layer as an [HTTPS proxy](/shadow/integrate/proxy-mode/) or [gateway](/shadow/integrate/gateway-mode/), so IDE agents, CLIs and third-party apps that call a [supported provider API](/shadow/reference/providers/) are covered too. It also masks [uploaded Office files](/shadow/concepts/file-masking/) in place.

## When should you choose NeMo Guardrails?

- You need dialogue control, topical rails, jailbreak checks or fact-checking, not only PII handling.
- You want PII checks on retrieved RAG chunks as part of the same framework.
- You are building a Python application and are comfortable with Colang configuration.
- Masking with labels, with no restoration, is acceptable for your use case.

## When should you choose Shadow?

- Answers must contain the real values, restored locally.
- You rely on streaming or agent tool calls.
- You need coverage for tools and services you did not write.
- You want consistent fakes across sessions and Office uploads masked.

## Can you use NeMo Guardrails with Shadow?

Yes. Put NeMo Guardrails in your application for dialogue and safety rails, and route its outbound model calls through Shadow's gateway. Shadow masks every request before it leaves and restores the answer before NeMo's output rails see it. If you do this, disable NeMo's PII masking on input, otherwise Shadow receives `[EMAIL]` labels instead of values it can translate and restore.

Both can share detectors: Shadow can call [Presidio](/shadow/concepts/detection/#presidio), and its optional open-weight sidecar runs OpenAI Privacy Filter and `urchade/gliner_multi_pii-v1`, both Apache-2.0. See [detection](/shadow/concepts/detection/).

## Migrating NeMo PII rails to Shadow

1. Keep your non-PII rails.
2. Remove the PII detection rail from input (and output, if it only masked values you now want restored).
3. Configure your LLM provider's base URL to Shadow's gateway, for example `http://127.0.0.1:8877/gw/openai/v1` ([gateway mode](/shadow/integrate/gateway-mode/)).
4. If you used Presidio with custom recognizers, point Shadow's `[presidio]` section at the same Analyzer.
5. Validate with `shadow scan` and the per-request log.

## How do they compare on accuracy?

Detection accuracy mostly depends on the detector each tool is configured with. The GLiNER-PII model card reports its own F1 figures, and third-party results on other benchmarks differ. Shadow's [benchmark](/shadow/benchmarks/) compares leak rate, over-masking and round-trip exactness with published code and label mappings.

## Sources

- [NeMo Guardrails PII detection](https://docs.nvidia.com/nemo/guardrails/latest/configure-guardrails/guardrail-catalog/pii-detection)
- [NeMo Guardrails on GitHub](https://github.com/NVIDIA-NeMo/Guardrails)
- [nvidia/gliner-PII model card](https://huggingface.co/nvidia/gliner-PII)
- [Shadow tool calls and agents](/shadow/concepts/tool-calls/)

---

*Last reviewed: 2026-10-31. NVIDIA and NeMo are trademarks of their owner; this comparison is based on public documentation as of the review date.*
