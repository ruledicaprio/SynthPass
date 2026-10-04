//! `synthpass export` — turn a deterministic `synthpass-gen` corpus into a
//! training dataset on disk (JSONL / Hugging Face), in the DeepSeek-OCR 0–1000
//! convention fixed by `knowledge/decisions/ADR-0007-dataset-export-format.md`.
//!
//! Export generates synthetic, PII-free data — it is not extraction, and it
//! never refuses on license grounds. Like `generate`, it always runs; unlike
//! `generate`, bulk dataset production is a capacity surface per
//! `knowledge/BRANDING.md` §5, so a missing/invalid license, or one that
//! lacks the `export` feature, is **metered**: `main.rs`'s dispatch prints one
//! stderr warning and lets this module run regardless (issue #494).
//! `SYNTHPASS_LICENSE_SKIP=1` suppresses the check (and the warning)
//! entirely, like everywhere else in the CLI.

use std::path::PathBuf;
use synthpass_export::{DocTypeChoice, ExportConfig, ExportFormat};

/// Parsed `synthpass export` arguments.
#[derive(Debug)]
struct ExportArgs {
    format: ExportFormat,
    count: u64,
    seed: u64,
    document_type: DocTypeChoice,
    viz_font: Option<synthpass_gen::VizFontChoice>,
    pack_pages: u32,
    out_dir: PathBuf,
}

fn usage() {
    eprintln!(
        "Usage: synthpass export --format FMT [--count N] [--seed N] [--document-type TYPE] [--viz-font NAME|random] [--pack-pages N] --out-dir DIR"
    );
    eprintln!("  --format FMT          jsonl | hf");
    eprintln!("  --count N             documents to generate and export (default: 100)");
    eprintln!("  --seed N              base seed; document i uses seed N+i (default: 0)");
    eprintln!("  --document-type TYPE  td1|td2|td3|mrva|mrvb|all (default: td3)");
    eprintln!("  --pack-pages N        documents concatenated per JSONL row, joined with <page> (default: 1)");
    eprintln!("  --profile clean       fixed at 'clean' in v1 (accepted for forward-compat)");
    eprintln!("  --out-dir DIR         output directory (required; created if absent)");
    eprintln!("  --viz-font NAME       pt-sans|liberation-sans|source-sans-3|liberation-serif|liberation-mono|random (default: PT Sans, unrecorded)");
    eprintln!();
    eprintln!("Format and schema: knowledge/EXPORTS.md. Runs without a license; a license");
    eprintln!("lacking the 'export' feature is metered with a warning, not refused.");
}

/// Hand-rolled flag parser, consistent with `generate.rs` (no clap).
fn parse_args(args: &[String]) -> Result<ExportArgs, String> {
    let mut format: Option<ExportFormat> = None;
    let mut count: u64 = 100;
    let mut seed: u64 = 0;
    let mut document_type = DocTypeChoice::One(synthpass_gen::DocumentType::TD3);
    let mut pack_pages: u32 = 1;
    let mut viz_font = None;
    let mut out_dir: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--format" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--format requires a value".to_string())?;
                format = Some(ExportFormat::parse(v)?);
                i += 2;
            }
            "--count" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--count requires a value".to_string())?;
                count = v
                    .parse()
                    .map_err(|_| format!("--count: not a valid number: {v}"))?;
                i += 2;
            }
            "--seed" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--seed requires a value".to_string())?;
                seed = v
                    .parse()
                    .map_err(|_| format!("--seed: not a valid number: {v}"))?;
                i += 2;
            }
            "--document-type" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--document-type requires a value".to_string())?;
                document_type =
                    DocTypeChoice::parse(v).map_err(|e| format!("--document-type: {e}"))?;
                i += 2;
            }
            "--viz-font" => {
                let value = args
                    .get(i + 1)
                    .ok_or_else(|| "--viz-font requires a value".to_string())?;
                viz_font = Some(
                    synthpass_gen::VizFontChoice::parse(value)
                        .map_err(|e| format!("--viz-font: {e}"))?,
                );
                i += 2;
            }
            "--pack-pages" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--pack-pages requires a value".to_string())?;
                pack_pages = v
                    .parse()
                    .map_err(|_| format!("--pack-pages: not a valid number: {v}"))?;
                if pack_pages == 0 {
                    return Err("--pack-pages must be at least 1".to_string());
                }
                i += 2;
            }
            "--profile" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--profile requires a value".to_string())?;
                if !v.eq_ignore_ascii_case("clean") {
                    return Err(format!(
                        "--profile: only 'clean' is supported in v1 (got '{v}') — see knowledge/EXPORTS.md"
                    ));
                }
                i += 2;
            }
            "--out-dir" => {
                let v = args
                    .get(i + 1)
                    .ok_or_else(|| "--out-dir requires a value".to_string())?;
                out_dir = Some(PathBuf::from(v));
                i += 2;
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(ExportArgs {
        format: format.ok_or_else(|| "--format is required (jsonl | hf)".to_string())?,
        count,
        seed,
        document_type,
        viz_font,
        pack_pages,
        out_dir: out_dir.ok_or_else(|| "--out-dir is required".to_string())?,
    })
}

/// `synthpass export` entry point. Never returns [`crate::Exit::License`]
/// (issue #494) — the caller in `main.rs` has already warned about a missing
/// or unentitled license before reaching here, and export runs regardless.
/// Exit codes (issue #492): a bad argument is a usage error (2); a
/// `synthpass_export::run` failure — the corpus generated fine but writing
/// the dataset didn't — is a runtime failure (1).
pub fn export_command(args: &[String]) -> Result<crate::Exit, Box<dyn std::error::Error>> {
    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ {e}");
            usage();
            return Ok(crate::Exit::Usage);
        }
    };

    let cfg = ExportConfig {
        format: parsed.format,
        count: parsed.count,
        seed_base: parsed.seed,
        document_type: parsed.document_type,
        viz_font: parsed.viz_font,
        pack_pages: parsed.pack_pages,
        out_dir: parsed.out_dir,
    };

    match synthpass_export::run(&cfg) {
        Ok(summary) => {
            println!(
                "✅ exported {} document(s) as {} row(s) ({}) -> {}",
                summary.documents,
                summary.rows,
                parsed.format.as_str(),
                summary.out_dir.display(),
            );
            println!(
                "   manifest: {}",
                summary.out_dir.join("manifest.json").display()
            );
            Ok(crate::Exit::Ok)
        }
        Err(e) => {
            eprintln!("❌ export failed: {e}");
            Ok(crate::Exit::Failure)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use synthpass_gen::DocumentType;

    #[test]
    fn viz_font_values_parse_and_unknown_lists_valid_values() {
        for name in synthpass_gen::fonts::VizFont::ALL
            .map(synthpass_gen::fonts::VizFont::name)
            .into_iter()
            .chain(["random"])
        {
            let parsed = parse(&["--format", "jsonl", "--out-dir", "out", "--viz-font", name])
                .expect("valid font");
            assert_eq!(
                parsed.viz_font,
                Some(synthpass_gen::VizFontChoice::parse(name).expect("known font"))
            );
        }
        let error = parse_err(&[
            "--format",
            "jsonl",
            "--out-dir",
            "out",
            "--viz-font",
            "unknown",
        ]);
        for name in synthpass_gen::fonts::VizFont::ALL
            .map(synthpass_gen::fonts::VizFont::name)
            .into_iter()
            .chain(["random"])
        {
            assert!(error.contains(name));
        }
        assert_eq!(
            parse(&["--format", "jsonl", "--out-dir", "out"])
                .expect("default")
                .viz_font,
            None
        );
    }

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn parse(v: &[&str]) -> Result<ExportArgs, String> {
        parse_args(&args(v))
    }

    fn parse_err(v: &[&str]) -> String {
        match parse(v) {
            Ok(a) => panic!("expected an error for {v:?}, got {a:?}"),
            Err(e) => e,
        }
    }

    #[test]
    fn required_flags_only_takes_documented_defaults() {
        let a = parse(&["--format", "jsonl", "--out-dir", "out"]).expect("valid args");
        assert_eq!(a.format, ExportFormat::Jsonl);
        assert_eq!(a.count, 100);
        assert_eq!(a.seed, 0);
        assert_eq!(a.document_type, DocTypeChoice::One(DocumentType::TD3));
        assert_eq!(a.pack_pages, 1);
        assert_eq!(a.out_dir, PathBuf::from("out"));
    }

    #[test]
    fn every_flag_is_applied_in_any_order() {
        let a = parse(&[
            "--out-dir",
            "ds",
            "--pack-pages",
            "4",
            "--document-type",
            "mrvb",
            "--seed",
            "18446744073709551615",
            "--count",
            "7",
            "--profile",
            "clean",
            "--format",
            "hf",
        ])
        .expect("valid args");
        assert_eq!(a.format, ExportFormat::Hf);
        assert_eq!(a.count, 7);
        assert_eq!(a.seed, u64::MAX);
        assert_eq!(a.document_type, DocTypeChoice::One(DocumentType::MrvB));
        assert_eq!(a.pack_pages, 4);
        assert_eq!(a.out_dir, PathBuf::from("ds"));
    }

    #[test]
    fn values_are_case_insensitive() {
        let a = parse(&[
            "--format",
            "JSONL",
            "--document-type",
            "ALL",
            "--profile",
            "Clean",
            "--out-dir",
            "out",
        ])
        .expect("valid args");
        assert_eq!(a.format, ExportFormat::Jsonl);
        assert_eq!(a.document_type, DocTypeChoice::All);
    }

    #[test]
    fn a_repeated_flag_keeps_the_last_value() {
        let a = parse(&[
            "--format",
            "jsonl",
            "--count",
            "5",
            "--count",
            "9",
            "--out-dir",
            "a",
            "--out-dir",
            "b",
        ])
        .expect("valid args");
        assert_eq!(a.count, 9);
        assert_eq!(a.out_dir, PathBuf::from("b"));
    }

    #[test]
    fn missing_required_flags_are_named() {
        assert_eq!(
            parse_err(&["--out-dir", "out"]),
            "--format is required (jsonl | hf)"
        );
        assert_eq!(parse_err(&["--format", "jsonl"]), "--out-dir is required");
        // With neither, --format is reported first.
        assert_eq!(parse_err(&[]), "--format is required (jsonl | hf)");
    }

    #[test]
    fn a_trailing_flag_without_its_value_is_named() {
        for flag in [
            "--format",
            "--count",
            "--seed",
            "--document-type",
            "--pack-pages",
            "--profile",
            "--out-dir",
        ] {
            assert_eq!(
                parse_err(&[flag]),
                format!("{flag} requires a value"),
                "flag {flag}"
            );
        }
    }

    #[test]
    fn numeric_flags_reject_non_u64_values() {
        for (flag, bad) in [
            ("--count", "ten"),
            ("--count", "-1"),
            ("--count", "18446744073709551616"),
            ("--seed", "1.5"),
            ("--seed", ""),
        ] {
            assert_eq!(
                parse_err(&["--format", "jsonl", "--out-dir", "o", flag, bad]),
                format!("{flag}: not a valid number: {bad}")
            );
        }
    }

    #[test]
    fn pack_pages_rejects_zero_and_non_u32_values() {
        assert_eq!(
            parse_err(&["--format", "jsonl", "--out-dir", "o", "--pack-pages", "0"]),
            "--pack-pages must be at least 1"
        );
        assert_eq!(
            parse_err(&[
                "--format",
                "jsonl",
                "--out-dir",
                "o",
                "--pack-pages",
                "4294967296"
            ]),
            "--pack-pages: not a valid number: 4294967296"
        );
    }

    #[test]
    fn unknown_and_deferred_formats_are_rejected() {
        let e = parse_err(&["--format", "csv", "--out-dir", "o"]);
        assert!(e.contains("unknown format 'csv'"), "{e}");
        // coco/yolo are on the roadmap, so they read as "not yet", not "unknown".
        for f in ["coco", "yolo"] {
            let e = parse_err(&["--format", f, "--out-dir", "o"]);
            assert!(e.contains("not implemented yet"), "{e}");
        }
    }

    #[test]
    fn a_bad_document_type_is_prefixed_with_its_flag() {
        let e = parse_err(&[
            "--format",
            "jsonl",
            "--out-dir",
            "o",
            "--document-type",
            "td4",
        ]);
        assert!(e.starts_with("--document-type: "), "{e}");
        assert!(e.contains("td4"), "{e}");
    }

    #[test]
    fn a_profile_other_than_clean_is_rejected() {
        let e = parse_err(&["--format", "jsonl", "--out-dir", "o", "--profile", "mobile"]);
        assert!(
            e.starts_with("--profile: only 'clean' is supported in v1 (got 'mobile')"),
            "{e}"
        );
    }

    #[test]
    fn unknown_flags_and_positionals_are_rejected() {
        assert_eq!(
            parse_err(&["--format", "jsonl", "--out-dir", "o", "--verbose"]),
            "unknown argument: --verbose"
        );
        assert_eq!(
            parse_err(&["dataset", "--format", "jsonl", "--out-dir", "o"]),
            "unknown argument: dataset"
        );
    }
}
