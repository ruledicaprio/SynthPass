//! `synthpass export` — turn a deterministic `synthpass-gen` corpus into a
//! training dataset on disk (JSONL / Hugging Face), in the DeepSeek-OCR 0–1000
//! convention fixed by `knowledge/decisions/ADR-0007-dataset-export-format.md`.
//!
//! Unlike `generate` (a single document, free), this is **gated on the license
//! `export` feature** — bulk dataset production is a capacity surface per
//! `knowledge/BRANDING.md` §5. `SYNTHPASS_LICENSE_SKIP=1` bypasses it for local
//! development, like everywhere else in the CLI.

use std::path::PathBuf;
use synthpass_export::{DocTypeChoice, ExportConfig, ExportFormat};

/// Parsed `synthpass export` arguments.
#[derive(Debug)]
struct ExportArgs {
    format: ExportFormat,
    count: u64,
    seed: u64,
    document_type: DocTypeChoice,
    pack_pages: u32,
    out_dir: PathBuf,
}

fn usage() {
    eprintln!(
        "Usage: synthpass export --format FMT [--count N] [--seed N] [--document-type TYPE] [--pack-pages N] --out-dir DIR"
    );
    eprintln!("  --format FMT          jsonl | hf");
    eprintln!("  --count N             documents to generate and export (default: 100)");
    eprintln!("  --seed N              base seed; document i uses seed N+i (default: 0)");
    eprintln!("  --document-type TYPE  td1|td2|td3|mrva|mrvb|all (default: td3)");
    eprintln!("  --pack-pages N        documents concatenated per JSONL row, joined with <page> (default: 1)");
    eprintln!("  --profile clean       fixed at 'clean' in v1 (accepted for forward-compat)");
    eprintln!("  --out-dir DIR         output directory (required; created if absent)");
    eprintln!();
    eprintln!("Format and schema: knowledge/EXPORTS.md. Needs the license 'export' feature");
    eprintln!("(or SYNTHPASS_LICENSE_SKIP=1 for local development).");
}

/// Hand-rolled flag parser, consistent with `generate.rs` (no clap).
fn parse_args(args: &[String]) -> Result<ExportArgs, String> {
    let mut format: Option<ExportFormat> = None;
    let mut count: u64 = 100;
    let mut seed: u64 = 0;
    let mut document_type = DocTypeChoice::One(synthpass_gen::DocumentType::TD3);
    let mut pack_pages: u32 = 1;
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
        pack_pages,
        out_dir: out_dir.ok_or_else(|| "--out-dir is required".to_string())?,
    })
}

/// `synthpass export` entry point. Returns `Ok(())` on a handled user error
/// (usage already printed) so the process exit code stays 0 for
/// "ran, told you what was wrong" — same convention as `generate`.
pub fn export_command(args: &[String], license_ok: bool) -> Result<(), Box<dyn std::error::Error>> {
    if !license_ok {
        return Ok(());
    }

    let parsed = match parse_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ {e}");
            usage();
            return Ok(());
        }
    };

    let cfg = ExportConfig {
        format: parsed.format,
        count: parsed.count,
        seed_base: parsed.seed,
        document_type: parsed.document_type,
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
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ export failed: {e}");
            Ok(())
        }
    }
}
