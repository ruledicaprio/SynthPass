//! ICAO 9303 Machine Readable Zone parser, emitter, and check-digit validator.
//!
//! Zero runtime dependencies, no `unsafe`, and no clock or network access, so it
//! compiles to native and `wasm32-unknown-unknown` targets alike and gives the
//! same answer on both. Supports:
//! - **TD3** (passports): 2 lines × 44 characters
//! - **TD2** (official travel documents / ID cards): 2 lines × 36 characters
//! - **TD1** (ID cards): 3 lines × 30 characters
//! - **MRV-A** (visas, passport-book size): 2 lines × 44 characters
//! - **MRV-B** (visas, smaller size): 2 lines × 36 characters
//!
//! Check digits use the standard 7-3-1 weighting over the value mapping
//! `0-9 → 0-9`, `A-Z → 10-35`, `< → 0`. A field checksum that validates is
//! deterministic evidence — arithmetic, not a probabilistic model. What it
//! establishes is that the candidate is *checksum-consistent* with the printed
//! check digit. It is a strong filter and a weak oracle: it does not establish
//! byte-identity with the printed zone, and it constrains only the fields a
//! digit actually covers — on TD2 and TD3 the whole of line 1, names included,
//! carries no check digit at all. [`Blindspot`] is the exact account.
//!
//! ```
//! let doc = mrz::parse_td3(
//!     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
//!     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
//! ).unwrap();
//!
//! assert_eq!(doc.surname, "ERIKSSON");
//! assert_eq!(doc.date_of_birth.to_string(), "1974-08-12"); // expanded to ISO 8601
//!
//! // Per-field evidence, not a single boolean.
//! assert_eq!(doc.checks.document_number, Some(true));
//! assert_eq!(doc.checks.composite, Some(true));
//! assert!(doc.valid()); // every printed check digit verified
//! ```
//!
//! `parse_td1`, `parse_td2`, `parse_mrv_a` and `parse_mrv_b` cover the other
//! formats.
//!
//! # Where to start
//!
//! | You have | Reach for |
//! | --- | --- |
//! | MRZ lines, already separated | [`parse_td3`], [`parse_td2`], [`parse_td1`], [`parse_mrv_a`], [`parse_mrv_b`] |
//! | Free-form OCR text | [`find_and_parse`] — finds the zone, repairs it only under check-digit agreement |
//! | Fields to print as an MRZ | [`format_td3`] and its siblings, fed by [`Td3Fields`] and friends |
//! | A name in a national script | [`transliterate`] (Doc 9303 Part 3 §6 A, Latin), [`transliterate_cyrillic`] (§6 B, Cyrillic), [`encode_name_component`] |
//! | A number too long for its field | [`MrzData::full_document_number`] |
//! | A parsed record to judge | [`MrzData::valid`] for the *read*, [`MrzData::validity`] for the *document's dates* |
//! | A glyph the OCR could not read | [`solve_field`], [`solve_substitution`], and [`Blindspot`] for what no check digit can catch |
//!
//! A valid composite check digit establishes checksum consistency, not
//! byte-identity. It does not prove the document is in date — see
//! [`MrzData::validity`] — and its blind set is exactly characterised, both
//! for single substitutions and for the combinations of individually-caught
//! ones that cancel mod 10 — see [`Blindspot`].
//!
//! # Feature flags
//!
//! Both are off by default, which keeps the default build zero-dependency:
//!
//! - **`serde`** — derives `Serialize` and `Deserialize` on the data types:
//!   [`MrzData`], [`Checks`], [`Format`], [`Field`], [`SequenceCompleteness`],
//!   [`ParseOptions`], [`Date`], [`DateValidity`], [`DateCompleteness`], and the
//!   five emitter inputs ([`Td3Fields`], [`Td2Fields`], [`Td1Fields`],
//!   [`MrvAFields`], [`MrvBFields`]). [`MrzDate`] and [`Sex`] implement both by
//!   hand as their text form (ADR-0020), so `MrzData`'s dates and sex serialise
//!   as strings.
//! - **`zeroize`** — derives `ZeroizeOnDrop` on [`MrzData`], wiping its
//!   PII-bearing fields from memory when the value is dropped. Best-effort: see
//!   [`MrzData`] for what it does not reach.
//!
//! # Stability
//!
//! The crate is pre-1.0, so the **minor** version is the breaking slot: `^0.8`
//! resolves any `0.8.x` but never `0.9.0`. Output types ([`MrzData`],
//! [`Checks`], [`Format`], [`Field`], [`MrzError`], [`SequenceCompleteness`] and
//! the other enums) are `#[non_exhaustive]`, so they can grow in a patch
//! release; the five `*Fields` emitter inputs are deliberately exhaustive, so
//! that struct-update syntax (`..Default::default()`) keeps working. The minimum
//! supported Rust version is 1.82, and raising it is a minor-version change.
//!
//! Release history is in the
//! [changelog](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/CHANGELOG.md),
//! and what each version slot is for, up to 1.0, is in the
//! [roadmap](https://github.com/ruledicaprio/SynthPass/blob/main/crates/mrz/ROADMAP.md).
//!
//! # Source layout
//!
//! The engine is split across private modules, whose public items are all
//! re-exported at the crate root:
//!
//! - `parser` — the five fixed-layout parsers and the free-text scanner
//! - `emit` — the five emitters and Part 3 §4.6 name encoding
//! - `checksum` — check-digit math and OCR line normalization
//! - `repair` — check-digit-guided recovery of damaged or misread fields
//! - `blindspot` — the substitutions check digits provably cannot catch
//! - `dates` — `YYMMDD` expansion, the calendar, and date plausibility
//! - `countries` — a subset of Part 3 §5 state and organization codes
//! - `doccode` — Part 4 §4.4 secondary passport document codes
//! - `translit` — Part 3 §6 A (Latin) and §6 B (Cyrillic) transliteration

// docs.rs builds with `--cfg docsrs` on nightly; `doc_cfg` then badges every
// feature-gated item. The attribute is inert on stable and on the 1.82 MSRV (#428).
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]
#![forbid(unsafe_code)]

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
mod doccode;
mod emit;
mod mrz_date;
mod parser;
mod repair;
mod sex;
#[allow(dead_code)] // Phase 0 check program; parser wiring is a later, measured change.
mod strip;
mod translit;

pub use blindspot::{blindspot, class_of, collisions, Blindspot, CLASSES};
pub use checksum::{check_digit, verify};
pub use countries::{code_for_name, codes, codes_equivalent, country_name};
pub use dates::{
    date_completeness, expand_date, expand_date_with_pivot, is_leap_year, Date, DateCompleteness,
    DateValidity, CURRENT_YY,
};
pub use doccode::{passport_type, PassportType};
pub use emit::{
    encode_name_component, format_mrv_a, format_mrv_b, format_td1, format_td2, format_td3,
    MrvAFields, MrvBFields, Td1Fields, Td2Fields, Td3Fields,
};
pub use mrz_date::{DateRole, InvalidRawDateField, MrzDate, ParseMrzDateError, RawDateField};
pub use parser::{
    find_and_parse, find_and_parse_with, parse_mrv_a, parse_mrv_a_with, parse_mrv_b,
    parse_mrv_b_with, parse_td1, parse_td1_with, parse_td2, parse_td2_with, parse_td3,
    parse_td3_with,
};
pub use repair::{
    solve_class_sweep, solve_field, solve_substitution, substitution_candidates, width_candidates,
    FieldKind, Resolution, CONFUSABLES, MRZ_ALPHABET, UNKNOWN,
};
pub use sex::{ParseSexError, Sex};
pub use translit::{
    transliterate, transliterate_char, transliterate_cyrillic, transliterate_cyrillic_char,
    transliterations, CyrillicLanguage, TransliterationStyle,
};

/// Tunables for the parsing entry points.
///
/// Every `parse_*` / [`find_and_parse`] function is the `ParseOptions::default()`
/// case of its `*_with` counterpart, so existing calls are unaffected.
///
/// ```
/// use mrz::{find_and_parse_with, ParseOptions, CURRENT_YY};
///
/// assert_eq!(ParseOptions::default().pivot_yy, CURRENT_YY);
///
/// // Pin the pivot so that replaying archived reads gives the same dates no
/// // matter which crate version (and so which `CURRENT_YY`) runs the replay.
/// let opts = ParseOptions::default().with_pivot_yy(30);
/// let text = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
///             L898902C36UTO7408122F1204159ZE184226B<<<<<10";
/// let doc = find_and_parse_with(text, &opts).unwrap();
/// assert_eq!(doc.date_of_birth.to_string(), "1974-08-12"); // 74 > 30, so last century
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct ParseOptions {
    /// Two-digit century pivot for [`expand_date_with_pivot`]. Defaults to
    /// [`CURRENT_YY`]; set it explicitly to pin behaviour instead of inheriting
    /// the constant this crate was compiled with.
    pub pivot_yy: u32,
    /// Try [`solve_class_sweep`] during the damaged-capture pass: one OCR
    /// confusable class applied uniformly across a field and its own check
    /// digit, for the run-of-one-character-read-as-its-lookalike case.
    ///
    /// **Off by default, and unmeasured.** It repairs a shape observed on a
    /// real document, but how many documents it *breaks* is exactly what has
    /// not been established — so this exists to be A/B'd against a control,
    /// not to be switched on. Only the damaged pass consults it, which runs
    /// only after an ordinary read has already failed to validate.
    #[cfg_attr(feature = "serde", serde(default))]
    pub class_sweep: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            pivot_yy: CURRENT_YY,
            class_sweep: false,
        }
    }
}

impl ParseOptions {
    /// Pin the two-digit-year century pivot, starting from [`Default`].
    ///
    /// This is how callers outside the crate build a `ParseOptions`: the
    /// struct is `#[non_exhaustive]`, so a struct expression cannot name its
    /// fields from another crate — **and functional update syntax does not
    /// lift that restriction**, which is the part worth knowing. `ParseOptions
    /// { pivot_yy: 30, ..Default::default() }` is rejected with `E0639` just
    /// as the bare literal is. Every option this struct grows gets a `with_*`
    /// method beside this one, and each is a non-breaking addition.
    ///
    /// ```
    /// use mrz::{ParseOptions, CURRENT_YY};
    ///
    /// assert_eq!(ParseOptions::default().pivot_yy, CURRENT_YY);
    /// assert_eq!(ParseOptions::default().with_pivot_yy(30).pivot_yy, 30);
    /// ```
    #[must_use]
    pub const fn with_pivot_yy(mut self, pivot_yy: u32) -> Self {
        self.pivot_yy = pivot_yy;
        self
    }

    /// Enable the uniform confusable-class sweep in the damaged pass — see
    /// [`ParseOptions::class_sweep`], which is off by default and unmeasured.
    ///
    /// ```
    /// use mrz::ParseOptions;
    ///
    /// assert!(!ParseOptions::default().class_sweep);
    /// assert!(ParseOptions::default().with_class_sweep(true).class_sweep);
    /// ```
    #[must_use]
    pub const fn with_class_sweep(mut self, class_sweep: bool) -> Self {
        self.class_sweep = class_sweep;
        self
    }
}

/// Per-field check-digit verification results.
///
/// `#[non_exhaustive]`: a future MRZ format may carry a check digit these five
/// fields don't name, and adding it should not be a breaking change. Construct
/// one from a `parse_*` function rather than by literal.
///
/// `Some(true)` means the printed check digit verified, `Some(false)` means it
/// failed, and `None` means the format does not print that check digit.
///
/// ```
/// // The ICAO specimen with its date of birth altered: 740812 → 750812.
/// let doc = mrz::parse_td3(
///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
///     "L898902C36UTO7508122F1204159ZE184226B<<<<<10",
/// )
/// .unwrap();
///
/// // The damage is located, not merely detected.
/// assert_eq!(doc.checks.document_number, Some(true));
/// assert_eq!(doc.checks.date_of_birth, Some(false));
/// assert_eq!(doc.checks.date_of_expiry, Some(true));
/// assert_eq!(doc.checks.composite, Some(false)); // the composite covers the date of birth too
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Checks {
    /// The primary document-number field (or, for a number too long for it,
    /// the reassembled number — see [`MrzData::full_document_number`]).
    pub document_number: Option<bool>,
    /// The date-of-birth field.
    pub date_of_birth: Option<bool>,
    /// The date-of-expiry field.
    pub date_of_expiry: Option<bool>,
    /// TD3's personal-number field; absent on the other four formats.
    pub personal_number: Option<bool>,
    /// The composite check digit over the zone; absent on MRV-A and MRV-B.
    pub composite: Option<bool>,
}

impl Checks {
    /// Every check digit this format prints agrees with the candidate.
    ///
    /// This is checksum *consistency*, and it is deterministic. It is not
    /// byte-identity with the printed zone: [`crate::Blindspot`] gives the
    /// substitutions the arithmetic cannot see, and a field no digit covers
    /// — on TD2 and TD3 that is all of line 1 — is not constrained at all.
    ///
    /// ```
    /// let doc = mrz::parse_td2(
    ///     "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
    ///     "D231458907UTO7408122F1204159<<<<<<<6",
    /// )
    /// .unwrap();
    /// assert!(doc.checks.all_valid());
    /// // TD2 prints no personal-number check digit, so it is explicitly absent.
    /// assert_eq!(doc.checks.personal_number, None);
    /// ```
    pub fn all_valid(&self) -> bool {
        self.applicable() > 0 && self.verified() == self.applicable()
    }

    /// Stamps the parser's five raw verification results with the check-digit
    /// contract for `format`. A parser supplies a value for every slot; the
    /// format matrix is the single source of truth for whether that value was
    /// actually printed and therefore observable.
    pub(crate) fn from_verifications(format: Format, verified: [bool; 5]) -> Self {
        let applicable = format.check_digit_applicability();
        Self {
            document_number: applicable[0].then_some(verified[0]),
            date_of_birth: applicable[1].then_some(verified[1]),
            date_of_expiry: applicable[2].then_some(verified[2]),
            personal_number: applicable[3].then_some(verified[3]),
            composite: applicable[4].then_some(verified[4]),
        }
    }
}

/// A single completeness signal spanning both outcomes of a
/// [`find_and_parse`] attempt.
///
/// A successful read reports how complete it is through [`MrzData`]'s
/// `checks` and the variant of its `date_of_birth`; a scan that found only part of
/// a zone reports it through [`MrzError::IncompleteSequence`]. Those are two
/// differently shaped vocabularies, and this type lets a caller hold *one*
/// value instead of branching on both arms of `Result<MrzData, MrzError>` —
/// [`from_parse_result`](Self::from_parse_result) is the constructor meant
/// for that.
///
/// Deliberately **not** a struct that duplicates [`MrzData`]'s own fields
/// (a second, independently-stale source of truth for the same facts):
/// [`Complete`](Self::Complete) is built from the values already on
/// `MrzData` via [`MrzData::sequence_completeness`], and
/// [`Partial`](Self::Partial) mirrors [`MrzError::IncompleteSequence`]'s
/// own fields exactly.
///
/// `lines_found`/`lines_expected` never actually differ inside `Complete`:
/// an `MrzData` cannot exist at all without every line the format's
/// fixed-offset `parse_*` needs already having been found, so a *complete*
/// read has nothing left to report on that axis — which is exactly why
/// those two fields live only on `Partial`, not duplicated onto `Complete`
/// as a pair of always-equal constants.
///
/// It covers only what check digits and the zone's shape can establish.
/// Whether line 1's document code, issuing state and name are plausible —
/// none of which any check digit covers — is a heuristic judgement this type
/// deliberately leaves to the caller.
///
/// ```
/// use mrz::{find_and_parse, format_td3, Format, SequenceCompleteness, Td3Fields};
///
/// let zone = format_td3(&Td3Fields {
///     issuing_country: "UTO".into(),
///     document_number: "E00000000".into(),
///     surname: "ESKANDARI".into(),
///     given_names: "MAREN".into(),
///     nationality: "UTO".into(),
///     date_of_birth: mrz::MrzDate::Calendar(mrz::Date::new(1980, 1, 1)),
///     sex: mrz::Sex::Female,
///     date_of_expiry: mrz::MrzDate::Calendar(mrz::Date::new(2030, 12, 30)),
///     ..Default::default()
/// });
///
/// // Both lines found: a complete sequence, carrying its per-field evidence.
/// assert!(matches!(
///     SequenceCompleteness::from_parse_result(&find_and_parse(&zone)),
///     Some(SequenceCompleteness::Complete { .. }),
/// ));
///
/// // Line 1 alone: recognizably a passport, but only half of one.
/// let line1 = zone.lines().next().unwrap();
/// assert_eq!(
///     SequenceCompleteness::from_parse_result(&find_and_parse(line1)),
///     Some(SequenceCompleteness::Partial {
///         format: Format::Td3,
///         lines_found: 1,
///         lines_expected: 2,
///     }),
/// );
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum SequenceCompleteness {
    /// Every line the format needs was found and parsed into an
    /// [`MrzData`]. Which *fields* carry check-digit evidence is a separate
    /// question — see `checks`/`date_of_birth`.
    Complete {
        /// Per-field check-digit evidence — see [`MrzData::checks`].
        checks: Checks,
        /// Date-of-birth field completeness: [`MrzDate::completeness`] of
        /// [`MrzData::date_of_birth`]. Expiry carries the same
        /// classification on [`MrzData::date_of_expiry`]; this variant has
        /// reported birth only since it shipped, and keeps that shape.
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
    /// [`find_and_parse`] result. `None` when the
    /// error carries no completeness signal of its own — a bare
    /// [`MrzError::NotFound`] (nothing MRZ-shaped at all) or one of the
    /// other structural/checksum error variants, which are about a
    /// *specific* zone a caller already had in hand, not a free-text scan.
    ///
    /// ```
    /// use mrz::{find_and_parse, SequenceCompleteness};
    ///
    /// // Nothing MRZ-shaped at all: there is no sequence to be complete.
    /// let nothing = find_and_parse("just a regular paragraph\nwith two lines");
    /// assert_eq!(SequenceCompleteness::from_parse_result(&nothing), None);
    /// ```
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

/// Which ICAO 9303 layout a zone uses.
///
/// `#[non_exhaustive]`: ICAO 9303 defines formats this crate does not parse yet
/// (MRP-style variants, future parts), so `match` on this must carry a `_` arm
/// and gaining a variant is not a breaking change. Adding MRV-A/MRV-B in 0.2.0
/// was breaking precisely because this attribute was missing; it arrived in
/// 0.4.0.
///
/// ```
/// use mrz::Format;
///
/// let doc = mrz::parse_td1(
///     "I<UTOD231458907<<<<<<<<<<<<<<<",
///     "7408122F1204159UTO<<<<<<<<<<<6",
///     "ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
/// )
/// .unwrap();
/// assert_eq!(doc.format, Format::Td1);
///
/// // Outside this crate, a match needs a wildcard arm for formats added later.
/// let lines = match doc.format {
///     Format::Td1 => 3,
///     Format::Td2 | Format::Td3 | Format::MrvA | Format::MrvB => 2,
///     _ => 0,
/// };
/// assert_eq!(lines, 3);
/// ```
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

impl Format {
    /// Whether each [`Checks`] field has a printed check digit in this layout,
    /// ordered as document number, date of birth, date of expiry, personal
    /// number, then composite. This is layout data, never a property of an
    /// individual read.
    pub const CHECK_DIGIT_APPLICABILITY: [(Self, [bool; 5]); 5] = [
        (Self::Td1, [true, true, true, false, true]),
        (Self::Td2, [true, true, true, false, true]),
        (Self::Td3, [true, true, true, true, true]),
        (Self::MrvA, [true, true, true, false, false]),
        (Self::MrvB, [true, true, true, false, false]),
    ];

    /// Check-digit applicability for this layout. Unknown future formats have
    /// no declared check-digit contract until their parser and table row land
    /// together.
    ///
    /// ```
    /// use mrz::Format;
    ///
    /// // A passport prints all five check digits.
    /// assert_eq!(Format::Td3.check_digit_applicability(), [true; 5]);
    /// // An MRV-A visa prints no personal-number or composite digit.
    /// assert_eq!(
    ///     Format::MrvA.check_digit_applicability(),
    ///     [true, true, true, false, false]
    /// );
    /// ```
    pub fn check_digit_applicability(self) -> [bool; 5] {
        Self::CHECK_DIGIT_APPLICABILITY
            .iter()
            .find_map(|(format, applicability)| (*format == self).then_some(*applicability))
            .unwrap_or([false; 5])
    }
}

/// Parsed and validated MRZ data.
///
/// Every field is decoded from the zone's fixed positions; nothing is taken
/// from outside it. Whether the *read* is checksum-consistent is
/// [`valid`](Self::valid) and, per field, [`checks`](Self::checks); whether the
/// *document* is in date is a separate question, answered by
/// [`validity`](Self::validity).
///
/// ```
/// let doc = mrz::parse_td3(
///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
///     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
/// )
/// .unwrap();
///
/// assert_eq!(doc.document_type, "P"); // trailing `<` filler trimmed
/// assert_eq!(doc.issuing_country, "UTO");
/// assert_eq!(doc.document_number, "L898902C3");
/// assert_eq!(doc.surname, "ERIKSSON");
/// assert_eq!(doc.given_names, "ANNA MARIA"); // `<` separators become spaces
/// assert_eq!(doc.nationality, "UTO");
/// assert_eq!(doc.sex, mrz::Sex::Female);
/// assert_eq!(doc.date_of_expiry.to_string(), "2012-04-15");
/// assert_eq!(doc.optional_data_1.as_deref(), Some("ZE184226B")); // the primary optional-data slot...
/// assert_eq!(doc.personal_number(), Some("ZE184226B"));         // ...which TD3 alone names a personal number
/// assert_eq!(doc.optional_data_2, None);                        // the second slot is TD1's alone
/// assert_eq!(doc.mrz_lines.lines().count(), 2); // exactly what was validated
/// ```
///
/// The `zeroize` feature (off by default) derives `ZeroizeOnDrop`. When a
/// value is dropped, its `String` and `Option<String>` fields are wiped, and
/// so are `date_of_birth`, `date_of_expiry` and `sex`, through the `Zeroize`
/// impls of [`MrzDate`] and [`Sex`]. The wipe is **best-effort**:
///
/// - `format`, `document_number_legacy_encoding` and `checks` are
///   `#[zeroize(skip)]`. They describe the layout and arithmetic, not the holder.
/// - [`MrzDate`] wipes its payload, but its variant remains, revealing the kind
///   of date read. [`Sex`] overwrites the whole value, including its variant.
/// - Copies made before the drop are out of reach: [`MrzDate`] and [`Sex`]
///   are `Copy`, and `clone`, `to_string` and `serde` buffers are not wiped.
/// - `mrz-wasm`, the browser-demo build for `wasm32-unknown-unknown`, never
///   enables the feature, so this crate stays zero-dependency there and wipes
///   nothing.
///
/// `#[non_exhaustive]`: this struct grows as the crate decodes more of the
/// zone — `document_number_full` arrived in 0.4.0, the two optional-data
/// slots in 0.8.0 — and that should not break
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
    /// Document code as printed, trailing filler trimmed: `"P"` for a passport
    /// printed `P<`, `"PD"` for a diplomatic one, `"I"` or `"ID"` for an
    /// identity card, `"V"` for a visa.
    pub document_type: String,
    /// Issuing state or organization (3-letter ICAO code).
    pub issuing_country: String,
    /// The 9-character document-number field as printed, truncated if the
    /// real number overflows — see [`document_number_full`](Self::document_number_full).
    pub document_number: String,
    /// The reassembled document number when it overflows the 9-character field
    /// (ICAO 9303 Part 5 note j / §4.2.4 for TD1, Part 6 note j for TD2; Part 4
    /// defines no such rule, so TD3 gets it by a deliberate extension, because
    /// issuers do it in practice. MRVs have no overflow encoding at all, per
    /// Part 7). `None` when the number fits, in which case
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
    /// The holder's date of birth, as the field holds it: see [`MrzDate`].
    /// Six digits are [`MrzDate::Calendar`], or [`MrzDate::OutOfCalendar`]
    /// when they name no real day, with the century inferred from
    /// [`ParseOptions::pivot_yy`] (see [`expand_date`]). Doc 9303 Part 3 §4.8
    /// fillers are [`MrzDate::PartiallyUnknown`] or [`MrzDate::Unknown`]; any
    /// other character is [`MrzDate::Malformed`].
    ///
    /// An unknown or partially unknown date of birth is not a bad read. Part
    /// 3 §4.8 explicitly lets an issuer fill unknown positions with `<`, and
    /// a filler counts as zero for check-digit purposes, so an all-filler date
    /// of birth with check digit `0` verifies. The variant distinguishes an
    /// issuer's unknown from OCR garbage. [`MrzDate::completeness`] gives the
    /// same answer as [`DateCompleteness`], and [`MrzDate::calendar`] gives a
    /// usable date when one exists.
    ///
    /// Its text form is ISO `YYYY-MM-DD` for six digits and the raw field
    /// otherwise: exactly the string this field held before 0.8.0 (ADR-0020).
    pub date_of_birth: MrzDate,
    /// The sex cell: [`Sex::Male`], [`Sex::Female`], [`Sex::Unspecified`] for
    /// the filler `<`, or [`Sex::NonConformant`] holding any other character
    /// as read. No check digit covers this cell. Its text form is the zone
    /// character; before 0.8.0 every cell other than `M`/`F` read as `"X"`.
    pub sex: Sex,
    /// The document's date of expiry, as the field holds it: the same
    /// [`MrzDate`] classification as [`date_of_birth`](Self::date_of_birth),
    /// except that six digits always read as 20xx (see [`expand_date`]).
    pub date_of_expiry: MrzDate,
    /// The format's *primary* optional-data element, trailing filler trimmed,
    /// and without the overflow remainder when a long document number spilled
    /// into it: TD1 line 1 positions 16-30 (Doc 9303 Part 5 §4.2.2.1's "Optional data
    /// elements"), TD2 line 2 positions 29-35, TD3 line 2 positions 29-42
    /// (Part 4 §4.2.2 titles it "personal number **or other optional data
    /// elements**"), MRV-A line 2 positions 29-44, MRV-B line 2 positions
    /// 29-36. It is the document-number overflow target on every format that
    /// defines one. `None` when the field is all filler.
    pub optional_data_1: Option<String>,
    /// TD1's *second* optional-data element, line 2 positions 19-29, trailing
    /// filler trimmed. `None` on every other format — each prints exactly one
    /// optional-data element, which lives in
    /// [`optional_data_1`](Self::optional_data_1) — and on a TD1 whose second
    /// element is all filler.
    pub optional_data_2: Option<String>,
    /// The MRZ lines this record was parsed from, newline-joined, exactly as
    /// validated — which is the *repaired* zone when [`find_and_parse`]
    /// normalized or repaired the input, not the caller's original text.
    pub mrz_lines: String,
    /// Per-field check-digit verification results — see [`Checks`].
    #[cfg_attr(feature = "zeroize", zeroize(skip))]
    pub checks: Checks,
}

impl MrzData {
    /// Every check digit this format prints agrees with the candidate.
    ///
    /// The named form of what [`valid`](Self::valid) computes, and the
    /// preferred spelling: `valid` sits three letters from
    /// [`validity`](Self::validity) and means something entirely different,
    /// so the short name reads as a verdict on the *document* when it is a
    /// verdict on the *arithmetic*. Both call
    /// [`checks.all_valid()`](Checks::all_valid); neither is going away
    /// inside 0.8.
    ///
    /// **This is checksum consistency — not document validity, and not
    /// byte-identity with the printed zone.** Whether the document is in date
    /// is [`validity`](Self::validity); what the arithmetic cannot see is
    /// [`Blindspot`].
    ///
    /// ```
    /// const L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";
    ///
    /// let doc = mrz::parse_td3("P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", L2).unwrap();
    /// assert!(doc.checksum_consistent());
    ///
    /// // And what that does not establish. TD3's composite spans line 2 only
    /// // (positions 1-10, 14-20, 22-43), so line 1 carries no check digit at
    /// // all: a different surname is exactly as consistent.
    /// let altered = mrz::parse_td3("P<UTOERIKSSDN<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", L2).unwrap();
    /// assert_eq!(altered.surname, "ERIKSSDN");
    /// assert!(altered.checksum_consistent());
    /// ```
    pub fn checksum_consistent(&self) -> bool {
        self.checks.all_valid()
    }

    /// Shorthand for [`checks.all_valid()`](Checks::all_valid): every check
    /// digit this format prints agrees with the candidate. Same answer as
    /// [`checksum_consistent`](Self::checksum_consistent), which is the
    /// clearer name for it.
    ///
    /// A failed check digit is a verdict on the read, not a parse error: the
    /// zone still parses, and this is where the verdict lives.
    ///
    /// **This is checksum consistency, not document validity and not
    /// byte-identity with the printed zone.** Whether the document is in date
    /// is [`validity`](Self::validity); what the arithmetic cannot see is
    /// [`Blindspot`].
    ///
    /// ```
    /// let l1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    /// assert!(mrz::parse_td3(l1, "L898902C36UTO7408122F1204159ZE184226B<<<<<10").unwrap().valid());
    ///
    /// // One digit of the expiry altered: still an `Ok`, no longer valid.
    /// let tampered = mrz::parse_td3(l1, "L898902C36UTO7408122F1204169ZE184226B<<<<<10").unwrap();
    /// assert!(!tampered.valid());
    /// ```
    pub fn valid(&self) -> bool {
        self.checksum_consistent()
    }

    /// The complete document number: the overflow reassembly when there is
    /// one, otherwise the 9-character field as printed.
    ///
    /// When a document number exceeds the 9-character field, Doc 9303 puts the
    /// nine principal characters in the field, a filler in the check-digit
    /// position, and the remainder plus its own check digit at the front of the
    /// optional-data field. See [`MrzData::document_number_full`] for the
    /// normative scope of that rule and the TD3 caveat; when the remainder does
    /// not fit the optional field, the number truncates to nine characters
    /// instead.
    ///
    /// ```
    /// use mrz::{format_td3, Td3Fields};
    ///
    /// let lines = format_td3(&Td3Fields {
    ///     issuing_country: "UTO".into(),
    ///     document_number: "L898902C31234".into(), // 13 chars, overflows the 9-char field
    ///     surname: "ERIKSSON".into(),
    ///     given_names: "ANNA MARIA".into(),
    ///     nationality: "UTO".into(),
    ///     date_of_birth: mrz::MrzDate::Calendar(mrz::Date::new(1974, 8, 12)),
    ///     sex: mrz::Sex::Female,
    ///     date_of_expiry: mrz::MrzDate::Calendar(mrz::Date::new(2012, 4, 15)),
    ///     ..Default::default()
    /// });
    ///
    /// let (l1, l2) = lines.split_once('\n').unwrap();
    /// let doc = mrz::parse_td3(l1, l2).unwrap();
    /// assert!(doc.valid());
    /// assert_eq!(doc.document_number, "L898902C3");            // the printed 9-char field
    /// assert_eq!(doc.full_document_number(), "L898902C31234"); // the reassembled number
    /// assert!(!doc.document_number_legacy_encoding);           // read as the Part 5/6 note j form, applied to TD3 by analogy
    /// ```
    pub fn full_document_number(&self) -> &str {
        self.document_number_full
            .as_deref()
            .unwrap_or(&self.document_number)
    }

    /// The personal number under this crate's ADR-0018 naming policy. TD3's
    /// line 2 positions 29-42 are the only field ICAO labels a personal number
    /// with its own check digit
    /// ([`Checks::personal_number`]). It is
    /// [`optional_data_1`](Self::optional_data_1) read through the name Doc
    /// 9303 Part 4 gives that field. On every other format the same slot
    /// holds issuer-discretionary optional data and this returns `None`, so
    /// a caller cannot mistake one for the other.
    ///
    /// Before 0.8.0 this was a field that also held TD2 and MRV optional
    /// data, and TD1's two elements joined with a space — a join that could
    /// not be undone (ADR-0018).
    ///
    /// ```
    /// let doc = mrz::parse_td3(
    ///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    ///     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
    /// )
    /// .unwrap();
    /// assert_eq!(doc.personal_number(), Some("ZE184226B"));
    ///
    /// let card = mrz::parse_td2(
    ///     "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
    ///     "D231458907UTO7408122F1204159<<<<<<<6",
    /// )
    /// .unwrap();
    /// assert_eq!(card.personal_number(), None); // ADR-0018 names TD2's field optional data
    /// ```
    pub fn personal_number(&self) -> Option<&str> {
        match self.format {
            Format::Td3 => self.optional_data_1.as_deref(),
            _ => None,
        }
    }

    /// Human-readable name of the issuing state, if the code is recognized.
    ///
    /// ```
    /// let doc = mrz::parse_td2(
    ///     "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
    ///     "D231458907UTO7408122F1204159<<<<<<<6",
    /// )
    /// .unwrap();
    /// assert_eq!(doc.issuing_country_name(), Some("Utopia (ICAO specimen)"));
    /// ```
    pub fn issuing_country_name(&self) -> Option<&'static str> {
        country_name(&self.issuing_country)
    }

    /// The passport type this document's code designates, per ICAO 9303
    /// Part 4 §4.4.
    ///
    /// `None` covers three cases the caller usually wants to tell apart, and
    /// which this method deliberately does not distinguish on its own:
    ///
    /// * the document is not a passport (`Format::Td1`, `Td2`, `MrvA`, `MrvB`);
    /// * it is a passport carrying the `P<` filler form, i.e. no secondary
    ///   code — conformant today, and the most common case in real corpora;
    /// * the second character is outside the §4.4 table.
    ///
    /// Check [`format`](Self::format) and [`document_type`](Self::document_type)
    /// when the difference matters. A code outside the table never makes a
    /// document unparseable — see [`PassportType`] for why recognition here is
    /// deliberately not rejection.
    ///
    /// ```
    /// use mrz::{format_td3, parse_td3, PassportType, Td3Fields};
    ///
    /// // A diplomatic passport carries the secondary code `PD`.
    /// let zone = format_td3(&Td3Fields { document_code: "PD".into(), ..Default::default() });
    /// let (l1, l2) = zone.split_once('\n').unwrap();
    /// assert_eq!(parse_td3(l1, l2).unwrap().passport_type(), Some(PassportType::Diplomatic));
    ///
    /// // The older-edition ICAO specimen prints `P<`: no secondary code, which is conformant.
    /// let specimen = parse_td3(
    ///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    ///     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
    /// )
    /// .unwrap();
    /// assert_eq!(specimen.passport_type(), None);
    /// ```
    pub fn passport_type(&self) -> Option<PassportType> {
        passport_type(&self.document_type)
    }

    /// Human-readable name of the nationality, if the code is recognized.
    ///
    /// The holder's nationality need not be the issuing state's:
    ///
    /// ```
    /// // Part 7-style visa issued by Utopia to a Brazilian national.
    /// let visa = mrz::parse_mrv_a(
    ///     "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    ///     "XK93054875BRA8502212F2703143R5T6U7V8W9<<<<<<",
    /// )
    /// .unwrap();
    /// assert_eq!(visa.issuing_country_name(), Some("Utopia (ICAO specimen)"));
    /// assert_eq!(visa.nationality_name(), Some("Brazil"));
    /// ```
    pub fn nationality_name(&self) -> Option<&'static str> {
        country_name(&self.nationality)
    }

    /// This record's [`SequenceCompleteness`] — always
    /// [`SequenceCompleteness::Complete`], computed from `self.checks`/
    /// `self.date_of_birth.completeness()` rather than stored separately.
    /// [`SequenceCompleteness::Partial`] only ever comes from
    /// [`SequenceCompleteness::from_parse_result`] on the `Err` side of a
    /// [`find_and_parse`] call, since a `Partial`
    /// read has no `MrzData` to hang this method off of in the first place.
    ///
    /// ```
    /// use mrz::{DateCompleteness, SequenceCompleteness};
    ///
    /// let doc = mrz::parse_td3(
    ///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    ///     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
    /// )
    /// .unwrap();
    /// match doc.sequence_completeness() {
    ///     SequenceCompleteness::Complete { checks, date_of_birth } => {
    ///         assert!(checks.all_valid());
    ///         assert_eq!(date_of_birth, DateCompleteness::Complete);
    ///     }
    ///     other => panic!("an MrzData is always a complete sequence, got {other:?}"),
    /// }
    /// ```
    pub fn sequence_completeness(&self) -> SequenceCompleteness {
        SequenceCompleteness::Complete {
            checks: self.checks.clone(),
            date_of_birth: self.date_of_birth.completeness(),
        }
    }
}

/// Which check-digit-bearing field an error refers to.
///
/// ```
/// use mrz::Field;
///
/// // The ICAO specimen with its expiry altered: 120415 → 120416.
/// let doc = mrz::parse_td3(
///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
///     "L898902C36UTO7408122F1204169ZE184226B<<<<<10",
/// )
/// .unwrap();
/// assert!(doc.checks.failed().contains(&Field::DateOfExpiry));
///
/// // `Display` uses the same name as the `Checks` field.
/// assert_eq!(Field::DateOfExpiry.to_string(), "date_of_expiry");
/// ```
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
    ///
    /// ```
    /// assert_eq!(mrz::Field::DocumentNumber.as_str(), "document_number");
    /// assert_eq!(mrz::Field::Composite.as_str(), "composite");
    /// ```
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
    /// Number of check digits this format prints. A zero-applicable `Checks`
    /// is never valid: there is no check-digit evidence to establish a read.
    ///
    /// ```
    /// use mrz::{parse_td3, Field};
    ///
    /// let line1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    /// let good = parse_td3(line1, "L898902C36UTO7408122F1204159ZE184226B<<<<<10").unwrap();
    /// // The date-of-birth check digit misprinted as 3: that check and the composite fail.
    /// let bad = parse_td3(line1, "L898902C36UTO7408123F1204159ZE184226B<<<<<10").unwrap();
    /// assert_eq!(good.checks.applicable(), 5);
    /// assert_eq!(bad.checks.applicable(), 5); // a failed check is still applicable
    /// ```
    pub fn applicable(&self) -> u8 {
        [
            self.document_number,
            self.date_of_birth,
            self.date_of_expiry,
            self.personal_number,
            self.composite,
        ]
        .into_iter()
        .filter(|check| check.is_some())
        .count() as u8
    }

    /// Number of printed check digits that verified.
    ///
    /// ```
    /// use mrz::{parse_td3, Field};
    ///
    /// let line1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    /// let good = parse_td3(line1, "L898902C36UTO7408122F1204159ZE184226B<<<<<10").unwrap();
    /// // The date-of-birth check digit misprinted as 3: that check and the composite fail.
    /// let bad = parse_td3(line1, "L898902C36UTO7408123F1204159ZE184226B<<<<<10").unwrap();
    /// assert_eq!(good.checks.verified(), 5);
    /// assert_eq!(bad.checks.verified(), 3);
    /// ```
    pub fn verified(&self) -> u8 {
        [
            self.document_number,
            self.date_of_birth,
            self.date_of_expiry,
            self.personal_number,
            self.composite,
        ]
        .into_iter()
        .filter(|check| *check == Some(true))
        .count() as u8
    }

    /// The fields whose printed check digits failed, in field order. Absent
    /// checks are not failures. Empty when [`all_valid`](Checks::all_valid) is
    /// `true`.
    ///
    /// ```
    /// use mrz::{parse_td3, Field};
    ///
    /// let line1 = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    /// let good = parse_td3(line1, "L898902C36UTO7408122F1204159ZE184226B<<<<<10").unwrap();
    /// // The date-of-birth check digit misprinted as 3: that check and the composite fail.
    /// let bad = parse_td3(line1, "L898902C36UTO7408123F1204159ZE184226B<<<<<10").unwrap();
    /// assert!(good.checks.failed().is_empty());
    /// assert_eq!(bad.checks.failed(), vec![Field::DateOfBirth, Field::Composite]);
    /// ```
    pub fn failed(&self) -> Vec<Field> {
        [
            (self.document_number, Field::DocumentNumber),
            (self.date_of_birth, Field::DateOfBirth),
            (self.date_of_expiry, Field::DateOfExpiry),
            (self.personal_number, Field::PersonalNumber),
            (self.composite, Field::Composite),
        ]
        .into_iter()
        .filter_map(|(ok, f)| (ok == Some(false)).then_some(f))
        .collect()
    }
}

/// Why parsing an MRZ zone failed outright — distinct from a failed check
/// digit, which `parse_*` reports through [`Checks`] instead of an error.
///
/// ```
/// use mrz::{find_and_parse, parse_td3, MrzError};
///
/// const L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";
///
/// // Structural failures are errors ...
/// assert_eq!(parse_td3("P<UTOERIKSSON", L2), Err(MrzError::BadLength { expected: 44, got: 13 }));
/// assert_eq!(
///     parse_td3("I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", L2),
///     Err(MrzError::BadDocumentCode("I<".into())), // not a passport code
/// );
/// assert_eq!(
///     find_and_parse("just a regular paragraph\nwith two lines"),
///     Err(MrzError::NotFound),
/// );
///
/// // ... a failed check digit is not: the zone still parses, and says which.
/// let doc = parse_td3("P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<", L2).unwrap();
/// assert!(doc.valid());
///
/// // Every variant renders a readable message.
/// assert_eq!(MrzError::NotFound.to_string(), "no MRZ found in text");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum MrzError {
    /// Line has the wrong length for the claimed format, counted in `char`s
    /// (a right-length line holding a non-MRZ character is a [`MrzError::BadCharacter`]).
    BadLength {
        /// The length the claimed format requires.
        expected: usize,
        /// The number of characters actually supplied.
        got: usize,
    },
    /// Character outside `[A-Z0-9<]`.
    ///
    /// Both indices are zero-based and counted in `char`s, not bytes.
    ///
    /// `line` is `None` when there was no zone to count lines in: the
    /// standalone [`check_digit`] helper is handed a
    /// single field, so its `position` is field-relative. It is deliberately
    /// **not** `Some(0)` -- line 0 is a real line of a real zone, and a caller
    /// must be able to tell "the first line" from "no line at all". See
    /// [ADR-0017] for why this crate does not spell absence as a legitimate
    /// value.
    ///
    /// [ADR-0017]: https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/decisions/ADR-0017-checks-distinguish-absent-from-verified.md
    BadCharacter {
        /// The character that is not in the MRZ alphabet.
        character: char,
        /// Zero-based line within the MRZ zone, or `None` when the error did
        /// not come from parsing one -- see the variant documentation.
        line: Option<usize>,
        /// Zero-based `char` column within the line, or within the field when
        /// `line` is `None`.
        position: usize,
    },
    /// Document code not recognized for the format; the payload is the two raw cells, including filler.
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
    /// [`SequenceCompleteness`] folds this and a successful read into one
    /// value.
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
            Self::BadCharacter {
                character,
                line: Some(line),
                position,
            } => write!(
                f,
                "invalid MRZ character: {character:?} at line {line}, column {position}"
            ),
            Self::BadCharacter {
                character,
                line: None,
                position,
            } => write!(
                f,
                "invalid MRZ character: {character:?} at position {position}"
            ),
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

    #[test]
    fn zero_applicable_checks_are_not_valid() {
        let checks = Checks {
            document_number: None,
            date_of_birth: None,
            date_of_expiry: None,
            personal_number: None,
            composite: None,
        };
        assert_eq!(checks.applicable(), 0);
        assert_eq!(checks.verified(), 0);
        assert!(!checks.all_valid());
        assert!(checks.failed().is_empty());
    }

    // Utopia / Anna Maria Eriksson: line 2 is printed in Part 3 §3.2
    // (PDF p11), including document-number check 6 and personal-number
    // check 1. This P< line 1 is from a pre-amendment edition; current
    // Parts 3 and 4 print PP and Part 4 uses a 2034 expiry.
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
        assert_eq!(d.date_of_birth.to_string(), "1974-08-12");
        assert_eq!(d.date_of_birth.completeness(), DateCompleteness::Complete);
        assert_eq!(d.sex, Sex::Female);
        assert_eq!(d.date_of_expiry.to_string(), "2012-04-15");
        assert_eq!(d.personal_number(), Some("ZE184226B"));
    }

    #[test]
    fn td3_tampered_dob_fails_checksum() {
        // Change one digit of the date of birth: 740812 → 750812.
        let tampered = TD3_L2.replacen("740812", "750812", 1);
        let d = parse_td3(TD3_L1, &tampered).unwrap();
        assert_eq!(d.checks.date_of_birth, Some(false));
        assert_eq!(d.checks.composite, Some(false));
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
        assert_eq!(d.checks.date_of_birth, Some(true));
        assert_eq!(d.date_of_birth, MrzDate::Unknown); // an issuer's unknown
        assert_eq!(d.date_of_birth.to_string(), "<<<<<<"); // raw text form
        assert_eq!(d.date_of_birth.completeness(), DateCompleteness::Unknown);
    }

    // Doc 9303 Part 4 §4.2.2.2 position 43: an unused TD3 personal number's check digit
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
        assert_eq!(d.checks.personal_number, Some(true));
        assert_eq!(d.checks.composite, Some(true));
        assert_eq!(d.personal_number(), None);
    }

    #[test]
    fn td3_empty_personal_number_with_filler_check_digit_filler() {
        // Same fixture, check digit '<' instead of '0' — same composite
        // digit, since both digit characters have ICAO value 0.
        let l2 = "L898902C36UTO7408122F1204159<<<<<<<<<<<<<<<8";
        let d = parse_td3(TD3_L1, l2).unwrap();
        assert_eq!(d.checks.personal_number, Some(true));
        assert_eq!(d.checks.composite, Some(true));
        assert_eq!(d.personal_number(), None);
    }

    // Same Utopia/Eriksson identity as the TD3/TD2 specimens, reshaped into
    // TD1's three lines are printed in Part 5 Appendix A Figure A-2
    // (PDF p29), byte for byte.
    const TD1_L1: &str = "I<UTOD231458907<<<<<<<<<<<<<<<";
    const TD1_L2: &str = "7408122F1204159UTO<<<<<<<<<<<6";
    const TD1_L3: &str = "ERIKSSON<<ANNA<MARIA<<<<<<<<<<";

    #[test]
    fn bad_character_reports_zero_based_td1_line_and_column() {
        for (line, expected_position) in [(0, 3), (1, 7), (2, 11)] {
            let mut lines = [TD1_L1.to_string(), TD1_L2.to_string(), TD1_L3.to_string()];
            lines[line].replace_range(expected_position..expected_position + 1, "?");

            let error = parse_td1(&lines[0], &lines[1], &lines[2]).unwrap_err();
            assert_eq!(
                error,
                MrzError::BadCharacter {
                    character: '?',
                    line: Some(line),
                    position: expected_position,
                }
            );
            assert_eq!(
                error.to_string(),
                format!("invalid MRZ character: '?' at line {line}, column {expected_position}")
            );
        }
    }

    #[test]
    fn bad_character_from_the_standalone_helper_reports_no_line() {
        // `check_digit` is handed a field, not a zone, so there is no line to
        // count. `None` says that; `Some(0)` would be indistinguishable from
        // the genuine first line of a genuine zone.
        let error = crate::check_digit("L8?8902C3").unwrap_err();
        assert_eq!(
            error,
            MrzError::BadCharacter {
                character: '?',
                line: None,
                position: 2,
            }
        );
        assert_eq!(
            error.to_string(),
            "invalid MRZ character: '?' at position 2"
        );
    }

    #[test]
    fn bad_character_position_counts_chars_not_bytes() {
        // The length gate counts `char`s, so a line of the format's width that
        // carries a two-byte character reaches the charset check. `position`
        // is a `char` index -- the column someone counting glyphs would name,
        // not a byte offset.
        let e_acute = char::from_u32(0xC9).expect("U+00C9 is a valid scalar value");
        let line1 = format!("P<UTO{e_acute}{}", "<".repeat(38));
        assert_eq!(
            line1.chars().count(),
            44,
            "char length must satisfy the TD3 gate"
        );
        assert_eq!(line1.len(), 45);

        let error = parse_td3(&line1, &"<".repeat(44)).unwrap_err();
        assert_eq!(
            error,
            MrzError::BadCharacter {
                character: e_acute,
                line: Some(0),
                position: 5,
            }
        );
    }

    #[test]
    fn td1_specimen_fully_valid() {
        let d = parse_td1(TD1_L1, TD1_L2, TD1_L3).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::Td1);
        assert_eq!(d.document_type, "I");
        assert_eq!(d.document_number, "D23145890");
        assert_eq!(d.surname, "ERIKSSON");
        assert_eq!(d.given_names, "ANNA MARIA");
        assert_eq!(d.date_of_birth.to_string(), "1974-08-12");
        assert_eq!(d.date_of_birth.completeness(), DateCompleteness::Complete);
        assert_eq!(d.date_of_expiry.to_string(), "2012-04-15");
    }

    #[test]
    fn td1_all_filler_dob_reports_unknown() {
        // Same all-filler-DOB scenario as the TD3 test above, but for TD1's
        // layout, where the dob+check field is line2[0..7] rather than
        // line2[13..20] — covers a second of the five formats per the S5 plan.
        let l2 = format!("{}{}", "<<<<<<0", &TD1_L2[7..]);
        assert_eq!(l2.len(), TD1_L2.len());
        let d = parse_td1(TD1_L1, &l2, TD1_L3).unwrap();
        assert_eq!(d.checks.date_of_birth, Some(true));
        assert_eq!(d.date_of_birth, MrzDate::Unknown);
        assert_eq!(d.date_of_birth.completeness(), DateCompleteness::Unknown);
    }

    // Official ICAO 9303 part 6 TD2 specimen (Utopia / Anna Maria Eriksson),
    // published verbatim as text:
    // `knowledge/docs9303/Doc_9303_Part6_Specs_for_TD2_MROTDs.md:487-488`.
    const TD2_L1: &str = "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<";
    const TD2_L2: &str = "D231458907UTO7408122F1204159<<<<<<<6";

    #[test]
    fn sex_cell_is_classified_not_collapsed() {
        for (cell, expected) in [
            ('M', Sex::Male),
            ('F', Sex::Female),
            ('<', Sex::Unspecified),
            ('X', Sex::NonConformant('X')),
            ('1', Sex::NonConformant('1')),
        ] {
            let mut l2 = TD3_L2.to_string();
            l2.replace_range(20..21, &cell.to_string());
            let d = parse_td3(TD3_L1, &l2).unwrap();
            assert!(d.valid(), "cell {cell:?}: checks {:?}", d.checks);
            assert_eq!(d.sex, expected, "cell {cell:?}");
            assert_eq!(d.sex.to_string(), cell.to_string());
        }
    }

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
        assert_eq!(d.date_of_birth.to_string(), "1974-08-12");
        assert_eq!(d.sex, Sex::Female);
        assert_eq!(d.date_of_expiry.to_string(), "2012-04-15");
    }

    #[test]
    fn td2_tampered_expiry_fails_checksum() {
        let tampered = TD2_L2.replacen("120415", "120416", 1);
        let d = parse_td2(TD2_L1, &tampered).unwrap();
        assert_eq!(d.checks.date_of_expiry, Some(false));
        assert_eq!(d.checks.composite, Some(false));
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
        assert_eq!(d.checks.document_number, Some(true));
        assert_eq!(d.checks.date_of_birth, Some(true));
        assert_eq!(d.checks.date_of_expiry, Some(true));
    }

    #[test]
    fn checksum_verified_ocr_repair() {
        // Verbatim tesseract.js output for the Croatian specimen at low
        // resolution: trailing fillers read as K/L runs, a hallucinated
        // leading '1' on line 2 (45 chars), and 'B' where '8' is printed.
        // The check digits accept exactly one of the repaired variants.
        let text = "I 01072009 PUJZAGREB 0\n\nBIDFD WH5SS A 2\n\n01072014\nP<HRVSPECIMEN<<SPECIMEN<KLLLLLLLLLLLLLLLLLKLKL\n10070070071HRVB212258F1407019<<<<<<<<<<<<<<06\n";
        let d = find_and_parse(text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.surname, "SPECIMEN");
        assert_eq!(d.given_names, "SPECIMEN");
        assert_eq!(d.document_number, "007007007");
        assert_eq!(d.date_of_birth.to_string(), "1982-12-25");
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
        assert_eq!(d.personal_number(), None);
    }

    #[test]
    fn td1_from_single_docling_line_with_k_misreads() {
        // Verbatim docling OCR of the Slovenian 2022 specimen ID card rear:
        // all three TD1 lines in ONE paragraph, `<` escaped as &lt;, and the
        // K-for-filler misreads in data fields (145K<→145<<, VZORECKK→VZOREC<<).
        // IK is also a legal TD1 code, so its second cell cannot be repaired
        // from the code alone and is preserved as read.
        let text = "1F9874543\n\nIKSVNIE987654302806985505145K&lt; 8506287F3203282SVN&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;&lt;2 VZORECKKJANAKKKKKKKKK&lt;&lt;KK";
        let d = find_and_parse(text).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::Td1);
        assert_eq!(d.document_type, "IK");
        assert_eq!(d.issuing_country, "SVN");
        assert_eq!(d.document_number, "IE9876543");
        assert_eq!(d.surname, "VZOREC");
        assert_eq!(d.given_names, "JANA");
        assert_eq!(d.date_of_birth.to_string(), "1985-06-28");
        assert_eq!(d.date_of_expiry.to_string(), "2032-03-28");
        // The trailing K in the EMŠO field is a filler misread that check
        // digits cannot catch (K ≡ < mod 10) — heuristic cleanup handles it.
        assert_eq!(d.optional_data_1.as_deref(), Some("2806985505145"));
        assert_eq!(d.personal_number(), None, "a TD1 prints no personal number");
    }

    #[test]
    fn ocr_repair_deeply_truncated_name_line() {
        // Verbatim ocrs output for the Croatian specimen at 600×421: line 2 is
        // read perfectly, but line 1 loses NINE trailing fillers (35/44 chars)
        // and its `<` document-code filler is misread as `K`. The name line
        // carries no check digit of its own, so padding the filler run back is
        // safe -- it cannot change any check digit, and line 2's still verify.
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
        assert_eq!(d.checks.date_of_birth, Some(false));
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
        assert_eq!(d.sex, Sex::Female);
        // The document-number field is `L898902C<`: eight characters padded to
        // nine with a filler, check digit 3. The filler is padding, not the
        // long-number signal — that would need the *check digit* position to
        // be a filler — so no overflow is read here.
        assert_eq!(d.document_number, "L898902C");
        assert_eq!(d.document_number_full, None);
        assert_eq!(d.optional_data_1.as_deref(), Some("ZE184226B"));
        assert_eq!(
            d.personal_number(),
            None,
            "a visa prints no personal number"
        );
        assert_eq!(d.date_of_birth.to_string(), "1969-08-06");
        // Doc 9303 defines no century rule (part 3 §4.8 is silent), so this
        // crate's own policy applies: expiry is always read as 20xx. ICAO's
        // specimen is a 1990s document, so that policy renders 940623 as 2094
        // rather than 1994. Pinned deliberately — it is the documented
        // heuristic behaving as designed, and the clearest illustration of its
        // limit. See `dates::expand_date`.
        assert_eq!(d.date_of_expiry.to_string(), "2094-06-23");
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
        assert_eq!(d.optional_data_1.as_deref(), Some("ZE184226"));
        assert_eq!(
            d.personal_number(),
            None,
            "a visa prints no personal number"
        );
        assert_eq!(d.date_of_birth.to_string(), "1969-08-06");
        assert_eq!(d.date_of_expiry.to_string(), "2094-06-23");
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
        assert_eq!(d.date_of_birth.to_string(), "1985-02-21");
        assert_eq!(d.sex, Sex::Female);
        assert_eq!(d.date_of_expiry.to_string(), "2027-03-14");
        assert_eq!(d.document_number, "XK9305487");
        assert_eq!(d.optional_data_1.as_deref(), Some("R5T6U7V8W9"));
        assert_eq!(
            d.personal_number(),
            None,
            "a visa prints no personal number"
        );
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
    fn parser_check_presence_matches_the_format_matrix() {
        let expected = [
            (Format::Td3, [true, true, true, true, true]),
            (Format::Td1, [true, true, true, false, true]),
            (Format::Td2, [true, true, true, false, true]),
            (Format::MrvA, [true, true, true, false, false]),
            (Format::MrvB, [true, true, true, false, false]),
        ];
        for (format, applicability) in expected {
            assert_eq!(format.check_digit_applicability(), applicability);
        }

        let parsed = [
            parse_td1(TD1_L1, TD1_L2, TD1_L3).unwrap(),
            parse_td2(TD2_L1, TD2_L2).unwrap(),
            parse_td3(TD3_L1, TD3_L2).unwrap(),
            parse_mrv_a(MRV_A_L1, MRV_A_L2).unwrap(),
            parse_mrv_b(MRV_B_L1, MRV_B_L2).unwrap(),
        ];

        for data in parsed {
            let observed = [
                data.checks.document_number.is_some(),
                data.checks.date_of_birth.is_some(),
                data.checks.date_of_expiry.is_some(),
                data.checks.personal_number.is_some(),
                data.checks.composite.is_some(),
            ];
            assert_eq!(
                observed,
                data.format.check_digit_applicability(),
                "{:?}: parser presence must follow the format matrix",
                data.format
            );
        }
    }

    #[test]
    fn mrv_b_specimen_fully_valid() {
        let d = parse_mrv_b(MRV_B_L1, MRV_B_L2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.format, Format::MrvB);
        assert_eq!(d.nationality, "DEU");
        assert_eq!(d.date_of_birth.to_string(), "1992-01-01");
        assert_eq!(d.date_of_expiry.to_string(), "2027-06-30");
        assert_eq!(d.document_number, "L23456789");
        assert_eq!(d.optional_data_1.as_deref(), Some("QW12ER34"));
        assert_eq!(
            d.personal_number(),
            None,
            "a visa prints no personal number"
        );
    }

    #[test]
    fn mrv_a_tampered_dob_fails_checksum() {
        let tampered = MRV_A_L2.replacen("850221", "860221", 1);
        let d = parse_mrv_a(MRV_A_L1, &tampered).unwrap();
        assert_eq!(d.checks.date_of_birth, Some(false));
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
            date_of_birth: MrzDate::Calendar(Date::new(1974, 8, 12)),
            sex: Sex::Female,
            date_of_expiry: MrzDate::Calendar(Date::new(2012, 4, 15)),
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
        assert_eq!(d.personal_number(), None);
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
        assert_eq!(d.personal_number(), Some("ZE184"));
    }

    #[test]
    fn td2_and_td1_long_document_numbers_round_trip() {
        let td2 = Td2Fields {
            document_number: "D23145890XY".into(), // 11 chars; remainder fits 7
            date_of_birth: MrzDate::Calendar(Date::new(1974, 8, 12)),
            date_of_expiry: MrzDate::Calendar(Date::new(2012, 4, 15)),
            ..Td2Fields::default()
        };
        let mrz = format_td2(&td2);
        let (l1, l2) = mrz.split_once('\n').unwrap();
        let d = parse_td2(l1, l2).unwrap();
        assert!(d.valid(), "checks: {:?}", d.checks);
        assert_eq!(d.document_number_full.as_deref(), Some("D23145890XY"));

        let td1 = Td1Fields {
            document_number: "D23145890ABCDE".into(), // 14 chars; remainder fits 15
            date_of_birth: MrzDate::Calendar(Date::new(1974, 8, 12)),
            date_of_expiry: MrzDate::Calendar(Date::new(2012, 4, 15)),
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
            date_of_birth: MrzDate::Calendar(Date::new(1974, 8, 12)),
            date_of_expiry: MrzDate::Calendar(Date::new(2012, 4, 15)),
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
        assert_eq!(d.checks.document_number, Some(false));
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
        assert_eq!(d.date_of_birth.to_string(), "1974-08-12");

        // With a pivot of 80, YY=74 reads as 2074 rather than 1974.
        let opts = ParseOptions::default().with_pivot_yy(80);
        let d = parse_td3_with(TD3_L1, TD3_L2, &opts).unwrap();
        assert_eq!(d.date_of_birth.to_string(), "2074-08-12");
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
        assert_eq!(d.checks.document_number, Some(true));
        assert_eq!(d.checks.date_of_birth, Some(false));
        assert_eq!(d.checks.date_of_expiry, Some(false));
    }

    #[test]
    fn country_names_surface_on_mrzdata() {
        let d = parse_td1(TD1_L1, TD1_L2, TD1_L3).unwrap();
        assert_eq!(d.issuing_country_name(), Some("Utopia (ICAO specimen)"));
        assert_eq!(d.nationality_name(), Some("Utopia (ICAO specimen)"));
    }
}
