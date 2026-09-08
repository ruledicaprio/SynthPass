`provider-bench --real-specimens` classifies a specimen whose MRZ is redacted in the image
(`*_redacted_mrz`) as its own miss kind, `redacted_mrz`, instead of `checksum_failed`. The
redaction bar carries no recoverable zone, so these specimens are also excluded from the
Tier-1 hit-rate denominator — the same treatment an unlabelled document already gets for
field accuracy. The stdout "misses by kind" line notes how many were excluded.
`MissReason::Redacted` is new (`synthpass_bench`).
