//! Every labelled bounding box must lie within the rendered image, for every
//! ICAO 9303 format's own card canvas (TD1/TD2/TD3 each render a different
//! size, see `synthpass_gen::layout::for_format`).

use synthpass_gen::{data::generate_passport, generate, DocumentType, GeneratorConfig};

#[test]
fn all_bounding_boxes_are_within_image_bounds() {
    for seed in 0..30u64 {
        for doc_type in [DocumentType::TD1, DocumentType::TD2, DocumentType::TD3] {
            let cfg = GeneratorConfig::with_document_type(seed, doc_type);
            let passport = generate_passport(&cfg);
            let (image, labels) = generate(&passport, &cfg);
            let (width, height) = (image.width(), image.height());

            let mut rects = vec![
                labels.document_type.rect,
                labels.issuing_country.rect,
                labels.surname.rect,
                labels.given_names.rect,
                labels.document_number.rect,
                labels.nationality.rect,
                labels.date_of_birth.rect,
                labels.sex.rect,
                labels.date_of_expiry.rect,
                labels.mrz_rect,
            ];
            if let Some(pn) = &labels.personal_number {
                rects.push(pn.rect);
            }

            for r in rects {
                assert!(
                    r.x + r.width <= width && r.y + r.height <= height,
                    "seed {seed}, {doc_type:?}: rect {r:?} escapes the {width}x{height} image"
                );
            }
        }
    }
}
