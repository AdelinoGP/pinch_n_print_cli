// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/AABBTreeIndirect.hpp
// Original code owner: Alec Jacobson (libigl) — Copyright (C) 2015 Alec Jacobson <alecjacobson@gmail.com>
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Minimal TASK-014 mesh-query API scaffolding.

use slicer_ir::{BoundingBox3, IndexedTriangleSet, Point3};

const RAY_EPSILON: f32 = 1.0e-6;
const HIT_DEDUP_EPSILON: f32 = 1.0e-5;

/// Acceleration structure for mesh raycasts, bounds queries, and closest-point lookups.
#[derive(Debug, Clone)]
pub struct AabbTree {
    mesh: IndexedTriangleSet,
    triangles: Vec<Triangle>,
    bounds: Option<BoundingBox3>,
}

impl AabbTree {
    /// Creates a query tree over the provided indexed triangle mesh.
    pub fn new(mesh: IndexedTriangleSet) -> Self {
        let triangles = build_triangles(&mesh);
        let bounds = mesh_bounds(&triangles);

        Self {
            mesh,
            triangles,
            bounds,
        }
    }

    /// Returns whether the tree contains no queryable triangles.
    pub fn is_empty(&self) -> bool {
        self.triangles.is_empty()
    }

    /// Returns the mesh-space axis-aligned bounding box, if any triangles exist.
    pub fn bounds(&self) -> Option<BoundingBox3> {
        self.bounds
    }

    /// Returns the nearest hit along a ray, if the mesh is intersected.
    pub fn raycast_first_hit(&self, origin: Point3, dir: Point3) -> Option<RayHit> {
        self.raycast_all_hits(origin, dir).into_iter().next()
    }

    /// Returns every hit along a ray, sorted by increasing distance.
    pub fn raycast_all_hits(&self, origin: Point3, dir: Point3) -> Vec<RayHit> {
        let _ = &self.mesh;

        let mut hits: Vec<RayHit> = self
            .triangles
            .iter()
            .filter_map(|triangle| ray_triangle_intersection(origin, dir, triangle))
            .collect();

        hits.sort_by(ray_hit_cmp);
        hits.dedup_by(|left, right| same_hit(*left, *right));
        hits
    }

    /// Returns the closest point on the mesh to the query point, if any triangles exist.
    pub fn closest_point(&self, point: Point3) -> Option<ClosestPointHit> {
        let _ = &self.mesh;

        self.triangles
            .iter()
            .map(|triangle| {
                let closest = closest_point_on_triangle(point, triangle);
                ClosestPointHit {
                    point: closest,
                    squared_distance: squared_distance(point, closest),
                }
            })
            .min_by(closest_hit_cmp)
    }
}

/// A ray intersection result in mesh space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RayHit {
    /// Distance from the ray origin in millimeters along the supplied direction.
    pub distance: f32,
    /// Intersection point in mesh coordinates.
    pub point: Point3,
}

/// The nearest point on the mesh to a query point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClosestPointHit {
    /// Closest point in mesh coordinates.
    pub point: Point3,
    /// Squared distance from the query point in square millimeters.
    pub squared_distance: f32,
}

#[derive(Debug, Clone, Copy)]
struct Triangle {
    a: Point3,
    b: Point3,
    c: Point3,
}

fn build_triangles(mesh: &IndexedTriangleSet) -> Vec<Triangle> {
    mesh.indices
        .chunks_exact(3)
        .filter_map(|indices| {
            let a = mesh.vertices.get(indices[0] as usize).copied()?;
            let b = mesh.vertices.get(indices[1] as usize).copied()?;
            let c = mesh.vertices.get(indices[2] as usize).copied()?;

            if point_is_finite(a) && point_is_finite(b) && point_is_finite(c) {
                Some(Triangle { a, b, c })
            } else {
                None
            }
        })
        .collect()
}

fn mesh_bounds(triangles: &[Triangle]) -> Option<BoundingBox3> {
    let mut points = triangles
        .iter()
        .flat_map(|triangle| [triangle.a, triangle.b, triangle.c]);
    let first = points.next()?;
    let mut min = first;
    let mut max = first;

    for point in points {
        min.x = min.x.min(point.x);
        min.y = min.y.min(point.y);
        min.z = min.z.min(point.z);
        max.x = max.x.max(point.x);
        max.y = max.y.max(point.y);
        max.z = max.z.max(point.z);
    }

    Some(BoundingBox3 { min, max })
}

fn ray_triangle_intersection(origin: Point3, dir: Point3, triangle: &Triangle) -> Option<RayHit> {
    let edge1 = sub(triangle.b, triangle.a);
    let edge2 = sub(triangle.c, triangle.a);
    let pvec = cross(dir, edge2);
    let det = dot(edge1, pvec);

    if det.abs() <= RAY_EPSILON {
        return None;
    }

    let inv_det = 1.0 / det;
    let tvec = sub(origin, triangle.a);
    let u = dot(tvec, pvec) * inv_det;
    if !(0.0 - RAY_EPSILON..=1.0 + RAY_EPSILON).contains(&u) {
        return None;
    }

    let qvec = cross(tvec, edge1);
    let v = dot(dir, qvec) * inv_det;
    if v < -RAY_EPSILON || u + v > 1.0 + RAY_EPSILON {
        return None;
    }

    let distance = dot(edge2, qvec) * inv_det;
    if !distance.is_finite() || distance < -RAY_EPSILON {
        return None;
    }

    Some(RayHit {
        distance: distance.max(0.0),
        point: add_scaled(origin, dir, distance.max(0.0)),
    })
}

fn closest_point_on_triangle(point: Point3, triangle: &Triangle) -> Point3 {
    let ab = sub(triangle.b, triangle.a);
    let ac = sub(triangle.c, triangle.a);
    let normal = cross(ab, ac);

    if squared_norm(normal) <= RAY_EPSILON * RAY_EPSILON {
        return closest_point_on_degenerate_triangle(point, triangle);
    }

    let ap = sub(point, triangle.a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return triangle.a;
    }

    let bp = sub(point, triangle.b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return triangle.b;
    }

    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return add_scaled(triangle.a, ab, v);
    }

    let cp = sub(point, triangle.c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return triangle.c;
    }

    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return add_scaled(triangle.a, ac, w);
    }

    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let edge = sub(triangle.c, triangle.b);
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return add_scaled(triangle.b, edge, w);
    }

    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    add(triangle.a, add(scale(ab, v), scale(ac, w)))
}

fn closest_point_on_degenerate_triangle(point: Point3, triangle: &Triangle) -> Point3 {
    [
        closest_point_on_segment(point, triangle.a, triangle.b),
        closest_point_on_segment(point, triangle.b, triangle.c),
        closest_point_on_segment(point, triangle.c, triangle.a),
    ]
    .into_iter()
    .min_by(|left, right| {
        let distance_cmp =
            squared_distance(point, *left).total_cmp(&squared_distance(point, *right));
        if distance_cmp.is_eq() {
            point_cmp(*left, *right)
        } else {
            distance_cmp
        }
    })
    .unwrap_or(triangle.a)
}

fn closest_point_on_segment(point: Point3, start: Point3, end: Point3) -> Point3 {
    let segment = sub(end, start);
    let length_sq = squared_norm(segment);
    if length_sq <= RAY_EPSILON * RAY_EPSILON {
        return start;
    }

    let t = (dot(sub(point, start), segment) / length_sq).clamp(0.0, 1.0);
    add_scaled(start, segment, t)
}

fn same_hit(left: RayHit, right: RayHit) -> bool {
    approx_eq(left.distance, right.distance)
        && approx_eq(left.point.x, right.point.x)
        && approx_eq(left.point.y, right.point.y)
        && approx_eq(left.point.z, right.point.z)
}

fn ray_hit_cmp(left: &RayHit, right: &RayHit) -> std::cmp::Ordering {
    let distance_cmp = left.distance.total_cmp(&right.distance);
    if distance_cmp.is_eq() {
        point_cmp(left.point, right.point)
    } else {
        distance_cmp
    }
}

fn closest_hit_cmp(left: &ClosestPointHit, right: &ClosestPointHit) -> std::cmp::Ordering {
    let distance_cmp = left.squared_distance.total_cmp(&right.squared_distance);
    if distance_cmp.is_eq() {
        point_cmp(left.point, right.point)
    } else {
        distance_cmp
    }
}

fn point_cmp(left: Point3, right: Point3) -> std::cmp::Ordering {
    left.x
        .total_cmp(&right.x)
        .then(left.y.total_cmp(&right.y))
        .then(left.z.total_cmp(&right.z))
}

fn point_is_finite(point: Point3) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

fn approx_eq(left: f32, right: f32) -> bool {
    (left - right).abs() <= HIT_DEDUP_EPSILON
}

fn squared_distance(left: Point3, right: Point3) -> f32 {
    let delta = sub(left, right);
    squared_norm(delta)
}

fn squared_norm(point: Point3) -> f32 {
    dot(point, point)
}

fn dot(left: Point3, right: Point3) -> f32 {
    left.x
        .mul_add(right.x, left.y.mul_add(right.y, left.z * right.z))
}

fn cross(left: Point3, right: Point3) -> Point3 {
    Point3 {
        x: left.y.mul_add(right.z, -(left.z * right.y)),
        y: left.z.mul_add(right.x, -(left.x * right.z)),
        z: left.x.mul_add(right.y, -(left.y * right.x)),
    }
}

fn add(left: Point3, right: Point3) -> Point3 {
    Point3 {
        x: left.x + right.x,
        y: left.y + right.y,
        z: left.z + right.z,
    }
}

fn sub(left: Point3, right: Point3) -> Point3 {
    Point3 {
        x: left.x - right.x,
        y: left.y - right.y,
        z: left.z - right.z,
    }
}

fn add_scaled(origin: Point3, direction: Point3, scale: f32) -> Point3 {
    Point3 {
        x: origin.x + (direction.x * scale),
        y: origin.y + (direction.y * scale),
        z: origin.z + (direction.z * scale),
    }
}

fn scale(point: Point3, factor: f32) -> Point3 {
    Point3 {
        x: point.x * factor,
        y: point.y * factor,
        z: point.z * factor,
    }
}

#[cfg(test)]
mod tests {
    use super::AabbTree;
    use slicer_ir::{IndexedTriangleSet, Point3};

    #[test]
    fn raycast_all_hits_deduplicates_coplanar_face_hits() {
        let mesh = IndexedTriangleSet {
            vertices: vec![
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
            ],
            indices: vec![0, 1, 2, 0, 2, 3],
        };

        let tree = AabbTree::new(mesh);
        let hits = tree.raycast_all_hits(
            Point3 {
                x: 0.5,
                y: 0.5,
                z: -1.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        );

        assert_eq!(hits.len(), 1);
        assert_eq!(
            hits[0].point,
            Point3 {
                x: 0.5,
                y: 0.5,
                z: 0.0
            }
        );
    }
}
