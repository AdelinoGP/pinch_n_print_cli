// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Slicing.cpp
// Additional portion (bridge-orientation detection, `detect_bridging_direction_deg`
// and `floating_edges_of_gated_area`): src/libslic3r/BridgeDetector.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Pre-pass slicing algorithms.
//!
//! Produces `SliceIR` for a single layer by slicing object meshes at a given Z.

use std::collections::HashMap;
use std::fmt;

use crate::polygon_ops::{
    clip_polylines, difference, intersection, offset, opening_ex, OffsetJoinType,
};
use crate::slice_mesh_ex;
use crate::triangle_mesh_slicer::apply_slice_closing_radius;
use slicer_ir::{
    ConfigValue, ExPolygon, GlobalLayer, IndexedTriangleSet, MeshIR, ObjectId, ObjectMesh,
    ObjectSurfaceData, Point2, Point3, Polygon, RegionKey, RegionMapIR, ResolvedConfig, SliceIR,
    SlicedRegion, SurfaceClassificationIR, Transform3d, CURRENT_SLICE_IR_SCHEMA_VERSION,
};

use slicer_ir::BlackboardError;

/// Structured failures for the host built-in `PrePass::Slice` stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerSliceError {
    /// A layer referenced an `ObjectId` that is not present in `MeshIR`.
    UnknownObject {
        /// Layer that referenced the unknown object.
        layer_index: u32,
        /// The missing object identifier.
        object_id: ObjectId,
    },
    /// `commit_slice_builtin` could not commit the produced Vec — typically
    /// because `PrePass::Slice` was committed twice for the same print.
    Blackboard(BlackboardError),
    /// `PrePass::Slice` was invoked before `PrePass::LayerPlanning` committed
    /// `LayerPlanIR`.
    MissingLayerPlan,
}

impl fmt::Display for LayerSliceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownObject {
                layer_index,
                object_id,
            } => write!(
                f,
                "PrePass::Slice at layer {layer_index} references unknown object '{object_id}'"
            ),
            Self::Blackboard(inner) => write!(f, "PrePass::Slice blackboard error: {inner}"),
            Self::MissingLayerPlan => write!(
                f,
                "PrePass::Slice ran before PrePass::LayerPlanning committed LayerPlanIR"
            ),
        }
    }
}

impl From<BlackboardError> for LayerSliceError {
    fn from(value: BlackboardError) -> Self {
        Self::Blackboard(value)
    }
}

impl std::error::Error for LayerSliceError {}

// ============================================================================
// Internal geometry helpers
// ============================================================================

/// Apply a 4x4 column-major transform to a 3-D point.
fn transform_point(t: &Transform3d, p: &Point3) -> Point3 {
    crate::transform_point3(&t.matrix, *p)
}

/// A copy of `object`'s mesh with its transform baked into every vertex, so it
/// is sliced in world/global space.
///
/// `PrePass::Slice` must slice world-space geometry: the object's bridge
/// classification ([`classify_region_surfaces`]) and its `BottomSurface`
/// footprint ([`crate::algos::mesh_analysis::bottom_surface_footprint`]) both
/// already apply the transform, and `PrePass::OverhangAnnotation` derives its
/// overhang bands from these slices — slicing local-space here would desync all
/// three for any non-identity transform (e.g. slicing a translated object at a
/// global-space layer Z would miss the mesh entirely). For an identity
/// transform this is a vertex-wise copy, so slice output is bit-identical.
fn object_world_mesh(object: &ObjectMesh) -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: object
            .mesh
            .vertices
            .iter()
            .map(|p| crate::transform_point3(&object.transform.matrix, *p))
            .collect(),
        indices: object.mesh.indices.clone(),
    }
}

/// Ray-casting point-in-polygon test (integer coordinate space).
fn point_in_ring(ring: &Polygon, pt: Point2) -> bool {
    let pts = &ring.points;
    if pts.len() < 3 {
        return false;
    }
    let px = pt.x;
    let py = pt.y;
    let mut inside = false;
    let n = pts.len();
    let mut j = n - 1;
    for i in 0..n {
        let xi = pts[i].x;
        let yi = pts[i].y;
        let xj = pts[j].x;
        let yj = pts[j].y;
        if (yi == py && xi == px) || (yj == py && xj == px) {
            return true;
        }
        if ((yi > py) != (yj > py))
            && (px as i128)
                < ((xj - xi) as i128 * (py - yi) as i128 / (yj - yi) as i128 + xi as i128)
        {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Return `true` if any of `polygons` contains `pt` in its contour ring.
fn any_polygon_contains(polygons: &[Polygon], pt: Point2) -> bool {
    polygons.iter().any(|p| point_in_ring(p, pt))
}

// ============================================================================
// Public helper: classify_region_surfaces
// ============================================================================

/// Detect whether the given region carries any bridge facets at this layer.
pub fn classify_region_surfaces(
    object_mesh: &ObjectMesh,
    surface_data: &ObjectSurfaceData,
    region_polygons: &[Polygon],
    layer_z: f32,
) -> bool {
    let mesh = &object_mesh.mesh;
    let t = &object_mesh.transform;
    let tri_count = mesh.indices.len() / 3;

    let bridge_set: std::collections::HashSet<u32> = surface_data
        .bridge_regions
        .iter()
        .filter(|br| br.is_valid)
        .flat_map(|br| br.facet_indices.iter().copied())
        .collect();

    for tri_idx in 0..tri_count {
        if !bridge_set.contains(&(tri_idx as u32)) {
            continue;
        }

        let i0 = mesh.indices[tri_idx * 3] as usize;
        let i1 = mesh.indices[tri_idx * 3 + 1] as usize;
        let i2 = mesh.indices[tri_idx * 3 + 2] as usize;
        if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
            continue;
        }

        let wv0 = transform_point(t, &mesh.vertices[i0]);
        let wv1 = transform_point(t, &mesh.vertices[i1]);
        let wv2 = transform_point(t, &mesh.vertices[i2]);

        let fz_min = wv0.z.min(wv1.z).min(wv2.z);
        let fz_max = wv0.z.max(wv1.z).max(wv2.z);
        if fz_min <= layer_z && layer_z <= fz_max {
            let p0 = Point2::from_mm(wv0.x, wv0.y);
            let p1 = Point2::from_mm(wv1.x, wv1.y);
            let p2 = Point2::from_mm(wv2.x, wv2.y);
            if any_polygon_contains(region_polygons, p0)
                || any_polygon_contains(region_polygons, p1)
                || any_polygon_contains(region_polygons, p2)
            {
                return true;
            }
        }
    }

    false
}

// ============================================================================
// assemble_bridge_areas
// ============================================================================

/// Assemble expanded bridge polygons for a slice region.
pub fn assemble_bridge_areas(
    region: &mut SlicedRegion,
    surface_class: Option<&SurfaceClassificationIR>,
) {
    let Some(sc) = surface_class else {
        return;
    };
    let Some(obj_data) = sc.per_object.get(&region.object_id) else {
        return;
    };

    let mut best_orientation_deg = 0.0_f32;
    let mut best_bridge_length = 0.0_f32;

    for br in &obj_data.bridge_regions {
        if !br.is_valid {
            continue;
        }
        if br.facet_indices.is_empty() {
            continue;
        }
        if br.xy_footprint.is_empty() {
            continue;
        }
        if !br.expansion_margin_mm.is_finite() || br.expansion_margin_mm < 0.0 {
            continue;
        }

        let footprint_as_expoly: Vec<ExPolygon> = br.xy_footprint.to_vec();
        let intersection_result = intersection(&footprint_as_expoly, &region.infill_areas);
        if intersection_result.is_empty() {
            continue;
        }

        let expanded: Vec<ExPolygon> = offset(
            &intersection_result,
            br.expansion_margin_mm,
            OffsetJoinType::Miter,
            0.0,
        );

        let final_polys = intersection(&expanded, &region.infill_areas);

        if final_polys.is_empty() {
            continue;
        }

        region.bridge_areas.extend(final_polys);

        if br.bridge_length_mm > best_bridge_length {
            best_bridge_length = br.bridge_length_mm;
            best_orientation_deg = br.bridge_direction_deg;
        }
    }

    region.bridge_orientation_deg = best_orientation_deg;
}

/// Keep mesh-derived bridge candidates only where the lower layer does not
/// provide an anchor.
///
/// `None` means the region has no lower layer at all (first layer): canonical
/// demotes first-layer `stBottomBridge` candidates to `stBottom`, so all
/// candidates are removed. `Some(slices)` means the lower layer exists; the
/// candidates are reduced by the ungrown lower-layer contours (canonical
/// `voids = diff(voids, *lower_layer_covered)`). An existing-but-empty lower
/// layer subtracts nothing, so the whole span stays unsupported and every
/// candidate is retained.
pub fn gate_bridge_areas_by_unsupported_span(
    region: &mut SlicedRegion,
    lower_layer_slices: Option<&[ExPolygon]>,
) {
    let Some(lower_layer_slices) = lower_layer_slices else {
        region.bridge_areas.clear();
        region.is_bridge = false;
        return;
    };

    if !lower_layer_slices.is_empty() {
        region.bridge_areas = difference(&region.bridge_areas, lower_layer_slices);
    }
    if region.bridge_areas.is_empty() {
        region.is_bridge = false;
    }
}

// ============================================================================
// detect_bridging_direction_deg
// ============================================================================

/// Growth applied to the raw anchors before floating-edge extraction.
///
/// Canonical OrcaSlicer expands anchors by `SCALED_EPSILON`, where canonical
/// `EPSILON` = 1e-4 mm == exactly 1 internal unit (1 unit = 100 nm). The
/// polyline-difference below is expressed as an intersection against the
/// complement of the grown anchors via [`clip_polylines`], which pre-inflates
/// its clip universe by 1 unit — shrinking the anchor holes back by 1 unit.
/// Growing by one extra unit makes the NET anchor growth exactly
/// `SCALED_EPSILON` (1 unit), so boundary-coincident anchor edges are
/// absorbed exactly as canonical.
const BRIDGE_ANCHOR_GROWTH_MM: f32 = 2.0e-4;

/// Quantization key multiplier for candidate-normal dedup, matching
/// canonical's `ceil(atan2(n.y, n.x) * 1000.0)` (radians x 1000, ceiled).
const ORIENTATION_QUANTIZATION_SCALE: f64 = 1000.0;

/// Canonical covariance threshold for the axis-aligned principal-component
/// shortcut (canonical `EPSILON` = 1e-4, mirrored as-is).
const COVARIANCE_AXIS_ALIGNED_EPSILON: f64 = 1e-4;

/// Extract the floating (non-anchored) boundary segments of a gated bridge
/// area, as canonical's polyline-difference of the overhang boundary against
/// `expand(anchors, SCALED_EPSILON)`.
///
/// `gated_area` is the already-differenced overhang area
/// (`difference(to_cover, anchors)`); `anchors` are the RAW, unexpanded
/// anchor polygons. Boundary segments lying on an anchor boundary are inside
/// the 1-unit growth and are absorbed; everything else is returned as
/// directed `(from, to)` segments following contour winding.
///
/// `pub` as part of this crate's algo surface (like its neighbors) so tests
/// can observe candidate counts directly.
pub fn floating_edges_of_gated_area(
    gated_area: &[ExPolygon],
    anchors: &[ExPolygon],
) -> Vec<(Point2, Point2)> {
    // Boundary rings (outer contours + holes) as closed polylines.
    let mut rings: Vec<Vec<Point2>> = Vec::new();
    for ex in gated_area {
        for poly in std::iter::once(&ex.contour).chain(ex.holes.iter()) {
            if poly.points.len() >= 2 {
                let mut ring = poly.points.clone();
                ring.push(poly.points[0]);
                rings.push(ring);
            }
        }
    }
    if rings.is_empty() {
        return Vec::new();
    }

    let grown_anchors = offset(anchors, BRIDGE_ANCHOR_GROWTH_MM, OffsetJoinType::Miter, 0.0);
    let kept: Vec<Vec<Point2>> = if grown_anchors.is_empty() {
        rings
    } else {
        // Polyline-difference against the grown anchors, expressed as an
        // intersection with their complement inside a generous bounding box
        // (`clip_polylines` performs open-path intersection only).
        let mut min = Point2 {
            x: i64::MAX,
            y: i64::MAX,
        };
        let mut max = Point2 {
            x: i64::MIN,
            y: i64::MIN,
        };
        for ring in &rings {
            for p in ring {
                min.x = min.x.min(p.x);
                min.y = min.y.min(p.y);
                max.x = max.x.max(p.x);
                max.y = max.y.max(p.y);
            }
        }
        const PAD_UNITS: i64 = 10_000; // 1 mm
        let bbox = ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 {
                        x: min.x - PAD_UNITS,
                        y: min.y - PAD_UNITS,
                    },
                    Point2 {
                        x: max.x + PAD_UNITS,
                        y: min.y - PAD_UNITS,
                    },
                    Point2 {
                        x: max.x + PAD_UNITS,
                        y: max.y + PAD_UNITS,
                    },
                    Point2 {
                        x: min.x - PAD_UNITS,
                        y: max.y + PAD_UNITS,
                    },
                ],
            },
            holes: vec![],
        };
        let complement = difference(&[bbox], &grown_anchors);
        if complement.is_empty() {
            return Vec::new();
        }
        clip_polylines(&rings, &complement)
    };

    let mut segments = Vec::new();
    for polyline in &kept {
        for w in polyline.windows(2) {
            if w[0] != w[1] {
                segments.push((w[0], w[1]));
            }
        }
    }
    segments
}

/// Convert a direction vector to degrees in the half-open range [0, 180).
fn direction_deg(dx: f64, dy: f64) -> f32 {
    dy.atan2(dx).to_degrees().rem_euclid(180.0) as f32
}

/// Canonical empty-candidate fallback: principal components of the overhang
/// area via triangle-fan signed-area moment accumulation (each contour fanned
/// from its first point, every fan triangle weighted by the sign of
/// `cross(p1 - p0, p2 - p1)`). Returns the MINOR axis as degrees in [0, 180).
/// Fully-degenerate input (signed area <= 0) falls back to canonical `{1, 0}`
/// -> 0.0 degrees.
fn principal_minor_axis_deg(overhang: &[ExPolygon]) -> f32 {
    let mut area = 0.0_f64;
    let (mut mx, mut my) = (0.0_f64, 0.0_f64);
    let (mut ixx, mut iyy, mut ixy) = (0.0_f64, 0.0_f64, 0.0_f64);

    for ex in overhang {
        for poly in std::iter::once(&ex.contour).chain(ex.holes.iter()) {
            let pts = &poly.points;
            if pts.len() < 3 {
                continue;
            }
            let p0 = (pts[0].x as f64, pts[0].y as f64);
            for w in pts.windows(2).skip(1) {
                let p1 = (w[0].x as f64, w[0].y as f64);
                let p2 = (w[1].x as f64, w[1].y as f64);
                let signed_double_area =
                    (p1.0 - p0.0) * (p2.1 - p1.1) - (p1.1 - p0.1) * (p2.0 - p1.0);
                let a = 0.5 * signed_double_area;
                if a == 0.0 {
                    continue;
                }
                let sx = p0.0 + p1.0 + p2.0;
                let sy = p0.1 + p1.1 + p2.1;
                area += a;
                mx += a * sx / 3.0;
                my += a * sy / 3.0;
                ixx += a / 12.0 * (sx * sx + p0.0 * p0.0 + p1.0 * p1.0 + p2.0 * p2.0);
                iyy += a / 12.0 * (sy * sy + p0.1 * p0.1 + p1.1 * p1.1 + p2.1 * p2.1);
                ixy += a / 12.0 * (sx * sy + p0.0 * p0.1 + p1.0 * p1.1 + p2.0 * p2.1);
            }
        }
    }

    if area <= 0.0 {
        return 0.0; // canonical fully-degenerate fallback: {1, 0}.
    }

    let cx = mx / area;
    let cy = my / area;
    let cxx = ixx / area - cx * cx;
    let cyy = iyy / area - cy * cy;
    let cxy = ixy / area - cx * cy;

    if cxy.abs() < COVARIANCE_AXIS_ALIGNED_EPSILON {
        // Axis-aligned shortcut: diagonal variances sorted larger-first; the
        // MINOR axis is the smaller-variance axis.
        let minor = if cxx <= cyy { (1.0, 0.0) } else { (0.0, 1.0) };
        direction_deg(minor.0, minor.1)
    } else {
        // Closed-form 2x2 eigendecomposition, eigenpairs sorted major-first;
        // take the MINOR eigenvector (smaller eigenvalue).
        let half_trace = 0.5 * (cxx + cyy);
        let half_diff = 0.5 * (cxx - cyy);
        let root = (half_diff * half_diff + cxy * cxy).sqrt();
        let lambda_minor = half_trace - root;
        // Eigenvector candidates for lambda_minor; pick the stabler one.
        let a = (cxy, lambda_minor - cxx);
        let b = (lambda_minor - cyy, cxy);
        let (vx, vy) = if a.0 * a.0 + a.1 * a.1 >= b.0 * b.0 + b.1 * b.1 {
            a
        } else {
            b
        };
        let len = vx.hypot(vy);
        if len == 0.0 {
            0.0 // numerically isotropic: canonical {1, 0}.
        } else {
            direction_deg(vx / len, vy / len)
        }
    }
}

/// Bridge-orientation detection: a faithful port of canonical OrcaSlicer's
/// active inline `detect_bridging_direction` overload pair declared in
/// `BridgeDetector.hpp` (the active call path is `LayerRegion::
/// process_external_surfaces`, `LayerRegion.cpp`), polygons-overload feeding
/// the lines-overload scoring.
///
/// Pipeline: `overhang = difference(to_cover, anchors)`; floating edges =
/// [`floating_edges_of_gated_area`]; candidate normals are the RIGHT normal
/// `(dy, -dx)` of each floating edge, unit-normalized and deduped by the
/// quantization key `ceil(atan2(n.y, n.x) * 1000)` keeping the first-seen
/// normal per key. Cost of a candidate is the sum of `|edge . n|` over ALL
/// floating edges with the edge vector UNNORMALIZED (cost embeds edge
/// length). The winner is the minimal-cost candidate; exact-cost ties resolve
/// to the SMALLEST quantized-angle key (ADR-0061, intentional deterministic
/// divergence from canonical's hash-iteration order). The returned direction
/// is the winning normal flipped — `dir = (n.y, -n.x)`, parallel to the
/// winning floating edge — converted once to degrees mod 180 in [0, 180).
///
/// Empty candidate sets fall back to the principal-component minor axis of
/// the overhang area; fully-degenerate input yields canonical `{1, 0}` ->
/// 0.0 degrees (never NaN, never panics).
///
/// Pure and deterministic. NOTE: not yet wired into
/// [`assemble_bridge_areas`] — see packet 235 (later steps wire the seam).
pub fn detect_bridging_direction_deg(to_cover: &[ExPolygon], anchors: &[ExPolygon]) -> f32 {
    // Clipper2 boolean ops with an empty clip set do not reliably return the
    // subject, so short-circuit the no-anchor case.
    let overhang = if anchors.is_empty() {
        to_cover.to_vec()
    } else {
        difference(to_cover, anchors)
    };
    let floating = floating_edges_of_gated_area(&overhang, anchors);

    // Candidate set: right normal (dy, -dx) of each floating edge,
    // unit-normalized, deduped by quantized angle (first-seen wins).
    let mut edges: Vec<(f64, f64)> = Vec::with_capacity(floating.len());
    let mut candidates: Vec<(i64, f64, f64)> = Vec::new();
    for (from, to) in &floating {
        let dx = (to.x - from.x) as f64;
        let dy = (to.y - from.y) as f64;
        let len = dx.hypot(dy);
        if len == 0.0 {
            continue;
        }
        edges.push((dx, dy));
        let nx = dy / len;
        let ny = -dx / len;
        // Canonicalize negative zero so atan2 takes its (+0, -1) -> +pi
        // branch rather than -pi; keeps every quantization key on the
        // canonical (-pi, pi] atan2 range. Required by ADR-0061's
        // "Key-space well-definedness" section: under the smallest-key
        // tie-break, signed-zero keys would order geometrically identical
        // normals 2*pi apart and invert tie outcomes.
        let nx = if nx == 0.0 { 0.0 } else { nx };
        let ny = if ny == 0.0 { 0.0 } else { ny };
        let key = (ny.atan2(nx) * ORIENTATION_QUANTIZATION_SCALE).ceil() as i64;
        if !candidates.iter().any(|&(k, _, _)| k == key) {
            candidates.push((key, nx, ny));
        }
    }

    if candidates.is_empty() {
        return principal_minor_axis_deg(&overhang);
    }

    // Minimal-cost candidate; exact-cost ties resolve to the smallest
    // quantized-angle key (ADR-0061).
    let mut best: Option<(i64, f64, f64)> = None; // (key, nx, ny) at min cost
    let mut best_cost = f64::INFINITY;
    for &(key, nx, ny) in &candidates {
        let cost: f64 = edges
            .iter()
            .map(|&(dx, dy)| (dx * nx + dy * ny).abs())
            .sum();
        let wins = match best {
            None => true,
            Some((best_key, _, _)) => cost < best_cost || (cost == best_cost && key < best_key),
        };
        if wins {
            best = Some((key, nx, ny));
            best_cost = cost;
        }
    }
    let (_, nx, ny) = best.expect("candidates is non-empty");

    // Return direction = winning normal flipped: parallel to the winning
    // floating edge, in degrees mod 180, half-open [0, 180).
    direction_deg(ny, -nx)
}

// ============================================================================
// update_external_bridge_orientation
// ============================================================================

/// Re-derive a region's bridge orientation from its GATED bridge areas and
/// the RAW previous-layer contours (packet 235 Step 2, AC-6).
///
/// Invoked from `commit_shell_classification_builtin` immediately after
/// [`gate_bridge_areas_by_unsupported_span`], so `region.bridge_areas` is
/// already the gated (unsupported-span-only) geometry and
/// `lower_layer_slices` are the same raw lower-layer contours the gate
/// subtracted. This call OVERWRITES the mesh-heuristic orientation that
/// [`assemble_bridge_areas`] wrote during `PrePass::Slice`; that earlier
/// value is retired in a later step.
///
/// No-op when the gated areas are empty (fully supported span, or `None`
/// lower layer — the gate already cleared the candidates in that case):
/// the pre-existing field value is left untouched.
pub fn update_external_bridge_orientation(
    region: &mut SlicedRegion,
    lower_layer_slices: Option<&[ExPolygon]>,
) {
    if region.bridge_areas.is_empty() {
        return;
    }
    let anchors = lower_layer_slices.unwrap_or(&[]);
    region.bridge_orientation_deg = detect_bridging_direction_deg(&region.bridge_areas, anchors);
}

// ============================================================================
// assemble_flat_bridge_areas
// ============================================================================

/// Canonical OrcaSlicer's `SCALED_EPSILON`, in millimetres: exactly one PnP
/// workspace unit (1 unit = 100 nm = 1e-4 mm). `PrintObject::detect_surfaces_type`
/// diffs the lower layer with `ApplySafetyOffset::Yes`, which inflates the lower
/// set by this much so boundary-coincident material is not labelled a bridge.
const SCALED_EPSILON_MM: f32 = 0.0001;

/// Miter limit canonical passes to the bridge opening
/// (`opening_ex(..., jtMiter, 3.0)`).
const FLAT_BRIDGE_OPENING_MITER_LIMIT: f64 = 3.0;

/// Arc tolerance (mm) for the safety-offset inflate of the lower layer. The
/// offset is one workspace unit with a square join, so this only bounds
/// numerical noise; it never becomes emitted geometry on its own.
const SAFETY_OFFSET_ARC_TOLERANCE_MM: f32 = 0.05;

/// Default extrusion width (mm) when neither `outer_wall_line_width` nor the
/// typed `line_width` carries a positive value — the same floor
/// `support_analysis_producer::resolve_contact_params`
/// (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`) applies,
/// and the same guest-side perimeter default.
const DEFAULT_LINE_WIDTH_MM: f32 = 0.4;

/// Resolve the external-perimeter extrusion width (mm) the way canonical's
/// `flow(frExternalPerimeter).scaled_width()` resolves it, mirroring
/// `resolve_contact_params` exactly: the `outer_wall_line_width` extension when
/// it carries an absolute width, else the typed `line_width`, else
/// [`DEFAULT_LINE_WIDTH_MM`].
fn resolve_external_perimeter_width_mm(config: &ResolvedConfig) -> f32 {
    let typed_line_width = if config.line_width > 0.0 {
        config.line_width as f32
    } else {
        DEFAULT_LINE_WIDTH_MM
    };
    match config.extensions.get("outer_wall_line_width") {
        Some(ConfigValue::Float(value)) => *value as f32,
        Some(ConfigValue::FloatOrPercent {
            value,
            is_percent: false,
        }) => *value as f32,
        _ => typed_line_width,
    }
}

/// Detect and record *flat* (perfectly horizontal) unsupported bridge spans.
///
/// The sloped-overhang bridge pipeline ([`assemble_bridge_areas`]) never sees
/// perfectly-horizontal downward facets: they classify as
/// [`slicer_ir::FacetClass::BottomSurface`], not
/// [`slicer_ir::FacetClass::Overhang`], so a flat beam spanning a gap between
/// two supports is missed (its underside normal points straight down, so the
/// facet-normal test alone cannot tell a build-plate bottom from a
/// gap-spanning bridge). This function recovers those spans by intersecting
/// two pre-computed polygon sets:
///
/// 1. `bottom_surface_footprint` — the XY projection of the region's
///    `BottomSurface` facets (the region's flat-bottom footprint,
///    via [`crate::algos::mesh_analysis::bottom_surface_footprint`]), and
/// 2. `unsupported_region` — this layer's area **not** supported by the layer
///    below, computed by the caller (`PrePass::Slice`) as
///    `diff(this-layer cross-section, safety_offset(previous-layer))`, the port
///    of `PrintObject::detect_surfaces_type`'s
///    `diff_ex(slices, lower_slices, ApplySafetyOffset::Yes)`,
///
/// then clipping the result to the region's own printable area
/// (`region.infill_areas`, mirroring [`assemble_bridge_areas`]), applying
/// canonical's opening, and appending every surviving component to
/// `region.bridge_areas`.
///
/// # Canonical classification
///
/// Canonical OrcaSlicer has no free-edge / enclosed-gap discriminator: a bottom
/// area the layer below does not cover **is** a bridge
/// (`PrintObject::detect_surfaces_type` tags it `stBottomBridge`) as long as it
/// survives canonical's opening. This function applies that rule to PnP's flat
/// family — [`assemble_bridge_areas`] owns the facet/sloped family — with the
/// same operator and the same radius:
///
/// ```text
/// bridged = opening_ex(flat_bridge, 0.1 × external_perimeter_width, Miter, 3.0)
/// ```
///
/// The opening deletes features narrower than `0.2 × width` (0.08 mm at a
/// 0.4 mm width), so a sub-printable sliver is not reported as bridge area.
/// Every component that survives the opening is kept **in full**, so a
/// free-edge band — the outward-growing perimeter of a sloped solid — now
/// classifies as a bridge exactly as canonical calls it.
///
/// `external_perimeter_width_mm` must be positive; callers resolve it through
/// [`resolve_external_perimeter_width_mm`] (`outer_wall_line_width`, else
/// `line_width`, else [`DEFAULT_LINE_WIDTH_MM`]).
///
/// # First-layer / build-plate exclusion
///
/// Canonical demotes a bottom with no lower layer (`lower_layer == nullptr`) to
/// `stBottom`, never a bridge. The caller enforces the same guard by requiring
/// a previous cross-section for the object. A genuinely *supported* bottom
/// surface (its footprint sits entirely on material below) diffs to empty and
/// is likewise a no-op here. The discriminator is exactly "is this flat bottom
/// supported by the layer below?" — never the facet normal in isolation.
pub fn assemble_flat_bridge_areas(
    region: &mut SlicedRegion,
    bottom_surface_footprint: &[ExPolygon],
    unsupported_region: &[ExPolygon],
    external_perimeter_width_mm: f32,
) {
    if bottom_surface_footprint.is_empty() || unsupported_region.is_empty() {
        return;
    }

    // Flat unsupported = flat-bottom footprint ∩ not-supported-by-below.
    // Intersecting with the bottom-surface footprint restricts the result to
    // genuinely-flat bottoms, so sloped overhangs (already handled by
    // `assemble_bridge_areas`, and never `BottomSurface`) are not re-flagged
    // here and keep their min-length / anchor-width validity filtering.
    let flat_unsupported = intersection(bottom_surface_footprint, unsupported_region);
    if flat_unsupported.is_empty() {
        return;
    }

    // Clip to this region's printable area (matches `assemble_bridge_areas`).
    let flat_bridge = intersection(&flat_unsupported, &region.infill_areas);
    if flat_bridge.is_empty() {
        return;
    }

    // Canonical classification: one opening at a tenth of the external
    // perimeter width, miter limit 3.0, every surviving component kept.
    let bridged = opening_ex(
        &flat_bridge,
        0.1 * external_perimeter_width_mm as f64,
        OffsetJoinType::Miter,
        FLAT_BRIDGE_OPENING_MITER_LIMIT,
    );
    if bridged.is_empty() {
        return;
    }

    region.bridge_areas.extend(bridged);
    region.is_bridge = true;
}

// ============================================================================
// batch_slice_objects_by_layer
// ============================================================================

/// Pre-slice every object mesh referenced across `global_layers` in one
/// `slice_mesh_ex` call per object, instead of one call per (layer, object)
/// pair. `slice_mesh_ex` performs a full O(triangle-count) scan of the mesh
/// regardless of how many Z planes it is given — calling it once per layer,
/// as a naive per-layer loop does, makes the whole pre-pass slice step
/// O(layer-count × triangle-count) instead of O(triangle-count +
/// intersections). Returns raw (pre-closing-radius) polygons keyed by
/// `(global_layer_index, object_id)`.
pub fn batch_slice_objects_by_layer(
    mesh: &MeshIR,
    global_layers: &[GlobalLayer],
) -> HashMap<u32, HashMap<ObjectId, Vec<ExPolygon>>> {
    let mut layers_and_zs_by_object: HashMap<ObjectId, (Vec<u32>, Vec<f32>)> = HashMap::new();
    for layer in global_layers {
        for active in &layer.active_regions {
            let entry = layers_and_zs_by_object
                .entry(active.object_id.clone())
                .or_default();
            if entry.0.last() != Some(&layer.index) {
                entry.0.push(layer.index);
                entry.1.push(layer.z);
            }
        }
    }

    let mut result: HashMap<u32, HashMap<ObjectId, Vec<ExPolygon>>> = HashMap::new();
    for (object_id, (layer_indices, zs)) in layers_and_zs_by_object {
        let Some(object) = mesh.objects.iter().find(|o| o.id == object_id) else {
            continue;
        };
        let sliced = slice_mesh_ex(&object_world_mesh(object), &zs);
        for (layer_index, polygons) in layer_indices.into_iter().zip(sliced) {
            result
                .entry(layer_index)
                .or_default()
                .insert(object_id.clone(), polygons);
        }
    }
    result
}

// ============================================================================
// batch_bottom_surface_footprints
// ============================================================================

/// Pre-compute each object's whole-mesh bottom-surface XY footprint once.
/// [`crate::algos::mesh_analysis::bottom_surface_footprint`] depends only on
/// an object's mesh, transform, and facet classes — never on a specific
/// layer — so calling it from inside the per-layer loop (as
/// `execute_prepass_slice_single_layer` does when no cache is supplied)
/// recomputes an identical boolean-union result once per layer instead of
/// once per object. Returns `None` if `surface_class` is absent (matching
/// the guard `execute_prepass_slice_single_layer` already applies before
/// ever needing this footprint).
pub fn batch_bottom_surface_footprints(
    mesh: &MeshIR,
    surface_class: Option<&SurfaceClassificationIR>,
) -> HashMap<ObjectId, Vec<ExPolygon>> {
    let mut result = HashMap::new();
    let Some(sc) = surface_class else {
        return result;
    };
    for object in &mesh.objects {
        let Some(obj_data) = sc.per_object.get(&object.id) else {
            continue;
        };
        let footprint = crate::algos::mesh_analysis::bottom_surface_footprint(
            &object.mesh,
            &object.transform,
            &obj_data.facet_classes,
        );
        result.insert(object.id.clone(), footprint);
    }
    result
}

/// Layer-invariant and per-layer data pre-computed once for a whole
/// `PrePass::Slice` run, built by [`batch_slice_objects_by_layer`] and
/// [`batch_bottom_surface_footprints`], and reused across every
/// [`execute_prepass_slice_single_layer_with_cache`] call for that run.
pub struct PrepassSliceCache<'a> {
    /// This layer's raw (pre-closing-radius) slice polygons, keyed by object id.
    pub raw_polygons: &'a HashMap<ObjectId, Vec<ExPolygon>>,
    /// The *previous* global layer's raw slice polygons, keyed by object id.
    /// `None` for the first layer in plan order (no predecessor). The
    /// flat-bridge classifier derives this layer's unsupported region as
    /// `diff(raw_polygons, safety_offset(prev_raw_polygons))`, and a missing
    /// entry for an object means "no lower layer" — canonical demotes that
    /// bottom to `stBottom`, so it is never a flat bridge.
    pub prev_raw_polygons: Option<&'a HashMap<ObjectId, Vec<ExPolygon>>>,
    /// Whole-object bottom-surface XY footprint, keyed by object id.
    pub bottom_surface_footprint: &'a HashMap<ObjectId, Vec<ExPolygon>>,
}

// ============================================================================
// execute_prepass_slice_single_layer
// ============================================================================

/// Produce the `SliceIR` for `layer` by slicing every referenced object mesh
/// at `layer.z`.
pub fn execute_prepass_slice_single_layer(
    mesh: &MeshIR,
    layer: &GlobalLayer,
    surface_class: Option<&SurfaceClassificationIR>,
    region_map: Option<&RegionMapIR>,
) -> Result<SliceIR, LayerSliceError> {
    execute_prepass_slice_single_layer_impl(mesh, layer, surface_class, region_map, None)
}

/// Same as [`execute_prepass_slice_single_layer`], but takes pre-computed
/// per-object data from `cache` (see [`PrepassSliceCache`]) instead of
/// calling `slice_mesh_ex` and `bottom_surface_footprint` from scratch.
pub fn execute_prepass_slice_single_layer_with_cache(
    mesh: &MeshIR,
    layer: &GlobalLayer,
    surface_class: Option<&SurfaceClassificationIR>,
    region_map: Option<&RegionMapIR>,
    cache: &PrepassSliceCache<'_>,
) -> Result<SliceIR, LayerSliceError> {
    execute_prepass_slice_single_layer_impl(mesh, layer, surface_class, region_map, Some(cache))
}

fn execute_prepass_slice_single_layer_impl(
    mesh: &MeshIR,
    layer: &GlobalLayer,
    surface_class: Option<&SurfaceClassificationIR>,
    region_map: Option<&RegionMapIR>,
    cache: Option<&PrepassSliceCache<'_>>,
) -> Result<SliceIR, LayerSliceError> {
    let mut regions = Vec::with_capacity(layer.active_regions.len());
    for active in &layer.active_regions {
        // Region mapping surfaces modifier sub-regions in the authoritative
        // layer plan before this stage. Their geometry is minted from modifier
        // footprints after shell/paint refinement, not by re-slicing the whole
        // object under the modifier id here.
        if slicer_ir::is_modifier_namespace_id(active.region_id) {
            continue;
        }
        let object = mesh
            .objects
            .iter()
            .find(|o| o.id == active.object_id)
            .ok_or_else(|| LayerSliceError::UnknownObject {
                layer_index: layer.index,
                object_id: active.object_id.clone(),
            })?;

        let (slice_closing_radius_mm, external_perimeter_width_mm) = if let Some(rm) = region_map {
            let key = RegionKey {
                global_layer_index: layer.index,
                object_id: active.object_id.clone(),
                region_id: active.region_id,
                variant_chain: Vec::new(),
            };
            let entry = rm.entries.get(&key);
            if entry.is_none() {
                log::warn!(
                    "PrePass::Slice: region_map present but missing entry for \
                     (layer={}, object={}, region={}) — falling back to legacy \
                     defaults; this indicates a partial RegionMap commit",
                    layer.index,
                    active.object_id,
                    active.region_id,
                );
                debug_assert!(
                    false,
                    "PrePass::Slice: region_map present but lookup miss for \
                     (layer={}, object={}, region={}); partial RegionMap \
                     violates the scheduler contract",
                    layer.index, active.object_id, active.region_id,
                );
            }
            match entry {
                Some(_) => {
                    let cfg = rm.config_for(&key);
                    (
                        cfg.slice_closing_radius,
                        resolve_external_perimeter_width_mm(cfg),
                    )
                }
                None => (0.0_f32, DEFAULT_LINE_WIDTH_MM),
            }
        } else {
            (0.0_f32, DEFAULT_LINE_WIDTH_MM)
        };

        let raw_polygons = match cache.and_then(|c| c.raw_polygons.get(&active.object_id)) {
            Some(cached) => cached.clone(),
            None => {
                let mut sliced = slice_mesh_ex(&object_world_mesh(object), &[layer.z]);
                sliced.pop().unwrap_or_default()
            }
        };
        let polygons = if slice_closing_radius_mm > 0.0 {
            apply_slice_closing_radius(raw_polygons, slice_closing_radius_mm)
        } else {
            raw_polygons
        };

        let is_bridge = if let Some(sc) = surface_class {
            if let Some(obj_data) = sc.per_object.get(&active.object_id) {
                let contours: Vec<Polygon> = if polygons.is_empty() {
                    let verts = &object.mesh.vertices;
                    if verts.is_empty() {
                        Vec::new()
                    } else {
                        let mut min_x = verts[0].x;
                        let mut max_x = verts[0].x;
                        let mut min_y = verts[0].y;
                        let mut max_y = verts[0].y;
                        for v in verts.iter().skip(1) {
                            if v.x < min_x {
                                min_x = v.x;
                            }
                            if v.x > max_x {
                                max_x = v.x;
                            }
                            if v.y < min_y {
                                min_y = v.y;
                            }
                            if v.y > max_y {
                                max_y = v.y;
                            }
                        }
                        vec![Polygon {
                            points: vec![
                                Point2::from_mm(min_x, min_y),
                                Point2::from_mm(max_x, min_y),
                                Point2::from_mm(max_x, max_y),
                                Point2::from_mm(min_x, max_y),
                            ],
                        }]
                    }
                } else {
                    polygons.iter().map(|ep| ep.contour.clone()).collect()
                };
                classify_region_surfaces(object, obj_data, &contours, layer.z)
            } else {
                false
            }
        } else {
            false
        };

        let mut sliced_region = SlicedRegion {
            object_id: active.object_id.clone(),
            region_id: active.region_id,
            polygons: polygons.clone(),
            infill_areas: polygons,
            nonplanar_surface: None,
            effective_layer_height: active.effective_layer_height,
            segment_annotations: HashMap::new(),
            variant_chain: Vec::new(),
            top_shell_index: None,
            bottom_shell_index: None,
            top_solid_fill: Vec::new(),
            bottom_solid_fill: Vec::new(),
            is_bridge,
            bridge_areas: vec![],
            bridge_orientation_deg: 0.0,
            sparse_infill_area: Vec::new(),
            internal_solid_fill: Vec::new(),
            internal_bridge_areas: Vec::new(),
            internal_bridge_angles_deg: Vec::new(),
            raft_fill: Vec::new(),
        };

        assemble_bridge_areas(&mut sliced_region, surface_class);

        // Flat (perfectly horizontal) unsupported bridge spans. The sloped
        // pipeline above only sees `Overhang`-derived clusters; a flat beam
        // over a gap has a `BottomSurface` underside and is missed there. The
        // unsupported region is this object's cross-section area not covered by
        // the previous layer, after canonical's `ApplySafetyOffset::Yes`
        // inflate of the lower set.
        //
        // The lower layer is also the guard: canonical's
        // `PrintObject::detect_surfaces_type` demotes a bottom with
        // `lower_layer == nullptr` to `stBottom`, never a bridge, so an object
        // with no previous cross-section in the batch (its first layer) has no
        // flat-bridge candidates at all. `gate_bridge_areas_by_unsupported_span`
        // would clear a layer-0 candidate downstream anyway; skipping it here
        // also avoids paying for geometry the gate throws away.
        if let Some(sc) = surface_class {
            if let Some(obj_data) = sc.per_object.get(&active.object_id) {
                let current_raw = cache
                    .and_then(|c| c.raw_polygons.get(&active.object_id))
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                let prev_raw = cache
                    .and_then(|c| c.prev_raw_polygons)
                    .and_then(|m| m.get(&active.object_id))
                    .map(Vec::as_slice);
                if let Some(prev_raw) = prev_raw {
                    let safety_offset = offset(
                        prev_raw,
                        SCALED_EPSILON_MM,
                        OffsetJoinType::Square,
                        SAFETY_OFFSET_ARC_TOLERANCE_MM,
                    );
                    let unsupported = difference(current_raw, &safety_offset);
                    if !unsupported.is_empty() {
                        let bottom_fp = match cache
                            .and_then(|c| c.bottom_surface_footprint.get(&active.object_id))
                        {
                            Some(cached) => cached.clone(),
                            None => crate::algos::mesh_analysis::bottom_surface_footprint(
                                &object.mesh,
                                &object.transform,
                                &obj_data.facet_classes,
                            ),
                        };
                        assemble_flat_bridge_areas(
                            &mut sliced_region,
                            &bottom_fp,
                            &unsupported,
                            external_perimeter_width_mm,
                        );
                    }
                }
            }
        }

        regions.push(sliced_region);
    }

    Ok(SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: layer.index,
        z: layer.z,
        regions,
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_external_perimeter_width_mm;
    use slicer_ir::{ConfigValue, ResolvedConfig};

    /// Canonical's `flow(frExternalPerimeter).scaled_width()` is the
    /// `outer_wall_line_width` knob when the user sets it; `line_width` is only
    /// the fallback. A regression here would silently classify at the wrong
    /// opening radius (0.1 × width), so the extension must win.
    #[test]
    fn external_perimeter_width_prefers_outer_wall_extension() {
        let mut config = ResolvedConfig {
            line_width: 0.4,
            ..ResolvedConfig::default()
        };
        config
            .extensions
            .insert("outer_wall_line_width".to_owned(), ConfigValue::Float(0.62));
        assert_eq!(resolve_external_perimeter_width_mm(&config), 0.62);
    }

    /// A *percent* `outer_wall_line_width` resolves against the nozzle, not
    /// against `line_width`, and only the config pipeline may do that; the
    /// resolver therefore ignores it and keeps the typed width, exactly as
    /// `resolve_contact_params` does.
    #[test]
    fn external_perimeter_width_ignores_percent_form() {
        let mut config = ResolvedConfig {
            line_width: 0.45,
            ..ResolvedConfig::default()
        };
        config.extensions.insert(
            "outer_wall_line_width".to_owned(),
            ConfigValue::FloatOrPercent {
                value: 150.0,
                is_percent: true,
            },
        );
        assert_eq!(resolve_external_perimeter_width_mm(&config), 0.45);
    }

    /// With no extension the typed `line_width` is the external perimeter
    /// width (canonical falls back the same way).
    #[test]
    fn external_perimeter_width_falls_back_to_typed_line_width() {
        let config = ResolvedConfig {
            line_width: 0.5,
            ..ResolvedConfig::default()
        };
        assert_eq!(resolve_external_perimeter_width_mm(&config), 0.5);
    }

    /// An unset typed width is `0.0`; the resolver must substitute the
    /// perimeter default rather than zero, which would make the opening a
    /// no-op and keep the sub-printable sliver tail.
    #[test]
    fn external_perimeter_width_defaults_when_unset() {
        let config = ResolvedConfig::default();
        assert_eq!(config.line_width, 0.0, "fixture precondition");
        assert_eq!(resolve_external_perimeter_width_mm(&config), 0.4);
    }
}
