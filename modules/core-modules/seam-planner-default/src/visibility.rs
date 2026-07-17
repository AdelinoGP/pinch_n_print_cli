// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/GCode/SeamPlacer.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Global mesh visibility sampling, per-candidate visibility lookup, and
//! overhang / layer-embedding computation, ported from canonical
//! `raycast_visibility`, `calculate_candidates_visibility` /
//! `calculate_point_visibility`, and `calculate_overhangs_and_layer_embedding`
//! (`SeamPlacer.cpp`).
//!
//! UNIT NOTE: everything in this module is f32 **millimetres** (mesh
//! vertices, seam positions, distances), angles in **radians** — matching
//! canonical `SeamPlacer`'s unscaled-mm domain, NOT the integer 100 nm
//! system used elsewhere (see `docs/08_coordinate_system.md`).

use crate::comparator::{EnforcedBlockedSeamPoint, SeamCandidate};
use crate::contours::{signed_distance_to_contours, Contour};

/// Number of uniform surface samples for global visibility raycasting.
/// Canonical `SeamPlacer::raycasting_visibility_samples_count` is 30000;
/// reduced here to a deterministic WASM budget. Deviation
/// D-168-SEAM-PREPASS-SOURCE material. Units: count.
const VISIBILITY_SAMPLES_COUNT: usize = 2000;

/// Stratified hemisphere rays per side; total rays = side^2.
/// Canonical `SeamPlacer::sqr_rays_per_sample_point` is 5 (25 rays);
/// reduced to 3 (9 rays) for the WASM budget. Deviation
/// D-168-SEAM-PREPASS-SOURCE material. Units: count.
const RAYS_PER_SIDE: usize = 3;

/// Total hemisphere rays per sample (`RAYS_PER_SIDE`^2; canonical 25).
const RAYS_PER_SAMPLE: usize = RAYS_PER_SIDE * RAYS_PER_SIDE;

/// Ray-origin offset along the surface normal to avoid self-intersection.
/// Canonical `raycast_visibility` uses `normal * 0.01`. Units: mm.
const RAY_ORIGIN_OFFSET_MM: f32 = 0.01;

/// Minimum ray parameter for a hit to count as occlusion. Units: mm.
const RAY_HIT_EPSILON_MM: f32 = 1e-4;

/// Expected samples per visibility query neighborhood.
/// Canonical `calculate_point_visibility` `samples_per_query`. Dimensionless.
const SAMPLES_PER_QUERY: f32 = 4.0;

/// Fraction of flow width added to the previous-layer distance for the
/// overhang test. Canonical `calculate_overhangs_and_layer_embedding`.
/// Dimensionless fraction of `flow_width` (mm).
const OVERHANG_FLOW_FRACTION: f32 = 0.65;

/// Fraction of flow width added for the unsupported-distance value.
/// Canonical `calculate_overhangs_and_layer_embedding`. Dimensionless.
const UNSUPPORTED_FLOW_FRACTION: f32 = 0.4;

/// Fraction of flow width added to the current-layer distance for
/// embedded distance. Canonical `calculate_overhangs_and_layer_embedding`.
/// Dimensionless.
const EMBEDDED_FLOW_FRACTION: f32 = 0.65;

/// Tangent of the overhang angle threshold (45 degrees).
/// Canonical `calculate_overhangs_and_layer_embedding`. Dimensionless.
const OVERHANG_ANGLE_TAN: f32 = 1.0;

/// One precomputed surface visibility sample.
/// Canonical stores these in a KD-tree; a linear scan suffices at the
/// reduced sample budget.
#[derive(Debug, Clone)]
pub(crate) struct VisibilitySample {
    /// Sample position on the mesh surface. Units: mm.
    pub position: [f32; 3],
    /// Surface normal (unit vector) at the sample. Dimensionless.
    pub normal: [f32; 3],
    /// Visibility score: 1.0 fully visible, 0.0 fully occluded; AlignedBack
    /// front bias may push it up to 2.0. Dimensionless penalty units.
    pub visibility: f32,
}

/// Precomputed global visibility data for one mesh.
#[derive(Debug, Clone)]
pub(crate) struct GlobalVisibility {
    /// Surface samples with visibility scores.
    pub samples: Vec<VisibilitySample>,
    /// Total mesh surface area. Units: mm^2.
    pub total_area: f32,
}

// --- small vector helpers (mm-domain [f32; 3]) ------------------------------

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

fn normalize(a: [f32; 3]) -> [f32; 3] {
    let n = norm(a);
    if n > 1e-12 {
        [a[0] / n, a[1] / n, a[2] / n]
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// Radical-inverse low-discrepancy sequence (deterministic; replaces the
/// canonical RNG for sample placement — no RNG anywhere in this module).
fn radical_inverse(mut i: u32, base: u32) -> f32 {
    let mut f = 1.0f32;
    let mut r = 0.0f32;
    while i > 0 {
        f /= base as f32;
        r += f * (i % base) as f32;
        i /= base;
    }
    r
}

/// Moeller-Trumbore ray/triangle intersection; returns the hit parameter t
/// (mm along the unit ray direction) if the ray hits the triangle.
fn ray_triangle(
    origin: [f32; 3],
    dir: [f32; 3],
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) -> Option<f32> {
    let e1 = sub(b, a);
    let e2 = sub(c, a);
    let p = cross(dir, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-9 {
        return None;
    }
    let inv_det = 1.0 / det;
    let tvec = sub(origin, a);
    let u = dot(tvec, p) * inv_det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(tvec, e1);
    let v = dot(dir, q) * inv_det;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let t = dot(e2, q) * inv_det;
    if t > RAY_HIT_EPSILON_MM {
        Some(t)
    } else {
        None
    }
}

/// Slab-test reject of a ray against an AABB (`lo`/`hi` in mm).
fn ray_hits_aabb(origin: [f32; 3], dir: [f32; 3], lo: [f32; 3], hi: [f32; 3]) -> bool {
    let mut tmin = 0.0f32;
    let mut tmax = f32::INFINITY;
    for axis in 0..3 {
        if dir[axis].abs() < 1e-12 {
            if origin[axis] < lo[axis] || origin[axis] > hi[axis] {
                return false;
            }
        } else {
            let inv = 1.0 / dir[axis];
            let mut t0 = (lo[axis] - origin[axis]) * inv;
            let mut t1 = (hi[axis] - origin[axis]) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            tmin = tmin.max(t0);
            tmax = tmax.min(t1);
            if tmin > tmax {
                return false;
            }
        }
    }
    true
}

/// Compute global surface visibility by deterministic low-discrepancy
/// surface sampling and stratified hemisphere raycasting.
///
/// Port of canonical `raycast_visibility` (`SeamPlacer.cpp`): each sample's
/// visibility starts at 1.0 and every occluded ray subtracts
/// `1 / RAYS_PER_SAMPLE` (canonical 1/25, here 1/9). With `aligned_back`,
/// the canonical AlignedBack front bias
/// `clamp((normal . (0,-1,0) + 1.2) * 0.5, 0, 1)` is added per sample.
///
/// Cost note: occlusion is a linear O(rays x triangles) scan with a per-
/// triangle AABB slab reject (no BVH); at the reduced budget this is
/// 2000 x 9 = 18000 rays. For large meshes this is the dominant cost of the
/// seam prepass.
pub(crate) fn compute_global_visibility(
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    aligned_back: bool,
) -> GlobalVisibility {
    // Per-triangle area, normal and AABB.
    let mut cumulative_area: Vec<f32> = Vec::with_capacity(triangles.len()); // mm^2
    let mut tri_normals: Vec<[f32; 3]> = Vec::with_capacity(triangles.len());
    let mut tri_aabbs: Vec<([f32; 3], [f32; 3])> = Vec::with_capacity(triangles.len());
    let mut total_area = 0.0f32; // mm^2
    for tri in triangles {
        let a = vertices[tri[0] as usize];
        let b = vertices[tri[1] as usize];
        let c = vertices[tri[2] as usize];
        let n = cross(sub(b, a), sub(c, a));
        let area = 0.5 * norm(n); // mm^2
        total_area += area;
        cumulative_area.push(total_area);
        tri_normals.push(normalize(n));
        let lo = [
            a[0].min(b[0]).min(c[0]),
            a[1].min(b[1]).min(c[1]),
            a[2].min(b[2]).min(c[2]),
        ];
        let hi = [
            a[0].max(b[0]).max(c[0]),
            a[1].max(b[1]).max(c[1]),
            a[2].max(b[2]).max(c[2]),
        ];
        tri_aabbs.push((lo, hi));
    }
    if total_area <= 0.0 || triangles.is_empty() {
        return GlobalVisibility {
            samples: Vec::new(),
            total_area: 0.0,
        };
    }

    let occluded = |origin: [f32; 3], dir: [f32; 3]| -> bool {
        for (ti, tri) in triangles.iter().enumerate() {
            let (lo, hi) = tri_aabbs[ti];
            if !ray_hits_aabb(origin, dir, lo, hi) {
                continue;
            }
            let a = vertices[tri[0] as usize];
            let b = vertices[tri[1] as usize];
            let c = vertices[tri[2] as usize];
            if ray_triangle(origin, dir, a, b, c).is_some() {
                return true;
            }
        }
        false
    };

    let mut samples: Vec<VisibilitySample> = Vec::with_capacity(VISIBILITY_SAMPLES_COUNT);
    for i in 0..VISIBILITY_SAMPLES_COUNT {
        // Deterministic area-uniform triangle pick (stratified by index).
        let target = (i as f32 + 0.5) / VISIBILITY_SAMPLES_COUNT as f32 * total_area; // mm^2
        let ti = cumulative_area
            .partition_point(|&acc| acc < target)
            .min(triangles.len() - 1);
        let tri = triangles[ti];
        let a = vertices[tri[0] as usize];
        let b = vertices[tri[1] as usize];
        let c = vertices[tri[2] as usize];

        // Low-discrepancy barycentric placement (Halton bases 2 and 3).
        let mut u = radical_inverse(i as u32 + 1, 2);
        let mut v = radical_inverse(i as u32 + 1, 3);
        if u + v > 1.0 {
            u = 1.0 - u;
            v = 1.0 - v;
        }
        let position = [
            a[0] + u * (b[0] - a[0]) + v * (c[0] - a[0]),
            a[1] + u * (b[1] - a[1]) + v * (c[1] - a[1]),
            a[2] + u * (b[2] - a[2]) + v * (c[2] - a[2]),
        ]; // mm
        let normal = tri_normals[ti];

        // Orthonormal basis around the normal.
        let helper = if normal[0].abs() < 0.9 {
            [1.0, 0.0, 0.0]
        } else {
            [0.0, 1.0, 0.0]
        };
        let tangent = normalize(cross(normal, helper));
        let bitangent = cross(normal, tangent);

        let origin = [
            position[0] + normal[0] * RAY_ORIGIN_OFFSET_MM,
            position[1] + normal[1] * RAY_ORIGIN_OFFSET_MM,
            position[2] + normal[2] * RAY_ORIGIN_OFFSET_MM,
        ]; // mm

        // Stratified uniform-hemisphere rays oriented to the normal.
        let mut visibility = 1.0f32;
        for ra in 0..RAYS_PER_SIDE {
            for rb in 0..RAYS_PER_SIDE {
                let su = (ra as f32 + 0.5) / RAYS_PER_SIDE as f32;
                let sv = (rb as f32 + 0.5) / RAYS_PER_SIDE as f32;
                // Uniform hemisphere: z = su, r = sqrt(1 - z^2), phi = 2*pi*sv.
                let z = su;
                let r = (1.0 - z * z).max(0.0).sqrt();
                let phi = 2.0 * std::f32::consts::PI * sv;
                let (lx, ly, lz) = (r * phi.cos(), r * phi.sin(), z);
                let dir = normalize([
                    tangent[0] * lx + bitangent[0] * ly + normal[0] * lz,
                    tangent[1] * lx + bitangent[1] * ly + normal[1] * lz,
                    tangent[2] * lx + bitangent[2] * ly + normal[2] * lz,
                ]);
                if occluded(origin, dir) {
                    visibility -= 1.0 / RAYS_PER_SAMPLE as f32;
                }
            }
        }

        if aligned_back {
            // Canonical AlignedBack front bias: normal . (0,-1,0).
            visibility += ((-normal[1] + 1.2) * 0.5).clamp(0.0, 1.0);
        }

        samples.push(VisibilitySample {
            position,
            normal,
            visibility,
        });
    }

    GlobalVisibility {
        samples,
        total_area,
    }
}

/// Weighted-average visibility at `point` from precomputed samples.
/// Port of canonical `calculate_point_visibility` (`SeamPlacer.cpp`):
/// neighborhood radius is `sqrt(search_area / PI)` with
/// `search_area = samples_per_query / (-ln(0.9) * density)` and
/// `density = sample_count / total_mesh_area`; weight per sample is
/// `(radius - dist_to_sample_plane) + (radius - euclid_dist)`. An empty
/// neighborhood yields 1.0 (fully visible). Linear scan (no KD-tree) at the
/// reduced budget.
pub(crate) fn calculate_point_visibility(global: &GlobalVisibility, point: [f32; 3]) -> f32 {
    if global.samples.is_empty() || global.total_area <= 0.0 {
        return 1.0;
    }
    let density = global.samples.len() as f32 / global.total_area; // samples per mm^2
    let search_area = SAMPLES_PER_QUERY / (-(0.9f32.ln()) * density); // mm^2
    let radius = (search_area / std::f32::consts::PI).sqrt(); // mm

    let mut weight_sum = 0.0f32;
    let mut value_sum = 0.0f32;
    for sample in &global.samples {
        let delta = sub(point, sample.position); // mm
        let euclid_dist = norm(delta); // mm
        if euclid_dist >= radius {
            continue;
        }
        let dist_to_plane = dot(delta, sample.normal).abs(); // mm
        let weight = (radius - dist_to_plane) + (radius - euclid_dist); // mm
        if weight <= 0.0 {
            continue;
        }
        weight_sum += weight;
        value_sum += weight * sample.visibility;
    }
    if weight_sum > 0.0 {
        value_sum / weight_sum
    } else {
        1.0
    }
}

/// Per-layer slicing info for candidate construction. Units: mm.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LayerInfo {
    /// Slice plane z of this layer. Units: mm.
    pub z: f32,
    /// Layer height. Units: mm.
    pub height: f32,
}

/// Build seam candidates for every contour vertex of every layer.
///
/// Combines the ported visibility lookup with the ported overhang / layer
/// embedding from canonical `calculate_overhangs_and_layer_embedding`
/// (`SeamPlacer.cpp`), all in mm:
/// - `prev_dist` = signed distance (negative inside) to the PREVIOUS layer's
///   contours; `curr_dist` = signed distance to the CURRENT layer's contours.
/// - `overhang = max(0, prev_dist + 0.65*flow_width - tan(45 deg)*layer_height)`
/// - `unsupported_dist = prev_dist + 0.4*flow_width`
/// - `embedded_distance = curr_dist + 0.65*flow_width`
/// - Layer 0 is fully supported by the bed: overhang = 0, unsupported = 0.
///
/// Returns one `Vec<SeamCandidate>` per layer, candidates ordered contour by
/// contour, vertex by vertex (deterministic given deterministic contours).
pub(crate) fn build_seam_candidates(
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    layers: &[LayerInfo],
    contours_per_layer: &[Vec<Contour>],
    aligned_back: bool,
    flow_width: f32,
) -> Vec<Vec<SeamCandidate>> {
    debug_assert_eq!(layers.len(), contours_per_layer.len());
    let global = compute_global_visibility(vertices, triangles, aligned_back);

    let mut result: Vec<Vec<SeamCandidate>> = Vec::with_capacity(layers.len());
    for (layer_idx, (layer, contours)) in layers.iter().zip(contours_per_layer.iter()).enumerate() {
        let prev_contours: Option<&Vec<Contour>> = if layer_idx > 0 {
            Some(&contours_per_layer[layer_idx - 1])
        } else {
            None
        };
        let mut layer_candidates: Vec<SeamCandidate> = Vec::new();
        for contour in contours {
            for (vi, p2d) in contour.points.iter().enumerate() {
                let position = [p2d[0], p2d[1], layer.z]; // mm
                let visibility = calculate_point_visibility(&global, position);

                let (overhang, unsupported_dist) = match prev_contours {
                    Some(prev) => {
                        let prev_dist = signed_distance_to_contours(prev, *p2d); // mm
                        (
                            (prev_dist + OVERHANG_FLOW_FRACTION * flow_width
                                - OVERHANG_ANGLE_TAN * layer.height)
                                .max(0.0),
                            prev_dist + UNSUPPORTED_FLOW_FRACTION * flow_width,
                        )
                    }
                    // Layer 0: fully supported by the bed.
                    None => (0.0, 0.0),
                };
                let curr_dist = signed_distance_to_contours(contours, *p2d); // mm
                let embedded_distance = curr_dist + EMBEDDED_FLOW_FRACTION * flow_width; // mm

                layer_candidates.push(SeamCandidate {
                    position,
                    visibility,
                    overhang,
                    unsupported_dist,
                    embedded_distance,
                    local_ccw_angle: contour.local_ccw_angles[vi], // rad
                    central_enforcer: false,
                    point_type: EnforcedBlockedSeamPoint::Neutral,
                    flow_width,
                });
            }
        }
        result.push(layer_candidates);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contours::{extract_layer_contours, test_mesh};

    /// 20 layers of 0.2 mm over a 10 x 10 x 4 mm prism, slice planes at
    /// layer mid-heights (avoids vertex-coincident planes).
    fn prism_setup() -> (
        Vec<[f32; 3]>,
        Vec<[u32; 3]>,
        Vec<LayerInfo>,
        Vec<Vec<Contour>>,
    ) {
        let (vertices, triangles) = test_mesh::cuboid(10.0, 10.0, 4.0);
        let layers: Vec<LayerInfo> = (0..20)
            .map(|i| LayerInfo {
                z: 0.1 + i as f32 * 0.2, // mm
                height: 0.2,             // mm
            })
            .collect();
        let contours: Vec<Vec<Contour>> = layers
            .iter()
            .map(|l| extract_layer_contours(&vertices, &triangles, l.z))
            .collect();
        (vertices, triangles, layers, contours)
    }

    #[test]
    fn visibility_prism_candidates_are_sane() {
        let (vertices, triangles, layers, contours) = prism_setup();
        // One 4-corner contour per layer.
        for layer_contours in &contours {
            assert_eq!(layer_contours.len(), 1);
            assert_eq!(layer_contours[0].points.len(), 4);
        }
        let candidates =
            build_seam_candidates(&vertices, &triangles, &layers, &contours, false, 0.4);
        assert_eq!(candidates.len(), 20);
        for layer in &candidates {
            assert_eq!(layer.len(), 4);
            for c in layer {
                // Finite visibility in [0, 1] (no AlignedBack bias here).
                assert!(c.visibility.is_finite());
                assert!(
                    (0.0..=1.0 + 1e-5).contains(&c.visibility),
                    "visibility out of range: {}",
                    c.visibility
                );
                // Convex prism corners: positive local CCW angle.
                assert!(
                    c.local_ccw_angle > 0.0,
                    "convex corner must be positive, got {}",
                    c.local_ccw_angle
                );
                assert!(c.overhang >= 0.0);
            }
        }
    }

    #[test]
    fn visibility_aligned_back_bias_stays_in_extended_range() {
        let (vertices, triangles, layers, contours) = prism_setup();
        let candidates =
            build_seam_candidates(&vertices, &triangles, &layers, &contours, true, 0.4);
        for layer in &candidates {
            for c in layer {
                // Bias adds at most 1.0 per canonical clamp.
                assert!(
                    (0.0..=2.0 + 1e-5).contains(&c.visibility),
                    "biased visibility out of range: {}",
                    c.visibility
                );
            }
        }
    }

    #[test]
    fn visibility_layer_zero_is_fully_supported() {
        let (vertices, triangles, layers, contours) = prism_setup();
        let candidates =
            build_seam_candidates(&vertices, &triangles, &layers, &contours, false, 0.4);
        for c in &candidates[0] {
            assert_eq!(c.overhang, 0.0);
            assert_eq!(c.unsupported_dist, 0.0);
        }
        // Vertically-walled prism: upper layers sit exactly on the previous
        // contour (prev_dist ~ 0), so unsupported_dist ~ 0.4 * flow_width.
        for c in &candidates[1] {
            assert!((c.unsupported_dist - 0.4 * 0.4).abs() < 1e-3);
        }
    }

    #[test]
    fn visibility_l_notch_candidate_angle_is_negative() {
        let (vertices, triangles) = test_mesh::l_prism(1.0);
        let layers = [LayerInfo {
            z: 0.5,
            height: 0.2,
        }];
        let contours = vec![extract_layer_contours(&vertices, &triangles, 0.5)];
        let candidates =
            build_seam_candidates(&vertices, &triangles, &layers, &contours, false, 0.4);
        let notch = candidates[0]
            .iter()
            .find(|c| (c.position[0] - 4.0).abs() < 1e-3 && (c.position[1] - 4.0).abs() < 1e-3)
            .expect("notch candidate must exist");
        assert!(
            notch.local_ccw_angle < 0.0,
            "concave notch must be negative, got {}",
            notch.local_ccw_angle
        );
    }

    #[test]
    fn visibility_pipeline_is_deterministic() {
        let (vertices, triangles, layers, contours) = prism_setup();
        let a = build_seam_candidates(&vertices, &triangles, &layers, &contours, true, 0.4);
        let b = build_seam_candidates(&vertices, &triangles, &layers, &contours, true, 0.4);
        // SeamCandidate has no PartialEq (comparator.rs is out of bounds for
        // this step); Debug formatting captures every field bit-for-bit at
        // f32 print precision plus exact equality below.
        assert_eq!(a.len(), b.len());
        for (la, lb) in a.iter().zip(b.iter()) {
            assert_eq!(la.len(), lb.len());
            for (ca, cb) in la.iter().zip(lb.iter()) {
                assert_eq!(ca.position, cb.position);
                assert_eq!(ca.visibility, cb.visibility);
                assert_eq!(ca.overhang, cb.overhang);
                assert_eq!(ca.unsupported_dist, cb.unsupported_dist);
                assert_eq!(ca.embedded_distance, cb.embedded_distance);
                assert_eq!(ca.local_ccw_angle, cb.local_ccw_angle);
            }
        }
    }
}
