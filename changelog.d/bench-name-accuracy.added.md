- **Name accuracy is now a measured metric, in both bench harnesses.** No ICAO 9303 check digit
  covers `surname`/`given_names`, so a Tier-1 hit proves nothing about them — 178 of 377
  synthetic-corpus Tier-1 hits carry a wrong name (Derived, MAIN `c617254`; per-format counts in
  `knowledge/benchmarks/README.md`). `synthpass-bench` adds `strict_hits`/`strict_hit_rate`/
  `names_exact_among_hits`; `provider-bench` adds `strict_tier1_hit_rate`/
  `names_exact_among_hits` (`NotApplicable` when nothing is name-scorable); both add
  per-document `names_exact`/`name_error`. `hit`/`hit_rate`/`tier1_hit_rate` and both CI gates
  are unchanged — see
  [ADR-0013](knowledge/decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md).
