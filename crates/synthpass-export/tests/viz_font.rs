use synthpass_export::GeneratedDoc;
use synthpass_gen::{fonts::VizFont, DocumentType, GeneratorConfig, VizFontChoice};

#[test]
fn explicit_font_export_only_adds_requested_metadata() {
    for format in [
        DocumentType::TD1,
        DocumentType::TD2,
        DocumentType::TD3,
        DocumentType::MrvA,
        DocumentType::MrvB,
    ] {
        let mut config = GeneratorConfig::with_document_type(42, format);
        let default = GeneratedDoc::build_with_config(&config);
        let default_json = serde_json::to_value(&default.record).expect("JSON");
        assert!(default_json.get("viz_font").is_none());
        config.viz_font = Some(VizFontChoice::Font(VizFont::PtSans));
        let explicit = GeneratedDoc::build_with_config(&config);
        let mut json = serde_json::to_value(&explicit.record).expect("JSON");
        assert_eq!(json["viz_font"], "pt-sans");
        json.as_object_mut().expect("object").remove("viz_font");
        assert_eq!(json, default_json);
        assert_eq!(explicit.image.to_rgb8(), default.image.to_rgb8());
        config.viz_font = Some(VizFontChoice::Random);
        let random = GeneratedDoc::build_with_config(&config);
        assert_eq!(
            random.record.viz_font,
            Some(VizFontChoice::Random.resolve(42).name())
        );
    }
}
