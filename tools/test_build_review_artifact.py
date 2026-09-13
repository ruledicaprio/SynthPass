#!/usr/bin/env python3
"""Offline unit tests for build_review_artifact.py and apply_verdicts.py.

Run: python -m unittest tools/test_build_review_artifact.py
"""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

import apply_verdicts
import build_review_artifact as bra

MINIMAL_ROW = {
    "id": "aaaaaaaaaaaa",
    "code": "XXX",
    "doc_td": "passport / TD3",
    "series": "2020",
    "host": "example.gov",
    "licence_evidence": "none stated",
    "specimen_signal": "page-says-specimen",
    "mrz_check": "invalid",
    "proposed_destination": "local",
    "proposed_licence": "none-stated",
    "local_path": "does/not/exist.jpg",
    "note": 'a "quoted" <note> & more',
}


class LoadRowsTests(unittest.TestCase):
    def test_later_packet_overrides_earlier_row_with_same_id(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            first = tmp_path / "packet-a.json"
            second = tmp_path / "packet-b.json"
            row_v1 = dict(MINIMAL_ROW, note="v1")
            row_v2 = dict(MINIMAL_ROW, note="v2")
            first.write_text(json.dumps([row_v1]), encoding="utf-8")
            second.write_text(json.dumps([row_v2]), encoding="utf-8")

            rows = bra.load_rows([first, second])

            self.assertEqual(len(rows), 1)
            self.assertEqual(rows["aaaaaaaaaaaa"]["note"], "v2")

    def test_missing_field_raises(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "packet.json"
            bad_row = dict(MINIMAL_ROW)
            del bad_row["mrz_check"]
            path.write_text(json.dumps([bad_row]), encoding="utf-8")

            with self.assertRaises(ValueError):
                bra.load_rows([path])


class EmbedImageTests(unittest.TestCase):
    def test_missing_image_falls_back_to_placeholder_not_a_crash(self):
        with tempfile.TemporaryDirectory() as tmp:
            uri = bra.embed_image(Path(tmp), "nowhere/nothing.jpg")
            self.assertTrue(uri.startswith("data:image/svg+xml,"))

    def test_present_image_is_base64_data_uri(self):
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            img = tmp_path / "x.png"
            img.write_bytes(b"\x89PNG\r\n\x1a\nnot a real png but bytes")
            uri = bra.embed_image(tmp_path, "x.png")
            self.assertTrue(uri.startswith("data:image/png;base64,"))


class EscTests(unittest.TestCase):
    def test_html_special_characters_are_escaped(self):
        out = bra.esc(MINIMAL_ROW["note"])
        self.assertNotIn("<note>", out)
        self.assertIn("&lt;note&gt;", out)


class ApplyVerdictsTests(unittest.TestCase):
    PACKET = (
        "# Packet\n\n"
        "| # | Code | Local path | Note | Verdict |\n"
        "|---|---|---|---|---|\n"
        "| 1 | LVA | [aaaaaaaaaaaa.jpg](x/aaaaaaaaaaaa.jpg) | some note | |\n"
        "| 2 | SMR | [bbbbbbbbbbbb.jpg](x/bbbbbbbbbbbb.jpg) | other note | |\n"
    )

    def test_only_matching_ids_with_a_verdict_are_updated(self):
        verdicts = {"aaaaaaaaaaaa": {"verdict": "local"}}
        new_text, applied = apply_verdicts.apply(self.PACKET, verdicts)

        self.assertEqual(applied, ["aaaaaaaaaaaa -> local"])
        self.assertIn("[aaaaaaaaaaaa.jpg]", new_text.splitlines()[4])
        self.assertTrue(new_text.splitlines()[4].rstrip().endswith("| local |"))
        # row 2 (no verdict recorded) stays exactly as it was, blank Verdict cell
        self.assertTrue(new_text.splitlines()[5].rstrip().endswith("| |"))

    def test_id_present_but_no_verdict_recorded_leaves_row_untouched(self):
        verdicts = {"aaaaaaaaaaaa": {}}
        new_text, applied = apply_verdicts.apply(self.PACKET, verdicts)

        self.assertEqual(applied, [])
        self.assertEqual(new_text, self.PACKET)

    def test_reapplying_the_same_verdict_is_idempotent(self):
        verdicts = {"aaaaaaaaaaaa": {"verdict": "local"}}
        once, _ = apply_verdicts.apply(self.PACKET, verdicts)
        twice, _ = apply_verdicts.apply(once, verdicts)

        self.assertEqual(once, twice)


if __name__ == "__main__":
    unittest.main()
