#!/usr/bin/env python3
"""
bench_ab_diff.py

Document-by-document comparison of two benchmark arms: a before and an after
build of one change, each measured on the synthetic formats and on the
real-specimen corpus.

`synth_ab_diff.py` compares one pair of synthetic reports and lists the seeds
whose scored state moved. Attributing an MRZ change needs more, every time: all
formats at once; the accepted-read counts from #578; the seeds whose read
changed but which `synth_ab_diff` does not list (the 2026-09-28 headline A/B
had 214 of them next to 225 movers); and the real-specimen run, where the
question is which zone each document now reads, not only its outcome. This
script reports all of it, so that every flip can be attributed.

Standard library only, matching every other `tools/` script.

## Arm layout

An arm is one directory, holding whatever the A/B measured:

    ARM/<name>.json   one `synthpass-bench --out` report per format, e.g.
                      synthpass-bench --document-type td1 --profile clean
                        --count 100 --seed 0 --out ARM/td1.json
    ARM/real/         one `provider-bench` run, e.g.
                      provider-bench --real-specimens --mrz-only --dump-ocr
                        --dump-ocr-hits --out ARM/real/report.json

Synthetic reports are paired by file name. A pair must cover the same document
type, profile and seeds, which `synth_ab_diff.compare` checks. The real run is
compared when both arms have `real/provider-bench-ocr-outcomes.jsonl`. Its zones
are compared when both also have `real/provider-bench-miss-ocr-dump.jsonl`, which
`--dump-ocr` writes (`--dump-ocr-hits` adds the hits' zones). Measure both arms
with the same flags.

## Usage

    python tools/bench_ab_diff.py BEFORE_ARM AFTER_ARM
    python tools/bench_ab_diff.py BEFORE_ARM AFTER_ARM --asset <asset_id> ...
    python tools/bench_ab_diff.py BEFORE_ARM AFTER_ARM --json

`--asset` (repeatable) adds a status block for a named real specimen whether or
not it changed: an `asset_id` as the outcome ledger spells it, such as
`passports/Kosovo_Passport_Specimen_P0_RKS_2023_mrz.jpg`.

Exit status: 0 if every pair compared is an A/B; 1 if a synthetic pair covers
different documents or the two real runs cover different assets (then it is
not an A/B); 2 on unreadable input, or when the arms have nothing in common to
compare.

## What it prints

- **Synthetic, per report:**
  - hits and strict-name hits;
  - accepted reads: hits plus `document_number_mismatch` misses, as #578 counts
    them;
  - the seeds whose accepted read has a wrong document code or issuer;
  - `synth_ab_diff`'s correct, wrong-accept and valid-miss totals.
- **Every seed whose read changed:** its outcome, its miss kind or any field
  value.
  - Its `synth_ab_diff` class. `refused -> refused` is a miss whose read changed.
  - Its miss kind and `retry_stop` in each arm.
  - Each changed field, and the zone's lines when they changed.
  - A `retry_stop` that differs between the arms deserves a second look. The OCR
    retry loop is wall-clock budgeted, so a `budget` stop can move a seed without
    any code change.
- **Real:**
  - the outcome counts;
  - every document whose outcome or MRZ format changed;
  - every document whose recovered zone changed, with its lines in each arm and,
    where the corpus has a fixture, which lines match it.

Accepted-read counts are recomputed from each seed's fields rather than read
from the report, so reports written before #578 compare too.

## Privacy

Synthetic truth is generated and carries no real person. The real corpus is
public specimens, whose MRZ lines the dated notes in `knowledge/benchmarks/`
already quote. This prints recovered zones, fixture agreement and outcomes. It
never prints a dump's raw OCR text.
"""
from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

import synth_ab_diff as synth

OUTCOMES = "provider-bench-ocr-outcomes.jsonl"
ZONES = "provider-bench-miss-ocr-dump.jsonl"
PREFIX_FIELDS = ("document_type", "issuing_country")
# The five formats in the order the dated notes list them; any other report
# name sorts after them, alphabetically.
FORMAT_ORDER = ("td1", "td2", "td3", "mrva", "mrvb")


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl(path: Path) -> dict[str, dict]:
    """Rows keyed by `asset_id`."""
    rows = {}
    for line in path.read_text(encoding="utf-8").splitlines():
        if line.strip():
            row = json.loads(line)
            rows[row["asset_id"]] = row
    return rows


def synthetic_reports(arm: Path) -> dict[str, Path]:
    """Every `synthpass-bench` report directly in `arm`, keyed by file name."""
    out = {}
    for path in sorted(arm.glob("*.json")):
        try:
            data = load_json(path)
        except (OSError, ValueError):
            continue
        if isinstance(data, dict) and isinstance(data.get("results"), list) and "document_type" in data:
            out[path.name] = path
    return out


def report_order(name: str) -> tuple[int, str]:
    stem = Path(name).stem
    return (FORMAT_ORDER.index(stem), "") if stem in FORMAT_ORDER else (len(FORMAT_ORDER), name)


def outcome(doc: dict) -> str:
    """`hit`, or the miss kind."""
    return "hit" if doc.get("hit") else str(doc.get("miss_kind"))


def is_accepted_read(doc: dict) -> bool:
    """A hit, or a checksum-valid read whose document number is wrong (#578)."""
    return bool(doc.get("hit")) or doc.get("miss_kind") == "document_number_mismatch"


def is_prefix_wrong(doc: dict) -> bool:
    return is_accepted_read(doc) and any(f in PREFIX_FIELDS for f in synth.wrong_fields(doc))


def read_of(doc: dict) -> tuple:
    """What a seed read: its outcome and every field's error and value.
    Timing and retry telemetry are left out on purpose: they move between two
    runs of one build."""
    return (
        outcome(doc),
        tuple((f.get("field"), f.get("cer", 0), f.get("got")) for f in doc.get("fields", [])),
    )


def zone(doc: dict) -> tuple[str | None, str | None]:
    """(what the seed read, the truth), from its `mrz_lines` field. The report
    keeps both strings only when they differ, so an exact read comes back as
    ("=truth", None)."""
    for f in doc.get("fields", []):
        if f.get("field") == "mrz_lines":
            if f.get("cer", 0) > 0:
                return f.get("got"), f.get("expected")
            return "=truth", None
    return None, None


def synthetic_summary(report: dict) -> dict:
    docs = report.get("results", [])
    return {
        "hits": sum(1 for d in docs if d.get("hit")),
        "strict_hits": report.get("strict_hits"),
        "accepted_reads": sum(1 for d in docs if is_accepted_read(d)),
        "prefix_wrong_accepted_read_seeds": [d["seed"] for d in docs if is_prefix_wrong(d)],
    }


def compare_synthetic(before: dict, after: dict) -> dict:
    base = synth.compare(before, after)
    rb = {d["seed"]: d for d in before.get("results", [])}
    ra = {d["seed"]: d for d in after.get("results", [])}
    changed = []
    for seed in sorted(set(rb) & set(ra)):
        b, a = rb[seed], ra[seed]
        if read_of(b) == read_of(a):
            continue
        (zone_b, truth_b), (zone_a, truth_a) = zone(b), zone(a)
        changed.append({
            "seed": seed,
            "class": f"{synth.state(b)} -> {synth.state(a)}",
            "outcome": [outcome(b), outcome(a)],
            "retry_stop": [synth.retry_cell(b, "retry_stop"), synth.retry_cell(a, "retry_stop")],
            "changes": synth.field_changes(b, a),
            "zone": None if zone_b == zone_a else {
                "before": zone_b, "after": zone_a, "truth": truth_b or truth_a,
            },
        })
    return {
        "problems": base["problems"],
        "before": {**synthetic_summary(before), **base["before"]},
        "after": {**synthetic_summary(after), **base["after"]},
        "changed": changed,
    }


def fixture_verdict(lines: list[str] | None, fixture: str | list[str] | None) -> str | None:
    """None without a fixture; else `exact`, or one mark per fixture line
    (`=` matches, `x` does not), e.g. `=x=`."""
    if not fixture:
        return None
    want = fixture.split("\n") if isinstance(fixture, str) else list(fixture)
    got = list(lines or [])
    if got == want:
        return "exact"
    return "".join("=" if i < len(got) and got[i] == w else "x" for i, w in enumerate(want))


def real_state(row: dict | None) -> dict | None:
    if row is None:
        return None
    return {k: row.get(k) for k in ("outcome", "mrz_format", "miss_reason")}


def compare_real(before_dir: Path, after_dir: Path, assets: list[str]) -> dict:
    ob, oa = load_jsonl(before_dir / OUTCOMES), load_jsonl(after_dir / OUTCOMES)
    problems = []
    if set(ob) != set(oa):
        problems.append(f"asset sets differ: {len(set(ob) - set(oa))} only before, "
                        f"{len(set(oa) - set(ob))} only after")
    zones = (before_dir / ZONES).is_file() and (after_dir / ZONES).is_file()
    zb = load_jsonl(before_dir / ZONES) if zones else {}
    za = load_jsonl(after_dir / ZONES) if zones else {}

    def hits_by_format(rows):
        return dict(sorted(Counter(str(r.get("mrz_format")) for r in rows.values()
                                   if r.get("outcome") == "hit").items()))

    outcome_changes = []
    for asset in sorted(set(ob) & set(oa)):
        b, a = ob[asset], oa[asset]
        if (b.get("outcome"), b.get("mrz_format")) != (a.get("outcome"), a.get("mrz_format")):
            outcome_changes.append({"asset": asset, "before": real_state(b), "after": real_state(a)})

    def zone_entry(asset):
        b, a = zb.get(asset), za.get(asset)
        fixture = (b or a or {}).get("ground_truth_mrz")
        lines_b = b.get("recovered_mrz_lines") if b else None
        lines_a = a.get("recovered_mrz_lines") if a else None
        return {
            "asset": asset,
            "outcome": [(ob.get(asset) or {}).get("outcome"), (oa.get(asset) or {}).get("outcome")],
            "before": lines_b,
            "after": lines_a,
            "fixture": [fixture_verdict(lines_b, fixture), fixture_verdict(lines_a, fixture)],
        }

    zone_changes = [zone_entry(asset) for asset in sorted(set(zb) & set(za))
                    if zb[asset].get("recovered_mrz_lines") != za[asset].get("recovered_mrz_lines")]
    watched = []
    for asset in assets:
        entry = zone_entry(asset)
        entry["known"] = asset in ob or asset in oa
        entry["dumped"] = [asset in zb, asset in za]
        entry["states"] = [real_state(ob.get(asset)), real_state(oa.get(asset))]
        watched.append(entry)
    return {
        "problems": problems,
        "counts": [dict(sorted(Counter(str(r.get("outcome")) for r in rows.values()).items()))
                   for rows in (ob, oa)],
        "hits_by_format": [hits_by_format(ob), hits_by_format(oa)],
        "zones_compared": zones,
        "dumped": [len(zb), len(za)],
        "dumped_in_one_arm_only": sorted(set(zb) ^ set(za)),
        "outcome_changes": outcome_changes,
        "zone_changes": zone_changes,
        "watched": watched,
    }


def compare_arms(before: Path, after: Path, assets: list[str]) -> dict:
    sb, sa = synthetic_reports(before), synthetic_reports(after)
    synthetic = {}
    for name in sorted(set(sb) & set(sa), key=report_order):
        synthetic[name] = compare_synthetic(load_json(sb[name]), load_json(sa[name]))
    real_b, real_a = before / "real", after / "real"
    has_real = [(real_b / OUTCOMES).is_file(), (real_a / OUTCOMES).is_file()]
    real = compare_real(real_b, real_a, assets) if all(has_real) else None
    return {
        "synthetic": synthetic,
        "synthetic_in_one_arm_only": {"before": sorted(set(sb) - set(sa)), "after": sorted(set(sa) - set(sb))},
        "real": real,
        "real_in_one_arm_only": None if all(has_real) or not any(has_real)
        else ("before" if has_real[0] else "after"),
    }


def problems_of(result: dict) -> list[str]:
    out = [f"{name}: {p}" for name, r in result["synthetic"].items() for p in r["problems"]]
    if result["real"]:
        out += [f"real: {p}" for p in result["real"]["problems"]]
    return out


def arrow(pair) -> str:
    return f"{pair[0]} -> {pair[1]}"


def join_zone(z) -> str:
    if z is None:
        return "-"
    return " / ".join(z) if isinstance(z, list) else z.replace("\n", " / ")


def render(result: dict) -> str:
    lines = [f"NOT AN A/B: {p}" for p in problems_of(result)]
    for name, r in result["synthetic"].items():
        b, a = r["before"], r["after"]
        lines.append(
            f"{name}: hits {b['hits']} -> {a['hits']}   strict {b['strict_hits']} -> {a['strict_hits']}   "
            f"accepted {b['accepted_reads']} -> {a['accepted_reads']}   "
            f"accepted with a wrong code or issuer {b['prefix_wrong_accepted_read_seeds']} -> "
            f"{a['prefix_wrong_accepted_read_seeds']}   correct {b['correct']} -> {a['correct']}   "
            f"wrong accepts {b['wrong_accepts']} -> {a['wrong_accepts']}   "
            f"valid misses {b['valid_misses']} -> {a['valid_misses']}   "
            f"reads changed {len(r['changed'])} (of {a['count']})")
        for c in r["changed"]:
            lines.append(f"  seed {c['seed']}: {c['class']}   {arrow(c['outcome'])}   "
                         f"retry_stop {arrow(c['retry_stop'])}")
            for f in c["changes"]:
                lines.append(f"      {f['field']:<16} expected {f['expected']!r:<20} "
                             f"before {f['before']!r:<20} after {f['after']!r}")
            if c["zone"]:
                lines.append(f"      zone before {join_zone(c['zone']['before'])}")
                lines.append(f"      zone after  {join_zone(c['zone']['after'])}")
                if c["zone"]["truth"]:
                    lines.append(f"      zone truth  {join_zone(c['zone']['truth'])}")
    for side, names in result["synthetic_in_one_arm_only"].items():
        if names:
            lines.append(f"only in the {side} arm, not compared: {', '.join(names)}")
    real = result["real"]
    if result["real_in_one_arm_only"]:
        lines.append(f"real run only in the {result['real_in_one_arm_only']} arm, not compared")
    if real:
        lines.append("real specimens:")
        for key in sorted(set(real["counts"][0]) | set(real["counts"][1])):
            lines.append(f"  {key}: {real['counts'][0].get(key, 0)} -> {real['counts'][1].get(key, 0)}")
        lines.append(f"  hits by format: {arrow(real['hits_by_format'])}")
        lines.append(f"  outcome changes: {len(real['outcome_changes'])}")
        for c in real["outcome_changes"]:
            b, a = c["before"], c["after"]
            lines.append(f"    {c['asset']}: {b['outcome']}/{b['mrz_format']} -> {a['outcome']}/{a['mrz_format']}")
            if a["miss_reason"] != b["miss_reason"]:
                lines.append(f"      reason after: {a['miss_reason']}")
        if real["zones_compared"]:
            lines.append(f"  zones dumped {arrow(real['dumped'])}; zone changes: {len(real['zone_changes'])}")
            for z in real["zone_changes"]:
                fixture = "" if z["fixture"] == [None, None] else f"   fixture {arrow(z['fixture'])}"
                lines.append(f"    {z['asset']}: {arrow(z['outcome'])}{fixture}")
                lines.append(f"      before {join_zone(z['before'])}")
                lines.append(f"      after  {join_zone(z['after'])}")
            if real["dumped_in_one_arm_only"]:
                lines.append(f"  dumped in one arm only, not compared: {len(real['dumped_in_one_arm_only'])}")
        else:
            lines.append(f"  zones not compared: an arm has no {ZONES} (measure with --dump-ocr)")
        for w in real["watched"]:
            if not w["known"]:
                lines.append(f"  watched {w['asset']}: in neither run")
                continue
            sb, sa = w["states"]
            state = f"{(sb or {}).get('outcome')}/{(sb or {}).get('mrz_format')} -> " \
                    f"{(sa or {}).get('outcome')}/{(sa or {}).get('mrz_format')}"
            fixture = "" if w["fixture"] == [None, None] else f"   fixture {arrow(w['fixture'])}"
            lines.append(f"  watched {w['asset']}: {state}   "
                         f"zone {'changed' if w['before'] != w['after'] else 'unchanged'}{fixture}")
            lines.append(f"      before {join_zone(w['before']) if w['dumped'][0] else 'not dumped'}")
            lines.append(f"      after  {join_zone(w['after']) if w['dumped'][1] else 'not dumped'}")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[1])
    ap.add_argument("before", type=Path, help="the before arm's directory")
    ap.add_argument("after", type=Path, help="the after arm's directory")
    ap.add_argument("--asset", action="append", default=[],
                    help="a real specimen's asset_id to report whether or not it changed (repeatable)")
    ap.add_argument("--json", action="store_true", help="print the result as JSON")
    args = ap.parse_args(argv)
    for arm in (args.before, args.after):
        if not arm.is_dir():
            print(f"bench_ab_diff: not a directory: {arm}", file=sys.stderr)
            return 2
    try:
        result = compare_arms(args.before, args.after, args.asset)
    except (OSError, ValueError, KeyError) as e:
        print(f"bench_ab_diff: cannot read the arms: {e}", file=sys.stderr)
        return 2
    if not result["synthetic"] and not result["real"]:
        print("bench_ab_diff: the two arms have no synthetic report or real run in common", file=sys.stderr)
        return 2
    print(json.dumps(result, indent=2) if args.json else render(result))
    return 1 if problems_of(result) else 0


if __name__ == "__main__":
    sys.exit(main())
