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
    /// Fields that came back with nothing usable.
    pub missing: Vec<CoreField>,
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
        self.line1_findings = verdict.kinds();
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
}
