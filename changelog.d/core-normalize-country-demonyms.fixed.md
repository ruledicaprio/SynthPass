Nationality and issuing country are now read from the adjectival form a document prints.
`normalize::country_code` resolves full country names through `mrz::code_for_name`, which holds
`Canada` but not `CANADIAN` and `Spain` but not `ESPAÑOLA` — so a passport's own wording fell
through unresolved. A small table of demonyms, consulted only after the shared name table
declines, closes that: tokens are matched whole-word across the whole string, so the bilingual
`CANADIAN/CANADIENNE` resolves to `CAN` while a string naming two *different* countries is left
alone rather than guessed at.

Every entry was observed in a real specimen. Measured on the parity corpus, both arms: **10
fields recovered, none lost — 52.5% to 55.6% overall, and the MRZ-holdout arm moved with it
(43.2% to 48.1% on reviewed fixtures).** Obvious sibling forms that no document here
prints are deliberately absent, as is `SLOVENSKA`, which collapses Slovakia and Slovenia once
OCR drops the accent that separates them. See
`knowledge/benchmarks/normalize-country-demonyms-2026-09-05.md`.

Added alongside: `cargo run -p synthpass-bench --example vocab_replay -- <parity log>` re-scores
a finished parity run against the current normalizers in under a second, without the model. It
accepts a change only when it flips at least one miss to a hit and no hits to misses.
