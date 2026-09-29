//! What was observed while reading a document — the input to a routing
//! decision, and nothing else.
//!
//! # Why this is a separate type from confidence
//!
//! `synthpass_core::v2::FieldConfidence` answers *"how strongly is this value
//! believed"* and is part of the published contract. [`Evidence`] answers
//! *"is it worth spending more compute on this document"*. They look similar
//! and are not: the first is a claim about truth, made to a consumer; the
//! second is an input to a spend decision, made to ourselves.
//!
//! Conflating them is the failure `synthpass_core::fusion::Support`'s doc
//! comment describes — laundering a guess into a number with decimal places.
//! Keeping them in different types, in different crates, with only one of them
//! serializable, is what stops that happening by accident.
//!
//! **[`Evidence`] derives no `Serialize`, deliberately and permanently.**

use mrz::MrzData;
use synthpass_core::fusion::{FindingKind, Verdict};
use synthpass_core::v2::{CoreField, MrzFormat};

/// Everything observed about one document, as signals a router can act on.
///
/// Every field here is a signal that **already exists** in the codebase — none
/// of this is new measurement. Several were being computed and discarded before
/// the provider model gave them somewhere to go.
///
/// `#[non_exhaustive]` with `Default`: adding a signal must never break a
/// provider that constructs one.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Evidence {
    // ---- from `mrz`, all already public ----
    /// An MRZ-shaped block was found and parsed.
    pub mrz_found: bool,
    /// Every ICAO check digit verified — `MrzData::valid()`.
    ///
    /// The single most important signal here, and the only one the default
    /// routing policy consults: it is a *mathematical proof* of a faithful
    /// read, not a heuristic.
    pub mrz_checksums_valid: bool,
    /// Which check digits failed, from `Checks::failed()`. `mrz::Field` is
    /// fieldless, so this is shape, not content.
    pub mrz_failed: Vec<mrz::Field>,
    /// Which MRZ layout was recognized.
    pub mrz_format: Option<MrzFormat>,
    /// Count of characters in the read whose OCR confusions are *invisible to
    /// the check digits* — `mrz::blindspot` classifies a substitution as
    /// `Blind` when the two characters are congruent mod 10, so the checksum
    /// cannot distinguish them.
    ///
    /// This is the one honest "the oracle cannot see this" signal available
    /// for a read that *passed* its checksums, which is otherwise the strongest
    /// evidence in the system. Shipped **observed-only**: recorded, never
    /// routed on, until a sweep says what a given count actually predicts.
    pub blind_positions: Option<u32>,

    // ---- from `synthpass_core::fusion` ----
    /// The deterministic line-1 integrity verdict, as PII-free kinds.
    ///
    /// [`FindingKind`], never `Finding`: this value gets logged and turned into
    /// a metric label, and `Finding` carries country codes read off a real
    /// document. The full findings stay in
    /// `ExtractionV2.line1_integrity`, inside the zeroized JSON.
    pub line1_findings: Vec<FindingKind>,

    // ---- from the recognizer ----
    /// How MRZ-shaped the best band candidate looked, in `[0, 1]`.
    pub mrz_band_score: Option<f64>,
    /// Whole-page character plausibility, in `[0, 1]`. A proxy, not a model
    /// score — see `synthpass_ocr::geometry::text_sanity`.
    pub text_sanity: Option<f32>,
    /// A portrait region was located.
    pub portrait_found: bool,
    /// Rotation applied before recognition, in degrees clockwise.
    pub rotation_applied: u16,
    /// Recognized character count. **Shape, never content** — a length is not
    /// a value.
    pub text_chars: usize,
    /// Recognized line count. Shape.
    pub text_lines: usize,

    // ---- derived from what a reader produced ----
    /// Fields that came back with nothing usable. Never lists the two
    /// optional-data elements — see `ExtractionFields::missing` (ADR-0018).
    /// Also never lists an occluded field — see `Reading::missing`'s own doc
    /// comment (ADR-0026); this is the same exclusion applied at the
    /// evidence layer, so `escalate_on_missing_fields` can never send a
    /// covered field to Tier 2.
    pub missing: Vec<CoreField>,

    // ---- from `synthpass_die::occlusion` ----
    /// An occlusion mask covered a cell an ICAO check digit verifies (or the
    /// format's one structural cell), so `synthpass_die::occlusion::apply`
    /// refused the zone and the reading was not accepted (ADR-0026, decision
    /// 7). `false` both when no occlusion was observed and when one was
    /// observed but applied cleanly — see [`RoutingPolicy::decide`] for how
    /// this is turned into an escalation.
    ///
    /// [`RoutingPolicy::decide`]: crate::routing::RoutingPolicy::decide
    pub mrz_occlusion_refused: bool,

    // ---- from `mrz::select_line1`, when its arm is not `off` ----
    /// What the shadow line-1 selector said about the accepted read (#574).
    /// `None` when the arm is `off`, and when Tier 1 accepted nothing to select
    /// on. Text-free by construction: verdict kinds and counts, never a name
    /// or a line. **Observed-only**: nothing routes on it.
    pub line1_selection: Option<Line1Summary>,
}

/// What the shadow line-1 selector concluded, as a verdict kind and counts.
///
/// [`Line1Outcome`] and [`Line1Reason`] are fieldless and `eligible`/`distinct`
/// are counts, so nothing here holds a name or a zone line and the summary is
/// safe to log and to write into a benchmark report (ADR-0027). It derives no
/// `Serialize`, like [`Evidence`]: a benchmark renders its own text-free view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Line1Summary {
    /// The arm the read was made under: `control` or `on`, never `off`.
    pub arm: crate::Line1Arm,
    /// What the selector found, with `on` reported as [`Line1Outcome::Applied`]
    /// where `control` reports [`Line1Outcome::Proposed`].
    pub outcome: Line1Outcome,
    /// Why the selector did nothing, for [`Line1Outcome::NoAction`] and
    /// [`Line1Outcome::Unresolved`]; `None` for the other outcomes.
    pub reason: Option<Line1Reason>,
    /// Candidate lines that passed every eligibility check, duplicates
    /// included. See `mrz::Line1Selection::eligible`.
    pub eligible: usize,
    /// Distinct name fields among them. See `mrz::Line1Selection::distinct`.
    pub distinct: usize,
    /// Whether the proposal's `surname` or `given_names` differ from the
    /// accepted read's. `false` unless the outcome is `Proposed` or `Applied`.
    pub names_changed: bool,
}

/// The kind of verdict a [`Line1Summary`] carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Line1Outcome {
    /// The arm is `on` and a proposal replaced the accepted read.
    Applied,
    /// The arm is `control` and the selector found a proposal, which was
    /// discarded.
    Proposed,
    /// The accepted zone shows a wrong-physical-line symptom, so the selector
    /// did not look at its name field. Carries the [`Line1Reason`].
    Unresolved,
    /// More than one distinct eligible name field.
    Ambiguous,
    /// Nothing to do, with the [`Line1Reason`] why. Spelled `none` in the report.
    NoAction,
}

impl Line1Outcome {
    /// The spelling the benchmark report uses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Applied => "applied",
            Self::Proposed => "proposed",
            Self::Unresolved => "unresolved",
            Self::Ambiguous => "ambiguous",
            Self::NoAction => "none",
        }
    }
}

/// Why a [`Line1Summary`] is `NoAction` or `Unresolved`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Line1Reason {
    /// The accepted read is TD1 or fails a check digit.
    OutOfScope,
    /// The accepted name field already follows the name grammar.
    Kept,
    /// The name field is ungrammatical and no other line is eligible.
    NoCandidate,
    /// Unresolved: line 1's issuing state is not in the country table.
    IssuerUnresolved,
    /// Unresolved: two of the zone's lines are near-identical.
    RepeatedLine,
    /// Unresolved: a digit sits in line 1's name field.
    DigitInNameField,
}

impl Line1Reason {
    /// The spelling the benchmark report uses.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OutOfScope => "out_of_scope",
            Self::Kept => "kept",
            Self::NoCandidate => "no_candidate",
            Self::IssuerUnresolved => "issuer_unresolved",
            Self::RepeatedLine => "repeated_line",
            Self::DigitInNameField => "digit_in_name_field",
        }
    }
}

impl Evidence {
    /// Record what a parsed MRZ says about itself.
    ///
    /// Takes the parsed record rather than raw text so the caller cannot
    /// accidentally hand this function a document body — everything read here
    /// is structural.
    pub fn observe_mrz(&mut self, data: &MrzData) {
        self.mrz_found = true;
        self.mrz_checksums_valid = data.valid();
        self.mrz_failed = data.checks.failed();
    }

    /// Record a line-1 integrity verdict, stripped to kinds.
    pub fn observe_line1(&mut self, verdict: &Verdict) {
        // Raw-line observations are serialized in the extraction verdict but
        // do not feed even the opt-in Line1Flagged routing clause.
        self.line1_findings = verdict
            .kinds()
            .into_iter()
            .filter(|kind| !kind.is_report_only())
            .collect();
    }

    /// Whether anything at all was recognized.
    pub fn has_text(&self) -> bool {
        self.text_chars > 0
    }

    /// Whether the deterministic line-1 checks flagged the record.
    pub fn line1_flagged(&self) -> bool {
        !self.line1_findings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The type-level half of the reported/routing split. If `Evidence` ever
    /// gains a `Serialize` impl, routing signal becomes serializable, and the
    /// only thing preventing it from reaching a consumer alongside `confidence`
    /// is somebody remembering not to. This test documents the intent; the
    /// compiler enforces it, because `Reading` cannot derive `Serialize` while
    /// this type does not.
    #[test]
    fn evidence_carries_only_shape_never_content() {
        let e = Evidence {
            text_chars: 1234,
            text_lines: 42,
            ..Evidence::default()
        };

        // Every field is a count, a boolean, a bounded score or a fieldless
        // enum. There is nowhere in this struct to put a name or a document
        // number, which is what makes it safe to log.
        let debug = format!("{e:?}");
        assert!(debug.contains("1234"));
        assert!(!debug.contains("ERIKSSON"));
    }

    #[test]
    fn line1_flagged_reflects_findings() {
        let mut e = Evidence::default();
        assert!(!e.line1_flagged());

        e.observe_line1(&Verdict::Accepted);
        assert!(!e.line1_flagged());

        e.observe_line1(&Verdict::NeedsReview {
            reasons: vec![synthpass_core::fusion::Finding::UnrecognizedNationality {
                got: "XXX".into(),
            }],
        });
        assert!(e.line1_flagged());
        assert_eq!(e.line1_findings, vec![FindingKind::UnrecognizedNationality]);

        // The PII the finding carried did not survive the projection.
        assert!(!format!("{:?}", e.line1_findings).contains("XXX"));
    }

    #[test]
    fn report_only_raw_findings_do_not_trigger_line1_routing() {
        let verdict = Verdict::NeedsReview {
            reasons: vec![synthpass_core::fusion::Finding::RawNameSeparatorMissing],
        };
        let mut evidence = Evidence::default();
        evidence.observe_line1(&verdict);
        assert!(evidence.line1_findings.is_empty());
        assert!(!evidence.line1_flagged());
        assert!(verdict.is_flagged());
    }
}
