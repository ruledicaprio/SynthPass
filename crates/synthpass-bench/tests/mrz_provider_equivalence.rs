//! Equivalence check for M6's "TD1/TD2/MRVA/MRVB as providers" criterion: for
//! a handful of seeds per ICAO 9303 format, the registered `mrz` provider
//! (`synthpass_die::MrzReader`, looked up through the catalog exactly the way
//! `synthpass-pipeline`'s `ocr_and_tier1` does) and a direct
//! `mrz::find_and_parse` call must agree on format, checksum validity, and
//! document number.
//!
//! Deliberately model-free and OCR-free: `synthpass-gen`'s ground-truth MRZ
//! text (`Labels::mrz_string()`) is fed to both sides directly, so this test
//! runs in milliseconds and never depends on the OCR engine or its models
//! being present. `provider_bench.rs`'s harness already measures the
//! OCR-in-the-loop rate; this test isolates the one claim OCR noise would
//! otherwise obscure — that the provider and the parser it wraps never
//! silently disagree.

use synthpass_die::{CostClass, DocumentContext, MrzReader, ProviderCatalog};
use synthpass_gen::data::generate_passport;
use synthpass_gen::labels::build_labels;
use synthpass_gen::model::{DocumentType, GeneratorConfig};

/// A few seeds per format — enough to exercise different identities (name
/// length, filler padding, personal-number presence) without turning this
/// into a slow sweep; `provider_bench.rs`'s corpus runs already cover volume.
const SEEDS: [u64; 3] = [1, 42, 1000];

const FORMATS: [DocumentType; 5] = [
    DocumentType::TD1,
    DocumentType::TD2,
    DocumentType::TD3,
    DocumentType::MrvA,
    DocumentType::MrvB,
];

#[tokio::test]
async fn catalog_read_and_direct_parse_agree_on_every_format_and_seed() {
    let catalog = ProviderCatalog::builder()
        .with_reader(std::sync::Arc::new(MrzReader::new()))
        .build()
        .expect("MrzReader is the only registered id");
    // The same lookup `synthpass-pipeline`'s `ocr_and_tier1` performs
    // (`find_reader(CostClass::Free, |c| c.deterministic)`).
    let reader = catalog
        .find_reader(CostClass::Free, |c| c.deterministic)
        .expect("the deterministic MRZ provider is always registered");

    let mut checked = 0usize;
    for doc_type in FORMATS {
        for seed in SEEDS {
            let config = GeneratorConfig::with_document_type(seed, doc_type);
            let passport = generate_passport(&config);
            let labels = build_labels(&passport, doc_type);
            let text = labels.mrz_string();

            let direct = mrz::find_and_parse(&text)
                .unwrap_or_else(|e| panic!("{doc_type:?} seed {seed}: direct parse failed: {e:?}"));
            assert!(
                direct.valid(),
                "{doc_type:?} seed {seed}: synthpass-gen's own ground-truth MRZ must be \
                 checksum-valid by construction"
            );

            let reading = reader
                .read(&DocumentContext::from_text(&text))
                .await
                .expect("MrzReader::read never returns Err");

            assert!(
                reading.evidence.mrz_checksums_valid,
                "{doc_type:?} seed {seed}: catalog-routed read disagreed with the direct parse \
                 on checksum validity"
            );
            assert_eq!(
                reading.evidence.mrz_format,
                Some(synthpass_die::mrz_reader::mrz_format_of(&direct)),
                "{doc_type:?} seed {seed}: catalog-routed read resolved a different format \
                 than the direct parse"
            );
            let got_number = reading
                .extraction
                .fields
                .get(synthpass_core::v2::CoreField::DocumentNumber)
                .unwrap_or_default();
            assert_eq!(
                got_number,
                direct.document_number.as_str(),
                "{doc_type:?} seed {seed}: catalog-routed read's document number disagreed \
                 with the direct parse"
            );
            checked += 1;
        }
    }
    assert_eq!(
        checked,
        FORMATS.len() * SEEDS.len(),
        "every format/seed pair must have run"
    );
}
