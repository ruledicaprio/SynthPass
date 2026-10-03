#!/usr/bin/env python3
"""
archive_query.py

Reads the per-document benchmark archive (ADR-0024) that `provider-bench` and
`synthpass-bench` write after every run, and answers questions from runs
already made, without a new OCR pass. Read-only: it never writes to the
archive, never reads a `*.partial` file (a run that was killed) and never
reads `private/`, the private track's text-free records (ADR-0024 Decision 7).

    archive_query.py runs [--track public|local] [--root DIR]
    archive_query.py diff A B [--track ...] [--root DIR]
    archive_query.py cell RUN --line L --col C [--format F | --document-type T] [--chars]
    archive_query.py codes RUN [RUN2] [--track ...] [--root DIR]

`A`, `B` and `RUN` name a run by a `run_id` prefix or by a file path.

* `runs`: one line per finished run file: file, run id, binary, start (UTC),
  commit and dirty flag, scope, record count, providers; the `*.partial` files
  are counted on one line.
* `diff A B`: the header arms that differ (`ocr_arms`, `mrz_arms`,
  `retry_budget`, `pivot_yy`, `env`, the commit, the binary's SHA-256, each OCR
  model file's SHA-256 as `model_sha256.<key>` prefixes, and the corpus scope), then the documents. A real specimen joins on `source_sha256`
  when both records have it (a rename keeps the key), else on `asset_id`, else
  on `name`; a synthetic seed joins on (format, profile, seed). It prints the
  documents only in A and only in B, the outcome changes, the per-field changes
  of `ledger_row` (the field classes are `rebless.py`'s, imported, and the
  timing fields are kept apart), how many recovered zones differ and at which
  cells, and the `truth` mismatch-count deltas. Document lines are capped at
  `rebless.LEDGER_DOC_LINE_CAP`; counts never are.
* `cell RUN --line L --col C`: the classes (letter, digit, filler, other,
  absent) at one cell of the recovered zone across a run's records. Lines and
  columns count from 1, so `--line 1 --col 1` is the first character of the
  first MRZ line, the position ADR-0021 asked about. `--chars` reports the
  characters' own counts instead; it is refused unless the run is public: the
  file must not sit in `local/`, and every record's own `track` must be `public`,
  `covers` or `synthetic` (a missing or unknown `track` refuses).
* `codes RUN [RUN2]`: the document-code witness matrix (#664). For each record that carries the
  printed code's classes (`truth.code_cells`, `A<` for `P<`, `AA` for `PS`), the observed class (the
  first two cells of `tier1_read.lines[0]`, or `zone_classes[0]` on a private record; `unread`
  when nothing was read), counted per provider and format (the format the read resolved; a document
  with no read is under `unknown`). Per printed class it prints the observed counts, the code's
  `document_type` entry of `field_correctness` (exact, wrong, unread, or `no entry`), and for the
  documents whose observed class differs from the printed one the `retry_variant_id` and
  `retry_stop` counts and the asset ids. A record with no `code_cells` is skipped and counted. With
  `RUN2`, the two matrices side by side and the documents whose observed class changed; two runs
  that are not comparable (`comparability_failures`: the same binary, models, scope, tracks, providers,
  data, corpus manifest and replay source, and every document joined one to one on `source_sha256`;
  the arms may differ, they are what is compared) are refused with the facts that differ, exit 2.
  The asset ids it lists are those of the `public`, `covers` and `synthetic` records only; a `local`
  or private record is counted and never named. `codes` is the one command that also reads a
  private run's `private_doc` records, from a file named by its path: they hold class symbols, a
  provider, a format and a SHA-256, never text (Decision 7).

**Disclosure.** The output holds asset ids, field names, enumerated values,
counts, cell positions and classes. It never prints OCR text, a zone line, a
field value, `argv`, `model_paths` or a `miss_reason` text (only its kind).
`--chars` is the one opt-in, and it prints characters only from a public run
(an allowlist of record tracks, so a local run copied out of `local/` still refuses). An error line names a file and a line number, never a line's content.

**Root**, as the benchmark binaries resolve it: `--root DIR`, else
`SYNTHPASS_BENCH_ARCHIVE` as a directory (`off` is an error for a reader),
else `synthpass-bench-archive/` in `git rev-parse --git-common-dir`.

**Schema.** Header `schema` 1 only; any other value is refused with one error
line. Unknown keys are ignored. A file whose first line is not a run header is
refused, and so is a record whose `run_id` is not its header's.

Exit 0 for a report, differences included; 2 for a usage, I/O or schema error.

Standard library only, plus `rebless` (a sibling module) for the ledger field
classes.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from datetime import datetime, timezone
from itertools import zip_longest
from pathlib import Path

import rebless

SCHEMA = 1
TRACKS = ("public", "local")
ARCHIVE_ENV = "SYNTHPASS_BENCH_ARCHIVE"
ARCHIVE_DIR_NAME = "synthpass-bench-archive"
EXIT_OK = 0
EXIT_ERROR = 2
DOC_LINE_CAP = rebless.LEDGER_DOC_LINE_CAP
# The record `track` values whose text `--chars` may print: an allowlist, so a missing or unknown
# value refuses too. The `local` track holds `samples/local/` text and is not on it, and neither
# is the private track, whose `private/` records this tool never reads.
CHARS_TRACKS = frozenset({"public", "covers", "synthetic"})

# Header fields `diff` compares, in print order. `argv` and `model_paths` are deliberately
# absent: they hold paths and are never printed.
ARM_FIELDS = ("ocr_arms", "mrz_arms", "retry_budget", "pivot_yy", "env")
# Fields of a synthetic `ledger_row` that identify the seed or are timing.
SYNTHETIC_JOIN_FIELDS = ("format", "seed", "profile")
SYNTHETIC_TIMING_FIELD = "elapsed_ms"
PROVIDER_TIMING_FIELD = "ocr_ms"


class ArchiveError(Exception):
    """A usage, I/O or schema problem: one error line, exit 2."""


# ------------------------------------------------------------------ root and files


def _git_common_dir() -> Path:
    """The one absolute path `git rev-parse --path-format=absolute --git-common-dir` prints."""
    try:
        out = subprocess.run(
            ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
            capture_output=True,
            text=True,
            check=False,
        )
    except OSError as exc:
        raise ArchiveError(f"cannot run git to find the archive ({type(exc).__name__}); use --root") from None
    lines = [line.strip() for line in out.stdout.splitlines() if line.strip()]
    if out.returncode != 0 or len(lines) != 1 or not Path(lines[0]).is_absolute():
        raise ArchiveError("git did not print one absolute common directory; use --root")
    return Path(lines[0])


def resolve_root(root_arg: str | None, env: dict[str, str] | None = None, git_common_dir=_git_common_dir) -> Path:
    """`--root`, else `SYNTHPASS_BENCH_ARCHIVE` as a directory, else the git common directory's
    `synthpass-bench-archive/`. The variable `off` is an error: a reader has nothing to read."""
    if root_arg:
        return Path(root_arg)
    env = os.environ if env is None else env
    variable = env.get(ARCHIVE_ENV)
    if variable is not None and variable.strip():
        if variable.strip().lower() == "off":
            raise ArchiveError(f"{ARCHIVE_ENV}=off: the archive is off, so there is nothing to read")
        return Path(variable)
    return git_common_dir() / ARCHIVE_DIR_NAME


def run_files(root: Path, track: str) -> tuple[list[Path], int]:
    """`(finished, partial_count)`: the `*.jsonl` files of `root/track` sorted by name (their names
    start with the UTC start, so this is chronological), and how many `*.jsonl.partial` files
    there are. A track directory that does not exist holds nothing."""
    directory = root / track
    if not directory.is_dir():
        return [], 0
    try:
        names = sorted(entry.name for entry in directory.iterdir() if entry.is_file())
    except OSError as exc:
        raise ArchiveError(f"cannot list {track}/ ({type(exc).__name__})") from None
    finished = [directory / name for name in names if name.endswith(".jsonl")]
    partial = sum(1 for name in names if name.endswith(".jsonl.partial"))
    return finished, partial


# ------------------------------------------------------------------ reading a run


@dataclass
class Run:
    path: Path
    header: dict
    records: list[dict]

    @property
    def run_id(self) -> str:
        return str(self.header.get("run_id", ""))


def _parse_header(line: str, path: Path) -> dict:
    """The run header on a file's first line; schema 1 only. An error names the file, never the
    line's content."""
    try:
        header = json.loads(line)
    except ValueError:
        raise ArchiveError(f"{path.name}: line 1 is not JSON, so this is not a run file") from None
    if not isinstance(header, dict) or header.get("kind") != "run":
        raise ArchiveError(f"{path.name}: line 1 is not a run header")
    schema = header.get("schema")
    if schema != SCHEMA or isinstance(schema, bool):
        shown = schema if isinstance(schema, int) and not isinstance(schema, bool) else "?"
        raise ArchiveError(f"{path.name}: schema {shown} is not supported (this tool reads schema {SCHEMA})")
    if not isinstance(header.get("run_id"), str) or not header["run_id"]:
        raise ArchiveError(f"{path.name}: the run header has no run_id")
    return header


def _read_lines(path: Path) -> list[str]:
    try:
        return path.read_text(encoding="utf-8").splitlines()
    except (OSError, UnicodeDecodeError) as exc:
        raise ArchiveError(f"cannot read {path.name} ({type(exc).__name__})") from None


def read_header(path: Path) -> dict:
    """The header only (a file's first line), for listing and for finding a run by id."""
    lines = _read_lines(path)
    if not lines:
        raise ArchiveError(f"{path.name}: the file is empty")
    return _parse_header(lines[0], path)


def read_run(path: Path, kinds: tuple[str, ...] = ("doc",)) -> Run:
    """A whole run: the header, then every record of the given `kinds` (`doc` unless a command asks
    for more). A record of another run is refused; a record of any other kind is ignored, as
    unknown keys are."""
    lines = _read_lines(path)
    if not lines:
        raise ArchiveError(f"{path.name}: the file is empty")
    header = _parse_header(lines[0], path)
    records: list[dict] = []
    for number, line in enumerate(lines[1:], start=2):
        if not line.strip():
            continue
        try:
            record = json.loads(line)
        except ValueError:
            raise ArchiveError(f"{path.name}: line {number} is not JSON") from None
        if not isinstance(record, dict):
            raise ArchiveError(f"{path.name}: line {number} is not a record")
        if record.get("kind") not in kinds:
            continue
        if record.get("run_id") != header["run_id"]:
            raise ArchiveError(f"{path.name}: line {number} is a record of another run")
        records.append(record)
    return Run(path=path, header=header, records=records)


def find_run(spec: str, root: Path, track: str, kinds: tuple[str, ...] = ("doc",)) -> Run:
    """A run named by a file path, or by a `run_id` prefix among the finished files of `track`."""
    candidate = Path(spec)
    if candidate.is_file():
        if candidate.name.endswith(".partial"):
            raise ArchiveError(f"{candidate.name}: a .partial file is a run that never finished and is not read")
        return read_run(candidate, kinds)
    finished, _partial = run_files(root, track)
    matches = []
    for path in finished:
        header = read_header(path)
        if header["run_id"].startswith(spec):
            matches.append(path)
    if not matches:
        raise ArchiveError(f"no finished run in {track}/ matches {spec!r} as a run_id prefix or a file")
    if len(matches) > 1:
        raise ArchiveError(f"{spec!r} matches {len(matches)} runs in {track}/; use a longer prefix")
    return read_run(matches[0], kinds)


# ------------------------------------------------------------------ small helpers


def _start_utc(header: dict) -> str:
    millis = header.get("started_unix_ms")
    if not isinstance(millis, int) or isinstance(millis, bool):
        return "?"
    try:
        return datetime.fromtimestamp(millis / 1000, timezone.utc).strftime("%Y-%m-%d %H:%M:%SZ")
    except (OverflowError, OSError, ValueError):
        return "?"


def _commit(header: dict) -> str:
    commit = header.get("git_commit")
    text = commit[:7] if isinstance(commit, str) and commit else "unknown"
    return text + ("+dirty" if header.get("working_tree_dirty") else "")


def _scope(header: dict) -> str:
    scope = header.get("scope") if isinstance(header.get("scope"), dict) else {}
    parts = [str(scope.get("corpus", "?"))]
    kind = scope.get("format") or scope.get("document_type")
    if kind:
        parts.append(str(kind))
    if scope.get("profile"):
        parts.append(str(scope["profile"]))
    if scope.get("seed_start") is not None:
        parts.append(f"seed={scope['seed_start']}")
    if scope.get("limit") is not None:
        parts.append(f"limit={scope['limit']}")
    parts.append(f"n={scope.get('count', '?')}")
    return " ".join(parts)


def _render(value: object) -> str:
    return rebless._render_value(value)


def _is_synthetic(record: dict) -> bool:
    """A `synthpass-bench` record: identified by (format, profile, seed), with no document name."""
    return "seed" in record and "format" in record and "name" not in record


def _ledger_row(record: dict) -> dict:
    row = record.get("ledger_row")
    return row if isinstance(row, dict) else {}


def outcome_of(record: dict) -> str:
    """The outcome kind: a provider ledger row's `outcome`, or, for a synthetic row, `hit` or its
    `miss_kind`. Never a `miss_reason` text."""
    row = _ledger_row(record)
    if isinstance(row.get("outcome"), str):
        return row["outcome"]
    if row.get("hit"):
        return "hit"
    return str(row.get("miss_kind") or "miss")


def _record_format(record: dict) -> str | None:
    value = record.get("format") or _ledger_row(record).get("mrz_format")
    return str(value).upper() if value else None


def _zone_lines(record: dict) -> list[str] | None:
    read = record.get("tier1_read")
    if not isinstance(read, dict) or not isinstance(read.get("lines"), list):
        return None
    return [line for line in read["lines"] if isinstance(line, str)]


def _cap(lines: list[str], total: int) -> list[str]:
    """`lines` already limited to the cap by the caller; adds `... and N more` when `total` is larger."""
    out = list(lines)
    if total > len(lines):
        out.append(f"  ... and {total - len(lines)} more")
    return out


# ------------------------------------------------------------------ runs


def _is_doc_line(line: str) -> bool:
    """Whether a line is a `doc` record, as `read_run` counts it. A line that is not JSON is not
    one: `runs` does not validate a file beyond its header, `diff` and `cell` do."""
    if not line.strip():
        return False
    try:
        record = json.loads(line)
    except ValueError:
        return False
    return isinstance(record, dict) and record.get("kind") == "doc"


def cmd_runs(root: Path, track: str, out) -> int:
    finished, partial = run_files(root, track)
    errors = 0
    listed = 0
    records_total = 0
    for path in finished:
        try:
            lines = _read_lines(path)
            if not lines:
                raise ArchiveError(f"{path.name}: the file is empty")
            header = _parse_header(lines[0], path)
        except ArchiveError as exc:
            print(f"error: {exc}", file=sys.stderr)
            errors += 1
            continue
        records = sum(1 for line in lines[1:] if _is_doc_line(line))
        providers = header.get("providers") if isinstance(header.get("providers"), list) else []
        out.write(
            f"{path.name} {header['run_id'][:12]} {header.get('binary_name', '?')} {_start_utc(header)} "
            f"{_commit(header)} {_scope(header)} records={records} "
            f"providers={','.join(str(p) for p in providers) or '-'}\n"
        )
        listed += 1
        records_total += records
    out.write(f"{track}/: {listed} run(s), {records_total} record(s); {partial} .partial file(s) skipped\n")
    return EXIT_ERROR if errors else EXIT_OK


# ------------------------------------------------------------------ diff: header


def _header_value(header: dict, field: str) -> object:
    return header.get(field)


def _prefix(value: object) -> str:
    """A hash as its first 12 characters, or `null` when it is not recorded."""
    return str(value)[:12] if isinstance(value, str) else "null"


def header_differences(a: dict, b: dict) -> list[str]:
    """One line per header arm or fact that differs. Dict-valued arms are compared key by key. Values
    are the enumerated settings the header records (arms, allowlisted environment settings),
    commits and hash prefixes; `argv` and `model_paths` are never read."""
    lines: list[str] = []
    for field in ARM_FIELDS:
        old, new = _header_value(a, field), _header_value(b, field)
        if old == new:
            continue
        if isinstance(old, dict) and isinstance(new, dict):
            for key in sorted(set(old) | set(new)):
                if old.get(key) != new.get(key):
                    lines.append(f"  {field}.{key}: {_render(old.get(key))} -> {_render(new.get(key))}")
        else:
            lines.append(f"  {field}: {_render(old)} -> {_render(new)}")
    if a.get("source") != b.get("source"):
        lines.append(f"  source: {_render(a.get('source'))} -> {_render(b.get('source'))}")
    label_a = a["machine"].get("label") if isinstance(a.get("machine"), dict) else None
    label_b = b["machine"].get("label") if isinstance(b.get("machine"), dict) else None
    if label_a != label_b:
        lines.append(f"  machine.label: {_render(label_a)} -> {_render(label_b)}")
    if (a.get("git_commit"), a.get("working_tree_dirty")) != (b.get("git_commit"), b.get("working_tree_dirty")):
        lines.append(f"  commit: {_commit(a)} -> {_commit(b)}")
    if a.get("binary_sha256") != b.get("binary_sha256"):
        lines.append(
            f"  binary_sha256: {str(a.get('binary_sha256'))[:12]} -> {str(b.get('binary_sha256'))[:12]}"
        )
    old_scope, new_scope = a.get("scope"), b.get("scope")
    if old_scope != new_scope and isinstance(old_scope, dict) and isinstance(new_scope, dict):
        for key in sorted(set(old_scope) | set(new_scope)):
            if old_scope.get(key) != new_scope.get(key):
                lines.append(f"  scope.{key}: {_render(old_scope.get(key))} -> {_render(new_scope.get(key))}")
    for field in ("corpus_manifest_sha256", "samples_data_sha"):
        if a.get(field) != b.get(field):
            lines.append(f"  {field}: {str(a.get(field))[:12]} -> {str(b.get(field))[:12]}")
    # The OCR model files' SHA-256, key by key, as prefixes. A header written before the key
    # existed, and a replay (which loads no model), both read as null.
    old_models, new_models = (h.get("model_sha256") if isinstance(h.get("model_sha256"), dict) else {} for h in (a, b))
    for key in sorted(set(old_models) | set(new_models)):
        if old_models.get(key) != new_models.get(key):
            lines.append(f"  model_sha256.{key}: {_prefix(old_models.get(key))} -> {_prefix(new_models.get(key))}")
    return lines


# ------------------------------------------------------------------ diff: joining


@dataclass
class Pair:
    key: str
    a: dict
    b: dict
    by: str  # "source_sha256", "asset_id", "name" or "seed"

    @property
    def renamed(self) -> bool:
        return self.by == "source_sha256" and _display_key(self.a) != _display_key(self.b)


def _display_key(record: dict) -> str:
    if _is_synthetic(record):
        return f"{record['format']} {record.get('profile', '?')} seed {record['seed']}"
    return str(record.get("asset_id") or record.get("name") or "?")


def _provider(record: dict) -> str:
    return str(record.get("provider") or "")


def join_records(a_records: list[dict], b_records: list[dict]) -> tuple[list[Pair], list[dict], list[dict]]:
    """`(pairs, only_a, only_b)`. Per provider: a synthetic seed joins on (format, profile, seed); a
    real specimen on `source_sha256` when both records have it and it is unique on both sides (a rename
    keeps it), then on `asset_id`, then on `name`, each only among what is still unmatched."""
    providers = sorted({_provider(r) for r in a_records} | {_provider(r) for r in b_records})
    multi = len(providers) > 1
    pairs: list[Pair] = []
    only_a: list[dict] = []
    only_b: list[dict] = []
    for provider in providers:
        a = [r for r in a_records if _provider(r) == provider]
        b = [r for r in b_records if _provider(r) == provider]
        matched_a: set[int] = set()
        matched_b: set[int] = set()

        def add(i: int, j: int, by: str) -> None:
            matched_a.add(i)
            matched_b.add(j)
            key = _display_key(a[i])
            pairs.append(Pair(f"{provider}: {key}" if multi and provider else key, a[i], b[j], by))

        def join_on(extract, by: str) -> None:
            index_a: dict[object, list[int]] = defaultdict(list)
            index_b: dict[object, list[int]] = defaultdict(list)
            for i, record in enumerate(a):
                value = extract(record)
                if i not in matched_a and value is not None:
                    index_a[value].append(i)
            for j, record in enumerate(b):
                value = extract(record)
                if j not in matched_b and value is not None:
                    index_b[value].append(j)
            for value, ids in index_a.items():
                if len(ids) == 1 and len(index_b.get(value, ())) == 1:
                    add(ids[0], index_b[value][0], by)

        def seed_key(record: dict):
            return (record["format"], record.get("profile"), record["seed"]) if _is_synthetic(record) else None

        join_on(seed_key, "seed")
        join_on(lambda r: r.get("source_sha256") if isinstance(r.get("source_sha256"), str) else None, "source_sha256")
        join_on(lambda r: r.get("asset_id") if isinstance(r.get("asset_id"), str) else None, "asset_id")
        join_on(lambda r: r.get("name") if isinstance(r.get("name"), str) else None, "name")
        prefix = f"{provider}: " if multi and provider else ""
        only_a.extend({**r, "_display": prefix + _display_key(r)} for i, r in enumerate(a) if i not in matched_a)
        only_b.extend({**r, "_display": prefix + _display_key(r)} for j, r in enumerate(b) if j not in matched_b)
    pairs.sort(key=lambda p: p.key)
    return pairs, only_a, only_b


# ------------------------------------------------------------------ diff: documents


def _synthetic_field_changes(old: dict, new: dict) -> list[tuple[str, str]]:
    """`(field, text)` for every synthetic ledger-row field that differs, apart from the seed's
    identity and its timing."""
    changes: list[tuple[str, str]] = []
    skip = set(SYNTHETIC_JOIN_FIELDS) | {SYNTHETIC_TIMING_FIELD}
    for field in sorted((set(old) | set(new)) - skip):
        if old.get(field) != new.get(field):
            changes.append((field, f"{field} {_render(old.get(field))} -> {_render(new.get(field))}"))
    return changes


def field_changes(pair: Pair) -> list[tuple[str, str]]:
    old, new = _ledger_row(pair.a), _ledger_row(pair.b)
    if _is_synthetic(pair.a):
        return _synthetic_field_changes(old, new)
    return rebless._ledger_field_changes(old, new)


def _field_totals(docs: list[tuple[str, list[tuple[str, str]]]]) -> str:
    counts: Counter = Counter()
    for _key, changes in docs:
        for name in {name for name, _text in changes}:
            counts[name] += 1
    known = [f for f in rebless.LEDGER_DIFFED_FIELDS if f in counts]
    others = sorted(f for f in counts if f not in rebless.LEDGER_DIFFED_FIELDS)
    return ", ".join(f"{field} {counts[field]}" for field in known + others)


def _timing(record: dict) -> int | None:
    row = _ledger_row(record)
    for field in (SYNTHETIC_TIMING_FIELD, PROVIDER_TIMING_FIELD):
        value = row.get(field)
        if isinstance(value, int) and not isinstance(value, bool):
            return value
    return None


def zone_cell_differences(a_lines: list[str], b_lines: list[str]) -> list[tuple[int, list[int]]]:
    """`(line, columns)` where two recovered zones differ, 1-based, in line order. A column past one
    zone's end differs. Positions only: no character is returned."""
    out: list[tuple[int, list[int]]] = []
    for line_no, (x, y) in enumerate(zip_longest(a_lines, b_lines, fillvalue=""), start=1):
        columns = [col for col, (p, q) in enumerate(zip_longest(x, y), start=1) if p != q]
        if columns:
            out.append((line_no, columns))
    return out


def _positions(diffs: list[tuple[int, list[int]]]) -> str:
    return "; ".join(f"line {line} col " + ",".join(str(c) for c in cols[:8]) + (",..." if len(cols) > 8 else "") for line, cols in diffs)


def _truth_numbers(record: dict) -> tuple[int | None, dict]:
    truth = record.get("truth")
    if not isinstance(truth, dict):
        return None, {}
    mismatch = truth.get("zone_mismatch")
    mismatch = mismatch if isinstance(mismatch, int) and not isinstance(mismatch, bool) else None
    field = truth.get("field_mismatch")
    by_field = field.get("by_field") if isinstance(field, dict) and isinstance(field.get("by_field"), dict) else {}
    return mismatch, {k: v for k, v in by_field.items() if isinstance(v, int) and not isinstance(v, bool)}


def document_diff_lines(pairs: list[Pair], only_a: list[dict], only_b: list[dict]) -> list[str]:
    lines: list[str] = []
    renamed = sum(1 for p in pairs if p.renamed)
    by_counts = Counter(p.by for p in pairs)
    joined_by = ", ".join(f"{by} {n}" for by, n in sorted(by_counts.items())) or "none"
    lines.append(f"documents: {len(pairs)} joined ({joined_by}), {len(only_a)} only in A, {len(only_b)} only in B")
    if renamed:
        lines.append(f"  {renamed} joined by source_sha256 under a different asset ID (renamed)")
    for label, only in (("A", only_a), ("B", only_b)):
        if only:
            lines.append(f"only in {label}: {len(only)} document(s)")
            lines.extend(_cap([f"  {r['_display']}" for r in only[:DOC_LINE_CAP]], len(only)))

    # outcomes
    moved = [(p.key, outcome_of(p.a), outcome_of(p.b)) for p in pairs if outcome_of(p.a) != outcome_of(p.b)]
    lines.append(f"outcome changes: {len(moved)} document(s)")
    lines.extend(_cap([f"  {key}: {old} -> {new}" for key, old, new in moved[:DOC_LINE_CAP]], len(moved)))

    # fields: deterministic apart from budget-limited, then timing
    deterministic: list[tuple[str, list[tuple[str, str]]]] = []
    budget_limited: list[tuple[str, list[tuple[str, str]]]] = []
    deltas: list[int] = []
    timed = old_total = new_total = 0
    for p in pairs:
        old_t, new_t = _timing(p.a), _timing(p.b)
        if old_t is not None and new_t is not None:
            timed += 1
            old_total += old_t
            new_total += new_t
            if old_t != new_t:
                deltas.append(abs(old_t - new_t))
        changes = field_changes(p)
        if changes:
            limited = rebless._is_budget_limited(_ledger_row(p.a), _ledger_row(p.b))
            (budget_limited if limited else deterministic).append((p.key, changes))
    lines.append(
        f"field changes (deterministic): {len(deterministic)} document(s)"
        + (f"; {_field_totals(deterministic)}" if deterministic else "")
    )
    lines.extend(
        _cap(
            [f"  {key}: " + "; ".join(text for _n, text in changes) for key, changes in deterministic[:DOC_LINE_CAP]],
            len(deterministic),
        )
    )
    lines.append(
        f"timing (kept apart, report-only): differs on {len(deltas)} of {timed} document(s), "
        f"median |delta| {rebless._median(deltas)} ms, total {old_total} ms -> {new_total} ms"
    )
    if budget_limited:
        lines.append(
            "budget-limited documents (every change on them is timing-sensitive): "
            f"{len(budget_limited)} document(s); {_field_totals(budget_limited)}"
        )
        lines.extend(
            _cap(
                [f"  {key}: " + "; ".join(text for _n, text in changes) for key, changes in budget_limited[:DOC_LINE_CAP]],
                len(budget_limited),
            )
        )

    # recovered zones
    compared = zone_only_a = zone_only_b = 0
    zone_diffs: list[tuple[str, list[tuple[int, list[int]]]]] = []
    for p in pairs:
        za, zb = _zone_lines(p.a), _zone_lines(p.b)
        if za is None and zb is None:
            continue
        if za is None:
            zone_only_b += 1
            continue
        if zb is None:
            zone_only_a += 1
            continue
        compared += 1
        diffs = zone_cell_differences(za, zb)
        if diffs:
            zone_diffs.append((p.key, diffs))
    lines.append(
        f"recovered zones: {len(zone_diffs)} of {compared} compared document(s) differ; "
        f"a zone in A only {zone_only_a}, in B only {zone_only_b}"
    )
    lines.extend(
        _cap(
            [f"  {key}: {_positions(diffs)}" for key, diffs in zone_diffs[:DOC_LINE_CAP]],
            len(zone_diffs),
        )
    )

    # truth mismatch counts
    changed: list[tuple[str, int, int]] = []
    sum_a = sum_b = both = 0
    field_a: Counter = Counter()
    field_b: Counter = Counter()
    for p in pairs:
        ma, fa = _truth_numbers(p.a)
        mb, fb = _truth_numbers(p.b)
        if ma is None or mb is None:
            continue
        both += 1
        sum_a += ma
        sum_b += mb
        field_a.update(fa)
        field_b.update(fb)
        if ma != mb:
            changed.append((p.key, ma, mb))
    lines.append(
        f"truth mismatch: {len(changed)} of {both} labelled document(s) changed; "
        f"zone_mismatch total {sum_a} -> {sum_b}"
    )
    by_field = [
        f"{field} {field_a[field]} -> {field_b[field]}"
        for field in sorted(set(field_a) | set(field_b))
        if field_a[field] != field_b[field]
    ]
    if by_field:
        lines.append("  by field: " + ", ".join(by_field))
    lines.extend(_cap([f"  {key}: {old} -> {new}" for key, old, new in changed[:DOC_LINE_CAP]], len(changed)))
    return lines


def cmd_diff(a: Run, b: Run, out) -> int:
    out.write(f"archive diff: A {a.path.name} (run {a.run_id[:12]}) vs B {b.path.name} (run {b.run_id[:12]})\n")
    differences = header_differences(a.header, b.header)
    if differences:
        out.write("header: differs\n")
        out.writelines(line + "\n" for line in differences)
    else:
        out.write("header: identical in the arms, retry budget, pivot, environment, source, machine, commit, binary and scope\n")
    pairs, only_a, only_b = join_records(a.records, b.records)
    out.writelines(line + "\n" for line in document_diff_lines(pairs, only_a, only_b))
    return EXIT_OK


# ------------------------------------------------------------------ cell


CELL_CLASSES = ("letter", "digit", "filler", "other", "absent")


def classify_char(char: str) -> str:
    if char == "<":
        return "filler"
    if char.isascii() and char.isalpha():
        return "letter"
    if char.isascii() and char.isdigit():
        return "digit"
    return "other"


def cmd_cell(run: Run, line: int, col: int, fmt: str | None, chars: bool, track: str, out) -> int:
    if line < 1 or col < 1:
        raise ArchiveError("--line and --col count from 1")
    # Where the file sits says nothing once it is copied: the records' own `track` decide too.
    if chars and (
        track != "public"
        or run.path.parent.name == "local"
        or any(record.get("track") not in CHARS_TRACKS for record in run.records)
    ):
        raise ArchiveError("--chars prints characters and is only allowed on the public track")
    records = [r for r in run.records if fmt is None or _record_format(r) == fmt]
    classes: Counter = Counter()
    characters: Counter = Counter()
    no_zone = 0
    for record in records:
        zone = _zone_lines(record)
        if zone is None:
            no_zone += 1
            continue
        if line > len(zone) or col > len(zone[line - 1]):
            classes["absent"] += 1
            characters["(absent)"] += 1
            continue
        char = zone[line - 1][col - 1]
        classes[classify_char(char)] += 1
        characters[char] += 1
    label = f" ({fmt})" if fmt else ""
    out.write(
        f"cell line {line} col {col}{label}: {len(records)} record(s) in run {run.run_id[:12]}, "
        f"{no_zone} with no zone read\n"
    )
    if chars:
        out.write("  characters: " + ", ".join(f"{char!r} {n}" for char, n in sorted(characters.items())) + "\n")
    else:
        out.write("  " + ", ".join(f"{name} {classes[name]}" for name in CELL_CLASSES) + "\n")
    return EXIT_OK


# ------------------------------------------------------------------ codes

UNREAD = "unread"
NO_FORMAT = "unknown"
NO_ENTRY = "no entry"
CODE_FIELD = "document_type"
CLASS_SYMBOLS = {"letter": "A", "digit": "9", "filler": "<", "other": "?"}
CLASS_SYMBOL_SET = frozenset(CLASS_SYMBOLS.values())
FIELD_VERDICTS = ("exact", "wrong", "unread")
# The retry facts are written as text by a replay too, so only the closed sets are printed.
RETRY_STOPS = frozenset({"general_valid", "variant_valid", "pass_cap", "budget", "exhausted"})
RETRY_VARIANT = re.compile(r"general|pass-\d{2,4}")
# The record tracks whose asset ids `codes` may print, the allowlist `--chars` uses.
LISTED_TRACKS = CHARS_TRACKS
PRIVATE_KIND = "private_doc"
CODE_KINDS = ("doc", PRIVATE_KIND)
# Header facts that must be equal for two runs to be compared, `promotion_gate.EQUAL_FACTS` (a test
# keeps the two the same). The arms are not here: they are what two runs differ in.
COMPARABLE_FACTS = ("scope", "tracks", "providers", "samples_data_sha", "corpus_manifest_sha256", "replay_of")


@dataclass
class CodeRow:
    """One document's code classes, and what `codes` may say about it: no text, only classes,
    enumerated facts and, for a record on a listed track, its asset id."""

    section: tuple[str, str]  # (provider, format)
    printed: str
    observed: str
    listed: bool
    shown: str  # the asset id; read only when `listed`
    variant: str
    stop: str
    verdict: str  # the `document_type` entry of `field_correctness`, or NO_ENTRY

    @property
    def differs(self) -> bool:
        return self.printed != self.observed


def _symbols(value: object, what: str) -> str:
    """`value` as a string of at most two class symbols, else an error that names no content."""
    if not isinstance(value, str) or len(value) > 2 or not set(value) <= CLASS_SYMBOL_SET:
        raise ArchiveError(f"a record's {what} is not at most two class symbols (A, 9, < or ?)")
    return value


def printed_class(record: dict) -> str | None:
    """The printed code's classes, `truth.code_cells`; `None` when the record does not carry them."""
    truth = record.get("truth")
    value = truth.get("code_cells") if isinstance(truth, dict) else None
    return None if value is None else _symbols(value, "truth.code_cells")


def observed_class(record: dict) -> str:
    """The first two cells of the recovered zone's first line as class symbols; `unread` when
    nothing was read. A private record has the classes already, in `zone_classes`."""
    if record.get("kind") == PRIVATE_KIND:
        zone = record.get("zone_classes")
        first = zone[0] if isinstance(zone, list) and zone else None
        symbols = "" if first is None else _symbols(first[:2] if isinstance(first, str) else first, "zone_classes")
    else:
        lines = _zone_lines(record)
        symbols = "".join(CLASS_SYMBOLS[classify_char(char)] for char in lines[0][:2]) if lines else ""
    return symbols or UNREAD


def _retry_facts(record: dict) -> dict:
    return record if record.get("kind") == PRIVATE_KIND else _ledger_row(record)


def _variant_text(value: object) -> str:
    if value is None:
        return "none"
    return value if isinstance(value, str) and RETRY_VARIANT.fullmatch(value) else "other"


def _stop_text(value: object) -> str:
    if value is None:
        return "none"
    return value if isinstance(value, str) and value in RETRY_STOPS else "other"


def _code_format(record: dict) -> str:
    value = record.get("mrz_format") if record.get("kind") == PRIVATE_KIND else _record_format(record)
    return str(value).upper() if isinstance(value, str) and value else NO_FORMAT


def _code_verdict(record: dict) -> str:
    correctness = record.get("field_correctness")
    verdict = correctness.get(CODE_FIELD) if isinstance(correctness, dict) else None
    return verdict if verdict in FIELD_VERDICTS else NO_ENTRY


def code_row(record: dict) -> CodeRow | None:
    """The row for one record, or `None` when it carries no printed code class."""
    printed = printed_class(record)
    if printed is None:
        return None
    facts = _retry_facts(record)
    listed = record.get("kind") == "doc" and record.get("track") in LISTED_TRACKS
    return CodeRow(
        section=(str(record.get("provider") or "?"), _code_format(record)),
        printed=printed,
        observed=observed_class(record),
        listed=listed,
        shown=_display_key(record) if listed else "",
        variant=_variant_text(facts.get("retry_variant_id")),
        stop=_stop_text(facts.get("retry_stop")),
        verdict=_code_verdict(record),
    )


def code_rows(run: Run) -> tuple[list[CodeRow], int]:
    """`(rows, skipped)`: a row per record with a printed code class, and how many records had none."""
    rows = []
    skipped = 0
    for record in run.records:
        row = code_row(record)
        if row is None:
            skipped += 1
        else:
            rows.append(row)
    return rows, skipped


def _class_order(name: str) -> tuple[bool, str]:
    return (name == UNREAD, name)


def _counts(counter: Counter) -> str:
    return ", ".join(f"{name} {counter[name]}" for name in sorted(counter)) or "none"


def _class_counts(counter: Counter) -> str:
    """Observed classes as `name n`, `unread` last."""
    return ", ".join(f"{name} {counter[name]}" for name in sorted(counter, key=_class_order)) or "none"


def _sections(*row_sets: list[CodeRow]) -> list[tuple[str, str]]:
    return sorted({row.section for rows in row_sets for row in rows})


def _document_lines(rows: list[CodeRow]) -> list[str]:
    """The documents whose observed class differs from the printed one: named when their record's
    track allows it, else counted."""
    differing = [row for row in rows if row.differs]
    named = [f"  {row.shown}: printed {row.printed}, observed {row.observed}" for row in differing if row.listed]
    unnamed = sum(1 for row in differing if not row.listed)
    lines = _cap(sorted(named)[:DOC_LINE_CAP], len(named))
    if unnamed:
        lines.append(f"  {unnamed} more on the local or private track, not named")
    return lines


def _differs_line(rows: list[CodeRow]) -> str:
    differing = [row for row in rows if row.differs]
    return (
        f"  observed class differs from printed: {len(differing)} document(s); "
        f"retry_variant_id {_counts(Counter(row.variant for row in differing))}; "
        f"retry_stop {_counts(Counter(row.stop for row in differing))}"
    )


def _section_lines(section: tuple[str, str], rows: list[CodeRow]) -> list[str]:
    provider, fmt = section
    lines = [f"provider {provider}, format {fmt}: {len(rows)} document(s)"]
    for printed in sorted({row.printed for row in rows}, key=_class_order):
        lines.append(f"  printed {printed} -> observed " + _class_counts(Counter(row.observed for row in rows if row.printed == printed)))
    lines.append(_differs_line(rows))
    lines.extend(_document_lines(rows))
    return lines


def _all_lines(rows: list[CodeRow]) -> list[str]:
    """Every format together: the code's `document_type` entry per printed class, and the totals."""
    lines = [f"all formats: {len(rows)} document(s)"]
    for printed in sorted({row.printed for row in rows}, key=_class_order):
        verdicts = Counter(row.verdict for row in rows if row.printed == printed)
        lines.append(
            f"  printed {printed}, {CODE_FIELD} field: "
            + ", ".join(f"{name} {verdicts[name]}" for name in (*FIELD_VERDICTS, NO_ENTRY))
        )
    lines.append(_differs_line(rows))
    return lines


def codes_report(run: Run) -> list[str]:
    rows, skipped = code_rows(run)
    lines = [
        f"document-code classes: {run.path.name} (run {run.run_id[:12]}): {len(run.records)} record(s), "
        f"{len(rows)} with a printed code class, {skipped} skipped for none"
    ]
    for section in _sections(rows):
        lines.extend(_section_lines(section, [row for row in rows if row.section == section]))
    lines.extend(_all_lines(rows))
    return lines


def _hex64(value: object) -> bool:
    return isinstance(value, str) and len(value) == 64 and all(char in "0123456789abcdef" for char in value)


def comparability_failures(a: Run, b: Run) -> list[str]:
    """Every reason two runs cannot be compared document by document; names facts, never values.
    The arms may differ. The facts are those of `promotion_gate.py`'s comparability check."""
    failures: list[str] = []
    for name, run in (("A", a), ("B", b)):
        if not isinstance(run.header.get("binary_sha256"), str) or not run.header["binary_sha256"]:
            failures.append(f"binary_sha256 is not recorded on run {name}")
        models = run.header.get("model_sha256")
        if run.header.get("replay_of") is None and not (
            isinstance(models, dict) and all(_hex64(models.get(key)) for key in ("detection", "recognition"))
        ):
            failures.append(f"model_sha256 is not recorded on run {name}")
    if a.header.get("binary_sha256") != b.header.get("binary_sha256"):
        failures.append("binary_sha256 differs")
    if a.header.get("model_sha256") != b.header.get("model_sha256"):
        failures.append("model_sha256 differs")
    for fact in COMPARABLE_FACTS:
        if a.header.get(fact) != b.header.get(fact):
            failures.append(f"{fact} differs")
    pairs, only_a, only_b = join_records(a.records, b.records)
    if only_a or only_b:
        failures.append(f"the document sets differ: {len(only_a)} only in A, {len(only_b)} only in B")
    loose = sum(1 for pair in pairs if pair.by != "source_sha256")
    if loose:
        failures.append(f"{loose} document(s) do not join one to one on source_sha256")
    return failures


def codes_diff_lines(a: Run, b: Run) -> list[str]:
    """Two comparable runs side by side, then the documents whose observed class changed."""
    failures = comparability_failures(a, b)
    if failures:
        raise ArchiveError("the runs are not comparable: " + "; ".join(failures))
    rows_a, skipped_a = code_rows(a)
    rows_b, skipped_b = code_rows(b)
    lines = [
        f"document-code classes: A {a.path.name} (run {a.run_id[:12]}) vs B {b.path.name} (run {b.run_id[:12]}): "
        f"{len(rows_a)} and {len(rows_b)} with a printed code class, {skipped_a} and {skipped_b} skipped for none"
    ]
    differences = header_differences(a.header, b.header)
    lines.append("header: differs" if differences else "header: identical in the arms, retry budget, pivot, environment, source, machine, commit, binary and scope")
    lines.extend(differences)
    for section in _sections(rows_a, rows_b):
        mine_a = [row for row in rows_a if row.section == section]
        mine_b = [row for row in rows_b if row.section == section]
        lines.append(f"provider {section[0]}, format {section[1]}: {len(mine_a)} document(s) in A, {len(mine_b)} in B")
        for printed in sorted({row.printed for row in (*mine_a, *mine_b)}, key=_class_order):
            observed_a = Counter(row.observed for row in mine_a if row.printed == printed)
            observed_b = Counter(row.observed for row in mine_b if row.printed == printed)
            lines.append(f"  printed {printed} -> observed A: {_class_counts(observed_a)} | B: {_class_counts(observed_b)}")
    pairs, _only_a, _only_b = join_records(a.records, b.records)
    changed: list[tuple[str, str, str]] = []
    unnamed = compared = 0
    for pair in pairs:
        row_a, row_b = code_row(pair.a), code_row(pair.b)
        if row_a is None or row_b is None:
            continue
        compared += 1
        if row_a.observed != row_b.observed:
            if row_a.listed and row_b.listed:
                changed.append((row_a.shown, row_a.observed, row_b.observed))
            else:
                unnamed += 1
    lines.append(f"observed class changed between the runs: {len(changed) + unnamed} of {compared} compared document(s)")
    lines.extend(_cap([f"  {name}: {old} -> {new}" for name, old, new in sorted(changed)[:DOC_LINE_CAP]], len(changed)))
    if unnamed:
        lines.append(f"  {unnamed} more on the local or private track, not named")
    return lines


def cmd_codes(a: Run, b: Run | None, out) -> int:
    lines = codes_report(a) if b is None else codes_diff_lines(a, b)
    out.writelines(line + "\n" for line in lines)
    return EXIT_OK


# ------------------------------------------------------------------ main


def build_parser() -> argparse.ArgumentParser:
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", help="the archive root (default: SYNTHPASS_BENCH_ARCHIVE, else the git common dir's synthpass-bench-archive/)")
    common.add_argument("--track", choices=TRACKS, default="public", help="which track to read (default: public)")
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0] if __doc__ else None)
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("runs", parents=[common], help="one line per finished run file")
    diff = sub.add_parser("diff", parents=[common], help="the documents that differ between two runs")
    diff.add_argument("a", metavar="A", help="a run_id prefix or a file path")
    diff.add_argument("b", metavar="B", help="a run_id prefix or a file path")
    cell = sub.add_parser("cell", parents=[common], help="the classes at one cell of the recovered zone")
    cell.add_argument("run", metavar="RUN", help="a run_id prefix or a file path")
    cell.add_argument("--line", type=int, required=True, help="MRZ line, from 1")
    cell.add_argument("--col", type=int, required=True, help="column, from 1")
    cell.add_argument("--format", dest="fmt", help="only records of this format (TD1, TD2, TD3, MRVA, MRVB)")
    cell.add_argument("--document-type", dest="document_type", help="the same filter, for a synthetic run")
    cell.add_argument("--chars", action="store_true", help="report the characters' counts (public track only)")
    codes = sub.add_parser("codes", parents=[common], help="the printed document code's classes against the observed ones")
    codes.add_argument("run", metavar="RUN", help="a run_id prefix or a file path")
    codes.add_argument("run2", metavar="RUN2", nargs="?", help="a second, comparable run: both matrices and what changed")
    return parser


def main(argv: list[str] | None = None, out=None) -> int:
    out = sys.stdout if out is None else out
    args = build_parser().parse_args(argv)
    try:
        root = resolve_root(args.root)
        if args.command == "runs":
            return cmd_runs(root, args.track, out)
        if args.command == "diff":
            return cmd_diff(find_run(args.a, root, args.track), find_run(args.b, root, args.track), out)
        if args.command == "codes":
            second = find_run(args.run2, root, args.track, CODE_KINDS) if args.run2 else None
            return cmd_codes(find_run(args.run, root, args.track, CODE_KINDS), second, out)
        fmt = args.fmt or args.document_type
        if args.fmt and args.document_type and args.fmt.upper() != args.document_type.upper():
            raise ArchiveError("--format and --document-type name different formats")
        return cmd_cell(
            find_run(args.run, root, args.track),
            args.line,
            args.col,
            fmt.upper() if fmt else None,
            args.chars,
            args.track,
            out,
        )
    except ArchiveError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return EXIT_ERROR


if __name__ == "__main__":
    sys.exit(main())
