//! Line-level checks that rank a checksum-valid zone below an alternative.
//!
//! A checksum-valid zone can still hold a *wrong physical line*: the scan pairs
//! lines by position, and a line that enters none of a format's check digits can
//! be any line the OCR produced. TD3, MRV-A and MRV-B carry no check digit on
//! line 1 at all, and TD1's line 1 is covered only over its document number and
//! the composite's cells 5..30. So a second reading of line 2, a visual-zone
//! header, or a name line can stand in for line 1 and the zone still validates.
//!
//! [`flagged`] reports whether a valid zone shows one of three such symptoms.
//! `find_and_parse_with` keeps looking past a flagged zone and returns it only
//! when no unflagged valid zone exists, so the checks can only *re-rank*: they
//! never turn a valid parse into an invalid one and never refuse.
//!
//! The three checks are deliberately narrow, and each is a symptom that a
//! correctly read zone does not show on the corpora measured (see the constants
//! below), not a definition of a well-formed zone.

use crate::{Format, MrzData};

/// The similarity at or above which two lines of one zone count as the same
/// physical line read twice (see [`similarity`]).
///
/// Measured over the real-specimen corpus (139 accepted zones) and the
/// synthetic corpus (five formats, 100 seeds each, clean profile): the most
/// similar pair of lines in a correctly read zone scores at most 0.30 on real
/// specimens and at most 0.28 on synthetic ones, while every zone that holds a
/// line twice scores at least 0.69. The threshold sits in that gap, so neither
/// side is close to it.
const REPEATED_LINE_SIMILARITY: f64 = 0.6;

/// The first cell of line 1 that belongs to the name field on the two-line
/// formats: cells 0..2 are the document code, 2..5 the issuing state.
const NAME_FIELD_START: usize = 5;

/// Whether `data`, a checksum-valid zone, shows a symptom of holding a wrong
/// physical line. True when any of these holds:
///
/// - **The issuer does not resolve** ([`issuer_unresolved`]).
/// - **A line repeats another** ([`repeats_a_line`]).
/// - **A digit sits in a two-line format's name field**
///   ([`digit_in_name_field`]).
pub(crate) fn flagged(data: &MrzData) -> bool {
    flag_reason(data).is_some()
}

/// Which of [`flagged`]'s checks fired.
///
/// The variants are in the order the checks run, and the first to fire is the
/// one reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FlagReason {
    /// [`issuer_unresolved`].
    IssuerUnresolved,
    /// [`repeats_a_line`].
    RepeatedLine,
    /// [`digit_in_name_field`].
    DigitInNameField,
}

/// [`flagged`] with the reason: `None` when no check fires, which is exactly
/// when `flagged` is false. `flagged` is defined by this function, so the two
/// cannot drift.
pub(crate) fn flag_reason(data: &MrzData) -> Option<FlagReason> {
    let lines: Vec<&str> = data.mrz_lines.lines().collect();
    let &line1 = lines.first()?;
    if issuer_unresolved(line1) {
        Some(FlagReason::IssuerUnresolved)
    } else if repeats_a_line(&lines) {
        Some(FlagReason::RepeatedLine)
    } else if digit_in_name_field(data.format, line1) {
        Some(FlagReason::DigitInNameField)
    } else {
        None
    }
}

/// Line 1's issuing-state cells (2..5) do not resolve in the country registry.
///
/// The raw cells go through the same `country_resolves` the parser's line-1
/// admissibility (`line1_admissible`) uses, but without its letterizing: that
/// step exists because `line1_admissible` chooses between repair candidates,
/// while here a digit in the issuer slot of a finished zone is itself a symptom.
///
/// A zone whose issuer is not in the registry at all (Somaliland) is flagged
/// too, and is still returned when nothing better exists.
fn issuer_unresolved(line1: &str) -> bool {
    line1
        .get(2..5)
        .is_some_and(|issuer| !crate::parser::country_resolves(issuer))
}

/// Two of the zone's lines are near-identical: their [`similarity`] is at least
/// [`REPEATED_LINE_SIMILARITY`]. The lines of a real zone hold different fields.
fn repeats_a_line(lines: &[&str]) -> bool {
    lines.iter().enumerate().any(|(i, a)| {
        lines[i + 1..]
            .iter()
            .any(|b| similarity(a, b) >= REPEATED_LINE_SIMILARITY)
    })
}

/// A two-line format's line 1 has an ASCII digit at cell 5 or later, where the
/// name field is. A name is letters and fillers; a digit there means the line
/// is a visual-zone line, not the machine-readable one. TD1 is exempt: its
/// line 1 holds the document number and optional data after cell 5.
fn digit_in_name_field(format: Format, line1: &str) -> bool {
    format != Format::Td1
        && line1
            .bytes()
            .skip(NAME_FIELD_START)
            .any(|c| c.is_ascii_digit())
}

/// The character a lookalike folds to, so that `O` and `0`, `I` and `1`, ...
/// compare equal. OCR reads the same glyph as either, so two readings of one
/// line differ by these far more often than by anything else.
fn fold_lookalike(c: u8) -> u8 {
    match c {
        b'O' | b'Q' | b'D' => b'0',
        b'I' | b'L' => b'1',
        b'Z' => b'2',
        b'S' => b'5',
        b'B' => b'8',
        b'G' => b'6',
        b'T' => b'7',
        other => other,
    }
}

/// `line` without any filler, with each lookalike folded.
fn comparable(line: &str) -> Vec<u8> {
    line.bytes()
        .filter(|&c| c != b'<')
        .map(fold_lookalike)
        .collect()
}

/// How alike two lines are: `1 - Levenshtein / longer length`, over the lines
/// with every `<` removed and the lookalikes folded. `0.0` when either line is
/// all filler, so an empty line resembles nothing.
fn similarity(a: &str, b: &str) -> f64 {
    let a = comparable(a);
    let b = comparable(b);
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, &ca) in a.iter().enumerate() {
        let mut current = Vec::with_capacity(b.len() + 1);
        current.push(i + 1);
        for (j, &cb) in b.iter().enumerate() {
            let substitution = previous[j] + usize::from(ca != cb);
            current.push(substitution.min(previous[j + 1] + 1).min(current[j] + 1));
        }
        previous = current;
    }
    1.0 - previous[b.len()] as f64 / a.len().max(b.len()) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filler_is_removed_before_comparing() {
        assert_eq!(comparable("AM<<C<"), b"AMC".to_vec());
        assert_eq!(comparable("<<<<"), Vec::<u8>::new());
        assert_eq!(comparable("A<<A"), comparable("AA"));
        assert!((similarity("A<<<<<A", "AA") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn lookalikes_fold_to_one_character() {
        for (from, to) in [
            (b'O', b'0'),
            (b'Q', b'0'),
            (b'D', b'0'),
            (b'I', b'1'),
            (b'L', b'1'),
            (b'Z', b'2'),
            (b'S', b'5'),
            (b'B', b'8'),
            (b'G', b'6'),
            (b'T', b'7'),
        ] {
            assert_eq!(fold_lookalike(from), to, "{}", from as char);
        }
        // Characters that are not lookalikes are unchanged.
        for c in [b'A', b'M', b'0', b'5', b'<'] {
            assert_eq!(fold_lookalike(c), c);
        }
        assert!((similarity("OQDILZSBGT", "0001125867") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn similarity_is_one_minus_distance_over_longer_length() {
        assert!((similarity("ABCDE", "ABCDE") - 1.0).abs() < f64::EPSILON);
        // One substitution in five cells.
        assert!((similarity("ABCDE", "ABCDF") - 0.8).abs() < 1e-12);
        // Length differs: two insertions over the longer length of five.
        assert!((similarity("ABC", "ABCDE") - 0.6).abs() < 1e-12);
        // Nothing in common.
        assert!(similarity("AAAA", "MMMM").abs() < f64::EPSILON);
        // An all-filler line resembles nothing, not even itself.
        assert!(similarity("<<<<", "<<<<").abs() < f64::EPSILON);
        assert!(similarity("ABC", "<<<").abs() < f64::EPSILON);
    }

    #[test]
    fn similarity_threshold_has_a_case_on_each_side() {
        // Exactly the threshold: two edits over five cells = 0.6, flagged.
        let at = ["ABCDE", "ABCMM"];
        assert!(similarity(at[0], at[1]) >= REPEATED_LINE_SIMILARITY);
        assert!(repeats_a_line(&at));
        // Just below: three edits over five cells = 0.4, not flagged.
        let below = ["ABCDE", "ABMMM"];
        assert!(similarity(below[0], below[1]) < REPEATED_LINE_SIMILARITY);
        assert!(!repeats_a_line(&below));
    }

    #[test]
    fn a_line_read_twice_is_a_repeat_whatever_the_slots() {
        let twice = [
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "P000000005UTO7408122F1204159<<<<<<<<<<<<<<04",
            "P0OOOO0005UTO7408122F1204159<<<<<<<<<<<<<<04",
        ];
        assert!(repeats_a_line(&twice));
        assert!(!repeats_a_line(&twice[..2]));
        assert!(!repeats_a_line(&twice[..1]));
        assert!(!repeats_a_line(&[]));
    }

    #[test]
    fn issuer_check_flags_only_an_unresolved_issuer() {
        assert!(!issuer_unresolved("P<UTOERIKSSON<<ANNA"));
        // Germany's code is filler-padded: `D<<`.
        assert!(!issuer_unresolved("P<D<<MUSTERMANN<<ERIKA"));
        assert!(issuer_unresolved("VISERINGVISASNE987654321"));
        assert!(issuer_unresolved("AL<EKS<<<<<<"));
        // A line too short to hold an issuer is not this check's business.
        assert!(!issuer_unresolved("P<U"));
        assert!(!issuer_unresolved(""));
    }

    #[test]
    fn name_field_check_flags_a_digit_from_cell_5_on_two_line_formats() {
        let visual = "P<UTO0ERIKSSON<<ANNA";
        for format in [Format::Td3, Format::Td2, Format::MrvA, Format::MrvB] {
            assert!(digit_in_name_field(format, visual), "{format:?}");
            assert!(
                !digit_in_name_field(format, "P<UTOERIKSSON<<ANNA"),
                "{format:?}"
            );
        }
        // TD1's line 1 carries the document number there.
        assert!(!digit_in_name_field(
            Format::Td1,
            "I<UTOD231458907<<<<<<<<<<<<<<<"
        ));
        // A digit inside the code/issuer cells is the other checks' business.
        assert!(!digit_in_name_field(Format::Td3, "P1UTOERIKSSON"));
        // Cell 5 is the first name cell.
        assert!(digit_in_name_field(Format::Td3, "P<UTO1RIKSSON"));
        assert!(!digit_in_name_field(Format::Td3, "P<UT1ERIKSSON"));
    }
}
