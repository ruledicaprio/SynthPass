#!/usr/bin/env python3
"""The nightly advisory: read the rows on `bench-data` and say what tonight's run changed.

Stdlib only. ADR-0027 decision 7. It reads `fresh.jsonl`, `fixed.jsonl` and `runs.jsonl` (schema 2)
through `bench_nightly_rows`, which refuses any row with a key outside its allowlist. It ignores
`dataset.jsonl`, the frozen schema-1 history. It judges tonight's run twice, per format:

  * Fixed slice (clean, seeds 0-99, the same 100 documents every night). Paired per seed against
    the most recent earlier run whose fixed slice has the same `generator_fingerprint`, a hash of
    the rendered pixels: any re-render resets the reference. Every seed whose hit, correct-read,
    wrong-accept or prefix-wrong status changed is named. With no such run, "no reference".
  * Fresh slice (all profiles, new seeds every night). The last 7 nights pooled against the prior
    28 nights with the same fixed-slice fingerprint, by a two-proportion z-test: a flag when the
    hit rate or the correct-read rate has z <= -3, or the wrong-accept rate has z >= +3. With too
    little history the test is skipped, and that is not a flag.

It reports in three places: a Markdown job summary (only when a path is given, or
`$GITHUB_STEP_SUMMARY` is set), a `::warning::` or `::error::` line per finding on stdout, and one
line per run appended to `advisory.jsonl` on `bench-data`.

Exit status. Non-zero only when the night is red:

  * a severe fixed-slice flag: a fixed seed newly a wrong accept, or newly a prefix-wrong read,
    against its same-fingerprint reference (the M4 ratchet for prefix-wrong reads is 0); or
  * an invalid instrument: a `budget` retry stop in the fixed slice, a format with missing or
    duplicate rows or headers for the run, or a fingerprint change that no commit to the generator
    explains. When the tool cannot decide whether the generator changed (no checkout of `main`, or
    the reference commit is not reachable), that one is a warning, not a failure.

Everything else is a warning. It never opens an issue, and it is never a required check.

Privacy. The output is aggregates only: counts, rates, z values, finding codes, the run id, the
fingerprints and the seed numbers of flipped fixed seeds. It never carries a field value, OCR text or
a `reason`, because it never reads one: those keys are not in the rows it accepts. `advisory.jsonl`
lines are checked against a key allowlist and a short-token rule before they are written.
"""

from __future__ import annotations

import argparse
import datetime
import json
import math
import os
import re
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

import bench_nightly_rows as rows_lib

# --- thresholds and windows (provisional: a one-week A/A dispatch sets them) ---------------------

# A warning when a night loses more fixed-slice hits than it gains, by more than this many seeds.
NET_FLIP_WARN_ABOVE = 3

# The two-proportion z threshold for the fresh slice: -Z on the hit and correct-read rates, +Z on
# the wrong-accept rate.
Z_FLAG = 3.0
# The recent window holds exactly this many nights, tonight included.
RECENT_NIGHTS = 7
# The baseline is the nights before those, at most this many.
BASELINE_NIGHTS = 28
# Fewer baseline nights than this and the test is skipped.
MIN_BASELINE_NIGHTS = 7

# What a re-render can be explained by: the render's inputs. `crates/synthpass-gen/` holds the
# generator, its degrade profiles and the vendored fonts it embeds (`fonts/`); it assembles the zone
# with `crates/mrz/` (`mrz::check_digit`); and `Cargo.lock` pins the `image`, `rand`, `rand_chacha`
# and `ab_glyph` versions the pixels depend on. A fingerprint change with none of these changed is a
# generator that no longer renders deterministically.
GENERATOR_PATHS = ("crates/synthpass-gen/", "crates/mrz/", "Cargo.lock")

ADVISORY_FILE = "advisory.jsonl"
ADVISORY_SCHEMA = 1

# The fixed slice's seeds, from the rows library so the two tools never disagree.
FIXED_SEEDS = tuple(range(rows_lib.FIXED_SEED_START, rows_lib.FIXED_SEED_START + rows_lib.FIXED_COUNT))

# The per-seed status changes a flipped seed can carry, in the order they are named.
FLIP_TOKENS = (
    "hit_lost",
    "hit_gained",
    "correct_lost",
    "correct_gained",
    "wrong_accept_new",
    "wrong_accept_cleared",
    "prefix_wrong_new",
    "prefix_wrong_cleared",
)

# The keys an `advisory.jsonl` line may hold, at any depth (the five format names are keys too).
ADVISORY_KEYS = frozenset(
    {
        "schema",
        "run_id",
        "run_timestamp_unix",
        "git_sha",
        "red",
        "red_codes",
        "warning_codes",
        "thresholds",
        "net_flip_warn_above",
        "z_flag",
        "recent_nights",
        "baseline_nights",
        "min_baseline_nights",
        "formats",
        "generator_fingerprint",
        "reference_run_id",
        "reference_git_sha",
        "fixed",
        "fresh",
        "status",
        "count",
        "hits",
        "correct_reads",
        "wrong_accepts",
        "prefix_wrong_accepts",
        "budget_stops",
        "flipped_seeds",
        "new_wrong_accept_seeds",
        "new_prefix_wrong_seeds",
        "flip_counts",
        "net_hit_loss",
        "recent",
        "baseline",
        "nights",
        "z",
        "hit_rate",
        "correct_rate",
        "wrong_accept_rate",
        *FLIP_TOKENS,
    }
)

_RUN_ID = re.compile(r"^[0-9]{1,20}-[0-9]{1,4}$")
_HEX40 = re.compile(r"^[0-9a-f]{40}$")
_HEX64 = re.compile(r"^[0-9a-f]{64}$")
# The only shape a string in `advisory.jsonl` may have: a short token (a run id, a sha, a
# fingerprint, a status, a finding code). No spaces, so no sentence and no text.
_AGGREGATE_TOKEN = re.compile(r"^[A-Za-z0-9_.:-]{1,64}$")


class AdvisoryError(ValueError):
    """The advisory could not be built, or would have written more than aggregates."""


@dataclass(frozen=True)
class Finding:
    """One thing to report. `kind` is `red` or `warning`; `code` is a short token; `message` is the
    line shown in the summary and the workflow annotation (aggregates and seed numbers only)."""

    kind: str
    code: str
    message: str


@dataclass
class History:
    """Everything the advisory reads from a checkout of `bench-data`."""

    fixed: list[dict] = field(default_factory=list)
    fresh: list[dict] = field(default_factory=list)
    headers: list[dict] = field(default_factory=list)


# --- reading ----------------------------------------------------------------------------------


def read_headers(path: Path) -> list[dict]:
    """The schema-2 lines of `runs.jsonl`, each checked to carry what this tool reads."""
    headers = []
    for number, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            raw = json.loads(line)
        except json.JSONDecodeError as error:
            raise rows_lib.RowError(f"{path}:{number}: not JSON ({error.msg})") from error
        if not isinstance(raw, dict) or raw.get("schema") != rows_lib.SCHEMA:
            raise rows_lib.RowError(f"{path}:{number}: a header must be a schema-2 object")
        where = f"{path}:{number}"
        if not isinstance(raw.get("run_id"), str) or not _RUN_ID.match(raw["run_id"]):
            raise rows_lib.RowError(f"{where}: run_id must be <run id>-<attempt>")
        if raw.get("slice") not in rows_lib.SLICES:
            raise rows_lib.RowError(f"{where}: slice is not fresh or fixed")
        if raw.get("document_type") not in rows_lib.FORMATS:
            raise rows_lib.RowError(f"{where}: document_type is not one of the five formats")
        if not isinstance(raw.get("git_sha"), str) or not _HEX40.match(raw["git_sha"]):
            raise rows_lib.RowError(f"{where}: git_sha must be 40 lowercase hex characters")
        stamp = raw.get("run_timestamp_unix")
        if not rows_lib._is_int(stamp) or stamp < 0:
            raise rows_lib.RowError(f"{where}: run_timestamp_unix must be an integer >= 0")
        fingerprint = raw.get("generator_fingerprint")
        if fingerprint is not None and (not isinstance(fingerprint, str) or not _HEX64.match(fingerprint)):
            raise rows_lib.RowError(f"{where}: generator_fingerprint must be 64 hex characters or null")
        counts = raw.get("counts")
        if not isinstance(counts, dict) or not rows_lib._is_int(counts.get("count")):
            raise rows_lib.RowError(f"{where}: counts.count must be an integer")
        headers.append(raw)
    return headers


def load_history(data_dir: Path) -> History:
    """`fresh.jsonl`, `fixed.jsonl` and `runs.jsonl` from a checkout of `bench-data`.

    A missing file reads as empty. `dataset.jsonl`, the frozen schema-1 history, is never read.
    """
    data_dir = Path(data_dir)
    history = History()
    for name, sink in (("fixed.jsonl", history.fixed), ("fresh.jsonl", history.fresh)):
        path = data_dir / name
        if path.is_file():
            sink.extend(row for row in rows_lib.read_rows(path) if row["schema"] == rows_lib.SCHEMA)
    runs = data_dir / rows_lib.RUN_HEADER_FILE
    if runs.is_file():
        history.headers = read_headers(runs)
    return history


# --- statistics -------------------------------------------------------------------------------


def two_proportion_z(x1: int, n1: int, x2: int, n2: int) -> float | None:
    """The pooled two-proportion z for `x1/n1` against `x2/n2`, or `None` when it is undefined
    (an empty sample, or a pooled rate of exactly 0 or 1, where there is no variance to test)."""
    if n1 <= 0 or n2 <= 0:
        return None
    pooled = (x1 + x2) / (n1 + n2)
    variance = pooled * (1 - pooled) * (1 / n1 + 1 / n2)
    if variance <= 0:
        return None
    return (x1 / n1 - x2 / n2) / math.sqrt(variance)


# --- the generator check ----------------------------------------------------------------------


def generator_changed(repo: Path | None, old_sha: str, new_sha: str) -> bool | None:
    """Whether the generator's tree differs between two commits; `None` when that cannot be told.

    A tree comparison, not a commit walk: a change that was reverted in between renders the same
    pixels, so it explains nothing. `None` when there is no checkout of `main`, or either commit
    is not reachable in it (a shallow clone), so the caller warns instead of failing.
    """
    if repo is None or not _HEX40.match(old_sha) or not _HEX40.match(new_sha):
        return None
    git = ["git", "-C", str(repo)]
    try:
        for sha in (old_sha, new_sha):
            if subprocess.run([*git, "cat-file", "-e", f"{sha}^{{commit}}"], capture_output=True).returncode != 0:
                return None
        diff = subprocess.run(
            [*git, "diff", "--quiet", old_sha, new_sha, "--", *GENERATOR_PATHS], capture_output=True
        )
    except OSError:
        return None
    if diff.returncode == 0:
        return False
    if diff.returncode == 1:
        return True
    return None


# --- one format -------------------------------------------------------------------------------


def _outcome(row: dict) -> dict:
    """The four per-seed statuses the fixed slice is paired on."""
    flags = rows_lib.derive(row)
    wrong = flags["wrong_accept"] is True
    return {
        "hit": row["hit"],
        "correct": row["hit"] and not wrong,
        "wrong_accept": wrong,
        "prefix_wrong": flags["prefix_wrong_accept"] is True,
    }


def _fixed_problems(rows: list[dict]) -> list[str]:
    """`missing` and/or `duplicate` when a fixed slice is not exactly one row per seed 0-99."""
    seeds = [row["seed"] for row in rows]
    problems = []
    if set(FIXED_SEEDS) - set(seeds):
        problems.append("missing")
    if len(seeds) != len(set(seeds)) or set(seeds) - set(FIXED_SEEDS):
        problems.append("duplicate")
    return problems


def _by_key(rows: list[dict]) -> dict:
    grouped: dict = {}
    for row in rows:
        grouped.setdefault((row["run_id"], row["document_type"]), []).append(row)
    return grouped


def _run_entries(history: History, fmt: str, fixed_index: dict, fresh_index: dict) -> list[dict]:
    """Every run of one format that has a fixed header, oldest first, with what can be compared.

    `fingerprint` is recomputed from the rows and is `None` for a run whose fixed slice is not
    exactly seeds 0-99 once each, or whose header's fingerprint disagrees: such a run is not a
    usable reference.
    """
    entries = []
    for header in history.headers:
        if header["document_type"] != fmt or header["slice"] != "fixed":
            continue
        run_id = header["run_id"]
        if any(entry["run_id"] == run_id for entry in entries):
            continue  # a duplicate header; the caller reports it for tonight
        fixed_rows = fixed_index.get((run_id, fmt), [])
        usable = not _fixed_problems(fixed_rows)
        fingerprint = rows_lib.generator_fingerprint(fixed_rows) if usable else None
        if fingerprint is not None and header["generator_fingerprint"] != fingerprint:
            fingerprint = None
        entries.append(
            {
                "run_id": run_id,
                "timestamp": header["run_timestamp_unix"],
                "git_sha": header["git_sha"],
                "fingerprint": fingerprint,
                "fixed": {row["seed"]: row for row in fixed_rows} if usable else {},
                "fresh": fresh_index.get((run_id, fmt), []),
            }
        )
    return sorted(entries, key=lambda entry: (entry["timestamp"], entry["run_id"]))


def _check_tonight(fmt: str, history: History, run_id: str, fixed_rows: list[dict], fresh_rows: list[dict]) -> list[Finding]:
    """Invalid-instrument findings that need only tonight's rows and headers."""
    findings = []

    def red(code: str, message: str) -> None:
        findings.append(Finding("red", f"{fmt}:{code}", f"{fmt}: {message}"))

    for slice_name, rows in (("fixed", fixed_rows), ("fresh", fresh_rows)):
        headers = [
            h for h in history.headers
            if h["run_id"] == run_id and h["document_type"] == fmt and h["slice"] == slice_name
        ]
        if not headers:
            red(f"missing_{slice_name}_header", f"no {slice_name} header for run {run_id}")
        elif len(headers) > 1:
            red(f"duplicate_{slice_name}_header", f"{len(headers)} {slice_name} headers for run {run_id}")
        elif headers[0]["counts"]["count"] != len(rows):
            red(
                f"{slice_name}_count_mismatch",
                f"the {slice_name} header counts {headers[0]['counts']['count']} rows, the file holds {len(rows)}",
            )

    problems = _fixed_problems(fixed_rows)
    if "missing" in problems:
        red("missing_fixed_rows", f"the fixed slice has {len(set(r['seed'] for r in fixed_rows))} of {len(FIXED_SEEDS)} seeds")
    if "duplicate" in problems:
        red("duplicate_fixed_rows", "the fixed slice repeats a seed or holds one outside 0-99")
    if not fresh_rows:
        red("missing_fresh_rows", "the fresh slice has no rows")
    elif rows_lib.find_duplicates(fresh_rows):
        red("duplicate_fresh_rows", "the fresh slice repeats a (document_type, seed, profile) key")

    budget = sorted(r["seed"] for r in fixed_rows if r["retry_stop"] == "budget")
    if budget:
        red("fixed_budget_stop", f"the retry budget stopped {len(budget)} fixed seed(s): {_seed_list(budget)}; the measurement is invalid")
    return findings


def _seed_list(seeds: list[int], limit: int = 20) -> str:
    shown = ", ".join(str(seed) for seed in seeds[:limit])
    return shown + (f" and {len(seeds) - limit} more" if len(seeds) > limit else "")


def _compare_fixed(tonight: dict, reference: dict) -> dict:
    """Pair two fixed slices by seed. `tonight` and `reference` map seed to row."""
    flipped: dict[int, list[str]] = {}
    for seed in FIXED_SEEDS:
        before, after = _outcome(reference[seed]), _outcome(tonight[seed])
        changes = []
        if before["hit"] != after["hit"]:
            changes.append("hit_gained" if after["hit"] else "hit_lost")
        if before["correct"] != after["correct"]:
            changes.append("correct_gained" if after["correct"] else "correct_lost")
        if before["wrong_accept"] != after["wrong_accept"]:
            changes.append("wrong_accept_new" if after["wrong_accept"] else "wrong_accept_cleared")
        if before["prefix_wrong"] != after["prefix_wrong"]:
            changes.append("prefix_wrong_new" if after["prefix_wrong"] else "prefix_wrong_cleared")
        if changes:
            flipped[seed] = changes
    counts = {token: sum(1 for changes in flipped.values() if token in changes) for token in FLIP_TOKENS}
    return {
        "flipped": flipped,
        "flip_counts": counts,
        "new_wrong_accept_seeds": sorted(s for s, c in flipped.items() if "wrong_accept_new" in c),
        "new_prefix_wrong_seeds": sorted(s for s, c in flipped.items() if "prefix_wrong_new" in c),
        "net_hit_loss": counts["hit_lost"] - counts["hit_gained"],
    }


def _pool(entries: list[dict]) -> dict:
    """Summed counts over several nights of one format's fresh slice."""
    pooled = {"count": 0, "hits": 0, "correct_reads": 0, "wrong_accepts": 0, "budget_stops": 0}
    for entry in entries:
        counts = rows_lib.totals(entry["fresh"])
        pooled["count"] += counts["count"]
        pooled["hits"] += counts["hits"]
        pooled["wrong_accepts"] += counts["wrong_accepts"]
        pooled["correct_reads"] += counts["hits"] - counts["wrong_accepts"]
        pooled["budget_stops"] += sum(1 for row in entry["fresh"] if row["retry_stop"] == "budget")
    return pooled


# (name, pooled count key, the direction a flag runs in)
FRESH_TESTS = (
    ("hit_rate", "hits", -1),
    ("correct_rate", "correct_reads", -1),
    ("wrong_accept_rate", "wrong_accepts", +1),
)


def _empty_fresh(status: str, rows: list[dict]) -> dict:
    """A fresh section before any test: tonight's pooled counts, empty windows, no z."""
    empty = {"nights": 0, "count": 0, "hits": 0, "correct_reads": 0, "wrong_accepts": 0, "budget_stops": 0}
    return {
        "status": status,
        **_pool([{"fresh": rows}]),
        "recent": dict(empty),
        "baseline": dict(empty),
        "z": {name: None for name, _, _ in FRESH_TESTS},
    }


# What one pooled window reports.
FRESH_WINDOW_KEYS = ("nights", "count", "hits", "correct_reads", "wrong_accepts")


def _night(timestamp: int | None) -> str:
    if timestamp is None:
        return "unknown date"
    return datetime.datetime.fromtimestamp(timestamp, datetime.timezone.utc).strftime("%Y-%m-%d")


def analyse_format(fmt: str, history: History, run_id: str, decide_generator) -> tuple[dict, list[Finding]]:
    """Everything the advisory says about one format, and the findings behind it."""
    fixed_index, fresh_index = _by_key(history.fixed), _by_key(history.fresh)
    fixed_rows = fixed_index.get((run_id, fmt), [])
    fresh_rows = fresh_index.get((run_id, fmt), [])
    findings = _check_tonight(fmt, history, run_id, fixed_rows, fresh_rows)

    entries = _run_entries(history, fmt, fixed_index, fresh_index)
    position = next((i for i, entry in enumerate(entries) if entry["run_id"] == run_id), None)
    tonight = entries[position] if position is not None else None
    earlier = entries[:position] if position is not None else []

    fixed_counts = rows_lib.totals(fixed_rows) if fixed_rows else None
    section = {
        "generator_fingerprint": tonight["fingerprint"] if tonight else None,
        "git_sha": tonight["git_sha"] if tonight else None,
        "timestamp": tonight["timestamp"] if tonight else None,
        "reference_run_id": None,
        "reference_git_sha": None,
        "reference_timestamp": None,
        "fixed": {
            "status": "invalid",
            "count": len(fixed_rows),
            "hits": fixed_counts["hits"] if fixed_counts else 0,
            "wrong_accepts": fixed_counts["wrong_accepts"] if fixed_counts else 0,
            "prefix_wrong_accepts": fixed_counts["prefix_wrong_accepts"] if fixed_counts else 0,
            "budget_stops": sum(1 for row in fixed_rows if row["retry_stop"] == "budget"),
            "flipped": {},
            "flip_counts": {token: 0 for token in FLIP_TOKENS},
            "new_wrong_accept_seeds": [],
            "new_prefix_wrong_seeds": [],
            "net_hit_loss": 0,
        },
        "fresh": _empty_fresh("no_data" if not fresh_rows else "not_judged", fresh_rows),
    }

    # From here on, tonight's fixed slice must be usable: a fingerprint exists only then.
    if tonight is None or tonight["fingerprint"] is None:
        if tonight is not None and not _fixed_problems(fixed_rows):
            findings.append(Finding("red", f"{fmt}:fingerprint_header_mismatch", f"{fmt}: the fixed header's fingerprint is not the rows' own"))
        return section, findings

    fingerprint = tonight["fingerprint"]
    findings.extend(_fingerprint_change(fmt, tonight, earlier, decide_generator))

    same_fingerprint = [entry for entry in earlier if entry["fingerprint"] == fingerprint]
    reference = same_fingerprint[-1] if same_fingerprint else None
    if reference is None:
        section["fixed"]["status"] = "no_reference"
    else:
        section["reference_run_id"] = reference["run_id"]
        section["reference_git_sha"] = reference["git_sha"]
        section["reference_timestamp"] = reference["timestamp"]
        compared = _compare_fixed(tonight["fixed"], reference["fixed"])
        section["fixed"].update(compared, status="paired")
        findings.extend(_fixed_findings(fmt, compared, reference["run_id"]))

    findings.extend(_fresh_section(fmt, section, tonight, same_fingerprint))
    return section, findings


def _fingerprint_change(fmt: str, tonight: dict, earlier: list[dict], decide_generator) -> list[Finding]:
    """A fingerprint that differs from the previous run's must be explained by a generator change."""
    previous = next((entry for entry in reversed(earlier) if entry["fingerprint"] is not None), None)
    if previous is None or previous["fingerprint"] == tonight["fingerprint"]:
        return []
    changed = decide_generator(previous["git_sha"], tonight["git_sha"])
    if changed is True:
        return []  # explained: the reference resets, and the fixed slice reports "no reference"
    if changed is False:
        return [
            Finding(
                "red",
                f"{fmt}:fingerprint_changed_without_generator_change",
                f"{fmt}: the fixed slice renders differently from run {previous['run_id']}, and no commit "
                f"changed the generator between {previous['git_sha'][:12]} and {tonight['git_sha'][:12]}",
            )
        ]
    return [
        Finding(
            "warning",
            f"{fmt}:fingerprint_change_undecided",
            f"{fmt}: the fixed slice renders differently from run {previous['run_id']}, and the generator "
            f"history between {previous['git_sha'][:12]} and {tonight['git_sha'][:12]} is not available to check",
        )
    ]


def _fixed_findings(fmt: str, compared: dict, reference_run: str) -> list[Finding]:
    findings = []
    if compared["new_wrong_accept_seeds"]:
        seeds = compared["new_wrong_accept_seeds"]
        findings.append(
            Finding("red", f"{fmt}:new_wrong_accept",
                    f"{fmt}: fixed seed(s) newly a wrong accept against run {reference_run}: {_seed_list(seeds)}")
        )
    if compared["new_prefix_wrong_seeds"]:
        seeds = compared["new_prefix_wrong_seeds"]
        findings.append(
            Finding("red", f"{fmt}:new_prefix_wrong",
                    f"{fmt}: fixed seed(s) newly a prefix-wrong read against run {reference_run}: {_seed_list(seeds)}")
        )
    if compared["net_hit_loss"] > NET_FLIP_WARN_ABOVE:
        findings.append(
            Finding("warning", f"{fmt}:net_hit_loss",
                    f"{fmt}: the fixed slice lost {compared['net_hit_loss']} more hits than it gained "
                    f"(warning above {NET_FLIP_WARN_ABOVE}) against run {reference_run}")
        )
    return findings


def _fresh_section(fmt: str, section: dict, tonight: dict, same_fingerprint: list[dict]) -> list[Finding]:
    """The fresh-slice pooled test. Fills `section['fresh']` and returns its findings."""
    findings = []
    usable = [entry for entry in same_fingerprint if entry["fresh"]]
    recent = [tonight, *reversed(usable[-(RECENT_NIGHTS - 1):])] if tonight["fresh"] else []
    older = usable[: max(0, len(usable) - (RECENT_NIGHTS - 1))]
    baseline = list(reversed(older[-BASELINE_NIGHTS:]))

    fresh = section["fresh"]
    fresh["status"] = "tested" if tonight["fresh"] else "no_data"
    fresh["recent"] = {"nights": len(recent), **_pool(recent)}
    fresh["baseline"] = {"nights": len(baseline), **_pool(baseline)}
    if fresh["budget_stops"]:
        findings.append(
            Finding("warning", f"{fmt}:fresh_budget_stop",
                    f"{fmt}: the retry budget stopped {fresh['budget_stops']} fresh document(s) tonight; those measurements are invalid")
        )
    if not tonight["fresh"]:
        return findings
    if len(recent) < RECENT_NIGHTS or len(baseline) < MIN_BASELINE_NIGHTS:
        fresh["status"] = "too_little_history"
        return findings

    now, before = fresh["recent"], fresh["baseline"]
    for name, key, direction in FRESH_TESTS:
        z = two_proportion_z(now[key], now["count"], before[key], before["count"])
        fresh["z"][name] = z
        if z is not None and z * direction >= Z_FLAG:
            side = "below" if direction < 0 else "above"
            findings.append(
                Finding("warning", f"{fmt}:fresh_{name}",
                        f"{fmt}: the fresh {name.replace('_', ' ')} is {side} the prior {before['nights']} nights "
                        f"(z = {z:+.2f}, last {now['nights']} nights: {now[key]}/{now['count']}, "
                        f"prior: {before[key]}/{before['count']})")
            )
    return findings


# --- the whole night --------------------------------------------------------------------------


def analyse(history: History, run_id: str, decide_generator) -> dict:
    """Judge one run. `decide_generator(old_sha, new_sha)` is `True`, `False` or `None` (cannot tell)."""
    if not _RUN_ID.match(run_id):
        raise AdvisoryError("run id must be <run id>-<attempt>")
    formats, findings = {}, []
    for fmt in rows_lib.FORMATS:
        formats[fmt], found = analyse_format(fmt, history, run_id, decide_generator)
        findings.extend(found)
    stamps = [section["timestamp"] for section in formats.values() if section["timestamp"] is not None]
    shas = sorted({section["git_sha"] for section in formats.values() if section["git_sha"]})
    return {
        "run_id": run_id,
        "timestamp": min(stamps) if stamps else None,
        "git_sha": shas[0] if len(shas) == 1 else None,
        "formats": formats,
        "findings": findings,
        "red": any(finding.kind == "red" for finding in findings),
    }


# --- the outputs ------------------------------------------------------------------------------


def _round(value: float | None, places: int = 3) -> float | None:
    return None if value is None else round(value, places)


def advisory_line(result: dict) -> dict:
    """The one `advisory.jsonl` line for a night: counts, rates, z values, codes, ids, seed numbers."""
    formats = {}
    for fmt, section in result["formats"].items():
        fixed, fresh = section["fixed"], section["fresh"]
        recent, baseline = fresh["recent"], fresh["baseline"]
        formats[fmt] = {
            "generator_fingerprint": section["generator_fingerprint"],
            "reference_run_id": section["reference_run_id"],
            "reference_git_sha": section["reference_git_sha"],
            "fixed": {
                "status": fixed["status"],
                "count": fixed["count"],
                "hits": fixed["hits"],
                "wrong_accepts": fixed["wrong_accepts"],
                "prefix_wrong_accepts": fixed["prefix_wrong_accepts"],
                "budget_stops": fixed["budget_stops"],
                "flipped_seeds": sorted(fixed["flipped"]),
                "new_wrong_accept_seeds": fixed["new_wrong_accept_seeds"],
                "new_prefix_wrong_seeds": fixed["new_prefix_wrong_seeds"],
                "flip_counts": fixed["flip_counts"],
                "net_hit_loss": fixed["net_hit_loss"],
            },
            "fresh": {
                "status": fresh["status"],
                "count": fresh["count"],
                "hits": fresh["hits"],
                "correct_reads": fresh["correct_reads"],
                "wrong_accepts": fresh["wrong_accepts"],
                "budget_stops": fresh["budget_stops"],
                "recent": {key: recent[key] for key in FRESH_WINDOW_KEYS},
                "baseline": {key: baseline[key] for key in FRESH_WINDOW_KEYS},
                "z": {name: _round(fresh["z"][name]) for name, _, _ in FRESH_TESTS},
            },
        }
    line = {
        "schema": ADVISORY_SCHEMA,
        "run_id": result["run_id"],
        "run_timestamp_unix": result["timestamp"],
        "git_sha": result["git_sha"],
        "red": result["red"],
        "red_codes": sorted(f.code for f in result["findings"] if f.kind == "red"),
        "warning_codes": sorted(f.code for f in result["findings"] if f.kind == "warning"),
        "thresholds": {
            "net_flip_warn_above": NET_FLIP_WARN_ABOVE,
            "z_flag": Z_FLAG,
            "recent_nights": RECENT_NIGHTS,
            "baseline_nights": BASELINE_NIGHTS,
            "min_baseline_nights": MIN_BASELINE_NIGHTS,
        },
        "formats": formats,
    }
    validate_advisory_line(line)
    return line


def validate_advisory_line(value, path: str = "line") -> None:
    """Refuse an advisory line that holds anything but aggregates: an unknown key, or a string that
    is not a short token. Raises `AdvisoryError`; the message names the path, never a value."""
    if isinstance(value, dict):
        for key, item in value.items():
            if key not in ADVISORY_KEYS and key not in rows_lib.FORMATS:
                raise AdvisoryError(f"{path}: key outside the advisory allowlist")
            validate_advisory_line(item, f"{path}.{key}")
    elif isinstance(value, list):
        for index, item in enumerate(value):
            validate_advisory_line(item, f"{path}[{index}]")
    elif isinstance(value, str):
        if not _AGGREGATE_TOKEN.match(value):
            raise AdvisoryError(f"{path}: a string that is not a short token")
    elif not (value is None or isinstance(value, (bool, int, float))):
        raise AdvisoryError(f"{path}: a value that is not a number, boolean or token")


def _percent(count: int, total: int) -> str:
    return f"{100 * count / total:.1f}%" if total else "n/a"


def render_summary(result: dict) -> str:
    """The Markdown job summary. Aggregates and seed numbers only."""
    out = [f"## Nightly advisory: run {result['run_id']} ({_night(result['timestamp'])})", ""]
    out.append("**Red.** The findings below marked `error` fail this job." if result["red"] else "**Green.** Nothing here fails the job.")
    out += ["", "### Fixed slice (clean, seeds 0-99), paired per seed against the latest night with the same generator fingerprint", ""]
    out += ["| Format | Reference | Hits | Wrong accepts | Prefix-wrong | Seeds flipped | Net hit loss |", "| --- | --- | ---: | ---: | ---: | ---: | ---: |"]
    for fmt, section in result["formats"].items():
        fixed = section["fixed"]
        if fixed["status"] == "paired":
            reference = f"run {section['reference_run_id']} ({_night(section['reference_timestamp'])})"
        elif fixed["status"] == "no_reference":
            reference = "no reference (no earlier night has this fingerprint)"
        else:
            reference = "not judged (invalid measurement)"
        flipped = len(fixed["flipped"]) if fixed["status"] == "paired" else "-"
        loss = fixed["net_hit_loss"] if fixed["status"] == "paired" else "-"
        out.append(
            f"| {fmt} | {reference} | {fixed['hits']}/{fixed['count']} | {fixed['wrong_accepts']} "
            f"| {fixed['prefix_wrong_accepts']} | {flipped} | {loss} |"
        )
    named = [
        f"- {fmt} seed {seed}: {', '.join(changes)}"
        for fmt, section in result["formats"].items()
        for seed, changes in sorted(section["fixed"]["flipped"].items())
    ]
    if named:
        out += ["", "Flipped seeds:", "", *named]

    out += ["", f"### Fresh slice (all profiles, new seeds), the last {RECENT_NIGHTS} nights against the prior {BASELINE_NIGHTS}", ""]
    out += ["| Format | Nights (recent / prior) | Hit rate | Correct-read rate | Wrong-accept rate |", "| --- | --- | ---: | ---: | ---: |"]
    for fmt, section in result["formats"].items():
        fresh = section["fresh"]
        if fresh["status"] == "no_data":
            out.append(f"| {fmt} | - | no fresh rows | | |")
            continue
        recent, baseline = fresh["recent"], fresh["baseline"]
        nights = f"{recent['nights']} / {baseline['nights']}"
        if fresh["status"] == "tested":
            cells = []
            for name, key, _ in FRESH_TESTS:
                cell = f"{_percent(recent[key], recent['count'])} vs {_percent(baseline[key], baseline['count'])}"
                z = fresh["z"][name]
                cells.append(f"{cell} (z {z:+.2f})" if z is not None else cell)
        else:
            reason = "too little history, test skipped" if fresh["status"] == "too_little_history" else "not judged"
            nights += f" ({reason})"
            cells = [f"{_percent(fresh[key], fresh['count'])} tonight" for _, key, _ in FRESH_TESTS]
        out.append(f"| {fmt} | {nights} | " + " | ".join(cells) + " |")

    out += ["", "### Findings", ""]
    if result["findings"]:
        out += [f"- `{'error' if f.kind == 'red' else 'warning'}` {f.message}" for f in result["findings"]]
    else:
        out.append("None.")
    return "\n".join(out) + "\n"


def annotations(result: dict) -> list[str]:
    """One workflow command per finding: `::error::` for red, `::warning::` otherwise."""
    return [f"::{'error' if f.kind == 'red' else 'warning'}::{f.message}" for f in result["findings"]]


def append_advisory(path: Path, line: dict) -> bool:
    """Append the night's line to `advisory.jsonl`, once per run id. Returns whether it wrote."""
    path = Path(path)
    if path.is_file():
        for number, existing in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
            if not existing.strip():
                continue
            try:
                parsed = json.loads(existing)
            except json.JSONDecodeError as error:
                raise AdvisoryError(f"{path}:{number}: not JSON ({error.msg})") from error
            if isinstance(parsed, dict) and parsed.get("run_id") == line["run_id"]:
                return False
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(line, sort_keys=True, separators=(",", ":")) + "\n")
    return True


# --- command line -----------------------------------------------------------------------------


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("--data-dir", type=Path, required=True, help="a checkout of the bench-data branch")
    parser.add_argument("--run-id", required=True, help="tonight's run, <GITHUB_RUN_ID>-<GITHUB_RUN_ATTEMPT>")
    parser.add_argument("--summary", type=Path, help="append the Markdown summary here (default: $GITHUB_STEP_SUMMARY)")
    parser.add_argument("--advisory-out", type=Path, help="the advisory.jsonl to append tonight's line to")
    parser.add_argument("--repo", type=Path, help="a checkout of main, to tell whether the generator changed")
    args = parser.parse_args(argv)

    summary_path = args.summary
    if summary_path is None and os.environ.get("GITHUB_STEP_SUMMARY"):
        summary_path = Path(os.environ["GITHUB_STEP_SUMMARY"])

    try:
        history = load_history(args.data_dir)
        result = analyse(history, args.run_id, lambda old, new: generator_changed(args.repo, old, new))
        line = advisory_line(result)
        if summary_path is not None:
            with summary_path.open("a", encoding="utf-8") as handle:
                handle.write(render_summary(result))
        if args.advisory_out is not None and not append_advisory(args.advisory_out, line):
            print(f"::warning::{args.advisory_out.name} already holds run {args.run_id}; not appended again")
    except (rows_lib.RowError, AdvisoryError, OSError, json.JSONDecodeError) as error:
        print(f"::error::advisory: {type(error).__name__}: {error}", file=sys.stderr)
        return 2

    for annotation in annotations(result):
        print(annotation)
    print(f"advisory: run {args.run_id}: {'red' if result['red'] else 'green'}, {len(result['findings'])} finding(s)")
    return 1 if result["red"] else 0


if __name__ == "__main__":
    sys.exit(main())
