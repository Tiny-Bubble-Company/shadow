---
title: "How to mask PII in LLM agents without breaking tool calls"
description: "Keep personal data away from the model while agents still email, query and update the real records: recipients pass through, everything else is restored locally."
keywords:
  - mask pii llm agents
  - pii in tool calls
  - agent data privacy
  - function calling pii redaction
  - mcp privacy pii
  - langchain pii masking
faq:
  - q: "If the model only sees fake data, how does the agent act on the real customer?"
    a: "The person the agent is asked to contact or act on is the recipient, and Shadow passes the recipient through as written (and logs it), so the model addresses the right person. Everything else is masked; the model writes tool calls with the fakes it saw, and Shadow restores the real values in the tool-call arguments on your machine before your code runs the tool. Set [policy.act] targets = \"surrogate\" to fake the recipient too and restore it locally."
  - q: "What about tools the provider runs, such as hosted MCP servers or web search?"
    a: "Shadow can't restore values inside the provider's cloud. By default it sends the real target of that action and logs the keep, while everything else stays masked. Set [policy.act] remote_tool_targets = \"surrogate\" if you would rather the action fail than reveal the target."
  - q: "Does this work with streamed tool calls?"
    a: "Yes. Tool-call arguments streamed in pieces are reassembled and restored, for OpenAI Chat Completions and Responses, Anthropic tool_use and Gemini functionCall."
  - q: "Does it work with LangChain, LlamaIndex or the OpenAI Agents SDK?"
    a: "Yes, if the framework lets you set the provider base URL or honours HTTPS_PROXY. Shadow works at the HTTP level, so no framework plugin is needed."
  - q: "How should agents handle large tool results such as query exports?"
    a: "Requests with many personal values or tabular data are classified as report, and people, emails, phones and addresses become typed tokens such as [PERSON_0001]. The model carries them through as opaque IDs and they are restored in the answer."
sidebar:
  order: 7
---

Route the agent's model calls through Shadow and keep tool execution in your own code. The person you ask the agent to contact or act on passes through as written, so the model writes to the right person; each keep is logged. Everything else the model sees in prompts, history and tool results is a realistic fake; when it calls a tool, Shadow translates the arguments back to real values before your tool runs, and masks the tool's result again on the way to the model. If the recipient must stay hidden too, set `targets = "surrogate"`. Provider-hosted tools are covered [below](#provider-hosted-tools-keep-or-surrogate).

## Why is PII in agents harder than in chat?

In chat, a model only needs to *write about* a person. An agent needs to *act on* one: send the email, update the CRM record, look up the order. Masking that ignores this breaks the agent:

| Approach | What goes wrong |
|---|---|
| No masking | Tool results (database rows, CRM records, inboxes) pour personal data into every turn |
| Placeholder redaction | The model calls `send_email(to="<EMAIL>")`. The action fails |
| One-way fakes | The model calls `send_email(to="hui.zhao@zhilan.cn")`. The action goes to a stranger |
| **Shadow** | The recipient is passed through, so the model calls `send_email(to="jane.doe@acme.example")` and writes "Dear Jane"; other people and values in the body are fakes, restored locally before your code sends it. With `targets = "surrogate"` the recipient is a fake too, restored the same way |

## How the round trip works

```text
agent ──(prompt + tool schemas)────────▶ Shadow ──(masked)──────────────▶ model
agent ◀──(tool call, real arguments)── Shadow ◀──(tool call, fake args)── model
agent runs the tool with real values
agent ──(tool result)──────────────────▶ Shadow ──(result masked)────────▶ model
```

1. Prompts, history and earlier tool results are masked. Protocol keys such as `type`, `id` and `tool_call_id` are never touched.
2. The model writes arguments with what it saw: the real recipient, fakes for everything else.
3. Shadow restores the fakes in the arguments before your code sees them, also when they are streamed in fragments.
4. Your tool's result contains real values; Shadow masks them with the same fakes, because the [vault](/shadow/concepts/vault/) maps values consistently.

Supported shapes: OpenAI Chat Completions `tool_calls`, OpenAI Responses function calls, Anthropic `tool_use`, and Gemini `functionCall`. Full detail in [tool calls and agents](/shadow/concepts/tool-calls/).

## How to set it up

### 1. Run Shadow next to the agent

```sh
shadow init --org "Acme Ltd"
shadow run
```

In production, run it as a [sidecar](/shadow/integrate/docker-kubernetes/) in the agent's pod, listening on loopback.

### 2. Point the agent's model client at Shadow

```sh
export OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1
export ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic
```

Most frameworks (LangChain, LlamaIndex, the OpenAI Agents SDK, the Vercel AI SDK) pick these up or accept a base URL parameter. Python example with plain function calling:

```python
from openai import OpenAI
import json

client = OpenAI(base_url="http://127.0.0.1:8877/gw/openai/v1")

tools = [{"type": "function", "function": {
    "name": "send_email",
    "parameters": {"type": "object", "properties": {
        "to": {"type": "string"}, "body": {"type": "string"}}, "required": ["to", "body"]}}}]

resp = client.chat.completions.create(
    model="gpt-4o-mini",
    messages=[{"role": "user", "content": "Email jane.doe@acme.example that her renewal is confirmed"}],
    tools=tools,
)
call = resp.choices[0].message.tool_calls[0]
args = json.loads(call.function.arguments)
print(args["to"])        # jane.doe@acme.example: the recipient, passed through and logged
```

### 3. Set the agent policy

```toml
[policy]
intent = true                    # classify act / report / code / converse

[policy.act]
targets = "keep"                 # the recipient: pass through, logged ("surrogate" hides it)
remote_tool_targets = "keep"     # provider-hosted tools: send the real target, logged

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"

[policy.always]
secret = "redact"
```

### 4. Prefer local tool execution

Only tools that **your code** executes can receive restored values. For actions on personal data, especially with `targets = "surrogate"`, run the tool yourself (your own MCP client and server, your function-calling loop) rather than a provider-hosted connector.

### 5. Watch the decisions

The log shows each request's intent and counts per treatment. Every `keep` is logged with the value's kind and reason, so you can see exactly what left in clear text.

## Provider-hosted tools: keep or surrogate?

| Setting | Provider sees | Hosted action |
|---|---|---|
| `remote_tool_targets = "keep"` (default) | The real target of that one action; everything else masked | Works |
| `remote_tool_targets = "surrogate"` | A fake target | Runs against the fake (the email goes nowhere useful) |

Choose `surrogate` when exposure matters more than the hosted action succeeding.

## Example: CRM lookup and follow-up

User instruction (fictional data): *"Email Jane Doe at Globex a status update on her open tickets. Tom Reed handled the last one."*

- Model sees: *"Email Jane Doe at Globex a status update on her open tickets. Wei Chen handled the last one."* Jane Doe and Globex are the recipient and pass through (logged); Tom Reed is not, so he is a fake.
- Model calls `search_tickets(customer="Globex", contact="Jane Doe")`; your code runs it as is.
- Your tool returns three tickets with other people's names and emails; the model sees them masked.
- Model calls `send_email(to=..., body="Dear Jane, ... Wei Chen handled ...")`; Shadow restores "Tom Reed" in the body before your code sends it to Jane.

Detectors can't know your customers, so add them as terms (`shadow terms add "org:Globex"`). Here Globex passes through because it is part of the recipient; in a request that only talks about Globex, the term makes sure it is masked.

## In-process with Rust

If your agent is written in Rust, [embed `shadow-core`](/shadow/integrate/rust/) and use one session per model call:

```rust
let session = shadow.session_for(&request_json);   // classifies intent from instruction and tools
session.mask_json(&mut request_json);
let mut response = send(&request_json)?;
session.unmask_json(&mut response);                // tool-call arguments come back real
let summary = session.summary();                   // surrogates, tokens, kept, redacted
```

For streams, `session.stream_unmasker()` restores text and arguments chunk by chunk.

## Practical tips

- Keep tool **descriptions** free of personal data.
- Shadow restores whole fakes, and a fake person's first or last name on its own ("Dear Hui," becomes "Dear Jane," with `targets = "surrogate"`). Other fragments, such as the domain of a fake email or the last four digits of a fake card, stay fake, so ask for full values when the text goes to a real person.
- When a tool must quote an exact value back (an order ID, an IBAN), surrogates still round-trip. Test with `shadow mask` and `shadow unmask`.
- Keep the vault persistent so fakes stay stable across long-running agent sessions.
- Not supported: Realtime and WebSocket agents (voice), and SigV4-signed Bedrock requests.

## Related

- [Protect customer data in RAG pipelines](/shadow/use-cases/protect-customer-data-in-rag-pipelines/)
- [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/)
- [Intent-aware policy](/shadow/concepts/intent-policy/), [Tool calls and agents](/shadow/concepts/tool-calls/), [Integrating any endpoint](/shadow/integrate/any-endpoint/)
- [Benchmarks](/shadow/benchmarks/) and [how Shadow compares](/shadow/compare/)
