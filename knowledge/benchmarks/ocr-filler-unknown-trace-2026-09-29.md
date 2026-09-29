# Isolated `<` and `?` per OCR pass: both engines emit isolated `<`, only the general engine emits `?`, and no `?` reaches a parsed zone

**Date:** 2026-09-29 · **MAIN:** `76f914e` (#595's branch head; its `synthpass-ocr` source is `main`'s at `a915742` apart from #598's report-only chargrid capture) · **DATA:** `396b22f` (the baseline pin; all 261 source hashes match the manifest, and 0 outcomes differ from the committed ledger); none for the synthetic runs (generated corpus) · **Evidence:** Observed (one local release `provider-bench --real-specimens --mrz-only --dump-ocr-passes` run and five `synthpass-bench --profile clean --count 100 --seed 0 --ocr-passes` runs, all from #595's neutrality measurement; no OCR ran for this note) plus Derived (each reading aligned against truth; a reading of `ocrs` 0.13.1 and `mrz` source) plus Hypothesized (mechanisms, marked where used) · **Status:** current

**2026-09-29.** This is the dated trace note asked for by
[#576](https://github.com/ruledicaprio/SynthPass/issues/576) item 3. The item asks, per OCR pass,
where the isolated `<` and the `?` in the OCR text come from. The comment fix in
`synthpass-ocr` is the other half of item 3 and is not part of this note. No code, ADR, corpus or
baseline changes with it. It quotes no OCR text: only counts, cell positions, character classes
and asset ids.

## The answer

- **Observed: the triage's first counts reproduce exactly on the real corpus.** `?` appears in 462
  of 2,044 general-pass readings (1,246 characters) and in 0 of 2,430 retry readings. An isolated
  `<` appears in 197 general readings (303) and in 625 retry readings (995).
- **`?` comes only from `ocrs`'s general engine, never from `mrz::UNKNOWN`.**
  - **Derived:** a reading is `ocrs`'s own line string, trimmed. `mrz::UNKNOWN` exists only in
    candidate strings inside `mrz`.
  - **Observed:** 0 `?` in all 4,284 retry readings (real and synthetic). 0 `?` in any accepted
    name-line reading. 0 `?` in any parsed zone (168 real, 467 synthetic).
- **Both engines emit isolated `<`.** In the name field, real: the general engine 105 in 73 of
  149 name-line readings, the retry engine 333 in 219 of 352.
  - **Observed:** on the labelled documents, the general pass keeps a real single separator as
    `<` in 18 of 32 reads. The retry passes keep one in 105 of 199.
  - So the comments "`ocrs` never emits an isolated `<`" and "never emits `<` at all" are wrong
    for both engines.
- **Most isolated `<` are not separators. They are half of a `<<`, or one cell of a filler
  tail.**
  - **Synthetic** (truth names hold no single `<`): all 595 isolated `<` in the name fields are
    damage. 393 are one cell of a `<<`, and 202 are in the filler tail.
  - **Hypothesized:** CTC repeat collapse. Of the 393 synthetic `<<` reads that came out as one
    `<`, 382 lost the partner cell outright, 8 read it as a letter and 3 as another character.
- **Observed: the diagnosis's "9 of 18 survive: 1 of 5 general, 8 of 13 retry" reproduces
  exactly, but the grouping is by the pass that validated, not the pass that read the line.**
  - The general pass's own reading keeps 10 of the 16 separator cells it read. 9 of those 10 are
    on documents that validated on a retry pass.
  - Grouped by the reading the parsed name line matches, survival is **1 of 6 general and 8 of 12
    retry**. One cell moves: Nigeria 2022, whose match wins by one edit.
  - The parser changed none of the 18 cells' `<` status. In 17 of 18 the parsed line and the
    matched reading agree at the cell.

## Populations and method

**Real, public only.** 261 documents, 1,894 executed passes and 4,474 readings, from
`provider-bench`'s `--dump-ocr-passes` output. The run used no `--include-private`, and all 261
`source_sha256` values match `samples/corpus.jsonl` at `76f914e`. Its outcome ledger differs from
the committed one on 0 of 261 documents: 139 hits, 139 / 151 = 92.1% scored and 139 / 261 = 53.3%
corpus-wide. Truth zones come from the 65 reviewed fixtures in `samples/ocr_fixtures/`.

**Synthetic.** Five formats × 100 clean seeds (seed 0 on), from `synthpass-bench --ocr-passes`:
500 documents, 1,755 passes and 3,416 readings. Hits are TD1 57, TD2 81, TD3 74, MRV-A 87 and
MRV-B 90, so 389 / 500.

- **Truth for 467 of the 500 seeds.** A report prints a seed's truth zone only when its
  `mrz_lines` CER is above 0. Truth was therefore pooled from all 146 clean seed-0 reports on
  disk, with 0 conflicts between them.
- **The 33 seeds with no truth anywhere** read their zone exactly in every report. They are in
  the reading-level counts but not in the name-line or truth-class counts.

**Definitions.**

- **Reading:** one line a pass kept under the retry loop's own filter (at least 20 non-whitespace
  characters). Every reading is counted, including visual-zone text that passes the filter.
  Counts are taken on the reading with its whitespace removed, as `mrz`'s `normalize_line` does
  first. Removing whitespace changes no count here: 197 and 625 either way.
- **Isolated `<`:** a `<` whose left and right neighbours, where they exist, are not `<`.
- **Pass kind:** `general` is order 0. Retry passes are grouped into families by the transform
  label before the colon: `mrz_variants`, `geometry_band`, `texture` and `quarter_turn`.
- **Zone line of a reading:** the reference line it is most similar to, if that similarity is at
  least 0.5. Similarity is 1 − Levenshtein distance / longer length. The reference is the truth
  zone where one exists. For the 106 real documents with a parsed zone but no fixture, the
  reference is the parser's own zone (`recovered_mrz_lines`), used for line identification only.
- **Name line:** line 1 of TD3, TD2, MRV-A and MRV-B, and line 3 of TD1. TD2 is included because
  its line 1 has the same layout.
- **Name field:** the cells aligned to truth cells from index 5 on (column 6) of line 1, or all
  of TD1 line 3. An inserted character takes the cell of the nearest aligned character to its
  left.
- **Truth class:** the truth cell each character aligns to. It is one of: a single `<`; one cell
  of a `<<` between letters; the filler tail (a run that reaches the end of the line); another
  `<` run; a letter; a digit; or none (an inserted character).

## Where they appear, per pass

**All readings** (Observed). "Zone-matched" means readings that match a reference zone line, on
documents that have one.

| Population | Pass kind | Passes | Readings | With `?` | `?` | With isolated `<` | Isolated `<` | Zone-matched readings | `?` in them | Isolated `<` in them |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Real | general | 261 | 2,044 | 462 | 1,246 | 197 | 303 | 332 | 46 | 247 |
| Real | retry | 1,633 | 2,430 | **0** | **0** | 625 | 995 | 879 | 0 | 684 |
| Real | `mrz_variants` | 920 | 1,696 | 0 | 0 | 445 | 715 | 643 | 0 | 502 |
| Real | `geometry_band` | 192 | 311 | 0 | 0 | 77 | 111 | 109 | 0 | 91 |
| Real | `texture` | 265 | 371 | 0 | 0 | 97 | 157 | 119 | 0 | 79 |
| Real | `quarter_turn` | 256 | 52 | 0 | 0 | 6 | 12 | 8 | 0 | 12 |
| Synthetic | general | 500 | 1,562 | 72 | 85 | 397 | 553 | 1,025 | 80 | 532 |
| Synthetic | retry | 1,255 | 1,854 | **0** | **0** | 438 | 598 | 1,444 | 0 | 580 |
| Synthetic | `mrz_variants` | 738 | 1,222 | 0 | 0 | 289 | 398 | 904 | 0 | 386 |
| Synthetic | `geometry_band` | 188 | 368 | 0 | 0 | 96 | 134 | 339 | 0 | 130 |
| Synthetic | `texture` | 167 | 264 | 0 | 0 | 53 | 66 | 201 | 0 | 64 |
| Synthetic | `quarter_turn` | 162 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 |

**Name field of name-line readings** (Observed; Derived for the truth classes, which cover
labelled real documents and synthetic seeds with truth only):

| Population | Pass kind | Name-line readings | With `?` | `?` | With isolated `<` | Isolated `<` | Truth classes of the isolated `<`: single / `<<` / tail / other run |
| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |
| Real | general | 149 | 20 | 29 | 73 | 105 | 18 / 10 / 9 / 1 (of 38 on 48 labelled readings) |
| Real | retry | 352 | 0 | 0 | 219 | 333 | 104 / 67 / 65 / 4 (of 240 on 220 labelled readings) |
| Real | `mrz_variants` | 262 | 0 | 0 | 158 | 248 | 72 / 45 / 47 / 2 |
| Real | `geometry_band` | 41 | 0 | 0 | 33 | 44 | 16 / 11 / 8 / 1 |
| Real | `texture` | 45 | 0 | 0 | 25 | 37 | 15 / 9 / 9 / 1 |
| Real | `quarter_turn` | 4 | 0 | 0 | 3 | 4 | 1 / 2 / 1 / 0 |
| Synthetic | general | 459 | 18 | 23 | 206 | 267 | 0 / 159 / 108 / 0 |
| Synthetic | retry | 531 | 0 | 0 | 274 | 328 | 0 / 234 / 94 / 0 |
| Synthetic | `mrz_variants` | 347 | 0 | 0 | 182 | 222 | 0 / 161 / 61 / 0 |
| Synthetic | `geometry_band` | 112 | 0 | 0 | 58 | 70 | 0 / 49 / 21 / 0 |
| Synthetic | `texture` | 72 | 0 | 0 | 34 | 36 | 0 / 24 / 12 / 0 |

No isolated `<` in a name field aligns to a truth letter or digit, on either population
(Derived). So in the name field an isolated `<` is always a printed `<`. The question is only
whether it is a real separator. On synthetic, it never is.

**How each reading renders a truth separator** (Derived, name field; one row per reading per
separator):

| Population | Pass kind | Single `<`: kept / letter / dropped / other | `<<`: both kept / one kept, partner dropped / one kept, partner a letter / one kept, partner other / none kept |
| --- | --- | --- | --- |
| Real (labelled) | general | 18 / 7 / 6 / 1 (of 32) | 26 / 7 / 2 / 1 / 7 (of 43) |
| Real (labelled) | retry | 105 / 49 / 45 / 0 (of 199) | 77 / 40 / 26 / 1 / 45 (of 189) |
| Synthetic | general | no single separators in truth | 216 / 153 / 3 / 3 / 84 (of 459) |
| Synthetic | retry | no single separators in truth | 181 / 229 / 5 / 0 / 116 (of 531) |

- **Every "one kept" `<` in these counts is isolated** (Observed). That makes `<<` → `<` the
  largest single origin of isolated `<` on synthetic, in both engines.
- **Hypothesized:** CTC decoding collapses repeated labels unless a blank separates them. A
  `<<` read as one `<`, with the partner cell gone rather than misread, is that collapse. It is
  more common in the retry engine (229 of 531) than in the general one (153 of 459) on synthetic
  name lines. This note does not test it.
- **Real "one kept, partner a letter"** is 26 of 189 retry reads, against 5 of 531 on synthetic.
  That is #575's filler-read-as-letter evidence showing up in the retry passes.

## Where the `?` sit

- **Real, general pass, 1,246 `?`** (Observed):
  - 632 are on the 90 documents with no zone to read: 490 on `redacted_mrz` and 142 on
    `no_mrz_expected`.
  - 614 are on the 171 documents with a parsed or truth zone. Only 46 of those are in
    zone-matched readings, and 29 are in a name field.
  - **Hypothesized:** the 490 on redacted documents are redaction bars read by the general engine.
    No image was inspected.
- **Truth class of the `?`** (Derived):
  - **Real name fields** (15 `?` on labelled readings): 11 on a printed `<` (8 tail, 2 `<<`,
    1 single), 3 on a letter, 1 inserted.
  - **Synthetic, all zone lines** (80 `?`): 32 on a digit, 24 on a letter, 18 on the filler tail
    and 6 on a `<<`.
  - **Synthetic name fields** (23 `?`): 13 on the tail, 6 on a `<<` and 4 on a letter.
- **Survival** (Observed):
  - **Real accepted passes:** 85 readings hold a `?` (154 characters). All 85 are in the 62
    general accepted passes, and none matches a zone line.
  - **Synthetic accepted passes:** none of the 1,224 readings holds a `?`.
  - **Parsed zones:** 0 `?` (168 real zones, 467 synthetic).

## `?` versus `mrz::UNKNOWN`

- **Derived from source: every character in a reading is `ocrs` output.**
  - `run_pass` (`crates/synthpass-ocr/src/lib.rs:1384` on `main`) returns each recognized line's
    `to_string()`. The pass trace records it trimmed and otherwise untouched (`pass_trace.rs`,
    `LineReading::text`).
  - No SynthPass code writes into a reading.
- **Derived: `mrz::UNKNOWN` (`crates/mrz/src/repair.rs:85`) cannot reach a reading.**
  - `width_candidates` inserts it (`repair.rs:248`), and only into candidate strings inside
    `mrz`.
  - `restored` (`crates/mrz/src/parser.rs:2144`) only widens a line that is already in the MRZ
    charset. The MRZ charset excludes `?`, so an `ocrs` `?` is never taken for an `UNKNOWN`
    cell.
  - A line that still holds a `?` does not parse: the parser returns `MrzError::BadCharacter`
    for it (pinned by `bad_character_reports_zero_based_td1_line_and_column` in
    `crates/mrz/src/lib.rs`). That fits the Observed 0 `?` in any parsed zone.
- **Derived: `ocrs` 0.13.1 has two ways to emit `?`.**
  - **The `?` class:** index 31 of its 96-character default alphabet, which the general engine
    uses.
  - **A fallback:** `recognition.rs:290` maps any output label past the end of the alphabet to
    `?` (`unwrap_or('?')`).
  - The MRZ engine's `allowed_chars` masks the `?` class. It lists alphabet indices only, so it
    does not mask a label past the end.
- **The trace cannot tell the two `ocrs` sources apart.** It records characters, not labels or
  scores. The 0 `?` in 4,284 retry readings says the fallback never fired there.
  **Hypothesized:** the recognition model has exactly one output per alphabet character plus
  the blank, so the fallback never fires anywhere. The model file was not inspected.
- **What the trace can show:** which pass emitted each `?`, and, against truth, which printed
  cell it sits on.
- **What it cannot show:** the glyph's pixels, the model's confidence, or what the other engine
  would have read on the same pixels. General and retry passes read different images (the full
  page against band crops, upscaled).

## The diagnosis's 18 separator cells, re-derived per pass

**Population:** the 33 name-scorable real hits. A cell is a truth `<` between two letters in the
name field. The overprinted Bangladesh 2023 specimen adds a nineteenth cell, which the parsed
text holds as a letter. It is excluded here, as the diagnosis excluded it. That leaves 18 cells
on 13 documents (Derived).

| Grouping | Cells | Kept as `<` | Read as a letter | Dropped |
| --- | ---: | ---: | ---: | ---: |
| All, parsed zone | 18 | **9** | 6 | 3 |
| By the pass that validated: general | 5 | 1 | 3 | 1 |
| By the pass that validated: retry | 13 | 8 | 3 | 2 |
| By the reading the parsed name line matches: general | 6 | 1 | 3 | 2 |
| By the reading the parsed name line matches: retry | 12 | 8 | 3 | 1 |
| Every general name-line reading at the cell (16 cells have one) | 16 | **10** | 3 | 3 |
| Every retry name-line reading at the cell (43 readings on 13 cells) | 43 | 31 | 4 | 8 |

- **The first three rows reproduce the diagnosis exactly.** Its split is by the pass that
  validated (`retry_stop`), which is also how #576 states it.
- **The general engine is not "nearly" unable to emit an isolated `<`.**
  - Its own reading keeps the cell on 10 of 16.
  - 9 of those 10 are on documents that validated on a retry pass: Djibouti 2019 (cells 11 and
    17), Dominican Republic 2020, India 2022, Somalia 2023, and the United Arab Emirates 2011
    (cells 18, 24, 31 and 37).
  - The tenth is Somaliland 2023, the one general-validated keep.
- **Attribution of the parsed line** (Derived):
  - 4 of 18 cells' parsed name lines equal a reading exactly. The other 14 are nearest matches,
    at similarity 0.86 to 0.98.
  - The one cell that changes group is `passports/Nigeria_Passport_Specimen_P0_NGA_2022_mrz.png`,
    cell 20. It validated on `pass-02`, but its parsed name line is nearest the general reading:
    2 edits against 3 for the best retry reading. The cell is dropped in both.
  - Two more documents are near ties across pass kinds, with the same outcome either way at their
    cells. They are Somalia 2023 (0.977 against 0.978) and Spain 2013 (0.84 against 0.86).
- **Parser against OCR** (Derived): at 17 of 18 cells the parsed line and its matched reading
  agree. `passports/Spain_Passport_Specimen_P0_ESP_2013_mrz.jpg` cell 12 is the exception: the
  matched reading drops the cell, and the parsed line has a letter aligned there.
- **What decides survival:** which reading the parser ends up with, not which engine can emit a
  `<`. The parser pairs lines across passes and does not always take the validating pass's line
  1 ([`wrong-physical-line1-2026-09-28.md`](wrong-physical-line1-2026-09-28.md), "Where the zones
  are assembled").

## What reaches the accepted reading and the parsed text

The name field of accepted-pass name-line readings (Observed; truth classes Derived):

| Population | Accepted pass kind | Name-line readings | With isolated `<` | Isolated `<` | Of which, labelled: single / `<<` / tail | `?` |
| --- | --- | ---: | ---: | ---: | --- | ---: |
| Real | general | 62 | 30 | 42 | 1 / 3 / 4 | 0 |
| Real | retry | 62 | 40 | 60 | 7 / 6 / 3 | 0 |
| Synthetic | general | 265 | 111 | 144 | 0 / 85 / 59 | 0 |
| Synthetic | retry | 119 | 66 | 80 | 0 / 55 / 25 | 0 |

**The parsed name field of hits:**

- **Real:** 97 isolated `<` across the 139 hits. The 33 labelled hits hold 27: 9 are real single
  separators, 7 are half of a `<<`, and 11 are in the filler tail.
- **Synthetic:** 151 across the 356 hits with truth: 120 half of a `<<` and 31 in the tail. None
  is a real separator.
- **What it means for names** (Derived): an isolated `<` in a parsed name field is a real
  separator in 9 of 27 labelled real cases and in 0 of 151 synthetic ones.

## How this was measured

- **The data** is #595's behaviour-neutrality measurement, produced 2026-09-29 by one script from
  release binaries built at `76f914e`:
  - `synthpass-bench`, sha256 prefix `9e5fddd275e08a41`;
  - `provider-bench`, sha256 prefix `8cc1a16538ecbcb2`.
- **Environment:** `SYNTHPASS_OCR_MODEL_DIR=/opt/synthpass/models`, and every other
  `SYNTHPASS_OCR_*` variable unset. The run manifest records chargrid `off`, texture `on`, and
  order, rotate and skew at `default`.

  ```
  synthpass-bench --document-type <td1|td2|td3|mrva|mrvb> --profile clean --count 100 --seed 0 \
    --dump-ocr --ocr-passes --out <fmt>.json
  provider-bench --real-specimens --mrz-only --progress \
    --dump-ocr --dump-ocr-hits --dump-ocr-passes \
    --assert-baseline knowledge/benchmarks/real-specimen-mrz-baseline.json --out report.json
  ```

- **Inputs read by this note:**
  - `provider-bench-ocr-passes.jsonl`, the per-pass records;
  - `provider-bench-miss-ocr-dump.jsonl`, for the parsed zones (`recovered_mrz_lines`);
  - `provider-bench-ocr-outcomes.jsonl`, for outcome, `names_exact` and `retry_stop`;
  - the five synthetic reports' `results[].ocr_passes` and `mrz_lines` rows.
  - None of these files is committed; they hold OCR text.
- **The analysis** is a Python script, not committed. It does Levenshtein alignment with
  backtrace, and nothing else.
- **Why the pass data describes `main`** (Observed): `git log 76f914e..origin/main --
  crates/synthpass-ocr` lists two commits.
  - `cdb649b` is #595's squash of this branch. Its `synthpass-ocr` source equals `76f914e`'s.
    Only `tests/native_ocr_e2e.rs` differs.
  - `26f1bba` (#598) records the chargrid attempt on the accepted pass. It is report-only, and
    chargrid is `off` by default.
- **What differs from `main`:** the build lacks five commits that `main` has, among them #593's
  `mrz` ranking (`9e974f3`) and #566's TD1 generator (`e9bea72`). See "What this does not claim".

## What this does not claim

- **Not a CI measurement, and nothing to re-bless.** It is one local run per population, and no
  count here is a headline.
- **Not `main`'s parsed zones.** The measured build lacks #593, which re-ranks seven real hits'
  zones (Kosovo 2011 and 2023, Canada `PP` 2023, Slovakia 2005, Netherlands 2014 blur, Sweden
  visa 2024 and Belgium ID 2018).
  - None of the seven is among the 13 documents behind the 18 cells.
  - **Derived:** ranking never turns a valid parse invalid, so the retry loop runs the same
    passes, and the pass readings are the same on `main`.
- **Synthetic TD1 is the pre-#566 generator (55 px line pitch).** `76f914e` is based on
  `a9461b1`, which predates `e9bea72`.
  - TD1's truth text is unchanged by #566: 0 conflicts across the 146 pooled reports, which span
    both generators.
  - TD1's OCR readings on `main` would differ.
- **The name-line counts depend on the 0.5 similarity cut.**
  - Measured at 0.4, 0.5, 0.6 and 0.7:
    - real general name-line readings: 157, 149, 138, 133;
    - real retry name-line readings: 385, 352, 306, 276;
    - synthetic retry name-line readings: 539, 531, 469, 389.
  - **Invariant at every cut:** 0 `?` in retry readings, 0 in accepted name lines and 0 in parsed
    zones. The 18-cell table is identical at every cut.
- **No mechanism is tested.** CTC collapse and redaction bars are Hypothesized.
- **Real truth classes cover the 65 labelled documents only.** The 106 unlabelled documents with
  a parsed zone enter the name-line counts, where their zone line is identified against the
  parser's own output.
- **Synthetic truth covers 467 of 500 seeds.** The 33 seeds without it are all exact reads.

## Candidates rejected on the way

- **Grouping survival by the validating pass as a statement about the engine.** It reproduces the
  diagnosis, but it attributes a retry-validated document's name line to the retry engine even
  when the parser took an earlier pass's line. It also hides that the general reading kept 9 of
  the 13 retry-validated cells it read.
- **Raw readings, with whitespace kept, for isolation.** Identical counts (197 and 625), so the
  simpler form was not kept.
- **A structural name-line rule** (the zone line with no digits). It cannot tell a name line
  from a visual-zone watermark or header line of letters.

## Open

- **For the comment lane (not this note):**
  - `chargrid.rs:7`, `lib.rs:66` and `lib.rs:1768` say `ocrs` never emits an isolated `<`.
  - `lib.rs:2061` says it never emits `<` at all.
  - Line numbers are on `main` at `a915742`. #576 cites `:1536` and `:1821` at `16221eb`.
- **Which `ocrs` path produced each `?`.** Answering it needs the recognizer's label output, or
  the model's output width, which the trace does not carry.
- **Whether the retry engine's higher `<<` → `<` rate is the beam search or the band images.**
  Separating the two needs a same-image, two-decoder probe.
