//! The report and baseline types `bin/provider-bench.rs` serializes: the
//! on-disk shape of a `provider-bench` report JSON and of the committed
//! `knowledge/benchmarks/real-specimen-mrz-baseline.json`.
//!
//! Moved out of that binary's `main` (PR-4.1) so a future `bench-report`
//! binary can consume these types directly instead of them being trapped in
//! one binary's private module. This is a pure move: no field was renamed,
//! reordered, or given a different `#[serde(...)]` attribute.
//!
//! **Field order is JSON key order.** `#[derive(Serialize)]` on a struct
//! serializes fields in declaration order, so the order below *is* the
//! schema for a file already committed to the repository
//! (`real-specimen-mrz-baseline.json`) and read by CI's regression gate.
//! Reordering, renaming, or adding a field without `#[serde(default)]`/
//! `skip_serializing_if` is a file-format change, not a refactor.

use crate::miss_kind;
use crate::provider_bench::{
    AssertionBucket, DocumentDetail, ProviderReport, StrictNameHitRate, Tier1HitRate,
    UnsupportedAssertion,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize)]
pub struct CapabilityReport {
    pub deterministic: bool,
    pub vision: bool,
    pub cost: &'static str,
}

/// `field_match_rate`/`mean_cer`/each `PerFieldCer::mean_cer` are `null` in
/// the JSON report exactly when `labelled_documents == 0` (whole report) or
/// no labelled document had ground truth for that specific field (per-field)
/// — see `synthpass_bench::provider_bench::AccuracyStats`'s doc for why that
/// is reported as absent rather than a fabricated `0.0`.
///
/// Those top-level rates cover every labelled document and are kept for
/// continuity. `accepted_reads` (read quality) and `scored` (end-to-end) are
/// the populations to quote, each with its own document count (#564).
#[derive(Serialize)]
pub struct AccuracyReport {
    pub labelled_documents: usize,
    pub field_match_rate: Option<f64>,
    pub mean_cer: Option<f64>,
    pub per_field_cer: Vec<PerFieldCer>,
    pub accepted_reads: PopulationAccuracyReport,
    pub scored: PopulationAccuracyReport,
}

/// Mirrors `synthpass_bench::provider_bench::PopulationAccuracy` for JSON.
#[derive(Serialize)]
pub struct PopulationAccuracyReport {
    pub documents: usize,
    pub field_match_rate: Option<f64>,
    pub mean_cer: Option<f64>,
    pub per_field: Vec<FieldAccuracyReport>,
}

/// Mirrors `synthpass_bench::provider_bench::FieldAccuracy` for JSON.
#[derive(Serialize)]
pub struct FieldAccuracyReport {
    pub field: &'static str,
    pub documents: usize,
    pub match_rate: Option<f64>,
    pub mean_cer: Option<f64>,
}

impl From<crate::provider_bench::PopulationAccuracy> for PopulationAccuracyReport {
    fn from(population: crate::provider_bench::PopulationAccuracy) -> Self {
        Self {
            documents: population.documents,
            field_match_rate: population.field_match_rate,
            mean_cer: population.mean_cer,
            per_field: population
                .per_field
                .into_iter()
                .map(|field| FieldAccuracyReport {
                    field: field.field,
                    documents: field.documents,
                    match_rate: field.match_rate,
                    mean_cer: field.mean_cer,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct PerFieldCer {
    pub field: &'static str,
    pub mean_cer: Option<f64>,
    pub documents: usize,
}

/// Mirrors `synthpass_bench::provider_bench::UnsupportedAssertion` for JSON:
/// `serde`'s internally-tagged enum representation, so a report reader sees
/// either `{"status": "computed", "overall": {...}, ...}` or
/// `{"status": "not_applicable", "reason": "..."}` — never a bare number
/// that would look the same whether it was measured or skipped.
///
/// `with_mrz_anchor`/`without_mrz_anchor` are `null` when the corpus had no
/// document of that kind — always the case for `without_mrz_anchor` on the
/// synthetic corpus, since every generated document carries an MRZ.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum UnsupportedAssertionReport {
    Computed {
        overall: AssertionBucketReport,
        with_mrz_anchor: Option<AssertionBucketReport>,
        without_mrz_anchor: Option<AssertionBucketReport>,
    },
    NotApplicable {
        reason: &'static str,
    },
}

#[derive(Serialize)]
/// `rate` is `null` when `assertions_total == 0` — the provider answered
/// nothing over this subset, which is a different fact from a measured rate
/// of zero and must not serialize as one.
pub struct AssertionBucketReport {
    pub rate: Option<f64>,
    pub assertions_total: usize,
    pub documents: usize,
}

/// Per-document rows behind a provider's aggregates. Counts and field *names*
/// only — never an asserted value, which would put document content into a
/// report file (see `synthpass_bench::provider_bench::DocumentDetail`).
#[derive(Serialize)]
pub struct DocumentDetailReport {
    pub name: String,
    /// Exact samples-relative asset identity for real specimens.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset_id: Option<String>,
    pub mrz_found: bool,
    /// The document's resolved ICAO 9303 MRZ format ("TD1"/"TD2"/"TD3"/
    /// "MRVA"/"MRVB"), `null` when none could be resolved — see
    /// `synthpass_bench::provider_bench::DocumentDetail::mrz_format`'s doc
    /// for the synthetic-vs-real-specimen resolution rules.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mrz_format: Option<&'static str>,
    pub read_ok: bool,
    pub mrz_checksums_valid: bool,
    /// Stable machine-readable miss class (`miss_kind`'s output) — `null` on
    /// a genuine Tier-1 hit. Never the free-text `MissReason` itself: some
    /// variants carry an inner detail string (a parse/provider error
    /// message), which is aggregate-report noise, not report content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub miss_reason: Option<&'static str>,
    /// Observed state of every check digit on every parsed MRZ, keyed by
    /// `mrz::Field::as_str()`. Values are `true` (verified), `false` (failed),
    /// or `null` (not printed by this layout); absent only if no MRZ parsed.
    /// See `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_states: Option<BTreeMap<&'static str, Option<bool>>>,
    pub assertions_total: usize,
    pub assertions_unsupported: usize,
    pub unsupported_fields: Vec<&'static str>,
    /// `synthpass_bench::provider_bench::DocumentDetail::names_exact`
    /// passthrough — `null` when not scored (either name field is missing
    /// from this document's ground truth, or the read errored), never a
    /// fabricated `false`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub names_exact: Option<bool>,
    /// `crate::NameError::as_str()` — KIND ONLY, same discipline as
    /// `unsupported_fields` above. `null` both when `names_exact` is
    /// `Some(true)` and when it is `null`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name_error: Option<&'static str>,
    /// `synthpass_bench::provider_bench::DocumentDetail::field_correctness`,
    /// serialized as `{"<field name>": "exact" | "wrong" | "unread"}` for every
    /// field this document's ground truth defines (#574). **Field names and
    /// the three verdicts only, never a value.** The key is **absent** for a
    /// document without ground truth; older reports lack it too, which
    /// `tools/bench_ab_diff.py` reports as "not recorded". Report-only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_correctness: Option<BTreeMap<&'static str, &'static str>>,
    /// Wall-clock milliseconds of this document's OCR pass, the per-document
    /// cost a real-specimen run is made of. The provider-level `speed` block
    /// times `reader.read` alone, which for the deterministic `mrz` provider is
    /// microseconds; this is the number that says where a 40-minute run went
    /// (ADR-0010, step 5). Additive: older reports simply lack it.
    pub ocr_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_variant_id: Option<String>,
    /// `DocumentDetail::retry_damaged_recovery` passthrough. **Always
    /// serialized** — `null` when `None`, never omitted — so the #473 A/B
    /// diff (`tools/synth_ab_diff.py`) can rely on the key being present in
    /// every report this PR produces, whether or not it has a value.
    pub retry_damaged_recovery: Option<bool>,
    pub retry_budget_hit: bool,
    pub retry_stop: Option<String>,
    /// `DocumentDetail::chargrid` passthrough — `null` whenever
    /// `SYNTHPASS_OCR_CHARGRID` was `off` (every default run today),
    /// matching every other unset-arm field on this struct.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chargrid: Option<String>,
    /// `DocumentDetail::tier1_damaged_recovery` passthrough. **Always
    /// serialized**, same reasoning as `retry_damaged_recovery` above.
    pub tier1_damaged_recovery: Option<bool>,
    /// The line-1 selector's verdict on this document (#574). The selector is
    /// on by default, so this is **present on a default run** whenever Tier 1
    /// accepted a zone in scope. It is **absent** only under
    /// `SYNTHPASS_MRZ_LINE1_SELECT=off`, for a provider that makes no Tier-1
    /// read, and when Tier 1 accepted nothing to select on. **Text-free**:
    /// kinds, counts and pass ids only. Report-only; nothing gates on it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line1_selection: Option<Line1SelectionReport>,
}

/// Mirrors `synthpass_bench::provider_bench::Line1SelectionDetail` for JSON.
/// Key set and order pinned by a test. **Text-free** (ADR-0027): a verdict, a
/// reason, counts, pass ids and `PassTransform` labels.
#[derive(Serialize, Debug, PartialEq, Eq)]
pub struct Line1SelectionReport {
    /// `control` or `on`.
    pub arm: &'static str,
    /// `applied`, `proposed`, `unresolved`, `ambiguous` or `none`.
    pub verdict: &'static str,
    /// `out_of_scope`, `kept`, `no_candidate`, `issuer_unresolved`,
    /// `repeated_line` or `digit_in_name_field`; `null` for `applied`,
    /// `proposed` and `ambiguous`.
    pub reason: Option<&'static str>,
    pub eligible: usize,
    pub distinct: usize,
    /// Whether the proposal's names differ from the accepted read's.
    pub names_changed: bool,
    /// The OCR passes whose readings include the proposed line 1, in execution
    /// order. Empty without a pass trace (a replay, or `--dump-ocr-passes`) and
    /// unless the verdict is `applied` or `proposed`.
    pub source_passes: Vec<String>,
    /// Those passes' `PassTransform` labels, each once.
    pub source_transforms: Vec<String>,
}

impl From<crate::provider_bench::Line1SelectionDetail> for Line1SelectionReport {
    fn from(detail: crate::provider_bench::Line1SelectionDetail) -> Self {
        Self {
            arm: detail.arm,
            verdict: detail.verdict,
            reason: detail.reason,
            eligible: detail.eligible,
            distinct: detail.distinct,
            names_changed: detail.names_changed,
            source_passes: detail.source_passes,
            source_transforms: detail.source_transforms,
        }
    }
}

impl From<AssertionBucket> for AssertionBucketReport {
    fn from(b: AssertionBucket) -> Self {
        Self {
            rate: b.rate,
            assertions_total: b.assertions_total,
            documents: b.documents,
        }
    }
}

impl From<UnsupportedAssertion> for UnsupportedAssertionReport {
    fn from(u: UnsupportedAssertion) -> Self {
        match u {
            UnsupportedAssertion::Computed {
                overall,
                with_mrz_anchor,
                without_mrz_anchor,
            } => Self::Computed {
                overall: overall.into(),
                with_mrz_anchor: with_mrz_anchor.map(Into::into),
                without_mrz_anchor: without_mrz_anchor.map(Into::into),
            },
            UnsupportedAssertion::NotApplicable { reason } => Self::NotApplicable { reason },
        }
    }
}

/// Mirrors `synthpass_bench::provider_bench::Tier1HitRate` for JSON: either
/// `{"status": "computed", "rate": ...}` or `{"status": "not_applicable",
/// "reason": "...", "rate": null}` — never a bare number that would look identical
/// whether it was measured or skipped for a nondeterministic provider or empty population.
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Tier1HitRateReport {
    Computed {
        rate: f64,
    },
    NotApplicable {
        reason: &'static str,
        rate: Option<f64>,
    },
}

impl From<Tier1HitRate> for Tier1HitRateReport {
    fn from(t: Tier1HitRate) -> Self {
        match t {
            Tier1HitRate::Computed(rate) => Self::Computed { rate },
            Tier1HitRate::NotApplicable { reason } => Self::NotApplicable { reason, rate: None },
        }
    }
}

/// Mirrors `synthpass_bench::provider_bench::StrictNameHitRate` for JSON, the
/// same tagged shape as `Tier1HitRateReport` above and for the same reason:
/// a bare number would look identical whether it was measured or skipped
/// (no deterministic provider, or no document in the scored Tier-1
/// population has ground truth for both name fields — see
/// `StrictNameHitRate`'s doc).
///
/// Two rates, two denominators, two names — see `StrictNameHitRate`'s doc
/// for why: `strict_tier1_hit_rate` divides by `name_scorable_documents`
/// (every name-scorable document in the scored Tier-1 population, hit or
/// not); `names_exact_among_hits` divides by `name_scorable_hits` (the
/// narrower population that is *also* a Tier-1 hit).
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum StrictNameHitRateReport {
    Computed {
        strict_hits: usize,
        name_scorable_documents: usize,
        name_scorable_hits: usize,
        strict_tier1_hit_rate: f64,
        names_exact_among_hits: Option<f64>,
    },
    NotApplicable {
        reason: &'static str,
        strict_tier1_hit_rate: Option<f64>,
        names_exact_among_hits: Option<f64>,
    },
}

impl From<StrictNameHitRate> for StrictNameHitRateReport {
    fn from(s: StrictNameHitRate) -> Self {
        match s {
            StrictNameHitRate::Computed {
                strict_hits,
                name_scorable_documents,
                name_scorable_hits,
                strict_tier1_hit_rate,
                names_exact_among_hits,
            } => Self::Computed {
                strict_hits,
                name_scorable_documents,
                name_scorable_hits,
                strict_tier1_hit_rate,
                names_exact_among_hits,
            },
            StrictNameHitRate::NotApplicable { reason } => Self::NotApplicable {
                reason,
                strict_tier1_hit_rate: None,
                names_exact_among_hits: None,
            },
        }
    }
}

/// Mirrors `synthpass_ocr::OcrArms` for JSON — every `SYNTHPASS_OCR_*`
/// measurement knob this run's OCR engine read, so a report is self-
/// describing about which arm produced its numbers.
#[derive(Serialize)]
pub struct OcrArmsReport {
    pub texture: &'static str,
    pub order: &'static str,
    pub rotate: &'static str,
    pub skew: &'static str,
    pub chargrid: &'static str,
}

impl From<synthpass_ocr::OcrArms> for OcrArmsReport {
    fn from(a: synthpass_ocr::OcrArms) -> Self {
        Self {
            texture: a.texture,
            order: a.order,
            rotate: a.rotate,
            skew: a.skew,
            chargrid: a.chargrid,
        }
    }
}

/// The two `.rten` model file paths a run actually loaded — a run-level fact
/// next to the run's other provenance (`OcrArmsReport`, `mrz_class_sweep_arm`),
/// not a per-document field and not part of `synthpass_ocr::OcrArms` (the
/// measurement-arm knob set). Exists because both bench binaries now resolve
/// `SYNTHPASS_OCR_MODEL_DIR` (issue #541) instead of always reading the build
/// tree's own `repo_root()`: a report with no record of which files a run
/// actually opened could not tell "the models on disk" from "the tree's own
/// build-time copy."
///
/// Always serialized. The reports that carry it are `Serialize`-only; their
/// readers — `tools/` by key, [`crate::bench_report::GateReport`] by ignoring
/// unknown fields — tolerate both a report without it and one with it.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
pub struct ModelPathsReport {
    pub detection: String,
    pub recognition: String,
}

impl ModelPathsReport {
    /// `std::fs::canonicalize` of each path a binary actually passed to
    /// `NativeOcr::load`, falling back to that same path exactly as given
    /// when canonicalization fails (e.g. it does not exist — `NativeOcr::load`
    /// itself already failed the whole run by the time a report would be
    /// written, but a test can still exercise this half independently).
    pub fn resolve(detection_path: &std::path::Path, recognition_path: &std::path::Path) -> Self {
        Self {
            detection: canonicalize_or_given(detection_path),
            recognition: canonicalize_or_given(recognition_path),
        }
    }
}

fn canonicalize_or_given(path: &std::path::Path) -> String {
    std::fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .display()
        .to_string()
}

#[derive(Serialize)]
pub struct SpeedReport {
    pub mean_ms: u128,
    pub p50_ms: u128,
    pub p95_ms: u128,
}

#[derive(Serialize)]
pub struct JsonValidityReport {
    pub repair_fallbacks: u64,
    pub documents: usize,
    /// `None` when no document was processed, never a fabricated zero.
    pub repair_fallback_rate: Option<f64>,
}

/// One provider's full row in a `provider-bench` report — capability,
/// accuracy, speed, and every other measured axis for one registered
/// [`synthpass_die::FieldReader`], built from its internal
/// [`ProviderReport`] via [`From`].
#[derive(Serialize)]
pub struct ProviderRow {
    pub provider_id: String,
    pub documents: usize,
    pub capability: CapabilityReport,
    pub accuracy: AccuracyReport,
    pub speed: SpeedReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_validity: Option<JsonValidityReport>,
    pub unsupported_assertion: UnsupportedAssertionReport,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub declared_resident_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub measured_rss_delta_bytes: Option<i64>,
    /// Always serialized, regardless of `--verbose` — the flag governs
    /// terminal noise, not what the report records. A run that took an hour
    /// should not have to be repeated because the per-document detail was
    /// only printed and never saved.
    pub documents_detail: Vec<DocumentDetailReport>,
    /// Genuine Tier-1 hit rate (MRZ found, checksums valid, document number
    /// matches when labelled) — see
    /// `synthpass_bench::provider_bench::ProviderReport::tier1_hit_rate`'s
    /// doc for why this is a different number from `read_ok`/nothing above.
    /// `NotApplicable` for a nondeterministic provider or an empty scored
    /// population — see `Tier1HitRate`'s doc.
    pub tier1_hit_rate: Tier1HitRateReport,
    /// Two name-accuracy rates over two different denominators — see
    /// `synthpass_bench::provider_bench::StrictNameHitRate`'s doc for
    /// `strict_tier1_hit_rate` vs `names_exact_among_hits`.
    pub strict_tier1_hit_rate: StrictNameHitRateReport,
    /// `synthpass_bench::provider_bench::ProviderReport::ocr_arms`
    /// passthrough — this run's `SYNTHPASS_OCR_*` configuration.
    pub ocr_arms: OcrArmsReport,
    /// How many `checksum_failed_specimen` documents this provider read a
    /// checksum-**valid** MRZ off — see
    /// [`checksum_valid_reads_on_failed_specimens`]. The printed zone fails
    /// its own check digits, so a read that verifies was manufactured by the
    /// parser (issue #443). **Report-only**: the document stays in its
    /// off-denominator bucket and no gate reads this count yet.
    pub checksum_valid_on_failed_specimen: usize,
    /// How many documents got each line-1 selector verdict (#574):
    /// `applied`, `proposed`, `unresolved`, `ambiguous` and `none`, every key
    /// present, over the documents whose `line1_selection` is recorded.
    /// **Absent** when none is (the arm is `off`, the provider makes no Tier-1
    /// read, or nothing was accepted). Report-only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line1_selection_counts: Option<BTreeMap<&'static str, usize>>,
}

/// The `checksum_failed_specimen` documents whose read nonetheless passed
/// every ICAO check digit, by name, in corpus order.
///
/// A specimen lands in `checksum_failed_specimen` because its *printed* zone
/// fails its own check digits (`specimen_nonconforming`), and that rung files
/// it there whatever OCR returned. So a checksum-valid read cannot become
/// `false_positive_mrz` and the outcome ledger, which compares only the
/// bucket, never sees it. A read that verifies on a zone that cannot verify
/// was built by the parser, not read off the page: Argentina 2026's
/// `damaged_pass` inserted a `V` at cell 0 and turned a TD3 passport into a
/// weaker-checked MRV-A visa zone (#443's step 1). This is the real-specimen
/// twin of `synthpass-bench`'s wrong accepts (#453). Report-only for now; the
/// failing step and its re-bless come in a later change.
pub fn checksum_valid_reads_on_failed_specimens(details: &[DocumentDetail]) -> Vec<&str> {
    details
        .iter()
        .filter(|d| {
            d.mrz_checksums_valid
                && d.miss_reason.as_ref().map(miss_kind) == Some("checksum_failed_specimen")
        })
        .map(|d| d.name.as_str())
        .collect()
}

impl From<ProviderReport> for ProviderRow {
    fn from(r: ProviderReport) -> Self {
        let checksum_valid_on_failed_specimen =
            checksum_valid_reads_on_failed_specimens(&r.documents_detail).len();
        let line1_selection_counts =
            crate::provider_bench::line1_selection_counts(&r.documents_detail);
        Self {
            provider_id: r.provider_id,
            documents: r.documents,
            capability: CapabilityReport {
                deterministic: r.capability.deterministic,
                vision: r.capability.vision,
                cost: r.capability.cost,
            },
            accuracy: AccuracyReport {
                labelled_documents: r.accuracy.labelled_documents,
                field_match_rate: r.accuracy.field_match_rate,
                mean_cer: r.accuracy.mean_cer,
                per_field_cer: r
                    .accuracy
                    .per_field_cer
                    .into_iter()
                    .map(|(field, mean_cer, documents)| PerFieldCer {
                        field,
                        mean_cer,
                        documents,
                    })
                    .collect(),
                accepted_reads: r.accuracy.accepted_reads.into(),
                scored: r.accuracy.scored.into(),
            },
            speed: SpeedReport {
                mean_ms: r.speed.mean.as_millis(),
                p50_ms: r.speed.p50.as_millis(),
                p95_ms: r.speed.p95.as_millis(),
            },
            json_validity: r.json_validity.map(|j| JsonValidityReport {
                repair_fallback_rate: (j.documents > 0)
                    .then(|| j.repair_fallbacks as f64 / j.documents as f64),
                repair_fallbacks: j.repair_fallbacks,
                documents: j.documents,
            }),
            unsupported_assertion: r.unsupported_assertion.into(),
            declared_resident_bytes: r.declared_resident_bytes,
            measured_rss_delta_bytes: r.measured_rss_delta_bytes,
            documents_detail: r
                .documents_detail
                .into_iter()
                .map(|d| DocumentDetailReport {
                    name: d.name,
                    asset_id: d.asset_id,
                    mrz_found: d.mrz_found,
                    mrz_format: d.mrz_format,
                    read_ok: d.read_ok,
                    mrz_checksums_valid: d.mrz_checksums_valid,
                    miss_reason: d.miss_reason.as_ref().map(miss_kind),
                    check_states: d.check_states,
                    assertions_total: d.assertions_total,
                    assertions_unsupported: d.assertions_unsupported,
                    unsupported_fields: d.unsupported_fields,
                    names_exact: d.names_exact,
                    name_error: d.name_error,
                    field_correctness: d.field_correctness.map(|fields| {
                        fields
                            .into_iter()
                            .map(|(field, correctness)| (field, correctness.as_str()))
                            .collect()
                    }),
                    ocr_ms: d.ocr_elapsed.as_millis(),
                    retry_variant_id: d.retry_variant_id,
                    retry_damaged_recovery: d.retry_damaged_recovery,
                    retry_budget_hit: d.retry_budget_hit,
                    retry_stop: d.retry_stop,
                    chargrid: d.chargrid,
                    tier1_damaged_recovery: d.tier1_damaged_recovery,
                    line1_selection: d.line1_selection.map(Into::into),
                })
                .collect(),
            tier1_hit_rate: r.tier1_hit_rate.into(),
            strict_tier1_hit_rate: r.strict_tier1_hit_rate.into(),
            ocr_arms: r.ocr_arms.into(),
            checksum_valid_on_failed_specimen,
            line1_selection_counts,
        }
    }
}

/// The top-level `provider-bench` report JSON: one run's provenance
/// (source, profile, count, the measured `mrz` class-sweep arm) plus one
/// [`ProviderRow`] per registered provider.
#[derive(Serialize)]
pub struct Report {
    pub timestamp_unix: u64,
    /// "synthetic-corpus" or "real-specimens" — which of `run_provider_bench`/
    /// `run_provider_bench_real` produced `providers` below, since the two
    /// populate `profile`/`count`/`seed_start` differently (a real specimen
    /// has no capture profile and no seed).
    pub source: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<&'static str>,
    /// The `--format` class this run was restricted to, `real-specimens`
    /// only — absent means every `samples/` class was included.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<&'static str>,
    pub count: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_start: Option<u64>,
    /// The `mrz` parse arm this run actually measured, as the binary
    /// resolved it -- `off`, `on` or `control`, from
    /// `SYNTHPASS_MRZ_CLASS_SWEEP`.
    ///
    /// Recorded because an unrecognised value falls back to `off`
    /// **silently**: without this, a three-arm A/B whose variable was
    /// misspelled in one arm produces two identical populations and reads as
    /// a clean null. Quote this field, never the variable you believe you
    /// set.
    pub mrz_class_sweep_arm: &'static str,
    /// The line-1 selector arm this run measured (#574), as the binary
    /// resolved it -- `off`, `control` or `on` (the default), from
    /// `SYNTHPASS_MRZ_LINE1_SELECT`. **Always serialized**, beside
    /// `mrz_class_sweep_arm` and for the same reason: an unrecognised value
    /// falls back to the default, `on`, silently, so quote this field, never
    /// the variable you believe you set.
    pub mrz_line1_select_arm: &'static str,
    /// The `text-detection.rten`/`text-recognition.rten` paths this run
    /// actually loaded — one `NativeOcr` instance shared across every
    /// provider row, so this is a single run-level fact, not per-provider.
    /// Always serialized; see [`ModelPathsReport`].
    pub model_paths: ModelPathsReport,
    pub providers: Vec<ProviderRow>,
}

/// One line of the outcome ledger (`real-specimen-outcomes.jsonl`): the
/// per-document evidence behind one CI run's `RealSpecimenSnapshot`, kept
/// alongside the aggregate baseline so a finding derived from it stays
/// re-derivable after the `real-specimen-gate-report` CI artifact expires
/// (the workflow keeps that artifact for 90 days, `retention-days: 90` since
/// #416, and the committed ledger does not expire).
///
/// Field order is the JSON key order — `#[derive(Serialize)]` on a struct
/// serializes fields in declaration order, so this order **is** the schema;
/// do not reorder the fields without treating that as a format change.
/// `miss_reason` is the full [`MissReason`](crate::MissReason) [`Display`](std::fmt::Display)
/// string (may carry a parse/provider error message, and for a
/// `document_number_mismatch` **both document numbers**: real-specimen text
/// never leaves the machine or the committed ledger, so `provider-bench`'s
/// assert run uploads a text-free projection with each `miss_reason` reduced to
/// its kind, see `text_free_projection` in `provider-bench.rs`); `outcome` is the
/// stable machine-readable class ([`miss_kind`] or `"hit"`) — the same
/// hit-vs-noise split [`DocumentDetailReport::miss_reason`] draws for the
/// report JSON, deliberately not applied here since this file exists
/// specifically to keep the detail an aggregate report drops.
///
/// Every field is always present (an absent optional serializes as `null`,
/// never an omitted key): the ledger is meant to be diffed line-by-line and
/// joined key-by-key across runs, and an omitted-vs-`null` distinction would
/// only make that harder for no benefit — unlike the report JSON's
/// `DocumentDetailReport`, nothing here is trying to keep an old report
/// backward-compatible.
///
/// `mrz_format`/`name_error` are owned `String`, unlike
/// [`DocumentDetail`]'s `&'static str` they are copied from: a derived
/// `Deserialize` for a `&'static str` field can only borrow from genuinely
/// `'static` input, which the bytes `parse_ledger` reads back off disk never
/// are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OutcomeRow {
    pub asset_id: Option<String>,
    pub name: String,
    pub outcome: String,
    pub miss_reason: Option<String>,
    pub mrz_format: Option<String>,
    pub mrz_found: bool,
    pub mrz_checksums_valid: bool,
    pub names_exact: Option<bool>,
    pub name_error: Option<String>,
    pub ocr_ms: u128,
    pub retry_variant_id: Option<String>,
    pub retry_budget_hit: bool,
    pub retry_stop: Option<String>,
}

impl OutcomeRow {
    /// The sort/join key: `asset_id` when present (every real specimen has
    /// one), falling back to `name` (the synthetic corpus's seed-derived
    /// identity, which never sets `asset_id`) — the same fallback the ledger
    /// is sorted by and `diff_outcomes` joins on.
    pub fn sort_key(&self) -> &str {
        self.asset_id.as_deref().unwrap_or(&self.name)
    }
}

impl From<&DocumentDetail> for OutcomeRow {
    fn from(d: &DocumentDetail) -> Self {
        Self {
            asset_id: d.asset_id.clone(),
            name: d.name.clone(),
            outcome: d
                .miss_reason
                .as_ref()
                .map(miss_kind)
                .unwrap_or("hit")
                .to_string(),
            miss_reason: d.miss_reason.as_ref().map(ToString::to_string),
            mrz_format: d.mrz_format.map(str::to_string),
            mrz_found: d.mrz_found,
            mrz_checksums_valid: d.mrz_checksums_valid,
            names_exact: d.names_exact,
            name_error: d.name_error.map(str::to_string),
            ocr_ms: d.ocr_elapsed.as_millis(),
            retry_variant_id: d.retry_variant_id.clone(),
            retry_budget_hit: d.retry_budget_hit,
            retry_stop: d.retry_stop.clone(),
        }
    }
}

/// The miss kinds that sit outside the Tier-1 hit-rate denominator: documents
/// that cannot yield a hit however good the pipeline gets, so counting them as
/// failures measures the corpus rather than the reader.
///
/// Must stay identical to the filter in `provider_bench::run_prepped`, which
/// computes the same number for `ProviderReport::tier1_hit_rate`.
pub const OFF_DENOMINATOR_KINDS: &[&str] = &[
    // The zone is physically blacked out by whoever published the specimen.
    "redacted_mrz",
    // The document has no machine-readable zone at all — an ID-card front, a
    // border pass, a driving-license face.
    "no_mrz_expected",
    // The *printed* zone fails its own ICAO check digits (TEMPLATE / ÖRNEK /
    // VZOR / all-zeros), so a byte-perfect read still fails. The 2026-09-08
    // `checksum_failed` writeup identified 16 of these and concluded they
    // "belong outside the denominator"; this is that conclusion, implemented.
    "checksum_failed_specimen",
];

/// The miss buckets whose growth is a Tier-1 regression — i.e. every kind that
/// is *inside* the denominator, where a miss is genuinely the pipeline's.
/// Everything in [`OFF_DENOMINATOR_KINDS`] is excluded on purpose: those counts
/// moving is a corpus change, not a parser regression, and each warns instead.
/// `false_positive_mrz` is included and is the most serious of them: it means a
/// checksum-valid MRZ was returned for a document that has none.
pub const REGRESSION_BUCKETS: &[&str] = &[
    "checksum_failed",
    "no_mrz_found",
    "ocr_error",
    "document_number_mismatch",
    "false_positive_mrz",
    // A document number whose first cell is the filler is refused outright
    // (`mrz::MrzError::LeadingFiller`, #536) rather than merely failing its
    // check digit — a distinct, genuine miss, so its growth is a regression
    // exactly like `checksum_failed`'s.
    "document_number_leading_filler",
];

/// The ADR-0013 strict-name counts for one real-specimen run: `strict_hits`
/// Tier-1 hits that also read both name fields exactly, over
/// `name_scorable_documents` (the denominator of `strict_tier1_hit_rate`) and
/// `name_scorable_hits` (the denominator of `names_exact_among_hits`) — the
/// same three counts [`crate::provider_bench::StrictNameHitRate::Computed`]
/// carries, kept as counts rather than the two derived rates, matching this
/// file's own `tier1_hits`/`scored` convention (rates are recomputed by
/// whoever reads the baseline, never stored pre-divided).
///
/// **Report-only per ADR-0013:** carried on [`RealSpecimenSnapshot`] and
/// [`RealSpecimenBaseline`] so it reaches the committed baseline, the
/// `rebless.py` diff tooling and `README.md`'s live block, but
/// `check_baseline` never fails a comparison on it — "the headline table
/// publishes the strict rate next to the Tier-1 rate only once a CI run has
/// measured it ... Gating on it is a separate decision"
/// (`knowledge/decisions/ADR-0013-names-are-scored-against-mrz-form-truth.md`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrictNamesBaseline {
    /// Tier-1 hits whose read also matched both name fields exactly.
    pub strict_hits: usize,
    /// Documents in the scored Tier-1 population whose ground truth carries
    /// both `surname` and `given_names` — `strict_tier1_hit_rate`'s
    /// denominator.
    pub name_scorable_documents: usize,
    /// Of `name_scorable_documents`, the ones that were also a Tier-1 hit —
    /// `names_exact_among_hits`'s denominator.
    pub name_scorable_hits: usize,
}

/// The measured Tier-1 facts for the deterministic `mrz` provider over one run —
/// what `--write-baseline` records (via [`RealSpecimenBaseline`]) and
/// `--assert-baseline` checks against.
#[derive(Debug)]
pub struct RealSpecimenSnapshot {
    /// Documents that OCR'd successfully and reached the reader loop.
    pub documents: usize,
    /// Denominator of the Tier-1 hit rate: `documents` minus the two
    /// populations that cannot answer the question. `redacted_mrz` is a zone
    /// blacked out by whoever published the specimen; `no_mrz_expected` is a
    /// document that never had one (an ID-card front, a border pass, a
    /// driving-license face). Neither can prove or disprove detection accuracy,
    /// and until 2026-09-09 the second was not excluded — 42 correct refusals
    /// sat in this denominator as `no_mrz_found` failures, which put the
    /// headline rate 11.6 points low and roughly doubled the apparent size of
    /// the detection problem `ADR-0008` exists to attack.
    pub scored: usize,
    /// Documents that were a genuine Tier-1 hit (MRZ found, checksums valid,
    /// document number matches when labelled).
    pub tier1_hits: usize,
    /// Every miss kind that occurred, with its document count.
    pub by_miss_kind: BTreeMap<String, usize>,
    /// ADR-0013 strict-name counts, when the `mrz` provider's
    /// `strict_tier1_hit_rate` was [`StrictNameHitRate::Computed`] — `None`
    /// when it was `NotApplicable` (no name-scorable document in this run's
    /// scored population; see that variant's doc for why `0.0` would be a
    /// fabrication here). Report-only — see [`StrictNamesBaseline`]'s doc.
    pub strict_names: Option<StrictNamesBaseline>,
    /// `documents - scored`, restated as a stored count rather than left to
    /// `off_denominator()` arithmetic: the population a false accept could
    /// come from — every document whose zone is absent, redacted, or
    /// non-conforming, where any checksum-valid MRZ returned is by
    /// construction a hallucination, not a correct read. Always computable
    /// from `by_miss_kind`, so `from_reports` always sets it (never `None`);
    /// it is `RealSpecimenBaseline::refusal_population` that stays optional,
    /// for a committed baseline written before this field existed.
    pub refusal_population: usize,
    /// [`RealSpecimenBaseline::outcomes_sha256`]'s value for *this* run —
    /// always `None` straight out of [`Self::from_reports`], since the
    /// ledger bytes it would hash do not exist until
    /// `write_baseline_and_ledger` writes them. Set by that function just
    /// before it writes the baseline JSON.
    pub outcomes_sha256: Option<String>,
}

impl RealSpecimenSnapshot {
    /// Build from the `"mrz"` provider's report. `None` when that provider is
    /// absent (a run that registered only the LLM — never the case under
    /// `--mrz-only`, and normally the full catalog has both).
    pub fn from_reports(reports: &[ProviderReport]) -> Option<Self> {
        let mrz = reports.iter().find(|r| r.provider_id == "mrz")?;
        // Every known bucket starts at `0`, not absent: a fresh `by_miss_kind`
        // built purely from `.entry(kind).or_default()` (as below) would never
        // record a zero, and the committed baseline JSON would then have no way
        // to say "measured zero false positives" versus "never checked" — see
        // `false_positive_mrz`'s doc on `REGRESSION_BUCKETS`. `check_baseline`'s
        // own reads stay `unwrap_or(0)` regardless, so an older baseline that
        // predates this pre-fill still compares correctly.
        let mut by_miss_kind: BTreeMap<String, usize> = REGRESSION_BUCKETS
            .iter()
            .chain(OFF_DENOMINATOR_KINDS)
            .map(|k| (k.to_string(), 0))
            .collect();
        let mut tier1_hits = 0usize;
        for d in &mrz.documents_detail {
            match &d.miss_reason {
                None => tier1_hits += 1,
                Some(reason) => {
                    *by_miss_kind
                        .entry(miss_kind(reason).to_string())
                        .or_default() += 1
                }
            }
        }
        let documents = mrz.documents_detail.len();
        let off_denominator = OFF_DENOMINATOR_KINDS
            .iter()
            .filter_map(|k| by_miss_kind.get(*k))
            .sum::<usize>();
        // Reuses the aggregate the report already computed rather than
        // re-deriving name-scorability from `documents_detail` a second time
        // — `StrictNameHitRate::Computed`'s three counts are exactly
        // `StrictNamesBaseline`'s fields.
        let strict_names = match &mrz.strict_tier1_hit_rate {
            StrictNameHitRate::Computed {
                strict_hits,
                name_scorable_documents,
                name_scorable_hits,
                ..
            } => Some(StrictNamesBaseline {
                strict_hits: *strict_hits,
                name_scorable_documents: *name_scorable_documents,
                name_scorable_hits: *name_scorable_hits,
            }),
            StrictNameHitRate::NotApplicable { .. } => None,
        };
        Some(Self {
            documents,
            scored: documents - off_denominator,
            tier1_hits,
            by_miss_kind,
            strict_names,
            refusal_population: off_denominator,
            outcomes_sha256: None,
        })
    }

    /// Documents scored out because no pipeline could have produced a hit —
    /// the total across [`OFF_DENOMINATOR_KINDS`]. Always equal to
    /// `self.refusal_population`; kept as a method (rather than reusing the
    /// field directly everywhere) because most call sites want "the
    /// off-denominator total" as a derived fact, not the specific
    /// baseline-schema field it also happens to back.
    pub fn off_denominator(&self) -> usize {
        OFF_DENOMINATOR_KINDS
            .iter()
            .filter_map(|k| self.by_miss_kind.get(*k))
            .sum()
    }
}

/// The committed baseline file
/// (`knowledge/benchmarks/real-specimen-mrz-baseline.json`): a
/// [`RealSpecimenSnapshot`]'s counts plus provenance and a `tolerance`.
#[derive(Debug, Serialize, Deserialize)]
pub struct RealSpecimenBaseline {
    /// What this file is and how to regenerate it. Ignored by the comparison.
    pub note: String,
    /// The commit the baseline run measured against (`GITHUB_SHA` on CI).
    pub measured_on_ci_sha: String,
    pub measured_date: String,
    /// Exact `samples-data` commit used by the measured run. CI resolves the
    /// baseline pin before materializing the corpus; this field is the
    /// authoritative DATA revision for reproducing that population.
    pub samples_data_sha: String,
    pub documents: usize,
    pub scored: usize,
    pub tier1_hits: usize,
    /// Slack for run-to-run OCR-inference variance across CI runner hardware:
    /// subtracted from the HIT-count floor and added to every "may not grow"
    /// bucket bound. `0` today; raise it (a one-line reviewed change) only if
    /// runs prove flaky.
    pub tolerance: usize,
    pub by_miss_kind: BTreeMap<String, usize>,
    /// ADR-0013 strict-name counts — report-only, see [`StrictNamesBaseline`].
    /// Absent (rather than `null`) on an older baseline that never measured
    /// names, and omitted on write for the same reason: `rebless.py`'s
    /// `flatten_baseline` and `scripts/check-headline-numbers.sh` both need to
    /// tell "not measured" apart from "measured as zero", which an
    /// always-present `null` would not preserve as cleanly as a genuinely
    /// missing key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strict_names: Option<StrictNamesBaseline>,
    /// Lowercase hex SHA-256 of the outcome ledger
    /// (`real-specimen-outcomes.jsonl`, ledger bytes) committed alongside
    /// this baseline — `--assert-baseline` fails the gate if the ledger next
    /// to the baseline no longer hashes to this. Absent on a baseline written
    /// before this field existed, in which case no ledger is expected and
    /// none of the ledger checks run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcomes_sha256: Option<String>,
    /// The population a false accept could come from: documents whose zone is
    /// absent, redacted, or non-conforming, where any checksum-valid MRZ
    /// returned is a hallucination — `documents - scored`. Report-only, same
    /// discipline as `strict_names`: absent (not `0`) on an older baseline
    /// that never recorded it, so `rebless.py` and
    /// `scripts/check-headline-numbers.sh` can tell "not measured" apart from
    /// "measured as zero".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal_population: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider_bench::{AccuracyStats, CapabilitySnapshot, SpeedStats};
    use crate::MissReason;
    use std::time::Duration;

    #[test]
    fn old_baseline_json_without_strict_names_round_trips_without_the_key() {
        let json = r#"{
            "note": "n",
            "measured_on_ci_sha": "abc",
            "measured_date": "2026-09-16",
            "samples_data_sha": "def",
            "documents": 10,
            "scored": 8,
            "tier1_hits": 7,
            "tolerance": 0,
            "by_miss_kind": {"checksum_failed": 1}
        }"#;
        let baseline: RealSpecimenBaseline =
            serde_json::from_str(json).expect("an old baseline with no strict_names key");
        assert_eq!(baseline.strict_names, None);
        let text = serde_json::to_string_pretty(&baseline).expect("serialize");
        assert!(
            !text.contains("strict_names"),
            "an absent field must not round-trip back in: {text}"
        );
    }

    /// Issue #541: falls back to the path exactly as given when it does not
    /// exist (canonicalization fails) — the case a report for a failed run
    /// would never actually reach (`NativeOcr::load` errors first), but the
    /// half `ModelPathsReport::resolve` owns must still behave correctly on
    /// its own.
    #[test]
    fn model_paths_report_falls_back_to_the_given_path_when_canonicalize_fails() {
        let missing = std::path::Path::new("/nonexistent/synthpass-541/text-detection.rten");
        let resolved = ModelPathsReport::resolve(missing, missing);
        assert_eq!(resolved.detection, missing.display().to_string());
        assert_eq!(resolved.recognition, missing.display().to_string());
    }

    /// A path that does exist is canonicalized, not merely echoed back.
    #[test]
    fn model_paths_report_canonicalizes_an_existing_path() {
        let dir = std::env::temp_dir();
        let resolved = ModelPathsReport::resolve(&dir, &dir);
        let expected = std::fs::canonicalize(&dir).unwrap().display().to_string();
        assert_eq!(resolved.detection, expected);
        assert_eq!(resolved.recognition, expected);
    }

    /// Always serialized — never omitted, matching the doc on the field in
    /// `provider-bench`'s top-level `Report`.
    #[test]
    fn model_paths_report_always_serializes_both_keys() {
        let resolved = ModelPathsReport {
            detection: "/models/text-detection.rten".to_string(),
            recognition: "/models/text-recognition.rten".to_string(),
        };
        let json = serde_json::to_value(&resolved).expect("serialize");
        assert_eq!(json["detection"], "/models/text-detection.rten");
        assert_eq!(json["recognition"], "/models/text-recognition.rten");
    }

    /// A freshly-built `provider-bench` report always carries `model_paths`
    /// in its JSON — the counterpart to the "old report still parses" test
    /// below.
    #[test]
    fn provider_bench_report_always_serializes_model_paths() {
        let report = Report {
            timestamp_unix: 0,
            source: "synthetic-corpus",
            profile: None,
            format: None,
            count: 1,
            seed_start: None,
            mrz_class_sweep_arm: "off",
            mrz_line1_select_arm: "off",
            model_paths: ModelPathsReport {
                detection: "/models/text-detection.rten".to_string(),
                recognition: "/models/text-recognition.rten".to_string(),
            },
            providers: Vec::new(),
        };
        let json = serde_json::to_value(&report).expect("serialize report");
        assert_eq!(
            json["model_paths"]["detection"],
            "/models/text-detection.rten"
        );
        assert_eq!(
            json["model_paths"]["recognition"],
            "/models/text-recognition.rten"
        );
    }

    /// The crate's own reader of this JSON, `bench-report`'s `GateReport`,
    /// still parses a report that carries the new `model_paths` key.
    #[test]
    fn a_report_carrying_model_paths_still_parses_as_a_gate_report() {
        let report = Report {
            timestamp_unix: 0,
            source: "synthetic-corpus",
            profile: None,
            format: None,
            count: 1,
            seed_start: None,
            mrz_class_sweep_arm: "off",
            mrz_line1_select_arm: "off",
            model_paths: ModelPathsReport {
                detection: "/models/text-detection.rten".to_string(),
                recognition: "/models/text-recognition.rten".to_string(),
            },
            providers: Vec::new(),
        };
        let json = serde_json::to_string(&report).expect("serialize report");
        let parsed: crate::bench_report::GateReport =
            serde_json::from_str(&json).expect("GateReport ignores the new key");
        assert_eq!(parsed.source, "synthetic-corpus");
        assert_eq!(parsed.mrz_class_sweep_arm, "off");
    }

    #[test]
    fn old_baseline_json_without_ledger_fields_round_trips_without_them() {
        let json = r#"{
            "note": "n",
            "measured_on_ci_sha": "abc",
            "measured_date": "2026-09-16",
            "samples_data_sha": "def",
            "documents": 10,
            "scored": 8,
            "tier1_hits": 7,
            "tolerance": 0,
            "by_miss_kind": {"checksum_failed": 1}
        }"#;
        let baseline: RealSpecimenBaseline =
            serde_json::from_str(json).expect("an old baseline with no ledger keys");
        assert_eq!(baseline.outcomes_sha256, None);
        assert_eq!(baseline.refusal_population, None);
        let text = serde_json::to_string_pretty(&baseline).expect("serialize");
        assert!(
            !text.contains("outcomes_sha256"),
            "an absent field must not round-trip back in: {text}"
        );
        assert!(
            !text.contains("refusal_population"),
            "an absent field must not round-trip back in: {text}"
        );
    }

    /// #473: unlike `retry_variant_id`/`retry_stop`/`chargrid`, these two
    /// keys are **always** serialized — `null` when `None`, never omitted —
    /// so the #473 A/B diff (`tools/synth_ab_diff.py`) can rely on the
    /// columns being stable across every row, whether or not native OCR
    /// telemetry exists for it.
    #[test]
    fn damaged_recovery_keys_are_always_present_null_or_set() {
        fn detail(name: &str) -> DocumentDetailReport {
            DocumentDetailReport {
                line1_selection: None,
                name: name.to_string(),
                asset_id: None,
                mrz_found: true,
                mrz_format: Some("TD3"),
                read_ok: true,
                mrz_checksums_valid: true,
                miss_reason: None,
                check_states: None,
                assertions_total: 0,
                assertions_unsupported: 0,
                unsupported_fields: Vec::new(),
                names_exact: None,
                name_error: None,
                field_correctness: None,
                ocr_ms: 0,
                retry_variant_id: None,
                retry_damaged_recovery: None,
                retry_budget_hit: false,
                retry_stop: None,
                chargrid: None,
                tier1_damaged_recovery: None,
            }
        }

        let absent = detail("absent");
        let json = serde_json::to_value(&absent).expect("serialize");
        assert_eq!(json["retry_damaged_recovery"], serde_json::Value::Null);
        assert_eq!(json["tier1_damaged_recovery"], serde_json::Value::Null);
        assert!(
            json.as_object()
                .unwrap()
                .contains_key("retry_damaged_recovery"),
            "retry_damaged_recovery must be present as null, not omitted: {json}"
        );
        assert!(
            json.as_object()
                .unwrap()
                .contains_key("tier1_damaged_recovery"),
            "tier1_damaged_recovery must be present as null, not omitted: {json}"
        );

        let mut set = detail("set");
        set.retry_variant_id = Some("pass-01".to_string());
        set.retry_damaged_recovery = Some(true);
        set.tier1_damaged_recovery = Some(false);
        let json = serde_json::to_value(&set).expect("serialize");
        assert_eq!(json["retry_variant_id"], "pass-01");
        assert_eq!(json["retry_damaged_recovery"], true);
        assert_eq!(json["tier1_damaged_recovery"], false);
    }

    #[test]
    fn outcome_row_serializes_as_one_line_with_fixed_key_order_and_null_for_absent_fields() {
        let row = OutcomeRow {
            asset_id: Some("passports/foo.png".to_string()),
            name: "foo".to_string(),
            outcome: "hit".to_string(),
            miss_reason: None,
            mrz_format: Some("TD3".to_string()),
            mrz_found: true,
            mrz_checksums_valid: true,
            names_exact: Some(true),
            name_error: None,
            ocr_ms: 42,
            retry_variant_id: None,
            retry_budget_hit: false,
            retry_stop: None,
        };
        let json = serde_json::to_string(&row).expect("serialize");
        assert_eq!(
            json,
            r#"{"asset_id":"passports/foo.png","name":"foo","outcome":"hit","miss_reason":null,"mrz_format":"TD3","mrz_found":true,"mrz_checksums_valid":true,"names_exact":true,"name_error":null,"ocr_ms":42,"retry_variant_id":null,"retry_budget_hit":false,"retry_stop":null}"#
        );
    }

    /// A minimal `mrz`-provider [`ProviderReport`] carrying only what
    /// [`RealSpecimenSnapshot::from_reports`] reads for the strict-name
    /// mapping — an empty `documents_detail` is fine, since that field feeds
    /// `documents`/`scored`/`tier1_hits`/`by_miss_kind`, not `strict_names`.
    fn mrz_report_with_strict(strict: StrictNameHitRate) -> ProviderReport {
        ProviderReport {
            provider_id: "mrz".to_string(),
            documents: 0,
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
            documents_detail: Vec::new(),
            tier1_hit_rate: Tier1HitRate::Computed(0.0),
            ocr_arms: synthpass_ocr::OcrArms::DEFAULT,
            strict_tier1_hit_rate: strict,
        }
    }

    #[test]
    fn from_reports_maps_computed_strict_name_hit_rate_to_some() {
        let report = mrz_report_with_strict(StrictNameHitRate::Computed {
            strict_hits: 3,
            name_scorable_documents: 5,
            name_scorable_hits: 4,
            strict_tier1_hit_rate: 3.0 / 5.0,
            names_exact_among_hits: Some(3.0 / 4.0),
        });
        let snap = RealSpecimenSnapshot::from_reports(&[report]).expect("mrz provider present");
        let strict = snap.strict_names.expect("Computed must map to Some");
        assert_eq!(strict.strict_hits, 3);
        assert_eq!(strict.name_scorable_documents, 5);
        assert_eq!(strict.name_scorable_hits, 4);
    }

    #[test]
    fn from_reports_maps_not_applicable_strict_name_hit_rate_to_none() {
        let report = mrz_report_with_strict(StrictNameHitRate::NotApplicable {
            reason: "no name-scorable documents",
        });
        let snap = RealSpecimenSnapshot::from_reports(&[report]).expect("mrz provider present");
        assert_eq!(snap.strict_names, None);
    }

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

    fn nonconforming_specimen() -> Option<MissReason> {
        Some(MissReason::ChecksumFailed {
            check_states: BTreeMap::new(),
            specimen_nonconforming: true,
        })
    }

    /// Issue #443: a `checksum_failed_specimen` document whose read verified
    /// is named; one whose read failed its checksums, as the printed zone
    /// does, is not. An ordinary `checksum_failed` miss and a `redacted_mrz`
    /// read are other buckets with their own handling, never counted here.
    #[test]
    fn only_a_checksum_valid_read_on_a_nonconforming_specimen_is_counted() {
        let mut manufactured = detail("Argentina_2026", None, nonconforming_specimen());
        manufactured.mrz_checksums_valid = true;
        let honest = detail("Bosnia_2013", None, nonconforming_specimen());
        let mut ordinary = detail(
            "Ordinary",
            None,
            Some(MissReason::ChecksumFailed {
                check_states: BTreeMap::new(),
                specimen_nonconforming: false,
            }),
        );
        ordinary.mrz_checksums_valid = true;
        let mut redacted = detail("Redacted", None, Some(MissReason::Redacted));
        redacted.mrz_checksums_valid = true;
        let hit = detail("Hit", None, None);

        let details = vec![manufactured, honest, ordinary, redacted, hit];
        assert_eq!(
            checksum_valid_reads_on_failed_specimens(&details),
            vec!["Argentina_2026"]
        );

        let row = ProviderRow::from(mrz_report_with_details(details));
        assert_eq!(row.checksum_valid_on_failed_specimen, 1);
        let json = serde_json::to_value(&row).expect("serialize row");
        assert_eq!(json["checksum_valid_on_failed_specimen"], 1);
    }

    /// No such read is a measured zero in the JSON, not an absent key — the
    /// later failing step needs the field present on every run.
    #[test]
    fn no_checksum_valid_read_on_a_nonconforming_specimen_reports_zero() {
        let details = vec![detail("Bosnia_2013", None, nonconforming_specimen())];
        assert!(checksum_valid_reads_on_failed_specimens(&details).is_empty());
        let json = serde_json::to_value(ProviderRow::from(mrz_report_with_details(details)))
            .expect("serialize row");
        assert_eq!(json["checksum_valid_on_failed_specimen"], 0);
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

    #[test]
    fn from_reports_prefills_every_known_bucket_with_zero() {
        // A single hit and nothing else: every `REGRESSION_BUCKETS` and
        // `OFF_DENOMINATOR_KINDS` key must still be present in `by_miss_kind`,
        // as an explicit `0`, not absent — see `from_reports`'s doc.
        let report = mrz_report_with_details(vec![detail("a", Some("a"), None)]);
        let snap = RealSpecimenSnapshot::from_reports(&[report]).expect("mrz provider present");
        for &bucket in REGRESSION_BUCKETS.iter().chain(OFF_DENOMINATOR_KINDS) {
            assert_eq!(
                snap.by_miss_kind.get(bucket).copied(),
                Some(0),
                "`{bucket}` must be present and zero, not absent"
            );
        }
    }

    #[test]
    fn refusal_population_equals_documents_minus_scored() {
        let report = mrz_report_with_details(vec![
            detail("a", Some("a"), None),
            detail("b", Some("b"), Some(MissReason::Redacted)),
            detail("c", Some("c"), Some(MissReason::NoMrzExpected)),
            detail(
                "d",
                Some("d"),
                Some(MissReason::NoMrzFound("nothing MRZ-shaped".to_string())),
            ),
        ]);
        let snap = RealSpecimenSnapshot::from_reports(&[report]).expect("mrz provider present");
        assert_eq!(snap.refusal_population, snap.documents - snap.scored);
        assert_eq!(
            snap.refusal_population, 2,
            "Redacted + NoMrzExpected are off-denominator; NoMrzFound is scored"
        );
    }

    // ---- #574: the shadow line-1 selector in the report ----

    fn selection(
        verdict: &'static str,
        reason: Option<&'static str>,
    ) -> crate::provider_bench::Line1SelectionDetail {
        crate::provider_bench::Line1SelectionDetail {
            arm: "control",
            verdict,
            reason,
            eligible: 1,
            distinct: 1,
            names_changed: true,
            source_passes: vec!["pass-04".to_string()],
            source_transforms: vec!["mrz_variants:2".to_string()],
        }
    }

    /// A document with no recorded verdict (the arm is `off`, or nothing was
    /// accepted) writes no `line1_selection` key, and a provider row with none
    /// writes no `line1_selection_counts` key. The selector is on by default
    /// (#574), so a default run does record verdicts; this pins the absent case.
    #[test]
    fn off_writes_no_line1_selection_key() {
        let details = vec![detail("a", Some("a"), None)];
        let json = serde_json::to_value(ProviderRow::from(mrz_report_with_details(details)))
            .expect("serialize row");
        assert!(json.get("line1_selection_counts").is_none(), "{json}");
        assert!(
            json["documents_detail"][0].get("line1_selection").is_none(),
            "{json}"
        );
    }

    /// The key set and its order are the schema: pinned here, and text-free.
    #[test]
    fn a_recorded_selection_serializes_its_pinned_keys_in_order() {
        let mut proposed = detail("a", Some("a"), None);
        proposed.line1_selection = Some(selection("proposed", None));
        let json =
            serde_json::to_string(&ProviderRow::from(mrz_report_with_details(vec![proposed])))
                .expect("serialize row");
        assert!(
            json.contains(
                r#""line1_selection":{"arm":"control","verdict":"proposed","reason":null,"eligible":1,"distinct":1,"names_changed":true,"source_passes":["pass-04"],"source_transforms":["mrz_variants:2"]}"#
            ),
            "{json}"
        );
    }

    /// Every verdict is a key of the counts, in a fixed order, so an arm that
    /// never proposed reads `0` and two arms' counts diff key by key.
    #[test]
    fn the_verdict_counts_carry_every_verdict_key() {
        let mut kept = detail("kept", Some("kept"), None);
        kept.line1_selection = Some(selection("none", Some("kept")));
        let mut applied = detail("applied", Some("applied"), None);
        applied.line1_selection = Some(selection("applied", None));
        let mut also_kept = detail("also", Some("also"), None);
        also_kept.line1_selection = Some(selection("none", Some("kept")));
        let json = serde_json::to_value(ProviderRow::from(mrz_report_with_details(vec![
            kept, applied, also_kept,
        ])))
        .expect("serialize row");
        assert_eq!(
            json["line1_selection_counts"],
            serde_json::json!({
                "applied": 1,
                "proposed": 0,
                "unresolved": 0,
                "ambiguous": 0,
                "none": 2,
            })
        );
    }

    /// The top-level arm is always serialized, beside the class-sweep arm.
    #[test]
    fn the_report_always_carries_the_line1_select_arm() {
        let report = Report {
            timestamp_unix: 0,
            source: "real-specimens",
            profile: None,
            format: None,
            count: 0,
            seed_start: None,
            mrz_class_sweep_arm: "off",
            mrz_line1_select_arm: "control",
            model_paths: ModelPathsReport::default(),
            providers: Vec::new(),
        };
        let json = serde_json::to_value(&report).expect("serialize report");
        assert_eq!(json["mrz_line1_select_arm"], "control");
        assert_eq!(json["mrz_class_sweep_arm"], "off");
    }
}
