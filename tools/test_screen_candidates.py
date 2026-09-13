#!/usr/bin/env python3
"""
Offline unit tests for tools/screen_candidates.py. No real network calls --
`fetch_url` is either not exercised directly (lower-level functions are
tested in isolation) or exercised with an injected fake opener.

Run with:
    python -m unittest tools/test_screen_candidates.py
"""

import json
import os
import shutil
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import screen_candidates as sc  # noqa: E402


class DenylistTests(unittest.TestCase):
    def test_prado_host_denylisted(self):
        self.assertTrue(sc.is_denylisted("consilium.europa.eu"))
        self.assertTrue(sc.is_denylisted("www.consilium.europa.eu"))
        self.assertTrue(sc.is_denylisted("photos.consilium.europa.eu"))

    def test_unrelated_host_not_denylisted(self):
        self.assertFalse(sc.is_denylisted("likumi.lv"))
        self.assertFalse(sc.is_denylisted("consilium.europa.eu.evil.example"))

    def test_empty_host(self):
        self.assertFalse(sc.is_denylisted(""))
        self.assertFalse(sc.is_denylisted(None))


class LinkCheckTests(unittest.TestCase):
    def test_absolute_url_linked_via_img_src(self):
        html_text = '<html><body><img src="https://example.com/a/b.jpg"></body></html>'
        self.assertTrue(
            sc.image_linked_from_page("https://example.com/page", html_text, "https://example.com/a/b.jpg")
        )

    def test_relative_url_resolved_against_page(self):
        html_text = '<html><body><a href="../files/spec.PNG">link</a></body></html>'
        self.assertTrue(
            sc.image_linked_from_page(
                "https://example.com/dir/page.html", html_text, "https://example.com/files/spec.PNG"
            )
        )

    def test_percent_encoded_filename_matches(self):
        html_text = '<img src="BILDES/MK_NOT_134/204E6DA8ED9A_PILSO%C5%86A_PASES_PARAUGS__1_.JPG">'
        image_url = "https://likumi.lv/wwwraksti/2025/248/BILDES/MK_NOT_134/204E6DA8ED9A_PILSO%C5%86A_PASES_PARAUGS__1_.JPG"
        self.assertTrue(sc.image_linked_from_page("https://likumi.lv/ta/id/244720", html_text, image_url))

    def test_not_linked_returns_false(self):
        html_text = '<html><body><img src="https://example.com/other.jpg"></body></html>'
        self.assertFalse(
            sc.image_linked_from_page("https://example.com/page", html_text, "https://example.com/a/b.jpg")
        )

    def test_srcset_is_parsed(self):
        html_text = '<img srcset="thumb.jpg 1x, full.jpg 2x">'
        self.assertTrue(sc.image_linked_from_page("https://example.com/page", html_text, "https://example.com/full.jpg"))

    def test_malformed_html_does_not_raise(self):
        html_text = "<img src='unterminated"
        # Should return False (not linked) rather than raising.
        result = sc.image_linked_from_page("https://example.com/page", html_text, "https://example.com/x.jpg")
        self.assertFalse(result)


class MagicByteTests(unittest.TestCase):
    def test_jpeg(self):
        self.assertEqual(sc.detect_image_type(b"\xff\xd8\xff\xe0rest"), "jpg")

    def test_png(self):
        self.assertEqual(sc.detect_image_type(b"\x89PNG\r\n\x1a\nrest"), "png")

    def test_gif87(self):
        self.assertEqual(sc.detect_image_type(b"GIF87arest"), "gif")

    def test_gif89(self):
        self.assertEqual(sc.detect_image_type(b"GIF89arest"), "gif")

    def test_webp(self):
        data = b"RIFF" + b"\x00\x00\x00\x00" + b"WEBP" + b"rest"
        self.assertEqual(sc.detect_image_type(data), "webp")

    def test_unrecognized_format_rejected(self):
        self.assertIsNone(sc.detect_image_type(b"%PDF-1.4 this is not an image"))
        self.assertIsNone(sc.detect_image_type(b""))


class CheckSampleParsingTests(unittest.TestCase):
    def test_mrz_hit_with_doc_number(self):
        text = (
            "MRZ HIT    doc_number=O1415403 format=Td3\n"
            "WATERMARK  OCR text contains \"specimen\" (case-insensitive) -- weak positive signal only\n"
            "VENDOR     CLEAR -- no known novelty/fake-document vendor signature in OCR text\n"
        )
        result = sc.parse_check_sample_output(text)
        self.assertEqual(result["mrz_status"], "hit")
        self.assertEqual(result["doc_number"], "O1415403")
        self.assertTrue(result["watermark"])
        self.assertEqual(result["vendor"], "clear")

    def test_mrz_miss_checksum_failed(self):
        text = (
            "MRZ MISS   parsed but checksums failed: [DocumentNumber]\n"
            "WATERMARK  no \"specimen\" text found in OCR output -- review the image yourself before adding this to the corpus\n"
            "VENDOR     CLEAR -- no known novelty/fake-document vendor signature in OCR text\n"
        )
        result = sc.parse_check_sample_output(text)
        self.assertEqual(result["mrz_status"], "miss")
        self.assertIsNone(result["doc_number"])
        self.assertFalse(result["watermark"])
        self.assertEqual(result["vendor"], "clear")

    def test_vendor_blocked(self):
        text = (
            "MRZ MISS   no MRZ found\n"
            "WATERMARK  no \"specimen\" text found in OCR output -- review the image yourself before adding this to the corpus\n"
            "VENDOR     BLOCKED -- OCR text matches known vendor signature(s): novelty\n"
        )
        result = sc.parse_check_sample_output(text)
        self.assertEqual(result["vendor"], "blocked")

    def test_missing_vendor_line_is_none_not_clear(self):
        # OCR-ERROR exits before any WATERMARK/VENDOR line is printed.
        text = "OCR-ERROR  /tmp/x.jpg: failed to decode image\n"
        result = sc.parse_check_sample_output(text)
        self.assertEqual(result["mrz_status"], "error")
        self.assertIsNone(result["vendor"])

    def test_missing_vendor_line_fails_closed_in_pipeline(self):
        # screen_candidate must treat vendor=None the same as vendor='blocked'.
        candidate = {
            "code": "XXX",
            "image_url": "https://example.com/img.jpg",
            "page_url": "https://example.com/page.html",
        }
        html_text = '<img src="img.jpg">'

        def fake_fetch(url, opener=None):
            del opener
            if url == candidate["page_url"]:
                return url, 200, html_text.encode("utf-8")
            if url == candidate["image_url"]:
                return url, 200, b"\xff\xd8\xff\xe0" + b"0" * 32
            raise AssertionError(f"unexpected fetch: {url}")

        with tempfile.TemporaryDirectory() as tmp:
            staging = os.path.join(tmp, "staging")
            # Fake binary path; run_check_sample is monkeypatched below.
            orig_run = sc.run_check_sample
            sc.run_check_sample = lambda binary_path, image_path, timeout=None: {
                "mrz_status": "error",
                "doc_number": None,
                "watermark": None,
                "vendor": None,
            }
            try:
                result = sc.screen_candidate(candidate, staging, [], [], "fake-binary", fetch=fake_fetch)
            finally:
                sc.run_check_sample = orig_run

            self.assertEqual(result["auto_reject"], "vendor")
            # The staged file must be deleted on a vendor reject.
            self.assertIsNone(result["staged_path"])
            self.assertEqual(os.listdir(staging) if os.path.isdir(staging) else [], [])


class DuplicateDetectionTests(unittest.TestCase):
    def test_ledger_duplicate_by_image_url(self):
        candidate = {"image_url": "https://x.example/a.jpg", "page_url": "https://x.example/page"}
        ledger = [{"url": "https://x.example/a.jpg"}]
        self.assertTrue(sc.ledger_url_duplicate(candidate, ledger))

    def test_ledger_duplicate_by_page_url(self):
        candidate = {"image_url": "https://x.example/a.jpg", "page_url": "https://x.example/page"}
        ledger = [{"url": "https://x.example/page"}]
        self.assertTrue(sc.ledger_url_duplicate(candidate, ledger))

    def test_ledger_no_duplicate(self):
        candidate = {"image_url": "https://x.example/a.jpg", "page_url": "https://x.example/page"}
        ledger = [{"url": "https://other.example/z.jpg"}]
        self.assertFalse(sc.ledger_url_duplicate(candidate, ledger))

    def test_sha_duplicate_against_corpus(self):
        corpus = [{"sha256": "abc123", "filename": "Country_Passport_Specimen_P0_AAA_2020_mrz.jpg"}]
        match = sc.sha_duplicate("abc123", corpus, [])
        self.assertEqual(match, "Country_Passport_Specimen_P0_AAA_2020_mrz.jpg")

    def test_sha_duplicate_against_ledger(self):
        ledger = [{"sha256": "def456", "url": "https://x.example/a.jpg"}]
        match = sc.sha_duplicate("def456", [], ledger)
        self.assertEqual(match, "https://x.example/a.jpg")

    def test_sha_no_duplicate(self):
        self.assertIsNone(sc.sha_duplicate("zzz999", [{"sha256": "abc"}], [{"sha256": "def"}]))


class EvidenceSnippetTests(unittest.TestCase):
    def test_licence_and_specimen_words_found(self):
        text = "This document is a SPECIMEN. All rights reserved, Copyright 2020 Ministry."
        licence = sc.find_snippets(text, sc.LICENCE_WORDS)
        specimen = sc.find_snippets(text, sc.SPECIMEN_WORDS)
        self.assertTrue(any("copyright" in s.lower() for s in licence))
        self.assertTrue(any("specimen" in s.lower() for s in specimen))

    def test_snippet_limit_respected(self):
        text = " specimen " * 20
        snippets = sc.find_snippets(text, ["specimen"], limit=3)
        self.assertLessEqual(len(snippets), 3)

    def test_no_match_returns_empty(self):
        self.assertEqual(sc.find_snippets("nothing relevant here", sc.LICENCE_WORDS), [])


class StripTagsTests(unittest.TestCase):
    def test_script_and_style_excluded(self):
        html_text = "<html><head><style>.x{}</style></head><body>Hello <script>var x=1;</script> World</body></html>"
        text = sc.strip_tags(html_text)
        self.assertIn("Hello", text)
        self.assertIn("World", text)
        self.assertNotIn("var x", text)
        self.assertNotIn(".x{}", text)


class DocClaimsMrzTests(unittest.TestCase):
    def test_td1_front_does_not_claim_mrz(self):
        candidate = {"doc": "identity card (citizen), front", "td": "TD1"}
        self.assertFalse(sc.doc_claims_mrz(candidate))

    def test_td3_biodata_claims_mrz(self):
        candidate = {"doc": "passport (citizen), biodata page with MRZ", "td": "TD3"}
        self.assertTrue(sc.doc_claims_mrz(candidate))

    def test_td1_back_claims_mrz(self):
        candidate = {"doc": "identity card (citizen), back", "td": "TD1"}
        self.assertTrue(sc.doc_claims_mrz(candidate))


class PacketWritingTests(unittest.TestCase):
    def test_packet_contains_survivor_row_and_blank_verdict(self):
        survivors = [
            {
                "code": "LVA",
                "doc": "passport, biodata",
                "td": "TD3",
                "series": "2025 regulation",
                "page_url_final": "https://likumi.lv/ta/id/244720",
                "page_url": "https://likumi.lv/ta/id/244720",
                "licence_snippets": ["Copyright 2025"],
                "specimen_snippets": ["PARAUGS"],
                "mrz_check": "valid",
                "staged_path": "/tmp/staging/abcdef123456.jpg",
                "needs_eyes": False,
                "variant_of": None,
            }
        ]
        with tempfile.TemporaryDirectory() as tmp:
            packet_path = os.path.join(tmp, "packet.md")
            sc.write_packet(packet_path, survivors)
            with open(packet_path, "r", encoding="utf-8") as f:
                content = f.read()
        self.assertIn("LVA", content)
        self.assertIn("likumi.lv", content)
        self.assertIn("abcdef123456.jpg", content)
        # Verdict / proposed destination / proposed licence columns stay blank.
        self.assertIn("| valid | | | [abcdef123456.jpg]", content)


class ScreenCandidatePipelineTests(unittest.TestCase):
    """End-to-end checks of screen_candidate() with a fake fetch function and
    a monkeypatched run_check_sample, still fully offline."""

    def setUp(self):
        self.tmp = tempfile.mkdtemp()
        self.orig_run_check_sample = sc.run_check_sample

    def tearDown(self):
        sc.run_check_sample = self.orig_run_check_sample
        shutil.rmtree(self.tmp, ignore_errors=True)

    def test_denylisted_short_circuits(self):
        candidate = {
            "code": "XXX",
            "image_url": "https://consilium.europa.eu/prado/img.jpg",
            "page_url": "https://consilium.europa.eu/prado/page.html",
        }
        result = sc.screen_candidate(candidate, self.tmp, [], [], "fake-binary")
        self.assertEqual(result["auto_reject"], "denylisted")

    def test_full_survivor_path(self):
        candidate = {
            "code": "LVA",
            "doc": "passport (citizen), biodata page with MRZ",
            "td": "TD3",
            "image_url": "https://example.com/specimen.jpg",
            "page_url": "https://example.com/page.html",
        }
        html_text = (
            '<html><body>This is a SPECIMEN passport. Copyright 2025.'
            '<img src="specimen.jpg"></body></html>'
        )

        def fake_fetch(url, opener=None):
            del opener
            if url == candidate["page_url"]:
                return url, 200, html_text.encode("utf-8")
            if url == candidate["image_url"]:
                return url, 200, b"\xff\xd8\xff\xe0" + b"1" * 64
            raise AssertionError(f"unexpected fetch {url}")

        sc.run_check_sample = lambda binary_path, image_path, timeout=None: {
            "mrz_status": "hit",
            "doc_number": "O1415403",
            "watermark": True,
            "vendor": "clear",
        }

        result = sc.screen_candidate(candidate, self.tmp, [], [], "fake-binary", fetch=fake_fetch)
        self.assertIsNone(result["auto_reject"])
        self.assertEqual(result["mrz_check"], "valid")
        self.assertTrue(result["staged_path"])
        self.assertTrue(os.path.isfile(result["staged_path"]))
        self.assertFalse(result["needs_eyes"])
        # doc_number is never written into the result dict.
        self.assertNotIn("doc_number", result)

    def test_variant_of_matches_corpus_in_memory_only(self):
        candidate = {
            "code": "LVA",
            "doc": "passport, biodata",
            "td": "TD3",
            "image_url": "https://example.com/specimen.jpg",
            "page_url": "https://example.com/page.html",
        }
        html_text = '<html><body>SPECIMEN<img src="specimen.jpg"></body></html>'

        def fake_fetch(url, opener=None):
            del opener
            if url == candidate["page_url"]:
                return url, 200, html_text.encode("utf-8")
            if url == candidate["image_url"]:
                return url, 200, b"\xff\xd8\xff\xe0" + b"2" * 64
            raise AssertionError(f"unexpected fetch {url}")

        sc.run_check_sample = lambda binary_path, image_path, timeout=None: {
            "mrz_status": "hit",
            "doc_number": "O1415403",
            "watermark": True,
            "vendor": "clear",
        }

        corpus = [
            {
                "filename": "Latvia_Passport_Specimen_P0_LVA_2025_mrz.jpg",
                "expected_document_number": "O1415403",
                "mrz": {"issuing_state": "LVA"},
            }
        ]

        result = sc.screen_candidate(candidate, self.tmp, [], corpus, "fake-binary", fetch=fake_fetch)
        self.assertEqual(result["variant_of"], "Latvia_Passport_Specimen_P0_LVA_2025_mrz.jpg")
        self.assertNotIn("doc_number", result)
        # And the output-record helper still guarantees no doc_number leaks.
        output_record = sc.screened_record_for_output(result)
        self.assertNotIn("doc_number", output_record)

    def test_not_linked_from_page_rejects(self):
        candidate = {
            "code": "XXX",
            "image_url": "https://example.com/specimen.jpg",
            "page_url": "https://example.com/page.html",
        }
        html_text = "<html><body>No relevant links here.</body></html>"

        def fake_fetch(url, opener=None):
            del opener
            if url == candidate["page_url"]:
                return url, 200, html_text.encode("utf-8")
            raise AssertionError("image should never be fetched when not linked")

        result = sc.screen_candidate(candidate, self.tmp, [], [], "fake-binary", fetch=fake_fetch)
        self.assertEqual(result["auto_reject"], "unresolvable")
        self.assertEqual(result["note"], "not-linked-from-page")

    def test_off_scope_non_image(self):
        candidate = {
            "code": "XXX",
            "image_url": "https://example.com/doc.pdf",
            "page_url": "https://example.com/page.html",
        }
        html_text = '<img src="doc.pdf">'

        def fake_fetch(url, opener=None):
            del opener
            if url == candidate["page_url"]:
                return url, 200, html_text.encode("utf-8")
            if url == candidate["image_url"]:
                return url, 200, b"%PDF-1.4 not an image"
            raise AssertionError(f"unexpected fetch {url}")

        result = sc.screen_candidate(candidate, self.tmp, [], [], "fake-binary", fetch=fake_fetch)
        self.assertEqual(result["auto_reject"], "off-scope")


if __name__ == "__main__":
    unittest.main()
