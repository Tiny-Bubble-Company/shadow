---
title: "How to redact PII before sending prompts to the OpenAI API"
seoTitle: "How to redact PII before sending prompts to the OpenAI API"
description: "Mask names, emails, cards and secrets in OpenAI API requests with one base URL change, and get real values back in responses, streams and tool calls."
keywords:
  - redact pii before openai api
  - mask pii openai python
  - openai pii filter
  - anonymize data before sending to gpt
  - openai api data privacy
  - pii redaction proxy openai
faq:
  - q: "Do I need to change my code to mask PII for the OpenAI API?"
    a: "Usually only the base URL. Point the OpenAI SDK at http://127.0.0.1:8877/gw/openai/v1, or set OPENAI_BASE_URL, and Shadow masks the sensitive values it detects in every request and restores them in every response. Your API key passes through unchanged."
  - q: "Is this redaction or pseudonymisation?"
    a: "By default it is reversible pseudonymisation: values become realistic fakes that are restored in the response. You can opt in to irreversible redaction per kind, for example secrets, with [policy.always] secret = \"redact\"."
  - q: "Does it work with streaming and the Responses API?"
    a: "Yes. Chat Completions and Responses API streams are restored on the fly, including fakes split across server-sent events and streamed tool-call arguments."
  - q: "Does it work with Azure OpenAI?"
    a: "Yes. Add a custom gateway route for your Azure resource under [proxy.routes], or use proxy mode, where *.openai.azure.com is intercepted by default."
  - q: "How much latency does masking add?"
    a: "Built-in masking is local, in-process string work: about 8 ms per document on the benchmark set, small next to model latency. Optional detectors such as Presidio or the NER sidecar add a local HTTP call each."
sidebar:
  order: 2
---

Run Shadow next to your code and change one thing: the OpenAI base URL. Shadow detects names, emails, phone numbers, addresses, card numbers, IBANs, national IDs, IPs, internal hostnames and secrets in every request body, replaces them with realistic fakes, and restores the real values in the response, including streams and tool calls. It runs locally or as a sidecar, so nothing goes to a third-party redaction service.

## Why redact PII before calling the OpenAI API?

Even with zero data retention or an enterprise agreement, prompts are processed and may be logged for abuse monitoring by a third party. Data protection rules (GDPR data minimisation, HIPAA minimum necessary, contractual limits on customer data) push you to send only what the task needs. For most prompts the model does not need the real email address to write an email.

Plain redaction (`<PERSON>`, `***`) breaks answers: the model can't tell two customers apart or address a reply. Shadow uses **format-preserving fakes** instead, so the prompt still reads naturally:

| You send | OpenAI sees |
|---|---|
| `jane.doe@acme.example` | `hui.zhao@zhilan.cn` |
| `Acme Ltd` | `Zhilan Technology Co., Ltd.` |
| `4111 1111 1111 1111` | `4837 2191 9471 3480` (valid Luhn) |
| `DE89 3704 0044 0532 0130 00` | another German IBAN (valid mod-97) |
| `sk-proj-…` | a fake key of the same shape, or `[REDACTED_SECRET]` |

The mapping stays in an [encrypted local vault](/shadow/concepts/vault/).

## How to set it up

### 1. Install and initialise

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme Ltd"
```

### 2. Try masking offline

```sh
echo "Refund jane.doe@acme.example, card 4111 1111 1111 1111" | shadow mask
echo "Refund jane.doe@acme.example, card 4111 1111 1111 1111" | shadow mask | shadow unmask
```

The first command prints the masked text; the second gets the original back. Nothing is sent.

### 3. Run Shadow

```sh
shadow run            # foreground on 127.0.0.1:8877
```

For a server, use the systemd unit or `shadow install --apply`; for containers, a [sidecar](/shadow/integrate/docker-kubernetes/).

### 4. Point the SDK at the gateway

Python:

```python
from openai import OpenAI

client = OpenAI(base_url="http://127.0.0.1:8877/gw/openai/v1")  # key from OPENAI_API_KEY

reply = client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[{"role": "user", "content": "Summarise this refund ticket from jane.doe@acme.example"}],
)
print(reply.choices[0].message.content)   # mentions jane.doe@acme.example; OpenAI never saw it
```

Node.js:

```ts
import OpenAI from "openai";
const client = new OpenAI({ baseURL: "http://127.0.0.1:8877/gw/openai/v1" });
```

Or without touching code, since the official SDKs read it from the environment:

```sh
export OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1
```

The same works for LangChain, LlamaIndex, the Vercel AI SDK and anything else that accepts an OpenAI base URL. Tools that can't set one can use [proxy mode](/shadow/integrate/proxy-mode/) instead.

### 5. Azure OpenAI

Add a route for your resource:

```toml
[proxy.routes]
azure = "https://my-resource.openai.azure.com"
```

Requests to `http://127.0.0.1:8877/gw/azure/openai/deployments/gpt-4o/chat/completions?api-version=2024-10-21` then go to Azure, masked.

### 6. Tune what gets masked

```toml
organization = ["Acme Ltd"]
terms = ["org:Globex", "Project Falcon", "person:Tom Reyes"]
internal_domains = ["acme.example", "acme.internal"]
allow_terms = ["Jane Austen"]           # never masked

[policy.always]
secret = "redact"                       # irreversible for secrets

[policy.report]
person = "token"                        # bulk data in reports becomes [PERSON_0001]
email = "token"
default = "surrogate"
```

Check coverage on a sample with `shadow scan sample.json --reveal`. For better recall on free-text names, enable the [open-weight NER sidecar or Presidio](/shadow/concepts/detection/).

## What about streaming, tools and files?

| Feature | Handling |
|---|---|
| Streaming (`stream: true`) | Restored on the fly; a fake split across events is reassembled. Hold-back is at most the length of the longest fake |
| Chat Completions `tool_calls`, Responses function calls | Arguments restored before your code sees them. See [agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/) |
| `POST /v1/files`, `input_file` base64 | Text, CSV, JSON, DOCX, XLSX, PPTX masked; PDF text extracted and sent masked. See [file masking](/shadow/concepts/file-masking/) |
| `user`, `safety_identifier` fields | Replaced with persona values |
| Embeddings | Inputs are masked like any JSON string; see [RAG pipelines](/shadow/use-cases/protect-customer-data-in-rag-pipelines/) before relying on it |
| Realtime API (WebSocket) | Not processed |

## Example: before and after

Request your code sends (fictional data):

```json
{"model": "gpt-4o-mini", "messages": [{"role": "user",
  "content": "Summarise: Jane Doe (jane.doe@acme.example, +1 415 555 0134) disputes invoice 7741 for Acme Ltd."}]}
```

What OpenAI receives:

```json
{"model": "gpt-4o-mini", "messages": [{"role": "user",
  "content": "Summarise: Hui Zhao (hui.zhao@zhilan.cn, +86 28 6512 0417) disputes invoice 7741 for Zhilan Technology Co., Ltd."}]}
```

What your code gets back: a summary about Jane Doe and Acme Ltd. With `shadow run` in the foreground, the log shows the intent and how many values were masked, never the values.

## Limits

- Detection is best effort. Internal reference formats (invoice, ticket, account numbers) are not detected by the built-ins; add known values as terms, or write a custom detector if you [embed Shadow in Rust](/shadow/integrate/rust/#custom-detectors).
- Dates are not masked. Images are not OCR'd.
- Masking hides content, not your account: the API key still identifies you to OpenAI.

## Related

- [Anonymise prompts for Claude](/shadow/use-cases/anonymize-prompts-for-claude/)
- [Reversible pseudonymisation for LLMs](/shadow/use-cases/reversible-pseudonymization-for-llms/)
- [Self-hosted AI gateway with PII masking](/shadow/use-cases/self-hosted-ai-gateway-with-pii-masking/)
- [Gateway mode](/shadow/integrate/gateway-mode/), [Configuration](/shadow/reference/configuration/), [Supported providers](/shadow/reference/providers/)
- [Benchmarks](/shadow/benchmarks/) and [how Shadow compares](/shadow/compare/)
