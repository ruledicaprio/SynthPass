- **`ParseOptions::class_sweep` (off by default) reaches `solve_class_sweep` from `find_and_parse`.**
  Built with `ParseOptions::default().with_class_sweep(true)` — the first option added since the
  struct became `#[non_exhaustive]`, and additive because of it.

  The sweep runs in its own pass at the head of the damaged-capture search, for the fields that
  carry a check digit of their own (TD1 line 1's document number and line 2's dates; line 2's
  document number, dates and personal number on TD3; the same minus the personal number on TD2 and
  the MRV formats). Fields covered only by the composite are excluded: the composite cannot
  localise an error to a field, so sweeping one would be a guess the format cannot arbitrate.

  **It is a separate pass, preferred over the ordinary search, and that is a measured decision
  rather than a stylistic one.** Pooled into the existing damaged-capture shapes, the sweep loses
  to the machinery it complements. On the motivating shape — a nine-cell document number of one
  character read as its lookalike, check digit included — the single-substitution search finds
  **three** further readings that also validate, because swapping one cell of that run shifts the
  checksum by exactly the amount needed at any weight-3 position. Four disagreeing readings then
  reach the ambiguity guard, which correctly refuses to choose, and nothing is recovered at all.
  The sweep worked and still lost.

  So the tie-break is explicit, and it is a **prior, not arithmetic**: on the checksum evidence
  alone the four readings are equal. The sweep explains every differing cell with one decision
  about one glyph class, while each rival requires the recogniser to have read eight of nine
  identical glyphs correctly and exactly one of them differently — contradicting the uniformity the
  capture actually shows. That judgement is stated in `class_sweep_pass`'s own documentation rather
  than buried in an ordering.

  **Unmeasured on real documents.** It repairs a shape observed on one card back; how many
  documents it *breaks* is what the arm exists to find out. Off by default, and the damaged pass
  runs only after an ordinary read has already failed to validate.
