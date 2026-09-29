- **`provider-bench` reports field accuracy over two quoted populations, each with its size.**
  Read quality covers accepted reads: Tier-1 hits and document-number mismatches. End-to-end covers
  the Tier-1 scored population, where an unread or errored document's fields score as absent. Both
  give a per-field mean CER and match rate with document counts. The existing all-labelled field
  match and mean CER are unchanged, labelled as such, and now carry per-field document counts
  (#564).
