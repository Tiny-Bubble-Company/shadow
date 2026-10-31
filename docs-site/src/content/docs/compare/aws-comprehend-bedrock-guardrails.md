---
title: "Shadow vs AWS Comprehend and Bedrock Guardrails — a self-hosted alternative"
description: Shadow vs AWS Comprehend PII and Amazon Bedrock Guardrails. AWS detects, blocks or tags PII; Shadow masks with reversible fakes and restores answers.
sidebar:
  label: AWS Comprehend / Bedrock Guardrails
  order: 10
keywords:
  - bedrock guardrails alternative
  - aws comprehend pii alternative
  - bedrock guardrails vs shadow
  - bedrock guardrails pii
  - aws comprehend pricing
  - bedrock guardrails pricing
  - PII redaction for LLM
  - open source AI DLP
faq:
  - q: "Is Shadow an alternative to Amazon Bedrock Guardrails for PII?"
    a: "For the PII part, yes. Bedrock Guardrails can block a request or anonymise PII with type tags such as {NAME}, and does not restore the original values. Shadow replaces values with realistic, reversible fakes and restores them in the response. Bedrock Guardrails also covers content filters, denied topics and grounding checks, which Shadow does not."
  - q: "Can Bedrock Guardrails restore masked PII in the model's response?"
    a: "Public AWS documentation describes BLOCK and ANONYMIZE actions for sensitive information filters. It does not describe restoring anonymised values in the response, so the user sees tags like {NAME} rather than the real name."
  - q: "Does Shadow work with Amazon Bedrock?"
    a: "Yes when Bedrock is called with bearer API keys. Requests signed with AWS SigV4 (IAM) cannot be modified without breaking the signature, so Shadow forwards them unmasked with a warning."
  - q: "How much does AWS Comprehend PII detection cost?"
    a: "Comprehend is billed per 100-character unit with a minimum per request; a third-party pricing guide lists about $0.0001 per unit. Check the AWS pricing page for current rates. Shadow is free and runs locally."
  - q: "Which is more accurate, AWS Comprehend or Shadow?"
    a: "Comprehend scored well in a vendor-run benchmark by Tonic. Results depend heavily on the dataset and label mapping. Comprehend is not in the Shadow benchmark, because commercial cloud APIs are impractical to run publicly and reproducibly."
---

**AWS offers two PII services relevant to LLMs: Amazon Comprehend detects and redacts PII in text, and Amazon Bedrock Guardrails can block or anonymise PII in prompts and responses for Bedrock models. Shadow is an open-source privacy layer that replaces sensitive values with realistic, reversible fakes before a request reaches any provider, and restores them in the answer.** If you run everything on Bedrock and want managed guardrails, AWS is the natural fit. If you call several providers, need the real values back in responses, or want data masked before it leaves your machine, Shadow covers that.

:::note[TL;DR]
Comprehend is a cloud PII detector; Bedrock Guardrails blocks or replaces PII with type tags like `{NAME}` and does not restore them. Shadow masks with consistent fakes on your own machine and restores values in responses, streams and tool calls, across many providers. Bedrock Guardrails also handles content safety, which Shadow does not.
:::

## At a glance

| | Shadow | AWS Comprehend / Bedrock Guardrails |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | No, proprietary AWS services |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | AWS-managed APIs; Guardrails attach to Bedrock calls or the `ApplyGuardrail` API |
| **Protection method** | Reversible realistic fakes; typed tokens for reports; optional redaction | Comprehend: detect and redact. Guardrails: BLOCK or ANONYMIZE with type tags, plus custom regex |
| **Restores responses** | Yes, automatically | No |
| **Streaming** | Yes, SSE and NDJSON | Guardrails streaming behaviour not verified here; Comprehend is not a stream processor |
| **Tool calls** | Yes, restored locally; intent-aware | No PII-specific tool-call handling documented |
| **Files** | DOCX, XLSX, PPTX in place; PDF to text | Comprehend async batch jobs on documents in S3 |
| **Self-hosted** | Yes | No |
| **Price model** | Free | Usage-based (per text unit) |

## What AWS does well

- **Managed and integrated.** No servers to run. Guardrails attach directly to Bedrock model invocations and agents, with IAM, CloudWatch and CloudTrail around them.
- **More than PII.** Bedrock Guardrails also offers content filters, denied topics, word filters and contextual grounding checks. Shadow only handles sensitive values.
- **Strong generic detection.** In Tonic's vendor-run benchmark (October 2025), Comprehend scored highest of the generic detectors tested, ahead of several open-source options. Treat vendor benchmarks with care, but it is a strong signal.
- **Batch processing.** Comprehend runs asynchronous PII jobs over documents stored in S3, which suits dataset preparation.
- **Custom patterns.** Guardrails accept custom regex for organisation-specific identifiers; AWS documents regex filters as not charged separately (per a third-party pricing guide).

## Where Shadow is different

### Does Bedrock Guardrails give the real values back?

No. ANONYMIZE replaces values with type tags, so a prompt about Jane Doe reaches the model as `{NAME}`, and the answer mentions `{NAME}` too. That is safe, but the user gets a less useful answer and has to fill in the blanks. Shadow sends `Hui Zhao` instead, from a persistent [persona](/shadow/concepts/personas/), and turns it back into `Jane Doe` before your app sees the response.

### Realistic fakes keep the model working

A Shadow fake email is a valid email at a consistent fake domain; a fake card number passes Luhn; an IBAN passes mod-97. The model reasons about a coherent fake world. Type tags lose the distinction between two people of the same type and break tasks like drafting an email or filling a table.

### Works with many providers, not only Bedrock

Shadow masks traffic to OpenAI, Anthropic, Gemini, Azure OpenAI, Mistral, OpenRouter and more, as a [proxy](/shadow/integrate/proxy-mode/) or [gateway](/shadow/integrate/gateway-mode/). Bedrock Guardrails protect Bedrock calls (and text you pass to `ApplyGuardrail`).

### Data never reaches a third-party detector

With Comprehend, the plaintext you want to protect is sent to an AWS API to be scanned. Shadow detects on the machine that sends the request. For organisations that already trust AWS with the data, this matters less.

### Streams, tool calls and intent

Shadow restores fakes split across SSE events and inside streamed tool-call arguments, and its [intent policy](/shadow/concepts/intent-policy/) chooses fakes, typed tokens like `[PERSON_0007]`, or keeps real the recipient of a message and what a provider-hosted action needs (each keep is logged). See [tool calls](/shadow/concepts/tool-calls/).

### Files in prompts

Shadow masks DOCX, XLSX and PPTX uploads in place, and base64 or multipart files in requests. See [file masking](/shadow/concepts/file-masking/).

## A limitation to know about: SigV4

Shadow cannot mask Bedrock requests signed with AWS SigV4, because changing the body breaks the signature. It forwards them unmasked with a warning. Bedrock calls authenticated with bearer API keys are masked normally. If your Bedrock usage is IAM-signed, Guardrails are the practical option for that traffic.

## When should you choose AWS?

- Your AI workloads run on Bedrock and you want a managed, IAM-governed guardrail.
- You also need content filters, denied topics or grounding checks.
- You need batch PII detection on documents in S3.
- Your requests are SigV4-signed.
- Placeholder output is acceptable for your use case.

## When should you choose Shadow?

- You call several model providers, or providers outside AWS.
- You need the real values back in responses, streams and agent tool calls.
- You want sensitive data masked before it leaves the machine, not scanned by a cloud API.
- You want a free, open-source tool you can audit and run anywhere.

## Can you use them together?

Yes. Shadow can mask requests to Bedrock (with API keys) before they arrive, and a Bedrock Guardrail can still apply content filters and denied topics to the masked request. For SigV4-signed traffic, keep Guardrails as the PII control.

## Migrating from Bedrock Guardrails PII filters

1. Keep your guardrail for content filters and topics; consider turning off PII ANONYMIZE once Shadow masks the traffic, so the model sees fakes rather than tags.
2. Switch Bedrock clients to API-key authentication, or add a [custom route](/shadow/integrate/gateway-mode/#custom-routes) for your Bedrock endpoint.
3. Port custom regex patterns to Shadow [dictionary terms](/shadow/concepts/detection/#dictionary-terms) or a custom detector.
4. Run `shadow scan` on representative prompts before switching traffic.

## How do they compare on accuracy?

Comprehend performed well in Tonic's vendor-run benchmark, and the language support is ambiguous in AWS's own documentation (the API reference lists 12 language codes while other AWS pages say English). Shadow's [benchmark](/shadow/benchmarks/) measures leak rate, over-masking and round-trip exactness with published code and mappings; commercial cloud APIs such as Comprehend are not included, because their terms and pricing make public, reproducible runs impractical.

## Sources

- [Amazon Comprehend DetectPiiEntities API reference](https://docs.aws.amazon.com/comprehend/latest/APIReference/API_DetectPiiEntities.html)
- [Amazon Bedrock Guardrails components](https://docs.aws.amazon.com/bedrock/latest/userguide/guardrails-components.html)
- [Amazon Comprehend pricing](https://aws.amazon.com/comprehend/pricing/)
- [Amazon Bedrock pricing](https://aws.amazon.com/bedrock/pricing/)
- [Tonic Textual benchmark (vendor-run)](https://www.tonic.ai/ai-model-benchmarks/textual-benchmark)
- [Shadow supported providers](/shadow/reference/providers/)

---

*Last reviewed: 2026-10-31. AWS, Amazon Comprehend and Amazon Bedrock are trademarks of their owner; this comparison is based on public documentation as of the review date.*
