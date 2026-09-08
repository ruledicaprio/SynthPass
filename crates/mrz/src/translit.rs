//! ICAO 9303 Part 3 §6 transliteration of national characters to the OCR-B-safe
//! `[A-Z]` MRZ alphabet
//! (`knowledge/docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:703-863`).
//!
//! - **§6 A — multinational Latin-based characters** ([`transliterate`],
//!   [`TransliterationStyle`]): `Ä`→`AE`, `Ø`→`OE`, `ß`→`SS` …
//! - **§6 B — Cyrillic characters** ([`transliterate_cyrillic`],
//!   [`CyrillicLanguage`]): `Иванов`→`IVANOV`. Twelve of the 48 rows and five
//!   positional rules depend on the name's language, so this half needs a
//!   [`CyrillicLanguage`] where §6 A needs only a style.
//!
//! This module can *produce* a conformant transliteration but cannot
//! *validate* one: §6 A admits several correct answers for five code points
//! (the issuing State chooses among them — see [`TransliterationStyle`]), and
//! §6 B's digraph outputs (`ZH`, `SHCH`, …) are not reversible. So there is no
//! single "the" transliteration of a name to check against.
//!
//! §6 C (Arabic) is **not** implemented here and is deferred to a separate
//! program.

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

// ---------------------------------------------------------------------------
// §6 B — Cyrillic
// ---------------------------------------------------------------------------

/// Which language's ICAO 9303 Part 3 §6 B column to apply.
///
/// Unlike §6 A — where the issuing State freely picks one of several equal
/// forms ([`TransliterationStyle`]) — §6 B makes twelve of its 48 rows and
/// five positional rules depend on the *language* the name is written in
/// (`Г`→`G` in Russian but `H` in Serbian; `Щ`→`SHCH` but `SHT` in Bulgarian).
/// The other 31 rows are the same for every language.
///
/// Doc 9303 spells [`Belarusian`](Self::Belarusian) "Belorussian" and never
/// names [`Macedonian`](Self::Macedonian) directly ("the language spoken in
/// the former Yugoslav Republic of Macedonia"); this enum uses the modern
/// short names. The default, [`Russian`](Self::Russian), is the table's
/// unnamed base column.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum CyrillicLanguage {
    /// The table's unnamed base column (effectively Russian).
    #[default]
    Russian,
    /// Spelled "Belorussian" in Doc 9303 §6 B.
    Belarusian,
    /// Bulgarian — the only column that maps `Щ` to `SHT` rather than `SHCH`.
    Bulgarian,
    /// Serbian — drops the digraphs (`Ж`→`Z`, `Ч`→`C`, `Ш`→`S`, `Х`→`H`, …).
    Serbian,
    /// Ukrainian — `И`→`Y`, `Г`→`H`, and the five word-initial `Y`- forms.
    Ukrainian,
    /// "The language spoken in the former Yugoslav Republic of Macedonia" in
    /// Doc 9303 §6 B.
    Macedonian,
}

/// ICAO Doc 9303 Part 3 §6 B: the 48 Cyrillic code points and their **base**
/// (unnamed column ≈ Russian) transliteration, sorted ascending by code point.
///
/// The twelve language-conditional rows and the five "Ukrainian first
/// character" positional rows carry their base form here; the exceptions live
/// in [`transliterate_cyrillic_char`], keeping the data table and the policy
/// visibly separate — the same split [`transliterate_char`] uses for §6 A's
/// five ambiguous rows.
///
/// Keyed on the source table's authoritative `Unicode` column. Its
/// `National character` glyph column carries three transcription slips that
/// this table is unaffected by: U+0402 is printed as `Ћ` (Tshe) but U+0402 is
/// `Ђ` (Dje, → `D`); U+040E is printed lowercase; U+0474 is printed as Latin
/// `V` but is `Ѵ` (Izhitsa). See `knowledge/docs9303/CONFORMANCE_BASIS.md`.
///
/// Source: knowledge/docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:809-863
const TABLE_B: &[(char, &str)] = &[
    ('\u{0401}', "E"),    // Ё   Belarusian = IO
    ('\u{0402}', "D"),    // Ђ   (printed Ћ — glyph slip; U+0402 is Dje)
    ('\u{0404}', "IE"),   // Є   Ukrainian first character = YE
    ('\u{0405}', "DZ"),   // Ѕ
    ('\u{0406}', "I"),    // І
    ('\u{0407}', "I"),    // Ї   Ukrainian first character = YI
    ('\u{0408}', "J"),    // Ј
    ('\u{0409}', "LJ"),   // Љ
    ('\u{040A}', "NJ"),   // Њ
    ('\u{040C}', "K"),    // Ќ   Macedonian = KJ
    ('\u{040E}', "U"),    // Ў   (printed lowercase — glyph slip)
    ('\u{040F}', "DZ"),   // Џ   Macedonian = DJ
    ('\u{0410}', "A"),    // А
    ('\u{0411}', "B"),    // Б
    ('\u{0412}', "V"),    // В
    ('\u{0413}', "G"),    // Г   Belarusian, Serbian, Ukrainian = H
    ('\u{0414}', "D"),    // Д
    ('\u{0415}', "E"),    // Е
    ('\u{0416}', "ZH"),   // Ж   Serbian = Z
    ('\u{0417}', "Z"),    // З
    ('\u{0418}', "I"),    // И   Ukrainian = Y
    ('\u{0419}', "I"),    // Й   Ukrainian first character = Y
    ('\u{041A}', "K"),    // К
    ('\u{041B}', "L"),    // Л
    ('\u{041C}', "M"),    // М
    ('\u{041D}', "N"),    // Н
    ('\u{041E}', "O"),    // О
    ('\u{041F}', "P"),    // П
    ('\u{0420}', "R"),    // Р
    ('\u{0421}', "S"),    // С
    ('\u{0422}', "T"),    // Т
    ('\u{0423}', "U"),    // У
    ('\u{0424}', "F"),    // Ф
    ('\u{0425}', "KH"),   // Х   Serbian, Macedonian = H
    ('\u{0426}', "TS"),   // Ц   Serbian, Macedonian = C
    ('\u{0427}', "CH"),   // Ч   Serbian = C
    ('\u{0428}', "SH"),   // Ш   Serbian = S
    ('\u{0429}', "SHCH"), // Щ   Bulgarian = SHT
    ('\u{042A}', "IE"),   // Ъ
    ('\u{042B}', "Y"),    // Ы
    ('\u{042D}', "E"),    // Э
    ('\u{042E}', "IU"),   // Ю   Ukrainian first character = YU
    ('\u{042F}', "IA"),   // Я   Ukrainian first character = YA
    ('\u{046A}', "U"),    // Ѫ
    ('\u{0474}', "Y"),    // Ѵ   (printed Latin V — glyph slip; U+0474 is Izhitsa)
    ('\u{0490}', "G"),    // Ґ
    ('\u{0492}', "G"),    // Ғ   Macedonian = GJ
    ('\u{04BA}', "C"),    // Һ
];

/// The §6 B transliteration of a single **upper-case** Cyrillic character, or
/// `None` if `c` is not one of the 48 §6 B code points. `lang` selects the
/// twelve language-conditional rows; `is_first` (whether `c` is the first character
/// of the name) applies the five "Ukrainian first character" positional rows.
///
/// Callers that have a whole string should use [`transliterate_cyrillic`],
/// which upper-cases and tracks `is_first` for you.
pub fn transliterate_cyrillic_char(
    c: char,
    lang: CyrillicLanguage,
    is_first: bool,
) -> Option<&'static str> {
    let base = match TABLE_B.binary_search_by_key(&c, |&(cp, _)| cp) {
        Ok(idx) => TABLE_B[idx].1,
        Err(_) => return None,
    };
    use CyrillicLanguage::*;
    // Explicit match on the 17 conditional code points, so TABLE_B (data) and
    // this policy stay visibly separate. Everything else is `base`.
    let selected = match c {
        // Language-conditional (12).
        '\u{0401}' if lang == Belarusian => "IO",
        '\u{040C}' if lang == Macedonian => "KJ",
        '\u{040F}' if lang == Macedonian => "DJ",
        '\u{0413}' if matches!(lang, Belarusian | Serbian | Ukrainian) => "H",
        '\u{0416}' if lang == Serbian => "Z",
        '\u{0418}' if lang == Ukrainian => "Y",
        '\u{0425}' if matches!(lang, Serbian | Macedonian) => "H",
        '\u{0426}' if matches!(lang, Serbian | Macedonian) => "C",
        '\u{0427}' if lang == Serbian => "C",
        '\u{0428}' if lang == Serbian => "S",
        '\u{0429}' if lang == Bulgarian => "SHT",
        '\u{0492}' if lang == Macedonian => "GJ",
        // Positional — "except if Ukrainian first character" (5).
        '\u{0404}' if lang == Ukrainian && is_first => "YE",
        '\u{0407}' if lang == Ukrainian && is_first => "YI",
        '\u{0419}' if lang == Ukrainian && is_first => "Y",
        '\u{042E}' if lang == Ukrainian && is_first => "YU",
        '\u{042F}' if lang == Ukrainian && is_first => "YA",
        _ => base,
    };
    Some(selected)
}

/// Upper-case `s` and transliterate every Cyrillic character per ICAO 9303
/// Part 3 §6 B, using `lang`'s column. The first character of `s` is the
/// "first character" for the five Ukrainian positional rows.
///
/// Non-Cyrillic characters (Latin included) pass through **unchanged**
/// (upper-cased) — §6 A's [`transliterate`] is *not* also applied here.
/// Compose the two if a name genuinely mixes scripts. This performs no MRZ
/// filler handling and no truncation (see [`transliterate`]).
///
/// Like [`transliterate`], this *produces* a conformant transliteration but
/// cannot *validate* one — several §6 B outputs are multi-letter digraphs
/// with no `X`-escape, so the mapping is not reversible.
///
/// ```
/// use mrz::{transliterate_cyrillic, CyrillicLanguage};
///
/// assert_eq!(transliterate_cyrillic("Иванов", CyrillicLanguage::Russian), "IVANOV");
/// // Serbian excepts Ж (ZH → Z) and Ч (CH → C).
/// assert_eq!(transliterate_cyrillic("Живков", CyrillicLanguage::Serbian), "ZIVKOV");
/// // Ukrainian: Г → H everywhere, Є → YE only as the first character.
/// assert_eq!(transliterate_cyrillic("Євген", CyrillicLanguage::Ukrainian), "YEVHEN");
/// ```
pub fn transliterate_cyrillic(s: &str, lang: CyrillicLanguage) -> String {
    let mut out = String::with_capacity(s.len());
    let mut is_first = true;
    for c in s.chars() {
        for u in c.to_uppercase() {
            match transliterate_cyrillic_char(u, lang, is_first) {
                Some(t) => out.push_str(t),
                None => out.push(u),
            }
            is_first = false;
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

    // ---- Part 3 §6 B — Cyrillic ------------------------------------------

    /// Every `CyrillicLanguage`, for the exhaustiveness-style checks below.
    const LANGS: &[CyrillicLanguage] = &[
        CyrillicLanguage::Russian,
        CyrillicLanguage::Belarusian,
        CyrillicLanguage::Bulgarian,
        CyrillicLanguage::Serbian,
        CyrillicLanguage::Ukrainian,
        CyrillicLanguage::Macedonian,
    ];

    #[test]
    fn table_b_has_48_entries() {
        assert_eq!(TABLE_B.len(), 48);
    }

    #[test]
    fn table_b_strictly_ascending_no_duplicates() {
        for pair in TABLE_B.windows(2) {
            assert!(
                pair[0].0 < pair[1].0,
                "not strictly ascending at {:?}, {:?}",
                pair[0].0,
                pair[1].0
            );
        }
    }

    #[test]
    fn table_b_keys_are_cyrillic_block() {
        for &(cp, _) in TABLE_B {
            assert!(!cp.is_ascii(), "{cp:?} should not be ASCII");
            let n = cp as u32;
            assert!(
                (0x0400..=0x04FF).contains(&n),
                "{cp:?} (U+{n:04X}) is outside the Cyrillic block"
            );
        }
    }

    #[test]
    fn table_b_base_output_is_nonempty_upper_ascii() {
        for &(cp, base) in TABLE_B {
            assert!(!base.is_empty(), "{cp:?} has an empty base transliteration");
            assert!(
                base.chars().all(|c| c.is_ascii_uppercase()),
                "{cp:?} base {base:?} is not [A-Z]+"
            );
        }
    }

    #[test]
    fn every_conditional_output_is_upper_ascii() {
        for &(cp, _) in TABLE_B {
            for &lang in LANGS {
                for is_first in [false, true] {
                    let out = transliterate_cyrillic_char(cp, lang, is_first)
                        .expect("every TABLE_B key resolves for every language");
                    assert!(
                        !out.is_empty() && out.chars().all(|c| c.is_ascii_uppercase()),
                        "{cp:?} / {lang:?} / first={is_first} → {out:?} is not [A-Z]+"
                    );
                }
            }
        }
    }

    /// The exact set of code points whose output ever differs from their
    /// `TABLE_B` base — the 12 language-conditional plus the 5
    /// "Ukrainian first character" positional rows = 17.
    #[test]
    fn conditional_set_is_exactly_the_seventeen_documented_rows() {
        let mut conditional: Vec<char> = TABLE_B
            .iter()
            .filter(|&&(cp, base)| {
                LANGS.iter().any(|&lang| {
                    [false, true]
                        .iter()
                        .any(|&f| transliterate_cyrillic_char(cp, lang, f) != Some(base))
                })
            })
            .map(|&(cp, _)| cp)
            .collect();
        conditional.sort();
        assert_eq!(
            conditional,
            vec![
                '\u{0401}', // Ё  Belarusian
                '\u{0404}', // Є  Ukrainian first
                '\u{0407}', // Ї  Ukrainian first
                '\u{040C}', // Ќ  Macedonian
                '\u{040F}', // Џ  Macedonian
                '\u{0413}', // Г  Belarusian/Serbian/Ukrainian
                '\u{0416}', // Ж  Serbian
                '\u{0418}', // И  Ukrainian
                '\u{0419}', // Й  Ukrainian first
                '\u{0425}', // Х  Serbian/Macedonian
                '\u{0426}', // Ц  Serbian/Macedonian
                '\u{0427}', // Ч  Serbian
                '\u{0428}', // Ш  Serbian
                '\u{0429}', // Щ  Bulgarian
                '\u{042E}', // Ю  Ukrainian first
                '\u{042F}', // Я  Ukrainian first
                '\u{0492}', // Ғ  Macedonian
            ]
        );
        assert_eq!(conditional.len(), 17);
    }

    #[test]
    fn cyrillic_lowercase_round_trips() {
        for &(cp, _) in TABLE_B {
            let upper = cp.to_string();
            let lower: String = cp.to_lowercase().collect();
            for &lang in LANGS {
                assert_eq!(
                    transliterate_cyrillic(&lower, lang),
                    transliterate_cyrillic(&upper, lang),
                    "lowercase of {cp:?} does not round-trip for {lang:?}"
                );
            }
        }
    }

    #[test]
    fn cyrillic_non_table_chars_pass_through() {
        // Latin is not touched by the Cyrillic path.
        assert_eq!(
            transliterate_cyrillic("ABC-1", CyrillicLanguage::Russian),
            "ABC-1"
        );
        // A Cyrillic letter not in §6 B's table (Ѱ, U+0470) passes through.
        assert_eq!(
            transliterate_cyrillic("\u{0470}", CyrillicLanguage::Russian),
            "\u{0470}"
        );
        assert_eq!(
            transliterate_cyrillic_char('A', CyrillicLanguage::Russian, false),
            None
        );
    }

    #[test]
    fn cyrillic_base_column_vectors() {
        let cases: &[(&str, &str)] = &[
            ("Иванов", "IVANOV"),
            ("Смирнова", "SMIRNOVA"),
            ("Жуков", "ZHUKOV"),     // Ж → ZH (base)
            ("Щедрин", "SHCHEDRIN"), // Щ → SHCH (base)
            ("Хабаров", "KHABAROV"), // Х → KH (base)
            ("Цветаева", "TSVETAEVA"),
        ];
        for (native, expected) in cases {
            assert_eq!(
                transliterate_cyrillic(native, CyrillicLanguage::Russian),
                *expected,
                "{native}"
            );
        }
    }

    #[test]
    fn cyrillic_language_conditional_vectors() {
        // Serbian: Ж → Z, Ч → C, Ш → S, Х → H, Ц → C, Г → H.
        assert_eq!(
            transliterate_cyrillic("Живков", CyrillicLanguage::Serbian),
            "ZIVKOV"
        );
        assert_eq!(
            transliterate_cyrillic("Чачак", CyrillicLanguage::Serbian),
            "CACAK"
        );
        // Bulgarian: Щ → SHT (only exception Bulgarian has).
        assert_eq!(
            transliterate_cyrillic("Щерев", CyrillicLanguage::Bulgarian),
            "SHTEREV"
        );
        // Bulgarian does NOT except Х — stays KH per §6 B.
        assert_eq!(
            transliterate_cyrillic("Христов", CyrillicLanguage::Bulgarian),
            "KHRISTOV"
        );
        // Macedonian: Ќ → KJ, Џ → DJ, Ѓ (Ғ U+0492) → GJ, Х → H.
        assert_eq!(
            transliterate_cyrillic("Ќоста", CyrillicLanguage::Macedonian),
            "KJOSTA"
        );
        // Belarusian: Ё → IO, Г → H.
        assert_eq!(
            transliterate_cyrillic("Гончарова", CyrillicLanguage::Belarusian),
            "HONCHAROVA"
        );
    }

    #[test]
    fn cyrillic_ukrainian_positional_vectors() {
        // Є/Ї/Й/Ю/Я take a Y- form ONLY as the first character; И → Y anywhere.
        assert_eq!(
            transliterate_cyrillic("Євген", CyrillicLanguage::Ukrainian),
            "YEVHEN" // Є→YE (first), В→V, Г→H (Ukrainian), Е→E, Н→N
        );
        assert_eq!(
            transliterate_cyrillic("Юрій", CyrillicLanguage::Ukrainian),
            "YURII" // Ю→YU (first), Р→R, І→I, Й→I (not first)
        );
        // Non-first Є stays IE.
        assert_eq!(
            transliterate_cyrillic("Гаєвий", CyrillicLanguage::Ukrainian),
            "HAIEVYI" // Г→H, А→A, Є→IE (not first), В→V, И→Y, Й→I (not first)
        );
        // И → Y for Ukrainian regardless of position; Й → I when not first.
        assert_eq!(
            transliterate_cyrillic("Мирний", CyrillicLanguage::Ukrainian),
            "MYRNYI"
        );
        // The very same string in Russian: И → I, Й → I.
        assert_eq!(
            transliterate_cyrillic("Мирний", CyrillicLanguage::Russian),
            "MIRNII"
        );
    }
}
