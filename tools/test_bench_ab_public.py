"""Public A/B safety tests using hand-built rows only."""
import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

import bench_ab_args as a
import bench_ab_diff as d
from test_bench_ab_args import SHA, context, inputs


LEDGER = Path(__file__).resolve().parent.parent / "knowledge" / "benchmarks" / "real-specimen-outcomes.jsonl"
LEDGER_ROWS = 40


def row():
    return {"asset_id": "public-doc", "mrz_format": "TD3", "outcome": "hit",
            "miss_reason": None, "check_states": {"composite": True},
            "names_exact": True}


class PublicArm(unittest.TestCase):
    def test_synthetic_plan_is_unchanged(self):
        self.assertEqual(a.validate(inputs()), a.validate(inputs(AB_CORPUS="synthetic")))
        self.assertNotIn("corpus", a.validate(inputs()))

    def test_tampered_public_plan_is_refused(self):
        plan = a.validate(inputs(AB_CORPUS="public"))
        plan["count"] = 1
        with self.assertRaises(a.Refused):
            a.check_plan(plan)

    def test_invalid_columns_never_echo_text(self):
        for key, value in (("outcome", "SECRET-123456"), ("names_exact", "SECRET-123456"),
                           ("check_states", {"SECRET-123456": True}),
                           ("mrz_format", "SECRET-123456")):
            with self.subTest(key=key), tempfile.TemporaryDirectory() as tmp:
                before, after = Path(tmp)/"before", Path(tmp)/"after"
                for directory in (before, after):
                    directory.mkdir()
                    (directory/d.PUBLIC_OUTCOMES).write_text(json.dumps(row() | {key: value})+"\n")
                output = io.StringIO()
                with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                    self.assertEqual(d.main([str(before),str(after),"--public"]),2)
                self.assertNotIn("SECRET-123456", output.getvalue())

    def test_corpus_grammar(self):
        self.assertEqual(a.validate(inputs(AB_CORPUS="public"))["corpus"], "public")
        for corpus in ("private", "local", "PUBLIC", "public\n"):
            with self.subTest(corpus=corpus), self.assertRaises(a.Refused):
                a.validate(inputs(AB_CORPUS=corpus))

    def test_public_refuses_synthetic_overrides(self):
        for key, value in (("FORMATS", "td3"), ("PROFILE", "all"),
                           ("COUNT", "20"), ("SEED", "1")):
            with self.subTest(key=key), self.assertRaises(a.Refused):
                a.validate(inputs(AB_CORPUS="public", **{f"AB_{key}": value}))

    def test_public_run_discards_console_text(self):
        plan = a.validate(inputs(AB_CORPUS="public"))
        with tempfile.TemporaryDirectory() as tmp, mock.patch.object(a.subprocess, "run") as run:
            run.return_value.returncode = 0
            a.run_arm(plan, "before", None, Path(tmp)/"binary", Path(tmp), Path(tmp)/"out")
            argv = run.call_args.args[0]
            self.assertIn("--write-text-free-outcomes", argv)
            self.assertNotIn("--dump-ocr", argv)
            self.assertEqual(run.call_args.kwargs["stdout"], a.subprocess.DEVNULL)
            self.assertEqual(run.call_args.kwargs["stderr"], a.subprocess.DEVNULL)

    def test_projection_ignores_planted_text(self):
        with tempfile.TemporaryDirectory() as tmp:
            before, after = Path(tmp)/"before", Path(tmp)/"after"
            for directory in (before, after):
                directory.mkdir()
                planted = row() | {"miss_reason": "document number SECRET-123456",
                                   "ocr_text": "SECRET-123456",
                                   "future_column": "SECRET-123456" if directory == before else "OTHER-SECRET"}
                (directory/d.PUBLIC_OUTCOMES).write_text(json.dumps(planted)+"\n")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                code = d.main([str(before), str(after), "--public", "--json"])
            self.assertEqual(code, 0)
            self.assertNotIn("SECRET-123456", output.getvalue())
            self.assertNotIn("future_column", output.getvalue())

    def test_committed_ledger_rows_read_as_a_neutral_projection(self):
        """Real column names, from the committed ledger: `text_free_projection` sets each
        `miss_reason` to its `outcome`, and the diff must accept every row that results."""
        with LEDGER.open(encoding="utf-8") as ledger:
            source = [json.loads(line) for _, line in zip(range(LEDGER_ROWS), ledger)]
        self.assertEqual(len(source), LEDGER_ROWS)
        texts = {r["miss_reason"] for r in source if r["miss_reason"] and r["miss_reason"] != r["outcome"]}
        self.assertTrue(texts, "the fixture rows must carry miss_reason text for this test to bite")
        projected = [r | {"miss_reason": r["outcome"]} for r in source]
        with tempfile.TemporaryDirectory() as tmp:
            before, after = Path(tmp)/"before", Path(tmp)/"after"
            for directory in (before, after):
                directory.mkdir()
                lines = [json.dumps(r) + "\n" for r in projected]
                (directory/d.PUBLIC_OUTCOMES).write_text("".join(lines), encoding="utf-8")
            for extra in ([], ["--json"]):
                with self.subTest(extra=extra):
                    output = io.StringIO()
                    with contextlib.redirect_stdout(output), contextlib.redirect_stderr(output):
                        code = d.main([str(before), str(after), "--public", "--expect-identical", *extra])
                    self.assertEqual(code, 0, output.getvalue())
                    for text in texts:
                        self.assertNotIn(text, output.getvalue())
                    if not extra:
                        self.assertIn(f"documents [{LEDGER_ROWS}, {LEDGER_ROWS}]", output.getvalue())
                        self.assertIn("NEUTRAL", output.getvalue())

    def test_public_run_names_the_binary_that_ran(self):
        plan = a.validate(inputs(AB_CORPUS="public"))
        with tempfile.TemporaryDirectory() as tmp:
            plan_path = Path(tmp)/"plan.json"
            plan_path.write_text(json.dumps(plan), encoding="utf-8")
            output = io.StringIO()
            with mock.patch.object(a, "run_arm", return_value=0), contextlib.redirect_stdout(output):
                code = a.main(["run", "--plan", str(plan_path), "--role", "before",
                               "--binary", str(Path(tmp)/"bin"/"provider-bench"),
                               "--cwd", tmp, "--out-dir", str(Path(tmp)/"out")])
        self.assertEqual(code, 0)
        self.assertIn("bench-ab: before: provider-bench exited 0", output.getvalue())
        self.assertNotIn("None", output.getvalue())

    def test_public_summary_states_the_corpus_not_synthetic_parameters(self):
        plan = a.validate(inputs(AB_CORPUS="public"))
        with tempfile.TemporaryDirectory() as tmp:
            binary = Path(tmp)/"provider-bench"
            binary.write_bytes(b"x")
            before = a.build_arm(plan, "before", SHA, binary, context())
            after = a.build_arm(plan, "after", "c" * 40, binary, context())
            text = " | ".join(a.summary_lines(plan, before, after))
        self.assertIn("corpus public; expect_identical false", text)
        for synthetic in ("formats:", "profile", "documents per format", "seed"):
            self.assertNotIn(synthetic, text)

    def test_per_asset_diff_and_neutral_verdict(self):
        with tempfile.TemporaryDirectory() as tmp:
            before, after = Path(tmp)/"before", Path(tmp)/"after"
            for directory in (before, after):
                directory.mkdir()
                (directory/d.PUBLIC_OUTCOMES).write_text(json.dumps(row())+"\n")
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(d.main([str(before),str(after),"--public","--expect-identical"]),0)
            changed = row() | {"names_exact": False}
            (after/d.PUBLIC_OUTCOMES).write_text(json.dumps(changed)+"\n")
            output = io.StringIO()
            with contextlib.redirect_stdout(output):
                self.assertEqual(d.main([str(before),str(after),"--public","--expect-identical"]),3)
            self.assertIn("public-doc",output.getvalue())
            self.assertIn("names_exact",output.getvalue())
