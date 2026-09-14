- One specimen added to the real-specimen corpus (`samples/corpus.jsonl` 257 → 258 rows) from the
  specimen-acquisition loop's tenth scout cycle (`c10`): a Cambodian passport bio-page, attributed
  to the Royal Embassy of Cambodia in Washington D.C. Unlike every other specimen in this corpus,
  this is a real person's actual document rather than an official blank template -- admitted only
  because every personally-identifying field, including the MRZ band itself, is fully redacted
  (see `knowledge/SPECIMEN_SOURCES.md`-adjacent H5 revision in the specimen-loop plan, 2026-09-14).
  Two related candidates from the same embassy page were *not* admitted: a K-Visa image whose
  issue date, expiry date and visa number were left unredacted (still rejected), and an older
  passport series that landed on the `samples/local/` track instead of this public one (user
  verdict). `KHM` stays *No specimen yet* -- the redaction removes the MRZ entirely, so this adds
  document-type depth without a coverage HIT. This cohort started its own branch rather than
  reopening PR #286, which had already crossed the 5-specimen batch floor and had a re-bless in
  flight. See `work/scouting/c10/packet-c10.md` for full review provenance.
