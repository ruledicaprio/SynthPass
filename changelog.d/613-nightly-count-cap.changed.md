- **The nightly bench refuses a `count` above 200.** `bench-data-collection.yml` accepted any positive
  integer for its `count` dispatch input, and runs five formats in parallel. One `synthpass-bench`
  process renders every image of its run before the first OCR pass, so memory grows with `--count`,
  and `--count 1000` and `--count 2000` each aborted five formats in about 10 s on 15.7 GB. The
  planning step now stops with an `::error::` that names the cap and the reason, before anything is
  built or measured. The default stays 200, which is the value that has run green nightly, and the
  fixed slice is untouched. Windowing `generate_corpus` is the real fix and is not part of this.
