- **Synthetic benchmark rates with no Tier-1 hits are unmeasured.** Names exact among hits and
  wrong-accept rate now print `n/a` and serialize as `null` when the hit count is zero. Measured
  zeroes with a nonempty hit population remain `0.0`.
