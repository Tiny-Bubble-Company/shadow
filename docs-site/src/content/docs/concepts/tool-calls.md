---
title: Tool calls and agents
description: "How Shadow keeps AI agents working on masked data: tool-call arguments are restored locally before tools run, and provider-hosted tools follow policy."
---

Agents turn model output into actions: sending email, querying databases, creating tickets, running shell commands. That raises a question plain masking cannot answer: if the model only ever sees fakes, how does the action reach the real target?

## The round trip

In a typical agent loop, the model proposes a tool call, **your code** runs it, and the result goes back to the model:

```text
app ──(prompt + tool schemas)──▶ Shadow ──(masked)──▶ model
app ◀──(tool call, real args)── Shadow ◀──(tool call, fake args)── model
app runs the tool with real values
app ──(tool result)──▶ Shadow ──(result masked)──▶ model
```

Shadow handles each leg:

1. **Request.** Prompts, prior messages and previous tool results are masked like any other text. Tool *schemas* are scanned like any other strings. Protocol keys such as `type` are never touched, and tool names and parameter definitions rarely contain anything a detector matches, so they normally arrive intact.
2. **Model's tool call.** The model writes arguments using the fakes it saw: `{"to": "hui.zhao@zhilan.cn"}`.
3. **Restoration.** Shadow restores the arguments before your code sees them: `{"to": "jane@acme.com"}`. This works for complete JSON responses and for streamed arguments, where a fake may be split across many deltas.
4. **Tool result.** When your code sends the result back, it contains real values again, and Shadow masks them with the same fakes as before, because the vault maps them consistently.

The model reasons about a coherent fake world from start to finish. Your tools operate on the real one.

Supported tool-call shapes include OpenAI Chat Completions `tool_calls`, the OpenAI Responses API function calls, Anthropic `tool_use` blocks and Gemini `functionCall` parts.

## Local tools versus provider-hosted tools

The round trip above requires the tool to run on your side of Shadow. Some tools run inside the provider's cloud instead:

- hosted MCP servers that the provider connects to on your behalf;
- server-side tools such as web search, code execution or connectors executed by the provider.

For these, the arguments never come back through Shadow before the action happens, so a fake target would be used as is: the email would go to `hui.zhao@zhilan.cn`.

The [intent policy](/shadow/concepts/intent-policy/) handles this explicitly:

```toml
[policy.act]
targets = "surrogate"          # local tools: mask, restore in the tool call
remote_tool_targets = "keep"   # provider-hosted tools: send the real target
```

- With `keep` (default), only what the action needs is sent in clear text: the contact details, people, companies, addresses, usernames and IBANs named in the latest instruction, and, for a follow-up such as "now send her the invite" that names no email or phone, the email, phone and person values anywhere in the request (earlier messages, pasted documents, tool results). Secrets, card numbers, national IDs, hostnames and IPs stay masked. Each keep is logged with its kind and reason, never the value.
- With `surrogate`, the provider never sees the target, and the hosted action operates on a fake. Use this when exposure matters more than the action succeeding.

## Practical guidance

- Prefer **local tool execution** for actions on personal data. It is the only way to have both privacy and a working action.
- Keep tool **descriptions** free of personal data. Anything sensitive in them is masked like other text, which can make the description harder for the model to follow.
- Use typed **report** handling for bulk data an agent reads (exports, query results): tokens are cheaper for the model to carry through than realistic fakes, and less likely to be embellished.
- When a tool returns data that the model must quote exactly (IDs, account numbers), surrogates still round-trip correctly as long as the model quotes the whole value. A fake person's first or last name on its own is restored, but other fragments (the last four digits of a card, part of an email) are not, so ask for full values in tool arguments. Test with `shadow mask` and `shadow unmask` if in doubt.
- In Rust, use one [session](/shadow/integrate/rust/#sessions-and-intent) per model call, and `session.unmask_json` on the response to restore tool-call arguments.
