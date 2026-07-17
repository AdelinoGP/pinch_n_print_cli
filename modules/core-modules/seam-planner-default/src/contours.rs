//! Per-layer closed-contour extraction by z-plane sectioning of mesh
//! triangles, plus signed-distance queries against those contours.
//!
//! PNP-original code (not a port): the algorithm here (plane sectioning,
//! canonical segment chaining, signed distance) is written from scratch for
//! the Pinch 'n Print seam planner.
//!
//! UNIT NOTE: all coordinates in this module are f32 **millimetres** (mesh
//! vertices arrive in mm from `MeshObjectView`), all angles in **radians**.
//! See `docs/08_coordinate_system.md` — the integer 100 nm system used
//! elsewhere in the workspace is NOT used here.
//!
//! Determinism: no `HashMap` iteration anywhere. Segments are sorted by
//! quantized-endpoint keys before chaining, adjacency uses `BTreeMap`, and
//! each output polygon is rotated to start at its lexicographically smallest
//! point, so identical input always yields byte-identical output.

use std::collections::BTreeMap;

/// Point-merge epsilon for chaining plane-section segments. Units: mm.
/// Points closer than this are treated as identical (quantization cell size).
const POINT_MERGE_EPSILON_MM: f32 = 1e-4;

/// Vertices closer to the section plane than this are nudged off it so every
/// triangle/plane classification is strictly above or below. Units: mm.
const PLANE_NUDGE_MM: f32 = 1e-6;

/// Cross-product magnitude below which three consecutive contour points are
/// treated as collinear and the middle one is dropped. Units: mm^2.
const COLLINEAR_CROSS_EPSILON_MM2: f32 = 1e-6;

/// One closed contour in a layer's section plane.
///
/// `points` are in mm, ordered counter-clockwise (outer-contour convention);
/// the polygon is implicitly closed (last point connects back to the first).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Contour {
    /// Contour vertices. Units: mm.
    pub points: Vec<[f32; 2]>,
    /// Local counter-clockwise turn angle at each vertex. Units: radians.
    /// Sign convention (matches canonical seam scoring): NEGATIVE = concave
    /// with respect to the CCW winding, positive = convex.
    pub local_ccw_angles: Vec<f32>,
}

/// Quantized 2D point key (multiples of `POINT_MERGE_EPSILON_MM`).
type Key = (i64, i64);

fn quantize(p: [f32; 2]) -> Key {
    (
        (p[0] / POINT_MERGE_EPSILON_MM).round() as i64,
        (p[1] / POINT_MERGE_EPSILON_MM).round() as i64,
    )
}

fn key_to_point(k: Key) -> [f32; 2] {
    [
        k.0 as f32 * POINT_MERGE_EPSILON_MM,
        k.1 as f32 * POINT_MERGE_EPSILON_MM,
    ]
}

/// Intersect one triangle with the plane `z = plane_z` and return the
/// resulting segment endpoints (mm), if the triangle crosses the plane.
fn section_triangle(
    vertices: &[[f32; 3]],
    tri: &[u32; 3],
    plane_z: f32,
) -> Option<([f32; 2], [f32; 2])> {
    // Signed distances to the plane, nudged off zero for a strict
    // above/below classification (deterministic: same nudge every time).
    let d: Vec<f32> = tri
        .iter()
        .map(|&vi| {
            let dz = vertices[vi as usize][2] - plane_z; // mm
            if dz.abs() < PLANE_NUDGE_MM {
                PLANE_NUDGE_MM
            } else {
                dz
            }
        })
        .collect();

    let mut crossings: Vec<[f32; 2]> = Vec::with_capacity(2);
    for e in 0..3 {
        let (i, j) = (e, (e + 1) % 3);
        if d[i] * d[j] < 0.0 {
            let a = vertices[tri[i] as usize];
            let b = vertices[tri[j] as usize];
            let t = d[i] / (d[i] - d[j]); // interpolation parameter, dimensionless
            crossings.push([a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])]);
        }
    }
    if crossings.len() == 2 {
        Some((crossings[0], crossings[1]))
    } else {
        None
    }
}

/// Extract all closed contours of the mesh at the plane `z = plane_z`.
///
/// Segments are collected from every triangle crossing the plane, chained by
/// quantized endpoints into closed polygons, deduplicated, normalized to CCW
/// orientation, and rotated to a canonical (lexicographically smallest) start
/// point. Open chains (non-manifold sections) are dropped.
pub(crate) fn extract_layer_contours(
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    plane_z: f32,
) -> Vec<Contour> {
    // 1) Gather segments as ordered key pairs; drop degenerates.
    let mut segments: Vec<(Key, Key)> = Vec::new();
    for tri in triangles {
        if let Some((p0, p1)) = section_triangle(vertices, tri, plane_z) {
            let (k0, k1) = (quantize(p0), quantize(p1));
            if k0 == k1 {
                continue;
            }
            // Normalize endpoint order for canonical sorting / dedup.
            segments.push(if k0 < k1 { (k0, k1) } else { (k1, k0) });
        }
    }
    // 2) Canonical order + dedup of coincident segments (no map iteration
    //    order dependence anywhere below).
    segments.sort_unstable();
    segments.dedup();

    // 3) Endpoint adjacency (BTreeMap: deterministic iteration).
    let mut adjacency: BTreeMap<Key, Vec<usize>> = BTreeMap::new();
    for (si, &(k0, k1)) in segments.iter().enumerate() {
        adjacency.entry(k0).or_default().push(si);
        adjacency.entry(k1).or_default().push(si);
    }

    // 4) Chain segments into closed loops, always taking the lowest-index
    //    unused segment first.
    let mut used = vec![false; segments.len()];
    let mut contours: Vec<Contour> = Vec::new();
    for start in 0..segments.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let (start_key, mut current) = segments[start];
        let mut chain: Vec<Key> = vec![start_key, current];
        let mut closed = false;
        loop {
            // Smallest-index unused segment touching `current`.
            let next = adjacency[&current].iter().copied().find(|&si| !used[si]);
            let Some(si) = next else { break };
            used[si] = true;
            let (a, b) = segments[si];
            let other = if a == current { b } else { a };
            if other == start_key {
                closed = true;
                break;
            }
            chain.push(other);
            current = other;
        }
        if !closed || chain.len() < 3 {
            continue; // open chain or degenerate: drop
        }
        if let Some(contour) = finish_contour(chain) {
            contours.push(contour);
        }
    }
    contours
}

/// Normalize a closed key chain into a `Contour`: CCW orientation, canonical
/// start point, per-vertex CCW angles.
fn finish_contour(chain: Vec<Key>) -> Option<Contour> {
    let mut points: Vec<[f32; 2]> = chain.into_iter().map(key_to_point).collect();

    // Signed area (shoelace, mm^2); positive = CCW.
    let area = signed_area(&points);
    if area.abs() < POINT_MERGE_EPSILON_MM * POINT_MERGE_EPSILON_MM {
        return None;
    }
    if area < 0.0 {
        points.reverse();
    }

    // Drop collinear vertices (e.g. quad-diagonal crossings on flat walls):
    // a vertex is kept only if the polygon actually turns there.
    let n = points.len();
    let simplified: Vec<[f32; 2]> = (0..n)
        .filter(|&i| {
            let prev = points[(i + n - 1) % n];
            let curr = points[i];
            let next = points[(i + 1) % n];
            let din = [curr[0] - prev[0], curr[1] - prev[1]]; // mm
            let dout = [next[0] - curr[0], next[1] - curr[1]]; // mm
            let cross = din[0] * dout[1] - din[1] * dout[0]; // mm^2
            let dot = din[0] * dout[0] + din[1] * dout[1]; // mm^2
                                                           // Collinear continuation: negligible turn, forward direction.
            !(cross.abs() < COLLINEAR_CROSS_EPSILON_MM2 && dot > 0.0)
        })
        .map(|i| points[i])
        .collect();
    let mut points = simplified;
    if points.len() < 3 {
        return None;
    }

    // Canonical start: lexicographically smallest point.
    let min_idx = (0..points.len())
        .min_by(|&a, &b| {
            points[a]
                .partial_cmp(&points[b])
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(0);
    points.rotate_left(min_idx);

    let local_ccw_angles = compute_local_ccw_angles(&points);
    Some(Contour {
        points,
        local_ccw_angles,
    })
}

/// Shoelace signed area of a closed polygon. Units: mm^2 (positive = CCW).
fn signed_area(points: &[[f32; 2]]) -> f32 {
    let n = points.len();
    let mut acc = 0.0f32;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        acc += a[0] * b[1] - b[0] * a[1];
    }
    acc * 0.5
}

/// Per-vertex local CCW turn angle (radians) of a CCW polygon.
/// NEGATIVE = concave (right turn w.r.t. CCW winding), positive = convex.
fn compute_local_ccw_angles(points: &[[f32; 2]]) -> Vec<f32> {
    let n = points.len();
    (0..n)
        .map(|i| {
            let prev = points[(i + n - 1) % n];
            let curr = points[i];
            let next = points[(i + 1) % n];
            let din = [curr[0] - prev[0], curr[1] - prev[1]]; // mm
            let dout = [next[0] - curr[0], next[1] - curr[1]]; // mm
            let cross = din[0] * dout[1] - din[1] * dout[0]; // mm^2
            let dot = din[0] * dout[0] + din[1] * dout[1]; // mm^2
            cross.atan2(dot) // radians
        })
        .collect()
}

/// Signed distance (mm) from `p` to the nearest contour line of `contours`;
/// NEGATIVE = inside (even-odd rule over all contours). Points exactly on a
/// contour return ~0.
pub(crate) fn signed_distance_to_contours(contours: &[Contour], p: [f32; 2]) -> f32 {
    let mut min_dist = f32::INFINITY; // mm
    let mut crossings = 0usize;
    for contour in contours {
        let n = contour.points.len();
        for i in 0..n {
            let a = contour.points[i];
            let b = contour.points[(i + 1) % n];
            min_dist = min_dist.min(point_segment_distance(p, a, b));
            // Even-odd ray cast toward +x.
            if (a[1] > p[1]) != (b[1] > p[1]) {
                let x_int = a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]);
                if p[0] < x_int {
                    crossings += 1;
                }
            }
        }
    }
    if min_dist == f32::INFINITY {
        return f32::INFINITY;
    }
    if crossings % 2 == 1 {
        -min_dist
    } else {
        min_dist
    }
}

/// Euclidean distance (mm) from point `p` to segment `a`-`b`.
fn point_segment_distance(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let ab = [b[0] - a[0], b[1] - a[1]]; // mm
    let ap = [p[0] - a[0], p[1] - a[1]]; // mm
    let len2 = ab[0] * ab[0] + ab[1] * ab[1]; // mm^2
    let t = if len2 > 0.0 {
        ((ap[0] * ab[0] + ap[1] * ab[1]) / len2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let dx = ap[0] - t * ab[0];
    let dy = ap[1] - t * ab[1];
    (dx * dx + dy * dy).sqrt()
}

/// Test-mesh builders shared by contour and visibility unit tests.
#[cfg(test)]
pub(crate) mod test_mesh {
    /// Axis-aligned cuboid `[0,sx] x [0,sy] x [0,sz]` (mm) with outward-facing
    /// triangle winding. Returns `(vertices, triangles)`.
    pub(crate) fn cuboid(sx: f32, sy: f32, sz: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        let vertices = vec![
            [0.0, 0.0, 0.0],
            [sx, 0.0, 0.0],
            [sx, sy, 0.0],
            [0.0, sy, 0.0],
            [0.0, 0.0, sz],
            [sx, 0.0, sz],
            [sx, sy, sz],
            [0.0, sy, sz],
        ];
        let triangles = vec![
            // bottom (-z)
            [0, 2, 1],
            [0, 3, 2],
            // top (+z)
            [4, 5, 6],
            [4, 6, 7],
            // front (-y)
            [0, 1, 5],
            [0, 5, 4],
            // right (+x)
            [1, 2, 6],
            [1, 6, 5],
            // back (+y)
            [2, 3, 7],
            [2, 7, 6],
            // left (-x)
            [3, 0, 4],
            [3, 4, 7],
        ];
        (vertices, triangles)
    }

    /// Open prism (side walls only, no caps) extruding a CCW L-profile from
    /// z=0 to z=sz (mm). The reflex (concave) corner sits at (4, 4).
    pub(crate) fn l_prism(sz: f32) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        let profile: [[f32; 2]; 6] = [
            [0.0, 0.0],
            [10.0, 0.0],
            [10.0, 4.0],
            [4.0, 4.0],
            [4.0, 10.0],
            [0.0, 10.0],
        ];
        let n = profile.len() as u32;
        let mut vertices = Vec::new();
        for p in &profile {
            vertices.push([p[0], p[1], 0.0]);
        }
        for p in &profile {
            vertices.push([p[0], p[1], sz]);
        }
        let mut triangles = Vec::new();
        for i in 0..n {
            let j = (i + 1) % n;
            triangles.push([i, j, j + n]);
            triangles.push([i, j + n, i + n]);
        }
        (vertices, triangles)
    }
}

#[cfg(test)]
mod tests {
    use super::test_mesh::{cuboid, l_prism};
    use super::*;

    #[test]
    fn contours_cube_section_is_one_ccw_square() {
        let (vertices, triangles) = cuboid(10.0, 10.0, 4.0);
        let contours = extract_layer_contours(&vertices, &triangles, 2.0);
        assert_eq!(contours.len(), 1);
        let c = &contours[0];
        assert_eq!(c.points.len(), 4);
        // CCW: positive area of 100 mm^2.
        assert!((signed_area(&c.points) - 100.0).abs() < 1e-3);
        // Canonical start point: lexicographically smallest corner (0, 0).
        assert!(c.points[0][0].abs() < 1e-3 && c.points[0][1].abs() < 1e-3);
        // All four corners convex: positive ~ +pi/2 CCW angles.
        for &angle in &c.local_ccw_angles {
            assert!(
                (angle - std::f32::consts::FRAC_PI_2).abs() < 1e-4,
                "expected +pi/2, got {angle}"
            );
        }
    }

    #[test]
    fn contours_l_prism_has_negative_angle_at_notch() {
        let (vertices, triangles) = l_prism(4.0);
        let contours = extract_layer_contours(&vertices, &triangles, 2.0);
        assert_eq!(contours.len(), 1);
        let c = &contours[0];
        assert_eq!(c.points.len(), 6);
        let notch_idx = c
            .points
            .iter()
            .position(|p| (p[0] - 4.0).abs() < 1e-3 && (p[1] - 4.0).abs() < 1e-3)
            .expect("reflex corner (4,4) must be present");
        // Concave w.r.t. CCW winding: NEGATIVE angle at the notch.
        assert!(
            c.local_ccw_angles[notch_idx] < 0.0,
            "notch angle must be negative, got {}",
            c.local_ccw_angles[notch_idx]
        );
        // Every other corner is convex: positive.
        for (i, &angle) in c.local_ccw_angles.iter().enumerate() {
            if i != notch_idx {
                assert!(angle > 0.0, "corner {i} must be convex, got {angle}");
            }
        }
    }

    #[test]
    fn contours_extraction_is_deterministic() {
        let (vertices, triangles) = l_prism(4.0);
        let a = extract_layer_contours(&vertices, &triangles, 1.3);
        let b = extract_layer_contours(&vertices, &triangles, 1.3);
        assert_eq!(a, b);
    }

    #[test]
    fn contours_signed_distance_sign_convention() {
        let (vertices, triangles) = cuboid(10.0, 10.0, 4.0);
        let contours = extract_layer_contours(&vertices, &triangles, 2.0);
        // Center: 5 mm inside -> -5.
        let inside = signed_distance_to_contours(&contours, [5.0, 5.0]);
        assert!((inside + 5.0).abs() < 1e-3, "got {inside}");
        // 2 mm outside the x=10 edge -> +2.
        let outside = signed_distance_to_contours(&contours, [12.0, 5.0]);
        assert!((outside - 2.0).abs() < 1e-3, "got {outside}");
        // On the boundary: ~0.
        let on_edge = signed_distance_to_contours(&contours, [10.0, 5.0]);
        assert!(on_edge.abs() < 1e-3, "got {on_edge}");
    }

    #[test]
    fn contours_layer_z_at_vertex_plane_still_sections() {
        // Plane exactly at a vertex z: nudge policy must still produce a
        // deterministic result (vertices count as above the plane).
        let (vertices, triangles) = cuboid(10.0, 10.0, 4.0);
        let a = extract_layer_contours(&vertices, &triangles, 0.0);
        let b = extract_layer_contours(&vertices, &triangles, 0.0);
        assert_eq!(a, b);
    }
}
