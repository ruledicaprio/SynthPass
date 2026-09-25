//! #461: `damaged_pass`'s TD3 line-1 side (`td3_line1_variants`, shared by
//! `class_sweep_pass` and two of `damaged_pass`'s four TD3 shapes) used to
//! call only [`mrz`]'s ordinary `repair_td3_line1` on its `Keep` verdict,
//! never `repair_td3_line1_shifted` -- the shift/unshift repair the ordinary
//! TD3 scan in `find_and_parse_with` already applies. So a line whose
//! position-1 filler was dropped by the OCR retry loop, paired with a line 2
//! that needed `damaged_pass`'s own single-substitution repair (not merely
//! `repair_td3_line2`'s ordinary lookalike fixes) to validate, returned the
//! still left-shifted, wrong `document_type`/`issuing_country`/`surname`
//! outright: the ordinary scan never got a chance to try its own unshifted
//! candidate, because line 2 never validated there at all.
//!
//! `crates/mrz/tests/line1_prefix_shift.rs` pins the ordinary-scan case this
//! mirrors; `synthetic TD3 --seed 0 --count 100` seeds 18, 26, 37, 60, 66,
//! 72 and 86 (`TRACE_461.md`) are the measured real-shape occurrences this
//! reproduces synthetically.
//!
//! #469: chaining `repair_td3_line1_shifted` ahead of `rep1` in
//! `td3_line1_variants` (the fix above) made two of those seven seeds
//! (26, 86) regress from a silent wrong hit to a needless refusal --
//! `single()` saw the corrected unshifted reading *and* the still-shifted
//! plain one disagree, even though the plain one is never admissible under
//! `td3_line1_admissible` (its issuer never resolves, and its document code
//! is never a genuine `P<`/section-4.4 code). `td3_line1_variants` now drops
//! the inadmissible candidates before `single()` ever sees them, but only
//! when at least one admissible candidate survives -- see its own doc
//! comment. `damaged_pass_no_longer_returns_a_left_shifted_td3_line1` below
//! is exactly that seed-26/86 shape (`document_type`/`issuing_country` "PR"/
//! "USP" is inadmissible on both counts) and its outcome flips from
//! checksum-failed to a validated hit as a result.

mod support;

use mrz::{find_and_parse, format_td3, Format, Td3Fields};

/// Drop line 1's position-1 filler -- see `line1_prefix_shift.rs`'s
/// `drop_position_1` for why this is the exact shape the OCR retry loop
/// produces.
fn drop_position_1(line1: &str) -> String {
    let mut damaged = line1.to_string();
    damaged.remove(1);
    damaged
}

/// Swap line 2's leading document-number digit for its `CONFUSABLES`
/// lookalike letter. Every range `repair_td3_line2` corrects is elsewhere on
/// the line (the check digit at position 9, nationality, the two dates, the
/// personal-number field) -- the document number's own digits are never
/// touched by the ordinary repair -- so only `substituted`'s single-glyph
/// sweep, not `repair_td3_line2`, can recover it.
///
/// The fixture's document number must use distinct digits (never a repeated
/// one): a repeated digit lets `substituted`'s single-glyph sweep find a
/// *second*, different position whose own substitution happens to shift the
/// weighted checksum by the same amount the leading corruption did, so
/// several different-but-equally-checksum-valid document numbers come back --
/// a real, documented residue-class ambiguity
/// (`crate::repair::substitution_candidates`'s doc comment), not this test's
/// concern.
fn confuse_document_number_lead(line2: &str) -> String {
    let mut chars: Vec<char> = line2.chars().collect();
    assert_eq!(
        chars[0], '1',
        "sanity: fixture's document number must start with a 1, got {line2}"
    );
    chars[0] = 'I';
    chars.into_iter().collect()
}

/// Distinct digits, no repeats -- see `confuse_document_number_lead`'s doc
/// comment for why a repeated digit invites an unrelated residue-class
/// ambiguity this fixture must not exercise.
const DISTINCT_DIGIT_DOCUMENT_NUMBER: &str = "123456789";

fn genuine_rus_petrov() -> (String, String) {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "RUS".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "PETROV".to_string(),
        given_names: "IVAN".to_string(),
        nationality: "RUS".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Male,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    (l1.to_string(), l2.to_string())
}

/// The exact #461 mechanism: line 1's position-1 filler is dropped (so the
/// ordinary TD3 scan's own unshifted candidate never gets a chance -- its
/// line 2 side only ever tries `repair_td3_line2`'s ordinary repairs, which
/// cannot fix the substitution below), and line 2 separately needs
/// `damaged_pass`'s single-substitution repair.
///
/// `single()` (#440) still arbitrates: chaining `repair_td3_line1_shifted`
/// into `td3_line1_variants` means this shape offers *two* line-1 readings
/// against the recovered line 2 -- the corrected unshifted one and the
/// un-repaired left-shifted one ("PR"/"USP", a structurally valid but
/// meaningless document-type/issuer pair) -- and both parse and validate
/// structurally. They disagree on `document_type`, `issuing_country` and
/// `surname`, but #469's admissibility filter drops "PR"/"USP" before
/// `single()` ever sees it: its document code is not `P<` nor a genuine
/// section 4.4 code, and "USP" does not resolve as a country, so
/// `td3_line1_admissible` rejects it outright while the unshifted "P<RUS"
/// reading passes. `single()` is left with one candidate, not two, so this
/// now returns a validated hit rather than refusing -- #461's own bug was
/// this exact wrong reading being returned as if it were the only one,
/// silently, as a Tier-1 hit; #469 restores the correct one instead of
/// merely refusing both.
#[test]
fn damaged_pass_no_longer_returns_a_left_shifted_td3_line1() {
    let (l1, l2) = genuine_rus_petrov();
    assert!(l1.starts_with("P<RUS"), "sanity: emitted line 1 is {l1}");

    let damaged_l1 = drop_position_1(&l1);
    let damaged_l2 = confuse_document_number_lead(&l2);
    let text = format!("{damaged_l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .unwrap_or_else(|e| panic!("expected the corrected reading, got a hard error: {e:?}"));

    // The left-shifted wrong reading (`document_type` "PR", `issuing_country`
    // "USP", `surname` starting from the wrong offset) that #461 reported as
    // a silent hit must never surface, whether or not it now validates.
    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P", "must not stay left-shifted");
    assert_eq!(data.issuing_country, "RUS", "must not stay left-shifted");
    assert_eq!(data.surname, "PETROV", "must not stay left-shifted");
    // #469: with the inadmissible "PR"/"USP" candidate dropped before
    // `single()`, only the corrected reading remains, so this is now a full
    // validated hit rather than a checksum-failed fallback.
    assert!(
        data.valid(),
        "#469's admissibility filter should leave only the corrected \
         reading, which fully validates"
    );
}

/// #446's genuine-code guard must still hold on this path: a real Part 4
/// section 4.4 second-letter code (`PP` + `NGA`, from `line1_prefix_shift.rs`'s
/// `genuine_table_code_whose_unshift_collides_is_kept`) whose position-1
/// filler was never dropped in the first place must survive the damaged
/// pass unchanged when its line 2 separately needs a substitution repair --
/// `td3_line1_is_genuine_table_code` must keep gating
/// `repair_td3_line1_shifted`'s extra candidates here exactly as it does for
/// the ordinary scan, so `td3_line1_variants` offers only one *distinct*
/// line-1 reading (the shifted and plain repairs coincide for a genuine
/// code) and the unanimity gate has nothing to refuse.
#[test]
fn a_genuine_table_code_survives_the_damaged_pass_line2_substitution() {
    let mrz = format_td3(&Td3Fields {
        document_code: "PP".to_string(),
        issuing_country: "NGA".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "OKAFOR".to_string(),
        given_names: "ADA".to_string(),
        nationality: "NGA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert!(l1.starts_with("PPNGA"), "sanity: emitted line 1 is {l1}");

    let damaged_l2 = confuse_document_number_lead(l2);
    let text = format!("{l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .expect("a genuine table code plus a line-2 substitution must still recover");
    assert_eq!(data.format, Format::Td3);
    assert_eq!(
        data.document_type, "PP",
        "a genuine section 4.4 code must survive the damaged pass unchanged"
    );
    assert_eq!(data.issuing_country, "NGA");
    assert_eq!(data.surname, "OKAFOR");
    assert_eq!(data.document_number, DISTINCT_DIGIT_DOCUMENT_NUMBER);
    assert!(
        data.valid(),
        "the line-2 substitution must be fully recovered"
    );
}

/// #469's filter only drops candidates whose *country or document code*
/// never resolves -- it must never paper over a genuine disagreement on
/// another field. A given-names field ending in a lone `L` right before the
/// name-padding filler run is exactly that: `repair_td3_line1`'s ordinary
/// `defiller` leaves a single K/L touching filler alone (its "at least 3 of
/// a run of 4" guard doesn't fire), so the *plain* candidate keeps "IVANL";
/// `variants`' `aggressive_defiller` last-resort form folds that same `L`
/// into the filler run instead, so a *second* candidate reads "IVAN". Both
/// have the same document code (`P<`) and issuing country (`RUS`), so both
/// are `td3_line1_admissible` -- #469's filter keeps both, exactly as before
/// this change, and `single()`'s unanimity gate still refuses on the
/// differing `given_names`. This is the synthetic shape behind seed 66's
/// real `IURII`/`IURIL` disagreement (`TRACE_461.md`): the proposal must
/// leave a genuine same-country, same-code name disagreement refused.
#[test]
fn two_admissible_same_country_readings_that_disagree_on_name_still_refuse() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "RUS".to_string(),
        document_number: DISTINCT_DIGIT_DOCUMENT_NUMBER.to_string(),
        surname: "PETROV".to_string(),
        given_names: "IVANL".to_string(),
        nationality: "RUS".to_string(),
        date_of_birth: support::birth("800101"),
        sex: mrz::Sex::Male,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert!(l1.starts_with("P<RUS"), "sanity: emitted line 1 is {l1}");
    assert!(
        l1.contains("IVANL<"),
        "sanity: given name must sit directly against the filler run, got {l1}"
    );

    let damaged_l2 = confuse_document_number_lead(l2);
    let text = format!("{l1}\n{damaged_l2}");

    let data = find_and_parse(&text)
        .unwrap_or_else(|e| panic!("expected a checksum-failed fallback, got a hard error: {e:?}"));

    // Both readings share `document_type`/`issuing_country`, so #469's
    // admissibility filter cannot distinguish them -- unanimity is the only
    // thing left to refuse this, and it must still do so.
    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.issuing_country, "RUS");
    assert!(
        !data.valid(),
        "two admissible readings disagreeing on given_names (\"IVANL\" vs \
         \"IVAN\") must still be refused by single()'s unanimity gate"
    );
}
