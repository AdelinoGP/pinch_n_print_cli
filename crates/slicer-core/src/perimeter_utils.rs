// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/PerimeterGenerator.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Shared perimeter-generation helpers used by both classic and Arachne perimeter
//! modules.

use std::collections::HashMap;

use slicer_ir::{
    ExPolygon, MaterialBoundarySegment, PaintSemantic, PaintValue, Point2, Point3, Point3WithWidth,
    WallBoundaryType, WallFeatureFlags,
};

use crate::geometry::closest_point_on_segment;

/// Default base speed used for normalizing speed factors (mm/s).
pub const BASE_SPEED: f32 = 50.0;

/// Build feature flags for wall points by propagating segment_annotations.
///
/// Reads Material and FuzzySkin semantics from `segment_annotations` for the given
/// polygon index. Sets `tool_index` from Material ToolIndex values, `fuzzy_skin`
/// from FuzzySkin Flag values. Detects adjacent material changes and returns
/// `WallBoundaryType::MaterialBoundary` with a segment for each transition.
///
/// The `is_outer` flag controls the fallback boundary type when no Material
/// annotations are present (or annotations are present but have no transitions):
/// - Outer walls (`is_outer = true`): return `WallBoundaryType::ExteriorSurface`.
/// - Inner walls (`is_outer = false`): return `WallBoundaryType::Interior`.
///
/// When Material annotations are present with transitions, both outer and inner walls
/// return `WallBoundaryType::MaterialBoundary` regardless of `is_outer`.
///
/// # Inner-wall paint sampling — geometric reprojection
///
/// Inner walls are produced by iterative polygon offsetting. The offset operation
/// does NOT carry paint data forward — `segment_annotations` remain keyed to the
/// ORIGINAL region polygons, not the inset polygons. On convex shapes the vertex
/// counts and ordering happen to match, but on concave shapes the inset ring has
/// different vertex counts and ordering, so naive index-based sampling assigns the
/// wrong tool/material color to inner-wall vertices near concave features.
///
/// When `inset_ring_points` and `original_polygons` are both `Some`, this function
/// uses **geometric reprojection** regardless of `is_outer`: for each wall vertex,
/// the nearest edge across all original contours is found, then the nearest
/// endpoint vertex of that edge is selected, and its annotation is used. This is
/// deterministic (pure function of inputs) and correct for all polygon shapes
/// including concave ones, and for any vertex set — including ones with no
/// correspondence at all to the original contour's vertex count/ordering (e.g.
/// Arachne's Voronoi-beading-derived walls).
///
/// Classic's outer wall withholds `inset_ring_points`/`original_polygons` (passing
/// `None`) because its offset preserves the original contour's vertex ordering
/// 1:1, so index-based lookup is exact and cheaper. When either input is `None`,
/// index-based lookup is used as a fallback (backward-compatible path for callers
/// that do not supply geometry) — this is correct only when the caller's vertex
/// ordering genuinely matches the original contour's.
pub fn build_wall_flags(
    num_points: usize,
    poly_idx: usize,
    segment_annotations: &HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>>,
    is_outer: bool,
    inset_ring_points: Option<&[Point2]>,
    original_polygons: Option<&[ExPolygon]>,
    variant_fuzzy: bool,
) -> (Vec<WallFeatureFlags>, WallBoundaryType) {
    let mut flags = vec![default_feature_flags(); num_points];

    // Painted-variant FuzzySkin (D14): the signal arrives on the region's
    // `variant_chain`, not `segment_annotations`, and applies uniformly to the
    // whole painted region. Seed every vertex; per-vertex segment_annotations
    // reads below can only add fuzzy, never clear it.
    if variant_fuzzy {
        for flag in flags.iter_mut() {
            flag.fuzzy_skin = true;
        }
    }

    // Determine which annotation source to use for each flag slot. Reprojection
    // is used whenever the caller supplies wall-vertex/original-polygon geometry,
    // regardless of `is_outer`: classic's outer wall withholds this geometry (its
    // offset preserves the original contour's vertex ordering 1:1, so index-based
    // lookup is exact and cheaper), but Arachne has no wall type with that
    // guarantee — its vertices come from Voronoi beading, not polygon offsetting —
    // so Arachne callers must supply geometry unconditionally, including for their
    // outer wall.
    let use_reprojection = inset_ring_points.is_some() && original_polygons.is_some();

    if use_reprojection {
        let ring_pts = inset_ring_points.unwrap();
        let orig_polys = original_polygons.unwrap();

        // For each flag slot, find the annotation values via reprojection.
        for (i, flag) in flags.iter_mut().enumerate() {
            // For the closing-repeat vertex (index == num_points - 1 when it duplicates
            // index 0), use the same annotation as vertex 0. ring_pts has N entries
            // (no closing repeat), so clamp to ring_pts.len() - 1.
            let pt = ring_pts[i.min(ring_pts.len().saturating_sub(1))];

            if let Some((orig_poly_idx, orig_vert_idx)) = nearest_original_vertex(pt, orig_polys) {
                // Material annotation
                if let Some(Some(PaintValue::ToolIndex(tool))) = segment_annotations
                    .get(&PaintSemantic::Material)
                    .and_then(|pp| pp.get(orig_poly_idx))
                    .and_then(|vv| vv.get(orig_vert_idx))
                {
                    flag.tool_index = Some(*tool);
                }
                // FuzzySkin annotation
                if let Some(Some(PaintValue::Flag(true))) = segment_annotations
                    .get(&PaintSemantic::FuzzySkin)
                    .and_then(|pp| pp.get(orig_poly_idx))
                    .and_then(|vv| vv.get(orig_vert_idx))
                {
                    flag.fuzzy_skin = true;
                }
            }
        }

        // Build the effective annotation sequence for boundary-type detection by
        // reprojecting each ring vertex to its nearest original annotation.
        let projected_mat_vals: Vec<Option<PaintValue>> = (0..num_points)
            .map(|i| {
                let pt = ring_pts[i.min(ring_pts.len().saturating_sub(1))];
                nearest_original_vertex(pt, orig_polys)
                    .and_then(|(opi, ovi)| {
                        segment_annotations
                            .get(&PaintSemantic::Material)
                            .and_then(|pp| pp.get(opi))
                            .and_then(|vv| vv.get(ovi))
                            .cloned()
                    })
                    .flatten()
            })
            .collect();

        let has_any_material = projected_mat_vals.iter().any(|v| v.is_some());
        let boundary_type = if has_any_material {
            let transitions = find_all_transitions(&projected_mat_vals);
            if transitions.is_empty() {
                if is_outer {
                    WallBoundaryType::ExteriorSurface
                } else {
                    WallBoundaryType::Interior
                }
            } else {
                WallBoundaryType::MaterialBoundary {
                    segments: transitions,
                }
            }
        } else if is_outer {
            WallBoundaryType::ExteriorSurface
        } else {
            WallBoundaryType::Interior
        };

        return (flags, boundary_type);
    }

    // ── Legacy index-based path (outer walls and callers without geometry) ────

    let material_values: Option<&Vec<Option<PaintValue>>> = segment_annotations
        .get(&PaintSemantic::Material)
        .and_then(|per_poly| per_poly.get(poly_idx));

    let fuzzy_values: Option<&Vec<Option<PaintValue>>> = segment_annotations
        .get(&PaintSemantic::FuzzySkin)
        .and_then(|per_poly| per_poly.get(poly_idx));

    if let Some(mat_vals) = material_values {
        for (i, flag) in flags.iter_mut().enumerate() {
            if let Some(Some(PaintValue::ToolIndex(tool))) = mat_vals.get(i) {
                flag.tool_index = Some(*tool);
            }
        }
    }

    if let Some(fuzzy_vals) = fuzzy_values {
        for (i, flag) in flags.iter_mut().enumerate() {
            if let Some(Some(PaintValue::Flag(true))) = fuzzy_vals.get(i) {
                flag.fuzzy_skin = true;
            }
        }
    }

    let boundary_type = match material_values {
        Some(mat_vals) => {
            let transitions = find_all_transitions(mat_vals);
            if transitions.is_empty() {
                if is_outer {
                    WallBoundaryType::ExteriorSurface
                } else {
                    WallBoundaryType::Interior
                }
            } else {
                WallBoundaryType::MaterialBoundary {
                    segments: transitions,
                }
            }
        }
        None => {
            if is_outer {
                WallBoundaryType::ExteriorSurface
            } else {
                WallBoundaryType::Interior
            }
        }
    };

    (flags, boundary_type)
}

/// Find the nearest original contour vertex to `p` across all `original_polygons`.
///
/// Returns `(polygon_index, vertex_index)` into `original_polygons`. The search
/// finds the closest edge endpoint along the nearest segment, which gives a stable
/// nearest-vertex that respects polygon topology rather than raw Euclidean vertex
/// proximity across disconnected polygons.
///
/// Returns `None` if `original_polygons` is empty or all contours have no vertices.
fn nearest_original_vertex(p: Point2, original_polygons: &[ExPolygon]) -> Option<(usize, usize)> {
    let mut best_dist_sq = f64::MAX;
    let mut best: Option<(usize, usize)> = None;

    for (poly_idx, ep) in original_polygons.iter().enumerate() {
        let pts = &ep.contour.points;
        let n = pts.len();
        if n == 0 {
            continue;
        }
        for edge_i in 0..n {
            let edge_j = (edge_i + 1) % n;
            let cp = closest_point_on_segment(p, pts[edge_i], pts[edge_j]);
            if cp.distance_sq < best_dist_sq {
                best_dist_sq = cp.distance_sq;
                // Pick the endpoint of this edge that is nearer to the projected point.
                let da_sq = {
                    let dx = pts[edge_i].x as f64 - cp.point.x as f64;
                    let dy = pts[edge_i].y as f64 - cp.point.y as f64;
                    dx * dx + dy * dy
                };
                let db_sq = {
                    let dx = pts[edge_j].x as f64 - cp.point.x as f64;
                    let dy = pts[edge_j].y as f64 - cp.point.y as f64;
                    dx * dx + dy * dy
                };
                let vert_idx = if da_sq <= db_sq { edge_i } else { edge_j };
                best = Some((poly_idx, vert_idx));
            }
        }
    }

    best
}

/// Find all material boundary transitions on a polygon contour.
///
/// Walks the circular material paint list and emits a `MaterialBoundarySegment`
/// for each edge where two adjacent points have different tool indices.
/// Each segment records the point range (half-open `[i, i+1)`) and the
/// near/far tool indices.
pub fn find_all_transitions(mat_vals: &[Option<PaintValue>]) -> Vec<MaterialBoundarySegment> {
    let n = mat_vals.len();
    if n < 2 {
        return Vec::new();
    }

    let mut segments = Vec::new();

    for i in 0..n {
        let next = (i + 1) % n;
        let tool_a = extract_tool_index(&mat_vals[i]);
        let tool_b = extract_tool_index(&mat_vals[next]);

        if tool_a != tool_b {
            segments.push(MaterialBoundarySegment {
                point_range: i as u32..(i as u32 + 1),
                near_tool: tool_a,
                far_tool: tool_b,
            });
        }
    }

    segments
}

/// Extract tool index from a PaintValue, if it is a ToolIndex variant.
pub fn extract_tool_index(val: &Option<PaintValue>) -> Option<u32> {
    match val {
        Some(PaintValue::ToolIndex(t)) => Some(*t),
        _ => None,
    }
}

/// Convert an ExPolygon contour to a Vec<Point3WithWidth> at the given Z and width.
///
/// Converts from scaled i64 coordinates to f32 mm. The returned Vec has N+1
/// entries for an N-vertex polygon: the first point is repeated at the end so
/// the path is a closed loop in OrcaSlicer convention
/// (`ExtrusionPath::is_closed()` at `ExtrusionEntity.hpp:269`). Downstream
/// consumers (seam-placer, fuzzy-skin, G-code emitter) rely on this so the
/// final closing edge is processed exactly like every other wall segment.
///
/// `overhang_bands` classifies each vertex's `overhang_quartile` by winding-number
/// membership against the region's quartile-band polygons (T-024-WIRE-VIEW-CONSUMER):
/// a vertex falling in more than one band (only possible at a shared boundary, since
/// bands are otherwise a distance-based partition) takes the most severe (highest)
/// matching quartile, mirroring `overhang-classifier-default`'s own MAX-per-entity
/// convention. Pass `&[]` for callers with no band data (e.g. non-planar shells,
/// which are an explicitly separate concern — see `docs/specs/perimeter-modules-orca-parity-roadmap.md`
/// D-3) to get the prior all-`None` behavior.
pub fn expolygon_to_path3d(
    contour: &slicer_ir::Polygon,
    z: f32,
    width: f32,
    overhang_bands: &[slicer_ir::slice_ir::QuartileBand],
) -> Vec<Point3WithWidth> {
    let mut pts: Vec<Point3WithWidth> = contour
        .points
        .iter()
        .map(|p| {
            let x = slicer_ir::units_to_mm(p.x);
            let y = slicer_ir::units_to_mm(p.y);
            let overhang_quartile = overhang_bands
                .iter()
                .filter(|band| {
                    band.polygons.iter().any(|poly| {
                        slicer_ir::point_in_polygon_winding(poly, f64::from(x), f64::from(y), 0.0)
                    })
                })
                .map(|band| band.quartile)
                .max();
            Point3WithWidth {
                x,
                y,
                z,
                width,
                flow_factor: 1.0,
                overhang_quartile,
                dist_to_top_mm: 0.0,
            }
        })
        .collect();
    close_loop(&mut pts);
    pts
}

/// Create default WallFeatureFlags (no paint, no bridge, no thin wall).
pub fn default_feature_flags() -> WallFeatureFlags {
    WallFeatureFlags {
        tool_index: None,
        fuzzy_skin: false,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: HashMap::new(),
    }
}

/// A seam candidate: a 3D position and a score (higher = better).
pub struct SeamCandidate {
    /// Position in mm.
    pub position: Point3,
    /// Score (higher is preferred).
    pub score: f32,
}

/// Generate seam candidates at sharp corners of the outer wall path.
///
/// All corners with a non-trivial turn angle are candidates. Concave corners
/// receive a higher score (seam is less visible there), convex corners get a
/// lower but positive score.
///
/// # Deprecated
///
/// Emits a candidate for *every* vertex with a non-zero turn angle, including
/// near-collinear noise from redundant/densely-sampled contour points. Use
/// [`generate_sharp_corner_seam_candidates`] instead, which gates candidacy on
/// an absolute turn-angle threshold (see packet 108).
#[deprecated(
    note = "use generate_sharp_corner_seam_candidates with an explicit angle threshold instead"
)]
pub fn generate_seam_candidates(contour: &slicer_ir::Polygon, z: f32) -> Vec<SeamCandidate> {
    let pts = &contour.points;
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }

    let mut signed_area: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        signed_area += (pts[i].x as i128) * (pts[j].y as i128);
        signed_area -= (pts[j].x as i128) * (pts[i].y as i128);
    }
    let is_ccw = signed_area > 0;

    let mut candidates = Vec::new();

    for i in 0..n {
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let next = (i + 1) % n;

        let dx1 = pts[i].x - pts[prev].x;
        let dy1 = pts[i].y - pts[prev].y;
        let dx2 = pts[next].x - pts[i].x;
        let dy2 = pts[next].y - pts[i].y;

        let cross = dx1 * dy2 - dy1 * dx2;
        if cross == 0 {
            continue;
        }

        let len1 = ((dx1 * dx1 + dy1 * dy1) as f64).sqrt();
        let len2 = ((dx2 * dx2 + dy2 * dy2) as f64).sqrt();
        let denom = len1 * len2;
        if denom == 0.0 {
            continue;
        }

        let sin_angle = (cross.unsigned_abs() as f64 / denom) as f32;
        let is_concave = if is_ccw { cross < 0 } else { cross > 0 };
        let score = if is_concave {
            sin_angle + 1.0
        } else {
            sin_angle * 0.5
        };

        let position = Point3 {
            x: slicer_ir::units_to_mm(pts[i].x),
            y: slicer_ir::units_to_mm(pts[i].y),
            z,
        };
        candidates.push(SeamCandidate { position, score });
    }

    candidates
}

/// Generate seam candidates at sharp corners of the outer wall path, gated by an
/// absolute turn-angle threshold.
///
/// Only vertices whose turn angle (deviation from a straight pass-through, in
/// degrees) is `>= angle_threshold_deg` become candidates. This filters out the
/// dense runs of near-collinear points that a contour tessellation can leave on
/// otherwise-straight edges, so a single sharp corner produces one candidate
/// instead of one per redundant point.
///
/// Both convex and concave corners qualify once they clear the threshold.
/// Concave corners keep the score bonus from [`generate_seam_candidates`] (seam
/// is less visible there); convex corners get a lower but positive score.
///
/// Parity note: OrcaSlicer's seam placer has no binary candidacy threshold — it
/// scores every vertex with a continuous Gaussian angle penalty and only uses a
/// 55° cutoff for "sharp corner" snapping. This port intentionally uses a binary
/// angle threshold (default 30°) per packet 108 design.
///
/// Guarantees a non-empty result for any non-degenerate contour (i.e. one with
/// at least one vertex whose adjacent edges both have non-zero length): a
/// contour with no vertex clearing `angle_threshold_deg` degrades to a single
/// candidate at the sharpest-turn vertex, scored by the same convention as the
/// threshold-gated candidates. OrcaSlicer has no binary candidacy cutoff — the
/// sharpest corner always competes — so this fallback keeps low-curvature
/// contours (e.g. MMU bisector-fragment perimeters) from producing zero
/// candidates, which is fatal downstream in `com.core.seam-placer`. Note that
/// [`apply_seam_paint_bias`] runs *after* this function and can still empty a
/// non-empty result via its blocker hard-filter — that fatal path is
/// intentional and unaffected by this fallback.
pub fn generate_sharp_corner_seam_candidates(
    contour: &slicer_ir::Polygon,
    z: f32,
    angle_threshold_deg: f32,
) -> Vec<SeamCandidate> {
    let pts = &contour.points;
    let n = pts.len();
    if n < 3 {
        return Vec::new();
    }

    let mut signed_area: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        signed_area += (pts[i].x as i128) * (pts[j].y as i128);
        signed_area -= (pts[j].x as i128) * (pts[i].y as i128);
    }
    let is_ccw = signed_area > 0;

    let threshold_rad = (angle_threshold_deg.max(0.0) as f64).to_radians();

    let mut candidates = Vec::new();

    // Tracks the sharpest-turn usable vertex seen so far (largest absolute
    // turn angle), regardless of whether it clears `threshold_rad`. Used as a
    // fallback when the threshold gate rejects every vertex.
    let mut sharpest: Option<(f64, SeamCandidate)> = None;

    for i in 0..n {
        let prev = if i == 0 { n - 1 } else { i - 1 };
        let next = (i + 1) % n;

        let dx1 = pts[i].x - pts[prev].x;
        let dy1 = pts[i].y - pts[prev].y;
        let dx2 = pts[next].x - pts[i].x;
        let dy2 = pts[next].y - pts[i].y;

        let len1 = ((dx1 * dx1 + dy1 * dy1) as f64).sqrt();
        let len2 = ((dx2 * dx2 + dy2 * dy2) as f64).sqrt();
        let denom = len1 * len2;
        if denom == 0.0 {
            continue;
        }

        let cross = dx1 * dy2 - dy1 * dx2;
        let dot = dx1 * dx2 + dy1 * dy2;

        // Absolute turn angle (deviation from straight pass-through), in [0, pi].
        let turn_angle_rad = (cross as f64).atan2(dot as f64).abs();

        let sin_angle = (cross.unsigned_abs() as f64 / denom) as f32;
        let is_concave = if is_ccw { cross < 0 } else { cross > 0 };
        let score = if is_concave {
            sin_angle + 1.0
        } else {
            sin_angle * 0.5
        };

        let position = Point3 {
            x: slicer_ir::units_to_mm(pts[i].x),
            y: slicer_ir::units_to_mm(pts[i].y),
            z,
        };

        if sharpest
            .as_ref()
            .is_none_or(|(best_angle, _)| turn_angle_rad > *best_angle)
        {
            sharpest = Some((turn_angle_rad, SeamCandidate { position, score }));
        }

        if turn_angle_rad < threshold_rad {
            continue;
        }

        candidates.push(SeamCandidate { position, score });
    }

    if candidates.is_empty() {
        if let Some((_, fallback)) = sharpest {
            candidates.push(fallback);
        }
    }

    candidates
}

/// Apply paint-driven seam bias/exclusion to already-generated seam candidates
/// (packet 108, T-P98-SEAM). `slicer-core` cannot depend on `slicer-sdk`
/// (`PaintRegionLayerView` lives there), so this helper is data-driven: the
/// caller (a perimeter-generation module) is responsible for extracting
/// `enforcer_polys`/`blocker_polys` from `PaintSemantic::Custom("seam_enforcer")`
/// / `Custom("seam_blocker")` segment annotations and passing them in as plain
/// `ExPolygon`s.
///
/// - **Blocker** (hard filter): any candidate whose position falls inside a
///   blocker polygon is removed from `candidates` entirely — mirrors
///   OrcaSlicer's hard `Blocked` discriminator (checked before all geometric
///   scoring).
/// - **Enforcer** (soft bias): any *surviving* candidate whose position falls
///   inside an enforcer polygon has `score` multiplied by `0.1`.
///
/// # Score-direction note
///
/// `SeamCandidate::score` is documented above as "higher is preferred", but
/// the only production consumer — `seam-placer`'s `select_seam_candidate`
/// (`SeamMode::Nearest`) — picks the candidate with the **minimum**
/// `effective_score` via `Iterator::min_by` (`modules/core-modules/seam-placer/src/lib.rs`).
/// Candidates emitted by the perimeter-generation modules always carry
/// `SeamReason::Aligned` (zero reason-bonus), so `effective_score ==
/// candidate.score` for this path and *lower actually wins*. Multiplying by
/// `0.1` therefore *increases* an enforced candidate's real-world selection
/// priority, matching the packet's locked design (see docs/05_module_sdk.md,
/// "Paint-seam consumption").
pub fn apply_seam_paint_bias(
    candidates: &mut Vec<SeamCandidate>,
    enforcer_polys: &[ExPolygon],
    blocker_polys: &[ExPolygon],
) {
    if !blocker_polys.is_empty() {
        candidates.retain(|c| {
            let p = Point2 {
                x: slicer_ir::mm_to_units(c.position.x),
                y: slicer_ir::mm_to_units(c.position.y),
            };
            !point_in_any_polygon(&p, blocker_polys)
        });
    }
    if !enforcer_polys.is_empty() {
        for c in candidates.iter_mut() {
            let p = Point2 {
                x: slicer_ir::mm_to_units(c.position.x),
                y: slicer_ir::mm_to_units(c.position.y),
            };
            if point_in_any_polygon(&p, enforcer_polys) {
                c.score *= 0.1;
            }
        }
    }
}

/// Test whether a point lies strictly inside any polygon in the given slice.
///
/// Returns `true` iff `pt` is strictly inside at least one `ExPolygon` contour
/// (i.e. the standard ray-casting winding test resolves to inside). A point
/// exactly ON a boundary edge returns `false` (strict-inside semantics).
///
/// Used for per-vertex `is_bridge` derivation against `region.bridge_areas()`.
pub fn point_in_any_polygon(pt: &Point2, polys: &[ExPolygon]) -> bool {
    polys
        .iter()
        .any(|ep| point_in_polygon_strict(pt, &ep.contour.points))
}

/// Ray-casting point-in-polygon test. Returns `true` iff `pt` is strictly inside
/// the polygon defined by `verts` (closed implicitly). Returns `false` for a point
/// exactly on a boundary edge.
fn point_in_polygon_strict(pt: &Point2, verts: &[Point2]) -> bool {
    let n = verts.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let vi = &verts[i];
        let vj = &verts[j];
        // Check exact boundary — return false for on-edge points.
        // The cross product of (vj→vi) × (vj→pt) == 0 and t in [0,1] means on-edge.
        let dx = vi.x - vj.x;
        let dy = vi.y - vj.y;
        let ex = pt.x - vj.x;
        let ey = pt.y - vj.y;
        let cross = dx * ey - dy * ex;
        if cross == 0 {
            // Collinear — check if pt is between vj and vi.
            let t_num_x = ex;
            let t_num_y = ey;
            let in_x = if dx != 0 {
                (t_num_x >= 0) == (dx > 0) && t_num_x.unsigned_abs() <= dx.unsigned_abs()
            } else {
                ex == 0
            };
            let in_y = if dy != 0 {
                (t_num_y >= 0) == (dy > 0) && t_num_y.unsigned_abs() <= dy.unsigned_abs()
            } else {
                ey == 0
            };
            if in_x && in_y {
                return false; // on the boundary
            }
        }
        // Standard ray-casting from pt in +X direction.
        // Cross-multiply to avoid integer division:
        //   pt.x < vj.x + (vi.x - vj.x) * (pt.y - vj.y) / (vi.y - vj.y)
        // ⟺ (pt.x - vj.x) * (vi.y - vj.y) < (vi.x - vj.x) * (pt.y - vj.y)  [when vi.y > vj.y]
        // ⟺ (pt.x - vj.x) * (vi.y - vj.y) > (vi.x - vj.x) * (pt.y - vj.y)  [when vi.y < vj.y]
        if (vi.y > pt.y) != (vj.y > pt.y) {
            let lhs = (pt.x - vj.x) as i128 * (vi.y - vj.y) as i128;
            let rhs = (vi.x - vj.x) as i128 * (pt.y - vj.y) as i128;
            if (vi.y > vj.y) == (lhs < rhs) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

fn close_loop<T: Clone>(items: &mut Vec<T>) {
    if let Some(first) = items.first().cloned() {
        items.push(first);
    }
}

// ── Wall sequence reorder (T-054, T-054b, T-054c) ────────────────────────

/// Wall emission sequence. Per OrcaSlicer `PerimeterGenerator::process`
/// (PerimeterGenerator.cpp:1801-1913).
///
/// - `InnerOuter` (canonical): `[Outer, Inner_0, Inner_1, ...]`. Most common.
/// - `OuterInner` (reversed): `[..., Inner_1, Inner_0, Outer]`. The first wall
///   emitted is the innermost; the outer is emitted last. Stronger outer corners.
/// - `InnerOuterInner` (sandwich): per-outer-contour grouping `[Inner_0, Outer,
///   Inner_1, ...]`. The first inner is emitted first, then the outer, then
///   the remaining inner walls. Improves outer-corner strength while keeping
///   inner walls first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WallSequence {
    /// Outer wall first, then inner walls (canonical).
    InnerOuter,
    /// Inner walls first, then outer wall last (reversed).
    OuterInner,
    /// Sandwich: first inner, then outer, then remaining inners.
    InnerOuterInner,
}

/// Reorder the generated `Vec<WallLoop>` per the configured `WallSequence`.
/// Per ADR-0011, the in-module wall tree is built during generation and
/// discarded after this call — the IR stays flat.
///
/// This is a pure function: same input → same output. No randomness, no global
/// state.
///
/// `tree` is a parallel slice of `PolygonTreeNode` (one per input polygon)
/// used to group outer-contour children for the sandwich mode. When `tree` is
/// empty or the mode is `InnerOuter` / `OuterInner`, the function falls back
/// to flat reordering (no per-outer-contour grouping).
pub fn wall_sequence_reorder(
    walls: &mut Vec<slicer_ir::WallLoop>,
    mode: WallSequence,
    tree: &[crate::polygon_tree::PolygonTreeNode],
) {
    if walls.is_empty() {
        return;
    }
    match mode {
        WallSequence::InnerOuter => {
            // Canonical order: outer (index 0) first, then inner (indices 1..N).
            // No reordering needed — generation already produces this order.
        }
        WallSequence::OuterInner => {
            // Reversed: outer (index 0) last, then inner (indices 1..N).
            walls.reverse();
        }
        WallSequence::InnerOuterInner => {
            // Sandwich: first inner, then outer, then remaining inners.
            // The walls Vec has shape `[Outer, Inner_0, Inner_1, ..., Inner_{N-1}]`.
            // After reorder: `[Inner_0, Outer, Inner_1, ..., Inner_{N-1}]`.
            let n = walls.len();
            if n == 2 {
                walls.swap(0, 1);
            } else if n >= 3 {
                let outer = walls[0].clone();
                let inners: Vec<_> = walls[1..].to_vec();
                walls.clear();
                walls.push(inners[0].clone());
                walls.push(outer);
                for inner in inners.iter().skip(1) {
                    walls.push(inner.clone());
                }
            }
        }
    }
    // `tree` is the in-module scaffold from ADR-0011; documented but unused in
    // the M1 implementation (per-outer-contour grouping happens in the caller
    // by passing per-contour wall subsets to this function for the sandwich
    // mode in M2). Suppress the unused warning without dropping the parameter.
    let _ = tree;
}

#[cfg(test)]
mod wall_sequence_reorder_tests {
    use super::*;
    use slicer_ir::{ExtrusionPath3D, ExtrusionRole, LoopType, WallLoop};

    fn make_wall(perimeter_index: u32, loop_type: LoopType, role: ExtrusionRole) -> WallLoop {
        WallLoop {
            perimeter_index,
            loop_type,
            path: ExtrusionPath3D {
                points: vec![],
                role,
                speed_factor: 1.0,
            },
            width_profile: Default::default(),
            feature_flags: Default::default(),
            boundary_type: WallBoundaryType::ExteriorSurface,
        }
    }

    #[test]
    fn inner_outer_is_canonical_no_reorder() {
        let mut walls = vec![
            make_wall(0, LoopType::Outer, ExtrusionRole::OuterWall),
            make_wall(1, LoopType::Inner, ExtrusionRole::InnerWall),
            make_wall(2, LoopType::Inner, ExtrusionRole::InnerWall),
        ];
        wall_sequence_reorder(&mut walls, WallSequence::InnerOuter, &[]);
        // Order unchanged: [Outer, Inner, Inner].
        assert_eq!(walls[0].perimeter_index, 0);
        assert_eq!(walls[1].perimeter_index, 1);
        assert_eq!(walls[2].perimeter_index, 2);
    }

    #[test]
    fn outer_inner_reverses() {
        let mut walls = vec![
            make_wall(0, LoopType::Outer, ExtrusionRole::OuterWall),
            make_wall(1, LoopType::Inner, ExtrusionRole::InnerWall),
            make_wall(2, LoopType::Inner, ExtrusionRole::InnerWall),
        ];
        wall_sequence_reorder(&mut walls, WallSequence::OuterInner, &[]);
        // Reversed: [Inner, Inner, Outer].
        assert_eq!(walls[0].perimeter_index, 2);
        assert_eq!(walls[1].perimeter_index, 1);
        assert_eq!(walls[2].perimeter_index, 0);
    }

    #[test]
    fn inner_outer_inner_sandwich() {
        let mut walls = vec![
            make_wall(0, LoopType::Outer, ExtrusionRole::OuterWall),
            make_wall(1, LoopType::Inner, ExtrusionRole::InnerWall),
            make_wall(2, LoopType::Inner, ExtrusionRole::InnerWall),
        ];
        wall_sequence_reorder(&mut walls, WallSequence::InnerOuterInner, &[]);
        // Sandwich: [Inner_0, Outer, Inner_1].
        assert_eq!(walls[0].perimeter_index, 1);
        assert_eq!(walls[1].perimeter_index, 0);
        assert_eq!(walls[2].perimeter_index, 2);
    }

    #[test]
    fn inner_outer_inner_with_two_walls_swaps_outer_and_first_inner() {
        let mut walls = vec![
            make_wall(0, LoopType::Outer, ExtrusionRole::OuterWall),
            make_wall(1, LoopType::Inner, ExtrusionRole::InnerWall),
        ];
        wall_sequence_reorder(&mut walls, WallSequence::InnerOuterInner, &[]);
        // For N == 2: [Inner_0, Outer].
        assert_eq!(walls[0].perimeter_index, 1);
        assert_eq!(walls[1].perimeter_index, 0);
    }
}
