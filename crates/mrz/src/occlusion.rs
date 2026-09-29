//! Applying an image-derived occlusion mask to an already-parsed MRZ zone.
//!
//! [`apply_occlusion`] is the only entry point (GitHub issue #565, ADR-0026).
//! It runs strictly *after* a `parse_*` call or [`crate::find_and_parse`]
//! already produced an [`MrzData`] — never during parsing or repair, so none
//! of `repair.rs`'s repair gates ever see a mask. Re-deriving the cell
//! template with [`strip::for_lines`] is exact at that point, because a
//! parsed zone's cell *k* is line position *k* by construction; a
//! repair-aware coverage map for the *search* itself (#551) is a different,
//! unbuilt feature and stays out of scope here.
//!
//! Two policies are deliberately conservative:
//! - A masked cell that feeds a check digit, or the one structural cell
//!   (line 1's first character), refuses the whole call
//!   ([`MrzError::OccludedCheckedCell`]) rather than reporting a value the
//!   arithmetic cannot back up. It is never rebuilt from check-digit
//!   arithmetic, even where the crate's own damaged-path repair could solve
//!   for it uniquely.
//! - A masked cell inside the name field withholds its whole component
//!   (surname, given names, or both) rather than the visible prefix up to
//!   the mask — see [`apply_occlusion`]'s name-grammar section.

use crate::strip::{self, CellClass, Field, LayoutMode};
use crate::{MrzData, MrzError, Sex};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// The widest line any ICAO 9303 layout this crate parses ever prints —
/// TD3 and MRV-A, at 44 columns. [`CellMask`] uses it as a fixed stride, so
/// every legitimate `(line, column)` pair maps to its own bit regardless of
/// which of the five formats is in play.
const CELL_STRIDE: usize = 44;

/// A bitmask of covered cells within one MRZ zone, for [`apply_occlusion`].
///
/// Cells are keyed by zero-based `line` and `column`, exactly as
/// [`MrzData::mrz_lines`] prints them — column within the *line*, not within
/// the concatenated strip — and counted the same way as
/// [`MrzError::OccludedCheckedCell`]'s `line` and `position`. Internally a
/// cell's bit is `line * 44 + column` (44 is the widest printed line,
/// TD3/MRV-A), which gives every `(line, column)` pair from any of the five
/// formats its own bit without needing to know which format is in play: the
/// tallest zone (TD1: 3 lines of 30) has 90 cells, comfortably inside a
/// `u128`. A coordinate that cannot belong to any real zone — a `column` of
/// 44 or more, or a `line` too far down for its bit to fit — is silently
/// ignored rather than panicking or aliasing onto another cell, since
/// [`with`](Self::with) and [`contains`](Self::contains) never need to reject
/// a caller's input to stay correct: an ignored coordinate simply never
/// matches a real cell.
///
/// ```
/// use mrz::CellMask;
///
/// let mask = CellMask::EMPTY.with(0, 5).with(1, 12);
/// assert!(mask.contains(0, 5));
/// assert!(mask.contains(1, 12));
/// assert!(!mask.contains(0, 6));
/// assert!(!mask.is_empty());
/// assert!(CellMask::EMPTY.is_empty());
///
/// // Column 50 exists on no format: ignored, never read as line 1, column 6.
/// assert!(CellMask::EMPTY.with(0, 50).is_empty());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CellMask(u128);

impl CellMask {
    /// No cell masked — the identity value for [`apply_occlusion`]: applying
    /// it returns the parsed data unchanged.
    ///
    /// ```
    /// use mrz::CellMask;
    /// assert!(CellMask::EMPTY.is_empty());
    /// assert_eq!(CellMask::EMPTY, CellMask::default());
    /// ```
    pub const EMPTY: Self = Self(0);

    /// Mark the cell at zero-based `line`/`column` as covered by an
    /// image-derived occluder. Returns the updated mask; out-of-range
    /// coordinates are silently ignored rather than panicking, since no
    /// real zone can ever place a cell there — see the type documentation.
    ///
    /// ```
    /// use mrz::CellMask;
    ///
    /// let mask = CellMask::EMPTY.with(0, 5);
    /// assert!(mask.contains(0, 5));
    /// ```
    pub fn with(self, line: usize, column: usize) -> Self {
        match Self::bit(line, column) {
            Some(bit) => Self(self.0 | (1u128 << bit)),
            None => self,
        }
    }

    /// Whether `line`/`column` was marked by [`with`](Self::with).
    ///
    /// ```
    /// use mrz::CellMask;
    ///
    /// let mask = CellMask::EMPTY.with(2, 7);
    /// assert!(mask.contains(2, 7));
    /// assert!(!mask.contains(2, 8));
    /// ```
    pub fn contains(self, line: usize, column: usize) -> bool {
        match Self::bit(line, column) {
            Some(bit) => self.0 & (1u128 << bit) != 0,
            None => false,
        }
    }

    /// No cell is masked.
    ///
    /// ```
    /// use mrz::CellMask;
    ///
    /// assert!(CellMask::EMPTY.is_empty());
    /// assert!(!CellMask::EMPTY.with(0, 0).is_empty());
    /// ```
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    /// The bit for `line`/`column`, or `None` for a coordinate no zone has.
    /// The column bound is what keeps the fixed stride honest: without it,
    /// `(0, 50)` would land on `(1, 6)`'s bit.
    fn bit(line: usize, column: usize) -> Option<u32> {
        if column >= CELL_STRIDE {
            return None;
        }
        let bit = line.checked_mul(CELL_STRIDE)?.checked_add(column)?;
        u32::try_from(bit).ok().filter(|&bit| bit < u128::BITS)
    }
}

/// A field [`apply_occlusion`] can report as withheld: exactly the fields no
/// ICAO 9303 check digit covers on any of the five formats (`mrz`'s own
/// "What a passing parse guarantees" table names the same set as
/// *unverifiable*). `#[non_exhaustive]`: a future coverage refinement (#519,
/// #551) could narrow this further without breaking a caller's `match`.
///
/// ```
/// use mrz::ZoneField;
///
/// assert_eq!(ZoneField::Surname, ZoneField::Surname);
/// assert_ne!(ZoneField::Surname, ZoneField::GivenNames);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub enum ZoneField {
    /// The document code's second cell ([`MrzData::document_type`]). The
    /// first cell is structural, not unverifiable, and a mask over it fails
    /// closed instead ([`MrzError::OccludedCheckedCell`]).
    DocumentCode,
    /// [`MrzData::issuing_country`].
    IssuingCountry,
    /// The primary identifier ([`MrzData::surname`]). See
    /// [`apply_occlusion`]'s name-grammar section for when a masked name
    /// cell withholds this component rather than
    /// [`GivenNames`](Self::GivenNames) alone.
    Surname,
    /// The secondary identifier ([`MrzData::given_names`]).
    GivenNames,
    /// [`MrzData::nationality`].
    Nationality,
    /// [`MrzData::sex`].
    Sex,
    /// [`MrzData::optional_data_1`]. No check digit covers this field only
    /// on MRV-A/MRV-B, which print no composite; on TD1, TD2 and TD3 it
    /// feeds the composite check, so a mask there fails closed instead.
    OptionalData1,
    /// [`MrzData::optional_data_2`]. TD1 only; TD1's second optional-data
    /// element always feeds the composite check, so this crate's coverage
    /// map (#519) never actually classifies it unverifiable — the variant
    /// exists for symmetry with [`MrzData::optional_data_2`] and to leave
    /// room for a future format that does not cover it.
    OptionalData2,
}

/// The result of [`apply_occlusion`]: a parsed zone with every masked,
/// unverifiable field withheld.
///
/// `#[non_exhaustive]`: constructed only by [`apply_occlusion`].
///
/// ```
/// use mrz::{apply_occlusion, parse_td3, CellMask};
///
/// let doc = parse_td3(
///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
///     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
/// )
/// .unwrap();
///
/// // Mask one cell inside the given names.
/// let occluded = apply_occlusion(&doc, CellMask::EMPTY.with(0, 20)).unwrap();
/// assert_eq!(occluded.data.given_names, "");
/// assert_eq!(occluded.data.surname, "ERIKSSON"); // the surname's own terminator is visible
/// assert_eq!(occluded.fields, vec![mrz::ZoneField::GivenNames]);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Occluded {
    /// The parsed zone, with every field in [`fields`](Self::fields) blanked
    /// (an empty `String`, or `None` for the two `Option<String>` optional-data
    /// slots, or [`Sex::Unspecified`] for the sex cell) so no covered value
    /// survives. Every other field is exactly as [`apply_occlusion`]'s
    /// `parsed` argument held it.
    pub data: MrzData,
    /// Which fields were withheld, in the order [`apply_occlusion`]
    /// discovered them: fields other than the name in the zone's cell order,
    /// then [`ZoneField::Surname`] and/or [`ZoneField::GivenNames`] last if
    /// the name field was affected. Empty when the mask covered no
    /// unverifiable cell — including an [`CellMask::EMPTY`] mask, which
    /// always returns an empty list.
    pub fields: Vec<ZoneField>,
}

/// Apply an image-derived [`CellMask`] to a zone [`MrzData`] already parsed.
///
/// `mask`'s coordinates are read against `parsed.mrz_lines`, so `mask` must
/// have been built against the same zone `parsed` came from — this function
/// has no way to check that, since [`MrzData`] does not carry cell-level
/// image coordinates itself.
///
/// # Semantics
///
/// - **An empty mask is the identity**: `apply_occlusion(parsed,
///   CellMask::EMPTY)` always returns `parsed.clone()` with an empty
///   `fields` list, and this holds before either of the rules below sees the
///   mask.
/// - Every other masked cell is classified by this crate's crate-private
///   coverage map (#519), re-derived from `parsed.format` and
///   `parsed.mrz_lines`:
///   - a cell that feeds a check digit, or the format's one structural cell
///     (line 1's first character), makes the whole call
///     `Err(MrzError::OccludedCheckedCell)`;
///   - on an overflow layout (a document number that spilled into the
///     optional-data field), any masked cell outside the name field is the
///     same error, regardless of what the coverage map would otherwise say
///     about it — #519 does not model an overflow layout's relocated cells
///     closely enough to trust here;
///   - every other masked cell withholds its [`ZoneField`]: listed once in
///     [`Occluded::fields`], and blanked in [`Occluded::data`].
///
/// # The name grammar
///
/// The name field splits into two components exactly as
/// [`MrzData::surname`]/[`MrzData::given_names`] do: the surname, up to the
/// first `<<` separator, and the given names after it. A component is
/// **complete** — reported normally, not withheld — only when a **visible**
/// terminator (both of its cells unmasked) is confirmed before the
/// component's first masked cell:
///
/// - the surname's terminator is the `<<` separator itself; a masked cell
///   anywhere before a confirmed, visible `<<` withholds the surname —
///   *and* the given names with it, since a surname that might not have
///   ended where it looks to have ended makes the split itself unreliable;
/// - the given names' terminator is a further, visible `<<` marking the
///   start of trailing filler, or the end of the field when the given names
///   run to the last column with no filler at all; a masked cell strictly
///   after that terminator (padding only) never withholds anything.
///
/// This is deliberately stricter than reporting the visible prefix: a
/// masked cell that could either complete the separator or continue a
/// multi-part surname is never guessed at either way (ADR-0026, decision 3).
///
/// # Errors
///
/// - [`MrzError::OccludedCheckedCell`] for the first masked cell the rules
///   above refuse, in the zone's cell order.
/// - [`MrzError::NotFound`] when `parsed.mrz_lines` does not hold a zone of
///   `parsed.format` at all. A value a `parse_*` call returned always does;
///   this is only reachable through an [`MrzData`] edited or deserialized
///   after the fact.
///
/// ```
/// use mrz::{apply_occlusion, parse_td1, CellMask, ZoneField};
///
/// let doc = parse_td1(
///     "I<UTOD231458907<<<<<<<<<<<<<<<",
///     "7408122F1204159UTO<<<<<<<<<<<6",
///     "ERIKSSON<<ANNA<MARIA<<<<<<<<<<",
/// )
/// .unwrap();
///
/// // The `<<` separator itself (line 2, columns 8-9 of the name line) is masked:
/// // neither component is provably complete.
/// let mask = CellMask::EMPTY.with(2, 8).with(2, 9);
/// let occluded = apply_occlusion(&doc, mask).unwrap();
/// assert_eq!(occluded.data.surname, "");
/// assert_eq!(occluded.data.given_names, "");
/// assert_eq!(occluded.fields, vec![ZoneField::Surname, ZoneField::GivenNames]);
///
/// // A check-covered cell fails closed instead of reporting a guess.
/// let err = apply_occlusion(&doc, CellMask::EMPTY.with(1, 0)).unwrap_err();
/// assert_eq!(
///     err,
///     mrz::MrzError::OccludedCheckedCell { line: 1, position: 0 },
/// );
/// ```
pub fn apply_occlusion(parsed: &MrzData, mask: CellMask) -> Result<Occluded, MrzError> {
    if mask.is_empty() {
        return Ok(Occluded {
            data: parsed.clone(),
            fields: Vec::new(),
        });
    }

    let lines: Vec<&str> = parsed.mrz_lines.lines().collect();
    // `MrzData`'s fields are public (and deserializable under `serde`), so a
    // zone that no longer fits its own format is refused, never a panic.
    let Some(template) = strip::for_lines(parsed.format, &lines) else {
        return Err(MrzError::NotFound);
    };

    let mut data = parsed.clone();
    let mut fields: Vec<ZoneField> = Vec::new();

    for cell in &template.cells {
        let (line, column) = (usize::from(cell.line), usize::from(cell.column));
        if !mask.contains(line, column) || cell.field == Field::Name {
            continue;
        }
        let refused = MrzError::OccludedCheckedCell {
            line,
            position: column,
        };
        let outside_name_in_overflow = template.mode != LayoutMode::Ordinary;
        if outside_name_in_overflow || !matches!(cell.class(), CellClass::Unverifiable) {
            return Err(refused);
        }
        // Fail closed: a masked cell with no field to withhold must never be
        // skipped silently, or its covered value would survive in `data`.
        let Some(field) = zone_field(cell.field) else {
            return Err(refused);
        };
        push_once(&mut fields, field);
        blank(&mut data, field);
    }

    withhold_name_if_masked(&mut data, &mut fields, &template, &lines, mask);

    Ok(Occluded { data, fields })
}

/// Maps a coverage-map [`Field`] to the [`ZoneField`] a masked, unverifiable
/// cell in it withholds. `Field::Name` is handled separately by
/// [`withhold_name_if_masked`]'s grammar, and every other `Field` variant
/// classifies `CheckCovered` on every format this crate parses
/// (`cell_class_map_is_total_and_names_the_only_structural_cell` in
/// `strip.rs` pins this), so `apply_occlusion` never reaches this function
/// for them. It still returns `None` for them, and `apply_occlusion` refuses
/// on `None`, so a future map change that made one unverifiable fails closed.
fn zone_field(field: Field) -> Option<ZoneField> {
    match field {
        Field::DocumentCode => Some(ZoneField::DocumentCode),
        Field::Issuer => Some(ZoneField::IssuingCountry),
        Field::Nationality => Some(ZoneField::Nationality),
        Field::Sex => Some(ZoneField::Sex),
        Field::Optional1 => Some(ZoneField::OptionalData1),
        Field::Optional2 => Some(ZoneField::OptionalData2),
        Field::Name
        | Field::DocumentNumber
        | Field::DocumentNumberMarker
        | Field::NumberRemainder
        | Field::DocumentNumberCheck
        | Field::Birth
        | Field::BirthCheck
        | Field::Expiry
        | Field::ExpiryCheck
        | Field::PersonalCheck
        | Field::CompositeCheck => None,
    }
}

fn push_once(fields: &mut Vec<ZoneField>, field: ZoneField) {
    if !fields.contains(&field) {
        fields.push(field);
    }
}

fn blank(data: &mut MrzData, field: ZoneField) {
    match field {
        ZoneField::DocumentCode => data.document_type.clear(),
        ZoneField::IssuingCountry => data.issuing_country.clear(),
        ZoneField::Surname => data.surname.clear(),
        ZoneField::GivenNames => data.given_names.clear(),
        ZoneField::Nationality => data.nationality.clear(),
        ZoneField::Sex => data.sex = Sex::Unspecified,
        ZoneField::OptionalData1 => data.optional_data_1 = None,
        ZoneField::OptionalData2 => data.optional_data_2 = None,
    }
}

/// Find, mask, and grade the zone's single name field against the grammar
/// documented on [`apply_occlusion`]. A no-op when no cell of the name field
/// is masked.
fn withhold_name_if_masked(
    data: &mut MrzData,
    fields: &mut Vec<ZoneField>,
    template: &strip::Template,
    lines: &[&str],
    mask: CellMask,
) {
    let mut name_cells = template
        .cells
        .iter()
        .filter(|cell| cell.field == Field::Name);
    let first = name_cells
        .next()
        .expect("every format this crate parses defines a name field");
    let line = usize::from(first.line);
    let start = usize::from(first.column);
    let end = name_cells
        .next_back()
        .map_or(start + 1, |cell| usize::from(cell.column) + 1);

    let chars: Vec<char> = lines[line][start..end].chars().collect();
    let masked: Vec<bool> = (0..chars.len())
        .map(|i| mask.contains(line, start + i))
        .collect();
    let Some(first_masked) = masked.iter().position(|&m| m) else {
        return;
    };

    let (surname_occluded, given_occluded) = match visible_double_filler(&chars, &masked, 0) {
        Some(separator) if first_masked > separator => {
            let padding_start =
                visible_double_filler(&chars, &masked, separator + 2).unwrap_or(chars.len());
            let given_names_masked = (separator + 2..padding_start).any(|i| masked[i]);
            (false, given_names_masked)
        }
        _ => (true, true),
    };

    if surname_occluded {
        push_once(fields, ZoneField::Surname);
        blank(data, ZoneField::Surname);
    }
    if given_occluded {
        push_once(fields, ZoneField::GivenNames);
        blank(data, ZoneField::GivenNames);
    }
}

/// The lowest index `i >= from` such that `chars[i..i + 2]` reads `<<` and
/// neither cell is masked — a *visible* `<<`, never trusting a masked cell's
/// printed character to be a filler it might not be.
fn visible_double_filler(chars: &[char], masked: &[bool], from: usize) -> Option<usize> {
    if chars.len() < 2 {
        return None;
    }
    (from..=chars.len() - 2)
        .find(|&i| !masked[i] && !masked[i + 1] && chars[i] == '<' && chars[i + 1] == '<')
}
