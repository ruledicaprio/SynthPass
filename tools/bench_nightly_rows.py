#!/usr/bin/env python3
"""Turn the nightly `synthpass-bench` reports into the rows published on `bench-data`.

Stdlib only. ADR-0027 decision 1: the published dataset is an allowlisted projection of
`synthpass-bench`'s report, never the report itself. This tool is that projection and the one
reader of what it writes.

    next-seed  the first fresh seed for one format (measure job, reads `bench-data`)
    context    the runner facts a run header records (measure job)
    assemble   every downloaded measurement -> new `fresh.jsonl`, `fixed.jsonl`, `runs.jsonl` lines
    guard      refuse a `fresh.jsonl` that repeats a (document_type, seed, profile)

What a row holds, and what it never holds:

  * A row carries identity, outcome classes and numbers: `hit`, the miss kind, the check-digit
    states, per-field CER, the name-error class, the line-1 flag, the retry stop and variant, the
    two damaged-recovery flags, and the pass count with each pass's id and outcome. Every value is
    checked to be a number, a boolean, or a short token from a fixed alphabet, so no text can
    travel in an allowed key.
  * It never carries `reason`, a field's expected or read value, OCR text, a pass's readings, or
    zone text. Those keys are not in the allowlist, and `validate_row` refuses a row that has one.
  * The derived flags (`wrong_accept`, `names_exact`, `accepted_read`, the prefix counts) are not
    stored. `derive` recomputes them from a row, and `extract_run` asserts that the recomputed
    totals equal the report's own top-level counts: a mismatch raises, and the job fails.

Schema 1 is `dataset.jsonl`, frozen. `read_rows` reads it: a row with no `schema` key is schema 1,
its `reason` maps to a miss kind by prefix (and is dropped), and every other schema-2 key reads as
`None`, meaning not measured, never `False` or `0`.

Privacy: the report and the context are read as untrusted input. Nothing here prints a report
value; an error names the seed and the key, not the value.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

SCHEMA = 2

FORMATS = ("td1", "td2", "td3", "mrva", "mrvb")
SLICES = ("fresh", "fixed")

# `synthpass-bench`'s `--profile all` round-robin, plus the two names a report can also carry.
PROFILES = ("clean", "mobile", "scanner", "worn", "border-kiosk", "damaged")

# The fixed slice is the synthetic-headline invocation (ADR-0027 decision 1): clean, seeds 0-99.
FIXED_PROFILE = "clean"
FIXED_SEED_START = 0
FIXED_COUNT = 100

# The fresh slice profile, and the lowest seed it may start at (the fixed slice owns 0-99).
FRESH_PROFILE = "all"
MIN_FRESH_SEED = FIXED_SEED_START + FIXED_COUNT

# `synthpass_bench::COMPARED_FIELDS` (the 12 ICAO-scored fields) plus the diagnostic `mrz_lines`.
SCORED_FIELDS = (
    "document_type",
    "issuing_country",
    "document_number",
    "surname",
    "given_names",
    "nationality",
    "date_of_birth",
    "sex",
    "date_of_expiry",
    "personal_number",
    "optional_data_1",
    "optional_data_2",
)
DIAGNOSTIC_FIELD = "mrz_lines"
FIELD_NAMES = SCORED_FIELDS + (DIAGNOSTIC_FIELD,)
# The line-1 prefix no check digit covers in any format (issue #453).
PREFIX_FIELDS = ("document_type", "issuing_country")

CHECK_STATE_KEYS = (
    "document_number",
    "date_of_birth",
    "date_of_expiry",
    "personal_number",
    "composite",
)

# Every key of a schema-2 row, in the order it is written. Adding a key is a schema decision:
# update ADR-0027's reading rule and PIPELINE.md 2.5 with it.
ROW_KEYS = (
    "schema",
    "run_id",
    "slice",
    "document_type",
    "seed",
    "profile",
    "render_sha256",
    "hit",
    "miss_kind",
    "check_states",
    "field_cer",
    "name_error",
    "line1_flagged",
    "retry_stop",
    "retry_variant_id",
    "retry_damaged_recovery",
    "tier1_damaged_recovery",
    "ocr_pass_count",
    "ocr_passes",
    "elapsed_ms",
)

# Schema 1: the seven keys `dataset.jsonl` rows carry.
V1_KEYS = (
    "run_timestamp_unix",
    "git_sha",
    "seed",
    "profile",
    "hit",
    "reason",
    "elapsed_ms",
)
# Keys a normalized schema-1 row keeps; `reason` is mapped to `miss_kind` and dropped.
V1_KEPT_KEYS = ("run_timestamp_unix", "git_sha", "seed", "profile", "hit", "elapsed_ms")
# `reason` prefixes across the dataset's epochs. A non-hit whose reason matches none of them (the
# four Debug-struct rows of 2026-07-21) has no miss kind: unknown, not invented.
V1_REASON_PREFIXES = (
    ("checksum invalid", "checksum_failed"),
    ("document number mismatch", "document_number_mismatch"),
    ("no MRZ found", "no_mrz_found"),
)

# The report's top-level counts the recomputed totals must equal.
COUNT_KEYS = (
    "count",
    "hits",
    "strict_hits",
    "wrong_accepts",
    "accepted_reads",
    "prefix_wrong_accepts",
    "prefix_wrong_accepted_reads",
)

RUN_HEADER_FILE = "runs.jsonl"

_HEX64 = re.compile(r"^[0-9a-f]{64}$")
_HEX40 = re.compile(r"^[0-9a-f]{40}$")
# A class label or a pass id: `general_valid`, `pass-07`, `mrz_variants:2`. No spaces, no `<`.
_TOKEN = re.compile(r"^[A-Za-z0-9_.:-]{1,40}$")
# `<GITHUB_RUN_ID>-<GITHUB_RUN_ATTEMPT>`
_RUN_ID = re.compile(r"^[0-9]{1,20}-[0-9]{1,4}$")
_PRINTABLE = re.compile(r"^[ -~]{1,120}$")
_ENV_NAME = re.compile(r"^SYNTHPASS_[A-Z0-9_]{1,60}$")


class RowError(ValueError):
    """A report, a row or a context broke the schema. The message names keys, never values."""


# --- small validators -------------------------------------------------------------------------


def _is_int(value) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _need(mapping: dict, key: str, where: str):
    if key not in mapping:
        raise RowError(f"{where}: missing key {key!r}")
    return mapping[key]


def _check_bool(value, key: str, where: str, *, nullable: bool = False) -> None:
    if value is None and nullable:
        return
    if not isinstance(value, bool):
        raise RowError(f"{where}: {key} must be a boolean" + (" or null" if nullable else ""))


def _check_int(value, key: str, where: str, *, nullable: bool = False, minimum: int = 0) -> None:
    if value is None and nullable:
        return
    if not _is_int(value) or value < minimum:
        raise RowError(f"{where}: {key} must be an integer >= {minimum}" + (" or null" if nullable else ""))


def _check_token(value, key: str, where: str, *, nullable: bool = False) -> None:
    if value is None and nullable:
        return
    if not isinstance(value, str) or not _TOKEN.match(value):
        raise RowError(f"{where}: {key} must be a short token" + (" or null" if nullable else ""))


def _check_enum(value, key: str, where: str, allowed: tuple[str, ...]) -> None:
    if value not in allowed:
        raise RowError(f"{where}: {key} is not one of the known values")


# --- the row ----------------------------------------------------------------------------------


def validate_row(row: dict) -> dict:
    """Refuse a schema-2 row that is not exactly the allowlist with allowlisted values.

    Any key outside `ROW_KEYS` is refused by name (`reason`, `fields`, `ocr_text`, ... are the ones
    this exists to stop), and so is any value that could carry text. Returns the row.
    """
    if not isinstance(row, dict):
        raise RowError("a row must be a JSON object")
    where = f"seed {row.get('seed')!r}" if _is_int(row.get("seed")) else "row"
    extra = sorted(set(row) - set(ROW_KEYS))
    if extra:
        raise RowError(f"{where}: keys outside the allowlist: {', '.join(map(str, extra))}")
    missing = [key for key in ROW_KEYS if key not in row]
    if missing:
        raise RowError(f"{where}: missing keys: {', '.join(missing)}")

    if row["schema"] != SCHEMA or isinstance(row["schema"], bool):
        raise RowError(f"{where}: schema must be {SCHEMA}")
    if not isinstance(row["run_id"], str) or not _RUN_ID.match(row["run_id"]):
        raise RowError(f"{where}: run_id must be <run id>-<attempt>")
    _check_enum(row["slice"], "slice", where, SLICES)
    _check_enum(row["document_type"], "document_type", where, FORMATS)
    _check_int(row["seed"], "seed", where)
    _check_enum(row["profile"], "profile", where, PROFILES)
    if not isinstance(row["render_sha256"], str) or not _HEX64.match(row["render_sha256"]):
        raise RowError(f"{where}: render_sha256 must be 64 lowercase hex characters")
    _check_bool(row["hit"], "hit", where)
    _check_token(row["miss_kind"], "miss_kind", where, nullable=True)
    if row["hit"] != (row["miss_kind"] is None):
        raise RowError(f"{where}: a row is a hit exactly when its miss_kind is null")
    _validate_check_states(row["check_states"], where)
    _validate_field_cer(row["field_cer"], where)
    _check_token(row["name_error"], "name_error", where, nullable=True)
    _check_bool(row["line1_flagged"], "line1_flagged", where)
    _check_token(row["retry_stop"], "retry_stop", where, nullable=True)
    _check_token(row["retry_variant_id"], "retry_variant_id", where, nullable=True)
    _check_bool(row["retry_damaged_recovery"], "retry_damaged_recovery", where, nullable=True)
    _check_bool(row["tier1_damaged_recovery"], "tier1_damaged_recovery", where, nullable=True)
    _validate_passes(row["ocr_pass_count"], row["ocr_passes"], where)
    _check_int(row["elapsed_ms"], "elapsed_ms", where)
    return row


def _validate_check_states(states, where: str) -> None:
    if states is None:
        return
    if not isinstance(states, dict) or set(states) - set(CHECK_STATE_KEYS):
        raise RowError(f"{where}: check_states keys must be among the five check digits")
    for key, state in states.items():
        _check_bool(state, f"check_states.{key}", where, nullable=True)


def _validate_field_cer(cer, where: str) -> None:
    if cer is None:
        return
    if not isinstance(cer, dict) or not cer or set(cer) - set(FIELD_NAMES):
        raise RowError(f"{where}: field_cer must be a non-empty map over the scored field names")
    for key, value in cer.items():
        if isinstance(value, bool) or not isinstance(value, (int, float)):
            raise RowError(f"{where}: field_cer.{key} must be a number")
        if not math.isfinite(value) or value < 0:
            raise RowError(f"{where}: field_cer.{key} must be finite and >= 0")


def _validate_passes(count, passes, where: str) -> None:
    if count is None and passes is None:
        return
    if not isinstance(passes, list) or not _is_int(count) or count != len(passes):
        raise RowError(f"{where}: ocr_pass_count must equal the length of ocr_passes")
    for entry in passes:
        if not isinstance(entry, dict) or set(entry) != {"id", "outcome"}:
            raise RowError(f"{where}: each pass is exactly {{id, outcome}}")
        _check_token(entry["id"], "ocr_passes.id", where)
        _check_token(entry["outcome"], "ocr_passes.outcome", where)


# --- projection from a report -----------------------------------------------------------------


def project_result(result: dict, *, slice_name: str, document_type: str, run_id: str) -> dict:
    """Build one validated row from one `results[]` entry of a `synthpass-bench` report.

    Only the keys named below are read. Everything else in the entry, `reason`, `fields[].expected`
    and `got`, `ocr_text` and each pass's readings included, is never copied.
    """
    if not isinstance(result, dict):
        raise RowError("a results[] entry must be a JSON object")
    where = f"seed {result.get('seed')!r}"

    fields = result.get("fields")
    field_cer = None
    if fields:
        if not isinstance(fields, list) or not all(isinstance(f, dict) for f in fields):
            raise RowError(f"{where}: fields must be a list of objects")
        field_cer = {_need(f, "field", where): _need(f, "cer", where) for f in fields}

    pass_count, passes = None, None
    if "ocr_passes" in result:
        raw = result["ocr_passes"]
        if not isinstance(raw, list) or not all(isinstance(p, dict) for p in raw):
            raise RowError(f"{where}: ocr_passes must be a list of objects")
        pass_count = len(raw)
        passes = [{"id": _need(p, "id", where), "outcome": _need(p, "outcome", where)} for p in raw]

    row = {
        "schema": SCHEMA,
        "run_id": run_id,
        "slice": slice_name,
        "document_type": document_type,
        "seed": _need(result, "seed", where),
        "profile": _need(result, "profile", where),
        "render_sha256": result.get("render_sha256"),
        "hit": _need(result, "hit", where),
        "miss_kind": result.get("miss_kind"),
        "check_states": result.get("check_states"),
        "field_cer": field_cer,
        "name_error": result.get("name_error"),
        "line1_flagged": _need(result, "line1_flagged", where),
        "retry_stop": result.get("retry_stop"),
        "retry_variant_id": result.get("retry_variant_id"),
        "retry_damaged_recovery": result.get("retry_damaged_recovery"),
        "tier1_damaged_recovery": result.get("tier1_damaged_recovery"),
        "ocr_pass_count": pass_count,
        "ocr_passes": passes,
        "elapsed_ms": _need(result, "elapsed_ms", where),
    }
    return validate_row(row)


# --- derived flags ----------------------------------------------------------------------------


def _and(a, b):
    """Three-valued AND: `None` is unknown, and `False` wins over it."""
    if a is False or b is False:
        return False
    if a is None or b is None:
        return None
    return True


def derive(row: dict) -> dict:
    """The flags a row does not store, recomputed from the keys it does.

    Each is `True`, `False`, or `None` when the row lacks what the flag needs (a schema-1 row).
    The rules are the report's own:
      * accepted read: a hit, or a `document_number_mismatch` miss (its check digits verified);
      * wrong fields: scored fields (not `mrz_lines`) with CER above 0;
      * wrong accept: a hit with at least one wrong field;
      * prefix wrong accept / accepted read: the same, with `document_type` or `issuing_country`
        among the wrong fields, over hits / over accepted reads;
      * names exact: an MRZ parsed (`check_states` present) and no name error. Schema 2 only.
    """
    hit = row["hit"]
    kind = row["miss_kind"]
    if hit:
        accepted = True
    elif kind is None:
        accepted = None
    else:
        accepted = kind == "document_number_mismatch"

    cer = row["field_cer"]
    wrong = None if cer is None else [f for f, v in cer.items() if f != DIAGNOSTIC_FIELD and v > 0]
    prefix_wrong = None if wrong is None else any(f in PREFIX_FIELDS for f in wrong)
    has_wrong = None if wrong is None else bool(wrong)

    if row["schema"] == SCHEMA:
        names_exact = row["check_states"] is not None and row["name_error"] is None
    else:
        names_exact = None

    return {
        "accepted_read": accepted,
        "wrong_fields": wrong,
        "wrong_accept": _and(hit, has_wrong),
        "prefix_wrong_accept": _and(_and(hit, has_wrong), prefix_wrong),
        "prefix_wrong_accepted_read": _and(accepted, prefix_wrong),
        "names_exact": names_exact,
        "strict_hit": _and(hit, names_exact),
    }


def totals(rows: list[dict]) -> dict:
    """The report's top-level counts, recomputed from schema-2 rows, plus the seed lists."""
    flags = [(row["seed"], derive(row)) for row in rows]
    return {
        "count": len(rows),
        "hits": sum(1 for row in rows if row["hit"]),
        "strict_hits": sum(1 for _, f in flags if f["strict_hit"] is True),
        "wrong_accepts": sum(1 for _, f in flags if f["wrong_accept"] is True),
        "accepted_reads": sum(1 for _, f in flags if f["accepted_read"] is True),
        "prefix_wrong_accepts": sum(1 for _, f in flags if f["prefix_wrong_accept"] is True),
        "prefix_wrong_accepted_reads": sum(
            1 for _, f in flags if f["prefix_wrong_accepted_read"] is True
        ),
        "prefix_wrong_accept_seeds": sorted(s for s, f in flags if f["prefix_wrong_accept"] is True),
        "prefix_wrong_accepted_read_seeds": sorted(
            s for s, f in flags if f["prefix_wrong_accepted_read"] is True
        ),
    }


def assert_counts_match(rows: list[dict], report: dict) -> dict:
    """Raise unless the rows' recomputed totals equal the report's own counts. Returns them.

    This catches drift between the binary and this reader, and a row the projection got wrong. It
    compares numbers and seed lists only.
    """
    recomputed = totals(rows)
    mismatched = []
    for key in COUNT_KEYS:
        if report.get(key) != recomputed[key]:
            mismatched.append(f"{key}: report {report.get(key)!r}, rows {recomputed[key]!r}")
    # The report omits an empty seed list.
    for key in ("prefix_wrong_accept_seeds", "prefix_wrong_accepted_read_seeds"):
        if sorted(report.get(key) or []) != recomputed[key]:
            mismatched.append(f"{key}: report and rows name different seeds")
    if mismatched:
        raise RowError("recomputed counts differ from the report's: " + "; ".join(mismatched))
    return recomputed


# --- the run header ---------------------------------------------------------------------------


def generator_fingerprint(rows: list[dict]) -> str:
    """One hash over a run's `(seed, render_sha256)` pairs, seed order.

    Two nights whose fixed slices hash the same rendered the same 100 documents, whatever changed
    around them; any re-render, from the generator or a degrade profile, changes it.
    """
    digest = hashlib.sha256()
    for row in sorted(rows, key=lambda r: r["seed"]):
        digest.update(f"{row['seed']}:{row['render_sha256']}\n".encode("ascii"))
    return digest.hexdigest()


CONTEXT_KEYS = (
    "run_id",
    "git_sha",
    "event",
    "cpu_model",
    "nproc",
    "rustc",
    "model_sha256",
    "ocr_env",
)


def validate_context(context: dict) -> dict:
    """Refuse a runner-facts file that is not exactly `CONTEXT_KEYS` with short, plain values."""
    if not isinstance(context, dict) or set(context) != set(CONTEXT_KEYS):
        raise RowError("context.json must hold exactly: " + ", ".join(CONTEXT_KEYS))
    if not isinstance(context["run_id"], str) or not _RUN_ID.match(context["run_id"]):
        raise RowError("context: run_id must be <run id>-<attempt>")
    if not isinstance(context["git_sha"], str) or not _HEX40.match(context["git_sha"]):
        raise RowError("context: git_sha must be 40 lowercase hex characters")
    _check_token(context["event"], "event", "context")
    for key in ("cpu_model", "rustc"):
        if not isinstance(context[key], str) or not _PRINTABLE.match(context[key]):
            raise RowError(f"context: {key} must be short printable ASCII")
    _check_int(context["nproc"], "nproc", "context", minimum=1)
    models = context["model_sha256"]
    if not isinstance(models, dict) or set(models) != {"detection", "recognition"}:
        raise RowError("context: model_sha256 must be {detection, recognition}")
    for key, value in models.items():
        if not isinstance(value, str) or not _HEX64.match(value):
            raise RowError(f"context: model_sha256.{key} must be 64 lowercase hex characters")
    env = context["ocr_env"]
    if not isinstance(env, dict):
        raise RowError("context: ocr_env must be an object")
    for name, value in env.items():
        if not _ENV_NAME.match(name) or not isinstance(value, str) or not re.match(r"^[A-Za-z0-9_.-]{0,40}$", value):
            raise RowError("context: ocr_env holds only SYNTHPASS_* names with short plain values")
    return context


def _env_int(env: dict, name: str):
    value = env.get(name)
    return int(value) if value is not None and value.isdigit() else None


def build_header(report: dict, rows: list[dict], context: dict, slice_name: str, counts: dict) -> dict:
    """One `runs.jsonl` line: the conditions this run measured under (ADR-0027 decision 2)."""
    arms = _need(report, "ocr_arms", "report")
    if not isinstance(arms, dict):
        raise RowError("report: ocr_arms must be an object")
    arm_values = {}
    for key in ("texture", "order", "rotate", "skew", "chargrid"):
        value = _need(arms, key, "report.ocr_arms")
        if not isinstance(value, str) or not re.match(r"^[a-z0-9-]{1,16}$", value):
            raise RowError(f"report: ocr_arms.{key} must be a short lowercase label")
        arm_values[key] = value
    mrz_arms = {}
    for key in ("mrz_class_sweep_arm", "mrz_line1_select_arm"):
        value = report.get(key)
        if value is not None and (not isinstance(value, str) or not re.match(r"^[a-z0-9-]{1,16}$", value)):
            raise RowError(f"report: {key} must be a short lowercase label")
        mrz_arms[key] = value
    timestamp = _need(report, "timestamp_unix", "report")
    _check_int(timestamp, "timestamp_unix", "report")
    env = context["ocr_env"]
    return {
        "schema": SCHEMA,
        "run_id": context["run_id"],
        "slice": slice_name,
        "document_type": rows[0]["document_type"],
        "git_sha": context["git_sha"],
        "event": context["event"],
        "run_timestamp_unix": timestamp,
        "generator_fingerprint": generator_fingerprint(rows) if slice_name == "fixed" else None,
        "invocation": {
            "document_type": rows[0]["document_type"],
            "profile": report["profile"],
            "count": report["count"],
            "seed_start": report["seed_start"],
            "ocr_passes": rows[0]["ocr_passes"] is not None,
        },
        "ocr_arms": arm_values,
        **mrz_arms,
        "ocr_env": dict(sorted(env.items())),
        # The report does not carry the budget. These are the runner's own override, `None` when
        # the variable was unset, meaning the binary's built-in default.
        "max_passes": _env_int(env, "SYNTHPASS_OCR_MAX_PASSES"),
        "max_seconds": _env_int(env, "SYNTHPASS_OCR_MAX_SECONDS"),
        "model_sha256": context["model_sha256"],
        "cpu_model": context["cpu_model"],
        "nproc": context["nproc"],
        "rustc": context["rustc"],
        "counts": {
            **{key: counts[key] for key in COUNT_KEYS},
            "budget_stops": sum(1 for row in rows if row["retry_stop"] == "budget"),
        },
    }


# --- one run ----------------------------------------------------------------------------------


def extract_run(report: dict, slice_name: str, context: dict) -> tuple[list[dict], dict]:
    """The rows and the run header of one report. Raises `RowError` on anything off-schema."""
    if slice_name not in SLICES:
        raise RowError(f"unknown slice {slice_name!r}")
    if not isinstance(report, dict) or not isinstance(report.get("results"), list):
        raise RowError("report: results must be a list")
    document_type = str(report.get("document_type", "")).lower()
    if document_type not in FORMATS:
        raise RowError("report: document_type is not one of the five formats")
    profile, seed_start, count = (report.get(k) for k in ("profile", "seed_start", "count"))
    _check_int(seed_start, "seed_start", "report")
    _check_int(count, "count", "report")
    if slice_name == "fixed":
        if (profile, seed_start, count) != (FIXED_PROFILE, FIXED_SEED_START, FIXED_COUNT):
            raise RowError(
                f"report: the fixed slice is --profile {FIXED_PROFILE} --count {FIXED_COUNT} "
                f"--seed {FIXED_SEED_START}"
            )
    else:
        if profile != FRESH_PROFILE or seed_start < MIN_FRESH_SEED:
            raise RowError(
                f"report: the fresh slice is --profile {FRESH_PROFILE} from seed {MIN_FRESH_SEED} up"
            )
    if not report["results"]:
        raise RowError("report: no results")

    context = validate_context(context)
    rows = [
        project_result(r, slice_name=slice_name, document_type=document_type, run_id=context["run_id"])
        for r in report["results"]
    ]
    counts = assert_counts_match(rows, report)
    return rows, build_header(report, rows, context, slice_name, counts)


# --- duplicates, reading and writing ----------------------------------------------------------


def find_duplicates(rows: list[dict]) -> list[tuple]:
    """The `(document_type, seed, profile)` keys that occur more than once.

    Applied to the fresh slice only: a fresh document is measured once, ever. In the fixed slice a
    repeat is the design.
    """
    seen, dupes = set(), set()
    for row in rows:
        key = (row["document_type"], row["seed"], row["profile"])
        (dupes if key in seen else seen).add(key)
    return sorted(dupes)


def normalize_v1(raw: dict) -> dict:
    """A frozen `dataset.jsonl` row, read under the schema-1 rule (ADR-0027 decision 1)."""
    extra = sorted(set(raw) - set(V1_KEYS))
    if extra:
        raise RowError(f"schema-1 row: keys outside the frozen set: {', '.join(map(str, extra))}")
    reason = raw.get("reason")
    kind = None
    if not raw.get("hit") and isinstance(reason, str):
        kind = next((k for prefix, k in V1_REASON_PREFIXES if reason.startswith(prefix)), None)
    row = {key: None for key in ROW_KEYS}
    row.update({key: raw.get(key) for key in V1_KEPT_KEYS})
    row["schema"] = 1
    row["miss_kind"] = kind
    return row


def read_rows(path: Path) -> list[dict]:
    """Every row of a `bench-data` file as one shape: schema 2 validated, schema 1 normalized.

    A row with no `schema` key is schema 1. Its `reason` becomes `miss_kind` and is dropped;
    every schema-2 key it never had reads as `None`. A schema-2 row that carries a key outside the
    allowlist is refused, so a file that ever held text cannot be read back as if it were clean.
    """
    rows = []
    for number, line in enumerate(Path(path).read_text(encoding="utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        try:
            raw = json.loads(line)
        except json.JSONDecodeError as error:
            raise RowError(f"{path}:{number}: not JSON ({error.msg})") from error
        if not isinstance(raw, dict):
            raise RowError(f"{path}:{number}: a row must be a JSON object")
        schema = raw.get("schema", 1)
        if schema == 1 and "schema" not in raw:
            rows.append(normalize_v1(raw))
        elif schema == SCHEMA:
            rows.append(validate_row(raw))
        else:
            raise RowError(f"{path}:{number}: unknown schema")
    return rows


def _jsonl(rows: list[dict]) -> str:
    return "".join(json.dumps(row, separators=(",", ":")) + "\n" for row in rows)


# --- assemble ---------------------------------------------------------------------------------


def assemble(artifacts: Path, out: Path) -> dict:
    """Every measurement under `artifacts` -> new lines for the three schema-2 files.

    A measurement is a directory holding `context.json`, `fresh.json` and `fixed.json`. Either all
    of them extract and are written, or nothing is: a count mismatch means the binary and this
    reader disagree, and that is not a night to publish. Returns the number of lines per file.
    """
    contexts = sorted(Path(artifacts).rglob("context.json"))
    if not contexts:
        raise RowError("no measurement found: no context.json under the artifacts")
    fresh, fixed, runs = [], [], []
    formats_seen = []
    for context_path in contexts:
        context = json.loads(context_path.read_text(encoding="utf-8"))
        document_types = set()
        for slice_name, sink in (("fresh", fresh), ("fixed", fixed)):
            report_path = context_path.parent / f"{slice_name}.json"
            if not report_path.is_file():
                raise RowError(f"{context_path.parent.name}: {slice_name}.json is missing")
            report = json.loads(report_path.read_text(encoding="utf-8"))
            rows, header = extract_run(report, slice_name, context)
            sink.extend(rows)
            runs.append(header)
            document_types.add(header["document_type"])
        if len(document_types) != 1:
            raise RowError(f"{context_path.parent.name}: fresh and fixed name different formats")
        formats_seen.extend(document_types)
    if len(formats_seen) != len(set(formats_seen)):
        raise RowError("two measurements name the same format")
    duplicates = find_duplicates(fresh)
    if duplicates:
        raise RowError(f"{len(duplicates)} fresh (document_type, seed, profile) key(s) repeat in this run")

    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    (out / "fresh.jsonl").write_text(_jsonl(fresh), encoding="utf-8")
    (out / "fixed.jsonl").write_text(_jsonl(fixed), encoding="utf-8")
    (out / RUN_HEADER_FILE).write_text(_jsonl(runs), encoding="utf-8")
    return {"fresh": len(fresh), "fixed": len(fixed), "runs": len(runs), "formats": sorted(formats_seen)}


# --- next-seed --------------------------------------------------------------------------------


def next_seed(data_dir: Path, document_type: str) -> int:
    """The first seed of a format's next fresh window.

    Continues from the highest fresh seed that format already has, and never below one past the
    frozen `dataset.jsonl`'s highest seed (so the TD3 sequence carries on, and no format restarts
    over documents the corpus already measured). With neither file, `MIN_FRESH_SEED`, above the
    fixed slice.
    """
    if document_type not in FORMATS:
        raise RowError("document type must be one of td1, td2, td3, mrva, mrvb")
    data_dir = Path(data_dir)
    highest = MIN_FRESH_SEED - 1
    fresh = data_dir / "fresh.jsonl"
    if fresh.is_file():
        seeds = [r["seed"] for r in read_rows(fresh) if r["document_type"] == document_type]
        highest = max([highest, *seeds])
    dataset = data_dir / "dataset.jsonl"
    if dataset.is_file():
        seeds = [r["seed"] for r in read_rows(dataset) if _is_int(r["seed"])]
        highest = max([highest, *seeds])
    return highest + 1


# --- context ----------------------------------------------------------------------------------


def _sha256_of(path: Path) -> str:
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def collect_context(model_dir: Path) -> dict:
    """The runner facts a header records, read from this machine and the workflow's environment."""
    cpu_model = "unknown"
    try:
        for line in Path("/proc/cpuinfo").read_text(encoding="utf-8", errors="replace").splitlines():
            if line.lower().startswith("model name"):
                cpu_model = line.split(":", 1)[1].strip()[:120]
                break
    except OSError:
        pass
    try:
        nproc = len(os.sched_getaffinity(0))
    except AttributeError:
        nproc = os.cpu_count() or 1
    rustc = subprocess.run(["rustc", "-V"], capture_output=True, text=True, check=True).stdout.strip()
    env = os.environ
    return {
        "run_id": f"{env['GITHUB_RUN_ID']}-{env.get('GITHUB_RUN_ATTEMPT', '1')}",
        "git_sha": env["GITHUB_SHA"],
        "event": env.get("GITHUB_EVENT_NAME", "unknown"),
        "cpu_model": cpu_model,
        "nproc": nproc,
        "rustc": rustc,
        "model_sha256": {
            "detection": _sha256_of(Path(model_dir) / "text-detection.rten"),
            "recognition": _sha256_of(Path(model_dir) / "text-recognition.rten"),
        },
        "ocr_env": {k: v for k, v in sorted(env.items()) if k.startswith("SYNTHPASS_")},
    }


# --- command line -----------------------------------------------------------------------------


def _fail(message: str) -> int:
    print(f"::error::{message}", file=sys.stderr)
    return 1


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)

    seed_cmd = commands.add_parser("next-seed", help="first fresh seed for one format")
    seed_cmd.add_argument("--data-dir", type=Path, required=True)
    seed_cmd.add_argument("--document-type", required=True)

    context_cmd = commands.add_parser("context", help="write the runner facts a header records")
    context_cmd.add_argument("--out", type=Path, required=True)
    context_cmd.add_argument("--model-dir", type=Path, default=Path("."))

    assemble_cmd = commands.add_parser("assemble", help="measurements -> new schema-2 lines")
    assemble_cmd.add_argument("--artifacts", type=Path, required=True)
    assemble_cmd.add_argument("--out", type=Path, required=True)

    guard_cmd = commands.add_parser("guard", help="refuse a fresh.jsonl with a repeated key")
    guard_cmd.add_argument("--file", type=Path, required=True)

    args = parser.parse_args(argv)
    try:
        if args.command == "next-seed":
            print(next_seed(args.data_dir, args.document_type))
        elif args.command == "context":
            context = validate_context(collect_context(args.model_dir))
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(json.dumps(context, indent=2) + "\n", encoding="utf-8")
        elif args.command == "assemble":
            print(json.dumps(assemble(args.artifacts, args.out)))
        elif args.command == "guard":
            rows = read_rows(args.file) if args.file.is_file() else []
            duplicates = find_duplicates(rows)
            if duplicates:
                return _fail(
                    f"{len(duplicates)} duplicate (document_type, seed, profile) key(s) in "
                    f"{args.file.name}: the seed window overlapped an earlier run. Not committing."
                )
    except (RowError, OSError, KeyError, json.JSONDecodeError, subprocess.CalledProcessError) as error:
        return _fail(f"{type(error).__name__}: {error}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
