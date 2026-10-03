//! Regressions for `SequenceCompleteness` (`crates/mrz/src/lib.rs`) —
//! `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 5: one type spanning both
//! arms of a `find_and_parse` result.

use mrz::{find_and_parse, parse_td3, DateCompleteness, Format, SequenceCompleteness};

const TD3_L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
const TD3_L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

/// What `TD3_L2` must report, derived by hand with the 7-3-1 rule (letters
/// A=10 .. Z=35, filler 0, weights 7, 3, 1 repeating), not by this crate:
///
/// - document number `L898902C3`: 147+24+9+56+27+0+14+36+3 = 316, so 6: the
///   printed `6`, `Some(true)`;
/// - date of birth `740812`: 49+12+0+56+3+2 = 122, so 2: printed `2`;
/// - date of expiry `120415`: 7+6+0+28+3+5 = 49, so 9: printed `9`;
/// - personal number `ZE184226B<<<<<`: 245+42+1+56+12+2+14+18+11 = 401, so 1:
///   printed `1`;
/// - composite over `L898902C36` `7408122` `1204159` `ZE184226B<<<<<1`
///   (39 cells): 880, so 0: printed `0`.
///
/// The date of birth `740812` is six ASCII digits, so it is `Complete`.
fn assert_complete_and_all_checks_pass(completeness: &SequenceCompleteness) {
    let SequenceCompleteness::Complete {
        checks,
        date_of_birth,
    } = completeness
    else {
        panic!("expected Complete, got {completeness:?}");
    };
    assert_eq!(checks.document_number, Some(true));
    assert_eq!(checks.date_of_birth, Some(true));
    assert_eq!(checks.date_of_expiry, Some(true));
    assert_eq!(checks.personal_number, Some(true));
    assert_eq!(checks.composite, Some(true));
    assert_eq!(*date_of_birth, DateCompleteness::Complete);
}

#[test]
fn a_fully_valid_read_reports_complete_with_matching_checks() {
    let data = parse_td3(TD3_L1, TD3_L2).unwrap();
    assert_complete_and_all_checks_pass(&data.sequence_completeness());
}

#[test]
fn a_checksum_invalid_read_is_still_complete_not_partial() {
    // Every line was found — the record is structurally complete even
    // though a check digit disagrees. `SequenceCompleteness` and
    // `MrzData::valid()` answer different questions.
    let tampered = TD3_L2.replacen("740812", "750812", 1);
    let data = parse_td3(TD3_L1, &tampered).unwrap();
    assert!(!data.valid());
    match data.sequence_completeness() {
        SequenceCompleteness::Complete { checks, .. } => {
            assert_eq!(checks.date_of_birth, Some(false));
            assert_eq!(checks.composite, Some(false));
        }
        other => panic!("expected Complete, got {other:?}"),
    }
}

#[test]
fn from_parse_result_maps_ok_to_complete() {
    let result = parse_td3(TD3_L1, TD3_L2);
    let completeness = SequenceCompleteness::from_parse_result(&result).expect("a hit maps");
    // The literal from `assert_complete_and_all_checks_pass`, not the same
    // call made a second time.
    assert_complete_and_all_checks_pass(&completeness);
}

#[test]
fn from_parse_result_maps_incomplete_sequence_to_partial() {
    let result = find_and_parse(TD3_L1); // line 1 alone, no companion
    let completeness =
        SequenceCompleteness::from_parse_result(&result).expect("an IncompleteSequence maps");
    assert_eq!(
        completeness,
        SequenceCompleteness::Partial {
            format: Format::Td3,
            lines_found: 1,
            lines_expected: 2,
        }
    );
}

#[test]
fn from_parse_result_is_none_for_not_found() {
    let result = find_and_parse("just some ordinary prose, no MRZ in sight");
    assert_eq!(SequenceCompleteness::from_parse_result(&result), None);
}

#[test]
fn date_of_birth_completeness_flows_through_unmodified() {
    // All-filler date of birth: a legitimately unknown field, not garbage —
    // see lib.rs's own td3_all_filler_dob_is_a_valid_field_not_a_bad_read.
    let l2 = format!("{}{}{}", &TD3_L2[0..13], "<<<<<<0", &TD3_L2[20..]);
    let data = parse_td3(TD3_L1, &l2).unwrap();
    match data.sequence_completeness() {
        SequenceCompleteness::Complete { date_of_birth, .. } => {
            assert_eq!(date_of_birth, DateCompleteness::Unknown);
        }
        other => panic!("expected Complete, got {other:?}"),
    }
}
