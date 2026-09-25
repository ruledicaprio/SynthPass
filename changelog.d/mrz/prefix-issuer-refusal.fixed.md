- **A TD3 line 1 one cell too long no longer defaults to an unresolved issuing state.** Line 1
  carries no check digit, so when the as-read issuing state doesn't resolve in `country_name`,
  `find_and_parse` now tries deleting each single cell of the three-letter issuing-state slot on
  its own. Exactly one resolving deletion is used; two or more real, distinct countries are
  equally admissible, so the read is refused instead of guessed. An already-resolving issuing
  state is left untouched either way.
