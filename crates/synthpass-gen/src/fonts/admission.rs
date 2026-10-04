//! ADR-0030 Decision 5, as amended in #708.
use super::{load_fonts, VizFont};
use crate::data::{viz_domain, VizDomain};
use crate::layout::{widest_ink_at_scale, LayoutField, ValidatedLayout};
use crate::model::{DocumentType, VizFontChoice};
use crate::render::viz_px_scale;
use ab_glyph::Font;

const FORMATS: [DocumentType; 5] = [
    DocumentType::TD1,
    DocumentType::TD2,
    DocumentType::TD3,
    DocumentType::MrvA,
    DocumentType::MrvB,
];

#[test]
fn all_viz_fonts_cover_every_domain() {
    let fonts = load_fonts().expect("embedded fonts");
    for format in FORMATS {
        for field in LayoutField::ALL {
            let Some(domain) = viz_domain(format, field) else {
                continue;
            };
            let characters: Vec<char> = match &domain {
                VizDomain::Pool(values) => values.iter().flat_map(|s| s.chars()).collect(),
                VizDomain::Positional(positions) => positions.iter().flatten().copied().collect(),
            };
            for font in VizFont::ALL {
                for &c in &characters {
                    assert_ne!(
                        fonts.viz_font(font).expect("embedded font").glyph_id(c).0,
                        0,
                        "{} {format:?} {field}: U+{:04X}",
                        font.name(),
                        c as u32
                    );
                }
            }
        }
    }
}

#[test]
fn computed_field_scales_satisfy_the_rule_and_are_maximal() {
    let fonts = load_fonts().expect("embedded fonts");
    for format in FORMATS {
        let layout = ValidatedLayout::builtin(format);
        for font in VizFont::ALL {
            let scales = layout.viz_scales(font).expect("built-in admission");
            assert!(std::ptr::eq(
                scales,
                layout.viz_scales(font).expect("cached")
            ));
            for field in LayoutField::ALL {
                let Some(domain) = viz_domain(format, field) else {
                    continue;
                };
                let rect = layout.spec().rect(field);
                let scale = scales.field(field);
                let reference = widest_ink_at_scale(&fonts.viz, &domain, rect, viz_px_scale(rect))
                    .expect("reference ink");
                let ink = widest_ink_at_scale(
                    fonts.viz_font(font).expect("embedded font"),
                    &domain,
                    rect,
                    scale.px,
                )
                .expect("font ink");
                assert!(scale.k > 0.0 && scale.k <= 1.0);
                assert!(
                    ink.min_x >= rect.x as i32
                        && ink.min_y >= rect.y as i32
                        && ink.max_x <= reference.max_x
                        && ink.max_y <= reference.max_y,
                    "{} {format:?} {field}: k={} ink={ink:?} ref={reference:?}",
                    font.name(),
                    scale.k
                );
                if scale.k < 1.0 {
                    let larger = widest_ink_at_scale(
                        fonts.viz_font(font).expect("embedded font"),
                        &domain,
                        rect,
                        (scale.k + 0.001) * viz_px_scale(rect),
                    )
                    .expect("larger ink");
                    assert!(
                        larger.max_x > reference.max_x || larger.max_y > reference.max_y,
                        "{} {format:?} {field}: k={} is not maximal",
                        font.name(),
                        scale.k
                    );
                }
            }
        }
    }
}

#[test]
fn per_field_scale_snapshot_is_exact() {
    let mut rows = Vec::new();
    for format in FORMATS {
        for field in LayoutField::ALL
            .into_iter()
            .filter(|field| field.is_visual_zone())
        {
            for font in VizFont::ALL {
                let scale = ValidatedLayout::builtin(format)
                    .viz_scales(font)
                    .expect("built-in admission")
                    .field(field);
                rows.push(format!(
                    "{}\t{}\t{}\t{:.6}\t{}",
                    format.as_str(),
                    field.name(),
                    font.name(),
                    scale.k,
                    scale.binding
                ));
            }
        }
    }
    for font in VizFont::ALL {
        let mut values: Vec<_> = FORMATS
            .into_iter()
            .flat_map(|format| {
                LayoutField::ALL
                    .into_iter()
                    .filter(|field| field.is_visual_zone())
                    .map(move |field| {
                        ValidatedLayout::builtin(format)
                            .viz_scales(font)
                            .expect("admitted")
                            .field(field)
                            .k
                    })
            })
            .collect();
        values.sort_by(f32::total_cmp);
        println!(
            "SUMMARY {} {:.6} {:.6} {:.6}",
            font.name(),
            values[0],
            (values[24] + values[25]) / 2.0,
            values[49]
        );
    }
    println!("SNAPSHOT_BEGIN\n{}\nSNAPSHOT_END", rows.join("\n"));
    assert_eq!(
        rows.join("\n"),
        include_str!("../../tests/fixtures/viz_field_scales.tsv").trim_end()
    );
}

#[test]
fn refusal_names_the_field_and_random_filters_it_out() {
    // Inject a single empty-ink domain: the reference area is undefined.
    // No production domain, font bytes or layout-validation invariant is changed.
    let layout = ValidatedLayout::builtin(DocumentType::TD2);
    let admits = |font| {
        super::scaling::compute_layout_with_domains(layout, font, |format, field| {
            if font == VizFont::LiberationMono && field == LayoutField::GivenNames {
                Some(VizDomain::Pool(vec![""]))
            } else {
                viz_domain(format, field)
            }
        })
        .map(|_| ())
    };
    let error = VizFontChoice::Font(VizFont::LiberationMono)
        .resolve_with(0, admits)
        .expect_err("refused domain");
    assert!(
        error.contains("liberation-mono") && error.contains("TD2") && error.contains("given_names"),
        "{error}"
    );
    let mut seen = std::collections::BTreeSet::new();
    let results: Vec<_> = VizFont::ALL
        .into_iter()
        .map(|font| (font, admits(font)))
        .collect();
    for seed in 0..200 {
        let font = VizFontChoice::Random
            .resolve_with(seed, |font| {
                results
                    .iter()
                    .find(|(candidate, _)| *candidate == font)
                    .expect("closed set")
                    .1
                    .clone()
            })
            .expect("remaining fonts");
        assert_ne!(font, VizFont::LiberationMono);
        seen.insert(font.name());
    }
    assert_eq!(seen.len(), 4);
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
        assert!(checked.insert(file.to_owned()), "duplicate provenance");
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
