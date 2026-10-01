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

## Amendment 2 (2026-09-29) — the chargrid capture for #575, on the same trace

**Status of this amendment:** Accepted (maintainer, 2026-09-29).

### Context

[#575](https://github.com/ruledicaprio/SynthPass/issues/575) part E reports that the chargrid name-line
repair (`SYNTHPASS_OCR_CHARGRID`, off by default) sometimes restores a filler one cell right of where it is
printed, and names candidate causes without evidence for any. The 2026-09-18 chargrid A/B could show that a
repaired name line was wrong, not why, because its reports held no read characters and no grid geometry
([reconciliation](../benchmarks/chargrid-ab-reconciliation-2026-09-24.md)).
[#411](https://github.com/ruledicaprio/SynthPass/issues/411) asks for the `ocrs` line-box height beside
chargrid's reads, so the ink floor's margin is measured on real documents rather than modelled. Both need
the same observation of one chargrid attempt.

### Decision

1. When the chargrid arm runs (`on` or `control`), `recognize_detailed_traced` attaches one chargrid record
   to the accepted pass, whose pixels the attempt read. It holds the arm and the verdict `OcrPage::chargrid`
   reports. Once a recognized line has matched the name line, it also holds: the name line's MRZ index and
   width, the parsed name line, the matched line's index and text, whether it was downscaled, the working
   image size, the `ocrs` line box, a SHA-256 of the grayscale band the ink is measured on, each glyph's
   character and edges, the dark-pixel count of every column of that band, and the ink floor. When a grid
   was fitted, it also holds the grid's origin and pitch, each glyph's cell, each cell's ink and the repair
   outcome.
2. The record is enough to replay the grid fit, the alignment, the ink measurement and both gates offline,
   with one input varied, without re-running OCR. Outputs are recorded as well as inputs, so a replay is
   checked against what ran before anything is varied.
3. The band is identified by its hash and never written. The column profile, the glyph characters and the
   lines are document content under the same rules as a pass's readings: the same opt-in flags, the same
   file, the same destination guard, the same refusal of `--include-private`, and never stdout, stderr, a
   log, `--out`, the outcome ledger or a tracked document. A dated note may cite positions, pitches, cell
   indices, per-cell ink and line-box heights by asset ID, as this ADR's disclosure rule allows.
4. It is behaviour-neutral, as amendment 1's Decision 6 requires. With the arm off nothing is recorded and
   nothing changes. With it on, the page and every outcome are identical with and without tracing, and
   identical to the arm before this change. The arm stays off by default, and its algorithm, constants and
   gates are unchanged.
5. The private track carries none of it (Decision 7), the hash included.

### Alternatives rejected

- **A second entry point or file.** Two traces of one OCR call could disagree about which pass the attempt
  read, and every reader would have to join them.
- **Log it under `SYNTHPASS_OCR_VERBOSE`.** That prints zone text and pixel profiles to a log, which this ADR
  forbids, and a log is not a schema a replay can check.
- **Write the band crop.** It moves a real document's pixels out of the image file. The hash lets a local
  analysis find and verify the same pixels from the image.
- **Record per-cell ink only.** Per-cell ink depends on the grid, so a replay that moves the origin or the
  pitch could not recompute it. The column profile can.
- **A separate logging path for #411.** Two records of one line box would drift.

### Consequences

- The note #575 asks for can overlay hand-marked cells on the recorded grid, and vary the pitch, the
  origin, the packing penalty or the glyph anchor one at a time.
- The ink floor's margin can be measured from recorded per-cell ink and line-box heights.
- The pass object gains one key, `chargrid`, which is `null` unless the arm ran.

## Amendment 3 (2026-09-29) — replaying a captured pass file, for #574's arms

**Status of this amendment:** Accepted (maintainer decision 2026-09-29, #574).

### Context

#574 compares three arms of a line-1 selector (off, control, on) that must read the same evidence. The
native retry loop is wall-clock budgeted, so two live runs of one binary can execute different passes and
read different text, and an A/B across live runs can attribute an OCR difference to the selector.
Amendment 1 records every executed pass, and each `provider-bench-ocr-passes.jsonl` row carries the page
text Tier 1 parsed (`ocr_text`); Tier 1 (`MrzReader`) is a pure function of that text. #575 E needs to
replay amendment 2's chargrid record from the same file.

### Decision

1. `provider-bench --real-specimens --mrz-only --replay-ocr-passes DIR` runs no OCR and loads no model.
   It builds each public-corpus document's page from DIR's pass-file row and runs the unchanged scoring
   path, so the scoring is shared and never copied. It writes the report, the outcome ledger and, when
   asked, the `--dump-ocr` and `--dump-ocr-hits` dumps a live run with the same flags writes. Unlike a
   live run it always writes the run manifest and the ledger, because the manifest names the capture.
   It covers Tier 1 only: Tier 2 would run an LLM over the replayed text, which is not measured here.
2. It refuses, naming the problem:
   - a row set that does not cover the public corpus exactly once (a missing `asset_id`, a duplicate, a
     document with no row, a row for an asset the corpus does not hold);
   - a row whose `source_sha256` differs from the corpus image's bytes;
   - a capture whose `corpus_manifest_sha256` differs from the current `samples/corpus.jsonl`;
   - a capture whose rows lack a key the replay needs (`retry_damaged_recovery`, `mrz_band_score`,
     listed by name);
   - `SYNTHPASS_OCR_*` arms that differ from the capture's, since the report's per-provider `ocr_arms`
     is read from the environment and must be true of the text it replays;
   - `--out` in the capture directory, which would replace the capture's own manifest and ledger;
   - `--include-private`, `--include-local`, `--write-baseline`, `--assert-baseline` and
     `--dump-ocr-passes`.
3. Its run manifest names the capture it replayed, as `replay_of`: the capture's run-manifest file name
   and the SHA-256 of that file. It copies the capture's `ocr_arms`. A live run's manifest has no such
   key and is byte-identical to what it was. The report's `mrz_class_sweep_arm` is the replaying
   process's, and its `model_paths` say that no OCR model was loaded.
4. Fidelity is checked, not assumed. A replay with every arm at its default is compared with the capture's
   own live outputs (`tools/bench_ab_diff.py --expect-identical --check-report`), and any difference
   stops the A/B. Every `BenchPage` field scoring reads is either derived from the corpus document, or
   carried by a row key. The row gains the two page values it did not carry, `retry_damaged_recovery` and
   `mrz_band_score`, both text-free, added at the end: every earlier key keeps its name, order and
   bytes. The file's text rules are unchanged.
5. One reader serves every replay, including #575 E's chargrid replay: `ocr_passes::read_rows`. The row
   types are owned and derive `Deserialize`, and a row written before a key existed reads back with the
   key defaulted and named in the reader's `missing_keys`, so a consumer refuses rather than replays a
   default.
6. A replayed result is labelled as a replay of a named capture. OCR runtime and retry behaviour are the
   capture's, never the replay's: `ocr_ms` is zero and a `budget` stop is the capture's.

### Alternatives rejected

- **Three live arms.** The same pass count is not guaranteed between runs, and a budget stop in one arm
  only makes the pair not an A/B.
- **A Python replay.** The parser and the selector are Rust; a mirror is a second implementation whose
  agreement nothing checks.
- **Replaying the miss dump.** It is written per provider and holds text only for dumped rows; the pass
  file is written once per document and carries provenance.
- **Overriding the report's `ocr_arms` with the capture's.** It would need the arms as values the report
  can hold, and the replaying process would then run under arms that are not the ones it reports. The
  replay is asked to run under the capture's arms instead.

### Consequences

- An arm A/B on the real corpus costs one live capture and seconds per arm, and every arm reads
  byte-identical text.
- A replay measures only code downstream of `OcrPage::text`. Anything upstream, the OCR passes, their
  order and the retry loop's stopping rule, is the capture's.
- The pass file is an input as well as an output: a schema change to it updates the reader too. A capture
  written before this amendment lacks the two new keys and cannot be replayed; recapture it.
- `mrz_band_score` is an `f64` the replay writes back into the dump row, so `synthpass-bench` parses
  JSON with `serde_json`'s `float_roundtrip`, which reads it back as the identical value.

## Amendment 4 (2026-10-01) — build step 3 as built (#645), and the size of a full run

**Status of this amendment:** Proposed. It records what #645 built, on the owner's answers in that
PR and its review; it changes no decision's intent.

### What step 3 settled

1. **One file per run and track.** Decision 3's "one JSON Lines file per run" means one file per
   run *and track*, because Decision 6 separates tracks by directory. A run with `--include-local`
   writes a `public/` file and a `local/` file that share one `run_id` and one header.
2. **A run with `--include-private` writes no archive.** Until build step 7's text-free records
   exist, such a run archives nothing, not even its public documents, and says so in one
   `warning: archive:` line. Decision 1's "every run" holds for every other run.
3. **The header, as built** (Decision 4):
   - The machine label is the operating system and the processor designation, for example
     `win11-i5-4570`, never the host name. It is restricted to `[A-Za-z0-9._-]`, and the
     designation to 48 characters.
   - `binary` is the file name; its SHA-256 identifies it. `argv` and `model_paths` keep their
     paths, for Decision 8's publisher to refuse or redact.
   - `env` holds the allowlisted variables, each looked up by its exact name; the environment is
     never iterated. A value outside `[A-Za-z0-9._-]{1,32}` is recorded as `null`.
   - In a replay, `retry_budget` is `null`: retry behaviour is the capture's (Amendment 3,
     Decision 6).
4. **No new tracing.** A record carries `ocr_passes` only when the run already traces them
   (`--ocr-passes`).

### The size of a full run

Consequences promised this number once the first runner landed. One live
`provider-bench --real-specimens --mrz-only` run of the public corpus, 261 documents, writes one
file of **656,152 bytes in 262 lines**: a 1,515-byte header and 261 records, with a median record
of 2,309 bytes and a largest of 12,537 (Observed, 2026-10-01, local, Linux, at #645's `e992e87`;
its review fixes changed only a few header bytes). That is about 0.66 MB a run, or about 240 MB a
year at one real run a day (Derived). Retention stays a manual prune.
