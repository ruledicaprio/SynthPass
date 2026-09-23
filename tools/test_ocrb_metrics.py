#!/usr/bin/env python3
"""
Offline unit tests for tools/ocrb_metrics.py. No network, no git, no cargo:
they read the vendored font and two Rust source files from the working tree.

The decoder is checked against the font's own redundancy (every glyph's glyf
header stores its bounding box, which the decoded points must reproduce) and
the geometry against itself (scanline runs integrated over the glyph must
reproduce the shoelace area). When fontTools is installed, the decoded points
are also compared with fontTools'; CI's tools job installs nothing, so that
test skips there and the self-consistency tests are the ones CI relies on.

Run with:
    python -m unittest tools/test_ocrb_metrics.py
"""

from __future__ import annotations

import contextlib
import io
import json
import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import ocrb_metrics as om  # noqa: E402

try:
    from fontTools.ttLib import TTFont  # type: ignore
except ImportError:  # the CI tools job is stdlib-only
    TTFont = None

FONT = om.load_font(om.DEFAULT_FONT)


def square(x0: int, y0: int, side: int, clockwise: bool) -> list[tuple[int, int, bool]]:
    pts = [(x0, y0), (x0 + side, y0), (x0 + side, y0 + side), (x0, y0 + side)]
    if clockwise:
        pts.reverse()
    return [(x, y, True) for x, y in pts]


# --------------------------------------------------------------------------
# Decoder
# --------------------------------------------------------------------------


class DecoderTests(unittest.TestCase):
    def test_every_mrz_character_is_decoded(self):
        self.assertEqual(set(FONT.glyphs), set(om.MRZ_ALPHABET))
        self.assertEqual(FONT.units_per_em, 1024)

    def test_points_reproduce_the_stored_bounding_box(self):
        # The glyf header's bbox is computed by the font editor from the same
        # points, so a wrong flag, delta or sign in the decoder shows up here.
        for ch, g in FONT.glyphs.items():
            xs = [x for c in g.contours for x, _, _ in c]
            ys = [y for c in g.contours for _, y, _ in c]
            with self.subTest(ch=ch):
                self.assertEqual((min(xs), min(ys), max(xs), max(ys)), g.header_bbox)

    def test_contour_counts(self):
        expected = {"B": 3, "8": 3, "0": 2, "O": 2, "A": 2, "S": 1, "C": 1, "<": 1}
        for ch, n in expected.items():
            with self.subTest(ch=ch):
                self.assertEqual(len(FONT.glyphs[ch].contours), n)

    def test_garbage_is_a_font_error(self):
        path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "ocrb_metrics.py")
        with self.assertRaises(om.FontError):
            om.load_font(om.Path(path))

    def test_a_missing_character_is_a_font_error(self):
        with self.assertRaises(om.FontError):
            om.load_font(om.DEFAULT_FONT, chars="一")

    @unittest.skipIf(TTFont is None, "fontTools not installed")
    def test_agrees_with_fonttools(self):
        tt = TTFont(str(om.DEFAULT_FONT))
        cmap = tt.getBestCmap()
        for ch, g in FONT.glyphs.items():
            name = cmap[ord(ch)]
            ref = tt["glyf"][name]
            coords, ends, flags = ref.getCoordinates(tt["glyf"])
            ours = [(x, y, on) for c in g.contours for x, y, on in c]
            with self.subTest(ch=ch):
                self.assertEqual(g.advance, tt["hmtx"][name][0])
                self.assertEqual(len(g.contours), ref.numberOfContours)
                self.assertEqual([(x, y) for x, y, _ in ours], [tuple(p) for p in coords])
                self.assertEqual([on for _, _, on in ours], [bool(f & 1) for f in flags])


# --------------------------------------------------------------------------
# Geometry
# --------------------------------------------------------------------------


class GeometryTests(unittest.TestCase):
    def test_square_is_all_straight_with_exact_area(self):
        g = om.RawGlyph(200, (0, 0, 100, 100), [square(0, 0, 100, clockwise=False)])
        m = om.measure("x", g)
        self.assertAlmostEqual(m.ink_area, 10_000)
        self.assertEqual(m.holes, 0)
        self.assertEqual(m.straight_fraction, 1.0)
        self.assertEqual(m.rows, [(1, 1.0)] * 5)

    def test_hole_is_counted_from_orientation_either_way_round(self):
        for outer_cw in (True, False):
            g = om.RawGlyph(200, (0, 0, 100, 100),
                            [square(0, 0, 100, clockwise=outer_cw), square(25, 25, 50, clockwise=not outer_cw)])
            m = om.measure("o", g)
            with self.subTest(outer_clockwise=outer_cw):
                self.assertEqual(m.holes, 1)
                self.assertAlmostEqual(m.ink_area, 10_000 - 2_500)
                self.assertEqual(m.rows[2], (2, 0.5))  # the middle row crosses both walls

    def test_all_off_curve_contour_closes_on_itself(self):
        # Four off-curve points: every on-curve point is an implied midpoint.
        contour = [(0, 50, False), (50, 100, False), (100, 50, False), (50, 0, False)]
        segs = om.segments(contour)
        self.assertEqual(len(segs), 4)
        self.assertTrue(all(kind == "quad" for kind, _ in segs))
        for (_, a), (_, b) in zip(segs, segs[1:] + segs[:1]):
            self.assertEqual(a[-1], b[0])

    def test_scanline_runs_integrate_to_the_shoelace_area(self):
        # Pins the even-odd pairing in `ink_runs` against the signed area for
        # every real glyph: overlapping contours or a wrong orientation would
        # make the two disagree.
        steps = 400
        for ch, g in FONT.glyphs.items():
            m = om.measure(ch, g)
            polys = [om.flatten(c)[0] for c in g.contours]
            dy = m.ink_h / steps
            integral = sum(om.ink_runs(polys, 1, m.y_min + (i + 0.5) * dy)[1] * dy for i in range(steps))
            with self.subTest(ch=ch):
                self.assertAlmostEqual(integral / m.ink_area, 1.0, delta=0.01)


# --------------------------------------------------------------------------
# The Rust constants
# --------------------------------------------------------------------------


class RustSourceTests(unittest.TestCase):
    def test_parse_confusables(self):
        src = 'pub const CONFUSABLES: &[(char, &str)] = &[\n    (\'2\', "Z7"),\n    (\'7\', "T2"),\n];'
        rows = om.parse_confusables(src)
        self.assertEqual(rows, [("2", "Z7"), ("7", "T2")])
        # 2-7 appears in both rows and is kept once.
        self.assertEqual(om.confusable_pairs(rows), [("2", "Z"), ("2", "7"), ("7", "T")])

    def test_real_confusables_are_read(self):
        rows = om.parse_confusables(om.REPAIR_RS.read_text(encoding="utf-8"))
        self.assertIn(("0", "ODQ"), rows)
        self.assertIn(("M", "N"), rows)

    def test_missing_constants_are_errors(self):
        with self.assertRaises(ValueError):
            om.parse_confusables("no table here")
        with self.assertRaises(ValueError):
            om.parse_ink_floor("no floor here")

    def test_real_ink_floor_is_read(self):
        floor = om.parse_ink_floor(om.CHARGRID_RS.read_text(encoding="utf-8"))
        self.assertGreater(floor, 0.0)
        self.assertLess(floor, 1.0)


# --------------------------------------------------------------------------
# Findings about the vendored font. A font swap should trip these on purpose:
# each is quoted in the report and in the PR that introduced this tool.
# --------------------------------------------------------------------------


class FindingsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.r = om.build(om.DEFAULT_FONT)

    def test_every_digit_is_taller_than_every_letter(self):
        self.assertGreater(self.r["height_gap"]["gap_units"], 0)

    def test_five_alone_has_a_narrow_advance(self):
        groups = self.r["font"]["advance_groups"]
        self.assertFalse(self.r["font"]["monospaced"])
        self.assertEqual(groups["782"], "5")

    def test_filler_is_one_straight_contour_without_holes(self):
        g = self.r["glyphs"]["<"]
        self.assertEqual((g["contours"], g["holes"], g["straight_fraction"]), (1, 0, 1.0))

    def test_every_pair_is_reported(self):
        got = {(p["a"], p["b"]) for p in self.r["pairs"]}
        self.assertIn(("<", "K"), got)
        self.assertIn(("5", "S"), got)
        self.assertIn(("M", "N"), got)

    def test_ideal_filler_clears_the_ink_floor(self):
        c = self.r["chargrid"]
        self.assertGreater(c["filler_cell_ink"], c["ink_floor"])

    def test_json_output_round_trips(self):
        out = io.StringIO()
        with contextlib.redirect_stdout(out):
            self.assertEqual(om.main(["--json"]), 0)
        self.assertEqual(json.loads(out.getvalue())["font"]["units_per_em"], 1024)


if __name__ == "__main__":
    unittest.main()
