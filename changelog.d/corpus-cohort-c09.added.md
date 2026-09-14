- Four specimens added to the real-specimen corpus (`samples/corpus.jsonl` 261 → 265 rows) from
  the specimen-acquisition loop's ninth scout cycle (`c09`), folded into the accumulating cohort
  PR from `c03`/`c07`: a Singapore passport design illustration (attributed to the Immigration &
  Checkpoints Authority), a Philippine PhilSys national ID sample (public domain, attributed to
  the Philippine Statistics Authority), and two Hong Kong passport design illustrations
  (2019 + 2007, attributed to the Immigration Department). `SGP` moves *No specimen yet* → **HIT**
  — a genuine checksum-valid TD3, confirmed by both `check_sample` and this manifest's OCR pass,
  the first clean HIT this loop has produced outside Germany's partial case. `HKG` stays *No
  specimen yet*: both passports fail checksum on this OCR pass (the project's known
  single-character-misread pattern, see c03's German row). `PHL` stays *No specimen yet*: the
  PhilSys card has no MRZ zone at all (a national ID, not a travel document). This crosses the
  cohort batch-size floor (5 verified `public` specimens minimum): running total is now 8/5,
  triggering the merge tail for the first time since the rule was adopted. See
  `work/scouting/c09/packet-c09.md` for full review provenance, including the licence note: none
  of the ICA/ImmD pages granted an explicit reuse licence, so `origin.licence` stays honestly
  `none-stated` even though the user's verdict placed all four rows on the public track "with
  attribution" — the attribution itself lives in each row's `origin.notes`.
