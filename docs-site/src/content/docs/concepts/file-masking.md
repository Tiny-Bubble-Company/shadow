---
title: File masking
seoTitle: "Mask PII in files sent to LLMs: DOCX, XLSX, PDF · Shadow"
description: "How Shadow masks PII in files sent to LLMs: multipart uploads, base64 and data URIs in text, CSV, JSON, DOCX, XLSX, PPTX and PDF formats."
faq:
  - q: "Which file formats does Shadow mask?"
    a: "Plain text, Markdown, CSV, JSON, HTML and XML as text; DOCX, XLSX and PPTX in place; and PDFs, whose text is extracted, masked and sent as text by default."
  - q: "Are images and scanned PDFs masked?"
    a: "No. Shadow does not run OCR yet, so images pass through unchanged by default. Set `[files] images = \"block\"` if they may contain sensitive data."
  - q: "Do Office documents stay valid?"
    a: "Yes. DOCX, XLSX and PPTX files are edited inside their ZIP container, so styles, tables and layout survive. A value split across differently formatted runs may not be recognised."
  - q: "What happens to very large files?"
    a: "Files above `max_bytes` (50 MiB by default) are forwarded unmasked with a warning, or refused when `block_oversize = true`."
---

Shadow masks sensitive data inside files sent to LLMs (multipart uploads, base64 attachments and `data:` URIs) with the same detectors, fakes and policy as prompts.

Prompts are not the only way data reaches a model. Files uploaded for retrieval, attached to a message or embedded as base64 often carry far more sensitive content than the prompt itself. Shadow masks them before they leave the machine.

## Where files are found

| Transport | Examples |
|---|---|
| `multipart/form-data` uploads | OpenAI `POST /v1/files`, `POST /v1/uploads` parts |
| Base64 files inside JSON | OpenAI `input_file.file_data`, Anthropic `document` blocks with a `base64` source, Gemini `inline_data` |
| `data:` URIs | `data:text/csv;base64,...` anywhere in a JSON string |

The format is recognised from the declared MIME type and the file name.

## Formats

| Format | Treatment | Result |
|---|---|---|
| Plain text, Markdown | Masked as text | Same file, masked |
| CSV | Masked as text, cell by cell | Same columns and rows |
| JSON | Masked as JSON (string values) | Valid JSON |
| HTML, XML | Masked as text | Markup intact |
| DOCX | Masked in place: the document's text runs | A valid Word document |
| XLSX | Masked in place: shared strings | A valid workbook; numbers and formulas untouched |
| PPTX | Masked in place: text runs on slides | A valid presentation |
| PDF | Text is extracted, masked and sent **as text** (`pdf = "text"`) | The model receives the masked text instead of the PDF |
| Images | Passed through unchanged by default (`images = "pass"`) | No OCR yet |
| Anything else | Passed through | |

Masking uses the same detectors, persona, vault and [intent policy](/shadow/concepts/intent-policy/) as prompts, so a customer named in the prompt and in an attached spreadsheet gets the same fake in both. A customer list attached to a report request gets typed tokens.

Office documents are edited inside their ZIP container, so styles, tables and layout survive. A value split across differently formatted runs (for example a name that is half bold) may not be recognised.

## PDF options

PDFs cannot be safely edited in place, so Shadow offers three choices:

| `pdf =` | Behaviour |
|---|---|
| `"text"` (default) | Extract the text, mask it, send it as a text document. Layout, images and scanned pages are lost |
| `"block"` | Refuse the request with an error, so a PDF never leaves unmasked |
| `"pass"` | Send the PDF unchanged (not masked) |

## Images

Shadow does not run OCR yet. Images, including screenshots and scanned documents, are passed through unchanged by default. If images may contain sensitive data, block them:

```toml
[files]
images = "block"
```

## Configuration

```toml
[files]
enabled = true          # mask uploaded files (default true)
pdf = "text"            # text | block | pass
images = "pass"         # pass | block
max_bytes = 52428800    # 50 MiB; larger files are not masked
```

## Limitations

- **No OCR.** Text inside images and scanned PDFs is not detected.
- **PDF layout is not preserved** in `text` mode. Questions about the visual layout of a PDF will get worse answers.
- **Files above `max_bytes`** are forwarded unmasked with a warning, or refused when `block_oversize = true`.
- **Uploads by reference.** A file already uploaded to the provider earlier (by `file_id`) is not re-scanned. Upload files through Shadow from the start.
- **Embedded objects** in Office files (charts' cached values, embedded workbooks, images) are not masked.
- **Archives** (`.zip`, `.tar`) other than Office formats are passed through.

Check what Shadow finds in a document's text with `shadow scan` on an exported text version before you rely on it.
