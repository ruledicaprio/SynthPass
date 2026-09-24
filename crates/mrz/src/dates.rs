//! Date handling: `YYMMDD` → ISO expansion, and non-cryptographic
//! plausibility checks (expiry vs. "today", DOB-before-expiry, well-formedness).
//!
//! A valid MRZ composite check digit makes the candidate *checksum-consistent*
//! with the printed zone — it says nothing about whether the document is in
//! date or whether the dates are internally consistent. Those judgements live here and take an
//! explicit reference date so the crate stays deterministic and clock-free
//! (the caller supplies "today").

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Expand `YYMMDD` to ISO `YYYY-MM-DD`.
///
/// Century heuristic: birth dates after the current two-digit year roll back
/// to 19xx; expiry dates are always 20xx (no valid travel document from the
/// 1900s remains in circulation).
///
/// This heuristic is this crate's own invention, not ICAO's: Part 3 §4.8
/// "Representation of Dates"
/// (`knowledge/docs9303/Doc_9303_Part3_Specs_Common_to_all_MRTDs.md:541-547`)
/// defines only the six-digit `YYMMDD` structure and the unknown-date filler
/// rule ("If all or part of the date of birth is unknown, the relevant
/// character positions shall be completed with filler characters"). It says
/// nothing about which century a two-digit year belongs to — checked, not
/// assumed: the corpus has no century-inference rule anywhere, for any
/// document type.
///
/// ```
/// use mrz::expand_date;
///
/// assert_eq!(expand_date("740812", true), "1974-08-12"); // birth date
/// assert_eq!(expand_date("120415", false), "2012-04-15"); // expiry: always 20xx
/// ```
pub fn expand_date(yymmdd: &str, is_birth: bool) -> String {
    // Two-digit year pivot for the 19xx/20xx decision on birth dates. Kept as a
    // single constant for auditability; callers wanting an explicit pivot can
    // use [`expand_date_with_pivot`].
    expand_date_with_pivot(yymmdd, is_birth, CURRENT_YY)
}

/// The default two-digit-year pivot (2026). Birth years greater than this map
/// to the 1900s.
///
/// Deliberately a hardcoded constant, not a clock read: this crate stays
/// deterministic and clock-free (its date judgements take an explicit "today"
/// instead — see [`MrzData::validity`](crate::MrzData::validity)), so the
/// constant must be bumped by hand as time passes rather than drift silently.
/// A stale pivot degrades gracefully in general but genuinely misreads at the
/// boundary — with this constant stuck at 26, a person born in 2027
/// (`yy == 27`) would be dated to 1927 once such people exist. Bumping it is a
/// patch release. To pin the pivot independently of the crate version, pass
/// [`ParseOptions`](crate::ParseOptions) to a `*_with` parser.
///
/// `scripts/check-century-pivot.sh` runs in CI on every push and fails once
/// this constant is more than 2 years behind the wall clock, so the review
/// cadence is enforced rather than left to memory.
///
/// ```
/// use mrz::{expand_date, CURRENT_YY};
///
/// // A birth year at the pivot stays in this century; one past it rolls back.
/// let at_pivot = format!("{CURRENT_YY:02}0101");
/// let past_pivot = format!("{:02}0101", CURRENT_YY + 1);
/// assert!(expand_date(&at_pivot, true).starts_with("20"));
/// assert!(expand_date(&past_pivot, true).starts_with("19"));
/// ```
pub const CURRENT_YY: u32 = 26;

/// Like [`expand_date`] but with a caller-supplied two-digit-year pivot.
///
/// ```
/// use mrz::expand_date_with_pivot;
///
/// // yy = 74. With the default pivot (26) a birth year of 74 is last century;
/// // raise the pivot past 74 and the same digits read as 2074.
/// assert_eq!(expand_date_with_pivot("740812", true, 26), "1974-08-12");
/// assert_eq!(expand_date_with_pivot("740812", true, 80), "2074-08-12");
/// // Expiry dates are always 20xx regardless of the pivot.
/// assert_eq!(expand_date_with_pivot("740812", false, 80), "2074-08-12");
/// ```
pub fn expand_date_with_pivot(yymmdd: &str, is_birth: bool, pivot_yy: u32) -> String {
    if yymmdd.len() != 6 || !yymmdd.chars().all(|c| c.is_ascii_digit()) {
        return yymmdd.to_string(); // leave unparseable input untouched
    }
    let yy: u32 = yymmdd[0..2].parse().unwrap();
    let century = century_for(yy, is_birth, pivot_yy) / 100;
    format!(
        "{century}{}-{}-{}",
        &yymmdd[0..2],
        &yymmdd[2..4],
        &yymmdd[4..6]
    )
}

/// The century (`1900` or `2000`) a two-digit MRZ year belongs to: birth
/// years after the pivot roll back to the 1900s, everything else is 20xx.
///
/// The one place the rule lives, shared by [`expand_date_with_pivot`] and
/// [`MrzDate::from_field`](crate::MrzDate::from_field) so the string and the
/// typed value can never disagree about a century.
pub(crate) fn century_for(yy: u32, is_birth: bool, pivot_yy: u32) -> i32 {
    if is_birth && yy > pivot_yy {
        1900
    } else {
        2000
    }
}

/// Gregorian leap-year rule: divisible by 4, except century years, which must
/// also be divisible by 400 (so 2000 is a leap year but 1900 is not).
///
/// The rule [`Date::is_well_formed`] applies to February.
///
/// ```
/// assert!(mrz::is_leap_year(2024));
/// assert!(mrz::is_leap_year(2000)); // a century year divisible by 400
/// assert!(!mrz::is_leap_year(1900)); // a century year that is not
/// assert!(!mrz::is_leap_year(2023));
/// ```
pub fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// How much of a six-character MRZ date field is actually known.
///
/// Doc 9303 Part 3 §4.8 (`:547`) lets an issuer fill unknown date-of-birth
/// positions with `<`. Because a filler counts as zero for check-digit
/// purposes (`:563`), an all-filler date of birth with check digit `0` is a
/// *valid* field, not a corrupt read — this type is what lets a caller tell
/// the two apart. See [`date_completeness`] to classify a raw field, and
/// [`MrzDate::completeness`](crate::MrzDate::completeness) for the same
/// answer about a parsed date.
///
/// ```
/// use mrz::{format_td3, parse_td3, DateCompleteness, MrzDate, Td3Fields};
///
/// // An issuer that does not know the holder's date of birth prints fillers.
/// let zone = format_td3(&Td3Fields {
///     issuing_country: "UTO".into(),
///     document_number: "L898902C3".into(),
///     nationality: "UTO".into(),
///     date_of_birth: mrz::MrzDate::Unknown,
///     date_of_expiry: mrz::MrzDate::Calendar(mrz::Date::new(2030, 12, 31)),
///     ..Default::default()
/// });
/// let (l1, l2) = zone.split_once('\n').unwrap();
/// let doc = parse_td3(l1, l2).unwrap();
///
/// assert!(doc.valid()); // every check digit verifies ...
/// assert_eq!(doc.date_of_birth, MrzDate::Unknown); // ... honestly
/// assert_eq!(doc.date_of_birth.completeness(), DateCompleteness::Unknown);
/// assert_eq!(doc.date_of_birth.to_string(), "<<<<<<"); // left raw, never invented
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DateCompleteness {
    /// All six characters are ASCII digits: a complete, ordinary date.
    #[default]
    Complete,
    /// A mix of ASCII digits and `<` fillers: part of the date is known.
    PartiallyUnknown,
    /// All six characters are `<` fillers: the date is entirely unknown, per
    /// Doc 9303 Part 3 §4.8 — not a bad read.
    Unknown,
    /// Not six characters, or contains a character that is neither an ASCII
    /// digit nor `<`: not a conformant MRZ date field at all.
    Malformed,
}

/// Classify a raw six-character MRZ `YYMMDD` date field.
///
/// [`expand_date`] / [`expand_date_with_pivot`] leave any input that is not
/// entirely ASCII digits untouched rather than expanding it to an ISO date,
/// so their output alone cannot say whether a non-ISO string means
/// "conformantly unknown" or "OCR garbage". This classification of the *raw*
/// field can. A parsed [`MrzData`](crate::MrzData) already carries it: each
/// date is an [`MrzDate`](crate::MrzDate), and
/// [`MrzDate::completeness`](crate::MrzDate::completeness) returns what this
/// function returns for the field it was read from.
///
/// Rules, in order:
/// - length != 6 → [`DateCompleteness::Malformed`]
/// - all six ASCII digits → [`DateCompleteness::Complete`]
/// - all six `<` → [`DateCompleteness::Unknown`]
/// - only ASCII digits and `<`, at least one of each → [`DateCompleteness::PartiallyUnknown`]
/// - anything else → [`DateCompleteness::Malformed`]
///
/// Part 3 §4.8 lets an issuer complete an unknown date with filler characters,
/// and §4.9 gives a filler the value zero. An entirely unknown date of birth
/// carrying check digit `0` is therefore a **valid** field, not a corrupt read:
///
/// ```
/// use mrz::{check_digit, date_completeness, DateCompleteness};
///
/// assert_eq!(check_digit("<<<<<<").unwrap(), 0);
///
/// assert_eq!(date_completeness("740812"), DateCompleteness::Complete);
/// assert_eq!(date_completeness("7408<<"), DateCompleteness::PartiallyUnknown);
/// assert_eq!(date_completeness("<<<<<<"), DateCompleteness::Unknown);
/// assert_eq!(date_completeness("7X0812"), DateCompleteness::Malformed);
/// ```
pub fn date_completeness(yymmdd: &str) -> DateCompleteness {
    if yymmdd.len() != 6 {
        return DateCompleteness::Malformed;
    }
    let digits = yymmdd.chars().filter(|c| c.is_ascii_digit()).count();
    let fillers = yymmdd.chars().filter(|&c| c == '<').count();
    if digits + fillers != yymmdd.len() {
        return DateCompleteness::Malformed;
    }
    match (digits, fillers) {
        (6, 0) => DateCompleteness::Complete,
        (0, 6) => DateCompleteness::Unknown,
        _ => DateCompleteness::PartiallyUnknown,
    }
}

/// Number of days in `month` of `year`; `0` for an out-of-range `month`
/// (`0` or `> 12`) so callers get an always-false range rather than a panic.
fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if is_leap_year(year) {
                29
            } else {
                28
            }
        }
        _ => 0,
    }
}

/// A simple proleptic-Gregorian calendar date. Used as the "today" reference
/// for [`crate::MrzData::validity`] and to measure days-until-expiry.
///
/// The crate never reads the system clock, so a caller turns its own clock
/// into a `Date` once, at the edge, and passes it in:
///
/// ```
/// use mrz::Date;
/// use std::time::{SystemTime, UNIX_EPOCH};
///
/// let secs = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
/// let today = Date::from_epoch_days((secs / 86_400) as i64);
/// assert!(today.is_well_formed());
///
/// assert_eq!(Date::from_epoch_days(0), Date::new(1970, 1, 1));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct Date {
    /// Full (not two-digit) calendar year, e.g. `2026`.
    pub year: i32,
    /// Month, 1-12. Not validated at construction — see [`is_well_formed`](Date::is_well_formed).
    pub month: u32,
    /// Day of month, 1-31. Not validated at construction — see [`is_well_formed`](Date::is_well_formed).
    pub day: u32,
}

impl Date {
    /// Build a date from its components. Does not validate — an out-of-range
    /// `month`/`day` constructs successfully; call [`is_well_formed`](Date::is_well_formed)
    /// to check.
    ///
    /// ```
    /// use mrz::Date;
    ///
    /// let d = Date::new(2012, 4, 15);
    /// assert!(d.is_well_formed());
    /// assert!(!Date::new(2023, 2, 30).is_well_formed()); // no such day
    /// ```
    pub fn new(year: i32, month: u32, day: u32) -> Self {
        Self { year, month, day }
    }

    /// Month and day fall within the true calendar range for `year` — Feb 30,
    /// Feb 29 in a non-leap year, and April 31 are all rejected rather than
    /// silently accepted the way a generous 1..=31 day check would.
    ///
    /// ```
    /// use mrz::Date;
    ///
    /// assert!(Date::new(2024, 2, 29).is_well_formed()); // leap day
    /// assert!(!Date::new(2023, 2, 29).is_well_formed()); // not a leap year
    /// assert!(!Date::new(2023, 4, 31).is_well_formed()); // April has 30 days
    /// assert!(!Date::new(2023, 13, 1).is_well_formed()); // no thirteenth month
    /// ```
    pub fn is_well_formed(self) -> bool {
        (1..=12).contains(&self.month)
            && (1..=days_in_month(self.year, self.month)).contains(&self.day)
    }

    /// Days since the Unix epoch (1970-01-01), proleptic Gregorian.
    /// Howard Hinnant's `days_from_civil` — pure integer math, no_std-friendly.
    ///
    /// Subtracting two of these is how [`MrzData::validity`](crate::MrzData::validity)
    /// measures days until expiry.
    ///
    /// ```
    /// use mrz::Date;
    ///
    /// assert_eq!(Date::new(1970, 1, 2).to_epoch_days(), 1);
    ///
    /// // 2012 is a leap year: Jan 31 + Feb 29 + Mar 31 + 14 days = 105.
    /// let span = Date::new(2012, 4, 15).to_epoch_days() - Date::new(2012, 1, 1).to_epoch_days();
    /// assert_eq!(span, 105);
    /// ```
    pub fn to_epoch_days(self) -> i64 {
        let y = if self.month <= 2 {
            self.year - 1
        } else {
            self.year
        } as i64;
        let m = self.month as i64;
        let d = self.day as i64;
        let era = (if y >= 0 { y } else { y - 399 }) / 400;
        let yoe = y - era * 400; // [0, 399]
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1; // [0, 365]
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy; // [0, 146096]
        era * 146097 + doe - 719468
    }

    /// Inverse of [`to_epoch_days`]: build a date from a Unix day number.
    /// Howard Hinnant's `civil_from_days`. Lets a caller turn a system clock
    /// (days since epoch) into a [`Date`] to use as "today".
    ///
    /// ```
    /// use mrz::Date;
    ///
    /// assert_eq!(Date::from_epoch_days(0), Date::new(1970, 1, 1));
    /// assert_eq!(Date::from_epoch_days(-1), Date::new(1969, 12, 31));
    ///
    /// // An exact round trip, leap days included.
    /// let leap_day = Date::new(2024, 2, 29);
    /// assert_eq!(Date::from_epoch_days(leap_day.to_epoch_days()), leap_day);
    /// ```
    ///
    /// [`to_epoch_days`]: Date::to_epoch_days
    pub fn from_epoch_days(days: i64) -> Date {
        let z = days + 719468;
        let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
        let doe = z - era * 146097; // [0, 146096]
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
        let mp = (5 * doy + 2) / 153; // [0, 11]
        let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
        let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
        Date {
            year: (y + i64::from(m <= 2)) as i32,
            month: m as u32,
            day: d as u32,
        }
    }
}

/// Wipes the three components to zero. A date of birth is PII, and once it is
/// held as a `Date` inside [`MrzDate`](crate::MrzDate) rather than as a
/// `String`, this is what keeps [`MrzData`](crate::MrzData)'s drop-time wipe
/// covering it.
#[cfg(feature = "zeroize")]
impl zeroize::Zeroize for Date {
    fn zeroize(&mut self) {
        self.year.zeroize();
        self.month.zeroize();
        self.day.zeroize();
    }
}

/// Date-plausibility summary for an MRZ, relative to a reference "today".
/// Distinct from the check digits: a checksum-valid MRZ can still be expired
/// or carry impossible dates. Obtain one from
/// [`MrzData::validity`](crate::MrzData::validity).
///
/// ```
/// use mrz::Date;
///
/// // The ICAO specimen expires 2012-04-15.
/// let doc = mrz::parse_td3(
///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
///     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
/// )
/// .unwrap();
///
/// let report = doc.validity(Date::new(2012, 4, 5));
/// assert!(report.dates_well_formed);
/// assert!(report.dob_before_expiry);
/// assert!(report.in_date);
/// assert_eq!(report.days_until_expiry, Some(10));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[non_exhaustive]
pub struct DateValidity {
    /// Both `date_of_birth` and `date_of_expiry` are [`MrzDate::Calendar`](crate::MrzDate::Calendar): real calendar dates.
    pub dates_well_formed: bool,
    /// Date of expiry is on or after the reference "today".
    pub in_date: bool,
    /// Date of birth is strictly before the date of expiry.
    pub dob_before_expiry: bool,
    /// Whole days until expiry (negative if already expired), when the expiry
    /// date is well-formed.
    pub days_until_expiry: Option<i64>,
}

impl DateValidity {
    /// Dates are well-formed, internally consistent, and the document is in date.
    ///
    /// ```
    /// use mrz::Date;
    ///
    /// let doc = mrz::parse_td3(
    ///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    ///     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
    /// )
    /// .unwrap();
    ///
    /// assert!(doc.validity(Date::new(2012, 4, 15)).all_ok()); // the last valid day
    /// let expired = doc.validity(Date::new(2012, 4, 16));
    /// assert!(!expired.all_ok());
    /// assert_eq!(expired.days_until_expiry, Some(-1));
    /// ```
    pub fn all_ok(&self) -> bool {
        self.dates_well_formed && self.in_date && self.dob_before_expiry
    }
}

impl crate::MrzData {
    /// Non-cryptographic date plausibility relative to `today`.
    ///
    /// A valid MRZ composite makes the *read* checksum-consistent with the
    /// printed zone; it does not establish that the document is in date, or
    /// that its dates are consistent — that separate judgement is computed
    /// here from the typed date fields. Only an
    /// [`MrzDate::Calendar`](crate::MrzDate::Calendar) date counts: an
    /// out-of-calendar, unknown or malformed field makes
    /// [`dates_well_formed`](DateValidity::dates_well_formed) false.
    ///
    /// `today` is an explicit reference date rather than a reading of the system
    /// clock, so the answer is deterministic and the crate stays clock-free:
    /// the same `MrzData` and the same `today` always agree, in a test, in a
    /// replay, or on a machine whose clock is wrong.
    ///
    /// ```
    /// use mrz::{parse_td3, Date};
    ///
    /// // The ICAO specimen (Utopia / Anna Maria Eriksson): expires 2012-04-15.
    /// let doc = parse_td3(
    ///     "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<",
    ///     "L898902C36UTO7408122F1204159ZE184226B<<<<<10",
    /// )
    /// .unwrap();
    /// assert!(doc.valid()); // every printed check digit agrees ...
    ///
    /// let report = doc.validity(Date::new(2010, 1, 1));
    /// assert!(report.in_date); // ... and, as of 2010, still in date
    /// assert!(report.dob_before_expiry);
    ///
    /// // The same zone, judged against a later day: still valid(), expired.
    /// assert!(!doc.validity(Date::new(2020, 1, 1)).in_date);
    /// ```
    pub fn validity(&self, today: Date) -> DateValidity {
        let dob = self.date_of_birth.calendar();
        let exp = self.date_of_expiry.calendar();
        let (in_date, days_until_expiry) = match exp {
            Some(e) => {
                let days = e.to_epoch_days() - today.to_epoch_days();
                (days >= 0, Some(days))
            }
            None => (false, None),
        };
        let dob_before_expiry = matches!(
            (dob, exp),
            (Some(b), Some(e)) if b.to_epoch_days() < e.to_epoch_days()
        );
        DateValidity {
            dates_well_formed: dob.is_some() && exp.is_some(),
            in_date,
            dob_before_expiry,
            days_until_expiry,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_td3;
    use proptest::prelude::*;

    // ICAO 9303 specimen identity (Utopia / Anna Maria Eriksson): DOB
    // 1974-08-12, expiry 2012-04-15. See `src/lib.rs`'s test module for this
    // specimen's provenance (Part 4's own copy is a figure, not extracted
    // text; corroborated via Part 6's literal TD2 specimen).
    const TD3_L1: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<";
    const TD3_L2: &str = "L898902C36UTO7408122F1204159ZE184226B<<<<<10";

    #[test]
    fn date_completeness_classifies_all_five_cases() {
        // Complete: six ASCII digits.
        assert_eq!(date_completeness("740812"), DateCompleteness::Complete);
        // Unknown: six fillers.
        assert_eq!(date_completeness("<<<<<<"), DateCompleteness::Unknown);
        // Partially unknown: a mix of digits and fillers.
        assert_eq!(
            date_completeness("74<<12"),
            DateCompleteness::PartiallyUnknown
        );
        // Malformed: wrong length.
        assert_eq!(date_completeness(""), DateCompleteness::Malformed);
        assert_eq!(date_completeness("74081"), DateCompleteness::Malformed); // 5 chars
        assert_eq!(date_completeness("7408122"), DateCompleteness::Malformed); // 7 chars
                                                                               // Malformed: contains a character that is neither a digit nor `<`.
        assert_eq!(date_completeness("7X0812"), DateCompleteness::Malformed);
    }

    #[test]
    fn date_century_pivot() {
        assert_eq!(expand_date("740812", true), "1974-08-12");
        assert_eq!(expand_date("150101", true), "2015-01-01");
        assert_eq!(expand_date("301231", false), "2030-12-31");
    }

    #[test]
    fn century_pivot_boundary() {
        // Birth: yy == CURRENT_YY (26) stays in the 2000s; yy == CURRENT_YY+1
        // (27) rolls back to the 1900s; yy == 99 is always 1900s.
        assert_eq!(expand_date("260101", true), "2026-01-01");
        assert_eq!(expand_date("270101", true), "1927-01-01");
        assert_eq!(expand_date("990101", true), "1999-01-01");
        // Expiry is always 20xx, regardless of yy.
        assert_eq!(expand_date("270101", false), "2027-01-01");
    }

    #[test]
    fn leap_year_rule() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
    }

    #[test]
    fn is_well_formed_rejects_impossible_calendar_dates() {
        assert!(Date::new(2024, 2, 29).is_well_formed());
        assert!(!Date::new(2023, 2, 29).is_well_formed());
        assert!(!Date::new(2023, 2, 30).is_well_formed());
        assert!(!Date::new(2023, 4, 31).is_well_formed());
        assert!(!Date::new(2023, 6, 31).is_well_formed());
        assert!(!Date::new(2023, 11, 31).is_well_formed());
        assert!(Date::new(2023, 12, 31).is_well_formed());
        assert!(Date::new(2023, 1, 31).is_well_formed());
        assert!(!Date::new(2023, 0, 10).is_well_formed());
        assert!(!Date::new(2023, 13, 1).is_well_formed());
        assert!(!Date::new(2023, 1, 0).is_well_formed());
    }

    proptest! {
        /// A date `is_well_formed()` iff it round-trips through epoch days
        /// unchanged: a genuinely valid calendar date maps to itself, while
        /// an impossible one (Feb 30, Apr 31, ...) normalizes to a different
        /// date under `to_epoch_days`/`from_epoch_days`'s civil-calendar math.
        #[test]
        fn is_well_formed_matches_epoch_day_round_trip(
            year in 1900i32..=2100,
            month in 1u32..=13,
            day in 0u32..=32,
        ) {
            let d = Date::new(year, month, day);
            prop_assert_eq!(
                d.is_well_formed(),
                Date::from_epoch_days(d.to_epoch_days()) == d
            );
        }
    }

    #[test]
    fn epoch_days_reference_points() {
        assert_eq!(Date::new(1970, 1, 1).to_epoch_days(), 0);
        assert_eq!(Date::new(1969, 12, 31).to_epoch_days(), -1);
        assert_eq!(Date::new(2000, 1, 1).to_epoch_days(), 10957);
    }

    #[test]
    fn epoch_days_roundtrip() {
        for &(y, m, d) in &[(1970, 1, 1), (1974, 8, 12), (2012, 4, 15), (2026, 7, 17)] {
            let date = Date::new(y, m, d);
            assert_eq!(Date::from_epoch_days(date.to_epoch_days()), date);
        }
    }

    #[test]
    fn validity_tracks_expiry_and_consistency() {
        let d = parse_td3(TD3_L1, TD3_L2).unwrap();

        let before = d.validity(Date::new(2011, 1, 1));
        assert!(before.dates_well_formed);
        assert!(before.dob_before_expiry);
        assert!(before.in_date);
        assert!(before.all_ok());

        let after = d.validity(Date::new(2020, 1, 1));
        assert!(!after.in_date);
        assert!(after.days_until_expiry.unwrap() < 0);
        assert!(!after.all_ok());
    }

    #[test]
    fn days_until_expiry_is_exact() {
        // 2012-04-14 → expiry 2012-04-15 is exactly one day.
        let d = parse_td3(TD3_L1, TD3_L2).unwrap();
        let v = d.validity(Date::new(2012, 4, 14));
        assert_eq!(v.days_until_expiry, Some(1));
        assert!(v.in_date);
    }
}
