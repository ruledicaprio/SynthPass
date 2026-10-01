- **`glyph_atlas --format F --code XY` measures the glyph atlas for one document code.** The example
  that reads the two document-code cells of MRZ line one under one degradation at a time measured
  only `P<` of a TD3 line. `--format` (`td1`, `td2`, `td3`, `mrva`, `mrvb`) sets the layout and the
  `--cells` limit, and `--code` (two characters: a first letter that fits the format, then `A`-`Z`
  or the filler `<`) sets the code line one starts with; a non-default code needs a `--release`
  build. The header and summary record both (schema 2) and a default run measures what it always
  did.
