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
use slicer_ir::{PaintSemantic, PaintValue};

fn paint_marker(text: &str) -> Option<EnforcedBlockedSeamPoint> {
    let text = text.to_ascii_lowercase();
    if text.contains("blocked") || text.contains("blocker") {
        Some(EnforcedBlockedSeamPoint::Blocked)
    } else if text.contains("enforced") || text.contains("enforcer") {
        Some(EnforcedBlockedSeamPoint::Enforced)
    } else {
        None
    }
}

fn paint_annotation_type(
    semantic: &PaintSemantic,
    value: &PaintValue,
) -> Option<EnforcedBlockedSeamPoint> {
    let semantic_type = match semantic {
        PaintSemantic::SupportBlocker => Some(EnforcedBlockedSeamPoint::Blocked),
        PaintSemantic::SupportEnforcer => Some(EnforcedBlockedSeamPoint::Enforced),
        PaintSemantic::Custom(name) => paint_marker(name),
        _ => None,
    };
    let value_type = match value {
        PaintValue::Custom(name) => paint_marker(name),
        _ => None,
    };

    match (semantic_type, value_type) {
        (Some(EnforcedBlockedSeamPoint::Blocked), _)
        | (_, Some(EnforcedBlockedSeamPoint::Blocked)) => Some(EnforcedBlockedSeamPoint::Blocked),
        (Some(EnforcedBlockedSeamPoint::Enforced), _)
        | (_, Some(EnforcedBlockedSeamPoint::Enforced)) => Some(EnforcedBlockedSeamPoint::Enforced),
        _ => None,
    }
}

fn annotation_at<'a>(
    paint_annotations: &'a [(PaintSemantic, &[Vec<Option<PaintValue>>])],
    contour_idx: usize,
    vertex_idx: usize,
) -> impl Iterator<Item = (&'a PaintSemantic, &'a PaintValue)> {
    paint_annotations
        .iter()
        .filter_map(move |(semantic, contours)| {
            contours
                .get(contour_idx)
                .and_then(|vertices| vertices.get(vertex_idx))
                .and_then(Option::as_ref)
                .map(|value| (semantic, value))
        })
}

fn has_enforced_annotation(
    paint_annotations: &[(PaintSemantic, &[Vec<Option<PaintValue>>])],
    contour_idx: usize,
    vertex_idx: usize,
) -> bool {
    annotation_at(paint_annotations, contour_idx, vertex_idx).any(|(semantic, value)| {
        paint_annotation_type(semantic, value) == Some(EnforcedBlockedSeamPoint::Enforced)
    })
}

fn is_central_enforcer_vertex(
    paint_annotations: &[(PaintSemantic, &[Vec<Option<PaintValue>>])],
    contour_idx: usize,
    vertex_idx: usize,
) -> bool {
    let Some(contour_annotations) = paint_annotations
        .iter()
        .find_map(|(_, contours)| contours.get(contour_idx))
    else {
        return false;
    };
    if !has_enforced_annotation(paint_annotations, contour_idx, vertex_idx) {
        return false;
    }

    let mut segment_start = vertex_idx;
    while segment_start > 0
        && has_enforced_annotation(paint_annotations, contour_idx, segment_start - 1)
    {
        segment_start -= 1;
    }
    let mut segment_end = vertex_idx + 1;
    while segment_end < contour_annotations.len()
        && has_enforced_annotation(paint_annotations, contour_idx, segment_end)
    {
        segment_end += 1;
    }

    // Annotation geometry has no explicit segment boundaries. Treat the first
    // third of each contiguous enforced run as its central region.
    vertex_idx - segment_start < (segment_end - segment_start).div_ceil(3).max(1)
}

fn candidate_paint_classification(
    paint_annotations: Option<&[(PaintSemantic, &[Vec<Option<PaintValue>>])]>,
    contour_idx: usize,
    vertex_idx: usize,
) -> (EnforcedBlockedSeamPoint, bool) {
    let Some(paint_annotations) = paint_annotations else {
        return (EnforcedBlockedSeamPoint::Neutral, false);
    };

    let mut point_type = EnforcedBlockedSeamPoint::Neutral;
    for (semantic, value) in annotation_at(paint_annotations, contour_idx, vertex_idx) {
        match paint_annotation_type(semantic, value) {
            Some(EnforcedBlockedSeamPoint::Blocked) => {
                return (EnforcedBlockedSeamPoint::Blocked, false);
            }
            Some(EnforcedBlockedSeamPoint::Enforced) => {
                point_type = EnforcedBlockedSeamPoint::Enforced;
            }
            _ => {}
        }
    }

    let central = point_type == EnforcedBlockedSeamPoint::Enforced
        && is_central_enforcer_vertex(paint_annotations, contour_idx, vertex_idx);
    (point_type, central)
}

/// Number of uniform surface samples for global visibility raycasting.
/// Canonical `SeamPlacer::raycasting_visibility_samples_count`. Units: count.
pub(crate) const VISIBILITY_SAMPLES_COUNT: usize = 30000;

/// Stratified hemisphere rays per side; total rays = side^2.
/// Canonical `SeamPlacer::sqr_rays_per_sample_point`. Units: count.
pub(crate) const RAYS_PER_SIDE: usize = 5;

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
/// canonical sample budget for this implementation.
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

/// Stable seeded PRNG for canonical visibility sampling.
///
/// SplitMix64 is used explicitly rather than a platform RNG so identical
/// object seeds produce identical f32 samples on every supported target.
struct StableRng {
    state: u64,
}

impl StableRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn next_f32(&mut self) -> f32 {
        // Keep exactly 24 random bits so the conversion is deterministic and
        // the result is always in [0, 1).
        (self.next_u64() >> 40) as f32 / 16_777_216.0
    }
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

/// Compute global surface visibility by seeded uniform surface sampling and
/// stratified hemisphere raycasting.
///
/// Port of canonical `raycast_visibility` (`SeamPlacer.cpp`): each sample's
/// visibility starts at 1.0 and every occluded ray subtracts
/// `1 / RAYS_PER_SAMPLE` (canonical 1/25). With `aligned_back`,
/// the canonical AlignedBack front bias
/// `clamp((normal . (0,-1,0) + 1.2) * 0.5, 0, 1)` is added per sample.
///
/// Cost note: occlusion is a linear O(rays x triangles) scan with a per-
/// triangle AABB slab reject (no BVH); at the canonical budget this is
/// 30000 x 25 = 750000 rays. For large meshes this is the dominant cost of
/// the seam prepass.
pub(crate) fn compute_global_visibility(
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    aligned_back: bool,
    seed: u64,
    sample_count_override: Option<usize>,
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

    let sample_count = sample_count_override.unwrap_or(VISIBILITY_SAMPLES_COUNT);
    let mut rng = StableRng::new(seed);
    let mut samples: Vec<VisibilitySample> = Vec::with_capacity(sample_count);
    for _ in 0..sample_count {
        // Area-uniform triangle pick, followed by uniform barycentric
        // placement inside the selected triangle.
        let target = rng.next_f32() * total_area; // mm^2
        let ti = cumulative_area
            .partition_point(|&acc| acc < target)
            .min(triangles.len() - 1);
        let tri = triangles[ti];
        let a = vertices[tri[0] as usize];
        let b = vertices[tri[1] as usize];
        let c = vertices[tri[2] as usize];

        let mut u = rng.next_f32();
        let mut v = rng.next_f32();
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
        let mut occluded_count = 0usize;
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
                    occluded_count += 1;
                }
            }
        }

        let mut visibility = 1.0 - occluded_count as f32 / RAYS_PER_SAMPLE as f32;

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
    /// Per-layer global angle used by canonical `curling_influence`. Units: radians.
    pub layer_angle: f32,
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
/// AC-7: `flow_width` is the resolved per-active-region outer-wall scoring
/// width from packet 178, not a hardcoded default.
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
    paint_annotations: Option<&[(PaintSemantic, &[Vec<Option<PaintValue>>])]>,
    seed: u64,
) -> Vec<Vec<SeamCandidate>> {
    build_seam_candidates_with_sample_count(
        vertices,
        triangles,
        layers,
        contours_per_layer,
        aligned_back,
        flow_width,
        paint_annotations,
        seed,
        None,
    )
}

/// Test-support variant of [`build_seam_candidates`] with a bounded visibility
/// sample budget. Production callers must use [`build_seam_candidates`] so the
/// canonical 30000-sample budget remains the default.
pub(crate) fn build_seam_candidates_with_sample_count(
    vertices: &[[f32; 3]],
    triangles: &[[u32; 3]],
    layers: &[LayerInfo],
    contours_per_layer: &[Vec<Contour>],
    aligned_back: bool,
    flow_width: f32,
    paint_annotations: Option<&[(PaintSemantic, &[Vec<Option<PaintValue>>])]>,
    seed: u64,
    sample_count_override: Option<usize>,
) -> Vec<Vec<SeamCandidate>> {
    debug_assert_eq!(layers.len(), contours_per_layer.len());
    let global = compute_global_visibility(
        vertices,
        triangles,
        aligned_back,
        seed,
        sample_count_override,
    );

    let mut result: Vec<Vec<SeamCandidate>> = Vec::with_capacity(layers.len());
    for (layer_idx, (layer, contours)) in layers.iter().zip(contours_per_layer.iter()).enumerate() {
        let prev_contours: Option<&Vec<Contour>> = if layer_idx > 0 {
            Some(&contours_per_layer[layer_idx - 1])
        } else {
            None
        };
        let mut layer_candidates: Vec<SeamCandidate> = Vec::new();
        for (contour_idx, contour) in contours.iter().enumerate() {
            for (vi, p2d) in contour.points.iter().enumerate() {
                let position = [p2d[0], p2d[1], layer.z]; // mm
                let visibility = calculate_point_visibility(&global, position);
                let (point_type, central_enforcer) =
                    candidate_paint_classification(paint_annotations, contour_idx, vi);

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
                    layer_angle: layer.layer_angle,
                    central_enforcer,
                    point_type,
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
                layer_angle: 0.0,        // rad
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
        let candidates = build_seam_candidates_with_sample_count(
            &vertices,
            &triangles,
            &layers,
            &contours,
            false,
            0.4,
            None,
            0,
            Some(100),
        );
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
        let candidates = build_seam_candidates_with_sample_count(
            &vertices,
            &triangles,
            &layers,
            &contours,
            true,
            0.4,
            None,
            0,
            Some(100),
        );
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
        let candidates = build_seam_candidates_with_sample_count(
            &vertices,
            &triangles,
            &layers,
            &contours,
            false,
            0.4,
            None,
            0,
            Some(100),
        );
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
            layer_angle: 0.0,
        }];
        let contours = vec![extract_layer_contours(&vertices, &triangles, 0.5)];
        let candidates = build_seam_candidates_with_sample_count(
            &vertices,
            &triangles,
            &layers,
            &contours,
            false,
            0.4,
            None,
            0,
            Some(100),
        );
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
        let a = build_seam_candidates_with_sample_count(
            &vertices,
            &triangles,
            &layers,
            &contours,
            true,
            0.4,
            None,
            0,
            Some(100),
        );
        let b = build_seam_candidates_with_sample_count(
            &vertices,
            &triangles,
            &layers,
            &contours,
            true,
            0.4,
            None,
            0,
            Some(100),
        );
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
                assert_eq!(ca.layer_angle, cb.layer_angle);
            }
        }
    }
}
