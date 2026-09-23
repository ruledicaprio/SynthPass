# `COMPARED_FIELDS` grows from ten to twelve columns: a discontinuity in every mean CER, not an improvement

**Date:** 2026-09-23 · **MAIN:** `19377a8` · **DATA:** `396b22f` · **Evidence:** Derived (from `synthpass-bench`'s `COMPARED_FIELDS`, `cer` and `total_loss`, and a census of `samples/ocr_fixtures/`; no benchmark run) · **Status:** current

ADR-0018 adds `optional_data_1` and `optional_data_2` to the extraction schema, and
`synthpass-bench`'s Tier-1 comparison scores exactly the schema's fields, in order — the `const _`
guard on `COMPARED_FIELDS` enforces it. So the per-document field comparison now has **twelve
columns plus `mrz_lines`** where it had ten, and the `personal_number` column narrows to what only
TD3 prints. Nothing about the reader changed. Three reported figures move anyway, and a later
reader comparing a report from before this change with one from after must not read the movement
as accuracy.

## What moves, and why

**`cer("", "") == 0.0`** — a correctly-read absent optional field is a perfect read, by design
(`crates/synthpass-bench/src/lib.rs`, `cer`'s doc comment). Both new columns are empty on both
sides for most documents, so each contributes a `0.0` wherever it is averaged in:

1. **`mean_cer_by_field`** gains two rows, `optional_data_1` and `optional_data_2`, both near
   zero on any mixed corpus. The `personal_number` row now compares the TD3 personal number only:
   on TD1, TD2 and the visas it is empty on both sides and contributes `0.0`, where before it
   carried those formats' optional data and any misread in it. A TD1 optional-data misread that
   used to show under `personal_number` now shows under `optional_data_1` or `optional_data_2`.
2. **`mean_cer_by_line`** averages the per-field rows that `mrz_field_line` places on each line.
   `optional_data_1` lands on TD1 line 1 and on TD2/MRV line 2; `optional_data_2` on TD1 line 2;
   neither on TD3 (its element is the `personal_number` span). Every line that gains a
   near-zero row has its mean pulled down with no change in what was read.
3. **`total_loss`** — a document whose MRZ was never found — now records twelve fields at
   `cer = 1.0` instead of ten. Per-field means are unaffected (each field is still one `1.0` per
   lost document), but any consumer summing outcomes across fields sees two more.

The census that sizes the effect, from the fixture walk that proved the slot rule on every
tracked zone (`2f91fc6`, `adr_0018_slot_proof.rs`, deleted with the field it proved recoverable):
**118 fixtures — 103 TD3, 15 TD1.** On the 103 TD3 documents both new columns are empty by rule
(the product schema keeps TD3's element under `personal_number`, so `optional_data_1` is `None`
there — see `synthpass_die::mrz_reader::reported_optional_data_1`). Of the 15 TD1 zones, 9 carry
optional data: 6 in slot 1 only, 1 in slot 2 only (Belgium 2021), 2 in both (Bosnia and
Herzegovina 2013, United States). So `optional_data_2` is populated on 3 of 118 documents and
`optional_data_1` on 8; everywhere else the two columns score a perfect `0.0`.

## What does not move

- The headline: hit rate, strict-name rate and the outcome ledger score documents and names, not
  per-field CER. Confidence values are not in the ledger either, so the real-specimen gate is
  expected to report **no outcome movement** for this change.
- Per-field CER on the ten original columns for TD3 documents. TD3 is 103 of 118 fixtures and
  every one of its ten columns compares exactly what it compared before.
- `field_match_rate` and `mean_cer` in the provider bench (`provider_bench.rs`): those score only
  fields that have ground truth, and `mrz_ground_truth`/`extraction_ground_truth` omit an empty
  value rather than storing it, so an empty optional-data field is excluded from the denominator
  instead of scoring as a free match.

## How to read a before/after pair

Compare per-field rows by name, not the line or document means. A drop in `mean_cer_by_line`
between a report at `19377a8` and one after ADR-0018, with the ten original rows unchanged, is
this discontinuity. A change in `optional_data_1`/`optional_data_2` themselves is real — those
columns had no name before and were folded into `personal_number` on every non-TD3 format.
