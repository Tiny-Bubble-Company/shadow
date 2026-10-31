---
title: Intent-aware policy
description: "Shadow classifies each AI request as act, report, code or converse and picks a fake, token, real value or redaction for every sensitive value."
---

One treatment does not fit every request. A chat about a customer works best with realistic fakes. A report over a thousand customer rows works best with opaque IDs. An email has to reach the real person. Shadow's policy looks at what a request is *for*, then picks a treatment for each detected value, so masking never gets in the way of the goal.

The rule behind every decision: **a value is masked unless masking it would change what happens in the real world.** Text the model writes comes back through Shadow, which restores the real values before you see it. Tool calls your own code runs also come back through Shadow first. Only an action the provider carries out itself (a hosted MCP server or other server-side tool) never passes back through Shadow, so for that action alone the values it needs are sent as they are, and every such value is logged.

## Treatments

| Treatment | What the provider sees | Reversible | Use |
|---|---|---|---|
| `surrogate` | A realistic, format-preserving fake from the persona: `hui.zhao@zhilan.cn` | Yes | **Default.** Keeps prompts natural so answers stay good |
| `token` | A typed placeholder: `[PERSON_0007]`, `[EMAIL_0002]` | Yes | Bulk data in reports, where the model should treat values as opaque IDs |
| `keep` | The real value | n/a | Only for the target of an action that runs in the provider's cloud. Every keep is logged |
| `redact` | `[REDACTED_SECRET]` | **No** | Opt-in, for values that must never leave and never need to come back |

Surrogates and tokens are both restored in the response. The difference is how the model treats them: a surrogate reads like real data, so the model writes naturally about it; a token is visibly a placeholder, so the model copies it through without inventing details about it.

## Intents

With `[policy] intent = true` (the default), every request is classified into one of four intents. The classifier is local and rule-based. It looks at the latest instruction, the tools offered to the model, and the shape of the data.

| Intent | Signals | Example |
|---|---|---|
| `act` | The model is asked to *do* something: send, email, message, call, invite, book, pay, post. The verb can open the sentence or follow "to", "and" or "then" ("I need you to email…", "write it and send it") | "Email jane@acme.com about the renewal" |
| `report` | Report, summarise, analyse, export or aggregate over data; or many PII values or tabular data in the request | "Build a churn report from this customer list" |
| `code` | Source code, diffs, stack traces, configuration | "Why does this config fail to connect?" |
| `converse` | Everything else, including drafting text such as an email you will send yourself | "Draft a renewal reminder to Jane Doe" |

In Rust, `session.classification()` returns the intent, whether any actions run remotely (`remote_actions`), and the reasons for the decision. The proxy logs the intent and a per-treatment summary for each request.

## Decision table

| Value | `converse` | `code` | `report` | `act` |
|---|---|---|---|---|
| Person, email, phone, address | surrogate | surrogate | **token** | surrogate |
| Target of the action, local tool | | | | surrogate, restored in the tool call |
| What a provider-hosted action needs: recipient, people, companies, addresses, usernames and IBANs in the instruction (and, for a follow-up like "now send her the invite", emails, phones and people anywhere in the request) | | | | **keep** (configurable) |
| Organisation, terms, hostnames, IDs | surrogate | surrogate | surrogate | surrogate |
| Secrets | surrogate | surrogate | surrogate | surrogate |
| Anything listed under `[policy.always]` | that treatment | that treatment | that treatment | that treatment |

Defaults are shown. Every cell is configurable.

## Example: an action with a local tool

> "Email jane@acme.com about the renewal"

Your app offers the model a `send_email` tool that **your code** executes.

1. Shadow classifies the request as `act`. `jane@acme.com` is the action's target.
2. The tool runs locally, so the target gets a **surrogate**. The provider sees *"Email hui.zhao@zhilan.cn about the renewal"*.
3. The model answers with a tool call: `send_email(to="hui.zhao@zhilan.cn", ...)`.
4. On the way back, Shadow restores the tool-call arguments on your machine: `send_email(to="jane@acme.com", ...)`.
5. Your code sends the email to Jane.

The action works, and the provider never saw the address. This also works when tool-call arguments are streamed in pieces.

## Example: an action with a provider-hosted tool

Same instruction, but the tool is a hosted MCP server or another server-side tool that the **provider** executes. Shadow cannot restore values inside the provider's cloud: if it sent a fake, the email would go to `hui.zhao@zhilan.cn`.

So by default what the action needs is **kept**: the recipient, and the contact details, people, companies, addresses, usernames and IBANs named in the instruction, because they end up in the email Jane receives. For a follow-up that names no email address or phone number ("now send her the invite"), the email, phone and person values anywhere in the request (earlier messages, pasted documents, tool results) are kept too, because that is where the recipient is. Secrets, card numbers, national IDs, hostnames and IPs are still masked, other values in earlier messages and files stay masked, and every keep is logged. If you would rather the action fail than reveal those values, set `remote_tool_targets = "surrogate"`.

## Example: drafting text you will send yourself

> "Draft a renewal reminder to Jane Doe (jane@acme.com) at Acme GmbH"

No tool is involved: the model only writes text. The provider sees *"Draft a renewal reminder to Hui Zhao (hui.zhao@zhilan.cn) at Zhilan GmbH"* and writes the draft with those names. On the way back Shadow restores them, so the draft you read and send says Jane Doe, jane@acme.com and Acme GmbH. If the model greets her as "Dear Hui", that is restored to "Dear Jane", because fake first and last names are restored on their own. Other fragments, such as the domain of the fake email, stay fake.

## Example: a report

> "Build a churn report from this customer list" (followed by 400 rows of names, emails and phones)

1. Shadow classifies the request as `report`.
2. People, emails, phones and addresses become **typed tokens**: `[PERSON_0001]`, `[EMAIL_0001]`, ... Organisation names and other values still get surrogates (`default = "surrogate"`).
3. The model treats the tokens as opaque IDs. It groups, counts and ranks them without inventing facts about fake people.
4. The answer comes back with every token replaced by the real value: a churn report about your real customers.

## Example: a secret with redaction turned on

> "Why does this fail? `OPENAI_API_KEY=sk-proj-…` …"

By default a secret is a surrogate like everything else: the provider sees a fake key of the same shape, and if the model quotes the config back, you see your real key restored locally. If you would rather secrets never leave in any form, opt in to redaction:

```toml
[policy.always]
secret = "redact"
```

The provider then sees `[REDACTED_SECRET]` and the value cannot be restored.

## Configuration

```toml
[policy]
intent = true                  # classify each request (default true)

[policy.act]
targets = "surrogate"          # surrogate | keep: values the action is aimed at (local tools)
remote_tool_targets = "keep"   # surrogate | keep: when the action runs in the provider's cloud

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"          # every other kind in a report

[policy.always]                # empty by default; applies regardless of intent
# secret = "redact"
# national_id = "token"
```

Keys under `[policy.report]` and `[policy.always]` are detector kinds: `email`, `phone`, `person`, `org`, `term`, `address`, `iban`, `credit_card`, `national_id`, `ip`, `mac`, `hostname`, `secret`, `username`. Values are `surrogate`, `token`, `keep` or `redact`.

With `intent = false`, every request is treated as `converse`: surrogates everywhere, plus whatever `[policy.always]` says.

## Observability

- The proxy logs, per request, the intent and the count of each treatment (surrogates, tokens, kept, redacted). It never logs values.
- Every `keep` is logged with the value's kind and the reason (remote tool target), so you can audit what left in clear text.
- In Rust, `session.summary()` returns the same counts.
