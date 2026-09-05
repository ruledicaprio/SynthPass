- **`mrz::codes()`.** Every `(code, name)` pair `country_name`/`code_for_name` are built on, in
  table order. Added so a caller outside the crate — `synthpass-core`'s `vocabulary_fingerprint`
  — can detect when this table itself changes, rather than only being able to hash the tables it
  owns directly.
