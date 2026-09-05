- **`vocab_replay` no longer flags a case-only difference as a regression.** `parity.rs`'s
  `fields_match` folds every field through `.trim().to_uppercase()`, including
  `document_number`/`surname`, which have no crate normalizer — `renormalized()` skipped that
  fold for those two, so a value like `"Martin"` vs. `"MARTIN"` replayed as a false hit-to-miss
  regression with no vocabulary change involved, tripping the tool's own blocker.
