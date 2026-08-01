- **`mrz_corpus` runs again.** Two `CORPUS`/`NEGATIVE` entries named specimens that had been
  renamed by the `samples/` reorganization (`Israel_Biometric_Passport.jpg`,
  `BulgariaID_face.png`), and `find_sample` panicked on a name it could not resolve. Because the
  stranded entry was last in `CORPUS`, `cargo run -p synthpass-ocr --release --example
  mrz_corpus` — the command `CONTRIBUTING.md` gives for vetting a new specimen — aborted before
  printing the Tier-1 hit rate and before reaching the negative-control sweep at all. Names
  corrected, and a missing specimen is now reported and skipped rather than aborting the run,
  with the count surfaced next to the hit rate (the denominator still counts the whole declared
  corpus, so a stale entry cannot inflate the rate).
- **`visual_zone_survey --only` no longer overwrites the checked-in survey.** The run wrote
  `knowledge/visual-zone-survey.jsonl` unconditionally, so a deliberately narrowed `--only` run
  silently replaced the committed full-corpus dataset with a partial one. It now skips the write
  and says so, matching what the `--file` path already did. The ranked table also no longer keys
  on bare filenames, so two specimens sharing a basename across subdirectories can't collide and
  drop one from the output.
