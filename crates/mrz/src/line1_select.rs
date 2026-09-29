//! The shadow line-1 selector: [`select_line1`].
//!
//! A two-line format's line 1 (TD3, TD2, MRV-A, MRV-B) carries no check digit,
//! so nothing arbitrates between OCR readings of it: the scan pairs the first
//! line 1 it can with a checksum-valid line 2, and a misread name field is
//! accepted as readily as a correct one. [`select_line1`] looks in the same OCR
//! text for a better line 1 for the zone `find_and_parse_with` already
//! accepted, and *says* what it found. It never replaces anything itself.
//!
//! The rule itself is documented on [`select_line1`], the one public item.

use crate::checksum::{is_mrz_charset, normalize_line};
use crate::parser::{
    candidate_lines, parse_mrv_a_with, parse_mrv_b_with, parse_td2_with, parse_td3_with,
};
use crate::rank::{flag_reason, FlagReason};
use crate::{Format, MrzData, MrzError, ParseOptions};

/// The first cell of line 1 that belongs to the name field: cells 0..2 are the
/// document code and 2..5 the issuing state.
const NAME_FIELD_START: usize = 5;

/// What [`select_line1`] found. **Unmeasured, and off by default in every
/// caller in this workspace.**
///
/// `#[non_exhaustive]`: an output type, obtained from [`select_line1`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Line1Selection {
    /// The verdict.
    pub verdict: Line1Verdict,
    /// How many candidate lines passed every eligibility check, duplicates
    /// included. `0` when the verdict was reached before candidates were
    /// looked at.
    pub eligible: usize,
    /// How many distinct name fields those eligible lines hold. `0` when the
    /// verdict was reached before candidates were looked at.
    pub distinct: usize,
}

/// The verdict of [`select_line1`].
///
/// `#[non_exhaustive]`: an output type that may grow a variant.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Line1Verdict {
    /// The accepted read is not checksum-valid, or is TD1. Nothing was looked at.
    OutOfScope,
    /// The accepted name field already follows the name grammar.
    Kept,
    /// The accepted zone shows a symptom of a wrong physical line, so its name
    /// field is not what to change. See [`Line1Unresolved`].
    Unresolved(Line1Unresolved),
    /// The name field is ungrammatical and no other line is eligible.
    NoCandidate,
    /// The name field is ungrammatical and more than one distinct name field
    /// is eligible. Nothing is picked: no tie-break has been measured.
    Ambiguous,
    /// The name field is ungrammatical and exactly one distinct name field is
    /// eligible. Holds the accepted read with only `surname`, `given_names`
    /// and line 1 of `mrz_lines` replaced, and its `damaged_recovery` carried
    /// over.
    Proposed(MrzData),
}

/// Why [`select_line1`] left a zone [`Unresolved`](Line1Verdict::Unresolved):
/// which of `find_and_parse_with`'s wrong-physical-line checks flagged it.
///
/// `#[non_exhaustive]`: an output type that may grow a variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Line1Unresolved {
    /// Line 1's issuing state is not in the country table.
    IssuerUnresolved,
    /// Two of the zone's lines are near-identical.
    RepeatedLine,
    /// A digit sits in line 1's name field.
    DigitInNameField,
}

/// The public reason for a `crate::rank` flag.
fn unresolved_from(reason: FlagReason) -> Line1Unresolved {
    match reason {
        FlagReason::IssuerUnresolved => Line1Unresolved::IssuerUnresolved,
        FlagReason::RepeatedLine => Line1Unresolved::RepeatedLine,
        FlagReason::DigitInNameField => Line1Unresolved::DigitInNameField,
    }
}

/// Look in `text`, the OCR text `find_and_parse_with` read `accepted` from,
/// for a better line 1 than `accepted`'s, and report the verdict.
///
/// Pure and text-only: it reads no image and runs no OCR, and gives the same
/// answer for the same arguments. Pass the same `opts` that produced
/// `accepted`, since the eligibility check compares parses.
///
/// **Off by default, and unmeasured.** Nothing in `mrz` calls this. It exists
/// so a caller can A/B it against a control, as
/// [`ParseOptions::class_sweep`] does; how many correct names it would break is
/// exactly what has not been established.
///
/// # The rule
///
/// A filter and a uniqueness test, not a ranking, so it has no constant to
/// tune. In order, the first step that fires sets the verdict:
///
/// 1. **Out of scope.** `accepted` is not checksum-valid, or is TD1 (whose
///    names are on line 3, and whose line 1 carries check digits).
/// 2. **Kept.** `accepted`'s name field (line 1 from cell 5) is already
///    grammatical: once trailing `<` is stripped it is non-empty and matches
///    `[A-Z]+(<[A-Z]+)*(<<[A-Z]+(<[A-Z]+)*)?`. A grammatical name field is
///    never replaced.
/// 3. **Unresolved.** One of the checks that make `find_and_parse_with` prefer
///    another valid zone flags `accepted` as probably holding a wrong physical
///    line: an issuer outside the country table, a repeated line, or a digit in
///    the name field. Then the name field is not what to change, and nothing
///    is guessed. See [`Line1Unresolved`].
/// 4. **Candidates.** Each line of `text` (the same line walk
///    `find_and_parse_with` uses, each line normalized the way it does) is
///    eligible only if **all** of these hold:
///    - it is MRZ charset only, so a single `?` disqualifies it;
///    - it is exactly the format's width as read (44 for TD3 and MRV-A, 36 for
///      TD2 and MRV-B): nothing is padded, trimmed or repaired, so no
///      character is invented;
///    - its first five cells, the document code and the issuing state, are
///      byte-equal to `accepted`'s;
///    - its name field is grammatical;
///    - with `accepted`'s line 2 it parses checksum-valid and equals
///      `accepted` in every field except `surname`, `given_names` and line 1
///      of `mrz_lines`.
/// 5. **No candidate**, **ambiguous** (more than one distinct eligible name
///    field: nothing is picked, because no tie-break has been measured), or
///    **proposed** (exactly one).
///
/// It changes the name fields only: never the document code, the issuer, any
/// other field or line 2. Order-independent: reordering the lines of `text`
/// does not change the verdict.
///
/// ```
/// use mrz::{
///     find_and_parse_with, format_td3, select_line1, DateRole, Line1Verdict, MrzDate,
///     ParseOptions, RawDateField, Sex, Td3Fields, CURRENT_YY,
/// };
///
/// let date = |raw: &str, role| {
///     MrzDate::from_field(RawDateField::try_from(raw).unwrap(), role, CURRENT_YY)
/// };
///
/// let zone = format_td3(&Td3Fields {
///     issuing_country: "UTO".to_string(),
///     document_number: "AB123457".to_string(),
///     surname: "SPECIMEN".to_string(),
///     given_names: "TEST".to_string(),
///     nationality: "UTO".to_string(),
///     date_of_birth: date("800101", DateRole::Birth),
///     sex: Sex::Female,
///     date_of_expiry: date("301230", DateRole::Expiry),
///     ..Td3Fields::default()
/// });
///
/// // A well-formed name field is never replaced.
/// let opts = ParseOptions::default();
/// let accepted = find_and_parse_with(&zone, &opts).unwrap();
/// let selection = select_line1(&zone, &accepted, &opts);
/// assert_eq!(selection.verdict, Line1Verdict::Kept);
/// ```
pub fn select_line1(text: &str, accepted: &MrzData, opts: &ParseOptions) -> Line1Selection {
    let done = |verdict| Line1Selection {
        verdict,
        eligible: 0,
        distinct: 0,
    };

    // 1. Out of scope: only a valid TD3, TD2, MRV-A or MRV-B read is examined.
    let width = match line1_width(accepted.format) {
        Some(width) if accepted.valid() => width,
        _ => return done(Line1Verdict::OutOfScope),
    };
    let mut zone_lines = accepted.mrz_lines.lines();
    let (Some(incumbent_line1), Some(accepted_line2)) = (zone_lines.next(), zone_lines.next())
    else {
        return done(Line1Verdict::OutOfScope);
    };
    let (Some(incumbent_prefix), Some(incumbent_field)) = (
        incumbent_line1.get(..NAME_FIELD_START),
        incumbent_line1.get(NAME_FIELD_START..),
    ) else {
        return done(Line1Verdict::OutOfScope);
    };

    // 2. Kept: a grammatical incumbent is never replaced.
    if name_field_is_grammatical(incumbent_field) {
        return done(Line1Verdict::Kept);
    }

    // 3. Unresolved: the zone shows a wrong-physical-line symptom.
    if let Some(reason) = flag_reason(accepted) {
        return done(Line1Verdict::Unresolved(unresolved_from(reason)));
    }

    // 4. Candidates: exact-width lines from the same text, each eligible only
    // if it passes every check in `eligible_proposal`.
    let candidates: Vec<String> = candidate_lines(text)
        .iter()
        .map(|line| normalize_line(line))
        .collect();
    let mut eligible = 0;
    let mut distinct_fields: Vec<&str> = Vec::new();
    let mut proposals: Vec<MrzData> = Vec::new();
    for candidate in &candidates {
        let Some(proposal) = eligible_proposal(
            candidate,
            width,
            incumbent_prefix,
            accepted_line2,
            accepted,
            opts,
        ) else {
            continue;
        };
        eligible += 1;
        // `eligible_proposal` returned, so the candidate has a name field.
        let field = candidate.get(NAME_FIELD_START..).unwrap_or("");
        if !distinct_fields.contains(&field) {
            distinct_fields.push(field);
            proposals.push(proposal);
        }
    }

    // 5. No candidate, ambiguous, or proposed.
    let distinct = distinct_fields.len();
    let verdict = match proposals.len() {
        0 => Line1Verdict::NoCandidate,
        1 => match proposals.pop() {
            Some(proposal) => Line1Verdict::Proposed(proposal),
            None => Line1Verdict::NoCandidate,
        },
        _ => Line1Verdict::Ambiguous,
    };
    Line1Selection {
        verdict,
        eligible,
        distinct,
    }
}

/// The width of line 1 for the formats this rule covers; `None` for TD1.
fn line1_width(format: Format) -> Option<usize> {
    match format {
        Format::Td3 | Format::MrvA => Some(44),
        Format::Td2 | Format::MrvB => Some(36),
        Format::Td1 => None,
    }
}

/// The accepted read with `candidate` as its line 1, when `candidate`, a
/// normalized line, passes every eligibility check; `None` otherwise.
fn eligible_proposal(
    candidate: &str,
    width: usize,
    incumbent_prefix: &str,
    accepted_line2: &str,
    accepted: &MrzData,
    opts: &ParseOptions,
) -> Option<MrzData> {
    // Charset only, and exactly the format's width as read. The charset check
    // comes first: `?` and every other stray character disqualify a line, and
    // afterwards the line is ASCII, so `len` counts cells.
    if !is_mrz_charset(candidate) || candidate.len() != width {
        return None;
    }
    // The document code and the issuing state are the incumbent's, byte for byte.
    if candidate.get(..NAME_FIELD_START)? != incumbent_prefix {
        return None;
    }
    if !name_field_is_grammatical(candidate.get(NAME_FIELD_START..)?) {
        return None;
    }
    // With the accepted line 2 it must be a checksum-valid read that differs
    // from the accepted one in the name fields alone.
    let parsed: Result<MrzData, MrzError> = match accepted.format {
        Format::Td3 => parse_td3_with(candidate, accepted_line2, opts),
        Format::Td2 => parse_td2_with(candidate, accepted_line2, opts),
        Format::MrvA => parse_mrv_a_with(candidate, accepted_line2, opts),
        Format::MrvB => parse_mrv_b_with(candidate, accepted_line2, opts),
        Format::Td1 => return None,
    };
    let mut proposal = parsed.ok()?;
    if !proposal.valid() {
        return None;
    }
    // The parser has no notion of a search, so it always reports `false`: the
    // proposal keeps the incumbent's, which is where the line-2 read came from.
    proposal.damaged_recovery = accepted.damaged_recovery;
    if !differs_in_names_only(&proposal, accepted) {
        return None;
    }
    Some(proposal)
}

/// Whether `proposal` equals `accepted` in every field except `surname`,
/// `given_names` and line 1 of `mrz_lines`.
///
/// Compares a copy of `proposal` with those three taken from `accepted`, so a
/// field added to `MrzData` later is compared without this function being
/// touched. Line 2 is compared as well: it must be the accepted line 2 exactly.
fn differs_in_names_only(proposal: &MrzData, accepted: &MrzData) -> bool {
    let mut probe = proposal.clone();
    probe.surname = accepted.surname.clone();
    probe.given_names = accepted.given_names.clone();
    let line1_of_accepted = accepted.mrz_lines.lines().next().unwrap_or("");
    let line2_of_proposal = proposal.mrz_lines.lines().nth(1).unwrap_or("");
    probe.mrz_lines = format!("{line1_of_accepted}\n{line2_of_proposal}");
    probe == *accepted
}

/// Whether a name field (line 1 from cell 5 on) follows the name grammar:
/// once trailing filler is stripped, what remains is non-empty and matches
/// `[A-Z]+(<[A-Z]+)*(<<[A-Z]+(<[A-Z]+)*)?`.
///
/// Letters, then single `<` between words, with at most one `<<` (the
/// surname/given-names separator). Zero `<<` is allowed: a one-word surname
/// with no given names, and the collapsed separator some issuers print. A
/// digit, a `<<<` run, a leading `<` or a second `<<` is not grammatical.
///
/// A hand-written state machine over runs: the crate has no regex dependency
/// and stays zero-dependency.
fn name_field_is_grammatical(field: &str) -> bool {
    let bytes = field.trim_end_matches('<').as_bytes();
    let mut at = 0;
    let mut seen_double = false;
    loop {
        // A run of letters, at least one.
        let letters_start = at;
        while at < bytes.len() && bytes[at].is_ascii_uppercase() {
            at += 1;
        }
        if at == letters_start {
            return false;
        }
        if at == bytes.len() {
            return true;
        }
        // The separator run between two words: `<`, or `<<` once.
        let filler_start = at;
        while at < bytes.len() && bytes[at] == b'<' {
            at += 1;
        }
        match at - filler_start {
            1 => {}
            2 if !seen_double => seen_double = true,
            // A digit or any other character (zero fillers), a `<<<` run, or a
            // second `<<`.
            _ => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammatical_name_fields() {
        for field in [
            "SPECIMEN<<TEST<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<",
            "SPECIMEN<<TEST<ANNA<<<<<<<<",
            "DE<LA<CRUZ<<MARIA<<<<<<<<<<",
            // Zero `<<`: a lone surname, and a collapsed separator.
            "SPECIMEN<<<<<<<<<<<<<<<<<<<",
            "HASSAN<FARID<RAGUE<<<<<<<<<",
            "A",
        ] {
            assert!(name_field_is_grammatical(field), "{field}");
        }
    }

    #[test]
    fn ungrammatical_name_fields() {
        for (field, why) in [
            ("", "empty"),
            ("<<<<<<<<<<", "all filler"),
            ("<SPECIMEN<<TEST", "leading filler"),
            ("SPECIMEN<<<TEST", "a <<< run inside"),
            (
                "IBRAHIM<<ABDIAZIZ<MOHAMUD<<<<<<Z",
                "a <<< run, then a letter",
            ),
            ("SPECIMEN<<TEST<<ANNA", "a second <<"),
            ("SPEC1MEN<<TEST", "a digit"),
            ("SPECIMEN<<TEST<<<<<<<<1", "a digit after filler"),
            ("SPECIMEN<<TE?T", "a stray character"),
        ] {
            assert!(!name_field_is_grammatical(field), "{why}: {field}");
        }
    }

    const L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    const L1_OTHER_NAME: &str = "P<UTOERIKSSDN<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    const L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";
    /// `L2` with one expiry digit altered: still parses, no longer valid.
    const L2_OTHER_EXPIRY: &str = "L898902C36UTO7408122F1204169ZE184226B<<<<<10";

    fn td3(line1: &str, line2: &str) -> MrzData {
        parse_td3_with(line1, line2, &ParseOptions::default()).expect("parses")
    }

    /// Guard (e): only the names and line 1 may differ.
    #[test]
    fn a_different_name_alone_passes_the_equality_guard() {
        let accepted = td3(L1, L2);
        let proposal = td3(L1_OTHER_NAME, L2);
        assert_ne!(proposal.surname, accepted.surname);
        assert!(differs_in_names_only(&proposal, &accepted));
        // Reflexive: the same read differs in no field.
        assert!(differs_in_names_only(&accepted, &accepted));
    }

    #[test]
    fn a_different_non_name_field_fails_the_equality_guard() {
        let accepted = td3(L1, L2);

        // A different expiry date and check result, from a different line 2.
        let other_line2 = td3(L1_OTHER_NAME, L2_OTHER_EXPIRY);
        assert!(!differs_in_names_only(&other_line2, &accepted));

        // A different issuer, with the same names.
        let other_issuer = td3("P<UTPERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", L2);
        assert!(!differs_in_names_only(&other_issuer, &accepted));

        // A different document code.
        let other_code = td3("PDUTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", L2);
        assert!(!differs_in_names_only(&other_code, &accepted));

        // The flag for how the read was reached.
        let mut recovered = td3(L1_OTHER_NAME, L2);
        recovered.damaged_recovery = true;
        assert!(!differs_in_names_only(&recovered, &accepted));
    }

    #[test]
    fn the_widths_are_the_formats_line_widths() {
        assert_eq!(line1_width(Format::Td3), Some(44));
        assert_eq!(line1_width(Format::MrvA), Some(44));
        assert_eq!(line1_width(Format::Td2), Some(36));
        assert_eq!(line1_width(Format::MrvB), Some(36));
        assert_eq!(line1_width(Format::Td1), None);
    }
}
