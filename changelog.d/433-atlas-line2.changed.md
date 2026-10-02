- **The glyph atlas example reads any MRZ line, and a cell's truth may vary between seeds.**
  `glyph_atlas --line N` (from 1, default 1; TD1 has three lines, the other formats two) measures
  that line, and `--cells` is limited by its length. A cell whose truth glyph differs between seeds
  (line 2's dates, check digits and nationality) no longer aborts the run: the aggregates are keyed
  by `(cell, truth glyph, axis, step)`, and the summary has one row per glyph seen. The header's
  schema is 3: `line` is now the 1-based line number, and each `cells` entry lists `truths`, the
  sorted glyphs the cell takes across the seeds, in place of `truth`. A default run writes every
  render, cell and summary row byte for byte as before (#433). This is a benchmark example, not
  product code.
