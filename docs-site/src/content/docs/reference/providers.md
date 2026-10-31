---
title: Supported providers and hosts
seoTitle: "Supported AI providers, hosts and API formats · Shadow"
description: "AI providers Shadow supports: OpenAI, Anthropic, Gemini, Azure OpenAI, Bedrock, Mistral and more. Intercepted hosts, gateway routes and API formats."
faq:
  - q: "Which AI providers does Shadow intercept?"
    a: "OpenAI, Anthropic, OpenRouter, Google Gemini and Vertex AI, Mistral, Groq, Together AI, DeepSeek, xAI, Azure OpenAI and Azure AI services, AWS Bedrock runtime, Cohere, Perplexity, Fireworks AI, Cerebras and Voyage AI. `shadow hosts` shows the effective list."
  - q: "Can I add a self-hosted or internal LLM endpoint?"
    a: "Yes. Add hosts with `[proxy] extra_hosts` and gateway routes under `[proxy.routes]`. Request bodies in any JSON shape are masked."
  - q: "Does Shadow work with AWS Bedrock?"
    a: "Yes with Bedrock API keys (bearer), which are masked normally. Requests signed with AWS SigV4 are forwarded unmasked with a warning, because changing the body would break the signature."
  - q: "Are WebSocket and Realtime APIs supported?"
    a: "No. WebSocket, Realtime and gRPC traffic is not parsed and passes through unmasked."
---

## Intercepted hosts (proxy mode)

In proxy mode, Shadow terminates TLS only for these hosts. Every other host is tunnelled without decryption. `*` matches any run of characters; matching is case-insensitive and exact otherwise (`openai.azure.com.evil.com` does **not** match `*.openai.azure.com`).

| Provider | Host pattern |
|---|---|
| OpenAI | `api.openai.com` |
| Anthropic | `api.anthropic.com` |
| OpenRouter | `openrouter.ai` |
| Google Gemini API | `generativelanguage.googleapis.com` |
| Google Vertex AI | `aiplatform.googleapis.com`, `*-aiplatform.googleapis.com` |
| Mistral | `api.mistral.ai` |
| Groq | `api.groq.com` |
| Together AI | `api.together.xyz`, `api.together.ai` |
| DeepSeek | `api.deepseek.com` |
| xAI | `api.x.ai` |
| Azure OpenAI | `*.openai.azure.com` |
| Azure AI services | `*.services.ai.azure.com`, `*.cognitiveservices.azure.com` |
| AWS Bedrock runtime | `bedrock-runtime.*.amazonaws.com` |
| Cohere | `api.cohere.com`, `api.cohere.ai` |
| Perplexity | `api.perplexity.ai` |
| Fireworks AI | `api.fireworks.ai` |
| Cerebras | `api.cerebras.ai` |
| Voyage AI | `api.voyageai.com` |

Add hosts with `[proxy] extra_hosts` and exclude hosts with `bypass_hosts`. `shadow hosts` shows the effective list. The [PAC file](/shadow/integrate/proxy-mode/#system-wide-with-a-pac-file) at `/proxy.pac` is generated from the same list.

## Gateway routes

| Route | Upstream |
|---|---|
| `openai` | `https://api.openai.com` |
| `anthropic` | `https://api.anthropic.com` |
| `openrouter` | `https://openrouter.ai/api` |
| `gemini` | `https://generativelanguage.googleapis.com` |
| `google` | `https://generativelanguage.googleapis.com` |
| `mistral` | `https://api.mistral.ai` |
| `groq` | `https://api.groq.com/openai` |
| `together` | `https://api.together.xyz` |
| `deepseek` | `https://api.deepseek.com` |
| `xai` | `https://api.x.ai` |
| `cohere` | `https://api.cohere.com` |
| `perplexity` | `https://api.perplexity.ai` |
| `fireworks` | `https://api.fireworks.ai/inference` |
| `cerebras` | `https://api.cerebras.ai` |

Azure OpenAI, Bedrock, Vertex AI and self-hosted endpoints have per-account URLs, so add them under `[proxy.routes]`. See [Gateway mode](/shadow/integrate/gateway-mode/#custom-routes).

## API formats

Request bodies in any JSON shape are masked. These formats get additional handling:

| Format | Request | Streaming restore with split-fake reassembly | Identifier scrubbing |
|---|---|---|---|
| OpenAI Chat Completions | ✓ | ✓ (content and tool-call arguments) | `user`, `safety_identifier` |
| OpenAI Responses | ✓ | ✓ | `user`, `safety_identifier` |
| Anthropic Messages | ✓ | ✓ (text, thinking and tool-input JSON deltas) | `metadata.user_id` |
| Google Gemini `generateContent` / `streamGenerateContent` | ✓ | ✓ | |
| Cohere Chat | ✓ | ✓ | |
| Ollama-style NDJSON streams | ✓ | ✓ | |
| Other OpenAI-compatible APIs (Groq, Mistral, Together, DeepSeek, xAI, OpenRouter, Fireworks, Cerebras, Perplexity) | ✓ | ✓ (OpenAI format) | `user` |

Compressed (`gzip`, `deflate`) request bodies are decoded before masking. [File uploads](/shadow/concepts/file-masking/) are masked for OpenAI, Anthropic and Gemini payload shapes.

## Not supported

| | Why |
|---|---|
| WebSocket and Realtime APIs | Not parsed; traffic is passed through |
| gRPC | Not parsed |
| AWS SigV4-signed requests (Bedrock with IAM) | Modifying the body breaks the signature. Forwarded unmasked with a warning. Bedrock API keys (bearer) are masked normally |
| Apps with certificate pinning | They reject the local CA in proxy mode. Use gateway mode |
