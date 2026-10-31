"""Head-to-head benchmark: Shadow vs other open-source PII protection tools on the same prompts.

Each tool masks every sample; metrics are computed the same way for all of them from what would
actually be sent to the model:

- leak rate:          real values still present in the masked text (digits normalised for numbers)
- partial leak rate:  for names, any part (first or last name) still present
- over-masking:       public names (Microsoft, Paris, Satya Nadella…) that were changed
- text kept:          ordinary words that survived (utility: the model still gets the request)
- detection F1:       span overlap with the labels, for tools that report spans
- format kept:        an email is still an email, an IBAN still passes mod-97, a card still passes Luhn
- round trip:         the original comes back exactly after unmasking (tools that can reverse)
- latency:            ms per sample (p50 / p95), warm

    python run.py --data datasets/synthetic-v1.jsonl --tools shadow,presidio,gliner,scrubadub,llm_guard
"""

from __future__ import annotations

import argparse
import json
import os
import re
import statistics
import subprocess
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent

NUMERIC = {"PHONE", "CREDIT_CARD", "IBAN", "NATIONAL_ID"}


def norm(s: str) -> str:
    return re.sub(r"[^0-9a-z]", "", s.lower())


# ------------------------------------------------------------------ tools


def run_shadow(samples: list[dict], presidio: str | None = None, model: str | None = None) -> list[dict]:
    env = dict(os.environ)
    if presidio:
        env["SHADOW_BENCH_PRESIDIO"] = presidio
    if model:
        env["SHADOW_BENCH_MODEL"] = model
    subprocess.run(["cargo", "build", "-q", "--release", "-p", "shadow-core", "--example", "bench"], cwd=ROOT, check=True)
    exe = ROOT / "target" / "release" / "examples" / "bench"
    inp = "".join(json.dumps({"id": s["id"], "text": s["text"]}) + "\n" for s in samples)
    out = subprocess.run([str(exe)], input=inp, capture_output=True, text=True, env=env, check=True).stdout
    res = [json.loads(l) for l in out.splitlines() if l.strip()]
    for r in res:
        r["ms"] = r.pop("micros") / 1000
    return res


def placeholder_mask(text: str, spans: list[dict]) -> str:
    out, last = [], 0
    for s in sorted(spans, key=lambda x: x["start"]):
        if s["start"] < last:
            continue
        out.append(text[last : s["start"]])
        out.append(f"<{s['label'].upper()}>")
        last = s["end"]
    out.append(text[last:])
    return "".join(out)


def run_presidio(samples: list[dict]) -> list[dict]:
    from presidio_analyzer import AnalyzerEngine
    from presidio_anonymizer import AnonymizerEngine

    analyzer, anonymizer = AnalyzerEngine(), AnonymizerEngine()
    analyzer.analyze(text="warm up John Smith", language="en")
    res = []
    for s in samples:
        t0 = time.perf_counter()
        found = analyzer.analyze(text=s["text"], language="en")
        masked = anonymizer.anonymize(text=s["text"], analyzer_results=found).text
        ms = (time.perf_counter() - t0) * 1000
        res.append({"id": s["id"], "spans": [{"start": r.start, "end": r.end, "label": r.entity_type} for r in found], "masked": masked, "ms": ms})
    return res


GLINER_LABELS = ["person", "email", "phone number", "address", "organization", "iban", "credit card number", "social security number",
                 "national id number", "ip address", "username", "password", "api key", "date of birth", "url"]


def cpu_only() -> None:
    """Keep PyTorch tools off Apple's MPS backend, which crashes on some macOS versions and would
    make latencies incomparable anyway: every tool runs on the same CPU."""
    import torch

    original = torch.backends.mps.is_available

    def unavailable() -> bool:
        return False

    unavailable.__wrapped__ = original  # transformers inspects this
    torch.backends.mps.is_available = unavailable


def run_gliner(samples: list[dict]) -> list[dict]:
    cpu_only()
    from gliner import GLiNER

    model = GLiNER.from_pretrained("urchade/gliner_multi_pii-v1")
    model.predict_entities("warm up John Smith", GLINER_LABELS)
    res = []
    for s in samples:
        t0 = time.perf_counter()
        text = s["text"]
        spans = []
        # GLiNER has a ~384-token window: run it over line chunks.
        offset = 0
        for chunk in re.split(r"(?<=\n)", text):
            if chunk.strip():
                for e in model.predict_entities(chunk, GLINER_LABELS, threshold=0.5):
                    spans.append({"start": offset + e["start"], "end": offset + e["end"], "label": e["label"]})
            offset += len(chunk)
        ms = (time.perf_counter() - t0) * 1000
        res.append({"id": s["id"], "spans": spans, "masked": placeholder_mask(text, spans), "ms": ms})
    return res


def run_privacy_filter(samples: list[dict]) -> list[dict]:
    """OpenAI Privacy Filter on its own, through the sidecar (`ml/serve_hf.py --backend privacy-filter`)."""
    import urllib.request

    url = os.environ.get("PRIVACY_FILTER_URL", "http://127.0.0.1:8879/detect")
    res = []
    for s in samples:
        t0 = time.perf_counter()
        req = urllib.request.Request(url, data=json.dumps({"text": s["text"]}).encode(),
                                     headers={"content-type": "application/json"})
        with urllib.request.urlopen(req, timeout=60) as r:
            spans = json.load(r)["entities"]
        ms = (time.perf_counter() - t0) * 1000
        res.append({"id": s["id"], "spans": spans, "masked": placeholder_mask(s["text"], spans), "ms": ms})
    return res


def run_scrubadub(samples: list[dict]) -> list[dict]:
    import scrubadub

    scrubber = scrubadub.Scrubber()
    res = []
    for s in samples:
        t0 = time.perf_counter()
        filth = list(scrubber.iter_filth(s["text"]))
        masked = scrubber.clean(s["text"])
        ms = (time.perf_counter() - t0) * 1000
        res.append({"id": s["id"], "spans": [{"start": f.beg, "end": f.end, "label": f.type} for f in filth], "masked": masked, "ms": ms})
    return res


def run_llm_guard(samples: list[dict]) -> list[dict]:
    cpu_only()
    from llm_guard.input_scanners import Anonymize
    from llm_guard.output_scanners import Deanonymize
    from llm_guard.vault import Vault

    res = []
    vault = Vault()
    scanner = Anonymize(vault, language="en")
    deanon = Deanonymize(vault)
    scanner.scan("warm up John Smith")
    for s in samples:
        t0 = time.perf_counter()
        masked, _, _ = scanner.scan(s["text"])
        ms = (time.perf_counter() - t0) * 1000
        restored, _, _ = deanon.scan(s["text"], masked)
        res.append({"id": s["id"], "spans": None, "masked": masked, "roundtrip": restored == s["text"], "ms": ms})
    return res


TOOLS = {
    "shadow": run_shadow,
    "shadow+presidio": lambda s: run_shadow(s, presidio=os.environ.get("PRESIDIO_URL", "http://127.0.0.1:5002/analyze")),
    # Shadow with its optional open-weight NER sidecar (ml/serve_hf.py) running on :8878.
    "shadow+ner": lambda s: run_shadow(s, model=os.environ.get("SHADOW_NER_URL", "http://127.0.0.1:8878/detect")),
    "presidio": run_presidio,
    "gliner": run_gliner,
    "privacy_filter": run_privacy_filter,
    "scrubadub": run_scrubadub,
    "llm_guard": run_llm_guard,
}

# ------------------------------------------------------------------ metrics

EMAIL_RE = re.compile(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+")


def luhn(d: str) -> bool:
    ds = [int(c) for c in d if c.isdigit()]
    if len(ds) < 12:
        return False
    s = 0
    for i, x in enumerate(reversed(ds)):
        if i % 2:
            x *= 2
            x -= 9 if x > 9 else 0
        s += x
    return s % 10 == 0


def leaked(span: dict, masked: str, masked_norm: str) -> bool:
    v = span["value"]
    if span["label"] in NUMERIC:
        n = norm(v)
        return len(n) >= 6 and n in masked_norm
    return v.lower() in masked.lower()


def evaluate(samples: list[dict], results: list[dict]) -> dict:
    by_id = {r["id"]: r for r in results}
    tot = {"spans": 0, "leaked": 0, "names": 0, "partial": 0, "public": 0, "public_changed": 0, "words": 0, "words_kept": 0,
           "tp": 0, "fp": 0, "fn": 0, "emails": 0, "emails_kept": 0, "cards": 0, "cards_valid": 0, "roundtrip": 0, "rt_n": 0}
    per_label: dict[str, list[int]] = {}
    per_scenario: dict[str, list[int]] = {}
    lat = []
    has_spans = True
    for s in samples:
        r = by_id[s["id"]]
        masked = r["masked"]
        mn = norm(masked)
        lat.append(r["ms"])
        for sp in s["spans"]:
            l = leaked(sp, masked, mn)
            tot["spans"] += 1
            tot["leaked"] += l
            per_label.setdefault(sp["label"], [0, 0])
            per_label[sp["label"]][0] += 1
            per_label[sp["label"]][1] += l
            per_scenario.setdefault(s["scenario"], [0, 0])
            per_scenario[s["scenario"]][0] += 1
            per_scenario[s["scenario"]][1] += l
            if sp["label"] == "PERSON":
                tot["names"] += 1
                parts = [p for p in re.split(r"\s+", sp["value"]) if len(p) >= 3 and p.lower() not in {"mr.", "mrs.", "dr.", "von", "van", "der", "del", "les", "des"}]
                tot["partial"] += any(re.search(rf"(?<!\w){re.escape(p)}(?!\w)", masked) for p in parts)
            if sp["label"] == "EMAIL":
                tot["emails"] += 1
            if sp["label"] == "CREDIT_CARD":
                tot["cards"] += 1
        tot["emails_kept"] += min(len(EMAIL_RE.findall(masked)), sum(1 for sp in s["spans"] if sp["label"] == "EMAIL"))
        cards = re.findall(r"(?:\d[ -]?){12,19}", masked)
        tot["cards_valid"] += min(sum(1 for c in cards if luhn(c)), sum(1 for sp in s["spans"] if sp["label"] == "CREDIT_CARD"))
        for p in s.get("public", []):
            tot["public"] += 1
            tot["public_changed"] += p["value"] not in masked
        # Ordinary words: outside every labelled span, 4+ letters.
        covered = [(x["start"], x["end"]) for x in s["spans"] + s.get("public", []) + s.get("ignore", [])]
        for m in re.finditer(r"[A-Za-zÀ-ÿ]{4,}", s["text"]):
            if any(a <= m.start() < b for a, b in covered):
                continue
            tot["words"] += 1
            tot["words_kept"] += re.search(rf"(?<!\w){re.escape(m.group())}(?!\w)", masked) is not None
        if r.get("spans") is None:
            has_spans = False
        else:
            pred = r["spans"]
            for g in s["spans"]:
                hit = any(min(g["end"], p["end"]) - max(g["start"], p["start"]) >= 0.5 * (g["end"] - g["start"]) for p in pred)
                tot["tp" if hit else "fn"] += 1
            # Spans a dataset marks "ignore" (labels with no counterpart here) are not false positives.
            for p in pred:
                if not any(min(g["end"], p["end"]) - max(g["start"], p["start"]) > 0 for g in s["spans"] + s.get("ignore", [])):
                    tot["fp"] += 1
        if "roundtrip" in r:
            tot["rt_n"] += 1
            tot["roundtrip"] += bool(r["roundtrip"])
    pct = lambda a, b: round(100 * a / b, 2) if b else None
    lat.sort()
    p = tot["tp"] / (tot["tp"] + tot["fp"]) if tot["tp"] + tot["fp"] else 0
    rc = tot["tp"] / (tot["tp"] + tot["fn"]) if tot["tp"] + tot["fn"] else 0
    return {
        "leak_rate": pct(tot["leaked"], tot["spans"]),
        "partial_name_leak": pct(tot["partial"], tot["names"]),
        "public_over_masking": pct(tot["public_changed"], tot["public"]),
        "text_kept": pct(tot["words_kept"], tot["words"]),
        "detection": {"precision": round(100 * p, 2), "recall": round(100 * rc, 2), "f1": round(200 * p * rc / (p + rc), 2) if p + rc else 0} if has_spans else None,
        "emails_kept_as_emails": pct(tot["emails_kept"], tot["emails"]),
        "cards_still_luhn_valid": pct(tot["cards_valid"], tot["cards"]),
        "round_trip": pct(tot["roundtrip"], tot["rt_n"]),
        "latency_ms": {"p50": round(statistics.median(lat), 2), "p95": round(lat[int(0.95 * (len(lat) - 1))], 2)},
        "leak_by_label": {k: pct(v[1], v[0]) for k, v in sorted(per_label.items())},
        "leak_by_scenario": {k: pct(v[1], v[0]) for k, v in sorted(per_scenario.items())},
        "samples": len(samples),
        "values": tot["spans"],
    }


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--data", default=str(HERE / "datasets" / "synthetic-v1.jsonl"))
    ap.add_argument("--tools", default="shadow,presidio,gliner,scrubadub,llm_guard")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--out", default=str(HERE / "results"))
    a = ap.parse_args()
    samples = [json.loads(l) for l in open(a.data, encoding="utf-8") if l.strip()]
    if a.limit:
        samples = samples[: a.limit]
    out = Path(a.out)
    out.mkdir(parents=True, exist_ok=True)
    summary = {}
    for tool in a.tools.split(","):
        t0 = time.time()
        results = TOOLS[tool](samples)
        with open(out / f"{tool}.jsonl", "w", encoding="utf-8") as f:
            for r in results:
                f.write(json.dumps(r, ensure_ascii=False) + "\n")
        summary[tool] = evaluate(samples, results)
        print(f"{tool:16s} leak {summary[tool]['leak_rate']}%  ({time.time() - t0:.0f}s)", flush=True)
        # Saved after every tool, so one crashing tool doesn't lose the others' results.
        prev = out / "summary.json"
        merged = json.loads(prev.read_text()) if prev.exists() else {}
        merged.update(summary)
        prev.write_text(json.dumps(merged, indent=2, ensure_ascii=False))


if __name__ == "__main__":
    main()
