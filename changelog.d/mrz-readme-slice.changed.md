- **`mrz`'s README is a landing page again (381 → 212 lines), and its best prose
  now reaches docs.rs.** The check-digit blind-spot analysis — the closed-form
  mod-10 law, the corpus measurements, why the oracle is a strong *filter* but a
  weak *oracle* — lived in `blindspot.rs`'s module comment, which rustdoc never
  renders for a private module, while the README carried a lossy paraphrase of
  it. The full text now documents the public `Blindspot` enum. The same move
  brought the long-document-number, unknown-date, transliteration, expiry and
  country-registry material onto the public items they describe, so the API
  documentation explains itself instead of pointing at a file on GitHub. The
  caught/blind pair table and the per-swap caveat that qualifies it stay in the
  README, where a reader deciding whether to depend on the crate will see them.
  No example was lost: four items that had none gained one, `find_and_parse_with`
  gained the first public doctest for the escaped-and-merged OCR input the crate
  advertises, and the rest were duplicates of doctests that already existed.
