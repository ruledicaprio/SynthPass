//! `bench-report`: turns three static artifacts — the gate report JSON, the
//! committed real-specimen baseline, and `samples/corpus.jsonl` — into a
//! deterministic Markdown benchmark report, with **no corpus image read and
//! no OCR run**. See `crates/synthpass-bench/src/bin/bench-report.rs` for the
//! CLI and `knowledge/benchmarks/PIPELINE.md` §7 item 4 for why this exists:
//! the Q4 assessment's one external deliverable — a generated,
//! provenance-complete report a stranger can read without cloning the repo.
//!
//! # Why this defines its own read-only mirror of the gate report JSON
//!
//! [`crate::report::Report`] and [`crate::report::ProviderRow`] only derive
//! `Serialize`: several of their fields are `&'static str` (miss-kind names,
//! capability strings, OCR-arm labels) that only ever come from compile-time
//! string literals in the writer. A derived `Deserialize` for a `&'static
//! str` field can only borrow from genuinely `'static` input, which bytes
//! read back off disk never are — the same reason [`crate::report::OutcomeRow`]
//! copies those fields into owned `String`s instead of reusing
//! [`crate::report::DocumentDetail`]'s borrowed ones.
//!
//! Rather than widen the writer's schema for a reader that needs only a
//! handful of its fields, this module defines its own minimal, owned-`String`
//! mirror ([`GateReport`], [`ProviderConfigRow`], [`CapabilityConfig`],
//! [`OcrArmsConfig`]) with no `#[serde(deny_unknown_fields)]`, so a gate
//! report gaining a field (e.g. a new `documents_detail` column) never breaks
//! this reader. The baseline JSON has no such problem —
//! [`crate::report::RealSpecimenBaseline`] is already `Deserialize` with
//! owned fields throughout — so this module reads it directly.
//!
//! # Two different "per-format" cuts — do not confuse them
//!
//! This report carries two sections that could both be mistaken for
//! "per-format," so they are named to keep them apart:
//!
//! - **"Per-format accuracy"** ([`write_per_format_accuracy`]) is *how well
//!   we read each format* — Observed, from the gate report's
//!   `documents_detail[].mrz_format` and `.miss_reason`, joined against
//!   `samples/corpus.jsonl`'s `mrz.issuing_state` (via `dir`/`filename`) for
//!   the per-issuer half. Never the manifest's own `observed.*` fields,
//!   which are a dated OCR snapshot frozen at whatever pipeline last touched
//!   `corpus_manifest.rs` — quoting them under this report's fresh
//!   provenance block would publish a stale number as if it were current.
//!   `PIPELINE.md`'s §7 rework-plan row 3 names this exact join.
//! - **"Corpus composition"** ([`write_corpus_composition`]) is *how many
//!   documents of each kind exist* — Derived, purely from
//!   `samples/corpus.jsonl`'s `dir`/`provenance`/`year`/`origin.licence`
//!   fields, with no per-document join at all.
//!
//! # Determinism
//!
//! [`render_markdown`] reads no clock, no environment variable, and no path
//! from its caller into its output — only the three parsed artifacts. Buckets
//! are walked in a fixed, hand-declared order
//! ([`KNOWN_BUCKET_ORDER`]); the one map iterated directly
//! ([`crate::report::RealSpecimenBaseline::by_miss_kind`]) is a `BTreeMap`,
//! whose iteration order is its sort order, not insertion order. The same
//! three input artifacts byte-for-byte always produce the same Markdown
//! byte-for-byte.

use crate::report::{RealSpecimenBaseline, OFF_DENOMINATOR_KINDS};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

type Result<T> = std::result::Result<T, String>;

// ---------------------------------------------------------------------
// Gate report JSON — read-only mirror; see the module doc for why this is
// not a derived `Deserialize` on `crate::report::Report` itself.
// ---------------------------------------------------------------------

/// The subset of `provider-bench --out PATH`'s top-level report JSON this
/// generator reads. Unknown fields (`timestamp_unix`, `seed_start`, …) are
/// ignored rather than rejected — see the module doc.
#[derive(Debug, Deserialize)]
pub struct GateReport {
    /// `"real-specimens"` or `"synthetic-corpus"`.
    pub source: String,
    #[serde(default)]
    pub profile: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    pub mrz_class_sweep_arm: String,
    pub providers: Vec<ProviderConfigRow>,
}

/// One provider row's configuration fields, plus the one per-document field
/// this report needs for the per-format/per-issuer accuracy cut. Not the
/// provider's accuracy/speed aggregates, which this report reads from the
/// baseline instead.
#[derive(Debug, Deserialize)]
pub struct ProviderConfigRow {
    pub provider_id: String,
    /// Cross-checked against [`RealSpecimenBaseline::documents`] in
    /// [`render_markdown`] — a report and a baseline that disagree on this
    /// count do not describe the same run, and reporting them together
    /// would put false provenance in the output.
    pub documents: usize,
    pub capability: CapabilityConfig,
    pub ocr_arms: OcrArmsConfig,
    /// One row per document. Always present on a genuine `provider-bench`
    /// report (`ProviderRow::documents_detail` is "always serialized,
    /// regardless of `--verbose`"); defaulted to empty here only so a
    /// hand-written or reduced fixture that omits it still parses instead of
    /// erroring — [`render_markdown`] itself checks the length reconciles
    /// against `documents`, so an empty default cannot silently pass as "no
    /// per-document data was ever collected."
    #[serde(default)]
    pub documents_detail: Vec<DocumentDetailRow>,
}

/// The subset of `DocumentDetailReport` the per-format/per-issuer accuracy
/// cut needs. All three fields are `#[serde(skip_serializing_if =
/// "Option::is_none")]` on the writer, so the JSON key can be entirely
/// absent (not merely `null`) when the value is `None` — `#[serde(default)]`
/// on each covers both that and an explicit `null`.
#[derive(Debug, Deserialize)]
pub struct DocumentDetailRow {
    /// Samples-relative asset identity (`"<dir>/<filename>"`), the join key
    /// into `samples/corpus.jsonl`. Absent for the synthetic corpus, whose
    /// documents have no manifest row at all.
    #[serde(default)]
    pub asset_id: Option<String>,
    /// The resolved ICAO 9303 MRZ format ("TD1"/"TD2"/"TD3"/"MRVA"/"MRVB"),
    /// `None` when no format could be resolved for this document.
    #[serde(default)]
    pub mrz_format: Option<String>,
    /// The stable machine-readable miss class (`miss_kind`'s output, e.g.
    /// `"checksum_failed"`) — `None` on a genuine Tier-1 hit.
    #[serde(default)]
    pub miss_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CapabilityConfig {
    pub deterministic: bool,
    pub vision: bool,
    pub cost: String,
}

#[derive(Debug, Deserialize)]
pub struct OcrArmsConfig {
    pub texture: String,
    pub order: String,
    pub rotate: String,
    pub skew: String,
    pub chargrid: String,
}

impl GateReport {
    /// The deterministic `mrz` provider's row — the only provider this
    /// report describes. `None` when the gate report was produced without
    /// the `mrz` reader registered (should never happen for
    /// `real-specimen-gate.yml`'s `--mrz-only` invocation, but never
    /// assumed).
    fn mrz_provider(&self) -> Option<&ProviderConfigRow> {
        self.providers.iter().find(|p| p.provider_id == "mrz")
    }
}

/// Parses the gate report JSON (`provider-bench --out PATH`'s output — e.g.
/// `artifacts/real-specimen-gate-report.json`).
pub fn parse_gate_report(text: &str) -> Result<GateReport> {
    serde_json::from_str(text).map_err(|e| format!("parse gate report: {e}"))
}

/// Parses the committed baseline JSON
/// (`knowledge/benchmarks/real-specimen-mrz-baseline.json`).
pub fn parse_baseline(text: &str) -> Result<RealSpecimenBaseline> {
    serde_json::from_str(text).map_err(|e| format!("parse baseline: {e}"))
}

// ---------------------------------------------------------------------
// samples/corpus.jsonl — one JSON object per line. Only the fields this
// report reads: `dir` (format directory) + `filename` (together, the join
// key into `documents_detail[].asset_id`), `provenance` (specimen/template),
// `year.value`, `origin.licence`, `mrz.issuing_state`. See
// `crates/synthpass-ocr/examples/corpus_manifest.rs` for the full schema
// this is a subset of.
// ---------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CorpusRow {
    pub dir: String,
    pub filename: String,
    pub provenance: String,
    #[serde(default)]
    pub year: Option<CorpusYear>,
    pub origin: CorpusOrigin,
    pub mrz: CorpusMrz,
}

impl CorpusRow {
    /// `"<dir>/<filename>"` — must match
    /// [`DocumentDetailRow::asset_id`]'s format exactly
    /// (`RealSpecimenDoc::asset_id`'s doc: "path identity relative to the
    /// samples root, using slash separators").
    fn asset_id(&self) -> String {
        format!("{}/{}", self.dir, self.filename)
    }
}

#[derive(Debug, Deserialize)]
pub struct CorpusYear {
    #[serde(default)]
    pub value: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CorpusOrigin {
    pub licence: String,
}

#[derive(Debug, Deserialize)]
pub struct CorpusMrz {
    /// The manifest's claimed issuing state/country code (may be a
    /// placeholder such as `"XXX"` for a template with no real issuer) —
    /// never the manifest's `observed.*` block, which is a dated OCR
    /// snapshot, not this run's own read.
    pub issuing_state: String,
}

/// Parses `samples/corpus.jsonl`'s one-JSON-object-per-line format. A
/// malformed line names its 1-based line number — this manifest is the
/// corpus-composition source of truth for this report, not a best-effort log
/// to skip lines in.
pub fn parse_corpus(text: &str) -> Result<Vec<CorpusRow>> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
        .map(|(i, line)| {
            serde_json::from_str(line).map_err(|e| format!("corpus.jsonl line {}: {e}", i + 1))
        })
        .collect()
}

// ---------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------

/// [`crate::report::REGRESSION_BUCKETS`] (in the Tier-1 denominator) then
/// [`crate::report::OFF_DENOMINATOR_KINDS`] (excluded) — the fixed order the
/// outcome table presents every known bucket in, kept identical to those two
/// constants' own declared order rather than re-derived, so a change there is
/// a visible diff here too.
const KNOWN_BUCKET_ORDER: &[&str] = &[
    "checksum_failed",
    "no_mrz_found",
    "ocr_error",
    "document_number_mismatch",
    "false_positive_mrz",
    "redacted_mrz",
    "no_mrz_expected",
    "checksum_failed_specimen",
];

/// Human-readable description and Tier-1-denominator membership for one known
/// outcome bucket. `None` for a bucket this generator does not recognise (a
/// `MissReason` variant added after this file was last updated) — see
/// [`write_outcome_table`] for how that case is still surfaced rather than
/// dropped.
fn bucket_description(key: &str) -> Option<(&'static str, bool)> {
    match key {
        "checksum_failed" => Some((
            "Conforming printed zone, read wrong — a genuine OCR error",
            true,
        )),
        "no_mrz_found" => Some(("No MRZ located on a document that has one", true)),
        "ocr_error" => Some((
            "The OCR engine itself failed before parsing could be attempted",
            true,
        )),
        "document_number_mismatch" => Some((
            "Checksum-valid MRZ read, but the document number does not match ground truth",
            true,
        )),
        "false_positive_mrz" => Some((
            "A checksum-valid MRZ returned for a document that carries none",
            true,
        )),
        "redacted_mrz" => Some((
            "Zone blacked out by whoever published the specimen — excluded from the Tier-1 \
             denominator, not an OCR miss",
            false,
        )),
        "no_mrz_expected" => Some((
            "Document carries no MRZ at all; none was read — a correct refusal, excluded from \
             the Tier-1 denominator",
            false,
        )),
        "checksum_failed_specimen" => Some((
            "The printed zone fails its own ICAO check digits — a byte-perfect read still \
             fails, excluded from the Tier-1 denominator",
            false,
        )),
        _ => None,
    }
}

/// `"{hits} / {denominator} = {rate}%"`, or an explicit "not computed"
/// message when the denominator is zero — never a fabricated rate for a
/// population that could not have produced one.
fn format_rate(hits: usize, denominator: usize) -> String {
    if denominator == 0 {
        "n/a (denominator is zero)".to_string()
    } else {
        format!(
            "{hits} / {denominator} = {:.1}%",
            (hits as f64 / denominator as f64) * 100.0
        )
    }
}

/// The benchmark maintenance contract's permanent home — an absolute URL,
/// not a relative Markdown link: this report has no fixed location relative
/// to the repository (a release artifact, a test fixture, a downloaded
/// attachment), so a relative path would resolve differently depending on
/// where the reader opens it. `scripts/check-doc-links.sh` skips http(s)
/// links entirely, by design (see its own comment on that check).
const CONTRACT_URL: &str = "https://github.com/ruledicaprio/SynthPass/blob/main/\
     knowledge/benchmarks/README.md#benchmark-maintenance-contract";

const NOT_RECORDED: &str = "not recorded in this artifact pair";

/// One group's document/scored/hit counts, the shared arithmetic behind
/// both accuracy cuts below (per format, per issuer): every document adds to
/// `documents`; it also adds to `scored` unless its miss reason is one of
/// [`OFF_DENOMINATOR_KINDS`] (the same rule [`crate::report::RealSpecimenSnapshot::from_reports`]
/// applies when it builds the baseline in the first place); a `None` miss
/// reason (a hit) adds to both `scored` and `hits`.
#[derive(Debug, Default, Clone, Copy)]
struct GroupStats {
    documents: usize,
    scored: usize,
    hits: usize,
}

impl GroupStats {
    fn add(&mut self, miss_reason: &Option<String>) {
        self.documents += 1;
        match miss_reason {
            None => {
                self.scored += 1;
                self.hits += 1;
            }
            Some(kind) if !OFF_DENOMINATOR_KINDS.contains(&kind.as_str()) => self.scored += 1,
            Some(_) => {}
        }
    }

    /// The sum over every document, regardless of format or issuer — used
    /// both to reconcile against the baseline in [`render_markdown`] and as
    /// the "Total" row every accuracy table ends with.
    fn totals(details: &[DocumentDetailRow]) -> GroupStats {
        let mut total = GroupStats::default();
        for d in details {
            total.add(&d.miss_reason);
        }
        total
    }

    fn rate(&self) -> String {
        format_rate(self.hits, self.scored)
    }
}

/// Renders the full Markdown report. `Err` when the two artifacts do not
/// appear to describe the same run (see [`ProviderConfigRow::documents`]'s
/// doc), the gate report has no `mrz` provider row, its `documents_detail`
/// length disagrees with its own `documents` count, or recomputing
/// scored/hit counts from `documents_detail` disagrees with the baseline —
/// every case is a provenance claim this generator cannot stand behind.
pub fn render_markdown(
    report: &GateReport,
    baseline: &RealSpecimenBaseline,
    corpus: &[CorpusRow],
) -> Result<String> {
    let mrz = report
        .mrz_provider()
        .ok_or_else(|| "the gate report has no `mrz` provider row".to_string())?;
    if mrz.documents != baseline.documents {
        return Err(format!(
            "the gate report and baseline disagree on document count: the report's `mrz` \
             provider measured {}, the baseline records {} — they do not appear to describe the \
             same run",
            mrz.documents, baseline.documents
        ));
    }
    if mrz.documents_detail.len() != mrz.documents {
        return Err(format!(
            "the gate report's `mrz` provider claims {} documents but its `documents_detail` \
             carries {} rows — internally inconsistent, not safe to report from",
            mrz.documents,
            mrz.documents_detail.len()
        ));
    }
    let totals = GroupStats::totals(&mrz.documents_detail);
    if totals.scored != baseline.scored || totals.hits != baseline.tier1_hits {
        return Err(format!(
            "the gate report's `documents_detail` does not reconcile to the baseline: \
             recomputing from per-document miss reasons gives {} scored / {} hits, the baseline \
             records {} scored / {} hits — they do not appear to describe the same run",
            totals.scored, totals.hits, baseline.scored, baseline.tier1_hits
        ));
    }

    let mut out = String::new();
    let _ = writeln!(out, "# SynthPass real-specimen Tier-1 benchmark report");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Generated by `bench-report` from a `real-specimen-gate.yml` run's artifacts, with no \
         corpus image read and no OCR run. Every figure below is either read directly from the \
         gate report or the baseline JSON (**Observed**) or computed from those figures by this \
         generator (**Derived**)."
    );
    let _ = writeln!(out);

    write_provenance(&mut out, report, mrz, baseline);
    let _ = writeln!(out);
    write_headline(&mut out, baseline);
    let _ = writeln!(out);
    write_outcome_table(&mut out, baseline);
    let _ = writeln!(out);
    write_per_format_accuracy(&mut out, &mrz.documents_detail, corpus, totals);
    let _ = writeln!(out);
    write_corpus_composition(&mut out, corpus, baseline);
    let _ = writeln!(out);
    write_strict_names(&mut out, baseline);
    let _ = writeln!(out);
    write_methodology(&mut out);
    let _ = writeln!(out);
    write_limitations(&mut out, corpus);
    let _ = writeln!(out);
    write_reproduction(&mut out, baseline);

    Ok(out)
}

fn write_provenance(
    out: &mut String,
    report: &GateReport,
    mrz: &ProviderConfigRow,
    baseline: &RealSpecimenBaseline,
) {
    let format = report.format.as_deref().unwrap_or("all classes");
    let profile = report
        .profile
        .as_deref()
        .unwrap_or("n/a (real specimens have no capture profile)");
    let _ = writeln!(out, "## Provenance");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Field | Value |");
    let _ = writeln!(out, "| --- | --- |");
    let _ = writeln!(out, "| MAIN SHA | `{}` |", baseline.measured_on_ci_sha);
    let _ = writeln!(
        out,
        "| `samples-data` SHA | `{}` |",
        baseline.samples_data_sha
    );
    let _ = writeln!(out, "| Measured date | {} |", baseline.measured_date);
    let _ = writeln!(out, "| Workflow/run identifier | {NOT_RECORDED} |");
    let _ = writeln!(
        out,
        "| Exact invocation | {NOT_RECORDED} — see Reproduction below for a comparable command |"
    );
    let _ = writeln!(out, "| Corpus source | `{}` |", report.source);
    let _ = writeln!(out, "| Format scope | `{format}` |");
    let _ = writeln!(out, "| Capture profile | {profile} |");
    let _ = writeln!(
        out,
        "| Provider | `{}` (deterministic={}, vision={}, cost={}) |",
        mrz.provider_id, mrz.capability.deterministic, mrz.capability.vision, mrz.capability.cost
    );
    let _ = writeln!(
        out,
        "| `mrz` class-sweep arm | `{}` |",
        report.mrz_class_sweep_arm
    );
    let _ = writeln!(
        out,
        "| OCR arms | texture={}, order={}, rotate={}, skew={}, chargrid={} |",
        mrz.ocr_arms.texture,
        mrz.ocr_arms.order,
        mrz.ocr_arms.rotate,
        mrz.ocr_arms.skew,
        mrz.ocr_arms.chargrid
    );
    let _ = writeln!(out, "| Candidate population | {} |", baseline.documents);
    let _ = writeln!(out, "| Scored population | {} |", baseline.scored);
    let _ = writeln!(out, "| Tier-1 hits | {} |", baseline.tier1_hits);
    let _ = writeln!(out, "| Baseline tolerance | {} |", baseline.tolerance);
}

fn write_headline(out: &mut String, baseline: &RealSpecimenBaseline) {
    let off_denominator = baseline.documents - baseline.scored;
    let _ = writeln!(out, "## Headline");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- **Tier-1 hit rate, scored population (Observed):** {}",
        format_rate(baseline.tier1_hits, baseline.scored)
    );
    let _ = writeln!(
        out,
        "- **Tier-1 hit rate, whole corpus (Observed):** {}",
        format_rate(baseline.tier1_hits, baseline.documents)
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "\"Scored population\" ({}) excludes {off_denominator} document(s) that cannot yield a \
         Tier-1 hit under any pipeline (`no_mrz_expected`, `redacted_mrz`, \
         `checksum_failed_specimen` — see the outcome table below); \"whole corpus\" ({}) counts \
         every candidate document this run walked, including those. Neither rate is published \
         without the other, and neither without its own denominator stated beside it.",
        baseline.scored, baseline.documents
    );
}

fn write_outcome_table(out: &mut String, baseline: &RealSpecimenBaseline) {
    let _ = writeln!(out, "## Outcome table");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Every outcome bucket the baseline records, zero included — a bucket that never fired \
         still prints `0`, not an absent row. \"In the Tier-1 denominator?\" answers whether \
         growth in that bucket is a Tier-1 regression; `no_mrz_expected`, `redacted_mrz` and \
         `checksum_failed_specimen` are excluded and are not OCR misses."
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "| Outcome | Count | In the Tier-1 denominator? | Meaning |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- |");
    let _ = writeln!(
        out,
        "| **Tier-1 HIT** | **{}** | numerator | Checksum-valid MRZ, document number matches \
         ground truth |",
        baseline.tier1_hits
    );
    for &key in KNOWN_BUCKET_ORDER {
        let count = baseline.by_miss_kind.get(key).copied().unwrap_or(0);
        let (description, in_denominator) =
            bucket_description(key).expect("every KNOWN_BUCKET_ORDER key has a description");
        let flag = if in_denominator { "yes" } else { "no" };
        let _ = writeln!(out, "| `{key}` | {count} | {flag} | {description} |");
    }
    // Any bucket the baseline carries that this generator does not recognise
    // — a new `MissReason` variant added after this file was last updated —
    // is still printed, never silently dropped; `by_miss_kind` is a
    // `BTreeMap`, so this loop is itself already deterministic.
    let known: BTreeSet<&str> = KNOWN_BUCKET_ORDER.iter().copied().collect();
    for (key, count) in &baseline.by_miss_kind {
        if !known.contains(key.as_str()) {
            let _ = writeln!(
                out,
                "| `{key}` | {count} | not recognised by this report generator | a new outcome \
                 bucket was added after this report generator was last updated |"
            );
        }
    }
}

/// The per-document accuracy cut PIPELINE.md §7 row 3 names: grouped by ICAO
/// format (`documents_detail[].mrz_format`) and by issuing state (joined
/// against `samples/corpus.jsonl`'s `mrz.issuing_state`) — "how well do we
/// read each format," not [`write_corpus_composition`]'s "how many documents
/// of each kind exist." `totals` is [`GroupStats::totals`]`(details)`,
/// already computed and reconciled against the baseline in
/// [`render_markdown`]; passed in rather than recomputed so the "Total" row
/// is provably the same sum the reconciliation check used.
fn write_per_format_accuracy(
    out: &mut String,
    details: &[DocumentDetailRow],
    corpus: &[CorpusRow],
    totals: GroupStats,
) {
    let _ = writeln!(out, "## Per-format accuracy (Observed)");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "How well this run read each ICAO format and each issuing state — not corpus \
         composition (see that section below for how many documents of each kind exist). Every \
         row's rate states its own scored count beside it; a format or issuer with a handful of \
         scored documents produces a noisy rate that should not be read the same way as one with \
         dozens."
    );
    let _ = writeln!(out);

    write_by_icao_format(out, details, totals);
    let _ = writeln!(out);
    write_by_issuing_state(out, details, corpus, totals);
}

/// One row per distinct `mrz_format` value plus an explicit "no format
/// resolved" row for `None` — never a silently dropped document. Rows sum to
/// `totals` exactly (asserted by [`render_markdown`]'s reconciliation guard
/// before this function is ever called).
fn write_by_icao_format(out: &mut String, details: &[DocumentDetailRow], totals: GroupStats) {
    let mut by_format: BTreeMap<&str, GroupStats> = BTreeMap::new();
    let mut no_format = GroupStats::default();
    for d in details {
        match d.mrz_format.as_deref() {
            Some(fmt) => by_format.entry(fmt).or_default().add(&d.miss_reason),
            None => no_format.add(&d.miss_reason),
        }
    }

    let _ = writeln!(out, "### By ICAO format");
    let _ = writeln!(out);
    let _ = writeln!(out, "| Format | Documents | Scored | Hits | Rate |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for (fmt, stats) in &by_format {
        let _ = writeln!(
            out,
            "| `{fmt}` | {} | {} | {} | {} |",
            stats.documents,
            stats.scored,
            stats.hits,
            stats.rate()
        );
    }
    let _ = writeln!(
        out,
        "| *(no format resolved)* | {} | {} | {} | {} |",
        no_format.documents,
        no_format.scored,
        no_format.hits,
        no_format.rate()
    );
    let _ = writeln!(
        out,
        "| **Total** | **{}** | **{}** | **{}** | {} |",
        totals.documents,
        totals.scored,
        totals.hits,
        totals.rate()
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "The Total row reconciles exactly to this report's Headline and Outcome table above — \
         every document this run walked is counted in exactly one format row, including the \
         `(no format resolved)` row for a document where no format could be determined at all."
    );
}

/// One row per distinct `mrz.issuing_state` value the manifest join
/// resolves, plus two explicit accounting rows so the join's own gaps are
/// counted rather than silently dropped: `(no asset id)` for a document with
/// no `asset_id` at all (the synthetic corpus has none), and `(no manifest
/// match)` for an `asset_id` present in `documents_detail` but absent from
/// `samples/corpus.jsonl` — a real possibility this join must surface, not
/// paper over. Rows sum to `totals` exactly, the same guarantee
/// [`write_by_icao_format`] gives.
fn write_by_issuing_state(
    out: &mut String,
    details: &[DocumentDetailRow],
    corpus: &[CorpusRow],
    totals: GroupStats,
) {
    let _ = writeln!(out, "### By issuing state");
    let _ = writeln!(out);

    if details.iter().all(|d| d.asset_id.is_none()) {
        let _ = writeln!(
            out,
            "Not applicable: no document in this run carries an `asset_id` (a synthetic-corpus \
             run has none — only real specimens are identified against the manifest). The \
             issuer cut only applies to a `--real-specimens` run."
        );
        return;
    }

    let index: BTreeMap<String, &CorpusRow> = corpus.iter().map(|r| (r.asset_id(), r)).collect();
    let mut by_issuer: BTreeMap<&str, GroupStats> = BTreeMap::new();
    let mut no_asset_id = GroupStats::default();
    let mut no_manifest_match = GroupStats::default();
    for d in details {
        match &d.asset_id {
            None => no_asset_id.add(&d.miss_reason),
            Some(id) => match index.get(id.as_str()) {
                Some(row) => by_issuer
                    .entry(row.mrz.issuing_state.as_str())
                    .or_default()
                    .add(&d.miss_reason),
                None => no_manifest_match.add(&d.miss_reason),
            },
        }
    }

    let _ = writeln!(
        out,
        "Joined via `documents_detail[].asset_id` against `samples/corpus.jsonl`'s \
         `\"<dir>/<filename>\"` and its `mrz.issuing_state` field — never the manifest's \
         `observed.*` block, a dated OCR snapshot from whenever `corpus_manifest.rs` last ran, \
         not this run's own read."
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "| Issuing state | Documents | Scored | Hits | Rate |");
    let _ = writeln!(out, "| --- | --- | --- | --- | --- |");
    for (issuer, stats) in &by_issuer {
        let _ = writeln!(
            out,
            "| `{issuer}` | {} | {} | {} | {} |",
            stats.documents,
            stats.scored,
            stats.hits,
            stats.rate()
        );
    }
    let _ = writeln!(
        out,
        "| *(no manifest match)* | {} | {} | {} | {} |",
        no_manifest_match.documents,
        no_manifest_match.scored,
        no_manifest_match.hits,
        no_manifest_match.rate()
    );
    let _ = writeln!(
        out,
        "| *(no asset id)* | {} | {} | {} | {} |",
        no_asset_id.documents,
        no_asset_id.scored,
        no_asset_id.hits,
        no_asset_id.rate()
    );
    let _ = writeln!(
        out,
        "| **Total** | **{}** | **{}** | **{}** | {} |",
        totals.documents,
        totals.scored,
        totals.hits,
        totals.rate()
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "The Total row reconciles exactly to the By ICAO format table above and to this report's \
         Headline: same documents, partitioned by issuer instead of by format. `(no manifest \
         match)` is a join failure, not a corpus fact — a non-zero count there means this gate \
         report and this `samples/corpus.jsonl` were not generated from the same corpus revision, \
         and the issuer figures above should not be trusted until that is resolved."
    );
}

/// How many documents of each kind the manifest holds — Derived from
/// `samples/corpus.jsonl` alone, with no per-document join. See the module
/// doc's "Two different \"per-format\" cuts" section for why this is a
/// different question from [`write_per_format_accuracy`] above.
fn write_corpus_composition(
    out: &mut String,
    corpus: &[CorpusRow],
    baseline: &RealSpecimenBaseline,
) {
    let total = corpus.len();
    let dirs: BTreeSet<&str> = corpus.iter().map(|r| r.dir.as_str()).collect();
    let provenances: BTreeSet<&str> = corpus.iter().map(|r| r.provenance.as_str()).collect();
    let mut counts: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    for row in corpus {
        *counts
            .entry((row.dir.as_str(), row.provenance.as_str()))
            .or_default() += 1;
    }

    let _ = writeln!(out, "## Corpus composition (Derived)");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "How many documents of each kind the manifest holds — not accuracy (see Per-format \
         accuracy above for how well each is read). Computed from `samples/corpus.jsonl` \
         ({total} rows) — the full specimen manifest, grouped by its `dir` and `provenance` \
         fields. This describes the corpus, not this run's Tier-1 result: the scored Tier-1 \
         population above ({} of {} candidate documents) is drawn from a subset of these \
         {total} manifest rows (some `samples/` tracks — e.g. `covers/`, `private/`, `local/` \
         — are excluded from the default gate walk). The two populations are not assumed to be \
         the same; this table reports the manifest, not the scored run.",
        baseline.scored, baseline.documents
    );
    let _ = writeln!(out);

    let mut header = String::from("| Format (`dir`) |");
    let mut sep = String::from("| --- |");
    for p in &provenances {
        let _ = write!(header, " `{p}` |");
        sep.push_str(" --- |");
    }
    header.push_str(" Total |");
    sep.push_str(" --- |");
    let _ = writeln!(out, "{header}");
    let _ = writeln!(out, "{sep}");

    let mut column_totals: BTreeMap<&str, usize> = provenances.iter().map(|p| (*p, 0)).collect();
    for dir in &dirs {
        let mut row_total = 0usize;
        let mut row = format!("| `{dir}` |");
        for p in &provenances {
            let n = counts.get(&(*dir, *p)).copied().unwrap_or(0);
            row_total += n;
            *column_totals.get_mut(p).expect("column seeded above") += n;
            let _ = write!(row, " {n} |");
        }
        let _ = write!(row, " {row_total} |");
        let _ = writeln!(out, "{row}");
    }

    let mut footer = String::from("| **Total** |");
    let mut grand_total = 0usize;
    for p in &provenances {
        let n = column_totals[p];
        grand_total += n;
        let _ = write!(footer, " **{n}** |");
    }
    let _ = write!(footer, " **{grand_total}** |");
    let _ = writeln!(out, "{footer}");
    let _ = writeln!(out);

    let with_year = corpus
        .iter()
        .filter(|r| r.year.as_ref().and_then(|y| y.value).is_some())
        .count();
    let without_year = total - with_year;
    let _ = writeln!(
        out,
        "Of {total} manifest rows, {with_year} record a year (`year.value`); {without_year} do \
         not."
    );
}

fn write_strict_names(out: &mut String, baseline: &RealSpecimenBaseline) {
    let _ = writeln!(out, "## Strict names (ADR-0013)");
    let _ = writeln!(out);
    match &baseline.strict_names {
        Some(s) => {
            let _ = writeln!(
                out,
                "- **Strict Tier-1 hit rate (Observed):** {} of name-scorable documents in the \
                 scored population",
                format_rate(s.strict_hits, s.name_scorable_documents)
            );
            let _ = writeln!(
                out,
                "- **Names exact among hits (Observed):** {} of name-scorable Tier-1 hits",
                format_rate(s.strict_hits, s.name_scorable_hits)
            );
            let _ = writeln!(out);
            let _ = writeln!(
                out,
                "Neither rate redefines `hit`/`tier1_hit_rate`: no ICAO 9303 check digit covers \
                 either name field, so a checksum-valid Tier-1 hit can still carry a wrong name."
            );
        }
        None => {
            let _ = writeln!(
                out,
                "Not measured in this baseline — `strict_names` is absent. Never printed as \
                 `0.0`: an absent measurement is not a measured zero."
            );
        }
    }
}

fn write_methodology(out: &mut String) {
    let _ = writeln!(out, "## Methodology");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Generated with no corpus image read and no OCR run: every figure above is read from the \
         gate report JSON, the committed real-specimen baseline, or `samples/corpus.jsonl` — \
         three static artifacts, nothing else. This report follows the eight-step benchmark \
         maintenance contract (`freeze → reconcile → measure → localize → change → regress → \
         inspect → record`) documented at {CONTRACT_URL}"
    );
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Evidence here is labelled **Observed** (read directly from a measured artifact), \
         **Derived** (computed by this generator from Observed figures), or stated as not \
         measured. Nothing here is Hypothesized: a projection with no run behind it does not \
         belong in a generated report."
    );
}

fn write_limitations(out: &mut String, corpus: &[CorpusRow]) {
    let total = corpus.len();
    let mut provenance_counts: BTreeMap<&str, usize> = BTreeMap::new();
    let mut licence_counts: BTreeMap<&str, usize> = BTreeMap::new();
    for row in corpus {
        *provenance_counts
            .entry(row.provenance.as_str())
            .or_default() += 1;
        *licence_counts
            .entry(row.origin.licence.as_str())
            .or_default() += 1;
    }
    let provenance_line = provenance_counts
        .iter()
        .map(|(k, v)| format!("{v} `{k}`"))
        .collect::<Vec<_>>()
        .join(", ");
    let licence_line = licence_counts
        .iter()
        .map(|(k, v)| format!("{v} `{k}`"))
        .collect::<Vec<_>>()
        .join(", ");

    let _ = writeln!(out, "## Limitations");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "- **Specimen-only corpus.** `samples/corpus.jsonl` lists {total} rows: {provenance_line}. \
         This is not a clean-room dataset; its composition is bounded by acquisition effort, not \
         experimental design."
    );
    let _ = writeln!(
        out,
        "- **Single OCR stack.** Every figure above runs the deterministic `mrz` provider over \
         one native OCR engine build (`ocrs`/`rten`); there is no Tier-2 LLM figure and no \
         browser-stack figure in this report."
    );
    let _ = writeln!(
        out,
        "- **Country and format coverage is whatever the manifest holds.** See the Per-format \
         accuracy and Corpus composition tables above; a format or issuer with a handful of \
         scored documents has a noisy rate, and nothing here claims the corpus is representative \
         of any wider population."
    );
    let _ = writeln!(
        out,
        "- **Local runs are not comparable to the committed baseline.** Local `rten` inference \
         differs from CI's by float rounding (see the Methodology contract link above); only a \
         CI-written baseline is authoritative."
    );
    let _ = writeln!(
        out,
        "- **Licence provenance is incomplete.** Of {total} manifest rows: {licence_line} — \
         per-specimen licence attribution is not yet complete \
         (`knowledge/benchmarks/PIPELINE.md` item 8)."
    );
}

fn write_reproduction(out: &mut String, baseline: &RealSpecimenBaseline) {
    let _ = writeln!(out, "## Reproduction");
    let _ = writeln!(out);
    let _ = writeln!(out, "```");
    let _ = writeln!(
        out,
        "./scripts/sync-samples.ps1 -DataRef {}",
        baseline.samples_data_sha
    );
    let _ = writeln!(
        out,
        "cargo run -p synthpass-bench --release --bin provider-bench -- \\"
    );
    let _ = writeln!(out, "  --real-specimens --mrz-only --progress \\");
    let _ = writeln!(
        out,
        "  --assert-baseline knowledge/benchmarks/real-specimen-mrz-baseline.json \\"
    );
    let _ = writeln!(out, "  --out artifacts/real-specimen-gate-report.json");
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "To regenerate this report from the resulting artifacts:"
    );
    let _ = writeln!(out);
    let _ = writeln!(out, "```");
    let _ = writeln!(
        out,
        "cargo run -p synthpass-bench --release --bin bench-report -- \\"
    );
    let _ = writeln!(
        out,
        "  --report artifacts/real-specimen-gate-report.json \\"
    );
    let _ = writeln!(
        out,
        "  --baseline knowledge/benchmarks/real-specimen-mrz-baseline.json \\"
    );
    let _ = writeln!(out, "  --corpus samples/corpus.jsonl");
    let _ = writeln!(out, "```");
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Local numbers will not match the committed baseline byte for byte — see Limitations \
         above."
    );
}

// ---------------------------------------------------------------------
// CLI entry point, called by `src/bin/bench-report.rs`.
// ---------------------------------------------------------------------

const HELP: &str = "Usage: bench-report --report PATH --baseline PATH --corpus PATH [--out PATH]\n\
     \n\
     Renders a deterministic Markdown benchmark report from three static artifacts — no corpus \
     image is read and no OCR runs.\n\
     \n\
     --report PATH    the provider-bench gate report JSON (e.g. artifacts/real-specimen-gate-report.json)\n\
     --baseline PATH  the committed real-specimen baseline (knowledge/benchmarks/real-specimen-mrz-baseline.json)\n\
     --corpus PATH    the corpus manifest (samples/corpus.jsonl)\n\
     --out PATH       write the report here instead of stdout\n";

struct Args {
    report: String,
    baseline: String,
    corpus: String,
    out: Option<String>,
}

fn parse_args(args: &[String]) -> Result<Args> {
    let mut report = None;
    let mut baseline = None;
    let mut corpus = None;
    let mut out = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--report" => {
                report = Some(args.get(i + 1).ok_or("--report requires a path")?.clone());
                i += 2;
            }
            "--baseline" => {
                baseline = Some(args.get(i + 1).ok_or("--baseline requires a path")?.clone());
                i += 2;
            }
            "--corpus" => {
                corpus = Some(args.get(i + 1).ok_or("--corpus requires a path")?.clone());
                i += 2;
            }
            "--out" => {
                out = Some(args.get(i + 1).ok_or("--out requires a path")?.clone());
                i += 2;
            }
            "--help" | "-h" => return Err(HELP.to_string()),
            other => return Err(format!("unknown argument: {other}\n\n{HELP}")),
        }
    }
    Ok(Args {
        report: report.ok_or_else(|| format!("--report is required\n\n{HELP}"))?,
        baseline: baseline.ok_or_else(|| format!("--baseline is required\n\n{HELP}"))?,
        corpus: corpus.ok_or_else(|| format!("--corpus is required\n\n{HELP}"))?,
        out,
    })
}

/// The `bench-report` binary's entire logic: parse args, read the three
/// artifacts from disk, render, write. No image, model, or network access —
/// see the module doc.
pub fn run(args: Vec<String>) -> Result<()> {
    let parsed = parse_args(&args)?;
    let report_text = std::fs::read_to_string(&parsed.report)
        .map_err(|e| format!("read {}: {e}", parsed.report))?;
    let baseline_text = std::fs::read_to_string(&parsed.baseline)
        .map_err(|e| format!("read {}: {e}", parsed.baseline))?;
    let corpus_text = std::fs::read_to_string(&parsed.corpus)
        .map_err(|e| format!("read {}: {e}", parsed.corpus))?;

    let report = parse_gate_report(&report_text)?;
    let baseline = parse_baseline(&baseline_text)?;
    let corpus = parse_corpus(&corpus_text)?;

    let markdown = render_markdown(&report, &baseline, &corpus)?;

    match parsed.out {
        Some(path) => {
            std::fs::write(&path, &markdown).map_err(|e| format!("write {path}: {e}"))?;
        }
        None => print!("{markdown}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_rate_states_a_zero_denominator_rather_than_dividing_by_it() {
        assert_eq!(format_rate(0, 0), "n/a (denominator is zero)");
    }

    #[test]
    fn format_rate_rounds_to_one_decimal_place() {
        assert_eq!(format_rate(1, 3), "1 / 3 = 33.3%");
    }

    const ONE_CORPUS_ROW: &str = "{\"dir\":\"passports\",\"filename\":\"x.jpg\",\"provenance\":\"specimen\",\"origin\":{\"licence\":\"unrecorded\"},\"mrz\":{\"issuing_state\":\"AFG\"}}";

    #[test]
    fn parse_corpus_names_the_1_based_line_number_of_a_malformed_row() {
        let text = format!("{ONE_CORPUS_ROW}\nnot json\n");
        let err = parse_corpus(&text).expect_err("the second line is not valid JSON");
        assert!(
            err.contains("line 2"),
            "expected the error to name line 2, got: {err}"
        );
    }

    #[test]
    fn parse_corpus_skips_blank_lines_without_treating_them_as_rows() {
        let text = format!("{ONE_CORPUS_ROW}\n\n");
        let rows = parse_corpus(&text).expect("a trailing blank line is not a malformed row");
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn unknown_fields_in_the_gate_report_are_ignored_not_rejected() {
        let json = r#"{
            "source": "real-specimens",
            "mrz_class_sweep_arm": "off",
            "timestamp_unix": 123,
            "a_field_added_after_this_reader_was_written": {"nested": true},
            "providers": [
                {
                    "provider_id": "mrz",
                    "documents": 1,
                    "capability": {"deterministic": true, "vision": false, "cost": "free"},
                    "ocr_arms": {"texture": "on", "order": "default", "rotate": "default", "skew": "default", "chargrid": "off"},
                    "documents_detail": [{"unrelated": "data"}]
                }
            ]
        }"#;
        let report = parse_gate_report(json).expect("unknown fields must not break parsing");
        assert_eq!(report.mrz_provider().unwrap().documents, 1);
    }
}
