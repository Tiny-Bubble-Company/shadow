---
title: "Generative AI controls for regulated enterprises: evidence-ready PII masking with Shadow"
description: "For banks, insurers, health systems and public bodies: deploying Shadow as a documented AI data control, with risk assessment, testing and change control."
keywords:
  - regulated enterprise generative ai controls
  - ai risk assessment pii masking
  - dpia llm pseudonymisation
  - model risk ai data minimisation
  - ai third party risk dora
  - iso 42001 ai controls
faq:
  - q: "Can Shadow be used as a control in a DPIA or AI risk assessment?"
    a: "Yes, as a technical measure that pseudonymises detected identifiers before data reaches an AI provider. Describe it accurately: detection is best effort, some data types are not covered, and the provider still receives masked content. Test it on your own data and record the results."
  - q: "What evidence can Shadow produce for auditors?"
    a: "Today: the versioned configuration, shadow hosts output showing what is intercepted, per-request log lines with intent and masking counts (never values), a log entry for every value sent in clear text, and your own shadow scan test results. A local append-only audit log is on the roadmap."
  - q: "Does using Shadow remove the AI provider from our third-party risk assessment?"
    a: "No. The provider still processes masked prompts and may receive context that identifies people or deals. Shadow lowers the sensitivity of what it receives, which can change the risk rating, but contracts, DPAs or BAAs and oversight still apply."
  - q: "Is Shadow itself a third party we need to assess?"
    a: "It is open-source software you run, not a service. It has no cloud component and no telemetry, so no data goes to its maintainers. Assess it as an internal software dependency: source review, pinned versions, your own builds and vulnerability tracking."
  - q: "Is Shadow validated for GxP or certified for any standard?"
    a: "No. Shadow has no certifications or validation packages. If your use requires computerised system validation, you would validate your configuration and intended use yourself."
sidebar:
  order: 6
---

In a regulated enterprise, a privacy control is only as useful as the evidence behind it. Shadow can be a documented technical measure for AI use: it pseudonymises detected identifiers before prompts, files and tool results reach an AI provider, keeps the mapping in your environment, and restores real values in the answer. This page covers how to deploy it so that risk, compliance and audit teams can rely on it, and where its limits must be written down.

## What regulators and auditors will ask

| Question | How to answer it with Shadow |
|---|---|
| What data reaches the AI provider? | Masked content. Detected identifiers are replaced; see the coverage table in your assessment |
| How do you know the control works? | `shadow scan` results on representative samples, recorded per release and per config change |
| What is not covered? | The documented gaps: dates, internal IDs unless configured, images, consumer web chat, Realtime APIs |
| Who can reverse the pseudonyms? | Only holders of the data directory: `vault.key` on the endpoint or in a Kubernetes secret |
| What happens on failure? | Traffic fails closed. Optional detectors fail open to the built-ins, with a warning |
| Is anything sent in clear text by design? | Only `keep` decisions: the recipient when a user asks the model to write to or act on someone, and provider-hosted tool targets, each logged with kind and reason; both configurable off |
| How is the control changed? | `shadow.toml` in version control, reviewed, deployed by pipeline or MDM |

## How Shadow supports common requirements

| Requirement | Shadow's contribution |
|---|---|
| GDPR Art. 5(1)(c), 25, 32 | Data minimisation and pseudonymisation by design for AI processing; mapping kept separately and encrypted |
| HIPAA minimum necessary | Reduces identifiers in prompts; does not de-identify (dates are not masked) |
| PCI DSS v4.0.1 | Card numbers detected (Luhn) and replaced before leaving; helps keep AI providers out of PAN flows |
| DORA (since 17 Jan 2025) | Lowers the sensitivity of data shared with ICT third-party AI services; supports your third-party risk measures |
| EU AI Act | Supports data governance for AI systems you deploy; high-risk obligations (Annex III) apply from 2 Dec 2027 under the 2026 Digital Omnibus |
| ISO/IEC 27001, ISO/IEC 42001, SOC 2 | A documented technical control with configuration and test evidence |
| NYDFS Part 500, GLBA Safeguards Rule | A safeguard for nonpublic information sent to AI service providers |

:::caution[Not legal advice]
Shadow is a technical control. It does not make any system compliant, and none of the above is legal advice. Pseudonymised data remains personal data for you, because you hold the key (GDPR Recital 26). Confirm your obligations with your legal, privacy and compliance teams.
:::

## Recommended deployment

Start from the [enterprise architecture](/shadow/company-size/enterprise/) and tighten it:

```toml
organization = ["Acme Bank plc"]
terms = ["org:Globex Holdings", "Project Falcon"]
internal_domains = ["acmebank.example", "acmebank.internal"]

[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"
entities = ["PERSON", "EMAIL_ADDRESS", "PHONE_NUMBER", "IBAN_CODE", "CREDIT_CARD", "US_SSN", "US_BANK_NUMBER", "UK_NINO"]

[policy.act]
targets = "surrogate"               # fake message recipients too; the model writes to a stand-in
remote_tool_targets = "surrogate"   # never send action targets in clear; hosted actions may fail

[policy.always]
secret = "redact"
national_id = "token"

[files]
pdf = "text"            # or "block" where layout-dependent PDFs are not allowed
images = "block"        # no OCR, so images never leave
```

Additional controls:

- **Build from source** in your pipeline from a reviewed, pinned commit; sign the binary and image with your own keys.
- **Dedicated service accounts** on servers, with the hardened systemd unit and a data directory readable only by that account.
- **Egress allow-listing**: providers are reachable only from Shadow instances, or through an egress proxy set as `upstream_proxy`.
- **Full-disk encryption** on every endpoint that runs Shadow.
- **Key handling**: `vault.key` and `ca/ca.key` treated as secrets; never in images or repositories.
- **Prefer local tool execution** for agents, so actions on personal data never need `keep`.

## Testing and evidence

1. Build a test corpus of representative, synthetic or approved samples for each workload: prompts, tickets, documents, logs.
2. Run `shadow scan --json` on each sample and record detections by kind.
3. Compare against expected identifiers; record misses and decide: add terms, enable Presidio or the NER sidecar, change workflow, or accept and document.
4. Repeat on every Shadow upgrade and every configuration change, and keep the results with the change record.
5. During rollout, review the per-request log lines (intent, counts, keeps) for unexpected patterns.

Public results on standard datasets are on the [benchmarks](/shadow/benchmarks/) page; your own corpus matters more.

## Rollout checklist

- [ ] DPIA or AI risk assessment that names Shadow as a measure, with its limits
- [ ] Data flow diagram showing where masking happens and where keys live
- [ ] Reviewed configuration per workload and population
- [ ] Test corpus and recorded `shadow scan` results
- [ ] Decision record for `targets` (message recipients), `remote_tool_targets`, secrets and files
- [ ] Egress controls verified by a negative test (direct provider access fails)
- [ ] Change-control process for `shadow.toml` and Shadow upgrades
- [ ] AI provider contracts (DPA, BAA where relevant) still in place
- [ ] Owner for the control and a review cycle

## Gaps to record in your assessment

:::note[Current limitations and roadmap]
- **No audit log yet.** Logs contain intent and counts, never values; an optional local append-only audit log is a [roadmap](/shadow/project/roadmap/) "Next" item.
- **No central console, RBAC or fleet inventory.**
- **No certifications, attestations or validation packages.**
- **Not detected:** dates, most internal identifiers (account, claim, policy, case numbers) unless listed as terms or found by Presidio or the NER sidecar; text in images and scanned PDFs.
- **Not processed:** WebSocket and Realtime APIs, gRPC, SigV4-signed Bedrock requests (forwarded unmasked with a warning).
- **Context can identify.** Masking values does not mask meaning.
:::

## Related

- [Financial services](/shadow/industries/financial-services/), [insurance](/shadow/industries/insurance/), [healthcare](/shadow/industries/healthcare/), [government](/shadow/industries/government-public-sector/)
- [GDPR](/shadow/use-cases/gdpr-compliant-llm-usage/) and [HIPAA](/shadow/use-cases/hipaa-safe-llm-usage/) guides
- [Security model](/shadow/reference/security/)
- [Intent-aware policy](/shadow/concepts/intent-policy/)
- [How Shadow compares](/shadow/compare/)
