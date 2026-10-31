---
title: "Secure LLM use for SaaS and technology companies: keep customer data out of AI prompts"
description: "Mask customer emails, IPs, hostnames and API keys in support, engineering and product AI traffic. Self-hosted proxy, gateway or sidecar; answers restored."
keywords:
  - saas customer data llm
  - soc 2 ai tools
  - mask pii in support tickets ai
  - llm privacy for product features
  - ai coding assistant secrets
  - customer data openai api
faq:
  - q: "How do we stop engineers sending customer data to AI coding assistants?"
    a: "Route API-based assistants such as Claude Code, Codex CLI, Aider, Continue or Cline through Shadow's proxy or gateway. Customer emails, IPs, internal hostnames, usernames in paths and secrets are replaced before code and logs leave the laptop. Assistants that use their vendor's own backend, such as GitHub Copilot, are not built-in hosts."
  - q: "Can we add an AI feature to our product without sending customer PII to OpenAI?"
    a: "Yes. Point the feature's model client at a Shadow sidecar or gateway. Customer values are masked in the request, restored in the response, including streams and tool calls, and the mapping stays in your infrastructure."
  - q: "Does Shadow help with SOC 2?"
    a: "It can be a documented control for data sent to AI subprocessors and supports confidentiality and privacy criteria. SOC 2 is an attestation of your controls by an auditor; Shadow does not provide a report or certification."
  - q: "What should we tell customers whose data might reach an AI provider?"
    a: "Describe the control accurately: identifiers such as names, emails, IPs and secrets are replaced with fakes on your infrastructure before requests reach the provider, and detection is best effort. Keep the AI provider on your subprocessor list, because content still reaches it."
  - q: "Will masking break code or logs the model needs to understand?"
    a: "Usually not, because fakes keep the format: an IP stays an IP, an email stays an email and a fake API key has the same shape. The code intent keeps identifiers readable, and the answer is restored with real values."
sidebar:
  order: 8
---

Shadow is for SaaS and technology companies whose support, engineering and product teams use LLMs on text that contains customer data. It runs on laptops, servers or as a sidecar, replaces customer names, emails, IPs, internal hostnames, usernames and secrets with realistic, format-preserving fakes, and restores them in the answer, including streamed responses and tool calls. Your customers' data stays on your infrastructure; the provider sees a coherent but fake world.

## What data leaks into AI tools at SaaS and tech companies

| Team | What ends up in the prompt |
|---|---|
| Support | Ticket threads with customer names, emails, phone numbers, account details |
| Engineering | Stack traces and logs with emails, IPs, tenant hostnames; configs with API keys and database URLs |
| SRE and security | Incident timelines, internal hostnames, IP ranges, usernames |
| Sales and success | Account names, contacts, renewal notes, usage exports |
| Product | AI features that send end-user content to a provider on every request |
| Data | Query results and CSV exports pasted for analysis |

For B2B companies this is also your customers' customers' data, covered by DPAs that list your AI provider as a subprocessor.

## How Shadow protects it

| Data | Treatment |
|---|---|
| Customer and end-user emails, names, phones | Built-in detectors with persona-consistent fakes |
| Customer company names | `org:` terms for key accounts; NER sidecar or Presidio for unknown ones |
| IPs, MACs, internal hostnames | `ip`, `mac` and `hostname` detectors; `internal_domains` covers subdomains, e.g. tenant hosts |
| Usernames in paths, OS user, git identity | `path` and `local_facts` detectors |
| API keys, tokens, private keys, passwords | `secret` detector; faked by default, redacted with `[policy.always] secret = "redact"` |
| Product and codenames | Dictionary `terms` |
| CSV exports and usage reports | The `report` intent: typed tokens for people and contact details |
| Request metadata | OpenAI `user`, Anthropic `metadata.user_id`, org and OS headers scrubbed |

```toml
organization = ["Acme Cloud Inc."]
terms = ["Project Falcon", "org:Globex", "org:Initech"]
internal_domains = ["acme.example", "acme.internal", "tenants.acme.example"]

[policy.always]
secret = "redact"

[policy.act]
remote_tool_targets = "keep"     # default; set "surrogate" if a failed hosted action is preferable

[files]
images = "block"                 # screenshots in tickets are not OCR'd
```

:::note[Which coding assistants are covered]
Shadow intercepts provider API hosts such as `api.openai.com` and `api.anthropic.com`. Claude Code, Codex CLI, Aider, Continue and Cline with your own key are covered through `HTTPS_PROXY` or a base URL. GitHub Copilot and Cursor's hosted models talk to their vendors' own backends, which are not on the built-in list. You can add hosts with `extra_hosts`, but those integrations are untested. See [protecting secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/).
:::

## Regulations and frameworks

| Framework | How Shadow helps |
|---|---|
| SOC 2 (confidentiality, privacy) | A documented, testable control for data sent to AI subprocessors. Evidence: config, per-request logs (intent and counts, never values), `shadow scan` results |
| GDPR Art. 28 processors, 5(1)(c), 25, 32 | Data protection by design for your processing and for subprocessor flows; pseudonymisation is still personal data for you (Recital 26) |
| Customer DPAs and security questionnaires | A clear answer to "what reaches your AI provider": self-hosted, open source, no telemetry |
| ISO/IEC 27001 and ISO/IEC 42001 | A technical control in your ISMS or AI management system |
| EU AI Act | Most SaaS AI features are not high-risk; if yours are (for example hiring or credit), see [HR and recruiting](/shadow/industries/hr-recruiting/) |

:::caution[Not legal advice]
This page explains what Shadow does. It is not an audit opinion or legal advice. Shadow does not make you SOC 2, ISO or GDPR compliant.
:::

## Example workflows

### Debugging with a coding assistant

Sent from Claude Code (fictional):

```text
ERROR auth: login failed for jane.doe@acme.example from 203.0.113.42
  at /Users/jdoe/src/acme-api/auth.rs:118
  db host=db01.acme.internal password=Pa55w0rd!2026
```

Seen by the provider (persona varies, `secret = "redact"` on):

```text
ERROR auth: login failed for hui.zhao@zhilan.cn from 198.51.100.17
  at /Users/huizhao/src/acme-api/auth.rs:118
  db host=zhilan-db-03.zhilan.cn password=[REDACTED_SECRET]
```

The fix comes back referring to your real hostname and path.

### Support macro drafting

An agent pastes a ticket thread and asks for a reply. The customer's name, email and company become fakes; the model writes naturally about them, and the reply is restored with the real customer before the agent sends it.

### Product AI feature

A "summarise this conversation" feature calls Anthropic through a Shadow sidecar. End-user names and emails are masked in each request, streamed responses are restored on the fly, and a fake split across two SSE events is reassembled before it is translated back.

## Deployment patterns

| Pattern | Use |
|---|---|
| **Endpoint** (`shadow install --apply`) | Engineering and support laptops; covers CLIs, scripts and IDE extensions that honour `HTTPS_PROXY` |
| **Gateway** | Internal tools: `OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1`, `ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic` |
| **Sidecar** | Product AI features in Kubernetes; `NetworkPolicy` so only the sidecar reaches provider hosts. See [Docker and Kubernetes](/shadow/integrate/docker-kubernetes/) |
| **CI runners** | `HTTPS_PROXY` on runners that call models for code review or test generation |

For several replicas presenting one identity, share the persona and vault through a `ReadWriteMany` volume or provision `persona.json`, `vault.key` and `vault.bin` from a secret.

## Related

- [Startups](/shadow/company-size/startups/), [mid-market](/shadow/company-size/mid-market/)
- [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/), [mask PII in LLM agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/)
- [Proxy mode](/shadow/integrate/proxy-mode/), [gateway mode](/shadow/integrate/gateway-mode/), [benchmarks](/shadow/benchmarks/), [compare](/shadow/compare/)
