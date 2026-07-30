- **`mrz`'s ICAO 9303 doc-comment citations now match the corpus (no behaviour change).**
  Nine citations of "part 4 §4.2.2.2" for the long-document-number overflow encoding were wrong —
  Part 4 defines no overflow rule at all; the real sources are Part 5 note j / §4.2.4 (TD1) and
  Part 6 note j (TD2), with TD3 covered only by a deliberate extension by analogy. Corrected across
  `src/emit.rs`, `src/lib.rs`, `src/parser.rs`, `tests/fuzz_props.rs`, `tests/roundtrip.rs` and
  `README.md`; the full explanation now lives once in `parser::read_overflow`'s doc comment, with
  the other sites pointing at it. `src/dates.rs`'s two-digit-year century-pivot heuristic is now
  documented as this crate's own invention (Part 3 §4.8 is silent on century inference — checked
  against the whole corpus, not assumed), and `scripts/check-century-pivot.sh` runs in CI to catch
  the pivot constant drifting more than two years stale. Several fixture comments claiming direct
  ICAO specimen provenance were corrected: the TD3/TD1 specimens' matching Part 4/5 appendix figures
  are images in the source PDF, not extracted text, so they're corroborated via Part 6's literal TD2
  specimen instead. The MRV-A/MRV-B fixtures turned out not to be ICAO specimens at all but
  hand-derived by this crate, so Part 7's **real** published MRV-A and MRV-B specimens are now
  transcribed verbatim and pinned as tests alongside them (all three MRV-A check digits recomputed
  independently and matching). Two entries in `tests/icao_vectors.rs` credited to Part 4 are now
  labelled honestly: Part 4's specimen data page is an image in the source PDF, so the corpus cannot
  corroborate them, while the date-of-birth and date-of-expiry vectors are correctly credited to
  Part 6's literal TD2 text. See `knowledge/docs9303/CONFORMANCE_BASIS.md`'s S2 entries for the full
  corroboration record. No parsing, emission or arithmetic changed.
