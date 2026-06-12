//! Stage I/O types for the prepass pipeline.
//!
//! These types describe the outputs a prepass stage can produce
//! (`PrepassStageOutput`) and the auxiliary mesh-analysis payloads
//! forwarded from guest WASM modules via the WIT `mesh-analysis-output`
//! resource (`MeshAnalysisAuxiliary` and its supporting records).

use std::sync::Arc;

use slicer_ir::{
    LayerPlanIR, MeshSegmentationIR, RegionMapIR, SeamPlanIR, SupportGeometryIR, SupportPlanIR,
    SurfaceClassificationIR,
};

/// One committed output produced by a prepass stage invocation.
#[derive(Debug, Clone)]
pub enum PrepassStageOutput {
    /// Stage produced no blackboard commit.
    None,
    /// Stage produced `SurfaceClassificationIR`.
    SurfaceClassification(Arc<SurfaceClassificationIR>),
    /// Stage produced `MeshSegmentationIR`.
    MeshSegmentation(Arc<MeshSegmentationIR>),
    /// Stage produced `LayerPlanIR`.
    LayerPlan(Arc<LayerPlanIR>),
    /// Stage produced `SeamPlanIR`.
    SeamPlan(Arc<SeamPlanIR>),
    /// Stage produced `SupportPlanIR`.
    SupportPlan(Arc<SupportPlanIR>),
    /// Stage produced `RegionMapIR`.
    RegionMap(Arc<RegionMapIR>),
    /// Stage produced `SupportGeometryIR`.
    SupportGeometry(Arc<SupportGeometryIR>),
    /// Guest-emitted mesh-analysis pushes collected via the
    /// `mesh-analysis-output` WIT resource. This variant carries the raw
    /// `(object_id, FacetAnnotation)` / `(object_id, SurfaceGroupProposal)`
    /// pairs the macro-path drain forwarded from the SDK builder; it does
    /// **not** commit to the blackboard because
    /// `SurfaceClassificationIR` is still owned by the host built-in
    /// (`mesh_analysis::execute_mesh_analysis`). The variant exists to
    /// let the prepass dispatcher surface the drained output so tests and
    /// future consumers can observe what reached the host.
    MeshAnalysisAuxiliary(Arc<MeshAnalysisAuxiliary>),
}

/// Raw mesh-analysis output drained from a guest's
/// `mesh-analysis-output` WIT resource. Insertion order is preserved
/// exactly as the guest pushed, so downstream consumers can rely on
/// deterministic sequencing.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshAnalysisAuxiliary {
    /// Per-object facet annotations in push order.
    pub facet_annotations: Vec<(String, FacetAnnotationRecord)>,
    /// Per-object surface-group proposals in push order.
    pub surface_groups: Vec<(String, SurfaceGroupRecord)>,
}

/// Host-side mirror of the WIT `facet-annotation` record, decoupled
/// from the wit-bindgen-generated types so the `PrepassStageOutput`
/// enum does not depend on the generated module.
#[derive(Debug, Clone, PartialEq)]
pub struct FacetAnnotationRecord {
    /// Triangle index in the object's mesh.
    pub facet_index: u32,
    /// Slope angle of the facet normal in degrees.
    pub slope_angle_deg: f32,
    /// Classification label.
    pub classification: FacetClassRecord,
}

/// Host-side mirror of the WIT `facet-class` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetClassRecord {
    /// No special classification.
    Normal,
    /// Nearly-horizontal surface (top/bottom candidate).
    NearHorizontal,
    /// Facet that overhangs printed material below.
    Overhang,
    /// Bridge-suitable facet (horizontal span).
    Bridge,
    /// Top-facing surface.
    TopSurface,
    /// Bottom-facing surface.
    BottomSurface,
}

/// Host-side mirror of the WIT `surface-group-proposal` record.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceGroupRecord {
    /// Facet indices belonging to the group.
    pub facet_indices: Vec<u32>,
    /// Minimum Z coordinate of the group in world space (mm).
    pub z_min: f32,
    /// Maximum Z coordinate of the group in world space (mm).
    pub z_max: f32,
    /// Number of shells (perimeter loops) to emit around the group.
    pub shell_count: u32,
}
