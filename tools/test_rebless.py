#!/usr/bin/env python3
"""
Offline unit tests for tools/rebless.py. No network, no git, no cargo --
every test exercises pure functions against fixtures or injected fakes,
matching tools/test_apply_cohort.py's pattern.

Run with:
    python -m unittest tools/test_rebless.py
"""

from __future__ import annotations

import copy
import os
import sys
import tempfile
import unittest
from unittest import mock
from datetime import datetime, timezone
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import rebless as rb  # noqa: E402


# --------------------------------------------------------------------------
# Baseline fixtures. Shaped like the real committed
# knowledge/benchmarks/real-specimen-mrz-baseline.json (a zero-count miss
# kind is simply absent from `by_miss_kind`, matching the Rust
# `.entry(kind).or_default()` construction -- see ALL_MISS_KIND_KEYS).
# --------------------------------------------------------------------------


def make_baseline(**overrides) -> dict:
    base = {
        "note": "Real-specimen Tier-1 no-regression baseline ...",
        "measured_on_ci_sha": "aaaaaaa",
        "measured_date": "2026-09-14",
        "samples_data_sha": "bbbbbbb",
        "documents": 265,
        "scored": 159,
        "tier1_hits": 145,
        "tolerance": 0,
        "by_miss_kind": {
            "checksum_failed": 10,
            "checksum_failed_specimen": 19,
            "no_mrz_expected": 50,
            "no_mrz_found": 4,
            "redacted_mrz": 37,
        },
    }
    base.update(overrides)
    return base


class FlattenBaselineTests(unittest.TestCase):
    def test_folds_by_miss_kind_into_top_level(self):
        flat = rb.flatten_baseline(make_baseline())
        self.assertEqual(flat["checksum_failed"], 10)
        self.assertEqual(flat["documents"], 265)
        self.assertNotIn("by_miss_kind", flat)
        self.assertNotIn("note", flat)

    def test_absent_miss_kind_defaults_to_zero(self):
        # false_positive_mrz never appears in the committed baseline because
        # its count is zero -- the Rust BTreeMap only gets an entry on an
        # actual miss (`.entry(kind).or_default()`), not a placeholder 0.
        flat = rb.flatten_baseline(make_baseline())
        self.assertEqual(flat["false_positive_mrz"], 0)
        self.assertEqual(flat["ocr_error"], 0)
        self.assertEqual(flat["document_number_mismatch"], 0)


class ClassifyBaselineDiffTests(unittest.TestCase):
    def test_only_provenance_moved_is_identical(self):
        old = make_baseline()
        new = make_baseline(measured_on_ci_sha="ccccccc", measured_date="2026-09-15", samples_data_sha="ddddddd")
        cls, changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_IDENTICAL)
        changed_keys = {c[0] for c in changed}
        self.assertEqual(changed_keys, {"measured_on_ci_sha", "measured_date", "samples_data_sha"})

    def test_byte_identical_baselines_are_identical(self):
        old = make_baseline()
        new = copy.deepcopy(old)
        cls, changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_IDENTICAL)
        self.assertEqual(changed, [])

    def test_non_scored_bucket_move_is_non_scored_delta(self):
        # The real cohort c10/c12 shape: documents and no_mrz_expected move by
        # one, everything scored holds.
        old = make_baseline()
        new = make_baseline(
            documents=266,
            measured_date="2026-09-15",
            by_miss_kind={**old["by_miss_kind"], "no_mrz_expected": 51},
        )
        cls, changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_NON_SCORED)
        changed_keys = {c[0] for c in changed}
        self.assertIn("documents", changed_keys)
        self.assertIn("no_mrz_expected", changed_keys)
        self.assertNotIn("scored", changed_keys)
        self.assertNotIn("tier1_hits", changed_keys)

    def test_tier1_hits_move_is_scored_delta(self):
        old = make_baseline()
        new = make_baseline(tier1_hits=146, scored=160, documents=266)
        cls, _changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_SCORED)

    def test_checksum_failed_move_is_scored_delta(self):
        old = make_baseline()
        new = make_baseline(
            documents=266,
            scored=160,
            by_miss_kind={**old["by_miss_kind"], "checksum_failed": 13},
        )
        cls, _changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_SCORED)

    def test_false_positive_appearing_is_scored_delta(self):
        old = make_baseline()
        new = make_baseline(by_miss_kind={**old["by_miss_kind"], "false_positive_mrz": 1})
        cls, _changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_SCORED)

    def test_unrecognized_field_move_is_scored_delta_not_guessed(self):
        old = make_baseline()
        new = make_baseline(tolerance=1)
        cls, _changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_SCORED)

    def test_changed_list_reports_old_and_new_values(self):
        old = make_baseline()
        new = make_baseline(documents=266, by_miss_kind={**old["by_miss_kind"], "no_mrz_expected": 51})
        _cls, changed = rb.classify_baseline_diff(old, new)
        by_key = {c[0]: (c[1], c[2]) for c in changed}
        self.assertEqual(by_key["documents"], (265, 266))
        self.assertEqual(by_key["no_mrz_expected"], (50, 51))


class FormatDiffTableTests(unittest.TestCase):
    def test_excludes_provenance_rows_but_states_measured_line(self):
        old = make_baseline()
        new = make_baseline(documents=266, measured_date="2026-09-15", measured_on_ci_sha="ccccccc")
        _cls, changed = rb.classify_baseline_diff(old, new)
        table = rb.format_diff_table(changed, rb.flatten_baseline(new))
        self.assertIn("documents", table)
        self.assertNotIn("measured_on_ci_sha ", table)  # not a row
        self.assertIn("(baseline measured 2026-09-15 on ccccccc)", table)

    def test_row_shows_signed_delta(self):
        old = make_baseline()
        new = make_baseline(documents=266)
        _cls, changed = rb.classify_baseline_diff(old, new)
        table = rb.format_diff_table(changed, rb.flatten_baseline(new))
        self.assertIn("265 -> 266", table)
        self.assertIn("(+1)", table)

    def test_no_field_moved_beyond_provenance_says_so(self):
        old = make_baseline()
        new = make_baseline(measured_date="2026-09-15")
        _cls, changed = rb.classify_baseline_diff(old, new)
        table = rb.format_diff_table(changed, rb.flatten_baseline(new))
        self.assertIn("no field moved beyond CI provenance", table)


class FormatRateTests(unittest.TestCase):
    def test_matches_the_committed_baseline_headline(self):
        self.assertEqual(rb.format_rate(145, 159), "91.2")
        self.assertEqual(rb.format_rate(145, 266), "54.5")

    def test_zero_denominator_raises(self):
        with self.assertRaises(ValueError):
            rb.format_rate(0, 0)


# --------------------------------------------------------------------------
# Templated prose
# --------------------------------------------------------------------------


class BuildWeakspotEntryTests(unittest.TestCase):
    def setUp(self):
        self.old = make_baseline()
        self.new = make_baseline(
            documents=266,
            measured_date="2026-09-15",
            measured_on_ci_sha="ccccccc",
            by_miss_kind={**self.old["by_miss_kind"], "no_mrz_expected": 51},
        )
        self.old_flat = rb.flatten_baseline(self.old)
        self.new_flat = rb.flatten_baseline(self.new)
        _cls, self.changed = rb.classify_baseline_diff(self.old, self.new)

    def test_heading_shape(self):
        entry = rb.build_weakspot_entry("cohort-c14", 1, self.changed, "34980671424", self.old_flat, self.new_flat)
        self.assertIn("### 2026-09-15 — cohort c14: 1 specimen(s),", entry)

    def test_names_the_run_id(self):
        entry = rb.build_weakspot_entry("cohort-c14", 1, self.changed, "34980671424", self.old_flat, self.new_flat)
        self.assertIn("34980671424", entry)

    def test_states_scored_rate_unchanged(self):
        entry = rb.build_weakspot_entry("cohort-c14", 1, self.changed, "34980671424", self.old_flat, self.new_flat)
        self.assertIn("The scored rate did not move", entry)
        self.assertIn("145 / 159 = 91.2%", entry)

    def test_never_invents_prose_beyond_the_numbers(self):
        # No adjectives, no causal claim -- just the delta table and the two
        # fixed sentences. A loose proxy: the entry contains only the
        # cohort_branch, run id, dates and the changed field names/numbers,
        # nothing else that looks like free-form explanation.
        entry = rb.build_weakspot_entry("cohort-c14", 1, self.changed, "34980671424", self.old_flat, self.new_flat)
        self.assertNotIn("because", entry)
        self.assertNotIn("<<<", entry)  # not a DRAFT placeholder -- this is real prose


# --------------------------------------------------------------------------
# README.md / knowledge/benchmarks/README.md fixture-snippet rewrites
# --------------------------------------------------------------------------

README_SNIPPET = """## Accuracy

- **145 / 159 = 91.2% on documents that can yield a hit** -- how often extraction succeeds when
  success is possible.
- **145 / 265 = 54.7% across the whole specimen corpus** -- what happens if you point it at a pile
  of real documents. The 106-specimen gap is not failure: those carry no machine-readable zone at
  all.
"""

BENCH_README_SNIPPET = """## Current headline numbers

| Metric | Value | Source |
| --- | --- | --- |
| **Tier-1 hit rate, real specimens** | **145 / 159 = 91.2%** on documents that can yield a hit | `real-specimen-mrz-baseline.json` (CI, 2026-09-14) |
| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |

**Why two rates.** 106 of the 265 specimens cannot produce a Tier-1 hit under any pipeline, so
counting them as failures measures the corpus rather than the reader.

**Real-specimen outcomes** (265 documents):

| Outcome | Count | In the denominator? | Meaning |
| --- | --- | --- | --- |
| **Tier-1 HIT** | **145** | numerator | Checksum-valid MRZ |
| `no_mrz_found` | 4 | yes | No MRZ located |
| `checksum_failed` | **10** | yes | A genuine OCR error |
| `false_positive_mrz` | 0 | yes | Any non-zero value here fails the build |
| `no_mrz_expected` | 50 | no | A correct refusal |
| `redacted_mrz` | 37 | no | Zone blacked out |
| `checksum_failed_specimen` | 19 | no | Printed zone fails its own check digits |
"""


class RewriteReadmeGapAndCorpusRateTests(unittest.TestCase):
    def setUp(self):
        self.old = make_baseline()
        self.new = make_baseline(
            documents=266,
            measured_date="2026-09-15",
            by_miss_kind={**self.old["by_miss_kind"], "no_mrz_expected": 51},
        )
        self.old_flat = rb.flatten_baseline(self.old)
        self.new_flat = rb.flatten_baseline(self.new)

    def test_updates_corpus_rate_and_gap(self):
        out = rb.rewrite_readme_gap_and_corpus_rate(README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("145 / 266 = 54.5%", out)
        self.assertIn("107-specimen gap", out)
        self.assertNotIn("145 / 265 = 54.7%", out)
        self.assertNotIn("106-specimen gap", out)

    def test_scored_rate_bullet_is_untouched(self):
        out = rb.rewrite_readme_gap_and_corpus_rate(README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("145 / 159 = 91.2%", out)

    def test_missing_pattern_raises_rather_than_silently_skipping(self):
        with self.assertRaises(ValueError):
            rb.rewrite_readme_gap_and_corpus_rate("no numbers here at all", 145, self.old_flat, self.new_flat)


class RewriteBenchmarksReadmeLiveBlockTests(unittest.TestCase):
    def setUp(self):
        self.old = make_baseline()
        self.new = make_baseline(
            documents=266,
            measured_date="2026-09-15",
            measured_on_ci_sha="ccccccc",
            by_miss_kind={**self.old["by_miss_kind"], "no_mrz_expected": 51},
        )
        self.old_flat = rb.flatten_baseline(self.old)
        self.new_flat = rb.flatten_baseline(self.new)

    def test_updates_corpus_rate_row(self):
        out = rb.rewrite_benchmarks_readme_live_block(BENCH_README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("145 / 266 = 54.5%", out)

    def test_updates_outcomes_heading_document_count(self):
        out = rb.rewrite_benchmarks_readme_live_block(BENCH_README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("(266 documents)", out)
        self.assertNotIn("(265 documents)", out)

    def test_updates_moved_bucket_count_preserving_no_bold(self):
        out = rb.rewrite_benchmarks_readme_live_block(BENCH_README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("`no_mrz_expected` | 51 |", out)
        self.assertNotIn("`no_mrz_expected` | 50 |", out)

    def test_unmoved_bold_bucket_count_is_untouched(self):
        out = rb.rewrite_benchmarks_readme_live_block(BENCH_README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("`checksum_failed` | **10** |", out)

    def test_updates_why_two_rates_sentence(self):
        out = rb.rewrite_benchmarks_readme_live_block(BENCH_README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("107 of the 266 specimens cannot produce", out)

    def test_updates_measured_date_source_cell(self):
        out = rb.rewrite_benchmarks_readme_live_block(BENCH_README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("(CI, 2026-09-15)", out)
        self.assertNotIn("(CI, 2026-09-14)", out)

    def test_scored_rate_row_numbers_are_untouched(self):
        out = rb.rewrite_benchmarks_readme_live_block(BENCH_README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertIn("**145 / 159 = 91.2%**", out)

    def test_missing_pattern_raises(self):
        with self.assertRaises(ValueError):
            rb.rewrite_benchmarks_readme_live_block("nothing to match here", 145, self.old_flat, self.new_flat)


# --------------------------------------------------------------------------
# select_dispatch_run -- distinguishing a fresh workflow_dispatch run from a
# push/pull_request run of the same workflow.
# --------------------------------------------------------------------------


class SelectDispatchRunTests(unittest.TestCase):
    def test_picks_the_workflow_dispatch_run_after_the_dispatch_time(self):
        dispatched_at = datetime(2026, 9, 15, 12, 0, 0, tzinfo=timezone.utc)
        runs = [
            {"databaseId": 1, "event": "push", "createdAt": "2026-09-15T11:59:00Z"},
            {"databaseId": 2, "event": "workflow_dispatch", "createdAt": "2026-09-15T12:00:05Z"},
            {"databaseId": 3, "event": "pull_request", "createdAt": "2026-09-15T12:00:10Z"},
        ]
        run = rb.select_dispatch_run(runs, dispatched_at)
        self.assertEqual(run["databaseId"], 2)

    def test_ignores_a_workflow_dispatch_run_created_before_the_dispatch(self):
        dispatched_at = datetime(2026, 9, 15, 12, 0, 0, tzinfo=timezone.utc)
        runs = [
            {"databaseId": 1, "event": "workflow_dispatch", "createdAt": "2026-09-14T09:00:00Z"},
            {"databaseId": 2, "event": "workflow_dispatch", "createdAt": "2026-09-15T12:00:05Z"},
        ]
        run = rb.select_dispatch_run(runs, dispatched_at)
        self.assertEqual(run["databaseId"], 2)

    def test_prefers_the_earliest_matching_run_when_several_qualify(self):
        dispatched_at = datetime(2026, 9, 15, 12, 0, 0, tzinfo=timezone.utc)
        runs = [
            {"databaseId": 2, "event": "workflow_dispatch", "createdAt": "2026-09-15T12:05:00Z"},
            {"databaseId": 1, "event": "workflow_dispatch", "createdAt": "2026-09-15T12:00:05Z"},
        ]
        run = rb.select_dispatch_run(runs, dispatched_at)
        self.assertEqual(run["databaseId"], 1)

    def test_raises_when_nothing_matches(self):
        dispatched_at = datetime(2026, 9, 15, 12, 0, 0, tzinfo=timezone.utc)
        runs = [{"databaseId": 1, "event": "push", "createdAt": "2026-09-15T12:00:05Z"}]
        with self.assertRaises(ValueError):
            rb.select_dispatch_run(runs, dispatched_at)

    def test_raises_on_empty_run_list(self):
        with self.assertRaises(ValueError):
            rb.select_dispatch_run([], datetime.now(timezone.utc))


# --------------------------------------------------------------------------
# verify_worktree_on_branch
# --------------------------------------------------------------------------


class VerifyWorktreeOnBranchTests(unittest.TestCase):
    def test_missing_worktree_raises(self):
        with tempfile.TemporaryDirectory() as td:
            missing = Path(td) / "does-not-exist"
            with self.assertRaises(rb.WorktreeBranchMismatch) as ctx:
                rb.verify_worktree_on_branch(missing, "cohort-c14", current_branch_fn=lambda w: "cohort-c14")
            self.assertIn("does not exist", str(ctx.exception))

    def test_wrong_branch_raises(self):
        with tempfile.TemporaryDirectory() as td:
            worktree = Path(td)
            with self.assertRaises(rb.WorktreeBranchMismatch) as ctx:
                rb.verify_worktree_on_branch(worktree, "cohort-c14", current_branch_fn=lambda w: "main")
            self.assertIn("expected 'cohort-c14'", str(ctx.exception))

    def test_matching_branch_does_not_raise(self):
        with tempfile.TemporaryDirectory() as td:
            worktree = Path(td)
            rb.verify_worktree_on_branch(worktree, "cohort-c14", current_branch_fn=lambda w: "cohort-c14")  # no raise

    def test_detached_or_empty_current_branch_raises(self):
        # `git branch --show-current` prints nothing when detached -- must
        # never be silently treated as a match.
        with tempfile.TemporaryDirectory() as td:
            worktree = Path(td)
            with self.assertRaises(rb.WorktreeBranchMismatch):
                rb.verify_worktree_on_branch(worktree, "cohort-c14", current_branch_fn=lambda w: "")


class DispatchCommandTests(unittest.TestCase):
    def test_write_baseline_includes_exact_data_ref(self):
        command = rb.build_workflow_dispatch_command("cohort-c14", "write-baseline", "a" * 40)
        self.assertEqual(command[-2:], ["-f", "data_ref=" + "a" * 40])

    def test_assert_rejects_data_ref(self):
        with self.assertRaises(ValueError):
            rb.build_workflow_dispatch_command("cohort-c14", "assert", "a" * 40)

    def test_resolves_single_samples_data_tip(self):
        with mock.patch.object(rb.ac, "run_cmd", return_value=("b" * 40) + "  refs/heads/samples-data\n") as run:
            self.assertEqual(rb.resolve_samples_data_ref(Path(".")), "b" * 40)
        run.assert_called_once_with(["git", "ls-remote", "origin", "refs/heads/samples-data"], cwd=Path("."))

    def test_rejects_ambiguous_samples_data_tip(self):
        with mock.patch.object(rb.ac, "run_cmd", return_value="not-a-sha\n"):
            with self.assertRaises(ValueError):
                rb.resolve_samples_data_ref(Path("."))


class WatchCommandTests(unittest.TestCase):
    """C2: one cohort's wait wrote 33,000 lines into the log. The wait is still
    one blocking call -- it just redraws once a minute and says two lines."""

    def test_interval_and_exit_status(self):
        cmd = rb.build_watch_command("34980671424")
        self.assertEqual(cmd[:4], ["gh", "run", "watch", "34980671424"])
        self.assertIn("--interval", cmd)
        self.assertEqual(cmd[cmd.index("--interval") + 1], "60")
        self.assertIn("--exit-status", cmd)


if __name__ == "__main__":
    unittest.main()
