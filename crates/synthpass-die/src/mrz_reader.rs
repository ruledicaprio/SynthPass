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
/// Zero configuration and zero state, with one opt-in exception —
/// `surface_partial_reads` — which is itself a pure function of the text
/// given, so `deterministic: true` still holds.
#[derive(Debug, Clone)]
pub struct MrzReader {
    capability: Capability,
    surface_partial_reads: bool,
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
            surface_partial_reads: false,
        }
    }

    /// **Gated experiment (Chunk 7 of `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`),
    /// off by default.** When the *only* failing check digit is the
    /// composite (`mrz::Checks::only_composite_failed`), report the four
    /// individually-proven fields at
    /// `synthpass_core::v2::FieldConfidence::mrz_checksum_scope_partial`
    /// instead of discarding the whole record. Every other case — no MRZ,
    /// or any individual field's own digit failing — is completely
    /// unaffected: this only changes behavior for the one narrow,
    /// explicitly-checked scenario.
    ///
    /// Not wired into `synthpass-pipeline`'s production catalog
    /// (`MrzReader::new()` there stays the plain, flag-off constructor) —
    /// this exists so the same-binary A/B the plan doc's gate requires can
    /// be run before any default changes, not to ship a new default.
    pub fn with_partial_read_surfacing() -> Self {
        Self {
            surface_partial_reads: true,
            ..Self::new()
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

        let extraction = if data.valid() {
            extraction_v2_from_mrz(&data)
        } else if self.surface_partial_reads && data.checks.only_composite_failed() {
            // Gated experiment (Chunk 7, off unless constructed via
            // `with_partial_read_surfacing`): the composite is the only
            // failure, so the four individually check-digited fields are
            // still worth reporting -- at `CHECKSUM_PARTIAL`, not full
            // proof; see `extraction_v2_from_mrz_partial`'s doc comment.
            // `evidence.mrz_checksums_valid` stays `false`, correctly: the
            // record as a whole still did not verify.
            extraction_v2_from_mrz_partial(&data)
        } else {
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
        };
        evidence.observe_line1(
            extraction
                .line1_integrity
                .as_ref()
                .expect("extraction_v2_from_mrz/_partial always sets line1_integrity"),
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
        // Was hardcoded `Some(true)`: harmless while this function's only
        // caller only ever ran on an already-valid `m` (this v1 shape's
        // `mrz_checksums_valid` is consumed transiently inside
        // `ExtractionV2::from`, immediately overwritten by
        // `extraction_v2_from_mrz{,_partial}`'s own `v2.mrz = Some(block)`,
        // so it never actually leaked a wrong value) -- but honest now that
        // `extraction_v2_from_mrz_partial` also calls this on a
        // composite-only-failed `m`.
        mrz_checksums_valid: Some(m.valid()),
        validity: Some(Validity {
            dates_well_formed: v.dates_well_formed,
            in_date: v.in_date,
            dob_before_expiry: v.dob_before_expiry,
            days_until_expiry: v.days_until_expiry,
        }),
        extraction_method: EXTRACTION_METHOD.to_string(),
    }
}

/// The [`MrzBlock`] a parsed `mrz::MrzData` corresponds to: the raw zone plus
/// the exact per-digit checksum results, carried verbatim rather than collapsed
/// to a single bool.
///
/// Public for the same reason as [`mrz_format_of`], and for a sharper one: the
/// Tier-2 escalation path in `synthpass-pipeline` must produce a *byte-identical*
/// block to the Tier-1 path, because the same document can reach a caller
/// through either tier. Two hand-written copies of this struct literal already
/// drifted once; there is now one.
pub fn mrz_block_from(m: &mrz::MrzData) -> MrzBlock {
    MrzBlock {
        lines: m.mrz_lines.clone(),
        format: mrz_format_of(m),
        checks: CheckDigits {
            document_number: m.checks.document_number,
            date_of_birth: m.checks.date_of_birth,
            date_of_expiry: m.checks.date_of_expiry,
            personal_number: m.checks.personal_number,
            composite: m.checks.composite,
        },
    }
}

/// Map validated MRZ data onto the v2 schema: the check-digited fields are
/// proven, the rest are structural parses, and the raw zone plus exact
/// per-digit results ride along in [`MrzBlock`].
///
/// Moved verbatim from `synthpass-pipeline`'s private
/// `extraction_v2_from_mrz`. Byte-for-byte the same output — that equivalence
/// is what makes wiring the provider into the pipeline a refactor rather than
/// a behaviour change. Only ever called on a fully-valid `m`
/// (`m.valid()` is `true`) — see [`extraction_v2_from_mrz_partial`] for the
/// composite-only-failure sibling.
pub fn extraction_v2_from_mrz(m: &mrz::MrzData) -> ExtractionV2 {
    extraction_v2_scoped(m, synthpass_core::v2::FieldConfidence::mrz_checksum_scope())
}

/// **Gated experiment (Chunk 7), off unless `MrzReader` was built via
/// [`MrzReader::with_partial_read_surfacing`].** [`extraction_v2_from_mrz`]'s
/// sibling for a record whose composite check digit failed while every
/// individually check-digited field still verified
/// (`m.checks.only_composite_failed()` — callers must check this themselves,
/// this function does not re-verify it). Identical field extraction and
/// `line1_integrity` downgrade; the only difference is the confidence scope
/// (`mrz_checksum_scope_partial` instead of `mrz_checksum_scope`) — the four
/// individually-proven fields get `synthpass_core::v2`'s private
/// `CHECKSUM_PARTIAL` band, not full proof, because the record's own
/// composite digit did not verify it.
fn extraction_v2_from_mrz_partial(m: &mrz::MrzData) -> ExtractionV2 {
    extraction_v2_scoped(
        m,
        synthpass_core::v2::FieldConfidence::mrz_checksum_scope_partial(),
    )
}

/// Shared body of [`extraction_v2_from_mrz`]/[`extraction_v2_from_mrz_partial`]
/// — everything both need is identical except which confidence scope applies.
fn extraction_v2_scoped(
    m: &mrz::MrzData,
    confidence: synthpass_core::v2::FieldConfidence,
) -> ExtractionV2 {
    let block = mrz_block_from(m);
    let mut v2 = ExtractionV2::from(&extraction_from_mrz(m));
    v2.confidence = confidence;
    v2.provenance = Provenance::MrzChecksum;
    v2.document.mrz_format = Some(block.format);
    v2.mrz = Some(block);
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

    /// Same Utopia/Eriksson identity reshaped into TD1's 3-line layout (Part
    /// 5's own examples are images in the source PDF, not extractable text;
    /// this zone is corroborated by cross-part agreement with the TD2
    /// specimen below — same document number, dates, and name). Mirrors
    /// `crates/mrz/src/lib.rs`'s `TD1_L1`/`TD1_L2`/`TD1_L3`.
    const TD1_SPECIMEN: &str = "I<UTOD231458907<<<<<<<<<<<<<<<\n\
                                7408122F1204159UTO<<<<<<<<<<<6\n\
                                ERIKSSON<<ANNA<MARIA<<<<<<<<<<";

    /// The official ICAO 9303 Part 6 TD2 specimen, published verbatim as
    /// text (`knowledge/docs9303/Doc_9303_Part6_Specs_for_TD2_MROTDs.md:487-488`).
    /// Mirrors `crates/mrz/src/lib.rs`'s `TD2_L1`/`TD2_L2`.
    const TD2_SPECIMEN: &str = "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n\
                                D231458907UTO7408122F1204159<<<<<<<6";

    /// ICAO 9303 Part 7's own published MRV-A specimen
    /// (`knowledge/docs9303/Doc_9303_Part7_Machine_Readable_Visas_MRVs.md`).
    /// Mirrors `crates/mrz/src/lib.rs`'s `MRV_A_ICAO_L1`/`MRV_A_ICAO_L2`.
    const MRV_A_SPECIMEN: &str = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
                                  L898902C<3UTO6908061F9406236ZE184226B<<<<<<<";

    /// ICAO 9303 Part 7's own published MRV-B specimen, same appendix as
    /// above. Mirrors `crates/mrz/src/lib.rs`'s `MRV_B_ICAO_L1`/`MRV_B_ICAO_L2`.
    const MRV_B_SPECIMEN: &str = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n\
                                  L898902C<3UTO6908061F9406236ZE184226";

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

    /// Corrupts only the composite check digit — line 2's last character —
    /// leaving all four individually check-digited fields untouched. The
    /// narrowest possible input for Chunk 7's gate: `only_composite_failed()`
    /// must be `true`.
    fn composite_only_corrupted() -> String {
        let mut corrupted = SPECIMEN.to_string();
        assert!(
            corrupted.ends_with('0'),
            "must actually change the composite digit"
        );
        corrupted.pop();
        corrupted.push('A');
        corrupted
    }

    /// The default constructor must behave exactly as before even for the
    /// narrow composite-only-failure case the gated flag targets — this is
    /// the same-binary half of the A/B: one build, flag off, no behavior
    /// change.
    #[test]
    fn composite_only_failure_is_fully_discarded_when_the_flag_is_off() {
        let reading = read(&composite_only_corrupted());
        assert!(reading.evidence.mrz_found);
        assert!(!reading.evidence.mrz_checksums_valid);
        assert!(reading.evidence.checksum_partially_valid());
        assert!(
            reading
                .extraction
                .fields
                .get(CoreField::DocumentNumber)
                .is_none(),
            "the flag is off by default -- a composite failure still discards everything"
        );
        assert_eq!(reading.missing().len(), CoreField::ALL.len());
    }

    /// The flag on, the narrow case: the four individually-proven fields
    /// surface at the new partial-proof band (strictly between
    /// `MRZ_STRUCTURAL` and full `PROVEN`), `mrz_checksums_valid` stays
    /// honestly `false`, and only the fields with no check digit at all that
    /// also came back empty remain in `missing`.
    #[test]
    fn composite_only_failure_surfaces_proven_fields_when_the_flag_is_on() {
        let reader = MrzReader::with_partial_read_surfacing();
        let reading =
            block_on(reader.read(&DocumentContext::from_text(&composite_only_corrupted())))
                .expect("never errs");

        assert!(reading.evidence.mrz_found);
        assert!(
            !reading.evidence.mrz_checksums_valid,
            "the record as a whole still did not verify"
        );

        let fields = &reading.extraction.fields;
        assert_eq!(fields.get(CoreField::DocumentNumber), Some("L898902C3"));
        assert_eq!(fields.get(CoreField::Surname), Some("ERIKSSON"));

        let c = reading.extraction.confidence;
        assert!(c.document_number > 0.9 && c.document_number < 1.0);
        assert_eq!(c.date_of_birth, c.document_number);
        assert_eq!(c.date_of_expiry, c.document_number);
        assert_eq!(c.personal_number, c.document_number);
        // Unchanged from the fully-valid case -- composite failing says
        // nothing new about a field it doesn't localize to.
        assert_eq!(c.surname, 0.9);

        assert!(
            reading.missing().is_empty(),
            "every field parsed structurally in this specimen"
        );
    }

    /// The flag on, but a *different* check digit also failed (not just
    /// composite): must still fully discard. The flag only ever changes
    /// behavior for the one narrow, explicitly-checked scenario.
    #[test]
    fn a_non_composite_failure_is_unaffected_by_the_flag() {
        let corrupted = SPECIMEN.replace("L898902C36UTO", "L898902C35UTO");
        let reader = MrzReader::with_partial_read_surfacing();
        let reading =
            block_on(reader.read(&DocumentContext::from_text(&corrupted))).expect("never errs");

        assert!(!reading.evidence.mrz_checksums_valid);
        assert!(!reading.evidence.checksum_partially_valid());
        assert!(reading.extraction.fields.get(CoreField::Surname).is_none());
        assert_eq!(reading.missing().len(), CoreField::ALL.len());
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

    /// `mrz::find_and_parse` is not TD3-scoped — this proves the *provider*
    /// (not just the underlying parser) correctly reads, tags, and
    /// checksum-verifies TD1, whose 3-line shape is the one most likely to
    /// trip up code that assumes 2 lines.
    #[test]
    fn reads_the_icao_td1_specimen_and_reports_valid_checksums() {
        let reading = read(TD1_SPECIMEN);
        assert!(reading.evidence.mrz_found);
        assert!(reading.evidence.mrz_checksums_valid);
        assert_eq!(reading.evidence.mrz_format, Some(MrzFormat::Td1));

        let fields = &reading.extraction.fields;
        assert_eq!(fields.get(CoreField::Surname), Some("ERIKSSON"));
        assert_eq!(fields.get(CoreField::DocumentNumber), Some("D23145890"));
        assert_eq!(fields.get(CoreField::IssuingCountry), Some("UTO"));
        assert_eq!(reading.extraction.provenance, Provenance::MrzChecksum);
        assert_eq!(reading.extraction.extraction_method, EXTRACTION_METHOD);
    }

    /// TD1's permanent, structural limitation: no check digit covers line 3
    /// (surname/given_names) or either optional-data field at all — unlike
    /// TD3, this is not something a future fix changes. A checksum-valid TD1
    /// record proves the document number/DOB/expiry/composite, never the
    /// name. See `knowledge/ROADMAP.md`'s M6 execution notes.
    #[test]
    fn td1_confidence_never_claims_the_name_is_proven() {
        let c = read(TD1_SPECIMEN).extraction.confidence;
        assert_eq!(c.document_number, 1.0);
        assert_eq!(c.date_of_birth, 1.0);
        assert_eq!(c.date_of_expiry, 1.0);
        assert!(
            c.surname < 1.0,
            "TD1 line 3 carries no check digit — the name is never proven"
        );
    }

    /// Same proof as the TD1 test above, for TD2's 2-line/36-char shape.
    #[test]
    fn reads_the_icao_td2_specimen_and_reports_valid_checksums() {
        let reading = read(TD2_SPECIMEN);
        assert!(reading.evidence.mrz_found);
        assert!(reading.evidence.mrz_checksums_valid);
        assert_eq!(reading.evidence.mrz_format, Some(MrzFormat::Td2));

        let fields = &reading.extraction.fields;
        assert_eq!(fields.get(CoreField::Surname), Some("ERIKSSON"));
        assert_eq!(fields.get(CoreField::DocumentNumber), Some("D23145890"));
        assert_eq!(fields.get(CoreField::IssuingCountry), Some("UTO"));
        assert_eq!(reading.extraction.provenance, Provenance::MrzChecksum);
        assert_eq!(reading.extraction.extraction_method, EXTRACTION_METHOD);
    }

    /// Same proof for MRV-A (visas): the provider correctly reads a `V`
    /// document-code, 44-char, 2-line format distinct from TD3 despite the
    /// identical line width.
    #[test]
    fn reads_the_icao_mrv_a_specimen_and_reports_valid_checksums() {
        let reading = read(MRV_A_SPECIMEN);
        assert!(reading.evidence.mrz_found);
        assert!(reading.evidence.mrz_checksums_valid);
        assert_eq!(reading.evidence.mrz_format, Some(MrzFormat::MrvA));

        let fields = &reading.extraction.fields;
        assert_eq!(fields.get(CoreField::Surname), Some("ERIKSSON"));
        assert_eq!(fields.get(CoreField::IssuingCountry), Some("UTO"));
        assert_eq!(reading.extraction.provenance, Provenance::MrzChecksum);
        assert_eq!(reading.extraction.extraction_method, EXTRACTION_METHOD);
    }

    /// Same proof for MRV-B: 36-char, 2-line, distinct from TD2 despite the
    /// identical line width.
    #[test]
    fn reads_the_icao_mrv_b_specimen_and_reports_valid_checksums() {
        let reading = read(MRV_B_SPECIMEN);
        assert!(reading.evidence.mrz_found);
        assert!(reading.evidence.mrz_checksums_valid);
        assert_eq!(reading.evidence.mrz_format, Some(MrzFormat::MrvB));

        let fields = &reading.extraction.fields;
        assert_eq!(fields.get(CoreField::Surname), Some("ERIKSSON"));
        assert_eq!(fields.get(CoreField::IssuingCountry), Some("UTO"));
        assert_eq!(reading.extraction.provenance, Provenance::MrzChecksum);
        assert_eq!(reading.extraction.extraction_method, EXTRACTION_METHOD);
    }
}
