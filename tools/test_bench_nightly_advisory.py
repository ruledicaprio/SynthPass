"""Tests for bench_nightly_advisory.py: hand-built rows, no network, no bench-data checkout.

Nights are built from a few helpers below. The only git the tests touch is a throwaway repository
in a temporary directory, for the generator-change check.
"""
import contextlib
import io
import json
import os
import re
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import bench_nightly_advisory as a
import bench_nightly_rows as n

FORMATS = n.FORMATS
FRESH_PROFILES = ("clean", "mobile", "scanner", "worn", "border-kiosk")
BASE_TIME = 1_790_000_000


def make_row(run_id, fmt, slice_name, seed, *, hit=True, wrong=(), retry_stop="general_valid", render=None,
             profile="clean"):
    """One validated schema-2 row. `wrong` names the fields with a non-zero CER."""
    cer = {name: 0.0 for name in n.FIELD_NAMES}
    for name in wrong:
        cer[name] = 0.5
    return n.validate_row(
        {
            "schema": 2,
            "run_id": run_id,
            "slice": slice_name,
            "document_type": fmt,
            "seed": seed,
            "profile": profile,
            "render_sha256": render or f"{seed:064x}",
            "hit": hit,
            "miss_kind": None if hit else "no_mrz_found",
            "check_states": {"document_number": True, "date_of_birth": True, "date_of_expiry": True,
                             "personal_number": None, "composite": True} if hit else None,
            "field_cer": cer if hit else None,
            "name_error": None,
            "line1_flagged": False,
            "retry_stop": retry_stop,
            "retry_variant_id": None,
            "retry_damaged_recovery": None,
            "tier1_damaged_recovery": False,
            "ocr_pass_count": None,
            "ocr_passes": None,
            "elapsed_ms": 1500,
        }
    )


def make_header(run_id, fmt, slice_name, sha, stamp, rows):
    return {
        "schema": 2,
        "run_id": run_id,
        "slice": slice_name,
        "document_type": fmt,
        "git_sha": sha,
        "event": "schedule",
        "run_timestamp_unix": stamp,
        "generator_fingerprint": n.generator_fingerprint(rows) if slice_name == "fixed" else None,
        "counts": {"count": len(rows)},
    }


def night_ids(number):
    return f"{1000 + number}-1", BASE_TIME + number * 86400, f"{number:040x}"


def add_night(history, number, *, salt="", fixed=None, fresh_hits=170, fresh_wrong=5, formats=FORMATS,
              fixed_rows_edit=None, fresh_rows_edit=None):
    """Append one night. `fixed` maps a format to `{seed: make_row kwargs}` for the seeds that differ
    from the clean default; `salt` changes every fixed render hash (a re-render)."""
    run_id, stamp, sha = night_ids(number)
    for fmt in formats:
        overrides = (fixed or {}).get(fmt, {})
        fixed_rows = [
            make_row(run_id, fmt, "fixed", seed, render=f"{seed:032x}{salt:0<32}", **overrides.get(seed, {}))
            for seed in a.FIXED_SEEDS
        ]
        fresh_rows = []
        for i in range(200):
            fresh_rows.append(
                make_row(
                    run_id, fmt, "fresh", 100 + number * 200 + i,
                    hit=i < fresh_hits,
                    wrong=("surname",) if i < fresh_wrong else (),
                    profile=FRESH_PROFILES[i % len(FRESH_PROFILES)],
                )
            )
        header_fixed = make_header(run_id, fmt, "fixed", sha, stamp, fixed_rows)
        header_fresh = make_header(run_id, fmt, "fresh", sha, stamp, fresh_rows)
        if fixed_rows_edit:
            fixed_rows = fixed_rows_edit(fmt, fixed_rows)
        if fresh_rows_edit:
            fresh_rows = fresh_rows_edit(fmt, fresh_rows)
        history.fixed.extend(fixed_rows)
        history.fresh.extend(fresh_rows)
        history.headers.extend([header_fresh, header_fixed])
    return run_id


def judge(history, run_id, changed=None):
    return a.analyse(history, run_id, lambda old, new: changed)


def codes(result, kind=None, prefix=""):
    return [f.code for f in result["findings"] if (kind is None or f.kind == kind) and f.code.startswith(prefix)]


class StatisticsTests(unittest.TestCase):
    def test_z_is_signed_and_pooled(self):
        z = a.two_proportion_z(1127, 1400, 1190, 1400)
        self.assertLessEqual(z, -a.Z_FLAG)
        self.assertGreaterEqual(a.two_proportion_z(1190, 1400, 1127, 1400), a.Z_FLAG)

    def test_just_inside_the_threshold_is_not_a_flag(self):
        z = a.two_proportion_z(1134, 1400, 1190, 1400)
        self.assertGreater(z, -a.Z_FLAG)
        self.assertAlmostEqual(z, -2.82, places=2)

    def test_undefined_cases_are_none(self):
        self.assertIsNone(a.two_proportion_z(0, 0, 5, 10))
        self.assertIsNone(a.two_proportion_z(0, 100, 0, 100))  # no variance
        self.assertIsNone(a.two_proportion_z(100, 100, 100, 100))


class FixedSliceTests(unittest.TestCase):
    def two_nights(self, **second):
        history = a.History()
        add_night(history, 1)
        run_id = add_night(history, 2, **second)
        return history, run_id

    def test_bootstrap_has_no_reference_and_is_green(self):
        history = a.History()
        run_id = add_night(history, 1)
        result = judge(history, run_id)
        self.assertFalse(result["red"])
        self.assertEqual(result["findings"], [])
        for section in result["formats"].values():
            self.assertEqual(section["fixed"]["status"], "no_reference")
            self.assertIsNone(section["reference_run_id"])
            self.assertEqual(section["fresh"]["status"], "too_little_history")

    def test_a_clean_pair_has_no_flip(self):
        history, run_id = self.two_nights()
        result = judge(history, run_id)
        self.assertFalse(result["red"])
        self.assertEqual(result["findings"], [])
        td1 = result["formats"]["td1"]
        self.assertEqual(td1["fixed"]["status"], "paired")
        self.assertEqual(td1["reference_run_id"], night_ids(1)[0])
        self.assertEqual(td1["fixed"]["flipped"], {})

    def test_a_flipped_seed_is_named_and_is_not_red(self):
        history, run_id = self.two_nights(fixed={"td2": {7: {"hit": False}}})
        result = judge(history, run_id)
        self.assertFalse(result["red"])
        self.assertEqual(result["formats"]["td2"]["fixed"]["flipped"], {7: ["hit_lost", "correct_lost"]})
        self.assertEqual(result["formats"]["td2"]["fixed"]["net_hit_loss"], 1)
        self.assertEqual(result["formats"]["td1"]["fixed"]["flipped"], {})
        self.assertIn("td2 seed 7: hit_lost, correct_lost", a.render_summary(result))

    def test_a_new_wrong_accept_is_red(self):
        history, run_id = self.two_nights(fixed={"td1": {5: {"wrong": ("surname",)}}})
        result = judge(history, run_id)
        self.assertTrue(result["red"])
        self.assertEqual(codes(result, "red"), ["td1:new_wrong_accept"])
        self.assertEqual(result["formats"]["td1"]["fixed"]["new_wrong_accept_seeds"], [5])
        self.assertEqual(result["formats"]["td1"]["fixed"]["flipped"][5], ["correct_lost", "wrong_accept_new"])

    def test_a_new_prefix_wrong_read_is_red(self):
        history, run_id = self.two_nights(fixed={"td3": {9: {"wrong": ("issuing_country",)}}})
        result = judge(history, run_id)
        self.assertTrue(result["red"])
        self.assertEqual(sorted(codes(result, "red")), ["td3:new_prefix_wrong", "td3:new_wrong_accept"])
        self.assertEqual(result["formats"]["td3"]["fixed"]["new_prefix_wrong_seeds"], [9])

    def test_a_wrong_accept_that_was_already_there_is_not_new(self):
        history = a.History()
        add_night(history, 1, fixed={"td1": {3: {"wrong": ("surname",)}}})
        run_id = add_night(history, 2, fixed={"td1": {3: {"wrong": ("surname",)}}})
        result = judge(history, run_id)
        self.assertEqual(result["findings"], [])
        self.assertEqual(result["formats"]["td1"]["fixed"]["wrong_accepts"], 1)

    def test_a_wrong_accept_that_clears_is_named_not_flagged(self):
        history = a.History()
        add_night(history, 1, fixed={"td1": {3: {"wrong": ("surname",)}}})
        run_id = add_night(history, 2)
        result = judge(history, run_id)
        self.assertEqual(result["findings"], [])
        self.assertEqual(result["formats"]["td1"]["fixed"]["flipped"], {3: ["correct_gained", "wrong_accept_cleared"]})

    def test_many_lost_hits_warn_above_the_threshold(self):
        lose = {seed: {"hit": False} for seed in range(a.NET_FLIP_WARN_ABOVE + 1)}
        history, run_id = self.two_nights(fixed={"mrva": lose})
        result = judge(history, run_id)
        self.assertFalse(result["red"])
        self.assertEqual(codes(result, "warning"), ["mrva:net_hit_loss"])
        at_threshold = {seed: {"hit": False} for seed in range(a.NET_FLIP_WARN_ABOVE)}
        history, run_id = self.two_nights(fixed={"mrva": at_threshold})
        self.assertEqual(judge(history, run_id)["findings"], [])

    def test_gained_hits_net_against_lost_hits(self):
        history = a.History()
        add_night(history, 1, fixed={"td1": {seed: {"hit": False} for seed in range(10, 20)}})
        run_id = add_night(history, 2, fixed={"td1": {seed: {"hit": False} for seed in range(0, 10)}})
        result = judge(history, run_id)
        self.assertEqual(result["formats"]["td1"]["fixed"]["net_hit_loss"], 0)
        self.assertEqual(len(result["formats"]["td1"]["fixed"]["flipped"]), 20)
        self.assertEqual(result["findings"], [])

    def test_a_budget_stop_in_the_fixed_slice_is_red(self):
        history, run_id = self.two_nights(fixed={"td1": {4: {"retry_stop": "budget"}}})
        result = judge(history, run_id)
        self.assertTrue(result["red"])
        self.assertEqual(codes(result, "red"), ["td1:fixed_budget_stop"])

    def test_a_budget_stop_in_the_fresh_slice_only_warns(self):
        def stop_one(fmt, rows):
            rows[0] = make_row(rows[0]["run_id"], fmt, "fresh", rows[0]["seed"], hit=False, retry_stop="budget",
                               profile=rows[0]["profile"])
            return rows

        history = a.History()
        run_id = add_night(history, 1, fresh_rows_edit=stop_one)
        result = judge(history, run_id)
        self.assertFalse(result["red"])
        self.assertEqual(codes(result, "warning", "td1"), ["td1:fresh_budget_stop"])

    def test_duplicate_fixed_rows_are_red(self):
        def repeat_a_seed(fmt, rows):
            return rows + [rows[0]] if fmt == "td1" else rows

        history = a.History()
        run_id = add_night(history, 1, fixed_rows_edit=repeat_a_seed)
        result = judge(history, run_id)
        self.assertTrue(result["red"])
        self.assertIn("td1:duplicate_fixed_rows", codes(result, "red"))
        self.assertEqual(result["formats"]["td1"]["fixed"]["status"], "invalid")

    def test_missing_fixed_rows_are_red(self):
        history = a.History()
        run_id = add_night(history, 1, fixed_rows_edit=lambda fmt, rows: rows[:-1] if fmt == "td2" else rows)
        result = judge(history, run_id)
        self.assertIn("td2:missing_fixed_rows", codes(result, "red"))

    def test_duplicate_fresh_rows_are_red(self):
        history = a.History()
        run_id = add_night(history, 1, fresh_rows_edit=lambda fmt, rows: rows + rows[:1] if fmt == "td3" else rows)
        result = judge(history, run_id)
        self.assertIn("td3:duplicate_fresh_rows", codes(result, "red"))

    def test_a_format_with_no_rows_or_headers_is_red(self):
        history = a.History()
        run_id = add_night(history, 1, formats=("td1", "td2", "td3", "mrva"))
        result = judge(history, run_id)
        self.assertTrue(result["red"])
        self.assertEqual(
            sorted(codes(result, "red", "mrvb")),
            ["mrvb:missing_fixed_header", "mrvb:missing_fixed_rows", "mrvb:missing_fresh_header",
             "mrvb:missing_fresh_rows"],
        )
        self.assertEqual(result["formats"]["mrvb"]["fixed"]["status"], "invalid")

    def test_a_run_that_is_not_in_the_data_is_red_everywhere(self):
        history = a.History()
        add_night(history, 1)
        result = judge(history, "999-1")
        self.assertTrue(result["red"])
        self.assertEqual(len(codes(result, "red", "td1")), 4)

    def test_a_duplicate_header_is_red(self):
        history = a.History()
        run_id = add_night(history, 1)
        history.headers.append(dict(history.headers[0]))
        result = judge(history, run_id)
        self.assertIn(f"{history.headers[0]['document_type']}:duplicate_fresh_header", codes(result, "red"))

    def test_a_header_that_disagrees_with_its_rows_is_red(self):
        history = a.History()
        run_id = add_night(history, 1)
        for header in history.headers:
            if header["slice"] == "fixed" and header["document_type"] == "td1":
                header["generator_fingerprint"] = "0" * 64
        result = judge(history, run_id)
        self.assertEqual(codes(result, "red"), ["td1:fingerprint_header_mismatch"])


class FingerprintTests(unittest.TestCase):
    def render_change(self, changed):
        history = a.History()
        add_night(history, 1)
        run_id = add_night(history, 2, salt="f")
        return judge(history, run_id, changed)

    def test_a_fingerprint_change_without_a_generator_change_is_red(self):
        result = self.render_change(False)
        self.assertTrue(result["red"])
        self.assertEqual(codes(result, "red", "td1"), ["td1:fingerprint_changed_without_generator_change"])
        self.assertEqual(result["formats"]["td1"]["fixed"]["status"], "no_reference")

    def test_a_fingerprint_change_with_a_generator_change_resets_the_reference(self):
        result = self.render_change(True)
        self.assertFalse(result["red"])
        self.assertEqual(result["findings"], [])
        self.assertEqual(result["formats"]["td1"]["fixed"]["status"], "no_reference")

    def test_a_fingerprint_change_that_cannot_be_decided_only_warns(self):
        result = self.render_change(None)
        self.assertFalse(result["red"])
        self.assertEqual(codes(result, "warning", "td1"), ["td1:fingerprint_change_undecided"])

    def test_the_check_names_the_previous_run_and_the_two_shas(self):
        seen = []
        history = a.History()
        add_night(history, 1)
        run_id = add_night(history, 2, salt="f")
        a.analyse(history, run_id, lambda old, new: seen.append((old, new)))
        self.assertEqual(seen[0], (night_ids(1)[2], night_ids(2)[2]))

    def test_the_reference_skips_a_night_with_another_fingerprint(self):
        history = a.History()
        add_night(history, 1)
        add_night(history, 2, salt="f")
        run_id = add_night(history, 3)
        result = judge(history, run_id, changed=True)
        self.assertEqual(result["formats"]["td1"]["reference_run_id"], night_ids(1)[0])
        self.assertEqual(result["formats"]["td1"]["fixed"]["status"], "paired")

    def test_the_reference_is_the_most_recent_same_fingerprint_night(self):
        history = a.History()
        add_night(history, 1, fixed={"td1": {1: {"hit": False}}})
        add_night(history, 2)
        run_id = add_night(history, 3)
        result = judge(history, run_id)
        self.assertEqual(result["formats"]["td1"]["reference_run_id"], night_ids(2)[0])
        self.assertEqual(result["formats"]["td1"]["fixed"]["flipped"], {})

    def test_a_later_night_is_never_a_reference(self):
        history = a.History()
        run_id = add_night(history, 1)
        add_night(history, 2, fixed={"td1": {1: {"wrong": ("surname",)}}})
        result = judge(history, run_id)
        self.assertEqual(result["formats"]["td1"]["fixed"]["status"], "no_reference")


class GeneratorChangeTests(unittest.TestCase):
    """The real git check, against a throwaway repository."""

    def git(self, repo, *args):
        env = {**os.environ, "GIT_CONFIG_GLOBAL": os.devnull, "GIT_CONFIG_SYSTEM": os.devnull}
        done = subprocess.run(
            ["git", "-C", str(repo), "-c", "user.name=t", "-c", "user.email=t@example.invalid",
             "-c", "commit.gpgsign=false", *args],
            capture_output=True, text=True, check=True, env=env,
        )
        return done.stdout.strip()

    def commit(self, repo, path, text):
        target = Path(repo) / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
        self.git(repo, "add", "-A")
        self.git(repo, "commit", "-q", "-m", "c")
        return self.git(repo, "rev-parse", "HEAD")

    def test_the_generator_tree_decides(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.git(tmp, "init", "-q")
            base = self.commit(tmp, "crates/synthpass-gen/src/lib.rs", "one")
            elsewhere = self.commit(tmp, "crates/synthpass-ocr/src/lib.rs", "x")
            font = self.commit(tmp, "crates/synthpass-gen/fonts/ocr-b.ttf", "font")
            self.assertFalse(a.generator_changed(Path(tmp), base, elsewhere))
            self.assertTrue(a.generator_changed(Path(tmp), base, font))
            self.assertTrue(a.generator_changed(Path(tmp), elsewhere, font))
            self.assertFalse(a.generator_changed(Path(tmp), base, base))

    def test_the_zone_crate_and_the_lockfile_are_generator_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.git(tmp, "init", "-q")
            base = self.commit(tmp, "crates/synthpass-gen/src/lib.rs", "one")
            zone = self.commit(tmp, "crates/mrz/src/checksum.rs", "check digits")
            lock = self.commit(tmp, "Cargo.lock", "image 0.25")
            bench = self.commit(tmp, "crates/synthpass-bench/src/lib.rs", "bench")
            self.assertTrue(a.generator_changed(Path(tmp), base, zone))
            self.assertTrue(a.generator_changed(Path(tmp), zone, lock))
            self.assertFalse(a.generator_changed(Path(tmp), lock, bench))

    def test_an_unreachable_commit_or_no_checkout_cannot_be_decided(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.git(tmp, "init", "-q")
            head = self.commit(tmp, "a.txt", "a")
            self.assertIsNone(a.generator_changed(Path(tmp), "0" * 40, head))
            self.assertIsNone(a.generator_changed(Path(tmp), head, "1" * 40))
            self.assertIsNone(a.generator_changed(None, head, head))
            self.assertIsNone(a.generator_changed(Path(tmp), "not-a-sha", head))
            self.assertIsNone(a.generator_changed(Path(tmp) / "missing", head, head))


class FreshSliceTests(unittest.TestCase):
    NIGHTS = a.RECENT_NIGHTS + a.MIN_BASELINE_NIGHTS

    def history(self, baseline, recent):
        """`baseline` and `recent` are `(hits, wrong)` per night; the last recent night is tonight."""
        history = a.History()
        run_id = None
        for number, (hits, wrong) in enumerate(baseline + recent, start=1):
            run_id = add_night(history, number, fresh_hits=hits, fresh_wrong=wrong)
        return history, run_id

    def test_a_drop_at_z_minus_three_flags_the_hit_and_correct_read_rates(self):
        baseline = [(170, 0)] * a.MIN_BASELINE_NIGHTS
        recent = [(161, 0)] * a.RECENT_NIGHTS
        history, run_id = self.history(baseline, recent)
        result = judge(history, run_id)
        self.assertFalse(result["red"])
        self.assertEqual(sorted(codes(result, "warning", "td1")), ["td1:fresh_correct_rate", "td1:fresh_hit_rate"])
        fresh = result["formats"]["td1"]["fresh"]
        self.assertEqual(fresh["status"], "tested")
        self.assertLessEqual(fresh["z"]["hit_rate"], -a.Z_FLAG)
        self.assertEqual(fresh["recent"]["nights"], a.RECENT_NIGHTS)
        self.assertEqual(fresh["baseline"]["nights"], a.MIN_BASELINE_NIGHTS)

    def test_a_drop_at_z_minus_2_9_is_no_flag(self):
        baseline = [(170, 0)] * a.MIN_BASELINE_NIGHTS
        recent = [(162, 0)] * a.RECENT_NIGHTS
        history, run_id = self.history(baseline, recent)
        result = judge(history, run_id)
        fresh = result["formats"]["td1"]["fresh"]
        self.assertGreater(fresh["z"]["hit_rate"], -a.Z_FLAG)
        self.assertLess(fresh["z"]["hit_rate"], -2.8)
        self.assertEqual(result["findings"], [])

    def test_a_rise_in_wrong_accepts_flags_only_the_wrong_accept_rate(self):
        baseline = [(170, 5)] * a.MIN_BASELINE_NIGHTS
        recent = [(170, 10)] * a.RECENT_NIGHTS
        history, run_id = self.history(baseline, recent)
        result = judge(history, run_id)
        self.assertEqual(codes(result, "warning", "td1"), ["td1:fresh_wrong_accept_rate"])
        self.assertGreaterEqual(result["formats"]["td1"]["fresh"]["z"]["wrong_accept_rate"], a.Z_FLAG)

    def test_an_improvement_is_never_a_flag(self):
        baseline = [(161, 10)] * a.MIN_BASELINE_NIGHTS
        recent = [(175, 0)] * a.RECENT_NIGHTS
        history, run_id = self.history(baseline, recent)
        self.assertEqual(judge(history, run_id)["findings"], [])

    def test_too_little_history_skips_the_test(self):
        history, run_id = self.history([(170, 0)] * (a.MIN_BASELINE_NIGHTS - 1), [(120, 0)] * a.RECENT_NIGHTS)
        result = judge(history, run_id)
        fresh = result["formats"]["td1"]["fresh"]
        self.assertEqual(fresh["status"], "too_little_history")
        self.assertEqual(fresh["z"], {"hit_rate": None, "correct_rate": None, "wrong_accept_rate": None})
        self.assertEqual(result["findings"], [])
        self.assertIn("too little history", a.render_summary(result))

    def test_a_short_recent_window_skips_the_test(self):
        history, run_id = self.history([], [(120, 0)] * (a.RECENT_NIGHTS - 1))
        result = judge(history, run_id)
        fresh = result["formats"]["td1"]["fresh"]
        self.assertEqual(fresh["status"], "too_little_history")
        self.assertEqual(fresh["recent"]["nights"], a.RECENT_NIGHTS - 1)

    def test_only_nights_with_the_same_fingerprint_are_pooled(self):
        history = a.History()
        for number in range(1, a.MIN_BASELINE_NIGHTS + 1):
            add_night(history, number, salt="e", fresh_hits=170)  # another render: not this baseline
        for number in range(a.MIN_BASELINE_NIGHTS + 1, a.MIN_BASELINE_NIGHTS + a.RECENT_NIGHTS + 1):
            run_id = add_night(history, number, fresh_hits=120)
        result = judge(history, run_id, changed=True)
        fresh = result["formats"]["td1"]["fresh"]
        self.assertEqual(fresh["status"], "too_little_history")
        self.assertEqual(fresh["baseline"]["nights"], 0)
        self.assertEqual(result["findings"], [])

    def test_the_windows_are_capped(self):
        history, run_id = self.history([(170, 0)] * (a.BASELINE_NIGHTS + 5), [(170, 0)] * a.RECENT_NIGHTS)
        fresh = judge(history, run_id)["formats"]["td1"]["fresh"]
        self.assertEqual(fresh["recent"]["nights"], a.RECENT_NIGHTS)
        self.assertEqual(fresh["baseline"]["nights"], a.BASELINE_NIGHTS)
        self.assertEqual(fresh["baseline"]["count"], a.BASELINE_NIGHTS * 200)


class OutputTests(unittest.TestCase):
    def flagged_night(self):
        history = a.History()
        for number in range(1, 15):
            add_night(history, number)
        run_id = add_night(
            history, 15,
            fixed={"td1": {5: {"wrong": ("document_type",)}, 6: {"hit": False}}},
            fresh_hits=150,
        )
        return history, run_id

    def walk(self, value):
        if isinstance(value, dict):
            for key, item in value.items():
                yield key
                yield from self.walk(item)
        elif isinstance(value, list):
            for item in value:
                yield from self.walk(item)
        else:
            yield value

    def test_the_advisory_line_holds_aggregates_only(self):
        history, run_id = self.flagged_night()
        result = judge(history, run_id)
        line = a.advisory_line(result)
        allowed = a.ADVISORY_KEYS | set(n.FORMATS)

        def keys(value):
            if isinstance(value, dict):
                for key, item in value.items():
                    yield key
                    yield from keys(item)
            elif isinstance(value, list):
                for item in value:
                    yield from keys(item)

        self.assertTrue(set(keys(line)) <= allowed, set(keys(line)) - allowed)
        strings = [v for v in self.walk(line) if isinstance(v, str)]
        self.assertTrue(strings)
        for text in strings:
            self.assertLessEqual(len(text), 64)
            self.assertRegex(text, r"^[A-Za-z0-9_.:-]+$")
        # keys are tokens too
        for key in keys(line):
            self.assertRegex(key, r"^[A-Za-z0-9_]+$")

    def test_the_advisory_line_shape(self):
        history, run_id = self.flagged_night()
        line = a.advisory_line(judge(history, run_id))
        self.assertEqual(line["schema"], a.ADVISORY_SCHEMA)
        self.assertEqual(line["run_id"], run_id)
        self.assertEqual(line["git_sha"], night_ids(15)[2])
        self.assertTrue(line["red"])
        self.assertEqual(line["red_codes"], ["td1:new_prefix_wrong", "td1:new_wrong_accept"])
        self.assertEqual(sorted(line["formats"]), sorted(n.FORMATS))
        td1 = line["formats"]["td1"]
        self.assertEqual(td1["reference_run_id"], night_ids(14)[0])
        self.assertEqual(len(td1["generator_fingerprint"]), 64)
        self.assertEqual(td1["fixed"]["flipped_seeds"], [5, 6])
        self.assertEqual(td1["fixed"]["new_wrong_accept_seeds"], [5])
        self.assertEqual(td1["fixed"]["new_prefix_wrong_seeds"], [5])
        self.assertEqual(td1["fresh"]["status"], "tested")
        self.assertEqual(line["warning_codes"], [])
        json.dumps(line)  # serialisable

    def test_a_line_with_a_stray_key_or_a_sentence_is_refused(self):
        history, run_id = self.flagged_night()
        line = a.advisory_line(judge(history, run_id))
        line["formats"]["td1"]["reason"] = "x"
        with self.assertRaises(a.AdvisoryError):
            a.validate_advisory_line(line)
        line = a.advisory_line(judge(history, run_id))
        line["red_codes"].append("a sentence with spaces")
        with self.assertRaises(a.AdvisoryError):
            a.validate_advisory_line(line)
        line = a.advisory_line(judge(history, run_id))
        line["git_sha"] = "x" * 65
        with self.assertRaises(a.AdvisoryError):
            a.validate_advisory_line(line)

    def test_the_summary_names_flips_and_findings_and_no_row_text(self):
        history, run_id = self.flagged_night()
        text = a.render_summary(judge(history, run_id))
        self.assertIn(f"run {run_id}", text)
        self.assertIn("td1 seed 5:", text)
        self.assertIn("newly a wrong accept", text)
        self.assertIn("z ", text)
        self.assertNotIn("reason", text.lower().replace("reference", ""))

    def test_the_summary_says_no_reference_in_the_bootstrap(self):
        history = a.History()
        run_id = add_night(history, 1)
        text = a.render_summary(judge(history, run_id))
        self.assertIn("no reference", text)
        self.assertIn("too little history", text)
        self.assertIn("**Green.**", text)

    def test_annotations_are_one_line_per_finding(self):
        history, run_id = self.flagged_night()
        result = judge(history, run_id)
        lines = a.annotations(result)
        self.assertEqual(len(lines), len(result["findings"]))
        for line in lines:
            self.assertRegex(line, r"^::(error|warning)::[^\n]+$")


class ReadingTests(unittest.TestCase):
    def write(self, directory, history):
        directory = Path(directory)
        (directory / "fixed.jsonl").write_text("".join(json.dumps(r) + "\n" for r in history.fixed), encoding="utf-8")
        (directory / "fresh.jsonl").write_text("".join(json.dumps(r) + "\n" for r in history.fresh), encoding="utf-8")
        (directory / "runs.jsonl").write_text("".join(json.dumps(h) + "\n" for h in history.headers), encoding="utf-8")

    def test_the_files_round_trip_and_dataset_jsonl_is_ignored(self):
        history = a.History()
        add_night(history, 1)
        with tempfile.TemporaryDirectory() as tmp:
            self.write(tmp, history)
            (Path(tmp) / "dataset.jsonl").write_text("this is not json\n", encoding="utf-8")
            loaded = a.load_history(tmp)
        self.assertEqual(len(loaded.fixed), len(history.fixed))
        self.assertEqual(len(loaded.fresh), len(history.fresh))
        self.assertEqual(len(loaded.headers), len(history.headers))

    def test_missing_files_read_as_empty(self):
        with tempfile.TemporaryDirectory() as tmp:
            loaded = a.load_history(tmp)
        self.assertEqual((loaded.fixed, loaded.fresh, loaded.headers), ([], [], []))

    def test_a_row_with_a_key_outside_the_allowlist_is_refused(self):
        history = a.History()
        run_id = add_night(history, 1)
        row = dict(history.fixed[0], reason="REASON-TEXT")
        with tempfile.TemporaryDirectory() as tmp:
            self.write(tmp, history)
            (Path(tmp) / "fixed.jsonl").write_text(json.dumps(row) + "\n", encoding="utf-8")
            with self.assertRaises(n.RowError):
                a.load_history(tmp)
            stderr = io.StringIO()
            with contextlib.redirect_stderr(stderr), contextlib.redirect_stdout(io.StringIO()):
                status = a.main(["--data-dir", tmp, "--run-id", run_id])
        self.assertEqual(status, 2)
        self.assertNotIn("REASON-TEXT", stderr.getvalue())

    def test_a_malformed_header_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "runs.jsonl"
            path.write_text(json.dumps({"schema": 2, "run_id": "bad"}) + "\n", encoding="utf-8")
            with self.assertRaises(n.RowError):
                a.read_headers(path)
            path.write_text("not json\n", encoding="utf-8")
            with self.assertRaises(n.RowError):
                a.read_headers(path)


class MainTests(unittest.TestCase):
    def run_main(self, args, env=None):
        out = io.StringIO()
        with mock.patch.dict(os.environ, env or {}, clear=False), contextlib.redirect_stdout(out), \
                contextlib.redirect_stderr(io.StringIO()):
            status = a.main(args)
        return status, out.getvalue()

    def setup(self, tmp, history):
        ReadingTests().write(tmp, history)

    def test_a_green_night_writes_the_summary_only_when_a_path_is_given(self):
        history = a.History()
        run_id = add_night(history, 1)
        with tempfile.TemporaryDirectory() as tmp:
            self.setup(tmp, history)
            summary = Path(tmp) / "summary.md"
            with mock.patch.dict(os.environ):
                os.environ.pop("GITHUB_STEP_SUMMARY", None)
                status, out = self.run_main(["--data-dir", tmp, "--run-id", run_id])
            self.assertEqual(status, 0)
            self.assertFalse(summary.exists())
            self.assertEqual(sorted(p.name for p in Path(tmp).iterdir()), ["fixed.jsonl", "fresh.jsonl", "runs.jsonl"])
            status, _ = self.run_main(["--data-dir", tmp, "--run-id", run_id, "--summary", str(summary)])
            self.assertEqual(status, 0)
            self.assertIn("no reference", summary.read_text(encoding="utf-8"))

    def test_the_summary_path_falls_back_to_the_environment(self):
        history = a.History()
        run_id = add_night(history, 1)
        with tempfile.TemporaryDirectory() as tmp:
            self.setup(tmp, history)
            summary = Path(tmp) / "step-summary.md"
            status, _ = self.run_main(["--data-dir", tmp, "--run-id", run_id],
                                      env={"GITHUB_STEP_SUMMARY": str(summary)})
            self.assertEqual(status, 0)
            self.assertIn("Nightly advisory", summary.read_text(encoding="utf-8"))

    def test_a_red_night_exits_non_zero_and_still_writes_its_line(self):
        history = a.History()
        add_night(history, 1)
        run_id = add_night(history, 2, fixed={"td1": {5: {"wrong": ("surname",)}}})
        with tempfile.TemporaryDirectory() as tmp:
            self.setup(tmp, history)
            out_file = Path(tmp) / "advisory.jsonl"
            status, out = self.run_main(["--data-dir", tmp, "--run-id", run_id, "--advisory-out", str(out_file)])
            self.assertEqual(status, 1)
            self.assertIn("::error::td1: fixed seed(s) newly a wrong accept", out)
            lines = out_file.read_text(encoding="utf-8").splitlines()
            self.assertEqual(len(lines), 1)
            self.assertEqual(json.loads(lines[0])["red_codes"], ["td1:new_wrong_accept"])

    def test_warnings_are_annotated_and_the_night_stays_green(self):
        history = a.History()
        add_night(history, 1)
        run_id = add_night(history, 2, fixed={"td1": {seed: {"hit": False} for seed in range(5)}})
        with tempfile.TemporaryDirectory() as tmp:
            self.setup(tmp, history)
            status, out = self.run_main(["--data-dir", tmp, "--run-id", run_id])
        self.assertEqual(status, 0)
        self.assertIn("::warning::td1: the fixed slice lost 5 more hits", out)

    def test_the_line_is_appended_once_per_run_and_output_is_deterministic(self):
        history = a.History()
        run_id = add_night(history, 1)
        with tempfile.TemporaryDirectory() as tmp:
            self.setup(tmp, history)
            out_file = Path(tmp) / "advisory.jsonl"
            out_file.write_text(json.dumps({"schema": 1, "run_id": "1-1"}) + "\n", encoding="utf-8")
            args = ["--data-dir", tmp, "--run-id", run_id, "--advisory-out", str(out_file)]
            self.run_main(args)
            first = out_file.read_text(encoding="utf-8")
            self.assertEqual(len(first.splitlines()), 2)
            status, out = self.run_main(args)
            self.assertEqual(status, 0)
            self.assertIn("already holds run", out)
            self.assertEqual(out_file.read_text(encoding="utf-8"), first)
            line = first.splitlines()[1]
            self.assertEqual(line, json.dumps(json.loads(line), sort_keys=True, separators=(",", ":")))

    def test_the_repo_flag_reaches_the_generator_check(self):
        history = a.History()
        add_night(history, 1)
        run_id = add_night(history, 2, salt="f")
        with tempfile.TemporaryDirectory() as tmp, tempfile.TemporaryDirectory() as repo:
            self.setup(tmp, history)
            status, out = self.run_main(["--data-dir", tmp, "--run-id", run_id, "--repo", repo])
        # An empty directory holds neither commit: undecided, so a warning and a green night.
        self.assertEqual(status, 0)
        self.assertIn("::warning::td1: the fixed slice renders differently", out)

    def test_a_bad_run_id_is_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            status, _ = self.run_main(["--data-dir", tmp, "--run-id", "nope"])
        self.assertEqual(status, 2)


class NoTextTests(unittest.TestCase):
    def test_nothing_the_advisory_prints_or_writes_can_hold_text(self):
        """The rows never hold text; assert the outputs hold none of the row keys' text either."""
        history = a.History()
        add_night(history, 1)
        run_id = add_night(history, 2, fixed={"td1": {5: {"wrong": ("surname",)}}})
        result = judge(history, run_id)
        blob = a.render_summary(result) + "\n".join(a.annotations(result)) + json.dumps(a.advisory_line(result))
        self.assertIsNone(re.search(r"surname|given_names|ocr|expected", blob))


if __name__ == "__main__":
    unittest.main()
