---
title: "Enterprise LLM data protection: where Shadow fits next to DLP, SSE and AI gateways"
description: "How enterprises use Shadow to mask PII and secrets in AI API traffic at scale: endpoints, sidecars, existing DLP and gateways, and the gaps to plan for."
keywords:
  - enterprise llm data protection
  - enterprise ai dlp
  - pii masking at scale kubernetes
  - genai security architecture
  - reversible pseudonymization enterprise
  - open source alternative ai dlp
faq:
  - q: "Does Shadow replace our DLP or SSE platform?"
    a: "No. Enterprise DLP and SSE products cover browsers, SaaS and web chat, and mostly block, warn or coach. Shadow covers API traffic from endpoints, servers and containers, and translates rather than blocks. Most enterprises would run both."
  - q: "Can Shadow scale to thousands of devices?"
    a: "Each instance is independent and lightweight, so scale is a rollout question rather than a capacity one. What does not exist yet is a central console, policy server or fleet view; you distribute configuration with existing tooling."
  - q: "How does Shadow work in Kubernetes?"
    a: "As a sidecar container in each pod that calls models. The app reaches it on 127.0.0.1 in gateway mode, Shadow keeps its persona and vault on a persistent volume, and a NetworkPolicy can make the sidecar the only path to AI providers."
  - q: "What does a security review of Shadow look like?"
    a: "The source is Apache-2.0 and public. Reviewers usually look at the local CA and TLS handling (rustls, path-length-0 CA, only listed hosts decrypted), the encrypted vault (ChaCha20-Poly1305), loopback defaults, absence of telemetry, and the documented limitations."
  - q: "Is there commercial support?"
    a: "Shadow is a community open-source project. Plan support as you would for any internal open-source dependency: pin versions, build from source in your pipeline and track the repository's releases and security advisories."
sidebar:
  order: 5
---

Enterprises already have DLP, a secure web gateway or SSE platform, and often an AI gateway. Shadow fills a specific gap between them: API traffic to AI providers from developer machines, internal applications, agents and pipelines, where blocking breaks the work and placeholders break the answers. It replaces sensitive values with consistent, reversible fakes on the endpoint or next to the workload, so detected values reach the provider only as fakes and the tools keep working. The only real values sent by design are the recipient of a message or action the user asks for, and those a provider-hosted action needs, each one logged, and you can turn both off.

## Where Shadow fits in the stack

| Layer | Typical products | Covers | Shadow's role |
|---|---|---|---|
| Browser and SaaS DLP / SSE | Netskope, Zscaler, Palo Alto, Purview | Web chat apps, uploads, SaaS | Not a replacement. Shadow does not cover chatgpt.com or claude.ai |
| AI gateway | LiteLLM, Kong, Portkey, cloud gateways | Routing, keys, budgets, rate limits | Complementary. Chain them: app → Shadow → gateway |
| Endpoint API traffic | Few options | SDKs, CLIs, coding assistants, agents | **Primary use.** Proxy mode on laptops and build agents |
| Workload API traffic | Gateway guardrails (usually block or redact) | Internal apps, RAG, agents | **Primary use.** Sidecar or service in gateway mode |

See [LLM DLP without blocking](/shadow/use-cases/llm-data-loss-prevention-dlp/) and the [comparison pages](/shadow/compare/) for how specific products differ.

## What matters at enterprise scale

- **Coverage you can reason about.** Know which traffic goes through Shadow and which does not. `shadow hosts` lists exactly which hosts and routes are intercepted.
- **Consistency.** One reviewed configuration per population (developers, support, data science) and per workload.
- **Separation of duties.** Shadow's data directory holds the vault key and CA key. On servers, run it as a dedicated user; in Kubernetes, keep `vault.key` in a secret.
- **Egress control.** Make Shadow the only path to providers, so a misconfigured app fails instead of leaking.
- **Fail-safe behaviour.** If Shadow is down, proxied and gateway traffic fails rather than bypassing it. Optional detectors fail open; the built-ins always run.

## Reference architecture

| Population | Deployment | Configuration |
|---|---|---|
| Developer laptops | MDM per-user script, `shadow install --apply`, managed `env.sh` | Developer profile: `secret = "redact"`, internal domains, codenames |
| Build agents and CI | `shadow run` as a service on the runner, gateway URLs | Same as developers, plus `images = "block"` |
| Internal apps | Kubernetes sidecar per pod, `NetworkPolicy` egress | Per-app terms; `[policy.report]` for analytics workloads |
| Agent platforms | Sidecar; local tool execution preferred | `remote_tool_targets` set by risk owner |
| High-recall workloads | Presidio or the NER sidecar in the same pod | Loopback only |

### Kubernetes sidecar with egress control

```yaml
containers:
  - name: app
    image: registry.example.com/support-assistant:2.4.1
    env:
      - { name: OPENAI_BASE_URL, value: "http://127.0.0.1:8877/gw/openai/v1" }
  - name: shadow
    image: registry.example.com/shadow:<pinned-version>
    args: ["run", "--listen", "127.0.0.1:8877"]
    readinessProbe:
      exec: { command: ["shadow", "status", "--quiet"] }
    volumeMounts:
      - { name: shadow-data, mountPath: /data }
      - { name: shadow-config, mountPath: /data/shadow.toml, subPath: shadow.toml }
```

Containers in a pod share a network namespace, so a pod-level `NetworkPolicy` cannot tell the app from the sidecar. Enforce "only Shadow reaches providers" with an egress proxy: set `[proxy] upstream_proxy` on the sidecar and allow provider hosts only from that proxy. For several replicas that should share one identity, use a `ReadWriteMany` volume or provision `persona.json`, `vault.key` and `vault.bin` from a secret. Full details in [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/).

### Chaining with an AI gateway

```toml
[proxy.routes]
llmgw = "https://llm-gateway.acme.internal"
```

The app calls `http://127.0.0.1:8877/gw/llmgw/v1/...`. Shadow masks, the gateway handles keys, quotas and routing, and the provider receives the masked request. Your gateway's logs hold the same masked content, not the real values Shadow detected.

## Rollout checklist

- [ ] Map AI traffic: which populations and workloads call which providers, through which tools
- [ ] Decide what Shadow covers and what your DLP/SSE covers; document the boundary
- [ ] Build signed binaries and the container image from a pinned commit in your pipeline
- [ ] Write per-population configuration; review with privacy, security and data owners
- [ ] Pilot with a developer group and one production workload
- [ ] Run `shadow scan` on representative samples; record misses and tune terms or detectors
- [ ] Add egress controls so workloads cannot reach providers without Shadow
- [ ] Add health checks (`shadow status --quiet`) to MDM and orchestration
- [ ] Record the control, its limits and its owner in the risk register

## Gaps to plan for

:::note[Roadmap items, not features]
- **No central management plane.** No console, policy server, fleet inventory or RBAC. Configuration is a file distributed by your tooling.
- **No SIEM integration or audit log yet.** Today: per-request log lines with intent and counts, and a log entry for every `keep`. An optional local, append-only audit log is a [roadmap](/shadow/project/roadmap/) "Next" item; shipping it to your SIEM would be your own log pipeline.
- **No shared team identity yet.** Each instance has its own persona unless you distribute the files. Team identities are a "Later" item.
- **Per-host and per-tool policy** (different treatments per provider) is a "Later" item.
- **Not covered:** consumer web chat, mobile, WebSocket and Realtime APIs, gRPC, SigV4-signed Bedrock requests, certificate-pinning apps, text in images.
- **Detection quality** is best effort and should be measured on your data. See the [benchmarks](/shadow/benchmarks/).
:::

## Related

- [Regulated enterprise](/shadow/company-size/regulated-enterprise/)
- [Security model](/shadow/reference/security/)
- [Self-hosted AI gateway with PII masking](/shadow/use-cases/self-hosted-ai-gateway-with-pii-masking/)
- [Mask PII in LLM agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/)
- [Financial services](/shadow/industries/financial-services/) and [healthcare](/shadow/industries/healthcare/)
