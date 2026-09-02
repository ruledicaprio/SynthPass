//! Generates `samples/corpus.jsonl` — one row per specimen image, lifting the
//! corpus metadata out of filenames and into reviewable, machine-checkable
//! text.
//!
//! # Why this exists
//!
//! The passport corpus encodes nine facts in each filename, including the
//! first five characters of MRZ line 1 (the ICAO document code at positions
//! 1-2 and the issuing state at 3-5, with `0` standing in for the filler `<`
//! because `<` is awkward in a path). That convention is doing a database's
//! job, and it has already failed in four distinct ways:
//!
//! * **arity** — `PE0_ARG` carries three characters where two are expected;
//!   `Spain_Passport_Specimen_P0_2022_mrz.png` lost a whole field.
//! * **homoglyphs** — `PO` (letter O, genuinely used by the DPRK) versus `P0`
//!   (the `P<` filler form) differ by one pixel and decide correctness.
//! * **contradiction** — several files carry a document code *and* `no_mrz`.
//! * **ambiguity** — the year slot means the issue year in most filenames and
//!   the expiry year in at least one.
//!
//! A filename cannot be validated. A manifest can, and
//! `tests/corpus_manifest.rs` does exactly that.
//!
//! # Truth versus observation
//!
//! Each row separates the two. `mrz.document_code` / `mrz.issuing_state` are
//! the **accepted values** and come from the filename alone — written by a
//! human who looked at the image. `mrz.observed` records what *this* OCR pass
//! read, with an `agrees` flag. There is deliberately no OCR fallback: a name
//! that claims nothing records nothing, and `mrz.source` says `"unrecorded"`
//! so the gap is visible rather than filled with a guess.
//!
//! That is not deference to convention. Line 1 positions 1-5 carry no check
//! digit in any ICAO format, so a misread there is unarbitrated, and this
//! corpus shows the recogniser getting them wrong on most passports — `PS` for
//! `P<` repeatedly, `OOO` for `SVK`, `SAB` for `USA`, `P<DKH…` for a true
//! `P<D<<HEINKEL…`. It will also return a structural parse from an image with
//! no MRZ at all: `Belgium_ID_Specimen_front_no_mrz.jpg` yields an issuing
//! state of `DEL`. An earlier draft of this file fell back to OCR and recorded
//! exactly that as truth.
//!
//! Everything else a human must decide — `year.kind`, `provenance`, and every
//! `notes` string — is **preserved from the existing manifest** and never
//! overwritten. The generator proposes; a person confirms; the result lands in
//! git as reviewed text.
//!
//! # Usage
//!
//! ```powershell
//! cargo run -p synthpass-ocr --release --example corpus_manifest
//! cargo run -p synthpass-ocr --release --example corpus_manifest -- --only Spain
//! cargo run -p synthpass-ocr --release --example corpus_manifest -- --check
//! ```
//!
//! `--check` re-reads the corpus and reports disagreements without writing,
//! which is the mode worth running after a rename. `--only <substr>` narrows
//! to matching filenames; a full pass OCRs every image and takes tens of
//! minutes. Rows whose image is byte-identical to the one already recorded
//! (same `sha256`) are reused rather than re-read, so a rerun after adding a
//! handful of specimens is cheap.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use synthpass_ocr::NativeOcr;

/// Extensions treated as corpus images.
///
/// `walk_samples` in `integrity_survey.rs` deliberately takes every file it
/// finds; here that would feed the loose `samples/synthpass-bench-report-*.json`
/// reports into the OCR engine, so this walk filters.
const IMAGE_EXTENSIONS: [&str; 5] = ["jpg", "jpeg", "png", "webp", "gif"];

/// Directories mirrored to `samples-data`, plus the tracked fixture directory.
const CORPUS_DIRS: [&str; 5] = [
    "passports",
    "id_cards",
    "driving_licenses",
    "misc",
    "ocr_fixtures",
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let check_only = args.iter().any(|a| a == "--check");
    let only = args
        .iter()
        .position(|a| a == "--only")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let root = repo_root();
    let samples = root.join("samples");
    let manifest_path = samples.join("corpus.jsonl");

    let previous = load_previous(&manifest_path);
    println!("existing manifest: {} row(s)", previous.len());

    let mut images = Vec::new();
    for dir in CORPUS_DIRS {
        collect_images(&samples.join(dir), &mut images);
    }
    images.sort();
    if let Some(substr) = &only {
        images.retain(|p| {
            p.file_name()
                .and_then(|f| f.to_str())
                .is_some_and(|f| f.contains(substr.as_str()))
        });
    }
    println!("corpus images: {}", images.len());

    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    // Hand-labelled ground truth is keyed by file stem under ocr_fixtures/.
    // Recording the link here is what lets a caller stop hard-coding that join
    // and lets the coverage gap be counted rather than guessed at.
    let ground_truth = GroundTruth::load(&samples);
    println!("ground-truth labels available: {}", ground_truth.len());

    let mut rows: BTreeMap<String, serde_json::Value> = BTreeMap::new();
    let mut reused = 0usize;
    let mut disagreements = Vec::new();

    for (i, path) in images.iter().enumerate() {
        let filename = file_name(path);
        let digest = sha256_of(path);
        let dir = parent_dir(path);
        let claims = FilenameClaims::parse(&filename);

        // Byte-identical to what the manifest already records: reuse the stored
        // OCR observation rather than re-reading the image. The row is still
        // rebuilt from it, so changing the row schema costs a fast rerun
        // instead of another full pass over the corpus (tens of minutes).
        let prev_row = previous.get(&filename);
        let unchanged = prev_row
            .and_then(|p| p.get("sha256"))
            .and_then(|v| v.as_str())
            == Some(digest.as_str());

        let observed = if unchanged {
            reused += 1;
            ObservedMrz::from_previous(prev_row.expect("checked above"))
        } else {
            let seen = read_mrz(&ocr, path);
            eprintln!(
                "[{}/{}] {filename} — {}",
                i + 1,
                images.len(),
                seen.summary()
            );
            seen
        };

        if let (Some(seen), Some(claimed)) = (&observed.document_code, &claims.document_code) {
            if seen != claimed {
                disagreements.push(format!(
                    "{filename}: filename {claimed:?}, OCR read {seen:?} (document code)"
                ));
            }
        }
        if let (Some(seen), Some(claimed)) = (&observed.issuing_state, &claims.issuing_state) {
            if seen != claimed {
                disagreements.push(format!(
                    "{filename}: filename {claimed:?}, OCR read {seen:?} (issuing state)"
                ));
            }
        }
        // A `no_mrz` tag contradicted by the image is worth separating by
        // strength, because the two readings mean opposite things.
        //
        // A *checksum-valid* read off a page tagged `no_mrz` is near-proof the
        // tag is wrong: passing every check digit by chance is vanishingly
        // unlikely, and the fields come back coherent. That is exactly what
        // happened to `Monaco_ID_Specimen_XXXX_front_no_mrz.png`, which read
        // `IC`/`MCO` with valid checksums — a correct read of a *back* side
        // filed under a front-side name. It was first reported here as a Tier-1
        // false positive, which it was not.
        //
        // A read that parses structurally but *fails* its check digits is the
        // opposite and is routine: visual-zone text lands in an MRZ-shaped line
        // often enough that 18 specimens do it. Reporting both at one volume
        // buried the signal in the noise.
        if claims.mrz_present == Some(false) {
            if observed.checksums_valid {
                disagreements.push(format!(
                    "{filename}: LABEL LIKELY WRONG — tagged no_mrz, but OCR read a \
                     CHECKSUM-VALID {} MRZ ({}{}). Check whether this is the other side \
                     of the document.",
                    observed.format.as_deref().unwrap_or("?"),
                    observed.document_code.as_deref().unwrap_or("??"),
                    observed.issuing_state.as_deref().unwrap_or("???"),
                ));
            }
        } else if claims.mrz_present == Some(true) && !observed.found {
            // Benign on its own — OCR misses MRZs on redacted and low-contrast
            // scans routinely — but worth listing so a genuinely absent MRZ can
            // be spotted.
            disagreements.push(format!("{filename}: tagged mrz, but OCR read none"));
        }

        rows.insert(
            filename.clone(),
            build_row(
                &filename,
                &dir,
                &digest,
                &claims,
                &observed,
                prev_row,
                &ground_truth,
            ),
        );
    }

    println!(
        "\n{} row(s) reused unchanged, {} read",
        reused,
        images.len() - reused
    );

    // These are reported, never auto-resolved. Line 1's first five characters
    // carry no check digit, and the recogniser loses `<` fillers routinely, so
    // a disagreement usually means the OCR is wrong and the human who named the
    // file was right. Each one is worth a human glance at the image; a genuine
    // filename error is then fixed by renaming, not by trusting this output.
    if disagreements.is_empty() {
        println!("OCR agreed with every filename claim it could check");
    } else {
        println!(
            "\n{} filename/OCR disagreement(s) — filename wins, review each:",
            disagreements.len()
        );
        for d in &disagreements {
            println!("  {d}");
        }
    }

    if check_only {
        println!("\n--check: nothing written");
        return;
    }

    // Rows carried over from the previous manifest whose image is gone stay
    // out: the manifest describes the corpus that exists, and a stale row is
    // exactly the drift this file is meant to prevent. When --only narrows the
    // pass, untouched rows are preserved so a partial run is not destructive.
    if only.is_some() {
        for (name, row) in &previous {
            rows.entry(name.clone()).or_insert_with(|| row.clone());
        }
    }

    let body: String = rows
        .values()
        .map(|r| format!("{}\n", serde_json::to_string(r).expect("row serializes")))
        .collect();
    std::fs::write(&manifest_path, body).expect("write manifest");
    println!(
        "\nwrote {} row(s) to {}",
        rows.len(),
        manifest_path.display()
    );
}

/// What *this OCR pass* read off the page — an observation, not the truth.
///
/// Line 1's first five characters are not check-digited in any ICAO format, so
/// nothing arbitrates a misread of them. In practice the recogniser loses the
/// `<` filler constantly: the German specimens here read as `PBDRM…` for a true
/// `P<BDRMUSTERMANN…`, and `P<DKH…` for a true `P<D<<HEINKEL…`. A human reading
/// the image is simply better at this than the current engine, which is why the
/// filename claim outranks this struct when the two disagree.
struct ObservedMrz {
    found: bool,
    /// Positions 1-2 of line 1, verbatim — `<` fillers included, untrimmed.
    document_code: Option<String>,
    /// Positions 3-5 of line 1, verbatim — so Germany's `D<<` stays `D<<`.
    issuing_state: Option<String>,
    format: Option<String>,
    checksums_valid: bool,
}

impl ObservedMrz {
    fn none() -> Self {
        Self {
            found: false,
            document_code: None,
            issuing_state: None,
            format: None,
            checksums_valid: false,
        }
    }

    /// Recovers a stored observation from an existing manifest row, so an
    /// unchanged image never needs re-reading.
    fn from_previous(row: &serde_json::Value) -> Self {
        let o = &row["mrz"]["observed"];
        let s = |k: &str| o[k].as_str().map(str::to_string);
        Self {
            found: o["read_by_ocr"].as_bool().unwrap_or(false),
            document_code: s("document_code"),
            issuing_state: s("issuing_state"),
            format: s("format"),
            checksums_valid: o["checksums_valid"].as_bool().unwrap_or(false),
        }
    }

    fn summary(&self) -> String {
        if !self.found {
            return "no MRZ found".to_string();
        }
        format!(
            "{}{} {} checksums={}",
            self.document_code.as_deref().unwrap_or("??"),
            self.issuing_state.as_deref().unwrap_or("???"),
            self.format.as_deref().unwrap_or("?"),
            self.checksums_valid,
        )
    }
}

fn read_mrz(ocr: &NativeOcr, path: &Path) -> ObservedMrz {
    let Ok(text) = ocr.recognize(path) else {
        return ObservedMrz::none();
    };
    // Accept a parse whose checksums fail: the point here is line 1's first
    // five characters, which are not check-digited in any format, so a failed
    // composite digit says nothing about them.
    let Ok(data) = mrz::find_and_parse(&text) else {
        return ObservedMrz::none();
    };
    let line1 = data.mrz_lines.lines().next().unwrap_or_default();
    let (code, state) = if line1.len() >= 5 {
        (Some(line1[0..2].to_string()), Some(line1[2..5].to_string()))
    } else {
        (None, None)
    };
    ObservedMrz {
        found: true,
        document_code: code,
        issuing_state: state,
        format: Some(format!("{:?}", data.format)),
        checksums_valid: data.valid(),
    }
}

/// Whether an issuing state resolves against [`mrz::country_name`] once its
/// fillers are trimmed.
///
/// This is the mechanical half of "is line 1 conformant". It accepts Germany's
/// legacy `D<<`, and rejects Spain's missing state field, Argentina's shifted
/// field boundary and Armenia's all-`X` template. It also rejects `BDR`, the
/// Bundesdruckerei specimen-printer code — which is the correct production
/// behaviour, since a `BDR` document is by construction not a real passport;
/// `notes` carries that distinction where it matters.
fn state_is_recognized(state: Option<&str>) -> bool {
    state.is_some_and(|s| mrz::country_name(s.trim_end_matches('<')).is_some())
}

/// The facts a filename asserts, for the passport convention
/// `<country>_<type>_<Specimen|Private>_<code>_<state>_<year>[_redacted]_<mrz|no_mrz>[_variant…]`.
///
/// Older `id_cards/` and `driving_licenses/` names predate the code/state
/// slots; those fields simply come back `None` rather than being forced.
struct FilenameClaims {
    provenance: Option<String>,
    /// Filename form translated back to MRZ characters (`0` → `<`), so it is
    /// directly comparable with what the image reads.
    document_code: Option<String>,
    issuing_state: Option<String>,
    year: Option<u32>,
    mrz_present: Option<bool>,
    redacted: bool,
    variants: Vec<String>,
}

impl FilenameClaims {
    fn parse(filename: &str) -> Self {
        let stem = filename.rsplit_once('.').map_or(filename, |(s, _)| s);
        let parts: Vec<&str> = stem.split('_').collect();
        let lower = stem.to_ascii_lowercase();

        let provenance = if lower.contains("specimen") {
            Some("specimen".to_string())
        } else if lower.contains("private") {
            Some("private".to_string())
        } else {
            None
        };

        // `no_mrz` must be tested before `mrz`, since it contains it.
        let mrz_present = if lower.contains("no_mrz") {
            Some(false)
        } else if lower.contains("mrz") {
            Some(true)
        } else {
            None
        };

        // The code/state pair sits immediately after Specimen/Private. A code
        // is 2-3 chars of [A-Z0-9]; a state is exactly 3. `XX`/`XXX` are the
        // corpus's sentinels for "unknown", not real MRZ characters, so they
        // are left unclaimed rather than compared against a real read.
        let mut document_code = None;
        let mut issuing_state = None;
        if let Some(i) = parts
            .iter()
            .position(|p| p.eq_ignore_ascii_case("Specimen") || p.eq_ignore_ascii_case("Private"))
        {
            if let (Some(code), Some(state)) = (parts.get(i + 1), parts.get(i + 2)) {
                if is_code_token(code) && is_state_token(state) {
                    document_code = Some(unslash(code));
                    issuing_state = Some(unslash(state));
                }
            }
        }

        let year = parts
            .iter()
            .find(|p| p.len() == 4 && p.chars().all(|c| c.is_ascii_digit()))
            .and_then(|p| p.parse().ok());

        let known = [
            "passport",
            "id",
            "driving",
            "license",
            "licence",
            "specimen",
            "private",
            "mrz",
            "no",
            "redacted",
            "borderpass",
            "visa",
        ];
        let variants: Vec<String> = parts
            .iter()
            .skip(1)
            .filter(|p| {
                let l = p.to_ascii_lowercase();
                !known.contains(&l.as_str())
                    && !(p.len() == 4 && p.chars().all(|c| c.is_ascii_digit()))
                    && !is_code_token(p)
                    && !is_state_token(p)
                    && l.chars().all(|c| c.is_ascii_lowercase())
            })
            .map(|p| p.to_string())
            .collect();

        Self {
            provenance,
            document_code,
            issuing_state,
            year,
            mrz_present,
            redacted: lower.contains("redacted"),
            variants,
        }
    }
}

fn is_code_token(s: &str) -> bool {
    (2..=3).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        && s.chars().next().is_some_and(|c| c.is_ascii_uppercase())
}

fn is_state_token(s: &str) -> bool {
    s.len() == 3
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Filenames substitute `0` for the MRZ filler `<`, which a path cannot hold.
fn unslash(token: &str) -> String {
    token.replace('0', "<")
}

fn build_row(
    filename: &str,
    dir: &str,
    sha256: &str,
    claims: &FilenameClaims,
    observed: &ObservedMrz,
    previous: Option<&serde_json::Value>,
    ground_truth: &GroundTruth,
) -> serde_json::Value {
    // Human-authored fields survive regeneration untouched.
    let carried = |key: &str| -> serde_json::Value {
        previous
            .and_then(|p| p.get(key))
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    };
    let year_kind = previous
        .and_then(|p| p.get("year"))
        .and_then(|y| y.get("kind"))
        .cloned()
        .unwrap_or_else(|| serde_json::Value::String("issue".to_string()));

    let provenance = carried("provenance");
    let provenance = if provenance.is_null() {
        claims
            .provenance
            .clone()
            .map_or(serde_json::Value::Null, serde_json::Value::String)
    } else {
        provenance
    };

    // The manifest asserts only what a human verified: the filename claim, and
    // nothing else. There is deliberately no OCR fallback here.
    //
    // Falling back looked reasonable and was wrong. Line 1 positions 1-5 carry
    // no check digit in any ICAO format, and this corpus shows the recogniser
    // misreading them on most passports (`PS` for `P<` repeatedly, `OOO` for
    // `SVK`, `SAB` for `USA`). Worse, on an image with no MRZ at all it can
    // still return a structural parse: `Belgium_ID_Specimen_front_no_mrz.jpg`
    // yielded an issuing state of `DEL`, which a fallback promoted to truth.
    //
    // So an old-convention name that claims nothing records nothing, and the
    // gap is visible rather than papered over with a guess. The machine's read
    // stays in `observed`, clearly labelled as machine output.
    let document_code = claims.document_code.clone();
    let issuing_state = claims.issuing_state.clone();

    let observed_agrees = observed.found
        && observed.document_code == document_code
        && observed.issuing_state == issuing_state;

    serde_json::json!({
        "filename": filename,
        "dir": dir,
        "sha256": sha256,
        "provenance": provenance,
        "mrz": {
            "present": claims.mrz_present.unwrap_or(observed.found),
            "redacted": claims.redacted,
            "document_code": document_code,
            "issuing_state": issuing_state,
            "issuing_state_recognized": state_is_recognized(issuing_state.as_deref()),
            "source": if claims.document_code.is_some() { "filename" } else { "unrecorded" },
            "observed": {
                "read_by_ocr": observed.found,
                "document_code": observed.document_code,
                "issuing_state": observed.issuing_state,
                "format": observed.format,
                "checksums_valid": observed.checksums_valid,
                "agrees": observed_agrees,
            },
        },
        "year": { "value": claims.year, "kind": year_kind },
        "variants": claims.variants,
        "ground_truth_stem": ground_truth.stem_for(filename),
        "expected_document_number": ground_truth.expected_document_number(filename, previous),
        "notes": carried("notes"),
    })
}

/// The hand-labelled ground truth available under `samples/ocr_fixtures/`,
/// and the lookups that resolve an image to it.
///
/// `synthpass_bench::load_ground_truth` joins `ocr_fixtures/<image stem>.json`,
/// so the link holds only while the two names agree. The corpus rename broke
/// 13 of 18 of them -- recording the survivors turns a silent `None` at lookup
/// time into a countable coverage number.
struct GroundTruth {
    dir: PathBuf,
    stems: std::collections::BTreeSet<String>,
}

impl GroundTruth {
    fn load(samples: &Path) -> Self {
        let dir = samples.join("ocr_fixtures");
        let stems = std::fs::read_dir(&dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .filter_map(|p| p.file_stem().and_then(|s| s.to_str()).map(str::to_string))
            .collect();
        Self { dir, stems }
    }

    fn len(&self) -> usize {
        self.stems.len()
    }

    /// The label stem for an image, if one exists.
    fn stem_for(&self, filename: &str) -> Option<String> {
        let stem = filename.rsplit_once('.').map_or(filename, |(s, _)| s);
        self.stems.contains(stem).then(|| stem.to_string())
    }

    /// The document number `mrz_corpus` should expect from this specimen.
    ///
    /// Read from the label when there is one; otherwise the value already in
    /// the manifest is carried forward, so a specimen can be a corpus entry
    /// without a full `Extraction` label -- which the old hard-coded `CORPUS`
    /// array supported and this must not quietly drop. Three entries (Oman
    /// 2004, Viet Nam 2023, Israel 2003) had a known document number and no
    /// `.json`; deriving purely from labels would have removed them, and since
    /// all three were *known misses* that would have moved the reported hit
    /// rate from 18/21 to a flattering 18/18 by shrinking the denominator --
    /// the exact failure `mrz_corpus`'s own comment warns against.
    fn expected_document_number(
        &self,
        filename: &str,
        previous: Option<&serde_json::Value>,
    ) -> serde_json::Value {
        if let Some(stem) = self.stem_for(filename) {
            let label = self.dir.join(format!("{stem}.json"));
            if let Ok(bytes) = std::fs::read(&label) {
                if let Ok(gt) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    if let Some(doc) = gt.get("document_number").and_then(|v| v.as_str()) {
                        return serde_json::Value::String(doc.to_string());
                    }
                }
            }
        }
        previous
            .and_then(|p| p.get("expected_document_number"))
            .cloned()
            .unwrap_or(serde_json::Value::Null)
    }
}

fn load_previous(path: &Path) -> BTreeMap<String, serde_json::Value> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return BTreeMap::new();
    };
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| {
            let name = v.get("filename")?.as_str()?.to_string();
            Some((name, v))
        })
        .collect()
}

fn collect_images(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_images(&path, out);
        } else if path.is_file() && is_image(&path) {
            out.push(path);
        }
    }
}

fn is_image(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .is_some_and(|e| IMAGE_EXTENSIONS.contains(&e.as_str()))
}

fn sha256_of(path: &Path) -> String {
    let bytes = std::fs::read(path).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    // sha2 0.11 returns a `hybrid_array::Array`, which has no `LowerHex`;
    // build.rs formats its model digests the same way.
    let mut out = String::new();
    for b in hasher.finalize() {
        use std::fmt::Write as _;
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default()
        .to_string()
}

fn parent_dir(path: &Path) -> String {
    path.parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or_default()
        .to_string()
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/synthpass-ocr → repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
