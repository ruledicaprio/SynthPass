//! Ground-truth labels: every field's exact string value plus its bounding
//! box, known ahead of drawing since the layout is a set of fixed constants —
//! so labels are 100% accurate by construction, never inferred after the fact.

use crate::layout::Rect;
use crate::model::{DocumentType, Passport};
use crate::mrz_line::build_mrz_lines;

/// One labelled field: the ground-truth text and where it was drawn.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldLabel {
    pub value: String,
    pub rect: Rect,
}

impl FieldLabel {
    fn new(value: impl Into<String>, rect: Rect) -> Self {
        Self {
            value: value.into(),
            rect,
        }
    }
}

/// Ground truth for one generated document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Labels {
    pub document_type: FieldLabel,
    pub issuing_country: FieldLabel,
    pub surname: FieldLabel,
    pub given_names: FieldLabel,
    pub document_number: FieldLabel,
    pub nationality: FieldLabel,
    pub date_of_birth: FieldLabel,
    pub sex: FieldLabel,
    pub date_of_expiry: FieldLabel,
    pub personal_number: Option<FieldLabel>,
    /// MRZ lines for the document (2 for TD2/TD3, 3 for TD1).
    pub mrz_lines: Vec<String>,
    /// Bounding box of the full MRZ band.
    pub mrz_rect: Rect,
}

impl Labels {
    /// The full MRZ as newline-joined lines.
    pub fn mrz_string(&self) -> String {
        self.mrz_lines.join("\n")
    }
}

/// Build the ground-truth [`Labels`] for `passport`, using the fixed
/// [`crate::layout`] rectangles. The MRZ lines come from
/// [`crate::mrz_line::build_mrz_lines`] — the single source of truth also
/// used by the renderer, so the drawn text and the label always agree.
pub fn build_labels(passport: &Passport, doc_type: DocumentType) -> Labels {
    use crate::layout::*;

    let mrz_line_strings = build_mrz_lines(passport, doc_type);
    let mrz_rects = crate::layout::mrz_lines(doc_type);
    
    // Calculate the full MRZ bounding box
    let first_line = &mrz_rects[0];
    let last_line = &mrz_rects[mrz_rects.len() - 1];
    let mrz_rect = Rect::new(
        first_line.x,
        first_line.y,
        first_line.width,
        (last_line.y + last_line.height) - first_line.y,
    );

    Labels {
        document_type: FieldLabel::new(passport.document_type.clone(), DOCUMENT_TYPE),
        issuing_country: FieldLabel::new(passport.issuing_country.clone(), ISSUING_COUNTRY),
        surname: FieldLabel::new(passport.surname.clone(), SURNAME),
        given_names: FieldLabel::new(passport.given_names.clone(), GIVEN_NAMES),
        document_number: FieldLabel::new(passport.document_number.clone(), DOCUMENT_NUMBER),
        nationality: FieldLabel::new(passport.nationality.clone(), NATIONALITY),
        date_of_birth: FieldLabel::new(iso_date(&passport.date_of_birth), DATE_OF_BIRTH),
        sex: FieldLabel::new(passport.sex.as_mrz_char().to_string(), SEX),
        date_of_expiry: FieldLabel::new(iso_date(&passport.date_of_expiry), DATE_OF_EXPIRY),
        personal_number: passport
            .personal_number
            .as_ref()
            .map(|v| FieldLabel::new(v.clone(), PERSONAL_NUMBER)),
        mrz_lines: mrz_line_strings,
        mrz_rect,
    }
}

fn iso_date(date: &mrz::Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::generate_passport;
    use crate::layout::{IMAGE_HEIGHT, IMAGE_WIDTH};
    use crate::model::GeneratorConfig;

    #[test]
    fn all_label_rects_within_image_bounds() {
        use crate::model::DocumentType;
        
        for seed in 0..20u64 {
            let p = generate_passport(&GeneratorConfig::new(seed));
            // Test all document types
            for doc_type in [DocumentType::TD1, DocumentType::TD2, DocumentType::TD3] {
                let labels = build_labels(&p, doc_type);
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
                        r.within_bounds(IMAGE_WIDTH, IMAGE_HEIGHT),
                        "{r:?} out of bounds for {doc_type:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn mrz_lines_match_document_type() {
        use crate::model::DocumentType;
        
        // Test TD3 (default)
        let p = generate_passport(&GeneratorConfig::new(7));
        let labels = build_labels(&p, DocumentType::TD3);
        assert_eq!(labels.mrz_lines.len(), 2);
        assert_eq!(labels.mrz_lines[0].len(), 44);
        assert_eq!(labels.mrz_lines[1].len(), 44);
        
        // Test TD2
        let labels_td2 = build_labels(&p, DocumentType::TD2);
        assert_eq!(labels_td2.mrz_lines.len(), 2);
        assert_eq!(labels_td2.mrz_lines[0].len(), 36);
        assert_eq!(labels_td2.mrz_lines[1].len(), 36);
        
        // Test TD1
        let labels_td1 = build_labels(&p, DocumentType::TD1);
        assert_eq!(labels_td1.mrz_lines.len(), 3);
        assert_eq!(labels_td1.mrz_lines[0].len(), 30);
        assert_eq!(labels_td1.mrz_lines[1].len(), 30);
        assert_eq!(labels_td1.mrz_lines[2].len(), 30);
    }
}
