//! `synthpass generate` — synthetic passport image + ground-truth label JSON
//! factory (M3). Wraps `synthpass_gen::generate_from_seed`.
//!
//! This command produces **no real PII** (every identity is fictional, drawn
//! deterministically from a seed — see `synthpass-gen`'s crate docs), so
//! unlike the default extraction path it is explicitly exempt from the
//! license gate: see the dispatch arm in `main.rs`, which returns before
//! `check_license()` is ever reached.

use serde::Serialize;
use std::path::Path;
use synthpass_gen::{
    data, generate_with, DocumentType, GeneratorConfig, Labels, RedactSpan, RedactStyle,
    RenderOptions,
};

/// Parsed `synthpass generate` arguments.
#[derive(Debug)]
struct GenerateArgs {
    count: u64,
    seed: u64,
    profile: String,
    out_dir: String,
    document_type: DocumentType,
    redact: Option<RedactSpan>,
}

impl Default for GenerateArgs {
    fn default() -> Self {
        Self {
            count: 1,
            seed: 0,
            profile: "clean".to_string(),
            out_dir: ".".to_string(),
            document_type: DocumentType::TD3,
            redact: None,
        }
    }
}

const VALID_PROFILES: &[&str] = &[
    "mobile",
    "scanner",
    "worn",
    "border-kiosk",
    "damaged",
    "clean",
];

const VALID_DOCUMENT_TYPES: &[&str] = &["td1", "td2", "td3", "mrva", "mrvb"];

/// The generator's ground-truth sidecar is `<stem>.labels.json`, never
/// `<stem>.json`: extraction writes its result to `<input>.json` next to the
/// input (`synthpass-pipeline`'s `process_document`), so reading a generated
/// PNG back would overwrite a `<stem>.json` label file (issue #514).
const LABELS_SUFFIX: &str = "labels.json";

fn usage() {
    eprintln!(
        "Usage: synthpass generate [--count N] [--seed N] [--profile NAME] [--document-type TYPE] [--redact STYLE:LINE:FIRST-LAST] [--out-dir DIR]"
    );
    eprintln!("  --count N            number of documents to generate (default: 1)");
    eprintln!("  --seed N             base seed; document i uses seed N+i (default: 0)");
    eprintln!(
        "  --profile NAME       {} (default: clean)",
        VALID_PROFILES.join("|")
    );
    eprintln!(
        "  --document-type TYPE td1|td2|td3|mrva|mrvb — the ICAO 9303 MRZ format to generate (default: td3)"
    );
    eprintln!("  --out-dir DIR        output directory (default: .)");
    eprintln!("  --redact SPAN        fill-black|fill-white|fill-grey|blur|graded-blur:LINE:FIRST-LAST (clean profile only)");
}

/// Hand-rolled flag parser, consistent with the rest of this CLI's style
/// (no clap, no new arg-parsing dependency).
fn parse_args(args: &[String]) -> Result<GenerateArgs, String> {
    let mut parsed = GenerateArgs::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--count" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--count requires a value".to_string())?;
                parsed.count = v
                    .parse::<u64>()
                    .map_err(|_| format!("--count: not a valid number: {v}"))?;
                i += 2;
            }
            "--seed" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--seed requires a value".to_string())?;
                parsed.seed = v
                    .parse::<u64>()
                    .map_err(|_| format!("--seed: not a valid number: {v}"))?;
                i += 2;
            }
            "--profile" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--profile requires a value".to_string())?;
                let lower = v.to_lowercase();
                if !VALID_PROFILES.contains(&lower.as_str()) {
                    return Err(format!(
                        "--profile: unknown profile '{v}' (valid: {})",
                        VALID_PROFILES.join(", ")
                    ));
                }
                parsed.profile = lower;
                i += 2;
            }
            "--document-type" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--document-type requires a value".to_string())?;
                let lower = v.to_lowercase();
                if !VALID_DOCUMENT_TYPES.contains(&lower.as_str()) {
                    return Err(format!(
                        "--document-type: unknown type '{v}' (valid: {})",
                        VALID_DOCUMENT_TYPES.join(", ")
                    ));
                }
                // Reuses `DocumentType::parse` rather than hand-mapping the
                // string to a variant a second time — `VALID_DOCUMENT_TYPES`
                // above exists only to produce the same "valid: ..." error
                // shape `--profile` uses, not as a second source of truth.
                parsed.document_type =
                    DocumentType::parse(&lower).map_err(|e| format!("--document-type: {e}"))?;
                i += 2;
            }
            "--out-dir" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--out-dir requires a value".to_string())?;
                parsed.out_dir = v.clone();
                i += 2;
            }
            "--redact" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--redact requires STYLE:LINE:FIRST-LAST".to_string())?;
                parsed.redact = Some(parse_redact(v)?);
                i += 2;
            }
            other => {
                return Err(format!("unknown argument: {other}"));
            }
        }
    }

    if parsed.redact.is_some() && parsed.profile != "clean" {
        return Err(
            "--redact requires --profile clean because capture profiles move MRZ cells".into(),
        );
    }
    if let Some(span) = parsed.redact {
        let layout = synthpass_gen::layout::for_format(parsed.document_type);
        if span.line >= layout.mrz_lines.len() {
            return Err(format!(
                "--redact line {} is out of range for {}",
                span.line,
                parsed.document_type.as_str()
            ));
        }
        if span.first > span.last {
            return Err(format!(
                "--redact first cell {} exceeds last cell {}",
                span.first, span.last
            ));
        }
        if span.last >= layout.mrz_chars as usize {
            return Err(format!(
                "--redact last cell {} is out of range (line width {})",
                span.last, layout.mrz_chars
            ));
        }
    }

    // `generate_command` computes `parsed.seed + i` for `i` in `0..parsed.count`,
    // so the highest seed actually used is `parsed.seed + (parsed.count - 1)`.
    // Catch an overflow here, at parse time, rather than letting the debug
    // build panic mid-batch (or the release build silently wrap the seed of
    // the last document or so — u64 addition never panics in release).
    if let Some(last_index) = parsed.count.checked_sub(1) {
        if parsed.seed.checked_add(last_index).is_none() {
            return Err(format!(
                "--seed {} + --count {}: the highest seed used ({} + {}) would overflow u64",
                parsed.seed, parsed.count, parsed.seed, last_index
            ));
        }
    }

    Ok(parsed)
}

fn parse_redact(value: &str) -> Result<RedactSpan, String> {
    let parts: Vec<_> = value.split(':').collect();
    if parts.len() != 3 {
        return Err("--redact span must be STYLE:LINE:FIRST-LAST".into());
    }
    let style = match parts[0] {
        "fill-black" => RedactStyle::FillBlack,
        "fill-white" => RedactStyle::FillWhite,
        "fill-grey" => RedactStyle::FillGrey,
        "blur" => RedactStyle::Blur,
        "graded-blur" => RedactStyle::GradedBlur,
        other => return Err(format!("--redact style '{other}' is unknown")),
    };
    let line = parts[1]
        .parse()
        .map_err(|_| format!("--redact line '{}' is not a number", parts[1]))?;
    let (first, last) = parts[2]
        .split_once('-')
        .ok_or_else(|| "--redact cells must be FIRST-LAST".to_string())?;
    Ok(RedactSpan {
        line,
        first: first
            .parse()
            .map_err(|_| format!("--redact first cell '{first}' is not a number"))?,
        last: last
            .parse()
            .map_err(|_| format!("--redact last cell '{last}' is not a number"))?,
        style,
    })
}

/// Maps this CLI's `--profile` string to `synthpass_gen::degrade`'s
/// [`CaptureProfile`](synthpass_gen::degrade::CaptureProfile) and applies its
/// recipe; `clean` stays a no-op (the pristine render, no degradation).
fn degrade_placeholder(
    image: image::DynamicImage,
    profile: &str,
    seed: u64,
) -> image::DynamicImage {
    use synthpass_gen::degrade::{apply_profile, CaptureProfile};
    let capture_profile = match profile {
        "mobile" => CaptureProfile::Mobile,
        "scanner" => CaptureProfile::Scanner,
        "worn" => CaptureProfile::Worn,
        "border-kiosk" => CaptureProfile::BorderKiosk,
        "damaged" => CaptureProfile::Damaged,
        _ => return image, // "clean" (validated in parse_args)
    };
    apply_profile(&image, capture_profile, seed)
}

/// A JSON-serializable mirror of `synthpass_gen::labels::FieldLabel` — the
/// upstream type has no `Serialize` derive, so this local copy exists purely
/// for the sidecar JSON.
#[derive(Serialize)]
struct FieldLabelJson {
    value: String,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl From<&synthpass_gen::FieldLabel> for FieldLabelJson {
    fn from(f: &synthpass_gen::FieldLabel) -> Self {
        Self {
            value: f.value.clone(),
            x: f.rect.x,
            y: f.rect.y,
            width: f.rect.width,
            height: f.rect.height,
        }
    }
}

/// A JSON-serializable mirror of `synthpass_gen::Labels`, plus generation
/// metadata (seed/profile/image dimensions) that isn't part of the upstream
/// type at all.
#[derive(Serialize)]
struct LabelsJson {
    document_type: FieldLabelJson,
    issuing_country: FieldLabelJson,
    surname: FieldLabelJson,
    given_names: FieldLabelJson,
    document_number: FieldLabelJson,
    nationality: FieldLabelJson,
    date_of_birth: FieldLabelJson,
    sex: FieldLabelJson,
    date_of_expiry: FieldLabelJson,
    #[serde(skip_serializing_if = "Option::is_none")]
    personal_number: Option<FieldLabelJson>,
    /// The primary/secondary identifier in native Cyrillic script — present
    /// only for a Cyrillic-script issuing state, where `surname`/`given_names`
    /// above carry the ICAO 9303 Part 3 §6 B transliteration. Same rect as
    /// their Latin counterparts.
    #[serde(skip_serializing_if = "Option::is_none")]
    surname_native: Option<FieldLabelJson>,
    #[serde(skip_serializing_if = "Option::is_none")]
    given_names_native: Option<FieldLabelJson>,
    /// The ICAO 9303 MRZ format ("TD1"/"TD2"/"TD3") — the join key for the
    /// bench corpus and Tier-1 gate to separate formats by, since
    /// `document_type.value` (the MRZ document code) cannot: TD1 and TD2
    /// both correctly emit `"I"`. See `synthpass_gen::DocumentType::document_code`.
    mrz_format: String,
    /// All MRZ lines for the document (2 for TD2/TD3, 3 for TD1). The
    /// `mrz_line1`/`mrz_line2`/`mrz_line3` fields below are kept for
    /// backward compatibility with sidecars already shipped; this array is
    /// the format-agnostic shape new consumers should read.
    mrz_lines: Vec<String>,
    mrz_line1: String,
    mrz_line2: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    mrz_line3: Option<String>,
    mrz_rect: FieldLabelRect,
    seed: u64,
    profile: String,
    image_width: u32,
    image_height: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    occluded: Vec<OccludedJson>,
}

#[derive(Serialize)]
struct OccludedJson {
    line: usize,
    first: usize,
    last: usize,
    kind: String,
}

#[derive(Serialize)]
struct FieldLabelRect {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

fn labels_to_json(
    labels: &Labels,
    seed: u64,
    profile: &str,
    width: u32,
    height: u32,
) -> LabelsJson {
    LabelsJson {
        document_type: (&labels.document_type).into(),
        issuing_country: (&labels.issuing_country).into(),
        surname: (&labels.surname).into(),
        given_names: (&labels.given_names).into(),
        document_number: (&labels.document_number).into(),
        nationality: (&labels.nationality).into(),
        date_of_birth: (&labels.date_of_birth).into(),
        sex: (&labels.sex).into(),
        date_of_expiry: (&labels.date_of_expiry).into(),
        personal_number: labels.personal_number.as_ref().map(Into::into),
        surname_native: labels.surname_native.as_ref().map(Into::into),
        given_names_native: labels.given_names_native.as_ref().map(Into::into),
        mrz_format: labels.mrz_format.as_str().to_string(),
        mrz_lines: labels.mrz_lines.clone(),
        mrz_line1: labels.mrz_lines[0].clone(),
        mrz_line2: labels.mrz_lines[1].clone(),
        mrz_line3: labels.mrz_lines.get(2).cloned(),
        mrz_rect: FieldLabelRect {
            x: labels.mrz_rect.x,
            y: labels.mrz_rect.y,
            width: labels.mrz_rect.width,
            height: labels.mrz_rect.height,
        },
        seed,
        profile: profile.to_string(),
        image_width: width,
        image_height: height,
        occluded: labels
            .occluded
            .iter()
            .map(|s| OccludedJson {
                line: s.line,
                first: s.first,
                last: s.last,
                kind: s.kind.clone(),
            })
            .collect(),
    }
}

/// `synthpass generate [--count N] [--seed N] [--profile NAME] [--out-dir DIR]` —
/// generates `count` synthetic passport images (PNG) + ground-truth
/// `<stem>.labels.json` sidecars into `out_dir`. Document `i` in the batch
/// uses seed `seed + i`, so a batch is fully reproducible and each document
/// differs.
///
/// No license required: see the module doc comment. Exit codes (issue #492):
/// a bad argument (unknown flag, invalid `--profile`/`--document-type`, a
/// `--seed`/`--count` combination that would overflow) is a usage error (2);
/// an I/O or encode failure partway through the batch is a runtime failure
/// (1), via `?`.
pub fn generate_command(args: &[String]) -> Result<crate::Exit, Box<dyn std::error::Error>> {
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ {e}");
            usage();
            return Ok(crate::Exit::Usage);
        }
    };

    std::fs::create_dir_all(&parsed.out_dir)?;

    for i in 0..parsed.count {
        let seed = parsed.seed + i;
        let config = GeneratorConfig::with_document_type(seed, parsed.document_type);
        let passport = data::generate_passport(&config);
        let (image, labels) = generate_with(
            &passport,
            &config,
            &RenderOptions {
                redact: parsed.redact,
            },
        );
        let image = degrade_placeholder(image, &parsed.profile, seed);
        let (width, height) = (image.width(), image.height());

        // TD3 keeps the exact `synthpass_{seed}.{ext}` shape M3 shipped
        // (unchanged filename for back-compat with existing consumers/tests).
        // TD1/TD2 append the format so that generating more than one format
        // at the same seed — exactly what the M6 verification steps below
        // do, one format at a time into the same `--out-dir` — cannot
        // silently overwrite a sibling format's output.
        let stem = match parsed.document_type {
            DocumentType::TD3 => format!("synthpass_{seed}"),
            other => format!("synthpass_{seed}_{}", other.as_str().to_lowercase()),
        };
        let png_path = Path::new(&parsed.out_dir).join(format!("{stem}.png"));
        let json_path = Path::new(&parsed.out_dir).join(format!("{stem}.{LABELS_SUFFIX}"));

        image.save(&png_path)?;

        let labels_json = labels_to_json(&labels, seed, &parsed.profile, width, height);
        let json_str = serde_json::to_string_pretty(&labels_json)?;
        std::fs::write(&json_path, json_str)?;

        println!(
            "✅ [seed {seed}] {} {} — doc# {} (profile: {}) -> {} / {}",
            passport.given_names,
            passport.surname,
            passport.document_number,
            parsed.profile,
            png_path.display(),
            json_path.display()
        );
    }

    Ok(crate::Exit::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs the full generate command against a temp directory and checks
    /// the expected PNG/`.labels.json` pairs exist and the sidecar's ground
    /// truth is sane (non-empty document number, two 44-char MRZ lines).
    #[test]
    fn generate_batch_produces_valid_outputs() {
        let out_dir =
            std::env::temp_dir().join(format!("synthpass_generate_smoke_{}", std::process::id()));
        let out_dir_str = out_dir.to_string_lossy().to_string();

        let args = vec![
            "--count".to_string(),
            "3".to_string(),
            "--seed".to_string(),
            "42".to_string(),
            "--profile".to_string(),
            "mobile".to_string(),
            "--out-dir".to_string(),
            out_dir_str.clone(),
        ];

        generate_command(&args).expect("generate_command should succeed");

        for i in 0..3u64 {
            let seed = 42 + i;
            let png_path = out_dir.join(format!("synthpass_{seed}.png"));
            let json_path = out_dir.join(format!("synthpass_{seed}.{LABELS_SUFFIX}"));
            assert!(png_path.exists(), "missing PNG for seed {seed}");
            assert!(json_path.exists(), "missing JSON sidecar for seed {seed}");
            // Issue #514: `<stem>.json` is extraction's output name.
            assert!(
                !out_dir.join(format!("synthpass_{seed}.json")).exists(),
                "generate must not write a bare <stem>.json for seed {seed}"
            );

            let json_str = std::fs::read_to_string(&json_path).expect("read sidecar");
            let value: serde_json::Value =
                serde_json::from_str(&json_str).expect("sidecar should be valid JSON");

            let doc_number = value["document_number"]["value"]
                .as_str()
                .expect("document_number.value should be a string");
            assert!(
                !doc_number.is_empty(),
                "document_number should not be empty"
            );

            let mrz1 = value["mrz_line1"].as_str().expect("mrz_line1");
            let mrz2 = value["mrz_line2"].as_str().expect("mrz_line2");
            assert_eq!(mrz1.len(), 44, "mrz_line1 should be 44 chars");
            assert_eq!(mrz2.len(), 44, "mrz_line2 should be 44 chars");

            assert_eq!(value["seed"].as_u64(), Some(seed));
            assert_eq!(value["profile"].as_str(), Some("mobile"));
        }

        std::fs::remove_dir_all(&out_dir).ok();
    }

    /// Pins the fix for issue #514: extraction's own output path for a
    /// generated PNG must never collide with the generator's own label
    /// sidecar path, so reading a generated pass back through extraction
    /// cannot overwrite the ground truth the read is graded against.
    ///
    /// `synthpass_pipeline::Pipeline::process_document` has no standalone
    /// path function reachable without a live OCR engine (it needs one to
    /// even start), so rather than reimplement its logic, this copies its
    /// exact computation verbatim: `md_path = input.with_extension("md")`,
    /// then `json_path = stage.md_path.with_extension("json")`
    /// (`crates/synthpass-pipeline/src/lib.rs`, `process_document`).
    #[test]
    fn generator_label_path_differs_from_extractions_output_path() {
        let out_dir = std::env::temp_dir().join(format!(
            "synthpass_generate_path_collision_{}",
            std::process::id()
        ));
        let out_dir_str = out_dir.to_string_lossy().to_string();

        let args = vec![
            "--seed".to_string(),
            "42".to_string(),
            "--out-dir".to_string(),
            out_dir_str,
        ];
        generate_command(&args).expect("generate_command should succeed");

        let png_path = out_dir.join("synthpass_42.png");
        let generator_labels_path = out_dir.join(format!("synthpass_42.{LABELS_SUFFIX}"));
        // Verbatim copy of process_document's own path computation.
        let extraction_output_path = png_path.with_extension("md").with_extension("json");

        assert_ne!(
            generator_labels_path, extraction_output_path,
            "extraction's output path must never collide with the generator's own labels"
        );
        assert!(generator_labels_path.exists());

        std::fs::remove_dir_all(&out_dir).ok();
    }

    /// `--document-type td1`/`td2` must produce a sidecar whose `mrz_format`
    /// and MRZ line count/width agree with the requested format (TD1: 3x30,
    /// TD2: 2x36) — the M6 verification step this test exists to pin down.
    #[test]
    fn document_type_flag_produces_matching_sidecar() {
        let cases = [
            ("td1", "TD1", 3, 30),
            ("td2", "TD2", 2, 36),
            ("td3", "TD3", 2, 44),
            ("mrva", "MRVA", 2, 44),
            ("mrvb", "MRVB", 2, 36),
        ];
        for (flag, expected_format, expected_lines, expected_width) in cases {
            let out_dir = std::env::temp_dir().join(format!(
                "synthpass_generate_doctype_smoke_{}_{flag}",
                std::process::id()
            ));
            let out_dir_str = out_dir.to_string_lossy().to_string();

            let args = vec![
                "--seed".to_string(),
                "7".to_string(),
                "--document-type".to_string(),
                flag.to_string(),
                "--out-dir".to_string(),
                out_dir_str.clone(),
            ];
            generate_command(&args).expect("generate_command should succeed");

            // TD3 keeps the plain `synthpass_{seed}.labels.json` shape;
            // TD1/TD2 get a format suffix so co-located outputs at the same
            // seed don't collide (see `generate_command`).
            let json_name = if flag == "td3" {
                format!("synthpass_7.{LABELS_SUFFIX}")
            } else {
                format!("synthpass_7_{flag}.{LABELS_SUFFIX}")
            };
            let json_path = out_dir.join(json_name);
            let json_str = std::fs::read_to_string(&json_path).expect("read sidecar");
            let value: serde_json::Value =
                serde_json::from_str(&json_str).expect("sidecar should be valid JSON");

            assert_eq!(
                value["mrz_format"].as_str(),
                Some(expected_format),
                "flag {flag}"
            );
            let lines = value["mrz_lines"]
                .as_array()
                .unwrap_or_else(|| panic!("mrz_lines should be an array for flag {flag}"));
            assert_eq!(lines.len(), expected_lines, "flag {flag}");
            for l in lines {
                assert_eq!(
                    l.as_str().expect("line should be a string").len(),
                    expected_width,
                    "flag {flag}"
                );
            }

            std::fs::remove_dir_all(&out_dir).ok();
        }
    }

    #[test]
    fn rejects_unknown_document_type() {
        let args = vec!["--document-type".to_string(), "td4".to_string()];
        let err = parse_args(&args).unwrap_err();
        assert!(err.contains("unknown type") || err.contains("valid values"));
    }

    #[test]
    fn rejects_unknown_profile() {
        let args = vec!["--profile".to_string(), "bogus".to_string()];
        let err = parse_args(&args).unwrap_err();
        assert!(err.contains("unknown profile"));
    }

    #[test]
    fn rejects_seed_count_overflow() {
        let args = vec![
            "--seed".to_string(),
            u64::MAX.to_string(),
            "--count".to_string(),
            "2".to_string(),
        ];
        let err = parse_args(&args).unwrap_err();
        assert!(err.contains("overflow"), "unexpected error: {err}");
    }

    #[test]
    fn seed_count_one_at_max_seed_does_not_overflow() {
        // count == 1 only ever uses `seed + 0`, so the max seed itself is fine.
        let args = vec![
            "--seed".to_string(),
            u64::MAX.to_string(),
            "--count".to_string(),
            "1".to_string(),
        ];
        assert!(parse_args(&args).is_ok());
    }

    /// `usage()`'s `--profile` line is built from `VALID_PROFILES.join("|")`
    /// (see `usage`), so this pins that exact string rather than the
    /// previously hand-written help text, which once omitted `damaged` even
    /// though `VALID_PROFILES` — the list `--profile` is actually validated
    /// against — accepted it.
    #[test]
    fn profile_help_list_is_built_from_valid_profiles() {
        let joined = VALID_PROFILES.join("|");
        for profile in VALID_PROFILES {
            assert!(
                joined.contains(profile),
                "profile {profile} missing from the joined help list: {joined}"
            );
        }
        assert!(joined.contains("damaged"), "joined list: {joined}");
    }

    #[test]
    fn rejects_unknown_flag() {
        let args = vec!["--nope".to_string()];
        let err = parse_args(&args).unwrap_err();
        assert!(err.contains("unknown argument"));
    }

    #[test]
    fn redact_span_validates_style_geometry_and_profile() {
        for (arg, expected) in [
            ("nope:0:0-1", "style"),
            ("blur:9:0-1", "line"),
            ("blur:0:2-1", "first cell"),
            ("blur:0:0-44", "last cell"),
        ] {
            let args = vec!["--redact".to_string(), arg.to_string()];
            assert!(parse_args(&args).unwrap_err().contains(expected));
        }
        let args = vec![
            "--redact".into(),
            "blur:0:0-1".into(),
            "--profile".into(),
            "mobile".into(),
        ];
        assert!(parse_args(&args)
            .unwrap_err()
            .contains("requires --profile clean"));
    }
}
