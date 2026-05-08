//! Prepass output builder types for SDK.
//!
//! These builders correspond to the WIT resources in docs/03_wit_and_manifest.md (world-prepass.wit).
//! They are used by PrepassModule implementations to emit mesh analysis and layer planning output.

use crate::prepass_types::{
    FacetAnnotation, LayerProposal, ObjectId, PaintValueView, SurfaceGroupProposal,
};

/// Output builder for mesh analysis stage.
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit):
/// ```wit
/// resource mesh-analysis-output {
///     push-facet-annotation: func(obj: object-id, ann: facet-annotation) -> result<_, string>;
///     push-surface-group:    func(obj: object-id, grp: surface-group-proposal) -> result<_, string>;
/// }
/// ```
pub struct MeshAnalysisOutput {
    facet_annotations: Vec<(ObjectId, FacetAnnotation)>,
    surface_groups: Vec<(ObjectId, SurfaceGroupProposal)>,
}

impl MeshAnalysisOutput {
    /// Create a new MeshAnalysisOutput.
    pub fn new() -> Self {
        Self {
            facet_annotations: Vec::new(),
            surface_groups: Vec::new(),
        }
    }

    /// Push a facet annotation for an object.
    ///
    /// Per docs/03_wit_and_manifest.md (world-prepass.wit):
    /// ```wit
    /// push-facet-annotation: func(obj: object-id, ann: facet-annotation) -> result<_, string>;
    /// ```
    pub fn push_facet_annotation(
        &mut self,
        object_id: ObjectId,
        annotation: FacetAnnotation,
    ) -> Result<(), String> {
        self.facet_annotations.push((object_id, annotation));
        Ok(())
    }

    /// Push a surface group proposal for an object.
    ///
    /// Per docs/03_wit_and_manifest.md (world-prepass.wit):
    /// ```wit
    /// push-surface-group: func(obj: object-id, grp: surface-group-proposal) -> result<_, string>;
    /// ```
    pub fn push_surface_group(
        &mut self,
        object_id: ObjectId,
        group: SurfaceGroupProposal,
    ) -> Result<(), String> {
        self.surface_groups.push((object_id, group));
        Ok(())
    }

    /// Get all facet annotations (for testing).
    #[doc(hidden)]
    pub fn facet_annotations(&self) -> &[(ObjectId, FacetAnnotation)] {
        &self.facet_annotations
    }

    /// Get all surface groups (for testing).
    #[doc(hidden)]
    pub fn surface_groups(&self) -> &[(ObjectId, SurfaceGroupProposal)] {
        &self.surface_groups
    }
}

impl Default for MeshAnalysisOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MeshAnalysisOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshAnalysisOutput")
            .field("facet_annotations", &self.facet_annotations.len())
            .field("surface_groups", &self.surface_groups.len())
            .finish()
    }
}

/// Output builder for layer planning stage.
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit):
/// ```wit
/// resource layer-plan-output {
///     push-layer: func(proposal: layer-proposal) -> result<_, string>;
/// }
/// ```
pub struct LayerPlanOutput {
    layers: Vec<LayerProposal>,
}

impl LayerPlanOutput {
    /// Create a new LayerPlanOutput.
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Push a layer proposal.
    ///
    /// Per docs/03_wit_and_manifest.md (world-prepass.wit):
    /// ```wit
    /// push-layer: func(proposal: layer-proposal) -> result<_, string>;
    /// ```
    pub fn push_layer(&mut self, proposal: LayerProposal) -> Result<(), String> {
        self.layers.push(proposal);
        Ok(())
    }

    /// Get all layer proposals (for testing).
    #[doc(hidden)]
    pub fn layers(&self) -> &[LayerProposal] {
        &self.layers
    }
}

impl Default for LayerPlanOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for LayerPlanOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LayerPlanOutput")
            .field("layers", &self.layers.len())
            .finish()
    }
}

/// Modification to an object's mesh produced by mesh segmentation.
///
/// Contains the (possibly unchanged) geometry and updated per-layer facet values
/// after sub-facet paint strokes have been normalized into whole-triangle assignments.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectMeshModification {
    /// The object this modification applies to.
    pub object_id: String,
    /// New vertex list (may be identical to original if no splitting occurred).
    pub new_vertices: Vec<[f32; 3]>,
    /// New triangle list (may be identical to original if no splitting occurred).
    pub new_triangles: Vec<[u32; 3]>,
    /// Updated facet values per paint layer (outer vec = paint layers, inner = per triangle).
    pub updated_facet_values: Vec<Vec<Option<PaintValueView>>>,
    /// Whether strokes were consumed/cleared during processing.
    pub strokes_cleared: bool,
}

/// A single per-triangle paint mark emitted by
/// `MeshSegmentationOutput::mark_triangle_paint`. Mirrors the WIT
/// `mesh-segmentation-output::mark-triangle-paint` method signature
/// from docs/03_wit_and_manifest.md (world-prepass.wit).
#[derive(Debug, Clone, PartialEq)]
pub struct TrianglePaintMark {
    /// Object this mark applies to.
    pub object_id: String,
    /// Triangle index inside the object's mesh.
    pub facet_index: u32,
    /// Paint semantic (e.g. `"support_enforcer"`, `"seam"`).
    pub semantic: String,
    /// Paint value, serialized as a string. Empty means "clear".
    pub value: String,
}

/// Output builder for mesh segmentation stage.
///
/// The canonical drain target is [`Self::mark_triangle_paint`] —
/// it matches the WIT `mesh-segmentation-output::mark-triangle-paint`
/// method one-to-one (docs/03_wit_and_manifest.md world-prepass.wit),
/// which is the only data the WIT boundary surfaces back to the host.
///
/// The legacy [`Self::push_modification`] API ships an
/// [`ObjectMeshModification`] carrying full mesh geometry + per-layer
/// facet values; that shape has no representation on the current WIT
/// surface (the host can't reconstruct vertices/triangles from per-
/// triangle marks). It remains available for native-mode module
/// authoring where the SDK can observe `modifications()` directly, but
/// the `#[slicer_module]` macro path drains only the
/// [`triangle_paint_marks`](Self::triangle_paint_marks) stream.
pub struct MeshSegmentationOutput {
    modifications: Vec<ObjectMeshModification>,
    triangle_paint_marks: Vec<TrianglePaintMark>,
}

impl MeshSegmentationOutput {
    /// Create a new empty output.
    pub fn new() -> Self {
        Self {
            modifications: Vec::new(),
            triangle_paint_marks: Vec::new(),
        }
    }

    /// Push a mesh modification for an object.
    ///
    /// Legacy API — see the struct-level docs. The `#[slicer_module]`
    /// macro path does not drain `modifications()` back through the
    /// WIT boundary because the current `world-prepass` surface
    /// carries per-triangle marks only.
    pub fn push_modification(
        &mut self,
        modification: ObjectMeshModification,
    ) -> Result<(), String> {
        self.modifications.push(modification);
        Ok(())
    }

    /// Record a per-triangle paint assignment. Mirrors the WIT
    /// `mesh-segmentation-output::mark-triangle-paint` method.
    ///
    /// Push order is preserved; the host-side harvest is deterministic.
    /// Validation happens at the host boundary (empty `object_id`,
    /// empty `semantic` are rejected there with a structured error).
    pub fn mark_triangle_paint(
        &mut self,
        object_id: String,
        facet_index: u32,
        semantic: String,
        value: String,
    ) -> Result<(), String> {
        self.triangle_paint_marks.push(TrianglePaintMark {
            object_id,
            facet_index,
            semantic,
            value,
        });
        Ok(())
    }

    /// Get all modifications (for testing).
    #[doc(hidden)]
    pub fn modifications(&self) -> &[ObjectMeshModification] {
        &self.modifications
    }

    /// Get all triangle paint marks in push order. The
    /// `#[slicer_module]` macro drains this slice into the WIT
    /// `mesh-segmentation-output.mark-triangle-paint` resource on the
    /// host after the trait body returns.
    pub fn triangle_paint_marks(&self) -> &[TrianglePaintMark] {
        &self.triangle_paint_marks
    }
}

impl Default for MeshSegmentationOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MeshSegmentationOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshSegmentationOutput")
            .field("modifications", &self.modifications.len())
            .field("triangle_paint_marks", &self.triangle_paint_marks.len())
            .finish()
    }
}

/// A 2-D polygon with an outer contour and optional holes, expressed in mm (f64).
///
/// This is the SDK-side view type bridging WIT `expolygon` (which uses i64 100 nm units
/// internally) to f64 mm values for module authors. Cannot re-export `slicer_ir::ExPolygon`
/// directly because of the unit mismatch.
#[derive(Debug, Clone, PartialEq)]
pub struct ExPolygonView {
    /// Outer contour vertices as `[x_mm, y_mm]` pairs.
    pub contour: Vec<[f64; 2]>,
    /// Zero or more hole contours, each as `[x_mm, y_mm]` pairs.
    pub holes: Vec<Vec<[f64; 2]>>,
}

impl ExPolygonView {
    /// Construct an `ExPolygonView` from a contour and a list of holes.
    pub fn new(contour: Vec<[f64; 2]>, holes: Vec<Vec<[f64; 2]>>) -> Self {
        Self { contour, holes }
    }
}

/// Typed paint value carried by a `PaintRegionEntry`.
///
/// Mirrors the WIT `paint-value-input` variant being added in Step 4.
#[derive(Debug, Clone, PartialEq)]
pub enum PaintValueInput {
    /// Boolean flag (e.g. on/off for support enforcer).
    Flag(bool),
    /// Floating-point scalar (e.g. flow multiplier).
    Scalar(f32),
    /// Zero-based tool/extruder index.
    ToolIndex(u32),
    /// Arbitrary string payload for extension semantics.
    Custom(String),
}

/// A single paint region entry produced by paint segmentation.
#[derive(Debug, Clone, PartialEq)]
pub struct PaintRegionEntry {
    /// Global layer index for this region.
    pub layer_index: u32,
    /// The paint semantic (e.g. "support_enforcer", "fuzzy_skin").
    pub semantic: String,
    /// Object this region belongs to.
    pub object_id: String,
    /// Typed paint value for this region.
    pub value: PaintValueInput,
    /// Order of the paint layer (used for precedence).
    pub paint_order: u64,
    /// One or more projected polygons (contour + holes) for this region.
    pub polygons: Vec<ExPolygonView>,
}

/// Output builder for paint segmentation stage.
///
/// Collects per-layer paint region entries produced by `PrepassModule::run_paint_segmentation`.
pub struct PaintSegmentationOutput {
    regions: Vec<PaintRegionEntry>,
}

impl PaintSegmentationOutput {
    /// Create a new empty output.
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// Push a paint region entry.
    pub fn push_paint_region(
        &mut self,
        layer_index: u32,
        semantic: String,
        object_id: String,
        paint_order: u64,
        value: PaintValueInput,
        polygons: Vec<ExPolygonView>,
    ) {
        self.regions.push(PaintRegionEntry {
            layer_index,
            semantic,
            object_id,
            value,
            paint_order,
            polygons,
        });
    }

    /// Get all regions (for testing).
    #[doc(hidden)]
    pub fn regions(&self) -> &[PaintRegionEntry] {
        &self.regions
    }
}

impl Default for PaintSegmentationOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PaintSegmentationOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PaintSegmentationOutput")
            .field("regions", &self.regions.len())
            .finish()
    }
}

/// Output builder for seam planning stage.
///
/// Collects seam plan entries produced by `PrepassModule::run_seam_planning`.
pub struct SeamPlanningOutput {
    entries: Vec<super::prepass_types::SeamPlanEntry>,
}

impl SeamPlanningOutput {
    /// Create a new empty output.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Push a seam plan entry.
    pub fn push_seam_plan(
        &mut self,
        entry: super::prepass_types::SeamPlanEntry,
    ) -> Result<(), String> {
        self.entries.push(entry);
        Ok(())
    }

    /// Get all entries (for testing).
    #[doc(hidden)]
    pub fn entries(&self) -> &[super::prepass_types::SeamPlanEntry] {
        &self.entries
    }
}

impl Default for SeamPlanningOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SeamPlanningOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SeamPlanningOutput")
            .field("entries", &self.entries.len())
            .finish()
    }
}

/// Output builder for support geometry stage.
///
/// Collects support plan entries produced by `PrepassModule::run_support_geometry`.
pub struct SupportGeometryOutput {
    entries: Vec<super::prepass_types::SupportPlanEntry>,
}

impl SupportGeometryOutput {
    /// Create a new empty output.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Push a support plan entry.
    pub fn push_support_plan_entry(
        &mut self,
        entry: super::prepass_types::SupportPlanEntry,
    ) -> Result<(), String> {
        self.entries.push(entry);
        Ok(())
    }

    /// Get all entries (for testing).
    #[doc(hidden)]
    pub fn entries(&self) -> &[super::prepass_types::SupportPlanEntry] {
        &self.entries
    }
}

impl Default for SupportGeometryOutput {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SupportGeometryOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SupportGeometryOutput")
            .field("entries", &self.entries.len())
            .finish()
    }
}
