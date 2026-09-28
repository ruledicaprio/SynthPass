"""Tests for synth_ab_diff.py: literal report fragments, no benchmark run."""
import unittest

import synth_ab_diff as d


def field(name, cer=0.0, expected=None, got=None):
    f = {"field": name, "cer": cer}
    if cer > 0:
        f.update(expected=expected, got=got)
    return f


def doc(seed, hit, wrong=None, fields=(), retry_stop="unset", retry_damaged_recovery="unset"):
    out = {"seed": seed, "hit": hit, "fields": list(fields)}
    if wrong is not None:
        out["wrong_accept"] = wrong
    # "unset" (the default) leaves the key entirely absent, matching a report
    # written before #473 added `retry_damaged_recovery` (or before #510
    # added `retry_stop` to every document) — the `retry_key`/`retry_cell`
    # "unknown" case. Pass `None` explicitly for a present-but-null value.
    if retry_stop != "unset":
        out["retry_stop"] = retry_stop
    if retry_damaged_recovery != "unset":
        out["retry_damaged_recovery"] = retry_damaged_recovery
    return out


def report(*docs, **meta):
    base = {"profile": "clean", "document_type": "TD3", "seed_start": 0, "count": len(docs)}
    base.update(meta)
    base["results"] = list(docs)
    return base


PREFIX_WRONG = [field("document_type", 1.0, "P", "PR"), field("issuing_country", 0.67, "RUS", "USP")]


class State(unittest.TestCase):
    def test_refused_correct_wrong(self):
        self.assertEqual(d.state(doc(0, False)), "refused")
        self.assertEqual(d.state(doc(0, True, wrong=False)), "correct")
        self.assertEqual(d.state(doc(0, True, wrong=True)), "wrong")

    def test_pre_457_report_falls_back_to_field_cer(self):
        self.assertEqual(d.state(doc(0, True, fields=PREFIX_WRONG)), "wrong")
        self.assertEqual(d.state(doc(0, True, fields=[field("surname")])), "correct")

    def test_checksum_valid_miss_is_not_refused(self):
        # PR #471's seed 99: every check digit passes, the document number is wrong.
        valid = dict(doc(99, False), check_states={"composite": True, "document_number": True,
                                                   "personal_number": None})
        self.assertEqual(d.state(valid), "valid-miss")
        failed = dict(doc(99, False), check_states={"composite": False, "document_number": True})
        self.assertEqual(d.state(failed), "refused")
        self.assertEqual(d.state(dict(doc(99, False), check_states={})), "refused")

    def test_mrz_lines_alone_is_not_wrong(self):
        self.assertEqual(d.state(doc(0, True, fields=[field("mrz_lines", 0.02, "a", "b")])), "correct")


class Compare(unittest.TestCase):
    def test_wrong_to_refused_and_refused_to_correct(self):
        before = report(doc(18, True, True, PREFIX_WRONG), doc(99, False, fields=PREFIX_WRONG))
        after = report(doc(18, False, fields=[field("document_number", 1.0, "MZ6", "PRU")]),
                       doc(99, True, False, [field("surname")]))
        r = d.compare(before, after)
        self.assertEqual(r["problems"], [])
        self.assertEqual(r["classes"], {"refused -> correct": 1, "wrong -> refused": 1})
        self.assertEqual((r["before"]["hits"], r["after"]["hits"]), (1, 1))
        self.assertEqual((r["before"]["correct"], r["after"]["correct"]), (0, 1))

    def test_unchanged_seed_is_not_listed(self):
        same = doc(1, True, False, [field("surname")])
        r = d.compare(report(same), report(dict(same)))
        self.assertEqual(r["moved"], [])

    def test_wrong_to_wrong_with_different_fields_is_listed(self):
        before = report(doc(37, True, True, PREFIX_WRONG + [field("surname", 1.1, "NYSTROM", "YSTROMLEILANI")]))
        after = report(doc(37, True, True, [field("surname", 1.0, "NYSTROM", "NYSTROMLEILANI")]))
        r = d.compare(before, after)
        self.assertEqual(r["classes"], {"wrong -> wrong": 1})
        changed = {c["field"] for c in r["moved"][0]["changes"]}
        self.assertEqual(changed, {"document_type", "issuing_country", "surname"})
        dt = next(c for c in r["moved"][0]["changes"] if c["field"] == "document_type")
        self.assertEqual((dt["before"], dt["after"]), ("PR", "=truth"))

    def test_different_corpora_are_not_an_ab(self):
        r = d.compare(report(doc(0, True, False)), report(doc(0, True, False), document_type="TD1"))
        self.assertTrue(any("document_type" in p for p in r["problems"]))

    def test_render_mentions_every_moved_seed(self):
        before = report(doc(18, True, True, PREFIX_WRONG))
        after = report(doc(18, False))
        text = d.render(d.compare(before, after))
        self.assertIn("seed 18: wrong -> refused", text)
        self.assertIn("hits 1 -> 0", text)


class RetryKey(unittest.TestCase):
    def test_absent_key_is_unknown(self):
        self.assertEqual(d.retry_cell({}, "retry_stop"), "unknown")
        self.assertEqual(d.retry_key({}), "unknown/unknown")

    def test_present_null_is_the_string_null_not_unknown(self):
        row = {"retry_stop": None, "retry_damaged_recovery": None}
        self.assertEqual(d.retry_cell(row, "retry_stop"), "null")
        self.assertEqual(d.retry_key(row), "null/null")

    def test_bool_renders_lowercase(self):
        row = {"retry_stop": "variant_valid_confirmed", "retry_damaged_recovery": True}
        self.assertEqual(d.retry_key(row), "variant_valid_confirmed/true")
        row["retry_damaged_recovery"] = False
        self.assertEqual(d.retry_key(row), "variant_valid_confirmed/false")


class ByRetryBreakdown(unittest.TestCase):
    def test_moved_seeds_are_tagged_and_counted_by_after_retry_state(self):
        before = report(
            doc(18, True, True, PREFIX_WRONG, retry_stop="general_valid",
                retry_damaged_recovery=False),
            doc(19, False, retry_stop="pass_cap", retry_damaged_recovery=None),
        )
        after = report(
            doc(18, True, False, [field("surname")], retry_stop="variant_valid_confirmed",
                retry_damaged_recovery=True),
            doc(19, True, False, [field("surname")], retry_stop="variant_valid_confirmed",
                retry_damaged_recovery=True),
        )
        r = d.compare(before, after)
        self.assertEqual(r["by_retry"], {"variant_valid_confirmed/true": 2})
        moved18 = next(m for m in r["moved"] if m["seed"] == 18)
        self.assertEqual(moved18["retry_before"], "general_valid/false")
        self.assertEqual(moved18["retry_after"], "variant_valid_confirmed/true")

    def test_old_report_with_no_retry_keys_at_all_is_unknown_not_a_crash(self):
        before = report(doc(0, True, True, PREFIX_WRONG))
        after = report(doc(0, False))
        r = d.compare(before, after)
        self.assertEqual(r["by_retry"], {"unknown/unknown": 1})
        text = d.render(r)
        self.assertIn("unknown/unknown", text)


if __name__ == "__main__":
    unittest.main()
