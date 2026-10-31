#!/usr/bin/env python3
"""Convert a sample of Gretel's synthetic financial PII dataset (Apache-2.0) into the benchmark
format. It is written by a third party, in seven languages, and was never used to tune Shadow,
so it is the out-of-distribution check next to our own synthetic sets.

    pip install huggingface_hub pyarrow
    python gretel.py --n 1000 --out gretel-finance-1000.jsonl

https://huggingface.co/datasets/gretelai/synthetic_pii_finance_multilingual

Gretel labels are mapped onto the benchmark's. Labels with no counterpart (times, plain dates,
internal customer or employee IDs, BIC codes, ...) become "ignore" spans: they count neither as
leaks nor as false positives.
"""
from __future__ import annotations

import argparse
import json
import random

LABELS = {
    "name": "PERSON", "first_name": "PERSON", "last_name": "PERSON",
    "company": "ORG",
    "street_address": "ADDRESS",
    "email": "EMAIL",
    "phone_number": "PHONE",
    "date_of_birth": "DATE_OF_BIRTH",
    "iban": "IBAN",
    "ipv4": "IP", "ipv6": "IP",
    "ssn": "NATIONAL_ID", "passport_number": "NATIONAL_ID", "driver_license_number": "NATIONAL_ID",
    "credit_card_number": "CREDIT_CARD",
    "password": "SECRET", "api_key": "SECRET",
    "user_name": "USERNAME",
}


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=1000)
    ap.add_argument("--seed", type=int, default=20261031)
    ap.add_argument("--out", default="gretel-finance-1000.jsonl")
    # "test" for reported results; "train" only for developing detectors.
    ap.add_argument("--split", choices=["test", "train"], default="test")
    a = ap.parse_args()

    import pyarrow.parquet as pq
    from huggingface_hub import hf_hub_download

    path = hf_hub_download("gretelai/synthetic_pii_finance_multilingual", f"data/{a.split}-00000-of-00001.parquet",
                           repo_type="dataset")
    rows = pq.read_table(path).to_pylist()
    random.Random(a.seed).shuffle(rows)
    n = 0
    with open(a.out, "w", encoding="utf-8") as f:
        for r in rows:
            text = r["generated_text"]
            spans, ignore = [], []
            for e in json.loads(r["pii_spans"]):
                value = text[e["start"]:e["end"]]
                if not value.strip():
                    continue
                item = {"start": e["start"], "end": e["end"], "value": value}
                if e["label"] in LABELS:
                    spans.append({**item, "label": LABELS[e["label"]]})
                else:
                    ignore.append({**item, "label": e["label"]})
            if not spans:
                continue
            f.write(json.dumps({"id": f"g{r['index']}", "scenario": r["language"].lower(), "text": text,
                                "spans": spans, "public": [], "ignore": ignore}, ensure_ascii=False) + "\n")
            n += 1
            if n == a.n:
                break
    print(f"wrote {n} documents to {a.out}")


if __name__ == "__main__":
    main()
