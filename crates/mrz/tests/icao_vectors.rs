//! Check-digit vectors pinned from ICAO Doc 9303's own worked examples.
//!
//! The specimen tests in `src/lib.rs` prove whole-document parses; this file
//! proves the check-digit primitive against values the standard *publishes*,
//! so a prospective user can see the arithmetic matches ICAO exactly rather
//! than only matching this crate's own fixtures.
//!
//! Each expected digit below is independently verifiable by hand under the
//! 7-3-1 weighting (`0-9 → 0-9`, `A-Z → 10-35`, `< → 0`); the computation is
//! shown for the two Part 3 examples in the comments.
//!
//! On provenance: the corpus in `knowledge/docs9303/` carries Doc 9303 as
//! converted text, and not every worked example survived that conversion as
//! text — several specimens are images in the source PDFs. Each vector below
//! therefore records whether the corpus can actually corroborate it, rather
//! than asserting a blanket "these come from ICAO". See
//! `knowledge/docs9303/CONFORMANCE_BASIS.md`.

use mrz::{check_digit, verify};

/// `(field, expected_check_digit, source)`.
///
/// The `source` string names where the value comes from and, where the corpus
/// cannot corroborate it, says so. Every expected digit is correct arithmetic
/// under the 7-3-1 rule regardless — what varies is whether *this corpus*
/// independently confirms the value was printed on an ICAO specimen.
const VECTORS: &[(&str, u32, &str)] = &[
    // ICAO 9303 Part 3, check-digit worked examples.
    //   5·7 + 2·3 + 0·1 + 7·7 + 2·3 + 7·1 = 35+6+0+49+6+7 = 103 → 103 % 10 = 3
    ("520727", 3, "9303 pt3 numeric example"),
    //   A·7 + B·3 + 2·1 + 1·7 + 3·3 + 4·1 = 70+33+2+7+9+4 = 125 → 125 % 10 = 5
    ("AB2134<<<", 5, "9303 pt3 alphanumeric+filler example"),
    // TD3 specimen (Utopia / Anna Maria Eriksson) field check digits.
    //
    // Part 4's specimen data page is an IMAGE in the source PDF and did not
    // survive conversion as text: searching the corpus for `L898902` or
    // `ZE184226` hits only Part 7's MRV specimens, never Part 4. So the two
    // document-specific values below cannot be corroborated from this corpus,
    // and are labelled accordingly rather than credited to Part 4.
    (
        "L898902C3",
        6,
        "TD3 specimen document number (not corroborated by the corpus - Part 4's specimen is image-only)",
    ),
    (
        "ZE184226B<<<<<",
        1,
        "TD3 specimen personal number (not corroborated by the corpus - Part 4's specimen is image-only)",
    ),
    // These two ARE corroborated, though from Part 6 rather than Part 4: the
    // TD2 specimen's line 2 survived as literal text and carries the same
    // date of birth and date of expiry, with the same check digits.
    // `knowledge/docs9303/Doc_9303_Part6_Specs_for_TD2_MROTDs.md:488`
    ("740812", 2, "9303 pt6 TD2 specimen date of birth"),
    ("120415", 9, "9303 pt6 TD2 specimen date of expiry"),
];

#[test]
fn check_digit_matches_icao_worked_examples() {
    for &(field, expected, source) in VECTORS {
        let got = check_digit(field).unwrap_or_else(|e| panic!("{source}: {field:?} errored: {e}"));
        assert_eq!(got, expected, "{source}: check_digit({field:?})");
    }
}

#[test]
fn verify_accepts_the_published_digit_and_rejects_others() {
    for &(field, expected, source) in VECTORS {
        let good = char::from_digit(expected, 10).unwrap();
        assert!(
            verify(field, good),
            "{source}: verify({field:?}, {good:?}) should accept the published digit"
        );
        let wrong = char::from_digit((expected + 1) % 10, 10).unwrap();
        assert!(
            !verify(field, wrong),
            "{source}: verify({field:?}, {wrong:?}) should reject a wrong digit"
        );
    }
}

/// Composite check-digit vectors from ICAO 9303 Part 3 Appendix A's three
/// fully worked composite examples (Examples 3-5,
/// `knowledge/docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:1241-1383`).
///
/// The composite input is *not* the human-readable MRZ line ICAO prints: it
/// excludes nationality and sex (TD2/TD3), and for TD1 it excludes the
/// entire third line (name field) as well
/// (`knowledge/docs9303/Doc_9303_Part5_Specs_for_TD1_MROTDs.md:478-479`).
/// Each entry below reconstructs the composite input from the format's
/// published MRZ line(s) using the crate's own composite ranges
/// (`src/parser.rs` `parse_td3`/`parse_td2`/`parse_td1`) and asserts both
/// that the reconstruction matches ICAO's published composite-input string
/// *and* that `check_digit` on it reproduces ICAO's published sum-derived
/// digit — asserting the digit alone would let a mistranscribed range and a
/// compensating arithmetic slip cancel each other out.
struct CompositeVector {
    source: &'static str,
    /// The full MRZ line(s) ICAO prints, exactly as published.
    lines: &'static [&'static str],
    /// Byte ranges (start, end) into the concatenation of `lines`, in the
    /// order the composite input is built from.
    ranges: &'static [(usize, usize)],
    /// ICAO's published composite input string (39/50/31 chars).
    expected_input: &'static str,
    /// ICAO's published sum of products, for the comment trail.
    expected_sum: u32,
    /// ICAO's published composite check digit.
    expected_digit: u32,
}

const COMPOSITE_VECTORS: &[CompositeVector] = &[
    // Example 3 — TD3, lower line positions 1-43.
    // Composite = line2[0..10] + line2[13..20] + line2[21..43] (parser.rs:141-144).
    // ICAO sum of products = 448 -> 448 % 10 = 8.
    CompositeVector {
        source: "9303 pt3 Example 3 (TD3 composite)",
        lines: &["HA672242<6YTO5802254M9601086<<<<<<<<<<<<<<0"],
        ranges: &[(0, 10), (13, 20), (21, 43)],
        expected_input: "HA672242<658022549601086<<<<<<<<<<<<<<0",
        expected_sum: 448,
        expected_digit: 8,
    },
    // Example 4 — TD1, upper line (30) + middle line positions 1-29.
    // Composite = line1[5..30] + line2[0..7] + line2[8..15] + line2[18..29]
    // (parser.rs:278-287) — the entire third (name) line is excluded, per
    // Doc_9303_Part5_Specs_for_TD1_MROTDs.md:478-479.
    // ICAO sum of products = 392 -> 392 % 10 = 2.
    CompositeVector {
        source: "9303 pt3 Example 4 (TD1 composite)",
        lines: &[
            "I<YTOD231458907<<<<<<<<<<<<<<<",
            "3407127M9507122YTO<<<<<<<<<<<",
        ],
        ranges: &[(5, 30), (30, 37), (38, 45), (48, 59)],
        expected_input: "D231458907<<<<<<<<<<<<<<<34071279507122<<<<<<<<<<<",
        expected_sum: 392,
        expected_digit: 2,
    },
    // Example 5 — TD2, lower line positions 1-35.
    // Composite = line2[0..10] + line2[13..20] + line2[21..35] (parser.rs:205-208).
    // ICAO sum of products = 448 -> 448 % 10 = 8.
    CompositeVector {
        source: "9303 pt3 Example 5 (TD2 composite)",
        lines: &["HA672242<6YTO5802254M9601086<<<<<<<"],
        ranges: &[(0, 10), (13, 20), (21, 35)],
        expected_input: "HA672242<658022549601086<<<<<<<",
        expected_sum: 448,
        expected_digit: 8,
    },
];

#[test]
fn composite_check_digit_matches_icao_worked_examples() {
    for v in COMPOSITE_VECTORS {
        let joined: String = v.lines.concat();
        let input: String = v.ranges.iter().map(|&(a, b)| &joined[a..b]).collect();
        assert_eq!(
            input, v.expected_input,
            "{}: reconstructed composite input from the published ranges did not match \
             ICAO's published composite-input string",
            v.source
        );

        let got = check_digit(&input)
            .unwrap_or_else(|e| panic!("{}: check_digit({input:?}) errored: {e}", v.source));
        assert_eq!(
            got, v.expected_digit,
            "{}: check_digit({input:?}) (ICAO sum of products = {}, {} % 10 = {})",
            v.source, v.expected_sum, v.expected_sum, v.expected_digit
        );
    }
}

/// §4.2.3.1's worked VIZ→MRZ name examples
/// (`knowledge/docs9303/Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:479-533`),
/// transcribed verbatim as string literals. Every TD3 upper line, name field
/// included, is a fixed 44 characters (`PP<UTO` document code/country
/// prefix + 39-character name field); ICAO's own examples must therefore
/// each be exactly 44 characters.
///
/// This is a length check only — it does *not* run these strings through
/// `format_td3`. The name-encoding implementation (hyphen/apostrophe
/// handling, truncation) is segment S3's job, and example (d) below
/// (`O'CONNOR`) is known to fail against the crate's current name encoder
/// today. A failure in this test means corpus transcription damage (a
/// dropped or added character when this file was typed in), not an
/// implementation bug — there is no implementation under test here.
///
/// The §4.2.3.3c example (`:571`, `PPUTOBENNEL<WOOLOOM<WARRAN<WARNAM<<DINGO<POTO`)
/// is deliberately excluded: it counts to 45 characters, not the 44 the
/// format requires, and is either a dropped/added character in the source
/// PDF's own typesetting or an erratum — a known corpus defect, not
/// something to pin as a passing vector. See `CONFORMANCE_BASIS.md`.
const SECTION_4_2_3_1_NAME_EXAMPLES: &[(&str, &str)] = &[
    (
        "a) Anna Maria Eriksson",
        "PPUTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    ),
    (
        "b) Deborah Heng Ming Lo",
        "PPUTOHENG<<DEBORAH<MING<LO<<<<<<<<<<<<<<<<<<",
    ),
    (
        "c) Susie Margaret Smith-Jones",
        "PPUTOSMITH<JONES<<SUSIE<MARGARET<<<<<<<<<<<<",
    ),
    (
        "d) Enya Siobhan O'Connor",
        "PPUTOOCONNOR<<ENYA<SIOBHAN<<<<<<<<<<<<<<<<<<",
    ),
    (
        "e) Martin Van Der Muellen",
        "PPUTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<<<<<<<<<",
    ),
    (
        "e) Huda Muhammad Jawad Al-Basri",
        "PPUTOAL<BASRI<<HUDA<MUHAMMAD<JAWAD<<<<<<<<<<",
    ),
    (
        "e) Jose Ramon Vilarchao Fernandez",
        "PPUTOVILARCHAO<FERNANDEZ<<JOSE<RAMON<<<<<<<<",
    ),
    (
        "f) Arkfreith",
        "PPUTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<",
    ),
    (
        "f) Satriya Sudarpa",
        "PPUTOSATRIYA<SUDARPA<<<<<<<<<<<<<<<<<<<<<<<<",
    ),
];

#[test]
fn section_4_2_3_1_examples_are_44_characters() {
    for &(source, mrz_line) in SECTION_4_2_3_1_NAME_EXAMPLES {
        assert_eq!(
            mrz_line.len(),
            44,
            "9303 pt4 §4.2.3.1 {source}: {mrz_line:?} is not 44 characters — likely \
             corpus transcription damage, not an implementation bug"
        );
    }
}
