//! Single-Z-plane cross-section helper.
//!
//! Thin wrapper around [`crate::slice_mesh_ex`] (see
//! `crates/slicer-core/src/triangle_mesh_slicer.rs`) for callers that only
//! need the polygons at one height and don't want to manage the
//! multi-Z batch API themselves. No plane-triangle intersection logic is
//! re-implemented here — this module only selects/flattens the single-Z
//! result out of the existing slicer's batch output.

use crate::slice_mesh_ex;
use slicer_ir::{ExPolygon, IndexedTriangleSet};

/// Returns the 2D cross-section of `mesh` at height `z`.
///
/// # Unit convention
///
/// `z` and all mesh vertex coordinates are in **millimeters** (unscaled
/// `f32`), matching [`IndexedTriangleSet`]'s existing convention (see
/// `slice_mesh_ex`'s doc-comment in `triangle_mesh_slicer.rs`) — **not**
/// the `1 unit = 100 nm` fixed-point integer convention used elsewhere in
/// the codebase (e.g. `Point2`/`Point3` scaled coordinates via
/// `mm_to_units`). Callers that hold scaled integer Z values must convert
/// to mm before calling this function.
///
/// This function takes a bare [`IndexedTriangleSet`] (single object,
/// already in world/local mesh space) rather than a full `MeshIR`, since
/// `MeshIR` is a multi-object container and resolving which object(s) and
/// whether to apply `Transform3d` is a caller-level decision, not part of
/// this helper's contract. Callers slicing a `MeshIR` object should pass
/// `object_mesh.mesh` (applying any needed transform beforehand).
///
/// # Returns
///
/// The `ExPolygon`s at `z`, or an empty `Vec` if the mesh is empty or `z`
/// falls outside the mesh's Z extent.
pub fn cross_section_at_z(mesh: &IndexedTriangleSet, z: f32) -> Vec<ExPolygon> {
    let zs = [z];
    let mut layers = slice_mesh_ex(mesh, &zs);
    layers.pop().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::{Point2, Point3};

    /// Builds a unit cube (0..1 mm in x/y/z), matching the fixture used by
    /// `triangle_mesh_slicer`'s own tests (same winding convention: bottom
    /// face CW-from-above, top face CCW-from-above, side faces skipped).
    fn unit_cube_mesh() -> IndexedTriangleSet {
        let vertices = vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 1.0,
            },
        ];
        #[rustfmt::skip]
        let indices = vec![
            0, 1, 2,  0, 2, 3,
            4, 5, 6,  4, 6, 7,
            0, 1, 5,  0, 5, 4,
            1, 2, 6,  1, 6, 5,
            2, 3, 7,  2, 7, 6,
            3, 0, 4,  3, 4, 7,
        ];
        IndexedTriangleSet { vertices, indices }
    }

    #[test]
    fn cube_cross_section_at_mid_height_yields_one_expolygon_matching_unit_square() {
        let mesh = unit_cube_mesh();
        let polys = cross_section_at_z(&mesh, 0.5);

        assert_eq!(polys.len(), 1, "expected a single ExPolygon cross-section");

        // ExPolygon coordinates come out of `slice_mesh_ex` in scaled
        // fixed-point units (1 unit = 100 nm => 1 mm = 10_000 units).
        let pts: Vec<Point2> = polys[0].contour.points.clone();
        let min_x = pts.iter().map(|p| p.x).min().unwrap();
        let max_x = pts.iter().map(|p| p.x).max().unwrap();
        let min_y = pts.iter().map(|p| p.y).min().unwrap();
        let max_y = pts.iter().map(|p| p.y).max().unwrap();
        assert_eq!(min_x, 0);
        assert_eq!(max_x, 10_000, "1 mm should be 10_000 scaled units");
        assert_eq!(min_y, 0);
        assert_eq!(max_y, 10_000, "1 mm should be 10_000 scaled units");
    }

    #[test]
    fn cross_section_outside_mesh_extent_is_empty() {
        let mesh = unit_cube_mesh();
        let polys = cross_section_at_z(&mesh, 5.0);
        assert!(polys.is_empty());
    }
}
