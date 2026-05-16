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

use slicer_ir::{ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

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

        // Deep-copy input witness: encode real per-region content into
        // one sparse-path extrusion so host-side arena commit can read
        // it back out and prove deep-copy IN + drain-back OUT both
        // crossed the macro-emitted boundary.
        //
        //   point[0].x = region_count
        //   point[0].y = total polygon count across regions
        //   point[0].z = first region's z (or 0.0 if none)
        //   point[0].width = first region's effective_layer_height (or 0.0)
        //   point[0].flow_factor = first region's infill_areas().len() as f32
        //   point[1].x = the forwarded layer_index (proves the typed u32 arrives
        //               independently of any SliceRegionView metadata)
        let region_count = regions.len() as f32;
        let total_polys: f32 = regions.iter().map(|r| r.polygons().len() as f32).sum();
        let (first_z, first_lh, first_infill_n) = regions
            .first()
            .map(|r| (
                r.z(),
                r.effective_layer_height(),
                r.infill_areas().len() as f32,
            ))
            .unwrap_or((0.0, 0.0, 0.0));

        let path = ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: region_count,
                    y: total_polys,
                    z: first_z,
                    width: first_lh,
                    flow_factor: first_infill_n,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: layer_index as f32,
                    y: 0.0,
                    z: 0.0,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
            ],
            role: ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
        };
        output
            .push_sparse_path(path)
            .map_err(|e| ModuleError::fatal(1, e))?;

        Ok(())
    }
}
