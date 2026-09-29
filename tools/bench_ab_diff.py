#!/usr/bin/env python3
"""
bench_ab_diff.py

Document-by-document comparison of two benchmark arms: a before and an after
build of one change, each measured on the synthetic formats and on the
real-specimen corpus.

`synth_ab_diff.py` compares one pair of synthetic reports and lists the seeds
whose scored state moved. Attributing an MRZ change needs more, every time:
- all formats at once;
- the accepted-read counts from #578;
- every seed whose read changed, including the ones whose state did not move;
- the real-specimen run, where the question is which zone each document now
  reads, not only its outcome.
This script reports all of it, so that every flip can be attributed.

A seed can also change only in how its read was reached (`check_states`,
`line1_flagged`, the damaged-recovery flags, `retry_stop`, `retry_variant_id`)
while it reads the same. Those seeds are counted apart, as flag-only changes.
The 214 seeds that `synth_ab_diff` did not list in the 2026-09-28 headline A/B
mix both kinds.

Standard library only, matching every other `tools/` script.

## Arm layout

An arm is one directory, holding whatever the A/B measured:

    ARM/<name>.json   one `synthpass-bench --out` report per format, e.g.
                      synthpass-bench --document-type td1 --profile clean
                        --count 100 --seed 0 --out ARM/td1.json
    ARM/real/         one `provider-bench` run, e.g.
                      provider-bench --real-specimens --mrz-only --dump-ocr
                        --dump-ocr-hits --out ARM/real/report.json

- **Synthetic reports** are paired by file name. A pair must cover the same
  document type, profile and seeds; `synth_ab_diff.compare` checks that.
- **The real run** is compared when both arms have
  `real/provider-bench-ocr-outcomes.jsonl`.
- **Real zones** are compared when both arms also have
  `real/provider-bench-miss-ocr-dump.jsonl`. `--dump-ocr` writes that file, and
  `--dump-ocr-hits` adds the hits' zones to it. Measure both arms with the same
  flags.

## Usage

    python tools/bench_ab_diff.py BEFORE_ARM AFTER_ARM
    python tools/bench_ab_diff.py BEFORE_ARM AFTER_ARM --asset <asset_id> ...
    python tools/bench_ab_diff.py BEFORE_ARM AFTER_ARM --json

`--asset` (repeatable) adds a status block for a named real specimen, whether or
not it changed. Give the `asset_id` as the outcome ledger spells it, such as
`passports/Kosovo_Passport_Specimen_P0_RKS_2023_mrz.jpg`.

**Exit status:**
- **0** if every pair compared is an A/B.
- **1** if a pair is not an A/B:
  - a synthetic pair covering different documents;
  - real runs over different assets, a different corpus manifest, or different
    image bytes;
  - a report whose emitted counts disagree with its own seeds.
- **2** on:
  - unreadable input;
  - two arms with nothing in common to compare;
  - an arm measured with a private or local track.

## What it prints

- **Synthetic, per report:**
  - hits and strict-name hits;
  - accepted reads: hits plus `document_number_mismatch` misses, as #578 counts
    them;
  - the seeds whose accepted read has a wrong document code or issuer, and the
    gated `prefix_wrong_accepts`;
  - `synth_ab_diff`'s correct, wrong-accept and valid-miss totals;
  - any difference in the two arms' OCR arms or model paths.
- **Every seed whose read changed:** its outcome, its miss kind or any field
  value.
  - Its `synth_ab_diff` class. `refused -> refused` is a miss whose read changed.
  - Its miss kind and `retry_stop` in each arm.
  - Each changed field, and the zone's lines when they changed.
  - A `retry_stop` that differs between the arms deserves a second look. The OCR
    retry loop is wall-clock budgeted, so a `budget` stop can move a seed without
    any code change.
- **The flag-only changes,** grouped by the keys that moved.
- **Real:**
  - **Each arm's identity:** commit, dirty tree, flags, OCR arms, corpus
    manifest, class-sweep arm and model paths. A different corpus or different
    image bytes means the pair is not an A/B. Any other difference is printed,
    since it may be the lever under test.
  - **Tier-1 hits over both denominators,** and names exact among the hits with
    name truth.
  - **An alarm** for any `false_positive_mrz`, and for any checksum-valid read of
    a `checksum_failed_specimen`.
  - Every document whose outcome, MRZ format or names result changed.
  - Every document whose recovered zone changed, with its lines in each arm and,
    where the corpus has a fixture, which lines match it.
  - Every document dumped in one arm only, named, with its outcome in each arm.
    `provider-bench` does not dump every outcome, so a hit that becomes a wrong
    accepted read loses its zone.

## Old reports

A report written before a key existed is compared on what both arms have:
- Accepted-read counts are recomputed from each seed, so reports from before
  #578 compare too.
- If either arm lacks `miss_kind`, outcomes compare only as hit or miss, and
  accepted reads print `n/a`.
- If either arm lacks `check_states`, no seed is classed a valid miss, and valid
  misses print `n/a`.

## Privacy

Synthetic truth is generated and carries no real person.

The real comparison covers the public corpus only. An arm is refused, with exit
status 2 and before anything is printed, when:
- its run archive shows `--include-private` or `--include-local`; or
- an asset sits on a `private` or `local` track.

The public corpus's MRZ lines are already quoted in the dated notes in
`knowledge/benchmarks/`. This prints recovered zones, fixture agreement and
outcomes, and never a dump's raw OCR text.
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
CURRENT_RUN = "provider-bench-ocr-current-run.txt"
REPORT = "report.json"
PROVIDER = "mrz"
PREFIX_FIELDS = ("document_type", "issuing_country")
# The five formats in the order the dated notes list them; any other report
# name sorts after them, alphabetically.
FORMAT_ORDER = ("td1", "td2", "td3", "mrva", "mrvb")
# Per-seed keys that record how a read was reached, not what it read.
FLAG_KEYS = ("check_states", "line1_flagged", "tier1_damaged_recovery",
             "retry_damaged_recovery", "retry_stop", "retry_variant_id")
PRIVATE_FLAGS = ("--include-private", "--include-local")
PRIVATE_TRACKS = ("private", "local")
# Flags that change nothing about what a run measures.
NEUTRAL_FLAGS = ("--progress", "--verbose")


class Refused(Exception):
    """An arm that must not be compared at all."""


def load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def load_jsonl(path: Path, provider: str | None = None) -> dict[str, dict]:
    """Rows keyed by `asset_id`. With `provider`, rows naming another provider
    are skipped. A repeated `asset_id` is an error, never a silent overwrite."""
    rows = {}
    for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        if not line.strip():
            continue
        row = json.loads(line)
        if provider is not None and row.get("provider", provider) != provider:
            continue
        asset = row["asset_id"]
        if asset in rows:
            raise ValueError(f"{path.name}:{number}: asset_id {asset!r} appears twice")
        rows[asset] = row
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


def supports(report: dict, key: str) -> bool:
    """Whether `report` was written by a bench that emits `key`. A seed omits a
    key it has no value for (a hit has no `miss_kind`, a miss with no parse no
    `check_states`), so a report predates the key only when no seed carries it."""
    return any(key in d for d in report.get("results", []))


def outcome(doc: dict, with_kind: bool = True) -> str:
    """`hit`, or the miss kind (`miss` when the reports predate `miss_kind`)."""
    if doc.get("hit"):
        return "hit"
    return str(doc.get("miss_kind")) if with_kind else "miss"


def is_accepted_read(doc: dict) -> bool:
    """A hit, or a checksum-valid read whose document number is wrong (#578)."""
    return bool(doc.get("hit")) or doc.get("miss_kind") == "document_number_mismatch"


def is_prefix_wrong(doc: dict) -> bool:
    return is_accepted_read(doc) and any(f in PREFIX_FIELDS for f in synth.wrong_fields(doc))


def read_of(doc: dict, with_kind: bool = True) -> tuple:
    """What a seed read: its outcome and every field's error and value. Timing,
    flags and retry telemetry are left out; `flags_of` holds the flags."""
    return (
        outcome(doc, with_kind),
        tuple((f.get("field"), f.get("cer", 0), f.get("got")) for f in doc.get("fields", [])),
    )


def flags_of(doc: dict) -> dict:
    return {k: doc[k] for k in FLAG_KEYS if k in doc}


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


def synthetic_summary(report: dict, with_kind: bool, with_checks: bool) -> dict:
    docs = report.get("results", [])
    if not with_checks:
        docs = [{k: v for k, v in d.items() if k != "check_states"} for d in docs]
    states = Counter(synth.state(d) for d in docs)
    return {
        "hits": sum(1 for d in docs if d.get("hit")),
        "strict_hits": report.get("strict_hits"),
        "accepted_reads": sum(1 for d in docs if is_accepted_read(d)) if with_kind else None,
        "prefix_wrong_accepted_read_seeds": [d["seed"] for d in docs if is_prefix_wrong(d)] if with_kind else None,
        "prefix_wrong_accepts": report.get("prefix_wrong_accepts"),
        "correct": states["correct"],
        "wrong_accepts": states["wrong"],
        "valid_misses": states["valid-miss"] if with_checks else None,
        "count": len(docs),
    }


def emitted_mismatches(report: dict, summary: dict) -> list[str]:
    """Where a report's own counts disagree with the counts recomputed from its
    seeds (reports from #578 on carry both)."""
    out = []
    for key, recomputed in (("accepted_reads", summary["accepted_reads"]),
                            ("prefix_wrong_accepted_reads",
                             None if summary["prefix_wrong_accepted_read_seeds"] is None
                             else len(summary["prefix_wrong_accepted_read_seeds"]))):
        emitted = report.get(key)
        if recomputed is not None and isinstance(emitted, int) and emitted != recomputed:
            out.append(f"{key}: the report says {emitted}, its seeds give {recomputed}")
    return out


def compare_synthetic(before: dict, after: dict) -> dict:
    with_kind = supports(before, "miss_kind") and supports(after, "miss_kind")
    with_checks = supports(before, "check_states") and supports(after, "check_states")

    def for_state(doc):
        return doc if with_checks else {k: v for k, v in doc.items() if k != "check_states"}

    sb = synthetic_summary(before, with_kind, with_checks)
    sa = synthetic_summary(after, with_kind, with_checks)
    problems = list(synth.compare(before, after)["problems"])
    problems += [f"before: {p}" for p in emitted_mismatches(before, sb)]
    problems += [f"after: {p}" for p in emitted_mismatches(after, sa)]
    notes = [f"{key} differs: {before.get(key)} -> {after.get(key)}"
             for key in ("ocr_arms", "model_paths")
             if key in before and key in after and before[key] != after[key]]
    if not with_kind:
        notes.append("a report predates miss_kind: outcomes compare as hit or miss only")
    if not with_checks:
        notes.append("a report predates check_states: no seed is classed a valid miss")

    rb = {d["seed"]: d for d in before.get("results", [])}
    ra = {d["seed"]: d for d in after.get("results", [])}
    changed, flag_only = [], []
    for seed in sorted(set(rb) & set(ra)):
        b, a = rb[seed], ra[seed]
        if read_of(b, with_kind) == read_of(a, with_kind):
            fb, fa = flags_of(b), flags_of(a)
            keys = sorted(k for k in set(fb) & set(fa) if fb[k] != fa[k])
            if keys:
                flag_only.append({"seed": seed, "keys": keys})
            continue
        (zone_b, truth_b), (zone_a, truth_a) = zone(b), zone(a)
        changed.append({
            "seed": seed,
            "class": f"{synth.state(for_state(b))} -> {synth.state(for_state(a))}",
            "outcome": [outcome(b, with_kind), outcome(a, with_kind)],
            "retry_stop": [synth.retry_cell(b, "retry_stop"), synth.retry_cell(a, "retry_stop")],
            "changes": synth.field_changes(b, a),
            "zone": None if zone_b == zone_a else {
                "before": zone_b, "after": zone_a, "truth": truth_b or truth_a,
            },
        })
    return {"problems": problems, "notes": notes, "before": sb, "after": sa,
            "changed": changed, "flag_only": flag_only}


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


def run_archive(real_dir: Path) -> dict | None:
    """The run archive `provider-bench-ocr-current-run.txt` points at, if any."""
    pointer = real_dir / CURRENT_RUN
    if not pointer.is_file():
        return None
    path = real_dir / pointer.read_text(encoding="utf-8").strip()
    return load_json(path) if path.is_file() else None


def private_reason(archive: dict | None, assets) -> str | None:
    flags = [f for f in (archive or {}).get("flags") or [] if f in PRIVATE_FLAGS]
    if flags:
        return f"was measured with {', '.join(flags)}"
    tracks = sorted({part for a in assets for part in Path(a).parts if part in PRIVATE_TRACKS})
    if tracks:
        return f"holds assets from the {', '.join(tracks)} track"
    return None


def normalized_flags(flags: list[str] | None) -> list[str] | None:
    """A run's flags without `--out`'s value and without flags that measure
    nothing (`--progress`, `--verbose`)."""
    if flags is None:
        return None
    out, skip = [], False
    for flag in flags:
        if skip:
            skip = False
        elif flag == "--out":
            skip = True
        elif flag not in NEUTRAL_FLAGS:
            out.append(flag)
    return out


def identity(archive: dict | None, report: dict | None) -> dict:
    archive, report = archive or {}, report or {}
    return {
        "git_commit": archive.get("git_commit"),
        "working_tree_dirty": archive.get("working_tree_dirty"),
        "flags": normalized_flags(archive.get("flags")),
        "ocr_arms": archive.get("ocr_arms"),
        "corpus_manifest_sha256": archive.get("corpus_manifest_sha256"),
        "mrz_class_sweep_arm": report.get("mrz_class_sweep_arm"),
        "model_paths": report.get("model_paths"),
    }


def mrz_provider(report: dict | None) -> dict:
    providers = (report or {}).get("providers") or []
    for p in providers:
        if p.get("provider_id") == PROVIDER:
            return p
    return providers[0] if providers else {}


def real_summary(outcomes: dict[str, dict], report: dict | None) -> dict:
    hits = sum(1 for r in outcomes.values() if r.get("outcome") == "hit")
    provider = mrz_provider(report)
    rate = (provider.get("tier1_hit_rate") or {}).get("rate")
    named = [r for r in outcomes.values() if r.get("outcome") == "hit" and r.get("names_exact") is not None]
    return {
        "counts": dict(sorted(Counter(str(r.get("outcome")) for r in outcomes.values()).items())),
        "hits": hits,
        "scored": round(hits / rate) if rate else None,
        "documents": len(outcomes),
        "hits_by_format": dict(sorted(Counter(str(r.get("mrz_format")) for r in outcomes.values()
                                              if r.get("outcome") == "hit").items())),
        "names_exact": sum(1 for r in named if r.get("names_exact")),
        "names_scored": len(named),
        "false_positive_mrz": sum(1 for r in outcomes.values() if r.get("outcome") == "false_positive_mrz"),
        "checksum_valid_on_failed_specimen": provider.get("checksum_valid_on_failed_specimen"),
    }


def real_state(row: dict | None) -> dict | None:
    if row is None:
        return None
    return {k: row.get(k) for k in ("outcome", "mrz_format", "miss_reason", "names_exact", "name_error")}


def compare_real(before_dir: Path, after_dir: Path, assets: list[str]) -> dict:
    ob, oa = load_jsonl(before_dir / OUTCOMES), load_jsonl(after_dir / OUTCOMES)
    archive_b, archive_a = run_archive(before_dir), run_archive(after_dir)
    for side, archive, rows in (("before", archive_b, ob), ("after", archive_a, oa)):
        reason = private_reason(archive, rows)
        if reason:
            raise Refused(f"the {side} arm {reason}; only the public corpus is compared")
    report_b = load_json(before_dir / REPORT) if (before_dir / REPORT).is_file() else None
    report_a = load_json(after_dir / REPORT) if (after_dir / REPORT).is_file() else None
    zones = (before_dir / ZONES).is_file() and (after_dir / ZONES).is_file()
    zb = load_jsonl(before_dir / ZONES, PROVIDER) if zones else {}
    za = load_jsonl(after_dir / ZONES, PROVIDER) if zones else {}

    problems, notes = [], []
    if set(ob) != set(oa):
        problems.append(f"asset sets differ: {len(set(ob) - set(oa))} only before, "
                        f"{len(set(oa) - set(ob))} only after")
    ids = [identity(archive_b, report_b), identity(archive_a, report_a)]
    for key in ids[0]:
        b, a = ids[0][key], ids[1][key]
        if key == "git_commit" or b == a:
            continue
        if b is None or a is None:
            notes.append(f"{key} is recorded in one arm only")
        elif key == "corpus_manifest_sha256":
            problems.append(f"corpus manifests differ: {b[:12]} -> {a[:12]}")
        else:
            notes.append(f"{key} differs: {b} -> {a}")
    new_bytes = sorted(k for k in set(zb) & set(za)
                       if zb[k].get("source_sha256") and za[k].get("source_sha256")
                       and zb[k]["source_sha256"] != za[k]["source_sha256"])
    if new_bytes:
        problems.append(f"image bytes differ for {len(new_bytes)} asset(s): {', '.join(new_bytes[:5])}")

    common = sorted(set(ob) & set(oa))
    outcome_changes = [{"asset": k, "before": real_state(ob[k]), "after": real_state(oa[k])} for k in common
                       if (ob[k].get("outcome"), ob[k].get("mrz_format")) != (oa[k].get("outcome"), oa[k].get("mrz_format"))]
    names_changes = [{"asset": k, "before": real_state(ob[k]), "after": real_state(oa[k])} for k in common
                     if (ob[k].get("names_exact"), ob[k].get("name_error")) != (oa[k].get("names_exact"), oa[k].get("name_error"))]

    def zone_entry(asset):
        b, a = zb.get(asset), za.get(asset)
        fixture = (b or a or {}).get("ground_truth_mrz")
        lines_b = b.get("recovered_mrz_lines") if b else None
        lines_a = a.get("recovered_mrz_lines") if a else None
        return {
            "asset": asset,
            "outcome": [(ob.get(asset) or {}).get("outcome"), (oa.get(asset) or {}).get("outcome")],
            "dumped": [b is not None, a is not None],
            "before": lines_b,
            "after": lines_a,
            "fixture": [fixture_verdict(lines_b, fixture) if b else None,
                        fixture_verdict(lines_a, fixture) if a else None],
        }

    zone_changes = [zone_entry(k) for k in sorted(set(zb) & set(za))
                    if zb[k].get("recovered_mrz_lines") != za[k].get("recovered_mrz_lines")]
    one_arm = [zone_entry(k) for k in sorted(set(zb) ^ set(za))]
    watched = []
    for asset in assets:
        entry = zone_entry(asset)
        entry["known"] = asset in ob or asset in oa
        entry["states"] = [real_state(ob.get(asset)), real_state(oa.get(asset))]
        watched.append(entry)
    return {
        "problems": problems,
        "notes": notes,
        "identity": ids,
        "summary": [real_summary(ob, report_b), real_summary(oa, report_a)],
        "zones_compared": zones,
        "dumped": [len(zb), len(za)],
        "outcome_changes": outcome_changes,
        "names_changes": names_changes,
        "zone_changes": zone_changes,
        "dumped_in_one_arm_only": one_arm,
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


def na(value) -> str:
    return "n/a" if value is None else str(value)


def join_zone(z) -> str:
    if z is None:
        return "-"
    return " / ".join(z) if isinstance(z, list) else z.replace("\n", " / ")


def short_identity(ident: dict) -> str:
    commit = (ident["git_commit"] or "unknown")[:7]
    dirty = {True: ", dirty tree", False: "", None: ", tree state unknown"}[ident["working_tree_dirty"]]
    corpus = (ident["corpus_manifest_sha256"] or "unknown")[:12]
    return f"commit {commit}{dirty}; corpus {corpus}; class sweep {na(ident['mrz_class_sweep_arm'])}"


def render_synthetic(name: str, r: dict) -> list[str]:
    b, a = r["before"], r["after"]
    lines = [
        f"{name}: hits {b['hits']} -> {a['hits']}   strict {na(b['strict_hits'])} -> {na(a['strict_hits'])}   "
        f"accepted {na(b['accepted_reads'])} -> {na(a['accepted_reads'])}   "
        f"accepted with a wrong code or issuer {na(b['prefix_wrong_accepted_read_seeds'])} -> "
        f"{na(a['prefix_wrong_accepted_read_seeds'])}   "
        f"gated prefix-wrong accepts {na(b['prefix_wrong_accepts'])} -> {na(a['prefix_wrong_accepts'])}   "
        f"correct {b['correct']} -> {a['correct']}   wrong accepts {b['wrong_accepts']} -> {a['wrong_accepts']}   "
        f"valid misses {na(b['valid_misses'])} -> {na(a['valid_misses'])}   "
        f"reads changed {len(r['changed'])}, flag-only {len(r['flag_only'])} (of {a['count']})"]
    lines += [f"  note: {n}" for n in r["notes"]]
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
    groups: dict[tuple, list[int]] = {}
    for f in r["flag_only"]:
        groups.setdefault(tuple(f["keys"]), []).append(f["seed"])
    for keys, seeds in sorted(groups.items()):
        lines.append(f"  flag-only ({', '.join(keys)}): seeds {', '.join(map(str, seeds))}")
    return lines


def render_real(real: dict) -> list[str]:
    sb, sa = real["summary"]
    lines = ["real specimens:",
             f"  before: {short_identity(real['identity'][0])}",
             f"  after:  {short_identity(real['identity'][1])}"]
    lines += [f"  note: {n}" for n in real["notes"]]
    if sb["scored"] and sa["scored"]:
        lines.append(f"  tier-1 hits {sb['hits']} / {sb['scored']} -> {sa['hits']} / {sa['scored']} scored, "
                     f"of {sb['documents']} -> {sa['documents']} documents")
    else:
        lines.append(f"  tier-1 hits {sb['hits']} -> {sa['hits']} of {sb['documents']} -> {sa['documents']} "
                     f"documents (no report.json, so no scored denominator)")
    lines.append(f"  names exact among hits with name truth {sb['names_exact']} / {sb['names_scored']} -> "
                 f"{sa['names_exact']} / {sa['names_scored']}")
    for key in sorted(set(sb["counts"]) | set(sa["counts"])):
        lines.append(f"  {key}: {sb['counts'].get(key, 0)} -> {sa['counts'].get(key, 0)}")
    lines.append(f"  hits by format: {arrow([sb['hits_by_format'], sa['hits_by_format']])}")
    for side, s in (("before", sb), ("after", sa)):
        if s["false_positive_mrz"]:
            lines.append(f"  ALARM ({side}): {s['false_positive_mrz']} false_positive_mrz")
        if s["checksum_valid_on_failed_specimen"]:
            lines.append(f"  ALARM ({side}): {s['checksum_valid_on_failed_specimen']} checksum-valid read(s) "
                         f"of a checksum_failed_specimen")
    lines.append(f"  outcome changes: {len(real['outcome_changes'])}")
    for c in real["outcome_changes"]:
        b, a = c["before"], c["after"]
        lines.append(f"    {c['asset']}: {b['outcome']}/{b['mrz_format']} -> {a['outcome']}/{a['mrz_format']}")
        if a["miss_reason"] != b["miss_reason"]:
            lines.append(f"      reason after: {a['miss_reason']}")
    lines.append(f"  names changes: {len(real['names_changes'])}")
    for c in real["names_changes"]:
        b, a = c["before"], c["after"]
        lines.append(f"    {c['asset']}: names_exact {b['names_exact']} -> {a['names_exact']}, "
                     f"name_error {b['name_error']} -> {a['name_error']}")
    if real["zones_compared"]:
        lines.append(f"  zones dumped {arrow(real['dumped'])}; zone changes: {len(real['zone_changes'])}")
        for z in real["zone_changes"]:
            fixture = "" if z["fixture"] == [None, None] else f"   fixture {arrow(z['fixture'])}"
            lines.append(f"    {z['asset']}: {arrow(z['outcome'])}{fixture}")
            lines.append(f"      before {join_zone(z['before'])}")
            lines.append(f"      after  {join_zone(z['after'])}")
        if real["dumped_in_one_arm_only"]:
            lines.append(f"  dumped in one arm only, zones not comparable: {len(real['dumped_in_one_arm_only'])}")
            for z in real["dumped_in_one_arm_only"]:
                side = "before" if z["dumped"][0] else "after"
                lines.append(f"    {z['asset']}: {arrow(z['outcome'])}, dumped {side} only")
    else:
        lines.append(f"  zones not compared: an arm has no {ZONES} (measure with --dump-ocr)")
    for w in real["watched"]:
        if not w["known"]:
            lines.append(f"  watched {w['asset']}: in neither run")
            continue
        sb_, sa_ = w["states"]
        state = f"{(sb_ or {}).get('outcome')}/{(sb_ or {}).get('mrz_format')} -> " \
                f"{(sa_ or {}).get('outcome')}/{(sa_ or {}).get('mrz_format')}"
        if all(w["dumped"]):
            fixture = "" if w["fixture"] == [None, None] else f"   fixture {arrow(w['fixture'])}"
            lines.append(f"  watched {w['asset']}: {state}   "
                         f"zone {'changed' if w['before'] != w['after'] else 'unchanged'}{fixture}")
        else:
            where = "neither arm" if not any(w["dumped"]) else ("the before arm only" if w["dumped"][0]
                                                                else "the after arm only")
            lines.append(f"  watched {w['asset']}: {state}   zone not comparable: dumped in {where}")
        lines.append(f"      before {join_zone(w['before']) if w['dumped'][0] else 'not dumped'}")
        lines.append(f"      after  {join_zone(w['after']) if w['dumped'][1] else 'not dumped'}")
    return lines


def render(result: dict) -> str:
    lines = [f"NOT AN A/B: {p}" for p in problems_of(result)]
    for name, r in result["synthetic"].items():
        lines += render_synthetic(name, r)
    for side, names in result["synthetic_in_one_arm_only"].items():
        if names:
            lines.append(f"only in the {side} arm, not compared: {', '.join(names)}")
    if result["real_in_one_arm_only"]:
        lines.append(f"real run only in the {result['real_in_one_arm_only']} arm, not compared")
    if result["real"]:
        lines += render_real(result["real"])
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
    except Refused as e:
        print(f"bench_ab_diff: refused: {e}", file=sys.stderr)
        return 2
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
