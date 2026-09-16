- **M6's MRZ-format-as-providers criterion clarified and tested.** `MrzReader`
  (`synthpass-die`, provider id `mrz`) already read all five ICAO 9303 MRZ formats — TD1, TD2,
  TD3, MRV-A and MRV-B — through `mrz::find_and_parse`; this was previously untested at the
  catalog level and undocumented on the type. Added a test proving a `ProviderCatalog` holding
  only `MrzReader`, looked up the same way `synthpass-pipeline`'s Tier-1 stage does
  (`find_reader(CostClass::Free, |c| c.deterministic)`), reads all five formats correctly, plus
  an OCR-free equivalence test in `synthpass-bench` confirming the catalog-routed read agrees
  with a direct `mrz::find_and_parse` call on format, checksum validity and document number.
  `MrzReader`'s rustdoc now states this explicitly. No behaviour change.
- **Per-format synthetic rates re-measured through the provider.** `benchmarks/README.md`'s
  synthetic row now reads 377 / 500 = 75.4% (TD3 78%, TD2 73%, TD1 54%, MRV-A 85%, MRV-B 87%),
  sourced to `provider-bench --mrz-only --document-type <fmt>`; the previous row mixed stale and
  30-document figures. `README.md`'s passport trend chart is now captioned as real specimens,
  which is what that track scores.
