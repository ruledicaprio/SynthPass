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


def provider_report(rate=None, failed_specimen_valid=0, details=None):
    provider = {"provider_id": "mrz", "checksum_valid_on_failed_specimen": failed_specimen_valid}
    if details is not None:
        provider["documents_detail"] = details
    if rate is not None:
        provider["tier1_hit_rate"] = {"status": "computed", "rate": rate}
    return {"mrz_class_sweep_arm": "off", "model_paths": {"detection": "d", "recognition": "r"},
            "providers": [provider]}


def write_arm(root, name, reports=None, outcomes=None, zones=None, run=None, real_report=None,
              stdouts=None, passes=None):
    arm = Path(root) / name
    arm.mkdir()
    for file_name, rep in (reports or {}).items():
        (arm / file_name).write_text(json.dumps(rep), encoding="utf-8")
    for file_name, text in (stdouts or {}).items():
        (arm / file_name).write_text(text, encoding="utf-8")
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
        if passes is not None:
            (real / d.PASSES).write_text("".join(json.dumps(r) + "\n" for r in passes), encoding="utf-8")
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

    def test_a_replays_identity_block_names_the_capture_it_replayed(self):
        asset = "passports/X.jpg"
        replay_of = {"run_manifest": "provider-bench-ocr-run-cap.json", "sha256": "d" * 64}
        capture = write_arm(self.root, "capture", outcomes=[outcome_row(asset, "hit")],
                            zones=[zone_row(asset, ["A"])], run=archive(), real_report=provider_report(1.0))
        replay = write_arm(self.root, "replay", outcomes=[outcome_row(asset, "hit")],
                           zones=[zone_row(asset, ["A"])], run={**archive(), "replay_of": replay_of},
                           real_report=provider_report(1.0))
        r = d.compare_arms(capture, replay, [])["real"]
        self.assertEqual(r["problems"], [], "a replay of a capture is an A/B")
        self.assertEqual([i["replay_of"] for i in r["identity"]], [None, replay_of])
        self.assertIn("replay_of is recorded in one arm only", r["notes"])
        out = d.render({"synthetic": {}, "synthetic_in_one_arm_only": {"before": [], "after": []},
                        "real": r, "real_in_one_arm_only": None})
        self.assertIn("after:  commit aaaaaaa; corpus cccccccccccc; class sweep off; "
                      "replay of provider-bench-ocr-run-cap.json (sha256 dddddddddddd)", out)
        self.assertNotIn("replay of", next(line for line in out.splitlines() if line.startswith("  before:")))
        # Two replays of different captures differ in that identity, as a note, not a refusal.
        other = write_arm(self.root, "other", outcomes=[outcome_row(asset, "hit")],
                          zones=[zone_row(asset, ["A"])],
                          run={**archive(), "replay_of": {**replay_of, "sha256": "e" * 64}},
                          real_report=provider_report(1.0))
        r = d.compare_arms(replay, other, [])["real"]
        self.assertEqual(r["problems"], [])
        self.assertTrue(any(n.startswith("replay_of differs") for n in r["notes"]))

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


def detail_row(asset, fields=None):
    """A `documents_detail` row; `fields` is the `field_correctness` map, absent when None."""
    row = {"name": asset.rsplit("/", 1)[-1], "asset_id": asset, "read_ok": True}
    if fields is not None:
        row["field_correctness"] = fields
    return row


class FieldCorrectness(unittest.TestCase):
    A, B, C, D = "passports/A.jpg", "passports/B.jpg", "id_cards/C.jpg", "misc/D.jpg"

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = self.tmp.name

    def arm(self, name, details):
        assets = [row["asset_id"] for row in details] if details else [self.A]
        return write_arm(self.root, name, outcomes=[outcome_row(a, "hit") for a in assets],
                         real_report=provider_report(1.0, details=details))

    def fc(self, before, after):
        n = len(list(Path(self.root).iterdir()))
        arms = self.arm(f"before{n}", before), self.arm(f"after{n}", after)
        return d.compare_arms(*arms, [])["real"]["field_correctness"]

    def pair(self):
        before = [
            detail_row(self.A, {"issuing_country": "exact", "surname": "exact", "sex": "wrong"}),
            detail_row(self.B, {"issuing_country": "exact", "date_of_birth": "unread", "sex": "exact"}),
            detail_row(self.C, {"issuing_country": "wrong", "surname": "exact"}),
            detail_row(self.D),
        ]
        after = [
            detail_row(self.A, {"issuing_country": "wrong", "surname": "exact", "sex": "exact"}),
            detail_row(self.B, {"issuing_country": "exact", "date_of_birth": "exact", "sex": "unread"}),
            detail_row(self.C, {"issuing_country": "exact", "surname": "unread"}),
            detail_row(self.D),
        ]
        return before, after

    def test_transitions_are_counted_per_field(self):
        fc = self.fc(*self.pair())
        self.assertEqual(fc["recorded"], [True, True])
        self.assertEqual((fc["documents"], fc["compared"]), ([3, 3], 3))
        self.assertEqual(list(fc["transitions"]), ["issuing_country", "surname", "date_of_birth", "sex"])
        self.assertEqual(fc["transitions"]["issuing_country"],
                         {"exact->wrong": 1, "wrong->exact": 1, "exact->unread": 0, "unread->exact": 0})
        self.assertEqual(fc["transitions"]["surname"],
                         {"exact->wrong": 0, "wrong->exact": 0, "exact->unread": 1, "unread->exact": 0})
        self.assertEqual(fc["transitions"]["date_of_birth"],
                         {"exact->wrong": 0, "wrong->exact": 0, "exact->unread": 0, "unread->exact": 1})
        self.assertEqual(fc["transitions"]["sex"],
                         {"exact->wrong": 0, "wrong->exact": 1, "exact->unread": 1, "unread->exact": 0})

    def test_documents_with_an_exact_to_non_exact_field_are_named_by_asset_and_field(self):
        fc = self.fc(*self.pair())
        self.assertEqual(fc["regressions"], [
            {"asset": self.C, "fields": {"surname": "unread"}},
            {"asset": self.A, "fields": {"issuing_country": "wrong"}},
            {"asset": self.B, "fields": {"sex": "unread"}},
        ])

    def test_an_unchanged_pair_has_no_transitions_and_no_regressions(self):
        before, _ = self.pair()
        fc = self.fc(before, before)
        self.assertEqual((fc["transitions"], fc["regressions"]), ({}, []))
        _, out, _ = run(self.arm("b2", before), self.arm("a2", before))
        self.assertIn("no transitions", out)
        self.assertIn("documents with an exact -> non-exact field: 0", out)

    def test_an_arm_without_the_key_is_not_recorded(self):
        before, after = self.pair()
        old = [detail_row(r["asset_id"]) for r in before]
        for label, pair, recorded in (("before", (old, after), [False, True]),
                                      ("after", (before, old), [True, False]),
                                      ("both", (old, old), [False, False])):
            fc = self.fc(*pair)
            self.assertEqual(fc["recorded"], recorded, label)
            self.assertIsNone(fc["transitions"], label)
            self.assertIsNone(fc["regressions"], label)
        _, out, _ = run(self.arm("b2", old), self.arm("a2", after))
        self.assertIn("field correctness: not recorded in the before arm's report.json", out)
        # A report with no documents_detail at all, and no report.json, are the same.
        bare = write_arm(self.root, "bare", outcomes=[outcome_row(self.A, "hit")], real_report=provider_report(1.0))
        no_report = write_arm(self.root, "no_report", outcomes=[outcome_row(self.A, "hit")])
        recorded = self.arm("recorded", before)
        for arm in (bare, no_report):
            r = d.compare_arms(arm, recorded, [])["real"]
            self.assertEqual(r["field_correctness"]["recorded"], [False, True])

    def test_only_the_fields_a_document_has_in_both_arms_are_compared(self):
        fc = self.fc([detail_row(self.A, {"surname": "exact", "sex": "exact"})],
                     [detail_row(self.A, {"surname": "wrong"}), detail_row(self.B, {"surname": "wrong"})])
        self.assertEqual(fc["documents"], [1, 2])
        self.assertEqual(fc["compared"], 0 + 1)
        self.assertEqual(list(fc["transitions"]), ["surname"])
        self.assertEqual(fc["regressions"], [{"asset": self.A, "fields": {"surname": "wrong"}}])

    def test_the_rendering_names_fields_and_assets_and_never_a_value(self):
        before, after = self.pair()
        code, out, _ = run(self.arm("b2", before), self.arm("a2", after))
        self.assertEqual(code, 0)
        self.assertIn("field correctness: 3 documents compared (3 -> 3 with truth)", out)
        self.assertIn("issuing_country: exact->wrong 1, wrong->exact 1, exact->unread 0, unread->exact 0", out)
        self.assertIn("documents with an exact -> non-exact field: 3", out)
        self.assertIn(f"{self.A}: issuing_country exact -> wrong", out)
        self.assertIn(f"{self.C}: surname exact -> unread", out)

    def test_a_map_that_carried_a_value_would_still_print_none(self):
        before = [detail_row(self.A, {"surname": "exact", "sex": "SECRETVALUE9"})]
        after = [detail_row(self.A, {"surname": "SECRETVALUE9", "sex": "exact"})]
        code, out, _ = run(self.arm("b2", before), self.arm("a2", after), "--json")
        self.assertEqual(code, 0)
        self.assertNotIn("SECRETVALUE9", out)
        code, out, _ = run(self.arm("b3", before), self.arm("a3", after))
        self.assertNotIn("SECRETVALUE9", out)
        self.assertIn(f"{self.A}: surname exact -> other", out)

    def test_the_json_result_carries_the_comparison(self):
        before, after = self.pair()
        _, out, _ = run(self.arm("b2", before), self.arm("a2", after), "--json")
        fc = json.loads(out)["real"]["field_correctness"]
        self.assertEqual(fc["compared"], 3)
        self.assertEqual(len(fc["regressions"]), 3)

    def test_expect_identical_does_not_read_the_report_maps(self):
        # The neutrality mode compares the ledger, dump and trace rows; report.json is not among them.
        before, after = self.pair()
        code, out, _ = run(self.arm("b2", before), self.arm("a2", after), "--expect-identical")
        self.assertEqual(code, 0, out)
        self.assertNotIn("field correctness", out)


def selection_row(verdict, reason=None, arm="control", **extra):
    """A `documents_detail[].line1_selection` object, as `provider-bench` writes it."""
    row = {"arm": arm, "verdict": verdict, "reason": reason, "eligible": 1, "distinct": 1,
           "names_changed": verdict in ("applied", "proposed"), "source_passes": [], "source_transforms": []}
    row.update(extra)
    return row


def selection_detail(asset, selection=None):
    row = {"name": asset.rsplit("/", 1)[-1], "asset_id": asset, "read_ok": True}
    if selection is not None:
        row["line1_selection"] = selection
    return row


class Line1Selection(unittest.TestCase):
    A, B, C, D = "passports/A.jpg", "passports/B.jpg", "id_cards/C.jpg", "misc/D.jpg"

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = self.tmp.name

    def arm(self, name, details, select_arm=None):
        assets = [row["asset_id"] for row in details] if details else [self.A]
        real_report = provider_report(1.0, details=details)
        if select_arm is not None:
            real_report["mrz_line1_select_arm"] = select_arm
        return write_arm(self.root, name, outcomes=[outcome_row(a, "hit") for a in assets],
                         real_report=real_report)

    def real(self, before, after, before_arm=None, after_arm=None):
        n = len(list(Path(self.root).iterdir()))
        arms = self.arm(f"before{n}", before, before_arm), self.arm(f"after{n}", after, after_arm)
        return d.compare_arms(*arms, [])["real"]

    def control_and_on(self):
        control = [
            selection_detail(self.A, selection_row("proposed")),
            selection_detail(self.B, selection_row("none", "kept")),
            selection_detail(self.C, selection_row("ambiguous")),
            selection_detail(self.D, selection_row("unresolved", "issuer_unresolved")),
        ]
        on = [
            selection_detail(self.A, selection_row("applied", arm="on")),
            selection_detail(self.B, selection_row("none", "kept", arm="on")),
            selection_detail(self.C, selection_row("ambiguous", arm="on")),
            selection_detail(self.D, selection_row("unresolved", "issuer_unresolved", arm="on")),
        ]
        return control, on

    def test_the_identity_block_names_each_arms_selector_arm(self):
        control, on = self.control_and_on()
        r = self.real(control, on, "control", "on")
        self.assertEqual([i["mrz_line1_select_arm"] for i in r["identity"]], ["control", "on"])
        self.assertIn("mrz_line1_select_arm differs: control -> on", r["notes"])
        out = d.render({"synthetic": {}, "synthetic_in_one_arm_only": {"before": [], "after": []},
                        "real": r, "real_in_one_arm_only": None})
        self.assertIn("line-1 select control", next(x for x in out.splitlines() if x.startswith("  before:")))
        self.assertTrue(next(x for x in out.splitlines() if x.startswith("  after:")).endswith("line-1 select on"))

    def test_verdict_counts_and_the_named_assets_per_arm(self):
        control, on = self.control_and_on()
        sel = self.real(control, on, "control", "on")["line1_selection"]
        before, after = sel["arms"]
        self.assertEqual(before["counts"], {"applied": 0, "proposed": 1, "unresolved": 1, "ambiguous": 1, "none": 1})
        self.assertEqual(after["counts"], {"applied": 1, "proposed": 0, "unresolved": 1, "ambiguous": 1, "none": 1})
        self.assertEqual(before["assets"], {"applied": [], "proposed": [self.A], "ambiguous": [self.C]})
        self.assertEqual(after["assets"], {"applied": [self.A], "proposed": [], "ambiguous": [self.C]})
        self.assertEqual(sel["flags"], [], "control's proposal is on's application: no flag")

    def test_an_asset_proposed_in_one_arm_and_not_applied_in_the_other_is_flagged(self):
        control, on = self.control_and_on()
        on[0] = selection_detail(self.A, selection_row("none", "no_candidate", arm="on"))
        sel = self.real(control, on, "control", "on")["line1_selection"]
        self.assertEqual(sel["flags"], [{"asset": self.A, "verdicts": ["proposed", "none"]}])
        # Either direction.
        flipped = self.real(on, control, "on", "control")["line1_selection"]
        self.assertEqual(flipped["flags"], [{"asset": self.A, "verdicts": ["none", "proposed"]}])
        out = d.render({"synthetic": {}, "synthetic_in_one_arm_only": {"before": [], "after": []},
                        "real": self.real(control, on, "control", "on"), "real_in_one_arm_only": None})
        self.assertIn(f"FLAG {self.A}: proposed -> none", out)

    def test_two_control_arms_are_not_flagged(self):
        control, _ = self.control_and_on()
        sel = self.real(control, control, "control", "control")["line1_selection"]
        self.assertEqual(sel["flags"], [])

    def test_an_arm_that_recorded_nothing_is_reported_not_flagged(self):
        control, _ = self.control_and_on()
        off = [selection_detail(row["asset_id"]) for row in control]
        sel = self.real(off, control, "off", "control")["line1_selection"]
        self.assertEqual([a["recorded"] for a in sel["arms"]], [False, True])
        self.assertEqual(sel["flags"], [])
        out = d.render({"synthetic": {}, "synthetic_in_one_arm_only": {"before": [], "after": []},
                        "real": self.real(off, control, "off", "control"), "real_in_one_arm_only": None})
        self.assertIn("before: arm off, not recorded", out)

    def test_neither_arm_recording_prints_nothing_and_an_old_report_still_compares(self):
        plain = [selection_detail(self.A), selection_detail(self.B)]
        r = self.real(plain, plain)
        self.assertIsNone(r["line1_selection"])
        self.assertEqual(r["notes"], [])
        out = d.render({"synthetic": {}, "synthetic_in_one_arm_only": {"before": [], "after": []},
                        "real": r, "real_in_one_arm_only": None})
        self.assertNotIn("line-1 selection", out)
        self.assertTrue(next(x for x in out.splitlines() if x.startswith("  before:")).endswith("line-1 select n/a"))

    def test_the_rendering_names_verdicts_and_assets_and_never_a_line(self):
        control, on = self.control_and_on()
        # A row that carried document text under a key this script never reads.
        control[0]["line1_selection"]["proposed_line1"] = "P<UTOSECRETLINE9<<"
        control[1]["line1_selection"]["verdict"] = "SECRETLINE9"
        out = d.render({"synthetic": {}, "synthetic_in_one_arm_only": {"before": [], "after": []},
                        "real": self.real(control, on, "control", "on"), "real_in_one_arm_only": None})
        self.assertIn("line-1 selection:", out)
        self.assertIn(f"proposed: {self.A}", out)
        self.assertIn(f"applied: {self.A}", out)
        self.assertIn("other 1", out, "an unknown verdict is counted as `other`, never echoed")
        self.assertNotIn("SECRETLINE9", out)
        self.assertNotIn("P<UTO", out)

    def test_the_json_result_carries_the_comparison(self):
        control, on = self.control_and_on()
        before = self.arm("jb", control, "control")
        after = self.arm("ja", on, "on")
        code, out, _ = run(before, after, "--json")
        self.assertEqual(code, 0, out)
        sel = json.loads(out)["real"]["line1_selection"]
        self.assertEqual(sel["arms"][0]["arm"], "control")
        self.assertEqual(sel["arms"][1]["assets"]["applied"], [self.A])


SECRET = "SECRETLINE9"


def dump_stdout(texts):
    """A `synthpass-bench` stdout: one `--- seed N raw OCR lines ---` block per seed."""
    out = []
    for seed, text in texts.items():
        out.append(f"--- seed {seed} raw OCR lines ---")
        out += [f"  [{i}] {json.dumps(line)}" for i, line in enumerate(text.split("\n"))]
        out.append("")
    return "\n".join(out) + "\n"


def traced(seed, **over):
    """A seed with a valid pass trace: general read nothing, pass-00 appended a
    line, pass-01 was accepted."""
    passes = [
        {"order": 0, "id": "general", "transform": "general", "outcome": "no_mrz_shaped_lines", "readings": []},
        {"order": 1, "id": "pass-00", "transform": "mrz_variants:0", "outcome": "appended",
         "readings": [{"text": "LINE-A"}]},
        {"order": 2, "id": "pass-01", "transform": "mrz_variants:1", "outcome": "accepted",
         "readings": [{"text": "LINE-B"}]},
    ]
    out = doc(seed, True, retry_stop="variant_valid", retry_variant_id="pass-01",
              ocr_text="GENERAL\nLINE-A\nLINE-B", ocr_passes=passes)
    out.update(over)
    return out


def untraced(row):
    return {k: v for k, v in row.items() if k not in ("ocr_text", "ocr_passes")}


class ExpectIdentical(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = self.tmp.name

    def synth(self, name, docs, texts=None):
        stdouts = None if texts is None else {"td1.stdout": dump_stdout(texts)}
        return write_arm(self.root, name, reports={"td1.json": report(*docs)}, stdouts=stdouts)

    def neutral(self, before, after, *args):
        code, out, err = run(before, after, "--expect-identical", *args)
        return code, out, err

    def test_identical_arms_are_neutral(self):
        docs = [doc(0, True), doc(1, False, "checksum_failed")]
        texts = {0: "A\nB", 1: "C"}
        code, out, err = self.neutral(self.synth("a", docs, texts), self.synth("b", docs, texts))
        self.assertEqual((code, err), (0, ""))
        self.assertTrue(out.endswith("NEUTRAL\n"))
        self.assertNotIn("NOT NEUTRAL", out)
        self.assertIn("ignoring elapsed_ms, ocr_ms, run_manifest", out)
        self.assertIn("td1.json: seeds 2/2", out)

    def test_a_difference_only_in_ignored_keys_is_neutral(self):
        asset = "passports/X.jpg"
        ledger_a = {**outcome_row(asset, "hit"), "ocr_ms": 10}
        ledger_b = {**outcome_row(asset, "hit"), "ocr_ms": 9999}
        before = write_arm(self.root, "a", reports={"td1.json": report(doc(0, True))},
                           outcomes=[ledger_a], zones=[{**zone_row(asset, ["A"]), "run_manifest": "m1"}])
        after = write_arm(self.root, "b", reports={"td1.json": report(doc(0, True, elapsed_ms=77))},
                          outcomes=[ledger_b], zones=[{**zone_row(asset, ["A"]), "run_manifest": "m2"}])
        code, out, _ = self.neutral(before, after)
        self.assertEqual(code, 0, out)

    def test_ignore_names_the_key_and_leaves_it_out(self):
        before = self.synth("a", [doc(0, True, note="x")])
        after = self.synth("b", [doc(0, True, note="y")])
        self.assertEqual(self.neutral(before, after)[0], 3)
        code, out, _ = self.neutral(before, after, "--ignore", "note")
        self.assertEqual(code, 0)
        self.assertIn("ignoring elapsed_ms, note, ocr_ms, run_manifest", out)

    def test_one_changed_field_is_not_neutral_and_names_the_seed(self):
        before = self.synth("a", [doc(0, True), doc(41, True)])
        after = self.synth("b", [doc(0, True), doc(41, True, line1_flagged=True)])
        code, out, _ = self.neutral(before, after)
        self.assertEqual(code, 3)
        self.assertIn("results[] differ: 41 (line1_flagged)", out)
        self.assertTrue(out.endswith("NOT NEUTRAL\n"))
        code, out, _ = self.neutral(before, after, "--json")
        result = json.loads(out)
        self.assertEqual((code, result["exit"], result["neutral"]), (3, 3, False))
        self.assertEqual(result["synthetic"]["td1.json"]["results_differ"],
                         {"failed": 1, "ids": ["41 (line1_flagged)"]})

    def test_a_missing_dump_row_or_block_is_not_neutral(self):
        asset = "passports/X.jpg"
        rows = [outcome_row(asset, "hit"), outcome_row("passports/Y.jpg", "hit")]
        before = write_arm(self.root, "a", outcomes=rows, zones=[zone_row(asset, ["A"]),
                                                                  zone_row("passports/Y.jpg", ["B"])])
        after = write_arm(self.root, "b", outcomes=rows, zones=[zone_row(asset, ["A"])])
        code, out, _ = self.neutral(before, after)
        self.assertEqual(code, 3)
        self.assertIn("passports/Y.jpg (only in before)", out)
        docs = [doc(0, True), doc(1, True)]
        code, out, _ = self.neutral(self.synth("c", docs, {0: "A", 1: "B"}), self.synth("d", docs, {0: "A"}))
        self.assertEqual(code, 3)
        self.assertIn("dump blocks differ: 1 (only in before)", out)

    def test_changed_raw_text_is_not_neutral_and_never_printed(self):
        docs = [doc(0, True)]
        before = self.synth("a", docs, {0: f"{SECRET}\nB"})
        after = self.synth("b", docs, {0: "CHANGED-TEXT\nB"})
        for args in ((), ("--json",)):
            code, out, err = self.neutral(before, after, *args)
            self.assertEqual(code, 3)
            for text in (SECRET, "CHANGED-TEXT"):
                self.assertNotIn(text, out + err)
        asset = "passports/X.jpg"
        real_before = write_arm(self.root, "c", outcomes=[outcome_row(asset, "hit")],
                                zones=[{**zone_row(asset, ["ZONE-A"]), "raw_ocr_text": SECRET}])
        real_after = write_arm(self.root, "d", outcomes=[outcome_row(asset, "hit")],
                               zones=[{**zone_row(asset, ["ZONE-B"]), "raw_ocr_text": "CHANGED-TEXT"}])
        for args in ((), ("--json",)):
            code, out, err = self.neutral(real_before, real_after, *args)
            self.assertEqual(code, 3)
            self.assertIn("passports/X.jpg (raw_ocr_text, recovered_mrz_lines)", out)
            for text in (SECRET, "CHANGED-TEXT", "ZONE-A", "ZONE-B"):
                self.assertNotIn(text, out + err)

    def test_ocr_text_in_one_arm_must_equal_the_other_arms_dump(self):
        texts = {0: "A\nB"}
        before = self.synth("a", [doc(0, True)], texts)
        good = self.synth("b", [doc(0, True, ocr_text="A\nB")], texts)
        bad = self.synth("c", [doc(0, True, ocr_text="A\nX")], texts)
        code, out, _ = self.neutral(before, good)
        self.assertEqual(code, 0, out)
        self.assertIn("also ignored here: ocr_text", out)
        code, out, _ = self.neutral(before, bad)
        self.assertEqual(code, 3)
        self.assertIn("ocr_text differs from the other arm's dump: 0 (after)", out)
        self.assertNotIn("A\nX", out)

    def test_rust_debug_quoting_is_decoded(self):
        self.assertEqual(d.rust_debug_str(r'"a\"b\\c\n\u{e9}\u{1f600}\0"'), 'a"b\\c\né\U0001f600\x00')
        path = Path(self.root) / "x.stdout"
        path.write_text('noise\n--- seed 3 raw OCR lines ---\n  [0] "caf\\u{e9}"\n  [1] "B"\nseed 3: hit\n',
                        encoding="utf-8")
        self.assertEqual(d.block_text(d.dump_blocks(path)[3]), "café\nB")

    def test_an_escaped_backslash_is_not_an_escape_start(self):
        # Spelled by Rust as: "a\\0b\\u{41}\u{301}\0\"x"
        literal = '"a\\\\0b\\\\u{41}\\u{301}\\0\\"x"'
        self.assertEqual(d.rust_debug_str(literal), 'a\\0b\\u{41}\u0301\x00"x')
        for bad in ('no quotes', '"\\q"', '"dangling\\"', '"a"b"', '"\\u{110000}"'):
            with self.assertRaises(ValueError, msg=bad):
                d.rust_debug_str(bad)

    def test_a_valid_trace_passes_and_a_trace_needs_no_pair(self):
        before = self.synth("a", [untraced(traced(0))], {0: "GENERAL\nLINE-A\nLINE-B"})
        after = self.synth("b", [traced(0)], {0: "GENERAL\nLINE-A\nLINE-B"})
        code, out, _ = self.neutral(before, after, "--check-pass-trace")
        self.assertEqual(code, 0, out)
        self.assertIn("trace breaks 0 (passes 3)", out)

    def trace_break(self, mutate, expect):
        row = traced(0)
        mutate(row)
        before = self.synth("a", [untraced(traced(0))])
        after = self.synth("b", [row])
        code, out, _ = self.neutral(before, after, "--check-pass-trace")
        self.assertEqual(code, 3, out)
        self.assertIn(expect, out)
        self.assertNotIn("LINE-A", out)
        # Without the flag the same pair is neutral: the trace is one-sided, so ignored.
        self.assertEqual(self.neutral(before, after)[0], 0)

    def test_non_contiguous_pass_orders_are_a_break(self):
        def mutate(row):
            row["ocr_passes"][2]["order"] = 5
        self.trace_break(mutate, "trace breaks: 0 (after: orders are not contiguous")

    def test_two_accepted_passes_are_a_break(self):
        def mutate(row):
            row["ocr_passes"][1]["outcome"] = "accepted"
        self.trace_break(mutate, "2 accepted passes with stop variant_valid")

    def test_a_reading_tail_that_is_not_the_end_of_ocr_text_is_a_break(self):
        def mutate(row):
            row["ocr_text"] = "GENERAL\nLINE-A\nOTHER"
        self.trace_break(mutate, "the appended and accepted readings are not the tail of ocr_text")

    def test_readings_must_be_empty_exactly_when_nothing_was_read(self):
        def mutate(row):
            row["ocr_passes"][0]["readings"] = [{"text": "GENERAL"}]
        self.trace_break(mutate, "readings disagree with the outcome")

    def test_check_pass_trace_without_any_trace_is_refused(self):
        arm = self.synth("a", [doc(0, True)])
        other = self.synth("b", [doc(0, True)])
        code, out, err = self.neutral(arm, other, "--check-pass-trace")
        self.assertEqual((code, out), (2, ""))
        self.assertIn("neither arm has a pass trace", err)

    def test_the_real_trace_must_match_the_dump_and_the_ledger(self):
        asset = "passports/X.jpg"
        ledger = {**outcome_row(asset, "hit"), "retry_stop": "variant_valid", "retry_variant_id": "pass-01",
                  "retry_budget_hit": False}
        trace = {"asset_id": asset, "ocr_text": "GENERAL\nLINE-A\nLINE-B", "retry_stop": "variant_valid",
                 "retry_variant_id": "pass-01", "retry_budget_hit": False,
                 "ocr_passes": traced(0)["ocr_passes"]}
        dump = {**zone_row(asset, ["A"]), "raw_ocr_text": "GENERAL\nLINE-A\nLINE-B"}
        before = write_arm(self.root, "a", outcomes=[ledger], zones=[dump])
        good = write_arm(self.root, "b", outcomes=[ledger], zones=[dump], passes=[trace])
        code, out, _ = self.neutral(before, good, "--check-pass-trace")
        self.assertEqual(code, 0, out)
        wrong = {**trace, "ocr_text": "GENERAL\nLINE-A\nLINE-B\nMORE", "retry_stop": "exhausted"}
        bad = write_arm(self.root, "c", outcomes=[ledger], zones=[dump], passes=[wrong])
        code, out, _ = self.neutral(before, bad, "--check-pass-trace")
        self.assertEqual(code, 3)
        self.assertIn("trace ocr_text differs from the dump: passports/X.jpg (after)", out)
        self.assertIn("trace retry fields differ from the ledger: passports/X.jpg (after)", out)
        self.assertNotIn("MORE", out)

    def test_a_one_sided_budget_stop_is_not_an_ab(self):
        before = self.synth("a", [doc(0, True), doc(1, True)])
        after = self.synth("b", [doc(0, True), doc(1, True, retry_stop="budget")])
        code, out, _ = self.neutral(before, after)
        self.assertEqual(code, 1)
        self.assertIn("NOT AN A/B: a budget stop in one arm only: td1.json:1", out)
        self.assertIn("td1.json:1 (after only)", out)

    def test_a_budget_stop_in_both_arms_is_listed_with_its_text_verdict(self):
        docs = [doc(0, True, retry_stop="budget"), doc(1, True)]
        texts = {0: "A", 1: "B"}
        code, out, _ = self.neutral(self.synth("a", docs, texts), self.synth("b", docs, texts))
        self.assertEqual(code, 0, out)
        self.assertIn("budget stops: 1 in both arms: td1.json:0 (text same); 0 in one arm only", out)
        asset = "passports/X.jpg"
        row = {**outcome_row(asset, "hit"), "retry_stop": "budget"}
        real_a = write_arm(self.root, "c", outcomes=[row], zones=[zone_row(asset, ["A"])])
        real_b = write_arm(self.root, "d", outcomes=[row], zones=[zone_row(asset, ["A"])])
        self.assertIn("passports/X.jpg (text same)", self.neutral(real_a, real_b)[1])

    def test_a_corpus_mismatch_is_not_an_ab(self):
        asset = "passports/X.jpg"
        before = write_arm(self.root, "a", outcomes=[outcome_row(asset, "hit")], run=archive())
        after = write_arm(self.root, "b", outcomes=[outcome_row(asset, "hit")], run=archive(corpus="d" * 64))
        code, out, _ = self.neutral(before, after)
        self.assertEqual(code, 1)
        self.assertIn("NOT AN A/B: real: corpus manifests differ", out)

    def test_a_private_arm_is_still_refused(self):
        public = [outcome_row("passports/X.jpg", "hit")]
        before = write_arm(self.root, "a", outcomes=public, run=archive())
        after = write_arm(self.root, "b", outcomes=public,
                          run=archive(flags=("--real-specimens", "--include-private")))
        self.assertEqual(self.neutral(before, after)[:2], (2, ""))

    def test_a_format_in_one_arm_only_is_not_neutral(self):
        rep = report(doc(0, True))
        before = write_arm(self.root, "a", reports={"td1.json": rep, "td3.json": rep})
        after = write_arm(self.root, "b", reports={"td1.json": rep})
        code, out, _ = self.neutral(before, after)
        self.assertEqual(code, 3)
        self.assertIn("NOT COMPARED: td3.json only in the before arm", out)

    def test_at_most_eight_ids_are_listed_per_check(self):
        before = self.synth("a", [doc(n, True) for n in range(12)])
        after = self.synth("b", [doc(n, True, line1_flagged=True) for n in range(12)])
        code, out, _ = self.neutral(before, after)
        self.assertEqual(code, 3)
        self.assertIn("7 (line1_flagged), and 4 more", out)
        self.assertNotIn("8 (line1_flagged)", out)

    def real_pair(self, trace_a, trace_b):
        asset = "passports/X.jpg"
        rows = [outcome_row(asset, "hit")]
        self.pairs = getattr(self, "pairs", 0) + 1
        before = write_arm(self.root, f"a{self.pairs}", outcomes=rows, passes=trace_a)
        after = write_arm(self.root, f"b{self.pairs}", outcomes=rows, passes=trace_b)
        return before, after

    def test_real_trace_rows_are_compared_when_both_arms_have_them(self):
        row = {"asset_id": "passports/X.jpg", "ocr_text": "T", "retry_stop": "exhausted", "run_manifest": "m1",
               "ocr_passes": [{"order": 0, "id": "general", "outcome": "no_mrz_shaped_lines", "readings": []}]}
        same = self.real_pair([row], [{**row, "run_manifest": "m2"}])
        code, out, _ = self.neutral(*same)
        self.assertEqual(code, 0, out)
        self.assertIn("trace rows 1/1 rows, differ 0", out)
        other = {**row, "retry_stop": "budget"}
        code, out, _ = self.neutral(*self.real_pair([row], [other]))
        self.assertEqual(code, 3)
        self.assertIn("trace rows differ: passports/X.jpg (retry_stop)", out)

    def test_a_real_trace_in_one_arm_only_is_not_compared(self):
        row = {"asset_id": "passports/X.jpg", "ocr_text": "T", "ocr_passes": []}
        code, out, _ = self.neutral(*self.real_pair(None, [row]))
        self.assertEqual(code, 0, out)
        self.assertIn("trace rows not compared", out)

    def test_ignored_keys_apply_at_any_depth(self):
        def with_chargrid(value):
            row = traced(0)
            for rec in row["ocr_passes"]:
                rec["chargrid"] = value
            return row

        before = self.synth("a", [with_chargrid(None)])
        after = self.synth("b", [{**with_chargrid("on")}])
        code, out, _ = self.neutral(before, after, "--check-pass-trace")
        self.assertEqual(code, 3)
        self.assertIn("results[] differ: 0 (ocr_passes)", out)
        code, out, _ = self.neutral(before, after, "--check-pass-trace", "--ignore", "chargrid")
        self.assertEqual(code, 0, out)
        self.assertIn("chargrid", out.splitlines()[0])
        self.assertIn("at any depth", out.splitlines()[0])
        # A real trace row gains a nested key in one arm.
        row = {"asset_id": "passports/X.jpg", "ocr_text": "T",
               "ocr_passes": [{"order": 0, "id": "general", "readings": []}]}
        gained = {**row, "ocr_passes": [{**row["ocr_passes"][0], "chargrid": None}]}
        pair = self.real_pair([row], [gained])
        self.assertEqual(self.neutral(*pair)[0], 3)
        self.assertEqual(self.neutral(*pair, "--ignore", "chargrid")[0], 0)

    def capture_and_replay(self, replay_report=None, replay_ledger=None, replay_dump=None):
        """A capture (a live run with the pass trace) and its replay (no OCR: `ocr_ms` 0,
        another run manifest, no pass trace), the pair `--replay-ocr-passes` produces."""
        asset = "passports/X.jpg"
        self.pairs = getattr(self, "pairs", 0) + 1
        ledger = {**outcome_row(asset, "hit", names_exact=True), "ocr_ms": 8123, "retry_stop": "variant_valid",
                  "retry_variant_id": "pass-01", "retry_budget_hit": False}
        dump = {**zone_row(asset, ["A", "B"]), "run_manifest": "provider-bench-ocr-run-cap.json",
                "mrz_band_score": 0.5}
        trace = {"asset_id": asset, "ocr_text": "RAW OCR TEXT", "retry_stop": "variant_valid",
                 "retry_variant_id": "pass-01", "retry_budget_hit": False,
                 "run_manifest": "provider-bench-ocr-run-cap.json", "ocr_passes": traced(0)["ocr_passes"]}
        detail = {**detail_row(asset, {"surname": "exact"}), "ocr_ms": 8123, "retry_damaged_recovery": False,
                  "tier1_damaged_recovery": False, "check_states": {"composite": True}}
        capture_flags = ("--real-specimens", "--mrz-only", "--dump-ocr", "--dump-ocr-hits", "--dump-ocr-passes",
                         "--out", "cap/real/report.json")
        replay_flags = ("--real-specimens", "--mrz-only", "--replay-ocr-passes", "cap/real", "--dump-ocr",
                        "--dump-ocr-hits", "--out", "rep/real/report.json")
        capture = write_arm(self.root, f"capture{self.pairs}", outcomes=[ledger], zones=[dump], passes=[trace],
                            run=archive(flags=capture_flags),
                            real_report=provider_report(1.0, details=[detail]))
        replay = write_arm(
            self.root, f"replay{self.pairs}",
            outcomes=[replay_ledger or {**ledger, "ocr_ms": 0}],
            zones=[replay_dump or {**dump, "run_manifest": "provider-bench-ocr-run-rep.json"}],
            run={**archive(flags=replay_flags),
                 "replay_of": {"run_manifest": "provider-bench-ocr-run-cap.json", "sha256": "d" * 64}},
            real_report=provider_report(1.0, details=[replay_report or {**detail, "ocr_ms": 0}]))
        return capture, replay

    def test_a_capture_and_its_replay_can_reach_neutral(self):
        # Only the run manifest's name, timing and the pass trace the replay does not write differ;
        # the existing ignore rules cover all three, so no --ignore is needed.
        capture, replay = self.capture_and_replay()
        code, out, err = self.neutral(capture, replay)
        self.assertEqual((code, err), (0, ""), out)
        self.assertTrue(out.endswith("NEUTRAL\n"))
        self.assertIn("trace rows not compared (an arm has no trace file)", out)
        self.assertIn("dump 1/1 rows, differ 0", out)
        self.assertIn("note: the runs differ in flags, replay_of", out)
        self.assertNotIn("RAW OCR TEXT", out)

    def test_a_replay_that_read_something_else_is_not_neutral(self):
        asset = "passports/X.jpg"
        moved = {**outcome_row(asset, "checksum_failed", names_exact=True), "ocr_ms": 0, "retry_stop": "variant_valid",
                 "retry_variant_id": "pass-01", "retry_budget_hit": False}
        capture, replay = self.capture_and_replay(replay_ledger=moved)
        code, out, _ = self.neutral(capture, replay)
        self.assertEqual(code, 3)
        self.assertIn("ledger differ: passports/X.jpg (outcome)", out)
        # A band score the replay lost is a dump difference, which is why the row carries it.
        capture, replay = self.capture_and_replay(replay_dump={**zone_row(asset, ["A", "B"]), "mrz_band_score": None,
                                                              "run_manifest": "provider-bench-ocr-run-rep.json"})
        code, out, _ = self.neutral(capture, replay)
        self.assertEqual(code, 3)
        self.assertIn("dump rows differ: passports/X.jpg (mrz_band_score)", out)

    def test_check_report_compares_the_per_document_report_fields_a_replay_must_reproduce(self):
        capture, replay = self.capture_and_replay()
        code, out, _ = self.neutral(capture, replay, "--check-report")
        self.assertEqual(code, 0, out)
        self.assertIn("report rows 1/1, differ 0", out)
        # The fields the ledger and the dump lack: without --check-report they go unseen ...
        lost = {**detail_row("passports/X.jpg", {"surname": "exact"}), "ocr_ms": 0, "retry_damaged_recovery": None,
                "tier1_damaged_recovery": False, "check_states": {"composite": True}}
        capture, replay = self.capture_and_replay(replay_report=lost)
        self.assertEqual(self.neutral(capture, replay)[0], 0)
        # ... and with it, the one that moved is named.
        code, out, _ = self.neutral(capture, replay, "--check-report")
        self.assertEqual(code, 3)
        self.assertIn("report rows differ: passports/X.jpg (retry_damaged_recovery)", out)
        self.assertTrue(out.endswith("NOT NEUTRAL\n"))
        code, out, _ = self.neutral(capture, replay, "--check-report", "--json")
        self.assertEqual(json.loads(out)["real"]["report_differ"],
                         {"failed": 1, "ids": ["passports/X.jpg (retry_damaged_recovery)"]})

    def test_check_report_needs_a_report_in_some_arm_and_skips_a_one_sided_pair(self):
        asset = "passports/X.jpg"
        rows = [outcome_row(asset, "hit")]
        none_a = write_arm(self.root, "n1", outcomes=rows)
        none_b = write_arm(self.root, "n2", outcomes=rows)
        code, out, err = self.neutral(none_a, none_b, "--check-report")
        self.assertEqual((code, out), (2, ""))
        self.assertIn("--check-report was asked", err)
        with_report = write_arm(self.root, "r1", outcomes=rows,
                                real_report=provider_report(1.0, details=[detail_row(asset, {})]))
        code, out, _ = self.neutral(with_report, none_a, "--check-report")
        self.assertEqual(code, 0, out)
        self.assertIn("report rows not compared (an arm has no report.json rows)", out)
        # Not requested: nothing about the report is printed.
        self.assertNotIn("report rows", self.neutral(with_report, none_a)[1])

    def test_a_report_row_without_an_asset_id_or_repeated_is_an_error(self):
        asset = "passports/X.jpg"
        rows = [outcome_row(asset, "hit")]
        good = write_arm(self.root, "g", outcomes=rows, real_report=provider_report(1.0, details=[detail_row(asset)]))
        repeated = write_arm(self.root, "d", outcomes=rows,
                             real_report=provider_report(1.0, details=[detail_row(asset), detail_row(asset)]))
        code, out, err = self.neutral(good, repeated, "--check-report")
        self.assertEqual((code, out), (2, ""))
        self.assertIn("twice", err)

    def test_the_new_flags_need_the_new_mode_and_default_exit_codes_hold(self):
        arm = self.synth("a", [doc(0, True)])
        for flag in (["--check-pass-trace"], ["--ignore", "note"]):
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
                run(arm, arm, *flag)
            self.assertEqual(raised.exception.code, 2)
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit):
            run(arm, arm, "--expect-identical", "--asset", "x")
        changed = self.synth("b", [doc(0, True, line1_flagged=True)])
        self.assertEqual(run(arm, changed)[0], 0)
        self.assertEqual(self.neutral(arm, changed)[0], 3)

    def test_check_report_needs_the_neutrality_mode(self):
        arm = self.synth("only", [doc(0, True)])
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
            run(arm, arm, "--check-report")
        self.assertEqual(raised.exception.code, 2)


if __name__ == "__main__":
    unittest.main()
