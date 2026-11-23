//! Owned, typed projections of committed preparation inputs.
//!
//! These values are independent of ordinary layer views. Configs are owned,
//! declared-key-filtered views; polygon coordinates retain the IR's 100 nm
//! units, while floating-point positions and heights remain in millimeters.

use std::collections::HashMap;

use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{
    BoundingBox3, ConfigView, ExPolygon, PaintSemantic, Point3, Point3WithWidth, SeamPosition,
    SurfaceGroup, SurfaceGroupId,
};

use super::PreparationInputError;

/// Capability-owned paint identity, including community-defined string values.
#[derive(Debug, Clone)]
pub enum PreparationPaintValue {
    /// Boolean flag.
    Flag(bool),
    /// Scalar value, retaining its exact floating-point bits.
    Scalar(f32),
    /// Tool/material index.
    ToolIndex(u32),
    /// Community-defined or module-defined string value.
    Custom(String),
}

impl PartialEq for PreparationPaintValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Flag(a), Self::Flag(b)) => a == b,
            (Self::Scalar(a), Self::Scalar(b)) => a.to_bits() == b.to_bits(),
            (Self::ToolIndex(a), Self::ToolIndex(b)) => a == b,
            (Self::Custom(a), Self::Custom(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for PreparationPaintValue {}

impl From<&slicer_ir::PaintValue> for PreparationPaintValue {
    fn from(value: &slicer_ir::PaintValue) -> Self {
        match value {
            slicer_ir::PaintValue::Flag(value) => Self::Flag(*value),
            slicer_ir::PaintValue::Scalar(value) => Self::Scalar(*value),
            slicer_ir::PaintValue::ToolIndex(value) => Self::ToolIndex(*value),
            slicer_ir::PaintValue::Custom(value) => Self::Custom(value.clone()),
        }
    }
}

impl From<&PreparationPaintValue> for slicer_ir::PaintValue {
    fn from(value: &PreparationPaintValue) -> Self {
        match value {
            PreparationPaintValue::Flag(value) => Self::Flag(*value),
            PreparationPaintValue::Scalar(value) => Self::Scalar(*value),
            PreparationPaintValue::ToolIndex(value) => Self::ToolIndex(*value),
            PreparationPaintValue::Custom(value) => Self::Custom(value.clone()),
        }
    }
}

/// Ordered semantic names and exact paint values identifying a variant.
pub type PreparationVariantChain = Vec<(String, PreparationPaintValue)>;

/// Full preparation region identity; only the WIT projection stringifies IDs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationRegionKey {
    /// Global schedule index.
    pub global_layer_index: u32,
    /// Object identity.
    pub object_id: String,
    /// Native region identity, without decimal-string conversion.
    pub region_id: u64,
    /// Ordered paint variant identity.
    pub variant_chain: PreparationVariantChain,
}

impl From<&slicer_ir::RegionKey> for PreparationRegionKey {
    fn from(key: &slicer_ir::RegionKey) -> Self {
        Self {
            global_layer_index: key.global_layer_index,
            object_id: key.object_id.clone(),
            region_id: key.region_id,
            variant_chain: key
                .variant_chain
                .iter()
                .map(|(semantic, value)| (semantic.clone(), value.into()))
                .collect(),
        }
    }
}

impl From<&PreparationRegionKey> for slicer_ir::RegionKey {
    fn from(key: &PreparationRegionKey) -> Self {
        Self {
            global_layer_index: key.global_layer_index,
            object_id: key.object_id.clone(),
            region_id: key.region_id,
            variant_chain: key
                .variant_chain
                .iter()
                .map(|(semantic, value)| (semantic.clone(), value.into()))
                .collect(),
        }
    }
}

/// One paint stroke with native triangle triples flattened in their original order.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationPaintStroke {
    /// Three consecutive vertices per original triangle, in millimeters.
    pub triangles: Vec<Point3>,
    /// Original paint semantic name.
    pub semantic: String,
    /// Exact paint value.
    pub value: PreparationPaintValue,
}

/// All paint of one semantic on a mesh object.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationPaintLayer {
    /// Original paint semantic name.
    pub semantic: String,
    /// Optional value per mesh facet, preserving facet order.
    pub facet_values: Vec<Option<PreparationPaintValue>>,
    /// Sub-facet strokes in their original order.
    pub strokes: Vec<PreparationPaintStroke>,
}

/// Nonplanar shell identity attached to an active region.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationNonplanarShell {
    /// Surface group owning the shell.
    pub surface_group_id: SurfaceGroupId,
    /// Index within that surface group's shells.
    pub shell_index: u32,
}

/// Active region in a committed global layer.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationActiveRegion {
    /// Object identity.
    pub object_id: String,
    /// Native region identity.
    pub region_id: u64,
    /// Owned effective config filtered to the module's declared keys.
    pub config: ConfigView,
    /// Effective layer height in millimeters.
    pub effective_layer_height: f32,
    /// Nonplanar shell, when present.
    pub nonplanar_shell: Option<PreparationNonplanarShell>,
    /// Whether this region catches up to a synchronization layer.
    pub is_catchup_layer: bool,
    /// Bottom of the catch-up interval in millimeters.
    pub catchup_z_bottom: f32,
    /// Selected tool/material index.
    pub tool_index: u32,
}

/// One committed global schedule layer.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationLayer {
    /// Global schedule index.
    pub index: u32,
    /// Layer Z in millimeters.
    pub z: f32,
    /// Whether the layer contains nonplanar regions.
    pub has_nonplanar: bool,
    /// Whether the layer synchronizes object schedules.
    pub is_sync_layer: bool,
    /// Whether the layer is a raft layer.
    pub is_raft: bool,
    /// Active regions in committed order.
    pub active_regions: Vec<PreparationActiveRegion>,
}

/// Maturity of the geometry exposed during preparation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationInputPhase {
    /// Committed PrePass geometry, not future per-layer stage output.
    CommittedPrepass,
}

/// One committed sliced region, including its paint and shell classifications.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationRegion {
    /// Global schedule index.
    pub global_layer_index: u32,
    /// Layer Z in millimeters.
    pub z: f32,
    /// Object identity.
    pub object_id: String,
    /// Native region identity.
    pub region_id: u64,
    /// Ordered paint variant identity.
    pub variant_chain: PreparationVariantChain,
    /// Committed region boundaries in scaled integer units.
    pub polygons: Vec<ExPolygon>,
    /// Committed infill input, not a promised future perimeter inset.
    pub infill_areas: Vec<ExPolygon>,
    /// Associated nonplanar surface group.
    pub nonplanar_surface: Option<SurfaceGroupId>,
    /// Effective layer height in millimeters.
    pub effective_layer_height: f32,
    /// Semantic -> polygon -> segment optional paint values, without flattening.
    pub segment_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PreparationPaintValue>>>>,
    /// Minimum depth within a top shell, when classified.
    pub top_shell_index: Option<u8>,
    /// Minimum depth within a bottom shell, when classified.
    pub bottom_shell_index: Option<u8>,
    /// Top shell solid-fill geometry.
    pub top_solid_fill: Vec<ExPolygon>,
    /// Bottom shell solid-fill geometry.
    pub bottom_solid_fill: Vec<ExPolygon>,
    /// Whether the region is classified as a bridge.
    pub is_bridge: bool,
    /// Bridge geometry.
    pub bridge_areas: Vec<ExPolygon>,
    /// Bridge orientation in degrees.
    pub bridge_orientation_deg: f32,
    /// Committed sparse-infill input; may be empty before perimeters.
    pub sparse_infill_area: Vec<ExPolygon>,
    /// Internal solid-fill geometry.
    pub internal_solid_fill: Vec<ExPolygon>,
    /// Internal bridge geometry.
    pub internal_bridge_areas: Vec<ExPolygon>,
    /// Angles parallel to internal bridge areas, or the native empty fallback.
    pub internal_bridge_angles_deg: Vec<f32>,
    /// Raft-fill geometry.
    pub raft_fill: Vec<ExPolygon>,
    /// Explicit committed-input maturity.
    pub phase: PreparationInputPhase,
}

/// Effective config for one exact committed region key.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationRegionConfig {
    /// Full region identity.
    pub key: PreparationRegionKey,
    /// Owned effective config filtered to the module's declared keys.
    pub config: ConfigView,
}

/// Complete classification of a mesh facet.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PreparationFacetClass {
    /// Ordinary facet.
    Normal,
    /// Near-horizontal facet.
    NearHorizontal {
        /// Slope angle in degrees.
        slope_angle_deg: f32,
    },
    /// Overhanging facet.
    Overhang {
        /// Overhang angle in degrees.
        angle_deg: f32,
    },
    /// Bridge facet.
    Bridge,
    /// Top surface facet.
    TopSurface,
    /// Bottom surface facet.
    BottomSurface,
}

/// Full committed bridge-region classification.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationBridgeRegion {
    /// Bridge identity.
    pub id: u64,
    /// Facets comprising this bridge.
    pub facet_indices: Vec<u32>,
    /// Bridge direction in degrees.
    pub bridge_direction_deg: f32,
    /// Shortest contiguous anchor run in millimeters.
    pub anchor_width_mm: f32,
    /// Longest unsupported span in millimeters.
    pub bridge_length_mm: f32,
    /// Frozen expansion margin in millimeters.
    pub expansion_margin_mm: f32,
    /// Whether the bridge passed classification filters.
    pub is_valid: bool,
    /// XY projection in scaled integer units.
    pub xy_footprint: Vec<ExPolygon>,
}

/// Full committed overhang-region classification.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationOverhangRegion {
    /// Overhang identity.
    pub id: u64,
    /// Facets comprising this overhang.
    pub facet_indices: Vec<u32>,
    /// Maximum angle in degrees.
    pub max_angle_deg: f32,
    /// Whether this overhang is support-eligible.
    pub needs_support: bool,
    /// XY projection in scaled integer units.
    pub xy_footprint: Vec<ExPolygon>,
}

/// Object-specific committed surface classifications.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSurfaceObject {
    /// Object identity.
    pub object_id: String,
    /// Classification per facet.
    pub facet_classes: Vec<PreparationFacetClass>,
    /// Complete native surface-group records.
    pub surface_groups: Vec<SurfaceGroup>,
    /// Complete bridge-region records.
    pub bridge_regions: Vec<PreparationBridgeRegion>,
    /// Complete overhang-region records.
    pub overhang_regions: Vec<PreparationOverhangRegion>,
}

/// Surface geometry for one object and global layer.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSurfaceLayer {
    /// Object identity, retained before the inner layer projection.
    pub object_id: String,
    /// Global schedule index.
    pub global_layer_index: u32,
    /// Native quartile/polygon records for this object and layer.
    pub overhang_quartile_polygons: Vec<QuartileBand>,
    /// Previous-layer boundaries in scaled integer units.
    pub prev_layer_boundaries: Vec<ExPolygon>,
}

/// Mesh metadata; world-space buffers are read through separate batch methods.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationMeshObject {
    /// Object identity.
    pub object_id: String,
    /// World-space bounds in millimeters; absent for an empty mesh.
    pub bounds: Option<BoundingBox3>,
    /// Number of world-space vertices.
    pub vertex_count: u64,
    /// Number of index triples.
    pub triangle_count: u64,
    /// Paint retaining every native value variant and stroke order.
    pub paint_layers: Vec<PreparationPaintLayer>,
}

/// World-space geometric query against a committed mesh.
#[derive(Debug, Clone, PartialEq)]
pub enum PreparationMeshQuery {
    /// Cast downwards from a world-space Z in millimeters.
    RaycastZDown {
        /// Object identity.
        object_id: String,
        /// World X in millimeters.
        x: f32,
        /// World Y in millimeters.
        y: f32,
        /// Starting world Z in millimeters.
        start_z: f32,
    },
    /// Query the surface normal at a world-space point in millimeters.
    SurfaceNormalAt {
        /// Object identity.
        object_id: String,
        /// World X in millimeters.
        x: f32,
        /// World Y in millimeters.
        y: f32,
        /// World Z in millimeters.
        z: f32,
    },
    /// Query world-space object bounds.
    ObjectBounds {
        /// Object identity.
        object_id: String,
    },
}

/// Mesh query response, in the same order as its corresponding request.
#[derive(Debug, Clone, PartialEq)]
pub enum PreparationMeshQueryResult {
    /// Optional hit Z in millimeters.
    RaycastZDown(Option<f32>),
    /// Optional surface normal.
    SurfaceNormalAt(Option<Point3>),
    /// World-space bounds in millimeters.
    ObjectBounds(BoundingBox3),
}

/// Reason attached to a scored seam candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationSeamReason {
    /// Concave corner.
    Concave,
    /// Aligned seam.
    Aligned,
    /// User-forced location.
    UserForced,
    /// Sharp corner.
    Sharp,
}

/// A seam candidate with its full-width point, score, and reason.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationScoredSeamCandidate {
    /// Native point retaining width, flow and overhang metadata.
    pub position: Point3WithWidth,
    /// Candidate score.
    pub score: f32,
    /// Reason for this candidate.
    pub reason: PreparationSeamReason,
}

/// Committed seam plan for an exact region key.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSeam {
    /// Full region identity.
    pub region_key: PreparationRegionKey,
    /// Chosen full-width point and wall index.
    pub chosen_candidate: SeamPosition,
    /// Scored candidates in committed order.
    pub scored_candidates: Vec<PreparationScoredSeamCandidate>,
}

/// Source identity and anchor of a support candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationSupportCandidateSource {
    /// Object identity.
    pub object_id: String,
    /// Native region identity.
    pub region_id: u64,
    /// Global schedule index.
    pub global_layer_index: u32,
    /// Anchor Z in scaled integer units, not millimeters.
    pub z_units: i64,
    /// Ordered paint variant identity.
    pub variant_chain: PreparationVariantChain,
}

/// One committed support-analysis candidate.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSupportCandidate {
    /// Candidate identity.
    pub id: u64,
    /// Candidate geometry in scaled integer units.
    pub geometry: Vec<ExPolygon>,
    /// Full source identity.
    pub source: PreparationSupportCandidateSource,
    /// Whether support was enforced.
    pub enforced: bool,
    /// Whether support was blocked.
    pub blocked: bool,
}

/// Full key of a support geometry map entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationSupportGeometryKey {
    /// Support layer index.
    pub global_support_layer_index: u32,
    /// Object identity.
    pub object_id: String,
    /// Native region identity.
    pub region_id: u64,
    /// Ordered paint variant identity.
    pub variant_chain: PreparationVariantChain,
}

/// One keyed collection of support polygons.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSupportGeometryEntry {
    /// Full support geometry identity.
    pub key: PreparationSupportGeometryKey,
    /// Geometry in scaled integer units.
    pub polygons: Vec<ExPolygon>,
}

/// Source key used for support-family assignments.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationSupportSourceKey {
    /// Object identity.
    pub object_id: String,
    /// Native region identity.
    pub region_id: u64,
    /// Ordered paint variant identity.
    pub variant_chain: PreparationVariantChain,
}

/// One support-family assignment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparationSupportFamilyAssignment {
    /// Full source identity.
    pub key: PreparationSupportSourceKey,
    /// Assigned family identifier.
    pub family_id: String,
}

/// Bounded support-analysis collections plus invariant metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSupportAnalysisBatch {
    /// Bounded candidates, independently ranged from every map below.
    pub candidates: Vec<PreparationSupportCandidate>,
    /// Bounded model occupancy entries.
    pub model_occupancy: Vec<PreparationSupportGeometryEntry>,
    /// Bounded cantilever surface entries.
    pub cantilever_surfaces: Vec<PreparationSupportGeometryEntry>,
    /// Bounded termination surface entries.
    pub termination_surfaces: Vec<PreparationSupportGeometryEntry>,
    /// Bounded support territory entries.
    pub support_territory: Vec<PreparationSupportGeometryEntry>,
    /// Invariant shared settings, not truncated with entries.
    pub shared_settings: Vec<(String, String)>,
    /// Invariant feasible envelope in scaled integer units.
    pub baseline_feasible_envelope: Vec<ExPolygon>,
    /// Independently bounded family assignments.
    pub family_assignments: Vec<PreparationSupportFamilyAssignment>,
}

/// Bounded support geometry plus invariant height/distance settings.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSupportGeometryBatch {
    /// Support layer height in millimeters.
    pub support_layer_height_mm: f32,
    /// Support top Z distance in millimeters.
    pub support_top_z_distance_mm: f32,
    /// Bounded canonically ordered geometry entries.
    pub entries: Vec<PreparationSupportGeometryEntry>,
}

/// Role of a support-plan geometry collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationSupportPlanRole {
    /// Support body.
    SupportBody,
    /// Top interface.
    TopInterface,
    /// Base interface.
    BaseInterface,
    /// Bottom interface.
    BottomInterface,
    /// Raft-related geometry.
    RaftRelated,
}

/// Geometry associated with one support-plan role.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSupportPlanRoleRegion {
    /// Support-plan role.
    pub role: PreparationSupportPlanRole,
    /// Geometry in scaled integer units.
    pub regions: Vec<ExPolygon>,
}

/// Optional support skeleton with parallel point and wall-count arrays.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSupportPlanSkeleton {
    /// Skeleton points in millimeters.
    pub points: Vec<Point3>,
    /// Wall counts aligned with the points.
    pub wall_counts: Vec<u32>,
}

/// Reason a support-plan entry declined work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreparationSupportPlanDeclineReason {
    /// Declined by policy.
    DeclinedPolicy,
    /// No route found.
    NoRoute,
    /// Route blocked.
    Blocked,
    /// Unsupported mode.
    UnsupportedMode,
}

/// Complete committed support-plan entry, retaining signed off-grid indices.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSupportPlanEntry {
    /// Signed layer index; negative off-grid rows are not raft/model keys.
    pub global_layer_index: i32,
    /// Object identity.
    pub object_id: String,
    /// Native region identity.
    pub region_id: u64,
    /// Support family identifier.
    pub family_id: String,
    /// Demand identities.
    pub demand_ids: Vec<String>,
    /// Body identities.
    pub body_ids: Vec<String>,
    /// Anchor's global schedule index.
    pub anchor_layer_index: u32,
    /// Anchor Z in scaled integer units, not millimeters.
    pub anchor_z: i64,
    /// Role-specific geometry.
    pub roles: Vec<PreparationSupportPlanRoleRegion>,
    /// Optional skeleton.
    pub skeleton: Option<PreparationSupportPlanSkeleton>,
    /// Declared capabilities.
    pub capabilities: Vec<String>,
    /// Original provenance.
    pub provenance: Vec<String>,
    /// Optional decline reason.
    pub decline_reason: Option<PreparationSupportPlanDeclineReason>,
    /// Lossless painted variant chain of the support region.
    pub variant_chain: PreparationVariantChain,
}

/// Invariant raft metadata attached to a support plan.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationRaftPlan {
    /// Total raft layer count.
    pub raft_layers: u32,
    /// First raft layer density.
    pub raft_first_layer_density: f32,
    /// Base raft layer count.
    pub base_raft_layers: u32,
    /// Interface raft layer count.
    pub interface_raft_layers: u32,
}

/// Bounded support-plan entries plus invariant optional raft metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSupportPlanBatch {
    /// Bounded canonically ordered entries.
    pub entries: Vec<PreparationSupportPlanEntry>,
    /// Optional raft metadata, not truncated with entries.
    pub raft_plan: Option<PreparationRaftPlan>,
}

/// Full source projection of a selected layer target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparationTargetSource {
    /// Model region and its exact merged member identities.
    Model {
        /// Selected model region key.
        key: PreparationRegionKey,
        /// Full merged member keys.
        members: Vec<PreparationRegionKey>,
    },
    /// Support carrier retaining its snapshot-local entry index and provenance.
    SupportCarrier {
        /// Original support-plan entry index, never presentation-renumbered.
        entry_index: u64,
        /// Object identity.
        object_id: String,
        /// Native region identity.
        region_id: u64,
        /// Ordered paint variant identity.
        variant_chain: PreparationVariantChain,
        /// Support family.
        family: String,
        /// Demand identities.
        demand_ids: Vec<String>,
        /// Body identities.
        body_ids: Vec<String>,
        /// Original provenance.
        provenance: Vec<String>,
        /// Anchor's global schedule index.
        anchor_layer_index: u32,
    },
    /// Explicit raft source, not a negative support row.
    Raft {
        /// Global raft schedule index.
        global_layer_index: u32,
    },
    /// Stage source anchored to a global layer.
    AnchoredStage {
        /// Anchor's global schedule index.
        anchor_layer_index: u32,
    },
}

/// Selected invocation context, separate from whole-print committed geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct PreparationSelectedTarget {
    /// Global invocation layer index.
    pub global_layer_index: u32,
    /// Frozen stage identity.
    pub stage_id: String,
    /// Whether the shared selection requests invocation.
    pub invoke: bool,
    /// Full source identity and provenance.
    pub source: PreparationTargetSource,
    /// Owned effective config filtered to the module's declared keys.
    pub config: ConfigView,
    /// Claims held for the selection.
    pub held_claims: Vec<String>,
    /// Whether coloring was authored.
    pub authored_coloring: bool,
    /// Resolved role tool, when present.
    pub resolved_role_tool: Option<u32>,
}

/// Owned typed reads shared by native hosts and generated guest adapters.
///
/// The backing owns authorization, availability, ordering and range checks.
/// Optional absence is `None`; a present empty product remains `Some(empty)`.
pub trait PrintPreparationRead: Send + Sync {
    /// Return the committed schedule length.
    fn layer_count(&self) -> Result<u64, PreparationInputError>;

    /// Read a bounded batch of committed schedule layers.
    fn layers(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationLayer>, PreparationInputError>;

    /// Read committed regions for one schedule layer.
    fn regions(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationRegion>, PreparationInputError>;

    /// Read exact-key effective region configs for one schedule layer.
    fn region_configs(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationRegionConfig>, PreparationInputError>;

    /// Read optional object-specific surface classifications.
    fn surface_objects(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSurfaceObject>>, PreparationInputError>;

    /// Read optional object-specific surface geometry for one layer.
    fn surface_layers(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSurfaceLayer>>, PreparationInputError>;

    /// Read world-space mesh metadata and lossless paint projections.
    fn mesh_objects(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationMeshObject>, PreparationInputError>;

    /// Read world-space mesh vertices in millimeters.
    fn mesh_vertices(
        &self,
        object_id: &str,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<Point3>, PreparationInputError>;

    /// Read mesh index triples in their original order.
    fn mesh_triangles(
        &self,
        object_id: &str,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<(u32, u32, u32)>, PreparationInputError>;

    /// Execute world-space queries, preserving caller request order.
    fn mesh_queries(
        &self,
        requests: &[PreparationMeshQuery],
    ) -> Result<Vec<PreparationMeshQueryResult>, PreparationInputError>;

    /// Read optional exact-key seam plans for one schedule layer.
    fn seams(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<Vec<PreparationSeam>>, PreparationInputError>;

    /// Independently range candidates, each geometry map and family assignments.
    /// Settings and the feasible envelope remain invariant; never zip collections.
    fn support_analysis(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportAnalysisBatch>, PreparationInputError>;

    /// Range entries, retaining invariant height/distance metadata.
    fn support_geometry(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportGeometryBatch>, PreparationInputError>;

    /// Range entries, retaining invariant optional raft metadata.
    fn support_plan(
        &self,
        offset: u64,
        max_items: u64,
    ) -> Result<Option<PreparationSupportPlanBatch>, PreparationInputError>;

    /// Read the frozen owner's selected invocation targets for one layer.
    fn selected_targets(
        &self,
        layer_index: u32,
        offset: u64,
        max_items: u64,
    ) -> Result<Vec<PreparationSelectedTarget>, PreparationInputError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paint_conversions_preserve_variants_utf8_and_signed_zero_bits() {
        let cases = [
            (
                slicer_ir::PaintValue::Flag(true),
                PreparationPaintValue::Flag(true),
            ),
            (
                slicer_ir::PaintValue::Scalar(-0.0),
                PreparationPaintValue::Scalar(-0.0),
            ),
            (
                slicer_ir::PaintValue::ToolIndex(u32::MAX),
                PreparationPaintValue::ToolIndex(u32::MAX),
            ),
            (
                slicer_ir::PaintValue::Custom("A/π".into()),
                PreparationPaintValue::Custom("A/π".into()),
            ),
        ];
        for (native, expected) in cases {
            let projected = PreparationPaintValue::from(&native);
            assert_eq!(projected, expected);
            // test-quality: the two PaintValue From impls must invert each other,
            // independently of the explicit projected-value oracle above.
            assert_eq!(slicer_ir::PaintValue::from(&projected), native);
        }
        let PreparationPaintValue::Scalar(scalar) =
            PreparationPaintValue::from(&slicer_ir::PaintValue::Scalar(-0.0))
        else {
            panic!("scalar conversion changed the paint variant");
        };
        assert_eq!(scalar.to_bits(), 0x8000_0000);
        assert_ne!(
            PreparationPaintValue::Scalar(-0.0),
            PreparationPaintValue::Scalar(0.0)
        );
    }

    #[test]
    fn region_key_conversions_preserve_ordered_full_identity() {
        let native = slicer_ir::RegionKey {
            global_layer_index: 7,
            object_id: "cube/π".into(),
            region_id: u64::MAX,
            variant_chain: vec![
                ("flag_probe".into(), slicer_ir::PaintValue::Flag(true)),
                ("scalar_probe".into(), slicer_ir::PaintValue::Scalar(-0.0)),
                ("material".into(), slicer_ir::PaintValue::ToolIndex(1)),
                (
                    "custom_route".into(),
                    slicer_ir::PaintValue::Custom("A/π".into()),
                ),
            ],
        };
        let projected = PreparationRegionKey::from(&native);
        assert_eq!(
            projected,
            PreparationRegionKey {
                global_layer_index: 7,
                object_id: "cube/π".into(),
                region_id: u64::MAX,
                variant_chain: vec![
                    ("flag_probe".into(), PreparationPaintValue::Flag(true)),
                    ("scalar_probe".into(), PreparationPaintValue::Scalar(-0.0)),
                    ("material".into(), PreparationPaintValue::ToolIndex(1)),
                    (
                        "custom_route".into(),
                        PreparationPaintValue::Custom("A/π".into())
                    ),
                ],
            }
        );
        let recovered = slicer_ir::RegionKey::from(&projected);
        // test-quality: the two RegionKey From impls must invert each other,
        // independently of the explicit full-key oracle above.
        assert_eq!(recovered, native);
        let slicer_ir::PaintValue::Scalar(scalar) = &recovered.variant_chain[1].1 else {
            panic!("region key conversion changed the scalar variant");
        };
        assert_eq!(scalar.to_bits(), 0x8000_0000);
    }
}
