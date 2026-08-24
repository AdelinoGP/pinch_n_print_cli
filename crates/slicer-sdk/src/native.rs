//! Native entry-point envelopes for integrated SDK modules.
//!
//! These types are deliberately only available to native hosts.  A native
//! entry is a plain function pointer so the host can retain it without a
//! component instance or any scheduler-specific state.

#![cfg(not(target_arch = "wasm32"))]

use slicer_ir::ConfigView;

use crate::error::ModuleError;
use crate::postpass_types::{GcodeCommand, GcodeOutputCommand};
use crate::traits::{FinalizationOutputBuilder, LayerCollectionView, PaintRegionLayerView};
use crate::views::{PerimeterRegionView, SliceRegionView};
use crate::{builders, prepass_builders};

/// Input to a native layer-stage entry.
pub struct NativeLayerRequest {
    /// Zero-based index of the layer being processed.
    pub layer_index: u32,
    /// Sliced regions for this layer.
    pub regions: Vec<SliceRegionView>,
    /// Perimeter regions for this layer, when the stage provides them.
    pub perimeter_regions: Option<Vec<PerimeterRegionView>>,
    /// Paint-region layer view, when paint is requested.
    pub paint: Option<PaintRegionLayerView>,
    /// Infill regions carried over from a prior layer stage.
    pub prior_infill: Option<Vec<slicer_ir::InfillRegion>>,
    /// Shared configuration view across the native seam.
    pub config: ConfigView,
    /// Identifier of the stage this entry is bound to.
    pub stage_export: &'static str,
}

/// Output accumulated by a native layer-stage entry.
pub struct NativeLayerResponse {
    /// Accumulated infill output, if the stage emits one.
    pub infill: Option<builders::InfillOutputBuilder>,
    /// Accumulated perimeter output, if the stage emits one.
    pub perimeters: Option<builders::PerimeterOutputBuilder>,
    /// Accumulated support output, if the stage emits one.
    pub support: Option<builders::SupportOutputBuilder>,
    /// Accumulated slice-postprocess output, if the stage emits one.
    pub slice_postprocess: Option<builders::SlicePostprocessBuilder>,
    /// Accumulated path-optimization output, if the stage emits one.
    pub path_optimization: Option<NativePathOptimizationOutput>,
}

/// Output accumulated by a native `Layer::PathOptimization` entry.
pub struct NativePathOptimizationOutput {
    /// G-code side effects emitted by the module.
    pub output: crate::postpass_builders::GcodeOutputBuilder,
    /// Entity-order proposal emitted by the module.
    pub collection: crate::layer_collection_builder::LayerCollectionBuilder,
}

/// Input to a native prepass entry.  Only the stage-selected option is
/// populated; keeping the envelope unified mirrors the SDK trait family.
pub struct NativePrepassRequest {
    /// Mesh objects for the prepass, when this stage consumes them.
    pub mesh_objects: Option<Vec<crate::prepass_types::MeshObjectView>>,
    /// Object ids targeted by this stage, when relevant.
    pub object_ids: Option<Vec<slicer_ir::ObjectId>>,
    /// Planned layer view, when this stage works on the layer plan.
    pub layer_plan: Option<crate::prepass_types::LayerPlanView>,
    /// Region segmentation view, when this stage runs segmentation.
    pub region_segmentation: Option<crate::prepass_types::RegionSegmentationView>,
    /// Strategy-neutral support analysis, when this stage plans support.
    pub support_analysis: Option<crate::prepass_types::SupportAnalysisView>,
    /// Support geometry view, when this stage plans support.
    pub support_geometry: Option<crate::prepass_types::SupportGeometryView>,
    /// Paint segmentation objects, when this stage handles paint.
    pub paint_objects: Option<Vec<crate::prepass_types::PaintSegmentationObjectView>>,
    /// Seam planning view, when this stage plans seams.
    pub seam_regions: Option<crate::prepass_types::SeamPlanningView>,
    /// Shared configuration view across the native seam.
    pub config: ConfigView,
    /// Identifier of the stage this entry is bound to.
    pub stage_export: &'static str,
}

/// Output accumulated by a native prepass entry.
pub struct NativePrepassResponse {
    /// Accumulated mesh-analysis output, if the stage emits one.
    pub mesh_analysis: Option<prepass_builders::MeshAnalysisOutput>,
    /// Accumulated layer-plan output, if the stage emits one.
    pub layer_plan: Option<prepass_builders::LayerPlanOutput>,
    /// Accumulated paint-segmentation output, if the stage emits one.
    pub paint_segmentation: Option<prepass_builders::PaintSegmentationOutput>,
    /// Accumulated seam-planning output, if the stage emits one.
    pub seam_planning: Option<prepass_builders::SeamPlanningOutput>,
    /// Accumulated support-geometry output, if the stage emits one.
    pub support_geometry: Option<prepass_builders::SupportGeometryOutput>,
}

/// Input to either postpass method.  The selected variant is determined by
/// the stage export name; entries must reject the other variant explicitly.
pub enum NativePostpassInput {
    /// G-code command input variant.
    Gcode(Vec<GcodeCommand>),
    /// Raw text input variant.
    Text(String),
}

/// Input to a native postpass entry.
pub struct NativePostpassRequest {
    /// Postpass input; variant determined by the stage export name.
    pub input: NativePostpassInput,
    /// Shared configuration view across the native seam.
    pub config: ConfigView,
    /// Identifier of the stage this entry is bound to.
    pub stage_export: &'static str,
}

/// Output from a native postpass entry.
pub enum NativePostpassResponse {
    /// G-code output variant.
    Gcode(Vec<GcodeOutputCommand>),
    /// Raw text output variant.
    Text(String),
}

/// Input to a native finalization entry.
pub struct NativeFinalizationRequest {
    /// Collection of layers to finalize.
    pub layers: Vec<LayerCollectionView>,
    /// Shared configuration view across the native seam.
    pub config: ConfigView,
    /// Identifier of the stage this entry is bound to.
    pub stage_export: &'static str,
}

/// Output accumulated by a native finalization entry.
pub struct NativeFinalizationResponse {
    /// Accumulated finalization output.
    pub output: FinalizationOutputBuilder,
}

/// A direct native call for one SDK trait family.
#[derive(Debug, Clone, Copy)]
pub enum NativeStageEntry {
    /// Layer-stage family entry.
    Layer(fn(&NativeLayerRequest) -> Result<NativeLayerResponse, ModuleError>),
    /// Prepass-stage family entry.
    Prepass(fn(&NativePrepassRequest) -> Result<NativePrepassResponse, ModuleError>),
    /// Postpass-stage family entry.
    Postpass(fn(&NativePostpassRequest) -> Result<NativePostpassResponse, ModuleError>),
    /// Finalization-stage family entry.
    Finalization(fn(&NativeFinalizationRequest) -> Result<NativeFinalizationResponse, ModuleError>),
}
