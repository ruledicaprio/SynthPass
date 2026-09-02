//! Real documents whose MRZ line 1 does not follow ICAO 9303 Part 4's layout,
//! and what this crate is expected to do with them.
//!
//! Line 1 positions 1-5 carry no check digit in any format, so nothing
//! arbitrates a misread or a misprint. That makes these cases worth pinning
//! precisely: in every one the correct behaviour is a **clean, strict parse of
//! what is actually printed** — never a silent repair into what the document
//! probably meant. A parser that guesses here would be inventing data, and it
//! would do so on exactly the field a caller uses to decide which state issued
//! the document.
//!
//! Each line below is transcribed from a specimen in `samples/`, recorded in
//! `samples/corpus.jsonl` with `notes` explaining the defect. They are written
//! as literal strings rather than read from the corpus so this file runs in a
//! clone with no images synced.

use mrz::{find_and_parse, Format};

/// Argentina's emergency passport puts a filler at position 3 and `ARG` at
/// 4-6, shifting the name field one character right of where Part 4 §4.2.2
/// puts it.
///
/// The VIZ prints type `PE` and country `ARG`, so the intent is unambiguous —
/// but positions 3-5 literally read `<AR`, and that is what a conformant
/// parser must report. Reading `ARG` here would require assuming the document
/// is wrong and the parser knows better, on a field with no check digit to
/// settle it.
#[test]
fn argentina_emergency_passport_shifts_the_name_field_one_right() {
    // samples/passports/Argentina_Passport_Specimen_PE_ARG_2015_mrz_emergency.png
    let l1 = "PE<ARGGONZALEZ<<SARA<EMMA<<<<<<<<<<<<<<<<<<<";
    let l2 = "12345678A8ARG6801014F1611106123456788<<<<<88";
    let data =
        find_and_parse(&format!("{l1}\n{l2}")).expect("a non-conformant line 1 still parses");

    assert_eq!(data.format, Format::Td3);
    assert_eq!(data.document_type, "PE");
    assert_eq!(
        data.issuing_country, "<AR",
        "positions 3-5 are read literally; ARG sits at 4-6 on this document"
    );
    assert_eq!(
        data.issuing_country_name(),
        None,
        "`<AR` must not resolve to a state — a caller has to see that this is wrong"
    );
    assert_eq!(
        data.surname, "GGONZALEZ",
        "the shifted boundary pushes the first name character into the surname; \
         inventing GONZALEZ here would be a guess"
    );
}

/// Spain's 2022 specimen omits the issuing state entirely: line 1 runs
/// straight from the document code into the name, so positions 3-5 hold the
/// first three characters of the surname.
///
/// Line 2 *does* carry `ESP` as the nationality, which makes this the exact
/// shape `synthpass_core::fusion` flags — an unrecognised issuing country that
/// disagrees with a valid nationality. Nothing may infer `ESP` from line 2:
/// that is the unproven-heuristic move `repair::solve_substitution`'s
/// "prove it or don't touch it" discipline exists to prevent.
#[test]
fn a_passport_with_no_issuing_state_reads_the_name_into_positions_3_to_5() {
    // samples/passports/Spain_Passport_Specimen_P0_2022_mrz.jpg
    let l1 = "P<TAPIA<JUAN<DASEC<<<<<<<<<<<<<<<<<<<<<<<<<<";
    let l2 = "KV24247253ESP7809062M3202193A3064941000<<<30";
    let data = find_and_parse(&format!("{l1}\n{l2}")).expect("a missing state field still parses");

    assert_eq!(data.document_type, "P");
    assert_eq!(
        data.issuing_country, "TAP",
        "with no state field, positions 3-5 are the start of the surname"
    );
    assert_eq!(data.issuing_country_name(), None);
    assert_eq!(
        data.nationality, "ESP",
        "line 2's nationality is intact and disagrees with the issuing country"
    );
}

/// A blank template specimen: every field, including the whole MRZ, is
/// literally `X`.
///
/// This belongs to the robustness set rather than the accuracy set. The
/// requirement is that it produces no *believable* data — the checksums must
/// fail, so `valid()` is false and the pipeline escalates instead of treating
/// a page of placeholder glyphs as a real document.
#[test]
fn an_all_x_template_specimen_never_validates() {
    // samples/passports/Armenia_Passport_Specimen_XX_XXX_XXXX_redacted_mrz.png
    let l1 = "X<XXXXXXXXXXXXXX<<XXXX<XXXXX<<<<<<<<<<<<<<<<";
    let l2 = "XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX<<<<<<<X";

    // Refusing to parse it at all is equally correct, so only a successful
    // parse carries a requirement: it must not claim valid checksums.
    if let Ok(data) = find_and_parse(&format!("{l1}\n{l2}")) {
        assert!(
            !data.valid(),
            "a placeholder MRZ must never report valid checksums"
        );
    }
}

/// Germany's specimen passports carry `BDR` — the Bundesdruckerei printer
/// code — where a real document carries `D<<`.
///
/// It is deliberately absent from the country table. A `BDR` document is by
/// construction not a real passport, so failing to resolve is the desired
/// production behaviour: it surfaces as an unrecognised issuing state rather
/// than passing silently as German.
#[test]
fn the_bundesdruckerei_specimen_code_does_not_resolve_to_a_state() {
    assert_eq!(mrz::country_name("BDR"), None);
    assert_eq!(
        mrz::country_name("D"),
        Some("Germany"),
        "the real legacy code still resolves"
    );
}

/// Kosovo prints `RKS`, which is not an ISO 3166-1 alpha-3 code but is what
/// the document actually carries, and the table recognises it.
#[test]
fn kosovos_non_iso_code_resolves() {
    assert!(
        mrz::country_name("RKS").is_some(),
        "RKS is legitimately printed on Kosovar passports"
    );
}
