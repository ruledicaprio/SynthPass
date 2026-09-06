# ROADMAP execution log (archived)

> **Superseded by** [`../ROADMAP.md`](../ROADMAP.md)'s `## Current state` section and
> [`../benchmarks/README.md`](../benchmarks/README.md) (the source of truth for every
> measured number). Kept for provenance per [`README.md`](README.md).
>
> This was an append-only log kept between M1 and v1.4.0, when it was ~70% of `ROADMAP.md`
> by length. It self-corrects in about a dozen places — where two notes disagree, the
> later-dated one wins. Still-open items have been promoted to `ROADMAP.md`'s
> `## Open backlog`; settled numbers live in `benchmarks/README.md`. Nothing here is
> maintained — do not cite it as current. The original planning-target quarters for the
> milestones (M1–M3 Q3–Q4 2026, M4–M5 Q1 2027, M6–M7 Q2 2027) are recorded here too, now
> that the milestone table no longer carries them.


- Milestones land as reviewable commits; **M1 → M2 → M3 → M4** is the dependency spine on the
  generation side. **M5** (Atlas) can interleave once M4's corpus exists to grade it against.
- Builds are verified through the Ubuntu Docker builder (`synthpass-builder`, see
  `docker/Dockerfile.builder` / [`CONTRIBUTING.md`](../../CONTRIBUTING.md)); native `cargo` also
  works directly on Windows/macOS/Linux without a linker workaround.
- The workspace crate rename (`mlis-*` → `synthpass-*`) has landed — see `CHANGELOG.md` for the
  executed mapping.
- **M1, M2, and M3 are done.** `mrz::format_td3`, the `synthpass-gen` factory, the degradation
  profiles, and the `synthpass generate` CLI subcommand are all shipped and tested, including
  real glyph rendering via vendored OFL fonts (`--features embedded-fonts`). See `CHANGELOG.md`
  for font provenance.
- **M4 is done** (`synthpass-bench`, PRs #27–#31): measurement library, corpus runner, CI accuracy
  gate, and [`knowledge/SYNTHPASS.md`](../SYNTHPASS.md) / [`knowledge/ADVERSARIAL.md`](../ADVERSARIAL.md) have all
  shipped. Measured Tier-1 hit rate: **55%** (100-seed clean-profile corpus), well under the
  originally-aspirational 95% — the CI gate is set to 30% as a deliberate regression floor with
  margin for cross-platform variance. See `CHANGELOG.md` for the measurement history, including a
  since-refuted hypothesis about which field drove the gap.
- **M5's `ExtractionV2` schema landed** (`crates/synthpass-core/src/v2.rs`), followed by six
  independently-safe, zero-new-dependency slices (PRs #33–#37): license-tier enforcement,
  per-field confidence scoring, a `GET /health` endpoint, configurable Tier-2 concurrency, and a
  `Retry-After` header on queue-full 503s. See `CHANGELOG.md` for details on each.
- **M5 licensing enforcement (Atlas §7) has landed.** `synthpass_license::check_feature` gates
  named features against license tiers ([`BRANDING.md`](../BRANDING.md) §5), and `max_llm_contexts`
  meters Tier-2 concurrency. The per-*endpoint* half of this gate lands with each surface as it
  ships (`metrics`, `batch`) rather than as a middleware guarding nothing yet. See `CHANGELOG.md`
  for details.
- **M5 observability (Atlas §6, and §5's `/metrics` half) has landed.** `tracing` replaces
  request-path logging, `/api/extract` opens a request-scoped span, and `synthpass-serve` exposes
  Prometheus text at `GET /metrics` (auth-gated, behind the `metrics` license feature). The "no
  PII in any log line" DoD is now an executable test
  (`crates/synthpass-pipeline/tests/pii_logging.rs`), not just a convention. See `CHANGELOG.md`
  for the full account, including a `tracing` callsite-caching trap the test had to work around.
- **M5 GBNF-constrained Tier-2 decoding (Atlas §8) has landed — with a flat accuracy result,
  recorded honestly.** `synthpass-llm` derives its GBNF grammar from `prompt::FIELDS` so prompt
  and grammar cannot drift; it constrains JSON *structure* only. A/B measurement: repair fallbacks
  **2 → 0**, field match rate **unchanged** (45.2%), wall time **+55%**. Ships on by default
  (`SYNTHPASS_LLM_GRAMMAR=0` opts out) because unrepresentable-by-construction beats
  repaired-after-the-fact. See `CHANGELOG.md` for the full measurement and a latent double-accept
  bug it uncovered.
- **M5 is complete — every Atlas DoD in §3–§8 has landed.** The bounded job queue / parallel OCR
  / batch API shipped in PR #49 (OCR geometry API and normalizers in #48, wired into
  `ExtractionV2` in #50). OCR region detection by orientation shipped last: a first design
  (position-based 0°/180° heuristic) was circular and measurably failed on its own test specimen;
  the fix compares a band's score on the page *and* its 180° flip and keeps the better, getting
  the direction right on 41 of 42 corpus images with zero false flips. See `CHANGELOG.md` for the
  full derivation, including why the position-based heuristic was wrong and what it does not yet
  close (M6's scoping note below picks that up).
- **Per-field CER measurement uncovered a structural gap: ICAO 9303 TD3 check digits cover line 2
  only.** Line 1 (document type, issuing country, names) carries no check digit, so a document can
  be checksum-proven and still return the wrong name — confirmed by a real failure mode
  (`synthpass-bench`'s per-field CER, plus a recurring OCR bug where interior `<` filler runs
  collapse and shift every field boundary left). The validation answer has since landed:
  `FieldConfidence::mrz_checksum_scope` demotes the six unverified fields to a new
  `MRZ_STRUCTURAL` (0.9) band, and `synthpass_core::fusion::check_line1_integrity` adds
  deterministic cross-checks (ICAO country-code table, issuing-country/nationality agreement, the
  filler-run signature) — measured to flag 28 of 29 (96.6%) of the cases it should. Detection
  before correction: filler-run fidelity itself remains future work. See `CHANGELOG.md` for the
  full numbers.
- **A nightly bench-data-collection workflow has shipped** (PR #38,
  `.github/workflows/bench-data-collection.yml`): runs `synthpass-bench --profile all` daily and
  appends outcomes to `dataset.jsonl` on a `bench-data` branch — data collection only, a first
  step toward future fine-tuning/dataset-characterisation work.
- **M6 scoping note — measure the page angle instead of searching for it.** Orientation handling
  currently brute-forces the angle twice: `choose_rotation` runs a full OCR *detection pass* at
  each of 0°/90°/180°/270° and keeps the best-scoring one, while `preprocess::deskew` separately
  sweeps `DESKEW_CANDIDATES_DEG` for small tilts. Both search for a quantity `ocrs` already
  reports — `detect_words` returns `RotatedRect`s carrying their own orientation, which
  `geometry.rs` discards when it converts to an axis-aligned `BBox`.

  Keeping that angle allows the dominant text angle to be *computed* in a single detection pass as
  a width-weighted **circular mean**: accumulate `w·e^(i·2θ)` over the detected words and take
  `½·arg(Σ)`. The doubling is the load-bearing part — a text line's orientation is mod π (179° and
  1° describe the same line), and naively averaging those two gives 90°, the worst possible
  answer. The resultant length `|Σ|/Σw ∈ [0,1]` falls out as a genuine confidence measure ("do the
  words agree on an orientation at all?"), which is a better gate than today's fixed
  beat-0°-by-a-margin margin. No new dependency: a complex sum is two `f64` accumulators and
  `atan2`/`sin`/`cos` are `std`. This is deliberately *not* Fourier–Mellin or a log-polar
  transform — those need a real FFT and buy nothing here.

  Three caveats, all real:
  - **It cannot resolve 0° vs 180°** (nor 90° vs 270°). Orientation mod π is intrinsic to the
    mathematics, not a limitation of the estimator. The MRZ-band tie-break shipped in M5 stays
    necessary — the angle gives you the *axis*, only content gives you the *direction*. Note that
    it is a *comparison between the two orientations*, not the band-position test this note
    originally assumed; see the M5 completion note above for why position does not work.
  - Rotating by an arbitrary angle resamples and costs sharpness, whereas right-angle rotations are
    lossless. Snap to the nearest right angle within tolerance; resample only for genuine tilt.
  - It assumes `ocrs`'s per-word angle is meaningful for near-horizontal text; verify empirically
    against the corpus before replacing the existing probe rather than adding alongside it.
- **M6 scoping note — document classes are not interchangeable.** M6's `TD1 / TD2 / MRVA / MRVB`
  line covers ID cards and residence permits (TD1, 3×30; TD2, 2×36) and visas (MRVA/MRVB).
  **The generator gap this note originally described is closed** — `synthpass-gen` has emitted all
  five formats since `6783045` (TD1/TD2) and `3637f5d` (MRV-A/MRV-B), and every one has a per-format
  hit rate recorded in the Execution notes below. **Driving licences remain a
  different mechanism, not a lower priority**: EU licences carry no MRZ at all, and US ones encode
  their data in an AAMVA PDF417 barcode. No amount of MRZ work reads one. They belong to
  `ExtractionV2.barcodes` — a declared slot with no decoder behind it — and should be scoped as a
  barcode project, not folded into the MRZ roadmap where they would quietly fail forever.
- **M7's contract layer landed, then both tiers were wired onto it.** The schema vocabulary (#74)
  — `synthpass_core::v2::{CoreField, ProviderId, EscalationKind, PromptRef, ExtractionTrace}`, plus
  `fusion::FindingKind`; the OCR evidence signals (#75) — `OcrResult`/`OcrPage` now carry
  `mrz_band_score` and `text_sanity`, computed and previously discarded; the `synthpass-die` crate
  itself (#76) — the `IntelligenceProvider`/`Recognizer`/`FieldReader` contract, the
  dependency-minimalism CI gate, `ProviderCatalog`, and the deterministic `MrzReader`; `RoutingPolicy`
  wired onto the pipeline's Tier-1 gate (#77) — `ocr_and_tier1` now consults the catalog's
  `MrzReader` instead of an inline `mrz::find_and_parse(..).filter(valid)` check; and Tier 2 (the
  LLM) wrapped as a registered `FieldReader` — `Pipeline::process_document` now resolves it via
  `catalog.find_reader(budget, |c| c.fields && !c.deterministic)` instead of calling
  `self.infer` directly. `process_document_stream` is the one exception: `FieldReader::read` has no
  delta-forwarding hook, so it still calls `InferBackend::extract_stream` directly — see
  `knowledge/technical_debt.md`'s "Streaming bypasses the provider contract".
- **M7's versioned prompts have landed.** `synthpass_llm::prompt` now exposes `PROMPT_ID`,
  `PROMPT_VERSION`, and `prompt_ref()`, computing `PromptRef.digest` from the compiled-in ChatML
  `TEMPLATE` (system text + field list + wrapper text, per-document OCR content excluded) via
  `synthpass_core::audit::sha256_hex` — no new dependency, since `synthpass-llm` already pulls in
  `sha2` transitively. `InferBackend::prompt_ref()` (default `None`, mirroring `model_id()`)
  threads it onto `NativeInferer`; `Pipeline::process_document` and `process_document_stream` both
  populate `ExtractionTrace.prompt` from `self.infer.prompt_ref()` at their Tier-2 call sites. A
  pinned-digest test in `prompt.rs` fails CI on any edit to `SYSTEM`/`FIELDS`/`TEMPLATE` that
  doesn't also bump `PROMPT_VERSION` and update the expected digest.
- **M7 is now complete — the multi-provider benchmark harness has landed.** `provider-bench`
  (new binary in `synthpass-bench`) runs every reader in `Pipeline::catalog()` — today the
  deterministic `MrzReader` and the LLM's `LlmFieldReader` — against the same OCR'd synthetic
  corpus and reports, per provider: field-match rate and mean CER (via the same `cer()`
  `synthpass-bench`'s Tier-1 tool already uses), speed (mean/p50/p95), JSON validity
  (`synthpass_llm::repair::repair_fallbacks()` delta, `None` for deterministic providers with no
  JSON step), an unsupported-assertion rate (does each answered field value appear, verbatim, in
  the OCR text the provider was given — the roadmap's own definition, deliberately not
  "hallucination"), and resident memory. Memory ships two ways: `Capability::estimated_resident_bytes`
  always (the declared figure the field's own doc comment already calls "the benchmark's memory
  column") plus, behind an off-by-default `measure-memory` Cargo feature and `--measure-memory`
  flag, a coarse `sysinfo`-based process-RSS delta — real but not per-provider-isolated, since
  every provider runs sequentially in one process.

  Reaching the real catalog required one new accessor: `LlmFieldReader` is `pub(crate)` to
  `synthpass-pipeline`, so `Pipeline::catalog()` (mirroring the `model_id()`/`prompt_ref()`
  pattern) is the only way a harness outside that crate can enumerate every registered provider —
  `ProviderCatalog::readers()`/`.profiles()` already supported full enumeration, distinct from
  `find_reader`'s first-match router. A first real run (5 documents, clean profile, the shipped
  Qwen2.5 GGUF) confirmed the harness measures real signal, not an artifact: `mrz` scored 70%
  field-match / 3 ms mean at zero JSON-repair cost, `llm` scored 56% field-match / ~24 s mean —
  both consistent with existing measurements (the M4 55% Tier-1 hit rate on OCR noise, the M5 GBNF
  parity run's ~45% field match). Ships standalone (`cargo run -p synthpass-bench --release --bin
  provider-bench`); CI wiring is a separable follow-up.
- **A local, multi-track `provider-bench` loop now exists** — still not CI wiring (that remains
  the separable follow-up above), but a way to accumulate real trend data by hand instead of
  one-off ad hoc runs, across as many benchmark "tracks" as get run. `provider-bench --format
  <passport|id_card|driving_license>` (backed by a new `SpecimenClass`/`classify_specimen` in
  `synthpass-bench`, directory-based with a ground-truth-`document_type` fallback for the mixed
  `ocr_fixtures/`/`misc/` directories) scopes a run to one `samples/` subdirectory;
  `--real-specimens` with no `--format` covers the whole real corpus. `scripts/run-bench.ps1
  -Track <name>` runs one, flattens the report to one row per `(run, provider)`, and appends to
  `results/<track>-bench/history.jsonl` on the `bench-data` branch (the same branch
  `bench-data-collection.yml` already uses, via the same isolated-`git worktree` checkout pattern,
  but pushed manually rather than on a schedule) — see `knowledge/benchmarks/README.md` for the
  row schema and the live per-track charts. `-FromReport PATH` flattens an existing report instead
  of re-running one, which both covers slow manual runs you don't want to redo and backfilling:
  the four ad hoc `qwen-real-{10,10-run-2,30,50}.json` sweeps from earlier the same day were folded
  into the `real-specimens` track this way (recorded with an honest `"unknown (backfilled
  from ...)"` `git_sha` rather than misattributed to a commit they weren't actually measured
  against). `bench-chart` (new bin, same crate, `plotters`' pure-Rust SVG backend, track-agnostic —
  one binary renders every track's history) renders each track's history into
  `knowledge/img/<track>-bench-trend.svg`, embedded in `knowledge/benchmarks/README.md`'s live-tracks
  dashboard (README's own "Accuracy, stated plainly" links out to it rather than embedding every
  chart inline).

  Passport ground truth also grew from 4 (one of which, on inspection, turned out to be malformed —
  `Cetis_Sample_Passport_Specimen_2022_inner_page_mrz.json` was missing `extraction_method`
  and had several wrong fields, e.g. `issuing_country: "CETIS d.d."`, the *printer's* name, not an
  ICAO code) to 8 good labelled specimens (fixed that one; added UAE, Slovakia Service passport, a
  second Canada specimen, China) — each hand-verified against the image rather than trusted from
  the OCR read, since only `document_number`/`date_of_birth`/`date_of_expiry`/`personal_number` are
  individually checksum-protected in a TD3 MRZ; `surname`/`given_names`/`document_type`/
  `issuing_country` are not, and the verification pass caught real Tier-1 imperfections this way (a
  dropped space in two specimens' `given_names`, and a century-pivot misread on a `11`
  year-of-birth specimen — see `scripts/check-century-pivot.sh`). Expanding beyond 8 remains future
  work; each label needs the same one-by-one visual check, not a batch script.

  **Grew from 8 to 13, 2026-08-17.** `crates/synthpass-ocr/examples/mrz_corpus.rs`'s `CORPUS` list
  already had 16 real specimens confirmed as checksum-valid Tier-1 hits with a hand-verified
  `document_number`; diffing against the 8 labelled passports left 7 not yet promoted to full
  `Extraction` ground truth. Two (`Vietnam_Passport_Specimen_2023_mrz.webp`, `Oman_Passport_Specimen_2004_mrz.jpg`)
  turned out to be **stale `CORPUS` entries** — both now reproducibly return "no MRZ found" against
  the current OCR/parser despite being listed as hits, a real regression or drift worth its own
  follow-up, not folded into this pass. The remaining 5
  (`Canada_Passport_Specimen_2013_mrz`, `Canada_Passport_Specimen_2023_2_mrz`, `Slovakia_Passport_Specimen_2005_mrz`,
  `Spain_Passport_Specimen_2013_mrz`, `Spain_Passport_Specimen_2015_colored_mrz`) were promoted after the same one-by-one
  visual check: even on genuine checksum-valid hits, the OCR's raw read of the non-checksummed
  fields is frequently badly garbled (e.g. `surname` merged with `document_type` and duplicated —
  `"PESPANOLASESPANOLA"` for a specimen actually reading `ESPANOLA<ESPANOLA`), confirming the same
  fact PR #134's line-1-integrity work established. `Slovakia_Passport_Specimen_2005_mrz` repeats the
  known century-pivot trap on its `11`-year-of-birth field (correct value `1911-11-11`, matching
  its sibling `Slovakia_Passport_Specimen_2014_mrz` fixture) — and separately, its MRZ-encoded
  `date_of_expiry` (`2015-01-04`) doesn't match the VIZ's printed `01.04.2015`
  (day/month swapped): a genuine inconsistency baked into that specimen template, not an OCR
  error. Ground truth here follows the MRZ zone (what `extraction_method: "mrz-deterministic"`
  actually reads), consistent with every other fixture, and the mismatch is recorded here rather
  than silently resolved one way. Expanding beyond 13 remains future work, same caveat as above.
- **Post-M7 real-world validation surfaced a Tier-1/Tier-2 fusion gap.** A private Turkish
  passport specimen (`samples/ocr_fixtures/Turkiye_Passport_private.jpeg`) escalated to Tier-2
  on a single misread MRZ character (`document_number`'s leading glyph), which threw away
  three other individually checksum-verified fields (`date_of_birth`, `date_of_expiry`,
  `personal_number`) that `MrzData::valid()`'s all-or-nothing gate does not distinguish from
  genuinely-unverifiable ones. The LLM then re-derived those fields from noisy OCR text alone
  and got `sex`, `date_of_birth`, `surname`, and `given_names` wrong — confirmed by hand
  against the ICAO check-digit arithmetic. Tracked as issues
  [#99](https://github.com/ruledicaprio/SynthPass/issues/99)–[#103](https://github.com/ruledicaprio/SynthPass/issues/103):
  single-character MRZ substitution repair, per-field (not document-level) checksum promotion
  into Tier-2 output, an MRZ-structural-contradiction `Finding` for line-1 fields, feeding the
  checksum-partial read into the Tier-2 prompt as a hint, and a follow-up check on visual-zone
  OCR quality for this locale. `apply_deterministic_mrz`
  (`crates/synthpass-pipeline/src/lib.rs:1011`) already states the operating principle — "the
  deterministic read is always the more trustworthy source, checksum-partial or not" — these
  issues finish applying it.
- **Issue #103 scoping note — visual-zone OCR noise is corpus-wide, not specimen-specific; a
  `preprocess`-style treatment for the non-MRZ zone is worth scoping (no code change proposed
  yet).** #103 reported OCR noise on a private, untracked Türkiye candidate specimen
  (`work/Turkiye_Passport_Private_snipped.jpg`, never committed). Reproduction: `check_sample`
  still returns a checksum-valid MRZ HIT for it under the current `ocrs` engine, so the MRZ zone
  itself is not the problem — the noise is confined to the visual zone (photo/header/data-field
  text above the MRZ). A new measurement harness, `crates/synthpass-ocr/examples/visual_zone_survey.rs`
  (modelled on `examples/integrity_survey.rs`), classifies each geometry-detected line as MRZ or
  non-MRZ (via `geometry::mrz_line_score`) and reports, per non-MRZ line, whether it's noise (<= 2
  non-whitespace characters, or >= 50% non-alphanumeric) — no ground truth required, so it ran over
  the whole `samples/` corpus (135 specimens with at least one non-MRZ line; JSONL at
  `knowledge/visual-zone-survey.jsonl`). Corpus-wide noise-line fraction: **median 0.243, IQR
  [0.111, 0.455]**. The Türkiye specimen scored **0.652** (46 non-MRZ lines, 30 noise) — above the
  IQR, but at roughly the 89th percentile of the full corpus (15 of 135 tracked specimens score as
  high or higher, across unrelated countries/formats) and *not* above the high end of a
  typographically-close proxy set drawn from already-tracked specimens (Azerbaijan, Albania,
  Romania, Croatia, Bosnia and Herzegovina, Kosovo, North Macedonia, Poland, Viet Nam) — Viet Nam's
  specimen scores higher (0.778). Conclusion: this is a **systematic** corpus property, not
  something unique to the Türkiye candidate. The natural follow-up — not implemented here, per
  #103's stated scope — is to extend the visual zone with the same deterministic
  upscale/contrast/threshold/deskew treatment `preprocess.rs` already applies to the MRZ band (see
  its module docs and e.g. `preprocess::mrz_variants`), rather than treating the MRZ-band retry
  passes as the only place OCR quality gets a second chance.
- **Re-measured 2026-09-02 on the regenerated corpus: the #103 conclusion holds, and holds harder.**
  `visual_zone_survey.rs` was re-run after the corpus rename and manifest work, over **229**
  specimens with at least one non-MRZ line (up from 135). Corpus-wide noise-line fraction:
  **median 0.237, IQR [0.146, 0.386]** — against the original median 0.243, IQR [0.111, 0.455].
  The median moved by 0.006 on a corpus 70% larger, which is the strongest form the original
  finding could take: visual-zone noise is a systematic property of this corpus, not an artefact
  of which specimens happened to be in it. The survey now covers 14 Türkiye specimens rather than
  one, and the highest (`Turkiye_Passport_Specimen_P0_TUR_2011_mrz.jpg`, 0.534) sits below the
  0.652 originally reported for the private candidate. The proposed follow-up is unchanged:
  extend `preprocess.rs`'s deterministic treatment to the non-MRZ zone. Data:
  `knowledge/visual-zone-survey.jsonl`.
- **2026-09-02: expanding the negative controls from 4 to 42 found a corpus labelling error —
  first reported here, wrongly, as a Tier-1 false positive.** `mrz_corpus.rs`'s negative set is now
  derived from `samples/corpus.jsonl` (every row recording `mrz.present == false`). Its first run
  flagged `Monaco_ID_Specimen_XXXX_front_no_mrz.png` returning a checksum-valid Td1 record. That
  was recorded as Tier 1 hallucinating an MRZ out of visual-zone text. **It was not.** The read was
  correct: `IC`/`MCO` with every check digit passing is exactly what a genuine Monaco identity-card
  MRZ looks like, and coherent values plus valid checksums are evidence *against* a hallucination,
  not for one. The file was a **back** side filed under a front-side name, and has since been
  renamed by its author.
  The useful lesson is about the harness rather than the parser: a negative control is only as good
  as the `no_mrz` tag it is derived from, so a `FALSE-POSITIVE` line has two explanations and the
  mislabel is by far the likelier. `corpus_manifest.rs` now separates the two by strength — a
  *checksum-valid* read off a `no_mrz` page is reported as `LABEL LIKELY WRONG`, while a read that
  parses structurally but fails its check digits is left unreported, because 18 specimens do that
  routinely when visual-zone text lands in an MRZ-shaped line.
  `crates/synthpass-bench/tests/corpus_manifest.rs` now fails if any `no_mrz` row records a
  checksum-valid observation.
- **2026-09-02: Tier-2 accuracy has a baseline. The parity harness went from 6 documents to 72, and
  the first thing it measured was that its own fixtures were wrong.**
  `crates/synthpass-bench/examples/ground_truth_candidates.rs` derives a candidate `Extraction` from
  every corpus specimen whose MRZ reads with valid check digits *and* survives
  `fusion::check_line1_integrity` plus the two date judgements that don't depend on today. Fixtures
  split by review status: `samples/ocr_fixtures/` (hand-verified, all nine prompt fields scored) and
  `samples/ocr_fixtures/derived/` (generated, the check-digited fields only). The split is not
  bureaucracy — ICAO check-digits exactly four fields, so a generated fixture's *name* is one OCR
  pass's reading of characters nothing arbitrates. Scoring a model against those inverts the
  measurement: a model reading the printed name correctly is marked wrong for disagreeing with an
  OCR error, so visual-zone work would register as a regression precisely when it worked.

  **Measured baseline** (`qwen2.5-1.5b-instruct-q4_k_m`, greedy, ~31 min): reviewed **43/162
  (26.5%)** over 18 documents; derived **47/162 (29.0%)** over 54; legacy seven-field rate over the
  reviewed set **38/126 (30.2%)**; repair fallbacks **0**. Per field, reviewed:
  `document_number` 50%, `given_names` 39%, `date_of_expiry` 28%, `sex` 28%, `surname` 28%,
  `issuing_country` 22%, `nationality` 22%, `date_of_birth` 17%, `document_type` 6%. The two sets
  agree closely on the only field both score (`document_number`, 50% vs 54%), which is the check
  that the generated half is not systematically easier than the hand-verified half.

  `document_type` (6%) and `issuing_country` (22%) are **format disagreements, not wrong answers** —
  the model returns `PASSPORT` where the MRZ says `P`, and `CROATIA` where it says `HRV`. The
  harness deliberately does not normalise them away, since that would raise the number without
  changing what the pipeline receives. It is the clearest evidence for the narrow-ask work in
  `knowledge/VIZ_TIER2_DESIGN.md` §2.3: the prompt never states which form it wants.
- **2026-09-02: the parity fixtures had been feeding the model an input shape production no longer
  produces, and it cost half the measured accuracy.** The six `.md` files the harness used came from
  `docling`, the Python engine retired in v0.7.5 — HTML-escaped (`P&lt;HRV…`), with `##` headings
  and `<!-- image -->` placeholders. The shipped pipeline feeds Tier 2 `NativeOcr::recognize`'s
  plain text. A same-binary A/B over the *same six documents*, changing only where the `.md` came
  from: docling **9/54 (16.7%)**, `NativeOcr` **18/54 (33.3%)** — and on the legacy seven-field
  basis, 9/42 vs 15/42. `document_number` alone went 1/6 → 4/6. The docling arm was re-run and
  reproduced byte-identically (zero differing per-field lines), so the delta is real rather than
  sampling noise. Every `.md` is now generated from the engine that ships.

  **Unreconciled:** that docling run scores 9/42, while `knowledge/technical_debt.md` records 16/42
  and `knowledge/prompts/README.md` records 19/42 for what should be the same six fixtures.
  Reproducibility within a session is exact, so the divergence is outside it — engine version, model
  file, or machine. Recorded rather than explained; the A/B above is internally valid regardless,
  because both arms ran on one machine minutes apart.
- **2026-09-02: three corpus specimens had a file extension that lied about their bytes, and were
  invisible to every OCR walk in the repo.** `Spain_Passport_Specimen_P0_2022_mrz.png`,
  `Sweden_ID_Specimen_2022_back_mrz.png` and
  `North_Macedonia_Passport_Specimen_P0_MKD_2009_redacted_mrz.webp` are all JPEG (`ff d8 ff`).
  `image::open` dispatches on the extension rather than sniffing content, so all three failed with
  `Invalid PNG signature` and were silently skipped by `mrz_corpus`, both surveys, the manifest
  generator and `synthpass-bench`. Confirmed by the narrowest possible A/B: the same bytes under a
  `.jpg` name return `MRZ HIT doc_number=KV2424725 format=Td3`. Renamed to `.jpg`; a corpus-wide
  magic-byte sweep found exactly these three of 232.

  Spain matters beyond the count. Its line 1 is pinned in `crates/mrz/tests/line1_nonconformance.rs`
  as the specimen with **no issuing state** — and that had only ever been a hand transcription,
  because OCR could not open the file. It now reads `P<TAP` TD3 with valid checksums: live
  confirmation that positions 3-5 hold the start of the surname, and that
  `fusion::check_line1_integrity` correctly refuses it as `unrecognized_issuing_country`. It is also
  correctly *rejected* as unreviewed ground truth, since nothing may infer `ESP` from line 2.
- **Issue #102 — feeding the checksum-partial MRZ read into the Tier-2 prompt as a hint measurably
  helps, at no measurable latency cost once measured on an uncontended machine.**
  `synthpass_llm::prompt::build_prompt` now accepts an optional `hint` (PROMPT_VERSION 1 → 2, new
  `{hint}` template slot, digest re-pinned), threaded end to end from
  `synthpass_die::DocumentContext::mrz_hint` through `InferBackend::extract`/`extract_stream` to
  the prompt. The hint is built from the individual `mrz::Checks` bits (not `DocumentContext.prior`,
  which only populates when the *whole* MRZ record validates) — `document_number`, `date_of_birth`,
  `date_of_expiry`, `personal_number` when their own check digit passes, plus `nationality`/`sex`
  when the line-1 composite check passes — so a checksum-partial read still contributes whatever
  it individually proved. Gated behind `SYNTHPASS_LLM_MRZ_HINT` (default **off**).

  First A/B (`provider-bench --count 10 --seed 42 --profile worn`, flag off vs on) ran while other
  `cargo`/`provider-bench` processes were active on the same machine and reported a **+39%** mean
  latency cost (33.3s → 46.3s) alongside the accuracy gains — caught as suspect (thanks to a sharp
  eye on the number) precisely because CPU-bound llama.cpp inference is exactly the kind of
  workload contention skews. Re-run back-to-back with nothing else running (`tasklist` checked
  clean before each half): field-match rate **65% → 69%**, mean CER **0.722 → 0.436**,
  unsupported-assertion rate **5.7% → 1.3%** — unchanged from the contended run, as expected since
  none of those depend on wall-clock — and mean latency **29.12s → 29.00s**, i.e. no cost at all.
  All three of the plan's ship criteria (+≥3pt field-match, CER not worse, latency +<10%) are now
  met at `n=10`. **Ship decision: land the plumbing now, keep the flag off pending a larger run**
  (`n=10` is still a small sample for a default-behavior flip) — `provider-bench` itself now
  attaches an MRZ prior the same way the pipeline does, so re-measuring at `--count 20+` needs no
  further code changes, only a clean (uncontended) machine to run it on.
- **M6 — the generator gap is closed for TD1/TD2, and the first per-format measurement is bad for
  TD1.** `synthpass-gen` now emits all three formats onto their own ICAO card geometry (TD1 →
  ID-1, 822×518; TD2 → ID-2, 1008×710; TD3 → ID-3, 1200×840, unchanged and byte-identical), and
  `--document-type td1|td2|td3` reaches it from both `synthpass generate` and both bench binaries.
  The separation that made this necessary: `DocumentType::document_code()` returns `"I"` for TD1
  *and* TD2 — correct per ICAO — so the document code can never distinguish them. `mrz::Format` is
  the discriminator, carried on `Labels` and emitted as the sidecar's `mrz_format` key.

  First measurement (30 seeds, `--profile clean`, seed 42), Tier-1 hit rate:

  | Format | Hit rate | |
  |---|---|---|
  | TD2 | **80.0%** (24/30) | |
  | TD3 | **76.7%** (23/30) | consistent with the 55% 100-seed figure above; n=30 and a different seed window |
  | TD1 | **26.7%** (8/30) | **below the 30% CI floor** |

  TD1 is roughly three times worse than the other two and would hard-fail the existing global gate
  if it were wired in — which is why `--min-hit-rate` deliberately stays a single global number
  and **no per-format floor has been added**. A floor over a corpus one day old is an invented
  threshold, not an earned one; the numbers get reported first.

  The likely cause is the TD1 *render*, not the parser: TD1's `surname` CER came back at **128%**,
  i.e. above 100%, which is only possible through insertions — the OCR is emitting characters that
  are not in the ground truth at all, rather than misreading the ones that are. The ID-1 canvas is
  the smallest of the three (822×518 against TD3's 1200×840), so absolute glyph resolution in the
  VIZ is the first thing to check. Not diagnosed further here.
- **M6 follow-up — the TD1 name-line loss, root-caused and fixed.** A `synthpass-bench --dump-ocr`
  probe (new flag, `--document-type td1 --count 3 --seed 42`) printed the raw OCR text underneath
  the parse for the first time, confirming the mechanism directly rather than by inference: TD1's
  third MRZ line — the only place `surname`/`given_names` live — came back as the watermark
  (`"STNHENLSPELJMENSTNNEL..."`) or a duplicate of line 1, exactly as this note's earlier per-field
  CER breakdown implied. Two compounding causes, both real:

  1. **The mandatory watermark sat 8px above MRZ line 1** (`layout.rs::td1_layout()`'s watermark
     rect was `y=282, height=26`, against a 28px-tall glyph and an MRZ band starting at `y=318`) —
     TD2 gets ~145px of separation, TD3 ~190px. Across OCR's own internal multi-pass retry loop,
     some passes correctly detected all three MRZ rows as distinct lines; others did not detect
     line 3 as its own region at all, leaving the watermark or a repeated line 1 in that slot.
     Fixed by giving the MRZ band a smaller, dedicated bottom margin (16px vs. the 40px top/VIZ
     margin) and growing the watermark rect to 32px, netting ~21px of real separation on each side
     without touching VIZ row spacing (so VIZ OCR quality is unaffected). TD3's `PageLayout` stays
     byte-identical.
  2. **TD1 was the only format whose three-line scan required strict adjacency** — TD2, TD3,
     MRV-A and MRV-B all tolerate a gap between candidate lines and all four accept the whole zone
     merged onto one physical line; TD1 had neither. Fixed: TD1 now searches the next 3 detected
     lines for line 2 and line 3 (bounded, not a full-page scan), and gained the merged-line
     branch the other four formats already have.

  A **third, previously undocumented cause** surfaced only once the dump made line 1 itself
  legible: OCR was dropping TD1's position-1 filler outright (`I<BRA...` read as `IBRA...`),
  shifting every field from `issuing_country` onward one position left. **Unlike TD3, whose check
  digits live entirely on line 2, TD1's document-number check digit is on line 1 itself** — so
  this single dropped character was breaking TD1's own checksum, not merely corrupting
  `issuing_country` cosmetically the way the equivalent misread does on TD3. `variants`'s
  length-fitting could not recover it: a line arriving one character short is padded by extending
  its *longest* filler run (the trailing one), never by reinserting a filler at position 1. Fixed
  with a new candidate reading, `repair_td1_line1_unshifted`, tried alongside the unrepaired one —
  checksums decide which, if either, is real.

  Measured before/after (30 documents, `--seed 42 --profile clean`, `./scripts/run-bench.ps1
  -Track td1 -Count 30 -Seed 42`):

  | | Hit rate | Mean `surname` CER |
  |---|---|---|
  | Before (`a2a7f64`) | 26.7% (8/30) | 128.79% |
  | After (`1cab58c`) | **56.7%** (17/30) | *(not re-measured; the fix targets line 1's checksum, not line 3's content)* |

  More than double, and above the repo's 30% CI floor for the first time. TD2 (80.0%, 24/30) and
  TD3 (76.7%, 23/30) were re-confirmed unaffected, as expected — neither format's layout or parser
  path was touched.

  **This does not close the check-digit blind spot, and should not be read as if it did.** Of the
  17 post-fix hits, 15 (88.2%) still carry a line-1 integrity finding — barely moved from the
  pre-fix 87.5% (7/8). **TD1's check digits do not cover line 3 at all** (document number, date of
  birth, expiry, and the composite all read from lines 1–2 only), so a checksum-valid TD1 record
  can carry a completely fabricated name, and nothing in this fix — or in TD1's format as ICAO
  defines it — can change that. A TD1 hit rate is a statement about lines 1–2 reading correctly,
  never a statement about the name. Five regressions in `crates/mrz/tests/td1_line_gap.rs` pin the
  adjacency/merged-line fix; deliberately, none of them try to prove a parser can pick the "right"
  line 3 out of two equally check-digit-silent candidates, because it can't.
- **Check-digit blind spots are compound far more often than they are per-character, and the
  counter we ship only sees the per-character kind.** Over the nightly `dataset.jsonl` corpus
  (2910 rows, TD3): 1804 hits, 987 `checksum invalid`, 53 `no MRZ found`, and **66
  `document number mismatch`** — the last being the only dangerous class, since those *passed*
  the check digits and were still wrong. With no ground truth in production they surface as
  confident Tier-1 hits at `document_number` confidence 1.0.

  Classifying all 66 by whether the differing characters are congruent mod 10: **19** are the
  single-character collisions [`blindspot.rs`](../../crates/mrz/src/blindspot.rs) documents and
  predicts (`G↔Q`, `U↔0`, `L↔1`, `I↔S`), and **46** are *compound cancellations* it does not.
  Example: `EE2GSWBOW` read as `FF2GSWBOW`. `E`=14 and `F`=15 are **not** congruent mod 10, so
  `blindspot('E','F')` correctly reports `Caught { delta_mod10: 1 }` — each substitution is
  individually detectable. But the two sit at weights **7 and 3**, so the shifts sum to
  `1×7 + 1×3 = 10 ≡ 0` and the check digit is unchanged.

  `blindspot.rs`'s law is exact as stated — it is stated for a *single* substitution. The gap is
  that `blind_positions` (`crates/synthpass-die/src/mrz_reader.rs`) counts only per-character
  collisions, so it reports **zero** blind positions for a read that is entirely invisible to the
  arithmetic. The real blind-spot space is *any multiset of substitutions whose 7-3-1 weighted
  delta sums to 0 mod 10*; the per-character rule is its special case.

  This is not a freak coincidence, which is why 46 > 19: **OCR error is systematic**. A document
  that misreads `O` as `0` does it to every `O`, so two of them landing on adjacent weight-7 and
  weight-3 positions is the expected behaviour of a consistent misread, not bad luck. Observed
  repeatedly in the corpus — `OO94XETJ5`→`0094XETJ5` (positions 1,2) and `OH39SQNOY`→`0H39SQN0Y`
  (positions 1,8) are both exactly this. No fix proposed here; recorded so the counter's blind
  spot is a known quantity rather than a surprise.
- **The M5 GBNF parity baseline re-measured: 16/42 (38.1%) at commit `493010a` (2026-08-16),
  not the 45.2% recorded above.** Reproduces identically whether or not the grammar is enabled
  (`repair fallbacks: 0`), so this is not a regression from the GBNF work itself, and the
  `llama-cpp-2` 0.1.151 → 0.1.154 bump was already ruled out separately (`technical_debt.md`
  recorded 16/42 on both versions). The likeliest cause is that the fixture/corpus set moved out
  from under the recorded 45.2% figure when `samples/` was reorganised and gained TD1/TD2
  rendering — a guess, not confirmed, same as before. Recording this rerun with its commit and
  date, per this note's own standing advice: future rate changes should be compared against
  *this* number, not the original 45.2%, and any future rerun should likewise record its own
  commit alongside the rate rather than being trusted indefinitely.
- **M6 — first-ever MRVA/MRVB per-format measurement, and a real cross-format name-parsing bug
  found and fixed along the way.** `synthpass-gen` has emitted MRV-A/MRV-B since `3637f5d`, and
  `MrzReader` gained provider-level specimen tests for both (alongside TD1/TD2) in `f36df5c` — but
  until now no bench run had ever recorded a hit-rate number for either format, unlike
  TD1/TD2/TD3 above.

  First measurement (30 seeds, `--profile clean`, seed 42), Tier-1 hit rate, alongside a same-day
  TD3 re-run for a same-commit baseline:

  | Format | Hit rate | |
  |---|---|---|
  | MRV-B | **93.3%** (28/30) | 2 `checksum_failed` |
  | MRV-A | **86.7%** (26/30) | 2 `checksum_failed`, 2 `document_number_mismatch` |
  | TD3 (`f36df5c`, re-run for comparison) | 76.7% (23/30) | matches the 76.7% recorded above |

  Hit rate lands clean on the first try for both — TD2/TD3's pattern, not TD1's — consistent with
  `layout.rs`: MRV-A's watermark-to-MRZ-line gap is 191px and MRV-B's is 145px (MRV-B's canvas is
  byte-for-byte TD2's own geometry), nowhere near TD1's former 8px squeeze, and both formats'
  two-line scan in `crates/mrz/src/parser.rs` already carried the gap-tolerant/merged-line logic
  TD1 was missing. Hit rate was never the concern here.

  **Per-field CER was**, and this *was* a fresh finding, not an already-understood one: `given_names`
  63.5%/76.7% and `surname` 47.0%/64.7% (MRV-A/MRV-B) — high enough to root-cause rather than wave
  off as "line 1 is never check-digit-covered, so noise is expected." `synthpass-bench --dump-ocr
  --document-type mrva --count 5 --seed 42` showed the actual mechanism directly: OCR routinely
  drops **one** of the two `<` filler characters in the `<<` primary/secondary name separator —
  `ESKANDARI<<MAREN` read as `ESKANDARIMAREN` (both dropped) or `KIRSCHNER<LUCA` (one dropped) — and
  `clean_name` (`crates/mrz/src/parser.rs`) required a literal `"<<"` to split at all. No match, no
  split: the *entire* field became `surname`, and `given_names` silently went to `""`. A **distinct**
  failure mode from the crate's existing `fix_name_separator` repair, which only catches `<`
  *misread as* `K` (a `KK` pair) — dropped-outright and misread-as-K produce different bytes, and
  the existing repair only watches for one of them. This is shared code, not MRV-specific:
  `clean_name` backs TD1/TD2/TD3/MRV-A/MRV-B alike, and the same-day TD3 baseline had the identical
  mechanism (confirmed via the same `--dump-ocr` comparison against `ESKANDARI`/`MAREN` at the same
  seed) — it had simply never been chased to a cause before now.

  **Fix**: `clean_name` falls back to a single `<` when no `<<` survives (new fallback branch,
  `crates/mrz/src/parser.rs`) — recovers the "one filler dropped" case, which dominates the observed
  failures; the rarer "both dropped" case has no separator evidence left at all and stays
  unrecoverable by construction (pinned, not silently guessed at — see
  `a_fully_collapsed_separator_is_not_recoverable_and_must_not_be_silently_guessed` below). Safe for
  this crate's own synthetic corpus specifically because `synthpass-gen`'s name pools
  (`SURNAMES`/`GIVEN_NAMES_M`/`GIVEN_NAMES_F`, `crates/synthpass-gen/src/data.rs`) are all
  single-token — no legitimate internal `<` ever competes with the fallback. Against a real compound
  name (`DE<LA<CRUZ<<MARIA` losing its `<<`) the fallback is a best-effort guess, not a proof — which
  is exactly why `FieldConfidence` never rates a name field above `MRZ_STRUCTURAL` (0.9) regardless
  of which branch produced it, fix or no fix. Four regressions in
  `crates/mrz/tests/name_separator_collapse.rs` pin this: MRV-A, MRV-B, and TD3 each recover the
  split after a collapsed separator, and a fourth test pins the known unrecoverable case so a future
  change doesn't start guessing with zero evidence.

  Measured before/after (30 documents, `--seed 42 --profile clean`, before `f36df5c` / after this
  fix), mean CER — hit rate is unchanged in every row, as expected, since the name field was never
  what gated a Tier-1 hit:

  | Format | `given_names` CER | `surname` CER | line-1 integrity findings (of hits) |
  |---|---|---|---|
  | MRV-A | 63.5% → **30.7%** | 47.0% → **15.5%** | 46.2% (12/26) → **23.1%** (6/26) |
  | MRV-B | 76.7% → **24.3%** | 64.7% → **18.7%** | 82.1% (23/28) → **50.0%** (14/28) |
  | TD3 | 63.9% → **25.1%** | 57.5% → **29.4%** | 87.0% (20/23) → **82.6%** (19/23) |

  More than halved on every format for both fields, without moving hit rate or touching any other
  field's check digit. Nor does this fix the fully-collapsed-separator case (both fillers dropped,
  no `<` left at all) — that remains the same already-documented line-1 blind spot TD1's note
  describes, just with a smaller share of the failures landing in it now.

  **`document_type`/`issuing_country` CER, corrected: this was never a VIZ-zone question — it was
  the same TD1 mechanism, ungeneralized.** An earlier version of this note called the elevated
  CER here (6.7%–76.7% across formats) "a VIZ-zone single/short-field OCR legibility question...
  not investigated" — wrong, and worth correcting in place rather than leaving standing.
  `synthpass-bench`'s `document_type`/`issuing_country` accessors (`crates/synthpass-bench/src/
  lib.rs`) read `mrz::MrzData` fields sourced *entirely* from MRZ line 1 on both the truth and got
  sides — the VIZ text-box rects in `layout.rs` are never consulted. The real mechanism: TD1
  already has a fix (`repair_td1_line1_unshifted`) for OCR dropping line 1's position-1 filler
  outright (`P<BRA...` read as `PBRA...`, shifting `issuing_country` and everything after it one
  position left) — but TD1 needed the fix because TD1's own document-number check digit lives on
  line 1, making the shift break checksums. **TD3/TD2/MRV-A/MRV-B have no check digit on line 1 at
  all**, so the identical corruption was silent — never a checksum failure, just a wrong
  `document_type`/`issuing_country` sitting behind a passing Tier-1 hit — and the fix was never
  generalized to them.

  A straight port of TD1's fix doesn't work here, and shipping the first attempt would have been a
  mistake: TD1 tries the unshifted reading as a *second candidate*, with its own line-1 checksum
  deciding which one (if either) is real. TD3/TD2/MRV-A/MRV-B have no such checksum to arbitrate
  with, so `consider()` (`crates/mrz/src/parser.rs`) accepts the *first* checksum-passing
  line-1/line-2 combination regardless of what line 1 says. A first version applied the unshift
  *unconditionally inside* `repair_td3_line1`/`repair_mrv_line1` (safe in principle for these three
  formats specifically, since their document code is unambiguously single-letter-plus-filler, no
  genuine two-letter variant modeled anywhere in this crate — unlike TD1/TD2's `"ID"`/`"AC"`-style
  codes). Measured against a real 30-seed TD3 corpus, that version fixed the CER to 0% but
  **dropped Tier-1 hit rate from 76.7% to 63.3%** — confirmed as a real, reproducible regression
  (not the run-to-run OCR-timing contention artifact already documented above) via a same-binary
  A/B env-var toggle. The mechanism: `variants()` (`crates/mrz/src/checksum.rs`) builds
  `[repaired, fitted, last_resort]` per candidate and deduplicates by exact string equality;
  mutating what `repaired` is (rather than adding it via a second, independent `variants()` call)
  also changes what `last_resort = aggressive_defiller(&repaired)` evaluates to, silently dropping
  a previously-tried, load-bearing candidate in some cases instead of only adding a new one.

  **Shipped fix**: the unshift is a genuine second candidate — `repair_td3_line1_unshifted`/
  `repair_mrv_a_line1_unshifted`/`repair_mrv_b_line1_unshifted` — tried via its own `variants()`
  call, *before* the ordinary repair at every TD3/MRV-A/MRV-B call site in `find_and_parse_with`
  (ordering matters: since nothing arbitrates, whichever candidate is tried first wins whenever
  both parse). This keeps the search space a strict superset of the unmodified pipeline's, so hit
  rate cannot regress by construction. TD2 is explicitly excluded (see `repair_td2_line1`'s own
  doc comment) — it shares TD1's genuine two-letter document-code family with no checksum
  backstop to disambiguate a wrong unshift, a harder case deferred rather than silently skipped.
  Four regressions in `crates/mrz/tests/line1_prefix_shift.rs` pin the recovery for TD3/MRV-A/
  MRV-B and confirm a genuine TD2 two-letter code (`"IP"`) survives untouched.

  Measured before/after (30 documents, `--seed 42 --profile clean`), same day:

  | Format | Hit rate | `document_type` CER | `issuing_country` CER |
  |---|---|---|---|
  | TD3 | 76.7% (unchanged) | 76.67% → **23.33%** | 52.22% → **16.67%** |
  | MRV-A | 86.7% (unchanged) | 6.67% → **0.00%** | 4.44% → **0.00%** |
  | MRV-B | 93.3% (unchanged) | 40.00% → **3.33%** | 26.67% → **2.22%** |

  Zero hit-rate cost on any format, MRV-A fully closed, TD3/MRV-B substantially improved but not
  closed — some corrupted readings still win over the unshifted candidate when both parse
  structurally, since nothing on line 1 discriminates between them once neither fails a checksum.
  A partial, hit-rate-safe improvement, not a claim of completeness.

  **The missing other half — an inserted character, not just a dropped one — closed 2026-08-17.**
  `crates/synthpass-ocr/examples/integrity_survey.rs --dir passports --mrz-only` (real specimens,
  not synthetic) found 34 of 57 `UnrecognizedIssuingCountry` findings across 24 distinct countries
  sharing one exact shape: a single spurious character *inserted* right after line 1's position-1
  filler, shifting `issuing_country` one position *right* — the mirror image of the dropped-filler
  case above, e.g. Czechia (`CZE`) read as `SCZ`, Iceland (`ISL`) as `AIS`, the UK (`GBR`) as `SGB`,
  the USA (`USA`) as `SUS`. `shift_line1_right_at_country` (`crates/mrz/src/parser.rs`) fixes it,
  using [`country_name`] as the discriminator rather than a shape-only guard — unlike the drop case,
  position 1 stays `<` either way, so shape alone can't tell a genuine insertion from an undamaged
  line. The two repairs can't be independent `variants()` candidates in either order (each is a
  no-op passthrough of the *other's* trigger case, and since line 1 carries no check digit,
  whichever is tried first wins even when it did nothing useful) — `shift_or_unshift_line1` composes
  them into one candidate instead. Implementing this also surfaced a real, pre-existing gap in the
  original fix: TD3's document code genuinely has two-letter forms (`PS` service, `PP` — Canada's
  and, effective 15 December 2025, Cyprus's ordinary-passport code), which the shape-only unshift
  guard couldn't tell apart from a real drop; the unshift fallback is now gated on `country_name`
  too, closing that gap alongside this one. Five new regressions in
  `crates/mrz/tests/line1_right_shift.rs` (insertion recovery for TD3/MRV-A/MRV-B, an undamaged-zone
  guard, and a genuine-two-letter-code guard using Cyprus's real `PP`).

  Real-specimen verification (not just synthetic): re-running the same `--dir passports --mrz-only`
  scan after the fix dropped `UnrecognizedIssuingCountry` fires from 57 to 20, with zero new false
  positives — every one of the 34 matching specimens now resolves correctly, and the 23 that didn't
  fit the pattern (different corruption shapes, e.g. `BIH`→`SIA`, genuinely garbled OCR) are
  unaffected either way. Synthetic-corpus check (30 seeds, `--seed 42 --profile clean`, same-binary
  A/B against pre-fix `main`) reproduced the exact pre-fix numbers byte-for-byte on all three
  formats (TD3 76.7%/23.33%/16.67%, MRV-A 86.7%/0%/0%, MRV-B 93.3%/3.33%/2.22%) — this specific
  seed's synthetic OCR noise model doesn't happen to produce the inserted-character artifact, the
  same conclusion reached earlier this session for the `fusion.rs` line-1-integrity heuristics.
  Hit rate unchanged on every format either way, confirming zero regression risk from the fix
  itself, independent of whether a given corpus exercises it.

  **MRV-A/MRV-B verify a narrower surface than TD1/TD2/TD3, and the hit-rate table above should be
  read accordingly.** ICAO 9303 Part 7 defines no personal-number check digit and no composite
  check digit for visas — `crates/mrz/src/parser.rs`'s `parse_mrv_a_with`/`parse_mrv_b_with`
  hardcode `checks.personal_number = true` and `checks.composite = true` (vacuous, not measured)
  — so only 3 of the 5 `Field` check digits gate an MRV hit (`document_number`, `date_of_birth`,
  `date_of_expiry`), against TD1/TD2's 4 and TD3's 5. MRV-A's/MRV-B's higher hit rates are not a
  claim that these formats are read more reliably than TD3 in any absolute sense — they are a
  claim over a smaller set of checked fields.
