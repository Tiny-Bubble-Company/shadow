---
title: "AI privacy for solo developers and freelancers: keep client data and keys out of LLMs"
seoTitle: "AI privacy for solo developers and freelancers · Shadow"
description: "Use Claude Code, Aider and LLM APIs on client work without sending client names, customer data or API keys. Free, local, five-minute setup."
keywords:
  - freelancer ai client confidentiality
  - claude code privacy
  - hide api keys from ai coding assistant
  - local pii masking for developers
  - ai nda client data
  - free llm privacy tool
faq:
  - q: "Can I use AI coding assistants on client code under an NDA?"
    a: "Check the NDA first: some forbid sharing client material with any third party, masked or not. Where AI use is allowed, Shadow reduces what the provider sees by replacing client names, customer data, hostnames, usernames and secrets with fakes, and restoring them in the answer."
  - q: "Which coding assistants does Shadow cover?"
    a: "Tools that call a model API with your own key: Claude Code, Codex CLI and other OpenAI-based CLIs, Aider, Continue and Cline, plus your own scripts and SDKs. GitHub Copilot and Cursor's hosted models go to their vendors' own backends, which are not built-in hosts."
  - q: "How do I keep different clients' data apart?"
    a: "Give each client its own data directory with SHADOW_HOME. Each directory has its own persona, vault and key. For most freelancers one persona with every client name added as a term is enough."
  - q: "Does Shadow slow down my assistant?"
    a: "Masking is local, in-process string work: about 8 ms per document on the benchmark set, which is small next to model latency. Streams are held back by at most the length of the longest fake."
  - q: "What does it cost?"
    a: "Nothing. Shadow is Apache-2.0 open source with no account, no subscription and no telemetry."
sidebar:
  order: 1
---

If you work alone, Shadow is a five-minute install that keeps your clients' names, their customers' data and your API keys out of the prompts your AI tools send. It runs on your laptop, costs nothing, needs no account, and restores the real values in every answer so your assistant keeps working. The main decision is which tools to route through it.

## What matters when you are on your own

| Concern | Why it matters to you | What Shadow does |
|---|---|---|
| Client confidentiality | NDAs and contracts often restrict sharing client material | Client names, people and codenames become consistent fakes |
| Customer data in code and logs | Debugging a client's app means pasting real emails and IPs | Built-in detectors replace them in prompts and tool results |
| Your own secrets | `.env` files and configs end up in context | Keys are replaced or, if you opt in, redacted |
| Machine identity | Paths like `/Users/jdoe/` and your git email go everywhere | Your username, hostname and git identity are replaced |
| Time and money | You have no IT team and no budget for DLP | Single binary, free, no service to run |

## Recommended setup

One laptop, proxy mode for coding assistants and terminal tools, gateway URLs for your own scripts.

### 1. Install and create your persona

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Rivera Code Studio"
```

`--org` protects your own business name. The install script uses a release binary where one exists and otherwise builds with `cargo`.

### 2. Add your clients and their vocabulary

```sh
shadow terms add "org:Globex Corporation" "org:Initech" "Project Falcon" "hostname:build01"
```

Or put them in `shadow.toml` in the data directory:

```toml
organization = ["Rivera Code Studio"]
terms = ["org:Globex Corporation", "org:Initech", "Project Falcon"]
internal_domains = ["globex.example", "initech.internal"]

[policy.always]
secret = "redact"      # API keys never leave, even as fakes
```

### 3. Cover your terminal and IDE

```sh
shadow install            # read the plan
shadow install --apply    # background service, trust the local CA, write env.sh
```

Add `source "<data dir>/env.sh"` to your shell profile. Claude Code, Aider and other API-based tools started from that shell now go through Shadow. For the Node-based CLIs, `env.sh` already sets `NODE_EXTRA_CA_CERTS`.

### 4. Point your own scripts at the gateway

```sh
export OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1
export ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic
```

### 5. Check it works

```sh
shadow status
shadow scan client-notes.md     # what would be masked, without sending anything
shadow vault stats              # how many values are protected, by kind
```

With `shadow run` in the foreground you also see one log line per request with the intent and the number of values masked.

## Example: debugging a client's app

You paste a stack trace into Claude Code:

```text
Before:  ConnectionError for jane.doe@globex.example at db01.globex.internal (10.20.4.17),
         OPENAI_API_KEY=sk-proj-AbC123... loaded from /Users/sam/clients/globex/.env

Sent:    ConnectionError for hui.zhao@lumeth.cn at db-kx4.lumeth.cn (10.84.211.9),
         OPENAI_API_KEY=[REDACTED_SECRET] loaded from /Users/yingchen/clients/lumeth/.env
```

The assistant's fix comes back with the real hostname and path. The fakes shown are illustrative; your persona generates its own.

## Keeping clients apart

For most freelancers one persona with all clients as terms is enough. If a client requires a stricter wall, give it its own data directory, persona and vault:

```sh
SHADOW_HOME=~/clients/globex shadow init --org "Rivera Code Studio"
SHADOW_HOME=~/clients/globex shadow run --listen 127.0.0.1:8879
```

Point that client's workspace at port 8879. The [consulting guide](/shadow/industries/consulting-professional-services/) shows the full pattern.

## Rollout checklist

- [ ] Read each client contract for AI or third-party restrictions
- [ ] `shadow init --org` with your business name
- [ ] Add client names, codenames and internal domains as terms
- [ ] Opt in to `secret = "redact"`
- [ ] `shadow install --apply` and source `env.sh`
- [ ] Run `shadow scan` on a real file from each client
- [ ] Back up `persona.json`, `vault.bin` and `vault.key` with your encrypted backups

## Honest limits

:::caution[What Shadow will not do for you]
- **Copilot and Cursor's hosted models** use their vendors' backends, not built-in hosts. Use an assistant that takes your own API key if you want it covered.
- **Code itself is not masked.** Shadow replaces identifiers, not logic. Proprietary algorithms still reach the model as written.
- **Passwords inside connection URLs** (`postgres://user:pw@host`) are not detected as secrets. Keep them in `password=`-style variables or out of context.
- **Images and screenshots** are not OCR'd.
:::

## Related

- [Protect secrets from AI coding assistants](/shadow/use-cases/protect-secrets-from-ai-coding-assistants/)
- [Anonymise prompts for Claude](/shadow/use-cases/anonymize-prompts-for-claude/)
- [Proxy mode](/shadow/integrate/proxy-mode/) and [gateway mode](/shadow/integrate/gateway-mode/)
- [Startups](/shadow/company-size/startups/), when you hire your first engineer
- [Benchmarks](/shadow/benchmarks/) and [comparisons](/shadow/compare/)
