---
title: "Privacy for AI in customer support: mask customer data in tickets and agents"
description: "Use LLMs for ticket replies, summaries and support agents without sending customer names, emails, orders or card numbers to the provider. Restored in answers."
keywords:
  - ai customer support privacy
  - mask pii in support tickets
  - chatgpt customer service data privacy
  - ai support agent pii
  - zendesk ai pii openai
  - anonymize tickets for llm
faq:
  - q: "Can support agents use AI to draft replies without exposing customer data?"
    a: "Yes, if the drafting tool calls the model through Shadow. The customer you reply to is the recipient: by default their name and email pass through as written, so the model greets the right person, and each keep is logged. Colleagues, other customers, card numbers and other details are replaced with realistic fakes and restored in the draft. Set [policy.act] targets = \"surrogate\" to fake the recipient too."
  - q: "Does Shadow work with Zendesk, Intercom or Freshdesk AI features?"
    a: "Not with features that run inside the help-desk vendor's cloud, because that traffic never passes through your machines. It works with AI features you build yourself on the OpenAI, Anthropic or other supported APIs, for example a help-desk app or webhook service that calls the model through the Shadow gateway."
  - q: "Will an AI support agent still be able to issue refunds or send emails?"
    a: "Yes, if the tools run in your code. The customer the agent acts on is passed through, so the model calls the tool with their real email; any other masked values in the arguments are restored on your side before the tool runs."
  - q: "Can we analyse thousands of tickets with an LLM safely?"
    a: "Yes. Requests that summarise or analyse many records are classified as report, so people, emails, phones and addresses become typed tokens such as [PERSON_0001]. The model groups and counts them, and the report is restored with real values locally."
  - q: "Are payment card numbers in tickets protected?"
    a: "Card numbers are detected with a Luhn check and replaced with a different valid-looking number, or with a token in reports. You can also send them as [CREDIT_CARD_0001] in every request with [policy.always] credit_card = \"token\"."
sidebar:
  order: 14
---

To use AI in customer support without exposing customers, mask their personal data before each request reaches the model and restore it in the reply. Shadow does that for ticket summaries, reply drafts, support agents and ticket analytics built on provider APIs: names, emails, phones, addresses, order references you register, and card numbers become reversible fakes, and streamed replies come back with the real details. When the goal is to reply to or act on a customer, that customer is the recipient and passes through as written (logged), so the reply greets the right person and tool calls such as refunds reach them.

## What customer data reaches the model in support workflows?

| Workflow | What the prompt contains |
|---|---|
| Drafting a reply | The whole thread: name, email, phone, address, order history, sometimes card digits |
| Summarising a ticket for hand-off | Customer details plus internal notes and colleague names |
| Support agent with tools | Customer lookup results, refund or shipping actions, outbound emails |
| Ticket analytics and QA | Hundreds of tickets at once, exported as CSV or JSON |
| Knowledge-base answers (RAG) | Retrieved past tickets that mention other customers |

Support data is high-volume and personal by nature, and it usually flows through your own integration code, which is exactly where Shadow fits.

## How Shadow protects support data

| Data | Handling |
|---|---|
| Names, emails, phones, street addresses | Built-in detectors; realistic fakes from the [persona](/shadow/concepts/personas/) |
| Card numbers, IBANs | Luhn- and mod-97-checked; replaced with valid-looking fakes |
| National IDs (US SSN, UK NINO, Aadhaar and others) | Built-in detectors |
| Your company, partner and product names | `organization` and `terms` |
| Agent and colleague names | Name detector, plus `person:` terms for unusual names |
| Ticket exports in reports | Typed tokens via the `report` intent |
| Attached CSV, DOCX, XLSX, PDF text | [File masking](/shadow/concepts/file-masking/) |
| The customer you reply to or act on | Passed through as the recipient and logged; `targets = "surrogate"` fakes it and restores it locally ([intent policy](/shadow/concepts/intent-policy/)) |

Order numbers, ticket IDs and loyalty IDs are not personal data on their own and are not detected by the built-in detectors. If they are sensitive for you, there is no custom regex setting in `shadow.toml`; use the NER sidecar for account-style numbers, or a custom detector if you embed Shadow in Rust.

## How to set it up

### 1. Run Shadow next to your support integration

```sh
shadow init --org "Acme GmbH"
shadow run
```

In production, run it as a [sidecar](/shadow/integrate/docker-kubernetes/) of the service that calls the model, with a persistent `/data` volume so a customer keeps the same fake across a long thread.

### 2. Point the model client at the gateway

```python
from openai import OpenAI

client = OpenAI(base_url="http://127.0.0.1:8877/gw/openai/v1")

stream = client.chat.completions.create(
    model="gpt-4o-mini",
    stream=True,
    messages=[
        {"role": "system", "content": "You are a support agent for Acme GmbH. Be concise."},
        {"role": "user", "content": ticket_thread},
    ],
)
for chunk in stream:
    print(chunk.choices[0].delta.content or "", end="")   # real names, restored on the fly
```

### 3. Configure the policy

```toml
organization = ["Acme GmbH"]
terms = ["org:Globex Ltd", "Acme Plus"]   # partner and plan names
internal_domains = ["acme.example", "support.acme.internal"]

[policy.act]
targets = "keep"                 # the customer you write to passes through, logged ("surrogate" hides them)
remote_tool_targets = "surrogate" # never send real targets to provider-hosted tools

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"

[policy.always]
credit_card = "token"
secret = "redact"
```

### 4. Run tools in your own code

Keep actions such as `issue_refund`, `send_email` and `lookup_order` as local tools your service executes. Other values in their arguments (and the recipient, with `targets = "surrogate"`) are restored on your side before they run. Provider-hosted tools cannot have values restored inside the provider's cloud.

### 5. Test on real-looking tickets

```sh
shadow scan sample-ticket.txt
shadow scan export.csv --json
```

## Example: a duplicate-charge ticket

What the support tool sends (fictional):

```text
Draft an apology to Jane Doe (jane.doe@acme.example) and refund ticket #88213.
It was charged twice on card ending 1111; our agent Tom Reyes confirmed it.
The order shipped to 221 Baker Street, London.
```

What the provider sees (persona values differ per machine):

```text
Draft an apology to Jane Doe (jane.doe@acme.example) and refund ticket #88213.
It was charged twice on card ending 1111; our agent Lei Li confirmed it.
The order shipped to 192 Keyuan Road, London.
```

Jane Doe and her email are the recipient, so they pass through as written and each keep is logged: the model greets her by name and calls `issue_refund(customer_email="jane.doe@acme.example", ...)` with her real address. Tom Reyes and the delivery address are not the recipient, so the provider sees fakes, and Shadow restores them in the draft reply. With `targets = "surrogate"`, Jane would be a fake as well and Shadow would restore her email in the tool arguments and her name in the draft.

Note that the last four card digits and the ticket number are sent as is: they are not full identifiers, and the built-in detectors do not treat them as such.

## Example: weekly ticket themes

An analyst sends 900 tickets as CSV and asks for the top complaint themes. Shadow classifies it as `report`: customers become `[PERSON_0001]`, `[EMAIL_0001]` and so on. The model clusters and counts without inventing details about fake people, and the report is restored locally.

## Limitations

- **Vendor-hosted AI features** in help-desk products run in the vendor's cloud and are not intercepted.
- **Voice and Realtime APIs** (WebSocket) are not processed, so AI phone agents built on them are not covered.
- **Screenshots** customers attach are not OCR'd. Set `[files] images = "block"` if they reach the model.
- **Context can identify.** A unique complaint about a named event can point to a customer even with every name replaced.

## Regulations and frameworks

Masking customer data before it reaches an AI provider supports GDPR data minimisation (Art. 5(1)(c)), pseudonymisation by design (Art. 25, 32), and keeping card data out of AI systems helps limit PCI DSS scope. It is a technical control, not a compliance programme.

:::caution[Not legal advice]
Shadow does not make your support operation compliant with GDPR, PCI DSS or any other framework. Confirm your obligations with your privacy and legal teams.
:::

## Related

- [Masking PII in LLM agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/)
- [Protect customer data in RAG pipelines](/shadow/use-cases/protect-customer-data-in-rag-pipelines/)
- [E-commerce and retail](/shadow/industries/ecommerce-retail/), [SaaS and technology](/shadow/industries/saas-technology/), [telecommunications](/shadow/industries/telecommunications/)
- [Intent-aware policy](/shadow/concepts/intent-policy/), [benchmarks](/shadow/benchmarks/), [how Shadow compares](/shadow/compare/)
