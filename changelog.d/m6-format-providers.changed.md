- **M6's MRZ-format-as-providers criterion clarified and tested.** `MrzReader`
  (`synthpass-die`, provider id `mrz`) already read all five ICAO 9303 MRZ formats — TD1, TD2,
  TD3, MRV-A and MRV-B — through `mrz::find_and_parse`; this was previously untested at the
  catalog level and undocumented on the type. Added a test proving a `ProviderCatalog` holding
  only `MrzReader`, looked up the same way `synthpass-pipeline`'s Tier-1 stage does
  (`find_reader(CostClass::Free, |c| c.deterministic)`), reads all five formats correctly, plus
  an OCR-free equivalence test in `synthpass-bench` confirming the catalog-routed read agrees
  with a direct `mrz::find_and_parse` call on format, checksum validity and document number.
  `MrzReader`'s rustdoc now states this explicitly. No behaviour change.
