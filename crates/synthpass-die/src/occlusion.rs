//! Applying the wire-level [`MrzOcclusion`] observation to an already-parsed
//! MRZ zone — the `synthpass-die` side of ADR-0026, wrapping
//! [`mrz::apply_occlusion`].
//!
//! [`mrz`] knows nothing about the schema crate's vocabulary
//! ([`CoreField`], [`MrzOcclusion`]); this module is the one place that
//! translates between them, so [`crate::mrz_reader::MrzReader`] never has to.

use mrz::{CellMask, MrzData, MrzError, ZoneField};
use synthpass_core::v2::{CoreField, MrzOcclusion};

/// Why [`apply`] refused to produce a masked reading. Every variant means
/// the same thing downstream: [`crate::mrz_reader::MrzReader::read`] does
/// not accept the reading (ADR-0026, decision 7 — a covered, check-digited
/// cell is never solved for, even where the arithmetic could do it
/// uniquely).
///
/// `#[non_exhaustive]`: this crate is not published, but every other error
/// type here (`ProviderError`) already follows this shape, and a future
/// refusal reason (a mask that straddles two zones, say) should not be a
/// breaking match for anything that already handles this type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Refused {
    /// A masked cell feeds an ICAO check digit, or is the format's one
    /// structural cell (line 1's first character) — `mrz::apply_occlusion`
    /// refused rather than reporting a value the arithmetic cannot back up.
    /// `line`/`position` are zero-based, exactly as
    /// [`MrzError::OccludedCheckedCell`] reports them.
    CheckedCell { line: usize, position: usize },
    /// `parsed.mrz_lines` does not hold a zone of `parsed.format` at all
    /// (`mrz::MrzError::NotFound`). Reachable only for an [`MrzData`] built
    /// or edited outside a `parse_*`/`find_and_parse` call — every value
    /// [`crate::mrz_reader::MrzReader`] hands to [`apply`] came from one.
    NotFound,
    /// [`mrz::apply_occlusion`] withheld a [`ZoneField`] variant
    /// [`core_field_of`] does not map to a [`CoreField`]. `ZoneField` is
    /// `#[non_exhaustive]`, so a future `mrz` release can add one this
    /// module does not yet know — refusing here, instead of silently
    /// dropping the field from [`Occluded::fields`], is what keeps that a
    /// safe upgrade rather than a value quietly reaching the wire with no
    /// covered-field marker at all.
    UnknownZoneField,
}

/// Apply an [`MrzOcclusion`] observation to a zone [`MrzData`] already
/// parsed, mapping [`mrz`]'s vocabulary onto this schema's
/// [`CoreField`].
///
/// - Builds the [`CellMask`] from `occ.spans`: every column from `first` to
///   `last` inclusive, on `line`, exactly as [`OcclusionSpan`] documents
///   its own fields ([`mrz::MrzError::OccludedCheckedCell`]'s `line` and
///   `position` use the same zero-based counting).
/// - **Empty spans are the identity**: `apply(parsed, &MrzOcclusion::default())`
///   always returns `Ok((parsed.clone(), Vec::new()))`, matching
///   [`mrz::apply_occlusion`]'s own identity rule for [`CellMask::EMPTY`].
/// - `Ok`: the masked [`MrzData`] plus every [`CoreField`] withheld, sorted
///   in whatever order [`mrz::apply_occlusion`] discovered them (the caller
///   sorts and deduplicates for the wire — see
///   `ExtractionV2::occluded`'s own doc comment).
/// - `Err`: see [`Refused`]'s variants.
///
/// [`OcclusionSpan`]: synthpass_core::v2::OcclusionSpan
pub fn apply(parsed: &MrzData, occ: &MrzOcclusion) -> Result<(MrzData, Vec<CoreField>), Refused> {
    let mask = occ.spans.iter().fold(CellMask::EMPTY, |mask, span| {
        (usize::from(span.first)..=usize::from(span.last)).fold(mask, |mask, column| {
            mask.with(usize::from(span.line), column)
        })
    });

    let occluded = mrz::apply_occlusion(parsed, mask).map_err(|err| match err {
        MrzError::OccludedCheckedCell { line, position } => Refused::CheckedCell { line, position },
        // `MrzError` is `#[non_exhaustive]`; `apply_occlusion`'s own doc
        // comment names only these two variants as reachable, but a future
        // `mrz` release adding a third must still fail closed here rather
        // than panic on an unmatched arm.
        _ => Refused::NotFound,
    })?;

    let fields = occluded
        .fields
        .iter()
        .map(|&zf| core_field_of(zf).ok_or(Refused::UnknownZoneField))
        .collect::<Result<Vec<_>, _>>()?;

    Ok((occluded.data, fields))
}

/// Maps every [`ZoneField`] this module knows to its [`CoreField`]. `None`
/// for anything else — `ZoneField` is `#[non_exhaustive]`, and [`apply`]
/// turns a `None` here into [`Refused::UnknownZoneField`] rather than
/// dropping the field silently.
fn core_field_of(field: ZoneField) -> Option<CoreField> {
    match field {
        ZoneField::DocumentCode => Some(CoreField::DocumentType),
        ZoneField::IssuingCountry => Some(CoreField::IssuingCountry),
        ZoneField::Surname => Some(CoreField::Surname),
        ZoneField::GivenNames => Some(CoreField::GivenNames),
        ZoneField::Nationality => Some(CoreField::Nationality),
        ZoneField::Sex => Some(CoreField::Sex),
        ZoneField::OptionalData1 => Some(CoreField::OptionalData1),
        ZoneField::OptionalData2 => Some(CoreField::OptionalData2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synthpass_core::v2::{OcclusionKind, OcclusionSpan};

    /// ICAO 9303 Part 4's own TD3 worked example — the same fixture
    /// `synthpass_core::fusion`'s tests and `mrz`'s own doc-tests use.
    fn specimen() -> MrzData {
        mrz::parse_td3(
            "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
            "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
        )
        .expect("fixture is a valid ICAO 9303 specimen")
    }

    #[test]
    fn empty_spans_are_the_identity() {
        let parsed = specimen();
        let (data, fields) = apply(&parsed, &MrzOcclusion::default()).expect("identity never errs");
        assert_eq!(data, parsed);
        assert!(fields.is_empty());
    }

    #[test]
    fn a_name_line_span_withholds_the_right_core_fields() {
        let parsed = specimen();
        // Column 20 sits inside `given_names` (see the equivalent
        // `mrz::apply_occlusion` doc-test), with the surname's own `<<`
        // terminator still visible.
        let occ = MrzOcclusion {
            spans: vec![OcclusionSpan {
                line: 0,
                first: 20,
                last: 20,
                kind: OcclusionKind::Fill,
            }],
        };
        let (data, fields) = apply(&parsed, &occ).expect("an unverifiable cell never refuses");
        assert_eq!(fields, vec![CoreField::GivenNames]);
        assert_eq!(data.given_names, "");
        assert_eq!(
            data.surname, "ERIKSSON",
            "the visible terminator protects it"
        );
    }

    #[test]
    fn a_span_over_a_check_digit_is_refused() {
        let parsed = specimen();
        // Line 1 (zero-based), column 0: the document-number check digit's
        // first cell.
        let occ = MrzOcclusion {
            spans: vec![OcclusionSpan {
                line: 1,
                first: 0,
                last: 0,
                kind: OcclusionKind::Blur,
            }],
        };
        assert_eq!(
            apply(&parsed, &occ),
            Err(Refused::CheckedCell {
                line: 1,
                position: 0
            })
        );
    }

    #[test]
    fn a_span_over_the_structural_cell_is_refused() {
        let parsed = specimen();
        // Line 0, column 0: the document code's first character — the one
        // structural (non-check-digited) cell `mrz::apply_occlusion` still
        // refuses on.
        let occ = MrzOcclusion {
            spans: vec![OcclusionSpan {
                line: 0,
                first: 0,
                last: 0,
                kind: OcclusionKind::Fill,
            }],
        };
        assert_eq!(
            apply(&parsed, &occ),
            Err(Refused::CheckedCell {
                line: 0,
                position: 0
            })
        );
    }

    #[test]
    fn a_span_spanning_multiple_columns_masks_every_cell_in_it() {
        let parsed = specimen();
        // The whole given-names field plus its leading filler run.
        let occ = MrzOcclusion {
            spans: vec![OcclusionSpan {
                line: 0,
                first: 15,
                last: 25,
                kind: OcclusionKind::Fill,
            }],
        };
        let (data, fields) = apply(&parsed, &occ).expect("still unverifiable cells only");
        assert_eq!(fields, vec![CoreField::GivenNames]);
        assert_eq!(data.given_names, "");
    }

    #[test]
    fn every_core_field_zone_field_maps_to_is_covered() {
        // Exhaustiveness by construction: if `core_field_of` ever drops a
        // mapping, this fails instead of `apply` silently refusing every
        // record that hits it.
        for (zf, expected) in [
            (ZoneField::DocumentCode, CoreField::DocumentType),
            (ZoneField::IssuingCountry, CoreField::IssuingCountry),
            (ZoneField::Surname, CoreField::Surname),
            (ZoneField::GivenNames, CoreField::GivenNames),
            (ZoneField::Nationality, CoreField::Nationality),
            (ZoneField::Sex, CoreField::Sex),
            (ZoneField::OptionalData1, CoreField::OptionalData1),
            (ZoneField::OptionalData2, CoreField::OptionalData2),
        ] {
            assert_eq!(core_field_of(zf), Some(expected), "{zf:?}");
        }
    }
}
