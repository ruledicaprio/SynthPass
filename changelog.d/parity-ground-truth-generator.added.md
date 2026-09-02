- **Tier-2 accuracy is measurable.** `crates/synthpass-llm/tests/parity.rs` — the only measurement
  of Tier-2 accuracy in the repository, and the one `knowledge/prompts/README.md` requires per
  `PROMPT_VERSION` bump — ran over **six documents**. Six cannot separate a regression from
  sampling noise, so in practice `knowledge/technical_debt.md`'s "the accuracy of the shipped
  Tier-2 path is unguarded between releases" was the whole story. A new generator,
  `crates/synthpass-bench/examples/ground_truth_candidates.rs`, derives a candidate fixture from
  every corpus specimen whose MRZ reads with valid check digits.

- **Fixtures are split by review status, and that split is the load-bearing part.** ICAO 9303
  check-digits exactly four fields — `document_number`, `date_of_birth`, `date_of_expiry`,
  `personal_number` — and the composite covers the same ranges and no more. So a generated
  fixture's *name* is not ground truth: it is one OCR pass's reading of characters nothing
  arbitrates, on exactly the positions this corpus shows the recogniser getting wrong. Scoring a
  model against them would **invert** the measurement — a model that read the printed name
  correctly would be marked wrong for disagreeing with an OCR error, so improving visual-zone
  handling would register as a regression. Hence `samples/ocr_fixtures/` (hand-verified, all nine
  prompt fields scored) and `samples/ocr_fixtures/derived/` (generated, check-digited fields only).
  Promotion is `git mv` one level up after a person checks the unproven fields against the image.
  The split is read from `FieldConfidence::mrz_checksum_scope` rather than restated, via a new
  `FieldConfidence::score(CoreField)` — the read-side counterpart to the existing `prove`.

- **Valid check digits are not enough to make a fixture, which the first run proved.**
  `Croatia_ID_Specimen_2002_back` OCR'd to a zone whose every date field was filler, and
  `MrzData::valid()` accepted it: the ICAO weighting scores `<` as zero, so an all-filler field
  sums to zero and its `<` check digit verifies vacuously. Only the document number's digit was a
  real 1-in-10 coincidence. The resulting record claimed a date of expiry of `<<<<<<`, a
  nationality of `OOO` and a surname of `OOOOOOOOOO K`, with `mrz_checksums_valid: true`. The
  generator now gates candidates on the crate's own deterministic cross-checks —
  `fusion::check_line1_integrity` plus the two date judgements that don't depend on today — rather
  than on the checksum alone. This is what `RoutingPolicy::escalate_on_line1_flagged` was added
  for; the rejection list is a first source of the data that hook is waiting on.

- **`parity.rs` reports a per-field breakdown and two separate floors.** Pooling a large easy set
  with a small hard one lets the first hide a regression in the second. The nine scored fields are
  now exactly the ones `prompt::build_prompt` asks for (up from an arbitrary seven), with the old
  seven-field rate still reported alongside so the historical baseline stays comparable across the
  change. The fixture list comes from the two directories rather than from `corpus.jsonl`'s
  `ground_truth_stem` links: a fixture pair is self-contained, and listing the directory picks up a
  promotion with no manifest regeneration.

- **Every `.md` fixture is regenerated from the engine that actually ships.** The six that existed
  came from `docling`, retired in v0.7.5 — HTML-escaped (`P&lt;HRV…`), with `##` headings and
  `<!-- image -->` placeholders. The pipeline feeds Tier 2 `NativeOcr::recognize`'s plain text,
  which is none of those, so the harness had been measuring the model against an input shape
  production no longer produces.

- **`knowledge/VIZ_TIER2_DESIGN.md`** — the written design for visual-zone work in Tier 2, and the
  correction that motivates it: the model already receives the full-page text, VIZ included, so the
  work is not "feed the VIZ in" but four separable things — clean it, label it, ask narrowly, add
  the fields only the VIZ carries — in that order. §5 records the bias that remains (every fixture
  has a checksum-valid MRZ, and Tier 2 only runs when Tier 1 fails, so the parity corpus is by
  construction the set of documents Tier 2 never sees) and the MRZ-holdout variant that closes it.
