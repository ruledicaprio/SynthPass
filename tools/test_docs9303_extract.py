#!/usr/bin/env python3
"""
Offline unit tests for tools/docs9303_extract.py's pure text-transform
functions: heading detection, furniture stripping, dehyphenation/reflow, and
the small helpers around them. No PDF, no network, no filesystem — every
fixture here is a literal line lifted from the Part 11/12 PDF text dumps
made while building the extractor, so a regression here is a regression a
human already found once by reading the PDF.

Run with:
    python -m unittest tools/test_docs9303_extract.py
"""

from __future__ import annotations

import os
import sys
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import docs9303_extract as ex  # noqa: E402


class BlankAndPageNumberTests(unittest.TestCase):
    def test_blank_line(self):
        self.assertTrue(ex.is_blank_line(""))
        self.assertTrue(ex.is_blank_line(" "))
        self.assertTrue(ex.is_blank_line("   \t  "))

    def test_non_blank_line(self):
        self.assertFalse(ex.is_blank_line("SCOPE"))

    def test_arabic_page_number(self):
        self.assertTrue(ex.is_page_number_line("12"))
        self.assertTrue(ex.is_page_number_line(" 2 "))

    def test_roman_folio(self):
        self.assertTrue(ex.is_page_number_line("(vii)"))
        self.assertTrue(ex.is_page_number_line("(iii)"))

    def test_appendix_folio(self):
        self.assertTrue(ex.is_page_number_line("App D-3"))
        self.assertTrue(ex.is_page_number_line("App A-1"))

    def test_not_a_page_number(self):
        self.assertFalse(ex.is_page_number_line("SCOPE"))
        self.assertFalse(ex.is_page_number_line("Table 1. Securing Electronic Data"))
        # a real clause number is not, on its own, a folio marker context —
        # but as a bare line it is indistinguishable from one, which is why
        # detect_heading requires a title on the SAME line.
        self.assertTrue(ex.is_page_number_line("4"))


class RevisionStampTests(unittest.TestCase):
    def test_revision_date(self):
        self.assertTrue(ex.is_revision_date_line("23/2/26"))
        self.assertTrue(ex.is_revision_date_line("14/6/24"))

    def test_not_a_revision_date(self):
        self.assertFalse(ex.is_revision_date_line("SCOPE"))
        self.assertFalse(ex.is_revision_date_line("1 January 2027"))

    def test_revision_number(self):
        self.assertTrue(ex.is_revision_number_line("No. 2"))
        self.assertTrue(ex.is_revision_number_line("No. 1"))

    def test_not_a_revision_number(self):
        self.assertFalse(ex.is_revision_number_line("character sets — Part 1: Latin alphabet No. 1"))

    def test_strip_furniture_removes_paired_stamp(self):
        lines = ["eMRTD chips implementing PACE only.", "23/2/26", "No. 2", ""]
        out = ex.strip_furniture(lines, "Machine Readable Travel Documents", "Part 11.    X")
        self.assertEqual(out, ["eMRTD chips implementing PACE only.", ""])

    def test_strip_furniture_keeps_real_no_1_reference(self):
        # "No. 1" not preceded by a bare date is real ICAO text and must
        # survive furniture stripping.
        lines = ["character sets — Part 1: Latin alphabet No. 1"]
        out = ex.strip_furniture(lines, "Machine Readable Travel Documents", "Part 11.    X")
        self.assertEqual(out, lines)


class RunningHeaderTests(unittest.TestCase):
    def test_verso_header(self):
        self.assertTrue(
            ex.is_running_header_line(
                "Machine Readable Travel Documents", "Machine Readable Travel Documents", "Part 11.    X"
            )
        )

    def test_recto_header(self):
        header = ex.recto_header_for(11, "Security Mechanisms for MRTDs")
        self.assertEqual(header, "Part 11.    Security Mechanisms for MRTDs")
        self.assertTrue(
            ex.is_running_header_line(
                "Part 11.    Security Mechanisms for MRTDs ", "Machine Readable Travel Documents", header
            )
        )

    def test_body_text_is_not_a_header(self):
        self.assertFalse(
            ex.is_running_header_line(
                "This part shall be read in conjunction with the following Parts of Doc 9303:",
                "Machine Readable Travel Documents",
                "Part 11.    Security Mechanisms for MRTDs",
            )
        )

    def test_parse_part_title_line(self):
        self.assertEqual(
            ex.parse_part_title_line("Part 11: Security Mechanisms for MRTDs"),
            (11, "Security Mechanisms for MRTDs"),
        )
        self.assertEqual(
            ex.parse_part_title_line("Part 12: Public Key Infrastructure for MRTDs "),
            (12, "Public Key Infrastructure for MRTDs"),
        )
        self.assertIsNone(ex.parse_part_title_line("Eighth Edition, 2021"))


class HeadingDetectionTests(unittest.TestCase):
    def test_top_level_heading(self):
        h = ex.detect_heading("1.    SCOPE ")
        self.assertIsNotNone(h)
        self.assertEqual(h.number, "1")
        self.assertEqual(h.title, "SCOPE")
        self.assertEqual(h.depth, 1)
        self.assertFalse(h.is_appendix_subsection)

    def test_two_digit_top_level(self):
        h = ex.detect_heading("10.    REFERENCES (NORMATIVE) ")
        self.assertEqual(h.number, "10")
        self.assertEqual(h.title, "REFERENCES (NORMATIVE)")

    def test_wide_gap_top_level(self):
        h = ex.detect_heading("8.     INSPECTION SYSTEM ")
        self.assertEqual(h.number, "8")

    def test_subsection(self):
        h = ex.detect_heading("2.1    Requirements for eMRTD Chips and Terminals ")
        self.assertEqual(h.number, "2.1")
        self.assertEqual(h.depth, 2)

    def test_deep_subsection(self):
        h = ex.detect_heading("7.1.4.3.7    Public Key Import ")
        self.assertEqual(h.number, "7.1.4.3.7")
        self.assertEqual(h.depth, 5)

    def test_appendix_subsection(self):
        h = ex.detect_heading("A.1    EXAMPLE 1 ")
        self.assertEqual(h.number, "A.1")
        self.assertTrue(h.is_appendix_subsection)
        self.assertEqual(h.depth, 2)

    def test_appendix_subsection_two_deep(self):
        h = ex.detect_heading("D.1.1    Certification Path Validation Procedure ")
        self.assertEqual(h.number, "D.1.1")
        self.assertTrue(h.is_appendix_subsection)

    def test_single_space_gap_is_not_a_heading(self):
        # A real false positive found in the Part 11 PDF: a wrapped sentence
        # that happens to start with a cross-reference number and a single
        # space, not a 2+ space heading gap.
        self.assertIsNone(ex.detect_heading("9.5.1 SHOULD be used. "))

    def test_numbered_bullet_is_not_a_heading(self):
        # Another real false positive: "56 Bit for a numeric Document
        # Number" from Appendix A's entropy bullet list.
        self.assertIsNone(ex.detect_heading("56 Bit for a numeric Document Number (3652 * 1012 possibilities) "))

    def test_plain_prose_is_not_a_heading(self):
        self.assertIsNone(ex.detect_heading("Part 11 to Doc 9303 provides specifications to enable States"))

    def test_lowercase_but_real_heading_is_detected(self):
        # "eMRTD" is a deliberately lower-case ICAO term; a heading titled
        # with it must not be rejected just because it isn't capitalised.
        h = ex.detect_heading("3.1    eMRTD PKI ")
        self.assertIsNotNone(h)
        self.assertEqual(h.title, "eMRTD PKI")

    def test_non_letter_title_start_is_not_a_heading(self):
        self.assertIsNone(ex.detect_heading("4.2    (stray fragment)"))


class HeadingLevelTests(unittest.TestCase):
    def test_depth1_is_h2(self):
        self.assertEqual(ex.heading_markdown_level(ex.Heading("1", "SCOPE")), 2)

    def test_depth2_is_h3(self):
        self.assertEqual(ex.heading_markdown_level(ex.Heading("2.1", "Notations")), 3)

    def test_depth3_is_h4(self):
        self.assertEqual(ex.heading_markdown_level(ex.Heading("4.7.1", "X")), 4)

    def test_depth4_is_h5(self):
        self.assertEqual(ex.heading_markdown_level(ex.Heading("4.7.3.2", "X")), 5)

    def test_depth5_caps_at_h5(self):
        self.assertEqual(ex.heading_markdown_level(ex.Heading("4.7.3.2.1", "X")), 5)

    def test_appendix_depth1_is_h4(self):
        self.assertEqual(ex.heading_markdown_level(ex.Heading("A.1", "EXAMPLE 1")), 4)

    def test_appendix_depth2_is_h5(self):
        self.assertEqual(ex.heading_markdown_level(ex.Heading("D.1.1", "X")), 5)


class AppendixStartTests(unittest.TestCase):
    def test_detects_appendix_start(self):
        self.assertEqual(ex.is_appendix_start("Appendix A to Part 11"), ("A", "11"))

    def test_case_insensitive(self):
        self.assertEqual(ex.is_appendix_start("appendix k to part 11"), ("k", "11"))

    def test_not_an_appendix_start(self):
        self.assertIsNone(ex.is_appendix_start("Appendix A describes something"))
        self.assertIsNone(ex.is_appendix_start("This is not an appendix at all"))


class BulletMarkerTests(unittest.TestCase):
    def test_dot_bullet(self):
        self.assertTrue(ex.is_bullet_marker_line("•"))
        self.assertTrue(ex.is_bullet_marker_line(" • "))

    def test_dash_bullet(self):
        self.assertTrue(ex.is_bullet_marker_line("-"))

    def test_not_a_bullet(self):
        self.assertFalse(ex.is_bullet_marker_line("- this has more text"))
        self.assertFalse(ex.is_bullet_marker_line("normal text"))


class DehyphenationTests(unittest.TestCase):
    """Every one of these pairs is a real line-wrap found in the Part 11/12
    PDF text outside a table; see join_wrapped's docstring for why the rule
    is "a trailing hyphen is always real content, never stripped"."""

    def test_no_hyphen_joins_with_space(self):
        self.assertEqual(ex.join_wrapped("Cryptographic protocols are specified", "to:"), "Cryptographic protocols are specified to:")

    def test_compound_word_keeps_hyphen(self):
        self.assertEqual(ex.join_wrapped("Does not require processor-", "ICs."), "Does not require processor-ICs.")

    def test_pseudo_random_keeps_hyphen(self):
        self.assertEqual(ex.join_wrapped("chosen randomly or pseudo-", "randomly"), "chosen randomly or pseudo-randomly")

    def test_standard_number_keeps_hyphen(self):
        self.assertEqual(ex.join_wrapped("according to [ISO/IEC 9796-", "2] Digital Signature scheme 1."), "according to [ISO/IEC 9796-2] Digital Signature scheme 1.")

    def test_asn1_identifier_keeps_hyphen(self):
        self.assertEqual(
            ex.join_wrapped("id-CA-ECDH-3DES-", "CBC-CBC, i.e. Secure Messaging is restricted to 3DES."),
            "id-CA-ECDH-3DES-CBC-CBC, i.e. Secure Messaging is restricted to 3DES.",
        )

    def test_trust_points_keeps_hyphen(self):
        self.assertEqual(ex.join_wrapped("new trust-points MUST be enabled, expired trust-", "points MUST be disabled"), "new trust-points MUST be enabled, expired trust-points MUST be disabled")

    def test_doc_reference_keeps_hyphen(self):
        self.assertEqual(ex.join_wrapped("encoding values can be found in Doc 9303-", "11."), "encoding values can be found in Doc 9303-11.")

    def test_reflow_multiple_lines(self):
        text = ex.reflow(["Part 11 to Doc 9303 provides specifications to enable States and suppliers", "to implement cryptographic security features"])
        self.assertEqual(text, "Part 11 to Doc 9303 provides specifications to enable States and suppliers to implement cryptographic security features")

    def test_reflow_with_trailing_hyphen(self):
        text = ex.reflow(["The following examples illustrate out-", "of-band confirmation"])
        self.assertEqual(text, "The following examples illustrate out-of-band confirmation")


class SlugifyTests(unittest.TestCase):
    def test_simple_heading(self):
        self.assertEqual(ex.slugify_heading("1. SCOPE"), "1-scope")

    def test_heading_with_parens(self):
        self.assertEqual(ex.slugify_heading("10. REFERENCES (NORMATIVE)"), "10-references-normative")

    def test_heading_with_multiword_title(self):
        self.assertEqual(
            ex.slugify_heading("4. ACCESS TO THE CONTACTLESS IC"), "4-access-to-the-contactless-ic"
        )


class LinkifyUrlTests(unittest.TestCase):
    def test_bare_www_gets_https_scheme(self):
        line = "Downloads and additional information are available at www.icao.int/security/mrtd"
        self.assertEqual(
            ex.linkify_url(line),
            "Downloads and additional information are available at "
            "[www.icao.int/security/mrtd](https://www.icao.int/security/mrtd)",
        )

    def test_https_url_keeps_its_own_scheme(self):
        line = "Downloads and additional information are available at https://www.icao.int/publications/doc-series"
        self.assertEqual(
            ex.linkify_url(line),
            "Downloads and additional information are available at "
            "[https://www.icao.int/publications/doc-series](https://www.icao.int/publications/doc-series)",
        )

    def test_no_url_is_unchanged(self):
        line = "This part shall be read in conjunction with the following Parts of Doc 9303:"
        self.assertEqual(ex.linkify_url(line), line)


class ChunkAmendmentRowsTests(unittest.TestCase):
    def test_single_row(self):
        self.assertEqual(ex.chunk_amendment_rows(["1", "14/6/24", "ICAO"]), [("1", "14/6/24", "ICAO")])

    def test_two_rows(self):
        cells = ["1", "14/6/24", "ICAO", "2", "23/2/26", "ICAO"]
        self.assertEqual(
            ex.chunk_amendment_rows(cells),
            [("1", "14/6/24", "ICAO"), ("2", "23/2/26", "ICAO")],
        )

    def test_empty(self):
        self.assertEqual(ex.chunk_amendment_rows([]), [])


class BuildBlocksTests(unittest.TestCase):
    """build_blocks/render_body need no PDF: they operate on plain per-page
    line lists, exactly as strip_furniture would hand them over."""

    def test_heading_and_paragraph(self):
        pages = [(1, ["1.    SCOPE", "", "Part 11 provides specifications", "for cryptographic security.", ""])]
        blocks = ex.build_blocks(pages)
        kinds = [b.kind for _, b in blocks]
        self.assertEqual(kinds, ["heading", "para"])
        self.assertEqual(blocks[0][1].heading.number, "1")
        self.assertEqual(blocks[1][1].text, "Part 11 provides specifications for cryptographic security.")

    def test_bullet_list_merges_adjacent_items(self):
        pages = [
            (
                1,
                [
                    "Cryptographic protocols are specified to:",
                    "",
                    "•",
                    "prevent skimming of data;",
                    "",
                    "•",
                    "prevent eavesdropping.",
                    "",
                ],
            )
        ]
        blocks = ex.build_blocks(pages)
        kinds = [b.kind for _, b in blocks]
        self.assertEqual(kinds, ["para", "list"])
        self.assertEqual(blocks[1][1].items, ["prevent skimming of data;", "prevent eavesdropping."])

    def test_appendix_heading_merges_title(self):
        pages = [
            (
                74,
                [
                    "Appendix A to Part 12",
                    "",
                    "LIFETIMES (INFORMATIVE)",
                    "",
                    "The following examples illustrate calculation.",
                    "",
                ],
            )
        ]
        blocks = ex.build_blocks(pages)
        self.assertEqual(blocks[0][1].kind, "heading")
        self.assertEqual(blocks[0][1].appendix_letter, "A")
        self.assertEqual(blocks[0][1].heading.title, "LIFETIMES (INFORMATIVE)")
        self.assertEqual(blocks[1][1].kind, "para")

    def test_page_marker_inserted_once_per_page(self):
        pages = [
            (1, ["1.    SCOPE", "", "First paragraph."]),
            (2, ["Second paragraph on the next page."]),
        ]
        blocks = ex.build_blocks(pages)
        rendered = ex.render_body(blocks)
        self.assertEqual(rendered.count("<!-- page 1 -->"), 1)
        self.assertEqual(rendered.count("<!-- page 2 -->"), 1)


class RenderTableMarkdownTests(unittest.TestCase):
    def test_simple_table(self):
        rows = [["A", "B"], ["1", "2"]]
        md = ex.render_table_markdown(rows)
        self.assertEqual(md[0], "| A | B |")
        self.assertEqual(md[1], "| :--- | :--- |")
        self.assertEqual(md[2], "| 1 | 2 |")

    def test_none_cell_becomes_empty(self):
        rows = [["A", "B"], ["x", None]]
        md = ex.render_table_markdown(rows)
        self.assertEqual(md[2], "| x |  |")

    def test_internal_newline_becomes_br(self):
        rows = [["A"], ["line one\nline two"]]
        md = ex.render_table_markdown(rows)
        self.assertEqual(md[2], "| line one<br>line two |")

    def test_pipe_in_cell_is_escaped(self):
        rows = [["A"], ["a | b"]]
        md = ex.render_table_markdown(rows)
        self.assertEqual(md[2], "| a \\| b |")


if __name__ == "__main__":
    unittest.main()
