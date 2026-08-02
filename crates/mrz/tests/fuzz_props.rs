//! Property-based "never panics on arbitrary input" regression suite.
//!
//! `mrz` is a zero-dependency parser that does raw byte-index surgery in its
//! checksum-verified OCR repair (reinserting dropped fillers, padding short
//! lines) and also compiles to WASM for the public browser demo — a panic
//! here is a real remote DoS in two places at once. This suite runs under
//! `cargo test --workspace` on every PR (both the Linux and macOS CI jobs),
//! giving an always-on regression net. Deeper coverage-guided fuzzing lives
//! in `fuzz/` (opt-in, `cargo fuzz run ...`).
//!
//! These properties assert only that the parser returns `Ok`/`Err` and never
//! panics — they say nothing about correctness (the existing unit tests in
//! `src/lib.rs`/`src/parser.rs` already cover that against real specimens).

use mrz::{
    find_and_parse, format_td1, format_td2, format_td3, parse_mrv_a, parse_mrv_b, parse_td1,
    parse_td2, parse_td3, substitution_candidates, transliterate, transliterations, Td1Fields,
    Td2Fields, Td3Fields, TransliterationStyle,
};
use proptest::prelude::*;

/// The real MRZ charset plus the OCR-noise characters this crate's own unit
/// tests are seeded with (`«`, lowercase, HTML-escaped fillers, stray
/// spaces) — biases the generator toward inputs that actually exercise the
/// repair heuristics instead of bouncing off the character-set check.
fn mrz_ish_char() -> impl Strategy<Value = char> {
    prop_oneof![
        // Valid MRZ charset: the common case, filler weighted heavily since
        // it's what most of a real MRZ line actually consists of.
        8 => prop::char::range('A', 'Z'),
        8 => prop::char::range('0', '9'),
        12 => Just('<'),
        // OCR-noise lookalikes the repair heuristics target.
        3 => prop::char::range('a', 'z'),
        2 => Just('«'),
        2 => Just(' '),
        1 => Just('&'),
        1 => Just(';'),
        1 => Just('l'),
        1 => Just('K'),
        1 => any::<char>(),
    ]
}

/// A string of `mrz_ish_char`s at roughly the given length (± a few, so both
/// exact-length and length-mismatch paths get exercised).
fn near_length_string(target: usize) -> impl Strategy<Value = String> {
    let lo = target.saturating_sub(3);
    let hi = target + 3;
    prop::collection::vec(mrz_ish_char(), lo..=hi).prop_map(|chars| chars.into_iter().collect())
}

/// Any string at all — the most adversarial case for `find_and_parse`, which
/// must scan free text for MRZ-shaped lines.
fn arbitrary_text() -> impl Strategy<Value = String> {
    prop::collection::vec(mrz_ish_char(), 0..200).prop_map(|chars| chars.into_iter().collect())
}

/// Mutates one of the crate's own valid specimen lines at a single random
/// position — reaches the repair heuristics (which assume "almost valid"
/// input) far more often than pure-random strings do.
fn mutated_specimen() -> impl Strategy<Value = String> {
    let specimens = [
        "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".to_string(),
        "L898902C36UTO7408122F1204159ZE184226B<<<<<10".to_string(),
        "I<UTOD231458907<<<<<<<<<<<<<<<".to_string(),
        "7408122F1204159UTO<<<<<<<<<<<6".to_string(),
        "ERIKSSON<<ANNA<MARIA<<<<<<<<<<".to_string(),
        "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<".to_string(),
        "D231458907UTO7408122F1204159<<<<<<<6".to_string(),
        "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<".to_string(),
        "XK93054875BRA8502212F2703143R5T6U7V8W9<<<<<<".to_string(),
        "L234567897DEU9201017F2706306QW12ER34".to_string(),
    ];
    (0..specimens.len()).prop_flat_map(move |i| {
        let base = specimens[i].clone();
        let len = base.len();
        (Just(base), 0..len.max(1), mrz_ish_char()).prop_map(|(mut s, pos, c)| {
            if pos < s.len() {
                let mut chars: Vec<char> = s.chars().collect();
                chars[pos] = c;
                s = chars.into_iter().collect();
            }
            s
        })
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// `find_and_parse` must never panic on arbitrary free text, however
    /// garbled — it always returns `Ok`/`Err`.
    #[test]
    fn find_and_parse_never_panics_on_arbitrary_text(text in arbitrary_text()) {
        let _ = find_and_parse(&text);
    }

    /// Same, but over multi-line text combining several arbitrary strings —
    /// closer to real OCR Markdown output than a single line.
    #[test]
    fn find_and_parse_never_panics_on_multiline_text(
        lines in prop::collection::vec(arbitrary_text(), 0..6)
    ) {
        let text = lines.join("\n");
        let _ = find_and_parse(&text);
    }

    /// `find_and_parse` over a mutated real specimen — reaches the OCR-repair
    /// machinery (dropped/extra fillers, lookalike digits) far more often.
    #[test]
    fn find_and_parse_never_panics_on_mutated_specimen(
        lines in prop::collection::vec(mutated_specimen(), 1..4)
    ) {
        let text = lines.join("\n");
        let _ = find_and_parse(&text);
    }

    /// `parse_td3` must never panic regardless of line length or content.
    #[test]
    fn parse_td3_never_panics(l1 in near_length_string(44), l2 in near_length_string(44)) {
        let _ = parse_td3(&l1, &l2);
    }

    /// `parse_td2` must never panic regardless of line length or content.
    #[test]
    fn parse_td2_never_panics(l1 in near_length_string(36), l2 in near_length_string(36)) {
        let _ = parse_td2(&l1, &l2);
    }

    /// `parse_td1` must never panic regardless of line length or content.
    #[test]
    fn parse_td1_never_panics(
        l1 in near_length_string(30),
        l2 in near_length_string(30),
        l3 in near_length_string(30),
    ) {
        let _ = parse_td1(&l1, &l2, &l3);
    }

    /// Exact-length (no length-mismatch early return) inputs over the full
    /// MRZ-ish charset — the case most likely to reach deep index math.
    #[test]
    fn parse_td3_never_panics_exact_length(
        l1 in prop::collection::vec(mrz_ish_char(), 44).prop_map(|c| c.into_iter().collect::<String>()),
        l2 in prop::collection::vec(mrz_ish_char(), 44).prop_map(|c| c.into_iter().collect::<String>()),
    ) {
        let _ = parse_td3(&l1, &l2);
    }

    /// `parse_mrv_a` must never panic regardless of line length or content.
    #[test]
    fn parse_mrv_a_never_panics(l1 in near_length_string(44), l2 in near_length_string(44)) {
        let _ = parse_mrv_a(&l1, &l2);
    }

    /// `parse_mrv_b` must never panic regardless of line length or content.
    #[test]
    fn parse_mrv_b_never_panics(l1 in near_length_string(36), l2 in near_length_string(36)) {
        let _ = parse_mrv_b(&l1, &l2);
    }

    /// Exact-length MRV-A inputs over the full MRZ-ish charset.
    #[test]
    fn parse_mrv_a_never_panics_exact_length(
        l1 in prop::collection::vec(mrz_ish_char(), 44).prop_map(|c| c.into_iter().collect::<String>()),
        l2 in prop::collection::vec(mrz_ish_char(), 44).prop_map(|c| c.into_iter().collect::<String>()),
    ) {
        let _ = parse_mrv_a(&l1, &l2);
    }

    /// `format_td3`'s name field (`emit::clean_name_half` /
    /// `emit::truncate_name_components`, segment S3 — ICAO 9303 Part 3 §4.6
    /// punctuation handling) must never panic on arbitrary surname/given_names
    /// input and must always produce an exact-width line, however much
    /// punctuation, digits, or raw `any::<char>()` noise the input contains.
    /// Unlike `short_field_strategy()`/`name_strategy()` elsewhere in this
    /// suite (deliberately `[A-Z]`-only, for round-trip equality), this draws
    /// from the same noisy `mrz_ish_char` population `arbitrary_text()` uses.
    #[test]
    fn format_td3_name_field_never_panics_on_arbitrary_input(
        surname in prop::collection::vec(mrz_ish_char(), 0..80).prop_map(|c| c.into_iter().collect::<String>()),
        given_names in prop::collection::vec(mrz_ish_char(), 0..80).prop_map(|c| c.into_iter().collect::<String>()),
    ) {
        let fields = Td3Fields {
            surname,
            given_names,
            issuing_country: "UTO".to_string(),
            ..Td3Fields::default()
        };
        let mrz = format_td3(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        prop_assert_eq!(l1.chars().count(), 44);
        prop_assert_eq!(l2.chars().count(), 44);
    }

    /// The one rule ICAO 9303 Part 4 §4.2.3 makes normative about truncation
    /// (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:467`): whenever a name
    /// is truncated, the name field's last character must be alphabetic
    /// `A`-`Z`. `surname`/`given_names` are both drawn from a 20-40 char
    /// letters-only range, so their combined length (>= 20 + 2 + 20 = 42) is
    /// always past TD3's 39-char name field, guaranteeing truncation fires on
    /// every case `emit::truncate_name_components` handles.
    #[test]
    fn truncated_td3_name_field_always_ends_alphabetic(
        surname in "[A-Z]{20,40}",
        given_names in "[A-Z]{20,40}",
    ) {
        let fields = Td3Fields {
            surname,
            given_names,
            issuing_country: "UTO".to_string(),
            ..Td3Fields::default()
        };
        let mrz = format_td3(&fields);
        let (l1, _l2) = mrz.split_once('\n').unwrap();
        let last = l1.chars().next_back().unwrap();
        prop_assert!(last.is_ascii_alphabetic(), "name field ended in {last:?}, not A-Z: {l1:?}");
    }
}

/// A document number of arbitrary MRZ-charset length 1..=20 — the whole
/// space this crate's overflow encoding (ICAO 9303 Part 5/6 note j; extended
/// to TD3 by analogy, see `parser::read_overflow`'s doc comment) has to
/// handle: fits-as-printed (<=9), fits-via-overflow, and too-long-so-truncate.
fn any_length_doc_number() -> impl Strategy<Value = String> {
    "[A-Z0-9]{1,20}"
}

fn short_field_strategy() -> impl Strategy<Value = String> {
    "[A-Z]{1,10}"
}

fn yymmdd_strategy() -> impl Strategy<Value = String> {
    (0u32..100, 1u32..=12, 1u32..=28).prop_map(|(yy, mm, dd)| format!("{yy:02}{mm:02}{dd:02}"))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    /// Unlike the panic-only properties above, this one asserts correctness:
    /// whatever length the document number is, emit -> parse must validate,
    /// and when overflow encoding kicks in `full_document_number()` must
    /// recover the original input exactly. A "valid but wrong" reassembly
    /// would be a silent corruption bug, worse than a rejected parse.
    #[test]
    fn td3_overflow_emit_parse_always_valid(
        document_number in any_length_doc_number(),
        surname in short_field_strategy(),
        given_names in short_field_strategy(),
        date_of_birth in yymmdd_strategy(),
        date_of_expiry in yymmdd_strategy(),
    ) {
        let fields = Td3Fields {
            document_number: document_number.clone(),
            surname,
            given_names,
            date_of_birth,
            date_of_expiry,
            ..Td3Fields::default()
        };
        let mrz = format_td3(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        let d = parse_td3(l1, l2).unwrap();
        prop_assert!(d.valid(), "checks: {:?}", d.checks);
        if d.document_number_full.is_some() {
            prop_assert_eq!(d.full_document_number(), document_number.as_str());
        }
    }

    #[test]
    fn td2_overflow_emit_parse_always_valid(
        document_number in any_length_doc_number(),
        surname in short_field_strategy(),
        given_names in short_field_strategy(),
        date_of_birth in yymmdd_strategy(),
        date_of_expiry in yymmdd_strategy(),
    ) {
        let fields = Td2Fields {
            document_number: document_number.clone(),
            surname,
            given_names,
            date_of_birth,
            date_of_expiry,
            ..Td2Fields::default()
        };
        let mrz = format_td2(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        let d = parse_td2(l1, l2).unwrap();
        prop_assert!(d.valid(), "checks: {:?}", d.checks);
        if d.document_number_full.is_some() {
            prop_assert_eq!(d.full_document_number(), document_number.as_str());
        }
    }

    #[test]
    fn td1_overflow_emit_parse_always_valid(
        document_number in any_length_doc_number(),
        surname in short_field_strategy(),
        given_names in short_field_strategy(),
        date_of_birth in yymmdd_strategy(),
        date_of_expiry in yymmdd_strategy(),
    ) {
        let fields = Td1Fields {
            document_number: document_number.clone(),
            surname,
            given_names,
            date_of_birth,
            date_of_expiry,
            ..Td1Fields::default()
        };
        let mrz = format_td1(&fields);
        let mut lines = mrz.lines();
        let l1 = lines.next().unwrap();
        let l2 = lines.next().unwrap();
        let l3 = lines.next().unwrap();
        let d = parse_td1(l1, l2, l3).unwrap();
        prop_assert!(d.valid(), "checks: {:?}", d.checks);
        if d.document_number_full.is_some() {
            prop_assert_eq!(d.full_document_number(), document_number.as_str());
        }
    }
}

/// Any Unicode `char`, weighted so Table A code points and their lowercase
/// counterparts show up often enough to be worth generating alongside
/// genuinely arbitrary input.
fn translit_ish_char() -> impl Strategy<Value = char> {
    prop_oneof![
        4 => prop::char::range('A', 'Z'),
        2 => prop::char::range('a', 'z'),
        2 => prop::char::range('0', '9'),
        3 => (0usize..95).prop_map(|i| {
            // Sample a Table A code point via `transliterations`'s own inverse:
            // walk the known Latin-1/Latin-Extended-A ranges the table covers
            // and pick one that has entries. Falls back to a fixed one on any
            // miss so the closure is total.
            const CANDIDATES: &[char] = &[
                '\u{00C0}', '\u{00C4}', '\u{00C5}', '\u{00D1}', '\u{00D6}', '\u{00DC}',
                '\u{00DF}', '\u{0110}', '\u{0131}', '\u{0132}', '\u{013F}', '\u{1E9E}',
                '\u{017D}',
            ];
            CANDIDATES[i % CANDIDATES.len()]
        }),
        1 => any::<char>(),
    ]
}

fn translit_ish_string() -> impl Strategy<Value = String> {
    prop::collection::vec(translit_ish_char(), 0..40).prop_map(|chars| chars.into_iter().collect())
}

/// The 37-character ICAO MRZ alphabet, unweighted — unlike `mrz_ish_char`
/// this strategy never emits noise, so every generated field is exactly what
/// `substitution_candidates` is meant to operate on.
fn strict_mrz_char() -> impl Strategy<Value = char> {
    prop_oneof![
        prop::char::range('A', 'Z'),
        prop::char::range('0', '9'),
        Just('<'),
    ]
}

fn strict_mrz_string() -> impl Strategy<Value = String> {
    prop::collection::vec(strict_mrz_char(), 0..40).prop_map(|c| c.into_iter().collect())
}

proptest! {
    /// Every candidate `substitution_candidates` proposes is the same length
    /// as the input, pure ASCII, and drawn from the MRZ charset — the
    /// invariant `substituted`/`solve_substitution` in `crates/mrz/src/parser.rs`
    /// and `repair.rs` depend on to feed candidates straight back into the
    /// check-digit oracle without a charset check of their own.
    #[test]
    fn substitution_candidates_preserve_length_ascii_and_charset(field in strict_mrz_string()) {
        let candidates = substitution_candidates(&field);
        for c in &candidates {
            prop_assert_eq!(
                c.chars().count(),
                field.chars().count(),
                "length changed for {:?} -> {:?}",
                field,
                c
            );
            prop_assert!(c.is_ascii(), "{c:?} is not ascii");
            prop_assert!(
                c.chars().all(|ch| matches!(ch, 'A'..='Z' | '0'..='9' | '<')),
                "{c:?} escapes the MRZ charset"
            );
        }
    }

    /// `substitution_candidates` must never panic on arbitrary noisy input
    /// either — the same "never panics" contract the rest of this suite holds
    /// the parser to.
    #[test]
    fn substitution_candidates_never_panics_on_arbitrary_text(field in arbitrary_text()) {
        let _ = substitution_candidates(&field);
    }
}

proptest! {
    /// `transliterate` never panics on arbitrary input, for any style.
    #[test]
    fn transliterate_never_panics(
        s in translit_ish_string(),
        style_idx in 0u8..3,
    ) {
        let style = match style_idx {
            0 => TransliterationStyle::Expanded,
            1 => TransliterationStyle::Simple,
            _ => TransliterationStyle::XxSuffix,
        };
        let _ = transliterate(&s, style);
    }

    /// Every Table A input character's transliteration output consists only
    /// of `A`-`Z` — the property the emit path's uppercase-first design and
    /// `[A-Z0-9<]` MRZ alphabet both depend on.
    #[test]
    fn transliterate_table_a_output_is_upper_ascii(
        s in translit_ish_string(),
    ) {
        for c in s.chars() {
            for u in c.to_uppercase() {
                if !transliterations(u).is_empty() {
                    let out = transliterate(&u.to_string(), TransliterationStyle::Expanded);
                    prop_assert!(
                        out.chars().all(|o| o.is_ascii_uppercase()),
                        "transliterate({u:?}) = {out:?} contains non-[A-Z]"
                    );
                }
            }
        }
    }
}
