`provider-bench --real-specimens` classifies a specimen whose MRZ is redacted in the image
(`*_redacted_mrz`) as its own miss kind, `redacted_mrz`, ahead of the checksum gate instead
of counting it as `checksum_failed` (or, in one case, a Tier-1 HIT over the redacted zone).
The redaction bar carries no recoverable zone, so these specimens are also excluded from the
Tier-1 hit-rate denominator — the same treatment an unlabelled document already gets for
field accuracy. Measured on the 238-specimen real corpus: 9 specimens move to `redacted_mrz`,
`checksum_failed` drops 32 → 24, Tier-1 hit rate 50.4 % → 52.0 %. `MissReason::Redacted` is
new (`synthpass_bench`).
