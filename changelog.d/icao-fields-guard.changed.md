- **The three ICAO field lists are pinned at compile time.** `CoreField::as_str` is now a
  `const fn`, and `synthpass-bench` and `synthpass-llm` each carry a `const` assertion against
  `CoreField::ALL`: the benchmark's `COMPARED_FIELDS` must match it name for name, and the Tier-2
  prompt's field list may differ from it only through two documented exceptions (`mrz_line` is
  prompt-only, `personal_number` is never prompted). Adding an ICAO field to one list and not the
  others is a build error instead of a silent gap. No runtime behaviour changes.
