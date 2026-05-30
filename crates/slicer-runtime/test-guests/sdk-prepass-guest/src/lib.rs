//! TASK-109 round-trip witness for the `world-prepass` world
//! (MeshAnalysis stage). Authored purely via `#[slicer_module]`.
//!
//! The guest stays minimal for the empty-config path so it still
//! satisfies the existing `prepass_world_macro_guest_*` round-trip
//! tests. When the caller sets `"emit_mesh_analysis"` on the config
//! view to a positive integer N, the guest emits N facet annotations
//! followed by one surface-group proposal per object through the SDK
//! `MeshAnalysisOutput` builder. This exercises STEP G's macro-path
//! drain: forwarded `_objects`, SDK `MeshAnalysisOutput`, and the
//! `mesh-analysis-output` WIT resource on the host side.

use slicer_sdk::error::ModuleError;
use slicer_sdk::prepass_builders::MeshAnalysisOutput;
use slicer_sdk::prepass_types::{FacetAnnotation, FacetClass, SurfaceGroupProposal};
use slicer_sdk::slicer_module;
use slicer_sdk::traits::PrepassModule;
use slicer_ir::{ConfigView, ObjectId};

pub struct SdkPrepassModule;

#[slicer_module]
impl PrepassModule for SdkPrepassModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_mesh_analysis(
        &self,
        objects: &[ObjectId],
        output: &mut MeshAnalysisOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        if let Some(code) = config.get_int("intentional_error_code") {
            return Err(ModuleError::non_fatal(
                code as u32,
                "sdk-prepass-guest: intentional typed error from config",
            ));
        }

        // When driven by the `emit_mesh_analysis` knob, emit deterministic
        // annotations and groups so the STEP G macro-path drain can be
        // observed end-to-end by the host. Emission order and field
        // values are stable so repeated runs are byte-identical.
        if let Some(n) = config.get_int("emit_mesh_analysis") {
            let n = n.max(0) as u32;
            for obj in objects {
                for i in 0..n {
                    let class = match i % 6 {
                        0 => FacetClass::Normal,
                        1 => FacetClass::NearHorizontal,
                        2 => FacetClass::Overhang,
                        3 => FacetClass::Bridge,
                        4 => FacetClass::TopSurface,
                        _ => FacetClass::BottomSurface,
                    };
                    let slope = (i as f32) * 10.0;
                    output
                        .push_facet_annotation(
                            obj.clone(),
                            FacetAnnotation { facet_index: i, slope_angle_deg: slope, classification: class },
                        )
                        .map_err(|e| ModuleError::fatal(8, e))?;
                }
                output
                    .push_surface_group(
                        obj.clone(),
                        SurfaceGroupProposal { facet_indices: (0..n).collect(), z_min: 0.0, z_max: (n as f32) * 0.2, shell_count: 2 },
                    )
                    .map_err(|e| ModuleError::fatal(9, e))?;
            }
        }

        Ok(())
    }
}
