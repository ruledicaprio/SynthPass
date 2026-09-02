//! Generates the parity harness's fixtures: for every corpus specimen whose
//! MRZ reads with valid check digits, a candidate ground-truth `Extraction`
//! and the OCR text a Tier-2 prompt would actually see.
//!
//! # Why this exists
//!
//! `crates/synthpass-llm/tests/parity.rs` is the only measurement of Tier-2
//! accuracy in the repository, and `knowledge/prompts/README.md` requires one
//! per `PROMPT_VERSION` bump — *"a prompt change with no measurement behind it
//! is indistinguishable from noise"*. It ran over **six** documents, because a
//! fixture is a hand-written pair and six is what anyone had written. Six
//! documents cannot separate a real regression from sampling noise, so in
//! practice nothing about Tier 2 was measurable at all. That is the gap this
//! closes, and it is the prerequisite for every VIZ change in
//! `knowledge/VIZ_TIER2_DESIGN.md`: cleaning up the visual zone is worth doing
//! only if the result can be seen.
//!
//! The corpus makes the fixtures nearly free. Ground truth here has always
//! carried `extraction_method: "mrz-deterministic"` — it was *derived from the
//! MRZ* in the first place. For a specimen whose check digits pass, the same
//! derivation is mechanical.
//!
//! # What a check digit does and does not prove
//!
//! It would be convenient to call a checksum-valid read "ground truth" and
//! score the model against all of it. That is false, and believing it would
//! corrupt the very measurement this exists to enable.
//!
//! ICAO 9303 check-digits exactly four fields — `document_number`,
//! `date_of_birth`, `date_of_expiry`, `personal_number` — in all of TD1, TD2
//! and TD3. The composite digit covers those same ranges and nothing else.
//! `nationality` and `sex` are excluded by the standard; every TD3 line-1 field
//! (`document_type`, `issuing_country`, `surname`, `given_names`) has no check
//! digit at all. This crate already states that split once, in
//! [`FieldConfidence::mrz_checksum_scope`], and this file reads it from there
//! rather than restating it.
//!
//! So an unreviewed candidate's name field is not truth — it is one OCR pass's
//! opinion, on the exact characters this corpus shows the recogniser getting
//! wrong (`PS` for `P<`, `OOO` for `SVK`, `SAB` for `USA`). Scoring a model
//! against it would do something worse than add noise: a model that reads the
//! *visual* zone correctly would be marked wrong for disagreeing with an OCR
//! error, and the VIZ work would measure as a regression precisely when it
//! succeeded.
//!
//! Hence two directories, and the difference is review, not format:
//!
//! | Path | Contents | Parity scores |
//! | :-- | :-- | :-- |
//! | `samples/ocr_fixtures/` | hand-verified by a person | every field |
//! | `samples/ocr_fixtures/derived/` | generated here, unreviewed | the four check-digited fields |
//!
//! Promoting a candidate is `git mv` one level up, after someone compares the
//! name fields against the image. Nothing here ever writes into the reviewed
//! directory's `.json` files.
//!
//! # Why the `.md` files are regenerated too
//!
//! Parity feeds the model a stored `.md` rather than running OCR, which is
//! right: it measures the model, not the recogniser, and keeps the two from
//! moving at once. But the six that existed came from `docling`, the Python
//! engine retired in v0.7.5 — HTML-escaped (`P&lt;HRV…`) and marked up with
//! `##` headings and `<!-- image -->` placeholders. The shipped pipeline feeds
//! Tier 2 `NativeOcr::recognize`'s plain text, which is neither. The harness
//! was measuring the model against an input shape production no longer
//! produces, so this writes every `.md` from the engine that actually runs.
//!
//! # Usage
//!
//! ```powershell
//! cargo run -p synthpass-bench --release --example ground_truth_candidates
//! cargo run -p synthpass-bench --release --example ground_truth_candidates -- --only Croatia
//! cargo run -p synthpass-bench --release --example ground_truth_candidates -- --check
//! ```
//!
//! `--check` reports without writing. `--only <substr>` narrows to matching
//! filenames; a full pass OCRs every specimen tagged as carrying an MRZ and
//! takes tens of minutes.
//!
//! # Regeneration is not bit-reproducible, and the fixtures are
//!
//! `NativeOcr::recognize_detailed` runs its MRZ retry passes under a wall-clock
//! budget, so a loaded or slower machine can stop retrying earlier and produce
//! different text. Rerunning this tool may therefore not reproduce the committed
//! files byte for byte. That is a property of the recogniser, not of these
//! fixtures: once written and reviewed in a pull request, a fixture is a fixed
//! input that never changes again, which is exactly what a measurement needs.
//! Treat a rerun as *proposing* new fixtures, and read the diff before taking
//! them.
//!
//! [`FieldConfidence::mrz_checksum_scope`]: synthpass_core::v2::FieldConfidence::mrz_checksum_scope
//! [`RoutingPolicy::escalate_on_line1_flagged`]: synthpass_die::routing::RoutingPolicy::escalate_on_line1_flagged

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use synthpass_core::fusion;
use synthpass_core::v2::CoreField;
use synthpass_ocr::NativeOcr;

/// Extraction method recorded in every generated fixture — the same value the
/// hand-written ones carry, because it describes how the record was derived
/// (from a checksum-valid MRZ) and that is identical either way. Review status
/// is carried by the directory, not by this field.
const EXTRACTION_METHOD: &str = "mrz-deterministic";

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
    let reviewed_dir = samples.join("ocr_fixtures");
    let derived_dir = reviewed_dir.join("derived");

    let candidates = load_candidates(&samples.join("corpus.jsonl"), only.as_deref());
    println!("specimens tagged as carrying an MRZ: {}", candidates.len());

    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    let mut report = Report::default();
    // Reviewed specimens are processed first so that when two names hold the
    // same bytes (this corpus has such a pair), the surviving fixture is the
    // one a person checked. `seen_content` then drops the twin: two fixtures
    // for one document would weight that document twice in the match rate.
    let mut seen_content: BTreeSet<&str> = BTreeSet::new();
    let mut seen_stem: BTreeSet<String> = BTreeSet::new();
    let mut outputs: Vec<Output> = Vec::new();

    for pass in [Pass::Reviewed, Pass::Derived] {
        for candidate in &candidates {
            let stem = candidate.stem();
            let is_reviewed = reviewed_dir.join(format!("{stem}.json")).is_file();
            if !pass.handles(is_reviewed) {
                continue;
            }
            // Both guards run before the OCR call: they are answered by the
            // manifest alone, and reading an image only to discard the result
            // costs seconds per specimen.
            if !seen_content.insert(&candidate.sha256) {
                report.skipped_duplicate.push(candidate.filename.clone());
                continue;
            }
            // Two images differing only in extension would fight over one
            // fixture stem, and the winner would depend on directory order.
            if !seen_stem.insert(stem.clone()) {
                report
                    .skipped_stem_collision
                    .push(candidate.filename.clone());
                continue;
            }

            let path = samples.join(&candidate.dir).join(&candidate.filename);
            let Ok(text) = ocr.recognize(&path) else {
                report.skipped_ocr_failed.push(candidate.filename.clone());
                continue;
            };

            match pass {
                // A reviewed fixture keeps its label whatever this pass reads:
                // a person checked it against the image, and this is one OCR
                // opinion. Only the model's input is refreshed.
                Pass::Reviewed => {
                    outputs.push(Output::markdown(
                        reviewed_dir.join(format!("{stem}.md")),
                        text,
                    ));
                    report.reviewed.push(stem);
                }
                Pass::Derived => match usable_as_fixture(&text) {
                    Ok(data) => {
                        outputs.push(Output {
                            path: derived_dir.join(format!("{stem}.json")),
                            // `to_string_pretty` emits no trailing newline,
                            // which is the shape the hand-written fixtures have.
                            body: fixture_json(&data),
                        });
                        outputs.push(Output::markdown(
                            derived_dir.join(format!("{stem}.md")),
                            text,
                        ));
                        report.derived.push(stem);
                    }
                    // Nothing here a fixture could honestly assert, so none is
                    // written. These stay visible as a coverage gap rather than
                    // becoming a fixture full of guesses.
                    Err(why) => report
                        .rejected
                        .push(format!("{} — {why}", candidate.filename)),
                },
            }
        }
    }

    report.print();

    if check_only {
        println!("\n--check: nothing written");
        return;
    }

    for dir in [&reviewed_dir, &derived_dir] {
        std::fs::create_dir_all(dir).expect("create fixture directory");
    }
    for out in &outputs {
        std::fs::write(&out.path, &out.body)
            .unwrap_or_else(|e| panic!("write {}: {e}", out.path.display()));
    }
    println!("\nwrote {} file(s)", outputs.len());
    println!(
        "review a candidate against its image, then promote it:\n  \
         git mv samples/ocr_fixtures/derived/<stem>.json samples/ocr_fixtures/"
    );
}

/// Which half of the corpus a pass handles. Reviewed specimens go first so a
/// content duplicate resolves in favour of the human-checked fixture.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pass {
    Reviewed,
    Derived,
}

impl Pass {
    fn handles(self, is_reviewed: bool) -> bool {
        (self == Pass::Reviewed) == is_reviewed
    }
}

/// One file to write, held until the whole corpus has been read so `--check`
/// and a failure part-way through both leave the fixtures untouched.
struct Output {
    path: PathBuf,
    body: String,
}

impl Output {
    /// OCR text is stored verbatim — it is the model's input, so anything added
    /// here would be measured as part of the prompt. A trailing newline is the
    /// one concession, so the file ends the way a text file should.
    fn markdown(path: PathBuf, text: String) -> Self {
        let body = if text.ends_with('\n') {
            text
        } else {
            format!("{text}\n")
        };
        Self { path, body }
    }
}

/// Decides whether an OCR pass produced something a fixture can honestly
/// assert, returning the parse or the reason it was rejected.
///
/// **Valid check digits are not enough, and assuming they were produced
/// nonsense on the first run of this tool.** `Croatia_ID_Specimen_2002_back`
/// OCR'd to
///
/// ```text
/// IOHRVOOOOOOOOOO<K<<<<<<<<<<<<<<<<<<K
/// TOHRVO0O00OOO00<<<<<<<<<<<<<<<<<<<<<
/// ```
///
/// which [`mrz::MrzData::valid`] accepts. It is not a coincidence and it is not
/// rare: the ICAO weighting scores the filler `<` as **zero**, so a field that
/// is entirely filler sums to zero and its `<` check digit — also zero —
/// verifies vacuously. Date of birth, date of expiry and the optional field all
/// passed that way here; only the document number's digit was a real 1-in-10
/// coincidence. The resulting record claims a date of expiry of `<<<<<<`, a
/// nationality of `OOO` and a surname of `OOOOOOOOOO K`, with
/// `mrz_checksums_valid: true`.
///
/// As ground truth that is worse than useless — a model would be scored against
/// noise. None of this is news to the codebase: it is exactly what
/// [`RoutingPolicy::escalate_on_line1_flagged`] was added for, and that hook is
/// off by default with the reason written down — *"a signal here gets enabled
/// only once the benchmark harness has produced data saying what it actually
/// predicts"*. So the gate is the crate's own deterministic cross-checks, not a
/// new heuristic invented here:
///
/// * [`fusion::check_line1_integrity`] — unrecognised country or nationality,
///   degenerate character runs, missing name separators. It catches the case
///   above three times over.
/// * [`mrz::MrzData::validity`] — the two judgements that do not depend on
///   today's date: both dates parse as real calendar dates, and birth precedes
///   expiry.
///
/// This deliberately also rejects the corpus's genuinely non-conformant line-1
/// specimens — Spain's missing issuing state, Argentina's shifted field
/// boundary, Germany's `BDR` printer code, Armenia's all-`X` template. Each is
/// a real document and each is pinned by a test in `crates/mrz/tests/`, but
/// none of them may become *unreviewed* ground truth: a person has to say what
/// those line-1 fields are, because no check digit can.
fn usable_as_fixture(text: &str) -> Result<mrz::MrzData, String> {
    let data = mrz::find_and_parse(text).map_err(|_| "no MRZ parsed".to_string())?;
    if !data.valid() {
        return Err("check digits failed".into());
    }
    let verdict = fusion::check_line1_integrity(&data);
    if verdict.is_flagged() {
        let kinds: Vec<String> = verdict.kinds().iter().map(ToString::to_string).collect();
        return Err(format!("line-1 integrity: {}", kinds.join(", ")));
    }
    // Any fixed reference date works: `dates_well_formed` and
    // `dob_before_expiry` are properties of the two printed dates alone. Only
    // `in_date` and `days_until_expiry` depend on *when* this runs, and neither
    // is read here — a fixture whose contents changed overnight would be a
    // determinism bug, not a measurement.
    let validity = data.validity(mrz::Date::from_epoch_days(0));
    if !validity.dates_well_formed {
        return Err("dates are not real calendar dates".into());
    }
    if !validity.dob_before_expiry {
        return Err("date of birth is not before date of expiry".into());
    }
    Ok(data)
}

/// Builds the fixture body from a checksum-valid parse.
///
/// Field values are read through [`CoreField`] rather than named one by one, so
/// this cannot drift from the schema, and they arrive via
/// `synthpass_die::mrz_reader::extraction_v2_from_mrz` — the single conversion
/// the Tier-1 provider itself uses. Two hand-written copies of that mapping
/// already drifted once in this repository; this is deliberately not a third.
///
/// `validity` is **omitted**, and that is not an oversight: it holds
/// `in_date` and `days_until_expiry`, both computed against *today*. A fixture
/// carrying them would change meaning every night and fail its own regeneration
/// check the next day. The hand-written fixtures omit it too.
fn fixture_json(data: &mrz::MrzData) -> String {
    let v2 = synthpass_die::mrz_reader::extraction_v2_from_mrz(data);
    let mut map = serde_json::Map::new();
    for field in CoreField::ALL {
        map.insert(
            field.as_str().to_string(),
            match v2.fields.get(field) {
                Some(v) => serde_json::Value::String(v.to_string()),
                None => serde_json::Value::Null,
            },
        );
    }
    map.insert("mrz_line".into(), data.mrz_lines.clone().into());
    map.insert("mrz_checksums_valid".into(), true.into());
    map.insert("extraction_method".into(), EXTRACTION_METHOD.into());
    serde_json::to_string_pretty(&serde_json::Value::Object(map)).expect("fixture serializes")
}

/// A corpus row this tool might turn into a fixture.
struct Candidate {
    filename: String,
    dir: String,
    sha256: String,
}

impl Candidate {
    fn stem(&self) -> String {
        self.filename
            .rsplit_once('.')
            .map_or(self.filename.as_str(), |(s, _)| s)
            .to_string()
    }
}

/// Reads `samples/corpus.jsonl` and keeps the rows tagged as carrying an MRZ.
///
/// Selection comes from the manifest rather than a directory walk for the
/// reason the manifest exists at all: `mrz.present` is a human's claim about
/// the image, so a page nobody believes has an MRZ is never OCR'd looking for
/// one. Rows whose *stored* observation failed its check digits are still
/// visited — the observation is one pass's result, and re-reading is the point.
fn load_candidates(manifest: &Path, only: Option<&str>) -> Vec<Candidate> {
    let text = std::fs::read_to_string(manifest).unwrap_or_else(|e| {
        panic!(
            "cannot read {} ({e}) — it is tracked on main; regenerate it with \
             `cargo run -p synthpass-ocr --release --example corpus_manifest`",
            manifest.display()
        )
    });
    let mut out: Vec<Candidate> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter(|row| row["mrz"]["present"].as_bool() == Some(true))
        .filter_map(|row| {
            Some(Candidate {
                filename: row["filename"].as_str()?.to_string(),
                dir: row["dir"].as_str()?.to_string(),
                sha256: row["sha256"].as_str()?.to_string(),
            })
        })
        .filter(|c| only.is_none_or(|s| c.filename.contains(s)))
        .collect();
    out.sort_by(|a, b| a.filename.cmp(&b.filename));
    out
}

#[derive(Default)]
struct Report {
    reviewed: Vec<String>,
    derived: Vec<String>,
    rejected: Vec<String>,
    skipped_duplicate: Vec<String>,
    skipped_stem_collision: Vec<String>,
    skipped_ocr_failed: Vec<String>,
}

impl Report {
    fn print(&self) {
        println!(
            "\nreviewed fixtures refreshed (.md only): {}",
            self.reviewed.len()
        );
        println!(
            "derived candidates written (.json + .md): {}",
            self.derived.len()
        );
        println!(
            "parity denominator: {} document(s) on the four check-digited fields, \
             {} on all ten",
            self.reviewed.len() + self.derived.len(),
            self.reviewed.len()
        );

        // Each of these is a reason a specimen produced no fixture. They are
        // printed in full rather than counted: a corpus that quietly stops
        // contributing documents is exactly how a measurement rots.
        Self::list(
            "not usable as unreviewed ground truth (see `usable_as_fixture`)",
            &self.rejected,
        );
        Self::list(
            "byte-identical to a specimen already written (one document, one fixture)",
            &self.skipped_duplicate,
        );
        Self::list(
            "another image already claimed this fixture stem",
            &self.skipped_stem_collision,
        );
        Self::list("OCR failed to read the image", &self.skipped_ocr_failed);
    }

    fn list(label: &str, items: &[String]) {
        if items.is_empty() {
            return;
        }
        println!("\n{} — {}:", items.len(), label);
        for i in items {
            println!("  {i}");
        }
    }
}

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = crates/synthpass-bench → repo root is two levels up.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}
