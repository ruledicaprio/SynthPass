//! Offline specimen review and fixture promotion, in Rust so validation uses
//! the benchmark's own `mrz` parser and `synthpass_core::Extraction` schema.
//!
//! Specimen images are PII-adjacent: review HTML is restricted to gitignored
//! `artifacts/` in this worktree. No network, telemetry, or automatic correction
//! of a human transcription is used. Names are MRZ-form truth (ADR-0013).

use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use base64::Engine;
use image::DynamicImage;
use serde::Deserialize;
use serde_json::Value;
use synthpass_core::v2::CoreField;
use synthpass_core::Extraction;
use synthpass_ocr::NativeOcr;

type Result<T> = std::result::Result<T, String>;
type Fields = BTreeMap<String, Option<String>>;

// ADR-0011 amendment (2026-09-17): frozen M6 residual, excluding the closed
// San Marino blank template. Cross-checked against corpus.jsonl and the ledger.
const BATCH_A: [&str; 13] = [
    "id_cards/France_ID_Specimen_2020_back_mrz.png",
    "id_cards/Italy_ID_Specimen_2022_back_mrz.jpg",
    "passports/Moldova_Passport_Specimen_PA_MDA_2014_no_mrz.jpeg",
    "passports/Afghanistan_Passport_Specimen_P0_AFG_2016_mrz.webp",
    "id_cards/Belgium_ID_Specimen_2021_back_mrz.png",
    "id_cards/Croatia_ID_Specimen_2021_back_mrz.jpg",
    "passports/Czechia_Passport_Specimen_P0_CZE_2005_mrz.jpg",
    "passports/Germany_Passport_Specimen_P0_D00_2024_mrz.jpg",
    "passports/Hong_Kong_Passport_Specimen_P0_HKG_2007_mrz.png",
    "passports/Hong_Kong_Passport_Specimen_P0_HKG_2019_mrz.png",
    "passports/Romania_Passport_Specimen_PE_ROU_2024_mrz.jpg",
    "passports/Russian_Federation_Passport_Specimen_P0_RUS_2019_mrz.jpg",
    "id_cards/Sweden_ID_Specimen_2022_back_mrz.jpg",
];
const FIELD_NAMES: [&str; 11] = [
    "document_type",
    "issuing_country",
    "surname",
    "given_names",
    "nationality",
    "sex",
    "document_number",
    "date_of_birth",
    "date_of_expiry",
    "personal_number",
    "mrz_line",
];
const HELP: &str = "ground-truth [--samples-root samples] [--fixtures-dir samples/ocr_fixtures]
  review [--out artifacts/ground-truth-review.html] [--batch a|b|all] [--no-ocr]
  apply <verified.json> [--write]
Global options may also follow the subcommand. Apply defaults to dry run.";

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Status {
    Verified,
    NonConforming,
    Skip,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    stem: String,
    asset: String,
    status: Status,
    fields: Fields,
}

struct Options {
    samples: PathBuf,
    fixtures: PathBuf,
    command: String,
    out: PathBuf,
    batch: String,
    no_ocr: bool,
    write: bool,
    input: Option<PathBuf>,
}

/// Run the command with arguments excluding the executable name.
/// Returns `false` if apply rejected any entries; diagnostics never print field values.
pub fn run(args: Vec<String>) -> Result<bool> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{HELP}");
        return Ok(true);
    }
    let options = options(args)?;
    match options.command.as_str() {
        "review" => {
            review(&options)?;
            Ok(true)
        }
        "apply" => apply(&options),
        _ => Err(HELP.into()),
    }
}

fn options(args: Vec<String>) -> Result<Options> {
    let mut o = Options {
        samples: "samples".into(),
        fixtures: "samples/ocr_fixtures".into(),
        command: String::new(),
        out: "artifacts/ground-truth-review.html".into(),
        batch: "all".into(),
        no_ocr: false,
        write: false,
        input: None,
    };
    let mut args = args.into_iter();
    let mut review_option = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--samples-root" => {
                o.samples = args
                    .next()
                    .ok_or("--samples-root needs a directory")?
                    .into()
            }
            "--fixtures-dir" => {
                o.fixtures = args
                    .next()
                    .ok_or("--fixtures-dir needs a directory")?
                    .into()
            }
            "--out" => {
                review_option = true;
                o.out = args.next().ok_or("--out needs a file")?.into();
            }
            "--batch" => {
                review_option = true;
                o.batch = args.next().ok_or("--batch needs a|b|all")?;
            }
            "--no-ocr" => {
                review_option = true;
                o.no_ocr = true;
            }
            "--write" => o.write = true,
            "review" | "apply" if o.command.is_empty() => o.command = arg,
            _ if o.command == "apply" && !arg.starts_with('-') && o.input.is_none() => {
                o.input = Some(arg.into())
            }
            _ => return Err(format!("unexpected argument; {HELP}")),
        }
    }
    if !matches!(o.batch.as_str(), "a" | "b" | "all")
        || (o.command == "review" && o.write)
        || (o.command == "apply" && (review_option || o.input.is_none()))
        || o.command.is_empty()
    {
        return Err(HELP.into());
    }
    Ok(o)
}

fn read(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))
}

fn json<T: serde::de::DeserializeOwned>(text: &str) -> Result<T> {
    // Serde can quote invalid field content in its errors. Only expose location.
    serde_json::from_str(text).map_err(|e| {
        format!(
            "invalid JSON/schema at line {}, column {}",
            e.line(),
            e.column()
        )
    })
}

fn fields_from_parse(data: &mrz::MrzData) -> Fields {
    let extraction = synthpass_die::mrz_reader::extraction_v2_from_mrz(data);
    let mut fields: Fields = CoreField::ALL
        .iter()
        .map(|field| {
            (
                field.as_str().to_string(),
                extraction.fields.get(*field).map(str::to_string),
            )
        })
        .collect();
    fields.insert("mrz_line".into(), Some(data.mrz_lines.clone()));
    fields
}

fn fields_from_fixture(text: &str) -> Result<Fields> {
    let extraction: Extraction = json(text)?;
    let value = serde_json::to_value(extraction).map_err(|_| "serialize fixture fields")?;
    Ok(FIELD_NAMES
        .iter()
        .map(|key| (key.to_string(), value[key].as_str().map(str::to_string)))
        .collect())
}

fn fields_match_exported_value(exported: &Option<String>, parsed: &Option<String>) -> bool {
    // The review page serializes an empty text input as `""`, while the MRZ
    // extraction omits an empty field. They represent the same transcription;
    // retain `""` in the fixture because reviewed fixtures require strings.
    exported.as_deref().filter(|value| !value.is_empty())
        == parsed.as_deref().filter(|value| !value.is_empty())
}

fn shape(lines: &str) -> Result<Vec<&str>> {
    let mut lines: Vec<_> = lines.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }
    let width = match lines.len() {
        3 => 30,
        2 if lines[0].starts_with('P') => 44,
        2 if lines[0].starts_with('V') && lines[0].len() > 36 => 44,
        2 => 36,
        _ => return Err("mrz_line: expected two TD3/TD2/visa lines or three TD1 lines".into()),
    };
    for (i, line) in lines.iter().enumerate() {
        if !line
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'<')
        {
            return Err(format!(
                "mrz_line: line {} contains characters outside A-Z0-9<",
                i + 1
            ));
        }
        if line.len() != width {
            return Err(format!(
                "mrz_line: line {} has {} bytes; expected {width}",
                i + 1,
                line.len()
            ));
        }
    }
    Ok(lines)
}

fn structural(lines: &[&str]) -> std::result::Result<mrz::MrzData, mrz::MrzError> {
    match (lines.len(), lines[0].len(), lines[0].starts_with('V')) {
        (3, _, _) => mrz::parse_td1(lines[0], lines[1], lines[2]),
        (2, 44, true) => mrz::parse_mrv_a(lines[0], lines[1]),
        (2, 36, true) => mrz::parse_mrv_b(lines[0], lines[1]),
        (2, 44, false) => mrz::parse_td3(lines[0], lines[1]),
        _ => mrz::parse_td2(lines[0], lines[1]),
    }
}

fn safe_identity(entry: &Entry) -> Result<()> {
    if entry.stem.is_empty()
        || !entry
            .stem
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    {
        return Err("stem: expected only ASCII letters, digits, underscores and hyphens".into());
    }
    let asset = Path::new(&entry.asset);
    if entry.asset.contains('\\')
        || entry.asset.contains(':')
        || !asset
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        || asset.file_stem().and_then(|s| s.to_str()) != Some(&entry.stem)
    {
        return Err("asset: must be a relative samples path with the same stem".into());
    }
    Ok(())
}

fn validate(entry: &Entry) -> Result<Extraction> {
    safe_identity(entry)?;
    if entry.fields.len() != FIELD_NAMES.len()
        || FIELD_NAMES.iter().any(|f| !entry.fields.contains_key(*f))
    {
        return Err("fields: expected the ten core fields and mrz_line, with no extra keys".into());
    }
    let zone = entry.fields["mrz_line"]
        .as_deref()
        .ok_or("mrz_line: required")?;
    let lines = shape(zone)?;
    let exact = structural(&lines);
    let found = mrz::find_and_parse(zone);
    if entry.status == Status::Verified {
        let parsed = exact
            .as_ref()
            .map_err(|_| "mrz_line: printed zone does not parse structurally")?;
        if !parsed.valid() {
            return Err(format!(
                "mrz_line: printed check digits failed: {:?}",
                parsed.checks.failed()
            ));
        }
        let found = found
            .as_ref()
            .map_err(|_| "mrz_line: find_and_parse failed")?;
        if !found.valid() || found.mrz_lines != zone {
            return Err(
                "mrz_line: benchmark parser cannot validate the exact printed zone without repair"
                    .into(),
            );
        }
        let expected = fields_from_parse(parsed);
        for field in FIELD_NAMES {
            if !fields_match_exported_value(&entry.fields[field], &expected[field]) {
                return Err(format!(
                    "{field}: differs from the printed MRZ parse (no automatic correction)"
                ));
            }
        }
    } else {
        if exact.as_ref().is_ok_and(|d| d.valid()) {
            return Err(
                "mrz_line: non_conforming zone validates; use verified after checking all fields"
                    .into(),
            );
        }
        if found.as_ref().is_ok_and(|d| d.valid()) {
            return Err("mrz_line: benchmark parser repairs this printed zone to a valid read; cannot record a false checksum verdict until that parser behavior is resolved".into());
        }
        for name in ["surname", "given_names"] {
            let value = entry.fields[name]
                .as_deref()
                .ok_or_else(|| format!("{name}: MRZ-form name required"))?;
            if !value.bytes().all(|b| b.is_ascii_uppercase() || b == b' ') {
                return Err(format!("{name}: MRZ-form names use A-Z and spaces"));
            }
            if let Ok(data) = &exact {
                if entry.fields[name] != fields_from_parse(data)[name] {
                    return Err(format!("{name}: differs from the printed MRZ name field"));
                }
            }
        }
    }
    let mut value = serde_json::to_value(&entry.fields).map_err(|_| "serialize fields")?;
    value["extraction_method"] = "mrz-deterministic".into();
    value["mrz_checksums_valid"] = (entry.status == Status::Verified).into();
    serde_json::from_value(value).map_err(|_| "fields do not match Extraction schema".into())
}

fn fixture_json(extraction: &Extraction) -> Result<String> {
    // Value's sorted keys match the reviewed files; skip volatile/enriched metadata.
    let value = serde_json::to_value(extraction).map_err(|_| "serialize Extraction")?;
    serde_json::to_string_pretty(&value)
        .map(|s| s + "\n")
        .map_err(|_| "serialize fixture".into())
}

fn no_link(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            Err(format!("refusing symlink: {}", path.display()))
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("inspect {}: {e}", path.display())),
    }
}

fn planned_files(
    fixtures: &Path,
    entry: &Entry,
    extraction: &Extraction,
) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    no_link(fixtures)?;
    no_link(&fixtures.join("derived"))?;
    let output = fixtures.join(format!("{}.json", entry.stem));
    no_link(&output)?;
    let mut extraction = extraction.clone();
    if output.exists() {
        let existing: Extraction = json(&read(&output)?).map_err(|_| {
            "existing reviewed fixture is not valid Extraction JSON; refusing overwrite"
        })?;
        // Historical reviewed fixtures use both methods. Preserve provenance,
        // but compare all transcribed fields and the verdict byte for byte.
        if matches!(
            existing.extraction_method.as_str(),
            "mrz-deterministic" | "hand-transcribed"
        ) {
            extraction
                .extraction_method
                .clone_from(&existing.extraction_method);
        }
    }
    let sidecar = output.with_extension("md");
    let candidate = fixtures.join("derived").join(format!("{}.md", entry.stem));
    no_link(&candidate.with_extension("json"))?;
    for path in [&output, &sidecar, &candidate] {
        no_link(path)?;
    }
    let markdown = if sidecar.exists() {
        read(&sidecar)?
    } else if candidate.exists() {
        read(&candidate)?
    } else {
        format!(
            "{}\n",
            extraction.mrz_line.as_deref().ok_or("mrz_line required")?
        )
    };
    let mut serialized = fixture_json(&extraction)?;
    // Git may check reviewed files out as CRLF on Windows. Keep their exact
    // checkout bytes for idempotency; new fixtures use canonical repository LF.
    if output.exists() && read(&output)?.contains("\r\n") {
        serialized = serialized.replace('\n', "\r\n");
    }
    let files = vec![
        (output, serialized.into_bytes()),
        (sidecar, markdown.into_bytes()),
    ];
    for (path, body) in &files {
        if path.exists() && fs::read(path).map_err(|e| e.to_string())? != *body {
            return Err(format!(
                "reviewed {} differs; refusing overwrite",
                path.display()
            ));
        }
    }
    Ok(files)
}

fn promote(fixtures: &Path, entry: &Entry, files: &[(PathBuf, Vec<u8>)]) -> Result<()> {
    fs::create_dir_all(fixtures).map_err(|e| e.to_string())?;
    let mut created = Vec::new();
    let result: Result<()> = (|| {
        for (path, body) in files {
            if path.exists() {
                no_link(path)?;
                if fs::read(path).map_err(|e| e.to_string())? != *body {
                    return Err("reviewed file changed since validation; refusing overwrite".into());
                }
                continue;
            }
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map_err(|e| e.to_string())?;
            created.push(path);
            file.write_all(body)
                .and_then(|()| file.sync_all())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    })();
    if result.is_err() {
        for path in created {
            let _ = fs::remove_file(path);
        }
        return result;
    }
    for ext in ["json", "md"] {
        let path = fixtures
            .join("derived")
            .join(format!("{}.{ext}", entry.stem));
        no_link(&path)?;
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(format!(
                    "reviewed pair written; could not remove {}: {e}",
                    path.display()
                ))
            }
        }
    }
    Ok(())
}

fn report_nonconforming(o: &Options, entry: &Entry) {
    match manifest(o) {
        Ok(rows) => match rows.iter().find(|row| row.asset() == entry.asset) {
            Some(row) => {
                if row.ground_truth_stem.as_deref() != Some(&entry.stem) {
                    println!("  Manifest ground_truth_stem needs regeneration after promotion.");
                }
                if row.expected_document_number.is_some() {
                    println!("  Manifest expected_document_number must be null: null is the recorded answer for a non-conforming zone. Regenerate after promotion.");
                }
                if row.mrz["present"].as_bool() != Some(true) {
                    println!("  Manifest mrz.present is not true: human review of that claim is still required.");
                }
                if row.mrz["redacted"].as_bool() != Some(false) || entry.stem.to_ascii_lowercase().contains("redacted") {
                    println!("  Redaction must be resolved separately: the benchmark gives a redacted filename priority over checksum_failed_specimen.");
                }
            }
            None => println!("  Asset has no manifest row; add it through manifest regeneration and review its claims."),
        },
        Err(_) => println!("  Manifest unavailable: check the fixture link, expected number, presence and redaction claims in the image checkout."),
    }
}
fn apply(o: &Options) -> Result<bool> {
    let input = o.input.as_ref().ok_or("apply needs an export")?;
    let entries: Vec<Entry> = json(&read(input)?)?;
    let mut counts = BTreeMap::new();
    for entry in &entries {
        if entry.status != Status::Skip {
            *counts
                .entry(entry.stem.to_ascii_lowercase())
                .or_insert(0usize) += 1;
        }
    }
    let (mut accepted, mut rejected, mut skipped) = (0, 0, 0);
    println!("entry\tresult\tdetail");
    for (index, entry) in entries.iter().enumerate() {
        if entry.status == Status::Skip {
            skipped += 1;
            println!("{}\tskip\tignored", index + 1);
            continue;
        }
        let result: Result<()> = (|| {
            if counts[&entry.stem.to_ascii_lowercase()] > 1 {
                return Err("duplicate stem in export".into());
            }
            let extraction = validate(entry)?;
            let files = planned_files(&o.fixtures, entry, &extraction)?;
            if o.write {
                promote(&o.fixtures, entry, &files)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                accepted += 1;
                println!(
                    "{}\taccepted\t{}",
                    index + 1,
                    if o.write {
                        "written / identical"
                    } else {
                        "dry run"
                    }
                );
                if entry.status == Status::NonConforming {
                    println!("  checksum_failed_specimen requires the reviewed fixture under the image stem, mrz_checksums_valid=false, manifest mrz.present=true and mrz.redacted=false. Regenerate ground_truth_stem and expected_document_number=null: null is the recorded answer for a non-conforming zone; review attribution and re-bless the gate separately.");
                    report_nonconforming(o, entry);
                    let zone = entry.fields["mrz_line"]
                        .as_deref()
                        .ok_or("mrz_line required")?;
                    if structural(&shape(zone)?).is_err() {
                        println!("  Printed zone does not parse structurally: names remain a human assertion; checksum failure cannot be localized.");
                    }
                }
            }
            Err(reason) => {
                rejected += 1;
                println!("{}\trejected\t{reason}", index + 1);
            }
        }
    }
    println!("\naccepted\trejected\tskipped\n{accepted}\t{rejected}\t{skipped}");
    println!("\nNext, after applying to the reviewed fixtures in the image checkout:\ncargo run --release -p synthpass-ocr --example corpus_manifest\ncargo test -p synthpass-bench --test ocr_fixtures");
    println!("corpus_manifest reads its checkout's samples/ocr_fixtures; copy reviewed changes there first when using separate image and fixture roots. No manifest was edited.");
    Ok(rejected == 0)
}

#[derive(Deserialize)]
struct ManifestRow {
    dir: String,
    filename: String,
    #[serde(default)]
    provenance: String,
    mrz: Value,
    #[serde(default)]
    ground_truth_stem: Option<String>,
    #[serde(default)]
    expected_document_number: Option<String>,
}

impl ManifestRow {
    fn asset(&self) -> String {
        format!("{}/{}", self.dir, self.filename)
    }
    fn stem(&self) -> Result<String> {
        Path::new(&self.filename)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(str::to_string)
            .ok_or_else(|| "manifest filename has no stem".into())
    }
}

struct Card {
    stem: String,
    asset: String,
    class: String,
    image: String,
    crop: Option<String>,
    note: String,
    ocr: String,
    fields: Fields,
}

struct SelectedCard {
    index: usize,
    fields: Option<Fields>,
    already_reviewed: bool,
}

fn manifest(o: &Options) -> Result<Vec<ManifestRow>> {
    // Labels and the manifest are revision-local; the image root may be another checkout.
    let local = o
        .fixtures
        .parent()
        .unwrap_or(Path::new("."))
        .join("corpus.jsonl");
    let path = if local.exists() {
        local
    } else {
        o.samples.join("corpus.jsonl")
    };
    read(&path)?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(json)
        .collect()
}

fn reviewed_fields(fixtures: &Path, stem: &str) -> Result<Option<Fields>> {
    let fixture = fixtures.join(format!("{stem}.json"));
    if fixture.is_file() {
        fields_from_fixture(&read(&fixture)?).map(Some)
    } else {
        Ok(None)
    }
}

fn selection(o: &Options, rows: &[ManifestRow]) -> Result<Vec<SelectedCard>> {
    let mut selected = BTreeMap::new();
    if o.batch != "b" {
        for asset in BATCH_A {
            let index = rows
                .iter()
                .position(|r| r.asset() == asset)
                .ok_or_else(|| format!("Batch A asset absent from manifest: {asset}"))?;
            let stem = rows[index].stem()?;
            let fields = reviewed_fields(&o.fixtures, &stem)?;
            selected.insert(
                stem,
                SelectedCard {
                    index,
                    already_reviewed: fields.is_some(),
                    fields,
                },
            );
        }
    }
    if o.batch != "a" {
        let mut paths: Vec<_> = fs::read_dir(o.fixtures.join("derived"))
            .map_err(|e| e.to_string())?
            .map(|entry| entry.map(|e| e.path()))
            .collect::<std::io::Result<_>>()
            .map_err(|e| e.to_string())?;
        paths.sort();
        for path in paths {
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .ok_or("candidate has no stem")?;
            // Some manifest stems have identical-content image variants. Sorted asset order
            // deterministically selects one image; there is still exactly one card per fixture.
            let index = rows
                .iter()
                .enumerate()
                .filter(|(_, row)| row.stem().is_ok_and(|s| s == stem))
                .min_by_key(|(_, row)| row.asset())
                .map(|(i, _)| i)
                .ok_or_else(|| format!("candidate absent from manifest: {stem}"))?;
            selected.insert(
                stem.to_string(),
                SelectedCard {
                    index,
                    fields: Some(fields_from_fixture(&read(&path)?)?),
                    already_reviewed: false,
                },
            );
        }
    }
    Ok(selected.into_values().collect())
}
fn load_ocr(o: &Options) -> (Option<NativeOcr>, String) {
    if o.no_ocr {
        return (None, "OCR disabled (--no-ocr); showing full image.".into());
    }
    let mut dirs = Vec::new();
    if let Some(dir) = std::env::var_os("SYNTHPASS_OCR_MODEL_DIR") {
        dirs.push(PathBuf::from(dir));
    }
    dirs.push(PathBuf::from("."));
    if let Some(parent) = o.samples.parent() {
        dirs.push(parent.to_path_buf());
    }
    for dir in dirs {
        let detection = dir.join("text-detection.rten");
        let recognition = dir.join("text-recognition.rten");
        if detection.is_file() && recognition.is_file() {
            return match NativeOcr::load(&detection, &recognition) {
                Ok(ocr) => (Some(ocr), String::new()),
                Err(_) => (
                    None,
                    "OCR models could not be loaded; showing full image.".into(),
                ),
            };
        }
    }
    (
        None,
        "OCR models missing; showing full image. Set SYNTHPASS_OCR_MODEL_DIR to local models."
            .into(),
    )
}

fn embedded(image: &DynamicImage, enlarge: bool) -> Result<String> {
    if enlarge {
        // Preserve original crop pixels losslessly; enlarge only for display.
        // Keep color rather than risk losing faint strokes in grayscale conversion.
        let mut bytes = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut bytes, image::ImageFormat::Png)
            .map_err(|_| "encode MRZ crop")?;
        return Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(bytes.into_inner())
        ));
    }
    if image.width() == 0 || image.height() == 0 {
        return Err("image has zero size".into());
    }
    // Full-page context yields the byte budget to the lossless MRZ crop.
    let resized = if image.width().max(image.height()) > 1000 {
        image
            .resize(1000, 1000, image::imageops::FilterType::Lanczos3)
            .to_rgb8()
    } else {
        image.to_rgb8()
    };
    let mut bytes = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 85)
        .encode_image(&resized)
        .map_err(|_| "encode review image")?;
    Ok(format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(bytes)
    ))
}

fn band_crop(
    image: &DynamicImage,
    page: &synthpass_ocr::geometry::OcrPage,
) -> Option<DynamicImage> {
    let band = page.mrz_band?;
    let image = match page.rotation {
        90 => image.rotate90(),
        180 => image.rotate180(),
        270 => image.rotate270(),
        _ => image.clone(),
    };
    let x = (band.x.floor().max(0.0) as u32).min(image.width());
    let y = (band.y.floor().max(0.0) as u32).min(image.height());
    let right = ((band.x + band.w).ceil().max(0.0) as u32).min(image.width());
    let bottom = ((band.y + band.h).ceil().max(0.0) as u32).min(image.height());
    (right > x && bottom > y).then(|| image.crop_imm(x, y, right - x, bottom - y))
}

fn context_for_budget(image: DynamicImage, edge: u32, preserve_full_image: bool) -> DynamicImage {
    if preserve_full_image || image.width().max(image.height()) <= edge {
        image
    } else {
        image.resize(edge, edge, image::imageops::FilterType::Lanczos3)
    }
}

fn mrz_shaped(text: &str) -> String {
    text.lines()
        .filter(|line| {
            let count = line.chars().count();
            count >= 20
                && line.contains('<')
                && line
                    .chars()
                    .filter(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || *c == '<')
                    .count()
                    * 5
                    >= count * 4
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn output_path(out: &Path) -> Result<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("workspace root unavailable")?
        .to_path_buf();
    let absolute = if out.is_absolute() {
        out.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| e.to_string())?
            .join(out)
    };
    let relative = absolute
        .strip_prefix(&root)
        .map_err(|_| "review output must be under this worktree's artifacts/")?;
    let mut components = relative.components();
    if components.next() != Some(Component::Normal(std::ffi::OsStr::new("artifacts")))
        || !components
            .clone()
            .all(|c| matches!(c, Component::Normal(_)))
        || relative.extension().and_then(|e| e.to_str()) != Some("html")
    {
        return Err(
            "review output must be an .html file under artifacts/, with no parent traversal".into(),
        );
    }
    let mut ancestor = root.clone();
    for component in relative.components() {
        ancestor.push(component);
        no_link(&ancestor)?;
    }
    let ignored = Command::new("git")
        .current_dir(&root)
        .args(["check-ignore", "--quiet", "--"])
        .arg(relative)
        .status()
        .map_err(|e| format!("verify gitignore: {e}"))?;
    if !ignored.success() {
        return Err(
            "review output is not gitignored (or is tracked); refusing specimen HTML".into(),
        );
    }
    let destination = root.join(relative);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Ok(destination)
}

fn review(o: &Options) -> Result<()> {
    let started = std::time::Instant::now();
    let out = output_path(&o.out)?;
    let rows = manifest(o)?;
    let selected = selection(o, &rows)?;
    let (ocr, fallback) = load_ocr(o);
    let mut cards = Vec::new();
    for selected in selected {
        let row = &rows[selected.index];
        let asset = row.asset();
        let stem = row.stem()?;
        safe_identity(&Entry {
            stem: stem.clone(),
            asset: asset.clone(),
            status: Status::Skip,
            fields: Fields::new(),
        })?;
        let path = o.samples.join(&asset);
        let image = synthpass_ocr::decode_image(&path)
            .map_err(|_| format!("cannot decode specimen: {asset}"))?;
        let mut fields = selected.fields.unwrap_or_default();
        let (mut note, mut crop, mut observed) = if selected.already_reviewed {
            (
                "Already reviewed fixture; prefilled from it and left at skip.".into(),
                None,
                String::new(),
            )
        } else {
            (fallback.clone(), None, String::new())
        };
        if let Some(ocr) = &ocr {
            match ocr.recognize_detailed(&path) {
                Ok(page) => {
                    observed = mrz_shaped(&page.text);
                    crop = band_crop(&image, &page)
                        .map(|im| embedded(&im, true))
                        .transpose()?;
                    if crop.is_none() && !selected.already_reviewed {
                        note = "No MRZ band detected; showing full image.".into();
                    }
                    if fields.is_empty() {
                        // Only exact structural OCR lines are prefilled. No repaired OCR
                        // output is allowed to masquerade as the printed transcription.
                        if let Ok(lines) = shape(&observed) {
                            if let Ok(data) = structural(&lines) {
                                fields = fields_from_parse(&data);
                            }
                        }
                        fields.insert("mrz_line".into(), Some(observed.clone()));
                    }
                }
                Err(_) if !selected.already_reviewed => {
                    note = "OCR failed; showing full image.".into()
                }
                Err(_) => {}
            }
        }
        let class = format!(
            "{:?} / {} / MRZ {} / {}",
            crate::classify_specimen(Path::new(&asset), None),
            row.provenance,
            if row.mrz["present"].as_bool() == Some(true) {
                "present"
            } else {
                "absent"
            },
            row.mrz["observed"]["format"]
                .as_str()
                .unwrap_or("format unrecorded")
        );
        cards.push(Card {
            stem,
            asset,
            class,
            image: embedded(&image, false)?,
            crop,
            note,
            ocr: observed,
            fields,
        });
    }
    let mut html = render(&cards);
    let mut context_edge = 1000u32;
    // The crop is review evidence: only context images yield to the page budget.
    while html.len() > 15_000_000 && context_edge > 320 {
        context_edge = (context_edge * 4 / 5).max(320);
        for card in &mut cards {
            let image = synthpass_ocr::decode_image(&o.samples.join(&card.asset))
                .map_err(|_| format!("cannot decode specimen: {}", card.asset))?;
            let context = context_for_budget(image, context_edge, card.crop.is_none());

            card.image = embedded(&context, false)?;
        }
        html = render(&cards);
    }
    if context_edge < 1000 {
        println!(
            "Context images capped at {context_edge}px to reserve space for native PNG crops."
        );
    }
    if html.len() > 15_000_000 {
        println!("Native crops keep this page above 15 MB; their pixels have been preserved.");
    }
    fs::write(&out, &html).map_err(|e| format!("write review: {e}"))?;
    println!(
        "{} cards; {} bytes; {:.2}s; {} (gitignored)",
        cards.len(),
        html.len(),
        started.elapsed().as_secs_f64(),
        o.out.display()
    );
    Ok(())
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render(cards: &[Card]) -> String {
    let mut html = String::from(
        r#"<!doctype html>
<html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'none'; form-action 'none'; base-uri 'none'">
<title>Ground truth review</title><style>
body{font:16px system-ui;background:#edf1f5;color:#182838;margin:0}header,main{max-width:1200px;margin:auto;padding:20px}header{position:sticky;top:0;background:#edf1f5;z-index:1}article{background:white;padding:24px;margin-bottom:24px;border-radius:12px;border:1px solid #b9c8d6}img{max-width:100%;height:auto}pre,textarea{font:15px monospace;white-space:pre-wrap;overflow-wrap:anywhere}textarea{width:100%;box-sizing:border-box;min-height:90px}.fields{display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:12px}label{display:block}input{display:block;box-sizing:border-box;width:100%;padding:8px}button,select{padding:10px}h2{overflow-wrap:anywhere}.checks{font-family:monospace;color:#824300}.note{color:#824300}.crop-controls{display:flex;gap:20px;align-items:center;flex-wrap:wrap}.crop-controls input{display:inline-block;width:auto;padding:0}.crop-controls input[type=range]{width:180px}.crop-viewport{overflow:auto;max-height:70vh}.mrz-crop{display:block;width:100%;max-width:none;filter:grayscale(var(--gray,0)) contrast(var(--contrast,1)) invert(var(--invert,0))}.mrz-crop.high-contrast{--gray:1;--contrast:1.6}.mrz-crop.inverted{--invert:1}
</style><header><h1>Ground truth review</h1><p>Compare every field with the printed MRZ. Names use MRZ spelling, with filler read as spaces. OCR is unverified. Each card starts at skip.</p><button id="export" type="button">Export ground-truth-verified.json</button><span id="export-status" role="status"></span></header><main>
"#,
    );
    for card in cards {
        html.push_str(&format!("<article data-stem=\"{}\" data-asset=\"{}\"><h2>{}</h2><p>{}</p><p>Class: {}</p><img alt=\"Specimen\" src=\"{}\"><p class=\"note\">{}</p>",
            escape(&card.stem), escape(&card.asset), escape(&card.stem), escape(&card.asset), escape(&card.class), escape(&card.image), escape(&card.note)));
        if let Some(crop) = &card.crop {
            html.push_str(&format!(
                "<h3>Enlarged MRZ band</h3><div class=\"crop-controls\"><label><input class=\"crop-contrast\" type=\"checkbox\"> High contrast</label><label><input class=\"crop-invert\" type=\"checkbox\"> Invert</label><label>Zoom (100% fits card) <input class=\"crop-zoom\" type=\"range\" min=\"100\" max=\"400\" step=\"25\" value=\"100\"> <output>100%</output></label></div><div class=\"crop-viewport\" tabindex=\"0\" role=\"region\" aria-label=\"Scrollable MRZ crop\"><img class=\"mrz-crop\" alt=\"MRZ band\" src=\"{}\"></div>",
                escape(crop)
            ));
        }
        html.push_str(&format!(
            "<h3>OCR read — unverified</h3><pre>{}</pre><div class=\"fields\">",
            escape(&card.ocr)
        ));
        for key in &FIELD_NAMES[..10] {
            let value = card.fields.get(*key).and_then(|v| v.as_deref());
            html.push_str(&format!(
                "<label>{}<input data-field=\"{}\" value=\"{}\"></label>",
                key,
                key,
                escape(value.unwrap_or(""))
            ));
        }
        html.push_str(&format!("</div><label>Printed MRZ lines<textarea data-field=\"mrz_line\" spellcheck=\"false\">{}</textarea></label><p class=\"checks\" aria-live=\"polite\"></p><label>Status <select><option value=\"skip\">skip</option><option value=\"verified\">verified</option><option value=\"non_conforming\">non_conforming</option></select></label><p>non_conforming: transcribed faithfully, but the printed zone fails its own check digits.</p></article>\n",
            escape(card.fields.get("mrz_line").and_then(|v| v.as_deref()).unwrap_or(""))));
    }
    html.push_str(r#"</main><script>
'use strict';
const cards = [...document.querySelectorAll('article')];
function check(card) {
  const lines = card.querySelector('textarea').value.split('\n');
  if (lines.at(-1) === '') lines.pop();
  const first = lines[0];
  const width = lines.length === 3 ? 30 : first.startsWith('P') || (first.startsWith('V') && first.length > 36) ? 44 : 36;
  const report = lines.map((line,i) => `Line ${i+1}: ${line.length}/${width}; ${/^[A-Z0-9<]+$/.test(line) ? 'characters OK' : 'invalid characters'}`);
  if (lines.length !== 2 && lines.length !== 3) report.push('Expected 2 or 3 lines');
  card.querySelector('.checks').textContent = report.join(' | ');
}
for (const card of cards) {
  card.addEventListener('input', () => check(card)); check(card);
  const crop = card.querySelector('.mrz-crop');
  if (crop) {
    card.querySelector('.crop-contrast').addEventListener('change', event => {
      crop.classList.toggle('high-contrast', event.target.checked);
    });
    card.querySelector('.crop-invert').addEventListener('change', event => {
      crop.classList.toggle('inverted', event.target.checked);
    });
    card.querySelector('.crop-zoom').addEventListener('input', event => {
      crop.style.width = `${event.target.value}%`;
      card.querySelector('.crop-controls output').value = `${event.target.value}%`;
    });
  }
}
document.querySelector('#export').addEventListener('click', () => {
  const entries = cards.map(card => ({stem:card.dataset.stem, asset:card.dataset.asset, status:card.querySelector('select').value,
    fields:Object.fromEntries([...card.querySelectorAll('[data-field]')].map(input => [input.dataset.field,
      input.value === '' && input.dataset.field === 'personal_number' ? null : input.value]))}));
  const url = URL.createObjectURL(new Blob([JSON.stringify(entries,null,2)+'\n'],{type:'application/json'}));
  const a = document.createElement('a'); a.href = url; a.download = 'ground-truth-verified.json'; a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  document.querySelector('#export-status').textContent = ` Exported ${entries.length} cards. Validate with ground-truth apply.`;
});
</script></html>
"#);
    html
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    const TD3: &str = "P<UTOERIKSSON<<ANNA<MARIA<<<<<<<<<<<<<<<<<<<\nL898902C36UTO7408122F1204159ZE184226B<<<<<10";
    const TD1: &str = "I<UTOD231458907<<<<<<<<<<<<<<<\n7408122F1204159UTO<<<<<<<<<<<6\nERIKSSON<<ANNA<MARIA<<<<<<<<<<";

    fn entry(zone: &str) -> Entry {
        Entry {
            stem: "test".into(),
            asset: "passports/test.png".into(),
            status: Status::Verified,
            fields: fields_from_parse(&structural(&shape(zone).unwrap()).unwrap()),
        }
    }

    fn bad_digit() -> Entry {
        let mut entry = entry(TD3);
        let mut zone = TD3.to_string();
        zone.pop();
        zone.push('1');
        entry.fields.insert("mrz_line".into(), Some(zone));
        entry
    }

    #[test]
    fn accepts_valid_td3_and_td1() {
        assert!(validate(&entry(TD3)).is_ok());
        assert!(validate(&entry(TD1)).is_ok());
    }

    #[test]
    fn rejects_wrong_check_digit_and_valid_non_conforming() {
        assert!(validate(&bad_digit())
            .unwrap_err()
            .contains("check digits failed"));
        let mut entry = entry(TD3);
        entry.status = Status::NonConforming;
        assert!(validate(&entry).unwrap_err().contains("zone validates"));
    }

    #[test]
    fn rejects_name_mismatch_and_45_character_line() {
        let mut entry = entry(TD3);
        entry
            .fields
            .insert("given_names".into(), Some("WRONG".into()));
        assert!(validate(&entry).unwrap_err().starts_with("given_names:"));
        entry
            .fields
            .insert("mrz_line".into(), Some(TD3.replacen('\n', "<\n", 1)));
        assert!(validate(&entry)
            .unwrap_err()
            .contains("45 bytes; expected 44"));
    }

    #[test]
    fn non_conforming_names_still_match_the_printed_field() {
        let mut entry = bad_digit();
        entry.status = Status::NonConforming;
        assert!(validate(&entry).is_ok());
        entry.fields.insert("surname".into(), Some("WRONG".into()));
        assert!(validate(&entry).unwrap_err().starts_with("surname:"));
    }

    #[test]
    fn refuses_repairs_charset_and_path_traversal() {
        let mut entry = entry(TD3);
        entry
            .fields
            .insert("mrz_line".into(), Some(TD3.replace('0', "O")));
        assert!(validate(&entry).is_err());
        entry
            .fields
            .insert("mrz_line".into(), Some(TD3.replacen('<', "&", 1)));
        assert!(validate(&entry).unwrap_err().contains("characters outside"));
        entry.stem = "../escape".into();
        assert!(validate(&entry).unwrap_err().starts_with("stem:"));
    }

    #[test]
    fn accepts_empty_exported_batch_b_field_and_writes_a_string() {
        let mut fields = fields_from_fixture(include_str!(
            "../../../samples/ocr_fixtures/derived/Kosovo_Passport_Specimen_P0_RKS_2013_mrz.json"
        ))
        .unwrap();
        assert_eq!(fields["given_names"], None);
        fields.insert("given_names".into(), Some(String::new()));
        let entry = Entry {
            stem: "Kosovo_Passport_Specimen_P0_RKS_2013_mrz".into(),
            asset: "passports/Kosovo_Passport_Specimen_P0_RKS_2013_mrz.jpg".into(),
            status: Status::Verified,
            fields,
        };
        let extraction = validate(&entry).unwrap();
        let dir = temp_dir();
        let files = planned_files(&dir, &entry, &extraction).unwrap();
        promote(&dir, &entry, &files).unwrap();
        let written: Value =
            json(&read(&dir.join(format!("{}.json", entry.stem))).unwrap()).unwrap();
        assert_eq!(written["given_names"], "");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn shape_trims_one_trailing_empty_line_and_reports_charset_first() {
        assert!(shape(&format!("{TD3}\n")).is_ok());
        let non_ascii = TD3.replacen('A', "é", 1);
        assert!(shape(&non_ascii)
            .unwrap_err()
            .contains("characters outside A-Z0-9<"));
    }

    #[test]
    fn fallback_context_is_never_shrunk_for_the_page_budget() {
        let image = DynamicImage::new_rgb8(1600, 900);
        assert_eq!(context_for_budget(image.clone(), 409, true).width(), 1600);
        assert_eq!(context_for_budget(image, 409, false).width(), 409);
    }

    #[test]
    fn reviewed_fixture_fields_are_available_for_batch_a_cards() {
        let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/ocr_fixtures");
        let fields = reviewed_fields(&fixtures, "Afghanistan_Passport_Specimen_P0_AFG_2016_mrz")
            .unwrap()
            .unwrap();
        assert!(fields["mrz_line"].is_some());
    }
    #[test]
    fn serializer_matches_reviewed_fixture_bytes() {
        let expected = r#"{
  "date_of_birth": "1985-01-01",
  "date_of_expiry": "2023-01-14",
  "document_number": "ZE001355",
  "document_type": "P",
  "extraction_method": "mrz-deterministic",
  "given_names": "SARAH",
  "issuing_country": "CAN",
  "mrz_checksums_valid": true,
  "mrz_line": "P<CANMARTIN<<SARAH<<<<<<<<<<<<<<<<<<<<<<<<<<\nZE001355<3CAN8501019F2301147<<<<<<<<<<<<<<00",
  "nationality": "CAN",
  "personal_number": null,
  "sex": "F",
  "surname": "MARTIN"
}
"#;
        let real = include_str!(
            "../../../samples/ocr_fixtures/Canada_Passport_Specimen_P0_CAN_2013_mrz.json"
        );
        assert_eq!(expected, real.replace("\r\n", "\n"));
        let extraction: Extraction = json(expected).unwrap();
        assert_eq!(fixture_json(&extraction).unwrap(), expected);
    }

    #[test]
    fn html_has_one_card_per_input_and_escapes_all_values() {
        let card = || Card {
            stem: "<stem>&\"".into(),
            asset: "<asset>&".into(),
            class: "<&>".into(),
            image: embedded(&DynamicImage::new_rgb8(2, 2), false).unwrap(),
            crop: None,
            note: "<note>&".into(),
            ocr: "P<UTO&".into(),
            fields: entry(TD3).fields,
        };
        let html = render(&[card(), card()]);
        assert_eq!(html.matches("<article ").count(), 2);
        assert!(html.contains("&lt;stem&gt;&amp;&quot;"));
        assert!(html.contains("P&lt;UTO&amp;"));
        assert!(!html.contains("P<UTO"));
        assert!(html.contains("<option value=\"skip\">skip</option>"));
    }

    #[test]
    fn batch_a_has_thirteen_unique_assets() {
        assert_eq!(BATCH_A.len(), 13);
        assert_eq!(BATCH_A.iter().collect::<BTreeSet<_>>().len(), 13);
        let rows: Vec<ManifestRow> = include_str!("../../../samples/corpus.jsonl")
            .lines()
            .map(|l| json(l).unwrap())
            .collect();
        let samples = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples");
        // Frozen membership must still resolve even as OCR outcomes improve.
        for asset in &BATCH_A {
            assert!(
                samples.join(asset).is_file() || rows.iter().any(|r| r.asset() == *asset),
                "Batch A path no longer resolves: {asset}"
            );
        }
    }

    fn temp_dir() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("ground-truth-test-{}-{nonce}", std::process::id()));
        fs::create_dir_all(path.join("derived")).unwrap();
        path
    }

    #[test]
    fn promotion_preserves_ocr_sidecar_and_refuses_conflicts() {
        let dir = temp_dir();
        let entry = entry(TD3);
        let extraction = validate(&entry).unwrap();
        fs::write(dir.join("derived/test.md"), "OCR read\n").unwrap();
        fs::write(dir.join("derived/test.json"), "candidate\n").unwrap();
        let planned = planned_files(&dir, &entry, &extraction).unwrap();
        assert!(!dir.join("test.json").exists()); // Planning is dry run.
        promote(&dir, &entry, &planned).unwrap();
        assert_eq!(read(&dir.join("test.md")).unwrap(), "OCR read\n");
        assert!(!dir.join("derived/test.json").exists());
        assert!(!dir.join("derived/test.md").exists());
        assert!(planned_files(&dir, &entry, &extraction).is_ok());
        fs::write(dir.join("test.json"), "existing review\n").unwrap();
        assert!(planned_files(&dir, &entry, &extraction)
            .unwrap_err()
            .contains("refusing overwrite"));
        assert_eq!(read(&dir.join("test.json")).unwrap(), "existing review\n");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn crop_uses_rotated_image_coordinates() {
        let image = DynamicImage::new_rgb8(4, 8);
        let page = synthpass_ocr::geometry::OcrPage {
            rotation: 90,
            mrz_band: Some(synthpass_ocr::geometry::BBox {
                x: 4.0,
                y: 1.0,
                w: 3.0,
                h: 2.0,
            }),
            ..Default::default()
        };
        let crop = band_crop(&image, &page).unwrap();
        assert_eq!((crop.width(), crop.height()), (3, 2));
    }

    #[test]
    fn cli_rejects_cross_command_flags() {
        assert!(options(vec!["review".into(), "--write".into()]).is_err());
        assert!(options(vec!["apply".into(), "test.json".into(), "--no-ocr".into()]).is_err());
        assert!(options(vec![
            "--samples-root".into(),
            "x".into(),
            "review".into(),
            "--batch".into(),
            "a".into()
        ])
        .is_ok());
    }
    #[test]
    fn reviewed_crlf_and_historical_method_are_idempotent() {
        let dir = temp_dir();
        let entry = entry(TD3);
        let extraction = validate(&entry).unwrap();
        let mut historical = extraction.clone();
        historical.extraction_method = "hand-transcribed".into();
        let original = fixture_json(&historical).unwrap().replace('\n', "\r\n");
        fs::write(dir.join("test.json"), &original).unwrap();
        let files = planned_files(&dir, &entry, &extraction).unwrap();
        promote(&dir, &entry, &files).unwrap();
        assert_eq!(
            fs::read(dir.join("test.json")).unwrap(),
            original.as_bytes()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn refuses_html_outside_ignored_artifacts() {
        assert!(output_path(Path::new("samples/review.html")).is_err());
        assert!(output_path(Path::new("artifacts/../review.html")).is_err());
        assert!(output_path(Path::new("artifacts/review.txt")).is_err());
    }
    #[test]
    fn distinguishes_parser_repair_from_a_valid_printed_zone() {
        let zone = TD3.replacen("740812", "74O812", 1);
        assert!(!structural(&shape(&zone).unwrap()).unwrap().valid());
        assert!(mrz::find_and_parse(&zone).unwrap().valid());
        let mut entry = entry(TD3);
        entry.status = Status::NonConforming;
        entry.fields.insert("mrz_line".into(), Some(zone));
        assert!(validate(&entry)
            .unwrap_err()
            .contains("benchmark parser repairs"));
    }
}
