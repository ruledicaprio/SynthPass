# ADR-0024 — Keep every benchmark run's per-document evidence in a local archive

**Status:** Accepted (2026-09-27: the owner chose text-free private records, Decision 7, and
accepted both defaults, on by default and stored in the git common directory)
**Date:** 2026-09-27

## Context

A benchmark run computes far more per document than it keeps. The committed outcome ledger
(`knowledge/benchmarks/real-specimen-outcomes.jsonl`) keeps outcomes and, by design, no read text.
`samples/corpus.jsonl` keeps a dated read after repair. Raw OCR survives only when it was asked for
in advance: `provider-bench --dump-ocr` writes two miss kinds (hits only with `--dump-ocr-hits`)
next to wherever `--out` points, and the next dump in that directory replaces it.

The cost is questions that runs already paid for cannot answer.
[ADR-0021](ADR-0021-fixed-grid-mrz-strips.md) needs what OCR reads at position 1 of TD3 line 1;
[#420](https://github.com/ruledicaprio/SynthPass/issues/420) records that only a minority of the
corpus had raw OCR on disk, and the rest needed a fresh full OCR pass — the stage that dominates the
benchmark's cost ([ADR-0010](ADR-0010-benchmark-cost-split-by-role.md)).

Three constraints shape the fix. Private specimens are real identity documents: never in CI
(ADR-0010, Decision 4), never on `bench-data`, and `provider-bench` already refuses an OCR dump
that includes them. Nothing published may carry zone text or personal data; disclosure is by asset
ID, counts, cell positions and character classes. And the archive's readers include AI-assisted
analysis sessions: text a session reads leaves the machine, so "local" alone does not make private
text safe to keep.

## Decision

1. **Every `provider-bench` and `synthpass-bench` run writes one record per document per provider
   to a local archive, by default.** `--no-archive` or `SYNTHPASS_BENCH_ARCHIVE=off` disables it.
   The archive is a side output: no run reads it, and a write failure is a warning that never
   changes an exit code.
2. **The archive lives outside every working tree** — `SYNTHPASS_BENCH_ARCHIVE` if set, else
   `synthpass-bench-archive/` in the repository's common git directory, shared by all worktrees of
   a clone and a path git refuses to track. A root inside the working tree is refused unless git
   ignores it.
3. **One JSON Lines file per run**: a run header, then one line per document. Written as
   `.partial` and renamed on completion; flushed per provider; a closed file is never rewritten.
4. **The run header** records source (`ci`/`local`), machine label, binary and its SHA-256, git
   commit and dirty flag, arguments, corpus scope, opt-in tracks, `samples_data_sha`, the corpus
   manifest's SHA-256, every `SYNTHPASS_OCR_*` arm, the `mrz` parse arm and pivot, and an
   allowlisted set of measurement environment variables — never a key, token or credential.
5. **A document record** holds identity and track, the outcome-ledger row, check-digit states, the
   provider-input OCR text verbatim, band score, rotation, the zone recovered after repair and
   whether damaged-zone recovery produced it, the provider's field values, and — for a labelled
   specimen — mismatch counts and cell positions against the fixture, never the fixture's text.
   When Tier 1's input is narrowed to the accepted OCR pass, that pass's lines before repair are
   added under schema 2.
6. **Tracks are separated by directory and decided by one predicate** — the function the corpus
   walk uses to exclude opt-in tracks. `public/` holds the public corpus, covers and synthetic
   runs; `local/` holds `samples/local/`.
7. **The private track is archived only as text-free records in `private/`**: a type with no
   free-text field — outcome kind, format, check states, retry fields, timings, band score,
   mismatch positions and counts, and the recovered zone as character classes (letter, digit,
   filler, other), keyed by the image's SHA-256 rather than its filename. The prohibition on OCR
   dumps of the private track stands.
8. **Publishing to `bench-data` stays parked** until a publisher reads `public/` only and refuses
   any other track, with a test that proves it.

## Alternatives rejected

**Extend `MissOcrDump`.** A miss-scoped diagnostic behind a flag, whose rows copy the
hand-transcribed zone, and whose exact shape and population `tools/classify_mrz_mechanisms.py` and
ADR-0021's Phase 0 method depend on. Widening it to every document by default changes the
population under a published method. It stays; once the classifier reads the archive it can become
a projection of it, or be retired.

**Extend the committed outcome ledger.** Committed, pinned by the baseline's SHA-256, and defined
to carry no read text. Adding text would publish zone text.

**Write to `artifacts/`.** Gitignored and free, but per worktree: removing a worktree or
`git clean -x` deletes the history, and the stacked record this ADR exists for ends up scattered.

**Opt-in archiving.** Today's arrangement, and the reason most documents had no raw OCR when
ADR-0021 asked. An opt-in record exists only for the questions someone predicted.

**A writer script instead of the binaries.** CI and the synthetic runners do not go through
`run-bench.ps1`, so a script would miss them or become a second implementation of the record.

**Full-text private records in a separate directory.** Local, simple — and read by analysis
sessions, which sends a real document's zone off the device. It would also reverse the dump
prohibition without new evidence.

**No private archive at all.** The dump prohibition extended to the archive: safest, but it leaves
the private track with no cross-run history even where a text-free record could answer the
question. Rejected in favour of Decision 7, whose types make zone text unwritable by construction.

**Add every OCR pass's lines to `OcrPage`.** `OcrPage` is `wasm32`-clean and flows through every
production extraction; a per-pass vector would carry diagnostic-only zone text through the
product. If pass attribution is ever needed, it goes behind an opt-in trace entry point in
`synthpass-ocr`, used only by the benchmark, under its own decision.

**One appended `history.jsonl` per scope.** Concurrent runs interleave lines, a killed run leaves
a torn tail, and retention means rewriting the file. Per-run files avoid all three and concatenate
trivially for a later publish.

## Consequences

**Positive**

- A question about what OCR read is answered from runs already made, not from a new full pass.
- A/B arms describe themselves: every header records every arm, including the pass-cap and
  time-budget variables `OcrArms` does not cover.
- Private-track exclusion becomes a property of types and directories — the precondition the
  parked `bench-data` publication was waiting for.

**Negative**

- Disk grows with every run; retention is a manual prune. The size of one full run is measured
  and recorded when the first runner lands.
- The archive is invisible — in the git directory, not the tree — and is lost with the clone.
- Public zone text now persists on disk after every run and is read by analysis sessions, as every
  dump already was. Tracked documents still disclose only asset ID, counts, positions and classes,
  and the query tool defaults to that form.
- The record schema becomes a versioned contract; narrowing Tier 1's input bumps it once.

**Explicitly not licensed by this decision:** publishing any archive record; archiving private
text; private specimens in CI; any change to a gate, baseline, tolerance or denominator; an
`OcrPage` change made for this ADR's sake.

## Build order

From the 2026-09-27 design pass; each step is its own PR.

1. **0a, privacy hygiene:** `OutcomeRow`'s document-number mismatch recorded without either
   value; the `--dump-ocr` directory refused inside the tree unless ignored; stale comments
   corrected. (The `.gitignore` half, ignoring private paths in any case, goes first as its own patch,
   [#529](https://github.com/ruledicaprio/SynthPass/pull/529).)
2. **0b, parse and manifest parity:** the benchmark's real-specimen parses use the provider's
   `mrz` parse options, and the dump manifest records every OCR arm.
3. **1, the archive core and the `provider-bench` hook**, public and local tracks.
4. **2, the `synthpass-bench` hook**, after #510.
5. **3, the query and diff tool** (`tools/`, standard library only, disclosure-safe output).
6. **4, schema 2**, after #508 settles which OCR pass Tier 1 reads.
7. **5, the private track's text-free records** (Decision 7).

## Amendment (2026-09-29) — the nightly synthetic dataset is not an archive publication

**Status of this amendment:** Accepted (maintainer, 2026-09-29).

Decision 8 governs archive records and is unchanged. The nightly synthetic dataset on `bench-data`
predates this ADR and is not an archive record. It is a separate, allowlisted projection of synthetic
results. [ADR-0027](ADR-0027-ci-runners-measure-public-benchmark-arms.md) sets its scope,
including that it publishes no text, and what CI may publish at all.

## Amendment 1 (2026-09-29) — per-pass OCR readings for #574, behind a benchmark-only entry point

**Status of this amendment:** Accepted (maintainer, 2026-09-29). It is the separate decision the rejected
alternative "Add every OCR pass's lines to `OcrPage`" asks for before pass attribution is built.

### Context

[#574](https://github.com/ruledicaprio/SynthPass/issues/574) measures line-1 attempt selection,
which [ADR-0021](ADR-0021-fixed-grid-mrz-strips.md)'s 2026-09-24 decision takes as a narrow task.
A selector has to say which OCR pass a line-1 reading came from. The native retry loop appends
each pass's MRZ-shaped lines to one page text, so that provenance is gone before Tier 1 sees the
text, and `OcrPage::retry_variant_id` names only the pass that stopped the loop, by an index that
depends on which OCR arms are on.

### Decision

1. `synthpass-ocr` gains `NativeOcr::recognize_detailed_traced`. It returns the same `OcrPage` as
   `recognize_detailed`, plus one record per executed pass, in execution order: the pass id
   (`general` or `pass-NN`, the `retry_variant_id` vocabulary), the transform that produced its
   pixels, its image size, whether it failed, added nothing, was appended or was accepted, and
   every MRZ-shaped line it read with the line's index and bounding box.
2. It records every MRZ-shaped line — the lines the loop already appends — not a line-1
   classification. Choosing a line-1 candidate is interpretation, which `synthpass-ocr` does not
   do (ARCHITECTURE §13.1), and the lines around line 1 are the evidence a selector needs.
3. One code path: the product's OCR pass runs the same three ocrs steps the traced entry point
   reads its boxes from, so the benchmark measures what ships.
4. `OcrPage`, `recognize`, `recognize_detailed`, `synthpass-imageprep`, `synthpass-pipeline`,
   `ExtractionTrace` and every `SYNTHPASS_OCR_*` variable are unchanged. No extraction-path caller
   uses the traced entry point, and its types derive no `Serialize`.
5. The benchmarks report the records only when asked: `synthpass-bench --ocr-passes`, and
   `provider-bench --real-specimens --dump-ocr-passes`, which writes a separate JSONL next to
   `--out`, refuses `--include-private`, and refuses a destination inside the working tree that
   git does not ignore. The records never enter the `--out` trend report or the outcome ledger.
6. The change must be behaviour-neutral: the introducing PR shows byte-identical OCR text and
   identical per-document outcomes, retry stops and accepted passes before and after, on the five
   synthetic formats and on the real-specimen run.
7. When this archive's record schema lands, the public and local tracks carry these records under
   the same field names. The private track carries none (Decision 7).

### Alternatives rejected

- **A per-pass vector on `OcrPage`.** Rejected above for this ADR, for the same reason: it would
  carry diagnostic zone text through every production extraction.
- **An `SYNTHPASS_OCR_*` arm.** Capturing is not a treatment. An arm would enter `OcrArms`,
  `config_overrides` and the baseline refusal for a switch that changes no output.
- **Only line-1 candidates.** Bakes the selector's classification into the observation, so a
  selector's arms could no longer replay the same evidence.
- **Two OCR pass implementations** (the product on `get_text`, the trace on the explicit steps). Their
  agreement would rest on ocrs internals, and an ocrs bump could split them.
- **Waiting for the archive.** Blocks #574 on build steps 1–4 for a record a flag can produce now.

### Consequences

- A selector can name the pass, transform and position behind every line it considers.
- A second opt-in file format exists until the archive absorbs it.
- Boxes are in each pass's own image space (crop, upscale, deskew, turn), so they compare within a
  pass, not across passes.
