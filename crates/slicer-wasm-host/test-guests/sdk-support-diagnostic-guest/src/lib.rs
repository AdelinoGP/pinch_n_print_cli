use slicer_sdk::error::ModuleError;
use slicer_sdk::prelude::*;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::PrepassModule;

pub struct SdkSupportDiagnosticGuest;

#[slicer_module]
impl PrepassModule for SdkSupportDiagnosticGuest {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_support_geometry(
        &self,
        _objects: &[MeshObjectView],
        _layer_plan: &LayerPlanView,
        _region_segmentation: &RegionSegmentationView,
        _support_geometry: &SupportGeometryView,
        output: &mut SupportGeometryOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        output
            .push_diagnostic(Diagnostic {
                severity: DiagnosticSeverity::Warn,
                code: 99,
                layer: Some(-1),
                object_id: Some("cube".to_string()),
                message: "round-trip".to_string(),
            })
            .map_err(|e| ModuleError::fatal(1, e))?;

        output
            .push_support_plan_entry(SupportPlanEntry {
                global_layer_index: 0,
                object_id: "cube".to_string(),
                region_id: "0".to_string(),
                branch_segments: Vec::new(),
            })
            .map_err(|e| ModuleError::fatal(2, e))?;

        Ok(())
    }
}
