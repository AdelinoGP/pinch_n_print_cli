//! Output builder types for writing IR data.
//!
//! These are mutable builders that modules use to produce output.
//! Per docs/03_wit_and_manifest.md, the host validates all writes against
//! declared ir-access.writes at call time.

use std::collections::HashMap;

use slicer_ir::{
    ExPolygon, ExtrusionPath3D, PaintSemantic, PaintValue, Point3, Point3WithWidth, RegionKey,
    SeamPosition, WallLoop,
};

/// Boundary paint map for a region: semantic -> per-polygon -> per-point paint values.
pub type BoundaryPaintMap = HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>;

/// Builder for infill output.
///
/// Matches WIT `resource infill-output-builder` from ir-types.wit.
/// Methods return Result<(), String> to match WIT error handling.
pub struct InfillOutputBuilder {
    sparse_paths: Vec<ExtrusionPath3D>,
    solid_paths: Vec<ExtrusionPath3D>,
    ironing_paths: Vec<ExtrusionPath3D>,
}

impl InfillOutputBuilder {
    /// Create a new InfillOutputBuilder.
    pub fn new() -> Self {
        Self {
            sparse_paths: Vec::new(),
            solid_paths: Vec::new(),
            ironing_paths: Vec::new(),
        }
    }

    /// Push a sparse infill path.
    pub fn push_sparse_path(&mut self, path: ExtrusionPath3D) -> Result<(), String> {
        self.sparse_paths.push(path);
        Ok(())
    }

    /// Push a solid infill path.
    pub fn push_solid_path(&mut self, path: ExtrusionPath3D) -> Result<(), String> {
        self.solid_paths.push(path);
        Ok(())
    }

    /// Push an ironing path.
    pub fn push_ironing_path(&mut self, path: ExtrusionPath3D) -> Result<(), String> {
        self.ironing_paths.push(path);
        Ok(())
    }

    /// Get all sparse paths (for testing).
    #[doc(hidden)]
    pub fn sparse_paths(&self) -> &[ExtrusionPath3D] {
        &self.sparse_paths
    }

    /// Get all solid paths (for testing).
    #[doc(hidden)]
    pub fn solid_paths(&self) -> &[ExtrusionPath3D] {
        &self.solid_paths
    }

    /// Get all ironing paths (for testing).
    #[doc(hidden)]
    pub fn ironing_paths(&self) -> &[ExtrusionPath3D] {
        &self.ironing_paths
    }
}

impl Default for InfillOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for InfillOutputBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InfillOutputBuilder")
            .field("sparse_paths", &self.sparse_paths.len())
            .field("solid_paths", &self.solid_paths.len())
            .field("ironing_paths", &self.ironing_paths.len())
            .finish()
    }
}

/// Builder for perimeter output.
///
/// Matches WIT `resource perimeter-output-builder` from ir-types.wit.
/// Methods return Result<(), String> to match WIT error handling.
pub struct PerimeterOutputBuilder {
    wall_loops: Vec<WallLoop>,
    infill_areas: Vec<ExPolygon>,
    seam_candidates: Vec<(Point3, f32)>,
    resolved_seam: Option<SeamPosition>,
    /// Rotated wall loops with seam at points[0], set by seam-placer.
    rotated_wall_loops: Vec<(Point3WithWidth, u32, WallLoop)>,
}

impl PerimeterOutputBuilder {
    /// Create a new PerimeterOutputBuilder.
    pub fn new() -> Self {
        Self {
            wall_loops: Vec::new(),
            infill_areas: Vec::new(),
            seam_candidates: Vec::new(),
            resolved_seam: None,
            rotated_wall_loops: Vec::new(),
        }
    }

    /// Push a wall loop.
    pub fn push_wall_loop(&mut self, loop_: WallLoop) -> Result<(), String> {
        self.wall_loops.push(loop_);
        Ok(())
    }

    /// Set the infill areas.
    pub fn set_infill_areas(&mut self, areas: Vec<ExPolygon>) -> Result<(), String> {
        self.infill_areas = areas;
        Ok(())
    }

    /// Push a seam candidate.
    pub fn push_seam_candidate(&mut self, pos: Point3, score: f32) -> Result<(), String> {
        self.seam_candidates.push((pos, score));
        Ok(())
    }

    /// Set the resolved seam position.
    pub fn set_resolved_seam(
        &mut self,
        point: Point3WithWidth,
        wall_index: u32,
    ) -> Result<(), String> {
        self.resolved_seam = Some(SeamPosition { point, wall_index });
        Ok(())
    }

    /// Push a wall loop with the seam at points[0] (rotated).
    ///
    /// Used by seam-placer during `Layer::WallPostProcess` to commit
    /// seam-first wall loop geometry. The `pos` and `wall_index` are
    /// the resolved seam reference; the `rotated_loop` contains the
    /// wall loop with path.points[0] as the seam vertex.
    ///
    /// When the SDK builder is drained back to the WIT boundary, this
    /// emits via `perimeter-output-builder.push-reordered-wall-loop`.
    pub fn push_reordered_wall_loop(
        &mut self,
        pos: Point3WithWidth,
        wall_index: u32,
        loop_: WallLoop,
    ) -> Result<(), String> {
        self.rotated_wall_loops
            .push((pos, wall_index, loop_));
        Ok(())
    }

    /// Get all wall loops (for testing).
    #[doc(hidden)]
    pub fn wall_loops(&self) -> &[WallLoop] {
        &self.wall_loops
    }

    /// Get the rotated wall loops (for testing).
    #[doc(hidden)]
    pub fn rotated_wall_loops(&self) -> &[(Point3WithWidth, u32, WallLoop)] {
        &self.rotated_wall_loops
    }

    /// Get the infill areas (for testing).
    #[doc(hidden)]
    pub fn infill_areas(&self) -> &[ExPolygon] {
        &self.infill_areas
    }

    /// Get all seam candidates (for testing).
    #[doc(hidden)]
    pub fn seam_candidates(&self) -> &[(Point3, f32)] {
        &self.seam_candidates
    }

    /// Get the resolved seam (for testing).
    #[doc(hidden)]
    pub fn resolved_seam(&self) -> Option<&SeamPosition> {
        self.resolved_seam.as_ref()
    }
}

impl Default for PerimeterOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for PerimeterOutputBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerimeterOutputBuilder")
            .field("wall_loops", &self.wall_loops.len())
            .field("infill_areas", &self.infill_areas.len())
            .field("seam_candidates", &self.seam_candidates.len())
            .field("resolved_seam", &self.resolved_seam.is_some())
            .field("rotated_wall_loops", &self.rotated_wall_loops.len())
            .finish()
    }
}

/// Builder for support output.
///
/// Matches WIT `resource support-output-builder` from ir-types.wit.
/// Methods return Result<(), String> to match WIT error handling.
pub struct SupportOutputBuilder {
    support_paths: Vec<ExtrusionPath3D>,
    interface_paths: Vec<(ExtrusionPath3D, bool)>, // (path, is_top_interface)
    raft_paths: Vec<ExtrusionPath3D>,
}

impl SupportOutputBuilder {
    /// Create a new SupportOutputBuilder.
    pub fn new() -> Self {
        Self {
            support_paths: Vec::new(),
            interface_paths: Vec::new(),
            raft_paths: Vec::new(),
        }
    }

    /// Push a support path.
    pub fn push_support_path(&mut self, path: ExtrusionPath3D) -> Result<(), String> {
        self.support_paths.push(path);
        Ok(())
    }

    /// Push an interface path.
    pub fn push_interface_path(
        &mut self,
        path: ExtrusionPath3D,
        is_top_interface: bool,
    ) -> Result<(), String> {
        self.interface_paths.push((path, is_top_interface));
        Ok(())
    }

    /// Push a raft path.
    pub fn push_raft_path(&mut self, path: ExtrusionPath3D) -> Result<(), String> {
        self.raft_paths.push(path);
        Ok(())
    }

    /// Get all support paths (for testing).
    #[doc(hidden)]
    pub fn support_paths(&self) -> &[ExtrusionPath3D] {
        &self.support_paths
    }

    /// Get all interface paths (for testing).
    #[doc(hidden)]
    pub fn interface_paths(&self) -> &[(ExtrusionPath3D, bool)] {
        &self.interface_paths
    }

    /// Get all raft paths (for testing).
    #[doc(hidden)]
    pub fn raft_paths(&self) -> &[ExtrusionPath3D] {
        &self.raft_paths
    }
}

impl Default for SupportOutputBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SupportOutputBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SupportOutputBuilder")
            .field("support_paths", &self.support_paths.len())
            .field("interface_paths", &self.interface_paths.len())
            .field("raft_paths", &self.raft_paths.len())
            .finish()
    }
}

/// Builder for slice postprocess output.
///
/// Matches WIT `resource slice-postprocess-builder` from ir-types.wit.
pub struct SlicePostprocessBuilder {
    polygon_updates: Vec<(RegionKey, Vec<ExPolygon>)>,
    path_z_updates: Vec<(RegionKey, u32, u32, f32)>,
    boundary_paint_updates: Vec<(RegionKey, BoundaryPaintMap)>,
}

impl SlicePostprocessBuilder {
    /// Create a new SlicePostprocessBuilder.
    pub fn new() -> Self {
        Self {
            polygon_updates: Vec::new(),
            path_z_updates: Vec::new(),
            boundary_paint_updates: Vec::new(),
        }
    }

    /// Set polygons for a region.
    pub fn set_polygons(&mut self, region: RegionKey, polys: Vec<ExPolygon>) -> Result<(), String> {
        self.polygon_updates.push((region, polys));
        Ok(())
    }

    /// Set path Z for a specific vertex.
    pub fn set_path_z(
        &mut self,
        region: RegionKey,
        path_idx: u32,
        vertex_idx: u32,
        z: f32,
    ) -> Result<(), String> {
        self.path_z_updates.push((region, path_idx, vertex_idx, z));
        Ok(())
    }

    /// Set boundary paint for a region.
    ///
    /// The boundary_paint map is keyed by `PaintSemantic`. For each semantic,
    /// the outer Vec has one entry per polygon in the region's `polygons`,
    /// and the inner Vec has one entry per contour point in that polygon's
    /// outer contour. Each entry is `Some(PaintValue)` if the point is inside
    /// a painted region, or `None` if unpainted.
    pub fn set_boundary_paint(
        &mut self,
        region: RegionKey,
        boundary_paint: BoundaryPaintMap,
    ) -> Result<(), String> {
        self.boundary_paint_updates.push((region, boundary_paint));
        Ok(())
    }

    /// Get all boundary paint updates (for testing).
    #[doc(hidden)]
    pub fn boundary_paint_updates(&self) -> &[(RegionKey, BoundaryPaintMap)] {
        &self.boundary_paint_updates
    }

    /// Get all polygon updates (for testing and macro drain-back).
    #[doc(hidden)]
    pub fn polygon_updates(&self) -> &[(RegionKey, Vec<ExPolygon>)] {
        &self.polygon_updates
    }

    /// Get all path-z updates (for testing and macro drain-back).
    #[doc(hidden)]
    pub fn path_z_updates(&self) -> &[(RegionKey, u32, u32, f32)] {
        &self.path_z_updates
    }
}

impl Default for SlicePostprocessBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for SlicePostprocessBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SlicePostprocessBuilder")
            .field("polygon_updates", &self.polygon_updates.len())
            .field("path_z_updates", &self.path_z_updates.len())
            .field("boundary_paint_updates", &self.boundary_paint_updates.len())
            .finish()
    }
}
