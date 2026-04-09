//! Output builder types for writing IR data.
//!
//! These are mutable builders that modules use to produce output.
//! Per docs/03_wit_and_manifest.md, the host validates all writes against
//! declared ir-access.writes at call time.

use slicer_ir::{ExPolygon, ExtrusionPath3D, Point3, WallLoop};

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
    pub fn push_sparse_path(&mut self, _path: ExtrusionPath3D) -> Result<(), String> {
        todo!("TASK-042: implement InfillOutputBuilder::push_sparse_path")
    }

    /// Push a solid infill path.
    pub fn push_solid_path(&mut self, _path: ExtrusionPath3D) -> Result<(), String> {
        todo!("TASK-042: implement InfillOutputBuilder::push_solid_path")
    }

    /// Push an ironing path.
    pub fn push_ironing_path(&mut self, _path: ExtrusionPath3D) -> Result<(), String> {
        todo!("TASK-042: implement InfillOutputBuilder::push_ironing_path")
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
}

impl PerimeterOutputBuilder {
    /// Create a new PerimeterOutputBuilder.
    pub fn new() -> Self {
        Self {
            wall_loops: Vec::new(),
            infill_areas: Vec::new(),
            seam_candidates: Vec::new(),
        }
    }

    /// Push a wall loop.
    pub fn push_wall_loop(&mut self, _loop_: WallLoop) -> Result<(), String> {
        todo!("TASK-042: implement PerimeterOutputBuilder::push_wall_loop")
    }

    /// Set the infill areas.
    pub fn set_infill_areas(&mut self, _areas: Vec<ExPolygon>) -> Result<(), String> {
        todo!("TASK-042: implement PerimeterOutputBuilder::set_infill_areas")
    }

    /// Push a seam candidate.
    pub fn push_seam_candidate(&mut self, _pos: Point3, _score: f32) -> Result<(), String> {
        todo!("TASK-042: implement PerimeterOutputBuilder::push_seam_candidate")
    }

    /// Get all wall loops (for testing).
    #[doc(hidden)]
    pub fn wall_loops(&self) -> &[WallLoop] {
        &self.wall_loops
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
    pub fn push_support_path(&mut self, _path: ExtrusionPath3D) -> Result<(), String> {
        todo!("TASK-042: implement SupportOutputBuilder::push_support_path")
    }

    /// Push an interface path.
    pub fn push_interface_path(
        &mut self,
        _path: ExtrusionPath3D,
        _is_top_interface: bool,
    ) -> Result<(), String> {
        todo!("TASK-042: implement SupportOutputBuilder::push_interface_path")
    }

    /// Push a raft path.
    pub fn push_raft_path(&mut self, _path: ExtrusionPath3D) -> Result<(), String> {
        todo!("TASK-042: implement SupportOutputBuilder::push_raft_path")
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
    polygon_updates: Vec<(slicer_ir::RegionKey, Vec<ExPolygon>)>,
    path_z_updates: Vec<(slicer_ir::RegionKey, u32, u32, f32)>,
}

impl SlicePostprocessBuilder {
    /// Create a new SlicePostprocessBuilder.
    pub fn new() -> Self {
        Self {
            polygon_updates: Vec::new(),
            path_z_updates: Vec::new(),
        }
    }

    /// Set polygons for a region.
    pub fn set_polygons(
        &mut self,
        _region: slicer_ir::RegionKey,
        _polys: Vec<ExPolygon>,
    ) -> Result<(), String> {
        todo!("TASK-042: implement SlicePostprocessBuilder::set_polygons")
    }

    /// Set path Z for a specific vertex.
    pub fn set_path_z(
        &mut self,
        _region: slicer_ir::RegionKey,
        _path_idx: u32,
        _vertex_idx: u32,
        _z: f32,
    ) -> Result<(), String> {
        todo!("TASK-042: implement SlicePostprocessBuilder::set_path_z")
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
            .finish()
    }
}
