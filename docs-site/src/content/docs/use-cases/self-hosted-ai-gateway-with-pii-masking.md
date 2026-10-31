---
title: "Self-hosted AI gateway with PII masking: put Shadow in front of any LLM API"
description: "Run a self-hosted, open-source gateway that masks PII and secrets before requests reach OpenAI, Anthropic, Azure OpenAI, Bedrock or vLLM, and restores answers."
keywords:
  - self hosted ai gateway pii
  - llm gateway pii masking
  - open source llm proxy redaction
  - openai proxy anonymize
  - litellm pii masking alternative
  - azure openai pii gateway
faq:
  - q: "Is Shadow a replacement for LiteLLM, Kong or Portkey?"
    a: "No. Shadow is a privacy layer, not a full AI gateway: it has no virtual keys, user authentication, rate limits, budgets or model fallback. It does one thing those gateways mostly do as a block-or-redact add-on, reversible masking, and it can sit in front of them."
  - q: "Does the gateway need a TLS certificate?"
    a: "No. In gateway mode your app talks plain HTTP to Shadow on loopback or inside the pod, and Shadow connects to the provider over TLS. Only proxy mode needs the local CA."
  - q: "Can I route Azure OpenAI, AWS Bedrock or a self-hosted model through Shadow?"
    a: "Yes. Add a custom route under [proxy.routes] pointing at your Azure resource, Bedrock runtime endpoint or vLLM/TGI server. Bedrock requests signed with SigV4 cannot be modified and are forwarded unmasked with a warning; Bedrock API keys work."
  - q: "Can I expose the Shadow gateway to the whole network?"
    a: "It is not designed for that. Anyone who can reach the port can use your persona and get responses unmasked with your vault. Keep it on loopback or inside the pod as a sidecar, one instance per app."
  - q: "Does Shadow handle streaming through the gateway?"
    a: "Yes. SSE and NDJSON streams are restored on the fly, including fakes split across events and streamed tool-call arguments."
sidebar:
  order: 10
---

To self-host a gateway that masks PII before it reaches an LLM provider, run Shadow next to your application and change the app's base URL to `http://127.0.0.1:8877/gw/<provider>/...`. Shadow masks names, emails, IDs, hostnames and secrets in each request with reversible fakes, forwards it over TLS, and restores real values in the response, including streams and tool calls. It is a single Apache-2.0 Rust binary with no account and no telemetry.

## How does the Shadow gateway work?

```text
http://127.0.0.1:8877/gw/<route>/<path>   →   <route base>/<path>
```

`/gw/openai/v1/chat/completions` goes to `https://api.openai.com/v1/chat/completions`. Query strings are kept, and your API key travels in the usual header and is passed through. On the way out, Shadow classifies the request's intent, detects sensitive values, replaces them with fakes from the machine's [persona](/shadow/concepts/personas/) and scrubs identifying fields such as OpenAI `user` and Anthropic `metadata.user_id`. On the way back, it restores every fake.

Built-in routes: `openai`, `anthropic`, `openrouter`, `gemini` (or `google`), `mistral`, `groq`, `together`, `deepseek`, `xai`, `cohere`, `perplexity`, `fireworks`, `cerebras`. See [gateway mode](/shadow/integrate/gateway-mode/) for SDK examples.

## How to set up a self-hosted PII-masking gateway

### 1. Install and initialise

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme GmbH"
```

On a server, use the hardened systemd unit described in [installation](/shadow/getting-started/installation/#run-as-a-background-service). In containers, build the image (a published image is on the [roadmap](/shadow/project/roadmap/)):

```sh
docker build -f packaging/Dockerfile -t shadow:local .
```

### 2. Add routes for your own endpoints

Azure OpenAI, Bedrock and self-hosted servers have per-account URLs:

```toml
[proxy.routes]
azure   = "https://my-resource.openai.azure.com"
bedrock = "https://bedrock-runtime.eu-central-1.amazonaws.com"
vllm    = "https://vllm.internal.example.com"
```

`shadow hosts` lists the routes in effect. An Azure call then looks like:

```text
http://127.0.0.1:8877/gw/azure/openai/deployments/gpt-4o/chat/completions?api-version=2024-10-21
```

Any JSON request shape is masked: Shadow masks every string value except protocol keys such as `model`, `role` and `type`, so internal APIs work without a plugin.

### 3. Point the application at Shadow

```sh
export OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1
export ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic
```

Most frameworks (LangChain, LlamaIndex, Vercel AI SDK, the OpenAI and Anthropic SDKs) accept a base URL. No code change beyond that.

### 4. Run it as a sidecar

One Shadow per application pod, on loopback:

```yaml
containers:
  - name: app
    image: my-app:latest
    env:
      - { name: OPENAI_BASE_URL, value: "http://127.0.0.1:8877/gw/openai/v1" }
  - name: shadow
    image: registry.example.com/shadow:0.1.0
    args: ["run", "--listen", "127.0.0.1:8877"]
    volumeMounts:
      - { name: shadow-data, mountPath: /data }
    readinessProbe:
      exec: { command: ["shadow", "status", "--quiet"] }
```

Mount a persistent volume at `/data` so fakes stay stable across restarts, and add a `NetworkPolicy` so only the sidecar can reach provider hosts. Full example: [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/).

### 5. Tune the policy and verify

```toml
[policy.always]
secret = "redact"        # opt-in; secrets are surrogates by default

[files]
images = "block"         # no OCR yet
```

```sh
shadow status            # proxy health, persona, vault, CA fingerprint
curl -s http://127.0.0.1:8877/healthz
```

With `shadow run` in the foreground, each request logs its intent and how many values got each treatment, never the values.

:::caution[Keep the gateway on loopback]
Anyone who can reach the gateway can talk to providers through your persona and have responses unmasked with your vault. Shadow warns when it listens on a non-loopback address. In containers, keep it inside the pod or publish the port only to `127.0.0.1`.
:::

## Shadow versus a full AI gateway

Shadow is deliberately narrow. Compared with general-purpose gateways:

| | Typical AI gateway (LiteLLM, Kong AI Gateway, Portkey) | Shadow |
|---|---|---|
| Routing, fallback, load balancing | Yes | Fixed routes only |
| Virtual keys, auth, rate limits, budgets, cost tracking | Yes | No |
| PII handling | Guardrail add-ons that mostly block or redact with placeholders; restoring is limited or plan-dependent | Realistic, format-preserving, reversible fakes from a persona |
| Streaming restoration | Varies; some buffer or have known gaps | SSE and NDJSON, including split fakes |
| Tool-call arguments | Usually not restored | Restored locally before your tools run |
| Where it runs | Central service | Next to each app: loopback, sidecar or in-process |

They combine well. Put Shadow next to the app and route it to your existing gateway, so the central gateway keeps doing keys, quotas and observability but sees prompts with detected values already masked:

```toml
[proxy.routes]
litellm = "https://litellm.internal.example.com"
```

```text
app ──▶ Shadow (sidecar) ──/gw/litellm/v1/...──▶ LiteLLM ──▶ OpenAI, Anthropic, Bedrock
```

A side benefit: gateway logs, traces and caches now hold fakes instead of customer data.

## Limitations

- **AWS SigV4.** IAM-signed Bedrock requests cannot be modified without breaking the signature, so they are forwarded unmasked with a warning. Use Bedrock API keys through Shadow.
- **WebSocket, Realtime and gRPC** are not parsed.
- **Detection is best effort.** Add terms, enable the [NER sidecar or Presidio](/shadow/concepts/detection/), and check with `shadow scan`.
- **Several replicas, one identity.** Share a `ReadWriteMany` volume or provision the same `persona.json`, `vault.key` and `vault.bin` from a secret. A shared team identity with key distribution is a roadmap item.

## Related

- [Gateway mode](/shadow/integrate/gateway-mode/), [integrating any endpoint](/shadow/integrate/any-endpoint/), [supported providers](/shadow/reference/providers/)
- [Redact PII before sending to the OpenAI API](/shadow/use-cases/redact-pii-before-sending-to-openai-api/)
- [LLM data loss prevention](/shadow/use-cases/llm-data-loss-prevention-dlp/) and [SaaS and technology companies](/shadow/industries/saas-technology/)
- [How Shadow compares](/shadow/compare/) and [benchmarks](/shadow/benchmarks/)
