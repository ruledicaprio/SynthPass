- **`mrz` rustdoc builds clean, and CI now keeps it that way.** Six public doc
  comments linked to private items (`clean`, `confusable_alternatives`,
  `MAX_SUBSTITUTIONS`, `MAX_SUBSTITUTION_CANDIDATES`); rustdoc drops such links
  silently, so they rendered on docs.rs as cross-references that navigate
  nowhere. They are now plain code spans — the prose still names the helper,
  without widening the public API to satisfy a link. `TransliterationStyle::Expanded`
  was unresolved from `emit.rs` and now uses its crate-root re-export path, and
  three redundant explicit link targets in `lib.rs` were dropped. A new `docs`
  CI job runs `cargo doc -p mrz --all-features --no-deps` under
  `RUSTDOCFLAGS: -D warnings`, matching the feature set docs.rs itself builds.
