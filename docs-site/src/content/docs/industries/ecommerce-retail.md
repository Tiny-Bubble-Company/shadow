---
title: "Secure AI for e-commerce and retail: protect customer and payment data with Shadow"
description: "Use LLMs for customer service, order operations and analytics without sending shopper names, addresses or card numbers to the AI provider."
keywords:
  - ecommerce ai customer data privacy
  - retail chatgpt pii
  - pci dss ai chatbot
  - mask customer addresses llm
  - ai customer service privacy
  - shopify openai pii
faq:
  - q: "Can we use ChatGPT for customer service without exposing customer data?"
    a: "If your support tool or bot calls a model API, route it through Shadow. Customer names, emails, phones, delivery addresses and card numbers are replaced with fakes before the request leaves, and restored in the reply. The consumer chatgpt.com app is not covered."
  - q: "Does Shadow keep card numbers out of AI prompts for PCI DSS?"
    a: "Card numbers are detected with a Luhn check and replaced with a fake, or a typed token if you configure it, so the PAN does not reach the provider. The machine running Shadow still handles the real PAN and stays in your PCI scope."
  - q: "Do order numbers get masked?"
    a: "Not by default. Order numbers are usually not personal on their own and the built-in detectors do not look for them. If yours are sensitive, add known values as terms or use a custom detector in Rust."
  - q: "Can an AI agent still refund or email the right customer?"
    a: "Yes. The customer the instruction names is the recipient and passes through as written, logged, so the model calls the tool with their real email. Any other masked value in the tool call is restored in the arguments before your code executes it. If the tool is hosted by the provider (a hosted MCP server or server-side tool), Shadow cannot restore inside the provider, so by default the customer contact details the action needs are kept real and each keep is logged."
  - q: "How do I analyse thousands of customers with an LLM safely?"
    a: "Ask for a report and attach the export. Shadow classifies the request as a report and replaces names, emails, phones and addresses with typed tokens such as [PERSON_0001], so the model treats them as IDs. The answer comes back with real values."
sidebar:
  order: 9
---

Shadow is for online retailers, marketplaces, D2C brands and store operators that use LLMs for customer service, order operations, marketing and analytics. It runs on your machines or next to your services, replaces shopper names, emails, phone numbers, delivery addresses and card numbers with realistic, format-preserving fakes, and restores them in the answer. Your bots and agents keep working on real orders while the AI provider sees fake customers.

## What data leaks into AI tools in e-commerce and retail

| Workflow | What ends up in the prompt |
|---|---|
| Support bots and agent assist | Customer name, email, phone, delivery address, order history, sometimes a card number typed into chat |
| Returns and fraud review | Billing and shipping addresses, IPs, card BINs and last-4, account emails |
| Review and survey analysis | Reviewer names and emails, free-text complaints |
| Marketing segmentation | CRM exports with contact details and purchase history |
| Store and supplier operations | Supplier contacts, contract terms, internal system names |
| Building commerce integrations | Sample orders with real data, internal hostnames, payment API keys |

## How Shadow protects it

| Data | Treatment |
|---|---|
| Card numbers | Luhn-checked detection, Luhn-valid fakes; or tokens with `[policy.always] credit_card = "token"` |
| Names, emails, phones | Built-in detectors with persona-consistent fakes |
| Delivery and billing addresses | Street-address detector (English and German styles); NER sidecar or Presidio for other formats |
| IPs | `ip` detector |
| Payment and platform API keys | `secret` detector, including Stripe live keys; redact with `secret = "redact"` |
| Brands, suppliers, campaign codenames | `organization` and dictionary `terms` |
| CRM and order exports | The `report` intent turns people and contact details into typed tokens |
| Order numbers | Not detected by default; terms or a custom Rust detector if needed |

```toml
organization = ["Northwind Outfitters Ltd"]
terms = ["org:Globex Logistics", "Spring Drop 2027"]
internal_domains = ["northwind.example", "oms.northwind.internal"]

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
default = "surrogate"

[policy.always]
secret = "redact"
credit_card = "token"

[files]
images = "block"          # photos of receipts or damaged goods are not OCR'd
```

:::note[Coverage of street addresses]
The built-in address detector recognises English-style (`221 Baker Street`) and German-style (`Hauptstraße 5`) street addresses. For other formats, or apartment and postcode details, enable the open-weight NER sidecar or Presidio, and check results with `shadow scan`.
:::

## Regulations and frameworks

| Framework | How Shadow helps |
|---|---|
| PCI DSS v4.0.1 | Keeps PAN out of what reaches the AI provider, supporting scope reduction. The Shadow host sees real PAN and is in scope; ideally keep PAN out of chat channels altogether |
| GDPR Art. 5(1)(c), 25, 32 | Minimises and pseudonymises customer data before it reaches a processor. Still personal data for you (Recital 26) |
| CCPA / CPRA and US state privacy laws | Reduces personal information shared with AI service providers |
| ePrivacy and marketing rules | Not addressed by Shadow; consent for marketing use of data is a separate question |
| SOC 2, ISO/IEC 27001 | Evidence of a technical control for AI data flows |

:::caution[Not legal advice]
This page describes technical behaviour. It does not determine your PCI DSS scope or privacy compliance. Ask your QSA and counsel.
:::

## Example workflows

### Delivery complaint

Sent (fictional):

```text
Draft an apology email to Jane Doe (jane.doe@acme.example, +1 415 555 0132) with next steps.
Her order NW-77812 to 221 Baker Street never arrived.
She pasted her card number 4111 1111 1111 1111 into the chat.
```

Seen by the provider (persona varies):

```text
Draft an apology email to Jane Doe (jane.doe@acme.example, +1 415 555 0132) with next steps.
Her order NW-77812 to 48 Huaihai Road never arrived.
She pasted her card number [CREDIT_CARD_0001] into the chat.
```

Jane Doe is the recipient, so her name, email and phone pass through as written and each keep is logged; the model writes to the right customer. Her delivery address is not part of the recipient phrase, so it is masked and restored in the draft, ready to send. The card number was never needed for the apology, so the provider only ever saw a token. The order number passes through unless you configure it.

### Refund agent with a local tool

A support agent built on the OpenAI SDK offers `issue_refund(email, amount)`. When the instruction names the customer ("Refund jane.doe@acme.example for order NW-77812"), the email is the recipient and passes through, logged, so the model calls the tool with it. When the email only appears in the ticket history or a lookup result, the model sees a fake and calls the tool with it; Shadow restores `jane.doe@acme.example` in the arguments before your code runs the refund. See [tool calls](/shadow/concepts/tool-calls/).

### Review mining

A merchandiser pastes 3,000 reviews as CSV and asks for the top complaints by product. With that many names and emails in tabular form, Shadow treats the request as a report, so reviewer names and emails become tokens, product names stay readable, and quoted reviews are restored with real names on the merchandiser's screen.

## Deployment patterns

| Pattern | Use |
|---|---|
| **Sidecar** next to the support bot or agent-assist service | Customer-facing AI with egress limited to the sidecar |
| **Gateway** on an app server | Order operations and marketing tools calling OpenAI, Anthropic or Gemini via a base URL |
| **Endpoint** (`shadow install --apply`) | Analysts and engineers using API-backed assistants on laptops |

Voice bots built on WebSocket or Realtime APIs are not processed by Shadow today. Hosted support platforms that call models from their own cloud are outside Shadow's reach; Shadow protects calls your own systems make.

## Related

- [Financial services](/shadow/industries/financial-services/), [small business](/shadow/company-size/small-business/)
- [Privacy for AI in customer support](/shadow/use-cases/privacy-for-ai-in-customer-support/), [mask PII in LLM agents and tool calls](/shadow/use-cases/mask-pii-in-llm-agents-and-tool-calls/)
- [Intent-aware policy](/shadow/concepts/intent-policy/), [benchmarks](/shadow/benchmarks/), [compare](/shadow/compare/)
