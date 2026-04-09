//! View types for reading IR data.
//!
//! These are read-only views that the host constructs and passes to modules.
//! Per docs/03_wit_and_manifest.md (ir-types.wit), view resources cannot be
//! constructed by modules.

use slicer_ir::{ExPolygon, ObjectId, RegionId, SeamCandidate, WallLoop};

/// Read-only view of a slice region.
///
/// Matches WIT `resource slice-region-view` from ir-types.wit.
/// Host constructs these; modules cannot construct them.
#[derive(Debug, Clone)]
pub struct SliceRegionView {
    object_id: ObjectId,
    region_id: RegionId,
    polygons: Vec<ExPolygon>,
    infill_areas: Vec<ExPolygon>,
    effective_layer_height: f32,
    z: f32,
    has_nonplanar: bool,
}

impl SliceRegionView {
    /// Create a new SliceRegionView (host-only, for testing).
    #[doc(hidden)]
    pub fn new(
        object_id: ObjectId,
        region_id: RegionId,
        polygons: Vec<ExPolygon>,
        infill_areas: Vec<ExPolygon>,
        effective_layer_height: f32,
        z: f32,
        has_nonplanar: bool,
    ) -> Self {
        Self {
            object_id,
            region_id,
            polygons,
            infill_areas,
            effective_layer_height,
            z,
            has_nonplanar,
        }
    }

    /// Returns the object ID this region belongs to.
    pub fn object_id(&self) -> &ObjectId {
        &self.object_id
    }

    /// Returns the region ID.
    pub fn region_id(&self) -> &RegionId {
        &self.region_id
    }

    /// Returns the slice polygons for this region.
    pub fn polygons(&self) -> &[ExPolygon] {
        &self.polygons
    }

    /// Returns the infill areas for this region.
    pub fn infill_areas(&self) -> &[ExPolygon] {
        &self.infill_areas
    }

    /// Returns the effective layer height at this Z.
    pub fn effective_layer_height(&self) -> f32 {
        self.effective_layer_height
    }

    /// Returns the Z height of this region.
    pub fn z(&self) -> f32 {
        self.z
    }

    /// Returns true if this region has non-planar surfaces.
    pub fn has_nonplanar(&self) -> bool {
        self.has_nonplanar
    }
}

/// Read-only view of a perimeter region.
///
/// Matches WIT `resource perimeter-region-view` from ir-types.wit.
/// Host constructs these; modules cannot construct them.
#[derive(Debug, Clone)]
pub struct PerimeterRegionView {
    object_id: ObjectId,
    region_id: RegionId,
    wall_loops: Vec<WallLoop>,
    infill_areas: Vec<ExPolygon>,
    seam_candidates: Vec<SeamCandidate>,
}

impl PerimeterRegionView {
    /// Create a new PerimeterRegionView (host-only, for testing).
    #[doc(hidden)]
    pub fn new(
        object_id: ObjectId,
        region_id: RegionId,
        wall_loops: Vec<WallLoop>,
        infill_areas: Vec<ExPolygon>,
        seam_candidates: Vec<SeamCandidate>,
    ) -> Self {
        Self {
            object_id,
            region_id,
            wall_loops,
            infill_areas,
            seam_candidates,
        }
    }

    /// Returns the object ID this region belongs to.
    pub fn object_id(&self) -> &ObjectId {
        &self.object_id
    }

    /// Returns the region ID.
    pub fn region_id(&self) -> &RegionId {
        &self.region_id
    }

    /// Returns the wall loops for this region.
    pub fn wall_loops(&self) -> &[WallLoop] {
        &self.wall_loops
    }

    /// Returns the infill areas for this region.
    pub fn infill_areas(&self) -> &[ExPolygon] {
        &self.infill_areas
    }

    /// Returns the seam candidates for this region.
    pub fn seam_candidates(&self) -> &[SeamCandidate] {
        &self.seam_candidates
    }
}
