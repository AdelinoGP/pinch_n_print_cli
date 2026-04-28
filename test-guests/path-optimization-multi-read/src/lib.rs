//! Test guest for the macro-call-once contract.
//!
//! `run_path_optimization` here calls `LayerCollectionBuilder::get_ordered_entities`
//! exactly five times in succession and asserts that every snapshot equals
//! the first. The host runs this guest under the `#[slicer_module]` macro,
//! which is required (per docs/03_wit_and_manifest.md) to call the WIT
//! host's `get-ordered-entities` exactly once per `run-path-optimization`
//! dispatch and to serve subsequent SDK reads from a local cache. The host
//! integration test reads the
//! `slicer_host::HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS` static counter to
//! verify the call-count contract holds.
//!
//! No `set_entity_order` proposal is emitted, so the host fallback ordering
//! applies after dispatch.

use slicer_ir::ConfigView;
use slicer_sdk::error::ModuleError;
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::{OrderedEntityView, PerimeterRegionView};

pub struct PathOptimizationMultiReadGuest;

#[slicer_module]
impl LayerModule for PathOptimizationMultiReadGuest {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_path_optimization(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut GcodeOutputBuilder,
        collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        let mut snapshots: Vec<Vec<OrderedEntityView>> = Vec::with_capacity(5);
        for _ in 0..5 {
            snapshots.push(collection.get_ordered_entities().to_vec());
        }
        let first = &snapshots[0];
        for snapshot in snapshots.iter().skip(1) {
            assert_eq!(
                snapshot, first,
                "path-optimization-multi-read: snapshot drifted across calls"
            );
        }
        Ok(())
    }
}
