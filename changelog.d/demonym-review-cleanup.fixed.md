- **`knowledge/ARCHITECTURE.md`'s Tier-2 accuracy paragraph was two numbers stale.** It quoted
  "~45% per-field exact-match" and a "25%-floor regression guard" from before the date and
  demonym normalizers shipped; the harness now measures 58.6%/52.5% (55.6% overall, 2026-09-05)
  and the floor was lowered to 15% when the scored field set widened. Updated to cite the current
  figures and point at `knowledge/benchmarks/normalize-country-demonyms-2026-09-05.md`.
- **`mine_country_vocab.rs` and `visual_zone_survey.rs` no longer panic** on a missing workspace
  root or failed OCR model load — both now print a clean diagnostic and exit(1), matching the
  rest of each file's own error handling.
- **The two examples' duplicated, drifted image-directory walkers are now one shared helper**
  (`crates/synthpass-ocr/examples/common/mod.rs`). Each carried its own 5-extension list
  (`jpg`/`jpeg`/`png`/`webp`/`gif`); `synthpass-bench`'s own copy of the same job already covered
  three more (`bmp`/`tif`/`tiff`) that these two silently skipped. The shared version uses the
  8-entry list.
- **`is_a_month_name` now shares `sex`/`document_type`'s exact-lookup helper** (`lookup`,
  generalized over the table's value type) instead of a second, differently-shaped table-search
  idiom for the same kind of question.
