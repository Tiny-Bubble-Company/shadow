---
title: "How to keep API keys and internal hosts out of AI coding assistants"
description: "Stop Claude Code, Aider, Cline and other AI coding tools sending API keys, internal hostnames, IPs and usernames to the model. Set up Shadow in minutes."
keywords:
  - ai coding assistant secrets leak
  - claude code api key privacy
  - prevent secrets in llm prompts
  - mask internal hostnames ai
  - copilot cursor data privacy
  - redact secrets before llm
faq:
  - q: "Which coding assistants does Shadow work with?"
    a: "Any tool that calls a supported provider API with your key and honours a base URL or HTTPS_PROXY: Claude Code, OpenAI-API-based CLIs, Aider, Continue and Cline with your own keys, and your own scripts. GitHub Copilot and Cursor's hosted models send traffic to their vendors' own backends, which are not built-in intercepted hosts."
  - q: "Are secrets redacted or replaced with fakes?"
    a: "By default a secret becomes a fake of the same shape, so code and configs stay coherent and the real key is restored locally if the model quotes it back. For coding work we recommend [policy.always] secret = \"redact\", which sends [REDACTED_SECRET] and never the key in any form."
  - q: "Does masking make code suggestions worse?"
    a: "Rarely. Requests with code are classified as code intent, identifiers stay readable, and only sensitive values such as keys, hostnames, IPs, emails and home-directory usernames are replaced."
  - q: "Will Shadow catch a password in a database URL?"
    a: "Not reliably. The secret detector finds known key formats, JWTs, PEM private keys, bearer tokens and password= style assignments, but not a password embedded in a connection URL such as postgres://user:pw@host. Keep such URLs in environment variables, or add the hostname to internal_domains so at least the host is masked."
  - q: "Can I bypass Shadow for a single command?"
    a: "Yes. Unset the proxy for that command with env -u HTTPS_PROXY, or drop the base URL variable. Shadow never falls back to a direct connection: if it is down, proxied tools fail to connect. Hosts that are not on the intercepted list, and anything in NO_PROXY, go direct by design."
sidebar:
  order: 4
---

Coding assistants read your working tree, your terminal output and whatever you paste, which means `.env` values, cloud keys, internal hostnames, IP addresses and your home-directory username go to the model provider. Run Shadow locally and point the assistant at it with `ANTHROPIC_BASE_URL`, `OPENAI_BASE_URL` or `HTTPS_PROXY`; Shadow replaces those values with fakes (or redacts secrets outright) and restores them in the reply. It works with any assistant that calls a supported provider API with your own key.

## What leaks from a developer's machine?

| Source | Example of what reaches the provider |
|---|---|
| Config files the agent opens | `STRIPE_SECRET_KEY=sk_live_…`, `AWS_SECRET_ACCESS_KEY=…`, `password = "…"` |
| Stack traces and logs | `ConnectionError: db01.prod.acme.internal:5432`, `10.20.4.17`, customer emails |
| File paths | `/Users/jdoe/src/billing/...` (your username) |
| Git metadata | `user.name`, `user.email` |
| Pasted snippets | JWTs, bearer tokens, private keys |
| Request metadata | SDK OS/arch headers, `metadata.user_id` |

Most of these are things you would never type into a chat on purpose. They leak because agents read files for context.

## Which assistants are covered?

| Assistant | How it connects | Covered by Shadow |
|---|---|---|
| Claude Code | `api.anthropic.com` | Yes: `ANTHROPIC_BASE_URL` or `HTTPS_PROXY` |
| CLIs and scripts using an OpenAI API key | `api.openai.com` | Yes: `OPENAI_BASE_URL` or `HTTPS_PROXY` |
| Aider, Continue, Cline and similar with your own key | The provider you configure | Yes, through a base URL setting or the proxy |
| VS Code / JetBrains extensions calling provider APIs | Provider API | Yes, through the IDE proxy setting and the trusted CA |
| GitHub Copilot | GitHub's own backend | Not built in |
| Cursor (hosted models) | Cursor's own backend | Not built in |

Shadow intercepts the [provider API hosts](/shadow/reference/providers/). Tools that route through their vendor's backend are not on that list. You can add other hosts with `[proxy] extra_hosts`, and Shadow masks string values in any JSON body it intercepts, but that is untested for these products; check the per-request log if you try it.

## How to set it up

### 1. Install and initialise

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme Ltd"
```

Shadow discovers your OS user, hostname and git identity at startup (`local_facts`) and maps them to the persona, so they are masked without configuration.

### 2. Configure for code

Edit `shadow.toml` in the data directory (`~/Library/Application Support/shadow` on macOS, `~/.local/share/shadow` on Linux):

```toml
organization = ["Acme Ltd"]
internal_domains = ["acme.internal", "acme.example", "corp.acme.example"]
terms = ["hostname:build01", "hostname:db01", "Project Falcon"]

[policy.always]
secret = "redact"        # keys, tokens and private keys leave as [REDACTED_SECRET]
```

With redaction on, the model can't echo a key back, so you won't get it restored in an answer either. That is usually what you want for code.

### 3. Scan before you trust it

Run a scan on the files your assistant is likely to read. Nothing is changed or sent:

```sh
shadow scan .env
shadow scan config/production.toml --reveal
```

If a value is missing from the output, Shadow will not mask it. Typical gaps are passwords inside connection URLs (`postgres://user:pw@host`) and unusual in-house token formats. Move those to variables the agent does not need to read, or add the host to `internal_domains`.

### 4. Point the assistant at Shadow

Gateway mode, no certificate needed:

```sh
shadow run &
export ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic
export OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1
claude
```

Proxy mode, which covers every tool in the shell that calls a supported provider API, including ones without a base URL setting:

```sh
shadow install --apply                     # background service, CA trust, env.sh
source "$HOME/Library/Application Support/shadow/env.sh"
```

`env.sh` sets `HTTPS_PROXY`, `NO_PROXY`, and the trust variables Node and Python tools need (`NODE_EXTRA_CA_CERTS`, `REQUESTS_CA_BUNDLE`, `SSL_CERT_FILE`, `CURL_CA_BUNDLE`). Add the `source` line to `~/.zshrc` or `~/.bashrc`. For IDE extensions, set the IDE's HTTP proxy to `http://127.0.0.1:8877`.

### 5. Watch the log

With `shadow run` in the foreground you see one line per request: the intent (usually `code`) and how many values were masked by kind. Values are never logged.

## Example: debugging a failing service

What the assistant sends (fictional data):

```text
Why does this fail?
  File "/Users/jdoe/src/billing/db.py", line 12
  OperationalError: could not connect to db01.prod.acme.internal (10.20.4.17)
  config: STRIPE_SECRET_KEY=sk_live_51Hx...  owner=jane.doe@acme.example
```

What the provider sees (persona values differ per machine):

```text
Why does this fail?
  File "/Users/yingchen/src/billing/db.py", line 12
  OperationalError: could not connect to lanxin-db-02.zhilan.internal (10.88.31.204)
  config: STRIPE_SECRET_KEY=[REDACTED_SECRET]  owner=ying.chen@xinghe.cn
```

The model's answer refers to `db01.prod.acme.internal` and your real path when it reaches you. Because `Acme Ltd` is configured as the organisation, the bare name `Acme` is protected wherever it appears, including in repository and package names.

## Code intent: why answers stay good

Shadow classifies requests that contain source code, diffs, stack traces or configuration as `code`. Identifiers, function names and structure pass through untouched; only detected values are replaced. A fake hostname is still a hostname, a fake IP is still a valid IP, and a surrogate key keeps the shape of the original, so generated code and configs remain coherent. See [intent-aware policy](/shadow/concepts/intent-policy/).

## Limits

- Copilot and Cursor's hosted backends are not covered (see the table).
- Detection is best effort, and proprietary code is not "masked": Shadow hides values, not your algorithms. If source code itself is the secret, decide which repositories an assistant may read at all.
- Apps that pin certificates reject the local CA in proxy mode. Use a base URL where possible.

## Related

- [Anonymise prompts for Claude](/shadow/use-cases/anonymize-prompts-for-claude/)
- [Mask PII in LLM agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/)
- [SaaS and technology](/shadow/industries/saas-technology/), [solo developers and freelancers](/shadow/company-size/solo-developers-freelancers/)
- [Proxy mode](/shadow/integrate/proxy-mode/), [Detection](/shadow/concepts/detection/), [Security model](/shadow/reference/security/)
- [Benchmarks](/shadow/benchmarks/) and [how Shadow compares](/shadow/compare/)
