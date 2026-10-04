"""Tests for bench_nightly_intervals.py over a small constructed history: no network, no bench-data.

A night is built by `add_night` below: a fixed slice of seeds 0-99 whose render hashes carry the
night's fingerprint, and a fresh slice of rows described one by one, under a header that records the
measurement conditions.
"""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

import bench_nightly_advisory as advisory
import bench_nightly_intervals as m
import bench_nightly_rows as rows_lib

BASE_TIME = 1_790_000_000
MODEL = {"detection": "a" * 64, "recognition": "b" * 64}


def make_row(run_id, fmt, slice_name, seed, *, hit=True, wrong=(), retry_stop="general_valid",
             profile="clean", render=None, miss_kind="no_mrz_found", field_cer=True):
    """One validated schema-2 row. `wrong` names the scored fields with a non-zero CER. A non-hit
    keeps its field outcomes when `field_cer` is true and `miss_kind` is an accepted-read kind, as
    the generator's own report does for a `document_number_mismatch`."""
    cer = {name: 0.0 for name in rows_lib.FIELD_NAMES}
    for name in wrong:
        cer[name] = 0.5
    has_cer = field_cer and (hit or miss_kind == "document_number_mismatch")
    return rows_lib.validate_row(
        {
            "schema": 2,
            "run_id": run_id,
            "slice": slice_name,
            "document_type": fmt,
            "seed": seed,
            "profile": profile,
            "render_sha256": render or f"{seed:064x}",
            "hit": hit,
            "miss_kind": None if hit else miss_kind,
            "check_states": {"document_number": True, "date_of_birth": True, "date_of_expiry": True,
                             "personal_number": None, "composite": True} if hit else None,
            "field_cer": cer if has_cer else None,
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


def make_header(run_id, fmt, slice_name, sha, stamp, rows, *, model=MODEL, max_passes=14, max_seconds=52,
                arms=None):
    return {
        "schema": 2,
        "run_id": run_id,
        "slice": slice_name,
        "document_type": fmt,
        "git_sha": sha,
        "event": "schedule",
        "run_timestamp_unix": stamp,
        "generator_fingerprint": rows_lib.generator_fingerprint(rows) if slice_name == "fixed" else None,
        "ocr_arms": arms or {"texture": "on", "order": "default", "rotate": "default", "skew": "default",
                             "chargrid": "off"},
        "mrz_class_sweep_arm": "off",
        "mrz_line1_select_arm": "on",
        "ocr_env": {},
        "max_passes": max_passes,
        "max_seconds": max_seconds,
        "model_sha256": model,
        "counts": {"count": len(rows)},
    }


def add_night(history, number, *, fmt="td3", salt="", fresh=(), fixed_edit=None, **conditions):
    """Append one night of one format. `salt` re-renders the fixed slice (a new fingerprint);
    `fresh` is a list of `make_row` keyword dicts; `fixed_edit` maps a seed to keyword overrides."""
    run_id = f"{1000 + number}-1"
    stamp, sha = BASE_TIME + number * 86400, f"{number:040x}"
    fixed_rows = [
        make_row(run_id, fmt, "fixed", seed, render=f"{seed:032x}{salt:0<32}", **(fixed_edit or {}).get(seed, {}))
        for seed in range(100)
    ]
    fresh_rows = [make_row(run_id, fmt, "fresh", 1000 + number * 1000 + i, **spec) for i, spec in enumerate(fresh)]
    history.fixed.extend(fixed_rows)
    history.fresh.extend(fresh_rows)
    history.headers.append(make_header(run_id, fmt, "fresh", sha, stamp, fresh_rows, **conditions))
    history.headers.append(make_header(run_id, fmt, "fixed", sha, stamp, fixed_rows, **conditions))
    return run_id


# Ten fresh documents: 8 hits (3 wrong on surname, 1 of them also on given_names), 2 non-hits.
SPEC = (
    [dict(profile="clean", wrong=("surname",))] * 2
    + [dict(profile="mobile", wrong=("surname", "given_names"))]
    + [dict(profile="clean")] * 3
    + [dict(profile="mobile")] * 2
    + [dict(profile="mobile", hit=False)] * 2
)


def one_pool(history):
    pools, _ = m.build_pools(history)
    return pools


class WilsonTests(unittest.TestCase):
    def close(self, got, low, high):
        self.assertIsNotNone(got)
        self.assertAlmostEqual(got[0], low, delta=0.0005)
        self.assertAlmostEqual(got[1], high, delta=0.0005)

    def test_known_values(self):
        self.close(m.wilson_interval(19, 38), 0.3485, 0.6515)
        self.close(m.wilson_interval(0, 38), 0.0000, 0.0918)
        self.close(m.wilson_interval(38, 50), 0.6259, 0.8570)
        self.close(m.wilson_interval(38, 38), 0.9082, 1.0000)

    def test_no_interval_without_trials_or_with_more_successes_than_trials(self):
        self.assertIsNone(m.wilson_interval(0, 0))
        self.assertIsNone(m.wilson_interval(39, 38))
        self.assertIsNone(m.wilson_interval(1, 0))

    def test_the_ends_are_exact(self):
        for n in range(1, 61):
            self.assertEqual(m.wilson_interval(0, n)[0], 0.0, f"0/{n}")
            self.assertEqual(m.wilson_interval(n, n)[1], 1.0, f"{n}/{n}")

    def test_stays_inside_zero_and_one_and_brackets_the_rate(self):
        for n in range(1, 61):
            for k in range(n + 1):
                low, high = m.wilson_interval(k, n)
                self.assertTrue(0.0 <= low <= k / n <= high <= 1.0, f"{k}/{n}")

    def test_figure_without_a_denominator_has_no_rate_and_no_interval(self):
        empty = m.figure(0, 0)
        self.assertEqual((empty["rate"], empty["ci95"]), (None, None))
        self.assertEqual(m.figure(19, 38)["rate"], 0.5)


class ShareTests(unittest.TestCase):
    def rows(self):
        return [make_row("1-1", "td3", "fresh", i, **spec) for i, spec in enumerate(SPEC)]

    def test_wrong_shares_are_over_the_hits_and_a_hit_counts_in_each_wrong_field(self):
        s = m.shares(self.rows())
        self.assertEqual((s["documents"], s["hit_rate"]["k"], s["hit_rate"]["n"]), (10, 8, 10))
        self.assertEqual((s["wrong_on_any"]["k"], s["wrong_on_any"]["n"]), (3, 8))
        self.assertEqual(s["wrong_by_field"]["surname"]["k"], 3)
        self.assertEqual(s["wrong_by_field"]["given_names"]["k"], 1)
        self.assertEqual(s["wrong_by_field"]["document_number"]["k"], 0)
        self.assertEqual(s["wrong_by_field"]["surname"]["n"], 8)

    def test_a_non_hit_with_field_outcomes_counts_nowhere(self):
        # A document_number_mismatch is an accepted read, not a hit; its wrong fields are not among the hits'.
        rows = self.rows() + [make_row("1-1", "td3", "fresh", 99, hit=False, miss_kind="document_number_mismatch",
                                       wrong=("surname", "given_names"))]
        s = m.shares(rows)
        self.assertEqual(s["documents"], 11)
        self.assertEqual((s["wrong_on_any"]["k"], s["wrong_on_any"]["n"]), (3, 8))
        self.assertEqual(s["wrong_by_field"]["given_names"]["k"], 1)

    def test_hits_without_field_outcomes_are_printed_when_there_are_any(self):
        history = advisory.History()
        add_night(history, 1, fresh=list(SPEC) + [dict(field_cer=False)])
        text = m.render_markdown(m.build_report(history))
        self.assertIn("Hits with no field outcomes, left out of the wrong shares' denominators: td3 all 1", text)
        clean = advisory.History()
        add_night(clean, 1, fresh=SPEC)
        self.assertNotIn("Hits with no field outcomes", m.render_markdown(m.build_report(clean)))

    def test_a_hit_without_field_outcomes_is_counted_apart_and_left_out_of_the_denominator(self):
        rows = self.rows() + [make_row("1-1", "td3", "fresh", 99, field_cer=False)]
        s = m.shares(rows)
        self.assertEqual(s["hits_without_field_outcomes"], 1)
        self.assertEqual(s["hit_rate"]["k"], 9)
        self.assertEqual(s["wrong_on_any"]["n"], 8)


class PoolingTests(unittest.TestCase):
    def test_nights_with_one_fingerprint_and_one_set_of_conditions_share_a_pool(self):
        history = advisory.History()
        for number in (1, 2, 3):
            add_night(history, number, fresh=SPEC)
        (pool,) = one_pool(history)
        self.assertEqual(len(pool["nights"]), 3)
        fresh = m.fresh_figures(pool["nights"])
        self.assertEqual(fresh["all"]["documents"], 30)
        self.assertEqual(fresh["all"]["hit_rate"]["k"], 24)

    def test_a_fingerprint_change_splits_the_pool_and_is_never_added_across(self):
        history = advisory.History()
        add_night(history, 1, fresh=SPEC)
        add_night(history, 2, fresh=SPEC)
        add_night(history, 3, fresh=SPEC, salt="1")
        before, after = one_pool(history)
        self.assertEqual((len(before["nights"]), len(after["nights"])), (2, 1))
        self.assertNotEqual(before["fingerprint"], after["fingerprint"])
        self.assertEqual(m.fresh_figures(before["nights"])["all"]["documents"], 20)
        self.assertEqual(m.fresh_figures(after["nights"])["all"]["documents"], 10)

    def test_a_key_that_returns_starts_a_new_pool_and_no_range_encloses_another_pool(self):
        history = advisory.History()
        add_night(history, 1, fresh=SPEC)
        add_night(history, 2, fresh=SPEC)
        add_night(history, 3, fresh=SPEC, salt="1")
        add_night(history, 4, fresh=SPEC)  # the first key again
        first, middle, last = one_pool(history)
        self.assertEqual([len(p["nights"]) for p in (first, middle, last)], [2, 1, 1])
        self.assertEqual(first["fingerprint"], last["fingerprint"])
        self.assertEqual(m.fresh_figures(first["nights"])["all"]["documents"], 10 * 2)
        self.assertEqual(m.fresh_figures(last["nights"])["all"]["documents"], 10)
        report = m.build_report(history)
        self.assertEqual([p["group"] for p in report["pools"]], ["A", "B", "C"])

    def test_a_night_the_tool_cannot_read_ends_the_pool(self):
        history = advisory.History()
        add_night(history, 1, fresh=SPEC)
        add_night(history, 2, fresh=SPEC)
        add_night(history, 3, fresh=SPEC)
        history.headers = [h for h in history.headers if not (h["run_id"] == "1002-1" and h["slice"] == "fresh")]
        before, after = one_pool(history)
        self.assertEqual((len(before["nights"]), len(after["nights"])), (1, 1))

    def test_a_new_model_arm_or_budget_also_splits_the_pool(self):
        for change in (
            {"model": {"detection": "c" * 64, "recognition": "b" * 64}},
            {"max_passes": 20},
            {"max_seconds": 90},
            {"arms": {"texture": "off", "order": "default", "rotate": "default", "skew": "default",
                      "chargrid": "off"}},
        ):
            history = advisory.History()
            add_night(history, 1, fresh=SPEC)
            add_night(history, 2, fresh=SPEC, **change)
            self.assertEqual(len(one_pool(history)), 2, change)

    def test_formats_are_pooled_apart(self):
        history = advisory.History()
        for fmt in ("td1", "td3"):
            add_night(history, 1, fmt=fmt, fresh=SPEC)
        pools = one_pool(history)
        self.assertEqual([p["document_type"] for p in pools], ["td1", "td3"])

    def test_the_report_names_the_nights_and_the_git_sha_range(self):
        history = advisory.History()
        for number in (1, 2, 3):
            add_night(history, number, fresh=SPEC)
        add_night(history, 4, fresh=SPEC, salt="1")
        report = m.build_report(history)
        first, second = report["pools"]
        self.assertEqual((first["group"], second["group"]), ("A", "B"))
        self.assertEqual((first["git_sha_first"], first["git_sha_last"]), (f"{1:040x}", f"{3:040x}"))
        self.assertEqual(first["distinct_git_shas"], 3)
        self.assertEqual(len(first["nights"]), 3)
        self.assertEqual(second["git_sha_first"], second["git_sha_last"])

    def test_a_run_without_a_usable_fixed_slice_or_a_fresh_header_is_left_out_and_counted(self):
        history = advisory.History()
        add_night(history, 1, fresh=SPEC)
        add_night(history, 2, fresh=SPEC)
        history.headers = [h for h in history.headers if not (h["run_id"] == "1002-1" and h["slice"] == "fresh")]
        add_night(history, 3, fresh=SPEC)
        history.fixed = [r for r in history.fixed if not (r["run_id"] == "1003-1" and r["seed"] == 7)]
        report = m.build_report(history)
        (pool,) = report["pools"]
        self.assertEqual(len(pool["nights"]), 1)
        self.assertEqual(report["left_out"]["no_fresh_header"], 1)
        self.assertEqual(report["left_out"]["fixed_slice_unusable"], 1)
        self.assertEqual(report["left_out"]["fresh_rows_outside_a_pool"], 20)


class FixedNightTests(unittest.TestCase):
    def test_fixed_nights_are_never_pooled(self):
        history = advisory.History()
        for number in (1, 2, 3):
            add_night(history, number, fresh=SPEC, fixed_edit={5: {"wrong": ("surname",)}})
        (pool,) = one_pool(history)
        fixed = m.fixed_figures(pool["nights"])
        self.assertEqual(fixed["all"]["documents"], 100)  # one night, not 300
        self.assertEqual(fixed["all"]["hit_rate"]["n"], 100)
        self.assertEqual(fixed["all"]["wrong_on_any"]["k"], 1)
        self.assertEqual(fixed["nights"], 3)
        self.assertEqual(fixed["figures_from_run"], "1003-1")
        self.assertTrue(fixed["identical_across_nights"])

    def test_a_single_night_has_nothing_to_compare(self):
        history = advisory.History()
        add_night(history, 1, fresh=SPEC)
        (pool,) = one_pool(history)
        self.assertIsNone(m.fixed_figures(pool["nights"])["identical_across_nights"])
        self.assertIn("n/a, one night", m.render_markdown(m.build_report(history)))

    def test_a_seed_that_reads_differently_on_one_night_is_reported(self):
        history = advisory.History()
        add_night(history, 1, fresh=SPEC)
        add_night(history, 2, fresh=SPEC, fixed_edit={5: {"wrong": ("given_names",)}, 6: {"hit": False}})
        add_night(history, 3, fresh=SPEC)
        (pool,) = one_pool(history)
        fixed = m.fixed_figures(pool["nights"])
        self.assertFalse(fixed["identical_across_nights"])
        self.assertEqual(fixed["seeds_differing_most"], 2)
        self.assertEqual(fixed["all"]["hit_rate"]["k"], 100)  # the latest night, the clean one


class BudgetStopTests(unittest.TestCase):
    def test_budget_stops_are_left_out_of_every_rate_and_counted(self):
        spec = list(SPEC) + [dict(retry_stop="budget"), dict(retry_stop="budget", hit=False)]
        history = advisory.History()
        add_night(history, 1, fresh=spec, fixed_edit={3: {"retry_stop": "budget"}})
        (pool,) = one_pool(history)
        fresh = m.fresh_figures(pool["nights"])
        self.assertEqual(fresh["budget_stops"], 2)
        self.assertEqual(fresh["all"]["documents"], 10)
        self.assertEqual((fresh["all"]["hit_rate"]["k"], fresh["all"]["hit_rate"]["n"]), (8, 10))
        self.assertEqual(fresh["all"]["wrong_on_any"]["n"], 8)
        fixed = m.fixed_figures(pool["nights"])
        self.assertEqual((fixed["budget_stops_latest_night"], fixed["budget_stops_all_nights"]), (1, 1))
        self.assertEqual(fixed["all"]["documents"], 99)

    def test_budget_stops_are_printed_on_their_own_line(self):
        history = advisory.History()
        add_night(history, 1, fresh=list(SPEC) + [dict(retry_stop="budget")])
        text = m.render_markdown(m.build_report(history))
        self.assertIn("Budget stops left out of every rate: td3 1.", text)


class ProfileTests(unittest.TestCase):
    def test_the_profile_rows_add_up_to_the_pooled_row(self):
        spec = [dict(profile=p, hit=i % 4 != 0, wrong=("surname",) if i % 3 == 0 else (),
                     **({"miss_kind": "no_mrz_found"} if i % 4 == 0 else {}))
                for i, p in enumerate(["clean", "mobile", "scanner", "worn", "border-kiosk"] * 12)]
        history = advisory.History()
        add_night(history, 1, fresh=spec)
        add_night(history, 2, fresh=spec)
        (pool,) = one_pool(history)
        fresh = m.fresh_figures(pool["nights"])
        total, parts = fresh["all"], list(fresh["by_profile"].values())
        self.assertEqual(len(parts), 5)
        self.assertEqual(sum(p["documents"] for p in parts), total["documents"])
        self.assertEqual(sum(p["hit_rate"]["k"] for p in parts), total["hit_rate"]["k"])
        self.assertEqual(sum(p["wrong_on_any"]["k"] for p in parts), total["wrong_on_any"]["k"])
        self.assertEqual(sum(p["wrong_on_any"]["n"] for p in parts), total["wrong_on_any"]["n"])
        for name in rows_lib.SCORED_FIELDS:
            self.assertEqual(
                sum(p["wrong_by_field"][name]["k"] for p in parts), total["wrong_by_field"][name]["k"], name
            )


class CommandLineTests(unittest.TestCase):
    def write(self, directory, history):
        def jsonl(rows):
            return "".join(json.dumps(row) + "\n" for row in rows)

        (directory / "fresh.jsonl").write_text(jsonl(history.fresh), encoding="utf-8")
        (directory / "fixed.jsonl").write_text(jsonl(history.fixed), encoding="utf-8")
        (directory / "runs.jsonl").write_text(jsonl(history.headers), encoding="utf-8")

    def run_main(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
            code = m.main(list(argv))
        return code, out.getvalue(), err.getvalue()

    def test_markdown_and_json_carry_aggregates_only(self):
        history = advisory.History()
        add_night(history, 1, fresh=SPEC)
        add_night(history, 2, fresh=SPEC, salt="1")
        with tempfile.TemporaryDirectory() as tmp:
            self.write(Path(tmp), history)
            code, text, _ = self.run_main(tmp)
            self.assertEqual(code, 0)
            self.assertIn("## Pool A", text)
            self.assertIn("## Pool B", text)
            self.assertIn("8/10 80.0%", text)
            code, raw, _ = self.run_main(tmp, "--json")
            self.assertEqual(code, 0)
            report = json.loads(raw)
            self.assertEqual(len(report["pools"]), 2)
            for output in (text, raw):
                for key in ("render_sha256", '"seed"', "miss_kind", "field_cer", "elapsed_ms"):
                    self.assertNotIn(key, output)

    def test_a_missing_directory_is_a_usage_error(self):
        code, out, err = self.run_main("/no/such/directory/anywhere")
        self.assertEqual((code, out), (2, ""))
        self.assertIn("not a directory", err)

    def test_a_row_with_a_key_outside_the_allowlist_is_refused(self):
        history = advisory.History()
        add_night(history, 1, fresh=SPEC)
        with tempfile.TemporaryDirectory() as tmp:
            self.write(Path(tmp), history)
            path = Path(tmp) / "fresh.jsonl"
            first, *rest = path.read_text(encoding="utf-8").splitlines()
            path.write_text("\n".join([json.dumps({**json.loads(first), "reason": "text"}), *rest]) + "\n", encoding="utf-8")
            code, out, err = self.run_main(tmp)
            self.assertEqual((code, out), (2, ""))
            self.assertIn("outside the allowlist", err)


if __name__ == "__main__":
    unittest.main()
