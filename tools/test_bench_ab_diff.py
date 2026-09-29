"""Tests for bench_ab_diff.py: literal arm directories in a temporary folder, no benchmark run."""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path

import bench_ab_diff as d


def field(name, cer=0.0, expected=None, got=None):
    f = {"field": name, "cer": cer}
    if cer > 0:
        f.update(expected=expected, got=got)
    return f


def doc(seed, hit, miss_kind=None, fields=(), retry_stop="exhausted", elapsed_ms=1000, **extra):
    out = {"seed": seed, "hit": hit, "miss_kind": miss_kind, "fields": list(fields),
           "retry_stop": retry_stop, "elapsed_ms": elapsed_ms}
    out.update(extra)
    return out


def report(*docs, **meta):
    base = {"profile": "clean", "document_type": "TD1", "seed_start": 0, "count": len(docs),
            "strict_hits": sum(1 for x in docs if x["hit"])}
    base.update(meta)
    base["results"] = list(docs)
    return base


ZONE_TRUTH = "I<USAK2HPBKQ2L<<<<<<<<<<<<<<<\n9001012M3001014USA<<<<<<<<<<<4\nDOE<<JOHN<<<<<<<<<<<<<<<<<<<<<"
SHIFTED = "IUSAK2HPBKQ210<<<<<<<<<<<<<<<<\n9001012M3001014USA<<<<<<<<<<<4\nDOE<<JOHN<<<<<<<<<<<<<<<<<<<<<"
PREFIX_WRONG = [field("document_type", 1.0, "I", "IU"), field("issuing_country", 0.67, "USA", "SAK"),
                field("document_number", 0.2, "K2HPBKQ2L", "2HPBKQ210"),
                field("mrz_lines", 0.05, ZONE_TRUTH, SHIFTED)]


def write_arm(root, name, reports=None, outcomes=None, zones=None):
    arm = Path(root) / name
    arm.mkdir()
    for file_name, rep in (reports or {}).items():
        (arm / file_name).write_text(json.dumps(rep), encoding="utf-8")
    if outcomes is not None:
        (arm / "real").mkdir()
        (arm / "real" / d.OUTCOMES).write_text(
            "".join(json.dumps(r) + "\n" for r in outcomes), encoding="utf-8")
        if zones is not None:
            (arm / "real" / d.ZONES).write_text(
                "".join(json.dumps(r) + "\n" for r in zones), encoding="utf-8")
    return arm


def outcome_row(asset, outcome, fmt="TD3", reason=None):
    return {"asset_id": asset, "outcome": outcome, "mrz_format": fmt, "miss_reason": reason}


def zone_row(asset, lines, fixture=None):
    return {"asset_id": asset, "recovered_mrz_lines": lines, "ground_truth_mrz": fixture,
            "raw_ocr_text": "RAW OCR TEXT"}


def run(before, after, *args):
    out = io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(io.StringIO()):
        code = d.main([str(before), str(after), *args])
    return code, out.getvalue()


class AcceptedReads(unittest.TestCase):
    def test_hits_and_document_number_mismatches_are_accepted(self):
        self.assertTrue(d.is_accepted_read(doc(0, True)))
        self.assertTrue(d.is_accepted_read(doc(0, False, "document_number_mismatch")))
        self.assertFalse(d.is_accepted_read(doc(0, False, "checksum_failed")))

    def test_a_wrong_prefix_counts_only_on_an_accepted_read(self):
        self.assertTrue(d.is_prefix_wrong(doc(52, False, "document_number_mismatch", PREFIX_WRONG)))
        self.assertFalse(d.is_prefix_wrong(doc(52, False, "checksum_failed", PREFIX_WRONG)))
        self.assertFalse(d.is_prefix_wrong(doc(52, True, fields=[field("surname", 0.5, "DOE", "D0E")])))


class Synthetic(unittest.TestCase):
    def compare(self, before_docs, after_docs):
        return d.compare_synthetic(report(*before_docs), report(*after_docs))

    def test_timing_and_retry_telemetry_alone_are_not_a_change(self):
        r = self.compare([doc(0, True)], [doc(0, True, retry_stop="budget", elapsed_ms=9000)])
        self.assertEqual(r["changed"], [])

    def test_a_read_that_changes_without_moving_state_is_listed(self):
        # PR 6's 17 TD1 seeds: still checksum-failed, but line 1 is no longer shifted.
        before = doc(2, False, "checksum_failed", PREFIX_WRONG)
        after = doc(2, False, "checksum_failed", [field("document_number", 0.1, "K2HPBKQ2L", "K2HPBKQ21"),
                                                  field("mrz_lines")])
        (c,) = self.compare([before], [after])["changed"]
        self.assertEqual(c["class"], "refused -> refused")
        self.assertEqual(c["outcome"], ["checksum_failed", "checksum_failed"])
        self.assertEqual({f["field"] for f in c["changes"]},
                         {"document_type", "issuing_country", "document_number"})
        self.assertEqual(c["zone"], {"before": SHIFTED, "after": "=truth", "truth": ZONE_TRUTH})

    def test_counts_and_prefix_seeds_per_arm(self):
        before = [doc(52, False, "document_number_mismatch", PREFIX_WRONG), doc(1, True)]
        after = [doc(52, False, "checksum_failed", [field("document_number", 0.1, "0WZ", "OWZ")]),
                 doc(1, True)]
        r = self.compare(before, after)
        self.assertEqual(r["before"]["accepted_reads"], 2)
        self.assertEqual(r["after"]["accepted_reads"], 1)
        self.assertEqual(r["before"]["prefix_wrong_accepted_read_seeds"], [52])
        self.assertEqual(r["after"]["prefix_wrong_accepted_read_seeds"], [])
        self.assertEqual([c["seed"] for c in r["changed"]], [52])
        self.assertEqual(r["changed"][0]["outcome"], ["document_number_mismatch", "checksum_failed"])

    def test_retry_stop_is_reported_for_a_changed_seed(self):
        r = self.compare([doc(94, False, "no_mrz_found", retry_stop="budget")],
                         [doc(94, False, "checksum_failed", retry_stop="exhausted")])
        self.assertEqual(r["changed"][0]["retry_stop"], ["budget", "exhausted"])

    def test_different_seeds_are_not_an_ab(self):
        r = d.compare_synthetic(report(doc(0, True)), report(doc(0, True), seed_start=5))
        self.assertTrue(r["problems"])


class Fixture(unittest.TestCase):
    def test_verdicts(self):
        self.assertIsNone(d.fixture_verdict(["A", "B"], None))
        self.assertEqual(d.fixture_verdict(["A", "B"], "A\nB"), "exact")
        self.assertEqual(d.fixture_verdict(["A", "X"], "A\nB"), "=x")
        self.assertEqual(d.fixture_verdict(["A"], "A\nB"), "=x")
        self.assertEqual(d.fixture_verdict(None, ["A", "B"]), "xx")


class Arms(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = self.tmp.name

    def test_real_outcome_and_zone_changes(self):
        kosovo, sweden, belgium = "passports/Kosovo_2023.jpg", "misc/Sweden_Visa_2024.jpg", "id_cards/Belgium.jpg"
        before = write_arm(self.root, "before", outcomes=[
            outcome_row(kosovo, "hit"), outcome_row(sweden, "hit", "MRVB"), outcome_row(belgium, "checksum_failed", "TD1"),
        ], zones=[zone_row(kosovo, ["POOOO", "L2"], fixture="P<RKS\nL2"), zone_row(sweden, ["VIS", "L2"]),
                  zone_row(belgium, ["A", "B", "C"])])
        after = write_arm(self.root, "after", outcomes=[
            outcome_row(kosovo, "hit"), outcome_row(sweden, "hit", "MRVB"), outcome_row(belgium, "hit", "TD1"),
        ], zones=[zone_row(kosovo, ["P<RKS", "L2"], fixture="P<RKS\nL2"), zone_row(sweden, ["VIS", "L2"]),
                  zone_row(belgium, ["A", "B", "C"])])
        r = d.compare_arms(before, after, [sweden])["real"]
        self.assertEqual(r["problems"], [])
        self.assertEqual([c["asset"] for c in r["outcome_changes"]], [belgium])
        (z,) = r["zone_changes"]
        self.assertEqual(z["asset"], kosovo)
        self.assertEqual(z["fixture"], ["x=", "exact"])
        self.assertEqual(r["counts"][0], {"checksum_failed": 1, "hit": 2})
        self.assertEqual(r["hits_by_format"][1], {"MRVB": 1, "TD1": 1, "TD3": 1})
        (w,) = r["watched"]
        self.assertTrue(w["known"])
        self.assertEqual(w["before"], w["after"])

    def test_raw_ocr_text_is_never_printed(self):
        asset = "passports/X.jpg"
        before = write_arm(self.root, "before", outcomes=[outcome_row(asset, "hit")],
                           zones=[zone_row(asset, ["A", "B"])])
        after = write_arm(self.root, "after", outcomes=[outcome_row(asset, "checksum_failed")],
                          zones=[zone_row(asset, ["A", "C"])])
        code, out = run(before, after, "--asset", asset)
        self.assertEqual(code, 0)
        self.assertIn("A / C", out)
        self.assertNotIn("RAW OCR TEXT", out)

    def test_different_asset_sets_are_not_an_ab(self):
        before = write_arm(self.root, "before", outcomes=[outcome_row("a", "hit"), outcome_row("b", "hit")])
        after = write_arm(self.root, "after", outcomes=[outcome_row("a", "hit")])
        code, out = run(before, after)
        self.assertEqual(code, 1)
        self.assertIn("NOT AN A/B: real: asset sets differ", out)

    def test_zones_need_a_dump_in_both_arms(self):
        before = write_arm(self.root, "before", outcomes=[outcome_row("a", "hit")], zones=[zone_row("a", ["A"])])
        after = write_arm(self.root, "after", outcomes=[outcome_row("a", "hit")])
        code, out = run(before, after)
        self.assertEqual(code, 0)
        self.assertIn("zones not compared", out)

    def test_reports_pair_by_file_name_and_formats_sort_in_note_order(self):
        rep = report(doc(0, True))
        before = write_arm(self.root, "before", reports={"mrvb.json": rep, "td1.json": rep, "td3.json": rep,
                                                        "notes.json": {"not": "a report"}})
        after = write_arm(self.root, "after", reports={"mrvb.json": rep, "td1.json": rep})
        result = d.compare_arms(before, after, [])
        self.assertEqual(list(result["synthetic"]), ["td1.json", "mrvb.json"])
        self.assertEqual(result["synthetic_in_one_arm_only"], {"before": ["td3.json"], "after": []})
        self.assertIsNone(result["real"])

    def test_nothing_in_common_is_an_error(self):
        before = write_arm(self.root, "before", reports={"td1.json": report(doc(0, True))})
        after = write_arm(self.root, "after")
        self.assertEqual(run(before, after)[0], 2)
        self.assertEqual(run(Path(self.root) / "missing", after)[0], 2)

    def test_json_output_round_trips(self):
        rep = report(doc(0, True))
        before = write_arm(self.root, "before", reports={"td1.json": rep})
        after = write_arm(self.root, "after", reports={"td1.json": rep})
        code, out = run(before, after, "--json")
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out)["synthetic"]["td1.json"]["changed"], [])


if __name__ == "__main__":
    unittest.main()
