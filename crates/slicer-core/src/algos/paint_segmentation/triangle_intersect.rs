// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the ModularSlicer architecture.
// -----------------------------------------------------------------------------
/// Z-plane intersection for triangles.
///
/// Coordinate constants divided by 100 (OrcaSlicer: 1 nm → ModularSlicer: 100 nm).
use slicer_ir::{Point2, Point3};

/// A 2D line segment with scaled-integer endpoints (1 unit = 100 nm).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    /// Start point.
    pub start: Point2,
    /// End point.
    pub end: Point2,
}

/// Intersect a 3D triangle with a horizontal Z plane.
///
/// Returns `Some(Line)` when the triangle crosses the Z plane, `None` otherwise.
pub fn triangle_z_intersection(p0: Point3, p1: Point3, p2: Point3, z: f32) -> Option<Line> {
    let eps = f32::EPSILON * 100.0;

    let classify = |p: &Point3| -> i8 {
        if p.z > z + eps {
            1
        }
        // above
        else if p.z < z - eps {
            -1
        }
        // below
        else {
            0
        } // on plane
    };

    let c0 = classify(&p0);
    let c1 = classify(&p1);
    let c2 = classify(&p2);

    // All on same side or all coplanar: no intersection
    if c0 == c1 && c1 == c2 {
        return None;
    }

    let to_point2 = |p: &Point3| Point2 {
        x: (p.x as f64 * 10000.0).round() as i64,
        y: (p.y as f64 * 10000.0).round() as i64,
    };

    let lerp = |a: &Point3, b: &Point3| -> Point2 {
        let dz = b.z - a.z;
        if dz.abs() < eps {
            return to_point2(a);
        }
        let t = (z - a.z) / dz;
        let x = a.x + (b.x - a.x) * t;
        let y = a.y + (b.y - a.y) * t;
        Point2 {
            x: (x as f64 * 10000.0).round() as i64,
            y: (y as f64 * 10000.0).round() as i64,
        }
    };

    let verts = [&p0, &p1, &p2];
    let codes = [c0, c1, c2];

    // Collect intersection points (up to 2)
    let mut pts: Vec<Point2> = Vec::new();

    for i in 0..3 {
        let j = (i + 1) % 3;
        let ci = codes[i];
        let cj = codes[j];

        // Vertex on plane
        if ci == 0 {
            pts.push(to_point2(verts[i]));
        }

        // Edge crosses plane (one above, one below)
        if ci != 0 && cj != 0 && ci != cj {
            pts.push(lerp(verts[i], verts[j]));
        }
    }

    // Need exactly 2 distinct intersection points
    if pts.len() < 2 {
        return None;
    }

    // Avoid degenerate line (identical endpoints)
    if pts[0] == pts[1] {
        return None;
    }

    Some(Line {
        start: pts[0],
        end: pts[1],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pt3(x: f32, y: f32, z: f32) -> Point3 {
        Point3 { x, y, z }
    }

    #[test]
    fn triangle_fully_above() {
        assert!(triangle_z_intersection(
            pt3(0.0, 0.0, 5.0),
            pt3(10.0, 0.0, 5.0),
            pt3(5.0, 10.0, 5.0),
            1.0
        )
        .is_none());
    }

    #[test]
    fn triangle_fully_below() {
        assert!(triangle_z_intersection(
            pt3(0.0, 0.0, 1.0),
            pt3(10.0, 0.0, 1.0),
            pt3(5.0, 10.0, 1.0),
            5.0
        )
        .is_none());
    }

    #[test]
    fn triangle_crossing_two_edges() {
        let result = triangle_z_intersection(
            pt3(0.0, 0.0, 0.0),
            pt3(10.0, 0.0, 10.0),
            pt3(5.0, 10.0, 10.0),
            5.0,
        );
        assert!(result.is_some());
    }

    #[test]
    fn triangle_crossing_one_edge_vertex_on_plane() {
        // p0 is exactly on the plane, p1 above, p2 below
        let result = triangle_z_intersection(
            pt3(0.0, 0.0, 5.0),
            pt3(10.0, 0.0, 8.0),
            pt3(5.0, 10.0, 2.0),
            5.0,
        );
        assert!(result.is_some());
    }

    #[test]
    fn coplanar_triangle_returns_none() {
        assert!(triangle_z_intersection(
            pt3(0.0, 0.0, 5.0),
            pt3(10.0, 0.0, 5.0),
            pt3(5.0, 10.0, 5.0),
            5.0
        )
        .is_none());
    }
}
