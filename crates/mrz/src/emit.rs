//! MRZ emission for all five ICAO 9303 formats — the deterministic inverse of
//! [`crate::parse_td3`], [`crate::parse_td2`], [`crate::parse_td1`],
//! [`crate::parse_mrv_a`], and [`crate::parse_mrv_b`].
//!
//! Given structured field values, [`format_td3`], [`format_td2`],
//! [`format_td1`], [`format_mrv_a`], and [`format_mrv_b`] produce the
//! ICAO-specified lines with every check digit computed via the same
//! [`crate::check_digit`] math the parsers verify against. Feeding the output
//! back through the matching `parse_*` function always yields a record with
//! `valid() == true`.
//!
//! Field widths and offsets mirror the parsers exactly:
//! - **TD3** (two 44-char lines): document code (2) + issuing country (3) +
//!   name field (39) on line 1; document number (9) + check (1) +
//!   nationality (3) + date of birth (6) + check (1) + sex (1) + date of
//!   expiry (6) + check (1) + personal number (14) + check (1) + composite
//!   check (1) on line 2.
//! - **TD2** (two 36-char lines): document code (2) + issuing country (3) +
//!   name field (31) on line 1; document number (9) + check (1) +
//!   nationality (3) + date of birth (6) + check (1) + sex (1) + date of
//!   expiry (6) + check (1) + optional data (7) + composite check (1) on
//!   line 2. TD2 has no separate check digit over the optional-data field.
//! - **TD1** (three 30-char lines): document code (2) + issuing country (3) +
//!   document number (9) + check (1) + optional data 1 (15) on line 1; date
//!   of birth (6) + check (1) + sex (1) + date of expiry (6) + check (1) +
//!   nationality (3) + optional data 2 (11) + composite check (1) on line 2;
//!   name field (30) on line 3. TD1 has no separate check digit over either
//!   optional-data field.
//! - **MRV-A** (two 44-char lines): document code (2, `"V"`) + issuing country
//!   (3) + name field (39) on line 1; document number (9) + check (1) +
//!   nationality (3) + date of birth (6) + check (1) + sex (1) + date of
//!   expiry (6) + check (1) + optional data (16) on line 2. No personal-number
//!   check digit and no composite check digit — MRVs don't have one.
//! - **MRV-B** (two 36-char lines): document code (2, `"V"`) + issuing country
//!   (3) + name field (31) on line 1; document number (9) + check (1) +
//!   nationality (3) + date of birth (6) + check (1) + sex (1) + date of
//!   expiry (6) + check (1) + optional data (8) on line 2. Same as MRV-A: no
//!   personal-number check digit, no composite check digit.
//!
//! Input fields are taken in MRZ-native form (`YYMMDD` dates, uppercase
//! `[A-Z0-9]`) rather than the parser's output form (ISO dates, spaced given
//! names) — this keeps emission lossless and deterministic instead of forcing
//! a fragile un-parse of the parser's normalized/century-inferred output.
//! Any character outside `[A-Z0-9]` (including spaces between given names) is
//! mapped to the filler `<`, and every fixed-width field is padded with `<`
//! or truncated to its exact width — this function never panics and always
//! returns exactly 44+1+44 = 89 characters.
//!
//! The `surname`/`given_names` pair feeding the name field is the one
//! exception to the blanket "non-alphanumeric maps to filler" rule above:
//! ICAO 9303 Part 3 §4.6 defines punctuation-specific handling for names
//! (`knowledge/docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:509-535`)
//! that an apostrophe is dropped with no filler, not mapped to one — see
//! [`clean_name_half`] for the full rule set. Every other field
//! (`document_number`, `personal_number`/`optional_data`) keeps the blanket
//! [`clean`] behavior, per that same Part 3 passage read together with
//! `Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:415`, which specifies
//! those fields as flat alphanumeric-or-filler, with no name-style
//! punctuation carve-out.

use crate::checksum::check_digit;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Raw TD3 sub-fields, in MRZ-native form (see module docs).
///
/// `document_number` and `personal_number` are limited to 9 and 14 characters
/// respectively (the field widths); longer input is silently truncated.
/// `date_of_birth` / `date_of_expiry` are `YYMMDD`, not ISO dates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Td3Fields {
    /// Document code, e.g. `"P"` for passport. Defaults to `"P"`.
    pub document_code: String,
    /// Issuing state (3-letter ICAO code).
    pub issuing_country: String,
    /// Document number, up to 9 characters.
    pub document_number: String,
    /// Primary identifier / surname, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub surname: String,
    /// Secondary identifier / given names, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub given_names: String,
    /// Nationality (3-letter ICAO code).
    pub nationality: String,
    /// Date of birth as `YYMMDD`.
    pub date_of_birth: String,
    /// `"M"`, `"F"`, or `"X"` (unspecified — emitted as the filler `<`).
    pub sex: String,
    /// Date of expiry as `YYMMDD`.
    pub date_of_expiry: String,
    /// Optional personal number, up to 14 characters.
    pub personal_number: Option<String>,
}

impl Default for Td3Fields {
    fn default() -> Self {
        Self {
            document_code: "P".to_string(),
            issuing_country: String::new(),
            document_number: String::new(),
            surname: String::new(),
            given_names: String::new(),
            nationality: String::new(),
            date_of_birth: String::new(),
            sex: String::new(),
            date_of_expiry: String::new(),
            personal_number: None,
        }
    }
}

/// Map any character outside `[A-Z0-9]` (after uppercasing) to the filler `<`.
fn clean(s: &str) -> String {
    s.chars()
        .map(|c| {
            let u = c.to_ascii_uppercase();
            if u.is_ascii_alphanumeric() {
                u
            } else {
                '<'
            }
        })
        .collect()
}

/// Clean and pad/truncate `s` to exactly `width` characters using `<` filler.
fn field(s: &str, width: usize) -> String {
    let cleaned = clean(s);
    let mut chars = cleaned.chars();
    let mut out = String::with_capacity(width);
    for _ in 0..width {
        out.push(chars.next().unwrap_or('<'));
    }
    out
}

/// Clean one half (primary or secondary identifier) of a name field per
/// ICAO 9303 Part 3 §4.6
/// (`knowledge/docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:509-535`).
/// Unlike `clean`, this does **not** map every non-alphanumeric character
/// to the filler `<` — names get their own punctuation rules:
///
/// - Each character is first uppercased via `char::to_uppercase()` (not
///   `to_ascii_uppercase`), then, for **each** resulting char `u` (usually
///   one, but see `ß`→`SS` below):
///   1. if `u` is a Table A national character (Part 3 §6,
///      `:703-807`), it is replaced by its
///      [`Expanded`](crate::TransliterationStyle::Expanded)
///      transliteration — e.g. `Ü`→`UE`, `Ñ`→`N` — with **no** separator
///      pushed around it. This is the transliteration Part 3 `:493` mandates
///      ("The issuing State or organization shall transliterate national
///      characters using only the allowed OCR-B characters") and fixes what
///      was previously silent data loss: `MÜLLER` used to emit as `MLLER`,
///      now `MUELLER`.
///   2. else if `u` is `A`-`Z` or `0`-`9`, it is kept as-is.
///   3. else the existing punctuation rules below apply to `u`.
/// - Hyphen (`:517-521`), comma (`:523-532`), and whitespace each become a
///   single separator filler `<`.
/// - **Apostrophe is dropped entirely, with no filler in its place**
///   (`:511-515`) — e.g. `O'CONNOR` becomes `OCONNOR`, not `O<CONNOR`. This
///   is the one rule the crate got wrong before this function existed.
/// - Every other punctuation character is likewise dropped entirely, with
///   no filler (`:534-535`).
/// - `0`-`9` is kept as-is. Part 3 §4.6 (`:507`) states plainly that
///   "numeric characters shall not be used in the name fields of the MRZ",
///   but — unlike the apostrophe/hyphen/comma cases above — it defines no
///   mapping for one that shows up anyway. Silently rewriting a digit to a
///   filler would erase information with no textual basis in the spec, so
///   this function instead preserves digits verbatim, matching the
///   "alphanumeric survives untouched" behavior `clean` already applies
///   to the document-number and optional-data fields. A digit reaching this
///   function means the caller handed it a name that isn't spec-clean;
///   per CLAUDE.md's OCR philosophy ("every field should be validated"),
///   that validation belongs upstream of emission, not inside it.
///
/// After the per-character pass, consecutive separators collapse to one and
/// leading/trailing separators are trimmed. That collapse is what makes
/// `"ANNA, MARIA"` (a comma *immediately followed by* a space) yield
/// `ANNA<MARIA` rather than `ANNA<<MARIA`, matching
/// `Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:531-532`'s worked example.
/// ICAO 9303 §4.6 encoding of one name component (a surname half or a given-
/// names half), as it would be printed into an MRZ name field: uppercased,
/// transliterated, apostrophes dropped with no filler, hyphens/commas/spaces
/// folded to a single `<`.
///
/// Public so a consumer can ask "could this visual-zone reading have produced
/// that MRZ name field?" using the same encoder that writes them, rather than
/// re-deriving the rules. `mrz::transliterate` alone is not a substitute — its
/// own doc comment notes it performs no filler handling and no truncation, so
/// a name containing an apostrophe or a separator round-trips through it
/// unchanged and then compares unequal to the MRZ form.
pub fn encode_name_component(s: &str) -> String {
    clean_name_half(s)
}

fn clean_name_half(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_sep = false;
    for c in s.chars() {
        for u in c.to_uppercase() {
            if let Some(t) = crate::translit::transliterate_char(
                u,
                crate::translit::TransliterationStyle::Expanded,
            ) {
                out.push_str(t);
                last_was_sep = false;
            } else if u.is_ascii_alphanumeric() {
                out.push(u);
                last_was_sep = false;
            } else if u == '\'' {
                // Apostrophe: dropped, no filler (:511-515). `last_was_sep`
                // is deliberately left unchanged — an apostrophe is
                // transparent to separator collapsing on either side of it.
            } else if (u == '-' || u == ',' || u.is_whitespace()) && !last_was_sep {
                out.push('<');
                last_was_sep = true;
            }
            // Every other punctuation character: dropped, no filler (:534-535).
        }
    }
    // Trim leading/trailing separators left over from a name starting or
    // ending on a hyphen/comma/space (e.g. a stray leading space).
    out.trim_matches('<').to_string()
}

/// Truncate the `primary`/`secondary` name-field components (already run
/// through [`clean_name_half`]) to fit exactly `width` characters, when
/// their combined length exceeds it.
///
/// ICAO 9303 Part 4 §4.2.3 (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:463-587`)
/// illustrates *several* issuer-discretionary truncation strategies —
/// truncating trailing components to initials, truncating components to a
/// shorter fixed length, or dropping whole components from the end. Part 4
/// does not prefer any one of these, so *which* characters get removed is a
/// genuine choice.
///
/// **What is not a choice** is the `shall` in the Data Element Directory row
/// that §4.2.3 itself cross-references
/// (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:407`, repeated verbatim
/// in Part 5:346, Part 6:299, Part 7:282 and Part 7:640):
///
/// > "Characters **shall** be removed from one or more components of the
/// > primary identifier until three character positions are freed, and two
/// > filler characters (`<<`) and the first character of the first component
/// > of the secondary identifier can be inserted. The last character shall be
/// > an alphabetic character (A through Z). ... **Further** truncation of the
/// > primary identifier **may** be carried out to allow characters of the
/// > secondary identifier to be included ... When the name consists of only a
/// > primary identifier which exceeds [the width], characters shall be removed
/// > from one or more components of the name until the last character in the
/// > name field is an alphabetic character."
///
/// So the normative rules this function guarantees are:
/// 1. a name that fits within `width` is never truncated — callers only reach
///    this function when it does not fit (`:465`);
/// 2. the **last character of the name field** is alphabetic `A`-`Z`, the
///    reader's only signal that truncation occurred (`:467`);
/// 3. when a secondary identifier exists, `<<` **and at least the first
///    character of its first component always survive** — the primary is
///    shortened to make room, never the other way around (`:407`);
/// 4. when there is no secondary identifier, the primary is shortened until
///    the field ends on a letter (`:407`).
///
/// Rule 3 is the one this function got wrong until the fix recorded in
/// `changelog.d/mrz-name-truncation-icao.fixed.md`: it filled left-to-right
/// and stopped at the first overflow, so a primary identifier that reached
/// `width` on its own consumed the whole field and the `<<` separator was
/// never emitted. `parse_*` then split on the single `<`
/// fallback and silently reported the *first* primary component as `surname`
/// with the remaining primary components as `given_names` — corruption, not
/// merely loss.
///
/// The strategy chosen for rule 3 reproduces ICAO's own §4.2.3.3(b) worked
/// example byte-for-byte (see [`shrink_primary`]). The narrower formats
/// (TD1 width 30, TD2/MRV-B width 31) run the same width-parameterised
/// algorithm and satisfy rules 1-4, but do **not** reproduce ICAO's
/// narrow-field illustrations byte-for-byte — those illustrations contradict
/// each other at the same width 31 (Part 6 §4.2.3.2(b) shows `<<D<P`, Part 7
/// §7.2.3.2(b) shows `<<DINGO`), so they are drawings of the discretion in
/// rule 3's "further truncation may", not an algorithm to match.
///
/// `Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:585-587` documents that
/// this ambiguity runs both ways:
/// a name that happens to fill the field exactly, with no truncation at
/// all, is indistinguishable from a truncated one by the same rule — "this
/// name has not been truncated but it must be assumed that it has been
/// truncated". That is spec-documented ambiguity to design around, not a
/// bug to fix later; ICAO's own `PAPANDROPOULOUS` example
/// (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:584`) is exactly this
/// case, and is pinned as a non-truncating golden vector in
/// `tests/icao_vectors.rs` for that reason.
fn truncate_name_components(primary: &str, secondary: &str, width: usize) -> String {
    let primary_parts: Vec<&str> = primary.split('<').filter(|c| !c.is_empty()).collect();
    let secondary_parts: Vec<&str> = secondary.split('<').filter(|c| !c.is_empty()).collect();

    // The primary identifier's length if every component were kept whole,
    // `<`-separated. Rule 3 needs three more positions than this (`<<` plus
    // one secondary letter) for the secondary identifier to survive.
    let primary_whole = joined_len(&primary_parts);

    if secondary_parts.is_empty() || primary_whole + 3 <= width {
        // Either there is no secondary identifier to protect (rule 4), or the
        // primary already leaves room for `<<` and at least one secondary
        // letter, so the plain left-to-right fill cannot violate rule 3. This
        // is the path ICAO's §4.2.3.2(a)/(b) secondary-truncation examples
        // take, and it reproduces them byte-for-byte — do not "simplify" it
        // into the reserve path below.
        return fill_components(&component_pairs(&primary_parts, &secondary_parts), width);
    }

    // Rule 3, the case this function used to get wrong: the primary would
    // consume the whole field. Reserve the secondary's share *first*, then
    // shrink the primary into what is left.
    let first_secondary = secondary_parts[0];
    // At least one primary character must survive, so the reserve can never
    // take the whole field.
    let reserve_len = (2 + first_secondary.len()).min(width.saturating_sub(1));
    let primary_budget = width - reserve_len;

    let primary_text = shrink_primary(&primary_parts, primary_budget);
    // Whatever the primary could not use goes back to the secondary — which
    // is exactly rule 3's "further truncation ... to allow characters of the
    // secondary identifier to be included". Slack is zero for every ICAO
    // worked example; it is non-zero only when the primary's components are
    // too short and too many to land on `primary_budget` exactly.
    let tail_budget = width - primary_text.len();
    let tail = fill_components(&secondary_pairs(&secondary_parts), tail_budget);
    format!("{primary_text}{tail}")
}

/// Length of `parts` joined by a single `<`, without building the string.
fn joined_len(parts: &[&str]) -> usize {
    let letters: usize = parts.iter().map(|c| c.len()).sum();
    letters + parts.len().saturating_sub(1)
}

/// (separator-before, component) pairs in emission order: the first component
/// has an empty separator, same-half components are joined by `<`, and the
/// primary/secondary boundary by `<<`.
fn component_pairs<'a>(primary: &[&'a str], secondary: &[&'a str]) -> Vec<(&'static str, &'a str)> {
    let mut pairs: Vec<(&'static str, &'a str)> = Vec::new();
    for (i, component) in primary.iter().enumerate() {
        pairs.push((if i == 0 { "" } else { "<" }, component));
    }
    pairs.extend(secondary_pairs(secondary));
    pairs
}

/// [`component_pairs`] for a secondary identifier emitted on its own — the
/// first component carries the `<<` boundary separator.
fn secondary_pairs<'a>(secondary: &[&'a str]) -> Vec<(&'static str, &'a str)> {
    secondary
        .iter()
        .enumerate()
        .map(|(i, component)| (if i == 0 { "<<" } else { "<" }, *component))
        .collect()
}

/// Emit `pairs` left-to-right into at most `width` characters, cutting the
/// last component short rather than emitting a separator nothing follows, so
/// the result always ends on a letter (rule 2).
fn fill_components(pairs: &[(&str, &str)], width: usize) -> String {
    let mut out = String::with_capacity(width);
    for (separator, component) in pairs {
        let remaining = width.saturating_sub(out.len());
        if remaining == 0 {
            break;
        }
        let full_len = separator.len() + component.len();
        if full_len <= remaining {
            out.push_str(separator);
            out.push_str(component);
            continue;
        }
        // This component doesn't fit whole. Prefer letters over a
        // separator that nothing would follow, so the field always ends on
        // an alphabetic character:
        if separator.len() < remaining {
            out.push_str(separator);
            let take = remaining - separator.len();
            out.push_str(&component[..take]);
        } else {
            // Not even room for the separator — borrow straight from this
            // component's letters instead.
            let take = remaining.min(component.len());
            out.push_str(&component[..take]);
        }
        break;
    }
    out
}

/// Shorten the primary identifier's `parts` to at most `budget` characters,
/// `<`-separated, always ending on a letter.
///
/// Reproduces ICAO 9303 Part 4 §4.2.3.3(b)
/// (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:559-566`) byte-for-byte,
/// which is the only width-39 worked example of "one or more components
/// truncated" and therefore the only place ICAO pins actual per-component
/// lengths. Those lengths are what fix the strategy below; they are not
/// derivable from the prose:
///
/// ```text
/// BENNELONG WOOLOOMOOLOO WARRANDYTE WARNAMBOOL  ->  lengths [9, 12, 10, 10]
/// budget 32 (= 39 - len("<<DINGO"))
///   keep the first component whole                 head = 9
///   water-fill the rest over 29 - 9 = 20:
///     cap 6 -> 6+6+6 = 18 <= 20      cap 7 -> 21 > 20      so cap = 6
///     spread the remaining 2 one at a time, left to right  -> [7, 7, 6]
///   BENNELONG<WOOLOOM<WARRAND<WARNAM                       = 32
/// ```
///
/// The one-at-a-time spread is load-bearing: handing the whole remainder to
/// the first component that can take it yields `[8, 6, 6]`
/// (`BENNELONG<WOOLOOMOO<WARRAN<WARNAM`), which is a *conformant* reading but
/// not ICAO's, and the golden vector would fail.
fn shrink_primary(parts: &[&str], budget: usize) -> String {
    if parts.is_empty() || budget == 0 {
        return String::new();
    }
    // Each component needs at least one letter and `k` components need `k-1`
    // separators, so `2k - 1 <= budget` caps how many can appear at all.
    let k = parts.len().min(budget.div_ceil(2));
    let content = budget - (k - 1); // positions left for letters
    let rest = &parts[1..k];
    // Keep the first component whole when it fits, leaving at least one
    // letter for each of the others.
    let head = parts[0].len().min(content - rest.len());
    let room = content - head;

    // Largest equal per-component cap that fits in `room`.
    let mut cap = 0;
    for candidate in 1..=room {
        let used: usize = rest.iter().map(|c| c.len().min(candidate)).sum();
        if used > room {
            break;
        }
        cap = candidate;
    }
    let mut alloc: Vec<usize> = rest.iter().map(|c| c.len().min(cap)).collect();

    // Spread what the cap left over one character at a time, left to right.
    let mut left = room - alloc.iter().sum::<usize>();
    while left > 0 {
        let mut progressed = false;
        for (allocated, component) in alloc.iter_mut().zip(rest) {
            if left == 0 {
                break;
            }
            if *allocated < component.len() {
                *allocated += 1;
                left -= 1;
                progressed = true;
            }
        }
        if !progressed {
            break; // every component is already whole
        }
    }

    let mut out = String::with_capacity(budget);
    out.push_str(&parts[0][..head]);
    for (component, allocated) in rest.iter().zip(&alloc) {
        out.push('<');
        out.push_str(&component[..*allocated]);
    }
    out
}

/// Build the name field: `SURNAME<<GIVEN<NAMES`, padded/truncated to
/// `width`. Each half is cleaned independently by [`clean_name_half`] per
/// ICAO 9303 Part 3 §4.6 before joining — see that function's doc comment
/// for the punctuation rules — and, when the combined result overflows
/// `width`, [`truncate_name_components`] applies this crate's truncation
/// strategy (documented there).
fn name_field(surname: &str, given_names: &str, width: usize) -> String {
    let primary = clean_name_half(surname);
    let secondary = clean_name_half(given_names);
    // `format!("{p}<<")` then padding with fillers is byte-identical to
    // padding `p` alone when `secondary` is empty (both just place `<` in
    // every position after `p`), so the empty-secondary case needs no
    // special-casing here.
    let combined = format!("{primary}<<{secondary}");
    if combined.len() <= width {
        // `field` re-cleans its input via `clean`, but `combined` already
        // contains only `[A-Z0-9<]`, on which `clean` is a no-op — so this
        // is purely padding/truncation to `width`, not a second cleaning
        // pass with different rules.
        return field(&combined, width);
    }
    let truncated = truncate_name_components(&primary, &secondary, width);
    field(&truncated, width)
}

/// The 9-character document-number field, its check-digit character, and any
/// prefix the overflow encoding forces onto the optional-data field.
struct DocNumber {
    field: String,
    check: char,
    /// Overflow prefix for the optional field: `remainder + check digit + '<'`.
    /// Empty when the number fits its 9 characters.
    optional_prefix: String,
}

/// Encode a document number, using the ICAO 9303 Part 5 note j / Part 6 note j
/// long-document-number overflow form when it exceeds 9 characters and the
/// remainder fits `optional_width`. Part 4 defines no such rule for TD3 —
/// applying it there is a deliberate extension by analogy; see
/// `parser::read_overflow`'s doc comment for the full citation.
///
/// The overflow form prints all nine principal characters, sets the
/// check-digit position to a filler (instead of a check digit, per Part 5
/// note j / §4.2.4) to signal a truncated number, and writes
/// `remainder + check digit over the whole number + a terminating filler` at
/// the start of the optional field. The check digit is computed over the nine
/// principal characters concatenated with the remainder — i.e. `cleaned`
/// itself. If the remainder cannot fit, the number is truncated to 9 as
/// before — this function never produces an over-long field.
fn doc_number(number: &str, optional_width: usize) -> DocNumber {
    let cleaned = clean(number);
    let remainder_len = cleaned.len().saturating_sub(9);
    if cleaned.len() > 9 && remainder_len + 2 <= optional_width {
        let check = digit_char(&cleaned);
        return DocNumber {
            field: cleaned[0..9].to_string(),
            check: '<',
            optional_prefix: format!("{}{check}<", &cleaned[9..]),
        };
    }
    let field = field(&cleaned, 9);
    DocNumber {
        check: digit_char(&field),
        field,
        optional_prefix: String::new(),
    }
}

fn digit_char(field: &str) -> char {
    // `field` is always built from `clean`/`field`, i.e. only `[A-Z0-9<]`,
    // so `check_digit` can never fail here.
    char::from_digit(check_digit(field).expect("MRZ-charset field"), 10).expect("0-9")
}

/// Emit a TD3 (passport) MRZ: two 44-character lines joined by `\n`.
///
/// All four field check digits and the composite check digit are computed
/// from `fields` — the result always round-trips through
/// [`crate::parse_td3`] with `valid() == true` (see `tests/roundtrip.rs`).
///
/// ```
/// use mrz::{format_td3, parse_td3, Td3Fields};
///
/// let fields = Td3Fields {
///     document_code: "P".into(),
///     issuing_country: "UTO".into(),
///     document_number: "L898902C3".into(),
///     surname: "ERIKSSON".into(),
///     given_names: "ANNA MARIA".into(),
///     nationality: "UTO".into(),
///     date_of_birth: "740812".into(),
///     sex: "F".into(),
///     date_of_expiry: "120415".into(),
///     personal_number: Some("ZE184226B".into()),
/// };
/// let mrz = format_td3(&fields);
/// let (l1, l2) = mrz.split_once('\n').unwrap();
/// assert!(parse_td3(l1, l2).unwrap().valid());
/// ```
///
/// # National characters
///
/// Doc 9303 Part 3 §6 A defines a recommended transliteration for Latin
/// national characters that do not fit the MRZ's `[A-Z0-9<]` alphabet. It is
/// applied automatically, so accented letters survive instead of being
/// silently deleted — see [`crate::transliterate`] for the cases where the
/// standard admits more than one correct answer.
///
/// ```
/// use mrz::{format_td3, Td3Fields};
///
/// let lines = format_td3(&Td3Fields {
///     issuing_country: "DEU".into(),
///     surname: "MÜLLER".into(),
///     given_names: "TÉRÈSA".into(),
///     ..Default::default()
/// });
/// let line1 = lines.split_once('\n').unwrap().0;
/// assert!(line1.contains("MUELLER"));
/// assert!(line1.contains("TERESA"));
/// ```
pub fn format_td3(fields: &Td3Fields) -> String {
    let doc_code = field(&fields.document_code, 2);
    let issuing = field(&fields.issuing_country, 3);
    let name = name_field(&fields.surname, &fields.given_names, 39);
    let line1 = format!("{doc_code}{issuing}{name}");

    let DocNumber {
        field: doc_num,
        check: doc_num_check,
        optional_prefix,
    } = doc_number(&fields.document_number, 14);
    let nationality = field(&fields.nationality, 3);
    let dob = field(&fields.date_of_birth, 6);
    let dob_check = digit_char(&dob);
    let sex = match fields.sex.to_ascii_uppercase().as_str() {
        "M" => 'M',
        "F" => 'F',
        _ => '<',
    };
    let expiry = field(&fields.date_of_expiry, 6);
    let expiry_check = digit_char(&expiry);
    let personal = field(
        &format!(
            "{optional_prefix}{}",
            clean(fields.personal_number.as_deref().unwrap_or(""))
        ),
        14,
    );
    let personal_check = digit_char(&personal);

    // 43 chars: everything except the composite digit itself, in the same
    // layout `parser::parse_td3` expects (offsets 0..43).
    let line2_body = format!(
        "{doc_num}{doc_num_check}{nationality}{dob}{dob_check}{sex}{expiry}{expiry_check}{personal}{personal_check}"
    );
    debug_assert_eq!(line2_body.len(), 43);

    // Composite: line2[0..10] (doc number + check) + line2[13..20]
    // (DOB + check) + line2[21..43] (expiry + check + personal + check) —
    // mirrors `parser::parse_td3`'s `checks.composite` computation exactly.
    let composite_input = format!(
        "{}{}{}",
        &line2_body[0..10],
        &line2_body[13..20],
        &line2_body[21..43]
    );
    let composite_check = digit_char(&composite_input);

    let line2 = format!("{line2_body}{composite_check}");
    debug_assert_eq!(line1.len(), 44);
    debug_assert_eq!(line2.len(), 44);

    format!("{line1}\n{line2}")
}

/// Raw TD2 sub-fields, in MRZ-native form (see module docs).
///
/// `document_number` is limited to 9 characters (the field width); longer
/// input is silently truncated. `date_of_birth` / `date_of_expiry` are
/// `YYMMDD`, not ISO dates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Td2Fields {
    /// Document code, e.g. `"I"` for identity card. Defaults to `"I"`.
    pub document_code: String,
    /// Issuing state (3-letter ICAO code).
    pub issuing_country: String,
    /// Document number, up to 9 characters.
    pub document_number: String,
    /// Primary identifier / surname, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub surname: String,
    /// Secondary identifier / given names, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub given_names: String,
    /// Nationality (3-letter ICAO code).
    pub nationality: String,
    /// Date of birth as `YYMMDD`.
    pub date_of_birth: String,
    /// `"M"`, `"F"`, or `"X"` (unspecified — emitted as the filler `<`).
    pub sex: String,
    /// Date of expiry as `YYMMDD`.
    pub date_of_expiry: String,
    /// Optional data, up to 7 characters. TD2 has no check digit over this
    /// field on its own — it only feeds the composite check.
    pub optional_data: Option<String>,
}

impl Default for Td2Fields {
    fn default() -> Self {
        Self {
            document_code: "I".to_string(),
            issuing_country: String::new(),
            document_number: String::new(),
            surname: String::new(),
            given_names: String::new(),
            nationality: String::new(),
            date_of_birth: String::new(),
            sex: String::new(),
            date_of_expiry: String::new(),
            optional_data: None,
        }
    }
}

/// Emit a TD2 (identity-card) MRZ: two 36-character lines joined by `\n`.
///
/// The document number, date-of-birth, date-of-expiry, and composite check
/// digits are computed from `fields` — the result always round-trips through
/// [`crate::parse_td2`] with `valid() == true` (see `tests/roundtrip.rs`).
/// TD2 has no separate check digit over the optional-data field.
///
/// ```
/// use mrz::{format_td2, parse_td2, Td2Fields};
///
/// let fields = Td2Fields {
///     document_number: "D23145890".into(),
///     surname: "ERIKSSON".into(),
///     given_names: "ANNA MARIA".into(),
///     nationality: "UTO".into(),
///     date_of_birth: "740812".into(),
///     sex: "F".into(),
///     date_of_expiry: "120415".into(),
///     ..Td2Fields::default()
/// };
/// let mrz = format_td2(&fields);
/// let (l1, l2) = mrz.split_once('\n').unwrap();
/// assert!(parse_td2(l1, l2).unwrap().valid());
/// ```
pub fn format_td2(fields: &Td2Fields) -> String {
    let doc_code = field(&fields.document_code, 2);
    let issuing = field(&fields.issuing_country, 3);
    let name = name_field(&fields.surname, &fields.given_names, 31);
    let line1 = format!("{doc_code}{issuing}{name}");
    debug_assert_eq!(line1.len(), 36);

    let DocNumber {
        field: doc_num,
        check: doc_num_check,
        optional_prefix,
    } = doc_number(&fields.document_number, 7);
    let nationality = field(&fields.nationality, 3);
    let dob = field(&fields.date_of_birth, 6);
    let dob_check = digit_char(&dob);
    let sex = match fields.sex.to_ascii_uppercase().as_str() {
        "M" => 'M',
        "F" => 'F',
        _ => '<',
    };
    let expiry = field(&fields.date_of_expiry, 6);
    let expiry_check = digit_char(&expiry);
    let optional = field(
        &format!(
            "{optional_prefix}{}",
            clean(fields.optional_data.as_deref().unwrap_or(""))
        ),
        7,
    );

    // 35 chars: everything except the composite digit itself, in the same
    // layout `parser::parse_td2` expects (offsets 0..35).
    let line2_body = format!(
        "{doc_num}{doc_num_check}{nationality}{dob}{dob_check}{sex}{expiry}{expiry_check}{optional}"
    );
    debug_assert_eq!(line2_body.len(), 35);

    // Composite: line2[0..10] (doc number + check) + line2[13..20]
    // (DOB + check) + line2[21..35] (expiry + check + optional data) —
    // mirrors `parser::parse_td2`'s `checks.composite` computation exactly.
    let composite_input = format!(
        "{}{}{}",
        &line2_body[0..10],
        &line2_body[13..20],
        &line2_body[21..35]
    );
    let composite_check = digit_char(&composite_input);

    let line2 = format!("{line2_body}{composite_check}");
    debug_assert_eq!(line2.len(), 36);

    format!("{line1}\n{line2}")
}

/// Raw TD1 sub-fields, in MRZ-native form (see module docs).
///
/// `document_number` is limited to 9 characters (the field width); longer
/// input is silently truncated. `date_of_birth` / `date_of_expiry` are
/// `YYMMDD`, not ISO dates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Td1Fields {
    /// Document code, e.g. `"I"` for identity card. Defaults to `"I"`.
    pub document_code: String,
    /// Issuing state (3-letter ICAO code).
    pub issuing_country: String,
    /// Document number, up to 9 characters.
    pub document_number: String,
    /// Optional data on line 1, up to 15 characters. TD1 has no separate
    /// check digit over this field on its own — it only feeds the composite.
    pub optional_data_1: Option<String>,
    /// Primary identifier / surname, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub surname: String,
    /// Secondary identifier / given names, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub given_names: String,
    /// Nationality (3-letter ICAO code).
    pub nationality: String,
    /// Date of birth as `YYMMDD`.
    pub date_of_birth: String,
    /// `"M"`, `"F"`, or `"X"` (unspecified — emitted as the filler `<`).
    pub sex: String,
    /// Date of expiry as `YYMMDD`.
    pub date_of_expiry: String,
    /// Optional data on line 2, up to 11 characters. TD1 has no separate
    /// check digit over this field on its own — it only feeds the composite.
    pub optional_data_2: Option<String>,
}

impl Default for Td1Fields {
    fn default() -> Self {
        Self {
            document_code: "I".to_string(),
            issuing_country: String::new(),
            document_number: String::new(),
            optional_data_1: None,
            surname: String::new(),
            given_names: String::new(),
            nationality: String::new(),
            date_of_birth: String::new(),
            sex: String::new(),
            date_of_expiry: String::new(),
            optional_data_2: None,
        }
    }
}

/// Emit a TD1 (ID-card) MRZ: three 30-character lines joined by `\n`.
///
/// The document number, date-of-birth, date-of-expiry, and composite check
/// digits are computed from `fields` — the result always round-trips through
/// [`crate::parse_td1`] with `valid() == true` (see `tests/roundtrip.rs`).
/// TD1 has no separate check digit over either optional-data field.
///
/// ```
/// use mrz::{format_td1, parse_td1, Td1Fields};
///
/// let fields = Td1Fields {
///     document_number: "D23145890".into(),
///     surname: "ERIKSSON".into(),
///     given_names: "ANNA MARIA".into(),
///     nationality: "UTO".into(),
///     date_of_birth: "740812".into(),
///     sex: "F".into(),
///     date_of_expiry: "120415".into(),
///     ..Td1Fields::default()
/// };
/// let mrz = format_td1(&fields);
/// let mut lines = mrz.lines();
/// let l1 = lines.next().unwrap();
/// let l2 = lines.next().unwrap();
/// let l3 = lines.next().unwrap();
/// assert!(parse_td1(l1, l2, l3).unwrap().valid());
/// ```
pub fn format_td1(fields: &Td1Fields) -> String {
    let doc_code = field(&fields.document_code, 2);
    let issuing = field(&fields.issuing_country, 3);
    let DocNumber {
        field: doc_num,
        check: doc_num_check,
        optional_prefix,
    } = doc_number(&fields.document_number, 15);
    let optional1 = field(
        &format!(
            "{optional_prefix}{}",
            clean(fields.optional_data_1.as_deref().unwrap_or(""))
        ),
        15,
    );
    let line1 = format!("{doc_code}{issuing}{doc_num}{doc_num_check}{optional1}");
    debug_assert_eq!(line1.len(), 30);

    let dob = field(&fields.date_of_birth, 6);
    let dob_check = digit_char(&dob);
    let sex = match fields.sex.to_ascii_uppercase().as_str() {
        "M" => 'M',
        "F" => 'F',
        _ => '<',
    };
    let expiry = field(&fields.date_of_expiry, 6);
    let expiry_check = digit_char(&expiry);
    let nationality = field(&fields.nationality, 3);
    let optional2 = field(fields.optional_data_2.as_deref().unwrap_or(""), 11);

    // 29 chars: everything except the composite digit itself, in the same
    // layout `parser::parse_td1` expects (offsets 0..29).
    let line2_body = format!("{dob}{dob_check}{sex}{expiry}{expiry_check}{nationality}{optional2}");
    debug_assert_eq!(line2_body.len(), 29);

    // Composite: line1[5..30] (doc number + check + optional-1) +
    // line2[0..7] (DOB + check) + line2[8..15] (expiry + check) +
    // line2[18..29] (optional-2) — mirrors `parser::parse_td1`'s
    // `checks.composite` computation exactly.
    let composite_input = format!(
        "{}{}{}{}",
        &line1[5..30],
        &line2_body[0..7],
        &line2_body[8..15],
        &line2_body[18..29]
    );
    let composite_check = digit_char(&composite_input);

    let line2 = format!("{line2_body}{composite_check}");
    debug_assert_eq!(line2.len(), 30);

    let line3 = name_field(&fields.surname, &fields.given_names, 30);
    debug_assert_eq!(line3.len(), 30);

    format!("{line1}\n{line2}\n{line3}")
}

/// Raw MRV-A sub-fields, in MRZ-native form (see module docs).
///
/// `document_number` is limited to 9 characters (the field width); longer
/// input is silently truncated. `date_of_birth` / `date_of_expiry` are
/// `YYMMDD`, not ISO dates. MRV-A has no personal-number or composite check
/// digit — `optional_data` is free-form data up to 16 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MrvAFields {
    /// Document code, always `"V"` for a visa. Defaults to `"V"`.
    pub document_code: String,
    /// Issuing state (3-letter ICAO code).
    pub issuing_country: String,
    /// Document number, up to 9 characters.
    pub document_number: String,
    /// Primary identifier / surname, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub surname: String,
    /// Secondary identifier / given names, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub given_names: String,
    /// Nationality (3-letter ICAO code).
    pub nationality: String,
    /// Date of birth as `YYMMDD`.
    pub date_of_birth: String,
    /// `"M"`, `"F"`, or `"X"` (unspecified — emitted as the filler `<`).
    pub sex: String,
    /// Date of expiry as `YYMMDD`.
    pub date_of_expiry: String,
    /// Optional free-form data, up to 16 characters. No check digit covers
    /// this field — MRVs have neither a personal-number nor composite check.
    pub optional_data: Option<String>,
}

impl Default for MrvAFields {
    fn default() -> Self {
        Self {
            document_code: "V".to_string(),
            issuing_country: String::new(),
            document_number: String::new(),
            surname: String::new(),
            given_names: String::new(),
            nationality: String::new(),
            date_of_birth: String::new(),
            sex: String::new(),
            date_of_expiry: String::new(),
            optional_data: None,
        }
    }
}

/// Emit an MRV-A (machine readable visa) MRZ: two 44-character lines joined
/// by `\n`.
///
/// The document number, date-of-birth, and date-of-expiry check digits are
/// computed from `fields` — the result always round-trips through
/// [`crate::parse_mrv_a`] with `valid() == true` (see `tests/roundtrip.rs`).
/// MRV-A has no personal-number check digit and no composite check digit.
///
/// ```
/// use mrz::{format_mrv_a, parse_mrv_a, MrvAFields};
///
/// let fields = MrvAFields {
///     document_number: "L898902C".into(),
///     surname: "ERIKSSON".into(),
///     given_names: "ANNA MARIA".into(),
///     nationality: "UTO".into(),
///     date_of_birth: "690806".into(),
///     sex: "F".into(),
///     date_of_expiry: "940623".into(),
///     ..MrvAFields::default()
/// };
/// let mrz = format_mrv_a(&fields);
/// let (l1, l2) = mrz.split_once('\n').unwrap();
/// assert!(parse_mrv_a(l1, l2).unwrap().valid());
/// ```
pub fn format_mrv_a(fields: &MrvAFields) -> String {
    let doc_code = field(&fields.document_code, 2);
    let issuing = field(&fields.issuing_country, 3);
    let name = name_field(&fields.surname, &fields.given_names, 39);
    let line1 = format!("{doc_code}{issuing}{name}");
    debug_assert_eq!(line1.len(), 44);

    let doc_num = field(&fields.document_number, 9);
    let doc_num_check = digit_char(&doc_num);
    let nationality = field(&fields.nationality, 3);
    let dob = field(&fields.date_of_birth, 6);
    let dob_check = digit_char(&dob);
    let sex = match fields.sex.to_ascii_uppercase().as_str() {
        "M" => 'M',
        "F" => 'F',
        _ => '<',
    };
    let expiry = field(&fields.date_of_expiry, 6);
    let expiry_check = digit_char(&expiry);
    let optional = field(fields.optional_data.as_deref().unwrap_or(""), 16);

    let line2 = format!(
        "{doc_num}{doc_num_check}{nationality}{dob}{dob_check}{sex}{expiry}{expiry_check}{optional}"
    );
    debug_assert_eq!(line2.len(), 44);

    format!("{line1}\n{line2}")
}

/// Raw MRV-B sub-fields, in MRZ-native form (see module docs).
///
/// `document_number` is limited to 9 characters (the field width); longer
/// input is silently truncated. `date_of_birth` / `date_of_expiry` are
/// `YYMMDD`, not ISO dates. MRV-B has no personal-number or composite check
/// digit — `optional_data` is free-form data up to 8 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct MrvBFields {
    /// Document code, always `"V"` for a visa. Defaults to `"V"`.
    pub document_code: String,
    /// Issuing state (3-letter ICAO code).
    pub issuing_country: String,
    /// Document number, up to 9 characters.
    pub document_number: String,
    /// Primary identifier / surname, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub surname: String,
    /// Secondary identifier / given names, per this module's name-field
    /// punctuation and transliteration rules (see the module docs).
    pub given_names: String,
    /// Nationality (3-letter ICAO code).
    pub nationality: String,
    /// Date of birth as `YYMMDD`.
    pub date_of_birth: String,
    /// `"M"`, `"F"`, or `"X"` (unspecified — emitted as the filler `<`).
    pub sex: String,
    /// Date of expiry as `YYMMDD`.
    pub date_of_expiry: String,
    /// Optional free-form data, up to 8 characters. No check digit covers
    /// this field — MRVs have neither a personal-number nor composite check.
    pub optional_data: Option<String>,
}

impl Default for MrvBFields {
    fn default() -> Self {
        Self {
            document_code: "V".to_string(),
            issuing_country: String::new(),
            document_number: String::new(),
            surname: String::new(),
            given_names: String::new(),
            nationality: String::new(),
            date_of_birth: String::new(),
            sex: String::new(),
            date_of_expiry: String::new(),
            optional_data: None,
        }
    }
}

/// Emit an MRV-B (machine readable visa) MRZ: two 36-character lines joined
/// by `\n`.
///
/// The document number, date-of-birth, and date-of-expiry check digits are
/// computed from `fields` — the result always round-trips through
/// [`crate::parse_mrv_b`] with `valid() == true` (see `tests/roundtrip.rs`).
/// MRV-B has no personal-number check digit and no composite check digit.
///
/// ```
/// use mrz::{format_mrv_b, parse_mrv_b, MrvBFields};
///
/// let fields = MrvBFields {
///     document_number: "L898902C".into(),
///     surname: "ERIKSSON".into(),
///     given_names: "ANNA MARIA".into(),
///     nationality: "UTO".into(),
///     date_of_birth: "690806".into(),
///     sex: "F".into(),
///     date_of_expiry: "940623".into(),
///     ..MrvBFields::default()
/// };
/// let mrz = format_mrv_b(&fields);
/// let (l1, l2) = mrz.split_once('\n').unwrap();
/// assert!(parse_mrv_b(l1, l2).unwrap().valid());
/// ```
pub fn format_mrv_b(fields: &MrvBFields) -> String {
    let doc_code = field(&fields.document_code, 2);
    let issuing = field(&fields.issuing_country, 3);
    let name = name_field(&fields.surname, &fields.given_names, 31);
    let line1 = format!("{doc_code}{issuing}{name}");
    debug_assert_eq!(line1.len(), 36);

    let doc_num = field(&fields.document_number, 9);
    let doc_num_check = digit_char(&doc_num);
    let nationality = field(&fields.nationality, 3);
    let dob = field(&fields.date_of_birth, 6);
    let dob_check = digit_char(&dob);
    let sex = match fields.sex.to_ascii_uppercase().as_str() {
        "M" => 'M',
        "F" => 'F',
        _ => '<',
    };
    let expiry = field(&fields.date_of_expiry, 6);
    let expiry_check = digit_char(&expiry);
    let optional = field(fields.optional_data.as_deref().unwrap_or(""), 8);

    let line2 = format!(
        "{doc_num}{doc_num_check}{nationality}{dob}{dob_check}{sex}{expiry}{expiry_check}{optional}"
    );
    debug_assert_eq!(line2.len(), 36);

    format!("{line1}\n{line2}")
}
