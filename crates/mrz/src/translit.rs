//! ICAO 9303 Part 3 §6 A: transliteration of multinational Latin-based
//! characters to the OCR-B-safe `[A-Z]` MRZ alphabet
//! (`knowledge/docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:703-807`).
//!
//! This module can *produce* a conformant transliteration but cannot
//! *validate* one: the standard admits several correct answers for five code
//! points (see [`TransliterationStyle`]) and the issuing State chooses among
//! them, so there is no single "the" transliteration of a name to check
//! against.
//!
//! §6 B (Cyrillic) and §6 C (Arabic) are **not** implemented here and are
//! deferred to a separate program.

/// ICAO Doc 9303 Part 3 Section 6 Table A, sorted ascending by code point.
/// Variants appear in the order Doc 9303 lists them.
/// Source: knowledge/docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:707-807
const TABLE_A: &[(char, &[&str])] = &[
    ('\u{00C0}', &["A"]),              // À  A grave
    ('\u{00C1}', &["A"]),              // Á  A acute
    ('\u{00C2}', &["A"]),              // Â  A circumflex
    ('\u{00C3}', &["A"]),              // Ã  A tilde
    ('\u{00C4}', &["AE", "A"]),        // Ä  A diaeresis
    ('\u{00C5}', &["AA", "A"]),        // Å  A ring above
    ('\u{00C6}', &["AE"]),             // Æ  ligature AE
    ('\u{00C7}', &["C"]),              // Ç  C cedilla
    ('\u{00C8}', &["E"]),              // È  E grave
    ('\u{00C9}', &["E"]),              // É  E acute
    ('\u{00CA}', &["E"]),              // Ê  E circumflex
    ('\u{00CB}', &["E"]),              // Ë  E diaeresis
    ('\u{00CC}', &["I"]),              // Ì  I grave
    ('\u{00CD}', &["I"]),              // Í  I acute
    ('\u{00CE}', &["I"]),              // Î  I circumflex
    ('\u{00CF}', &["I"]),              // Ï  I diaeresis
    ('\u{00D0}', &["D"]),              // Ð  Eth
    ('\u{00D1}', &["N", "NXX"]),       // Ñ  N tilde
    ('\u{00D2}', &["O"]),              // Ò  O grave
    ('\u{00D3}', &["O"]),              // Ó  O acute
    ('\u{00D4}', &["O"]),              // Ô  O circumflex
    ('\u{00D5}', &["O"]),              // Õ  O tilde
    ('\u{00D6}', &["OE", "O"]),        // Ö  O diaeresis
    ('\u{00D8}', &["OE"]),             // Ø  O stroke
    ('\u{00D9}', &["U"]),              // Ù  U grave
    ('\u{00DA}', &["U"]),              // Ú  U acute
    ('\u{00DB}', &["U"]),              // Û  U circumflex
    ('\u{00DC}', &["UE", "UXX", "U"]), // Ü  U diaeresis
    ('\u{00DD}', &["Y"]),              // Ý  Y acute
    ('\u{00DE}', &["TH"]),             // Þ  Thorn (Iceland)
    ('\u{0100}', &["A"]),              // Ā  A macron
    ('\u{0102}', &["A"]),              // Ă  A breve
    ('\u{0104}', &["A"]),              // Ą  A ogonek
    ('\u{0106}', &["C"]),              // Ć  C acute
    ('\u{0108}', &["C"]),              // Ĉ  C circumflex
    ('\u{010A}', &["C"]),              // Ċ  C dot above
    ('\u{010C}', &["C"]),              // Č  C caron
    ('\u{010E}', &["D"]),              // Ď  D caron
    ('\u{0110}', &["D"]),              // Đ  D stroke
    ('\u{0112}', &["E"]),              // Ē  E macron
    ('\u{0114}', &["E"]),              // Ĕ  E breve
    ('\u{0116}', &["E"]),              // Ė  E dot above
    ('\u{0118}', &["E"]),              // Ę  E ogonek
    ('\u{011A}', &["E"]),              // Ě  E caron
    ('\u{011C}', &["G"]),              // Ĝ  G circumflex
    ('\u{011E}', &["G"]),              // Ğ  G breve
    ('\u{0120}', &["G"]),              // Ġ  G dot above
    ('\u{0122}', &["G"]),              // Ģ  G cedilla
    ('\u{0124}', &["H"]),              // Ĥ  H circumflex
    ('\u{0126}', &["H"]),              // Ħ  H stroke
    ('\u{0128}', &["I"]),              // Ĩ  I tilde
    ('\u{012A}', &["I"]),              // Ī  I macron
    ('\u{012C}', &["I"]),              // Ĭ  I breve
    ('\u{012E}', &["I"]),              // Į  I ogonek
    ('\u{0130}', &["I"]),              // İ  I dot above
    ('\u{0131}', &["I"]),              // ı  I without dot (Turkey)
    ('\u{0132}', &["IJ"]),             // Ĳ  ligature IJ
    ('\u{0134}', &["J"]),              // Ĵ  J circumflex
    ('\u{0136}', &["K"]),              // Ķ  K cedilla
    ('\u{0139}', &["L"]),              // Ĺ  L acute
    ('\u{013B}', &["L"]),              // Ļ  L cedilla
    ('\u{013D}', &["L"]),              // Ľ  L caron
    ('\u{013F}', &["L"]),              // Ŀ  L middle dot
    ('\u{0141}', &["L"]),              // Ł  L stroke
    ('\u{0143}', &["N"]),              // Ń  N acute
    ('\u{0145}', &["N"]),              // Ņ  N cedilla
    ('\u{0147}', &["N"]),              // Ň  N caron
    ('\u{014A}', &["N"]),              // Ŋ  Eng
    ('\u{014C}', &["O"]),              // Ō  O macron
    ('\u{014E}', &["O"]),              // Ŏ  O breve
    ('\u{0150}', &["O"]),              // Ő  O double acute
    ('\u{0152}', &["OE"]),             // Œ  ligature OE
    ('\u{0154}', &["R"]),              // Ŕ  R acute
    ('\u{0156}', &["R"]),              // Ŗ  R cedilla
    ('\u{0158}', &["R"]),              // Ř  R caron
    ('\u{015A}', &["S"]),              // Ś  S acute
    ('\u{015C}', &["S"]),              // Ŝ  S circumflex
    ('\u{015E}', &["S"]),              // Ş  S cedilla
    ('\u{0160}', &["S"]),              // Š  S caron
    ('\u{0162}', &["T"]),              // Ţ  T cedilla
    ('\u{0164}', &["T"]),              // Ť  T caron
    ('\u{0166}', &["T"]),              // Ŧ  T stroke
    ('\u{0168}', &["U"]),              // Ũ  U tilde
    ('\u{016A}', &["U"]),              // Ū  U macron
    ('\u{016C}', &["U"]),              // Ŭ  U breve
    ('\u{016E}', &["U"]),              // Ů  U ring above
    ('\u{0170}', &["U"]),              // Ű  U double acute
    ('\u{0172}', &["U"]),              // Ų  U ogonek
    ('\u{0174}', &["W"]),              // Ŵ  W circumflex
    ('\u{0176}', &["Y"]),              // Ŷ  Y circumflex
    ('\u{0178}', &["Y"]),              // Ÿ  Y diaeresis
    ('\u{0179}', &["Z"]),              // Ź  Z acute
    ('\u{017B}', &["Z"]),              // Ż  Z dot above
    ('\u{017D}', &["Z"]),              // Ž  Z caron
    ('\u{1E9E}', &["SS"]),             // ẞ  double s (Germany)
];

/// Which of Doc 9303's alternative recommendations to use where Table A
/// offers more than one.
///
/// Only five Table A code points are ambiguous — Doc 9303 lists more than
/// one recommended transliteration for each and leaves the choice to the
/// issuing State:
///
/// | cp | `Expanded` | `Simple` | `XxSuffix` |
/// |---|---|---|---|
/// | U+00C4 Ä | `AE` | `A` | `AE` |
/// | U+00C5 Å | `AA` | `A` | `AA` |
/// | U+00D1 Ñ | `N` | `N` | `NXX` |
/// | U+00D6 Ö | `OE` | `O` | `OE` |
/// | U+00DC Ü | `UE` | `U` | `UXX` |
///
/// Every value in that table is one ICAO actually lists (`:703-807`); the
/// *grouping into these three named styles* is this crate's own organisation
/// of a choice ICAO leaves open, not something Doc 9303 names.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum TransliterationStyle {
    /// The digraph/doubled-letter form: `Ä`→`AE`, `Å`→`AA`, `Ñ`→`N`, `Ö`→`OE`,
    /// `Ü`→`UE`. This is `variants[0]` for all 95 Table A rows.
    #[default]
    Expanded,
    /// The bare-base-letter form: `Ä`→`A`, `Å`→`A`, `Ñ`→`N`, `Ö`→`O`, `Ü`→`U`.
    Simple,
    /// The `XX`-suffixed form Doc 9303's Appendix B worked example uses for
    /// `Ñ`→`NXX` and `Ü`→`UXX`, otherwise matching `Expanded`
    /// (`Ä`→`AE`, `Å`→`AA`, `Ö`→`OE`).
    XxSuffix,
}

/// Every ICAO-recommended transliteration for `c`, in the order Doc 9303
/// lists them. Empty if `c` is not in Table A.
pub fn transliterations(c: char) -> &'static [&'static str] {
    match TABLE_A.binary_search_by_key(&c, |&(cp, _)| cp) {
        Ok(idx) => TABLE_A[idx].1,
        Err(_) => &[],
    }
}

/// The single transliteration for `c` under `style`, or `None` if `c` is not
/// in Table A.
pub fn transliterate_char(c: char, style: TransliterationStyle) -> Option<&'static str> {
    let variants = transliterations(c);
    if variants.is_empty() {
        return None;
    }
    // Explicit match on the five ambiguous code points, so the data table
    // (TABLE_A) and this policy stay visibly separate. Falls back to
    // `variants[0]` for everything else.
    let selected = match (c, style) {
        ('\u{00C4}', TransliterationStyle::Simple) => "A",
        ('\u{00C4}', _) => "AE",
        ('\u{00C5}', TransliterationStyle::Simple) => "A",
        ('\u{00C5}', _) => "AA",
        ('\u{00D1}', TransliterationStyle::XxSuffix) => "NXX",
        ('\u{00D1}', _) => "N",
        ('\u{00D6}', TransliterationStyle::Simple) => "O",
        ('\u{00D6}', _) => "OE",
        ('\u{00DC}', TransliterationStyle::Simple) => "U",
        ('\u{00DC}', TransliterationStyle::XxSuffix) => "UXX",
        ('\u{00DC}', _) => "UE",
        _ => variants[0],
    };
    Some(selected)
}

/// Uppercase `s` and replace every Table A character with its `style`
/// transliteration. Characters outside Table A pass through **unchanged**
/// (uppercased). This performs no MRZ filler handling and no truncation.
///
/// Five of Table A's 95 characters have **more than one** ICAO-recommended
/// transliteration, and the issuing State chooses among them. This function
/// exposes that choice through `style` rather than pretending to a single
/// canonical answer — which is why the crate can *produce* a conformant
/// transliteration but cannot *validate* one: the standard admits several
/// correct answers for the same input, and nothing in the MRZ records which
/// the issuer picked.
///
/// Only Latin-based characters (§6 A) are implemented. Cyrillic (§6 B) and
/// Arabic (§6 C) are out of scope.
///
/// ```
/// use mrz::{transliterate, TransliterationStyle};
///
/// // ICAO 9303 Part 3 Appendix B's own worked example.
/// assert_eq!(transliterate("Térèsa", TransliterationStyle::XxSuffix), "TERESA");
/// assert_eq!(transliterate("CAÑON", TransliterationStyle::XxSuffix), "CANXXON");
///
/// // The multi-valued cells: same input, two conformant answers.
/// assert_eq!(transliterate("Ñ", TransliterationStyle::Expanded), "N");
/// assert_eq!(transliterate("Ñ", TransliterationStyle::XxSuffix), "NXX");
/// assert_eq!(transliterate("ß", TransliterationStyle::Simple), "SS"); // only one ICAO answer
/// ```
pub fn transliterate(s: &str, style: TransliterationStyle) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        for u in c.to_uppercase() {
            match transliterate_char(u, style) {
                Some(t) => out.push_str(t),
                None => out.push(u),
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_95_entries() {
        assert_eq!(TABLE_A.len(), 95);
    }

    #[test]
    fn table_strictly_ascending_no_duplicates() {
        for pair in TABLE_A.windows(2) {
            assert!(
                pair[0].0 < pair[1].0,
                "not strictly ascending at {:?}, {:?}",
                pair[0].0,
                pair[1].0
            );
        }
    }

    #[test]
    fn every_variant_nonempty_and_uppercase_alpha() {
        for &(cp, variants) in TABLE_A {
            assert!(!variants.is_empty(), "{cp:?} has no variants");
            for v in variants {
                assert!(!v.is_empty(), "{cp:?} has an empty variant");
                assert!(
                    v.chars().all(|c| c.is_ascii_uppercase()),
                    "{cp:?} variant {v:?} is not [A-Z]+"
                );
            }
        }
    }

    #[test]
    fn no_key_is_ascii() {
        for &(cp, _) in TABLE_A {
            assert!(!cp.is_ascii(), "{cp:?} should not be ASCII");
        }
    }

    #[test]
    fn expanded_is_always_variants_zero() {
        for &(cp, variants) in TABLE_A {
            assert_eq!(
                transliterate_char(cp, TransliterationStyle::Expanded),
                Some(variants[0]),
                "Expanded mismatch at {cp:?}"
            );
        }
    }

    #[test]
    fn multi_valued_set_is_exactly_five() {
        let multi: Vec<char> = TABLE_A
            .iter()
            .filter(|&&(_, v)| v.len() > 1)
            .map(|&(cp, _)| cp)
            .collect();
        assert_eq!(
            multi,
            vec!['\u{00C4}', '\u{00C5}', '\u{00D1}', '\u{00D6}', '\u{00DC}']
        );
    }

    #[test]
    fn lowercase_round_trip() {
        // The invariant the emit path's uppercase-first design actually
        // relies on is end-to-end: `transliterate` of a table char's
        // lowercase counterpart must equal `transliterate` of the char
        // itself. That's a weaker (and more accurate) claim than "the
        // uppercased lowercase form is itself a Table A key" — two rows
        // reach their correct output via the ASCII-passthrough branch in
        // `transliterate`, not via a second table hit:
        //
        // - U+0131 (ı, dotless i) is *already* lowercase per Unicode, and
        //   `'ı'.to_uppercase()` yields ASCII `I`, which is not itself a
        //   Table A key (no key is ASCII) — `I` reaches its output by
        //   passing straight through `transliterate`'s ASCII-alphanumeric
        //   branch. The brief's own VERIFIED section calls this out
        //   explicitly: "ı uppercases to ASCII I; the table also maps
        //   U+0131 → I, so either path gives I. No conflict."
        // - U+0130 (İ) is a genuine, confirmed exception, skipped below.
        //   It has no single-char simple lowercase mapping; Rust's
        //   `char::to_lowercase` gives the Unicode *full* mapping instead:
        //   the two-char sequence `i` + COMBINING DOT ABOVE (U+0307). This
        //   module's own [`transliterate`] leaves characters outside Table A
        //   **unchanged** (by contract — see its doc comment), so U+0307
        //   survives verbatim: `transliterate("i\u{307}")` = `"I\u{0307}"`,
        //   not `"I"`. Only `clean_name_half` in `emit.rs` (which drops
        //   unrecognized punctuation entirely) collapses that back to a
        //   match with İ's own Table A output. So this is a real exception
        //   to the brief's VERIFIED claim ("every lowercase counterpart
        //   uppercases to a char in the table") under both readings tried
        //   here — table membership and this module's own `transliterate`
        //   — and is harmless only because `clean_name_half`, one layer up,
        //   happens to discard the leftover combining mark.
        for &(cp, _) in TABLE_A {
            if cp == '\u{0130}' {
                continue;
            }
            let upper_str = cp.to_string();
            let lower_str: String = cp.to_lowercase().collect();
            assert_eq!(
                transliterate(&lower_str, TransliterationStyle::Expanded),
                transliterate(&upper_str, TransliterationStyle::Expanded),
                "lowercase of {cp:?} ({lower_str:?}) does not round-trip"
            );
        }
    }

    #[test]
    fn sharp_s_uppercases_to_ss() {
        assert_eq!(transliterate("ß", TransliterationStyle::Expanded), "SS");
        assert_eq!(transliterate("ß", TransliterationStyle::Simple), "SS");
        assert_eq!(transliterate("ß", TransliterationStyle::XxSuffix), "SS");
        // Confirmed via Table A's own U+1E9E ẞ row.
        assert_eq!(
            transliterate_char('\u{1E9E}', TransliterationStyle::Expanded),
            Some("SS")
        );
    }

    #[test]
    fn golden_vector_teresa_canon() {
        // Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:1572-1574 (Appendix B,
        // informative): "Térèsa CAÑON" -> "CANXXON<<TERESA".
        assert_eq!(
            transliterate("Térèsa", TransliterationStyle::XxSuffix),
            "TERESA"
        );
        assert_eq!(
            transliterate("CAÑON", TransliterationStyle::XxSuffix),
            "CANXXON"
        );
    }

    #[test]
    fn not_in_table_returns_empty_and_none() {
        assert_eq!(transliterations('A'), &[] as &[&str]);
        assert_eq!(
            transliterate_char('A', TransliterationStyle::Expanded),
            None
        );
        assert_eq!(transliterate("A1!", TransliterationStyle::Expanded), "A1!");
    }
}
