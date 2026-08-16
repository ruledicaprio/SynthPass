- **`mrz` recovers a dropped MRZ name-separator filler (`mrz` 0.6.3, no API change).** OCR
  routinely drops one (or both) of the two `<` filler characters in the `<<` primary/secondary
  name separator — `ESKANDARI<<MAREN` read as `KIRSCHNER<LUCA` or even `ESKANDARIMAREN` — and
  `clean_name` required a literal `"<<"` to split at all, so a collapsed separator silently sent
  `given_names` to `""`. A different failure mode than the crate's existing `fix_name_separator`
  repair, which only catches `<` misread as `K` (a `KK` pair), not `<` dropped outright. Shared
  code across every MRZ format (TD1/TD2/TD3/MRV-A/MRV-B), not specific to any one of them — first
  measured on MRV-A/MRV-B's first-ever accuracy run, where `given_names`/`surname` CER sat at
  47%-77%, but present on TD3 at the same magnitude once checked.

  Fix: `clean_name` falls back to a single `<` when no `<<` survives. More than halves
  `given_names`/`surname` CER on MRV-A, MRV-B, and TD3 alike (30 documents, `--seed 42 --profile
  clean`), with hit rate unchanged in every case — the name field was never what gated a Tier-1
  hit, since no format's check digits cover it. Safe for this crate's own generated corpus
  specifically because `synthpass-gen`'s name pools are single-token, never containing a
  legitimate internal `<`; against a real compound name the fallback is a best-effort guess, not a
  proof, which is why `FieldConfidence` never rates a name field above `MRZ_STRUCTURAL` regardless
  of which branch produced it. The rarer fully-collapsed case (both fillers dropped, no `<` left
  anywhere) has no separator evidence left and stays unrecoverable by construction — pinned as a
  known limit, not silently guessed at.

  Four regressions in `crates/mrz/tests/name_separator_collapse.rs` pin the recovery on MRV-A,
  MRV-B, TD3, and the still-unrecoverable fully-collapsed case.
