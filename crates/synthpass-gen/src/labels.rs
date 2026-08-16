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
    /// The ICAO 9303 MRZ format this document was built as — the join key
    /// every downstream consumer (sidecar JSON, the bench corpus, the
    /// Tier-1 gate) reads to separate TD1/TD2/TD3, since
    /// [`FieldLabel::value`] on `document_type` (the MRZ document *code*,
    /// `"I"`/`"P"`) cannot: TD1 and TD2 both correctly emit `"I"`. See
    /// [`crate::model::DocumentType::document_code`].
    pub mrz_format: DocumentType,
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
    let page = crate::layout::for_format(doc_type);
    let mrz_line_strings = build_mrz_lines(passport, doc_type);
    let mrz_rects = &page.mrz_lines;

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
        document_type: FieldLabel::new(passport.document_type.clone(), page.document_type),
        issuing_country: FieldLabel::new(passport.issuing_country.clone(), page.issuing_country),
        surname: FieldLabel::new(passport.surname.clone(), page.surname),
        given_names: FieldLabel::new(passport.given_names.clone(), page.given_names),
        document_number: FieldLabel::new(passport.document_number.clone(), page.document_number),
        nationality: FieldLabel::new(passport.nationality.clone(), page.nationality),
        date_of_birth: FieldLabel::new(iso_date(&passport.date_of_birth), page.date_of_birth),
        sex: FieldLabel::new(passport.sex.as_mrz_char().to_string(), page.sex),
        date_of_expiry: FieldLabel::new(iso_date(&passport.date_of_expiry), page.date_of_expiry),
        personal_number: passport
            .personal_number
            .as_ref()
            .map(|v| FieldLabel::new(v.clone(), page.personal_number)),
        mrz_lines: mrz_line_strings,
        mrz_rect,
        mrz_format: doc_type,
    }
}

fn iso_date(date: &mrz::Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::generate_passport;
    use crate::model::GeneratorConfig;

    #[test]
    fn all_label_rects_within_image_bounds() {
        use crate::model::DocumentType;

        for seed in 0..20u64 {
            // Test all document types. Each needs its own passport, generated
            // for that doc_type: `build_labels`/`build_mrz_lines` debug-assert
            // that `passport.document_type` agrees with the requested
            // `doc_type`'s MRZ document code, and `generate_passport` is what
            // sets that field correctly per doc_type.
            for doc_type in [
                DocumentType::TD1,
                DocumentType::TD2,
                DocumentType::TD3,
                DocumentType::MrvA,
                DocumentType::MrvB,
            ] {
                let p = generate_passport(&GeneratorConfig::with_document_type(seed, doc_type));
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
                let page = crate::layout::for_format(doc_type);
                for r in rects {
                    assert!(
                        r.within_bounds(page.width, page.height),
                        "{r:?} out of bounds for {doc_type:?}'s {}x{} canvas",
                        page.width,
                        page.height
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
        assert_eq!(labels.mrz_format, DocumentType::TD3);

        // Test TD2 — needs its own passport (see `build_mrz_lines`'s
        // debug_assert on `passport.document_type`).
        let p_td2 = generate_passport(&GeneratorConfig::with_document_type(7, DocumentType::TD2));
        let labels_td2 = build_labels(&p_td2, DocumentType::TD2);
        assert_eq!(labels_td2.mrz_lines.len(), 2);
        assert_eq!(labels_td2.mrz_lines[0].len(), 36);
        assert_eq!(labels_td2.mrz_lines[1].len(), 36);
        assert_eq!(labels_td2.mrz_format, DocumentType::TD2);

        // Test TD1
        let p_td1 = generate_passport(&GeneratorConfig::with_document_type(7, DocumentType::TD1));
        let labels_td1 = build_labels(&p_td1, DocumentType::TD1);
        assert_eq!(labels_td1.mrz_lines.len(), 3);
        assert_eq!(labels_td1.mrz_lines[0].len(), 30);
        assert_eq!(labels_td1.mrz_lines[1].len(), 30);
        assert_eq!(labels_td1.mrz_lines[2].len(), 30);
        assert_eq!(labels_td1.mrz_format, DocumentType::TD1);

        // Test MRV-A
        let p_mrva = generate_passport(&GeneratorConfig::with_document_type(7, DocumentType::MrvA));
        let labels_mrva = build_labels(&p_mrva, DocumentType::MrvA);
        assert_eq!(labels_mrva.mrz_lines.len(), 2);
        assert_eq!(labels_mrva.mrz_lines[0].len(), 44);
        assert_eq!(labels_mrva.mrz_lines[1].len(), 44);
        assert_eq!(labels_mrva.mrz_format, DocumentType::MrvA);

        // Test MRV-B
        let p_mrvb = generate_passport(&GeneratorConfig::with_document_type(7, DocumentType::MrvB));
        let labels_mrvb = build_labels(&p_mrvb, DocumentType::MrvB);
        assert_eq!(labels_mrvb.mrz_lines.len(), 2);
        assert_eq!(labels_mrvb.mrz_lines[0].len(), 36);
        assert_eq!(labels_mrvb.mrz_lines[1].len(), 36);
        assert_eq!(labels_mrvb.mrz_format, DocumentType::MrvB);
    }

    /// The debug assertion in `build_mrz_lines` is the guardrail that stops
    /// the labels/pixels divergence bug (a `Passport` carrying `"P"` silently
    /// rendering TD1's `"I"` under a `"P"` label) from being reintroduced. In
    /// a debug build, mismatched input must panic rather than silently
    /// diverge.
    #[test]
    #[should_panic(expected = "disagrees with the requested")]
    #[cfg(debug_assertions)]
    fn mismatched_document_type_panics_in_debug() {
        use crate::model::DocumentType;

        // TD3 passport ("P") pushed through TD1 assembly ("I" expected).
        let p = generate_passport(&GeneratorConfig::new(1));
        let _ = build_labels(&p, DocumentType::TD1);
    }
}
