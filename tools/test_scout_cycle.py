#!/usr/bin/env python3
"""
Offline unit tests for tools/scout_cycle.py. No network, no git, no dsh --
every test exercises pure functions against fixtures, matching
tools/test_apply_cohort.py's pattern.

Run with:
    python -m unittest tools/test_scout_cycle.py
"""
from __future__ import annotations

import os
import sys
import unittest
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import scout_cycle as scy  # noqa: E402

COUNTRIES_RS = '''
const CODES: &[(&str, &str)] = &[
    // ── Africa ──
    ("DZA", "Algeria"),
    ("BWA", "Botswana"),
    // ── Asia ──
    ("THA", "Thailand"),
    ("HKG", "Hong Kong"),
    ("LAO", "Lao People's Democratic Republic"),
    // ── Europe ──
    ("D", "Germany"),
    ("DEU", "Germany"),
    // ── ICAO 9303 special / non-ISO codes ──
    ("XPO", "International Criminal Police Organization (INTERPOL)"),
    ("UTO", "Utopia (ICAO specimen)"),
];
'''

WORKER_OUT = '''dsh: reasoning:
Let me think. The template is:
{"code":"","doc":"passport","side":"biodata","td":"TD3","series":"","image_url":"...","page_url":"...","host_kind":"consular","licence_evidence":"none stated","specimen_signal":"page-says-specimen","notes":"one short line"}
Draft record (will revise):
{"code":"KHM","doc":"passport","side":"biodata","td":"TD3","series":"draft","image_url":"https://x.example/a.jpg","page_url":"https://x.example/p.html","host_kind":"consular","licence_evidence":"none stated","specimen_signal":"page-says-specimen","notes":"draft"}
dsh: reasoning:
Finalising.
- THA: none found — looked at MFA pages, Commons category
- KHM: 2 found
{"code":"KHM","doc":"passport","side":"biodata","td":"TD3","series":"new","image_url":"https://x.example/a.jpg","page_url":"https://x.example/p.html","host_kind":"consular","licence_evidence":"none stated","specimen_signal":"page-says-specimen","notes":"final"}
{"code":"KHM","doc":"visa","side":"full","td":"unknown","series":"K","image_url":"https://x.example/k.jpg","page_url":"https://x.example/p.html","host_kind":"consular","licence_evidence":"none stated","specimen_signal":"page-says-specimen","notes":"visa"}
not json {"broken": }
'''

STATE_MD = """# Specimen loop — state
updated: 2026-09-13T22:35Z · plan: ~/.claude/plans/specimen-acquisition-loop.md (v2)

## Locks
samples-data lock: free

## Codes
exhausted (2/2): EUE, XPO, XOM (c02); UNO, MNE (c03)
now 1/2: XCC, RKS (c06)
tried once, none found (c10): THA, MAC
resolved: SGP — flipped to HIT via cohort c09/PR #286
**c07:** DZA/BWA all none found (covers only)
claimed this cycle: —

## Cycles
| cycle | codes | wall | tokens u/cr/out | scouted | screened | packet | pub/loc/drop | review min | faults |
|---|---|---|---|---|---|---|---|---|---|
| c09 | SGP NZL | ~584 s (9m44s) | 113.7k / 1.95M / 36.2k | 4 | 4 | 4 | 4/0/0 | ~immediate | 0 |

\\* footnote line about c01.

## Log (append-only, one line per step)
2026-09-13T17:00Z T0 done.
"""

COVERAGE_MD = """## Full table
| Code | Country | Docs | Status | Note |
|---|---|---|---|---|
| DZA | Algeria | Passport | MISS (checksum failed) | scan |
| BWA | Botswana | -- | No specimen yet | -- |
| THA | Thailand | -- | No specimen yet | -- |
| HKG | Hong Kong | Passport | HIT | -- |
| LAO | Lao PDR | -- | No specimen yet | -- |
| XPO | INTERPOL | -- | No specimen yet | -- |
| UTO | Utopia | -- | No specimen yet | -- |
"""


class CountryTableTests(unittest.TestCase):
    def test_parses_codes_with_regions(self):
        table = scy.parse_country_table(COUNTRIES_RS)
        self.assertIn(("Asia", "THA", "Thailand"), table)
        self.assertIn(("Europe", "D", "Germany"), table)
        self.assertIn(("ICAO 9303 special / non-ISO codes", "UTO", "Utopia (ICAO specimen)"), table)
        self.assertEqual(len(table), 9)

    def test_real_countries_rs_parses_every_row(self):
        real = Path(__file__).resolve().parent.parent / "crates" / "mrz" / "src" / "countries.rs"
        if not real.is_file():
            self.skipTest("countries.rs not present")
        table = scy.parse_country_table(real.read_text(encoding="utf-8"))
        codes = {c for _, c, _ in table}
        self.assertGreater(len(table), 200)
        for expected in ("DEU", "D", "UTO", "XXA", "GBD"):
            self.assertIn(expected, codes)


class PrefixTests(unittest.TestCase):
    def test_prefix_must_end_with_task_line(self):
        self.assertEqual(scy.load_prefix_text("ROLE: x\r\n\r\nTASK:\r\n"), "ROLE: x\n\nTASK:\n")
        with self.assertRaises(ValueError):
            scy.load_prefix_text("ROLE: x\nTASK:\n\n")
        with self.assertRaises(ValueError):
            scy.load_prefix_text("ROLE: x\n")

    def test_shipped_prefix_is_v3(self):
        prefix_path = Path(__file__).resolve().parent / scy.PREFIX_FILENAME
        raw = scy.load_prefix_text(prefix_path.read_text(encoding="utf-8"))
        self.assertTrue(raw.startswith("ROLE: You are a research scout."))
        text = " ".join(raw.split())  # phrases may wrap across lines
        # v2 (plan §8.1): tell the worker the full name is the answer.
        self.assertIn("do not spend time researching what a code stands for", text)
        self.assertIn("NEVER: consilium.europa.eu/prado", text)
        # v3 (plan §8.1): official-host-only signal, PDFs reported unopened, BLOCKED lines.
        self.assertIn('"specimen_signal":"official-host-only"', text)
        self.assertIn('"format":"pdf"', text)
        self.assertIn("BLOCKED: <url>", text)


class SuffixTests(unittest.TestCase):
    CORPUS = [
        {"dir": "passports", "filename": "Slovakia_Passport_Specimen_P0_SVK_2005_mrz.png", "year": {"kind": "issue", "value": 2005}, "mrz": {"issuing_state": "SVK"}},
        {"dir": "passports", "filename": "Slovakia_Passport_Specimen_PS_SVK_2014_mrz.jpg", "year": {"kind": "issue", "value": 2014}, "mrz": {"issuing_state": "SVK"}},
        {"dir": "passports", "filename": "Germany_Passport_Specimen_P0_D00_2018_mrz.webp", "year": {"kind": "issue", "value": 2018}, "mrz": {"issuing_state": "D<<"}},
        {"dir": "id_cards", "filename": "Germany_ID_Specimen_2021_front_no_mrz.jpg", "year": {"kind": "issue", "value": 2021}, "mrz": {"issuing_state": None}},
        {"dir": "passports", "filename": "Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz.png", "year": {"kind": "issue", "value": 2019}, "mrz": {"issuing_state": "HKG"}},
    ]

    def test_held_series_by_state_and_by_filename(self):
        self.assertEqual(scy.held_series(self.CORPUS, "SVK", "Slovakia"), "passport 2005, 2014")
        self.assertEqual(scy.held_series(self.CORPUS, "D", "Germany"), "passport 2018; id-card 2021")
        self.assertEqual(scy.held_series(self.CORPUS, "HKG", "Hong Kong"), "passport 2019")
        self.assertEqual(scy.held_series(self.CORPUS, "THA", "Thailand"), "none held")

    def test_suffix_shape_matches_plan_section_8(self):
        names = {"THA": "Thailand", "KHM": "Cambodia"}
        held = {"THA": "none held", "KHM": "none held"}
        self.assertEqual(
            scy.build_suffix(["THA", "KHM"], names, held),
            "Codes to scout: THA (Thailand), KHM (Cambodia). Already held (skip these series): THA: none held; KHM: none held.\n",
        )


class CandidateExtractionTests(unittest.TestCase):
    def test_final_records_win_and_templates_are_dropped(self):
        candidates, per_code = scy.extract_candidates(WORKER_OUT)
        self.assertEqual(len(candidates), 2)
        self.assertEqual(candidates[0]["notes"], "final")  # the draft was superseded
        self.assertEqual(candidates[1]["doc"], "visa")
        self.assertEqual(per_code["KHM"], "2")
        self.assertTrue(per_code["THA"].startswith("none found"))
        self.assertIn("MFA pages", per_code["THA"])

    def test_blocked_lines(self):
        out = (
            "- THA: none found — looked\n"
            "BLOCKED: https://a.example/p.html — 403 Cloudflare\n"
            "- BLOCKED: https://b.example/x.pdf: unsupported content type application/pdf\n"
            "BLOCKED: https://a.example/p.html — retried, still 403\n"
            "BLOCKED: not-a-url — junk\n"
        )
        blocked = scy.extract_blocked(out)
        self.assertEqual([b["url"] for b in blocked], ["https://b.example/x.pdf", "https://a.example/p.html"])
        self.assertEqual(blocked[1]["reason"], "retried, still 403")
        self.assertEqual(blocked[0]["reason"], "unsupported content type application/pdf")
        self.assertEqual(scy.extract_blocked("nothing here"), [])

    def test_tagging(self):
        tagged = scy.tag_candidates([{"code": "KHM"}], "c11", "w2")
        self.assertEqual(tagged[0], {"code": "KHM", "status": "unreviewed", "cycle": "c11", "worker": "w2"})


class TokenAndFaultTests(unittest.TestCase):
    SESSION = {
        "version": 7,
        "record": {
            "identity": {"cwd": "D:\\Projects\\worktrees\\SynthPass\\scout-c11-w1", "createdAt": 1789369505820},
            "rows": {"tokenUsage": {"ver": 2, "val": {"totals": {"uncachedInputTokens": 205073, "outputTokens": 21123, "cacheReadTokens": 2360704, "cacheWriteTokens": 0}}}},
        },
    }

    def test_token_totals(self):
        totals = scy.parse_token_totals(self.SESSION)
        self.assertEqual(totals, {"uncached": 205073, "cache_read": 2360704, "output": 21123})
        self.assertEqual(scy.format_tokens(totals), "205.1k / 2.36M / 21.1k")
        self.assertEqual(scy.format_tokens(None), "unrecorded")
        self.assertIsNone(scy.parse_token_totals({"record": {}}))

    def test_session_matching_is_by_worktree_and_time(self):
        wt = Path("D:/Projects/worktrees/SynthPass/scout-c11-w1")
        self.assertTrue(scy.session_matches(self.SESSION, wt, 1789369505000))
        self.assertFalse(scy.session_matches(self.SESSION, wt, 1789369506000))
        self.assertFalse(scy.session_matches(self.SESSION, Path("D:/Projects/worktrees/SynthPass/scout-c11-w2"), 0))

    def test_faults_outside_scratch(self):
        status = "?? scratch/notes.txt\n?? findings.md\n M README.md\n"
        self.assertEqual(scy.detect_faults(status), ["wrote outside scratch/: findings.md", "wrote outside scratch/: README.md"])
        self.assertEqual(scy.detect_faults(""), [])

    def test_format_wall(self):
        self.assertEqual(scy.format_wall(284.4), "~284 s (4m44s)")


class PacketRowsTests(unittest.TestCase):
    def test_rows_mirror_write_packet_and_skip_rejects(self):
        screened = [
            {
                "code": "KHM", "doc": "passport", "td": "TD3", "series": "new",
                "page_url": "https://www.embassy.example/p.html", "page_url_final": "https://www.embassy.example/p.html",
                "licence_evidence": "none stated", "licence_snippets": ["© Royal Embassy"],
                "specimen_signal": "page-says-specimen", "specimen_snippets": [],
                "mrz_check": "invalid", "staged_path": "work/scouting/c11/staging\\9c23d8fbd053.jpg",
                "needs_eyes": True, "variant_of": None, "auto_reject": None, "sha256": "9c23d8fbd053aaaa",
            },
            {"code": "LAO", "auto_reject": "unresolvable", "staged_path": None},
        ]
        rows = scy.packet_rows_from_screened(screened)
        self.assertEqual(len(rows), 1)
        row = rows[0]
        self.assertEqual(row["id"], "9c23d8fbd053")
        self.assertEqual(row["doc_td"], "passport / TD3")
        self.assertEqual(row["host"], "www.embassy.example")
        self.assertEqual(row["licence_evidence"], "© Royal Embassy")
        self.assertEqual(row["specimen_signal"], "page-says-specimen")
        self.assertEqual(row["local_path"], "work/scouting/c11/staging/9c23d8fbd053.jpg")
        self.assertEqual(row["note"], "needs eyes")
        self.assertEqual(row["proposed_destination"], "")
        self.assertEqual(row["proposed_licence"], "")
        for field in ("id", "code", "doc_td", "series", "host", "licence_evidence", "specimen_signal", "mrz_check", "proposed_destination", "proposed_licence", "local_path", "note"):
            self.assertIn(field, row)


class SuggestTests(unittest.TestCase):
    def test_uncovered_minus_state_history_minus_orgs(self):
        table = scy.parse_country_table(COUNTRIES_RS)
        uncovered = scy.coverage_uncovered_codes(COVERAGE_MD)
        self.assertEqual(uncovered, ["BWA", "THA", "LAO", "XPO", "UTO"])
        known = {c for _, c, _ in table}
        mentioned = scy.state_codes_mentioned(STATE_MD, known)
        self.assertIn("THA", mentioned)
        self.assertIn("XPO", mentioned)
        self.assertIn("DZA", mentioned)  # from the slash-separated narrative line
        self.assertNotIn("LAO", mentioned)
        self.assertNotIn("PR", mentioned)  # not a code, so the intersection drops it
        picks = scy.suggest_codes(table, uncovered, mentioned, None, include_orgs=False)
        self.assertEqual([c for _, c, _ in picks], ["LAO"])
        picks = scy.suggest_codes(table, uncovered, mentioned, "asia", include_orgs=True)
        self.assertEqual([c for _, c, _ in picks], ["LAO"])
        picks = scy.suggest_codes(table, uncovered, set(), None, include_orgs=True)
        self.assertEqual([c for _, c, _ in picks], ["BWA", "THA", "LAO", "XPO", "UTO"])


class StateEditTests(unittest.TestCase):
    def test_insert_cycle_row_goes_after_last_table_row(self):
        row = scy.cycle_row("c11", "THA MAC", "~284 s (4m44s)", "205.1k / 2.36M / 21.1k", 4, 0)
        out = scy.insert_cycle_row(STATE_MD, row)
        lines = out.splitlines()
        i = lines.index("| c09 | SGP NZL | ~584 s (9m44s) | 113.7k / 1.95M / 36.2k | 4 | 4 | 4 | 4/0/0 | ~immediate | 0 |")
        self.assertEqual(lines[i + 1], row)
        self.assertTrue(lines[i + 2] == "")
        self.assertIn("\\* footnote line about c01.", out)

    def test_update_cycle_cells(self):
        row = scy.cycle_row("c11", "THA MAC", "~284 s (4m44s)", "unrecorded", 4, 1)
        text = scy.insert_cycle_row(STATE_MD, row)
        text = scy.update_cycle_cells(text, "c11", {"screened": "3", "packet": "3"})
        self.assertIn("| c11 | THA MAC | ~284 s (4m44s) | unrecorded | 4 | 3 | 3 | TBD | TBD | 1 |", text)
        self.assertEqual(scy.update_cycle_cells(text, "c77", {"screened": "1"}), text)

    def test_append_log_and_touch_updated(self):
        out = scy.append_log(STATE_MD, "c11 scout w1 done")
        self.assertTrue(out.rstrip("\n").endswith(" c11 scout w1 done"))
        self.assertIn("2026-09-13T17:00Z T0 done.", out)
        touched = scy.touch_updated(out)
        self.assertNotIn("updated: 2026-09-13T22:35Z", touched)
        self.assertRegex(touched.splitlines()[1], r"^updated: \d{4}-\d{2}-\d{2}T\d{2}:\d{2}Z ·")

    def test_add_codes_line_lands_at_end_of_codes_section(self):
        out = scy.add_codes_line(STATE_MD, "tried (c11), none found: ARM BTN")
        lines = out.splitlines()
        i = lines.index("tried (c11), none found: ARM BTN")
        self.assertEqual(lines[i - 1], "claimed this cycle: —")
        self.assertEqual(lines[i + 1], "")
        self.assertEqual(lines[i + 2], "## Cycles")
        known = {"ARM", "BTN", "THA"}
        self.assertEqual(scy.state_codes_mentioned(out, known), known)

    def test_missing_cycles_table_is_created_not_lost(self):
        out = scy.insert_cycle_row("# state\n", "| c11 | X | w | t | 0 | — | — | TBD | TBD | 0 |")
        self.assertIn("## Cycles", out)
        self.assertIn("| c11 | X |", out)


class EnvTests(unittest.TestCase):
    def test_dsh_home_from_env(self):
        self.assertEqual(scy.resolve_dsh_home({"DSH_HOME": "D:\\DevCache\\dsh"}), "D:\\DevCache\\dsh")

    def test_worktrees_root_layout(self):
        self.assertEqual(scy.worktrees_root(Path("D:/Projects/SynthPass")), Path("D:/Projects/worktrees/SynthPass"))


if __name__ == "__main__":
    unittest.main()
