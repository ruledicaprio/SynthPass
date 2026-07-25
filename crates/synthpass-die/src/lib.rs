//! # Document Intelligence Engine
//!
//! The contract every intelligence provider implements — MRZ, OCR, LLM today;
//! vision, barcode, layout and whatever comes next without changing the shape.
//!
//! ## What this crate is for
//!
//! Before it, extraction was two hardcoded tiers: parse the MRZ, and if the
//! check digits fail, call the one LLM the pipeline knows the name of. That
//! works for exactly the providers it was written for. Adding TD1/TD2/MRVA/MRVB
//! means more branches; adding a barcode decoder for driving licences (which
//! carry no MRZ at all) means more branches; comparing two models on the same
//! corpus is not possible, because there is nowhere to put the second one.
//!
//! So the shape of the change is: **Tier 2 stops meaning "run the LLM" and
//! starts meaning "ask a more capable provider only for what deterministic code
//! could not recover."**
//!
//! ## The dependency list is the deliverable
//!
//! This crate depends on `synthpass-core`, `mrz`, `async-trait` and `serde`.
//! Not tokio, not an OCR engine, not llama.cpp — and CI asserts their absence.
//!
//! That is not tidiness. A contract that names one engine cannot be a contract
//! for the others, and "write a provider" must not start with "install
//! llama.cpp". The doc-test on [`FieldReader`] implements a working provider
//! in about twenty lines against this crate alone.
//!
//! ## The one thing to understand before changing anything here
//!
//! **Reported confidence and routing evidence are different things, and the
//! separation is structural.**
//!
//! [`Reading`] holds both an [`ExtractionV2`](synthpass_core::v2::ExtractionV2)
//! (the reported half — the ordinal confidence model, unchanged, part of the
//! published JSON contract) and an [`Evidence`] (the routing half). `Reading`
//! derives no `Serialize`. Neither does `Evidence`. There is therefore no code
//! path that can serialize routing signal into the document JSON, because the
//! type holding both cannot be serialized at all.
//!
//! This exists because the two are easy to conflate and expensive to confuse.
//! `synthpass_core::fusion::Support` explains the reasoning at length: SynthPass
//! has never produced a calibration curve, so a numeric confidence would be
//! "laundering a guess into a number with decimal places". A *routing* score may
//! be uncalibrated — being wrong costs CPU seconds, not truth — but it must
//! never reach a consumer looking like a confidence. Hence two types, and only
//! one of them serializable.
//!
//! See `knowledge/project_principles.md` §2.
//!
//! ## Layout
//!
//! - [`provider`] — the traits, [`Capability`], [`DocumentContext`], errors.
//! - [`evidence`] — what was observed, for routing only.
//! - [`catalog`] — the ordered, immutable provider set.
//! - [`mrz_reader`] — the deterministic ICAO 9303 provider.
//! - [`routing`] — turns [`Evidence`] into a spend decision, consulted by
//!   `synthpass-pipeline`'s Tier-1 gate.

pub mod catalog;
pub mod evidence;
pub mod mrz_reader;
pub mod provider;
pub mod routing;

pub use catalog::{CatalogError, ProviderCatalog, ProviderCatalogBuilder};
pub use evidence::Evidence;
pub use mrz_reader::{MrzReader, MRZ_PROVIDER_ID};
pub use provider::{
    Capability, CostClass, DocumentContext, FieldReader, IntelligenceProvider, ProviderError,
    Reading, Recognition, Recognizer,
};
pub use routing::{Decision, RoutingPolicy};

// Wire vocabulary lives in `synthpass-core` (the schema crate must not depend
// on this one), and is re-exported here so a provider author needs one import.
pub use synthpass_core::fusion::{FindingKind, Verdict};
pub use synthpass_core::v2::{CoreField, EscalationKind, ExtractionTrace, PromptRef, ProviderId};
