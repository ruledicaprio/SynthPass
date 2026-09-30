#!/usr/bin/env python3
"""
bench_ab_args.py

The trust boundary of `.github/workflows/bench-ab.yml` (ADR-0027, decisions 3, 4 and 8).

A `workflow_dispatch` input is text a person typed. The workflow hands every raw input to `validate`
through the environment, and nothing else reads one: no later step touches a raw input, and no
`${{ }}` expression sits inside a `run:` script. `validate` refuses anything outside a small,
explicit grammar and writes a normalized plan. Every later step takes what it needs from that plan:
`run` measures one format in one arm with the arm's environment assignments applied by
`subprocess`, so no shell ever splits or re-parses them; `arm-json` records what an arm was;
`summary` prints the counts-only lines the job log may carry.

Standard library only, matching every other `tools/` script.

## Commands

    bench_ab_args.py validate --plan PLAN.json [--github-output FILE]
        Reads AB_BEFORE_REF, AB_AFTER_REF, AB_BEFORE_ENV, AB_AFTER_ENV, AB_FORMATS, AB_PROFILE,
        AB_COUNT, AB_SEED and AB_EXPECT_IDENTICAL. Writes PLAN.json, and `name=value` lines for
        `$GITHUB_OUTPUT`. Every value is validated, so a later `with:` or `env:` may carry it.
    bench_ab_args.py run --plan PLAN.json --role before|after --format F --binary B --cwd D --out-dir O
        Runs `synthpass-bench --document-type F --profile P --count N --seed S --dump-ocr --out O/F.json`
        in D, with the arm's assignments plus the pinned budget. Standard output, which
        `--dump-ocr` fills with synthetic OCR text, goes to O/F.stdout and never to the job log.
    bench_ab_args.py arm-json --plan PLAN.json --role R --git-sha SHA --binary B --out O/arm.json
        Writes the arm record. `context` is the nightly's runner-facts block
        (`bench_nightly_rows.CONTEXT_KEYS`, checked by its `validate_context`), with `ocr_env`
        set to the assignments this arm actually ran under.
    bench_ab_args.py summary --plan PLAN.json --before ARM.json --after ARM.json
        Prints the two arms' refs, commits, environments and binary hashes: counts and short
        validated tokens only.

Exit status: 0; 2 with one `::error::` line on any refusal. The line never repeats a raw input
verbatim: an input is shown only after every character outside `[A-Za-z0-9._/=@-]` is replaced,
because a workflow command (`::add-mask::`, `::set-env::`) is a line of text in the log.

## The rules

- **Refs** match `[A-Za-z0-9][A-Za-z0-9._/-]{0,199}` and hold no `..`, `@{` or `//`, do not end in
  `/`, `.` or `.lock`, and have no path component that starts with `.` or ends in `.lock`.
  A full 40-character commit id is a ref too.
- **Environment assignments** are whitespace-separated `NAME=VALUE`. NAME is one of `ALLOWED_ENV`,
  the knobs that change what an arm reads, each documented in
  `knowledge/architecture/configuration.md`. VALUE matches `[A-Za-z0-9._-]{1,32}`. A NAME may not
  repeat. `SYNTHPASS_OCR_MAX_SECONDS` is refused by name: the workflow pins it (below).
  A name that says path, model, hash, verify, skip, download, licence, key, token, TLS, log or LLM
  is refused with a message that says so.
- **Formats:** a non-empty, duplicate-free subset of `td1 td2 td3 mrva mrvb`, separated by spaces
  or commas, normalized to that order.
- **Profile:** one that `synthpass-bench --profile` accepts (`synthpass_bench::ProfileChoice::parse`).
- **Count** 1 to 500 and **seed** 0 to 4294967295, in canonical decimal: no sign, no leading zero.
- **expect_identical:** `true` or `false`.

## The pinned budget

Both arms run with `SYNTHPASS_OCR_MAX_SECONDS=600`. The retry loop stops on elapsed time, so under
the shipped 52 s a slower runner can read a document differently with no code change (ADR-0027,
decision 3). 600 s is far above what any synthetic document needs (the slowest of 14,110 nightly
documents took 15.7 s), so the budget never binds and machine speed cannot enter the diff.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

import bench_nightly_rows as nightly

PLAN_SCHEMA = 1
ARM_SCHEMA = 1
ROLES = ("before", "after")
FORMATS = ("td1", "td2", "td3", "mrva", "mrvb")
# `synthpass_bench::ProfileChoice::parse`: everything `--profile` accepts, lowercase.
PROFILES = ("clean", "mobile", "scanner", "worn", "border-kiosk", "damaged", "all")
COUNT_MAX = 500
SEED_MAX = 4294967295

# The pinned budget: set by the workflow on both arms, never by a dispatcher.
PINNED = {"SYNTHPASS_OCR_MAX_SECONDS": "600"}

# Knobs that change what an arm reads. Each is documented in
# `knowledge/architecture/configuration.md` (a test pins that) and read by the code in
# `synthpass-ocr` or `synthpass-die`. `SYNTHPASS_MRZ_DATE_DIGITS` joins this list when #631 merges.
#
# Left out on purpose:
# - `SYNTHPASS_OCR_STOP` and `SYNTHPASS_OCR_CONFIRM_PASSES` are gone (#473): the code only warns that
#   they have no effect, so an A/B on them would compare two identical arms and read as a null result.
# - `SYNTHPASS_OCR_THREADS`, `SYNTHPASS_OCR_VERBOSE` and `SYNTHPASS_OCR_DUMP_VARIANTS` change no read
#   (concurrency, and diagnostics that write to the log or the disk).
ALLOWED_ENV = (
    "SYNTHPASS_MRZ_CLASS_SWEEP",
    "SYNTHPASS_MRZ_LINE1_SELECT",
    "SYNTHPASS_MRZ_REFUSE_REPEATED_LINE",
    "SYNTHPASS_OCR_CHARGRID",
    "SYNTHPASS_OCR_MAX_PASSES",
    "SYNTHPASS_OCR_ORDER",
    "SYNTHPASS_OCR_ROTATE",
    "SYNTHPASS_OCR_SKEW",
    "SYNTHPASS_OCR_TEXTURE",
)
# Words in a variable name that mean it is not a measurement knob, for a specific refusal message.
# The allowlist decides; these only make the message say why.
REFUSED_WORDS = (
    "PATH", "DIR", "MODEL", "SHA256", "HASH", "VERIFY", "SKIP", "DOWNLOAD", "LICENSE", "LICENCE",
    "KEY", "PUBKEY", "PRIVKEY", "TOKEN", "TLS", "CERT", "LOG", "AUDIT", "LLM",
)

REF = re.compile(r"[A-Za-z0-9][A-Za-z0-9._/-]{0,199}")
ENV_NAME = re.compile(r"[A-Z][A-Z0-9_]{0,63}")
ENV_VALUE = re.compile(r"[A-Za-z0-9._-]{1,32}")
# What an environment-assignment input may contain at all: printable ASCII, space, tab, newline.
ENV_TEXT = re.compile(r"[\x20-\x7E\t\r\n]*")
ENV_TEXT_MAX = 1000
INTEGER = re.compile(r"0|[1-9][0-9]{0,9}")
HEX40 = re.compile(r"[0-9a-f]{40}")
HEX64 = re.compile(r"[0-9a-f]{64}")
UNSAFE_CHARACTERS = re.compile(r"[^A-Za-z0-9._/=@-]")
SHOWN_MAX = 48

ENV_VARIABLES = {
    "before_ref": "AB_BEFORE_REF",
    "after_ref": "AB_AFTER_REF",
    "before_env": "AB_BEFORE_ENV",
    "after_env": "AB_AFTER_ENV",
    "formats": "AB_FORMATS",
    "profile": "AB_PROFILE",
    "count": "AB_COUNT",
    "seed": "AB_SEED",
    "expect_identical": "AB_EXPECT_IDENTICAL",
}
PLAN_KEYS = (
    "schema", "before_ref", "after_ref", "before_env", "after_env", "formats", "profile", "count",
    "seed", "expect_identical", "pinned",
)
ARM_KEYS = ("schema", "role", "ref", "git_sha", "env", "pinned", "binary_sha256", "context")


class Refused(Exception):
    """An input or a file the workflow must not use. The message never repeats raw input."""


def show(text: str) -> str:
    """A raw input made safe to put in a one-line message: unsafe characters become `?`."""
    shown = UNSAFE_CHARACTERS.sub("?", text[: SHOWN_MAX + 1])
    return f"'{shown[:SHOWN_MAX]}...'" if len(shown) > SHOWN_MAX else f"'{shown}'"


# --- the inputs -------------------------------------------------------------------------------


def parse_ref(text: str, label: str) -> str:
    if not REF.fullmatch(text):
        raise Refused(f"{label}: {show(text)} is not a ref name (letters, digits, . _ / -, at most 200, "
                      "starting with a letter or digit)")
    if ".." in text or "@{" in text or "//" in text:
        raise Refused(f"{label}: {show(text)} holds '..', '@{{' or '//'")
    if text.endswith(("/", ".", ".lock")):
        raise Refused(f"{label}: {show(text)} ends in '/', '.' or '.lock'")
    if any(part.startswith(".") or part.endswith(".lock") for part in text.split("/")):
        raise Refused(f"{label}: {show(text)} has a component that starts with '.' or ends in '.lock'")
    return text


def _env_name_refusal(name: str, label: str) -> str:
    if name in PINNED:
        return (f"{label}: {name} is pinned by the workflow to {PINNED[name]} on both arms "
                "(ADR-0027 decision 3) and cannot be set")
    words = sorted(set(name.split("_")) & set(REFUSED_WORDS))
    if words:
        return (f"{label}: {name} names a {'/'.join(w.lower() for w in words)} setting; an A/B varies "
                "only the measurement knobs")
    return f"{label}: {show(name)} is not one of the allowed measurement knobs ({', '.join(ALLOWED_ENV)})"


def parse_env(text: str, label: str) -> dict[str, str]:
    if len(text) > ENV_TEXT_MAX or not ENV_TEXT.fullmatch(text):
        raise Refused(f"{label}: at most {ENV_TEXT_MAX} printable ASCII characters of NAME=VALUE pairs")
    assignments: dict[str, str] = {}
    for token in text.split():
        name, separator, value = token.partition("=")
        if not separator:
            raise Refused(f"{label}: {show(token)} is not NAME=VALUE")
        if not ENV_NAME.fullmatch(name):
            raise Refused(f"{label}: {show(name)} is not a variable name")
        if name not in ALLOWED_ENV:
            raise Refused(_env_name_refusal(name, label))
        if not ENV_VALUE.fullmatch(value):
            raise Refused(f"{label}: the value of {name} must match [A-Za-z0-9._-]{{1,32}}")
        if name in assignments:
            raise Refused(f"{label}: {name} is given twice")
        assignments[name] = value
    return dict(sorted(assignments.items()))


def parse_formats(text: str) -> list[str]:
    tokens = text.replace(",", " ").split()
    if not tokens or not ENV_TEXT.fullmatch(text):
        raise Refused(f"formats: give at least one of {' '.join(FORMATS)}")
    for token in tokens:
        if token not in FORMATS:
            raise Refused(f"formats: {show(token)} is not one of {' '.join(FORMATS)}")
    if len(set(tokens)) != len(tokens):
        raise Refused("formats: a format is named twice")
    return [f for f in FORMATS if f in tokens]


def parse_profile(text: str) -> str:
    if text not in PROFILES:
        raise Refused(f"profile: {show(text)} is not one of {' '.join(PROFILES)}")
    return text


def parse_integer(text: str, label: str, low: int, high: int) -> int:
    if not INTEGER.fullmatch(text):
        raise Refused(f"{label}: {show(text)} is not a canonical decimal integer (no sign, no leading zero)")
    value = int(text)
    if not low <= value <= high:
        raise Refused(f"{label}: must be {low} to {high}")
    return value


def parse_bool(text: str, label: str) -> bool:
    if text not in ("true", "false"):
        raise Refused(f"{label}: must be 'true' or 'false'")
    return text == "true"


def validate(environ) -> dict:
    """The normalized plan for the raw inputs in `environ`, or `Refused`."""
    raw = {}
    for key, variable in ENV_VARIABLES.items():
        value = environ.get(variable)
        if value is None:
            # An unset variable is the workflow's mistake for every input but the two assignment
            # lists, which are empty by default.
            if key in ("before_env", "after_env"):
                value = ""
            else:
                raise Refused(f"{variable} is not set")
        raw[key] = value
    return {
        "schema": PLAN_SCHEMA,
        "before_ref": parse_ref(raw["before_ref"], "before_ref"),
        "after_ref": parse_ref(raw["after_ref"], "after_ref"),
        "before_env": parse_env(raw["before_env"], "before_env"),
        "after_env": parse_env(raw["after_env"], "after_env"),
        "formats": parse_formats(raw["formats"]),
        "profile": parse_profile(raw["profile"]),
        "count": parse_integer(raw["count"], "count", 1, COUNT_MAX),
        "seed": parse_integer(raw["seed"], "seed", 0, SEED_MAX),
        "expect_identical": parse_bool(raw["expect_identical"], "expect_identical"),
        "pinned": dict(PINNED),
    }


def output_lines(plan: dict) -> list[str]:
    """`$GITHUB_OUTPUT` lines. Every value has passed a validator above, so none holds a newline."""
    return [
        f"before_ref={plan['before_ref']}",
        f"after_ref={plan['after_ref']}",
        f"formats={' '.join(plan['formats'])}",
        f"profile={plan['profile']}",
        f"count={plan['count']}",
        f"seed={plan['seed']}",
        f"expect_identical={'true' if plan['expect_identical'] else 'false'}",
    ]


# --- the plan on disk -------------------------------------------------------------------------


def check_plan(plan) -> dict:
    """A plan read back from a file is checked again, so a step never trusts a file it did not write."""
    if not isinstance(plan, dict) or set(plan) != set(PLAN_KEYS) or plan["schema"] != PLAN_SCHEMA:
        raise Refused("the plan file is not a schema-1 plan")
    for key in ("before_ref", "after_ref"):
        if not isinstance(plan[key], str):
            raise Refused(f"the plan's {key} is not a string")
        parse_ref(plan[key], key)
    for key in ("before_env", "after_env"):
        assignments = plan[key]
        if not isinstance(assignments, dict) or any(
            name not in ALLOWED_ENV or not isinstance(value, str) or not ENV_VALUE.fullmatch(value)
            for name, value in assignments.items()
        ):
            raise Refused(f"the plan's {key} holds an assignment that validate would refuse")
    formats = plan["formats"]
    if not isinstance(formats, list) or not formats or any(f not in FORMATS for f in formats) \
            or len(set(formats)) != len(formats):
        raise Refused("the plan's formats are not a duplicate-free subset of the five formats")
    parse_profile(str(plan["profile"]))
    for key, high, low in (("count", COUNT_MAX, 1), ("seed", SEED_MAX, 0)):
        value = plan[key]
        if isinstance(value, bool) or not isinstance(value, int) or not low <= value <= high:
            raise Refused(f"the plan's {key} is out of range")
    if not isinstance(plan["expect_identical"], bool) or plan["pinned"] != PINNED:
        raise Refused("the plan's expect_identical or pinned block is wrong")
    return plan


def load_plan(path: Path) -> dict:
    try:
        return check_plan(json.loads(path.read_text(encoding="utf-8")))
    except (OSError, ValueError) as error:
        raise Refused(f"cannot read the plan: {type(error).__name__}") from error


# --- measuring --------------------------------------------------------------------------------


def arm_environment(plan: dict, role: str, inherited) -> dict[str, str]:
    """The process environment of one arm: the runner's own, with every inherited `SYNTHPASS_*`
    removed so a stray variable cannot reach one arm, then the arm's assignments, then the pin."""
    env = {k: v for k, v in inherited.items() if not k.startswith("SYNTHPASS_")}
    env.update(plan[f"{role}_env"])
    env.update(plan["pinned"])
    return env


def run_arm(plan: dict, role: str, fmt: str, binary: Path, cwd: Path, out_dir: Path) -> int:
    """Run `synthpass-bench` for one format in one arm; return its exit status."""
    if role not in ROLES:
        raise Refused("role must be before or after")
    if fmt not in plan["formats"]:
        raise Refused("that format is not in the plan")
    out_dir.mkdir(parents=True, exist_ok=True)
    argv = [
        str(binary.resolve()),
        "--document-type", fmt,
        "--profile", plan["profile"],
        "--count", str(plan["count"]),
        "--seed", str(plan["seed"]),
        "--dump-ocr",
        "--out", str((out_dir / f"{fmt}.json").resolve()),
    ]
    # `--dump-ocr` prints every document's raw OCR text to standard output. That is synthetic text
    # and belongs in the 3-day artifact, never in the job log.
    with (out_dir / f"{fmt}.stdout").open("wb") as sink:
        return subprocess.run(argv, cwd=cwd, env=arm_environment(plan, role, os.environ), stdout=sink).returncode


# --- the arm record ---------------------------------------------------------------------------


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def build_arm(plan: dict, role: str, git_sha: str, binary: Path, context: dict) -> dict:
    """One arm's `arm.json`. `context` is the runner's facts as `bench_nightly_rows.collect_context`
    writes them; its `ocr_env` is replaced with what this arm's process actually ran under."""
    if role not in ROLES:
        raise Refused("role must be before or after")
    if not HEX40.fullmatch(git_sha):
        raise Refused("the commit is not 40 lowercase hex characters")
    arm_env = dict(plan[f"{role}_env"])
    context = dict(context)
    context["ocr_env"] = dict(sorted({**arm_env, **plan["pinned"]}.items()))
    arm = {
        "schema": ARM_SCHEMA,
        "role": role,
        "ref": plan[f"{role}_ref"],
        "git_sha": git_sha,
        "env": arm_env,
        "pinned": dict(plan["pinned"]),
        "binary_sha256": sha256_of(binary),
        "context": context,
    }
    return check_arm(arm)


def check_arm(arm) -> dict:
    """Refuse an arm record that is not exactly `ARM_KEYS` with the values `build_arm` writes."""
    if not isinstance(arm, dict) or set(arm) != set(ARM_KEYS) or arm["schema"] != ARM_SCHEMA:
        raise Refused("an arm.json must hold exactly: " + ", ".join(ARM_KEYS))
    if arm["role"] not in ROLES:
        raise Refused("arm.json: role must be before or after")
    if not isinstance(arm["ref"], str):
        raise Refused("arm.json: ref must be a string")
    parse_ref(arm["ref"], "arm.json ref")
    if not isinstance(arm["git_sha"], str) or not HEX40.fullmatch(arm["git_sha"]):
        raise Refused("arm.json: git_sha must be 40 lowercase hex characters")
    if not isinstance(arm["binary_sha256"], str) or not HEX64.fullmatch(arm["binary_sha256"]):
        raise Refused("arm.json: binary_sha256 must be 64 lowercase hex characters")
    env = arm["env"]
    if not isinstance(env, dict) or any(
        name not in ALLOWED_ENV or not isinstance(value, str) or not ENV_VALUE.fullmatch(value)
        for name, value in env.items()
    ):
        raise Refused("arm.json: env holds an assignment that validate would refuse")
    if arm["pinned"] != PINNED:
        raise Refused("arm.json: pinned must be exactly the workflow's pin")
    try:
        nightly.validate_context(arm["context"])
    except nightly.RowError as error:
        raise Refused(f"arm.json: {error}") from error
    if arm["context"]["ocr_env"] != dict(sorted({**env, **arm["pinned"]}.items())):
        raise Refused("arm.json: context.ocr_env is not the arm's env plus the pin")
    return arm


def load_arm(path: Path) -> dict:
    try:
        return check_arm(json.loads(path.read_text(encoding="utf-8")))
    except (OSError, ValueError) as error:
        if isinstance(error, Refused):
            raise
        raise Refused(f"cannot read {path.name}: {type(error).__name__}") from error


def format_env(env: dict[str, str]) -> str:
    return " ".join(f"{name}={value}" for name, value in env.items()) or "(none: shipped defaults)"


def summary_lines(plan: dict, before: dict, after: dict) -> list[str]:
    """Counts and validated tokens only: nothing here comes from a report."""
    lines = [
        f"formats: {' '.join(plan['formats'])}; profile {plan['profile']}; {plan['count']} documents "
        f"per format from seed {plan['seed']}; expect_identical {str(plan['expect_identical']).lower()}",
        f"pinned on both arms: {format_env(plan['pinned'])}",
    ]
    for arm in (before, after):
        lines.append(f"{arm['role']}: ref {arm['ref']}, commit {arm['git_sha']}, env {format_env(arm['env'])}, "
                     f"binary sha256 {arm['binary_sha256']}")
    context = before["context"]
    lines.append(f"runner: {context['cpu_model']}, {context['nproc']} cores, {context['rustc']}, "
                 f"run {context['run_id']}")
    return lines


# --- command line -----------------------------------------------------------------------------


def _fail(message: str) -> int:
    print(f"::error::bench-ab: {message}", file=sys.stderr)
    return 2


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)

    validate_cmd = commands.add_parser("validate", help="raw inputs (environment) -> normalized plan")
    validate_cmd.add_argument("--plan", type=Path, required=True)
    validate_cmd.add_argument("--github-output", type=Path)

    run_cmd = commands.add_parser("run", help="measure one format in one arm")
    run_cmd.add_argument("--plan", type=Path, required=True)
    run_cmd.add_argument("--role", required=True)
    run_cmd.add_argument("--format", required=True)
    run_cmd.add_argument("--binary", type=Path, required=True)
    run_cmd.add_argument("--cwd", type=Path, required=True)
    run_cmd.add_argument("--out-dir", type=Path, required=True)

    arm_cmd = commands.add_parser("arm-json", help="write one arm's record")
    arm_cmd.add_argument("--plan", type=Path, required=True)
    arm_cmd.add_argument("--role", required=True)
    arm_cmd.add_argument("--git-sha", required=True)
    arm_cmd.add_argument("--binary", type=Path, required=True)
    arm_cmd.add_argument("--out", type=Path, required=True)
    arm_cmd.add_argument("--model-dir", type=Path, default=Path("."))

    summary_cmd = commands.add_parser("summary", help="the counts-only lines for the job log")
    summary_cmd.add_argument("--plan", type=Path, required=True)
    summary_cmd.add_argument("--before", type=Path, required=True)
    summary_cmd.add_argument("--after", type=Path, required=True)

    args = parser.parse_args(argv)
    try:
        if args.command == "validate":
            plan = validate(os.environ)
            args.plan.parent.mkdir(parents=True, exist_ok=True)
            args.plan.write_text(json.dumps(plan, indent=2, sort_keys=True) + "\n", encoding="utf-8")
            lines = output_lines(plan)
            if args.github_output:
                with args.github_output.open("a", encoding="utf-8") as sink:
                    sink.write("\n".join(lines) + "\n")
            print("bench-ab: inputs accepted: " + "; ".join(lines))
            print(f"bench-ab: before env {format_env(plan['before_env'])}; after env {format_env(plan['after_env'])}")
        elif args.command == "run":
            code = run_arm(load_plan(args.plan), args.role, args.format, args.binary, args.cwd, args.out_dir)
            print(f"bench-ab: {args.role} {args.format}: synthpass-bench exited {code}", flush=True)
            return code
        elif args.command == "arm-json":
            plan = load_plan(args.plan)
            arm = build_arm(plan, args.role, args.git_sha, args.binary, nightly.collect_context(args.model_dir))
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(json.dumps(arm, indent=2, sort_keys=True) + "\n", encoding="utf-8")
        elif args.command == "summary":
            print("\n".join(summary_lines(load_plan(args.plan), load_arm(args.before), load_arm(args.after))))
    except Refused as error:
        return _fail(str(error))
    except (OSError, KeyError, subprocess.CalledProcessError) as error:
        return _fail(f"{type(error).__name__} while running {args.command}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
