"""Synthetic checks for the fixture checksum audit."""

import contextlib
import io
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import audit_fixture_checksums as audit


TD3 = ("P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n"
       "L898902C36UTO7408122F1204159ZE184226B<<<<<10")
TD1 = ("I<UTOD231458907<<<<<<<<<<<<<<<\n"
       "7408122F1204159UTO<<<<<<<<<<<6\n"
       "ERIKSSON<<ANNA<MARIA<<<<<<<<<<")


class FixtureChecksumAuditTests(unittest.TestCase):
    def test_doc_9303_td3_specimen_is_valid(self):
        result = audit.audit_zone(TD3)
        self.assertEqual(result.format, "TD3")
        self.assertTrue(result.valid)

    def test_changed_td3_composite_fails_only_composite(self):
        lines = TD3.split("\n")
        lines[1] = lines[1][:-1] + "1"
        result = audit.audit_zone("\n".join(lines))
        self.assertEqual(result.failing, ("composite",))

    def test_td1_checks_are_valid(self):
        result = audit.audit_zone(TD1)
        self.assertEqual(result.format, "TD1")
        self.assertTrue(result.valid)

    def test_td2_and_mrv_examples_use_their_own_check_sets(self):
        examples = (
            ("TD2", "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n"
             "D231458907UTO7408122F1204159<<<<<<<6"),
            ("MRV-A", "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n"
             "L898902C<3UTO6908061F9406236ZE184226B<<<<<<<"),
            ("MRV-B", "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n"
             "L898902C<3UTO6908061F9406236ZE184226"),
        )
        for format_name, zone in examples:
            with self.subTest(format=format_name):
                result = audit.audit_zone(zone)
                self.assertEqual(result.format, format_name)
                self.assertTrue(result.valid)

    def test_td1_long_document_number_uses_optional_field_digit(self):
        line1, line2, line3 = TD1.split("\n")
        number = "A12345678"
        optional = "Z" + audit.check_digit(number + "Z") + "<" + "<" * 12
        line1 = line1[:5] + number + "<" + optional
        composite = line1[5:30] + line2[0:7] + line2[8:15] + line2[18:29]
        line2 = line2[:29] + audit.check_digit(composite)
        self.assertTrue(audit.audit_zone("\n".join((line1, line2, line3))).valid)

    def test_unused_td3_personal_number_accepts_filler_digit(self):
        line1, line2 = TD3.split("\n")
        line2 = line2[:28] + "<" * 15 + line2[43]
        composite = line2[0:10] + line2[13:20] + line2[21:43]
        line2 = line2[:43] + audit.check_digit(composite)
        self.assertTrue(audit.audit_zone("\n".join((line1, line2))).valid)

    def test_claim_mismatch_is_reported_without_zone_text(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "synthetic.json"
            path.write_text(json.dumps({"mrz_line": TD3, "mrz_checksums_valid": False}),
                            encoding="utf-8")
            output = io.StringIO()
            with patch.object(audit, "FIXTURES", Path(directory)), contextlib.redirect_stdout(output):
                code = audit.main()
        self.assertEqual(code, 1)
        self.assertIn("mismatched=1", output.getvalue())
        self.assertIn("failing=none", output.getvalue())
        self.assertNotIn(TD3.split("\n")[0], output.getvalue())

    def test_malformed_zone_is_counted(self):
        self.assertEqual(audit.audit_zone("BAD\nZONE").anomaly, "malformed")


if __name__ == "__main__":
    unittest.main()
