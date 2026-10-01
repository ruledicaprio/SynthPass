"""Tests for archive_query.py: the reader of the per-document benchmark archive (ADR-0024).

Offline and read-only: every archive here is a few hand-built files in a temporary directory.
The files carry one sentinel string wherever the real archive carries document content (OCR text,
zone lines, field values, `miss_reason` text, `argv`, `model_paths`), so a test can say that no
subcommand prints it.
"""
import contextlib
import io
import json
import sys
import tempfile
import unittest
from pathlib import Path

import archive_query as aq

SENTINEL = "SENTINEL-DOCUMENT-CONTENT-DO-NOT-PRINT"
RUN_A = "a" * 64
RUN_B = "b" * 64
RUN_C = "c" * 64
SHA_1 = "1" * 64
SHA_2 = "2" * 64
SHA_3 = "3" * 64
LINE_1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<"
LINE_2 = "L898902C36UTO7408122F1204159ZE184226B<<<<<10"


def header(run_id=RUN_A, **over):
    base = {
        "kind": "run",
        "schema": 1,
        "run_id": run_id,
        "binary_name": "provider-bench",
        "started_unix_ms": 1_000_000_000_000,
        "pid": 1,
        "source": "local",
        "machine": {"label": "win11-i5-4570", "os": "windows", "arch": "x86_64", "cpus": 4},
        "binary": "provider-bench.exe",
        "binary_sha256": "d" * 64,
        "git_commit": "0123456789abcdef0123456789abcdef01234567",
        "working_tree_dirty": False,
        "argv": ["--real-specimens", SENTINEL],
        "scope": {"corpus": "real-specimens", "format": None, "limit": None, "document_type": None,
                  "profile": None, "seed_start": None, "count": 3},
        "tracks": {"private": False, "local": False, "covers": False},
        "samples_data_sha": None,
        "corpus_manifest_sha256": "e" * 64,
        "documents_loaded": 3,
        "labelled_loaded": 3,
        "providers": ["mrz"],
        "ocr_arms": {"texture": "on", "order": "default"},
        "retry_budget": {"max_passes": 14, "max_seconds": 52},
        "mrz_arms": {"class_sweep": "off", "line1_select": "on"},
        "pivot_yy": 26,
        "model_paths": {"detection": SENTINEL, "recognition": SENTINEL},
        "replay_of": None,
        "env": {},
    }
    base.update(over)
    return base


def ledger_row(asset_id, outcome="hit", **over):
    row = {
        "asset_id": asset_id, "name": asset_id.rsplit("/", 1)[-1], "outcome": outcome,
        "miss_reason": None if outcome == "hit" else f"{outcome}: {SENTINEL}",
        "mrz_format": "TD3", "mrz_found": True, "mrz_checksums_valid": outcome == "hit",
        "names_exact": True, "name_error": None, "ocr_ms": 100, "retry_variant_id": "general",
        "retry_budget_hit": False, "retry_stop": "general_valid", "check_states": {"composite": True},
        "retry_damaged_recovery": False, "tier1_damaged_recovery": False,
    }
    row.update(over)
    return row


def doc(run_id, asset_id, sha, outcome="hit", lines=None, zone_mismatch=0, by_field=None, track="public", **row):
    return {
        "kind": "doc", "run_id": run_id, "provider": "mrz", "track": track,
        "name": asset_id.rsplit("/", 1)[-1], "asset_id": asset_id, "source_sha256": sha,
        "ledger_row": ledger_row(asset_id, outcome, **row),
        "read_ok": True, "read_us": 10, "field_correctness": {"surname": "exact"},
        "ocr": {"text": SENTINEL, "rotation": 0, "mrz_band_score": 0.5, "chargrid": None, "ocr_passes": None},
        "tier1_read": None if lines is None else {"lines": lines, "damaged_recovery": False, "valid": True},
        "fields": {"surname": SENTINEL},
        "truth": {"zone_mismatch": zone_mismatch, "compared_cells": 88,
                  "field_mismatch": {"by_field": by_field or {}, "by_line": {}, "coverage": {}}},
    }


def synthetic(run_id, seed, hit=True, miss_kind=None, lines=None, profile="clean", fmt="TD3", **row):
    ledger = {
        "format": fmt, "seed": seed, "profile": profile, "hit": hit, "miss_kind": miss_kind,
        "wrong_accept": False, "prefix_wrong_accept": False, "wrong_fields": [],
        "check_states": {"composite": hit}, "names_exact": True, "name_error": None,
        "line1_flagged": False, "retry_stop": "general_valid", "retry_variant_id": "general",
        "retry_damaged_recovery": False, "tier1_damaged_recovery": False, "elapsed_ms": 1000,
    }
    ledger.update(row)
    return {
        "kind": "doc", "run_id": run_id, "track": "synthetic", "format": fmt, "profile": profile,
        "seed": seed, "ledger_row": ledger,
        "ocr": {"text": SENTINEL, "rotation": 0, "mrz_band_score": 0.5, "chargrid": None, "ocr_passes": None},
        "tier1_read": None if lines is None else {"lines": lines, "damaged_recovery": False, "valid": hit},
        "fields": {"surname": SENTINEL},
        "truth": {"zone_mismatch": 0, "compared_cells": 88, "field_mismatch": None},
    }


class ArchiveCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def write(self, head, records, track="public", name=None, partial=False):
        directory = self.root / track
        directory.mkdir(parents=True, exist_ok=True)
        stem = name or f"20260101T00000{len(list(directory.iterdir()))}Z-{head['binary_name']}-{head['run_id'][:12]}"
        path = directory / (stem + (".jsonl.partial" if partial else ".jsonl"))
        path.write_text("\n".join(json.dumps(x) for x in [head, *records]) + "\n", encoding="utf-8")
        return path

    def run_main(self, *argv):
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stderr(err):
            code = aq.main(list(argv), out=out)
        return code, out.getvalue(), err.getvalue()

    def cli(self, *argv):
        """`main` with `--root` pointing at the temporary archive."""
        command, rest = argv[0], list(argv[1:])
        return self.run_main(command, "--root", str(self.root), *rest)


class RootTests(unittest.TestCase):
    def test_root_is_the_flag_then_the_variable_then_the_git_common_dir(self):
        git = lambda: Path("/clone/.git")  # noqa: E731
        self.assertEqual(aq.resolve_root("/given", {aq.ARCHIVE_ENV: "/var"}, git), Path("/given"))
        self.assertEqual(aq.resolve_root(None, {aq.ARCHIVE_ENV: "/var"}, git), Path("/var"))
        for unset in ({}, {aq.ARCHIVE_ENV: ""}, {aq.ARCHIVE_ENV: "   "}):
            self.assertEqual(aq.resolve_root(None, unset, git), Path("/clone/.git") / aq.ARCHIVE_DIR_NAME)

    def test_the_variable_off_is_an_error_for_a_reader(self):
        for off in ("off", "OFF", "  Off "):
            with self.assertRaises(aq.ArchiveError):
                aq.resolve_root(None, {aq.ARCHIVE_ENV: off}, lambda: Path("/x"))


class RunsTests(ArchiveCase):
    def test_runs_lists_finished_files_skips_and_counts_partial(self):
        self.write(header(RUN_A), [doc(RUN_A, "passports/a.png", SHA_1), doc(RUN_A, "passports/b.png", SHA_2)])
        self.write(header(RUN_B, binary_name="synthpass-bench", providers=["mrz"],
                          scope={"corpus": "synthetic-corpus", "document_type": "TD3", "profile": "clean",
                                 "seed_start": 0, "count": 1, "format": None, "limit": None}),
                   [synthetic(RUN_B, 0)])
        self.write(header(RUN_C), [doc(RUN_C, "passports/a.png", SHA_1)], partial=True)
        code, out, err = self.cli("runs")
        self.assertEqual(code, 0, err)
        lines = out.splitlines()
        self.assertEqual(len(lines), 3, out)
        self.assertIn(RUN_A[:12], lines[0])
        self.assertIn("provider-bench", lines[0])
        self.assertIn("2001-09-09 01:46:40Z", lines[0])
        self.assertIn("0123456", lines[0])
        self.assertIn("real-specimens n=3", lines[0])
        self.assertIn("records=2", lines[0])
        self.assertIn("providers=mrz", lines[0])
        self.assertIn("synthpass-bench", lines[1])
        self.assertIn("synthetic-corpus TD3 clean seed=0 n=1", lines[1])
        self.assertNotIn(RUN_C[:12], out, "a .partial file is not listed")
        self.assertIn("2 run(s), 3 record(s); 1 .partial file(s) skipped", lines[2])

    def test_a_dirty_commit_is_flagged(self):
        self.write(header(RUN_A, working_tree_dirty=True), [])
        self.assertIn("0123456+dirty", self.cli("runs")[1])

    def test_a_schema_2_header_is_refused_and_the_other_runs_are_still_listed(self):
        self.write(header(RUN_A), [doc(RUN_A, "passports/a.png", SHA_1)])
        self.write(header(RUN_B, schema=2), [])
        code, out, err = self.cli("runs")
        self.assertEqual(code, 2)
        self.assertIn(RUN_A[:12], out)
        self.assertNotIn(RUN_B[:12], out)
        self.assertEqual(len(err.strip().splitlines()), 1, err)
        self.assertIn("schema 2 is not supported", err)

    def test_the_local_track_is_a_separate_directory(self):
        self.write(header(RUN_A), [], track="public")
        self.write(header(RUN_B), [], track="local")
        self.assertIn(RUN_A[:12], self.cli("runs")[1])
        self.assertNotIn(RUN_B[:12], self.cli("runs")[1])
        code, out, _ = self.cli("runs", "--track", "local")
        self.assertEqual(code, 0)
        self.assertIn(RUN_B[:12], out)
        self.assertNotIn(RUN_A[:12], out)

    def test_an_absent_archive_is_an_empty_listing_not_an_error(self):
        code, out, _ = self.cli("runs")
        self.assertEqual(code, 0)
        self.assertIn("0 run(s), 0 record(s); 0 .partial file(s) skipped", out)


class ValidationTests(ArchiveCase):
    def test_a_first_line_that_is_not_a_run_header_is_refused(self):
        directory = self.root / "public"
        directory.mkdir()
        path = directory / "20260101T000000Z-provider-bench-aaaaaaaaaaaa.jsonl"
        path.write_text(json.dumps(doc(RUN_A, "passports/a.png", SHA_1)) + "\n", encoding="utf-8")
        with self.assertRaises(aq.ArchiveError) as caught:
            aq.read_run(path)
        self.assertIn("not a run header", str(caught.exception))

    def test_a_record_of_another_run_is_refused(self):
        path = self.write(header(RUN_A), [doc(RUN_B, "passports/a.png", SHA_1)])
        with self.assertRaises(aq.ArchiveError) as caught:
            aq.read_run(path)
        self.assertIn("another run", str(caught.exception))

    def test_a_line_that_is_not_json_is_refused_without_echoing_it(self):
        path = self.write(header(RUN_A), [])
        with path.open("a", encoding="utf-8") as handle:
            handle.write(f"{SENTINEL} not json\n")
        with self.assertRaises(aq.ArchiveError) as caught:
            aq.read_run(path)
        self.assertIn("line 2 is not JSON", str(caught.exception))
        self.assertNotIn(SENTINEL, str(caught.exception))

    def test_unknown_keys_and_unknown_record_kinds_are_ignored(self):
        record = doc(RUN_A, "passports/a.png", SHA_1)
        record["a_future_key"] = {"nested": 1}
        path = self.write(header(RUN_A, a_future_header_key=1), [record, {"kind": "note", "run_id": "x"}])
        run = aq.read_run(path)
        self.assertEqual(len(run.records), 1)

    def test_a_run_is_found_by_prefix_or_path_and_a_partial_is_never_read(self):
        finished = self.write(header(RUN_A), [])
        partial = self.write(header(RUN_B), [], partial=True)
        self.assertEqual(aq.find_run(RUN_A[:6], self.root, "public").path, finished)
        self.assertEqual(aq.find_run(str(finished), self.root, "public").path, finished)
        with self.assertRaises(aq.ArchiveError):
            aq.find_run(RUN_B[:6], self.root, "public")
        with self.assertRaises(aq.ArchiveError):
            aq.find_run(str(partial), self.root, "public")
        with self.assertRaises(aq.ArchiveError):
            aq.find_run("f" * 6, self.root, "public")

    def test_an_ambiguous_prefix_is_an_error(self):
        self.write(header("ab" + "0" * 62), [])
        self.write(header("ab" + "1" * 62), [])
        with self.assertRaises(aq.ArchiveError) as caught:
            aq.find_run("ab", self.root, "public")
        self.assertIn("matches 2 runs", str(caught.exception))

    def test_errors_exit_2_and_a_report_exits_0(self):
        self.write(header(RUN_A), [])
        self.assertEqual(self.cli("diff", RUN_A[:6], "nosuchrun")[0], 2)
        self.assertEqual(self.cli("diff", RUN_A[:6], RUN_A[:6])[0], 0)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as caught:
            aq.main(["cell", "--root", str(self.root)])
        self.assertEqual(caught.exception.code, 2)


class DiffTests(ArchiveCase):
    def two_runs(self, a_docs, b_docs, head_a=None, head_b=None):
        self.write(head_a or header(RUN_A), a_docs)
        self.write(head_b or header(RUN_B), b_docs)
        return self.cli("diff", RUN_A[:8], RUN_B[:8])

    def test_a_renamed_document_joins_on_source_sha256_and_is_the_same_document(self):
        code, out, err = self.two_runs(
            [doc(RUN_A, "passports/old-name.png", SHA_1, "hit"), doc(RUN_A, "passports/other.png", SHA_2)],
            [doc(RUN_B, "passports/new-name.png", SHA_1, "checksum_failed"), doc(RUN_B, "passports/other.png", SHA_2)],
        )
        self.assertEqual(code, 0, err)
        self.assertIn("2 joined (source_sha256 2), 0 only in A, 0 only in B", out)
        self.assertIn("1 joined by source_sha256 under a different asset ID (renamed)", out)
        self.assertIn("outcome changes: 1 document(s)", out)
        self.assertIn("passports/old-name.png: hit -> checksum_failed", out)

    def test_duplicate_images_do_not_join_on_the_hash_alone(self):
        # The same bytes under two asset IDs on both sides: the hash is not a key, the asset ID is.
        code, out, _ = self.two_runs(
            [doc(RUN_A, "passports/x.png", SHA_1, "hit"), doc(RUN_A, "passports/y.png", SHA_1, "hit")],
            # B lists them in the other order, so a join on the hash alone would pair x with y.
            [doc(RUN_B, "passports/y.png", SHA_1, "checksum_failed"), doc(RUN_B, "passports/x.png", SHA_1, "hit")],
        )
        self.assertIn("2 joined (asset_id 2)", out)
        self.assertIn("outcome changes: 1 document(s)\n  passports/y.png: hit -> checksum_failed", out)

    def test_a_document_falls_back_to_the_name_when_nothing_else_is_shared(self):
        a, b = doc(RUN_A, "ignored", None), doc(RUN_B, "ignored", None)
        for record in (a, b):
            record["asset_id"] = None
            record["source_sha256"] = None
            record["name"] = "seed-derived-name"
        code, out, _ = self.two_runs([a], [b])
        self.assertIn("1 joined (name 1)", out)

    def test_documents_in_one_run_only_are_listed_by_asset_id(self):
        code, out, _ = self.two_runs(
            [doc(RUN_A, "passports/only-a.png", SHA_1), doc(RUN_A, "passports/both.png", SHA_2)],
            [doc(RUN_B, "passports/only-b.png", SHA_3), doc(RUN_B, "passports/both.png", SHA_2)],
        )
        self.assertIn("1 joined (source_sha256 1), 1 only in A, 1 only in B", out)
        self.assertIn("only in A: 1 document(s)\n  passports/only-a.png", out)
        self.assertIn("only in B: 1 document(s)\n  passports/only-b.png", out)

    def test_document_lines_are_capped_and_counts_are_not(self):
        import rebless

        self.assertEqual(aq.DOC_LINE_CAP, rebless.LEDGER_DOC_LINE_CAP)
        n = rebless.LEDGER_DOC_LINE_CAP + 5
        a = [doc(RUN_A, f"passports/d{i:03}.png", f"{i:064x}", "hit") for i in range(n)]
        b = [doc(RUN_B, f"passports/d{i:03}.png", f"{i:064x}", "checksum_failed") for i in range(n)]
        code, out, _ = self.two_runs(a, b)
        self.assertIn(f"outcome changes: {n} document(s)", out)
        self.assertIn("  ... and 5 more", out)
        outcome_lines = [line for line in out.splitlines() if line.endswith(": hit -> checksum_failed")]
        self.assertEqual(len(outcome_lines), rebless.LEDGER_DOC_LINE_CAP)

    def test_synthetic_seeds_join_on_format_profile_and_seed(self):
        head_a = header(RUN_A, binary_name="synthpass-bench")
        head_b = header(RUN_B, binary_name="synthpass-bench")
        a = [synthetic(RUN_A, 0), synthetic(RUN_A, 1), synthetic(RUN_A, 2, profile="mobile"), synthetic(RUN_A, 3, fmt="TD1")]
        b = [
            synthetic(RUN_B, 0),
            synthetic(RUN_B, 1, hit=False, miss_kind="checksum_failed"),  # one outcome change
            synthetic(RUN_B, 2, profile="mobile", names_exact=False),  # one field change
            synthetic(RUN_B, 3, fmt="TD2"),  # a different format: a different document
        ]
        code, out, err = self.two_runs(a, b, head_a, head_b)
        self.assertEqual(code, 0, err)
        self.assertIn("3 joined (seed 3), 1 only in A, 1 only in B", out)
        self.assertIn("TD1 clean seed 3", out)
        self.assertIn("TD2 clean seed 3", out)
        self.assertIn("outcome changes: 1 document(s)\n  TD3 clean seed 1: hit -> checksum_failed", out)
        self.assertIn("names_exact true -> false", out)
        self.assertIn("TD3 mobile seed 2", out)

    def test_the_arms_that_differ_are_named(self):
        head_b = header(
            RUN_B,
            ocr_arms={"texture": "off", "order": "default"},
            mrz_arms={"class_sweep": "on", "line1_select": "on"},
            retry_budget={"max_passes": 14, "max_seconds": 600},
            pivot_yy=27,
            env={"SYNTHPASS_OCR_MAX_SECONDS": "600"},
            git_commit="f" * 40,
            working_tree_dirty=True,
            binary_sha256="9" * 64,
        )
        code, out, _ = self.two_runs([], [], header(RUN_A), head_b)
        self.assertIn("header: differs", out)
        for expected in (
            "ocr_arms.texture: on -> off",
            "mrz_arms.class_sweep: off -> on",
            "retry_budget.max_seconds: 52 -> 600",
            "pivot_yy: 26 -> 27",
            "env.SYNTHPASS_OCR_MAX_SECONDS: null -> 600",
            "commit: 0123456 -> fffffff+dirty",
            "binary_sha256: dddddddddddd -> 999999999999",
        ):
            self.assertIn(expected, out)
        self.assertNotIn("ocr_arms.order", out, "an arm that did not change is not named")

    def test_identical_headers_say_so(self):
        code, out, _ = self.two_runs([], [])
        self.assertIn("header: identical", out)

    def test_a_replays_null_budget_is_a_difference(self):
        code, out, _ = self.two_runs([], [], header(RUN_A), header(RUN_B, retry_budget=None))
        self.assertIn("retry_budget:", out)
        self.assertIn("-> null", out)

    def test_timing_alone_is_no_change_and_is_kept_apart(self):
        a = [doc(RUN_A, "passports/a.png", SHA_1, ocr_ms=100), doc(RUN_A, "passports/b.png", SHA_2, ocr_ms=200)]
        b = [doc(RUN_B, "passports/a.png", SHA_1, ocr_ms=150), doc(RUN_B, "passports/b.png", SHA_2, ocr_ms=200)]
        code, out, _ = self.two_runs(a, b)
        self.assertIn("outcome changes: 0 document(s)", out)
        self.assertIn("field changes (deterministic): 0 document(s)", out)
        self.assertIn("timing (kept apart, report-only): differs on 1 of 2 document(s), median |delta| 50 ms, total 300 ms -> 350 ms", out)

    def test_field_changes_use_rebless_field_classes_and_a_budget_limited_document_is_apart(self):
        a = [
            doc(RUN_A, "passports/a.png", SHA_1, retry_variant_id="general", names_exact=True),
            doc(RUN_A, "passports/b.png", SHA_2, retry_stop="general_valid"),
        ]
        b = [
            doc(RUN_B, "passports/a.png", SHA_1, retry_variant_id="pass-03", names_exact=False),
            doc(RUN_B, "passports/b.png", SHA_2, retry_stop="budget", retry_budget_hit=True, retry_variant_id="pass-02"),
        ]
        code, out, _ = self.two_runs(a, b)
        self.assertIn("field changes (deterministic): 1 document(s); names_exact 1, retry_variant_id 1", out)
        self.assertIn("passports/a.png: names_exact true -> false; retry_variant_id general -> pass-03", out)
        self.assertIn("budget-limited documents (every change on them is timing-sensitive): 1 document(s)", out)
        self.assertIn("passports/b.png:", out.split("budget-limited")[1])

    def test_recovered_zone_differences_are_counted_with_cell_positions_and_no_characters(self):
        other = LINE_2[:4] + "X" + LINE_2[5:9] + "Y" + LINE_2[10:]
        a = [
            doc(RUN_A, "passports/a.png", SHA_1, lines=[LINE_1, LINE_2]),
            doc(RUN_A, "passports/b.png", SHA_2, lines=[LINE_1, LINE_2]),
            doc(RUN_A, "passports/c.png", SHA_3, lines=None, outcome="no_mrz_found"),
        ]
        b = [
            doc(RUN_B, "passports/a.png", SHA_1, lines=[LINE_1, other]),
            doc(RUN_B, "passports/b.png", SHA_2, lines=[LINE_1, LINE_2]),
            doc(RUN_B, "passports/c.png", SHA_3, lines=[LINE_1, LINE_2]),
        ]
        code, out, _ = self.two_runs(a, b)
        self.assertIn("recovered zones: 1 of 2 compared document(s) differ; a zone in A only 0, in B only 1", out)
        self.assertIn("passports/a.png: line 2 col 5,10", out)
        self.assertNotIn("X", out.split("recovered zones")[1].split("truth mismatch")[0].replace("recovered zones", ""))

    def test_zone_cell_differences_count_a_missing_cell_as_a_difference(self):
        self.assertEqual(aq.zone_cell_differences(["ABC", "DEF"], ["ABC", "DXF"]), [(2, [2])])
        self.assertEqual(aq.zone_cell_differences(["ABC"], ["ABCD"]), [(1, [4])])
        self.assertEqual(aq.zone_cell_differences(["ABC"], ["ABC", "Z"]), [(2, [1])])
        self.assertEqual(aq.zone_cell_differences(["ABC"], ["ABC"]), [])

    def test_truth_mismatch_deltas_are_counted_by_document_and_by_field(self):
        a = [
            doc(RUN_A, "passports/a.png", SHA_1, zone_mismatch=2, by_field={"surname": 2}),
            doc(RUN_A, "passports/b.png", SHA_2, zone_mismatch=1, by_field={"document_number": 1}),
        ]
        b = [
            doc(RUN_B, "passports/a.png", SHA_1, zone_mismatch=5, by_field={"surname": 4, "nationality": 1}),
            doc(RUN_B, "passports/b.png", SHA_2, zone_mismatch=1, by_field={"document_number": 1}),
        ]
        code, out, _ = self.two_runs(a, b)
        self.assertIn("truth mismatch: 1 of 2 labelled document(s) changed; zone_mismatch total 3 -> 6", out)
        self.assertIn("by field: nationality 0 -> 1, surname 2 -> 4", out)
        self.assertIn("passports/a.png: 2 -> 5", out)

    def test_a_document_with_no_truth_is_not_compared(self):
        a, b = doc(RUN_A, "passports/a.png", SHA_1), doc(RUN_B, "passports/a.png", SHA_1, zone_mismatch=9)
        a["truth"] = None
        code, out, _ = self.two_runs([a], [b])
        self.assertIn("truth mismatch: 0 of 0 labelled document(s) changed", out)

    def test_two_providers_prefix_the_document_keys(self):
        a = [doc(RUN_A, "passports/a.png", SHA_1), doc(RUN_A, "passports/a.png", SHA_1)]
        a[1]["provider"] = "llm"
        b = [doc(RUN_B, "passports/a.png", SHA_1, "checksum_failed"), doc(RUN_B, "passports/a.png", SHA_1)]
        b[1]["provider"] = "llm"
        code, out, _ = self.two_runs(a, b)
        self.assertIn("2 joined", out)
        self.assertIn("mrz: passports/a.png: hit -> checksum_failed", out)


class CellTests(ArchiveCase):
    def run_with_zones(self, zones, fmt="TD3"):
        records = []
        for i, zone in enumerate(zones):
            record = doc(RUN_A, f"passports/d{i}.png", f"{i:064x}", lines=zone)
            record["ledger_row"]["mrz_format"] = fmt
            records.append(record)
        self.write(header(RUN_A), records)

    def test_the_classes_at_a_cell_are_counted_and_a_short_line_is_absent(self):
        self.run_with_zones([
            ["P<UTO", "L8989"],
            ["I<UTO", "98765"],
            ["<<<<<", "<<<<<"],
            ["p-UTO", "l8989"],  # a lowercase letter is a letter; a hyphen is other
            ["P", "L"],  # line 1 is one cell long: column 2 is absent
            None,  # no zone was read at all
        ])
        code, out, err = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "2")
        self.assertEqual(code, 0, err)
        self.assertIn("cell line 1 col 2: 6 record(s)", out)
        self.assertIn("1 with no zone read", out)
        self.assertIn("letter 0, digit 0, filler 3, other 1, absent 1", out)
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "2", "--col", "1")
        self.assertIn("letter 3, digit 1, filler 1, other 0, absent 0", out)

    def test_the_cell_asked_about_in_adr_0021_is_position_1_of_line_1(self):
        self.run_with_zones([["P<UTO", "L8989"], ["P<UTO", "L8989"], ["I<UTO", "L8989"]])
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1")
        self.assertIn("letter 3, digit 0, filler 0, other 0, absent 0", out)

    def test_a_line_past_the_zone_is_absent(self):
        self.run_with_zones([["P<UTO", "L8989"]])
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "3", "--col", "1")
        self.assertIn("absent 1", out)

    def test_the_format_filter_keeps_one_format(self):
        records = [doc(RUN_A, "passports/a.png", SHA_1, lines=["P<UTO"]), doc(RUN_A, "ids/b.png", SHA_2, lines=["I<UTO", "x", "y"])]
        records[1]["ledger_row"]["mrz_format"] = "TD1"
        self.write(header(RUN_A), records)
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1", "--format", "td1")
        self.assertIn("cell line 1 col 1 (TD1): 1 record(s)", out)
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1", "--document-type", "TD3")
        self.assertIn("(TD3): 1 record(s)", out)

    def test_a_synthetic_run_answers_by_its_own_format(self):
        self.write(header(RUN_A, binary_name="synthpass-bench"),
                   [synthetic(RUN_A, 0, lines=["P<UTO"]), synthetic(RUN_A, 1, lines=["I<UTO"], fmt="TD1")])
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1", "--document-type", "TD1")
        self.assertIn("(TD1): 1 record(s)", out)

    def test_chars_reports_the_characters_counts_on_the_public_track_only(self):
        self.run_with_zones([["P<UTO", "x"], ["P<UTO", "x"], ["I<UTO", "x"]])
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1", "--chars")
        self.assertEqual(code, 0)
        self.assertIn("'I' 1", out)
        self.assertIn("'P' 2", out)

    def test_chars_refuses_the_local_track(self):
        self.write(header(RUN_A), [doc(RUN_A, "local/a.png", SHA_1, lines=["P<UTO"], track="local")], track="local")
        code, out, err = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1", "--chars", "--track", "local")
        self.assertEqual(code, 2)
        self.assertEqual(out, "")
        self.assertIn("only allowed on the public track", err)
        # The same file named by its path, with the default track: still a local file.
        path = next((self.root / "local").iterdir())
        code, out, err = self.cli("cell", str(path), "--line", "1", "--col", "1", "--chars")
        self.assertEqual(code, 2)
        self.assertEqual(out, "")
        # A file outside any track directory, named with `--track local`: refused on the flag.
        elsewhere = self.write(header(RUN_B), [doc(RUN_B, "local/b.png", SHA_2, lines=["P<UTO"], track="local")], track="elsewhere")
        code, out, err = self.cli("cell", str(elsewhere), "--line", "1", "--col", "1", "--chars", "--track", "local")
        self.assertEqual(code, 2)
        self.assertEqual(out, "")
        # Without --chars the local track is readable: classes, no characters.
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1", "--track", "local")
        self.assertEqual(code, 0)
        self.assertIn("letter 1", out)

    def test_a_line_or_column_below_1_is_a_usage_error(self):
        self.run_with_zones([["P<UTO"]])
        self.assertEqual(self.cli("cell", RUN_A[:8], "--line", "0", "--col", "1")[0], 2)
        self.assertEqual(self.cli("cell", RUN_A[:8], "--line", "1", "--col", "0")[0], 2)

    def test_conflicting_format_flags_are_a_usage_error(self):
        self.run_with_zones([["P<UTO"]])
        code, _, err = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1", "--format", "TD1", "--document-type", "TD3")
        self.assertEqual(code, 2)


class DisclosureTests(ArchiveCase):
    """No subcommand prints document content: OCR text, a zone line, a field value, `argv`,
    `model_paths` or the text of a `miss_reason`."""

    def build(self):
        zone = [f"P<UTO{SENTINEL}<<<", f"L898902C36{SENTINEL}"]
        a = [
            doc(RUN_A, "passports/a.png", SHA_1, "hit", lines=zone, zone_mismatch=1, by_field={"surname": 1}),
            doc(RUN_A, "passports/b.png", SHA_2, "checksum_failed", lines=zone),
            doc(RUN_A, "passports/only-a.png", SHA_3, "no_mrz_found"),
        ]
        zone_b = [f"P<UTO{SENTINEL}<<<", f"L898902C3X{SENTINEL}"]
        b = [
            doc(RUN_B, "passports/a.png", SHA_1, "checksum_failed", lines=zone_b, zone_mismatch=4, by_field={"surname": 4}),
            doc(RUN_B, "passports/b.png", SHA_2, "hit", lines=zone_b),
        ]
        self.write(header(RUN_A), a)
        self.write(header(RUN_B), b)
        self.write(header(RUN_C, binary_name="synthpass-bench"), [synthetic(RUN_C, 0, lines=zone)])
        self.write(header("d" * 64, binary_name="synthpass-bench"), [synthetic("d" * 64, 0, hit=False, miss_kind="checksum_failed", lines=zone_b)])

    def test_every_subcommand_runs_without_printing_the_sentinel(self):
        self.build()
        invocations = [
            ("runs",),
            ("diff", RUN_A[:8], RUN_B[:8]),
            ("diff", RUN_C[:8], "d" * 8),
            ("cell", RUN_A[:8], "--line", "1", "--col", "6"),
            ("cell", RUN_C[:8], "--line", "2", "--col", "12"),
            ("cell", RUN_A[:8], "--line", "1", "--col", "6", "--chars"),
        ]
        for invocation in invocations:
            code, out, err = self.cli(*invocation)
            self.assertEqual(code, 0, f"{invocation}: {err}")
            self.assertNotIn(SENTINEL, out + err, f"{invocation} printed document content")
            self.assertNotIn("SENTINEL", out + err, f"{invocation} printed part of a document's content")
        # The refusals and the error lines do not echo content either.
        for invocation in (("diff", RUN_A[:8], "nosuch"), ("cell", RUN_A[:8], "--line", "0", "--col", "1")):
            code, out, err = self.cli(*invocation)
            self.assertEqual(code, 2)
            self.assertNotIn(SENTINEL, out + err)

    def test_a_miss_reason_appears_as_its_kind_only(self):
        self.build()
        code, out, _ = self.cli("diff", RUN_A[:8], RUN_B[:8])
        self.assertIn("hit -> checksum_failed", out)
        self.assertNotIn("checksum_failed:", out)

    def test_a_changed_miss_reason_is_reported_as_detail_changed_never_as_text(self):
        a = doc(RUN_A, "passports/a.png", SHA_1, "checksum_failed")
        b = doc(RUN_B, "passports/a.png", SHA_1, "checksum_failed")
        b["ledger_row"]["miss_reason"] = f"checksum_failed: other {SENTINEL}"
        self.write(header(RUN_A), [a])
        self.write(header(RUN_B), [b])
        code, out, _ = self.cli("diff", RUN_A[:8], RUN_B[:8])
        self.assertIn("miss_reason checksum_failed (detail changed)", out)
        self.assertNotIn(SENTINEL, out)


if __name__ == "__main__":
    unittest.main()
