---
title: "LLM data privacy for startups: protect customer data in AI tools and features"
description: "Keep customer PII and secrets out of OpenAI and Anthropic from day one: engineer laptops, product AI features and security questionnaires. Free and self-hosted."
keywords:
  - startup llm data privacy
  - soc 2 ai tools startup
  - pii masking for ai features
  - security questionnaire ai subprocessor
  - mask customer data openai startup
  - open source ai privacy layer
faq:
  - q: "Do startups need to worry about customer data in AI prompts?"
    a: "Yes, as soon as you have customers. Support replies, debugging sessions and AI product features all send customer data to your AI provider, and enterprise buyers ask about it in security reviews. Masking it early is cheaper than retrofitting it."
  - q: "Will Shadow help us pass SOC 2?"
    a: "It can be one documented control for data sent to AI subprocessors, which supports the confidentiality and privacy criteria. SOC 2 is an auditor's attestation of your whole control set; Shadow does not provide a report or certification."
  - q: "How do we add Shadow to our product's AI feature?"
    a: "Run Shadow as a sidecar or local service next to the backend and set the model client's base URL to the gateway, for example OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1. Requests are masked and responses, streams and tool calls restored."
  - q: "Does Shadow replace an AI gateway like LiteLLM or Portkey?"
    a: "No. Shadow masks and restores data; it does not do routing across models, budgets, rate limits or key management. You can chain them: your app calls Shadow, and Shadow forwards to the gateway through a custom route."
  - q: "What happens if Shadow is down?"
    a: "Requests fail rather than bypass it: gateway calls to 127.0.0.1:8877 are refused and proxied tools cannot connect. Use shadow status --quiet as a health check and run it as a service that restarts on failure."
sidebar:
  order: 2
---

For a startup, Shadow is a cheap way to keep the customer data it detects out of AI providers from the start: a free binary on each engineer's laptop and a sidecar next to any product feature that calls a model. It takes minutes per machine, needs no vendor contract, and gives you a concrete answer when an enterprise prospect asks what customer data reaches OpenAI or Anthropic.

## What matters at this stage

| Priority | What it means for AI privacy |
|---|---|
| Speed | Nothing that slows engineers or needs a procurement cycle |
| Product AI features | Customer content goes to a provider on every request, in production |
| First enterprise deals | Security questionnaires ask about AI subprocessors and data minimisation |
| SOC 2 readiness | You need controls you can describe and evidence you can show |
| Small team | Whatever you deploy has to run itself |

## Recommended deployment

| Where | Pattern | Why |
|---|---|---|
| Engineer laptops | `shadow install --apply`, `env.sh` in the shell profile | Covers Claude Code, Codex CLI, Aider and scripts in one step |
| Product backend | Sidecar or systemd service, gateway base URL | No code change; restores streams and tool calls |
| CI jobs that call models | `shadow run` in the job, gateway URLs | Test data and logs are masked too |
| Shared config | `shadow.toml` in your infra repo | Same terms and policy everywhere |

### Laptops

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme Labs Inc."
shadow install --apply
```

Keep a shared `shadow.toml` in your infra repo and copy it into each engineer's data directory (`~/Library/Application Support/shadow/` on macOS, `~/.local/share/shadow/` on Linux):

```sh
cp ~/src/infra/shadow/shadow.toml "$HOME/Library/Application Support/shadow/shadow.toml"
```

:::caution[Don't point SHADOW_CONFIG at a repo]
When `--config` or `SHADOW_CONFIG` names a file without a `data_dir` key, Shadow uses that file's folder as its data directory. Pointing it into a git checkout would put the vault key there. Copy the file into the data directory instead, or set an absolute `data_dir` in it.
:::

```toml
organization = ["Acme Labs Inc."]
terms = ["org:Globex", "org:Initech", "Project Falcon"]
internal_domains = ["acme.example", "acme.internal"]

[policy.always]
secret = "redact"
```

### The product feature

```yaml
services:
  api:
    image: acme-api:latest
    environment:
      OPENAI_BASE_URL: http://shadow:8877/gw/openai/v1
      ANTHROPIC_BASE_URL: http://shadow:8877/gw/anthropic
  shadow:
    image: shadow:local          # docker build -f packaging/Dockerfile -t shadow:local .
    volumes:
      - shadow-data:/data
      - ./shadow.toml:/data/shadow.toml:ro
volumes:
  shadow-data:
```

Keep the volume: it holds the persona and vault, so the same customer gets the same fake across requests. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) for the Kubernetes sidecar and replica options.

If your feature uses tools (sending emails, updating records), read [Mask PII in agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/) first: tools your code runs get real values back; tools the provider runs need a policy decision.

## What to tell customers and auditors

Describe the control precisely, because overclaiming shows up in diligence:

> "Requests from our application to AI providers pass through an open-source privacy layer on our infrastructure. Detected personal data, customer identifiers, internal hostnames and secrets are replaced with format-preserving pseudonyms before leaving, and restored in responses. The mapping is encrypted and never leaves our infrastructure. Detection is best effort and the provider remains a subprocessor."

Keep the AI provider on your subprocessor list. Shadow reduces what it receives; it does not remove it from the data flow.

## Rollout checklist

- [ ] Write down which tools and features call which providers
- [ ] Create a shared `shadow.toml` with your company, customers and internal domains
- [ ] Install on every engineer laptop; source `env.sh`
- [ ] Put the product feature behind a Shadow sidecar or service
- [ ] Add `shadow status --quiet` to health checks
- [ ] Run `shadow scan` on sample tickets, logs and prompts; add missed terms
- [ ] Decide `remote_tool_targets` if you use provider-hosted tools
- [ ] Note the control in your security policy and SOC 2 system description

## Gaps to know about

:::note[Not there yet]
- **No audit log export.** The proxy logs intent and counts per request; an optional local audit log is on the [roadmap](/shadow/project/roadmap/).
- **No published container image.** Build it from `packaging/Dockerfile` in CI and push to your own registry.
- **Detection is best effort.** Internal customer IDs and dates are not detected. See the [benchmarks](/shadow/benchmarks/).
- **Not a full AI gateway.** No auth, budgets or model routing. See [Self-hosted AI gateway with PII masking](/shadow/use-cases/self-hosted-ai-gateway-with-pii-masking/).
:::

## Related

- [SaaS and technology](/shadow/industries/saas-technology/)
- [Protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/)
- [Redact PII before the OpenAI API](/shadow/use-cases/redact-pii-before-sending-to-openai-api/)
- [Mid-market](/shadow/company-size/mid-market/), for when you outgrow a shared file in a repo
- [How Shadow compares](/shadow/compare/)
