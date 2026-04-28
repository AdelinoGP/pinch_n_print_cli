//! SDK guest builder for the `layer-collection-builder` WIT resource.
//!
//! Per docs/03_wit_and_manifest.md (ir-types.wit):
//! ```wit
//! resource layer-collection-builder {
//!     set-entity-order: func(items: list<tuple<u32, bool>>) -> result<_, string>;
//!     get-ordered-entities: func() -> list<ordered-entity-view>;
//! }
//! ```
//!
//! `Layer::PathOptimization` modules call `set_entity_order` once with a
//! permutation of the host-staged `LayerCollectionIR.ordered_entities` plus
//! an optional per-entity reversal flag (mirrors OrcaSlicer's
//! `vector<pair<size_t, bool>>` from `ShortestPath::chain_segments_closest_point`).
//! The host validates the proposal and applies it after the module returns.
//! Calling `set_entity_order` more than once within a single
//! `run-path-optimization` invocation is a contract violation.
//!
//! The `get_ordered_entities` reader exposes a per-call snapshot of the
//! host-staged ordering. The `#[slicer_module]` macro populates the
//! snapshot once at the top of the trait dispatch by calling the WIT
//! resource's `get-ordered-entities` host method, then re-reading the
//! cached `Vec<OrderedEntityView>` from the SDK builder is a cheap local
//! lookup — see the macro-call-once contract in
//! docs/03_wit_and_manifest.md.

use crate::views::OrderedEntityView;

/// SDK guest builder for `layer-collection-builder`.
///
/// Modules construct one per `run_path_optimization` call (the
/// `#[slicer_module]` macro creates and drains it). Module authors call
/// `set_entity_order` at most once and may read
/// `get_ordered_entities` any number of times — the snapshot is captured
/// once at the top of the macro's dispatch and served from the local
/// cache thereafter.
#[derive(Debug, Default)]
pub struct LayerCollectionBuilder {
    proposal: Option<Vec<(u32, bool)>>,
    ordered_entities: Vec<OrderedEntityView>,
}

impl LayerCollectionBuilder {
    /// Create an empty builder. Used by the `#[slicer_module]` macro and
    /// by tests.
    pub fn new() -> Self {
        Self {
            proposal: None,
            ordered_entities: Vec::new(),
        }
    }

    /// Declare a permutation of `LayerCollectionIR.ordered_entities`.
    ///
    /// `items` is a list of `(original_index, reverse)` tuples — exactly one
    /// entry per existing entity in the host-staged ordering. The host
    /// validates the proposal (length, range, uniqueness) after the module
    /// returns and applies it atomically; a malformed proposal causes a
    /// `FatalModule` error and leaves the staged IR untouched.
    ///
    /// Calling this method twice within one `run_path_optimization`
    /// invocation returns `Err`.
    pub fn set_entity_order(&mut self, items: Vec<(u32, bool)>) -> Result<(), String> {
        if self.proposal.is_some() {
            return Err(
                "set-entity-order called twice within one run-path-optimization".to_string(),
            );
        }
        self.proposal = Some(items);
        Ok(())
    }

    /// Read the host-staged ordering snapshot.
    ///
    /// Returns the snapshot captured by the `#[slicer_module]` macro at
    /// the top of `run-path-optimization` dispatch. Repeated calls hit
    /// the SDK-local cache (`Vec<OrderedEntityView>`) and never round-trip
    /// to the WIT host — the macro contract is to call the WIT host's
    /// `get-ordered-entities` exactly once per dispatch.
    pub fn get_ordered_entities(&self) -> &[OrderedEntityView] {
        &self.ordered_entities
    }

    /// Read the captured proposal (used by the macro drain helper).
    #[doc(hidden)]
    pub fn proposal(&self) -> Option<&[(u32, bool)]> {
        self.proposal.as_deref()
    }

    /// Populate the per-call snapshot of the host-staged ordering. Called
    /// by the `#[slicer_module]` macro at the top of `run-path-optimization`
    /// dispatch from the WIT host's `get-ordered-entities` result.
    #[doc(hidden)]
    pub fn set_ordered_entities(&mut self, snapshot: Vec<OrderedEntityView>) {
        self.ordered_entities = snapshot;
    }
}
