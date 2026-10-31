#!/usr/bin/env python3
"""Render benchmark summaries as Markdown tables for the docs.

    python report.py results-main results-heldout > report.md

Each argument is a results folder written by run.py (it holds summary.json).
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

NAMES = {
    "shadow": "Shadow (built-in rules)",
    "shadow+presidio": "Shadow + Presidio detector",
    "shadow+ner": "Shadow + open-weight NER sidecar",
    "presidio": "Microsoft Presidio",
    "privacy_filter": "OpenAI Privacy Filter",
    "gliner": "GLiNER multi PII",
    "llm_guard": "LLM Guard (Anonymize)",
    "scrubadub": "scrubadub",
}
ORDER = list(NAMES)


def fmt(v, suffix="%"):
    return "n/a" if v is None else f"{v:g}{suffix}"


def overall(summary: dict) -> str:
    rows = [
        "| Tool | Leak rate ↓ | Partial name leak ↓ | Public names changed ↓ | Text kept ↑ | Precision | Recall | F1 | Round trip | p50 latency |",
        "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
    ]
    for tool in sorted(summary, key=lambda t: summary[t]["leak_rate"]):
        s = summary[tool]
        d = s.get("detection") or {}
        name = f"**{NAMES.get(tool, tool)}**" if tool.startswith("shadow") else NAMES.get(tool, tool)
        rows.append(
            f"| {name} | {fmt(s['leak_rate'])} | {fmt(s['partial_name_leak'])} | {fmt(s['public_over_masking'])} | "
            f"{fmt(s['text_kept'])} | {fmt(d.get('precision'))} | {fmt(d.get('recall'))} | {fmt(d.get('f1'), '')} | "
            f"{fmt(s['round_trip']) if s['round_trip'] is not None else 'not reversible'} | {fmt(s['latency_ms']['p50'], ' ms')} |"
        )
    return "\n".join(rows)


def by_key(summary: dict, key: str) -> str:
    tools = [t for t in ORDER if t in summary]
    labels = sorted({l for t in tools for l in summary[t][key]})
    rows = ["| " + key.split("_")[-1].title() + " | " + " | ".join(NAMES[t].split(" (")[0] for t in tools) + " |",
            "| --- |" + " ---: |" * len(tools)]
    for l in labels:
        cells = [fmt(summary[t][key].get(l)) for t in tools]
        rows.append(f"| {l.replace('_', ' ').lower()} | " + " | ".join(cells) + " |")
    return "\n".join(rows)


def main() -> None:
    for folder in sys.argv[1:]:
        summary = json.loads((Path(folder) / "summary.json").read_text())
        any_tool = next(iter(summary.values()))
        print(f"## {folder} ({any_tool['samples']} samples, {any_tool['values']} sensitive values)\n")
        print(overall(summary), "\n")
        print(by_key(summary, "leak_by_label"), "\n")
        print("Detection recall by type\n")
        print(by_key({t: v for t, v in summary.items() if v.get("recall_by_label")}, "recall_by_label"), "\n")
        print(by_key(summary, "leak_by_scenario"), "\n")


if __name__ == "__main__":
    main()
