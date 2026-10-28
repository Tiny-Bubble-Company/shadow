# Shadow ML: optional named-entity detector

Shadow can also be a specialised PyTorch model.

This folder holds an **optional** sidecar: a small, pure-PyTorch BiLSTM tagger
(no HuggingFace, no downloaded weights, no vocab file). It finds names that
Shadow's built-in regex and dictionary detectors miss. Shadow's Rust
`ModelDetector` calls it over localhost HTTP. The Rust build never needs
Python. If you don't enable the sidecar, nothing here runs.

## When to use it

The built-in detectors handle structured PII well: emails, phone numbers, cards,
keys and IPs. Free-text names are harder:

| Label     | Examples                                           |
|-----------|----------------------------------------------------|
| `PERSON`  | `Priya Patel`, `Dr. Okafor`, `Zoë García`          |
| `ORG`     | `Globex GmbH`, `Northwind Pty Ltd`, `Hooli Labs`   |
| `LOC`     | `Kraków`, `221 Baker Street, London`               |
| `PROJECT` | internal codenames such as `Falcon`, `ORION NEXT`  |

Turn it on if your prompts often contain colleagues, customers, partner
companies, sites or internal codenames.

## Install

```sh
cd ml
python -m venv .venv && . .venv/bin/activate
pip install -r requirements.txt        # just torch>=2.1
```

## Train

```sh
python train.py --out model.pt --epochs 8 --synthetic 20000 [--data my.jsonl] [--device cpu]
```

`train.py` generates synthetic, BIO-tagged training data from templates: emails,
chat prompts, code comments, meeting notes and support tickets, filled with
person names from many cultures, organisations, addresses and project codenames.
It holds out 10% of the data and prints the loss and the entity-level
precision, recall and F1 after each epoch. It saves the best checkpoint to one
`.pt` file. Runs are reproducible through `--seed`. A CPU is enough.

To fine-tune on **your own** data, pass one or more JSONL files with `--data`.
Each line looks like this:

```json
{"text": "Ping Ana at Hooli about Falcon", "entities": [{"start": 5, "end": 8, "label": "PERSON"}, {"start": 12, "end": 17, "label": "ORG"}, {"start": 24, "end": 30, "label": "PROJECT"}]}
```

Offsets are Python string indices, which are Unicode codepoints. `end` is
exclusive. Labels can be `PERSON`, `ORG`, `LOC` or `PROJECT`.

## Serve

```sh
python serve.py --model model.pt --host 127.0.0.1 --port 8878
```

The server uses only the standard library (`http.server.ThreadingHTTPServer`).
FastAPI is not needed.

## Enable in `shadow.toml`

```toml
[model_detector]
enabled = true
url = "http://127.0.0.1:8878/detect"
timeout_ms = 300
min_score = 0.6
```

## HTTP contract

`GET /healthz`

```json
{"ok": true, "model": "model.pt", "meta": {"epoch": 8, "f1": 0.97}}
```

`POST /detect` with `Content-Type: application/json`

```json
{"text": "Email Zoë García at Globex GmbH"}
```

Response `200`:

```json
{"entities": [
  {"start": 6, "end": 16, "label": "PERSON", "score": 0.98},
  {"start": 20, "end": 31, "label": "ORG", "score": 0.95}
]}
```

- `start` and `end` are **Unicode codepoint** offsets into `text`, and `end` is
  exclusive. (The Rust side converts them to byte offsets.)
- `label` is one of `PERSON`, `ORG`, `LOC` or `PROJECT`. `score` is a float in
  [0, 1]: the mean tag probability across the span.
- The request body can be at most 2 MB. A larger body gets `413`.
  Malformed JSON or a missing `text` gets `400`.

## Privacy

- The sidecar must bind to loopback only. `serve.py` refuses to bind any
  address that isn't loopback unless you pass `--allow-remote`. Don't pass it:
  prompt text should never leave the machine.
- The server never logs request bodies. It logs only the method, path and
  status.
- Checkpoints load with `torch.load(weights_only=True)`, so a model file can't
  run arbitrary code.

## Failure behaviour

Shadow **fails open** on this detector. If the sidecar is down, slow (past
`timeout_ms`) or returns an error, Shadow skips the model results for that
request. The built-in regex and dictionary detectors still run and still mask
everything they find.

## Tests

```sh
python -m unittest discover -s tests     # or: pytest tests
```

The tokenizer, data and HTTP contract tests run without torch. The model tests
are skipped when torch isn't installed.
