---
title: Supported providers and hosts
seoTitle: "Supported AI providers, hosts and API formats · Shadow"
description: "AI services Shadow masks: OpenAI, Anthropic, Gemini, Bedrock, 20+ inference clouds, and ChatGPT, Claude.ai and Gemini in a browser. Hosts and formats."
faq:
  - q: "Which AI providers does Shadow intercept?"
    a: "Model APIs (OpenAI, Anthropic, Google Gemini and Vertex AI, Mistral, DeepSeek, xAI, Cohere, Perplexity and more), inference clouds (Azure OpenAI, AWS Bedrock, OpenRouter, Groq, Together, Fireworks, Replicate, Hugging Face, NVIDIA NIM and more) and six chat apps in a browser: ChatGPT, Claude.ai, Gemini, Perplexity, DeepSeek and Grok. `shadow hosts` shows the effective list."
  - q: "Does Shadow mask ChatGPT and Claude.ai in the browser?"
    a: "Yes, with the system proxy pointed at Shadow (`shadow install --system-proxy --apply`). It masks the prompt you send and restores your values in the answer and the conversation history. Microsoft Copilot, Le Chat, Meta AI and Poe are not supported yet."
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
| Replicate | `api.replicate.com` |
| Hugging Face | `api-inference.huggingface.co`, `router.huggingface.co`, `*.endpoints.huggingface.cloud` |
| DeepInfra | `api.deepinfra.com` |
| Hyperbolic | `api.hyperbolic.xyz` |
| SambaNova | `api.sambanova.ai` |
| Novita AI | `api.novita.ai` |
| Lambda | `api.lambda.ai` |
| Baseten | `inference.baseten.co`, `*.api.baseten.co` |
| NVIDIA NIM | `integrate.api.nvidia.com` |
| Nebius AI Studio | `api.studio.nebius.com` |
| SiliconFlow | `api.siliconflow.cn` |
| Cloudflare AI Gateway | `gateway.ai.cloudflare.com` |
| AI21 | `api.ai21.com` |
| Reka | `api.reka.ai` |
| Moonshot AI | `api.moonshot.ai`, `api.moonshot.cn` |
| Alibaba Qwen (DashScope) | `dashscope.aliyuncs.com`, `dashscope-intl.aliyuncs.com` |
| Zhipu AI | `open.bigmodel.cn` |
| MiniMax | `api.minimax.io` |
| Meta Llama API | `api.llama.com` |

### AI chat apps

With `[proxy] chat_apps = true` (the default), these chat apps are intercepted too. Only their prompt requests are masked, and only the prompt text; their answers and reloaded history are restored. They are reached through the [system proxy](/shadow/integrate/proxy-mode/#system-wide-with-the-system-proxy).

| App | Host | Prompt requests masked |
|---|---|---|
| ChatGPT | `chatgpt.com`, `chat.openai.com` | `POST /backend-api/conversation`, `/backend-api/f/conversation…` (and `backend-anon`) |
| Claude.ai | `claude.ai` | `POST /api/organizations/…/chat_conversations/…/completion` (and `retry_completion`) |
| Gemini | `gemini.google.com` | `POST …/StreamGenerate` (`f.req` form field) |
| Perplexity | `perplexity.ai`, `www.perplexity.ai` | `POST /rest/sse/perplexity_ask` |
| DeepSeek | `chat.deepseek.com` | `POST /api/v0/chat/completion` |
| Grok | `grok.com` | `POST /rest/app-chat/conversations/…` |

These are the vendors' private web formats and can change without notice; a request in a shape Shadow does not recognise is forwarded unchanged. The dashboard also names Microsoft Copilot, Le Chat, Meta AI and Poe, which are not masked yet.

Add hosts with `[proxy] extra_hosts` and exclude hosts with `bypass_hosts`. `shadow hosts` shows the effective list. The [PAC file](/shadow/integrate/proxy-mode/#only-ai-hosts-with-a-pac-file) at `/proxy.pac` is generated from the same list; `/proxy.pac?scope=all` is the [system proxy](/shadow/integrate/proxy-mode/#system-wide-with-the-system-proxy) file that sends everything to Shadow.

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
| `huggingface` | `https://router.huggingface.co` |
| `deepinfra` | `https://api.deepinfra.com/v1/openai` |
| `nvidia` | `https://integrate.api.nvidia.com` |
| `sambanova` | `https://api.sambanova.ai` |
| `qwen` | `https://dashscope-intl.aliyuncs.com/compatible-mode` |
| `moonshot` | `https://api.moonshot.ai` |

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
| ChatGPT web (JSON-patch event stream) | ✓ (prompt parts) | ✓ | timezone set to the persona's |
| Claude.ai web | ✓ (prompt, attachments) | ✓ | timezone set to the persona's |
| Gemini web (`f.req` form) | ✓ (prompt) | ✓ (whole answer) | |
| Perplexity, DeepSeek and Grok web | ✓ (question or prompt) | ✓ | |
| Other OpenAI-compatible APIs (Groq, Mistral, Together, DeepSeek, xAI, OpenRouter, Fireworks, Cerebras, Perplexity, DeepInfra, Hugging Face, NVIDIA NIM, Qwen, Moonshot and more) | ✓ | ✓ (OpenAI format) | `user` |

Compressed (`gzip`, `deflate`) request bodies are decoded before masking. [File uploads](/shadow/concepts/file-masking/) are masked for OpenAI, Anthropic and Gemini payload shapes.

## Not supported

| | Why |
|---|---|
| WebSocket and Realtime APIs | Not parsed; traffic is passed through. This includes voice and live features of chat apps |
| Microsoft Copilot, Le Chat, Meta AI, Poe | Not masked yet; recognised by name only |
| AI apps on phones | Not on the machine Shadow runs on |
| gRPC | Not parsed |
| AWS SigV4-signed requests (Bedrock with IAM) | Modifying the body breaks the signature. Forwarded unmasked with a warning. Bedrock API keys (bearer) are masked normally |
| Apps with certificate pinning | They reject the local CA in proxy mode. Use gateway mode |
