//! The provider contract: identity, declared capability, and the two shapes of
//! work a provider can do.

use std::path::Path;

use serde::Serialize;
use synthpass_core::v2::{ExtractionV2, ImageRef, ProviderId};

use crate::evidence::Evidence;

/// Identity and declared capability. Object-safe, synchronous, allocation-free
/// apart from [`describe`](IntelligenceProvider::describe).
///
/// Supertrait of both work traits, so the catalog can hold heterogeneous
/// providers and still ask each one who it is without a downcast.
///
/// This trait deliberately has no `run` method. A single
/// `run(Request) -> Response` would force every provider to `match` on request
/// variants it structurally cannot serve and return "unsupported" at runtime —
/// demoting a compile-time fact to a runtime failure. MRZ, OCR and an LLM are
/// two shapes, not three: MRZ and an LLM both turn context into fields; OCR
/// turns an image into text. Hence [`Recognizer`] and [`FieldReader`].
pub trait IntelligenceProvider: Send + Sync {
    /// Stable, compile-time identity. Used as a metric label, so the set must
    /// stay closed — see [`ProviderId`].
    fn id(&self) -> ProviderId;

    /// Declared capability. Borrowed, not returned by value: capability is
    /// fixed at construction and read on every routing decision.
    fn capability(&self) -> &Capability;

    /// Short human-readable identity for logs. **Shape only, never document
    /// content** — this string is emitted verbatim into the log stream, which
    /// sits outside the trust boundary the document itself never leaves.
    fn describe(&self) -> String;

    /// Cheap preflight: are the model files present, is the engine loadable?
    ///
    /// Synchronous and cheap by contract — no model load, no inference. An
    /// expensive check (hashing a multi-gigabyte GGUF, for instance) belongs in
    /// the adapter that owns the file, not in a method the catalog may call for
    /// every provider at startup.
    fn health(&self) -> Result<(), ProviderError> {
        Ok(())
    }
}

/// Image → text and layout geometry. Today: the `ocrs`/`rten` engine, via an
/// adapter in `synthpass-pipeline`.
#[async_trait::async_trait]
pub trait Recognizer: IntelligenceProvider {
    async fn recognize(&self, image: &Path) -> Result<Recognition, ProviderError>;
}

/// Context → fields. Today: MRZ (deterministic) and the text-only LLM.
///
/// A future vision model implements *this* trait, reading
/// [`DocumentContext::image`] instead of ignoring it. That is the point of
/// passing a context struct rather than a `&str`: adding a multimodal provider
/// is a new implementation, not a new trait.
///
/// # Writing a provider out of tree
///
/// The contract needs no runtime and no engine — a third-party provider
/// compiles against `synthpass-die` alone. This is M6's *"a third-party plugin
/// builds against a stable interface following the docs"* criterion, executed
/// as a doc-test so it cannot silently stop being true.
///
/// ```
/// use synthpass_die::{
///     Capability, CostClass, DocumentContext, FieldReader, IntelligenceProvider,
///     ProviderCatalog, ProviderError, ProviderId, Reading,
/// };
///
/// /// A provider that recovers a document number from a barcode payload —
/// /// stubbed here, but the shape is real: no OCR engine, no model, no runtime.
/// struct BarcodeReader {
///     capability: Capability,
/// }
///
/// impl BarcodeReader {
///     fn new() -> Self {
///         Self {
///             // `Capability` is `#[non_exhaustive]`, so it is built rather
///             // than struct-literalled — which is what lets a new capability
///             // be added later without breaking this code.
///             capability: Capability::deterministic_reader()
///                 .with_bboxes(true),
///         }
///     }
/// }
///
/// impl IntelligenceProvider for BarcodeReader {
///     fn id(&self) -> ProviderId {
///         ProviderId("pdf417")
///     }
///     fn capability(&self) -> &Capability {
///         &self.capability
///     }
///     fn describe(&self) -> String {
///         "AAMVA PDF417 barcode reader".into()
///     }
/// }
///
/// #[async_trait::async_trait]
/// impl FieldReader for BarcodeReader {
///     async fn read(&self, ctx: &DocumentContext<'_>) -> Result<Reading, ProviderError> {
///         if ctx.text.is_empty() {
///             return Err(ProviderError::Unsupported { needs: "recognized text" });
///         }
///         Ok(Reading::empty(self.id()))
///     }
/// }
///
/// // It registers like any built-in provider.
/// let catalog = ProviderCatalog::builder()
///     .with_reader(std::sync::Arc::new(BarcodeReader::new()))
///     .build()
///     .expect("no duplicate ids");
///
/// assert_eq!(catalog.ids(), vec![ProviderId("pdf417")]);
/// assert_eq!(catalog.readers()[0].capability().cost, CostClass::Free);
/// ```
#[async_trait::async_trait]
pub trait FieldReader: IntelligenceProvider {
    async fn read(&self, ctx: &DocumentContext<'_>) -> Result<Reading, ProviderError>;
}

/// Everything a reader is allowed to look at.
///
/// Borrowed throughout: the pipeline owns the buffers and zeroizes them, and a
/// provider never takes ownership of PII. `#[non_exhaustive]` so adding a
/// signal later cannot break an out-of-tree provider — construct it with
/// [`DocumentContext::from_text`] and the `with_*` builders.
#[non_exhaustive]
pub struct DocumentContext<'a> {
    /// The source image on disk, when there is one. `None` when the caller
    /// holds only text.
    pub image: Option<&'a Path>,
    /// Recognized text. Empty when no recognizer ran.
    pub text: &'a str,
    /// Layout geometry from whichever recognizer produced `text`.
    pub recognition: Option<&'a Recognition>,
    /// What cheaper providers already established.
    ///
    /// This is what makes "ask only for what deterministic code could not
    /// recover" possible: a reader can consult
    /// [`ExtractionFields::missing`](synthpass_core::v2::ExtractionFields::missing)
    /// on the prior reading and request exactly those fields. Costs nothing to
    /// carry now; a prompt change later.
    pub prior: Option<&'a Reading>,
    /// A short, pre-rendered `k=v` hint string of checksum-verified MRZ
    /// line-1 fields, for a [`FieldReader`] to fold into its own prompt.
    ///
    /// Deliberately not sourced from [`Self::prior`]: a Tier-1 [`MrzReader`]
    /// reading reports fields only when the *whole* record validates
    /// ([`MrzData::valid`](mrz::MrzData::valid)), so a checksum-partial read
    /// (some individual check digits pass, the composite or another does
    /// not) leaves `prior` empty even though some fields are still
    /// mathematically proven. The caller that holds the raw `mrz::MrzData`
    /// builds this string directly from the individual check bits instead.
    /// Already fully rendered (not the raw `MrzData`) so this crate's
    /// dependency boundary — no `tokio`, no LLM engine, no prompt format
    /// opinions — stays intact; it is just a string.
    pub mrz_hint: Option<&'a str>,
}

impl<'a> DocumentContext<'a> {
    /// A context over recognized text alone.
    pub fn from_text(text: &'a str) -> Self {
        Self {
            image: None,
            text,
            recognition: None,
            prior: None,
            mrz_hint: None,
        }
    }

    pub fn with_image(mut self, image: &'a Path) -> Self {
        self.image = Some(image);
        self
    }

    pub fn with_recognition(mut self, recognition: &'a Recognition) -> Self {
        self.recognition = Some(recognition);
        self
    }

    pub fn with_prior(mut self, prior: &'a Reading) -> Self {
        self.prior = Some(prior);
        self
    }

    pub fn with_mrz_hint(mut self, hint: &'a str) -> Self {
        self.mrz_hint = Some(hint);
        self
    }
}

/// What a [`Recognizer`] produced.
///
/// Geometry uses [`ImageRef`] — already the schema's box type for `portrait`
/// and `barcodes[].bbox` — rather than a fourth bounding-box struct.
#[derive(Debug, Clone, Default, PartialEq)]
#[non_exhaustive]
pub struct Recognition {
    pub text: String,
    /// The region scored as the MRZ zone, if one scored confidently.
    pub mrz_band: Option<ImageRef>,
    /// How MRZ-shaped that band looked, in `[0, 1]`. A different question from
    /// *where* it was, and the one an escalation decision needs.
    pub mrz_band_score: Option<f64>,
    /// The region scored as the ID photo. **Crop coordinates only** — no face
    /// recognition, ever ([`VISION.md`]'s permanent non-goal). A bounding box
    /// is not a face.
    ///
    /// [`VISION.md`]: https://github.com/ruledicaprio/SynthPass/blob/main/knowledge/VISION.md
    pub portrait: Option<ImageRef>,
    /// Rotation in degrees clockwise applied before recognition.
    pub rotation: u16,
    /// Whole-page character-plausibility score. **Not a model confidence** —
    /// see `synthpass_ocr::geometry::text_sanity`.
    pub text_sanity: Option<f32>,
}

impl Recognition {
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            ..Self::default()
        }
    }
}

/// What a [`FieldReader`] produced: the reported record, and the routing
/// evidence gathered on the way.
///
/// **This type is the split between reported confidence and routing signal,
/// made structural.** `Reading` derives no `Serialize`, and neither does
/// [`Evidence`]. There is therefore no code path — not a careless one, not a
/// future one — that can serialize routing evidence into the document JSON,
/// because the type that holds both cannot be serialized at all. Only
/// `extraction` is serializable, and it carries the ordinal confidence model
/// exactly as it always has.
///
/// See `knowledge/project_principles.md` §2.
#[derive(Debug, Clone)]
pub struct Reading {
    /// The reported half: the existing public contract, unchanged.
    pub extraction: ExtractionV2,
    /// The routing half. Never serialized.
    pub evidence: Evidence,
    /// Which provider produced this.
    pub by: ProviderId,
}

impl Reading {
    /// A reading that established nothing — the honest result when a provider
    /// ran and found no document.
    pub fn empty(by: ProviderId) -> Self {
        Self {
            extraction: ExtractionV2::default(),
            evidence: Evidence::default(),
            by,
        }
    }

    /// Fields this reading left empty, in the schema's own vocabulary.
    pub fn missing(&self) -> Vec<synthpass_core::v2::CoreField> {
        self.extraction.fields.missing()
    }
}

/// Order-of-magnitude cost of consulting a provider.
///
/// Ordinal on purpose, and only three values wide. The routing engine needs to
/// answer "is this worth spending more" — which is a comparison, not an
/// arithmetic. A millisecond estimate would be a number nobody measured,
/// varying by an order of magnitude with hardware, and would invite exactly the
/// arithmetic on invented quantities that `synthpass_core::fusion::Support`
/// exists to refuse.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CostClass {
    /// Deterministic arithmetic over data already in memory. Always worth
    /// trying first: MRZ parsing and check-digit verification.
    #[default]
    Free,
    /// Real work, bounded and predictable — an OCR pass.
    Cheap,
    /// Model inference. Seconds to minutes, and the thing escalation exists to
    /// avoid spending unnecessarily.
    Expensive,
}

/// What a provider declares it can do.
///
/// `#[non_exhaustive]` with `Default`, so adding a capability later is additive
/// for out-of-tree providers: they construct with `..Capability::default()`.
///
/// Deliberately a struct rather than bitflags — half these are not booleans
/// (`max_context`, `estimated_resident_bytes`, `weights_license`), and
/// `bitflags` would be a new dependency for something that is not a bit set.
///
/// This is the "Model Capability Profile" from the design notes, as a type.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Capability {
    /// Consumes pixels rather than text.
    ///
    /// **`false` for every provider shipped in v1.3.0.** The existence of this
    /// field is the deliverable; a provider setting it is v1.4.0. Stated
    /// plainly so the honest claim — *the interface exists and has zero vision
    /// implementations* — stays visible in the type rather than only in a
    /// roadmap.
    pub vision: bool,
    /// Produces ICAO field values; implements [`FieldReader`].
    pub fields: bool,
    /// Produces text and layout geometry; implements [`Recognizer`].
    pub recognition: bool,
    /// Output is a mathematical consequence of the input: same input, same
    /// output, forever, with no model weights involved. True only for MRZ.
    ///
    /// The routing engine treats this as "free, and always worth trying
    /// first" — principle 1, *deterministic whenever possible*.
    pub deterministic: bool,
    /// Constrained decoding is available (GBNF or equivalent).
    pub grammar: bool,
    /// Emits per-field bounding boxes. False for everything in v1.3.0.
    pub bboxes: bool,
    /// Token budget, when the provider has one.
    pub max_context: Option<u32>,
    /// Rough resident cost when loaded, for capacity planning and the
    /// benchmark's memory column.
    ///
    /// **Declared, not measured** — it is the provider's own claim about
    /// itself. `knowledge/hardware/` holds what was actually observed. The
    /// field is named `estimated_` so a reader cannot mistake one for the
    /// other.
    pub estimated_resident_bytes: Option<u64>,
    /// SPDX identifier of the **model weights**, not of this crate's code.
    ///
    /// `deny.toml` governs crate licences; nothing in the build governs
    /// weights, and weights are what made Qwen2.5-3B unusable despite being
    /// technically attractive (`other`, "Qwen Research"). Recording it on the
    /// provider is the only place this fact has ever lived in the codebase.
    /// `None` for providers with no weights.
    pub weights_license: Option<&'static str>,
    /// How expensive it is to consult.
    pub cost: CostClass,
}

impl Capability {
    /// A deterministic field reader: free, no weights, no model.
    ///
    /// The MRZ provider's shape, and the shape of any future provider that is
    /// pure arithmetic over data already in hand (a barcode decoder, a
    /// checksum validator for another standard).
    pub fn deterministic_reader() -> Self {
        Self {
            fields: true,
            deterministic: true,
            cost: CostClass::Free,
            ..Self::default()
        }
    }

    /// A field reader backed by a model.
    pub fn model_reader(cost: CostClass) -> Self {
        Self {
            fields: true,
            cost,
            ..Self::default()
        }
    }

    /// A recognizer: image in, text and geometry out.
    pub fn recognizer(cost: CostClass) -> Self {
        Self {
            recognition: true,
            cost,
            ..Self::default()
        }
    }

    /// Declare that this provider consumes pixels.
    ///
    /// Nothing in v1.3.0 calls this. The first caller is the multimodal
    /// provider, and at that point the roadmap's claim — *the interface exists
    /// and has zero vision implementations* — stops being true and should be
    /// updated rather than quietly outgrown.
    pub fn with_vision(mut self, vision: bool) -> Self {
        self.vision = vision;
        self
    }

    /// Declare constrained (grammar-guided) decoding.
    pub fn with_grammar(mut self, grammar: bool) -> Self {
        self.grammar = grammar;
        self
    }

    /// Declare per-field bounding-box output.
    pub fn with_bboxes(mut self, bboxes: bool) -> Self {
        self.bboxes = bboxes;
        self
    }

    /// Declare a token budget.
    pub fn with_max_context(mut self, tokens: u32) -> Self {
        self.max_context = Some(tokens);
        self
    }

    /// Declare the provider's own estimate of its resident cost. Declared,
    /// never measured — see the field's doc comment.
    pub fn with_estimated_resident_bytes(mut self, bytes: u64) -> Self {
        self.estimated_resident_bytes = Some(bytes);
        self
    }

    /// Declare the SPDX licence of the **model weights**.
    ///
    /// Worth setting even when it seems obvious: weights licensing is what
    /// ruled out an otherwise attractive model, and this is the only place in
    /// the codebase that fact can live.
    pub fn with_weights_license(mut self, spdx: &'static str) -> Self {
        self.weights_license = Some(spdx);
        self
    }
}

/// Why a provider could not produce a result.
///
/// Neither `String` (which [`InferBackend`] uses) nor the pipeline's error type
/// (which `OcrEngine` uses) — the seam is the right place to stop that
/// inconsistency spreading to every future provider.
///
/// [`InferBackend`]: https://docs.rs/synthpass-pipeline
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ProviderError {
    /// Model or weights absent, wrong checksum, engine not loaded. The
    /// provider never ran.
    Unavailable { detail: String },
    /// The provider ran and failed.
    ///
    /// **`detail` is logged verbatim by the caller, so it must describe the
    /// failure without quoting document content.** "grammar-constrained decode
    /// produced no output after 3 attempts" is a good detail; anything
    /// containing a name, a document number, or recognized text is a PII leak
    /// into the log stream. See `CONTRIBUTING.md`'s PII checklist.
    Failed { detail: String },
    /// The context lacks something this provider's capability requires — no
    /// image for a vision provider, no text for a text one.
    Unsupported { needs: &'static str },
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { detail } => write!(f, "provider unavailable: {detail}"),
            Self::Failed { detail } => write!(f, "provider failed: {detail}"),
            Self::Unsupported { needs } => write!(f, "provider needs {needs}"),
        }
    }
}

impl std::error::Error for ProviderError {}
