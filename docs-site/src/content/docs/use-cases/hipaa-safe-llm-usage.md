---
title: "How to use LLMs with patient data more safely under HIPAA"
description: "Keep PHI out of OpenAI, Azure OpenAI and Claude prompts: Shadow masks patient identifiers locally, supports minimum necessary, and restores answers."
keywords:
  - hipaa safe llm
  - hipaa compliant chatgpt alternative
  - phi redaction llm
  - minimum necessary ai
  - mask patient data before openai
  - hipaa ai prompts
faq:
  - q: "Is there a HIPAA-compliant way to use ChatGPT?"
    a: "Compliance comes from your whole programme, not one tool. The common pattern is an API-based service under a business associate agreement, used through an approved client, with technical safeguards such as Shadow keeping identifiers out of prompts. Consumer ChatGPT without a BAA is generally not suitable for PHI."
  - q: "Does masking with Shadow de-identify PHI under the Safe Harbor method?"
    a: "No. Safe Harbor requires removing all 18 identifier types. Shadow does not mask dates and does not detect medical record numbers or health plan IDs by default, so treat masked text as PHI with reduced risk, not as de-identified data."
  - q: "Do we still need a BAA with the AI provider?"
    a: "Yes, whenever PHI may reach that provider. Shadow reduces what is sent, but detection is best effort and dates, unknown IDs or context can still identify patients."
  - q: "Can Shadow mask PHI in clinical documents and spreadsheets?"
    a: "It masks the text of DOCX, XLSX and PPTX files in place and sends PDF text masked. Scanned documents and images are not OCR'd, so block images if they may contain PHI."
  - q: "Where are the real patient identifiers stored?"
    a: "In an encrypted vault (ChaCha20-Poly1305) on the machine or pod running Shadow. Nothing is sent to a Shadow service; there isn't one."
sidebar:
  order: 6
---

To use LLMs with patient data more safely, send the provider only what the task needs. Shadow does that technically: it runs on your own machines or pods, replaces patient and clinician names, emails, phone numbers, addresses, SSNs and other identifiers with realistic fakes before a request reaches OpenAI, Azure OpenAI, Anthropic or Gemini, and restores them in the answer. It supports the HIPAA minimum necessary standard; it does not de-identify data on its own, and it does not replace a business associate agreement (BAA).

## What does HIPAA mean for LLM prompts?

| HIPAA concept | What it means for AI use | Where Shadow fits |
|---|---|---|
| Minimum necessary (45 CFR 164.502(b)) | Use and disclose only the PHI needed for the purpose | Most writing and summarising tasks need no real identifiers. Shadow removes the detected ones automatically |
| Business associates | A vendor that receives PHI on your behalf needs a BAA | Still required if PHI may reach the vendor. Shadow reduces exposure, it does not change the relationship |
| Security Rule safeguards | Technical safeguards proportionate to risk | A control on a specific data flow, on infrastructure you run, with no telemetry |
| De-identification (45 CFR 164.514) | Safe Harbor (remove 18 identifiers) or expert determination | Shadow covers several identifier types but not all. It is not a Safe Harbor method |

:::caution[Not legal advice]
This page describes technical capabilities, not legal conclusions. Using Shadow does not make an organisation HIPAA compliant. Involve your privacy officer, security officer and counsel.
:::

## Which identifiers does Shadow mask?

| Safe Harbor identifier | Shadow today |
|---|---|
| Names | Built-in name detector; `person:` terms; NER sidecar or Presidio for better recall |
| Geographic subdivisions smaller than a state | English and German street addresses built in; Presidio and the NER sidecar add locations. City names alone are often not masked |
| **Dates (except year)** | **Not masked** |
| Phone numbers, emails | Built in |
| Fax numbers | Detected when formatted like phone numbers |
| SSNs | Built in (US SSN and ITIN; also UK NHS number, NINO) |
| **Medical record numbers, health plan beneficiary numbers, account numbers** | **Not detected by built-ins.** Presidio covers `US_MBI` and some account formats; the NER sidecar finds some account-style numbers; add known values as terms |
| Certificate or licence numbers | Presidio `MEDICAL_LICENSE`, `US_DRIVER_LICENSE` |
| Vehicle and device identifiers, serial numbers | Not detected unless added as terms |
| URLs, IP addresses | Internal hostnames and IPs built in; public URLs left alone |
| Biometric identifiers, full-face photos | Not handled; images are not OCR'd or analysed |
| Any other unique code | Terms, or a custom detector if you [embed Shadow in Rust](/shadow/integrate/rust/#custom-detectors) |

There is no custom regex setting in `shadow.toml`. Be explicit about these gaps in your risk analysis.

## How to set it up

### 1. Choose the AI service and sign the BAA

Pick a provider API offered under a BAA (for example an enterprise or cloud-platform offering). Shadow then sits in front of that API. It covers provider APIs such as `api.openai.com`, `*.openai.azure.com`, `api.anthropic.com`, Gemini and Vertex AI, not consumer web apps such as `chatgpt.com`.

### 2. Install Shadow where PHI originates

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "St Mary's Health Partners"
shadow install --apply        # workstation: background service, CA trust, proxy env
```

For an internal scribe or summarisation service, run Shadow as a [gateway](/shadow/integrate/gateway-mode/) or [sidecar](/shadow/integrate/docker-kubernetes/) and set the app's base URL, for example a custom `azure` route.

### 3. Configure for PHI

```toml
organization = ["St Mary's Health Partners"]
terms = ["org:Northside Imaging", "person:Dr Amara Okafor", "term:Ward 7B"]
internal_domains = ["stmarys.example", "ehr.stmarys.internal"]

[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"
entities = ["PERSON", "PHONE_NUMBER", "EMAIL_ADDRESS", "US_SSN", "US_MBI", "MEDICAL_LICENSE", "US_DRIVER_LICENSE", "LOCATION"]

[policy.act]
targets = "keep"         # default: a patient you write to is sent as written; "surrogate" hides them

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
pdf = "text"
images = "block"

[proxy.routes]
azure = "https://my-resource.openai.azure.com"
```

Run Presidio locally: `docker run -d --name presidio -p 127.0.0.1:5002:3000 mcr.microsoft.com/presidio-analyzer`. For multilingual names, add the open-weight NER sidecar (`python serve_hf.py --backend ensemble --host 127.0.0.1 --port 8878` in `ml/`, then `[model_detector] enabled = true`).

### 4. Handle dates and record numbers in the workflow

Because dates and MRNs are not masked, design workflows around them: strip date-of-birth and MRN columns from exports, use relative times ("day 3 of admission") in prompts, and add known identifiers as terms where practical.

### 5. Test, then keep testing

```sh
shadow scan synthetic-discharge-letter.txt --reveal
```

Use synthetic or already-approved test documents. Re-run scans when templates or EHR exports change.

## Example: drafting a patient letter

What the clinician sends (fictional data):

```text
Draft a follow-up letter to Maria Lopez, 12 Elm Road, Springfield, phone (217) 555-0142, about her MRI results with Dr Amara Okafor.
```

What the provider sees (persona values differ):

```text
Draft a follow-up letter to Maria Lopez, 12 Elm Road, Springfield, phone (217) 555-0142, about her MRI results with Dr Tao Zhu.
```

The goal is to write to Maria Lopez, so she is the recipient: her name, address and phone pass through as written, each keep is logged, and the model greets the right patient. Dr Amara Okafor comes after "about", so she is masked and restored in the draft ("Dr Zhu" on its own is restored too, because fake first and last names are restored on their own).

This means the recipient's identifiers reach the provider, under your BAA, whenever you ask the model to write to a patient. If that is not acceptable, set `[policy.act] targets = "surrogate"`: the provider then writes to a stand-in patient and Shadow restores Maria's name and address in the draft. The trade-off is that the model may personalise the letter for the wrong person, so review it before sending. Leave identifiers the letter does not need, such as an SSN, out of the prompt altogether.

## Example: a report over patient rows

Ask for "a breakdown of readmissions by ward" and attach a CSV of 800 rows. Shadow classifies the request as `report`, turns names, emails, phones and addresses into typed tokens (`[PERSON_0001]`), and restores them in the answer. The model counts and groups without inventing facts about fake patients.

## Rollout notes

- Fleet rollout uses your MDM or configuration management; there is no central admin console yet.
- Logs record intent and counts per treatment, never values. An optional local audit log is on the [roadmap](/shadow/project/roadmap/).
- Protect the data directory: anyone who can read `vault.key` can re-identify masked text. Use full-disk encryption.

## Related

- [Healthcare](/shadow/industries/healthcare/) and [pharma and life sciences](/shadow/industries/pharma-life-sciences/)
- [Redact documents before uploading to AI](/shadow/use-cases/redact-documents-before-uploading-to-ai/)
- [GDPR-compliant LLM usage](/shadow/use-cases/gdpr-compliant-llm-usage/)
- [Detection](/shadow/concepts/detection/), [File masking](/shadow/concepts/file-masking/), [Security model](/shadow/reference/security/)
- [Benchmarks](/shadow/benchmarks/) and [how Shadow compares](/shadow/compare/)
