#!/usr/bin/env python3
"""Shadow NER sidecar backed by open-weight models (all Apache-2.0):

  privacy-filter   openai/privacy-filter: people, addresses, private dates, emails, phones,
                   URLs, account numbers, secrets. Strongest on English.
  gliner           urchade/gliner_multi_pii-v1: multilingual names, organisations, addresses
                   and more, zero-shot labels.
  ensemble         both, merged (default). Shadow resolves overlaps and applies its public-name
                   allowlist, so over-eager spans on "Microsoft" or "London" are dropped there.

    pip install -r requirements-hf.txt
    python serve_hf.py --backend ensemble --host 127.0.0.1 --port 8878

Same contract as serve.py, so Shadow's `[model_detector]` works unchanged:
POST /detect {"text": "..."} -> {"entities": [{"start", "end", "label", "score"}]}
Offsets are Unicode code-point indices, end exclusive. GET /healthz -> {"ok": true, ...}.
"""
from __future__ import annotations

import argparse
import json
import re
import threading
import time
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

from serve import is_loopback

MAX_BODY = 4 * 1024 * 1024
CHUNK = 1800  # characters per model call; split at line boundaries

# Shadow labels understood by the Rust ModelDetector.
PF_LABELS = {
    "private_person": "PERSON", "private_address": "ADDRESS", "private_date": "DATE", "private_email": "EMAIL",
    "private_phone": "PHONE", "private_url": "URL", "account_number": "ACCOUNT", "secret": "SECRET",
}
GLINER_LABELS = {
    "person": "PERSON", "organization": "ORG", "address": "ADDRESS", "email": "EMAIL", "phone number": "PHONE",
    "date of birth": "DOB", "username": "USERNAME", "password": "SECRET", "national id number": "NATIONAL_ID",
    "social security number": "NATIONAL_ID", "passport number": "NATIONAL_ID", "iban": "IBAN",
    "credit card number": "CREDIT_CARD", "bank account number": "ACCOUNT",
}


def device(choice: str = "auto"):
    import torch

    if choice != "auto":
        return choice

    if torch.cuda.is_available():
        return "cuda"
    if getattr(torch.backends, "mps", None) and torch.backends.mps.is_available():
        return "mps"
    return "cpu"


def chunks(text: str):
    """Yield (offset, piece) of at most ~CHUNK characters, cut at line or sentence ends."""
    start = 0
    while start < len(text):
        end = min(len(text), start + CHUNK)
        if end < len(text):
            cut = max(text.rfind("\n", start, end), text.rfind(". ", start, end))
            if cut > start + CHUNK // 2:
                end = cut + 1
        yield start, text[start:end]
        start = end


class PrivacyFilter:
    name = "openai/privacy-filter"

    def __init__(self, dev: str) -> None:
        import torch
        from transformers import AutoModelForTokenClassification, AutoTokenizer

        self.torch = torch
        self.tok = AutoTokenizer.from_pretrained(self.name)
        self.model = AutoModelForTokenClassification.from_pretrained(self.name, torch_dtype=torch.float32).to(dev).eval()
        self.dev = dev
        self.id2label = self.model.config.id2label

    def __call__(self, text: str) -> list[dict]:
        out = []
        for off, piece in chunks(text):
            enc = self.tok(piece, return_offsets_mapping=True, return_tensors="pt", truncation=True, max_length=4096)
            offsets = enc.pop("offset_mapping")[0].tolist()
            with self.torch.no_grad():
                logits = self.model(**{k: v.to(self.dev) for k, v in enc.items()}).logits[0]
            probs = logits.float().softmax(-1).cpu()
            scores, ids = probs.max(-1)
            cur = None  # [label, start, end, scores]
            for (s, e), i, sc in zip(offsets, ids.tolist(), scores.tolist()):
                if s == e:
                    continue
                tag = self.id2label[i]
                if tag == "O":
                    if cur:
                        out.append(cur)
                        cur = None
                    continue
                prefix, label = tag.split("-", 1)
                if prefix in ("B", "S") or not cur or cur[0] != label:
                    if cur:
                        out.append(cur)
                    cur = [label, off + s, off + e, [sc]]
                else:
                    cur[2] = off + e
                    cur[3].append(sc)
                if prefix in ("E", "S"):
                    out.append(cur)
                    cur = None
            if cur:
                out.append(cur)
        ents = []
        for label, s, e, sc in out:
            # Token offsets can include a leading space.
            while s < e and text[s].isspace():
                s += 1
            if e > s and label in PF_LABELS:
                ents.append({"start": s, "end": e, "label": PF_LABELS[label], "score": round(sum(sc) / len(sc), 4)})
        return ents


class Gliner:
    name = "urchade/gliner_multi_pii-v1"

    def __init__(self, dev: str) -> None:
        from gliner import GLiNER

        self.model = GLiNER.from_pretrained(self.name)
        try:
            self.model.to(dev)
        except Exception:  # some backends only run on CPU
            pass

    def __call__(self, text: str) -> list[dict]:
        ents = []
        labels = list(GLINER_LABELS)
        for off, piece in chunks(text):
            # GLiNER's window is ~384 tokens: run it line by line within the chunk.
            line_off = 0
            for line in re.split(r"(?<=\n)", piece):
                if line.strip():
                    for e in self.model.predict_entities(line, labels, threshold=0.5):
                        ents.append({"start": off + line_off + e["start"], "end": off + line_off + e["end"],
                                     "label": GLINER_LABELS[e["label"]], "score": round(float(e["score"]), 4)})
                line_off += len(line)
        return ents


def merge(*lists: list[dict]) -> list[dict]:
    """Union; for identical spans keep the higher score. Shadow resolves remaining overlaps."""
    best: dict[tuple, dict] = {}
    for l in lists:
        for e in l:
            k = (e["start"], e["end"])
            if k not in best or e["score"] > best[k]["score"]:
                best[k] = e
    return sorted(best.values(), key=lambda e: (e["start"], e["end"]))


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--backend", choices=["privacy-filter", "gliner", "ensemble"], default="ensemble")
    ap.add_argument("--host", default="127.0.0.1")
    ap.add_argument("--port", type=int, default=8878)
    ap.add_argument("--min-score", type=float, default=0.5)
    # Apple's MPS backend crashes on some macOS versions; pass --device cpu there.
    ap.add_argument("--device", choices=["auto", "cpu", "cuda", "mps"], default="auto")
    ap.add_argument("--allow-remote", action="store_true",
                    help="permit binding a non-loopback address (prompts would leave this machine)")
    a = ap.parse_args()
    if not is_loopback(a.host) and not a.allow_remote:
        sys.exit(f"refusing to bind non-loopback host {a.host!r}: prompt text must stay on this "
                 "machine. Use --host 127.0.0.1, or --allow-remote if you really mean it.")
    dev = device(a.device)
    models = []
    if a.backend in ("privacy-filter", "ensemble"):
        models.append(PrivacyFilter(dev))
    if a.backend in ("gliner", "ensemble"):
        models.append(Gliner(dev))
    for m in models:
        m("Warm up: Jane Doe, jane@acme.com")
    lock = threading.Lock()

    class Handler(BaseHTTPRequestHandler):
        server_version = "shadow-ner-hf/1"

        def log_message(self, *_):  # quiet
            pass

        def _send(self, code: int, body: dict) -> None:
            raw = json.dumps(body).encode()
            self.send_response(code)
            self.send_header("content-type", "application/json")
            self.send_header("content-length", str(len(raw)))
            self.end_headers()
            self.wfile.write(raw)

        def do_GET(self):
            if self.path == "/healthz":
                self._send(200, {"ok": True, "backend": a.backend, "models": [m.name for m in models], "device": dev})
            else:
                self._send(404, {"error": "not found"})

        def do_POST(self):
            if self.path != "/detect":
                return self._send(404, {"error": "not found"})
            n = int(self.headers.get("content-length") or 0)
            if n > MAX_BODY:
                return self._send(413, {"error": "too large"})
            try:
                text = json.loads(self.rfile.read(n))["text"]
            except Exception:
                return self._send(400, {"error": "expected {\"text\": ...}"})
            t0 = time.perf_counter()
            with lock:
                ents = merge(*(m(text) for m in models))
            ents = [e for e in ents if e["score"] >= a.min_score]
            self._send(200, {"entities": ents, "ms": round((time.perf_counter() - t0) * 1000, 1)})

    print(f"shadow NER sidecar ({a.backend} on {dev}) listening on http://{a.host}:{a.port}/detect", flush=True)
    ThreadingHTTPServer((a.host, a.port), Handler).serve_forever()


if __name__ == "__main__":
    main()
