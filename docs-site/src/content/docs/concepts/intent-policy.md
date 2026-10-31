---
title: Intent-aware policy
seoTitle: "Intent-aware PII policy for LLM requests · Shadow"
description: "Shadow classifies each AI request as act, report, code or converse and picks a fake, token, real value or redaction for every sensitive value."
faq:
  - q: "Why does Shadow send a message recipient's real details?"
    a: "If you ask the model to write to or act on someone, faking that person makes it greet the wrong name or address the wrong inbox. So the recipient phrase passes through as written and every keep is logged; everything else in the request stays masked."
  - q: "How do I hide the recipient from the provider too?"
    a: "Set `[policy.act] targets = \"surrogate\"`. The model then writes to a stand-in, and Shadow restores the real recipient in the answer and in tool-call arguments."
  - q: "What is the difference between a surrogate and a token?"
    a: "A surrogate is a realistic fake such as `hui.zhao@zhilan.cn`, so the model writes naturally about it. A token such as `[PERSON_0007]` is visibly a placeholder for bulk reports; both are restored in the response."
  - q: "Can I make sure secrets never leave in any form?"
    a: "Yes. Set `[policy.always] secret = \"redact\"` and the provider receives `[REDACTED_SECRET]`. Redaction is irreversible by design."
---

Shadow's intent-aware policy classifies each AI request as `act`, `report`, `code` or `converse`, and picks a treatment for every detected value: a realistic fake, a typed token, the real value or a redaction.

One treatment does not fit every request. A chat about a customer works best with realistic fakes. A report over a thousand customer rows works best with opaque IDs. An email has to reach the real person, so the model has to know who that person is. Shadow's policy looks at what a request is *for*, then picks a treatment for each detected value, so masking never gets in the way of the goal.

The rule behind every decision: **a value is masked unless masking it would change the goal of the request.** If you ask the model to write to or act on a person, faking that person makes it greet the wrong name, research the wrong company or address the wrong inbox. So the recipient of a message or action passes through as written, and every such value is logged. Everything else in the request (other people, health data, IDs, cards, secrets, hostnames) is still masked, and text the model writes comes back through Shadow, which restores those values before you see it.

## Treatments

| Treatment | What the provider sees | Reversible | Use |
|---|---|---|---|
| `surrogate` | A realistic, format-preserving fake from the persona: `hui.zhao@zhilan.cn` | Yes | **Default.** Keeps prompts natural so answers stay good |
| `token` | A typed placeholder: `[PERSON_0007]`, `[EMAIL_0002]` | Yes | Bulk data in reports, where the model should treat values as opaque IDs |
| `keep` | The real value | n/a | Only for the recipient of a message or action: the person you ask the model to write to or act on (default), and what a provider-hosted action needs. Every keep is logged |
| `redact` | `[REDACTED_SECRET]` | **No** | Opt-in, for values that must never leave and never need to come back |

Surrogates and tokens are both restored in the response. The difference is how the model treats them: a surrogate reads like real data, so the model writes naturally about it; a token is visibly a placeholder, so the model copies it through without inventing details about it.

## Intents

With `[policy] intent = true` (the default), every request is classified into one of four intents. The classifier is local and rule-based. It looks at the latest instruction, the tools offered to the model, and the shape of the data.

| Intent | Signals | Example |
|---|---|---|
| `act` | The model is asked to contact or *do* something to someone: send, email, message, call, invite, book, pay, reply, remind. The verb can open a clause or follow "to", "and", "then", "you" or "please" ("I need you to email…", "write it and send it"). Composing a message counts too: draft, write, compose, prepare or create followed within six words by email, letter, message, reply, note, reminder, invitation and similar | "Email jane@acme.com about the renewal", "Draft a renewal reminder to Jane Doe" |
| `report` | Report, summarise, analyse, export or aggregate over data; or many PII values or tabular data in the request | "Build a churn report from this customer list" |
| `code` | Source code, diffs, stack traces, configuration | "Why does this config fail to connect?" |
| `converse` | Everything else | "What should I say in a renewal negotiation?" |

In Rust, `session.classification()` returns the intent, whether any actions run remotely (`remote_actions`), and the reasons for the decision. The proxy logs the intent and a per-treatment summary for each request.

## Decision table

| Value | `converse` | `code` | `report` | `act` |
|---|---|---|---|---|
| Person, email, phone, address | surrogate | surrogate | **token** | surrogate |
| Recipient of the message or action (names, emails, phones, companies, addresses, IBANs, usernames in the recipient phrase) | | | | **keep** (default; set `targets = "surrogate"` for a stand-in restored in the answer and tool calls) |
| What a provider-hosted action needs: recipient, people, companies, addresses, usernames and IBANs in the instruction (and, for a follow-up like "now send her the invite", emails, phones and people anywhere in the request) | | | | **keep** (configurable) |
| Organisation, terms, hostnames, IDs | surrogate | surrogate | surrogate | surrogate |
| Secrets | surrogate | surrogate | surrogate | surrogate |
| Anything listed under `[policy.always]` | that treatment | that treatment | that treatment | that treatment |

Defaults are shown. Every cell is configurable.

## How the recipient is found

For an `act` request, Shadow reads the **recipient phrase**: the words after "to", "for" or "at" that follow the verb, or the words right after a verb that takes the person as its object ("Email jane@acme.com", "Call Jo"). The phrase ends at "about", "regarding", "that", "saying", "telling", "asking", "to say/tell/ask", "with the/a/our", "explaining", "to confirm", "to let", "so that", "because", "if", "when" or the end of the sentence. Names, emails, phones, companies, addresses, IBANs and usernames inside that phrase are kept; everything after it is masked as usual.

## Example: an action with a local tool

> "Email jane@acme.com about the renewal"

Your app offers the model a `send_email` tool that **your code** executes.

1. Shadow classifies the request as `act`. `jane@acme.com` is the recipient.
2. The recipient is **kept** (and logged). The provider sees *"Email jane@acme.com about the renewal"*.
3. The model answers with a tool call: `send_email(to="jane@acme.com", ...)`.
4. Your code sends the email to Jane.

Anything else in the request, such as a colleague's name or a card number in the body, is still masked and restored in the tool-call arguments on your machine, also when they are streamed in pieces.

If the recipient must stay hidden from the provider and your own code sends the message, set `targets = "surrogate"`. The provider then sees *"Email hui.zhao@zhilan.cn about the renewal"*, the model calls `send_email(to="hui.zhao@zhilan.cn", ...)`, and Shadow restores `jane@acme.com` in the arguments before your code runs. The trade-off: the model writes to a stand-in, so it may greet or research the wrong person.

## Example: an action with a provider-hosted tool

Same instruction, but the tool is a hosted MCP server or another server-side tool that the **provider** executes. Shadow cannot restore values inside the provider's cloud: if it sent a fake, the email would go to `hui.zhao@zhilan.cn`.

So by default what the action needs is **kept**: the recipient, and the contact details, people, companies, addresses, usernames and IBANs named in the instruction, because they end up in the email Jane receives. For a follow-up that names no email address or phone number ("now send her the invite"), the email, phone and person values anywhere in the request (earlier messages, pasted documents, tool results) are kept too, because that is where the recipient is. Secrets, card numbers, national IDs, hostnames and IPs are still masked, other values in earlier messages and files stay masked, and every keep is logged. If you would rather the action fail than reveal those values, set `remote_tool_targets = "surrogate"`.

## Example: drafting text you will send yourself

> "Draft a renewal reminder to Jane Doe (jane@acme.com) at Acme GmbH. Mention that Max Weber approved it and card 4111 1111 1111 1111 will be charged."

No tool is involved, but the goal is still to write to Jane, so the request is `act`. Jane Doe, her email and Acme GmbH are the recipient phrase and pass through as written, so the model greets the right person and writes about the right company. Max Weber and the card number are not the recipient: the provider sees a fake name and a fake card, and Shadow restores the real ones in the draft you read and send.

This means the recipient's details reach the provider whenever you ask the model to write to them. To hide them, set `targets = "surrogate"`: the provider then writes to a stand-in and Shadow restores the real name and address in the draft (fake first and last names are restored on their own, so "Dear Hui" becomes "Dear Jane").

Requests that are *about* people but do not contact them ("summarise this thread", "analyse these complaints") are not `act`, so everyone in them stays masked.

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
targets = "keep"               # keep | surrogate: the recipient of a message or action
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
- Every `keep` is logged with the value's kind and the reason (message recipient or remote tool target), so you can audit what left in clear text.
- In Rust, `session.summary()` returns the same counts.
