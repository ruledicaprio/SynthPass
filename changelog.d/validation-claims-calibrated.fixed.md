- **The live demo footer and the server UI's provenance badge no longer say a check digit
  "proves" the read.** A ✔ composite means the read is consistent with the printed check digits
  over the fields they cover — never the names, and not byte-for-byte proof. `synthpass-core`'s
  `v2` module keeps "checksum-proven" as its confidence vocabulary and now defines it, at the
  constant, as exactly that consistency; the pipeline, provider-context, README and roadmap prose
  that said "mathematically proven" now says what the arithmetic establishes.
