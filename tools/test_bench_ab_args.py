"""Tests for bench_ab_args.py: the validator that stands between a dispatch input and a shell.

Offline: no benchmark run, no network, no git. `run` is exercised with a stand-in for the
`synthpass-bench` binary.
"""
import contextlib
import io
import json
import os
import re
import stat
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import bench_ab_args as a
import bench_nightly_rows as nightly

REPO = Path(__file__).resolve().parent.parent
CONFIGURATION = REPO / "knowledge" / "architecture" / "configuration.md"
SHA = "a" * 40
HASH = "b" * 64

# Every knob's accepted values, written out here rather than read from the validator, so a test
# that fails names the drift. Sources: `configuration.md` and the readers in `synthpass-die` and
# `synthpass-ocr` (`ConfigurationPin` re-checks each row against the doc).
WORD_KNOBS = {
    "SYNTHPASS_MRZ_CLASS_SWEEP": ("off", "on", "control"),
    "SYNTHPASS_MRZ_DATE_DIGITS": ("off", "on", "control"),
    "SYNTHPASS_MRZ_LINE1_SELECT": ("off", "control", "on"),
    "SYNTHPASS_MRZ_REFUSE_REPEATED_LINE": ("off", "control", "on"),
    "SYNTHPASS_OCR_CHARGRID": ("off", "on", "control"),
    "SYNTHPASS_OCR_ORDER": ("default", "band-first", "control"),
    "SYNTHPASS_OCR_ROTATE": ("default", "legacy", "off"),
    "SYNTHPASS_OCR_SKEW": ("default", "legacy"),
    "SYNTHPASS_OCR_TEXTURE": ("off", "on", "control"),
}
MAX_PASSES = "SYNTHPASS_OCR_MAX_PASSES"
MAX_PASSES_HIGH = 14


def inputs(**over):
    """The raw dispatch inputs, as the workflow passes them, with any of them replaced."""
    base = {
        "AB_BEFORE_REF": "main",
        "AB_AFTER_REF": "claude/some-branch",
        "AB_BEFORE_ENV": "",
        "AB_AFTER_ENV": "",
        "AB_FORMATS": "td1 td2 td3 mrva mrvb",
        "AB_PROFILE": "clean",
        "AB_COUNT": "100",
        "AB_SEED": "0",
        "AB_EXPECT_IDENTICAL": "false",
    }
    base.update(over)
    return base


def context(**over):
    base = {
        "run_id": "123456789-1",
        "git_sha": SHA,
        "event": "workflow_dispatch",
        "cpu_model": "Test CPU 3.0GHz",
        "nproc": 4,
        "rustc": "rustc 1.90.0 (0000000 2025-09-01)",
        "model_sha256": {"detection": "c" * 64, "recognition": "d" * 64},
        "ocr_env": {},
    }
    base.update(over)
    return base


class Refusals(unittest.TestCase):
    def refuses(self, fragment=None, **over):
        with self.assertRaises(a.Refused) as caught:
            a.validate(inputs(**over))
        if fragment:
            self.assertIn(fragment, str(caught.exception))
        return str(caught.exception)


class Defaults(Refusals):
    def test_the_defaults_make_a_plan(self):
        plan = a.validate(inputs())
        self.assertEqual(plan["before_ref"], "main")
        self.assertEqual(plan["formats"], ["td1", "td2", "td3", "mrva", "mrvb"])
        self.assertEqual((plan["profile"], plan["count"], plan["seed"]), ("clean", 100, 0))
        self.assertEqual(plan["before_env"], {})
        self.assertFalse(plan["expect_identical"])
        self.assertEqual(plan["pinned"], {"SYNTHPASS_OCR_MAX_SECONDS": "600"})
        self.assertEqual(set(plan), set(a.PLAN_KEYS))

    def test_the_assignment_lists_may_be_unset_but_no_other_input(self):
        env = inputs()
        del env["AB_BEFORE_ENV"], env["AB_AFTER_ENV"]
        self.assertEqual(a.validate(env)["after_env"], {})
        for variable in ("AB_BEFORE_REF", "AB_AFTER_REF", "AB_FORMATS", "AB_PROFILE", "AB_COUNT", "AB_SEED",
                         "AB_EXPECT_IDENTICAL"):
            env = inputs()
            del env[variable]
            with self.assertRaises(a.Refused):
                a.validate(env)

    def test_a_before_ref_may_equal_the_after_ref_for_an_a_a_run(self):
        plan = a.validate(inputs(AB_AFTER_REF="main", AB_EXPECT_IDENTICAL="true"))
        self.assertEqual(plan["before_ref"], plan["after_ref"])
        self.assertTrue(plan["expect_identical"])

    def test_the_output_lines_are_single_plain_lines(self):
        lines = a.output_lines(a.validate(inputs(AB_FORMATS="td3,td1")))
        self.assertIn("formats=td1 td3", lines)
        for line in lines:
            self.assertRegex(line, r"^[a-z_]+=[A-Za-z0-9._/ -]+$")


class Refs(Refusals):
    def test_good_refs(self):
        for ref in ("main", "claude/613-bench-ab", "v1.2.3", "release-2026.09", "refs/pull/631/head", SHA,
                    "a", "x" * 200):
            with self.subTest(ref=ref):
                self.assertEqual(a.validate(inputs(AB_AFTER_REF=ref))["after_ref"], ref)

    def test_bad_refs(self):
        for ref in ("", "-rf", ".hidden", "/main", "a b", "a;b", "a$(id)", "a`id`", "a|b", "a&b", "a>b", "a'b",
                    'a"b', "a\\b", "a:b", "a~1", "a^", "a?b", "a*b", "a[b", "aéb", "x" * 201,
                    "a..b", "a@{u}", "a@{-1}", "a//b", "a/", "a.", "a.lock", "a/b.lock", "a/.b", "a/b.lock/c",
                    "main\n", "main\nAB", "\nmain", "main ", " main", "a\tb"):
            with self.subTest(ref=ref):
                self.refuses(AB_AFTER_REF=ref)
                self.refuses(AB_BEFORE_REF=ref)

    def test_a_trailing_newline_is_not_forgiven(self):
        # `re.match(r"...$")` accepts "main\n"; the validator must not.
        self.refuses(AB_AFTER_REF="main\n")


class EnvAssignments(Refusals):
    def test_every_allowed_knob_is_accepted_with_each_of_its_values(self):
        for name, values in WORD_KNOBS.items():
            for value in values:
                with self.subTest(name=name, value=value):
                    self.assertEqual(a.validate(inputs(AB_AFTER_ENV=f"{name}={value}"))["after_env"],
                                     {name: value})
                    self.assertEqual(a.validate(inputs(AB_BEFORE_ENV=f"{name}={value}"))["before_env"],
                                     {name: value})

    def test_the_table_covers_exactly_the_allowed_knobs(self):
        self.assertEqual(set(WORD_KNOBS) | {MAX_PASSES}, set(a.ALLOWED_ENV))

    def test_several_assignments_on_any_whitespace_normalize_sorted(self):
        plan = a.validate(inputs(
            AB_BEFORE_ENV="  SYNTHPASS_OCR_ORDER=band-first\tSYNTHPASS_MRZ_CLASS_SWEEP=on\nSYNTHPASS_OCR_MAX_PASSES=6  "))
        self.assertEqual(list(plan["before_env"]), sorted(plan["before_env"]))
        self.assertEqual(plan["before_env"]["SYNTHPASS_OCR_ORDER"], "band-first")
        self.assertEqual(len(plan["before_env"]), 3)

    def test_the_budget_is_refused_by_name(self):
        message = self.refuses(AB_AFTER_ENV="SYNTHPASS_OCR_MAX_SECONDS=1")
        self.assertIn("pinned by the workflow", message)
        self.assertIn("SYNTHPASS_OCR_MAX_SECONDS", message)

    def test_knobs_that_are_not_measurement_arms_are_refused_with_the_reason(self):
        cases = {
            "SYNTHPASS_OCR_MODEL_DIR": "model",
            "SYNTHPASS_MODEL_PATH": "path",
            "SYNTHPASS_MODEL_SHA256": "sha256",
            "SYNTHPASS_OCR_DETECTION_SHA256": "sha256",
            "SYNTHPASS_OCR_MODEL_SKIP_VERIFY": "verify",
            "SYNTHPASS_MODEL_SKIP_VERIFY": "verify",
            "SYNTHPASS_OCR_AUTO_DOWNLOAD": "download",
            "SYNTHPASS_LICENSE_SKIP": "license",
            "SYNTHPASS_LICENSE_PUBKEY": "license",
            "SYNTHPASS_KEY": "key",
            "SYNTHPASS_TOKEN": "token",
            "SYNTHPASS_TLS_CERT": "tls",
            "SYNTHPASS_LOG": "log",
            "SYNTHPASS_LOG_FORMAT": "log",
            "SYNTHPASS_LLM_GRAMMAR": "llm",
            "SYNTHPASS_LLM_MRZ_HINT": "llm",
        }
        for name, word in cases.items():
            with self.subTest(name=name):
                message = self.refuses(AB_BEFORE_ENV=f"{name}=1")
                self.assertIn(word, message)
                self.assertIn(name, message)
                self.assertNotIn("not one of the allowed", message)

    def test_removed_and_diagnostic_knobs_are_not_allowed(self):
        for name in ("SYNTHPASS_OCR_STOP", "SYNTHPASS_OCR_CONFIRM_PASSES", "SYNTHPASS_OCR_THREADS",
                     "SYNTHPASS_OCR_VERBOSE", "SYNTHPASS_OCR_DUMP_VARIANTS", "SYNTHPASS_OCR_ENGINE"):
            with self.subTest(name=name):
                self.refuses("not one of the allowed", AB_AFTER_ENV=f"{name}=on")

    def test_variables_outside_synthpass_are_refused(self):
        for name in ("PATH", "LD_PRELOAD", "RUST_LOG", "HOME", "GITHUB_ENV", "BASH_ENV", "RUSTFLAGS"):
            with self.subTest(name=name):
                self.refuses(AB_AFTER_ENV=f"{name}=x")

    def test_malformed_assignments(self):
        for text in ("SYNTHPASS_OCR_ORDER", "SYNTHPASS_OCR_ORDER=", "=on", "synthpass_ocr_order=on",
                     "SYNTHPASS_OCR_ORDER==on", "SYNTHPASS_OCR_ORDER=on;id", "SYNTHPASS_OCR_ORDER=$(id)",
                     "SYNTHPASS_OCR_ORDER=a b=c", "SYNTHPASS_OCR_ORDER=`id`", "SYNTHPASS_OCR_ORDER='on'",
                     'SYNTHPASS_OCR_ORDER="on"', "SYNTHPASS_OCR_ORDER=" + "x" * 33, "SYNTHPASS_OCR_ORDER=a/b",
                     "SYNTHPASS_OCR_ORDER=é", "SYNTHPASS_OCR_ORDER=on SYNTHPASS_OCR_SKEW=legacy",
                     "SYNTHPASS_OCR_ORDER=on\x00", "SYNTHPASS_OCR_ORDER=on\x0bSYNTHPASS_OCR_SKEW=legacy",
                     "SYNTHPASS_OCR_ORDER=on SYNTHPASS_OCR_SKEW=legacy", "A" * 1001):
            with self.subTest(text=text[:40]):
                self.refuses(AB_AFTER_ENV=text)

    def test_a_value_of_the_maximum_length_is_refused_as_not_one_the_knob_reads(self):
        self.refuses("is not one of", AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=" + "x" * 32)

    def test_a_repeated_name_is_refused_even_with_the_same_value(self):
        self.refuses("given twice", AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=default SYNTHPASS_OCR_ORDER=default")

    def test_the_two_arms_are_checked_separately(self):
        plan = a.validate(inputs(AB_BEFORE_ENV="SYNTHPASS_OCR_ORDER=default",
                                 AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=band-first"))
        self.assertNotEqual(plan["before_env"], plan["after_env"])
        self.refuses("before_env", AB_BEFORE_ENV="PATH=x")
        self.refuses("after_env", AB_AFTER_ENV="PATH=x")


class KnobValues(Refusals):
    """#613: every knob falls back to its default silently on a value it does not recognise, so an
    A/B on a typo compares two identical arms and reads as a clean null. The dispatch refuses it."""

    def test_a_typo_is_refused_naming_the_knob_and_its_values(self):
        message = self.refuses(AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=bandfirst")
        self.assertIn("SYNTHPASS_OCR_ORDER: 'bandfirst' is not one of default, band-first, control", message)
        self.assertIn("after_env", message)
        for name, values in WORD_KNOBS.items():
            with self.subTest(name=name):
                message = self.refuses(AB_BEFORE_ENV=f"{name}=typo")
                self.assertIn(f"{name}: 'typo' is not one of {', '.join(values)}", message)
                self.assertIn("before_env", message)

    def test_a_value_another_knob_takes_is_refused(self):
        self.refuses("is not one of", AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=on")
        self.refuses("is not one of", AB_AFTER_ENV="SYNTHPASS_OCR_SKEW=off")
        self.refuses("is not one of", AB_AFTER_ENV="SYNTHPASS_OCR_ROTATE=control")
        self.refuses("is not one of", AB_AFTER_ENV="SYNTHPASS_MRZ_CLASS_SWEEP=default")

    def test_only_the_canonical_form_is_accepted(self):
        # The Rust readers trim and lowercase, so `ON` works there. A dispatcher types the value
        # once and `arm.json` should then hold the canonical one, so the validator is strict.
        for name, values in WORD_KNOBS.items():
            for value in (values[0].upper(), values[0].capitalize(), values[0] + "x", values[0][:-1]):
                with self.subTest(name=name, value=value):
                    self.refuses("is not one of", AB_AFTER_ENV=f"{name}={value}")

    def test_whitespace_splits_an_assignment_so_a_padded_value_is_not_one(self):
        # `NAME= value` is `NAME=` (an empty value) and a stray token, so it is refused as
        # malformed; `NAME=value ` is a canonical value followed by a separator, and is accepted.
        self.refuses(AB_AFTER_ENV="SYNTHPASS_OCR_ORDER= band-first")
        self.assertEqual(a.validate(inputs(AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=band-first "))["after_env"],
                         {"SYNTHPASS_OCR_ORDER": "band-first"})

    def test_a_spelling_close_to_band_first_is_not_band_first(self):
        for value in ("band_first", "bandfirst", "Band-First", "band-first-"):
            with self.subTest(value=value):
                self.refuses("is not one of", AB_AFTER_ENV=f"SYNTHPASS_OCR_ORDER={value}")

    def test_max_passes_takes_an_integer_from_1_to_the_default_in_canonical_form(self):
        for good in ("1", "2", "9", "10", "13", "14"):
            with self.subTest(good=good):
                self.assertEqual(a.validate(inputs(AB_AFTER_ENV=f"{MAX_PASSES}={good}"))["after_env"],
                                 {MAX_PASSES: good})
        for bad in ("0", "15", "100", "014", "01", "-1", "x", "1.0", "1e1", "0x1", "fourteen", "1_0",
                    "99999999999999999999"):
            with self.subTest(bad=bad):
                message = self.refuses(AB_AFTER_ENV=f"{MAX_PASSES}={bad}")
                self.assertIn(MAX_PASSES, message)
                self.assertIn("1 to 14", message)
        # `+1` and a non-ASCII digit are already outside the value grammar (the digit, outside the
        # input's character set): refused, with that refusal's own message.
        for bad in ("+1", "١"):
            with self.subTest(bad=bad):
                self.refuses(AB_AFTER_ENV=f"{MAX_PASSES}={bad}")

    def test_the_value_of_another_knob_is_not_a_max_passes_value(self):
        self.refuses("1 to 14", AB_AFTER_ENV=f"{MAX_PASSES}=on")

    def test_the_refusal_is_one_error_line_with_exit_2(self):
        code, stderr, stdout, written = Messages().run_validate(AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=bandfirst")
        self.assertEqual(code, 2)
        self.assertEqual((stdout, written), ("", ""))
        self.assertEqual(len(stderr.splitlines()), 1)
        self.assertTrue(stderr.startswith("::error::bench-ab: "))
        self.assertIn("SYNTHPASS_OCR_ORDER: 'bandfirst' is not one of default, band-first, control", stderr)

    def test_a_plan_file_holding_an_unknown_value_is_refused_on_load(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "plan.json"
            plan = a.validate(inputs(AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=band-first"))
            for env in ({"SYNTHPASS_OCR_ORDER": "bandfirst"}, {"SYNTHPASS_OCR_ORDER": "ON"},
                        {MAX_PASSES: "015"}, {MAX_PASSES: "15"}):
                for key in ("after_env", "before_env"):
                    with self.subTest(env=env, key=key):
                        path.write_text(json.dumps(dict(plan, **{key: env})), encoding="utf-8")
                        with self.assertRaises(a.Refused):
                            a.load_plan(path)
            for env in ({"SYNTHPASS_OCR_ORDER": "default"}, {MAX_PASSES: "14"}):
                with self.subTest(env=env):
                    path.write_text(json.dumps(dict(plan, after_env=env)), encoding="utf-8")
                    self.assertEqual(a.load_plan(path)["after_env"], env)


class ConfigurationPin(unittest.TestCase):
    def row(self, text, name):
        rows = [line for line in text.splitlines() if line.startswith(f"| `{name}`")]
        self.assertEqual(len(rows), 1, f"configuration.md has {len(rows)} rows for {name}, expected 1")
        return rows[0]

    def test_every_allowed_knob_s_values_are_the_ones_configuration_md_lists(self):
        # A row lists its values as one slash-separated run of backticked words (the default is a
        # single backticked word in its own column), e.g. `off`/`on`/`control`.
        run = re.compile(r"`[a-z][a-z-]*`(?:/`[a-z][a-z-]*`)+")
        text = CONFIGURATION.read_text(encoding="utf-8")
        for name, values in WORD_KNOBS.items():
            with self.subTest(name=name):
                listed = run.search(self.row(text, name))
                self.assertIsNotNone(listed, f"{name}'s row lists no `a`/`b` values")
                documented = tuple(word.strip("`") for word in listed.group(0).split("/"))
                self.assertEqual(len(documented), len(set(documented)))
                self.assertEqual(set(documented), set(values))
                self.assertEqual(len(a.KNOB_VALUES[name]), len(documented))
                self.assertEqual(set(a.KNOB_VALUES[name]), set(documented))

    def test_the_validator_has_a_rule_for_every_allowed_name_and_no_other(self):
        self.assertEqual(set(a.KNOB_VALUES) | set(a.KNOB_INTEGERS), set(a.ALLOWED_ENV))
        self.assertFalse(set(a.KNOB_VALUES) & set(a.KNOB_INTEGERS))

    def test_max_passes_ends_at_the_ocr_crate_s_default(self):
        # `DEFAULT_MAX_PASSES` = `MAX_RETRY_VARIANTS + 1` in `synthpass-ocr`: the most passes any
        # arm can start, so a larger value measures the same as it. Zero or a non-number falls back
        # silently.
        source = (REPO / "crates" / "synthpass-ocr" / "src" / "lib.rs").read_text(encoding="utf-8")
        variants = re.search(r"^const MAX_RETRY_VARIANTS: usize = (\d+);", source, re.MULTILINE)
        default = re.search(r"^const DEFAULT_MAX_PASSES: usize = MAX_RETRY_VARIANTS \+ 1;", source,
                            re.MULTILINE)
        self.assertIsNotNone(variants, "MAX_RETRY_VARIANTS is no longer a plain literal: update this pin")
        self.assertIsNotNone(default, "DEFAULT_MAX_PASSES is no longer MAX_RETRY_VARIANTS + 1: update this pin")
        self.assertEqual(a.KNOB_INTEGERS[MAX_PASSES], (1, int(variants.group(1)) + 1))
        self.assertEqual(a.KNOB_INTEGERS[MAX_PASSES], (1, MAX_PASSES_HIGH))
        # configuration.md states the same default on its row.
        text = CONFIGURATION.read_text(encoding="utf-8")
        self.assertIn(f"| `{MAX_PASSES}` / `SYNTHPASS_OCR_MAX_SECONDS` | `{MAX_PASSES_HIGH}` /", text)

    def test_every_allowed_name_is_documented_in_configuration_md(self):
        text = CONFIGURATION.read_text(encoding="utf-8")
        for name in a.ALLOWED_ENV:
            with self.subTest(name=name):
                self.assertIn(name, text)

    def test_allowed_names_are_read_by_the_code_not_only_documented(self):
        # A name that configuration.md lists as gone (#473) must not be allowed: an A/B on it
        # compares two identical arms.
        text = CONFIGURATION.read_text(encoding="utf-8")
        gone = re.search(r"(`SYNTHPASS_[A-Z_]+` and `SYNTHPASS_[A-Z_]+` are gone)", text)
        self.assertIsNotNone(gone)
        for name in re.findall(r"SYNTHPASS_[A-Z_]+", gone.group(1)):
            self.assertNotIn(name, a.ALLOWED_ENV)

    def test_the_pin_is_not_allowed_and_no_allowed_name_is_a_refused_word(self):
        self.assertNotIn("SYNTHPASS_OCR_MAX_SECONDS", a.ALLOWED_ENV)
        for name in a.ALLOWED_ENV:
            self.assertRegex(name, r"^SYNTHPASS_[A-Z0-9_]+$")
            self.assertFalse(set(name.split("_")) & set(a.REFUSED_WORDS), name)
        self.assertEqual(len(set(a.ALLOWED_ENV)), len(a.ALLOWED_ENV))


class FormatsProfileNumbers(Refusals):
    def test_formats(self):
        self.assertEqual(a.validate(inputs(AB_FORMATS="mrvb, td1  mrva"))["formats"], ["td1", "mrva", "mrvb"])
        self.assertEqual(a.validate(inputs(AB_FORMATS="td2"))["formats"], ["td2"])
        for text in ("", " ", ",", "td4", "TD1", "td1 td1", "td1,td1", "td1;td2", "td1 all", "td1 td2",
                     "td1 $(id)", "*"):
            with self.subTest(text=text):
                self.refuses(AB_FORMATS=text)

    def test_profiles_match_what_the_bench_accepts(self):
        # The names `ProfileChoice::parse` lists in its error message.
        source = (REPO / "crates" / "synthpass-bench" / "src" / "lib.rs").read_text(encoding="utf-8")
        listed = re.search(r"\(valid: ([a-z, -]+)\)", source)
        self.assertIsNotNone(listed)
        self.assertEqual(tuple(p.strip() for p in listed.group(1).split(",")), a.PROFILES)
        for profile in a.PROFILES:
            self.assertEqual(a.validate(inputs(AB_PROFILE=profile))["profile"], profile)
        for text in ("", "Clean", "border_kiosk", "clean ", "clean\n", "none", "clean;id"):
            with self.subTest(text=text):
                self.refuses(AB_PROFILE=text)

    def test_count(self):
        for good in ("1", "100", "500"):
            self.assertEqual(a.validate(inputs(AB_COUNT=good))["count"], int(good))
        for bad in ("", "0", "501", "-1", "+5", "007", "1e2", "1.0", " 5", "5 ", "5\n", "٥", "99999999999", "x"):
            with self.subTest(count=bad):
                self.refuses(AB_COUNT=bad)

    def test_seed(self):
        for good in ("0", "1", "4294967295"):
            self.assertEqual(a.validate(inputs(AB_SEED=good))["seed"], int(good))
        for bad in ("", "-0", "00", "01", "4294967296", "+1", "0x10", "1_000", "1\n", "١", "99999999999999999999"):
            with self.subTest(seed=bad):
                self.refuses(AB_SEED=bad)

    def test_expect_identical(self):
        self.assertTrue(a.validate(inputs(AB_EXPECT_IDENTICAL="true"))["expect_identical"])
        for bad in ("", "True", "TRUE", "1", "yes", "false ", "true\n"):
            with self.subTest(value=bad):
                self.refuses(AB_EXPECT_IDENTICAL=bad)


class PlanFile(Refusals):
    def test_a_plan_round_trips_and_is_checked_again_on_load(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "plan.json"
            plan = a.validate(inputs(AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=band-first"))
            path.write_text(json.dumps(plan), encoding="utf-8")
            self.assertEqual(a.load_plan(path), plan)
            for key, bad in (("after_env", {"PATH": "x"}), ("after_env", {"SYNTHPASS_OCR_ORDER": "a b"}),
                             ("before_ref", "a b"), ("formats", ["td1", "td1"]), ("formats", []),
                             ("profile", "nope"), ("count", 501), ("count", True), ("seed", -1),
                             ("expect_identical", "true"), ("pinned", {"SYNTHPASS_OCR_MAX_SECONDS": "1"}),
                             ("schema", 2)):
                with self.subTest(key=key, bad=bad):
                    tampered = dict(plan, **{key: bad})
                    path.write_text(json.dumps(tampered), encoding="utf-8")
                    with self.assertRaises(a.Refused):
                        a.load_plan(path)
            path.write_text(json.dumps({k: v for k, v in plan.items() if k != "seed"}), encoding="utf-8")
            with self.assertRaises(a.Refused):
                a.load_plan(path)
            path.write_text("not json", encoding="utf-8")
            with self.assertRaises(a.Refused):
                a.load_plan(path)


class Messages(unittest.TestCase):
    """The one-line `::error::` a refusal prints must not carry a raw input's own line breaks or a
    workflow command."""

    def run_validate(self, **over):
        with tempfile.TemporaryDirectory() as tmp:
            stderr, stdout = io.StringIO(), io.StringIO()
            with mock.patch.dict(os.environ, inputs(**over), clear=False), \
                    contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(stdout):
                code = a.main(["validate", "--plan", str(Path(tmp) / "plan.json"),
                               "--github-output", str(Path(tmp) / "out.txt")])
            written = (Path(tmp) / "out.txt").read_text(encoding="utf-8") if (Path(tmp) / "out.txt").exists() else ""
            return code, stderr.getvalue(), stdout.getvalue(), written

    def test_a_refusal_is_exit_2_one_error_line_and_writes_nothing(self):
        hostile = "main\n::add-mask::secret\n::error::forged %0A"
        for variable in ("AB_AFTER_REF", "AB_BEFORE_REF", "AB_AFTER_ENV", "AB_BEFORE_ENV", "AB_FORMATS",
                         "AB_PROFILE", "AB_COUNT", "AB_SEED", "AB_EXPECT_IDENTICAL"):
            with self.subTest(variable=variable):
                code, stderr, stdout, written = self.run_validate(**{variable: hostile})
                self.assertEqual(code, 2)
                self.assertEqual(written, "")
                self.assertEqual(stdout, "")
                lines = stderr.splitlines()
                self.assertEqual(len(lines), 1)
                self.assertTrue(lines[0].startswith("::error::bench-ab: "))
                # Only the leading `::error::` is a workflow command: the input's own `::` and `%`
                # are replaced, so nothing in it can start or escape a command.
                self.assertEqual(stderr.count("::"), 2)
                self.assertNotIn("%", stderr)

    def test_a_long_input_is_truncated_in_the_message(self):
        _, stderr, _, _ = self.run_validate(AB_AFTER_REF="a b" + "z" * 500)
        self.assertLess(len(stderr), 400)

    def test_an_accepted_run_writes_the_plan_and_the_output_lines(self):
        with tempfile.TemporaryDirectory() as tmp:
            plan_path, out_path = Path(tmp) / "sub" / "plan.json", Path(tmp) / "out.txt"
            stdout = io.StringIO()
            with mock.patch.dict(os.environ, inputs(AB_COUNT="20"), clear=False), contextlib.redirect_stdout(stdout):
                self.assertEqual(a.main(["validate", "--plan", str(plan_path), "--github-output", str(out_path)]), 0)
            self.assertEqual(json.loads(plan_path.read_text(encoding="utf-8"))["count"], 20)
            self.assertIn("count=20", out_path.read_text(encoding="utf-8").splitlines())
            self.assertIn("bench-ab: inputs accepted", stdout.getvalue())


class RunAnArm(unittest.TestCase):
    """`run` with a stand-in binary that records its argv, its cwd and its environment."""

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        body = (
            "import json, os, sys\n"
            "print('--- seed 0 raw OCR lines ---')\n"
            "print('  [0] \"SYNTHETIC-ZONE-TEXT\"')\n"
            "out = sys.argv[sys.argv.index('--out') + 1]\n"
            "open(out, 'w').write(json.dumps({'argv': sys.argv[1:], 'cwd': os.getcwd(), "
            "'env': {k: v for k, v in os.environ.items() if k.startswith('SYNTHPASS_')}}))\n"
            "sys.exit(int(os.environ.get('FAKE_EXIT', '0')))\n")
        if os.name == "nt":
            # Windows starts no script by its #! line (WinError 193), so the stand-in is a .cmd
            # shim that hands every argument to the interpreter and returns its exit status.
            script = self.root / "fake-bench.py"
            script.write_text(body, encoding="utf-8")
            self.binary = self.root / "fake-bench.cmd"
            # Text mode writes "\n" as "\r\n" on Windows, which cmd expects.
            self.binary.write_text(f'@"{sys.executable}" "{script}" %*\n@exit /b %ERRORLEVEL%\n',
                                   encoding="utf-8")
        else:
            self.binary = self.root / "fake-bench"
            self.binary.write_text(f"#!{sys.executable}\n" + body, encoding="utf-8")
            self.binary.chmod(self.binary.stat().st_mode | stat.S_IXUSR)
        self.arm_dir = self.root / "arm-before"
        self.arm_dir.mkdir()
        self.plan = a.validate(inputs(
            AB_BEFORE_ENV="SYNTHPASS_OCR_ORDER=band-first", AB_AFTER_ENV="SYNTHPASS_OCR_SKEW=legacy",
            AB_COUNT="7", AB_SEED="12", AB_PROFILE="worn"))

    def run_arm(self, role="before", fmt="td3", env=None):
        stdout = io.StringIO()
        with mock.patch.dict(os.environ, env or {}, clear=False), contextlib.redirect_stdout(stdout):
            code = a.run_arm(self.plan, role, fmt, self.binary, self.arm_dir, self.root / "ab" / role)
        return code, stdout.getvalue()

    def test_argv_cwd_env_and_pinned_budget(self):
        code, printed = self.run_arm()
        self.assertEqual(code, 0)
        seen = json.loads((self.root / "ab" / "before" / "td3.json").read_text(encoding="utf-8"))
        self.assertEqual(seen["argv"][:12], ["--document-type", "td3", "--profile", "worn", "--count", "7",
                                            "--seed", "12", "--dump-ocr", "--out",
                                            str((self.root / "ab" / "before" / "td3.json").resolve())])
        self.assertEqual(Path(seen["cwd"]).resolve(), self.arm_dir.resolve())
        self.assertEqual(seen["env"], {"SYNTHPASS_OCR_ORDER": "band-first", "SYNTHPASS_OCR_MAX_SECONDS": "600"})

    def test_the_other_arm_gets_its_own_assignments(self):
        self.run_arm(role="after")
        seen = json.loads((self.root / "ab" / "after" / "td3.json").read_text(encoding="utf-8"))
        self.assertEqual(seen["env"], {"SYNTHPASS_OCR_SKEW": "legacy", "SYNTHPASS_OCR_MAX_SECONDS": "600"})

    def test_an_inherited_synthpass_variable_never_reaches_an_arm(self):
        self.run_arm(env={"SYNTHPASS_OCR_MAX_SECONDS": "1", "SYNTHPASS_OCR_ROTATE": "off",
                          "SYNTHPASS_OCR_MODEL_DIR": "/elsewhere"})
        seen = json.loads((self.root / "ab" / "before" / "td3.json").read_text(encoding="utf-8"))
        self.assertEqual(seen["env"], {"SYNTHPASS_OCR_ORDER": "band-first", "SYNTHPASS_OCR_MAX_SECONDS": "600"})

    def test_standard_output_goes_to_a_file_and_not_to_the_log(self):
        code, printed = self.run_arm()
        self.assertNotIn("SYNTHETIC-ZONE-TEXT", printed)
        text = (self.root / "ab" / "before" / "td3.stdout").read_text(encoding="utf-8")
        self.assertIn("--- seed 0 raw OCR lines ---", text)
        self.assertIn("SYNTHETIC-ZONE-TEXT", text)

    def test_the_exit_status_is_the_bench_s(self):
        code, _ = self.run_arm(env={"FAKE_EXIT": "3"})
        self.assertEqual(code, 3)

    def test_a_format_or_role_outside_the_plan_is_refused(self):
        self.plan["formats"] = ["td1"]
        with self.assertRaises(a.Refused):
            self.run_arm(fmt="td3")
        with self.assertRaises(a.Refused):
            self.run_arm(role="both", fmt="td1")


class ArmRecord(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.binary = Path(self.tmp.name) / "synthpass-bench"
        self.binary.write_bytes(b"not really a binary")
        self.plan = a.validate(inputs(AB_AFTER_ENV="SYNTHPASS_OCR_ORDER=band-first"))

    def test_keys_are_exactly_the_documented_ones(self):
        arm = a.build_arm(self.plan, "after", SHA, self.binary, context())
        self.assertEqual(set(arm), {"schema", "role", "ref", "git_sha", "env", "pinned", "binary_sha256",
                                    "context"})
        self.assertEqual(arm["schema"], 1)
        self.assertEqual(arm["role"], "after")
        self.assertEqual(arm["ref"], "claude/some-branch")
        self.assertEqual(arm["git_sha"], SHA)
        self.assertEqual(arm["env"], {"SYNTHPASS_OCR_ORDER": "band-first"})
        self.assertEqual(arm["pinned"], {"SYNTHPASS_OCR_MAX_SECONDS": "600"})
        self.assertRegex(arm["binary_sha256"], r"^[0-9a-f]{64}$")
        self.assertEqual(arm["binary_sha256"], a.sha256_of(self.binary))

    def test_the_context_is_the_nightly_s_and_passes_its_validator(self):
        arm = a.build_arm(self.plan, "after", SHA, self.binary, context())
        self.assertEqual(set(arm["context"]), set(nightly.CONTEXT_KEYS))
        nightly.validate_context(arm["context"])

    def test_ocr_env_is_the_arm_s_own_assignments_plus_the_pin(self):
        before = a.build_arm(self.plan, "before", SHA, self.binary, context(ocr_env={"SYNTHPASS_OCR_ROTATE": "off"}))
        after = a.build_arm(self.plan, "after", SHA, self.binary, context())
        self.assertEqual(before["context"]["ocr_env"], {"SYNTHPASS_OCR_MAX_SECONDS": "600"})
        self.assertEqual(after["context"]["ocr_env"],
                         {"SYNTHPASS_OCR_MAX_SECONDS": "600", "SYNTHPASS_OCR_ORDER": "band-first"})

    def test_a_bad_commit_role_or_context_is_refused(self):
        for sha in ("main", "A" * 40, "a" * 39, "a" * 41, ""):
            with self.subTest(sha=sha), self.assertRaises(a.Refused):
                a.build_arm(self.plan, "after", sha, self.binary, context())
        with self.assertRaises(a.Refused):
            a.build_arm(self.plan, "middle", SHA, self.binary, context())
        with self.assertRaises(a.Refused):
            a.build_arm(self.plan, "after", SHA, self.binary, context(nproc=0))
        broken = context()
        del broken["rustc"]
        with self.assertRaises(a.Refused):
            a.build_arm(self.plan, "after", SHA, self.binary, broken)

    def test_check_arm_refuses_a_tampered_record(self):
        arm = a.build_arm(self.plan, "after", SHA, self.binary, context())
        for key, bad in (("role", "x"), ("git_sha", "z" * 40), ("binary_sha256", "z" * 64), ("ref", "a b"),
                         ("env", {"PATH": "x"}), ("pinned", {"SYNTHPASS_OCR_MAX_SECONDS": "1"}), ("schema", 2)):
            with self.subTest(key=key), self.assertRaises(a.Refused):
                a.check_arm(dict(arm, **{key: bad}))
        with self.assertRaises(a.Refused):
            a.check_arm(dict(arm, extra=1))
        for bad in ({"SYNTHPASS_OCR_ORDER": "bandfirst"}, {"SYNTHPASS_OCR_ORDER": "BAND-FIRST"},
                    {MAX_PASSES: "15"}, {MAX_PASSES: "014"}):
            with self.subTest(env=bad), self.assertRaises(a.Refused):
                a.check_arm(dict(arm, env=bad, context=dict(
                    arm["context"], ocr_env=dict(sorted({**bad, **arm["pinned"]}.items())))))
        with self.assertRaises(a.Refused):
            a.check_arm(dict(arm, env={}))  # ocr_env no longer matches env plus the pin

    def test_summary_lines_carry_only_validated_tokens_and_counts(self):
        before = a.build_arm(self.plan, "before", SHA, self.binary, context())
        after = a.build_arm(self.plan, "after", "c" * 40, self.binary, context())
        text = "\n".join(a.summary_lines(self.plan, before, after))
        self.assertIn("before: ref main, commit " + SHA, text)
        self.assertIn("after: ref claude/some-branch, commit " + "c" * 40, text)
        self.assertIn("env SYNTHPASS_OCR_ORDER=band-first", text)
        self.assertIn("env (none: shipped defaults)", text)
        self.assertIn("pinned on both arms: SYNTHPASS_OCR_MAX_SECONDS=600", text)

    def test_the_command_line_writes_and_reads_the_record(self):
        plan_path = Path(self.tmp.name) / "plan.json"
        plan_path.write_text(json.dumps(self.plan), encoding="utf-8")
        out = Path(self.tmp.name) / "ab" / "after" / "arm.json"
        with mock.patch.object(nightly, "collect_context", return_value=context()):
            code = a.main(["arm-json", "--plan", str(plan_path), "--role", "after", "--git-sha", SHA,
                           "--binary", str(self.binary), "--out", str(out)])
        self.assertEqual(code, 0)
        self.assertEqual(a.load_arm(out)["role"], "after")
        stdout = io.StringIO()
        with contextlib.redirect_stdout(stdout):
            self.assertEqual(a.main(["summary", "--plan", str(plan_path), "--before", str(out),
                                     "--after", str(out)]), 0)
        self.assertIn("binary sha256", stdout.getvalue())


if __name__ == "__main__":
    unittest.main()
