- Two specimens added to the real-specimen corpus (`samples/corpus.jsonl` 257 → 259 rows) from
  the specimen-acquisition loop's third scout cycle (`c03`): a German passport biodata page and
  ID-card front (`D`, the legacy single-letter code), both sourced from the official
  Bundesgesetzblatt (`gesetze-im-internet.de`) and carrying the standard "Erika Mustermann"
  placeholder identity with a MUSTER (SPECIMEN) watermark. `D` moves *No specimen yet* →
  *MISS (checksum failed)* in `knowledge/CORPUS_COVERAGE.md` — not yet a HIT: the passport's MRZ
  reads but fails checksum on this OCR pass (a likely single-character misread, same pattern as
  the pre-existing `D<<` passport fixture), and the ID card is front-only, so it carries no MRZ
  at all (`Personalausweis` MRZ is on the back). See that file for the per-country notes and
  `work/scouting/c03/packet-c03.md` for full review provenance.
