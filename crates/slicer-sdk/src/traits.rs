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
    LayerPlanOutput, MeshAnalysisOutput, PaintSegmentationOutput, SeamPlanningOutput,
    SupportGeometryOutput,
};
use crate::prepass_types::{
    LayerPlanView, MeshObjectView, ObjectId, PaintSegmentationObjectView, RegionSegmentationView,
    SupportGeometryView,
};
use crate::views::{PerimeterRegionView, SliceRegionView};
use slicer_ir::{
    ConfigView, ExPolygon, ExtrusionPath3D, LayerAnnotation, LayerAnnotationKind,
    LayerCollectionIR, PaintSemantic, PrintEntity, RegionKey, SliceIR, SupportPlanIR,
};

/// Support-paint policy for a per-region eligibility decision.
///
/// Computed from D14 `SlicedRegion.segment_annotations` queries.  Used by both
/// `tree-support` and `traditional-support` modules to honour SupportEnforcer
/// (force-on) and SupportBlocker (force-off) paint annotations, with
/// blocker > enforcer precedence (docs/10 §"Scenario Trace 2").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportPaintPolicy {
    /// At least one SupportBlocker annotation covers this region — skip support
    /// regardless of overhang-angle or `needs_support`.
    Blocked,
    /// At least one SupportEnforcer annotation covers this region (and no
    /// blocker) — generate support regardless of overhang-angle or
    /// `needs_support`.
    Enforced,
    /// No paint policy override — defer to overhang-angle / `needs_support`.
    DefaultEligible,
}

/// Paint region layer view.
///
/// In packet 95 the v1 PaintRegionIR/SemanticRegion types were deleted (D8).
/// Per-layer paint annotations now travel inline on `SliceIR.regions[*].segment_annotations`
/// (D14).  This view carries a reference to the current layer's `SliceIR` so
/// support modules can query enforcer/blocker policy via `paint_policy_for`.
#[derive(Debug, Clone)]
pub struct PaintRegionLayerView {
    layer_index: u32,
    support_plan: Option<Arc<SupportPlanIR>>,
    slice_ir: Option<Arc<SliceIR>>,
}

impl PaintRegionLayerView {
    /// Create a new PaintRegionLayerView (host-only, for testing).
    #[doc(hidden)]
    pub fn new(layer_index: u32) -> Self {
        Self {
            layer_index,
            support_plan: None,
            slice_ir: None,
        }
    }

    /// Compatibility constructor — paint_regions argument is ignored; body is no-op (AC-16).
    #[doc(hidden)]
    #[allow(unused_variables)]
    pub fn with_paint_regions(layer_index: u32, _paint_regions: std::sync::Arc<()>) -> Self {
        Self {
            layer_index,
            support_plan: None,
            slice_ir: None,
        }
    }

    /// Attach a committed `SupportPlanIR` to this layer view (host-only).
    #[doc(hidden)]
    pub fn with_support_plan(mut self, support_plan: Arc<SupportPlanIR>) -> Self {
        self.support_plan = Some(support_plan);
        self
    }

    /// Attach a committed `SliceIR` to this layer view (host-only).  Required
    /// for `paint_policy_for` to surface enforcer/blocker decisions.
    #[doc(hidden)]
    pub fn with_slice_ir(mut self, slice_ir: Arc<SliceIR>) -> Self {
        self.slice_ir = Some(slice_ir);
        self
    }

    /// Returns the layer index.
    pub fn layer_index(&self) -> u32 {
        self.layer_index
    }

    /// Returns the attached `SliceIR`, if any.  Hosts must call
    /// `with_slice_ir` before dispatching a support layer for paint annotations
    /// to be visible to module bodies.
    pub fn slice_ir(&self) -> Option<&Arc<SliceIR>> {
        self.slice_ir.as_ref()
    }

    /// Returns all semantics that have paint data on this layer.  Computed
    /// from `SliceIR.regions[*].segment_annotations` when a SliceIR is
    /// attached; empty otherwise.
    pub fn semantics_on_layer(&self) -> Vec<PaintSemantic> {
        let Some(slice) = self.slice_ir.as_ref() else {
            return Vec::new();
        };
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for region in &slice.regions {
            for (sem, perimeters) in &region.segment_annotations {
                if perimeters
                    .iter()
                    .any(|edges| edges.iter().any(|v| v.is_some()))
                    && seen.insert(sem.clone())
                {
                    out.push(sem.clone());
                }
            }
        }
        out
    }

    /// Returns the full committed support plan, if any.
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
                entry.global_layer_index == self.layer_index as i32
                    && entry.object_id == object_id
                    && entry.region_id == region_id
            })
            .flat_map(|entry| entry.branch_segments.iter())
            .collect()
    }

    /// Compute the support paint policy for `expoly` at this layer.
    ///
    /// D14 contract: walks `SliceIR.regions[*]`; for any region whose polygon
    /// covers `expoly`'s centroid, checks `segment_annotations[SupportBlocker]`
    /// and `segment_annotations[SupportEnforcer]` for any `Some(value)` entry.
    /// Blocker wins over enforcer (docs/10 §"Scenario Trace 2").
    ///
    /// Returns `DefaultEligible` when no SliceIR is attached, when no region
    /// covers `expoly`, or when no enforcer/blocker annotations are present.
    pub fn paint_policy_for(&self, expoly: &ExPolygon) -> SupportPaintPolicy {
        let Some(slice) = self.slice_ir.as_ref() else {
            return SupportPaintPolicy::DefaultEligible;
        };
        let probe = expolygon_centroid(expoly);
        let mut blocked = false;
        let mut enforced = false;
        for region in &slice.regions {
            if !regions_cover_point(&region.polygons, probe) {
                continue;
            }
            if annotations_have_some(
                region
                    .segment_annotations
                    .get(&PaintSemantic::SupportBlocker),
            ) {
                blocked = true;
            }
            if annotations_have_some(
                region
                    .segment_annotations
                    .get(&PaintSemantic::SupportEnforcer),
            ) {
                enforced = true;
            }
        }
        if blocked {
            SupportPaintPolicy::Blocked
        } else if enforced {
            SupportPaintPolicy::Enforced
        } else {
            SupportPaintPolicy::DefaultEligible
        }
    }
}

#[inline]
fn annotations_have_some(entries: Option<&Vec<Vec<Option<slicer_ir::PaintValue>>>>) -> bool {
    let Some(entries) = entries else { return false };
    entries
        .iter()
        .any(|perim| perim.iter().any(|v| v.is_some()))
}

/// Average of the outer contour points in integer coordinate space.  Sufficient
/// as a point-in-polygon probe for the convex / near-convex paint regions
/// produced by `cells_to_expolygons_by_color`; modifier-volume polygons are also
/// near-convex by construction (slices of axis-aligned solids).
fn expolygon_centroid(expoly: &ExPolygon) -> slicer_ir::Point2 {
    let pts = &expoly.contour.points;
    if pts.is_empty() {
        return slicer_ir::Point2 { x: 0, y: 0 };
    }
    let mut sx: i64 = 0;
    let mut sy: i64 = 0;
    for p in pts {
        sx += p.x;
        sy += p.y;
    }
    let n = pts.len() as i64;
    slicer_ir::Point2 {
        x: sx / n,
        y: sy / n,
    }
}

fn regions_cover_point(polygons: &[ExPolygon], pt: slicer_ir::Point2) -> bool {
    polygons.iter().any(|p| point_in_expolygon(pt, p))
}

/// Ray-cast point-in-polygon (with hole subtraction).
fn point_in_expolygon(pt: slicer_ir::Point2, ep: &ExPolygon) -> bool {
    if !point_in_polygon(pt, &ep.contour.points) {
        return false;
    }
    for hole in &ep.holes {
        if point_in_polygon(pt, &hole.points) {
            return false;
        }
    }
    true
}

fn point_in_polygon(pt: slicer_ir::Point2, ring: &[slicer_ir::Point2]) -> bool {
    if ring.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = ring.len() - 1;
    for i in 0..ring.len() {
        let pi = &ring[i];
        let pj = &ring[j];
        // The `(pi.y > pt.y) != (pj.y > pt.y)` precondition guarantees
        // `pj.y - pi.y != 0`, so the division below is safe.
        let pi_above = pi.y > pt.y;
        let pj_above = pj.y > pt.y;
        if pi_above != pj_above {
            let cross = (pj.x as i128 - pi.x as i128) * (pt.y as i128 - pi.y as i128)
                / (pj.y as i128 - pi.y as i128)
                + pi.x as i128;
            if (pt.x as i128) < cross {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
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
    /// Uses facet annotations from `run_mesh_analysis` to score and select
    /// seam positions for each region. Default implementation does nothing.
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

// ---------------------------------------------------------------------------
// FinalizationOutputBuilder — output builder for the finalization stage.
//
// Per docs/03_wit_and_manifest.md (world-finalization.wit):
//   resource finalization-output-builder {
//       push-entity-to-layer: func(layer-index, path, region-key) -> result<_, string>;
//       insert-synthetic-layer: func(z, paths) -> result<_, string>;
//   }
//
// Packet-40 extension: four new methods (`push_entity_with_priority`,
// `modify_entity`, `sort_layer_by`, `insert_synthetic_layer_after`) plus the
// merge applier `apply_to` that drives the full host-side merge sequence.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Step-2 types: EntityMutation, SortKey, SyntheticLayerData
// These are the locked variant lists from packet 41. Step 3 will wire them
// into FinalizationOutputBuilder's API; for now they just need to exist and
// compile so that the SDK test file can import them.
// ---------------------------------------------------------------------------

/// A discrete mutation to apply to a `PrintEntity` during finalization merge.
#[derive(Debug, Clone)]
pub enum EntityMutation {
    /// Set the path-level speed factor for an entity.
    SetSpeedFactor(f32),
    /// Set the flow factor for every point on an entity's path.
    SetFlowFactor(f32),
}

/// Ordering key for sorting entities within a layer during finalization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    /// Sort by priority first, then by entity id (stable tiebreak).
    ByPriorityAndEntityId,
    /// Sort by entity id only.
    ByEntityId,
    /// Sort by object id, then by priority within the same object.
    /// Resolves the per-entity object id via `PrintEntity.region_key.object_id`
    /// (PrintEntity carries no direct `object_id` field).
    ByObjectIdThenPriority,
}

/// A fully-formed synthetic layer ready to be inserted into the layer collection.
#[derive(Debug, Clone)]
pub struct SyntheticLayerData {
    /// Z coordinate of the synthetic layer (same unit system as the rest of the IR).
    pub z: f32,
    /// Extrusion paths that make up this synthetic layer.
    pub paths: Vec<ExtrusionPath3D>,
}

// Internal operation log — deferred until apply_to() is called.

/// A pending mutation recorded by the builder and applied in `apply_to`.
#[derive(Debug)]
pub enum MergeOp {
    /// Mutate a single entity by id.
    ModifyEntity {
        /// Layer index where the target entity lives.
        layer: u32,
        /// Stable entity identifier within that layer.
        entity_id: u64,
        /// Mutation to apply to the matched entity.
        mutation: EntityMutation,
    },
    /// Sort an entire layer's entity vec by a named key.
    SortLayer {
        /// Layer index whose entities are to be reordered.
        layer: u32,
        /// Sort key applied via stable sort.
        key: SortKey,
    },
    /// Insert a synthetic layer after `idx` in the outer Vec.
    InsertSynthLayer {
        /// Insert the synthetic layer immediately after this layer index.
        idx: u32,
        /// Minimal payload (`z` plus extrusion paths) used to construct the new layer.
        data: SyntheticLayerData,
    },
    /// Insert a new entity at a specific positional index within an existing layer.
    ///
    /// Entities at indices >= `position` shift right by 1. All positional
    /// after_entity_index references (ToolChange, ZHop, TravelRetract) with
    /// `after_entity_index >= position` are incremented by 1. Bounds-checked
    /// at apply_to time; out-of-bounds `position` returns Err atomically.
    InsertEntityAt {
        /// Global layer index of the target layer.
        layer: u32,
        /// Index at which to insert the new entity (0 = prepend).
        position: u32,
        /// Extrusion path for the new entity.
        path: ExtrusionPath3D,
        /// Tool/extruder selector for the new entity (explicit since the split).
        tool_index: u32,
        /// Region key for the new entity (pure identity).
        region_key: RegionKey,
    },
    /// Reorder all entities in a layer according to a permutation.
    ///
    /// `items[k] = (old_index, reversed)` means the entity originally at `old_index`
    /// moves to position `k`. The items must form a valid permutation of `0..N`
    /// where `N == ordered_entities.len()`. After reordering, all positional
    /// after_entity_index references are remapped through the inverse permutation.
    /// Validated atomically at apply_to time.
    SetEntityOrder {
        /// Global layer index of the target layer.
        layer: u32,
        /// Permutation: items[new_position] = (original_position, reversed).
        items: Vec<(u32, bool)>,
    },
}

/// Priority-aware entity push record (Packet-40).
struct PriorityPush {
    layer_index: u32,
    path: ExtrusionPath3D,
    /// Tool/extruder selector for the pushed entity (explicit since the split).
    tool_index: u32,
    region_key: RegionKey,
    priority: u32,
}

/// Output builder for the finalization stage (Packet-40 extended form).
pub struct FinalizationOutputBuilder {
    /// Legacy pushes — backcompat accessor `entity_pushes()` returns this slice.
    entity_pushes: Vec<(u32, ExtrusionPath3D, RegionKey)>,
    /// Synthetic layer fragments from the old WIT-level `insert_synthetic_layer`.
    synthetic_layers: Vec<(f32, Vec<ExtrusionPath3D>)>,
    /// Priority-aware pushes from `push_entity_with_priority`.
    priority_pushes: Vec<PriorityPush>,
    /// Deferred operations applied by `apply_to`.
    merge_ops: Vec<MergeOp>,
    /// Per-layer annotations (raw GCode / comments) to splice into the emitted output.
    annotations: Vec<(u32, LayerAnnotation)>,
    /// Layer indices already permuted via `set_entity_order` within this builder's
    /// lifetime. Enforces the packet-58 locked invariant "single permutation per
    /// layer per `run_finalization` invocation".
    permuted_layers: std::collections::HashSet<u32>,
}

impl FinalizationOutputBuilder {
    /// Create a new FinalizationOutputBuilder.
    pub fn new() -> Self {
        Self {
            entity_pushes: Vec::new(),
            synthetic_layers: Vec::new(),
            priority_pushes: Vec::new(),
            merge_ops: Vec::new(),
            annotations: Vec::new(),
            permuted_layers: std::collections::HashSet::new(),
        }
    }

    /// Append an extrusion path to an existing layer (legacy alias; priority = 0).
    ///
    /// This is preserved for backcompat. It delegates to `push_entity_with_priority`
    /// with priority = 0, which sorts to the front (lowest value = highest urgency).
    #[inline]
    pub fn push_entity_to_layer(
        &mut self,
        layer_index: u32,
        path: ExtrusionPath3D,
        tool_index: u32,
        region_key: RegionKey,
    ) -> Result<(), String> {
        // Keep the legacy slice intact so `entity_pushes()` callers remain correct.
        self.entity_pushes
            .push((layer_index, path.clone(), region_key.clone()));
        // Also record as a priority push so apply_to can include it in the merge.
        self.priority_pushes.push(PriorityPush {
            layer_index,
            path,
            tool_index,
            region_key,
            priority: 0,
        });
        Ok(())
    }

    /// Append an extrusion path to an existing layer with an explicit merge priority.
    ///
    /// Lower `priority` values sort earlier in the merged layer; equal priorities
    /// preserve insertion order (stable sort).
    ///
    /// Records to BOTH `entity_pushes` (legacy 3-tuple, for `entity_pushes()` backcompat)
    /// and `priority_pushes` (priority-aware record, for `apply_to`).
    pub fn push_entity_with_priority(
        &mut self,
        layer_index: u32,
        path: ExtrusionPath3D,
        tool_index: u32,
        region_key: RegionKey,
        priority: u32,
    ) -> Result<(), String> {
        // Mirror into the legacy slice so `entity_pushes()` returns ALL pushes.
        self.entity_pushes
            .push((layer_index, path.clone(), region_key.clone()));
        self.priority_pushes.push(PriorityPush {
            layer_index,
            path,
            tool_index,
            region_key,
            priority,
        });
        Ok(())
    }

    /// Record a discrete mutation to apply to a single `PrintEntity` (identified
    /// by `entity_id`) in `layer_index`.  The mutation is deferred and applied by
    /// `apply_to`; if the entity is not found at that time an error is returned.
    pub fn modify_entity(
        &mut self,
        layer_index: u32,
        entity_id: u64,
        mutation: EntityMutation,
    ) -> Result<(), String> {
        self.merge_ops.push(MergeOp::ModifyEntity {
            layer: layer_index,
            entity_id,
            mutation,
        });
        Ok(())
    }

    /// Record a sort key for an entire layer's entity vec.
    ///
    /// The layer is stable-sorted ascending by the named key during `apply_to`.
    pub fn sort_layer_by(&mut self, layer_index: u32, key: SortKey) -> Result<(), String> {
        self.merge_ops.push(MergeOp::SortLayer {
            layer: layer_index,
            key,
        });
        Ok(())
    }

    /// Record the insertion of a synthetic layer immediately after position `idx`
    /// in the outer `Vec<LayerCollectionIR>`.  Bounds checking is deferred to
    /// `apply_to`; the `LayerCollectionIR` is constructed from `data` at apply time.
    pub fn insert_synthetic_layer_after(
        &mut self,
        idx: u32,
        data: SyntheticLayerData,
    ) -> Result<(), String> {
        self.merge_ops.push(MergeOp::InsertSynthLayer { idx, data });
        Ok(())
    }

    /// Insert a new synthetic layer at an arbitrary Z (legacy WIT method).
    pub fn insert_synthetic_layer(
        &mut self,
        z: f32,
        paths: Vec<ExtrusionPath3D>,
    ) -> Result<(), String> {
        self.synthetic_layers.push((z, paths));
        Ok(())
    }

    /// Insert a new entity at a specific positional index within an existing layer.
    ///
    /// Records the operation as a deferred `MergeOp::InsertEntityAt`. Bounds
    /// validation (`position <= ordered_entities.len()`) and index-remap of
    /// ToolChange/ZHop/TravelRetract references are performed by `apply_to`.
    pub fn insert_entity_at(
        &mut self,
        layer_index: u32,
        position: u32,
        path: ExtrusionPath3D,
        tool_index: u32,
        region_key: RegionKey,
    ) -> Result<(), String> {
        self.merge_ops.push(MergeOp::InsertEntityAt {
            layer: layer_index,
            position,
            path,
            tool_index,
            region_key,
        });
        Ok(())
    }

    /// Return the staged ordered entities for `layer_index`, reflecting every
    /// push / insert / permutation recorded so far against the supplied
    /// `initial` layer set.
    ///
    /// This is the read-back side of the packet-58 locked invariant
    /// "`get-ordered-entities` reflects staged state". The method simulates
    /// the builder's recorded actions against the initial layer's
    /// `ordered_entities` without mutating either the builder or the layer:
    ///
    /// 1. `push_entity_*` calls are appended in record order (priority sorting
    ///    is intentionally NOT simulated — that is `apply_to`'s job).
    /// 2. `insert_entity_at` insertions are applied at the recorded position
    ///    (clamped to the current staged length).
    /// 3. `set_entity_order` permutations are applied when well-formed;
    ///    malformed proposals are skipped (they will fail at `apply_to` time).
    /// 4. Synthetic entities receive unique `entity_id` values monotonically
    ///    above the max id in the initial layer.
    pub fn get_ordered_entities(
        &self,
        layer_index: u32,
        initial: &[LayerCollectionIR],
    ) -> Vec<slicer_ir::PrintEntity> {
        let mut staged = initial
            .iter()
            .find(|l| l.global_layer_index == layer_index)
            .map(|l| l.ordered_entities.clone())
            .unwrap_or_default();

        let mut next_id: u64 = staged.iter().map(|e| e.entity_id).max().unwrap_or(0) + 1;

        let make_entity = |id: u64,
                           path: &ExtrusionPath3D,
                           tool_index: u32,
                           region_key: &RegionKey|
         -> slicer_ir::PrintEntity {
            slicer_ir::PrintEntity {
                entity_id: id,
                path: path.clone(),
                role: path.role.clone(),
                // Tool is an explicit selector; region_id stays a pure identity.
                tool_index,
                region_key: region_key.clone(),
                topo_order: 0,
            }
        };

        // 1. Replay pushes (push_entity_to_layer / push_entity_with_priority).
        //    `priority_pushes` mirrors every push and carries the explicit
        //    tool_index, so iterating it covers both methods in insertion order.
        for push in &self.priority_pushes {
            if push.layer_index == layer_index {
                staged.push(make_entity(
                    next_id,
                    &push.path,
                    push.tool_index,
                    &push.region_key,
                ));
                next_id += 1;
            }
        }

        // 2. Replay deferred merge_ops in record order.
        for op in &self.merge_ops {
            match op {
                MergeOp::InsertEntityAt {
                    layer,
                    position,
                    path,
                    tool_index,
                    region_key,
                } if *layer == layer_index => {
                    let pos = (*position as usize).min(staged.len());
                    staged.insert(pos, make_entity(next_id, path, *tool_index, region_key));
                    next_id += 1;
                }
                MergeOp::SetEntityOrder { layer, items } if *layer == layer_index => {
                    if items.len() != staged.len() {
                        continue;
                    }
                    let mut seen = vec![false; staged.len()];
                    let mut valid = true;
                    for &(idx, _) in items {
                        let i = idx as usize;
                        if i >= staged.len() || seen[i] {
                            valid = false;
                            break;
                        }
                        seen[i] = true;
                    }
                    if !valid {
                        continue;
                    }
                    let original = staged.clone();
                    staged = items
                        .iter()
                        .map(|&(old_idx, _)| original[old_idx as usize].clone())
                        .collect();
                }
                _ => {}
            }
        }

        staged
    }

    /// Reorder all entities in a layer according to a permutation.
    ///
    /// Records the operation as a deferred `MergeOp::SetEntityOrder`. Permutation
    /// validation and index-remap of ToolChange/ZHop/TravelRetract references are
    /// performed by `apply_to`. Returns `Err` if this layer has already been
    /// permuted within this builder's lifetime (packet-58 locked invariant:
    /// single permutation per layer per `run_finalization` invocation).
    pub fn set_entity_order(
        &mut self,
        layer_index: u32,
        items: Vec<(u32, bool)>,
    ) -> Result<(), String> {
        if !self.permuted_layers.insert(layer_index) {
            return Err(format!(
                "set_entity_order called twice for layer {layer_index} within one run-finalization"
            ));
        }
        self.merge_ops.push(MergeOp::SetEntityOrder {
            layer: layer_index,
            items,
        });
        Ok(())
    }

    /// Push a layer annotation (comment or raw GCode line).
    pub fn push_annotation(
        &mut self,
        layer_index: u32,
        annotation: LayerAnnotation,
    ) -> Result<(), String> {
        self.annotations.push((layer_index, annotation));
        Ok(())
    }

    /// Push a fan speed command as a raw GCode annotation.
    ///
    /// `value` 0 emits `M107`; any other value emits `M106 S{value}`.
    pub fn push_fan_speed(&mut self, layer_index: u32, value: u8) -> Result<(), String> {
        let text = if value == 0 {
            "M107".to_string()
        } else {
            format!("M106 S{}", value)
        };
        self.annotations.push((
            layer_index,
            LayerAnnotation {
                after_entity_index: 0,
                kind: LayerAnnotationKind::Raw(text),
            },
        ));
        Ok(())
    }

    /// Get all entity pushes (for testing / wit_host backcompat).
    ///
    /// Returns ALL pushes regardless of which recording method was used
    /// (`push_entity_to_layer` or `push_entity_with_priority`).  The priority
    /// field is an internal detail and is not exposed here.  Use `apply_to` for
    /// the authoritative priority-sorted merged result.
    #[doc(hidden)]
    pub fn entity_pushes(&self) -> &[(u32, ExtrusionPath3D, RegionKey)] {
        &self.entity_pushes
    }

    /// Get all synthetic layers (for testing).
    #[doc(hidden)]
    pub fn synthetic_layers(&self) -> &[(f32, Vec<ExtrusionPath3D>)] {
        &self.synthetic_layers
    }

    /// Get all annotations (for testing).
    #[doc(hidden)]
    pub fn annotations(&self) -> &[(u32, LayerAnnotation)] {
        &self.annotations
    }

    /// Get all priority-aware pushes as flat tuples
    /// `(layer_index, path, tool_index, region_key, priority)`.
    ///
    /// Includes ALL pushes regardless of recording method: `push_entity_to_layer` records
    /// at priority=0, `push_entity_with_priority` records at the given priority.
    /// Used by the slicer-macros drain-back loop to relay pushes across the WIT boundary
    /// with their correct tool index and priorities.
    #[doc(hidden)]
    pub fn priority_pushes(
        &self,
    ) -> impl Iterator<Item = (u32, &ExtrusionPath3D, u32, &RegionKey, u32)> {
        self.priority_pushes.iter().map(|p| {
            (
                p.layer_index,
                &p.path,
                p.tool_index,
                &p.region_key,
                p.priority,
            )
        })
    }

    /// Get all recorded merge operations (for drain-back in slicer-macros Step 5).
    ///
    /// Returns an iterator over all deferred `MergeOp` records in record order.
    /// The macro drain-back loop (Step 5) iterates over this to relay operations
    /// across the WIT boundary.
    #[doc(hidden)]
    pub fn merge_ops(&self) -> impl Iterator<Item = &MergeOp> {
        self.merge_ops.iter()
    }

    // -----------------------------------------------------------------------
    // apply_to — full host-side merge sequence
    // -----------------------------------------------------------------------

    /// Apply all recorded pushes and deferred operations to `layers`.
    ///
    /// Order of operations (matches packet-40 spec):
    /// 1. Append priority-aware entity pushes per layer, stamp entity_ids.
    /// 2. Stable-sort each modified layer by (effective_priority, original_index).
    /// 3. Apply `ModifyEntity` ops in record order.
    /// 4. Apply `SortLayer` ops in record order.
    /// 5. Apply `InsertSynthLayer` ops in record order.
    pub fn apply_to(self, layers: &mut Vec<LayerCollectionIR>) -> Result<(), String> {
        // ---- Phase 1 & 2: push entities + priority sort per layer ----------
        //
        // We operate on each layer that has at least one priority_push.
        // (Legacy entity_pushes are ALSO in priority_pushes with priority=0.)
        // To avoid double-applying, entity_pushes is not iterated again here.

        // Group pushes by layer_index.
        // We need them ordered as (layer_index, original_insertion_index) so
        // that stable sort tiebreaks by insertion order.
        for push in &self.priority_pushes {
            // Find the target layer by global_layer_index.
            let layer = layers
                .iter_mut()
                .find(|l| l.global_layer_index == push.layer_index);
            let layer = match layer {
                Some(l) => l,
                None => continue, // layer not found → skip silently (matches dispatch.rs behaviour)
            };

            // Compute next entity_id = max existing + 1, or 1 if empty.
            let next_id = layer
                .ordered_entities
                .iter()
                .map(|e| e.entity_id)
                .max()
                .map(|m| m + 1)
                .unwrap_or(1);

            let role = push.path.role.clone();
            let new_entity = PrintEntity {
                entity_id: next_id,
                path: push.path.clone(),
                role,
                tool_index: push.tool_index,
                region_key: push.region_key.clone(),
                topo_order: layer.ordered_entities.len() as u32,
            };
            layer.ordered_entities.push(new_entity);
        }

        // Phase 2: stable-sort each layer that received priority pushes.
        //
        // Strategy: for each entity in ordered_entities, compute effective
        // priority = its role's default_priority() for pre-existing entities,
        // or the explicit priority for newly pushed entities.
        //
        // We need to know which entity_ids were just pushed and at what
        // priority.  Build a map from entity_id → explicit_priority for the
        // newly appended entities.
        //
        // Because max+1 id stamping can collide across layers (ids are
        // per-layer-local), we iterate per layer.

        // Build a lookup: (layer_index, entity_id) → explicit_priority, only
        // for priority_pushes entries that landed with explicit priorities.
        // Since entity_ids were assigned as max+1 per push sequentially,
        // we need to re-derive them.  Easiest: after the push phase the newly
        // appended entities are the LAST `priority_pushes_for_this_layer.len()`
        // entries in ordered_entities.
        //
        // Count how many pushes land per layer first, then sort.

        // Collect per-layer push counts.
        let mut push_counts: std::collections::HashMap<u32, Vec<u32>> =
            std::collections::HashMap::new();
        for push in &self.priority_pushes {
            push_counts
                .entry(push.layer_index)
                .or_default()
                .push(push.priority);
        }

        for layer in layers.iter_mut() {
            let Some(push_priorities) = push_counts.get(&layer.global_layer_index) else {
                continue;
            };

            let total = layer.ordered_entities.len();
            let n_pushed = push_priorities.len();
            let n_original = total.saturating_sub(n_pushed);

            // Build a parallel priority vec (one per entity).
            let mut priorities: Vec<u32> = Vec::with_capacity(total);
            for entity in layer.ordered_entities[..n_original].iter() {
                priorities.push(entity.role.default_priority());
            }
            for &p in push_priorities {
                priorities.push(p);
            }

            // Stable-sort by (priority, original_index) — Rust sort_by is stable.
            let mut indexed: Vec<(usize, u32)> = priorities
                .into_iter()
                .enumerate()
                .map(|(i, p)| (i, p))
                .collect();
            indexed.sort_by_key(|&(i, p)| (p, i));

            let mut sorted_entities: Vec<PrintEntity> =
                Vec::with_capacity(layer.ordered_entities.len());
            // We need to clone to reorder; swap-based reorder would be more
            // efficient but clone is clearer and correct.
            let mut scratch: Vec<Option<PrintEntity>> =
                layer.ordered_entities.drain(..).map(Some).collect();
            for (orig_idx, _priority) in indexed {
                sorted_entities.push(scratch[orig_idx].take().unwrap());
            }
            layer.ordered_entities = sorted_entities;
        }

        // ---- Phase 3–5: deferred MergeOps ---------------------------------

        for op in self.merge_ops {
            match op {
                MergeOp::ModifyEntity {
                    layer: layer_idx,
                    entity_id,
                    mutation,
                } => {
                    let layer = layers
                        .iter_mut()
                        .find(|l| l.global_layer_index == layer_idx);
                    // If layer not found, treat as entity-not-found (entity_id error).
                    let entity = layer.and_then(|l| {
                        l.ordered_entities
                            .iter_mut()
                            .find(|e| e.entity_id == entity_id)
                    });
                    match entity {
                        Some(e) => match mutation {
                            EntityMutation::SetSpeedFactor(v) => {
                                e.path.speed_factor = v;
                            }
                            EntityMutation::SetFlowFactor(v) => {
                                for pt in e.path.points.iter_mut() {
                                    pt.flow_factor = v;
                                }
                            }
                        },
                        None => {
                            return Err(format!(
                                "modify_entity: entity_id {} not found in layer {}",
                                entity_id, layer_idx
                            ));
                        }
                    }
                }

                MergeOp::SortLayer {
                    layer: layer_idx,
                    key,
                } => {
                    if let Some(layer) = layers
                        .iter_mut()
                        .find(|l| l.global_layer_index == layer_idx)
                    {
                        match key {
                            SortKey::ByPriorityAndEntityId => {
                                layer
                                    .ordered_entities
                                    .sort_by_key(|e| (e.role.default_priority(), e.entity_id));
                            }
                            SortKey::ByEntityId => {
                                layer.ordered_entities.sort_by_key(|e| e.entity_id);
                            }
                            SortKey::ByObjectIdThenPriority => {
                                layer.ordered_entities.sort_by(|a, b| {
                                    let key_a =
                                        (&a.region_key.object_id, a.role.default_priority());
                                    let key_b =
                                        (&b.region_key.object_id, b.role.default_priority());
                                    key_a.cmp(&key_b)
                                });
                            }
                        }
                        // Packet-39 travel-anchor invariant: travel_moves reference
                        // entity_ids by value; sort only reorders entities in the vec,
                        // it does NOT re-aim travel_moves. The entity_ids themselves are
                        // preserved, so the anchor invariant is maintained automatically.
                    }
                    // Layer not found → no-op (matches ModifyEntity layer-not-found skip).
                }

                MergeOp::InsertSynthLayer { idx, data } => {
                    // Validate bounds: idx must be a valid existing position.
                    if idx as usize >= layers.len() {
                        return Err(format!(
                            "insert_synthetic_layer: synthetic layer insert idx {} out of bounds (layers.len() = {})",
                            idx,
                            layers.len()
                        ));
                    }
                    // Build a fresh LayerCollectionIR from SyntheticLayerData.
                    // The insertion index in the outer vec (post-insert) is idx + 1.
                    let insert_pos = (idx + 1) as usize;
                    let global_layer_index = insert_pos as u32;
                    // Compute global max entity_id across all existing layers so that
                    // synth-layer IDs don't collide with any neighbor.
                    let max_existing_id = layers
                        .iter()
                        .flat_map(|l| l.ordered_entities.iter().map(|e| e.entity_id))
                        .max()
                        .unwrap_or(0);
                    let mut next_synth_id = max_existing_id + 1;
                    let ordered_entities: Vec<PrintEntity> = data
                        .paths
                        .into_iter()
                        .enumerate()
                        .map(|(topo_order, path)| {
                            let entity_id = next_synth_id;
                            next_synth_id += 1;
                            let role = path.role.clone();
                            PrintEntity {
                                entity_id,
                                path,
                                role,
                                tool_index: 0,
                                region_key: RegionKey {
                                    global_layer_index,
                                    object_id: String::new(),
                                    region_id: 0,
                                    variant_chain: Vec::new(),
                                },
                                topo_order: topo_order as u32,
                            }
                        })
                        .collect();
                    let new_layer = LayerCollectionIR {
                        global_layer_index,
                        z: data.z,
                        ordered_entities,
                        ..Default::default()
                    };
                    layers.insert(insert_pos, new_layer);
                }

                MergeOp::InsertEntityAt {
                    layer: layer_idx,
                    position,
                    path,
                    tool_index,
                    region_key,
                } => {
                    let layer = match layers
                        .iter_mut()
                        .find(|l| l.global_layer_index == layer_idx)
                    {
                        Some(l) => l,
                        None => {
                            return Err(format!("insert_entity_at: layer {} not found", layer_idx));
                        }
                    };
                    let n = layer.ordered_entities.len();
                    if position as usize > n {
                        return Err(format!(
                            "position {} out of bounds; layer has {} entities",
                            position, n
                        ));
                    }
                    // Generate next entity_id = max existing + 1, or 1 if empty.
                    let next_id = layer
                        .ordered_entities
                        .iter()
                        .map(|e| e.entity_id)
                        .max()
                        .map(|m| m + 1)
                        .unwrap_or(1);
                    let role = path.role.clone();
                    let new_entity = PrintEntity {
                        entity_id: next_id,
                        path,
                        role,
                        tool_index,
                        region_key,
                        topo_order: position,
                    };
                    layer.ordered_entities.insert(position as usize, new_entity);
                    // Remap all positional after_entity_index references >= position.
                    for tc in layer.tool_changes.iter_mut() {
                        if tc.after_entity_index >= position {
                            tc.after_entity_index += 1;
                        }
                    }
                    for zh in layer.z_hops.iter_mut() {
                        if zh.after_entity_index >= position {
                            zh.after_entity_index += 1;
                        }
                    }
                    for tr in layer.retracts.iter_mut() {
                        if tr.after_entity_index >= position {
                            tr.after_entity_index += 1;
                        }
                    }
                }

                MergeOp::SetEntityOrder {
                    layer: layer_idx,
                    items,
                } => {
                    let layer = match layers
                        .iter_mut()
                        .find(|l| l.global_layer_index == layer_idx)
                    {
                        Some(l) => l,
                        None => {
                            return Err(format!("set_entity_order: layer {} not found", layer_idx));
                        }
                    };
                    let n = layer.ordered_entities.len();
                    // Validate: items.len() must equal n.
                    if items.len() != n {
                        return Err(format!(
                            "invalid permutation: expected {} items, got {}",
                            n,
                            items.len()
                        ));
                    }
                    // Validate: items must be a permutation of 0..n.
                    let mut seen = vec![false; n];
                    for &(idx, _) in &items {
                        if idx as usize >= n {
                            return Err(format!(
                                "invalid permutation: index {} out of range (layer has {} entities)",
                                idx, n
                            ));
                        }
                        if seen[idx as usize] {
                            return Err(format!(
                                "invalid permutation: index {} appears more than once",
                                idx
                            ));
                        }
                        seen[idx as usize] = true;
                    }
                    // Build the reordered entity vec.
                    // new_entities[k] = original_entities[items[k].0]
                    // Reversal (items[k].1 == true) is a no-op TODO for this packet.
                    let original = layer.ordered_entities.clone();
                    let new_entities: Vec<PrintEntity> = items
                        .iter()
                        .map(|&(old_idx, _reversed)| original[old_idx as usize].clone())
                        .collect();
                    // Build inverse permutation: inverse[old_idx] = new_idx.
                    let mut inverse = vec![0u32; n];
                    for (new_idx, &(old_idx, _)) in items.iter().enumerate() {
                        inverse[old_idx as usize] = new_idx as u32;
                    }
                    // Remap positional after_entity_index references.
                    for tc in layer.tool_changes.iter_mut() {
                        let old = tc.after_entity_index as usize;
                        if old < n {
                            tc.after_entity_index = inverse[old];
                        }
                    }
                    for zh in layer.z_hops.iter_mut() {
                        let old = zh.after_entity_index as usize;
                        if old < n {
                            zh.after_entity_index = inverse[old];
                        }
                    }
                    for tr in layer.retracts.iter_mut() {
                        let old = tr.after_entity_index as usize;
                        if old < n {
                            tr.after_entity_index = inverse[old];
                        }
                    }
                    layer.ordered_entities = new_entities;
                }
            }
        }

        // Merge guest-emitted annotations into target layers.
        for (layer_index, annotation) in &self.annotations {
            if let Some(layer) = layers
                .iter_mut()
                .find(|l| l.global_layer_index == *layer_index)
            {
                layer.annotations.push(annotation.clone());
            }
        }

        Ok(())
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
            .field("priority_pushes", &self.priority_pushes.len())
            .field("merge_ops", &self.merge_ops.len())
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
