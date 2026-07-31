//! M7 multi-provider benchmark: runs every registered [`FieldReader`] in a
//! [`ProviderCatalog`] against the same synthetic corpus and reports, per
//! provider, accuracy, speed, JSON validity, an unsupported-assertion rate,
//! and resident memory.
//!
//! Deliberately bypasses `ProviderCatalog::find_reader`'s first-match router
//! — every reader sees every document, not just the one the pipeline's
//! routing policy would have picked, because the whole point is comparing
//! providers against each other, not reproducing the router's decision.

use crate::CorpusDoc;
use std::time::{Duration, Instant};
use synthpass_core::v2::CoreField;
use synthpass_die::{Capability, CostClass, DocumentContext, ProviderCatalog};
use synthpass_ocr::{NativeOcr, OcrPage};

/// Everything measured for one provider over one corpus run.
pub struct ProviderReport {
    pub provider_id: String,
    pub documents: usize,
    pub capability: CapabilitySnapshot,
    pub accuracy: AccuracyStats,
    pub speed: SpeedStats,
    /// `None` for deterministic providers — there is no JSON step to measure.
    pub json_validity: Option<JsonValidityStats>,
    /// Fraction of non-null answered fields whose value does not appear
    /// (case-insensitive substring) anywhere in the OCR text the provider was
    /// given — "a value the provider asserted that appears nowhere in its
    /// own input" (`knowledge/ROADMAP.md`'s M7 definition, deliberately not
    /// "hallucination", which is not measurable this way).
    pub unsupported_assertion_rate: f64,
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
}

pub struct CapabilitySnapshot {
    pub deterministic: bool,
    pub cost: &'static str,
}

impl CapabilitySnapshot {
    fn from(capability: &Capability) -> Self {
        Self {
            deterministic: capability.deterministic,
            cost: match capability.cost {
                CostClass::Free => "free",
                CostClass::Cheap => "cheap",
                CostClass::Expensive => "expensive",
            },
        }
    }
}

pub struct AccuracyStats {
    /// Fraction of `(document, field)` pairs where the provider's answer
    /// exactly matches ground truth, over fields ground truth actually has a
    /// value for.
    pub field_match_rate: f64,
    /// Mean character error rate over the same population, via
    /// [`crate::cer`] — reuses the exact metric `synthpass-bench`'s Tier-1
    /// report already uses, so the two numbers are comparable.
    pub mean_cer: f64,
    pub per_field_cer: Vec<(&'static str, f64)>,
}

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
/// [`mrz::MrzData`] — the corpus's ground truth. A local table rather than
/// reusing `synthpass-bench`'s private `COMPARED_FIELDS`: that one is keyed
/// by `&str` for the Tier-1-only report; this one is keyed by `CoreField` so
/// it can index `ExtractionFields::get` directly.
fn ground_truth(field: CoreField, truth: &mrz::MrzData) -> String {
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

/// OCRs `doc.image` (via a temp file, the same pattern
/// [`crate::check_document`] uses — neither `NativeOcr` nor the pipeline's
/// `OcrEngine` expose an in-memory entry point) and returns the detailed
/// page: text plus the geometry signals (`mrz_band_score`, `text_sanity`,
/// `rotation`) a [`DocumentContext`] carries.
fn ocr_detailed(ocr: &NativeOcr, doc: &CorpusDoc) -> Result<OcrPage, String> {
    let path = std::env::temp_dir().join(format!(
        "provider-bench-{}-{}.png",
        std::process::id(),
        doc.seed
    ));
    doc.image
        .save(&path)
        .map_err(|e| format!("failed to write temp image: {e}"))?;
    let result = ocr.recognize_detailed(&path);
    let _ = std::fs::remove_file(&path);
    result
}

/// Runs every reader in `catalog` against every document in `corpus`,
/// OCRing each document once and sharing that OCR pass across all readers —
/// the comparison is only fair if every provider sees the identical input.
pub async fn run_provider_bench(
    catalog: &ProviderCatalog,
    ocr: &NativeOcr,
    corpus: &[CorpusDoc],
    measure_memory: bool,
) -> Vec<ProviderReport> {
    // OCR once per document, shared across every reader below — the
    // comparison is only fair if every provider sees identical input.
    let pages: Vec<Option<(OcrPage, mrz::MrzData)>> = corpus
        .iter()
        .map(|doc| {
            let page = ocr_detailed(ocr, doc).ok()?;
            let truth = mrz::parse_td3(&doc.labels.mrz_line1, &doc.labels.mrz_line2).ok()?;
            Some((page, truth))
        })
        .collect();
    let ocr_documents = pages.iter().filter(|p| p.is_some()).count();

    let mut reports = Vec::with_capacity(catalog.readers().len());
    for reader in catalog.readers() {
        let capability = reader.capability();
        let rss_before = measure_memory.then(sample_rss).flatten();
        let repair_before = synthpass_llm::repair::repair_fallbacks();

        let mut elapsed_per_doc = Vec::with_capacity(pages.len());
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

        for page_and_truth in &pages {
            let Some((page, truth)) = page_and_truth else {
                continue;
            };
            let ctx = DocumentContext::from_text(&page.text);
            let started = Instant::now();
            let reading = reader.read(&ctx).await;
            elapsed_per_doc.push(started.elapsed());

            let Ok(reading) = reading else { continue };

            for (i, field) in CoreField::ALL.iter().enumerate() {
                let expected = ground_truth(*field, truth);
                if expected.is_empty() {
                    continue;
                }
                let got = reading.extraction.fields.get(*field);
                field_total += 1;
                let got_str = got.unwrap_or("");
                if got_str == expected {
                    field_hits += 1;
                }
                let field_cer = crate::cer(&expected, got_str);
                cer_sum += field_cer;
                cer_count += 1;
                per_field[i].1 += field_cer;
                per_field[i].2 += 1;

                if let Some(value) = got {
                    if !value.is_empty() {
                        assertions_total += 1;
                        if !page.text.to_lowercase().contains(&value.to_lowercase()) {
                            assertions_unsupported += 1;
                        }
                    }
                }
            }
        }

        let repair_after = synthpass_llm::repair::repair_fallbacks();
        let rss_after = measure_memory.then(sample_rss).flatten();

        elapsed_per_doc.sort();
        let speed = SpeedStats {
            mean: mean_duration(&elapsed_per_doc),
            p50: percentile_duration(&elapsed_per_doc, 0.50),
            p95: percentile_duration(&elapsed_per_doc, 0.95),
        };

        reports.push(ProviderReport {
            provider_id: reader.id().as_str().to_string(),
            documents: ocr_documents,
            capability: CapabilitySnapshot::from(capability),
            accuracy: AccuracyStats {
                field_match_rate: if field_total == 0 {
                    0.0
                } else {
                    field_hits as f64 / field_total as f64
                },
                mean_cer: if cer_count == 0 {
                    0.0
                } else {
                    cer_sum / cer_count as f64
                },
                per_field_cer: per_field
                    .into_iter()
                    .map(|(name, sum, n)| (name, if n == 0 { 0.0 } else { sum / n as f64 }))
                    .collect(),
            },
            speed,
            json_validity: (!capability.deterministic).then(|| JsonValidityStats {
                repair_fallbacks: repair_after.saturating_sub(repair_before),
                documents: ocr_documents,
            }),
            unsupported_assertion_rate: if assertions_total == 0 {
                0.0
            } else {
                assertions_unsupported as f64 / assertions_total as f64
            },
            declared_resident_bytes: capability.estimated_resident_bytes,
            measured_rss_delta_bytes: match (rss_before, rss_after) {
                (Some(before), Some(after)) => Some(after as i64 - before as i64),
                _ => None,
            },
        });
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
    use synthpass_die::{
        Evidence, FieldReader, IntelligenceProvider, ProviderError, ProviderId, Reading,
    };

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
}
