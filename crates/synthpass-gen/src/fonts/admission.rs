//! ADR-0030 Decision 5, as amended in #704. Share the validator's domain bound
//! and the renderer's placement, rather than approximating glyph advances.
use super::{load_fonts, VizFont};
use crate::data::{viz_domain, VizDomain};
use crate::layout::{widest_ink_at_scale, LayoutField, ValidatedLayout};
use crate::model::DocumentType;
use crate::render::viz_px_scale;
use ab_glyph::{Font, FontArc};

const FORMATS: [DocumentType; 5] = [
    DocumentType::TD1,
    DocumentType::TD2,
    DocumentType::TD3,
    DocumentType::MrvA,
    DocumentType::MrvB,
];

fn characters(domain: &VizDomain) -> Vec<char> {
    match domain {
        VizDomain::Pool(values) => values.iter().flat_map(|s| s.chars()).collect(),
        VizDomain::Positional(positions) => positions.iter().flatten().copied().collect(),
    }
}

fn assert_coverage(font: &FontArc, domain: &VizDomain, context: &str) {
    for c in characters(domain) {
        assert_ne!(
            font.glyph_id(c).0,
            0,
            "{context}: uncovered U+{:04X}",
            c as u32
        );
    }
}

#[test]
fn all_viz_fonts_cover_every_domain() {
    let fonts = load_fonts().expect("all embedded fonts load");
    for format in FORMATS {
        for field in LayoutField::ALL {
            let Some(domain) = viz_domain(format, field) else {
                continue;
            };
            for family in VizFont::ALL {
                assert_coverage(
                    fonts.viz_font(family),
                    &domain,
                    &format!("{} {format:?} {field:?}", family.name()),
                );
            }
        }
    }
}

#[test]
fn pinned_viz_scales_are_admissible_and_maximal() {
    let fonts = load_fonts().expect("all embedded fonts load");
    let mut results = Vec::new();
    for family in VizFont::ALL {
        let font = fonts.viz_font(family);
        let mut maximum = 1.0f32;
        let mut binding = "cap at 1.0".to_string();
        for format in FORMATS {
            let layout = ValidatedLayout::builtin(format);
            for field in LayoutField::ALL {
                let Some(domain) = viz_domain(format, field) else {
                    continue;
                };
                let rect = layout.spec().rect(field);
                let px = viz_px_scale(rect);
                let Some(reference) = widest_ink_at_scale(&fonts.viz, &domain, rect, px) else {
                    continue;
                };
                let ink_at =
                    |k| widest_ink_at_scale(font, &domain, rect, k * px).expect("domain has ink");
                let pinned = ink_at(family.scale());
                results.push((
                    family,
                    pinned.min_x >= rect.x as i32 && pinned.min_y >= rect.y as i32,
                    format!("{format:?} {field:?}: top/left failure {pinned:?}, rect {rect:?}"),
                ));
                for (edge, bound) in [("right", reference.max_x), ("bottom", reference.max_y)] {
                    let extent = |k| {
                        let ink = ink_at(k);
                        if edge == "right" {
                            ink.max_x
                        } else {
                            ink.max_y
                        }
                    };
                    let mut low = 0.0001f32;
                    let mut high = 1.0f32;
                    assert!(extent(low) <= bound, "no positive admissible scale");
                    if extent(high) <= bound {
                        continue;
                    }
                    for _ in 0..24 {
                        let mid = (low + high) / 2.0;
                        if extent(mid) <= bound {
                            low = mid;
                        } else {
                            high = mid;
                        }
                    }
                    if low < maximum {
                        maximum = low;
                        binding = format!("{format:?} {field:?} {edge}");
                    }
                }
                let ink = ink_at(family.scale());
                // Record violations after measuring all families, so a diagnostic
                // run prints the whole table rather than just its first failure.
                results.push((
                    family,
                    ink.max_x <= reference.max_x && ink.max_y <= reference.max_y,
                    format!("{format:?} {field:?}: {ink:?}, PT {reference:?}"),
                ));
            }
        }
        println!(
            "{} pinned {:.6}, maximum {:.6}, binding {binding}",
            family.name(),
            family.scale(),
            maximum
        );
        results.push((
            family,
            family.scale() > 0.0 && family.scale() <= maximum && maximum - family.scale() <= 0.01,
            format!("maximum {maximum}, binding {binding}"),
        ));
    }
    for (family, passes, context) in results {
        assert!(
            passes,
            "{} pinned {}: {context}",
            family.name(),
            family.scale()
        );
    }
}

#[test]
fn font_sha256_matches_provenance() {
    use sha2::{Digest, Sha256};
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fonts");
    let readme = std::fs::read_to_string(directory.join("README.md")).expect("provenance README");
    let mut checked = std::collections::BTreeSet::new();
    for line in readme
        .lines()
        .filter(|line| line.starts_with("| `") && line.contains(".ttf`"))
    {
        let columns: Vec<_> = line.split('|').map(str::trim).collect();
        let file = columns[1].trim_matches('`');
        let bytes = std::fs::read(directory.join(file)).expect("font bytes");
        let digest: String = Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        assert_eq!(digest, columns[4].trim_matches('`'), "{file}");
        assert_eq!(bytes.len().to_string(), columns[5], "{file} size");
        assert!(
            checked.insert(file.to_owned()),
            "duplicate provenance entry"
        );
    }
    let files: std::collections::BTreeSet<_> = std::fs::read_dir(directory)
        .expect("fonts directory")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".ttf"))
        .collect();
    assert_eq!(checked, files, "every TTF needs one provenance record");
}
