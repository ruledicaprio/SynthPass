//! The provider catalog: an ordered, immutable set of providers, built once.

use std::sync::Arc;

use synthpass_core::v2::ProviderId;

use crate::provider::{Capability, CostClass, FieldReader, Recognizer};

/// Why a catalog could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogError {
    /// Two providers claimed the same [`ProviderId`].
    ///
    /// Rejected rather than last-write-wins: a silently shadowed provider is a
    /// measurement bug that surfaces weeks later as an unexplained accuracy
    /// change nobody can attribute.
    DuplicateId(ProviderId),
}

impl std::fmt::Display for CatalogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DuplicateId(id) => {
                write!(f, "two providers registered under the id `{id}`")
            }
        }
    }
}

impl std::error::Error for CatalogError {}

/// The providers available to a pipeline, in deterministic consultation order.
///
/// Named a *catalog*, not a registry: `synthpass-pipeline` already has a
/// `JobRegistry`, which is a mutable job-queue table and an entirely different
/// idea. This is an ordered list, built once and never mutated after
/// [`ProviderCatalogBuilder::build`].
///
/// # Why not `inventory`-style link-time registration
///
/// It looks attractive — providers register themselves, no wiring. Three
/// reasons it is wrong here:
///
/// 1. **New dependency, no justification available.** Both `inventory` and
///    `linkme` work through linker sections, which is exactly the machinery
///    that becomes fragile under this project's release profile
///    (`lto = true`, `codegen-units = 1`, `strip = true`) and its static musl
///    build.
/// 2. **Registration order is unspecified.** Routing order would then vary
///    between builds, and a benchmark whose providers run in a different order
///    on two runs is not a benchmark.
/// 3. **It is invisible to Cargo features.** The existing
///    `ocr-native-rust` / `inferer-native` gating would have no counterpart.
pub struct ProviderCatalog {
    recognizers: Vec<Arc<dyn Recognizer>>,
    readers: Vec<Arc<dyn FieldReader>>,
}

impl ProviderCatalog {
    pub fn builder() -> ProviderCatalogBuilder {
        ProviderCatalogBuilder::default()
    }

    /// Readers in consultation order: cheapest first, insertion order within a
    /// cost class.
    ///
    /// Deterministic by construction. Two runs over the same corpus consult the
    /// same providers in the same sequence, which is what makes a measured
    /// difference attributable to the change under test rather than to
    /// ordering.
    pub fn readers(&self) -> &[Arc<dyn FieldReader>] {
        &self.readers
    }

    /// Recognizers in consultation order, cheapest first.
    pub fn recognizers(&self) -> &[Arc<dyn Recognizer>] {
        &self.recognizers
    }

    /// The first (cheapest) recognizer, which is the only one the pipeline
    /// currently runs.
    pub fn recognizer(&self) -> Option<&Arc<dyn Recognizer>> {
        self.recognizers.first()
    }

    /// The first reader whose capability satisfies `wanted`, skipping any
    /// costing more than `budget`.
    ///
    /// Selection is by *declared capability*, never by index or by name — the
    /// property that lets a provider be swapped without touching routing.
    pub fn find_reader(
        &self,
        budget: CostClass,
        wanted: impl Fn(&Capability) -> bool,
    ) -> Option<&Arc<dyn FieldReader>> {
        self.readers
            .iter()
            .find(|p| p.capability().cost <= budget && wanted(p.capability()))
    }

    /// Every provider's id and declared capability — the "Model Capability
    /// Profile" table, for `synthpass providers` and the benchmark's report
    /// header.
    pub fn profiles(&self) -> Vec<(ProviderId, &Capability)> {
        self.recognizers
            .iter()
            .map(|p| (p.id(), p.capability()))
            .chain(self.readers.iter().map(|p| (p.id(), p.capability())))
            .collect()
    }

    /// Ids of everything registered, in consultation order.
    pub fn ids(&self) -> Vec<ProviderId> {
        self.profiles().into_iter().map(|(id, _)| id).collect()
    }
}

impl std::fmt::Debug for ProviderCatalog {
    /// Ids only. A provider's `describe()` may name a local model path, which
    /// is not something to splash into a `{:?}` in a log line.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProviderCatalog")
            .field(
                "recognizers",
                &self.recognizers.iter().map(|p| p.id()).collect::<Vec<_>>(),
            )
            .field(
                "readers",
                &self.readers.iter().map(|p| p.id()).collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// Builds a [`ProviderCatalog`], rejecting duplicate ids.
#[derive(Default)]
pub struct ProviderCatalogBuilder {
    recognizers: Vec<Arc<dyn Recognizer>>,
    readers: Vec<Arc<dyn FieldReader>>,
}

impl ProviderCatalogBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_recognizer(mut self, provider: Arc<dyn Recognizer>) -> Self {
        self.recognizers.push(provider);
        self
    }

    pub fn with_reader(mut self, provider: Arc<dyn FieldReader>) -> Self {
        self.readers.push(provider);
        self
    }

    /// Sort each list by cost (stable, so insertion order survives within a
    /// class) and reject duplicate ids across the whole catalog.
    pub fn build(self) -> Result<ProviderCatalog, CatalogError> {
        let mut seen: Vec<ProviderId> = Vec::new();
        for id in self
            .recognizers
            .iter()
            .map(|p| p.id())
            .chain(self.readers.iter().map(|p| p.id()))
        {
            if seen.contains(&id) {
                return Err(CatalogError::DuplicateId(id));
            }
            seen.push(id);
        }

        let mut recognizers = self.recognizers;
        let mut readers = self.readers;
        // `sort_by_key` is stable, which is the load-bearing part: providers of
        // equal cost keep the order they were registered in, so the sequence is
        // fully determined by the wiring rather than by the sort.
        recognizers.sort_by_key(|p| p.capability().cost);
        readers.sort_by_key(|p| p.capability().cost);

        Ok(ProviderCatalog {
            recognizers,
            readers,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{DocumentContext, IntelligenceProvider, ProviderError, Reading};

    struct StubReader {
        id: ProviderId,
        capability: Capability,
    }

    impl StubReader {
        /// Returns the erased `Arc` the catalog stores, not `Self` — every
        /// call site wants the trait object.
        fn arc(id: &'static str, cost: CostClass) -> Arc<dyn FieldReader> {
            Arc::new(Self {
                id: ProviderId(id),
                capability: if cost == CostClass::Free {
                    Capability::deterministic_reader()
                } else {
                    Capability::model_reader(cost)
                },
            })
        }
    }

    impl IntelligenceProvider for StubReader {
        fn id(&self) -> ProviderId {
            self.id
        }
        fn capability(&self) -> &Capability {
            &self.capability
        }
        fn describe(&self) -> String {
            format!("stub {}", self.id)
        }
    }

    #[async_trait::async_trait]
    impl FieldReader for StubReader {
        async fn read(&self, _ctx: &DocumentContext<'_>) -> Result<Reading, ProviderError> {
            Ok(Reading::empty(self.id))
        }
    }

    #[test]
    fn readers_are_ordered_cheapest_first() {
        let catalog = ProviderCatalog::builder()
            .with_reader(StubReader::arc("expensive", CostClass::Expensive))
            .with_reader(StubReader::arc("free", CostClass::Free))
            .with_reader(StubReader::arc("cheap", CostClass::Cheap))
            .build()
            .expect("distinct ids");

        assert_eq!(
            catalog.ids(),
            vec![
                ProviderId("free"),
                ProviderId("cheap"),
                ProviderId("expensive")
            ]
        );
    }

    /// Stable sort: equal-cost providers keep registration order, so the
    /// sequence is decided by the wiring rather than by the sort algorithm.
    #[test]
    fn equal_cost_providers_keep_registration_order() {
        let catalog = ProviderCatalog::builder()
            .with_reader(StubReader::arc("first", CostClass::Free))
            .with_reader(StubReader::arc("second", CostClass::Free))
            .with_reader(StubReader::arc("third", CostClass::Free))
            .build()
            .expect("distinct ids");

        assert_eq!(
            catalog.ids(),
            vec![
                ProviderId("first"),
                ProviderId("second"),
                ProviderId("third")
            ]
        );
    }

    /// A shadowed provider is a measurement bug that shows up much later as an
    /// unexplained accuracy change. Fail at construction instead.
    #[test]
    fn duplicate_ids_are_rejected() {
        let err = ProviderCatalog::builder()
            .with_reader(StubReader::arc("mrz", CostClass::Free))
            .with_reader(StubReader::arc("mrz", CostClass::Expensive))
            .build()
            .expect_err("a duplicate id must not build");

        assert_eq!(err, CatalogError::DuplicateId(ProviderId("mrz")));
        assert!(err.to_string().contains("mrz"));
    }

    #[test]
    fn find_reader_respects_the_cost_budget() {
        let catalog = ProviderCatalog::builder()
            .with_reader(StubReader::arc("free", CostClass::Free))
            .with_reader(StubReader::arc("pricey", CostClass::Expensive))
            .build()
            .expect("distinct ids");

        let cheap_only = catalog
            .find_reader(CostClass::Cheap, |c| c.fields)
            .expect("the free provider is within budget");
        assert_eq!(cheap_only.id(), ProviderId("free"));

        // Selection is by capability, not by name: nothing here declares
        // vision, so nothing matches at any budget.
        assert!(catalog
            .find_reader(CostClass::Expensive, |c| c.vision)
            .is_none());
    }

    #[test]
    fn debug_shows_ids_not_model_paths() {
        let catalog = ProviderCatalog::builder()
            .with_reader(StubReader::arc("free", CostClass::Free))
            .build()
            .expect("distinct ids");
        let rendered = format!("{catalog:?}");
        assert!(rendered.contains("free"));
        assert!(
            !rendered.contains("stub free"),
            "describe() must not leak in"
        );
    }
}
