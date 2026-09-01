//! The type that turns routing [`Evidence`] into a spend decision.
//!
//! Consulted by `synthpass-pipeline`'s `ocr_and_tier1`, which builds an
//! [`Evidence`] from the catalog's `MrzReader` and calls
//! [`RoutingPolicy::default().decide`](RoutingPolicy::decide) in place of the
//! inline `mrz::find_and_parse(..).filter(|m| m.valid())` gate it used to run.
//! The default policy reproduces that predicate's outcome exactly — see
//! `default_policy_is_exactly_the_v1_2_0_predicate` below.

use synthpass_core::v2::EscalationKind;

use crate::evidence::Evidence;
use crate::provider::CostClass;

/// What to do after a provider returned. Not serializable — it is a spend
/// decision, and [`EscalationKind`] is its only PII-free, wire-legal
/// projection. See `synthpass_core::v2::ExtractionTrace::escalation`, which is
/// what a caller records once it has acted on a [`Decision`] here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Decision {
    /// The reading stands. No further provider is consulted.
    Accept,
    /// Consult the cheapest unconsulted provider at or below `budget`.
    Escalate {
        reason: EscalationKind,
        budget: CostClass,
    },
}

/// Which routing signals are allowed to trigger an escalation, and how far
/// escalation is allowed to spend.
///
/// `#[non_exhaustive]` so a future signal is additive for anyone who built one
/// of these with `..RoutingPolicy::default()` rather than a struct literal.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct RoutingPolicy {
    /// Escalate when the reading left one or more [`CoreField`](synthpass_core::v2::CoreField)s empty.
    pub escalate_on_missing_fields: bool,
    /// Escalate when `check_line1_integrity` flagged the record.
    pub escalate_on_line1_flagged: bool,
    /// Escalate when `Evidence::text_sanity` is present and below this floor.
    ///
    /// `None` disables the gate entirely, and that is the default for a
    /// specific reason, not an oversight: `text_sanity` is a
    /// **character-plausibility proxy, not a model confidence** — see
    /// `knowledge/technical_debt.md`, "OCR confidence is a character-
    /// plausibility proxy". A threshold on a proxy is a threshold on
    /// whatever the proxy correlates with, and nobody has measured that
    /// correlation yet. Setting a floor here before that measurement exists
    /// would invent a number this policy cannot justify; the field exists so
    /// that once the measurement exists, there is somewhere to put it.
    pub sanity_floor: Option<f32>,
    /// The most expensive provider an escalation under this policy may reach
    /// for. Every `Decision::Escalate` this policy produces carries a budget
    /// no higher than this.
    pub max_budget: CostClass,
}

impl Default for RoutingPolicy {
    fn default() -> Self {
        Self::v1_2_0_compatible()
    }
}

impl RoutingPolicy {
    /// The policy whose `decide` reproduces v1.2.0's routing exactly: escalate
    /// if and only if the MRZ did not parse or did not verify.
    ///
    /// Every opt-in signal is off. That is the point, not a placeholder to be
    /// tightened later on a whim: the default policy must reproduce v1.2.0
    /// behaviour exactly, because this crate's introduction must not itself be
    /// a behaviour change. A signal here gets enabled only once the benchmark
    /// harness (a later PR) has produced data saying what it actually
    /// predicts — not because it seems plausible that it should help.
    pub const fn v1_2_0_compatible() -> Self {
        Self {
            escalate_on_missing_fields: false,
            escalate_on_line1_flagged: false,
            sanity_floor: None,
            max_budget: CostClass::Expensive,
        }
    }

    /// Decide what to do with a reading, given what was observed about it.
    ///
    /// # Clause order is normative
    ///
    /// The clauses below are checked in this exact order, and reordering them
    /// changes behaviour, not just which reason gets reported for the same
    /// outcome:
    ///
    /// 1. no MRZ found at all → [`EscalationKind::MrzNotFound`]
    /// 2. an MRZ parsed but a check digit failed → [`EscalationKind::MrzChecksumFailed`]
    /// 3. (opt-in) line 1 integrity flagged the record → [`EscalationKind::Line1Flagged`]
    /// 4. (opt-in) text sanity fell below the floor → [`EscalationKind::OcrBelowSanityFloor`]
    /// 5. (opt-in) one or more fields came back empty → [`EscalationKind::FieldsMissing`]
    /// 6. otherwise → [`Decision::Accept`]
    ///
    /// Clause 1 must precede clause 2: `Evidence::mrz_checksums_valid` is
    /// `false` by construction (`Evidence::default()`) whenever no MRZ was
    /// found at all, since there is no parsed record to have verified. Check
    /// clause 2 first and a blank page — no MRZ-shaped text anywhere — is
    /// reported as `MrzChecksumFailed`, which is a lie: nothing failed to
    /// verify, because nothing was there to verify. `MrzNotFound` is the
    /// honest, more specific cause, and it must win.
    ///
    /// `Evidence::blind_positions` is never read here. It is shipped
    /// observed-only (see `mrz_reader.rs`, `blind_positions_are_observed_only`)
    /// because it is uncalibrated: nobody has measured what a given count
    /// predicts, so routing on it would be inventing a threshold rather than
    /// acting on one.
    pub fn decide(&self, ev: &Evidence) -> Decision {
        if !ev.mrz_found {
            return Decision::Escalate {
                reason: EscalationKind::MrzNotFound,
                budget: self.max_budget,
            };
        }
        if !ev.mrz_checksums_valid {
            return Decision::Escalate {
                reason: EscalationKind::MrzChecksumFailed,
                budget: self.max_budget,
            };
        }
        if self.escalate_on_line1_flagged && ev.line1_flagged() {
            return Decision::Escalate {
                reason: EscalationKind::Line1Flagged,
                budget: self.max_budget,
            };
        }
        if let Some(floor) = self.sanity_floor {
            if let Some(sanity) = ev.text_sanity {
                if sanity < floor {
                    return Decision::Escalate {
                        reason: EscalationKind::OcrBelowSanityFloor,
                        budget: self.max_budget,
                    };
                }
            }
        }
        if self.escalate_on_missing_fields && !ev.missing.is_empty() {
            return Decision::Escalate {
                reason: EscalationKind::FieldsMissing,
                budget: self.max_budget,
            };
        }
        Decision::Accept
    }
}

#[cfg(test)]
mod tests {
    use synthpass_core::fusion::FindingKind;
    use synthpass_core::v2::CoreField;

    use super::*;

    /// Every combination of the opt-in-independent signals, so
    /// `RoutingPolicy::default()` is checked against the whole space rather
    /// than a handful of hand-picked cases.
    ///
    /// `mrz_failed` is swept for the same reason `blind_positions` is, and the
    /// sibling test `blind_positions_never_influences_a_decision` states it
    /// directly: the policy must be provably *independent* of the signals it
    /// does not consult. `mrz_failed` is populated on every parsed record by
    /// `Evidence::observe_mrz` (from `Checks::failed`), so "routing ignores it"
    /// is a property worth pinning rather than assuming.
    ///
    /// It was added while a since-reverted opt-in
    /// (`accept_composite_only_failure`, Chunk 7 of
    /// `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`) *did* consult it, where every
    /// case leaving `mrz_failed` empty via `..Evidence::default()` meant "the
    /// opt-in is off" and "the opt-in was never offered a case to act on" were
    /// indistinguishable here. That opt-in is gone — the measurement that
    /// removed it is in that doc's Chunk 7 section — but the dimension stays:
    /// it costs one loop, it makes the sweep's "whole space" claim honest, and
    /// any future signal that reads `mrz_failed` starts out covered instead of
    /// silently uncovered.
    #[test]
    fn default_policy_is_exactly_the_v1_2_0_predicate() {
        let bools = [false, true];
        let line1_options: [Vec<FindingKind>; 2] =
            [Vec::new(), vec![FindingKind::UnrecognizedNationality]];
        let sanity_options = [None, Some(0.0_f32), Some(0.5_f32), Some(1.0_f32)];
        let missing_options: [Vec<CoreField>; 2] =
            [Vec::new(), vec![CoreField::Surname, CoreField::DateOfBirth]];
        let blind_options = [None, Some(0_u32), Some(40_u32)];
        // Empty, a single-digit failure, and a multi-field one — the shapes
        // `Checks::failed` actually produces.
        let failed_options: [Vec<mrz::Field>; 3] = [
            Vec::new(),
            vec![mrz::Field::Composite],
            vec![mrz::Field::DateOfBirth, mrz::Field::Composite],
        ];

        for &mrz_found in &bools {
            for &mrz_checksums_valid in &bools {
                for line1_findings in &line1_options {
                    for &text_sanity in &sanity_options {
                        for missing in &missing_options {
                            for &blind_positions in &blind_options {
                                for mrz_failed in &failed_options {
                                    let ev = Evidence {
                                        mrz_found,
                                        mrz_checksums_valid,
                                        line1_findings: line1_findings.clone(),
                                        text_sanity,
                                        missing: missing.clone(),
                                        blind_positions,
                                        mrz_failed: mrz_failed.clone(),
                                        ..Evidence::default()
                                    };

                                    // This predicate is exactly today's Tier-1
                                    // gate: `mrz::find_and_parse(t).ok()
                                    // .filter(|m| m.valid()).is_some()`.
                                    let expected = mrz_found && mrz_checksums_valid;
                                    let decision = RoutingPolicy::default().decide(&ev);
                                    assert_eq!(
                                        matches!(decision, Decision::Accept),
                                        expected,
                                        "ev={ev:?} decision={decision:?}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn escalation_reason_prefers_the_more_specific_cause() {
        let ev = Evidence {
            mrz_found: false,
            mrz_checksums_valid: false,
            ..Evidence::default()
        };

        let decision = RoutingPolicy::default().decide(&ev);
        assert_eq!(
            decision,
            Decision::Escalate {
                reason: EscalationKind::MrzNotFound,
                budget: CostClass::Expensive,
            }
        );
    }

    #[test]
    fn blind_positions_never_influences_a_decision() {
        let accepting = Evidence {
            mrz_found: true,
            mrz_checksums_valid: true,
            ..Evidence::default()
        };

        let mut accepting_blind = accepting.clone();
        accepting_blind.blind_positions = Some(0);
        let mut accepting_blind2 = accepting.clone();
        accepting_blind2.blind_positions = Some(40);

        let policy = RoutingPolicy::default();
        assert_eq!(policy.decide(&accepting), policy.decide(&accepting_blind));
        assert_eq!(policy.decide(&accepting), policy.decide(&accepting_blind2));

        let escalating = Evidence {
            mrz_found: false,
            ..Evidence::default()
        };

        let mut escalating_blind = escalating.clone();
        escalating_blind.blind_positions = Some(0);
        let mut escalating_blind2 = escalating.clone();
        escalating_blind2.blind_positions = Some(40);

        assert_eq!(policy.decide(&escalating), policy.decide(&escalating_blind));
        assert_eq!(
            policy.decide(&escalating),
            policy.decide(&escalating_blind2)
        );
    }

    #[test]
    fn opt_in_signals_are_off_by_default() {
        let ev = Evidence {
            mrz_found: true,
            mrz_checksums_valid: true,
            line1_findings: vec![FindingKind::UnrecognizedNationality],
            text_sanity: Some(0.0),
            missing: CoreField::ALL.to_vec(),
            ..Evidence::default()
        };

        assert_eq!(RoutingPolicy::default().decide(&ev), Decision::Accept);

        let line1_on = RoutingPolicy {
            escalate_on_line1_flagged: true,
            ..RoutingPolicy::default()
        };
        assert_eq!(
            line1_on.decide(&ev),
            Decision::Escalate {
                reason: EscalationKind::Line1Flagged,
                budget: line1_on.max_budget,
            }
        );

        let sanity_on = RoutingPolicy {
            sanity_floor: Some(0.5),
            ..RoutingPolicy::default()
        };
        assert_eq!(
            sanity_on.decide(&ev),
            Decision::Escalate {
                reason: EscalationKind::OcrBelowSanityFloor,
                budget: sanity_on.max_budget,
            }
        );

        let missing_on = RoutingPolicy {
            escalate_on_missing_fields: true,
            ..RoutingPolicy::default()
        };
        assert_eq!(
            missing_on.decide(&ev),
            Decision::Escalate {
                reason: EscalationKind::FieldsMissing,
                budget: missing_on.max_budget,
            }
        );
    }
}
