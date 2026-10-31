// Plain-Markdown summary of the home page for /llms-full.txt. Most of the home page is
// built from components in src/components/home/, which the Markdown export cannot read,
// so keep this in step with them (and with the benchmarks page for numbers).

export const HOME_SUMMARY = `## What Shadow does

Shadow is a free, open-source (Apache-2.0) privacy layer between your apps and AI providers. It replaces sensitive values in each outbound request with realistic, format-preserving fakes and restores the real values in the response. Install it with one command:

\`\`\`sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
\`\`\`

- Single binary for macOS, Linux and Windows; no cloud service, account or telemetry.
- Works with 15+ provider APIs, including OpenAI, Anthropic, Google Gemini, Vertex AI, Azure OpenAI, AWS Bedrock, Mistral, OpenRouter, Groq, DeepSeek, xAI and Cohere, plus self-hosted APIs.
- Covers apps that call AI provider APIs (such as api.openai.com and api.anthropic.com). It does not see what people type into chatgpt.com, claude.ai or other AI web apps in a browser.

## The problem: prompts carry far more than the question

Requests to AI providers carry names, emails and phone numbers; customer lists, invoices with IBANs and card numbers; API keys and tokens; internal hostnames, IPs and paths with usernames; company names and project codenames; and uploaded spreadsheets, documents and PDFs. Once sent, that content can end up in provider logs, abuse-monitoring queues, support tooling, training data or a future breach.

## How it works: translate, don't block

1. **Intercept.** Requests reach Shadow through \`HTTPS_PROXY\`, a gateway base URL or the Rust API. Only known AI hosts are decrypted; everything else is tunnelled untouched.
2. **Detect.** Checksummed patterns, emails, phones, secrets, hostnames, IPs and paths, your own dictionary of customers and codenames, and optional local NER or Presidio.
3. **Translate.** Each value gets a consistent, format-preserving fake from the machine's persona, or a typed token such as \`[PERSON_0001]\` for bulk reports. Secrets can be redacted outright.
4. **Restore.** Fakes in the answer become real values again, in full responses, streamed tokens and tool-call arguments. Whole values and a fake person's first or last name on their own are restored.

When you ask the model to write to or act on someone, that recipient is passed through as written and logged, so the model addresses the right person; everything else stays masked.

## Features

- Realistic, reversible fakes from one persistent persona.
- Encrypted local vault (ChaCha20-Poly1305) for the real-to-fake mapping, which never leaves the machine.
- Layered detection: checksummed patterns, secrets, machine facts, your dictionary, and optional NER or Presidio.
- Streams and tool calls: fakes split across SSE events are reassembled; tool-call arguments are restored before your tools run.
- Intent-aware policy: act, report, code or converse, each with the right treatment.
- Files: text, CSV, JSON, DOCX, XLSX, PPTX and PDF uploads are masked, and Office files stay valid.
- Activity dashboard, menu-bar app (Mac and Windows) and \`shadow top\` terminal view that record counts and categories, never values.

## Ways to run it

| Mode | For | Setup |
|---|---|---|
| Proxy | Every AI API client on a machine | \`HTTPS_PROXY=http://127.0.0.1:8877\` |
| Gateway | Anything with a base URL | \`OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1\` |
| Sidecar | Services in Docker or Kubernetes | \`localhost:8877\` next to your service |
| Library | Rust services | \`shadow-core\`, masking in-process |

## Benchmarks

On the held-out test set of the reproducible benchmark (https://getshadow.dev/benchmarks/), Shadow's built-in rules leaked 3.08% of sensitive values at about 8 ms per document, changed 0% of public names and round-tripped 100% of documents exactly. With the Presidio detector added, Shadow leaked 1.33%. Standalone tools leaked more: OpenAI Privacy Filter 11.01%, GLiNER multi PII 13.28%, Microsoft Presidio 20.74%, LLM Guard 29.26% and scrubadub 67.38%.

## Who it is for

Developers, security and privacy teams, and platform teams building LLM features: from solo developers and startups to enterprises and regulated industries such as healthcare, financial services, legal, insurance and the public sector.`;
