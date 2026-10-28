"""Contract tests. Run: python -m unittest discover -s tests   (or pytest)

Tokenizer/data/HTTP tests need no torch; model tests skip when torch is absent.
"""
from __future__ import annotations

import importlib.util
import json
import random
import sys
import tempfile
import threading
import unittest
import urllib.request
from http.server import ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

from shadow_ner import LABELS, TAGS  # noqa: E402
from shadow_ner.data import (Example, Span, bio_spans, load_jsonl, repair_bio,  # noqa: E402
                             synth_example, to_bio, tokenize)
import serve  # noqa: E402

HAS_TORCH = importlib.util.find_spec("torch") is not None


class TokenizerTest(unittest.TestCase):
    def test_offsets_are_codepoints(self) -> None:
        text = "Zoë Müller 👋 emailed jean-luc@acme.io about Kraków."
        toks = tokenize(text)
        for t in toks:
            self.assertEqual(text[t.start:t.end], t.text)
        words = [t.text for t in toks]
        self.assertIn("Zoë", words)
        self.assertIn("jean-luc@acme.io", words)
        self.assertIn("👋", words)
        self.assertEqual(tokenize("   "), [])


class DataTest(unittest.TestCase):
    def test_synthetic_bio_consistent(self) -> None:
        rng = random.Random(0)
        for _ in range(500):
            ex = synth_example(rng)
            toks, tags = to_bio(ex)
            self.assertEqual(len(toks), len(tags))
            self.assertTrue(all(t in TAGS for t in tags))
            self.assertEqual(repair_bio(tags), tags, ex.text)  # already valid BIO
            spans = bio_spans(tags)
            self.assertEqual(len(spans), len(ex.entities), ex.text)
            for (a, b, lab), ent in zip(spans, sorted(ex.entities, key=lambda s: s.start)):
                self.assertEqual(lab, ent.label)
                self.assertEqual((toks[a].start, toks[b - 1].end), (ent.start, ent.end), ex.text)

    def test_repair(self) -> None:
        self.assertEqual(repair_bio(["O", "I-ORG", "I-ORG", "I-LOC"]),
                         ["O", "B-ORG", "I-ORG", "B-LOC"])
        self.assertEqual(bio_spans(["B-PERSON", "I-PERSON", "O", "I-LOC"]),
                         [(0, 2, "PERSON"), (3, 4, "LOC")])

    def test_jsonl_loader(self) -> None:
        line = {"text": "Ping Ana at Hooli", "entities": [
            {"start": 5, "end": 8, "label": "person"}, {"start": 12, "end": 17, "label": "ORG"}]}
        with tempfile.NamedTemporaryFile("w", suffix=".jsonl", delete=False, encoding="utf-8") as fh:
            fh.write(json.dumps(line) + "\n\n")
        self.addCleanup(Path(fh.name).unlink)
        [ex] = load_jsonl(fh.name)
        self.assertEqual([(s.label, ex.text[s.start:s.end]) for s in ex.entities],
                         [("PERSON", "Ana"), ("ORG", "Hooli")])
        _, tags = to_bio(ex)
        self.assertEqual(tags, ["O", "B-PERSON", "O", "B-ORG"])


class FakeModel:
    meta = {"fake": True}

    def predict(self, text: str, min_score: float = 0.0) -> list[dict]:
        i = text.find("Ana")
        return [] if i < 0 else [{"start": i, "end": i + 3, "label": "PERSON", "score": 0.9}]


class HttpContractTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.srv = ThreadingHTTPServer(("127.0.0.1", 0), serve.make_handler(FakeModel(), "fake", 0.0))
        cls.base = f"http://127.0.0.1:{cls.srv.server_address[1]}"
        threading.Thread(target=cls.srv.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls) -> None:
        cls.srv.shutdown()
        cls.srv.server_close()

    def _post(self, body: bytes) -> tuple[int, dict]:
        req = urllib.request.Request(self.base + "/detect", data=body,
                                     headers={"Content-Type": "application/json"})
        try:
            with urllib.request.urlopen(req) as r:
                return r.status, json.loads(r.read())
        except urllib.error.HTTPError as e:
            return e.code, json.loads(e.read())

    def test_healthz(self) -> None:
        with urllib.request.urlopen(self.base + "/healthz") as r:
            self.assertEqual(json.loads(r.read())["ok"], True)

    def test_detect(self) -> None:
        text = "👋 hi Ana"
        code, body = self._post(json.dumps({"text": text}).encode())
        self.assertEqual(code, 200)
        [e] = body["entities"]
        self.assertEqual(text[e["start"]:e["end"]], "Ana")
        self.assertEqual(self._post(b'{"nope": 1}')[0], 400)

    def test_loopback_guard(self) -> None:
        self.assertTrue(serve.is_loopback("127.0.0.1"))
        self.assertFalse(serve.is_loopback("0.0.0.0"))


@unittest.skipUnless(HAS_TORCH, "torch not installed")
class ModelTest(unittest.TestCase):
    def test_predict_roundtrip(self) -> None:
        from shadow_ner.model import Config, Tagger, load, save
        model = Tagger(Config(word_buckets=1024, hidden=16))
        text = "Email Zoë García at Globex GmbH in Berlin."
        for ent in model.predict(text):
            self.assertIn(ent["label"], LABELS)
            self.assertTrue(0 <= ent["start"] < ent["end"] <= len(text))
        with tempfile.TemporaryDirectory() as d:
            save(model, Path(d) / "m.pt")
            again = load(Path(d) / "m.pt")
        self.assertEqual(again.predict(text), model.eval().predict(text))


if __name__ == "__main__":
    unittest.main()
