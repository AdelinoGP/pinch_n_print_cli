//! Prepass output builder types for SDK.
//!
//! These builders correspond to the WIT resources in docs/03_wit_and_manifest.md (world-prepass.wit).
//! They are used by PrepassModule implementations to emit mesh analysis and layer planning output.

use crate::prepass_types::{
    Diagnostic, FacetAnnotation, LayerProposal, ObjectId, SurfaceGroupProposal,
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
    diagnostics: Vec<Diagnostic>,
}

impl SupportGeometryOutput {
    /// Create a new empty output.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            diagnostics: Vec::new(),
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

    /// Push a diagnostic record.
    ///
    /// Per docs/adr/0010-typed-diagnostic-channel.md:
    /// ```wit
    /// push-diagnostic: func(d: diagnostic) -> result<_, string>;
    /// ```
    pub fn push_diagnostic(&mut self, d: Diagnostic) -> Result<(), String> {
        self.diagnostics.push(d);
        Ok(())
    }

    /// Get all entries (for testing).
    #[doc(hidden)]
    pub fn entries(&self) -> &[super::prepass_types::SupportPlanEntry] {
        &self.entries
    }

    /// Get all diagnostics in insertion order (for testing).
    #[doc(hidden)]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
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
            .field("diagnostics", &self.diagnostics.len())
            .finish()
    }
}
