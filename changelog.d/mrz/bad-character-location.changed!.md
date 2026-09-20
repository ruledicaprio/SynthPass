- **`MrzError::BadCharacter` now reports where the character was.** The variant carries
  `character`, `line` and `position` instead of a bare `char`; both indices are zero-based and
  counted in `char`s. `line` is `Option<usize>` and is `None` when there was no zone to count lines
  in -- the standalone `check_digit` helper is handed a single field, so its `position` is
  field-relative. Absence is not spelled `Some(0)`, because line 0 is a real line of a real zone.
