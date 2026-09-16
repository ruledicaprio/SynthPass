#!/usr/bin/env python3
"""
rebless.py

One command from "cohort PR open, images pushed" to "PR ready for review" --
the re-bless procedure `tools/apply_cohort.py` deliberately leaves undone
(its own docstring: "Out of scope ... dispatching the real-specimen re-bless
workflow, waiting for it, downloading or committing the baseline, or merging
the PR"). Every cohort PR since c01 has had a human do the following steps by
hand, in order; this tool automates the mechanical parts and stops for a human
(or `synthpass-analyst`) exactly where judgement is required:

1. Dispatch `real-specimen-gate.yml` on the cohort branch with
   `mode=write-baseline` and wait for it (35-50 minutes in CI).
2. Download the `real-specimen-mrz-baseline` and `real-specimen-gate-report`
   artifacts.
3. Diff the new baseline against the one already committed on the branch, key
   by key, and classify the result:
   - **identical** -- only CI provenance (`measured_on_ci_sha`,
     `samples_data_sha`, `measured_date`) moved;
   - **non-scored delta** -- an off-denominator bucket moved (`documents`,
     `no_mrz_expected`, `redacted_mrz`, `checksum_failed_specimen`) while
     `scored`, `tier1_hits` and every scored miss bucket
     (`checksum_failed`, `no_mrz_found`, `ocr_error`,
     `document_number_mismatch`, `false_positive_mrz`) held still;
   - **scored delta** -- anything scored moved, or an unrecognized field
     moved (never guessed at: an unknown field is treated as a scored delta
     on purpose, the conservative direction).
4. Install the new baseline. For an "identical" or "non-scored delta"
   result, mechanically rewrite the numbers this loop has hand-edited every
   time so far: `README.md`'s gap sentence and corpus-wide rate,
   `knowledge/benchmarks/README.md`'s live block (both rates, the outcomes
   heading's document count, the six outcome-table bucket counts, and the
   measured-date Source cell), and a templated dated entry appended under
   `knowledge/benchmarks/README.md`'s `## Weak-spot findings`. Then re-runs
   `scripts/check-headline-numbers.sh` to confirm the rewrite actually
   closed the gap. `knowledge/ROADMAP.md` states only the scored rate, which
   by definition cannot move in either of these two classes, so it is never
   touched here.
5. For a **scored delta**, this tool stops right after installing the new
   baseline and printing the diff table. It never writes prose for a result
   that moved the numbers accuracy work is scored on -- that entry is
   `synthpass-analyst`'s to write, same discipline as
   `tools/apply_cohort.py`'s labelled DRAFT notes.
6. Commit, push, dispatch the `mode=assert` run and wait for it, `gh pr
   ready`, and PATCH the PR body with a "Re-bless result" paragraph -- all
   gated behind `--confirm` (see "Two independent safety layers" below).

Standard library only, matching `tools/apply_cohort.py`. Reuses that module's
`run_cmd`/`find_git_bash`/`run_bash_script`/`to_msys_path` and its three `gh`
helpers (`gh_repo_owner_and_name`, `gh_pr_number_for_branch`,
`gh_patch_pr_title_and_body`) by import rather than duplicating them --
`apply_cohort.py` has no import-time side effects (its `argparse` setup lives
entirely inside `main()`), so a plain `import apply_cohort` is safe and no
`tools/gh_helpers.py` split was needed.

## Two independent safety layers, not one (same shape as `apply_cohort.py`)

- **`--dry-run`** performs the one read that must always run regardless --
  verifying the worktree exists and is on the cohort branch -- and then
  stops before anything else, printing what it would do. It never dispatches
  a workflow, never downloads an artifact (that writes local files), never
  edits a document, never commits. This is deliberately the whole of what
  `--dry-run` means here, with no case-by-case exception: a run whose only
  purpose is preview must never leave a trace, in CI state or on disk.
- **`--confirm`** gates the truly consequential steps specifically: dispatch
  (both the `write-baseline` run this tool needs to get a result, and the
  `assert` run at the end), `git commit`/`git push`, `gh pr ready`, and the
  PATCH of the PR body. Without `--confirm`, and without `--run-id` to reuse
  an already-completed run, there is nothing to diff against, so the tool
  stops immediately and says so. With `--run-id` but without `--confirm`,
  it downloads the (already-produced, so read-only from this tool's
  perspective) artifacts, classifies the diff, writes the mechanical doc
  edits to the worktree, and stops -- exactly the point at which a human
  should read the diff table and the rewritten prose before anything is
  pushed anywhere.

## Why the branch/worktree check always runs, dry-run or not

Every git operation here is `git -C <worktree> ...` -- this tool never runs
`git checkout`/`git switch` in the calling repository, and never assumes the
worktree it was pointed at is actually on the branch its name implies. A
worktree can be reused across a rebase, a stale checkout, or simply the wrong
directory; committing a re-blessed baseline onto the wrong branch is exactly
the kind of mistake `CLAUDE.md`'s branch-state discipline exists to catch
before it happens, not after a `git log` surprises someone.

## Usage

    python tools/rebless.py --cohort-branch cohort-c14 [--dry-run]
    python tools/rebless.py --cohort-branch cohort-c14 --confirm
    python tools/rebless.py --cohort-branch cohort-c14 --run-id 34980671424 --confirm
    python tools/rebless.py --cohort-branch cohort-c14 --run-id 34980671424 \\
        --skip-assert --confirm   # re-run after an earlier interrupted attempt
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from datetime import datetime, timezone
from decimal import ROUND_HALF_UP, Decimal
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import apply_cohort as ac  # noqa: E402 -- see the module docstring for why a plain import is safe

# --------------------------------------------------------------------------
# Baseline schema constants, mirrored from
# crates/synthpass-bench/src/bin/provider-bench.rs (`OFF_DENOMINATOR_KINDS`,
# `REGRESSION_BUCKETS`) the same way apply_cohort.py mirrors the manifest
# contract: that Rust source, not this file, is what CI actually enforces.
# --------------------------------------------------------------------------

WORKFLOW = "real-specimen-gate.yml"
EXPECTED_RUNTIME = "35-50 minutes (measured 39m29s once; see real-specimen-gate.yml)"

BASELINE_REL_PATH = "knowledge/benchmarks/real-specimen-mrz-baseline.json"
README_REL_PATH = "README.md"
BENCH_README_REL_PATH = "knowledge/benchmarks/README.md"

# knowledge/ROADMAP.md is deliberately absent here: its one live figure is the
# scored rate (`tier1_hits`/`scored`), which by construction cannot move in
# either class this tool ever edits docs for (identical, non-scored delta) --
# only a scored delta moves it, and a scored delta stops before any doc edit.

PROVENANCE_KEYS = ("measured_on_ci_sha", "samples_data_sha", "measured_date")
NON_SCORED_KEYS = ("documents", "no_mrz_expected", "redacted_mrz", "checksum_failed_specimen")
SCORED_KEYS = ("scored", "tier1_hits")
# The full REGRESSION_BUCKETS set, not just the three the task narrative names
# (the three that happen to be non-zero in the corpus today) -- an unnamed
# member moving is still a scored miss and must never be classified otherwise.
SCORED_MISS_KEYS = ("checksum_failed", "no_mrz_found", "ocr_error", "document_number_mismatch", "false_positive_mrz")
# `by_miss_kind` is a Rust BTreeMap built with `.entry(kind).or_default()` --
# a miss kind with zero occurrences is simply absent from the JSON, not
# present with a 0 value. Every key in this tuple must default to 0 when
# flattening, not just `false_positive_mrz` (the one the committed baseline
# happens to omit today).
ALL_MISS_KIND_KEYS = tuple(sorted(set(SCORED_MISS_KEYS) | {"redacted_mrz", "no_mrz_expected", "checksum_failed_specimen"}))

CLASS_IDENTICAL = "identical"
CLASS_NON_SCORED = "non-scored delta"
CLASS_SCORED = "scored delta"


# ==========================================================================
# Baseline diffing (pure)
# ==========================================================================


def flatten_baseline(doc: dict) -> dict:
    """A baseline JSON's `by_miss_kind` sub-object folded into the same flat
    namespace `scripts/check-headline-numbers.sh` and this module both use,
    with every known miss-kind key defaulted to 0 when the bucket had zero
    occurrences (see `ALL_MISS_KIND_KEYS`'s docstring above)."""
    flat = {k: v for k, v in doc.items() if k not in ("note", "by_miss_kind")}
    flat.update(doc.get("by_miss_kind") or {})
    for key in ALL_MISS_KIND_KEYS:
        flat.setdefault(key, 0)
    return flat


def classify_baseline_diff(old: dict, new: dict) -> tuple[str, list[tuple[str, object, object]]]:
    """Classifies the move from `old` to `new` (both full baseline JSON
    documents) into one of `CLASS_IDENTICAL` / `CLASS_NON_SCORED` /
    `CLASS_SCORED`, per the rules in the module docstring's step 3. Returns
    `(class, changed)` where `changed` is every key whose flattened value
    differs, `(key, old_value, new_value)`, sorted by key -- including the
    provenance fields, so a caller can still report exactly what CI
    measured even when nothing else moved.

    An unrecognized key moving (something in neither `NON_SCORED_KEYS` nor
    `SCORED_KEYS`/`SCORED_MISS_KEYS`/`PROVENANCE_KEYS` -- `tolerance`
    changing, or a future schema field) is classified `CLASS_SCORED`: never
    guessed at, always the conservative direction that stops for a human."""
    old_flat, new_flat = flatten_baseline(old), flatten_baseline(new)
    all_keys = sorted(set(old_flat) | set(new_flat))
    changed = [(k, old_flat.get(k), new_flat.get(k)) for k in all_keys if old_flat.get(k) != new_flat.get(k)]
    changed_keys = {c[0] for c in changed}

    non_provenance = changed_keys - set(PROVENANCE_KEYS)
    if not non_provenance:
        return CLASS_IDENTICAL, changed

    scored_relevant = set(SCORED_KEYS) | set(SCORED_MISS_KEYS)
    if non_provenance & scored_relevant:
        return CLASS_SCORED, changed
    if non_provenance <= set(NON_SCORED_KEYS):
        return CLASS_NON_SCORED, changed
    return CLASS_SCORED, changed


def format_diff_table(changed: list[tuple[str, object, object]], new_flat: dict) -> str:
    """The delta table in the shape `provider-bench`'s own
    `print_baseline_table` prints (`crates/synthpass-bench/src/bin/provider-bench.rs`):
    `  {label:<26} {was:>5} -> {now:<5} ({diff:+d})`, one row per non-provenance
    field that moved, plus the trailing `(baseline measured DATE on SHA)` line.
    Provenance fields are excluded from the row list -- `provider-bench` never
    rows them either, they're the trailing line instead."""
    rows = [c for c in changed if c[0] not in PROVENANCE_KEYS]
    lines = []
    for key, old, new in sorted(rows):
        old_i, new_i = int(old or 0), int(new or 0)
        lines.append(f"  {key:<26} {old_i:>5} -> {new_i:<5} ({new_i - old_i:+d})")
    if not lines:
        lines.append("  (no field moved beyond CI provenance)")
    lines.append(f"  (baseline measured {new_flat.get('measured_date')} on {new_flat.get('measured_on_ci_sha')})")
    return "\n".join(lines)


def format_rate(hits: int, denom: int) -> str:
    """One decimal place, rounded half-up -- the same convention
    `scripts/check-headline-numbers.sh`'s `awk '%.1f'` uses, via `Decimal`
    for a rounding rule this module states explicitly rather than inherits
    from float formatting's platform-dependent tie-breaking."""
    if denom == 0:
        raise ValueError("cannot compute a rate over a zero denominator")
    value = Decimal(hits * 100) / Decimal(denom)
    return str(value.quantize(Decimal("0.1"), rounding=ROUND_HALF_UP))


# ==========================================================================
# Templated prose (identical / non-scored delta only -- see module docstring)
# ==========================================================================


def build_rebless_commit_message(cohort_branch: str, cls: str, changed: list[tuple[str, object, object]], run_id: str, new_flat: dict) -> str:
    cycle = cohort_branch.removeprefix("cohort-")
    subject = f"bench: re-bless the real-specimen baseline for cohort {cycle} ({cls})"
    table = format_diff_table(changed, new_flat)
    body = f"CI run {run_id} (`{WORKFLOW}`, mode=write-baseline) on `{cohort_branch}`:\n\n{table}\n"
    return f"{subject}\n\n{body}"


def build_rebless_result_paragraph(cls: str, changed: list[tuple[str, object, object]], run_id: str, new_flat: dict) -> str:
    """The "Re-bless result" paragraph appended to the cohort PR's body via
    `gh api -X PATCH` (`gh pr edit` lacks the `read:org` scope on this
    machine -- see `apply_cohort.py::gh_patch_pr_title_and_body`)."""
    table = format_diff_table(changed, new_flat)
    return (
        f"## Re-bless result\n\n"
        f"`{WORKFLOW}` run [{run_id}](../actions/runs/{run_id}) (mode=write-baseline), classified "
        f"**{cls}**:\n\n```\n{table}\n```\n"
    )


def build_weakspot_entry(cohort_branch: str, n_specimens: int, changed: list[tuple[str, object, object]], run_id: str, old_flat: dict, new_flat: dict) -> str:
    """A dated entry in the house shape (`knowledge/benchmarks/README.md`'s
    `## Weak-spot findings`, e.g. the 2026-09-15 cohort c10/c12 entry): a
    heading naming the cohort and what moved, a paragraph naming the run and
    the delta table, and the sentence stating the scored rate held. Templated
    from the numbers only -- this function never invents an explanation for
    *why* a bucket moved; that is exactly what a "non-scored delta" means
    (an ingest, not a behaviour change), and a scored delta never reaches
    this function at all (see the module docstring's step 5)."""
    cycle = cohort_branch.removeprefix("cohort-")
    measured_date = new_flat.get("measured_date")
    rows = sorted(c for c in changed if c[0] not in PROVENANCE_KEYS)
    what_moved = ", ".join(f"`{key}` {int(old or 0)} → **{int(new or 0)}**" for key, old, new in rows)
    heading = f"### {measured_date} — cohort {cycle}: {n_specimens} specimen(s), {what_moved}"
    table = format_diff_table(changed, new_flat)
    hits, scored = int(new_flat["tier1_hits"]), int(new_flat["scored"])
    rate = format_rate(hits, scored)
    body = (
        f"CI re-blessed on branch `{cohort_branch}` (`{WORKFLOW}` run `{run_id}`, mode=write-baseline): "
        f"only non-scored bucket(s) moved.\n\n```\n{table}\n```\n\n"
        f"The scored rate did not move: **{hits} / {scored} = {rate}%**, unchanged.\n"
    )
    return f"\n{heading}\n\n{body}"


def rewrite_readme_gap_and_corpus_rate(text: str, hits: int, old_flat: dict, new_flat: dict) -> str:
    """README.md's "gap" sentence (`The N-specimen gap ...`) and the
    corpus-wide rate bullet immediately above it (`hits / documents =
    rate%`) -- the two numbers a "non-scored delta" moves, since `documents`
    is the only `NON_SCORED_KEYS` member that changes either of them
    (`no_mrz_expected`/`redacted_mrz`/`checksum_failed_specimen` alone leave
    both untouched). Raises `ValueError` if the expected text is not found
    rather than silently leaving it stale."""
    old_documents, new_documents = int(old_flat["documents"]), int(new_flat["documents"])
    old_scored = int(old_flat["scored"])
    old_rate_str = f"{hits} / {old_documents} = {format_rate(hits, old_documents)}%"
    new_rate_str = f"{hits} / {new_documents} = {format_rate(hits, new_documents)}%"
    if old_rate_str not in text:
        raise ValueError(f"README.md: expected corpus-wide rate bullet {old_rate_str!r} not found")
    text = text.replace(old_rate_str, new_rate_str, 1)

    old_gap, new_gap = old_documents - old_scored, new_documents - int(new_flat["scored"])
    old_gap_str, new_gap_str = f"{old_gap}-specimen gap", f"{new_gap}-specimen gap"
    if old_gap_str not in text:
        raise ValueError(f"README.md: expected gap sentence {old_gap_str!r} not found")
    return text.replace(old_gap_str, new_gap_str, 1)


def _replace_or_raise(text: str, old: str, new: str, what: str) -> str:
    if old not in text:
        raise ValueError(f"{what}: expected text {old!r} not found")
    return text.replace(old, new, 1)


def rewrite_benchmarks_readme_live_block(text: str, hits: int, old_flat: dict, new_flat: dict) -> str:
    """`knowledge/benchmarks/README.md`'s "Current headline numbers" live
    block: the corpus-wide rate row, the outcomes heading's document count,
    every outcome-table bucket count that moved, and the scored-rate row's
    Source-cell measured date. The scored-rate row's own numbers
    (`hits / scored = rate%`) are never touched here -- `scored` and `hits`
    cannot move in a non-scored delta, the only class that reaches this
    function."""
    old_documents, new_documents = int(old_flat["documents"]), int(new_flat["documents"])
    old_scored = int(old_flat["scored"])

    old_corpus = f"{hits} / {old_documents} = {format_rate(hits, old_documents)}%"
    new_corpus = f"{hits} / {new_documents} = {format_rate(hits, new_documents)}%"
    text = _replace_or_raise(text, old_corpus, new_corpus, f"{BENCH_README_REL_PATH} corpus-wide rate")

    old_heading = f"({old_documents} documents)"
    new_heading = f"({new_documents} documents)"
    text = _replace_or_raise(text, old_heading, new_heading, f"{BENCH_README_REL_PATH} outcomes heading")

    for key in NON_SCORED_KEYS:
        if key == "documents":
            continue
        old_v, new_v = int(old_flat.get(key, 0)), int(new_flat.get(key, 0))
        if old_v == new_v:
            continue
        pattern = re.compile(rf"(`{key}`[^|\n]*\|[ \t]*)(\*{{0,2}})({old_v})(\*{{0,2}})([ \t]*\|)")
        if not pattern.search(text):
            raise ValueError(f"{BENCH_README_REL_PATH}: expected outcome-table row for `{key}` = {old_v} not found")
        text = pattern.sub(rf"\g<1>\g<2>{new_v}\g<4>\g<5>", text, count=1)

    old_gap, new_gap = old_documents - old_scored, new_documents - int(new_flat["scored"])
    old_why = f"{old_gap} of the {old_documents} specimens cannot produce"
    new_why = f"{new_gap} of the {new_documents} specimens cannot produce"
    text = _replace_or_raise(text, old_why, new_why, f"{BENCH_README_REL_PATH} 'Why two rates' sentence")

    date_pattern = re.compile(r"(real-specimen-mrz-baseline\.json`\s*\(CI,\s*)\d{4}-\d{2}-\d{2}(\))")
    if not date_pattern.search(text):
        raise ValueError(f"{BENCH_README_REL_PATH}: expected the scored-rate row's '(CI, YYYY-MM-DD)' Source cell not found")
    text = date_pattern.sub(rf"\g<1>{new_flat.get('measured_date')}\g<2>", text, count=1)

    return text


# ==========================================================================
# `gh run list` selection -- distinguishing this tool's own dispatch from a
# `push`/`pull_request` run of the same workflow the branch push also fired.
# ==========================================================================


def _parse_gh_timestamp(ts: str) -> datetime:
    return datetime.fromisoformat(ts.replace("Z", "+00:00"))


def select_dispatch_run(runs: list[dict], dispatched_at: datetime) -> dict:
    """`runs` is already-parsed `gh run list --json databaseId,status,event,createdAt`
    output. Returns the earliest `workflow_dispatch` run created at or after
    `dispatched_at` -- a push to the branch also triggers a `pull_request` (or
    `push`) run of the same workflow, which must never be mistaken for the run
    this tool itself just dispatched. Raises `ValueError` when none match,
    rather than guessing at the most recent run of any event type."""
    candidates: list[tuple[datetime, dict]] = []
    for run in runs:
        if run.get("event") != "workflow_dispatch":
            continue
        created = run.get("createdAt")
        if not created:
            continue
        created_dt = _parse_gh_timestamp(created)
        if created_dt >= dispatched_at:
            candidates.append((created_dt, run))
    if not candidates:
        raise ValueError(
            f"no workflow_dispatch run of {WORKFLOW!r} created at/after {dispatched_at.isoformat()} "
            "found in `gh run list` output -- the dispatch may not have registered yet"
        )
    candidates.sort(key=lambda pair: pair[0])
    return candidates[0][1]


# ==========================================================================
# Worktree / branch verification (always runs, dry-run or not)
# ==========================================================================


class WorktreeBranchMismatch(RuntimeError):
    pass


def build_watch_command(run_id: str) -> list[str]:
    """The blocking wait on one CI run. `--interval 60` because the default
    redraws the run's job list every few seconds and every redraw is a line in
    a captured log: one cohort's wait left 33,000 of them behind.
    `--exit-status` is what makes the wait report the run's own result."""
    return ["gh", "run", "watch", run_id, "--interval", "60", "--exit-status"]


def watch_run(repo_root: Path, run_id: str) -> None:
    """Blocks on `run_id` with the progress output discarded -- two printed
    lines, one before and one after, instead of a live-redrawn job table.
    Raises `RuntimeError` when the run did not succeed."""
    print(f"watching run {run_id}, expected {EXPECTED_RUNTIME}")
    proc = subprocess.run(
        build_watch_command(run_id),
        cwd=repo_root,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
        text=True,
        check=False,
    )
    if proc.returncode != 0:
        raise RuntimeError(
            f"run {run_id}: did not succeed (the wait exited {proc.returncode})\n{proc.stderr}"
        )
    print(f"run {run_id}: success")


def verify_worktree_on_branch(worktree: Path, expected_branch: str, current_branch_fn=None) -> None:
    """Fails loudly when `worktree` does not exist, or exists but is not
    checked out on `expected_branch`. `current_branch_fn` is injected for
    testing (defaults to a real `git -C <worktree> branch --show-current`
    call) -- this check must always run, `--dry-run` or not, see the module
    docstring."""
    if not worktree.is_dir():
        raise WorktreeBranchMismatch(f"worktree {worktree} does not exist -- create it first (see tools/apply_cohort.py)")
    if current_branch_fn is None:
        proc = subprocess.run(
            ["git", "-C", str(worktree), "branch", "--show-current"], capture_output=True, text=True, check=False
        )
        if proc.returncode != 0:
            raise WorktreeBranchMismatch(f"`git -C {worktree} branch --show-current` failed: {proc.stderr.strip()}")
        current = proc.stdout.strip()
    else:
        current = current_branch_fn(worktree)
    if current != expected_branch:
        raise WorktreeBranchMismatch(
            f"worktree {worktree} is on branch {current!r}, expected {expected_branch!r} -- "
            "refusing to re-bless a baseline onto the wrong branch"
        )


# ==========================================================================
# GitHub plumbing not already in apply_cohort.py
# ==========================================================================


def gh_pr_title_and_body(repo_root: Path, pr_number: int) -> tuple[str, str]:
    out = ac.run_cmd(["gh", "pr", "view", str(pr_number), "--json", "title,body"], cwd=repo_root)
    data = json.loads(out)
    return data.get("title", ""), data.get("body", "") or ""


def dispatch_and_wait(repo_root: Path, cohort_branch: str, mode: str, dry_run: bool) -> str | None:
    """Dispatches `real-specimen-gate.yml -f mode=<mode>` on `cohort_branch`,
    finds the run this dispatch itself created (`select_dispatch_run`), and
    blocks on it with a single `watch_run` (one wait, `--interval 60`, output
    discarded) -- never a short-interval polling loop (see `CLAUDE.md`'s
    CI-monitoring rule).
    Returns the run id, or `None` under `--dry-run` (nothing was dispatched
    to find)."""
    dispatched_at = datetime.now(timezone.utc)
    ac.run_cmd(["gh", "workflow", "run", WORKFLOW, "--ref", cohort_branch, "-f", f"mode={mode}"], cwd=repo_root, dry_run=dry_run)
    if dry_run:
        print(f"expected runtime: {EXPECTED_RUNTIME}")
        return None

    runs: list[dict] = []
    # A handful of short, bounded attempts to let the dispatch register on
    # GitHub's side -- not the "poll a slow job" pattern CLAUDE.md forbids
    # (this loop only waits for `gh run list` to notice a run that was just
    # created, seconds at most, and prints nothing between attempts), then
    # one real blocking wait via `gh run watch` below.
    for attempt in range(5):
        out = ac.run_cmd(
            ["gh", "run", "list", "--workflow", WORKFLOW, "--branch", cohort_branch, "--limit", "10", "--json", "databaseId,status,event,createdAt"],
            cwd=repo_root,
        )
        runs = json.loads(out or "[]")
        try:
            run = select_dispatch_run(runs, dispatched_at)
            break
        except ValueError:
            if attempt == 4:
                raise
            time.sleep(5)
    run_id = str(run["databaseId"])
    watch_run(repo_root, run_id)
    return run_id


def download_baseline_artifacts(repo_root: Path, run_id: str) -> tuple[Path, Path]:
    tmpdir = Path(tempfile.mkdtemp(prefix="rebless-"))
    ac.run_cmd(["gh", "run", "download", run_id, "-D", str(tmpdir)], cwd=repo_root)
    baseline_path = tmpdir / "real-specimen-mrz-baseline" / "real-specimen-mrz-baseline.json"
    report_path = tmpdir / "real-specimen-gate-report" / "real-specimen-gate-report.json"
    return baseline_path, report_path


# ==========================================================================
# Orchestration
# ==========================================================================


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--cohort-branch", required=True)
    parser.add_argument("--repo-root", type=Path, default=None)
    parser.add_argument("--worktree", type=Path, default=None)
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--confirm", action="store_true", help="required to dispatch/commit/push/gh pr ready/PATCH")
    parser.add_argument("--run-id", default=None, help="reuse an already-completed write-baseline run instead of dispatching")
    parser.add_argument("--skip-assert", action="store_true", help="skip dispatching the assert run (for a rerun after an earlier interrupted attempt)")
    args = parser.parse_args(argv)

    repo_root = (args.repo_root or Path(__file__).resolve().parent.parent).resolve()
    worktree = args.worktree or (repo_root.parent / "worktrees" / repo_root.name / args.cohort_branch)

    # Always runs, --dry-run or not -- see the module docstring.
    verify_worktree_on_branch(worktree, args.cohort_branch)
    print(f"worktree {worktree} is on branch {args.cohort_branch!r} -- OK")

    if args.dry_run:
        print("\n--dry-run: stopping here. Without it this run would:")
        if args.run_id:
            print(f"  - reuse already-completed run {args.run_id} (no dispatch)")
        else:
            print(f"  $ gh workflow run {WORKFLOW} --ref {args.cohort_branch} -f mode=write-baseline  (needs --confirm)")
        print(f"  $ gh run download <run-id> -D <tmpdir>")
        print(f"  - classify the diff against {worktree / BASELINE_REL_PATH}, write doc edits for identical/non-scored-delta")
        print(f"  $ bash scripts/check-headline-numbers.sh  (in the worktree)")
        print("  - (with --confirm) git add/commit/push, dispatch mode=assert, gh pr ready, PATCH the PR body")
        return 0

    if args.run_id:
        run_id = args.run_id
        print(f"reusing already-completed run {run_id} (--run-id given, skipping dispatch)")
    else:
        if not args.confirm:
            print("\nno --run-id given and --confirm not set: dispatching the write-baseline run is a")
            print("confirm-gated action (see the module docstring's 'Two independent safety layers').")
            print("Re-run with --confirm to dispatch, or pass --run-id to reuse an already-completed run.")
            return 0
        run_id = dispatch_and_wait(repo_root, args.cohort_branch, "write-baseline", dry_run=False)

    baseline_artifact, _report_artifact = download_baseline_artifacts(repo_root, run_id)
    new_baseline = json.loads(baseline_artifact.read_text(encoding="utf-8"))

    baseline_path = worktree / BASELINE_REL_PATH
    old_baseline = json.loads(baseline_path.read_text(encoding="utf-8"))
    old_flat, new_flat = flatten_baseline(old_baseline), flatten_baseline(new_baseline)
    hits = int(new_flat["tier1_hits"])

    cls, changed = classify_baseline_diff(old_baseline, new_baseline)
    print(f"\nbaseline diff classification: {cls}")
    print(format_diff_table(changed, new_flat))

    # Install the new baseline unconditionally: CI's freshly measured numbers
    # are always the new source of truth, whatever class the diff falls into
    # -- an analyst finishing a scored-delta entry still needs this file
    # updated to write prose against.
    baseline_path.write_text(json.dumps(new_baseline, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    touched = {BASELINE_REL_PATH}

    if cls == CLASS_SCORED:
        print("\nSCORED delta: tier1_hits or a scored miss bucket moved (or an unrecognized field did).")
        print("This needs prose from synthpass-analyst -- the benchmarks/README.md Weak-spot entry and")
        print("the README.md headline numbers -- before it can be committed. The new baseline has been")
        print(f"installed at {baseline_path} for the analyst to work from; nothing else was written.")
        return 2

    if cls == CLASS_NON_SCORED:
        readme_path = worktree / README_REL_PATH
        readme_path.write_text(
            rewrite_readme_gap_and_corpus_rate(readme_path.read_text(encoding="utf-8"), hits, old_flat, new_flat),
            encoding="utf-8",
        )
        touched.add(README_REL_PATH)

        bench_readme_path = worktree / BENCH_README_REL_PATH
        bench_text = rewrite_benchmarks_readme_live_block(bench_readme_path.read_text(encoding="utf-8"), hits, old_flat, new_flat)
        n_specimens = int(new_flat["documents"]) - int(old_flat["documents"])
        entry = build_weakspot_entry(args.cohort_branch, n_specimens, changed, run_id, old_flat, new_flat)
        bench_readme_path.write_text(bench_text.rstrip("\n") + "\n" + entry, encoding="utf-8")
        touched.add(BENCH_README_REL_PATH)
        print(f"wrote mechanical doc edits: {README_REL_PATH}, {BENCH_README_REL_PATH}")

    bash_exe = ac.find_git_bash()
    headline_out = ac.run_bash_script(bash_exe, "scripts/check-headline-numbers.sh", worktree, dry_run=False)
    print(headline_out)

    if not args.confirm:
        print("\n--confirm not given: stopping after diff classification + doc edits.")
        print(f"Would git add {sorted(touched)}, commit, push, dispatch mode=assert, wait, gh pr ready,")
        print("and PATCH the PR body. Re-run with --confirm once the rewritten prose has been reviewed.")
        return 0

    commit_message = build_rebless_commit_message(args.cohort_branch, cls, changed, run_id, new_flat)
    print("--- commit message ---")
    print(commit_message)
    print("--- end ---")
    ac.run_cmd(["git", "-C", str(worktree), "add"] + sorted(touched), cwd=repo_root)
    ac.run_cmd(["git", "-C", str(worktree), "commit", "-m", commit_message], cwd=repo_root)
    ac.run_cmd(["git", "-C", str(worktree), "push"], cwd=repo_root)

    if args.skip_assert:
        print("--skip-assert: not dispatching the assert run.")
    else:
        assert_run_id = dispatch_and_wait(repo_root, args.cohort_branch, "assert", dry_run=False)
        print(f"assert run {assert_run_id} passed.")

    pr_number = ac.gh_pr_number_for_branch(repo_root, args.cohort_branch)
    if pr_number is None:
        print(f"WARNING: no open PR found for branch {args.cohort_branch!r}; skipping `gh pr ready` and the PATCH.")
        return 0

    ac.run_cmd(["gh", "pr", "ready", str(pr_number)], cwd=repo_root)
    owner, name = ac.gh_repo_owner_and_name(repo_root)
    title, body = gh_pr_title_and_body(repo_root, pr_number)
    paragraph = build_rebless_result_paragraph(cls, changed, run_id, new_flat)
    body_file = worktree / ".pr-body-rebless.md"
    body_file.write_text(f"{body.rstrip()}\n\n{paragraph}", encoding="utf-8")
    ac.gh_patch_pr_title_and_body(repo_root, owner, name, pr_number, title, body_file, dry_run=False)
    if body_file.is_file():
        body_file.unlink()

    print(f"\nPR #{pr_number}: marked ready for review, body updated with the re-bless result.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
