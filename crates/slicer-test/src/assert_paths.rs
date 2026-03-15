//! Assertion helpers for extrusion path fixtures.

use slicer_ir::ExtrusionPath3D;

/// Assert that every point z-coordinate is within `tolerance` of `expected_z_mm`.
///
/// # Panics
/// Panics if any point is outside the tolerance.
///
/// # Examples
///
/// ```rust
/// use slicer_ir::{ExtrusionPath3D, ExtrusionRole};
/// use slicer_test::assert_paths::assert_paths_planar;
///
/// let paths = vec![ExtrusionPath3D {
///     points: Vec::new(),
///     role: ExtrusionRole::SparseInfill,
///     speed_factor: 1.0,
/// }];
/// assert_paths_planar(&paths, 0.2, 1e-3);
/// ```
pub fn assert_paths_planar(paths: &[ExtrusionPath3D], expected_z_mm: f32, tolerance: f32) {
    for (path_index, path) in paths.iter().enumerate() {
        for (point_index, point) in path.points.iter().enumerate() {
            let delta = (point.z - expected_z_mm).abs();
            assert!(
                delta <= tolerance,
                "path {path_index} point {point_index} has z={} expected {} +/- {}",
                point.z,
                expected_z_mm,
                tolerance
            );
        }
    }
}
