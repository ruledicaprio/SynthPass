"""Synthetic, image-free tests for the fixture intake gate."""

import contextlib
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import audit_fixture_checksums as audit
import check_fixture_intake as intake


def synthetic_zone():
    line1 = "P<ZZZ" + "ALPHA<<BETA".ljust(39, "<")
    number = "A12345678"
    birth = "900101"
    expiry = "300101"
    personal = "<" * 14
    line2 = (number + audit.check_digit(number) + "ZZZ"
             + birth + audit.check_digit(birth) + "M"
             + expiry + audit.check_digit(expiry) + personal + "<")
    composite = audit.check_digit(line2[:10] + line2[13:20] + line2[21:43])
    zone = line1 + "\n" + line2 + composite
    assert audit.audit_zone(zone).valid
    return zone


class FixtureIntakeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        exempt_patch = patch.object(intake, "TRAITS_EXEMPT", {})
        exempt_patch.start()
        self.addCleanup(exempt_patch.stop)
        self.root = Path(self.temp.name)
        self.samples = self.root / "samples"
        self.fixtures = self.samples / "ocr_fixtures"
        self.fixtures.mkdir(parents=True)
        self.test_dir = self.root / "crates" / "synthpass-bench" / "tests"
        (self.test_dir / "fixtures").mkdir(parents=True)
        self.corpus = self.samples / "corpus.jsonl"
        self.fixture = self.fixtures / "synthetic.json"
        self.traits = self.samples / "template_traits.jsonl"
        self.golden = self.test_dir / "fixtures" / "mrz_wire_golden.jsonl"
        self.zone = synthetic_zone()
        self.manifest_row = {
            "dir": "passports",
            "filename": "synthetic.jpg",
            "ground_truth_stem": "synthetic",
            "expected_document_number": "A12345678",
            "mrz": {"redacted": False},
        }
        self.fixture_row = {"mrz_line": self.zone, "mrz_checksums_valid": True}
        self.trait_row = {"source": [{"filename": "synthetic.jpg"}]}
        self.write_jsonl(self.corpus, [self.manifest_row])
        self.fixture.write_text(json.dumps(self.fixture_row), encoding="utf-8")
        self.write_jsonl(self.traits, [self.trait_row])
        self.write_jsonl(self.golden, [{"fixture": "synthetic.json"}])
        (self.test_dir / "template_traits.rs").write_text(
            'const EXPECTED: &[(&str, bool, usize)] = &[("Td3", false, 1),];\n',
            encoding="utf-8",
        )

    @staticmethod
    def write_jsonl(path, rows):
        path.write_text("".join(json.dumps(row) + "\n" for row in rows),
                        encoding="utf-8")

    def problems(self):
        return "\n".join(intake.check(self.root))

    def test_clean_tree_passes(self):
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            code = intake.main(self.root)
        self.assertEqual(code, 0)
        self.assertEqual(output.getvalue(), "")

    def test_checksum_claim_mismatch_is_zone_free(self):
        self.fixture_row["mrz_checksums_valid"] = False
        self.manifest_row["expected_document_number"] = None
        self.fixture.write_text(json.dumps(self.fixture_row), encoding="utf-8")
        self.write_jsonl(self.corpus, [self.manifest_row])
        problems = self.problems()
        self.assertIn("synthetic.json: checksum claim disagrees", problems)
        self.assertIn("python tools/audit_fixture_checksums.py", problems)
        self.assertNotIn(self.zone.split("\n")[0], problems)
        self.assertNotIn(self.zone.split("\n")[1], problems)

    def test_missing_link_and_orphan_fixture_are_reported(self):
        self.manifest_row["ground_truth_stem"] = "missing"
        self.write_jsonl(self.corpus, [self.manifest_row])
        problems = self.problems()
        self.assertIn("missing root fixture missing.json", problems)
        self.assertIn("synthetic.json: named by 0 manifest rows", problems)

    def test_duplicate_manifest_owner_is_reported(self):
        self.write_jsonl(self.corpus, [self.manifest_row, self.manifest_row])
        self.assertIn("synthetic.json: named by 2 manifest rows", self.problems())

    def test_nonconforming_fixture_requires_null_expected_number(self):
        self.fixture_row["mrz_line"] = self.zone[:-1] + (
            "0" if self.zone[-1] != "0" else "1")
        self.fixture_row["mrz_checksums_valid"] = False
        self.fixture.write_text(json.dumps(self.fixture_row), encoding="utf-8")
        problems = self.problems()
        self.assertIn("nonconforming fixture has expected_document_number", problems)
        self.assertNotIn("checksum claim disagrees", problems)

    def test_template_trait_missing_asset_and_pin_are_reported(self):
        self.write_jsonl(self.traits, [])
        problems = self.problems()
        self.assertIn("asset synthetic.jpg: rows=0, linked fixtures=1", problems)
        self.assertIn("rows=0, EXPECTED pin=1", problems)
        self.assertIn(intake.TRAITS_REGEN, problems)

    def test_template_trait_extra_asset_is_reported(self):
        self.write_jsonl(self.traits, [{"source": [{"filename": "extra.jpg"}]}])
        problems = self.problems()
        self.assertIn("asset synthetic.jpg: rows=0, linked fixtures=1", problems)
        self.assertIn("asset extra.jpg: rows=1, linked fixtures=0", problems)

    def test_exempt_asset_has_no_registry_row_and_is_visible(self):
        intake.TRAITS_EXEMPT["synthetic.jpg"] = "synthetic issuer is absent"
        self.write_jsonl(self.traits, [])
        (self.test_dir / "template_traits.rs").write_text(
            'const EXPECTED: &[(&str, bool, usize)] = &[("Td3", false, 0),];\n',
            encoding="utf-8",
        )
        output = io.StringIO()
        with contextlib.redirect_stdout(output):
            code = intake.main(self.root)
        self.assertEqual(code, 0)
        self.assertIn("exempt template_traits assets:", output.getvalue())
        self.assertIn("synthetic.jpg", output.getvalue())
        self.assertNotIn(self.zone.split("\n")[0], output.getvalue())

    def test_stale_exemption_fails_for_registry_row_or_missing_manifest(self):
        intake.TRAITS_EXEMPT["synthetic.jpg"] = "synthetic issuer is absent"
        intake.TRAITS_EXEMPT["missing.jpg"] = "no manifest row"
        problems = self.problems()
        self.assertIn("stale trait exemption for synthetic.jpg: registry rows=1", problems)
        self.assertIn("stale trait exemption for missing.jpg: linked fixtures=0", problems)

    def test_golden_missing_and_stale_rows_are_reported(self):
        self.write_jsonl(self.golden, [{"fixture": "derived/missing.json"}])
        problems = self.problems()
        self.assertIn("fixture derived/missing.json does not exist", problems)
        self.assertIn("root fixture synthetic.json: rows=0, expected 1", problems)
        self.assertIn(intake.GOLDEN_BLESS, problems)

    def test_existing_derived_golden_row_is_allowed(self):
        derived = self.fixtures / "derived"
        derived.mkdir()
        (derived / "other.json").write_text("{}", encoding="utf-8")
        self.write_jsonl(self.golden, [
            {"fixture": "synthetic.json"},
            {"fixture": "derived/other.json"},
        ])
        self.assertEqual(self.problems(), "")

    def test_private_path_and_redaction_disagreement_are_reported(self):
        self.manifest_row["dir"] = "private"
        self.manifest_row["filename"] = "synthetic_redacted.jpg"
        self.write_jsonl(self.corpus, [self.manifest_row])
        problems = self.problems()
        self.assertIn("manifest path enters samples/private", problems)
        self.assertIn("redaction flag disagrees with filename", problems)

    def test_images_present_ignores_private_and_metadata_images(self):
        private = self.samples / "private"
        private.mkdir()
        (private / "hidden.png").touch()
        (self.fixtures / "fixture.jpg").touch()
        self.assertFalse(intake.images_present(self.samples))
        passport = self.samples / "passports"
        passport.mkdir()
        (passport / "synthetic.PNG").touch()
        self.assertTrue(intake.images_present(self.samples))


if __name__ == "__main__":
    unittest.main()