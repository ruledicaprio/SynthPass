//! Deterministic field normalizers (M5 §8).
//!
//! Tier-2 (LLM) extractions read the *visual* zone of a document, which is
//! free-form printed text — not the constrained MRZ dialect Tier 1 produces.
//! A parity miss like `"CROATIA"` vs `HRV`, or `"JAAK-KRISTJAN"` vs
//! `JAAK KRISTJAN`, is not the LLM failing to *understand* the document; the
//! model read the field correctly and simply reproduced the visual zone's
//! own formatting. These are normalization failures, fixable deterministically
//! with no model and no new dependency — estimated worth +5–7 accuracy
//! points against the Tier-1/Tier-2 parity suite.
//!
//! Every function here is **pure and idempotent**: same input always gives
//! the same output, and normalizing an already-normalized value is a no-op
//! (`f(f(x)) == f(x)`). None of them invent data — input this module can't
//! confidently resolve passes through unchanged rather than guessing.
//!
//! **Wired into both pipeline paths.** `synthpass-pipeline` calls
//! [`extraction`] on every Tier-2 result, in `process_document` and in
//! `process_document_stream` alike, before anything downstream sees it. This
//! doc previously said the opposite, left over from before that integration
//! landed — and the stale claim cost real accuracy: `parity.rs` was written to
//! match it, skipped normalization entirely, and under-reported Tier-2 field
//! accuracy by roughly 20 percentage points until 2026-09-04. Anything
//! measuring Tier-2 output must normalize it first, or it is measuring a value
//! the product never emits.

/// Normalize an `issuing_country`/`nationality` field: a full country name
/// (any case) resolves to its 3-letter ICAO/ISO 3166-1 code via
/// [`mrz::code_for_name`] — the *same* table [`mrz::country_name`] already
/// uses, so this can never drift from what the rest of the workspace
/// considers a valid code. A string that already looks like a valid code (3
/// ASCII letters, or the legacy single-letter `"D"`) passes through
/// unchanged except for uppercasing. Anything unresolved passes through
/// unchanged, verbatim.
pub fn country_code(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }
    if looks_like_a_code(trimmed) {
        return trimmed.to_uppercase();
    }
    match mrz::code_for_name(trimmed) {
        Some(code) => code.to_string(),
        None => trimmed.to_string(),
    }
}

fn looks_like_a_code(s: &str) -> bool {
    (s.len() == 3 && s.chars().all(|c| c.is_ascii_alphabetic())) || s.eq_ignore_ascii_case("D")
}

/// Normalize `Extraction::issuing_country`. See [`country_code`].
pub fn issuing_country(input: &str) -> String {
    country_code(input)
}

/// Normalize `Extraction::nationality`. See [`country_code`].
pub fn nationality(input: &str) -> String {
    country_code(input)
}

/// Normalize `Extraction::given_names` to MRZ convention: `<` (a leaked MRZ
/// filler, when a Tier-2 read pulls formatting from the machine-readable
/// zone instead of the visual zone) and `-` (a hyphenated given name, e.g.
/// `"JAAK-KRISTJAN"`, which the MRZ encodes as a filler/space rather than a
/// hyphen — ICAO 9303 part 3 §4.6.1) both become a single space; runs of
/// whitespace collapse to one; the result is uppercased to match every other
/// MRZ-sourced name field in this schema.
pub fn given_names(input: &str) -> String {
    let spaced: String = input
        .chars()
        .map(|c| if c == '<' || c == '-' { ' ' } else { c })
        .collect();
    spaced
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_uppercase()
}

/// Plausible calendar-year bounds a normalized date must fall within to be
/// accepted — mirrors `v2::looks_like_a_date`'s own range (see that
/// function's doc comment in `v2.rs`) so this module's judgement of
/// "plausible enough to normalize" agrees with the confidence scorer that
/// already ships in this crate, rather than inventing a second opinion.
const PLAUSIBLE_YEAR_RANGE: std::ops::RangeInclusive<u32> = 1900..=2999;

/// Normalize a date field to the strict ISO `YYYY-MM-DD` form
/// `mrz::dates::parse_iso` requires (4-digit year, zero-padded 2-digit month
/// and day, `-`-separated, exactly 10 characters) — read that parser first;
/// this targets exactly its dialect rather than inventing a new one, since
/// it is what every date-plausibility check in this workspace (`Validity`,
/// `MrzData::validity`) is ultimately built on.
///
/// Recognizes:
/// - already-correct or loosely-padded ISO order (`"2014-7-1"` →
///   `"2014-07-01"`);
/// - day-first `DD.MM.YYYY` / `DD/MM/YYYY` / `DD-MM-YYYY` — the common
///   non-US format a Tier-2 read of a visual zone tends to reproduce;
/// - unseparated `YYYYMMDD` (8 digits, the raw shape once an MRZ `YYMMDD`
///   has been century-expanded);
/// - a **named month** as printed in a document's visual zone, including the
///   bilingual renderings travel documents use (`"14 OCT /OUT 2000"` →
///   `"2000-10-14"`) — see [`month_from_named_tokens`].
///
/// Genuinely ambiguous input (e.g. `MM/DD/YYYY`-shaped, where neither
/// outer component is a 4-digit year) passes through unchanged rather than
/// guessing which of two readings is right.
///
/// # A two-digit year needs to know which field it is
///
/// This function does not know whether it is normalizing a birth date or an
/// expiry date, so it **refuses** a two-digit year outright: `"22 FEB 78"`
/// could be 1978 or 2078 and nothing here can tell. [`date_of_birth`] and
/// [`date_of_expiry`] can, and accept that form; prefer them wherever the
/// field is known. [`extraction`] uses them.
pub fn date(input: &str) -> String {
    date_with_role(input, DateRole::Unknown)
}

/// [`date`], told that this is a **birth** date — so a two-digit year is
/// resolved through [`mrz::expand_date`]'s audited pivot instead of being
/// refused. See [`date`]'s "A two-digit year needs to know which field it is".
pub fn date_of_birth(input: &str) -> String {
    date_with_role(input, DateRole::Birth)
}

/// [`date`], told that this is an **expiry** date — so a two-digit year
/// resolves to `20xx`, which is what [`mrz::expand_date`] does for every
/// non-birth date. See [`date`]'s "A two-digit year needs to know which field
/// it is".
pub fn date_of_expiry(input: &str) -> String {
    date_with_role(input, DateRole::Expiry)
}

/// Which field a date string was read from, which is the only thing that can
/// resolve a two-digit year. `Unknown` is the honest answer for [`date`]'s
/// field-agnostic callers and makes the two-digit form unparseable rather
/// than guessed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DateRole {
    Birth,
    Expiry,
    Unknown,
}

impl DateRole {
    /// Expand a two-digit year, or `None` when the field is unknown.
    ///
    /// Routed through [`mrz::expand_date`] rather than reimplementing the
    /// pivot: that constant is audited, documented as this workspace's own
    /// heuristic rather than an ICAO rule, and CI (`scripts/check-century-pivot.sh`)
    /// fails once it drifts more than two years behind the wall clock. A
    /// second copy of the rule here would eventually disagree with it — the
    /// same reasoning that makes [`country_code`] defer to
    /// [`mrz::code_for_name`].
    fn expand_year(self, yy: u32, month: u32, day: u32) -> Option<u32> {
        let is_birth = match self {
            Self::Birth => true,
            Self::Expiry => false,
            Self::Unknown => return None,
        };
        let expanded = mrz::expand_date(&format!("{yy:02}{month:02}{day:02}"), is_birth);
        expanded.get(..4)?.parse().ok()
    }
}

fn date_with_role(input: &str, role: DateRole) -> String {
    let trimmed = input.trim();
    match parse_date_parts(trimmed, role) {
        Some((y, m, d)) => format!("{y:04}-{m:02}-{d:02}"),
        None => trimmed.to_string(),
    }
}

fn parse_date_parts(s: &str, role: DateRole) -> Option<(u32, u32, u32)> {
    // A named month is unambiguous where a numeric one is not, so it is tried
    // first: "14 OCT /OUT 2000" has no separator layout the numeric branches
    // below could read, and its month needs no day-vs-month judgement call.
    if let Some(parts) = parse_named_month_date(s) {
        return Some(parts);
    }
    // Then the same shape read positionally, which is strictly more permissive
    // and therefore only ever runs on strings the strict form already refused.
    if let Some(parts) = parse_named_month_date_positional(s, role) {
        return Some(parts);
    }
    if s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit()) {
        let (y, m, d) = (
            s[0..4].parse().ok()?,
            s[4..6].parse().ok()?,
            s[6..8].parse().ok()?,
        );
        return valid_date(y, m, d).then_some((y, m, d));
    }
    // Whitespace is a separator alongside `-`, `.` and `/`: a visual zone
    // prints `01 10 1990` as readily as `01.10.1990`, and a Tier-2 read
    // reproduces whichever it saw. Empty components are dropped so a run of
    // separators (`25.12 1982`, `1 / 7 / 2031`) reads as one.
    let parts: Vec<&str> = s
        .split(|c: char| matches!(c, '-' | '.' | '/') || c.is_ascii_whitespace())
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    let [a, b, c] = parts[..] else {
        return None;
    };
    let (y, m, d) = if a.len() == 4 {
        // ISO order: YYYY-MM-DD.
        (a.parse().ok()?, b.parse().ok()?, c.parse().ok()?)
    } else if c.len() == 4 {
        // Day-first order: DD-MM-YYYY.
        (c.parse().ok()?, b.parse().ok()?, a.parse().ok()?)
    } else {
        return None; // ambiguous (neither end names a 4-digit year) — don't guess
    };
    valid_date(y, m, d).then_some((y, m, d))
}

/// Month names as printed on travel documents, in the languages this corpus
/// actually contains. Both the three-letter abbreviation a document prints in
/// its date line and the full word are listed, because a visual zone uses
/// either.
///
/// Entries are matched **whole-token**, never as substrings — French `AOUT`
/// (August) ends in the Portuguese `OUT` (October), and a substring match
/// would silently read every French August as an October.
///
/// Laid out one month per line rather than one entry per line — the grouping
/// is what makes a missing or misfiled language visible at a glance, which is
/// the only review this table can realistically get.
#[rustfmt::skip]
const MONTH_NAMES: &[(&str, u32)] = &[
    ("JAN", 1), ("JANUARY", 1), ("JANVIER", 1), ("JANUAR", 1), ("ENE", 1), ("ENERO", 1),
    ("GEN", 1), ("GENNAIO", 1), ("JANEIRO", 1),
    ("FEB", 2), ("FEBRUARY", 2), ("FEBRUAR", 2), ("FEV", 2), ("FEVRIER", 2), ("FEVEREIRO", 2),
    ("FEBRERO", 2), ("FEBBRAIO", 2),
    ("MAR", 3), ("MARCH", 3), ("MARS", 3), ("MARZ", 3), ("MARZO", 3), ("MARCO", 3),
    ("APR", 4), ("APRIL", 4), ("AVR", 4), ("AVRIL", 4), ("ABR", 4), ("ABRIL", 4), ("APRILE", 4),
    ("MAY", 5), ("MAI", 5), ("MAIO", 5), ("MAYO", 5), ("MAG", 5), ("MAGGIO", 5),
    ("JUN", 6), ("JUNE", 6), ("JUIN", 6), ("JUNI", 6), ("JUNIO", 6), ("JUNHO", 6),
    ("GIU", 6), ("GIUGNO", 6),
    ("JUL", 7), ("JULY", 7), ("JUIL", 7), ("JUILLET", 7), ("JULI", 7), ("JULIO", 7),
    ("JULHO", 7), ("LUG", 7), ("LUGLIO", 7),
    ("AUG", 8), ("AUGUST", 8), ("AOUT", 8), ("AGO", 8), ("AGOSTO", 8),
    ("SEP", 9), ("SEPT", 9), ("SEPTEMBER", 9), ("SEPTEMBRE", 9), ("SETEMBRO", 9),
    ("SEPTIEMBRE", 9), ("SET", 9), ("SETTEMBRE", 9),
    ("OCT", 10), ("OCTOBER", 10), ("OCTOBRE", 10), ("OCTUBRE", 10), ("OKT", 10),
    ("OKTOBER", 10), ("OUT", 10), ("OUTUBRO", 10), ("OTT", 10), ("OTTOBRE", 10),
    ("NOV", 11), ("NOVEMBER", 11), ("NOVEMBRE", 11), ("NOVIEMBRE", 11), ("NOVEMBRO", 11),
    ("DEC", 12), ("DECEMBER", 12), ("DECEMBRE", 12), ("DEZ", 12), ("DEZEMBER", 12),
    ("DEZEMBRO", 12), ("DIC", 12), ("DICIEMBRE", 12), ("DICEMBRE", 12),
];

/// The month named by `s`, or `None` when no token names one **or** two
/// tokens disagree.
///
/// Travel documents routinely print a date bilingually — `"14 OCT /OUT 2000"`,
/// `"27 DEC /DEZ 2032"` — so several tokens naming the *same* month is the
/// normal case and is accepted. Tokens naming *different* months mean the
/// string was misread or is not a single date, and that returns `None` rather
/// than picking one: this module's existing contract is that ambiguous input
/// passes through untouched instead of being guessed at.
fn month_from_named_tokens(s: &str) -> Option<u32> {
    let mut found: Option<u32> = None;
    for token in s.split(|c: char| !c.is_ascii_alphabetic()) {
        if token.is_empty() {
            continue;
        }
        let upper = token.to_ascii_uppercase();
        let Some((_, month)) = MONTH_NAMES.iter().find(|(name, _)| *name == upper) else {
            continue;
        };
        match found {
            Some(existing) if existing != *month => return None,
            _ => found = Some(*month),
        }
    }
    found
}

/// Parse a date whose month is written as a word, e.g. `"14 OCT /OUT 2000"`.
///
/// Requires exactly one 4-digit year token and exactly one distinct 1-2 digit
/// day value; anything else is ambiguous and returns `None`. Note the day is
/// compared by *value*, not by text, so the day printed twice in a bilingual
/// line (`"01 JAN/JAN 01 2000"`) does not read as a conflict.
fn parse_named_month_date(s: &str) -> Option<(u32, u32, u32)> {
    let month = month_from_named_tokens(s)?;
    let mut year: Option<u32> = None;
    let mut day: Option<u32> = None;
    for token in s.split(|c: char| !c.is_ascii_digit()) {
        if token.is_empty() {
            continue;
        }
        let value: u32 = token.parse().ok()?;
        match token.len() {
            4 => match year {
                Some(existing) if existing != value => return None,
                _ => year = Some(value),
            },
            1 | 2 => match day {
                Some(existing) if existing != value => return None,
                _ => day = Some(value),
            },
            // A 3-, 5- or longer digit run is not a day or a year on any date
            // rendering; its presence means this is not the shape we think.
            _ => return None,
        }
    }
    let (y, d) = (year?, day?);
    valid_date(y, month, d).then_some((y, month, d))
}

/// [`parse_named_month_date`] read **positionally** instead of by token
/// width: the digits before the month name are the day, the digits after it
/// are the year.
///
/// This exists for the form the strict parser cannot represent — a two-digit
/// year, `"22 FEB 78"` — where `78` and `22` are indistinguishable by width
/// and the strict parser correctly refuses rather than guessing which is the
/// day. Position resolves it: every rendering in this corpus prints the day
/// before the month and the year after it.
///
/// It runs **only after** the strict parser has declined, so it can never
/// change an existing reading. That ordering is load-bearing: a bilingual line
/// that prints the day twice (`"01 JAN/JAN 01 2000"`) has two leading digit
/// groups and is refused here, but the strict parser reads it correctly by
/// comparing day *values*.
///
/// Requires exactly one digit group on each side of the month, a 1-2 digit
/// day, and a 2- or 4-digit year — so `"JAN 15 1985"` (month first) and
/// `"2000 OCT 14"` (year first) are refused rather than misread, and a stray
/// digit group from OCR noise (`"05 2A/FEB 2022"`) is too.
fn parse_named_month_date_positional(s: &str, role: DateRole) -> Option<(u32, u32, u32)> {
    let month = month_from_named_tokens(s)?;

    // Split into alphabetic and numeric runs, keeping their order — the whole
    // point of this parser is where each group sits relative to the month.
    let mut before: Vec<&str> = Vec::new();
    let mut after: Vec<&str> = Vec::new();
    let mut seen_month = false;
    for token in s.split(|c: char| !c.is_ascii_alphanumeric()) {
        if token.is_empty() {
            continue;
        }
        if token.bytes().all(|b| b.is_ascii_digit()) {
            if seen_month { &mut after } else { &mut before }.push(token);
        } else if is_a_month_name(token) {
            // The *last* month name is the boundary: `"24 CEP/AUG 91"` is one
            // misread month name beside a good one, not two dates, and a
            // non-month word between them (`"TJAN"`, `"JUI"`) is OCR noise
            // that must not be read as a boundary either.
            seen_month = true;
            after.clear();
        }
    }

    let ([day], [year]) = (&before[..], &after[..]) else {
        return None;
    };
    if !(1..=2).contains(&day.len()) {
        return None;
    }
    let d: u32 = day.parse().ok()?;
    let y: u32 = match year.len() {
        4 => year.parse().ok()?,
        2 => role.expand_year(year.parse().ok()?, month, d)?,
        _ => return None,
    };
    valid_date(y, month, d).then_some((y, month, d))
}

/// Whether `token` is one of [`MONTH_NAMES`], matched whole-token and
/// case-insensitively — the same rule [`month_from_named_tokens`] applies,
/// shared so the two cannot disagree about what counts as a month.
fn is_a_month_name(token: &str) -> bool {
    let upper = token.to_ascii_uppercase();
    MONTH_NAMES.iter().any(|(name, _)| *name == upper)
}

fn valid_date(y: u32, m: u32, d: u32) -> bool {
    PLAUSIBLE_YEAR_RANGE.contains(&y) && (1..=12).contains(&m) && (1..=31).contains(&d)
}

/// Normalize `Extraction::sex` to the single-letter ICAO 9303 code
/// (`M`/`F`/`X`): long forms map to their code, anything already a single
/// letter is uppercased and passed through unchanged (so an already-correct
/// `M`/`F` — or any other single letter a document might print — survives
/// untouched, rather than this function silently coercing every unknown
/// value to `X`).
pub fn sex(input: &str) -> String {
    let upper = input.trim().to_uppercase();
    match upper.as_str() {
        "MALE" => "M".to_string(),
        "FEMALE" => "F".to_string(),
        "UNSPECIFIED" | "UNKNOWN" | "OTHER" => "X".to_string(),
        _ => upper,
    }
}

/// Normalize `Extraction::document_type` to the ICAO 9303 single-letter
/// document code (`P` passport, `I` ID card, `V` visa): long forms map to
/// their code; anything else is uppercased and passed through unchanged.
pub fn document_type(input: &str) -> String {
    let upper = input.trim().to_uppercase();
    match upper.as_str() {
        "PASSPORT" => "P".to_string(),
        "IDENTITY CARD" | "ID CARD" | "NATIONAL IDENTITY CARD" | "NATIONAL ID" => "I".to_string(),
        "VISA" => "V".to_string(),
        _ => upper,
    }
}

/// Apply every per-field normalizer above to a full [`crate::Extraction`]
/// record, in place, leaving absent (`None`) fields untouched. This is the
/// natural aggregate over the individually pure/idempotent functions this
/// module already proves — applying each field's normalizer independently
/// means the whole record is idempotent too (`extraction(extraction(x)) ==
/// extraction(x)`), which the test below checks directly rather than trusting
/// the composition.
///
/// `document_number`, `surname`, and `personal_number` have no normalizer
/// here and pass through by omission: `document_number` is already
/// MRZ-alphanumeric by construction on both tiers, and `surname`/
/// `personal_number` are free text with no canonical form for this module to
/// converge on without inventing one.
///
/// **Not called from here on Tier-1 output.** `synthpass-pipeline` is the
/// only caller (`extract_via_inferer`/`extract_via_inferer_stream`, Tier-2
/// only) — see this module's top doc for why a second opinion on an
/// already checksum-proven Tier-1 read must never run through this.
pub fn extraction(e: &mut crate::Extraction) {
    e.issuing_country = e.issuing_country.take().map(|v| issuing_country(&v));
    e.nationality = e.nationality.take().map(|v| nationality(&v));
    e.given_names = e.given_names.take().map(|v| given_names(&v));
    // The field-aware entry points, not `date`: this is the one place that
    // knows which field each string came from, which is what lets a two-digit
    // year ("22 FEB 78") resolve instead of passing through unparsed.
    e.date_of_birth = e.date_of_birth.take().map(|v| date_of_birth(&v));
    e.date_of_expiry = e.date_of_expiry.take().map(|v| date_of_expiry(&v));
    e.sex = e.sex.take().map(|v| sex(&v));
    e.document_type = e.document_type.take().map(|v| document_type(&v));
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── country_code / issuing_country / nationality ──

    #[test]
    fn country_code_maps_full_names_case_insensitively() {
        let cases = [
            ("CROATIA", "HRV"),
            ("Croatia", "HRV"),
            ("croatia", "HRV"),
            ("Slovenia", "SVN"),
            ("United States of America", "USA"),
        ];
        for (input, expected) in cases {
            assert_eq!(country_code(input), expected, "input: {input:?}");
        }
    }

    #[test]
    fn country_code_passes_through_valid_codes_unchanged_except_case() {
        assert_eq!(country_code("HRV"), "HRV");
        assert_eq!(country_code("hrv"), "HRV");
        assert_eq!(country_code("D"), "D");
    }

    #[test]
    fn country_code_passes_through_unrecognized_input() {
        assert_eq!(country_code("Narnia"), "Narnia");
        assert_eq!(country_code(""), "");
    }

    #[test]
    fn country_code_is_idempotent() {
        for input in ["CROATIA", "HRV", "hrv", "Narnia", ""] {
            let once = country_code(input);
            let twice = country_code(&once);
            assert_eq!(once, twice, "not idempotent for input: {input:?}");
        }
    }

    #[test]
    fn issuing_country_and_nationality_are_the_same_normalizer() {
        assert_eq!(issuing_country("Croatia"), nationality("Croatia"));
    }

    // ── given_names ──

    #[test]
    fn given_names_converts_hyphen_to_space_matching_mrz_convention() {
        assert_eq!(given_names("JAAK-KRISTJAN"), "JAAK KRISTJAN");
    }

    #[test]
    fn given_names_converts_mrz_filler_to_space() {
        assert_eq!(given_names("ANNA<MARIA"), "ANNA MARIA");
    }

    #[test]
    fn given_names_collapses_whitespace_and_uppercases() {
        assert_eq!(given_names("  anna   maria  "), "ANNA MARIA");
    }

    #[test]
    fn given_names_is_idempotent() {
        for input in ["JAAK-KRISTJAN", "ANNA<MARIA", "  anna   maria  ", "SINGLE"] {
            let once = given_names(input);
            let twice = given_names(&once);
            assert_eq!(once, twice, "not idempotent for input: {input:?}");
        }
    }

    // ── date ──

    #[test]
    fn date_pads_a_loosely_formatted_iso_date() {
        assert_eq!(date("2014-7-1"), "2014-07-01");
    }

    #[test]
    fn date_passes_through_a_well_formed_iso_date() {
        assert_eq!(date("1974-08-12"), "1974-08-12");
    }

    #[test]
    fn date_converts_day_first_formats() {
        assert_eq!(date("12.08.1974"), "1974-08-12");
        assert_eq!(date("12/08/1974"), "1974-08-12");
        assert_eq!(date("12-08-1974"), "1974-08-12");
    }

    #[test]
    fn date_converts_unseparated_yyyymmdd() {
        assert_eq!(date("19740812"), "1974-08-12");
    }

    #[test]
    fn date_passes_through_ambiguous_and_garbage_input() {
        // Neither end names a 4-digit year: genuinely ambiguous, don't guess.
        assert_eq!(date("01/02/03"), "01/02/03");
        assert_eq!(date("not a date"), "not a date");
        assert_eq!(date(""), "");
    }

    #[test]
    fn date_rejects_out_of_range_components_without_guessing() {
        assert_eq!(date("1974-13-40"), "1974-13-40");
    }

    #[test]
    fn date_is_idempotent() {
        for input in [
            "2014-7-1",
            "12.08.1974",
            "19740812",
            "not a date",
            "",
            "14 OCT /OUT 2000",
            "01 AOUT 1990",
        ] {
            let once = date(input);
            let twice = date(&once);
            assert_eq!(once, twice, "not idempotent for input: {input:?}");
        }
    }

    /// A name listed twice under *different* months would make lookups depend
    /// on table order — the first entry would win and the second would be
    /// dead, silently. `MARZO` was in fact listed twice while this table was
    /// being written (harmlessly, same month, but it is the shape of the real
    /// bug), which is why this test exists rather than a review comment.
    #[test]
    fn month_names_are_unique_and_unambiguous() {
        let mut seen: std::collections::BTreeMap<&str, u32> = std::collections::BTreeMap::new();
        for (name, month) in MONTH_NAMES {
            assert!(
                (1..=12).contains(month),
                "{name:?} maps to month {month}, which is not a month"
            );
            assert!(
                name.chars().all(|c| c.is_ascii_uppercase()),
                "{name:?} must be uppercase ASCII — lookups uppercase the token before comparing"
            );
            if let Some(existing) = seen.insert(name, *month) {
                assert_eq!(
                    existing, *month,
                    "{name:?} is listed under two different months"
                );
                panic!("{name:?} is listed twice");
            }
        }
    }

    #[test]
    fn date_reads_a_named_month_as_printed_in_a_visual_zone() {
        // Every one of these is a real rendering from the parity corpus's
        // baseline run, where the model read the date correctly and the
        // comparison failed purely on format.
        assert_eq!(date("14 OCT /OUT 2000"), "2000-10-14");
        assert_eq!(date("27 DEC /DEZ 2032"), "2032-12-27");
        assert_eq!(date("04 MAY 1991"), "1991-05-04");
        assert_eq!(date("25 MAY 2024"), "2024-05-25");
    }

    #[test]
    fn date_accepts_agreeing_bilingual_month_names_and_rejects_disagreeing_ones() {
        // The same month in two languages is the normal case on a travel
        // document and must be accepted.
        assert_eq!(date("11 MAR/MAR 1985"), "1985-03-11");
        assert_eq!(date("05 MAY/MAI 2001"), "2001-05-05");
        // Two *different* months means the string was misread. Guessing one
        // would be inventing data, so it passes through untouched.
        assert_eq!(date("05 MAY/DEC 2001"), "05 MAY/DEC 2001");
    }

    /// French `AOUT` (August) ends with Portuguese `OUT` (October). A
    /// substring search would silently turn every French August into an
    /// October — a wrong date that still looks perfectly well-formed, which
    /// is the worst failure shape this module can produce.
    #[test]
    fn date_does_not_read_a_month_name_inside_a_longer_word() {
        assert_eq!(date("01 AOUT 1990"), "1990-08-01");
        assert_eq!(date("15 SEPTEMBRE 2010"), "2010-09-15");
    }

    #[test]
    fn date_leaves_an_incomplete_or_implausible_named_month_alone() {
        // No day.
        assert_eq!(date("OCT 2000"), "OCT 2000");
        // No year.
        assert_eq!(date("14 OCT"), "14 OCT");
        // Day out of range — rejected by `valid_date`, not silently clamped.
        assert_eq!(date("45 OCT 2000"), "45 OCT 2000");
        // Two different days: ambiguous, so untouched.
        assert_eq!(date("14 OCT 15 2000"), "14 OCT 15 2000");
        // A digit run that is neither a day nor a year.
        assert_eq!(date("14 OCT 20001"), "14 OCT 20001");
    }

    #[test]
    fn date_named_month_does_not_disturb_the_numeric_forms() {
        // The named-month branch runs first, so these prove it declines
        // cleanly rather than capturing input the numeric branches own.
        assert_eq!(date("2014-7-1"), "2014-07-01");
        assert_eq!(date("12.08.1974"), "1974-08-12");
        assert_eq!(date("19740812"), "1974-08-12");
        assert_eq!(date("07/04/1976"), "1976-04-07");
    }

    /// Every case here is a real Tier-2 answer from the 2026-09-04 parity run
    /// that the pipeline threw away as unparseable while the model had in fact
    /// read the printed date correctly. See
    /// `knowledge/benchmarks/normalize-date-forms-2026-09-04.md`.
    #[test]
    fn date_reads_a_whitespace_separated_numeric_date() {
        assert_eq!(date("01 10 1990"), "1990-10-01");
        assert_eq!(date("30 04 2036"), "2036-04-30");
        assert_eq!(date("29 03 2029"), "2029-03-29");
        // A mixed and a doubled separator both read as one.
        assert_eq!(date("25.12 1982"), "1982-12-25");
        assert_eq!(date("1 / 7 / 2031"), "2031-07-01");
    }

    #[test]
    fn date_still_refuses_a_whitespace_date_that_names_no_four_digit_year() {
        // Whitespace becoming a separator must not turn an ambiguous date into
        // a guessed one: neither end is a 4-digit year, so this stays raw
        // exactly as `07 04 76`'s `-`-separated twin already did.
        assert_eq!(date("07 04 76"), "07 04 76");
        // Nor may a four-component string be salvaged by dropping one.
        assert_eq!(date("22 03 01 2"), "22 03 01 2");
        assert_eq!(date("2014-07-01 00:00:00"), "2014-07-01 00:00:00");
    }

    #[test]
    fn date_of_birth_and_expiry_resolve_a_two_digit_year_by_field() {
        // `mrz::expand_date`'s pivot: a birth year past it is 19xx, an expiry
        // year is always 20xx. Asserted through both entry points on the same
        // string, since the field is the only thing that separates them.
        assert_eq!(date_of_birth("22 FEB 78"), "1978-02-22");
        assert_eq!(date_of_expiry("22 FEB 78"), "2078-02-22");
        assert_eq!(date_of_birth("04 DEC 88"), "1988-12-04");
        assert_eq!(date_of_expiry("12 JAN 25"), "2025-01-12");
        // Bilingual, with the two-digit year — the common visual-zone form.
        assert_eq!(date_of_birth("13 ABR/AVR 90"), "1990-04-13");
    }

    #[test]
    fn date_refuses_a_two_digit_year_when_the_field_is_unknown() {
        // `date` cannot tell 1978 from 2078, and says so by declining rather
        // than by picking one. This is the whole reason the two field-aware
        // entry points exist.
        assert_eq!(date("22 FEB 78"), "22 FEB 78");
        assert_eq!(date("04 DEC 88"), "04 DEC 88");
    }

    #[test]
    fn date_reads_past_ocr_noise_between_the_month_and_the_year() {
        // A misread month name beside a good one, and a stray word after it,
        // are noise — not a second date and not a boundary.
        assert_eq!(date_of_birth("24 CEP/AUG 91"), "1991-08-24");
        assert_eq!(date_of_birth("01 JAN TJAN 85"), "1985-01-01");
        assert_eq!(date_of_expiry("1 JUL/ JUI/JUL 31"), "2031-07-01");
        // A token that is neither a number nor a word (`2A`, a garbled `02`)
        // is noise too, and skipping it leaves an unambiguous day/month/year.
        assert_eq!(date_of_expiry("05 2A/FEB 2022"), "2022-02-05");
    }

    #[test]
    fn date_positional_reading_never_overrides_the_strict_one() {
        // A bilingual line that prints the day on both sides of the month has
        // two leading digit groups; the positional parser refuses it and the
        // strict parser — which runs first and compares day *values* — reads
        // it. If the order were reversed this would return the raw string.
        assert_eq!(date("01 JAN/JAN 01 2000"), "2000-01-01");
        assert_eq!(date("14 OCT /OUT 2000"), "2000-10-14");
    }

    #[test]
    fn date_refuses_a_named_month_it_cannot_place_positionally() {
        // Month first, and year first. Both are shown with a *two-digit* year,
        // because that is the only case the positional parser is reached for:
        // with a 4-digit year the strict parser reads either order correctly
        // by token width, and does so before this parser runs.
        assert_eq!(date_of_birth("JAN 15 85"), "JAN 15 85");
        assert_eq!(date_of_birth("85 OCT 14"), "85 OCT 14");
        // A second *pure-digit* group before the month does make the day
        // ambiguous, and that is refused.
        assert_eq!(date_of_expiry("05 02/FEB 22"), "05 02/FEB 22");
        // A glued month name is not a month name — whole-token matching still
        // holds, which is what keeps French AOUT out of Portuguese OUT.
        assert_eq!(date_of_expiry("05 JULJJUL 2025"), "05 JULJJUL 2025");
        // An implausible year is still rejected after positional reading.
        assert_eq!(date_of_birth("16 MAR/MAR 20049"), "16 MAR/MAR 20049");
    }

    #[test]
    fn date_of_birth_and_expiry_are_idempotent() {
        for raw in ["22 FEB 78", "01 10 1990", "24 CEP/AUG 91", "not a date"] {
            let once = date_of_birth(raw);
            assert_eq!(date_of_birth(&once), once, "date_of_birth on {raw:?}");
            let once = date_of_expiry(raw);
            assert_eq!(date_of_expiry(&once), once, "date_of_expiry on {raw:?}");
        }
    }

    // ── sex ──

    #[test]
    fn sex_maps_long_forms() {
        assert_eq!(sex("MALE"), "M");
        assert_eq!(sex("Female"), "F");
        assert_eq!(sex("unspecified"), "X");
    }

    #[test]
    fn sex_passes_through_single_letters_uppercased() {
        assert_eq!(sex("m"), "M");
        assert_eq!(sex("F"), "F");
    }

    #[test]
    fn sex_is_idempotent() {
        for input in ["MALE", "female", "m", "X", ""] {
            let once = sex(input);
            let twice = sex(&once);
            assert_eq!(once, twice, "not idempotent for input: {input:?}");
        }
    }

    // ── document_type ──

    #[test]
    fn document_type_maps_long_forms() {
        assert_eq!(document_type("PASSPORT"), "P");
        assert_eq!(document_type("Identity Card"), "I");
        assert_eq!(document_type("national id"), "I");
        assert_eq!(document_type("visa"), "V");
    }

    #[test]
    fn document_type_passes_through_short_codes_uppercased() {
        assert_eq!(document_type("p"), "P");
        assert_eq!(document_type("IR"), "IR");
    }

    #[test]
    fn document_type_is_idempotent() {
        for input in ["PASSPORT", "identity card", "p", "IR", ""] {
            let once = document_type(input);
            let twice = document_type(&once);
            assert_eq!(once, twice, "not idempotent for input: {input:?}");
        }
    }

    // ── extraction (full-record aggregate) ──

    /// Build an [`crate::Extraction`] with every normalizable field set to a
    /// realistic, not-yet-normalized value — the kind of parity miss a
    /// Tier-2 visual-zone read actually produces (module-level doc: `"CROATIA"`
    /// vs `HRV`, `"JAAK-KRISTJAN"` vs `JAAK KRISTJAN`) — plus the untouched
    /// fields left deliberately messy, so a test bug that accidentally
    /// normalizes one of them would be caught by the "left alone" assertions
    /// below.
    fn messy_extraction() -> crate::Extraction {
        let mut e = crate::Extraction::default();
        e.document_type = Some("passport".into());
        e.issuing_country = Some("Croatia".into());
        e.document_number = Some("x1234567".into());
        e.surname = Some("specimen".into());
        e.given_names = Some("JAAK-KRISTJAN".into());
        e.nationality = Some("croatia".into());
        e.date_of_birth = Some("12.08.1974".into());
        e.sex = Some("female".into());
        e.date_of_expiry = Some("2014-7-1".into());
        e.personal_number = Some("abc123".into());
        e
    }

    #[test]
    fn extraction_normalizes_every_field_it_owns() {
        let mut e = messy_extraction();
        extraction(&mut e);
        assert_eq!(e.document_type.as_deref(), Some("P"));
        assert_eq!(e.issuing_country.as_deref(), Some("HRV"));
        assert_eq!(e.given_names.as_deref(), Some("JAAK KRISTJAN"));
        assert_eq!(e.nationality.as_deref(), Some("HRV"));
        assert_eq!(e.date_of_birth.as_deref(), Some("1974-08-12"));
        assert_eq!(e.sex.as_deref(), Some("F"));
        assert_eq!(e.date_of_expiry.as_deref(), Some("2014-07-01"));
    }

    #[test]
    fn extraction_leaves_fields_it_does_not_own_untouched() {
        let mut e = messy_extraction();
        extraction(&mut e);
        // No normalizer exists for these — they must survive verbatim.
        assert_eq!(e.document_number.as_deref(), Some("x1234567"));
        assert_eq!(e.surname.as_deref(), Some("specimen"));
        assert_eq!(e.personal_number.as_deref(), Some("abc123"));
    }

    #[test]
    fn extraction_leaves_absent_fields_as_none() {
        let mut e = crate::Extraction::default();
        extraction(&mut e);
        assert_eq!(e.document_type, None);
        assert_eq!(e.issuing_country, None);
        assert_eq!(e.given_names, None);
        assert_eq!(e.nationality, None);
        assert_eq!(e.date_of_birth, None);
        assert_eq!(e.sex, None);
        assert_eq!(e.date_of_expiry, None);
    }

    #[test]
    fn extraction_is_idempotent_over_a_full_record() {
        // Table-driven: a normalized-once record and a from-scratch messy
        // one both converge to the same fixed point on a second pass —
        // `extraction(extraction(x)) == extraction(x)`, proven field by
        // field rather than assumed from the individual functions' own
        // idempotency tests above.
        let cases: Vec<crate::Extraction> = vec![messy_extraction(), crate::Extraction::default()];
        for mut e in cases {
            extraction(&mut e);
            let once = e.clone();
            extraction(&mut e);
            assert_eq!(
                e, once,
                "extraction() is not idempotent for input: {once:?}"
            );
        }
    }
}
