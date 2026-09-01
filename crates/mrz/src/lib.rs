//! ICAO 9303 Machine Readable Zone parser, emitter, and check-digit validator.
//!
//! Zero runtime dependencies so it compiles to native and `wasm32-unknown-unknown`
//! targets alike. Supports:
//! - **TD3** (passports): 2 lines × 44 characters
//! - **TD2** (official travel documents / ID cards): 2 lines × 36 characters
//! - **TD1** (ID cards): 3 lines × 30 characters
//! - **MRV-A** (visas, passport-book size): 2 lines × 44 characters
//! - **MRV-B** (visas, smaller size): 2 lines × 36 characters
//!
//! Check digits use the standard 7-3-1 weighting over the value mapping
//! `0-9 → 0-9`, `A-Z → 10-35`, `< → 0`. A field checksum that validates
//! mathematically proves the OCR read is faithful to the printed document —
//! no probabilistic model involved.
//!
//! ```
//! let doc = mrz::parse_td3(
//!     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
//!     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
//! ).unwrap();
//!
//! assert_eq!(doc.surname, "ERIKSSON");
//! assert_eq!(doc.date_of_birth, "1974-08-12"); // expanded to ISO 8601
//!
//! // Per-field proof, not a single boolean.
//! assert!(doc.checks.document_number);
//! assert!(doc.checks.composite);
//! assert!(doc.valid()); // every check digit verified
//! ```
//!
//! `parse_td1`, `parse_td2`, `parse_mrv_a` and `parse_mrv_b` cover the other
//! formats. See [`README.md`](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/README.md)
//! for the full walkthrough — free-text OCR scanning, emitting, long document
//! numbers, national-character transliteration, and what a check digit
//! cannot prove.
//!
//! The engine is split across (private) modules:
//! - `checksum` — check-digit math and generic OCR-repair primitives
//! - `blindspot` — the substitutions check digits provably cannot catch
//! - `parser` — the TD1/TD2/TD3 parsers and the free-text scanner
//! - `dates` — `YYMMDD` expansion and date-plausibility checks
//! - `countries` — ICAO/ISO 3166-1 code → country name
//!
//! A valid composite check digit proves a faithful *read*; it does not prove
//! the document is in date — see [`MrzData::validity`].

#![warn(missing_docs)]

/// Compiles and runs every Rust example in `README.md` as a doctest, so a
/// README snippet can never drift from the API it demonstrates. `cfg(doctest)`
/// means this is *only* built while collecting doctests — the README is not
/// injected into the rendered crate documentation, which has its own prose
/// above.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "zeroize")]
use zeroize::ZeroizeOnDrop;

mod blindspot;
mod checksum;
mod countries;
mod dates;
mod emit;
mod parser;
mod repair;
mod translit;

pub use blindspot::{blindspot, class_of, collisions, Blindspot, CLASSES};
pub use checksum::{check_digit, verify};
pub use countries::{code_for_name, codes_equivalent, country_name};
pub use dates::{
    date_completeness, expand_date, expand_date_with_pivot, Date, DateCompleteness, DateValidity,
    CURRENT_YY,
};
pub use emit::{
    encode_name_component, format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3,
    MrvAFields, MrvBFields, Td1Fields, Td2Fields, Td3Fields,
};
pub use parser::{
    find_and_parse, find_and_parse_with, parse_mrv_a, parse_mrv_a_with, parse_mrv_b,
    parse_mrv_b_with, parse_td1, parse_td1_with, parse_td2, parse_td2_with, parse_td3,
    parse_td3_with,
};
pub use repair::{
    solve_field, solve_substitution, substitution_candidates, width_candidates, FieldKind,
    Resolution, CONFUSABLES, MRZ_ALPHABET, UNKNOWN,
};
pub use translit::{transliterate, transliterate_char, transliterations, TransliterationStyle};

/// Tunables for the parsing entry points.
///
/// Every `parse_*` / [`find_and_parse`] function is the `ParseOptions::default()`
/// case of its `*_with` counterpart, so existing calls are unaffected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ParseOptions {
    /// Two-digit century pivot for [`expand_date_with_pivot`]. Defaults to
    /// [`CURRENT_YY`]; set it explicitly to pin behaviour instead of inheriting
    /// the constant this crate was compiled with.
    pub pivot_yy: u32,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            pivot_yy: CURRENT_YY,
        }
    }
}

/// Per-field check-digit verification results.
///
/// `#[non_exhaustive]`: a future MRZ format may carry a check digit these five
/// fields don't name, and adding it should not be a breaking change. Construct
/// one from a `parse_*` function rather than by literal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Checks {
    /// The primary document-number field.
    pub document_number: bool,
    /// The date-of-birth field.
    pub date_of_birth: bool,
    /// The date-of-expiry field.
    pub date_of_expiry: bool,
    /// TD3 only; `true` for TD1/TD2 (no such check digit exists there).
    pub personal_number: bool,
    /// The composite check digit over the whole zone.
    pub composite: bool,
}

impl Checks {
    /// All check digits valid — the MRZ read is mathematically verified.
    pub fn all_valid(&self) -> bool {
        self.document_number
            && self.date_of_birth
            && self.date_of_expiry
            && self.personal_number
            && self.composite
    }
}

/// A single, coordinated completeness signal spanning both outcomes of a
/// [`find_and_parse`](crate::find_and_parse) attempt — the "several
/// separate vocabularies for 'is this record complete'" gap
/// `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 5 exists to close.
///
/// Deliberately **not** a struct that duplicates [`MrzData`]'s own fields
/// (a second, independently-stale source of truth for the same facts):
/// [`Complete`](Self::Complete) is built from the values already on
/// `MrzData` via [`MrzData::sequence_completeness`], and
/// [`Partial`](Self::Partial) mirrors [`MrzError::IncompleteSequence`]'s
/// own fields exactly. This type's only job is to be the *one* thing a
/// caller can hold instead of separately branching on
/// `Result<MrzData, MrzError>`'s two differently-shaped arms —
/// [`from_parse_result`](Self::from_parse_result) is the constructor meant
/// for that.
///
/// `lines_found`/`lines_expected` never actually differ inside `Complete`:
/// an `MrzData` cannot exist at all without every line the format's
/// fixed-offset `parse_*` needs already having been found, so a *complete*
/// read has nothing left to report on that axis — which is exactly why
/// those two fields live only on `Partial`, not duplicated onto `Complete`
/// as a pair of always-equal constants.
///
/// Deliberately does **not** fold in
/// `synthpass_core::fusion::check_line1_integrity`'s verdict — that
/// heuristic layer (`document_type`/`issuing_country`/`surname`/
/// `given_names` carry no check digit at all) lives one layer up in
/// `synthpass-core` on purpose; this type unifies only the vocabularies
/// that already live inside `crates/mrz` itself.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum SequenceCompleteness {
    /// Every line the format needs was found and parsed into an
    /// [`MrzData`]. Whether every *field* is proven is a separate
    /// question — see `checks`/`date_of_birth`.
    Complete {
        /// Per-field check-digit proof — see [`MrzData::checks`].
        checks: Checks,
        /// Date-of-birth field completeness — see
        /// [`MrzData::date_of_birth_completeness`]. No analogous signal
        /// exists for date-of-expiry today; that asymmetry is inherited
        /// from [`DateCompleteness`] itself, not introduced here.
        date_of_birth: DateCompleteness,
    },
    /// Fewer than `lines_expected` lines were found — mirrors
    /// [`MrzError::IncompleteSequence`] exactly.
    Partial {
        /// The format whose line 1 was recognized.
        format: Format,
        /// How many of `lines_expected` lines were actually located.
        lines_found: u8,
        /// How many lines `format` requires.
        lines_expected: u8,
    },
}

impl SequenceCompleteness {
    /// Builds the coordinated signal from either arm of a
    /// [`find_and_parse`](crate::find_and_parse) result. `None` when the
    /// error carries no completeness signal of its own — a bare
    /// [`MrzError::NotFound`] (nothing MRZ-shaped at all) or one of the
    /// other structural/checksum error variants, which are about a
    /// *specific* zone a caller already had in hand, not a free-text scan.
    pub fn from_parse_result(result: &Result<MrzData, MrzError>) -> Option<Self> {
        match result {
            Ok(data) => Some(data.sequence_completeness()),
            Err(MrzError::IncompleteSequence {
                format,
                lines_found,
                lines_expected,
            }) => Some(Self::Partial {
                format: *format,
                lines_found: *lines_found,
                lines_expected: *lines_expected,
            }),
            Err(_) => None,
        }
    }
}

/// `#[non_exhaustive]`: ICAO 9303 defines formats this crate does not parse yet
/// (MRP-style variants, future parts), so `match` on this must carry a `_` arm
/// and gaining a variant is not a breaking change. Adding MRV-A/MRV-B in 0.3.0
/// was breaking precisely because this attribute was missing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Format {
    /// Passport (TD3): 2 lines × 44 characters.
    Td3,
    /// Official travel document / ID card (TD2): 2 lines × 36 characters.
    Td2,
    /// ID card (TD1): 3 lines × 30 characters.
    Td1,
    /// Machine readable visa, type A (ICAO 9303 part 7): two 44-char lines,
    /// geometry mirrors TD3 through the expiry check digit, but there is no
    /// personal-number field and no composite check digit.
    MrvA,
    /// Machine readable visa, type B (ICAO 9303 part 7): two 36-char lines,
    /// geometry mirrors TD2 through the expiry check digit, but there is no
    /// personal-number field and no composite check digit.
    MrvB,
}

/// Parsed and validated MRZ data.
///
/// The `zeroize` feature (off by default, kept off for the `wasm32-unknown-unknown`
/// browser-demo build so this crate stays zero-dependency there) derives
/// `ZeroizeOnDrop`, wiping the PII-bearing `String` fields from memory when a
/// value is dropped. `format` and `checks` carry no PII and are `Copy`, so
/// they're `#[zeroize(skip)]`.
///
/// `#[non_exhaustive]`: this struct grows as the crate decodes more of the
/// zone — `document_number_full` arrived in 0.4.0 — and that should not break
/// downstream code. Obtain one from a `parse_*` function; it is an output type
/// and there is no reason to build it by literal.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "zeroize", derive(ZeroizeOnDrop))]
#[non_exhaustive]
pub struct MrzData {
    /// Which ICAO 9303 layout this was parsed from.
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub format: Format,
    /// Document code, e.g. "P" (passport), "ID"/"I" (identity card).
    pub document_type: String,
    /// Issuing state or organization (3-letter ICAO code).
    pub issuing_country: String,
    /// The 9-character document-number field as printed, truncated if the
    /// real number overflows — see [`document_number_full`](Self::document_number_full).
    pub document_number: String,
    /// The reassembled document number when it overflows the 9-character field
    /// (ICAO 9303 Part 5 note j / §4.2.4 for TD1, Part 6 note j for TD2; Part 4
    /// defines no such rule, so TD3 gets it by a deliberate extension — see
    /// `parser::read_overflow`'s doc comment. MRVs have no overflow encoding at
    /// all, per Part 7). `None` when the number fits, in which case
    /// [`document_number`](Self::document_number) is already complete.
    pub document_number_full: Option<String>,
    /// `true` when the long document number was recovered from the pre-0.6
    /// eight-character encoding this crate used to emit, rather than the
    /// Doc 9303 form (nine principal characters, filler in the check-digit
    /// position). Always `false` for conformant zones and when the number
    /// fits its field.
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub document_number_legacy_encoding: bool,
    /// Primary identifier / surname, as printed (uppercase, `<` runs
    /// collapsed to single spaces between components).
    pub surname: String,
    /// Secondary identifier / given names, as printed (same cleanup as
    /// [`surname`](Self::surname)).
    pub given_names: String,
    /// Nationality (3-letter ICAO code).
    pub nationality: String,
    /// ISO 8601 (`YYYY-MM-DD`), century inferred (see [`expand_date`]).
    /// Holds the raw, unexpanded `YYMMDD` field instead whenever
    /// [`date_of_birth_completeness`](Self::date_of_birth_completeness) is
    /// not [`DateCompleteness::Complete`] — see that field's doc comment.
    pub date_of_birth: String,
    /// Whether the holder's date of birth was fully known, partly filled with
    /// `<`, or entirely unknown (Doc 9303 Part 3 §4.8, `:547`). `date_of_birth`
    /// holds the raw field rather than an ISO date whenever this is not
    /// [`DateCompleteness::Complete`].
    ///
    /// An unknown or partially unknown date of birth is not a bad read: Part
    /// 3 §4.8 explicitly lets an issuer fill unknown positions with `<`, and
    /// a filler counts as zero for check-digit purposes (`:563`), so an
    /// all-filler date of birth with check digit `0` verifies. This field is
    /// how a caller distinguishes "legitimately unknown, per the issuer" from
    /// "garbage OCR" — `DateValidity::dates_well_formed` stays `false` either
    /// way, since an unknown date is genuinely not a parseable calendar date,
    /// so that struct alone cannot make the distinction.
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub date_of_birth_completeness: DateCompleteness,
    /// "M", "F" or "X" (unspecified).
    pub sex: String,
    /// ISO 8601 (`YYYY-MM-DD`).
    pub date_of_expiry: String,
    /// TD3: personal number field. TD1: optional data 1 + 2 joined.
    /// TD2: optional data field.
    pub personal_number: Option<String>,
    /// The raw MRZ lines, newline-joined, exactly as validated.
    pub mrz_lines: String,
    /// Per-field check-digit verification results — see [`Checks`].
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub checks: Checks,
}

impl MrzData {
    /// Shorthand for `checks.all_valid()`.
    pub fn valid(&self) -> bool {
        self.checks.all_valid()
    }

    /// The complete document number: the overflow reassembly when there is
    /// one, otherwise the 9-character field as printed.
    pub fn full_document_number(&self) -> &str {
        self.document_number_full
            .as_deref()
            .unwrap_or(&self.document_number)
    }

    /// Human-readable name of the issuing state, if the code is recognized.
    pub fn issuing_country_name(&self) -> Option<&'static str> {
        country_name(&self.issuing_country)
    }

    /// Human-readable name of the nationality, if the code is recognized.
    pub fn nationality_name(&self) -> Option<&'static str> {
        country_name(&self.nationality)
    }

    /// This record's [`SequenceCompleteness`] — always
    /// [`SequenceCompleteness::Complete`], computed from `self.checks`/
    /// `self.date_of_birth_completeness` rather than stored separately.
    /// [`SequenceCompleteness::Partial`] only ever comes from
    /// [`SequenceCompleteness::from_parse_result`] on the `Err` side of a
    /// [`find_and_parse`](crate::find_and_parse) call, since a `Partial`
    /// read has no `MrzData` to hang this method off of in the first place.
    pub fn sequence_completeness(&self) -> SequenceCompleteness {
        SequenceCompleteness::Complete {
            checks: self.checks.clone(),
            date_of_birth: self.date_of_birth_completeness,
        }
    }
}

/// Which check-digit-bearing field an error refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum Field {
    /// [`Checks::document_number`].
    DocumentNumber,
    /// [`Checks::date_of_birth`].
    DateOfBirth,
    /// [`Checks::date_of_expiry`].
    DateOfExpiry,
    /// [`Checks::personal_number`].
    PersonalNumber,
    /// [`Checks::composite`].
    Composite,
}

impl Field {
    /// Field name as it appears on [`Checks`].
    pub fn as_str(self) -> &'static str {
        match self {
            Self::DocumentNumber => "document_number",
            Self::DateOfBirth => "date_of_birth",
            Self::DateOfExpiry => "date_of_expiry",
            Self::PersonalNumber => "personal_number",
            Self::Composite => "composite",
        }
    }
}

impl core::fmt::Display for Field {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Checks {
    /// The fields whose check digits failed, in field order. Empty when
    /// [`all_valid`](Checks::all_valid) is `true`.
    pub fn failed(&self) -> Vec<Field> {
        [
            (self.document_number, Field::DocumentNumber),
            (self.date_of_birth, Field::DateOfBirth),
            (self.date_of_expiry, Field::DateOfExpiry),
            (self.personal_number, Field::PersonalNumber),
            (self.composite, Field::Composite),
        ]
        .into_iter()
        .filter_map(|(ok, f)| (!ok).then_some(f))
        .collect()
    }

    /// How many of the five check digits validate — the ranking signal the
    /// scanner uses to pick its best-effort reading.
    pub(crate) fn score(&self) -> u8 {
        5 - self.failed().len() as u8
    }
}

/// Why parsing an MRZ zone failed outright — distinct from a failed check
/// digit, which `parse_*` reports through [`Checks`] instead of an error.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MrzError {
    /// Line has the wrong length for the claimed format.
    BadLength {
        /// The length the claimed format requires.
        expected: usize,
        /// The length actually supplied.
        got: usize,
    },
    /// Character outside `[A-Z0-9<]`.
    BadCharacter(char),
    /// Document code not recognized for the format.
    BadDocumentCode(String),
    /// A check digit did not validate against its field.
    ///
    /// Note that the `parse_*` functions deliberately do *not* return this:
    /// they return an [`MrzData`] whose [`Checks`] report the failure, so a
    /// caller can show the user which digits disagreed. This variant exists
    /// for callers that convert a failed [`Checks`] into an error of their own.
    BadChecksum {
        /// Which check-digit-bearing field disagreed.
        field: Field,
        /// Byte offset of the check digit within the field's line.
        position: usize,
    },
    /// [`find_and_parse`]/[`find_and_parse_with`] only: a line matching one
    /// format's document-code prefix and charset was found, but no companion
    /// line ever combined with it into a full parse — distinct from
    /// [`NotFound`](Self::NotFound), which means nothing MRZ-shaped was seen
    /// at all. `lines_found` is always `1` today: only line 1's document-code
    /// prefix is a reliably shape-checkable signal on its own (a format's
    /// other lines carry no distinguishing prefix to detect in isolation).
    /// See `knowledge/MRZ_SEQUENCE_COMPLETENESS.md` chunk 4.
    IncompleteSequence {
        /// The format whose line 1 was recognized.
        format: Format,
        /// How many of `lines_expected` lines were actually located.
        lines_found: u8,
        /// How many lines `format` requires (2 for TD2/TD3/MRV-A/MRV-B, 3
        /// for TD1).
        lines_expected: u8,
    },
    /// No plausible MRZ found in the supplied text.
    NotFound,
}

impl core::fmt::Display for MrzError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::BadLength { expected, got } => {
                write!(f, "bad MRZ line length: expected {expected}, got {got}")
            }
            Self::BadCharacter(c) => write!(f, "invalid MRZ character: {c:?}"),
            Self::BadDocumentCode(c) => write!(f, "unrecognized document code: {c:?}"),
            Self::BadChecksum { field, position } => {
                write!(f, "check digit failed for {field} at position {position}")
            }
            Self::IncompleteSequence {
                format,
                lines_found,
                lines_expected,
            } => {
                write!(
                    f,
                    "incomplete {format:?} sequence: found {lines_found} of {lines_expected} lines"
                )
            }
            Self::NotFound => write!(f, "no MRZ found in text"),
        }
    }
}

impl std::error::Error for MrzError {}

#[cfg(test)]
mod tests {
    use super::*;

    // ICAO 9303 specimen identity (Utopia / Anna Maria Eriksson). Part 4's own
    // copy is Appendix B Figure B-1 — an image in the source PDF, not
    // extracted as text into this corpus
    // (`knowledge/docs9303/Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:669-687`
    // is the figure caption, no MRZ text). Corroborated instead by the
    // byte-identical TD2 zone Part 6 publishes as literal text (see below,
    // `knowledge/docs9303/Doc_9303_Part6_Specs_for_TD2_MROTDs.md:487-488`).
    const TD3_L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    const TD3_L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

    #[test]
    fn td3_specimen_fully_valid() {
        let d = parse_td3(TD3_L1, TD3_L2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.document_type, "P");
        assert_eq!(d.issuing_country, "UTO");
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.given_names, "ANNA MARIA");
        assert_eq!(d.document_number, "L898902C3");
        assert_eq!(d.nationality, "UTO");
        assert_eq!(d.date_of_birth, "1974-08-12");
        assert_eq!(d.date_of_birth_completeness, DateCompleteness::Complete);
        assert_eq!(d.sex, "F");
        assert_eq!(d.date_of_expiry, "2012-04-15");
        assert_eq!(d.personal_number.as_deref(), Some("ZE184226B"));
    }

    #[test]
    fn td3_tampered_dob_fails_checksum() {
        // Change one digit of the date of birth: 740812 → 750812.
        let tampered = TD3_L2.replacen("740812", "750812", 1);
        let d = parse_td3(TD3_L1, &tampered).unwrap();
        assert!(!d.checks.date_of_birth);
        assert!(!d.checks.composite);
        assert!(!d.valid());
    }

    #[test]
    fn td3_all_filler_dob_is_a_valid_field_not_a_bad_read() {
        // Doc 9303 Part 3 §4.8 (`:547`) lets an issuer complete an unknown
        // date of birth with filler characters, and `:563` gives a `<` the
        // value zero for check-digit purposes — so an all-filler DOB field
        // with check digit 0 is mathematically valid, not corrupt. Replace
        // TD3_L2's dob+check field (positions 14-20, 0-indexed 13..20) with
        // seven fillers followed by check digit '0'.
        assert_eq!(check_digit("<<<<<<").unwrap(), 0);
        let l2 = format!("{}{}{}", &TD3_L2[0..13], "<<<<<<0", &TD3_L2[20..]);
        assert_eq!(l2.len(), TD3_L2.len());
        let d = parse_td3(TD3_L1, &l2).unwrap();
        assert!(d.checks.date_of_birth);
        assert_eq!(d.date_of_birth, "<<<<<<"); // left unexpanded, not a lie
        assert_eq!(d.date_of_birth_completeness, DateCompleteness::Unknown);
    }

    // Mrz_Field_Layout.md §2.3: an unused TD3 personal number's check digit
    // may be issued as either '0' or '<' — both are zero-valued under the
    // ICAO 7-3-1 arithmetic (`char_value('<') == char_value('0') == 0`), so
    // the composite digit is identical either way. These two tests pin both
    // issuer-option forms independently through the real parser, not just
    // the `checksum::verify` primitive.

    #[test]
    fn td3_empty_personal_number_with_filler_check_digit_zero() {
        // Personal number all fillers, check digit '0' (value 0). Composite
        // recomputed independently of the crate (not copied from TD3_L2,
        // whose personal number differs): 8, not the '6' this fixture
        // originally carried unasserted — see the comment above.
        let l2 = "L898902C36UTO7408122F1204159<<<<<<<<<<<<<<08";
        let d = parse_td3(TD3_L1, l2).unwrap();
        assert!(d.checks.personal_number);
        assert!(d.checks.composite);
        assert_eq!(d.personal_number, None);
    }

    #[test]
    fn td3_empty_personal_number_with_filler_check_digit_filler() {
        // Same fixture, check digit '<' instead of '0' — same composite
        // digit, since both digit characters have ICAO value 0.
        let l2 = "L898902C36UTO7408122F1204159<<<<<<<<<<<<<<<8";
        let d = parse_td3(TD3_L1, l2).unwrap();
        assert!(d.checks.personal_number);
        assert!(d.checks.composite);
        assert_eq!(d.personal_number, None);
    }

    // Same Utopia/Eriksson identity as the TD3/TD2 specimens, reshaped into
    // TD1's 3-line layout. Part 5's own Appendix A/B examples are images in
    // the source PDF, not extracted as text
    // (`knowledge/docs9303/Doc_9303_Part5_Specs_for_TD1_MROTDs.md:504-534`);
    // this zone is corroborated by cross-part agreement with the literal TD2
    // specimen (same document number, dates and name), not a published Part 5
    // text specimen.
    const TD1_L1: &str = "I<UTOD231458907<<<<<<<<<<<<<<<";
    const TD1_L2: &str = "7408122F1204159UTO<<<<<<<<<<<6";
    const TD1_L3: &str = "ERIKSSON<<ANNA<MARIA<<<<<<<<<<";

    #[test]
    fn td1_specimen_fully_valid() {
        let d = parse_td1(TD1_L1, TD1_L2, TD1_L3).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::Td1);
        assert_eq!(d.document_type, "I");
        assert_eq!(d.document_number, "D23145890");
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.given_names, "ANNA MARIA");
        assert_eq!(d.date_of_birth, "1974-08-12");
        assert_eq!(d.date_of_birth_completeness, DateCompleteness::Complete);
        assert_eq!(d.date_of_expiry, "2012-04-15");
    }

    #[test]
    fn td1_all_filler_dob_reports_unknown() {
        // Same all-filler-DOB scenario as the TD3 test above, but for TD1's
        // layout, where the dob+check field is line2[0..7] rather than
        // line2[13..20] — covers a second of the five formats per the S5 plan.
        let l2 = format!("{}{}", "<<<<<<0", &TD1_L2[7..]);
        assert_eq!(l2.len(), TD1_L2.len());
        let d = parse_td1(TD1_L1, &l2, TD1_L3).unwrap();
        assert!(d.checks.date_of_birth);
        assert_eq!(d.date_of_birth, "<<<<<<");
        assert_eq!(d.date_of_birth_completeness, DateCompleteness::Unknown);
    }

    // Official ICAO 9303 part 6 TD2 specimen (Utopia / Anna Maria Eriksson),
    // published verbatim as text:
    // `knowledge/docs9303/Doc_9303_Part6_Specs_for_TD2_MROTDs.md:487-488`.
    const TD2_L1: &str = "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<";
    const TD2_L2: &str = "D231458907UTO7408122F1204159<<<<<<<6";

    #[test]
    fn td2_specimen_fully_valid() {
        let d = parse_td2(TD2_L1, TD2_L2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::Td2);
        assert_eq!(d.document_type, "I");
        assert_eq!(d.issuing_country, "UTO");
        assert_eq!(d.document_number, "D23145890");
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.given_names, "ANNA MARIA");
        assert_eq!(d.nationality, "UTO");
        assert_eq!(d.date_of_birth, "1974-08-12");
        assert_eq!(d.sex, "F");
        assert_eq!(d.date_of_expiry, "2012-04-15");
    }

    #[test]
    fn td2_tampered_expiry_fails_checksum() {
        let tampered = TD2_L2.replacen("120415", "120416", 1);
        let d = parse_td2(TD2_L1, &tampered).unwrap();
        assert!(!d.checks.date_of_expiry);
        assert!(!d.checks.composite);
        assert!(!d.valid());
    }

    #[test]
    fn td2_found_in_ocr_text() {
        let text = format!("## IDENTITY CARD\n\nnoise\n\n{TD2_L1}\n{TD2_L2}\n\nfooter");
        let d = find_and_parse(&text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::Td2);
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.document_number, "D23145890");
    }

    #[test]
    fn find_in_ocr_noise() {
        let text = format!(
            "## REPUBLIC OF UTOPIA\n\nSome OCR noise here\n\n{}\n{}\n\nfooter",
            // OCR quirks: lowercase, stray spaces, « for <<, dropped fillers.
            "p<utoeriksson«anna<maria<<<<<<<<<<<<<<<<<",
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10"
        );
        let d = find_and_parse(&text).unwrap();
        assert!(d.valid());
        assert_eq!(d.surname, "ERIKSSON");
    }

    #[test]
    fn find_html_escaped_and_merged_lines() {
        // Real docling output shape: fillers escaped as &lt; and both TD3
        // lines on one physical markdown line (Croatian specimen).
        let text = "## PUTOVNICA\n\nP&lt;HRVSPECIMEN&lt;&lt;SPECIMEN&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt; 0070070071HRV8212258F1407019&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;06\n";
        let d = find_and_parse(text).unwrap();
        assert_eq!(d.surname, "SPECIMEN");
        assert_eq!(d.document_number, "007007007");
        assert_eq!(d.issuing_country, "HRV");
        assert!(d.checks.document_number);
        assert!(d.checks.date_of_birth);
        assert!(d.checks.date_of_expiry);
    }

    #[test]
    fn checksum_verified_ocr_repair() {
        // Verbatim tesseract.js output for the Croatian specimen at low
        // resolution: trailing fillers read as K/L runs, a hallucinated
        // leading '1' on line 2 (45 chars), and 'B' where '8' is printed.
        // The check digits prove which repaired variant is the true read.
        let text = "I 01072009 PUJZAGREB 0\n\nBIDFD WH5SS A 2\n\n01072014\nP<HRVSPECIMEN<<SPECIMEN<KLLLLLLLLLLLLLLLLLKLKL\n10070070071HRVB212258F1407019<<<<<<<<<<<<<<06\n";
        let d = find_and_parse(text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.surname, "SPECIMEN");
        assert_eq!(d.given_names, "SPECIMEN");
        assert_eq!(d.document_number, "007007007");
        assert_eq!(d.date_of_birth, "1982-12-25");
    }

    #[test]
    fn ocr_repair_dropped_filler_mid_line() {
        // Second verbatim tesseract.js reading of the same specimen: an
        // L-run inside the personal-number field and one filler DROPPED
        // (43 chars) — the missing character must be re-inserted inside the
        // filler run, not appended, or the check digits shift.
        let text = "RF 01072009 PUZAGREB\n01072014\nP<HRVSPECIMEN<<SPECIMEN<<K<KLLLLLLLLLLLLLLLLKLKL\n0070070071HRVB212258F1407019<<<<LLLLLLL<<06\n";
        let d = find_and_parse(text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.surname, "SPECIMEN");
        assert_eq!(d.document_number, "007007007");
        assert_eq!(d.personal_number, None);
    }

    #[test]
    fn td1_from_single_docling_line_with_k_misreads() {
        // Verbatim docling OCR of the Slovenian 2022 specimen ID card rear:
        // all three TD1 lines in ONE paragraph, `<` escaped as &lt;, and the
        // usual K-for-filler misreads (IK→I<, 145K<→145<<, VZORECKK→VZOREC<<).
        let text = "1F9874543\n\nIKSVNIE987654302806985505145K&lt; 8506287F3203282SVN&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;2 VZORECKKJANAKKKKKKKKK&lt;&lt;KK";
        let d = find_and_parse(text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::Td1);
        assert_eq!(d.document_type, "I");
        assert_eq!(d.issuing_country, "SVN");
        assert_eq!(d.document_number, "IE9876543");
        assert_eq!(d.surname, "VZOREC");
        assert_eq!(d.given_names, "JANA");
        assert_eq!(d.date_of_birth, "1985-06-28");
        assert_eq!(d.date_of_expiry, "2032-03-28");
        // The trailing K in the EMŠO field is a filler misread that check
        // digits cannot catch (K ≡ < mod 10) — heuristic cleanup handles it.
        assert_eq!(d.personal_number.as_deref(), Some("2806985505145"));
    }

    #[test]
    fn ocr_repair_deeply_truncated_name_line() {
        // Verbatim ocrs output for the Croatian specimen at 600×421: line 2 is
        // read perfectly, but line 1 loses NINE trailing fillers (35/44 chars)
        // and its `<` document-code filler is misread as `K`. The name line
        // carries no check digit of its own, so padding the filler run back is
        // safe — line 2's check digits still prove the read.
        let text = "PUTOVNICA\nPKHRVSPECIMEN<<SPECIMEN<<<<<<<<<<<<\n0070070071HRV8212258F1407019<<<<<<<<<<<<<<06\n";
        let d = find_and_parse(text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.surname, "SPECIMEN");
        assert_eq!(d.given_names, "SPECIMEN");
        assert_eq!(d.document_number, "007007007");
    }

    #[test]
    fn invalid_checksums_still_reported() {
        // A tampered MRZ parses but is flagged invalid rather than dropped.
        let tampered = TD3_L2.replacen("740812", "750812", 1);
        let text = format!("{TD3_L1}\n{tampered}");
        let d = find_and_parse(&text).unwrap();
        assert!(!d.valid());
        assert!(!d.checks.date_of_birth);
    }

    #[test]
    fn find_nothing_in_plain_text() {
        assert_eq!(
            find_and_parse("just a regular paragraph\nwith two lines"),
            Err(MrzError::NotFound)
        );
    }

    // ICAO 9303 part 7's own published MRV-A specimen, transcribed verbatim
    // from
    // `knowledge/docs9303/Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:1104-1106`.
    // Unlike the hand-derived pair below, every character here comes from the
    // standard, so this is the MRV equivalent of the part 4/5/6 specimens.
    const MRV_A_ICAO_L1: &str = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    const MRV_A_ICAO_L2: &str = "L898902C<3UTO6908061F9406236ZE184226B<<<<<<<";

    // ICAO 9303 part 7's own published MRV-B specimen, from the same appendix:
    // `knowledge/docs9303/Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:1136-1138`.
    const MRV_B_ICAO_L1: &str = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<";
    const MRV_B_ICAO_L2: &str = "L898902C<3UTO6908061F9406236ZE184226";

    #[test]
    fn mrv_a_icao_published_specimen_fully_valid() {
        let d = parse_mrv_a(MRV_A_ICAO_L1, MRV_A_ICAO_L2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvA);
        assert_eq!(d.document_type, "V");
        assert_eq!(d.issuing_country, "UTO");
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.given_names, "ANNA MARIA");
        assert_eq!(d.nationality, "UTO");
        assert_eq!(d.sex, "F");
        // The document-number field is `L898902C<`: eight characters padded to
        // nine with a filler, check digit 3. The filler is padding, not the
        // long-number signal — that would need the *check digit* position to
        // be a filler — so no overflow is read here.
        assert_eq!(d.document_number, "L898902C");
        assert_eq!(d.document_number_full, None);
        assert_eq!(d.personal_number.as_deref(), Some("ZE184226B"));
        assert_eq!(d.date_of_birth, "1969-08-06");
        // Doc 9303 defines no century rule (part 3 §4.8 is silent), so this
        // crate's own policy applies: expiry is always read as 20xx. ICAO's
        // specimen is a 1990s document, so that policy renders 940623 as 2094
        // rather than 1994. Pinned deliberately — it is the documented
        // heuristic behaving as designed, and the clearest illustration of its
        // limit. See `dates::expand_date`.
        assert_eq!(d.date_of_expiry, "2094-06-23");
    }

    #[test]
    fn mrv_b_icao_published_specimen_fully_valid() {
        let d = parse_mrv_b(MRV_B_ICAO_L1, MRV_B_ICAO_L2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvB);
        assert_eq!(d.document_type, "V");
        assert_eq!(d.issuing_country, "UTO");
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.given_names, "ANNA MARIA");
        assert_eq!(d.nationality, "UTO");
        assert_eq!(d.document_number, "L898902C");
        assert_eq!(d.document_number_full, None);
        assert_eq!(d.personal_number.as_deref(), Some("ZE184226"));
        assert_eq!(d.date_of_birth, "1969-08-06");
        assert_eq!(d.date_of_expiry, "2094-06-23");
    }

    // Hand-derived by this crate — NOT an ICAO-published specimen. Kept
    // alongside the published pair above because it exercises a different
    // nationality, dates and optional-data shape, and because the tamper
    // tests below are wired to its values. Part 7's real MRV-A worked example
    // is the one transcribed above:
    // `knowledge/docs9303/Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:1104-1106`.
    // Line 2 (44 chars); check-digit arithmetic (7-3-1, values A-Z=10-35,
    // <=0), worked independently so the constant is verified rather than
    // merely asserted:
    //   doc#   XK9305487: X=33,K=20,9,3,0,5,4,8,7 * 7,3,1,7,3,1,7,3,1
    //          = 231+60+9+21+0+5+28+24+7 = 385 -> 385 mod 10 = 5
    //   DOB    850221: 8,5,0,2,2,1 * 7,3,1,7,3,1
    //          = 56+15+0+14+6+1 = 92 -> 92 mod 10 = 2
    //   expiry 270314: 2,7,0,3,1,4 * 7,3,1,7,3,1
    //          = 14+21+0+21+3+4 = 63 -> 63 mod 10 = 3
    const MRV_A_L1: &str = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    const MRV_A_L2: &str = "XK93054875BRA8502212F2703143R5T6U7V8W9<<<<<<";

    #[test]
    fn mrv_a_specimen_fully_valid() {
        let d = parse_mrv_a(MRV_A_L1, MRV_A_L2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvA);
        assert_eq!(d.document_type, "V");
        assert_eq!(d.issuing_country, "UTO");
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.given_names, "ANNA MARIA");
        assert_eq!(d.nationality, "BRA");
        assert_eq!(d.date_of_birth, "1985-02-21");
        assert_eq!(d.sex, "F");
        assert_eq!(d.date_of_expiry, "2027-03-14");
        assert_eq!(d.document_number, "XK9305487");
        assert_eq!(d.personal_number.as_deref(), Some("R5T6U7V8W9"));
    }

    // Hand-derived by this crate — NOT an ICAO-published specimen. Kept for
    // the same reason as the MRV-A pair above; Part 7's real MRV-B worked
    // example is transcribed as `MRV_B_ICAO_L1`/`MRV_B_ICAO_L2`:
    // `knowledge/docs9303/Doc_9303_Part7_Machine_Readable_Visas_MRVs.md:1136-1138`.
    // Line 2 (36 chars); check-digit arithmetic (7-3-1, values A-Z=10-35,
    // <=0), worked independently so the constant is verified rather than
    // merely asserted:
    //   doc#   L23456789: L=21,2,3,4,5,6,7,8,9 * 7,3,1,7,3,1,7,3,1
    //          = 147+6+3+28+15+6+49+24+9 = 287 -> 287 mod 10 = 7
    //   DOB    920101: 9,2,0,1,0,1 * 7,3,1,7,3,1
    //          = 63+6+0+7+0+1 = 77 -> 77 mod 10 = 7
    //   expiry 270630: 2,7,0,6,3,0 * 7,3,1,7,3,1
    //          = 14+21+0+42+9+0 = 86 -> 86 mod 10 = 6
    const MRV_B_L1: &str = "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<";
    const MRV_B_L2: &str = "L234567897DEU9201017F2706306QW12ER34";

    #[test]
    fn mrv_b_specimen_fully_valid() {
        let d = parse_mrv_b(MRV_B_L1, MRV_B_L2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvB);
        assert_eq!(d.nationality, "DEU");
        assert_eq!(d.date_of_birth, "1992-01-01");
        assert_eq!(d.date_of_expiry, "2027-06-30");
        assert_eq!(d.document_number, "L23456789");
        assert_eq!(d.personal_number.as_deref(), Some("QW12ER34"));
    }

    #[test]
    fn mrv_a_tampered_dob_fails_checksum() {
        let tampered = MRV_A_L2.replacen("850221", "860221", 1);
        let d = parse_mrv_a(MRV_A_L1, &tampered).unwrap();
        assert!(!d.checks.date_of_birth);
        assert!(!d.valid());
    }

    #[test]
    fn mrv_rejects_non_v_document_code() {
        let line1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
        let result = parse_mrv_a(line1, MRV_A_L2);
        assert!(matches!(result, Err(MrzError::BadDocumentCode(_))));
    }

    #[test]
    fn mrv_a_found_in_ocr_text() {
        let text = format!("## VISA\n\nnoise\n\n{MRV_A_L1}\n{MRV_A_L2}\n\nfooter");
        let d = find_and_parse(&text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvA);
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.nationality, "BRA");
    }

    #[test]
    fn mrv_b_found_in_ocr_text() {
        let text = format!("## VISA\n\nnoise\n\n{MRV_B_L1}\n{MRV_B_L2}\n\nfooter");
        let d = find_and_parse(&text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvB);
        assert_eq!(d.nationality, "DEU");
    }

    #[test]
    fn mrv_found_html_escaped_or_merged() {
        // HTML-escaped fillers.
        let escaped_l1 = MRV_A_L1.replace('<', "&lt;");
        let escaped_l2 = MRV_A_L2.replace('<', "&lt;");
        let text = format!("## VISA\n\n{escaped_l1}\n{escaped_l2}\n");
        let d = find_and_parse(&text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvA);

        // Both lines merged onto one physical line with no separator at all
        // (the ~88-char merged-line case, like docling's single-paragraph
        // OCR output for a TD3 specimen).
        let merged = format!("## VISA\n\n{MRV_A_L1}{MRV_A_L2}\n");
        let d = find_and_parse(&merged).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvA);
    }

    // ---- Document-number overflow (Part 5/6 note j; TD3 by analogy — see
    // parser::read_overflow's doc comment) ----

    #[test]
    fn td3_long_document_number_round_trips() {
        let fields = Td3Fields {
            document_code: "P".into(),
            issuing_country: "UTO".into(),
            document_number: "L898902C31234".into(), // 13 chars, overflows 9
            surname: "ERIKSSON".into(),
            given_names: "ANNA MARIA".into(),
            nationality: "UTO".into(),
            date_of_birth: "740812".into(),
            sex: "F".into(),
            date_of_expiry: "120415".into(),
            personal_number: None,
        };
        let mrz = format_td3(&fields);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        // Nine principal characters, then a filler where the check digit goes.
        assert_eq!(&l2[0..10], "L898902C3<");
        let d = parse_td3(l1, l2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.document_number_full.as_deref(), Some("L898902C31234"));
        assert_eq!(d.full_document_number(), "L898902C31234");
        assert!(!d.document_number_legacy_encoding);
        // The 9-char field reading stays available and unsurprising.
        assert_eq!(d.document_number, "L898902C3");
        assert_eq!(d.personal_number, None);
    }

    #[test]
    fn overflow_coexists_with_personal_number() {
        let fields = Td3Fields {
            document_number: "AB1234567890".into(), // 12 chars
            personal_number: Some("ZE184".into()),
            ..Td3Fields::default()
        };
        let d = parse_td3_str(&format_td3(&fields));
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.document_number_full.as_deref(), Some("AB1234567890"));
        assert_eq!(d.personal_number.as_deref(), Some("ZE184"));
    }

    #[test]
    fn td2_and_td1_long_document_numbers_round_trip() {
        let td2 = Td2Fields {
            document_number: "D23145890XY".into(), // 11 chars; remainder fits 7
            date_of_birth: "740812".into(),
            date_of_expiry: "120415".into(),
            ..Td2Fields::default()
        };
        let mrz = format_td2(&td2);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        let d = parse_td2(l1, l2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.document_number_full.as_deref(), Some("D23145890XY"));

        let td1 = Td1Fields {
            document_number: "D23145890ABCDE".into(), // 14 chars; remainder fits 15
            date_of_birth: "740812".into(),
            date_of_expiry: "120415".into(),
            ..Td1Fields::default()
        };
        let mrz = format_td1(&td1);
        let mut lines = mrz.lines();
        let d = parse_td1(
            lines.next().unwrap(),
            lines.next().unwrap(),
            lines.next().unwrap(),
        )
        .unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.document_number_full.as_deref(), Some("D23145890ABCDE"));
    }

    #[test]
    fn overflow_remainder_too_long_falls_back_to_truncation() {
        // TD2's optional field is 7 wide, so a remainder of 6 + check + filler
        // does not fit — the number is truncated to 9 as it always was, and the
        // ordinary (non-overflow) encoding still validates.
        let td2 = Td2Fields {
            document_number: "D23145890ABCDEF".into(), // remainder 7 → needs 9
            date_of_birth: "740812".into(),
            date_of_expiry: "120415".into(),
            ..Td2Fields::default()
        };
        let mrz = format_td2(&td2);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        let d = parse_td2(l1, l2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.document_number_full, None);
        assert_eq!(d.document_number, "D23145890");
    }

    #[test]
    fn tampered_overflow_remainder_fails_the_check_digit() {
        let fields = Td3Fields {
            document_number: "L898902C31234".into(),
            ..Td3Fields::default()
        };
        let mrz = format_td3(&fields);
        // The nine principal characters ("L898902C3") are printed in the
        // number field; only the 4-character remainder ("1234") is written
        // into the personal-number field, so corrupting the remainder means
        // corrupting "1234", not the pre-0.6 8-character-boundary "31234".
        let tampered = mrz.replacen("1234", "1235", 1);
        let d = parse_td3_str(&tampered);
        assert!(!d.checks.document_number);
        assert!(!d.valid());
        assert!(d.checks.failed().contains(&Field::DocumentNumber));
    }

    #[test]
    fn ordinary_specimens_report_no_overflow() {
        assert_eq!(
            parse_td3(TD3_L1, TD3_L2).unwrap().document_number_full,
            None
        );
        assert_eq!(
            parse_td1(TD1_L1, TD1_L2, TD1_L3)
                .unwrap()
                .document_number_full,
            None
        );
        assert_eq!(
            parse_td2(TD2_L1, TD2_L2).unwrap().document_number_full,
            None
        );
        // Empty document-number field is a blank field, not an overflow.
        let blank = "<<<<<<<<<<UTO7408122F1204159<<<<<<<<<<<<<<02";
        assert_eq!(parse_td3(TD3_L1, blank).unwrap().document_number_full, None);
    }

    fn parse_td3_str(mrz: &str) -> MrzData {
        let (l1, l2) = mrz.split_once('\n').unwrap();
        parse_td3(l1, l2).unwrap()
    }

    // ---- ParseOptions ----

    #[test]
    fn pivot_is_configurable_per_call() {
        // Birth dates land in the past relative to the pivot, expiry ahead.
        let d = parse_td3(TD3_L1, TD3_L2).unwrap();
        assert_eq!(d.date_of_birth, "1974-08-12");

        // With a pivot of 80, YY=74 reads as 2074 rather than 1974.
        let opts = ParseOptions { pivot_yy: 80 };
        let d = parse_td3_with(TD3_L1, TD3_L2, &opts).unwrap();
        assert_eq!(d.date_of_birth, "2074-08-12");
        // Check digits are untouched by the pivot — it only affects display.
        assert!(d.valid());
    }

    #[test]
    fn default_options_match_the_plain_entry_points() {
        let opts = ParseOptions::default();
        assert_eq!(opts.pivot_yy, CURRENT_YY);
        assert_eq!(
            parse_td3(TD3_L1, TD3_L2).unwrap(),
            parse_td3_with(TD3_L1, TD3_L2, &opts).unwrap()
        );
        let text = format!("## VISA\n\n{MRV_A_L1}\n{MRV_A_L2}\n");
        assert_eq!(
            find_and_parse(&text).unwrap(),
            find_and_parse_with(&text, &opts).unwrap()
        );
    }

    // ---- Checks diagnostics ----

    #[test]
    fn failed_lists_the_failing_fields() {
        let clean = parse_td3(TD3_L1, TD3_L2).unwrap();
        assert!(clean.checks.failed().is_empty());

        let tampered = TD3_L2.replacen("740812", "750812", 1);
        let d = parse_td3(TD3_L1, &tampered).unwrap();
        assert_eq!(
            d.checks.failed(),
            vec![Field::DateOfBirth, Field::Composite]
        );
        assert_eq!(Field::DateOfBirth.to_string(), "date_of_birth");
    }

    #[test]
    fn scanner_keeps_the_best_scoring_partial_read() {
        // Two check digits wrong: the returned record must be the real zone
        // with exactly those two flagged, not some worse-scoring variant.
        let tampered = TD3_L2
            .replacen("740812", "750812", 1)
            .replacen("120415", "120416", 1);
        let text = format!("noise\n\n{TD3_L1}\n{tampered}\n\nfooter");
        let d = find_and_parse(&text).unwrap();
        assert!(!d.valid());
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.document_number, "L898902C3");
        assert!(d.checks.document_number);
        assert!(!d.checks.date_of_birth);
        assert!(!d.checks.date_of_expiry);
    }

    #[test]
    fn country_names_surface_on_mrzdata() {
        let d = parse_td1(TD1_L1, TD1_L2, TD1_L3).unwrap();
        assert_eq!(d.issuing_country_name(), Some("Utopia (ICAO specimen)"));
        assert_eq!(d.nationality_name(), Some("Utopia (ICAO specimen)"));
    }
}
