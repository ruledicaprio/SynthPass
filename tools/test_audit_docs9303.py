"""Pure, PDF-free checks for the Doc 9303 auditor."""

import unittest

import audit_docs9303 as audit


class Docs9303AuditTests(unittest.TestCase):
    def test_digit_tokens_ignore_page_markers_and_link_targets(self):
        markdown = ("<!-- page 77 -->\n"
                    "Value 23 is in [Part 4](Doc_9303_Part4.md#section-19).\n"
                    '<img src="figure_p88.png"> Then 2.54 is printed.\n')
        self.assertEqual(audit.digit_tokens(markdown),
                         [(2, "23"), (2, "4"), (3, "2"), (3, "54")])

    def test_heading_diff_reports_both_directions(self):
        pdf = ["1    SCOPE", "1.1    Real Heading", "2    Common Clause"]
        markdown = "## 1. SCOPE\n### 1.1 Different Heading\n## 3. Extra Clause\n"
        missing, extra = audit.heading_diff(pdf, markdown)
        self.assertEqual(missing, ["2 Common Clause"])
        self.assertEqual(extra, ["3 Extra Clause"])

    def test_figure_table_pairing_ignores_image_and_editorial_markup(self):
        markdown = ("> **Figure 2. Zone layout**\n"
                    ">\n"
                    '> <img src="figure.png">\n'
                    "> [Editorial description, not ICAO text: schematic]\n"
                    "| Dimension | Value |\n"
                    "|---|---|\n"
                    "\n"
                    "> **Figure 3. No adjacent table**\n"
                    "A prose paragraph intervenes.\n"
                    "| Column | Value |\n")
        self.assertEqual(audit.figure_tables(markdown),
                         [audit.FigureTable(1, "2", "Zone layout")])

    def test_fivegram_score_normalizes_case_whitespace_and_line_wrap_hyphen(self):
        pdf = ("The biometric infor-\n"
               "mation is stored in the contactless integrated circuit for inspection.")
        markdown = ("the BIOMETRIC information is stored in the contactless "
                    "integrated circuit for inspection.")
        self.assertEqual(audit.prose_score(markdown, set(audit.fivegrams(pdf))), 1.0)
        changed = "This unrelated paragraph has many other words and no matching sequence anywhere."
        self.assertLess(audit.prose_score(changed, set(audit.fivegrams(pdf))), 0.8)

    def test_paragraphs_skip_editorial_text_and_short_blocks(self):
        markdown = ("<!-- page 3 -->\n\n"
                    "## 1. SCOPE\n\n"
                    "This paragraph contains enough ordinary words to qualify for a meaningful "
                    "five gram comparison with the source PDF.\n\n"
                    "[Editorial description, not ICAO text: this long description has many "
                    "words but is intentionally skipped by the prose audit.]\n\n"
                    "Too short.\n")
        found = audit.paragraphs(markdown)
        self.assertEqual(len(found), 1)
        self.assertEqual(found[0][0], 5)

    def test_heading_diff_finds_a_split_heading_the_old_detector_missed(self):
        # Literal shape of Part 11 pages 25-26: heading_diff now uses
        # extract.find_headings_in_lines, so a detector fix in
        # docs9303_extract.py shows up here automatically.
        pdf_lines = [
            "4.4.3.3    Encrypting and Mapping Nonces",
            "",
            "4.4.3.3.1",
            "Generic Mapping",
        ]
        markdown = "## 4.4.3.3 Encrypting and Mapping Nonces\n"
        missing, extra = audit.heading_diff(pdf_lines, markdown)
        self.assertEqual(missing, ["4.4.3.3.1 Generic Mapping"])
        self.assertEqual(extra, [])

    def test_heading_diff_does_not_promote_a_quoted_rfc_number(self):
        pdf_lines = [
            "4.1.2    LDS2 Signer Keys and Certificates",
            "",
            "  version",
            "RFC 5280 –",
            "4.1.2.1",
            "When extensions are used, as expected in this profile,",
            "version MUST be 3 (value is 2).",
        ]
        markdown = "## 4.1.2 LDS2 Signer Keys and Certificates\n"
        missing, extra = audit.heading_diff(pdf_lines, markdown)
        self.assertEqual(missing, [])
        self.assertEqual(extra, [])

    def test_pdf_headings_drops_bare_integer_after_appendix_marker(self):
        # Literal shape of Part 3 Appendix B: a 16-item numbered list of
        # name-transliteration variants collides with a bare top-level
        # clause number ("9.   Mohammed" reads exactly like a real "9."
        # heading would) -- but only once the appendix has actually begun.
        lines = [
            "8.    REFERENCES (NORMATIVE)",
            "",
            "APPENDIX B TO PART 3.    TRANSLITERATION OF ARABIC SCRIPT",
            "",
            "8.   Mohamed",
            "9.   Mohammed",
        ]
        found = audit.pdf_headings(lines)
        self.assertIn("8", found)
        self.assertNotIn("9", found)

    def test_pdf_headings_keeps_bare_integer_headings_before_any_appendix(self):
        lines = ["1.    SCOPE", "", "2.    ASSUMPTIONS AND NOTATIONS"]
        found = audit.pdf_headings(lines)
        self.assertEqual(set(found), {"1", "2"})

    def test_appendix_marker_requires_full_caps_not_a_cross_reference_sentence(self):
        # A body sentence that merely mentions an appendix, wrapped so it
        # starts a new physical PDF line with the word "Appendix", must not
        # be mistaken for the Part's own appendix heading (Part 3's own body
        # text has exactly this line before its real Appendix A begins).
        self.assertFalse(audit._is_appendix_marker_line("Appendix A to this Part."))
        self.assertTrue(audit._is_appendix_marker_line("APPENDIX B TO PART 3.    TRANSLITERATION"))

    def test_number_alone_candidates_finds_a_capitalised_next_line(self):
        lines = ["4.4.3.3.1", "Generic Mapping", "not capitalised", "4.1.2.1", "When extensions"]
        found = audit.number_alone_candidates(lines)
        self.assertEqual(
            found,
            [(0, "4.4.3.3.1", "Generic Mapping"), (3, "4.1.2.1", "When extensions")],
        )

    def test_number_alone_candidates_ignores_lowercase_next_line(self):
        lines = ["4.4.3.3.1", "not capitalised"]
        self.assertEqual(audit.number_alone_candidates(lines), [])


if __name__ == "__main__":
    unittest.main()