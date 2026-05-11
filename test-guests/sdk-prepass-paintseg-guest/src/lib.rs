//! TASK-130b round-trip witness for PrePass::PaintSegmentation. Authored purely via #[slicer_module]. Sibling of sdk-prepass-guest (MeshAnalysis-only) per packet 43-rev1.

use slicer_sdk::error::ModuleError;
use slicer_sdk::prepass_builders::{ExPolygonView, PaintSegmentationOutput, PaintValueInput};
use slicer_sdk::prepass_types::PaintSegmentationObjectView;
use slicer_sdk::slicer_module;
use slicer_sdk::traits::PrepassModule;
use slicer_ir::ConfigView;

pub struct SdkPrepassPaintsegModule;

#[slicer_module]
impl PrepassModule for SdkPrepassPaintsegModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_paint_segmentation(
        &self,
        _objects: &[PaintSegmentationObjectView],
        output: &mut PaintSegmentationOutput,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        match config.get_string("fixture_case") {
            Some("hole_bearing") => {
                // One region at layer 0 with one ExPolygon that has a 4-point outer
                // contour and one inner hole. Coordinates are integer mm values so
                // mm × 10_000 is always an integer multiple of 100 (100 nm units).
                let contour = vec![
                    [1.0_f64,  1.0_f64],
                    [10.0,     1.0],
                    [10.0,    10.0],
                    [1.0,     10.0],
                ];
                let hole = vec![
                    [3.0_f64, 3.0_f64],
                    [7.0,     3.0],
                    [7.0,     7.0],
                    [3.0,     7.0],
                ];
                let polygon = ExPolygonView::new(contour, vec![hole]);
                output.push_paint_region(
                    0u32,
                    "fuzzy_skin".to_string(),
                    "obj-a".to_string(),
                    0u64,
                    PaintValueInput::Custom("test-semantic|hole-bearing".to_string()),
                    vec![polygon],
                );
            }

            Some("custom_payload") => {
                // One region at layer 0 with a single minimal triangle (no holes).
                let contour = vec![
                    [1.0_f64, 1.0_f64],
                    [3.0,     1.0],
                    [3.0,     3.0],
                ];
                let polygon = ExPolygonView::new(contour, vec![]);
                output.push_paint_region(
                    0u32,
                    "fuzzy_skin".to_string(),
                    "obj-a".to_string(),
                    0u64,
                    PaintValueInput::Custom("test-semantic|DEADBEEF".to_string()),
                    vec![polygon],
                );
            }

            Some("force_push_failure") => {
                // Push a region with an empty polygon list. The host validator
                // rejects empty polygons; the macro surfaces this as fatal
                // ModuleError code 10. We do not attempt to handle it here.
                output.push_paint_region(
                    0u32,
                    "fuzzy_skin".to_string(),
                    "obj-a".to_string(),
                    0u64,
                    PaintValueInput::Custom("test-semantic|force-fail".to_string()),
                    vec![],
                );
            }

            // Default (config not set or unrecognised value): no-op.
            _ => {}
        }

        Ok(())
    }
}
