#!/usr/bin/env python3
"""Score line-2 MRZ grid offsets against the geometric 18-cell template.

Reads only the classes and geometry in elimination_probe's cells.jsonl. This
is a standard-library report reader; it does not read images or run OCR.
"""

from __future__ import annotations

import json
import math
import sys
from collections import defaultdict
from pathlib import Path
from typing import Any, Iterable


SHAPES = {"2x44": (2, 44), "2x36": (2, 36), "3x30": (3, 30)}
CLASSES = {"digit", "letter", "K", "filler"}
TEMPLATE_A = (0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1)
TEMPLATE_B = (1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0)


class InputError(Exception):
    def __init__(self, line: int, problem: str):
        super().__init__(problem)
        self.line = line
        self.problem = problem


def _single_line(text: str) -> str:
    return " ".join(text.replace("\r", "\n").splitlines())


def _number(value: Any) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value)


def _integer(value: Any) -> bool:
    return isinstance(value, int) and not isinstance(value, bool)


def _pearson(a: list[float], b: tuple[int, ...]) -> float | None:
    if len(a) != len(b) or not a or max(a) == min(a) or max(b) == min(b):
        return None
    ma = math.fsum(a) / len(a)
    mb = math.fsum(b) / len(b)
    da = [x - ma for x in a]
    db = [x - mb for x in b]
    va = math.fsum(x * x for x in da)
    vb = math.fsum(x * x for x in db)
    if va == 0.0 or vb == 0.0:
        return None
    return math.fsum(x * y for x, y in zip(da, db)) / math.sqrt(va * vb)


def _step_is_expected(step: float | None, pitch_px: float) -> bool:
    return step is None or step < pitch_px


def _correlations(rises: list[float], template: tuple[int, ...]) -> list[float | None]:
    return [
        _pearson(rises[offset : offset + 18], template)
        for offset in range(len(rises) - 18 + 1)
    ]


def _summary(scores: list[float | None], true_offset: int) -> dict[str, Any]:
    true = scores[true_offset]
    flat = true is None
    if flat:
        return {"r_true": None, "rank": None, "hit": False, "flat": True,
                "margin": None, "adjacent_margin": None}
    others = [v for i, v in enumerate(scores) if i != true_offset and v is not None]
    rank = 1 + sum(v >= true for v in others)
    margin = true - max(others) if others else None
    adjacent = [scores[i] for i in (true_offset - 1, true_offset + 1)
                if 0 <= i < len(scores) and scores[i] is not None]
    adjacent_margin = true - max(adjacent) if adjacent else None
    return {"r_true": true, "rank": rank, "hit": rank == 1, "flat": False,
            "margin": margin, "adjacent_margin": adjacent_margin}


def _median(values: Iterable[float]) -> float | None:
    xs = sorted(values)
    if not xs:
        return None
    mid = len(xs) // 2
    return xs[mid] if len(xs) % 2 else (xs[mid - 1] + xs[mid]) / 2.0


def read_cells(path: str | Path) -> tuple[dict[str, Any], dict[tuple[str, int, float | None], list[dict[str, Any]]], dict[tuple[str, int], dict[str, Any]], int]:
    source = Path(path)
    try:
        stream = source.open("r", encoding="utf-8")
    except OSError as exc:
        raise InputError(1, f"cannot read file: {exc.strerror or 'I/O error'}") from None

    with stream:
        try:
            first = stream.readline()
        except UnicodeDecodeError:
            raise InputError(1, "file is not valid UTF-8") from None
        if first == "":
            raise InputError(1, "empty file")
        try:
            head_obj = json.loads(first)
        except (json.JSONDecodeError, ValueError):
            raise InputError(1, "invalid JSON header") from None
        if not isinstance(head_obj, dict) or set(head_obj) != {"header"} or not isinstance(head_obj["header"], dict):
            raise InputError(1, "expected elimination_probe header")
        header = head_obj["header"]
        if header.get("tool") != "elimination_probe":
            raise InputError(1, "expected elimination_probe header")
        for key in ("fractions", "steps_px_per_cell", "bins"):
            if key not in header:
                raise InputError(1, f"header missing {key}")
        fractions = header["fractions"]
        steps = header["steps_px_per_cell"]
        bin_count = header["bins"]
        if not isinstance(fractions, list) or not fractions or not all(_number(x) for x in fractions):
            raise InputError(1, "header fractions must be a non-empty array of finite numbers")
        if not isinstance(steps, list) or not all(_number(x) and x > 0 for x in steps):
            raise InputError(1, "header steps_px_per_cell must be an array of positive finite numbers")
        if len(set(steps)) != len(steps):
            raise InputError(1, "header steps_px_per_cell contains duplicates")
        if not _integer(bin_count) or bin_count <= 0:
            raise InputError(1, "header bins must be a positive integer")
        for key in ("main", "located", "documents"):
            if key not in header:
                raise InputError(1, f"header missing {key}")
        if (not isinstance(header["main"], str) or not header["main"] or "\n" in header["main"] or "\r" in header["main"]
                or not _integer(header["located"]) or not _integer(header["documents"])):
            raise InputError(1, "header main, located or documents has the wrong type")

        groups: dict[tuple[str, int, float | None], list[dict[str, Any]]] = defaultdict(list)
        line_info: dict[tuple[str, int], dict[str, Any]] = {}
        shapes_by_doc: dict[str, tuple[str, int]] = {}
        seen: set[tuple[str, int, float | None, int]] = set()
        group_first_line: dict[tuple[str, int, float | None], int] = {}
        skipped: set[tuple[str, int]] = set()
        required = {"document", "shape", "line", "column", "class", "step_px_per_cell",
                    "pitch_px", "top", "bottom", "rise", "bins", "left_quarter"}
        number = 2
        while True:
            try:
                raw = stream.readline()
            except UnicodeDecodeError:
                raise InputError(number, "row is not valid UTF-8") from None
            if raw == "":
                break
            try:
                row = json.loads(raw)
            except (json.JSONDecodeError, ValueError):
                raise InputError(number, "invalid JSON row") from None
            if not isinstance(row, dict):
                raise InputError(number, "row must be an object")
            missing = required - row.keys()
            if missing:
                raise InputError(number, f"row missing {sorted(missing)[0]}")
            doc, shape, line, col, cls = (row["document"], row["shape"], row["line"], row["column"], row["class"])
            if not isinstance(doc, str) or not doc or "\n" in doc or "\r" in doc:
                raise InputError(number, "document must be a non-empty string")
            if not isinstance(shape, str) or shape not in SHAPES:
                raise InputError(number, "unknown shape")
            nlines, width = SHAPES[shape]
            if not _integer(line) or not 1 <= line <= nlines:
                raise InputError(number, "line is outside the shape")
            if not _integer(col) or not 0 <= col < width:
                raise InputError(number, "column is outside the shape")
            if not isinstance(cls, str) or cls not in CLASSES:
                raise InputError(number, "unknown class")
            pitch = row["pitch_px"]
            if not _number(pitch) or pitch <= 0:
                raise InputError(number, "pitch_px must be a positive finite number")
            step = row["step_px_per_cell"]
            if step is not None:
                if not _number(step) or step not in steps:
                    raise InputError(number, "step_px_per_cell is not in the header")
                if step >= pitch:
                    raise InputError(number, "step_px_per_cell must be below pitch_px")
            for key in ("top", "bottom"):
                arr = row[key]
                if not isinstance(arr, list) or len(arr) != len(fractions) or not all(_number(x) for x in arr):
                    raise InputError(number, f"{key} must have one finite number per fraction")
            rise = row["rise"]
            if not isinstance(rise, list) or len(rise) != len(fractions) or not all(_number(x) for x in rise):
                raise InputError(number, "rise must have one finite number per fraction")
            bins = row["bins"]
            if not isinstance(bins, list) or len(bins) != bin_count or not all(_number(x) for x in bins):
                raise InputError(number, f"bins must have {bin_count} finite numbers")
            if not _number(row["left_quarter"]):
                raise InputError(number, "left_quarter must be a finite number")

            previous_shape = shapes_by_doc.setdefault(doc, (shape, number))[0]
            if previous_shape != shape:
                raise InputError(number, "document has two shapes")
            line_key = (doc, line)
            info = line_info.setdefault(line_key, {"shape": shape, "pitch_px": pitch, "classes": {}})
            if info["pitch_px"] != pitch:
                raise InputError(number, "line has two pitch_px values")
            old_class = info["classes"].setdefault(col, cls)
            if old_class != cls:
                raise InputError(number, "cell class changes across steps")
            group_key = (doc, line, step)
            dup = (*group_key, col)
            if dup in seen:
                raise InputError(number, "duplicate (document, line, step, column)")
            seen.add(dup)
            groups[group_key].append(row)
            group_first_line.setdefault(group_key, number)
            if line != 2:
                skipped.add(line_key)
            number += 1

        for key, cells in groups.items():
            width = SHAPES[line_info[key[:2]]["shape"]][1]
            columns = {r["column"] for r in cells}
            if columns != set(range(width)):
                raise InputError(group_first_line[key], f"(document, line, step) is missing columns for {key[0]} line {key[1]}")
        return header, dict(groups), line_info, len(skipped)


def analyze(header: dict[str, Any], groups: dict[tuple[str, int, float | None], list[dict[str, Any]]],
            line_info: dict[tuple[str, int], dict[str, Any]], skipped_lines: int) -> tuple[dict[str, Any], list[dict[str, Any]], list[dict[str, Any]], dict[str, Any]]:
    lines: list[dict[str, Any]] = []
    floors: list[dict[str, Any]] = []
    source_lines = sorted((key for key in line_info if key[1] == 2), key=lambda k: (k[0], k[1]))
    line2_by_shape = {shape: sum(1 for key in source_lines if line_info[key]["shape"] == shape)
                      for shape in SHAPES}
    fraction_values = header["fractions"]
    steps = header["steps_px_per_cell"]
    score_lookup: dict[tuple[str, int, float | None, int], dict[str, Any]] = {}
    expected_by_line: dict[tuple[str, int], list[float | None]] = {}

    for doc, line in source_lines:
        info = line_info[(doc, line)]
        shape = info["shape"]
        width = SHAPES[shape][1]
        template = TEMPLATE_B if shape == "3x30" else TEMPLATE_A
        true_offset = 0 if shape == "3x30" else 10
        classes = [info["classes"][i] for i in range(width)]
        expected_steps: list[float | None] = [None] + [s for s in steps if _step_is_expected(s, info["pitch_px"])]
        expected_by_line[(doc, line)] = expected_steps
        for fi, fraction in enumerate(fraction_values):
            by_step: list[tuple[float, bool]] = []
            for step in expected_steps:
                rows = groups.get((doc, line, step))
                result: dict[str, Any] | None = None
                if rows is not None:
                    ordered = sorted(rows, key=lambda r: r["column"])
                    rises = [float(r["rise"][fi]) for r in ordered]
                    scores = _correlations(rises, template)
                    result = _summary(scores, true_offset)
                    disagreeing = sum(
                        (want == 1 and classes[i] != "digit") or (want == 0 and classes[i] == "digit")
                        for i, want in enumerate(template, true_offset)
                    )
                    result.update({
                        "document": doc, "shape": shape, "line": line,
                        "step_px_per_cell": step, "fraction": fraction,
                        "disagreeing": disagreeing,
                    })
                    lines.append(result)
                    score_lookup[(doc, line, step, fi)] = result
                hit = bool(result and result["hit"])
                by_step.append((info["pitch_px"] if step is None else step, hit))
            reached: float | None = None
            for value, hit in by_step:
                if not hit:
                    break
                reached = value
            if reached is not None:
                floors.append({"document": doc, "line": line, "fraction": fraction, "floor": reached})

    lines.sort(key=lambda x: (x["document"], x["line"], -1 if x["step_px_per_cell"] is None else steps.index(x["step_px_per_cell"]), x["fraction"]))
    floors.sort(key=lambda x: (x["document"], x["line"], fraction_values.index(x["fraction"])))
    input_summary = {"main": header["main"], "located": header["located"], "documents": header["documents"],
                     "line2_by_shape": line2_by_shape, "skipped_lines": skipped_lines}
    return input_summary, lines, floors, {"source_lines": source_lines, "line_info": line_info,
                                         "expected_steps": steps, "fractions": fraction_values,
                                         "score_lookup": score_lookup, "expected_by_line": expected_by_line}


def _fmt(value: float | None) -> str:
    return "-" if value is None else f"{value:.3f}"


def markdown_report(header: dict[str, Any], summary: dict[str, Any], lines: list[dict[str, Any]],
                    floors: list[dict[str, Any]], details: dict[str, Any]) -> str:
    out = ["# Grid-offset recovery against the eighteen-cell template", "",
           f"- Main: `{header['main']}`", f"- Located documents: {header['located']} / {header['documents']}",
           f"- Line 2 counts: " + ", ".join(f"{k}: {v}" for k, v in summary["line2_by_shape"].items()),
           f"- Skipped lines 1 and 3: {summary['skipped_lines']}", ""]
    source_lines = details["source_lines"]
    step_values: list[float | None] = [None] + details["expected_steps"]
    for fi, fraction in enumerate(details["fractions"]):
        out.extend([f"## Fraction {fraction:.3f}", "", "| Step (px/cell) | Expected | Measured | Hits | Flat | Median margin | Min margin | Median adjacent margin |",
                    "| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |"])
        any_step = False
        for step in step_values:
            eligible = [key for key in source_lines if _step_is_expected(step, details["line_info"][key]["pitch_px"])]
            if not eligible:
                continue
            any_step = True
            measured = [details["score_lookup"].get((doc, line, step, fi)) for doc, line in eligible]
            values = [v for v in measured if v is not None]
            margins = [v["margin"] for v in values if v["margin"] is not None]
            adjacent = [v["adjacent_margin"] for v in values if v["adjacent_margin"] is not None]
            out.append(f"| {'native' if step is None else _fmt(step)} | {len(eligible)} | {len(values)} | "
                       f"{sum(v['hit'] for v in values)} | {sum(v['flat'] for v in values)} | "
                       f"{_fmt(_median(margins))} | {_fmt(min(margins) if margins else None)} | {_fmt(_median(adjacent))} |")
        if not any_step:
            out.append("| - | 0 | 0 | 0 | 0 | - | - | - |")
        out.extend(["", "### Floors", ""])
        ff = [f for f in floors if f["fraction"] == fraction]
        vals = [f["floor"] for f in ff]
        coarsest = min(ff, key=lambda f: (f["floor"], f["document"], f["line"])) if ff else None
        out.append(f"Lines with a floor: {len(ff)}; median floor: {_fmt(_median(vals))}; "
                   f"coarsest floor: {(_fmt(coarsest['floor']) + ' (' + coarsest['document'] + ', line ' + str(coarsest['line']) + ')') if coarsest else '-'}.")
        out.extend(["", "### Native misses", "", "| Document | Shape | Rank | Margin | Adjacent margin | Disagreeing |",
                    "| --- | --- | ---: | ---: | ---: | ---: |"])
        misses = [v for v in lines if v["fraction"] == fraction and v["step_px_per_cell"] is None and not v["hit"]]
        for v in misses:
            rank = "-" if v["rank"] is None else str(v["rank"])
            out.append(f"| {v['document']} | {v['shape']} | {rank} | {_fmt(v['margin'])} | {_fmt(v['adjacent_margin'])} | {v['disagreeing']} |")
        if not misses:
            out.append("| - | - | - | - | - | - |")
        out.append("")
    return "\n".join(out)


def _arguments(argv: list[str]) -> tuple[Path, bool] | None:
    json_output = False
    paths: list[str] = []
    for arg in argv:
        if arg == "--json" and not json_output:
            json_output = True
        elif arg.startswith("-"):
            return None
        else:
            paths.append(arg)
    if len(paths) != 1:
        return None
    return Path(paths[0]), json_output


def main(argv: list[str] | None = None) -> int:
    args = _arguments(sys.argv[1:] if argv is None else argv)
    if args is None:
        print("usage: elimination_offsets.py CELLS_JSONL [--json]", file=sys.stderr)
        return 2
    path, json_output = args
    try:
        header, groups, line_info, skipped = read_cells(path)
        summary, lines, floors, details = analyze(header, groups, line_info, skipped)
    except InputError as exc:
        print(f"{_single_line(str(path))}:{exc.line}: {_single_line(exc.problem)}", file=sys.stderr)
        return 2
    if json_output:
        print(json.dumps({"input": summary, "lines": lines, "floors": floors}, sort_keys=True,
                         separators=(",", ":"), allow_nan=False))
    else:
        sys.stdout.write(markdown_report(header, summary, lines, floors, details))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
