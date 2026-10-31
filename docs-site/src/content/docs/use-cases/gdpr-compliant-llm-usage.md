---
title: "How to use ChatGPT, Claude and other LLMs in line with GDPR"
description: "Use LLMs with less personal data: Shadow pseudonymises prompts locally (GDPR Art. 4(5), 25, 32) before they reach OpenAI, Anthropic or Gemini."
keywords:
  - gdpr compliant llm
  - gdpr chatgpt
  - pseudonymisation llm gdpr
  - data minimisation ai prompts
  - gdpr openai api personal data
  - eu ai privacy llm
faq:
  - q: "Does Shadow make our use of ChatGPT or the OpenAI API GDPR compliant?"
    a: "No single tool does. Shadow is a technical measure that supports data minimisation (Art. 5(1)(c)), data protection by design (Art. 25) and security of processing (Art. 32) by pseudonymising personal data before it reaches the provider. You still need a lawful basis, a DPA with the provider, records of processing and, where required, a DPIA."
  - q: "Is pseudonymised data still personal data under GDPR?"
    a: "For you, yes. Shadow keeps the key (the vault) on your side, and Recital 26 treats pseudonymised data as personal data for anyone who can reasonably re-identify it. Whether masked prompts are personal data in the provider's hands depends on the circumstances; do not assume they are not."
  - q: "Do we still need a data processing agreement with OpenAI or Anthropic?"
    a: "Yes, if personal data may reach them. Detection is best effort, free text can contain identifiers Shadow does not recognise, and context can identify people. Shadow reduces what is sent; it does not remove the need for a DPA."
  - q: "Does Shadow help with international data transfers?"
    a: "It reduces the personal data that is transferred, which can be relevant to your transfer risk assessment. It does not replace a transfer mechanism such as the EU-US Data Privacy Framework or standard contractual clauses."
  - q: "Does Shadow itself process personal data in the cloud?"
    a: "No. Shadow runs on your machines or in your pods, has no account, no telemetry and no hosted service. The real-to-fake mapping stays in an encrypted vault under your control."
sidebar:
  order: 5
---

Using LLMs under GDPR comes down to sending the provider as little personal data as the task needs, and protecting what you do send. Shadow helps with the first part technically: it pseudonymises names, emails, phone numbers, addresses, national IDs, IBANs and other identifiers in every prompt on your own infrastructure, sends realistic fakes to the provider, and restores the real values in the answer. You still need the organisational side (lawful basis, DPA, records, DPIA); Shadow does not make anything "GDPR compliant" on its own.

## What does GDPR require when you use an LLM?

When staff or systems send personal data to an AI provider, you are disclosing it to a processor (or, depending on the terms, another controller), often outside the EEA. The obligations that matter most in practice:

| GDPR provision | What it asks | How Shadow helps |
|---|---|---|
| Art. 5(1)(c) data minimisation | Only data adequate, relevant and limited to what is necessary | Most prompts don't need real names or addresses to be answered. Shadow replaces them before sending |
| Art. 4(5) pseudonymisation | Data that can't be attributed to a person without separately kept additional information | The real↔fake mapping is the additional information, held in an [encrypted local vault](/shadow/concepts/vault/), never sent |
| Art. 25 data protection by design and by default | Technical measures built into processing | Masking happens automatically on the request path, not by user discipline |
| Art. 32 security of processing | Appropriate measures, explicitly including pseudonymisation | Pseudonymisation plus encryption at rest (ChaCha20-Poly1305) on infrastructure you control |
| Art. 35 DPIA | Assess high-risk processing | Shadow is a mitigation you can document; the DPIA itself is yours |
| Art. 44+ transfers | A valid transfer mechanism for data leaving the EEA | Less personal data is transferred. A transfer mechanism is still required |

Recital 26 matters here: pseudonymised data remains personal data for whoever can re-identify it, and you can. The [EDPB Guidelines 01/2025 on pseudonymisation](https://www.edpb.europa.eu/) describe pseudonymisation as a risk-reduction measure, not an exemption.

:::caution[Not legal advice]
This page describes technical capabilities. Whether your processing complies with GDPR depends on facts only you and your counsel or DPO can assess.
:::

## Why pseudonymise rather than redact?

Redaction (`[NAME]`, `***`) minimises well but breaks many tasks: a model can't draft a reply to `[NAME]` at `[EMAIL]` or keep three customers apart. Shadow's surrogates are **format-preserving and consistent**: the same person always gets the same fake, emails stay emails, IBANs still pass mod-97. The answer comes back about your real data. For bulk processing, such as reports over customer lists, Shadow switches to typed tokens (`[PERSON_0001]`) so the model treats people as opaque IDs. For values that must never leave in any form, you can opt in to irreversible redaction per kind.

## How to set it up

### 1. Map where personal data meets an LLM

List the tools and flows: chat clients, coding assistants, support tools, internal apps, agents, RAG pipelines. Note which provider each one calls. Shadow covers the provider APIs (OpenAI, Azure OpenAI, Anthropic, Gemini, Vertex AI, Mistral and [others](/shadow/reference/providers/)), not consumer web apps such as `chatgpt.com`.

### 2. Install Shadow where the data originates

```sh
# Laptops and workstations
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme GmbH"
shadow install --apply
```

For services, run it as a [gateway](/shadow/integrate/gateway-mode/) or [Kubernetes sidecar](/shadow/integrate/docker-kubernetes/) next to the app. Shadow runs in the same place as the data, so it introduces no new processor.

### 3. Configure for personal data

```toml
organization = ["Acme GmbH"]
terms = ["org:Globex AG", "person:Jana Novak"]
internal_domains = ["acme.example"]

[persona]
region = "JP"            # where the fake identities "live"; pick one that suits your work

[presidio]
enabled = true           # wider recall: many national ID formats (ES_NIF, IT_FISCAL_CODE, PL_PESEL, ...)
url = "http://127.0.0.1:5002/analyze"
language = "en"          # other languages need a Presidio image with that language's NLP model

[policy.report]
person = "token"
email = "token"
phone = "token"
address = "token"
national_id = "token"
default = "surrogate"

[policy.always]
secret = "redact"

[files]
images = "block"         # images are not OCR'd
```

Built in, Shadow detects emails, phones, names, English and German street addresses, IBANs, cards, IPs, MACs, hostnames, secrets, and national IDs including US SSN and ITIN, UK NINO and NHS number, Aadhaar, Indian PAN and Australian TFN and ABN. [Presidio](/shadow/concepts/detection/#presidio) and the open-weight NER sidecar add multilingual names and more ID formats.

### 4. Test on representative data

```sh
shadow scan sample-ticket-de.txt
shadow scan export.csv --json
```

Keep the results as evidence for your DPIA: what is detected, what is not, and which mitigations cover the gaps.

### 5. Decide on provider-hosted tools

If an agent uses tools that run in the provider's cloud, Shadow by default sends the real **target** of that action (and logs it) so the action works. If that is not acceptable, set:

```toml
[policy.act]
remote_tool_targets = "surrogate"
```

### 6. Document it

Record Shadow in your records of processing and DPIA as a pseudonymisation measure: where it runs, which detectors and terms are configured, the policy, where the vault key lives and who can read it, and the known limits below.

## Example: a customer email in German

What the support agent sends (fictional data):

```text
Antworte höflich: Frau Jana Novak (jana.novak@globex.example, +49 30 1234567,
Hauptstraße 5, IBAN DE89 3704 0044 0532 0130 00) fordert eine Rückerstattung.
```

What the provider sees (persona values differ per machine):

```text
Antworte höflich: Frau Yui Sato (yui.sato@hoshino.jp, +81 3 5412 7780,
Sakura-dori 12, IBAN DE44 5001 0517 5407 3249 31) fordert eine Rückerstattung.
```

The reply arrives addressed to Frau Novak.

## Limits to record in your DPIA

- **Dates are not masked**, including dates of birth.
- **Detection is best effort**, especially for names in languages and formats the detectors have not seen. Measure it: see [benchmarks](/shadow/benchmarks/).
- **Context can identify.** A unique event described in detail can identify a person with every name replaced.
- **Not processed:** images (no OCR), audio, WebSocket and Realtime APIs, SigV4-signed Bedrock requests.
- **Access to the vault** re-identifies everything. Protect the data directory and `vault.key`.

## Related

- [Reversible pseudonymisation for LLMs](/shadow/use-cases/reversible-pseudonymization-for-llms/)
- [HIPAA-safe LLM usage](/shadow/use-cases/hipaa-safe-llm-usage/)
- [Regulated enterprise](/shadow/company-size/regulated-enterprise/), [HR and recruiting](/shadow/industries/hr-recruiting/)
- [Security model](/shadow/reference/security/), [Vault](/shadow/concepts/vault/), [Intent-aware policy](/shadow/concepts/intent-policy/)
- [How Shadow compares](/shadow/compare/)
