//! Prepass stage types for SDK.
//!
//! These types correspond to the WIT definitions in docs/03_wit_and_manifest.md (world-prepass.wit).
//! They are used by PrepassModule implementations for mesh analysis and layer planning stages.

use serde::{Deserialize, Serialize};
use slicer_ir::Point3WithWidth;

/// Type alias for object IDs (per ir-types.wit: `type object-id = string`).
pub type ObjectId = String;

/// Type alias for region IDs (per ir-types.wit: `type region-id = string`).
pub type RegionId = String;

/// Classification of mesh facets for surface analysis.
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit):
/// ```wit
/// enum facet-class { normal, near-horizontal, overhang, bridge, top-surface, bottom-surface }
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacetClass {
    /// Normal surface with standard slope.
    #[default]
    Normal,
    /// Near-horizontal surface (close to build plate plane).
    NearHorizontal,
    /// Overhang surface requiring support consideration.
    Overhang,
    /// Bridge surface spanning between supports.
    Bridge,
    /// Top-facing surface (final layer for a region).
    TopSurface,
    /// Bottom-facing surface (first layer for a region).
    BottomSurface,
}

/// Annotation for a single mesh facet.
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit):
/// ```wit
/// record facet-annotation { facet-index: u32, slope-angle-deg: f32, classification: facet-class }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FacetAnnotation {
    /// Index of the facet in the mesh.
    pub facet_index: u32,
    /// Slope angle in degrees (0 = horizontal, 90 = vertical).
    pub slope_angle_deg: f32,
    /// Classification of the facet for processing decisions.
    pub classification: FacetClass,
}

/// Proposal for grouping related surface facets.
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit):
/// ```wit
/// record surface-group-proposal { facet-indices: list<u32>, z-min: f32, z-max: f32, shell-count: u32 }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SurfaceGroupProposal {
    /// Indices of facets belonging to this surface group.
    pub facet_indices: Vec<u32>,
    /// Minimum Z coordinate of the group.
    pub z_min: f32,
    /// Maximum Z coordinate of the group.
    pub z_max: f32,
    /// Number of shells (perimeters) for this group.
    pub shell_count: u32,
}

/// Proposal for a region's participation in a layer.
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit):
/// ```wit
/// record region-layer-proposal {
///     object-id: object-id, region-id: region-id,
///     effective-layer-height: f32,
///     is-catchup: bool, catchup-z-bottom: f32,
/// }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegionLayerProposal {
    /// Object this region belongs to.
    pub object_id: ObjectId,
    /// Region identifier within the object.
    pub region_id: RegionId,
    /// Effective layer height for this region at this layer.
    pub effective_layer_height: f32,
    /// Whether this is a catch-up layer (adaptive layer height).
    pub is_catchup: bool,
    /// Bottom Z coordinate for catch-up layers.
    pub catchup_z_bottom: f32,
}

/// A read-only view of an object's mesh and paint data.
///
/// Used by `PrepassModule::run_mesh_segmentation` to provide mesh geometry
/// and paint stroke information to segmentation modules.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MeshObjectView {
    /// Unique identifier of the object.
    pub object_id: String,
    /// Mesh vertices as `[x, y, z]` coordinates.
    pub vertices: Vec<[f32; 3]>,
    /// Triangle indices (each triple indexes into `vertices`).
    pub triangles: Vec<[u32; 3]>,
    /// Paint layers associated with this object's mesh.
    pub paint_layers: Vec<PaintLayerView>,
}

/// A read-only view of a single paint layer on an object.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PaintLayerView {
    /// The paint semantic (e.g. "support_enforcer", "seam").
    pub semantic: String,
    /// Per-facet paint values, parallel to `MeshObjectView::triangles`.
    pub facet_values: Vec<Option<PaintValueView>>,
    /// Sub-facet paint strokes that need normalization.
    pub strokes: Vec<PaintStrokeView>,
}

/// A paint value that can represent a flag, scalar, or tool index.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PaintValueView {
    /// Kind discriminator: `"flag"`, `"scalar"`, or `"tool_index"`.
    pub kind: String,
    /// Flag value (when `kind == "flag"`).
    pub flag: Option<bool>,
    /// Scalar value (when `kind == "scalar"`).
    pub scalar: Option<f32>,
    /// Tool index value (when `kind == "tool_index"`).
    pub tool_index: Option<u32>,
}

/// A sub-facet paint stroke that needs to be resolved into whole-facet values.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PaintStrokeView {
    /// Stroke triangles as vertex positions `[[x,y,z]; 3]`.
    pub triangles: Vec<[[f32; 3]; 3]>,
    /// The paint semantic this stroke belongs to.
    pub semantic: String,
    /// The paint value this stroke carries.
    pub value: PaintValueView,
}

/// View of an object for paint segmentation, with transform and layer participation.
///
/// Used by `PrepassModule::run_paint_segmentation` to provide mesh geometry,
/// paint layer data, transform, and participating layer indices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintSegmentationObjectView {
    /// Unique identifier of the object.
    pub object_id: String,
    /// Mesh vertices as `[x, y, z]` coordinates.
    pub vertices: Vec<[f32; 3]>,
    /// Triangle indices (each triple indexes into `vertices`).
    pub triangles: Vec<[u32; 3]>,
    /// Paint layers associated with this object's mesh.
    pub paint_layers: Vec<PaintLayerView>,
    /// 4x4 column-major transform matrix.
    pub transform_matrix: [f64; 16],
    /// Global layer indices this object participates in.
    pub participating_layer_indices: Vec<u32>,
}

impl Default for PaintSegmentationObjectView {
    fn default() -> Self {
        Self {
            object_id: String::new(),
            vertices: Vec::new(),
            triangles: Vec::new(),
            paint_layers: Vec::new(),
            transform_matrix: [0.0; 16],
            participating_layer_indices: Vec::new(),
        }
    }
}

impl PaintSegmentationObjectView {
    /// Returns the object ID.
    pub fn object_id(&self) -> &str {
        &self.object_id
    }

    /// Returns the vertices.
    pub fn vertices(&self) -> &[[f32; 3]] {
        &self.vertices
    }

    /// Returns the triangles.
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }

    /// Returns the paint layers.
    pub fn paint_layers(&self) -> &[PaintLayerView] {
        &self.paint_layers
    }

    /// Returns the transform matrix.
    pub fn transform_matrix(&self) -> &[f64; 16] {
        &self.transform_matrix
    }

    /// Returns the participating layer indices.
    pub fn participating_layer_indices(&self) -> &[u32] {
        &self.participating_layer_indices
    }
}

/// Proposal for a complete layer in the slice plan.
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit):
/// ```wit
/// record layer-proposal { z: f32, active-regions: list<region-layer-proposal> }
/// ```
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerProposal {
    /// Z coordinate of this layer.
    pub z: f32,
    /// Regions active at this layer.
    pub active_regions: Vec<RegionLayerProposal>,
}

/// Reason tag for seam scoring (mirrors WIT `seam-reason`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SeamReason {
    /// Semantic tag describing why this candidate scored this way.
    pub tag: String,
}

/// One scored seam candidate from the prepass planner.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ScoredSeamCandidate {
    /// 3D position with extrusion width.
    pub position: Point3WithWidth,
    /// Numerical score (higher = better).
    pub score: f32,
    /// Human-readable reason for the score.
    pub reason: SeamReason,
}

/// One entry in the global seam plan.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SeamPlanEntry {
    /// Global layer index for this seam plan entry.
    pub global_layer_index: u32,
    /// Object this entry belongs to.
    pub object_id: ObjectId,
    /// Region identifier within the object.
    pub region_id: RegionId,
    /// The chosen seam position for this region at this layer.
    pub chosen_position: Point3WithWidth,
    /// Wall index the chosen seam belongs to (0 = outermost).
    pub chosen_wall_index: u32,
    /// All scored candidates considered, including the chosen one.
    pub scored_candidates: Vec<ScoredSeamCandidate>,
}

/// One entry in the global support plan.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SupportPlanEntry {
    /// Global layer index for this support plan entry.
    /// Negative values (`-1`, `-2`, ...) are reserved for raft prefix layers.
    /// Non-negative values (`0`, `1`, ...) refer to model layers.
    pub global_layer_index: i32,
    /// Object this entry belongs to.
    pub object_id: ObjectId,
    /// Region identifier within the object.
    pub region_id: RegionId,
    /// Planned branch geometry: each inner `Vec<Point3WithWidth>` is a single
    /// polyline branch (typically a two-point MST edge).
    pub branch_segments: Vec<Vec<Point3WithWidth>>,
}

/// Entry in the layer plan view, representing one layer's metadata.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerPlanViewEntry {
    /// Global layer index (0-based).
    pub global_layer_index: u32,
    /// Z coordinate of this layer in mm.
    pub z: f32,
    /// Effective layer height for this layer in mm.
    pub effective_layer_height: f32,
}

/// Read-only view of the committed LayerPlanIR layers.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerPlanView {
    /// Ordered list of layer entries (ascending by global_layer_index).
    pub layers: Vec<LayerPlanViewEntry>,
}

/// Entry in the region segmentation view, listing regions for one (object, layer) pair.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegionSegmentationViewEntry {
    /// Object this entry belongs to.
    pub object_id: ObjectId,
    /// Global layer index.
    pub layer_index: u32,
    /// Region IDs active for this object on this layer (ascending order).
    pub region_ids: Vec<RegionId>,
}

/// Read-only view of the committed RegionMapIR entries.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct RegionSegmentationView {
    /// Ordered list of entries (ascending by (layer_index, object_id)).
    pub entries: Vec<RegionSegmentationViewEntry>,
}

/// Entry in the support geometry view, listing coarse outlines for one support layer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SupportGeometryViewEntry {
    /// u32::MAX sentinel = intermediate model-resolution layer
    pub global_support_layer_index: u32,
    /// Object this entry belongs to.
    pub object_id: ObjectId,
    /// Region identifier within the object.
    pub region_id: RegionId,
    /// Coarse support outlines at this support layer boundary.
    pub outlines: Vec<slicer_ir::ExPolygon>,
}

/// Read-only view of the committed SupportGeometryIR.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SupportGeometryView {
    /// Ordered list of entries (ascending by (global_support_layer_index, object_id, region_id)).
    pub entries: Vec<SupportGeometryViewEntry>,
}
