"""Torch-free data utilities: tokenizer, BIO conversion, synthetic + JSONL data.

All offsets are Python ``str`` indices, i.e. Unicode codepoints, end exclusive.
"""
from __future__ import annotations

import json
import random
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Iterable, Iterator

from . import LABELS

# Words (letters/digits/underscore, allowing inner - ' . @ joins such as
# "O'Neil", "jean-luc", "a.b@c.io") or any single non-space symbol.
_TOKEN_RE = re.compile(r"\w+(?:[-'’.@]\w+)*|[^\w\s]", re.UNICODE)


@dataclass(frozen=True)
class Token:
    text: str
    start: int
    end: int


@dataclass
class Span:
    start: int
    end: int
    label: str


@dataclass
class Example:
    text: str
    entities: list[Span] = field(default_factory=list)


def tokenize(text: str) -> list[Token]:
    """Split ``text`` into word tokens with codepoint offsets into ``text``."""
    return [Token(m.group(), m.start(), m.end()) for m in _TOKEN_RE.finditer(text)]


def to_bio(ex: Example) -> tuple[list[Token], list[str]]:
    """Project character spans onto tokens as BIO tags (any overlap counts)."""
    toks = tokenize(ex.text)
    tags = ["O"] * len(toks)
    for span in sorted(ex.entities, key=lambda s: s.start):
        first = True
        for i, t in enumerate(toks):
            if t.end > span.start and t.start < span.end and tags[i] == "O":
                tags[i] = ("B-" if first else "I-") + span.label
                first = False
    return toks, tags


def repair_bio(tags: list[str]) -> list[str]:
    """Make a tag sequence valid: an I-X not continuing X becomes B-X."""
    out: list[str] = []
    prev = "O"
    for tag in tags:
        if tag.startswith("I-") and prev[2:] != tag[2:]:
            tag = "B-" + tag[2:]
        out.append(tag)
        prev = tag
    return out


def bio_spans(tags: list[str]) -> list[tuple[int, int, str]]:
    """Token-index spans ``(first, last_exclusive, label)`` from repaired BIO tags."""
    spans: list[tuple[int, int, str]] = []
    start, label = -1, ""
    for i, tag in enumerate(list(repair_bio(tags)) + ["O"]):
        if start >= 0 and not (tag.startswith("I-") and tag[2:] == label):
            spans.append((start, i, label))
            start = -1
        if tag.startswith("B-"):
            start, label = i, tag[2:]
    return spans


# --------------------------------------------------------------------------- #
# Synthetic data
# --------------------------------------------------------------------------- #
FIRST = """James Maria Wei Priya Ahmed Olusegun Sofia Kenji Aisha Mateo Ingrid Dmitri
Fatima Lars Chloe Rahul Yuki Kwame Elena Tomasz Ana Mohammed Zara Liam Mei Arjun Nadia
Diego Hana Oluwaseun Sven Leila Pierre Siobhan Jin Amara Rafael Ewa Kofi Thandiwe
Björn José Zoë Nguyen Ravi Carmen Ibrahim Astrid Chidi Noor Pablo Anika""".split()
LAST = """Smith García Chen Patel Khan Adeyemi Rossi Tanaka Hassan López Johansson Ivanov
Al-Sayed Nielsen Dubois Sharma Sato Mensah Popescu Kowalski Silva Okafor Müller Kim
Nakamura Reddy Haddad Fernández Lindqvist O'Brien Park Mbeki Novak Costa Wiśniewski
Nguyen Schmidt Gupta Moreau Ahmadi Varga Andersson Oyelaran""".split()
ORG_HEAD = """Acme, Globex, Initech, Umbrella, Northwind, Contoso, Fabrikam, Hooli, Vandelay,
Stark, Wayne, Tyrell, Soylent, Cyberdyne, Wonka, Aperture, Pied Piper, Blue Harbor,
Silver Lake, Kestrel, Orbital, Lumen, Brightwave, Helios, Quantia, Redwood, Nimbus,
Sterling""".replace("\n", " ").split(", ")
ORG_TAIL = ["GmbH", "Inc", "Inc.", "Ltd", "Ltd.", "LLC", "AG", "S.A.", "Pty Ltd", "plc",
            "Corp", "Holdings", "Labs", "Systems", "Capital", "Partners", "Bank",
            "Logistics", "Health", "Technologies", "& Co", "BV", "KK", "Pvt Ltd"]
CITY = """London, Berlin, Lagos, Mumbai, Tokyo, São Paulo, Toronto, Nairobi, Sydney, Paris,
Seoul, Dubai, Stockholm, Warsaw, Chicago, Austin, Kraków, Bengaluru, Lisbon, Accra, Zürich,
Osaka, Dublin, Singapore, Kochi, Melbourne, Madrid, Cairo, Jakarta, Oslo, New York,
Rio de Janeiro, Hong Kong""".replace("\n", " ").split(", ")
STREET = ["Baker Street", "Main St", "Elm Avenue", "Hauptstraße", "Rue de Rivoli",
          "MG Road", "Queen Street West", "Calle Mayor", "Harbour Road", "Oak Lane"]
PROJ_A = """Falcon Nightingale Bluebird Atlas Orion Helix Kraken Phoenix Tangerine Granite
Mercury Aurora Basilisk Cobalt Juniper Lighthouse Monsoon Nebula Saffron Tundra""".split()
PROJ_B = ["", "", "", "2", "X", "v2", "Prime", "Next", "Zero", "II"]

TEMPLATES = [
    # emails
    "Hi {PERSON}, please forward the {ORG} contract to {PERSON} before Friday.",
    "Dear {PERSON},\nThanks for visiting our {LOC} office. Regards, {PERSON}",
    "From: {PERSON} <{EMAIL}>\nSubject: {PROJECT} budget\n{ORG} approved the spend.",
    "Can you loop in {PERSON} from {ORG}? They own the {PROJECT} rollout.",
    # chat prompts
    "Summarize this email from {PERSON} about the {ORG} acquisition.",
    "Write a polite reply to {PERSON} declining the meeting in {LOC}.",
    "Rewrite the {PROJECT} launch plan so {ORG} legal can review it.",
    "Translate the note {PERSON} sent from {LOC} into English please.",
    "What risks should {ORG} consider before moving {PROJECT} to the cloud?",
    "my manager {PERSON} wants the {PROJECT} slides by tomorrow, help me draft them",
    # code comments
    "# TODO({PERSON}): remove this hack once {PROJECT} ships",
    "// Ported from the {ORG} SDK, see ticket from {PERSON}",
    "def sync(): # {PROJECT} integration owned by {PERSON}, {ORG}",
    "/* Customer {ORG} in {LOC} hits this path, ask {PERSON} */",
    # meeting notes
    "Meeting notes: {PERSON}, {PERSON} and {PERSON} discussed {PROJECT} timelines.",
    "Action item: {PERSON} to call {ORG} re: the {LOC} warehouse lease.",
    "Attendees: {PERSON} ({ORG}), {PERSON}. Location: {LOC}.",
    "Decision: pause {PROJECT} until {ORG} signs. Owner: {PERSON}.",
    # support tickets
    "Ticket #{NUM}: {PERSON} at {ORG} cannot log in since the update.",
    "Customer {PERSON} reports shipping delays to {LOC}. Order {NUM}.",
    "Escalated by {PERSON}: {ORG} invoice {NUM} is wrong, ship to {LOC}.",
    "User {PERSON} says Project {PROJECT} dashboard crashes on load.",
    # mostly-negative / distractors
    "The quick brown fox jumps over the lazy dog.",
    "Please Review The Attached Document And Reply By Monday.",
    "Error: NullPointerException at line {NUM} in Main.java",
    "Q3 revenue grew {NUM} percent compared to last year.",
    "Explain how a BiLSTM tagger works in simple terms.",
]


def _person(rng: random.Random) -> str:
    r = rng.random()
    if r < 0.15:
        return rng.choice(FIRST)
    if r < 0.25:
        return f"{rng.choice(['Dr.', 'Mr.', 'Ms.', 'Prof.'])} {rng.choice(LAST)}"
    if r < 0.35:
        return f"{rng.choice(FIRST)} {rng.choice(FIRST)[0]}. {rng.choice(LAST)}"
    return f"{rng.choice(FIRST)} {rng.choice(LAST)}"


def _org(rng: random.Random) -> str:
    head = rng.choice(ORG_HEAD)
    if rng.random() < 0.3:
        head += " " + rng.choice(ORG_HEAD)
    return head if rng.random() < 0.15 else f"{head} {rng.choice(ORG_TAIL)}"


def _loc(rng: random.Random) -> str:
    r = rng.random()
    if r < 0.6:
        return rng.choice(CITY)
    return f"{rng.randint(1, 999)} {rng.choice(STREET)}, {rng.choice(CITY)}"


def _project(rng: random.Random) -> str:
    name = f"{rng.choice(PROJ_A)} {rng.choice(PROJ_B)}".strip()
    return name.upper() if rng.random() < 0.2 else name


_FILL = {"PERSON": _person, "ORG": _org, "LOC": _loc, "PROJECT": _project}
_SLOT_RE = re.compile(r"\{(\w+)\}")


def synth_example(rng: random.Random) -> Example:
    """Fill one random template, recording entity spans as we build the text."""
    tpl = rng.choice(TEMPLATES)
    parts: list[str] = []
    spans: list[Span] = []
    pos = 0
    cursor = 0
    for m in _SLOT_RE.finditer(tpl):
        lit = tpl[cursor:m.start()]
        parts.append(lit)
        pos += len(lit)
        slot = m.group(1)
        if slot in _FILL:
            value = _FILL[slot](rng)
            spans.append(Span(pos, pos + len(value), slot))
        elif slot == "EMAIL":
            value = f"{rng.choice(FIRST).lower()}.{rng.choice(LAST).lower()}@example.com"
        else:
            value = str(rng.randint(10, 99999))
        parts.append(value)
        pos += len(value)
        cursor = m.end()
    parts.append(tpl[cursor:])
    text = "".join(parts)
    if rng.random() < 0.1:  # case noise: models must not rely on capitals alone
        text = text.lower()
    return Example(text, spans)


def synthetic(n: int, seed: int = 0) -> list[Example]:
    rng = random.Random(seed)
    return [synth_example(rng) for _ in range(n)]


def load_jsonl(path: str | Path) -> list[Example]:
    """Load ``{"text": str, "entities": [{"start", "end", "label"}]}`` lines."""
    out: list[Example] = []
    with open(path, encoding="utf-8") as fh:
        for lineno, line in enumerate(fh, 1):
            if not line.strip():
                continue
            obj = json.loads(line)
            spans = []
            for e in obj.get("entities", []):
                label = str(e["label"]).upper()
                if label not in LABELS:
                    raise ValueError(f"{path}:{lineno}: unknown label {label!r}")
                spans.append(Span(int(e["start"]), int(e["end"]), label))
            out.append(Example(obj["text"], spans))
    return out


def iter_bio(examples: Iterable[Example]) -> Iterator[tuple[list[Token], list[str]]]:
    for ex in examples:
        toks, tags = to_bio(ex)
        if toks:
            yield toks, tags
