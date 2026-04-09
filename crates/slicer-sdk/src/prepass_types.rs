//! Prepass stage types for SDK.
//!
//! These types correspond to the WIT definitions in docs/03_wit_and_manifest.md (world-prepass.wit).
//! They are used by PrepassModule implementations for mesh analysis and layer planning stages.

use serde::{Deserialize, Serialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FacetClass {
    /// Normal surface with standard slope.
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FacetAnnotation {
    /// Index of the facet in the mesh.
    pub facet_index: u32,
    /// Slope angle in degrees (0 = horizontal, 90 = vertical).
    pub slope_angle_deg: f32,
    /// Classification of the facet for processing decisions.
    pub classification: FacetClass,
}

impl FacetAnnotation {
    /// Create a new FacetAnnotation.
    pub fn new(facet_index: u32, slope_angle_deg: f32, classification: FacetClass) -> Self {
        Self {
            facet_index,
            slope_angle_deg,
            classification,
        }
    }
}

/// Proposal for grouping related surface facets.
///
/// Per docs/03_wit_and_manifest.md (world-prepass.wit):
/// ```wit
/// record surface-group-proposal { facet-indices: list<u32>, z-min: f32, z-max: f32, shell-count: u32 }
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl SurfaceGroupProposal {
    /// Create a new SurfaceGroupProposal.
    pub fn new(facet_indices: Vec<u32>, z_min: f32, z_max: f32, shell_count: u32) -> Self {
        Self {
            facet_indices,
            z_min,
            z_max,
            shell_count,
        }
    }
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl RegionLayerProposal {
    /// Create a new RegionLayerProposal.
    pub fn new(
        object_id: ObjectId,
        region_id: RegionId,
        effective_layer_height: f32,
        is_catchup: bool,
        catchup_z_bottom: f32,
    ) -> Self {
        Self {
            object_id,
            region_id,
            effective_layer_height,
            is_catchup,
            catchup_z_bottom,
        }
    }
}

/// A read-only view of an object's mesh and paint data.
///
/// Used by `PrepassModule::run_mesh_segmentation` to provide mesh geometry
/// and paint stroke information to segmentation modules.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PaintLayerView {
    /// The paint semantic (e.g. "support_enforcer", "seam").
    pub semantic: String,
    /// Per-facet paint values, parallel to `MeshObjectView::triangles`.
    pub facet_values: Vec<Option<PaintValueView>>,
    /// Sub-facet paint strokes that need normalization.
    pub strokes: Vec<PaintStrokeView>,
}

/// A paint value that can represent a flag, scalar, or tool index.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

impl PaintSegmentationObjectView {
    /// Create a new PaintSegmentationObjectView.
    pub fn new(
        object_id: String,
        vertices: Vec<[f32; 3]>,
        triangles: Vec<[u32; 3]>,
        paint_layers: Vec<PaintLayerView>,
        transform_matrix: [f64; 16],
        participating_layer_indices: Vec<u32>,
    ) -> Self {
        Self {
            object_id,
            vertices,
            triangles,
            paint_layers,
            transform_matrix,
            participating_layer_indices,
        }
    }

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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerProposal {
    /// Z coordinate of this layer.
    pub z: f32,
    /// Regions active at this layer.
    pub active_regions: Vec<RegionLayerProposal>,
}

impl LayerProposal {
    /// Create a new LayerProposal.
    pub fn new(z: f32, active_regions: Vec<RegionLayerProposal>) -> Self {
        Self { z, active_regions }
    }
}
