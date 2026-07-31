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

use mrz::{
    check_digit, format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3, verify,
    MrvAFields, MrvBFields, Td1Fields, Td2Fields, Td3Fields,
};

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
/// `format_td3`. `name_encoding_matches_icao_worked_examples` below (segment
/// S3) does that: it re-derives the same examples from their `VIZ:` source
/// lines and drives them through the real emitter, including example (d)
/// (`O'CONNOR`), which the crate's name encoder got wrong before S3 (it
/// produced `O<CONNOR` instead of `OCONNOR` — see
/// `Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:511-515`). A failure in
/// *this* test means corpus transcription damage (a dropped or added
/// character when this file was typed in), not an implementation bug —
/// there is no implementation under test here.
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

/// Segment S3 — golden name-encoding vectors, one table per MRZ format.
///
/// Each entry is `(VIZ, expected full MRZ name line)`, transcribed verbatim
/// from the corpus (never retyped from memory — see the citations on each
/// table). The test below splits `VIZ` on the *first* comma into primary and
/// secondary identifiers (`Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:495`),
/// feeds them to the real `format_*` emitter as `surname`/`given_names`, and
/// asserts the emitted name-bearing line matches the expected line
/// byte-for-byte. This exercises `crate::emit::clean_name_half` (hyphen,
/// comma, apostrophe, and other-punctuation handling) end-to-end, not just
/// the length check `SECTION_4_2_3_1_NAME_EXAMPLES` performs above.
///
/// Only ICAO examples that **fit** their field without truncation are
/// pinned here. Truncation (§4.2.3.2/§4.2.3.3-family examples, and Part 7's
/// equivalents) is issuer-discretionary per Part 4 §4.2.3
/// (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:463-587`) — this crate
/// picks one conformant strategy (`emit::truncate_name_components`'s doc
/// comment) rather than ICAO's specific worked truncations, so those
/// examples are deliberately not asserted against here. See
/// `knowledge/docs9303/CONFORMANCE_BASIS.md`'s S3 entry.
///
/// Three corpus examples are excluded regardless: all three are from the
/// truncation-family sections (§4.2.3.2/§4.2.3.3, MRV-A's equivalent), which
/// are never pinned here at all (see above) because truncation strategy is
/// issuer-discretionary, not because of a length defect — also recorded in
/// `CONFORMANCE_BASIS.md`:
/// - `Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:576` (§4.2.3.3c): was
///   45 characters on a 44-character TD3 line due to a corpus transcription
///   error (an extra `M`); fixed in the corpus (now a clean 44 characters),
///   but still excluded here as a truncation example, not a defect.
/// - `Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:332` (MRV-A example a):
///   45 characters on a 44-character line — confirmed a genuine ICAO
///   erratum present in the source PDF itself, not corpus damage.
/// - `Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:350` (MRV-A example d,
///   `O'CONNOR`): 36 characters in what should be a 44-character section —
///   confirmed a genuine ICAO erratum (the MRV-B width was pasted in by
///   mistake in ICAO's own text, not this corpus's conversion).
struct NameVector {
    source: &'static str,
    viz: &'static str,
    /// The full MRZ line ICAO publishes, including the document
    /// code/country prefix (TD3/TD2/MRV-A/MRV-B) or, for TD1, the bare
    /// 30-character name line with no prefix at all.
    expected_line: &'static str,
}

/// `Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:479-584`. Prefix `PPUTO`
/// (5 chars: document code `PP` + issuing country `UTO`), name field 39
/// chars, upper line 44 chars total.
const TD3_NAME_VECTORS: &[NameVector] = &[
    NameVector {
        source: "9303 pt4 §4.2.3.1(a):480",
        viz: "ERIKSSON, ANNA MARIA",
        expected_line: "PPUTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.1(b):488",
        viz: "HENG, DEBORAH MING LO",
        expected_line: "PPUTOHENG<<DEBORAH<MING<LO<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.1(c):496",
        viz: "SMITH-JONES, SUSIE MARGARET",
        expected_line: "PPUTOSMITH<JONES<<SUSIE<MARGARET<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.1(d):504 — the O'CONNOR case",
        viz: "O'CONNOR, ENYA SIOBHAN",
        expected_line: "PPUTOOCONNOR<<ENYA<SIOBHAN<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.1(e) van der Muellen:512",
        viz: "VAN DER MUELLEN, MARTIN",
        expected_line: "PPUTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.1(e) Al-Basri:516",
        viz: "AL-BASRI, HUDA MUHAMMAD JAWAD",
        expected_line: "PPUTOAL<BASRI<<HUDA<MUHAMMAD<JAWAD<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.1(e) Vilarchao Fernandez:520",
        viz: "VILARCHAO FERNANDEZ, JOSE RAMON",
        expected_line: "PPUTOVILARCHAO<FERNANDEZ<<JOSE<RAMON<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.1(f) Arkfreith:528",
        viz: "ARKFREITH",
        expected_line: "PPUTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.1(f) Satriya Sudarpa:532",
        viz: "SATRIYA SUDARPA",
        expected_line: "PPUTOSATRIYA<SUDARPA<<<<<<<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt4 §4.2.3.4 Papandropoulous:584 — fits exactly, \
                  NOT truncated (see :585-587's note)",
        viz: "PAPANDROPOULOUS, JONATHON WARREN TREVOR",
        expected_line: "PPUTOPAPANDROPOULOUS<<JONATHON<WARREN<TREVOR",
    },
];

/// `Doc_9303_Part5_Specs_for_TD1_MROTDs.md:427-457`. Bare 30-character name
/// line (the TD1 third line), no document-code/country prefix.
const TD1_NAME_VECTORS: &[NameVector] = &[
    NameVector {
        source: "9303 pt5 §4.2.3.3:427",
        viz: "PAPANDROPOULOUS, JONATHON ALEC",
        expected_line: "PAPANDROPOULOUS<<JONATHON<ALEC",
    },
    NameVector {
        source: "9303 pt5 §4.2.3.4 van der Muellen:437",
        viz: "VAN DER MUELLEN, MARTIN",
        expected_line: "VAN<DER<MUELLEN<<MARTIN<<<<<<<",
    },
    NameVector {
        source: "9303 pt5 §4.2.3.4 Al-Basri:441",
        viz: "AL-BASRI, HUDA MUHAMMAD JAWAD",
        expected_line: "AL<BASRI<<HUDA<MUHAMMAD<JAWAD<",
    },
    NameVector {
        source: "9303 pt5 §4.2.3.5 Arkfreith:453",
        viz: "ARKFREITH",
        expected_line: "ARKFREITH<<<<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt5 §4.2.3.5 Satriya Sudarpa:457",
        viz: "SATRIYA SUDARPA",
        expected_line: "SATRIYA<SUDARPA<<<<<<<<<<<<<<<",
    },
];

/// `Doc_9303_Part6_Specs_for_TD2_MROTDs.md:399-429`. Prefix `I<UTO` (5
/// chars), name field 31 chars, upper line 36 chars total.
const TD2_NAME_VECTORS: &[NameVector] = &[
    NameVector {
        source: "9303 pt6 §4.2.3.3:399",
        viz: "PAPANDROPOULOUS, JONATHOON ALEC",
        expected_line: "I<UTOPAPANDROPOULOUS<<JONATHOON<ALEC",
    },
    NameVector {
        source: "9303 pt6 §4.2.3.4 van der Muellen:409",
        viz: "VAN DER MUELLEN, MARTIN",
        expected_line: "I<UTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<",
    },
    NameVector {
        source: "9303 pt6 §4.2.3.4 Al-Basri:413",
        viz: "AL-BASRI, HUDA MUHAMMAD JAWAD",
        expected_line: "I<UTOAL<BASRI<<HUDA<MUHAMMAD<JAWAD<<",
    },
    NameVector {
        source: "9303 pt6 §4.2.3.4 Vilarchao Fernandez:417",
        viz: "VILARCHAO FERNANDEZ, JOSE RAMON",
        expected_line: "I<UTOVILARCHAO<FERNANDEZ<<JOSE<RAMON",
    },
    NameVector {
        source: "9303 pt6 §4.2.3.5 Arkfreith:425",
        viz: "ARKFREITH",
        expected_line: "I<UTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt6 §4.2.3.5 Satriya Sudarpa:429",
        viz: "SATRIYA SUDARPA",
        expected_line: "I<UTOSATRIYA<SUDARPA<<<<<<<<<<<<<<<<",
    },
];

/// `Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:338-402`. Prefix `V<UTO`
/// (5 chars), name field 39 chars, upper line 44 chars total. Examples (a)
/// `:332` and (d) `:350` are excluded — both are known corpus defects (wrong
/// length), see the module-level doc comment above.
const MRV_A_NAME_VECTORS: &[NameVector] = &[
    NameVector {
        source: "9303 pt7 §4.2.3(b):338",
        viz: "HENG, DEBORAH MING LO",
        expected_line: "V<UTOHENG<<DEBORAH<MING<LO<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §4.2.3(c):344",
        viz: "SMITH-JONES, SUSIE MARGARET",
        expected_line: "V<UTOSMITH<JONES<<SUSIE<MARGARET<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §4.2.3(e):356",
        viz: "VAN DER MUELLEN, MARTIN",
        expected_line: "V<UTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §4.2.3(f):362",
        viz: "ARKFREITH",
        expected_line: "V<UTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §4.2.3.3 Papandropoulous:402 — fits exactly, \
                  NOT truncated",
        viz: "PAPANDROPOULOUS, JONATHON WARREN TREVOR",
        expected_line: "V<UTOPAPANDROPOULOUS<<JONATHON<WARREN<TREVOR",
    },
];

/// `Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:690-760`. Prefix `V<UTO`
/// (5 chars), name field 31 chars, upper line 36 chars total.
const MRV_B_NAME_VECTORS: &[NameVector] = &[
    NameVector {
        source: "9303 pt7 §7.2.3(a):690",
        viz: "ERIKSSON, ANNA MARIA",
        expected_line: "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §7.2.3(b):696",
        viz: "HENG, DEBORAH MING LO",
        expected_line: "V<UTOHENG<<DEBORAH<MING<LO<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §7.2.3(c):702",
        viz: "SMITH-JONES, SUSIE MARGARET",
        expected_line: "V<UTOSMITH<JONES<<SUSIE<MARGARET<<<<",
    },
    NameVector {
        source: "9303 pt7 §7.2.3(d):708 — the O'CONNOR case",
        viz: "O'CONNOR, ENYA SIOBHAN",
        expected_line: "V<UTOOCONNOR<<ENYA<SIOBHAN<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §7.2.3(e):714",
        viz: "VAN DER MUELLEN, MARTIN",
        expected_line: "V<UTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §7.2.3(f):720",
        viz: "ARKFREITH",
        expected_line: "V<UTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<",
    },
    NameVector {
        source: "9303 pt7 §7.2.3.3 Papandropoulous:760 — fits exactly, \
                  NOT truncated",
        viz: "PAPANDROPOULOUS, STEPHEN TREVOR",
        expected_line: "V<UTOPAPANDROPOULOUS<<STEPHEN<TREVOR",
    },
];

/// Split a `VIZ` name on the *first* comma into (primary, secondary),
/// trimming the whitespace `Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:495`
/// puts around the comma. No comma means no secondary identifier.
fn split_viz(viz: &str) -> (&str, &str) {
    match viz.split_once(',') {
        Some((primary, secondary)) => (primary.trim(), secondary.trim()),
        None => (viz.trim(), ""),
    }
}

#[test]
fn name_encoding_matches_icao_worked_examples() {
    for v in TD3_NAME_VECTORS {
        let (surname, given_names) = split_viz(v.viz);
        let lines = format_td3(&Td3Fields {
            document_code: "PP".into(),
            issuing_country: "UTO".into(),
            surname: surname.into(),
            given_names: given_names.into(),
            ..Default::default()
        });
        let line1 = lines
            .split_once('\n')
            .expect("format_td3 emits two lines")
            .0;
        assert_eq!(line1, v.expected_line, "{}: VIZ {:?}", v.source, v.viz);
    }

    for v in TD1_NAME_VECTORS {
        let (surname, given_names) = split_viz(v.viz);
        let lines = format_td1(&Td1Fields {
            surname: surname.into(),
            given_names: given_names.into(),
            ..Default::default()
        });
        let line3 = lines
            .rsplit_once('\n')
            .expect("format_td1 emits three lines")
            .1;
        assert_eq!(line3, v.expected_line, "{}: VIZ {:?}", v.source, v.viz);
    }

    for v in TD2_NAME_VECTORS {
        let (surname, given_names) = split_viz(v.viz);
        let lines = format_td2(&Td2Fields {
            document_code: "I".into(),
            issuing_country: "UTO".into(),
            surname: surname.into(),
            given_names: given_names.into(),
            ..Default::default()
        });
        let line1 = lines
            .split_once('\n')
            .expect("format_td2 emits two lines")
            .0;
        assert_eq!(line1, v.expected_line, "{}: VIZ {:?}", v.source, v.viz);
    }

    for v in MRV_A_NAME_VECTORS {
        let (surname, given_names) = split_viz(v.viz);
        let lines = format_mrv_a(&MrvAFields {
            issuing_country: "UTO".into(),
            surname: surname.into(),
            given_names: given_names.into(),
            ..Default::default()
        });
        let line1 = lines
            .split_once('\n')
            .expect("format_mrv_a emits two lines")
            .0;
        assert_eq!(line1, v.expected_line, "{}: VIZ {:?}", v.source, v.viz);
    }

    for v in MRV_B_NAME_VECTORS {
        let (surname, given_names) = split_viz(v.viz);
        let lines = format_mrv_b(&MrvBFields {
            issuing_country: "UTO".into(),
            surname: surname.into(),
            given_names: given_names.into(),
            ..Default::default()
        });
        let line1 = lines
            .split_once('\n')
            .expect("format_mrv_b emits two lines")
            .0;
        assert_eq!(line1, v.expected_line, "{}: VIZ {:?}", v.source, v.viz);
    }
}

// ---- Part 3 §6 Table A transliteration (S4) ----

/// `Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:1572-1574` (Appendix B,
/// informative): "the name in a European national script: Térèsa CAÑON and
/// the transliteration into the MRZ: `CANXXON<<TERESA`".
///
/// This is the `XxSuffix` style (`Ñ`→`NXX`), not the `Expanded` style the
/// emit path wires in by default (`Ñ`→`N`), so it is built here directly
/// from [`mrz::transliterate`] plus the same `<<`-join/pad-to-width §4.6
/// applies, rather than through `format_td3` (which is Expanded-only).
#[test]
fn xxsuffix_golden_vector_teresa_canon() {
    use mrz::{transliterate, TransliterationStyle};

    let surname = transliterate("CAÑON", TransliterationStyle::XxSuffix);
    let given_names = transliterate("Térèsa", TransliterationStyle::XxSuffix);
    assert_eq!(surname, "CANXXON");
    assert_eq!(given_names, "TERESA");

    let combined = format!("{surname}<<{given_names}");
    assert_eq!(combined, "CANXXON<<TERESA");

    let mut padded = combined.clone();
    while padded.len() < 39 {
        padded.push('<');
    }
    assert_eq!(padded.len(), 39);
    assert!(padded.starts_with(&combined));
    assert!(padded[combined.len()..].chars().all(|c| c == '<'));
}

/// Emit-path regression: `clean_name_half` now transliterates Table A
/// characters (`Expanded` style) instead of silently dropping them as
/// "other punctuation". Exercises the real `format_td3` path end-to-end.
#[test]
fn emit_transliterates_table_a_characters_expanded() {
    let cases: &[(&str, &str)] = &[
        ("MÜLLER", "MUELLER"),
        ("Térèsa", "TERESA"),
        ("GROß", "GROSS"),
        ("Øst", "OEST"),
        ("Škoda", "SKODA"),
        ("CAÑON", "CANON"),
    ];
    for (surname, expected) in cases {
        let mrz = format_td3(&Td3Fields {
            surname: (*surname).into(),
            ..Default::default()
        });
        let line1 = mrz.split_once('\n').unwrap().0;
        // Name field starts at offset 5 (doc code 2 + issuing country 3).
        let name_field = &line1[5..];
        let expected_field: String = {
            let mut s = expected.to_string();
            while s.len() < 39 {
                s.push('<');
            }
            s
        };
        assert_eq!(
            name_field, expected_field,
            "surname {surname:?} should transliterate to {expected:?}"
        );
    }
}
