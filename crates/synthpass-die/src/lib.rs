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
//!   Tier-1 read it, the pipeline's v1 record and both benchmarks share (with the
//!   line-1 selector, on by default and measured in
//!   `knowledge/benchmarks/line1-selection-ab-2026-09-29.md`;
//!   `SYNTHPASS_MRZ_LINE1_SELECT=off` opts out).
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

/// The repeated-line refusal arm this process measures, from
/// `SYNTHPASS_MRZ_REFUSE_REPEATED_LINE` (#579).
///
/// The same three arms as [`class_sweep_arm`]: `on` enables
/// [`mrz::ParseOptions::refuse_repeated_line`], `off` (the default) does not,
/// and `control` is a **placebo**, behaviourally identical to `off`. It is
/// opt-in and unmeasured as a default.
///
/// **An unrecognised value falls back to `off` silently**, so the arm is
/// returned by name: quote what the binary says it measured, never the
/// variable you believe you set.
///
/// Like [`class_sweep_arm`] it reaches `mrz` through [`mrz_parse_options`], so
/// the product's [`MrzReader`] and both benches measure the same arm. The OCR
/// retry loop's stopping rule does not take it: that loop keeps its own
/// checksum-only oracle on the default options.
#[must_use]
pub fn refuse_repeated_line_arm() -> (&'static str, bool) {
    refuse_repeated_line_arm_from(
        &std::env::var("SYNTHPASS_MRZ_REFUSE_REPEATED_LINE").unwrap_or_default(),
    )
}

/// [`refuse_repeated_line_arm`] for an explicit value, so the mapping is tested
/// without touching the process environment.
#[must_use]
pub fn refuse_repeated_line_arm_from(value: &str) -> (&'static str, bool) {
    match value.trim().to_ascii_lowercase().as_str() {
        "on" => ("on", true),
        "control" => ("control", false),
        _ => ("off", false),
    }
}

/// [`mrz::ParseOptions`] for the arms this process measures.
#[must_use]
pub fn mrz_parse_options() -> mrz::ParseOptions {
    mrz_parse_options_for(class_sweep_arm().1, refuse_repeated_line_arm().1)
}

/// [`mrz_parse_options`] for explicit class-sweep and repeated-line-refusal
/// settings, so the mapping is tested without touching the process
/// environment. With both off it is exactly [`mrz::ParseOptions::default`],
/// which is what makes `mrz::find_and_parse` (default options) and
/// [`read_tier1`] under the default arms the same parse: a benchmark's dump
/// zone, which used to call the former, is unchanged by taking the latter.
#[must_use]
pub fn mrz_parse_options_for(class_sweep: bool, refuse_repeated_line: bool) -> mrz::ParseOptions {
    mrz::ParseOptions::default()
        .with_class_sweep(class_sweep)
        .with_refuse_repeated_line(refuse_repeated_line)
}

/// The line-1 selector's arm, from `SYNTHPASS_MRZ_LINE1_SELECT` (#574).
///
/// The same three arms as [`class_sweep_arm`], with two differences: the
/// selector is not an `mrz::ParseOptions` field, so `control` and `on` are
/// both "run [`mrz::select_line1`] on the accepted read" and differ only in
/// what happens to a proposal; and the default is `on`, not `off`.
///
/// - [`Off`](Line1Arm::Off): the selector is not consulted. An explicit opt-out
///   that restores the read as it was before the selector.
/// - [`Control`](Line1Arm::Control): it runs and its verdict is recorded, and a
///   proposal is discarded. The read is byte-identical to `off`, so `off` vs
///   `control` shows what the recording alone changes (nothing) and `control`
///   vs `on` isolates the swap.
/// - [`On`](Line1Arm::On), the default: a proposal replaces the accepted read.
///
/// **Measured.** The A/B behind the promotion is
/// `knowledge/benchmarks/line1-selection-ab-2026-09-29.md`: no outcome changed,
/// no field went from correct to wrong, strict names went from 13/45 to 15/45,
/// and every synthetic format was identical seed for seed. See [`read_tier1`]
/// for where it applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Line1Arm {
    /// The selector is not consulted. An explicit opt-out.
    Off,
    /// The selector runs and is recorded; a proposal is discarded.
    Control,
    /// The selector runs and is recorded; a proposal replaces the read. The
    /// default.
    #[default]
    On,
}

impl Line1Arm {
    /// The arm a `SYNTHPASS_MRZ_LINE1_SELECT` value names: `on`, `control` or
    /// `off`, case-insensitive and trimmed. **Anything else, including empty,
    /// is `on`, the default, silently**, the rule [`class_sweep_arm`] follows
    /// for its own default. Pure, so the mapping is tested without touching the
    /// process environment.
    #[must_use]
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "off" => Self::Off,
            "control" => Self::Control,
            _ => Self::On,
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
/// Read from `SYNTHPASS_MRZ_LINE1_SELECT`; **an unset, empty or unrecognised
/// value falls back to `on`, the default, silently**, which is why the name is returned: quote what the binary
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
/// Each knob changes what Tier 1 returns, so a record produced under any of
/// them must say so (principle 7): a knob appears here exactly when its arm is
/// not its default, and an all-default process returns an empty map. The
/// default of the class sweep is `off`, so it is listed when not `off`; the
/// default of the line-1 selector is `on`, so it is listed when not `on`; the
/// default of the repeated-line refusal (#579) is `off`. `control` is listed
/// although it is a placebo, because a run under it is not a default run.
#[must_use]
pub fn mrz_config_overrides() -> std::collections::BTreeMap<String, String> {
    mrz_config_overrides_from(
        class_sweep_arm().0,
        line1_select_arm().0,
        refuse_repeated_line_arm().0,
    )
}

/// [`mrz_config_overrides`] from explicit arm names, so the mapping is tested
/// without touching the process environment.
#[must_use]
pub fn mrz_config_overrides_from(
    class_sweep: &str,
    line1_select: &str,
    refuse_repeated_line: &str,
) -> std::collections::BTreeMap<String, String> {
    let mut overrides = std::collections::BTreeMap::new();
    for (variable, arm, default) in [
        ("SYNTHPASS_MRZ_CLASS_SWEEP", class_sweep, "off"),
        ("SYNTHPASS_MRZ_LINE1_SELECT", line1_select, "on"),
        (
            "SYNTHPASS_MRZ_REFUSE_REPEATED_LINE",
            refuse_repeated_line,
            "off",
        ),
    ] {
        if arm != default {
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
            ("OFF", Line1Arm::Off),
            // Anything unrecognised, including empty, is `on`, the default, silently.
            ("", Line1Arm::On),
            ("garbage", Line1Arm::On),
            ("true", Line1Arm::On),
            ("1", Line1Arm::On),
            ("offf", Line1Arm::On),
        ] {
            assert_eq!(Line1Arm::parse(value), arm, "{value:?}");
        }
        assert_eq!(Line1Arm::default(), Line1Arm::On);
    }

    #[test]
    fn the_line1_arm_names_round_trip() {
        for arm in [Line1Arm::Off, Line1Arm::Control, Line1Arm::On] {
            assert_eq!(Line1Arm::parse(arm.name()), arm);
        }
    }

    /// With both switches off the parse options are the crate default, so
    /// `mrz::find_and_parse` and `read_tier1` under the default arms are one
    /// parse (#574: the real-specimen dump zone moved from the former to the
    /// latter).
    #[test]
    fn the_parse_options_with_every_switch_off_are_the_default() {
        assert_eq!(
            mrz_parse_options_for(false, false),
            mrz::ParseOptions::default()
        );
        assert!(mrz_parse_options_for(true, false).class_sweep);
        assert!(!mrz_parse_options_for(true, false).refuse_repeated_line);
        assert!(mrz_parse_options_for(false, true).refuse_repeated_line);
        assert!(!mrz_parse_options_for(false, true).class_sweep);
    }

    /// #579: the refusal arm is parsed without touching the environment. Only
    /// `on` enables it; `control` is a placebo; anything unrecognised is `off`,
    /// silently, and the arm is returned by name.
    #[test]
    fn the_refuse_repeated_line_arm_is_parsed_without_touching_the_environment() {
        for (value, arm) in [
            ("off", ("off", false)),
            ("control", ("control", false)),
            ("on", ("on", true)),
            // Case and surrounding whitespace do not matter.
            ("ON", ("on", true)),
            ("  Control\n", ("control", false)),
            // Anything unrecognised, including empty, is `off`, silently.
            ("", ("off", false)),
            ("garbage", ("off", false)),
            ("true", ("off", false)),
            ("1", ("off", false)),
            ("onn", ("off", false)),
        ] {
            assert_eq!(refuse_repeated_line_arm_from(value), arm, "{value:?}");
        }
    }

    /// The defaults are class sweep `off`, line-1 select `on` and the repeated-
    /// line refusal `off`.
    #[test]
    fn no_override_is_reported_at_the_defaults() {
        assert!(mrz_config_overrides_from("off", "on", "off").is_empty());
    }

    #[test]
    fn each_mrz_knob_is_reported_by_its_own_variable_when_not_its_default() {
        use std::collections::BTreeMap;
        assert_eq!(
            mrz_config_overrides_from("on", "on", "off"),
            BTreeMap::from([("SYNTHPASS_MRZ_CLASS_SWEEP".to_string(), "on".to_string())])
        );
        assert_eq!(
            mrz_config_overrides_from("off", "off", "off"),
            BTreeMap::from([("SYNTHPASS_MRZ_LINE1_SELECT".to_string(), "off".to_string())])
        );
        assert_eq!(
            mrz_config_overrides_from("off", "control", "off"),
            BTreeMap::from([(
                "SYNTHPASS_MRZ_LINE1_SELECT".to_string(),
                "control".to_string()
            )])
        );
        assert_eq!(
            mrz_config_overrides_from("control", "off", "off"),
            BTreeMap::from([
                (
                    "SYNTHPASS_MRZ_CLASS_SWEEP".to_string(),
                    "control".to_string()
                ),
                ("SYNTHPASS_MRZ_LINE1_SELECT".to_string(), "off".to_string()),
            ])
        );
    }

    /// #579: the refusal knob is reported by its own variable when not `off`,
    /// `control` included, and the three knobs compose independently.
    #[test]
    fn the_refusal_knob_is_reported_by_its_own_variable_when_not_off() {
        use std::collections::BTreeMap;
        for arm in ["on", "control"] {
            assert_eq!(
                mrz_config_overrides_from("off", "on", arm),
                BTreeMap::from([(
                    "SYNTHPASS_MRZ_REFUSE_REPEATED_LINE".to_string(),
                    arm.to_string()
                )])
            );
        }
        assert_eq!(
            mrz_config_overrides_from("on", "off", "on"),
            BTreeMap::from([
                ("SYNTHPASS_MRZ_CLASS_SWEEP".to_string(), "on".to_string()),
                ("SYNTHPASS_MRZ_LINE1_SELECT".to_string(), "off".to_string()),
                (
                    "SYNTHPASS_MRZ_REFUSE_REPEATED_LINE".to_string(),
                    "on".to_string()
                ),
            ])
        );
    }
}
