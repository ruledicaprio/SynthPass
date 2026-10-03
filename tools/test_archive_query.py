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

    def test_runs_counts_only_the_doc_records(self):
        path = self.write(header(RUN_A), [doc(RUN_A, "passports/a.png", SHA_1), {"kind": "note", "run_id": RUN_A}, doc(RUN_A, "passports/b.png", SHA_2)])
        with path.open("a", encoding="utf-8") as handle:
            handle.write("\n")
        code, out, _ = self.cli("runs")
        self.assertIn("records=2", out)
        self.assertIn("1 run(s), 2 record(s)", out)
        # `diff` and `cell` read the same two records.
        self.assertEqual(len(aq.read_run(path).records), 2)

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

    def test_a_differing_model_hash_is_named_as_a_prefix_and_an_equal_one_is_not(self):
        models = {"detection": "a" * 64, "recognition": "b" * 64}
        head_a = header(RUN_A, model_sha256=models)
        head_b = header(RUN_B, model_sha256={"detection": "c" * 64, "recognition": "b" * 64})
        code, out, _ = self.two_runs([], [], head_a, head_b)
        self.assertIn("  model_sha256.detection: aaaaaaaaaaaa -> cccccccccccc", out)
        self.assertNotIn("model_sha256.recognition", out, "an equal hash is not named")
        self.assertNotIn("a" * 13, out, "a prefix, never the whole hash")
        self.assertNotIn(SENTINEL, out, "the model paths are still never printed")
        self.setUp()
        code, out, _ = self.two_runs([], [], head_a, header(RUN_B, model_sha256=dict(models)))
        self.assertIn("header: identical", out)
        self.assertNotIn("model_sha256", out)

    def test_an_archive_written_before_the_hashes_still_reads(self):
        # The default fixture has no `model_sha256` key, as a file written before it does not.
        self.assertNotIn("model_sha256", header(RUN_A))
        code, out, err = self.two_runs([doc(RUN_A, "passports/a.png", SHA_1)], [doc(RUN_B, "passports/a.png", SHA_1)])
        self.assertEqual(code, 0, err)
        self.assertIn("header: identical", out)
        # One side with hashes and one without is a difference, shown as null against a prefix.
        self.setUp()
        head_b = header(RUN_B, model_sha256={"detection": "c" * 64, "recognition": None})
        code, out, _ = self.two_runs([], [], header(RUN_A), head_b)
        self.assertIn("  model_sha256.detection: null -> cccccccccccc", out)
        self.assertNotIn("model_sha256.recognition", out, "null against null is no difference")

    def test_identical_headers_say_so(self):
        code, out, _ = self.two_runs([], [])
        self.assertIn("header: identical", out)

    def test_the_source_and_the_machine_label_are_named_when_they_differ(self):
        head_b = header(
            RUN_B,
            source="ci",
            machine={"label": "linux-xeon-8272CL", "os": "linux", "arch": "x86_64", "cpus": 4},
        )
        code, out, _ = self.two_runs([], [], header(RUN_A), head_b)
        self.assertIn("header: differs", out)
        self.assertIn("  source: local -> ci", out)
        self.assertIn("  machine.label: win11-i5-4570 -> linux-xeon-8272CL", out)
        self.assertNotIn("machine.os", out, "only the label is named: it is the hardware, never a host")
        # The same source and label on both sides: nothing to name.
        self.write(header(RUN_C), [])
        self.write(header("d" * 64), [])
        code, out, _ = self.cli("diff", RUN_C[:8], "d" * 8)
        self.assertIn("header: identical", out)
        self.assertNotIn("source:", out)

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
        # The flag stands on its own too: a file whose records say `public`, outside `local/`,
        # that the caller names with `--track local`.
        named = self.write(header("e" * 64), [doc("e" * 64, "passports/e.png", SHA_2, lines=["P<UTO"])], track="named")
        code, out, err = self.cli("cell", str(named), "--line", "1", "--col", "1", "--chars", "--track", "local")
        self.assertEqual((code, out), (2, ""))
        # The directory check stands on its own: records that say `public` in a file that sits in
        # `local/` are still refused.
        sitting = self.write(header(RUN_C), [doc(RUN_C, "passports/c.png", SHA_3, lines=["P<UTO"])], track="local")
        code, out, err = self.cli("cell", str(sitting), "--line", "1", "--col", "1", "--chars")
        self.assertEqual((code, out), (2, ""))
        # Without --chars the local track is readable: classes, no characters.
        code, out, _ = self.cli("cell", RUN_A[:8], "--line", "1", "--col", "1", "--track", "local")
        self.assertEqual(code, 0)
        self.assertIn("letter 1", out)

    def test_chars_refuses_a_run_whose_records_are_not_all_on_an_allowed_track(self):
        # A local run file copied out of `local/`: the directory says nothing, the records do.
        copied = self.write(header(RUN_A), [doc(RUN_A, "local/a.png", SHA_1, lines=["P<UTO"], track="local")], track="copied")
        code, out, err = self.cli("cell", str(copied), "--line", "1", "--col", "1", "--chars")
        self.assertEqual(code, 2)
        self.assertEqual(out, "")
        self.assertIn("only allowed on the public track", err)
        # Without --chars the same file is readable: classes only, no character.
        code, out, _ = self.cli("cell", str(copied), "--line", "1", "--col", "1")
        self.assertEqual(code, 0)
        self.assertIn("letter 1", out)
        self.assertNotIn("'P'", out)

    def test_chars_is_an_allowlist_of_tracks(self):
        for track in ("public", "covers", "synthetic"):
            path = self.write(header(RUN_A), [doc(RUN_A, "passports/a.png", SHA_1, lines=["P<UTO"], track=track)], track=f"ok-{track}")
            code, out, err = self.cli("cell", str(path), "--line", "1", "--col", "1", "--chars")
            self.assertEqual(code, 0, f"{track}: {err}")
            self.assertIn("'P' 1", out)
        refused = ["local", "private", "", "Public", 7, None]
        for n, track in enumerate(refused):
            record = doc(RUN_A, "passports/a.png", SHA_1, lines=["P<UTO"], track=track)
            path = self.write(header(RUN_A), [record], track=f"bad-{n}")
            code, out, _ = self.cli("cell", str(path), "--line", "1", "--col", "1", "--chars")
            self.assertEqual((code, out), (2, ""), f"track {track!r} must be refused")
        # A missing `track` key is unknown too.
        record = doc(RUN_A, "passports/a.png", SHA_1, lines=["P<UTO"])
        del record["track"]
        path = self.write(header(RUN_A), [record], track="bad-missing")
        self.assertEqual(self.cli("cell", str(path), "--line", "1", "--col", "1", "--chars")[0], 2)
        # One record outside the list is enough, whatever the others say.
        mixed = [doc(RUN_A, "passports/a.png", SHA_1, lines=["P<UTO"]), doc(RUN_A, "local/b.png", SHA_2, lines=["P<UTO"], track="local")]
        path = self.write(header(RUN_A), mixed, track="bad-mixed")
        self.assertEqual(self.cli("cell", str(path), "--line", "1", "--col", "1", "--chars")[0], 2)

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


MODELS = {"detection": "f" * 64, "recognition": "9" * 64}
SHA_4 = "4" * 64
SHA_5 = "5" * 64
SHA_6 = "6" * 64
# What a zone line holds where the tests plant text that must never be printed: a letter pair no
# class symbol, id or count could spell.
TEXT_SENTINEL = "QXJZ"


def code_doc(run_id, asset_id, sha, printed, read, fmt="TD3", verdict=None, track="public", tail=TEXT_SENTINEL, **row):
    """A public-style record whose printed code is `printed` (class symbols) and whose recovered zone
    starts with `read` (text; `None` for no read), then `tail`."""
    lines = None if read is None else [read + tail + ("<<<" if tail else ""), "L898902C36" + TEXT_SENTINEL]
    record = doc(run_id, asset_id, sha, "hit" if read is not None else "no_mrz_found", lines=lines, track=track, **row)
    record["ledger_row"]["mrz_format"] = fmt if read is not None else None
    record["truth"]["code_cells"] = printed
    record["truth"]["zone"] = TEXT_SENTINEL  # a key the real record never has: it must not be printed either
    if verdict is not None:
        record["field_correctness"] = {CODE_KEY: verdict, "surname": "exact"}
    return record


CODE_KEY = "document_type"


def private_doc(run_id, sha, printed, zone_classes, fmt="TD3", **over):
    record = {
        "kind": "private_doc", "run_id": run_id, "provider": "mrz", "track": "private",
        "source_sha256": sha, "outcome": "hit", "mrz_format": fmt, "mrz_found": True,
        "mrz_checksums_valid": True, "check_states": {"composite": True},
        "retry_variant_id": "pass-03", "retry_budget_hit": False, "retry_stop": "variant_valid",
        "retry_damaged_recovery": False, "tier1_damaged_recovery": None, "read_us": 1, "ocr_ms": 1,
        "mrz_band_score": 0.5, "rotation": 0,
        "truth": {"zone_mismatch": 0, "compared_cells": 88, "field_mismatch": None, "code_cells": printed},
        "zone_classes": zone_classes,
    }
    record.update(over)
    return record


def witness_run(run_id, **head):
    """Six public documents: a `P<` read as `PS`, a `PS` read as `P<`, a `PO` read correctly, a `P<`
    read correctly, a document nothing was read from, and one with no printed code class."""
    records = [
        code_doc(run_id, "passports/p-lt-as-ps.png", SHA_1, "A<", "PS", verdict="wrong", retry_variant_id="pass-05",
                 retry_stop="variant_valid"),
        code_doc(run_id, "passports/ps-as-p-lt.png", SHA_2, "AA", "P<", verdict="exact"),
        code_doc(run_id, "passports/po-right.png", SHA_3, "AA", "PO", verdict="exact"),
        code_doc(run_id, "passports/p-lt-right.png", SHA_4, "A<", "P<", verdict="exact"),
        code_doc(run_id, "passports/unread.png", SHA_5, "AA", None, verdict="unread", retry_stop="budget",
                 retry_variant_id=None),
    ]
    unlabelled = doc(run_id, "passports/unlabelled.png", SHA_6, lines=["P<UTO", "L8"])
    unlabelled["truth"] = None
    return header(run_id, **{"model_sha256": MODELS, **head}), records + [unlabelled]


class CodesTests(ArchiveCase):
    def write_witness(self, run_id=RUN_A, **head):
        head, records = witness_run(run_id, **head)
        return self.write(head, records)

    def test_the_matrix_counts_the_printed_class_by_the_observed_class(self):
        self.write_witness()
        code, out, err = self.cli("codes", RUN_A[:8])
        self.assertEqual(code, 0, err)
        self.assertIn("5 with a printed code class, 1 skipped for none", out)
        self.assertIn("provider mrz, format TD3: 4 document(s)", out)
        self.assertIn("provider mrz, format unknown: 1 document(s)", out)
        self.assertIn("  printed A< -> observed A< 1, AA 1", out)
        self.assertIn("  printed AA -> observed A< 1, AA 1", out)
        self.assertIn("  printed AA -> observed unread 1", out)

    def test_a_record_without_a_printed_class_is_skipped_and_counted(self):
        head, records = witness_run(RUN_A)
        records[0]["truth"].pop("code_cells")  # an archive written before the key existed
        records[1]["truth"]["code_cells"] = None
        self.write(head, records)
        _code, out, _ = self.cli("codes", RUN_A[:8])
        self.assertIn("3 with a printed code class, 3 skipped for none", out)

    def test_a_printed_class_that_is_not_class_symbols_is_a_schema_error_naming_no_content(self):
        head, records = witness_run(RUN_A)
        records[0]["truth"]["code_cells"] = "PS"
        self.write(head, records)
        code, out, err = self.cli("codes", RUN_A[:8])
        self.assertEqual((code, out), (2, ""))
        self.assertIn("class symbols", err)
        self.assertNotIn("PS", err.replace("class symbols", ""))

    def test_the_observed_class_is_the_first_two_cells_in_classes(self):
        head, records = witness_run(RUN_A)
        reads = ["p1", "é<", "<", "9A", "-"]
        records = [
            code_doc(RUN_A, f"passports/r{i}.png", f"{i + 1:064x}", "AA", read, tail="")
            for i, read in enumerate(reads)
        ]
        self.write(head, records)
        _code, out, _ = self.cli("codes", RUN_A[:8])
        # lowercase is a letter, a non-ASCII letter and a hyphen are other, one cell is one symbol
        self.assertIn("  printed AA -> observed 9A 1, < 1, ? 1, ?< 1, A9 1", out)

    def test_an_empty_first_line_is_unread(self):
        head, _records = witness_run(RUN_A)
        record = code_doc(RUN_A, "passports/e.png", SHA_1, "AA", "")
        record["tier1_read"]["lines"] = []
        self.write(head, [record])
        _code, out, _ = self.cli("codes", RUN_A[:8])
        self.assertIn("printed AA -> observed unread 1", out)

    def test_the_document_type_field_is_counted_per_printed_class(self):
        self.write_witness()
        _code, out, _ = self.cli("codes", RUN_A[:8])
        self.assertIn("  printed A<, document_type field: exact 1, wrong 1, unread 0, no entry 0", out)
        self.assertIn("  printed AA, document_type field: exact 2, wrong 0, unread 1, no entry 0", out)

    def test_a_record_with_no_document_type_entry_is_counted_as_no_entry(self):
        head, _ = witness_run(RUN_A)
        record = code_doc(RUN_A, "passports/n.png", SHA_1, "A<", "P<")
        self.write(head, [record])
        _code, out, _ = self.cli("codes", RUN_A[:8])
        self.assertIn("printed A<, document_type field: exact 0, wrong 0, unread 0, no entry 1", out)

    def test_the_retry_facts_are_counted_for_the_documents_that_differ_only(self):
        self.write_witness()
        _code, out, _ = self.cli("codes", RUN_A[:8])
        # the two misreads and the unread one; the two correct reads (retry general) are not in it
        lines = out.splitlines()
        td3 = lines[lines.index("provider mrz, format TD3: 4 document(s)"):]
        self.assertIn(
            "  observed class differs from printed: 2 document(s); retry_variant_id general 1, pass-05 1; "
            "retry_stop general_valid 1, variant_valid 1",
            td3,
        )
        unknown = lines[lines.index("provider mrz, format unknown: 1 document(s)"):]
        self.assertIn(
            "  observed class differs from printed: 1 document(s); retry_variant_id none 1; retry_stop budget 1",
            unknown,
        )

    def test_a_retry_fact_outside_the_closed_sets_is_not_printed(self):
        head, _ = witness_run(RUN_A)
        record = code_doc(RUN_A, "passports/x.png", SHA_1, "AA", "P<", retry_variant_id=TEXT_SENTINEL, retry_stop=TEXT_SENTINEL)
        self.write(head, [record])
        _code, out, _ = self.cli("codes", RUN_A[:8])
        self.assertIn("retry_variant_id other 1; retry_stop other 1", out)
        self.assertNotIn(TEXT_SENTINEL, out)

    def test_the_documents_that_differ_are_named_by_asset_id(self):
        self.write_witness()
        _code, out, _ = self.cli("codes", RUN_A[:8])
        self.assertIn("  passports/p-lt-as-ps.png: printed A<, observed AA", out)
        self.assertIn("  passports/ps-as-p-lt.png: printed AA, observed A<", out)
        self.assertIn("  passports/unread.png: printed AA, observed unread", out)
        self.assertNotIn("po-right", out)
        self.assertNotIn("p-lt-right", out)

    def test_the_local_track_is_counted_and_never_named(self):
        head, _ = witness_run(RUN_A)
        record = code_doc(RUN_A, "local/secret-name.png", SHA_1, "AA", "P<", track="local")
        self.write(head, [record], track="local")
        code, out, err = self.cli("codes", RUN_A[:8], "--track", "local")
        self.assertEqual(code, 0, err)
        self.assertIn("observed class differs from printed: 1 document(s)", out)
        self.assertIn("1 more on the local or private track, not named", out)
        self.assertNotIn("secret-name", out)

    def test_a_private_record_is_read_from_its_classes_and_never_named(self):
        head = header(RUN_C, tracks={"private": True, "local": False, "covers": False})
        records = [
            private_doc(RUN_C, SHA_1, "A<", ["AA9<<", "A9"]),  # a `P<` read as two letters
            private_doc(RUN_C, SHA_2, "AA", ["AA9<<", "A9"]),  # right
            private_doc(RUN_C, SHA_3, "AA", None, retry_variant_id=None, retry_stop="exhausted"),  # unread
        ]
        path = self.write(head, records, track="private")
        code, out, err = self.cli("codes", str(path))
        self.assertEqual(code, 0, err)
        self.assertIn("provider mrz, format TD3: 3 document(s)", out)
        self.assertIn("  printed A< -> observed AA 1", out)
        self.assertIn("  printed AA -> observed AA 1, unread 1", out)
        self.assertIn("  printed AA, document_type field: exact 0, wrong 0, unread 0, no entry 2", out)
        self.assertIn("observed class differs from printed: 2 document(s); retry_variant_id none 1, pass-03 1", out)
        self.assertIn("  2 more on the local or private track, not named", out)
        for sha in (SHA_1, SHA_2, SHA_3):
            self.assertNotIn(sha[:12], out)
        self.assertNotIn("private_doc", out)

    def test_a_private_record_in_a_public_listing_is_not_named_either(self):
        head, records = witness_run(RUN_A)
        records.append(private_doc(RUN_A, SHA_6, "AA", ["A9"]))
        self.write(head, records)
        _code, out, _ = self.cli("codes", RUN_A[:8])
        self.assertIn("1 more on the local or private track, not named", out)
        self.assertNotIn(SHA_6[:12], out)

    def test_the_other_commands_still_ignore_private_records(self):
        head = header(RUN_C)
        self.write(head, [private_doc(RUN_C, SHA_1, "A<", ["AA"])], track="public")
        code, out, _ = self.cli("runs")
        self.assertIn("records=0", out)
        code, out, _ = self.cli("cell", RUN_C[:8], "--line", "1", "--col", "1")
        self.assertIn("0 record(s)", out)

    # ---- two runs

    def write_pair(self, mutate_b=None, **head_b):
        head_a, records_a = witness_run(RUN_A)
        head_b_full, records_b = witness_run(RUN_B, **head_b)
        # B reads the `P<` document as `P<` at last, and loses the `PO`
        records_b[0] = code_doc(RUN_B, "passports/p-lt-as-ps.png", SHA_1, "A<", "P<", verdict="exact")
        records_b[2] = code_doc(RUN_B, "passports/po-right.png", SHA_3, "AA", "P<", verdict="wrong")
        if mutate_b:
            mutate_b(head_b_full, records_b)
        self.write(head_a, records_a)
        self.write(head_b_full, records_b)

    def test_two_runs_print_both_matrices_and_the_documents_whose_class_changed(self):
        self.write_pair()
        code, out, err = self.cli("codes", RUN_A[:8], RUN_B[:8])
        self.assertEqual(code, 0, err)
        self.assertIn("  printed A< -> observed A: A< 1, AA 1 | B: A< 2", out)
        self.assertIn("  printed AA -> observed A: A< 1, AA 1 | B: A< 2", out)
        self.assertIn("  printed AA -> observed A: unread 1 | B: unread 1", out)
        self.assertIn("observed class changed between the runs: 2 of 5 compared document(s)", out)
        self.assertIn("  passports/p-lt-as-ps.png: AA -> A<", out)
        self.assertIn("  passports/po-right.png: AA -> A<", out)
        self.assertNotIn("p-lt-right", out.split("observed class changed")[1])
        self.assertIn("header: identical", out)

    def test_two_runs_that_differ_in_an_arm_are_compared_and_the_arm_is_named(self):
        self.write_pair(ocr_arms={"texture": "off", "order": "default"})
        code, out, err = self.cli("codes", RUN_A[:8], RUN_B[:8])
        self.assertEqual(code, 0, err)
        self.assertIn("header: differs", out)
        self.assertIn("ocr_arms.texture: on -> off", out)

    def test_incomparable_runs_are_refused_naming_each_fact_that_differs(self):
        cases = {
            "binary_sha256 differs": dict(binary_sha256="0" * 64),
            "model_sha256 differs": dict(model_sha256={"detection": "0" * 64, "recognition": "9" * 64}),
            "scope differs": dict(scope={"corpus": "real-specimens", "count": 9}),
            "samples_data_sha differs": dict(samples_data_sha="s" * 40),
            "corpus_manifest_sha256 differs": dict(corpus_manifest_sha256="0" * 64),
            "providers differs": dict(providers=["mrz", "other"]),
            "tracks differs": dict(tracks={"private": False, "local": True, "covers": False}),
            "replay_of differs": dict(replay_of={"run_manifest": "x", "sha256": "0" * 64}),
        }
        for message, head in cases.items():
            with self.subTest(message):
                self.setUp()
                self.write_pair(**head)
                code, out, err = self.cli("codes", RUN_A[:8], RUN_B[:8])
                self.assertEqual((code, out), (2, ""), err)
                self.assertIn("the runs are not comparable", err)
                self.assertIn(message, err)

    def test_runs_that_do_not_hold_the_same_documents_are_refused(self):
        self.write_pair(lambda head, records: records.pop())
        code, out, err = self.cli("codes", RUN_A[:8], RUN_B[:8])
        self.assertEqual((code, out), (2, ""))
        self.assertIn("the document sets differ: 1 only in A, 0 only in B", err)

    def test_a_document_that_does_not_join_on_the_image_hash_is_refused(self):
        def rename(head, records):
            records[0]["source_sha256"] = None

        self.write_pair(rename)
        code, _out, err = self.cli("codes", RUN_A[:8], RUN_B[:8])
        self.assertEqual(code, 2)
        self.assertIn("do not join one to one on source_sha256", err)

    def test_a_live_run_without_model_hashes_is_refused(self):
        self.write_pair(lambda head, records: head.pop("model_sha256"))
        code, _out, err = self.cli("codes", RUN_A[:8], RUN_B[:8])
        self.assertEqual(code, 2)
        self.assertIn("model_sha256 is not recorded on run B", err)

    def test_the_comparable_facts_are_the_promotion_gates(self):
        import promotion_gate

        self.assertEqual(aq.COMPARABLE_FACTS, promotion_gate.EQUAL_FACTS)

    # ---- disclosure

    def test_no_character_of_a_zone_or_a_truth_is_printed(self):
        self.write_pair()
        for invocation in (("codes", RUN_A[:8]), ("codes", RUN_A[:8], RUN_B[:8])):
            code, out, err = self.cli(*invocation)
            self.assertEqual(code, 0, err)
            self.assertNotIn(TEXT_SENTINEL, out + err)
            self.assertNotIn("QX", out + err)
            self.assertNotIn(SENTINEL, out + err)
            self.assertNotIn("SENTINEL", out + err)
            # the reads are PS, P< and PO: their characters are not in the report
            for text in ("PS", "PO", "P<"):
                self.assertNotIn(text, out.replace("passports/", ""), invocation)

    def test_errors_exit_2_and_name_no_content(self):
        self.write_witness()
        for invocation in (("codes", "nosuch"), ("codes", RUN_A[:8], "nosuch")):
            code, out, err = self.cli(*invocation)
            self.assertEqual((code, out), (2, ""))
            self.assertNotIn(SENTINEL, err)


if __name__ == "__main__":
    unittest.main()
