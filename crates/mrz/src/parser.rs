//! MRZ line parsers (TD1/TD2/TD3) and the free-text scanner [`find_and_parse`].
//!
//! Field offsets follow ICAO 9303 parts 4 (TD3), 5 (TD1) and 6 (TD2). Each
//! parser verifies every printed check digit; the scanner drives the OCR-repair
//! machinery in [`crate::checksum`] and accepts a candidate reading only when
//! its composite check digit proves the read.

use crate::checksum::{
    aggressive_defiller, char_value, defiller, digitize, fix_doc_code, fix_name_separator,
    is_mrz_charset, letterize, normalize_line, repair_positions, variants, verify,
};
use crate::dates::{date_completeness, expand_date_with_pivot};
use crate::{country_name, Checks, Format, MrzData, MrzError, ParseOptions};

/// A document number that overflowed its 9-character field.
struct Overflow {
    /// The nine principal characters (spec form) or the legacy crate's
    /// truncated eight (legacy form), plus the remainder read from the
    /// optional field. See [`legacy`](Overflow::legacy).
    full: String,
    /// Whether the remainder's own check digit validates the reassembly.
    check_ok: bool,
    /// `true` when `full` was reassembled using the pre-0.6 eight-character
    /// encoding this crate used to emit, rather than the Doc 9303 form.
    legacy: bool,
    /// Characters of the optional field consumed by the overflow encoding
    /// (remainder + its check digit + the terminating filler).
    consumed: usize,
}

/// Decode the ICAO 9303 long-document-number overflow encoding: TD1 note j
/// (`knowledge/docs9303/Doc_9303_Part5_Specs_for_TD1_MROTDs.md:368`) and its
/// §4.2.4 check-digit table (`:464-469`), TD2 note j
/// (`knowledge/docs9303/Doc_9303_Part6_Specs_for_TD2_MROTDs.md:340`).
///
/// **Part 4 defines no overflow encoding for TD3 at all** — nothing in the
/// corpus states a "more than 9 characters" rule for TD3; that language
/// appears only in Parts 5, 6 and 7 (Part 7 explicitly caps at "positions 1
/// to 9" with no spillover — MRV-A/MRV-B never call this function). Applying
/// the TD1/TD2 rule to TD3 below is a deliberate extension by analogy, made
/// because issuers do it in practice, not something Part 4 licenses. See
/// `knowledge/docs9303/CONFORMANCE_BASIS.md`'s "Part 5 §4.2.4 / Part 6 note
/// j" entry for the full corroboration.
///
/// When the number exceeds the 9-character field, all nine principal
/// characters are printed, the field's check-digit position is set to a
/// filler to signal a truncated number, and the *remainder* plus a check
/// digit computed over the whole number (nine principal characters plus
/// remainder) is written at the start of the optional/personal-number field,
/// terminated by a filler.
///
/// This crate emitted a non-conformant eight-character form before 0.6.0
/// (first 8 characters + filler, remainder starting at the 9th). Both forms
/// are attempted here so a zone written by an older version of this crate
/// still parses. **The two forms are distinguishable but not perfectly**:
/// because `<` has character value 0 yet still consumes a 7-3-1 weight slot,
/// the spec-form and legacy-form candidate inputs differ in both length and
/// weight alignment, so they generally produce different check digits — but
/// a legacy zone will spuriously satisfy the spec-form check roughly 1 time
/// in 10 (one digit out of ten possible check-digit values), yielding a full
/// number with one extra stray character. That is inherent to testing two
/// encodings against a single printed check digit; there is no heuristic
/// that defeats it, so this function does not attempt one.
///
/// Returns `None` when the zone does not carry an overflow signature at all
/// (check digit position isn't a filler, or the optional field has no
/// remainder to read), in which case the caller keeps the ordinary
/// fixed-width reading. MRV-A/MRV-B never use this — ICAO 9303 part 7 defines
/// no overflow encoding.
fn read_overflow(number_field: &str, check: char, optional: &str) -> Option<Overflow> {
    if check != '<' {
        return None;
    }
    if number_field.trim_end_matches('<').is_empty() {
        // A blank document-number field is a blank field, not an overflow.
        return None;
    }
    // The remainder runs to the terminating filler and ends with its check digit.
    let end = optional.find('<')?;
    if end < 2 {
        return None; // need at least one remainder character plus a check digit
    }
    let (remainder, check_digit) = optional[..end].split_at(end - 1);
    let check_digit = check_digit.chars().next()?;
    if !check_digit.is_ascii_digit() {
        return None;
    }
    let consumed = end + 1;

    // Spec form: all nine principal characters plus the remainder.
    let spec_full = format!("{number_field}{remainder}");
    if verify(&spec_full, check_digit) {
        return Some(Overflow {
            full: spec_full,
            check_ok: true,
            legacy: false,
            consumed,
        });
    }

    // Legacy form: this crate's pre-0.6 eight-character truncation + filler.
    if number_field.ends_with('<') {
        let head = number_field[0..8].trim_end_matches('<');
        let legacy_full = format!("{head}{remainder}");
        if verify(&legacy_full, check_digit) {
            return Some(Overflow {
                full: legacy_full,
                check_ok: true,
                legacy: true,
                consumed,
            });
        }
    }

    // Neither form verifies: surface the spec-form reassembly with a failed
    // check so the caller still sees the full number, not a truncated one.
    Some(Overflow {
        full: spec_full,
        check_ok: false,
        legacy: false,
        consumed,
    })
}

/// Trim an optional-data field to what remains after any overflow encoding.
fn optional_tail<'a>(optional: &'a str, overflow: &Option<Overflow>) -> &'a str {
    match overflow {
        Some(o) => optional[o.consumed..].trim_end_matches('<'),
        None => optional.trim_end_matches('<'),
    }
}

fn opt_string(s: &str) -> Option<String> {
    (!s.is_empty()).then(|| s.to_string())
}

/// Split a name field on the ICAO 9303 primary/secondary identifier
/// separator `<<`. Falls back to a single `<` when no `<<` survives OCR —
/// a real, measured failure mode distinct from [`fix_name_separator`]'s `<`
/// misread as `KK`: OCR sometimes drops one of the two filler characters
/// outright rather than misreading it, collapsing `SURNAME<<GIVEN` to
/// `SURNAME<GIVEN` with no `KK` anywhere for that repair to catch. Before
/// this fallback, that single dropped character sent `given_names` to `""`
/// unconditionally — first measured on MRV-A/MRV-B (`given_names` CER
/// 63.5%/76.7% at first measurement, 30 seeds, `--profile clean`, seed 42;
/// see `knowledge/ROADMAP.md`'s M6 section) but present on every format that
/// calls this function, since none of them check-digit-cover line 1's name
/// field to begin with. The fallback is unambiguous for this crate's own
/// generated corpus (`synthpass-gen`'s `SURNAMES`/`GIVEN_NAMES_M`/
/// `GIVEN_NAMES_F` are single tokens, never containing an internal `<`), but
/// is a best-effort guess against a real-world compound name (e.g. `DE<LA
/// <CRUZ<<MARIA` losing its `<<` down to `<` is indistinguishable from a
/// compound *surname*'s own internal `<`) — never claimed as proven, since
/// `FieldConfidence` never rates a name field above `MRZ_STRUCTURAL` (0.9)
/// regardless of which branch below produced it.
fn clean_name(field: &str) -> (String, String) {
    let trimmed = field.trim_end_matches('<');
    let (surname, given) = match trimmed.split_once("<<") {
        Some((s, g)) => (s, g),
        None => match trimmed.split_once('<') {
            Some((s, g)) => (s, g),
            None => (trimmed, ""),
        },
    };
    (
        surname.replace('<', " ").trim().to_string(),
        given.replace('<', " ").trim().to_string(),
    )
}

fn clean_sex(c: char) -> String {
    match c {
        'M' => "M".into(),
        'F' => "F".into(),
        _ => "X".into(),
    }
}

fn ensure_charset(line: &str) -> Result<(), MrzError> {
    for c in line.chars() {
        char_value(c)?;
    }
    Ok(())
}

/// Parse a TD3 (passport) MRZ: two lines of exactly 44 characters
/// (ICAO 9303 part 4 §4.2.2). Long-document-number overflow is decoded here
/// too, though Part 4 defines no such rule for TD3 — it is applied by analogy
/// to part 5 note j / part 6 note j, because issuers do it in practice.
pub fn parse_td3(line1: &str, line2: &str) -> Result<MrzData, MrzError> {
    parse_td3_with(line1, line2, &ParseOptions::default())
}

/// [`parse_td3`] with an explicit [`ParseOptions`].
pub fn parse_td3_with(line1: &str, line2: &str, opts: &ParseOptions) -> Result<MrzData, MrzError> {
    for line in [line1, line2] {
        if line.len() != 44 {
            return Err(MrzError::BadLength {
                expected: 44,
                got: line.len(),
            });
        }
        ensure_charset(line)?;
    }
    if !line1.starts_with('P') {
        return Err(MrzError::BadDocumentCode(line1[0..2].to_string()));
    }

    let (surname, given_names) = clean_name(&line1[5..44]);

    let document_number = line2[0..9].trim_end_matches('<').to_string();
    let personal_raw = &line2[28..42];
    let overflow = read_overflow(&line2[0..9], line2.as_bytes()[9] as char, personal_raw);
    let personal = optional_tail(personal_raw, &overflow);

    let checks = Checks {
        document_number: match &overflow {
            Some(o) => o.check_ok,
            None => verify(&line2[0..9], line2.as_bytes()[9] as char),
        },
        date_of_birth: verify(&line2[13..19], line2.as_bytes()[19] as char),
        date_of_expiry: verify(&line2[21..27], line2.as_bytes()[27] as char),
        personal_number: verify(personal_raw, line2.as_bytes()[42] as char),
        // Composite: doc number + check, DOB + check, expiry + check +
        // personal number + check (line 2 positions 1-10, 14-20, 22-43).
        composite: verify(
            &format!("{}{}{}", &line2[0..10], &line2[13..20], &line2[21..43]),
            line2.as_bytes()[43] as char,
        ),
    };

    let document_number_legacy_encoding = overflow.as_ref().is_some_and(|o| o.legacy);
    Ok(MrzData {
        format: Format::Td3,
        document_type: line1[0..2].trim_end_matches('<').to_string(),
        issuing_country: line1[2..5].trim_end_matches('<').to_string(),
        document_number,
        document_number_full: overflow.map(|o| o.full),
        document_number_legacy_encoding,
        surname,
        given_names,
        nationality: line2[10..13].trim_end_matches('<').to_string(),
        date_of_birth: expand_date_with_pivot(&line2[13..19], true, opts.pivot_yy),
        date_of_birth_completeness: date_completeness(&line2[13..19]),
        sex: clean_sex(line2.as_bytes()[20] as char),
        date_of_expiry: expand_date_with_pivot(&line2[21..27], false, opts.pivot_yy),
        personal_number: opt_string(personal),
        mrz_lines: format!("{line1}\n{line2}"),
        checks,
    })
}

/// Parse a TD2 MRZ: two lines of exactly 36 characters (ICAO 9303 part 6).
/// Covers identity-card document codes (`I`/`A`/`C`); MRV-B visas share the
/// geometry but lack a composite check digit and are not handled here.
///
/// ```
/// let doc = mrz::parse_td2(
///     "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
///     "D231458907UTO7408122F1204159<<<<<<<6",
/// ).unwrap();
/// assert_eq!(doc.surname, "ERIKSSON");
/// assert!(doc.valid());
/// ```
pub fn parse_td2(line1: &str, line2: &str) -> Result<MrzData, MrzError> {
    parse_td2_with(line1, line2, &ParseOptions::default())
}

/// [`parse_td2`] with an explicit [`ParseOptions`].
pub fn parse_td2_with(line1: &str, line2: &str, opts: &ParseOptions) -> Result<MrzData, MrzError> {
    for line in [line1, line2] {
        if line.len() != 36 {
            return Err(MrzError::BadLength {
                expected: 36,
                got: line.len(),
            });
        }
        ensure_charset(line)?;
    }
    let code = line1[0..2].trim_end_matches('<');
    if !matches!(code.as_bytes().first(), Some(b'I' | b'A' | b'C')) {
        return Err(MrzError::BadDocumentCode(code.to_string()));
    }

    let (surname, given_names) = clean_name(&line1[5..36]);

    let document_number = line2[0..9].trim_end_matches('<').to_string();
    let optional_raw = &line2[28..35];
    let overflow = read_overflow(&line2[0..9], line2.as_bytes()[9] as char, optional_raw);
    let optional = optional_tail(optional_raw, &overflow);

    let checks = Checks {
        document_number: match &overflow {
            Some(o) => o.check_ok,
            None => verify(&line2[0..9], line2.as_bytes()[9] as char),
        },
        date_of_birth: verify(&line2[13..19], line2.as_bytes()[19] as char),
        date_of_expiry: verify(&line2[21..27], line2.as_bytes()[27] as char),
        personal_number: true, // TD2 has no separate personal-number check digit
        // Composite: line 2 positions 1-10, 14-20, 22-35 (doc number + check,
        // DOB + check, expiry + check + optional data).
        composite: verify(
            &format!("{}{}{}", &line2[0..10], &line2[13..20], &line2[21..35]),
            line2.as_bytes()[35] as char,
        ),
    };

    let document_number_legacy_encoding = overflow.as_ref().is_some_and(|o| o.legacy);
    Ok(MrzData {
        format: Format::Td2,
        document_type: code.to_string(),
        issuing_country: line1[2..5].trim_end_matches('<').to_string(),
        document_number,
        document_number_full: overflow.map(|o| o.full),
        document_number_legacy_encoding,
        surname,
        given_names,
        nationality: line2[10..13].trim_end_matches('<').to_string(),
        date_of_birth: expand_date_with_pivot(&line2[13..19], true, opts.pivot_yy),
        date_of_birth_completeness: date_completeness(&line2[13..19]),
        sex: clean_sex(line2.as_bytes()[20] as char),
        date_of_expiry: expand_date_with_pivot(&line2[21..27], false, opts.pivot_yy),
        personal_number: opt_string(optional),
        mrz_lines: format!("{line1}\n{line2}"),
        checks,
    })
}

/// Parse a TD1 (ID card) MRZ: three lines of exactly 30 characters
/// (ICAO 9303 part 5).
///
/// ```
/// let doc = mrz::parse_td1(
///     "I<UTOD231458907<<<<<<<<<<<<<<<",
///     "7408122F1204159UTO<<<<<<<<<<<6",
///     "ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
/// ).unwrap();
/// assert_eq!(doc.surname, "ERIKSSON");
/// assert!(doc.valid());
/// ```
pub fn parse_td1(line1: &str, line2: &str, line3: &str) -> Result<MrzData, MrzError> {
    parse_td1_with(line1, line2, line3, &ParseOptions::default())
}

/// [`parse_td1`] with an explicit [`ParseOptions`].
pub fn parse_td1_with(
    line1: &str,
    line2: &str,
    line3: &str,
    opts: &ParseOptions,
) -> Result<MrzData, MrzError> {
    for line in [line1, line2, line3] {
        if line.len() != 30 {
            return Err(MrzError::BadLength {
                expected: 30,
                got: line.len(),
            });
        }
        ensure_charset(line)?;
    }
    let code = line1[0..2].trim_end_matches('<');
    if !matches!(code.as_bytes().first(), Some(b'I' | b'A' | b'C')) {
        return Err(MrzError::BadDocumentCode(code.to_string()));
    }

    let (surname, given_names) = clean_name(line3);

    let optional1_raw = &line1[15..30];
    let overflow = read_overflow(&line1[5..14], line1.as_bytes()[14] as char, optional1_raw);
    let optional1 = optional_tail(optional1_raw, &overflow);
    let optional2 = line2[18..29].trim_end_matches('<');
    let personal = [optional1, optional2]
        .iter()
        .filter(|s| !s.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");

    let checks = Checks {
        document_number: match &overflow {
            Some(o) => o.check_ok,
            None => verify(&line1[5..14], line1.as_bytes()[14] as char),
        },
        date_of_birth: verify(&line2[0..6], line2.as_bytes()[6] as char),
        date_of_expiry: verify(&line2[8..14], line2.as_bytes()[14] as char),
        personal_number: true, // TD1 has no personal-number check digit
        // Composite: line1 positions 6-30, line2 positions 1-7, 9-15, 19-29.
        composite: verify(
            &format!(
                "{}{}{}{}",
                &line1[5..30],
                &line2[0..7],
                &line2[8..15],
                &line2[18..29]
            ),
            line2.as_bytes()[29] as char,
        ),
    };

    let document_number_legacy_encoding = overflow.as_ref().is_some_and(|o| o.legacy);
    Ok(MrzData {
        format: Format::Td1,
        document_type: code.to_string(),
        issuing_country: line1[2..5].trim_end_matches('<').to_string(),
        document_number: line1[5..14].trim_end_matches('<').to_string(),
        document_number_full: overflow.map(|o| o.full),
        document_number_legacy_encoding,
        surname,
        given_names,
        nationality: line2[15..18].trim_end_matches('<').to_string(),
        date_of_birth: expand_date_with_pivot(&line2[0..6], true, opts.pivot_yy),
        date_of_birth_completeness: date_completeness(&line2[0..6]),
        sex: clean_sex(line2.as_bytes()[7] as char),
        date_of_expiry: expand_date_with_pivot(&line2[8..14], false, opts.pivot_yy),
        personal_number: opt_string(&personal),
        mrz_lines: format!("{line1}\n{line2}\n{line3}"),
        checks,
    })
}

/// Parse an MRV-A machine readable visa: two lines of exactly 44 characters
/// (ICAO 9303 part 7). Geometry mirrors TD3 through the expiry check digit,
/// but there is no personal-number field and no composite check digit.
///
/// ```
/// let doc = mrz::parse_mrv_a(
///     "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
///     "L898902C<3UTO6908061F9406236ZE184226B<<<<<<<",
/// ).unwrap();
/// assert_eq!(doc.surname, "ERIKSSON");
/// assert!(doc.valid());
/// ```
pub fn parse_mrv_a(line1: &str, line2: &str) -> Result<MrzData, MrzError> {
    parse_mrv_a_with(line1, line2, &ParseOptions::default())
}

/// [`parse_mrv_a`] with an explicit [`ParseOptions`].
pub fn parse_mrv_a_with(
    line1: &str,
    line2: &str,
    opts: &ParseOptions,
) -> Result<MrzData, MrzError> {
    for line in [line1, line2] {
        if line.len() != 44 {
            return Err(MrzError::BadLength {
                expected: 44,
                got: line.len(),
            });
        }
        ensure_charset(line)?;
    }
    if !line1.starts_with('V') {
        return Err(MrzError::BadDocumentCode(line1[0..2].to_string()));
    }

    let (surname, given_names) = clean_name(&line1[5..44]);

    let document_number = line2[0..9].trim_end_matches('<').to_string();
    let optional = line2[28..44].trim_end_matches('<');

    let checks = Checks {
        document_number: verify(&line2[0..9], line2.as_bytes()[9] as char),
        date_of_birth: verify(&line2[13..19], line2.as_bytes()[19] as char),
        date_of_expiry: verify(&line2[21..27], line2.as_bytes()[27] as char),
        // MRV-A has no personal-number or composite check digit at all;
        // vacuously true, same convention TD1/TD2 use for personal_number.
        personal_number: true,
        composite: true,
    };

    Ok(MrzData {
        format: Format::MrvA,
        document_type: line1[0..2].trim_end_matches('<').to_string(),
        issuing_country: line1[2..5].trim_end_matches('<').to_string(),
        document_number,
        // ICAO 9303 part 7 defines no overflow encoding for visas.
        document_number_full: None,
        document_number_legacy_encoding: false,
        surname,
        given_names,
        nationality: line2[10..13].trim_end_matches('<').to_string(),
        date_of_birth: expand_date_with_pivot(&line2[13..19], true, opts.pivot_yy),
        date_of_birth_completeness: date_completeness(&line2[13..19]),
        sex: clean_sex(line2.as_bytes()[20] as char),
        date_of_expiry: expand_date_with_pivot(&line2[21..27], false, opts.pivot_yy),
        personal_number: opt_string(optional),
        mrz_lines: format!("{line1}\n{line2}"),
        checks,
    })
}

/// Parse an MRV-B machine readable visa: two lines of exactly 36 characters
/// (ICAO 9303 part 7). Geometry mirrors TD2 through the expiry check digit,
/// but there is no personal-number field and no composite check digit.
///
/// ```
/// let doc = mrz::parse_mrv_b(
///     "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
///     "L898902C<3UTO6908061F9406236ZE184226",
/// ).unwrap();
/// assert_eq!(doc.surname, "ERIKSSON");
/// assert!(doc.valid());
/// ```
pub fn parse_mrv_b(line1: &str, line2: &str) -> Result<MrzData, MrzError> {
    parse_mrv_b_with(line1, line2, &ParseOptions::default())
}

/// [`parse_mrv_b`] with an explicit [`ParseOptions`].
pub fn parse_mrv_b_with(
    line1: &str,
    line2: &str,
    opts: &ParseOptions,
) -> Result<MrzData, MrzError> {
    for line in [line1, line2] {
        if line.len() != 36 {
            return Err(MrzError::BadLength {
                expected: 36,
                got: line.len(),
            });
        }
        ensure_charset(line)?;
    }
    if !line1.starts_with('V') {
        return Err(MrzError::BadDocumentCode(line1[0..2].to_string()));
    }

    let (surname, given_names) = clean_name(&line1[5..36]);

    let document_number = line2[0..9].trim_end_matches('<').to_string();
    let optional = line2[28..36].trim_end_matches('<');

    let checks = Checks {
        document_number: verify(&line2[0..9], line2.as_bytes()[9] as char),
        date_of_birth: verify(&line2[13..19], line2.as_bytes()[19] as char),
        date_of_expiry: verify(&line2[21..27], line2.as_bytes()[27] as char),
        // MRV-B has no personal-number or composite check digit at all;
        // vacuously true, same convention TD1/TD2 use for personal_number.
        personal_number: true,
        composite: true,
    };

    Ok(MrzData {
        format: Format::MrvB,
        document_type: line1[0..2].trim_end_matches('<').to_string(),
        issuing_country: line1[2..5].trim_end_matches('<').to_string(),
        document_number,
        // ICAO 9303 part 7 defines no overflow encoding for visas.
        document_number_full: None,
        document_number_legacy_encoding: false,
        surname,
        given_names,
        nationality: line2[10..13].trim_end_matches('<').to_string(),
        date_of_birth: expand_date_with_pivot(&line2[13..19], true, opts.pivot_yy),
        date_of_birth_completeness: date_completeness(&line2[13..19]),
        sex: clean_sex(line2.as_bytes()[20] as char),
        date_of_expiry: expand_date_with_pivot(&line2[21..27], false, opts.pivot_yy),
        personal_number: opt_string(optional),
        mrz_lines: format!("{line1}\n{line2}"),
        checks,
    })
}

/// Undo a **dropped** (not merely misread) line-1 position-1 filler —
/// generalizes [`repair_td1_line1_unshifted`]'s transformation to formats
/// whose document code can *only* ever be a single letter plus filler, never
/// a genuine second letter.
///
/// **Ordering is load-bearing here, unlike TD1.** TD1 tries its unshifted
/// reading as a second candidate in either order, because its own line-1
/// check digit (the document number's) arbitrates which candidate — if
/// either — is real. TD3/MRV-A/MRV-B have no check digit on line 1 at all,
/// so `consider()` in `find_and_parse_with` accepts the *first*
/// checksum-passing line-1/line-2 combination it tries, regardless of what
/// line 1 says — meaning an unshifted candidate offered only *after* the
/// unrepaired one is dead code in practice: the unrepaired reading already
/// satisfies line 2's checksums just as well (line 1 never gates them), so
/// it always wins first and the unshifted candidate never gets a chance to
/// matter. The call sites (TD3/MRV-A/MRV-B in `find_and_parse_with`)
/// therefore try `repair_*_line1_unshifted` *before* `repair_*_line1`.
///
/// This is also why the fix is a **second candidate at all**, rather than
/// applied unconditionally inside `repair_td3_line1`/`repair_mrv_line1`
/// (the first version of this fix, before either of the two points above
/// were understood). That version — despite "position 1 isn't `<`" being
/// unambiguous evidence of the drop, not a guess, for these formats (see
/// `Td3Fields`'/`MrvAFields`'/`MrvBFields`' doc comments: no diplomatic/
/// subtype variant with a genuine second letter is modeled anywhere in this
/// crate) — measured a real, reproducible **Tier-1 hit-rate regression** on
/// a 30-seed TD3 corpus (`--profile clean --seed 42`): 76.7% → 63.3%,
/// confirmed via a same-binary A/B env-var toggle to rule out the run-to-run
/// OCR-timing variance this repo's contention note already documents. The
/// mechanism: `variants()` builds `[repaired, fitted, last_resort]` per
/// candidate and deduplicates by exact string equality; mutating what
/// `repaired` is (rather than adding a second, independent `variants()` call
/// for it) also changes what `last_resort = aggressive_defiller(&repaired)`
/// evaluates to, silently removing a previously-tried, load-bearing
/// candidate string in some cases rather than only adding a new one. Kept as
/// two separate `variants()` calls — one per repair function — specifically
/// avoids that collapse: each keeps its own independent `[repaired, fitted,
/// last_resort]` triplet, so the search space is provably a strict superset
/// of the unmodified pipeline's.
///
/// Net effect, measured 30 seeds, `--profile clean`, seed 42, same-day: hit
/// rate unchanged for all three formats (TD3 76.7%, MRV-A 86.7%, MRV-B
/// 93.3% — identical to the pre-fix baseline), `document_type`/
/// `issuing_country` CER: TD3 76.67%/52.22% → 23.33%/16.67%, MRV-A
/// 6.67%/4.44% → 0%/0%, MRV-B 40.00%/26.67% → 3.33%/2.22%. Not a full fix
/// for TD3/MRV-B (some corrupted readings still win over the unshifted
/// candidate when both parse structurally, since nothing on line 1
/// discriminates between them) — a partial, hit-rate-safe improvement, not
/// a claim of completeness.
///
/// First measured via `synthpass-bench --dump-ocr` against TD3 and MRV-B
/// (`knowledge/ROADMAP.md`'s M6 note): the OCR retry loop's internal passes
/// sometimes drop this filler exactly the way TD1's did, e.g.
/// `V<BRAESKANDARI...` read as `VBRAESKANDARI...` — shifting
/// `issuing_country` (and everything after it) one position left. Silent on
/// these formats specifically because nothing check-digit-covers line 1
/// here, so it never fails a Tier-1 hit on its own, only corrupts
/// `document_type`/`issuing_country`/the name behind a passing read.
///
/// `target_width` is `l.len()` at the call site (44 for TD3/MRV-A, 36 for
/// MRV-B) — `variants()` always calls its `repair` function on an
/// already-`fit_length`-padded string (`checksum.rs`'s `variants`), so the
/// input is guaranteed to already be at the format's fixed width; the
/// length check below is a defensive no-op guard, not a real branch, for
/// the same reason [`repair_td1_line1_unshifted`] carries one.
fn unshift_line1_prefix(repaired: String, target_width: usize) -> String {
    if repaired.len() != target_width || repaired.as_bytes().get(1) == Some(&b'<') {
        return repaired;
    }
    format!("{}<{}", &repaired[0..1], &repaired[1..target_width - 1])
}

/// Undo a spurious extra character **inserted** at line-1 position 2, right
/// after `document_type`'s filler and before `issuing_country` — the mirror
/// image of [`unshift_line1_prefix`]'s dropped-filler case, and the missing
/// half [`unshift_line1_prefix`]'s own doc comment already flags ("not a
/// full fix... a partial, hit-rate-safe improvement, not a claim of
/// completeness").
///
/// Measured on real specimens (`crates/synthpass-ocr/examples/
/// integrity_survey.rs --dir passports --mrz-only`, 2026-08-17): 34 of 57
/// `UnrecognizedIssuingCountry` findings across 24 distinct countries shared
/// this exact shape — one garbage character prepended, the real third
/// letter lost, e.g. Czechia (`CZE`) read as `SCZ`, Iceland (`ISL`) as
/// `AIS`, Slovakia (`SVK`) as `SSV`, the UK (`GBR`) as `SGB`, the USA
/// (`USA`) as `SUS`.
///
/// **Why this needs a content-based guard, unlike the drop case.**
/// `unshift_line1_prefix` self-limits purely from shape: position 1 not
/// being `<` is unambiguous evidence of the drop, since a correctly-formed
/// line always has `<` there. This corruption leaves position 1 alone — the
/// filler survives — so shape alone can't distinguish a genuine insertion
/// from a correctly-formed line (both have `<` at position 1). Worse, some
/// real ICAO document codes genuinely have a second real letter there
/// (`PS` = service passport, `PP` = Canada/Cyprus's ordinary-passport code
/// since 2025-12-15 — see `synthpass_core::fusion::document_types_agree`) —
/// a `fix_doc_code`-style "no real code has X there" blocklist would be
/// simply wrong for this position. [`crate::country_name`] is the
/// discriminator instead: only apply the shift when the as-read
/// `issuing_country` fails to resolve to a real code but the
/// one-position-right-shifted read does. A line with a genuine two-letter
/// document code and a genuinely correct `issuing_country` already resolves
/// on the first check and is returned unchanged.
fn shift_line1_right_at_country(repaired: String, target_width: usize) -> String {
    if repaired.len() != target_width
        || target_width < 6
        || repaired.as_bytes().get(1) != Some(&b'<')
    {
        return repaired;
    }
    if country_name(&repaired[2..5]).is_some() {
        return repaired;
    }
    if country_name(&repaired[3..6]).is_none() {
        return repaired;
    }
    format!("{}{}<", &repaired[0..2], &repaired[3..target_width])
}

/// Tries [`shift_line1_right_at_country`] first, falling back to
/// [`unshift_line1_prefix`] when it doesn't apply.
///
/// The two can't be registered as independent `variants()` candidates in a
/// fixed order the way `repair_td3_line1`/`repair_td3_line1_unshifted` are:
/// each is a **no-op passthrough of the other's trigger case**. A dropped
/// filler (position 1 not `<`) is exactly `shift_line1_right_at_country`'s
/// guard condition for skipping — it returns the corrupted string
/// unchanged. An inserted character (position 1 still `<`) is exactly
/// `unshift_line1_prefix`'s guard condition for skipping — same thing.
/// TD3/MRV-A/MRV-B carry no check digit on line 1, so `consider()` accepts
/// whichever checksum-passing candidate it's offered first regardless of
/// what line 1 says — an unhelpful no-op tried first would silently outrun
/// the real fix for the *other* corruption, exactly the failure mode this
/// composition avoids by trying both, in order, inside one candidate.
///
/// The unshift fallback is additionally gated on [`country_name`] —
/// shared across all three callers, though only TD3 actually needs it:
/// TD3's document code genuinely has real two-letter forms (`PS` = service
/// passport, `PP` = Canada's and — effective 15 December 2025 — Cyprus's
/// ordinary-passport code; see `synthpass_core::fusion::document_types_agree`'s
/// doc comment), which [`unshift_line1_prefix`]'s shape-only guard cannot
/// tell apart from a genuine drop (both leave position 1 as a real letter,
/// not `<`). Applying the transform unconditionally would silently corrupt
/// those — pinned by `crates/mrz/tests/line1_right_shift.rs`'s
/// `a_genuine_two_letter_document_code_is_unaffected`, which caught this
/// pre-existing gap in `unshift_line1_prefix` itself (present before this
/// fix, on the raw candidate — this composition is the first place it's
/// actually guarded). MRV-A/MRV-B's document code is unconditionally `V`
/// plus filler (see `MrvAFields`'/`MrvBFields`' doc comments), so the gate
/// is a no-op there — a genuine drop always resolves to a real country once
/// unshifted, same as before. Only accept the unshifted reading when it
/// actually resolves to a real country; otherwise there's no positive
/// evidence it helped, so leave the line as
/// `repair_td3_line1`/`repair_mrv_*_line1` already produced it.
fn shift_or_unshift_line1(repaired: String, target_width: usize) -> String {
    let shifted = shift_line1_right_at_country(repaired.clone(), target_width);
    if shifted != repaired {
        return shifted;
    }
    unshift_if_country_resolves(repaired, target_width)
}

/// [`unshift_line1_prefix`], accepted only when the *unshifted* reading's
/// issuing-country slot (positions 2..5) resolves to a real [`country_name`].
/// Factored out of [`shift_or_unshift_line1`] so [`repair_td2_line1_shifted`]
/// can reuse the exact same gate without also pulling in
/// [`shift_line1_right_at_country`]'s independent insertion-repair, which is
/// out of scope for TD2 (see that function's call site).
fn unshift_if_country_resolves(repaired: String, target_width: usize) -> String {
    let unshifted = unshift_line1_prefix(repaired.clone(), target_width);
    if unshifted != repaired && target_width >= 5 && country_name(&unshifted[2..5]).is_some() {
        unshifted
    } else {
        repaired
    }
}

fn repair_td3_line1(l: &str) -> String {
    // Issuing state must be letters; the name field suffers filler misreads.
    let l = fix_doc_code(&repair_positions(l, &[(2..5, letterize)]));
    format!("{}{}", &l[0..5], fix_name_separator(&defiller(&l[5..])))
}

/// Second candidate alongside [`repair_td3_line1`] — composes both
/// [`shift_line1_right_at_country`] and [`unshift_line1_prefix`], see
/// [`shift_or_unshift_line1`]'s doc comment for why they can't be two
/// independently-ordered candidates the way TD1's are, and why the unshift
/// half is gated on [`country_name`] here specifically (TD3's document code
/// has genuine two-letter forms `unshift_line1_prefix`'s shape-only guard
/// can't tell apart from a real drop — the whole reason for the gate).
fn repair_td3_line1_shifted(l: &str) -> String {
    let target_width = l.len();
    shift_or_unshift_line1(repair_td3_line1(l), target_width)
}

fn repair_td3_line2(l: &str) -> String {
    let l = repair_positions(
        l,
        &[
            (9..10, digitize),   // doc number check digit
            (10..13, letterize), // nationality
            (13..20, digitize),  // DOB + check digit
            (21..28, digitize),  // expiry + check digit
            (42..44, digitize),  // personal-number check + composite
        ],
    );
    // The personal-number field is filler-dominated on most passports.
    // Note: `K` ≡ `<` (both value 20 ≡ 0 mod 10) under every 7-3-1 weight, so
    // check digits are provably blind to this misread — heuristics must do it.
    format!(
        "{}{}{}",
        &l[0..28],
        aggressive_defiller(&defiller(&l[28..42])),
        &l[42..44]
    )
}

/// Deliberately does **not** call [`unshift_line1_prefix`], unlike
/// [`repair_td3_line1`]/[`repair_mrv_line1`]. TD2 shares TD1's ICAO
/// ID-document-family code space (`"I"`, `"A"`, `"C"`, and genuine
/// two-real-letter subtypes — `crates/mrz/tests/td1_line_gap.rs`'s own
/// fixture uses `document_code: "ID"` deliberately, for the sibling
/// format), so "position 1 isn't `<`" is not unambiguous evidence of a
/// dropped filler here the way it is for TD3/MRV-A/MRV-B (whose document
/// code is always a single letter plus filler, no exceptions modeled
/// anywhere in this crate). TD1 resolves that ambiguity by trying the
/// unshifted reading as a *second* candidate and letting its own line-1
/// check digit (the document number's) decide — but TD2 has no check
/// digit on line 1 either, so a checksum can't arbitrate an unshifted TD2
/// candidate against the unrepaired one the way TD1's can. See
/// [`repair_td2_line1_shifted`] for the fix: an issuing-country-based gate,
/// not a checksum, arbitrates instead.
fn repair_td2_line1(l: &str) -> String {
    let l = fix_doc_code(&repair_positions(l, &[(2..5, letterize)]));
    format!("{}{}", &l[0..5], fix_name_separator(&defiller(&l[5..])))
}

/// Second candidate alongside [`repair_td2_line1`]: undoes a dropped
/// position-1 filler, TD2's version of the drop
/// [`repair_td3_line1_shifted`]/[`repair_mrv_a_line1_shifted`] already fix
/// for their formats — see [`repair_td2_line1`]'s doc comment for why TD2
/// couldn't get this for free the way those formats did, and why this one
/// works anyway.
///
/// **The document-code dictionary this crate's own MRZ_SEQUENCE_COMPLETENESS
/// plan proposed for this doesn't exist.** ICAO 9303 Part 5 §Note k / Part 6
/// §Note k are explicit: the document code's second character is "at the
/// discretion of the issuing State or organization" — only `V`, and `C`
/// immediately after `A`, are excluded. That's not a closed enumerable set;
/// building a "known-legitimate TD2 code" table would mean guessing at
/// real-world issuer choices with no normative ground truth, exactly the
/// kind of unproven heuristic `crate::repair::solve_substitution`'s "prove
/// it or don't touch it" discipline (and `synthpass_core::fusion.rs`'s
/// reverted 69.5%→61.5% heuristic) argue against.
///
/// The fix is [`unshift_if_country_resolves`] instead — the exact
/// country-name gate [`shift_or_unshift_line1`] already ships and has
/// measured for TD3/MRV-A/MRV-B (`crates/mrz/tests/line1_prefix_shift.rs`).
/// TD2's line-1 layout puts `issuing_country` at the same positions 2..5 as
/// TD3's, and [`crate::country_name`] — unlike a document-code table — *is*
/// a genuine closed ICAO/ISO registry already in production use. Only the
/// unshift half is reused (not [`shift_line1_right_at_country`]'s insertion
/// repair, the mirror-image corruption): this chunk's scope is the dropped
/// filler only, per `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`'s Chunk 6.
///
/// Correctly leaves a genuine two-letter document code alone: unshifting
/// `"IPBRA..."` (document code `IP`, country `BRA`) produces
/// `"I<PBR..."`, whose positions 2..5 read `PBR` — not a real country — so
/// the gate fails and the original is returned untouched (pinned by
/// `crates/mrz/tests/line1_prefix_shift.rs`'s
/// `td2_with_a_genuine_two_letter_document_code_is_unaffected`). A genuinely
/// dropped filler (`"IBRA..."` from `"I<BRA..."`) unshifts to `"I<BRA..."`,
/// whose positions 2..5 read `BRA` — a real country — so the gate passes.
fn repair_td2_line1_shifted(l: &str) -> String {
    let target_width = l.len();
    unshift_if_country_resolves(repair_td2_line1(l), target_width)
}

fn repair_td2_line2(l: &str) -> String {
    let l = repair_positions(
        l,
        &[
            (9..10, digitize),   // doc number check digit
            (10..13, letterize), // nationality
            (13..20, digitize),  // DOB + check digit
            (21..28, digitize),  // expiry + check digit
            (35..36, digitize),  // composite check digit
        ],
    );
    format!(
        "{}{}{}",
        &l[0..28],
        aggressive_defiller(&defiller(&l[28..35])),
        &l[35..36]
    )
}

fn repair_td1_line1(l: &str) -> String {
    let l = fix_doc_code(&repair_positions(
        l,
        &[(2..5, letterize), (14..15, digitize)],
    ));
    format!("{}{}", &l[0..15], aggressive_defiller(&defiller(&l[15..])))
}

/// A second candidate for TD1 line 1, alongside [`repair_td1_line1`]: undoes
/// a **dropped** (not merely misread) filler at position 1.
///
/// Measured via `synthpass-bench --dump-ocr` against the M6 TD1 corpus
/// (`knowledge/ROADMAP.md`'s M6 execution note): OCR does not misread the
/// position-1 filler as some lookalike character (that case is
/// [`fix_doc_code`]'s `K`↔`<` blind spot) — it drops the glyph outright,
/// which shifts every field from `issuing_country` onward one position left
/// and reads as e.g. `IBRAFLLF2W1316...` where the truth is `I<BRAFLLF2W1316...`.
/// `variants`'s own length-fitting (`fit_length`) cannot undo this: a line
/// that arrives one character short gets padded by *extending its longest
/// filler run*, which is the trailing run, not by inserting a filler back
/// at position 1 — so the shift survives unrepaired through the ordinary
/// pipeline, and TD1 is the one format where that matters: unlike TD3 (whose
/// only check digits live on line 2), TD1's document-number check digit is
/// on line 1 itself, so this single dropped character was breaking the
/// checksum outright, not just cosmetically corrupting `issuing_country`.
///
/// Reinserting `<` at position 1 and dropping the now-redundant final
/// character (the one `fit_length` added to compensate the length deficit)
/// undoes both steps in one move. Applied *after* [`repair_td1_line1`]'s
/// ordinary pipeline, so `fix_doc_code`/letterize/digitize/defiller already
/// ran; a no-op when position 1 already reads `<` (nothing to unshift, and
/// unshifting an already-correct line would wrongly discard a real
/// character). Like every repair candidate in this module, this is not an
/// assumption — `variants` tries it *alongside* the unshifted reading, and
/// the printed check digits are what decide which one, if either, is real.
fn repair_td1_line1_unshifted(l: &str) -> String {
    let repaired = repair_td1_line1(l);
    if repaired.len() != 30 || repaired.as_bytes().get(1) == Some(&b'<') {
        return repaired;
    }
    format!("{}<{}", &repaired[0..1], &repaired[1..29])
}

fn repair_td1_line2(l: &str) -> String {
    let l = repair_positions(
        l,
        &[
            (0..7, digitize),    // DOB + check digit
            (8..15, digitize),   // expiry + check digit
            (15..18, letterize), // nationality
            (29..30, digitize),  // composite
        ],
    );
    format!(
        "{}{}{}",
        &l[0..18],
        aggressive_defiller(&defiller(&l[18..29])),
        &l[29..30]
    )
}

fn repair_td1_line3(l: &str) -> String {
    fix_name_separator(&defiller(l))
}

/// MRV name/document-code line repair, shared shape by MRV-A (44 chars) and
/// MRV-B (36 chars). Modeled on `repair_td3_line1`/`repair_td2_line1`: letterize
/// the issuing-state field, fix the doc-code filler, defiller + fix the name
/// separator. Neither `repair_positions` (range starts at index 2) nor
/// `fix_doc_code` (only ever rewrites index 1) can touch index 0, so the
/// leading `V` document code always survives repair unchanged — the caller's
/// `l1.starts_with('V')` check after repair is exactly as strict as before.
///
fn repair_mrv_line1(l: &str) -> String {
    let l = fix_doc_code(&repair_positions(l, &[(2..5, letterize)]));
    format!("{}{}", &l[0..5], fix_name_separator(&defiller(&l[5..])))
}

fn repair_mrv_a_line1(l: &str) -> String {
    repair_mrv_line1(l)
}

fn repair_mrv_b_line1(l: &str) -> String {
    repair_mrv_line1(l)
}

/// Second candidate alongside `repair_mrv_a_line1`/`repair_mrv_b_line1` —
/// see [`unshift_line1_prefix`]'s doc comment for why this is additive, not
/// a replacement, and why the call site must try this one *first*. MRV-A/
/// MRV-B's document code is unconditionally `"V"` plus filler (see
/// `MrvAFields`'/`MrvBFields`' doc comments), so unlike TD2 there is no
/// genuine two-real-letter document code this could mistakenly "fix" — the
/// only reason this is a second candidate rather than an in-place mutation,
/// like the TD3 sibling, is the hit-rate regression documented on
/// [`unshift_line1_prefix`], not a document-code ambiguity here.
fn repair_mrv_a_line1_unshifted(l: &str) -> String {
    let target_width = l.len();
    unshift_line1_prefix(repair_mrv_a_line1(l), target_width)
}

/// See [`repair_mrv_a_line1_unshifted`] — identical reasoning, MRV-B side.
fn repair_mrv_b_line1_unshifted(l: &str) -> String {
    let target_width = l.len();
    unshift_line1_prefix(repair_mrv_b_line1(l), target_width)
}

/// Third candidate alongside `repair_mrv_a_line1`/`repair_mrv_a_line1_unshifted`
/// — see [`shift_line1_right_at_country`]'s doc comment, and
/// [`repair_td3_line1_shifted`]'s for why the call site must try this one
/// *before* `repair_mrv_a_line1_unshifted`.
fn repair_mrv_a_line1_shifted(l: &str) -> String {
    let target_width = l.len();
    shift_or_unshift_line1(repair_mrv_a_line1(l), target_width)
}

/// See [`repair_mrv_a_line1_shifted`] — identical reasoning, MRV-B side.
fn repair_mrv_b_line1_shifted(l: &str) -> String {
    let target_width = l.len();
    shift_or_unshift_line1(repair_mrv_b_line1(l), target_width)
}

/// MRV-A line-2 repair: same geometry as TD3 through the expiry check digit,
/// but positions 28..44 are free-form optional data with no check digit of
/// its own — unlike `repair_td3_line2`, we must NOT digitize/defiller the
/// tail as if it were a personal-number + composite check field, since that
/// data is never checked and unconstrained.
fn repair_mrv_a_line2(l: &str) -> String {
    repair_positions(
        l,
        &[
            (9..10, digitize),   // doc number check digit
            (10..13, letterize), // nationality
            (13..20, digitize),  // DOB + check digit
            (21..28, digitize),  // expiry + check digit
        ],
    )
}

/// MRV-B line-2 repair: same geometry as TD2 through the expiry check digit;
/// positions 28..36 are free-form optional data, left untouched.
fn repair_mrv_b_line2(l: &str) -> String {
    repair_positions(
        l,
        &[
            (9..10, digitize),   // doc number check digit
            (10..13, letterize), // nationality
            (13..20, digitize),  // DOB + check digit
            (21..28, digitize),  // expiry + check digit
        ],
    )
}

/// Scan free-form text (e.g. OCR output) for an MRZ and parse it.
///
/// Tries TD3 (two 44-char lines starting with `P`), then TD1 (three 30-char
/// lines starting with `I`/`A`/`C`), then TD2 (two 36-char lines starting with
/// `I`/`A`/`C`). Tolerates HTML-escaped fillers (`&lt;`, as produced by
/// docling's Markdown) and MRZ lines merged onto a single physical line.
///
/// A reading whose check digits all validate is returned immediately. When no
/// candidate fully validates, the *best-scoring* one — the reading with the
/// most passing check digits — is returned with its honest (partially `false`)
/// [`Checks`], so callers can see how close the read came and decide whether to
/// escalate. [`MrzError::NotFound`] means nothing MRZ-shaped was found at all.
///
/// ```
/// let text = "## PASSPORT\n\nsome OCR noise\n\n\
///     P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
///     L898902C36UTO7408122F1204159ZE184226B<<<<<10\n\n\
///     footer text";
/// let doc = mrz::find_and_parse(text).unwrap();
/// assert_eq!(doc.surname, "ERIKSSON");
/// assert!(doc.valid());
/// ```
pub fn find_and_parse(text: &str) -> Result<MrzData, MrzError> {
    find_and_parse_with(text, &ParseOptions::default())
}

/// [`find_and_parse`] with an explicit [`ParseOptions`].
pub fn find_and_parse_with(text: &str, opts: &ParseOptions) -> Result<MrzData, MrzError> {
    // Markdown/HTML pipelines escape the filler character.
    let text = text.replace("&lt;", "<");
    // OCR often emits several MRZ lines as ONE physical line, space-separated
    // (docling renders the whole zone as a single paragraph) — treat long
    // whitespace-separated tokens as individual candidate lines.
    let mut lines: Vec<&str> = Vec::new();
    for line in text.lines() {
        let tokens: Vec<&str> = line.split_whitespace().filter(|t| t.len() >= 20).collect();
        if tokens.len() >= 2 {
            lines.extend(tokens);
        } else {
            lines.push(line);
        }
    }

    // Best parseable-but-checksum-failed hit, reported when nothing fully
    // validates so callers can see which check digits failed and how close the
    // read came. Ties keep the earlier candidate: the repair pipeline emits its
    // most conservative variants first, so the first reading at a given score
    // is the one that assumed least about the OCR noise.
    let mut fallback: Option<MrzData> = None;
    let mut consider = |data: MrzData| -> Option<MrzData> {
        if data.valid() {
            return Some(data);
        }
        match &fallback {
            Some(best) if best.checks.score() >= data.checks.score() => {}
            _ => fallback = Some(data),
        }
        None
    };

    // The first format whose line-1 document-code prefix was recognized
    // among the *split-line* candidates below, even though no companion line
    // ever combined with it into a full parse — kept only from the
    // split-line loops, never the merged-single-token fast path just above
    // each one: a merged token already carries the full byte length for
    // every field the format has, so if nothing there validates that is a
    // checksum/repair problem, not a missing-line one. Consulted only if
    // `fallback` is still `None` at the very end, to distinguish
    // [`MrzError::IncompleteSequence`] from a bare
    // [`MrzError::NotFound`] — see `knowledge/MRZ_SEQUENCE_COMPLETENESS.md`
    // chunk 4. Purely additive bookkeeping: it never influences which
    // `MrzData` is returned, only which error variant is returned when none
    // is.
    let mut shape_seen: Option<Format> = None;

    // TD3: a line starting with 'P' followed by a candidate line — or both
    // 44-char lines merged into one ~88-char physical line.
    for i in 0..lines.len() {
        let merged = normalize_line(lines[i]);
        if merged.starts_with('P') && (84..=92).contains(&merged.len()) && is_mrz_charset(&merged) {
            let head = &merged[0..44];
            let tail = &merged[44..];
            for l1 in [
                repair_td3_line1_shifted(head),
                repair_td3_line1(head),
                head.to_string(),
            ] {
                for l2 in variants(tail, 44, repair_td3_line2) {
                    if let Ok(data) = parse_td3_with(&l1, &l2, opts) {
                        if let Some(valid) = consider(data) {
                            return Ok(valid);
                        }
                    }
                }
            }
        }

        let l1_candidates = variants(lines[i], 44, repair_td3_line1_shifted)
            .into_iter()
            .chain(variants(lines[i], 44, repair_td3_line1));
        for l1 in l1_candidates {
            if !l1.starts_with('P') {
                continue;
            }
            shape_seen.get_or_insert(Format::Td3);
            for l2_raw in lines.iter().skip(i + 1).take(3) {
                for l2 in variants(l2_raw, 44, repair_td3_line2) {
                    if let Ok(data) = parse_td3_with(&l1, &l2, opts) {
                        if let Some(valid) = consider(data) {
                            return Ok(valid);
                        }
                    }
                }
            }
        }
    }

    // MRV-B: a line starting with 'V' followed by a candidate line — or both
    // 36-char lines merged into one ~68-76 char physical line. Tried before
    // MRV-A: both share the 'V' document-code prefix (no length hint from the
    // code itself), and `variants`'s padding tolerance (+14) is far more
    // generous than its truncation tolerance (+4) — trying the *narrower*
    // format first means a genuine MRV-B line never gets loosely padded up
    // and misparsed as MRV-A before MRV-B gets a chance at its exact length.
    for i in 0..lines.len() {
        let merged = normalize_line(lines[i]);
        if merged.starts_with('V') && (68..=76).contains(&merged.len()) && is_mrz_charset(&merged) {
            let head = &merged[0..36];
            let tail = &merged[36..];
            for l1 in [
                repair_mrv_b_line1_shifted(head),
                repair_mrv_b_line1_unshifted(head),
                repair_mrv_b_line1(head),
                head.to_string(),
            ] {
                for l2 in variants(tail, 36, repair_mrv_b_line2) {
                    if let Ok(data) = parse_mrv_b_with(&l1, &l2, opts) {
                        if let Some(valid) = consider(data) {
                            return Ok(valid);
                        }
                    }
                }
            }
        }

        let l1_candidates = variants(lines[i], 36, repair_mrv_b_line1_shifted)
            .into_iter()
            .chain(variants(lines[i], 36, repair_mrv_b_line1_unshifted))
            .chain(variants(lines[i], 36, repair_mrv_b_line1));
        for l1 in l1_candidates {
            if !l1.starts_with('V') {
                continue;
            }
            shape_seen.get_or_insert(Format::MrvB);
            for l2_raw in lines.iter().skip(i + 1).take(3) {
                for l2 in variants(l2_raw, 36, repair_mrv_b_line2) {
                    if let Ok(data) = parse_mrv_b_with(&l1, &l2, opts) {
                        if let Some(valid) = consider(data) {
                            return Ok(valid);
                        }
                    }
                }
            }
        }
    }

    // MRV-A: a line starting with 'V' followed by a candidate line — or both
    // 44-char lines merged into one ~84-92 char physical line. Disjoint from
    // TD3 ('P'-prefixed) and TD1/TD2 (I/A/C-prefixed), so no cannibalization
    // against those; see the MRV-B comment above for why MRV-B runs first.
    for i in 0..lines.len() {
        let merged = normalize_line(lines[i]);
        if merged.starts_with('V') && (84..=92).contains(&merged.len()) && is_mrz_charset(&merged) {
            let head = &merged[0..44];
            let tail = &merged[44..];
            for l1 in [
                repair_mrv_a_line1_shifted(head),
                repair_mrv_a_line1_unshifted(head),
                repair_mrv_a_line1(head),
                head.to_string(),
            ] {
                for l2 in variants(tail, 44, repair_mrv_a_line2) {
                    if let Ok(data) = parse_mrv_a_with(&l1, &l2, opts) {
                        if let Some(valid) = consider(data) {
                            return Ok(valid);
                        }
                    }
                }
            }
        }

        let l1_candidates = variants(lines[i], 44, repair_mrv_a_line1_shifted)
            .into_iter()
            .chain(variants(lines[i], 44, repair_mrv_a_line1_unshifted))
            .chain(variants(lines[i], 44, repair_mrv_a_line1));
        for l1 in l1_candidates {
            if !l1.starts_with('V') {
                continue;
            }
            shape_seen.get_or_insert(Format::MrvA);
            for l2_raw in lines.iter().skip(i + 1).take(3) {
                for l2 in variants(l2_raw, 44, repair_mrv_a_line2) {
                    if let Ok(data) = parse_mrv_a_with(&l1, &l2, opts) {
                        if let Some(valid) = consider(data) {
                            return Ok(valid);
                        }
                    }
                }
            }
        }
    }

    // TD1: three candidate lines starting with I/A/C — tolerating a gap
    // between them (line 2 within the next 3 detected lines after line 1,
    // line 3 within the next 3 after line 2), or all three merged into one
    // ~90-char physical line. Runs before TD2 below deliberately: a genuine
    // TD1 line 1 starts with 'I' too, and `variants`'s padding tolerance
    // (+14) is generous enough that a 30-char TD1 line is a valid-shaped TD2
    // line-1 candidate once padded — see the MRV-B-before-MRV-A comment
    // above for the same cannibalization hazard, guarded here by running
    // first rather than by any length check
    // (`a_genuine_td1_never_parses_as_td2` pins this).
    //
    // The gap tolerance exists because this crate's own OCR probe
    // (`synthpass-bench --dump-ocr`, see knowledge/ROADMAP.md's M6 note)
    // found TD1's three MRZ rows are not always the *next* three detected
    // lines: the OCR engine's internal multi-pass retry loop concatenates
    // several attempts into one text blob, and a pass that fails to detect
    // line 3 as its own region leaves the watermark or a stray repeated
    // line sitting where line 3 "should" be, adjacent to a line 1/line 2
    // pair from a *different* pass. `take(3)`, not an unbounded scan, keeps
    // this from turning into a scan of the whole page — same bound the
    // two-line formats below already use for their own gap tolerance.
    for i in 0..lines.len() {
        let merged = normalize_line(lines[i]);
        if matches!(merged.as_bytes().first(), Some(b'I' | b'A' | b'C'))
            && (86..=94).contains(&merged.len())
            && is_mrz_charset(&merged)
        {
            let head = &merged[0..30];
            let mid = &merged[30..60];
            let tail = &merged[60..];
            for l1 in [
                repair_td1_line1(head),
                repair_td1_line1_unshifted(head),
                head.to_string(),
            ] {
                for l2 in [repair_td1_line2(mid), mid.to_string()] {
                    for l3 in variants(tail, 30, repair_td1_line3) {
                        if let Ok(data) = parse_td1_with(&l1, &l2, &l3, opts) {
                            if let Some(valid) = consider(data) {
                                return Ok(valid);
                            }
                        }
                    }
                }
            }
        }

        let l1_candidates = variants(lines[i], 30, repair_td1_line1)
            .into_iter()
            .chain(variants(lines[i], 30, repair_td1_line1_unshifted));
        for l1 in l1_candidates {
            if !matches!(l1.as_bytes().first(), Some(b'I' | b'A' | b'C')) {
                continue;
            }
            shape_seen.get_or_insert(Format::Td1);
            for (j, l2_raw) in lines.iter().enumerate().skip(i + 1).take(3) {
                for l2 in variants(l2_raw, 30, repair_td1_line2) {
                    for l3_raw in lines.iter().skip(j + 1).take(3) {
                        for l3 in variants(l3_raw, 30, repair_td1_line3) {
                            if let Ok(data) = parse_td1_with(&l1, &l2, &l3, opts) {
                                if let Some(valid) = consider(data) {
                                    return Ok(valid);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // TD2: two 36-char lines starting with I/A/C — or both merged into one
    // ~72-char physical line.
    for &line in &lines {
        let merged = normalize_line(line);
        if (68..=76).contains(&merged.len())
            && is_mrz_charset(&merged)
            && matches!(merged.as_bytes().first(), Some(b'I' | b'A' | b'C'))
        {
            let head = &merged[0..36];
            let tail = &merged[36..];
            for l1 in [
                repair_td2_line1_shifted(head),
                repair_td2_line1(head),
                head.to_string(),
            ] {
                for l2 in variants(tail, 36, repair_td2_line2) {
                    if let Ok(data) = parse_td2_with(&l1, &l2, opts) {
                        if let Some(valid) = consider(data) {
                            return Ok(valid);
                        }
                    }
                }
            }
        }
    }
    for i in 0..lines.len().saturating_sub(1) {
        let l1_candidates = variants(lines[i], 36, repair_td2_line1_shifted)
            .into_iter()
            .chain(variants(lines[i], 36, repair_td2_line1));
        for l1 in l1_candidates {
            if !matches!(l1.as_bytes().first(), Some(b'I' | b'A' | b'C')) {
                continue;
            }
            shape_seen.get_or_insert(Format::Td2);
            for l2 in variants(lines[i + 1], 36, repair_td2_line2) {
                if let Ok(data) = parse_td2_with(&l1, &l2, opts) {
                    if let Some(valid) = consider(data) {
                        return Ok(valid);
                    }
                }
            }
        }
    }

    // Nothing validated on any ordinary variant. Before giving up, try the
    // damaged-capture case (see `crate::repair`): a line that arrived *narrow*
    // because the recognizer dropped a destroyed glyph instead of emitting a
    // placeholder, so every field after the damage is shifted and no amount of
    // lookalike repair can help. Runs only here, at the end, and only when
    // some line already matched a format's shape — a document with no MRZ at
    // all never pays for it.
    if fallback.is_some() {
        if let Some(data) = damaged_pass(&lines, opts) {
            return Ok(data);
        }
    }

    fallback.ok_or_else(|| match shape_seen {
        Some(format) => MrzError::IncompleteSequence {
            format,
            lines_found: 1,
            lines_expected: expected_lines(format),
        },
        None => MrzError::NotFound,
    })
}

/// How many MRZ lines `format` requires — the one place that count lives,
/// so [`MrzError::IncompleteSequence`] doesn't hardcode it at its one call
/// site above.
fn expected_lines(format: Format) -> u8 {
    match format {
        Format::Td1 => 3,
        Format::Td2 | Format::Td3 | Format::MrvA | Format::MrvB => 2,
    }
}

/// A two-line format's parse entry point, as [`damaged_pass`]'s table stores it.
type TwoLineParse = fn(&str, &str, &ParseOptions) -> Result<MrzData, MrzError>;

/// One row of [`damaged_pass`]'s two-line format table: the ICAO line width,
/// the line-1 document-code bytes that identify the format, the per-line
/// lookalike repairs, and the parser. Adding a two-line format is a row here
/// rather than another nested loop.
type TwoLineFormat = (
    usize,
    &'static [u8],
    fn(&str) -> String,
    fn(&str) -> String,
    TwoLineParse,
);

/// Upper bound on parse attempts across the whole damaged pass.
///
/// The pass is a search, and a search inside an OCR retry loop needs a ceiling
/// rather than good intentions — the same reasoning as
/// `checksum::MAX_DEFILL_PASSES`. Generous enough for one narrow line crossed
/// against the ordinary variants of its neighbours; far below anything a user
/// would notice.
const MAX_DAMAGED_ATTEMPTS: usize = 200_000;

/// Concrete candidate readings for a line that is **exactly one** character
/// too narrow, with the missing character swept across every insertion point.
///
/// One is not an arbitrary cutoff. A single destroyed position admits one
/// residue class, which the calendar constraint in [`accept_damaged`] can cut
/// to a unique reading; two destroyed positions leave hundreds of readings that
/// every check digit accepts (`tests/repair.rs` pins this), so no amount of
/// searching can return an answer — it can only return a guess. Attempting it
/// would cost 37² candidates per insertion point for a result this function is
/// required to throw away.
///
/// Empty for any other line, which is what keeps this off the happy path.
fn restored(raw: &str, target: usize, repair: fn(&str) -> String) -> Vec<String> {
    let n = normalize_line(raw);
    if n.len() + 1 != target || !is_mrz_charset(&n) {
        return Vec::new();
    }
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for shaped in crate::repair::width_candidates(&n, target) {
        if !shaped.contains(crate::repair::UNKNOWN) {
            continue;
        }
        for concrete in crate::repair::concrete_fillings(&shaped) {
            for form in [repair(&concrete), concrete] {
                if seen.insert(form.clone()) {
                    out.push(form);
                }
            }
        }
    }
    out
}

/// Concrete candidate readings for a line that arrived **exactly** the target
/// width, with a single glyph swept across [`crate::repair::CONFUSABLES`].
///
/// The mirror image of [`restored`]: that function handles a line one
/// character too narrow, this one a line the right width but with one
/// position misread. Guarded by `n.len() == target` rather than `restored`'s
/// `n.len() + 1 == target`, so the two are mutually exclusive by
/// construction — a line can match at most one of them, and there is no
/// ordering question between the two repair kinds.
///
/// Empty for any other line, which is what keeps this off the happy path.
fn substituted(raw: &str, target: usize, repair: fn(&str) -> String) -> Vec<String> {
    let n = normalize_line(raw);
    if n.len() != target || !is_mrz_charset(&n) {
        return Vec::new();
    }
    let mut seen = std::collections::HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for concrete in crate::repair::substitution_candidates(&n) {
        for form in [repair(&concrete), concrete] {
            if seen.insert(form.clone()) {
                out.push(form);
            }
        }
    }
    out
}

/// A reading recovered from damage has to clear a higher bar than one read
/// cleanly: every check digit valid **and** both dates real calendar dates.
///
/// The dates are load-bearing, not belt-and-braces. A check digit sees a field
/// only mod 10 and the composite sees the same characters, so a single
/// destroyed position admits an entire residue class that *both* digits
/// accept — four readings on the card this was measured against. Three of them
/// are not dates. Without this filter the pass would have to pick one, and a
/// wrong pick is indistinguishable from a proof.
fn accept_damaged(data: &MrzData) -> bool {
    data.valid()
        && data
            .validity(crate::Date::new(2000, 1, 1))
            .dates_well_formed
}

/// Try to recover a record in which exactly one line arrived too narrow.
///
/// Only one line is damaged in the captures this exists for (a punched hole, a
/// finger over one end), so each shape below restores a single line and
/// crosses it against the ordinary variants of the others — linear in the
/// number of restored candidates rather than a product of three searches.
///
/// Returns `Some` only when the surviving readings all agree on the fields
/// that matter. Several genuinely different readings means the MRZ cannot
/// distinguish them, and the honest answer is the ordinary checksum-failed
/// fallback, not the first candidate off the list.
fn damaged_pass(lines: &[&str], opts: &ParseOptions) -> Option<MrzData> {
    let mut budget = MAX_DAMAGED_ATTEMPTS;
    let mut hits: Vec<MrzData> = Vec::new();

    let record = |data: MrzData, hits: &mut Vec<MrzData>| {
        if accept_damaged(&data) && !hits.iter().any(|h| h.mrz_lines == data.mrz_lines) {
            hits.push(data);
        }
    };

    // TD1: three consecutive lines, any one of them narrow.
    for i in 0..lines.len().saturating_sub(2) {
        let (a, b, c) = (lines[i], lines[i + 1], lines[i + 2]);
        let shapes: [(Vec<String>, Vec<String>, Vec<String>); 6] = [
            (
                restored(a, 30, repair_td1_line1),
                variants(b, 30, repair_td1_line2),
                variants(c, 30, repair_td1_line3),
            ),
            (
                variants(a, 30, repair_td1_line1),
                restored(b, 30, repair_td1_line2),
                variants(c, 30, repair_td1_line3),
            ),
            (
                variants(a, 30, repair_td1_line1),
                variants(b, 30, repair_td1_line2),
                restored(c, 30, repair_td1_line3),
            ),
            (
                substituted(a, 30, repair_td1_line1),
                variants(b, 30, repair_td1_line2),
                variants(c, 30, repair_td1_line3),
            ),
            (
                variants(a, 30, repair_td1_line1),
                substituted(b, 30, repair_td1_line2),
                variants(c, 30, repair_td1_line3),
            ),
            (
                variants(a, 30, repair_td1_line1),
                variants(b, 30, repair_td1_line2),
                substituted(c, 30, repair_td1_line3),
            ),
        ];
        for (v1, v2, v3) in shapes {
            for l1 in &v1 {
                if !matches!(l1.as_bytes().first(), Some(b'I' | b'A' | b'C')) {
                    continue;
                }
                for l2 in &v2 {
                    for l3 in &v3 {
                        if budget == 0 {
                            return single(hits);
                        }
                        budget -= 1;
                        if let Ok(data) = parse_td1_with(l1, l2, l3, opts) {
                            record(data, &mut hits);
                        }
                    }
                }
            }
        }
    }

    // Two-line formats: TD3/MRV-A at 44, TD2/MRV-B at 36. Each entry is the
    // line-1 prefix that identifies the format plus its parse function, so a
    // new two-line format is one row rather than another nested loop.
    let two_line: [TwoLineFormat; 4] = [
        (44, b"P", repair_td3_line1, repair_td3_line2, parse_td3_with),
        (
            44,
            b"V",
            repair_mrv_a_line1,
            repair_mrv_a_line2,
            parse_mrv_a_with,
        ),
        (
            36,
            b"IAC",
            repair_td2_line1,
            repair_td2_line2,
            parse_td2_with,
        ),
        (
            36,
            b"V",
            repair_mrv_b_line1,
            repair_mrv_b_line2,
            parse_mrv_b_with,
        ),
    ];
    for i in 0..lines.len().saturating_sub(1) {
        let (a, b) = (lines[i], lines[i + 1]);
        for (width, prefixes, rep1, rep2, parse) in two_line {
            for (v1, v2) in [
                (restored(a, width, rep1), variants(b, width, rep2)),
                (variants(a, width, rep1), restored(b, width, rep2)),
                (substituted(a, width, rep1), variants(b, width, rep2)),
                (variants(a, width, rep1), substituted(b, width, rep2)),
            ] {
                for l1 in &v1 {
                    if !l1.bytes().next().is_some_and(|c| prefixes.contains(&c)) {
                        continue;
                    }
                    for l2 in &v2 {
                        if budget == 0 {
                            return single(hits);
                        }
                        budget -= 1;
                        if let Ok(data) = parse(l1, l2, opts) {
                            record(data, &mut hits);
                        }
                    }
                }
            }
        }
    }

    single(hits)
}

/// The one recovered reading, or `None` if the damage left more than one
/// record standing. Readings that differ only in their raw zone but agree on
/// every extracted field are the same answer reached twice (two insertion
/// points inside one filler run, say) and count as one.
fn single(mut hits: Vec<MrzData>) -> Option<MrzData> {
    let first = hits.first()?.clone();
    let same = |a: &MrzData, b: &MrzData| {
        a.document_number == b.document_number
            && a.date_of_birth == b.date_of_birth
            && a.date_of_expiry == b.date_of_expiry
            && a.surname == b.surname
            && a.given_names == b.given_names
            && a.nationality == b.nationality
    };
    if hits.iter().all(|h| same(h, &first)) {
        return Some(hits.remove(0));
    }
    None
}
