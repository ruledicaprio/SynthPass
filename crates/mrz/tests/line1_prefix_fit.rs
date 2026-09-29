//! #574 (PR 1, "D"): line 1 of every two-line format (TD3/TD2/MRV-A/MRV-B)
//! carries no check digit (Doc 9303 Part 4 §4.2.1 and its TD1/TD2/MRV
//! analogues), so nothing arbitrates a wrong candidate the way a check digit
//! arbitrates line 2 — the first candidate that pairs with a checksum-valid
//! line 2 wins outright. `fit_length` (`crate::checksum`) used to fit a
//! short line 1 by inflating whichever `<` run is longest, and
//! `longest_filler_run` keeps the first of equal-length runs. When every run
//! on the read line is one cell long — the shape OCR noise inside a filler
//! run produces (`crates/mrz/tests/line1_prefix_shift.rs`'s
//! `drop_position_1` is the single-cell version of the same defect) — the
//! one-cell document-code filler at index 1 is that first run, so the
//! padding lands there and pushes the issuing-state slot's own bytes into
//! the name field. The accepted issuing country then reads as `<<<`.
//!
//! `line1_variants`/`fit_line1_length` (`crate::checksum`) fix this: on a
//! short line, a `<` run that starts before index 5 — where the name field
//! starts — is never inflated; the tail is padded instead. Every two-line
//! format's line 1 now routes through this helper; every other caller
//! (checked lines, TD1's three lines) is unaffected.
//!
//! The two pinned specimens below are public corpus entries; the strings are
//! the exact OCR text the real-specimen run recorded on `main` before this
//! fix, transcribed rather than read from the corpus so this file runs in a
//! clone with no images synced — the same discipline
//! `crates/mrz/tests/line1_nonconformance.rs` follows.

mod support;

use mrz::{
    find_and_parse, find_and_parse_with, format_mrv_a, format_mrv_b, format_td1, format_td2,
    format_td3, Format, MrvAFields, MrvBFields, ParseOptions, Sex, Td1Fields, Td2Fields, Td3Fields,
};

/// Canada's 2013 specimen
/// (`samples/passports/Canada_Passport_Specimen_P0_CAN_2013_mrz.jpg`).
///
/// OCR read the name-field filler run as a wall of `K`s interrupted by
/// scattered lone `<`s, so every `<` run on the line — including the
/// document-code filler at index 1 — is exactly one cell long. Before this
/// fix, `longest_filler_run`'s tie rule picked that first, index-1 run, and
/// inflating it pushed `CAN` out of the issuing-state slot: the zone read
/// back as `P<<<<<<<<CANMARTIN<SARA<<<...`, issuer `<<<`.
///
/// The printed name is `MARTIN<<SARAH` (the print has a trailing `H` and a
/// real `<<` separator OCR did not recover here); the names below are
/// pinned to whatever the tail-padded line actually parses to, not to the
/// print, and are expected to stay wrong — this fix is about the issuing
/// country and document code, not the name field's own OCR noise.
#[test]
fn canada_2013_short_line1_no_longer_corrupts_the_issuer() {
    let l1 = "P<CANMARTIN<SARAKKKKKKKKKKK<K<K<KK<K<";
    assert_eq!(l1.len(), 37);
    let l2 = "ZE001355<3CAN8501019F2301147<<<<<<<<<<<<<<00";

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "CAN",
        "the issuing state must survive, not become the unresolved <<<"
    );
    assert_eq!(
        data.issuing_country_name(),
        Some("Canada"),
        "the recovered issuer must actually resolve"
    );
    // Pinned as read, not as printed — see the doc comment above.
    assert_eq!(data.surname, "MARTIN");
    assert_eq!(data.given_names, "SARA");
    assert!(data.valid(), "line 2's real check digits are untouched");
}

/// Djibouti's 2017 specimen
/// (`samples/passports/Djibouti_Passport_Specimen_P0_DJI_2017_mrz_partly_censored.jpg`).
///
/// Same mechanism as the Canada specimen above, on a different document:
/// OCR noise collapses every `<` run on line 1 to one cell, so the
/// document-code filler at index 1 used to win the tie and the issuer read
/// back as `<<<` instead of `DJI`. The names are pinned as read; line 2's
/// OCR text includes a stray space (from the source specimen), which
/// `normalize_line` strips before parsing.
#[test]
fn djibouti_2017_short_line1_no_longer_corrupts_the_issuer() {
    let l1 = "P<DJIHAMZAGOUMANEHCAMALFH<CCCCCCC<CCC<CKC";
    assert_eq!(l1.len(), 41);
    let l2 = "16RE991586D J19808232M2204112<<<<<<<<<<<<<<08";

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "DJI",
        "the issuing state must survive, not become the unresolved <<<"
    );
    assert_eq!(data.issuing_country_name(), Some("Djibouti"));
    // Pinned as read, not as printed — see the doc comment above.
    assert_eq!(data.surname, "HAMZAGOUMANEHCAMALFH");
    assert_eq!(data.given_names, "CCCCCCC CCC CKC");
    assert!(data.valid(), "line 2's real check digits are untouched");
}

/// The fix is unconditional on whether the recovered issuer happens to
/// resolve to a real country: the exact Canada mechanism above, but with an
/// issuing-state slot (`XYZ`) that is not in the country table at all. The
/// structural "never inflate a run before index 5" rule in
/// `crate::checksum::fit_line1_length` has no access to the country table —
/// it must behave identically whether or not the tail-padded issuer turns
/// out to be real.
#[test]
fn an_unresolvable_issuer_is_read_literally_not_inflated() {
    let l1 = "P<XYZMARTIN<SARAKKKKKKKKKKK<K<K<KK<K<";
    let l2 = "ZE001355<3CAN8501019F2301147<<<<<<<<<<<<<<00";

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "XYZ",
        "the tail-padded issuer must be read literally, not inflated into filler"
    );
    assert_eq!(
        data.issuing_country_name(),
        None,
        "XYZ is not a real issuing state either way"
    );
    assert!(data.valid());
}

/// A `format_td3`-built TD3 zone whose issuing state is Germany's legacy
/// single-letter code, letter-padded to `D<<` (Part 3 §5's filler rule),
/// with its trailing name-field padding entirely dropped (the realistic
/// shape `crates/mrz/tests/line1_prefix_shift.rs`'s pinned Germany specimen
/// and this fixture's own `emitted line 1 starts with P<D<<` sanity check
/// both document).
///
/// `D<<`'s own filler run (index 3, length 2) ties with the name field's
/// `<<` separator (also length 2); the tie rule picks the issuing-state
/// one first. This is the "German-style `D<<` issuer" witness: the fillers
/// inside the issuing-state slot must never be inflated, and the issuer
/// must stay `D<<` (letterized down to `D`).
#[test]
fn a_germany_style_d_filler_issuer_survives_a_short_line1() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "D".to_string(),
        document_number: "G28P5FV81".to_string(),
        surname: "SCHWARZENEGGER".to_string(),
        given_names: "REYNALDALEXANDER".to_string(),
        nationality: "D".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Male,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert!(
        l1.starts_with("P<D<<"),
        "sanity: the emitter must filler-pad a single-letter state: {l1}"
    );

    // OCR dropping every trailing filler cell wholesale, exactly as measured
    // for TD3 name lines on low-resolution scans (see `crate::checksum::variants`'s
    // doc comment).
    let short_l1 = l1.trim_end_matches('<');
    assert_eq!(short_l1, "P<D<<SCHWARZENEGGER<<REYNALDALEXANDER");

    let data =
        find_and_parse(&format!("{short_l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "D",
        "the D<< filler slot must never be inflated"
    );
    assert_eq!(data.issuing_country_name(), Some("Germany"));
    assert_eq!(data.surname, "SCHWARZENEGGER");
    assert_eq!(data.given_names, "REYNALDALEXANDER");
    assert!(data.valid(), "line 2's real check digits are untouched");
}

/// A genuine Part 4 §4.4 two-letter document code (`PD`) on a short TD3
/// line 1 — regression pin for point 3 of the fix: when the longest `<` run
/// is *not* before index 5 (here the only run is the name separator, well
/// past it), fitting must be byte-identical to before this change. Uses
/// `BRA`, not `ZAF`, as the issuer: `PD`+`ZAF` is one of the #445 collision
/// pairs `crates/mrz/tests/line1_prefix_shift.rs` pins separately, and is
/// not this test's concern.
#[test]
fn a_genuine_td3_two_letter_code_on_a_short_line1_is_unaffected() {
    let mrz = format_td3(&Td3Fields {
        document_code: "PD".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "OKAFORCHUKWUDI".to_string(),
        given_names: "ADAEZINWA".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    let short_l1 = l1.trim_end_matches('<');
    assert_eq!(short_l1, "PDBRAOKAFORCHUKWUDI<<ADAEZINWA");

    let data =
        find_and_parse(&format!("{short_l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "PD");
    assert_eq!(data.issuing_country, "BRA");
    assert_eq!(data.surname, "OKAFORCHUKWUDI");
    assert_eq!(data.given_names, "ADAEZINWA");
    assert!(data.valid());
}

/// Same regression pin as the TD3 case above, for a genuine TD2/MRV-family
/// two-letter document code (`IP`) on a short TD2 line 1.
#[test]
fn a_genuine_td2_two_letter_code_on_a_short_line1_is_unaffected() {
    let mrz = format_td2(&Td2Fields {
        document_code: "IP".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "OKAFORCHUKWUDIOB".to_string(),
        given_names: "ADAEZ".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td2 emits two lines");
    let short_l1 = l1.trim_end_matches('<');
    assert_eq!(short_l1, "IPBRAOKAFORCHUKWUDIOB<<ADAEZ");

    let data =
        find_and_parse(&format!("{short_l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::Td2);
    assert_eq!(data.document_type, "IP");
    assert_eq!(data.issuing_country, "BRA");
    assert_eq!(data.surname, "OKAFORCHUKWUDIOB");
    assert_eq!(data.given_names, "ADAEZ");
    assert!(data.valid());
}

/// MRV-A's ordinary scan routes through the same helper — an unresolvable
/// issuer on a short line must not be inflated there either.
#[test]
fn mrv_a_short_line1_with_an_unresolvable_issuer_is_not_corrupted() {
    let mrz = format_mrv_a(&MrvAFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "AAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
        given_names: String::new(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (_, l2) = mrz.split_once('\n').expect("format_mrv_a emits two lines");
    let l1 = "V<XYZAAAAAAAAAAAAAAAAAAAAAAAAA";
    assert_eq!(l1.len(), 30);

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::MrvA);
    assert_eq!(data.document_type, "V");
    assert_eq!(
        data.issuing_country, "XYZ",
        "the tail-padded issuer must be read literally, not inflated into filler"
    );
    assert_eq!(data.surname, "AAAAAAAAAAAAAAAAAAAAAAAAA");
    assert!(data.valid());
}

/// Same as the MRV-A case above, for MRV-B.
#[test]
fn mrv_b_short_line1_with_an_unresolvable_issuer_is_not_corrupted() {
    let mrz = format_mrv_b(&MrvBFields {
        document_code: "V".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E0000000".to_string(),
        surname: "AAAAAAAAAAAAAAAAA".to_string(),
        given_names: String::new(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (_, l2) = mrz.split_once('\n').expect("format_mrv_b emits two lines");
    let l1 = "V<XYZAAAAAAAAAAAAAAAAA";
    assert_eq!(l1.len(), 22);

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::MrvB);
    assert_eq!(data.document_type, "V");
    assert_eq!(
        data.issuing_country, "XYZ",
        "the tail-padded issuer must be read literally, not inflated into filler"
    );
    assert_eq!(data.surname, "AAAAAAAAAAAAAAAAA");
    assert!(data.valid());
}

/// Same as the MRV-A/MRV-B cases above, for TD2's ordinary scan.
#[test]
fn td2_short_line1_with_an_unresolvable_issuer_is_not_corrupted() {
    let mrz = format_td2(&Td2Fields {
        document_code: "I".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E00000000".to_string(),
        surname: "AAAAAAAAAAAAAAAAA".to_string(),
        given_names: String::new(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data: None,
    });
    let (_, l2) = mrz.split_once('\n').expect("format_td2 emits two lines");
    let l1 = "I<XYZAAAAAAAAAAAAAAAAA";
    assert_eq!(l1.len(), 22);

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("a short line 1 must still parse");

    assert_eq!(data.format, Format::Td2);
    assert_eq!(data.document_type, "I");
    assert_eq!(
        data.issuing_country, "XYZ",
        "the tail-padded issuer must be read literally, not inflated into filler"
    );
    assert_eq!(data.surname, "AAAAAAAAAAAAAAAAA");
    assert!(data.valid());
}

/// The class-sweep path (`class_sweep_pass`, gated behind
/// [`ParseOptions::class_sweep`]) shares `td3_line1_variants` with the
/// ordinary scan, so it must route through the same fix. Pairs the short
/// Germany-style `D<<` line 1 above with a line 2 whose document number is
/// nine identical digits, uniformly misread as their confusable letter
/// (check digit included) — the exact shape `crates/mrz/tests/class_sweep_wiring.rs`
/// exercises for TD1, here for TD3's `td3_line1_variants`/`two_line_line1_variants_shifted`
/// wiring instead.
#[test]
fn class_sweep_path_recovers_a_short_germany_style_line1_and_the_document_number() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "D".to_string(),
        document_number: "111111111".to_string(),
        surname: "SCHWARZENEGGER".to_string(),
        given_names: "REYNALDALEXANDER".to_string(),
        nationality: "DEU".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Male,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    let short_l1 = l1.trim_end_matches('<');
    assert_eq!(short_l1, "P<D<<SCHWARZENEGGER<<REYNALDALEXANDER");

    // Every '1' in the document number (and its own check digit, if it were
    // a '1') read as its confusable letter 'I' — the check digit here is
    // '3', so only the nine document-number cells change.
    let swept_l2: String = l2
        .chars()
        .enumerate()
        .map(|(i, c)| if i < 10 && c == '1' { 'I' } else { c })
        .collect();
    assert_ne!(swept_l2, l2, "sanity: corruption changed the line");

    let opts = ParseOptions::default().with_class_sweep(true);
    let data = find_and_parse_with(&format!("{short_l1}\n{swept_l2}"), &opts)
        .expect("the class sweep must recover a short line 1 alongside the swept document number");

    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "D",
        "the D<< filler slot must never be inflated, even on the class-sweep path"
    );
    assert_eq!(data.surname, "SCHWARZENEGGER");
    assert_eq!(data.given_names, "REYNALDALEXANDER");
    assert_eq!(data.document_number, "111111111");
    assert!(data.valid());
}

/// The damaged-capture path (`damaged_pass`'s single-substitution search,
/// tried when `class_sweep_pass` cannot help) also shares
/// `td3_line1_variants`. Distinct digits in the document number — as
/// `crates/mrz/tests/damaged_td3_line1_shift.rs`'s own fixture explains —
/// mean the class sweep cannot resolve a single corrupted cell, forcing the
/// single-substitution search instead; `class_sweep` is left off (the
/// default) so `class_sweep_pass` cannot even attempt it.
#[test]
fn damaged_path_recovers_a_short_germany_style_line1_and_a_substituted_digit() {
    let mrz = format_td3(&Td3Fields {
        document_code: "P".to_string(),
        issuing_country: "D".to_string(),
        document_number: "123456789".to_string(),
        surname: "SCHWARZENEGGER".to_string(),
        given_names: "REYNALDALEXANDER".to_string(),
        nationality: "DEU".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Male,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    let short_l1 = l1.trim_end_matches('<');
    assert_eq!(short_l1, "P<D<<SCHWARZENEGGER<<REYNALDALEXANDER");

    let mut chars: Vec<char> = l2.chars().collect();
    assert_eq!(chars[0], '1', "sanity: document number starts with a 1");
    chars[0] = 'I';
    let corrupted_l2: String = chars.into_iter().collect();

    let data = find_and_parse(&format!("{short_l1}\n{corrupted_l2}"))
        .expect("the damaged pass must recover a short line 1 alongside the substituted digit");

    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "D",
        "the D<< filler slot must never be inflated, even on the damaged-capture path"
    );
    assert_eq!(data.surname, "SCHWARZENEGGER");
    assert_eq!(data.given_names, "REYNALDALEXANDER");
    assert_eq!(data.document_number, "123456789");
    assert!(data.valid());
    assert!(data.damaged_recovery);
}

/// Exact-width lines are untouched by this fix (`fit_line1_length` only
/// ever differs from `fit_length` when `n.len() < target`) — a full,
/// undamaged TD3 zone must parse exactly as before.
#[test]
fn an_exact_width_line1_is_unaffected() {
    let mrz = format_td3(&Td3Fields {
        document_code: "PD".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "OKAFORCHUKWUDI".to_string(),
        given_names: "ADAEZINWA".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert_eq!(l1.len(), 44, "sanity: the emitted line is full width");

    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("an undamaged zone parses");

    assert_eq!(data.document_type, "PD");
    assert_eq!(data.issuing_country, "BRA");
    assert_eq!(data.surname, "OKAFORCHUKWUDI");
    assert_eq!(data.given_names, "ADAEZINWA");
    assert!(data.valid());
}

/// An interior filler dropped well inside the name field's own padding run
/// (not at the document-code/issuer boundary) leaves the longest run at or
/// after index 5, so this is unaffected by the fix — regression pin for
/// point 3, at one cell short rather than the fully-dropped-tail shape the
/// other tests above use.
#[test]
fn an_interior_dropped_filler_inside_the_name_padding_is_unaffected() {
    let mrz = format_td3(&Td3Fields {
        document_code: "PD".to_string(),
        issuing_country: "BRA".to_string(),
        document_number: "E000000000".to_string(),
        surname: "OKAFORCHUKWUDI".to_string(),
        given_names: "ADAEZINWA".to_string(),
        nationality: "BRA".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        personal_number: None,
    });
    let (l1, l2) = mrz.split_once('\n').expect("format_td3 emits two lines");
    assert_eq!(l1.len(), 44);

    // Drop exactly one filler cell from the trailing name padding.
    let mut damaged = l1.to_string();
    let last = damaged.pop();
    assert_eq!(last, Some('<'), "sanity: the dropped cell is a filler");
    assert_eq!(damaged.len(), 43);

    let data =
        find_and_parse(&format!("{damaged}\n{l2}")).expect("a one-cell-short line still parses");

    assert_eq!(data.document_type, "PD");
    assert_eq!(data.issuing_country, "BRA");
    assert_eq!(data.surname, "OKAFORCHUKWUDI");
    assert_eq!(data.given_names, "ADAEZINWA");
    assert!(data.valid());
}

/// TD1's own line 1 is not routed through the new helper (see
/// `crate::checksum::line1_variants`'s doc comment): its issuing-state slot
/// sits at the same positions 2..5 as TD3's, but TD1's own check digit on
/// line 1 (the document number's) arbitrates a shifted reading the way
/// TD3/MRV-A/MRV-B/TD2 line 1 never can, so it keeps using plain
/// `variants`/`fit_length` unconditionally. This mirrors
/// `crates/mrz/tests/td1_line_gap.rs`'s
/// `a_dropped_line_1_filler_position_is_recovered`, confirming that
/// fixture is unaffected by this change.
#[test]
fn td1_short_line1_is_unaffected_by_the_line1_fit_change() {
    let mrz = format_td1(&Td1Fields {
        document_code: "ID".to_string(),
        issuing_country: "UTO".to_string(),
        document_number: "E00000000".to_string(),
        surname: "ESKANDARI".to_string(),
        given_names: "MAREN".to_string(),
        nationality: "UTO".to_string(),
        date_of_birth: support::birth("800101"),
        sex: Sex::Female,
        date_of_expiry: support::expiry("301230"),
        optional_data_1: None,
        optional_data_2: None,
    });
    let lines: Vec<&str> = mrz.lines().collect();
    assert_eq!(lines.len(), 3, "format_td1 emits exactly three lines");

    let mut damaged_l1 = lines[0].to_string();
    damaged_l1.remove(1);
    assert_eq!(damaged_l1.len(), 29);

    let text = format!("{damaged_l1}\n{}\n{}", lines[1], lines[2]);
    let data = find_and_parse(&text).expect("the unshifted candidate must recover the read");

    assert_eq!(data.format, Format::Td1);
    assert_eq!(data.document_number, "E00000000");
    assert!(data.valid());
}
