//! Output capture types for module tests.
//!
//! Each capture mirrors the corresponding SDK output builder shape so tests
//! can inspect categorized outputs without hitting the real host.

use slicer_ir::{ExPolygon, ExtrusionPath3D, Point3, WallLoop};

// ---------------------------------------------------------------------------
// InfillOutputCapture
// ---------------------------------------------------------------------------

/// Capture sink for infill output, mirroring [`slicer_sdk::builders::InfillOutputBuilder`].
///
/// Stores paths by category: sparse, solid, and ironing.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
/// use slicer_sdk::test_support::capture::InfillOutputCapture;
///
/// let mut cap = InfillOutputCapture::new();
/// cap.push_sparse_path(ExtrusionPath3D {
///     points: Vec::new(),
///     role: ExtrusionRole::SparseInfill,
///     speed_factor: 1.0,
/// });
/// assert_eq!(cap.sparse_paths().len(), 1);
/// ```
#[derive(Debug, Default)]
pub struct InfillOutputCapture {
    sparse_paths: Vec<ExtrusionPath3D>,
    solid_paths: Vec<ExtrusionPath3D>,
    ironing_paths: Vec<ExtrusionPath3D>,
}

impl InfillOutputCapture {
    /// Create an empty capture sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a sparse infill path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
    /// use slicer_sdk::test_support::capture::InfillOutputCapture;
    ///
    /// let mut cap = InfillOutputCapture::new();
    /// cap.push_sparse_path(ExtrusionPath3D {
    ///     points: Vec::new(),
    ///     role: ExtrusionRole::SparseInfill,
    ///     speed_factor: 1.0,
    /// });
    /// assert_eq!(cap.sparse_paths().len(), 1);
    /// ```
    pub fn push_sparse_path(&mut self, path: ExtrusionPath3D) {
        self.sparse_paths.push(path);
    }

    /// Push a solid infill path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
    /// use slicer_sdk::test_support::capture::InfillOutputCapture;
    ///
    /// let mut cap = InfillOutputCapture::new();
    /// cap.push_solid_path(ExtrusionPath3D {
    ///     points: Vec::new(),
    ///     role: ExtrusionRole::TopSolidInfill,
    ///     speed_factor: 1.0,
    /// });
    /// assert_eq!(cap.solid_paths().len(), 1);
    /// ```
    pub fn push_solid_path(&mut self, path: ExtrusionPath3D) {
        self.solid_paths.push(path);
    }

    /// Push an ironing path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
    /// use slicer_sdk::test_support::capture::InfillOutputCapture;
    ///
    /// let mut cap = InfillOutputCapture::new();
    /// cap.push_ironing_path(ExtrusionPath3D {
    ///     points: Vec::new(),
    ///     role: ExtrusionRole::Ironing,
    ///     speed_factor: 1.0,
    /// });
    /// assert_eq!(cap.ironing_paths().len(), 1);
    /// ```
    pub fn push_ironing_path(&mut self, path: ExtrusionPath3D) {
        self.ironing_paths.push(path);
    }

    /// Borrow all captured sparse infill paths.
    #[must_use]
    pub fn sparse_paths(&self) -> &[ExtrusionPath3D] {
        &self.sparse_paths
    }

    /// Borrow all captured solid infill paths.
    #[must_use]
    pub fn solid_paths(&self) -> &[ExtrusionPath3D] {
        &self.solid_paths
    }

    /// Borrow all captured ironing paths.
    #[must_use]
    pub fn ironing_paths(&self) -> &[ExtrusionPath3D] {
        &self.ironing_paths
    }
}

// ---------------------------------------------------------------------------
// PerimeterOutputCapture
// ---------------------------------------------------------------------------

/// Capture sink for perimeter output, mirroring [`slicer_sdk::builders::PerimeterOutputBuilder`].
///
/// Stores wall loops, infill areas, and seam candidates.
///
/// # Examples
///
/// ```rust
/// use slicer_sdk::test_support::capture::PerimeterOutputCapture;
///
/// let cap = PerimeterOutputCapture::new();
/// assert!(cap.wall_loops().is_empty());
/// ```
#[derive(Debug, Default)]
pub struct PerimeterOutputCapture {
    wall_loops: Vec<WallLoop>,
    infill_areas: Vec<ExPolygon>,
    seam_candidates: Vec<(Point3, f32)>,
}

impl PerimeterOutputCapture {
    /// Create an empty capture sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a wall loop.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::*;
    /// use slicer_sdk::test_support::capture::PerimeterOutputCapture;
    ///
    /// let mut cap = PerimeterOutputCapture::new();
    /// cap.push_wall_loop(WallLoop {
    ///     perimeter_index: 0,
    ///     loop_type: LoopType::Outer,
    ///     path: ExtrusionPath3D { points: vec![], role: ExtrusionRole::OuterWall, speed_factor: 1.0 },
    ///     width_profile: WidthProfile { widths: vec![] },
    ///     feature_flags: vec![],
    ///     boundary_type: WallBoundaryType::ExteriorSurface,
    /// });
    /// assert_eq!(cap.wall_loops().len(), 1);
    /// ```
    pub fn push_wall_loop(&mut self, loop_: WallLoop) {
        self.wall_loops.push(loop_);
    }

    /// Set the infill areas, replacing any previously set areas.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{ExPolygon, Polygon};
    /// use slicer_sdk::test_support::capture::PerimeterOutputCapture;
    ///
    /// let mut cap = PerimeterOutputCapture::new();
    /// cap.set_infill_areas(vec![ExPolygon { contour: Polygon { points: vec![] }, holes: vec![] }]);
    /// assert_eq!(cap.infill_areas().len(), 1);
    /// ```
    pub fn set_infill_areas(&mut self, areas: Vec<ExPolygon>) {
        self.infill_areas = areas;
    }

    /// Push a seam candidate position with its score.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::Point3;
    /// use slicer_sdk::test_support::capture::PerimeterOutputCapture;
    ///
    /// let mut cap = PerimeterOutputCapture::new();
    /// cap.push_seam_candidate(Point3 { x: 0.0, y: 0.0, z: 0.0 }, 0.5);
    /// assert_eq!(cap.seam_candidates().len(), 1);
    /// ```
    pub fn push_seam_candidate(&mut self, pos: Point3, score: f32) {
        self.seam_candidates.push((pos, score));
    }

    /// Borrow all captured wall loops.
    #[must_use]
    pub fn wall_loops(&self) -> &[WallLoop] {
        &self.wall_loops
    }

    /// Borrow all captured infill areas.
    #[must_use]
    pub fn infill_areas(&self) -> &[ExPolygon] {
        &self.infill_areas
    }

    /// Borrow all captured seam candidates.
    #[must_use]
    pub fn seam_candidates(&self) -> &[(Point3, f32)] {
        &self.seam_candidates
    }
}

// ---------------------------------------------------------------------------
// SupportOutputCapture
// ---------------------------------------------------------------------------

/// Capture sink for support output, mirroring [`slicer_sdk::builders::SupportOutputBuilder`].
///
/// Stores support paths, interface paths (with `is_top_interface` flag), and raft paths.
///
/// # Examples
///
/// ```rust
/// use slicer_sdk::test_support::capture::SupportOutputCapture;
///
/// let cap = SupportOutputCapture::new();
/// assert!(cap.support_paths().is_empty());
/// ```
#[derive(Debug, Default)]
pub struct SupportOutputCapture {
    support_paths: Vec<ExtrusionPath3D>,
    interface_paths: Vec<(ExtrusionPath3D, bool)>,
    raft_paths: Vec<ExtrusionPath3D>,
}

impl SupportOutputCapture {
    /// Create an empty capture sink.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push a support path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
    /// use slicer_sdk::test_support::capture::SupportOutputCapture;
    ///
    /// let mut cap = SupportOutputCapture::new();
    /// cap.push_support_path(ExtrusionPath3D {
    ///     points: Vec::new(),
    ///     role: ExtrusionRole::SupportMaterial,
    ///     speed_factor: 1.0,
    /// });
    /// assert_eq!(cap.support_paths().len(), 1);
    /// ```
    pub fn push_support_path(&mut self, path: ExtrusionPath3D) {
        self.support_paths.push(path);
    }

    /// Push an interface path with its `is_top_interface` flag.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
    /// use slicer_sdk::test_support::capture::SupportOutputCapture;
    ///
    /// let mut cap = SupportOutputCapture::new();
    /// cap.push_interface_path(ExtrusionPath3D {
    ///     points: Vec::new(),
    ///     role: ExtrusionRole::SupportInterface,
    ///     speed_factor: 1.0,
    /// }, true);
    /// assert_eq!(cap.interface_paths().len(), 1);
    /// assert!(cap.interface_paths()[0].1);
    /// ```
    pub fn push_interface_path(&mut self, path: ExtrusionPath3D, is_top_interface: bool) {
        self.interface_paths.push((path, is_top_interface));
    }

    /// Push a raft path.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
    /// use slicer_sdk::test_support::capture::SupportOutputCapture;
    ///
    /// let mut cap = SupportOutputCapture::new();
    /// cap.push_raft_path(ExtrusionPath3D {
    ///     points: Vec::new(),
    ///     role: ExtrusionRole::SupportMaterial,
    ///     speed_factor: 1.0,
    /// });
    /// assert_eq!(cap.raft_paths().len(), 1);
    /// ```
    pub fn push_raft_path(&mut self, path: ExtrusionPath3D) {
        self.raft_paths.push(path);
    }

    /// Borrow all captured support paths.
    #[must_use]
    pub fn support_paths(&self) -> &[ExtrusionPath3D] {
        &self.support_paths
    }

    /// Borrow all captured interface paths with their `is_top_interface` flags.
    #[must_use]
    pub fn interface_paths(&self) -> &[(ExtrusionPath3D, bool)] {
        &self.interface_paths
    }

    /// Borrow all captured raft paths.
    #[must_use]
    pub fn raft_paths(&self) -> &[ExtrusionPath3D] {
        &self.raft_paths
    }
}
