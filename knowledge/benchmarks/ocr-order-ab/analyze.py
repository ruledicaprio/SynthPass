#!/usr/bin/env python3
"""Compare SYNTHPASS_OCR_ORDER arms from provider-bench --real-specimens --mrz-only reports.

Usage: python analyze.py default.json band-first.json [control.json ...]

Compares POSITIONALLY: the corpus walk is deterministic and sorted, so row i is
the same document in every report. (Keying by `name` loses the ~3 duplicate
stems the corpus carries across sub-directories.)
"""
import json
import sys
from collections import Counter

OFF_DENOMINATOR = {"redacted_mrz", "no_mrz_expected", "checksum_failed_specimen"}


def rows(path):
    return json.load(open(path))["providers"][0]["documents_detail"]


def summarize(rs, tag):
    c = Counter(r.get("miss_reason") for r in rs)
    hits = c.get(None, 0)
    off = sum(v for k, v in c.items() if k in OFF_DENOMINATOR)
    scored = len(rs) - off
    print(f"{tag:22} {len(rs)} docs | scored {scored} | hits {hits} "
          f"= {100 * hits / scored:.2f}%  "
          f"cf={c.get('checksum_failed', 0)} nmf={c.get('no_mrz_found', 0)}")


def diff(a, b, la, lb):
    assert len(a) == len(b), f"{la}/{lb} row count differs"
    assert [x["name"] for x in a] == [x["name"] for x in b], "row order differs"
    changes = [(x["name"], x.get("miss_reason"), y.get("miss_reason"))
               for x, y in zip(a, b)
               if x.get("miss_reason") != y.get("miss_reason")]
    rec = [c for c in changes if c[2] is None]
    reg = [c for c in changes if c[1] is None]
    shift = [c for c in changes if c[1] is not None and c[2] is not None]
    print(f"\n{la} -> {lb}: {len(changes)} change(s) "
          f"({len(rec)} recovered, {len(reg)} regressed, {len(shift)} shifted)")
    for n, mx, my in changes:
        kind = "RECOVER" if my is None else ("REGRESS" if mx is None else "shift  ")
        print(f"    {kind}  {mx} -> {my}   {n}")


paths = sys.argv[1:]
reps = [rows(p) for p in paths]
labels = [p.split("/")[-1].split("\\")[-1].replace(".json", "") for p in paths]
for rs, lab in zip(reps, labels):
    summarize(rs, lab)
for i in range(1, len(reps)):
    diff(reps[0], reps[i], labels[0], labels[i])
