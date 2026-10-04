#!/usr/bin/env python3
"""Nightly error shares per format and profile, each with a 95% Wilson interval.

Stdlib only. Reads a checkout of `bench-data` (`fresh.jsonl`, `fixed.jsonl`, `runs.jsonl`, schema 2)
through `bench_nightly_advisory.load_history` and `bench_nightly_rows`, and prints, per format and per
profile, the Tier-1 hit rate, the share of hits that are wrong on at least one of the 12 scored
fields, and the share of hits wrong on each of those fields. It reads no OCR text and prints no
per-document record: counts, rates, intervals, run ids, short commit ids and fingerprints only.

    python tools/bench_nightly_intervals.py DATA_DIR            Markdown tables on stdout
    python tools/bench_nightly_intervals.py DATA_DIR --json     the same figures as JSON

Pools. A night joins a pool only when the generator fingerprint (the fixed header's hash of the
rendered pixels, which a fresh row inherits from the fixed header of the same run and format), the
OCR model hashes, the OCR and MRZ arms and `ocr_env` knobs, and `max_passes` and `max_seconds` all
match. Nights that differ in any of them are never added together: a re-render or a new model is a
different measured population. A pool is per format, because the fingerprint is. Each pool lists
its nights and the first and last `git_sha` it spans.

Slices.
  * Fresh (`--profile all`, new seeds every night) is pooled across a pool's nights: every night
    adds new documents, so every night adds n.
  * Fixed (`clean`, seeds 0-99) is never pooled across nights. The same 100 seeds read the same
    way, so a second night adds no n. One night's figures are reported (the latest in the pool),
    with whether every night in the pool read every seed identically (the same hit, miss kind and
    wrong fields).

Budget stops. A row whose `retry_stop` is `budget` was cut off by the wall clock, so what it read
depends on runner speed. It is left out of every rate and counted on its own line.

Intervals. 95% Wilson (`z` = 1.96), as `synthpass_bench::stats::wilson_interval` prints beside the
rates of the synthetic summary. The interval says which generator-wide rates the documents drawn are
consistent with. It is not a prediction interval. To compare two pools use the advisory's
two-proportion z-test (`bench_nightly_advisory.two_proportion_z`), not two overlapping intervals.

Exit status: 0 for a report, 2 for a usage, I/O or schema error.
"""

from __future__ import annotations

import argparse
import datetime
import json
import math
import sys
from pathlib import Path

import bench_nightly_advisory as advisory
import bench_nightly_rows as rows_lib

# `z` for a two-sided 95% interval; the same constant as `synthpass_bench::stats::Z_95`.
Z_95 = 1.96

REPORT_SCHEMA = 1

# The header facts, besides the fingerprint, that must match for two nights to share a pool.
CONDITION_KEYS = (
    "model_sha256",
    "ocr_arms",
    "mrz_class_sweep_arm",
    "mrz_line1_select_arm",
    "ocr_env",
    "max_passes",
    "max_seconds",
)


def wilson_interval(k: int, n: int) -> tuple[float, float] | None:
    """The 95% Wilson score interval for `k` successes in `n` trials, as `(low, high)` fractions.

    `None` when `n` is 0 (no trials, no interval) and when `k` exceeds `n`. The ends are exact: the
    low end is 0.0 at `k` = 0 and the high end 1.0 at `k` = `n`, not a rounding of them. The same
    function as `synthpass_bench::stats::wilson_interval`.
    """
    if n <= 0 or k < 0 or k > n:
        return None
    p = k / n
    z2 = Z_95 * Z_95
    denominator = 1.0 + z2 / n
    centre = (p + z2 / (2.0 * n)) / denominator
    half = Z_95 * math.sqrt(p * (1.0 - p) / n + z2 / (4.0 * n * n)) / denominator
    low = 0.0 if k == 0 else max(centre - half, 0.0)
    high = 1.0 if k == n else min(centre + half, 1.0)
    return low, high


# --- figures ----------------------------------------------------------------------------------


def figure(k: int, n: int) -> dict:
    """One count over its denominator: `k`, `n`, the rate, and the interval (both `None` for n = 0)."""
    interval = wilson_interval(k, n)
    return {
        "k": k,
        "n": n,
        "rate": k / n if n else None,
        "ci95": list(interval) if interval else None,
    }


def split_budget(rows: list[dict]) -> tuple[list[dict], int]:
    """The rows the retry budget did not cut off, and how many it did."""
    kept = [row for row in rows if row["retry_stop"] != "budget"]
    return kept, len(rows) - len(kept)


def shares(rows: list[dict]) -> dict:
    """Hit rate over `rows`, and the wrong shares over the hits among them.

    A hit whose row carries no field outcomes cannot be called right or wrong on a field, so it is
    left out of the wrong shares' denominator and counted in `hits_without_field_outcomes`.
    """
    hits = [row for row in rows if row["hit"]]
    wrong = [rows_lib.derive(row)["wrong_fields"] for row in hits]
    measured = [fields for fields in wrong if fields is not None]
    return {
        "documents": len(rows),
        "hit_rate": figure(len(hits), len(rows)),
        "wrong_on_any": figure(sum(1 for fields in measured if fields), len(measured)),
        "wrong_by_field": {
            name: figure(sum(1 for fields in measured if name in fields), len(measured))
            for name in rows_lib.SCORED_FIELDS
        },
        "hits_without_field_outcomes": len(wrong) - len(measured),
    }


def by_profile(rows: list[dict]) -> dict:
    """`shares` for each profile that occurs in `rows`, in the generator's profile order."""
    return {
        profile: shares([row for row in rows if row["profile"] == profile])
        for profile in rows_lib.PROFILES
        if any(row["profile"] == profile for row in rows)
    }


def _outcome(row: dict) -> tuple:
    wrong = rows_lib.derive(row)["wrong_fields"]
    return (row["hit"], row["miss_kind"], None if wrong is None else tuple(sorted(wrong)))


def fresh_figures(nights: list[dict]) -> dict:
    """The fresh slice, pooled across the pool's nights (each night's documents are new)."""
    kept, stopped = split_budget([row for night in nights for row in night["fresh"]])
    return {
        "nights": len(nights),
        "budget_stops": stopped,
        "all": shares(kept),
        "by_profile": by_profile(kept),
    }


def fixed_figures(nights: list[dict]) -> dict:
    """The fixed slice: the latest night's figures, never a sum over nights.

    The same seeds render the same documents every night, so a second night adds no n. What a pool
    of nights does say is whether the reads moved: `identical` is true when every night read every
    seed with the same hit, miss kind and wrong fields as the latest night (`None` for a pool of
    one night, which has nothing to compare).
    """
    latest = nights[-1]
    kept, stopped = split_budget(list(latest["fixed"].values()))
    reference = {seed: _outcome(row) for seed, row in latest["fixed"].items()}
    differing = []
    for night in nights[:-1]:
        seen = {seed: _outcome(row) for seed, row in night["fixed"].items()}
        differing.append(sum(1 for seed in reference.keys() | seen.keys() if seen.get(seed) != reference.get(seed)))
    return {
        "nights": len(nights),
        "figures_from_run": latest["run_id"],
        "budget_stops_latest_night": stopped,
        "budget_stops_all_nights": sum(split_budget(list(night["fixed"].values()))[1] for night in nights),
        "identical_across_nights": (not any(differing)) if differing else None,
        "seeds_differing_most": max(differing, default=0),
        "all": shares(kept),
        "by_profile": by_profile(kept),
    }


# --- pools ------------------------------------------------------------------------------------


def _conditions(header: dict) -> str:
    return json.dumps({key: header.get(key) for key in CONDITION_KEYS}, sort_keys=True)


def build_pools(history: advisory.History) -> tuple[list[dict], dict]:
    """Every pool of nights, per format, oldest first, and the runs and rows left outside any.

    The nights come from `advisory._run_entries`, which drops a run whose fixed slice is not exactly
    seeds 0-99 once each or whose header fingerprint disagrees with the rows. A run with no fresh
    header has nothing to say about its fresh slice's conditions, so it is left out too.
    """
    fixed_index = advisory._by_key(history.fixed)
    fresh_index = advisory._by_key(history.fresh)
    headers: dict = {}
    for header in history.headers:
        headers.setdefault((header["run_id"], header["document_type"], header["slice"]), header)

    pools: dict = {}
    skipped = {"fixed_slice_unusable": 0, "no_fresh_header": 0}
    for fmt in rows_lib.FORMATS:
        for entry in advisory._run_entries(history, fmt, fixed_index, fresh_index):
            fixed_header = headers[(entry["run_id"], fmt, "fixed")]
            fresh_header = headers.get((entry["run_id"], fmt, "fresh"))
            if entry["fingerprint"] is None:
                skipped["fixed_slice_unusable"] += 1
                continue
            if fresh_header is None:
                skipped["no_fresh_header"] += 1
                continue
            key = (fmt, entry["fingerprint"], _conditions(fixed_header), _conditions(fresh_header))
            pool = pools.setdefault(
                key,
                {
                    "document_type": fmt,
                    "fingerprint": entry["fingerprint"],
                    "conditions": {k: fixed_header.get(k) for k in CONDITION_KEYS},
                    "nights": [],
                },
            )
            pool["nights"].append(entry)

    pooled_fresh = sum(len(night["fresh"]) for pool in pools.values() for night in pool["nights"])
    pooled_fixed = sum(len(night["fixed"]) for pool in pools.values() for night in pool["nights"])
    skipped["fresh_rows_outside_a_pool"] = len(history.fresh) - pooled_fresh
    skipped["fixed_rows_outside_a_pool"] = len(history.fixed) - pooled_fixed
    ordered = sorted(pools.values(), key=lambda pool: (pool["nights"][0]["timestamp"], rows_lib.FORMATS.index(pool["document_type"])))
    return ordered, skipped


def _utc_date(stamp: int) -> str:
    return datetime.datetime.fromtimestamp(stamp, datetime.timezone.utc).strftime("%Y-%m-%d")


def build_report(history: advisory.History) -> dict:
    """The whole report: the pools with their figures, grouped for printing by the nights they span."""
    pools, skipped = build_pools(history)
    group_ids: dict = {}
    entries = []
    for pool in pools:
        nights = pool["nights"]
        span = tuple(night["run_id"] for night in nights)
        group = group_ids.setdefault(span, chr(ord("A") + len(group_ids)))
        entries.append(
            {
                "group": group,
                "document_type": pool["document_type"],
                "generator_fingerprint": pool["fingerprint"],
                "conditions": pool["conditions"],
                "nights": [
                    {"run_id": n["run_id"], "utc_date": _utc_date(n["timestamp"]), "git_sha": n["git_sha"]}
                    for n in nights
                ],
                "git_sha_first": nights[0]["git_sha"],
                "git_sha_last": nights[-1]["git_sha"],
                "distinct_git_shas": len({n["git_sha"] for n in nights}),
                "fresh": fresh_figures(nights),
                "fixed": fixed_figures(nights),
            }
        )
    return {
        "schema": REPORT_SCHEMA,
        "interval": {"kind": "wilson", "confidence": 0.95, "z": Z_95},
        "source": {
            "fresh_rows": len(history.fresh),
            "fixed_rows": len(history.fixed),
            "run_headers": len(history.headers),
        },
        "left_out": skipped,
        "pools": entries,
    }


# --- Markdown ---------------------------------------------------------------------------------


def _cell(item: dict) -> str:
    if not item["n"]:
        return "n/a (0)"
    low, high = item["ci95"]
    return f"{item['k']}/{item['n']} {item['rate'] * 100:.1f}% ({low * 100:.1f}–{high * 100:.1f})"


def _table(header: list[str], rows: list[list[str]]) -> list[str]:
    lines = ["| " + " | ".join(header) + " |", "|" + "|".join(" --- " for _ in header) + "|"]
    lines.extend("| " + " | ".join(row) + " |" for row in rows)
    return lines


def _slice_rows(pools: list[dict], slice_name: str) -> list[tuple[str, str, dict]]:
    """`(format, profile label, shares)` rows: each format's total, then its profiles when it has
    more than one (the fixed slice is `clean` only, so its profile row would repeat the total)."""
    out = []
    for pool in pools:
        data = pool[slice_name]
        out.append((pool["document_type"], "all", data["all"]))
        if len(data["by_profile"]) > 1:
            out.extend((pool["document_type"], profile, s) for profile, s in data["by_profile"].items())
    return out


def _slice_tables(pools: list[dict], slice_name: str) -> list[str]:
    rows = _slice_rows(pools, slice_name)
    lines = _table(
        ["Format", "Profile", "Documents", "Hit rate", "Hits wrong on at least one field"],
        [[fmt, profile, str(s["documents"]), _cell(s["hit_rate"]), _cell(s["wrong_on_any"])] for fmt, profile, s in rows],
    )
    shown = [name for name in rows_lib.SCORED_FIELDS if any(s["wrong_by_field"][name]["k"] for _, _, s in rows)]
    lines += ["", f"Share of hits wrong on each scored field ({slice_name}):", ""]
    if shown:
        lines += _table(
            ["Format", "Profile", *shown],
            [[fmt, profile, *(_cell(s["wrong_by_field"][name]) for name in shown)] for fmt, profile, s in rows],
        )
    quiet = [name for name in rows_lib.SCORED_FIELDS if name not in shown]
    if quiet:
        lines += ["", "No hit was wrong on: " + ", ".join(quiet) + "."]
    return lines


def render_markdown(report: dict) -> str:
    out = ["# Nightly error shares per format and profile", ""]
    source = report["source"]
    out.append(
        f"{source['fresh_rows']} fresh rows, {source['fixed_rows']} fixed rows and "
        f"{source['run_headers']} run headers. Each figure is `k/n`, the rate, and its 95% Wilson "
        "interval in parentheses; an interval says which generator-wide rates the documents drawn "
        "are consistent with, not what the next night will read. Pools are per format and never "
        "span a change of generator fingerprint, OCR model, arm, retry budget or time limit."
    )
    left = report["left_out"]
    if any(left.values()):
        out += ["", "Left out of every pool: " + ", ".join(f"{k.replace('_', ' ')} {v}" for k, v in left.items() if v) + "."]

    out += ["", "## Pools", ""]
    out += _table(
        ["Pool", "Format", "Nights", "UTC dates", "git_sha first..last (distinct)", "Fingerprint"],
        [
            [
                p["group"],
                p["document_type"],
                str(len(p["nights"])),
                f"{p['nights'][0]['utc_date']} .. {p['nights'][-1]['utc_date']}",
                f"{p['git_sha_first'][:8]}..{p['git_sha_last'][:8]} ({p['distinct_git_shas']})",
                p["generator_fingerprint"][:12],
            ]
            for p in report["pools"]
        ],
    )

    groups: dict = {}
    for pool in report["pools"]:
        groups.setdefault(pool["group"], []).append(pool)
    for group, pools in groups.items():
        first = pools[0]
        out += [
            "",
            f"## Pool {group}: {len(first['nights'])} night(s), "
            f"{first['nights'][0]['utc_date']} .. {first['nights'][-1]['utc_date']}",
            "",
            "### Fresh slice, pooled across the nights",
            "",
            "Budget stops left out of every rate: "
            + ", ".join(f"{p['document_type']} {p['fresh']['budget_stops']}" for p in pools)
            + ".",
            "",
        ]
        out += _slice_tables(pools, "fresh")
        out += ["", "### Fixed slice, the latest night (never summed across nights)", ""]
        out += _table(
            ["Format", "Nights", "Every seed read identically", "Seeds differing, most", "Budget stops, latest night / all nights"],
            [
                [
                    p["document_type"],
                    str(p["fixed"]["nights"]),
                    {True: "yes", False: "no", None: "n/a, one night"}[p["fixed"]["identical_across_nights"]],
                    str(p["fixed"]["seeds_differing_most"]),
                    f"{p['fixed']['budget_stops_latest_night']} / {p['fixed']['budget_stops_all_nights']}",
                ]
                for p in pools
            ],
        )
        out += [""]
        out += _slice_tables(pools, "fixed")
    return "\n".join(out) + "\n"


# --- command line -----------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("data_dir", type=Path, help="a checkout of bench-data: fresh.jsonl, fixed.jsonl, runs.jsonl")
    parser.add_argument("--json", action="store_true", help="print the figures as JSON instead of Markdown")
    args = parser.parse_args(argv)
    # The tables carry an en dash; a Windows console would otherwise write it as cp1252.
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    try:
        if not args.data_dir.is_dir():
            raise rows_lib.RowError(f"{args.data_dir} is not a directory")
        report = build_report(advisory.load_history(args.data_dir))
    except (OSError, rows_lib.RowError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    if args.json:
        print(json.dumps(report, indent=2))
    else:
        sys.stdout.write(render_markdown(report))
    return 0


if __name__ == "__main__":
    sys.exit(main())
