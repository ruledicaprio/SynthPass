#!/usr/bin/env python3
"""
apply_cohort.py

Automates the mechanical, error-prone parts of the specimen-acquisition loop's
"P-DATA-PR" procedure (`~/.claude/plans/specimen-acquisition-loop.md` §6.6,
`knowledge/SPECIMEN_SOURCES.md`) once a cohort packet has a Verdict on every
row: worktree setup, the cross-PR duplicate guard, image placement, manifest
regeneration and `origin` patching, the local-track manifest, the mechanical
checks, a changelog-fragment draft, and -- guarded hard -- the commit/push/PR
and the `samples-data` push.

This tool never decides a licence class, never writes prose it wasn't given,
and never silently lets a `samples-data` push delete another PR's images (see
"The single most important guardrail" below). The per-image `origin.notes`
field in `samples/corpus.jsonl` is always written as a labelled DRAFT for a
human to review, never committed unreviewed. Everything genuinely mechanical
-- HIT / MISS (checksum_failed) / no-coverage-claim -- is derived from the
regenerated manifest and reported plainly instead.

`knowledge/CORPUS_COVERAGE.md`'s Docs/Note columns are the one place this
tool writes real prose, not a draft, and only for a row whose target path is
under `covers/`: a cover is never a coverage claim (ADR-0012), so there is no
Status decision for a human to make, and the Docs cell + Note text follow a
fixed template (see "CORPUS_COVERAGE.md cover-row templates" below). Every
other row -- one with a data page -- still gets a labelled DRAFT coverage
note for a human to fold in by hand, because a Status move there is a human
call this tool does not make.

Standard library only, matching tools/screen_candidates.py and
tools/apply_verdicts.py. The one non-stdlib import is its sibling
`tools/scout_cycle.py`, for the STATE.md helpers a covers-only cohort needs
(`cover_only_codes_line`, `edit_state`, `add_codes_line`) -- that module is
standard library only too and has no import-time side effects.

## Packet format

Reads the Markdown table `tools/screen_candidates.py::write_packet` produces
(and `tools/apply_verdicts.py` fills the last column of):

    | # | Code | Doc/TD | Series | Host | Licence evidence | Specimen signal |
    | MRZ check | Proposed destination | Proposed licence | Local path | Note | Verdict |

Every row's Verdict cell must already be `public`, `local` or `drop` -- a
blank cell anywhere fails loudly rather than silently processing a subset.

## --filenames

A small JSON object mapping each packet row (by the sha12 id in its Local
path cell, e.g. `"5784cf79a00b"`, or by `"#3"` when a row carries no id) to
its target path *relative to `samples/`*:

    {
      "5784cf79a00b": "passports/Foo_Passport_Specimen_P0_FOO_2020_mrz.jpg",
      "#3": "local/id_cards/Bar_ID_Specimen_2021_back_mrz.jpg"
    }

A `local`-verdicted row's path must start with `local/`; a `public`-verdicted
row's must not. The tool validates the structural rules `samples/README.md`'s
naming convention states (extension, `DocType` token matching the target
subdirectory) but never invents a filename -- that still needs a human's
judgement about legacy state codes, `XXXX` for an unknown year, and so on.

## The single most important guardrail

Before ANY real `sync-samples.ps1 -Push`, this tool always runs `-Push
-DryRun` first, parses its `renamed=/added=/deleted=/modified=` summary, and
refuses to proceed to a real push if `deleted > 0` unless the caller passes
`--allow-deletions` -- and even then, it prints the exact file list and
requires `--confirm` again. `sync-samples.ps1 -Push` mirrors the whole local
`samples/` tree; a worktree that still has another open PR's not-yet-merged
images excluded (the cross-PR duplicate guard, below) would otherwise mirror
that exclusion onto `samples-data` as a real deletion. This is exactly the
near-miss the task this tool automates hit by hand once already.

## Usage

    python tools/apply_cohort.py \\
        --packet work/scouting/c01/packet-c01.md \\
        --filenames work/scouting/c01/filenames-c01.json \\
        --cohort-branch cohort-c01 \\
        [--reuse-existing] [--dry-run] \\
        [--exclude-branch cohort-c02 ...] \\
        [--allow-deletions] [--confirm] \\
        [--commit-message-file path]

Two independent safety layers, not one:

- `--dry-run` performs every read-only step -- packet parsing and
  validation, structural filename checks, `gh pr list` / `git show` for the
  cross-PR duplicate guard -- and then stops, always, before any mutation:
  no worktree is created, no file is copied, no binary is built, whether or
  not `--reuse-existing` was also given. That is deliberately the whole of
  what `--dry-run` means here, with no case-by-case exception to reason
  about -- the report from this layer is packet/filenames problems plus
  exactly which filenames the duplicate guard would exclude and why.
- Regardless of `--dry-run`, the truly consequential steps -- `git commit`,
  `git push`, `gh pr create`/`gh api PATCH`, and any REAL `sync-samples.ps1
  -Push` -- never run without `--confirm`. Running without `--dry-run` gets
  you as far as a worktree with the manifest regenerated, the checks
  passed, and a draft commit message and changelog fragment printed;
  `--confirm` is the second, separate gate for the git/GitHub/samples-data
  writes themselves.

Out of scope, per the plan's batch-size rule (§7): dispatching the
real-specimen re-bless workflow, waiting for it, downloading or committing
the baseline, or merging the PR. This tool's job ends after the
manifest/coverage/changelog commit and the `samples-data` push.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from datetime import date
from pathlib import Path
from urllib.parse import urlparse

sys.path.insert(0, str(Path(__file__).resolve().parent))

import scout_cycle as scy  # noqa: E402 -- STATE.md lives behind these helpers; no import-time side effects

# --------------------------------------------------------------------------
# Constants mirrored from the Rust side of the manifest contract. Keep these
# in sync with crates/synthpass-bench/tests/corpus_manifest.rs -- that file,
# not this one, is what CI actually enforces; these copies only let this tool
# fail fast, in Python, before a slow `cargo test` run would catch the same
# mistake.
# --------------------------------------------------------------------------

ORIGIN_FOUND_BY = ("unrecorded", "human", "commons-fetcher", "dsh", "firecrawl")
ORIGIN_LICENCES = (
    "unrecorded",
    "public-domain",
    "cc-by",
    "cc-by-sa",
    "gov-published",
    "none-stated",
)

# A `public` verdict paired with one of these licence classes is the doubtful
# case SPECIMEN_SOURCES.md and the plan's H15 both call a human judgement,
# not a rule this tool enforces or silently papers over.
DOUBTFUL_LICENCES = ("none-stated", "unrecorded", "")

CORPUS_DIRS = ("passports", "id_cards", "driving_licenses", "misc", "ocr_fixtures", "covers")
IMAGE_EXTENSIONS = ("jpg", "jpeg", "png", "webp", "gif")

# Specimen images are not tracked on main: every one of these directories is
# ignored in `.gitignore` because the images live on the orphan `samples-data`
# branch (`scripts/sync-samples.ps1`), and `samples/local/` -- the local-only
# track, its manifest included -- is ignored outright and deliberately never
# committed. Staging one of those paths does not merely do nothing: it makes
# the whole `git add` fail ("paths are ignored by one of your .gitignore
# files"), which is how a cohort c14 run died at the commit step with every
# other step already done. `samples/ocr_fixtures/` is absent on purpose -- its
# JSON/Markdown ground truth (and four force-added images) IS tracked.
UNTRACKED_SAMPLE_DIRS = ("passports", "id_cards", "driving_licenses", "misc", "covers", "local")

VALID_VERDICTS = ("public", "local", "drop")

# The 13 columns `screen_candidates.py::write_packet` emits, in order.
PACKET_COLUMNS = (
    "row_number",
    "code",
    "doc_td",
    "series",
    "host",
    "licence_evidence",
    "specimen_signal",
    "mrz_check",
    "proposed_destination",
    "proposed_licence",
    "local_path",
    "note",
    "verdict",
)


# ==========================================================================
# Packet parsing
# ==========================================================================


def split_table_row(line: str) -> list[str]:
    """Splits one Markdown table row on unescaped `|`, unescaping `\\|` back
    to a literal pipe (the inverse of `screen_candidates.py::_escape_md`).
    Returns `[]` for a line that isn't a table row at all."""
    stripped = line.strip()
    if not (stripped.startswith("|") and stripped.endswith("|") and len(stripped) >= 2):
        return []
    body = stripped[1:-1]
    cells: list[str] = []
    current: list[str] = []
    i = 0
    while i < len(body):
        ch = body[i]
        if ch == "\\" and i + 1 < len(body) and body[i + 1] == "|":
            current.append("|")
            i += 2
            continue
        if ch == "|":
            cells.append("".join(current).strip())
            current = []
            i += 1
            continue
        current.append(ch)
        i += 1
    cells.append("".join(current).strip())
    return cells


def _is_separator_row(cells: list[str]) -> bool:
    return bool(cells) and all(re.fullmatch(r":?-+:?", c) for c in cells)


def parse_packet(text: str) -> list[dict]:
    """Parses the packet's Markdown table into a list of row dicts keyed by
    `PACKET_COLUMNS`. Ignores every line before the header and after the
    table (the generated prose paragraph, any trailing notes)."""
    lines = text.splitlines()
    header_idx = None
    for i, line in enumerate(lines):
        cells = split_table_row(line)
        if cells and cells[0].strip().lstrip("*").strip().lower() == "#":
            header_idx = i
            break
    if header_idx is None:
        raise ValueError("no Markdown table with a '#' header column found in the packet")

    body_start = header_idx + 1
    if body_start < len(lines) and _is_separator_row(split_table_row(lines[body_start])):
        body_start += 1

    rows: list[dict] = []
    for line in lines[body_start:]:
        cells = split_table_row(line)
        if not cells:
            continue
        row = {name: (cells[idx] if idx < len(cells) else "") for idx, name in enumerate(PACKET_COLUMNS)}
        row["raw_line"] = line
        rows.append(row)
    return rows


def extract_row_id(local_path_cell: str) -> str | None:
    """The sha12 candidate id from a row's Local path cell, a Markdown link
    `[<sha12>.<ext>](<path>)` -- the same identity `apply_verdicts.py` keys
    on."""
    m = re.search(r"\[([0-9a-f]{12})\.[A-Za-z0-9]+\]", local_path_cell)
    if m:
        return m.group(1)
    m = re.search(r"\b([0-9a-f]{12})\b", local_path_cell)
    return m.group(1) if m else None


def row_key(row: dict) -> str:
    """The preferred `--filenames` lookup key for a packet row: its sha12 id
    when the Local path cell carries one, else `"#<row number>"`."""
    row_id = extract_row_id(row.get("local_path", ""))
    if row_id:
        return row_id
    return f"#{row.get('row_number', '').strip()}"


def validate_all_verdicts_present(rows: list[dict]) -> None:
    """Fails loudly on a partially-reviewed packet, per the task's explicit
    requirement: a blank Verdict cell anywhere must stop the run rather than
    silently processing a subset."""
    if not rows:
        raise ValueError("packet has no data rows to process")
    blank = [r.get("row_number", "?") for r in rows if not r.get("verdict", "").strip()]
    if blank:
        raise ValueError(
            f"packet has {len(blank)} row(s) with a blank Verdict cell (rows {blank}) -- "
            "a partially-reviewed packet cannot be processed"
        )
    bad = [
        (r.get("row_number", "?"), r.get("verdict"))
        for r in rows
        if r.get("verdict", "").strip().lower() not in VALID_VERDICTS
    ]
    if bad:
        raise ValueError(
            f"packet has row(s) with an unrecognized Verdict (must be one of {VALID_VERDICTS}): {bad}"
        )


def rows_with_verdict(rows: list[dict], verdict: str) -> list[dict]:
    return [r for r in rows if r.get("verdict", "").strip().lower() == verdict]


# ==========================================================================
# --filenames
# ==========================================================================


def load_filenames_map(path: Path) -> dict[str, str]:
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"{path}: must be a JSON object of {{row id -> target path}}")
    return {str(k): str(v) for k, v in data.items()}


def validate_target_filename(verdict: str, target_rel_path: str) -> list[str]:
    """Structural checks against `samples/README.md`'s naming convention --
    the right extension, and the `DocType` token agreeing with the target
    subdirectory (mirroring
    `corpus_manifest.rs::the_directory_agrees_with_the_document_type_in_the_name`).
    `covers/` is exempt from the DocType-vs-directory check, the same way
    `corpus_manifest.rs`'s Rust mirror exempts it: a cover-only image can be
    of any document type (ADR-0012's amendment), so the directory alone is
    the label, not the `DocType` token in the name.
    Never invents or corrects a filename; returns a list of error strings,
    empty when the structural checks pass."""
    errors: list[str] = []
    path = target_rel_path.replace("\\", "/").strip("/")
    parts = path.split("/") if path else []

    if verdict == "local":
        if not parts or parts[0] != "local":
            errors.append(f"{target_rel_path!r}: a 'local' verdict's target path must start with 'local/'")
        else:
            parts = parts[1:]
    elif verdict == "public":
        if parts and parts[0] == "local":
            errors.append(f"{target_rel_path!r}: a 'public' verdict's target path must not start with 'local/'")
    else:
        errors.append(f"unrecognized verdict {verdict!r} (must be 'public' or 'local')")
        return errors

    if len(parts) < 2:
        errors.append(f"{target_rel_path!r}: expected '<subdir>/<basename>' under samples/")
        return errors

    subdir, basename = parts[0], parts[-1]
    if subdir not in CORPUS_DIRS:
        errors.append(f"{target_rel_path!r}: unrecognized target subdirectory {subdir!r}, expected one of {CORPUS_DIRS}")

    ext = basename.rsplit(".", 1)[-1].lower() if "." in basename else ""
    if ext not in IMAGE_EXTENSIONS:
        errors.append(f"{basename!r}: extension {ext!r} is not one of {IMAGE_EXTENSIONS}")

    stem = basename.rsplit(".", 1)[0] if "." in basename else basename
    tokens = [t.lower() for t in stem.split("_")]
    if subdir == "passports" and "passport" not in tokens:
        errors.append(f"{basename!r}: filed under passports/ but the name has no 'Passport' token")
    if subdir == "id_cards" and "id" not in tokens:
        errors.append(f"{basename!r}: filed under id_cards/ but the name has no 'ID' token")

    return errors


# ==========================================================================
# What may be staged (see UNTRACKED_SAMPLE_DIRS above)
# ==========================================================================


def is_untracked_sample_path(path: str) -> bool:
    """True for a path under one of the `samples/` directories that is not
    tracked on main -- every placed specimen image, and anything at all under
    `samples/local/`. `samples/corpus.jsonl`, `knowledge/...` and
    `changelog.d/...` are tracked text and return False."""
    parts = (path or "").replace("\\", "/").strip("/").split("/")
    return len(parts) >= 2 and parts[0] == "samples" and parts[1] in UNTRACKED_SAMPLE_DIRS


def stageable_paths(paths) -> tuple[list[str], list[str]]:
    """Splits the paths a run touched into `(stageable, skipped)`, sorted.
    Only the tracked text products of a cohort -- `samples/corpus.jsonl`,
    `knowledge/CORPUS_COVERAGE.md`, the changelog fragment -- are ever staged;
    the images themselves reach `samples-data` through
    `scripts/sync-samples.ps1`, never through this commit."""
    stageable, skipped = [], []
    for path in paths:
        (skipped if is_untracked_sample_path(path) else stageable).append(path)
    return sorted(stageable), sorted(skipped)


# ==========================================================================
# Cross-PR duplicate guard (fixes bug #1: manifest duplication across two
# open cohort PRs)
# ==========================================================================


def load_corpus_filenames(jsonl_text: str) -> set[str]:
    names: set[str] = set()
    for line in jsonl_text.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            row = json.loads(line)
        except json.JSONDecodeError:
            continue
        name = row.get("filename")
        if name:
            names.add(name)
    return names


def compute_cross_pr_exclusions(
    main_filenames: set[str], other_branch_filenames: dict[str, set[str]]
) -> dict[str, str]:
    """For every filename recorded on another open cohort branch's
    `samples/corpus.jsonl` but not yet on `origin/main`, returns
    `{filename: reason}`. These filenames must be excluded from this
    worktree's local `samples/` tree before `corpus_manifest` runs, or its
    directory-wide scan silently folds the other PR's not-yet-merged rows
    into this branch's manifest too (bug #1)."""
    exclusions: dict[str, str] = {}
    for branch, filenames in sorted(other_branch_filenames.items()):
        for name in sorted(filenames):
            if name in main_filenames or name in exclusions:
                continue
            exclusions[name] = f"pending in open PR branch {branch!r} (not yet on origin/main)"
    return exclusions


def discover_open_cohort_branches(exclude_self: str, list_open_prs) -> list[str]:
    """Open PR branches whose name starts with `cohort-`, excluding the
    branch this run is itself using.

    Uses `gh pr list --state open --json headRefName` (via the injected
    `list_open_prs` callable) rather than enumerating local branches or
    worktrees: what the duplicate guard actually needs to know is "is there
    an open PR carrying pending images", which is a GitHub-side fact. A
    same-named local branch or worktree can exist without an open PR (not
    yet pushed, or its PR already closed/merged) and an open PR's branch can
    exist with no local worktree at all on this machine right now -- `gh pr
    list` is the one source that reflects the fact this guard cares about
    directly, instead of inferring it from local state that can be stale.

    Known edge case (see the handoff report): a cohort branch that has been
    created and pushed to but has no PR opened yet is invisible to this
    check. P-DATA-PR's own procedure opens the PR (as a draft) immediately
    after the first commit, which keeps that window short, but it is not
    zero -- `--exclude-branch` is the explicit escape hatch for that case.
    """
    prs = list_open_prs()
    branches = [p.get("headRefName", "") for p in prs if p.get("headRefName", "").startswith("cohort-")]
    return sorted({b for b in branches if b and b != exclude_self})


# ==========================================================================
# HIT / MISS / no-coverage-claim determination (fully mechanical)
# ==========================================================================


def determine_coverage_status(manifest_row: dict) -> str:
    """Derives a regenerated manifest row's coverage status exactly the way
    it has been done by hand throughout the loop:

    - `mrz.observed.checksums_valid` is `True`             -> `"HIT"`
    - `mrz.present` is `True` but checksums are not valid   -> `"MISS (checksum_failed)"`
    - `mrz.present` is `False`                              -> `"no-coverage-claim"`

    This never edits `CORPUS_COVERAGE.md` prose -- it only classifies, so the
    classification is fast to eyeball against the draft note a human still
    writes by hand."""
    mrz = manifest_row.get("mrz") or {}
    observed = mrz.get("observed") or {}
    if observed.get("checksums_valid") is True:
        return "HIT"
    if mrz.get("present") is True:
        return "MISS (checksum_failed)"
    return "no-coverage-claim"


# ==========================================================================
# Licence-vs-verdict agreement (warn, never enforce)
# ==========================================================================


def check_licence_verdict_agreement(verdict: str, proposed_licence: str) -> str | None:
    """A `public` verdict paired with a doubtful licence class
    (`none-stated`/`unrecorded`) is a human call every cycle so far, not a
    rule this tool enforces. Returns a loud warning string, or `None` when
    there's nothing to flag -- it never blocks and never silently
    proceeds."""
    if verdict == "public" and (proposed_licence or "").strip().lower() in DOUBTFUL_LICENCES:
        return (
            f"WARNING: verdict is 'public' but the proposed licence is {proposed_licence!r}, which "
            "SPECIMEN_SOURCES.md treats as doubtful ('none-stated' normally proposes 'local'). "
            "Confirm this was a deliberate human call before origin.licence is written for this row."
        )
    return None


# ==========================================================================
# origin / local-manifest row construction
# ==========================================================================


def build_origin_patch(found_by: str, url: str | None, page: str | None, licence: str, fetched: str) -> dict:
    if found_by not in ORIGIN_FOUND_BY:
        raise ValueError(f"origin.found_by {found_by!r} is not in the closed set {ORIGIN_FOUND_BY}")
    if licence not in ORIGIN_LICENCES:
        raise ValueError(f"origin.licence {licence!r} is not in the closed set {ORIGIN_LICENCES}")
    return {"found_by": found_by, "url": url, "page": page, "licence": licence, "fetched": fetched}


def sidecar_path_for(packet_path: Path, cohort_branch: str) -> Path:
    """The `screened-cNN.jsonl` that belongs to a packet. Derived from the
    packet's own name (`packet-c12.md` -> `screened-c12.jsonl`), because an
    accumulating cohort branch (`cohort-c10`) folds in later cycles' packets
    and the branch name then says nothing about which cycle a row came from
    -- deriving it from the branch left every c12 origin url/page null on
    2026-09-15. The branch-derived name stays as the fallback for packets
    not named `packet-<cycle>`."""
    stem = packet_path.stem
    if stem.startswith("packet-"):
        return packet_path.parent / f"screened-{stem.removeprefix('packet-')}.jsonl"
    return packet_path.parent / f"screened-{cohort_branch.removeprefix('cohort-')}.jsonl"


def load_screened_sidecar(jsonl_text: str) -> dict[str, dict]:
    """Keys `screened-cNN.jsonl` records by the sha12 stem of their staged
    file, matching `row_key`'s id -- this is where a raw fetched `url`/`page`
    lives, since the packet's own columns don't cleanly carry one."""
    by_id: dict[str, dict] = {}
    for line in jsonl_text.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            rec = json.loads(line)
        except json.JSONDecodeError:
            continue
        staged = rec.get("staged_path") or ""
        stem = Path(staged).stem
        if re.fullmatch(r"[0-9a-f]{12}", stem or ""):
            by_id[stem] = rec
    return by_id


def build_local_manifest_row(filename: str, dir_: str, sha256: str, origin: dict, notes_draft: str) -> dict:
    return {
        "filename": filename,
        "dir": dir_,
        "sha256": sha256,
        "origin": origin,
        "notes": notes_draft,
    }


# ==========================================================================
# CORPUS_COVERAGE.md draft note
# ==========================================================================


def draft_coverage_note(filename: str, status: str, licence: str, verdict: str, today: str | None = None) -> str:
    """A DRAFT prose fragment in the shape of an existing `CORPUS_COVERAGE.md`
    Note cell, always prefixed `DRAFT -- review before commit`. Never
    overwrites the file; the caller decides which row it belongs to and
    whether it changes that row's Status column."""
    today = today or date.today().isoformat()
    dest = "public" if verdict == "public" else "local-only (not pushed, not in CI)"
    return (
        f"DRAFT -- review before commit: added {today} ({dest}, `{filename}`, {licence}); "
        f"regenerated manifest reports {status}. <<< fill in: does this change the row's Status "
        "column, and any redaction/attribution wording >>>"
    )


# ==========================================================================
# CORPUS_COVERAGE.md cover-row templates (A3, T11)
# ==========================================================================
#
# A cover is never a coverage claim (ADR-0012 and its 2026-09-15 amendment) --
# there is no Status decision for a human to make, so this is the one place
# this tool writes real `knowledge/CORPUS_COVERAGE.md` prose instead of a
# labelled DRAFT. The templates below are the shape used by hand on main for
# cohort c12 (the PAN/GTM/HND/LKA/PNG rows) and cohort c13.

COVER_NOTE_SINGLE = (
    "Cover only in `samples/covers/` ({cycle}, {date}, {licence}); no data page, never a "
    "coverage claim (ADR-0012)"
)
COVER_NOTE_MULTI = (
    "Covers only in `samples/covers/` ({cycle}, {date}, {licence}): {series}; no data page, "
    "never a coverage claim (ADR-0012)"
)

# origin.licence's closed set (ORIGIN_LICENCES above) collapsed to the short
# label this prose uses. Note that "cc0" is not a distinct origin.licence
# value -- it buckets to "public-domain" -- so a Commons CC0 file and a
# Commons public-domain file both read "public domain" here. That is a real
# simplification versus c12's fully hand-written prose, which knew the exact
# Commons licence tag from the file page; this tool only has the closed-set
# class the packet's proposed licence was recorded as.
LICENCE_SUMMARY_LABELS = {
    "cc-by-sa": "CC-BY-SA",
    "cc-by": "CC-BY",
    "public-domain": "public domain",
    "gov-published": "gov-published",
}
COMMONS_HOSTS = {"commons.wikimedia.org", "upload.wikimedia.org"}


def licence_summary(host: str, licence: str) -> str:
    """The short licence phrase a cover-row Note uses, e.g. `Commons
    CC-BY-SA` or `example.gov, none-stated licence`."""
    host = (host or "").strip()
    licence = (licence or "").strip().lower()
    if licence in ("none-stated", "unrecorded", ""):
        return f"{host}, none-stated licence" if host else "none-stated licence"
    label = LICENCE_SUMMARY_LABELS.get(licence, licence)
    if host in COMMONS_HOSTS or host.endswith(".wikimedia.org"):
        return f"Commons {label}"
    return f"{host}, {label}" if host else label


def join_series(labels: list[str]) -> str:
    """Oxford-joins `"<year> series"`-shaped labels: one label unchanged,
    two joined with `"and"`, three or more comma-separated with `"and"`
    before the last."""
    if not labels:
        return ""
    if len(labels) == 1:
        return labels[0]
    if len(labels) == 2:
        return f"{labels[0]} and {labels[1]}"
    return ", ".join(labels[:-1]) + f" and {labels[-1]}"


def cover_doctype_label(basename: str) -> str:
    """`Passport` / `ID` from the filename's DocType token -- the same
    tokens `validate_target_filename` checks for `passports/`/`id_cards/`;
    `covers/` is exempt from that check, so a cover can legitimately carry
    either token."""
    stem = basename.rsplit(".", 1)[0] if "." in basename else basename
    tokens = [t.lower() for t in stem.split("_")]
    if "passport" in tokens:
        return "Passport"
    if "id" in tokens:
        return "ID"
    return "Document"


def is_cover_target(target_rel_path: str) -> bool:
    """True when a `--filenames` target path places the image under
    `covers/` (public track) or `local/covers/` (local track)."""
    parts = target_rel_path.replace("\\", "/").strip("/").split("/")
    if parts and parts[0] == "local":
        parts = parts[1:]
    return bool(parts) and parts[0] == "covers"


def build_cover_coverage_update(code: str, entries: list[dict], cycle_id: str, today: str) -> tuple[str, str]:
    """`entries` is `[{"basename":..., "year_label":..., "host":..., "licence":...}, ...]`
    for one code's cover row(s) in this cohort. Returns `(docs_cell, note_text)`
    in the shape of the PAN/GTM/HND/LKA/PNG rows on main."""
    doctype_labels = sorted({cover_doctype_label(e["basename"]) for e in entries})
    doctype_label = "/".join(doctype_labels)
    licence_labels: list[str] = []
    for e in entries:
        summary = licence_summary(e["host"], e["licence"])
        if summary not in licence_labels:
            licence_labels.append(summary)
    licence_text = " / ".join(licence_labels)
    if len(entries) == 1:
        docs = f"{doctype_label} (cover)"
        note = COVER_NOTE_SINGLE.format(cycle=cycle_id, date=today, licence=licence_text)
    else:
        docs = f"{doctype_label} (cover, {len(entries)} series)"
        series = join_series([f"{e['year_label']} series" for e in entries])
        note = COVER_NOTE_MULTI.format(cycle=cycle_id, date=today, licence=licence_text, series=series)
    return docs, note


def split_coverage_row(line: str) -> list[str] | None:
    """Splits one `knowledge/CORPUS_COVERAGE.md` table row on `|`. Unlike
    `split_table_row` (the packet parser), this table has no escaped-pipe
    convention -- it is hand-maintained prose, not machine-generated -- so a
    plain split is the faithful round trip. Returns `None` for a line that
    isn't a table row at all."""
    stripped = line.strip()
    if not (stripped.startswith("|") and stripped.endswith("|") and len(stripped) >= 2):
        return None
    return [c.strip() for c in stripped[1:-1].split("|")]


def update_corpus_coverage_for_cover(text: str, code: str, docs_addendum: str, note_text: str) -> str:
    """Rewrites `knowledge/CORPUS_COVERAGE.md`'s Full-table row for `code`.
    A cover never moves Status (ADR-0012), so that column is untouched. When
    the row's Docs cell already carries a value other than `--` (an existing
    data page), `; cover` is appended to it instead and the Note is left
    alone, rather than overwriting prose a human wrote. Raises `ValueError`
    when no row for `code` is found -- silently doing nothing would leave
    the cohort's cover undocumented."""
    lines = text.split("\n")
    updated = False
    for i, line in enumerate(lines):
        cells = split_coverage_row(line)
        if cells and len(cells) == 5 and cells[0] == code:
            docs_cell = cells[2]
            if docs_cell and docs_cell != "--":
                new_docs, new_note = f"{docs_cell}; cover", cells[4]
            else:
                new_docs, new_note = docs_addendum, note_text
            lines[i] = f"| {cells[0]} | {cells[1]} | {new_docs} | {cells[3]} | {new_note} |"
            updated = True
            break
    if not updated:
        raise ValueError(f"knowledge/CORPUS_COVERAGE.md: no Full-table row found for code {code!r}")
    return "\n".join(lines)


# ==========================================================================
# Changelog fragment draft
# ==========================================================================


def build_changelog_fragment(cohort_branch: str, public_rows: list[tuple[str, str]], corpus_count_before: int) -> str:
    """`public_rows` is `[(filename, coverage_status), ...]`. Written as a
    valid changelog bullet (so `scripts/check-changelog.sh`'s grammar check
    passes) but visibly marked as a draft, since the prose reasoning in a
    changelog entry has consistently needed a human's judgement in this
    loop, same as `origin.notes` and the coverage Note column."""
    after = corpus_count_before + len(public_rows)
    lines = [
        f"- **DRAFT -- review before commit.** {len(public_rows)} specimen(s) added to the "
        f"real-specimen corpus (`samples/corpus.jsonl` {corpus_count_before} -> {after} rows) "
        f"from the specimen-acquisition loop's `{cohort_branch}` cohort:"
    ]
    for filename, status in public_rows:
        lines.append(f"  - `{filename}` -- {status}")
    lines.append(
        "  <<< fill in: which codes move Status in `knowledge/CORPUS_COVERAGE.md`, and the "
        "licence for each >>>"
    )
    return "\n".join(lines) + "\n"


def code_to_country_name(coverage_text: str) -> dict[str, str]:
    """code -> Country/Entity name, parsed from
    `knowledge/CORPUS_COVERAGE.md`'s Full table -- so a cover-only changelog
    fragment can name codes by their country without inventing text this
    tool wasn't given."""
    names: dict[str, str] = {}
    for line in coverage_text.split("\n"):
        cells = split_coverage_row(line)
        if cells and len(cells) == 5 and re.fullmatch(r"[A-Z0-9<]{1,4}", cells[0] or ""):
            names[cells[0]] = cells[1]
    return names


def licence_count_sentence(licences: list[str]) -> str:
    """One sentence for a cover-only changelog fragment: how many cover rows
    used each `origin.licence` class, and how many were kept public despite
    a none-stated licence. `"cc0"` is not a distinct `origin.licence` value
    (see `LICENCE_SUMMARY_LABELS`'s docstring), so it is not distinguishable
    from `"public-domain"` here."""
    counts: dict[str, int] = {}
    for licence in licences:
        counts[licence] = counts.get(licence, 0) + 1
    order = [
        ("cc-by-sa", "CC-BY-SA"),
        ("cc-by", "CC-BY"),
        ("public-domain", "public-domain"),
        ("gov-published", "gov-published"),
    ]
    parts = [f"{counts[key]} {label}" for key, label in order if counts.get(key)]
    none_stated = counts.get("none-stated", 0) + counts.get("unrecorded", 0)
    if parts and none_stated:
        return (
            f"{', '.join(parts)} from Wikimedia Commons or an official host under their stated "
            f"licence, {none_stated} kept public on a none-stated licence."
        )
    if parts:
        return f"{', '.join(parts)} from Wikimedia Commons or an official host under their stated licence."
    return f"{none_stated} kept public on a none-stated licence."


def build_cover_only_changelog_fragment(
    cohort_branch: str,
    cover_entries_by_code: dict[str, list[dict]],
    licences: list[str],
    corpus_count_before: int,
    n_images: int,
    country_names: dict[str, str],
) -> str:
    """The real (non-draft) fragment for a cohort whose every placed row is a
    cover (A4). Follows the template the task specifies, not a verbatim
    reproduction of an existing hand-written fragment: the headline image
    count and the trailing "N codes' Docs column" count use different
    numbers when one code carries more than one cover (as on main's c12
    fragment: six covers, five codes), a distinction this preserves rather
    than collapsing to one count."""
    cycle = cohort_branch.removeprefix("cohort-")
    codes = sorted(cover_entries_by_code)
    n_codes = len(codes)
    country_parts = []
    for code in codes:
        name = country_names.get(code, code)
        entries = cover_entries_by_code[code]
        if len(entries) > 1:
            years = ", ".join(e["year_label"] for e in entries)
            country_parts.append(f"{name} ({years})")
        else:
            country_parts.append(name)
    country_list = join_series(country_parts)
    doctype_labels = sorted(
        {cover_doctype_label(e["basename"]) for entries in cover_entries_by_code.values() for e in entries}
    )
    docs_phrase = " / ".join(f"{d} (cover)" for d in doctype_labels)
    after = corpus_count_before + n_images
    licence_sentence = licence_count_sentence(licences)
    image_word = "cover" if n_images == 1 else "covers"
    codes_possessive = "code's" if n_codes == 1 else "codes'"
    return (
        f"- **Cohort {cycle}: {n_images} passport {image_word} in `samples/covers/`** -- "
        f"{country_list} (`samples/corpus.jsonl` {corpus_count_before} -> {after} rows). "
        f"{licence_sentence} Covers carry no MRZ and never enter the scored denominator "
        "(ADR-0012 as amended), so no `CORPUS_COVERAGE.md` status moves; the "
        f"{n_codes} {codes_possessive} Docs column now reads `{docs_phrase}`.\n"
    )


# ==========================================================================
# sync-samples.ps1 -Push -DryRun parsing + the samples-data push guardrail
# ==========================================================================


def parse_sync_samples_dryrun(output: str) -> dict:
    """Parses the `renamed=N added=N deleted=N modified=N` summary line
    `scripts/sync-samples.ps1 -Push -DryRun` prints. Raises rather than
    guessing when that line is missing -- an unparseable DryRun output must
    never be treated as "nothing to worry about"."""
    m = re.search(r"renamed=(\d+)\s+added=(\d+)\s+deleted=(\d+)\s+modified=(\d+)", output)
    if m:
        renamed, added, deleted, modified = (int(x) for x in m.groups())
        return {"renamed": renamed, "added": added, "deleted": deleted, "modified": modified, "no_changes": False}
    if "No changes" in output:
        return {"renamed": 0, "added": 0, "deleted": 0, "modified": 0, "no_changes": True}
    raise ValueError(
        "could not find a 'renamed=/added=/deleted=/modified=' summary line in "
        "sync-samples.ps1 -Push -DryRun output -- refusing to guess whether a push is safe"
    )


def extract_deleted_paths(name_status_output: str) -> list[str]:
    """From `git diff --cached -M --name-status`-style output, the paths
    that would be deleted (`^D<TAB>path`)."""
    out = []
    for line in name_status_output.splitlines():
        if line.startswith("D\t"):
            out.append(line.split("\t", 1)[1].strip())
    return out


class SamplesDataPushRefused(RuntimeError):
    """Raised by `guard_samples_data_push` when a real push must not proceed."""


def guard_samples_data_push(
    stats: dict, allow_deletions: bool, confirm: bool, deleted_paths: list[str] | None = None
) -> None:
    """The single most important guardrail in this tool (see the module
    docstring and bug #2 in the task description). Raises
    `SamplesDataPushRefused` -- never silently passes a deletion through --
    unless the caller opted in with BOTH `--allow-deletions` and
    `--confirm`."""
    deleted = stats.get("deleted", 0)
    if deleted <= 0:
        return
    if not allow_deletions:
        raise SamplesDataPushRefused(
            f"refusing to push: sync-samples.ps1 -Push -DryRun reports deleted={deleted}. A real "
            "push would remove those files from samples-data, which may be another open PR's "
            "images (this is exactly the near-miss this tool exists to prevent). Re-run with "
            "--allow-deletions if this is genuinely intended."
        )
    if not confirm:
        listing = "\n".join(f"  - {p}" for p in (deleted_paths or [])) or "  (file list unavailable)"
        raise SamplesDataPushRefused(
            f"--allow-deletions was given but --confirm was not. Refusing to push. The following "
            f"{deleted} file(s) would be deleted:\n{listing}\nRe-run with --confirm to proceed."
        )


# ==========================================================================
# Subprocess plumbing
# ==========================================================================


def run_cmd(cmd: list[str], cwd: Path | None = None, dry_run: bool = False, check: bool = True) -> str:
    """Prints the command, runs it (unless `dry_run`), prints its output, and
    returns stdout. Raises `RuntimeError` with the captured output on a
    non-zero exit when `check` is true."""
    where = f"  (cwd={cwd})" if cwd else ""
    print(f"$ {' '.join(cmd)}{where}")
    if dry_run:
        print("  [dry-run] not executed")
        return ""
    proc = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True, check=False)
    if proc.stdout:
        print(proc.stdout)
    if proc.stderr:
        print(proc.stderr, file=sys.stderr)
    if check and proc.returncode != 0:
        raise RuntimeError(f"command failed ({proc.returncode}): {' '.join(cmd)}\n{proc.stdout}\n{proc.stderr}")
    return proc.stdout


def find_git_bash() -> str:
    """Resolves Git for Windows' `bash.exe` explicitly. Plain `run_cmd(["bash",
    ...])` resolves to WSL bash on this machine, which fails immediately on a
    `D:\\` path (`/mnt/d ... not a git repository` fatal) -- `scripts/check-doc-links.sh`
    and `scripts/check-changelog.sh` must run under Git Bash instead.

    Derives the location from wherever `git` itself is found on PATH, since
    Git for Windows puts `bash.exe` at `<root>/bin/bash.exe` or
    `<root>/usr/bin/bash.exe` and `git.exe` can be found at any of
    `<root>/cmd`, `<root>/bin` or `<root>/mingw64/bin` depending on install
    and PATH order -- so this walks every ancestor of the resolved `git`
    path rather than assuming a fixed number of `..` steps."""
    if sys.platform != "win32":
        found = shutil.which("bash")
        if found:
            return found
        raise RuntimeError("no 'bash' found on PATH")
    git_path = shutil.which("git")
    if git_path:
        for ancestor in Path(git_path).resolve().parents:
            for rel in ("bin/bash.exe", "usr/bin/bash.exe"):
                candidate = ancestor / rel
                if candidate.is_file():
                    return str(candidate)
    raise RuntimeError(
        f"could not resolve Git for Windows' bash.exe from 'git' at {git_path!r} -- "
        "install Git for Windows, or ensure it is on PATH"
    )


def to_msys_path(path: Path) -> str:
    """`D:\\foo\\bar` -> `/d/foo/bar`, the POSIX form Git for Windows' bash
    expects."""
    resolved = str(path.resolve())
    drive, rest = resolved.split(":", 1)
    return "/" + drive.lower() + rest.replace("\\", "/")


def run_bash_script(bash_exe: str, script_rel_path: str, worktree: Path, dry_run: bool) -> str:
    """Runs a repo shell script under `bash_exe`.

    On Windows, wraps the script in `cd '<posix path>' && <script>` and lets
    bash's own `cd` builtin set `$PWD`, instead of relying on
    `subprocess.run(cwd=...)`'s externally-set Win32 working directory: MSYS
    bash launched that way (as every `run_cmd` call does) reports `$PWD` in
    drive-letter form (`D:/...`) rather than POSIX form (`/d/...`). That
    silently breaks `scripts/check-doc-links.sh`'s
    `realpath --relative-to="$repo_root"` logic -- every relative link
    resolves to a bogus doubled path (`D:/foo/d/foo/bar`) and the check
    fails on files that plainly exist, discovered running this tool's own
    checks step. A normal interactive Git Bash session does not hit this,
    because its `$PWD` is set by its own `cd` at shell startup, which is
    exactly what this reproduces."""
    if sys.platform == "win32":
        posix = to_msys_path(worktree)
        cmd = [bash_exe, "-c", f"cd '{posix}' && {script_rel_path}"]
        return run_cmd(cmd, cwd=worktree, dry_run=dry_run)
    return run_cmd([bash_exe, script_rel_path], cwd=worktree, dry_run=dry_run)


def gh_repo_owner_and_name(repo_root: Path) -> tuple[str, str]:
    out = run_cmd(["gh", "repo", "view", "--json", "owner,name"], cwd=repo_root)
    data = json.loads(out)
    return data["owner"]["login"], data["name"]


def gh_pr_number_for_branch(repo_root: Path, branch: str) -> int | None:
    out = run_cmd(["gh", "pr", "list", "--head", branch, "--json", "number", "--limit", "1"], cwd=repo_root)
    try:
        data = json.loads(out or "[]")
    except json.JSONDecodeError as e:
        raise RuntimeError(f"could not parse `gh pr list --head {branch}` output as JSON: {e}") from e
    return data[0]["number"] if data else None


def build_pr_title(cohort_branch: str, public_count: int, local_count: int, summary: str | None = None) -> str:
    cycle = cohort_branch.removeprefix("cohort-")
    body_summary = summary or f"{public_count} public specimen(s)"
    return f"data(corpus): cohort {cycle} -- {body_summary} ({public_count}/{local_count})"


def gh_patch_pr_title_and_body(
    repo_root: Path, owner: str, name: str, pr_number: int, title: str, body_file: Path, dry_run: bool
) -> None:
    """PATCHes an existing PR's title and body via the raw GitHub API --
    `gh pr edit` fails on this machine for lack of the `read:org` scope, so
    `gh api -X PATCH repos/<owner>/<repo>/pulls/<n>` is used instead, which
    only needs the `repo` scope."""
    run_cmd(
        [
            "gh",
            "api",
            "-X",
            "PATCH",
            f"repos/{owner}/{name}/pulls/{pr_number}",
            "-f",
            f"title={title}",
            "-F",
            f"body=@{body_file}",
        ],
        cwd=repo_root,
        dry_run=dry_run,
    )


def gh_list_open_cohort_prs(repo_root: Path) -> list[dict]:
    out = run_cmd(
        ["gh", "pr", "list", "--state", "open", "--json", "headRefName", "--limit", "200"],
        cwd=repo_root,
    )
    try:
        return json.loads(out or "[]")
    except json.JSONDecodeError as e:
        raise RuntimeError(f"could not parse `gh pr list` output as JSON: {e}") from e


def git_show(repo_root: Path, ref_path: str) -> str | None:
    """`git show <ref>:<path>`, or `None` when it doesn't resolve (the branch
    or path doesn't exist)."""
    proc = subprocess.run(
        ["git", "show", ref_path], cwd=repo_root, capture_output=True, text=True, check=False
    )
    if proc.returncode != 0:
        return None
    return proc.stdout


# ==========================================================================
# Orchestration
# ==========================================================================


def find_release_binary(root: Path, name: str) -> Path | None:
    for candidate in (f"{name}.exe", name):
        p = root / "target" / "release" / "examples" / candidate
        if p.is_file():
            return p
    return None


def ensure_binaries_built(worktree: Path, dry_run: bool) -> None:
    """Always rebuilds both release example binaries this tool depends on,
    rather than only building whichever one is missing. A build that only
    filled in a missing binary silently reused a stale
    `target/release/examples/corpus_manifest.exe` whose source was newer
    than it (a worktree created before a crate change) and produced zero
    cover rows on cohort c12. `cargo build` is incremental, so a no-op
    rebuild finishes in seconds; `--dry-run` prints the command without
    running it, same as every other command this tool runs."""
    cmd = [
        "cargo",
        "build",
        "-p",
        "synthpass-ocr",
        "--release",
        "--example",
        "corpus_manifest",
        "--example",
        "check_sample",
    ]
    print("rebuilding release examples (corpus_manifest, check_sample) -- cargo is incremental:")
    run_cmd(cmd, cwd=worktree, dry_run=dry_run)


def copy_rten_models(main_repo_root: Path, worktree: Path, dry_run: bool) -> None:
    models = sorted(main_repo_root.glob("*.rten"))
    if not models:
        print(f"WARNING: no *.rten files found at {main_repo_root} -- corpus_manifest will fail to load OCR models")
    for model in models:
        dest = worktree / model.name
        print(f"copy {model} -> {dest}")
        if not dry_run:
            shutil.copy2(model, dest)


def setup_worktree(repo_root: Path, worktree: Path, cohort_branch: str, reuse_existing: bool, dry_run: bool) -> None:
    if reuse_existing:
        if not dry_run and not worktree.is_dir():
            raise RuntimeError(f"--reuse-existing was given but {worktree} does not exist")
        print(f"reusing existing worktree at {worktree}" if worktree.is_dir() else f"[dry-run] would reuse {worktree}")
    else:
        if not dry_run and worktree.exists():
            raise RuntimeError(f"{worktree} already exists -- pass --reuse-existing or remove it first")
        run_cmd(["git", "fetch", "origin"], cwd=repo_root, dry_run=dry_run)
        run_cmd(
            ["git", "worktree", "add", "-b", cohort_branch, str(worktree), "origin/main"],
            cwd=repo_root,
            dry_run=dry_run,
        )
    if dry_run:
        # --dry-run performs no mutation at all, unconditionally -- see the
        # module docstring's "Two independent safety layers" note. Nothing
        # further in this function is safe to run without a real worktree.
        print("[dry-run] stopping before samples-data pull / model copy / binary build")
        return
    run_cmd(["pwsh", "-File", "./scripts/sync-samples.ps1"], cwd=worktree, dry_run=dry_run)
    copy_rten_models(repo_root, worktree, dry_run)
    ensure_binaries_built(worktree, dry_run)


def apply_cross_pr_exclusions(
    repo_root: Path,
    worktree: Path,
    cohort_branch: str,
    explicit_excludes: list[str],
    dry_run: bool,
) -> dict[str, str]:
    # `gh pr list` and `git show` are read-only -- always run, even under
    # --dry-run, since the duplicate-guard *report* is exactly what a dry run
    # exists to show.
    prs = gh_list_open_cohort_prs(repo_root)
    discovered = discover_open_cohort_branches(cohort_branch, lambda: prs)
    other_branches = sorted(set(discovered) | set(explicit_excludes))

    main_text = git_show(repo_root, "origin/main:samples/corpus.jsonl") or ""
    main_filenames = load_corpus_filenames(main_text)

    other_filenames: dict[str, set[str]] = {}
    for branch in other_branches:
        text = git_show(repo_root, f"origin/{branch}:samples/corpus.jsonl")
        other_filenames[branch] = load_corpus_filenames(text) if text else set()

    exclusions = compute_cross_pr_exclusions(main_filenames, other_filenames)
    if not exclusions:
        print(f"cross-PR duplicate guard: no exclusions needed (checked branches: {other_branches or 'none open'})")
        return exclusions

    print(f"cross-PR duplicate guard: excluding {len(exclusions)} filename(s) already staged in another open PR:")
    for name, reason in exclusions.items():
        print(f"  - {name}: {reason}")

    if dry_run or not worktree.is_dir():
        # --dry-run performs no mutation -- report the exclusions, move
        # nothing. Also nothing to move when there is no worktree yet.
        return exclusions

    quarantine = worktree / "_excluded-pending-other-pr"
    for dir_name in CORPUS_DIRS:
        d = worktree / "samples" / dir_name
        if not d.is_dir():
            continue
        for f in list(d.iterdir()):
            if f.is_file() and f.name in exclusions:
                quarantine.mkdir(parents=True, exist_ok=True)
                dest = quarantine / f.name
                print(f"  quarantining {f} -> {dest} (kept, not deleted, restorable after the other PR merges)")
                if not dry_run:
                    shutil.move(str(f), str(dest))
    return exclusions


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--packet", required=True, type=Path)
    parser.add_argument("--filenames", required=True, type=Path)
    parser.add_argument("--cohort-branch", required=True)
    parser.add_argument("--reuse-existing", action="store_true")
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument("--exclude-branch", action="append", default=[])
    parser.add_argument("--allow-deletions", action="store_true")
    parser.add_argument("--confirm", action="store_true", help="required to actually commit/push/open a PR")
    parser.add_argument("--commit-message-file", type=Path, default=None)
    parser.add_argument("--repo-root", type=Path, default=None)
    args = parser.parse_args(argv)

    repo_root = (args.repo_root or Path(__file__).resolve().parent.parent).resolve()
    worktree = repo_root.parent / "worktrees" / repo_root.name / args.cohort_branch
    # Every path this run itself writes, staged explicitly at commit time
    # instead of `git add -A`, so the commit never picks up an unrelated
    # pre-existing change in the worktree -- and filtered through
    # `stageable_paths`, since the placed images are not tracked on main at
    # all and naming one of them fails the whole staging step.
    touched_paths: set[str] = set()

    packet_text = args.packet.read_text(encoding="utf-8")
    rows = parse_packet(packet_text)
    validate_all_verdicts_present(rows)
    filenames_map = load_filenames_map(args.filenames)

    public_rows = rows_with_verdict(rows, "public")
    local_rows = rows_with_verdict(rows, "local")
    drop_rows = rows_with_verdict(rows, "drop")
    print(
        f"packet: {len(rows)} row(s) -- {len(public_rows)} public, {len(local_rows)} local, "
        f"{len(drop_rows)} drop"
    )

    # Structural filename validation up front, before any worktree/network work.
    problems: list[str] = []
    for row in public_rows + local_rows:
        key = row_key(row)
        target = filenames_map.get(key)
        if target is None:
            problems.append(f"row {row.get('row_number')} ({key}): no entry in --filenames")
            continue
        problems.extend(validate_target_filename(row["verdict"], target))
        warning = check_licence_verdict_agreement(row["verdict"], row.get("proposed_licence", ""))
        if warning:
            print(warning)
    if problems:
        for p in problems:
            print(f"ERROR: {p}", file=sys.stderr)
        raise SystemExit(1)

    setup_worktree(repo_root, worktree, args.cohort_branch, args.reuse_existing, args.dry_run)
    apply_cross_pr_exclusions(repo_root, worktree, args.cohort_branch, args.exclude_branch, args.dry_run)

    if args.dry_run:
        print("\n--dry-run: stopping here -- no worktree was created, no file was written or copied.")
        print("Everything above (packet/filenames validation, the cross-PR duplicate-guard report) is real;")
        print("re-run without --dry-run to place images, regenerate the manifest and run the checks.")
        return 0

    # --- image placement ---
    placed: list[tuple[dict, str]] = []  # (row, target_rel_path)
    for row in public_rows + local_rows:
        target_rel = filenames_map[row_key(row)]
        src = Path(row["local_path"].split("](", 1)[-1].rstrip(")")) if "](" in row["local_path"] else Path(
            row["local_path"]
        )
        if not src.is_absolute():
            src = repo_root / src
        dest = worktree / "samples" / target_rel
        print(f"copy {src} -> {dest}")
        if not args.dry_run:
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(src, dest)
        placed.append((row, target_rel))
        touched_paths.add(f"samples/{Path(target_rel).as_posix()}")

    for row in drop_rows:
        print(f"drop row {row.get('row_number')}: nothing to place (see task note: drop's ledger entry stays manual)")

    # --- manifest regeneration ---
    binary = find_release_binary(worktree, "corpus_manifest")
    if binary is None:
        raise RuntimeError("corpus_manifest binary missing after ensure_binaries_built -- build must have failed")
    run_cmd([str(binary)], cwd=worktree, dry_run=args.dry_run)

    manifest_path = worktree / "samples" / "corpus.jsonl"
    public_coverage: list[tuple[str, str]] = []
    draft_entries: list[tuple[str, str]] = []  # non-cover rows: still a human's coverage call
    cover_entries_by_code: dict[str, list[dict]] = {}
    cover_licences: list[str] = []
    country_names: dict[str, str] = {}
    corpus_count_before = 0
    public_added_count = 0
    if not args.dry_run and manifest_path.is_file():
        manifest_rows = [json.loads(l) for l in manifest_path.read_text(encoding="utf-8").splitlines() if l.strip()]
        by_name = {r["filename"]: r for r in manifest_rows}
        corpus_count_before = len(manifest_rows) - len(placed)
        screened_sidecar: dict[str, dict] = {}
        screened_path = sidecar_path_for(args.packet, args.cohort_branch)
        if screened_path.is_file():
            screened_sidecar = load_screened_sidecar(screened_path.read_text(encoding="utf-8"))
        else:
            print(f"WARNING: no screened sidecar at {screened_path}; origin.url/page will be null and the manifest test will fail")

        today = date.today().isoformat()
        local_new_rows = []
        for row, target_rel in placed:
            filename = Path(target_rel).name
            manifest_row = by_name.get(filename)
            if manifest_row is None:
                print(f"WARNING: {filename} not found in regenerated manifest after copy -- check the target path")
                continue
            status = determine_coverage_status(manifest_row)
            print(f"coverage: {filename} -> {status}")

            sidecar = screened_sidecar.get(row_key(row), {})
            origin = build_origin_patch(
                found_by="dsh",
                url=sidecar.get("image_url_final") or sidecar.get("image_url"),
                page=sidecar.get("page_url_final") or sidecar.get("page_url"),
                licence=row.get("proposed_licence", "").strip() or "none-stated",
                fetched=today,
            )

            if row["verdict"] == "public":
                manifest_row["origin"] = origin
                manifest_row["notes"] = draft_coverage_note(
                    filename, status, origin["licence"], row["verdict"], today
                )
                public_added_count += 1
                public_coverage.append((filename, status))
            else:  # local
                local_new_rows.append(
                    build_local_manifest_row(
                        filename,
                        Path(target_rel).parts[1] if len(Path(target_rel).parts) > 1 else "",
                        manifest_row.get("sha256", ""),
                        origin,
                        draft_coverage_note(filename, status, origin["licence"], "local", today),
                    )
                )

            if is_cover_target(target_rel):
                code = (row.get("code") or "").strip()
                year_value = (manifest_row.get("year") or {}).get("value")
                year_label = str(year_value) if year_value else "XXXX"
                host = urlparse(origin.get("url") or origin.get("page") or "").netloc
                cover_entries_by_code.setdefault(code, []).append(
                    {"basename": filename, "year_label": year_label, "host": host, "licence": origin["licence"]}
                )
                cover_licences.append(origin["licence"])
            else:
                # A row with a data page still needs a human's Status call --
                # a cover never does (ADR-0012), see the CORPUS_COVERAGE.md
                # block below.
                draft_entries.append((filename, status))

        manifest_path.write_text(
            "\n".join(json.dumps(r, ensure_ascii=False) for r in manifest_rows) + "\n", encoding="utf-8"
        )
        touched_paths.add("samples/corpus.jsonl")
        print(f"patched origin/notes for {public_added_count} public row(s)")

        # --- local-track manifest ---
        local_manifest_path = worktree / "samples" / "local" / "local-manifest.jsonl"
        if local_new_rows:
            local_manifest_path.parent.mkdir(parents=True, exist_ok=True)
            with local_manifest_path.open("a", encoding="utf-8") as f:
                for r in local_new_rows:
                    f.write(json.dumps(r, ensure_ascii=False) + "\n")
            print(f"appended {len(local_new_rows)} row(s) to {local_manifest_path}")
            touched_paths.add("samples/local/local-manifest.jsonl")

        # --- CORPUS_COVERAGE.md: real Docs/Note prose for cover rows, never
        # a Status move (ADR-0012) -- see "CORPUS_COVERAGE.md cover-row
        # templates" above. A row with a data page still gets the labelled
        # DRAFT file below instead, since a Status move there is a human call.
        if cover_entries_by_code:
            coverage_md_path = worktree / "knowledge" / "CORPUS_COVERAGE.md"
            coverage_text = coverage_md_path.read_text(encoding="utf-8")
            country_names = code_to_country_name(coverage_text)
            cycle_id = args.cohort_branch.removeprefix("cohort-")
            for code, entries in sorted(cover_entries_by_code.items()):
                docs_cell, note_text = build_cover_coverage_update(code, entries, cycle_id, today)
                coverage_text = update_corpus_coverage_for_cover(coverage_text, code, docs_cell, note_text)
                print(f"CORPUS_COVERAGE.md: {code} Docs -> {docs_cell!r}")
            coverage_md_path.write_text(coverage_text, encoding="utf-8")
            touched_paths.add("knowledge/CORPUS_COVERAGE.md")

            # A cover leaves CORPUS_COVERAGE.md's Status alone (ADR-0012), so
            # nothing in the repo records that the code was scouted at all and
            # `scout_cycle.py suggest` re-proposes it at full priority -- it
            # re-proposed all fourteen c13 codes for c14. The one place that
            # history belongs is STATE.md's `## Codes` section, in the same
            # wording used by hand for c12/c13/c14; `suggest` ranks a code
            # named only there after every fresh code, never out.
            codes_line = scy.cover_only_codes_line(cycle_id, list(cover_entries_by_code))
            scy.edit_state(repo_root, lambda t: scy.add_codes_line(t, codes_line))
            print(f"STATE.md: {codes_line}")

        if draft_entries:
            coverage_draft_path = args.packet.parent / f"coverage-draft-{args.cohort_branch.removeprefix('cohort-')}.md"
            coverage_draft_path.write_text(
                "DRAFT -- review before pasting into knowledge/CORPUS_COVERAGE.md\n\n"
                + "\n".join(f"- {n}: {s}" for n, s in draft_entries)
                + "\n",
                encoding="utf-8",
            )
            print(f"wrote coverage draft to {coverage_draft_path} ({len(draft_entries)} row(s) needing a human Status call)")

    # --- checks ---
    print("\n=== checks (stop on first failure) ===")
    run_cmd([str(binary), "--", "--check"], cwd=worktree, dry_run=args.dry_run)
    run_cmd(["cargo", "test", "-p", "synthpass-bench", "--test", "corpus_manifest"], cwd=worktree, dry_run=args.dry_run)
    bash_exe = find_git_bash()
    run_bash_script(bash_exe, "scripts/check-doc-links.sh", worktree, args.dry_run)
    run_bash_script(bash_exe, "scripts/check-changelog.sh", worktree, args.dry_run)

    # --- changelog fragment: real prose for a covers-only cohort (A4), the
    # existing labelled-draft shape otherwise. Unified on the
    # corpus-cohort-cNN.added.md name the fragments on main actually use
    # (the tool previously wrote corpus-cNN.added.md, a stale name that
    # never matched); a leftover file under that stale name from an earlier
    # run of this tool in the same worktree is removed rather than left to
    # confuse `scripts/check-changelog.sh`.
    cycle_id = args.cohort_branch.removeprefix("cohort-")
    fragment_path = worktree / "changelog.d" / f"corpus-cohort-{cycle_id}.added.md"
    stale_fragment_path = worktree / "changelog.d" / f"corpus-{cycle_id}.added.md"
    if stale_fragment_path != fragment_path and stale_fragment_path.is_file():
        print(f"removing stale fragment from an earlier naming convention: {stale_fragment_path}")
        stale_fragment_path.unlink()

    all_cover = bool(placed) and all(is_cover_target(target_rel) for _, target_rel in placed)
    if all_cover:
        fragment = build_cover_only_changelog_fragment(
            args.cohort_branch,
            cover_entries_by_code,
            cover_licences,
            corpus_count_before,
            len(placed),
            country_names,
        )
    else:
        fragment = build_changelog_fragment(args.cohort_branch, public_coverage, corpus_count_before)
    print(f"writing changelog fragment to {fragment_path}")
    fragment_path.write_text(fragment, encoding="utf-8")
    touched_paths.add(f"changelog.d/corpus-cohort-{cycle_id}.added.md")

    # --- git commit / push / PR: gated behind --confirm, never automatic ---
    print("\n=== commit / push / PR (requires --confirm) ===")
    commit_message = (
        args.commit_message_file.read_text(encoding="utf-8")
        if args.commit_message_file
        else f"data(corpus): {args.cohort_branch} -- {len(public_rows)} public specimen(s)\n\n"
        "DRAFT commit message -- review and pass --commit-message-file to override.\n"
    )
    print("--- draft commit message ---")
    print(commit_message)
    print("--- end draft ---")
    if not args.confirm:
        print("--confirm not given: stopping before git add/commit/push and before any samples-data push.")
        print("Review the worktree, the manifest diff, the coverage draft and the changelog fragment, then re-run")
        print("with --confirm once satisfied.")
        return 0

    to_stage, not_staged = stageable_paths(touched_paths)
    for path in not_staged:
        print(f"not staged (not tracked on main; reaches samples-data through sync-samples.ps1): {path}")
    if not to_stage:
        raise RuntimeError("nothing tracked to stage -- expected at least samples/corpus.jsonl and a changelog fragment")
    run_cmd(["git", "add"] + to_stage, cwd=worktree, dry_run=args.dry_run)
    run_cmd(["git", "commit", "-m", commit_message], cwd=worktree, dry_run=args.dry_run)
    if args.reuse_existing:
        run_cmd(["git", "push"], cwd=worktree, dry_run=args.dry_run)
        pr_number = gh_pr_number_for_branch(repo_root, args.cohort_branch)
        if pr_number is None:
            print(
                f"WARNING: --reuse-existing was given but no open PR was found for branch "
                f"{args.cohort_branch!r}; not patching a PR title/body -- open one by hand."
            )
        else:
            owner, name = gh_repo_owner_and_name(repo_root)
            pr_title = build_pr_title(args.cohort_branch, len(public_rows), len(local_rows))
            body_file = args.commit_message_file
            temp_body_file = None
            if body_file is None:
                temp_body_file = worktree / ".pr-body-draft.md"
                temp_body_file.write_text(commit_message, encoding="utf-8")
                body_file = temp_body_file
            gh_patch_pr_title_and_body(repo_root, owner, name, pr_number, pr_title, body_file, args.dry_run)
            if temp_body_file is not None and temp_body_file.is_file():
                temp_body_file.unlink()
    else:
        run_cmd(["git", "push", "-u", "origin", args.cohort_branch], cwd=worktree, dry_run=args.dry_run)
        run_cmd(
            ["gh", "pr", "create", "--draft", "--title", f"data(corpus): {args.cohort_branch}", "--body", commit_message],
            cwd=worktree,
            dry_run=args.dry_run,
        )

    # --- samples-data push: dry-run first, always ---
    print("\n=== samples-data push ===")
    dryrun_out = run_cmd(["pwsh", "-File", "./scripts/sync-samples.ps1", "-Push", "-DryRun"], cwd=worktree)
    stats = parse_sync_samples_dryrun(dryrun_out)
    print(f"sync-samples.ps1 -Push -DryRun: {stats}")
    deleted_paths: list[str] = []
    if stats.get("deleted", 0) > 0:
        name_status = run_cmd(["git", "diff", "--cached", "-M", "--name-status"], cwd=worktree.parent / "synthpass-samples-data-worktree")
        deleted_paths = extract_deleted_paths(name_status)
    try:
        guard_samples_data_push(stats, args.allow_deletions, args.confirm, deleted_paths)
    except SamplesDataPushRefused as e:
        print(f"ERROR: {e}", file=sys.stderr)
        return 1

    run_cmd(["pwsh", "-File", "./scripts/sync-samples.ps1", "-Push"], cwd=worktree, dry_run=args.dry_run)

    print("\n=== stopping here, deliberately ===")
    print("Not done, on purpose: dispatching the real-specimen re-bless workflow, waiting for it,")
    print("downloading/committing the baseline, or merging the PR (plan §6.6 steps 9-10). Run those")
    print("by hand once this cohort's PR is ready.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
