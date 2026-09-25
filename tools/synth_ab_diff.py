#!/usr/bin/env python3
"""
synth_ab_diff.py

Per-seed comparison of two `synthpass-bench` JSON reports (an A/B of two builds,
or a before/after of one change) on the same synthetic corpus.

Every change to the MRZ path has been judged the same way: #440's M4 drop,
the 2026-09-25 five-format headline, #461's dropped-filler fix. Match the
documents by seed. For each seed whose outcome moved, show the hit and
wrong-accept bits and every field that differs, with its expected and got
values. Then classify each move. This script does that one job, so the
next A/B doesn't rebuild it by hand.

Standard library only, matching every other `tools/` script. Synthetic data
only: the generator's truth is exact and carries no real person, so the
report's `expected`/`got` strings are safe to print. Never point this at a
real-specimen report.

## Usage

    python tools/synth_ab_diff.py BEFORE.json AFTER.json
    python tools/synth_ab_diff.py BEFORE.json AFTER.json --json

Exit status: 0 if the two reports cover the same seeds and profile, 1 if
they don't (a comparison across different corpora is not an A/B), 2 on
unreadable input.

## Classification of a moved seed

`wrong` means a hit with at least one of the 12 scored fields off
(`wrong_accept`, #453/#457). The classes read "before -> after":

- `wrong -> refused`, `correct -> refused`, `refused -> correct`,
  `refused -> wrong`, `wrong -> correct`, `correct -> wrong`;
- `wrong -> wrong` when both are wrong accepts but the wrong fields changed.

A report written before #457 has no `wrong_accept`. For those reports, a hit
is classed from its `fields` CER, the same rule `wrong_scored_fields` uses.
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

DIAGNOSTIC_FIELDS = {"mrz_lines"}


def load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def wrong_fields(doc: dict) -> tuple[str, ...]:
    """The scored fields that differ from truth, in report order."""
    return tuple(
        f["field"]
        for f in doc.get("fields", [])
        if f.get("field") not in DIAGNOSTIC_FIELDS and f.get("cer", 0) > 0
    )


def state(doc: dict) -> str:
    """One of `correct`, `wrong`, `refused`."""
    if not doc.get("hit"):
        return "refused"
    if "wrong_accept" in doc:
        return "wrong" if doc["wrong_accept"] else "correct"
    return "wrong" if wrong_fields(doc) else "correct"


def field_changes(before: dict, after: dict) -> list[dict]:
    """Fields whose (expected, got) differs between the two reads."""
    def by_name(doc):
        return {f["field"]: f for f in doc.get("fields", [])}

    b, a = by_name(before), by_name(after)
    out = []
    for name in list(b) + [n for n in a if n not in b]:
        if name in DIAGNOSTIC_FIELDS:
            continue
        fb, fa = b.get(name, {}), a.get(name, {})
        got_b = fb.get("got") if fb.get("cer", 0) > 0 else fb.get("expected", "=truth")
        got_a = fa.get("got") if fa.get("cer", 0) > 0 else fa.get("expected", "=truth")
        if (fb.get("cer", 0) > 0) != (fa.get("cer", 0) > 0) or got_b != got_a:
            out.append({
                "field": name,
                "expected": fb.get("expected", fa.get("expected")),
                "before": got_b if fb.get("cer", 0) > 0 else "=truth",
                "after": got_a if fa.get("cer", 0) > 0 else "=truth",
            })
    return out


def compare(before: dict, after: dict) -> dict:
    problems = []
    for key in ("profile", "document_type", "seed_start", "count"):
        if before.get(key) != after.get(key):
            problems.append(f"{key}: {before.get(key)!r} != {after.get(key)!r}")
    rb = {d["seed"]: d for d in before.get("results", [])}
    ra = {d["seed"]: d for d in after.get("results", [])}
    if set(rb) != set(ra):
        problems.append(f"seed sets differ: {len(set(rb) ^ set(ra))} seeds in only one report")

    moved, classes = [], Counter()
    for seed in sorted(set(rb) & set(ra)):
        sb, sa = state(rb[seed]), state(ra[seed])
        changes = field_changes(rb[seed], ra[seed])
        if sb == sa and not (sb == "wrong" and wrong_fields(rb[seed]) != wrong_fields(ra[seed])):
            continue
        cls = f"{sb} -> {sa}"
        classes[cls] += 1
        moved.append({"seed": seed, "class": cls, "changes": changes})

    def totals(r):
        docs = r.get("results", [])
        return {
            "hits": sum(1 for d in docs if d.get("hit")),
            "correct": sum(1 for d in docs if state(d) == "correct"),
            "wrong_accepts": sum(1 for d in docs if state(d) == "wrong"),
            "count": len(docs),
        }

    return {
        "problems": problems,
        "before": totals(before),
        "after": totals(after),
        "classes": dict(sorted(classes.items())),
        "moved": moved,
    }


def render(result: dict) -> str:
    lines = []
    for p in result["problems"]:
        lines.append(f"NOT AN A/B: {p}")
    b, a = result["before"], result["after"]
    lines.append(f"hits {b['hits']} -> {a['hits']}   correct {b['correct']} -> {a['correct']}   "
                 f"wrong accepts {b['wrong_accepts']} -> {a['wrong_accepts']}   (of {a['count']})")
    for cls, n in result["classes"].items():
        lines.append(f"  {n:3d}  {cls}")
    for m in result["moved"]:
        lines.append(f"seed {m['seed']}: {m['class']}")
        for c in m["changes"]:
            lines.append(f"    {c['field']:<16} expected {c['expected']!r:<20} "
                         f"before {c['before']!r:<20} after {c['after']!r}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[1])
    ap.add_argument("before", type=Path)
    ap.add_argument("after", type=Path)
    ap.add_argument("--json", action="store_true", help="print the result as JSON")
    args = ap.parse_args(argv)
    try:
        result = compare(load(args.before), load(args.after))
    except (OSError, ValueError, KeyError) as e:
        print(f"synth_ab_diff: cannot read reports: {e}", file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2) if args.json else render(result))
    return 1 if result["problems"] else 0


if __name__ == "__main__":
    sys.exit(main())
