//! M7 multi-provider benchmark: runs every registered [`FieldReader`] in a
//! [`ProviderCatalog`] against the same corpus and reports, per provider,
//! accuracy, speed, JSON validity, an unsupported-assertion rate, and
//! resident memory.
//!
//! Deliberately bypasses `ProviderCatalog::find_reader`'s first-match router
//! — every reader sees every document, not just the one the pipeline's
//! routing policy would have picked, because the whole point is comparing
//! providers against each other, not reproducing the router's decision.
//!
//! Two corpus sources feed the same reader loop: [`run_provider_bench`] over
//! `synthpass-gen`'s synthetic corpus (ground truth always present — every
//! `CorpusDoc` carries a generated, checksum-valid MRZ by construction), and
//! [`run_provider_bench_real`] over real specimens in `samples/` (ground
//! truth present only when a `samples/ocr_fixtures/<stem>.json` label file
//! exists). This module's original form only had the first: `CorpusDoc`
//! required a generated `Labels`, and ground truth was derived by parsing
//! its MRZ lines, so a document with no MRZ — every real MRZ-less ID-card
//! front — had no path through this harness at all. That gap is exactly why
//! a serious Tier-2 failure mode (the LLM inventing entire identities on
//! MRZ-less fronts) was found by hand instead of by CI.
//!
//! Both entry points funnel into [`run_prepped`], the one place accuracy and
//! unsupported-assertion are computed, so the two corpus sources cannot
//! silently diverge in what "correct" means.

use crate::{CorpusDoc, MissReason, RealSpecimenDoc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use synthpass_core::v2::{CoreField, MrzFormat};
use synthpass_die::{Capability, CostClass, DocumentContext, Evidence, ProviderCatalog};
use synthpass_ocr::{NativeOcr, OcrPage};

/// Stable label for a resolved ICAO 9303 MRZ format in a report — mirrors
/// `synthpass_gen::DocumentType::as_str`'s `"TD1"`/`"TD2"`/`"TD3"` naming for
/// the three the generator can produce, plus the two real-specimen-only
/// outcomes (`synthpass-gen` never emits an MRV-A/MRV-B).
fn mrz_format_str(format: MrzFormat) -> &'static str {
    match format {
        MrzFormat::Td1 => "TD1",
        MrzFormat::Td2 => "TD2",
        MrzFormat::Td3 => "TD3",
        MrzFormat::MrvA => "MRVA",
        MrzFormat::MrvB => "MRVB",
    }
}

/// Everything measured for one provider over one corpus run.
pub struct ProviderReport {
    pub provider_id: String,
    /// How many documents this provider actually saw (OCR succeeded for
    /// them). The population `speed`/`json_validity` are measured over, and
    /// the denominator `AccuracyStats::labelled_documents` is a subset of.
    pub documents: usize,
    pub capability: CapabilitySnapshot,
    pub accuracy: AccuracyStats,
    pub speed: SpeedStats,
    /// `None` for deterministic providers — there is no JSON step to measure.
    pub json_validity: Option<JsonValidityStats>,
    /// Whether a value the provider asserted appears, verbatim, anywhere in
    /// the OCR text it was given — computed only for a text-only provider.
    /// See [`UnsupportedAssertion`]'s doc for why a vision-capable provider
    /// gets `NotApplicable` instead of a number.
    pub unsupported_assertion: UnsupportedAssertion,
    /// `Capability::estimated_resident_bytes` passthrough — self-declared,
    /// always available, zero measurement cost.
    pub declared_resident_bytes: Option<u64>,
    /// Coarse process-RSS delta sampled around this provider's whole loop,
    /// only populated with `--measure-memory` (feature `measure-memory`).
    /// **Not** an isolated per-provider measurement: all providers run
    /// sequentially in one process, and a model's warm-load dominates any
    /// one call's delta. Treat as an order-of-magnitude sanity check on
    /// `declared_resident_bytes`, not a precise figure.
    pub measured_rss_delta_bytes: Option<i64>,
    /// Per-document breakdown behind the aggregates above, in corpus order.
    ///
    /// An aggregate says a provider made 162 unsupported assertions; it never
    /// says *which document* produced them, and a rate you cannot drill into
    /// is a rate you cannot act on. Always collected — it is a handful of
    /// integers per document — and rendered only on request
    /// (`provider-bench --verbose`).
    pub documents_detail: Vec<DocumentDetail>,
    /// Fraction of documents that were a genuine Tier-1 hit: MRZ found, ICAO
    /// checksums valid, and (when ground truth exists for the document)
    /// document number matches. `documents_detail.iter().filter(|d|
    /// d.miss_reason.is_none())`'s count over its length.
    ///
    /// **Not the same thing as `read_ok`/`read_ok_rate`.** `read_ok` only
    /// means "the provider returned without erroring" — for the deterministic
    /// `mrz` provider that is unconditionally `true` (`MrzReader::read` is
    /// documented to never return `Err`: a document with no MRZ is a
    /// legitimate answer, not an error), so `read_ok_rate` alone cannot tell
    /// a clean specimen from one where OCR found nothing at all. This field
    /// is the actual "did Tier-1 work" gate — the same one
    /// `synthpass-bench`'s `hit_rate` already is for the synthetic corpus —
    /// so real-specimen and synthetic tracks are finally comparable on the
    /// same axis.
    pub tier1_hit_rate: f64,
}

/// What one provider did on one document. **Shape only, never content**:
/// counts and field *names*, never the values asserted.
///
/// The values are the document's identity fields — exactly the PII
/// `CONTRIBUTING.md` forbids in any log line, and the discipline
/// `synthpass_core::fusion::Finding` already follows by carrying a field name
/// and never the value that disagreed. Knowing *that* `surname` was
/// unsupported on a given specimen is enough to go and look; printing the
/// fabricated surname itself would put document content into a terminal and a
/// JSON report for no additional diagnostic power.
pub struct DocumentDetail {
    /// Corpus-local identity: a synthetic document's seed, or a real
    /// specimen's file stem. Public-domain specimen filenames, not PII.
    pub name: String,
    /// Whether `mrz::find_and_parse` recovered an MRZ from this document's
    /// OCR text — which bucket this document counted toward.
    pub mrz_found: bool,
    /// The document's ICAO 9303 MRZ format, resolved per corpus source —
    /// the M6 plan's "report Tier-1 hit rate keyed by mrz::Format" applied to
    /// this harness's own metrics (see `miss_reason` for the
    /// `check_document`-style hit/miss equivalent this loop now has):
    ///
    /// - Synthetic corpus: always `Some`, the exact format
    ///   `synthpass_gen::Labels::mrz_format` recorded (Phase 1) — never
    ///   re-derived from the read.
    /// - Real specimens: this provider's own Tier-1 read, via
    ///   `Evidence::mrz_format` (populated only by the deterministic MRZ
    ///   provider's `mrz_format_of`) when it parsed anything at all; else a
    ///   best-effort guess from the ground-truth `mrz_line` via
    ///   `MrzFormat::guess_from_lines`. `None` when neither resolved a
    ///   format — a specimen with no MRZ is not a TD-anything.
    pub mrz_format: Option<&'static str>,
    /// Whether the provider returned a reading at all. `false` means it
    /// errored and contributed nothing to any aggregate. **Not** a Tier-1
    /// signal — see `miss_reason` for that.
    pub read_ok: bool,
    /// `Evidence::mrz_checksums_valid` passthrough — `false` whenever
    /// `read_ok` is `false` too (nothing to check), since a provider that
    /// errored produced no MRZ to validate.
    pub mrz_checksums_valid: bool,
    /// Why this document is not a Tier-1 hit. `None` on a genuine hit: MRZ
    /// found, checksums valid, and (when labelled) document number matches
    /// ground truth. Mirrors `synthpass-bench`'s own miss classification
    /// (`crate::MissReason`) so real-specimen and synthetic-corpus misses
    /// aggregate the same way — see `crate::miss_kind`.
    pub miss_reason: Option<MissReason>,
    pub assertions_total: usize,
    pub assertions_unsupported: usize,
    /// Which fields were asserted but absent from the OCR text. Names only.
    pub unsupported_fields: Vec<&'static str>,
}

pub struct CapabilitySnapshot {
    pub deterministic: bool,
    pub vision: bool,
    pub cost: &'static str,
}

impl CapabilitySnapshot {
    fn from(capability: &Capability) -> Self {
        Self {
            deterministic: capability.deterministic,
            vision: capability.vision,
            cost: match capability.cost {
                CostClass::Free => "free",
                CostClass::Cheap => "cheap",
                CostClass::Expensive => "expensive",
            },
        }
    }
}

/// Field-match rate and mean CER, scored only over documents that actually
/// have ground truth.
///
/// `field_match_rate`/`mean_cer`/`per_field_cer` are `Option`, not `f64`
/// defaulting to `0.0`, on purpose: `0.0` accuracy is a measurement (the
/// provider got everything wrong), while `None` is the honest report of "no
/// labelled population to measure against." Collapsing the two would let an
/// all-unlabelled real-specimen run silently print "0% field match" — a
/// fabricated number, not a measured one, for exactly the population this
/// crate's real-specimen mode exists to cover honestly (every MRZ-less
/// ID-card front in `samples/` has no `ocr_fixtures` label). Carrying
/// `labelled_documents` alongside makes the `Option` legible on its own: a
/// report can always say *how many* documents a rate (or its absence) was
/// computed over.
pub struct AccuracyStats {
    /// How many of `documents` contributed at least one ground-truth field.
    /// Always equal to `documents` for the synthetic corpus (every
    /// `CorpusDoc` has labels by construction); a subset of it for real
    /// specimens (only the ones with a matching `samples/ocr_fixtures/
    /// <stem>.json`).
    pub labelled_documents: usize,
    /// Fraction of `(document, field)` pairs where the provider's answer
    /// exactly matches ground truth, over fields ground truth actually has a
    /// value for. `None` iff `labelled_documents == 0`.
    pub field_match_rate: Option<f64>,
    /// Mean character error rate over the same population, via
    /// [`crate::cer`] — reuses the exact metric `synthpass-bench`'s Tier-1
    /// report already uses, so the two numbers are comparable. `None` iff
    /// `labelled_documents == 0`.
    pub mean_cer: Option<f64>,
    /// Per-field mean CER. A field's entry is `None` when no document in the
    /// labelled population had ground truth for that specific field (for
    /// instance every real specimen loaded so far omitting
    /// `personal_number`), for the same reason the whole-report rate is
    /// `Option`.
    pub per_field_cer: Vec<(&'static str, Option<f64>)>,
}

/// The unsupported-assertion metric outcome for one provider: either a
/// computed rate, or the capability reason it wasn't computed.
///
/// The predicate — "the answer must appear, verbatim, in the OCR text the
/// provider was given" — is correct for a **text-only** provider:
/// `synthpass-llm` consumes OCR Markdown, so a value not in that text came
/// from the model's weights, not the document. It is **invalid** for a
/// **vision** provider reading pixels: a VLM producing a correct value that
/// appears nowhere in the OCR text is exactly the point of using one. Ship
/// this metric unconditionally and it would report a working vision
/// provider as maximally hallucinating precisely when it is right — the
/// scenario `knowledge/ROADMAP.md`'s M7 section calls out by name for the
/// planned Qwen-vs-Moondream comparison. So this is computed only when
/// `Capability::vision` is `false`; a vision-capable provider gets
/// `NotApplicable` with the reason recorded, never a silently-wrong number.
pub enum UnsupportedAssertion {
    /// Computed over the non-null answered fields of a `!capability.vision`
    /// provider, split by whether the document gave it an MRZ to anchor on.
    Computed {
        overall: AssertionBucket,
        /// Documents where `mrz::find_and_parse` recovered an MRZ. `None`
        /// when the corpus contained no such document.
        with_mrz_anchor: Option<AssertionBucket>,
        /// Documents with no MRZ at all — the population where a text-only
        /// provider has no deterministic anchor whatsoever. `None` when the
        /// corpus contained no such document, which is always the case for
        /// the synthetic corpus (every generated document carries an MRZ by
        /// construction).
        without_mrz_anchor: Option<AssertionBucket>,
    },
    /// Not computed. `capability.vision` was `true` for this provider.
    NotApplicable { reason: &'static str },
}

/// One unsupported-assertion measurement over a named subset of documents.
///
/// `documents` is carried alongside the rate because the two halves of the
/// split are not the same size and a rate alone would hide that: "40% over
/// 4 documents" and "40% over 120" are different claims, and the whole point
/// of this split is that the smaller half is the one nobody had measured.
pub struct AssertionBucket {
    /// `None` when the provider made **no assertions at all** over this
    /// subset — not the same fact as a measured rate of zero, and on this
    /// metric the difference is the whole point.
    ///
    /// The first real run made that concrete: on the 22 corpus documents with
    /// no MRZ, the deterministic reader asserted nothing whatsoever, while the
    /// LLM asserted 162 field values. Rendering the former as `0.0` printed it
    /// as "measured, and perfectly clean" — praise for the one provider that
    /// had correctly declined to answer, in the same column where a genuine
    /// 0.0 would mean "answered a lot, all of it supported". Exactly the
    /// fabricated-zero that [`AccuracyStats`]'s `Option`s exist to prevent,
    /// one struct over.
    pub rate: Option<f64>,
    pub assertions_total: usize,
    pub documents: usize,
}

impl AssertionBucket {
    /// `None` when the subset had no documents at all — absent, rather than a
    /// bucket that would have to invent a rate for an empty population. A
    /// bucket that *does* exist may still carry `rate: None`; see
    /// [`AssertionBucket::rate`].
    fn new(unsupported: usize, total: usize, documents: usize) -> Option<Self> {
        (documents > 0).then_some(Self {
            rate: (total > 0).then(|| unsupported as f64 / total as f64),
            assertions_total: total,
            documents,
        })
    }
}

const VISION_REASON: &str = "capability.vision is true — the verbatim-in-OCR-text predicate \
    assumes a text-only provider and would misreport a correct pixel-grounded read as an \
    unsupported assertion";

pub struct SpeedStats {
    pub mean: Duration,
    pub p50: Duration,
    pub p95: Duration,
}

pub struct JsonValidityStats {
    /// `synthpass_llm::repair::repair_fallbacks()` delta across this
    /// provider's whole loop — how many of its answers needed JSON repair
    /// rather than parsing clean the first time.
    pub repair_fallbacks: u64,
    pub documents: usize,
}

/// The 10 [`CoreField`]s, paired with how to read the matching value off
/// [`mrz::MrzData`] — the synthetic corpus's ground truth. A local table
/// rather than reusing `synthpass-bench`'s private `COMPARED_FIELDS`: that
/// one is keyed by `&str` for the Tier-1-only report; this one is keyed by
/// `CoreField` so it can index `ExtractionFields::get` directly.
fn mrz_field(field: CoreField, truth: &mrz::MrzData) -> String {
    match field {
        CoreField::DocumentType => truth.document_type.clone(),
        CoreField::IssuingCountry => truth.issuing_country.clone(),
        CoreField::DocumentNumber => truth.document_number.clone(),
        CoreField::Surname => truth.surname.clone(),
        CoreField::GivenNames => truth.given_names.clone(),
        CoreField::Nationality => truth.nationality.clone(),
        CoreField::DateOfBirth => truth.date_of_birth.clone(),
        CoreField::Sex => truth.sex.clone(),
        CoreField::DateOfExpiry => truth.date_of_expiry.clone(),
        CoreField::PersonalNumber => truth.personal_number.clone().unwrap_or_default(),
    }
}

/// Ground truth for a synthetic document, keyed by [`CoreField`]. A field
/// with an empty string in `mrz::MrzData` (only ever `personal_number`,
/// which is genuinely optional in a TD3 MRZ) is omitted rather than stored
/// empty — "no ground truth for this field" and "ground truth is the empty
/// string" must be distinguishable, and only the former should be excluded
/// from `field_match_rate`/`mean_cer`.
fn mrz_ground_truth(truth: &mrz::MrzData) -> HashMap<CoreField, String> {
    CoreField::ALL
        .iter()
        .filter_map(|&field| {
            let value = mrz_field(field, truth);
            (!value.is_empty()).then_some((field, value))
        })
        .collect()
}

/// Ground truth for a real specimen, keyed by [`CoreField`], from its
/// optional v1 `Extraction` label. `None`/empty fields are omitted for the
/// same reason as [`mrz_ground_truth`]: an absent field in the label file
/// (most of them, for any specimen not hand-verified in full) must read as
/// "no ground truth," not as a false "expected empty string" that would
/// score a non-empty answer as wrong.
fn extraction_ground_truth(extraction: &synthpass_core::Extraction) -> HashMap<CoreField, String> {
    let fields: [(CoreField, &Option<String>); 10] = [
        (CoreField::DocumentType, &extraction.document_type),
        (CoreField::IssuingCountry, &extraction.issuing_country),
        (CoreField::DocumentNumber, &extraction.document_number),
        (CoreField::Surname, &extraction.surname),
        (CoreField::GivenNames, &extraction.given_names),
        (CoreField::Nationality, &extraction.nationality),
        (CoreField::DateOfBirth, &extraction.date_of_birth),
        (CoreField::Sex, &extraction.sex),
        (CoreField::DateOfExpiry, &extraction.date_of_expiry),
        (CoreField::PersonalNumber, &extraction.personal_number),
    ];
    fields
        .into_iter()
        .filter_map(|(field, value)| match value.as_deref() {
            Some(v) if !v.is_empty() => Some((field, v.to_string())),
            _ => None,
        })
        .collect()
}

/// One OCR'd document, ready for the reader loop: the OCR text/geometry,
/// this document's optional per-field ground truth, and the on-disk path its
/// image was written to.
///
/// `image_path` is deliberately kept alive rather than cleaned up the moment
/// OCR finishes (contrast [`crate::check_document`], which deletes its temp
/// file immediately after OCR): [`DocumentContext::with_image`] needs a live
/// `&Path` for every reader that sees this document, and readers are visited
/// in the *outer* loop of [`run_prepped`] (once per provider, each pass over
/// every document) rather than once per document — so the path has to
/// outlive the whole run, not just one OCR call. [`run_prepped`] removes
/// every `image_path` after the last reader has finished with it.
struct BenchPage {
    /// Corpus-local identity, carried for `DocumentDetail`: a synthetic
    /// document's seed or a real specimen's file stem.
    name: String,
    page: OcrPage,
    ground_truth: Option<HashMap<CoreField, String>>,
    image_path: PathBuf,
    /// Whether `mrz::find_and_parse` recovered *any* MRZ from this document's
    /// OCR text — parsed, not necessarily checksum-valid.
    ///
    /// This is the "zero anchor vs some anchor" split, and it is deliberately
    /// the found/not-found line rather than the checksum-valid/invalid one.
    /// It mirrors the routing distinction the pipeline already draws:
    /// `EscalationKind::MrzNotFound` versus `MrzChecksumFailed`
    /// (`synthpass_die::RoutingPolicy::decide`, whose clause order is
    /// documented as normative for exactly this reason). A checksum-partial
    /// read still pins several fields mathematically; no MRZ at all pins
    /// nothing, and that is the population where a text-only provider has
    /// nothing to be right about — which is what
    /// [`UnsupportedAssertion`]'s split exists to measure separately.
    mrz_found: bool,
    /// `true` for a synthetic-corpus document (from [`prep_corpus`]), `false`
    /// for a real specimen (from [`prep_specimens`]) — which of
    /// [`DocumentDetail::mrz_format`]'s two resolution rules applies.
    synthetic: bool,
    /// Precomputed format info [`run_prepped`] resolves into
    /// [`DocumentDetail::mrz_format`]:
    /// - synthetic: always `Some`, the exact `Labels::mrz_format`.
    /// - real specimens: a best-effort guess from the ground-truth
    ///   `mrz_line` (`MrzFormat::guess_from_lines`), used only when no
    ///   provider's own Tier-1 read resolves a format for this document.
    known_or_guessed_format: Option<&'static str>,
}

/// Writes `image` to a uniquely-named temp file and OCRs it via
/// `recognize_detailed`, returning both the OCR result and the path — the
/// caller owns cleanup (see [`BenchPage`]'s doc for why the path can't be
/// removed here). `name` disambiguates the temp filename across documents in
/// the same run (a corpus seed for synthetic documents, a file stem for real
/// specimens); the process id further disambiguates across concurrent runs,
/// the same pattern [`crate::check_document`] and the original single-corpus
/// version of this function both already used.
fn ocr_and_keep_path(
    ocr: &NativeOcr,
    image: &image::DynamicImage,
    name: &str,
) -> Result<(OcrPage, PathBuf), String> {
    let path =
        std::env::temp_dir().join(format!("provider-bench-{}-{name}.png", std::process::id(),));
    image
        .save(&path)
        .map_err(|e| format!("failed to write temp image: {e}"))?;
    match ocr.recognize_detailed(&path) {
        Ok(page) => Ok((page, path)),
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            Err(e)
        }
    }
}

/// OCRs every document in the synthetic `corpus`, pairing each with its
/// always-present ground truth (derived from `Labels`' MRZ lines, parsed back
/// through the per-format dispatcher [`crate::parse_ground_truth_mrz`] the
/// same way [`crate::check_document`] does — TD1/TD2/TD3 alike, not TD3
/// only). `None` at an index means that document's OCR or ground-truth parse
/// failed — carried as a hole rather than shrinking the `Vec`, so the
/// original document count is still recoverable if a caller wants it.
fn prep_corpus(ocr: &NativeOcr, corpus: &[CorpusDoc]) -> Vec<Option<BenchPage>> {
    corpus
        .iter()
        .map(|doc| {
            let (page, image_path) =
                ocr_and_keep_path(ocr, &doc.image, &doc.seed.to_string()).ok()?;
            let truth = crate::parse_ground_truth_mrz(&doc.labels).ok()?;
            // Read from the OCR text, never from `labels`: the question is
            // what this run's OCR pass actually recovered, which is what a
            // provider had to work with. A generated document always *has*
            // an MRZ, but a degraded capture can leave none of it legible.
            let mrz_found = mrz::find_and_parse(&page.text).is_ok();
            Some(BenchPage {
                name: doc.seed.to_string(),
                page,
                ground_truth: Some(mrz_ground_truth(&truth)),
                image_path,
                mrz_found,
                synthetic: true,
                known_or_guessed_format: Some(doc.labels.mrz_format.as_str()),
            })
        })
        .collect()
}

/// OCRs every document in `specimens`, pairing each with its optional
/// ground truth (`None` when the specimen had no `samples/ocr_fixtures/
/// <stem>.json` label). Unlike [`prep_corpus`], a missing label is not a
/// reason to drop the document — only an OCR failure is.
fn prep_specimens(ocr: &NativeOcr, specimens: &[RealSpecimenDoc]) -> Vec<Option<BenchPage>> {
    specimens
        .iter()
        .map(|doc| {
            let (page, image_path) = ocr_and_keep_path(ocr, &doc.image, &doc.name).ok()?;
            let ground_truth = doc.labels.as_ref().map(extraction_ground_truth);
            let mrz_found = mrz::find_and_parse(&page.text).is_ok();
            // Best-effort fallback only — used in `run_prepped` when no
            // provider's own Tier-1 read resolves a format for this
            // document. Reuses `MrzFormat::guess_from_lines` rather than a
            // new heuristic; `None` when there is no ground-truth `mrz_line`
            // to guess from at all (most of `samples/`, by construction).
            let known_or_guessed_format = doc
                .labels
                .as_ref()
                .and_then(|l| l.mrz_line.as_deref())
                .and_then(MrzFormat::guess_from_lines)
                .map(mrz_format_str);
            Some(BenchPage {
                name: doc.name.clone(),
                page,
                ground_truth,
                image_path,
                mrz_found,
                synthetic: false,
                known_or_guessed_format,
            })
        })
        .collect()
}

/// Runs every reader in `catalog` against every document in `corpus`,
/// OCRing each document once and sharing that OCR pass across all readers —
/// the comparison is only fair if every provider sees identical input.
/// Ground truth is always present (`CorpusDoc`'s contract) — every document
/// contributes to `AccuracyStats::labelled_documents`.
pub async fn run_provider_bench(
    catalog: &ProviderCatalog,
    ocr: &NativeOcr,
    corpus: &[CorpusDoc],
    measure_memory: bool,
) -> Vec<ProviderReport> {
    let prepped = prep_corpus(ocr, corpus);
    run_prepped(catalog, &prepped, measure_memory, false).await
}

/// Runs every reader in `catalog` against every real specimen in
/// `specimens`, the same way [`run_provider_bench`] does for the synthetic
/// corpus, except ground truth is present only for specimens with a
/// `samples/ocr_fixtures/<stem>.json` label — see [`AccuracyStats`]'s doc
/// for how that is reported honestly rather than scored as misses.
pub async fn run_provider_bench_real(
    catalog: &ProviderCatalog,
    ocr: &NativeOcr,
    specimens: &[RealSpecimenDoc],
    measure_memory: bool,
    dump_ocr: bool,
) -> Vec<ProviderReport> {
    let prepped = prep_specimens(ocr, specimens);
    run_prepped(catalog, &prepped, measure_memory, dump_ocr).await
}

/// The literal MRZ substring a date field's ISO value (`YYYY-MM-DD`) was
/// parsed from — `YYMMDD`, e.g. `"1974-08-12"` -> `"740812"`.
///
/// `synthpass_core::normalize` reformats every date field to ISO before it
/// reaches this harness (see `crates/synthpass-core/src/normalize.rs`), but
/// the OCR text a text-only provider actually read only ever contains the
/// raw MRZ digits (or a printed-page date in some other format) — never the
/// ISO string. Checking the verbatim ISO value against OCR text therefore
/// flags a *correct* date read as an unsupported assertion almost every
/// time, which is exactly the near-universal `date_of_birth`/`date_of_expiry`
/// pattern seen on real runs. Falling back to this raw digit string when the
/// direct match misses recovers the true positive without weakening the
/// check for any other field.
fn mrz_date_digits(iso_date: &str) -> Option<String> {
    let (year, rest) = iso_date.split_once('-')?;
    let (month, day) = rest.split_once('-')?;
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    if !year.bytes().all(|b| b.is_ascii_digit())
        || !month.bytes().all(|b| b.is_ascii_digit())
        || !day.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some(format!("{}{month}{day}", &year[2..]))
}

/// Whether `value` (the field this provider asserted) is grounded in
/// `ocr_text` — a verbatim substring match for every field, plus a
/// raw-MRZ-digit fallback for date fields specifically (see
/// [`mrz_date_digits`]).
fn is_supported(field: CoreField, value: &str, ocr_text_lower: &str) -> bool {
    if ocr_text_lower.contains(&value.to_lowercase()) {
        return true;
    }
    if matches!(field, CoreField::DateOfBirth | CoreField::DateOfExpiry) {
        if let Some(digits) = mrz_date_digits(value) {
            return ocr_text_lower.contains(&digits);
        }
    }
    false
}

/// The shared reader loop both [`run_provider_bench`] and
/// [`run_provider_bench_real`] funnel into — the one place accuracy and
/// unsupported-assertion are computed, so the two corpus sources cannot
/// silently diverge in what "correct" or "unsupported" means.
///
/// `dump_ocr`: when a document's `miss_reason` resolves to
/// `MissReason::ChecksumFailed`, print its raw MRZ zone text (debug-quoted,
/// so a misread or invisible character is visible) and which check digit(s)
/// failed. Mirrors `synthpass-bench`'s own `--dump-ocr`, but scoped to this
/// one miss kind rather than every document: `checksum_failed` is the
/// unambiguous signal (OCR found MRZ-shaped text and misread a character in
/// it — unlike `no_mrz_found`, which also catches genuinely MRZ-less
/// documents), and a real-specimen run is large enough that dumping every
/// hit would be noise a synthetic diagnostic run never has to contend with.
/// `synthpass_bench::CorpusDoc` runs (`run_provider_bench`) never pass
/// `true` here — `synthpass-bench`'s own `--dump-ocr` already covers that
/// path.
async fn run_prepped(
    catalog: &ProviderCatalog,
    prepped: &[Option<BenchPage>],
    measure_memory: bool,
    dump_ocr: bool,
) -> Vec<ProviderReport> {
    let ocr_documents = prepped.iter().filter(|p| p.is_some()).count();
    let labelled_documents = prepped
        .iter()
        .filter(|p| matches!(p, Some(bp) if bp.ground_truth.is_some()))
        .count();
    // Counted over documents, not per reader: the split is a property of the
    // corpus, identical for every provider, so counting it once also makes it
    // impossible for two providers to disagree about how many anchored
    // documents they saw.
    let anchored_documents = prepped
        .iter()
        .filter(|p| matches!(p, Some(bp) if bp.mrz_found))
        .count();
    let unanchored_documents = prepped
        .iter()
        .filter(|p| matches!(p, Some(bp) if !bp.mrz_found))
        .count();

    let mut reports = Vec::with_capacity(catalog.readers().len());
    for reader in catalog.readers() {
        let capability = reader.capability();
        let rss_before = measure_memory.then(sample_rss).flatten();
        let repair_before = synthpass_llm::repair::repair_fallbacks();

        let mut elapsed_per_doc = Vec::with_capacity(prepped.len());
        let mut field_hits = 0usize;
        let mut field_total = 0usize;
        let mut cer_sum = 0.0f64;
        let mut cer_count = 0usize;
        let mut per_field: Vec<(&'static str, f64, usize)> = CoreField::ALL
            .iter()
            .map(|f| (f.as_str(), 0.0, 0))
            .collect();
        let mut assertions_total = 0usize;
        let mut assertions_unsupported = 0usize;
        // Same two counters again, restricted to documents with / without an
        // MRZ anchor. Accumulated separately rather than derived afterwards
        // because an assertion belongs to the document it was made about, and
        // that association is only available here inside the loop.
        let mut documents_detail: Vec<DocumentDetail> = Vec::with_capacity(prepped.len());
        let mut anchored_total = 0usize;
        let mut anchored_unsupported = 0usize;
        let mut unanchored_total = 0usize;
        let mut unanchored_unsupported = 0usize;

        for bench_page in prepped.iter().flatten() {
            // Mirrors `synthpass_pipeline::Pipeline::ocr_and_tier1`'s own
            // `mrz::find_and_parse(&markdown)` — a *read* of this provider's
            // OCR text, not any ground-truth labels: the hint must reflect
            // what this run's OCR pass actually recovered, including a
            // checksum-partial read, the same as production. `mrz_hint`
            // itself is a no-op unless `SYNTHPASS_LLM_MRZ_HINT=1` is set, so
            // this harness measures exactly the same gate the pipeline does.
            let read_mrz = mrz::find_and_parse(&bench_page.page.text).ok();
            let hint = synthpass_pipeline::mrz_hint(read_mrz.as_ref());
            // `with_image`: harmless for every provider shipped today (all
            // text-only, so `DocumentContext::image` is ignored), and the
            // reason this harness is also the substrate for the planned
            // vision-provider comparison — see this file's top doc comment.
            let mut ctx = DocumentContext::from_text(&bench_page.page.text)
                .with_image(&bench_page.image_path);
            if let Some(hint) = &hint {
                ctx = ctx.with_mrz_hint(hint);
            }
            let started = Instant::now();
            let reading = reader.read(&ctx).await;
            elapsed_per_doc.push(started.elapsed());

            // See `DocumentDetail::mrz_format`'s doc: synthetic documents
            // always resolve to the label's exact format, independent of
            // whether this read even succeeded; real specimens prefer this
            // provider's own Tier-1 read (`Evidence::mrz_format`, populated
            // only by the deterministic MRZ provider) and fall back to the
            // ground-truth guess computed once in `prep_specimens`.
            let resolve_format = |evidence: Option<&Evidence>| -> Option<&'static str> {
                if bench_page.synthetic {
                    bench_page.known_or_guessed_format
                } else {
                    evidence
                        .and_then(|e| e.mrz_format)
                        .map(mrz_format_str)
                        .or(bench_page.known_or_guessed_format)
                }
            };

            let reading = match reading {
                Ok(reading) => reading,
                Err(e) => {
                    // Recorded rather than skipped silently: a provider that
                    // errored on a document contributed nothing to any
                    // aggregate, and a per-document view that simply omitted
                    // the row would make that indistinguishable from a
                    // document it handled cleanly with no assertions.
                    documents_detail.push(DocumentDetail {
                        name: bench_page.name.clone(),
                        mrz_found: bench_page.mrz_found,
                        mrz_format: resolve_format(None),
                        read_ok: false,
                        mrz_checksums_valid: false,
                        miss_reason: Some(MissReason::OcrError(e.to_string())),
                        assertions_total: 0,
                        assertions_unsupported: 0,
                        unsupported_fields: Vec::new(),
                    });
                    continue;
                }
            };
            let mrz_format = resolve_format(Some(&reading.evidence));

            let mut doc_assertions = 0usize;
            let mut doc_unsupported_fields: Vec<&'static str> = Vec::new();
            // Identical for every field in the loop below — computed once
            // per document rather than once per assertion.
            let ocr_text_lower = bench_page.page.text.to_lowercase();

            for (i, field) in CoreField::ALL.iter().enumerate() {
                let got = reading.extraction.fields.get(*field);

                // Field-match / CER: only over fields this document has
                // ground truth for — see `AccuracyStats`'s doc on why an
                // absent label must not be scored as a miss.
                if let Some(expected) = bench_page
                    .ground_truth
                    .as_ref()
                    .and_then(|gt| gt.get(field))
                {
                    field_total += 1;
                    let got_str = got.unwrap_or("");
                    if got_str == expected.as_str() {
                        field_hits += 1;
                    }
                    let field_cer = crate::cer(expected, got_str);
                    cer_sum += field_cer;
                    cer_count += 1;
                    per_field[i].1 += field_cer;
                    per_field[i].2 += 1;
                }

                // Unsupported-assertion: needs no ground truth at all — only
                // the OCR text this provider was given — but is only a
                // meaningful predicate for a text-only provider. See
                // `UnsupportedAssertion`'s doc.
                if !capability.vision {
                    if let Some(value) = got {
                        if !value.is_empty() {
                            let unsupported = !is_supported(*field, value, &ocr_text_lower);
                            assertions_total += 1;
                            if unsupported {
                                assertions_unsupported += 1;
                            }
                            let (total, bad) = if bench_page.mrz_found {
                                (&mut anchored_total, &mut anchored_unsupported)
                            } else {
                                (&mut unanchored_total, &mut unanchored_unsupported)
                            };
                            *total += 1;
                            if unsupported {
                                *bad += 1;
                            }

                            doc_assertions += 1;
                            if unsupported {
                                // The field *name*, never `value` — see
                                // `DocumentDetail`'s doc on why the asserted
                                // value must not reach a log or a report.
                                doc_unsupported_fields.push(field.as_str());
                            }
                        }
                    }
                }
            }

            // Mirrors `run_check`'s miss classification in `lib.rs` exactly:
            // no MRZ found, then checksum validity, then — only when this
            // document is labelled — a document-number check against ground
            // truth. A specimen with no label that clears the first two gets
            // no third check at all, since there is no truth to compare
            // against; that is a hit, not an unknown.
            let miss_reason = if !bench_page.mrz_found {
                Some(MissReason::NoMrzFound(String::new()))
            } else if !reading.evidence.mrz_checksums_valid {
                Some(MissReason::ChecksumFailed)
            } else {
                bench_page
                    .ground_truth
                    .as_ref()
                    .and_then(|gt| gt.get(&CoreField::DocumentNumber))
                    .and_then(|expected| {
                        let got = reading
                            .extraction
                            .fields
                            .get(CoreField::DocumentNumber)
                            .unwrap_or("");
                        (got != expected).then(|| MissReason::DocumentNumberMismatch {
                            got: got.to_string(),
                            expected: expected.clone(),
                        })
                    })
            };

            if dump_ocr && matches!(miss_reason, Some(MissReason::ChecksumFailed)) {
                println!(
                    "--- {} ({}) raw MRZ zone ---",
                    bench_page.name,
                    reader.id().as_str()
                );
                match &read_mrz {
                    Some(data) => {
                        for (i, line) in data.mrz_lines.lines().enumerate() {
                            println!("  [{i}] {line:?}");
                        }
                        let failed: Vec<String> =
                            data.checks.failed().iter().map(|f| f.to_string()).collect();
                        println!("  failed check digit(s): {}", failed.join(", "));
                    }
                    // read_mrz is None here only if mrz::find_and_parse found no
                    // candidate at all despite bench_page.mrz_found being true —
                    // shouldn't happen (both are the same call on the same text),
                    // but printed rather than silently skipped if it ever does.
                    None => println!("  (no parsed MRZ data despite mrz_found)"),
                }
            }

            documents_detail.push(DocumentDetail {
                name: bench_page.name.clone(),
                mrz_found: bench_page.mrz_found,
                mrz_format,
                read_ok: true,
                mrz_checksums_valid: reading.evidence.mrz_checksums_valid,
                miss_reason,
                assertions_total: doc_assertions,
                assertions_unsupported: doc_unsupported_fields.len(),
                unsupported_fields: doc_unsupported_fields,
            });
        }

        let repair_after = synthpass_llm::repair::repair_fallbacks();
        let rss_after = measure_memory.then(sample_rss).flatten();

        elapsed_per_doc.sort();
        let speed = SpeedStats {
            mean: mean_duration(&elapsed_per_doc),
            p50: percentile_duration(&elapsed_per_doc, 0.50),
            p95: percentile_duration(&elapsed_per_doc, 0.95),
        };

        let unsupported_assertion = if capability.vision {
            UnsupportedAssertion::NotApplicable {
                reason: VISION_REASON,
            }
        } else {
            UnsupportedAssertion::Computed {
                overall: AssertionBucket::new(
                    assertions_unsupported,
                    assertions_total,
                    ocr_documents,
                )
                // An empty corpus is the one case with no documents at all,
                // so there is no rate to report and nothing was asserted.
                .unwrap_or(AssertionBucket {
                    rate: None,
                    assertions_total: 0,
                    documents: 0,
                }),
                with_mrz_anchor: AssertionBucket::new(
                    anchored_unsupported,
                    anchored_total,
                    anchored_documents,
                ),
                without_mrz_anchor: AssertionBucket::new(
                    unanchored_unsupported,
                    unanchored_total,
                    unanchored_documents,
                ),
            }
        };

        let tier1_hits = documents_detail
            .iter()
            .filter(|d| d.miss_reason.is_none())
            .count();
        let tier1_hit_rate = if documents_detail.is_empty() {
            0.0
        } else {
            tier1_hits as f64 / documents_detail.len() as f64
        };

        reports.push(ProviderReport {
            provider_id: reader.id().as_str().to_string(),
            documents: ocr_documents,
            capability: CapabilitySnapshot::from(capability),
            accuracy: AccuracyStats {
                labelled_documents,
                field_match_rate: (field_total > 0).then(|| field_hits as f64 / field_total as f64),
                mean_cer: (cer_count > 0).then(|| cer_sum / cer_count as f64),
                per_field_cer: per_field
                    .into_iter()
                    .map(|(name, sum, n)| (name, (n > 0).then(|| sum / n as f64)))
                    .collect(),
            },
            speed,
            json_validity: (!capability.deterministic).then(|| JsonValidityStats {
                repair_fallbacks: repair_after.saturating_sub(repair_before),
                documents: ocr_documents,
            }),
            unsupported_assertion,
            declared_resident_bytes: capability.estimated_resident_bytes,
            documents_detail,
            tier1_hit_rate,
            measured_rss_delta_bytes: match (rss_before, rss_after) {
                (Some(before), Some(after)) => Some(after as i64 - before as i64),
                _ => None,
            },
        });
    }

    // Every reader has now seen every document — safe to remove the temp
    // images `prep_corpus`/`prep_specimens` wrote and kept alive for
    // `DocumentContext::with_image`.
    for bench_page in prepped.iter().flatten() {
        let _ = std::fs::remove_file(&bench_page.image_path);
    }

    reports
}

fn mean_duration(sorted: &[Duration]) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    sorted.iter().sum::<Duration>() / sorted.len() as u32
}

/// `sorted` must already be sorted ascending. `p` in `[0, 1]`.
fn percentile_duration(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[cfg(feature = "measure-memory")]
fn sample_rss() -> Option<u64> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut system = System::new();
    let pid = Pid::from_u32(std::process::id());
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::everything(),
    );
    system.process(pid).map(|p| p.memory())
}

#[cfg(not(feature = "measure-memory"))]
fn sample_rss() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use synthpass_core::v2::ExtractionV2;
    use synthpass_die::{FieldReader, IntelligenceProvider, ProviderError, ProviderId, Reading};

    /// A fixed-answer provider for testing the harness's own comparison
    /// logic without needing a real MRZ reader or the GGUF model.
    struct FixedReader {
        capability: Capability,
        surname: &'static str,
    }

    #[async_trait::async_trait]
    impl IntelligenceProvider for FixedReader {
        fn id(&self) -> ProviderId {
            ProviderId("fixed-test-reader")
        }
        fn capability(&self) -> &Capability {
            &self.capability
        }
        fn describe(&self) -> String {
            "fixed-answer test reader".into()
        }
    }

    #[async_trait::async_trait]
    impl FieldReader for FixedReader {
        async fn read(&self, _ctx: &DocumentContext<'_>) -> Result<Reading, ProviderError> {
            // `ExtractionV2` implements `Drop` (it zeroizes PII on drop), so
            // struct-update syntax (`..Default::default()`) is unavailable —
            // that would require partially moving out of a `Drop` type. A
            // field assignment on an owned `mut` value has no such
            // restriction.
            let mut extraction = ExtractionV2::default();
            extraction.fields.surname = Some(self.surname.to_string());
            Ok(Reading {
                extraction,
                evidence: Evidence::default(),
                by: self.id(),
            })
        }
    }

    #[test]
    fn unsupported_assertion_is_flagged_when_value_is_absent_from_input() {
        let ocr_text = "some markdown mentioning DOE but not the other name";
        let echoed = "DOE";
        let fabricated = "SMITH";
        assert!(ocr_text.to_lowercase().contains(&echoed.to_lowercase()));
        assert!(!ocr_text.to_lowercase().contains(&fabricated.to_lowercase()));
    }

    #[test]
    fn mrz_date_digits_converts_iso_to_the_printed_mrz_substring() {
        assert_eq!(mrz_date_digits("1974-08-12"), Some("740812".to_string()));
        assert_eq!(mrz_date_digits("2005-01-09"), Some("050109".to_string()));
    }

    #[test]
    fn mrz_date_digits_rejects_non_iso_input() {
        assert_eq!(mrz_date_digits("12.08.1974"), None);
        assert_eq!(mrz_date_digits(""), None);
        assert_eq!(mrz_date_digits("DOE"), None);
    }

    #[test]
    fn a_correct_iso_date_absent_from_ocr_text_is_still_supported_via_mrz_digits() {
        // The OCR text carries the raw MRZ line (YYMMDD), never the ISO
        // string synthpass_core::normalize reformats it to -- this is the
        // false-positive the near-universal date_of_birth/date_of_expiry
        // unsupported-assertion pattern on real runs was coming from.
        let ocr_text_lower = "p<utoDOE<<JOHN<<<<<<<<<<<<<<<<<<<<<<<<<<<<<\n\
            l898902c3utO7408122m1204159<<<<<<<<<<<<<<06"
            .to_lowercase();
        assert!(is_supported(
            CoreField::DateOfBirth,
            "1974-08-12",
            &ocr_text_lower
        ));
    }

    #[test]
    fn a_fabricated_date_is_still_unsupported() {
        let ocr_text_lower = "l898902c3uto7408122m1204159<<<<<<<<<<<<<<06".to_lowercase();
        assert!(!is_supported(
            CoreField::DateOfBirth,
            "1999-12-31",
            &ocr_text_lower
        ));
    }

    #[test]
    fn the_mrz_digit_fallback_is_scoped_to_date_fields_only() {
        // "740812" happening to be a substring of a non-date field's OCR
        // text must not make an unrelated fabricated value "supported" --
        // the fallback only ever applies to DateOfBirth/DateOfExpiry.
        let ocr_text_lower = "document number 740812 issued".to_lowercase();
        assert!(!is_supported(
            CoreField::DocumentNumber,
            "1974-08-12",
            &ocr_text_lower
        ));
    }

    #[tokio::test]
    async fn field_reader_answering_a_value_present_in_input_is_not_unsupported() {
        let reader = FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
        };
        let ctx = DocumentContext::from_text("surname DOE date of birth 1990");
        let reading = reader.read(&ctx).await.expect("reader never fails");
        let surname = reading.extraction.fields.get(CoreField::Surname).unwrap();
        assert!(ctx.text.to_lowercase().contains(&surname.to_lowercase()));
    }

    #[tokio::test]
    async fn field_reader_answering_a_fabricated_value_is_unsupported() {
        let reader = FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "SMITH",
        };
        let ctx = DocumentContext::from_text("surname DOE date of birth 1990");
        let reading = reader.read(&ctx).await.expect("reader never fails");
        let surname = reading.extraction.fields.get(CoreField::Surname).unwrap();
        assert!(!ctx.text.to_lowercase().contains(&surname.to_lowercase()));
    }

    #[test]
    fn percentile_duration_picks_the_right_index() {
        let sorted = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];
        assert_eq!(percentile_duration(&sorted, 0.0), Duration::from_millis(10));
        assert_eq!(percentile_duration(&sorted, 1.0), Duration::from_millis(50));
        assert_eq!(percentile_duration(&sorted, 0.5), Duration::from_millis(30));
    }

    #[test]
    fn mean_duration_of_empty_slice_is_zero() {
        assert_eq!(mean_duration(&[]), Duration::ZERO);
    }

    /// The crux of 1.2: ground truth is present for one document and absent
    /// for another, and `run_prepped` must score only the labelled one — an
    /// unlabelled document contributes zero field comparisons, not a wrong
    /// answer on every field.
    #[tokio::test]
    async fn unlabelled_documents_are_excluded_from_accuracy_not_scored_as_misses() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let mut labelled_truth = HashMap::new();
        labelled_truth.insert(CoreField::Surname, "DOE".to_string());
        let prepped = vec![
            Some(BenchPage {
                name: "fixture".to_string(),
                page: OcrPage {
                    text: "surname DOE".to_string(),
                    ..OcrPage::default()
                },
                ground_truth: Some(labelled_truth),
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
                mrz_found: false,
                synthetic: false,
                known_or_guessed_format: None,
            }),
            Some(BenchPage {
                name: "fixture".to_string(),
                page: OcrPage {
                    text: "surname DOE, no label file for this one".to_string(),
                    ..OcrPage::default()
                },
                ground_truth: None,
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test-2.png"),
                mrz_found: false,
                synthetic: false,
                known_or_guessed_format: None,
            }),
        ];

        let reports = run_prepped(&catalog, &prepped, false, false).await;
        let report = &reports[0];
        assert_eq!(report.accuracy.labelled_documents, 1);
        assert_eq!(report.accuracy.field_match_rate, Some(1.0));
    }

    /// An entirely unlabelled corpus must report `None`, not a fabricated
    /// `0.0` that would read as "0% accuracy" instead of "nothing measured."
    #[tokio::test]
    async fn no_labelled_documents_reports_accuracy_as_none_not_zero() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            name: "fixture".to_string(),
            page: OcrPage {
                text: "surname DOE".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            synthetic: false,
            known_or_guessed_format: None,
        })];

        let reports = run_prepped(&catalog, &prepped, false, false).await;
        let report = &reports[0];
        assert_eq!(report.accuracy.labelled_documents, 0);
        assert_eq!(report.accuracy.field_match_rate, None);
        assert_eq!(report.accuracy.mean_cer, None);
    }

    /// The crux of 1.3: a `!vision` provider gets a computed rate, even
    /// though the fixture has no ground truth at all — unsupported-assertion
    /// needs no labels, only OCR text, which is exactly what 1.2 must not
    /// gate it behind.
    #[tokio::test]
    async fn non_vision_provider_gets_a_computed_unsupported_assertion_rate() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "SMITH", // absent from the OCR text below
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            name: "fixture".to_string(),
            page: OcrPage {
                text: "surname DOE, birth date 1990".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None, // deliberately unlabelled — must not block this metric
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            synthetic: false,
            known_or_guessed_format: None,
        })];

        let reports = run_prepped(&catalog, &prepped, false, false).await;
        match &reports[0].unsupported_assertion {
            UnsupportedAssertion::Computed {
                overall,
                with_mrz_anchor,
                without_mrz_anchor,
            } => {
                assert_eq!(overall.assertions_total, 1);
                assert_eq!(
                    overall.rate,
                    Some(1.0),
                    "SMITH never appears in the OCR text"
                );
                // The fixture's OCR text carries no MRZ, so the whole
                // measurement belongs to the unanchored half — the population
                // Phase 2's grounding gate is aimed at.
                assert!(
                    with_mrz_anchor.is_none(),
                    "no fixture document had a parseable MRZ"
                );
                let unanchored = without_mrz_anchor
                    .as_ref()
                    .expect("the one fixture document had no MRZ");
                assert_eq!(unanchored.documents, 1);
                assert_eq!(unanchored.rate, Some(1.0));
            }
            UnsupportedAssertion::NotApplicable { .. } => {
                panic!("a deterministic (!vision) provider must get a computed rate")
            }
        }
    }

    /// The other half of 1.3: a `vision` provider gets `NotApplicable`
    /// rather than a number, even when its answer is absent from the OCR
    /// text — that absence is exactly what a working vision provider looks
    /// like, not evidence of hallucination.
    #[tokio::test]
    async fn vision_provider_gets_not_applicable_instead_of_a_rate() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader().with_vision(true),
            surname: "SMITH", // absent from the OCR text — must not be penalized
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            name: "fixture".to_string(),
            page: OcrPage {
                text: "surname DOE, no SMITH anywhere in this text".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            synthetic: false,
            known_or_guessed_format: None,
        })];

        let reports = run_prepped(&catalog, &prepped, false, false).await;
        match &reports[0].unsupported_assertion {
            UnsupportedAssertion::NotApplicable { reason } => {
                assert!(!reason.is_empty());
            }
            UnsupportedAssertion::Computed { .. } => {
                panic!("a vision-capable provider must not get a verbatim-substring rate")
            }
        }
    }
}
