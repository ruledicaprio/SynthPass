//! The deterministic MRZ provider: ICAO 9303 parsing and check-digit
//! verification, as a [`FieldReader`].
//!
//! This is principle 1 — *deterministic whenever possible; AI only fills
//! uncertainty* — expressed as a provider. It declares
//! [`Capability::deterministic`] and [`CostClass::Free`](crate::CostClass::Free),
//! so the router always
//! consults it first, and anything more expensive runs only on what it could
//! not establish.
//!
//! Until now this logic lived inline in `synthpass-pipeline` as
//! `mrz::find_and_parse(&text).ok().filter(|m| m.valid())` plus a pair of
//! private helpers. Nothing about the arithmetic changes here; it gains an
//! identity, a declared capability, and somewhere to report evidence.

use std::time::{SystemTime, UNIX_EPOCH};

use synthpass_core::v2::{CheckDigits, ExtractionV2, MrzBlock, MrzFormat, Provenance, ProviderId};
use synthpass_core::{Extraction, Validity};

use crate::evidence::Evidence;
use crate::provider::{
    Capability, DocumentContext, FieldReader, IntelligenceProvider, ProviderError, Reading,
};

/// The `extraction_method` string this provider stamps, matching v1's
/// vocabulary exactly so the wire value is unchanged.
const EXTRACTION_METHOD: &str = "mrz-deterministic";

/// [`ProviderId`] of the deterministic MRZ reader.
pub const MRZ_PROVIDER_ID: ProviderId = ProviderId("mrz");

/// Deterministic ICAO 9303 MRZ reader.
///
/// Zero configuration and zero state: the whole provider is a pure function of
/// the text it is given, which is what `deterministic: true` promises.
#[derive(Debug, Clone)]
pub struct MrzReader {
    capability: Capability,
}

impl Default for MrzReader {
    fn default() -> Self {
        Self::new()
    }
}

impl MrzReader {
    pub fn new() -> Self {
        Self {
            // No `with_weights_license`: there is no model here to license.
            // That absence is the point of this provider.
            capability: Capability::deterministic_reader(),
        }
    }
}

impl IntelligenceProvider for MrzReader {
    fn id(&self) -> ProviderId {
        MRZ_PROVIDER_ID
    }

    fn capability(&self) -> &Capability {
        &self.capability
    }

    fn describe(&self) -> String {
        "deterministic ICAO 9303 MRZ (check-digit verified)".into()
    }
}

#[async_trait::async_trait]
impl FieldReader for MrzReader {
    /// Parse and verify. Never fails: a document with no MRZ is a legitimate
    /// answer ("I found nothing"), not an error, and reporting it as `Err`
    /// would make the router unable to distinguish "this provider broke" from
    /// "this provider looked and there was nothing there".
    ///
    /// Evidence is recorded in **all three** cases — no MRZ, invalid MRZ, valid
    /// MRZ — because "an MRZ parsed but two check digits failed" is exactly the
    /// signal an escalation decision wants, and the previous inline version
    /// discarded it with `.filter(|m| m.valid())`.
    async fn read(&self, ctx: &DocumentContext<'_>) -> Result<Reading, ProviderError> {
        let recognition = ctx.recognition;
        let mut evidence = Evidence {
            text_chars: ctx.text.chars().count(),
            text_lines: ctx.text.lines().count(),
            mrz_band_score: recognition.and_then(|r| r.mrz_band_score),
            text_sanity: recognition.and_then(|r| r.text_sanity),
            portrait_found: recognition.is_some_and(|r| r.portrait.is_some()),
            rotation_applied: recognition.map_or(0, |r| r.rotation),
            ..Evidence::default()
        };

        let Some(data) = mrz::find_and_parse(ctx.text).ok() else {
            evidence.missing = synthpass_core::v2::CoreField::ALL.to_vec();
            return Ok(Reading {
                extraction: ExtractionV2::default(),
                evidence,
                by: self.id(),
            });
        };

        evidence.observe_mrz(&data);
        evidence.mrz_format = Some(mrz_format_of(&data));
        evidence.blind_positions = Some(blind_positions(&data.mrz_lines));

        if !data.valid() {
            // A record whose check digits do not verify is not a Tier-1 result.
            // Its *evidence* is still valuable — which digits failed is the
            // most actionable escalation reason there is — but the fields are
            // not reported, exactly as before.
            evidence.missing = synthpass_core::v2::CoreField::ALL.to_vec();
            return Ok(Reading {
                extraction: ExtractionV2::default(),
                evidence,
                by: self.id(),
            });
        }

        let extraction = extraction_v2_from_mrz(&data);
        evidence.observe_line1(
            extraction
                .line1_integrity
                .as_ref()
                .expect("extraction_v2_from_mrz always sets line1_integrity"),
        );
        evidence.missing = extraction.fields.missing();

        Ok(Reading {
            extraction,
            evidence,
            by: self.id(),
        })
    }
}

/// How many characters of the zone sit in a check-digit **blind spot**: a
/// substitution the ICAO arithmetic provably cannot detect, because the two
/// characters are congruent mod 10.
///
/// Counted over the whole zone rather than the check-digited substrings,
/// deliberately: it is a property of the *read*, and this value is recorded
/// observed-only until a sweep says what a given count predicts. Interpreting
/// it before measuring it would be inventing a threshold.
fn blind_positions(lines: &str) -> u32 {
    lines
        .chars()
        .filter(|c| !c.is_whitespace())
        .filter(|c| !mrz::collisions(*c).is_empty())
        .count() as u32
}

/// `mrz::Format` is `#[non_exhaustive]`, so a future ICAO format can arrive
/// without a matching [`MrzFormat`]. Fall back to the line-shape heuristic
/// rather than asserting a wrong variant: the geometry is recoverable from the
/// zone even when the name for it is not.
///
/// Public so `synthpass-pipeline` can reuse this mapping when it has its own
/// checksum-tested `mrz::MrzData` in hand (the Tier-2 escalation path) rather
/// than re-guessing the format from raw text or duplicating the match arm.
pub fn mrz_format_of(m: &mrz::MrzData) -> MrzFormat {
    match m.format {
        mrz::Format::Td1 => MrzFormat::Td1,
        mrz::Format::Td2 => MrzFormat::Td2,
        mrz::Format::Td3 => MrzFormat::Td3,
        mrz::Format::MrvA => MrzFormat::MrvA,
        mrz::Format::MrvB => MrzFormat::MrvB,
        _ => MrzFormat::guess_from_lines(&m.mrz_lines).unwrap_or(MrzFormat::Td3),
    }
}

/// Today, for the date-plausibility summary. Checksum-valid does not imply
/// in-date.
fn today() -> mrz::Date {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    mrz::Date::from_epoch_days((secs / 86_400) as i64)
}

/// Map validated MRZ data onto the canonical v1 [`Extraction`] shape — the
/// same shape Tier 2 and the WASM demo produce. Enriches with resolved country
/// names and a date-plausibility summary.
fn extraction_from_mrz(m: &mrz::MrzData) -> Extraction {
    let v = m.validity(today());
    Extraction {
        document_type: Some(m.document_type.clone()),
        issuing_country: Some(m.issuing_country.clone()),
        issuing_country_name: m.issuing_country_name().map(str::to_string),
        document_number: Some(m.document_number.clone()),
        surname: Some(m.surname.clone()),
        given_names: Some(m.given_names.clone()),
        nationality: Some(m.nationality.clone()),
        nationality_name: m.nationality_name().map(str::to_string),
        date_of_birth: Some(m.date_of_birth.clone()),
        sex: Some(m.sex.clone()),
        date_of_expiry: Some(m.date_of_expiry.clone()),
        personal_number: m.personal_number.clone(),
        mrz_line: Some(m.mrz_lines.clone()),
        mrz_checksums_valid: Some(true),
        validity: Some(Validity {
            dates_well_formed: v.dates_well_formed,
            in_date: v.in_date,
            dob_before_expiry: v.dob_before_expiry,
            days_until_expiry: v.days_until_expiry,
        }),
        extraction_method: EXTRACTION_METHOD.to_string(),
    }
}

/// Map validated MRZ data onto the v2 schema: the check-digited fields are
/// proven, the rest are structural parses, and the raw zone plus exact
/// per-digit results ride along in [`MrzBlock`].
///
/// Moved verbatim from `synthpass-pipeline`'s private
/// `extraction_v2_from_mrz`. Byte-for-byte the same output — that equivalence
/// is what makes wiring the provider into the pipeline a refactor rather than
/// a behaviour change.
pub fn extraction_v2_from_mrz(m: &mrz::MrzData) -> ExtractionV2 {
    let format = mrz_format_of(m);
    let mut v2 = ExtractionV2::from(&extraction_from_mrz(m));
    v2.provenance = Provenance::MrzChecksum;
    v2.document.mrz_format = Some(format);
    v2.mrz = Some(MrzBlock {
        lines: m.mrz_lines.clone(),
        format,
        checks: CheckDigits {
            document_number: m.checks.document_number,
            date_of_birth: m.checks.date_of_birth,
            date_of_expiry: m.checks.date_of_expiry,
            personal_number: m.checks.personal_number,
            composite: m.checks.composite,
        },
    });
    // A field a deterministic check contradicts must not keep the confidence it
    // had when nothing contradicted it.
    let verdict = synthpass_core::fusion::check_line1_integrity(m);
    v2.confidence.downgrade_flagged(&verdict);
    v2.line1_integrity = Some(verdict);
    v2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::CostClass;
    use synthpass_core::v2::CoreField;

    /// The ICAO 9303 Part 4 TD3 specimen.
    const SPECIMEN: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
                            L898902C36UTO7408122F1204159ZE184226B<<<<<10";

    /// Drives an `async fn` without a runtime — proof, not convenience: a
    /// provider must be usable by a caller that has no executor, which is why
    /// `synthpass-die` does not depend on tokio.
    fn block_on<F: std::future::Future>(mut fut: F) -> F::Output {
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        fn noop(_: *const ()) {}
        fn clone(_: *const ()) -> RawWaker {
            RawWaker::new(std::ptr::null(), &VTABLE)
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        let waker = unsafe { Waker::from_raw(clone(std::ptr::null())) };
        let mut cx = Context::from_waker(&waker);
        // Safety: `fut` is owned and never moved after this shadowing pin.
        let mut fut = unsafe { std::pin::Pin::new_unchecked(&mut fut) };
        loop {
            match fut.as_mut().poll(&mut cx) {
                Poll::Ready(v) => return v,
                Poll::Pending => continue,
            }
        }
    }

    fn read(text: &str) -> Reading {
        block_on(MrzReader::new().read(&DocumentContext::from_text(text)))
            .expect("the MRZ reader never returns Err")
    }

    #[test]
    fn declares_itself_free_and_deterministic() {
        let reader = MrzReader::new();
        let cap = reader.capability();
        assert!(cap.deterministic, "MRZ arithmetic is deterministic");
        assert_eq!(cap.cost, CostClass::Free);
        assert!(cap.fields);
        assert!(!cap.vision, "no provider ships vision in v1.3.0");
        assert!(cap.weights_license.is_none(), "there are no weights here");
        assert_eq!(reader.id(), MRZ_PROVIDER_ID);
    }

    #[test]
    fn reads_the_icao_specimen_and_reports_valid_checksums() {
        let reading = read(SPECIMEN);
        assert!(reading.evidence.mrz_found);
        assert!(reading.evidence.mrz_checksums_valid);
        assert!(reading.evidence.mrz_failed.is_empty());
        assert_eq!(reading.evidence.mrz_format, Some(MrzFormat::Td3));

        let fields = &reading.extraction.fields;
        assert_eq!(fields.get(CoreField::Surname), Some("ERIKSSON"));
        assert_eq!(fields.get(CoreField::DocumentNumber), Some("L898902C3"));
        assert_eq!(fields.get(CoreField::IssuingCountry), Some("UTO"));
        assert_eq!(reading.extraction.provenance, Provenance::MrzChecksum);
        assert_eq!(reading.extraction.extraction_method, EXTRACTION_METHOD);
        assert!(reading.missing().is_empty());
    }

    /// The check-digited fields are proven; the rest are structural. This is
    /// the honesty the whole confidence model rests on — line 1 carries no
    /// check digit at all, so a "proven" surname would be a lie.
    #[test]
    fn only_check_digited_fields_are_proven() {
        let c = read(SPECIMEN).extraction.confidence;
        assert_eq!(c.document_number, 1.0);
        assert_eq!(c.date_of_birth, 1.0);
        assert_eq!(c.date_of_expiry, 1.0);
        assert!(c.surname < 1.0, "line 1 carries no check digit");
        assert!(c.issuing_country < 1.0);
    }

    /// "I looked and there is nothing here" is an answer, not an error. An
    /// `Err` would make the router unable to tell a broken provider from an
    /// empty page.
    #[test]
    fn no_mrz_is_an_empty_reading_not_an_error() {
        let reading = read("just some ordinary prose with no machine readable zone");
        assert!(!reading.evidence.mrz_found);
        assert!(!reading.evidence.mrz_checksums_valid);
        assert_eq!(reading.missing().len(), CoreField::ALL.len());
        assert_eq!(reading.by, MRZ_PROVIDER_ID);
    }

    /// The signal the old inline `.filter(|m| m.valid())` threw away: an MRZ
    /// that parsed but did not verify is the single most actionable escalation
    /// reason available, and it used to vanish.
    #[test]
    fn a_parsed_but_invalid_mrz_still_reports_which_digits_failed() {
        // Same specimen with the document-number check digit corrupted.
        let corrupted = SPECIMEN.replace("L898902C36UTO", "L898902C35UTO");
        let reading = read(&corrupted);

        assert!(reading.evidence.mrz_found, "it still parses");
        assert!(!reading.evidence.mrz_checksums_valid);
        assert!(
            !reading.evidence.mrz_failed.is_empty(),
            "the failing digits must be reported, not discarded"
        );
        assert!(
            reading.extraction.fields.get(CoreField::Surname).is_none(),
            "an unverified record reports no fields, exactly as before"
        );
    }

    #[test]
    fn recognition_geometry_reaches_the_evidence() {
        use crate::provider::Recognition;
        let mut recognition = Recognition::from_text(SPECIMEN);
        recognition.mrz_band_score = Some(0.91);
        recognition.text_sanity = Some(0.87);
        recognition.rotation = 180;

        let ctx = DocumentContext::from_text(SPECIMEN).with_recognition(&recognition);
        let reading = block_on(MrzReader::new().read(&ctx)).expect("never errs");

        assert_eq!(reading.evidence.mrz_band_score, Some(0.91));
        assert_eq!(reading.evidence.text_sanity, Some(0.87));
        assert_eq!(reading.evidence.rotation_applied, 180);
    }

    /// Blind positions are counted and recorded, and deliberately not acted
    /// on. Recording a signal before anyone has measured what it predicts is
    /// how you earn a threshold; routing on it first is how you invent one.
    #[test]
    fn blind_positions_are_observed_only() {
        let reading = read(SPECIMEN);
        let observed = reading
            .evidence
            .blind_positions
            .expect("counted for any parsed zone");
        assert!(
            observed > 0,
            "the ICAO specimen contains characters with mod-10 collisions"
        );
    }
}
