#!/usr/bin/env python3
"""
promotion_gate.py

A read-only gate an OCR arm passes before it is promoted to a default (#664). It reads two kinds of
archived real-specimen `provider-bench` runs (ADR-0024): `BASE`, a run of the default arm, and two
or more `CANDIDATE` runs of the treatment arm. It answers three questions, in this order:

    promotion_gate.py check BASE CANDIDATE CANDIDATE [CANDIDATE ...] --declare FILE
                            [--track public|local] [--root DIR]

1. **Comparable?** The runs must be the same experiment but for one declared arm: the same binary
   (its SHA-256 recorded), models (compared by path until the header records hashes), scope, tracks,
   providers, `samples_data_sha`, corpus manifest and replay source; the same arms, environment,
   retry budget and pivot, except the declared treatment key, which must show the declared values;
   every document joined one to one on the image's SHA-256 (`archive_query.join_records`, then
   every pair must carry a `source_sha256` on both records and the two must be equal); and no
   budget-limited document unless the declaration makes the budget its subject. The candidate repeats are held to the same
   facts. Any failure is an error (exit 2) that lists every failing fact and gives no result.
2. **Safe?** Per joined document, base against each candidate run, any one vetoes: a Tier-1 hit
   lost; a new `false_positive_mrz`; a truth-backed field going from `exact` to anything else; an
   exact `names_exact` lost; a new `document_number_mismatch`; or the candidate runs disagreeing on
   a document's canonical record (the record without its timing keys, `TIMING_KEYS`).
3. **Effective?** The declared target metric, counted over the joined documents (or the listed
   assets), must improve by at least `min_effect`; with several candidate runs the smallest effect
   decides. The documents that moved are listed.

`--declare FILE` is a JSON file written before the candidate runs:

    {"schema": 1, "declared_utc": "2026-10-02T08:00:00Z",
     "treatment": {"key": "ocr_arms.chargrid", "base": "off", "candidate": "on"},
     "target": {"metric": "strict_names", "min_effect": 5, "assets": null},
     "budget_is_subject": false}

`treatment.key` is a dotted header path under `ocr_arms`, `mrz_arms`, `env`, `retry_budget` or
`pivot_yy`. `target.metric` is `hits`, `strict_names`, `mrz_found` or `field_exact` (which also
takes `"field": "<name>"`). The declaration must be dated earlier than every candidate's start.
Unknown, missing and mistyped keys are refused. The report prints the file's SHA-256.

Exit 0 `PROMOTABLE`; 3 `VETOED` or `BELOW TARGET`; 2 `REFUSED` (a usage, I/O, schema, declaration or
comparability problem). The last line of the output is the verdict with its counts.

**Disclosure** follows `archive_query.py`: run ids, asset ids, field names, outcome kinds, counts and
the declaration's hash. Never OCR text, a zone line, a field value, a `miss_reason` text, `argv` or a
model path. An error names a file and a line number, never a line's content, and never the name a
caller typed. Only the `public` and `local` tracks are read; `private/` never is.

Standard library only, plus `archive_query` (a sibling module) for the root, run loading and joins.
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path

import archive_query as aq

EXIT_PROMOTABLE = 0
EXIT_NOT_PROMOTABLE = 3
EXIT_REFUSED = 2
MOVER_LINE_CAP = aq.DOC_LINE_CAP
SCHEMA = 1

# The keys that differ between two runs of the same input and carry no semantics, spelled out: a
# dotted path into a provider record. A repeat that differs only here is the same record. No
# suffix or pattern rule: a new timing key is added here, with a test.
TIMING_KEYS = ("run_id", "read_us", "ledger_row.ocr_ms")

TREATMENT_ROOTS = ("ocr_arms", "mrz_arms", "env", "retry_budget", "pivot_yy")
# Header facts that must be equal on every run, besides the arms.
EQUAL_FACTS = ("scope", "tracks", "providers", "samples_data_sha", "corpus_manifest_sha256", "replay_of")
METRICS = ("hits", "strict_names", "mrz_found", "field_exact")
FALSE_POSITIVE = "false_positive_mrz"
DOCUMENT_NUMBER_MISMATCH = "document_number_mismatch"


class Refused(Exception):
    """A usage, I/O, schema, declaration or comparability problem: the report says why, exit 2."""

    def __init__(self, *lines: str):
        super().__init__("; ".join(lines))
        self.lines = list(lines)


# ------------------------------------------------------------------ the declaration


@dataclass
class Declaration:
    sha256: str
    declared_ms: int
    treatment_key: str
    treatment_base: object
    treatment_candidate: object
    metric: str
    field: str | None
    min_effect: int
    assets: list[str] | None
    budget_is_subject: bool


def _is_int(value: object) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _show(value: object) -> str:
    """A declared value as the declaration wrote it (a string bare, anything else as JSON)."""
    return value if isinstance(value, str) else json.dumps(value)


def _is_scalar(value: object) -> bool:
    return value is None or isinstance(value, (str, bool)) or _is_int(value)


def _exact_keys(label: str, value: object, required: set[str], optional: set[str] = frozenset()) -> dict:
    if not isinstance(value, dict):
        raise Refused(f"declaration: {label} must be an object")
    missing = sorted(required - set(value))
    if missing:
        raise Refused(f"declaration: {label} lacks the key {missing[0]!r}")
    unknown = sorted(set(value) - required - optional, key=str)
    if unknown:
        raise Refused(f"declaration: {label} has an unknown key {str(unknown[0])[:40]!r}")
    return value


def _parse_utc(text: object) -> int:
    if not isinstance(text, str):
        raise Refused("declaration: declared_utc must be a string like 2026-10-02T08:00:00Z")
    try:
        moment = datetime.strptime(text, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    except ValueError:
        raise Refused("declaration: declared_utc must look like 2026-10-02T08:00:00Z") from None
    return int(moment.timestamp() * 1000)


def load_declaration(path: Path) -> Declaration:
    try:
        raw = path.read_bytes()
    except OSError as exc:
        raise Refused(f"declaration: cannot read the --declare file ({type(exc).__name__})") from None
    sha = hashlib.sha256(raw).hexdigest()
    try:
        value = json.loads(raw.decode("utf-8"))
    except (ValueError, UnicodeDecodeError):
        raise Refused(f"declaration: the --declare file is not JSON (sha256 {sha})") from None
    top = _exact_keys("the declaration", value, {"schema", "declared_utc", "treatment", "target", "budget_is_subject"})
    if top["schema"] != SCHEMA or isinstance(top["schema"], bool):
        raise Refused(f"declaration: schema must be {SCHEMA}")
    declared_ms = _parse_utc(top["declared_utc"])
    if not isinstance(top["budget_is_subject"], bool):
        raise Refused("declaration: budget_is_subject must be true or false")
    treatment = _exact_keys("treatment", top["treatment"], {"key", "base", "candidate"})
    key = treatment["key"]
    if not isinstance(key, str) or not key:
        raise Refused("declaration: treatment.key must be a dotted header path")
    segments = key.split(".")
    if segments[0] not in TREATMENT_ROOTS or any(not segment for segment in segments):
        raise Refused(f"declaration: treatment.key must start with one of {', '.join(TREATMENT_ROOTS)}")
    if segments[0] == "pivot_yy" and len(segments) != 1:
        raise Refused("declaration: pivot_yy has no sub-keys")
    if not _is_scalar(treatment["base"]) or not _is_scalar(treatment["candidate"]):
        raise Refused("declaration: treatment.base and treatment.candidate must be a string, number, boolean or null")
    if treatment["base"] == treatment["candidate"] and type(treatment["base"]) is type(treatment["candidate"]):
        raise Refused("declaration: treatment.base and treatment.candidate are the same value, so nothing is treated")
    target = _exact_keys("target", top["target"], {"metric", "min_effect", "assets"}, {"field"})
    metric = target["metric"]
    if metric not in METRICS:
        raise Refused(f"declaration: target.metric must be one of {', '.join(METRICS)}")
    field = target.get("field")
    if metric == "field_exact":
        if not isinstance(field, str) or not field:
            raise Refused("declaration: target.metric field_exact needs target.field, a field name")
    elif "field" in target:
        raise Refused("declaration: target.field belongs to the field_exact metric only")
    if not _is_int(target["min_effect"]) or target["min_effect"] < 1:
        raise Refused("declaration: target.min_effect must be a whole number of at least 1")
    assets = target["assets"]
    if assets is not None:
        if not isinstance(assets, list) or not assets or not all(isinstance(a, str) and a for a in assets):
            raise Refused("declaration: target.assets must be null or a non-empty list of asset ids")
        if len(set(assets)) != len(assets):
            raise Refused("declaration: target.assets lists an asset id twice")
    return Declaration(
        sha256=sha, declared_ms=declared_ms, treatment_key=key, treatment_base=treatment["base"],
        treatment_candidate=treatment["candidate"], metric=metric, field=field if metric == "field_exact" else None,
        min_effect=target["min_effect"], assets=list(assets) if assets else None,
        budget_is_subject=top["budget_is_subject"],
    )


# ------------------------------------------------------------------ loading the runs


@dataclass
class Named:
    label: str  # "base", "candidate 1", ...
    run: aq.Run

    @property
    def tag(self) -> str:
        return f"{self.label} ({self.run.run_id[:12]})"


def _load(label: str, spec: str, root_arg: str | None, track: str) -> Named:
    """One run, by file or by `run_id` prefix. A message never repeats `spec`: it is what the caller
    typed, and may be a path."""
    candidate = Path(spec)
    try:
        if candidate.is_file():
            parent = candidate.parent.name
            if parent == "private":
                raise Refused(f"{label}: a file under private/ is never read")
            if parent in aq.TRACKS and parent != track:
                raise Refused(f"{label}: the file is in {parent}/, not the {track} track (use --track {parent})")
            run = aq.find_run(spec, Path("."), track)
        else:
            run = aq.find_run(spec, aq.resolve_root(root_arg), track)
    except aq.ArchiveError as exc:
        text = str(exc)
        if text.startswith("no finished run"):
            text = f"no finished run in {track}/ matches the name given"
        elif " runs in " in text and text.endswith("use a longer prefix"):
            text = f"the name given matches more than one run in {track}/; use a longer prefix"
        raise Refused(f"{label}: {text}") from None
    return Named(label, run)


def _require_real_provider_bench(named: Named) -> None:
    head = named.run.header
    scope = head.get("scope") if isinstance(head.get("scope"), dict) else {}
    synthetic = (
        head.get("binary_name") != "provider-bench"
        or scope.get("corpus") != "real-specimens"
        or any(aq._is_synthetic(r) or r.get("track") == "synthetic" for r in named.run.records)
    )
    if synthetic:
        raise Refused(
            f"{named.tag}: not a real-specimen provider-bench run; a synthetic run is calibration evidence "
            "only and is refused"
        )


# ------------------------------------------------------------------ header comparison


def _path_get(header: dict, key: str) -> object:
    node: object = header
    for segment in key.split("."):
        if not isinstance(node, dict):
            return None
        node = node.get(segment)
    return node


def _without_path(value: object, segments: list[str]) -> object:
    """A copy of `value` with the key at `segments` removed (a dict walk; a missing path is a no-op)."""
    value = copy.deepcopy(value)
    node = value
    for segment in segments[:-1]:
        if not isinstance(node, dict) or segment not in node:
            return value
        node = node[segment]
    if isinstance(node, dict):
        node.pop(segments[-1], None)
    return value


def _differing_keys(a: object, b: object, prefix: str) -> list[str]:
    if isinstance(a, dict) and isinstance(b, dict):
        return [f"{prefix}.{key}" for key in sorted(set(a) | set(b), key=str) if a.get(key) != b.get(key)]
    return [prefix]


def header_failures(base: Named, other: Named, decl: Declaration) -> list[str]:
    """Every header fact on which `other` differs from `base`, or fails the declaration. Names the
    field and the run; never a value (a model path or a hash is not printed)."""
    failures: list[str] = []
    a, b = base.run.header, other.run.header
    for named in (base, other):
        sha = named.run.header.get("binary_sha256")
        if not isinstance(sha, str) or not sha:
            failures.append(f"binary_sha256 is not recorded on {named.tag}")
    if a.get("binary_sha256") != b.get("binary_sha256"):
        failures.append(f"binary_sha256 differs: {other.tag} against {base.tag}")
    if a.get("model_paths") != b.get("model_paths"):
        failures.append(f"model_paths differ: {other.tag} against {base.tag}")
    for field in EQUAL_FACTS:
        if a.get(field) != b.get(field):
            for key in _differing_keys(a.get(field), b.get(field), field):
                failures.append(f"{key} differs: {other.tag} against {base.tag}")
    segments = decl.treatment_key.split(".")
    for field in aq.ARM_FIELDS:
        old, new = a.get(field), b.get(field)
        if segments[0] == field:
            old, new = _without_path(old, segments[1:]) if len(segments) > 1 else None, (
                _without_path(new, segments[1:]) if len(segments) > 1 else None
            )
        if old != new:
            for key in _differing_keys(old, new, field):
                failures.append(f"{key} differs and is not the declared treatment: {other.tag} against {base.tag}")
    if _path_get(a, decl.treatment_key) != decl.treatment_base:
        failures.append(f"{decl.treatment_key} on {base.tag} is not the declared base value")
    if _path_get(b, decl.treatment_key) != decl.treatment_candidate:
        failures.append(f"{decl.treatment_key} on {other.tag} is not the declared candidate value")
    return failures


# ------------------------------------------------------------------ records


def _row(record: dict) -> dict:
    return aq._ledger_row(record)


def _asset(record: dict) -> str:
    return str(record.get("asset_id") or record.get("name") or "?")


def _outcome(record: dict) -> str:
    return aq.outcome_of(record)


def _correctness(record: dict) -> dict | None:
    value = record.get("field_correctness")
    return value if isinstance(value, dict) else None


def metric_value(decl: Declaration, record: dict) -> bool:
    if decl.metric == "hits":
        return _outcome(record) == "hit"
    if decl.metric == "strict_names":
        return _row(record).get("names_exact") is True
    if decl.metric == "mrz_found":
        return _row(record).get("mrz_found") is True
    correctness = _correctness(record)
    return correctness is not None and correctness.get(decl.field) == "exact"


def canonical(record: dict) -> dict:
    """The record without its timing keys (`TIMING_KEYS`)."""
    value = copy.deepcopy(record)
    for path in TIMING_KEYS:
        segments = path.split(".")
        node = value
        for segment in segments[:-1]:
            node = node.get(segment) if isinstance(node, dict) else None
        if isinstance(node, dict):
            node.pop(segments[-1], None)
    return value


def differing_paths(a: object, b: object, prefix: str = "") -> list[str]:
    """The dotted key paths at which two canonical records differ; lists and scalars are compared
    whole. Paths only, never a value."""
    if isinstance(a, dict) and isinstance(b, dict):
        paths: list[str] = []
        for key in sorted(set(a) | set(b), key=str):
            if key in a and key in b:
                paths.extend(differing_paths(a[key], b[key], f"{prefix}.{key}" if prefix else str(key)))
            else:
                paths.append(f"{prefix}.{key}" if prefix else str(key))
        return paths
    return [] if a == b else [prefix or "(record)"]


@dataclass
class Veto:
    kind: str
    key: str
    detail: str
    run: str

    def line(self) -> str:
        detail = f" {self.detail}" if self.detail else ""
        return f"  veto {self.kind}: {self.key}{detail} (candidate {self.run})"


def pair_vetoes(pair: aq.Pair, run_tag: str) -> list[Veto]:
    old, new = pair.a, pair.b
    found: list[Veto] = []
    if _outcome(old) == "hit" and _outcome(new) != "hit":
        found.append(Veto("hit_lost", pair.key, "", run_tag))
    for kind in (FALSE_POSITIVE, DOCUMENT_NUMBER_MISMATCH):
        if _outcome(new) == kind and _outcome(old) != kind:
            found.append(Veto(kind, pair.key, "", run_tag))
    before = _correctness(old)
    if before is not None:
        after = _correctness(new) or {}
        for field in sorted(before, key=str):
            if before[field] == "exact" and after.get(field) != "exact":
                found.append(Veto("field_regressed", pair.key, f"field={field}", run_tag))
    if _row(old).get("names_exact") is True and _row(new).get("names_exact") is not True:
        found.append(Veto("names_exact_lost", pair.key, "", run_tag))
    return found


# ------------------------------------------------------------------ the check


def _capped(lines: list[str], total: int | None = None) -> list[str]:
    total = len(lines) if total is None else total
    shown = lines[:MOVER_LINE_CAP]
    if total > len(shown):
        shown.append(f"    ... {total - len(shown)} more")
    return shown


def _is_sha(value: object) -> bool:
    return isinstance(value, str) and bool(value)


def _join(left: Named, right: Named) -> tuple[list[aq.Pair], list[str]]:
    pairs, only_left, only_right = aq.join_records(left.run.records, right.run.records)
    failures = []
    if only_left:
        ids = ", ".join(r["_display"] for r in only_left[:MOVER_LINE_CAP])
        failures.append(
            f"documents do not join one to one: {len(only_left)} only in {left.tag} against {right.tag}: {ids}"
        )
    if only_right:
        ids = ", ".join(r["_display"] for r in only_right[:MOVER_LINE_CAP])
        failures.append(
            f"documents do not join one to one: {len(only_right)} only in {right.tag} against {left.tag}: {ids}"
        )
    # The join falls back to the asset id and the name, so a changed or unrecorded image still
    # pairs. The image's SHA-256 is compared by value, never by how the pair was found.
    unmatched = sorted(
        {p.key for p in pairs if not _is_sha(p.a.get("source_sha256")) or p.a.get("source_sha256") != p.b.get("source_sha256")}
    )
    if unmatched:
        ids = ", ".join(unmatched[:MOVER_LINE_CAP])
        more = f" (and {len(unmatched) - MOVER_LINE_CAP} more)" if len(unmatched) > MOVER_LINE_CAP else ""
        failures.append(
            f"source_sha256 differs or is missing: {len(unmatched)} document(s) of {right.tag} against {left.tag}: {ids}{more}"
        )
    return pairs, failures


def run_check(base_spec: str, candidate_specs: list[str], declare: Path, root_arg: str | None, track: str, out) -> int:
    emit = lambda text="": print(text, file=out)  # noqa: E731
    emit("promotion gate: check")
    try:
        decl = load_declaration(declare)
    except Refused as exc:
        for line in exc.lines:
            emit(f"refused: {line}")
        emit(f"REFUSED ({len(exc.lines)} problem)")
        return EXIT_REFUSED
    emit(f"declaration sha256 {decl.sha256}")
    emit(f"treatment {decl.treatment_key}: {_show(decl.treatment_base)} -> {_show(decl.treatment_candidate)}")
    try:
        if len(candidate_specs) < 2:
            raise Refused("at least two candidate runs are needed: the determinism veto compares the repeats")
        base = _load("base", base_spec, root_arg, track)
        candidates = [_load(f"candidate {i + 1}", spec, root_arg, track) for i, spec in enumerate(candidate_specs)]
        everyone = [base, *candidates]
        if len({n.run.run_id for n in everyone}) != len(everyone):
            raise Refused("the same run is named twice: the base and the repeats must be distinct runs")
        for named in everyone:
            _require_real_provider_bench(named)
    except Refused as exc:
        for line in exc.lines:
            emit(f"refused: {line}")
        emit(f"REFUSED ({len(exc.lines)} problem)")
        return EXIT_REFUSED

    # ---- comparability: every failing fact, then no result
    failures: list[str] = []
    for named in candidates:
        failures.extend(header_failures(base, named, decl))
        started = named.run.header.get("started_unix_ms")
        if not _is_int(started):
            failures.append(f"started_unix_ms is not recorded on {named.tag}")
        elif decl.declared_ms >= started:
            failures.append(f"declared_utc is not earlier than the start of {named.tag}")
    joins: dict[str, list[aq.Pair]] = {}
    for named in candidates:
        pairs, join_failures = _join(base, named)
        joins[named.label] = pairs
        failures.extend(join_failures)
    if not decl.budget_is_subject:
        for named in everyone:
            limited = sorted({_asset(r) for r in named.run.records if _row(r).get("retry_budget_hit")})
            if limited:
                shown = ", ".join(limited[:MOVER_LINE_CAP])
                failures.append(f"retry_budget_hit on {len(limited)} document(s) of {named.tag}: {shown}")
    if decl.assets is not None and not failures:
        known = {_asset(p.a) for p in joins[candidates[0].label]}
        for asset in decl.assets:
            if asset not in known:
                failures.append(f"the declared asset {asset} is not in the runs")
    if failures:
        for line in failures:
            emit(f"refused: {line}")
        emit(f"REFUSED ({len(failures)} comparability failure(s))")
        return EXIT_REFUSED
    emit("comparable: yes")
    emit("models: compared by path; the header records no hashes yet, so a replaced file under one path is not detected")

    # ---- safety vetoes
    vetoes: list[Veto] = []
    for named in candidates:
        for pair in joins[named.label]:
            vetoes.extend(pair_vetoes(pair, named.run.run_id[:12]))
    first = candidates[0]
    for named in candidates[1:]:
        for pair in _join(first, named)[0]:
            paths = differing_paths(canonical(pair.a), canonical(pair.b))
            if paths:
                vetoes.append(Veto("candidate_runs_disagree", pair.key, "keys=" + ",".join(paths[:8]),
                                   f"{first.run.run_id[:12]} and {named.run.run_id[:12]}"))
    if vetoes:
        counts: dict[str, int] = {}
        for veto in vetoes:
            counts[veto.kind] = counts.get(veto.kind, 0) + 1
        emit(f"vetoes: {len(vetoes)} (" + ", ".join(f"{k} {n}" for k, n in sorted(counts.items())) + ")")
        for veto in vetoes[:MOVER_LINE_CAP]:
            emit(veto.line())
        if len(vetoes) > MOVER_LINE_CAP:
            emit(f"    ... {len(vetoes) - MOVER_LINE_CAP} more")
    else:
        emit("vetoes: none")

    # ---- declared efficacy
    selected = decl.assets
    movers: dict[str, list[str]] = {}
    efforts: list[tuple[str, int, int, int]] = []
    for named in candidates:
        pairs = [p for p in joins[named.label] if selected is None or _asset(p.a) in selected]
        base_count = sum(metric_value(decl, p.a) for p in pairs)
        cand_count = sum(metric_value(decl, p.b) for p in pairs)
        efforts.append((named.run.run_id[:12], base_count, cand_count, cand_count - base_count))
        movers[named.run.run_id[:12]] = [
            f"    {p.key}: {str(metric_value(decl, p.a)).lower()} -> {str(metric_value(decl, p.b)).lower()}"
            for p in pairs
            if metric_value(decl, p.a) != metric_value(decl, p.b)
        ]
    smallest = min(effect for *_rest, effect in efforts)
    scope_text = "every joined document" if selected is None else f"{len(selected)} listed asset(s)"
    metric_text = decl.metric if decl.field is None else f"{decl.metric} {decl.field}"
    emit(f"target {metric_text}, min effect {decl.min_effect}, over {scope_text}")
    for run_id, base_count, cand_count, effect in efforts:
        emit(f"  run {run_id}: base {base_count}, candidate {cand_count}, effect {effect:+d}")
    emit(f"  smallest effect {smallest:+d} (min effect {decl.min_effect})")
    emit("movers (before -> after):")
    for run_id, lines in movers.items():
        emit(f"  run {run_id}: {len(lines)} moved")
        for line in _capped(lines):
            emit(line)

    met = smallest >= decl.min_effect
    tail = f"{metric_text} effect {smallest:+d}"
    runs = f"{len(candidates)} candidate runs"
    if vetoes:
        emit(f"VETOED ({len(vetoes)} vetoes; {tail} {'>=' if met else '<'} {decl.min_effect}; {runs})")
        return EXIT_NOT_PROMOTABLE
    if not met:
        emit(f"BELOW TARGET ({tail} < {decl.min_effect}; 0 vetoes; {runs})")
        return EXIT_NOT_PROMOTABLE
    emit(f"PROMOTABLE ({tail} >= {decl.min_effect}; 0 vetoes; {runs})")
    return EXIT_PROMOTABLE


# ------------------------------------------------------------------ main


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0] if __doc__ else None)
    sub = parser.add_subparsers(dest="command", required=True)
    check = sub.add_parser("check", help="gate two or more candidate runs against a base run")
    check.add_argument("base", metavar="BASE", help="a run_id prefix or a file path: the default arm")
    check.add_argument("candidates", metavar="CANDIDATE", nargs="+",
                       help="a run_id prefix or a file path: the treatment arm, at least twice")
    check.add_argument("--declare", required=True, metavar="FILE", help="the declaration, written before the candidate runs")
    check.add_argument("--track", choices=aq.TRACKS, default="public", help="which track to read (default: public)")
    check.add_argument("--root", help="the archive root (default: SYNTHPASS_BENCH_ARCHIVE, else the git common dir's archive)")
    return parser


def main(argv: list[str] | None = None, out=None) -> int:
    out = sys.stdout if out is None else out
    args = build_parser().parse_args(argv)
    return run_check(args.base, args.candidates, Path(args.declare), args.root, args.track, out)


if __name__ == "__main__":
    sys.exit(main())
