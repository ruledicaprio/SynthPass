"""Classify MRZ dump evidence without publishing any OCR or identity text.

Labels are string-level candidate explanations ranked by edit cost; not observed
mechanisms. A string comparison cannot show a physical cell shift. Per-character
OCR boxes or fitted grid-cell positions showing a glyph in a different cell
would supply that evidence (ADR-0015's detection layer); geometry is outside
this script. The input dump contains document text and must stay in gitignored
artifacts/. Output contains asset IDs, positions/counts, and labels only.
"""

from __future__ import annotations

import argparse
from collections import Counter
import json
from pathlib import Path
import re

OFF_DENOMINATOR = {"redacted_mrz", "no_mrz_expected", "checksum_failed_specimen"}
MRZ_ALPHABET = frozenset("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789<")
LABEL_ORDER = (
    "band location",
    "wrong line/attempt association",
    "format/window selection",
    "indel",
    "substitution",
    "inherited repair",
    "printed non-conformance",
)


def levenshtein(left: str, right: str) -> int:
    """Unit-cost distance; only integer evidence enters decisions."""
    previous = list(range(len(right) + 1))
    for i, a in enumerate(left, 1):
        current = [i]
        for j, b in enumerate(right, 1):
            current.append(
                min(previous[j] + 1, current[j - 1] + 1, previous[j - 1] + (a != b))
            )
        previous = current
    return previous[-1]


def best_with_indel(left: str, right: str) -> int:
    """Minimum unit-cost edit script containing at least one insert/delete."""
    rows, columns = len(left) + 1, len(right) + 1
    infinity = rows + columns
    without = [[infinity] * columns for _ in range(rows)]
    with_indel = [[infinity] * columns for _ in range(rows)]
    without[0][0] = 0
    for i in range(rows):
        for j in range(columns):
            if i < len(left) and j < len(right):
                cost = left[i] != right[j]
                without[i + 1][j + 1] = min(without[i + 1][j + 1], without[i][j] + cost)
                with_indel[i + 1][j + 1] = min(
                    with_indel[i + 1][j + 1], with_indel[i][j] + cost
                )
            if i < len(left):
                with_indel[i + 1][j] = min(
                    with_indel[i + 1][j], without[i][j] + 1, with_indel[i][j] + 1
                )
            if j < len(right):
                with_indel[i][j + 1] = min(
                    with_indel[i][j + 1], without[i][j] + 1, with_indel[i][j] + 1
                )
    return with_indel[-1][-1]


def candidate_lines(raw: str, truth: str) -> list[str]:
    """Keep MRZ-like input lines near the expected width; retain attempt order."""
    candidates = []
    for line in raw.splitlines():
        text = line.strip().upper().replace(" ", "")
        if not text or abs(len(text) - len(truth)) > 8:
            continue
        if 10 * sum(char in MRZ_ALPHABET for char in text) < 7 * len(text):
            continue
        candidates.append(text)
    return candidates


def line_distance(raw: str, truth: str) -> dict[str, int | str | None]:
    candidates = candidate_lines(raw, truth)
    if not candidates:
        return {"relation": "no_candidate", "levenshtein": None, "hamming": None,
                "indel_best": None}
    # Choose by edit distance, then input order. Equal-cost alternative lines
    # are not silently called the same observation.
    distances = [levenshtein(line, truth) for line in candidates]
    best = min(distances)
    best_lines = [line for line, distance in zip(candidates, distances) if distance == best]
    if len(set(best_lines)) > 1:
        return {"relation": "candidate_tie", "levenshtein": best, "hamming": None,
                "indel_best": None}
    candidate = best_lines[0]
    indel_best = best_with_indel(candidate, truth)
    if best == 0:
        return {"relation": "exact", "levenshtein": 0, "hamming": 0,
                "indel_best": indel_best}
    if len(candidate) != len(truth):
        return {"relation": "indel", "levenshtein": best, "hamming": None,
                "indel_best": indel_best}
    hamming = sum(a != b for a, b in zip(candidate, truth))
    if best < hamming:
        relation = "indel"
    elif indel_best == best:
        relation = "tie"
    else:
        relation = "substitution"
    return {"relation": relation, "levenshtein": best, "hamming": hamming,
            "indel_best": indel_best}


def truth_format(lines: list[str]) -> str | None:
    sizes = tuple(map(len, lines))
    if sizes == (30, 30, 30):
        return "TD1"
    if sizes == (36, 36):
        return "MRVB" if lines[0].startswith("V") else "TD2"
    if sizes == (44, 44):
        return "MRVA" if lines[0].startswith("V") else "TD3"
    return None


def classify_row(ledger: dict, dump: dict | None) -> dict:
    asset_id = ledger["asset_id"]
    outcome = ledger["outcome"]
    record = {
        "asset_id": asset_id,
        "outcome": outcome,
        "scored": outcome not in OFF_DENOMINATOR,
        "review_target": False,
        "truth_status": "not_in_dump" if dump is None else "unlabelled",
        "labels": [],
        "distance_relations": [],
        "evidence_status": "missing_dump" if dump is None else "available",
    }
    labels: set[str] = set()
    if outcome == "checksum_failed_specimen":
        labels.add("printed non-conformance")
    if dump is None:
        record["labels"] = sorted(labels, key=LABEL_ORDER.index)
        record["review_target"] = outcome not in OFF_DENOMINATOR and outcome != "hit"
        return record

    truth = dump.get("ground_truth_mrz")
    recovered = dump.get("recovered_mrz_lines") or []
    raw = dump.get("raw_ocr_text") or ""
    if outcome == "no_mrz_found" and dump.get("mrz_band_score") is None:
        labels.add("band location")
    if recovered and any(line not in raw for line in recovered):
        labels.add("inherited repair")

    if truth:
        record["truth_status"] = "labelled"
        true_lines = truth.splitlines()
        expected_format = truth_format(true_lines)
        if expected_format and ledger.get("mrz_format") not in (None, expected_format):
            labels.add("format/window selection")
        if len(true_lines) == len(recovered) and true_lines and true_lines[0] != recovered[0]:
            record["line1_error"] = True
        else:
            record["line1_error"] = False
        raw_lines = [line.strip().upper().replace(" ", "") for line in raw.splitlines()]
        if all(line in raw_lines for line in true_lines):
            contiguous = any(
                raw_lines[i : i + len(true_lines)] == true_lines
                for i in range(len(raw_lines) - len(true_lines) + 1)
            )
            if not contiguous:
                labels.add("wrong line/attempt association")
            if recovered != true_lines:
                labels.add("format/window selection")
        for line in true_lines:
            distance = line_distance(raw, line)
            record["distance_relations"].append(distance)
            if distance["relation"] == "indel":
                labels.add("indel")
            elif distance["relation"] == "substitution":
                labels.add("substitution")
            elif distance["relation"] == "tie":
                labels.update(("indel", "substitution"))
        if any(d["relation"] in ("tie", "candidate_tie") for d in record["distance_relations"]):
            record["distance_tie"] = True
    else:
        record["line1_error"] = None

    record["review_target"] = (
        outcome not in OFF_DENOMINATOR and outcome != "hit"
    ) or (outcome == "hit" and (
        ledger.get("names_exact") is False
        or ledger.get("name_error") is not None
        or record["line1_error"] is True
    ))
    record["labels"] = sorted(labels, key=LABEL_ORDER.index)
    return record


def classify(ledger_rows: list[dict], dump_rows: list[dict]) -> dict:
    ledger_by_id = {}
    for row in ledger_rows:
        asset_id = row.get("asset_id")
        if not asset_id or asset_id in ledger_by_id:
            raise ValueError("ledger requires one unique asset_id per document")
        ledger_by_id[asset_id] = row
    dump_by_id = {}
    for row in dump_rows:
        if row.get("provider") != "mrz":
            continue
        asset_id = row.get("asset_id")
        if not asset_id or asset_id in dump_by_id:
            raise ValueError("mrz dump requires one unique asset_id per document")
        if not re.fullmatch(r"[0-9a-f]{64}", row.get("source_sha256") or "") or not row.get("run_manifest"):
            raise ValueError(f"mrz dump metadata missing for asset_id {asset_id}")
        if asset_id not in ledger_by_id:
            raise ValueError(f"mrz dump asset_id absent from ledger: {asset_id}")
        dump_by_id[asset_id] = row
    records = [classify_row(ledger_by_id[asset_id], dump_by_id.get(asset_id))
               for asset_id in sorted(ledger_by_id)]
    scored = sum(record["scored"] for record in records)
    targets = [record for record in records if record["review_target"]]
    labels = Counter(label for record in targets for label in record["labels"])
    return {
        "summary": {
            "total_documents": len(records),
            "scored_documents": scored,
            "dumped_mrz_documents": len(dump_by_id),
            "review_targets": len(targets),
            "review_targets_with_missing_dump": sum(r["evidence_status"] == "missing_dump" for r in targets),
            "unlabelled_hits": sum(r["outcome"] == "hit" and r["truth_status"] == "unlabelled" for r in records),
            "distance_ties": sum(bool(r.get("distance_tie")) for r in targets),
            "labels_among_targets": {label: labels[label] for label in LABEL_ORDER},
        },
        "records": records,
    }


def read_jsonl(path: Path) -> list[dict]:
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dump", type=Path, required=True)
    parser.add_argument("--ledger", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    result = classify(read_jsonl(args.ledger), read_jsonl(args.dump))
    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps(result["summary"], sort_keys=True))


if __name__ == "__main__":
    main()
