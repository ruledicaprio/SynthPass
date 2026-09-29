//! Line-1 integrity checks for ICAO 9303 MRZ reads.
//!
//! **The defect this module exists to catch**: TD1/TD2/TD3 check digits cover
//! only `document_number`, `date_of_birth`, `date_of_expiry`, and
//! `personal_number` (verified directly against the real ICAO fixture in
//! `mrz::dates` tests and `mrz::parser`'s composite ranges — the composite
//! excludes `nationality` and `sex` too, matching the published standard, not
//! a bug in this codebase). `document_type`, `issuing_country`, `surname`,
//! `given_names` carry **no check digit at all**. A document can be
//! checksum-proven — `MrzData::valid() == true` — and still have the wrong
//! name, because nothing mathematically ties line 1 to anything.
//!
//! Measured on the synthetic corpus (`synthpass-bench`, `feat/bench-per-field-cer`):
//! of documents passing the Tier-1 gate, the dominant failure is OCR
//! collapsing interior `<` filler runs in line 1 while the trailing filler
//! absorbs the loss, so the line stays the correct length and parses without
//! error while every field boundary shifts left
//! (`P<JPNSTRAND<<ALEKSANDER<<<…` → `PJPNSTRANDALEKSANDER<<<<…`). This module
//! catches that specific, reproducible corruption deterministically, using
//! data that already ships (`mrz::country_name`) rather than a model.
//!
//! **A candidate-selection fix was tried and measured worse, not better —
//! record kept so it isn't retried blind.** The obvious next step reads as:
//! teach `mrz::find_and_parse` to prefer a checksum-valid candidate whose
//! line 1 also passes this module's reasoning over one that doesn't, instead
//! of accepting the first checksum-valid candidate outright. Implemented and
//! measured on the 200-doc `synthpass-bench --profile all` corpus: it made
//! things worse — hit rate 69.5%→61.5%, and `given_names`/`surname` CER both
//! rose rather than fell. Root cause: continuing to search after the first
//! checksum-valid hit, hunting for one that also looks plausible, tries far
//! more line-1×line-2 combinations than before, and each extra combination
//! is one more chance of a coincidental false match against a checksum with
//! known blind spots (`mrz::Blindspot`) — that risk grew faster than the
//! benefit of occasionally finding a genuinely better line 1. Pairing it
//! with a change to `synthpass-ocr`'s MRZ-retry oracle (require plausibility,
//! not just checksum validity, before stopping retries) made it worse
//! still, by appending more noisy candidate lines to search through in the
//! first place. Both changes were reverted; this module stays a
//! reporting-only layer for now.
//!
//! What this is not: a posterior probability. See `Support`'s doc comment —
//! the ranking is ordinal on purpose.
//!
//! **`UnrecognizedNationality` and `NonAlphabeticName`** (added after the
//! above) were chosen by measuring candidate checks over ~150 specimens
//! (`crates/synthpass-ocr/examples/integrity_survey.rs`,
//! `knowledge/integrity-survey.jsonl`) before shipping them: both fired only on
//! records that an existing finding already flagged, never alone on a
//! checksum-valid, otherwise-`Accepted` document. A third candidate —
//! reconstructing the 39-char name field via `mrz::emit`'s canonicalization
//! and comparing against the raw line — was measured and **rejected**: it
//! false-positived on multiple genuine, checksum-valid specimens (e.g.
//! `Spain_Passport_Specimen.png`) because `mrz::parser::clean_name` is lossy
//! — it collapses any interior filler run of 2+ `<` to a single space via
//! `.trim()`, so a name with a wider-than-minimum filler gap can never be
//! byte-reconstructed from the parsed `surname`/`given_names` strings alone.
//! Same failure shape as a naive filler-count check would have: measure
//! before shipping.

use mrz::{MrzData, TransliterationStyle};
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;

/// Why a field's value is believed, ranked from strongest to weakest.
///
/// Deliberately an ordinal enum, not a float. A calibrated posterior needs a
/// measured likelihood function; SynthPass has never produced a calibration
/// curve (a reliability diagram from `synthpass-gen`'s labeled corpus would
/// be the way to earn one). Inventing likelihoods and combining them with
/// Bayes' rule would launder a guess into a number with decimal places —
/// exactly the failure `FieldConfidence::proven() == 1.0` on unverified line-1
/// fields already demonstrates. An ordinal scale says only "which claim is
/// stronger", which is what the data actually supports today.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Support {
    /// An ICAO check digit mathematically verifies this exact value.
    CheckDigit,
    /// Two values parsed from different MRZ lines by the same OCR pass agree.
    /// Weaker than a check digit — neither side is proven — but the two
    /// values are not derived from the same bytes, so agreement is a real,
    /// if modest, reduction in correlated risk.
    CrossField,
    /// Parsed at a fixed offset; nothing beyond charset and length checks it.
    Structural,
}

/// One integrity finding about a parsed MRZ record. `NeedsReview`'s `reasons`
/// are these, rendered.
///
/// Carries PII: `got`/`issuing_country`/`nationality` are copies of the same
/// ICAO country codes already stored (and zeroized) in `ExtractionFields` —
/// short, but real. `Zeroize`d field-by-field rather than skipped, matching
/// [`crate::v2::MrzBlock`]'s discipline for its own copy of the raw zone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Finding {
    /// `issuing_country` is not a recognized ICAO/ISO 3166-1 code — the
    /// clearest, cheapest signal of a shifted line 1.
    UnrecognizedIssuingCountry { got: String },
    /// `issuing_country` and `nationality` disagree, and `nationality` *is*
    /// a recognized code (so this isn't just two unrecognized strings talking
    /// past each other). `Support::CrossField` — see the doc comment above.
    IssuingCountryNationalityMismatch {
        issuing_country: String,
        nationality: String,
    },
    /// `given_names` is empty while `surname` is long — the signature of the
    /// collapsed-filler corruption: `parse_td3`'s `<<` split
    /// (`mrz::parser::clean_name`) never fired, so the whole name line landed
    /// in `surname`. `surname_len` is a length, not PII, but carries no
    /// `Zeroize` impl of its own (`usize` isn't `Copy`-zeroizable by derive
    /// without an explicit skip).
    MissingNameSeparator {
        #[zeroize(skip)]
        surname_len: usize,
    },
    /// `nationality` is not a recognized ICAO/ISO 3166-1 code. Separate from
    /// `IssuingCountryNationalityMismatch` — that only fires when
    /// `nationality` *is* recognized but disagrees with `issuing_country`.
    /// Worth checking on its own: the TD3 composite check digit excludes
    /// `nationality` entirely (see the module doc comment), so nothing else
    /// in the parser or the checksum math ever looks at this field.
    UnrecognizedNationality { got: String },
    /// A non-empty `surname` or `given_names` is 1-2 characters. Genuine
    /// ICAO name fields are essentially never a single stray letter; this is
    /// the signature of a name reduced to a fragment by OCR without
    /// triggering [`MissingNameSeparator`], which only fires when
    /// `given_names` is *empty*. Measured over ~200 real specimens
    /// (`crates/synthpass-ocr/examples/integrity_survey.rs`): 14 fires,
    /// zero false positives — every fire co-occurred with independent
    /// evidence of corruption (an unrecognized country/nationality code, a
    /// digit in the name, or a non-canonical name field), including two
    /// checksum-valid records that were previously `Accepted` outright.
    ///
    /// [`MissingNameSeparator`]: Self::MissingNameSeparator
    SuspiciouslyShortNameComponent {
        field: String,
        #[zeroize(skip)]
        len: usize,
    },
    /// 4 or more identical consecutive letters appear in `surname` or
    /// `given_names` — OCR garbage that survives
    /// `mrz::checksum::defiller`'s narrower K/L-run repair (a different
    /// repeated letter, or a run under its own ≥4-with-≥3-real-K/L
    /// threshold). Measured the same way as
    /// [`Finding::SuspiciouslyShortNameComponent`]: 5 fires, zero false positives.
    DegenerateRepeatedCharacterRun { field: String },
    /// A ASCII digit appears in `surname` or `given_names`. ICAO 9303 names
    /// are alphabetic by convention, but `parser::ensure_charset` accepts
    /// `0-9` across the whole line (it has to — line 2 is mostly digits), so
    /// nothing upstream of this module rejects a digit landing in a name
    /// field. Deliberately doesn't carry the matched character or the name
    /// itself: which field is enough to act on, and it keeps this variant
    /// off the highest-PII field in the record. `field` is always
    /// `"surname"` or `"given_names"` — not PII, but `String` (not
    /// `&'static str`) so `Finding` can keep deriving `Deserialize`.
    NonAlphabeticName { field: String },
    /// A Tier-2 (LLM) field disagrees, after normalization, with the MRZ's own
    /// structural read of the same field.
    ///
    /// **The MRZ side here is not checksum-verified**, and this doc used to
    /// claim it was. The only caller is
    /// `synthpass_pipeline::apply_deterministic_mrz`, which the pipeline
    /// reaches solely on the Tier-2 branch — i.e. only for records that
    /// *failed* the Tier-1 gate, so at least one check digit did not verify.
    /// Neither side of this comparison is proven; that is exactly why the
    /// finding is [`Support::CrossField`] and not something stronger, and why
    /// it means "two independent reads disagree, a human should look", never
    /// "the model is wrong".
    ///
    /// Covers only the six line-1 fields with **no** ICAO check digit —
    /// `document_type`, `issuing_country`, `surname`, `given_names`,
    /// `nationality`, `sex` — the checksummed fields (`document_number`,
    /// `date_of_birth`, `date_of_expiry`, `personal_number`) are promoted
    /// straight to `PROVEN` by `promote_verified_mrz_fields` and never reach
    /// this check. `field` is always one
    /// of the six names above; deliberately field-name-only, matching
    /// [`NonAlphabeticName`]'s discipline of never carrying the value that
    /// disagreed.
    ///
    /// [`NonAlphabeticName`]: Self::NonAlphabeticName
    LlmContradictsMrzStructural { field: String },
    /// The zone's own name field holds no filler at all: neither the `<<`
    /// between the primary and secondary identifiers nor a `<` between name
    /// components. Raised only when the unsplit name is too short for
    /// [`Finding::MissingNameSeparator`], so the same field is never reported
    /// twice. A holder with a single short name prints this way too, which is
    /// why it is report-only ([`FindingKind::is_report_only`]).
    ///
    /// Read from the zone's text, not the parsed names: line 3 of a TD1, and
    /// line 1 from cell 5 on for every two-line format. Carries no name text.
    RawNameSeparatorMissing,
    /// The zone's own name field holds a filler run that a printed name never
    /// holds: three or more `<` in a row, or a second `<<`. A printed name
    /// holds one `<<`, between the primary and secondary identifiers, and
    /// single `<` between components. The parsed names cannot show this,
    /// because `mrz` turns every filler into a space. Report-only, read from
    /// the same field as [`Finding::RawNameSeparatorMissing`], and carries no
    /// name text.
    RawInteriorFillerRun,
}

/// The fieldless projection of [`Finding`] — a stable, PII-free label for a
/// finding's *kind*.
///
/// [`Finding`] carries `got` / `issuing_country` / `nationality`: real ICAO
/// country codes read off a real document, which is why it derives `Zeroize`.
/// That makes it safe inside the extraction JSON (zeroized on drop) and unsafe
/// everywhere else. A routing decision made *because* of a finding gets logged
/// and turned into a metric label, so it needs a form with nothing in it —
/// this one.
///
/// `CONTRIBUTING.md`'s PII checklist states the rule this implements: logs may
/// carry shape, never content. `crates/synthpass-pipeline/tests/pii_logging.rs`
/// enforces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum FindingKind {
    UnrecognizedIssuingCountry,
    IssuingCountryNationalityMismatch,
    MissingNameSeparator,
    UnrecognizedNationality,
    SuspiciouslyShortNameComponent,
    DegenerateRepeatedCharacterRun,
    NonAlphabeticName,
    LlmContradictsMrzStructural,
    RawNameSeparatorMissing,
    RawInteriorFillerRun,
}

impl FindingKind {
    /// Whether this kind only reports what the zone's own text shows.
    ///
    /// A report-only finding is serialized with the verdict like any other,
    /// but it is not evidence that a parsed field is wrong: it lowers no
    /// confidence ([`crate::v2::FieldConfidence::downgrade_flagged`]) and does
    /// not feed the opt-in `Line1Flagged` routing clause in `synthpass-die`.
    pub fn is_report_only(self) -> bool {
        matches!(
            self,
            Self::RawNameSeparatorMissing | Self::RawInteriorFillerRun
        )
    }

    /// Stable snake_case label, identical to the serde representation. Safe as
    /// a metric label value: the set is closed and fixed at compile time.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnrecognizedIssuingCountry => "unrecognized_issuing_country",
            Self::IssuingCountryNationalityMismatch => "issuing_country_nationality_mismatch",
            Self::MissingNameSeparator => "missing_name_separator",
            Self::UnrecognizedNationality => "unrecognized_nationality",
            Self::SuspiciouslyShortNameComponent => "suspiciously_short_name_component",
            Self::DegenerateRepeatedCharacterRun => "degenerate_repeated_character_run",
            Self::NonAlphabeticName => "non_alphabetic_name",
            Self::LlmContradictsMrzStructural => "llm_contradicts_mrz_structural",
            Self::RawNameSeparatorMissing => "raw_name_separator_missing",
            Self::RawInteriorFillerRun => "raw_interior_filler_run",
        }
    }
}

impl std::fmt::Display for FindingKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Finding {
    /// Drop the payload, keep the classification.
    ///
    /// Deliberately exhaustive with no catch-all, mirroring
    /// [`crate::v2::FieldConfidence::downgrade_flagged`]: adding a [`Finding`]
    /// variant must be a compile error here rather than silently mapping to
    /// some default kind, because the result is what reaches logs and metrics.
    pub fn kind(&self) -> FindingKind {
        match self {
            Self::UnrecognizedIssuingCountry { .. } => FindingKind::UnrecognizedIssuingCountry,
            Self::IssuingCountryNationalityMismatch { .. } => {
                FindingKind::IssuingCountryNationalityMismatch
            }
            Self::MissingNameSeparator { .. } => FindingKind::MissingNameSeparator,
            Self::UnrecognizedNationality { .. } => FindingKind::UnrecognizedNationality,
            Self::SuspiciouslyShortNameComponent { .. } => {
                FindingKind::SuspiciouslyShortNameComponent
            }
            Self::DegenerateRepeatedCharacterRun { .. } => {
                FindingKind::DegenerateRepeatedCharacterRun
            }
            Self::NonAlphabeticName { .. } => FindingKind::NonAlphabeticName,
            Self::LlmContradictsMrzStructural { .. } => FindingKind::LlmContradictsMrzStructural,
            Self::RawNameSeparatorMissing => FindingKind::RawNameSeparatorMissing,
            Self::RawInteriorFillerRun => FindingKind::RawInteriorFillerRun,
        }
    }
}

/// Verdict for an [`MrzData`] record, from the checks in this module.
/// Distinct from [`mrz::MrzData::valid`] — that asks "do the check digits
/// verify", this asks "does the rest of the record look internally
/// consistent". A document can pass one and fail the other.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Zeroize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Verdict {
    /// No integrity findings.
    Accepted,
    /// At least one finding, none severe enough to reject outright.
    NeedsReview { reasons: Vec<Finding> },
}

impl Verdict {
    /// The kinds of every finding in this verdict, payloads stripped. Empty for
    /// [`Verdict::Accepted`]. This is the form safe to log or route on — see
    /// [`FindingKind`].
    pub fn kinds(&self) -> Vec<FindingKind> {
        match self {
            Self::Accepted => Vec::new(),
            Self::NeedsReview { reasons } => reasons.iter().map(Finding::kind).collect(),
        }
    }

    /// Whether any finding was raised. Cheaper and clearer at a call site than
    /// matching, when the caller only needs the boolean.
    pub fn is_flagged(&self) -> bool {
        matches!(self, Self::NeedsReview { .. })
    }
}

/// Threshold, in characters, above which an empty `given_names` next to a
/// long `surname` is flagged rather than treated as a plausible single-name
/// document (real single-name passports exist; ICAO allows it).
const SUSPICIOUSLY_LONG_UNSPLIT_NAME: usize = 12;

/// Runs the line-1 integrity checks over an already-parsed, checksum-passing
/// [`MrzData`] record. Callers should still gate on [`MrzData::valid`]
/// first — this module says nothing about line 2.
///
/// Thin wrapper over [`check_line1_integrity_excluding`] with an empty
/// occluded set — kept as a separate, unchanged-signature function because
/// `synthpass-pipeline` and `synthpass-bench` already call it with one
/// argument, and neither is in this PR's scope (#565 PR 3).
pub fn check_line1_integrity(m: &MrzData) -> Verdict {
    check_line1_integrity_excluding(m, &[])
}

/// [`check_line1_integrity`], but every check that would read a field named
/// in `occluded` is skipped instead of run.
///
/// **Why this exists (ADR-0026, decision 6).** [`mrz::apply_occlusion`]
/// blanks a covered, unverifiable field to an empty `String` rather than
/// guessing at it — exactly the shape this module already treats as "nothing
/// there". Run unmodified over a masked record, this module cannot tell a
/// covered `given_names` from a genuinely empty one: a long, visible
/// `surname` next to a masked `given_names` reads as
/// [`Finding::MissingNameSeparator`], not as "the given names are covered".
/// Naming the occluded fields up front — instead of filtering the resulting
/// [`Verdict`] afterwards — matters because a filter can only drop findings
/// that *are* about an occluded field; it cannot undo
/// [`Finding::MissingNameSeparator`] flagging a **visible, untouched**
/// `surname` collaterally, which is what would happen here if `given_names`
/// alone were occluded and the check still ran.
///
/// Every clause below is gated in the same shape and order as
/// [`check_line1_integrity`]'s own body, so `check_line1_integrity(m) ==
/// check_line1_integrity_excluding(m, &[])` by construction — there is
/// exactly one place this reasoning lives.
pub fn check_line1_integrity_excluding(m: &MrzData, occluded: &[crate::v2::CoreField]) -> Verdict {
    use crate::v2::CoreField;
    let occludes = |field: CoreField| occluded.contains(&field);
    let name_occluded = occludes(CoreField::Surname) || occludes(CoreField::GivenNames);

    let mut reasons = Vec::new();

    if !occludes(CoreField::IssuingCountry) {
        match mrz::country_name(&m.issuing_country) {
            None => reasons.push(Finding::UnrecognizedIssuingCountry {
                got: m.issuing_country.clone(),
            }),
            Some(_) => {
                if !occludes(CoreField::Nationality)
                    && mrz::country_name(&m.nationality).is_some()
                    && m.issuing_country != m.nationality
                {
                    reasons.push(Finding::IssuingCountryNationalityMismatch {
                        issuing_country: m.issuing_country.clone(),
                        nationality: m.nationality.clone(),
                    });
                }
            }
        }
    }

    if !name_occluded
        && m.given_names.is_empty()
        && m.surname.len() > SUSPICIOUSLY_LONG_UNSPLIT_NAME
    {
        reasons.push(Finding::MissingNameSeparator {
            surname_len: m.surname.len(),
        });
    }

    if !occludes(CoreField::Nationality) && mrz::country_name(&m.nationality).is_none() {
        reasons.push(Finding::UnrecognizedNationality {
            got: m.nationality.clone(),
        });
    }

    if !occludes(CoreField::Surname) && m.surname.chars().any(|c| c.is_ascii_digit()) {
        reasons.push(Finding::NonAlphabeticName {
            field: "surname".to_string(),
        });
    }
    if !occludes(CoreField::GivenNames) && m.given_names.chars().any(|c| c.is_ascii_digit()) {
        reasons.push(Finding::NonAlphabeticName {
            field: "given_names".to_string(),
        });
    }

    if !occludes(CoreField::Surname) && !m.surname.is_empty() && m.surname.chars().count() <= 2 {
        reasons.push(Finding::SuspiciouslyShortNameComponent {
            field: "surname".to_string(),
            len: m.surname.chars().count(),
        });
    }
    if !occludes(CoreField::GivenNames)
        && !m.given_names.is_empty()
        && m.given_names.chars().count() <= 2
    {
        reasons.push(Finding::SuspiciouslyShortNameComponent {
            field: "given_names".to_string(),
            len: m.given_names.chars().count(),
        });
    }

    if !occludes(CoreField::Surname) && has_repeated_letter_run(&m.surname) {
        reasons.push(Finding::DegenerateRepeatedCharacterRun {
            field: "surname".to_string(),
        });
    }
    if !occludes(CoreField::GivenNames) && has_repeated_letter_run(&m.given_names) {
        reasons.push(Finding::DegenerateRepeatedCharacterRun {
            field: "given_names".to_string(),
        });
    }

    // The zone's own name field, for what the parsed names cannot show (#576):
    // `mrz` splits a field with no `<<` at its first single `<` and turns every
    // filler into a space. Report-only (`FindingKind::is_report_only`), and
    // after the checks above so that a field `MissingNameSeparator` already
    // reports is not reported twice.
    if !name_occluded {
        if let Some(populated) = name_field(m).map(|field| field.trim_end_matches('<')) {
            let already_reported = reasons
                .iter()
                .any(|reason| matches!(reason, Finding::MissingNameSeparator { .. }));
            if !populated.is_empty() && !populated.contains('<') && !already_reported {
                reasons.push(Finding::RawNameSeparatorMissing);
            }
            if has_interior_filler_run(populated) {
                reasons.push(Finding::RawInteriorFillerRun);
            }
        }
    }

    if reasons.is_empty() {
        Verdict::Accepted
    } else {
        Verdict::NeedsReview { reasons }
    }
}

/// The zone's name field as the chosen lines hold it: line 3 of a TD1, and
/// line 1 from cell 5 on for every two-line format. `None` for a format with
/// no known name field, or a zone too short to hold one.
fn name_field(m: &MrzData) -> Option<&str> {
    match m.format {
        mrz::Format::Td1 => m.mrz_lines.lines().nth(2),
        mrz::Format::Td2 | mrz::Format::Td3 | mrz::Format::MrvA | mrz::Format::MrvB => {
            m.mrz_lines.lines().next().and_then(|line| line.get(5..))
        }
        _ => None,
    }
}

/// `true` if a name field, with its trailing filler already trimmed, holds a
/// filler run a printed name never holds: three or more `<` in a row, or more
/// than one `<<` — see [`Finding::RawInteriorFillerRun`].
fn has_interior_filler_run(populated: &str) -> bool {
    let mut doubles = 0usize;
    for run in populated.split(|c| c != '<').filter(|run| !run.is_empty()) {
        match run.len() {
            1 => {}
            2 => doubles += 1,
            _ => return true,
        }
    }
    doubles > 1
}

/// `true` if `s` contains a run of 4 or more identical consecutive ASCII
/// letters — see [`Finding::DegenerateRepeatedCharacterRun`].
fn has_repeated_letter_run(s: &str) -> bool {
    const MIN_RUN: usize = 4;
    let mut run = 0usize;
    let mut last: Option<u8> = None;
    for b in s.bytes() {
        if !b.is_ascii_alphabetic() {
            run = 0;
            last = None;
            continue;
        }
        run = if last == Some(b) { run + 1 } else { 1 };
        last = Some(b);
        if run >= MIN_RUN {
            return true;
        }
    }
    false
}

/// The three transliteration styles [`mrz::transliterate`] can produce for an
/// ambiguous Table A code point (see [`TransliterationStyle`]), tried in this
/// order when checking whether a Tier-2 name reproduces the MRZ's own
/// transliteration under *any* ICAO-sanctioned style choice.
const NAME_TRANSLITERATION_STYLES: [TransliterationStyle; 3] = [
    TransliterationStyle::Expanded,
    TransliterationStyle::Simple,
    TransliterationStyle::XxSuffix,
];

/// Whether a raw (possibly diacritic-bearing) Tier-2 name value could have
/// produced `mrz_value` under some ICAO-sanctioned transliteration style.
///
/// Checks all three styles rather than just the record's own, because nothing
/// in an `MrzData` says which style the issuing State chose (see the
/// module-level `translit` doc comment).
///
/// # Why this goes through `mrz::encode_name_component`
///
/// [`mrz::transliterate`] alone is not the MRZ name encoding, and its own doc
/// comment says so: it performs "no MRZ filler handling and no truncation".
/// Two consequences made this check fire on correct Tier-2 reads:
///
/// - **Apostrophes.** ICAO drops them with no filler, so `O'Brien` is printed
///   `OBRIEN`. Transliteration alone leaves the apostrophe in place, and
///   `O'BRIEN != OBRIEN`.
/// - **Truncation.** A name too long for the fixed-width field is cut to fit,
///   so the parsed `mrz_value` is a *prefix* of the full name, never equal to
///   it. Handled below by accepting a prefix match — but only against the
///   encoded form, so a prefix match cannot be claimed for two names that
///   merely start alike in their raw spelling.
///
/// Both sides are compared with interior whitespace collapsed:
/// `mrz::parser::clean_name` turns each `<` into one space without collapsing
/// runs, so a wider-than-minimum filler gap reaches this function as several
/// spaces where the encoder emits exactly one.
fn name_matches_any_transliteration(tier2_value: &str, mrz_value: &str) -> bool {
    let expected = collapse_spaces(mrz_value);
    if expected.is_empty() {
        return true;
    }
    NAME_TRANSLITERATION_STYLES.iter().any(|&style| {
        // Transliterate first (picks the style), then apply the ICAO name
        // encoding (filler, apostrophes, punctuation) to the result.
        let translit = mrz::transliterate(&tier2_value.to_uppercase(), style);
        let encoded = collapse_spaces(&mrz::encode_name_component(&translit).replace('<', " "));
        encoded == expected
            // Truncated into the fixed-width field: the MRZ holds a prefix.
            // Requires a real truncation (the MRZ side must be shorter), so
            // this never turns a genuinely different, longer MRZ name into a
            // match.
            || (expected.len() < encoded.len() && encoded.starts_with(&expected))
    })
}

/// Fold runs of whitespace to a single space and trim. Used on both sides of
/// the name comparison so filler-width differences can't masquerade as
/// disagreements — see [`name_matches_any_transliteration`].
fn collapse_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Whether a Tier-2 document type agrees with the MRZ's own document code.
///
/// Compared on the **first character only**, which is the part ICAO 9303
/// defines as the document class. The MRZ field is two characters wide and
/// its second position is issuer-discretionary until Doc 9303 Part 4 §4.4's table
/// becomes mandatory in 2028: `PO` (official/service), `PD`
/// (diplomatic), `PS` (stateless) are all passports, and most TD1/TD2 national
/// identity cards print `ID`. Canada and — effective 15 December 2025 —
/// Cyprus print an ordinary citizen passport as `PP` for the same reason
/// (confirmed against real specimens in `samples/passports/`; see
/// <https://www.gov.cy/moi/en/documents/passports/passport-specimens/>).
/// [`crate::normalize::document_type`] can only ever produce the single
/// letters `P`/`I`/`V`, so a whole-string comparison reported a contradiction
/// on every one of those documents — the model and the MRZ agreeing that it
/// is a passport was recorded as them disagreeing.
///
/// `I` and `ID` are the same class under this rule. A genuine disagreement
/// (`P` vs `ID`) still differs in the first character and is still flagged.
fn document_types_agree(tier2_value: &str, mrz_value: &str) -> bool {
    let first = |s: &str| s.chars().next().map(|c| c.to_ascii_uppercase());
    match (first(tier2_value), first(mrz_value)) {
        (Some(a), Some(b)) => a == b,
        // An empty Tier-2 read is not a contradiction; the MRZ side is
        // already guarded as non-empty at the call site.
        _ => true,
    }
}

/// Cross-checks the six **non-checksummed** MRZ line-1 fields —
/// `document_type`, `issuing_country`, `surname`, `given_names`,
/// `nationality`, `sex` — against a Tier-2 (LLM) record's own read of the
/// same document.
///
/// Sibling of [`check_line1_integrity`], deliberately not folded into it:
/// that function asks whether line 1 is internally consistent with itself
/// (and has other, unrelated callers this must not disturb); this asks
/// whether a *second, independent* read — the model's — agrees with the MRZ's
/// structural read of the same field.
///
/// **Neither side is proven.** The only caller reaches this on the Tier-2
/// branch alone, so the `MrzData` in hand has already failed at least one
/// check digit; its line 1 may itself be the garbled one. A finding here is a
/// disagreement to be reviewed, not a verdict against the model — see
/// [`Finding::LlmContradictsMrzStructural`].
///
/// The four
/// checksummed fields (`document_number`, `date_of_birth`, `date_of_expiry`,
/// `personal_number`) are out of scope here: they are promoted to `PROVEN`
/// directly by `synthpass_pipeline::promote_verified_mrz_fields` from the
/// ICAO check digit itself, a strictly stronger signal than anything this
/// function could add.
///
/// A field is compared only when the Tier-2 side has a usable value
/// ([`crate::v2::ExtractionFields::get`] already treats absent and
/// present-but-empty alike) — a missing Tier-2 read is not a contradiction —
/// and only when the MRZ side has a value, for the same reason: a non-empty
/// string, or for `sex` a product value from [`crate::mrz_product::sex`].
/// `document_type`/`issuing_country`/`nationality`/`sex` are normalized with
/// this crate's existing [`crate::normalize`] functions before comparing;
/// `surname`/`given_names` are compared permissively against every
/// transliteration style [`mrz::transliterate`] can produce (see
/// [`name_matches_any_transliteration`]), so a legitimate diacritic
/// transliteration ambiguity (`Müller` → `MULLER`, `MUELLER`, or `MUXXER`,
/// all ICAO-sanctioned) is never flagged as a contradiction.
pub fn check_tier2_against_mrz(fields: &crate::v2::ExtractionFields, m: &MrzData) -> Vec<Finding> {
    check_tier2_against_mrz_excluding(fields, m, &[])
}

/// [`check_tier2_against_mrz`], but the comparison for a field named in
/// `occluded` is skipped instead of run.
///
/// **Why this exists (ADR-0026, decision 6).** An occluded MRZ field is
/// withheld, not read — an occluded `surname` is blank in `m` for a reason
/// unrelated to whatever Tier 2 reported, so comparing the two and flagging
/// [`Finding::LlmContradictsMrzStructural`] would call a deliberate
/// withholding a contradiction. Each comparison below is independent per
/// field (unlike [`check_line1_integrity_excluding`]'s
/// [`Finding::MissingNameSeparator`], no comparison here can flag a
/// *different*, non-occluded field), so gating each `if let` individually is
/// sufficient and keeps `check_tier2_against_mrz(f, m) ==
/// check_tier2_against_mrz_excluding(f, m, &[])` by construction.
pub fn check_tier2_against_mrz_excluding(
    fields: &crate::v2::ExtractionFields,
    m: &MrzData,
    occluded: &[crate::v2::CoreField],
) -> Vec<Finding> {
    use crate::v2::CoreField;
    let occludes = |field: CoreField| occluded.contains(&field);

    let mut findings = Vec::new();

    if !occludes(CoreField::DocumentType) {
        if let Some(v) = fields.get(CoreField::DocumentType) {
            if !m.document_type.is_empty()
                && !document_types_agree(&crate::normalize::document_type(v), &m.document_type)
            {
                findings.push(Finding::LlmContradictsMrzStructural {
                    field: CoreField::DocumentType.as_str().to_string(),
                });
            }
        }
    }

    if !occludes(CoreField::IssuingCountry) {
        if let Some(v) = fields.get(CoreField::IssuingCountry) {
            if !m.issuing_country.is_empty()
                && !mrz::codes_equivalent(&crate::normalize::country_code(v), &m.issuing_country)
            {
                findings.push(Finding::LlmContradictsMrzStructural {
                    field: CoreField::IssuingCountry.as_str().to_string(),
                });
            }
        }
    }

    if !occludes(CoreField::Nationality) {
        if let Some(v) = fields.get(CoreField::Nationality) {
            if !m.nationality.is_empty()
                && !mrz::codes_equivalent(&crate::normalize::country_code(v), &m.nationality)
            {
                findings.push(Finding::LlmContradictsMrzStructural {
                    field: CoreField::Nationality.as_str().to_string(),
                });
            }
        }
    }

    if !occludes(CoreField::Sex) {
        if let Some(v) = fields.get(CoreField::Sex) {
            // Compare in the product vocabulary that Tier 2 normalizes into.
            if let Some(mrz_sex) = crate::mrz_product::sex(m.sex) {
                let normalized = crate::normalize::sex(v);
                if normalized != mrz_sex && !(m.sex == mrz::Sex::Unspecified && normalized == "<") {
                    findings.push(Finding::LlmContradictsMrzStructural {
                        field: CoreField::Sex.as_str().to_string(),
                    });
                }
            }
        }
    }

    if !occludes(CoreField::Surname) {
        if let Some(v) = fields.get(CoreField::Surname) {
            if !m.surname.is_empty() && !name_matches_any_transliteration(v, &m.surname) {
                findings.push(Finding::LlmContradictsMrzStructural {
                    field: CoreField::Surname.as_str().to_string(),
                });
            }
        }
    }

    if !occludes(CoreField::GivenNames) {
        if let Some(v) = fields.get(CoreField::GivenNames) {
            if !m.given_names.is_empty() && !name_matches_any_transliteration(v, &m.given_names) {
                findings.push(Finding::LlmContradictsMrzStructural {
                    field: CoreField::GivenNames.as_str().to_string(),
                });
            }
        }
    }

    findings
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> MrzData {
        // The canonical ICAO 9303 worked example, same fixture `mrz::dates`'
        // tests use.
        mrz::parse_td3(
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
        )
        .expect("fixture is a valid ICAO 9303 specimen")
    }

    #[test]
    fn a_clean_document_is_accepted() {
        assert_eq!(check_line1_integrity(&base()), Verdict::Accepted);
    }

    /// `line1` padded with filler to `width`, as a zone prints it.
    fn padded(line1: &str, width: usize) -> String {
        format!("{line1:<<width$}")
    }

    /// A TD3 record parsed from `line1` and the worked example's line 2, so the
    /// parsed fields and the zone text agree as `mrz` leaves them.
    fn td3(line1: &str) -> MrzData {
        mrz::parse_td3(
            &padded(line1, 44),
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
        )
        .expect("a 44-character line 1 parses")
    }

    /// A TD1 record from the ICAO 9303 worked example's first two lines and
    /// `line3`, the name line.
    fn td1(line3: &str) -> MrzData {
        mrz::parse_td1(
            "I<UTOD231458907<<<<<<<<<<<<<<<",
            "7408122F1204159UTO<<<<<<<<<<<6",
            &padded(line3, 30),
        )
        .expect("a 30-character line 3 parses")
    }

    /// #576 lists an issuer outside the registry and a digit in a name field
    /// among the line-1 anomalies to report. Both already are, by kinds that
    /// lower confidence: `issuing_country` is the zone's cells 2..5 with the
    /// trailing filler trimmed, and the parsed names keep every digit. So no
    /// report-only kind restates them.
    #[test]
    fn the_issuer_and_digit_anomalies_of_576_are_reported_by_the_existing_kinds() {
        for issuer in ["RCS", "<<<", "OOO", "DOR", "TRC"] {
            let kinds =
                check_line1_integrity(&td3(&format!("P<{issuer}ERIKSSON<<ANNA<MARIA"))).kinds();
            assert!(
                kinds.contains(&FindingKind::UnrecognizedIssuingCountry),
                "{issuer}: {kinds:?}"
            );
            assert!(
                !kinds.iter().any(|kind| kind.is_report_only()),
                "{issuer}: {kinds:?}"
            );
        }
        let digit = td3("P<UTOERIKSS0N<<ANNA<MARIA");
        assert_eq!(
            check_line1_integrity(&digit).kinds(),
            [FindingKind::NonAlphabeticName]
        );
    }

    #[test]
    fn a_name_field_with_no_filler_at_all_is_reported_once() {
        // A short unsplit name: no other check reports it.
        let short = td3("P<UTOERIKSSON");
        assert_eq!(
            check_line1_integrity(&short).kinds(),
            [FindingKind::RawNameSeparatorMissing]
        );

        // A long one is `MissingNameSeparator`'s already, so it is not reported
        // a second time.
        let kinds = check_line1_integrity(&td3("P<UTOERIKSSONKKANNAKMARIA")).kinds();
        assert!(
            kinds.contains(&FindingKind::MissingNameSeparator),
            "{kinds:?}"
        );
        assert!(
            !kinds.contains(&FindingKind::RawNameSeparatorMissing),
            "{kinds:?}"
        );

        // A single `<` is a filler: a `<<` read as `<`, or a zone that prints
        // no `<<` at all. The parsed split already handles it.
        for line1 in ["P<UTOERIKSSON<ANNA<MARIA", "P<UTOERIKSSON<ANNA"] {
            assert_eq!(
                check_line1_integrity(&td3(line1)),
                Verdict::Accepted,
                "{line1}"
            );
        }
    }

    #[test]
    fn a_filler_run_a_printed_name_never_holds_is_reported() {
        for line1 in [
            "P<UTOERIKSSON<<<ANNA<MARIA",
            "P<UTOERIKSSON<<ANNA<<MARIA",
            "P<UTOERIKSSON<ANNA<<<<MARIA",
        ] {
            let kinds = check_line1_integrity(&td3(line1)).kinds();
            assert!(
                kinds.contains(&FindingKind::RawInteriorFillerRun),
                "{line1}: {kinds:?}"
            );
        }
        // One `<<`, single `<` between components, and the trailing filler:
        // what a printed name holds.
        assert_eq!(check_line1_integrity(&base()), Verdict::Accepted);
    }

    /// A TD1 prints its names on line 3, so that is the field both checks read.
    #[test]
    fn td1_names_are_read_from_line_3() {
        assert_eq!(
            check_line1_integrity(&td1("ERIKSSON<<ANNA<MARIA")),
            Verdict::Accepted
        );
        assert_eq!(
            check_line1_integrity(&td1("ERIKSSON")).kinds(),
            [FindingKind::RawNameSeparatorMissing]
        );
        let kinds = check_line1_integrity(&td1("ERIKSSON<<ANNA<<MARIA")).kinds();
        assert!(
            kinds.contains(&FindingKind::RawInteriorFillerRun),
            "{kinds:?}"
        );
    }

    /// An occluded name is covered, not read, so neither check runs on it.
    #[test]
    fn an_occluded_name_skips_both_name_field_checks() {
        for field in [
            crate::v2::CoreField::Surname,
            crate::v2::CoreField::GivenNames,
        ] {
            for m in [td3("P<UTOERIKSSON"), td3("P<UTOERIKSSON<<<ANNA")] {
                let kinds = check_line1_integrity_excluding(&m, &[field]).kinds();
                assert!(
                    !kinds.iter().any(|kind| kind.is_report_only()),
                    "{field:?}: {kinds:?}"
                );
            }
        }
    }

    #[test]
    fn report_only_findings_serialize_as_their_kind_alone() {
        for (finding, label) in [
            (
                Finding::RawNameSeparatorMissing,
                "raw_name_separator_missing",
            ),
            (Finding::RawInteriorFillerRun, "raw_interior_filler_run"),
        ] {
            let json = serde_json::to_value(&finding).expect("serialize finding");
            assert_eq!(json["kind"], label);
            assert_eq!(json.as_object().expect("object").len(), 1, "no name text");
            assert!(finding.kind().is_report_only());
            assert_eq!(finding.kind().as_str(), label);
        }
    }

    #[test]
    fn an_unrecognized_issuing_country_is_flagged() {
        let mut m = base();
        "ZZZ".clone_into(&mut m.issuing_country);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::UnrecognizedIssuingCountry { got: "ZZZ".into() }]
            }
        );
    }

    #[test]
    fn issuing_country_disagreeing_with_a_valid_nationality_is_flagged() {
        let mut m = base();
        // UTO (Utopia) is a real specimen code, distinct from the fixture's
        // own UTO nationality, so this is a genuine mismatch, not a typo.
        "HRV".clone_into(&mut m.issuing_country);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::IssuingCountryNationalityMismatch {
                    issuing_country: "HRV".into(),
                    nationality: "UTO".into(),
                }]
            }
        );
    }

    /// Pinned to the actual corpus corruption
    /// (`P<JPNSTRAND<<ALEKSANDER<<<…` -> `PJPNSTRANDALEKSANDER<<<<…`): the
    /// `<<` separator is gone, so `clean_name` puts the whole line into
    /// `surname` and leaves `given_names` empty.
    #[test]
    fn the_collapsed_filler_run_corruption_is_flagged() {
        let mut m = base();
        "TRANDALEKSANDER".clone_into(&mut m.surname);
        String::new().clone_into(&mut m.given_names);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::MissingNameSeparator { surname_len: 15 }]
            }
        );
    }

    #[test]
    fn a_short_single_name_with_no_given_names_is_not_flagged() {
        // Real ICAO documents can legitimately have no given names (mononyms).
        // Only a *long* unsplit name is suspicious.
        let mut m = base();
        "CHER".clone_into(&mut m.surname);
        String::new().clone_into(&mut m.given_names);
        assert_eq!(check_line1_integrity(&m), Verdict::Accepted);
    }

    #[test]
    fn an_unrecognized_nationality_is_flagged() {
        let mut m = base();
        "ZZZ".clone_into(&mut m.nationality);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::UnrecognizedNationality { got: "ZZZ".into() }]
            }
        );
    }

    #[test]
    fn a_digit_in_surname_is_flagged() {
        let mut m = base();
        "ER1KSSON".clone_into(&mut m.surname);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::NonAlphabeticName {
                    field: "surname".to_string()
                }]
            }
        );
    }

    #[test]
    fn a_digit_in_given_names_is_flagged() {
        let mut m = base();
        "ANNA MAR1A".clone_into(&mut m.given_names);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::NonAlphabeticName {
                    field: "given_names".to_string()
                }]
            }
        );
    }

    #[test]
    fn a_one_character_surname_is_flagged() {
        let mut m = base();
        "E".clone_into(&mut m.surname);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::SuspiciouslyShortNameComponent {
                    field: "surname".to_string(),
                    len: 1,
                }]
            }
        );
    }

    #[test]
    fn a_two_character_given_names_is_flagged() {
        let mut m = base();
        "AB".clone_into(&mut m.given_names);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::SuspiciouslyShortNameComponent {
                    field: "given_names".to_string(),
                    len: 2,
                }]
            }
        );
    }

    #[test]
    fn a_three_character_name_component_is_not_flagged() {
        // The threshold is deliberately tight (<=2): a real short surname
        // (e.g. many East Asian romanizations) is more common at 3+
        // characters than a genuine 1-2 character MRZ name field.
        let mut m = base();
        "OTT".clone_into(&mut m.given_names);
        assert_eq!(check_line1_integrity(&m), Verdict::Accepted);
    }

    #[test]
    fn a_repeated_letter_run_in_surname_is_flagged() {
        let mut m = base();
        "ERIKKKKSON".clone_into(&mut m.surname); // four consecutive 'K's
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::DegenerateRepeatedCharacterRun {
                    field: "surname".to_string(),
                }]
            }
        );
    }

    #[test]
    fn a_short_run_of_repeated_letters_is_not_flagged() {
        // Three, not four: real names do carry doubled letters
        // ("MISSISSIPPI"-style), so the threshold must not fire below it.
        let mut m = base();
        "ERIKKKSON".clone_into(&mut m.surname);
        assert_eq!(check_line1_integrity(&m), Verdict::Accepted);
    }

    /// The failure mode a naive filler-count/round-trip check would have hit
    /// (see the module doc comment): a wider-than-minimum internal gap
    /// between given names (parsed as extra spaces, since `clean_name`
    /// converts every interior `<` in the raw MRZ to one space each) must
    /// never be flagged by anything in this module.
    #[test]
    fn a_document_with_a_wide_internal_name_gap_is_not_flagged() {
        let mut m = base();
        "ANNA   MARIA".clone_into(&mut m.given_names);
        assert_eq!(check_line1_integrity(&m), Verdict::Accepted);
    }

    #[test]
    fn multiple_findings_are_all_reported() {
        let mut m = base();
        "ZZZ".clone_into(&mut m.issuing_country);
        "TRANDALEKSANDER".clone_into(&mut m.surname);
        String::new().clone_into(&mut m.given_names);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![
                    Finding::UnrecognizedIssuingCountry { got: "ZZZ".into() },
                    Finding::MissingNameSeparator { surname_len: 15 },
                ]
            }
        );
    }

    // ── check_tier2_against_mrz ──

    /// `base()`'s line 1, restated as the [`crate::v2::ExtractionFields`] a
    /// perfectly agreeing Tier-2 (LLM) read would have produced: same
    /// document type, country, names, nationality, and sex, already in MRZ
    /// convention.
    fn agreeing_fields() -> crate::v2::ExtractionFields {
        crate::v2::ExtractionFields {
            document_type: Some("P".into()),
            issuing_country: Some("UTO".into()),
            surname: Some("ERIKSSON".into()),
            given_names: Some("ANNA MARIA".into()),
            nationality: Some("UTO".into()),
            sex: Some("F".into()),
            ..Default::default()
        }
    }

    #[test]
    fn a_sex_mismatch_is_flagged() {
        let m = base();
        let mut fields = agreeing_fields();
        fields.sex = Some("M".into());
        assert_eq!(
            check_tier2_against_mrz(&fields, &m),
            vec![Finding::LlmContradictsMrzStructural {
                field: "sex".to_string()
            }]
        );
    }

    #[test]
    fn nonconformant_mrz_sex_does_not_contradict_llm() {
        let mut m = base();
        m.sex = mrz::Sex::NonConformant('1');
        for value in ["F", "M", "X", "<"] {
            let mut fields = agreeing_fields();
            fields.sex = Some(value.into());
            assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new(), "{value}");
        }
    }

    #[test]
    fn unspecified_mrz_sex_agrees_with_x_and_filler() {
        let mut m = base();
        m.sex = mrz::Sex::Unspecified;
        for value in ["X", "<"] {
            let mut fields = agreeing_fields();
            fields.sex = Some(value.into());
            assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new(), "{value}");
        }
        let mut fields = agreeing_fields();
        fields.sex = Some("M".into());
        assert_eq!(
            check_tier2_against_mrz(&fields, &m),
            vec![Finding::LlmContradictsMrzStructural {
                field: "sex".to_string()
            }]
        );
    }

    #[test]
    fn a_fully_agreeing_record_has_no_findings() {
        let m = base();
        assert_eq!(check_tier2_against_mrz(&agreeing_fields(), &m), Vec::new());
    }

    #[test]
    fn a_long_form_sex_value_is_not_flagged() {
        let m = base();
        let mut fields = agreeing_fields();
        fields.sex = Some("Female".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_diacritic_given_name_matching_any_transliteration_style_is_not_flagged() {
        let mut m = base();
        "TERESA".clone_into(&mut m.given_names);
        let mut fields = agreeing_fields();
        fields.given_names = Some("Térèsa".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_diacritic_surname_matching_the_expanded_style_is_not_flagged() {
        let mut m = base();
        "MUELLER".clone_into(&mut m.surname);
        let mut fields = agreeing_fields();
        fields.surname = Some("Müller".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_diacritic_surname_matching_the_simple_style_is_not_flagged() {
        let mut m = base();
        "MULLER".clone_into(&mut m.surname);
        let mut fields = agreeing_fields();
        fields.surname = Some("Müller".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn an_unnormalized_country_name_matching_the_mrz_code_is_not_flagged() {
        let mut m = base();
        "HRV".clone_into(&mut m.issuing_country);
        "HRV".clone_into(&mut m.nationality);
        let mut fields = agreeing_fields();
        fields.issuing_country = Some("CROATIA".into());
        fields.nationality = Some("CROATIA".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_genuinely_different_country_is_flagged() {
        let m = base(); // issuing_country/nationality: UTO
        let mut fields = agreeing_fields();
        fields.issuing_country = Some("FRANCE".into());
        assert_eq!(
            check_tier2_against_mrz(&fields, &m),
            vec![Finding::LlmContradictsMrzStructural {
                field: "issuing_country".to_string()
            }]
        );
    }

    // ── demonyms (normalize::DEMONYMS, added alongside country_code) ──
    //
    // This check runs `normalize::country_code` on the Tier-2 value before
    // comparing to MRZ ground truth (see `check_tier2_against_mrz` above), so
    // it silently started resolving demonyms the moment that table shipped —
    // with no test here exercising the interaction until these two.

    #[test]
    fn a_demonym_matching_the_mrz_code_is_not_flagged() {
        // Before `DEMONYMS` existed, "Canadian" resolved to nothing and
        // compared unequal to "CAN" — a false positive on every Canadian
        // document that reached Tier 2, the same shape as the Croatia/Germany
        // cases above. This is the now-correct behavior, pinned.
        let mut m = base();
        "CAN".clone_into(&mut m.issuing_country);
        "CAN".clone_into(&mut m.nationality);
        for value in ["CANADIAN", "CANADIENNE", "Canadian"] {
            let mut fields = agreeing_fields();
            fields.issuing_country = Some(value.into());
            fields.nationality = Some(value.into());
            assert_eq!(
                check_tier2_against_mrz(&fields, &m),
                Vec::new(),
                "value: {value:?}"
            );
        }
    }

    #[test]
    fn a_demonym_naming_a_different_country_is_still_flagged() {
        // Resolving a value through `country_code` must never quietly
        // suppress a genuine contradiction — a demonym is just another form
        // of the same field, not an exemption from this check.
        let m = base(); // issuing_country/nationality: UTO
        let mut fields = agreeing_fields();
        fields.nationality = Some("Serbian".into());
        assert_eq!(
            check_tier2_against_mrz(&fields, &m),
            vec![Finding::LlmContradictsMrzStructural {
                field: "nationality".to_string()
            }]
        );
    }

    // ── the four systematic false positives this check used to produce ──

    #[test]
    fn the_german_legacy_country_code_is_not_a_contradiction() {
        // German documents print the legacy single-letter `D` in the MRZ,
        // while `normalize::country_code` resolves "Germany"/"DEU" to the
        // primary code `DEU`. Comparing those as plain strings flagged — and
        // downgraded — both country fields on every German document that
        // reached Tier 2.
        let mut m = base();
        "D".clone_into(&mut m.issuing_country);
        "D".clone_into(&mut m.nationality);
        for value in ["Germany", "DEU", "D"] {
            let mut fields = agreeing_fields();
            fields.issuing_country = Some(value.into());
            fields.nationality = Some(value.into());
            assert_eq!(
                check_tier2_against_mrz(&fields, &m),
                Vec::new(),
                "{value} should agree with the MRZ's `D`"
            );
        }
    }

    #[test]
    fn a_two_character_document_code_agrees_on_its_class() {
        // `PO` (official), `PD` (diplomatic), `PS` (service) are passports;
        // `ID` is an identity card. `normalize::document_type` only ever
        // yields `P`/`I`/`V`, so a whole-string compare could never match any
        // of them.
        for (mrz_code, tier2_value) in [
            ("PO", "P"),
            ("PD", "PASSPORT"),
            ("PS", "P"),
            ("ID", "IDENTITY CARD"),
            ("ID", "I"),
        ] {
            let mut m = base();
            mrz_code.clone_into(&mut m.document_type);
            let mut fields = agreeing_fields();
            fields.document_type = Some(tier2_value.into());
            assert_eq!(
                check_tier2_against_mrz(&fields, &m),
                Vec::new(),
                "{tier2_value} should agree with the MRZ's {mrz_code}"
            );
        }
    }

    #[test]
    fn a_genuinely_different_document_class_is_still_flagged() {
        let mut m = base();
        "ID".clone_into(&mut m.document_type);
        let mut fields = agreeing_fields();
        fields.document_type = Some("PASSPORT".into());
        assert_eq!(
            check_tier2_against_mrz(&fields, &m),
            vec![Finding::LlmContradictsMrzStructural {
                field: "document_type".to_string()
            }]
        );
    }

    #[test]
    fn an_apostrophe_dropped_by_icao_name_encoding_is_not_a_contradiction() {
        // ICAO 9303 §4.6 drops apostrophes with no filler, so `O'Brien` is
        // printed `OBRIEN`. `mrz::transliterate` alone performs no filler
        // handling, so the old comparison left the apostrophe in and flagged.
        let mut m = base();
        "OBRIEN".clone_into(&mut m.surname);
        let mut fields = agreeing_fields();
        fields.surname = Some("O'Brien".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_hyphenated_name_encoded_with_filler_is_not_a_contradiction() {
        // A hyphen becomes `<`, which `parser::clean_name` reads back as a
        // space.
        let mut m = base();
        "SMITH JONES".clone_into(&mut m.surname);
        let mut fields = agreeing_fields();
        fields.surname = Some("Smith-Jones".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_wider_than_minimum_filler_gap_is_not_a_contradiction() {
        // `clean_name` turns each `<` into one space without collapsing runs,
        // so a wide filler gap arrives here as several spaces where the
        // encoder emits exactly one.
        let mut m = base();
        "ANNA   MARIA".clone_into(&mut m.given_names);
        let mut fields = agreeing_fields();
        fields.given_names = Some("Anna Maria".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_name_truncated_into_the_fixed_width_field_is_not_a_contradiction() {
        // The MRZ name field is fixed-width; a long name is cut to fit, so the
        // parsed value is a prefix of the full name and can never equal it.
        let mut m = base();
        "BARTHOLOMEW MAXIMILI".clone_into(&mut m.given_names);
        let mut fields = agreeing_fields();
        fields.given_names = Some("Bartholomew Maximilian".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_shorter_mrz_name_that_is_not_a_prefix_is_still_flagged() {
        // Truncation tolerance must not become "any two names that differ in
        // length agree".
        let mut m = base();
        "NIELSEN".clone_into(&mut m.surname);
        let mut fields = agreeing_fields();
        fields.surname = Some("Eriksson".into());
        assert_eq!(
            check_tier2_against_mrz(&fields, &m),
            vec![Finding::LlmContradictsMrzStructural {
                field: "surname".to_string()
            }]
        );
    }

    #[test]
    fn a_longer_mrz_name_is_never_absorbed_by_the_prefix_rule() {
        // The prefix rule only fires when the MRZ side is *shorter* — the
        // direction a truncation can actually produce.
        let mut m = base();
        "ERIKSSONSDOTTIR".clone_into(&mut m.surname);
        let mut fields = agreeing_fields();
        fields.surname = Some("Eriksson".into());
        assert_eq!(
            check_tier2_against_mrz(&fields, &m),
            vec![Finding::LlmContradictsMrzStructural {
                field: "surname".to_string()
            }]
        );
    }

    #[test]
    fn an_absent_tier2_field_is_skipped_not_flagged() {
        let mut m = base();
        // MRZ sex disagrees with nothing, since Tier-2's own value is absent.
        m.sex = mrz::Sex::Male;
        let mut fields = agreeing_fields();
        fields.sex = None;
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn an_empty_tier2_field_is_skipped_not_flagged() {
        let mut m = base();
        m.sex = mrz::Sex::Male;
        let mut fields = agreeing_fields();
        fields.sex = Some("   ".into());
        assert_eq!(check_tier2_against_mrz(&fields, &m), Vec::new());
    }

    #[test]
    fn a_finding_kind_label_round_trips() {
        let finding = Finding::LlmContradictsMrzStructural {
            field: "sex".to_string(),
        };
        assert_eq!(finding.kind(), FindingKind::LlmContradictsMrzStructural);
        assert_eq!(finding.kind().as_str(), "llm_contradicts_mrz_structural");
    }

    // ── ADR-0026: occluded fields are skipped, not flagged ──

    #[test]
    fn missing_name_separator_is_suppressed_when_given_names_is_occluded() {
        use crate::v2::CoreField;
        // The exact corpus corruption shape from `the_collapsed_filler_run_
        // corruption_is_flagged` above, restated as what `mrz::apply_occlusion`
        // leaves behind when `given_names` (not `surname`) is the covered
        // field: a visible, untouched surname next to a blanked given_names.
        let mut m = base();
        "TRANDALEKSANDER".clone_into(&mut m.surname);
        String::new().clone_into(&mut m.given_names);
        assert_eq!(
            check_line1_integrity(&m),
            Verdict::NeedsReview {
                reasons: vec![Finding::MissingNameSeparator { surname_len: 15 }]
            },
            "sanity: unmasked, this still fires"
        );
        assert_eq!(
            check_line1_integrity_excluding(&m, &[CoreField::GivenNames]),
            Verdict::Accepted,
            "given_names occluded: the finding must not fire — and it must not \
             fire at all, not just get filtered after the fact, or the visible \
             surname would be flagged as a side effect"
        );
    }

    #[test]
    fn given_names_occlusion_does_not_collaterally_downgrade_the_visible_surname() {
        use crate::v2::{CoreField, FieldConfidence};
        let mut m = base();
        "TRANDALEKSANDER".clone_into(&mut m.surname);
        String::new().clone_into(&mut m.given_names);
        let verdict = check_line1_integrity_excluding(&m, &[CoreField::GivenNames]);
        let mut confidence = FieldConfidence::mrz_checksum_scope();
        confidence.downgrade_flagged(&verdict);
        assert_eq!(
            confidence.surname,
            FieldConfidence::mrz_checksum_scope().surname,
            "a finding that never fired must not downgrade a field it would \
             have named — this is the case a post-hoc filter cannot fix"
        );
    }

    #[test]
    fn suspiciously_short_name_component_is_suppressed_when_occluded() {
        use crate::v2::CoreField;
        let mut m = base();
        "E".clone_into(&mut m.surname);
        assert!(check_line1_integrity(&m).is_flagged(), "sanity");
        assert_eq!(
            check_line1_integrity_excluding(&m, &[CoreField::Surname]),
            Verdict::Accepted
        );
    }

    #[test]
    fn unrecognized_nationality_is_suppressed_when_occluded() {
        use crate::v2::CoreField;
        let mut m = base();
        "ZZZ".clone_into(&mut m.nationality);
        assert!(check_line1_integrity(&m).is_flagged(), "sanity");
        assert_eq!(
            check_line1_integrity_excluding(&m, &[CoreField::Nationality]),
            Verdict::Accepted
        );
    }

    #[test]
    fn an_occluded_field_with_nothing_else_wrong_stays_accepted() {
        use crate::v2::CoreField;
        // Empty is exactly what `mrz::apply_occlusion` leaves an occluded
        // field at, and a document with nothing else wrong must read clean.
        let mut m = base();
        String::new().clone_into(&mut m.surname);
        String::new().clone_into(&mut m.given_names);
        assert_eq!(
            check_line1_integrity_excluding(&m, &[CoreField::Surname, CoreField::GivenNames]),
            Verdict::Accepted
        );
    }

    #[test]
    fn tier2_sex_contradiction_is_suppressed_when_occluded() {
        use crate::v2::CoreField;
        let mut m = base();
        // What `mrz::apply_occlusion` leaves a covered sex cell at.
        m.sex = mrz::Sex::Unspecified;
        let mut fields = agreeing_fields();
        fields.sex = Some("M".into());
        assert_eq!(
            check_tier2_against_mrz(&fields, &m),
            vec![Finding::LlmContradictsMrzStructural {
                field: "sex".to_string()
            }],
            "sanity: unmasked, this still fires (see \
             unspecified_mrz_sex_agrees_with_x_and_filler above)"
        );
        assert_eq!(
            check_tier2_against_mrz_excluding(&fields, &m, &[CoreField::Sex]),
            Vec::new(),
            "sex occluded: the withheld MRZ value must not be treated as a \
             contradiction of Tier 2's own read"
        );
    }
}
