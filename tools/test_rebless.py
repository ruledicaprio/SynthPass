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
import json
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

    def test_strict_names_fold_into_top_level(self):
        flat = rb.flatten_baseline(
            make_baseline(strict_names={"strict_hits": 40, "name_scorable_documents": 60, "name_scorable_hits": 50})
        )
        self.assertEqual(flat["strict_hits"], 40)
        self.assertEqual(flat["name_scorable_documents"], 60)
        self.assertEqual(flat["name_scorable_hits"], 50)
        self.assertNotIn("strict_names", flat)

    def test_absent_strict_names_is_not_defaulted_to_zero(self):
        # Unlike a miss kind, "never measured" and "measured as zero" are
        # different facts for strict_names -- the committed baseline has no
        # such key today, and flatten_baseline must leave it absent, not
        # invent a 0 the live-block rewrite would then mistake for a
        # measurement.
        flat = rb.flatten_baseline(make_baseline())
        self.assertNotIn("strict_hits", flat)
        self.assertNotIn("name_scorable_documents", flat)
        self.assertNotIn("name_scorable_hits", flat)


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

    def test_strict_names_move_alone_is_non_scored_delta(self):
        # ADR-0013's report-only counts moving, with nothing scored touched,
        # must classify the same way an off-denominator bucket does.
        old = make_baseline()
        new = make_baseline(
            strict_names={"strict_hits": 40, "name_scorable_documents": 60, "name_scorable_hits": 50}
        )
        cls, changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_NON_SCORED)
        changed_keys = {c[0] for c in changed}
        self.assertIn("strict_hits", changed_keys)
        self.assertIn("name_scorable_documents", changed_keys)
        self.assertIn("name_scorable_hits", changed_keys)

    def test_strict_names_appearing_from_absent_is_non_scored_delta(self):
        old = make_baseline()
        new = make_baseline(
            strict_names={"strict_hits": 1, "name_scorable_documents": 1, "name_scorable_hits": 1}
        )
        cls, _changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_NON_SCORED)

    def test_strict_names_move_alongside_a_scored_move_is_still_scored_delta(self):
        old = make_baseline()
        new = make_baseline(
            tier1_hits=146,
            scored=160,
            documents=266,
            strict_names={"strict_hits": 40, "name_scorable_documents": 60, "name_scorable_hits": 50},
        )
        cls, _changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_SCORED)

    def test_changed_list_reports_old_and_new_values(self):
        old = make_baseline()
        new = make_baseline(documents=266, by_miss_kind={**old["by_miss_kind"], "no_mrz_expected": 51})
        _cls, changed = rb.classify_baseline_diff(old, new)
        by_key = {c[0]: (c[1], c[2]) for c in changed}
        self.assertEqual(by_key["documents"], (265, 266))
        self.assertEqual(by_key["no_mrz_expected"], (50, 51))

    def test_outcomes_sha256_alone_moving_is_identical(self):
        # The ledger's per-document `ocr_ms` is wall-clock, so the hash moves
        # on almost every CI run -- it must classify exactly like the other
        # provenance fields, never as a scored or non-scored delta on its own.
        old = make_baseline(outcomes_sha256="a" * 64)
        new = make_baseline(outcomes_sha256="b" * 64)
        cls, changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_IDENTICAL)
        self.assertIn("outcomes_sha256", {c[0] for c in changed})

    def test_refusal_population_alone_moving_is_non_scored_delta(self):
        old = make_baseline(documents=265, refusal_population=106)
        new = make_baseline(documents=266, refusal_population=107, measured_date="2026-09-15")
        cls, changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_NON_SCORED)
        self.assertIn("refusal_population", {c[0] for c in changed})

    def test_refusal_population_move_alongside_a_scored_move_is_still_scored_delta(self):
        old = make_baseline(refusal_population=106)
        new = make_baseline(tier1_hits=146, scored=160, documents=266, refusal_population=106)
        cls, _changed = rb.classify_baseline_diff(old, new)
        self.assertEqual(cls, rb.CLASS_SCORED)


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

    def test_report_only_row_is_labelled(self):
        old = make_baseline()
        new = make_baseline(
            documents=266,
            strict_names={"strict_hits": 40, "name_scorable_documents": 60, "name_scorable_hits": 50},
        )
        _cls, changed = rb.classify_baseline_diff(old, new)
        table = rb.format_diff_table(changed, rb.flatten_baseline(new))
        self.assertIn("(report-only)", table)
        # A non-report-only row moving in the same diff must not carry the marker.
        for line in table.splitlines():
            if line.strip().startswith("documents"):
                self.assertNotIn("(report-only)", line)


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

# Same fixture, with the ADR-0013 Strict name hit rate row present as it ships
# in the real file -- placeholder text until the first baseline carries
# `strict_names`.
BENCH_README_SNIPPET_WITH_STRICT_ROW = BENCH_README_SNIPPET.replace(
    "| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |\n",
    "| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |\n"
    f"| Strict name hit rate, real specimens | {rb.STRICT_NAME_PLACEHOLDER_VALUE} | {rb.STRICT_NAME_PLACEHOLDER_SOURCE} |\n",
)

# Same fixture, with the False accepts row present as it ships in the real
# file -- placeholder text until the first baseline carries
# `refusal_population`.
BENCH_README_SNIPPET_WITH_FALSE_ACCEPTS_ROW = BENCH_README_SNIPPET.replace(
    "| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |\n",
    "| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |\n"
    f"| False accepts (a checksum-valid MRZ returned for a document that carries none) | {rb.FALSE_ACCEPTS_PLACEHOLDER_VALUE} | {rb.FALSE_ACCEPTS_PLACEHOLDER_SOURCE} |\n",
)


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

    def test_strict_row_untouched_when_no_strict_row_present_and_no_strict_names(self):
        # The committed baseline has no strict_names key today, so the
        # default-shaped snippet round-trips with no strict row at all --
        # confirms the integration is a true no-op, not just "doesn't crash".
        out = rb.rewrite_benchmarks_readme_live_block(BENCH_README_SNIPPET, 145, self.old_flat, self.new_flat)
        self.assertNotIn("Strict name hit rate", out)

    def test_leaves_strict_placeholder_row_untouched_when_new_baseline_has_no_strict_names(self):
        out = rb.rewrite_benchmarks_readme_live_block(
            BENCH_README_SNIPPET_WITH_STRICT_ROW, 145, self.old_flat, self.new_flat
        )
        self.assertIn(rb.STRICT_NAME_PLACEHOLDER_VALUE, out)

    def test_rewrites_strict_placeholder_row_when_new_baseline_carries_strict_names(self):
        new = make_baseline(
            documents=266,
            measured_date="2026-09-15",
            measured_on_ci_sha="ccccccc",
            by_miss_kind={**self.old["by_miss_kind"], "no_mrz_expected": 51},
            strict_names={"strict_hits": 40, "name_scorable_documents": 60, "name_scorable_hits": 50},
        )
        new_flat = rb.flatten_baseline(new)
        out = rb.rewrite_benchmarks_readme_live_block(
            BENCH_README_SNIPPET_WITH_STRICT_ROW, 145, self.old_flat, new_flat
        )
        self.assertIn("**40 / 60 = 66.7%**", out)
        self.assertIn("(CI, 2026-09-15); ADR-0013", out)
        self.assertNotIn(rb.STRICT_NAME_PLACEHOLDER_VALUE, out)


class RewriteBenchmarksReadmeStrictNameRowTests(unittest.TestCase):
    """Unit-level coverage of `rewrite_benchmarks_readme_strict_name_row`
    itself, independent of the rest of the live-block rewrite -- the
    end-to-end path is also covered above, via
    `RewriteBenchmarksReadmeLiveBlockTests`."""

    def test_rewrites_from_the_placeholder(self):
        old_flat = rb.flatten_baseline(make_baseline())
        new_flat = rb.flatten_baseline(
            make_baseline(
                strict_names={"strict_hits": 40, "name_scorable_documents": 60, "name_scorable_hits": 50},
                measured_date="2026-09-15",
            )
        )
        out = rb.rewrite_benchmarks_readme_strict_name_row(
            BENCH_README_SNIPPET_WITH_STRICT_ROW, old_flat, new_flat
        )
        self.assertIn("**40 / 60 = 66.7%**", out)
        self.assertIn("80.0% of name-scorable hits", out)  # 40 / 50
        self.assertIn("same baseline (CI, 2026-09-15); ADR-0013", out)
        self.assertNotIn("not yet measured in CI", out)

    def test_rewrites_from_a_previous_number(self):
        old_flat = rb.flatten_baseline(
            make_baseline(
                strict_names={"strict_hits": 30, "name_scorable_documents": 50, "name_scorable_hits": 45},
                measured_date="2026-09-14",
            )
        )
        old_value, old_source = rb._format_strict_name_row_cells(old_flat)
        snippet = BENCH_README_SNIPPET.replace(
            "| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |\n",
            "| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |\n"
            f"| Strict name hit rate, real specimens | {old_value} | {old_source} |\n",
        )
        new_flat = rb.flatten_baseline(
            make_baseline(
                strict_names={"strict_hits": 32, "name_scorable_documents": 52, "name_scorable_hits": 46},
                measured_date="2026-09-16",
            )
        )
        out = rb.rewrite_benchmarks_readme_strict_name_row(snippet, old_flat, new_flat)
        self.assertIn("**32 / 52 = 61.5%**", out)
        self.assertNotIn("30 / 50 = 60.0%", out)

    def test_no_op_when_new_baseline_has_no_strict_names(self):
        old_flat = rb.flatten_baseline(make_baseline())
        new_flat = rb.flatten_baseline(make_baseline())
        out = rb.rewrite_benchmarks_readme_strict_name_row(
            BENCH_README_SNIPPET_WITH_STRICT_ROW, old_flat, new_flat
        )
        self.assertEqual(out, BENCH_README_SNIPPET_WITH_STRICT_ROW)

    def test_missing_row_raises_rather_than_silently_skipping(self):
        old_flat = rb.flatten_baseline(make_baseline())
        new_flat = rb.flatten_baseline(
            make_baseline(strict_names={"strict_hits": 1, "name_scorable_documents": 1, "name_scorable_hits": 1})
        )
        with self.assertRaises(ValueError):
            rb.rewrite_benchmarks_readme_strict_name_row("no strict row in this text at all", old_flat, new_flat)


class RewriteBenchmarksReadmeFalseAcceptsRowTests(unittest.TestCase):
    """Unit-level coverage of `rewrite_benchmarks_readme_false_accepts_row`,
    mirroring `RewriteBenchmarksReadmeStrictNameRowTests` for the other
    report-only row this PR adds."""

    def test_rewrites_from_the_placeholder(self):
        old_flat = rb.flatten_baseline(make_baseline())
        new_flat = rb.flatten_baseline(
            make_baseline(refusal_population=106, measured_date="2026-09-15")
        )
        out = rb.rewrite_benchmarks_readme_false_accepts_row(
            BENCH_README_SNIPPET_WITH_FALSE_ACCEPTS_ROW, old_flat, new_flat
        )
        self.assertIn("**0 / 106**", out)
        self.assertIn("same baseline (CI, 2026-09-15)", out)
        self.assertNotIn("not yet in the baseline", out)

    def test_rewrites_from_a_previous_number(self):
        old_flat = rb.flatten_baseline(make_baseline(refusal_population=100, measured_date="2026-09-14"))
        old_value, old_source = rb._format_false_accepts_row_cells(old_flat)
        snippet = BENCH_README_SNIPPET.replace(
            "| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |\n",
            "| Tier-1 hit rate, whole specimen corpus | 145 / 265 = 54.7% | same baseline; the gap is explained below |\n"
            f"| False accepts (a checksum-valid MRZ returned for a document that carries none) | {old_value} | {old_source} |\n",
        )
        new_flat = rb.flatten_baseline(make_baseline(refusal_population=106, measured_date="2026-09-16"))
        out = rb.rewrite_benchmarks_readme_false_accepts_row(snippet, old_flat, new_flat)
        self.assertIn("**0 / 106**", out)
        self.assertNotIn("**0 / 100**", out)

    def test_no_op_when_new_baseline_has_no_refusal_population(self):
        old_flat = rb.flatten_baseline(make_baseline())
        new_flat = rb.flatten_baseline(make_baseline())
        out = rb.rewrite_benchmarks_readme_false_accepts_row(
            BENCH_README_SNIPPET_WITH_FALSE_ACCEPTS_ROW, old_flat, new_flat
        )
        self.assertEqual(out, BENCH_README_SNIPPET_WITH_FALSE_ACCEPTS_ROW)

    def test_missing_row_raises_rather_than_silently_skipping(self):
        old_flat = rb.flatten_baseline(make_baseline())
        new_flat = rb.flatten_baseline(make_baseline(refusal_population=1))
        with self.assertRaises(ValueError):
            rb.rewrite_benchmarks_readme_false_accepts_row(
                "no false accepts row in this text at all", old_flat, new_flat
            )


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


# --------------------------------------------------------------------------
# FINDINGS.md is the append target for a non-scored re-bless entry (the F1
# "one findings home" change moved `## Weak-spot findings` out of
# knowledge/benchmarks/README.md). This exercises the same two calls
# `main()`'s CLASS_NON_SCORED branch makes -- appending `build_weakspot_entry`'s
# output to FINDINGS.md, then `ixf.write_index` -- against a fixture tree,
# without going through `main()` itself (which needs a real worktree, git and
# gh, out of scope for this offline suite; see the module docstring).
# --------------------------------------------------------------------------


class FindingsAppendTargetTests(unittest.TestCase):
    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.root = Path(self._tmpdir.name)
        bench_dir = self.root / "knowledge" / "benchmarks"
        bench_dir.mkdir(parents=True)
        (self.root / "knowledge" / "WEB_OCR_BASELINE.md").write_text("# Web OCR baseline\n\nNo sections in this fixture.\n", encoding="utf-8")
        (bench_dir / "FINDINGS.md").write_text(
            "# Findings\n\nIntro.\n\n"
            f"{rb.ixf.INDEX_START}\n{rb.ixf.INDEX_END}\n\n"
            "## Weak-spot findings\n\n"
            "Intro to the log.\n\n"
            "### 2026-09-10 — a prior entry\n\nPrior prose.\n",
            encoding="utf-8",
        )
        self.old = make_baseline()
        self.new = make_baseline(
            documents=266,
            measured_date="2026-09-15",
            by_miss_kind={**self.old["by_miss_kind"], "no_mrz_expected": 51},
        )
        self.old_flat, self.new_flat = rb.flatten_baseline(self.old), rb.flatten_baseline(self.new)
        _cls, self.changed = rb.classify_baseline_diff(self.old, self.new)
        self.addCleanup(self._tmpdir.cleanup)

    def test_constant_points_at_findings_not_readme(self):
        self.assertEqual(rb.FINDINGS_REL_PATH, "knowledge/benchmarks/FINDINGS.md")
        self.assertNotEqual(rb.FINDINGS_REL_PATH, rb.BENCH_README_REL_PATH)

    def test_entry_is_appended_to_findings_not_replacing_the_prior_log(self):
        findings_path = self.root / rb.FINDINGS_REL_PATH
        entry = rb.build_weakspot_entry("cohort-c14", 1, self.changed, "34980671424", self.old_flat, self.new_flat)
        findings_path.write_text(findings_path.read_text(encoding="utf-8").rstrip("\n") + "\n" + entry, encoding="utf-8")
        text = findings_path.read_text(encoding="utf-8")
        self.assertIn("a prior entry", text)  # untouched, not overwritten
        self.assertIn("cohort c14", text)  # the new entry landed

    def test_index_regenerates_to_include_the_new_entry(self):
        findings_path = self.root / rb.FINDINGS_REL_PATH
        entry = rb.build_weakspot_entry("cohort-c14", 1, self.changed, "34980671424", self.old_flat, self.new_flat)
        findings_path.write_text(findings_path.read_text(encoding="utf-8").rstrip("\n") + "\n" + entry, encoding="utf-8")
        changed = rb.ixf.write_index(self.root)
        self.assertTrue(changed)
        indexed_text = findings_path.read_text(encoding="utf-8")
        self.assertIn("cohort c14", indexed_text.split(rb.ixf.INDEX_START)[1].split(rb.ixf.INDEX_END)[0])


# --------------------------------------------------------------------------
# download_baseline_artifacts / install_baseline_and_ledger -- the outcome
# ledger's install path, offline (no `gh run download`, just tempdirs).
# --------------------------------------------------------------------------


class DownloadBaselineArtifactsTests(unittest.TestCase):
    def test_returns_baseline_ledger_and_report_paths_under_the_same_artifact_dir(self):
        with mock.patch.object(rb.ac, "run_cmd", return_value="") as run:
            baseline_path, ledger_path, report_path = rb.download_baseline_artifacts(Path("."), "12345")
        run.assert_called_once()
        self.assertEqual(baseline_path.name, "real-specimen-mrz-baseline.json")
        self.assertEqual(ledger_path.name, "real-specimen-outcomes.jsonl")
        self.assertEqual(baseline_path.parent, ledger_path.parent)
        self.assertEqual(report_path.name, "real-specimen-gate-report.json")


class InstallBaselineAndLedgerTests(unittest.TestCase):
    def test_installs_the_ledger_when_the_artifact_has_one(self):
        with tempfile.TemporaryDirectory() as td:
            worktree = Path(td) / "worktree"
            worktree.mkdir()
            artifact_dir = Path(td) / "artifact"
            artifact_dir.mkdir()
            ledger_artifact = artifact_dir / "real-specimen-outcomes.jsonl"
            ledger_artifact.write_text('{"asset_id": "a"}\n', encoding="utf-8")

            touched = rb.install_baseline_and_ledger(make_baseline(), ledger_artifact, worktree)

            self.assertEqual(touched, {rb.BASELINE_REL_PATH, rb.LEDGER_REL_PATH})
            installed_ledger = worktree / rb.LEDGER_REL_PATH
            self.assertTrue(installed_ledger.is_file())
            self.assertEqual(installed_ledger.read_text(encoding="utf-8"), '{"asset_id": "a"}\n')
            installed_baseline = worktree / rb.BASELINE_REL_PATH
            self.assertTrue(installed_baseline.is_file())
            self.assertEqual(json.loads(installed_baseline.read_text(encoding="utf-8")), make_baseline())

    def test_installs_the_baseline_alone_when_the_artifact_has_no_ledger(self):
        # A `real-specimen-mrz-baseline` artifact from a CI run that predates
        # the outcome ledger -- `ledger_artifact` simply does not exist.
        with tempfile.TemporaryDirectory() as td:
            worktree = Path(td) / "worktree"
            worktree.mkdir()
            missing_ledger = Path(td) / "artifact" / "real-specimen-outcomes.jsonl"

            touched = rb.install_baseline_and_ledger(make_baseline(), missing_ledger, worktree)

            self.assertEqual(touched, {rb.BASELINE_REL_PATH})
            self.assertFalse((worktree / rb.LEDGER_REL_PATH).exists())
            self.assertTrue((worktree / rb.BASELINE_REL_PATH).is_file())


if __name__ == "__main__":
    unittest.main()
