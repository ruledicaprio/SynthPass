//! `MrzData`'s JSON, pinned byte-for-byte over every ground-truth zone.
//!
//! ADR-0020 makes `mrz`'s text form a contract that must survive to 1.0, and
//! `mrz-wasm` hands exactly this JSON to the browser demo. A change to it has to
//! be a reviewed diff, not a side effect. This test serialises the parse of
//! every `samples/ocr_fixtures/**/*.json` zone and compares the result with
//! `tests/fixtures/mrz_wire_golden.jsonl`.
//!
//! It was blessed from `String`-typed `MrzData`, *before* ADR-0019 swapped the
//! dates and sex for `MrzDate` and `Sex`. So when that swap lands, this test is
//! the detector: the diff must show exactly the declared wire changes (the
//! removed `date_of_birth_completeness` key, and sex cells that are not `M`/`F`
//! serialising as their zone character instead of `"X"`) and nothing else. An
//! empty diff at that point would mean the test never saw the new code.
//!
//! Each zone is parsed with the parser its shape names, not `find_and_parse`,
//! which re-flows at least one TD1 zone into a TD2 read. A zone that does not
//! parse is recorded with its error, so a parser change that starts or stops
//! rejecting a fixture shows up here too.
//!
//! To re-bless after a reviewed change:
//! `MRZ_WIRE_GOLDEN_BLESS=1 cargo test -p synthpass-bench --test mrz_wire_golden`.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

fn golden_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mrz_wire_golden.jsonl")
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

/// Parse a zone with the parser its shape names: three 30-character lines are
/// TD1; two 44-character lines are TD3 when the document code starts with `P`,
/// else MRV-A; two 36-character lines are MRV-B when it starts with `V`, else
/// TD2.
fn parse_by_shape(zone: &str) -> Result<mrz::MrzData, String> {
    let lines: Vec<&str> = zone.lines().collect();
    let parsed = match lines.as_slice() {
        [l1, l2, l3] if l1.len() == 30 => mrz::parse_td1(l1, l2, l3),
        [l1, l2] if l1.len() == 44 && l1.starts_with('P') => mrz::parse_td3(l1, l2),
        [l1, l2] if l1.len() == 44 => mrz::parse_mrv_a(l1, l2),
        [l1, l2] if l1.len() == 36 && l1.starts_with('V') => mrz::parse_mrv_b(l1, l2),
        [l1, l2] if l1.len() == 36 => mrz::parse_td2(l1, l2),
        _ => return Err("zone shape is not TD1, TD2, TD3, MRV-A or MRV-B".to_string()),
    };
    parsed.map_err(|e| format!("{e:?}"))
}

fn render() -> (String, usize) {
    let root = workspace_root();
    let fixtures = root.join("samples/ocr_fixtures");
    let mut paths = Vec::new();
    collect_json(&fixtures, &mut paths);
    paths.sort();

    let mut out = String::new();
    for path in &paths {
        let raw = fs::read_to_string(path).expect("readable fixture");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("fixture is JSON");
        let zone = value["mrz_line"]
            .as_str()
            .expect("fixture has a string mrz_line");
        let name = path
            .strip_prefix(&fixtures)
            .expect("fixture under the fixtures directory")
            .to_string_lossy()
            .replace('\\', "/");
        let line = match parse_by_shape(zone) {
            Ok(data) => serde_json::json!({ "fixture": name, "mrz": data }),
            Err(error) => serde_json::json!({ "fixture": name, "error": error }),
        };
        out.push_str(&serde_json::to_string(&line).expect("serialisable"));
        out.push('\n');
    }
    (out, paths.len())
}

#[test]
fn mrz_json_matches_the_golden_for_every_fixture() {
    let (rendered, count) = render();
    // The denominator trap: a top-level-only walk sees 64.
    assert_eq!(count, 118, "every fixture, derived/ included");

    let golden = golden_path();
    if std::env::var_os("MRZ_WIRE_GOLDEN_BLESS").is_some() {
        fs::write(&golden, &rendered).expect("golden is writable");
        return;
    }
    let expected = fs::read_to_string(&golden)
        .expect("golden missing: bless it with MRZ_WIRE_GOLDEN_BLESS=1 after reviewing the output");
    if rendered == expected {
        return;
    }
    let changed: Vec<String> = rendered
        .lines()
        .zip(expected.lines())
        .filter(|(got, want)| got != want)
        .map(|(got, want)| format!("  - {want}\n  + {got}"))
        .collect();
    panic!(
        "MrzData JSON differs from tests/fixtures/mrz_wire_golden.jsonl on {} of {} lines \
         (line counts {} vs {}):\n{}",
        changed.len(),
        count,
        rendered.lines().count(),
        expected.lines().count(),
        changed.join("\n"),
    );
}
