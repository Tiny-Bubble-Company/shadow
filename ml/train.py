#!/usr/bin/env python3
"""Train the Shadow NER tagger.

    python train.py --out model.pt --epochs 8 --synthetic 20000 [--data my.jsonl] [--device cpu]
"""
from __future__ import annotations

import argparse
import random
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

try:
    import torch
    from torch import nn
except ImportError:  # pragma: no cover
    sys.exit("PyTorch is required: python -m venv .venv && pip install -r requirements.txt")

from shadow_ner import TAGS
from shadow_ner.data import Example, Token, bio_spans, iter_bio, load_jsonl, synthetic
from shadow_ner.model import Config, Tagger, encode, save

Batch = list[tuple[list[Token], list[str]]]
TAG_ID = {t: i for i, t in enumerate(TAGS)}


def evaluate(model: Tagger, data: Batch, bs: int = 256) -> tuple[float, float, float]:
    """Entity-level micro precision / recall / F1 (exact span + label match)."""
    tp = n_pred = n_gold = 0
    for i in range(0, len(data), bs):
        chunk = data[i:i + bs]
        preds = model.predict_tokens([toks for toks, _ in chunk])
        for (_, gold), (pred, _) in zip(chunk, preds):
            g, p = set(bio_spans(gold)), set(bio_spans(pred))
            tp += len(g & p)
            n_pred += len(p)
            n_gold += len(g)
    prec = tp / n_pred if n_pred else 0.0
    rec = tp / n_gold if n_gold else 0.0
    f1 = 2 * prec * rec / (prec + rec) if prec + rec else 0.0
    return prec, rec, f1


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", default="model.pt", help="output checkpoint path")
    ap.add_argument("--epochs", type=int, default=8)
    ap.add_argument("--synthetic", type=int, default=20000, help="number of synthetic examples")
    ap.add_argument("--data", action="append", default=[], help="JSONL file(s) of your own labelled data")
    ap.add_argument("--device", default="cpu")
    ap.add_argument("--batch-size", type=int, default=64)
    ap.add_argument("--lr", type=float, default=2e-3)
    ap.add_argument("--holdout", type=float, default=0.1, help="fraction held out for evaluation")
    ap.add_argument("--seed", type=int, default=13)
    args = ap.parse_args()

    random.seed(args.seed)
    torch.manual_seed(args.seed)

    examples: list[Example] = synthetic(args.synthetic, seed=args.seed)
    for path in args.data:
        user = load_jsonl(path)
        print(f"loaded {len(user)} examples from {path}")
        examples += user * 3  # upweight real data relative to synthetic
    data: Batch = list(iter_bio(examples))
    random.shuffle(data)
    n_dev = max(1, int(len(data) * args.holdout))
    dev, train = data[:n_dev], data[n_dev:]
    print(f"train={len(train)} heldout={len(dev)} device={args.device}")

    cfg = Config()
    model = Tagger(cfg).to(args.device)
    opt = torch.optim.Adam(model.parameters(), lr=args.lr)
    loss_fn = nn.CrossEntropyLoss(ignore_index=-100)

    best_f1 = -1.0
    for epoch in range(1, args.epochs + 1):
        model.train()
        random.shuffle(train)
        total, t0 = 0.0, time.time()
        for i in range(0, len(train), args.batch_size):
            chunk = train[i:i + args.batch_size]
            enc = {k: v.to(args.device) for k, v in encode([t for t, _ in chunk], cfg).items()}
            gold = torch.full(enc["words"].shape, -100, dtype=torch.long)
            for j, (_, tags) in enumerate(chunk):
                gold[j, :len(tags)] = torch.tensor([TAG_ID[t] for t in tags])
            logits = model(**enc)
            loss = loss_fn(logits.reshape(-1, len(TAGS)), gold.to(args.device).reshape(-1))
            opt.zero_grad()
            loss.backward()
            nn.utils.clip_grad_norm_(model.parameters(), 5.0)
            opt.step()
            total += loss.item() * len(chunk)
        p, r, f1 = evaluate(model, dev)
        print(f"epoch {epoch:2d}  loss {total / len(train):.4f}  "
              f"P {p:.3f}  R {r:.3f}  F1 {f1:.3f}  ({time.time() - t0:.1f}s)")
        if f1 > best_f1:
            best_f1 = f1
            save(model, args.out, meta={"epoch": epoch, "f1": round(f1, 4), "seed": args.seed,
                                        "synthetic": args.synthetic, "data": list(args.data)})
    print(f"saved best model (F1 {best_f1:.3f}) to {args.out}")


if __name__ == "__main__":
    main()
