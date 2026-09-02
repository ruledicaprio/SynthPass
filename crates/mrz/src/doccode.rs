//! ICAO 9303 Part 4 §4.4 — secondary document codes for machine readable
//! passports.
//!
//! MRZ line 1 positions 1-2 hold the document code. For an MRP the first
//! character is `P`; the second identifies the passport *type* against a table
//! Part 4 §4.4 enumerates in full:
//!
//! ```text
//! PP  National/ordinary passport      PT  Alien/Non-citizen passport
//! PE  Emergency passport              PS  Stateless passport
//! PD  Diplomatic passport             PL  Laissez-passer passport
//! PO  Official/service passport       PM  Military passport
//! PR  Refugee passport                PU  Single-sheet document (Part 8)
//! ```
//!
//! # Why this table exists when the TD1/TD2 one deliberately does not
//!
//! `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` records a decision *not* to build a
//! document-code dictionary, and that decision stands — but it was reasoned
//! about **TD1 and TD2**, citing Part 5 §Note k and Part 6 §Note k: *"The
//! second character shall be at the discretion of the issuing State or
//! organization except that V shall not be used, and C shall not be used after
//! A."* That is an open set with two exclusions, so there is no registry to
//! build a dictionary from, and a table assembled from observed codes would be
//! exactly the unproven heuristic this crate refuses to ship.
//!
//! Part 4 is the one format where that reasoning does not apply. §4.4 is a
//! closed normative table, quoted above, and it is already in this repository
//! at `knowledge/docs9303/Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md`.
//!
//! # Recognition, never rejection
//!
//! An unrecognised second character does not make a passport unparseable, and
//! [`parse_td3`](crate::parse_td3) still accepts any character the MRZ alphabet
//! allows. §4.4 itself is explicit that conformance is still arriving:
//!
//! > Effective 1 January 2026, MRPs issued with a secondary document code shall
//! > be in accordance with Section 4.4. Effective 1 January 2028, all MRPs shall
//! > be issued with a secondary document code in accordance with Section 4.4.
//! > MRPs issued without a harmonized secondary document code in accordance with
//! > Section 4.4 shall expire before 1 January 2038.
//!
//! So a passport reading `P<` — the filler form, by far the most common in this
//! crate's corpus — is conformant today and will remain valid for years. This
//! module reports what a code *means*; it never decides whether a document is
//! acceptable.

/// The passport type a §4.4 secondary document code designates.
///
/// Obtained from [`MrzData::passport_type`](crate::MrzData::passport_type).
/// `None` there covers three distinct situations that callers usually want to
/// keep apart, and which this enum deliberately does not merge: a non-passport
/// format, the `P<` filler form carrying no secondary code, and a second
/// character outside the table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PassportType {
    /// `PP` — national/ordinary passport.
    National,
    /// `PE` — emergency passport.
    Emergency,
    /// `PD` — diplomatic passport.
    Diplomatic,
    /// `PO` — official/service passport.
    Official,
    /// `PR` — refugee passport.
    Refugee,
    /// `PT` — alien/non-citizen passport.
    Alien,
    /// `PS` — stateless passport.
    Stateless,
    /// `PL` — laissez-passer.
    LaissezPasser,
    /// `PM` — military passport.
    Military,
    /// `PU` — single-sheet travel document; the designated code per Part 8.
    SingleSheet,
}

impl PassportType {
    /// The two-character document code, as it appears in MRZ line 1.
    pub const fn code(self) -> &'static str {
        match self {
            Self::National => "PP",
            Self::Emergency => "PE",
            Self::Diplomatic => "PD",
            Self::Official => "PO",
            Self::Refugee => "PR",
            Self::Alien => "PT",
            Self::Stateless => "PS",
            Self::LaissezPasser => "PL",
            Self::Military => "PM",
            Self::SingleSheet => "PU",
        }
    }

    /// The document type §4.4 names for this code.
    pub const fn name(self) -> &'static str {
        match self {
            Self::National => "National/ordinary passport",
            Self::Emergency => "Emergency passport",
            Self::Diplomatic => "Diplomatic passport",
            Self::Official => "Official/service passport",
            Self::Refugee => "Refugee passport",
            Self::Alien => "Alien/Non-citizen passport",
            Self::Stateless => "Stateless passport",
            Self::LaissezPasser => "Laissez-passer passport",
            Self::Military => "Military passport",
            Self::SingleSheet => "Single-sheet travel document",
        }
    }

    /// Every code in the §4.4 table, in the order the specification lists them.
    pub const ALL: [Self; 10] = [
        Self::National,
        Self::Emergency,
        Self::Diplomatic,
        Self::Official,
        Self::Refugee,
        Self::Alien,
        Self::Stateless,
        Self::LaissezPasser,
        Self::Military,
        Self::SingleSheet,
    ];
}

/// Looks up a document code against the §4.4 table.
///
/// Accepts the code as it appears in the MRZ, trailing filler included, so both
/// `"PP"` and a already-trimmed `"P"` behave sensibly:
///
/// ```
/// use mrz::{passport_type, PassportType};
///
/// assert_eq!(passport_type("PD"), Some(PassportType::Diplomatic));
/// assert_eq!(passport_type("PP"), Some(PassportType::National));
///
/// // `P<` is the filler form: a passport with no secondary code, which is
/// // conformant today and not the same thing as an unknown code.
/// assert_eq!(passport_type("P<"), None);
/// assert_eq!(passport_type("P"), None);
///
/// // Not in the table, and not a passport at all.
/// assert_eq!(passport_type("PZ"), None);
/// assert_eq!(passport_type("ID"), None);
/// ```
pub fn passport_type(code: &str) -> Option<PassportType> {
    let code = code.trim_end_matches('<');
    PassportType::ALL.into_iter().find(|t| t.code() == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_code_in_the_table_round_trips() {
        for t in PassportType::ALL {
            assert_eq!(
                passport_type(t.code()),
                Some(t),
                "{} did not round-trip",
                t.code()
            );
            assert!(!t.name().is_empty());
        }
    }

    #[test]
    fn the_table_matches_part_4_section_4_4_exactly() {
        // Transcribed from §4.4 of the Part 4 specification under
        // `knowledge/docs9303/`, plus Note 1's PU from Part 8. Pinned so a
        // future edit to the enum has to be a deliberate change to the spec
        // table. (The full path is cited unbroken in this module's header —
        // scripts/check-doc-links.sh matches these citations per line, so
        // wrapping one across two comment lines makes it look like a dangling
        // reference.)
        let expected = [
            ("PP", "National/ordinary passport"),
            ("PE", "Emergency passport"),
            ("PD", "Diplomatic passport"),
            ("PO", "Official/service passport"),
            ("PR", "Refugee passport"),
            ("PT", "Alien/Non-citizen passport"),
            ("PS", "Stateless passport"),
            ("PL", "Laissez-passer passport"),
            ("PM", "Military passport"),
            ("PU", "Single-sheet travel document"),
        ];
        let actual: Vec<(&str, &str)> = PassportType::ALL
            .into_iter()
            .map(|t| (t.code(), t.name()))
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn the_filler_form_is_not_an_unknown_code() {
        // `P<` means "no secondary code", which §4.4's own effective dates make
        // conformant for years yet. Reporting it as unrecognised would be wrong.
        assert_eq!(passport_type("P<"), None);
        assert_eq!(passport_type("P"), None);
    }

    #[test]
    fn a_second_character_outside_the_table_does_not_resolve() {
        for code in ["PZ", "PX", "PA", "PB", "PC", "PN", "PV"] {
            assert_eq!(passport_type(code), None, "{code} must not resolve");
        }
    }

    #[test]
    fn non_passport_codes_never_resolve() {
        for code in ["ID", "I<", "AC", "C<", "V<", "IP"] {
            assert_eq!(passport_type(code), None, "{code} is not an MRP code");
        }
    }
}
