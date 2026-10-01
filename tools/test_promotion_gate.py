"""Tests for promotion_gate.py: the read-only arm-promotion gate over archived runs (#664).

Offline: every archive here is a few hand-built files in a temporary directory, in the shape
`test_archive_query.py` builds. The records carry one sentinel string wherever the real archive
carries document content (OCR text, zone lines, field values, `miss_reason` text, `argv`,
`model_paths`), so a test can say that no output, a refusal included, prints it.
"""
import contextlib
import copy
import hashlib
import io
import json
import tempfile
import unittest
from pathlib import Path

import promotion_gate as pg

SENTINEL = "SENTINEL-DOCUMENT-CONTENT-DO-NOT-PRINT"
BASE = "a" * 64
CAND_1 = "b" * 64
CAND_2 = "c" * 64
CAND_3 = "d" * 64
ASSETS = ("passports/a.png", "passports/b.png", "passports/c.png")
SHAS = {asset: str(index + 1) * 64 for index, asset in enumerate(ASSETS)}
SHAS["passports/d.png"] = "4" * 64
BASE_START_MS = 1_000_000_000_000  # 2001-09-09T01:46:40Z
DECLARED_UTC = "2001-09-09T00:00:00Z"


def header(run_id, started_ms, chargrid="off", **over):
    head = {
        "kind": "run", "schema": 1, "run_id": run_id, "binary_name": "provider-bench",
        "started_unix_ms": started_ms, "pid": 1, "source": "local",
        "machine": {"label": "win11-i5-4570", "os": "windows", "arch": "x86_64", "cpus": 4},
        "binary": "provider-bench.exe", "binary_sha256": "9" * 64,
        "git_commit": "0123456789abcdef0123456789abcdef01234567", "working_tree_dirty": False,
        "argv": ["--real-specimens", SENTINEL],
        "scope": {"corpus": "real-specimens", "format": None, "limit": None, "document_type": None,
                  "profile": None, "seed_start": None, "count": 3},
        "tracks": {"private": False, "local": False, "covers": False},
        "samples_data_sha": "5" * 40, "corpus_manifest_sha256": "e" * 64,
        "documents_loaded": 3, "labelled_loaded": 3, "providers": ["mrz"],
        "ocr_arms": {"texture": "on", "chargrid": chargrid},
        "retry_budget": {"max_passes": 14, "max_seconds": 520}, "mrz_arms": {"class_sweep": "off"},
        "pivot_yy": 26, "model_paths": {"detection": SENTINEL, "recognition": SENTINEL},
        "model_sha256": {"detection": "1" * 64, "recognition": "2" * 64},
        "replay_of": None, "env": {"RTEN_NUM_THREADS": "4"},
    }
    head.update(over)
    return head


def doc(run_id, asset, outcome="hit", names_exact=True, fields=None, budget_hit=False, mrz_found=True,
        read_us=10, ocr_ms=100, correctness="default", **top):
    """One provider record. `fields` maps a field name to `exact`, `wrong` or `unread`."""
    fields = {"surname": "exact", "given_names": "exact"} if fields is None else fields
    record = {
        "kind": "doc", "run_id": run_id, "provider": "mrz", "track": "public",
        "name": asset.rsplit("/", 1)[-1], "asset_id": asset, "source_sha256": SHAS[asset],
        "ledger_row": {
            "asset_id": asset, "name": asset.rsplit("/", 1)[-1], "outcome": outcome,
            "miss_reason": None if outcome == "hit" else f"{outcome}: {SENTINEL}",
            "mrz_format": "TD3", "mrz_found": mrz_found, "mrz_checksums_valid": outcome == "hit",
            "names_exact": names_exact, "name_error": None, "ocr_ms": ocr_ms,
            "retry_variant_id": "general", "retry_budget_hit": budget_hit,
            "retry_stop": "budget" if budget_hit else "general_valid", "check_states": {"composite": True},
            "retry_damaged_recovery": False, "tier1_damaged_recovery": False,
        },
        "read_ok": True, "read_us": read_us,
        "field_correctness": fields if correctness == "default" else correctness,
        "ocr": {"text": SENTINEL, "rotation": 0, "mrz_band_score": 0.5, "chargrid": None, "ocr_passes": None},
        "tier1_read": {"lines": [SENTINEL, SENTINEL], "damaged_recovery": False, "valid": True},
        "fields": {"surname": SENTINEL, "given_names": SENTINEL},
        "truth": {"zone_mismatch": 0, "compared_cells": 88, "field_mismatch": None},
    }
    record.update(top)
    return record


def base_docs(run_id=BASE):
    """Three documents: a reads exactly, b is a hit with a wrong name, c is a hit with an exact name."""
    return [
        doc(run_id, ASSETS[0]),
        doc(run_id, ASSETS[1], names_exact=False, fields={"surname": "wrong", "given_names": "exact"}),
        doc(run_id, ASSETS[2]),
    ]


def improved_docs(run_id):
    """The base documents with document b's names fixed: one more exact name."""
    return [
        doc(run_id, ASSETS[0]),
        doc(run_id, ASSETS[1], names_exact=True),
        doc(run_id, ASSETS[2]),
    ]


def declaration(**over):
    value = {
        "schema": 1, "declared_utc": DECLARED_UTC,
        "treatment": {"key": "ocr_arms.chargrid", "base": "off", "candidate": "on"},
        "target": {"metric": "strict_names", "min_effect": 1, "assets": None},
        "budget_is_subject": False,
    }
    value.update(over)
    return value


class GateCase(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = Path(self._tmp.name) / "archive"
        self.declare_path = Path(self._tmp.name) / "declare.json"
        self.addCleanup(self._tmp.cleanup)
        self.counter = 0

    def write_run(self, head, records, track="public", partial=False):
        directory = self.root / track
        directory.mkdir(parents=True, exist_ok=True)
        self.counter += 1
        name = f"20260101T00000{self.counter}Z-{head['binary_name']}-{head['run_id'][:12]}"
        path = directory / (name + (".jsonl.partial" if partial else ".jsonl"))
        path.write_text("\n".join(json.dumps(x) for x in [head, *records]) + "\n", encoding="utf-8")
        return path

    def write_declaration(self, value=None, raw=None):
        text = raw if raw is not None else json.dumps(declaration() if value is None else value)
        self.declare_path.write_text(text, encoding="utf-8")
        return hashlib.sha256(text.encode("utf-8")).hexdigest()

    def standard(self, base_records=None, cand_records=None, base_head=None, cand_heads=None):
        """A base run and two candidate repeats, written to the public track."""
        base = self.write_run(base_head or header(BASE, BASE_START_MS), base_records or base_docs())
        cands = []
        for index, run_id in enumerate((CAND_1, CAND_2)):
            head = (cand_heads[index] if cand_heads else None) or header(run_id, BASE_START_MS + 60_000 * (index + 1), "on")
            records = cand_records[index] if cand_records else improved_docs(run_id)
            cands.append(self.write_run(head, records))
        return base, cands

    def check(self, base, cands, *extra, declare=True):
        argv = ["check", str(base), *[str(c) for c in cands]]
        if declare:
            argv += ["--declare", str(self.declare_path)]
        argv += list(extra)
        out, err = io.StringIO(), io.StringIO()
        with contextlib.redirect_stderr(err):
            try:
                code = pg.main(argv, out=out)
            except SystemExit as exc:  # argparse: a missing required option
                code = exc.code
        return code, out.getvalue(), err.getvalue()

    def run_standard(self, **kwargs):
        self.write_declaration()
        base, cands = self.standard(**kwargs)
        return self.check(base, cands)

    def last_line(self, out):
        return out.strip().splitlines()[-1]

    def assertRefused(self, result, *fragments):
        code, out, _ = result
        self.assertEqual(code, 2, out)
        self.assertTrue(self.last_line(out).startswith("REFUSED"), out)
        for fragment in fragments:
            self.assertIn(fragment, out)
        self.assertNotIn(SENTINEL, out)


class ComparabilityTests(GateCase):
    def test_a_clean_pair_is_comparable(self):
        code, out, err = self.run_standard()
        self.assertEqual(code, 0, out + err)
        self.assertIn("comparable: yes", out)

    def test_the_binary_must_match_and_be_present(self):
        other = header(CAND_1, BASE_START_MS + 60_000, "on", binary_sha256="8" * 64)
        self.assertRefused(self.run_standard(cand_heads=[other, None]), "binary_sha256")
        self.setUp()
        absent = [header(BASE, BASE_START_MS, binary_sha256=None),
                  header(CAND_1, BASE_START_MS + 60_000, "on", binary_sha256=None),
                  header(CAND_2, BASE_START_MS + 120_000, "on", binary_sha256=None)]
        result = self.run_standard(base_head=absent[0], cand_heads=absent[1:])
        self.assertRefused(result, "is not recorded")

    def test_a_changed_model_hash_refuses_naming_the_key_and_printing_no_hash(self):
        for key in ("detection", "recognition"):
            with self.subTest(key=key):
                self.setUp()
                hashes = {"detection": "1" * 64, "recognition": "2" * 64, key: "9" * 64}
                other = header(CAND_1, BASE_START_MS + 60_000, "on", model_sha256=hashes)
                result = self.run_standard(cand_heads=[other, None])
                self.assertRefused(result, f"model_sha256 differs: {key}:")
                self.assertNotIn("9" * 12, result[1], "a hash is never printed")
                self.assertNotIn("1" * 12, result[1])
                other_key = "recognition" if key == "detection" else "detection"
                self.assertNotIn(f"differs: {other_key}", result[1], "only the key that differs is named")

    def test_paths_that_differ_with_equal_hashes_are_comparable(self):
        other = header(CAND_1, BASE_START_MS + 60_000, "on",
                       model_paths={"detection": SENTINEL + "-elsewhere", "recognition": SENTINEL + "-elsewhere"})
        code, out, err = self.run_standard(cand_heads=[other, None])
        self.assertEqual(code, 0, out + err)
        self.assertNotIn("model_paths", out)
        self.assertNotIn(SENTINEL, out + err)

    def test_a_live_run_without_the_hashes_refuses_as_not_recorded(self):
        # Written before the key existed: no `model_sha256` at all, on every run so they are equal.
        old = []
        for run_id, started, chargrid in ((BASE, BASE_START_MS, "off"), (CAND_1, BASE_START_MS + 60_000, "on"),
                                          (CAND_2, BASE_START_MS + 120_000, "on")):
            head = header(run_id, started, chargrid)
            del head["model_sha256"]
            old.append(head)
        result = self.run_standard(base_head=old[0], cand_heads=old[1:])
        self.assertRefused(result, "model_sha256 not recorded on base", "model_sha256 not recorded on candidate 1")

    def test_a_malformed_model_hash_refuses_as_not_recorded(self):
        bad = {
            "not 64 characters": {"detection": "1" * 63, "recognition": "2" * 64},
            "uppercase": {"detection": "A" * 64, "recognition": "2" * 64},
            "not hex": {"detection": "g" * 64, "recognition": "2" * 64},
            "a key missing": {"detection": "1" * 64},
            "a null key": {"detection": "1" * 64, "recognition": None},
            "not a string": {"detection": 5, "recognition": "2" * 64},
            "an empty object": {},
            "a string": "1" * 64,
            "null on a live run": None,
        }
        for label, value in bad.items():
            with self.subTest(label=label):
                self.setUp()
                broken = [header(run_id, started, chargrid, model_sha256=value)
                          for run_id, started, chargrid in ((BASE, BASE_START_MS, "off"),
                                                            (CAND_1, BASE_START_MS + 60_000, "on"),
                                                            (CAND_2, BASE_START_MS + 120_000, "on"))]
                result = self.run_standard(base_head=broken[0], cand_heads=broken[1:])
                self.assertRefused(result, "model_sha256 not recorded on")

    def test_two_replays_of_one_capture_are_comparable_and_say_no_model_was_loaded(self):
        capture = {"run_manifest": "provider-bench-ocr-run-aa.json", "sha256": "1" * 64}
        self.write_declaration(declaration(treatment={"key": "mrz_arms.class_sweep", "base": "off", "candidate": "on"}))
        heads = [header(run_id, started, "off", model_sha256=None, replay_of=capture, retry_budget=None,
                        model_paths={"detection": "(replay: no OCR model loaded)", "recognition": "(replay: no OCR model loaded)"},
                        mrz_arms={"class_sweep": sweep})
                 for run_id, started, sweep in ((BASE, BASE_START_MS, "off"), (CAND_1, BASE_START_MS + 60_000, "on"),
                                                (CAND_2, BASE_START_MS + 120_000, "on"))]
        base, cands = self.standard(base_head=heads[0], cand_heads=heads[1:])
        code, out, err = self.check(base, cands)
        self.assertEqual(code, 0, out + err)
        self.assertIn("models: none loaded (replays of one capture)", out)
        self.assertNotIn("compared by", out)

    def test_a_replay_against_a_live_run_is_not_comparable(self):
        capture = {"run_manifest": "provider-bench-ocr-run-aa.json", "sha256": "1" * 64}
        replay = header(CAND_1, BASE_START_MS + 60_000, "on", model_sha256=None, replay_of=capture)
        self.assertRefused(self.run_standard(cand_heads=[replay, None]), "replay_of", "model_sha256 differs")

    def test_a_live_report_says_the_models_were_compared_by_sha256(self):
        code, out, _ = self.run_standard()
        self.assertEqual(code, 0)
        self.assertIn("models: compared by SHA-256", out)
        self.assertNotIn("by path", out)
        self.assertNotIn("no hashes", out)

    def test_the_scope_must_match(self):
        scope = dict(header(BASE, 0)["scope"], count=2)
        other = header(CAND_1, BASE_START_MS + 60_000, "on", scope=scope)
        self.assertRefused(self.run_standard(cand_heads=[other, None]), "scope")

    def test_every_other_equal_fact_is_checked(self):
        cases = {
            "tracks": {"private": False, "local": True, "covers": False},
            "providers": ["mrz", "other"],
            "samples_data_sha": "6" * 40,
            "corpus_manifest_sha256": "f" * 64,
            "replay_of": {"run_manifest": "x.json", "sha256": "1" * 64},
        }
        for key, value in cases.items():
            with self.subTest(key=key):
                self.setUp()
                other = header(CAND_1, BASE_START_MS + 60_000, "on", **{key: value})
                self.assertRefused(self.run_standard(cand_heads=[other, None]), key)

    def test_the_document_set_must_join_one_to_one(self):
        self.assertRefused(self.run_standard(cand_records=[improved_docs(CAND_1)[:2], improved_docs(CAND_2)]),
                           "only in base")
        extra = improved_docs(CAND_1) + [doc(CAND_1, "passports/d.png", source_sha256="4" * 64)]
        self.setUp()
        self.assertRefused(self.run_standard(cand_records=[extra, improved_docs(CAND_2)]), "only in candidate")

    def test_a_changed_image_hash_refuses_naming_the_asset(self):
        changed = [improved_docs(CAND_1), improved_docs(CAND_2)]
        for run_id, records in zip((CAND_1, CAND_2), changed):
            records[0] = doc(run_id, ASSETS[0], source_sha256="f" * 64)
        result = self.run_standard(cand_records=changed)
        self.assertRefused(result, "source_sha256 differs or is missing", ASSETS[0])
        self.assertNotIn("f" * 64, result[1], "a hash is not printed")
        self.assertNotIn(ASSETS[1], result[1].split("source_sha256")[1])

    def test_a_missing_image_hash_refuses_naming_the_asset(self):
        for side in ("candidate", "base", "every run"):
            with self.subTest(side=side):
                self.setUp()
                if side == "every run":
                    # Equal on both sides, so only the presence rule can refuse it.
                    base = base_docs()
                    base[1] = doc(BASE, ASSETS[1], source_sha256=None, names_exact=False,
                                  fields={"surname": "wrong", "given_names": "exact"})
                    cands = [improved_docs(CAND_1), improved_docs(CAND_2)]
                    for run_id, records in zip((CAND_1, CAND_2), cands):
                        records[1] = doc(run_id, ASSETS[1], source_sha256=None)
                    result = self.run_standard(base_records=base, cand_records=cands)
                elif side == "candidate":
                    cands = [improved_docs(CAND_1), improved_docs(CAND_2)]
                    cands[1][1] = doc(CAND_2, ASSETS[1], source_sha256=None)
                    result = self.run_standard(cand_records=cands)
                else:
                    base = base_docs()
                    base[2] = doc(BASE, ASSETS[2], source_sha256=None)
                    result = self.run_standard(base_records=base)
                self.assertRefused(result, "source_sha256 differs or is missing")
                self.assertIn(ASSETS[2] if side == "base" else ASSETS[1], result[1])

    def test_two_documents_of_one_image_join_on_the_asset_id_when_the_hashes_are_equal(self):
        # Duplicate-byte images share a hash, so the join falls back to the asset id; the hashes
        # are still equal, and that is comparable.
        def with_shared_hash(records, run_id):
            records[2] = doc(run_id, ASSETS[2], source_sha256=SHAS[ASSETS[0]])
            return records

        base = with_shared_hash(base_docs(), BASE)
        cands = [with_shared_hash(improved_docs(r), r) for r in (CAND_1, CAND_2)]
        code, out, err = self.run_standard(base_records=base, cand_records=cands)
        self.assertEqual(code, 0, out + err)

    def test_an_undeclared_arm_that_differs_refuses(self):
        for field, value in (
            ("ocr_arms", {"texture": "off", "chargrid": "on"}),
            ("mrz_arms", {"class_sweep": "on"}),
            ("env", {"RTEN_NUM_THREADS": "2"}),
            ("retry_budget", {"max_passes": 14, "max_seconds": 600}),
            ("pivot_yy", 27),
        ):
            with self.subTest(field=field):
                self.setUp()
                other = header(CAND_1, BASE_START_MS + 60_000, "on", **{field: value})
                self.assertRefused(self.run_standard(cand_heads=[other, None]), field)

    def test_a_declared_arm_with_the_wrong_values_refuses(self):
        wrong_candidate = header(CAND_1, BASE_START_MS + 60_000, "control")
        self.assertRefused(self.run_standard(cand_heads=[wrong_candidate, None]), "ocr_arms.chargrid")
        self.setUp()
        self.assertRefused(self.run_standard(base_head=header(BASE, BASE_START_MS, "on")), "ocr_arms.chargrid")

    def test_a_budget_limited_document_refuses_unless_the_budget_is_the_subject(self):
        limited = base_docs()
        limited[0] = doc(BASE, ASSETS[0], budget_hit=True)
        self.assertRefused(self.run_standard(base_records=limited), "retry_budget_hit")
        self.setUp()
        candidate_limited = [improved_docs(CAND_1), improved_docs(CAND_2)]
        candidate_limited[1][2] = doc(CAND_2, ASSETS[2], budget_hit=True)
        self.assertRefused(self.run_standard(cand_records=candidate_limited), "retry_budget_hit")
        self.setUp()
        self.write_declaration(declaration(budget_is_subject=True))
        base, cands = self.standard(base_records=limited)
        code, out, _ = self.check(base, cands)
        self.assertNotEqual(code, 2, out)

    def test_the_candidate_repeats_are_compared_with_one_another(self):
        other = header(CAND_2, BASE_START_MS + 120_000, "on", binary_sha256="8" * 64)
        self.assertRefused(self.run_standard(cand_heads=[None, other]), "binary_sha256")

    def test_every_failing_fact_is_listed(self):
        other = header(CAND_1, BASE_START_MS + 60_000, "on", binary_sha256="8" * 64, pivot_yy=30,
                       scope=dict(header(BASE, 0)["scope"], count=2))
        _, out, _ = self.run_standard(cand_heads=[other, None])
        for fact in ("binary_sha256", "pivot_yy", "scope"):
            self.assertIn(fact, out)


class VetoTests(GateCase):
    def vetoed(self, candidate_records, kind, **kwargs):
        code, out, err = self.run_standard(cand_records=candidate_records, **kwargs)
        self.assertEqual(code, 3, out + err)
        self.assertEqual(self.last_line(out).split()[0], "VETOED", out)
        self.assertIn(f"veto {kind}", out)
        self.assertNotIn(SENTINEL, out)
        return out

    def both(self, edit):
        """Candidate repeats that both carry `edit` applied to the improved documents."""
        runs = []
        for run_id in (CAND_1, CAND_2):
            records = improved_docs(run_id)
            edit(records, run_id)
            runs.append(records)
        return runs

    def test_a_lost_hit_vetoes(self):
        def edit(records, run_id):
            records[2] = doc(run_id, ASSETS[2], outcome="no_mrz_found", names_exact=None, correctness=None)
        out = self.vetoed(self.both(edit), "hit_lost")
        self.assertIn(ASSETS[2], out)

    def test_a_field_regression_with_the_hit_count_unchanged_vetoes(self):
        def edit(records, run_id):
            records[0] = doc(run_id, ASSETS[0], fields={"surname": "exact", "given_names": "wrong"})
        out = self.vetoed(self.both(edit), "field_regressed")
        self.assertIn("given_names", out)
        self.assertNotIn("hit_lost", out)
        # An exact field that becomes unread is the same veto.
        def unread(records, run_id):
            records[0] = doc(run_id, ASSETS[0], fields={"surname": "unread", "given_names": "exact"})
        self.setUp()
        self.vetoed(self.both(unread), "field_regressed")

    def test_a_field_that_was_wrong_may_stay_wrong_or_change(self):
        def edit(records, run_id):
            records[1] = doc(run_id, ASSETS[1], names_exact=True, fields={"surname": "unread", "given_names": "exact"})
        code, out, _ = self.run_standard(cand_records=self.both(edit))
        self.assertEqual(code, 0, out)

    def test_a_document_without_truth_cannot_regress_a_field(self):
        base = base_docs()
        base[0] = doc(BASE, ASSETS[0], correctness=None)
        def edit(records, run_id):
            records[0] = doc(run_id, ASSETS[0], fields={"surname": "wrong", "given_names": "wrong"})
        code, out, _ = self.run_standard(base_records=base, cand_records=self.both(edit))
        self.assertEqual(code, 0, out)

    def test_a_new_false_positive_vetoes(self):
        def edit(records, run_id):
            records[1] = doc(run_id, ASSETS[1], outcome="false_positive_mrz", names_exact=False, correctness=None)
        out = self.vetoed(self.both(edit), "false_positive_mrz")
        self.assertIn(ASSETS[1], out)

    def test_a_false_positive_already_in_the_base_is_not_new(self):
        base = base_docs()
        base[1] = doc(BASE, ASSETS[1], outcome="false_positive_mrz", names_exact=False, correctness=None)
        def edit(records, run_id):
            records[1] = doc(run_id, ASSETS[1], outcome="false_positive_mrz", names_exact=False, correctness=None)
        code, out, _ = self.run_standard(base_records=base, cand_records=self.both(edit))
        self.assertEqual(code, 3, out)  # below target: no exact name gained, and no veto
        self.assertNotIn("  veto ", out)

    def test_a_new_document_number_mismatch_vetoes(self):
        def edit(records, run_id):
            records[1] = doc(run_id, ASSETS[1], outcome="document_number_mismatch", names_exact=True, correctness=None)
        self.vetoed(self.both(edit), "document_number_mismatch")

    def test_a_lost_exact_name_vetoes_even_with_the_hit_kept(self):
        def edit(records, run_id):
            records[0] = doc(run_id, ASSETS[0], names_exact=False)
        out = self.vetoed(self.both(edit), "names_exact_lost")
        self.assertNotIn("hit_lost", out)

    def test_a_null_names_exact_after_a_true_one_is_also_a_loss(self):
        def edit(records, run_id):
            records[0] = doc(run_id, ASSETS[0], names_exact=None)
        self.vetoed(self.both(edit), "names_exact_lost")

    def test_candidate_repeats_that_disagree_on_a_semantic_key_veto(self):
        second = improved_docs(CAND_2)
        second[1] = doc(CAND_2, ASSETS[1], names_exact=True, fields={"surname": "exact", "given_names": "unread"})
        out = self.vetoed([improved_docs(CAND_1), second], "candidate_runs_disagree")
        self.assertIn("field_correctness", out)
        self.assertNotIn(SENTINEL, out)

    def test_candidate_repeats_that_differ_in_a_value_field_veto_without_printing_it(self):
        second = improved_docs(CAND_2)
        second[0]["fields"] = {"surname": SENTINEL + "-OTHER", "given_names": SENTINEL}
        out = self.vetoed([improved_docs(CAND_1), second], "candidate_runs_disagree")
        self.assertIn("fields.surname", out)

    def test_the_timing_keys_are_these_and_no_others(self):
        self.assertEqual(pg.TIMING_KEYS, ("run_id", "read_us", "ledger_row.ocr_ms"))

    def test_repeats_that_differ_only_in_a_timing_key_do_not_veto(self):
        for label, edit in (
            ("run_id", lambda records: None),  # the run ids already differ between the repeats
            ("read_us", lambda records: records[0].__setitem__("read_us", 99_999)),
            ("ledger_row.ocr_ms", lambda records: records[1]["ledger_row"].__setitem__("ocr_ms", 99_999)),
        ):
            with self.subTest(timing=label):
                self.setUp()
                second = improved_docs(CAND_2)
                edit(second)
                code, out, _ = self.run_standard(cand_records=[improved_docs(CAND_1), second])
                self.assertEqual(code, 0, out)
                self.assertNotIn("candidate_runs_disagree", out)

    def test_a_document_in_the_wrong_order_is_still_the_same_document(self):
        second = list(reversed(improved_docs(CAND_2)))
        code, out, _ = self.run_standard(cand_records=[improved_docs(CAND_1), second])
        self.assertEqual(code, 0, out)

    def test_a_veto_beats_a_missed_target(self):
        def edit(records, run_id):
            records[0] = doc(run_id, ASSETS[0], names_exact=False)
            records[1] = doc(run_id, ASSETS[1], names_exact=False, fields={"surname": "wrong", "given_names": "exact"})
        self.vetoed(self.both(edit), "names_exact_lost")


class EfficacyTests(GateCase):
    def test_a_clean_improvement_that_meets_its_target_is_promotable(self):
        code, out, err = self.run_standard()
        self.assertEqual(code, 0, out + err)
        self.assertTrue(self.last_line(out).startswith("PROMOTABLE"), out)
        self.assertIn("strict_names", out)
        self.assertIn("base 2", out)
        self.assertIn("candidate 3", out)
        self.assertIn("effect +1", out)
        self.assertIn(ASSETS[1], out, "the mover is named")
        self.assertIn("false -> true", out)

    def test_a_clean_run_that_misses_its_target_is_below_target(self):
        self.write_declaration(declaration(target={"metric": "strict_names", "min_effect": 2, "assets": None}))
        base, cands = self.standard()
        code, out, _ = self.check(base, cands)
        self.assertEqual(code, 3, out)
        self.assertTrue(self.last_line(out).startswith("BELOW TARGET"), out)
        self.assertNotIn("  veto ", out)

    def test_with_several_candidates_the_smallest_effect_decides(self):
        self.write_declaration(declaration(target={"metric": "strict_names", "min_effect": 1, "assets": None}))
        base = self.write_run(header(BASE, BASE_START_MS), base_docs())
        runs = [self.write_run(header(CAND_1, BASE_START_MS + 60_000, "on"), improved_docs(CAND_1)),
                self.write_run(header(CAND_2, BASE_START_MS + 120_000, "on"), improved_docs(CAND_2))]
        code, out, _ = self.check(base, runs)
        self.assertEqual(code, 0, out)
        self.assertIn(CAND_1[:12], out)
        self.assertIn(CAND_2[:12], out)
        self.assertIn("smallest effect +1", out)

    def test_with_disagreeing_repeats_the_smallest_effect_is_the_one_reported(self):
        # The repeats disagree, so the verdict is a veto; the report still says which effect is the smallest.
        unchanged = base_docs(CAND_2)
        self.write_declaration(declaration(target={"metric": "strict_names", "min_effect": 1, "assets": None}))
        base, cands = self.standard(cand_records=[improved_docs(CAND_1), unchanged])
        code, out, _ = self.check(base, cands)
        self.assertEqual(code, 3, out)
        self.assertIn("smallest effect +0", out)
        self.assertTrue(self.last_line(out).startswith("VETOED"), out)

    def test_each_metric_counts_what_it_says(self):
        def not_a_hit(run_id, mrz_found):
            return doc(run_id, ASSETS[0], outcome="checksum_failed", names_exact=None, correctness=None, mrz_found=mrz_found)

        cases = (
            # `hits` moves, `mrz_found` does not: the MRZ was found in both runs.
            ({"metric": "hits", "min_effect": 1, "assets": None},
             [not_a_hit(BASE, True), *base_docs()[1:]], lambda run_id: improved_docs(run_id), "base 2", "candidate 3"),
            # `mrz_found` moves, `hits` does not: neither run has the hit.
            ({"metric": "mrz_found", "min_effect": 1, "assets": None},
             [not_a_hit(BASE, False), *base_docs()[1:]],
             lambda run_id: [not_a_hit(run_id, True), *improved_docs(run_id)[1:]], "base 2", "candidate 3"),
            ({"metric": "field_exact", "field": "surname", "min_effect": 1, "assets": None}, base_docs(),
             lambda run_id: [improved_docs(run_id)[0], doc(run_id, ASSETS[1], fields={"surname": "exact", "given_names": "exact"}),
                             doc(run_id, ASSETS[2])], "base 2", "candidate 3"),
        )
        for target, base_records, candidate_factory, base_text, candidate_text in cases:
            with self.subTest(metric=target["metric"]):
                self.setUp()
                self.write_declaration(declaration(target=target))
                base, cands = self.standard(base_records=base_records,
                                            cand_records=[candidate_factory(CAND_1), candidate_factory(CAND_2)])
                code, out, err = self.check(base, cands)
                self.assertEqual(code, 0, out + err)
                self.assertIn(base_text, out)
                self.assertIn(candidate_text, out)

    def test_the_target_can_be_limited_to_listed_assets(self):
        self.write_declaration(declaration(target={"metric": "strict_names", "min_effect": 1, "assets": [ASSETS[0]]}))
        base, cands = self.standard()
        code, out, _ = self.check(base, cands)
        self.assertEqual(code, 3, out)
        self.assertTrue(self.last_line(out).startswith("BELOW TARGET"), out)
        self.write_declaration(declaration(target={"metric": "strict_names", "min_effect": 1, "assets": [ASSETS[1]]}))
        code, out, _ = self.check(base, cands)
        self.assertEqual(code, 0, out)

    def test_a_listed_asset_the_runs_lack_refuses(self):
        self.write_declaration(declaration(target={"metric": "strict_names", "min_effect": 1, "assets": ["passports/zzz.png"]}))
        base, cands = self.standard()
        self.assertRefused(self.check(base, cands), "passports/zzz.png")

    def test_the_report_prints_the_declaration_hash_and_the_verdict_counts(self):
        digest = self.write_declaration()
        base, cands = self.standard()
        code, out, _ = self.check(base, cands)
        self.assertIn(digest, out)
        self.assertIn("effect +1", self.last_line(out))
        self.assertIn("0 vetoes", self.last_line(out))


class DeclarationTests(GateCase):
    def refused_declaration(self, value=None, raw=None, fragment=""):
        self.write_declaration(value, raw)
        base, cands = self.standard()
        result = self.check(base, cands)
        self.assertRefused(result, fragment)
        return result

    def test_a_declaration_dated_after_a_candidate_run_is_refused(self):
        self.refused_declaration(declaration(declared_utc="2001-09-09T01:47:40Z"), fragment="declared_utc")
        self.setUp()
        # Equal to a candidate's start is also not "earlier".
        self.refused_declaration(declaration(declared_utc="2001-09-09T01:47:40Z"), fragment="declared_utc")
        self.setUp()
        self.refused_declaration(declaration(declared_utc="2030-01-01T00:00:00Z"), fragment="declared_utc")

    def test_a_declaration_dated_before_every_candidate_is_accepted(self):
        self.write_declaration(declaration(declared_utc="2001-09-09T01:47:39Z"))
        base, cands = self.standard()
        self.assertEqual(self.check(base, cands)[0], 0)

    def test_an_unknown_key_is_refused_at_every_level(self):
        self.refused_declaration({**declaration(), "extra": 1}, fragment="extra")
        for path in ("treatment", "target"):
            self.setUp()
            value = declaration()
            value[path] = {**value[path], "extra": 1}
            self.refused_declaration(value, fragment="extra")

    def test_a_missing_key_is_refused(self):
        for key in ("schema", "declared_utc", "treatment", "target", "budget_is_subject"):
            with self.subTest(key=key):
                self.setUp()
                value = declaration()
                del value[key]
                self.refused_declaration(value, fragment=key)
        for path, key in (("treatment", "base"), ("treatment", "candidate"), ("target", "metric"),
                          ("target", "min_effect"), ("target", "assets")):
            with self.subTest(key=f"{path}.{key}"):
                self.setUp()
                value = declaration()
                del value[path][key]
                self.refused_declaration(value, fragment=key)

    def test_a_wrong_type_is_refused_and_says_which(self):
        def target(**over):
            return declaration(target={"metric": "strict_names", "min_effect": 1, "assets": None, **over})

        bad = [
            (declaration(schema=2), "schema must be"),
            (declaration(schema="1"), "schema must be"),
            (declaration(declared_utc=5), "declared_utc"),
            (declaration(declared_utc="yesterday"), "declared_utc"),
            (declaration(budget_is_subject="no"), "budget_is_subject"),
            (target(min_effect="5"), "min_effect"),
            (target(min_effect=0), "min_effect"),
            (target(min_effect=True), "min_effect"),
            (target(min_effect=1.5), "min_effect"),
            (target(metric="improved"), "target.metric"),
            (target(assets="passports/a.png"), "target.assets"),
            (target(assets=[1]), "target.assets"),
            (target(assets=[]), "target.assets"),
            (target(assets=["passports/a.png", "passports/a.png"]), "twice"),
            (declaration(treatment={"key": "ocr_arms.chargrid", "base": ["off"], "candidate": "on"}), "treatment.base"),
            (declaration(treatment={"key": "model_paths.detection", "base": "a", "candidate": "b"}), "treatment.key must start"),
            (declaration(treatment={"key": "pivot_yy.deeper", "base": 1, "candidate": 2}), "pivot_yy has no"),
            (declaration(treatment={"key": "", "base": "a", "candidate": "b"}), "treatment.key"),
            (declaration(treatment={"key": "ocr_arms..x", "base": "a", "candidate": "b"}), "treatment.key"),
            (declaration(treatment={"key": "ocr_arms.chargrid", "base": "on", "candidate": "on"}), "same value"),
            (target(metric="field_exact"), "needs target.field"),
            (target(field="surname"), "belongs to"),
            ([], "must be an object"),
        ]
        for index, (value, fragment) in enumerate(bad):
            with self.subTest(index=index, fragment=fragment):
                self.setUp()
                self.refused_declaration(value, fragment=fragment)

    def test_a_declaration_that_is_not_json_or_not_there_is_refused_without_its_content(self):
        self.refused_declaration(raw="{" + SENTINEL)
        self.setUp()
        base, cands = self.standard()
        self.assertRefused(self.check(base, cands))  # --declare names a file that does not exist
        self.setUp()
        base, cands = self.standard()
        result = self.check(base, cands, declare=False)
        self.assertEqual(result[0], 2)
        self.assertIn("--declare", result[2])

    def test_a_dotted_treatment_key_reaches_a_nested_value(self):
        self.write_declaration(declaration(treatment={"key": "retry_budget.max_seconds", "base": 520, "candidate": 600}))
        base = self.write_run(header(BASE, BASE_START_MS), base_docs())
        cands = [
            self.write_run(header(run_id, BASE_START_MS + 60_000 * (i + 1), "off",
                                  retry_budget={"max_passes": 14, "max_seconds": 600}), improved_docs(run_id))
            for i, run_id in enumerate((CAND_1, CAND_2))
        ]
        code, out, err = self.check(base, cands)
        self.assertEqual(code, 0, out + err)

    def test_an_unset_env_value_is_a_null_the_declaration_can_name(self):
        self.write_declaration(declaration(treatment={"key": "env.SYNTHPASS_OCR_CHARGRID", "base": None, "candidate": "on"}))
        env = {"RTEN_NUM_THREADS": "4"}
        base = self.write_run(header(BASE, BASE_START_MS, "off", env=env), base_docs())
        cands = [
            self.write_run(header(run_id, BASE_START_MS + 60_000 * (i + 1), "off",
                                  env={**env, "SYNTHPASS_OCR_CHARGRID": "on"}), improved_docs(run_id))
            for i, run_id in enumerate((CAND_1, CAND_2))
        ]
        code, out, err = self.check(base, cands)
        self.assertEqual(code, 0, out + err)


class InputTests(GateCase):
    def test_a_synthetic_run_is_refused(self):
        self.write_declaration()
        synthetic_head = header(BASE, BASE_START_MS, binary_name="synthpass-bench",
                                scope={"corpus": "synthetic-corpus", "document_type": "TD3", "profile": "clean",
                                       "seed_start": 0, "count": 1, "format": None, "limit": None})
        synthetic_record = {"kind": "doc", "run_id": BASE, "track": "synthetic", "format": "TD3", "profile": "clean",
                            "seed": 0, "ledger_row": {"hit": True}, "ocr": {"text": SENTINEL}}
        base = self.write_run(synthetic_head, [synthetic_record])
        _, cands = self.standard()
        self.assertRefused(self.check(base, cands), "synthetic")
        # A synthetic record inside a run whose header claims real specimens is refused too.
        self.setUp()
        self.write_declaration()
        mixed = self.write_run(header(BASE, BASE_START_MS), [synthetic_record | {"run_id": BASE}])
        _, cands = self.standard()
        self.assertRefused(self.check(mixed, cands), "synthetic")

    def test_a_synthetic_candidate_is_refused(self):
        self.write_declaration()
        base = self.write_run(header(BASE, BASE_START_MS), base_docs())
        head = header(CAND_1, BASE_START_MS + 60_000, "on", binary_name="synthpass-bench")
        cands = [self.write_run(head, improved_docs(CAND_1)),
                 self.write_run(header(CAND_2, BASE_START_MS + 120_000, "on"), improved_docs(CAND_2))]
        self.assertRefused(self.check(base, cands), "synthetic")

    def test_a_path_under_private_is_refused_and_never_read(self):
        self.write_declaration()
        base, cands = self.standard()
        private_dir = self.root / "private"
        private_dir.mkdir()
        copied = private_dir / base.name
        copied.write_text(base.read_text(encoding="utf-8"), encoding="utf-8")
        self.assertRefused(self.check(copied, cands), "private")

    def test_a_private_doc_record_in_a_public_file_is_not_a_document(self):
        self.write_declaration()
        base, cands = self.standard()
        with base.open("a", encoding="utf-8") as handle:
            handle.write(json.dumps({"kind": "private_doc", "run_id": BASE, "source_sha256": "7" * 64}) + "\n")
        code, out, err = self.check(base, cands)
        self.assertEqual(code, 0, out + err)

    def test_fewer_than_two_candidates_is_refused(self):
        self.write_declaration()
        base, cands = self.standard()
        self.assertRefused(self.check(base, cands[:1]), "at least two")

    def test_the_same_run_twice_is_refused(self):
        self.write_declaration()
        base, cands = self.standard()
        self.assertRefused(self.check(base, [cands[0], cands[0]]), "same run")
        self.assertRefused(self.check(base, [base, cands[0]]), "same run")

    def test_runs_are_named_by_a_run_id_prefix_in_the_root(self):
        self.write_declaration()
        self.standard()
        code, out, err = self.check(BASE[:12], [CAND_1[:12], CAND_2[:12]], "--root", str(self.root))
        self.assertEqual(code, 0, out + err)

    def test_a_missing_or_partial_run_is_refused(self):
        self.write_declaration()
        base, cands = self.standard()
        self.assertRefused(self.check("f" * 12, cands, "--root", str(self.root)))
        partial = self.write_run(header("e" * 64, BASE_START_MS + 180_000, "on"), improved_docs("e" * 64), partial=True)
        self.assertRefused(self.check(base, [cands[0], partial]))

    def test_the_local_track_is_read_with_the_flag(self):
        self.write_declaration()
        base = self.write_run(header(BASE, BASE_START_MS), base_docs(), track="local")
        cands = [self.write_run(header(run_id, BASE_START_MS + 60_000 * (i + 1), "on"), improved_docs(run_id), track="local")
                 for i, run_id in enumerate((CAND_1, CAND_2))]
        code, out, err = self.check(BASE[:12], [CAND_1[:12], CAND_2[:12]], "--root", str(self.root), "--track", "local")
        self.assertEqual(code, 0, out + err)
        self.assertEqual(self.check(BASE[:12], [CAND_1[:12], CAND_2[:12]], "--root", str(self.root))[0], 2,
                         "the same prefixes are not in public/")

    def test_a_file_in_the_other_track_directory_is_refused(self):
        self.write_declaration()
        base = self.write_run(header(BASE, BASE_START_MS), base_docs(), track="local")
        _, cands = self.standard()
        self.assertRefused(self.check(base, cands), "local")

    def test_a_schema_2_run_is_refused_with_a_file_name_only(self):
        self.write_declaration()
        base, cands = self.standard()
        odd = self.write_run(header("e" * 64, BASE_START_MS + 180_000, "on", schema=2), [])
        result = self.check(base, [cands[0], odd])
        self.assertRefused(result, "schema 2")


class DisclosureTests(GateCase):
    def assertQuiet(self, result):
        code, out, err = result
        self.assertNotIn(SENTINEL, out)
        self.assertNotIn(SENTINEL, err)
        return code

    def test_no_output_holds_a_planted_string_in_any_scenario(self):
        codes = set()
        # promotable
        self.write_declaration()
        base, cands = self.standard()
        codes.add(self.assertQuiet(self.check(base, cands)))
        # below target
        self.write_declaration(declaration(target={"metric": "strict_names", "min_effect": 5, "assets": None}))
        codes.add(self.assertQuiet(self.check(base, cands)))
        # vetoed: a hit is lost, and the lost record's `miss_reason` holds the sentinel
        self.setUp()
        self.write_declaration()
        lost = [improved_docs(CAND_1), improved_docs(CAND_2)]
        for run_id, records in zip((CAND_1, CAND_2), lost):
            records[2] = doc(run_id, ASSETS[2], outcome="no_mrz_found", names_exact=None, correctness=None)
        base, cands = self.standard(cand_records=lost)
        codes.add(self.assertQuiet(self.check(base, cands)))
        # vetoed: the repeats disagree in a text field
        self.setUp()
        self.write_declaration()
        second = improved_docs(CAND_2)
        second[0]["ocr"]["text"] = SENTINEL + "-DIFFERENT"
        base, cands = self.standard(cand_records=[improved_docs(CAND_1), second])
        codes.add(self.assertQuiet(self.check(base, cands)))
        # refused: models, argv-bearing headers, a bad declaration, a missing run
        self.setUp()
        self.write_declaration()
        other = header(CAND_1, BASE_START_MS + 60_000, "on", model_paths={"detection": SENTINEL + "-2"},
                       model_sha256={"detection": "9" * 64, "recognition": "2" * 64})
        base, cands = self.standard(cand_heads=[other, None])
        codes.add(self.assertQuiet(self.check(base, cands)))
        self.write_declaration(raw="[" + SENTINEL)
        codes.add(self.assertQuiet(self.check(base, cands)))
        self.write_declaration(declaration(), None)
        codes.add(self.assertQuiet(self.check(base, [cands[0], self.root / SENTINEL], "--root", str(self.root))))
        self.assertEqual(codes, {0, 2, 3})

    def test_a_malformed_record_line_is_named_by_line_number_only(self):
        self.write_declaration()
        base, cands = self.standard()
        with cands[0].open("a", encoding="utf-8") as handle:
            handle.write("{" + SENTINEL + "\n")
        result = self.check(base, cands)
        self.assertEqual(result[0], 2)
        self.assertNotIn(SENTINEL, result[1] + result[2])
        self.assertIn("line 5", result[1] + result[2])

    def test_an_error_names_a_file_and_never_its_directory(self):
        self.write_declaration()
        base, cands = self.standard()
        result = self.check(base, [cands[0], self.root / "public" / "nothing.jsonl"], "--root", str(self.root))
        self.assertEqual(result[0], 2)
        self.assertNotIn(str(self.root), result[1] + result[2])


class ReportTests(GateCase):
    def test_a_promotable_and_a_vetoed_report_read_as_evidence(self):
        code, out, _ = self.run_standard()
        self.assertEqual(code, 0)
        lines = out.splitlines()
        self.assertTrue(lines[0].startswith("promotion gate"), lines[0])
        for expected in ("declaration sha256", "treatment ocr_arms.chargrid: off -> on", "comparable: yes",
                         "vetoes: none", "target strict_names", "movers"):
            self.assertTrue(any(expected in line for line in lines), f"{expected!r} missing from\n{out}")
        self.setUp()
        def edit(records, run_id):
            records[0] = doc(run_id, ASSETS[0], names_exact=False)
        edited = [improved_docs(CAND_1), improved_docs(CAND_2)]
        for run_id, records in zip((CAND_1, CAND_2), edited):
            edit(records, run_id)
        code, out, _ = self.run_standard(cand_records=edited)
        self.assertEqual(code, 3)
        self.assertTrue(self.last_line(out).startswith("VETOED"))

    def test_the_mover_lines_are_capped_but_the_counts_are_not(self):
        total = pg.MOVER_LINE_CAP + 5
        assets = [f"passports/m{i:03}.png" for i in range(total)]
        shas = {asset: f"{i:064x}" for i, asset in enumerate(assets)}

        def records(run_id, exact):
            out = []
            for asset in assets:
                record = doc(run_id, ASSETS[0], names_exact=exact, asset_id=asset, source_sha256=shas[asset],
                             name=asset.rsplit("/", 1)[-1])
                record["ledger_row"]["asset_id"] = asset
                out.append(record)
            return out

        scope = dict(header(BASE, 0)["scope"], count=total)
        self.write_declaration(declaration(target={"metric": "strict_names", "min_effect": 1, "assets": None}))
        base = self.write_run(header(BASE, BASE_START_MS, scope=scope), records(BASE, False))
        cands = [self.write_run(header(run_id, BASE_START_MS + 60_000 * (i + 1), "on", scope=scope), records(run_id, True))
                 for i, run_id in enumerate((CAND_1, CAND_2))]
        code, out, err = self.check(base, cands)
        self.assertEqual(code, 0, out + err)
        self.assertIn(f"effect +{total}", out)
        shown = [line for line in out.splitlines() if "false -> true" in line and "passports/m" in line]
        self.assertEqual(len(shown), pg.MOVER_LINE_CAP * 2, "the cap applies to each candidate run")
        self.assertIn(f"{total - pg.MOVER_LINE_CAP} more", out)


class ReadOnlyTests(GateCase):
    def test_the_gate_writes_nothing(self):
        self.write_declaration()
        base, cands = self.standard()
        before = {p: p.read_bytes() for p in list(self.root.rglob("*.jsonl")) + [self.declare_path]}
        listing = sorted(p.name for p in self.root.rglob("*"))
        self.check(base, cands)
        self.assertEqual(before, {p: p.read_bytes() for p in before})
        self.assertEqual(listing, sorted(p.name for p in self.root.rglob("*")))


if __name__ == "__main__":
    unittest.main()
