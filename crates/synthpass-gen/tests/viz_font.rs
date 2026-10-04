//! ADR-0030: opt-in fonts preserve identity, layout and default pixels.
use sha2::{Digest, Sha256};
use synthpass_gen::{
    fonts::VizFont, generate_from_seed, DocumentType, GeneratorConfig, VizFontChoice,
};

const FORMATS: [DocumentType; 5] = [
    DocumentType::TD1,
    DocumentType::TD2,
    DocumentType::TD3,
    DocumentType::MrvA,
    DocumentType::MrvB,
];

#[test]
fn every_font_choice_parses_and_unknown_lists_valid_values() {
    for font in VizFont::ALL {
        assert_eq!(
            VizFontChoice::parse(font.name()),
            Ok(VizFontChoice::Font(font))
        );
    }
    assert_eq!(VizFontChoice::parse("random"), Ok(VizFontChoice::Random));
    let error = VizFontChoice::parse("unknown").expect_err("unknown font");
    for name in VizFont::ALL
        .map(VizFont::name)
        .into_iter()
        .chain(["random"])
    {
        assert!(error.contains(name), "{error}");
    }
}

#[test]
fn random_font_picks_are_pinned_and_cover_the_closed_set() {
    let picks: Vec<_> = (0..10)
        .map(|seed| VizFontChoice::Random.resolve(seed).name())
        .collect();
    println!("PINNED_PICKS={picks:?}");
    let mut counts = [0usize; 5];
    for seed in 0..1000 {
        let font = VizFontChoice::Random.resolve(seed);
        assert_eq!(font, VizFontChoice::Random.resolve(seed));
        counts[VizFont::ALL
            .iter()
            .position(|&f| f == font)
            .expect("closed set")] += 1;
    }
    println!("PICK_COUNTS={counts:?}");
    for font in VizFont::ALL {
        assert!((0..200).any(|seed| VizFontChoice::Random.resolve(seed) == font));
    }
    assert_eq!(
        picks,
        [
            "pt-sans",
            "liberation-serif",
            "liberation-serif",
            "source-sans-3",
            "pt-sans",
            "source-sans-3",
            "liberation-mono",
            "liberation-sans",
            "liberation-mono",
            "liberation-serif"
        ]
    );
}

#[test]
fn absent_font_labels_and_explicit_pt_sans_preserve_default_pixels() {
    for format in FORMATS {
        let mut config = GeneratorConfig::with_document_type(42, format);
        let (default_image, default_labels, default_passport) = generate_from_seed(&config);
        assert_eq!(default_labels.viz_font, None);
        config.viz_font = Some(VizFontChoice::Font(VizFont::PtSans));
        let (image, mut labels, passport) = generate_from_seed(&config);
        assert_eq!(labels.viz_font, Some("pt-sans"));
        labels.viz_font = None;
        assert_eq!(labels, default_labels);
        assert_eq!(passport, default_passport);
        assert_eq!(image.to_rgb8(), default_image.to_rgb8());
    }
}

#[test]
fn random_font_is_independent_of_identity_and_every_label_rectangle() {
    for format in FORMATS {
        for seed in 0..50 {
            let mut config = GeneratorConfig::with_document_type(seed, format);
            let (_, default_labels, default_passport) = generate_from_seed(&config);
            config.viz_font = Some(VizFontChoice::Random);
            let (_, mut labels, passport) = generate_from_seed(&config);
            assert_eq!(
                labels.viz_font,
                Some(VizFontChoice::Random.resolve(seed).name())
            );
            labels.viz_font = None;
            assert_eq!(passport, default_passport, "{format:?} seed {seed}");
            assert_eq!(labels.mrz_lines, default_labels.mrz_lines);
            assert_eq!(labels, default_labels, "{format:?} seed {seed}");
        }
    }
}

#[cfg(feature = "embedded-fonts")]
#[test]
fn liberation_mono_smallest_scale_matches_five_golden_hashes() {
    assert!(VizFont::ALL
        .iter()
        .all(|font| font.scale() >= VizFont::LiberationMono.scale()));
    let mut actual = Vec::new();
    for format in FORMATS {
        let mut config = GeneratorConfig::with_document_type(42, format);
        config.viz_font = Some(VizFontChoice::Font(VizFont::LiberationMono));
        let (image, labels, _) = generate_from_seed(&config);
        assert_eq!(labels.viz_font, Some("liberation-mono"));
        let rgb = image.to_rgb8();
        let hash: String = Sha256::digest(rgb.as_raw())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        actual.push(format!(
            "{}\t42\tliberation-mono\t{}\t{}\t{hash}",
            format.as_str(),
            rgb.width(),
            rgb.height()
        ));
    }
    println!("MONO_GOLDENS=\n{}", actual.join("\n"));
    assert_eq!(
        actual.join("\n"),
        include_str!("fixtures/golden_viz_liberation_mono.tsv").trim_end()
    );
}
