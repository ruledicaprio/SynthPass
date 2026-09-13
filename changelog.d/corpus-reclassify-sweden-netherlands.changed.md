- **Two real specimens with no ICAO zone left the Tier-1 denominator.** A
  [manifest review](knowledge/benchmarks/manifest-review-no-mrz-found-2026-09-13.md)
  found `Sweden_ID_Specimen_2027_mrz` is the card *front* — its `_mrz` tag was a
  human error — and that the Netherlands driving licence prints one 30-character
  line that is not ICAO 9303 in any format. Both were renamed on `samples-data`
  to carry `_no_mrz` and now score `no_mrz_expected`. The CI-written baseline
  moves `scored` 157 → 155 and `no_mrz_found` 7 → 5 with the HIT count unchanged,
  so the headline becomes **143 / 155 = 92.3%**, and `checksum_failed` (7) is
  now the larger scored miss. The Sweden coverage row, which said "No specimen
  yet", now records four specimens and two HITs.
