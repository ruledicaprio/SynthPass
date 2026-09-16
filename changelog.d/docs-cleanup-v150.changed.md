- **Documentation brought in line with the code and the committed baseline before v1.5.0.**
  README and ROADMAP now call the real-specimen gate advisory (it is not a required check), record
  the M6 MRZ-formats criterion as done (#311, `ADR-0011`'s amendment), and name the roadmap
  `M1–M8`. Licensing prose (`ARCHITECTURE.md` §6/§8, `LICENSING.md`, `VISION.md` §2, the
  `synthpass-license` rustdoc) matches `BRANDING.md` §5 — entitlement and capacity metering, never a
  feature gate — and states that no production license can be issued while the verifying key is a
  placeholder. Stale figures are corrected or labelled: the coverage badge (75/238), the baseline
  date, Tier-2 parity in `prompts/README.md`, the M4-era `~55%`/`~42%`, and "100% in v1.1.0" (6/6).
  The 140 / 154 headline now cites the CI run that measured it (`35120396453`, commit `2f14e00`)
  with a dated weak-spot entry, and six superseded dated reports point to their successor.
  `CONTRIBUTING.md` links to `ARCHITECTURE.md` §13.1 instead of keeping its own crate list. Five
  broken rustdoc intra-doc links, a dead `geometry.rs` path and three stale sample paths are fixed.
- **Three new drift guards.** CI builds `cargo doc --workspace` with
  `-D rustdoc::broken_intra_doc_links`. `scripts/check-doc-links.sh` gains check 5: every
  `crates/`, `scripts/`, `tools/` or `.github/` file path cited anywhere must exist (one
  `git grep` pass, crate-relative paths resolved). `scripts/check-headline-numbers.sh` gains
  checks 11–13: the baseline's `measured_date` against the dates cited beside it, the README
  coverage badge against `CORPUS_COVERAGE.md`, and a denylist of retired figures unless labelled
  `M4-era`.
