#!/usr/bin/env python3
"""
Offline unit tests for tools/index_findings.py. No network, no git, no cargo
-- every test builds a small fixture tree under a temp directory shaped like
`knowledge/benchmarks/` + `knowledge/WEB_OCR_BASELINE.md` and exercises the
module's pure functions and its `--write`/`--check` CLI against it.

Run with:
    python -m unittest tools/test_index_findings.py
"""

from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import index_findings as ix  # noqa: E402


# --------------------------------------------------------------------------
# Pure-function tests
# --------------------------------------------------------------------------


class StripTitleDateSuffixTests(unittest.TestCase):
    def test_em_dash_suffix(self):
        self.assertEqual(
            ix.strip_title_date_suffix("Check-digit blind spots, measured — 2026-08-05"),
            "Check-digit blind spots, measured",
        )

    def test_parens_suffix(self):
        self.assertEqual(
            ix.strip_title_date_suffix("Phase D provider-gap measurement setup (2026-09-17)"),
            "Phase D provider-gap measurement setup",
        )

    def test_comma_suffix(self):
        self.assertEqual(
            ix.strip_title_date_suffix("Texture suppression: three-arm A/B, 2026-09-03"),
            "Texture suppression: three-arm A/B",
        )

    def test_no_trailing_date_is_untouched(self):
        title = "Two of the seven `no_mrz_found` documents have no ICAO zone to find"
        self.assertEqual(ix.strip_title_date_suffix(title), title)

    def test_mid_title_comma_is_not_stripped(self):
        # "Denominator refinement -- the Argentina 2026 specimen has a fake printed MRZ"
        # contains a bare year mid-sentence, not a YYYY-MM-DD suffix -- must survive.
        title = "Denominator refinement — the Argentina 2026 specimen has a fake printed MRZ"
        self.assertEqual(ix.strip_title_date_suffix(title), title)

    def test_mid_title_em_dash_is_not_stripped_when_no_trailing_date(self):
        title = "ADR-0008 chunk 1 — the OCR-stack gap, attributed"
        self.assertEqual(ix.strip_title_date_suffix(title), title)


class GithubAnchorTests(unittest.TestCase):
    """Every expected value here is a real, already-published anchor from
    knowledge/benchmarks/README.md's own self-referencing links (now moved to
    FINDINGS.md) -- not a guess at GitHub's algorithm."""

    def test_em_dash_leaves_a_double_hyphen(self):
        heading = "2026-08-16 — first genuine real-specimen Tier-1 numbers"
        self.assertEqual(
            ix.github_anchor(heading),
            "2026-08-16--first-genuine-real-specimen-tier-1-numbers",
        )

    def test_backticks_and_underscore(self):
        heading = "2026-09-08 — `checksum_failed` is not a clean OCR-accuracy signal"
        self.assertEqual(
            ix.github_anchor(heading),
            "2026-09-08--checksum_failed-is-not-a-clean-ocr-accuracy-signal",
        )

    def test_colon_and_commas_dropped(self):
        heading = "2026-09-14 — cohort c01: three specimens, one HIT, and a filler template in `no_mrz_found`"
        self.assertEqual(
            ix.github_anchor(heading),
            "2026-09-14--cohort-c01-three-specimens-one-hit-and-a-filler-template-in-no_mrz_found",
        )

    def test_the_gate_cost_heading(self):
        # The real heading named in the task: an apostrophe, a comma, and a
        # parenthesized suffix all in one, plus the standard em-dash.
        heading = "2026-09-17 — where the gate's time goes, by outcome class (ADR-0010 step 5)"
        self.assertEqual(
            ix.github_anchor(heading),
            "2026-09-17--where-the-gates-time-goes-by-outcome-class-adr-0010-step-5",
        )


class FirstParagraphEvidenceTests(unittest.TestCase):
    def test_no_marker_is_em_dash(self):
        body = ["Some prose with no bold evidence marker at all.", "", "A second paragraph."]
        self.assertEqual(ix.first_paragraph_evidence(body), "—")

    def test_observed_marker(self):
        body = ["Measured directly. **Observed** and confirmed by a second run."]
        self.assertEqual(ix.first_paragraph_evidence(body), "Observed")

    def test_observed_with_colon(self):
        body = ["**Observed:** same hit count in every format."]
        self.assertEqual(ix.first_paragraph_evidence(body), "Observed")

    def test_derived_marker(self):
        body = ["**Derived** from the observed run, not measured directly."]
        self.assertEqual(ix.first_paragraph_evidence(body), "Derived")

    def test_hypothesized_marker(self):
        body = ["**Hypothesized:** a prediction, not yet run."]
        self.assertEqual(ix.first_paragraph_evidence(body), "Hypothesized")

    def test_marker_in_second_paragraph_is_not_seen(self):
        body = ["First paragraph, no marker.", "", "**Observed** appears only here."]
        self.assertEqual(ix.first_paragraph_evidence(body), "—")

    def test_first_marker_wins_when_paragraph_has_two(self):
        body = ["**Observed:** X moved. **Derived:** Y is computed from X."]
        self.assertEqual(ix.first_paragraph_evidence(body), "Observed")

    def test_leading_blank_lines_are_skipped(self):
        body = ["", "", "**Observed** right after the blank lines."]
        self.assertEqual(ix.first_paragraph_evidence(body), "Observed")


class ParseReportHeaderTests(unittest.TestCase):
    def _text(self, header_line: str) -> str:
        return f"# Some title\n\n{header_line}\n\nBody text.\n"

    def test_parses_all_five_fields(self):
        header = (
            "**Date:** 2026-09-17 · **MAIN:** `b2a0afd` · **DATA:** `469a4ee7` · "
            "**Evidence:** Observed · **Status:** current"
        )
        parsed = ix.parse_report_header(self._text(header), "fixture.md")
        self.assertEqual(parsed["date"], "2026-09-17")
        self.assertEqual(parsed["main"], "`b2a0afd`")
        self.assertEqual(parsed["data"], "`469a4ee7`")
        self.assertEqual(parsed["evidence"], "Observed")
        self.assertEqual(parsed["status"], "current")

    def test_not_stated_main_and_data(self):
        header = "**Date:** 2026-08-05 · **MAIN:** not stated · **DATA:** not stated · **Evidence:** unlabelled (pre-standard) · **Status:** current"
        parsed = ix.parse_report_header(self._text(header), "fixture.md")
        self.assertEqual(parsed["main"], "not stated")
        self.assertEqual(parsed["evidence"], "unlabelled (pre-standard)")

    def test_superseded_status_with_link(self):
        header = (
            "**Date:** 2026-09-08 · **MAIN:** not stated · **DATA:** not stated · "
            "**Evidence:** unlabelled (pre-standard) · **Status:** superseded by "
            "[`denominator-correction-2026-09-09.md`](denominator-correction-2026-09-09.md)"
        )
        parsed = ix.parse_report_header(self._text(header), "fixture.md")
        self.assertEqual(
            parsed["status"],
            "superseded by [`denominator-correction-2026-09-09.md`](denominator-correction-2026-09-09.md)",
        )

    def test_all_four_evidence_values(self):
        for evidence in ("Observed", "Derived", "Hypothesized", "unlabelled (pre-standard)"):
            header = f"**Date:** 2026-09-17 · **MAIN:** not stated · **DATA:** not stated · **Evidence:** {evidence} · **Status:** current"
            parsed = ix.parse_report_header(self._text(header), "fixture.md")
            self.assertEqual(parsed["evidence"], evidence)

    def test_missing_header_raises_naming_the_file(self):
        with self.assertRaises(ValueError) as ctx:
            ix.parse_report_header("# Title\n\nNo header here.\n", "broken.md")
        self.assertIn("broken.md", str(ctx.exception))


# --------------------------------------------------------------------------
# Fixture-tree tests -- a temp directory shaped like the real repo, exercised
# through collect_*_entries / write_index / check_index / main().
# --------------------------------------------------------------------------


def make_report(root: Path, filename: str, title: str, header_line: str, body: str = "Body text.\n") -> None:
    bench_dir = root / "knowledge" / "benchmarks"
    bench_dir.mkdir(parents=True, exist_ok=True)
    text = f"# {title}\n\n{header_line}\n\n{body}"
    (bench_dir / filename).write_text(text, encoding="utf-8")


def make_findings_md(root: Path, log_entries_markdown: str) -> None:
    bench_dir = root / "knowledge" / "benchmarks"
    bench_dir.mkdir(parents=True, exist_ok=True)
    text = (
        "# Findings — the dated log of measurements, sweeps and rejected candidates\n\n"
        "Intro paragraph.\n\n"
        f"{ix.INDEX_START}\n{ix.INDEX_END}\n\n"
        "## Weak-spot findings\n\n"
        "Intro to the log.\n\n"
        f"{log_entries_markdown}"
    )
    (bench_dir / "FINDINGS.md").write_text(text, encoding="utf-8")


def make_web_ocr_baseline(root: Path, sections_markdown: str) -> None:
    knowledge_dir = root / "knowledge"
    knowledge_dir.mkdir(parents=True, exist_ok=True)
    text = "# Web OCR baseline\n\nIntro.\n\n---\n\n" + sections_markdown
    (knowledge_dir / "WEB_OCR_BASELINE.md").write_text(text, encoding="utf-8")


STANDARD_REPORT_HEADER = (
    "**Date:** {date} · **MAIN:** not stated · **DATA:** not stated · "
    "**Evidence:** unlabelled (pre-standard) · **Status:** current"
)


class FixtureTreeTestCase(unittest.TestCase):
    def setUp(self):
        self._tmpdir = tempfile.TemporaryDirectory()
        self.root = Path(self._tmpdir.name)

    def tearDown(self):
        self._tmpdir.cleanup()


class CollectReportEntriesTests(FixtureTreeTestCase):
    def test_collects_dated_report_files_only(self):
        make_report(self.root, "alpha-report-2026-08-05.md", "Alpha report", STANDARD_REPORT_HEADER.format(date="2026-08-05"))
        # Not a dated report (no trailing -YYYY-MM-DD.md) -- must be skipped.
        (self.root / "knowledge" / "benchmarks" / "README.md").write_text("# benchmarks/\n", encoding="utf-8")
        entries = ix.collect_report_entries(self.root)
        self.assertEqual(len(entries), 1)
        self.assertEqual(entries[0].link, "alpha-report-2026-08-05.md")
        self.assertEqual(entries[0].date, "2026-08-05")

    def test_title_date_suffix_is_stripped_in_the_finding_title(self):
        make_report(
            self.root, "beta-2026-09-03.md", "Beta report, 2026-09-03",
            STANDARD_REPORT_HEADER.format(date="2026-09-03"),
        )
        entries = ix.collect_report_entries(self.root)
        self.assertEqual(entries[0].title, "Beta report")

    def test_header_date_mismatch_raises(self):
        make_report(
            self.root, "gamma-2026-09-03.md", "Gamma report",
            STANDARD_REPORT_HEADER.format(date="2026-09-04"),  # wrong date on purpose
        )
        with self.assertRaises(ValueError):
            ix.collect_report_entries(self.root)


class CollectFindingsLogEntriesTests(FixtureTreeTestCase):
    def test_collects_log_headings_in_document_order(self):
        make_findings_md(
            self.root,
            "### 2026-09-09 — first entry\n\nProse.\n\n"
            "### 2026-09-10 — second entry\n\nMore prose.\n",
        )
        entries = ix.collect_findings_log_entries(self.root)
        self.assertEqual([e.date for e in entries], ["2026-09-09", "2026-09-10"])
        self.assertEqual(entries[0].encounter_index, 0)
        self.assertEqual(entries[1].encounter_index, 1)
        self.assertEqual(entries[0].link, "#2026-09-09--first-entry")

    def test_evidence_marker_detected_in_first_paragraph(self):
        make_findings_md(
            self.root,
            "### 2026-09-16 — an observed result\n\n"
            "**Observed:** the count moved as predicted.\n",
        )
        entries = ix.collect_findings_log_entries(self.root)
        self.assertEqual(entries[0].evidence, "Observed")

    def test_status_is_always_current(self):
        make_findings_md(self.root, "### 2026-09-16 — entry\n\nProse.\n")
        entries = ix.collect_findings_log_entries(self.root)
        self.assertEqual(entries[0].status, "current")


class CollectWebOcrEntriesTests(FixtureTreeTestCase):
    def test_collects_top_level_sections_only(self):
        make_web_ocr_baseline(
            self.root,
            "## 2026-09-03 (d) — rotation\n\n"
            "Prose.\n\n"
            "### Before: a sideways photo did not work\n\n"
            "Nested subsection, must not be treated as its own entry.\n\n"
            "## 2026-09-03 (c) — the band upscale filter\n\n"
            "More prose.\n",
        )
        entries = ix.collect_web_ocr_entries(self.root)
        self.assertEqual(len(entries), 2)
        self.assertEqual(entries[0].title, "2026-09-03 (d) — rotation")
        self.assertEqual(entries[0].link, "../WEB_OCR_BASELINE.md#2026-09-03-d--rotation")
        self.assertEqual(entries[0].where, "WEB_OCR_BASELINE.md")

    def test_document_order_is_preserved_as_encounter_index(self):
        make_web_ocr_baseline(
            self.root,
            "## 2026-09-03 (d) — rotation\n\nProse.\n\n"
            "## 2026-09-03 (c) — upscale\n\nProse.\n\n"
            "## 2026-09-03 (b) — wasm\n\nProse.\n",
        )
        entries = ix.collect_web_ocr_entries(self.root)
        self.assertEqual([e.encounter_index for e in entries], [0, 1, 2])


class SortEntriesTests(unittest.TestCase):
    def _entry(self, date: str, file_label: str, encounter_index: int = 0) -> ix.Entry:
        return ix.Entry(
            date=date, title=f"{file_label}-{encounter_index}", link="x", evidence="—",
            status="current", where="x", file_label=file_label, encounter_index=encounter_index,
        )

    def test_newest_date_first(self):
        entries = [self._entry("2026-08-05", "a.md"), self._entry("2026-09-17", "b.md")]
        ordered = ix.sort_entries(entries)
        self.assertEqual([e.date for e in ordered], ["2026-09-17", "2026-08-05"])

    def test_same_date_ties_break_by_filename_ascending(self):
        entries = [self._entry("2026-09-10", "z-report.md"), self._entry("2026-09-10", "a-report.md")]
        ordered = ix.sort_entries(entries)
        self.assertEqual([e.file_label for e in ordered], ["a-report.md", "z-report.md"])

    def test_same_date_same_file_ties_break_by_encounter_index(self):
        entries = [
            self._entry("2026-09-03", "web_ocr_baseline.md", encounter_index=2),
            self._entry("2026-09-03", "web_ocr_baseline.md", encounter_index=0),
            self._entry("2026-09-03", "web_ocr_baseline.md", encounter_index=1),
        ]
        ordered = ix.sort_entries(entries)
        self.assertEqual([e.encounter_index for e in ordered], [0, 1, 2])


# --------------------------------------------------------------------------
# End-to-end: build_index_block / write_index / check_index / main()
# --------------------------------------------------------------------------


class EndToEndTests(FixtureTreeTestCase):
    def _build_minimal_tree(self):
        make_report(
            self.root, "alpha-2026-08-05.md", "Alpha report",
            STANDARD_REPORT_HEADER.format(date="2026-08-05"),
        )
        make_findings_md(self.root, "### 2026-09-09 — a log entry\n\nProse.\n")
        make_web_ocr_baseline(self.root, "## 2026-09-03 (a) — first measurement\n\nProse.\n")

    def test_write_inserts_a_table_between_markers(self):
        self._build_minimal_tree()
        changed = ix.write_index(self.root)
        self.assertTrue(changed)
        text = (self.root / ix.FINDINGS_REL_PATH).read_text(encoding="utf-8")
        self.assertIn("| Date | Finding | Evidence | Status | Where |", text)
        self.assertIn("alpha-2026-08-05.md", text)
        self.assertIn("2026-09-09--a-log-entry", text)
        self.assertIn("WEB_OCR_BASELINE.md#2026-09-03-a--first-measurement", text)

    def test_write_is_idempotent(self):
        self._build_minimal_tree()
        self.assertTrue(ix.write_index(self.root))
        self.assertFalse(ix.write_index(self.root))  # second call: no further change

    def test_check_passes_after_write(self):
        self._build_minimal_tree()
        ix.write_index(self.root)
        self.assertTrue(ix.check_index(self.root))

    def test_check_fails_before_write(self):
        self._build_minimal_tree()
        self.assertFalse(ix.check_index(self.root))

    def test_check_detects_a_hand_edit(self):
        self._build_minimal_tree()
        ix.write_index(self.root)
        findings_path = self.root / ix.FINDINGS_REL_PATH
        text = findings_path.read_text(encoding="utf-8")
        findings_path.write_text(text.replace("alpha-2026-08-05.md", "tampered.md"), encoding="utf-8")
        self.assertFalse(ix.check_index(self.root))

    def test_missing_markers_raise(self):
        bench_dir = self.root / "knowledge" / "benchmarks"
        bench_dir.mkdir(parents=True)
        (bench_dir / "FINDINGS.md").write_text("# Findings\n\nNo markers here.\n", encoding="utf-8")
        with self.assertRaises(ValueError):
            ix.write_index(self.root)

    def test_cli_write_then_check_exit_codes(self):
        self._build_minimal_tree()
        self.assertEqual(ix.main(["--root", str(self.root), "--write"]), 0)
        self.assertEqual(ix.main(["--root", str(self.root), "--check"]), 0)

    def test_cli_check_before_write_exits_1(self):
        self._build_minimal_tree()
        self.assertEqual(ix.main(["--root", str(self.root), "--check"]), 1)


if __name__ == "__main__":
    unittest.main()
