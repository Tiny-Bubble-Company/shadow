"""Small pure-PyTorch BiLSTM tagger (no HuggingFace, no vocab file).

Per word: hashed word embedding + char-CNN + shape features -> BiLSTM -> tags.
"""
from __future__ import annotations

import zlib
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any, Sequence

import torch
from torch import Tensor, nn

from . import TAGS
from .data import Token, bio_spans, repair_bio, tokenize

__all__ = ["Config", "Tagger", "tokenize", "encode", "save", "load"]

MAX_CHARS = 24
N_SHAPE = 8


@dataclass
class Config:
    word_buckets: int = 2**18
    char_buckets: int = 1024
    word_dim: int = 64
    char_dim: int = 24
    char_filters: int = 48
    hidden: int = 128
    dropout: float = 0.3


def word_id(word: str, buckets: int) -> int:
    """Stable bucket hash (crc32, not Python's salted hash). 0 is padding."""
    return 1 + zlib.crc32(word.lower().encode("utf-8")) % (buckets - 1)


def char_ids(word: str, buckets: int) -> list[int]:
    return [1 + ord(c) % (buckets - 1) for c in word[:MAX_CHARS]]


def shape(word: str) -> list[float]:
    return [
        float(word[:1].isupper()),
        float(word.isupper() and any(c.isalpha() for c in word)),
        float(word.islower()),
        float(any(c.isdigit() for c in word)),
        float("@" in word),
        float(not any(c.isalnum() for c in word)),
        float(any(c in "-'’." for c in word)),
        min(len(word), 20) / 20.0,
    ]


def encode(batch: Sequence[Sequence[Token]], cfg: Config) -> dict[str, Tensor]:
    """Turn token lists into padded tensors: words [B,T], chars [B,T,C], shape [B,T,S]."""
    b, t = len(batch), max(1, max((len(x) for x in batch), default=1))
    words = torch.zeros(b, t, dtype=torch.long)
    chars = torch.zeros(b, t, MAX_CHARS, dtype=torch.long)
    shapes = torch.zeros(b, t, N_SHAPE)
    lengths = torch.tensor([max(1, len(x)) for x in batch], dtype=torch.long)
    for i, toks in enumerate(batch):
        for j, tok in enumerate(toks):
            words[i, j] = word_id(tok.text, cfg.word_buckets)
            cid = char_ids(tok.text, cfg.char_buckets)
            chars[i, j, : len(cid)] = torch.tensor(cid, dtype=torch.long)
            shapes[i, j] = torch.tensor(shape(tok.text))
    return {"words": words, "chars": chars, "shapes": shapes, "lengths": lengths}


class Tagger(nn.Module):
    def __init__(self, cfg: Config) -> None:
        super().__init__()
        self.cfg = cfg
        self.word_emb = nn.Embedding(cfg.word_buckets, cfg.word_dim, padding_idx=0)
        self.char_emb = nn.Embedding(cfg.char_buckets, cfg.char_dim, padding_idx=0)
        self.char_cnn = nn.Conv1d(cfg.char_dim, cfg.char_filters, kernel_size=3, padding=1)
        self.drop = nn.Dropout(cfg.dropout)
        in_dim = cfg.word_dim + cfg.char_filters + N_SHAPE
        self.lstm = nn.LSTM(in_dim, cfg.hidden, batch_first=True, bidirectional=True)
        self.out = nn.Linear(2 * cfg.hidden, len(TAGS))

    def forward(self, words: Tensor, chars: Tensor, shapes: Tensor, lengths: Tensor) -> Tensor:
        b, t, c = chars.shape
        ch = self.char_emb(chars.view(b * t, c)).transpose(1, 2)  # [BT, D, C]
        ch = torch.relu(self.char_cnn(ch)).max(dim=2).values.view(b, t, -1)
        x = self.drop(torch.cat([self.word_emb(words), ch, shapes], dim=-1))
        packed = nn.utils.rnn.pack_padded_sequence(
            x, lengths.cpu(), batch_first=True, enforce_sorted=False)
        h, _ = self.lstm(packed)
        h, _ = nn.utils.rnn.pad_packed_sequence(h, batch_first=True, total_length=t)
        return self.out(self.drop(h))  # [B, T, n_tags]

    @torch.no_grad()
    def predict_tokens(self, batch: Sequence[Sequence[Token]]) -> list[tuple[list[str], list[float]]]:
        """Greedy decode + BIO repair; returns (tags, probs) per sequence."""
        self.eval()
        dev = next(self.parameters()).device
        enc = {k: v.to(dev) for k, v in encode(batch, self.cfg).items()}
        probs = torch.softmax(self(**enc), dim=-1)
        best_p, best_i = probs.max(dim=-1)
        out = []
        for i, toks in enumerate(batch):
            n = len(toks)
            tags = repair_bio([TAGS[k] for k in best_i[i, :n].tolist()])
            out.append((tags, best_p[i, :n].tolist()))
        return out

    def predict(self, text: str, min_score: float = 0.0) -> list[dict[str, Any]]:
        """Entities as ``{start, end, label, score}`` with codepoint offsets into ``text``."""
        toks = tokenize(text)
        if not toks:
            return []
        tags, probs = self.predict_tokens([toks])[0]
        ents = []
        for a, b, label in bio_spans(tags):
            score = sum(probs[a:b]) / (b - a)
            if score >= min_score:
                ents.append({"start": toks[a].start, "end": toks[b - 1].end,
                             "label": label, "score": round(score, 4)})
        return ents


def save(model: Tagger, path: str | Path, meta: dict[str, Any] | None = None) -> None:
    """Write config, tag set, weights and metadata to one .pt file."""
    torch.save({"config": asdict(model.cfg), "tags": list(TAGS),
                "state_dict": model.state_dict(), "meta": meta or {}}, str(path))


def load(path: str | Path, device: str = "cpu") -> Tagger:
    ckpt = torch.load(str(path), map_location=device, weights_only=True)  # no pickle code execution
    if list(ckpt.get("tags", [])) != list(TAGS):
        raise ValueError(f"{path}: tag set mismatch; retrain with this version of train.py")
    model = Tagger(Config(**ckpt["config"]))
    model.load_state_dict(ckpt["state_dict"])
    model.meta = ckpt.get("meta", {})  # type: ignore[attr-defined]
    return model.to(device).eval()
