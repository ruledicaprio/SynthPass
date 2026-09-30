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
//!
//! The read goes through [`read_tier1`], which applies the line-1 selector
//! (#574) by default; `SYNTHPASS_MRZ_LINE1_SELECT=off` restores the plain
//! `mrz::find_and_parse_with`. The pipeline's v1 record reads Tier 1 the same
//! way, so the v1 and v2 records agree under every arm.

use std::time::{SystemTime, UNIX_EPOCH};

use synthpass_core::v2::{
    CheckDigits, CoreField, ExtractionV2, MrzBlock, MrzFormat, Provenance, ProviderId,
};
use synthpass_core::{Extraction, Validity};

use crate::evidence::{Evidence, Line1Outcome, Line1Reason, Line1Summary};
use crate::provider::{
    Capability, DocumentContext, FieldReader, IntelligenceProvider, ProviderError, Reading,
};
use crate::Line1Arm;

/// The `extraction_method` string this provider stamps, matching v1's
/// vocabulary exactly so the wire value is unchanged.
const EXTRACTION_METHOD: &str = "mrz-deterministic";

/// [`ProviderId`] of the deterministic MRZ reader.
pub const MRZ_PROVIDER_ID: ProviderId = ProviderId("mrz");

/// Deterministic ICAO 9303 MRZ reader.
///
/// Zero configuration and zero state: the whole provider is a pure function of
/// the text it is given, which is what `deterministic: true` promises.
///
/// Reads all five ICAO 9303 MRZ formats — TD1, TD2, TD3, MRV-A and MRV-B — as
/// one provider. Which format a text holds is decided inside
/// [`mrz::find_and_parse`], in the fixed priority order ARCHITECTURE.md §13.3
/// records (TD3 → MRV-B → MRV-A → TD1 → TD2), and reported per read in
/// [`Evidence::mrz_format`]. It is deliberately not five providers: splitting
/// would move that ordering out of the parser and into catalog insertion
/// order. [`Capability`] carries no format list because nothing routes on one;
/// add it when a second MRZ-capable provider exists and routing must choose
/// between them.
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

        let tier1 = read_tier1(ctx.text);
        evidence.line1_selection = tier1.line1;
        let Some(mut data) = tier1.parsed.ok() else {
            evidence.missing = synthpass_core::v2::ExtractionFields::default().missing();
            return Ok(Reading {
                extraction: ExtractionV2::default(),
                evidence,
                by: self.id(),
            });
        };

        evidence.observe_mrz(&data);
        evidence.mrz_format = Some(mrz_format_of(&data));
        evidence.blind_positions = Some(blind_positions(&data.mrz_lines));

        // ADR-0026: apply an occlusion observation, if the recognizer made
        // one, to the zone that just parsed — before anything downstream
        // (the checksum gate, `extraction_v2_from_mrz`) sees the covered
        // cells' raw text. No producer sets `Recognition::mrz_occlusion` yet
        // (#565 PR 6 wires `synthpass-ocr`'s detector), so this is
        // unreachable on every path in this workspace today; it is
        // unconditional rather than feature-gated so that landing a
        // producer later needs no further change here.
        let mut occluded_fields: Vec<CoreField> = Vec::new();
        if let Some(occ) = recognition
            .and_then(|r| r.mrz_occlusion.as_ref())
            .filter(|occ| !occ.spans.is_empty())
        {
            match crate::occlusion::apply(&data, occ) {
                Ok((masked, fields)) => {
                    data = masked;
                    occluded_fields = fields;
                }
                Err(_refused) => {
                    // A masked cell fed an ICAO check digit, or was the
                    // format's structural cell: never solved for, even
                    // where the arithmetic could do it uniquely (decision
                    // 7). The reading is not accepted; the routing policy
                    // turns this flag into `EscalationKind::MrzOccluded`.
                    evidence.mrz_occlusion_refused = true;
                    evidence.missing = synthpass_core::v2::ExtractionFields::default().missing();
                    return Ok(Reading {
                        extraction: ExtractionV2::default(),
                        evidence,
                        by: self.id(),
                    });
                }
            }
        }

        if !data.valid() {
            // A record whose check digits do not verify is not a Tier-1 result.
            // Its *evidence* is still valuable — which digits failed is the
            // most actionable escalation reason there is — but the fields are
            // not reported, exactly as before.
            evidence.missing = synthpass_core::v2::ExtractionFields::default().missing();
            return Ok(Reading {
                extraction: ExtractionV2::default(),
                evidence,
                by: self.id(),
            });
        }

        let extraction = extraction_v2_from_mrz_impl(&data, &occluded_fields);
        evidence.observe_line1(
            extraction
                .line1_integrity
                .as_ref()
                .expect("extraction_v2_from_mrz_impl always sets line1_integrity"),
        );
        // Excludes an occluded field (ADR-0026): it is `null` for a reason
        // unrelated to a failed read, and `ExtractionFields::missing` alone
        // cannot tell the two apart — see `Reading::missing`'s doc comment.
        evidence.missing = extraction
            .fields
            .missing()
            .into_iter()
            .filter(|field| !extraction.occluded.contains(field))
            .collect();

        Ok(Reading {
            extraction,
            evidence,
            by: self.id(),
        })
    }
}

/// A Tier-1 read: what `mrz` accepted from a text, and what the shadow line-1
/// selector said about it.
#[derive(Debug, Clone)]
pub struct Tier1Read {
    /// The accepted read, or why there is none. Under the `on` arm this is the
    /// selector's proposal when it made one; otherwise it is exactly what
    /// `mrz::find_and_parse_with` returned.
    pub parsed: Result<mrz::MrzData, mrz::MrzError>,
    /// The selector's verdict, when its arm is `control` or `on` and there was
    /// an accepted read to select on; `None` under `off`, and when nothing was
    /// accepted.
    pub line1: Option<Line1Summary>,
    /// Line 1 of the selector's proposal, under `control` as well as `on`,
    /// for a benchmark that looks the proposal up in the OCR passes' readings
    /// (provenance, never used to decide anything). **Document text**: never
    /// serialise it into a report or log it. `None` when the selector made no
    /// proposal.
    pub proposed_line1: Option<String>,
}

/// The Tier-1 read of an OCR text: [`mrz::find_and_parse_with`] under this
/// process's arms, then the line-1 selector under its own
/// (`SYNTHPASS_MRZ_LINE1_SELECT`, on by default, see [`crate::line1_select_arm`]).
///
/// This is the one place a benchmark and the product read Tier 1, so every arm
/// reads the same way. [`MrzReader`], the pipeline's v1 record
/// (`PipelineResult.mrz`), the synthetic benchmark's Tier-1 read and the
/// real-specimen benchmark's dump zone all call it. The pipeline's Tier-2 hint
/// is built from that same read, but it carries check-digit fields only, which
/// the selector never changes. **Not routed through it, on purpose:** the OCR
/// retry loop's own parse (so its stopping rule and "no extra OCR" hold), the
/// real-specimen benchmark's own hint parse (ADR-0024 step 0b routes it), the
/// prep `mrz_found` parses and the printed-zone validity parse. Those keep
/// their own calls.
#[must_use]
pub fn read_tier1(text: &str) -> Tier1Read {
    read_tier1_with(
        text,
        &crate::mrz_parse_options(),
        crate::line1_select_arm().1,
    )
}

/// [`read_tier1`] with the parse options and the line-1 arm given explicitly,
/// so the arms are tested without touching the process environment.
///
/// - [`Line1Arm::Off`]: exactly `mrz::find_and_parse_with(text, opts)`;
///   `mrz::select_line1` is never called.
/// - [`Line1Arm::Control`]: the selector runs and its verdict is recorded; a
///   proposal is discarded, so `parsed` is what `Off` returns.
/// - [`Line1Arm::On`]: as `Control`, and a proposal replaces `parsed`.
///
/// **Measured** in `knowledge/benchmarks/line1-selection-ab-2026-09-29.md`. The
/// selector changes name fields only.
#[must_use]
pub fn read_tier1_with(text: &str, opts: &mrz::ParseOptions, arm: Line1Arm) -> Tier1Read {
    let mut parsed = mrz::find_and_parse_with(text, opts);
    let selected = match &parsed {
        Ok(accepted) if arm != Line1Arm::Off => Some(summarize(
            arm,
            mrz::select_line1(text, accepted, opts),
            accepted,
        )),
        _ => None,
    };
    let Some((summary, proposal)) = selected else {
        return Tier1Read {
            parsed,
            line1: None,
            proposed_line1: None,
        };
    };
    let proposed_line1 = proposal
        .as_ref()
        .and_then(|data| data.mrz_lines.lines().next())
        .map(str::to_string);
    if let (Line1Arm::On, Some(proposal)) = (arm, proposal) {
        parsed = Ok(proposal);
    }
    Tier1Read {
        parsed,
        line1: Some(summary),
        proposed_line1,
    }
}

/// The text-free summary of `selection` for `accepted`, and the proposal when
/// there is one (which the caller applies only under `on`).
fn summarize(
    arm: Line1Arm,
    selection: mrz::Line1Selection,
    accepted: &mrz::MrzData,
) -> (Line1Summary, Option<mrz::MrzData>) {
    let mut names_changed = false;
    let mut proposal = None;
    let (outcome, reason) = match selection.verdict {
        mrz::Line1Verdict::Proposed(data) => {
            names_changed =
                data.surname != accepted.surname || data.given_names != accepted.given_names;
            proposal = Some(data);
            let outcome = if arm == Line1Arm::On {
                Line1Outcome::Applied
            } else {
                Line1Outcome::Proposed
            };
            (outcome, None)
        }
        mrz::Line1Verdict::Ambiguous => (Line1Outcome::Ambiguous, None),
        mrz::Line1Verdict::Unresolved(why) => (Line1Outcome::Unresolved, unresolved_reason(why)),
        mrz::Line1Verdict::NoCandidate => (Line1Outcome::NoAction, Some(Line1Reason::NoCandidate)),
        mrz::Line1Verdict::Kept => (Line1Outcome::NoAction, Some(Line1Reason::Kept)),
        mrz::Line1Verdict::OutOfScope => (Line1Outcome::NoAction, Some(Line1Reason::OutOfScope)),
        // `mrz::Line1Verdict` is `#[non_exhaustive]`: a verdict this code does
        // not know is reported as no action, without a reason, and never applied.
        _ => (Line1Outcome::NoAction, None),
    };
    let summary = Line1Summary {
        arm,
        outcome,
        reason,
        eligible: selection.eligible,
        distinct: selection.distinct,
        names_changed,
    };
    (summary, proposal)
}

/// The report reason for an `mrz::Line1Unresolved`; `None` for a variant this
/// code does not know (the type is `#[non_exhaustive]`).
fn unresolved_reason(why: mrz::Line1Unresolved) -> Option<Line1Reason> {
    match why {
        mrz::Line1Unresolved::IssuerUnresolved => Some(Line1Reason::IssuerUnresolved),
        mrz::Line1Unresolved::RepeatedLine => Some(Line1Reason::RepeatedLine),
        mrz::Line1Unresolved::DigitInNameField => Some(Line1Reason::DigitInNameField),
        _ => None,
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
///
/// Derived from the system clock with pure arithmetic (via
/// [`mrz::Date::from_epoch_days`]) so the `mrz` crate itself stays clock-free.
fn today() -> mrz::Date {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    mrz::Date::from_epoch_days((secs / 86_400) as i64)
}

/// Map validated MRZ data onto the canonical v1 [`Extraction`] shape — the
/// same shape Tier 2 and the WASM demo produce. Enriches with resolved country
/// names and a date-plausibility summary (checksum-valid does not imply
/// in-date — see [`mrz::MrzData::validity`]).
///
/// Public for the same reason as [`mrz_block_from`]: `synthpass-pipeline`'s
/// Tier-2 accept path serializes this v1 shape for the very same document the
/// Tier-1 path produces, so the two must agree field for field. It carried its
/// own copy until now — the third construction in this module to be unified,
/// after `mrz_block_from` (which had already drifted) and
/// `extraction_v2_from_mrz`. That copy stamped
/// `Method::MrzDeterministic.as_str()` where this one stamps the private
/// `EXTRACTION_METHOD`; both are `"mrz-deterministic"`, which is what makes
/// removing it a dedupe and not a wire change.
pub fn extraction_from_mrz(m: &mrz::MrzData) -> Extraction {
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
        date_of_birth: Some(m.date_of_birth.to_string()),
        sex: synthpass_core::mrz_product::sex(m.sex).map(str::to_string),
        date_of_expiry: Some(m.date_of_expiry.to_string()),
        personal_number: m.personal_number().map(str::to_string),
        optional_data_1: reported_optional_data_1(m).map(str::to_string),
        optional_data_2: m.optional_data_2.clone(),
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

/// The product-schema view of [`mrz::MrzData::optional_data_1`]: the primary
/// optional-data element on the formats that print one *as optional data*
/// (TD1, TD2, MRV-A, MRV-B), and `None` on TD3, where that element is the
/// personal number and is reported under `personal_number` — the field ICAO
/// 9303 Part 4 names, and the one a check digit covers.
///
/// One rule, applied by [`extraction_from_mrz`] and by `synthpass-bench`'s
/// Tier-1 columns, so a v2 record and a benchmark row never disagree about
/// where a value lives (ADR-0018).
pub fn reported_optional_data_1(m: &mrz::MrzData) -> Option<&str> {
    match m.format {
        mrz::Format::Td3 => None,
        _ => m.optional_data_1.as_deref(),
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
/// a behaviour change.
///
/// Thin wrapper over [`extraction_v2_from_mrz_impl`] with an empty occluded
/// set — kept as a separate, unchanged-signature function because
/// `synthpass-bench` already calls it with one argument, and it is not in
/// this PR's scope (#565 PR 3).
pub fn extraction_v2_from_mrz(m: &mrz::MrzData) -> ExtractionV2 {
    extraction_v2_from_mrz_impl(m, &[])
}

/// [`extraction_v2_from_mrz`], plus ADR-0026's occlusion bookkeeping:
/// `occluded` is nulled and set to [`FieldConfidence::OCCLUDED`][occ] in the
/// result, listed in [`ExtractionV2::occluded`] (sorted, deduplicated), and
/// excluded from [`synthpass_core::fusion::check_line1_integrity_excluding`]'s
/// checks so a field `m` already carries as withheld (blanked by
/// [`crate::occlusion::apply`]) can never itself be the cause of a line-1
/// integrity finding — see that function's own doc comment for why a
/// post-hoc filter cannot substitute for excluding it before the checks run.
///
/// [occ]: synthpass_core::v2::FieldConfidence::OCCLUDED
fn extraction_v2_from_mrz_impl(m: &mrz::MrzData, occluded: &[CoreField]) -> ExtractionV2 {
    let block = mrz_block_from(m);
    let mut v2 = ExtractionV2::from(&extraction_from_mrz(m));
    v2.provenance = Provenance::MrzChecksum;
    v2.document.mrz_format = Some(block.format);
    v2.mrz = Some(block);
    // A field a deterministic check contradicts must not keep the confidence it
    // had when nothing contradicted it.
    let verdict = synthpass_core::fusion::check_line1_integrity_excluding(m, occluded);
    v2.confidence.downgrade_flagged(&verdict);
    v2.line1_integrity = Some(verdict);

    for &field in occluded {
        v2.fields.set(field, None);
        v2.confidence.occlude(field);
    }
    if !occluded.is_empty() {
        let mut listed = occluded.to_vec();
        listed.sort();
        listed.dedup();
        v2.occluded = listed;
    }

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
        // Everything a provider could be asked for is missing — which is every
        // field but the two optional-data elements, by `missing`'s own rule.
        assert!(!reading.missing().is_empty());
        assert_eq!(
            reading.missing(),
            synthpass_core::v2::ExtractionFields::default().missing()
        );
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

    // ── ADR-0026: occlusion wiring ──
    //
    // No producer in this workspace sets `Recognition::mrz_occlusion` yet
    // (#565 PR 6) — every `Recognition` below is built by hand, exercising
    // the wiring itself rather than a real detector.

    use crate::provider::Recognition;
    use synthpass_core::v2::{MrzOcclusion, OcclusionKind, OcclusionSpan};

    /// Column 20 of `SPECIMEN`'s line 0 sits inside `given_names`, with the
    /// surname's own `<<` terminator still visible — the same cell
    /// `mrz::apply_occlusion`'s own doc-test uses.
    fn given_names_occlusion() -> MrzOcclusion {
        MrzOcclusion {
            spans: vec![OcclusionSpan {
                line: 0,
                first: 20,
                last: 20,
                kind: OcclusionKind::Fill,
            }],
        }
    }

    fn recognition_with(occ: MrzOcclusion) -> Recognition {
        let mut recognition = Recognition::from_text(SPECIMEN);
        recognition.mrz_occlusion = Some(occ);
        recognition
    }

    #[test]
    fn an_occluded_field_is_null_listed_sorted_deduplicated_and_at_zero_confidence() {
        let recognition = recognition_with(given_names_occlusion());
        let ctx = DocumentContext::from_text(SPECIMEN).with_recognition(&recognition);
        let reading = block_on(MrzReader::new().read(&ctx)).expect("never errs");

        assert_eq!(reading.extraction.occluded, vec![CoreField::GivenNames]);
        assert_eq!(reading.extraction.fields.given_names, None);
        assert_eq!(reading.extraction.confidence.given_names, 0.0);
        // Untouched field: still reported normally.
        assert_eq!(
            reading.extraction.fields.get(CoreField::Surname),
            Some("ERIKSSON")
        );
    }

    #[test]
    fn evidence_missing_excludes_an_occluded_field() {
        let recognition = recognition_with(given_names_occlusion());
        let ctx = DocumentContext::from_text(SPECIMEN).with_recognition(&recognition);
        let reading = block_on(MrzReader::new().read(&ctx)).expect("never errs");

        assert!(
            !reading.evidence.missing.contains(&CoreField::GivenNames),
            "an occluded field is withheld on purpose, not a failed read — \
             `escalate_on_missing_fields` must never see it as missing"
        );
        assert!(
            !reading.missing().contains(&CoreField::GivenNames),
            "Reading::missing must agree with Evidence::missing"
        );
    }

    /// `synthpass_core::fusion`'s
    /// `missing_name_separator_is_suppressed_when_given_names_is_occluded`
    /// pins the fusion function in isolation; this reproduces the same
    /// shape through `extraction_v2_from_mrz_impl` (the function
    /// `MrzReader::read` actually calls), so the wiring itself — not just
    /// the fusion function — is proven to withhold `given_names` before
    /// line-1 integrity ever runs over it. Built with an explicit surname
    /// long enough to trip `MissingNameSeparator` if `given_names` read
    /// back empty for the wrong reason (12 or fewer would never have fired
    /// it, occluded or not, and would prove nothing).
    #[test]
    fn occluding_given_names_never_collaterally_flags_a_long_visible_surname() {
        const SURNAME: &str = "VONHOHENZOLLERNSIGI"; // 19 chars > the 12-char threshold
        const GIVEN: &str = "ANNA<MARIA"; // 10 chars, single-`<`-separated as ICAO requires
        const NAME_FIELD_WIDTH: usize = 39; // TD3: 44 total - 2 (doc code) - 3 (country)
        let filler = "<".repeat(NAME_FIELD_WIDTH - SURNAME.len() - 2 - GIVEN.len());
        let name_field = format!("{SURNAME}<<{GIVEN}{filler}");
        assert_eq!(
            name_field.chars().count(),
            NAME_FIELD_WIDTH,
            "fixture arithmetic"
        );
        let line1 = format!("P<UTO{name_field}");
        assert_eq!(line1.chars().count(), 44, "fixture arithmetic");
        // Unrelated to line 1 — TD3 carries no check digit for any line-1
        // field — reused verbatim from `SPECIMEN` for a valid checksum.
        let line2 = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

        let parsed = mrz::parse_td3(&line1, line2).expect("structural line 1, valid line 2");
        assert!(parsed.valid(), "fixture arithmetic");
        assert_eq!(parsed.surname, SURNAME, "fixture arithmetic");
        assert_eq!(parsed.given_names, "ANNA MARIA", "fixture arithmetic");

        // Mask one cell strictly inside `given_names`' own content (after
        // the surname's `<<` separator, before the field's trailing
        // filler), leaving that separator and the given-names' own visible
        // `<<` terminator untouched — the shape that withholds exactly
        // `given_names`, per `mrz::apply_occlusion`'s name grammar.
        let masked_column = 5 + SURNAME.len() + 2 + 5; // the "M" of "MARIA"
        let occ = MrzOcclusion {
            spans: vec![OcclusionSpan {
                line: 0,
                first: masked_column as u8,
                last: masked_column as u8,
                kind: OcclusionKind::Fill,
            }],
        };
        let (masked, occluded) =
            crate::occlusion::apply(&parsed, &occ).expect("an unverifiable cell never refuses");
        assert_eq!(occluded, vec![CoreField::GivenNames]);
        assert_eq!(masked.given_names, "");
        assert_eq!(
            masked.surname, SURNAME,
            "the visible surname must survive untouched"
        );

        let extraction = extraction_v2_from_mrz_impl(&masked, &occluded);
        assert_eq!(
            extraction.line1_integrity,
            Some(synthpass_core::fusion::Verdict::Accepted),
            "given_names occluded, nothing else wrong: no finding, and no \
             collateral flag on the long, visible surname: {:?}",
            extraction.line1_integrity
        );
    }

    #[test]
    fn a_span_over_a_checked_cell_refuses_the_reading() {
        let recognition = recognition_with(MrzOcclusion {
            // Line 1 (zero-based), column 0: the document-number check
            // digit's first cell.
            spans: vec![OcclusionSpan {
                line: 1,
                first: 0,
                last: 0,
                kind: OcclusionKind::Blur,
            }],
        });
        let ctx = DocumentContext::from_text(SPECIMEN).with_recognition(&recognition);
        let reading = block_on(MrzReader::new().read(&ctx)).expect("never errs");

        assert!(reading.evidence.mrz_occlusion_refused);
        assert_eq!(reading.extraction, ExtractionV2::default());
        assert_eq!(
            crate::routing::RoutingPolicy::default().decide(&reading.evidence),
            crate::routing::Decision::Escalate {
                reason: synthpass_core::v2::EscalationKind::MrzOccluded,
                budget: crate::provider::CostClass::Expensive,
            }
        );
    }

    #[test]
    fn empty_occlusion_spans_change_nothing() {
        let with_empty_occlusion = recognition_with(MrzOcclusion::default());
        let ctx_a = DocumentContext::from_text(SPECIMEN).with_recognition(&with_empty_occlusion);
        let with_empty = block_on(MrzReader::new().read(&ctx_a)).expect("never errs");

        let without = read(SPECIMEN);

        assert_eq!(with_empty.extraction, without.extraction);
        assert_eq!(with_empty.evidence, without.evidence);
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

        let parsed = mrz::find_and_parse(TD1_SPECIMEN).expect("TD1 parses");
        let block = mrz_block_from(&parsed);
        assert_eq!(block.checks.personal_number, None);
        assert_eq!(block.checks.composite, Some(true));
    }

    /// TD1's permanent, structural limitation: no check digit covers line 3
    /// (surname/given_names) or either optional-data field at all — unlike
    /// TD3, this is not something a future fix changes. A checksum-valid TD1
    /// record is consistent with its check digits on the document
    /// number/DOB/expiry/composite, and says nothing about the name. See
    /// `knowledge/ROADMAP.md`'s M6 execution notes.
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
        assert!(
            c.optional_data_1 < 1.0 && c.optional_data_2 < 1.0,
            "neither TD1 optional-data element carries a check digit (ADR-0018)"
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

        let parsed = mrz::find_and_parse(MRV_A_SPECIMEN).expect("MRV-A parses");
        let block = mrz_block_from(&parsed);
        assert_eq!(block.checks.personal_number, None);
        assert_eq!(block.checks.composite, None);
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

    /// M6's "TD1/TD2/MRVA/MRVB as providers" criterion, proven against the
    /// catalog rather than against `MrzReader` directly: this is not "the
    /// parser handles five formats" (the specimen tests above already prove
    /// that) but "the one *registered provider*, looked up the exact way
    /// `synthpass-pipeline`'s `ocr_and_tier1` looks it up
    /// (`find_reader(CostClass::Free, |c| c.deterministic)`),
    /// reads all five ICAO 9303 formats." A catalog holding only `MrzReader`
    /// is the whole point: no second provider exists to register per format,
    /// which is the ADR-0011 amendment's argument made executable.
    #[test]
    fn all_five_formats_read_through_the_provider_catalog() {
        let catalog = crate::ProviderCatalog::builder()
            .with_reader(std::sync::Arc::new(MrzReader::new()))
            .build()
            .expect("MrzReader is the only registered id");

        let reader = catalog
            .find_reader(CostClass::Free, |c| c.deterministic)
            .expect("the deterministic MRZ provider is always registered");

        for (text, expected_format) in [
            (SPECIMEN, MrzFormat::Td3),
            (TD1_SPECIMEN, MrzFormat::Td1),
            (TD2_SPECIMEN, MrzFormat::Td2),
            (MRV_A_SPECIMEN, MrzFormat::MrvA),
            (MRV_B_SPECIMEN, MrzFormat::MrvB),
        ] {
            let reading = block_on(reader.read(&DocumentContext::from_text(text)))
                .expect("the MRZ reader never returns Err");
            assert!(
                reading.evidence.mrz_checksums_valid,
                "{expected_format:?} specimen must validate through the catalog lookup"
            );
            assert_eq!(
                reading.evidence.mrz_format,
                Some(expected_format),
                "catalog-routed read must report the same format as a direct read"
            );
        }
    }

    // ── #574: the shadow line-1 selector, through `read_tier1_with` ──
    //
    // Every case names its arm and its parse options explicitly, so nothing
    // here reads or writes the process environment.

    /// The ICAO specimen's line 2, which every case below pairs with a line 1
    /// of its own making.
    const SPECIMEN_LINE2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

    /// A TD3 line 1 for the specimen's issuer with `name_field` after it,
    /// filler-padded to 44 cells. No name here holds a `K` or an `L`, which the
    /// scanner's repairs read as misread fillers.
    fn specimen_line1(name_field: &str) -> String {
        let mut line = format!("P<UTO{name_field}");
        while line.len() < 44 {
            line.push('<');
        }
        assert_eq!(line.len(), 44);
        line
    }

    /// A `<<<<` run inside the name field breaks the name grammar, and the
    /// scanner accepts the line as read: line 1 has no check digit.
    fn broken_line1() -> String {
        specimen_line1("SPECIMEN<<TESTXYZ<<<<Q")
    }

    fn good_line1() -> String {
        specimen_line1("SPECIMEN<<TESTXYZ")
    }

    /// An OCR run that read the broken line 1 first and the good one second.
    fn broken_then_good() -> String {
        format!(
            "{}\n{SPECIMEN_LINE2}\n\n{}\n{SPECIMEN_LINE2}",
            broken_line1(),
            good_line1()
        )
    }

    fn tier1(text: &str, arm: Line1Arm) -> Tier1Read {
        read_tier1_with(text, &mrz::ParseOptions::default(), arm)
    }

    #[test]
    fn off_is_find_and_parse_with_exactly_and_never_selects() {
        let corrupted = SPECIMEN.replace("L898902C36UTO", "L898902C35UTO");
        let texts = [
            SPECIMEN.to_string(),
            TD1_SPECIMEN.to_string(),
            TD2_SPECIMEN.to_string(),
            MRV_A_SPECIMEN.to_string(),
            MRV_B_SPECIMEN.to_string(),
            "just some ordinary prose with no machine readable zone".to_string(),
            corrupted,
            broken_then_good(),
        ];
        for opts in [
            mrz::ParseOptions::default(),
            mrz::ParseOptions::default().with_class_sweep(true),
        ] {
            for text in &texts {
                let read = read_tier1_with(text, &opts, Line1Arm::Off);
                assert_eq!(read.parsed, mrz::find_and_parse_with(text, &opts), "{text}");
                assert_eq!(read.line1, None, "{text}");
            }
        }
    }

    /// The scanner accepts the broken line 1 as read, which the cases below
    /// build on. If this fails, the fixture no longer builds the state the
    /// selector is meant to see.
    #[test]
    fn the_scanner_accepts_the_broken_line_as_read() {
        let accepted = mrz::find_and_parse_with(&broken_then_good(), &mrz::ParseOptions::default())
            .expect("parses");
        assert!(accepted.valid());
        assert_eq!(
            accepted.mrz_lines,
            format!("{}\n{SPECIMEN_LINE2}", broken_line1())
        );
    }

    #[test]
    fn control_records_a_proposal_and_discards_it() {
        let text = broken_then_good();
        let off = tier1(&text, Line1Arm::Off);
        let control = tier1(&text, Line1Arm::Control);

        assert_eq!(control.parsed, off.parsed, "a placebo changes nothing");
        let summary = control.line1.expect("control records a verdict");
        assert_eq!(summary.arm, Line1Arm::Control);
        assert_eq!(summary.outcome, Line1Outcome::Proposed);
        assert_eq!(summary.reason, None);
        assert_eq!((summary.eligible, summary.distinct), (1, 1));
        assert!(summary.names_changed, "the broken given names carry spaces");
        // The proposal's line 1 is kept for provenance under `control` too, and
        // is absent when the arm is `off`.
        assert_eq!(control.proposed_line1, Some(good_line1()));
        assert_eq!(off.proposed_line1, None);
    }

    #[test]
    fn on_applies_the_proposal_and_changes_only_the_names() {
        let text = broken_then_good();
        let off = tier1(&text, Line1Arm::Off).parsed.expect("parses");
        let on = tier1(&text, Line1Arm::On);

        let summary = on.line1.expect("on records a verdict");
        assert_eq!(summary.arm, Line1Arm::On);
        assert_eq!(summary.outcome, Line1Outcome::Applied);
        assert!(summary.names_changed);

        let applied = on.parsed.expect("still parses");
        assert!(applied.valid());
        assert_eq!(
            applied.mrz_lines,
            format!("{}\n{SPECIMEN_LINE2}", good_line1())
        );
        assert_eq!(applied.surname, "SPECIMEN");
        assert_eq!(applied.given_names, "TESTXYZ");
        assert_eq!(off.given_names, "TESTXYZ    Q");

        let mut restored = applied.clone();
        restored.given_names = off.given_names.clone();
        restored.mrz_lines = off.mrz_lines.clone();
        assert_eq!(restored, off, "nothing but the names and line 1 moved");
    }

    #[test]
    fn a_grammatical_read_is_kept_under_both_arms() {
        for text in [SPECIMEN, TD2_SPECIMEN, MRV_A_SPECIMEN, MRV_B_SPECIMEN] {
            let off = tier1(text, Line1Arm::Off);
            for arm in [Line1Arm::Control, Line1Arm::On] {
                let read = tier1(text, arm);
                assert_eq!(read.parsed, off.parsed, "{arm:?}: {text}");
                let summary = read.line1.expect("a verdict is recorded");
                assert_eq!(summary.outcome, Line1Outcome::NoAction, "{arm:?}");
                assert_eq!(summary.reason, Some(Line1Reason::Kept), "{arm:?}");
                assert!(!summary.names_changed, "{arm:?}");
            }
        }
    }

    #[test]
    fn td1_and_an_unverified_read_are_out_of_scope() {
        let corrupted = SPECIMEN.replace("L898902C36UTO", "L898902C35UTO");
        for text in [TD1_SPECIMEN.to_string(), corrupted] {
            let off = tier1(&text, Line1Arm::Off);
            let on = tier1(&text, Line1Arm::On);
            assert_eq!(on.parsed, off.parsed, "{text}");
            let summary = on.line1.expect("a verdict is recorded");
            assert_eq!(summary.outcome, Line1Outcome::NoAction, "{text}");
            assert_eq!(summary.reason, Some(Line1Reason::OutOfScope), "{text}");
        }
    }

    #[test]
    fn no_accepted_read_has_no_selection_to_record() {
        let prose = "just some ordinary prose with no machine readable zone";
        for arm in [Line1Arm::Off, Line1Arm::Control, Line1Arm::On] {
            let read = tier1(prose, arm);
            assert!(read.parsed.is_err(), "{arm:?}");
            assert_eq!(read.line1, None, "{arm:?}");
        }
    }

    /// Two different well-formed line 1s: nothing is picked, under either arm.
    #[test]
    fn an_ambiguous_verdict_changes_nothing_even_when_on() {
        let other = specimen_line1("SPECIMEN<<TESTXYZW");
        let text = format!("{}\n\n{other}\n{SPECIMEN_LINE2}", broken_then_good());
        let off = tier1(&text, Line1Arm::Off);
        let on = tier1(&text, Line1Arm::On);
        assert_eq!(on.parsed, off.parsed);
        let summary = on.line1.expect("a verdict is recorded");
        assert_eq!(summary.outcome, Line1Outcome::Ambiguous);
        assert_eq!((summary.eligible, summary.distinct), (2, 2));
        assert!(!summary.names_changed);
    }

    #[test]
    fn the_reader_reports_the_selection_in_its_evidence_only_when_the_arm_is_on() {
        // The process arm is `on` unless the environment says otherwise, and
        // this test makes no claim about the environment: it asserts only that
        // the evidence agrees with whatever arm the process is under.
        let arm = crate::line1_select_arm().1;
        let reading = read(SPECIMEN);
        assert_eq!(
            reading.evidence.line1_selection.is_some(),
            arm != Line1Arm::Off
        );
    }
}
