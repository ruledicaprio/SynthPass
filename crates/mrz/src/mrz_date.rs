//! Typed MRZ date fields: what a six-character date field actually holds
//! (ADR-0019), and its text form (ADR-0020).
//!
//! A date field in the zone is not always a date. Doc 9303 Part 3 §4.8 lets an
//! issuer fill unknown date-of-birth positions with `<`; printed specimens carry six-digit
//! placeholders such as `000000` that name no calendar day; and a recogniser
//! can put any charset character in the cell. [`MrzDate`] gives each of those
//! its own variant, so a caller branches on one value instead of reconciling
//! a string with a second field.
//!
//! The text form is a contract ([ADR-0020]): `Display`, the serde string and
//! the zone agree. A six-digit field renders as the same ISO string
//! [`expand_date_with_pivot`](crate::expand_date_with_pivot) produces, and
//! every other field renders as its six raw characters, so the text of any
//! date the parser has ever produced is unchanged.
//!
//! [ADR-0020]: https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/decisions/ADR-0020-mrz-value-wire-contract.md

use std::fmt;
use std::str::FromStr;

use crate::checksum::char_value;
use crate::dates::{century_for, date_completeness, Date, DateCompleteness};

/// The six characters of an MRZ date field, exactly as printed.
///
/// Construction admits only six characters from the MRZ alphabet (`0-9`,
/// `A-Z`, `<`), so the value is always six ASCII bytes. Held inline rather
/// than as a `String`, which keeps [`MrzDate`] `Copy` and leaves no heap copy
/// behind for the `zeroize` feature to miss.
///
/// ```
/// use mrz::RawDateField;
///
/// let field = RawDateField::try_from("74<<12").unwrap();
/// assert_eq!(field.as_str(), "74<<12");
/// assert!(RawDateField::try_from("74<<1").is_err()); // five characters
/// assert!(RawDateField::try_from("74a812").is_err()); // not the MRZ alphabet
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RawDateField([u8; 6]);

impl RawDateField {
    /// The six characters as printed.
    pub fn as_str(&self) -> &str {
        // Construction admits only MRZ-alphabet characters, which are ASCII,
        // so this never falls back. A zeroized field (all NUL bytes) is still
        // valid UTF-8.
        std::str::from_utf8(&self.0).unwrap_or_default()
    }
}

impl TryFrom<&str> for RawDateField {
    type Error = InvalidRawDateField;

    fn try_from(field: &str) -> Result<Self, Self::Error> {
        let bytes: [u8; 6] = field
            .as_bytes()
            .try_into()
            .map_err(|_| InvalidRawDateField)?;
        if field.chars().all(|c| char_value(c).is_some()) {
            Ok(Self(bytes))
        } else {
            Err(InvalidRawDateField)
        }
    }
}

impl fmt::Display for RawDateField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Rejection from [`RawDateField::try_from`]: the text was not exactly six
/// characters of the MRZ alphabet (`0-9`, `A-Z`, `<`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct InvalidRawDateField;

impl fmt::Display for InvalidRawDateField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an MRZ date field is exactly six characters of 0-9, A-Z and <")
    }
}

impl std::error::Error for InvalidRawDateField {}

/// Which date a field holds, which decides the century of a two-digit year.
///
/// Birth years after the pivot belong to the 1900s. Expiry years are always
/// 20xx, since no valid travel document from the 1900s is still in circulation.
/// The rule is this crate's, not ICAO's: see [`CURRENT_YY`](crate::CURRENT_YY).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DateRole {
    /// The holder's date of birth.
    Birth,
    /// The document's date of expiry.
    Expiry,
}

/// What an MRZ date field holds.
///
/// ```
/// use mrz::{Date, DateRole, MrzDate, RawDateField};
///
/// let read = |field: &str| {
///     MrzDate::from_field(RawDateField::try_from(field).unwrap(), DateRole::Birth, 26)
/// };
///
/// assert_eq!(read("740812"), MrzDate::Calendar(Date::new(1974, 8, 12)));
/// assert_eq!(read("000000"), MrzDate::OutOfCalendar(Date::new(2000, 0, 0)));
/// assert!(matches!(read("74<<12"), MrzDate::PartiallyUnknown(_)));
/// assert_eq!(read("<<<<<<"), MrzDate::Unknown);
/// assert!(matches!(read("74O812"), MrzDate::Malformed(_))); // letter O, a misread
///
/// // The text form is the ISO date for six digits, the raw field otherwise.
/// assert_eq!(read("740812").to_string(), "1974-08-12");
/// assert_eq!(read("74<<12").to_string(), "74<<12");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MrzDate {
    /// Six digits naming a real calendar day when produced by the parser, century
    /// from the parse pivot. The public variant also accepts unchecked `Date` values.
    Calendar(Date),
    /// Six digits that name no calendar day: month `00` or `45`, 29 February
    /// in a non-leap year, the `000000` placeholder some specimens print. The
    /// components are what was read, and
    /// [`Date::is_well_formed`](crate::Date::is_well_formed) is `false`.
    OutOfCalendar(Date),
    /// Digits and `<` fillers. Part 3 §4.8 permits this for birth dates;
    /// accepting it in an expiry field is parser tolerance, not an ICAO rule.
    PartiallyUnknown(RawDateField),
    /// Six `<` fillers. Part 3 §4.8 permits this for a birth date; accepting
    /// it in an expiry field is parser tolerance, not an ICAO rule.
    Unknown,
    /// A character that is neither a digit nor `<`: not a conformant date
    /// field, so a misread rather than an issuer's choice.
    Malformed(RawDateField),
}

impl MrzDate {
    /// Classify a date field, expanding a two-digit year by `role` and
    /// `pivot_yy` exactly as
    /// [`expand_date_with_pivot`](crate::expand_date_with_pivot) does.
    pub fn from_field(field: RawDateField, role: DateRole, pivot_yy: u32) -> MrzDate {
        match date_completeness(field.as_str()) {
            DateCompleteness::Complete => {
                // Six ASCII digits, established by `date_completeness`.
                let pair =
                    |i: usize| u32::from(field.0[i] - b'0') * 10 + u32::from(field.0[i + 1] - b'0');
                let yy = pair(0);
                let century = century_for(yy, role == DateRole::Birth, pivot_yy);
                // `yy` is below 100, so the cast is lossless.
                let date = Date::new(century + yy as i32, pair(2), pair(4));
                if date.is_well_formed() {
                    MrzDate::Calendar(date)
                } else {
                    MrzDate::OutOfCalendar(date)
                }
            }
            DateCompleteness::PartiallyUnknown => MrzDate::PartiallyUnknown(field),
            DateCompleteness::Unknown => MrzDate::Unknown,
            DateCompleteness::Malformed => MrzDate::Malformed(field),
        }
    }

    /// The same classification [`date_completeness`](crate::date_completeness)
    /// gives the raw field: both six-digit variants are `Complete`.
    pub fn completeness(self) -> DateCompleteness {
        match self {
            MrzDate::Calendar(_) | MrzDate::OutOfCalendar(_) => DateCompleteness::Complete,
            MrzDate::PartiallyUnknown(_) => DateCompleteness::PartiallyUnknown,
            MrzDate::Unknown => DateCompleteness::Unknown,
            MrzDate::Malformed(_) => DateCompleteness::Malformed,
        }
    }

    /// The date, when the field names a real calendar day, and `None` for
    /// every other variant. The accessor to reach for when a caller needs a
    /// usable date and nothing else.
    pub fn calendar(self) -> Option<Date> {
        match self {
            MrzDate::Calendar(date) => Some(date),
            _ => None,
        }
    }
}

/// The ADR-0020 text form: `YYYY-MM-DD` for the two six-digit variants, the
/// six raw characters otherwise. Round-trips through [`FromStr`] for every
/// value [`MrzDate::from_field`] or [`FromStr`] produces.
impl fmt::Display for MrzDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MrzDate::Calendar(d) | MrzDate::OutOfCalendar(d) => {
                write!(f, "{:04}-{:02}-{:02}", d.year, d.month, d.day)
            }
            MrzDate::PartiallyUnknown(raw) | MrzDate::Malformed(raw) => f.write_str(raw.as_str()),
            MrzDate::Unknown => f.write_str("<<<<<<"),
        }
    }
}

/// Rejection from parsing an [`MrzDate`]'s text form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct ParseMrzDateError;

impl fmt::Display for ParseMrzDateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(
            "an MRZ date is YYYY-MM-DD, or six characters of 0-9, A-Z and < that are not all digits",
        )
    }
}

impl std::error::Error for ParseMrzDateError {}

/// The inverse of `Display` for values produced by [`MrzDate::from_field`]
/// or this parser. A caller can construct variants that normalize on parsing
/// (for example `Malformed("<<<<<<")` becomes `Unknown`). A ten-character
/// `YYYY-MM-DD` of digits is
/// `Calendar` or `OutOfCalendar` by well-formedness, deliberately without the
/// calendar check that would reject the second. Six fillers are `Unknown`;
/// six charset characters mixing digits and fillers are `PartiallyUnknown`;
/// any other six charset characters are `Malformed`. Six bare digits are
/// rejected: no century can be recovered without a pivot, although caller-built variants can display them.
impl FromStr for MrzDate {
    type Err = ParseMrzDateError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let b = text.as_bytes();
        let iso_shaped = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && [0, 1, 2, 3, 5, 6, 8, 9]
                .iter()
                .all(|&i| b[i].is_ascii_digit());
        if iso_shaped {
            let number = |range: std::ops::Range<usize>| {
                text[range].parse::<u32>().map_err(|_| ParseMrzDateError)
            };
            // Four digits, so at most 9999: the cast is lossless.
            let date = Date::new(number(0..4)? as i32, number(5..7)?, number(8..10)?);
            return Ok(if date.is_well_formed() {
                MrzDate::Calendar(date)
            } else {
                MrzDate::OutOfCalendar(date)
            });
        }
        let field = RawDateField::try_from(text).map_err(|_| ParseMrzDateError)?;
        match date_completeness(field.as_str()) {
            DateCompleteness::Complete => Err(ParseMrzDateError),
            DateCompleteness::PartiallyUnknown => Ok(MrzDate::PartiallyUnknown(field)),
            DateCompleteness::Unknown => Ok(MrzDate::Unknown),
            DateCompleteness::Malformed => Ok(MrzDate::Malformed(field)),
        }
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl serde::Serialize for MrzDate {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<'de> serde::Deserialize<'de> for MrzDate {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        text.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "zeroize")]
#[cfg_attr(docsrs, doc(cfg(feature = "zeroize")))]
impl zeroize::Zeroize for RawDateField {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

/// Wipes the payload in place. The variant is left as it was: a discriminant
/// says which kind of date was read, not the date itself.
#[cfg(feature = "zeroize")]
#[cfg_attr(docsrs, doc(cfg(feature = "zeroize")))]
impl zeroize::Zeroize for MrzDate {
    fn zeroize(&mut self) {
        match self {
            MrzDate::Calendar(date) | MrzDate::OutOfCalendar(date) => date.zeroize(),
            MrzDate::PartiallyUnknown(raw) | MrzDate::Malformed(raw) => raw.zeroize(),
            MrzDate::Unknown => {}
        }
    }
}
