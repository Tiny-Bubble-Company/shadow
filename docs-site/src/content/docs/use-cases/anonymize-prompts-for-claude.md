---
title: "How to anonymise prompts before sending them to Claude"
description: "Mask personal data, customer names and secrets in Anthropic Claude API requests locally, and restore them in streamed answers and tool use."
keywords:
  - anonymize prompts claude
  - claude api pii masking
  - anthropic data privacy
  - redact pii before claude
  - claude code privacy
  - mask data anthropic sdk
faq:
  - q: "How do I route the Anthropic SDK through Shadow?"
    a: "Set the base URL to http://127.0.0.1:8877/gw/anthropic, either in code or with ANTHROPIC_BASE_URL. The SDK appends /v1/messages itself. Your x-api-key header passes through unchanged."
  - q: "Can I use Shadow with Claude Code?"
    a: "Yes. Claude Code talks to api.anthropic.com, so set ANTHROPIC_BASE_URL to the Shadow gateway or run it with HTTPS_PROXY pointing at Shadow and the local CA trusted through NODE_EXTRA_CA_CERTS."
  - q: "Does it work with Claude on Amazon Bedrock or Google Vertex AI?"
    a: "Vertex AI hosts are intercepted in proxy mode. Bedrock works with API-key (bearer) auth; requests signed with AWS SigV4 are forwarded unmasked with a warning, because changing the body would break the signature."
  - q: "Can I use Shadow with the claude.ai website or desktop app?"
    a: "Not as a supported integration. Shadow intercepts api.anthropic.com, not claude.ai. Use an API-backed client or Claude Code through Shadow instead."
  - q: "Is anonymised text still personal data?"
    a: "For you, yes. Shadow pseudonymises reversibly and you hold the mapping, so under GDPR the data remains personal data in your hands. It reduces what Anthropic receives; it does not take the data out of scope for you."
sidebar:
  order: 3
---

Put Shadow between your code and `api.anthropic.com` by setting `ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic`. Shadow replaces names, emails, phone numbers, customer and company names, IDs, internal hostnames and secrets with realistic fakes before the request leaves, then restores them in Claude's answer, including streamed text, thinking and `tool_use` input. Strictly speaking this is reversible pseudonymisation rather than anonymisation, which is what keeps the answers useful.

## Why anonymise prompts for Claude?

Claude is often handed long, rich context: entire ticket threads, contracts, repositories, meeting transcripts. That is where personal data and confidential names pile up. Anthropic's commercial terms and retention settings govern what happens to it, but the simplest protection is not to send it.

Placeholder redaction (`[NAME]`) undermines long-context work: Claude can't keep five people apart across a 40-page document if they are all `[NAME]`. Shadow gives each real value a distinct, consistent fake from one [persona](/shadow/concepts/personas/), so Claude reasons about a coherent fake world and you read the answer about the real one.

## Which Claude workflows benefit most?

| Workflow | What Shadow replaces |
|---|---|
| Long-document analysis (contracts, transcripts, ticket exports) | People, companies, emails, phones, addresses, IBANs, national IDs; DOCX, XLSX and PPTX attachments masked in place |
| Claude Code and other coding agents | API keys, internal hostnames, IPs, home-directory usernames, git identity |
| Agents with `tool_use` | Values in prompts and tool results; real values restored in tool input |
| Batch summarisation and reporting | Bulk personal data becomes typed tokens such as `[PERSON_0001]` |
| Prompts naming customers and codenames | Your own `terms` and `organization` entries |

## How to set it up

### 1. Install and create a persona

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme Ltd"
shadow identity          # the fake identity Claude will see
```

### 2. Start Shadow

```sh
shadow run               # or: shadow install --apply for a background service
```

### 3. Point the Anthropic SDK at the gateway

Python:

```python
import anthropic

client = anthropic.Anthropic(base_url="http://127.0.0.1:8877/gw/anthropic")

msg = client.messages.create(
    model="claude-sonnet-4-5",
    max_tokens=512,
    messages=[{"role": "user", "content": "Write a short update to Jane Doe at Globex about Project Falcon"}],
)
print(msg.content[0].text)    # mentions Jane Doe, Globex and Project Falcon
```

Shadow restores whole fakes, and a fake person's first or last name on its own, so a greeting by first name comes back right. Other fragments, such as the domain of a fake email, stay fake, so ask for full values when the text goes to a real person.

TypeScript:

```ts
import Anthropic from "@anthropic-ai/sdk";
const client = new Anthropic({ baseURL: "http://127.0.0.1:8877/gw/anthropic" });
```

Or for any tool built on the official SDKs:

```sh
export ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic
```

The base URL stops at the route: the SDK appends `/v1/messages` itself.

### 4. Claude Code and other CLIs

Claude Code calls `api.anthropic.com`, so either of these works:

```sh
# Gateway: no certificate needed
ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic claude

# Proxy: covers everything the process sends to known AI hosts
source "$HOME/Library/Application Support/shadow/env.sh"   # written by shadow install --apply
claude
```

The proxy route needs the local CA trusted; `env.sh` sets `NODE_EXTRA_CA_CERTS` for Node-based tools. For coding work, also read [protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/).

### 5. Teach Shadow your names

```toml
organization = ["Acme Ltd"]
terms = ["org:Globex", "Project Falcon", "person:Jane Doe"]
internal_domains = ["acme.example"]
```

Or `shadow terms add "org:Globex" "Project Falcon"`. Check a sample with `shadow scan sample.txt`.

## Claude on Bedrock and Vertex AI

| Platform | Status |
|---|---|
| Anthropic API (`api.anthropic.com`) | Gateway route `anthropic` and proxy mode |
| Google Vertex AI (`aiplatform.googleapis.com`, `*-aiplatform.googleapis.com`) | Intercepted in proxy mode; add a custom route for gateway mode |
| AWS Bedrock runtime with an API key (bearer) | Masked normally; add a `bedrock` route under `[proxy.routes]` |
| AWS Bedrock with SigV4 (IAM) signing | **Forwarded unmasked with a warning.** Modifying a signed body breaks the signature |

```toml
[proxy.routes]
bedrock = "https://bedrock-runtime.eu-central-1.amazonaws.com"
```

## Example: before and after

What you send (fictional data):

```text
Summarise this escalation for the account team. Jane Doe (jane.doe@globex.example)
says the Project Falcon rollout at Globex broke SSO on idp.acme.internal.
```

What Claude sees with this machine's persona (yours will differ):

```text
Summarise this escalation for the account team. Ying Chen (ying.chen@lanxin.cn)
says the Velora Quilent rollout at Lanxin Trading broke SSO on xinghe-srv-11.
```

The summary you receive names Jane Doe, Globex, Project Falcon and `idp.acme.internal` again. Streamed responses are restored as they arrive, even when a fake is split across two events.

## Tool use and agents

When Claude answers with a `tool_use` block, its `input` contains the fakes it saw. Shadow restores the real values before your code runs the tool, including when the input JSON is streamed in pieces. Tools hosted by the provider are handled by [policy](/shadow/concepts/intent-policy/): by default the action's target is kept so the action works, and the keep is logged. Details in [mask PII in agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/).

## Limits

- `claude.ai` and the Claude desktop and mobile apps are not intercepted.
- Images in prompts are not OCR'd; set `[files] images = "block"` if screenshots may contain personal data. PDFs in `document` blocks are sent as masked text by default.
- Dates are not masked, and detection is best effort. Unique context can still identify a person or company.

## Related

- [Redact PII before sending to the OpenAI API](/shadow/use-cases/redact-pii-before-sending-to-openai-api/)
- [GDPR-compliant LLM usage](/shadow/use-cases/gdpr-compliant-llm-usage/)
- [Gateway mode](/shadow/integrate/gateway-mode/), [Proxy mode](/shadow/integrate/proxy-mode/), [Supported providers](/shadow/reference/providers/)
- [Benchmarks](/shadow/benchmarks/) and [how Shadow compares](/shadow/compare/)
