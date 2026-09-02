//! Regressions for `unshift_line1_prefix` (`crates/mrz/src/parser.rs`),
//! wired into `repair_td3_line1`/`repair_mrv_line1`.
//!
//! `synthpass-bench --dump-ocr` (`knowledge/ROADMAP.md`'s M6 note) showed
//! OCR dropping line 1's position-1 filler outright — `P<BRAESKANDARI...`
//! read as `PBRAESKANDARI...` — for TD3 and MRV-B alike, the same mechanism
//! `crates/mrz/tests/td1_line_gap.rs`'s
//! `a_dropped_line_1_filler_position_is_recovered` already pins for TD1.
//! Unlike TD1, none of TD3/MRV-A/MRV-B carry a check digit on line 1, so the
//! shift never fails a Tier-1 hit — it silently corrupts `document_type`/
//! `issuing_country`/the name behind a passing read instead, which is why
//! `synthpass-bench`'s per-field CER (not a hit-rate drop) is what surfaced
//! it.
//!
//! **Fixtures are emitted, not transcribed**, same discipline
//! `tests/td1_line_gap.rs` and `tests/name_separator_collapse.rs` follow.

use mrz::{
    find_and_parse, format_mrv_a, format_mrv_b, format_td2, format_td3, Format, MrvAFields,
    MrvBFields, Td2Fields, Td3Fields,
};

/// Drop line 1's position-1 filler, reproducing the measured OCR failure —
/// the line is now one character short, the same shape `find_and_parse`'s
/// `variants()`/`fit_length` machinery has to recover from in production
/// (it pads the deficit back at the *end*, which is exactly what leaves the
/// prefix shifted unless `unshift_line1_prefix` corrects it).
fn drop_position_1(line1: &str) -> String {
    let mut damaged = line1.to_string();
    damaged.remove(1);
    damaged
}

#[test]
fn td3_recovers_document_type_and_issuing_country_after_a_dropped_position_1_filler() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("a dropped position-1 filler must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid(), "line 2's real check digits are untouched");
}

#[test]
fn mrv_a_recovers_document_type_and_issuing_country_after_a_dropped_position_1_filler() {
    let mrz = format_mrv_a(&MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_a emits two lines");

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("a dropped position-1 filler must still parse");

    assert_eq!(data.format, Format::MrvA);
    assert_eq!(data.document_type, "V");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

#[test]
fn mrv_b_recovers_document_type_and_issuing_country_after_a_dropped_position_1_filler() {
    let mrz = format_mrv_b(&MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E0000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_mrv_b emits two lines");

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data = find_and_parse(&text).expect("a dropped position-1 filler must still parse");

    assert_eq!(data.format, Format::MrvB);
    assert_eq!(data.document_type, "V");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

/// The guard case: an undamaged zone (position 1 genuinely already `<`)
/// must round-trip unchanged — `unshift_line1_prefix` must never fire when
/// there's nothing to unshift.
#[test]
fn an_undamaged_td3_zone_is_not_spuriously_unshifted() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("emitted TD3 parses");
    assert_eq!(data.document_type, "P");
    assert_eq!(data.issuing_country, "BRA");
    assert!(data.valid());
}

/// TD2 shares TD1's ICAO ID-document-family code space and can legitimately
/// carry a genuine two-real-letter document code — `unshift_line1_prefix`
/// is deliberately *not* wired into `repair_td2_line1` (see its doc
/// comment) precisely so a fixture like this one is never corrupted by a
/// future change that naively extends the TD3/MRV-A/MRV-B fix to TD2 too.
/// This pins the current, correct behavior as a regression guard.
#[test]
fn td2_with_a_genuine_two_letter_document_code_is_unaffected() {
    let mrz = format_td2(&Td2Fields {
        document_code: "IP".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: "800101".to_string(),
        sex: "F".to_string(),
        date_of_expiry: "301230".to_string(),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td2 emits two lines");

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("emitted TD2 parses");
    assert_eq!(data.format, Format::Td2);
    assert_eq!(
        data.document_type, "IP",
        "a genuine two-letter document code must survive untouched"
    );
    assert_eq!(data.issuing_country, "BRA");
}

/// Germany's legacy MRZ issuing-state code is the single letter `D`, which the
/// MRZ pads to `D<<`. Recovering a dropped position-1 filler on such a document
/// requires the repair gate to look the state up *after* trimming that padding.
///
/// It did not. All three gates passed the raw 3-byte slice to `country_name`,
/// which is exact string equality against a table storing `("D", "Germany")`,
/// so `country_name("D<<")` was `None` and the correct repair was produced and
/// then discarded. Nothing caught it: the synthetic generator only ever emits
/// `DEU`, and no corpus specimen exercised the path until
/// `Germany_Passport_Specimen_P0_D00_2018_mrz.webp` — whose line 1 reads
/// `P<D<<HEINKEL<<REYNALD` — was read against the manifest.
#[test]
fn td3_recovers_a_filler_padded_single_letter_issuing_state() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "D".to_string(),
        document_number: "G28P5FV81".to_string(),
        surname: "HEINKEL".to_string(),
        given_names: "REYNALD".to_string(),
        nationality: "D".to_string(),
        date_of_birth: "980628".to_string(),
        sex: "M".to_string(),
        date_of_expiry: "281201".to_string(),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert!(
        l1.starts_with("P<D<<"),
        "the emitter must filler-pad a single-letter state: {l1}"
    );

    let damaged = drop_position_1(l1);
    let text = format!("{damaged}\n{l2}");
    let data =
        find_and_parse(&text).expect("a dropped position-1 filler must still parse for `D<<`");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "D",
        "the repair gate must resolve `D<<` to Germany, not discard the repair"
    );
    assert_eq!(data.issuing_country_name(), Some("Germany"));
    assert!(data.valid(), "line 2's real check digits are untouched");
}

/// The trimming above must not make an all-filler state resolve: `<<<` trims to
/// the empty string, which is not in the table, so a genuinely damaged zone is
/// still left alone rather than "repaired" into nonsense.
#[test]
fn an_all_filler_issuing_state_still_does_not_resolve() {
    assert_eq!(mrz::country_name(""), None);
    assert_eq!(mrz::country_name("<<<"), None);
}
