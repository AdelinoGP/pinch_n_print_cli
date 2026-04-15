//! TASK-109 round-trip witness for the `world-finalization` world.
//! Authored purely via `#[slicer_module]` — no hand-rolled
//! `wit_bindgen::generate!` or `export!(Component)` block.

use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point3WithWidth, RegionKey};
use slicer_sdk::error::ModuleError;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::{FinalizationModule, FinalizationOutputBuilder, LayerCollectionView};
use slicer_ir::ConfigView;

pub struct SdkFinalizationModule;

#[slicer_module]
impl FinalizationModule for SdkFinalizationModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_finalization(
        &self,
        layers: &[LayerCollectionView],
        output: &mut FinalizationOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // Intentional-error path: preserved from the earlier TASK-109
        // round-trip step so a test can still assert the typed
        // `ModuleError { code, fatal, message }` marshalling.
        if let Some(code) = config.get_int("intentional_error_code") {
            return Err(ModuleError::non_fatal(
                code as u32,
                "sdk-finalization-guest: intentional typed error from config",
            ));
        }

        // Deep-copy input witness: emit one synthetic extrusion per
        // observed layer encoding the observed `(layer_index, z,
        // entity_count, tool_changes.len())` into a deterministic
        // `Point3WithWidth` tuple. A host test can then assert the
        // synthesised entity's numbers match the source
        // `LayerCollectionIR`, which is only possible if the input
        // deep-copy pipeline actually forwarded real per-layer data
        // from the wit-bindgen resource accessors.
        for layer in layers {
            let tc = layer.tool_changes();
            let marker = ExtrusionPath3D {
                points: vec![Point3WithWidth {
                    x: layer.layer_index() as f32,
                    y: layer.z(),
                    z: layer.entity_count() as f32,
                    width: tc.len() as f32,
                    flow_factor: 1.0,
                }],
                role: ExtrusionRole::Custom("task-109-finalization-witness".into()),
                speed_factor: 1.0,
            };
            output
                .push_entity_to_layer(
                    layer.layer_index(),
                    marker,
                    RegionKey {
                        global_layer_index: layer.layer_index(),
                        object_id: "__task109_fin_witness__".into(),
                        region_id: 109,
                    },
                )
                .map_err(|e| ModuleError::fatal(1, e))?;
        }

        // Synthetic-layer witness: if the config requests it, emit one
        // synthetic layer at the documented `synthetic_layer_z` so the
        // drain-back path gets exercised even without existing layers.
        if let Some(z) = config.get_float("synthetic_layer_z") {
            let synth = ExtrusionPath3D {
                points: vec![Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: z as f32,
                    width: 0.4,
                    flow_factor: 1.0,
                }],
                role: ExtrusionRole::Custom("task-109-finalization-synth".into()),
                speed_factor: 1.0,
            };
            output
                .insert_synthetic_layer(z as f32, vec![synth])
                .map_err(|e| ModuleError::fatal(2, e))?;
        }

        Ok(())
    }
}
