//! Module traits for SDK.
//!
//! The `LayerModule` trait is the core trait that module authors implement.
//! Per docs/05_module_sdk.md and docs/03_wit_and_manifest.md (world-layer.wit).

use crate::builders::{
    InfillOutputBuilder, PerimeterOutputBuilder, SlicePostprocessBuilder, SupportOutputBuilder,
};
use crate::error::ModuleError;
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
