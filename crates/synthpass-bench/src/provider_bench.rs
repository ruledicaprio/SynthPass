//! M7 multi-provider benchmark: runs every registered [`FieldReader`](synthpass_die::FieldReader) in a
//! [`ProviderCatalog`] against the same corpus and reports, per provider,
//! accuracy, speed, JSON validity, an unsupported-assertion rate, and
//! resident memory.
//!
//! Deliberately bypasses `ProviderCatalog::find_reader`'s first-match router
//! — every reader sees every document, not just the one the pipeline's
//! routing policy would have picked, because the whole point is comparing
//! providers against each other, not reproducing the router's decision.
//!
//! Two corpus sources feed the same reader loop: [`run_provider_bench`] over
//! `synthpass-gen`'s synthetic corpus (ground truth always present — every
//! `CorpusDoc` carries a generated, checksum-valid MRZ by construction), and
//! [`run_provider_bench_real`] over real specimens in `samples/` (ground
//! truth present only when a `samples/ocr_fixtures/<stem>.json` label file
//! exists). This module's original form only had the first: `CorpusDoc`
//! required a generated `Labels`, and ground truth was derived by parsing
//! its MRZ lines, so a document with no MRZ — every real MRZ-less ID-card
//! front — had no path through this harness at all. That gap is exactly why
//! a serious Tier-2 failure mode (the LLM inventing entire identities on
//! MRZ-less fronts) was found by hand instead of by CI.
//!
//! Both entry points funnel into [`run_prepped_with_dump_options`], the one place accuracy and
//! unsupported-assertion are computed, so the two corpus sources cannot
//! silently diverge in what "correct" means.
//!
//! A third source is a **replay** ([`run_provider_bench_replay`], ADR-0024
//! amendment 3): the public corpus's pages are rebuilt from a captured
//! `provider-bench-ocr-passes.jsonl` instead of by OCR, and then run through the
//! same [`run_prepped_with_dump_options`]. Tier 1 is a pure function of the page text, so a replay
//! measures everything downstream of `OcrPage::text` and nothing upstream.

use crate::archive::{Archive, ArchiveTrack, DocRecord, OcrRecord, Tier1ReadRecord};
use crate::{classify_names, miss_kind, CorpusDoc, MissReason, NameError, RealSpecimenDoc};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use synthpass_core::v2::{CoreField, ExtractionFields, MrzFormat};
use synthpass_die::{Capability, CostClass, DocumentContext, Evidence, ProviderCatalog};
use synthpass_ocr::{NativeOcr, OcrPage};

/// Stable label for a resolved ICAO 9303 MRZ format in a report — mirrors
/// `synthpass_gen::DocumentType::as_str`'s `"TD1"`/`"TD2"`/`"TD3"` naming for
/// the three the generator can produce, plus the two real-specimen-only
/// outcomes (`synthpass-gen` never emits an MRV-A/MRV-B).
fn mrz_format_str(format: MrzFormat) -> &'static str {
    match format {
        MrzFormat::Td1 => "TD1",
        MrzFormat::Td2 => "TD2",
        MrzFormat::Td3 => "TD3",
        MrzFormat::MrvA => "MRVA",
        MrzFormat::MrvB => "MRVB",
    }
}

/// Resolves [`DocumentDetail::mrz_format`] for one document/reading pair —
/// extracted to a named function, rather than left as the closure it started
/// as, so it is unit-testable without a full `run_prepped` pass.
///
/// - Synthetic corpus: always `known_or_guessed_format`, the label's exact
///   format, independent of `evidence` — the ground truth is generated, not
///   read, so there is nothing for a read to override.
/// - Real specimens: `evidence.mrz_format` wins, but **only when that read's
///   checksums are valid** (`Evidence::mrz_checksums_valid`) — a checksum-
///   invalid read is exactly as apt to guess the wrong MRZ format from noise
///   as it is to misread any other field, so it earns no more trust here than
///   anywhere else. Otherwise (no evidence, no resolved format, or a
///   checksum-invalid read) falls back to `known_or_guessed_format`, the
///   ground-truth guess from `MrzFormat::guess_from_lines`, or `None` when
///   neither resolved a format — a specimen with no MRZ is not a
///   TD-anything. See #555.
fn resolve_mrz_format(
    synthetic: bool,
    known_or_guessed_format: Option<&'static str>,
    evidence: Option<&Evidence>,
) -> Option<&'static str> {
    if synthetic {
        known_or_guessed_format
    } else {
        evidence
            .filter(|e| e.mrz_checksums_valid)
            .and_then(|e| e.mrz_format)
            .map(mrz_format_str)
            .or(known_or_guessed_format)
    }
}

/// Characters where the run's recovered MRZ zone differs from the
/// hand-transcribed true zone, compared line by line over the longer of the
/// two (a missing character counts as a mismatch). `0` means the OCR/parse
/// pipeline recovered the printed zone faithfully — a checksum failure on such
/// a zone is the *specimen's* printed check digits, not an OCR error. See the
/// 2026-09-08 checksum_failed writeup.
fn compared_cells(recovered: &str, truth: &str) -> usize {
    recovered
        .lines()
        .zip(truth.lines())
        .map(|(a, b)| a.chars().zip(b.chars()).count())
        .sum()
}

fn mrz_zone_mismatch(recovered: &str, truth: &str) -> usize {
    let rec: Vec<&str> = recovered.lines().collect();
    let tru: Vec<&str> = truth.lines().collect();
    (0..rec.len().max(tru.len()))
        .map(|i| {
            let mut a = rec.get(i).copied().unwrap_or("").chars();
            let mut b = tru.get(i).copied().unwrap_or("").chars();
            std::iter::from_fn(|| match (a.next(), b.next()) {
                (None, None) => None,
                (x, y) => Some(x != y),
            })
            .filter(|&differs| differs)
            .count()
        })
        .sum()
}

/// Checksum-coverage state of one MRZ field: whether an error inside it
/// would be caught by a check digit, and by which one.
///
/// Two things false about a document that reads as a clean Tier-1 hit
/// motivate this: nationality and sex sit inside *no* ICAO composite on any
/// format, and a field like TD1's optional data has no check digit of its
/// own — the composite is the only thing standing over it. Whether a
/// differing field is checksum-covered is as important as the fact that it
/// differs; see this module's top-of-file note and the 2026-09-18
/// hand-analysis this instrumentation replaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CheckCoverage {
    /// A dedicated ICAO check digit protects this field on its own —
    /// document number, date of birth, date of expiry, or (TD3 only)
    /// personal number — or this field *is* that check digit's own
    /// character position (`<field>_cd`).
    OwnCheckDigit,
    /// No dedicated check digit exists for this field, but it lies inside
    /// the composite check's protected span, or it *is* the composite check
    /// digit's own position (`composite_cd`). An error here fails the
    /// composite check and nothing else — the "composite check failed and
    /// nothing else did" signature localises the error to one of these
    /// fields before a single character is examined.
    CompositeOnly,
    /// No check digit — dedicated or composite — covers this field at all.
    /// A document can validate every check digit it prints while a field in
    /// this state is silently wrong.
    #[serde(rename = "none")]
    Uncovered,
}

/// One named, positionally-bounded field inside an MRZ zone.
///
/// `line` is 1-based and matches `mrz::parser`'s own `line1`/`line2`/`line3`
/// naming (TD1 has three physical lines; every other format has two), so a
/// field-layout table reads the same way as the parser code it must not
/// drift from.
#[derive(Debug, Clone, Copy)]
struct MrzFieldSpan {
    line: usize,
    /// Used verbatim as a JSON object key in the dump's per-field mismatch
    /// maps — deliberately `snake_case` and stable. `<field>_cd` names a
    /// field's own ICAO check-digit character as a field distinct from the
    /// data it protects, so a check-digit-only or composite-only error is
    /// distinguishable from a data error in the same field.
    name: &'static str,
    /// Start column, 0-based, inclusive.
    start: usize,
    /// End column, 0-based, exclusive.
    end: usize,
    coverage: CheckCoverage,
}

use CheckCoverage::{CompositeOnly, OwnCheckDigit, Uncovered};

/// TD1 (ICAO 9303 part 5): three 30-character lines. Composite spans
/// `line1[5..30] + line2[0..7] + line2[8..15] + line2[18..29]`
/// (`crates/mrz/src/parser.rs`'s `parse_td1_with`, composite `verify` call) —
/// pinned against that function's actual behaviour by
/// `field_layout_matches_the_parsers_own_check_digit_spans` below, one
/// mutated character at a time, rather than duplicated here as a second copy
/// of the offsets that could silently drift from it.
const TD1_FIELDS: &[MrzFieldSpan] = &[
    MrzFieldSpan {
        line: 1,
        name: "document_code",
        start: 0,
        end: 2,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "issuing_country",
        start: 2,
        end: 5,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "document_number",
        start: 5,
        end: 14,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 1,
        name: "document_number_cd",
        start: 14,
        end: 15,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 1,
        name: "optional_data_1",
        start: 15,
        end: 30,
        coverage: CompositeOnly,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth",
        start: 0,
        end: 6,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth_cd",
        start: 6,
        end: 7,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "sex",
        start: 7,
        end: 8,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry",
        start: 8,
        end: 14,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry_cd",
        start: 14,
        end: 15,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "nationality",
        start: 15,
        end: 18,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "optional_data_2",
        start: 18,
        end: 29,
        coverage: CompositeOnly,
    },
    MrzFieldSpan {
        line: 2,
        name: "composite_cd",
        start: 29,
        end: 30,
        coverage: CompositeOnly,
    },
    MrzFieldSpan {
        line: 3,
        name: "name",
        start: 0,
        end: 30,
        coverage: Uncovered,
    },
];

/// TD2 (ICAO 9303 part 6): two 36-character lines. Composite spans
/// `line2[0..10] + line2[13..20] + line2[21..35]` (`parse_td2_with`) — see
/// [`TD1_FIELDS`]'s doc for the pinning approach.
const TD2_FIELDS: &[MrzFieldSpan] = &[
    MrzFieldSpan {
        line: 1,
        name: "document_code",
        start: 0,
        end: 2,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "issuing_country",
        start: 2,
        end: 5,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "name",
        start: 5,
        end: 36,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "document_number",
        start: 0,
        end: 9,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "document_number_cd",
        start: 9,
        end: 10,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "nationality",
        start: 10,
        end: 13,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth",
        start: 13,
        end: 19,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth_cd",
        start: 19,
        end: 20,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "sex",
        start: 20,
        end: 21,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry",
        start: 21,
        end: 27,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry_cd",
        start: 27,
        end: 28,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "optional_data",
        start: 28,
        end: 35,
        coverage: CompositeOnly,
    },
    MrzFieldSpan {
        line: 2,
        name: "composite_cd",
        start: 35,
        end: 36,
        coverage: CompositeOnly,
    },
];

/// TD3 (ICAO 9303 part 4): two 44-character lines. Composite spans
/// `line2[0..10] + line2[13..20] + line2[21..43]` (`parse_td3_with`) — see
/// [`TD1_FIELDS`]'s doc for the pinning approach. TD3 is the one format where
/// `personal_number` carries its own check digit (`personal_number_cd`), so
/// unlike TD1/TD2's optional-data field, its content is `OwnCheckDigit`
/// rather than `CompositeOnly` even though the composite also spans it.
const TD3_FIELDS: &[MrzFieldSpan] = &[
    MrzFieldSpan {
        line: 1,
        name: "document_code",
        start: 0,
        end: 2,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "issuing_country",
        start: 2,
        end: 5,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "name",
        start: 5,
        end: 44,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "document_number",
        start: 0,
        end: 9,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "document_number_cd",
        start: 9,
        end: 10,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "nationality",
        start: 10,
        end: 13,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth",
        start: 13,
        end: 19,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth_cd",
        start: 19,
        end: 20,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "sex",
        start: 20,
        end: 21,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry",
        start: 21,
        end: 27,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry_cd",
        start: 27,
        end: 28,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "personal_number",
        start: 28,
        end: 42,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "personal_number_cd",
        start: 42,
        end: 43,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "composite_cd",
        start: 43,
        end: 44,
        coverage: CompositeOnly,
    },
];

/// MRV-A (ICAO 9303 part 7): two 44-character lines, geometry mirroring TD3
/// through the expiry check digit but with **no personal-number check digit
/// and no composite at all** (`parse_mrv_a_with` hardwires both
/// `Checks::personal_number` and `Checks::composite` to `true`) — so the
/// optional-data field is `Uncovered`, not `CompositeOnly`: there is no
/// composite here for anything to be covered by.
const MRVA_FIELDS: &[MrzFieldSpan] = &[
    MrzFieldSpan {
        line: 1,
        name: "document_code",
        start: 0,
        end: 2,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "issuing_country",
        start: 2,
        end: 5,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "name",
        start: 5,
        end: 44,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "document_number",
        start: 0,
        end: 9,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "document_number_cd",
        start: 9,
        end: 10,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "nationality",
        start: 10,
        end: 13,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth",
        start: 13,
        end: 19,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth_cd",
        start: 19,
        end: 20,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "sex",
        start: 20,
        end: 21,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry",
        start: 21,
        end: 27,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry_cd",
        start: 27,
        end: 28,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "optional_data",
        start: 28,
        end: 44,
        coverage: Uncovered,
    },
];

/// MRV-B (ICAO 9303 part 7): two 36-character lines, geometry mirroring TD2
/// through the expiry check digit but with no personal-number check digit
/// and no composite — see [`MRVA_FIELDS`]'s doc.
const MRVB_FIELDS: &[MrzFieldSpan] = &[
    MrzFieldSpan {
        line: 1,
        name: "document_code",
        start: 0,
        end: 2,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "issuing_country",
        start: 2,
        end: 5,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 1,
        name: "name",
        start: 5,
        end: 36,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "document_number",
        start: 0,
        end: 9,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "document_number_cd",
        start: 9,
        end: 10,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "nationality",
        start: 10,
        end: 13,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth",
        start: 13,
        end: 19,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_birth_cd",
        start: 19,
        end: 20,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "sex",
        start: 20,
        end: 21,
        coverage: Uncovered,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry",
        start: 21,
        end: 27,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "date_of_expiry_cd",
        start: 27,
        end: 28,
        coverage: OwnCheckDigit,
    },
    MrzFieldSpan {
        line: 2,
        name: "optional_data",
        start: 28,
        end: 36,
        coverage: Uncovered,
    },
];

/// The field layout for a resolved MRZ format string, in the same
/// `"TD1"`/`"TD2"`/`"TD3"`/`"MRVA"`/`"MRVB"` vocabulary [`mrz_format_str`]
/// produces and [`MissOcrDump::mrz_format`] carries. `None` for anything
/// else (there is no sixth format).
fn mrz_field_layout(format: &str) -> Option<&'static [MrzFieldSpan]> {
    match format {
        "TD1" => Some(TD1_FIELDS),
        "TD2" => Some(TD2_FIELDS),
        "TD3" => Some(TD3_FIELDS),
        "MRVA" => Some(MRVA_FIELDS),
        "MRVB" => Some(MRVB_FIELDS),
        _ => None,
    }
}

/// Maps a `synthpass-bench` field name — `CoreField::as_str`'s vocabulary,
/// which is also `bin/synthpass-bench.rs`'s `COMPARED_FIELDS`/
/// `FieldOutcome::field` — to the physical MRZ line that field lives on for
/// a resolved format.
///
/// Looks the name up in the very same [`mrz_field_layout`] table
/// [`mrz_field_mismatch`] already uses for checksum-coverage attribution,
/// rather than restating the offsets as a second table: a change to one of
/// the `*_FIELDS` arrays' `line` values cannot silently leave this reporting
/// path out of step, which is exactly the drift
/// `every_layout_declares_the_line_geometry_its_format_actually_has` guards
/// against for the layout itself.
///
/// The two vocabularies mostly agree, but not always — an MRZ line does not
/// itself separate a name into surname and given names (that split happens
/// in `mrz::parser`, downstream of the physical zone this table describes),
/// so both bench fields alias the layout's single `"name"` span; likewise
/// `"document_type"` aliases the layout's `"document_code"`. Every other
/// bench field name matches its span verbatim.
///
/// The two optional-data fields are format-aware (ADR-0018): each format
/// prints its optional-data element under its own span name — `optional_data_1`
/// and `optional_data_2` on TD1, `optional_data` on TD2 and the MRVs, and
/// `personal_number` on TD3, where the schema reports it under that name — so
/// `"optional_data_1"` resolves to the `optional_data` span on TD2/MRV-A/MRV-B
/// and to nothing on TD3, and `"optional_data_2"` resolves on TD1 only.
///
/// `None` covers two different reasons a field cannot be placed on a line,
/// deliberately left undistinguished here so a caller reports both under one
/// clearly-labelled group instead of inventing a line for either:
/// - `field` has no span in `format`'s layout at all — TD1 has no
///   `"personal_number"`, for instance; its equivalent content lives in
///   `optional_data_1`/`optional_data_2`, which are not the same field —
///   or `format` itself is not one [`mrz_field_layout`] resolves;
/// - `field` is `"mrz_lines"`, the whole-MRZ-zone CER `synthpass-bench`
///   reports alongside the per-field ones — never a field with a span, on
///   any format.
pub fn mrz_field_line(format: &str, field: &str) -> Option<usize> {
    let layout = mrz_field_layout(format)?;
    let span_name = match (format, field) {
        (_, "document_type") => "document_code",
        (_, "surname" | "given_names") => "name",
        ("TD2" | "MRVA" | "MRVB", "optional_data_1") => "optional_data",
        (_, other) => other,
    };
    layout
        .iter()
        .find(|span| span.name == span_name)
        .map(|span| span.line)
}

/// Per-field breakdown of an MRZ zone mismatch, for a resolved format: how
/// many characters differ inside each ICAO field, exactly which raw
/// positions those are, and — for every field that differed at all —
/// whether any check digit would have caught an error there.
///
/// Every field here is a name (from the static [`mrz_field_layout`] table)
/// or a position (`usize`); nothing on this type or [`mrz_field_mismatch`],
/// which builds it, ever reads a character out of the zones it is given.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct FieldMismatch {
    /// Differing character count per field name, e.g. `{"optional_data_2":
    /// 7, "composite_cd": 1}`.
    by_field: BTreeMap<String, usize>,
    /// Differing 0-based column positions per 1-based line number, e.g.
    /// `{2: [20, 21, 24, 25, 26, 27, 28, 29]}` — same line numbering as
    /// [`MrzFieldSpan::line`].
    by_line: BTreeMap<usize, Vec<usize>>,
    /// Checksum-coverage state of every field named in `by_field`.
    coverage: BTreeMap<String, CheckCoverage>,
}

/// Position-level sibling of [`mrz_zone_mismatch`]: same longer-of-the-two,
/// missing-character-counts-as-mismatch alignment, but returns *where* the
/// differences are instead of only how many there are.
/// [`mrz_zone_mismatch`]'s own return value and behaviour are unchanged by
/// this function's existence — see its doc.
///
/// Keys are 1-based line numbers (matching [`MrzFieldSpan::line`]); a line
/// with no differences is absent, not an empty vector.
fn mrz_zone_mismatch_positions(recovered: &str, truth: &str) -> BTreeMap<usize, Vec<usize>> {
    let rec: Vec<&str> = recovered.lines().collect();
    let tru: Vec<&str> = truth.lines().collect();
    (0..rec.len().max(tru.len()))
        .filter_map(|i| {
            let mut a = rec.get(i).copied().unwrap_or("").chars();
            let mut b = tru.get(i).copied().unwrap_or("").chars();
            let differing: Vec<usize> = std::iter::from_fn(|| match (a.next(), b.next()) {
                (None, None) => None,
                (x, y) => Some(x != y),
            })
            .enumerate()
            .filter_map(|(col, differs)| differs.then_some(col))
            .collect();
            (!differing.is_empty()).then_some((i + 1, differing))
        })
        .collect()
}

/// The width each 1-based line of a layout declares, derived from the spans
/// themselves rather than restated as constants, so it cannot drift from the
/// table it guards.
fn mrz_layout_line_widths(layout: &[MrzFieldSpan]) -> BTreeMap<usize, usize> {
    let mut widths: BTreeMap<usize, usize> = BTreeMap::new();
    for span in layout {
        let width = widths.entry(span.line).or_insert(0);
        *width = (*width).max(span.end);
    }
    widths
}

/// Does `zone` have the shape `layout` describes — the same number of lines,
/// each exactly the declared width?
///
/// Applied to the **ground truth** only. The truth is hand-transcribed and is
/// the authority on what the document actually prints, whereas a recovered
/// zone is OCR output that may legitimately be short or long; requiring the
/// read to match would suppress exactly the attributions this exists to make.
fn mrz_zone_matches_layout(zone: &str, layout: &[MrzFieldSpan]) -> bool {
    let widths = mrz_layout_line_widths(layout);
    let lines: Vec<&str> = zone.lines().collect();
    lines.len() == widths.len()
        && widths.iter().all(|(line, width)| {
            lines
                .get(line - 1)
                .is_some_and(|l| l.chars().count() == *width)
        })
}

/// Attribute an MRZ zone mismatch to ICAO fields, for a resolved format.
///
/// `None` when `format` isn't one [`mrz_field_layout`] recognises, **and also
/// when `truth` does not have the shape that format declares.** The second
/// case is not hypothetical: `resolve_mrz_format` takes the provider's own
/// `Evidence::mrz_format` whenever that read is checksum-valid (see #555), so
/// a checksum-valid read that still misdetects the format hands this
/// function a layout that does not describe the document at all. Measured
/// 2026-09-19 on a TD3 specimen read as TD1 — without the guard it reported
/// differing characters on *line 3 of a two-line zone* and populated TD1-only
/// fields (`issuing_country`, `optional_data_1`/`_2`) that the printed format
/// does not have, alongside `"unmapped": 28`.
///
/// That output is worse than none: it is well-formed, confident and wrong, and
/// nothing in the JSONL schema distinguishes it from a real attribution. So a
/// format the truth's own shape does not corroborate yields the same absent
/// result as an unresolved one, and **`Some` now carries a guarantee** — the
/// zone really is the declared format.
///
/// `"unmapped"` survives for a narrower case it is actually right for: a
/// recovered zone *longer* than the truth contributes positions past the last
/// declared span, and those are still counted rather than dropped silently.
///
/// **Structurally cannot serialize a character**: it is built entirely from
/// [`mrz_zone_mismatch_positions`]'s positions and the static field table —
/// `recovered`/`truth`'s characters are compared (`!=`) and never stored.
fn mrz_field_mismatch(format: &str, recovered: &str, truth: &str) -> Option<FieldMismatch> {
    let layout = mrz_field_layout(format)?;
    if !mrz_zone_matches_layout(truth, layout) {
        return None;
    }
    let by_line = mrz_zone_mismatch_positions(recovered, truth);
    let mut result = FieldMismatch {
        by_line: by_line.clone(),
        ..Default::default()
    };
    for (line, cols) in &by_line {
        for &col in cols {
            let field = layout
                .iter()
                .find(|f| f.line == *line && col >= f.start && col < f.end);
            let name = field.map_or("unmapped", |f| f.name);
            *result.by_field.entry(name.to_string()).or_insert(0) += 1;
            if let Some(f) = field {
                result.coverage.insert(name.to_string(), f.coverage);
            }
        }
    }
    Some(result)
}

/// How a labelled specimen's recovered zone compares with its hand-transcribed truth: counts
/// and column positions, never a character of either zone. Computed once per labelled
/// document ([`truth_comparison`]) and shared by the OCR miss dump and the archive
/// (ADR-0024), so the two cannot disagree.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub(crate) struct TruthComparison {
    /// Differing characters over the longer of the two zones ([`mrz_zone_mismatch`]).
    pub(crate) zone_mismatch: Option<usize>,
    /// Cells compared ([`compared_cells`]).
    pub(crate) compared_cells: Option<usize>,
    /// Where the differences are, by field ([`mrz_field_mismatch`]); needs a resolved format.
    pub(crate) field_mismatch: Option<FieldMismatch>,
}

/// [`TruthComparison`] of `recovered` against `truth`: `None` for a document with no
/// hand-transcribed zone, and otherwise a comparison whose parts are `None` where there is no
/// recovered zone (or, for the field map, no resolved `format`).
fn truth_comparison(
    recovered: Option<&mrz::MrzData>,
    truth: Option<&str>,
    format: Option<&str>,
) -> Option<TruthComparison> {
    truth_comparison_of_zone(recovered.map(|data| data.mrz_lines.as_str()), truth, format)
}

/// [`truth_comparison`] from the recovered zone's text, for a caller that holds the zone and not
/// the parsed [`mrz::MrzData`] (the `synthpass-bench` archive).
pub(crate) fn truth_comparison_of_zone(
    recovered: Option<&str>,
    truth: Option<&str>,
    format: Option<&str>,
) -> Option<TruthComparison> {
    let truth = truth?;
    Some(TruthComparison {
        zone_mismatch: recovered.map(|zone| mrz_zone_mismatch(zone, truth)),
        compared_cells: recovered.map(|zone| compared_cells(zone, truth)),
        field_mismatch: recovered
            .zip(format)
            .and_then(|(zone, format)| mrz_field_mismatch(format, zone, truth)),
    })
}

/// Writes one document's archive record for one provider (ADR-0024, Decision 5).
///
/// `detail` is the [`DocumentDetail`] just pushed, so the record's `ledger_row` is exactly
/// what the outcome ledger writes for the document. `tier1_zone` is the Tier-1 parse of the
/// provider's own input (`None` when nothing parsed), `fields` the provider's values (`None`
/// when the reader errored). The record holds the provider-input OCR text verbatim and never
/// the fixture's text: a labelled specimen contributes [`TruthComparison`]'s counts and
/// positions only. Nothing here can fail the run: [`Archive::record`] returns `()`.
#[allow(clippy::too_many_arguments)]
fn archive_document(
    archive: &Archive,
    provider: &str,
    bench_page: &BenchPage,
    detail: &DocumentDetail,
    read_elapsed: Duration,
    tier1_zone: Option<&mrz::MrzData>,
    fields: Option<BTreeMap<&'static str, Option<String>>>,
    truth: Option<&TruthComparison>,
) {
    let track = if bench_page.synthetic {
        ArchiveTrack::Synthetic
    } else {
        bench_page
            .asset_id
            .as_deref()
            // A real page with no asset ID cannot be placed in a track, so it is treated
            // as the most restrictive one and dropped, never filed as public.
            .map_or(ArchiveTrack::Private, |id| {
                ArchiveTrack::from_corpus(crate::asset_track(id))
            })
    };
    let record = DocRecord {
        kind: "doc",
        run_id: archive.run_id(),
        provider: provider.to_string(),
        track: track.as_str(),
        name: &bench_page.name,
        asset_id: bench_page.asset_id.as_deref(),
        source_sha256: bench_page.source_sha256.as_deref(),
        ledger_row: crate::report::OutcomeRow::from(detail),
        read_ok: detail.read_ok,
        read_us: read_elapsed.as_micros(),
        field_correctness: detail.field_correctness.as_ref().map(|fields| {
            fields
                .iter()
                .map(|(field, correctness)| (*field, correctness.as_str()))
                .collect()
        }),
        ocr: OcrRecord {
            text: &bench_page.page.text,
            rotation: bench_page.page.rotation,
            mrz_band_score: bench_page.page.mrz_band_score,
            chargrid: bench_page.page.chargrid.as_deref(),
            ocr_passes: bench_page.pass_objects.as_deref(),
        },
        tier1_read: tier1_zone.map(|data| Tier1ReadRecord {
            lines: data.mrz_lines.lines().map(str::to_string).collect(),
            damaged_recovery: data.damaged_recovery,
            valid: data.valid(),
        }),
        fields,
        truth,
    };
    archive.record(track, &record);
}

/// One in-denominator real-specimen miss (`checksum_failed` or
/// `no_mrz_found`), as written to `provider-bench-miss-ocr-dump.jsonl` when
/// `run_prepped` is given a `dump_ocr_dir`. Owned strings throughout — the
/// per-document Tier-1 read it draws `recovered_mrz_lines` from
/// (`synthpass_die::read_tier1`) is a loop local that does not outlive the
/// iteration. This is a diagnostic artifact, not a hot path.
///
/// For a `no_mrz_found` row `recovered_mrz_lines` is empty, `check_states` is
/// absent, and `zone_mismatch` is `None`: nothing MRZ-shaped parsed. What
/// matters there is `mrz_band_score` (was a band even found?) and
/// `raw_ocr_text` (what did the recognizer see?) — the localization-vs-
/// recognition question `ADR-0008` chunk 1C cell (b) asks.
#[derive(serde::Serialize)]
struct MissOcrDump {
    /// Real specimen file stem.
    name: String,
    /// Corpus-relative path identity; names are not unique in the corpus.
    asset_id: Option<String>,
    /// SHA-256 of the original encoded image, before OCR preprocessing.
    source_sha256: Option<String>,
    /// Filename, in this dump's directory, of the run details JSON.
    run_manifest: Option<String>,
    /// `"checksum_failed"` or `"no_mrz_found"` — which gate this row is
    /// under, so a consumer can filter without re-deriving it from the
    /// other fields.
    miss_reason: Option<&'static str>,
    /// Which reader produced the miss (`"mrz"`, `"llm"`, …).
    provider: String,
    /// Resolved ICAO format for the row, when one is known.
    mrz_format: Option<String>,
    /// `OcrPage::mrz_band_score` — a low value reframes the miss as band
    /// detection rather than character OCR.
    mrz_band_score: Option<f64>,
    /// The full pre-parse OCR text the provider was handed.
    raw_ocr_text: String,
    /// The MRZ zone `synthpass_die::read_tier1` recovered (post
    /// width/substitution repair, and post line-1 selection under
    /// `SYNTHPASS_MRZ_LINE1_SELECT=on`, the default), one entry per line; empty
    /// if nothing parsed. With the class sweep `off` and the line-1 selector
    /// `off` this is what `mrz::find_and_parse` recovers, as it was before the
    /// arms reached it.
    recovered_mrz_lines: Vec<String>,
    /// Per-check-digit state: `true` verified, `false` failed, `null` not
    /// printed by this layout.
    #[serde(skip_serializing_if = "Option::is_none")]
    check_states: Option<BTreeMap<String, Option<bool>>>,
    /// The hand-transcribed true printed MRZ zone, when this specimen carries
    /// a `samples/ocr_fixtures/<stem>.json` label; `None` for the unlabelled
    /// majority.
    ground_truth_mrz: Option<String>,
    /// Characters where `recovered_mrz_lines` differs from `ground_truth_mrz`
    /// ([`mrz_zone_mismatch`]). `Some(0)` → the printed zone was recovered
    /// faithfully and its own check digits are what failed
    /// (`checksum_failed_specimen`); `Some(n > 0)` → OCR introduced the error
    /// and this is the Phase-3d character-fix material; `None` → no label.
    zone_mismatch: Option<usize>,
    /// [`FieldMismatch::by_field`]: differing character count per ICAO field
    /// name, e.g. `{"optional_data_2": 7, "composite_cd": 1}`. `None` under
    /// the same conditions as `zone_mismatch` (no ground truth, or nothing
    /// parsed), plus when the resolved format is itself unknown — there is
    /// no field table to attribute against without one. Positions and
    /// counts only; see [`mrz_field_mismatch`]'s doc for why no character
    /// value can reach this field.
    field_mismatch_counts: Option<BTreeMap<String, usize>>,
    /// [`FieldMismatch::by_line`]: differing 0-based column positions per
    /// 1-based line number, e.g. `{"2": [20, 21, 24, 25, 26, 27, 28, 29]}`.
    /// Same availability as `field_mismatch_counts`.
    field_mismatch_positions: Option<BTreeMap<usize, Vec<usize>>>,
    /// [`FieldMismatch::coverage`]: checksum-coverage state of every field
    /// named in `field_mismatch_counts` — whether a check digit (dedicated
    /// or composite, or none at all) would have caught an error there. Same
    /// availability as `field_mismatch_counts`.
    field_mismatch_coverage: Option<BTreeMap<String, CheckCoverage>>,
    /// Number of recovered/ground-truth character cells actually compared.
    compared_cells: Option<usize>,
}

/// Everything measured for one provider over one corpus run.
pub struct ProviderReport {
    pub provider_id: String,
    /// How many documents this provider actually saw (OCR succeeded for
    /// them). The population `speed`/`json_validity` are measured over, and
    /// the denominator `AccuracyStats::labelled_documents` is a subset of.
    pub documents: usize,
    pub capability: CapabilitySnapshot,
    pub accuracy: AccuracyStats,
    pub speed: SpeedStats,
    /// `None` for deterministic providers — there is no JSON step to measure.
    pub json_validity: Option<JsonValidityStats>,
    /// Whether a value the provider asserted appears, verbatim, anywhere in
    /// the OCR text it was given — computed only for a text-only provider.
    /// See [`UnsupportedAssertion`]'s doc for why a vision-capable provider
    /// gets `NotApplicable` instead of a number.
    pub unsupported_assertion: UnsupportedAssertion,
    /// `Capability::estimated_resident_bytes` passthrough — self-declared,
    /// always available, zero measurement cost.
    pub declared_resident_bytes: Option<u64>,
    /// Coarse process-RSS delta sampled around this provider's whole loop,
    /// only populated with `--measure-memory` (feature `measure-memory`).
    /// **Not** an isolated per-provider measurement: all providers run
    /// sequentially in one process, and a model's warm-load dominates any
    /// one call's delta. Treat as an order-of-magnitude sanity check on
    /// `declared_resident_bytes`, not a precise figure.
    pub measured_rss_delta_bytes: Option<i64>,
    /// Per-document breakdown behind the aggregates above, in corpus order.
    ///
    /// An aggregate says a provider made 162 unsupported assertions; it never
    /// says *which document* produced them, and a rate you cannot drill into
    /// is a rate you cannot act on. Always collected — it is a handful of
    /// integers per document — and rendered only on request
    /// (`provider-bench --verbose`).
    pub documents_detail: Vec<DocumentDetail>,
    /// Fraction of documents that were a genuine Tier-1 hit: MRZ found, ICAO
    /// checksums valid, and (when ground truth exists for the document)
    /// document number matches. `documents_detail.iter().filter(|d|
    /// d.miss_reason.is_none())`'s count over its length.
    ///
    /// **Not the same thing as `read_ok`/`read_ok_rate`.** `read_ok` only
    /// means "the provider returned without erroring" — for the deterministic
    /// `mrz` provider that is unconditionally `true` (`MrzReader::read` is
    /// documented to never return `Err`: a document with no MRZ is a
    /// legitimate answer, not an error), so `read_ok_rate` alone cannot tell
    /// a clean specimen from one where OCR found nothing at all. This field
    /// is the actual "did Tier-1 work" gate — the same one
    /// `synthpass-bench`'s `hit_rate` already is for the synthetic corpus —
    /// so real-specimen and synthetic tracks are finally comparable on the
    /// same axis.
    ///
    /// `NotApplicable` for a non-`capability.deterministic` provider — see
    /// [`Tier1HitRate`]'s doc for why.
    pub tier1_hit_rate: Tier1HitRate,
    /// This run's `SYNTHPASS_OCR_*` measurement-arm configuration
    /// (`synthpass_ocr::OcrArms::from_env`), read once per provider loop —
    /// the same for every provider in a run, since it is process-global env
    /// state, not per-provider. Carried on the report so a baseline write
    /// can refuse a non-default configuration (see
    /// `knowledge/benchmarks/README.md`'s maintenance contract).
    pub ocr_arms: synthpass_ocr::OcrArms,
    /// Of the *name-scorable* documents in `tier1_hit_rate`'s own scored
    /// denominator (ground truth carries both `surname` and `given_names`),
    /// what fraction were both a Tier-1 hit and read both names exactly —
    /// see [`StrictNameHitRate`]'s doc for the exact denominator and why it
    /// needs its own `Option`-shaped outcome rather than folding into
    /// `tier1_hit_rate` itself.
    pub strict_tier1_hit_rate: StrictNameHitRate,
}

/// What one provider did on one document. **Shape only, never content**:
/// counts and field *names*, never the values asserted.
///
/// The values are the document's identity fields — exactly the PII
/// `CONTRIBUTING.md` forbids in any log line, and the discipline
/// `synthpass_core::fusion::Finding` already follows by carrying a field name
/// and never the value that disagreed. Knowing *that* `surname` was
/// unsupported on a given specimen is enough to go and look; printing the
/// fabricated surname itself would put document content into a terminal and a
/// JSON report for no additional diagnostic power.
#[derive(Debug)]
pub struct DocumentDetail {
    /// Corpus-local identity: a synthetic document's seed, or a real
    /// specimen's file stem. Public-domain specimen filenames, not PII.
    pub name: String,
    /// Exact samples-relative asset identity for real-specimen joins.
    pub asset_id: Option<String>,
    /// Whether `mrz::find_and_parse` recovered an MRZ from this document's
    /// OCR text — which bucket this document counted toward.
    pub mrz_found: bool,
    /// The document's ICAO 9303 MRZ format, resolved per corpus source —
    /// the M6 plan's "report Tier-1 hit rate keyed by mrz::Format" applied to
    /// this harness's own metrics (see `miss_reason` for the
    /// `check_document`-style hit/miss equivalent this loop now has):
    ///
    /// - Synthetic corpus: always `Some`, the exact format
    ///   `synthpass_gen::Labels::mrz_format` recorded (Phase 1) — never
    ///   re-derived from the read.
    /// - Real specimens: this provider's own Tier-1 read, via
    ///   `Evidence::mrz_format` (populated only by the deterministic MRZ
    ///   provider's `mrz_format_of`), but **only when that read's checksums
    ///   are valid** (`Evidence::mrz_checksums_valid`); else a best-effort
    ///   guess from the ground-truth `mrz_line` via
    ///   `MrzFormat::guess_from_lines`. `None` when neither resolved a
    ///   format — a specimen with no MRZ is not a TD-anything. A
    ///   checksum-invalid read used to win regardless, which mislabelled a
    ///   redacted specimen's fallback read as whatever format its garbage
    ///   happened to start with — see `resolve_mrz_format` and #555.
    pub mrz_format: Option<&'static str>,
    /// Whether the provider returned a reading at all. `false` means it
    /// errored and contributed nothing to any aggregate. **Not** a Tier-1
    /// signal — see `miss_reason` for that.
    pub read_ok: bool,
    /// `Evidence::mrz_checksums_valid` passthrough — `false` whenever
    /// `read_ok` is `false` too (nothing to check), since a provider that
    /// errored produced no MRZ to validate.
    pub mrz_checksums_valid: bool,
    /// Per-check-digit state from the parsed OCR MRZ. Absent only when no MRZ
    /// parsed; otherwise all five fields are recorded, including `None` for
    /// digits this layout does not print.
    pub check_states: Option<BTreeMap<&'static str, Option<bool>>>,
    /// Why this document is not a Tier-1 hit. `None` on a genuine hit: MRZ
    /// found, checksums valid, and (when labelled) document number matches
    /// ground truth. Mirrors `synthpass-bench`'s own miss classification
    /// (`crate::MissReason`) so real-specimen and synthetic-corpus misses
    /// aggregate the same way — see `crate::miss_kind`.
    pub miss_reason: Option<MissReason>,
    pub assertions_total: usize,
    pub assertions_unsupported: usize,
    /// Which fields were asserted but absent from the OCR text. Names only.
    pub unsupported_fields: Vec<&'static str>,
    /// Whether this document's `surname` and `given_names` both matched
    /// ground truth exactly (`crate::classify_names` returning `None`).
    ///
    /// `None` means "not name-scorable" — this document's ground truth is
    /// missing `surname` and/or `given_names` entirely, so there is nothing
    /// to compare against. `Some(_)` — including `Some(false)` — means
    /// ground truth had both fields, so this document counts toward
    /// [`StrictNameHitRate`]'s denominator regardless of whether the read
    /// itself succeeded: a provider whose read *errored* on a name-scorable
    /// document is `Some(false)` here (with `name_error: None`, since there
    /// is no read to classify the shape of), not `None` — an error is not
    /// the same fact as "nothing to measure". This is the one place
    /// `names_exact.is_some()` alone tells you "name-scorable"; every other
    /// reader of this field should ask that question, not re-derive it from
    /// `read_ok`/`miss_reason`.
    ///
    /// See `crate::NameError`'s doc for why this is a reporting-only axis
    /// distinct from `miss_reason`/the Tier-1 hit itself, and
    /// `mrz_ground_truth`/`extraction_ground_truth` for why real-specimen
    /// ground truth is scorable here at all (both are hand-transcribed from
    /// the printed MRZ zone in ICAO/MRZ form, not the visual zone — see
    /// `CORPUS_COVERAGE.md`'s 2026-09-08 note and
    /// `knowledge/benchmarks/README.md`'s name-accuracy section).
    pub names_exact: Option<bool>,
    /// Which way the names diverged, as `crate::NameError::as_str()` — KIND
    /// ONLY, per this struct's module doc: never the surname/given-names
    /// values themselves. `None` both when `names_exact` is `Some(true)` and
    /// when it is `None`.
    pub name_error: Option<&'static str>,
    /// Per-field correctness of this document's **accepted read**, for every
    /// field its ground truth defines, keyed by [`CoreField::as_str`]. **Field
    /// names and the [`FieldCorrectness`] enum only, never a value**: the
    /// report is uploaded from CI and the truth is real-specimen text
    /// (ADR-0027).
    ///
    /// `None` when the document has no ground truth (or truth for none of the
    /// twelve fields), the same population `labelled_documents` counts.
    /// Report-only: nothing gates on it (#574).
    ///
    /// Derived from the comparison `FieldTally` runs (see `compare_document`),
    /// so summing `Exact` over the documents reproduces
    /// [`AccuracyStats::accepted_reads`]'s per-field exact counts. A document
    /// whose read was not accepted ([`crate::is_accepted_read`]), and one whose
    /// reader errored, reads every field `Unread`: there is no accepted read
    /// to have got anything right or wrong. That is what makes this a
    /// per-document view of the read-quality population rather than of the
    /// looser all-answers one.
    pub field_correctness: Option<BTreeMap<&'static str, FieldCorrectness>>,
    /// Wall-clock time of this document's OCR pass (`recognize_detailed`),
    /// carried through from preparation. The same for every reader, since OCR
    /// runs once per document and is shared. Recorded per document because a
    /// run's total cannot say *which* documents cost it
    /// (`knowledge/decisions/ADR-0010-benchmark-cost-split-by-role.md`, step 5).
    pub ocr_elapsed: Duration,
    /// Native OCR retry pass selected for this asset, when available (`pass-NN`; its number depends on retry configuration).
    pub retry_variant_id: Option<String>,
    /// `OcrPage::retry_damaged_recovery` passthrough — `MrzData::damaged_recovery`
    /// of the reading the native retry loop accepted. `Some` exactly
    /// when `retry_variant_id` is `Some`.
    pub retry_damaged_recovery: Option<bool>,
    /// Whether the native retry loop hit its wall-clock budget.
    pub retry_budget_hit: bool,
    /// Why the native retry loop stopped, when native OCR telemetry exists.
    pub retry_stop: Option<String>,
    /// `OcrPage::chargrid` passthrough — the post-hit chargrid name-line
    /// repair arm's outcome for this document, when native OCR telemetry
    /// exists and `SYNTHPASS_OCR_CHARGRID` was not `off`. See that field's
    /// doc for the possible values.
    pub chargrid: Option<String>,
    /// This document's Tier-1 read's `MrzData::damaged_recovery` —
    /// independent of `retry_damaged_recovery`, which describes the reading
    /// the *native OCR retry loop* accepted. `None` when Tier 1 found no MRZ
    /// at all.
    pub tier1_damaged_recovery: Option<bool>,
    /// The line-1 selector's verdict on this document's Tier-1 read (#574),
    /// text-free. Present on a default run (the selector is on by default);
    /// `None` when the arm is `off`, for a provider whose reading records none
    /// (only the deterministic `mrz` provider does), and when Tier 1 accepted
    /// nothing to select on.
    pub line1_selection: Option<Line1SelectionDetail>,
}

/// What the line-1 selector said about one document, for the report:
/// [`synthpass_die::Line1Summary`] plus where the proposed line was read.
///
/// **Text-free (ADR-0027).** Every string here is a verdict kind, a reason
/// kind, a pass id (`pass-03`, `general`) or a `PassTransform` wire label
/// (`mrz_variants:2`): none is document text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line1SelectionDetail {
    /// `control` or `on`; a document has none under `off`.
    pub arm: &'static str,
    /// `applied`, `proposed`, `unresolved`, `ambiguous` or `none`
    /// ([`synthpass_die::Line1Outcome::as_str`]).
    pub verdict: &'static str,
    /// Why the selector did nothing, for `none` and `unresolved`
    /// ([`synthpass_die::Line1Reason::as_str`]).
    pub reason: Option<&'static str>,
    /// Candidate lines that passed every eligibility check, duplicates included.
    pub eligible: usize,
    /// Distinct name fields among them.
    pub distinct: usize,
    /// Whether the proposal's `surname` or `given_names` differ from the
    /// accepted read's.
    pub names_changed: bool,
    /// The OCR passes whose readings include the proposed line 1, in execution
    /// order. Empty unless the verdict is `proposed` or `applied` **and** a pass
    /// trace exists: a replay, or a live run with `--dump-ocr-passes`.
    pub source_passes: Vec<String>,
    /// The `PassTransform` labels of those passes, each once, in the order
    /// they first appear.
    pub source_transforms: Vec<String>,
}

/// One OCR pass's readings, kept beside a document's page so a line-1 proposal
/// can be traced to the passes that read it (#574). **Document text**: it is
/// compared and never written anywhere.
#[derive(Debug, Clone, PartialEq, Eq)]
struct PassProvenance {
    id: String,
    /// The `synthpass_ocr::PassTransform` wire label.
    transform: String,
    /// The text of each MRZ-shaped line the pass read.
    readings: Vec<String>,
}

/// The provenance of a pass trace, in execution order.
fn pass_provenance(passes: &[crate::ocr_passes::PassObject]) -> Vec<PassProvenance> {
    passes
        .iter()
        .map(|pass| PassProvenance {
            id: pass.id.clone(),
            transform: pass.transform.clone(),
            readings: pass
                .readings
                .iter()
                .map(|reading| reading.text.clone())
                .collect(),
        })
        .collect()
}

/// A reading with whitespace stripped and case folded: the form
/// `mrz`'s line walk compares, so a proposal's line 1 (already in that form)
/// can be found in a pass's raw reading.
fn comparable_reading(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_uppercase()
}

/// The report detail for `summary`, with provenance from `passes`.
///
/// `proposed_line1` is the proposal's line 1 (`synthpass_die::Tier1Read::
/// proposed_line1`); it is compared against the passes' readings and never
/// stored. The source lists stay empty unless the verdict is `proposed` or
/// `applied`, so a `kept` document never claims a source.
fn line1_selection_detail(
    summary: &synthpass_die::Line1Summary,
    proposed_line1: Option<&str>,
    passes: &[PassProvenance],
) -> Line1SelectionDetail {
    let mut source_passes: Vec<String> = Vec::new();
    let mut source_transforms: Vec<String> = Vec::new();
    let proposed = matches!(
        summary.outcome,
        synthpass_die::Line1Outcome::Proposed | synthpass_die::Line1Outcome::Applied
    );
    if let (true, Some(line1)) = (proposed, proposed_line1) {
        for pass in passes {
            if pass
                .readings
                .iter()
                .any(|reading| comparable_reading(reading) == line1)
            {
                source_passes.push(pass.id.clone());
                if !source_transforms.contains(&pass.transform) {
                    source_transforms.push(pass.transform.clone());
                }
            }
        }
    }
    Line1SelectionDetail {
        arm: summary.arm.name(),
        verdict: summary.outcome.as_str(),
        reason: summary.reason.map(synthpass_die::Line1Reason::as_str),
        eligible: summary.eligible,
        distinct: summary.distinct,
        names_changed: summary.names_changed,
        source_passes,
        source_transforms,
    }
}

/// `documents_detail`'s verdicts, counted: every verdict kind is a key, so an
/// arm that never proposed reads `proposed: 0` rather than an absent key.
/// `None` when no document carries a selection (the arm is `off`, or the
/// provider records none), so the key is absent from such a report.
pub fn line1_selection_counts(details: &[DocumentDetail]) -> Option<BTreeMap<&'static str, usize>> {
    let mut counts: Option<BTreeMap<&'static str, usize>> = None;
    for selection in details.iter().filter_map(|d| d.line1_selection.as_ref()) {
        let counts = counts.get_or_insert_with(|| {
            ["applied", "proposed", "unresolved", "ambiguous", "none"]
                .into_iter()
                .map(|verdict| (verdict, 0))
                .collect()
        });
        *counts.entry(selection.verdict).or_insert(0) += 1;
    }
    counts
}

pub struct CapabilitySnapshot {
    pub deterministic: bool,
    pub vision: bool,
    pub cost: &'static str,
}

impl CapabilitySnapshot {
    fn from(capability: &Capability) -> Self {
        Self {
            deterministic: capability.deterministic,
            vision: capability.vision,
            cost: match capability.cost {
                CostClass::Free => "free",
                CostClass::Cheap => "cheap",
                CostClass::Expensive => "expensive",
            },
        }
    }
}

/// Field-match rate and mean CER, scored only over documents that actually
/// have ground truth.
///
/// `field_match_rate`/`mean_cer`/`per_field_cer` are `Option`, not `f64`
/// defaulting to `0.0`, on purpose: `0.0` accuracy is a measurement (the
/// provider got everything wrong), while `None` is the honest report of "no
/// labelled population to measure against." Collapsing the two would let an
/// all-unlabelled real-specimen run silently print "0% field match" — a
/// fabricated number, not a measured one, for exactly the population this
/// crate's real-specimen mode exists to cover honestly (every MRZ-less
/// ID-card front in `samples/` has no `ocr_fixtures` label). Carrying
/// `labelled_documents` alongside makes the `Option` legible on its own: a
/// report can always say *how many* documents a rate (or its absence) was
/// computed over.
///
/// **Three populations (#564).** `field_match_rate`, `mean_cer` and
/// `per_field_cer` cover every labelled document the reader answered,
/// whatever its outcome. That mixes read quality with the corpus's share of
/// misses and non-conforming specimens, so they are kept for continuity with
/// the bench history and not quoted as accuracy. The two populations to quote
/// are [`Self::accepted_reads`] (read quality) and [`Self::scored`]
/// (end-to-end), each with its own document count.
#[derive(Debug)]
pub struct AccuracyStats {
    /// How many of `documents` contributed at least one ground-truth field.
    /// Always equal to `documents` for the synthetic corpus (every
    /// `CorpusDoc` has labels by construction); a subset of it for real
    /// specimens (only the ones with a matching `samples/ocr_fixtures/
    /// <stem>.json`).
    pub labelled_documents: usize,
    /// Fraction of `(document, field)` pairs where the provider's answer
    /// exactly matches ground truth, over fields ground truth actually has a
    /// value for, on every labelled document. `None` iff
    /// `labelled_documents == 0`.
    pub field_match_rate: Option<f64>,
    /// Mean character error rate over the same population, via
    /// [`crate::cer`] — reuses the exact metric `synthpass-bench`'s Tier-1
    /// report already uses, so the two numbers are comparable. `None` iff
    /// `labelled_documents == 0`.
    pub mean_cer: Option<f64>,
    /// Per-field mean CER over the same population, with the number of
    /// documents that had truth for the field. A field's mean is `None` when
    /// no labelled document had ground truth for that specific field (for
    /// instance every real specimen loaded so far omitting
    /// `personal_number`), for the same reason the whole-report rate is
    /// `Option`.
    pub per_field_cer: Vec<(&'static str, Option<f64>, usize)>,
    /// **Read quality:** field accuracy over the labelled documents with an
    /// accepted read ([`crate::is_accepted_read`]), what a caller receives
    /// when the check digits verify.
    ///
    /// Not "Tier-1 hits": a hit also requires the document number to equal
    /// the truth, so a hits-only population would make `document_number`
    /// right by construction and keep only the reads that already agree with
    /// the truth. A document-number mismatch is an accepted read, and its
    /// wrong fields count here.
    pub accepted_reads: PopulationAccuracy,
    /// **End-to-end:** field accuracy over the labelled documents in
    /// [`Tier1HitRate`]'s scored population ([`in_scored_tier1_population`]),
    /// the same documents the hit rate is computed over.
    ///
    /// A scored document the reader did not read contributes every field it
    /// has truth for, scored as absent, and so does one the reader errored
    /// on. A `checksum_failed_specimen` is outside it: its printed zone fails
    /// its own check digits, so no read of it can be accepted, and the reader
    /// returns no fields for a failed checksum.
    pub scored: PopulationAccuracy,
}

/// Field-match rate and mean CER over one population of labelled documents,
/// with the population's size and a per-field breakdown.
///
/// `documents` counts the documents that contributed at least one field
/// comparison. The rates are `None` iff it is zero, for the reason
/// [`AccuracyStats`]'s doc gives: no population is not 0% accuracy.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PopulationAccuracy {
    pub documents: usize,
    pub field_match_rate: Option<f64>,
    pub mean_cer: Option<f64>,
    /// One entry per [`CoreField`], in [`CoreField::ALL`] order.
    pub per_field: Vec<FieldAccuracy>,
}

/// One field's accuracy within a [`PopulationAccuracy`].
#[derive(Debug, Clone, PartialEq)]
pub struct FieldAccuracy {
    pub field: &'static str,
    /// How many documents in the population had truth for this field. Each
    /// rate is over these documents only, so a mean over one document is
    /// visibly one document.
    pub documents: usize,
    pub match_rate: Option<f64>,
    pub mean_cer: Option<f64>,
}

/// Running counts behind one [`PopulationAccuracy`].
struct FieldTally {
    documents: usize,
    comparisons: usize,
    exact: usize,
    cer_sum: f64,
    /// Indexed like [`CoreField::ALL`]: `(comparisons, exact, cer_sum)`.
    per_field: Vec<(usize, usize, f64)>,
}

impl FieldTally {
    fn new() -> Self {
        Self {
            documents: 0,
            comparisons: 0,
            exact: 0,
            cer_sum: 0.0,
            per_field: vec![(0, 0, 0.0); CoreField::ALL.len()],
        }
    }

    /// Adds one document's field comparisons, each `(index into
    /// CoreField::ALL, exact, cer)`. A document with none is not counted.
    fn add_document(&mut self, comparisons: &[(usize, bool, f64)]) {
        if comparisons.is_empty() {
            return;
        }
        self.documents += 1;
        for &(field, exact, cer) in comparisons {
            self.comparisons += 1;
            self.exact += usize::from(exact);
            self.cer_sum += cer;
            let (n, field_exact, field_cer) = &mut self.per_field[field];
            *n += 1;
            *field_exact += usize::from(exact);
            *field_cer += cer;
        }
    }

    fn finish(self) -> PopulationAccuracy {
        let rate = |sum: f64, count: usize| (count > 0).then(|| sum / count as f64);
        PopulationAccuracy {
            documents: self.documents,
            field_match_rate: rate(self.exact as f64, self.comparisons),
            mean_cer: rate(self.cer_sum, self.comparisons),
            per_field: CoreField::ALL
                .iter()
                .zip(self.per_field)
                .map(|(field, (n, exact, cer_sum))| FieldAccuracy {
                    field: field.as_str(),
                    documents: n,
                    match_rate: rate(exact as f64, n),
                    mean_cer: rate(cer_sum, n),
                })
                .collect(),
        }
    }
}

/// How one field of one document came out against its ground truth.
///
/// A name and a verdict, never the value: see
/// [`DocumentDetail::field_correctness`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldCorrectness {
    /// The read equals the truth exactly.
    Exact,
    /// A value was read and it differs from the truth.
    Wrong,
    /// No accepted read, or the field is absent from the read.
    Unread,
}

impl FieldCorrectness {
    /// The lower-case wire name used in the report JSON.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::Wrong => "wrong",
            Self::Unread => "unread",
        }
    }
}

/// One field's comparison against its truth, the single rule behind the
/// aggregates ([`FieldTally`], the all-labelled rate and CER) and the
/// per-document [`FieldCorrectness`] map.
struct FieldComparison {
    exact: bool,
    cer: f64,
    correctness: FieldCorrectness,
}

/// Compares one field. `got` is `None` when the field is absent from the read
/// (`ExtractionFields::get` also returns `None` for an empty or blank value).
///
/// Exactness is decided first and by the aggregates' rule (`got == expected`,
/// an absent read being `""`), so the map's `Exact` count cannot differ from
/// the tally's. Only a non-exact answer is split into `Unread` and `Wrong`.
fn compare_field(expected: &str, got: Option<&str>) -> FieldComparison {
    let got_str = got.unwrap_or("");
    let exact = got_str == expected;
    let correctness = if exact {
        FieldCorrectness::Exact
    } else if got_str.is_empty() {
        FieldCorrectness::Unread
    } else {
        FieldCorrectness::Wrong
    };
    FieldComparison {
        exact,
        cer: crate::cer(expected, got_str),
        correctness,
    }
}

/// One document's field comparisons in both forms the harness needs.
struct DocumentComparison {
    /// `(index into CoreField::ALL, exact, cer)` for each field the document
    /// has truth for, in `CoreField::ALL` order: [`FieldTally::add_document`]'s
    /// input.
    tally: Vec<(usize, bool, f64)>,
    /// The same comparisons as a verdict per field name.
    correctness: BTreeMap<&'static str, FieldCorrectness>,
}

impl DocumentComparison {
    /// The map for [`DocumentDetail::field_correctness`]. `None` when the
    /// document has no truth to compare against. When the read was not an
    /// accepted one, every field is `Unread` whatever the tally scored.
    fn field_correctness(
        &self,
        accepted: bool,
    ) -> Option<BTreeMap<&'static str, FieldCorrectness>> {
        if self.correctness.is_empty() {
            return None;
        }
        Some(
            self.correctness
                .iter()
                .map(|(field, correctness)| {
                    let correctness = if accepted {
                        *correctness
                    } else {
                        FieldCorrectness::Unread
                    };
                    (*field, correctness)
                })
                .collect(),
        )
    }
}

/// Compares a document's read, when it has one, against its ground truth,
/// field by field. `fields` is `None` when the reader returned nothing: every
/// field the document has truth for is then scored as absent.
///
/// Only fields the truth has a value for are compared, see
/// [`AccuracyStats`]'s doc on why an absent label must not be scored as a
/// miss. A document without truth yields no comparisons.
fn compare_document(
    ground_truth: Option<&HashMap<CoreField, String>>,
    fields: Option<&ExtractionFields>,
) -> DocumentComparison {
    let mut comparison = DocumentComparison {
        tally: Vec::new(),
        correctness: BTreeMap::new(),
    };
    let Some(truth) = ground_truth else {
        return comparison;
    };
    for (i, field) in CoreField::ALL.iter().enumerate() {
        let Some(expected) = truth.get(field) else {
            continue;
        };
        let got = fields.and_then(|fields| fields.get(*field));
        let compared = compare_field(expected, got);
        comparison.tally.push((i, compared.exact, compared.cer));
        comparison
            .correctness
            .insert(field.as_str(), compared.correctness);
    }
    comparison
}

/// The unsupported-assertion metric outcome for one provider: either a
/// computed rate, or the capability reason it wasn't computed.
///
/// The predicate — "the answer must appear, verbatim, in the OCR text the
/// provider was given" — is correct for a **text-only** provider:
/// `synthpass-llm` consumes OCR Markdown, so a value not in that text came
/// from the model's weights, not the document. It is **invalid** for a
/// **vision** provider reading pixels: a VLM producing a correct value that
/// appears nowhere in the OCR text is exactly the point of using one. Ship
/// this metric unconditionally and it would report a working vision
/// provider as maximally hallucinating precisely when it is right — the
/// scenario `knowledge/ROADMAP.md`'s M7 section calls out by name for the
/// planned Qwen-vs-Moondream comparison. So this is computed only when
/// `Capability::vision` is `false`; a vision-capable provider gets
/// `NotApplicable` with the reason recorded, never a silently-wrong number.
#[derive(Debug)]
pub enum UnsupportedAssertion {
    /// Computed over the non-null answered fields of a `!capability.vision`
    /// provider, split by whether the document gave it an MRZ to anchor on.
    Computed {
        overall: AssertionBucket,
        /// Documents where `mrz::find_and_parse` recovered an MRZ. `None`
        /// when the corpus contained no such document.
        with_mrz_anchor: Option<AssertionBucket>,
        /// Documents with no MRZ at all — the population where a text-only
        /// provider has no deterministic anchor whatsoever. `None` when the
        /// corpus contained no such document, which is always the case for
        /// the synthetic corpus (every generated document carries an MRZ by
        /// construction).
        without_mrz_anchor: Option<AssertionBucket>,
    },
    /// Not computed. `capability.vision` was `true` for this provider.
    NotApplicable { reason: &'static str },
}

/// One unsupported-assertion measurement over a named subset of documents.
///
/// `documents` is carried alongside the rate because the two halves of the
/// split are not the same size and a rate alone would hide that: "40% over
/// 4 documents" and "40% over 120" are different claims, and the whole point
/// of this split is that the smaller half is the one nobody had measured.
#[derive(Debug)]
pub struct AssertionBucket {
    /// `None` when the provider made **no assertions at all** over this
    /// subset — not the same fact as a measured rate of zero, and on this
    /// metric the difference is the whole point.
    ///
    /// The first real run made that concrete: on the 22 corpus documents with
    /// no MRZ, the deterministic reader asserted nothing whatsoever, while the
    /// LLM asserted 162 field values. Rendering the former as `0.0` printed it
    /// as "measured, and perfectly clean" — praise for the one provider that
    /// had correctly declined to answer, in the same column where a genuine
    /// 0.0 would mean "answered a lot, all of it supported". Exactly the
    /// fabricated-zero that [`AccuracyStats`]'s `Option`s exist to prevent,
    /// one struct over.
    pub rate: Option<f64>,
    pub assertions_total: usize,
    pub documents: usize,
}

impl AssertionBucket {
    /// `None` when the subset had no documents at all — absent, rather than a
    /// bucket that would have to invent a rate for an empty population. A
    /// bucket that *does* exist may still carry `rate: None`; see
    /// [`AssertionBucket::rate`].
    fn new(unsupported: usize, total: usize, documents: usize) -> Option<Self> {
        (documents > 0).then_some(Self {
            rate: (total > 0).then(|| unsupported as f64 / total as f64),
            assertions_total: total,
            documents,
        })
    }
}

const VISION_REASON: &str = "capability.vision is true — the verbatim-in-OCR-text predicate \
    assumes a text-only provider and would misreport a correct pixel-grounded read as an \
    unsupported assertion";

/// Whether [`ProviderReport::tier1_hit_rate`] could be computed for a given provider.
///
/// The rate is defined in terms of ICAO checksum validity
/// (`Evidence::mrz_checksums_valid`), which only a deterministic, MRZ-format-aware
/// reader ever computes. A provider that doesn't set `capability.deterministic`
/// never asserts checksum validity at all — not even correctly — so treating its
/// `mrz_checksums_valid: false` default as a real miss would fabricate a rate of
/// exactly `0.0` regardless of how accurate the provider actually is. That's the
/// same shape of bug [`AssertionBucket::rate`]'s `Option` and
/// [`UnsupportedAssertion::NotApplicable`] both exist to prevent, one metric over.
#[derive(Debug)]
pub enum Tier1HitRate {
    /// Computed over a deterministic provider's nonempty scored population:
    /// the fraction with `miss_reason.is_none()`, excluding off-denominator classes.
    Computed(f64),
    /// Not computed: the provider is nondeterministic or the scored population is empty.
    NotApplicable { reason: &'static str },
}

impl std::fmt::Display for Tier1HitRate {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Computed(rate) => write!(formatter, "{:.1}%", rate * 100.0),
            Self::NotApplicable { reason } => write!(formatter, "n/a ({reason})"),
        }
    }
}

const NOT_DETERMINISTIC_REASON: &str = "capability.deterministic is false — this provider never \
    asserts ICAO checksum validity, so a checksum-validated Tier-1 hit rate can't be computed for \
    it; its Evidence::mrz_checksums_valid defaulting to false would otherwise fabricate a rate of \
    exactly 0.0 regardless of actual accuracy";

/// Whether [`ProviderReport::strict_tier1_hit_rate`] could be computed.
///
/// **Two related but differently-denominated numbers live here, and the
/// benchmark maintenance contract (`knowledge/benchmarks/README.md`) is
/// explicit that two similarly-named rates with different denominators is
/// exactly the trap to avoid — so both are named for what they divide by,
/// not just for what they count:**
///
/// - **`strict_tier1_hit_rate`** (this type's own name): `strict_hits /
///   name_scorable_documents`. The denominator is every document in
///   [`Tier1HitRate`]'s own scored population (i.e. excluding the same three
///   off-denominator kinds `no_mrz_expected`/`redacted_mrz`/
///   `checksum_failed_specimen` — see `run_prepped`'s `tier1_hit_rate`
///   computation, which this reuses) that is *also* name-scorable
///   (`DocumentDetail::names_exact` is `Some(_)`) — a document is counted
///   here whether it ended up a hit, a miss, or an errored read, as long as
///   its ground truth had both name fields. This is the rate that answers
///   "of everything I could have checked a name on, how often did I get a
///   correct, checksum-valid, name-matching read".
/// - **[`StrictNameHitRate::Computed`]'s `names_exact_among_hits`**: `strict_hits /
///   name_scorable_hits`, i.e. of the *Tier-1 hits* specifically that are
///   also name-scorable, how many also read both names exactly. This is the
///   number the CTC-collapse finding is stated in terms of (roughly half of
///   every synthetic format's hits carry a wrong name) — it isolates the
///   name-read question from detection accuracy, since a document that
///   missed Tier-1 for an unrelated reason never enters this denominator.
///
/// No ICAO check digit covers `surname`/`given_names` in any format, so a
/// Tier-1 hit proves nothing about them — see `crate::NameError`'s doc for
/// the CTC-decoder cause this metric exists to track.
///
/// A third, honest "nothing to report" case sits alongside
/// [`Tier1HitRate::NotApplicable`]'s reason: even for a deterministic
/// provider, both rates need at least one name-scorable document in the
/// scored population. An all-unlabelled real-specimen run — most of
/// `samples/`, by construction — has zero such documents, and reporting
/// `0.0` there would fabricate "every name wrong" out of "nothing was
/// measured", the exact bug [`AccuracyStats`]'s `Option`s and
/// [`AssertionBucket::rate`] both already exist to prevent.
#[derive(Debug)]
pub enum StrictNameHitRate {
    /// `strict_hits`, `name_scorable_documents` and `name_scorable_hits` are
    /// carried alongside both rates (rather than just the rates) so a caller
    /// can always tell "high rate over a handful of documents" from "over
    /// most of them" — see `AssertionBucket`'s identical reasoning for
    /// carrying `documents` next to `rate`.
    Computed {
        /// Tier-1 hits whose read also matched both name fields exactly.
        strict_hits: usize,
        /// Documents in the scored Tier-1 population whose ground truth
        /// carries both `surname` and `given_names` — `strict_tier1_hit_rate`'s
        /// denominator.
        name_scorable_documents: usize,
        /// Of `name_scorable_documents`, the ones that were also a Tier-1
        /// hit — `names_exact_among_hits`'s denominator.
        name_scorable_hits: usize,
        /// `strict_hits / name_scorable_documents`.
        strict_tier1_hit_rate: f64,
        /// `strict_hits / name_scorable_hits`, or `None` when there are no
        /// name-scorable hits. Labelled misses do not populate this denominator.
        names_exact_among_hits: Option<f64>,
    },
    /// Not computed. Either [`Tier1HitRate`] itself was `NotApplicable`
    /// (same `reason`), or no document in the scored population had ground
    /// truth for both name fields.
    NotApplicable { reason: &'static str },
}

const NO_NAME_SCORABLE_DOCUMENTS_REASON: &str = "no document in this run's scored Tier-1 \
    population had ground truth for both surname and given_names — most real specimens only \
    carry a hand-verified document_number, so an all-unlabelled or partially-labelled run has \
    nothing to compute a strict name hit rate over; reporting 0.0 here would fabricate \"every \
    name wrong\" out of \"nothing measured\"";

pub struct SpeedStats {
    pub mean: Duration,
    pub p50: Duration,
    pub p95: Duration,
}

#[derive(Debug)]
pub struct JsonValidityStats {
    /// `synthpass_llm::repair::repair_fallbacks()` delta across this
    /// provider's whole loop — how many of its answers needed JSON repair
    /// rather than parsing clean the first time.
    pub repair_fallbacks: u64,
    pub documents: usize,
}

/// The 12 [`CoreField`]s, paired with how to read the matching value off
/// [`mrz::MrzData`] — the synthetic corpus's ground truth. A local table
/// rather than reusing `synthpass-bench`'s private `COMPARED_FIELDS`: that
/// one is keyed by `&str` for the Tier-1-only report; this one is keyed by
/// `CoreField` so it can index `ExtractionFields::get` directly.
fn mrz_field(field: CoreField, truth: &mrz::MrzData) -> String {
    match field {
        CoreField::DocumentType => truth.document_type.clone(),
        CoreField::IssuingCountry => truth.issuing_country.clone(),
        CoreField::DocumentNumber => truth.document_number.clone(),
        CoreField::Surname => truth.surname.clone(),
        CoreField::GivenNames => truth.given_names.clone(),
        CoreField::Nationality => truth.nationality.clone(),
        CoreField::DateOfBirth => truth.date_of_birth.to_string(),
        CoreField::Sex => synthpass_core::mrz_product::sex(truth.sex)
            .unwrap_or_default()
            .to_string(),
        CoreField::DateOfExpiry => truth.date_of_expiry.to_string(),
        CoreField::PersonalNumber => truth.personal_number().unwrap_or_default().to_string(),
        CoreField::OptionalData1 => synthpass_die::mrz_reader::reported_optional_data_1(truth)
            .unwrap_or_default()
            .to_string(),
        CoreField::OptionalData2 => truth.optional_data_2.clone().unwrap_or_default(),
    }
}

/// Ground truth for a synthetic document, keyed by [`CoreField`]. A field
/// with an empty string in `mrz::MrzData` (`personal_number` and the two
/// optional-data fields, all genuinely optional on the zone) is omitted rather than stored
/// empty — "no ground truth for this field" and "ground truth is the empty
/// string" must be distinguishable, and only the former should be excluded
/// from `field_match_rate`/`mean_cer`.
fn mrz_ground_truth(truth: &mrz::MrzData) -> HashMap<CoreField, String> {
    CoreField::ALL
        .iter()
        .filter_map(|&field| {
            let value = mrz_field(field, truth);
            (!value.is_empty()).then_some((field, value))
        })
        .collect()
}

/// Ground truth for a real specimen, keyed by [`CoreField`], from its
/// optional v1 `Extraction` label. `None`/empty fields are omitted for the
/// same reason as [`mrz_ground_truth`]: an absent field in the label file
/// (most of them, for any specimen not hand-verified in full) must read as
/// "no ground truth," not as a false "expected empty string" that would
/// score a non-empty answer as wrong.
fn extraction_ground_truth(extraction: &synthpass_core::Extraction) -> HashMap<CoreField, String> {
    // Sized by the schema, not by a literal: a fixed `10` here compiled
    // unchanged when `CoreField` grew, and the new fields would simply never
    // have had ground truth. Now a missing entry is a type error.
    let fields: [(CoreField, &Option<String>); CoreField::ALL.len()] = [
        (CoreField::DocumentType, &extraction.document_type),
        (CoreField::IssuingCountry, &extraction.issuing_country),
        (CoreField::DocumentNumber, &extraction.document_number),
        (CoreField::Surname, &extraction.surname),
        (CoreField::GivenNames, &extraction.given_names),
        (CoreField::Nationality, &extraction.nationality),
        (CoreField::DateOfBirth, &extraction.date_of_birth),
        (CoreField::Sex, &extraction.sex),
        (CoreField::DateOfExpiry, &extraction.date_of_expiry),
        (CoreField::PersonalNumber, &extraction.personal_number),
        (CoreField::OptionalData1, &extraction.optional_data_1),
        (CoreField::OptionalData2, &extraction.optional_data_2),
    ];
    fields
        .into_iter()
        .filter_map(|(field, value)| match value.as_deref() {
            Some(v) if !v.is_empty() => Some((field, v.to_string())),
            _ => None,
        })
        .collect()
}

/// One OCR'd document, ready for the reader loop: the OCR text/geometry,
/// this document's optional per-field ground truth, and the on-disk path its
/// image was written to.
///
/// `image_path` is deliberately kept alive rather than cleaned up the moment
/// OCR finishes (contrast [`crate::check_document`], which deletes its temp
/// file immediately after OCR): [`DocumentContext::with_image`] needs a live
/// `&Path` for every reader that sees this document, and readers are visited
/// in the *outer* loop of [`run_prepped`] (once per provider, each pass over
/// every document) rather than once per document — so the path has to
/// outlive the whole run, not just one OCR call. [`run_prepped`] removes
/// every `image_path` after the last reader has finished with it.
struct BenchPage {
    /// Corpus-local identity, carried for `DocumentDetail`: a synthetic
    /// document's seed or a real specimen's file stem.
    name: String,
    /// Explicit real-asset identity for cross-provider joins; synthetic rows have none.
    asset_id: Option<String>,
    source_sha256: Option<String>,
    page: OcrPage,
    ground_truth: Option<HashMap<CoreField, String>>,
    /// The hand-transcribed true printed MRZ zone (`Extraction::mrz_line` from
    /// `samples/ocr_fixtures/<stem>.json`), newline-joined, when this specimen
    /// is labelled. Only real specimens carry one; synthetic documents leave
    /// this `None`. Used solely to sub-classify a `checksum_failed` miss.
    ground_truth_mrz: Option<String>,
    image_path: PathBuf,
    /// Whether `mrz::find_and_parse` recovered *any* MRZ from this document's
    /// OCR text — parsed, not necessarily checksum-valid.
    ///
    /// This is the "zero anchor vs some anchor" split, and it is deliberately
    /// the found/not-found line rather than the checksum-valid/invalid one.
    /// It mirrors the routing distinction the pipeline already draws:
    /// `EscalationKind::MrzNotFound` versus `MrzChecksumFailed`
    /// (`synthpass_die::RoutingPolicy::decide`, whose clause order is
    /// documented as normative for exactly this reason). A checksum-partial
    /// read still pins several fields mathematically; no MRZ at all pins
    /// nothing, and that is the population where a text-only provider has
    /// nothing to be right about — which is what
    /// [`UnsupportedAssertion`]'s split exists to measure separately.
    mrz_found: bool,
    /// `mrz::find_and_parse`/`find_and_parse_with` refused every candidate
    /// because its document number's first cell was the filler
    /// (`mrz::MrzError::LeadingFiller`, #536), rather than finding nothing
    /// MRZ-shaped at all. Captured alongside `mrz_found` (both come from the
    /// same call, which `mrz_found` collapses to `.is_ok()`) so
    /// [`run_prepped`]'s miss classification can report
    /// `MissReason::DocumentNumberLeadingFiller` instead of the generic
    /// `MissReason::NoMrzFound`. Always `false` when `mrz_found` is `true`.
    document_number_leading_filler: bool,
    /// The specimen's MRZ zone is redacted in the image (the `*_redacted_mrz`
    /// filename tag). Whatever OCR read there comes from the redaction bar, not
    /// a real zone — classified `MissReason::Redacted` and excluded from the
    /// Tier-1 hit-rate denominator, never scored as a `checksum_failed` miss.
    /// Always `false` for synthetic-corpus documents.
    redacted: bool,
    /// Whether this document carries an MRZ at all
    /// ([`crate::RealSpecimenDoc::mrz_expected`]). Always `true` for synthetic
    /// documents — `synthpass-gen` draws the zone itself, so there is always
    /// one to find.
    mrz_expected: bool,
    /// The specimen's own **printed** MRZ fails its ICAO check digits — a
    /// `TEMPLATE`/`ÖRNEK`/`VZOR`/all-zeros zone, or one that is structurally
    /// malformed. Computed from the hand-transcribed `mrz_line` ground truth,
    /// so it is a property of the document rather than of the run: no OCR
    /// pipeline, however good, can return a checksum-valid record for one of
    /// these, and the 2026-09-08 `checksum_failed` writeup concluded that all
    /// 16 of them "belong outside the denominator".
    ///
    /// Deliberately **not** "OCR recovered the printed zone exactly", which is
    /// what this used to be. That conflated a fact about the specimen with an
    /// achievement of the run, and it recognised 1 of the 16 — the other 15
    /// were non-conforming *and* imperfectly read, so they were scored as
    /// ordinary OCR failures against a target they could never hit.
    ///
    /// `false` for an unlabelled specimen: with no transcription there is no
    /// evidence the printed zone is bad, and assuming it would quietly excuse
    /// real misreads.
    printed_zone_nonconforming: bool,
    /// `true` for a synthetic-corpus document (from [`prep_corpus`]), `false`
    /// for a real specimen (from [`prep_specimens`]) — which of
    /// [`DocumentDetail::mrz_format`]'s two resolution rules applies.
    synthetic: bool,
    /// Precomputed format info [`run_prepped`] resolves into
    /// [`DocumentDetail::mrz_format`]:
    /// - synthetic: always `Some`, the exact `Labels::mrz_format`.
    /// - real specimens: a best-effort guess from the ground-truth
    ///   `mrz_line` (`MrzFormat::guess_from_lines`), used only when no
    ///   provider's own Tier-1 read resolves a format for this document.
    known_or_guessed_format: Option<&'static str>,
    /// Wall-clock time of this document's `recognize_detailed` call. Measured
    /// once in [`prep_specimens`]/[`prep_corpus`] and shared by every reader,
    /// the same way the OCR text is.
    ocr_elapsed: Duration,
    /// Every OCR pass, when a pass trace exists for this document: a replay, or a live run
    /// with `--dump-ocr-passes`; `None` otherwise (the benchmark adds no tracing of its own,
    /// which would change timing under the retry loop's budget). Used to say which passes read
    /// a line-1 proposal ([`line1_selection_detail`], through [`pass_provenance`]) and, under
    /// ADR-0024, written to the archive's `ocr_passes`. **Document text.**
    pass_objects: Option<Vec<crate::ocr_passes::PassObject>>,
}

/// Process-wide sequence keeps duplicate display names and concurrent runs distinct.
fn temporary_image_path() -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "provider-bench-{}-{sequence}.png",
        std::process::id()
    ))
}

/// Writes `image` to a uniquely-named temp file and OCRs it via
/// `recognize_detailed`, returning both the OCR result and the path — the
/// caller owns cleanup (see [`BenchPage`]'s doc for why the path can't be
/// removed here). A process-wide sequence disambiguates documents and concurrent
/// runs within one process; the process ID separates concurrently running processes.
///
/// The returned [`Duration`] times `recognize_detailed` alone, not the temp
/// file write, so it measures the OCR pipeline rather than the disk. It is
/// where a real-specimen run spends almost all of its time, and until it was
/// recorded nothing said which documents that time went to (ADR-0010, step 5).
///
/// With `trace` `Some`, the OCR call is `recognize_detailed_traced`, which
/// returns the same page and fills the vector with one record per executed
/// pass (ADR-0024, amendment 1); with `None` it is `recognize_detailed`.
fn ocr_and_keep_path(
    ocr: &NativeOcr,
    image: &image::DynamicImage,
    trace: Option<&mut Vec<synthpass_ocr::PassRecord>>,
) -> Result<(OcrPage, PathBuf, Duration), String> {
    let path = temporary_image_path();
    image
        .save(&path)
        .map_err(|e| format!("failed to write temp image: {e}"))?;
    let started = Instant::now();
    let result = match trace {
        Some(records) => ocr.recognize_detailed_traced(&path).map(|(page, passes)| {
            *records = passes;
            page
        }),
        None => ocr.recognize_detailed(&path),
    };
    let ocr_elapsed = started.elapsed();
    match result {
        Ok(page) => Ok((page, path, ocr_elapsed)),
        Err(e) => {
            let _ = std::fs::remove_file(&path);
            Err(e)
        }
    }
}

/// OCRs every document in the synthetic `corpus`, pairing each with its
/// always-present ground truth (derived from `Labels`' MRZ lines, parsed back
/// through the per-format dispatcher [`crate::parse_ground_truth_mrz`] the
/// same way [`crate::check_document`] does — TD1/TD2/TD3 alike, not TD3
/// only). `None` at an index means that document's OCR or ground-truth parse
/// failed — carried as a hole rather than shrinking the `Vec`, so the
/// original document count is still recoverable if a caller wants it.
fn prep_corpus(ocr: &NativeOcr, corpus: &[CorpusDoc], progress: bool) -> Vec<Option<BenchPage>> {
    let total = corpus.len();
    corpus
        .iter()
        .enumerate()
        .map(|(i, doc)| {
            if progress {
                eprintln!("[ocr {}/{total}] {}", i + 1, doc.seed);
            }
            let (page, image_path, ocr_elapsed) = ocr_and_keep_path(ocr, &doc.image, None).ok()?;
            let truth = crate::parse_ground_truth_mrz(&doc.labels).ok()?;
            // Read from the OCR text, never from `labels`: the question is
            // what this run's OCR pass actually recovered, which is what a
            // provider had to work with. A generated document always *has*
            // an MRZ, but a degraded capture can leave none of it legible.
            let found = mrz::find_and_parse_with(&page.text, &synthpass_die::mrz_parse_options());
            let mrz_found = found.is_ok();
            let document_number_leading_filler =
                matches!(found, Err(mrz::MrzError::LeadingFiller { .. }));
            Some(BenchPage {
                name: doc.seed.to_string(),
                asset_id: None,
                source_sha256: None,
                page,
                ground_truth: Some(mrz_ground_truth(&truth)),
                ground_truth_mrz: None,
                image_path,
                mrz_found,
                document_number_leading_filler,
                redacted: false,
                // `synthpass-gen` drew the MRZ, so there is always one to find.
                mrz_expected: true,
                printed_zone_nonconforming: false,
                synthetic: true,
                known_or_guessed_format: Some(doc.labels.mrz_format.as_str()),
                ocr_elapsed,
                pass_objects: None,
            })
        })
        .collect()
}

/// OCRs every document in `specimens`, pairing each with its optional
/// ground truth (`None` when the specimen had no `samples/ocr_fixtures/
/// <stem>.json` label). Unlike [`prep_corpus`], a missing label is not a
/// reason to drop the document — only an OCR failure is.
///
/// `passes`, when `Some`, receives one [`OcrPassesRow`](crate::ocr_passes::OcrPassesRow)
/// per OCR'd document. This is the only place the rows are made, and this runs
/// once per run, so the rows are per document however many providers read the
/// documents afterwards.
fn prep_specimens(
    ocr: &NativeOcr,
    specimens: &[RealSpecimenDoc],
    progress: bool,
    mut passes: Option<&mut crate::ocr_passes::OcrPassesCollector>,
) -> Vec<Option<BenchPage>> {
    let total = specimens.len();
    specimens
        .iter()
        .enumerate()
        .map(|(i, doc)| {
            if progress {
                eprintln!("[ocr {}/{total}] {}", i + 1, doc.name);
            }
            let mut records = Vec::new();
            let (page, image_path, ocr_elapsed) =
                ocr_and_keep_path(ocr, &doc.image, passes.is_some().then_some(&mut records))
                    .ok()?;
            let traced = passes.is_some();
            let pass_objects = crate::ocr_passes::pass_objects(&records);
            if let Some(collector) = passes.as_deref_mut() {
                collector.push(
                    &doc.name,
                    Some(&doc.asset_id),
                    Some(&doc.source_sha256),
                    &page,
                    &records,
                );
            }
            let mut bench_page = specimen_bench_page(doc, page, image_path, ocr_elapsed);
            bench_page.pass_objects = traced.then_some(pass_objects);
            Some(bench_page)
        })
        .collect()
}

/// The parse options for the printed-zone validity parse: the arm's options
/// with the repeated-line refusal and the date-digits rule forced off (#579).
///
/// That parse decides whether the *document's* printed zone conforms, which
/// selects the `checksum_failed_specimen` denominator rung. A refusal, or a
/// letter-date rule, is a property of a run's reading, not of the document, so
/// neither may move the denominator between arms.
fn printed_zone_parse_options() -> mrz::ParseOptions {
    printed_zone_parse_options_from(synthpass_die::mrz_parse_options())
}

/// [`printed_zone_parse_options`] over explicit options, testable without the
/// process environment.
fn printed_zone_parse_options_from(opts: mrz::ParseOptions) -> mrz::ParseOptions {
    opts.with_refuse_repeated_line(false)
        .with_date_digits(false)
}

/// One real specimen's [`BenchPage`], from its corpus document and the OCR page
/// of its run.
///
/// The one place a specimen's page is built, shared by the live prep
/// ([`prep_specimens`]) and the replay ([`replay_pages`]), so the two cannot
/// disagree about anything that is not the OCR. `mrz_found` and
/// `document_number_leading_filler` are recomputed from `page.text` here, which
/// is why a replay needs only the text, not those two values.
fn specimen_bench_page(
    doc: &RealSpecimenDoc,
    page: OcrPage,
    image_path: PathBuf,
    ocr_elapsed: Duration,
) -> BenchPage {
    let ground_truth = doc.labels.as_ref().map(extraction_ground_truth);
    // The hand-transcribed true printed MRZ zone, when this specimen
    // has a `samples/ocr_fixtures/<stem>.json` label. `run_prepped`
    // compares the run's recovered zone against it to tell a genuine
    // OCR misread apart from a specimen whose *printed* check digits
    // are wrong by design.
    let ground_truth_mrz = doc.labels.as_ref().and_then(|l| l.mrz_line.clone());
    // Does the *printed* zone pass its own check digits? Parsed from
    // the hand transcription, not from OCR, so the answer describes the
    // document and stays the same whatever the run reads off it.
    // `None` (unlabelled) is not evidence of non-conformance — see
    // `BenchPage::printed_zone_nonconforming`.
    let printed_zone_nonconforming = ground_truth_mrz.as_deref().is_some_and(|zone| {
        !mrz::find_and_parse_with(zone, &printed_zone_parse_options()).is_ok_and(|d| d.valid())
    });
    let found = mrz::find_and_parse(&page.text);
    let mrz_found = found.is_ok();
    let document_number_leading_filler = matches!(found, Err(mrz::MrzError::LeadingFiller { .. }));
    // Derived from the filename, the same way the corpus manifest
    // generator records `mrz.redacted` (`corpus_manifest.rs`). The
    // bench never reads `samples/corpus.jsonl` — it walks the image
    // directory — so the stem is the signal it has.
    let redacted = doc.name.to_ascii_lowercase().contains("redacted");
    // Best-effort fallback only — used in `run_prepped` when no
    // provider's own Tier-1 read resolves a format for this
    // document. Reuses `MrzFormat::guess_from_lines` rather than a
    // new heuristic; `None` when there is no ground-truth `mrz_line`
    // to guess from at all (most of `samples/`, by construction).
    let known_or_guessed_format = doc
        .labels
        .as_ref()
        .and_then(|l| l.mrz_line.as_deref())
        .and_then(MrzFormat::guess_from_lines)
        .map(mrz_format_str);
    BenchPage {
        name: doc.name.clone(),
        asset_id: Some(doc.asset_id.clone()),
        source_sha256: Some(doc.source_sha256.clone()),
        page,
        ground_truth,
        ground_truth_mrz,
        image_path,
        mrz_found,
        document_number_leading_filler,
        redacted,
        // From `samples/corpus.jsonl`'s `mrz.present`, resolved at load
        // time — unlike `redacted`, which the stem can carry on its own.
        // The manifest is the signal here because two driving-license
        // fronts predate the `_no_mrz` naming convention entirely.
        mrz_expected: doc.mrz_expected,
        printed_zone_nonconforming,
        synthetic: false,
        known_or_guessed_format,
        ocr_elapsed,
        pass_objects: None,
    }
}

/// Runs every reader in `catalog` against every document in `corpus`,
/// OCRing each document once and sharing that OCR pass across all readers —
/// the comparison is only fair if every provider sees identical input.
/// Ground truth is always present (`CorpusDoc`'s contract) — every document
/// contributes to `AccuracyStats::labelled_documents`.
pub async fn run_provider_bench(
    catalog: &ProviderCatalog,
    ocr: &NativeOcr,
    corpus: &[CorpusDoc],
    measure_memory: bool,
    progress: bool,
    archive: Option<&Archive>,
) -> Vec<ProviderReport> {
    let prepped = prep_corpus(ocr, corpus, progress);
    run_prepped_with_dump_options(
        catalog,
        &prepped,
        measure_memory,
        None,
        false,
        None,
        progress,
        archive,
    )
    .await
}

/// Runs every reader in `catalog` against every real specimen in
/// `specimens`, the same way [`run_provider_bench`] does for the synthetic
/// corpus, except ground truth is present only for specimens with a
/// `samples/ocr_fixtures/<stem>.json` label — see [`AccuracyStats`]'s doc
/// for how that is reported honestly rather than scored as misses.
pub async fn run_provider_bench_real(
    catalog: &ProviderCatalog,
    ocr: &NativeOcr,
    specimens: &[RealSpecimenDoc],
    measure_memory: bool,
    dump_ocr_dir: Option<&Path>,
    progress: bool,
) -> Vec<ProviderReport> {
    run_provider_bench_real_with_dump_options(
        catalog,
        ocr,
        specimens,
        measure_memory,
        dump_ocr_dir,
        false,
        progress,
    )
    .await
}

pub async fn run_provider_bench_real_with_dump_options(
    catalog: &ProviderCatalog,
    ocr: &NativeOcr,
    specimens: &[RealSpecimenDoc],
    measure_memory: bool,
    dump_ocr_dir: Option<&Path>,
    dump_ocr_hits: bool,
    progress: bool,
) -> Vec<ProviderReport> {
    let run_manifest = current_run_manifest(dump_ocr_dir);
    let prepped = prep_specimens(ocr, specimens, progress, None);
    run_prepped_with_dump_options(
        catalog,
        &prepped,
        measure_memory,
        dump_ocr_dir,
        dump_ocr_hits,
        run_manifest.as_deref(),
        progress,
        None,
    )
    .await
}

/// The run manifest file name the CLI wrote next to the dumps, if any. The CLI
/// writes this pointer immediately before the run; reading it here keeps the
/// public benchmark function signatures free of it.
fn current_run_manifest(dir: Option<&Path>) -> Option<String> {
    dir.and_then(|dir| {
        std::fs::read_to_string(dir.join("provider-bench-ocr-current-run.txt"))
            .ok()
            .filter(|name| {
                !name.is_empty() && !name.chars().any(|c| matches!(c, '/' | '\\' | '\n' | '\r'))
            })
    })
}

/// The opt-in dumps a real-specimen run can write, next to the `--out` report.
#[derive(Debug, Clone, Copy, Default)]
pub struct RealDumpOptions<'a> {
    /// `--dump-ocr`: directory for the miss OCR dump.
    pub ocr_dir: Option<&'a Path>,
    /// `--dump-ocr-hits`: add Tier-1 hits to that dump.
    pub ocr_hits: bool,
    /// `--dump-ocr-passes`: directory for
    /// [`OCR_PASSES_FILENAME`](crate::ocr_passes::OCR_PASSES_FILENAME). The
    /// caller has already checked the destination
    /// ([`crate::ocr_passes::check_passes_destination`]) and refused the
    /// private track.
    pub ocr_passes_dir: Option<&'a Path>,
    /// The per-document archive (ADR-0024), when this run writes one. A side output: it
    /// changes nothing else, and no run reads it.
    pub archive: Option<&'a Archive>,
}

/// [`run_provider_bench_real_with_dump_options`] with the full set of dumps.
///
/// With `dumps.ocr_passes_dir` set, every document is OCR'd through the traced
/// entry point, which returns the same page as the untraced one, and one row
/// per OCR'd document is written to `provider-bench-ocr-passes.jsonl` right
/// after the corpus prep — once, before any provider reads a document. The
/// readings never reach a [`ProviderReport`], [`DocumentDetail`] or the outcome
/// ledger. `Err` only when that file cannot be written, and it is returned
/// before any provider runs.
pub async fn run_provider_bench_real_with_options(
    catalog: &ProviderCatalog,
    ocr: &NativeOcr,
    specimens: &[RealSpecimenDoc],
    measure_memory: bool,
    dumps: &RealDumpOptions<'_>,
    progress: bool,
) -> Result<Vec<ProviderReport>, String> {
    let run_manifest = current_run_manifest(dumps.ocr_dir.or(dumps.ocr_passes_dir));
    let mut passes = dumps
        .ocr_passes_dir
        .map(|_| crate::ocr_passes::OcrPassesCollector::new(run_manifest.clone()));
    let prepped = prep_specimens(ocr, specimens, progress, passes.as_mut());
    if let (Some(dir), Some(collector)) = (dumps.ocr_passes_dir, passes.as_ref()) {
        // The path and a count only: the readings are document OCR.
        let (path, rows) = collector.write(dir).inspect_err(|_| {
            // `run_prepped` removes these after the last reader; nobody will.
            for bench_page in prepped.iter().flatten() {
                let _ = std::fs::remove_file(&bench_page.image_path);
            }
        })?;
        eprintln!(
            "OCR pass readings written to {} ({rows} rows)",
            path.display()
        );
    }
    Ok(run_prepped_with_dump_options(
        catalog,
        &prepped,
        measure_memory,
        dumps.ocr_dir,
        dumps.ocr_hits,
        run_manifest.as_deref(),
        progress,
        dumps.archive,
    )
    .await)
}

/// The `OcrPage` a captured row stands for: the text and every value scoring,
/// the report, the ledger or a dump reads from the page. The rest of `OcrPage`
/// (lines, band box, portrait, text sanity) is read by none of them and stays at
/// its default.
fn replayed_page(row: &crate::ocr_passes::OcrPassesRow) -> OcrPage {
    OcrPage {
        text: row.ocr_text.clone(),
        rotation: row.rotation,
        mrz_band_score: row.mrz_band_score,
        retry_variant_id: row.retry_variant_id.clone(),
        retry_damaged_recovery: row.retry_damaged_recovery,
        retry_budget_hit: row.retry_budget_hit,
        retry_stop: row.retry_stop.clone(),
        chargrid: row.chargrid.clone(),
        ..OcrPage::default()
    }
}

/// At most five ids for an error message, then a count of the rest.
fn some_ids(ids: &[&str]) -> String {
    const SHOWN: usize = 5;
    let mut out = ids
        .iter()
        .take(SHOWN)
        .copied()
        .collect::<Vec<_>>()
        .join(", ");
    if ids.len() > SHOWN {
        out.push_str(&format!(", and {} more", ids.len() - SHOWN));
    }
    out
}

/// One page per corpus document, in corpus order, from `capture`'s rows: what a
/// replay scores. `Err` says why the capture cannot stand in for this corpus,
/// checked in this order:
///
/// 1. the rows lack a key the replay needs ([`PassesFile::missing_keys`]);
/// 2. the rows do not cover the corpus exactly once: a row with no `asset_id`,
///    two rows for one asset, a corpus document with no row, a row for an asset
///    the corpus does not hold;
/// 3. a row's `source_sha256` is not the SHA-256 of the corpus image's bytes
///    (`RealSpecimenDoc::source_sha256`): the capture read other bytes.
///
/// The corpus manifest's hash is the caller's to check; it is not on the rows.
/// Each page is built by [`specimen_bench_page`], the function the live prep
/// uses, so `mrz_found`, the ground truth and every flag derived from the
/// document are computed the same way; the page's own values come from the row
/// ([`replayed_page`]) and `ocr_elapsed` is zero, since no OCR ran. No image is
/// written: `image_path` names a file that does not exist, which no reader in
/// a `--mrz-only` run opens and which [`run_prepped`]'s cleanup ignores.
///
/// [`PassesFile::missing_keys`]: crate::ocr_passes::PassesFile::missing_keys
fn replay_pages(
    specimens: &[RealSpecimenDoc],
    capture: &crate::ocr_passes::PassesFile,
) -> Result<Vec<Option<BenchPage>>, String> {
    if !capture.missing_keys.is_empty() {
        return Err(format!(
            "the capture's rows lack {}: a replay needs {} to reproduce the live run's report; \
             recapture with a build that writes them",
            capture.missing_keys.join(", "),
            if capture.missing_keys.len() == 1 {
                "that key"
            } else {
                "those keys"
            },
        ));
    }

    let mut by_asset: BTreeMap<&str, &crate::ocr_passes::OcrPassesRow> = BTreeMap::new();
    let mut repeated: Vec<&str> = Vec::new();
    for (index, row) in capture.rows.iter().enumerate() {
        let Some(asset) = row.asset_id.as_deref() else {
            return Err(format!("capture row {} has no asset_id", index + 1));
        };
        if by_asset.insert(asset, row).is_some() && !repeated.contains(&asset) {
            repeated.push(asset);
        }
    }
    let corpus: std::collections::BTreeSet<&str> =
        specimens.iter().map(|doc| doc.asset_id.as_str()).collect();
    let without_row: Vec<&str> = corpus
        .iter()
        .copied()
        .filter(|asset| !by_asset.contains_key(asset))
        .collect();
    let outside_corpus: Vec<&str> = by_asset
        .keys()
        .copied()
        .filter(|asset| !corpus.contains(asset))
        .collect();
    let mut coverage: Vec<String> = Vec::new();
    if !repeated.is_empty() {
        coverage.push(format!(
            "{} asset(s) have more than one row: {}",
            repeated.len(),
            some_ids(&repeated)
        ));
    }
    if !without_row.is_empty() {
        coverage.push(format!(
            "{} corpus document(s) have no row: {}",
            without_row.len(),
            some_ids(&without_row)
        ));
    }
    if !outside_corpus.is_empty() {
        coverage.push(format!(
            "{} row(s) are for assets this corpus does not hold: {}",
            outside_corpus.len(),
            some_ids(&outside_corpus)
        ));
    }
    if !coverage.is_empty() {
        return Err(format!(
            "the capture does not cover the corpus exactly once ({}); a capture and its replay \
             must run over the same documents, with the same --format, --limit and --include-* \
             flags",
            coverage.join("; ")
        ));
    }

    let other_bytes: Vec<&str> = specimens
        .iter()
        .filter(|doc| {
            by_asset[doc.asset_id.as_str()].source_sha256.as_deref()
                != Some(doc.source_sha256.as_str())
        })
        .map(|doc| doc.asset_id.as_str())
        .collect();
    if !other_bytes.is_empty() {
        return Err(format!(
            "{} capture row(s) were read from image bytes this corpus does not hold \
             (source_sha256 differs): {}; the capture was measured on another samples revision",
            other_bytes.len(),
            some_ids(&other_bytes)
        ));
    }

    Ok(specimens
        .iter()
        .map(|doc| {
            let row = by_asset[doc.asset_id.as_str()];
            let mut bench_page = specimen_bench_page(
                doc,
                replayed_page(row),
                temporary_image_path(),
                Duration::ZERO,
            );
            bench_page.pass_objects = Some(row.ocr_passes.clone());
            Some(bench_page)
        })
        .collect())
}

/// Whether `capture` can stand in for `specimens`: [`replay_pages`]'s checks,
/// without running anything. Called before a replay writes a file, so a refused
/// replay leaves nothing behind.
pub fn check_replay(
    specimens: &[RealSpecimenDoc],
    capture: &crate::ocr_passes::PassesFile,
) -> Result<(), String> {
    replay_pages(specimens, capture).map(|_| ())
}

/// Runs every reader in `catalog` over the real specimens in `specimens` **without
/// OCR**, reading each document's page from `capture` (ADR-0024, amendment 3).
///
/// The scoring is [`run_prepped_with_dump_options`], the function a live run
/// uses, so the report, the outcome rows and the dumps come out of the same code;
/// only where the page came from differs. `dumps.ocr_dir` and `dumps.ocr_hits`
/// behave as in [`run_provider_bench_real_with_options`]. `dumps.ocr_passes_dir`
/// must be `None`: a replay reads the pass trace and writes none.
///
/// A replay measures only what happens to `OcrPage::text` and the values beside
/// it; OCR runtime and retry behaviour are the capture's. `Err` is
/// [`replay_pages`]'s refusal, returned before any reader runs.
pub async fn run_provider_bench_replay(
    catalog: &ProviderCatalog,
    specimens: &[RealSpecimenDoc],
    capture: &crate::ocr_passes::PassesFile,
    measure_memory: bool,
    dumps: &RealDumpOptions<'_>,
    progress: bool,
) -> Result<Vec<ProviderReport>, String> {
    if dumps.ocr_passes_dir.is_some() {
        return Err("a replay reads the pass file and writes none".to_string());
    }
    let prepped = replay_pages(specimens, capture)?;
    let run_manifest = current_run_manifest(dumps.ocr_dir);
    Ok(run_prepped_with_dump_options(
        catalog,
        &prepped,
        measure_memory,
        dumps.ocr_dir,
        dumps.ocr_hits,
        run_manifest.as_deref(),
        progress,
        dumps.archive,
    )
    .await)
}

/// The literal MRZ substring a date field's ISO value (`YYYY-MM-DD`) was
/// parsed from — `YYMMDD`, e.g. `"1974-08-12"` -> `"740812"`.
///
/// `synthpass_core::normalize` reformats every date field to ISO before it
/// reaches this harness (see `crates/synthpass-core/src/normalize.rs`), but
/// the OCR text a text-only provider actually read only ever contains the
/// raw MRZ digits (or a printed-page date in some other format) — never the
/// ISO string. Checking the verbatim ISO value against OCR text therefore
/// flags a *correct* date read as an unsupported assertion almost every
/// time, which is exactly the near-universal `date_of_birth`/`date_of_expiry`
/// pattern seen on real runs. Falling back to this raw digit string when the
/// direct match misses recovers the true positive without weakening the
/// check for any other field.
fn mrz_date_digits(iso_date: &str) -> Option<String> {
    let (year, rest) = iso_date.split_once('-')?;
    let (month, day) = rest.split_once('-')?;
    if year.len() != 4 || month.len() != 2 || day.len() != 2 {
        return None;
    }
    if !year.bytes().all(|b| b.is_ascii_digit())
        || !month.bytes().all(|b| b.is_ascii_digit())
        || !day.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some(format!("{}{month}{day}", &year[2..]))
}

/// Whether `value` (the field this provider asserted) is grounded in
/// `ocr_text` — a verbatim substring match for every field, plus a
/// raw-MRZ-digit fallback for date fields specifically (see
/// [`mrz_date_digits`]).
fn is_supported(field: CoreField, value: &str, ocr_text_lower: &str) -> bool {
    if ocr_text_lower.contains(&value.to_lowercase()) {
        return true;
    }
    if matches!(field, CoreField::DateOfBirth | CoreField::DateOfExpiry) {
        if let Some(digits) = mrz_date_digits(value) {
            return ocr_text_lower.contains(&digits);
        }
    }
    false
}

/// Whether `ground_truth` carries both name fields — the "name-scorable"
/// predicate [`StrictNameHitRate`]'s two denominators are built from.
/// `DocumentDetail::names_exact.is_some()` is equivalent and usually more
/// convenient once a `DocumentDetail` exists; this exists for the one call
/// site (an errored read) that has to decide the answer before one does.
fn has_name_ground_truth(ground_truth: Option<&HashMap<CoreField, String>>) -> bool {
    ground_truth.is_some_and(|gt| {
        gt.contains_key(&CoreField::Surname) && gt.contains_key(&CoreField::GivenNames)
    })
}

/// Whether `miss_reason` sits inside [`Tier1HitRate`]'s scored population —
/// the three off-denominator kinds (`no_mrz_expected`/`redacted_mrz`/
/// `checksum_failed_specimen`) can never yield a Tier-1 hit however good the
/// pipeline gets, so they are excluded the same way from *both*
/// `tier1_hit_rate` and [`StrictNameHitRate`]'s denominators. One predicate
/// so the two computations cannot silently diverge on what "scored" means —
/// see `run_prepped`'s `tier1_hit_rate` computation, and this list's own
/// "must stay identical to `RealSpecimenSnapshot`'s off-denominator set in
/// `report.rs`" note there.
fn in_scored_tier1_population(miss_reason: &Option<MissReason>) -> bool {
    !matches!(
        miss_reason,
        Some(MissReason::Redacted)
            | Some(MissReason::NoMrzExpected)
            | Some(MissReason::ChecksumFailed {
                specimen_nonconforming: true,
                ..
            })
    )
}

/// The shared reader loop both [`run_provider_bench`] and
/// [`run_provider_bench_real`] funnel into — the one place accuracy and
/// unsupported-assertion are computed, so the two corpus sources cannot
/// silently diverge in what "correct" or "unsupported" means.
///
/// `dump_ocr_dir`: when `Some`, for every document whose `miss_reason`
/// resolves to `MissReason::ChecksumFailed` or `MissReason::NoMrzFound`,
/// print — debug-quoted, so a misread or invisible character shows — the
/// full pre-parse OCR text, the MRZ band score, and (for `checksum_failed`)
/// the recovered/repaired MRZ zone with the check digit(s) that failed; and
/// append one row per such miss to
/// `<dir>/provider-bench-miss-ocr-dump.jsonl` so the population can be
/// analysed from a file rather than terminal scrollback. Mirrors
/// `synthpass-bench`'s own `--dump-ocr` (which prints the full OCR text for
/// *every* synthetic document), but scoped to the two in-denominator miss
/// kinds. Both are now clean signals: since the 2026-09-09 denominator
/// correction, `no_mrz_found` means "MRZ expected, none found" — a genuine
/// detection failure — because the MRZ-less documents that used to pollute
/// it are scored out as `no_mrz_expected` first. A real-specimen run is
/// large enough that dumping every *hit* would still be noise a synthetic
/// diagnostic run never has to contend with. A `*_redacted_mrz` specimen is
/// classified `MissReason::Redacted` before either gate, so it never reaches
/// this dump — its zone is a redaction bar, not recoverable OCR material.
/// `synthpass_bench::CorpusDoc` runs (`run_provider_bench`)
/// always pass `None` here — `synthpass-bench`'s own `--dump-ocr` already
/// covers that path. The JSONL carries specimen OCR content and is written
/// only under `/artifacts/` (`.gitignore`d); it never enters the `--out`
/// trend report, which stays shape-only.
///
/// `progress`: print one stderr line per document as it completes, tagged with
/// the provider and a `n/total` counter. A full real-specimen run takes over
/// an hour and was otherwise completely silent until the final report; stderr
/// keeps it clear of the stdout summary and the `--out` JSON that
/// `scripts/run-bench.ps1` consumes.
#[cfg(test)]
async fn run_prepped(
    catalog: &ProviderCatalog,
    prepped: &[Option<BenchPage>],
    measure_memory: bool,
    dump_ocr_dir: Option<&Path>,
    progress: bool,
) -> Vec<ProviderReport> {
    run_prepped_with_dump_options(
        catalog,
        prepped,
        measure_memory,
        dump_ocr_dir,
        false,
        None,
        progress,
        None,
    )
    .await
}

/// [`run_prepped`] with the OCR dump flags and the archive.
///
/// `archive`, when `Some`, receives one record per document per provider, written right
/// after the document's [`DocumentDetail`] is pushed (the error arm and the success arm are
/// its only two ends) and flushed at each provider's end. It is a side output: nothing this
/// loop computes reads it or changes because of it, and it can only warn (ADR-0024,
/// Decision 1).
#[allow(clippy::too_many_arguments)]
async fn run_prepped_with_dump_options(
    catalog: &ProviderCatalog,
    prepped: &[Option<BenchPage>],
    measure_memory: bool,
    dump_ocr_dir: Option<&Path>,
    dump_ocr_hits: bool,
    run_manifest: Option<&str>,
    progress: bool,
    archive: Option<&Archive>,
) -> Vec<ProviderReport> {
    let ocr_documents = prepped.iter().filter(|p| p.is_some()).count();
    let labelled_documents = prepped
        .iter()
        .filter(|p| matches!(p, Some(bp) if bp.ground_truth.is_some()))
        .count();
    // Counted over documents, not per reader: the split is a property of the
    // corpus, identical for every provider, so counting it once also makes it
    // impossible for two providers to disagree about how many anchored
    // documents they saw.
    let anchored_documents = prepped
        .iter()
        .filter(|p| matches!(p, Some(bp) if bp.mrz_found))
        .count();
    let unanchored_documents = prepped
        .iter()
        .filter(|p| matches!(p, Some(bp) if !bp.mrz_found))
        .count();

    let mut reports = Vec::with_capacity(catalog.readers().len());
    // Accumulated across every provider (each tagged in the row) and written
    // once after the loop — see `dump_ocr_dir` in this fn's doc.
    let mut dump_rows: Vec<MissOcrDump> = Vec::new();
    for reader in catalog.readers() {
        let capability = reader.capability();
        let rss_before = measure_memory.then(sample_rss).flatten();
        let repair_before = synthpass_llm::repair::repair_fallbacks();

        let mut elapsed_per_doc = Vec::with_capacity(prepped.len());
        let mut field_hits = 0usize;
        let mut field_total = 0usize;
        let mut cer_sum = 0.0f64;
        let mut cer_count = 0usize;
        // The two quoted populations (#564); see `AccuracyStats`.
        let mut accepted_reads = FieldTally::new();
        let mut scored = FieldTally::new();
        let mut per_field: Vec<(&'static str, f64, usize)> = CoreField::ALL
            .iter()
            .map(|f| (f.as_str(), 0.0, 0))
            .collect();
        let mut assertions_total = 0usize;
        let mut assertions_unsupported = 0usize;
        // Same two counters again, restricted to documents with / without an
        // MRZ anchor. Accumulated separately rather than derived afterwards
        // because an assertion belongs to the document it was made about, and
        // that association is only available here inside the loop.
        let mut documents_detail: Vec<DocumentDetail> = Vec::with_capacity(prepped.len());
        let mut anchored_total = 0usize;
        let mut anchored_unsupported = 0usize;
        let mut unanchored_total = 0usize;
        let mut unanchored_unsupported = 0usize;

        if progress {
            eprintln!(
                "== provider {} ({ocr_documents} documents) ==",
                reader.id().as_str()
            );
        }

        for (doc_index, bench_page) in prepped.iter().flatten().enumerate() {
            let tier1 = synthpass_die::read_tier1(&bench_page.page.text);
            let read_mrz = tier1.parsed.as_ref().ok();
            // `read_mrz` is the reader's Tier-1 read under this process's
            // arms, so the hint, the dump zone, the check states and
            // `tier1_damaged_recovery` all describe one read (ADR-0024 step 0b).
            let hint = synthpass_pipeline::mrz_hint(read_mrz);
            let dump_zone: Option<&mrz::MrzData> = read_mrz;
            // `with_image`: harmless for every provider shipped today (all
            // text-only, so `DocumentContext::image` is ignored), and the
            // reason this harness is also the substrate for the planned
            // vision-provider comparison — see this file's top doc comment.
            let mut ctx = DocumentContext::from_text(&bench_page.page.text)
                .with_image(&bench_page.image_path);
            if let Some(hint) = &hint {
                ctx = ctx.with_mrz_hint(hint);
            }
            let started = Instant::now();
            let reading = reader.read(&ctx).await;
            let elapsed = started.elapsed();
            elapsed_per_doc.push(elapsed);

            // See `DocumentDetail::mrz_format`'s doc and `resolve_mrz_format`'s
            // own: synthetic documents always resolve to the label's exact
            // format, independent of whether this read even succeeded; real
            // specimens take this provider's own Tier-1 read
            // (`Evidence::mrz_format`, populated only by the deterministic MRZ
            // provider) only when that read is checksum-valid, and otherwise
            // fall back to the ground-truth guess computed once in
            // `prep_specimens` (#555 — a checksum-invalid read is no more
            // trustworthy about the format than about any other field).
            let resolve_format = |evidence: Option<&Evidence>| -> Option<&'static str> {
                resolve_mrz_format(
                    bench_page.synthetic,
                    bench_page.known_or_guessed_format,
                    evidence,
                )
            };

            let reading = match reading {
                Ok(reading) => reading,
                Err(e) => {
                    // Recorded rather than skipped silently: a provider that
                    // errored on a document contributed nothing to any
                    // aggregate, and a per-document view that simply omitted
                    // the row would make that indistinguishable from a
                    // document it handled cleanly with no assertions.
                    let error_comparison = compare_document(bench_page.ground_truth.as_ref(), None);
                    documents_detail.push(DocumentDetail {
                        name: bench_page.name.clone(),
                        asset_id: bench_page.asset_id.clone(),
                        mrz_found: bench_page.mrz_found,
                        mrz_format: resolve_format(None),
                        read_ok: false,
                        mrz_checksums_valid: false,
                        check_states: None,
                        miss_reason: Some(MissReason::OcrError(e.to_string())),
                        assertions_total: 0,
                        assertions_unsupported: 0,
                        unsupported_fields: Vec::new(),
                        // `Some(false)` when this document is name-scorable:
                        // an error is not exact, and it is not the same fact
                        // as "nothing to measure" — see
                        // `DocumentDetail::names_exact`'s doc. There is no
                        // read to classify the shape of, so `name_error`
                        // stays `None` either way.
                        names_exact: has_name_ground_truth(bench_page.ground_truth.as_ref())
                            .then_some(false),
                        name_error: None,
                        // No read, so no accepted read: every field with
                        // truth is `Unread`.
                        field_correctness: error_comparison.field_correctness(false),
                        ocr_elapsed: bench_page.ocr_elapsed,
                        retry_variant_id: bench_page.page.retry_variant_id.clone(),
                        retry_damaged_recovery: bench_page.page.retry_damaged_recovery,
                        retry_budget_hit: bench_page.page.retry_budget_hit,
                        retry_stop: bench_page.page.retry_stop.clone(),
                        chargrid: bench_page.page.chargrid.clone(),
                        tier1_damaged_recovery: read_mrz.map(|d| d.damaged_recovery),
                        // No reading, so no selection was recorded.
                        line1_selection: None,
                    });
                    if let (Some(archive), Some(detail)) = (archive, documents_detail.last()) {
                        // No read: the fields are null; the truth comparison still
                        // describes the Tier-1 zone against the fixture.
                        let truth_cmp = truth_comparison(
                            dump_zone,
                            bench_page.ground_truth_mrz.as_deref(),
                            detail.mrz_format,
                        );
                        archive_document(
                            archive,
                            reader.id().as_str(),
                            bench_page,
                            detail,
                            elapsed,
                            dump_zone,
                            None,
                            truth_cmp.as_ref(),
                        );
                    }
                    // `OcrError` sits inside the scored population: a reader
                    // that errored read nothing, so end-to-end counts every
                    // field this document has truth for as absent. It is not
                    // an accepted read.
                    scored.add_document(&error_comparison.tally);
                    if progress {
                        eprintln!(
                            "[{} {}/{ocr_documents}] {} MISS ocr_error ({elapsed:.1?})",
                            reader.id().as_str(),
                            doc_index + 1,
                            bench_page.name,
                        );
                    }
                    continue;
                }
            };
            let mrz_format = resolve_format(Some(&reading.evidence));

            let mut doc_assertions = 0usize;
            let mut doc_unsupported_fields: Vec<&'static str> = Vec::new();
            // Identical for every field in the loop below — computed once
            // per document rather than once per assertion.
            let ocr_text_lower = bench_page.page.text.to_lowercase();
            // Each field this document has truth for, compared once. The
            // tally goes to the populations once `miss_reason` is known, and
            // the same comparisons make `DocumentDetail::field_correctness`,
            // so the per-document map cannot disagree with the aggregates.
            let comparison = compare_document(
                bench_page.ground_truth.as_ref(),
                Some(&reading.extraction.fields),
            );
            // Field-match / CER: only over fields this document has ground
            // truth for — see `AccuracyStats`'s doc on why an absent label
            // must not be scored as a miss. Summed in `CoreField::ALL` order,
            // as the comparisons are made.
            for &(i, exact, field_cer) in &comparison.tally {
                field_total += 1;
                field_hits += usize::from(exact);
                cer_sum += field_cer;
                cer_count += 1;
                per_field[i].1 += field_cer;
                per_field[i].2 += 1;
            }

            for field in CoreField::ALL.iter() {
                let got = reading.extraction.fields.get(*field);

                // Unsupported-assertion: needs no ground truth at all — only
                // the OCR text this provider was given — but is only a
                // meaningful predicate for a text-only provider. See
                // `UnsupportedAssertion`'s doc.
                if !capability.vision {
                    if let Some(value) = got {
                        if !value.is_empty() {
                            let unsupported = !is_supported(*field, value, &ocr_text_lower);
                            assertions_total += 1;
                            if unsupported {
                                assertions_unsupported += 1;
                            }
                            let (total, bad) = if bench_page.mrz_found {
                                (&mut anchored_total, &mut anchored_unsupported)
                            } else {
                                (&mut unanchored_total, &mut unanchored_unsupported)
                            };
                            *total += 1;
                            if unsupported {
                                *bad += 1;
                            }

                            doc_assertions += 1;
                            if unsupported {
                                // The field *name*, never `value` — see
                                // `DocumentDetail`'s doc on why the asserted
                                // value must not reach a log or a report.
                                doc_unsupported_fields.push(field.as_str());
                            }
                        }
                    }
                }
            }

            // Mirrors `run_check`'s miss classification in `lib.rs`: no MRZ
            // found, then checksum validity, then — only when this document is
            // labelled — a document-number check against ground truth. A
            // specimen with no label that clears those gets no further check;
            // that is a hit, not an unknown.
            //
            // The rungs are ordered by **what the document makes possible**,
            // before anything about what the run achieved. Three populations
            // cannot produce a Tier-1 hit no matter how good the pipeline gets,
            // and each is scored off the denominator rather than counted as a
            // failure to do the impossible:
            //
            //   * `no_mrz_expected` — the document carries no zone at all.
            //   * `redacted_mrz`    — the zone is physically blacked out.
            //   * `checksum_failed_specimen` — the *printed* zone fails its own
            //     ICAO check digits, so even a byte-perfect read fails.
            //
            // Getting that order wrong is what this whole classification was
            // rebuilt for. Every one of the three used to be decided by what OCR
            // happened to return:
            //
            //   * An MRZ-less front has no `ocr_fixtures/` label to contradict,
            //     so a hallucinated checksum-valid zone reached the final `else`,
            //     found no ground truth, and was counted as a Tier-1 **hit** —
            //     while the 42 correct refusals were counted as `no_mrz_found`.
            //   * `Redacted` sat *after* the `mrz_found` gate, so 9 of the 36
            //     redacted specimens were scored out and 27 were scored in as
            //     detection failures, split by nothing but whether the blackout
            //     bar happened to OCR into parseable noise. The cleaner the
            //     redaction, the worse the document scored.
            //   * A non-conforming printed zone was only recognised when OCR
            //     recovered it *exactly*, which caught 1 of the 16 the
            //     2026-09-08 writeup identified.
            let miss_reason = if !bench_page.mrz_expected {
                if bench_page.mrz_found && reading.evidence.mrz_checksums_valid {
                    Some(MissReason::FalsePositiveMrz)
                } else {
                    // Includes the parseable-but-checksum-failing case: the
                    // check digits rejected it, which is the system working.
                    Some(MissReason::NoMrzExpected)
                }
            } else if bench_page.redacted {
                // Before the `mrz_found` gate, not after. Whether a blackout bar
                // resolves into something parseable is a property of the bar's
                // texture, not of the pipeline, and it is not stable run to run.
                Some(MissReason::Redacted)
            } else if bench_page.printed_zone_nonconforming {
                // The printed zone fails its own check digits, so this document
                // has no reachable hit. Reported whatever OCR returned, for the
                // same reason as `redacted`: the document decides this, not the
                // run.
                Some(MissReason::ChecksumFailed {
                    check_states: read_mrz
                        .map(|d| crate::check_states(&d.checks))
                        .unwrap_or_default(),
                    specimen_nonconforming: true,
                })
            } else if bench_page.document_number_leading_filler {
                // A structural refusal (#536), not "nothing MRZ-shaped was
                // found" — its own bucket rather than folding into the
                // `NoMrzFound` rung below.
                Some(MissReason::DocumentNumberLeadingFiller)
            } else if !bench_page.mrz_found {
                Some(MissReason::NoMrzFound(String::new()))
            } else if !reading.evidence.mrz_checksums_valid {
                // A genuine OCR misread: the printed zone is conforming (the
                // rung above already took the specimens where it is not), so
                // every failing check digit here is the pipeline's own.
                //
                // `read_mrz` is the reader's Tier-1 read, the same source
                // `--dump-ocr` prints below, so this check state describes
                // that read.
                Some(MissReason::ChecksumFailed {
                    check_states: read_mrz
                        .map(|d| crate::check_states(&d.checks))
                        .unwrap_or_default(),
                    specimen_nonconforming: false,
                })
            } else {
                bench_page
                    .ground_truth
                    .as_ref()
                    .and_then(|gt| gt.get(&CoreField::DocumentNumber))
                    .and_then(|expected| {
                        let got = reading
                            .extraction
                            .fields
                            .get(CoreField::DocumentNumber)
                            .unwrap_or("");
                        (got != expected).then(|| MissReason::DocumentNumberMismatch {
                            got: got.to_string(),
                            expected: expected.clone(),
                        })
                    })
            };

            // One predicate each, shared with the rates they sit beside, so
            // "accepted" and "scored" cannot drift from `accepted_reads` and
            // `tier1_hit_rate`.
            let accepted = crate::is_accepted_read(miss_reason.as_ref());
            if accepted {
                accepted_reads.add_document(&comparison.tally);
            }
            if in_scored_tier1_population(&miss_reason) {
                scored.add_document(&comparison.tally);
            }

            // The comparison against the hand-transcribed zone, once per labelled
            // document: the dump below and the archive record both read it.
            let truth_cmp = truth_comparison(
                dump_zone,
                bench_page.ground_truth_mrz.as_deref(),
                mrz_format,
            );
            let dump_miss_kind = match &miss_reason {
                Some(
                    r @ (MissReason::ChecksumFailed { .. }
                    | MissReason::NoMrzFound(_)
                    | MissReason::DocumentNumberLeadingFiller),
                ) => Some(miss_kind(r)),
                _ => None,
            };
            let dump_hit = dump_ocr_hits && miss_reason.is_none();
            if dump_ocr_dir.is_some() && (dump_miss_kind.is_some() || dump_hit) {
                let provider = reader.id().as_str();

                // The full OCR text the provider actually consumed — this is
                // what `synthpass-bench --dump-ocr` prints for synthetic
                // misses, and what makes a systematic shift or dropped filler
                // visible next to the recovered zone below.
                println!("--- {} ({provider}) raw OCR text ---", bench_page.name);
                if bench_page.page.text.is_empty() {
                    println!("  (OCR returned no text)");
                } else {
                    for (i, line) in bench_page.page.text.lines().enumerate() {
                        println!("  [{i}] {line:?}");
                    }
                }
                match bench_page.page.mrz_band_score {
                    Some(s) => println!("  mrz band score: {s:.3}"),
                    None => println!("  mrz band score: (none)"),
                }

                println!(
                    "--- {} ({provider}) recovered MRZ zone ---",
                    bench_page.name
                );
                let (recovered, check_states): (
                    Vec<String>,
                    Option<BTreeMap<String, Option<bool>>>,
                ) = match dump_zone {
                    Some(data) => {
                        let recovered: Vec<String> =
                            data.mrz_lines.lines().map(str::to_string).collect();
                        for (i, line) in recovered.iter().enumerate() {
                            println!("  [{i}] {line:?}");
                        }
                        let failing: BTreeMap<String, Option<bool>> =
                            crate::check_states(&data.checks)
                                .into_iter()
                                .map(|(field, state)| (field.to_string(), state))
                                .collect();
                        let failed: Vec<_> = failing
                            .iter()
                            .filter_map(|(field, state)| {
                                (*state == Some(false)).then_some(field.as_str())
                            })
                            .collect();
                        println!("  failed check digit(s): {}", failed.join(", "));
                        (recovered, Some(failing))
                    }
                    // `no_mrz_found`: nothing MRZ-shaped parsed, so there is
                    // no zone to show — the band score and raw text above are
                    // the whole story. (Also the "shouldn't happen" case where
                    // `mrz_found` is true but `find_and_parse` recovered
                    // nothing — same output, and just as informative.)
                    None => {
                        println!("  (nothing MRZ-shaped parsed)");
                        (Vec::new(), None)
                    }
                };

                let compared_cells = truth_cmp.as_ref().and_then(|t| t.compared_cells);
                let zone_mismatch = truth_cmp.as_ref().and_then(|t| t.zone_mismatch);
                // Field-level attribution needs a resolved format (to know
                // which table to look positions up against) on top of
                // `zone_mismatch`'s own two preconditions — see
                // `MissOcrDump::field_mismatch_counts`'s doc.
                let field_mismatch = truth_cmp.as_ref().and_then(|t| t.field_mismatch.as_ref());
                // Printed alongside the zone above so a run stopped before the
                // final JSONL flush (the usual `llm`-pass kill) still carries the
                // ground-truth comparison for every labelled specimen.
                if let Some(truth) = &bench_page.ground_truth_mrz {
                    println!("  ground-truth MRZ zone (hand-transcribed):");
                    for (i, line) in truth.lines().enumerate() {
                        println!("    [{i}] {line:?}");
                    }
                    match zone_mismatch {
                        Some(0) => println!(
                            "  zone mismatch vs ground truth: 0 \
                             (printed zone recovered faithfully — specimen is non-conforming)"
                        ),
                        Some(n) => println!(
                            "  zone mismatch vs ground truth: {n} character(s) \
                             (OCR introduced the error)"
                        ),
                        None => {}
                    }
                    if let Some(fm) = field_mismatch {
                        println!("  per-field mismatch: {:?}", fm.by_field);
                        println!("  per-field coverage: {:?}", fm.coverage);
                    }
                }
                dump_rows.push(MissOcrDump {
                    name: bench_page.name.clone(),
                    asset_id: bench_page.asset_id.clone(),
                    source_sha256: bench_page.source_sha256.clone(),
                    run_manifest: run_manifest.map(str::to_string),
                    miss_reason: dump_miss_kind,
                    provider: provider.to_string(),
                    mrz_format: mrz_format.map(str::to_string),
                    mrz_band_score: bench_page.page.mrz_band_score,
                    raw_ocr_text: bench_page.page.text.clone(),
                    recovered_mrz_lines: recovered,
                    check_states,
                    ground_truth_mrz: bench_page.ground_truth_mrz.clone(),
                    zone_mismatch,
                    field_mismatch_counts: field_mismatch.map(|f| f.by_field.clone()),
                    field_mismatch_positions: field_mismatch.map(|f| f.by_line.clone()),
                    field_mismatch_coverage: field_mismatch.map(|f| f.coverage.clone()),
                    compared_cells,
                });
            }

            if progress {
                // `miss_reason` is `None` exactly when this document counted as
                // a hit — the same predicate `miss_kind` reports on below, read
                // here before the value is moved into `documents_detail`.
                let verdict = match &miss_reason {
                    None => "HIT".to_string(),
                    Some(reason) => format!("MISS {}", miss_kind(reason)),
                };
                eprintln!(
                    "[{} {}/{ocr_documents}] {} {verdict} ({elapsed:.1?})",
                    reader.id().as_str(),
                    doc_index + 1,
                    bench_page.name,
                );
            }

            // Scored only when ground truth carries both name fields — an
            // absent field (most real specimens: only `document_number` is
            // hand-verified on many) must read as "not scored", never as a
            // fabricated miss. See `DocumentDetail::names_exact`'s doc.
            let (names_exact, name_error) = match bench_page.ground_truth.as_ref().and_then(|gt| {
                Some((
                    gt.get(&CoreField::Surname)?,
                    gt.get(&CoreField::GivenNames)?,
                ))
            }) {
                Some((truth_surname, truth_given)) => {
                    let got_surname = reading
                        .extraction
                        .fields
                        .get(CoreField::Surname)
                        .unwrap_or("");
                    let got_given = reading
                        .extraction
                        .fields
                        .get(CoreField::GivenNames)
                        .unwrap_or("");
                    let name_error =
                        classify_names(truth_surname, truth_given, got_surname, got_given);
                    (
                        Some(name_error.is_none()),
                        name_error.map(NameError::as_str),
                    )
                }
                None => (None, None),
            };
            let check_states = read_mrz.map(|data| crate::check_states(&data.checks));

            documents_detail.push(DocumentDetail {
                name: bench_page.name.clone(),
                asset_id: bench_page.asset_id.clone(),
                mrz_found: bench_page.mrz_found,
                mrz_format,
                read_ok: true,
                mrz_checksums_valid: reading.evidence.mrz_checksums_valid,
                check_states,
                miss_reason,
                assertions_total: doc_assertions,
                assertions_unsupported: doc_unsupported_fields.len(),
                unsupported_fields: doc_unsupported_fields,
                names_exact,
                name_error,
                field_correctness: comparison.field_correctness(accepted),
                ocr_elapsed: bench_page.ocr_elapsed,
                retry_variant_id: bench_page.page.retry_variant_id.clone(),
                retry_damaged_recovery: bench_page.page.retry_damaged_recovery,
                retry_budget_hit: bench_page.page.retry_budget_hit,
                retry_stop: bench_page.page.retry_stop.clone(),
                chargrid: bench_page.page.chargrid.clone(),
                tier1_damaged_recovery: read_mrz.map(|d| d.damaged_recovery),
                // From the reading's own evidence, so a provider that made no
                // Tier-1 read records none; the proposal's line 1 is looked up
                // in the pass trace, when there is one, and never stored.
                line1_selection: reading.evidence.line1_selection.as_ref().map(|summary| {
                    line1_selection_detail(
                        summary,
                        tier1.proposed_line1.as_deref(),
                        &bench_page
                            .pass_objects
                            .as_deref()
                            .map(pass_provenance)
                            .unwrap_or_default(),
                    )
                }),
            });
            if let (Some(archive), Some(detail)) = (archive, documents_detail.last()) {
                let fields = CoreField::ALL
                    .iter()
                    .map(|field| {
                        (
                            field.as_str(),
                            reading.extraction.fields.get(*field).map(str::to_string),
                        )
                    })
                    .collect();
                archive_document(
                    archive,
                    reader.id().as_str(),
                    bench_page,
                    detail,
                    elapsed,
                    dump_zone,
                    Some(fields),
                    truth_cmp.as_ref(),
                );
            }
        }

        // The provider's records are on disk before the next provider starts, so a run
        // killed in a later provider (the `llm` pass) still leaves this one's.
        if let Some(archive) = archive {
            archive.flush();
        }
        let repair_after = synthpass_llm::repair::repair_fallbacks();
        let rss_after = measure_memory.then(sample_rss).flatten();

        elapsed_per_doc.sort();
        let speed = SpeedStats {
            mean: mean_duration(&elapsed_per_doc),
            p50: percentile_duration(&elapsed_per_doc, 0.50),
            p95: percentile_duration(&elapsed_per_doc, 0.95),
        };

        let unsupported_assertion = if capability.vision {
            UnsupportedAssertion::NotApplicable {
                reason: VISION_REASON,
            }
        } else {
            UnsupportedAssertion::Computed {
                overall: AssertionBucket::new(
                    assertions_unsupported,
                    assertions_total,
                    ocr_documents,
                )
                // An empty corpus is the one case with no documents at all,
                // so there is no rate to report and nothing was asserted.
                .unwrap_or(AssertionBucket {
                    rate: None,
                    assertions_total: 0,
                    documents: 0,
                }),
                with_mrz_anchor: AssertionBucket::new(
                    anchored_unsupported,
                    anchored_total,
                    anchored_documents,
                ),
                without_mrz_anchor: AssertionBucket::new(
                    unanchored_unsupported,
                    unanchored_total,
                    unanchored_documents,
                ),
            }
        };

        let tier1_hit_rate = if !capability.deterministic {
            Tier1HitRate::NotApplicable {
                reason: NOT_DETERMINISTIC_REASON,
            }
        } else {
            // Three populations are neither a hit nor a fair miss, and each is
            // excluded from the denominator the same way an unlabelled document
            // is excluded from field accuracy — because no pipeline, however
            // good, can turn one into a Tier-1 hit:
            //
            //   * `no_mrz_expected`          — no zone on the document at all.
            //   * `redacted_mrz`             — the zone is blacked out.
            //   * `checksum_failed_specimen` — the printed zone fails its own
            //                                  ICAO check digits.
            //
            // They still appear in `documents_detail` and in the "misses by
            // kind" table; they just do not cap the achievable rate. What is
            // left is the population where a miss is genuinely ours.
            //
            // This list must stay identical to `RealSpecimenSnapshot`'s
            // off-denominator set in `report.rs` — they are two
            // computations of the same number, and a divergence would put the
            // reported hit rate and the committed baseline quietly at odds.
            // `in_scored_tier1_population` is also `StrictNameHitRate`'s own
            // denominator's starting population, below — one predicate, so
            // the two computations cannot silently diverge on what "scored"
            // means.
            let scored = documents_detail
                .iter()
                .filter(|d| in_scored_tier1_population(&d.miss_reason))
                .count();
            let tier1_hits = documents_detail
                .iter()
                .filter(|d| d.miss_reason.is_none())
                .count();
            if scored == 0 {
                Tier1HitRate::NotApplicable {
                    reason: "no documents in the scored Tier-1 population (denominator is zero)",
                }
            } else {
                Tier1HitRate::Computed(tier1_hits as f64 / scored as f64)
            }
        };

        let strict_name_hit_rate = match &tier1_hit_rate {
            Tier1HitRate::NotApplicable { reason } => StrictNameHitRate::NotApplicable { reason },
            Tier1HitRate::Computed(_) => {
                // Name-scorable: in the scored Tier-1 population (see
                // `in_scored_tier1_population`'s doc) *and* ground truth has
                // both name fields (`names_exact.is_some()` — true whether
                // the document ended up a hit, a miss, or an errored read;
                // see `DocumentDetail::names_exact`'s doc). This is
                // `strict_tier1_hit_rate`'s denominator.
                let name_scorable: Vec<&DocumentDetail> = documents_detail
                    .iter()
                    .filter(|d| {
                        in_scored_tier1_population(&d.miss_reason) && d.names_exact.is_some()
                    })
                    .collect();
                let name_scorable_documents = name_scorable.len();
                // Of those, the ones that were also a Tier-1 hit —
                // `names_exact_among_hits`'s denominator.
                let name_scorable_hits = name_scorable
                    .iter()
                    .filter(|d| d.miss_reason.is_none())
                    .count();
                let strict_hits = name_scorable
                    .iter()
                    .filter(|d| d.miss_reason.is_none() && d.names_exact == Some(true))
                    .count();
                if name_scorable_documents == 0 {
                    StrictNameHitRate::NotApplicable {
                        reason: NO_NAME_SCORABLE_DOCUMENTS_REASON,
                    }
                } else {
                    StrictNameHitRate::Computed {
                        strict_hits,
                        name_scorable_documents,
                        name_scorable_hits,
                        strict_tier1_hit_rate: strict_hits as f64 / name_scorable_documents as f64,
                        names_exact_among_hits: (name_scorable_hits > 0)
                            .then(|| strict_hits as f64 / name_scorable_hits as f64),
                    }
                }
            }
        };

        reports.push(ProviderReport {
            provider_id: reader.id().as_str().to_string(),
            documents: ocr_documents,
            capability: CapabilitySnapshot::from(capability),
            accuracy: AccuracyStats {
                labelled_documents,
                field_match_rate: (field_total > 0).then(|| field_hits as f64 / field_total as f64),
                mean_cer: (cer_count > 0).then(|| cer_sum / cer_count as f64),
                per_field_cer: per_field
                    .into_iter()
                    .map(|(name, sum, n)| (name, (n > 0).then(|| sum / n as f64), n))
                    .collect(),
                accepted_reads: accepted_reads.finish(),
                scored: scored.finish(),
            },
            speed,
            json_validity: (!capability.deterministic).then(|| JsonValidityStats {
                repair_fallbacks: repair_after.saturating_sub(repair_before),
                documents: ocr_documents,
            }),
            unsupported_assertion,
            declared_resident_bytes: capability.estimated_resident_bytes,
            documents_detail,
            tier1_hit_rate,
            ocr_arms: synthpass_ocr::OcrArms::from_env(),
            strict_tier1_hit_rate: strict_name_hit_rate,
            measured_rss_delta_bytes: match (rss_before, rss_after) {
                (Some(before), Some(after)) => Some(after as i64 - before as i64),
                _ => None,
            },
        });
    }

    // Every reader has now seen every document — safe to remove the temp
    // images `prep_corpus`/`prep_specimens` wrote and kept alive for
    // `DocumentContext::with_image`.
    for bench_page in prepped.iter().flatten() {
        let _ = std::fs::remove_file(&bench_page.image_path);
    }

    if let Some(dir) = dump_ocr_dir {
        if dump_rows.is_empty() {
            println!("no checksum_failed or no_mrz_found misses to dump");
        } else {
            let path = dir.join("provider-bench-miss-ocr-dump.jsonl");
            let mut body = String::new();
            for row in &dump_rows {
                body.push_str(&serde_json::to_string(row).expect("serialize dump row"));
                body.push('\n');
            }
            match std::fs::create_dir_all(dir).and_then(|_| std::fs::write(&path, &body)) {
                Ok(()) => {
                    let specimen = dump_rows
                        .iter()
                        .filter(|r| r.zone_mismatch == Some(0))
                        .count();
                    let ocr_misread = dump_rows
                        .iter()
                        .filter(|r| r.zone_mismatch.is_some_and(|n| n > 0))
                        .count();
                    println!(
                        "OCR dump written to {} ({} rows; \
                         {specimen} labelled specimen-non-conforming, \
                         {ocr_misread} labelled OCR-misread)",
                        path.display(),
                        dump_rows.len(),
                    );
                }
                Err(e) => eprintln!("warning: could not write {}: {e}", path.display()),
            }
        }
    }

    reports
}

/// Mean of `sorted`, or [`Duration::ZERO`] when it is empty.
pub fn mean_duration(sorted: &[Duration]) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    sorted.iter().sum::<Duration>() / sorted.len() as u32
}

/// `sorted` must already be sorted ascending. `p` in `[0, 1]`.
pub fn percentile_duration(sorted: &[Duration], p: f64) -> Duration {
    if sorted.is_empty() {
        return Duration::ZERO;
    }
    let idx = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[idx.min(sorted.len() - 1)]
}

#[cfg(feature = "measure-memory")]
fn sample_rss() -> Option<u64> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
    let mut system = System::new();
    let pid = Pid::from_u32(std::process::id());
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::everything(),
    );
    system.process(pid).map(|p| p.memory())
}

#[cfg(not(feature = "measure-memory"))]
fn sample_rss() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    #[test]
    fn temporary_images_are_unique_across_concurrent_runs() {
        // Each run can contain identical display names: naming no longer uses them.
        let runs: Vec<_> = (0..8)
            .map(|_| {
                std::thread::spawn(|| {
                    (0..32)
                        .map(|_| super::temporary_image_path())
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        let paths: Vec<_> = runs
            .into_iter()
            .flat_map(|run| run.join().unwrap())
            .collect();
        let unique: std::collections::HashSet<_> = paths.iter().collect();
        assert_eq!(unique.len(), 8 * 32);
        assert!(paths
            .iter()
            .all(|path| path.parent() == Some(std::env::temp_dir().as_path())
                && path.extension().is_some_and(|ext| ext == "png")));
    }

    use super::*;
    use synthpass_core::v2::ExtractionV2;
    use synthpass_die::{FieldReader, IntelligenceProvider, ProviderError, ProviderId, Reading};

    // `resolve_mrz_format` (#555): a real specimen's `mrz_format` must come
    // from the provider's own read only when that read is checksum-valid.

    /// Builds an `Evidence` naming a resolved MRZ format and its checksum
    /// validity — the two fields `resolve_mrz_format` reads — leaving every
    /// other field at its `Default`. Struct-literal syntax can't do this
    /// directly: `Evidence` is `#[non_exhaustive]`, even within this crate.
    fn evidence_with_format(format: MrzFormat, checksums_valid: bool) -> Evidence {
        let mut evidence = Evidence::default();
        evidence.mrz_format = Some(format);
        evidence.mrz_checksums_valid = checksums_valid;
        evidence
    }

    #[test]
    fn checksum_invalid_real_specimen_read_falls_back_to_the_guess() {
        let checksum_invalid = evidence_with_format(MrzFormat::MrvB, false);
        assert_eq!(
            resolve_mrz_format(false, Some("TD3"), Some(&checksum_invalid)),
            Some("TD3"),
            "a checksum-invalid read must not override the ground-truth guess, \
             even though it did resolve a format"
        );
    }

    #[test]
    fn checksum_invalid_real_specimen_read_with_no_guess_is_unresolved() {
        let checksum_invalid = evidence_with_format(MrzFormat::MrvB, false);
        assert_eq!(
            resolve_mrz_format(false, None, Some(&checksum_invalid)),
            None,
            "with no ground-truth guess to fall back to, a checksum-invalid \
             read must resolve to unresolved, not the read's own format"
        );
    }

    #[test]
    fn checksum_valid_real_specimen_read_wins_even_against_a_disagreeing_guess() {
        let checksum_valid = evidence_with_format(MrzFormat::Td3, true);
        assert_eq!(
            resolve_mrz_format(false, Some("TD1"), Some(&checksum_valid)),
            Some("TD3"),
            "a checksum-valid read is trusted over the ground-truth guess, \
             even when they disagree"
        );
    }

    #[test]
    fn synthetic_document_keeps_the_label_format_whatever_the_read_says() {
        let checksum_valid_but_wrong = evidence_with_format(MrzFormat::MrvB, true);
        assert_eq!(
            resolve_mrz_format(true, Some("TD1"), Some(&checksum_valid_but_wrong)),
            Some("TD1"),
            "a synthetic document's label always wins, independent of the read"
        );
        assert_eq!(
            resolve_mrz_format(true, Some("TD1"), None),
            Some("TD1"),
            "a synthetic document's label wins even when there is no read at all"
        );
    }

    fn rate_test_page() -> BenchPage {
        BenchPage {
            asset_id: None,
            source_sha256: None,
            name: "synthetic-rate-test".into(),
            page: OcrPage::default(),
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("unused-rate-test.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: false,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
            pass_objects: None,
        }
    }

    #[tokio::test]
    async fn undefined_rates_all_documents_off_denominator() {
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(std::sync::Arc::new(FixedReader::default()))
            .build()
            .unwrap();
        // One of each exclusion, with no labelled fields and no assertions.
        let mut redacted = rate_test_page();
        redacted.mrz_expected = true;
        redacted.redacted = true;
        let mut nonconforming = rate_test_page();
        nonconforming.mrz_expected = true;
        nonconforming.printed_zone_nonconforming = true;
        let pages = vec![Some(rate_test_page()), Some(redacted), Some(nonconforming)];
        let report = run_prepped(&catalog, &pages, false, None, false)
            .await
            .remove(0);
        assert!(report.tier1_hit_rate.to_string().starts_with("n/a"));
        let json = serde_json::to_value(crate::report::ProviderRow::from(report)).unwrap();
        assert_eq!(json["documents"], 3);
        for pointer in [
            "/tier1_hit_rate/rate",
            "/strict_tier1_hit_rate/strict_tier1_hit_rate",
            "/strict_tier1_hit_rate/names_exact_among_hits",
        ] {
            assert_eq!(json.pointer(pointer), Some(&serde_json::Value::Null));
        }
        assert_eq!(json["tier1_hit_rate"]["rate"], serde_json::Value::Null);
        assert_eq!(
            json["strict_tier1_hit_rate"]["strict_tier1_hit_rate"],
            serde_json::Value::Null
        );
        assert_eq!(
            json["strict_tier1_hit_rate"]["names_exact_among_hits"],
            serde_json::Value::Null
        );
        assert!(json["accuracy"]["field_match_rate"].is_null());
        assert!(json["accuracy"]["mean_cer"].is_null());
        assert!(json["accuracy"]["per_field_cer"]
            .as_array()
            .unwrap()
            .iter()
            .all(|field| field["mean_cer"].is_null()));
        assert!(json["unsupported_assertion"]["overall"]["rate"].is_null());
        assert!(json["unsupported_assertion"]["without_mrz_anchor"]["rate"].is_null());
    }

    #[tokio::test]
    async fn undefined_rates_name_scorable_miss_has_no_hits_denominator() {
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(std::sync::Arc::new(FixedReader::default()))
            .build()
            .unwrap();
        let mut page = rate_test_page();
        page.mrz_expected = true;
        page.ground_truth = Some(HashMap::from([
            (CoreField::Surname, "DOE".into()),
            (CoreField::GivenNames, "JANE".into()),
        ]));
        let report = run_prepped(&catalog, &[Some(page)], false, None, false)
            .await
            .remove(0);
        let json = serde_json::to_value(crate::report::ProviderRow::from(report)).unwrap();
        assert_eq!(json["tier1_hit_rate"]["rate"], 0.0);
        assert_eq!(json["strict_tier1_hit_rate"]["strict_tier1_hit_rate"], 0.0);
        assert!(json["strict_tier1_hit_rate"]["names_exact_among_hits"].is_null());
    }

    #[tokio::test]
    async fn undefined_rates_empty_provider_run_has_no_repair_denominator() {
        let mut capability = Capability::deterministic_reader();
        capability.deterministic = false;
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(std::sync::Arc::new(FixedReader {
                capability,
                ..FixedReader::default()
            }))
            .build()
            .unwrap();
        let report = run_prepped(&catalog, &[], false, None, false)
            .await
            .remove(0);
        let json = serde_json::to_value(crate::report::ProviderRow::from(report)).unwrap();
        assert_eq!(json["json_validity"]["documents"], 0);
        assert!(json["json_validity"]["repair_fallback_rate"].is_null());
    }

    /// A fixed-answer provider for testing the harness's own comparison
    /// logic without needing a real MRZ reader or the GGUF model.
    struct FixedReader {
        capability: Capability,
        surname: &'static str,
        /// Defaults to `""` (`Default for FixedReader`, below) for every test
        /// that predates the name-accuracy metric and only ever cared about
        /// `surname` — an empty string never contributes an assertion (see
        /// `run_prepped`'s `!value.is_empty()` gate) and is never scored
        /// against ground truth unless a test's `BenchPage::ground_truth`
        /// actually carries `CoreField::GivenNames`, so those tests are
        /// unaffected by this field's addition.
        given_names: &'static str,
        /// The `Evidence` every `read` returns. `Evidence::default()` for most
        /// tests (nothing proven); a test that needs the provider to look like
        /// a successful deterministic read sets `mrz_found` /
        /// `mrz_checksums_valid` here.
        evidence: Evidence,
    }

    impl Default for FixedReader {
        fn default() -> Self {
            Self {
                capability: Capability::deterministic_reader(),
                surname: "",
                given_names: "",
                evidence: Evidence::default(),
            }
        }
    }

    #[async_trait::async_trait]
    impl IntelligenceProvider for FixedReader {
        fn id(&self) -> ProviderId {
            ProviderId("fixed-test-reader")
        }
        fn capability(&self) -> &Capability {
            &self.capability
        }
        fn describe(&self) -> String {
            "fixed-answer test reader".into()
        }
    }

    #[async_trait::async_trait]
    impl FieldReader for FixedReader {
        async fn read(&self, _ctx: &DocumentContext<'_>) -> Result<Reading, ProviderError> {
            // `ExtractionV2` implements `Drop` (it zeroizes PII on drop), so
            // struct-update syntax (`..Default::default()`) is unavailable —
            // that would require partially moving out of a `Drop` type. A
            // field assignment on an owned `mut` value has no such
            // restriction.
            let mut extraction = ExtractionV2::default();
            extraction.fields.surname = Some(self.surname.to_string());
            extraction.fields.given_names = Some(self.given_names.to_string());
            Ok(Reading {
                extraction,
                evidence: self.evidence.clone(),
                by: self.id(),
            })
        }
    }

    #[test]
    fn unsupported_assertion_is_flagged_when_value_is_absent_from_input() {
        let ocr_text = "some markdown mentioning DOE but not the other name";
        let echoed = "DOE";
        let fabricated = "SMITH";
        assert!(ocr_text.to_lowercase().contains(&echoed.to_lowercase()));
        assert!(!ocr_text.to_lowercase().contains(&fabricated.to_lowercase()));
    }

    #[test]
    fn mrz_date_digits_converts_iso_to_the_printed_mrz_substring() {
        assert_eq!(mrz_date_digits("1974-08-12"), Some("740812".to_string()));
        assert_eq!(mrz_date_digits("2005-01-09"), Some("050109".to_string()));
    }

    #[test]
    fn mrz_date_digits_rejects_non_iso_input() {
        assert_eq!(mrz_date_digits("12.08.1974"), None);
        assert_eq!(mrz_date_digits(""), None);
        assert_eq!(mrz_date_digits("DOE"), None);
    }

    #[test]
    fn a_correct_iso_date_absent_from_ocr_text_is_still_supported_via_mrz_digits() {
        // The OCR text carries the raw MRZ line (YYMMDD), never the ISO
        // string synthpass_core::normalize reformats it to -- this is the
        // false-positive the near-universal date_of_birth/date_of_expiry
        // unsupported-assertion pattern on real runs was coming from.
        let ocr_text_lower = "p<utoDOE<<JOHN<<<<<<<<<<<<<<<<<<<<<<<<<<<<<\n\
            l898902c3utO7408122m1204159<<<<<<<<<<<<<<06"
            .to_lowercase();
        assert!(is_supported(
            CoreField::DateOfBirth,
            "1974-08-12",
            &ocr_text_lower
        ));
    }

    #[test]
    fn a_fabricated_date_is_still_unsupported() {
        let ocr_text_lower = "l898902c3uto7408122m1204159<<<<<<<<<<<<<<06".to_lowercase();
        assert!(!is_supported(
            CoreField::DateOfBirth,
            "1999-12-31",
            &ocr_text_lower
        ));
    }

    #[test]
    fn the_mrz_digit_fallback_is_scoped_to_date_fields_only() {
        // "740812" happening to be a substring of a non-date field's OCR
        // text must not make an unrelated fabricated value "supported" --
        // the fallback only ever applies to DateOfBirth/DateOfExpiry.
        let ocr_text_lower = "document number 740812 issued".to_lowercase();
        assert!(!is_supported(
            CoreField::DocumentNumber,
            "1974-08-12",
            &ocr_text_lower
        ));
    }

    #[tokio::test]
    async fn field_reader_answering_a_value_present_in_input_is_not_unsupported() {
        let reader = FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence: Evidence::default(),
        };
        let ctx = DocumentContext::from_text("surname DOE date of birth 1990");
        let reading = reader.read(&ctx).await.expect("reader never fails");
        let surname = reading.extraction.fields.get(CoreField::Surname).unwrap();
        assert!(ctx.text.to_lowercase().contains(&surname.to_lowercase()));
    }

    #[tokio::test]
    async fn field_reader_answering_a_fabricated_value_is_unsupported() {
        let reader = FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "SMITH",
            given_names: "",
            evidence: Evidence::default(),
        };
        let ctx = DocumentContext::from_text("surname DOE date of birth 1990");
        let reading = reader.read(&ctx).await.expect("reader never fails");
        let surname = reading.extraction.fields.get(CoreField::Surname).unwrap();
        assert!(!ctx.text.to_lowercase().contains(&surname.to_lowercase()));
    }

    #[test]
    fn percentile_duration_picks_the_right_index() {
        let sorted = vec![
            Duration::from_millis(10),
            Duration::from_millis(20),
            Duration::from_millis(30),
            Duration::from_millis(40),
            Duration::from_millis(50),
        ];
        assert_eq!(percentile_duration(&sorted, 0.0), Duration::from_millis(10));
        assert_eq!(percentile_duration(&sorted, 1.0), Duration::from_millis(50));
        assert_eq!(percentile_duration(&sorted, 0.5), Duration::from_millis(30));
    }

    #[test]
    fn mean_duration_of_empty_slice_is_zero() {
        assert_eq!(mean_duration(&[]), Duration::ZERO);
    }

    /// The crux of 1.2: ground truth is present for one document and absent
    /// for another, and `run_prepped` must score only the labelled one — an
    /// unlabelled document contributes zero field comparisons, not a wrong
    /// answer on every field.
    #[tokio::test]
    async fn unlabelled_documents_are_excluded_from_accuracy_not_scored_as_misses() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let mut labelled_truth = HashMap::new();
        labelled_truth.insert(CoreField::Surname, "DOE".to_string());
        let prepped = vec![
            Some(BenchPage {
                pass_objects: None,
                asset_id: None,
                source_sha256: None,
                name: "fixture".to_string(),
                page: OcrPage {
                    text: "surname DOE".to_string(),
                    ..OcrPage::default()
                },
                ground_truth: Some(labelled_truth),
                ground_truth_mrz: None,
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
                mrz_found: false,
                document_number_leading_filler: false,
                redacted: false,
                mrz_expected: true,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
            }),
            Some(BenchPage {
                pass_objects: None,
                asset_id: None,
                source_sha256: None,
                name: "fixture".to_string(),
                page: OcrPage {
                    text: "surname DOE, no label file for this one".to_string(),
                    ..OcrPage::default()
                },
                ground_truth: None,
                ground_truth_mrz: None,
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test-2.png"),
                mrz_found: false,
                document_number_leading_filler: false,
                redacted: false,
                mrz_expected: true,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
            }),
        ];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let report = &reports[0];
        assert_eq!(report.accuracy.labelled_documents, 1);
        assert_eq!(report.accuracy.field_match_rate, Some(1.0));
    }

    /// #564: read quality is over accepted reads, end-to-end is over the
    /// scored population, and a non-conforming specimen is in neither.
    #[tokio::test]
    async fn field_accuracy_populations_follow_the_accepted_and_scored_predicates() {
        let mut evidence = Evidence::default();
        evidence.mrz_checksums_valid = true;
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence,
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("one reader");
        let page = |truth: &[(CoreField, &str)]| {
            let mut page = rate_test_page();
            page.mrz_expected = true;
            page.mrz_found = true;
            page.ground_truth = Some(
                truth
                    .iter()
                    .map(|(field, value)| (*field, value.to_string()))
                    .collect(),
            );
            page
        };
        // A hit.
        let hit = page(&[(CoreField::Surname, "DOE")]);
        // Checksum-valid, but the reader returns no document number: an
        // accepted read that the scorer files as a document-number mismatch.
        let mismatch = page(&[
            (CoreField::Surname, "DOE"),
            (CoreField::DocumentNumber, "L898902C3"),
        ]);
        // A scored miss: nothing MRZ-shaped was found.
        let mut unread = page(&[(CoreField::Surname, "SMITH")]);
        unread.mrz_found = false;
        // A printed zone that fails its own check digits.
        let mut nonconforming = page(&[(CoreField::Surname, "DOE")]);
        nonconforming.printed_zone_nonconforming = true;

        let reports = run_prepped(
            &catalog,
            &[Some(hit), Some(mismatch), Some(unread), Some(nonconforming)],
            false,
            None,
            false,
        )
        .await;
        let accuracy = &reports[0].accuracy;

        // All labelled, kept for continuity: 3 of 5 comparisons exact.
        assert_eq!(accuracy.labelled_documents, 4);
        assert_eq!(accuracy.field_match_rate, Some(0.6));

        // Read quality: the hit and the mismatch, whose missing document
        // number counts against it.
        let accepted = &accuracy.accepted_reads;
        assert_eq!(accepted.documents, 2);
        assert_eq!(accepted.field_match_rate, Some(2.0 / 3.0));
        let document_number = accepted
            .per_field
            .iter()
            .find(|field| field.field == "document_number")
            .expect("document_number");
        assert_eq!(document_number.documents, 1);
        assert_eq!(document_number.match_rate, Some(0.0));
        assert_eq!(document_number.mean_cer, Some(1.0));

        // End-to-end: the hit, the mismatch and the unread miss, but not the
        // non-conforming specimen.
        let scored = &accuracy.scored;
        assert_eq!(scored.documents, 3);
        assert_eq!(scored.field_match_rate, Some(0.5));
        let surname = scored
            .per_field
            .iter()
            .find(|field| field.field == "surname")
            .expect("surname");
        assert_eq!(surname.documents, 3);

        let json = serde_json::to_value(crate::report::ProviderRow::from(
            reports.into_iter().next().expect("one report"),
        ))
        .expect("serialize report");
        assert_eq!(json["accuracy"]["accepted_reads"]["documents"], 2);
        assert_eq!(json["accuracy"]["scored"]["documents"], 3);
        let scored_surname = json["accuracy"]["scored"]["per_field"]
            .as_array()
            .expect("per_field")
            .iter()
            .find(|field| field["field"] == "surname")
            .expect("surname")
            .clone();
        assert_eq!(scored_surname["documents"], 3);
        let all_labelled_surname = json["accuracy"]["per_field_cer"]
            .as_array()
            .expect("per_field_cer")
            .iter()
            .find(|field| field["field"] == "surname")
            .expect("surname")
            .clone();
        assert_eq!(all_labelled_surname["documents"], 4);
    }

    /// A reader that fails on every document.
    struct ErroringReader {
        capability: Capability,
    }

    #[async_trait::async_trait]
    impl IntelligenceProvider for ErroringReader {
        fn id(&self) -> ProviderId {
            ProviderId("erroring-test-reader")
        }
        fn capability(&self) -> &Capability {
            &self.capability
        }
        fn describe(&self) -> String {
            "always-failing test reader".into()
        }
    }

    #[async_trait::async_trait]
    impl FieldReader for ErroringReader {
        async fn read(&self, _ctx: &DocumentContext<'_>) -> Result<Reading, ProviderError> {
            Err(ProviderError::Failed {
                detail: "test failure".into(),
            })
        }
    }

    /// A reader error is a failed read: end-to-end scores the document's
    /// fields as absent, and it is not an accepted read.
    #[tokio::test]
    async fn a_reader_error_scores_absent_fields_end_to_end_only() {
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(std::sync::Arc::new(ErroringReader {
                capability: Capability::deterministic_reader(),
            }))
            .build()
            .expect("one reader");
        let mut page = rate_test_page();
        page.mrz_expected = true;
        page.mrz_found = true;
        page.ground_truth = Some(HashMap::from([
            (CoreField::Surname, "DOE".to_string()),
            (CoreField::GivenNames, "JOHN".to_string()),
        ]));

        let reports = run_prepped(&catalog, &[Some(page)], false, None, false).await;
        let accuracy = &reports[0].accuracy;
        assert_eq!(accuracy.scored.documents, 1);
        assert_eq!(accuracy.scored.field_match_rate, Some(0.0));
        assert_eq!(accuracy.scored.mean_cer, Some(1.0));
        assert_eq!(accuracy.accepted_reads.documents, 0);
        assert_eq!(accuracy.accepted_reads.field_match_rate, None);
        // The all-labelled figure, kept for continuity, never counted an
        // errored read.
        assert_eq!(accuracy.field_match_rate, None);
    }

    /// An entirely unlabelled corpus must report `None`, not a fabricated
    /// `0.0` that would read as "0% accuracy" instead of "nothing measured."
    #[tokio::test]
    async fn no_labelled_documents_reports_accuracy_as_none_not_zero() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "fixture".to_string(),
            page: OcrPage {
                text: "surname DOE".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let report = &reports[0];
        assert_eq!(report.accuracy.labelled_documents, 0);
        assert_eq!(report.accuracy.field_match_rate, None);
        assert_eq!(report.accuracy.mean_cer, None);
    }

    /// The crux of 1.3: a `!vision` provider gets a computed rate, even
    /// though the fixture has no ground truth at all — unsupported-assertion
    /// needs no labels, only OCR text, which is exactly what 1.2 must not
    /// gate it behind.
    #[tokio::test]
    async fn non_vision_provider_gets_a_computed_unsupported_assertion_rate() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "SMITH", // absent from the OCR text below
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "fixture".to_string(),
            page: OcrPage {
                text: "surname DOE, birth date 1990".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None, // deliberately unlabelled — must not block this metric
            ground_truth_mrz: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        match &reports[0].unsupported_assertion {
            UnsupportedAssertion::Computed {
                overall,
                with_mrz_anchor,
                without_mrz_anchor,
            } => {
                assert_eq!(overall.assertions_total, 1);
                assert_eq!(
                    overall.rate,
                    Some(1.0),
                    "SMITH never appears in the OCR text"
                );
                // The fixture's OCR text carries no MRZ, so the whole
                // measurement belongs to the unanchored half — the population
                // Phase 2's grounding gate is aimed at.
                assert!(
                    with_mrz_anchor.is_none(),
                    "no fixture document had a parseable MRZ"
                );
                let unanchored = without_mrz_anchor
                    .as_ref()
                    .expect("the one fixture document had no MRZ");
                assert_eq!(unanchored.documents, 1);
                assert_eq!(unanchored.rate, Some(1.0));
            }
            UnsupportedAssertion::NotApplicable { .. } => {
                panic!("a deterministic (!vision) provider must get a computed rate")
            }
        }
    }

    /// `--dump-ocr` (a `Some(dir)` handed to `run_prepped`) writes one JSONL
    /// row per `checksum_failed` miss, carrying the full pre-parse OCR text,
    /// the recovered MRZ zone, and the failing check field(s) — the file
    /// Phase 2's real-specimen root-cause pass reads.
    #[tokio::test]
    async fn dump_ocr_writes_one_jsonl_row_per_checksum_failed_miss() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "ERIKSSON",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        // A TD3 that parses but does not validate: the DOB digits carry a
        // malformed month (13), which both fails the check digit and blocks
        // `damaged_pass` repair (it accepts a damaged read only when both
        // dates are well-formed), so the miss stays `checksum_failed` rather
        // than being silently recovered.
        let mrz = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<<<<\n\
                   L898902C36UTO7413122F1204159ZE184226B<<<<<10";
        let parsed = mrz::find_and_parse(mrz).expect("the corrupted TD3 still parses");
        assert!(!parsed.valid(), "but it must not validate");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: Some("passports/corrupted-td3-fixture.png".to_string()),
            source_sha256: Some("a".repeat(64)),
            name: "corrupted-td3-fixture".to_string(),
            page: OcrPage {
                text: mrz.to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: true,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let dir = std::env::temp_dir().join(format!(
            "provider-bench-dump-ocr-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _reports = run_prepped_with_dump_options(
            &catalog,
            &prepped,
            false,
            Some(&dir),
            false,
            Some("test-run.json"),
            false,
            None,
        )
        .await;

        let path = dir.join("provider-bench-miss-ocr-dump.jsonl");
        let dump = std::fs::read_to_string(&path).expect("dump file written");
        let _ = std::fs::remove_dir_all(&dir);

        let lines: Vec<&str> = dump.lines().collect();
        assert_eq!(lines.len(), 1, "one row for the one checksum_failed miss");
        let row: serde_json::Value = serde_json::from_str(lines[0]).expect("valid JSON row");
        assert_eq!(row["name"], "corrupted-td3-fixture");
        assert_eq!(row["asset_id"], "passports/corrupted-td3-fixture.png");
        assert_eq!(row["source_sha256"], "a".repeat(64));
        assert_eq!(row["run_manifest"], "test-run.json");
        assert_eq!(row["provider"], "fixed-test-reader");
        assert!(
            row["raw_ocr_text"]
                .as_str()
                .is_some_and(|t| t.contains("ERIKSSON")),
            "full pre-parse OCR text is carried verbatim"
        );
        let check_states = row["check_states"]
            .as_object()
            .expect("a parsed MRZ records every check state");
        assert_eq!(check_states.len(), 5);
        assert_eq!(check_states["document_number"], true);
        assert_eq!(check_states["date_of_birth"], false);
        assert!(
            check_states.values().any(|state| state == false),
            "at least one check must fail"
        );
        assert!(
            row["ground_truth_mrz"].is_null() && row["zone_mismatch"].is_null(),
            "an unlabelled specimen carries no ground-truth MRZ"
        );
        assert_eq!(
            row["miss_reason"], "checksum_failed",
            "the row records which gate it is under"
        );
    }

    /// The dump also covers `no_mrz_found` — a genuine detection failure
    /// since the 2026-09-09 denominator correction (MRZ-less documents are
    /// scored out as `no_mrz_expected` first). Those rows carry the band
    /// score and raw OCR text but no recovered zone: nothing MRZ-shaped
    /// parsed. This is the localization-vs-recognition material for
    /// `ADR-0008` chunk 1C cell (b).
    #[test]
    #[should_panic(expected = "at least one check must fail")]
    fn checksum_failed_assertion_rejects_a_clean_read() {
        let clean = mrz::parse_td3(
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
        )
        .expect("fixture parses");
        let states = crate::check_states(&clean.checks);
        assert!(
            states.values().any(|state| *state == Some(false)),
            "at least one check must fail"
        );
    }

    #[tokio::test]
    async fn dump_ocr_also_writes_a_row_for_a_no_mrz_found_miss() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "UNUSED",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        // OCR text with nothing MRZ-shaped in it: an MRZ is expected
        // (`mrz_expected: true`) but none was found.
        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "no-mrz-found-fixture".to_string(),
            page: OcrPage {
                text: "REPUBLIC OF EXAMPLE\nNAME  JANE DOE\n".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let dir = std::env::temp_dir().join(format!(
            "provider-bench-dump-nomrz-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _reports = run_prepped(&catalog, &prepped, false, Some(&dir), false).await;

        let path = dir.join("provider-bench-miss-ocr-dump.jsonl");
        let dump = std::fs::read_to_string(&path).expect("dump file written");
        let _ = std::fs::remove_dir_all(&dir);

        let row: serde_json::Value =
            serde_json::from_str(dump.lines().next().expect("one row")).expect("valid JSON");
        assert_eq!(row["name"], "no-mrz-found-fixture");
        assert_eq!(row["miss_reason"], "no_mrz_found");
        assert!(
            row["raw_ocr_text"]
                .as_str()
                .is_some_and(|t| t.contains("JANE DOE")),
            "the raw OCR text the recognizer produced is carried"
        );
        assert!(
            row["recovered_mrz_lines"].as_array().unwrap().is_empty()
                && row.get("check_states").is_none(),
            "nothing MRZ-shaped parsed, so no recovered zone or check observations"
        );
    }

    /// A specimen whose **printed** zone fails its own check digits is
    /// `checksum_failed_specimen` — and, since 2026-09-09, that verdict no
    /// longer depends on OCR having recovered the zone exactly.
    ///
    /// The old rule was "the run recovered the transcription byte for byte",
    /// which conflated a fact about the document with an achievement of the
    /// run. It recognised 1 of the 16 non-conforming specimens the 2026-09-08
    /// writeup identified; the other 15 were non-conforming *and* imperfectly
    /// read, so they were scored as ordinary OCR failures against a target no
    /// pipeline could ever hit. See the second case below.
    #[tokio::test]
    async fn a_specimen_with_a_nonconforming_printed_zone_is_scored_out() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "ERIKSSON",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        // A non-conforming TD3 (month 13 in the DOB field). The OCR text *is*
        // this exact zone, so `find_and_parse` recovers it unchanged.
        let zone = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<<\n\
                    L898902C36UTO7413122F1204159ZE184226B<<<<<10";
        let recovered = mrz::find_and_parse(zone).expect("parses");
        assert!(!recovered.valid());

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "labelled-nonconforming-specimen".to_string(),
            page: OcrPage {
                text: zone.to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: Some(recovered.mrz_lines.clone()),
            image_path: PathBuf::from("unused.png"),
            mrz_found: true,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: true,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let detail = &reports[0].documents_detail[0];
        assert!(matches!(
            detail.miss_reason,
            Some(MissReason::ChecksumFailed {
                specimen_nonconforming: true,
                ..
            })
        ));
        assert_eq!(
            detail.miss_reason.as_ref().map(miss_kind),
            Some("checksum_failed_specimen")
        );
    }

    /// A **conforming** printed zone that the run misreads stays
    /// `checksum_failed` — a genuine OCR error, inside the denominator, and the
    /// population accuracy work is actually aimed at.
    ///
    /// This is the other half of the rule above, and the reason it is keyed on
    /// the printed zone rather than on how faithfully OCR did: the two cases
    /// differ in the *document*, not in the quality of the read. Note the true
    /// zone here is ICAO 9303's own canonical example, asserted valid below —
    /// the fixture used to use a month-13 zone, which was non-conforming, so it
    /// claimed to test a conforming-document misread while doing the opposite.
    #[tokio::test]
    async fn a_conforming_zone_read_with_an_ocr_error_stays_checksum_failed() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "ERIKSSON",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        // ICAO 9303's canonical TD3 example: every check digit verifies.
        let true_zone = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<<\n\
                         L898902C36UTO7408122F1204159ZE184226B<<<<<10";
        assert!(
            mrz::find_and_parse(true_zone).is_ok_and(|d| d.valid()),
            "the fixture's printed zone must be conforming, or this test is measuring the \
             other case"
        );
        // OCR misreads the document number's leading `L` as `1`.
        let ocr_zone = true_zone.replacen("L898902C36", "1898902C36", 1);

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "labelled-ocr-misread".to_string(),
            page: OcrPage {
                text: ocr_zone,
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: Some(true_zone.to_string()),
            image_path: PathBuf::from("unused.png"),
            mrz_found: true,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let detail = &reports[0].documents_detail[0];
        assert!(matches!(
            detail.miss_reason,
            Some(MissReason::ChecksumFailed {
                specimen_nonconforming: false,
                ..
            })
        ));
        assert_eq!(
            detail.miss_reason.as_ref().map(miss_kind),
            Some("checksum_failed")
        );
    }

    /// `printed_zone_nonconforming` is a property of the transcription, not of
    /// the read: a non-conforming zone stays non-conforming even when OCR
    /// mangles it.
    ///
    /// This is the 15 documents the old rule missed. They were non-conforming
    /// *and* imperfectly read, so "did OCR recover the zone exactly?" answered
    /// no and they were filed as ordinary `checksum_failed` — scored inside the
    /// denominator, against a hit they could never reach. Asserted on the
    /// derivation directly, because `prep_specimens` (where it is computed) needs
    /// real images and does not run in a unit test.
    #[test]
    fn a_nonconforming_printed_zone_is_recognised_however_badly_it_was_read() {
        // Month 13 in the date of birth — the zone is malformed as printed.
        let printed = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<<\n\
                       L898902C36UTO7413122F1204159ZE184226B<<<<<10";
        let nonconforming =
            |zone: &str| !mrz::find_and_parse(zone).is_ok_and(|d: mrz::MrzData| d.valid());

        assert!(
            nonconforming(printed),
            "the printed zone fails its own check digits"
        );

        // The old rule compared the recovered zone against the transcription and
        // needed an exact match. Under it, this document — mangled by OCR on top
        // of being non-conforming — was scored as an ordinary OCR failure.
        let mangled = printed.replacen("L898902C36", "1898902C36", 1);
        assert_ne!(
            mrz_zone_mismatch(&mangled, printed),
            0,
            "the read differs from the transcription, which is what used to disqualify it"
        );
        assert!(
            nonconforming(printed),
            "and yet the document is still one no pipeline can turn into a hit"
        );
    }

    /// A `*_redacted_mrz` specimen whose redaction bar still OCR'd into a
    /// parseable (checksum-failing) zone is reported as `redacted_mrz`, not
    /// `checksum_failed` — the zone is an artefact of the blackout, not a
    /// misread of a real MRZ.
    #[tokio::test]
    async fn a_redacted_specimen_is_reported_as_redacted_not_checksum_failed() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        // A structurally valid TD3 whose check digits do not verify — exactly
        // what `find_and_parse` yields for many redaction bars.
        let zone = "P<UTODOE<<JANE<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<\n\
                    XXXXXXXXX0UTO8001014F2501017<<<<<<<<<<<<<<08";
        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "Wonderland_Passport_Specimen_P0_UTO_2020_redacted_mrz".to_string(),
            page: OcrPage {
                text: zone.to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("unused.png"),
            mrz_found: true,
            document_number_leading_filler: false,
            redacted: true,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let detail = &reports[0].documents_detail[0];
        assert!(matches!(detail.miss_reason, Some(MissReason::Redacted)));
        assert_eq!(
            detail.miss_reason.as_ref().map(miss_kind),
            Some("redacted_mrz")
        );
    }

    /// The redacted rung sits *before* the `mrz_found` gate: a specimen whose
    /// zone is blacked out is `redacted_mrz` whether or not the bar happened to
    /// OCR into something parseable.
    ///
    /// This test asserted the opposite until 2026-09-09, and the behaviour it
    /// pinned was the bug. Ordering redaction after `mrz_found` split the 36
    /// redacted specimens into 9 scored out and 27 scored in as detection
    /// failures — decided entirely by the texture of the blackout bar, which is
    /// not a property of the pipeline and is not stable run to run. It also ran
    /// the wrong way round: the *cleaner* the redaction, the worse the document
    /// scored, because a bar that produced no parseable noise looked exactly
    /// like a passport whose MRZ we had failed to find.
    #[tokio::test]
    async fn a_redacted_specimen_is_redacted_whether_or_not_its_bar_parsed() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "Wonderland_Passport_Specimen_P0_UTO_2020_redacted_mrz".to_string(),
            page: OcrPage {
                text: "just some redaction smudge, nothing MRZ-shaped".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("unused.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: true,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let detail = &reports[0].documents_detail[0];
        assert_eq!(
            detail.miss_reason.as_ref().map(miss_kind),
            Some("redacted_mrz"),
            "a blacked-out zone is not a detection failure, however cleanly it was blacked out"
        );
    }

    /// A redacted specimen is dropped from the Tier-1 hit-rate denominator —
    /// one redacted miss alongside one genuine hit reports `1.0`, not `0.5`.
    #[tokio::test]
    async fn redacted_specimens_are_excluded_from_the_tier1_denominator() {
        let mut hit_evidence = Evidence::default();
        hit_evidence.mrz_found = true;
        hit_evidence.mrz_checksums_valid = true;
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence: hit_evidence,
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let page = |name: &str, redacted: bool| {
            Some(BenchPage {
                pass_objects: None,
                asset_id: None,
                source_sha256: None,
                name: name.to_string(),
                page: OcrPage {
                    text: "surname DOE".to_string(),
                    ..OcrPage::default()
                },
                ground_truth: None,
                ground_truth_mrz: None,
                image_path: PathBuf::from("unused.png"),
                mrz_found: true,
                document_number_leading_filler: false,
                redacted,
                mrz_expected: true,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
            })
        };
        let prepped = vec![
            page("Clean_Passport_Specimen_P0_UTO_2020_mrz", false),
            page("Blacked_Passport_Specimen_P0_UTO_2019_redacted_mrz", true),
        ];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        match reports[0].tier1_hit_rate {
            Tier1HitRate::Computed(rate) => assert_eq!(rate, 1.0),
            Tier1HitRate::NotApplicable { .. } => {
                panic!("a deterministic provider must get a computed tier1_hit_rate")
            }
        }
    }

    /// ...and, like a redacted one, it is dropped from the Tier-1 denominator:
    /// one correct refusal alongside one genuine hit reports `1.0`, not `0.5`.
    ///
    /// The rate and the committed baseline compute `scored` in two different
    /// places (`Tier1HitRate::Computed` here, `RealSpecimenSnapshot` in
    /// `bin/provider-bench.rs`). This pins the half that would otherwise be
    /// caught only by a CI run against the real corpus.
    #[tokio::test]
    async fn mrz_less_specimens_are_excluded_from_the_tier1_denominator() {
        let mut hit_evidence = Evidence::default();
        hit_evidence.mrz_found = true;
        hit_evidence.mrz_checksums_valid = true;
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence: hit_evidence,
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        // Both documents read a checksum-valid MRZ from this reader. The
        // MRZ-less one is therefore a false positive — but the point here is
        // the denominator, and `false_positive_mrz` stays *in* it, so use a
        // reader-agnostic pairing instead: mark the second `mrz_found: false`
        // so it lands in `no_mrz_expected`.
        let page = |name: &str, mrz_expected: bool, mrz_found: bool| {
            Some(BenchPage {
                pass_objects: None,
                asset_id: None,
                source_sha256: None,
                name: name.to_string(),
                page: OcrPage {
                    text: "surname DOE".to_string(),
                    ..OcrPage::default()
                },
                ground_truth: None,
                ground_truth_mrz: None,
                image_path: PathBuf::from("unused.png"),
                mrz_found,
                document_number_leading_filler: false,
                redacted: false,
                mrz_expected,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
            })
        };
        let prepped = vec![
            page("Clean_Passport_Specimen_P0_UTO_2020_mrz", true, true),
            page("Wonderland_ID_Specimen_2021_front_no_mrz", false, false),
        ];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        assert_eq!(
            reports[0].documents_detail[1]
                .miss_reason
                .as_ref()
                .map(miss_kind),
            Some("no_mrz_expected")
        );
        match reports[0].tier1_hit_rate {
            Tier1HitRate::Computed(rate) => assert_eq!(
                rate, 1.0,
                "a document with no MRZ to find must not cap the achievable rate"
            ),
            Tier1HitRate::NotApplicable { .. } => {
                panic!("a deterministic provider must get a computed tier1_hit_rate")
            }
        }
    }

    /// A document that carries no MRZ, read correctly as carrying none, is a
    /// **correct refusal** — `no_mrz_expected`, not `no_mrz_found`.
    ///
    /// 42 of the real corpus are exactly this: ID-card fronts, border passes
    /// and driving-license faces. Every one used to be scored as a detection
    /// failure inside the hit-rate denominator, which put the headline rate
    /// 11.6 points low and roughly doubled the apparent size of the
    /// `no_mrz_found` bucket that `ADR-0008` picked as its target metric.
    #[tokio::test]
    async fn a_document_with_no_mrz_reading_none_is_a_correct_refusal() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "Wonderland_ID_Specimen_2021_front_no_mrz".to_string(),
            page: OcrPage {
                text: "IDENTITY CARD  DOE  JANE".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("unused.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: false,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let detail = &reports[0].documents_detail[0];
        assert_eq!(
            detail.miss_reason.as_ref().map(miss_kind),
            Some("no_mrz_expected"),
            "an ID-card front with no MRZ must not be scored as a detection failure"
        );
    }

    /// The other direction, and the serious one: a checksum-**valid** MRZ read
    /// off a document that carries none is a hallucinated record.
    ///
    /// This had no representation before. Such a document is unlabelled by
    /// construction (`ocr_fixtures/` covers the MRZ-bearing specimens), so it
    /// passed the `mrz_found` gate, passed the checksum gate, found no ground
    /// truth to be compared against, and was counted as a **Tier-1 hit**.
    /// `Monaco_ID_Specimen_XXXX_front_no_mrz.png` really did produce one.
    #[tokio::test]
    async fn a_checksum_valid_read_off_an_mrz_less_document_is_a_false_positive() {
        let mut evidence = Evidence::default();
        evidence.mrz_found = true;
        evidence.mrz_checksums_valid = true;
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "DOE",
            given_names: "",
            evidence,
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "Wonderland_ID_Specimen_2021_front_no_mrz".to_string(),
            page: OcrPage {
                text: "I<UTODOE<<JANE<<<<<<<<<<<<<<<<".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("unused.png"),
            mrz_found: true,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: false,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let detail = &reports[0].documents_detail[0];
        assert_eq!(
            detail.miss_reason.as_ref().map(miss_kind),
            Some("false_positive_mrz"),
            "a hallucinated MRZ must never be counted as a Tier-1 hit"
        );
        // The regression this pins: it used to reach the ground-truth rung,
        // find `None`, and fall out of the `if` chain as a hit.
        assert!(
            detail.miss_reason.is_some(),
            "a false positive must be a miss, not an unchecked hit"
        );
    }

    #[test]
    fn mrz_zone_mismatch_counts_differing_characters_line_by_line() {
        assert_eq!(mrz_zone_mismatch("ABC\nDEF", "ABC\nDEF"), 0);
        assert_eq!(mrz_zone_mismatch("ABC\nDXF", "ABC\nDEF"), 1);
        // a missing trailing character still counts
        assert_eq!(mrz_zone_mismatch("ABC\nDE", "ABC\nDEF"), 1);
        // a whole extra line counts every character
        assert_eq!(mrz_zone_mismatch("ABC", "ABC\nDEF"), 3);
    }

    /// A different character than `c`, with a different ICAO check-digit
    /// numeric value, staying inside the MRZ charset. `<` and `0` collide
    /// (both have check-digit value `0`), so a naive "next letter/digit"
    /// mapping through that pair would silently pick a no-op mutation —
    /// going through the numeric value directly, mod 36, avoids it and
    /// guarantees the mutation changes what every `verify(...)` call
    /// computes (the weighted-sum delta can never land back on a multiple
    /// of 10 for a `+1 mod 36` step, since the three ICAO weights 7/3/1 are
    /// all coprime with 10).
    fn mrz_test_char_value(c: char) -> u32 {
        match c {
            '0'..='9' => c as u32 - '0' as u32,
            'A'..='Z' => c as u32 - 'A' as u32 + 10,
            '<' => 0,
            other => panic!("test fixture char {other:?} is outside the MRZ charset"),
        }
    }

    fn mrz_test_char_from_value(v: u32) -> char {
        match v {
            0..=9 => char::from(b'0' + v as u8),
            10..=35 => char::from(b'A' + (v - 10) as u8),
            other => unreachable!("value {other} out of MRZ charset range"),
        }
    }

    fn bump(c: char) -> char {
        mrz_test_char_from_value((mrz_test_char_value(c) + 1) % 36)
    }

    /// TD1's textbook composite-only signature (this module's top-of-file
    /// note, point 1): a single wrong character inside an optional-data
    /// field — which carries no check digit of its own — fails the
    /// composite check and *only* the composite check, before any character
    /// is examined.
    #[test]
    fn composite_only_error_fails_only_the_composite_check() {
        let l1 = "I<UTOD231458907<<<<<<<<<<<<<<<";
        let l2 = "7408122F1204159UTO<<<<<<<<<<<6";
        let l3 = "ERIKSSON<<ANNA<MARIA<<<<<<<<<<";
        let baseline = mrz::parse_td1(l1, l2, l3).expect("fixture parses");
        assert!(
            baseline.checks.all_valid(),
            "fixture must start out checksum-clean"
        );

        // Column 20 of line 2 (1-based) sits inside optional_data_2 (18..29).
        let mut chars: Vec<char> = l2.chars().collect();
        chars[20] = bump(chars[20]);
        let mutated_l2: String = chars.into_iter().collect();
        let mutated = mrz::parse_td1(l1, &mutated_l2, l3).expect("still parses");
        assert_eq!(
            mutated.checks.failed(),
            [mrz::Field::Composite],
            "an error inside a composite-only field must fail composite alone"
        );

        let recovered = format!("{l1}\n{mutated_l2}\n{l3}");
        let truth = format!("{l1}\n{l2}\n{l3}");
        let fm = mrz_field_mismatch("TD1", &recovered, &truth).expect("TD1 has a layout");
        assert_eq!(fm.by_field.get("optional_data_2"), Some(&1));
        assert_eq!(
            fm.coverage.get("optional_data_2"),
            Some(&CheckCoverage::CompositeOnly)
        );
    }

    /// The case that motivates this whole instrumentation (this module's
    /// top-of-file note, point 2): nationality and sex sit inside no ICAO
    /// composite on any format, so an error there fails *no* check digit at
    /// all — the document can still read as a clean Tier-1 hit — yet
    /// `mrz_field_mismatch` still reports the field as differing, with
    /// coverage `Uncovered`.
    #[test]
    fn an_uncovered_field_error_fails_no_check_but_is_still_reported() {
        let l1 = "I<UTOD231458907<<<<<<<<<<<<<<<";
        let l2 = "7408122F1204159UTO<<<<<<<<<<<6";
        let l3 = "ERIKSSON<<ANNA<MARIA<<<<<<<<<<";

        // Column 16 of line 2 (1-based) sits inside nationality (15..18).
        let mut chars: Vec<char> = l2.chars().collect();
        chars[16] = bump(chars[16]);
        let mutated_l2: String = chars.into_iter().collect();
        let mutated = mrz::parse_td1(l1, &mutated_l2, l3).expect("still parses");
        assert!(
            mutated.checks.all_valid(),
            "a wrong nationality must not fail any check digit"
        );

        let recovered = format!("{l1}\n{mutated_l2}\n{l3}");
        let truth = format!("{l1}\n{l2}\n{l3}");
        let fm = mrz_field_mismatch("TD1", &recovered, &truth).expect("TD1 has a layout");
        assert_eq!(fm.by_field.get("nationality"), Some(&1));
        assert_eq!(
            fm.coverage.get("nationality"),
            Some(&CheckCoverage::Uncovered)
        );
    }

    /// A format the truth's own shape contradicts yields **no attribution**,
    /// not a confident wrong one.
    ///
    /// This is the measured 2026-09-19 case, not a hypothetical: a TD3
    /// specimen whose reader misdetected the format as TD1. `resolve_mrz_format`
    /// takes the provider's own evidence when that read is checksum-valid
    /// (#555), so the wrong layout reached
    /// `mrz_field_mismatch`, which happily attributed a two-line 44-column
    /// zone against TD1's three-line 30-column table — reporting differing
    /// characters on a line the document does not have, and populating
    /// `issuing_country` and `optional_data_1`/`_2`, which TD3 has no such
    /// fields for.
    #[test]
    fn a_format_the_truth_shape_contradicts_yields_no_attribution() {
        // Built from line arrays, never a multi-line literal: a trailing-
        // backslash continuation bakes the source indentation into the
        // continuation bakes the source indentation into the string and makes
        // a 44-column line 69 columns wide, which the guard then correctly
        // rejects for the wrong reason.
        let td3_truth = [
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
        ]
        .join("\n");
        let td3_read = [
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "L898902C36UTO7408122F1204159ZE184226B<<<<<11",
        ]
        .join("\n");
        for line in td3_truth.lines() {
            assert_eq!(line.chars().count(), 44, "test fixture must be TD3-shaped");
        }

        // Declared correctly: attribution still happens.
        assert!(
            mrz_field_mismatch("TD3", &td3_read, &td3_truth).is_some(),
            "a TD3 zone declared TD3 must still attribute normally"
        );

        // Declared TD1 (three lines of 30) - the shape cannot be TD3's.
        assert_eq!(
            mrz_field_mismatch("TD1", &td3_read, &td3_truth),
            None,
            "a TD3-shaped zone declared TD1 must attribute nothing: the layout              does not describe this document, so every field name it would              produce is meaningless"
        );

        // Mirror direction, so the guard is not accidentally one-sided.
        let td1_truth = [
            "I<UTOD231458907<<<<<<<<<<<<<<<",
            "7408122F1204159UTO<<<<<<<<<<<6",
            "ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
        ]
        .join("\n");
        assert_eq!(
            mrz_field_mismatch("TD3", &td1_truth, &td1_truth),
            None,
            "a TD1-shaped zone declared TD3 must attribute nothing either"
        );
    }

    /// The guard reads the layout's own spans, so it cannot drift from the
    /// table: every format's declared widths must match the ICAO line
    /// geometry, and a zone of that shape must pass its own check.
    #[test]
    fn every_layout_declares_the_line_geometry_its_format_actually_has() {
        // (format, expected line count, expected width of every line)
        for (format, lines, width) in [
            ("TD1", 3usize, 30usize),
            ("TD2", 2, 36),
            ("TD3", 2, 44),
            ("MRVA", 2, 44),
            ("MRVB", 2, 36),
        ] {
            let layout = mrz_field_layout(format).expect("format has a layout");
            let widths = mrz_layout_line_widths(layout);
            assert_eq!(
                widths.len(),
                lines,
                "{format}: layout declares {} line(s), ICAO says {lines}",
                widths.len()
            );
            for (line, declared) in &widths {
                assert_eq!(
                    *declared, width,
                    "{format} line {line}: layout declares width {declared},                      ICAO says {width}"
                );
            }
            // A zone of exactly that shape must satisfy the guard, and one a
            // single character short must not.
            let good = vec!["<".repeat(width); lines].join(
                "
",
            );
            assert!(
                mrz_zone_matches_layout(&good, layout),
                "{format}: a correctly shaped zone must pass the guard"
            );
            let short = vec!["<".repeat(width - 1); lines].join(
                "
",
            );
            assert!(
                !mrz_zone_matches_layout(&short, layout),
                "{format}: a zone one character short per line must not pass"
            );
        }
    }

    /// The drift guard for [`mrz_field_line`] itself: expected lines are
    /// derived from [`mrz_field_layout`]'s own spans, never hand-restated, so
    /// this fails the moment `mrz_field_line` stops delegating to the shared
    /// table — e.g. if a future edit copied the offsets into a second,
    /// hand-maintained map instead of looking them up here. Runs over every
    /// span in every format's layout, not only the bench-field aliases, so it
    /// catches drift on any name.
    #[test]
    fn mrz_field_line_never_drifts_from_the_shared_layout_table() {
        for format in ["TD1", "TD2", "TD3", "MRVA", "MRVB"] {
            let layout = mrz_field_layout(format).expect("format has a layout");
            for span in layout {
                assert_eq!(
                    mrz_field_line(format, span.name),
                    Some(span.line),
                    "{format} field {:?}: mrz_field_line disagrees with mrz_field_layout's own line",
                    span.name
                );
            }
        }
    }

    /// `mrz_field_line` is format-aware in the way that actually matters: TD1
    /// puts the combined name field on physical line 3, while TD2/TD3/MRV-A/
    /// MRV-B put it on line 1 — so a bench field's line cannot be a fixed
    /// lookup independent of the format.
    #[test]
    fn mrz_field_line_resolves_bench_field_aliases_per_format() {
        // (format, name-field line, document-code-field line)
        for (format, name_line, code_line) in [
            ("TD1", 3, 1),
            ("TD2", 1, 1),
            ("TD3", 1, 1),
            ("MRVA", 1, 1),
            ("MRVB", 1, 1),
        ] {
            assert_eq!(
                mrz_field_line(format, "surname"),
                Some(name_line),
                "{format}: surname aliases the combined name span"
            );
            assert_eq!(
                mrz_field_line(format, "given_names"),
                Some(name_line),
                "{format}: given_names aliases the combined name span"
            );
            assert_eq!(
                mrz_field_line(format, "document_type"),
                Some(code_line),
                "{format}: document_type aliases document_code"
            );
        }
    }

    /// A field with no span in a format's layout must not silently land in a
    /// line bucket — TD1 has no `personal_number` span at all (its optional
    /// data fields are not the same field), and `mrz_lines` is a whole-zone
    /// aggregate on every format, never a field with a span.
    #[test]
    fn mrz_field_line_is_none_when_nothing_resolves() {
        assert_eq!(
            mrz_field_line("TD1", "personal_number"),
            None,
            "TD1 has no personal_number span"
        );
        assert_eq!(
            mrz_field_line("TD3", "mrz_lines"),
            None,
            "mrz_lines is a whole-zone aggregate, never a field with a span"
        );
        assert_eq!(
            mrz_field_line("BOGUS", "document_type"),
            None,
            "an unresolved format has no layout at all"
        );
    }

    /// ADR-0018's two schema fields land on the span each format prints them
    /// under, and nowhere else: `optional_data_1` is TD2/MRV `optional_data`
    /// and TD1 line 1, `optional_data_2` is TD1 line 2 only, and on TD3 the
    /// element is `personal_number`, so `optional_data_1` resolves to nothing
    /// there rather than double-counting the personal-number span.
    #[test]
    fn mrz_field_line_maps_the_optional_data_fields_per_format() {
        assert_eq!(mrz_field_line("TD1", "optional_data_1"), Some(1));
        assert_eq!(mrz_field_line("TD1", "optional_data_2"), Some(2));
        for format in ["TD2", "MRVA", "MRVB"] {
            assert_eq!(
                mrz_field_line(format, "optional_data_1"),
                Some(2),
                "{format}: optional_data_1 is the printed optional_data span"
            );
            assert_eq!(
                mrz_field_line(format, "optional_data_2"),
                None,
                "{format}: only TD1 prints a second element"
            );
        }
        assert_eq!(mrz_field_line("TD3", "personal_number"), Some(2));
        assert_eq!(
            mrz_field_line("TD3", "optional_data_1"),
            None,
            "TD3's element is personal_number; optional_data_1 must not alias it"
        );
        assert_eq!(mrz_field_line("TD3", "optional_data_2"), None);
    }

    /// `"unmapped"` survives the guard, for the one case it is right for: the
    /// guard constrains the *truth*, so a recovered zone longer than the truth
    /// still contributes positions past the last declared span, and those are
    /// counted rather than dropped silently.
    #[test]
    fn a_longer_recovered_zone_still_reports_unmapped_positions() {
        let truth = [
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
        ]
        .join("\n");
        // Same zone with one extra trailing character on line 2: position 44
        // falls past TD3's last declared span.
        let longer = [
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10X",
        ]
        .join("\n");
        let m = mrz_field_mismatch("TD3", &longer, &truth).expect("truth is well-formed TD3");
        assert_eq!(
            m.by_field.get("unmapped"),
            Some(&1),
            "a position past the declared spans must still be counted"
        );
    }

    /// Per-format sanity: a known single-position error in a known field
    /// attributes to exactly that field, with count 1, and the coverage this
    /// module's own table declares for it.
    #[test]
    fn per_format_single_position_error_attributes_to_the_expected_field() {
        // (format, truth lines joined with \n, 0-based line index, 0-based
        // column, expected field name, expected coverage)
        let cases: &[(&str, &str, usize, usize, &str, CheckCoverage)] = &[
            (
                "TD1",
                "I<UTOD231458907<<<<<<<<<<<<<<<\n\
                 7408122F1204159UTO<<<<<<<<<<<6\n\
                 ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
                1,
                20,
                "optional_data_2",
                CheckCoverage::CompositeOnly,
            ),
            (
                "TD2",
                "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n\
                 D231458907UTO7408122F1204159<<<<<<<6",
                1,
                30,
                "optional_data",
                CheckCoverage::CompositeOnly,
            ),
            (
                "TD3",
                "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
                 L898902C36UTO7408122F1204159ZE184226B<<<<<10",
                1,
                30,
                "personal_number",
                CheckCoverage::OwnCheckDigit,
            ),
            (
                "MRVA",
                "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n\
                 L898902C<3UTO6908061F9406236ZE184226B<<<<<<<",
                1,
                30,
                "optional_data",
                CheckCoverage::Uncovered,
            ),
            (
                "MRVB",
                "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<\n\
                 L898902C<3UTO6908061F9406236ZE184226",
                1,
                30,
                "optional_data",
                CheckCoverage::Uncovered,
            ),
        ];
        for &(format, truth, line_idx, col, expected_field, expected_coverage) in cases {
            let lines: Vec<&str> = truth.lines().collect();
            let mut chars: Vec<char> = lines[line_idx].chars().collect();
            chars[col] = bump(chars[col]);
            let mut mutated_lines: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
            mutated_lines[line_idx] = chars.into_iter().collect();
            let recovered = mutated_lines.join("\n");

            let fm = mrz_field_mismatch(format, &recovered, truth)
                .unwrap_or_else(|| panic!("{format} must have a field layout"));
            assert_eq!(
                fm.by_field.get(expected_field),
                Some(&1),
                "{format}: expected exactly one differing character in {expected_field}"
            );
            assert_eq!(
                fm.coverage.get(expected_field),
                Some(&expected_coverage),
                "{format}: unexpected coverage for {expected_field}"
            );
            assert_eq!(fm.by_line.get(&(line_idx + 1)), Some(&vec![col]));
        }
    }

    /// No function on the field-mismatch path may ever serialize a printed
    /// character: `FieldMismatch` is built entirely from positions, field
    /// names, and coverage states, never from the zone text itself. This
    /// plants a distinctive watermark string in each of two differing
    /// fields and confirms neither watermark — nor the value that replaced
    /// it — reaches the serialized output.
    #[test]
    fn field_mismatch_never_serializes_a_character_value_from_the_input_zone() {
        let truth = "I<UTOZZZZZZZZZ7<<<<<<<<<<<<<<<\n\
                     7408122F1204159UTO<<<<<<<<<<<6\n\
                     ERIKSSON<<ANNA<MARIA<<<<<<<<<<";
        let recovered = "I<UTOYYYYYYYYY7<<<<<<<<<<<<<<<\n\
                          7408122F1204159UTOQQQQQQQQQQQ6\n\
                          ERIKSSON<<ANNA<MARIA<<<<<<<<<<";

        let fm = mrz_field_mismatch("TD1", recovered, truth).expect("TD1 has a layout");
        assert_eq!(fm.by_field.get("document_number"), Some(&9));
        assert_eq!(fm.by_field.get("optional_data_2"), Some(&11));

        let serialized = serde_json::to_string(&fm).expect("FieldMismatch serializes");
        for watermark in ["ZZZZZZZZZ", "YYYYYYYYY", "QQQQQQQQQQQ"] {
            assert!(
                !serialized.contains(watermark),
                "serialized field-mismatch output must never contain a character value \
                 from the input zone, but found {watermark:?} in {serialized}"
            );
        }
    }

    /// Pins every field-layout table (`TD1_FIELDS`, `TD2_FIELDS`,
    /// `TD3_FIELDS`, `MRVA_FIELDS`, `MRVB_FIELDS`) against `mrz::parse_*`'s
    /// *actual* check-digit behaviour, rather than duplicating the parser's
    /// literal slice offsets as a second, driftable copy of them: for every
    /// position of a valid specimen, mutate that one character, reparse
    /// with the real `mrz` parser, and confirm which check(s) broke matches
    /// what the field's declared `CheckCoverage` predicts. A future change
    /// to any `verify(...)` span in `crates/mrz/src/parser.rs` that isn't
    /// mirrored in this module's field-layout tables fails this test, not
    /// silently.
    #[test]
    fn field_layout_matches_the_parsers_own_check_digit_spans() {
        fn checks_failed_names(checks: &mrz::Checks) -> std::collections::BTreeSet<&'static str> {
            checks.failed().iter().map(|f| f.as_str()).collect()
        }

        struct Case {
            format: &'static str,
            mrz_format: mrz::Format,
            lines: Vec<&'static str>,
            parse: fn(&[String]) -> Option<mrz::Checks>,
        }

        let cases = [
            Case {
                format: "TD1",
                mrz_format: mrz::Format::Td1,
                lines: vec![
                    "I<UTOD231458907<<<<<<<<<<<<<<<",
                    "7408122F1204159UTO<<<<<<<<<<<6",
                    "ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
                ],
                parse: |l| {
                    mrz::parse_td1(&l[0], &l[1], &l[2])
                        .ok()
                        .map(|d| d.checks.clone())
                },
            },
            Case {
                format: "TD2",
                mrz_format: mrz::Format::Td2,
                lines: vec![
                    "I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
                    "D231458907UTO7408122F1204159<<<<<<<6",
                ],
                parse: |l| mrz::parse_td2(&l[0], &l[1]).ok().map(|d| d.checks.clone()),
            },
            Case {
                format: "TD3",
                mrz_format: mrz::Format::Td3,
                lines: vec![
                    "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
                    "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
                ],
                parse: |l| mrz::parse_td3(&l[0], &l[1]).ok().map(|d| d.checks.clone()),
            },
            Case {
                format: "MRVA",
                mrz_format: mrz::Format::MrvA,
                lines: vec![
                    "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
                    "L898902C<3UTO6908061F9406236ZE184226B<<<<<<<",
                ],
                parse: |l| {
                    mrz::parse_mrv_a(&l[0], &l[1])
                        .ok()
                        .map(|d| d.checks.clone())
                },
            },
            Case {
                format: "MRVB",
                mrz_format: mrz::Format::MrvB,
                lines: vec![
                    "V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<",
                    "L898902C<3UTO6908061F9406236ZE184226",
                ],
                parse: |l| {
                    mrz::parse_mrv_b(&l[0], &l[1])
                        .ok()
                        .map(|d| d.checks.clone())
                },
            },
        ];

        for case in cases {
            let original: Vec<String> = case.lines.iter().map(|s| s.to_string()).collect();
            let baseline = (case.parse)(&original)
                .unwrap_or_else(|| panic!("{}: fixture must parse", case.format));
            assert!(
                baseline.all_valid(),
                "{}: fixture must be checksum-clean before mutation",
                case.format
            );
            let actual_applicability = [
                baseline.document_number.is_some(),
                baseline.date_of_birth.is_some(),
                baseline.date_of_expiry.is_some(),
                baseline.personal_number.is_some(),
                baseline.composite.is_some(),
            ];
            assert_eq!(
                actual_applicability,
                case.mrz_format.check_digit_applicability(),
                "{}: applicability table must match the parser's printed check digits",
                case.format
            );

            let layout = mrz_field_layout(case.format)
                .unwrap_or_else(|| panic!("{}: must have a field layout", case.format));

            // Full coverage: every position in every line maps to exactly
            // one declared field, with no gaps and no overlaps.
            for (li, line) in case.lines.iter().enumerate() {
                for col in 0..line.chars().count() {
                    let matches: Vec<_> = layout
                        .iter()
                        .filter(|f| f.line == li + 1 && col >= f.start && col < f.end)
                        .collect();
                    assert_eq!(
                        matches.len(),
                        1,
                        "{} line {} col {col} must map to exactly one field, got {matches:?}",
                        case.format,
                        li + 1
                    );
                }
            }

            for field in layout {
                // Position 0 of `document_code` gates the parser's own
                // document-code validation (`starts_with('P'/'V')` or
                // `matches!(.., I|A|C)`); mutating it changes which document
                // this is, not a checksum, so it is excluded here — every
                // other position of every other field is exercised.
                let start = if field.name == "document_code" {
                    field.start.max(1)
                } else {
                    field.start
                };
                for col in start..field.end {
                    let mut mutated = original.clone();
                    let mut chars: Vec<char> = mutated[field.line - 1].chars().collect();
                    chars[col] = bump(chars[col]);
                    mutated[field.line - 1] = chars.into_iter().collect();

                    let checks = (case.parse)(&mutated).unwrap_or_else(|| {
                        panic!(
                            "{} field {} col {col}: mutation broke parsing entirely",
                            case.format, field.name
                        )
                    });
                    let failed = checks_failed_names(&checks);

                    match field.coverage {
                        CheckCoverage::OwnCheckDigit => {
                            let expected = match field.name.trim_end_matches("_cd") {
                                "document_number" => "document_number",
                                "date_of_birth" => "date_of_birth",
                                "date_of_expiry" => "date_of_expiry",
                                "personal_number" => "personal_number",
                                other => panic!(
                                    "{}: unexpected own-check field name {other}",
                                    case.format
                                ),
                            };
                            assert!(
                                failed.contains(expected),
                                "{} field {} col {col}: expected {expected} to fail, got \
                                 {failed:?}",
                                case.format,
                                field.name
                            );
                        }
                        CheckCoverage::CompositeOnly => {
                            let expected: std::collections::BTreeSet<&'static str> =
                                std::collections::BTreeSet::from(["composite"]);
                            assert_eq!(
                                failed, expected,
                                "{} field {} col {col}: expected ONLY composite to fail, got \
                                 {failed:?}",
                                case.format, field.name
                            );
                        }
                        CheckCoverage::Uncovered => {
                            assert!(
                                failed.is_empty(),
                                "{} field {} col {col}: expected no check to fail, got \
                                 {failed:?}",
                                case.format,
                                field.name
                            );
                        }
                    }
                }
            }
        }
    }

    /// The other half of 1.3: a `vision` provider gets `NotApplicable`
    /// rather than a number, even when its answer is absent from the OCR
    /// text — that absence is exactly what a working vision provider looks
    /// like, not evidence of hallucination.
    #[tokio::test]
    async fn vision_provider_gets_not_applicable_instead_of_a_rate() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader().with_vision(true),
            surname: "SMITH", // absent from the OCR text — must not be penalized
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "fixture".to_string(),
            page: OcrPage {
                text: "surname DOE, no SMITH anywhere in this text".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        match &reports[0].unsupported_assertion {
            UnsupportedAssertion::NotApplicable { reason } => {
                assert!(!reason.is_empty());
            }
            UnsupportedAssertion::Computed { .. } => {
                panic!("a vision-capable provider must not get a verbatim-substring rate")
            }
        }
    }

    /// A deterministic provider's `tier1_hit_rate` is `Computed`, not
    /// skipped — mirrors `non_vision_provider_gets_a_computed_unsupported_assertion_rate`
    /// but for the `tier1_hit_rate` carve-out, which is gated on
    /// `capability.deterministic` rather than `capability.vision`.
    #[tokio::test]
    async fn deterministic_provider_gets_a_computed_tier1_hit_rate() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "SMITH",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "fixture".to_string(),
            page: OcrPage {
                text: "surname DOE, birth date 1990".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        match reports[0].tier1_hit_rate {
            // `FixedReader::read` always returns `Evidence::default()` and
            // this fixture has no MRZ at all, so the one document is a
            // genuine miss (`NoMrzFound`) — the rate is still `Computed`,
            // just `Computed(0.0)`. That's the point: the *type* must stay
            // `Computed` for a deterministic provider even when the number
            // happens to be zero.
            Tier1HitRate::Computed(rate) => assert_eq!(rate, 0.0),
            Tier1HitRate::NotApplicable { .. } => {
                panic!("a deterministic provider must get a computed tier1_hit_rate")
            }
        }
    }

    /// A non-deterministic (e.g. LLM) provider's `tier1_hit_rate` is
    /// `NotApplicable`, not a fabricated `0.0` — the bug this carve-out
    /// fixes. `FixedReader`'s `Evidence::default()` never sets
    /// `mrz_checksums_valid`, which previously made every non-deterministic
    /// provider look like it hit zero Tier-1 documents regardless of actual
    /// accuracy.
    #[tokio::test]
    async fn non_deterministic_provider_gets_not_applicable_tier1_hit_rate() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::model_reader(CostClass::Expensive),
            surname: "SMITH",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let prepped = vec![Some(BenchPage {
            pass_objects: None,
            asset_id: None,
            source_sha256: None,
            name: "fixture".to_string(),
            page: OcrPage {
                text: "surname DOE, birth date 1990".to_string(),
                ..OcrPage::default()
            },
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed: Duration::ZERO,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        match &reports[0].tier1_hit_rate {
            Tier1HitRate::NotApplicable { reason } => assert!(!reason.is_empty()),
            Tier1HitRate::Computed(rate) => {
                panic!(
                    "a non-deterministic provider must not get a computed \
                     tier1_hit_rate, got {rate}"
                )
            }
        }
    }

    /// The OCR time measured in preparation must reach the per-document row
    /// unchanged. It is the only per-document timing a real-specimen run
    /// records: `speed` times `reader.read` alone, microseconds for the
    /// deterministic provider. A `DocumentDetail` built with `Duration::ZERO`
    /// would compile, serialize, and quietly report every document as free.
    #[tokio::test]
    async fn ocr_elapsed_reaches_the_document_row_unchanged() {
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::model_reader(CostClass::Expensive),
            surname: "SMITH",
            given_names: "",
            evidence: Evidence::default(),
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let ocr_elapsed = Duration::from_millis(1234);
        let prepped = vec![Some(BenchPage {
            asset_id: None,
            source_sha256: None,
            name: "fixture".to_string(),
            page: OcrPage::default(),
            ground_truth: None,
            ground_truth_mrz: None,
            image_path: PathBuf::from("does-not-need-to-exist-for-this-test.png"),
            mrz_found: false,
            document_number_leading_filler: false,
            redacted: false,
            mrz_expected: true,
            printed_zone_nonconforming: false,
            synthetic: false,
            known_or_guessed_format: None,
            ocr_elapsed,
            pass_objects: None,
        })];

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        assert_eq!(reports[0].documents_detail[0].ocr_elapsed, ocr_elapsed);
    }

    /// `strict_tier1_hit_rate`/`names_exact_among_hits` over five documents,
    /// covering every population the two rates need to tell apart:
    ///
    /// 1. `exact-hit` — a Tier-1 hit, name-scorable, names read exactly.
    /// 2. `wrong-name-hit` — a Tier-1 hit, name-scorable, names read wrong
    ///    (`FixedReader` always answers `"JOHN"`, so the document whose
    ///    truth is `"JANE"` is a hit with a wrong name — this is exactly the
    ///    checksum-valid-but-wrong-name gap `NameError` exists to surface).
    /// 3. `miss-with-name-truth` — no MRZ found, but still name-scorable: a
    ///    miss must count toward `strict_tier1_hit_rate`'s denominator
    ///    (`name_scorable_documents`) without counting toward
    ///    `names_exact_among_hits`'s narrower one (`name_scorable_hits`) —
    ///    the whole reason the two rates need different names, not just
    ///    different numbers.
    /// 4. `unlabelled-hit` — a Tier-1 hit with no ground truth for either
    ///    name field at all: not name-scorable, excluded from both
    ///    denominators.
    /// 5. `off-denominator` — carries name ground truth but is
    ///    `no_mrz_expected` (outside `tier1_hit_rate`'s own scored
    ///    population): excluded from both denominators despite having a
    ///    name truth to score against, proving `in_scored_tier1_population`
    ///    gates `StrictNameHitRate` the same way it gates `tier1_hit_rate`.
    ///
    /// Expected: `name_scorable_documents` = 3 (docs 1-3), `name_scorable_hits`
    /// = 2 (docs 1-2), `strict_hits` = 1 (doc 1) — `strict_tier1_hit_rate` =
    /// 1/3, `names_exact_among_hits` = 1/2.
    #[tokio::test]
    async fn strict_tier1_hit_rate_and_names_exact_among_hits_use_different_denominators() {
        let mut hit_evidence = Evidence::default();
        hit_evidence.mrz_found = true;
        hit_evidence.mrz_checksums_valid = true;

        let mut exact_truth = HashMap::new();
        exact_truth.insert(CoreField::Surname, "SMITH".to_string());
        exact_truth.insert(CoreField::GivenNames, "JOHN".to_string());

        let mut wrong_given_truth = HashMap::new();
        wrong_given_truth.insert(CoreField::Surname, "SMITH".to_string());
        wrong_given_truth.insert(CoreField::GivenNames, "JANE".to_string());

        let mut miss_truth = HashMap::new();
        miss_truth.insert(CoreField::Surname, "SMITH".to_string());
        miss_truth.insert(CoreField::GivenNames, "JOHN".to_string());

        let mut off_denominator_truth = HashMap::new();
        off_denominator_truth.insert(CoreField::Surname, "SMITH".to_string());
        off_denominator_truth.insert(CoreField::GivenNames, "JOHN".to_string());

        let prepped = vec![
            // 1: Tier-1 hit, names read exactly right.
            Some(BenchPage {
                asset_id: None,
                source_sha256: None,
                name: "exact-hit".to_string(),
                page: OcrPage::default(),
                ground_truth: Some(exact_truth),
                ground_truth_mrz: None,
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test-1.png"),
                mrz_found: true,
                document_number_leading_filler: false,
                redacted: false,
                mrz_expected: true,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
                pass_objects: None,
            }),
            // 2: Tier-1 hit, but the reader's fixed "JOHN" does not match
            // this document's true given names.
            Some(BenchPage {
                asset_id: None,
                source_sha256: None,
                name: "wrong-name-hit".to_string(),
                page: OcrPage::default(),
                ground_truth: Some(wrong_given_truth),
                ground_truth_mrz: None,
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test-2.png"),
                mrz_found: true,
                document_number_leading_filler: false,
                redacted: false,
                mrz_expected: true,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
                pass_objects: None,
            }),
            // 3: no MRZ found, but still labelled with a name truth —
            // name-scorable, not a hit.
            Some(BenchPage {
                asset_id: None,
                source_sha256: None,
                name: "miss-with-name-truth".to_string(),
                page: OcrPage::default(),
                ground_truth: Some(miss_truth),
                ground_truth_mrz: None,
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test-3.png"),
                mrz_found: false,
                document_number_leading_filler: false,
                redacted: false,
                mrz_expected: true,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
                pass_objects: None,
            }),
            // 4: a Tier-1 hit with no ground truth for either name field —
            // not name-scorable.
            Some(BenchPage {
                asset_id: None,
                source_sha256: None,
                name: "unlabelled-hit".to_string(),
                page: OcrPage::default(),
                ground_truth: None,
                ground_truth_mrz: None,
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test-4.png"),
                mrz_found: true,
                document_number_leading_filler: false,
                redacted: false,
                mrz_expected: true,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
                pass_objects: None,
            }),
            // 5: carries a name truth, but is outside `tier1_hit_rate`'s own
            // scored population (`mrz_expected: false` and `mrz_found:
            // false` together resolve to `NoMrzExpected`, not
            // `FalsePositiveMrz` — see `run_prepped`'s `miss_reason`
            // derivation).
            Some(BenchPage {
                asset_id: None,
                source_sha256: None,
                name: "off-denominator".to_string(),
                page: OcrPage::default(),
                ground_truth: Some(off_denominator_truth),
                ground_truth_mrz: None,
                image_path: PathBuf::from("does-not-need-to-exist-for-this-test-5.png"),
                mrz_found: false,
                document_number_leading_filler: false,
                redacted: false,
                mrz_expected: false,
                printed_zone_nonconforming: false,
                synthetic: false,
                known_or_guessed_format: None,
                ocr_elapsed: Duration::ZERO,
                pass_objects: None,
            }),
        ];
        // `FixedReader` returns the same `hit_evidence` for every document
        // regardless of which one it is asked about, but `miss_reason` is
        // driven by each `BenchPage`'s own `mrz_found`/`mrz_expected`/
        // `redacted` flags, not by `Evidence` alone (see `run_prepped`'s
        // `miss_reason` derivation), so docs 3 and 5 are still correctly
        // `NoMrzFound`/`NoMrzExpected` despite sharing this reader.
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "SMITH",
            given_names: "JOHN",
            evidence: hit_evidence,
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("no duplicate ids");

        let reports = run_prepped(&catalog, &prepped, false, None, false).await;
        let detail = &reports[0].documents_detail;
        assert_eq!(detail[0].miss_reason, None, "doc 1 is a Tier-1 hit");
        assert_eq!(detail[0].names_exact, Some(true));
        assert_eq!(detail[1].miss_reason, None, "doc 2 is a Tier-1 hit");
        assert_eq!(detail[1].names_exact, Some(false));
        assert_eq!(detail[1].name_error, Some(NameError::Other.as_str()));
        assert!(detail[2].miss_reason.is_some(), "doc 3 is a genuine miss");
        assert_eq!(
            detail[2].names_exact,
            Some(true),
            "doc 3 is still name-scorable"
        );
        assert_eq!(
            detail[3].names_exact, None,
            "doc 4 has no name truth at all"
        );
        assert!(
            matches!(detail[4].miss_reason, Some(MissReason::NoMrzExpected)),
            "doc 5 must resolve to the off-denominator miss kind: {:?}",
            detail[4].miss_reason
        );

        match reports[0].strict_tier1_hit_rate {
            StrictNameHitRate::Computed {
                strict_hits,
                name_scorable_documents,
                name_scorable_hits,
                strict_tier1_hit_rate,
                names_exact_among_hits,
            } => {
                assert_eq!(
                    name_scorable_documents, 3,
                    "docs 1-3 are name-scorable and in the scored population; \
                     doc 4 has no name truth, doc 5 is off-denominator"
                );
                assert_eq!(
                    name_scorable_hits, 2,
                    "docs 1-2 are the name-scorable documents that are also Tier-1 hits"
                );
                assert_eq!(
                    strict_hits, 1,
                    "only doc 1 is a hit that also read both names exactly"
                );
                assert_eq!(strict_tier1_hit_rate, 1.0 / 3.0);
                assert_eq!(names_exact_among_hits, Some(0.5));
            }
            StrictNameHitRate::NotApplicable { reason } => {
                panic!("expected a computed strict name hit rate, got NotApplicable: {reason}")
            }
        }
    }

    // ---------------------------------------------------------------------
    // #574: per-document field correctness (`DocumentDetail::field_correctness`).
    // ---------------------------------------------------------------------

    /// A read carrying exactly `values`; every other field is absent.
    fn read_fields(values: &[(CoreField, &str)]) -> ExtractionFields {
        let mut fields = ExtractionFields::default();
        for (field, value) in values {
            fields.set(*field, Some((*value).to_string()));
        }
        fields
    }

    fn truth_of(values: &[(CoreField, &str)]) -> HashMap<CoreField, String> {
        values
            .iter()
            .map(|(field, value)| (*field, (*value).to_string()))
            .collect()
    }

    /// Exact, wrong and unread, per field, on one document; a field the truth
    /// does not define is not in the map at all.
    #[test]
    fn field_correctness_is_exact_wrong_or_unread_per_field_with_truth() {
        let truth = truth_of(&[
            (CoreField::IssuingCountry, "UTO"),
            (CoreField::DocumentNumber, "L898902C3"),
            (CoreField::Surname, "ERIKSSON"),
            (CoreField::DateOfBirth, "1974-08-12"),
            (CoreField::PersonalNumber, "ZE184226B"),
        ]);
        let read = read_fields(&[
            (CoreField::IssuingCountry, "UTO"),
            (CoreField::DocumentNumber, "L898902C8"),
            // A blank answer is no answer.
            (CoreField::Surname, "   "),
            // Read but not in the truth: never in the map.
            (CoreField::Sex, "F"),
        ]);
        let comparison = compare_document(Some(&truth), Some(&read));
        let map = comparison
            .field_correctness(true)
            .expect("a document with truth has a map");
        assert_eq!(
            map,
            BTreeMap::from([
                ("issuing_country", FieldCorrectness::Exact),
                ("document_number", FieldCorrectness::Wrong),
                ("surname", FieldCorrectness::Unread),
                ("date_of_birth", FieldCorrectness::Unread),
                ("personal_number", FieldCorrectness::Unread),
            ])
        );
    }

    /// No accepted read means no field was read, however the fields compare.
    #[test]
    fn field_correctness_is_all_unread_without_an_accepted_read() {
        let truth = truth_of(&[
            (CoreField::Surname, "DOE"),
            (CoreField::DocumentNumber, "L898902C3"),
        ]);
        let read = read_fields(&[
            (CoreField::Surname, "DOE"),
            (CoreField::DocumentNumber, "L898902C8"),
        ]);
        let comparison = compare_document(Some(&truth), Some(&read));
        assert_eq!(
            comparison.field_correctness(false),
            Some(BTreeMap::from([
                ("surname", FieldCorrectness::Unread),
                ("document_number", FieldCorrectness::Unread),
            ]))
        );
        // A reader that returned nothing at all scores the same way.
        let errored = compare_document(Some(&truth), None);
        assert_eq!(
            errored.field_correctness(false),
            comparison.field_correctness(false)
        );
    }

    #[test]
    fn a_document_without_truth_has_no_field_correctness() {
        let read = read_fields(&[(CoreField::Surname, "DOE")]);
        assert!(compare_document(None, Some(&read))
            .field_correctness(true)
            .is_none());
        // Truth that defines none of the fields is no truth either.
        let empty = HashMap::new();
        assert!(compare_document(Some(&empty), Some(&read))
            .field_correctness(true)
            .is_none());
        assert!(compare_document(None, None)
            .field_correctness(false)
            .is_none());
    }

    /// The per-document maps and the aggregate tally are one comparison:
    /// summing `Exact` over the documents' maps reproduces the tally's
    /// per-field exact counts, and every truth field is either exact, wrong or
    /// unread. Issuer and name fields get no normalisation the aggregate path
    /// does not: case and padding differences are `Wrong` in both.
    #[test]
    fn summed_field_correctness_reproduces_the_tally_exact_counts() {
        let documents = [
            (
                truth_of(&[
                    (CoreField::IssuingCountry, "GBR"),
                    (CoreField::Surname, "SMITH"),
                    (CoreField::GivenNames, "ANNA MARIA"),
                    (CoreField::DocumentNumber, "L898902C3"),
                ]),
                read_fields(&[
                    (CoreField::IssuingCountry, "GBR"),
                    (CoreField::Surname, "SMITH"),
                    (CoreField::GivenNames, "ANNA MARIA"),
                    (CoreField::DocumentNumber, "L898902C3"),
                ]),
            ),
            (
                truth_of(&[
                    (CoreField::IssuingCountry, "GBR"),
                    (CoreField::Surname, "SMITH"),
                    (CoreField::GivenNames, "ANNA MARIA"),
                    (CoreField::Sex, "F"),
                ]),
                read_fields(&[
                    // Differences of case and padding only.
                    (CoreField::IssuingCountry, "gbr"),
                    (CoreField::Surname, " SMITH"),
                    (CoreField::GivenNames, "ANNA"),
                ]),
            ),
            (
                truth_of(&[(CoreField::IssuingCountry, "D"), (CoreField::Sex, "M")]),
                read_fields(&[(CoreField::IssuingCountry, "D"), (CoreField::Sex, "F")]),
            ),
        ];

        let mut tally = FieldTally::new();
        let mut summed_exact = vec![0usize; CoreField::ALL.len()];
        let mut summed_truth = vec![0usize; CoreField::ALL.len()];
        for (truth, read) in &documents {
            let comparison = compare_document(Some(truth), Some(read));
            tally.add_document(&comparison.tally);
            let map = comparison
                .field_correctness(true)
                .expect("every document here has truth");
            assert_eq!(map.len(), truth.len());
            for (i, field) in CoreField::ALL.iter().enumerate() {
                if let Some(verdict) = map.get(field.as_str()) {
                    summed_truth[i] += 1;
                    summed_exact[i] += usize::from(*verdict == FieldCorrectness::Exact);
                }
            }
        }

        for (i, field) in CoreField::ALL.iter().enumerate() {
            let (comparisons, exact, _) = tally.per_field[i];
            assert_eq!(
                summed_truth[i], comparisons,
                "{field}: documents with truth"
            );
            assert_eq!(summed_exact[i], exact, "{field}: exact count");
        }
        // The rules the two paths share, pinned on the fields named above.
        let second = compare_document(Some(&documents[1].0), Some(&documents[1].1))
            .field_correctness(true)
            .expect("truth");
        assert_eq!(second["issuing_country"], FieldCorrectness::Wrong);
        assert_eq!(second["surname"], FieldCorrectness::Wrong);
        assert_eq!(second["given_names"], FieldCorrectness::Wrong);
        assert_eq!(second["sex"], FieldCorrectness::Unread);
        let third = compare_document(Some(&documents[2].0), Some(&documents[2].1))
            .field_correctness(true)
            .expect("truth");
        assert_eq!(third["issuing_country"], FieldCorrectness::Exact);
        assert_eq!(third["sex"], FieldCorrectness::Wrong);
    }

    /// End to end through `run_prepped`: the report's maps sum to
    /// `accepted_reads`' exact counts, the serialized key set is pinned, and
    /// no field value, truth or read, reaches the map.
    #[tokio::test]
    async fn report_field_correctness_matches_accepted_reads_and_holds_no_values() {
        let mut evidence = Evidence::default();
        evidence.mrz_checksums_valid = true;
        let reader = std::sync::Arc::new(FixedReader {
            capability: Capability::deterministic_reader(),
            surname: "READSURNAME",
            given_names: "",
            evidence,
        });
        let catalog = synthpass_die::ProviderCatalog::builder()
            .with_reader(reader)
            .build()
            .expect("one reader");
        let page = |name: &str, truth: &[(CoreField, &str)], mrz_found: bool| {
            let mut page = rate_test_page();
            page.name = name.to_string();
            page.mrz_expected = true;
            page.mrz_found = mrz_found;
            if !truth.is_empty() {
                page.ground_truth = Some(truth_of(truth));
            }
            page
        };
        let pages = [
            // Accepted: surname exact, given names unread (the reader answers
            // an empty string), document number unread (no such answer).
            Some(page(
                "accepted-a",
                &[
                    (CoreField::Surname, "READSURNAME"),
                    (CoreField::GivenNames, "TRUTHGIVEN"),
                    (CoreField::DocumentNumber, "TRUTHNUMBER1"),
                ],
                true,
            )),
            // Accepted: surname wrong.
            Some(page(
                "accepted-b",
                &[(CoreField::Surname, "TRUTHSURNAME")],
                true,
            )),
            // No MRZ found: not an accepted read, so every field is unread
            // although the surname would have matched.
            Some(page(
                "not-accepted",
                &[(CoreField::Surname, "READSURNAME")],
                false,
            )),
            // No truth at all.
            Some(page("no-truth", &[], true)),
        ];
        let reports = run_prepped(&catalog, &pages, false, None, false).await;
        let detail = &reports[0].documents_detail;

        let mut exact_per_field: BTreeMap<&str, usize> = BTreeMap::new();
        for d in detail {
            for (field, verdict) in d.field_correctness.iter().flatten() {
                if *verdict == FieldCorrectness::Exact {
                    *exact_per_field.entry(*field).or_default() += 1;
                }
            }
        }
        for field in &reports[0].accuracy.accepted_reads.per_field {
            let expected = field
                .match_rate
                .map_or(0, |rate| (rate * field.documents as f64).round() as usize);
            assert_eq!(
                exact_per_field.get(field.field).copied().unwrap_or(0),
                expected,
                "{}",
                field.field
            );
        }

        assert!(detail[3].field_correctness.is_none());
        let json = serde_json::to_value(crate::report::ProviderRow::from(
            reports.into_iter().next().expect("one report"),
        ))
        .expect("serialize report");
        let rows = json["documents_detail"].as_array().expect("rows");
        assert!(
            !rows[3]
                .as_object()
                .expect("row")
                .contains_key("field_correctness"),
            "a document without truth has no key: {}",
            rows[3]
        );
        assert_eq!(
            rows[0]["field_correctness"],
            serde_json::json!({
                "document_number": "unread",
                "given_names": "unread",
                "surname": "exact",
            })
        );
        assert_eq!(
            rows[1]["field_correctness"],
            serde_json::json!({ "surname": "wrong" })
        );
        assert_eq!(
            rows[2]["field_correctness"],
            serde_json::json!({ "surname": "unread" })
        );
        // Names and verdicts only: no value, truth or read, is in any map.
        for row in &rows[..3] {
            let map = row["field_correctness"].to_string();
            for value in ["READSURNAME", "TRUTHSURNAME", "TRUTHGIVEN", "TRUTHNUMBER1"] {
                assert!(!map.contains(value), "{value} leaked into {map}");
            }
        }
    }

    // --- replay (ADR-0024, amendment 3) -----------------------------------
    //
    // Synthetic text throughout: the zone below is the same made-up specimen
    // the tests above use, never a real document's OCR (ADR-0027).

    const REPLAY_LINE_1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    const REPLAY_LINE_2_VALID: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";
    /// The same line with a malformed date of birth: it parses, and fails a check.
    const REPLAY_LINE_2_BROKEN: &str = "L898902C36UTO7413122F1204159ZE184226B<<<<<10";

    fn replay_doc(name: &str, labels: Option<synthpass_core::Extraction>) -> RealSpecimenDoc {
        RealSpecimenDoc {
            name: name.to_string(),
            asset_id: format!("passports/{name}.png"),
            source_sha256: format!("{name:0<64}"),
            image: image::DynamicImage::new_rgb8(1, 1),
            labels,
            class: crate::SpecimenClass::Passport,
            mrz_expected: true,
        }
    }

    fn replay_labels() -> synthpass_core::Extraction {
        let mut labels = synthpass_core::Extraction::default();
        labels.document_number = Some("L898902C3".to_string());
        labels.surname = Some("ERIKSSON".to_string());
        labels.given_names = Some("ANNA MARIA".to_string());
        labels.mrz_line = Some(format!("{REPLAY_LINE_1}\n{REPLAY_LINE_2_VALID}"));
        labels
    }

    /// Three documents, each exercising other page values: a labelled hit with a
    /// retry variant, a checksum miss with a band score and a spent budget, and a
    /// page with no MRZ at all.
    fn replay_corpus() -> Vec<(RealSpecimenDoc, OcrPage)> {
        vec![
            (
                replay_doc("specimen-a", Some(replay_labels())),
                OcrPage {
                    text: format!("REPUBLIC OF UTOPIA\n{REPLAY_LINE_1}\n{REPLAY_LINE_2_VALID}"),
                    rotation: 0,
                    mrz_band_score: Some(0.8125),
                    retry_variant_id: Some("pass-01".to_string()),
                    retry_damaged_recovery: Some(false),
                    retry_stop: Some("variant_valid".to_string()),
                    chargrid: Some("unchanged".to_string()),
                    ..OcrPage::default()
                },
            ),
            (
                replay_doc("specimen-b", None),
                OcrPage {
                    text: format!("{REPLAY_LINE_1}\n{REPLAY_LINE_2_BROKEN}"),
                    rotation: 90,
                    mrz_band_score: Some(0.6125),
                    retry_budget_hit: true,
                    retry_stop: Some("budget".to_string()),
                    ..OcrPage::default()
                },
            ),
            (
                replay_doc("specimen-c", None),
                OcrPage {
                    text: "NOTHING MRZ-SHAPED HERE".to_string(),
                    rotation: 180,
                    retry_stop: Some("exhausted".to_string()),
                    ..OcrPage::default()
                },
            ),
        ]
    }

    /// The capture file a live run over `corpus` would have written.
    fn replay_capture(corpus: &[(RealSpecimenDoc, OcrPage)]) -> crate::ocr_passes::PassesFile {
        let mut collector = crate::ocr_passes::OcrPassesCollector::new(Some("run.json".into()));
        for (doc, page) in corpus {
            collector.push(
                &doc.name,
                Some(&doc.asset_id),
                Some(&doc.source_sha256),
                page,
                &[],
            );
        }
        crate::ocr_passes::PassesFile {
            rows: collector.rows().to_vec(),
            missing_keys: Vec::new(),
        }
    }

    fn replay_specimens(corpus: &[(RealSpecimenDoc, OcrPage)]) -> Vec<RealSpecimenDoc> {
        corpus.iter().map(|(doc, _)| doc.clone()).collect()
    }

    fn mrz_catalog() -> ProviderCatalog {
        ProviderCatalog::builder()
            .with_reader(std::sync::Arc::new(synthpass_die::MrzReader::new()))
            .build()
            .expect("one reader")
    }

    fn scratch_dir(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "provider-bench-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    // --- the per-document archive (ADR-0024) -------------------------------

    /// A run header for a test: only the fields a record's shape or a file's name depends on.
    fn archive_test_header(started_unix_ms: u64) -> crate::archive::RunHeader {
        use crate::archive::{Machine, RetryBudget, RunHeader, Scope, TrackFlags, SCHEMA};
        RunHeader {
            kind: "run",
            schema: SCHEMA,
            run_id: String::new(),
            binary_name: "provider-bench".to_string(),
            started_unix_ms,
            pid: 1,
            source: "local",
            machine: Machine {
                label: "test".to_string(),
                os: "testos",
                arch: "testarch",
                cpus: 1,
            },
            binary: "provider-bench".to_string(),
            binary_sha256: None,
            git_commit: None,
            working_tree_dirty: None,
            argv: Vec::new(),
            scope: Scope {
                corpus: "real-specimens",
                format: None,
                limit: None,
                document_type: None,
                profile: None,
                seed_start: None,
                count: 3,
            },
            tracks: TrackFlags {
                private: false,
                local: false,
                covers: false,
            },
            samples_data_sha: None,
            corpus_manifest_sha256: None,
            documents_loaded: 3,
            labelled_loaded: 1,
            providers: Vec::new(),
            ocr_arms: BTreeMap::new(),
            retry_budget: Some(RetryBudget {
                max_passes: 14,
                max_seconds: 52,
            }),
            mrz_arms: BTreeMap::new(),
            pivot_yy: 26,
            model_paths: crate::report::ModelPathsReport::default(),
            replay_of: None,
            env: BTreeMap::new(),
        }
        .with_run_id()
    }

    /// Every line of every finished archive file under `root/track`, parsed.
    fn archive_lines(root: &Path, track: &str) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(root.join(track)) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "jsonl") {
                let body = std::fs::read_to_string(&path).expect("read the archive file");
                out.extend(
                    body.lines()
                        .map(|line| serde_json::from_str(line).expect("a JSON line")),
                );
            }
        }
        out
    }

    fn two_provider_catalog() -> ProviderCatalog {
        ProviderCatalog::builder()
            .with_reader(std::sync::Arc::new(synthpass_die::MrzReader::new()))
            .with_reader(std::sync::Arc::new(ErroringReader {
                capability: Capability::deterministic_reader(),
            }))
            .build()
            .expect("two readers")
    }

    #[tokio::test]
    async fn the_archive_sees_one_record_per_document_per_provider() {
        let root = scratch_dir("archive-per-provider");
        let corpus = replay_corpus();
        let archive = Archive::start(&root, &archive_test_header(1_000_000_000_000));
        run_provider_bench_replay(
            &two_provider_catalog(),
            &replay_specimens(&corpus),
            &replay_capture(&corpus),
            false,
            &RealDumpOptions {
                archive: Some(&archive),
                ..RealDumpOptions::default()
            },
            false,
        )
        .await
        .unwrap_or_else(|e| panic!("covered: {e}"));
        // Before `finish`, the per-provider flush alone has put every record on disk: a run
        // killed now would still leave them in the `.partial`.
        let partial = std::fs::read_dir(root.join("public"))
            .expect("the public track directory")
            .flatten()
            .map(|entry| entry.path())
            .find(|path| path.extension().is_some_and(|ext| ext == "partial"))
            .expect("the run is still partial");
        let on_disk = std::fs::read_to_string(&partial).expect("readable while open");
        assert_eq!(
            on_disk.lines().count(),
            1 + 3 * 2,
            "every record is on disk before `finish`"
        );
        assert!(on_disk.ends_with('\n'), "no record is cut short");
        archive.finish();

        let lines = archive_lines(&root, "public");
        assert_eq!(
            lines.len(),
            1 + 3 * 2,
            "one header, three documents, two providers"
        );
        assert_eq!(lines[0]["kind"], "run");
        let docs = &lines[1..];
        assert!(docs
            .iter()
            .all(|d| d["kind"] == "doc" && d["track"] == "public"));
        assert!(docs.iter().all(|d| d["run_id"] == lines[0]["run_id"]));
        for provider in ["mrz", "erroring-test-reader"] {
            let mut names: Vec<&str> = docs
                .iter()
                .filter(|d| d["provider"] == provider)
                .map(|d| d["name"].as_str().expect("name"))
                .collect();
            names.sort_unstable();
            assert_eq!(
                names,
                ["specimen-a", "specimen-b", "specimen-c"],
                "{provider}"
            );
        }
        // The error arm is a record too: no read, so no fields.
        let errored = docs
            .iter()
            .find(|d| d["provider"] == "erroring-test-reader" && d["name"] == "specimen-a")
            .expect("the erroring provider's record");
        assert_eq!(errored["read_ok"], false);
        assert!(errored["fields"].is_null());
        assert_eq!(errored["ledger_row"]["outcome"], "ocr_error");
        // A replay traces, so the record carries its (empty) pass list; the labelled document
        // carries a comparison against its fixture, the others none.
        let hit = docs
            .iter()
            .find(|d| d["provider"] == "mrz" && d["name"] == "specimen-a")
            .expect("the mrz record of the labelled hit");
        assert_eq!(hit["ledger_row"]["outcome"], "hit");
        assert_eq!(hit["fields"]["surname"], "ERIKSSON");
        assert_eq!(hit["ocr"]["ocr_passes"], serde_json::json!([]));
        assert_eq!(hit["ocr"]["rotation"], 0);
        assert_eq!(hit["tier1_read"]["valid"], true);
        assert_eq!(hit["truth"]["zone_mismatch"], 0);
        let unlabelled = docs
            .iter()
            .find(|d| d["provider"] == "mrz" && d["name"] == "specimen-b")
            .expect("the mrz record of an unlabelled document");
        assert!(unlabelled["truth"].is_null());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// What a run reports, with the timings that differ between two runs of the same input
    /// left out (`speed`, `ocr_elapsed`, `measured_rss_delta_bytes`): each provider's whole
    /// report as text, its `documents_detail` rows included, and the ledger rows as the ledger
    /// writes them. A replay's OCR time is zero, so the per-document detail is deterministic.
    fn run_outputs(reports: &[ProviderReport]) -> (Vec<String>, Vec<crate::report::OutcomeRow>) {
        (
            reports
                .iter()
                .map(|r| {
                    format!(
                        "{} {} {:?} {:?} {:?} {:?} {:?} {:?} {:?}",
                        r.provider_id,
                        r.documents,
                        r.accuracy,
                        r.json_validity,
                        r.unsupported_assertion,
                        r.declared_resident_bytes,
                        r.tier1_hit_rate,
                        r.ocr_arms,
                        (&r.strict_tier1_hit_rate, &r.documents_detail),
                    )
                })
                .collect(),
            reports
                .iter()
                .flat_map(|r| {
                    r.documents_detail
                        .iter()
                        .map(crate::report::OutcomeRow::from)
                })
                .collect(),
        )
    }

    /// One replay over the three-document corpus, with the OCR dump on so its bytes can be
    /// compared, and `archive` (or none) attached.
    async fn replay_with_dump(
        dump_dir: &Path,
        archive: Option<&Archive>,
    ) -> (Vec<ProviderReport>, Vec<u8>) {
        let corpus = replay_corpus();
        let reports = run_provider_bench_replay(
            &two_provider_catalog(),
            &replay_specimens(&corpus),
            &replay_capture(&corpus),
            false,
            &RealDumpOptions {
                ocr_dir: Some(dump_dir),
                ocr_hits: true,
                archive,
                ..RealDumpOptions::default()
            },
            false,
        )
        .await
        .unwrap_or_else(|e| panic!("covered: {e}"));
        let dump = std::fs::read(dump_dir.join("provider-bench-miss-ocr-dump.jsonl"))
            .expect("the dump was written");
        (reports, dump)
    }

    #[tokio::test]
    async fn the_archive_changes_no_report_or_dump() {
        let root = scratch_dir("archive-neutral-root");
        let without_dir = scratch_dir("archive-neutral-without");
        let with_dir = scratch_dir("archive-neutral-with");
        let (without, dump_without) = replay_with_dump(&without_dir, None).await;
        let archive = Archive::start(&root, &archive_test_header(1_000_000_000_000));
        let (with, dump_with) = replay_with_dump(&with_dir, Some(&archive)).await;
        archive.finish();

        assert!(
            !archive_lines(&root, "public").is_empty(),
            "the archive did write"
        );
        assert_eq!(run_outputs(&with), run_outputs(&without));
        assert_eq!(
            dump_with, dump_without,
            "the miss dump is byte for byte the same"
        );
        for dir in [&root, &without_dir, &with_dir] {
            let _ = std::fs::remove_dir_all(dir);
        }
    }

    #[tokio::test]
    async fn a_failing_archive_changes_no_report() {
        let dir = scratch_dir("archive-failing");
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let root_is_a_file = dir.join("not-a-directory");
        std::fs::write(&root_is_a_file, b"x").expect("a file where the root should be");
        let without_dir = dir.join("without");
        let with_dir = dir.join("with");
        let (without, dump_without) = replay_with_dump(&without_dir, None).await;
        let archive = Archive::start(&root_is_a_file, &archive_test_header(1_000_000_000_000));
        let (with, dump_with) = replay_with_dump(&with_dir, Some(&archive)).await;
        archive.finish();

        assert_eq!(archive.warnings().len(), 1, "one warning, then off");
        assert_eq!(run_outputs(&with), run_outputs(&without));
        assert_eq!(dump_with, dump_without);
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A labelled specimen whose OCR text is one zone and whose fixture is another, so the
    /// fixture's text can be told apart from the OCR text in the bytes of a record.
    fn labelled_page_with_a_different_fixture_zone() -> BenchPage {
        let mut page = rate_test_page();
        page.name = "labelled".to_string();
        page.asset_id = Some("passports/labelled.png".to_string());
        page.source_sha256 = Some("0".repeat(64));
        page.page = OcrPage {
            text: format!("{REPLAY_LINE_1}\n{REPLAY_LINE_2_VALID}"),
            ..OcrPage::default()
        };
        page.mrz_found = true;
        page.mrz_expected = true;
        let mut fixture_line_1 = String::from("P<UTOSENTINELTRUTH<<ZQXJ");
        while fixture_line_1.len() < 44 {
            fixture_line_1.push('<');
        }
        page.ground_truth_mrz = Some(format!("{fixture_line_1}\n{REPLAY_LINE_2_VALID}"));
        page.ground_truth = Some(HashMap::from([(
            CoreField::Surname,
            "ERIKSSON".to_string(),
        )]));
        page.known_or_guessed_format = Some("TD3");
        page
    }

    #[tokio::test]
    async fn a_doc_record_never_holds_the_fixture_zone() {
        let root = scratch_dir("archive-no-fixture");
        let archive = Archive::start(&root, &archive_test_header(1_000_000_000_000));
        let prepped = vec![Some(labelled_page_with_a_different_fixture_zone())];
        run_prepped_with_dump_options(
            &mrz_catalog(),
            &prepped,
            false,
            None,
            false,
            None,
            false,
            Some(&archive),
        )
        .await;
        archive.finish();

        let file = std::fs::read_dir(root.join("public"))
            .expect("public/")
            .flatten()
            .next()
            .expect("one file")
            .path();
        let bytes = std::fs::read_to_string(&file).expect("read");
        assert!(bytes.contains("ERIKSSON"), "the OCR text is in the record");
        for fixture_only in ["SENTINELTRUTH", "ZQXJ"] {
            assert!(
                !bytes.contains(fixture_only),
                "{fixture_only} is fixture text"
            );
        }
        // What the record does hold about the fixture: counts and positions.
        let doc: serde_json::Value =
            serde_json::from_str(bytes.lines().nth(1).expect("one record")).expect("JSON");
        assert!(doc["truth"]["zone_mismatch"]
            .as_u64()
            .is_some_and(|n| n > 0));
        assert!(doc["truth"]["compared_cells"].as_u64().is_some());
        assert!(doc["truth"]["field_mismatch"]["by_field"].is_object());
        assert!(
            doc["ocr"]["ocr_passes"].is_null(),
            "an untraced run adds no tracing"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn a_real_page_without_an_asset_id_is_not_archived() {
        // It cannot be placed in a track, so it is treated as the most restrictive one.
        let root = scratch_dir("archive-no-asset-id");
        let archive = Archive::start(&root, &archive_test_header(1_000_000_000_000));
        let mut page = labelled_page_with_a_different_fixture_zone();
        page.asset_id = None;
        run_prepped_with_dump_options(
            &mrz_catalog(),
            &[Some(page)],
            false,
            None,
            false,
            None,
            false,
            Some(&archive),
        )
        .await;
        archive.finish();
        assert!(!root.join("public").exists() && !root.join("local").exists());
        assert!(!root.join("private").exists());
        assert!(archive.warnings().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn a_private_asset_is_never_archived() {
        let root = scratch_dir("archive-private-asset");
        let archive = Archive::start(&root, &archive_test_header(1_000_000_000_000));
        let mut page = labelled_page_with_a_different_fixture_zone();
        page.asset_id = Some("private/labelled.png".to_string());
        run_prepped_with_dump_options(
            &mrz_catalog(),
            &[Some(page)],
            false,
            None,
            false,
            None,
            false,
            Some(&archive),
        )
        .await;
        archive.finish();
        assert!(!root.join("private").exists());
        assert!(!root.join("public").exists() && !root.join("local").exists());
        assert!(archive.warnings().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    /// The property the whole feature rests on: for the same page, a replayed row
    /// yields the same `DocumentDetail`, the same ledger row and the same dump
    /// row as the live path. Only `ocr_ms` differs, because no OCR ran.
    #[tokio::test]
    async fn a_replayed_row_scores_exactly_as_the_live_page_does() {
        let corpus = replay_corpus();
        let live: Vec<Option<BenchPage>> = corpus
            .iter()
            .map(|(doc, page)| {
                Some(specimen_bench_page(
                    doc,
                    page.clone(),
                    PathBuf::from("live-unused.png"),
                    Duration::from_millis(1234),
                ))
            })
            .collect();
        let capture = replay_capture(&corpus);
        let replayed = replay_pages(&replay_specimens(&corpus), &capture)
            .unwrap_or_else(|e| panic!("the capture covers the corpus: {e}"));

        let catalog = mrz_catalog();
        let (live_dir, replay_dir) = (scratch_dir("live"), scratch_dir("replay"));
        // Hits are dumped too, so the dump holds a row of every kind.
        let live_reports = run_prepped_with_dump_options(
            &catalog,
            &live,
            false,
            Some(&live_dir),
            true,
            Some("run.json"),
            false,
            None,
        )
        .await;
        let replay_reports = run_prepped_with_dump_options(
            &catalog,
            &replayed,
            false,
            Some(&replay_dir),
            true,
            Some("run.json"),
            false,
            None,
        )
        .await;

        // The ledger rows, with the one timing field zeroed.
        let ledger = |reports: &[ProviderReport]| -> Vec<crate::report::OutcomeRow> {
            reports[0]
                .documents_detail
                .iter()
                .map(|d| crate::report::OutcomeRow {
                    ocr_ms: 0,
                    ..crate::report::OutcomeRow::from(d)
                })
                .collect()
        };
        assert_eq!(ledger(&live_reports), ledger(&replay_reports));
        assert_eq!(
            live_reports[0].documents_detail[0].ocr_elapsed.as_millis(),
            1234
        );
        assert_eq!(
            replay_reports[0].documents_detail[0].ocr_elapsed,
            Duration::ZERO
        );

        // Every recorded per-document field, as the report writes it.
        let detail = |reports: Vec<ProviderReport>| -> Vec<serde_json::Value> {
            let json = serde_json::to_value(crate::report::ProviderRow::from(
                reports.into_iter().next().expect("one report"),
            ))
            .expect("serialize");
            let mut rows = json["documents_detail"].as_array().expect("rows").clone();
            for row in &mut rows {
                row.as_object_mut().expect("row").remove("ocr_ms");
            }
            rows
        };
        let (live_rows, replay_rows) = (detail(live_reports), detail(replay_reports));
        assert_eq!(live_rows, replay_rows);
        // The two values the row gained are what this comparison could not
        // have passed without.
        assert_eq!(live_rows[0]["retry_damaged_recovery"], false);
        assert_eq!(live_rows[0]["chargrid"], "unchanged");
        assert_eq!(live_rows[1]["retry_budget_hit"], true);
        assert_eq!(live_rows[1]["miss_reason"], "checksum_failed");

        // The dump: byte-identical files, with a row for the hit, the checksum
        // miss and the page with no zone.
        let dump = |dir: &Path| {
            std::fs::read_to_string(dir.join("provider-bench-miss-ocr-dump.jsonl"))
                .expect("dump written")
        };
        let (live_dump, replay_dump) = (dump(&live_dir), dump(&replay_dir));
        let _ = std::fs::remove_dir_all(&live_dir);
        let _ = std::fs::remove_dir_all(&replay_dir);
        assert_eq!(live_dump, replay_dump);
        let dump_rows: Vec<serde_json::Value> = live_dump
            .lines()
            .map(|line| serde_json::from_str(line).expect("row"))
            .collect();
        assert_eq!(dump_rows.len(), 3);
        assert_eq!(dump_rows[0]["mrz_band_score"], 0.8125);
        assert_eq!(dump_rows[1]["mrz_band_score"], 0.6125);
        assert!(dump_rows[2]["mrz_band_score"].is_null());
    }

    #[test]
    fn a_replayed_page_takes_the_documents_own_values_from_the_corpus_not_the_row() {
        let corpus = replay_corpus();
        let capture = replay_capture(&corpus);
        let pages = replay_pages(&replay_specimens(&corpus), &capture)
            .unwrap_or_else(|e| panic!("covered: {e}"));
        let labelled = pages[0].as_ref().expect("a page per document");
        assert_eq!(labelled.name, "specimen-a");
        assert_eq!(
            labelled.asset_id.as_deref(),
            Some("passports/specimen-a.png")
        );
        assert!(labelled.mrz_found, "recomputed from the row's text");
        assert!(labelled.ground_truth.is_some() && labelled.ground_truth_mrz.is_some());
        assert!(!labelled.synthetic && labelled.mrz_expected);
        assert_eq!(labelled.ocr_elapsed, Duration::ZERO);
        assert_eq!(labelled.page.mrz_band_score, Some(0.8125));
        assert_eq!(labelled.page.retry_damaged_recovery, Some(false));
        assert_eq!(labelled.page.rotation, 0);
        assert!(!pages[2].as_ref().expect("page").mrz_found);
        assert_eq!(pages[1].as_ref().expect("page").page.rotation, 90);
    }

    #[test]
    fn rows_are_matched_by_asset_id_and_come_out_in_corpus_order() {
        let corpus = replay_corpus();
        let mut capture = replay_capture(&corpus);
        capture.rows.reverse();
        let pages = replay_pages(&replay_specimens(&corpus), &capture)
            .unwrap_or_else(|e| panic!("covered: {e}"));
        let names: Vec<&str> = pages
            .iter()
            .map(|page| page.as_ref().expect("page").name.as_str())
            .collect();
        assert_eq!(names, ["specimen-a", "specimen-b", "specimen-c"]);
        assert_eq!(
            pages[2].as_ref().expect("page").page.text,
            "NOTHING MRZ-SHAPED HERE"
        );
    }

    #[test]
    fn a_capture_lacking_a_key_the_replay_needs_is_refused_naming_the_keys() {
        let corpus = replay_corpus();
        let mut capture = replay_capture(&corpus);
        capture.missing_keys = vec!["retry_damaged_recovery", "mrz_band_score"];
        let err = check_replay(&replay_specimens(&corpus), &capture).expect_err("refused");
        assert!(
            err.contains("retry_damaged_recovery, mrz_band_score"),
            "{err}"
        );
    }

    #[test]
    fn a_capture_that_does_not_cover_the_corpus_exactly_once_is_refused() {
        let corpus = replay_corpus();
        let specimens = replay_specimens(&corpus);

        let mut duplicated = replay_capture(&corpus);
        duplicated.rows.push(duplicated.rows[1].clone());
        let err = check_replay(&specimens, &duplicated).expect_err("a duplicate row");
        assert!(
            err.contains("more than one row: passports/specimen-b.png"),
            "{err}"
        );

        let mut short = replay_capture(&corpus);
        short.rows.remove(0);
        let err = check_replay(&specimens, &short).expect_err("a missing row");
        assert!(err.contains("no row: passports/specimen-a.png"), "{err}");

        let mut extra = replay_capture(&corpus);
        extra.rows.push(crate::ocr_passes::OcrPassesRow {
            asset_id: Some("passports/other.png".to_string()),
            ..extra.rows[0].clone()
        });
        let err = check_replay(&specimens, &extra).expect_err("a row for another asset");
        assert!(err.contains("does not hold: passports/other.png"), "{err}");

        let mut anonymous = replay_capture(&corpus);
        anonymous.rows[2].asset_id = None;
        let err = check_replay(&specimens, &anonymous).expect_err("a row with no asset_id");
        assert!(err.contains("capture row 3 has no asset_id"), "{err}");

        // A replay over a subset of a capture is not the capture's corpus either.
        let err = check_replay(&specimens[..2], &replay_capture(&corpus)).expect_err("a subset");
        assert!(err.contains("passports/specimen-c.png"), "{err}");
        assert!(err.contains("exactly once"), "{err}");
    }

    #[test]
    fn a_row_read_from_other_image_bytes_is_refused_naming_the_asset() {
        let corpus = replay_corpus();
        let mut capture = replay_capture(&corpus);
        capture.rows[1].source_sha256 = Some("f".repeat(64));
        let err = check_replay(&replay_specimens(&corpus), &capture).expect_err("refused");
        assert!(err.contains("source_sha256 differs"), "{err}");
        assert!(err.contains("passports/specimen-b.png"), "{err}");
        assert!(
            !err.contains("specimen-a"),
            "only the differing asset: {err}"
        );
        capture.rows[1].source_sha256 = None;
        assert!(check_replay(&replay_specimens(&corpus), &capture).is_err());
    }

    #[test]
    fn a_long_list_of_offending_ids_is_cut_to_five() {
        let ids: Vec<String> = (0..8).map(|n| format!("passports/{n}.png")).collect();
        let refs: Vec<&str> = ids.iter().map(String::as_str).collect();
        let listed = some_ids(&refs);
        assert!(listed.ends_with(", and 3 more"), "{listed}");
        assert!(listed.contains("passports/4.png") && !listed.contains("passports/5.png"));
        assert_eq!(some_ids(&refs[..2]), "passports/0.png, passports/1.png");
    }

    #[tokio::test]
    async fn a_replay_refuses_to_write_a_pass_trace() {
        let corpus = replay_corpus();
        let dir = scratch_dir("no-trace");
        let dumps = RealDumpOptions {
            ocr_passes_dir: Some(dir.as_path()),
            ..RealDumpOptions::default()
        };
        let err = run_provider_bench_replay(
            &mrz_catalog(),
            &replay_specimens(&corpus),
            &replay_capture(&corpus),
            false,
            &dumps,
            false,
        )
        .await
        .err()
        .expect("refused");
        assert!(err.contains("writes none"), "{err}");
        assert!(!dir.exists(), "nothing was written");
    }

    #[tokio::test]
    async fn a_replay_runs_the_scoring_path_and_reports_the_captured_retry_state() {
        let corpus = replay_corpus();
        let reports = run_provider_bench_replay(
            &mrz_catalog(),
            &replay_specimens(&corpus),
            &replay_capture(&corpus),
            false,
            &RealDumpOptions::default(),
            false,
        )
        .await
        .unwrap_or_else(|e| panic!("covered: {e}"));
        let detail = &reports[0].documents_detail;
        assert_eq!(detail.len(), 3);
        assert!(
            detail[0].miss_reason.is_none(),
            "the labelled zone is a hit"
        );
        assert_eq!(detail[0].names_exact, Some(true));
        assert_eq!(detail[0].retry_variant_id.as_deref(), Some("pass-01"));
        assert_eq!(detail[0].retry_damaged_recovery, Some(false));
        assert!(detail[1].retry_budget_hit);
        assert_eq!(detail[1].ocr_elapsed, Duration::ZERO);
    }

    // ---- #574: the shadow line-1 selector in the benchmark ----

    /// The ICAO specimen's line 2, which every selector case pairs with a line 1
    /// of its own making. Constructed text only: no real-specimen OCR.
    const SELECTOR_LINE2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

    /// A TD3 line 1 for the specimen's issuer, filler-padded to 44 cells. No
    /// name holds a `K` or an `L`, which the scanner's repairs read as fillers.
    fn selector_line1(name_field: &str) -> String {
        let mut line = format!("P<UTO{name_field}");
        while line.len() < 44 {
            line.push('<');
        }
        line
    }

    /// The scanner accepts the first line 1 as read (its name field breaks the
    /// grammar), and the second is the well-formed reading of the same line.
    fn broken_then_good_text() -> String {
        format!(
            "{}\n{SELECTOR_LINE2}\n\n{}\n{SELECTOR_LINE2}",
            selector_line1("SPECIMEN<<TESTXYZ<<<<Q"),
            selector_line1("SPECIMEN<<TESTXYZ"),
        )
    }

    fn control_read(text: &str) -> synthpass_die::Tier1Read {
        synthpass_die::read_tier1_with(
            text,
            &synthpass_die::mrz_parse_options_for(false, false, false),
            synthpass_die::Line1Arm::Control,
        )
    }

    fn provenance(id: &str, transform: &str, readings: &[&str]) -> PassProvenance {
        PassProvenance {
            id: id.to_string(),
            transform: transform.to_string(),
            readings: readings.iter().map(|r| r.to_string()).collect(),
        }
    }

    /// The source fields name the passes whose readings hold the proposed
    /// line 1 once whitespace is stripped, in execution order, each transform
    /// once.
    #[test]
    fn a_proposal_is_traced_to_the_passes_that_read_it() {
        let read = control_read(&broken_then_good_text());
        let summary = read.line1.expect("control records a verdict");
        assert_eq!(summary.outcome, synthpass_die::Line1Outcome::Proposed);
        let good = selector_line1("SPECIMEN<<TESTXYZ");
        assert_eq!(read.proposed_line1.as_deref(), Some(good.as_str()));

        let spaced = format!("{} {}", &good[..12], &good[12..]);
        let passes = vec![
            provenance(
                "general",
                "general",
                &[&selector_line1("SPECIMEN<<TESTXYZ<<<<Q"), SELECTOR_LINE2],
            ),
            provenance("pass-02", "mrz_variants:2", &[&spaced, SELECTOR_LINE2]),
            provenance("pass-03", "mrz_variants:2", &[&good]),
            provenance("pass-04", "mrz_variants:3", &["NOT<A<ZONE"]),
        ];
        let detail = line1_selection_detail(&summary, read.proposed_line1.as_deref(), &passes);

        assert_eq!(detail.arm, "control");
        assert_eq!(detail.verdict, "proposed");
        assert_eq!(detail.reason, None);
        assert_eq!((detail.eligible, detail.distinct), (1, 1));
        assert!(detail.names_changed);
        assert_eq!(detail.source_passes, vec!["pass-02", "pass-03"]);
        assert_eq!(detail.source_transforms, vec!["mrz_variants:2"]);

        // No pass trace, no provenance; the verdict is still reported.
        let untraced = line1_selection_detail(&summary, read.proposed_line1.as_deref(), &[]);
        assert!(untraced.source_passes.is_empty() && untraced.source_transforms.is_empty());
        assert_eq!(untraced.verdict, "proposed");
    }

    /// A verdict that is not a proposal never claims a source, even when a
    /// pass happens to hold the same line.
    #[test]
    fn only_a_proposal_carries_a_source() {
        let zone = format!("{}\n{SELECTOR_LINE2}", selector_line1("SPECIMEN<<TESTXYZ"));
        let read = control_read(&zone);
        let summary = read.line1.expect("control records a verdict");
        assert_eq!(summary.outcome, synthpass_die::Line1Outcome::NoAction);
        let passes = vec![provenance(
            "pass-01",
            "general",
            &[&selector_line1("SPECIMEN<<TESTXYZ")],
        )];
        let detail = line1_selection_detail(
            &summary,
            Some(&selector_line1("SPECIMEN<<TESTXYZ")),
            &passes,
        );
        assert_eq!(detail.verdict, "none");
        assert_eq!(detail.reason, Some("kept"));
        assert!(detail.source_passes.is_empty());
    }

    /// The reading's own evidence decides whether a document carries a
    /// selection: absent (the arm is `off`) writes none, present is passed
    /// through.
    #[tokio::test]
    async fn a_document_carries_the_selection_its_reading_recorded() {
        let read = control_read(&broken_then_good_text());
        let mut recording = Evidence::default();
        recording.line1_selection = read.line1;

        let mut page = rate_test_page();
        page.page = OcrPage {
            text: "no zone here".to_string(),
            ..OcrPage::default()
        };
        let prepped = vec![Some(page)];
        for (evidence, expected) in [(Evidence::default(), None), (recording, Some("proposed"))] {
            let catalog = synthpass_die::ProviderCatalog::builder()
                .with_reader(std::sync::Arc::new(FixedReader {
                    evidence,
                    ..FixedReader::default()
                }))
                .build()
                .expect("no duplicate ids");
            let reports = run_prepped(&catalog, &prepped, false, None, false).await;
            let detail = &reports[0].documents_detail[0];
            assert_eq!(detail.line1_selection.as_ref().map(|s| s.verdict), expected);
        }
    }

    /// #579: the printed-zone validity parse ignores the repeated-line refusal
    /// and the date-digits arm, so the `checksum_failed_specimen` denominator is
    /// the same in every arm; the other options are untouched.
    #[test]
    fn the_printed_zone_parse_ignores_the_refusal_and_date_digits_arms() {
        let refusing = synthpass_die::mrz_parse_options_for(false, true, false);
        assert!(refusing.refuse_repeated_line);
        assert!(!printed_zone_parse_options_from(refusing).refuse_repeated_line);
        let digits = synthpass_die::mrz_parse_options_for(false, false, true);
        assert!(digits.date_digits);
        assert!(!printed_zone_parse_options_from(digits).date_digits);
        let swept = synthpass_die::mrz_parse_options_for(true, true, true);
        let pinned = printed_zone_parse_options_from(swept);
        assert!(pinned.class_sweep);
        assert!(!pinned.refuse_repeated_line);
        assert!(!pinned.date_digits);
        assert_eq!(
            printed_zone_parse_options_from(mrz::ParseOptions::default()),
            mrz::ParseOptions::default()
        );
    }

    /// With every `SYNTHPASS_MRZ_*` arm `off` the dump zone comes from
    /// `read_tier1`, which is what `mrz::find_and_parse` returns: the dump is
    /// byte-identical to what it was before the arms reached it (#574, finding
    /// 3). Pins the equivalence over the shapes a dump sees: a clean zone, a
    /// broken-name zone, a checksum failure, prose and a merged zone.
    #[test]
    fn the_dump_zone_with_every_arm_off_is_what_find_and_parse_returns() {
        let specimen = format!("P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\n{SELECTOR_LINE2}");
        let texts = [
            specimen.clone(),
            broken_then_good_text(),
            specimen.replace("L898902C36UTO", "L898902C35UTO"),
            "just some ordinary prose with no machine readable zone".to_string(),
            format!("{} {SELECTOR_LINE2}", selector_line1("SPECIMEN<<TESTXYZ")),
        ];
        for text in &texts {
            let off = synthpass_die::read_tier1_with(
                text,
                &synthpass_die::mrz_parse_options_for(false, false, false),
                synthpass_die::Line1Arm::Off,
            );
            assert_eq!(off.parsed, mrz::find_and_parse(text), "{text}");
            assert_eq!(off.line1, None, "{text}");
        }
    }
}
