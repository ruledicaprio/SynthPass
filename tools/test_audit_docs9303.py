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


if __name__ == "__main__":
    unittest.main()