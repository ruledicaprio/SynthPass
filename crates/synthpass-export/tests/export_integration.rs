//! End-to-end: `synthpass_export::run` against a temp directory.

use std::path::PathBuf;
use synthpass_export::{run, DocTypeChoice, ExportConfig, ExportFormat};
use synthpass_gen::DocumentType;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "synthpass_export_it_{tag}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn cfg(format: ExportFormat, out: PathBuf) -> ExportConfig {
    ExportConfig {
        format,
        count: 6,
        seed_base: 100,
        document_type: DocTypeChoice::One(DocumentType::TD3),
        pack_pages: 1,
        out_dir: out,
    }
}

#[test]
fn jsonl_export_writes_the_expected_tree() {
    let out = scratch("jsonl");
    let summary = run(&cfg(ExportFormat::Jsonl, out.clone())).expect("export runs");

    assert_eq!(summary.rows, 6);
    assert_eq!(summary.documents, 6);

    let jsonl = out.join("data.jsonl");
    assert!(jsonl.is_file(), "data.jsonl missing");
    assert!(out.join("manifest.json").is_file(), "manifest.json missing");

    let body = std::fs::read_to_string(&jsonl).unwrap();
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 6);

    // Every row parses, carries one document, and its image file exists.
    for line in &lines {
        let row: serde_json::Value = serde_json::from_str(line).expect("row is JSON");
        assert_eq!(row["documents"].as_array().unwrap().len(), 1);
        let doc = &row["documents"][0];
        assert_eq!(doc["document_type"], "td3");
        assert_eq!(doc["mrz_format"], "TD3");
        let img = doc["image"].as_str().unwrap();
        assert!(out.join(img).is_file(), "missing image {img}");
        // 0-1000 boxes.
        for block in doc["blocks"].as_array().unwrap() {
            for v in block["box"].as_array().unwrap() {
                assert!((0..=1000).contains(&v.as_u64().unwrap()));
            }
        }
        assert!(doc["ground_truth"].as_str().unwrap().starts_with("<|ref|>"));
    }

    // Manifest sha256 matches the file on disk.
    let manifest: synthpass_export::Manifest =
        serde_json::from_str(&std::fs::read_to_string(out.join("manifest.json")).unwrap()).unwrap();
    assert_eq!(manifest.format, "jsonl");
    assert_eq!(manifest.profile, "clean");
    assert_eq!(manifest.files.len(), 1);
    assert_eq!(manifest.files[0].rows, 6);

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn export_is_deterministic_in_the_config() {
    let a = scratch("det_a");
    let b = scratch("det_b");
    run(&cfg(ExportFormat::Jsonl, a.clone())).unwrap();
    run(&cfg(ExportFormat::Jsonl, b.clone())).unwrap();

    let ja = std::fs::read(a.join("data.jsonl")).unwrap();
    let jb = std::fs::read(b.join("data.jsonl")).unwrap();
    assert_eq!(ja, jb, "same config must produce byte-identical JSONL");

    // ...and the PNG bytes too (pixels are a pure function of the seed).
    let pa = std::fs::read(a.join("images/td3-000100.png")).unwrap();
    let pb = std::fs::read(b.join("images/td3-000100.png")).unwrap();
    assert_eq!(pa, pb);

    let _ = std::fs::remove_dir_all(&a);
    let _ = std::fs::remove_dir_all(&b);
}

#[test]
fn hf_export_is_datasets_loadable_shape() {
    let out = scratch("hf");
    run(&cfg(ExportFormat::Hf, out.clone())).unwrap();

    assert!(out.join("data/train.jsonl").is_file());
    assert!(out.join("data/images/td3-000100.png").is_file());
    assert!(out.join("dataset_infos.json").is_file());
    assert!(out.join("README.md").is_file());
    assert!(out.join("manifest.json").is_file());

    let infos: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(out.join("dataset_infos.json")).unwrap())
            .unwrap();
    assert_eq!(infos["default"]["splits"]["train"]["num_examples"], 6);

    let card = std::fs::read_to_string(out.join("README.md")).unwrap();
    assert!(card.starts_with("---\nlicense: mit"));
    assert!(card.contains("SYNTHETIC / SPECIMEN"));

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn document_type_all_round_robins_and_packs() {
    let out = scratch("all");
    let c = ExportConfig {
        format: ExportFormat::Jsonl,
        count: 10,
        seed_base: 0,
        document_type: DocTypeChoice::All,
        pack_pages: 3,
        out_dir: out.clone(),
    };
    run(&c).unwrap();

    let body = std::fs::read_to_string(out.join("data.jsonl")).unwrap();
    let rows: Vec<serde_json::Value> = body
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(rows.len(), 4, "10 docs / 3 per row = 4 rows");
    assert_eq!(rows[0]["documents"].as_array().unwrap().len(), 3);
    assert!(rows[0]["ground_truth"].as_str().unwrap().contains("<page>"));

    // First five documents cover all five formats (TD1..MRVB round-robin).
    let mut seen: Vec<String> = rows
        .iter()
        .flat_map(|r| r["documents"].as_array().unwrap())
        .take(5)
        .map(|d| d["document_type"].as_str().unwrap().to_string())
        .collect();
    seen.sort();
    seen.dedup();
    assert_eq!(seen.len(), 5, "round-robin should hit every format");

    let _ = std::fs::remove_dir_all(&out);
}

#[test]
fn rejects_empty_requests() {
    let out = scratch("empty");
    let mut c = cfg(ExportFormat::Jsonl, out);
    c.count = 0;
    assert!(run(&c).is_err());
    c.count = 1;
    c.pack_pages = 0;
    assert!(run(&c).is_err());
}
