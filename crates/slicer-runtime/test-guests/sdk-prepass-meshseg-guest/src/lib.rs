//! TASK-130b round-trip witness for PrePass::MeshSegmentation. Authored purely via #[slicer_module]. Sibling of sdk-prepass-guest (MeshAnalysis-only) per packet 43-rev1.

use slicer_sdk::error::ModuleError;
use slicer_sdk::prepass_builders::MeshSegmentationOutput;
use slicer_sdk::prepass_types::MeshObjectView;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::PrepassModule;
use slicer_ir::ConfigView;

pub struct SdkPrepassMeshsegModule;

#[slicer_module]
impl PrepassModule for SdkPrepassMeshsegModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_mesh_segmentation(
        &self,
        _objects: &[MeshObjectView],
        output: &mut MeshSegmentationOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        match config.get_string("fixture_case") {
            Some("marks_basic") => {
                // Mark triangle index 12 on object "obj-a" with semantic "material" / value "1".
                // The harvest assertion (AC-8) checks the marked triangle index only.
                output
                    .mark_triangle_paint("obj-a".to_string(), 12u32, "material".to_string(), "1".to_string())
                    .map_err(|e| ModuleError::from_str(&e))?;
            }

            // Default (config not set or unrecognised value): no-op.
            _ => {}
        }

        Ok(())
    }
}
