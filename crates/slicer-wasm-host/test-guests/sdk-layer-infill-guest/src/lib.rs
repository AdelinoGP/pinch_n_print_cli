//! TASK-109 round-trip witness for the `world-layer` world
//! (Layer::Infill stage). Authored purely via `#[slicer_module]`.
//!
//! Blocker-#2 content witness: this guest proves the macro-emitted
//! layer-world glue performs real resource-level deep copy. It reads
//! the per-region fields of the incoming `SliceRegionView` values
//! (`polygons()`, `infill_areas()`, `z()`, `effective_layer_height()`)
//! and writes SDK `InfillOutputBuilder::push_sparse_path` so the host
//! can observe whether real content reached the trait body *and*
//! whether the drain-back path applied the SDK builder's output.

use slicer_ir::{ConfigView, ExtrusionPath3D, ExtrusionRole};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;
use witness::SdkInfillWitness;

pub struct SdkLayerInfillModule;

#[slicer_module]
impl LayerModule for SdkLayerInfillModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_infill(
        &self,
        layer_index: u32,
        regions: &[SliceRegionView],
        output: &mut InfillOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Preserved: the typed `ConfigView` intentional-error channel
        // (witnesses typed-config round-trip). The error message now
        // also carries the observed region count so a failing test
        // can distinguish "trait body ran but saw zero regions" from
        // "trait body did not run".
        if let Some(code) = config.get_int("intentional_error_code") {
            let region_count = regions.len();
            let total_polys: usize = regions.iter().map(|r| r.polygons().len()).sum();
            let msg = format!(
                "sdk-layer-infill-guest: typed error from config at layer {layer_index} \
                 (regions={region_count}, total_polygons={total_polys})"
            );
            return Err(ModuleError::non_fatal(code as u32, msg));
        }

        // Deep-copy input witness: encode real per-region content via
        // `witness::SdkInfillWitness` so the field meanings are defined
        // once in the witness crate (see witness/src/lib.rs).
        let (first_z, first_lh, first_infill_n) = regions
            .first()
            .map(|r| (
                r.z(),
                r.effective_layer_height(),
                r.infill_areas().len() as f32,
            ))
            .unwrap_or((0.0, 0.0, 0.0));

        let w = SdkInfillWitness {
            region_count: regions.len() as f32,
            total_polys: regions.iter().map(|r| r.polygons().len() as f32).sum(),
            first_region_z: first_z,
            first_region_layer_height: first_lh,
            first_region_infill_areas_len: first_infill_n,
        };

        let path = ExtrusionPath3D {
            points: w.encode(layer_index),
            role: ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
        };
        output
            .push_sparse_path(path)
            .map_err(|e| ModuleError::fatal(1, e))?;

        Ok(())
    }
}
