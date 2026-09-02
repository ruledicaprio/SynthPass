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

- **Every `.md` fixture is regenerated from the engine that actually ships, and it was worth half
  the measured accuracy.** The six that existed came from `docling`, retired in v0.7.5 —
  HTML-escaped (`P&lt;HRV…`), with `##` headings and `<!-- image -->` placeholders. The pipeline
  feeds Tier 2 `NativeOcr::recognize`'s plain text, which is none of those, so the harness had been
  measuring the model against an input shape production no longer produces. A same-binary A/B over
  the same six documents, changing only the `.md` source: docling **9/54 (16.7%)**, `NativeOcr`
  **18/54 (33.3%)**; `document_number` alone 1/6 → 4/6. The docling arm reproduced byte-identically
  on a repeat run, so the delta is not sampling noise.

- **Measured baseline over the full set** (`qwen2.5-1.5b-instruct-q4_k_m`, greedy, ~31 min):
  reviewed **43/162 (26.5%)** over 18 documents, derived **47/162 (29.0%)** over 54, repair
  fallbacks **0**. The two sets agree on the one field both score (`document_number`: 50% vs 54%),
  which is the check that generated fixtures are not systematically easier than hand-verified ones.
  The floors move from 0.25 to 0.15 — keeping 0.25 while widening the scored set from seven fields
  to nine would have tightened the gate by accident, leaving barely two fields of headroom. These
  catch a broken prompt, not drift; `knowledge/technical_debt.md` already concluded that the
  recorded rate, not the pass/fail, is the half that detects a regression.

- **Three corpus specimens had a file extension that lied about their bytes.**
  `Spain_Passport_Specimen_P0_2022_mrz.png`, `Sweden_ID_Specimen_2022_back_mrz.png` and
  `North_Macedonia_Passport_Specimen_P0_MKD_2009_redacted_mrz.webp` are all JPEG. `image::open`
  dispatches on the extension rather than sniffing content, so all three failed with
  `Invalid PNG signature` and were silently skipped by every OCR walk in the repo — `mrz_corpus`,
  both surveys, the manifest generator and `synthpass-bench`. Renamed to `.jpg`; a magic-byte sweep
  found exactly these three of 232. Spain's line 1 is pinned in
  `crates/mrz/tests/line1_nonconformance.rs` as the specimen with no issuing state, which until now
  had only ever been a hand transcription — it reads `P<TAP` TD3 with valid checksums, confirming
  both the transcription and that `fusion::check_line1_integrity` correctly refuses it.

- **`knowledge/VIZ_TIER2_DESIGN.md`** — the written design for visual-zone work in Tier 2, and the
  correction that motivates it: the model already receives the full-page text, VIZ included, so the
  work is not "feed the VIZ in" but four separable things — clean it, label it, ask narrowly, add
  the fields only the VIZ carries — in that order. §5 records the bias that remains (every fixture
  has a checksum-valid MRZ, and Tier 2 only runs when Tier 1 fails, so the parity corpus is by
  construction the set of documents Tier 2 never sees) and the MRZ-holdout variant that closes it.
