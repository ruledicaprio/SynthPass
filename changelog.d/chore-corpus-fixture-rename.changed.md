- **OCR corpus fixtures renamed to one convention.** Every `samples/ocr_fixtures/`
  fixture and every `crates/synthpass-ocr/examples/mrz_corpus.rs` `CORPUS`/`NEGATIVE`
  entry now follows `Country_DocType_Specimen[_YYYY][_side]_mrz.ext` (`DocType` is
  `Passport` or `ID`; `_mrz` / `_no_mrz` / `_redacted_mrz` tags the MRZ-bearing side,
  keying the `integrity_survey.rs --mrz-only` filter). Injected issue years were
  verified against each specimen; several were corrected against the wider corpus
  (`Serbia_Passport_Specimen_2012_mrz`, `United_Arab_Emirates_Passport_Specimen_2011_mrz`,
  `China_Passport_Specimen_2012_mrz`). Run `./scripts/sync-samples.ps1` to pick up the
  renamed images once `samples-data` catches up. `samples/README.md`'s "existing files
  are intentionally left un-renamed" rule is retired.
