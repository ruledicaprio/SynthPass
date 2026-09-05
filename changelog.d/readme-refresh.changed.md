- **README caught up to the repo it describes.** The Repository-layout table was missing
  `synthpass-imageprep` (shipped since the browser demo started sharing the native
  preprocessing crate) and said nothing about `synthpass-core`'s deterministic
  country/demonym resolution for `nationality`/`issuing_country`, both now referenced
  alongside where they live and how they're proved. The "Corpus coverage" badge and
  `knowledge/CORPUS_COVERAGE.md`'s own Summary table had also drifted apart from the
  per-country table underneath them — 56/238 HIT and a leftover "Stale: 2" row that no
  country actually carried anymore. Recounted directly from the table: 57 HIT, 21 MISS,
  0 Stale, 158 no-specimen-yet, unchanged 238 total tracked codes.
