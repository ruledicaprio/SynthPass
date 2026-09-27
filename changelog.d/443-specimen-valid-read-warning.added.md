- **`provider-bench` flags checksum-valid reads on `checksum_failed_specimen` documents.** A
  specimen whose printed zone fails its own ICAO check digits is filed in
  `checksum_failed_specimen` whatever OCR returned. A read that verifies there was manufactured
  by the parser, and until now neither `false_positive_mrz` nor the outcome ledger could see it
  (issue #443). The run summary now prints a ⚠ line naming each such document, and every
  provider row in the JSON report carries `checksum_valid_on_failed_specimen`. Report-only: no
  bucket moves, the committed baseline is unchanged, and no gate reads the count yet.
