# M6 checksum-failed miss mechanisms

**Date:** 2026-09-18 · **MAIN:** `d2d5855` · **DATA:** `5988c7bd878e58cf0279bbe078b0e6a679ac46af` · **Evidence:** Observed (native runner, `--dump-ocr`) · **Status:** current

This note closes the six previously unattributed frozen M6 `checksum_failed` documents under [ADR-0011](../decisions/ADR-0011-split-m6-packaging-into-m8.md)'s attribution criterion. The run used the default `mrz` provider with `--real-specimens --mrz-only --dump-ocr`, every `SYNTHPASS_OCR_*` arm cleared so the attribution describes shipped behaviour: **257 documents, `checksum_failed = 10`**, and all six frozen targets present in the dump, one row each. The recorded fixtures are checksum-valid, so each is a recognition or segmentation miss rather than an expected refusal.

Each entry below is derived by aligning the read zone against the reviewed fixture **position by position** and reporting which ICAO field each differing position falls in. Positions are 0-based within a line. **No MRZ content, name or character value is reproduced here** — only field names, positions and counts.

## Why check states are themselves evidence

### Historical notation and current wire shape

The bracketed `failing_checks = [...]` signatures below are literal observations
from the 2026-09-18 failed-only report format. They deliberately remain quoted
as historical evidence: that format recorded failures but omitted the passing
and not-printed states, so its exact rows cannot be reconstructed as today's
five-key `check_states` map without inventing observations. Current parsed rows
record `check_states` as `true` (verified), `false` (failed), or `null` (the
identified layout does not print that digit); no-MRZ rows omit the map entirely.

Two facts about ICAO 9303 do most of the work in the entries below, and both are load-bearing:

- **The composite check digit covers some fields and not others.** For TD3 it spans line 2 positions 0–9, 13–19 and 21–42; for TD1, line 1 positions 5–29 plus line 2 positions 0–6, 8–14 and 18–28. **Nationality and sex are excluded from both.** A character error there moves no check digit at all.
- **Some covered fields have no check digit of their own.** TD1's optional data has none; TD3's personal number has one. In the legacy failed-only notation, a field inside the composite but without its own digit produces `failing_checks == ["composite"]`; current rows record the same fact as a `check_states` map whose only `false` entry is `composite`.

## Afghanistan passport specimen

**Observed:** TD3, `failing_checks = [document_number, composite]`, band score 0.898. Line 1 is character-for-character correct. Line 2 differs in **exactly one position: 0** — the first cell of the document number. Whole-zone mismatch: 1 character.

**Mechanism:** A single-glyph substitution in the document number's first cell. Because the composite's input includes the document number, **one wrong character fails two checks** — which is why `document_number` and `composite` co-occur here rather than indicating two defects. This is the narrowest possible recognition miss: 1 of 88 cells.

## Belgium identity-card specimen

**Observed:** TD1, `failing_checks = [composite]` **only**, band score 0.794. Line 1 is character-for-character correct — document number, its check digit and optional data 1 all clean. Line 2 differs in 8 positions: **7 inside optional data 2 (18–28)** and the composite check digit itself (29). Nationality (15–17), sex (7), both dates and both date check digits are all correct. Line 3 (names) differs in 9 positions.

**Mechanism:** The error is **confined to optional data 2**, the one TD1 line-2 field carrying no check digit of its own. Only the composite can observe it, which is exactly the failing-check set the run reports. The `[composite]`-alone signature is not a weak signal here — it is a positional fingerprint, and it points at the field the format leaves unverified.

## Czech identity-card specimen

**Observed:** TD3, `failing_checks = [personal_number, composite]`, band score 0.772. Line 2 differs in **2 positions, 33 and 41**, both inside the personal-number field (28–41). Line 1 differs in 1 position (29, inside the name field). All lines are full length. Whole-zone mismatch: 3 characters.

**Mechanism:** Two isolated substitutions inside the personal number. **There is no shift:** a displaced run would misalign every position downstream, and positions 34–40 are correct. The dump does not determine which print feature caused either substitution.

## Romania passport specimen

**Observed:** TD3, `failing_checks = [personal_number, composite]`, band score 0.525. Line 2 differs in **exactly one position, 41** — the last cell of the personal-number field. Line 1 differs in **22 positions**, all inside the name field. Whole-zone mismatch: 23 characters.

**Mechanism:** A single-cell substitution at the end of the personal number accounts for **both** failing checks. The striking fact is the split: 1 wrong character on line 2 and 22 on line 1. **The check digits see the one and are blind to the twenty-two**, because no ICAO check digit covers the name field ([ADR-0013](../decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md)). Treating this document as "a personal-number miss" would describe 1/23rd of the damage.

## Russian Federation passport specimen

**Observed:** **The reviewed fixture is TD3 — two lines of 44. The reader returned three lines of 30, a TD1 shape.** `failing_checks = [document_number, date_of_birth, composite]`, `mrz_band_score = null`, whole-zone mismatch 112 characters, 63 raw OCR lines in the dump.

**Mechanism:** This is **not a recognition miss.** It is a format/segmentation failure: the band was never scored, the wrong document format was selected, and the check digits were then computed over a zone that does not correspond to the printed one. The failing-field list is an artefact of parsing a TD1 out of a TD3 and carries no information about which characters were misread.

**This bears on classification, not just attribution.** A document that never located its zone belongs with the detection misses ([ADR-0015](../decisions/ADR-0015-geometric-mrz-band-location.md)'s territory), not with recognition failures — no recogniser improvement of any quality would change this outcome. **Recommended, not applied here:** review whether this document should be reclassified. That is a separate change and needs its own approval.

## Sweden identity-card specimen

**Observed:** TD1, `failing_checks = [date_of_expiry, composite]`, band score 0.399 — the lowest of the six. Line 1 is character-for-character correct. Line 2 differs in **five contiguous positions, 13–17**: the final digit of the expiry date (13), the expiry check digit (14), and **all three nationality characters (15–17)**. Line 3 differs in 22 positions. Whole-zone mismatch: 27 characters.

**Mechanism:** A contiguous five-cell disturbance spanning an expiry/nationality boundary, on the weakest band score in the set. The check digits catch the first two cells and are **structurally blind to the other three**: nationality is excluded from TD1's composite, so all three characters are wrong and no check digit anywhere in the format can say so. Repairing the expiry alone would turn this document into a Tier-1 *hit* still carrying a wrong nationality. The dump does not determine whether print, security texture or perspective initiated the run.

## What the six have in common

**The checksum failures are tiny, and they sit where the format is weakest.** Differing positions on the check-digit-bearing line: Afghanistan 1, Romania 1, Czechia 2, Sweden 5, Belgium 7. Five of the six are recognition misses of **one to seven cells**, and every one of those cells falls in the document number, the personal/optional data, or a date — never in a field the format constrains tightly.

**The damage the check digits cannot see is far larger.** Name-field differences in the same six reads: Romania 22, Sweden 22, Belgium 9, Czechia 1. Plus Sweden's three nationality characters. None of it moves a check digit, and a document whose composite is repaired would report as a clean hit while still carrying it.

**And one of the six is not a recognition miss at all** — the Russian Federation specimen is a format misdetection that recognition work cannot reach.

## Scope

Documentation only. No code, no data, no manifest, no `samples-data`, and no baseline number moves. The `--dump-ocr` output containing read text stays in `artifacts/` and is not committed.

One observation recorded without being chased: this run reports `no_mrz_found = 4` over 257 documents, where the committed baseline records 3 over its own population. The two runs do not share a denominator, so the figures are not in conflict — but the difference has not been explained here.
