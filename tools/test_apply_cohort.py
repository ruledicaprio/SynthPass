#!/usr/bin/env python3
"""
Offline unit tests for tools/apply_cohort.py. No network, no git, no cargo --
every test exercises pure functions against fixtures or injected fakes,
matching tools/test_build_review_artifact.py's pattern.

Run with:
    python -m unittest tools/test_apply_cohort.py
"""

from __future__ import annotations

import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import apply_cohort as ac  # noqa: E402


# A packet in exactly the shape `screen_candidates.py::write_packet` emits,
# with the Verdict column already filled (as `apply_verdicts.py` would leave
# it).
SAMPLE_PACKET = """# Specimen candidates -- automated screen survivors

Generated 2026-09-14 00:00Z by `tools/screen_candidates.py`. 3 candidate(s) survived.

| # | Code | Doc/TD | Series | Host | Licence evidence | Specimen signal | MRZ check | Proposed destination | Proposed licence | Local path | Note | Verdict |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | LTU | passport / TD3 | 2019 | commons.wikimedia.org | CC BY-SA | watermark | valid | public | cc-by-sa | [5784cf79a00b.jpg](work/scouting/c01/staging/5784cf79a00b.jpg) | | public |
| 2 | SMR | id-card / TD1 | 2017 | commons.wikimedia.org | public domain | page-says-specimen | invalid | public | public-domain | [aa11bb22cc33.jpg](work/scouting/c01/staging/aa11bb22cc33.jpg) | blank template | local |
| 3 | XYZ | passport / TD3 | 2020 | example.gov | none stated | watermark | invalid | drop | none-stated | [ff00ee11dd22.jpg](work/scouting/c01/staging/ff00ee11dd22.jpg) | low signal | drop |
"""

SAMPLE_PACKET_UNVERDICTED = SAMPLE_PACKET.replace("| public |\n", "| |\n", 1)


class SplitTableRowTests(unittest.TestCase):
    def test_basic_row(self):
        cells = ac.split_table_row("| a | b | c |")
        self.assertEqual(cells, ["a", "b", "c"])

    def test_escaped_pipe_is_unescaped_and_not_a_split_point(self):
        cells = ac.split_table_row(r"| a\|b | c |")
        self.assertEqual(cells, ["a|b", "c"])

    def test_non_table_line_returns_empty(self):
        self.assertEqual(ac.split_table_row("just prose, no pipes at the edges"), [])

    def test_empty_cells_are_kept(self):
        cells = ac.split_table_row("| 1 | LTU |  |")
        self.assertEqual(cells, ["1", "LTU", ""])


class ParsePacketTests(unittest.TestCase):
    def test_parses_three_rows(self):
        rows = ac.parse_packet(SAMPLE_PACKET)
        self.assertEqual(len(rows), 3)
        self.assertEqual(rows[0]["code"], "LTU")
        self.assertEqual(rows[0]["verdict"], "public")
        self.assertEqual(rows[1]["verdict"], "local")
        self.assertEqual(rows[2]["verdict"], "drop")

    def test_row_key_prefers_sha12_id(self):
        rows = ac.parse_packet(SAMPLE_PACKET)
        self.assertEqual(ac.row_key(rows[0]), "5784cf79a00b")
        self.assertEqual(ac.row_key(rows[1]), "aa11bb22cc33")

    def test_row_key_falls_back_to_row_number(self):
        row = {"local_path": "no link here", "row_number": "7"}
        self.assertEqual(ac.row_key(row), "#7")

    def test_no_header_raises(self):
        with self.assertRaises(ValueError):
            ac.parse_packet("just some prose\nwith no table at all\n")


class VerdictValidationTests(unittest.TestCase):
    def test_full_verdicts_pass(self):
        rows = ac.parse_packet(SAMPLE_PACKET)
        ac.validate_all_verdicts_present(rows)  # should not raise

    def test_blank_verdict_cell_raises_loudly(self):
        rows = ac.parse_packet(SAMPLE_PACKET_UNVERDICTED)
        with self.assertRaises(ValueError) as ctx:
            ac.validate_all_verdicts_present(rows)
        self.assertIn("blank Verdict cell", str(ctx.exception))

    def test_unrecognized_verdict_raises(self):
        rows = ac.parse_packet(SAMPLE_PACKET)
        rows[0]["verdict"] = "maybe"
        with self.assertRaises(ValueError) as ctx:
            ac.validate_all_verdicts_present(rows)
        self.assertIn("unrecognized Verdict", str(ctx.exception))

    def test_empty_packet_raises(self):
        with self.assertRaises(ValueError):
            ac.validate_all_verdicts_present([])

    def test_rows_with_verdict_filters(self):
        rows = ac.parse_packet(SAMPLE_PACKET)
        self.assertEqual(len(ac.rows_with_verdict(rows, "public")), 1)
        self.assertEqual(len(ac.rows_with_verdict(rows, "local")), 1)
        self.assertEqual(len(ac.rows_with_verdict(rows, "drop")), 1)


# --------------------------------------------------------------------------
# 1. Cross-PR duplicate-detection logic
# --------------------------------------------------------------------------


class CrossPrDuplicateGuardTests(unittest.TestCase):
    def test_pending_only_on_another_open_branch_is_excluded(self):
        main_filenames = {"Foo_Passport_Specimen_P0_FOO_2020_mrz.jpg"}
        other = {
            "cohort-c02": {
                "Foo_Passport_Specimen_P0_FOO_2020_mrz.jpg",  # already on main: not excluded
                "Bar_ID_Specimen_2021_back_mrz.jpg",  # only on c02: excluded
            }
        }
        exclusions = ac.compute_cross_pr_exclusions(main_filenames, other)
        self.assertEqual(
            exclusions,
            {"Bar_ID_Specimen_2021_back_mrz.jpg": "pending in open PR branch 'cohort-c02' (not yet on origin/main)"},
        )

    def test_no_other_branches_means_no_exclusions(self):
        self.assertEqual(ac.compute_cross_pr_exclusions({"a.jpg"}, {}), {})

    def test_two_branches_each_contribute_and_first_wins_on_conflict(self):
        other = {
            "cohort-c02": {"X.jpg"},
            "cohort-c03": {"X.jpg", "Y.jpg"},
        }
        exclusions = ac.compute_cross_pr_exclusions(set(), other)
        self.assertEqual(set(exclusions), {"X.jpg", "Y.jpg"})
        # Deterministic: branches are walked in sorted order, so X.jpg's
        # reason names whichever branch sorts first.
        self.assertIn("cohort-c02", exclusions["X.jpg"])

    def test_discover_open_cohort_branches_filters_prefix_and_self(self):
        prs = [
            {"headRefName": "cohort-c02"},
            {"headRefName": "cohort-c03"},
            {"headRefName": "cohort-c01"},  # this run's own branch
            {"headRefName": "feature-unrelated"},
        ]
        branches = ac.discover_open_cohort_branches("cohort-c01", lambda: prs)
        self.assertEqual(branches, ["cohort-c02", "cohort-c03"])

    def test_discover_open_cohort_branches_empty_when_none_open(self):
        self.assertEqual(ac.discover_open_cohort_branches("cohort-c01", lambda: []), [])

    def test_load_corpus_filenames_skips_malformed_lines(self):
        text = '{"filename": "a.jpg"}\n\nnot json\n{"filename": "b.jpg"}\n'
        self.assertEqual(ac.load_corpus_filenames(text), {"a.jpg", "b.jpg"})


# --------------------------------------------------------------------------
# 2. HIT / MISS (checksum_failed) / no-coverage-claim determination
# --------------------------------------------------------------------------


class CoverageStatusTests(unittest.TestCase):
    def test_checksum_valid_is_hit(self):
        row = {"mrz": {"present": True, "observed": {"checksums_valid": True}}}
        self.assertEqual(ac.determine_coverage_status(row), "HIT")

    def test_present_but_checksum_invalid_is_miss(self):
        row = {"mrz": {"present": True, "observed": {"checksums_valid": False}}}
        self.assertEqual(ac.determine_coverage_status(row), "MISS (checksum_failed)")

    def test_not_present_is_no_coverage_claim(self):
        row = {"mrz": {"present": False, "observed": {"checksums_valid": False}}}
        self.assertEqual(ac.determine_coverage_status(row), "no-coverage-claim")

    def test_missing_fields_default_to_no_coverage_claim(self):
        self.assertEqual(ac.determine_coverage_status({}), "no-coverage-claim")

    def test_checksum_valid_wins_even_if_present_is_somehow_false(self):
        # Shouldn't happen in a real manifest (a_no_mrz_specimen_never_records_a_checksum_valid_read
        # in corpus_manifest.rs guards exactly this), but the precedence is
        # checksums_valid first, matching determine_coverage_status's own
        # docstring order.
        row = {"mrz": {"present": False, "observed": {"checksums_valid": True}}}
        self.assertEqual(ac.determine_coverage_status(row), "HIT")


# --------------------------------------------------------------------------
# 3. sync-samples.ps1 -Push -DryRun parsing and the deletion-refusal guardrail
# --------------------------------------------------------------------------


class ParseSyncSamplesDryRunTests(unittest.TestCase):
    def test_parses_summary_line(self):
        out = "--- staged change ---\nrenamed=0 added=3 deleted=0 modified=0\nsome trailing text\n"
        self.assertEqual(
            ac.parse_sync_samples_dryrun(out),
            {"renamed": 0, "added": 3, "deleted": 0, "modified": 0, "no_changes": False},
        )

    def test_no_changes_is_recognized(self):
        out = "No changes -- a push would be a no-op.\n"
        stats = ac.parse_sync_samples_dryrun(out)
        self.assertTrue(stats["no_changes"])
        self.assertEqual(stats["deleted"], 0)

    def test_unparseable_output_raises_rather_than_guessing(self):
        with self.assertRaises(ValueError):
            ac.parse_sync_samples_dryrun("something unexpected happened\n")

    def test_extract_deleted_paths(self):
        name_status = "A\tsamples/passports/new.jpg\nD\tsamples/passports/old.jpg\nM\tsamples/corpus.jsonl\n"
        self.assertEqual(ac.extract_deleted_paths(name_status), ["samples/passports/old.jpg"])

    def test_extract_deleted_paths_ignores_renames(self):
        # A rename shows as "R100\told\tnew", not a leading "D", and must
        # never be counted as a deletion.
        name_status = "R100\tsamples/passports/old.jpg\tsamples/passports/new.jpg\n"
        self.assertEqual(ac.extract_deleted_paths(name_status), [])


class GuardSamplesDataPushTests(unittest.TestCase):
    """The single most important guardrail: never let a real push through
    when the DryRun reports a deletion, without an explicit, doubly-gated
    opt-in."""

    def test_zero_deletions_passes_silently(self):
        stats = {"renamed": 0, "added": 3, "deleted": 0, "modified": 0}
        ac.guard_samples_data_push(stats, allow_deletions=False, confirm=False)  # must not raise

    def test_deletions_without_allow_deletions_is_refused(self):
        stats = {"renamed": 0, "added": 0, "deleted": 8, "modified": 0}
        with self.assertRaises(ac.SamplesDataPushRefused) as ctx:
            ac.guard_samples_data_push(stats, allow_deletions=False, confirm=False)
        self.assertIn("--allow-deletions", str(ctx.exception))

    def test_deletions_with_allow_deletions_but_no_confirm_is_still_refused(self):
        stats = {"renamed": 0, "added": 0, "deleted": 2, "modified": 0}
        with self.assertRaises(ac.SamplesDataPushRefused) as ctx:
            ac.guard_samples_data_push(
                stats, allow_deletions=True, confirm=False, deleted_paths=["samples/passports/x.jpg"]
            )
        self.assertIn("--confirm", str(ctx.exception))
        self.assertIn("samples/passports/x.jpg", str(ctx.exception))

    def test_deletions_with_both_flags_passes(self):
        stats = {"renamed": 0, "added": 0, "deleted": 2, "modified": 0}
        ac.guard_samples_data_push(
            stats, allow_deletions=True, confirm=True, deleted_paths=["samples/passports/x.jpg"]
        )  # must not raise

    def test_renamed_or_modified_alone_never_trips_the_guard(self):
        stats = {"renamed": 5, "added": 0, "deleted": 0, "modified": 3}
        ac.guard_samples_data_push(stats, allow_deletions=False, confirm=False)  # must not raise


# --------------------------------------------------------------------------
# Filename convention validation
# --------------------------------------------------------------------------


class ValidateTargetFilenameTests(unittest.TestCase):
    def test_valid_public_passport_path(self):
        errors = ac.validate_target_filename("public", "passports/Foo_Passport_Specimen_P0_FOO_2020_mrz.jpg")
        self.assertEqual(errors, [])

    def test_valid_local_id_card_path(self):
        errors = ac.validate_target_filename("local", "local/id_cards/Bar_ID_Specimen_2021_back_mrz.jpg")
        self.assertEqual(errors, [])

    def test_local_verdict_without_local_prefix_is_an_error(self):
        errors = ac.validate_target_filename("local", "id_cards/Bar_ID_Specimen_2021_back_mrz.jpg")
        self.assertTrue(any("must start with 'local/'" in e for e in errors))

    def test_public_verdict_with_local_prefix_is_an_error(self):
        errors = ac.validate_target_filename("public", "local/passports/Foo_Passport_Specimen_2020_mrz.jpg")
        self.assertTrue(any("must not start with 'local/'" in e for e in errors))

    def test_bad_extension_is_an_error(self):
        errors = ac.validate_target_filename("public", "passports/Foo_Passport_Specimen_2020_mrz.bmp")
        self.assertTrue(any("extension" in e for e in errors))

    def test_unrecognized_subdir_is_an_error(self):
        errors = ac.validate_target_filename("public", "not_a_real_dir/Foo_Passport_Specimen_2020_mrz.jpg")
        self.assertTrue(any("unrecognized target subdirectory" in e for e in errors))

    def test_doctype_mismatch_with_passports_dir_is_an_error(self):
        errors = ac.validate_target_filename("public", "passports/Foo_ID_Specimen_2020_mrz.jpg")
        self.assertTrue(any("no 'Passport' token" in e for e in errors))

    def test_doctype_mismatch_with_id_cards_dir_is_an_error(self):
        errors = ac.validate_target_filename("public", "id_cards/Foo_Passport_Specimen_2020_mrz.jpg")
        self.assertTrue(any("no 'ID' token" in e for e in errors))

    def test_driving_licenses_dir_has_no_doctype_check(self):
        errors = ac.validate_target_filename("public", "driving_licenses/Foo_Driving_License_Specimen_2020_mrz.jpg")
        self.assertEqual(errors, [])

    def test_covers_dir_accepts_a_passport_cover(self):
        errors = ac.validate_target_filename(
            "public", "covers/Foo_Passport_Specimen_XXX_XXX_2020_no_mrz_cover.jpg"
        )
        self.assertEqual(errors, [])

    def test_covers_dir_accepts_an_id_card_cover(self):
        errors = ac.validate_target_filename(
            "public", "covers/Foo_ID_Specimen_2020_no_mrz_cover.jpg"
        )
        self.assertEqual(errors, [])

    def test_covers_dir_is_a_recognized_subdirectory(self):
        errors = ac.validate_target_filename(
            "local", "local/covers/Foo_Passport_Specimen_XXX_XXX_2020_no_mrz_cover.jpg"
        )
        self.assertEqual(errors, [])


# --------------------------------------------------------------------------
# Licence-vs-verdict warning (must warn, never block)
# --------------------------------------------------------------------------


class LicenceVerdictAgreementTests(unittest.TestCase):
    def test_public_with_none_stated_warns(self):
        warning = ac.check_licence_verdict_agreement("public", "none-stated")
        self.assertIsNotNone(warning)
        self.assertIn("WARNING", warning)

    def test_public_with_cc_by_sa_is_silent(self):
        self.assertIsNone(ac.check_licence_verdict_agreement("public", "cc-by-sa"))

    def test_local_with_none_stated_is_silent(self):
        # none-stated -> local is the norm, not a doubtful pairing.
        self.assertIsNone(ac.check_licence_verdict_agreement("local", "none-stated"))


# --------------------------------------------------------------------------
# origin patch construction: closed-set enforcement
# --------------------------------------------------------------------------


class BuildOriginPatchTests(unittest.TestCase):
    def test_valid_patch(self):
        origin = ac.build_origin_patch("dsh", "https://example.gov/a.jpg", None, "cc-by-sa", "2026-09-14")
        self.assertEqual(origin["found_by"], "dsh")
        self.assertEqual(origin["licence"], "cc-by-sa")

    def test_unknown_found_by_rejected(self):
        with self.assertRaises(ValueError):
            ac.build_origin_patch("worker", None, None, "cc-by-sa", "2026-09-14")

    def test_unknown_licence_rejected(self):
        with self.assertRaises(ValueError):
            ac.build_origin_patch("dsh", None, None, "totally-free", "2026-09-14")


# --------------------------------------------------------------------------
# Changelog fragment: must always pass check-changelog.sh's grammar rule
# --------------------------------------------------------------------------


class ChangelogFragmentTests(unittest.TestCase):
    def test_body_starts_with_a_bullet(self):
        text = ac.build_changelog_fragment("cohort-c01", [("Foo.jpg", "HIT")], 254)
        first_nonblank = next(line for line in text.splitlines() if line.strip())
        self.assertTrue(first_nonblank.startswith("- "))

    def test_marked_as_draft(self):
        text = ac.build_changelog_fragment("cohort-c01", [("Foo.jpg", "HIT")], 254)
        self.assertIn("DRAFT", text)

    def test_row_count_and_totals_appear(self):
        text = ac.build_changelog_fragment("cohort-c01", [("Foo.jpg", "HIT"), ("Bar.jpg", "MISS (checksum_failed)")], 254)
        self.assertIn("254 -> 256", text)
        self.assertIn("Foo.jpg", text)
        self.assertIn("Bar.jpg", text)


# --------------------------------------------------------------------------
# Screened sidecar lookup
# --------------------------------------------------------------------------


class ScreenedSidecarTests(unittest.TestCase):
    def test_keyed_by_sha12_stem_of_staged_path(self):
        text = (
            '{"staged_path": "work/scouting/c01/staging/5784cf79a00b.jpg", '
            '"image_url_final": "https://commons.wikimedia.org/x.jpg", "page_url_final": "https://commons.wikimedia.org/p"}\n'
        )
        by_id = ac.load_screened_sidecar(text)
        self.assertIn("5784cf79a00b", by_id)
        self.assertEqual(by_id["5784cf79a00b"]["image_url_final"], "https://commons.wikimedia.org/x.jpg")

    def test_missing_staged_path_is_skipped(self):
        text = '{"staged_path": null}\n{"no_staged_path_field": true}\n'
        self.assertEqual(ac.load_screened_sidecar(text), {})


class SidecarPathTests(unittest.TestCase):
    def test_sidecar_follows_the_packet_not_the_branch(self):
        from pathlib import Path

        p = Path("work/scouting/c12/packet-c12.md")
        self.assertEqual(ac.sidecar_path_for(p, "cohort-c10"), Path("work/scouting/c12/screened-c12.jsonl"))

    def test_sidecar_falls_back_to_the_branch_name(self):
        from pathlib import Path

        p = Path("work/scouting/x/combined.md")
        self.assertEqual(ac.sidecar_path_for(p, "cohort-c03"), Path("work/scouting/x/screened-c03.jsonl"))


# --------------------------------------------------------------------------
# T11 / A3: CORPUS_COVERAGE.md cover-row templates
# --------------------------------------------------------------------------

SAMPLE_COVERAGE_TABLE = """# Corpus coverage

## Full table

| Code | Country/Entity | Document type(s) | Status | Note |
|---|---|---|---|---|
| BEN | Benin | -- | No specimen yet | -- |
| KHM | Cambodia | Passport | No specimen yet | Current-series passport bio-page, no MRZ present at all |
| GTM | Guatemala | -- | No specimen yet | -- |
"""


class LicenceSummaryTests(unittest.TestCase):
    def test_commons_cc_by_sa(self):
        self.assertEqual(ac.licence_summary("commons.wikimedia.org", "cc-by-sa"), "Commons CC-BY-SA")

    def test_commons_public_domain(self):
        self.assertEqual(ac.licence_summary("upload.wikimedia.org", "public-domain"), "Commons public domain")

    def test_none_stated_uses_host(self):
        self.assertEqual(ac.licence_summary("example.gov", "none-stated"), "example.gov, none-stated licence")

    def test_unrecorded_treated_like_none_stated(self):
        self.assertEqual(ac.licence_summary("example.gov", "unrecorded"), "example.gov, none-stated licence")

    def test_non_commons_host_with_a_stated_licence(self):
        self.assertEqual(ac.licence_summary("example.gov", "cc-by"), "example.gov, CC-BY")


class JoinSeriesTests(unittest.TestCase):
    def test_single(self):
        self.assertEqual(ac.join_series(["2024 series"]), "2024 series")

    def test_pair(self):
        self.assertEqual(ac.join_series(["2024 series", "XXXX series"]), "2024 series and XXXX series")

    def test_three_or_more(self):
        self.assertEqual(
            ac.join_series(["a series", "b series", "c series"]), "a series, b series and c series"
        )

    def test_empty(self):
        self.assertEqual(ac.join_series([]), "")


class CoverDoctypeLabelTests(unittest.TestCase):
    def test_passport_token(self):
        self.assertEqual(ac.cover_doctype_label("Guatemala_Passport_Specimen_P0_GTM_2018_no_mrz_cover.jpg"), "Passport")

    def test_id_token(self):
        self.assertEqual(ac.cover_doctype_label("Foo_ID_Specimen_2020_no_mrz_cover.jpg"), "ID")

    def test_neither_token_falls_back(self):
        self.assertEqual(ac.cover_doctype_label("Foo_Specimen_2020_cover.jpg"), "Document")


class IsCoverTargetTests(unittest.TestCase):
    def test_public_covers_dir(self):
        self.assertTrue(ac.is_cover_target("covers/Foo_Passport_Specimen_2020_no_mrz_cover.jpg"))

    def test_local_covers_dir(self):
        self.assertTrue(ac.is_cover_target("local/covers/Foo_Passport_Specimen_2020_no_mrz_cover.jpg"))

    def test_non_cover_dir(self):
        self.assertFalse(ac.is_cover_target("passports/Foo_Passport_Specimen_P0_FOO_2020_mrz.jpg"))


class BuildCoverCoverageUpdateTests(unittest.TestCase):
    def test_single_entry(self):
        entries = [{"basename": "x_Passport_cover.jpg", "year_label": "2018", "host": "commons.wikimedia.org", "licence": "public-domain"}]
        docs, note = ac.build_cover_coverage_update("GTM", entries, "c13", "2026-09-15")
        self.assertEqual(docs, "Passport (cover)")
        self.assertIn("Cover only in `samples/covers/`", note)
        self.assertIn("(c13, 2026-09-15, Commons public domain)", note)
        self.assertIn("ADR-0012", note)
        self.assertNotIn("Covers only", note)  # singular phrasing only

    def test_multi_entry(self):
        entries = [
            {"basename": "x_Passport_2024_cover.jpg", "year_label": "2024", "host": "commons.wikimedia.org", "licence": "public-domain"},
            {"basename": "x_Passport_XXXX_cover.jpg", "year_label": "XXXX", "host": "commons.wikimedia.org", "licence": "public-domain"},
        ]
        docs, note = ac.build_cover_coverage_update("LKA", entries, "c12", "2026-09-15")
        self.assertEqual(docs, "Passport (cover, 2 series)")
        self.assertIn("Covers only in `samples/covers/`", note)
        self.assertIn("2024 series and XXXX series", note)

    def test_different_licences_are_joined(self):
        entries = [
            {"basename": "a_Passport_cover.jpg", "year_label": "2020", "host": "commons.wikimedia.org", "licence": "cc-by-sa"},
            {"basename": "b_Passport_cover.jpg", "year_label": "2021", "host": "commons.wikimedia.org", "licence": "public-domain"},
        ]
        _docs, note = ac.build_cover_coverage_update("XYZ", entries, "c13", "2026-09-15")
        self.assertIn("Commons CC-BY-SA / Commons public domain", note)


class UpdateCorpusCoverageForCoverTests(unittest.TestCase):
    def test_untouched_row_gets_docs_and_note(self):
        result = ac.update_corpus_coverage_for_cover(
            SAMPLE_COVERAGE_TABLE, "GTM", "Passport (cover)", "Cover only in `samples/covers/` (...)"
        )
        self.assertIn("| GTM | Guatemala | Passport (cover) | No specimen yet | Cover only in `samples/covers/` (...) |", result)
        # Status column (No specimen yet) is never touched.
        self.assertIn("No specimen yet", result)

    def test_row_with_existing_docs_gets_cover_appended_and_note_untouched(self):
        result = ac.update_corpus_coverage_for_cover(
            SAMPLE_COVERAGE_TABLE, "KHM", "Passport (cover)", "would-be-overwritten"
        )
        self.assertIn("| KHM | Cambodia | Passport; cover | No specimen yet | Current-series passport bio-page, no MRZ present at all |", result)
        self.assertNotIn("would-be-overwritten", result)

    def test_other_rows_are_left_byte_identical(self):
        result = ac.update_corpus_coverage_for_cover(SAMPLE_COVERAGE_TABLE, "GTM", "Passport (cover)", "note")
        self.assertIn("| BEN | Benin | -- | No specimen yet | -- |", result)

    def test_unknown_code_raises(self):
        with self.assertRaises(ValueError):
            ac.update_corpus_coverage_for_cover(SAMPLE_COVERAGE_TABLE, "ZZZ", "Passport (cover)", "note")


class CodeToCountryNameTests(unittest.TestCase):
    def test_parses_full_table_rows(self):
        names = ac.code_to_country_name(SAMPLE_COVERAGE_TABLE)
        self.assertEqual(names["GTM"], "Guatemala")
        self.assertEqual(names["KHM"], "Cambodia")
        self.assertNotIn("Code", names)  # header row excluded


# --------------------------------------------------------------------------
# T11 / A4: cover-only changelog fragment
# --------------------------------------------------------------------------


class LicenceCountSentenceTests(unittest.TestCase):
    def test_mixed_licences_and_none_stated(self):
        sentence = ac.licence_count_sentence(["cc-by-sa", "cc-by-sa", "public-domain", "none-stated"])
        self.assertIn("2 CC-BY-SA", sentence)
        self.assertIn("1 public-domain", sentence)
        self.assertIn("1 kept public", sentence)

    def test_all_stated(self):
        sentence = ac.licence_count_sentence(["public-domain", "public-domain"])
        self.assertIn("2 public-domain", sentence)
        self.assertNotIn("kept public on a none-stated", sentence)

    def test_all_none_stated(self):
        sentence = ac.licence_count_sentence(["none-stated"])
        self.assertIn("1 kept public on a none-stated licence.", sentence)


class BuildCoverOnlyChangelogFragmentTests(unittest.TestCase):
    def test_shape_matches_house_style(self):
        cover_entries_by_code = {
            "GTM": [{"basename": "Guatemala_Passport_Specimen_cover.jpg", "year_label": "2018", "host": "commons.wikimedia.org", "licence": "public-domain"}],
            "LKA": [
                {"basename": "Sri_Lanka_Passport_Specimen_2024_cover.jpg", "year_label": "2024", "host": "commons.wikimedia.org", "licence": "public-domain"},
                {"basename": "Sri_Lanka_Passport_Specimen_XXXX_cover.jpg", "year_label": "XXXX", "host": "commons.wikimedia.org", "licence": "public-domain"},
            ],
        }
        text = ac.build_cover_only_changelog_fragment(
            "cohort-c12",
            cover_entries_by_code,
            ["public-domain", "public-domain", "public-domain"],
            266,
            3,
            {"GTM": "Guatemala", "LKA": "Sri Lanka"},
        )
        first_nonblank = next(line for line in text.splitlines() if line.strip())
        self.assertTrue(first_nonblank.startswith("- "))
        self.assertIn("Cohort c12: 3 passport covers in `samples/covers/`", text)
        self.assertIn("Guatemala", text)
        self.assertIn("Sri Lanka (2024, XXXX)", text)
        self.assertIn("`samples/corpus.jsonl` 266 -> 269 rows", text)
        self.assertIn("ADR-0012 as amended", text)
        self.assertIn("2 codes' Docs column now reads `Passport (cover)`", text)

    def test_single_code_singular_wording(self):
        cover_entries_by_code = {
            "GTM": [{"basename": "Guatemala_Passport_Specimen_cover.jpg", "year_label": "2018", "host": "commons.wikimedia.org", "licence": "public-domain"}],
        }
        text = ac.build_cover_only_changelog_fragment(
            "cohort-c20", cover_entries_by_code, ["public-domain"], 300, 1, {"GTM": "Guatemala"}
        )
        self.assertIn("Cohort c20: 1 passport cover in `samples/covers/`", text)
        self.assertIn("1 code's Docs column now reads `Passport (cover)`", text)


# --------------------------------------------------------------------------
# T11 / A6: Git for Windows bash.exe resolution
# --------------------------------------------------------------------------


class FindGitBashTests(unittest.TestCase):
    def _make_git_for_windows_layout(self, root: Path, git_relpath: str, bash_relpath: str) -> Path:
        git_path = root / git_relpath
        git_path.parent.mkdir(parents=True, exist_ok=True)
        git_path.write_text("", encoding="utf-8")
        bash_path = root / bash_relpath
        bash_path.parent.mkdir(parents=True, exist_ok=True)
        bash_path.write_text("", encoding="utf-8")
        return git_path

    def test_resolves_from_mingw64_bin_git_exe(self):
        # git.exe found under mingw64/bin -- a naive parent.parent would land
        # on mingw64 itself, which has no bash.exe; must walk further up.
        with tempfile.TemporaryDirectory() as td:
            root = Path(td) / "Git"
            git_path = self._make_git_for_windows_layout(root, "mingw64/bin/git.exe", "bin/bash.exe")
            with mock.patch.object(ac.sys, "platform", "win32"), mock.patch.object(
                ac.shutil, "which", return_value=str(git_path)
            ):
                self.assertEqual(ac.find_git_bash(), str((root / "bin" / "bash.exe").resolve()))

    def test_resolves_from_cmd_git_exe_with_usr_bin_bash(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td) / "Git"
            git_path = self._make_git_for_windows_layout(root, "cmd/git.exe", "usr/bin/bash.exe")
            with mock.patch.object(ac.sys, "platform", "win32"), mock.patch.object(
                ac.shutil, "which", return_value=str(git_path)
            ):
                self.assertEqual(ac.find_git_bash(), str((root / "usr" / "bin" / "bash.exe").resolve()))

    def test_raises_when_no_bash_found(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td) / "Git"
            git_path = root / "cmd" / "git.exe"
            git_path.parent.mkdir(parents=True, exist_ok=True)
            git_path.write_text("", encoding="utf-8")
            with mock.patch.object(ac.sys, "platform", "win32"), mock.patch.object(
                ac.shutil, "which", return_value=str(git_path)
            ):
                with self.assertRaises(RuntimeError):
                    ac.find_git_bash()

    def test_raises_when_git_not_on_path(self):
        with mock.patch.object(ac.sys, "platform", "win32"), mock.patch.object(ac.shutil, "which", return_value=None):
            with self.assertRaises(RuntimeError):
                ac.find_git_bash()


class ToMsysPathTests(unittest.TestCase):
    def test_converts_drive_letter_form(self):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            posix = ac.to_msys_path(root)
            resolved = str(root.resolve())
            drive, rest = resolved.split(":", 1)
            self.assertEqual(posix, "/" + drive.lower() + rest.replace("\\", "/"))
            self.assertTrue(posix.startswith("/"))
            self.assertNotIn(":", posix)


class RunBashScriptTests(unittest.TestCase):
    def test_win32_wraps_in_cd_to_posix_path(self):
        calls = []

        def fake_run_cmd(cmd, cwd=None, dry_run=False, check=True):
            calls.append((cmd, cwd, dry_run))
            return ""

        with tempfile.TemporaryDirectory() as td:
            worktree = Path(td)
            with mock.patch.object(ac.sys, "platform", "win32"), mock.patch.object(
                ac, "run_cmd", side_effect=fake_run_cmd
            ):
                ac.run_bash_script("bash.exe", "scripts/check-doc-links.sh", worktree, dry_run=False)

        self.assertEqual(len(calls), 1)
        cmd, cwd, dry_run = calls[0]
        self.assertEqual(cmd[0], "bash.exe")
        self.assertEqual(cmd[1], "-c")
        self.assertIn("cd '", cmd[2])
        self.assertIn("scripts/check-doc-links.sh", cmd[2])
        self.assertNotIn(":", cmd[2].split("&&")[0])  # the cd target is POSIX-form, no drive letter
        self.assertEqual(cwd, worktree)
        self.assertFalse(dry_run)

    def test_non_windows_runs_plainly(self):
        calls = []

        def fake_run_cmd(cmd, cwd=None, dry_run=False, check=True):
            calls.append((cmd, cwd, dry_run))
            return ""

        with tempfile.TemporaryDirectory() as td:
            worktree = Path(td)
            with mock.patch.object(ac.sys, "platform", "linux"), mock.patch.object(
                ac, "run_cmd", side_effect=fake_run_cmd
            ):
                ac.run_bash_script("/bin/bash", "scripts/check-doc-links.sh", worktree, dry_run=False)

        self.assertEqual(calls[0][0], ["/bin/bash", "scripts/check-doc-links.sh"])
        self.assertEqual(calls[0][1], worktree)


# --------------------------------------------------------------------------
# T11 / A2: release examples are always rebuilt, not just missing ones
# --------------------------------------------------------------------------


class EnsureBinariesBuiltTests(unittest.TestCase):
    def test_always_runs_the_build_command_with_both_examples(self):
        calls = []

        def fake_run_cmd(cmd, cwd=None, dry_run=False, check=True):
            calls.append(cmd)
            return ""

        with mock.patch.object(ac, "run_cmd", side_effect=fake_run_cmd):
            ac.ensure_binaries_built(Path("unused-worktree"), dry_run=False)

        self.assertEqual(len(calls), 1)
        cmd = calls[0]
        self.assertIn("--example", cmd)
        self.assertIn("corpus_manifest", cmd)
        self.assertIn("check_sample", cmd)
        self.assertEqual(cmd[:2], ["cargo", "build"])

    def test_dry_run_still_prints_command_without_running(self):
        with mock.patch.object(ac.subprocess, "run") as mock_run:
            ac.ensure_binaries_built(Path("unused-worktree"), dry_run=True)
        mock_run.assert_not_called()


if __name__ == "__main__":
    unittest.main()
