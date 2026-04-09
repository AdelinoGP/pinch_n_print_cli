//! Module traits for SDK.
//!
//! The `LayerModule` trait is the core trait that per-layer module authors implement.
//! The `PrepassModule` trait is for prepass module authors (mesh analysis, layer planning).
//! The `PostpassModule` trait is for postpass module authors (gcode and text postprocessing).
//! Per docs/05_module_sdk.md and docs/03_wit_and_manifest.md (world-layer.wit, world-prepass.wit, world-postpass.wit).

use crate::builders::{
    InfillOutputBuilder, PerimeterOutputBuilder, SlicePostprocessBuilder, SupportOutputBuilder,
};
use crate::error::ModuleError;
use crate::postpass_builders::GcodeOutputBuilder;
use crate::postpass_types::GcodeCommandView;
use crate::prepass_builders::{
    LayerPlanOutput, MeshAnalysisOutput, MeshSegmentationOutput, PaintSegmentationOutput,
};
use crate::prepass_types::{MeshObjectView, ObjectId, PaintSegmentationObjectView};
use crate::views::{PerimeterRegionView, SliceRegionView};
use slicer_ir::ConfigView;

/// Paint region layer view for accessing painted regions.
///
/// This is a placeholder type that will be connected to PaintRegionIR.
#[derive(Debug, Clone)]
pub struct PaintRegionLayerView {
    layer_index: u32,
}

impl PaintRegionLayerView {
    /// Create a new PaintRegionLayerView (host-only).
    #[doc(hidden)]
    pub fn new(layer_index: u32) -> Self {
        Self { layer_index }
    }

    /// Returns the layer index.
    pub fn layer_index(&self) -> u32 {
        self.layer_index
    }
}

/// The core trait for per-layer modules.
///
/// Module authors implement this trait and annotate with `#[slicer_module]`.
/// Per docs/05_module_sdk.md:
/// - `on_print_start` is called once before the per-layer loop
/// - `on_print_end` is called after all layers are processed
/// - Exactly one of the `run_*` methods should be implemented based on manifest stage
///
/// Per docs/03_wit_and_manifest.md (world-layer.wit), this maps to:
/// - `export on-print-start: func(config: config-view) -> result<_, module-error>;`
/// - `export on-print-end: func() -> result<_, module-error>;`
/// - Stage-specific exports (run_infill, run_perimeters, etc.)
pub trait LayerModule: Sized {
    /// Called once before the per-layer loop starts.
    ///
    /// Use this to validate config and initialize expensive resources.
    /// Returns Self on success, or a fatal ModuleError on failure.
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError>;

    /// Called once after all layers are processed.
    ///
    /// Use this for cleanup. Default implementation does nothing.
    /// Note: This is best-effort cleanup; correctness must not depend on it
    /// running after a fatal abort.
    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run infill generation for a layer.
    ///
    /// Per docs/03_wit_and_manifest.md (world-layer.wit):
    /// ```wit
    /// export run-infill: func(
    ///     layer-index: layer-idx,
    ///     regions: list<slice-region-view>,
    ///     output: infill-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_infill(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        todo!("TASK-042: implement LayerModule::run_infill default")
    }

    /// Run perimeter generation for a layer.
    ///
    /// Per docs/03_wit_and_manifest.md (world-layer.wit):
    /// ```wit
    /// export run-perimeters: func(
    ///     layer-index: layer-idx,
    ///     regions: list<slice-region-view>,
    ///     paint: paint-region-layer-view,
    ///     output: perimeter-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_perimeters(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        todo!("TASK-042: implement LayerModule::run_perimeters default")
    }

    /// Run wall post-processing for a layer.
    ///
    /// Per docs/03_wit_and_manifest.md (world-layer.wit):
    /// ```wit
    /// export run-wall-postprocess: func(
    ///     layer-index: layer-idx,
    ///     regions: list<perimeter-region-view>,
    ///     output: perimeter-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_wall_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut PerimeterOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        todo!("TASK-042: implement LayerModule::run_wall_postprocess default")
    }

    /// Run infill post-processing for a layer.
    ///
    /// Per docs/03_wit_and_manifest.md (world-layer.wit):
    /// ```wit
    /// export run-infill-postprocess: func(
    ///     layer-index: layer-idx,
    ///     regions: list<perimeter-region-view>,
    ///     output: infill-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_infill_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut InfillOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        todo!("TASK-042: implement LayerModule::run_infill_postprocess default")
    }

    /// Run slice post-processing for a layer.
    ///
    /// Per docs/03_wit_and_manifest.md (world-layer.wit):
    /// ```wit
    /// export run-slice-postprocess: func(
    ///     layer-index: layer-idx,
    ///     regions: list<slice-region-view>,
    ///     paint: paint-region-layer-view,
    ///     output: slice-postprocess-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_slice_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut SlicePostprocessBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        todo!("TASK-042: implement LayerModule::run_slice_postprocess default")
    }

    /// Run support generation for a layer.
    ///
    /// Per docs/03_wit_and_manifest.md (world-layer.wit):
    /// ```wit
    /// export run-support: func(
    ///     layer-index: layer-idx,
    ///     regions: list<slice-region-view>,
    ///     paint: paint-region-layer-view,
    ///     output: support-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_support(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        todo!("TASK-042: implement LayerModule::run_support default")
    }
}

/// The trait for prepass modules.
///
/// Module authors implement this trait for mesh analysis and layer planning stages.
/// Per docs/05_module_sdk.md and docs/03_wit_and_manifest.md (world-prepass.wit):
/// - `on_print_start` is called once before prepass stages
/// - `on_print_end` is called after prepass stages complete
/// - `run_mesh_analysis` is for MeshAnalysis stage modules
/// - `run_layer_planning` is for LayerPlanning stage modules
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit), this maps to:
/// - `export run-mesh-analysis: func(objects, output, config) -> result<_, module-error>;`
/// - `export run-layer-planning: func(objects, output, config) -> result<_, module-error>;`
pub trait PrepassModule: Sized {
    /// Called once before prepass stages start.
    ///
    /// Use this to validate config and initialize expensive resources.
    /// Returns Self on success, or a fatal ModuleError on failure.
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError>;

    /// Called once after prepass stages complete.
    ///
    /// Use this for cleanup. Default implementation does nothing.
    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run mesh analysis for the given objects.
    ///
    /// Per docs/03_wit_and_manifest.md (world-prepass.wit):
    /// ```wit
    /// export run-mesh-analysis: func(
    ///     objects: list<object-id>,
    ///     output: mesh-analysis-output,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    ///
    /// Default implementation does nothing. Override if your module targets MeshAnalysis stage.
    fn run_mesh_analysis(
        &self,
        _objects: &[ObjectId],
        _output: &mut MeshAnalysisOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run layer planning for the given objects.
    ///
    /// Per docs/03_wit_and_manifest.md (world-prepass.wit):
    /// ```wit
    /// export run-layer-planning: func(
    ///     objects: list<object-id>,
    ///     output: layer-plan-output,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    ///
    /// Default implementation does nothing. Override if your module targets LayerPlanning stage.
    fn run_layer_planning(
        &self,
        _objects: &[ObjectId],
        _output: &mut LayerPlanOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run mesh segmentation to normalize sub-facet paint strokes.
    ///
    /// Clips triangles at paint stroke boundaries so each triangle carries
    /// exactly one paint value per semantic. Default implementation does nothing.
    fn run_mesh_segmentation(
        &self,
        _objects: &[MeshObjectView],
        _output: &mut MeshSegmentationOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run paint segmentation to project 3D painted facets into 2D per-layer regions.
    ///
    /// Receives objects with paint layers, transform matrices, and participating
    /// layer indices. Produces 2D polygon regions grouped by layer, semantic,
    /// object, value, and paint order. Default implementation does nothing.
    fn run_paint_segmentation(
        &self,
        _objects: &[PaintSegmentationObjectView],
        _output: &mut PaintSegmentationOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}

/// The trait for postpass modules.
///
/// Module authors implement this trait for gcode and text postprocessing stages.
/// Per docs/05_module_sdk.md and docs/03_wit_and_manifest.md (world-postpass.wit):
/// - `on_print_start` is called once before postpass stages
/// - `on_print_end` is called after postpass stages complete
/// - `run_gcode_postprocess` is for GcodePostprocess stage modules
/// - `run_text_postprocess` is for TextPostprocess stage modules
///
/// Per docs/03_wit_and_manifest.md (world-postpass.wit), this maps to:
/// - `export run-gcode-postprocess: func(commands, output, config) -> result<_, module-error>;`
/// - `export run-text-postprocess: func(gcode-text, config) -> result<string, module-error>;`
pub trait PostpassModule: Sized {
    /// Called once before postpass stages start.
    ///
    /// Use this to validate config and initialize expensive resources.
    /// Returns Self on success, or a fatal ModuleError on failure.
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError>;

    /// Called once after postpass stages complete.
    ///
    /// Use this for cleanup. Default implementation does nothing.
    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run GCode postprocessing on the command list.
    ///
    /// Per docs/03_wit_and_manifest.md (world-postpass.wit):
    /// ```wit
    /// export run-gcode-postprocess: func(
    ///     commands: list<gcode-command-view>,
    ///     output: gcode-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    ///
    /// Default implementation does nothing. Override if your module targets GcodePostprocess stage.
    fn run_gcode_postprocess(
        &self,
        _commands: &[GcodeCommandView],
        _output: &mut GcodeOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run text postprocessing on the final GCode text.
    ///
    /// Per docs/03_wit_and_manifest.md (world-postpass.wit):
    /// ```wit
    /// export run-text-postprocess: func(
    ///     gcode-text: string,
    ///     config: config-view,
    /// ) -> result<string, module-error>;
    /// ```
    ///
    /// Default implementation returns the input unchanged. Override if your module targets TextPostprocess stage.
    fn run_text_postprocess(
        &self,
        gcode_text: &str,
        _config: &ConfigView,
    ) -> Result<String, ModuleError> {
        Ok(gcode_text.to_string())
    }
}
