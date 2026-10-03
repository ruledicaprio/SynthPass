//! The five built-in renders, pinned pixel-for-pixel.
//!
//! ADR-0022 Decision 6 requires the built-in layouts to stay "byte-identical to
//! golden hashes recorded before the change". This test records them: for each of
//! TD1, TD2, TD3, MRV-A and MRV-B at seeds 0, 1, 42 and 1000, plus TD3 at seed 0
//! without a personal number (21 cases), it compares the image width, height and
//! the SHA-256 of the raw RGB8 buffer with `tests/fixtures/golden_render.tsv`
//! (columns: format, seed, personal-number flag, width, height, hash).
//!
//! A missing fixture or any mismatch fails and names the case with both hashes.
//!
//! To re-bless:
//! `SYNTHPASS_GOLDEN_RENDER_BLESS=1 cargo test -p synthpass-gen --test golden_render`.
//!
//! A re-bless is a deliberate, reviewed change (ADR-0022 Decision 6), never a way
//! to make a red test green.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use synthpass_gen::{generate_from_seed, DocumentType, GeneratorConfig};

const SEEDS: [u64; 4] = [0, 1, 42, 1000];

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden_render.tsv")
}

fn format_name(fmt: DocumentType) -> &'static str {
    match fmt {
        DocumentType::TD1 => "TD1",
        DocumentType::TD2 => "TD2",
        DocumentType::TD3 => "TD3",
        DocumentType::MrvA => "MrvA",
        DocumentType::MrvB => "MrvB",
    }
}

/// The 21 `(format, seed, include_personal_number)` cases, in fixture order.
fn cases() -> Vec<(DocumentType, u64, bool)> {
    let formats = [
        DocumentType::TD1,
        DocumentType::TD2,
        DocumentType::TD3,
        DocumentType::MrvA,
        DocumentType::MrvB,
    ];
    let mut out: Vec<_> = formats
        .iter()
        .flat_map(|&fmt| SEEDS.iter().map(move |&seed| (fmt, seed, true)))
        .collect();
    out.push((DocumentType::TD3, 0, false));
    out
}

fn render_case(fmt: DocumentType, seed: u64, personal_number: bool) -> (u32, u32, String) {
    let mut cfg = GeneratorConfig::with_document_type(seed, fmt);
    cfg.include_personal_number = personal_number;
    let (image, _labels, _passport) = generate_from_seed(&cfg);
    let rgb = image.to_rgb8();
    let (width, height) = (rgb.width(), rgb.height());
    let digest = Sha256::digest(rgb.into_raw());
    let mut hash = String::with_capacity(64);
    for byte in digest.iter() {
        write!(hash, "{byte:02x}").expect("writing to a String cannot fail");
    }
    (width, height, hash)
}

fn line_for(fmt: DocumentType, seed: u64, personal_number: bool) -> String {
    let (width, height, hash) = render_case(fmt, seed, personal_number);
    format!(
        "{}\t{seed}\t{personal_number}\t{width}\t{height}\t{hash}",
        format_name(fmt)
    )
}

#[test]
fn built_in_renders_match_golden_hashes() {
    let path = fixture_path();
    let actual: Vec<String> = cases()
        .into_iter()
        .map(|(fmt, seed, pn)| line_for(fmt, seed, pn))
        .collect();

    if std::env::var_os("SYNTHPASS_GOLDEN_RENDER_BLESS").is_some() {
        let mut body = actual.join("\n");
        body.push('\n');
        fs::write(&path, body).expect("write golden fixture");
        return;
    }

    let expected_raw = fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "golden fixture {} unreadable ({e}); bless with SYNTHPASS_GOLDEN_RENDER_BLESS=1",
            path.display()
        )
    });
    let expected: Vec<&str> = expected_raw.lines().collect();

    for line in &actual {
        // Case key = format, seed, personal-number flag (first three columns).
        let key: String = line.splitn(4, '\t').take(3).collect::<Vec<_>>().join("\t");
        let found = expected
            .iter()
            .find(|l| l.splitn(4, '\t').take(3).collect::<Vec<_>>().join("\t") == key);
        match found {
            None => panic!("golden render case missing from fixture: {key:?}"),
            Some(exp) => assert_eq!(
                line.as_str(),
                *exp,
                "golden render mismatch for case {key:?}\n  actual:   {line}\n  expected: {exp}"
            ),
        }
    }
    assert_eq!(
        expected.len(),
        actual.len(),
        "fixture has {} lines, expected {}",
        expected.len(),
        actual.len()
    );
}
