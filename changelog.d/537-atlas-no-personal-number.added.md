- **`glyph_atlas --no-personal-number` measures an empty personal-number field.** The generator fills the
  personal number by default, so TD3 line 2's cells 28-41 never held a filler truth. The new switch (off by
  default) builds each seed's `GeneratorConfig` with `include_personal_number = false`: 14 fillers in cells
  28-41 and a `0` check digit in cell 42, as a passport with no personal number prints it. The generator reuses
  that number as every format's trailing optional field, so the flag blanks it on every format; no other field
  of a seed changes. A default run is byte for byte what it was, the flag shows in the header's `args`, and a
  run with it gets its own default `--out` suffix, `-nopn`. This is a benchmark example, not product code (#537).
