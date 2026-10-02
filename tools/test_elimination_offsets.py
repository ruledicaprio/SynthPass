"""Hand-built schema/scoring tests for elimination_offsets (no specimen data)."""

from __future__ import annotations

import contextlib
import io
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import elimination_offsets as eo


HEADER = {
    "tool": "elimination_probe", "main": "abc123", "fractions": [0.02, 0.05],
    "steps_px_per_cell": [20.0, 16.0, 13.0, 11.0], "bins": 8,
    "grey": "max(R, G, B)", "documents": 2, "located": 2,
}


def motif(shape: str, offset: int, *, doc: str = "doc-A", line: int = 2,
          step: float | None = None, pitch: float = 22.0,
          disagree: bool = False, class_offset: int | None = None) -> list[dict]:
    width = eo.SHAPES[shape][1]
    template = eo.TEMPLATE_B if shape == "3x30" else eo.TEMPLATE_A
    rises = [0.0] * width
    classes = ["letter"] * width
    class_offset = offset if class_offset is None else class_offset
    for i, bit in enumerate(template):
        col = offset + i
        class_col = class_offset + i
        if bit:
            rises[col] = 0.06
            classes[class_col] = "digit"
        else:
            classes[class_col] = "letter"
    # Fillers have a lower rise, but remain part of the S (low) class.
    filler_col = class_offset if template[0] == 0 else class_offset + 15
    rises[filler_col] = -0.05
    classes[filler_col] = "filler"
    if disagree:
        classes[class_offset + 3] = "filler"  # T expected digit
        classes[class_offset + 16] = "digit"  # S expected non-digit
    rows = []
    for col in range(width):
        rows.append({
            "document": doc, "shape": shape, "line": line, "column": col,
            "class": classes[col], "step_px_per_cell": step, "pitch_px": pitch,
            "top": [0.1, 0.2], "bottom": [0.8, 0.7],
            "rise": [rises[col], rises[col]], "bins": [0.125] * 8,
            "left_quarter": 0.25,
        })
    return rows


def write_input(path: Path, rows: list[dict], header: dict | None = None) -> Path:
    head = HEADER if header is None else header
    path.write_text("\n".join([json.dumps({"header": head})] + [json.dumps(r) for r in rows]) + "\n", encoding="utf-8")
    return path


def read_analyze(path: Path, rows: list[dict], header: dict | None = None):
    write_input(path, rows, header)
    h, groups, info, skipped = eo.read_cells(path)
    return eo.analyze(h, groups, info, skipped)


class EliminationOffsetTests(unittest.TestCase):
    def test_template_a_hits_2x44_and_2x36_and_shift_misses(self):
        for shape in ("2x44", "2x36"):
            width = eo.SHAPES[shape][1]
            values = [r["rise"][0] for r in motif(shape, 10)]
            true = eo._summary(eo._correlations(values, eo.TEMPLATE_A), 10)
            self.assertTrue(true["hit"])
            self.assertEqual(true["rank"], 1)
            self.assertGreater(true["margin"], 0)
            moved = [r["rise"][0] for r in motif(shape, 11)]
            shifted = eo._summary(eo._correlations(moved, eo.TEMPLATE_A), 10)
            self.assertFalse(shifted["hit"])
            self.assertGreaterEqual(shifted["rank"], 2)
            self.assertLess(shifted["margin"], 0)
            self.assertEqual(len(values), width)

    def test_template_b_shape_map_and_wrong_template(self):
        rows = motif("3x30", 0)
        rises = [r["rise"][0] for r in rows]
        with tempfile.TemporaryDirectory() as td:
            _, scored, _, _ = read_analyze(Path(td) / "cells.jsonl", rows)
        good = next(x for x in scored if x["fraction"] == HEADER["fractions"][0])
        wrong = eo._summary(eo._correlations(rises, eo.TEMPLATE_A), 10)
        self.assertTrue(good["hit"])
        self.assertFalse(wrong["hit"])

    def test_equal_true_window_is_flat(self):
        scores = eo._correlations([0.1] * 44, eo.TEMPLATE_A)
        result = eo._summary(scores, 10)
        self.assertTrue(result["flat"])
        self.assertFalse(result["hit"])
        self.assertIsNone(result["rank"])

    def test_tie_for_top_ranks_two(self):
        result = eo._summary([1.0, 1.0, 0.1], 0)
        self.assertEqual(result["rank"], 2)
        self.assertFalse(result["hit"])

    def test_correlation_is_affine_invariant_and_negation_misses(self):
        base = [r["rise"][0] for r in motif("2x44", 10)]
        reference = eo._summary(eo._correlations(base, eo.TEMPLATE_A), 10)
        scaled = [3 * x + 0.2 for x in base]
        transformed = eo._summary(eo._correlations(scaled, eo.TEMPLATE_A), 10)
        self.assertAlmostEqual(reference["r_true"], transformed["r_true"], places=12)
        self.assertEqual((reference["rank"], reference["hit"]), (transformed["rank"], transformed["hit"]))
        reversed_result = eo._summary(eo._correlations([-x for x in base], eo.TEMPLATE_A), 10)
        self.assertFalse(reversed_result["hit"])

    def test_floor_stops_on_first_miss(self):
        rows = []
        rows += motif("2x44", 10, step=None)
        rows += motif("2x44", 10, step=20.0)
        rows += motif("2x44", 10, step=16.0)
        rows += motif("2x44", 11, step=13.0, class_offset=10)
        rows += motif("2x44", 10, step=11.0)
        with tempfile.TemporaryDirectory() as td:
            summary, lines, floors, _ = read_analyze(Path(td) / "cells.jsonl", rows)
        self.assertEqual(floors[0]["floor"], 16.0)
        self.assertEqual(len(lines), 5 * 2)

    def test_nonexpected_steps_are_ignored_and_missing_step_breaks_floor(self):
        rows = motif("2x44", 10, pitch=12.0)
        rows += motif("2x44", 10, step=11.0, pitch=12.0)
        with tempfile.TemporaryDirectory() as td:
            _, lines, floors, details = read_analyze(Path(td) / "standard.jsonl", rows)
            summary = {"line2_by_shape": {"2x44": 1, "2x36": 0, "3x30": 0}, "skipped_lines": 0}
            report = eo.markdown_report(HEADER, summary, lines, floors, details)
        self.assertEqual(len(lines), 4)
        self.assertEqual(floors[0]["floor"], 11.0)
        self.assertEqual(details["expected_by_line"][("doc-A", 2)], [None, 11.0])
        self.assertIn("| 11.000 |", report)
        for step in (20, 16, 13):
            self.assertNotIn(f"| {step}.000 |", report)

        rows = motif("2x44", 10, step=None) + motif("2x44", 10, step=20.0)
        rows += motif("2x44", 10, step=13.0) + motif("2x44", 10, step=11.0)
        with tempfile.TemporaryDirectory() as td:
            _, _, floors, _ = read_analyze(Path(td) / "gap.jsonl", rows)
        self.assertEqual(floors[0]["floor"], 20.0)

    def test_native_miss_has_no_floor_and_disagreeing_and_skipped_lines(self):
        rows = motif("2x44", 11, doc="doc-A", line=2)
        rows += motif("2x44", 10, doc="doc-A", line=1)
        rows += motif("3x30", 1, doc="doc-B", line=2, disagree=True, class_offset=0)
        rows += motif("3x30", 0, doc="doc-B", line=1) + motif("3x30", 0, doc="doc-B", line=3)
        with tempfile.TemporaryDirectory() as td:
            summary, lines, floors, details = read_analyze(Path(td) / "cells.jsonl", rows)
            report = eo.markdown_report(HEADER, summary, lines, floors, details)
        self.assertEqual(floors, [])
        self.assertEqual(summary["skipped_lines"], 3)
        native = next(x for x in lines if x["document"] == "doc-B" and x["line"] == 2)
        self.assertEqual(native["disagreeing"], 2)
        for miss in (x for x in lines if x["step_px_per_cell"] is None):
            self.assertIn(f"| {miss['document']} | {miss['shape']} | {miss['rank']} |", report)
            self.assertNotIn(f"| {miss['document']} | {miss['shape']} | {miss['rank']}.000 |", report)

    def test_report_json_and_even_median(self):
        rows = motif("2x44", 10, doc="doc-B") + motif("3x30", 0, doc="doc-A")
        with tempfile.TemporaryDirectory() as td:
            path = write_input(Path(td) / "cells.jsonl", rows)
            header, groups, info, skipped = eo.read_cells(path)
            summary, lines, floors, details = eo.analyze(header, groups, info, skipped)
            markdown = eo.markdown_report(header, summary, lines, floors, details)
            self.assertIn("## Fraction 0.020", markdown)
            self.assertIn("| Step (px/cell) | Expected | Measured | Hits | Flat |", markdown)
            self.assertIn("| native | 2 | 2 | 2 | 0 |", markdown)
            self.assertIn("coarsest floor:", markdown)
            self.assertIn("doc-A", markdown)
            self.assertAlmostEqual(eo._median([1.0, 3.0]), 2.0)
            stdout = io.StringIO()
            with contextlib.redirect_stdout(stdout):
                self.assertEqual(eo.main([str(path), "--json"]), 0)
            parsed = json.loads(stdout.getvalue())
        self.assertEqual(set(parsed), {"input", "lines", "floors"})
        self.assertEqual(set(parsed["lines"][0]), {"document", "shape", "line", "step_px_per_cell", "fraction", "r_true", "rank", "hit", "flat", "margin", "adjacent_margin", "disagreeing"})
        self.assertEqual(parsed["input"]["line2_by_shape"]["2x44"], 1)
        self.assertEqual([x["document"] for x in parsed["lines"][:2]], ["doc-A", "doc-A"])

    def test_bins_follow_positive_header_count(self):
        header = HEADER | {"bins": 3}
        rows = motif("2x44", 10)
        for row in rows:
            row["bins"] = [0.25, 0.5, 0.25]
        with tempfile.TemporaryDirectory() as td:
            path = write_input(Path(td) / "three-bins.jsonl", rows, header)
            eo.read_cells(path)
            rows[0]["bins"].append(0.0)
            write_input(path, rows, header)
            with self.assertRaises(eo.InputError) as raised:
                eo.read_cells(path)
        self.assertEqual(raised.exception.problem, "bins must have 3 finite numbers")

    def test_validation_errors_exit_two_once_without_traceback(self):
        base = motif("2x44", 10)
        cases: list[tuple[str, str, list[str] | None, dict | None, str]] = []
        cases.append(("empty", "", None, None, "empty file"))
        cases.append(("wrong-header", json.dumps({"header": {"tool": "other"}}) + "\n", None, None,
                      "expected elimination_probe header"))
        cases.append(("header-array", json.dumps({"header": {"tool": "elimination_probe", "steps_px_per_cell": []}}) + "\n", None, None,
                      "header missing fractions"))
        cases.append(("header-steps", json.dumps({"header": {"tool": "elimination_probe", "fractions": [0.02]}}) + "\n", None, None,
                      "header missing steps_px_per_cell"))
        missing_bins = dict(HEADER)
        del missing_bins["bins"]
        cases.append(("header-missing-bins", json.dumps({"header": missing_bins}) + "\n", None, None,
                      "header missing bins"))
        cases.append(("header-bins", json.dumps({"header": HEADER | {"bins": 0}}) + "\n", None, None,
                      "header bins must be a positive integer"))
        cases.append(("bad-json", json.dumps({"header": HEADER}) + "\n{bad\n", None, None, "invalid JSON row"))
        cases.append(("invalid-utf8", "\xff", None, None, "file is not valid UTF-8"))
        missing = [dict(r) for r in base]
        del missing[0]["class"]
        cases.append(("missing-key", "", missing, None, "row missing class"))
        wrong_type = [dict(r) for r in base]
        wrong_type[0]["pitch_px"] = "22"
        cases.append(("wrong-type", "", wrong_type, None, "pitch_px must be a positive finite number"))
        for field, bad_value, problem in (
                ("document", 7, "document must be a non-empty string"),
                ("line", "2", "line is outside the shape"),
                ("column", False, "column is outside the shape"),
                ("step_px_per_cell", True, "step_px_per_cell is not in the header"),
                ("top", "0.1", "top must have one finite number per fraction"),
                ("bottom", None, "bottom must have one finite number per fraction"),
                ("rise", 0.1, "rise must have one finite number per fraction"),
                ("bins", "bins", "bins must have 8 finite numbers"),
                ("left_quarter", "0.25", "left_quarter must be a finite number")):
            malformed = [dict(r) for r in base]
            malformed[0][field] = bad_value
            cases.append((f"type-{field}", "", malformed, None, problem))
        bad_rise = [dict(r) for r in base]
        bad_rise[0]["rise"] = [0.0]
        cases.append(("rise-length", "", bad_rise, None, "rise must have one finite number per fraction"))
        bad_shape = [dict(r) for r in base]
        bad_shape[0]["shape"] = []
        cases.append(("shape", "", bad_shape, None, "unknown shape"))
        bad_class = [dict(r) for r in base]
        bad_class[0]["class"] = "punctuation"
        cases.append(("class", "", bad_class, None, "unknown class"))
        bad_step = [dict(r) for r in base]
        bad_step[0]["step_px_per_cell"] = 7.0
        cases.append(("unknown-step", "", bad_step, None, "step_px_per_cell is not in the header"))
        high_step = [dict(r) for r in motif("2x44", 10, step=20.0, pitch=20.0)]
        cases.append(("step-at-pitch", "", high_step, None, "step_px_per_cell must be below pitch_px"))
        bad_col = [dict(r) for r in base]
        bad_col[0]["column"] = 44
        cases.append(("column", "", bad_col, None, "column is outside the shape"))
        cases.append(("duplicate", "", base + [dict(base[0])], None,
                      "duplicate (document, line, step, column)"))
        cases.append(("missing-column", "", base[:-1], None,
                      "(document, line, step) is missing columns for doc-A line 2"))
        cases.append(("two-shapes", "", base + motif("3x30", 0), None, "document has two shapes"))
        two_pitch = base + motif("2x44", 10, step=20.0, pitch=21.0)
        two_pitch[-1]["pitch_px"] = 23.0
        cases.append(("two-pitches", "", two_pitch, None, "line has two pitch_px values"))
        with tempfile.TemporaryDirectory() as td:
            for label, raw, rows, header, expected_problem in cases:
                with self.subTest(label=label):
                    path = Path(td) / f"{label}.jsonl"
                    if raw == "\xff":
                        path.write_bytes(b"\xff\n")
                    elif raw:
                        path.write_text(raw, encoding="utf-8")
                    elif label == "empty":
                        path.write_text("", encoding="utf-8")
                    else:
                        write_input(path, rows or [], header)
                    proc = subprocess.run([sys.executable, str(Path(eo.__file__)), str(path), "--json"], text=True, capture_output=True)
                    self.assertEqual(proc.returncode, 2, proc.stdout + proc.stderr)
                    self.assertEqual(len(proc.stderr.splitlines()), 1, proc.stderr)
                    self.assertIn(str(path), proc.stderr)
                    self.assertNotIn("Traceback", proc.stderr)
                    location, separator, problem = proc.stderr.rstrip().removeprefix(f"{path}:").partition(": ")
                    self.assertTrue(separator, proc.stderr)
                    self.assertTrue(location.isdigit(), proc.stderr)
                    self.assertEqual(problem, expected_problem)

    def test_cli_usage_and_file_errors_are_single_line(self):
        for argv in ([], ["a", "b"], ["--bad", "a"]):
            err = io.StringIO()
            with contextlib.redirect_stderr(err):
                self.assertEqual(eo.main(argv), 2)
            self.assertEqual(len(err.getvalue().splitlines()), 1)
            self.assertTrue(err.getvalue().startswith("usage:"))
        err = io.StringIO()
        with contextlib.redirect_stderr(err):
            self.assertEqual(eo.main(["does-not-exist.jsonl"]), 2)
        self.assertEqual(len(err.getvalue().splitlines()), 1)
        self.assertIn(":1:", err.getvalue())


if __name__ == "__main__":
    unittest.main()
