---
title: "How to redact PDFs, Word and Excel files before uploading them to AI"
description: "Mask names, emails and IDs in DOCX, XLSX, PPTX, CSV and PDF uploads to OpenAI, Anthropic or Gemini automatically, with Office files kept valid."
keywords:
  - redact pdf before chatgpt
  - remove pii from documents for ai
  - mask excel data before uploading to ai
  - anonymize word document for llm
  - redact files openai api
  - document redaction llm
faq:
  - q: "Can Shadow redact a PDF before it is sent to ChatGPT or Claude?"
    a: "Through the API, yes for text. With the default pdf = \"text\", Shadow extracts the PDF's text, masks it and sends it as a text document. Layout and images are lost, and scanned PDFs are not OCR'd. Consumer web apps such as chatgpt.com are not intercepted."
  - q: "Will my Word or Excel file still open after masking?"
    a: "Yes. DOCX, XLSX and PPTX are edited inside their ZIP container, so styles, tables and layout survive. In XLSX only shared strings are changed; numbers and formulas are untouched."
  - q: "Does Shadow mask text in images or screenshots?"
    a: "No. Shadow does not run OCR yet. Images pass through unchanged by default; set images = \"block\" to refuse requests that carry them. Local OCR is on the roadmap."
  - q: "What happens to a file uploaded earlier without Shadow?"
    a: "It is not re-scanned. If a request references a file by an earlier upload id, the provider already holds the original. Upload files through Shadow from the start."
  - q: "How do I check what Shadow will find in a document?"
    a: "Export the document's text and run shadow scan on it. It reports each finding's kind, position and detector without changing anything or touching the vault."
sidebar:
  order: 9
---

To redact documents before uploading them to an AI model, mask the file's text in transit rather than editing copies by hand. Shadow does this for uploads to OpenAI, Anthropic and Gemini APIs: DOCX, XLSX and PPTX are masked in place and stay valid documents, text formats are masked line by line, and PDF text is extracted, masked and sent as text. The answer comes back with real values restored.

## Why are documents riskier than prompts?

A prompt is a sentence or two. An attached contract, export or slide deck carries pages of names, addresses, account numbers and internal codenames, and nobody reads every cell before pressing upload. Manual redaction is slow, inconsistent between people, and usually destroys the structure the model needed.

## Which upload paths does Shadow mask?

| Transport | Examples |
|---|---|
| `multipart/form-data` uploads | OpenAI `POST /v1/files`, `POST /v1/uploads` parts |
| Base64 files inside JSON | OpenAI `input_file.file_data`, Anthropic `document` blocks with a `base64` source, Gemini `inline_data` |
| `data:` URIs | `data:text/csv;base64,...` anywhere in a JSON string |

The format is recognised from the declared MIME type and the file name.

## Which formats are supported?

| Format | Treatment | Result |
|---|---|---|
| Plain text, Markdown | Masked as text | Same file, masked |
| CSV | Masked cell by cell | Same columns and rows |
| JSON | String values masked | Valid JSON |
| HTML, XML | Text masked | Markup intact |
| DOCX | Text runs masked in place | A valid Word document |
| XLSX | Shared strings masked in place | A valid workbook; numbers and formulas untouched |
| PPTX | Slide text runs masked in place | A valid presentation |
| PDF | Text extracted, masked, sent as text (`pdf = "text"`) | The model receives masked text, not the PDF |
| Images | Passed through (`images = "pass"`) or refused (`"block"`) | No OCR |
| Anything else | Passed through | |

Files use the same detectors, persona, vault and [policy](/shadow/concepts/intent-policy/) as prompts. A customer named in the prompt and in the spreadsheet gets the same fake in both, and a customer list attached to a "build a report" request gets typed tokens such as `[PERSON_0001]`.

## How to mask documents before they reach the model

### 1. Install Shadow and start it

```sh
curl -fsSL https://raw.githubusercontent.com/Tiny-Bubble-Company/shadow/main/packaging/install.sh | sh
shadow init --org "Acme GmbH"
shadow run
```

### 2. Set file handling in `shadow.toml`

```toml
[files]
enabled = true          # default
pdf = "text"            # text | block | pass
images = "block"        # no OCR: refuse screenshots and scans instead of sending them unmasked
max_bytes = 52428800    # 50 MiB; larger files are not masked
block_oversize = true   # refuse files above max_bytes instead of forwarding them unmasked
```

If your users need the model to reason about PDF layout (forms, charts), `pdf = "text"` will give worse answers. Prefer `pdf = "block"` for those workflows and send the source DOCX or XLSX instead, which is masked in place.

### 3. Teach Shadow your vocabulary

Customer names, matter or project codenames and colleagues are not something a regex can know:

```sh
shadow terms add "org:Globex" "org:Initech" "Project Falcon" "person:Jane Doe"
```

For better recall on free-text names in long documents, enable the open-weight [NER sidecar](/shadow/concepts/detection/#local-ner-sidecar) or [Presidio](/shadow/concepts/detection/#presidio).

### 4. Test on a representative document

`shadow scan` reads text files. Export the document's text (for example *Save as plain text* in Word, or CSV from Excel) and scan it:

```sh
shadow scan contract.txt            # redacted previews
shadow scan contract.txt --reveal   # full values, on your screen only
shadow scan customers.csv --json    # kind, line, column, detector, score, value
```

Anything missing from the output will reach the provider as is. Add terms until the scan covers what matters.

### 5. Upload through Shadow

Point the SDK at the gateway, or use [proxy mode](/shadow/integrate/proxy-mode/) for tools you don't control:

```python
from openai import OpenAI

client = OpenAI(base_url="http://127.0.0.1:8877/gw/openai/v1")
with open("q3-renewals.xlsx", "rb") as f:
    file = client.files.create(file=f, purpose="user_data")   # masked in transit
```

## Example: a contract review

Clause in the DOCX the lawyer attaches (fictional):

```text
This Agreement is made between Acme GmbH and Globex Ltd. Notices to Jane Doe,
jane.doe@acme.example, Hauptstraße 5, 10115 Berlin. Payments to DE89 3704 0044 0532 0130 00.
```

With `organization = ["Acme GmbH"]` and the term `org:Globex Ltd`, the provider receives this inside the still-valid DOCX (output from `shadow mask` on one machine; your persona's fakes will differ):

```text
This Agreement is made between Lingxi Technology Co., Ltd. and Talari Technology Co., Ltd. Notices to Yong Sun,
xiu.wang@lingxi.cn, 53 Binjiang Avenue, 10115 Berlin. Payments to DE46 0256 5725 3509 8212 53.
```

The street address takes the persona's style while the postcode and city stay, and the IBAN is a different German IBAN that still passes mod-97. Without the `org:Globex Ltd` term, Globex would have been sent as is: add counterparties before you rely on it.

The model's markup of risky clauses comes back about Acme GmbH, Globex Ltd and Jane Doe, so the lawyer works on the real contract. Shadow restores whole fakes, and a fake person's first or last name on its own; if the model shortens a company ("Talari" for Talari Technology Co., Ltd.), that short form stays fake, so ask it to quote party names in full.

## Limitations

:::caution[Read before relying on file masking]
- **No OCR.** Text inside images, screenshots and scanned PDFs is not detected. Block images if they may carry sensitive data.
- **PDF layout is lost** in `text` mode.
- **Embedded objects** in Office files (cached chart values, embedded workbooks, images) are not masked.
- **Split formatting.** A name that is half bold spans two text runs and may not be recognised.
- **Uploads by reference.** A file uploaded earlier and referenced by `file_id` is not re-scanned.
- **Archives** (`.zip`, `.tar`) other than Office formats pass through.
- **Consumer web apps.** Uploads to chatgpt.com or claude.ai in a browser are not intercepted. Shadow covers API traffic.
:::

## Related

- [File masking](/shadow/concepts/file-masking/) and [detection](/shadow/concepts/detection/)
- [Protect customer data in RAG pipelines](/shadow/use-cases/protect-customer-data-in-rag-pipelines/)
- Industries with document-heavy work: [legal](/shadow/industries/legal/), [healthcare](/shadow/industries/healthcare/), [financial services](/shadow/industries/financial-services/)
- [Benchmarks](/shadow/benchmarks/) and [how Shadow compares](/shadow/compare/)
