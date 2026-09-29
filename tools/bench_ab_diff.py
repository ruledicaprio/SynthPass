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
    python tools/bench_ab_diff.py A B --expect-identical [--check-pass-trace] [--ignore KEY ...] [--json]

`--asset` (repeatable) adds a status block for a named real specimen, whether or
not it changed. Give the `asset_id` as the outcome ledger spells it, such as
`passports/Kosovo_Passport_Specimen_P0_RKS_2023_mrz.jpg`.

`--expect-identical` is the neutrality mode, for a change that should move
nothing (see below). `--check-pass-trace` and `--ignore` are refused without
it, and `--asset` is refused with it: that mode prints no zones.

**Exit status:**
- **0** if every pair compared is an A/B. Under `--expect-identical`, also
  identical.
- **1** if a pair is not an A/B:
  - a synthetic pair covering different documents;
  - real runs over different assets, a different corpus manifest, or different
    image bytes;
  - a report whose emitted counts disagree with its own seeds;
  - under `--expect-identical`, a `budget` stop in one arm only.
- **2** on:
  - unreadable input;
  - two arms with nothing in common to compare;
  - an arm measured with a private or local track;
  - under `--expect-identical`, `--check-pass-trace` when neither arm has a
    pass trace.
- **3**, only under `--expect-identical`: the pair is an A/B but not identical.
  A difference in a row, a dump, a pass trace or a dumped text; a dump row or
  block in one arm only; a format or the real run in one arm only.

Codes 0, 1 and 2 mean the same in both modes. When several apply, 2 wins over 1
and 1 wins over 3.

## Neutrality mode

`--expect-identical` answers one question for a change that should not move a
single read: does every document read the same in both arms? It prints counts,
seeds, asset ids and sha256 prefixes, and **never a line of OCR or zone text**,
so its output is safe for a public job summary (ADR-0027). One line per format
and one for the real run, then `NEUTRAL` or `NOT NEUTRAL`; at most 8 ids are
listed per failing check, and `--json` carries the same result.

It first runs the default mode's checks, which decide exit 1 and 2. Then:

- **Ignored keys.** `elapsed_ms`, `ocr_ms` and `run_manifest` always.
  `ocr_text` and `ocr_passes` only where just one arm has them. Anything else
  needs `--ignore KEY` (repeatable). An ignored key is dropped at any depth of a
  row, so `--ignore chargrid` also drops it inside each pass record. The
  one-sided rule for `ocr_text` and `ocr_passes` applies to a row's top level.
  The first output line names every key.
- **Synthetic, per format:**
  - `results[]` is identical per seed after the ignores;
  - the `--- seed N raw OCR lines ---` blocks of `ARM/<name>.stdout` are
    byte-identical, when both arms have that file;
  - an arm's `ocr_text` equals the text the other arm printed for the seed.
- **Real:**
  - the outcome ledger is identical per asset;
  - the dump rows are identical, with `raw_ocr_text` compared by sha256 and
    never printed. A row in one arm only is a difference.
- **The real pass trace rows** (`provider-bench-ocr-passes.jsonl`) are identical
  per asset, whenever both arms have the file, with or without
  `--check-pass-trace`. With the file in one arm only, they are not compared,
  which is no failure, as for the dump.
- **`--check-pass-trace`,** in whichever arm has the trace (`ocr_passes` in a
  synthetic report; `provider-bench-ocr-passes.jsonl` on the real side):
  - orders and ids are contiguous, and `general` appears only at 0;
  - readings are empty exactly when the outcome is `failed` or
    `no_mrz_shaped_lines`;
  - one `accepted` pass exactly when the stop is `general_valid` or
    `variant_valid`, and it is the last pass and equals `retry_variant_id`;
  - the appended and accepted readings form the tail of `ocr_text`;
  - on real arms, the trace's `ocr_text` equals the dump's `raw_ocr_text` (by
    hash), and its retry fields equal the ledger's.
- **Budget stops** are listed by id. One in both arms is reported, with the
  dumped texts compared by hash. One in a single arm is not an A/B: the retry
  loop is wall-clock budgeted, so the two arms did not run the same search.

A difference in the run identity (flags, OCR arms, model paths) is named as a
note and does not change the verdict.

`report.json` is not read in this mode, so `field_correctness` is not compared
here. The ledger, dump and trace rows are.

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
  - **Field correctness per document** (`report.json`'s
    `documents_detail[].field_correctness`, #574): per field, how many
    documents went exact -> wrong, wrong -> exact, exact -> unread and
    unread -> exact, and the documents with any exact -> non-exact field,
    named by asset id and field name. The maps hold field names and the verdicts
    `exact`, `wrong` and `unread`, never a value. A field is compared on the
    documents whose map has it in both arms. An arm whose report lacks the maps
    prints "not recorded".
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
- If either arm's `report.json` lacks `field_correctness`, or the report is
  absent, the per-document field transitions print "not recorded".

## Privacy

Synthetic truth is generated and carries no real person.

The real comparison covers the public corpus only. An arm is refused, with exit
status 2 and before anything is printed, when:
- its run archive shows `--include-private` or `--include-local`; or
- an asset sits on a `private` or `local` track.

The public corpus's MRZ lines are already quoted in the dated notes in
`knowledge/benchmarks/`. The default mode prints recovered zones, fixture
agreement and outcomes, and never a dump's raw OCR text. `--expect-identical`
prints no text at all.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import re
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
# `CoreField::ALL`'s order, the order the report's field names print in. A name
# outside it (a field added later) sorts after these.
FIELD_ORDER = ("document_type", "issuing_country", "document_number", "surname", "given_names",
               "nationality", "date_of_birth", "sex", "date_of_expiry", "personal_number",
               "optional_data_1", "optional_data_2")
FIELD_VERDICTS = ("exact", "wrong", "unread")
# The transitions counted per field: (before, after) verdicts.
FIELD_TRANSITIONS = (("exact", "wrong"), ("wrong", "exact"), ("exact", "unread"), ("unread", "exact"))
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


def field_correctness_maps(report: dict | None) -> dict[str, dict[str, str]] | None:
    """The mrz provider's per-document `field_correctness`, by asset id: field
    name -> `exact`, `wrong` or `unread`. `None` when the report is absent or
    records none, an arm measured before #574 PR 2. A document without ground
    truth has no map and is not in the result."""
    rows = mrz_provider(report).get("documents_detail") or []
    maps = {r["asset_id"]: r["field_correctness"] for r in rows
            if r.get("asset_id") and isinstance(r.get("field_correctness"), dict)}
    return maps or None


def field_order(name: str) -> tuple[int, str]:
    return (FIELD_ORDER.index(name) if name in FIELD_ORDER else len(FIELD_ORDER), name)


def compare_field_correctness(before: dict | None, after: dict | None) -> dict:
    """Per-field transitions between two arms' maps, by field name and asset id,
    never by value. A field is compared on the documents whose map has it in
    both arms. `transitions` and `regressions` are `None` unless both arms
    recorded the maps."""
    out = {"recorded": [before is not None, after is not None], "documents": [None, None],
           "compared": None, "transitions": None, "regressions": None}
    if before is None or after is None:
        return out
    common = sorted(set(before) & set(after))
    counts: dict[str, Counter] = {}
    regressions = []
    for asset in common:
        lost = {}
        for name in sorted(set(before[asset]) & set(after[asset]), key=field_order):
            # Only the three verdicts are ever kept: a map that carried anything
            # else, a value, would still never reach the output.
            pair = tuple(v if v in FIELD_VERDICTS else "other" for v in (before[asset][name], after[asset][name]))
            if pair in FIELD_TRANSITIONS:
                counts.setdefault(name, Counter())[f"{pair[0]}->{pair[1]}"] += 1
            if pair[0] == "exact" and pair[1] != "exact":
                lost[name] = pair[1]
        if lost:
            regressions.append({"asset": asset, "fields": lost})
    out.update({
        "documents": [len(before), len(after)],
        "compared": len(common),
        "transitions": {name: {f"{b}->{a}": counts[name][f"{b}->{a}"] for b, a in FIELD_TRANSITIONS}
                        for name in sorted(counts, key=field_order)},
        "regressions": regressions,
    })
    return out


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
        "field_correctness": compare_field_correctness(field_correctness_maps(report_b),
                                                       field_correctness_maps(report_a)),
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


def render_field_correctness(fc: dict) -> list[str]:
    """Field names, asset ids and counts only: the maps hold no value."""
    if fc["transitions"] is None:
        missing = [side for side, recorded in zip(("before", "after"), fc["recorded"]) if not recorded]
        return [f"  field correctness: not recorded in the {' and '.join(missing)} arm's report.json "
                f"(documents_detail.field_correctness)"]
    lines = [f"  field correctness: {fc['compared']} documents compared "
             f"({fc['documents'][0]} -> {fc['documents'][1]} with truth)"]
    for name, counts in fc["transitions"].items():
        lines.append(f"    {name}: " + ", ".join(f"{t} {n}" for t, n in counts.items()))
    if not fc["transitions"]:
        lines.append("    no transitions")
    lines.append(f"  documents with an exact -> non-exact field: {len(fc['regressions'])}")
    for r in fc["regressions"]:
        lines.append(f"    {r['asset']}: " + ", ".join(f"{name} exact -> {now}" for name, now in r["fields"].items()))
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
    lines += render_field_correctness(real["field_correctness"])
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


# --- Neutrality mode (`--expect-identical`) ---------------------------------
#
# The default mode above explains what moved. This mode answers one question,
# for a change that should move nothing: is every document read the same? It
# prints counts, seeds, asset ids and sha256 prefixes, never a line of OCR or
# zone text, so its output can go into a public job summary (ADR-0027).

# Keys that never carry a measured result: timing, and the archive a row came from.
ALWAYS_IGNORED = ("elapsed_ms", "ocr_ms", "run_manifest")
# Keys only one arm may have measured (`--dump-ocr-passes` on one side). They are
# ignored where just one arm carries them, compared where both do.
ONE_SIDED_IGNORED = ("ocr_text", "ocr_passes")
PASSES = "provider-bench-ocr-passes.jsonl"
MAX_IDS = 8
DUMP_BLOCK = re.compile(r"^--- seed (\d+) raw OCR lines ---$")
DUMP_LINE = re.compile(r'^  \[(\d+)\] (".*")$')
EMPTY_PASS_OUTCOMES = ("failed", "no_mrz_shaped_lines")
ACCEPTING_STOPS = ("general_valid", "variant_valid")
RETRY_FIELDS = ("retry_stop", "retry_variant_id", "retry_budget_hit")
_MISSING = object()


def digest(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def text_digest(value) -> str | None:
    return digest(value) if isinstance(value, str) else None


RUST_ESCAPE = re.compile(r"\\(?:u\{([0-9a-fA-F]+)\}|(.))", re.DOTALL)
RUST_PLAIN = re.compile(r'[^\\"]+')
RUST_SIMPLE_ESCAPES = {'"': '"', "\\": "\\", "n": "\n", "r": "\r", "t": "\t", "0": "\0", "'": "'"}


def rust_debug_str(literal: str) -> str:
    """The string a Rust `{:?}` literal spells, read left to right in one pass.
    Rust escapes a quote, a backslash, `\\n`, `\\r`, `\\t`, `\\0`, `\\'` and
    `\\u{XXXX}`. Any other escape, or a literal that is not quoted, is a
    ValueError."""
    if len(literal) < 2 or literal[0] != '"' or literal[-1] != '"':
        raise ValueError("not a quoted Rust string literal")
    body, pos, out = literal[1:-1], 0, []
    while pos < len(body):
        if body[pos] == "\\":
            escape = RUST_ESCAPE.match(body, pos)
            if escape is None:
                raise ValueError("a dangling backslash in a Rust string literal")
            if escape.group(1) is not None:
                code = int(escape.group(1), 16)
                if code > 0x10FFFF or 0xD800 <= code <= 0xDFFF:
                    raise ValueError(f"\\u{{{escape.group(1)}}} is not a character")
                out.append(chr(code))
            elif escape.group(2) in RUST_SIMPLE_ESCAPES:
                out.append(RUST_SIMPLE_ESCAPES[escape.group(2)])
            else:
                raise ValueError("an unknown escape in a Rust string literal")
            pos = escape.end()
        else:
            plain = RUST_PLAIN.match(body, pos)
            if plain is None:
                raise ValueError("an unescaped quote in a Rust string literal")
            out.append(plain.group())
            pos = plain.end()
    return "".join(out)


def dump_blocks(path: Path) -> dict[int, list[str]]:
    """The `--- seed N raw OCR lines ---` blocks of a `synthpass-bench` stdout,
    by seed: each block's header and its `  [i] "..."` lines, verbatim."""
    out, seed, block = {}, None, []
    for line in path.read_text(encoding="utf-8").splitlines():
        header = DUMP_BLOCK.match(line)
        if header:
            if seed is not None:
                out[seed] = block
            seed, block = int(header.group(1)), [line]
        elif seed is not None:
            if DUMP_LINE.match(line):
                block.append(line)
            else:
                out[seed], seed, block = block, None, []
    if seed is not None:
        out[seed] = block
    return out


def block_text(block: list[str]) -> str:
    """The OCR text a block prints: its lines decoded and joined by newlines."""
    return "\n".join(rust_debug_str(DUMP_LINE.match(line).group(2)) for line in block[1:])


def failing(ids: list) -> dict:
    """A failed check: how many, and the first `MAX_IDS` ids."""
    return {"failed": len(ids), "ids": [str(i) for i in ids[:MAX_IDS]]}


def without_keys(value, ignore: set[str]):
    """`value` with every ignored key dropped, at any depth of dicts and lists."""
    if isinstance(value, dict):
        return {k: without_keys(v, ignore) for k, v in value.items() if k not in ignore}
    if isinstance(value, list):
        return [without_keys(v, ignore) for v in value]
    return value


def comparable(row: dict, ignore: set[str]) -> dict:
    """A row without its ignored keys, at any depth, and with a top-level
    `raw_ocr_text` reduced to its sha256, so a comparison can never surface the
    text."""
    out = without_keys(row, ignore)
    if isinstance(out.get("raw_ocr_text"), str):
        out["raw_ocr_text"] = "sha256:" + digest(out["raw_ocr_text"])
    return out


def row_differences(before: dict, after: dict, ignore: set[str]) -> list[str]:
    """One entry per id whose row differs: the id, then the keys that differ (or
    the arm the row is missing from). Values are never listed."""
    out = []
    for key in sorted(set(before) | set(after)):
        if key not in after:
            out.append(f"{key} (only in before)")
        elif key not in before:
            out.append(f"{key} (only in after)")
        else:
            b, a = comparable(before[key], ignore), comparable(after[key], ignore)
            keys = sorted(k for k in set(b) | set(a) if b.get(k, _MISSING) != a.get(k, _MISSING))
            if keys:
                out.append(f"{key} ({', '.join(keys)})")
    return out


def ignored_keys(before: dict, after: dict, extra: list[str]) -> tuple[set[str], list[str]]:
    """The keys to leave out of a comparison of two row sets, and which of them
    were left out because only one arm carries them."""
    one_sided = [k for k in ONE_SIDED_IGNORED
                 if any(k in r for r in before.values()) != any(k in r for r in after.values())]
    return set(ALWAYS_IGNORED) | set(extra) | set(one_sided), one_sided


def is_budget_stop(row: dict) -> bool:
    return row.get("retry_stop") == "budget" or row.get("retry_budget_hit") is True


def trace_breaks(doc: dict) -> list[str]:
    """What is wrong with one document's pass trace. The messages hold counts
    and fixed words, never OCR text."""
    passes, text = doc.get("ocr_passes"), doc.get("ocr_text")
    if passes is None:
        return [] if text is None else ["ocr_passes missing"]
    if not isinstance(passes, list):
        return ["ocr_passes is not a list"]
    if text is None:
        return [] if not passes else ["ocr_text null but passes present"]
    if not isinstance(text, str):
        return ["ocr_text is not a string"]
    breaks: list[str] = []

    def add(message: str) -> None:
        if message not in breaks:
            breaks.append(message)

    records = [p for p in passes if isinstance(p, dict)]
    if len(records) != len(passes):
        add("a pass is not an object")
    for n, rec in enumerate(records):
        if rec.get("order") != n:
            add("orders are not contiguous")
        if rec.get("id") != ("general" if n == 0 else f"pass-{n - 1:02d}"):
            add("ids are not contiguous")
        if (rec.get("transform") == "general") != (n == 0):
            add("general appears other than at 0")
        readings = rec.get("readings")
        if not isinstance(readings, list):
            add("readings missing")
        elif (rec.get("outcome") in EMPTY_PASS_OUTCOMES) != (len(readings) == 0):
            add("readings disagree with the outcome")
    accepted = [rec for rec in records if rec.get("outcome") == "accepted"]
    stop = doc.get("retry_stop")
    if stop in ACCEPTING_STOPS:
        if len(accepted) != 1:
            add(f"{len(accepted)} accepted passes with stop {stop}")
        elif accepted[0].get("id") != doc.get("retry_variant_id") or accepted[0].get("order") != len(records) - 1:
            add("the accepted pass is not the last, or is not retry_variant_id")
    elif accepted:
        add(f"{len(accepted)} accepted passes with stop {stop}")
    tail = "".join(
        "\n" + "\n".join(r["text"] for r in rec["readings"] if isinstance(r, dict) and isinstance(r.get("text"), str))
        for rec in records[1:]
        if rec.get("outcome") in ("appended", "accepted") and isinstance(rec.get("readings"), list))
    if not text.endswith(tail):
        add("the appended and accepted readings are not the tail of ocr_text")
    return breaks


def trace_failures(rows: dict, arm: str) -> list[str]:
    """`<id> (<arm>: <first two breaks>)` for each row of `rows` with a break."""
    out = []
    for key, doc in rows.items():
        breaks = trace_breaks(doc)
        if breaks:
            out.append(f"{key} ({arm}: {'; '.join(breaks[:2])})")
    return out


def budget_entries(prefix: str, stops: tuple[set, set], text_status) -> list[dict]:
    """Every budget stop, by id, and the arms it happened in. `text_status(id)`
    says whether the two arms' dumped texts agree, by hash."""
    return [{"id": f"{prefix}{key}", "in": [key in stops[0], key in stops[1]],
             "text": text_status(key)} for key in sorted(stops[0] | stops[1])]


def neutral_synthetic(name: str, before: Path, after: Path, extra: list[str], check_trace: bool) -> dict:
    report_b, report_a = load_json(before), load_json(after)
    rows_b = {d["seed"]: d for d in report_b.get("results", [])}
    rows_a = {d["seed"]: d for d in report_a.get("results", [])}
    ignore, one_sided = ignored_keys(rows_b, rows_a, extra)
    out: dict = {
        "seeds": [len(rows_b), len(rows_a)],
        "hits": [sum(1 for d in rows_b.values() if d.get("hit")), sum(1 for d in rows_a.values() if d.get("hit"))],
        "also_ignored": one_sided,
        "differs_in": [k for k in ("ocr_arms", "model_paths")
                       if k in report_b and k in report_a and report_b[k] != report_a[k]],
        "results_differ": failing(row_differences(rows_b, rows_a, ignore)),
        "dump_blocks_differ": None,
        "text_vs_dump_differ": None,
        "text_checked": 0,
        "trace_breaks": None,
        "passes": 0,
    }
    stdout_b, stdout_a = before.with_suffix(".stdout"), after.with_suffix(".stdout")
    blocks_b = dump_blocks(stdout_b) if stdout_b.is_file() else None
    blocks_a = dump_blocks(stdout_a) if stdout_a.is_file() else None
    hashed = [None if b is None else {s: {"block": digest("\n".join(v))} for s, v in b.items()}
              for b in (blocks_b, blocks_a)]
    if hashed[0] is not None and hashed[1] is not None:
        out["dump_blocks_differ"] = failing(row_differences(hashed[0], hashed[1], set()))
    # An arm with ocr_text must read what the other arm printed.
    mismatched, checked = [], 0
    for side, rows, other in (("before", rows_b, blocks_a), ("after", rows_a, blocks_b)):
        if other is None:
            continue
        for seed in sorted(rows):
            text = rows[seed].get("ocr_text")
            if text is None:
                continue
            checked += 1
            if seed not in other or block_text(other[seed]) != text:
                mismatched.append(f"{seed} ({side})")
    if checked:
        out["text_vs_dump_differ"], out["text_checked"] = failing(mismatched), checked
    if check_trace:
        broken, traced = [], False
        for side, rows in (("before", rows_b), ("after", rows_a)):
            if any("ocr_passes" in r for r in rows.values()):
                traced = True
                broken += trace_failures(rows, side)
                out["passes"] += sum(len(r.get("ocr_passes") or []) for r in rows.values())
        if traced:
            out["trace_breaks"] = failing(broken)

    def text_status(seed):
        if hashed[0] is None or hashed[1] is None or seed not in hashed[0] or seed not in hashed[1]:
            return "not dumped"
        return "same" if hashed[0][seed] == hashed[1][seed] else "differs"

    out["budget"] = budget_entries(f"{name}:", ({s for s, r in rows_b.items() if is_budget_stop(r)},
                                                {s for s, r in rows_a.items() if is_budget_stop(r)}),
                                   text_status)
    return out


def neutral_real(before: Path, after: Path, extra: list[str], check_trace: bool) -> dict:
    ledgers = [load_jsonl(d / OUTCOMES) for d in (before, after)]
    ignore, _ = ignored_keys(*ledgers, extra)
    out: dict = {
        "documents": [len(ledgers[0]), len(ledgers[1])],
        "ledger_differ": failing(row_differences(ledgers[0], ledgers[1], ignore)),
        "dump_rows": None,
        "dump_differ": None,
        "trace_rows": None,
        "trace_differ": None,
        "trace": None,
        "trace_documents_without_row": None,
        "trace_breaks": None,
        "trace_text_differs_from_dump": None,
        "trace_retry_differs_from_ledger": None,
        "passes": 0,
    }
    dumps = [load_jsonl(d / ZONES, PROVIDER) if (d / ZONES).is_file() else None for d in (before, after)]
    if dumps[0] is not None and dumps[1] is not None:
        dump_ignore, _ = ignored_keys(*dumps, extra)
        out["dump_rows"] = [len(dumps[0]), len(dumps[1])]
        out["dump_differ"] = failing(row_differences(dumps[0], dumps[1], dump_ignore))
    traces = [load_jsonl(d / PASSES) if (d / PASSES).is_file() else None for d in (before, after)]
    if traces[0] is not None and traces[1] is not None:
        trace_ignore, _ = ignored_keys(*traces, extra)
        out["trace_rows"] = [len(traces[0]), len(traces[1])]
        out["trace_differ"] = failing(row_differences(traces[0], traces[1], trace_ignore))
    if check_trace:
        untraced, broken, text_bad, retry_bad = 0, [], [], []
        for index, (side, ledger, dump) in enumerate((("before", ledgers[0], dumps[0]),
                                                      ("after", ledgers[1], dumps[1]))):
            rows = traces[index]
            if rows is None:
                continue
            out["trace"] = out["trace"] or {"documents": [0, 0]}
            out["trace"]["documents"][index] = len(rows)
            out["passes"] += sum(len(r.get("ocr_passes") or []) for r in rows.values())
            untraced += len(set(ledger) - set(rows))
            broken += trace_failures(rows, side)
            text_bad += [f"{k} ({side})" for k in sorted(rows) if dump is not None and k in dump
                         and text_digest(rows[k].get("ocr_text")) != text_digest(dump[k].get("raw_ocr_text"))]
            retry_bad += [f"{k} ({side})" for k in sorted(rows) if k in ledger
                          and any(rows[k].get(f) != ledger[k].get(f) for f in RETRY_FIELDS)]
        if out["trace"] is not None:
            out["trace_documents_without_row"] = untraced
            out["trace_breaks"] = failing(broken)
            out["trace_text_differs_from_dump"] = failing(text_bad)
            out["trace_retry_differs_from_ledger"] = failing(retry_bad)

    def text_status(asset):
        if dumps[0] is None or dumps[1] is None or asset not in dumps[0] or asset not in dumps[1]:
            return "not dumped"
        same = text_digest(dumps[0][asset].get("raw_ocr_text")) == text_digest(dumps[1][asset].get("raw_ocr_text"))
        return "same" if same else "differs"

    out["budget"] = budget_entries("", ({k for k, r in ledgers[0].items() if is_budget_stop(r)},
                                        {k for k, r in ledgers[1].items() if is_budget_stop(r)}), text_status)
    return out


CHECK_KEYS = ("results_differ", "dump_blocks_differ", "text_vs_dump_differ", "trace_breaks",
              "ledger_differ", "dump_differ", "trace_differ", "trace_text_differs_from_dump", "trace_retry_differs_from_ledger")


def failed_checks(section: dict) -> int:
    return sum((section.get(k) or {}).get("failed", 0) for k in CHECK_KEYS)


def compare_neutral(before: Path, after: Path, base: dict, extra: list[str], check_trace: bool) -> dict:
    """The neutrality verdict for two arms, given `compare_arms`'s result for
    them. `exit` follows the default mode's codes, and adds 3, a pair that is an
    A/B but not identical."""
    problems = problems_of(base)
    synthetic_b, synthetic_a = synthetic_reports(before), synthetic_reports(after)
    synthetic = {name: neutral_synthetic(name, synthetic_b[name], synthetic_a[name], extra, check_trace)
                 for name in base["synthetic"]}
    real = neutral_real(before / "real", after / "real", extra, check_trace) if base["real"] else None
    if check_trace and not any(s["trace_breaks"] is not None for s in synthetic.values()) \
            and not (real and real["trace"]):
        raise Refused("--check-pass-trace was asked, and neither arm has a pass trace to check")
    if base["real"]:
        real["differs_in"] = [k for k in base["real"]["identity"][0] if k != "git_commit"
                              and base["real"]["identity"][0][k] != base["real"]["identity"][1][k]]
    unpaired = [f"{name} only in the {side} arm" for side, names in base["synthetic_in_one_arm_only"].items()
                for name in names]
    if base["real_in_one_arm_only"]:
        unpaired.append(f"real run only in the {base['real_in_one_arm_only']} arm")
    budget = [e for s in synthetic.values() for e in s["budget"]] + (real["budget"] if real else [])
    one_sided = [e["id"] for e in budget if e["in"][0] != e["in"][1]]
    if one_sided:
        problems.append(f"a budget stop in one arm only: {', '.join(one_sided[:MAX_IDS])}"
                        + (f", and {len(one_sided) - MAX_IDS} more" if len(one_sided) > MAX_IDS else ""))
    differing = sum(failed_checks(s) for s in synthetic.values()) + (failed_checks(real) if real else 0)
    code = 1 if problems else (3 if differing or unpaired else 0)
    return {
        "ignored": sorted(set(ALWAYS_IGNORED) | set(extra)),
        "ignored_when_one_arm_only": list(ONE_SIDED_IGNORED),
        "problems": problems, "unpaired": unpaired,
        "synthetic": synthetic, "real": real, "budget": budget,
        "exit": code, "neutral": code == 0,
    }


def show_check(check: dict | None) -> str:
    return "n/a" if check is None else str(check["failed"])


def check_detail(label: str, check: dict | None) -> list[str]:
    if not check or not check["failed"]:
        return []
    more = check["failed"] - len(check["ids"])
    return [f"    {label}: {', '.join(check['ids'])}" + (f", and {more} more" if more else "")]


def render_neutral(result: dict) -> str:
    lines = [f"expect-identical: ignoring {', '.join(result['ignored'])} (at any depth); "
             f"{' and '.join(result['ignored_when_one_arm_only'])} are ignored where only one arm has them"]
    lines += [f"NOT AN A/B: {p}" for p in result["problems"]]
    lines += [f"NOT COMPARED: {u}" for u in result["unpaired"]]
    for name, s in result["synthetic"].items():
        mark = "OK" if not failed_checks(s) else "DIFFERS"
        lines.append(
            f"{name}: seeds {s['seeds'][0]}/{s['seeds'][1]}, hits {s['hits'][0]}/{s['hits'][1]} | "
            f"results[] differ {show_check(s['results_differ'])} | dump blocks differ {show_check(s['dump_blocks_differ'])} | "
            f"ocr_text vs the other arm's dump differ {show_check(s['text_vs_dump_differ'])} (checked {s['text_checked']}) | "
            f"trace breaks {show_check(s['trace_breaks'])} (passes {s['passes']}) -> {mark}"
            + (f" | also ignored here: {', '.join(s['also_ignored'])}" if s["also_ignored"] else ""))
        if s["differs_in"]:
            lines.append(f"  note: the arms differ in {', '.join(s['differs_in'])}")
        for label, key in (("results[] differ", "results_differ"), ("dump blocks differ", "dump_blocks_differ"),
                           ("ocr_text differs from the other arm's dump", "text_vs_dump_differ"),
                           ("trace breaks", "trace_breaks")):
            lines += check_detail(label, s[key])
    r = result["real"]
    if r:
        mark = "OK" if not failed_checks(r) else "DIFFERS"
        dump = "not compared (an arm has no dump)" if r["dump_rows"] is None else \
            f"{r['dump_rows'][0]}/{r['dump_rows'][1]} rows, differ {show_check(r['dump_differ'])} (raw_ocr_text by sha256)"
        rows = "not compared (an arm has no trace file)" if r["trace_rows"] is None else \
            f"{r['trace_rows'][0]}/{r['trace_rows'][1]} rows, differ {show_check(r['trace_differ'])}"
        trace = "invariants not checked" if r["trace"] is None else (
            f"{r['passes']} passes, documents without a row {r['trace_documents_without_row']}, "
            f"ocr_text != dump {show_check(r['trace_text_differs_from_dump'])}, "
            f"retry fields != ledger {show_check(r['trace_retry_differs_from_ledger'])}, "
            f"breaks {show_check(r['trace_breaks'])}")
        lines.append(f"real: documents {r['documents'][0]}/{r['documents'][1]} | ledger differ "
                     f"{show_check(r['ledger_differ'])} | dump {dump} | trace rows {rows} | trace {trace} -> {mark}")
        if r["differs_in"]:
            lines.append(f"  note: the runs differ in {', '.join(r['differs_in'])}")
        for label, key in (("ledger differ", "ledger_differ"), ("dump rows differ", "dump_differ"),
                           ("trace rows differ", "trace_differ"),
                           ("trace ocr_text differs from the dump", "trace_text_differs_from_dump"),
                           ("trace retry fields differ from the ledger", "trace_retry_differs_from_ledger"),
                           ("trace breaks", "trace_breaks")):
            lines += check_detail(label, r[key])
    if result["budget"]:
        both = [f"{e['id']} (text {e['text']})" for e in result["budget"] if all(e["in"])]
        single = [f"{e['id']} ({'before' if e['in'][0] else 'after'} only)"
                  for e in result["budget"] if not all(e["in"])]
        lines.append(f"budget stops: {len(both)} in both arms" + (f": {', '.join(both[:MAX_IDS])}" if both else "")
                     + f"; {len(single)} in one arm only" + (f": {', '.join(single[:MAX_IDS])}" if single else ""))
    else:
        lines.append("budget stops: none")
    lines.append("NEUTRAL" if result["neutral"] else "NOT NEUTRAL")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[1])
    ap.add_argument("before", type=Path, help="the before arm's directory")
    ap.add_argument("after", type=Path, help="the after arm's directory")
    ap.add_argument("--asset", action="append", default=[],
                    help="a real specimen's asset_id to report whether or not it changed (repeatable)")
    ap.add_argument("--json", action="store_true", help="print the result as JSON")
    ap.add_argument("--expect-identical", action="store_true",
                    help="neutrality mode: exit 0 if every document reads the same in both arms, 3 if not; "
                         "prints counts and ids, never OCR text")
    ap.add_argument("--check-pass-trace", action="store_true",
                    help="with --expect-identical: check the invariants of `ocr_passes` and the real pass trace")
    ap.add_argument("--ignore", action="append", default=[], metavar="KEY",
                    help="with --expect-identical: a row key to leave out of the comparison (repeatable)")
    args = ap.parse_args(argv)
    if not args.expect_identical:
        if args.check_pass_trace or args.ignore:
            ap.error("--check-pass-trace and --ignore need --expect-identical")
    elif args.asset:
        ap.error("--asset does not apply to --expect-identical, which prints no zones")
    for arm in (args.before, args.after):
        if not arm.is_dir():
            print(f"bench_ab_diff: not a directory: {arm}", file=sys.stderr)
            return 2
    try:
        result = compare_arms(args.before, args.after, args.asset)
        if args.expect_identical and (result["synthetic"] or result["real"]):
            verdict = compare_neutral(args.before, args.after, result, args.ignore, args.check_pass_trace)
    except Refused as e:
        print(f"bench_ab_diff: refused: {e}", file=sys.stderr)
        return 2
    except (OSError, ValueError, KeyError) as e:
        print(f"bench_ab_diff: cannot read the arms: {e}", file=sys.stderr)
        return 2
    if not result["synthetic"] and not result["real"]:
        print("bench_ab_diff: the two arms have no synthetic report or real run in common", file=sys.stderr)
        return 2
    if args.expect_identical:
        print(json.dumps(verdict, indent=2) if args.json else render_neutral(verdict))
        return verdict["exit"]
    print(json.dumps(result, indent=2) if args.json else render(result))
    return 1 if problems_of(result) else 0


if __name__ == "__main__":
    sys.exit(main())
