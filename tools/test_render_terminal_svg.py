"""Tests for render_terminal_svg.render: deterministic, escaped, whitespace-preserving."""
import unittest

import render_terminal_svg as rts


class RenderTests(unittest.TestCase):
    def test_same_input_gives_byte_identical_svg(self):
        lines = ["a  b", "", "<c> & d"]
        self.assertEqual(rts.render("synthpass", lines, "t"), rts.render("synthpass", lines, "t"))

    def test_markup_characters_are_escaped(self):
        svg = rts.render("x", ["<path_to_image> & more"], "t")
        self.assertIn("&lt;path_to_image&gt; &amp; more", svg)
        self.assertNotIn("<path_to_image>", svg)

    def test_every_text_line_preserves_its_spaces(self):
        svg = rts.render("x", ["  indented   columns"], "t")
        for line in svg.splitlines():
            if line.startswith("<text x=") and "text-anchor" not in line:
                self.assertIn('xml:space="preserve"', line)
        self.assertIn("  indented   columns", svg)

    def test_width_grows_with_the_longest_line(self):
        narrow = rts.render("x", ["ab"], "t")
        wide = rts.render("x", ["a" * 200], "t")
        width = lambda svg: int(svg.split('width="', 1)[1].split('"', 1)[0])
        self.assertGreater(width(wide), width(narrow))


if __name__ == "__main__":
    unittest.main()
