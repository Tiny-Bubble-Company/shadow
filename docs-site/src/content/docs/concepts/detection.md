---
title: Detection
description: "How Shadow detects PII and secrets: checksummed patterns, secrets, dictionary terms, local machine facts, an optional NER model and Microsoft Presidio."
---

Detection decides *what* is sensitive. Shadow layers several sources and merges their findings into one set of non-overlapping spans.

Test detection on your own content at any time. Nothing is changed or sent:

```sh
shadow scan notes.txt            # redacted previews
shadow scan notes.txt --reveal   # full values
shadow scan notes.txt --json     # kind, line, column, detector, score, value
```

## Built-in detectors

All built-in detectors are on by default. Turn individual ones off under `[detectors]`.

| Toggle | Finds | How |
|---|---|---|
| `email` | Email addresses | Pattern |
| `phone` | International numbers (`+49 30 1234567`) and US formats (`(415) 555-0100`, `415-555-0100`) | Pattern |
| `person` | Person names: a known first name followed by a capitalised surname, or an honorific (`Mr`, `Ms`, `Dr`, `Prof`...) plus a name | Pattern + name list |
| `address` | Street addresses in English (`221 Baker Street`) and German (`Hauptstraße 5`) styles | Pattern |
| `iban` | IBANs | Pattern + **mod-97** check |
| `credit_card` | Payment card numbers, 13 to 19 digits with optional separators | Pattern + **Luhn** check |
| `national_id` | US SSN and ITIN, UK National Insurance and NHS numbers, Indian Aadhaar and PAN, Australian TFN and ABN | Pattern and checksum |
| `ip` | IPv4 and full IPv6 addresses | Pattern |
| `mac` | MAC addresses | Pattern |
| `hostname` | Hostnames in URLs and bare domains that are not well-known public services; anything under `internal_domains` or an internal TLD (`.internal`, `.corp`, `.lan`, `.local`) | Pattern + allow list |
| `secret` | OpenAI, Anthropic, OpenRouter, GitHub, GitLab, Slack, AWS, Google, Groq, xAI, Hugging Face, npm and Stripe live keys; JWTs; PEM private keys; bearer tokens; `password=`, `api_key:`, `client_secret=`-style assignments | Pattern; placeholders like `your-key`, `changeme`, `${VAR}` are ignored |
| `path` | Usernames inside home paths: `/Users/<u>`, `/home/<u>`, `C:\Users\<u>` | Pattern |
| `local_facts` | This machine's OS user name, hostname, git `user.name` and `user.email`, and local domain | Discovered at startup |

Public domains (well-known services such as `github.com` or `google.com`) are on a built-in allow list so that ordinary URLs in prompts are left alone. Add your own public domains with `allow_domains`.

## Dictionary terms

Detectors cannot know your company's codenames, customers or colleagues. Tell Shadow:

```toml
organization = ["Acme GmbH"]                         # first one maps to the persona company
terms = ["Project Falcon", "Nightjar", "org:Globex", "person:Jane Doe"]
internal_domains = ["acme.com", "acme.internal"]
```

or at runtime (stored in `terms.json`):

```sh
shadow terms add "Project Falcon" "org:Globex" "person:Jane Doe" "hostname:build01"
shadow terms list
```

An optional prefix sets the kind, which decides the shape of the fake: `person:`, `org:`, `term:`, `hostname:`, `email:`, `username:`. Without a prefix, a term that ends in a legal suffix (`GmbH`, `Ltd`, `Inc`...) is treated as an organisation, and anything else as a term. For organisations, the bare name without the suffix (`Acme`) is protected too. Dictionary matching is case-insensitive and respects word boundaries.

## Allow lists

```toml
allow_domains = ["docs.mycompany-public.io"]   # never masked as hostnames
allow_terms = ["Jane Austen"]                  # never masked, whatever detects them
```

## Local NER sidecar

Regex and dictionaries are precise for structured values but miss free-text names, addresses and organisations they have not seen. The `ml/` directory contains an optional sidecar that adds a named-entity model. It is a standard-library HTTP server on loopback, and there are two ways to run it.

### Open-weight models (recommended)

`serve_hf.py` runs open-weight models, all Apache-2.0, downloaded from Hugging Face on first start:

| Backend | Model | Finds |
| --- | --- | --- |
| `privacy-filter` | [OpenAI Privacy Filter](https://huggingface.co/openai/privacy-filter) | people, addresses, private dates, emails, phones, URLs, account numbers, secrets |
| `gliner` | [GLiNER multi PII](https://huggingface.co/urchade/gliner_multi_pii-v1) | multilingual names, organisations, addresses, IDs, usernames |
| `ensemble` (default) | both, merged | the union of the two |

```sh
cd ml
python -m venv .venv && . .venv/bin/activate
pip install -r requirements-hf.txt
python serve_hf.py --backend ensemble --host 127.0.0.1 --port 8878
```

It uses CUDA or Apple MPS when available. If MPS crashes on your macOS version, pass `--device cpu`. On CPU the ensemble takes roughly a second per prompt, so raise `timeout_ms` to match. Shadow applies its public-name allowlist to whatever the model returns, so "London" or "Microsoft" stay readable.

### Small from-scratch model

`serve.py` runs a tiny pure-PyTorch model (a BiLSTM with a character CNN, no downloaded weights) that you train yourself. It recognises `PERSON`, `ORG`, `LOC` and `PROJECT`, is fast on CPU, and suits air-gapped machines:

```sh
pip install -r requirements.txt
python train.py --out model.pt --epochs 8 --synthetic 20000
python serve.py --model model.pt --host 127.0.0.1 --port 8878
```

You can fine-tune it on your own JSONL data with `--data`.

### Enable it in Shadow

```toml
[model_detector]
enabled = true
url = "http://127.0.0.1:8878/detect"   # must be loopback
timeout_ms = 300                         # raise to ~2000 for the open-weight models on CPU
min_score = 0.6
```

Both servers refuse to bind a non-loopback address unless you pass `--allow-remote`, and they never log request bodies. Shadow **fails open** on the sidecar: if it is down or slow, the built-in detectors still run. See `ml/README.md` for the HTTP contract, and the [benchmarks](/shadow/benchmarks/) for what each option adds.

## Presidio

[Microsoft Presidio](https://microsoft.github.io/presidio/) is the leading open-source PII framework. Its Analyzer has more than 50 recognizers, including spaCy and transformer-based NER and many national ID formats. Shadow can call a Presidio Analyzer as an additional detector.

Presidio only **detects**. Shadow still does everything else: the persona, the fakes, the vault, the policy and the restoration. You get Presidio's recall with Shadow's reversible, format-preserving translation.

Run the analyzer locally:

```sh
docker run -d --name presidio -p 127.0.0.1:5002:3000 mcr.microsoft.com/presidio-analyzer
```

or with Compose next to Shadow:

```yaml
services:
  presidio:
    image: mcr.microsoft.com/presidio-analyzer
    ports: ["127.0.0.1:5002:3000"]
```

Enable it:

```toml
[presidio]
enabled = true
url = "http://127.0.0.1:5002/analyze"
language = "en"
min_score = 0.5
entities = []        # empty = all entities Presidio supports; or e.g. ["PERSON", "IBAN_CODE", "US_PASSPORT"]
```

Behaviour:

- Presidio's entity types are mapped to the closest Shadow kind (for example `PERSON` → person, `EMAIL_ADDRESS` → email, `PHONE_NUMBER` → phone, `IBAN_CODE` → iban), so the fake keeps the right shape. Findings below `min_score` are dropped.
- Its findings are merged with Shadow's own. Where spans overlap, the highest-confidence span wins and the result is a non-overlapping set.
- If Presidio is unreachable or returns an error, Shadow **fails open**: it logs a warning and continues with its built-in detectors.
- Keep Presidio on loopback or inside the same pod. Shadow sends it the plaintext of each prompt.

## How findings are merged

All sources produce findings (byte range, kind, score, detector). Shadow then:

1. drops findings that are on `allow_terms`, whitespace-only, or already a known fake;
2. resolves overlaps into a non-overlapping set, preferring longer spans and higher-priority kinds (secrets first, then organisation terms, emails, financial IDs, names), and confidence where sources disagree;
3. hands the result to the [policy](/shadow/concepts/intent-policy/) to choose a treatment per finding.

## Custom detectors

In Rust, implement the `Detector` trait and pass it to `Shadow::open_with_detectors`. See [Embed in Rust](/shadow/integrate/rust/#custom-detectors).
