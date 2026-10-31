# Benchmarks

Reproducible comparison of Shadow with other open-source PII detection and redaction tools on
the same labelled text. Results and methodology are published at
<https://tiny-bubble-company.github.io/shadow/benchmarks/>.

## Datasets

| File | Source | Use |
| --- | --- | --- |
| `datasets/synthetic-v1.jsonl` | `datasets/generate.py`, seed 20261031 | development set |
| `datasets/heldout-v1.jsonl` | `datasets/generate.py`, seed 7 | held-out test set, never used for tuning |
| `datasets/gretel-finance-1000.jsonl` | `datasets/gretel.py`, test split | third-party test set, never used for tuning |

`generate.py` writes original synthetic text (Faker, ten locales, eleven scenarios: chat
requests, support tickets, CSV exports, HR records, DevOps logs, code, meetings, signatures,
payments, multilingual). Every sensitive value is labelled, and so are public names (companies,
cities, public figures) that a good tool should leave alone.

`gretel-finance-1000.jsonl` is a random sample of 1,000 documents from the test split of
[gretelai/synthetic_pii_finance_multilingual](https://huggingface.co/datasets/gretelai/synthetic_pii_finance_multilingual)
by Gretel.ai, licensed under the [Apache License 2.0](https://www.apache.org/licenses/LICENSE-2.0).
It was modified: documents were sampled and its labels mapped onto this benchmark's (see
`gretel.py`). Rules were developed only against samples of its *train* split.

## Metrics

- **Leak rate**: share of labelled sensitive values still readable in the output (also after
  normalising case and whitespace). Lower is better. This is the headline number.
- **Partial name leak**: names where any part (a first or last name) survives.
- **Public names changed**: labelled public names the tool altered (over-masking).
- **Text kept**: ordinary words outside any label that survive, i.e. how readable the prompt stays.
- **Precision / recall / F1**: span detection, a gold span counts as found when half of it is
  covered.
- **Format kept**: masked emails still look like emails, masked cards still pass Luhn.
- **Round trip**: Shadow only; unmasking the masked text gives back the original exactly.
- **Latency**: per sample on one CPU, after warm-up.

## Running

```sh
python -m venv .venv && . .venv/bin/activate
pip install presidio-analyzer presidio-anonymizer gliner scrubadub llm-guard faker huggingface_hub pyarrow
python -m spacy download en_core_web_lg

python datasets/gretel.py --out datasets/gretel-finance-1000.jsonl   # optional, needs network
python run.py --data datasets/heldout-v1.jsonl --out results-heldout \
    --tools shadow,presidio,gliner,scrubadub,llm_guard
python report.py results-heldout
```

`shadow+presidio` needs a Presidio Analyzer on `127.0.0.1:5002` (see `packaging/presidio/`).
`shadow+ner` and `privacy_filter` need the open-weight sidecar from `ml/serve_hf.py`, on ports
8878 (`--backend ensemble`) and 8879 (`--backend privacy-filter`).
