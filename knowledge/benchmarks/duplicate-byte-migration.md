# Duplicate-byte migration (2026-09-16)

This record reconciles five duplicate-byte pairs without asserting physical-document identity. The existing 145/159 scored and 145/266 corpus-wide measurements remain historical measurements of the pre-migration 266-asset population. A new CI benchmark must establish any post-migration result.

The four reviewed fixture images moved to their canonical passport paths and their reviewed JSON/Markdown remained under `samples/ocr_fixtures/`; China keeps the reviewed `China_Passport_Specimen_2012_mrz` stem so the different-byte `P0_CHN` PNG remains unlabelled. Serbia retains the 2012 path and evidence; the 2009 path is removed as duplicated asset history, with no physical-document claim.

The exact byte hashes and path operations are immutable in `duplicate-byte-migration.json`. The corresponding samples-data operation is commit `469a4ee7723148917cf023f1a7ab2af80a0ef7d3`.

Counterfactual arithmetic, not a measured result: 261 candidate assets, 154 scored, 140 HIT, 140/154 (90.9%) and 140/261 corpus-wide (53.6%), assuming retained rows reproduce their former outcomes.
