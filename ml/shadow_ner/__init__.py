"""shadow_ner: a small, pure-PyTorch named-entity tagger for Shadow.

`shadow_ner.data` is torch-free (tokenizer, BIO helpers, synthetic data).
`shadow_ner.model` needs torch (tagger, training helpers, predict).
"""

LABELS = ("PERSON", "ORG", "LOC", "PROJECT")
TAGS = ("O",) + tuple(f"{p}-{lab}" for lab in LABELS for p in ("B", "I"))

__all__ = ["LABELS", "TAGS"]
