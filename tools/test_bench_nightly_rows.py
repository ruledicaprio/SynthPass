"""Tests for bench_nightly_rows.py: a small synthetic report, no benchmark run.

Every string that stands for text in the fixture (a `reason`, a field's expected and read values,
OCR text, a pass's readings) is a sentinel, so a test can prove none of them reaches a row.
"""
import copy
import json
import tempfile
import unittest
from pathlib import Path

import bench_nightly_rows as n

# Text the report carries and no row may.
SENTINELS = ("REASON-TEXT", "EXPECTED-TEXT", "GOT-TEXT", "OCR-PAGE-TEXT", "READING-TEXT", "ZONE-TEXT")

RUN_ID = "123456789-1"
SHA = "a" * 40
HASH = "b" * 64


def context(**over):
    base = {
        "run_id": RUN_ID,
        "git_sha": SHA,
        "event": "schedule",
        "cpu_model": "Test CPU 3.0GHz",
        "nproc": 4,
        "rustc": "rustc 1.90.0 (0000000 2025-09-01)",
        "model_sha256": {"detection": "c" * 64, "recognition": "d" * 64},
        "ocr_env": {},
    }
    base.update(over)
    return base


def fields(**cer):
    """Thirteen fields; a nonzero CER carries expected and got text, as the report does."""
    out = []
    for name in n.FIELD_NAMES:
        value = cer.get(name, 0.0)
        entry = {"field": name, "cer": value}
        if value > 0:
            entry.update(expected="EXPECTED-TEXT", got="GOT-TEXT")
        out.append(entry)
    return out


def passes(*outcomes):
    return [
        {
            "order": i,
            "id": "general" if i == 0 else f"pass-{i:02d}",
            "transform": "mrz_variants:2",
            "turn": 0,
            "image_width": 10,
            "image_height": 10,
            "outcome": outcome,
            "readings": [{"line_index": 0, "bbox": {"x": 0, "y": 0, "w": 1, "h": 1}, "text": "READING-TEXT"}],
            "chargrid": None,
        }
        for i, outcome in enumerate(outcomes)
    ]


def result(seed, profile, hit, *, miss_kind=None, check_states="ok", field_list=None, name_error=None,
           retry_stop="general_valid", **extra):
    states = {"document_number": True, "date_of_birth": True, "date_of_expiry": True,
              "personal_number": None, "composite": True}
    out = {
        "seed": seed,
        "profile": profile,
        "render_sha256": f"{seed:064x}",
        "hit": hit,
        "elapsed_ms": 1500,
        "retry_variant_id": None,
        "retry_damaged_recovery": None,
        "tier1_damaged_recovery": False,
        "line1_flagged": False,
        "fields": fields() if field_list is None else field_list,
        "ocr_text": "OCR-PAGE-TEXT",
        "ocr_passes": passes("accepted"),
    }
    if miss_kind:
        out["miss_kind"] = miss_kind
        out["reason"] = "REASON-TEXT"
    if check_states == "ok":
        out["check_states"] = states
    elif check_states:
        out["check_states"] = check_states
    if name_error:
        out["name_error"] = name_error
    if retry_stop:
        out["retry_stop"] = retry_stop
    out.update(extra)
    return out


def fresh_report():
    """Seven documents, chosen so every derived flag is exercised. Counts are written out by hand."""
    results = [
        # 100: an exact hit.
        result(100, "clean", True),
        # 101: a hit that reads the surname wrong (wrong accept, names not exact).
        result(101, "mobile", True, field_list=fields(surname=0.2), name_error="other"),
        # 102: a hit that reads document_type wrong (prefix wrong accept).
        result(102, "scanner", True, field_list=fields(document_type=1.0)),
        # 103: checksum-valid, wrong document number and issuer (accepted read, prefix wrong).
        result(103, "worn", False, miss_kind="document_number_mismatch",
               field_list=fields(document_number=0.1, issuing_country=0.3)),
        # 104: a checksum failure that hit the time budget.
        result(104, "border-kiosk", False, miss_kind="checksum_failed",
               check_states={"document_number": False, "date_of_birth": True, "date_of_expiry": True,
                             "personal_number": None, "composite": False},
               field_list=fields(surname=0.5), name_error="other", retry_stop="budget"),
        # 105: no MRZ found: total loss, no check states.
        result(105, "clean", False, miss_kind="no_mrz_found", check_states=None,
               field_list=[{"field": name, "cer": 1.0, "expected": "EXPECTED-TEXT"} for name in n.FIELD_NAMES],
               retry_stop="exhausted"),
        # 106: OCR failed: no fields, no check states, no retry stop, no passes.
        result(106, "mobile", False, miss_kind="ocr_error", check_states=None, field_list=[], retry_stop=None,
               ocr_text=None, ocr_passes=[]),
    ]
    return {
        "timestamp_unix": 1790000000,
        "profile": "all",
        "document_type": "TD3",
        "count": 7,
        "seed_start": 100,
        "ocr_arms": {"texture": "on", "order": "default", "rotate": "default", "skew": "default",
                     "chargrid": "off"},
        "mrz_class_sweep_arm": "off",
        "mrz_line1_select_arm": "off",
        "model_paths": {"detection": "/x/text-detection.rten", "recognition": "/x/text-recognition.rten"},
        "hits": 3,
        "hit_rate": 3 / 7,
        "strict_hits": 2,
        "wrong_accepts": 2,
        "prefix_wrong_accepts": 1,
        "prefix_wrong_accept_seeds": [102],
        "accepted_reads": 4,
        "prefix_wrong_accepted_reads": 2,
        "prefix_wrong_accepted_read_seeds": [102, 103],
        "results": results,
    }


def fixed_report(document_type="TD3"):
    results = [result(seed, "clean", True) for seed in range(100)]
    return {
        "timestamp_unix": 1790000100,
        "profile": "clean",
        "document_type": document_type,
        "count": 100,
        "seed_start": 0,
        "ocr_arms": {"texture": "on", "order": "default", "rotate": "default", "skew": "default",
                     "chargrid": "off"},
        "mrz_class_sweep_arm": "off",
        "mrz_line1_select_arm": "off",
        "hits": 100,
        "strict_hits": 100,
        "wrong_accepts": 0,
        "prefix_wrong_accepts": 0,
        "accepted_reads": 100,
        "prefix_wrong_accepted_reads": 0,
        "results": results,
    }


def write_measurement(root: Path, name: str, fresh, fixed, ctx=None):
    directory = root / name
    directory.mkdir(parents=True)
    (directory / "fresh.json").write_text(json.dumps(fresh))
    (directory / "fixed.json").write_text(json.dumps(fixed))
    (directory / "context.json").write_text(json.dumps(ctx or context()))


class RowShapeTests(unittest.TestCase):
    def rows(self):
        rows, _ = n.extract_run(fresh_report(), "fresh", context())
        return rows

    def test_a_row_holds_exactly_the_allowlisted_keys(self):
        for row in self.rows():
            self.assertEqual(tuple(row), n.ROW_KEYS)
        # The plan's list, plus run_id (the join key to runs.jsonl).
        self.assertEqual(
            set(n.ROW_KEYS) - {"run_id"},
            {"schema", "slice", "document_type", "seed", "profile", "render_sha256", "hit", "miss_kind",
             "check_states", "field_cer", "name_error", "line1_flagged", "retry_stop", "retry_variant_id",
             "retry_damaged_recovery", "tier1_damaged_recovery", "ocr_pass_count", "ocr_passes",
             "elapsed_ms"},
        )

    def test_no_text_from_the_report_reaches_a_row_or_a_header(self):
        rows, header = n.extract_run(fresh_report(), "fresh", context())
        published = json.dumps(rows) + json.dumps(header)
        for sentinel in SENTINELS:
            self.assertNotIn(sentinel, published)
        for banned in ("reason", "expected", "got", "ocr_text", "readings", "wrong_fields", "names_exact"):
            self.assertNotIn(f'"{banned}"', published)

    def test_values_are_projected_from_the_report(self):
        by_seed = {row["seed"]: row for row in self.rows()}
        exact = by_seed[100]
        self.assertEqual((exact["schema"], exact["slice"], exact["document_type"]), (2, "fresh", "td3"))
        self.assertEqual((exact["profile"], exact["hit"], exact["miss_kind"]), ("clean", True, None))
        self.assertEqual(exact["render_sha256"], f"{100:064x}")
        self.assertEqual(exact["field_cer"]["surname"], 0.0)
        self.assertEqual(len(exact["field_cer"]), 13)
        self.assertEqual((exact["ocr_pass_count"], exact["ocr_passes"]), (1, [{"id": "general", "outcome": "accepted"}]))
        self.assertEqual(by_seed[104]["retry_stop"], "budget")
        self.assertEqual(by_seed[104]["name_error"], "other")
        # Absent in the report reads as null, not as false or zero.
        self.assertIsNone(by_seed[105]["check_states"])
        self.assertIsNone(by_seed[106]["field_cer"])
        self.assertIsNone(by_seed[106]["retry_stop"])
        self.assertEqual((by_seed[106]["ocr_pass_count"], by_seed[106]["ocr_passes"]), (0, []))

    def test_a_report_without_pass_traces_gives_null_pass_keys(self):
        report = fresh_report()
        for r in report["results"]:
            del r["ocr_passes"]
        rows, header = n.extract_run(report, "fresh", context())
        self.assertTrue(all(r["ocr_pass_count"] is None and r["ocr_passes"] is None for r in rows))
        self.assertFalse(header["invocation"]["ocr_passes"])


class AllowlistRefusalTests(unittest.TestCase):
    def row(self):
        return copy.deepcopy(n.extract_run(fresh_report(), "fresh", context())[0][0])

    def test_any_key_outside_the_allowlist_is_refused(self):
        for key in ("reason", "fields", "ocr_text", "wrong_accept", "names_exact", "wrong_fields",
                    "expected", "readings", "anything_new"):
            row = self.row()
            row[key] = "x"
            with self.assertRaises(n.RowError, msg=key) as raised:
                n.validate_row(row)
            self.assertIn(key, str(raised.exception))

    def test_a_missing_key_is_refused(self):
        row = self.row()
        del row["hit"]
        with self.assertRaises(n.RowError):
            n.validate_row(row)

    def test_nested_keys_are_allowlisted_too(self):
        cases = [
            ("ocr_passes", [{"id": "general", "outcome": "accepted", "readings": []}]),
            ("ocr_passes", [{"id": "general"}]),
            ("check_states", {"document_number": True, "surprise": True}),
            ("field_cer", {"surname": 0.0, "nickname": 0.0}),
        ]
        for key, value in cases:
            row = self.row()
            row[key] = value
            if key == "ocr_passes":
                row["ocr_pass_count"] = len(value)
            with self.assertRaises(n.RowError, msg=key):
                n.validate_row(row)

    def test_text_cannot_travel_in_an_allowed_key(self):
        cases = [
            ("retry_variant_id", "P<UTOERIKSSON<<ANNA"),
            ("name_error", "surname was ERIKSSON"),
            ("miss_kind", "checksum invalid: personal_number"),
            ("retry_stop", "budget exceeded"),
            ("render_sha256", "not-a-hash"),
            ("profile", "clean copy"),
            ("hit", "true"),
            ("seed", -1),
            ("elapsed_ms", "1500"),
        ]
        for key, value in cases:
            row = self.row()
            row[key] = value
            with self.assertRaises(n.RowError, msg=key):
                n.validate_row(row)

    def test_a_pass_id_or_outcome_with_text_is_refused(self):
        row = self.row()
        row["ocr_passes"] = [{"id": "P<UTO", "outcome": "accepted"}]
        row["ocr_pass_count"] = 1
        with self.assertRaises(n.RowError):
            n.validate_row(row)

    def test_hit_and_miss_kind_must_agree(self):
        row = self.row()
        row["hit"] = False
        with self.assertRaises(n.RowError):
            n.validate_row(row)

    def test_the_projection_never_reads_a_key_it_was_not_given(self):
        report = fresh_report()
        report["results"][0]["a_future_text_key"] = "ZONE-TEXT"
        rows, _ = n.extract_run(report, "fresh", context())
        self.assertNotIn("ZONE-TEXT", json.dumps(rows))

    def test_a_file_that_holds_a_forbidden_key_cannot_be_read_back(self):
        row = self.row()
        row["reason"] = "REASON-TEXT"
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "fresh.jsonl"
            path.write_text(json.dumps(row) + "\n")
            with self.assertRaises(n.RowError):
                n.read_rows(path)


class DerivedFlagTests(unittest.TestCase):
    def setUp(self):
        rows, _ = n.extract_run(fresh_report(), "fresh", context())
        self.by_seed = {row["seed"]: n.derive(row) for row in rows}

    def test_flags_are_recomputed_from_the_row(self):
        d = self.by_seed
        self.assertEqual(d[100]["wrong_accept"], False)
        self.assertEqual(d[100]["names_exact"], True)
        self.assertEqual(d[100]["strict_hit"], True)
        # 101: wrong surname on a hit.
        self.assertEqual(d[101]["wrong_fields"], ["surname"])
        self.assertEqual((d[101]["wrong_accept"], d[101]["names_exact"], d[101]["strict_hit"]), (True, False, False))
        self.assertEqual(d[101]["prefix_wrong_accept"], False)
        # 102: the prefix.
        self.assertEqual((d[102]["wrong_accept"], d[102]["prefix_wrong_accept"]), (True, True))
        self.assertEqual(d[102]["strict_hit"], True)
        # 103: accepted, not a hit, so wrong_accept is false and the accepted-read prefix flag is true.
        self.assertEqual(d[103]["accepted_read"], True)
        self.assertEqual((d[103]["wrong_accept"], d[103]["prefix_wrong_accept"]), (False, False))
        self.assertEqual(d[103]["prefix_wrong_accepted_read"], True)
        self.assertEqual(d[103]["names_exact"], True)
        self.assertEqual(d[103]["strict_hit"], False)
        # 104 and 105: refused, not accepted.
        self.assertEqual(d[104]["accepted_read"], False)
        self.assertEqual(d[104]["names_exact"], False)
        self.assertEqual(d[105]["accepted_read"], False)
        # 105: no MRZ parsed, so names are not exact even with no name error.
        self.assertEqual(d[105]["names_exact"], False)

    def test_mrz_lines_is_diagnostic_and_never_a_wrong_field(self):
        row = n.extract_run(fresh_report(), "fresh", context())[0][0]
        row = copy.deepcopy(row)
        row["field_cer"]["mrz_lines"] = 0.4
        self.assertEqual(n.derive(row)["wrong_fields"], [])
        self.assertEqual(n.derive(row)["wrong_accept"], False)

    def test_totals_equal_the_reports_counts(self):
        rows, header = n.extract_run(fresh_report(), "fresh", context())
        t = n.totals(rows)
        self.assertEqual(
            (t["count"], t["hits"], t["strict_hits"], t["wrong_accepts"], t["accepted_reads"],
             t["prefix_wrong_accepts"], t["prefix_wrong_accepted_reads"]),
            (7, 3, 2, 2, 4, 1, 2),
        )
        self.assertEqual(t["prefix_wrong_accept_seeds"], [102])
        self.assertEqual(t["prefix_wrong_accepted_read_seeds"], [102, 103])
        self.assertEqual(header["counts"]["budget_stops"], 1)
        self.assertEqual(header["counts"]["hits"], 3)

    def test_a_count_that_disagrees_with_the_rows_fails_the_run(self):
        for key, bad in (("hits", 4), ("strict_hits", 3), ("wrong_accepts", 1), ("accepted_reads", 5),
                         ("prefix_wrong_accepts", 0), ("prefix_wrong_accepted_reads", 3), ("count", 8)):
            report = fresh_report()
            report[key] = bad
            with self.assertRaises(n.RowError, msg=key) as raised:
                n.extract_run(report, "fresh", context())
            self.assertIn(key, str(raised.exception))

    def test_a_seed_list_that_disagrees_fails_the_run(self):
        report = fresh_report()
        report["prefix_wrong_accepted_read_seeds"] = [102]
        with self.assertRaises(n.RowError):
            n.extract_run(report, "fresh", context())

    def test_a_row_the_projection_got_wrong_fails_the_run(self):
        report = fresh_report()
        # The binary says seed 100 is exact; the row would say otherwise if a cer were altered.
        report["results"][0]["fields"] = fields(document_type=1.0)
        with self.assertRaises(n.RowError):
            n.extract_run(report, "fresh", context())

    def test_an_empty_seed_list_is_omitted_by_the_report(self):
        report = fixed_report()
        rows, _ = n.extract_run(report, "fixed", context())
        self.assertEqual(n.totals(rows)["prefix_wrong_accept_seeds"], [])


class DuplicateGuardTests(unittest.TestCase):
    def rows(self):
        return n.extract_run(fresh_report(), "fresh", context())[0]

    def test_unique_rows_pass(self):
        self.assertEqual(n.find_duplicates(self.rows()), [])

    def test_a_repeated_document_type_seed_and_profile_is_found(self):
        rows = self.rows()
        rows.append(copy.deepcopy(rows[0]))
        self.assertEqual(n.find_duplicates(rows), [("td3", 100, "clean")])

    def test_the_same_seed_in_another_format_or_profile_is_not_a_duplicate(self):
        rows = self.rows()
        other_format = copy.deepcopy(rows[0])
        other_format["document_type"] = "td1"
        other_profile = copy.deepcopy(rows[0])
        other_profile["profile"] = "worn"
        self.assertEqual(n.find_duplicates(rows + [other_format, other_profile]), [])

    def test_the_guard_command_refuses_a_file_with_a_repeat(self):
        rows = self.rows()
        rows.append(copy.deepcopy(rows[0]))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "fresh.jsonl"
            path.write_text("".join(json.dumps(r) + "\n" for r in rows))
            self.assertEqual(n.main(["guard", "--file", str(path)]), 1)
            path.write_text("".join(json.dumps(r) + "\n" for r in rows[:-1]))
            self.assertEqual(n.main(["guard", "--file", str(path)]), 0)

    def test_assemble_does_not_guard_the_fixed_slice(self):
        # Every night repeats seeds 0-99 by design: two nights of fixed rows are not duplicates.
        rows, _ = n.extract_run(fixed_report(), "fixed", context())
        self.assertEqual(len(rows), 100)
        # find_duplicates would flag two nights; assemble never applies it to fixed rows.
        self.assertNotEqual(n.find_duplicates(rows + rows), [])
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_measurement(root / "in", "td3", fresh_report(), fixed_report())
            counts = n.assemble(root / "in", root / "out")
        self.assertEqual((counts["fresh"], counts["fixed"], counts["runs"]), (7, 100, 2))


class SchemaOneTests(unittest.TestCase):
    V1 = [
        {"run_timestamp_unix": 1, "git_sha": "abc", "seed": 4100000, "profile": "clean", "hit": True,
         "reason": None, "elapsed_ms": 900},
        {"run_timestamp_unix": 1, "git_sha": "abc", "seed": 4100001, "profile": "mobile", "hit": False,
         "reason": "checksum invalid: personal_number, composite", "elapsed_ms": 1000},
        {"run_timestamp_unix": 1, "git_sha": "abc", "seed": 4100002, "profile": "worn", "hit": False,
         "reason": "document number mismatch: SECRET-NUMBER", "elapsed_ms": 1100},
        {"run_timestamp_unix": 1, "git_sha": "abc", "seed": 4100003, "profile": "worn", "hit": False,
         "reason": "no MRZ found", "elapsed_ms": 1200},
        # The 2026-07-21 epoch: a Debug struct, no known prefix.
        {"run_timestamp_unix": 1, "git_sha": "abc", "seed": 4100004, "profile": "worn", "hit": False,
         "reason": "ChecksumFailed { .. }", "elapsed_ms": 1300},
    ]

    def read(self, rows):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "dataset.jsonl"
            path.write_text("".join(json.dumps(r) + "\n" for r in rows))
            return n.read_rows(path)

    def test_a_row_with_no_schema_key_is_schema_one(self):
        self.assertEqual({row["schema"] for row in self.read(self.V1)}, {1})

    def test_reason_maps_to_a_miss_kind_by_prefix_and_is_dropped(self):
        rows = self.read(self.V1)
        self.assertEqual(
            [row["miss_kind"] for row in rows],
            [None, "checksum_failed", "document_number_mismatch", "no_mrz_found", None],
        )
        self.assertNotIn("reason", rows[0])
        self.assertNotIn("SECRET-NUMBER", json.dumps(rows))

    def test_every_other_key_reads_as_absent_never_false_or_zero(self):
        for row in self.read(self.V1):
            # Every schema-2 key, plus the two a schema-1 row carried that schema 2 moved to runs.jsonl.
            self.assertEqual(tuple(row), n.ROW_KEYS + ("run_timestamp_unix", "git_sha"))
            for key in ("run_id", "slice", "document_type", "render_sha256", "check_states", "field_cer",
                        "name_error", "line1_flagged", "retry_stop", "retry_variant_id",
                        "retry_damaged_recovery", "tier1_damaged_recovery", "ocr_pass_count", "ocr_passes"):
                self.assertIsNone(row[key], key)
        first = self.read(self.V1)[0]
        self.assertEqual((first["seed"], first["profile"], first["hit"], first["elapsed_ms"]),
                         (4100000, "clean", True, 900))

    def test_derived_flags_of_a_schema_one_row_are_unknown_not_false(self):
        hit, checksum, mismatch, _, debug = (n.derive(row) for row in self.read(self.V1))
        self.assertIsNone(hit["wrong_accept"])
        self.assertIsNone(hit["names_exact"])
        self.assertIsNone(hit["strict_hit"])
        # Knowable from the miss kind alone.
        self.assertEqual((hit["accepted_read"], checksum["accepted_read"], mismatch["accepted_read"]),
                         (True, False, True))
        # A non-hit with no known reason prefix is unknown, and a non-hit is never a wrong accept.
        self.assertIsNone(debug["accepted_read"])
        self.assertEqual(checksum["wrong_accept"], False)

    def test_a_schema_one_row_with_another_key_is_refused(self):
        with self.assertRaises(n.RowError):
            self.read([dict(self.V1[0], ocr_text="OCR-PAGE-TEXT")])

    def test_an_unknown_schema_is_refused(self):
        with self.assertRaises(n.RowError):
            self.read([{"schema": 3, "seed": 1}])

    def test_schema_two_rows_round_trip(self):
        rows, _ = n.extract_run(fresh_report(), "fresh", context())
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "fresh.jsonl"
            path.write_text("".join(json.dumps(r) + "\n" for r in rows))
            self.assertEqual(n.read_rows(path), rows)


class FixedSliceAndHeaderTests(unittest.TestCase):
    def test_the_fixed_slice_must_be_the_headline_invocation(self):
        for key, bad in (("profile", "all"), ("seed_start", 7), ("count", 50)):
            report = fixed_report()
            report[key] = bad
            with self.assertRaises(n.RowError, msg=key):
                n.extract_run(report, "fixed", context())

    def test_the_fresh_slice_stays_above_the_fixed_seeds(self):
        report = fresh_report()
        report["seed_start"] = 50
        with self.assertRaises(n.RowError):
            n.extract_run(report, "fresh", context())
        report = fresh_report()
        report["profile"] = "clean"
        with self.assertRaises(n.RowError):
            n.extract_run(report, "fresh", context())

    def test_the_generator_fingerprint_follows_the_rendered_pixels(self):
        rows, header = n.extract_run(fixed_report(), "fixed", context())
        again, _ = n.extract_run(fixed_report(), "fixed", context())
        self.assertEqual(n.generator_fingerprint(rows), n.generator_fingerprint(again))
        self.assertEqual(header["generator_fingerprint"], n.generator_fingerprint(rows))
        # Row order does not matter; one re-rendered seed does.
        self.assertEqual(n.generator_fingerprint(rows), n.generator_fingerprint(list(reversed(rows))))
        changed = copy.deepcopy(fixed_report())
        changed["results"][40]["render_sha256"] = HASH
        rows_changed, _ = n.extract_run(changed, "fixed", context())
        self.assertNotEqual(n.generator_fingerprint(rows), n.generator_fingerprint(rows_changed))

    def test_only_the_fixed_run_carries_a_fingerprint(self):
        self.assertIsNone(n.extract_run(fresh_report(), "fresh", context())[1]["generator_fingerprint"])
        self.assertIsNotNone(n.extract_run(fixed_report(), "fixed", context())[1]["generator_fingerprint"])

    def test_the_header_records_the_conditions_of_the_run(self):
        _, header = n.extract_run(fresh_report(), "fresh", context())
        self.assertEqual(header["run_id"], RUN_ID)
        self.assertEqual((header["git_sha"], header["event"]), (SHA, "schedule"))
        self.assertEqual(header["invocation"], {"document_type": "td3", "profile": "all", "count": 7,
                                                "seed_start": 100, "ocr_passes": True})
        self.assertEqual(header["ocr_arms"]["chargrid"], "off")
        self.assertEqual((header["nproc"], header["cpu_model"]), (4, "Test CPU 3.0GHz"))
        self.assertEqual(header["model_sha256"]["detection"], "c" * 64)
        self.assertEqual(header["counts"]["wrong_accepts"], 2)
        # No override set: the binary's own defaults, recorded as null.
        self.assertEqual((header["max_passes"], header["max_seconds"], header["ocr_env"]), (None, None, {}))

    def test_an_overridden_budget_is_recorded(self):
        ctx = context(ocr_env={"SYNTHPASS_OCR_MAX_SECONDS": "3600", "SYNTHPASS_OCR_MAX_PASSES": "14"})
        _, header = n.extract_run(fresh_report(), "fresh", ctx)
        self.assertEqual((header["max_passes"], header["max_seconds"]), (14, 3600))

    def test_a_context_with_an_extra_key_or_a_long_value_is_refused(self):
        with self.assertRaises(n.RowError):
            n.validate_context(dict(context(), hostname="runner-1"))
        with self.assertRaises(n.RowError):
            n.validate_context(context(cpu_model="x" * 500))
        with self.assertRaises(n.RowError):
            n.validate_context(context(ocr_env={"PATH": "/usr/bin"}))
        with self.assertRaises(n.RowError):
            n.validate_context(context(git_sha="main"))


class AssembleAndSeedTests(unittest.TestCase):
    def test_assemble_writes_the_three_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_measurement(root / "in", "td3", fresh_report(), fixed_report())
            counts = n.assemble(root / "in", root / "out")
            self.assertEqual(counts["formats"], ["td3"])
            fresh = n.read_rows(root / "out" / "fresh.jsonl")
            fixed = n.read_rows(root / "out" / "fixed.jsonl")
            runs = [json.loads(line) for line in (root / "out" / "runs.jsonl").read_text().splitlines()]
            self.assertEqual((len(fresh), len(fixed)), (7, 100))
            self.assertEqual([r["slice"] for r in runs], ["fresh", "fixed"])

    def test_assemble_writes_nothing_when_a_count_mismatches(self):
        bad = fresh_report()
        bad["hits"] = 6
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_measurement(root / "in", "td3", bad, fixed_report())
            with self.assertRaises(n.RowError):
                n.assemble(root / "in", root / "out")
            self.assertFalse((root / "out").exists())

    def test_two_measurements_of_one_format_are_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            write_measurement(root / "in", "a", fresh_report(), fixed_report())
            write_measurement(root / "in", "b", fresh_report(), fixed_report())
            with self.assertRaises(n.RowError):
                n.assemble(root / "in", root / "out")

    def test_assemble_without_a_measurement_fails(self):
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(n.RowError):
                n.assemble(Path(tmp), Path(tmp) / "out")

    def test_next_seed(self):
        with tempfile.TemporaryDirectory() as tmp:
            data = Path(tmp)
            # Nothing on the branch: above the fixed slice.
            self.assertEqual(n.next_seed(data, "td1"), 100)
            # The frozen dataset raises the floor for every format.
            (data / "dataset.jsonl").write_text(
                json.dumps({"run_timestamp_unix": 1, "git_sha": "a", "seed": 4100199, "profile": "clean",
                            "hit": True, "reason": None, "elapsed_ms": 1}) + "\n"
            )
            self.assertEqual(n.next_seed(data, "td1"), 4100200)
            # A format continues from its own highest fresh seed.
            rows, _ = n.extract_run(fresh_report(), "fresh", context())
            for row in rows:
                row["seed"] += 5_000_000
            (data / "fresh.jsonl").write_text("".join(json.dumps(r) + "\n" for r in rows))
            self.assertEqual(n.next_seed(data, "td3"), 5_000_107)
            self.assertEqual(n.next_seed(data, "td2"), 4100200)
            with self.assertRaises(n.RowError):
                n.next_seed(data, "td4")


if __name__ == "__main__":
    unittest.main()
