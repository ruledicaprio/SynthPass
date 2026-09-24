"""Synthetic-only tests for the local MRZ evidence classifier."""

import unittest

from classify_mrz_mechanisms import best_with_indel, classify, line_distance


def ledger(asset_id, outcome="hit", **extra):
    return {"asset_id": asset_id, "name": "duplicate-name", "outcome": outcome,
            "mrz_format": "MRVB", "names_exact": None, **extra}


def dump(asset_id, truth=None, raw="", recovered=None, **extra):
    return {"asset_id": asset_id, "name": "duplicate-name", "provider": "mrz",
            "source_sha256": "a" * 64, "run_manifest": "run.json",
            "ground_truth_mrz": truth, "raw_ocr_text": raw,
            "recovered_mrz_lines": recovered or [], "mrz_band_score": 0.7, **extra}


class MechanismClassifierTests(unittest.TestCase):
    def test_indel_constrained_distance_separates_substitutions_from_ties(self):
        self.assertEqual(line_distance("ABCDEF", "ABXDEF")["relation"], "substitution")
        self.assertEqual(best_with_indel("ABCDEF", "ABXDEF"), 2)
        # ABCD -> BACD: Hamming and Levenshtein cost 2. Delete A,
        # then insert A after B also costs 2, so this is a genuine tie.
        self.assertEqual(line_distance("ABCD", "BACD")["relation"], "tie")
        self.assertEqual(best_with_indel("ABCD", "BACD"), 2)
        self.assertEqual(line_distance("ABXCDEF", "ABCDEF")["relation"], "indel")

    def test_duplicate_names_remain_distinct_assets_and_unlabelled_is_not_correct(self):
        result = classify(
            [ledger("passports/one.png"), ledger("id_cards/one.png")],
            [dump("passports/one.png"), dump("id_cards/one.png")],
        )
        self.assertEqual(result["summary"]["total_documents"], 2)
        self.assertEqual(result["summary"]["scored_documents"], 2)
        self.assertEqual(result["summary"]["unlabelled_hits"], 2)
        self.assertEqual({r["truth_status"] for r in result["records"]}, {"unlabelled"})

    def test_one_asset_can_have_multiple_mechanism_labels_and_a_tie(self):
        truth = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n4FWF<E9E21UTO7408122F1204159<<<<<<<<"
        raw = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n4FWFE9G21UTO7408122F1204159<<<<<<<<"
        result = classify(
            [ledger("visas/case.png", "checksum_failed")],
            [dump("visas/case.png", truth, raw,
                  ["V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
                   "4FWFE9G21UTO7408122F1204159<<<<<<<<<"])],
        )
        row = result["records"][0]
        self.assertTrue(row["review_target"])
        self.assertIn("indel", row["labels"])
        self.assertIn("inherited repair", row["labels"])
        self.assertEqual(result["summary"]["scored_documents"], 1)

    def test_tied_distances_report_both_candidate_mechanisms(self):
        truth = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n4FWF<E9E21UTO7408122F1204159<<<<<<<<"
        # Swap the first two cells: two substitutions or a delete/insert
        # pair each cost 2. A one-cell replacement would not be a tie.
        raw = truth.replace("4F", "F4", 1)
        result = classify(
            [ledger("visas/tie.png", "checksum_failed")],
            [dump("visas/tie.png", truth, raw, truth.splitlines())],
        )
        row = result["records"][0]
        self.assertTrue(row["distance_tie"])
        self.assertIn("indel", row["labels"])
        self.assertIn("substitution", row["labels"])
        self.assertEqual(result["summary"]["distance_ties"], 1)

    def test_printed_nonconformance_is_off_denominator(self):
        result = classify([ledger("id_cards/nonconforming.png", "checksum_failed_specimen")], [])
        self.assertEqual(result["summary"]["total_documents"], 1)
        self.assertEqual(result["summary"]["scored_documents"], 0)
        self.assertEqual(result["records"][0]["labels"], ["printed non-conformance"])

    def test_hit_with_ledger_name_error_is_a_target_without_fixture_truth(self):
        row = ledger("passports/name-error.png")
        row["name_error"] = "other"
        result = classify([row], [dump("passports/name-error.png")])
        self.assertTrue(result["records"][0]["review_target"])
        self.assertEqual(result["records"][0]["truth_status"], "unlabelled")

    def test_td1_hit_with_only_name_line_different_is_a_target(self):
        truth_lines = ["A" * 30, "B" * 30, "C" * 30]
        recovered = [*truth_lines[:2], "D" + "C" * 29]
        row = ledger("id_cards/td1.png", mrz_format="TD1", names_exact=True)
        result = classify([row], [dump("id_cards/td1.png", "\n".join(truth_lines),
                                       "\n".join(recovered), recovered)])
        record = result["records"][0]
        self.assertFalse(record["line1_error"])
        self.assertEqual(record["zone_lines_differing"], [2])
        self.assertTrue(record["review_target"])

    def test_td3_hit_with_only_second_line_different_is_a_target(self):
        truth_lines = ["P" * 44, "A" * 44]
        recovered = [truth_lines[0], "B" + "A" * 43]
        row = ledger("passports/td3.png", mrz_format="TD3", names_exact=True)
        result = classify([row], [dump("passports/td3.png", "\n".join(truth_lines),
                                       "\n".join(recovered), recovered)])
        record = result["records"][0]
        self.assertFalse(record["line1_error"])
        self.assertEqual(record["zone_lines_differing"], [1])
        self.assertTrue(record["review_target"])

    def test_hit_with_exact_zone_is_not_a_target(self):
        truth = "A" * 30 + "\n" + "B" * 30 + "\n" + "C" * 30
        row = ledger("id_cards/exact.png", mrz_format="TD1", names_exact=True)
        result = classify([row], [dump("id_cards/exact.png", truth, truth,
                                       truth.splitlines())])
        record = result["records"][0]
        self.assertEqual(record["zone_lines_differing"], [])
        self.assertFalse(record["review_target"])

    def test_hit_without_truth_has_no_zone_comparison(self):
        row = ledger("passports/no-truth.png", names_exact=True)
        result = classify([row], [dump("passports/no-truth.png")])
        record = result["records"][0]
        self.assertIsNone(record["zone_lines_differing"])
        self.assertFalse(record["review_target"])

    def test_hit_with_line_count_mismatch_keeps_existing_target_rule(self):
        truth = "A" * 30 + "\n" + "B" * 30 + "\n" + "C" * 30
        row = ledger("id_cards/missing-line.png", mrz_format="TD1", names_exact=True)
        result = classify([row], [dump("id_cards/missing-line.png", truth, truth,
                                       truth.splitlines()[:2])])
        record = result["records"][0]
        self.assertIsNone(record["zone_lines_differing"])
        self.assertFalse(record["review_target"])

    def test_missing_asset_id_or_hash_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "unique asset_id"):
            classify([ledger("a")], [dump("")])
        bad = dump("a")
        bad.pop("source_sha256")
        with self.assertRaisesRegex(ValueError, "metadata missing"):
            classify([ledger("a")], [bad])


if __name__ == "__main__":
    unittest.main()
