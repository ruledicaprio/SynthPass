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
ARCHIVE = "provider-bench-ocr-run-0000.json"


def archive(flags=("--real-specimens", "--mrz-only", "--dump-ocr"), commit="a" * 40, corpus="c" * 64,
            ocr_arms=None):
    return {"flags": list(flags), "git_commit": commit, "working_tree_dirty": False,
            "corpus_manifest_sha256": corpus,
            "ocr_arms": ocr_arms or {"chargrid": "off", "texture": "on"}}


def provider_report(rate=None, failed_specimen_valid=0):
    provider = {"provider_id": "mrz", "checksum_valid_on_failed_specimen": failed_specimen_valid}
    if rate is not None:
        provider["tier1_hit_rate"] = {"status": "computed", "rate": rate}
    return {"mrz_class_sweep_arm": "off", "model_paths": {"detection": "d", "recognition": "r"},
            "providers": [provider]}


def write_arm(root, name, reports=None, outcomes=None, zones=None, run=None, real_report=None):
    arm = Path(root) / name
    arm.mkdir()
    for file_name, rep in (reports or {}).items():
        (arm / file_name).write_text(json.dumps(rep), encoding="utf-8")
    if outcomes is not None:
        real = arm / "real"
        real.mkdir()
        (real / d.OUTCOMES).write_text("".join(json.dumps(r) + "\n" for r in outcomes), encoding="utf-8")
        if zones is not None:
            (real / d.ZONES).write_text("".join(json.dumps(r) + "\n" for r in zones), encoding="utf-8")
        if run is not None:
            (real / ARCHIVE).write_text(json.dumps(run), encoding="utf-8")
            (real / d.CURRENT_RUN).write_text(ARCHIVE + "\n", encoding="utf-8")
        if real_report is not None:
            (real / d.REPORT).write_text(json.dumps(real_report), encoding="utf-8")
    return arm


def outcome_row(asset, outcome, fmt="TD3", reason=None, names_exact=None, name_error=None):
    return {"asset_id": asset, "outcome": outcome, "mrz_format": fmt, "miss_reason": reason,
            "names_exact": names_exact, "name_error": name_error}


def zone_row(asset, lines, fixture=None, provider="mrz", sha="s1"):
    return {"asset_id": asset, "provider": provider, "recovered_mrz_lines": lines,
            "ground_truth_mrz": fixture, "source_sha256": sha, "raw_ocr_text": "RAW OCR TEXT"}


def run(before, after, *args):
    out, err = io.StringIO(), io.StringIO()
    with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
        code = d.main([str(before), str(after), *args])
    return code, out.getvalue(), err.getvalue()


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
    def compare(self, before_docs, after_docs, **meta):
        return d.compare_synthetic(report(*before_docs, **meta), report(*after_docs, **meta))

    def test_timing_alone_is_no_change_and_retry_telemetry_is_flag_only(self):
        r = self.compare([doc(0, True), doc(1, True)],
                         [doc(0, True, elapsed_ms=9000), doc(1, True, retry_stop="budget")])
        self.assertEqual(r["changed"], [])
        self.assertEqual(r["flag_only"], [{"seed": 1, "keys": ["retry_stop"]}])

    def test_a_flag_present_in_one_arm_only_is_not_a_change(self):
        r = self.compare([doc(0, True)], [doc(0, True, line1_flagged=True)])
        self.assertEqual((r["changed"], r["flag_only"]), ([], []))

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

    def test_emitted_counts_must_match_the_seeds(self):
        good = report(doc(0, True), accepted_reads=1, prefix_wrong_accepted_reads=0)
        bad = report(doc(0, True), accepted_reads=2, prefix_wrong_accepted_reads=0)
        self.assertEqual(d.compare_synthetic(good, good)["problems"], [])
        (problem,) = d.compare_synthetic(good, bad)["problems"]
        self.assertIn("accepted_reads: the report says 2, its seeds give 1", problem)

    def test_a_report_without_miss_kind_compares_hit_or_miss_only(self):
        old = doc(3, False, "checksum_failed")
        del old["miss_kind"]
        r = d.compare_synthetic(report(old), report(doc(3, False, "checksum_failed")))
        self.assertEqual(r["changed"], [])
        self.assertIsNone(r["before"]["accepted_reads"])
        self.assertTrue(any("miss_kind" in n for n in r["notes"]))

    def test_a_report_without_check_states_never_classes_a_valid_miss(self):
        valid = doc(99, False, "document_number_mismatch",
                    check_states={"composite": True, "document_number": True})
        r = d.compare_synthetic(report(doc(99, False, "document_number_mismatch")), report(valid))
        self.assertIsNone(r["after"]["valid_misses"])
        self.assertEqual(r["changed"], [])

    def test_ocr_arm_differences_are_noted(self):
        r = d.compare_synthetic(report(doc(0, True), ocr_arms={"chargrid": "off"}),
                                report(doc(0, True), ocr_arms={"chargrid": "on"}))
        self.assertEqual(r["problems"], [])
        self.assertTrue(any("ocr_arms differs" in n for n in r["notes"]))


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

    def test_real_outcome_names_and_zone_changes(self):
        kosovo, sweden, belgium = "passports/Kosovo_2023.jpg", "misc/Sweden_Visa_2024.jpg", "id_cards/Belgium.jpg"
        before = write_arm(self.root, "before", outcomes=[
            outcome_row(kosovo, "hit", names_exact=False, name_error="other"),
            outcome_row(sweden, "hit", "MRVB"), outcome_row(belgium, "checksum_failed", "TD1"),
        ], zones=[zone_row(kosovo, ["POOOO", "L2"], fixture="P<RKS\nL2"), zone_row(sweden, ["VIS", "L2"]),
                  zone_row(belgium, ["A", "B", "C"])], run=archive(), real_report=provider_report(2 / 3))
        after = write_arm(self.root, "after", outcomes=[
            outcome_row(kosovo, "hit", names_exact=True), outcome_row(sweden, "hit", "MRVB"),
            outcome_row(belgium, "hit", "TD1"),
        ], zones=[zone_row(kosovo, ["P<RKS", "L2"], fixture="P<RKS\nL2"), zone_row(sweden, ["VIS", "L2"]),
                  zone_row(belgium, ["A", "B", "C"])], run=archive(), real_report=provider_report(1.0))
        r = d.compare_arms(before, after, [sweden])["real"]
        self.assertEqual(r["problems"], [])
        self.assertEqual([c["asset"] for c in r["outcome_changes"]], [belgium])
        self.assertEqual([c["asset"] for c in r["names_changes"]], [kosovo])
        (z,) = r["zone_changes"]
        self.assertEqual(z["asset"], kosovo)
        self.assertEqual(z["fixture"], ["x=", "exact"])
        sb, sa = r["summary"]
        self.assertEqual((sb["hits"], sb["scored"], sb["documents"]), (2, 3, 3))
        self.assertEqual((sa["hits"], sa["scored"]), (3, 3))
        self.assertEqual((sb["names_exact"], sb["names_scored"], sa["names_exact"]), (0, 1, 1))
        self.assertEqual(sa["hits_by_format"], {"MRVB": 1, "TD1": 1, "TD3": 1})
        (w,) = r["watched"]
        self.assertTrue(w["known"])
        self.assertEqual(w["before"], w["after"])

    def test_raw_ocr_text_is_never_printed(self):
        asset = "passports/X.jpg"
        before = write_arm(self.root, "before", outcomes=[outcome_row(asset, "hit")],
                           zones=[zone_row(asset, ["A", "B"])])
        after = write_arm(self.root, "after", outcomes=[outcome_row(asset, "checksum_failed")],
                          zones=[zone_row(asset, ["A", "C"])])
        code, out, _ = run(before, after, "--asset", asset)
        self.assertEqual(code, 0)
        self.assertIn("A / C", out)
        self.assertNotIn("RAW OCR TEXT", out)

    def test_a_private_or_local_arm_is_refused_before_anything_is_printed(self):
        public = [outcome_row("passports/X.jpg", "hit")]
        before = write_arm(self.root, "before", outcomes=public, zones=[zone_row("passports/X.jpg", ["A"])],
                           run=archive())
        after = write_arm(self.root, "after", outcomes=public, zones=[zone_row("passports/X.jpg", ["B"])],
                          run=archive(flags=("--real-specimens", "--include-private", "--dump-ocr")))
        code, out, err = run(before, after)
        self.assertEqual((code, out), (2, ""))
        self.assertIn("--include-private", err)
        local = write_arm(self.root, "local", outcomes=[outcome_row("local/Y.jpg", "hit")])
        code, out, err = run(before, local)
        self.assertEqual((code, out), (2, ""))
        self.assertIn("local track", err)

    def test_a_different_corpus_or_image_is_not_an_ab_but_a_different_arm_is_a_note(self):
        asset = "passports/X.jpg"
        before = write_arm(self.root, "before", outcomes=[outcome_row(asset, "hit")],
                           zones=[zone_row(asset, ["A"])], run=archive())
        other_arm = write_arm(self.root, "chargrid", outcomes=[outcome_row(asset, "hit")],
                              zones=[zone_row(asset, ["A"])], run=archive(ocr_arms={"chargrid": "on",
                                                                                     "texture": "on"}))
        r = d.compare_arms(before, other_arm, [])["real"]
        self.assertEqual(r["problems"], [])
        self.assertTrue(any(n.startswith("ocr_arms differs") for n in r["notes"]))
        other_corpus = write_arm(self.root, "corpus", outcomes=[outcome_row(asset, "hit")],
                                 zones=[zone_row(asset, ["A"], sha="s2")], run=archive(corpus="d" * 64))
        problems = d.compare_arms(before, other_corpus, [])["real"]["problems"]
        self.assertTrue(any(p.startswith("corpus manifests differ") for p in problems))
        self.assertTrue(any(p.startswith("image bytes differ for 1 asset") for p in problems))
        self.assertEqual(run(before, other_corpus)[0], 1)

    def test_a_document_dumped_in_one_arm_only_is_named(self):
        # provider-bench does not dump a document_number_mismatch, so a hit that
        # becomes one leaves the dump.
        asset = "passports/X.jpg"
        before = write_arm(self.root, "before", outcomes=[outcome_row(asset, "hit")],
                           zones=[zone_row(asset, ["A"])])
        after = write_arm(self.root, "after", outcomes=[outcome_row(asset, "document_number_mismatch")],
                          zones=[])
        r = d.compare_arms(before, after, [asset])["real"]
        (one,) = r["dumped_in_one_arm_only"]
        self.assertEqual((one["asset"], one["outcome"]), (asset, ["hit", "document_number_mismatch"]))
        (w,) = r["watched"]
        self.assertEqual(w["fixture"][1], None)
        code, out, _ = run(before, after, "--asset", asset)
        self.assertIn("passports/X.jpg: hit -> document_number_mismatch, dumped before only", out)
        self.assertIn("zone not comparable: dumped in the before arm only", out)

    def test_only_the_mrz_provider_is_read_and_duplicates_are_an_error(self):
        asset = "passports/X.jpg"
        before = write_arm(self.root, "before", outcomes=[outcome_row(asset, "hit")],
                           zones=[zone_row(asset, ["A"]), zone_row(asset, ["LLM"], provider="llm")])
        after = write_arm(self.root, "after", outcomes=[outcome_row(asset, "hit")],
                          zones=[zone_row(asset, ["B"])])
        (z,) = d.compare_arms(before, after, [])["real"]["zone_changes"]
        self.assertEqual((z["before"], z["after"]), (["A"], ["B"]))
        dup = write_arm(self.root, "dup", outcomes=[outcome_row(asset, "hit")],
                        zones=[zone_row(asset, ["A"]), zone_row(asset, ["A2"])])
        self.assertEqual(run(before, dup)[0], 2)

    def test_false_positives_and_valid_reads_of_failed_specimens_raise_an_alarm(self):
        asset = "passports/X.jpg"
        before = write_arm(self.root, "before", outcomes=[outcome_row(asset, "no_mrz_expected")],
                           real_report=provider_report())
        after = write_arm(self.root, "after", outcomes=[outcome_row(asset, "false_positive_mrz")],
                          real_report=provider_report(failed_specimen_valid=1))
        _, out, _ = run(before, after)
        self.assertIn("ALARM (after): 1 false_positive_mrz", out)
        self.assertIn("ALARM (after): 1 checksum-valid read(s) of a checksum_failed_specimen", out)

    def test_different_asset_sets_are_not_an_ab(self):
        before = write_arm(self.root, "before", outcomes=[outcome_row("a", "hit"), outcome_row("b", "hit")])
        after = write_arm(self.root, "after", outcomes=[outcome_row("a", "hit")])
        code, out, _ = run(before, after)
        self.assertEqual(code, 1)
        self.assertIn("NOT AN A/B: real: asset sets differ", out)

    def test_zones_need_a_dump_in_both_arms(self):
        before = write_arm(self.root, "before", outcomes=[outcome_row("a", "hit")], zones=[zone_row("a", ["A"])])
        after = write_arm(self.root, "after", outcomes=[outcome_row("a", "hit")])
        code, out, _ = run(before, after)
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
        code, out, _ = run(before, after, "--json")
        self.assertEqual(code, 0)
        self.assertEqual(json.loads(out)["synthetic"]["td1.json"]["changed"], [])


if __name__ == "__main__":
    unittest.main()
