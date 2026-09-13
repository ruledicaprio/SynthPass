- **Four real specimens that can never yield a hit left the Tier-1 denominator.** A
  [manifest review](knowledge/benchmarks/manifest-review-no-mrz-found-2026-09-13.md)
  and the reclassification that followed found four of the seven `no_mrz_found`
  documents unreadable by construction: `Sweden_ID_Specimen_2027` is a card *front*
  (its `_mrz` tag was a human error) and the Netherlands driving licence prints a
  line that is not ICAO 9303 — both renamed `_no_mrz`; Egypt 2012's zone is
  pixel-masked by the publisher — renamed `_redacted_mrz`; and Argentina 2021
  child's printed zone fails four of its five check digits — given a hand-verified
  fixture, so it scores `checksum_failed_specimen`. The CI-written baseline moves
  `scored` 157 → 153 and `no_mrz_found` 7 → 3 with the HIT count unchanged, so the
  headline becomes **143 / 153 = 93.5%**, and `checksum_failed` (7) is now the
  larger scored miss. The Sweden coverage row, which said "No specimen yet", now
  records four specimens and two HITs.
