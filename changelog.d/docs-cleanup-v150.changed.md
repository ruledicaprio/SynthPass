- **Documentation cleanup ahead of v1.5.0: stale claims, dead paths and superseded numbers
  corrected across `knowledge/`, `README.md`, `CONTRIBUTING.md` and rustdoc.** The real-specimen
  gate is advisory, not blocking — README and ROADMAP now say so, matching
  `real-specimen-gate.yml`'s own comment. `M1–M7` became `M1–M8` everywhere the platform roadmap
  is named, since M8 has been a real milestone since `ADR-0011`. The MRZ-formats criterion is
  recorded as **done** (all five formats read through the registered `mrz` provider, per-format
  rate measured through it, `ADR-0011`'s amendment) rather than as an open item, and M6 now closes
  on the named Tier-1 residual alone. The README world-coverage badge moved 73→75 to match
  `CORPUS_COVERAGE.md`'s own count.
  Licensing prose (`ARCHITECTURE.md` §6/§8, `crates/synthpass-license/src/lib.rs`,
  `LICENSING.md`, `VISION.md` §2) now matches `BRANDING.md` §5: capacity metering and
  entitlement, never a feature gate, and a stated caveat that no production license can be
  issued yet (the verifying key is still a placeholder — `technical_debt.md`) — distribution is
  source-build only until M8.
  Five broken rustdoc intra-doc links and one redundant explicit link target are fixed
  (`synthpass-bench`, `synthpass-die`, `synthpass-core`, `synthpass-gen` ×2); CI now builds
  `cargo doc --workspace --no-deps` with `-D rustdoc::broken_intra_doc_links` so a new one fails
  the build instead of rendering as dead prose on docs.rs. A dead crate path
  (`synthpass-ocr/src/geometry.rs`, moved to `synthpass-imageprep` when the crate split) is
  corrected in `project_principles.md` and `benchmarks/README.md`. Three stale sample-path/fixture
  citations in `synthpass-cli`, `synthpass-llm` and `synthpass-serve` rustdoc are corrected to
  files that actually exist.
  Retired M4-era synthetic-corpus figures (`~55%`/`~42%`) are labelled as such wherever they are
  not already superseded by a dated benchmark report, with a pointer to the current per-format
  rate in `benchmarks/README.md`; six dated benchmark reports gained a one-line "Superseded by"
  pointer to their successor or to the live headline; the duplicate-byte migration re-bless
  (run `35120396453`, commit `2f14e00`) is recorded as CI-measured (Observed), not just
  counterfactual arithmetic, with a matching dated entry in `benchmarks/README.md`'s weak-spot
  list. Tier-2 parity numbers in `prompts/README.md` and `technical_debt.md` are updated to the
  current 58.6%/52.5%/55.6% and repointed at `parity.rs`/`benchmarks/README.md` instead of
  `ROADMAP.md`. "Tier-1 hit rate reached 100%" is qualified as "6/6 (100%) on the v1.1.0
  six-document corpus" everywhere it appears unqualified.
  `CONTRIBUTING.md`'s Layout section now points at `ARCHITECTURE.md` §13.1 (which gains
  `synthpass-export`'s row) and the README's repository tree, instead of carrying its own
  third, drifting crate list.
  `scripts/check-headline-numbers.sh` gained three checks: the committed baseline's
  `measured_date` must match the date ROADMAP and `benchmarks/README.md` cite alongside it; the
  README's coverage badge must equal `CORPUS_COVERAGE.md`'s own Summary HIT count; and a retired
  M4-era figure appearing outside a dated report, an ADR, the archive or a changelog must be
  labelled `M4-era` or the build fails.
