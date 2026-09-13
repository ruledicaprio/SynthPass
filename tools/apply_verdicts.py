#!/usr/bin/env python3
"""
apply_verdicts.py

Syncs a `verdicts.json` saved from the HTML review artifact
(`build_review_artifact.py`) into the **Verdict** column of the plan's
official `packet-cNN.md` file (P-PACKET, plan §6.4), leaving every other
column untouched. `packet-cNN.md` stays the single record P-APPLY-VERDICTS
(plan §6.5) reads -- this script is just a typing-saver for filling its last
column, not a new source of truth.

Matches a verdicts entry to a packet row by looking for the row's candidate
id (the staged file's sha12 stem, e.g. `5784cf79a00b`) inside that row's own
Markdown line -- it appears in the "Local path" column's link text and URL.
A row with no id match, or an id with no verdict recorded yet, is left
exactly as it was (blank), so partial review sessions stay safe to re-apply.

Standard library only. Idempotent: re-running with the same verdicts.json
against an already-updated packet is a no-op.

Usage:
    python tools/apply_verdicts.py --packet work/scouting/c01/packet-c01.md \\
        --verdicts work/scouting/c01/verdicts-c01.json [--dry-run]
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def apply(packet_text: str, verdicts: dict[str, dict]) -> tuple[str, list[str]]:
    lines = packet_text.splitlines(keepends=True)
    applied: list[str] = []
    out_lines: list[str] = []

    for line in lines:
        stripped = line.rstrip("\n")
        matched_id = None
        if stripped.strip().startswith("|") and stripped.rstrip().endswith("|"):
            for candidate_id in verdicts:
                if candidate_id in stripped:
                    matched_id = candidate_id
                    break

        if matched_id is None:
            out_lines.append(line)
            continue

        verdict = verdicts[matched_id].get("verdict")
        if not verdict:
            out_lines.append(line)
            continue

        row = stripped.rstrip()
        last_pipe = len(row) - 1
        second_last_pipe = row.rfind("|", 0, last_pipe)
        if second_last_pipe == -1:
            out_lines.append(line)
            continue

        new_row = f"{row[:second_last_pipe + 1]} {verdict} {row[last_pipe:]}"
        newline = line[len(stripped):] or "\n"
        out_lines.append(new_row + newline)
        applied.append(f"{matched_id} -> {verdict}")

    return "".join(out_lines), applied


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--packet", required=True, type=Path)
    parser.add_argument("--verdicts", required=True, type=Path)
    parser.add_argument("--dry-run", action="store_true", help="print what would change, write nothing")
    args = parser.parse_args()

    verdicts = json.loads(args.verdicts.read_text(encoding="utf-8"))
    packet_text = args.packet.read_text(encoding="utf-8")

    new_text, applied = apply(packet_text, verdicts)

    if not applied:
        print("no matching unset rows found -- nothing to apply")
        return

    print(f"{len(applied)} row(s) to update:")
    for line in applied:
        print(f"  {line}")

    if args.dry_run:
        print("--dry-run: not written")
        return

    args.packet.write_text(new_text, encoding="utf-8")
    print(f"wrote {args.packet}")


if __name__ == "__main__":
    main()
