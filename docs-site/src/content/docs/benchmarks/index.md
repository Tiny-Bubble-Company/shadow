---
title: "PII redaction benchmark: Shadow vs Presidio, GLiNER, OpenAI Privacy Filter, LLM Guard"
seoTitle: "PII redaction benchmark: Shadow vs Presidio, GLiNER & more"
description: "Shadow's rules leak 3.08% of PII on a held-out set vs 20.74% for Presidio. Leak rate, recall, precision and latency for 6 open-source tools."
sidebar:
  label: Results
  order: 0
keywords:
  - PII redaction benchmark
  - PII detection benchmark
  - PII masking accuracy
  - presidio benchmark
  - gliner pii benchmark
  - openai privacy filter benchmark
  - llm guard anonymize benchmark
  - best open source PII redaction
  - PII leak rate
faq:
  - q: "Which open-source tool leaks the least PII in LLM prompts?"
    a: "On the held-out test set, Shadow's built-in rules leaked 3.08% of sensitive values and 1.33% with the Presidio detector, and 0.09% with the open-weight NER sidecar. The best standalone tools were OpenAI Privacy Filter (11.01%), GLiNER multi PII (13.28%) and Microsoft Presidio (20.74%). On the third-party Gretel finance set GLiNER alone leaked least (17.61%) and Shadow's rules leaked 30.94%."
  - q: "Is Shadow more accurate than Microsoft Presidio?"
    a: "On both test sets Shadow's built-in rules leak fewer sensitive values than Presidio's default analyzer (3.08% vs 20.74% held-out, 30.94% vs 43.39% on Gretel), with higher precision. Shadow can also run Presidio as an extra detector, which lowers its leak rate further."
  - q: "How fast is Shadow compared with other PII tools?"
    a: "Shadow's built-in rules took 7.9 ms per document at the median on one Apple M3 CPU, including masking and the round-trip check. Presidio took 8.8 ms, LLM Guard 110 ms, OpenAI Privacy Filter 227 ms and GLiNER 405 ms on the same CPU. With the NER sidecar on CPU, Shadow took 1400 ms; a GPU cuts that sharply."
  - q: "Are these benchmark results biased towards Shadow?"
    a: "Partly, and the page says where. The synthetic test sets come from Shadow's own generator, and its rules were tuned on a separate development set from the same generator. That is why the page also reports a third-party dataset (Gretel synthetic PII finance, Apache-2.0) that was never used for tuning, on which GLiNER alone leaks less than Shadow's rules alone."
  - q: "Can I reproduce the benchmark?"
    a: "Yes. The datasets, label mappings, runner and report script are in the bench folder of the Shadow repository on GitHub, under Apache-2.0, and each run's summary.json is committed next to the code."
  - q: "Which tools can restore the original values after the LLM answers?"
    a: "Only Shadow and LLM Guard's Anonymize/Deanonymize pair are reversible here. Shadow's built-in rules restored the exact original text in 100% of held-out samples. Presidio, GLiNER, OpenAI Privacy Filter and scrubadub replace values one way."
---

**Shadow's built-in rules leak 3.08% of sensitive values on a held-out test set, against 11.01% for OpenAI Privacy Filter, 13.28% for GLiNER and 20.74% for Microsoft Presidio, at 7.9 ms per document.** This page measures how much personal data and secrets six open-source tools let through to an LLM, how much harmless text they destroy, how well they find each type of value, and how fast they are, on three labelled datasets. Everything is reproducible from the [`bench/`](https://github.com/Tiny-Bubble-Company/shadow/tree/main/bench) folder. Results from the run on 1 November 2026.

:::note[TL;DR]
- On the **held-out test set** (1,500 documents, 7,800 sensitive values), Shadow's built-in rules leak **3.08%** at **7.9 ms** per document. Adding the Presidio detector brings that to **1.33%**, and adding the open-weight NER sidecar brings it to **0.09%**. The best standalone tool, OpenAI Privacy Filter, leaks **11.01%**; Presidio leaks **20.74%**.
- Shadow's rules changed **0%** of public names (companies, cities and public figures that should stay readable) and restored the original text exactly in **100%** of samples.
- On the **third-party Gretel finance set**, which Shadow was never tuned on, every tool does worse. **GLiNER alone (17.61%) beats Shadow's rules alone (30.94%)**, mainly on company and person names; Shadow with Presidio leaks 23.34%. With the NER sidecar, Shadow leaks **12.07%** on Gretel.
:::

## Why leak rate is the headline number

When a prompt goes to a hosted model, a missed value is a value disclosed. So the main metric counts **sensitive values still readable in the output**, after normalising case, spacing and punctuation, rather than span-level F1, which rewards finding *most* of a value. A tool that masks "Jane" but leaves "Okafor" has leaked a surname; F1 calls that a partial hit.

Leak rate alone would reward masking everything, so the benchmark also measures:

- **Public names changed**: labelled names that should stay readable (Microsoft, Berlin, Taylor Swift). Masking them makes answers worse.
- **Text kept**: ordinary words outside any label that survive. Low values mean the model gets a mangled prompt.
- **Precision, recall, F1**: span detection; a gold span counts as found when at least half of it is covered.
- **Recall by type**: the share of each type of value (emails, names, IBANs, ...) a tool found. Precision per type is not shown because each tool labels its findings differently and a false positive has no gold type.
- **Round trip**: for tools that can reverse their masking, whether unmasking gives back the original exactly.
- **Latency**: median time per document on one CPU, after warm-up.

## Held-out test set

1,500 documents, 7,800 labelled sensitive values, ten locales and eleven scenarios. Generated with a different seed from the development set and never used for tuning.

| Tool | Leak rate ↓ | Partial name leak ↓ | Public names changed ↓ | Text kept ↑ | Precision | Recall | F1 | Round trip | p50 latency |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **Shadow + open-weight NER sidecar** | 0.09% | 0.3% | 1.98% | 86.44% | 78.8% | 99.6% | 87.99 | 100% | 1400.13 ms |
| **Shadow + Presidio detector** | 1.33% | 5.18% | 0% | 94.79% | 95.68% | 97.77% | 96.72 | 100% | 24.18 ms |
| **Shadow (built-in rules)** | 3.08% | 10.8% | 0% | 98.12% | 99.96% | 96.46% | 98.18 | 100% | 7.93 ms |
| OpenAI Privacy Filter | 11.01% | 4.57% | 0.61% | 93.12% | 86.55% | 85.87% | 86.21 | not reversible | 227.03 ms |
| GLiNER multi PII | 13.28% | 1.36% | 49.28% | 88.88% | 74.6% | 86.58% | 80.14 | not reversible | 404.73 ms |
| Microsoft Presidio | 20.74% | 22.76% | 26.01% | 90.21% | 79.39% | 72.64% | 75.87 | not reversible | 8.79 ms |
| LLM Guard (Anonymize) | 29.26% | 15.73% | 2.29% | 97.87% | n/a | n/a | n/a | 100% | 109.99 ms |
| scrubadub | 67.38% | 92.36% | 0% | 98.14% | 100% | 34.37% | 51.16 | not reversible | 0.37 ms |

### Leak rate by type of value

| Label | Shadow | Shadow + Presidio detector | Shadow + open-weight NER sidecar | Microsoft Presidio | OpenAI Privacy Filter | GLiNER multi PII | LLM Guard | scrubadub |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| address | 2.15% | 0.59% | 0% | 14.48% | 3.13% | 16.83% | 92.56% | 99.61% |
| credit card | 0% | 0% | 0% | 2.8% | 0% | 0% | 18.18% | 23.78% |
| date of birth | 0% | 0% | 0% | 0% | 0% | 0% | 100% | 100% |
| email | 0% | 0% | 0% | 0% | 0.28% | 5.8% | 0% | 0% |
| hostname | 0% | 0% | 0% | 0% | 1.42% | 46.81% | 0% | 0% |
| iban | 0% | 0% | 0% | 6.23% | 0.33% | 0% | 9.18% | 98.69% |
| ip | 0% | 0% | 0% | 0% | 58.87% | 21.99% | 0% | 100% |
| national id | 0% | 0% | 0% | 22.92% | 0% | 88.89% | 88.19% | 88.89% |
| org | 21.61% | 10.44% | 1.28% | 61.17% | 55.68% | 3.66% | 81.87% | 100% |
| person | 5.58% | 2.21% | 0% | 22.86% | 4.02% | 0.1% | 1.31% | 100% |
| phone | 0% | 0% | 0% | 3.68% | 0.49% | 13.82% | 16.19% | 54.7% |
| secret | 0% | 0% | 0% | 74.17% | 0% | 1.28% | 51.15% | 98.98% |
| url | 0% | 0% | 0% | 0% | 30.83% | 80.45% | 100% | 0% |
| username | 0% | 0% | 0% | 89.43% | 69.04% | 56.76% | 85.26% | 100% |

### Detection recall by type of value

| Label | Shadow | Shadow + Presidio detector | Shadow + open-weight NER sidecar | Microsoft Presidio | OpenAI Privacy Filter | GLiNER multi PII | scrubadub |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| address | 97.65% | 97.65% | 99.41% | 9.59% | 60.47% | 80.63% | 0% |
| credit card | 100% | 100% | 100% | 96.5% | 98.6% | 100% | 76.22% |
| date of birth | 100% | 100% | 100% | 100% | 100% | 100% | 0% |
| email | 100% | 100% | 100% | 100% | 99.72% | 92.33% | 100% |
| hostname | 100% | 100% | 100% | 100% | 95.04% | 47.52% | 100% |
| iban | 100% | 100% | 100% | 90.16% | 98.36% | 100% | 1.31% |
| ip | 100% | 100% | 100% | 100% | 31.91% | 78.01% | 0% |
| national id | 100% | 100% | 100% | 75.69% | 100% | 11.11% | 11.11% |
| org | 73.44% | 79.3% | 95.05% | 23.26% | 31.87% | 93.22% | 0% |
| person | 94.02% | 97.54% | 99.95% | 74.82% | 95.13% | 99.85% | 0% |
| phone | 100% | 100% | 100% | 93.05% | 99.18% | 86.18% | 45.3% |
| secret | 100% | 100% | 100% | 7.42% | 99.49% | 97.7% | 0.51% |
| url | 100% | 100% | 100% | 100% | 54.51% | 19.55% | 100% |
| username | 100% | 100% | 100% | 42.26% | 54.79% | 57.74% | 34.64% |

### Leak rate by kind of text

| Scenario | Shadow | Shadow + Presidio detector | Shadow + open-weight NER sidecar | Microsoft Presidio | OpenAI Privacy Filter | GLiNER multi PII | LLM Guard | scrubadub |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| chat request | 7.28% | 2.43% | 0.13% | 21.33% | 16.99% | 1.66% | 32.69% | 76.12% |
| code | 0% | 0% | 0% | 49.28% | 5.44% | 26.56% | 56.32% | 59.36% |
| csv | 0.38% | 0.14% | 0% | 8.73% | 0.58% | 4.99% | 9.88% | 52.09% |
| devops | 0% | 0% | 0% | 40.9% | 49.05% | 49.29% | 54.49% | 66.67% |
| hr record | 0.42% | 0.14% | 0% | 10.56% | 0.14% | 26.94% | 57.22% | 89.44% |
| meeting | 15.93% | 8.14% | 0.18% | 33.98% | 18.76% | 0.18% | 17.52% | 80% |
| multilingual | 1.46% | 0.49% | 0% | 12.34% | 3.41% | 1.14% | 20.13% | 68.18% |
| payment | 9.01% | 4.5% | 1.2% | 34.23% | 15.32% | 3.3% | 37.24% | 100% |
| signature | 6.56% | 2.6% | 0.15% | 18.02% | 12.82% | 18.78% | 31.76% | 72.37% |
| support ticket | 0% | 0% | 0% | 6.82% | 0.35% | 0% | 6.99% | 55.24% |

What stands out:

- **Structured identifiers are solved by rules.** Shadow's rules found every email, phone number, IBAN, card number, national ID, IP address, URL, hostname, username and secret in this set. Model-only tools miss many of these: GLiNER found 11.11% of national IDs, OpenAI Privacy Filter 31.91% of IP addresses, Presidio 7.42% of secrets.
- **Names are where tools differ.** Shadow's rules still leak 21.61% of company names and 5.58% of person names, mostly names in free text with no surrounding cue. Model-based detectors close that gap.
- **GLiNER over-masks.** It changed 49.28% of public names, so prompts about real companies or places lose their meaning.
- LLM Guard reports no spans, so precision, recall and recall by type are not available for it.

## Third-party test set: Gretel synthetic PII finance

1,000 documents sampled from the test split of [gretelai/synthetic_pii_finance_multilingual](https://huggingface.co/datasets/gretelai/synthetic_pii_finance_multilingual) (Apache-2.0), with 5,077 sensitive values in seven languages: loan agreements, SWIFT messages, customer records, privacy notices and similar financial documents. It was written by a third party and **never used to tune Shadow**; detector rules were developed only on samples of its separate train split. Gretel does not label public names, so over-masking is not measured here.

| Tool | Leak rate ↓ | Partial name leak ↓ | Public names changed ↓ | Text kept ↑ | Precision | Recall | F1 | Round trip | p50 latency |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| **Shadow + open-weight NER sidecar** | 12.07% | 21.71% | n/a | 89% | 30.18% | 88.79% | 45.05 | 100% | 5541.43 ms |
| GLiNER multi PII | 17.61% | 24.24% | n/a | 92.96% | 35.25% | 85.44% | 49.91 | not reversible | 2303.33 ms |
| **Shadow + Presidio detector** | 23.34% | 30.42% | n/a | 94.34% | 47.2% | 76.82% | 58.47 | 100% | 64.42 ms |
| **Shadow (built-in rules)** | 30.94% | 42.44% | n/a | 98.83% | 89.08% | 69.75% | 78.24 | 100% | 7.02 ms |
| Microsoft Presidio | 43.39% | 46.09% | n/a | 91.09% | 17.77% | 49.26% | 26.12 | not reversible | 22.72 ms |
| OpenAI Privacy Filter | 48.59% | 42.07% | n/a | 98.89% | 62.03% | 51.23% | 56.12 | not reversible | 433.72 ms |
| LLM Guard (Anonymize) | 62.5% | 60.74% | n/a | 98.37% | n/a | n/a | n/a | 100% | 409.19 ms |
| scrubadub | 91.06% | 99.67% | n/a | 98.67% | 56.41% | 9.1% | 15.67 | not reversible | 0.64 ms |

### Leak rate by type of value (Gretel)

| Label | Shadow | Shadow + Presidio detector | Shadow + open-weight NER sidecar | Microsoft Presidio | OpenAI Privacy Filter | GLiNER multi PII | LLM Guard | scrubadub |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| address | 10.16% | 6.66% | 1.52% | 26.52% | 30.72% | 7.94% | 93.57% | 99.88% |
| credit card | 18.18% | 18.18% | 0% | 31.82% | 22.73% | 22.73% | 22.73% | 81.82% |
| date of birth | 12.5% | 12.5% | 7.14% | 7.14% | 7.14% | 5.36% | 100% | 100% |
| email | 2.7% | 2.4% | 0% | 2.4% | 34.23% | 48.05% | 3% | 3% |
| iban | 5.26% | 5.26% | 0% | 21.05% | 7.89% | 21.05% | 7.89% | 100% |
| ip | 4.55% | 2.27% | 4.55% | 2.27% | 18.18% | 56.82% | 2.27% | 90.91% |
| national id | 27.78% | 24.44% | 1.11% | 10% | 6.67% | 67.78% | 61.11% | 74.44% |
| org | 46.37% | 42.16% | 14.54% | 84.33% | 95.64% | 6.06% | 94.99% | 99.92% |
| person | 38.79% | 24.85% | 18.39% | 38.65% | 37.76% | 21.57% | 47.45% | 99.81% |
| phone | 3.98% | 3.48% | 2.49% | 11.94% | 29.35% | 5.97% | 4.48% | 56.22% |
| secret | 35.71% | 35.71% | 14.29% | 80.95% | 19.05% | 23.81% | 71.43% | 95.24% |
| username | 45% | 45% | 45% | 55% | 30% | 30% | 65% | 80% |

### Detection recall by type of value (Gretel)

| Label | Shadow | Shadow + Presidio detector | Shadow + open-weight NER sidecar | Microsoft Presidio | OpenAI Privacy Filter | GLiNER multi PII | scrubadub |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| address | 89.14% | 90.42% | 98.13% | 25% | 58.53% | 92.99% | 0% |
| credit card | 81.82% | 81.82% | 90.91% | 68.18% | 77.27% | 77.27% | 18.18% |
| date of birth | 89.29% | 89.29% | 92.86% | 92.86% | 94.64% | 96.43% | 0% |
| email | 96.1% | 96.1% | 97.6% | 96.7% | 67.57% | 59.46% | 97% |
| iban | 94.74% | 94.74% | 100% | 73.68% | 92.11% | 78.95% | 0% |
| ip | 95.45% | 97.73% | 95.45% | 97.73% | 77.27% | 45.45% | 9.09% |
| national id | 73.33% | 77.78% | 97.78% | 90% | 92.22% | 34.44% | 27.78% |
| org | 54.68% | 59.21% | 87.24% | 13.25% | 5.01% | 96.53% | 0.48% |
| person | 62.61% | 75.99% | 83.06% | 65.33% | 65.51% | 82.22% | 0.28% |
| phone | 96.02% | 96.02% | 100% | 87.06% | 70.15% | 96.52% | 43.78% |
| secret | 64.29% | 64.29% | 85.71% | 4.76% | 78.57% | 69.05% | 4.76% |
| username | 55% | 60% | 55% | 45% | 85% | 85% | 20% |

### Leak rate by language (Gretel)

| Scenario | Shadow | Shadow + Presidio detector | Shadow + open-weight NER sidecar | Microsoft Presidio | OpenAI Privacy Filter | GLiNER multi PII | LLM Guard | scrubadub |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| dutch | 46.1% | 32.2% | 26.83% | 43.41% | 55.61% | 33.17% | 71.46% | 94.15% |
| english | 21.42% | 17.62% | 5.79% | 40.02% | 47.53% | 12.65% | 60.42% | 89.78% |
| france | 37.97% | 30.72% | 11.88% | 51.01% | 39.71% | 19.71% | 52.17% | 88.7% |
| german | 39.13% | 24.78% | 21.3% | 42.17% | 48.7% | 22.39% | 62.61% | 91.96% |
| italian | 36.59% | 24.83% | 9.31% | 42.35% | 48.78% | 13.3% | 71.62% | 91.8% |
| spanish | 39.6% | 27.82% | 12.03% | 50.88% | 44.11% | 17.54% | 62.66% | 91.98% |
| swedish | 43.89% | 34.93% | 27.51% | 52.18% | 58.52% | 29.26% | 64.63% | 94.76% |

This is the honest picture of out-of-distribution performance:

- **Every tool leaks more** on these documents than on the synthetic sets. Long legal and financial prose, company names in many languages and names without cues are harder for everyone.
- **GLiNER alone beats Shadow's rules alone** on overall leak rate (17.61% vs 30.94%), driven by company names (6.06% vs 46.37% leaked) and person names. It pays for that with 35.25% precision, and took 2303 ms per document on CPU.
- **Shadow's rules stay ahead on identifiers** (phones 3.98%, IBANs 5.26%, emails 2.7% leaked) and keep the most text intact (98.83%) with 89.08% precision.
- Part of Shadow's company-name leak is deliberate: its public allowlist keeps well-known company names readable, and Gretel labels every company as sensitive.
## Development set

The development set (1,500 documents, 7,637 values, seed 20261031) is what Shadow's rules were tuned against, so its numbers flatter Shadow and are shown only for completeness. They match the held-out set closely (Shadow rules 3.16%, with Presidio 1.31%, with the NER sidecar 0.1%; Privacy Filter 11.27%, GLiNER 13.75%, Presidio 20.65%, LLM Guard 29.8%, scrubadub 68.08%), which suggests the rules are not overfitted to individual examples, though they share the generator's style.

## Methodology

### Datasets

| Dataset | Documents | Sensitive values | Source | Use |
| --- | ---: | ---: | --- | --- |
| `heldout-v1` | 1,500 | 7,800 | Shadow generator, seed 7 | test, never tuned on |
| `gretel-finance-1000` | 1,000 | 5,077 | Gretel.ai, test split, Apache-2.0 | test, never tuned on |
| `synthetic-v1` | 1,500 | 7,637 | Shadow generator, seed 20261031 | development |

The Shadow generator writes original text with [Faker](https://faker.readthedocs.io/) in ten locales and eleven scenarios: chat requests to an assistant, support tickets, CSV exports, HR records, DevOps logs, source code, meeting notes, email signatures, payments and multilingual messages. Every sensitive value is labelled, and so are public names a good tool should leave alone. No real personal data is used anywhere.

Gretel labels are mapped onto the benchmark's: `name`, `first_name` and `last_name` to person, `company` to org, `street_address` to address, `ssn`, `passport_number` and `driver_license_number` to national ID, `password` and `api_key` to secret, and so on. Labels with no counterpart (times, plain dates, internal customer IDs, BIC codes) become *ignore* spans: masking them is not a false positive and leaving them is not a leak. The full mapping is in [`bench/datasets/gretel.py`](https://github.com/Tiny-Bubble-Company/shadow/blob/main/bench/datasets/gretel.py).

### Tools and configuration

| Tool | Version | Configuration |
| --- | --- | --- |
| Shadow (built-in rules) | `main`, 1 November 2026 | default detectors: regex, checksums, dictionaries, names, fields, addresses, public allowlist |
| Shadow + Presidio detector | Presidio Analyzer 2.2.362 container | Shadow's built-in rules plus Presidio over HTTP |
| Shadow + open-weight NER sidecar | `ml/serve_hf.py --backend ensemble` | built-in rules plus OpenAI Privacy Filter and GLiNER multi PII v1 |
| Microsoft Presidio | presidio-analyzer 2.2.358, spaCy `en_core_web_lg` | default recognisers, `replace` operator |
| OpenAI Privacy Filter | `openai/privacy-filter` | sidecar `--backend privacy-filter`, all predicted spans masked |
| GLiNER multi PII | `urchade/gliner_multi_pii-v1`, gliner 0.2.21 | PII labels, threshold 0.5 |
| LLM Guard | llm-guard 0.3.16 | `Anonymize` scanner with its default vault, `Deanonymize` for round trip |
| scrubadub | 2.0.1 | default detectors |

Every tool ran with its defaults or documented recommended settings, on the same text, scored by the same code. No tool was given the gold labels. Shadow is scored on plain text masking, without a prompt context, so its intent policy (which passes the recipient of a message through) does not apply here.

### Hardware

One Apple M3 (8 cores), CPU only, PyTorch 2.15 and transformers 4.51. Apple's GPU backend crashed on this macOS version, so all models ran on CPU; on a GPU the model-based tools, and Shadow with its sidecar, are many times faster. Latency is the median per document after warm-up, measured inside each tool's process and including Shadow's unmask check.

### Limitations

- The synthetic sets come from Shadow's own generator. Their formats and phrasing are the ones Shadow's rules were written for, which favours Shadow. Treat the Gretel numbers as the fairer comparison.
- Gretel's documents are synthetic too, generated by an LLM, and only financial.
- Leak rate checks whether the exact value survives. A tool that changes one character of a value is counted as having masked it.
- Commercial services (Private AI, Nightfall, AWS Comprehend, Google Sensitive Data Protection, Azure AI Language) are not included because their terms or pricing make public, reproducible runs impractical. See the [comparison pages](/shadow/compare/) for what their documentation says.

## Reproduce

```sh
git clone https://github.com/Tiny-Bubble-Company/shadow && cd shadow/bench
python -m venv .venv && . .venv/bin/activate
pip install presidio-analyzer presidio-anonymizer gliner scrubadub llm-guard faker huggingface_hub pyarrow
python -m spacy download en_core_web_lg

python run.py --data datasets/heldout-v1.jsonl --out results-heldout \
    --tools shadow,presidio,gliner,scrubadub,llm_guard
python report.py results-heldout
```

`shadow+presidio` needs a Presidio Analyzer on `127.0.0.1:5002`; `shadow+ner` and `privacy_filter` need the [open-weight sidecar](/shadow/concepts/detection/) on ports 8878 and 8879. The [bench README](https://github.com/Tiny-Bubble-Company/shadow/blob/main/bench/README.md) has the details, and each run's `summary.json` is committed next to the code. `python run.py ... --rescore` scores saved outputs again without re-running the tools.

## Related

- [How Shadow detects sensitive data](/shadow/concepts/detection/)
- [Shadow vs Presidio](/shadow/compare/presidio/), [vs GLiNER](/shadow/compare/gliner-pii/), [vs OpenAI Privacy Filter](/shadow/compare/openai-privacy-filter/), [vs LLM Guard](/shadow/compare/llm-guard/)
- [Open-source PII redaction tools compared](/shadow/compare/open-source-pii-redaction-tools/)
