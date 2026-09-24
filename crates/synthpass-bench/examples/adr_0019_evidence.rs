//! The evidence behind ADR-0019's decision, as a runnable check rather than a
//! one-off count.
//!
//! ADR-0019 replaces `MrzData`'s `String` dates and sex with typed values. Its
//! argument is that the strings hide real distinctions: a date field can hold
//! a calendar day, six digits that are not a calendar day, an issuer's
//! "partly unknown", an issuer's "unknown", or a misread; and the pre-0.8 `clean_sex`
//! collapses every character other than `M`/`F` into `"X"`, ICAO's word for
//! *unspecified*. An argument like that is only as good as the claim that
//! those cases occur. This counts them over every tracked ground-truth zone
//! and asserts the counts, so the numbers the ADR quotes are reproducible and
//! a corpus change that moves them fails loudly instead of silently
//! outdating the record.
//!
//! # What it reads
//!
//! The raw `mrz_line` of every `samples/ocr_fixtures/**/*.json` — 118 files,
//! the reviewed top level *and* `derived/`. A top-level-only glob sees 64, the
//! denominator trap this corpus has already paid for once.
//!
//! The date and sex cells sit at fixed positions, so they are sliced by the
//! zone's shape (line 2 `[0..6]`/`[7]`/`[8..14]` on a three-line TD1, line 2
//! `[13..19]`/`[20]`/`[21..27]` on every two-line format). No parser runs:
//! the question is what the zone *holds*, before any interpretation, and
//! `mrz::find_and_parse` would re-flow at least one TD1 zone into a TD2 read.
//!
//! These are lower bounds for OCR output. Ground truth records the printed
//! zone; a recogniser can only add malformed reads, never remove the printed
//! ones.
//!
//! # Read the kinds with the zone's own checksum verdict
//!
//! A slice is only a field if the zone is aligned. Several specimens print a
//! deliberately non-conformant zone (`mrz_checksums_valid: false`); in
//! Türkiye's 2024 passport specimen the document-number check digit is `<`
//! and every later field sits one column off, so its "date of expiry" slice
//! `48<123` is cut across two fields and is not an issuer's partly-unknown
//! date. Each notable line therefore carries the fixture's checksum verdict,
//! and the counts are asserted separately for checksum-valid zones.
//!
//! Run: `cargo run -p synthpass-bench --example adr_0019_evidence`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Out-of-calendar date fields (both roles pooled) in checksum-valid zones.
const VALID_OUT_OF_CALENDAR: usize = 3;
/// Non-conformant sex cells in checksum-valid zones.
const VALID_NON_CONFORMANT_SEX: usize = 2;

/// The five kinds of a six-character MRZ date field that ADR-0019's `MrzDate`
/// distinguishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum DateKind {
    /// Six digits naming a real calendar day.
    Calendar,
    /// Six digits that do not name a real day (`000000`, `110229`).
    OutOfCalendar,
    /// Digits and `<` fillers: the issuer left part of the date unknown.
    PartiallyUnknown,
    /// Six `<` fillers: the issuer left the whole date unknown.
    Unknown,
    /// A character that is neither a digit nor `<`.
    Malformed,
}

fn date_kind(field: &str, is_birth: bool) -> DateKind {
    match mrz::date_completeness(field) {
        mrz::DateCompleteness::Complete => {
            // Same expansion the parser applies, so "calendar" means exactly
            // what `MrzData::validity().dates_well_formed` would say.
            let iso = mrz::expand_date_with_pivot(field, is_birth, mrz::CURRENT_YY);
            let component = |range: std::ops::Range<usize>| iso[range].parse().unwrap_or(0);
            let date = mrz::Date::new(component(0..4) as i32, component(5..7), component(8..10));
            if date.is_well_formed() {
                DateKind::Calendar
            } else {
                DateKind::OutOfCalendar
            }
        }
        mrz::DateCompleteness::PartiallyUnknown => DateKind::PartiallyUnknown,
        mrz::DateCompleteness::Unknown => DateKind::Unknown,
        _ => DateKind::Malformed,
    }
}

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .join("samples/ocr_fixtures")
}

fn collect_json(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable fixtures directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_json(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
            out.push(path);
        }
    }
}

fn main() {
    let root = fixtures_dir();
    let mut paths = Vec::new();
    collect_json(&root, &mut paths);
    paths.sort();

    let mut birth: BTreeMap<DateKind, usize> = BTreeMap::new();
    let mut expiry: BTreeMap<DateKind, usize> = BTreeMap::new();
    let mut sex: BTreeMap<char, usize> = BTreeMap::new();
    let mut notable = Vec::new();
    // Both dates of every checksum-valid zone, pooled.
    let mut valid_kinds: BTreeMap<DateKind, usize> = BTreeMap::new();
    let mut valid_non_conformant_sex = 0;

    for path in &paths {
        let raw = fs::read_to_string(path).expect("readable fixture");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("fixture is JSON");
        let zone = value["mrz_line"]
            .as_str()
            .expect("fixture has a string mrz_line");
        let checksum_valid = value["mrz_checksums_valid"]
            .as_bool()
            .expect("fixture has a bool mrz_checksums_valid");
        let lines: Vec<&str> = zone.lines().collect();
        let (dob, sex_cell, doe) = match lines.as_slice() {
            [_, l2, _] if l2.len() == 30 => (&l2[0..6], l2.as_bytes()[7] as char, &l2[8..14]),
            [_, l2] if l2.len() == 44 || l2.len() == 36 => {
                (&l2[13..19], l2.as_bytes()[20] as char, &l2[21..27])
            }
            _ => panic!("{}: not a TD1, TD2/MRV-B or TD3/MRV-A zone", path.display()),
        };

        let name = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .display()
            .to_string();
        let zone_state = if checksum_valid {
            "checksum-valid zone"
        } else {
            "checksum-INVALID zone"
        };
        let (kb, ke) = (date_kind(dob, true), date_kind(doe, false));
        if kb != DateKind::Calendar {
            notable.push(format!(
                "date_of_birth  {dob}  {kb:?}  {name}  ({zone_state})"
            ));
        }
        if ke != DateKind::Calendar {
            notable.push(format!(
                "date_of_expiry {doe}  {ke:?}  {name}  ({zone_state})"
            ));
        }
        if !matches!(sex_cell, 'M' | 'F' | '<') {
            notable.push(format!(
                "sex            {sex_cell}       non-conformant  {name}  ({zone_state})"
            ));
        }
        if checksum_valid {
            *valid_kinds.entry(kb).or_default() += 1;
            *valid_kinds.entry(ke).or_default() += 1;
            if !matches!(sex_cell, 'M' | 'F' | '<') {
                valid_non_conformant_sex += 1;
            }
        }
        *birth.entry(kb).or_default() += 1;
        *expiry.entry(ke).or_default() += 1;
        *sex.entry(sex_cell).or_default() += 1;
    }

    println!(
        "{} fixture zones under samples/ocr_fixtures/**\n",
        paths.len()
    );
    println!("date_of_birth:  {birth:?}");
    println!("date_of_expiry: {expiry:?}");
    println!("sex cells:      {sex:?}");
    println!("checksum-valid zones, both dates pooled: {valid_kinds:?}");
    println!("checksum-valid zones, non-conformant sex cells: {valid_non_conformant_sex}\n");
    for line in &notable {
        println!("  {line}");
    }

    // The ADR quotes these. A corpus change that moves one must update the ADR
    // in the same commit, which is the point of asserting them.
    assert_eq!(
        paths.len(),
        118,
        "denominator: every fixture, derived/ included"
    );
    let count = |m: &BTreeMap<DateKind, usize>, k| m.get(&k).copied().unwrap_or(0);
    assert_eq!(count(&birth, DateKind::Calendar), 114);
    assert_eq!(count(&birth, DateKind::OutOfCalendar), 3);
    assert_eq!(count(&birth, DateKind::PartiallyUnknown), 0);
    assert_eq!(count(&birth, DateKind::Unknown), 0);
    assert_eq!(count(&birth, DateKind::Malformed), 1);
    assert_eq!(count(&expiry, DateKind::Calendar), 114);
    assert_eq!(count(&expiry, DateKind::OutOfCalendar), 3);
    assert_eq!(count(&expiry, DateKind::PartiallyUnknown), 1);
    assert_eq!(count(&expiry, DateKind::Unknown), 0);
    assert_eq!(count(&expiry, DateKind::Malformed), 0);
    let cells = |c| sex.get(&c).copied().unwrap_or(0);
    assert_eq!((cells('M'), cells('F'), cells('<')), (58, 56, 0));
    let non_conformant: usize = sex
        .iter()
        .filter(|(c, _)| !matches!(c, 'M' | 'F' | '<'))
        .map(|(_, n)| n)
        .sum();
    assert_eq!(
        non_conformant, 4,
        "sex cells the pre-0.8 clean_sex turns into \"X\""
    );
    // In checksum-valid zones, the only non-calendar dates are six-digit
    // specimen placeholders; the partially-unknown and malformed slices above
    // both come from one misaligned, checksum-invalid zone. No issuer in this
    // corpus prints a filler date: those variants exist because ICAO permits
    // them (Part 3 §4.8) and OCR produces malformed reads, not because this
    // corpus shows them. Sex is different: non-conformant cells occur in
    // checksum-valid zones too, since no check digit covers sex.
    assert_eq!(count(&valid_kinds, DateKind::PartiallyUnknown), 0);
    assert_eq!(count(&valid_kinds, DateKind::Unknown), 0);
    assert_eq!(count(&valid_kinds, DateKind::Malformed), 0);
    assert_eq!(
        count(&valid_kinds, DateKind::OutOfCalendar),
        VALID_OUT_OF_CALENDAR
    );
    assert_eq!(valid_non_conformant_sex, VALID_NON_CONFORMANT_SEX);
    println!("\nall ADR-0019 evidence counts reproduce");
}
