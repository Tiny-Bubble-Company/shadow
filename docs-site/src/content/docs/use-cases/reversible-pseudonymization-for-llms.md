---
title: "Reversible pseudonymisation for LLMs: mask data and restore it in the answer"
description: "How to pseudonymise prompts for OpenAI, Anthropic or Gemini with realistic, reversible fakes, keep the key local, and get real values back in responses."
keywords:
  - reversible pseudonymization llm
  - pseudonymize prompts openai
  - reversible anonymization chatgpt
  - format preserving pseudonymisation
  - gdpr pseudonymisation ai
  - de-anonymize llm response
faq:
  - q: "What is the difference between pseudonymisation and anonymisation for LLM prompts?"
    a: "Anonymised data cannot be linked back to a person by anyone. Pseudonymised data can be, but only with additional information kept separately. Shadow pseudonymises: the provider sees fakes, and the mapping back to real values stays in an encrypted vault on your machine."
  - q: "How does Shadow restore the real values in the model's answer?"
    a: "Every fake is recorded in the vault next to the real value. When the response comes back, including streamed SSE events and tool-call arguments, Shadow replaces each fake with its real value before your app sees it."
  - q: "Can the AI provider reverse Shadow's fakes?"
    a: "No. Fakes are generated randomly from the persona, not derived from the real value, so there is nothing to decrypt or brute-force. The only link is the vault, which never leaves your machine."
  - q: "Is pseudonymised data still personal data under GDPR?"
    a: "For you, yes: you hold the key, so Recital 26 treats it as personal data. Pseudonymisation is still a recognised safeguard under Articles 25 and 32. Whether masked text counts as personal data for the recipient is a legal question; ask your DPO."
  - q: "When should I use tokens or redaction instead of realistic fakes?"
    a: "Use typed tokens such as [PERSON_0001] for bulk data in reports, so the model does not invent facts about fake people. Use redaction for values that must never leave in any form and never need to come back, such as secrets."
sidebar:
  order: 8
---

To pseudonymise LLM prompts reversibly, replace each sensitive value with a substitute before the request leaves, keep the mapping somewhere the provider cannot reach, and translate the substitutes back in the response. Shadow does all three on your machine: it swaps names, emails, IDs and secrets for format-preserving fakes from a persistent persona, stores the pairs in an encrypted local vault, and restores real values in answers, streams and tool calls.

## What does reversible pseudonymisation mean for LLM prompts?

GDPR Article 4(5) defines pseudonymisation as processing personal data so that it can no longer be attributed to a person *without additional information*, provided that information is kept separately and protected. Shadow maps onto that definition directly:

| GDPR element | In Shadow |
|---|---|
| Personal data replaced | Detected values become fakes or typed tokens |
| Additional information | The real↔fake mapping in `vault.bin` |
| Kept separately | The vault and its key stay on the machine or pod running Shadow. The provider receives fakes for every detected value, never the mapping |
| Technical measures | ChaCha20-Poly1305 encryption, a 32-byte key in `vault.key` with mode `0600`, atomic writes |

The "reversible" part is what makes it practical. Plain redaction (`<PERSON>`) is safe but destroys meaning, and the answer refers to placeholders you then have to resolve by hand. With reversible pseudonymisation the answer comes back about your real customer.

## How does Shadow pseudonymise a prompt?

```text
you      ──"Summarise this thread: jane.doe@acme.example asks about the Acme GmbH renewal"────────────────▶ Shadow
Shadow   ──"Summarise this thread: xiu.wang@lingxi.cn asks about the Lingxi Technology Co., Ltd. renewal"──▶ provider
provider ──"xiu.wang@lingxi.cn wants renewal terms…"──────────────────────────────────────────▶ Shadow
Shadow   ──"jane.doe@acme.example wants renewal terms…"───────────────────────────────────────▶ you
```

Three properties make the round trip reliable:

- **Format-preserving.** A fake email is an email, a fake card passes Luhn, a fake IBAN passes mod-97, a fake UK NINO looks like a NINO. Code, validation logic and the model's reasoning all keep working.
- **Consistent.** The same real value always gets the same fake, and no two real values share one, so restoration is exact. Because fakes come from one [persona](/shadow/concepts/personas/), they are also consistent with each other: your company maps to the persona's company, and emails at your domain map to the persona's domain.
- **Persistent.** The vault survives restarts, so a customer gets the same fake tomorrow. Conversation history and long-running agents stay coherent.

## Surrogates, tokens or redaction: which treatment?

Shadow's [intent-aware policy](/shadow/concepts/intent-policy/) picks a treatment per value. Three of them are pseudonymisation choices:

| Treatment | Provider sees | Reversible | Best for |
|---|---|---|---|
| `surrogate` (default) | `hui.zhao@zhilan.cn` | Yes | Conversation, analysis, code: the model writes naturally |
| `token` | `[PERSON_0001]`, `[EMAIL_0002]` | Yes | Reports over many rows: the model treats values as opaque IDs |
| `redact` | `[REDACTED_SECRET]` | **No** | Values that must never leave and never need to come back |

The fourth treatment, `keep`, sends the real value. It is used for the recipient when you ask the model to write to or act on someone (faking the recipient would make the model address the wrong person) and for what an action in the provider's cloud needs; every keep is logged. Set `[policy.act] targets = "surrogate"` to pseudonymise recipients too, restored in the answer. See [intent-aware policy](/shadow/concepts/intent-policy/).

## How to set up reversible pseudonymisation

### 1. Install and create the persona

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme GmbH"
shadow identity          # the fake identity providers will see
```

### 2. Try it offline

```sh
echo "Ask Jane Doe (jane.doe@acme.example, +1 415 555 0100) about Acme GmbH" | shadow mask > masked.txt
cat masked.txt
shadow unmask < masked.txt   # the original text comes back
```

Nothing is sent anywhere by these commands. They use the same vault as the proxy.

### 3. Add the names detectors cannot know

```sh
shadow terms add "org:Globex" "person:Jane Doe" "Project Falcon"
```

or in `shadow.toml`:

```toml
organization = ["Acme GmbH"]
terms = ["org:Globex", "person:Jane Doe", "Project Falcon"]
internal_domains = ["acme.example"]
```

### 4. Choose treatments

```toml
[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"

[policy.always]
secret = "redact"        # opt-in; irreversible
```

### 5. Route traffic through Shadow

```sh
shadow run
export OPENAI_BASE_URL=http://127.0.0.1:8877/gw/openai/v1
export ANTHROPIC_BASE_URL=http://127.0.0.1:8877/gw/anthropic
```

Or cover every tool on the machine that calls a [supported provider](/shadow/reference/providers/) with [proxy mode](/shadow/integrate/proxy-mode/) (`shadow install --apply`). If you build in Rust, [embed `shadow-core`](/shadow/integrate/rust/) and call `mask` / `unmask` directly.

### 6. Inspect and manage the mapping

```sh
shadow vault stats                          # counts by kind
shadow vault export --i-understand          # real↔fake pairs in clear text
shadow vault forget "jane.doe@acme.example" # drop one mapping
```

After `forget`, the value gets a new fake next time it appears, and older responses that mention the old fake are no longer restored. That is useful for honouring an erasure request against your local mapping. It does not delete anything the provider already holds, which for a detected value is only the fake.

## Limits to know

:::caution[Pseudonymisation is not anonymity]
- **Detection is best effort.** A value that is not detected is not pseudonymised. Add terms, enable the [NER sidecar or Presidio](/shadow/concepts/detection/), and test with `shadow scan`.
- **Dates and internal record numbers** (MRNs, claim or matter numbers) are not detected by the built-in detectors. There is no custom regex setting in `shadow.toml`.
- **Context can identify.** A unique event or a rare combination of facts can point to a person or company even with every name replaced.
- **Whole values, plus name parts.** Restoration matches complete fakes, and a fake person's first or last name on its own. Other fragments, such as the last four digits of a fake card or the domain of a fake email, are not translated back.
- **Coincidental fakes.** If the model happens to write text equal to one of your fakes, it is translated back too. This is rare and usually harmless.
- **The vault is the key.** Anyone who can read the data directory as your user can reverse everything. Use full-disk encryption and treat `vault.key` like a credential.
:::

## Regulations and frameworks

Pseudonymisation is named in GDPR Article 25 (data protection by design) and Article 32 (security of processing), and the EDPB's Guidelines 01/2025 on pseudonymisation describe it as a way to reduce risk while keeping data useful. Shadow gives you a technical implementation of it for AI traffic: the substitution happens before transfer, and the additional information never leaves your infrastructure. Pseudonymised data is still personal data for whoever holds the key (Recital 26), so your other obligations remain.

:::caution[Not legal advice]
Shadow is a technical control. It does not make processing lawful or compliant on its own. Confirm your legal position with your DPO or counsel.
:::

## Related

- [The vault](/shadow/concepts/vault/), [personas](/shadow/concepts/personas/), [intent-aware policy](/shadow/concepts/intent-policy/)
- [Help meet GDPR when using LLMs](/shadow/use-cases/gdpr-compliant-llm-usage/) and [anonymise prompts for Claude](/shadow/use-cases/anonymize-prompts-for-claude/)
- [Masking PII in agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/)
- How Shadow compares with placeholder-based tools: [compare](/shadow/compare/). Detection numbers: [benchmarks](/shadow/benchmarks/)
