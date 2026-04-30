//! Module traits for SDK.
//!
//! The `LayerModule` trait is the core trait that per-layer module authors implement.
//! The `PrepassModule` trait is for prepass module authors (mesh analysis, layer planning).
//! The `FinalizationModule` trait is for layer finalization modules.
//! The `PostpassModule` trait is for postpass module authors (gcode and text postprocessing).
//! Per docs/05_module_sdk.md and docs/03_wit_and_manifest.md (world-layer.wit, world-prepass.wit, world-finalization.wit, world-postpass.wit).

use std::sync::Arc;

use crate::builders::{
    InfillOutputBuilder, PerimeterOutputBuilder, SlicePostprocessBuilder, SupportOutputBuilder,
};
use crate::error::ModuleError;
use crate::layer_collection_builder::LayerCollectionBuilder;
use crate::postpass_builders::GcodeOutputBuilder;
use crate::postpass_types::GcodeCommand;
use crate::prepass_builders::{
    LayerPlanOutput, MeshAnalysisOutput, MeshSegmentationOutput, PaintSegmentationOutput,
    SeamPlanningOutput, SupportGeometryOutput,
};
use crate::prepass_types::{
    LayerPlanView, MeshObjectView, ObjectId, PaintSegmentationObjectView, RegionSegmentationView,
    SupportGeometryView,
};
use crate::views::{PerimeterRegionView, SliceRegionView};
use slicer_ir::{
    ConfigView, ExtrusionPath3D, LayerCollectionIR, PaintRegionIR, PaintSemantic, RegionKey,
    SemVer, SemanticRegion, SupportPlanIR,
};

/// Paint region layer view for accessing painted regions.
///
/// Wraps `PaintRegionIR` data for a specific layer, providing read-only
/// access to paint region queries. Host constructs this; modules use it
/// to look up paint semantics for contour points.
#[derive(Debug, Clone)]
pub struct PaintRegionLayerView {
    layer_index: u32,
    paint_regions: Arc<PaintRegionIR>,
    support_plan: Option<Arc<SupportPlanIR>>,
}

impl PaintRegionLayerView {
    /// Create a new PaintRegionLayerView with empty paint regions (host-only, for testing).
    #[doc(hidden)]
    pub fn new(layer_index: u32) -> Self {
        Self {
            layer_index,
            paint_regions: Arc::new(PaintRegionIR {
                schema_version: SemVer {
                    major: 0,
                    minor: 1,
                    patch: 0,
                },
                per_layer: std::collections::HashMap::new(),
            }),
            support_plan: None,
        }
    }

    /// Create a new PaintRegionLayerView wrapping paint region data (host-only).
    #[doc(hidden)]
    pub fn with_paint_regions(layer_index: u32, paint_regions: Arc<PaintRegionIR>) -> Self {
        Self {
            layer_index,
            paint_regions,
            support_plan: None,
        }
    }

    /// Attach a committed `SupportPlanIR` to this layer view (host-only).
    ///
    /// `Layer::Support` modules that declare `SupportPlanIR` as a read
    /// (e.g. `tree-support`) consult the plan via
    /// [`Self::support_plan_segments_for`] to emit pre-planned branch
    /// geometry instead of running a per-layer filler.
    #[doc(hidden)]
    pub fn with_support_plan(mut self, support_plan: Arc<SupportPlanIR>) -> Self {
        self.support_plan = Some(support_plan);
        self
    }

    /// Returns the layer index.
    pub fn layer_index(&self) -> u32 {
        self.layer_index
    }

    /// Returns the paint regions for this layer and semantic.
    ///
    /// Returns an empty slice if no paint regions exist for the given semantic.
    pub fn get_regions(&self, semantic: &PaintSemantic) -> &[SemanticRegion] {
        self.paint_regions.get(self.layer_index, semantic)
    }

    /// Returns the underlying paint region IR (for direct query use).
    pub fn paint_regions(&self) -> &PaintRegionIR {
        &self.paint_regions
    }

    /// Returns all semantics that have paint data on this layer.
    pub fn semantics_on_layer(&self) -> Vec<PaintSemantic> {
        self.paint_regions
            .per_layer
            .get(&self.layer_index)
            .map(|lpm| lpm.semantic_regions.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Returns the full committed support plan, if any. Modules that want
    /// to iterate across all entries for their object should use this and
    /// filter on `global_layer_index == self.layer_index()` plus their
    /// object/region ids.
    pub fn support_plan(&self) -> Option<&Arc<SupportPlanIR>> {
        self.support_plan.as_ref()
    }

    /// Returns all pre-planned branch segments for the `(layer, object_id,
    /// region_id)` triple matching this view's `layer_index`. Returns an
    /// empty vector if no plan is committed or no entry matches.
    pub fn support_plan_segments_for(
        &self,
        object_id: &str,
        region_id: u64,
    ) -> Vec<&ExtrusionPath3D> {
        let Some(plan) = self.support_plan.as_ref() else {
            return Vec::new();
        };
        plan.entries
            .iter()
            .filter(|entry| {
                entry.global_layer_index == self.layer_index
                    && entry.object_id == object_id
                    && entry.region_id == region_id
            })
            .flat_map(|entry| entry.branch_segments.iter())
            .collect()
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
        Ok(())
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
        Ok(())
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
        Ok(())
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
        Ok(())
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
        Ok(())
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
    ///
    /// Documented eligibility precedence (docs/01_system_architecture.md
    /// §"Layer::Support" and docs/02_ir_schemas.md support precedence rules):
    /// 1. `PaintSemantic::SupportBlocker` → no support, even with enforcer.
    /// 2. `PaintSemantic::SupportEnforcer` → support generated regardless of
    ///    overhang and regardless of `needs_support`.
    /// 3. Default (no paint) → generate support iff
    ///    `SliceRegionView::needs_support()` is true (the
    ///    `SurfaceClassificationIR`-derived eligibility flag).
    fn run_support(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _paint: &PaintRegionLayerView,
        _output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run support post-processing for a layer.
    ///
    /// Per docs/03_wit_and_manifest.md (world-layer.wit):
    /// ```wit
    /// export run-support-postprocess: func(
    ///     layer-index: layer-idx,
    ///     regions: list<slice-region-view>,
    ///     output: support-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_support_postprocess(
        &self,
        _layer_index: u32,
        _regions: &[SliceRegionView],
        _output: &mut SupportOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run path optimization for a layer.
    ///
    /// Per docs/03_wit_and_manifest.md (world-layer.wit):
    /// ```wit
    /// export run-path-optimization: func(
    ///     layer-index: layer-idx,
    ///     regions: list<perimeter-region-view>,
    ///     output: gcode-output-builder,
    ///     collection: layer-collection-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_path_optimization(
        &self,
        _layer_index: u32,
        _regions: &[PerimeterRegionView],
        _output: &mut GcodeOutputBuilder,
        _collection: &mut LayerCollectionBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
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

    /// Run seam planning to compute optimal seam positions for each region.
    ///
    /// Uses the mesh from `run_mesh_segmentation` and facet annotations from
    /// `run_mesh_analysis` to score and select seam positions for each region.
    /// Default implementation does nothing.
    fn run_seam_planning(
        &self,
        _objects: &[MeshObjectView],
        _output: &mut SeamPlanningOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run support geometry to compute multi-layer organic branch geometry.
    ///
    /// Propagates contact points from overhang/bridge facets and support-enforcer
    /// paint regions top-down through the layer stack, grouping and merging via
    /// per-layer minimum spanning trees. Emits branch segments that per-layer
    /// `Layer::Support` modules (notably `tree-support`) can consume directly.
    ///
    /// Per docs/03_wit_and_manifest.md (world-prepass.wit):
    /// ```wit
    /// export run-support-geometry: func(
    ///     objects: list<mesh-object-view>,
    ///     layer-plan: layer-plan-view,
    ///     region-segmentation: region-segmentation-view,
    ///     support-geometry: support-geometry-view,
    /// ) -> support-geometry-output;
    /// ```
    ///
    /// Corresponds to `PrePass::SupportGeometry`. Default implementation returns `unimplemented`.
    fn run_support_geometry(
        &self,
        _objects: &[MeshObjectView],
        _layer_plan: &LayerPlanView,
        _region_segmentation: &RegionSegmentationView,
        _support_geometry: &SupportGeometryView,
        _output: &mut SupportGeometryOutput,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Err(ModuleError::from_str(
            "run_support_geometry is not implemented",
        ))
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
    ///     commands: list<gcode-command>,
    ///     output: gcode-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    ///
    /// Default implementation does nothing. Override if your module targets GcodePostprocess stage.
    fn run_gcode_postprocess(
        &self,
        _commands: &[GcodeCommand],
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

/// Read-only view of a completed layer for finalization modules.
///
/// Per docs/03_wit_and_manifest.md (world-finalization.wit):
/// ```wit
/// resource layer-collection-view {
///     layer-index:  func() -> layer-idx;
///     z:            func() -> f32;
///     entity-count: func() -> u32;
///     ordered-entities: func() -> list<print-entity-view>;
///     tool-changes: func() -> list<tool-change-view>;
///     z-hops: func() -> list<z-hop-view>;
/// }
/// ```
#[derive(Debug, Clone)]
pub struct LayerCollectionView {
    layer: LayerCollectionIR,
}

impl LayerCollectionView {
    /// Create a new LayerCollectionView wrapping a completed layer.
    #[doc(hidden)]
    pub fn new(layer: LayerCollectionIR) -> Self {
        Self { layer }
    }

    /// Returns the global layer index.
    pub fn layer_index(&self) -> u32 {
        self.layer.global_layer_index
    }

    /// Returns the Z height of this layer.
    pub fn z(&self) -> f32 {
        self.layer.z
    }

    /// Returns the number of extrusion entities in this layer.
    pub fn entity_count(&self) -> u32 {
        self.layer.ordered_entities.len() as u32
    }

    /// Returns tool changes in this layer as (after_entity_index, from_tool, to_tool).
    pub fn tool_changes(&self) -> &[slicer_ir::ToolChange] {
        &self.layer.tool_changes
    }

    /// Returns the ordered extrusion entities in this layer.
    pub fn ordered_entities(&self) -> &[slicer_ir::PrintEntity] {
        &self.layer.ordered_entities
    }

    /// Returns the Z hops in this layer.
    pub fn z_hops(&self) -> &[slicer_ir::ZHop] {
        &self.layer.z_hops
    }
}

/// Output builder for the finalization stage.
///
/// Per docs/03_wit_and_manifest.md (world-finalization.wit):
/// ```wit
/// resource finalization-output-builder {
///     push-entity-to-layer: func(layer-index, path, region-key) -> result<_, string>;
///     insert-synthetic-layer: func(z, paths) -> result<_, string>;
/// }
/// ```
pub struct FinalizationOutputBuilder {
    entity_pushes: Vec<(u32, ExtrusionPath3D, RegionKey)>,
    synthetic_layers: Vec<(f32, Vec<ExtrusionPath3D>)>,
}

impl FinalizationOutputBuilder {
    /// Create a new FinalizationOutputBuilder.
    pub fn new() -> Self {
        Self {
            entity_pushes: Vec::new(),
            synthetic_layers: Vec::new(),
        }
    }

    /// Append an extrusion path to an existing layer.
    pub fn push_entity_to_layer(
        &mut self,
        layer_index: u32,
        path: ExtrusionPath3D,
        region_key: RegionKey,
    ) -> Result<(), String> {
        self.entity_pushes.push((layer_index, path, region_key));
        Ok(())
    }

    /// Insert a new synthetic layer at an arbitrary Z.
    pub fn insert_synthetic_layer(
        &mut self,
        z: f32,
        paths: Vec<ExtrusionPath3D>,
    ) -> Result<(), String> {
        self.synthetic_layers.push((z, paths));
        Ok(())
    }

    /// Get all entity pushes (for testing).
    #[doc(hidden)]
    pub fn entity_pushes(&self) -> &[(u32, ExtrusionPath3D, RegionKey)] {
        &self.entity_pushes
    }

    /// Get all synthetic layers (for testing).
    #[doc(hidden)]
    pub fn synthetic_layers(&self) -> &[(f32, Vec<ExtrusionPath3D>)] {
        &self.synthetic_layers
    }
}

impl Default for FinalizationOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for FinalizationOutputBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FinalizationOutputBuilder")
            .field("entity_pushes", &self.entity_pushes.len())
            .field("synthetic_layers", &self.synthetic_layers.len())
            .finish()
    }
}

/// The trait for finalization modules.
///
/// Module authors implement this trait for PostPass::LayerFinalization stage.
/// Per docs/03_wit_and_manifest.md (world-finalization.wit):
/// - Modules receive read-only views of all completed layers
/// - Modules may append entities to existing layers or insert synthetic layers
/// - Modules are always serialized (never parallel)
///
/// Per docs/01_system_architecture.md:
/// - Modules must set `layer-parallel-safe = false` in hints
/// - Host instantiates exactly one WASM instance for finalization modules
pub trait FinalizationModule: Sized {
    /// Called once before finalization begins.
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError>;

    /// Called once after finalization completes.
    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }

    /// Run layer finalization across all completed layers.
    ///
    /// Per docs/03_wit_and_manifest.md (world-finalization.wit):
    /// ```wit
    /// export run-finalization: func(
    ///     layers: list<layer-collection-view>,
    ///     output: finalization-output-builder,
    ///     config: config-view,
    /// ) -> result<_, module-error>;
    /// ```
    fn run_finalization(
        &self,
        _layers: &[LayerCollectionView],
        _output: &mut FinalizationOutputBuilder,
        _config: &ConfigView,
    ) -> Result<(), ModuleError> {
        Ok(())
    }
}
