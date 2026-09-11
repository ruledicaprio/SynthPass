- Fifteen passport specimens added to the real-specimen corpus (`samples/corpus.jsonl`
  238 → 254 rows, counting one previously unmanifested Kenya no-MRZ front): Angola,
  Azerbaijan, Bangladesh, Dominican Republic, India (×2), Indonesia, Nepal, Nigeria,
  Pakistan, Somalia, Somaliland, Uzbekistan, and two Djibouti books. Each carries a
  hand-transcribed, checksum-valid ground-truth MRZ under `samples/ocr_fixtures/`
  (`.json` + `.md`). Three (Indonesia, Pakistan, Angola) are photographed 90° sideways —
  deliberate test material for the ADR-0008 orientation track. Somaliland issues under
  `RSL`, which is not an ISO 3166 code (`issuing_state_recognized: false`, explained in the
  manifest `notes`).
