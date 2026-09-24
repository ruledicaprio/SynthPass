- **Only a real calendar date is promoted as verified** (ADR-0019): a checksum-valid date of
  birth or expiry now reaches `extracted_v2` at confidence 1.0, and the Tier-2 MRZ hint, only
  when it names a real day. A `000000` placeholder, an all-filler field or a partly unknown one
  passes its check digit, and until now it was promoted as non-ISO text (`2000-00-00`, `<<<<<<`) in
  an ISO-typed slot. The expiry had no such gate at all. Those dates now keep the LLM's value at
  its own confidence. Tier-1 output and the real-specimen gate are unaffected, because promotion runs
  at Tier 2.
