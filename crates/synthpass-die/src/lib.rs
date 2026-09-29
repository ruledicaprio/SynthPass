//! # Document Intelligence Engine
//!
//! The contract every intelligence provider implements — MRZ, OCR, LLM today;
//! vision, barcode, layout and whatever comes next without changing the shape.
//!
//! ## What this crate is for
//!
//! Before it, extraction was two hardcoded tiers: parse the MRZ, and if the
//! check digits fail, call the one LLM the pipeline knows the name of. That
//! works for exactly the providers it was written for. Adding TD1/TD2/MRVA/MRVB
//! means more branches; adding a barcode decoder for driving licences (which
//! carry no MRZ at all) means more branches; comparing two models on the same
//! corpus is not possible, because there is nowhere to put the second one.
//!
//! So the shape of the change is: **Tier 2 stops meaning "run the LLM" and
//! starts meaning "ask a more capable provider only for what deterministic code
//! could not recover."**
//!
//! ## The dependency list is the deliverable
//!
//! This crate depends on `synthpass-core`, `mrz`, `async-trait` and `serde`.
//! Not tokio, not an OCR engine, not llama.cpp — and CI asserts their absence.
//!
//! That is not tidiness. A contract that names one engine cannot be a contract
//! for the others, and "write a provider" must not start with "install
//! llama.cpp". The doc-test on [`FieldReader`] implements a working provider
//! in about twenty lines against this crate alone.
//!
//! ## The one thing to understand before changing anything here
//!
//! **Reported confidence and routing evidence are different things, and the
//! separation is structural.**
//!
//! [`Reading`] holds both an [`ExtractionV2`](synthpass_core::v2::ExtractionV2)
//! (the reported half — the ordinal confidence model, unchanged, part of the
//! published JSON contract) and an [`Evidence`] (the routing half). `Reading`
//! derives no `Serialize`. Neither does `Evidence`. There is therefore no code
//! path that can serialize routing signal into the document JSON, because the
//! type holding both cannot be serialized at all.
//!
//! This exists because the two are easy to conflate and expensive to confuse.
//! `synthpass_core::fusion::Support` explains the reasoning at length: SynthPass
//! has never produced a calibration curve, so a numeric confidence would be
//! "laundering a guess into a number with decimal places". A *routing* score may
//! be uncalibrated — being wrong costs CPU seconds, not truth — but it must
//! never reach a consumer looking like a confidence. Hence two types, and only
//! one of them serializable.
//!
//! See `knowledge/project_principles.md` §2.
//!
//! ## Layout
//!
//! - [`provider`] — the traits, [`Capability`], [`DocumentContext`], errors.
//! - [`evidence`] — what was observed, for routing only.
//! - [`catalog`] — the ordered, immutable provider set.
//! - [`mrz_reader`] — the deterministic ICAO 9303 provider, and [`read_tier1`], the
//!   Tier-1 read it and both benchmarks share (with the opt-in shadow line-1 selector,
//!   `SYNTHPASS_MRZ_LINE1_SELECT`, off by default and unmeasured).
//! - [`routing`] — turns [`Evidence`] into a spend decision, consulted by
//!   `synthpass-pipeline`'s Tier-1 gate.
//! - [`occlusion`] — applies an image-derived occlusion observation to an
//!   already-parsed MRZ zone (ADR-0026), wrapping `mrz::apply_occlusion`.

pub mod catalog;
pub mod evidence;
pub mod mrz_reader;
pub mod occlusion;
pub mod provider;
pub mod routing;

/// The `mrz` parse arm this process measures, from `SYNTHPASS_MRZ_CLASS_SWEEP`.
///
/// Three arms, the shape this repo uses for every same-binary A/B: `on`
/// enables the uniform confusable-class sweep in `mrz`, `off` (the default)
/// does not, and `control` is a **placebo** -- behaviourally identical to
/// `off`, present so a run can tell a real effect from the noise between two
/// nominally identical arms. The decision rule is `on > control >= off`.
///
/// **An unrecognised value falls back to `off` silently**, which is why the
/// arm is returned by name: quote what the binary says it measured, never the
/// variable you believe you set.
///
/// It lives here, beside the MRZ provider, because this is the parse the
/// real-specimen benchmark actually exercises -- the synthetic path in
/// `synthpass-bench` reuses it rather than reading the variable a second time,
/// so the two cannot drift into measuring different arms.
#[must_use]
pub fn class_sweep_arm() -> (&'static str, bool) {
    match std::env::var("SYNTHPASS_MRZ_CLASS_SWEEP")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "on" => ("on", true),
        "control" => ("control", false),
        _ => ("off", false),
    }
}

/// [`mrz::ParseOptions`] for the arm this process measures.
#[must_use]
pub fn mrz_parse_options() -> mrz::ParseOptions {
    mrz_parse_options_for(class_sweep_arm().1)
}

/// [`mrz_parse_options`] for an explicit class-sweep setting, so the mapping is
/// tested without touching the process environment. With the sweep off it is
/// exactly [`mrz::ParseOptions::default`], which is what makes
/// `mrz::find_and_parse` (default options) and [`read_tier1`] under the default
/// arms the same parse: a benchmark's dump zone, which used to call the former,
/// is unchanged by taking the latter.
#[must_use]
pub fn mrz_parse_options_for(class_sweep: bool) -> mrz::ParseOptions {
    mrz::ParseOptions::default().with_class_sweep(class_sweep)
}

/// The shadow line-1 selector's arm, from `SYNTHPASS_MRZ_LINE1_SELECT` (#574).
///
/// The same three arms as [`class_sweep_arm`], with one difference: the
/// selector is not an `mrz::ParseOptions` field, so `control` and `on` are
/// both "run [`mrz::select_line1`] on the accepted read" and differ only in
/// what happens to a proposal.
///
/// - [`Off`](Line1Arm::Off), the default: the selector is not consulted.
/// - [`Control`](Line1Arm::Control): it runs and its verdict is recorded, and a
///   proposal is discarded. The read is byte-identical to `off`, so `off` vs
///   `control` shows what the recording alone changes (nothing) and `control`
///   vs `on` isolates the swap.
/// - [`On`](Line1Arm::On): a proposal replaces the accepted read.
///
/// **Unmeasured.** See [`read_tier1`] for where it applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Line1Arm {
    /// The selector is not consulted. The default.
    #[default]
    Off,
    /// The selector runs and is recorded; a proposal is discarded.
    Control,
    /// The selector runs and is recorded; a proposal replaces the read.
    On,
}

impl Line1Arm {
    /// The arm a `SYNTHPASS_MRZ_LINE1_SELECT` value names: `on`, `control` or
    /// `off`, case-insensitive and trimmed. **Anything else is `off`,
    /// silently**, as [`class_sweep_arm`] does. Pure, so the mapping is tested
    /// without touching the process environment.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "on" => Self::On,
            "control" => Self::Control,
            _ => Self::Off,
        }
    }

    /// The arm's name as the report and the run manifest spell it.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Control => "control",
            Self::On => "on",
        }
    }
}

/// The line-1 selector arm this process measures, by name and by value.
///
/// Read from `SYNTHPASS_MRZ_LINE1_SELECT`; **an unrecognised value falls back
/// to `off` silently**, which is why the name is returned: quote what the binary
/// says it measured, never the variable you believe you set. Beside
/// [`class_sweep_arm`] for the same reason: [`read_tier1`] applies it, and both
/// benches reuse that function, so they cannot drift into measuring different
/// arms.
#[must_use]
pub fn line1_select_arm() -> (&'static str, Line1Arm) {
    let arm = Line1Arm::parse(&std::env::var("SYNTHPASS_MRZ_LINE1_SELECT").unwrap_or_default());
    (arm.name(), arm)
}

/// The non-default `SYNTHPASS_MRZ_*` arms this process runs under, keyed by
/// environment variable name, for `ExtractionTrace::config_overrides`.
///
/// Both knobs change what Tier 1 returns, so a record produced under either
/// must say so (principle 7): a knob appears here exactly when its arm is not
/// `off`, and an all-default process returns an empty map. `control` is listed
/// although it is a placebo, because a run under it is not a default run.
#[must_use]
pub fn mrz_config_overrides() -> std::collections::BTreeMap<String, String> {
    mrz_config_overrides_from(class_sweep_arm().0, line1_select_arm().0)
}

/// [`mrz_config_overrides`] from explicit arm names, so the mapping is tested
/// without touching the process environment.
#[must_use]
pub fn mrz_config_overrides_from(
    class_sweep: &str,
    line1_select: &str,
) -> std::collections::BTreeMap<String, String> {
    let mut overrides = std::collections::BTreeMap::new();
    for (variable, arm) in [
        ("SYNTHPASS_MRZ_CLASS_SWEEP", class_sweep),
        ("SYNTHPASS_MRZ_LINE1_SELECT", line1_select),
    ] {
        if arm != "off" {
            overrides.insert(variable.to_string(), arm.to_string());
        }
    }
    overrides
}

pub use catalog::{CatalogError, ProviderCatalog, ProviderCatalogBuilder};
pub use evidence::{Evidence, Line1Outcome, Line1Reason, Line1Summary};
pub use mrz_reader::{read_tier1, read_tier1_with, MrzReader, Tier1Read, MRZ_PROVIDER_ID};
pub use occlusion::Refused as OcclusionRefused;
pub use provider::{
    Capability, CostClass, DocumentContext, FieldReader, IntelligenceProvider, ProviderError,
    Reading, Recognition, Recognizer,
};
pub use routing::{Decision, RoutingPolicy};

// Wire vocabulary lives in `synthpass-core` (the schema crate must not depend
// on this one), and is re-exported here so a provider author needs one import.
pub use synthpass_core::fusion::{FindingKind, Verdict};
pub use synthpass_core::v2::{CoreField, EscalationKind, ExtractionTrace, PromptRef, ProviderId};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_line1_arm_is_parsed_without_touching_the_environment() {
        for (value, arm) in [
            ("off", Line1Arm::Off),
            ("control", Line1Arm::Control),
            ("on", Line1Arm::On),
            // Case and surrounding whitespace do not matter.
            ("ON", Line1Arm::On),
            ("  Control\n", Line1Arm::Control),
            // Anything unrecognised, including empty, is `off`, silently.
            ("", Line1Arm::Off),
            ("true", Line1Arm::Off),
            ("1", Line1Arm::Off),
            ("onn", Line1Arm::Off),
        ] {
            assert_eq!(Line1Arm::parse(value), arm, "{value:?}");
        }
        assert_eq!(Line1Arm::default(), Line1Arm::Off);
    }

    #[test]
    fn the_line1_arm_names_round_trip() {
        for arm in [Line1Arm::Off, Line1Arm::Control, Line1Arm::On] {
            assert_eq!(Line1Arm::parse(arm.name()), arm);
        }
    }

    /// With the class sweep off the parse options are the crate default, so
    /// `mrz::find_and_parse` and `read_tier1` under the default arms are one
    /// parse (#574: the real-specimen dump zone moved from the former to the
    /// latter).
    #[test]
    fn the_parse_options_with_the_class_sweep_off_are_the_default() {
        assert_eq!(mrz_parse_options_for(false), mrz::ParseOptions::default());
        assert!(mrz_parse_options_for(true).class_sweep);
    }

    #[test]
    fn no_override_is_reported_at_the_defaults() {
        assert!(mrz_config_overrides_from("off", "off").is_empty());
    }

    #[test]
    fn each_mrz_knob_is_reported_by_its_own_variable_when_not_off() {
        use std::collections::BTreeMap;
        assert_eq!(
            mrz_config_overrides_from("on", "off"),
            BTreeMap::from([("SYNTHPASS_MRZ_CLASS_SWEEP".to_string(), "on".to_string())])
        );
        assert_eq!(
            mrz_config_overrides_from("off", "control"),
            BTreeMap::from([(
                "SYNTHPASS_MRZ_LINE1_SELECT".to_string(),
                "control".to_string()
            )])
        );
        assert_eq!(
            mrz_config_overrides_from("control", "on"),
            BTreeMap::from([
                (
                    "SYNTHPASS_MRZ_CLASS_SWEEP".to_string(),
                    "control".to_string()
                ),
                ("SYNTHPASS_MRZ_LINE1_SELECT".to_string(), "on".to_string()),
            ])
        );
    }
}
