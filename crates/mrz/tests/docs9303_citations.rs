//! Every line citation of `knowledge/docs9303/` in this crate still lands on the text it cites.
//!
//! The crate cites ICAO Doc 9303 by its Markdown transcription's line numbers, in two forms:
//! doc and test comments (`Doc_9303_Part4_Specs_for_MRPs_and_TD3_MRTDs.md:482-606`) and the
//! `source:` labels on the ICAO vectors in `icao_vectors.rs` (`"9303 pt4 §4.2.3.1(a):499"`).
//! A docs edit that adds or removes lines above a cited line moves the text out from under the
//! citation without any error: 45 citations had drifted this way before they were re-pointed
//! (#418 restructured Parts 4 and 5, then #526 lengthened Part 6). This test turns that drift
//! into a failure that names the citation and the text it expected.
//!
//! [`CITED`] pins a prefix of every cited line's text. The test checks that:
//! 1. every citation in the crate is in [`CITED`], so a new citation needs a row;
//! 2. every row of [`CITED`] is still cited, so a removed citation drops its row;
//! 3. every cited file exists and every cited line starts with its pinned text.
//!
//! When it fails because the Markdown moved, re-point the citation to wherever the pinned text
//! now is and update the row's line number; `git log -L` shows where a line went. `CHANGELOG.md`
//! is history and is not scanned. The published crate does not ship `knowledge/`, so outside
//! this repository there is nothing to check and the test passes.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// `(Part, line, prefix of that line's text)` for every Doc 9303 line the crate cites, sorted.
/// The prefix is the line with leading whitespace removed, cut to at most 56 characters.
const CITED: &[(u32, usize, &str)] = &[
    (
        3,
        509,
        r#"Punctuation characters are not allowed in the MRZ. Where"#,
    ),
    (3, 511, r#"**Apostrophe:**"#),
    (3, 515, r#"MRZ: DARTAGNAN"#),
    (3, 523, r#"**Comma:**"#),
    (3, 527, r#"MRZ: ERIKSSON<<ANNA<MARIA"#),
    (3, 531, r#"Example VIZ: ANNA, MARIA"#),
    (3, 532, r#"MRZ: ANNA<MARIA"#),
    (
        3,
        535,
        r#"All other punctuation characters shall be omitted from t"#,
    ),
    (3, 541, r#"### 4.8 Representation of Dates"#),
    (
        3,
        547,
        r#"If all or part of the date of birth is unknown, the rele"#,
    ),
    (
        3,
        703,
        r#"### A. Transliteration of Multinational Latin-based Char"#,
    ),
    (3, 707, r#"| 00C0 | À | A grave | A |"#),
    (3, 807, r#"| 1E9E | ẞ | double s (Germany) | SS |"#),
    (3, 809, r#"### B. Transliteration of Cyrillic Characters"#),
    (3, 863, r#"| 04BA | Һ | C |"#),
    (
        3,
        1241,
        r#"### Example 3 — Composite check digit calculation for TD"#,
    ),
    (3, 1383, r#"HA672242<6YTO5802254M9601086<<<<<<<8."#),
    (
        3,
        1572,
        r#"the name in a European national script: **Térèsa CAÑON**"#,
    ),
    (
        3,
        1574,
        r#"and the transliteration into the MRZ: CANXXON<<TERESA"#,
    ),
    (
        4,
        426,
        r#"| | | Truncation of the name | When the primary and seco"#,
    ),
    (
        4,
        434,
        r#"| 1 to 9 | 05 | Passport number | As given by the issuin"#,
    ),
    (4, 482, r#"#### 4.2.3 Truncation of names in the MRZ"#),
    (4, 498, r#"VIZ: ERIKSSON, ANNA MARIA"#),
    (
        4,
        499,
        r#"MRZ: PPUTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<"#,
    ),
    (
        4,
        507,
        r#"MRZ: PPUTOHENG<<DEBORAH<MING<LO<<<<<<<<<<<<<<<<<<"#,
    ),
    (
        4,
        515,
        r#"MRZ: PPUTOSMITH<JONES<<SUSIE<MARGARET<<<<<<<<<<<<"#,
    ),
    (
        4,
        523,
        r#"MRZ: PPUTOOCONNOR<<ENYA<SIOBHAN<<<<<<<<<<<<<<<<<<"#,
    ),
    (
        4,
        531,
        r#"MRZ: PPUTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<<<<<<<<<"#,
    ),
    (
        4,
        535,
        r#"MRZ: PPUTOAL<BASRI<<HUDA<MUHAMMAD<JAWAD<<<<<<<<<<"#,
    ),
    (
        4,
        539,
        r#"MRZ: PPUTOVILARCHAO<FERNANDEZ<<JOSE<RAMON<<<<<<<<"#,
    ),
    (
        4,
        547,
        r#"MRZ: PPUTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<<<<<<<<<"#,
    ),
    (
        4,
        551,
        r#"MRZ: PPUTOSATRIYA<SUDARPA<<<<<<<<<<<<<<<<<<<<<<<<"#,
    ),
    (4, 552, r#"```"#),
    (
        4,
        561,
        r#"MRZ: PPUTONILAVADHANANANDA<<CHAYAPA<DEJTHAMRONG<K"#,
    ),
    (
        4,
        569,
        r#"MRZ: PPUTONILAVADHANANANDA<<ARNPOL<PETCH<CHARONGU"#,
    ),
    (4, 582, r#"**b) One or more components truncated:**"#),
    (
        4,
        587,
        r#"MRZ: PPUTOBENNELONG<WOOLOOM<WARRAND<WARNAM<<DINGO"#,
    ),
    (4, 588, r#"```"#),
    (
        4,
        595,
        r#"MRZ: PPUTOBENNEL<WOOLOO<WARRAN<WARNAM<<DINGO<POTO"#,
    ),
    (
        4,
        603,
        r#"MRZ: PPUTOPAPANDROPOULOUS<<JONATHON<WARREN<TREVOR"#,
    ),
    (4, 604, r#"```"#),
    (
        4,
        606,
        r#"> *Note.— Even though there is an alphabetic character i"#,
    ),
    (
        5,
        373,
        r#"j) The number of characters in the VIZ may be variable;"#,
    ),
    (
        5,
        432,
        r#"MRZ (lower line): PAPANDROPOULOUS<<JONATHON<ALEC"#,
    ),
    (
        5,
        442,
        r#"MRZ (lower line): VAN<DER<MUELLEN<<MARTIN<<<<<<<"#,
    ),
    (
        5,
        446,
        r#"MRZ (lower line): AL<BASRI<<HUDA<MUHAMMAD<JAWAD<"#,
    ),
    (
        5,
        458,
        r#"MRZ (lower line): ARKFREITH<<<<<<<<<<<<<<<<<<<<<"#,
    ),
    (
        5,
        462,
        r#"MRZ (lower line): SATRIYA<SUDARPA<<<<<<<<<<<<<<<"#,
    ),
    (
        5,
        483,
        r#"| Composite check digit | 6 – 30 (upper line), 1 – 7, 9"#,
    ),
    (
        5,
        484,
        r#"| | *Note.— Positions 1 – 5 (upper line), positions 8, 1"#,
    ),
    (
        6,
        346,
        r#"j) The number of characters in the VIZ may be variable;"#,
    ),
    (
        6,
        405,
        r#"MRZ (upper line): I<UTOPAPANDROPOULOUS<<JONATHOON<ALEC"#,
    ),
    (
        6,
        415,
        r#"MRZ (upper line): I<UTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<"#,
    ),
    (
        6,
        419,
        r#"MRZ (upper line): I<UTOAL<BASRI<<HUDA<MUHAMMAD<JAWAD<<"#,
    ),
    (
        6,
        423,
        r#"MRZ (upper line): I<UTOVILARCHAO<FERNANDEZ<<JOSE<RAMON"#,
    ),
    (
        6,
        431,
        r#"MRZ (upper line): I<UTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<"#,
    ),
    (
        6,
        435,
        r#"MRZ (upper line): I<UTOSATRIYA<SUDARPA<<<<<<<<<<<<<<<<"#,
    ),
    (6, 493, r#"> I<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<"#),
    (6, 494, r#"> D231458907UTO7408122F1204159<<<<<<<6"#),
    (
        7,
        332,
        r#"- MRZ (upper line): V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<"#,
    ),
    (
        7,
        338,
        r#"- MRZ (upper line): V<UTOHENG<<DEBORAH<MING<LO<<<<<<<<<<"#,
    ),
    (
        7,
        344,
        r#"- MRZ (upper line): V<UTOSMITH<JONES<<SUSIE<MARGARET<<<<"#,
    ),
    (
        7,
        350,
        r#"- MRZ (upper line): V<UTOOCONNOR<<ENYA<SIOBHAN<<<<<<<<<<"#,
    ),
    (
        7,
        356,
        r#"- MRZ (upper line): V<UTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<"#,
    ),
    (
        7,
        362,
        r#"- MRZ (upper line): V<UTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<"#,
    ),
    (
        7,
        370,
        r#"- MRZ (upper line): V<UTONILAVADHANANANDA<<CHAYAPA<DEJTH"#,
    ),
    (
        7,
        376,
        r#"- MRZ (upper line): V<UTONILAVADHANANANDA<<ARNPOL<PETCH<"#,
    ),
    (
        7,
        390,
        r#"- MRZ (upper line): V<UTOBENNELONG<WOOLOOM<WARRAND<WARNA"#,
    ),
    (
        7,
        402,
        r#"- MRZ (upper line): V<UTOPAPANDROPOULOUS<<JONATHON<WARRE"#,
    ),
    (
        7,
        690,
        r#"- MRZ (upper line): V<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<"#,
    ),
    (
        7,
        696,
        r#"- MRZ (upper line): V<UTOHENG<<DEBORAH<MING<LO<<<<<<<<<<"#,
    ),
    (
        7,
        702,
        r#"- MRZ (upper line): V<UTOSMITH<JONES<<SUSIE<MARGARET<<<<"#,
    ),
    (
        7,
        708,
        r#"- MRZ (upper line): V<UTOOCONNOR<<ENYA<SIOBHAN<<<<<<<<<<"#,
    ),
    (
        7,
        714,
        r#"- MRZ (upper line): V<UTOVAN<DER<MUELLEN<<MARTIN<<<<<<<<"#,
    ),
    (
        7,
        720,
        r#"- MRZ (upper line): V<UTOARKFREITH<<<<<<<<<<<<<<<<<<<<<<"#,
    ),
    (
        7,
        760,
        r#"- MRZ (upper line): V<UTOPAPANDROPOULOUS<<STEPHEN<TREVOR"#,
    ),
    (7, 1104, r#"```"#),
    (7, 1106, r#"L898902C<3UTO6908061F9406236ZE184226B<<<<<<<"#),
    (7, 1136, r#"```"#),
    (7, 1138, r#"L898902C<3UTO6908061F9406236ZE184226"#),
];

/// One citation found in the crate: where it is, which Part and lines it names, and the file
/// name it spells out (`None` for a `source:` label, which names only the Part).
struct Citation {
    at: String,
    part: u32,
    lines: Vec<usize>,
    file: Option<String>,
}

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The `.rs` files under `dir`, recursively, in a stable order.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(read) => read.filter_map(|e| e.ok().map(|e| e.path())).collect(),
        Err(_) => return,
    };
    entries.sort();
    for path in entries {
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// The leading ASCII digits of `s`, parsed, and the rest of `s`.
fn leading_number(s: &str) -> Option<(usize, &str)> {
    let end = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    if end == 0 {
        return None;
    }
    s[..end].parse().ok().map(|n| (n, &s[end..]))
}

/// Every citation on one line of a source file.
fn citations_in_line(at: &str, line: &str, found: &mut Vec<Citation>) {
    // `Doc_9303_Part<N>_<title>.md:<a>` or `...md:<a>-<b>`.
    let mut rest = line;
    while let Some(start) = rest.find("Doc_9303_Part") {
        let tail = &rest[start..];
        let Some(md) = tail.find(".md") else { break };
        let file = &tail[..md + 3];
        rest = &tail[md + 3..];
        let Some(after_colon) = rest.strip_prefix(':') else {
            continue;
        };
        let Some((a, after_a)) = leading_number(after_colon) else {
            continue;
        };
        let mut lines = vec![a];
        if let Some((b, _)) = after_a.strip_prefix('-').and_then(leading_number) {
            lines.push(b);
        }
        let part = leading_number(&file["Doc_9303_Part".len()..]).map_or(0, |(p, _)| p as u32);
        found.push(Citation {
            at: at.to_string(),
            part,
            lines,
            file: Some(file.to_string()),
        });
    }
    // `source:` labels: `"9303 pt<N> §<clause>…:<line>…"`.
    let mut rest = line;
    while let Some(start) = rest.find("9303 pt") {
        let tail = &rest["9303 pt".len() + start..];
        rest = tail;
        let Some((part, after_part)) = leading_number(tail) else {
            continue;
        };
        if !after_part.starts_with(" §") {
            continue;
        }
        let label_end = after_part.find('"').unwrap_or(after_part.len());
        let label = &after_part[..label_end];
        if let Some((n, _)) = label
            .find(':')
            .and_then(|colon| leading_number(&label[colon + 1..]))
        {
            found.push(Citation {
                at: at.to_string(),
                part: part as u32,
                lines: vec![n],
                file: None,
            });
        }
    }
}

#[test]
fn docs9303_line_citations_land_on_the_cited_text() {
    let docs = crate_dir().join("../../knowledge/docs9303");
    if !docs.is_dir() {
        eprintln!("knowledge/docs9303 is not present (packaged crate?); nothing to check");
        return;
    }

    // The transcriptions, by Part: file name and lines.
    let mut parts: BTreeMap<u32, (String, Vec<String>)> = BTreeMap::new();
    for entry in fs::read_dir(&docs).expect("read knowledge/docs9303") {
        let name = entry
            .expect("directory entry")
            .file_name()
            .to_string_lossy()
            .into_owned();
        let Some(tail) = name.strip_prefix("Doc_9303_Part") else {
            continue;
        };
        let Some((part, _)) = leading_number(tail) else {
            continue;
        };
        if !name.ends_with(".md") {
            continue;
        }
        let text = fs::read_to_string(docs.join(&name)).expect("read a Doc 9303 transcription");
        parts.insert(
            part as u32,
            (name, text.lines().map(str::to_string).collect()),
        );
    }

    // Every citation in the crate's sources and its two prose files. CHANGELOG.md is history.
    let mut files = Vec::new();
    for dir in ["src", "tests", "examples"] {
        rust_files(&crate_dir().join(dir), &mut files);
    }
    for doc in ["README.md", "ROADMAP.md"] {
        files.push(crate_dir().join(doc));
    }
    let mut found = Vec::new();
    for path in &files {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        let shown = path
            .strip_prefix(crate_dir())
            .unwrap_or(path)
            .display()
            .to_string();
        for (i, line) in text.lines().enumerate() {
            citations_in_line(&format!("crates/mrz/{shown}:{}", i + 1), line, &mut found);
        }
    }
    assert!(
        !found.is_empty(),
        "the scanner found no citation at all; has the citation format changed?"
    );

    let pinned: BTreeMap<(u32, usize), &str> = CITED.iter().map(|&(p, n, s)| ((p, n), s)).collect();
    let mut cited: BTreeMap<(u32, usize), BTreeSet<String>> = BTreeMap::new();
    let mut problems = Vec::new();

    for c in &found {
        let Some((file_name, _)) = parts.get(&c.part) else {
            problems.push(format!(
                "{}: cites Doc 9303 Part {}, which has no transcription",
                c.at, c.part
            ));
            continue;
        };
        if let Some(file) = &c.file {
            if file != file_name {
                problems.push(format!(
                    "{}: cites {file}, but Part {}'s file is {file_name}",
                    c.at, c.part
                ));
            }
        }
        if c.lines.len() == 2 && c.lines[0] > c.lines[1] {
            problems.push(format!(
                "{}: range {}-{} runs backwards",
                c.at, c.lines[0], c.lines[1]
            ));
        }
        for &n in &c.lines {
            cited.entry((c.part, n)).or_default().insert(c.at.clone());
            if !pinned.contains_key(&(c.part, n)) {
                problems.push(format!(
                    "{}: cites Part {} line {n}, which has no row in CITED; add one with that line's text",
                    c.at, c.part
                ));
            }
        }
    }

    for (&(part, n), &prefix) in &pinned {
        let Some(sites) = cited.get(&(part, n)) else {
            problems.push(format!(
                "CITED row (Part {part}, line {n}) is no longer cited; remove it"
            ));
            continue;
        };
        let Some((file_name, lines)) = parts.get(&part) else {
            continue;
        };
        let actual = lines
            .get(n - 1)
            .map_or("<past the end of the file>", |l| l.trim_start());
        if !actual.starts_with(prefix) {
            let sites: Vec<&str> = sites.iter().map(String::as_str).collect();
            problems.push(format!(
                "{file_name}:{n} no longer starts with {prefix:?}; it reads {actual:?}.\n    Cited at: {}",
                sites.join(", ")
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "{} Doc 9303 citation problem(s):\n  {}",
        problems.len(),
        problems.join("\n  ")
    );
}
