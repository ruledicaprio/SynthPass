//! TEMPORARY — ADR-0018's slot proof on the real corpus, removed by the commit
//! that deletes `MrzData::personal_number`.
//!
//! The round-trip proptests in `crates/mrz` prove, over arbitrary zones, that
//! `personal_number` is exactly the non-empty space-join of `optional_data_1`
//! and `optional_data_2` on TD1, and equals `optional_data_1` on every other
//! format. This walks every tracked OCR fixture — real zones, including the
//! document-number overflow and filler-inside-the-field cases the strategies
//! never generate — and asserts the same relation, so the break that follows
//! removes a field whose value is recoverable from the two that replace it on
//! every document the corpus has.
//!
//! The denominator is pinned on purpose: `samples/ocr_fixtures/` holds 118
//! fixtures, 64 at the top level and the rest under `derived/`. A glob that
//! sees only the top level reports 9 TD1 zones, not 15, and 5 populated
//! optional-data fields, not 9.
//!
//! Zones are parsed the way `template_traits.rs` parses them — by their line
//! count and width, with the exact `parse_*` function — not through
//! `find_and_parse`, whose repair passes may re-read a hand-verified zone
//! into a different shape. The relation under test is a property of one
//! parse's output, so the parse must be the plain one.

use std::fs;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("workspace root")
}

fn json_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("readable fixture directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            json_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "json") {
            out.push(path);
        }
    }
}

fn join_nonempty(a: Option<&str>, b: Option<&str>) -> Option<String> {
    let parts: Vec<&str> = [a, b].into_iter().flatten().collect();
    (!parts.is_empty()).then(|| parts.join(" "))
}

/// Parse a hand-verified zone with the exact function its shape names.
/// `Err` carries the shape so a fixture that does not fit any layout is
/// reported, not skipped.
fn parse_exact(zone: &str) -> Result<mrz::MrzData, String> {
    let lines: Vec<&str> = zone.lines().collect();
    let widths: Vec<usize> = lines.iter().map(|l| l.chars().count()).collect();
    let result = match (lines.len(), widths.first().copied()) {
        (3, Some(30)) => mrz::parse_td1(lines[0], lines[1], lines[2]),
        (2, Some(44)) if lines[0].starts_with('V') => mrz::parse_mrv_a(lines[0], lines[1]),
        (2, Some(44)) => mrz::parse_td3(lines[0], lines[1]),
        (2, Some(36)) if lines[0].starts_with('V') => mrz::parse_mrv_b(lines[0], lines[1]),
        (2, Some(36)) => mrz::parse_td2(lines[0], lines[1]),
        _ => return Err(format!("no layout for shape {widths:?}")),
    };
    result.map_err(|e| format!("shape {widths:?}: {e:?}"))
}

#[test]
fn every_tracked_fixture_keeps_personal_number_recoverable_from_the_two_slots() {
    let mut files = Vec::new();
    json_files(&repo_root().join("samples/ocr_fixtures"), &mut files);
    files.sort();
    assert_eq!(files.len(), 118, "the fixture denominator moved");

    let mut td1 = 0;
    let mut td1_populated = 0;
    let mut slot1_only = 0;
    let mut slot2_only = 0;
    let mut both = 0;
    let mut unparsed = Vec::new();

    for path in &files {
        let text = fs::read_to_string(path).expect("fixture is readable");
        let fixture: serde_json::Value = serde_json::from_str(&text).expect("fixture is JSON");
        let Some(zone) = fixture.get("mrz_line").and_then(|v| v.as_str()) else {
            continue;
        };
        let parsed = match parse_exact(zone) {
            Ok(parsed) => parsed,
            Err(e) => {
                unparsed.push(format!("{}: {e}", path.display()));
                continue;
            }
        };

        let o1 = parsed.optional_data_1.as_deref();
        let o2 = parsed.optional_data_2.as_deref();
        if parsed.format == mrz::Format::Td1 {
            td1 += 1;
            match (o1.is_some(), o2.is_some()) {
                (true, true) => both += 1,
                (true, false) => slot1_only += 1,
                (false, true) => slot2_only += 1,
                (false, false) => {}
            }
            if parsed.personal_number.is_some() {
                td1_populated += 1;
            }
            assert_eq!(
                parsed.personal_number,
                join_nonempty(o1, o2),
                "{}: TD1 personal_number must be the non-empty join of both slots",
                path.display()
            );
        } else {
            assert_eq!(
                o2,
                None,
                "{}: only TD1 has a second optional-data element",
                path.display()
            );
            assert_eq!(
                parsed.personal_number.as_deref(),
                o1,
                "{}: the one optional-data element is the primary slot",
                path.display()
            );
        }
    }

    assert!(
        unparsed.is_empty(),
        "every tracked fixture must parse with the exact function its shape names:\n{}",
        unparsed.join("\n")
    );
    assert_eq!(td1, 15, "TD1 fixture count moved");
    assert_eq!(td1_populated, 9, "populated TD1 optional-data count moved");
    // The nine populated TD1 zones by slot placement — the re-key PR 3c
    // touches exactly these files, derived by the parser rather than by
    // slicing (Belgium's apparent `7027` is the overflow remainder).
    assert_eq!(
        (slot1_only, slot2_only, both),
        (6, 1, 2),
        "slot placement of the populated TD1 zones moved"
    );
}
