//! `provider-bench` — M7 multi-provider benchmark. Runs every registered
//! [`FieldReader`](synthpass_die::FieldReader) (today: the deterministic
//! `MrzReader` and the LLM's `LlmFieldReader`) against the same corpus and
//! reports, per provider: accuracy, speed, JSON validity, an
//! unsupported-assertion rate, and resident memory.
//!
//! Two corpus sources: the synthetic one from `synthpass-gen` (default), and
//! the real specimens in `samples/` (`--real-specimens`). The latter is the
//! only one that can contain a document with no MRZ, which is the population
//! the unsupported-assertion split below exists to measure separately.
//!
//! Requires the shipped GGUF present at the repo root (same precondition as
//! the existing `#[ignore]`d `native_llm_e2e`/`parity` tests in
//! `synthpass-llm`) — the LLM provider has to actually run to be measured.
//!
//! ```text
//! provider-bench [--count N] [--seed N] [--profile NAME] [--document-type TYPE] [--out PATH]
//!                [--measure-memory] [--real-specimens] [--limit N] [--mrz-only]
//!                [--format NAME] [--verbose] [--dump-ocr] [--dump-ocr-passes]
//!                [--replay-ocr-passes DIR] [--no-archive]
//!                [--write-baseline PATH] [--assert-baseline PATH]
//!   --count N          number of documents to check (default: 20)
//!   --seed N           base seed; document i uses seed N+i (default: 0)
//!   --profile NAME     clean|mobile|scanner|worn|border-kiosk|damaged|all (default: clean)
//!   --document-type TYPE  td1|td2|td3|mrva|mrvb — the ICAO 9303 MRZ format to *generate* for the
//!                      synthetic corpus (default: td3). Not the same axis as --format
//!                      below (which scopes which real samples/ directory --real-specimens
//!                      reads) — rejected together with --real-specimens, the same way
//!                      --format is rejected without it.
//!   --out PATH         report JSON path (default: artifacts/provider-bench-report.json)
//!   --measure-memory   sample process RSS around each provider's loop
//!                      (requires the `measure-memory` feature; see its doc
//!                      comment in Cargo.toml for why this is coarse)
//!   --real-specimens   run over samples/ instead of the synthetic corpus;
//!                      ground truth is optional per specimen, and
//!                      --count/--seed/--profile/--document-type are ignored
//!   --limit N          with --real-specimens: run N specimens spread evenly
//!                      across the (possibly --format-scoped) corpus
//!   --format NAME      with --real-specimens: restrict to one document
//!                      class — passport|id_card|driving_license|cover
//!                      (default: all classes), applied before --limit
//!   --verbose, -v      print the per-document breakdown behind each
//!                      provider's aggregates
//!   --dump-ocr         with --real-specimens: for every checksum_failed
//!                      miss, print the full pre-parse OCR text, the MRZ band
//!                      score, and the recovered MRZ zone + failing check
//!                      digit(s), and append a row per miss to
//!                      <out-dir>/provider-bench-miss-ocr-dump.jsonl.
//!                      Refuses an output directory inside the working tree
//!                      unless git ignores it.
//!                      The real-specimen equivalent of synthpass-bench's own
//!                      --dump-ocr, scoped to this one miss kind since a
//!                      real-specimen run is much larger than a synthetic
//!                      diagnostic one
//!   --dump-ocr-hits    with --real-specimens: add Tier-1 hits to the same OCR
//!                      dump; the same destination guard applies
//!   --dump-ocr-passes  with --real-specimens: write
//!                      <out-dir>/provider-bench-ocr-passes.jsonl, one row per OCR'd
//!                      document (whatever its outcome, written once from the OCR
//!                      prep, not once per provider): the full OCR text and every
//!                      executed OCR pass in order — its id, the transform that
//!                      produced its pixels, its image size, what the retry loop did
//!                      with it, and each MRZ-shaped line it read with its bounding
//!                      box (ADR-0024, amendment 1). Each row also carries `chargrid`,
//!                      the verdict the page reports (`null` with the arm off), and,
//!                      when SYNTHPASS_OCR_CHARGRID is `on` or `control`, the pass that
//!                      read the MRZ carries a `chargrid` object: the name-line repair's
//!                      matched line, glyph positions, fitted grid and ink (amendment
//!                      2). Refuses --include-private, and
//!                      refuses an output directory inside the working tree that git
//!                      does not ignore. The readings never enter the --out report,
//!                      the outcome ledger, stdout or stderr
//!   --replay-ocr-passes DIR
//!                      with --real-specimens --mrz-only: run **no OCR**. Read
//!                      DIR/provider-bench-ocr-passes.jsonl, a captured run's pass file
//!                      (`--dump-ocr-passes`), rebuild each public-corpus document's page
//!                      from its row, and score it with the same code a live run uses
//!                      (ADR-0024, amendment 3). Writes the same report, and, with
//!                      --dump-ocr / --dump-ocr-hits, the same dumps a live run with those
//!                      flags writes; it always writes the run manifest and the outcome
//!                      ledger, because the manifest names the capture: `replay_of` is the
//!                      capture's run-manifest file name and its SHA-256, and `ocr_arms`
//!                      is the capture's. Refuses, naming the problem: a row set that does
//!                      not cover the corpus exactly once; a row whose `source_sha256` is
//!                      not the corpus image's; a capture measured against another
//!                      `samples/corpus.jsonl`; a capture whose rows lack `retry_damaged_recovery`
//!                      or `mrz_band_score`; SYNTHPASS_OCR_* arms that differ from the
//!                      capture's; --out in DIR itself; and --include-private,
//!                      --include-local, --write-baseline, --assert-baseline and
//!                      --dump-ocr-passes. It measures only what happens to the captured
//!                      text (Tier 1): OCR runtime and retry behaviour are the capture's,
//!                      and the report's `model_paths` say no OCR model was loaded
//!   --no-archive       do not write the per-document archive (ADR-0024). By default every
//!                      run writes one JSON Lines file per track under
//!                      `<git common dir>/synthpass-bench-archive/{public,local}/`, or under
//!                      `SYNTHPASS_BENCH_ARCHIVE` when it names a directory
//!                      (`SYNTHPASS_BENCH_ARCHIVE=off` also turns it off). The files hold the
//!                      providers' OCR text, so they are document content and are never
//!                      printed or copied; a run with `--include-private` writes none. The
//!                      archive is a side output: no run reads it, and a problem with it is a
//!                      one-line `warning: archive: ...` on stderr, never a changed exit code,
//!                      report, ledger, dump or manifest
//!   --progress         force the per-document stderr progress log on even
//!                      when stderr is redirected. It is already on by
//!                      default whenever stderr is a terminal, so this flag
//!                      is only needed to keep it when piping to a file
//!   --mrz-only         register only the deterministic `mrz` reader, skipping
//!                      the Tier-2 LLM provider (and its ~1 GB GGUF, never
//!                      loaded). Leaves the deterministic OCR + Tier-1 pass, without
//!                      the LLM pass that dominates a full real-specimen run
//!                      (hours) — what the per-PR `real-specimen-gate.yml` CI job
//!                      runs (tens of minutes: the OCR retry chain is most of it).
//!                      Valid over either corpus source.
//!   --write-baseline PATH
//!                      after the run, write the `mrz` provider's Tier-1
//!                      snapshot (HIT count + miss-kind histogram + denominator)
//!                      as JSON to PATH and exit 0. How
//!                      `knowledge/benchmarks/real-specimen-mrz-baseline.json`
//!                      is (re)generated — always on CI, never hand-edited.
//!                      Also writes `real-specimen-outcomes.jsonl` next to PATH:
//!                      one JSON object per document (the outcome ledger), whose
//!                      SHA-256 is pinned in the baseline's `outcomes_sha256` —
//!                      see this file's `OutcomeRow` doc.
//!   --assert-baseline PATH
//!                      compare the `mrz` provider's Tier-1 snapshot against the
//!                      committed baseline at PATH and exit non-zero on a
//!                      regression: HIT count dropped, any miss bucket
//!                      (`checksum_failed`, `no_mrz_found`, …) grew, or the
//!                      outcome ledger next to PATH no longer hashes to
//!                      `outcomes_sha256`. A missing PATH is written and passes
//!                      (first-run bootstrap). Writes a text-free projection of
//!                      this run's own outcome ledger
//!                      (`real-specimen-outcomes-text-free.jsonl`, every
//!                      `miss_reason` reduced to its kind) under the `--out`
//!                      directory, never next to the baseline. When a committed
//!                      ledger is present,
//!                      also prints an informational (non-failing) per-document
//!                      diff against this run: the outcome changes, then every
//!                      other recorded field in two groups (deterministic;
//!                      timing-sensitive), also appended to
//!                      `$GITHUB_STEP_SUMMARY` when that is set. The per-PR
//!                      no-regression gate. Refuses to run (exit non-zero,
//!                      writes nothing) unless every `SYNTHPASS_OCR_*`
//!                      measurement arm is at its default — a baseline is only
//!                      valid for the default provider configuration.
//! ```
//!
//! Called with no arguments at all, this prints usage and exits `2` rather
//! than falling through to `Args::default()` — that default is a
//! 20-document synthetic run that also loads and runs the `llm` provider,
//! never what a bare invocation was meant to start (issue #510).

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use synthpass_bench::archive::{self, Archive, RunHeader};
use synthpass_bench::provider_bench::{
    run_provider_bench, run_provider_bench_real_with_options, run_provider_bench_replay,
    AssertionBucket, PopulationAccuracy, ProviderReport, RealDumpOptions, StrictNameHitRate,
    Tier1HitRate, UnsupportedAssertion,
};
use synthpass_bench::report::{
    ModelPathsReport, OutcomeRow, ProviderRow, RealSpecimenBaseline, RealSpecimenSnapshot, Report,
    OFF_DENOMINATOR_KINDS, REGRESSION_BUCKETS,
};
use synthpass_bench::{
    generate_corpus, load_real_specimens, miss_kind, MissReason, ProfileChoice, SpecimenClass,
};
use synthpass_die::{MrzReader, ProviderCatalog};
use synthpass_gen::DocumentType;
use synthpass_ocr::NativeOcr;
use synthpass_pipeline::{InferBackend, NativeInferer, OcrEngine, Pipeline, RustOcrEngine};

fn format_percentage(rate: Option<f64>) -> String {
    rate.map(|value| format!("{:.1}%", value * 100.0))
        .unwrap_or_else(|| "n/a".to_string())
}

struct Args {
    count: u64,
    seed: u64,
    profile: ProfileChoice,
    out: String,
    measure_memory: bool,
    /// Run over real specimens in `samples/` instead of the synthetic
    /// corpus — see `synthpass_bench::load_real_specimens`. `--count`/
    /// `--seed`/`--profile` are synthetic-corpus-only and ignored in this
    /// mode: a real specimen has no seed to start from and no capture
    /// profile to apply, it is what it is.
    real_specimens: bool,
    /// Cap on how many real specimens to run (`--real-specimens` only).
    /// `None` runs the whole corpus.
    limit: Option<usize>,
    /// Print the per-document breakdown behind each provider's aggregates.
    verbose: bool,
    /// With `real_specimens`: for every `checksum_failed` or `no_mrz_found`
    /// miss, dump the full pre-parse OCR text and the MRZ band score (and,
    /// for `checksum_failed`, the recovered MRZ zone + failing check
    /// digit(s)) to stdout, and a row per miss to
    /// `<out-dir>/provider-bench-miss-ocr-dump.jsonl`. A `checksum_failed`
    /// row for a labelled specimen also carries the hand-transcribed true
    /// zone and the character-mismatch count against it (`zone_mismatch`:
    /// `0` → the printed zone was read faithfully and its own check digits
    /// failed; `> 0` → OCR introduced the error). See
    /// `synthpass_bench::provider_bench::run_prepped`'s doc for why this is
    /// scoped to those two in-denominator miss kinds rather than every
    /// document the way `synthpass-bench --dump-ocr` is. This flag and
    /// `--dump-ocr-hits` refuse an output directory inside the working tree
    /// unless git ignores it.
    dump_ocr: bool,
    dump_ocr_hits: bool,
    /// With `real_specimens`: write `<out-dir>/provider-bench-ocr-passes.jsonl`,
    /// one row per OCR'd document with the full OCR text and every executed
    /// OCR pass's MRZ-shaped lines (transform, outcome, bounding box) — #574,
    /// ADR-0024 amendment 1. Its own flag, not a widening of `--dump-ocr`: the
    /// miss dump's row shape is depended on by
    /// `tools/classify_mrz_mechanisms.py`. Refuses `--include-private` (same
    /// rule as the OCR dumps) and an output directory inside the working tree
    /// that git does not ignore. Not an `SYNTHPASS_OCR_*` arm: it changes no
    /// output, so it does not enter the baseline refusal.
    dump_ocr_passes: bool,
    /// With `real_specimens` and `mrz_only`: run no OCR and replay the pass file
    /// in this directory instead (`provider-bench-ocr-passes.jsonl`, from an
    /// earlier `--dump-ocr-passes` run) — ADR-0024 amendment 3. See
    /// `load_replay` for what it refuses, and `check_replay_flags` for the
    /// flags it cannot be combined with.
    replay_ocr_passes: Option<String>,
    /// Force the per-document progress log on even when stderr is not a
    /// terminal. Progress is *already* on by default for an interactive run
    /// (see `show_progress` in `main`) — a full real-specimen pass takes over
    /// an hour and being silent for it is the complaint this exists to fix.
    /// The flag only matters when stderr is redirected, where the default is
    /// off so a captured log or a CI step stays clean.
    progress: bool,
    /// Restrict `--real-specimens` to one `samples/` document class (e.g.
    /// `passport`) via `synthpass_bench::classify_specimen`. `--real-specimens`
    /// only, applied before `--limit` so a stride subsamples the already-scoped
    /// population, not the whole corpus.
    format: Option<SpecimenClass>,
    /// The ICAO 9303 MRZ format to *generate* for the synthetic corpus.
    /// `None` means "unspecified" (defaults to TD3 where used) — kept as an
    /// `Option`, not a bare `DocumentType` defaulting to `TD3`, so
    /// `parse_args` can tell "explicitly TD3" apart from "never mentioned"
    /// the same way `format: Option<SpecimenClass>` does, and reject
    /// `--document-type` together with `--real-specimens` accordingly.
    /// Deliberately named `--document-type`, not `--format`: `--format`
    /// above already means `SpecimenClass` (which real `samples/` directory
    /// to read).
    document_type: Option<DocumentType>,
    /// Register only the deterministic `mrz` reader — skip the Tier-2 LLM
    /// provider entirely. The LLM pass dominates a real-specimen run (~19-37s
    /// per document, hours for the whole corpus) while the deterministic
    /// reader is µs-ms per document, and the LLM's GGUF (never loaded here) is
    /// a ~1 GB download. The per-PR `real-specimen-gate.yml` CI job runs with
    /// this on; it is also useful for any quick local Tier-1 measurement.
    mrz_only: bool,
    /// Add the gitignored, real-PII `samples/private/` specimens back into
    /// the `--real-specimens` walk, which excludes them by default (see
    /// `synthpass_bench::load_real_specimens`). Local, opt-in, and never set
    /// by any CI workflow: a private-track run's report names real identity
    /// documents and must not become a CI artifact. `--real-specimens` only.
    include_private: bool,
    /// Add the gitignored `samples/local/` track back into the
    /// `--real-specimens` walk: specimens usable on this machine whose source
    /// does not allow redistribution. Excluded by default so the committed
    /// baseline measures the public corpus only, and never set by any CI
    /// workflow. `--real-specimens` only.
    include_local: bool,
    /// Add the `samples/covers/` track back into the `--real-specimens`
    /// walk: cover-only images of any document type (ADR-0012's amendment).
    /// Excluded by default because a cover never carries an MRZ and so never
    /// enters the scored denominator — walking it on every run would spend
    /// real-specimen-gate OCR time for no accuracy signal. The one thing this
    /// flag is for: the hallucination check, since a checksum-valid MRZ read
    /// off a cover is `MissReason::FalsePositiveMrz`, which does not need to
    /// run on every PR. `--real-specimens` only.
    include_covers: bool,
    /// `--no-archive`: do not write the per-document archive (ADR-0024). Valid in every
    /// mode. `SYNTHPASS_BENCH_ARCHIVE=off` does the same; the flag wins over the variable.
    no_archive: bool,
    /// Write the `mrz` provider's Tier-1 snapshot to this path as JSON and
    /// exit 0 — the regeneration path for the committed real-specimen
    /// baseline. CI-only by convention (`--assert-baseline`'s doc explains
    /// why local and CI numbers differ).
    write_baseline: Option<String>,
    /// Compare the `mrz` provider's Tier-1 snapshot against the committed
    /// baseline at this path; exit non-zero on a regression (HIT count down,
    /// or a miss bucket up, beyond the baseline's `tolerance`). A path that
    /// does not exist yet is written and passes — the first-run bootstrap.
    assert_baseline: Option<String>,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            count: 20,
            seed: 0,
            profile: ProfileChoice::Clean,
            out: "artifacts/provider-bench-report.json".to_string(),
            measure_memory: false,
            real_specimens: false,
            limit: None,
            verbose: false,
            dump_ocr: false,
            dump_ocr_hits: false,
            dump_ocr_passes: false,
            replay_ocr_passes: None,
            progress: false,
            format: None,
            document_type: None,
            mrz_only: false,
            include_private: false,
            include_local: false,
            include_covers: false,
            no_archive: false,
            write_baseline: None,
            assert_baseline: None,
        }
    }
}

fn usage() {
    eprintln!(
        "Usage: provider-bench [--count N] [--seed N] [--profile NAME] [--out PATH] \
         [--measure-memory] [--real-specimens]"
    );
    eprintln!("  --count N          number of documents to check (default: 20)");
    eprintln!("  --seed N           base seed; document i uses seed N+i (default: 0)");
    eprintln!(
        "  --profile NAME     clean|mobile|scanner|worn|border-kiosk|damaged|all (default: clean)"
    );
    eprintln!(
        "  --out PATH         report JSON path (default: artifacts/provider-bench-report.json)"
    );
    eprintln!("  --measure-memory   sample process RSS around each provider's loop (coarse)");
    eprintln!(
        "  --real-specimens   run over samples/ instead of the synthetic corpus (ground truth \
         optional per specimen; --count/--seed/--profile are ignored)"
    );
    eprintln!(
        "  --limit N          with --real-specimens: run N specimens spread evenly across the \
         corpus (deterministic; default: all)"
    );
    eprintln!(
        "  --format NAME      with --real-specimens: restrict to one document class — \
         passport|id_card|driving_license (default: all classes)"
    );
    eprintln!(
        "  --document-type TYPE  td1|td2|td3|mrva|mrvb — the ICAO 9303 MRZ format to *generate* \
         for the synthetic corpus (default: td3). Not the same axis as --format, which scopes \
         which real samples/ directory --real-specimens reads; rejected together with \
         --real-specimens the same way --format is rejected without it."
    );
    eprintln!(
        "  --dump-ocr         with --real-specimens: for every checksum_failed or no_mrz_found \
         miss, print the full pre-parse OCR text + MRZ band score (+ recovered MRZ zone + failing \
         check digit(s) for checksum_failed), and append a row per miss to \
         <out-dir>/provider-bench-miss-ocr-dump.jsonl. Refuses an output directory inside the \
         working tree unless git ignores it"
    );
    eprintln!(
        "  --dump-ocr-hits    with --real-specimens: add Tier-1 hits to the same OCR dump; \
         without this flag --dump-ocr remains miss-only; the same destination guard applies"
    );
    eprintln!(
        "  --dump-ocr-passes  with --real-specimens: write <out-dir>/provider-bench-ocr-passes.jsonl, \
         one row per OCR'd document: the full OCR text and every executed OCR pass with its \
         transform, outcome and the MRZ-shaped lines it read (with boxes), each row's \
         chargrid verdict, and, with SYNTHPASS_OCR_CHARGRID on or control, a chargrid object \
         on the pass that read the MRZ. Refuses --include-private and an output directory \
         git does not ignore"
    );
    eprintln!(
        "  --replay-ocr-passes DIR  with --real-specimens --mrz-only: run no OCR; rebuild each \
         document's page from DIR/provider-bench-ocr-passes.jsonl (a captured --dump-ocr-passes \
         run) and score it with the same code. Writes the report, the outcome ledger and the run \
         manifest (whose replay_of names the capture), plus the --dump-ocr/--dump-ocr-hits dumps \
         when asked. Refuses a capture that does not cover the corpus exactly once, that read \
         other image bytes or another samples/corpus.jsonl, or that lacks retry_damaged_recovery \
         or mrz_band_score; --out in DIR; and --include-private, --include-local, \
         --write-baseline, --assert-baseline and --dump-ocr-passes"
    );
    eprintln!(
        "  --progress         force the per-document stderr progress log on when stderr is \
         redirected (already on by default when stderr is a terminal)"
    );
    eprintln!(
        "  --mrz-only         register only the deterministic mrz reader, skipping the Tier-2 \
         LLM provider and its GGUF (the per-PR real-specimen CI gate runs this way)"
    );
    eprintln!(
        "  --include-private  with --real-specimens: add the gitignored samples/private/ \
         real-PII specimens back into the walk (excluded by default; local opt-in only — a \
         private-track report must never become a CI artifact)"
    );
    eprintln!(
        "  --include-local    with --real-specimens: add the gitignored samples/local/ track \
         (specimens whose source does not allow redistribution) back into the walk (excluded \
         by default; local opt-in only)"
    );
    eprintln!(
        "  --include-covers   with --real-specimens: add the samples/covers/ track (cover-only \
         images, any document type) back into the walk, for the hallucination check — a \
         checksum-valid MRZ read off a cover. Off by default: covers never enter the scored \
         denominator, so they are excluded from the per-PR gate's default walk"
    );
    eprintln!(
        "  --no-archive       do not write the per-document archive (default: one JSON Lines file \
         per track under <git common dir>/synthpass-bench-archive/, or SYNTHPASS_BENCH_ARCHIVE; \
         SYNTHPASS_BENCH_ARCHIVE=off also disables it; a --include-private run writes none)"
    );
    eprintln!(
        "  --write-baseline PATH  write the mrz provider's Tier-1 snapshot (HIT count + \
         miss-kind histogram) to PATH as JSON, plus the outcome ledger next to it, and exit"
    );
    eprintln!(
        "  --assert-baseline PATH  compare the mrz provider's Tier-1 snapshot against the \
         committed baseline at PATH; exit non-zero on a regression or an outcome-ledger sha \
         mismatch (missing PATH is written and passes)"
    );
}

/// `true` iff `main` should print usage and exit rather than parse `args` at
/// all — currently just "no arguments", the one case `Args::default()` would
/// otherwise silently accept and turn into a real (and, with the `llm`
/// provider, expensive) run (issue #510).
fn requires_usage(args: &[String]) -> bool {
    args.is_empty()
}

/// Hand-rolled flag parser, consistent with `synthpass-bench`'s own binary
/// (no clap, no new arg-parsing dependency).
fn parse_args(args: &[String]) -> Result<Args, String> {
    let mut parsed = Args::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--count" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--count requires a value".to_string())?;
                parsed.count = v
                    .parse::<u64>()
                    .map_err(|_| format!("--count: not a valid number: {v}"))?;
                i += 2;
            }
            "--seed" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--seed requires a value".to_string())?;
                parsed.seed = v
                    .parse::<u64>()
                    .map_err(|_| format!("--seed: not a valid number: {v}"))?;
                i += 2;
            }
            "--profile" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--profile requires a value".to_string())?;
                parsed.profile = ProfileChoice::parse(v)?;
                i += 2;
            }
            "--out" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--out requires a value".to_string())?;
                parsed.out = v.clone();
                i += 2;
            }
            "--measure-memory" => {
                parsed.measure_memory = true;
                i += 1;
            }
            "--real-specimens" => {
                parsed.real_specimens = true;
                i += 1;
            }
            "--verbose" | "-v" => {
                parsed.verbose = true;
                i += 1;
            }
            "--dump-ocr" => {
                parsed.dump_ocr = true;
                i += 1;
            }
            "--dump-ocr-hits" => {
                parsed.dump_ocr_hits = true;
                i += 1;
            }
            "--dump-ocr-passes" => {
                parsed.dump_ocr_passes = true;
                i += 1;
            }
            "--replay-ocr-passes" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--replay-ocr-passes requires a directory".to_string())?;
                parsed.replay_ocr_passes = Some(v.clone());
                i += 2;
            }
            // Deliberately not gated on --real-specimens the way --dump-ocr
            // is: the synthetic corpus is slow enough to want progress too,
            // and a flag that only forces on an already-default behaviour has
            // no invalid combination to reject.
            "--progress" => {
                parsed.progress = true;
                i += 1;
            }
            "--limit" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--limit requires a value".to_string())?;
                let n = v
                    .parse::<usize>()
                    .map_err(|_| format!("--limit: not a valid number: {v}"))?;
                if n == 0 {
                    return Err("--limit must be at least 1".to_string());
                }
                parsed.limit = Some(n);
                i += 2;
            }
            "--format" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--format requires a value".to_string())?;
                parsed.format = Some(SpecimenClass::parse(v)?);
                i += 2;
            }
            "--document-type" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--document-type requires a value".to_string())?;
                parsed.document_type = Some(DocumentType::parse(v)?);
                i += 2;
            }
            "--mrz-only" => {
                parsed.mrz_only = true;
                i += 1;
            }
            "--include-private" => {
                parsed.include_private = true;
                i += 1;
            }
            "--include-local" => {
                parsed.include_local = true;
                i += 1;
            }
            "--include-covers" => {
                parsed.include_covers = true;
                i += 1;
            }
            "--no-archive" => {
                parsed.no_archive = true;
                i += 1;
            }
            "--write-baseline" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--write-baseline requires a path".to_string())?;
                parsed.write_baseline = Some(v.clone());
                i += 2;
            }
            "--assert-baseline" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--assert-baseline requires a path".to_string())?;
                parsed.assert_baseline = Some(v.clone());
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if parsed.format.is_some() && !parsed.real_specimens {
        return Err("--format is only valid together with --real-specimens".to_string());
    }
    if (parsed.dump_ocr || parsed.dump_ocr_hits) && !parsed.real_specimens {
        return Err(
            "--dump-ocr and --dump-ocr-hits are only valid together with --real-specimens"
                .to_string(),
        );
    }
    if parsed.include_private && !parsed.real_specimens {
        return Err("--include-private is only valid together with --real-specimens".to_string());
    }
    if parsed.dump_ocr_passes && !parsed.real_specimens {
        return Err("--dump-ocr-passes is only valid together with --real-specimens".to_string());
    }
    if parsed.include_private && (parsed.dump_ocr || parsed.dump_ocr_hits || parsed.dump_ocr_passes)
    {
        return Err("OCR dumps cannot include samples/private/".to_string());
    }
    if parsed.include_local && !parsed.real_specimens {
        return Err("--include-local is only valid together with --real-specimens".to_string());
    }
    if parsed.include_covers && !parsed.real_specimens {
        return Err("--include-covers is only valid together with --real-specimens".to_string());
    }
    if parsed.document_type.is_some() && parsed.real_specimens {
        return Err(
            "--document-type generates the synthetic corpus and is not valid together with \
             --real-specimens (use --format to scope which real samples/ directory is read)"
                .to_string(),
        );
    }
    if parsed.write_baseline.is_some() && parsed.assert_baseline.is_some() {
        return Err(
            "--write-baseline and --assert-baseline are mutually exclusive: one regenerates the \
             baseline, the other checks against it"
                .to_string(),
        );
    }
    // The committed baseline is defined over the public corpus. Writing it
    // from a walk that includes an opt-in track would bless a denominator CI
    // can never reproduce; asserting against it would report the extra
    // documents as a corpus-size change on every run.
    if (parsed.include_private || parsed.include_local || parsed.include_covers)
        && (parsed.write_baseline.is_some() || parsed.assert_baseline.is_some())
    {
        return Err(
            "--include-private/--include-local/--include-covers cannot be combined with \
             --write-baseline or --assert-baseline: the committed baseline measures the public \
             corpus only"
                .to_string(),
        );
    }
    if parsed.replay_ocr_passes.is_some() {
        check_replay_flags(&parsed)?;
    }
    Ok(parsed)
}

/// What `--replay-ocr-passes` cannot be combined with, each with its reason.
///
/// A replay is Tier 1 over the public corpus, scored by the code a live run
/// uses, from text a capture recorded: anything that would make it something
/// else is refused rather than quietly ignored.
fn check_replay_flags(parsed: &Args) -> Result<(), String> {
    if !parsed.real_specimens {
        return Err("--replay-ocr-passes is only valid together with --real-specimens".to_string());
    }
    if !parsed.mrz_only {
        return Err(
            "--replay-ocr-passes requires --mrz-only: a replay covers Tier 1 only, and Tier 2 \
             would run an LLM over the replayed text, which this replay does not measure"
                .to_string(),
        );
    }
    for (flag, given, why) in [
        (
            "--include-private",
            parsed.include_private,
            "a replay covers the public corpus only, and a capture never holds the private track",
        ),
        (
            "--include-local",
            parsed.include_local,
            "a replay covers the public corpus only",
        ),
        (
            "--write-baseline",
            parsed.write_baseline.is_some(),
            "the committed baseline is measured by a live run",
        ),
        (
            "--assert-baseline",
            parsed.assert_baseline.is_some(),
            "the committed baseline is measured by a live run",
        ),
        (
            "--dump-ocr-passes",
            parsed.dump_ocr_passes,
            "a replay reads the pass file and writes none",
        ),
    ] {
        if given {
            return Err(format!(
                "{flag} cannot be combined with --replay-ocr-passes: {why}"
            ));
        }
    }
    Ok(())
}

/// Take `n` specimens spread evenly across `all`, preserving order.
///
/// Deliberately a stride, not `all.truncate(n)`. `load_real_specimens` returns
/// paths sorted, and `samples/` is organised by document class
/// (`driving_licenses/`, `id_cards/`, `misc/`, `ocr_fixtures/`, `passports/`),
/// so taking the first `n` would fill the whole subset from the alphabetically
/// earliest directories and include no passports at all — a subset that
/// silently answers a different question than the corpus it claims to sample.
/// An even stride keeps the class mix roughly proportional.
///
/// Deterministic by construction: same `n`, same corpus, same subset, every
/// run. A random sample would be a better estimator in principle and a worse
/// benchmark in practice — two runs that disagree because they drew different
/// documents cannot be compared, which is the same reasoning
/// `ProviderCatalog`'s insertion-ordered consultation already applies to
/// provider order.
fn subsample<T>(all: Vec<T>, n: usize) -> Vec<T> {
    let total = all.len();
    if n >= total {
        return all;
    }
    // `i * total / n` for i in 0..n — integer arithmetic, so no float rounding
    // decides membership, and the first element is always included.
    let mut keep: Vec<Option<T>> = all.into_iter().map(Some).collect();
    (0..n)
        .map(|i| i * total / n)
        .filter_map(|idx| keep[idx].take())
        .collect()
}

/// `SYNTHPASS_MODEL_PATH`/default GGUF filename, mirroring
/// `synthpass-pipeline`'s private `infer::backend_from_env` — that function
/// isn't reachable from outside the crate, so this benchmark constructs
/// `NativeInferer` directly the same way `backend_from_env` does internally.
fn model_path() -> String {
    std::env::var("SYNTHPASS_MODEL_PATH")
        .unwrap_or_else(|_| "./qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string())
}

fn n_ctx() -> u32 {
    std::env::var("SYNTHPASS_MODEL_N_CTX")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2048)
}

/// A one-reader catalog holding just the deterministic `mrz` provider — the
/// `--mrz-only` path. [`MrzReader::new`] takes no arguments and pulls in no
/// OCR/LLM dependency (it lives in `synthpass-die`), so this needs neither a
/// [`Pipeline`] nor a model file.
fn mrz_only_catalog() -> ProviderCatalog {
    ProviderCatalog::builder()
        .with_reader(std::sync::Arc::new(MrzReader::new()))
        .build()
        .expect("a single reader cannot collide on id")
}

/// The fixed name `--write-baseline`/`--assert-baseline` write the outcome
/// ledger under, always in the same directory as the baseline JSON itself
/// (`outcomes_ledger_path`) — never a name the caller chooses, so a reader who
/// knows the baseline's path always knows the ledger's.
const OUTCOMES_LEDGER_FILENAME: &str = "real-specimen-outcomes.jsonl";

/// The `mrz` provider's `documents_detail`, one [`OutcomeRow`] each, sorted by
/// [`OutcomeRow::sort_key`] — byte-deterministic for a given run, so the
/// ledger diffs cleanly across CI runs that measured the same corpus.
fn build_outcome_rows(mrz: &ProviderReport) -> Vec<OutcomeRow> {
    let mut rows: Vec<OutcomeRow> = mrz.documents_detail.iter().map(OutcomeRow::from).collect();
    rows.sort_by(|a, b| a.sort_key().cmp(b.sort_key()));
    rows
}

/// One compact JSON line per row, each terminated by `\n` (including the
/// last) — the exact bytes [`sha256_hex`] hashes for
/// `RealSpecimenBaseline::outcomes_sha256`, so any change to this function is
/// a ledger-format change.
fn ledger_bytes(rows: &[OutcomeRow]) -> Vec<u8> {
    let mut buf = String::new();
    for row in rows {
        buf.push_str(&serde_json::to_string(row).expect("serialize outcome row"));
        buf.push('\n');
    }
    buf.into_bytes()
}

/// Lowercase hex SHA-256, byte-by-byte the same way `synthpass-export`'s
/// `writer.rs` and `synthpass-ocr`'s `build.rs` already do: sha2 0.11's
/// `finalize()` returns a `hybrid_array::Array` with no `LowerHex` impl.
use synthpass_bench::archive::sha256_hex;

/// Where `--write-baseline PATH` / `--assert-baseline PATH` read or write the
/// outcome ledger: [`OUTCOMES_LEDGER_FILENAME`] in the same directory as the
/// baseline itself, never a path the caller chooses independently — the two
/// files are a pair (`outcomes_sha256` only means something next to the
/// ledger it was computed from).
fn outcomes_ledger_path(baseline_path: &str) -> std::path::PathBuf {
    let dir = std::path::Path::new(baseline_path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    dir.join(OUTCOMES_LEDGER_FILENAME)
}

/// Verifies `ledger_bytes` hashes to `expected` (`RealSpecimenBaseline::outcomes_sha256`,
/// lowercase hex). `Err` is the one way the outcome ledger turns into a gate
/// **failure**: a committed ledger that no longer matches its baseline was
/// edited by hand, or belongs to a different run entirely — either way the
/// per-document evidence beneath the aggregate counts can no longer be
/// trusted.
fn verify_ledger_sha(ledger_bytes: &[u8], expected: &str) -> Result<(), String> {
    let actual = sha256_hex(ledger_bytes);
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "outcome ledger sha256 mismatch: baseline says {expected}, the committed ledger \
             hashes to {actual} — the committed ledger was edited by hand or does not belong to \
             this baseline"
        ))
    }
}

/// Parses a ledger file's exact bytes back into rows, one per non-empty line
/// — the inverse of [`ledger_bytes`], so a round trip through disk is
/// lossless and a malformed committed ledger is reported rather than panicking.
fn parse_ledger(bytes: &[u8]) -> Result<Vec<OutcomeRow>, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| format!("ledger is not valid UTF-8: {e}"))?;
    text.lines()
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_str(line).map_err(|e| format!("malformed ledger row: {e}")))
        .collect()
}

/// Informational per-document diff between the ledger committed alongside the
/// baseline and this run's own outcome rows, joined on [`OutcomeRow::sort_key`].
/// **Never a gate failure** — `check_baseline`'s bucket checks already decide
/// regressions from the aggregate counts; this exists so a reviewer can see
/// *which* documents moved without downloading two CI artifacts and diffing
/// them by hand. Returns ready-to-print lines: a one-line summary, then up to
/// 20 `key: old -> new` rows for documents whose outcome changed, sorted by
/// key.
fn diff_outcomes(committed: &[OutcomeRow], actual: &[OutcomeRow]) -> Vec<String> {
    let committed_by_key: BTreeMap<&str, &OutcomeRow> =
        committed.iter().map(|r| (r.sort_key(), r)).collect();
    let actual_by_key: BTreeMap<&str, &OutcomeRow> =
        actual.iter().map(|r| (r.sort_key(), r)).collect();

    let mut moved: Vec<(&str, &str, &str)> = Vec::new();
    let mut only_committed = 0usize;
    for (key, row) in &committed_by_key {
        match actual_by_key.get(key) {
            Some(now) if now.outcome != row.outcome => {
                moved.push((key, row.outcome.as_str(), now.outcome.as_str()))
            }
            Some(_) => {}
            None => only_committed += 1,
        }
    }
    let only_actual = actual_by_key
        .keys()
        .filter(|k| !committed_by_key.contains_key(*k))
        .count();
    moved.sort_unstable();

    let mut lines = vec![format!(
        "outcome ledger diff vs committed: {} document(s) changed outcome, {only_committed} only \
         in the committed ledger, {only_actual} only in this run",
        moved.len(),
    )];
    for (key, old, new) in moved.iter().take(20) {
        lines.push(format!("  {key}: {old} -> {new}"));
    }
    if moved.len() > 20 {
        lines.push(format!("  ... and {} more", moved.len() - 20));
    }
    lines
}

/// Most document lines any one group of [`diff_ledger_fields`] prints; the
/// per-field totals above them are never capped.
const DOC_LINE_CAP: usize = 20;

/// The per-document fields [`diff_ledger_fields`] compares, in
/// [`OutcomeRow`] schema (declaration) order — the order its totals print in.
/// `outcome` is left to [`diff_outcomes`], and `ocr_ms` has its own summary
/// line, so neither is here.
const DIFFED_FIELDS: &[&str] = &[
    "miss_reason",
    "mrz_format",
    "mrz_found",
    "mrz_checksums_valid",
    "names_exact",
    "name_error",
    "retry_variant_id",
    "retry_budget_hit",
    "retry_stop",
    "check_states",
    "retry_damaged_recovery",
    "tier1_damaged_recovery",
];

/// One field that differs on one document, already rendered for printing.
struct FieldChange {
    field: &'static str,
    /// `field old -> new`, or `miss_reason kind (detail changed)`. Built only
    /// from enumerated values (format labels, booleans, retry ids, stop kinds,
    /// name-error classes) and the miss *kind*: the step summary this reaches
    /// is public, so nothing here may carry the ledger's `miss_reason` text
    /// (it can hold both document numbers, ADR-0024 build step 0a) or any
    /// value read from a document.
    text: String,
}

fn render_optional(value: Option<&str>) -> String {
    value.unwrap_or("null").to_string()
}

fn render_optional_bool(value: Option<bool>) -> String {
    value.map_or_else(|| "null".to_string(), |b| b.to_string())
}

/// A `check_states` map as compact JSON (field names and booleans only), or
/// `null`. The map is a `BTreeMap`, so the text is sorted and stable.
fn render_check_states(value: Option<&BTreeMap<String, Option<bool>>>) -> String {
    value.map_or_else(
        || "null".to_string(),
        |states| serde_json::to_string(states).unwrap_or_else(|_| "null".to_string()),
    )
}

fn push_transition(changes: &mut Vec<FieldChange>, field: &'static str, old: String, new: String) {
    if old != new {
        changes.push(FieldChange {
            field,
            text: format!("{field} {old} -> {new}"),
        });
    }
}

/// Every [`DIFFED_FIELDS`] entry that differs between two rows of the same
/// document, in schema order.
fn field_changes(old: &OutcomeRow, new: &OutcomeRow) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    // The kind is `outcome`; the text behind it is never printed.
    if old.miss_reason != new.miss_reason {
        let text = if old.outcome == new.outcome {
            format!("miss_reason {} (detail changed)", new.outcome)
        } else {
            format!(
                "miss_reason {} -> {} (detail changed)",
                old.outcome, new.outcome
            )
        };
        changes.push(FieldChange {
            field: "miss_reason",
            text,
        });
    }
    push_transition(
        &mut changes,
        "mrz_format",
        render_optional(old.mrz_format.as_deref()),
        render_optional(new.mrz_format.as_deref()),
    );
    push_transition(
        &mut changes,
        "mrz_found",
        old.mrz_found.to_string(),
        new.mrz_found.to_string(),
    );
    push_transition(
        &mut changes,
        "mrz_checksums_valid",
        old.mrz_checksums_valid.to_string(),
        new.mrz_checksums_valid.to_string(),
    );
    push_transition(
        &mut changes,
        "names_exact",
        render_optional_bool(old.names_exact),
        render_optional_bool(new.names_exact),
    );
    push_transition(
        &mut changes,
        "name_error",
        render_optional(old.name_error.as_deref()),
        render_optional(new.name_error.as_deref()),
    );
    push_transition(
        &mut changes,
        "retry_variant_id",
        render_optional(old.retry_variant_id.as_deref()),
        render_optional(new.retry_variant_id.as_deref()),
    );
    push_transition(
        &mut changes,
        "retry_budget_hit",
        old.retry_budget_hit.to_string(),
        new.retry_budget_hit.to_string(),
    );
    push_transition(
        &mut changes,
        "retry_stop",
        render_optional(old.retry_stop.as_deref()),
        render_optional(new.retry_stop.as_deref()),
    );
    push_transition(
        &mut changes,
        "check_states",
        render_check_states(old.check_states.as_ref()),
        render_check_states(new.check_states.as_ref()),
    );
    push_transition(
        &mut changes,
        "retry_damaged_recovery",
        render_optional_bool(old.retry_damaged_recovery),
        render_optional_bool(new.retry_damaged_recovery),
    );
    push_transition(
        &mut changes,
        "tier1_damaged_recovery",
        render_optional_bool(old.tier1_damaged_recovery),
        render_optional_bool(new.tier1_damaged_recovery),
    );
    changes
}

/// A document that hit the OCR retry-pass time budget on either side. Which
/// passes finished depends on runner speed, so nothing about such a document
/// is a stable signal.
fn is_budget_limited(old: &OutcomeRow, new: &OutcomeRow) -> bool {
    old.retry_budget_hit
        || new.retry_budget_hit
        || old.retry_stop.as_deref() == Some("budget")
        || new.retry_stop.as_deref() == Some("budget")
}

/// `field count, field count, …` in [`DIFFED_FIELDS`] order, zero counts left
/// out. Counts are documents, and are never capped.
fn field_totals(docs: &[(&str, Vec<FieldChange>)]) -> String {
    DIFFED_FIELDS
        .iter()
        .filter_map(|field| {
            let count = docs
                .iter()
                .filter(|(_, changes)| changes.iter().any(|c| c.field == *field))
                .count();
            (count > 0).then(|| format!("{field} {count}"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// `  key: change; change` per document (at most [`DOC_LINE_CAP`]), then
/// `  ... and N more`.
fn push_document_lines(lines: &mut Vec<String>, docs: &[(&str, Vec<FieldChange>)]) {
    for (key, changes) in docs.iter().take(DOC_LINE_CAP) {
        let texts: Vec<&str> = changes.iter().map(|c| c.text.as_str()).collect();
        lines.push(format!("  {key}: {}", texts.join("; ")));
    }
    if docs.len() > DOC_LINE_CAP {
        lines.push(format!("  ... and {} more", docs.len() - DOC_LINE_CAP));
    }
}

/// Median of `values`, sorted in place; the integer mean of the two middle
/// values (rounded down) when the count is even. `0` for an empty slice.
fn median(values: &mut [u128]) -> u128 {
    values.sort_unstable();
    let n = values.len();
    match n {
        0 => 0,
        _ if n % 2 == 1 => values[n / 2],
        _ => (values[n / 2 - 1] + values[n / 2]) / 2,
    }
}

/// Informational per-field diff of every [`OutcomeRow`] field other than
/// `outcome` (which [`diff_outcomes`] reports) between the committed ledger
/// and this run's rows, joined on [`OutcomeRow::sort_key`] (issue #557).
/// **Never a gate failure**: like [`diff_outcomes`] the return type has no
/// failure variant. Documents present on one side only are counted by
/// [`diff_outcomes`] and skipped here.
///
/// Two groups, so a reviewer reads the stable signal separately from the
/// timing-sensitive one:
///
/// - **Deterministic** — `miss_reason` (the kind and "detail changed", never
///   the text), `mrz_format`, `mrz_found`, `mrz_checksums_valid`,
///   `names_exact`, `name_error`, `retry_variant_id`, `retry_stop`,
///   `check_states`, `retry_damaged_recovery` and `tier1_damaged_recovery`. One
///   totals line (documents per field, always complete), then one line per
///   document, at most [`DOC_LINE_CAP`].
/// - **Timing-sensitive** — `ocr_ms` (one summary line: documents that differ,
///   median |delta|, both totals) and every change on a *budget-limited*
///   document (see [`is_budget_limited`]), which includes `retry_budget_hit`
///   and `retry_stop == "budget"`.
///
/// Nothing is printed for a group with nothing in it. Documents come in
/// `sort_key` order and fields in schema order, so the output is a pure
/// function of the two inputs.
fn diff_ledger_fields(committed: &[OutcomeRow], actual: &[OutcomeRow]) -> Vec<String> {
    let committed_by_key: BTreeMap<&str, &OutcomeRow> =
        committed.iter().map(|r| (r.sort_key(), r)).collect();
    let actual_by_key: BTreeMap<&str, &OutcomeRow> =
        actual.iter().map(|r| (r.sort_key(), r)).collect();

    let mut deterministic: Vec<(&str, Vec<FieldChange>)> = Vec::new();
    let mut budget_limited: Vec<(&str, Vec<FieldChange>)> = Vec::new();
    let mut ocr_deltas: Vec<u128> = Vec::new();
    let mut common = 0usize;
    let mut ocr_old_total = 0u128;
    let mut ocr_new_total = 0u128;
    for (key, old) in &committed_by_key {
        let Some(new) = actual_by_key.get(key) else {
            continue;
        };
        common += 1;
        ocr_old_total += old.ocr_ms;
        ocr_new_total += new.ocr_ms;
        if old.ocr_ms != new.ocr_ms {
            ocr_deltas.push(old.ocr_ms.abs_diff(new.ocr_ms));
        }
        let changes = field_changes(old, new);
        if changes.is_empty() {
            continue;
        }
        if is_budget_limited(old, new) {
            budget_limited.push((*key, changes));
        } else {
            deterministic.push((*key, changes));
        }
    }

    let mut lines = Vec::new();
    if !deterministic.is_empty() {
        lines.push(format!(
            "deterministic field diff vs committed (report-only): {} document(s); {}",
            deterministic.len(),
            field_totals(&deterministic),
        ));
        push_document_lines(&mut lines, &deterministic);
    }
    if !ocr_deltas.is_empty() {
        lines.push(format!(
            "timing-sensitive field diff vs committed (report-only): ocr_ms differs on {} of \
             {common} document(s), median |delta| {} ms, total {ocr_old_total} ms -> \
             {ocr_new_total} ms",
            ocr_deltas.len(),
            median(&mut ocr_deltas),
        ));
    }
    if !budget_limited.is_empty() {
        lines.push(format!(
            "budget-limited document(s) vs committed (report-only; every change on them is \
             timing-sensitive): {} document(s); {}",
            budget_limited.len(),
            field_totals(&budget_limited),
        ));
        push_document_lines(&mut lines, &budget_limited);
    }
    lines
}

/// The heading of this binary's step summary block. The writer itself is
/// `synthpass_bench::step_summary`, shared with `synthpass-bench`'s M4 ledger
/// diff; the lines it is given must already be limited to asset ids, field
/// names, enumerated values and counts (see [`FieldChange`]), since the
/// summary is public.
const STEP_SUMMARY_TITLE: &str = "### Real-specimen per-document ledger diff (report-only)";

/// The fixed name of the text-free projection of an assert run's ledger,
/// written under the `--out` directory (see [`write_run_ledger_projection`]).
/// Deliberately not [`OUTCOMES_LEDGER_FILENAME`]: the name says it is a
/// projection, and it can never be mistaken for, or written over, the
/// committed ledger.
const RUN_LEDGER_PROJECTION_FILENAME: &str = "real-specimen-outcomes-text-free.jsonl";

/// `rows` with every `miss_reason` reduced to its kind (`outcome`); every other
/// field is kept as it is. Rows stay [`OutcomeRow`]s, so the file parses with
/// [`parse_ledger`] and diffs like a ledger.
///
/// Why: some full `miss_reason` text can carry values read from the document.
/// The audit of every `MissReason` `Display`
/// format (`crates/synthpass-bench/src/lib.rs`):
///
/// - `DocumentNumberMismatch`: a fixed string carrying no document numbers.
/// - `NoMrzFound`: the `Debug` of an `mrz::MrzError` — `BadCharacter` carries
///   the offending character and `BadDocumentCode` the first characters of
///   line 1, both OCR-read from the specimen. (`BadLength`, `BadChecksum`,
///   `LeadingFiller`, `RepeatedLine`, `IncompleteSequence`, `NotFound` carry
///   numbers and enums only.)
/// - `OcrError`: the OCR engine's error string, unaudited for values, so
///   treated as carrying them.
/// - `ChecksumFailed`: check-digit field names and a fixed suffix only;
///   `Redacted`, `NoMrzExpected`, `FalsePositiveMrz` and
///   `DocumentNumberLeadingFiller`: fixed strings.
///
/// CI never publishes real-specimen text, even as a short-lived artifact
/// (ADR-0027 Decision 5), so what an assert run uploads is this projection,
/// never the full rows. ADR-0024 build step 0a makes the committed ledger's
/// document-number mismatch reason value-free too; the projection remains
/// necessary for the other reasons above.
fn text_free_projection(rows: &[OutcomeRow]) -> Vec<OutcomeRow> {
    rows.iter()
        .map(|row| OutcomeRow {
            miss_reason: row.miss_reason.as_ref().map(|_| row.outcome.clone()),
            ..row.clone()
        })
        .collect()
}

/// Writes the [`text_free_projection`] of this run's rows next to the report
/// `--out`, so the workflow can upload the evidence behind the per-document
/// diff. Returns the path written. It is under the `--out` directory and under
/// [`RUN_LEDGER_PROJECTION_FILENAME`], so it can never overwrite the committed
/// ledger.
fn write_run_ledger_projection(
    out: &str,
    rows: &[OutcomeRow],
) -> std::io::Result<std::path::PathBuf> {
    let path = outcomes_ledger_path(out).with_file_name(RUN_LEDGER_PROJECTION_FILENAME);
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    std::fs::write(&path, ledger_bytes(&text_free_projection(rows)))?;
    Ok(path)
}

const BASELINE_NOTE: &str = "Real-specimen Tier-1 no-regression baseline for the deterministic \
    `mrz` provider. Regenerate ONLY via CI: `gh workflow run real-specimen-gate.yml -f \
    mode=write-baseline`, download the artifact, commit it. Local numbers differ from CI \
    (OCR-inference float variance), so never hand-edit the counts. See \
    knowledge/benchmarks/README.md.";

fn baseline_from_snapshot(snap: &RealSpecimenSnapshot, ts_unix: u64) -> RealSpecimenBaseline {
    RealSpecimenBaseline {
        note: BASELINE_NOTE.to_string(),
        measured_on_ci_sha: env_or("GITHUB_SHA", git_head),
        measured_date: iso_date(ts_unix),
        samples_data_sha: env_or("SAMPLES_DATA_SHA", || "unknown".to_string()),
        documents: snap.documents,
        scored: snap.scored,
        tier1_hits: snap.tier1_hits,
        tolerance: 0,
        by_miss_kind: snap.by_miss_kind.clone(),
        strict_names: snap.strict_names.clone(),
        outcomes_sha256: snap.outcomes_sha256.clone(),
        refusal_population: Some(snap.refusal_population),
    }
}

/// Compare a fresh run against the committed baseline.
///
/// `Err` lists every regression — a HIT-count drop, or a [`REGRESSION_BUCKETS`]
/// bucket growing — beyond `baseline.tolerance`. `Ok(warnings)` is a clean run:
/// the `Vec` carries corpus-shape drift (`documents` or `redacted_mrz` moved),
/// which means the numbers legitimately changed and the author should
/// regenerate the baseline, but is not a parser regression to block a merge on.
fn check_baseline(
    actual: &RealSpecimenSnapshot,
    baseline: &RealSpecimenBaseline,
) -> Result<Vec<String>, Vec<String>> {
    let tol = baseline.tolerance;
    let mut failures = Vec::new();
    let mut warnings = Vec::new();

    if actual.tier1_hits + tol < baseline.tier1_hits {
        failures.push(format!(
            "Tier-1 HIT count regressed: {} -> {} (tolerance {tol})",
            baseline.tier1_hits, actual.tier1_hits
        ));
    }
    for &bucket in REGRESSION_BUCKETS {
        let was = baseline.by_miss_kind.get(bucket).copied().unwrap_or(0);
        let now = actual.by_miss_kind.get(bucket).copied().unwrap_or(0);
        if now > was + tol {
            failures.push(format!(
                "miss bucket `{bucket}` grew: {was} -> {now} (tolerance {tol})"
            ));
        }
    }

    if actual.documents != baseline.documents {
        warnings.push(format!(
            "corpus size changed: {} -> {} documents — regenerate the baseline in this PR",
            baseline.documents, actual.documents
        ));
    }
    // The denominator moving changes the published hit *rate* even when the HIT
    // count does not, and it was the one baseline field nothing compared. That
    // blind spot is not hypothetical: reclassifying the 42 MRZ-less specimens
    // as correct refusals moved `scored` 229 -> 187 and every regression check
    // above stayed silent, because no bucket grew and no hit was lost.
    if actual.scored != baseline.scored {
        warnings.push(format!(
            "hit-rate denominator changed: {} -> {} scored — the published rate moves even with \
             the same HIT count; regenerate the baseline in this PR",
            baseline.scored, actual.scored
        ));
    }
    // Every off-denominator population drifts the same way and warns the same
    // way: a count that moved means the corpus changed, not that anything
    // regressed. Reported per bucket rather than as one total so the message
    // names which population moved.
    for &bucket in OFF_DENOMINATOR_KINDS {
        let was = baseline.by_miss_kind.get(bucket).copied().unwrap_or(0);
        let now = actual.by_miss_kind.get(bucket).copied().unwrap_or(0);
        if now != was {
            warnings.push(format!(
                "{bucket} count changed: {was} -> {now} (off-denominator) — regenerate the \
                 baseline if the corpus changed"
            ));
        }
    }

    // ADR-0013 strict-name counts: report-only, so a move here is never a
    // failure, whatever direction it moves in — see `StrictNamesBaseline`'s
    // doc for why gating on it is a separate, undecided question.
    match (&baseline.strict_names, &actual.strict_names) {
        (Some(was), Some(now)) if was != now => {
            warnings.push(format!(
                "strict-name counts changed: strict_hits {} -> {}, name_scorable_documents \
                 {} -> {}, name_scorable_hits {} -> {} (report-only; ADR-0013)",
                was.strict_hits,
                now.strict_hits,
                was.name_scorable_documents,
                now.name_scorable_documents,
                was.name_scorable_hits,
                now.name_scorable_hits,
            ));
        }
        (Some(_), None) => {
            warnings.push(
                "strict-name counts were in the baseline but this run has none (report-only; \
                 ADR-0013)"
                    .to_string(),
            );
        }
        (None, Some(_)) => {
            warnings.push(
                "strict-name counts appeared in this run but are not in the baseline \
                 (report-only; ADR-0013)"
                    .to_string(),
            );
        }
        _ => {}
    }

    if failures.is_empty() {
        Ok(warnings)
    } else {
        failures.extend(warnings);
        Err(failures)
    }
}

/// Writes the outcome ledger at [`outcomes_ledger_path`]`(path)`, then the
/// baseline JSON at `path` with `outcomes_sha256` set to that ledger's hash —
/// the two files are written as a pair specifically so `outcomes_sha256`
/// always describes the ledger actually sitting next to it. Used by both
/// `--write-baseline` and the `--assert-baseline` first-run bootstrap.
/// Returns the written baseline, for the caller's summary line.
fn write_baseline_and_ledger(
    path: &str,
    snap: &RealSpecimenSnapshot,
    rows: &[OutcomeRow],
    ts_unix: u64,
) -> RealSpecimenBaseline {
    let bytes = ledger_bytes(rows);
    let ledger_path = outcomes_ledger_path(path);
    if let Some(parent) = ledger_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create outcome ledger directory");
        }
    }
    std::fs::write(&ledger_path, &bytes)
        .unwrap_or_else(|e| panic!("write {}: {e}", ledger_path.display()));
    let mut baseline = baseline_from_snapshot(snap, ts_unix);
    baseline.outcomes_sha256 = Some(sha256_hex(&bytes));
    write_json_pretty(path, &baseline);
    baseline
}

/// Why `--write-baseline`/`--assert-baseline` must refuse to run: `None`
/// when `arms` is [`synthpass_ocr::OcrArms::DEFAULT`] and every `SYNTHPASS_MRZ_*`
/// arm is at its default (class sweep `off`, line-1 select `on`, repeated-line
/// refusal `off`, date digits `off`),
/// `Some(message)` otherwise. A pure function of its arguments
/// alone (no env reads, no I/O) so it is directly unit-testable without setting
/// process environment variables — see `knowledge/benchmarks/README.md`'s
/// maintenance contract for why a baseline may only describe the default
/// provider configuration: every other arm is a measurement in progress, and
/// committing a baseline against one would make its own A/B look like a
/// regression against itself the moment the env var is unset again.
///
/// `class_sweep`, `line1_select`, `refuse_repeated_line` and `date_digits` are
/// the arm names `synthpass_die::class_sweep_arm`,
/// `synthpass_die::line1_select_arm`, `synthpass_die::refuse_repeated_line_arm`
/// and `synthpass_die::date_digits_arm` return. Each changes what Tier 1 reads,
/// so a baseline written under any of them when it is not at its default would
/// describe a configuration nobody runs by default (#574, #579). The line-1
/// selector's default is `on` since its promotion
/// (`knowledge/benchmarks/line1-selection-ab-2026-09-29.md`), so `off` is the
/// arm that is refused there; the date-digits rule (#579) is `off` until it is
/// promoted, so `on` is refused there. `control` is refused as well: it is a
/// placebo, but a baseline is a claim about the default.
fn refuse_non_default_baseline(
    arms: &synthpass_ocr::OcrArms,
    class_sweep: &str,
    line1_select: &str,
    refuse_repeated_line: &str,
    date_digits: &str,
) -> Option<String> {
    if arms.is_default()
        && class_sweep == "off"
        && line1_select == "on"
        && refuse_repeated_line == "off"
        && date_digits == "off"
    {
        return None;
    }
    Some(format!(
        "❌ --write-baseline/--assert-baseline require every SYNTHPASS_OCR_* arm at its default \
         (texture=on, order=default, rotate=default, skew=default, chargrid=off) and \
         SYNTHPASS_MRZ_CLASS_SWEEP at off, SYNTHPASS_MRZ_LINE1_SELECT at on and \
         SYNTHPASS_MRZ_REFUSE_REPEATED_LINE and SYNTHPASS_MRZ_DATE_DIGITS at off, their \
         defaults — a baseline is only valid for the default provider configuration (see knowledge/benchmarks/README.md). \
         This run measured: texture={}, order={}, rotate={}, skew={}, chargrid={}, \
         mrz_class_sweep={class_sweep}, mrz_line1_select={line1_select}, \
         mrz_refuse_repeated_line={refuse_repeated_line}, \n         mrz_date_digits={date_digits}.",
        arms.texture, arms.order, arms.rotate, arms.skew, arms.chargrid,
    ))
}

/// `--write-baseline` / `--assert-baseline`, run after the report JSON is on
/// disk. May [`std::process::exit`] non-zero on a regression.
fn run_baseline_step(
    parsed: &Args,
    snapshot: Option<(RealSpecimenSnapshot, Vec<OutcomeRow>)>,
    ts_unix: u64,
) {
    if let Some(msg) = refuse_non_default_baseline(
        &synthpass_ocr::OcrArms::from_env(),
        synthpass_die::class_sweep_arm().0,
        synthpass_die::line1_select_arm().0,
        synthpass_die::refuse_repeated_line_arm().0,
        synthpass_die::date_digits_arm().0,
    ) {
        eprintln!("{msg}");
        std::process::exit(1);
    }

    let Some((snapshot, rows)) = snapshot else {
        eprintln!(
            "❌ --write-baseline/--assert-baseline need the `mrz` provider in the run — it was \
             not registered (check the catalog wiring)"
        );
        std::process::exit(1);
    };

    if let Some(path) = parsed.write_baseline.as_deref() {
        let baseline = write_baseline_and_ledger(path, &snapshot, &rows, ts_unix);
        println!(
            "baseline written to {path} (tier1_hits={}, scored={}, {} miss kind(s)); outcome \
             ledger written to {}",
            baseline.tier1_hits,
            baseline.scored,
            baseline.by_miss_kind.len(),
            outcomes_ledger_path(path).display(),
        );
        return;
    }

    let path = parsed
        .assert_baseline
        .as_deref()
        .expect("run_baseline_step is only reached with one of the two flags set");

    if !std::path::Path::new(path).exists() {
        write_baseline_and_ledger(path, &snapshot, &rows, ts_unix);
        println!(
            "ℹ no baseline at {path} yet — wrote the current snapshot (and its outcome ledger) \
             and passing. Review and commit it (CI owns the committed value)."
        );
        return;
    }

    let text =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read baseline {path}: {e}"));
    let baseline: RealSpecimenBaseline =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse baseline {path}: {e}"));

    print_baseline_table(&baseline, &snapshot);

    // A text-free projection of this run's own ledger, under the `--out`
    // directory (never next to the committed baseline), so the workflow can
    // upload the evidence behind the diff below — see `text_free_projection`
    // for why it is not the full ledger. Written before anything here can exit
    // non-zero. A failure to write it is a warning: the evidence is not a gate
    // condition.
    match write_run_ledger_projection(&parsed.out, &rows) {
        Ok(projection) => println!(
            "text-free projection of this run's outcome ledger written to {}",
            projection.display()
        ),
        Err(e) => eprintln!("⚠ could not write the text-free ledger projection: {e}"),
    }

    // Outcome-ledger integrity + an informational per-document diff, only
    // when a committed ledger actually sits next to the baseline (absent for
    // one written before this change). A sha mismatch is a gate failure in
    // its own right — see `verify_ledger_sha`'s doc — independent of whatever
    // `check_baseline` below finds; the diff itself never fails anything.
    let mut ledger_failures: Vec<String> = Vec::new();
    let ledger_path = outcomes_ledger_path(path);
    if ledger_path.exists() {
        let committed_bytes = std::fs::read(&ledger_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", ledger_path.display()));
        if let Some(expected) = &baseline.outcomes_sha256 {
            if let Err(msg) = verify_ledger_sha(&committed_bytes, expected) {
                ledger_failures.push(msg);
            }
        }
        match parse_ledger(&committed_bytes) {
            Ok(committed_rows) => {
                let mut diff_lines = diff_outcomes(&committed_rows, &rows);
                diff_lines.extend(diff_ledger_fields(&committed_rows, &rows));
                for line in &diff_lines {
                    println!("{line}");
                }
                // The same lines go to the job's step summary. Report-only:
                // a failed write warns and never changes the gate result.
                if let Err(e) = synthpass_bench::step_summary::append_step_summary(
                    synthpass_bench::step_summary::step_summary_path().as_deref(),
                    STEP_SUMMARY_TITLE,
                    &diff_lines,
                ) {
                    eprintln!("⚠ could not write the step summary: {e}");
                }
            }
            Err(e) => eprintln!(
                "⚠ could not parse the committed outcome ledger at {}: {e} — skipping the \
                 per-document diff",
                ledger_path.display()
            ),
        }
    }

    let check_result = check_baseline(&snapshot, &baseline);
    if ledger_failures.is_empty() {
        match check_result {
            Ok(warnings) => {
                for w in &warnings {
                    eprintln!("⚠ {w}");
                }
                println!("✅ real-specimen Tier-1 (mrz): no regression vs baseline");
            }
            Err(problems) => {
                for p in &problems {
                    eprintln!("❌ {p}");
                }
                eprintln!(
                    "\nIf this change is intentional (parser improvement, corpus edit), \
                     regenerate the baseline in this PR — see knowledge/benchmarks/README.md."
                );
                std::process::exit(1);
            }
        }
    } else {
        if let Err(problems) = &check_result {
            for p in problems {
                eprintln!("❌ {p}");
            }
        }
        for p in &ledger_failures {
            eprintln!("❌ {p}");
        }
        eprintln!(
            "\nThe committed outcome ledger no longer matches the baseline it was written with — \
             regenerate both together (`--write-baseline`), never edit either by hand."
        );
        std::process::exit(1);
    }
}

fn print_baseline_table(baseline: &RealSpecimenBaseline, actual: &RealSpecimenSnapshot) {
    println!("\nreal-specimen Tier-1 baseline check (mrz provider):");
    let row = |label: &str, was: usize, now: usize| {
        println!(
            "  {label:<26} {was:>5} -> {now:<5} ({:+})",
            now as i64 - was as i64
        );
    };
    row("tier1_hits", baseline.tier1_hits, actual.tier1_hits);
    row("scored (denominator)", baseline.scored, actual.scored);
    row("documents", baseline.documents, actual.documents);
    // Stated explicitly so `documents - scored` is not left as arithmetic for
    // the reader: it is the count of specimens that cannot yield a hit at all.
    row(
        "  of which unattackable",
        baseline.documents - baseline.scored,
        actual.off_denominator(),
    );
    // `refusal_population` restates the row above as the named baseline
    // field it backs (`documents - scored`) — printed only when the
    // committed baseline actually carries it, since an older one never
    // recorded it and printing a fabricated `0` would misstate "not
    // measured" as "measured zero".
    if let Some(baseline_refusal) = baseline.refusal_population {
        row(
            "refusal_population",
            baseline_refusal,
            actual.refusal_population,
        );
    }
    // Printed explicitly, even when both sides are exactly `0` — the row
    // `false_positive_mrz` exists specifically so a reader can see "zero
    // false accepts across N documents that could have produced one" stated,
    // not merely absent because nothing happened to trip the general loop
    // below. Excluded from that loop's `kinds` list so it is never printed
    // twice.
    row(
        "false_positive_mrz",
        baseline
            .by_miss_kind
            .get("false_positive_mrz")
            .copied()
            .unwrap_or(0),
        actual
            .by_miss_kind
            .get("false_positive_mrz")
            .copied()
            .unwrap_or(0),
    );
    let mut kinds: Vec<&str> = baseline
        .by_miss_kind
        .keys()
        .chain(actual.by_miss_kind.keys())
        .map(String::as_str)
        .filter(|k| *k != "false_positive_mrz")
        .collect();
    kinds.sort_unstable();
    kinds.dedup();
    for k in kinds {
        row(
            k,
            baseline.by_miss_kind.get(k).copied().unwrap_or(0),
            actual.by_miss_kind.get(k).copied().unwrap_or(0),
        );
    }
    // ADR-0013 strict-name counts, report-only: printed whenever either side
    // has them, with `—` standing in for a side that has none, so a reader
    // can see "not measured" rather than a misleading `0`.
    if baseline.strict_names.is_some() || actual.strict_names.is_some() {
        let opt_row = |label: &str, was: Option<usize>, now: Option<usize>| {
            let was_s = was.map_or_else(|| "—".to_string(), |v| v.to_string());
            let now_s = now.map_or_else(|| "—".to_string(), |v| v.to_string());
            println!("  {label:<26} {was_s:>5} -> {now_s:<5} (report-only; ADR-0013)");
        };
        opt_row(
            "strict_hits",
            baseline.strict_names.as_ref().map(|s| s.strict_hits),
            actual.strict_names.as_ref().map(|s| s.strict_hits),
        );
        opt_row(
            "name_scorable_documents",
            baseline
                .strict_names
                .as_ref()
                .map(|s| s.name_scorable_documents),
            actual
                .strict_names
                .as_ref()
                .map(|s| s.name_scorable_documents),
        );
        opt_row(
            "name_scorable_hits",
            baseline.strict_names.as_ref().map(|s| s.name_scorable_hits),
            actual.strict_names.as_ref().map(|s| s.name_scorable_hits),
        );
    }
    println!(
        "  (baseline measured {} on {})",
        baseline.measured_date, baseline.measured_on_ci_sha
    );
    if let Some(sha) = &baseline.outcomes_sha256 {
        println!("  (outcome ledger sha256 {}…)", &sha[..sha.len().min(12)]);
    }
}

fn write_json_pretty<T: Serialize>(path: &str, value: &T) {
    let json = serde_json::to_string_pretty(value).expect("serialize");
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create baseline directory");
        }
    }
    std::fs::write(path, format!("{json}\n")).unwrap_or_else(|e| panic!("write {path}: {e}"));
}

fn env_or(var: &str, fallback: impl FnOnce() -> String) -> String {
    std::env::var(var)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(fallback)
}

fn git_head() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

#[derive(Serialize)]
struct OcrDumpRunManifest<'a> {
    started_unix_seconds: u64,
    git_commit: String,
    working_tree_dirty: bool,
    flags: &'a [String],
    pivot_yy: u32,
    ocr_arms: BTreeMap<String, String>,
    /// The `SYNTHPASS_MRZ_*` arms this process ran under, as it resolved them
    /// (#574). Always this process's own, also in a replay: the capture's OCR
    /// arms are copied above because the text came from that OCR, but the MRZ
    /// arms describe the read a replay makes.
    mrz_arms: MrzArms,
    corpus_manifest: &'static str,
    corpus_manifest_sha256: Option<String>,
    documents_loaded: usize,
    labelled_loaded: usize,
    raw_ocr_stage: &'static str,
    source_hash_stage: &'static str,
    outcome_ledger: &'static str,
    /// Present only in a replay's manifest: which capture the text came from.
    /// Skipped otherwise, so a live run's manifest is byte-identical to what it
    /// was before replay existed (its file name is a hash of these bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    replay_of: Option<ReplayOf<'a>>,
}

/// The `SYNTHPASS_MRZ_*` arms as the run manifest spells them: each name is
/// what the binary resolved, never the variable's raw value (an unrecognised
/// value falls back to `off`).
#[derive(Serialize, Debug, PartialEq, Eq)]
struct MrzArms {
    class_sweep: &'static str,
    line1_select: &'static str,
    refuse_repeated_line: &'static str,
    date_digits: &'static str,
}

impl MrzArms {
    fn from_env() -> Self {
        Self {
            class_sweep: synthpass_die::class_sweep_arm().0,
            line1_select: synthpass_die::line1_select_arm().0,
            refuse_repeated_line: synthpass_die::refuse_repeated_line_arm().0,
            date_digits: synthpass_die::date_digits_arm().0,
        }
    }
}

/// What a replay's manifest says about the capture it replayed (ADR-0024,
/// amendment 3, Decision 3): the capture's run-manifest file name and the
/// SHA-256 of that file's bytes. Neither holds document text.
#[derive(Serialize)]
struct ReplayOf<'a> {
    run_manifest: &'a str,
    sha256: &'a str,
}

/// The `SYNTHPASS_MRZ_*` arms as a map, from [`MrzArms`] itself: the run manifest and the
/// archive header both spell them, and neither keeps its own list of names.
fn mrz_arms_map() -> BTreeMap<String, String> {
    serde_json::to_value(MrzArms::from_env())
        .ok()
        .and_then(|value| serde_json::from_value(value).ok())
        .unwrap_or_default()
}

/// The five `SYNTHPASS_OCR_*` arms as the run manifest and the replay's arm
/// check spell them.
fn ocr_arms_map(arms: &synthpass_ocr::OcrArms) -> BTreeMap<String, String> {
    [
        ("texture", arms.texture),
        ("order", arms.order),
        ("rotate", arms.rotate),
        ("skew", arms.skew),
        ("chargrid", arms.chargrid),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value.to_string()))
    .collect()
}

/// SHA-256 of `samples/corpus.jsonl`, `None` when it cannot be read.
fn corpus_manifest_sha256(root: &Path) -> Option<String> {
    std::fs::read(root.join("samples/corpus.jsonl"))
        .ok()
        .map(|bytes| sha256_hex(&bytes))
}

/// A capture, as far as its manifest tells a replay: which file it is, and the
/// OCR arms its text was read under.
#[derive(Debug, PartialEq)]
struct ReplaySource {
    /// The capture's run-manifest file name, as its rows name it.
    run_manifest: String,
    /// SHA-256 of that file's bytes.
    run_manifest_sha256: String,
    /// The capture's `ocr_arms`, copied into the replay's manifest: the text
    /// came from that OCR.
    ocr_arms: BTreeMap<String, String>,
}

/// The parts of a capture's run manifest a replay reads. Everything else in it
/// (flags, commit, timing) is the capture's own record and is not interpreted.
#[derive(Deserialize)]
struct CaptureManifest {
    ocr_arms: Option<BTreeMap<String, String>>,
    corpus_manifest_sha256: Option<String>,
}

/// The first 12 characters of a hash, for a message.
fn short_hash(hash: &str) -> &str {
    hash.get(..12).unwrap_or(hash)
}

/// The one run manifest a capture's rows name (`OcrPassesRow::run_manifest`).
/// `Err` when there is none, when the rows name more than one, or when the name
/// is not a plain file name (it is joined onto the capture directory, so a path
/// is refused rather than followed).
fn capture_manifest_name(
    rows: &[synthpass_bench::ocr_passes::OcrPassesRow],
) -> Result<&str, String> {
    let mut names = rows.iter().map(|row| row.run_manifest.as_deref());
    let first = names
        .next()
        .flatten()
        .ok_or_else(|| "the capture's rows name no run manifest".to_string())?;
    if names.any(|name| name != Some(first)) {
        return Err(
            "the capture's rows name more than one run manifest, or some name none: a capture is \
             one run"
                .to_string(),
        );
    }
    if Path::new(first).file_name() != Some(std::ffi::OsStr::new(first)) {
        return Err("the capture's run manifest name is not a plain file name".to_string());
    }
    Ok(first)
}

/// Checks a capture's run manifest against this process and returns what a
/// replay records about it. `bytes` is the manifest file's content.
///
/// Refuses, naming the problem:
/// - a manifest that does not parse, or records no `ocr_arms` or no
///   `corpus_manifest_sha256`: a replay cannot say what it replays;
/// - a `corpus_manifest_sha256` that is not `current_corpus`: the capture ran on
///   another `samples/corpus.jsonl`;
/// - `ocr_arms` that are not `current_arms`. The report's per-provider
///   `ocr_arms` comes from this process's environment, and it must be true of the
///   text being replayed, so the replaying process runs with the capture's
///   `SYNTHPASS_OCR_*` values.
fn replay_source_from(
    name: &str,
    bytes: &[u8],
    current_corpus: Option<&str>,
    current_arms: &BTreeMap<String, String>,
) -> Result<ReplaySource, String> {
    let manifest: CaptureManifest = serde_json::from_slice(bytes).map_err(|e| {
        format!(
            "the capture's run manifest {name} does not parse ({:?})",
            e.classify()
        )
    })?;
    let Some(captured_corpus) = manifest.corpus_manifest_sha256 else {
        return Err(format!(
            "the capture's run manifest {name} records no corpus_manifest_sha256, so a replay \
             cannot tell which corpus it was measured on"
        ));
    };
    let Some(current_corpus) = current_corpus else {
        return Err(
            "cannot read samples/corpus.jsonl to compare it with the capture's".to_string(),
        );
    };
    if captured_corpus != current_corpus {
        return Err(format!(
            "the capture was measured against a different samples/corpus.jsonl (capture {}, \
             current {}): a replay needs the corpus the capture ran on",
            short_hash(&captured_corpus),
            short_hash(current_corpus),
        ));
    }
    let Some(ocr_arms) = manifest.ocr_arms else {
        return Err(format!(
            "the capture's run manifest {name} records no ocr_arms, so a replay cannot say which \
             OCR the text came from"
        ));
    };
    if ocr_arms != *current_arms {
        return Err(format!(
            "the capture read its text under ocr_arms {ocr_arms:?}, and this process's \
             SYNTHPASS_OCR_* environment gives {current_arms:?}: run the replay with the \
             capture's values, so the report's ocr_arms is true of the text it replays"
        ));
    }
    Ok(ReplaySource {
        run_manifest: name.to_string(),
        run_manifest_sha256: sha256_hex(bytes),
        ocr_arms,
    })
}

/// Reads the run manifest `capture`'s rows name from `dir` and checks it
/// ([`replay_source_from`]).
fn replay_source(
    dir: &Path,
    rows: &[synthpass_bench::ocr_passes::OcrPassesRow],
    current_corpus: Option<&str>,
    current_arms: &BTreeMap<String, String>,
) -> Result<ReplaySource, String> {
    let name = capture_manifest_name(rows)?;
    let path = dir.join(name);
    let bytes = std::fs::read(&path).map_err(|e| {
        format!(
            "cannot read the capture's run manifest {}: {e}",
            path.display()
        )
    })?;
    replay_source_from(name, &bytes, current_corpus, current_arms)
}

/// Everything `--replay-ocr-passes DIR` checks before it writes a file: reads the
/// capture, checks its run manifest against this corpus and these arms, and
/// checks its rows against the loaded `specimens` (coverage, image bytes and the
/// keys a replay needs). Returns the capture and what to record about it.
fn load_replay(
    dir: &Path,
    root: &Path,
    specimens: &[synthpass_bench::RealSpecimenDoc],
) -> Result<(synthpass_bench::ocr_passes::PassesFile, ReplaySource), String> {
    let capture = synthpass_bench::ocr_passes::read_rows(dir)?;
    let source = replay_source(
        dir,
        &capture.rows,
        corpus_manifest_sha256(root).as_deref(),
        &ocr_arms_map(&synthpass_ocr::OcrArms::from_env()),
    )?;
    synthpass_bench::provider_bench::check_replay(specimens, &capture)?;
    Ok((capture, source))
}

/// Whether two paths name the same existing directory. `false` when either does
/// not exist (a replay's `--out` directory may not yet).
fn same_directory(a: &Path, b: &Path) -> bool {
    matches!(
        (std::fs::canonicalize(a), std::fs::canonicalize(b)),
        (Ok(a), Ok(b)) if a == b
    )
}

/// What the report's `model_paths` say when no OCR model was loaded.
const NO_MODEL_LOADED: &str = "(replay: no OCR model loaded)";

/// The directory the dumps next to `--out` go in: its parent, or the current
/// directory when `--out` has no directory part.
fn out_dir(out: &str) -> std::path::PathBuf {
    std::path::Path::new(out)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

/// Whether this run asks to write any document-OCR dump next to `--out`.
/// Every such flag goes through the shared destination guard before a model
/// loads or a document is read.
fn writes_document_ocr_dump(parsed: &Args) -> bool {
    parsed.dump_ocr || parsed.dump_ocr_hits || parsed.dump_ocr_passes
}

/// The `ocr_arms` a run manifest records: this process's, or a replay's capture's, because the
/// text came from that OCR. The archive header records the same map.
fn manifest_ocr_arms(replay: Option<&ReplaySource>) -> BTreeMap<String, String> {
    match replay {
        Some(source) => source.ocr_arms.clone(),
        None => ocr_arms_map(&synthpass_ocr::OcrArms::from_env()),
    }
}

/// The `model_paths` the report records: the paths the OCR models were loaded from, or a note
/// that a replay loaded none. The archive header records the same value.
fn model_paths_report(replay: bool, detection: &Path, recognition: &Path) -> ModelPathsReport {
    if replay {
        ModelPathsReport {
            detection: NO_MODEL_LOADED.to_string(),
            recognition: NO_MODEL_LOADED.to_string(),
        }
    } else {
        ModelPathsReport::resolve(detection, recognition)
    }
}

/// Persist a content-addressed run description next to the raw OCR dump.
/// A row's `run_manifest` is a filename relative to its JSONL, so repeated
/// runs in one output directory cannot silently re-point old rows.
///
/// For a replay, `replay` names the capture: the manifest records it as
/// `replay_of` and copies the capture's `ocr_arms`, because the text came from
/// that OCR. With `None` the manifest is a live run's, byte for byte what it was
/// before replay existed.
fn write_ocr_run_manifest(
    root: &Path,
    dir: &Path,
    flags: &[String],
    documents_loaded: usize,
    labelled_loaded: usize,
    replay: Option<&ReplaySource>,
) -> Result<String, String> {
    let ocr_arms = manifest_ocr_arms(replay);
    let git = archive::git_state(root)?;
    let manifest = OcrDumpRunManifest {
        started_unix_seconds: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("system clock before Unix epoch: {e}"))?
            .as_secs(),
        git_commit: git.commit,
        working_tree_dirty: git.dirty,
        flags,
        pivot_yy: synthpass_die::mrz_parse_options().pivot_yy,
        ocr_arms,
        mrz_arms: MrzArms::from_env(),
        corpus_manifest: "samples/corpus.jsonl",
        corpus_manifest_sha256: corpus_manifest_sha256(root),
        documents_loaded,
        labelled_loaded,
        raw_ocr_stage: "provider input after any OCR retries (may join attempts)",
        source_hash_stage: "original encoded image bytes",
        outcome_ledger: "provider-bench-ocr-outcomes.jsonl",
        replay_of: replay.map(|source| ReplayOf {
            run_manifest: &source.run_manifest,
            sha256: &source.run_manifest_sha256,
        }),
    };
    let body = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    let name = format!("provider-bench-ocr-run-{}.json", sha256_hex(&body));
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join(&name), body).map_err(|e| e.to_string())?;
    std::fs::write(
        dir.join("provider-bench-ocr-current-run.txt"),
        name.as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    Ok(name)
}

/// What a run does about the per-document archive (ADR-0024), decided before any model loads.
#[derive(Debug, PartialEq)]
enum ArchivePlan {
    /// No archive, and nothing to say: `--no-archive` or `SYNTHPASS_BENCH_ARCHIVE=off`.
    Off,
    /// No archive, and one `warning: archive: ...` line to print.
    Warn(String),
    /// Write the archive under this root, which has passed the tree check.
    Root(PathBuf),
}

/// The archive plan for `parsed`, from the process's facts injected as arguments so every rule
/// is testable without the environment or git: `variable` is `SYNTHPASS_BENCH_ARCHIVE`, `repo`
/// the working tree, `cwd` the current directory.
///
/// A `--include-private` run is not refused, it writes no archive at all, with one warning: the
/// private track may only be archived as text-free records (ADR-0024, Decision 7), which are a
/// later step, and the archive may never change an exit code (Decision 1). Every other problem
/// (a root git would stage, a git that cannot say where its directory is) is a warning too.
fn archive_plan(
    parsed: &Args,
    variable: Option<&str>,
    repo: &Path,
    cwd: &Path,
    git_common_dir: impl FnOnce() -> Result<PathBuf, String>,
    is_ignored: impl Fn(&Path) -> Result<bool, String>,
) -> ArchivePlan {
    // A run that turned the archive off says nothing more about it; only a run that asked for
    // one (by default) is told why it will not get one.
    if parsed.include_private && archive::archive_requested(parsed.no_archive, variable) {
        return ArchivePlan::Warn(
            "warning: archive: --include-private writes no archive: the private track is only \
             archived as text-free records, which are a later step (ADR-0024, Decision 7)"
                .to_string(),
        );
    }
    match archive::resolve_archive_root(
        parsed.no_archive,
        variable,
        repo,
        cwd,
        git_common_dir,
        is_ignored,
    ) {
        Ok(None) => ArchivePlan::Off,
        Ok(Some(root)) => ArchivePlan::Root(root),
        Err(e) => ArchivePlan::Warn(format!("warning: archive: {e}")),
    }
}

/// The plan for this process: `SYNTHPASS_BENCH_ARCHIVE`, the working directory, and git.
fn plan_archive(parsed: &Args, root: &Path) -> ArchivePlan {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let variable = match std::env::var_os(archive::ARCHIVE_ENV).map(std::ffi::OsString::into_string)
    {
        None => None,
        Some(Ok(value)) => Some(value),
        Some(Err(_)) => {
            return ArchivePlan::Warn(format!(
                "warning: archive: {} is not valid Unicode, so no archive is written",
                archive::ARCHIVE_ENV
            ));
        }
    };
    archive_plan(
        parsed,
        variable.as_deref(),
        root,
        &cwd,
        || archive::git_common_dir(root),
        |relative| synthpass_bench::ocr_passes::git_ignores(root, relative),
    )
}

/// What one run tells the archive about itself, beyond the process's own facts.
struct ArchiveRun<'a> {
    argv: &'a [String],
    documents_loaded: usize,
    labelled_loaded: usize,
    providers: Vec<String>,
    replay: Option<&'a ReplaySource>,
    model_paths: ModelPathsReport,
}

/// The header of this run's archive files (ADR-0024, Decision 4). The arms come from the run
/// manifest's own code ([`manifest_ocr_arms`], [`mrz_arms_map`]), so the two cannot disagree,
/// and the retry budget is recorded here rather than left to the manifest.
fn archive_header(parsed: &Args, root: &Path, run: &ArchiveRun<'_>) -> RunHeader {
    let git = archive::git_state(root).ok();
    let binary = std::env::current_exe().ok();
    let (max_passes, max_seconds) = synthpass_ocr::effective_retry_budget();
    let real = parsed.real_specimens;
    RunHeader {
        kind: "run",
        schema: archive::SCHEMA,
        run_id: String::new(),
        binary_name: "provider-bench".to_string(),
        started_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as u64),
        pid: std::process::id(),
        source: archive::source_from(std::env::var("GITHUB_ACTIONS").ok().as_deref()),
        machine: archive::Machine::detect(),
        // The file name only: the path says whose machine this is, and the SHA-256
        // below identifies the binary. `argv` and `model_paths` keep their paths, which
        // the Decision 8 publisher is to refuse or redact.
        binary: binary.as_ref().and_then(|b| b.file_name()).map_or_else(
            || "provider-bench".to_string(),
            |name| name.to_string_lossy().into_owned(),
        ),
        binary_sha256: binary
            .as_ref()
            .and_then(|b| std::fs::read(b).ok())
            .map(|bytes| archive::sha256_hex(&bytes)),
        git_commit: git.as_ref().map(|g| g.commit.clone()),
        working_tree_dirty: git.as_ref().map(|g| g.dirty),
        argv: run.argv.to_vec(),
        scope: archive::Scope {
            corpus: if real {
                "real-specimens"
            } else {
                "synthetic-corpus"
            },
            format: parsed.format.map(|f| f.as_str().to_string()),
            limit: parsed.limit,
            document_type: (!real).then(|| {
                parsed
                    .document_type
                    .unwrap_or(DocumentType::TD3)
                    .as_str()
                    .to_string()
            }),
            profile: (!real).then(|| parsed.profile.as_str().to_string()),
            seed_start: (!real).then_some(parsed.seed),
            count: run.documents_loaded as u64,
        },
        tracks: archive::TrackFlags {
            private: parsed.include_private,
            local: parsed.include_local,
            covers: parsed.include_covers,
        },
        samples_data_sha: archive::samples_data_sha_from(
            std::env::var("SAMPLES_DATA_SHA").ok().as_deref(),
        ),
        corpus_manifest_sha256: real.then(|| corpus_manifest_sha256(root)).flatten(),
        documents_loaded: run.documents_loaded,
        labelled_loaded: run.labelled_loaded,
        providers: run.providers.clone(),
        ocr_arms: manifest_ocr_arms(run.replay),
        retry_budget: run.replay.is_none().then_some(archive::RetryBudget {
            max_passes,
            max_seconds,
        }),
        mrz_arms: mrz_arms_map(),
        pivot_yy: synthpass_die::mrz_parse_options().pivot_yy,
        model_paths: run.model_paths.clone(),
        replay_of: run.replay.map(|source| archive::ReplayOfRecord {
            run_manifest: source.run_manifest.clone(),
            sha256: source.run_manifest_sha256.clone(),
        }),
        env: archive::process_env(),
    }
    .with_run_id()
}

/// The ids of the providers in `catalog`, in registration order.
fn catalog_provider_ids(catalog: &ProviderCatalog) -> Vec<String> {
    catalog
        .readers()
        .iter()
        .map(|reader| reader.id().as_str().to_string())
        .collect()
}

/// The archive for this run: on under the planned root, else off.
fn start_archive(plan: &ArchivePlan, parsed: &Args, root: &Path, run: &ArchiveRun<'_>) -> Archive {
    match plan {
        ArchivePlan::Root(dir) => Archive::start(dir, &archive_header(parsed, root, run)),
        ArchivePlan::Off | ArchivePlan::Warn(_) => Archive::disabled(),
    }
}

/// `YYYY-MM-DD` (UTC) from a Unix timestamp — a trimmed civil-from-days
/// (Howard Hinnant's algorithm). One date string in a report does not justify a
/// `chrono`/`time` dependency.
fn iso_date(unix_secs: u64) -> String {
    let (y, m, d) = archive::civil_date(unix_secs);
    format!("{y:04}-{m:02}-{d:02}")
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // No arguments is never a real invocation on purpose: every default
    // (`Args::default()`) starts a 20-document synthetic run that also loads
    // and runs the ~1 GB `llm` provider (issue #510) — expensive, and never
    // what a bare `provider-bench` was meant to ask for. `2` is this
    // workspace's usage-error convention (see e.g. `examples/check_sample.rs`),
    // distinct from `1` below for an argument that was actually given and
    // rejected.
    if requires_usage(&args) {
        usage();
        std::process::exit(2);
    }
    let parsed = match parse_args(&args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ {e}");
            usage();
            std::process::exit(1);
        }
    };

    if parsed.measure_memory && !cfg!(feature = "measure-memory") {
        eprintln!(
            "❌ --measure-memory requires building with `--features measure-memory` \
             (declared_resident_bytes is still reported without it)"
        );
        std::process::exit(1);
    }

    // On by default for an interactive run, off when stderr is redirected —
    // a captured log or a CI step (`bench-charts.yml` runs this in a pwsh
    // step) stays exactly as clean as it is today, while a developer watching
    // a 1h37m real-specimen pass sees it advance. `--progress` forces it back
    // on for the redirected case.
    let show_progress = parsed.progress || std::io::stderr().is_terminal();

    let root = repo_root();
    // Refused before any model loads or document is read: these dumps hold
    // document OCR, and a destination git would stage is not a place for them.
    if writes_document_ocr_dump(&parsed) {
        let dir = out_dir(&parsed.out);
        let cwd = std::env::current_dir().unwrap_or_else(|e| {
            eprintln!("❌ cannot determine the working directory: {e}");
            std::process::exit(1);
        });
        if let Err(e) =
            synthpass_bench::ocr_passes::check_passes_destination(&root, &cwd, &dir, |relative| {
                synthpass_bench::ocr_passes::git_ignores(&root, relative)
            })
        {
            eprintln!("❌ {e}");
            std::process::exit(1);
        }
    }
    // The archive is decided here too, before any model loads: a root git would stage is not
    // written to, and the reason is one warning, never a refusal (ADR-0024, Decision 1).
    let archive_plan = plan_archive(&parsed, &root);
    if let ArchivePlan::Warn(warning) = &archive_plan {
        eprintln!("{warning}");
    }
    // A replay runs no OCR and loads no model. It also writes a run manifest and
    // an outcome ledger next to `--out`, and the capture's own are named the same:
    // `--out` in the capture directory would replace them.
    let replay_dir = parsed.replay_ocr_passes.as_deref().map(Path::new);
    if let Some(dir) = replay_dir {
        if same_directory(&out_dir(&parsed.out), dir) {
            eprintln!(
                "❌ --out is in the capture directory {}: a replay writes a run manifest and an \
                 outcome ledger there, replacing the capture's own",
                dir.display()
            );
            std::process::exit(1);
        }
    }
    // `SYNTHPASS_OCR_MODEL_DIR` if set, otherwise this binary's own
    // build-tree repo root (today's behaviour, unchanged) — issue #541.
    let model_dir = synthpass_bench::resolve_model_dir(&root, |k| std::env::var_os(k));
    let detection_path = model_dir.join("text-detection.rten");
    let recognition_path = model_dir.join("text-recognition.rten");
    let ocr: Option<NativeOcr> = if replay_dir.is_some() {
        None
    } else {
        eprintln!(
            "OCR models: detection={} recognition={}",
            detection_path.display(),
            recognition_path.display()
        );
        Some(NativeOcr::load(&detection_path, &recognition_path).expect(
            "failed to load OCR models — run from the repo root, or set SYNTHPASS_OCR_MODEL_DIR",
        ))
    };

    // The full M7 catalog is `mrz` + the Tier-2 `LlmFieldReader`, and the only
    // way to reach the latter's registered instance is through a `Pipeline`
    // (`LlmFieldReader` is `pub(crate)` there). `--mrz-only` skips all of it —
    // no second OCR engine, no `NativeInferer`, no GGUF ever touched — and
    // builds a one-reader catalog directly instead, leaving the deterministic
    // OCR + Tier-1 pass the per-PR `real-specimen-gate.yml` CI job needs
    // without the LLM pass that turns a real-specimen run into hours.
    let pipeline;
    let deterministic_only;
    let catalog: &ProviderCatalog = if parsed.mrz_only {
        deterministic_only = mrz_only_catalog();
        &deterministic_only
    } else {
        // A second OCR engine handle purely to satisfy `Pipeline::new`'s
        // constructor — never actually invoked. Every document in this harness
        // is OCR'd once via `ocr` above and shared across every reader.
        let pipeline_ocr: Box<dyn OcrEngine> = Box::new(RustOcrEngine::new(&root));
        let infer: Box<dyn InferBackend> = Box::new(NativeInferer::new(model_path(), n_ctx()));
        pipeline = Pipeline::new(pipeline_ocr, infer);
        pipeline.catalog()
    };

    // Real specimens (ground truth optional, MRZ-less fronts included) vs.
    // the synthetic corpus (ground truth always present) — see
    // `synthpass_bench::provider_bench`'s top doc comment for why both
    // sources feed the exact same reader loop and reporting shape.
    let (reports, source, profile, count, seed_start) = if parsed.real_specimens {
        let tracks = synthpass_bench::OptInTracks {
            private: parsed.include_private,
            local: parsed.include_local,
            covers: parsed.include_covers,
        };
        let mut specimens = load_real_specimens(&root.join("samples"), tracks);
        if specimens.is_empty() {
            eprintln!(
                "❌ no image files found under samples/ — is this being run from the repo root?"
            );
            std::process::exit(1);
        }
        if let Some(class) = parsed.format {
            specimens.retain(|s| s.class == class);
            if specimens.is_empty() {
                eprintln!(
                    "❌ no samples/ specimens classified as {} — nothing to run",
                    class.as_str()
                );
                std::process::exit(1);
            }
        }
        if let Some(n) = parsed.limit {
            specimens = subsample(specimens, n);
        }
        let labelled = specimens.iter().filter(|s| s.labels.is_some()).count();
        eprintln!(
            "loaded {} real specimens ({labelled} with samples/ocr_fixtures/ ground truth, {} \
             unlabelled)",
            specimens.len(),
            specimens.len() - labelled,
        );
        // A replay is checked against the loaded corpus before anything is
        // written: a refused replay leaves no manifest or ledger behind.
        let replay = replay_dir
            .map(|dir| load_replay(dir, &root, &specimens))
            .transpose()
            .unwrap_or_else(|e| {
                eprintln!("❌ {e}");
                std::process::exit(1);
            });
        // `--dump-ocr` and `--dump-ocr-passes` write their JSONL next to the
        // `--out` report; an --out with no directory part means the current
        // directory. A replay always writes the run manifest, because it names
        // the capture the text came from.
        let dump_dir =
            (parsed.dump_ocr || parsed.dump_ocr_hits || parsed.dump_ocr_passes || replay.is_some())
                .then(|| out_dir(&parsed.out));
        if let Some(dir) = dump_dir.as_deref() {
            let source = replay.as_ref().map(|(_, source)| source);
            write_ocr_run_manifest(&root, dir, &args, specimens.len(), labelled, source)
                .unwrap_or_else(|e| {
                    eprintln!("❌ cannot write OCR run manifest: {e}");
                    std::process::exit(1);
                });
        }
        let archive = start_archive(
            &archive_plan,
            &parsed,
            &root,
            &ArchiveRun {
                argv: &args,
                documents_loaded: specimens.len(),
                labelled_loaded: labelled,
                providers: catalog_provider_ids(catalog),
                replay: replay.as_ref().map(|(_, source)| source),
                model_paths: model_paths_report(
                    replay.is_some(),
                    &detection_path,
                    &recognition_path,
                ),
            },
        );
        let dumps = RealDumpOptions {
            ocr_dir: dump_dir
                .as_deref()
                .filter(|_| parsed.dump_ocr || parsed.dump_ocr_hits),
            ocr_hits: parsed.dump_ocr_hits,
            ocr_passes_dir: dump_dir.as_deref().filter(|_| parsed.dump_ocr_passes),
            archive: archive.is_on().then_some(&archive),
        };
        let outcome = match (&replay, ocr.as_ref()) {
            (Some((capture, source)), _) => {
                eprintln!(
                    "replaying {} documents from {} (no OCR)",
                    specimens.len(),
                    source.run_manifest
                );
                run_provider_bench_replay(
                    catalog,
                    &specimens,
                    capture,
                    parsed.measure_memory,
                    &dumps,
                    show_progress,
                )
                .await
            }
            (None, Some(ocr)) => {
                run_provider_bench_real_with_options(
                    catalog,
                    ocr,
                    &specimens,
                    parsed.measure_memory,
                    &dumps,
                    show_progress,
                )
                .await
            }
            (None, None) => Err("no OCR models were loaded for a live run".to_string()),
        };
        let reports = outcome.unwrap_or_else(|e| {
            eprintln!("❌ {e}");
            std::process::exit(1);
        });
        // The reader loops have returned. The archive is closed now, before the report, the
        // ledger and the baseline step, which may `exit` and skip any `Drop`.
        archive.finish();
        (
            reports,
            "real-specimens",
            None,
            specimens.len() as u64,
            None,
        )
    } else {
        let corpus = generate_corpus(
            parsed.profile,
            parsed.seed,
            parsed.count,
            parsed.document_type.unwrap_or(DocumentType::TD3),
        );
        let Some(ocr) = ocr.as_ref() else {
            // `--replay-ocr-passes` needs `--real-specimens`, which `parse_args` enforces.
            eprintln!("❌ the synthetic corpus needs the OCR models, which this run did not load");
            std::process::exit(1);
        };
        let archive = start_archive(
            &archive_plan,
            &parsed,
            &root,
            &ArchiveRun {
                argv: &args,
                documents_loaded: corpus.len(),
                labelled_loaded: corpus.len(),
                providers: catalog_provider_ids(catalog),
                replay: None,
                model_paths: model_paths_report(false, &detection_path, &recognition_path),
            },
        );
        let reports = run_provider_bench(
            catalog,
            ocr,
            &corpus,
            parsed.measure_memory,
            show_progress,
            archive.is_on().then_some(&archive),
        )
        .await;
        archive.finish();
        (
            reports,
            "synthetic-corpus",
            Some(parsed.profile.as_str()),
            parsed.count,
            Some(parsed.seed),
        )
    };

    // The committed baseline ledger may describe a different OCR run. Keep
    // this run's exact outcomes beside its dump so the classifier never has
    // to guess that historical outcomes still match the current pass.
    // The run manifest names this ledger, so any dump flag that writes the
    // manifest writes it too.
    if parsed.real_specimens
        && (parsed.dump_ocr
            || parsed.dump_ocr_hits
            || parsed.dump_ocr_passes
            || replay_dir.is_some())
    {
        let mrz = reports
            .iter()
            .find(|r| r.provider_id == "mrz")
            .unwrap_or_else(|| {
                eprintln!("❌ OCR mechanism dump requires the mrz provider");
                std::process::exit(1);
            });
        let dir = std::path::Path::new(&parsed.out)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        let path = dir.join("provider-bench-ocr-outcomes.jsonl");
        std::fs::write(&path, ledger_bytes(&build_outcome_rows(mrz))).unwrap_or_else(|e| {
            eprintln!("❌ cannot write OCR outcome ledger {}: {e}", path.display());
            std::process::exit(1);
        });
    }

    for r in &reports {
        let cer = |mean: Option<f64>| {
            mean.map(|v| format!("{v:.3}"))
                .unwrap_or_else(|| "n/a".to_string())
        };
        let unsupported = match &r.unsupported_assertion {
            UnsupportedAssertion::Computed {
                overall,
                with_mrz_anchor,
                without_mrz_anchor,
            } => {
                // The split is the point of this line, not a detail: a single
                // blended figure hides that the anchored and unanchored
                // populations fail in different *kinds*, not just degrees.
                // Each half prints its own document count, because they are
                // never the same size and a bare rate would conceal that.
                //
                // "no claims" rather than "0.0%" when nothing was asserted.
                // The first real run made the difference matter: over the 22
                // documents with no MRZ, the deterministic reader asserted
                // nothing at all, and printing that as `0.0%` read as a
                // measured, perfect score in the same column where a genuine
                // 0.0 means "answered plenty, all of it supported". Declining
                // to answer is the correct behaviour there and deserves to be
                // legible as such, not disguised as a good grade.
                let rate = |b: &AssertionBucket| match b.rate {
                    Some(r) => format!("{:.1}%", r * 100.0),
                    None => "n/a (no claims)".to_string(),
                };
                let half = |label: &str, b: &Option<AssertionBucket>| match b {
                    Some(b) => format!(", {label} {} ({} docs)", rate(b), b.documents),
                    None => String::new(),
                };
                format!(
                    "{}{}{}",
                    rate(overall),
                    half("with-MRZ", with_mrz_anchor),
                    half("no-MRZ", without_mrz_anchor),
                )
            }
            UnsupportedAssertion::NotApplicable { reason } => format!("n/a ({reason})"),
        };
        let tier1_hit_rate: &Tier1HitRate = &r.tier1_hit_rate;
        println!(
            "{}: {} docs ({} labelled), Tier-1 hit rate {tier1_hit_rate}, mean {} ms, \
             unsupported-assertion rate {unsupported}",
            r.provider_id,
            r.documents,
            r.accuracy.labelled_documents,
            r.speed.mean.as_millis(),
        );

        // Field accuracy over three populations, each with its size (#564).
        // Read quality and end-to-end are the two to quote; the all-labelled
        // figure is kept for continuity with the bench history and mixes in
        // misses and non-conforming specimens. See
        // `synthpass_bench::provider_bench::AccuracyStats`'s doc.
        let accepted = &r.accuracy.accepted_reads;
        let scored = &r.accuracy.scored;
        println!(
            "  field accuracy, accepted reads ({} docs): field match {}, mean CER {}",
            accepted.documents,
            format_percentage(accepted.field_match_rate),
            cer(accepted.mean_cer),
        );
        println!(
            "  field accuracy, scored end-to-end ({} docs): field match {}, mean CER {}",
            scored.documents,
            format_percentage(scored.field_match_rate),
            cer(scored.mean_cer),
        );
        println!(
            "  field accuracy, all labelled incl. non-conforming ({} docs): field match {}, \
             mean CER {}",
            r.accuracy.labelled_documents,
            format_percentage(r.accuracy.field_match_rate),
            cer(r.accuracy.mean_cer),
        );
        let in_population = |population: &PopulationAccuracy, field: &str| {
            population
                .per_field
                .iter()
                .find(|entry| entry.field == field)
                .map(|entry| (entry.mean_cer, entry.documents))
                .unwrap_or((None, 0))
        };
        println!("  per-field mean CER (docs): accepted reads | scored | all labelled");
        for (field, all_mean, all_documents) in &r.accuracy.per_field_cer {
            let (accepted_mean, accepted_documents) = in_population(accepted, field);
            let (scored_mean, scored_documents) = in_population(scored, field);
            if accepted_documents + scored_documents + all_documents == 0 {
                continue;
            }
            println!(
                "    {field}: {} ({accepted_documents}) | {} ({scored_documents}) | {} \
                 ({all_documents})",
                cer(accepted_mean),
                cer(scored_mean),
                cer(*all_mean),
            );
        }

        // Two name-accuracy rates over two different denominators — no ICAO
        // check digit covers `surname`/`given_names`, so `tier1_hit_rate`
        // alone says nothing about them. See
        // `synthpass_bench::provider_bench::StrictNameHitRate`'s doc for
        // exactly what each divides by and the "not computed" reasons this
        // can print.
        match &r.strict_tier1_hit_rate {
            StrictNameHitRate::Computed {
                strict_hits,
                name_scorable_documents,
                name_scorable_hits,
                strict_tier1_hit_rate,
                names_exact_among_hits,
            } => {
                println!(
                    "    strict Tier-1 hit rate: {:.1}% ({strict_hits}/{name_scorable_documents} \
                     name-scorable documents in the scored Tier-1 population are both a hit and \
                     read exactly)",
                    strict_tier1_hit_rate * 100.0
                );
                println!(
                    "    names exact among hits: {} ({strict_hits}/{name_scorable_hits} \
                     name-scorable Tier-1 hits read both names exactly)",
                    format_percentage(*names_exact_among_hits)
                );
            }
            StrictNameHitRate::NotApplicable { reason } => {
                println!("    strict Tier-1 hit rate: n/a ({reason})");
                println!("    names exact among hits: n/a ({reason})");
            }
        }

        // Where the time actually went. `mean` above times `reader.read` alone,
        // microseconds for the deterministic provider, while the OCR pass each
        // document shares across readers is where a real-specimen run spends its
        // tens of minutes (ADR-0010, step 5).
        let mut ocr_times: Vec<_> = r.documents_detail.iter().map(|d| d.ocr_elapsed).collect();
        if !ocr_times.is_empty() {
            ocr_times.sort();
            let total: std::time::Duration = ocr_times.iter().sum();
            println!(
                "    ocr: total {:.1} min, mean {} ms, p50 {} ms, p95 {} ms, max {} ms",
                total.as_secs_f64() / 60.0,
                synthpass_bench::provider_bench::mean_duration(&ocr_times).as_millis(),
                synthpass_bench::provider_bench::percentile_duration(&ocr_times, 0.50).as_millis(),
                synthpass_bench::provider_bench::percentile_duration(&ocr_times, 0.95).as_millis(),
                ocr_times.last().copied().unwrap_or_default().as_millis(),
            );
        }

        // Per-format document counts — the M6 plan's "report the unparsed
        // population separately; a specimen with no MRZ is not a
        // TD-anything" applied to this harness: a distribution, keyed on the
        // same `mrz_format` the Tier-1 hit rate above and `miss_reason`
        // below share.
        let mut by_format: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for d in &r.documents_detail {
            *by_format
                .entry(d.mrz_format.unwrap_or("unresolved"))
                .or_default() += 1;
        }
        if !by_format.is_empty() {
            let counts: Vec<String> = by_format
                .iter()
                .map(|(fmt, n)| format!("{fmt}={n}"))
                .collect();
            println!("    by format: {}", counts.join(", "));
        }

        // Misses by kind — mirrors `synthpass-bench`'s own "misses by kind"
        // summary, so a real-specimen run answers "what kind of failure
        // dominates this track" from stdout, not just `--verbose` +
        // manual JSON inspection. Skipped entirely when there is nothing to
        // report, same as the format breakdown above.
        let mut by_miss_kind: std::collections::BTreeMap<&str, usize> =
            std::collections::BTreeMap::new();
        for d in &r.documents_detail {
            if let Some(reason) = &d.miss_reason {
                *by_miss_kind.entry(miss_kind(reason)).or_default() += 1;
            }
        }
        if !by_miss_kind.is_empty() {
            let counts: Vec<String> = by_miss_kind
                .iter()
                .map(|(kind, n)| format!("{kind}={n}"))
                .collect();
            println!("    misses by kind: {}", counts.join(", "));

            // The three populations that cannot yield a hit, named individually
            // so the reader can see what the denominator actually excludes
            // rather than having to reconstruct it from the table above.
            for (kind, why) in [
                (
                    "no_mrz_expected",
                    "carry no MRZ at all and correctly yielded none — a correct refusal",
                ),
                ("redacted_mrz", "have a physically redacted zone"),
                (
                    "checksum_failed_specimen",
                    "have a printed zone that fails its own ICAO check digits",
                ),
            ] {
                if let Some(n) = by_miss_kind.get(kind) {
                    println!(
                        "    ({kind}: {n} specimen(s) that {why} — excluded from the Tier-1 \
                         hit-rate denominator, not scored as a miss)"
                    );
                }
            }
            // The mirror of `false_positive_mrz`, and it costs us a hit rather
            // than inventing one: a specimen tagged `*_redacted_mrz` whose zone
            // reads checksum-valid is evidence the *zone* is not what was
            // redacted. Passing every ICAO check digit by chance is vanishingly
            // unlikely — the same argument `corpus_manifest.rs`'s
            // `a_no_mrz_specimen_never_records_a_checksum_valid_read` makes for
            // the `no_mrz` tag, and the same shape as the Monaco file that was
            // reported as a Tier-1 false positive and turned out to be a label
            // error. Scoring it out is right if the tag is right, and silently
            // costs a genuine hit if it is not, so the harness says so either
            // way instead of leaving it to whoever next reads the JSON.
            let mislabelled_redactions: Vec<&str> = r
                .documents_detail
                .iter()
                .filter(|d| {
                    d.mrz_checksums_valid
                        && d.miss_reason.as_ref().map(miss_kind) == Some("redacted_mrz")
                })
                .map(|d| d.name.as_str())
                .collect();
            if !mislabelled_redactions.is_empty() {
                println!(
                    "    ⚠ {} specimen(s) tagged redacted read a checksum-VALID MRZ, so the zone \
                     is probably not what was redacted — they are scored out and may be costing \
                     a genuine hit: {}",
                    mislabelled_redactions.len(),
                    mislabelled_redactions.join(", ")
                );
            }

            // Issue #443, report-only: a `checksum_failed_specimen` document
            // is filed there because its *printed* zone fails its own check
            // digits, whatever OCR returned — so a checksum-valid read on one
            // can never surface as `false_positive_mrz`, and the outcome
            // ledger (bucket only) never sees it. A read that verifies on a
            // zone that cannot verify was built by the parser. Named here and
            // counted in the JSON (`checksum_valid_on_failed_specimen`); the
            // failing step, with its re-bless, is a later change.
            let manufactured = synthpass_bench::report::checksum_valid_reads_on_failed_specimens(
                &r.documents_detail,
            );
            if !manufactured.is_empty() {
                println!(
                    "    ⚠ {} checksum_failed_specimen document(s) read a checksum-VALID MRZ off a \
                     printed zone that fails its own check digits — the read was manufactured, \
                     not read (issue #443; report-only, not gated): {}",
                    manufactured.len(),
                    manufactured.join(", ")
                );
            }

            // Loud on purpose, and phrased as a defect rather than a count: a
            // checksum-valid MRZ off a document that has none is either a
            // hallucinated record or a mislabelled corpus file, and both need
            // someone to look. It reads as a hit in every other summary.
            if let Some(n) = by_miss_kind.get("false_positive_mrz") {
                println!(
                    "    ⚠ FALSE POSITIVES: {n} specimen(s) tagged as carrying no MRZ returned a \
                     checksum-valid one. Either the read is invented or the file is mislabelled — \
                     see `--verbose` for which, and knowledge/benchmarks/README.md."
                );
            }

            // Sub-breakdown of checksum_failed specifically — see
            // `synthpass-bench.rs`'s identical summary and
            // `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 1. Tallies
            // occurrences, not documents, so this need not sum to
            // `by_miss_kind["checksum_failed"]`.
            let mut by_failing_field: std::collections::BTreeMap<&str, usize> =
                std::collections::BTreeMap::new();
            for d in &r.documents_detail {
                if let Some(MissReason::ChecksumFailed { check_states, .. }) = &d.miss_reason {
                    for (field, state) in check_states {
                        if *state == Some(false) {
                            *by_failing_field.entry(field).or_default() += 1;
                        }
                    }
                }
            }
            if !by_failing_field.is_empty() {
                let counts: Vec<String> = by_failing_field
                    .iter()
                    .map(|(field, n)| format!("{field}={n}"))
                    .collect();
                println!(
                    "    checksum_failed, by failing field: {}",
                    counts.join(", ")
                );
            }
        }

        if parsed.verbose {
            // Worst first. An aggregate tells you a provider made 61
            // unsupported assertions; this tells you which documents produced
            // them, which is the difference between a number and something
            // you can go and fix. Documents where nothing was asserted are
            // skipped — on the MRZ-less half that is most of the deterministic
            // reader's rows, and listing 22 empty lines would bury the ones
            // that matter.
            let mut rows: Vec<_> = r
                .documents_detail
                .iter()
                .filter(|d| !d.read_ok || d.assertions_total > 0)
                .collect();
            rows.sort_by(|a, b| {
                b.assertions_unsupported
                    .cmp(&a.assertions_unsupported)
                    .then_with(|| a.name.cmp(&b.name))
            });
            for d in rows {
                let anchor = if d.mrz_found { "mrz" } else { "no-mrz" };
                let format = d.mrz_format.unwrap_or("?");
                if !d.read_ok {
                    println!("    {:<6}  {:<5}  {}  READ FAILED", anchor, format, d.name);
                    continue;
                }
                let fields = if d.unsupported_fields.is_empty() {
                    String::new()
                } else {
                    format!("  [{}]", d.unsupported_fields.join(", "))
                };
                println!(
                    "    {:<6}  {:<5}  {:<52}  {}/{} unsupported{}",
                    anchor, format, d.name, d.assertions_unsupported, d.assertions_total, fields,
                );
            }
        }
    }

    // Captured before `reports` is consumed into the report below. `None` when
    // no baseline flag was passed, or `Some(None)` when one was but the `mrz`
    // provider is somehow absent from the run (handled in `run_baseline_step`).
    // The outcome rows are built from the same `mrz` report the snapshot
    // reads, borrowed here rather than taken so `reports` is still whole for
    // the `Report` JSON below.
    let baseline_snapshot = (parsed.write_baseline.is_some() || parsed.assert_baseline.is_some())
        .then(|| {
            reports
                .iter()
                .find(|r| r.provider_id == "mrz")
                .and_then(|mrz| {
                    RealSpecimenSnapshot::from_reports(&reports)
                        .map(|s| (s, build_outcome_rows(mrz)))
                })
        });

    let timestamp_unix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let report = Report {
        timestamp_unix,
        source,
        profile,
        format: parsed.format.map(SpecimenClass::as_str),
        count,
        seed_start,
        mrz_class_sweep_arm: synthpass_die::class_sweep_arm().0,
        mrz_line1_select_arm: synthpass_die::line1_select_arm().0,
        mrz_refuse_repeated_line_arm: synthpass_die::refuse_repeated_line_arm().0,
        mrz_date_digits_arm: synthpass_die::date_digits_arm().0,
        model_paths: model_paths_report(replay_dir.is_some(), &detection_path, &recognition_path),
        providers: reports.into_iter().map(ProviderRow::from).collect(),
    };
    let json = serde_json::to_string_pretty(&report).expect("serialize report");
    if let Some(parent) = std::path::Path::new(&parsed.out).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).expect("create report directory");
        }
    }
    std::fs::write(&parsed.out, json).expect("write report");
    println!(
        "mrz class-sweep arm measured: {}",
        synthpass_die::class_sweep_arm().0
    );
    println!(
        "mrz line-1 select arm measured: {}",
        synthpass_die::line1_select_arm().0
    );
    println!(
        "mrz repeated-line refusal arm measured: {}",
        synthpass_die::refuse_repeated_line_arm().0
    );
    println!(
        "mrz date-digits arm measured: {}",
        synthpass_die::date_digits_arm().0
    );
    println!("report written to {}", parsed.out);
    if let Some(dir) = replay_dir {
        println!("replay of the capture in {}", dir.display());
    }

    // Baseline write / assert — the per-PR real-specimen no-regression gate.
    // Runs last, after the report JSON is safely on disk, and may
    // `std::process::exit(1)` on a regression.
    if let Some(snapshot) = baseline_snapshot {
        run_baseline_step(&parsed, snapshot, timestamp_unix);
    }
}

fn repo_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("crates/synthpass-bench is two levels below the repo root")
        .to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undefined_rates_render_as_na_but_measured_zero_is_zero_percent() {
        assert_eq!(format_percentage(None), "n/a");
        assert_eq!(format_percentage(Some(0.0)), "0.0%");
        assert_eq!(format_percentage(Some(0.5)), "50.0%");
        assert_eq!(Tier1HitRate::Computed(0.0).to_string(), "0.0%");
        assert_eq!(
            Tier1HitRate::NotApplicable {
                reason: "denominator is zero"
            }
            .to_string(),
            "n/a (denominator is zero)"
        );
    }

    use std::time::Duration;
    use synthpass_bench::provider_bench::{
        AccuracyStats, CapabilitySnapshot, DocumentDetail, SpeedStats,
    };
    use synthpass_bench::report::StrictNamesBaseline;

    #[test]
    fn refuse_non_default_baseline_allows_the_default_arms() {
        assert_eq!(
            refuse_non_default_baseline(
                &synthpass_ocr::OcrArms::DEFAULT,
                "off",
                "on",
                "off",
                "off"
            ),
            None
        );
    }

    #[test]
    fn refuse_non_default_baseline_rejects_any_single_moved_knob() {
        let mut arms = synthpass_ocr::OcrArms::DEFAULT;
        arms.chargrid = "on";
        let msg =
            refuse_non_default_baseline(&arms, "off", "on", "off", "off").expect("must refuse");
        assert!(msg.contains("chargrid=on"), "message: {msg}");

        let mut arms = synthpass_ocr::OcrArms::DEFAULT;
        arms.texture = "off";
        assert!(refuse_non_default_baseline(&arms, "off", "on", "off", "off").is_some());
    }

    /// #574: the class-sweep and line-1 `SYNTHPASS_MRZ_*` arms change what Tier
    /// 1 reads, so a baseline may not be written or asserted with either off
    /// its default: class sweep `off`, line-1 select `on`. `control` is refused
    /// on both (the refusal arm has its own test below).
    #[test]
    fn refuse_non_default_baseline_rejects_either_mrz_arm() {
        let arms = synthpass_ocr::OcrArms::DEFAULT;
        for value in ["on", "control"] {
            let msg =
                refuse_non_default_baseline(&arms, value, "on", "off", "off").expect("class sweep");
            assert!(msg.contains(&format!("mrz_class_sweep={value}")), "{msg}");
        }
        for value in ["off", "control"] {
            let msg = refuse_non_default_baseline(&arms, "off", value, "off", "off")
                .expect("line-1 select");
            assert!(msg.contains(&format!("mrz_line1_select={value}")), "{msg}");
        }
        // Both moved at once is refused once, and the message names both.
        let msg = refuse_non_default_baseline(&arms, "on", "off", "off", "off").expect("both");
        assert!(msg.contains("mrz_class_sweep=on") && msg.contains("mrz_line1_select=off"));
    }

    /// The pinned pairs from the #574 promotion: `("off", "on")` is accepted,
    /// `("off", "off")` and `("off", "control")` are refused.
    #[test]
    fn refuse_non_default_baseline_accepts_only_the_line1_default() {
        let arms = synthpass_ocr::OcrArms::DEFAULT;
        assert_eq!(
            refuse_non_default_baseline(&arms, "off", "on", "off", "off"),
            None
        );
        assert!(refuse_non_default_baseline(&arms, "off", "off", "off", "off").is_some());
        assert!(refuse_non_default_baseline(&arms, "off", "control", "off", "off").is_some());
    }

    /// #579: the repeated-line refusal arm changes what Tier 1 reads, so a
    /// baseline may not be written or asserted with it on; `control` is
    /// refused too, and the message names it.
    #[test]
    fn refuse_non_default_baseline_rejects_the_repeated_line_refusal() {
        let arms = synthpass_ocr::OcrArms::DEFAULT;
        assert_eq!(
            refuse_non_default_baseline(&arms, "off", "on", "off", "off"),
            None
        );
        for value in ["on", "control"] {
            let msg =
                refuse_non_default_baseline(&arms, "off", "on", value, "off").expect("refusal arm");
            assert!(
                msg.contains(&format!("mrz_refuse_repeated_line={value}")),
                "{msg}"
            );
            assert!(msg.contains("SYNTHPASS_MRZ_REFUSE_REPEATED_LINE"), "{msg}");
        }
    }

    /// #579: the date-digits arm changes what Tier 1 reads too, so a baseline
    /// may not be written or asserted with it on or on `control`, and the message
    /// names it.
    #[test]
    fn refuse_non_default_baseline_rejects_the_date_digits_arm() {
        let arms = synthpass_ocr::OcrArms::DEFAULT;
        for value in ["on", "control"] {
            let msg =
                refuse_non_default_baseline(&arms, "off", "on", "off", value).expect("date digits");
            assert!(msg.contains(&format!("mrz_date_digits={value}")), "{msg}");
            assert!(msg.contains("SYNTHPASS_MRZ_DATE_DIGITS at off"), "{msg}");
        }
        assert_eq!(
            refuse_non_default_baseline(&arms, "off", "on", "off", "off"),
            None
        );
    }

    #[test]
    fn subsample_returns_everything_when_the_limit_is_not_binding() {
        let all: Vec<u32> = (0..10).collect();
        assert_eq!(subsample(all.clone(), 10), all);
        assert_eq!(subsample(all.clone(), 99), all);
    }

    #[test]
    fn subsample_takes_a_spread_not_a_prefix() {
        // The property that matters: `samples/` is ordered by document class,
        // so a prefix would be all one class. A stride over 137 documents must
        // reach the end of the corpus, not stop a third of the way in.
        let all: Vec<u32> = (0..137).collect();
        let picked = subsample(all, 50);
        assert_eq!(picked.len(), 50);
        assert_eq!(picked[0], 0, "the first document is always included");
        assert!(
            *picked.last().expect("non-empty") > 100,
            "a stride must reach the far end of the corpus; got {:?}",
            picked.last()
        );
        let mut sorted = picked.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 50, "no duplicates");
        assert_eq!(sorted, picked, "corpus order is preserved");
    }

    #[test]
    fn subsample_is_deterministic() {
        // Two runs that disagree because they drew different documents cannot
        // be compared against each other, which is the whole point of a
        // benchmark subset.
        let all: Vec<u32> = (0..137).collect();
        assert_eq!(subsample(all.clone(), 50), subsample(all, 50));
    }

    #[test]
    fn subsample_handles_a_limit_of_one() {
        assert_eq!(subsample((0..137).collect::<Vec<u32>>(), 1), vec![0]);
    }

    #[test]
    fn format_parses_alongside_real_specimens() {
        let args: Vec<String> = ["--real-specimens", "--format", "passport"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).expect("valid combination");
        assert_eq!(parsed.format, Some(SpecimenClass::Passport));
        assert!(parsed.real_specimens);
    }

    /// Issue #510: no arguments must never fall through to `Args::default()`
    /// — that default silently starts a 20-document synthetic run including
    /// the `llm` provider, which no bare invocation was ever meant to ask
    /// for. Any argument at all, valid or not, is `parse_args`'s job instead.
    #[test]
    fn no_arguments_requires_usage() {
        assert!(requires_usage(&[]));
        let one_arg: Vec<String> = vec!["--real-specimens".to_string()];
        assert!(!requires_usage(&one_arg));
        let bogus_arg: Vec<String> = vec!["--not-a-real-flag".to_string()];
        assert!(!requires_usage(&bogus_arg));
    }

    #[test]
    fn format_without_real_specimens_is_rejected() {
        let args: Vec<String> = ["--format", "passport"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            parse_args(&args).is_err(),
            "--format only makes sense scoping --real-specimens"
        );
    }

    #[test]
    fn dump_ocr_parses_alongside_real_specimens() {
        let args: Vec<String> = ["--real-specimens", "--dump-ocr"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).expect("valid combination");
        assert!(parsed.dump_ocr);
    }

    #[test]
    fn dump_ocr_hits_is_opt_in_and_requires_real_specimens() {
        let args: Vec<String> = ["--real-specimens", "--dump-ocr-hits"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).expect("valid combination");
        assert!(parsed.dump_ocr_hits);

        let bad: Vec<String> = ["--dump-ocr-hits"].iter().map(|s| s.to_string()).collect();
        assert!(parse_args(&bad).is_err());
    }

    #[test]
    fn raw_ocr_dump_rejects_the_private_track() {
        let args: Vec<String> = ["--real-specimens", "--include-private", "--dump-ocr-hits"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args)
            .err()
            .expect("private OCR dump must be rejected")
            .contains("samples/private/"));
    }

    fn args_of(flags: &[&str]) -> Vec<String> {
        flags.iter().map(|s| s.to_string()).collect()
    }

    /// #574 / ADR-0024 amendment 1: the pass readings are document OCR, so the
    /// flag is real-specimen-only like the other OCR dumps, and off by default.
    #[test]
    fn dump_ocr_passes_needs_real_specimens_and_is_off_by_default() {
        assert!(
            !parse_args(&args_of(&["--real-specimens"]))
                .expect("parses")
                .dump_ocr_passes
        );
        let parsed = parse_args(&args_of(&["--real-specimens", "--dump-ocr-passes"]))
            .expect("valid combination");
        assert!(parsed.dump_ocr_passes);
        assert!(
            !parsed.dump_ocr && !parsed.dump_ocr_hits,
            "the pass dump is its own flag, not a widening of --dump-ocr"
        );
        let err = parse_args(&args_of(&["--dump-ocr-passes"]))
            .err()
            .expect("refused without --real-specimens");
        assert!(err.contains("--real-specimens"), "{err}");
    }

    /// Same message and rule as `--dump-ocr`'s: a private specimen never gets
    /// an OCR dump, whichever dump flag asks.
    #[test]
    fn dump_ocr_passes_rejects_the_private_track() {
        let err = parse_args(&args_of(&[
            "--real-specimens",
            "--include-private",
            "--dump-ocr-passes",
        ]))
        .err()
        .expect("private pass dump must be rejected");
        assert_eq!(err, "OCR dumps cannot include samples/private/");
    }

    /// `--include-local` follows `--dump-ocr`'s rule, which allows it (the
    /// local track is gitignored).
    #[test]
    fn dump_ocr_passes_allows_the_local_track() {
        let parsed = parse_args(&args_of(&[
            "--real-specimens",
            "--include-local",
            "--dump-ocr-passes",
        ]))
        .expect("local track is allowed");
        assert!(parsed.include_local && parsed.dump_ocr_passes);
    }

    #[test]
    fn out_dir_is_the_parent_of_out_or_the_current_directory() {
        assert_eq!(
            out_dir("artifacts/provider-bench-report.json"),
            std::path::PathBuf::from("artifacts")
        );
        assert_eq!(out_dir("report.json"), std::path::PathBuf::from("."));
    }

    #[test]
    fn every_document_ocr_dump_flag_uses_the_destination_guard() {
        assert!(!writes_document_ocr_dump(
            &parse_args(&args_of(&["--real-specimens"])).expect("parses")
        ));
        for flag in ["--dump-ocr", "--dump-ocr-hits", "--dump-ocr-passes"] {
            let parsed = parse_args(&args_of(&["--real-specimens", flag])).expect("parses");
            assert!(writes_document_ocr_dump(&parsed), "{flag}");
        }
    }

    #[test]
    fn ocr_run_manifest_pins_flags_pivot_and_corpus_hash() {
        let dir = std::env::temp_dir().join(format!(
            "provider-bench-ocr-run-test-{}",
            std::process::id()
        ));
        let flags = vec!["--real-specimens".to_string(), "--dump-ocr".to_string()];
        let name = write_ocr_run_manifest(&repo_root(), &dir, &flags, 2, 1, None)
            .expect("write local run manifest");
        let body = std::fs::read(dir.join(&name)).expect("manifest exists");
        assert_eq!(
            name,
            format!("provider-bench-ocr-run-{}.json", sha256_hex(&body))
        );
        let manifest: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(manifest["flags"], serde_json::json!(flags));
        assert_eq!(manifest["documents_loaded"], 2);
        assert_eq!(manifest["labelled_loaded"], 1);
        assert!(manifest["started_unix_seconds"].as_u64().is_some());
        assert_eq!(
            manifest["pivot_yy"],
            synthpass_die::mrz_parse_options().pivot_yy
        );
        assert_eq!(manifest["git_commit"], git_head());
        // #574, #579: every MRZ arm, as this process resolved them.
        assert_eq!(
            manifest["mrz_arms"],
            serde_json::json!({
                "class_sweep": synthpass_die::class_sweep_arm().0,
                "line1_select": synthpass_die::line1_select_arm().0,
                "refuse_repeated_line": synthpass_die::refuse_repeated_line_arm().0,
                "date_digits": synthpass_die::date_digits_arm().0,
            })
        );
        assert_eq!(manifest["corpus_manifest"], "samples/corpus.jsonl");
        assert!(manifest["corpus_manifest_sha256"]
            .as_str()
            .is_some_and(|s| s.len() == 64));
        assert_eq!(
            std::fs::read_to_string(dir.join("provider-bench-ocr-current-run.txt")).unwrap(),
            name
        );
        let _ = std::fs::remove_file(dir.join(name));
        let _ = std::fs::remove_file(dir.join("provider-bench-ocr-current-run.txt"));
        let _ = std::fs::remove_dir(dir);
    }

    // --- the per-document archive (ADR-0024) -------------------------------

    fn args_from(flags: &[&str]) -> Args {
        let flags: Vec<String> = flags.iter().map(|s| s.to_string()).collect();
        parse_args(&flags).unwrap_or_else(|e| panic!("{flags:?} should parse: {e}"))
    }

    /// The flag is valid in every mode, on its own and beside the flags of that mode: a
    /// synthetic run, a real-specimen run, a replay, and a run with an opt-in track.
    #[test]
    fn no_archive_parses_in_every_mode() {
        let modes: [&[&str]; 6] = [
            &[],
            &[
                "--count",
                "5",
                "--profile",
                "clean",
                "--document-type",
                "td1",
            ],
            &["--real-specimens"],
            &[
                "--real-specimens",
                "--mrz-only",
                "--replay-ocr-passes",
                "capture-dir",
            ],
            &[
                "--real-specimens",
                "--dump-ocr",
                "--include-local",
                "--include-covers",
            ],
            &["--real-specimens", "--include-private"],
        ];
        for mode in modes {
            assert!(!args_from(mode).no_archive, "off by default: {mode:?}");
            for position in [0, mode.len()] {
                let mut flags = mode.to_vec();
                flags.insert(position, "--no-archive");
                assert!(args_from(&flags).no_archive, "{flags:?}");
            }
        }
    }

    /// The plan for a run, from injected facts: neither git nor the environment is asked
    /// unless a case says so.
    fn plan_for(flags: &[&str], variable: Option<&str>) -> ArchivePlan {
        // A working tree of its own, so a named root under the temp directory is outside it.
        let repo =
            std::env::temp_dir().join(format!("provider-bench-plan-repo-{}", std::process::id()));
        std::fs::create_dir_all(&repo).expect("create the fake working tree");
        archive_plan(
            &args_from(flags),
            variable,
            &repo,
            &repo,
            || panic!("this plan must not ask git"),
            |_| panic!("this plan must not ask git to check-ignore"),
        )
    }

    #[test]
    fn include_private_turns_the_archive_off_with_a_warning() {
        let ArchivePlan::Warn(warning) = plan_for(&["--real-specimens", "--include-private"], None)
        else {
            panic!("a private run writes no archive, and says so");
        };
        assert!(warning.starts_with("warning: archive: "), "{warning}");
        assert!(warning.contains("--include-private"), "{warning}");
        assert!(warning.contains("Decision 7"), "{warning}");
        // Even when a directory is named: the private track has no place in the archive.
        let dir = std::env::temp_dir().join("named-archive-root");
        let named = dir.to_str().expect("utf-8 temp path");
        assert!(matches!(
            plan_for(&["--real-specimens", "--include-private"], Some(named)),
            ArchivePlan::Warn(_)
        ));
        // A run that turned the archive off says nothing more about it.
        assert_eq!(
            plan_for(
                &["--real-specimens", "--include-private", "--no-archive"],
                None
            ),
            ArchivePlan::Off
        );
        assert_eq!(
            plan_for(&["--real-specimens", "--include-private"], Some("off")),
            ArchivePlan::Off
        );
    }

    #[test]
    fn the_flag_and_the_variable_choose_the_archive_plan() {
        assert_eq!(
            plan_for(&["--real-specimens", "--no-archive"], None),
            ArchivePlan::Off
        );
        assert_eq!(
            plan_for(&["--real-specimens"], Some("off")),
            ArchivePlan::Off
        );
        assert_eq!(
            plan_for(&["--real-specimens"], Some(" OFF ")),
            ArchivePlan::Off
        );
        // The flag beats a variable that names a directory.
        let dir = std::env::temp_dir().join("named-archive-root");
        let named = dir.to_str().expect("utf-8 temp path");
        assert_eq!(
            plan_for(&["--real-specimens", "--no-archive"], Some(named)),
            ArchivePlan::Off
        );
        // A named root outside the tree is used as it is.
        match plan_for(&["--real-specimens"], Some(named)) {
            ArchivePlan::Root(root) => assert!(root.ends_with("named-archive-root"), "{root:?}"),
            other => panic!("a root outside the tree is accepted: {other:?}"),
        }
    }

    /// The archive header and the OCR run manifest are two records of one run's arms, and they
    /// come from the same code: every key both files record is equal, and the keys they share
    /// are the ones a reader compares (extend this when the manifest half of #420 0b lands).
    #[test]
    fn header_arms_equal_the_dump_manifests() {
        let dir = std::env::temp_dir().join(format!(
            "provider-bench-header-vs-manifest-{}",
            std::process::id()
        ));
        let parsed = args_from(&["--real-specimens", "--mrz-only"]);
        let flags = vec!["--real-specimens".to_string(), "--mrz-only".to_string()];
        let replay = ReplaySource {
            run_manifest: "provider-bench-ocr-run-aa.json".to_string(),
            run_manifest_sha256: "b".repeat(64),
            ocr_arms: BTreeMap::from([("texture".to_string(), "off".to_string())]),
        };
        for source in [None, Some(&replay)] {
            let name = write_ocr_run_manifest(&repo_root(), &dir, &flags, 7, 3, source)
                .expect("write the manifest");
            let manifest: serde_json::Value =
                serde_json::from_slice(&std::fs::read(dir.join(&name)).expect("manifest exists"))
                    .expect("JSON");
            let header = serde_json::to_value(archive_header(
                &parsed,
                &repo_root(),
                &ArchiveRun {
                    argv: &flags,
                    documents_loaded: 7,
                    labelled_loaded: 3,
                    providers: vec!["mrz".to_string()],
                    replay: source,
                    model_paths: ModelPathsReport::default(),
                },
            ))
            .expect("serialize the header");

            let mut shared = Vec::new();
            for (key, value) in manifest.as_object().expect("an object") {
                if let Some(in_header) = header.get(key) {
                    assert_eq!(
                        in_header, value,
                        "{key} differs between the header and the manifest"
                    );
                    shared.push(key.as_str());
                }
            }
            for key in [
                "ocr_arms",
                "mrz_arms",
                "pivot_yy",
                "git_commit",
                "working_tree_dirty",
                "corpus_manifest_sha256",
                "documents_loaded",
                "labelled_loaded",
            ] {
                assert!(
                    shared.contains(&key),
                    "{key} is not recorded by both: {shared:?}"
                );
            }
            assert_eq!(
                header["replay_of"].is_null(),
                source.is_none(),
                "only a replay names its capture"
            );
            // A replay's OCR ran in the capture, so it records no budget of its own.
            assert_eq!(
                header["retry_budget"].is_null(),
                source.is_some(),
                "only a live run records its retry budget"
            );
            // The binary is a file name, never the path of the machine's user.
            assert!(
                header["binary"]
                    .as_str()
                    .is_some_and(|b| !b.contains('\\') && !b.contains('/')),
                "{}",
                header["binary"]
            );
            if let Some(source) = source {
                assert_eq!(header["ocr_arms"], serde_json::json!(source.ocr_arms));
                assert_eq!(manifest["replay_of"], header["replay_of"]);
            }
        }
        // The retry budget is the header's own record, from the source `synthpass-bench`'s
        // report uses.
        let header = archive_header(
            &parsed,
            &repo_root(),
            &ArchiveRun {
                argv: &flags,
                documents_loaded: 0,
                labelled_loaded: 0,
                providers: Vec::new(),
                replay: None,
                model_paths: ModelPathsReport::default(),
            },
        );
        let (max_passes, max_seconds) = synthpass_ocr::effective_retry_budget();
        let budget = header
            .retry_budget
            .as_ref()
            .expect("a live run records its retry budget");
        assert_eq!(
            (budget.max_passes, budget.max_seconds),
            (max_passes, max_seconds)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dump_ocr_without_real_specimens_is_rejected() {
        let args: Vec<String> = ["--dump-ocr"].iter().map(|s| s.to_string()).collect();
        assert!(
            parse_args(&args).is_err(),
            "--dump-ocr only makes sense scoping --real-specimens"
        );
    }

    #[test]
    fn include_private_parses_alongside_real_specimens_and_is_rejected_without_it() {
        let ok: Vec<String> = ["--real-specimens", "--include-private"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&ok).expect("valid combination").include_private);

        let bad: Vec<String> = ["--include-private"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            parse_args(&bad).is_err(),
            "--include-private only makes sense scoping --real-specimens"
        );
    }

    #[test]
    fn include_local_parses_alongside_real_specimens_and_is_rejected_without_it() {
        let ok: Vec<String> = ["--real-specimens", "--include-local"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&ok).expect("valid combination");
        assert!(parsed.include_local);
        assert!(
            !parsed.include_private,
            "the two opt-in tracks are independent: asking for one must not add the other"
        );

        let bad: Vec<String> = ["--include-local"].iter().map(|s| s.to_string()).collect();
        assert!(
            parse_args(&bad).is_err(),
            "--include-local only makes sense scoping --real-specimens"
        );
    }

    #[test]
    fn include_covers_parses_alongside_real_specimens_and_is_rejected_without_it() {
        let ok: Vec<String> = ["--real-specimens", "--include-covers"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&ok).expect("valid combination");
        assert!(parsed.include_covers);
        assert!(
            !parsed.include_local && !parsed.include_private,
            "the opt-in tracks are independent: asking for one must not add another"
        );

        let bad: Vec<String> = ["--include-covers"].iter().map(|s| s.to_string()).collect();
        assert!(
            parse_args(&bad).is_err(),
            "--include-covers only makes sense scoping --real-specimens"
        );
    }

    #[test]
    fn an_opt_in_track_is_rejected_with_either_baseline_flag() {
        for track in ["--include-private", "--include-local", "--include-covers"] {
            for baseline in ["--write-baseline", "--assert-baseline"] {
                let args: Vec<String> = ["--real-specimens", track, baseline, "baseline.json"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
                assert!(
                    parse_args(&args).is_err(),
                    "{track} with {baseline}: the committed baseline measures the public corpus \
                     only, so an opt-in track must never write or assert it"
                );
            }
        }
    }

    #[test]
    fn progress_parses_on_its_own() {
        // Unlike --dump-ocr, --progress carries no --real-specimens
        // precondition: it forces on a log that is already the default for an
        // interactive run, over either corpus source.
        let args: Vec<String> = ["--progress"].iter().map(|s| s.to_string()).collect();
        assert!(parse_args(&args).expect("--progress parses alone").progress);
    }

    #[test]
    fn progress_parses_alongside_real_specimens() {
        let args: Vec<String> = ["--real-specimens", "--progress"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args).expect("--progress parses").progress);
    }

    #[test]
    fn progress_flag_defaults_off() {
        // The *flag* defaults off; the effective behaviour is
        // `parsed.progress || stderr().is_terminal()`, resolved in `main` so
        // that a redirected run stays silent unless asked.
        assert!(!Args::default().progress);
    }

    #[test]
    fn format_rejects_an_unknown_class() {
        let args: Vec<String> = ["--real-specimens", "--format", "visa"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn document_type_parses_for_the_synthetic_corpus() {
        let args: Vec<String> = ["--document-type", "td1"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let parsed = parse_args(&args).expect("valid on its own");
        assert_eq!(parsed.document_type, Some(DocumentType::TD1));
        assert!(!parsed.real_specimens);
    }

    #[test]
    fn document_type_with_real_specimens_is_rejected() {
        // `--document-type` generates the synthetic corpus; `--real-specimens`
        // reads samples/ instead. Combining them is the same category error
        // `--format` without `--real-specimens` already rejects, mirrored.
        let args: Vec<String> = ["--real-specimens", "--document-type", "td2"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args).is_err());
    }

    #[test]
    fn document_type_rejects_an_unknown_value() {
        let args: Vec<String> = ["--document-type", "td4"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(parse_args(&args).is_err());
    }

    // --- --mrz-only + the real-specimen no-regression gate ----------------

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn mrz_only_parses_alone_and_with_real_specimens() {
        assert!(parse_args(&argv(&["--mrz-only"])).expect("parses").mrz_only);
        let p = parse_args(&argv(&["--real-specimens", "--mrz-only"])).expect("parses");
        assert!(p.mrz_only && p.real_specimens);
    }

    #[test]
    fn mrz_only_catalog_holds_exactly_the_deterministic_reader() {
        let catalog = mrz_only_catalog();
        assert_eq!(catalog.readers().len(), 1);
        assert_eq!(catalog.readers()[0].id().as_str(), "mrz");
    }

    #[test]
    fn baseline_flags_parse_and_conflict() {
        let p = parse_args(&argv(&[
            "--real-specimens",
            "--mrz-only",
            "--assert-baseline",
            "x.json",
        ]))
        .expect("parses");
        assert_eq!(p.assert_baseline.as_deref(), Some("x.json"));
        assert!(parse_args(&argv(&[
            "--write-baseline",
            "a.json",
            "--assert-baseline",
            "b.json"
        ]))
        .is_err());
    }

    fn snap(hits: usize, kinds: &[(&str, usize)]) -> RealSpecimenSnapshot {
        let by_miss_kind: BTreeMap<String, usize> =
            kinds.iter().map(|(k, n)| ((*k).to_string(), *n)).collect();
        let misses: usize = kinds.iter().map(|(_, n)| n).sum();
        let documents = hits + misses;
        // Via the same constant `RealSpecimenSnapshot::from_reports` uses, not a
        // second hand-written list — a copy here would let these tests exercise
        // a denominator the production path does not have.
        let off: usize = OFF_DENOMINATOR_KINDS
            .iter()
            .filter_map(|k| by_miss_kind.get(*k))
            .sum();
        RealSpecimenSnapshot {
            documents,
            scored: documents - off,
            tier1_hits: hits,
            by_miss_kind,
            strict_names: None,
            refusal_population: off,
            outcomes_sha256: None,
        }
    }

    fn baseline_with_tolerance(
        base: &RealSpecimenSnapshot,
        tolerance: usize,
    ) -> RealSpecimenBaseline {
        let mut b = baseline_from_snapshot(base, 1_757_030_400);
        b.tolerance = tolerance;
        b
    }

    #[test]
    fn baseline_identical_run_passes_clean() {
        let s = snap(
            120,
            &[
                ("checksum_failed", 24),
                ("no_mrz_found", 85),
                ("redacted_mrz", 9),
            ],
        );
        let b = baseline_with_tolerance(&s, 0);
        assert_eq!(check_baseline(&s, &b), Ok(vec![]));
    }

    #[test]
    fn baseline_fails_on_a_hit_count_drop() {
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 0);
        let regressed = snap(119, &[("checksum_failed", 25), ("no_mrz_found", 85)]);
        let err = check_baseline(&regressed, &b).expect_err("hits dropped");
        assert!(err.iter().any(|m| m.contains("HIT count regressed")));
    }

    #[test]
    fn baseline_fails_on_bucket_churn_that_nets_zero_on_hits() {
        // HITs flat at 120, but 3 documents moved checksum_failed -> no_mrz_found.
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 0);
        let churned = snap(120, &[("checksum_failed", 21), ("no_mrz_found", 88)]);
        let err = check_baseline(&churned, &b).expect_err("no_mrz_found grew");
        assert!(err.iter().any(|m| m.contains("`no_mrz_found` grew")));
    }

    /// #536: a baseline written before `document_number_leading_filler` existed
    /// carries no key for it at all — `check_baseline`'s
    /// `.get(bucket).copied().unwrap_or(0)` must read that absence as zero, so
    /// adding the bucket alone never fails the gate, and a run that now
    /// produces a few such misses is still caught as a genuine regression from
    /// that implied zero.
    #[test]
    fn baseline_missing_the_leading_filler_bucket_reads_as_zero() {
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 0);
        assert!(!b
            .by_miss_kind
            .contains_key("document_number_leading_filler"));

        // No leading-filler misses: passes clean, exactly as before the
        // bucket was added.
        let clean = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        assert_eq!(check_baseline(&clean, &b), Ok(vec![]));

        // HITs flat at 120 (same total documents, 229): 3 moved from
        // `no_mrz_found` to the new bucket. The missing baseline key defaults
        // to zero, so this growth must still fail the gate.
        let regressed = snap(
            120,
            &[
                ("checksum_failed", 24),
                ("no_mrz_found", 82),
                ("document_number_leading_filler", 3),
            ],
        );
        let err = check_baseline(&regressed, &b).expect_err("leading-filler misses grew from zero");
        assert!(err
            .iter()
            .any(|m| m.contains("`document_number_leading_filler` grew: 0 -> 3")));
    }

    #[test]
    fn baseline_passes_a_genuine_improvement() {
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 0);
        let better = snap(123, &[("checksum_failed", 21), ("no_mrz_found", 85)]);
        assert_eq!(check_baseline(&better, &b), Ok(vec![]));
    }

    #[test]
    fn baseline_tolerance_absorbs_small_drift_but_not_more() {
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 2);
        assert_eq!(
            check_baseline(
                &snap(118, &[("checksum_failed", 26), ("no_mrz_found", 85)]),
                &b
            ),
            Ok(vec![])
        );
        assert!(check_baseline(
            &snap(117, &[("checksum_failed", 27), ("no_mrz_found", 85)]),
            &b
        )
        .is_err());
    }

    #[test]
    fn baseline_corpus_growth_warns_but_does_not_fail() {
        let base = snap(
            120,
            &[
                ("checksum_failed", 24),
                ("no_mrz_found", 85),
                ("redacted_mrz", 9),
            ],
        );
        let b = baseline_with_tolerance(&base, 0);
        // two specimens added: one HIT, one redacted.
        let grown = snap(
            121,
            &[
                ("checksum_failed", 24),
                ("no_mrz_found", 85),
                ("redacted_mrz", 10),
            ],
        );
        let warnings = check_baseline(&grown, &b).expect("growth is not a regression");
        assert!(warnings.iter().any(|w| w.contains("corpus size")));
        assert!(warnings.iter().any(|w| w.contains("redacted_mrz")));
    }

    /// The exact shape of the 2026-09-09 reclassification, and the reason the
    /// denominator warning exists: documents move between buckets, HITs are
    /// untouched, and no regression bucket grows — so every failure check stays
    /// silent while the published hit rate goes 52.0% -> 82.6%.
    ///
    /// The three populations that move: 42 with no MRZ at all, 27 redacted
    /// specimens that used to be filed as detection failures because their
    /// blackout bar produced no parseable noise, and 15 whose printed zone was
    /// non-conforming but imperfectly read.
    #[test]
    fn baseline_warns_when_only_the_denominator_moved() {
        let base = snap(
            119,
            &[
                ("checksum_failed", 24),
                ("checksum_failed_specimen", 1),
                ("no_mrz_found", 85),
                ("redacted_mrz", 9),
            ],
        );
        // `snap` applies today's rules, so this is 228 rather than the 229 the
        // baseline was actually published with — the single
        // `checksum_failed_specimen` is off-denominator now and was not then.
        // The published figure is prose here on purpose; asserting a historical
        // number through a helper that no longer models it would pin nothing.
        let b = baseline_with_tolerance(&base, 0);

        let reclassified = snap(
            119,
            &[
                ("checksum_failed", 7),
                ("checksum_failed_specimen", 16),
                ("no_mrz_found", 18),
                ("no_mrz_expected", 42),
                ("redacted_mrz", 36),
            ],
        );
        assert_eq!(reclassified.scored, 144, "the corrected denominator");

        let warnings =
            check_baseline(&reclassified, &b).expect("a reclassification is not a regression");
        assert!(
            warnings.iter().any(|w| w.contains("denominator changed")),
            "the denominator moving must be reported: {warnings:?}"
        );
        for bucket in [
            "no_mrz_expected",
            "redacted_mrz",
            "checksum_failed_specimen",
        ] {
            assert!(
                warnings.iter().any(|w| w.contains(bucket)),
                "{bucket} drifted and must be named: {warnings:?}"
            );
        }
    }

    /// `checksum_failed_specimen` growing is no longer a regression — it is an
    /// off-denominator population, so a document moving into it is a corpus
    /// fact, not a parser fault. Its *conforming* sibling still is.
    #[test]
    fn nonconforming_specimens_warn_while_real_checksum_failures_fail() {
        let base = snap(
            119,
            &[("checksum_failed", 7), ("checksum_failed_specimen", 16)],
        );
        let b = baseline_with_tolerance(&base, 0);

        let more_nonconforming = snap(
            119,
            &[("checksum_failed", 7), ("checksum_failed_specimen", 18)],
        );
        let warnings = check_baseline(&more_nonconforming, &b)
            .expect("a non-conforming specimen appearing is not a regression");
        assert!(warnings
            .iter()
            .any(|w| w.contains("checksum_failed_specimen")));

        let more_real = snap(
            119,
            &[("checksum_failed", 9), ("checksum_failed_specimen", 16)],
        );
        let err = check_baseline(&more_real, &b).expect_err("real OCR failures growing is");
        assert!(err.iter().any(|m| m.contains("`checksum_failed` grew")));
    }

    /// A false positive is inside the denominator and inside the regression
    /// buckets: a checksum-valid MRZ returned for a document that carries none
    /// must block a merge, not warn.
    #[test]
    fn baseline_fails_when_false_positives_appear() {
        let base = snap(119, &[("no_mrz_found", 43), ("no_mrz_expected", 42)]);
        let b = baseline_with_tolerance(&base, 0);
        let hallucinating = snap(
            119,
            &[
                ("no_mrz_found", 43),
                ("no_mrz_expected", 41),
                ("false_positive_mrz", 1),
            ],
        );
        let err = check_baseline(&hallucinating, &b).expect_err("a false positive is a regression");
        assert!(err.iter().any(|m| m.contains("`false_positive_mrz` grew")));
    }

    #[test]
    fn baseline_json_round_trips() {
        let s = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&s, 1);
        let text = serde_json::to_string_pretty(&b).expect("serialize");
        let back: RealSpecimenBaseline = serde_json::from_str(&text).expect("deserialize");
        assert_eq!(back.tier1_hits, b.tier1_hits);
        assert_eq!(back.by_miss_kind, b.by_miss_kind);
        assert_eq!(back.tolerance, 1);
    }

    #[test]
    fn iso_date_is_utc_civil_from_days() {
        assert_eq!(iso_date(0), "1970-01-01");
        assert_eq!(iso_date(1_757_030_400), "2025-09-05");
    }

    // ---------------------------------------------------------------------
    // ADR-0013 strict-name counts: report-only, never gated. See
    // `StrictNamesBaseline`'s doc.
    // ---------------------------------------------------------------------

    #[test]
    fn baseline_with_strict_names_round_trips() {
        let mut s = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        s.strict_names = Some(StrictNamesBaseline {
            strict_hits: 40,
            name_scorable_documents: 60,
            name_scorable_hits: 50,
        });
        let b = baseline_from_snapshot(&s, 1_757_030_400);
        let text = serde_json::to_string_pretty(&b).expect("serialize");
        assert!(text.contains("strict_names"));
        let back: RealSpecimenBaseline = serde_json::from_str(&text).expect("deserialize");
        assert_eq!(back.strict_names, b.strict_names);
    }

    #[test]
    fn check_baseline_warns_but_does_not_fail_when_strict_counts_move() {
        let mut base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        base.strict_names = Some(StrictNamesBaseline {
            strict_hits: 40,
            name_scorable_documents: 60,
            name_scorable_hits: 50,
        });
        let b = baseline_with_tolerance(&base, 0);

        let mut moved = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        moved.strict_names = Some(StrictNamesBaseline {
            strict_hits: 41,
            name_scorable_documents: 60,
            name_scorable_hits: 50,
        });

        let warnings =
            check_baseline(&moved, &b).expect("a strict-name count moving is never a regression");
        assert!(warnings
            .iter()
            .any(|w| w.contains("strict-name counts changed") && w.contains("ADR-0013")));
    }

    #[test]
    fn check_baseline_warns_once_when_strict_names_appears_or_disappears() {
        let base = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let b = baseline_with_tolerance(&base, 0);

        let mut appeared = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        appeared.strict_names = Some(StrictNamesBaseline {
            strict_hits: 1,
            name_scorable_documents: 1,
            name_scorable_hits: 1,
        });
        let warnings = check_baseline(&appeared, &b).expect("appearing is not a regression");
        assert!(warnings.iter().any(|w| w.contains("appeared in this run")));

        let mut with_strict = base;
        with_strict.strict_names = Some(StrictNamesBaseline {
            strict_hits: 1,
            name_scorable_documents: 1,
            name_scorable_hits: 1,
        });
        let b_with_strict = baseline_with_tolerance(&with_strict, 0);
        let disappeared = snap(120, &[("checksum_failed", 24), ("no_mrz_found", 85)]);
        let warnings =
            check_baseline(&disappeared, &b_with_strict).expect("disappearing is not a regression");
        assert!(warnings.iter().any(|w| w.contains("has none")));
    }

    // ---------------------------------------------------------------------
    // Outcome ledger: `OutcomeRow`, `build_outcome_rows`, `ledger_bytes`,
    // `sha256_hex`, `refusal_population`, the every-bucket-present pre-fill,
    // and the assert-mode ledger integrity check + per-document diff.
    // ---------------------------------------------------------------------

    /// A minimal [`DocumentDetail`] with only the fields a test needs varied
    /// set explicitly — everything else is a fixed, inert default.
    fn detail(
        name: &str,
        asset_id: Option<&str>,
        miss_reason: Option<MissReason>,
    ) -> DocumentDetail {
        DocumentDetail {
            name: name.to_string(),
            asset_id: asset_id.map(str::to_string),
            mrz_found: miss_reason.is_none(),
            mrz_format: Some("TD3"),
            read_ok: true,
            mrz_checksums_valid: miss_reason.is_none(),
            check_states: None,
            miss_reason,
            assertions_total: 0,
            assertions_unsupported: 0,
            unsupported_fields: Vec::new(),
            names_exact: None,
            name_error: None,
            field_correctness: None,
            ocr_elapsed: Duration::from_millis(7),
            retry_variant_id: None,
            retry_damaged_recovery: None,
            retry_budget_hit: false,
            retry_stop: None,
            chargrid: None,
            tier1_damaged_recovery: None,
            line1_selection: None,
        }
    }

    /// A minimal `mrz`-provider [`ProviderReport`] carrying real
    /// `documents_detail` rows — the outcome-ledger equivalent of
    /// `mrz_report_with_strict` above, which deliberately leaves
    /// `documents_detail` empty.
    fn mrz_report_with_details(details: Vec<DocumentDetail>) -> ProviderReport {
        ProviderReport {
            provider_id: "mrz".to_string(),
            documents: details.len(),
            capability: CapabilitySnapshot {
                deterministic: true,
                vision: false,
                cost: "free",
            },
            accuracy: AccuracyStats {
                labelled_documents: 0,
                field_match_rate: None,
                mean_cer: None,
                per_field_cer: Vec::new(),
                accepted_reads: Default::default(),
                scored: Default::default(),
            },
            speed: SpeedStats {
                mean: Duration::ZERO,
                p50: Duration::ZERO,
                p95: Duration::ZERO,
            },
            json_validity: None,
            unsupported_assertion: UnsupportedAssertion::NotApplicable {
                reason: "test fixture",
            },
            declared_resident_bytes: None,
            measured_rss_delta_bytes: None,
            documents_detail: details,
            tier1_hit_rate: Tier1HitRate::Computed(0.0),
            ocr_arms: synthpass_ocr::OcrArms::DEFAULT,
            strict_tier1_hit_rate: StrictNameHitRate::NotApplicable {
                reason: "test fixture",
            },
        }
    }

    /// A tiny, fully-specified [`OutcomeRow`] for tests that only care about
    /// `asset_id`/`outcome` (the join key and the field [`diff_outcomes`]
    /// compares).
    fn outcome_row(asset_id: &str, outcome: &str) -> OutcomeRow {
        OutcomeRow {
            asset_id: Some(asset_id.to_string()),
            name: asset_id.to_string(),
            outcome: outcome.to_string(),
            miss_reason: None,
            mrz_format: None,
            mrz_found: outcome == "hit",
            mrz_checksums_valid: outcome == "hit",
            names_exact: None,
            name_error: None,
            ocr_ms: 1,
            retry_variant_id: None,
            retry_budget_hit: false,
            retry_stop: None,
            check_states: None,
            retry_damaged_recovery: None,
            tier1_damaged_recovery: None,
        }
    }

    #[test]
    fn build_outcome_rows_sorts_by_asset_id_falling_back_to_name() {
        let report = mrz_report_with_details(vec![
            detail("zzz-no-asset", None, None),
            detail("b-doc", Some("passports/b.png"), None),
            detail(
                "a-doc",
                Some("passports/a.png"),
                Some(MissReason::NoMrzFound("no MRZ-shaped text".to_string())),
            ),
        ]);
        let rows = build_outcome_rows(&report);
        let keys: Vec<&str> = rows.iter().map(OutcomeRow::sort_key).collect();
        assert_eq!(
            keys,
            vec!["passports/a.png", "passports/b.png", "zzz-no-asset"],
            "asset_id sorts first; a row with no asset_id falls back to its name"
        );
    }

    #[test]
    fn build_outcome_rows_carries_check_states_and_both_damaged_recovery_flags() {
        let mut with_mrz = detail("a", Some("a"), None);
        with_mrz.check_states = Some(BTreeMap::from([
            ("composite", Some(true)),
            ("personal_number", None),
        ]));
        with_mrz.retry_damaged_recovery = Some(true);
        with_mrz.tier1_damaged_recovery = Some(false);
        let rows = build_outcome_rows(&mrz_report_with_details(vec![
            with_mrz,
            detail("b", Some("b"), Some(MissReason::Redacted)),
        ]));
        assert_eq!(
            rows[0].check_states,
            Some(BTreeMap::from([
                ("composite".to_string(), Some(true)),
                ("personal_number".to_string(), None),
            ]))
        );
        assert_eq!(rows[0].retry_damaged_recovery, Some(true));
        assert_eq!(rows[0].tier1_damaged_recovery, Some(false));
        assert_eq!(rows[1].check_states, None);
        assert_eq!(rows[1].retry_damaged_recovery, None);
        assert_eq!(rows[1].tier1_damaged_recovery, None);
        let line = serde_json::to_string(&rows[1]).expect("serialize");
        assert!(
            line.ends_with(
                r#""check_states":null,"retry_damaged_recovery":null,"tier1_damaged_recovery":null}"#
            ),
            "absent values serialize as null, never as omitted keys: {line}"
        );
    }

    #[test]
    fn build_outcome_rows_is_deterministic_across_calls() {
        let report = mrz_report_with_details(vec![
            detail("c", Some("c"), None),
            detail("a", Some("a"), None),
            detail("b", Some("b"), Some(MissReason::Redacted)),
        ]);
        assert_eq!(build_outcome_rows(&report), build_outcome_rows(&report));
    }

    #[test]
    fn ledger_bytes_are_one_compact_line_per_row_with_a_trailing_newline() {
        let report = mrz_report_with_details(vec![
            detail("a", Some("passports/a.png"), None),
            detail("b", Some("passports/b.png"), Some(MissReason::Redacted)),
        ]);
        let rows = build_outcome_rows(&report);
        let bytes = ledger_bytes(&rows);
        let text = String::from_utf8(bytes).expect("valid UTF-8");
        assert!(text.ends_with('\n'), "must end with a trailing newline");
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        for line in &lines {
            assert!(!line.contains('\n'), "each line is exactly one JSON object");
            serde_json::from_str::<OutcomeRow>(line).expect("each line parses as one OutcomeRow");
        }
    }

    #[test]
    fn sha256_hex_matches_known_test_vectors() {
        // NIST's two smallest published SHA-256 test vectors — proof this
        // hashes the exact bytes given, not some other digest.
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn outcomes_ledger_path_sits_next_to_the_baseline() {
        let p = outcomes_ledger_path("knowledge/benchmarks/real-specimen-mrz-baseline.json");
        assert_eq!(p.file_name().unwrap(), OUTCOMES_LEDGER_FILENAME);
        assert_eq!(
            p.parent().unwrap(),
            std::path::Path::new("knowledge/benchmarks")
        );
    }

    #[test]
    fn write_baseline_and_ledger_writes_a_ledger_whose_sha_matches_the_baseline_field() {
        let reports = [mrz_report_with_details(vec![
            detail("a", Some("a"), None),
            detail("b", Some("b"), Some(MissReason::Redacted)),
        ])];
        let snap = RealSpecimenSnapshot::from_reports(&reports).expect("mrz provider present");
        let rows = build_outcome_rows(&reports[0]);

        let dir = std::env::temp_dir().join(format!(
            "synthpass-outcome-ledger-test-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        let baseline_path = dir.join("baseline.json");
        let baseline_path_str = baseline_path.to_str().expect("utf8 temp path").to_string();

        let baseline = write_baseline_and_ledger(&baseline_path_str, &snap, &rows, 1_757_030_400);

        let ledger_path = outcomes_ledger_path(&baseline_path_str);
        let ledger_bytes_on_disk = std::fs::read(&ledger_path).expect("ledger was written");
        let expected_sha = sha256_hex(&ledger_bytes_on_disk);
        assert_eq!(baseline.outcomes_sha256, Some(expected_sha));
        assert_eq!(baseline.refusal_population, Some(1));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn verify_ledger_sha_passes_when_the_hash_matches() {
        let bytes: &[u8] = b"{\"a\":1}\n";
        let sha = sha256_hex(bytes);
        assert_eq!(verify_ledger_sha(bytes, &sha), Ok(()));
    }

    #[test]
    fn verify_ledger_sha_fails_on_a_mismatch() {
        // The exact check `--assert-baseline` uses to fail the gate when the
        // committed ledger no longer matches the baseline it shipped with.
        let bytes: &[u8] = b"{\"a\":1}\n";
        let wrong_sha = "0".repeat(64);
        let err = verify_ledger_sha(bytes, &wrong_sha).expect_err("hashes must not match");
        assert!(err.contains("sha256 mismatch"));
        assert!(err.contains(&wrong_sha));
    }

    #[test]
    fn diff_outcomes_reports_changed_and_one_sided_rows_without_a_failure_signal() {
        // `diff_outcomes` has no `Result`/failure variant at all — its return
        // type is the proof that an outcome-ledger diff is purely
        // informational, whatever it finds.
        let committed = vec![
            outcome_row("a", "hit"),
            outcome_row("b", "no_mrz_found"),
            outcome_row("only-committed", "hit"),
        ];
        let actual = vec![
            outcome_row("a", "hit"),
            outcome_row("b", "hit"),
            outcome_row("only-actual", "hit"),
        ];
        let lines = diff_outcomes(&committed, &actual);
        assert!(lines[0].contains("1 document(s) changed outcome"));
        assert!(lines[0].contains("1 only in the committed ledger"));
        assert!(lines[0].contains("1 only in this run"));
        assert!(
            lines.iter().any(|l| l.contains("b: no_mrz_found -> hit")),
            "the changed row must be named: {lines:?}"
        );
    }

    #[test]
    fn diff_outcomes_is_empty_summary_when_nothing_moved() {
        let rows = vec![outcome_row("a", "hit"), outcome_row("b", "checksum_failed")];
        let lines = diff_outcomes(&rows, &rows);
        assert!(lines[0].contains("0 document(s) changed outcome"));
        assert!(lines[0].contains("0 only in the committed ledger"));
        assert!(lines[0].contains("0 only in this run"));
        assert_eq!(lines.len(), 1, "no per-document rows when nothing moved");
    }

    /// `outcome_row(asset_id, "hit")` with `edit` applied — a one-line way to
    /// vary the field a `diff_ledger_fields` test is about.
    fn hit_row_with(asset_id: &str, edit: impl FnOnce(&mut OutcomeRow)) -> OutcomeRow {
        let mut row = outcome_row(asset_id, "hit");
        edit(&mut row);
        row
    }

    #[test]
    fn diff_ledger_fields_prints_nothing_when_no_field_moved() {
        let rows = vec![outcome_row("a", "hit"), outcome_row("b", "checksum_failed")];
        // Only the existing outcome line remains for an unchanged ledger.
        assert_eq!(diff_ledger_fields(&rows, &rows), Vec::<String>::new());
        assert_eq!(diff_outcomes(&rows, &rows).len(), 1);
    }

    /// One `check_states` map: `document_number` verified, `composite` as given.
    fn states(composite: bool) -> BTreeMap<String, Option<bool>> {
        BTreeMap::from([
            ("composite".to_string(), Some(composite)),
            ("document_number".to_string(), Some(true)),
            ("personal_number".to_string(), None),
        ])
    }

    #[test]
    fn diff_ledger_fields_reports_a_check_states_change_as_compact_json() {
        let committed = vec![hit_row_with("a", |r| r.check_states = Some(states(true)))];
        let actual = vec![hit_row_with("a", |r| r.check_states = Some(states(false)))];
        assert_eq!(
            diff_ledger_fields(&committed, &actual),
            vec![
                "deterministic field diff vs committed (report-only): 1 document(s); \
                 check_states 1"
                    .to_string(),
                "  a: check_states {\"composite\":true,\"document_number\":true,\"personal_number\":null} \
                 -> {\"composite\":false,\"document_number\":true,\"personal_number\":null}"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn diff_ledger_fields_reports_a_retry_damaged_recovery_change() {
        let committed = vec![hit_row_with("a", |r| {
            r.retry_damaged_recovery = Some(false)
        })];
        let actual = vec![hit_row_with("a", |r| r.retry_damaged_recovery = Some(true))];
        assert_eq!(
            diff_ledger_fields(&committed, &actual),
            vec![
                "deterministic field diff vs committed (report-only): 1 document(s); \
                 retry_damaged_recovery 1"
                    .to_string(),
                "  a: retry_damaged_recovery false -> true".to_string(),
            ]
        );
    }

    #[test]
    fn diff_ledger_fields_reports_a_tier1_damaged_recovery_change() {
        let committed = vec![hit_row_with("a", |r| r.tier1_damaged_recovery = Some(true))];
        let actual = vec![hit_row_with("a", |r| r.tier1_damaged_recovery = None)];
        assert_eq!(
            diff_ledger_fields(&committed, &actual),
            vec![
                "deterministic field diff vs committed (report-only): 1 document(s); \
                 tier1_damaged_recovery 1"
                    .to_string(),
                "  a: tier1_damaged_recovery true -> null".to_string(),
            ]
        );
    }

    /// A ledger committed before the three fields existed still parses: they
    /// read as `None`, exactly as `null` does, so the diff against a run that
    /// has them reports `null -> value` (until the ledger is re-blessed) and
    /// the outcome line is untouched.
    #[test]
    fn a_ledger_without_the_recovery_and_check_state_fields_parses_and_diffs_from_null() {
        let old_line = r#"{"asset_id":"a","name":"a","outcome":"hit","miss_reason":null,"mrz_format":"TD3","mrz_found":true,"mrz_checksums_valid":true,"names_exact":null,"name_error":null,"ocr_ms":1,"retry_variant_id":null,"retry_budget_hit":false,"retry_stop":null}"#;
        let committed =
            parse_ledger(format!("{old_line}\n").as_bytes()).expect("old ledger parses");
        assert_eq!(committed[0].check_states, None);
        assert_eq!(committed[0].retry_damaged_recovery, None);
        assert_eq!(committed[0].tier1_damaged_recovery, None);

        let actual = vec![hit_row_with("a", |r| {
            r.mrz_format = Some("TD3".to_string());
            r.check_states = Some(states(true));
            r.retry_damaged_recovery = Some(false);
            r.tier1_damaged_recovery = Some(false);
        })];
        assert_eq!(diff_outcomes(&committed, &actual).len(), 1);
        let lines = diff_ledger_fields(&committed, &actual);
        assert_eq!(
            lines[0],
            "deterministic field diff vs committed (report-only): 1 document(s); check_states 1, \
             retry_damaged_recovery 1, tier1_damaged_recovery 1"
        );
        assert!(
            lines[1].starts_with("  a: check_states null -> {"),
            "{lines:?}"
        );
        assert!(lines[1].ends_with(
            "; retry_damaged_recovery null -> false; tier1_damaged_recovery null -> false"
        ));
    }

    #[test]
    fn diff_ledger_fields_reports_a_deterministic_change_in_schema_order() {
        // Like `diff_outcomes`, the return type has no failure variant: a
        // per-field diff is informational whatever it finds.
        let committed = vec![hit_row_with("a", |r| {
            r.mrz_format = Some("MRVA".to_string())
        })];
        let actual = vec![hit_row_with("a", |r| {
            r.mrz_format = Some("TD3".to_string());
            r.mrz_checksums_valid = false;
        })];
        assert_eq!(
            diff_ledger_fields(&committed, &actual),
            vec![
                "deterministic field diff vs committed (report-only): 1 document(s); mrz_format \
                 1, mrz_checksums_valid 1"
                    .to_string(),
                "  a: mrz_format MRVA -> TD3; mrz_checksums_valid true -> false".to_string(),
            ]
        );
    }

    #[test]
    fn diff_ledger_fields_prints_the_miss_kind_and_never_the_miss_reason_text() {
        let no_mrz = |detail: &str| {
            hit_row_with("d", |r| {
                r.outcome = "no_mrz_found".to_string();
                r.miss_reason = Some(format!("no MRZ found: {detail}"));
            })
        };
        let committed = vec![no_mrz("SECRET-OLD")];
        let actual = vec![no_mrz("SECRET-NEW")];
        let lines = diff_ledger_fields(&committed, &actual);
        assert_eq!(
            lines,
            vec![
                "deterministic field diff vs committed (report-only): 1 document(s); miss_reason 1"
                    .to_string(),
                "  d: miss_reason no_mrz_found (detail changed)".to_string(),
            ]
        );
        let printed = lines.join("\n");
        for text in ["SECRET-OLD", "SECRET-NEW"] {
            assert!(
                !printed.contains(text),
                "the miss_reason text must never be printed: {printed}"
            );
        }
    }

    #[test]
    fn diff_ledger_fields_names_both_kinds_when_the_kind_changed() {
        let committed = vec![hit_row_with("d", |r| {
            r.outcome = "checksum_failed".to_string();
            r.miss_reason = Some("checksum invalid: composite".to_string());
        })];
        let actual = vec![outcome_row("d", "hit")];
        let lines = diff_ledger_fields(&committed, &actual);
        assert!(
            lines.contains(&"  d: miss_reason checksum_failed -> hit (detail changed)".to_string()),
            "{lines:?}"
        );
        assert!(
            !lines.join("\n").contains("composite"),
            "the miss_reason text must never be printed"
        );
    }

    #[test]
    fn diff_ledger_fields_reports_ocr_ms_only_changes_as_one_timing_line() {
        let with_ms = |id: &str, ms: u128| hit_row_with(id, |r| r.ocr_ms = ms);
        let committed = vec![with_ms("a", 1000), with_ms("b", 2000), with_ms("c", 300)];
        let actual = vec![with_ms("a", 1500), with_ms("b", 1000), with_ms("c", 300)];
        // |delta| is 500 and 1000: the median of an even count is the integer
        // mean of the middle two. `c` did not move and is not counted as
        // differing, but is in the "of 3" and in both totals.
        assert_eq!(
            diff_ledger_fields(&committed, &actual),
            vec![
                "timing-sensitive field diff vs committed (report-only): ocr_ms differs on 2 of 3 \
                 document(s), median |delta| 750 ms, total 3300 ms -> 2800 ms"
                    .to_string()
            ]
        );
    }

    #[test]
    fn diff_ledger_fields_moves_every_change_on_a_budget_limited_document_to_the_timing_group() {
        let committed = vec![
            hit_row_with("b", |r| {
                r.mrz_format = Some("TD3".to_string());
                r.retry_stop = Some("exhausted".to_string());
            }),
            // Budget-limited on the committed side only.
            hit_row_with("c", |r| r.retry_stop = Some("budget".to_string())),
            hit_row_with("d", |r| r.mrz_found = true),
        ];
        let actual = vec![
            hit_row_with("b", |r| {
                r.mrz_format = Some("MRVA".to_string());
                r.retry_budget_hit = true;
                r.retry_stop = Some("budget".to_string());
            }),
            hit_row_with("c", |r| {
                r.names_exact = Some(true);
                r.retry_stop = Some("exhausted".to_string());
            }),
            hit_row_with("d", |r| r.mrz_found = false),
        ];
        assert_eq!(
            diff_ledger_fields(&committed, &actual),
            vec![
                // `d` is not budget-limited, so it alone is deterministic.
                "deterministic field diff vs committed (report-only): 1 document(s); mrz_found 1"
                    .to_string(),
                "  d: mrz_found true -> false".to_string(),
                "budget-limited document(s) vs committed (report-only; every change on them is \
                 timing-sensitive): 2 document(s); mrz_format 1, names_exact 1, retry_budget_hit \
                 1, retry_stop 2"
                    .to_string(),
                "  b: mrz_format TD3 -> MRVA; retry_budget_hit false -> true; retry_stop \
                 exhausted -> budget"
                    .to_string(),
                "  c: names_exact null -> true; retry_stop budget -> exhausted".to_string(),
            ]
        );
    }

    #[test]
    fn diff_ledger_fields_caps_document_lines_at_twenty_but_keeps_complete_totals() {
        let ids: Vec<String> = (0..25).map(|i| format!("doc-{i:02}")).collect();
        let committed: Vec<OutcomeRow> = ids.iter().map(|id| outcome_row(id, "hit")).collect();
        let actual: Vec<OutcomeRow> = ids
            .iter()
            .map(|id| hit_row_with(id, |r| r.mrz_found = false))
            .collect();
        let lines = diff_ledger_fields(&committed, &actual);
        assert_eq!(
            lines[0],
            "deterministic field diff vs committed (report-only): 25 document(s); mrz_found 25"
        );
        assert_eq!(
            lines.len(),
            22,
            "totals, 20 document lines, the overflow line"
        );
        assert!(lines[1].starts_with("  doc-00: "));
        assert!(lines[20].starts_with("  doc-19: "));
        assert_eq!(lines[21], "  ... and 5 more");
    }

    #[test]
    fn diff_ledger_fields_is_ordered_by_key_whatever_the_input_order() {
        let changed = |id: &str| hit_row_with(id, |r| r.name_error = Some("swapped".to_string()));
        let committed = vec![
            outcome_row("c", "hit"),
            outcome_row("a", "hit"),
            outcome_row("b", "hit"),
        ];
        let actual = vec![changed("b"), changed("c"), changed("a")];
        let lines = diff_ledger_fields(&committed, &actual);
        let keys: Vec<&str> = lines[1..]
            .iter()
            .map(|l| l.trim_start().split(':').next().expect("key before colon"))
            .collect();
        assert_eq!(keys, vec!["a", "b", "c"]);
        let mut committed_sorted = committed.clone();
        committed_sorted.sort_by(|x, y| x.sort_key().cmp(y.sort_key()));
        assert_eq!(diff_ledger_fields(&committed_sorted, &actual), lines);
    }

    #[test]
    fn diff_ledger_fields_skips_a_document_present_on_one_side_only() {
        let committed = vec![
            outcome_row("a", "hit"),
            hit_row_with("only-committed", |r| r.mrz_found = false),
        ];
        let actual = vec![
            hit_row_with("a", |r| r.mrz_format = Some("TD3".to_string())),
            hit_row_with("only-actual", |r| r.ocr_ms = 99_999),
        ];
        let lines = diff_ledger_fields(&committed, &actual);
        assert_eq!(
            lines,
            vec![
                "deterministic field diff vs committed (report-only): 1 document(s); mrz_format 1"
                    .to_string(),
                "  a: mrz_format null -> TD3".to_string(),
            ],
            "documents on one side only are counted by `diff_outcomes`, not diffed here"
        );
    }

    /// A fresh scratch directory under the system temp dir, unique per test.
    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "synthpass-field-diff-test-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    /// The step summary writer moved to `synthpass_bench::step_summary`, where
    /// its test (`append_step_summary_writes_the_diff_lines_and_nothing_without_a_path`)
    /// went with it. What stays here is this binary's own heading.
    #[test]
    fn this_binarys_step_summary_heading_names_the_real_specimen_diff() {
        assert!(
            STEP_SUMMARY_TITLE.starts_with("### Real-specimen"),
            "{STEP_SUMMARY_TITLE}"
        );
        assert!(
            STEP_SUMMARY_TITLE.contains("report-only"),
            "{STEP_SUMMARY_TITLE}"
        );
    }

    #[test]
    fn text_free_projection_holds_no_miss_reason_text_for_a_document_number_mismatch() {
        let mismatch = hit_row_with("d", |r| {
            r.outcome = "document_number_mismatch".to_string();
            r.miss_reason = Some("document number mismatch".to_string());
            r.mrz_format = Some("TD3".to_string());
        });
        let bad_code = hit_row_with("e", |r| {
            r.outcome = "no_mrz_found".to_string();
            r.miss_reason = Some("no MRZ found: BadDocumentCode(\"PZ\")".to_string());
        });
        let hit = outcome_row("h", "hit");
        let rows = vec![mismatch.clone(), bad_code, hit.clone()];

        let projected = text_free_projection(&rows);
        let bytes = String::from_utf8(ledger_bytes(&projected)).expect("valid UTF-8");
        for text in [
            "X1234567",
            "Y7654321",
            "got",
            "expected",
            "BadDocumentCode",
            "PZ",
        ] {
            assert!(
                !bytes.contains(text),
                "the projection must hold no miss_reason text, found {text:?}: {bytes}"
            );
        }
        // The kind survives, every other field is kept, and a hit stays a hit.
        assert_eq!(
            projected[0].miss_reason.as_deref(),
            Some("document_number_mismatch")
        );
        assert_eq!(
            projected[0],
            OutcomeRow {
                miss_reason: Some("document_number_mismatch".to_string()),
                ..mismatch
            }
        );
        assert_eq!(projected[1].miss_reason.as_deref(), Some("no_mrz_found"));
        assert_eq!(projected[2], hit);
        // Still a ledger: it parses back and diffs like one.
        assert_eq!(
            parse_ledger(&ledger_bytes(&projected)).expect("projection parses"),
            projected
        );
    }

    #[test]
    fn write_run_ledger_projection_writes_under_out_and_never_as_the_committed_ledger() {
        let dir = scratch_dir("run-ledger");
        let out_dir = dir.join("artifacts");
        let rows = vec![hit_row_with("a", |r| {
            r.outcome = "document_number_mismatch".to_string();
            r.miss_reason = Some("document number mismatch".to_string());
        })];

        let out = out_dir.join("report.json");
        let written =
            write_run_ledger_projection(out.to_str().expect("utf8"), &rows).expect("write");
        assert_eq!(written, out_dir.join(RUN_LEDGER_PROJECTION_FILENAME));
        assert_ne!(
            written.file_name().expect("file name"),
            OUTCOMES_LEDGER_FILENAME,
            "the projection must not share the committed ledger's name"
        );
        let on_disk = std::fs::read_to_string(&written).expect("projection on disk");
        assert!(
            !on_disk.contains("X1") && !on_disk.contains("Y2"),
            "{on_disk}"
        );
        assert_eq!(
            on_disk.as_bytes(),
            ledger_bytes(&text_free_projection(&rows)).as_slice()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_ledger_round_trips_what_ledger_bytes_writes() {
        let report = mrz_report_with_details(vec![
            detail("a", Some("a"), None),
            detail("b", Some("b"), Some(MissReason::Redacted)),
        ]);
        let rows = build_outcome_rows(&report);
        let bytes = ledger_bytes(&rows);
        let parsed = parse_ledger(&bytes).expect("well-formed ledger bytes parse");
        assert_eq!(parsed, rows);
    }

    // --- --replay-ocr-passes (ADR-0024, amendment 3) ------------------------

    const REPLAY_BASE: [&str; 4] = [
        "--real-specimens",
        "--mrz-only",
        "--replay-ocr-passes",
        "capture-dir",
    ];

    fn replay_args(extra: &[&str]) -> Vec<String> {
        REPLAY_BASE
            .iter()
            .chain(extra)
            .map(|s| s.to_string())
            .collect()
    }

    #[test]
    fn replay_is_off_by_default_and_parses_with_its_directory() {
        assert_eq!(Args::default().replay_ocr_passes, None);
        assert_eq!(
            parse_args(&argv(&["--real-specimens", "--mrz-only"]))
                .expect("parses")
                .replay_ocr_passes,
            None
        );
        let parsed = parse_args(&replay_args(&[])).expect("a valid replay");
        assert_eq!(parsed.replay_ocr_passes.as_deref(), Some("capture-dir"));
        let err = parse_args(&argv(&["--real-specimens", "--replay-ocr-passes"]))
            .err()
            .expect("needs a value");
        assert_eq!(err, "--replay-ocr-passes requires a directory");
    }

    #[test]
    fn replay_needs_real_specimens_and_mrz_only() {
        let err = parse_args(&argv(&["--mrz-only", "--replay-ocr-passes", "d"]))
            .err()
            .expect("refused without --real-specimens");
        assert!(err.contains("--real-specimens"), "{err}");
        let err = parse_args(&argv(&["--real-specimens", "--replay-ocr-passes", "d"]))
            .err()
            .expect("refused without --mrz-only");
        assert!(
            err.contains("--mrz-only") && err.contains("Tier 1 only"),
            "{err}"
        );
    }

    /// Each flag that would make a replay something else is refused, and the
    /// message names both the flag and the replay.
    #[test]
    fn replay_refuses_each_flag_that_contradicts_it() {
        for extra in [
            &["--include-private"][..],
            &["--include-local"],
            &["--write-baseline", "baseline.json"],
            &["--assert-baseline", "baseline.json"],
            &["--dump-ocr-passes"],
        ] {
            let err = parse_args(&replay_args(extra))
                .err()
                .unwrap_or_else(|| panic!("{extra:?} must be refused with --replay-ocr-passes"));
            assert!(
                err.starts_with(&format!(
                    "{} cannot be combined with --replay-ocr-passes",
                    extra[0]
                )),
                "{err}"
            );
        }
    }

    #[test]
    fn replay_allows_the_dump_flags_and_the_corpus_scoping_flags() {
        let parsed = parse_args(&replay_args(&[
            "--dump-ocr",
            "--dump-ocr-hits",
            "--limit",
            "5",
            "--format",
            "passport",
            "--include-covers",
            "--out",
            "artifacts/replay/real/report.json",
        ]))
        .expect("a replay is scoped and dumped like a live run");
        assert!(parsed.dump_ocr && parsed.dump_ocr_hits && parsed.include_covers);
        assert_eq!(parsed.limit, Some(5));
    }

    fn arms(chargrid: &str) -> BTreeMap<String, String> {
        let mut arms = ocr_arms_map(&synthpass_ocr::OcrArms::DEFAULT);
        arms.insert("chargrid".to_string(), chargrid.to_string());
        arms
    }

    fn capture_manifest(
        corpus: Option<&str>,
        ocr_arms: Option<&BTreeMap<String, String>>,
    ) -> Vec<u8> {
        let mut manifest = serde_json::json!({
            "flags": ["--real-specimens", "--dump-ocr-passes", "--out", "cap/real/report.json"],
            "git_commit": "0".repeat(40),
        });
        if let Some(corpus) = corpus {
            manifest["corpus_manifest_sha256"] = corpus.into();
        }
        if let Some(ocr_arms) = ocr_arms {
            manifest["ocr_arms"] = serde_json::json!(ocr_arms);
        }
        serde_json::to_vec_pretty(&manifest).unwrap()
    }

    #[test]
    fn the_ocr_arms_map_has_the_five_keys_the_manifest_always_had() {
        let map = ocr_arms_map(&synthpass_ocr::OcrArms::DEFAULT);
        assert_eq!(
            map.into_iter().collect::<Vec<_>>(),
            [
                ("chargrid".to_string(), "off".to_string()),
                ("order".to_string(), "default".to_string()),
                ("rotate".to_string(), "default".to_string()),
                ("skew".to_string(), "default".to_string()),
                ("texture".to_string(), "on".to_string()),
            ]
        );
    }

    #[test]
    fn a_matching_capture_manifest_is_named_hashed_and_its_arms_copied() {
        let corpus = "c".repeat(64);
        let bytes = capture_manifest(Some(&corpus), Some(&arms("off")));
        let source = replay_source_from("cap-run.json", &bytes, Some(&corpus), &arms("off"))
            .expect("the capture matches");
        assert_eq!(
            source,
            ReplaySource {
                run_manifest: "cap-run.json".to_string(),
                run_manifest_sha256: sha256_hex(&bytes),
                ocr_arms: arms("off"),
            }
        );
    }

    #[test]
    fn a_capture_measured_against_another_corpus_manifest_is_refused() {
        let bytes = capture_manifest(Some(&"a".repeat(64)), Some(&arms("off")));
        let err = replay_source_from("m.json", &bytes, Some(&"b".repeat(64)), &arms("off"))
            .expect_err("refused");
        assert!(err.contains("different samples/corpus.jsonl"), "{err}");
        assert!(
            err.contains(&"a".repeat(12)) && err.contains(&"b".repeat(12)),
            "{err}"
        );
        assert!(
            !err.contains(&"a".repeat(13)),
            "a short prefix, not the hash: {err}"
        );
        let err = replay_source_from("m.json", &bytes, None, &arms("off")).expect_err("refused");
        assert!(err.contains("cannot read samples/corpus.jsonl"), "{err}");
    }

    #[test]
    fn a_capture_manifest_that_says_too_little_or_is_unreadable_is_refused() {
        let corpus = "c".repeat(64);
        let err = replay_source_from(
            "m.json",
            &capture_manifest(None, Some(&arms("off"))),
            Some(&corpus),
            &arms("off"),
        )
        .expect_err("no corpus hash");
        assert!(err.contains("records no corpus_manifest_sha256"), "{err}");
        let err = replay_source_from(
            "m.json",
            &capture_manifest(Some(&corpus), None),
            Some(&corpus),
            &arms("off"),
        )
        .expect_err("no arms");
        assert!(err.contains("records no ocr_arms"), "{err}");
        let err = replay_source_from(
            "m.json",
            b"{ not json SECRET-TEXT",
            Some(&corpus),
            &arms("off"),
        )
        .expect_err("unparsable");
        assert!(err.contains("does not parse"), "{err}");
        assert!(!err.contains("SECRET-TEXT"), "{err}");
    }

    /// The report's per-provider `ocr_arms` comes from this process's
    /// environment, so a replay under other arms than the capture's would label
    /// the text with OCR that did not read it.
    #[test]
    fn a_replay_under_other_ocr_arms_than_the_capture_is_refused() {
        let corpus = "c".repeat(64);
        let bytes = capture_manifest(Some(&corpus), Some(&arms("on")));
        let err = replay_source_from("m.json", &bytes, Some(&corpus), &arms("off"))
            .expect_err("arms differ");
        assert!(err.contains("chargrid"), "{err}");
        assert!(err.contains("SYNTHPASS_OCR_*"), "{err}");
        assert!(replay_source_from("m.json", &bytes, Some(&corpus), &arms("on")).is_ok());
    }

    fn row_naming(manifest: Option<&str>) -> synthpass_bench::ocr_passes::OcrPassesRow {
        synthpass_bench::ocr_passes::OcrPassesRow {
            run_manifest: manifest.map(str::to_string),
            ..Default::default()
        }
    }

    #[test]
    fn a_capture_names_exactly_one_plain_run_manifest_file() {
        assert_eq!(
            capture_manifest_name(&[row_naming(Some("m.json")), row_naming(Some("m.json"))]),
            Ok("m.json")
        );
        for rows in [
            vec![],
            vec![row_naming(None)],
            vec![row_naming(Some("m.json")), row_naming(None)],
            vec![row_naming(Some("m.json")), row_naming(Some("n.json"))],
        ] {
            assert!(capture_manifest_name(&rows).is_err(), "{rows:?}");
        }
        for name in ["../m.json", "sub/m.json", "/etc/passwd", ".."] {
            assert!(
                capture_manifest_name(&[row_naming(Some(name))]).is_err(),
                "{name} is a path, not a file name"
            );
        }
    }

    #[test]
    fn the_capture_manifest_is_read_from_the_capture_directory() {
        let dir =
            std::env::temp_dir().join(format!("provider-bench-capture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let corpus = "c".repeat(64);
        let bytes = capture_manifest(Some(&corpus), Some(&arms("off")));
        std::fs::write(dir.join("cap-run.json"), &bytes).unwrap();
        let source = replay_source(
            &dir,
            &[row_naming(Some("cap-run.json"))],
            Some(&corpus),
            &arms("off"),
        )
        .expect("read and checked");
        assert_eq!(source.run_manifest, "cap-run.json");
        assert_eq!(source.run_manifest_sha256, sha256_hex(&bytes));
        let err = replay_source(
            &dir,
            &[row_naming(Some("gone.json"))],
            Some(&corpus),
            &arms("off"),
        )
        .expect_err("no such manifest");
        assert!(err.contains("gone.json"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn out_in_the_capture_directory_is_the_same_directory_and_a_new_one_is_not() {
        let dir =
            std::env::temp_dir().join(format!("provider-bench-samedir-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(same_directory(&dir, &dir.join(".")));
        assert!(same_directory(&dir, &dir));
        assert!(!same_directory(&dir, &std::env::temp_dir()));
        assert!(
            !same_directory(&dir, &dir.join("not-yet-created")),
            "a directory that does not exist is no one's capture"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A replay's manifest names the capture and copies its arms; a live run's
    /// carries no `replay_of` at all, so its bytes (and its hash-derived file
    /// name) are what they were before replay existed.
    #[test]
    fn a_replay_manifest_records_the_capture_and_a_live_one_does_not() {
        let dir =
            std::env::temp_dir().join(format!("provider-bench-replay-run-{}", std::process::id()));
        let flags = vec![
            "--real-specimens".to_string(),
            "--replay-ocr-passes".to_string(),
        ];
        let source = ReplaySource {
            run_manifest: "provider-bench-ocr-run-cap.json".to_string(),
            run_manifest_sha256: "d".repeat(64),
            // Not the environment's arms: the manifest must copy the capture's.
            ocr_arms: arms("on"),
        };
        let name = write_ocr_run_manifest(&repo_root(), &dir, &flags, 2, 1, Some(&source))
            .expect("write the replay manifest");
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join(&name)).unwrap()).unwrap();
        assert_eq!(
            manifest["replay_of"],
            serde_json::json!({
                "run_manifest": "provider-bench-ocr-run-cap.json",
                "sha256": "d".repeat(64),
            })
        );
        assert_eq!(manifest["ocr_arms"]["chargrid"], "on");
        assert_eq!(manifest["flags"], serde_json::json!(flags));

        let live = write_ocr_run_manifest(&repo_root(), &dir, &flags, 2, 1, None)
            .expect("write a live manifest");
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.join(&live)).unwrap()).unwrap();
        assert!(
            !manifest.as_object().unwrap().contains_key("replay_of"),
            "a live manifest has no replay_of key"
        );
        assert_eq!(
            manifest["ocr_arms"],
            serde_json::json!(ocr_arms_map(&synthpass_ocr::OcrArms::from_env()))
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
