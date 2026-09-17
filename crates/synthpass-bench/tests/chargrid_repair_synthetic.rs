//! PR-1.4 synthetic no-regression check for `SYNTHPASS_OCR_CHARGRID=on`:
//! over a small fixed-seed synthetic corpus, the chargrid post-hit
//! name-line repair must never turn a Tier-1 hit `off` had into a miss.
//!
//! `#[ignore]`d because it needs the real `.rten` OCR models at the repo
//! root — the same requirement as `crates/synthpass-ocr/tests/native_ocr_e2e.rs`
//! and `synthpass-bench`'s own `some_clean_generated_documents_hit`. Run with:
//!
//! ```text
//! cargo test -p synthpass-bench --test chargrid_repair_synthetic -- --ignored --nocapture
//! ```
//!
//! # Why one process, not two builds
//!
//! `SYNTHPASS_OCR_CHARGRID` is read fresh on every
//! `NativeOcr::recognize_detailed` call, so this test toggles it in-process
//! between two passes over the identical generated images with the identical
//! warm `NativeOcr` — the same same-binary A/B discipline the project's own
//! benchmark A/Bs use (a rebuild-based before/after has hidden a real
//! hit-rate regression here before; see
//! `knowledge/benchmarks/texture-suppression-ab-2026-09-03.md` for the
//! precedent this follows).
//!
//! # Why 5 formats, a handful of seeds each
//!
//! `synthpass-gen` can produce all five ICAO 9303 formats
//! (`mrz_provider_equivalence.rs` already exercises all five against the
//! deterministic parser); chargrid's own `format_geometry`/`name_line_index`
//! cover the same five, so this corpus is built the same way, generated with
//! `synthpass-gen`'s `embedded-fonts` feature (already enabled for this
//! crate's `synthpass-gen` dependency) rather than depending on system fonts
//! being installed. `SEEDS_PER_FORMAT` is intentionally modest — this is a
//! regression gate on the *direction* of the delta (never fewer hits), not
//! the benchmark harness's own accuracy measurement, which already runs a
//! much larger sweep.

use synthpass_bench::check_document;
use synthpass_gen::{generate_from_seed, DocumentType, GeneratorConfig};
use synthpass_ocr::NativeOcr;

const SEEDS_PER_FORMAT: u64 = 20;

const FORMATS: [DocumentType; 5] = [
    DocumentType::TD1,
    DocumentType::TD2,
    DocumentType::TD3,
    DocumentType::MrvA,
    DocumentType::MrvB,
];

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("workspace root")
        .to_path_buf()
}

#[test]
#[ignore]
fn chargrid_on_never_loses_a_tier1_hit_off_had() {
    let root = repo_root();
    let ocr = NativeOcr::load(
        &root.join("text-detection.rten"),
        &root.join("text-recognition.rten"),
    )
    .expect("failed to load OCR models — run from the repo root");

    // Generate every image once; both passes read the identical bytes.
    let mut docs = Vec::new();
    for doc_type in FORMATS {
        for seed in 0..SEEDS_PER_FORMAT {
            let config = GeneratorConfig::with_document_type(seed, doc_type);
            let (image, labels, _passport) = generate_from_seed(&config);
            docs.push((doc_type, seed, image, labels));
        }
    }

    unsafe { std::env::set_var("SYNTHPASS_OCR_CHARGRID", "off") };
    let off: Vec<_> = docs
        .iter()
        .map(|(doc_type, seed, image, labels)| {
            let r = check_document(&ocr, image, labels);
            (*doc_type, *seed, r.hit, r.names_exact)
        })
        .collect();

    unsafe { std::env::set_var("SYNTHPASS_OCR_CHARGRID", "on") };
    let on: Vec<_> = docs
        .iter()
        .map(|(doc_type, seed, image, labels)| {
            let r = check_document(&ocr, image, labels);
            (*doc_type, *seed, r.hit, r.names_exact)
        })
        .collect();

    unsafe { std::env::remove_var("SYNTHPASS_OCR_CHARGRID") };

    let off_hits = off.iter().filter(|(.., hit, _)| *hit).count();
    let on_hits = on.iter().filter(|(.., hit, _)| *hit).count();
    let off_names_exact = off.iter().filter(|(.., exact)| *exact).count();
    let on_names_exact = on.iter().filter(|(.., exact)| *exact).count();
    println!(
        "chargrid synthetic A/B over {} documents: hits off={off_hits} on={on_hits}, \
         names_exact off={off_names_exact} on={on_names_exact}",
        docs.len()
    );

    let mut regressions = Vec::new();
    for ((doc_type, seed, off_hit, _), (_, _, on_hit, _)) in off.iter().zip(on.iter()) {
        if *off_hit && !*on_hit {
            regressions.push(format!("{doc_type:?} seed {seed}"));
        }
    }
    assert!(
        regressions.is_empty(),
        "chargrid mode `on` lost a Tier-1 hit that `off` had on: {regressions:?}"
    );
}
