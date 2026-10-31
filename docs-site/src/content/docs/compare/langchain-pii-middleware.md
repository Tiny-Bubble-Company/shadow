---
title: "Shadow vs LangChain PIIMiddleware — reversible PII masking for agents"
seoTitle: "Shadow vs LangChain PIIMiddleware: reversible PII masking"
description: Shadow vs LangChain PIIMiddleware for agent PII protection. LangChain redacts, masks or hashes one way; Shadow restores values in tool calls and streams.
sidebar:
  label: LangChain PIIMiddleware
  order: 19
keywords:
  - langchain pii middleware alternative
  - langchain PIIMiddleware vs shadow
  - langchain pii redaction
  - langgraph pii masking
  - agent PII protection
  - PII redaction for LLM
  - reversible pseudonymization LLM
faq:
  - q: "What is LangChain PIIMiddleware?"
    a: "An agent middleware in LangChain that detects PII and applies a strategy (redact, mask, hash or block) to the agent's input, output and tool results. It is MIT-licensed and runs inside your LangChain application."
  - q: "Is Shadow an alternative to LangChain PIIMiddleware?"
    a: "Yes, if you need the originals back. PIIMiddleware's strategies are one-way, so the agent's final answer and tool calls see redacted or hashed values. Shadow masks with reversible fakes and restores them in responses and tool-call arguments before your tools run."
  - q: "Does Shadow work with LangChain and LangGraph?"
    a: "Yes. Point the model client's base URL at Shadow's gateway, or set HTTPS_PROXY. Shadow masks every request the agent makes and restores tool-call arguments, so your LangChain tools receive real values."
  - q: "Can PIIMiddleware restore PII in tool calls?"
    a: "Public LangChain documentation does not describe restoring values. A tool that needs a real email address would receive the redacted, masked or hashed form."
  - q: "Is LangChain PIIMiddleware free?"
    a: "Yes. It is part of the MIT-licensed LangChain library. Shadow is open source under Apache-2.0."
---

**LangChain's PIIMiddleware is an in-application agent middleware that redacts, masks, hashes or blocks PII in an agent's input, output and tool results. Shadow is an open-source privacy layer for AI traffic that replaces sensitive values with realistic, reversible fakes before a request leaves the machine, and restores them in responses and tool calls.** PIIMiddleware is the quickest way to add one-way PII handling to a LangChain agent. Shadow is the choice when the agent must still act on real data, such as emailing the right person.

:::note[TL;DR]
PIIMiddleware runs inside LangChain and is one way: redact, mask, hash or block. Shadow runs outside the app, works with any framework or language, uses reversible realistic fakes, and restores values in streams and tool-call arguments so tools act on the real data.
:::

## At a glance

| | Shadow | LangChain PIIMiddleware |
|---|---|---|
| **Open source / licence** | Yes, Apache-2.0 | Yes, MIT |
| **Deployment** | Endpoint HTTPS proxy, gateway (`/gw/<provider>/`), Rust library | Python middleware inside a LangChain agent |
| **Protection method** | Reversible realistic fakes; typed tokens for reports; optional redaction | Redact, mask, hash, block |
| **Restores responses** | Yes, automatically | No |
| **Streaming** | Yes, SSE and NDJSON with split-fake reassembly | n/a (no restoration) |
| **Tool calls** | Yes, arguments restored locally; intent-aware | Yes, can apply to tool results (one way) |
| **Files** | DOCX, XLSX, PPTX in place; PDF to text | No |
| **Self-hosted** | Yes | Yes (in your app) |
| **Price model** | Free | Free |

## What PIIMiddleware does well

- **Zero extra infrastructure.** It is a few lines in your agent definition; no proxy, sidecar or certificate.
- **Agent-aware placement.** It can apply to the user input, the model output and, notably, tool results, which is where a lot of PII enters an agent (database rows, CRM records, emails).
- **Clear strategies.** Redact, mask, hash or block per PII type, so you can block secrets and mask emails in the same agent.
- **Custom detectors.** Built-in regex detectors cover common types, and you can add your own.
- **Native to LangChain.** It fits naturally into LangChain and LangGraph agent code, tracing and testing.

## Where Shadow is different

### Can the agent still act on real data?

With a one-way strategy, the model sees `[REDACTED_EMAIL]` or a hash, so a tool call it writes contains that too. Your `send_email` tool then gets a value it can't use. Shadow passes the recipient the instruction names through as written (and logs it), so the model writes `send_email(to="jane@acme.com")` and greets the right person; everything else in the request gets a realistic fake that Shadow restores in the arguments before your tool runs. With `[policy.act] targets = "surrogate"` the recipient is a fake too, such as `hui.zhao@zhilan.cn`, restored to `jane@acme.com` locally, and the provider never sees the address. See [tool calls and agents](/shadow/concepts/tool-calls/).

### Provider-hosted tools

Some tools run in the provider's cloud, where nothing can restore a fake. Shadow's [intent policy](/shadow/concepts/intent-policy/) handles this explicitly: by default it keeps real only what the action needs (the contact details, people, companies and addresses named in the instruction) while secrets, cards and IDs stay masked, and logs each keep; or you can choose to send a fake and accept the action failing.

### Streaming restoration

Shadow restores fakes that arrive split across streamed events, including streamed tool-call argument deltas. See [streaming hold-back](/shadow/concepts/how-it-works/#streaming-hold-back).

### Framework- and language-independent

PIIMiddleware protects LangChain agents. Shadow protects any client that talks to a [supported model API](/shadow/reference/providers/) through it: other frameworks, other languages, IDE assistants, CLIs and desktop apps. See [integration modes](/shadow/integrate/overview/).

### Consistent fakes and files

Shadow's [persona](/shadow/concepts/personas/) keeps fakes consistent across sessions, so long-running agents and memory stay coherent. [File masking](/shadow/concepts/file-masking/) covers DOCX, XLSX and PPTX uploads.

### Detection breadth

Shadow adds checksum-validated financial IDs, secrets in many key formats, local machine facts, a dictionary for your own codenames, and optional NER or Presidio. See [detection](/shadow/concepts/detection/).

## When should you choose PIIMiddleware?

- Your agent is built on LangChain and one-way handling is acceptable.
- You want to block or hash specific PII types in tool results with no extra infrastructure.
- You need a quick guard for a prototype.

## When should you choose Shadow?

- Agents must act on real people, addresses and IDs through tools.
- You use several frameworks, languages or off-the-shelf tools.
- You stream responses and tool calls.
- You want consistent fakes across sessions and Office uploads masked.

## Can you use them together?

Yes. A common split is Shadow for everything that leaves the machine, and PIIMiddleware with the `block` strategy for types you never want in a tool result at all. Be aware that if both mask the same values, the model sees PIIMiddleware's redaction rather than Shadow's fakes, and those values can't be restored.

## Migrating from PIIMiddleware

1. Point your chat model's base URL at Shadow, for example `http://127.0.0.1:8877/gw/openai/v1` ([gateway mode](/shadow/integrate/gateway-mode/)), or set `HTTPS_PROXY` ([proxy mode](/shadow/integrate/proxy-mode/)).
2. Remove PIIMiddleware, or keep it only for strategies you want to stay one way (`block`).
3. Map hash or redact choices to Shadow's `[policy.always]`, for example `secret = "redact"`.
4. Add your organisation's names and codenames as [dictionary terms](/shadow/concepts/detection/#dictionary-terms).
5. Run your agent's tests: tools should now receive real values in their arguments.

## How do they compare on accuracy?

PIIMiddleware's built-in detectors are regex-based and no accuracy figures are published. Shadow's [benchmark](/shadow/benchmarks/) measures leak rate, over-masking, round-trip exactness and latency, with code published. On the held-out set, Shadow's built-in rules leaked 3.08% of sensitive values and restored 100% of documents exactly.

## Sources

- [LangChain PIIMiddleware reference](https://reference.langchain.com/python/langchain/agents/middleware/pii)
- [LangChain on GitHub](https://github.com/langchain-ai/langchain)
- [Shadow tool calls and agents](/shadow/concepts/tool-calls/)

---

*Last reviewed: 2026-10-31. LangChain is a trademark of its owner; this comparison is based on public documentation as of the review date.*
