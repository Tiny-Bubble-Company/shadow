---
title: "AI privacy use cases: mask PII and secrets in LLM prompts with Shadow"
description: "Step-by-step guides: stop PII leaks to ChatGPT, redact prompts for OpenAI and Claude, protect secrets from coding assistants, agents, RAG and documents."
keywords:
  - mask pii in llm prompts
  - redact pii before openai
  - anonymize prompts for claude
  - llm data loss prevention
  - pii masking proxy
  - reversible pseudonymization llm
faq:
  - q: "What problem does Shadow solve?"
    a: "It keeps the sensitive values it detects in prompts, files and tool results from reaching AI providers, without breaking the tools. Names, emails, IDs, card numbers, hostnames and secrets are replaced with realistic fakes before the request leaves and restored in the answer. When the goal is to write to or act on someone, that recipient is passed through as written so the model addresses the right person; recipients and the values a provider-hosted action needs are the only real values sent, and each one is logged."
  - q: "How is this different from redacting PII with placeholders?"
    a: "Placeholders such as <PERSON> destroy meaning, so answers get worse and actions cannot reach the real target. Shadow uses format-preserving fakes from one consistent persona and restores them in the response, including streams and tool calls. It uses typed tokens only where they work better, such as bulk reports."
  - q: "Do I need to change my code?"
    a: "Usually not. Point the SDK's base URL at Shadow's gateway, or set HTTPS_PROXY so the tool goes through Shadow's proxy. Rust applications can embed the library instead."
  - q: "Is Shadow free?"
    a: "Yes. It is open source under Apache-2.0, runs on your own machines and servers, and has no account, cloud service or telemetry."
  - q: "Where should I start?"
    a: "Run the five-minute quick start, then pick the guide below that matches your task. If you are rolling Shadow out to a team, read the company-size guides as well."
sidebar:
  label: Overview
  order: 0
---

These guides answer the questions people actually search for when they want to use LLMs without leaking data: how to stop customer data reaching ChatGPT, how to redact PII before the OpenAI API, how to keep API keys out of coding assistants. Each one starts with a direct answer, then gives the exact commands and configuration.

All of them use the same tool. Shadow is an open-source privacy layer that replaces sensitive values with realistic, reversible fakes before a request leaves your machine or server, and restores them in the answer.

## Stop data leaking to AI tools

| Guide | Answers |
|---|---|
| [How to stop employees pasting customer data into ChatGPT](/shadow/use-cases/stop-pii-leaks-to-chatgpt/) | Policy, blocking or translation; what works and what people route around |
| [LLM data loss prevention (DLP) without blocking](/shadow/use-cases/llm-data-loss-prevention-dlp/) | How Shadow differs from and complements DLP products |
| [Shadow AI governance](/shadow/use-cases/shadow-ai-governance/) | Managing unsanctioned AI tools by giving staff a safe approved path |
| [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/) | Tickets, macros, support agents and ticket analytics |

## Mask prompts for a specific provider or tool

| Guide | Answers |
|---|---|
| [Redact PII before sending prompts to the OpenAI API](/shadow/use-cases/redact-pii-before-sending-to-openai-api/) | One base-URL change for the OpenAI SDKs, Azure OpenAI and compatible APIs |
| [Anonymise prompts before sending them to Claude](/shadow/use-cases/anonymize-prompts-for-claude/) | The Anthropic SDK, Claude Code, Bedrock and Vertex |
| [Keep API keys and internal hosts out of AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/) | Claude Code, Codex CLI, Aider, Continue, Cline; what is and is not covered |
| [Self-hosted AI gateway with PII masking](/shadow/use-cases/self-hosted-ai-gateway-with-pii-masking/) | Gateway routes, custom upstreams and sidecars for internal apps |

## Build AI applications that handle personal data

| Guide | Answers |
|---|---|
| [Mask PII in LLM agents without breaking tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/) | Restoring tool-call arguments locally; provider-hosted tools |
| [Protect customer data in RAG pipelines](/shadow/use-cases/protect-customer-data-in-rag-pipelines/) | Where retrieval leaks, and where to put the masking |
| [Redact PDFs, Word and Excel files before uploading them to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/) | DOCX, XLSX and PPTX in place; PDF as text; images |
| [Reversible pseudonymisation for LLMs](/shadow/use-cases/reversible-pseudonymization-for-llms/) | Surrogates, tokens and redaction; the vault; restoring answers |

## Support regulatory requirements

| Guide | Answers |
|---|---|
| [Use LLMs in line with GDPR](/shadow/use-cases/gdpr-compliant-llm-usage/) | Data minimisation, pseudonymisation, data protection by design |
| [Use LLMs with patient data more safely under HIPAA](/shadow/use-cases/hipaa-safe-llm-usage/) | Minimum necessary, Safe Harbor identifiers, what Shadow does not cover |

For sector-specific guidance, see [Shadow by industry](/shadow/industries/).

## The building blocks every guide uses

| Need | Shadow feature | Reference |
|---|---|---|
| Cover AI API tools on a laptop | HTTPS proxy with a local CA; only AI hosts are decrypted | [Proxy mode](/shadow/integrate/proxy-mode/) |
| Cover an SDK or service | `http://127.0.0.1:8877/gw/<provider>/` base URL | [Gateway mode](/shadow/integrate/gateway-mode/) |
| Run next to an app in containers | Sidecar on `localhost` | [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |
| Protect your own vocabulary | `organization`, `terms`, `internal_domains`, `shadow terms add` | [Detection](/shadow/concepts/detection/) |
| Different handling per request | `act`, `report`, `code`, `converse` intents | [Intent-aware policy](/shadow/concepts/intent-policy/) |
| See what would be masked | `shadow scan <file>` | [CLI](/shadow/reference/cli/#shadow-scan) |

## Try it before you read further

```sh
shadow init --org "Acme GmbH"
echo "Summarise: jane.doe@acme.example at Acme GmbH disputes a charge on card 4111 1111 1111 1111" | shadow mask
```

The output contains a fake email, the persona's company name and a different card number that still passes the Luhn check. Pipe it into `shadow unmask` to get the original back. Nothing is sent anywhere. The full walkthrough is in the [quick start](/shadow/getting-started/quick-start/), and the [benchmarks](/shadow/benchmarks/) show how detection performs on public datasets.
