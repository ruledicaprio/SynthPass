//! Tier 2 (the LLM), wrapped as a registered [`FieldReader`] in the M7
//! provider catalog — the same treatment Tier 1's [`MrzReader`](synthpass_die::MrzReader)
//! already got.
//!
//! Lives here, not in `synthpass-die`, because it holds `Arc<dyn InferBackend>`
//! and the pipeline's own semaphore/queue-depth/metrics types — all
//! tokio-flavored, and `synthpass-die`'s CI-enforced dependency boundary
//! forbids tokio, an OCR engine, llama.cpp, and `tracing` in that crate. A
//! contract that named one engine could not be a contract for the others.

use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use synthpass_die::{
    Capability, DocumentContext, Evidence, FieldReader, IntelligenceProvider, ProviderError,
    ProviderId, Reading,
};
use tokio::sync::Semaphore;

use crate::{lift_tier2_extraction, run_tier2, InferBackend, PipelineMetrics};

/// [`ProviderId`] of the LLM field reader — matches `Method::Llm.as_str()`.
pub(crate) const LLM_PROVIDER_ID: ProviderId = ProviderId("llm");

/// Tier 2 as a [`FieldReader`]: wraps the same `Arc<dyn InferBackend>` and
/// concurrency/metrics plumbing [`crate::Pipeline::extract_via_inferer`] uses,
/// via the shared [`run_tier2`] seam — there is exactly one place that can
/// invoke the model, regardless of which caller reaches it.
pub(crate) struct LlmFieldReader {
    pub(crate) infer: Arc<dyn InferBackend>,
    pub(crate) llm_semaphore: Arc<Semaphore>,
    pub(crate) llm_queue_depth: Arc<AtomicUsize>,
    pub(crate) metrics: Arc<PipelineMetrics>,
    pub(crate) capability: Capability,
}

impl IntelligenceProvider for LlmFieldReader {
    fn id(&self) -> ProviderId {
        LLM_PROVIDER_ID
    }

    fn capability(&self) -> &Capability {
        &self.capability
    }

    fn describe(&self) -> String {
        self.infer.describe()
    }
}

#[async_trait::async_trait]
impl FieldReader for LlmFieldReader {
    /// Runs the model and lifts its result into [`Reading`]. A backend
    /// failure becomes [`ProviderError::Failed`] with the original message
    /// carried verbatim — the same text `PipelineResult.llm_error` has always
    /// reported, just relayed through a different error type.
    async fn read(&self, ctx: &DocumentContext<'_>) -> Result<Reading, ProviderError> {
        let extraction = run_tier2(
            &self.infer,
            &self.llm_semaphore,
            &self.llm_queue_depth,
            &self.metrics,
            ctx.text,
        )
        .await
        .map_err(|detail| ProviderError::Failed { detail })?;

        let v2 = lift_tier2_extraction(&extraction, self.infer.model_id());
        // `Evidence` is `#[non_exhaustive]`, so struct-literal update syntax
        // (`..Evidence::default()`) isn't available outside `synthpass-die` —
        // mutate the one field that matters instead.
        let mut evidence = Evidence::default();
        evidence.missing = v2.fields.missing();

        Ok(Reading {
            extraction: v2,
            evidence,
            by: self.id(),
        })
    }
}
