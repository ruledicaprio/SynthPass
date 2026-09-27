"""Pure, UCD-version-independent tests for latin_letter_with_coverage.py.

Deliberately does not scan the full Unicode Character Database (that count
varies by Python version -- see the tool's module docstring). Every code
point asserted on below was assigned in Unicode 1.1 (1993) and its name has
never changed since, so the assertions hold on any Python this repository
supports.
"""

import unittest

import latin_letter_with_coverage as coverage


class TableACodepointsTests(unittest.TestCase):
    def test_parses_hex_keys_from_the_array_literal_only(self):
        source = (
            "const OTHER: &[(char, &str)] = &[('\\u{FFFF}', \"nope\")];\n"
            "const TABLE_A: &[(char, &[&str])] = &[\n"
            "    ('\\u{00C0}', &[\"A\"]),              // À  A grave\n"
            "    ('\\u{1E9E}', &[\"SS\"]),             // ẞ  double s\n"
            "];\n"
            "const AFTER: &[(char, &str)] = &[('\\u{0000}', \"nope\")];\n"
        )
        self.assertEqual(coverage.table_a_codepoints(source), {0x00C0, 0x1E9E})


class IsLatinWithNameTests(unittest.TestCase):
    def test_matches_capital_and_small_with_forms(self):
        self.assertTrue(coverage.is_latin_with_name("LATIN CAPITAL LETTER A WITH GRAVE"))
        self.assertTrue(coverage.is_latin_with_name("LATIN SMALL LETTER U WITH DIAERESIS"))

    def test_rejects_non_with_and_non_latin_names(self):
        self.assertFalse(coverage.is_latin_with_name("LATIN CAPITAL LETTER AE"))
        self.assertFalse(coverage.is_latin_with_name("LATIN SMALL LETTER DOTLESS I"))
        self.assertFalse(coverage.is_latin_with_name("GREEK CAPITAL LETTER OMEGA"))
        self.assertFalse(coverage.is_latin_with_name("LATIN CAPITAL LETTER A"))


class ClassifyTests(unittest.TestCase):
    # U+00C0 (A grave) and U+00E0 (a grave) have carried these exact names
    # since Unicode 1.1; U+01FA (A WITH RING ABOVE AND ACUTE, upper-case
    # only, no distinct lower-case Table A counterpart) was assigned in
    # Unicode 1.1 too and has no Table A row in any version of Doc 9303.
    A_GRAVE_UPPER = 0x00C0
    A_GRAVE_LOWER = 0x00E0
    A_RING_ACUTE_UPPER_ONLY_GAP = 0x01FA

    def test_direct_row_when_codepoint_itself_is_a_table_a_key(self):
        table_a = {self.A_GRAVE_UPPER}
        result = coverage.classify([self.A_GRAVE_UPPER], table_a)
        self.assertEqual(result.direct_rows, [self.A_GRAVE_UPPER])
        self.assertEqual(result.case_folded, [])
        self.assertEqual(result.gaps, [])

    def test_case_folded_when_uppercase_form_is_a_table_a_key(self):
        table_a = {self.A_GRAVE_UPPER}
        result = coverage.classify([self.A_GRAVE_LOWER], table_a)
        self.assertEqual(result.direct_rows, [])
        self.assertEqual(result.case_folded, [self.A_GRAVE_LOWER])
        self.assertEqual(result.gaps, [])

    def test_gap_when_neither_the_codepoint_nor_its_uppercase_is_a_table_a_key(self):
        table_a = {self.A_GRAVE_UPPER}
        result = coverage.classify([self.A_RING_ACUTE_UPPER_ONLY_GAP], table_a)
        self.assertEqual(result.direct_rows, [])
        self.assertEqual(result.case_folded, [])
        self.assertEqual(result.gaps, [self.A_RING_ACUTE_UPPER_ONLY_GAP])

    def test_empty_table_a_puts_everything_in_gaps(self):
        codepoints = [self.A_GRAVE_UPPER, self.A_GRAVE_LOWER, self.A_RING_ACUTE_UPPER_ONLY_GAP]
        result = coverage.classify(codepoints, set())
        self.assertEqual(result.direct_rows, [])
        self.assertEqual(result.case_folded, [])
        self.assertEqual(sorted(result.gaps), sorted(codepoints))


class RenderMarkdownTests(unittest.TestCase):
    def test_counts_and_sections_reflect_the_classification(self):
        classification = coverage.Classification(
            direct_rows=[0x00C0],
            case_folded=[0x00E0],
            gaps=[0x01FA],
        )
        markdown = coverage.render_markdown(classification, "99.0.0", total=3)
        self.assertIn("Unicode Character Database version: **99.0.0**", markdown)
        self.assertIn("Total `LATIN ... WITH ...` characters: **3**", markdown)
        self.assertIn("Direct Table A row: **1**", markdown)
        self.assertIn("case-folded hit\"): **1**", markdown)
        self.assertIn("unchanged (upper-cased): **1**", markdown)
        self.assertIn("U+00C0", markdown)
        self.assertIn("LATIN CAPITAL LETTER A WITH GRAVE", markdown)
        self.assertIn("U+00E0", markdown)
        self.assertIn("U+01FA", markdown)

    def test_empty_sections_still_render_their_header_and_no_rows(self):
        classification = coverage.Classification(direct_rows=[], case_folded=[], gaps=[])
        markdown = coverage.render_markdown(classification, "99.0.0", total=0)
        self.assertIn("## Gaps", markdown)
        self.assertNotIn("U+", markdown)


if __name__ == "__main__":
    unittest.main()
