// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Arachne/WallToolPaths.cpp:86-201
// (stage 2's `simplify()`), src/libslic3r/Arachne/WallToolPaths.cpp:565-604
// (the remaining 8-stage pipeline), src/libslic3r/MultiMaterialSegmentation.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! T-204 (9-stage Arachne input-outline preprocessing) and T-P96-E
//! (per-color MMU input validation) — packet 110 step 5.

use crate::arachne::ToolIndex;
use crate::polygon_ops::{
    intersection_ex, offset, offset2_ex, remove_duplicates, remove_small_and_small_holes, union_ex,
    OffsetJoinType,
};
use slicer_ir::{point_in_polygon_winding, ExPolygon, Point2, Polygon, UNITS_PER_MM};

// ---------------------------------------------------------------------------
// Diagnostics
// ---------------------------------------------------------------------------
//
// AC-N3's dropped-feature warning and AC-7's overlap warning use this
// crate's standard `log::warn!` facade. `slicer-core`'s `log` dependency is
// unconditional (`crates/slicer-core/Cargo.toml`), matching the convention
// every other host crate in this workspace already uses unconditionally
// (`slicer-gcode`, `slicer-runtime`, `slicer-model-io`, `slicer-wasm-host`,
// `slicer-scheduler` all pin `log = "0.4"` outside any feature gate). No
// `tracing` dependency exists anywhere in this workspace (verified by grep),
// so `log` — not `tracing` — is this module's diagnostics facade.
//
// The detection logic itself (which features were dropped / which color
// pairs overlap) is factored into small pure functions
// ([`detect_dropped_features`], [`detect_color_overlaps`]) that both the
// `log::warn!` call sites and this crate's tests use directly. Tests assert
// on these pure functions' return values rather than capturing `log`
// output: no log-capture crate (`test-log`, `env_logger`, a hand-rolled
// `log::Log` implementation, etc.) exists anywhere in this workspace
// (verified by grep across every `crates/*/Cargo.toml` and
// `crates/*/tests/*.rs`), and asserting on the pure detection function
// exercises the actual meaningful behavior without depending on the global
// `log` facade, which is fragile under parallel test execution (a single
// process-wide logger, racing test threads).

// ---------------------------------------------------------------------------
// AC-6: preprocess_input_outline (9-stage pipeline)
// ---------------------------------------------------------------------------

/// Parameters controlling [`preprocess_input_outline`]'s 9-stage pipeline.
///
/// `Default` computes `epsilon_offset_mm` from `allowed_distance_mm` per the
/// verified OrcaSlicer/Cura Arachne formula (see that field's doc-comment).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PreprocessParams {
    /// Minimum segment length (mm) below which stage 2's simplify pass
    /// merges consecutive vertices. OrcaSlicer/Cura default: `0.5`.
    pub smallest_segment_mm: f64,
    /// Ramer-Douglas-Peucker tolerance (mm) for stage 2's simplify pass.
    /// OrcaSlicer/Cura default: `0.025`.
    pub allowed_distance_mm: f64,
    /// Net-zero triple-offset delta (mm) used by stage 1 and re-used by
    /// stages 3/6's self-intersection fix-up scale and stage 9's dropped-
    /// feature detection threshold.
    ///
    /// Formula (`WallToolPaths.cpp:565-604`): `epsilon_offset = (allowed_distance / 2) - 1nm`.
    /// Computed here in real mm units, not by blindly copying an OrcaSlicer
    /// integer literal (OrcaSlicer's own Arachne/Cura coordinate space is
    /// natively micron-scaled, not the 100 nm/unit scale this codebase uses
    /// — see `docs/08_coordinate_system.md`). With
    /// `allowed_distance_mm = 0.025`: `0.025 / 2.0 - 0.000001 = 0.012499 mm`
    /// (≈ 12.499 µm). This is within the packet's own sanity-check band of
    /// its stated "~11.5 µm" order-of-magnitude (≈ 8.7% apart, under the
    /// documented 15% tolerance) without being force-fit to exactly 11.5 µm;
    /// see [`preprocess_input_outline`]'s doc-comment for the mandatory
    /// hazard string and the follow-up note on why 11.5 µm itself likely
    /// comes from Cura's native micron-unit literal `- 1` (1 micron) rather
    /// than a literal nanometer.
    pub epsilon_offset_mm: f64,
    /// Angle tolerance (radians) for stage 5's colinear-edge removal.
    /// OrcaSlicer/Cura default: `0.005`.
    pub colinear_angle_tolerance_rad: f64,
    /// Side length (mm) used to derive stage 8's `removeSmallAreas` area
    /// threshold (`small_area_length_mm^2`). Not given a numeric value by
    /// this packet's spec text; defaults to `smallest_segment_mm` as a
    /// conservative choice consistent with the pipeline's other minimum-
    /// feature-size input. Override via this field if a different threshold
    /// is needed.
    pub small_area_length_mm: f64,
    /// Offset join style used by every offset stage.
    pub join: OffsetJoinType,
    /// Miter limit forwarded to the underlying Clipper2 offset calls.
    pub miter_limit: f64,
}

impl Default for PreprocessParams {
    fn default() -> Self {
        let smallest_segment_mm = 0.5;
        let allowed_distance_mm = 0.025;
        // epsilon_offset = (allowed_distance / 2) - 1nm, computed in real mm.
        let epsilon_offset_mm = (allowed_distance_mm / 2.0) - 1.0e-6;
        Self {
            smallest_segment_mm,
            allowed_distance_mm,
            epsilon_offset_mm,
            colinear_angle_tolerance_rad: 0.005,
            small_area_length_mm: smallest_segment_mm,
            join: OffsetJoinType::Miter,
            miter_limit: 2.0,
        }
    }
}

/// Runs the verified 9-stage Arachne input-outline preprocessing pipeline
/// (`WallToolPaths.cpp:565-604`):
///
/// 1. Net-zero triple offset (shrink `epsilon_offset`, grow `2*epsilon_offset`,
///    shrink `epsilon_offset`) — snaps/removes sub-epsilon features.
/// 2. Simplify (`smallest_segment` merge + Ramer-Douglas-Peucker at
///    `allowed_distance`).
/// 3. Fix self-intersections.
/// 4. Remove degenerate vertices.
/// 5. Remove colinear edges (`colinear_angle_tolerance_rad`).
/// 6. Fix self-intersections (repeat — stage 5 can reintroduce them).
/// 7. Remove degenerate vertices (repeat).
/// 8. Remove small areas (`small_area_length^2`, holes untouched).
/// 9. Final union.
///
/// **Hazard:** this pipeline intentionally destroys features < epsilon_offset ~11.5 µm
/// scale (stage 1's triple offset and stage 8's small-area removal both
/// operate at that order of magnitude — see [`PreprocessParams::epsilon_offset_mm`]
/// for this implementation's exact computed value). Do not feed this
/// pipeline geometry with intentional sub-epsilon detail; it will not
/// survive. When a fed-in feature does vanish, a `log::warn!` diagnostic is
/// emitted naming the dropped feature's centroid (see
/// [`detect_dropped_features`] for the pure detection logic backing it).
pub fn preprocess_input_outline(polys: &[ExPolygon], params: &PreprocessParams) -> Vec<ExPolygon> {
    let stages = run_nine_stage_pipeline(polys, params);
    let stage9 = stages.into_iter().last().unwrap_or_default();

    warn_dropped_features(polys, &stage9, params.epsilon_offset_mm);

    stage9
}

/// Test-introspection variant of [`preprocess_input_outline`]: runs the same
/// 9-stage pipeline (including the dropped-feature diagnostic emitted after
/// the final stage) but returns every stage's intermediate output instead of
/// just the last one — one entry per stage, 9 entries total, in pipeline
/// order (index 0 = stage 1's output, ..., index 8 = stage 9's/final
/// output). Exists so AC-6's "matches the recorded reference polygons within
/// tolerance per stage" acceptance criterion can be asserted on directly by
/// `preprocess_golden.rs`, without changing this module's public
/// `preprocess_input_outline` signature or behavior. Not intended for
/// production call sites — [`preprocess_input_outline`] remains the only
/// production entry point.
#[doc(hidden)]
pub fn preprocess_input_outline_with_stages(
    polys: &[ExPolygon],
    params: &PreprocessParams,
) -> Vec<Vec<ExPolygon>> {
    let stages = run_nine_stage_pipeline(polys, params);
    let stage9 = stages.last().cloned().unwrap_or_default();
    warn_dropped_features(polys, &stage9, params.epsilon_offset_mm);
    stages
}

/// Shared implementation backing both [`preprocess_input_outline`] and
/// [`preprocess_input_outline_with_stages`]: runs all 9 pipeline stages and
/// returns every intermediate result (9 entries, pipeline order). Does not
/// itself emit the dropped-feature diagnostic — callers do that against
/// whichever output they treat as final, so the diagnostic fires exactly
/// once regardless of which public entry point is used.
fn run_nine_stage_pipeline(polys: &[ExPolygon], params: &PreprocessParams) -> Vec<Vec<ExPolygon>> {
    let stage1 = triple_offset(polys, params);
    let stage2 = simplify_stage(
        &stage1,
        params.smallest_segment_mm,
        params.allowed_distance_mm,
    );
    let stage3 = fix_self_intersections(&stage2);
    let stage4 = remove_degenerate_verts(&stage3);
    let stage5 = remove_colinear_edges(&stage4, params.colinear_angle_tolerance_rad);
    let stage6 = fix_self_intersections(&stage5);
    let stage7 = remove_degenerate_verts(&stage6);
    let stage8 = remove_small_areas_stage(stage7.clone(), params.small_area_length_mm);
    let stage9 = union_ex(&stage8);

    vec![
        stage1, stage2, stage3, stage4, stage5, stage6, stage7, stage8, stage9,
    ]
}

/// Stage 1: net-zero triple offset (shrink `epsilon_offset`, grow
/// `2*epsilon_offset`, shrink `epsilon_offset`).
///
/// [`offset2_ex`] already implements a two-pass shrink-then-grow offset in
/// one call (`offset2_ex(polys, delta1, delta2, ...)`), so passes 1+2
/// (`-epsilon_offset` then `+2*epsilon_offset`) are a single `offset2_ex`
/// call; pass 3 (`-epsilon_offset`) is a follow-up [`offset`] call.
fn triple_offset(polys: &[ExPolygon], params: &PreprocessParams) -> Vec<ExPolygon> {
    let eps = params.epsilon_offset_mm;
    let shrink_grow = offset2_ex(polys, -eps, 2.0 * eps, params.join, params.miter_limit);
    if shrink_grow.is_empty() {
        return Vec::new();
    }
    offset(&shrink_grow, -eps as f32, params.join, 0.0)
}

/// Stage 2: simplify (`WallToolPaths.cpp:86-201`'s `simplify()`, called from
/// the `Polygons` wrapper at `WallToolPaths.cpp:232-245`/`:487-496`).
///
/// This is a single distance-gated linear pass, **not** length-only vertex
/// merging followed by a separate Ramer-Douglas-Peucker pass — canonical has
/// no RDP step here. A vertex is removed only when it is both (a) adjacent
/// to a short segment (`< smallest_line_segment_squared`, i.e.
/// `smallest_segment_mm`) *and* (b) removing it would deviate the outline by
/// no more than `allowed_error_distance_squared` (`allowed_distance_mm`),
/// with a small unconditional "always allowed to delete" floor for
/// near-degenerate (near-zero-length / near-exactly-colinear) vertices. See
/// [`simplify_ring`] for the per-ring port. Rings (and, for the contour
/// specifically, whole polygons) that collapse below 3 points are dropped —
/// `WallToolPaths.cpp:239-243`'s "erase from the collection".
fn simplify_stage(
    polys: &[ExPolygon],
    smallest_segment_mm: f64,
    allowed_distance_mm: f64,
) -> Vec<ExPolygon> {
    let smallest_segment_units = smallest_segment_mm * UNITS_PER_MM;
    let allowed_distance_units = allowed_distance_mm * UNITS_PER_MM;
    let smallest_line_segment_squared = smallest_segment_units * smallest_segment_units;
    let allowed_error_distance_squared = allowed_distance_units * allowed_distance_units;

    polys
        .iter()
        .filter_map(|exp| {
            let contour = simplify_ring(
                &exp.contour,
                smallest_line_segment_squared,
                allowed_error_distance_squared,
            );
            if contour.points.len() < 3 {
                // Canonical: a polygon whose contour collapses below a
                // triangle is erased from the collection entirely.
                return None;
            }
            let holes = exp
                .holes
                .iter()
                .map(|h| {
                    simplify_ring(
                        h,
                        smallest_line_segment_squared,
                        allowed_error_distance_squared,
                    )
                })
                .filter(|h| h.points.len() >= 3)
                .collect();
            Some(ExPolygon { contour, holes })
        })
        .collect()
}

/// "Always allowed to delete" floor: canonical hardcodes `scaled(0.005)` (5
/// micron = 0.005 mm) regardless of the caller's `smallest_segment`/
/// `allowed_distance` parameters (`WallToolPaths.cpp:138,157-158`). Ported to
/// this workspace's unit scale (1 unit = 100 nm): `0.005 mm * UNITS_PER_MM =
/// 50 units`.
const SIMPLIFY_ULTRA_SHORT_UNITS: f64 = 0.005 * UNITS_PER_MM;

/// Faithful port of `Arachne::simplify(Polygon&, int64_t, int64_t)`
/// (`WallToolPaths.cpp:86-201`) for a single ring (contour or hole).
///
/// Returns a ring with fewer than 3 points when the whole ring collapses;
/// callers must check `.points.len() < 3` and drop accordingly (mirrors
/// canonical's `thiss.points.clear()` at line 88 for the `size() < 3` input
/// case, generalized to also cover the ring simplifying down to nothing).
///
/// # Overflow note
///
/// Coordinates are workspace units (1 unit = 100 nm), so Shoelace cross
/// products (`x*y` terms) reach roughly `1e14` for a room-scale model and
/// the *squared* deviation term (`area_removed_so_far * area_removed_so_far`,
/// canonical line 155) would reach roughly `1e28` — this overflows `i64`
/// (and is uncomfortably close to `i128`'s ~`1.7e38` ceiling once further
/// products are added). Canonical dodges this by casting to `double` right
/// before the multiply (`WallToolPaths.cpp:155`); this port does the same:
/// exact-integer accumulation (`i128`, which safely holds every individual
/// Shoelace/length term at this workspace's realistic coordinate range) up
/// until the point where canonical itself would overflow, then `f64` for the
/// squaring/division that produces `height_2`.
fn simplify_ring(
    poly: &Polygon,
    smallest_line_segment_squared: f64,
    allowed_error_distance_squared: f64,
) -> Polygon {
    let pts = &poly.points;
    let n = pts.len();
    if n < 3 {
        // Canonical line 88: thiss.points.clear(); return.
        return Polygon { points: Vec::new() };
    }
    if n == 3 {
        // Canonical line 92: never simplify a triangle.
        return poly.clone();
    }

    let mut new_path: Vec<Point2> = Vec::with_capacity(n);
    let mut previous = pts[n - 1];
    let mut previous_previous = pts[n - 2];
    let mut current = pts[0];
    let mut accumulated_area_removed = shoelace_term(previous, current);

    for point_idx in 0..n {
        current = pts[point_idx % n];

        let next = if point_idx + 1 < n {
            pts[point_idx + 1]
        } else if point_idx + 1 == n && new_path.len() > 1 {
            // Spill over to the (partially built) new polygon so the last
            // vertex's removed-area check still has a valid "next".
            new_path[0]
        } else {
            pts[(point_idx + 1) % n]
        };

        let removed_area_next = shoelace_term(current, next);
        let negative_area_closing = shoelace_term(next, previous);
        accumulated_area_removed += removed_area_next;

        let length2 = dist_sq(current, previous);

        if (length2 as f64) < SIMPLIFY_ULTRA_SHORT_UNITS * SIMPLIFY_ULTRA_SHORT_UNITS {
            // Always allowed to delete segments < 5 micron.
            continue;
        }

        let area_removed_so_far = accumulated_area_removed + negative_area_closing;
        let base_length_2 = dist_sq(next, previous);

        if base_length_2 == 0 {
            // Two segments form a line back and forth with no area -> remove vertex.
            continue;
        }

        // h^2 = L^2 / b^2 (L = 2*Area via Shoelace doubling, b = base length).
        // See the overflow note above for why this multiply is done in f64.
        let area_removed_so_far_f = area_removed_so_far as f64;
        let height_2 = (area_removed_so_far_f * area_removed_so_far_f) / (base_length_2 as f64);

        if height_2 <= SIMPLIFY_ULTRA_SHORT_UNITS * SIMPLIFY_ULTRA_SHORT_UNITS
            && distance_to_infinite(current, previous, next) <= SIMPLIFY_ULTRA_SHORT_UNITS
        {
            // Almost exactly colinear (barring rounding); guard vs.
            // cancellation of +/- areas via the direct distance check.
            continue;
        }

        if (length2 as f64) < smallest_line_segment_squared
            && height_2 <= allowed_error_distance_squared
        {
            // Removing the vertex doesn't introduce too much error.
            let next_length2 = dist_sq(current, next);
            if (next_length2 as f64) > 4.0 * smallest_line_segment_squared {
                // Special case: next line is long. Removing would cause
                // artifacts. Instead move this point to the intersection of
                // the two lines (preserving direction), and drop the
                // previous point we wanted to keep — but only if the
                // intersection is itself safe.
                let intersection_point =
                    intersection_infinite(previous_previous, previous, current, next);
                let safe = match intersection_point {
                    Some(ip) => {
                        distance_to_infinite_squared(ip, previous, current)
                            <= allowed_error_distance_squared
                            && (dist_sq(ip, previous) as f64) <= smallest_line_segment_squared
                            && (dist_sq(ip, next) as f64) <= smallest_line_segment_squared
                    }
                    None => false,
                };
                if safe {
                    // New point seems valid.
                    current = intersection_point.expect("checked Some above");
                    if !new_path.is_empty() {
                        // A previous point was added; remove it.
                        new_path.pop();
                        previous = previous_previous;
                    }
                }
                // Else: can't find a better spot, and the line is > 5
                // micron. Leave it in (falls through to "don't remove").
            } else {
                // Remove the vertex.
                continue;
            }
        }

        // Don't remove the vertex.
        accumulated_area_removed = removed_area_next;
        previous_previous = previous;
        previous = current; // "previous" only updates if we DON'T remove the vertex.
        new_path.push(current);
    }

    Polygon { points: new_path }
}

/// Shoelace cross-product term `a.x*b.y - a.y*b.x` for two consecutive ring
/// vertices, exact in `i128` (see [`simplify_ring`]'s overflow note).
fn shoelace_term(a: Point2, b: Point2) -> i128 {
    (a.x as i128) * (b.y as i128) - (a.y as i128) * (b.x as i128)
}

/// Squared Euclidean distance between two points, exact in `i128`.
fn dist_sq(a: Point2, b: Point2) -> i128 {
    let dx = (a.x - b.x) as i128;
    let dy = (a.y - b.y) as i128;
    dx * dx + dy * dy
}

/// Squared perpendicular distance from point `p` to the infinite line
/// through `a` and `b`. Degenerate (`a == b`) falls back to squared distance
/// from `p` to `a`.
fn distance_to_infinite_squared(p: Point2, a: Point2, b: Point2) -> f64 {
    let abx = (b.x - a.x) as f64;
    let aby = (b.y - a.y) as f64;
    let apx = (p.x - a.x) as f64;
    let apy = (p.y - a.y) as f64;
    let len_sq = abx * abx + aby * aby;
    if len_sq <= 0.0 {
        return apx * apx + apy * apy;
    }
    let cross = abx * apy - aby * apx;
    (cross * cross) / len_sq
}

/// Perpendicular distance from point `p` to the infinite line through `a`
/// and `b`.
fn distance_to_infinite(p: Point2, a: Point2, b: Point2) -> f64 {
    distance_to_infinite_squared(p, a, b).sqrt()
}

/// Intersection point of infinite line `a1`-`a2` with infinite line
/// `b1`-`b2`, or `None` if the lines are parallel (or coincident).
/// Coordinates round to the nearest workspace unit, matching canonical's
/// integer `Point` intersection result.
fn intersection_infinite(a1: Point2, a2: Point2, b1: Point2, b2: Point2) -> Option<Point2> {
    let (x1, y1) = (a1.x as f64, a1.y as f64);
    let (x2, y2) = (a2.x as f64, a2.y as f64);
    let (x3, y3) = (b1.x as f64, b1.y as f64);
    let (x4, y4) = (b2.x as f64, b2.y as f64);

    let denom = (x1 - x2) * (y3 - y4) - (y1 - y2) * (x3 - x4);
    if denom == 0.0 {
        return None;
    }

    let a_cross = x1 * y2 - y1 * x2;
    let b_cross = x3 * y4 - y3 * x4;
    let px = (a_cross * (x3 - x4) - (x1 - x2) * b_cross) / denom;
    let py = (a_cross * (y3 - y4) - (y1 - y2) * b_cross) / denom;

    Some(Point2 {
        x: px.round() as i64,
        y: py.round() as i64,
    })
}

/// Stages 3 and 6: fix self-intersections.
///
/// `polygon_ops.rs` has no dedicated `fixSelfIntersections` primitive, but
/// its own [`crate::polygon_ops::validate_polygon_simplicity`] doc-comment
/// already documents the technique this reuses: re-running geometry through
/// Clipper2's `NonZero`-fill-rule union reconstructs simple polygons from
/// self-intersecting rings (e.g. bowties). [`union_ex`] is exactly that
/// self-union operation applied to a polygon *set*.
fn fix_self_intersections(polys: &[ExPolygon]) -> Vec<ExPolygon> {
    union_ex(polys)
}

/// Stages 4 and 7: remove degenerate vertices (consecutive duplicates, and
/// the closing duplicate on a ring whose first and last point coincide).
/// Rings that collapse below a triangle are dropped entirely.
fn remove_degenerate_verts(polys: &[ExPolygon]) -> Vec<ExPolygon> {
    polys
        .iter()
        .map(|exp| ExPolygon {
            contour: dedup_ring(&exp.contour),
            holes: exp.holes.iter().map(dedup_ring).collect(),
        })
        .filter(|exp| exp.contour.points.len() >= 3)
        .collect()
}

fn dedup_ring(poly: &Polygon) -> Polygon {
    let mut pts = poly.points.clone();
    remove_duplicates(&mut pts);
    if pts.len() > 1 && pts.first() == pts.last() {
        pts.pop();
    }
    Polygon { points: pts }
}

/// Stage 5: remove colinear edges. A vertex is dropped when the angle
/// between its incoming and outgoing edge vectors is within
/// `angle_tolerance_rad` of a straight line (0 radians of deviation). Rings
/// with fewer than 4 points, or that would collapse below a triangle, are
/// left unmodified.
fn remove_colinear_edges(polys: &[ExPolygon], angle_tolerance_rad: f64) -> Vec<ExPolygon> {
    polys
        .iter()
        .map(|exp| ExPolygon {
            contour: remove_colinear_ring(&exp.contour, angle_tolerance_rad),
            holes: exp
                .holes
                .iter()
                .map(|h| remove_colinear_ring(h, angle_tolerance_rad))
                .collect(),
        })
        .collect()
}

fn remove_colinear_ring(poly: &Polygon, angle_tolerance_rad: f64) -> Polygon {
    let pts = &poly.points;
    let n = pts.len();
    if n < 4 {
        return poly.clone();
    }
    let mut keep = vec![true; n];
    for i in 0..n {
        let prev = pts[(i + n - 1) % n];
        let cur = pts[i];
        let next = pts[(i + 1) % n];
        let v1x = (cur.x - prev.x) as f64;
        let v1y = (cur.y - prev.y) as f64;
        let v2x = (next.x - cur.x) as f64;
        let v2y = (next.y - cur.y) as f64;
        let len1 = (v1x * v1x + v1y * v1y).sqrt();
        let len2 = (v2x * v2x + v2y * v2y).sqrt();
        if len1 <= f64::EPSILON || len2 <= f64::EPSILON {
            continue;
        }
        let cos_theta = ((v1x * v2x + v1y * v2y) / (len1 * len2)).clamp(-1.0, 1.0);
        let angle = cos_theta.acos();
        if angle.abs() <= angle_tolerance_rad {
            keep[i] = false;
        }
    }
    let filtered: Vec<Point2> = pts
        .iter()
        .zip(keep.iter())
        .filter_map(|(p, &k)| if k { Some(*p) } else { None })
        .collect();
    if filtered.len() < 3 {
        return poly.clone();
    }
    Polygon { points: filtered }
}

/// Stage 8: remove small areas. Mirrors OrcaSlicer/Cura's
/// `removeSmallAreas(small_area_length^2, remove_holes=false)` by reusing
/// [`remove_small_and_small_holes`] with `min_hole_area = 0.0` (never
/// removes a hole here, matching `remove_holes=false`).
fn remove_small_areas_stage(
    mut polys: Vec<ExPolygon>,
    small_area_length_mm: f64,
) -> Vec<ExPolygon> {
    let side_units = small_area_length_mm * UNITS_PER_MM;
    let min_area_units2 = side_units * side_units;
    remove_small_and_small_holes(&mut polys, min_area_units2, 0.0);
    polys
}

/// AC-N3: pure detection logic backing [`warn_dropped_features`] — computes
/// the centroid (mm) of every `input` polygon smaller than
/// `epsilon_offset_mm` whose centroid is no longer covered by any `output`
/// polygon. Emits no diagnostic itself; see the module's "Diagnostics" note
/// for why this is factored out and asserted on directly by tests instead of
/// capturing `log` output.
pub fn detect_dropped_features(
    input: &[ExPolygon],
    output: &[ExPolygon],
    epsilon_offset_mm: f64,
) -> Vec<(f64, f64)> {
    let small_area_threshold_units2 = (epsilon_offset_mm * UNITS_PER_MM).powi(2);
    let mut dropped = Vec::new();
    for in_poly in input {
        let area_units2 = polygon_area_units2(&in_poly.contour);
        if area_units2 >= small_area_threshold_units2 {
            continue;
        }
        let (cx_mm, cy_mm) = polygon_centroid_mm(&in_poly.contour);
        let survived = output
            .iter()
            .any(|out_poly| point_in_polygon_winding(out_poly, cx_mm, cy_mm, 0.0));
        if !survived {
            dropped.push((cx_mm, cy_mm));
        }
    }
    dropped
}

/// AC-N3: emits a `log::warn!` diagnostic naming the centroid of every
/// feature [`detect_dropped_features`] reports as dropped by the pipeline.
fn warn_dropped_features(input: &[ExPolygon], output: &[ExPolygon], epsilon_offset_mm: f64) {
    for (cx_mm, cy_mm) in detect_dropped_features(input, output, epsilon_offset_mm) {
        log::warn!(
            "preprocess_input_outline dropped a feature smaller than epsilon_offset \
             ({epsilon_offset_mm:.6} mm) at centroid ({cx_mm:.6}, {cy_mm:.6}) mm"
        );
    }
}

/// Absolute shoelace area of a polygon ring, in workspace-unit² (1 unit =
/// 100 nm). Mirrors `polygon_tree.rs`'s private `contour_area_abs`.
fn polygon_area_units2(poly: &Polygon) -> f64 {
    let pts = &poly.points;
    if pts.len() < 3 {
        return 0.0;
    }
    let n = pts.len();
    let mut area: i128 = 0;
    for i in 0..n {
        let j = (i + 1) % n;
        area += (pts[i].x as i128) * (pts[j].y as i128);
        area -= (pts[j].x as i128) * (pts[i].y as i128);
    }
    (area.unsigned_abs() as f64) * 0.5
}

/// Area-weighted centroid of a polygon ring, in millimeters. Falls back to
/// the plain vertex average for degenerate (near-zero-area or < 3 point)
/// rings.
fn polygon_centroid_mm(poly: &Polygon) -> (f64, f64) {
    let pts = &poly.points;
    if pts.is_empty() {
        return (0.0, 0.0);
    }
    if pts.len() < 3 {
        return vertex_average_mm(pts);
    }
    let mut area = 0.0f64;
    let mut cx = 0.0f64;
    let mut cy = 0.0f64;
    for i in 0..pts.len() {
        let j = (i + 1) % pts.len();
        let xi = pts[i].x as f64;
        let yi = pts[i].y as f64;
        let xj = pts[j].x as f64;
        let yj = pts[j].y as f64;
        let cross = xi * yj - xj * yi;
        area += cross;
        cx += (xi + xj) * cross;
        cy += (yi + yj) * cross;
    }
    area *= 0.5;
    if area.abs() < f64::EPSILON {
        return vertex_average_mm(pts);
    }
    cx /= 6.0 * area;
    cy /= 6.0 * area;
    (cx / UNITS_PER_MM, cy / UNITS_PER_MM)
}

fn vertex_average_mm(pts: &[Point2]) -> (f64, f64) {
    let n = pts.len() as f64;
    if n == 0.0 {
        return (0.0, 0.0);
    }
    let (sx, sy) = pts
        .iter()
        .fold((0.0, 0.0), |(ax, ay), p| (ax + p.x as f64, ay + p.y as f64));
    ((sx / n) / UNITS_PER_MM, (sy / n) / UNITS_PER_MM)
}

// ---------------------------------------------------------------------------
// AC-7 (corrected per T-P96-E design correction): preprocess_per_color_inputs
// ---------------------------------------------------------------------------

/// T-P96-E per-color Arachne input validation — a **validated pass-through**,
/// not the bisector-contraction/tie-break algorithm this packet's original
/// `requirements.md`/`design.md` described.
///
/// # Why this deviates from the packet's original design text
///
/// The packet's own design text described this function as contracting or
/// removing bisector edges between neighboring different-color cells per a
/// `TieBreakRule` (default "lower tool index wins"), citing ADR-0013. That
/// citation is stale: `docs/adr/0013-mmu-per-color-outer-wall-fragmentation.md`
/// (as it reads today) states plainly (lines 9, 29) "There is no skip mask,
/// no per-edge ownership, and no tie-break rule," and documents that
/// Arachne's old union-trace special case was deliberately *removed* so
/// Arachne "also fragments per-color" like the classic perimeter generator
/// (line 32/40). The only tie-break rule the ADR names anywhere
/// ("lower-color-ID owns the bisector edge") is explicitly the **retired**
/// prior revision (line 56), not current doctrine.
///
/// Direct verification against OrcaSlicer's C++ source
/// (`PerimeterGenerator.cpp:2600-2653`'s `process_arachne()`,
/// `Arachne::WallToolPaths` constructor in `WallToolPaths.hpp:63-83`) shows
/// Arachne itself carries **zero** color/extruder/material-aware logic — no
/// tie-break, no bisector contraction. Per-color isolation happens entirely
/// upstream, during paint/region segmentation
/// (`MultiMaterialSegmentation.cpp` → per-extruder `LayerRegion` split);
/// `process_arachne()` receives one already-isolated color's polygon per
/// call, with no shared-bisector concept left for Arachne to resolve.
///
/// # Contract
///
/// This codebase's own upstream paint/region-split pipeline (P91-94)
/// already produces the non-overlapping per-color partition before this
/// function runs — matching where OrcaSlicer does that work. So each
/// color's cell boundary passes through **unmodified**; this function's job
/// is a sanity check on the non-overlap invariant, not to (re)produce or
/// repair the partition.
///
/// For every pair of different-color cells, if their contours overlap by
/// more than a small epsilon (checked via [`intersection_ex`], the same
/// Clipper2-backed primitive [`crate::polygon_tree`]/`polygon_ops.rs`
/// containment checks build on), that indicates an **upstream** bug (paint
/// segmentation produced an invalid partition). This function does not
/// panic and does not silently "fix" the overlap by resurrecting
/// contraction/tie-break logic — that would reintroduce the exact retired
/// behavior this correction rejects. Instead it emits a `log::warn!`
/// diagnostic naming the offending color pair and overlap area (see
/// [`detect_color_overlaps`] for the pure detection logic backing it), and
/// passes the cells through unmodified regardless. Partition repair, if ever
/// needed, belongs to the upstream paint pipeline — out of scope for Arachne
/// input preprocessing.
///
/// Output preserves input order (no `HashMap` iteration) for determinism.
///
/// Note: the packet's original signature used a `slicer_ir::Polygons` type
/// alias that does not exist anywhere in this workspace (verified by grep);
/// `Vec<ExPolygon>` — the type `polygon_ops.rs` itself uses throughout — is
/// used here instead. The packet's `tie_break: TieBreakRule` parameter is
/// dropped entirely (no `TieBreakRule` enum is defined) since it would be
/// dead code with zero effect under a pass-through contract.
pub fn preprocess_per_color_inputs(
    painted_cells: &[(ToolIndex, Vec<ExPolygon>)],
) -> Vec<(ToolIndex, Vec<ExPolygon>)> {
    for (color_a, color_b, overlap_area_mm2) in detect_color_overlaps(painted_cells) {
        log::warn!(
            "preprocess_per_color_inputs: colors {color_a} and {color_b} overlap by \
             {overlap_area_mm2:.6} mm^2 beyond epsilon; upstream paint-segmentation \
             partition invariant violated (see ADR-0013) — passing cells through \
             unmodified"
        );
    }

    painted_cells.to_vec()
}

/// Float-noise floor for [`detect_color_overlaps`]: two independently-
/// produced boundaries sharing a bisector can disagree by sub-micron
/// rounding without indicating a real partition violation.
const OVERLAP_EPSILON_MM2: f64 = 1.0e-6;

/// AC-7: pure detection logic backing the overlap diagnostic in
/// [`preprocess_per_color_inputs`] — computes every pair of different-color
/// cells in `painted_cells` whose contours overlap by more than
/// [`OVERLAP_EPSILON_MM2`], along with the overlap area (mm²). Emits no
/// diagnostic itself; see the module's "Diagnostics" note for why this is
/// factored out and asserted on directly by tests instead of capturing `log`
/// output.
pub fn detect_color_overlaps(
    painted_cells: &[(ToolIndex, Vec<ExPolygon>)],
) -> Vec<(ToolIndex, ToolIndex, f64)> {
    let mut overlaps = Vec::new();
    for i in 0..painted_cells.len() {
        for j in (i + 1)..painted_cells.len() {
            let (color_a, polys_a) = &painted_cells[i];
            let (color_b, polys_b) = &painted_cells[j];
            if color_a == color_b {
                continue;
            }
            let overlap = intersection_ex(polys_a, polys_b);
            let overlap_area_mm2: f64 = overlap
                .iter()
                .map(|p| polygon_area_units2(&p.contour) / (UNITS_PER_MM * UNITS_PER_MM))
                .sum();
            if overlap_area_mm2 > OVERLAP_EPSILON_MM2 {
                overlaps.push((*color_a, *color_b, overlap_area_mm2));
            }
        }
    }
    overlaps
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square_mm(x0: f64, y0: f64, side: f64) -> ExPolygon {
        let to_pt = |x: f64, y: f64| Point2 {
            x: (x * UNITS_PER_MM).round() as i64,
            y: (y * UNITS_PER_MM).round() as i64,
        };
        ExPolygon {
            contour: Polygon {
                points: vec![
                    to_pt(x0, y0),
                    to_pt(x0 + side, y0),
                    to_pt(x0 + side, y0 + side),
                    to_pt(x0, y0 + side),
                ],
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn triple_offset_is_net_zero_on_a_large_square() {
        let square = vec![square_mm(0.0, 0.0, 10.0)];
        let params = PreprocessParams::default();
        let out = triple_offset(&square, &params);
        assert!(!out.is_empty(), "large square must survive triple offset");
        let before = polygon_area_units2(&square[0].contour) / (UNITS_PER_MM * UNITS_PER_MM);
        let after: f64 = out
            .iter()
            .map(|p| polygon_area_units2(&p.contour) / (UNITS_PER_MM * UNITS_PER_MM))
            .sum();
        // Net-zero shrink/grow/shrink should preserve area closely for a
        // feature much larger than epsilon_offset.
        assert!(
            (before - after).abs() < 0.05,
            "expected near-identical area before={before} after={after}"
        );
    }

    #[test]
    fn triple_offset_destroys_a_sub_epsilon_square() {
        let params = PreprocessParams::default();
        // Side length far below epsilon_offset (~0.0125 mm): 0.001 mm.
        let tiny = vec![square_mm(100.0, 100.0, 0.001)];
        let out = triple_offset(&tiny, &params);
        let survives = out.iter().any(|p| polygon_area_units2(&p.contour) > 0.0);
        assert!(
            !survives,
            "sub-epsilon feature must not survive triple offset"
        );
    }

    #[test]
    fn remove_colinear_ring_drops_a_midpoint_on_a_straight_edge() {
        let poly = Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 500, y: 0 },
                Point2 { x: 1000, y: 0 },
                Point2 { x: 1000, y: 1000 },
            ],
        };
        let out = remove_colinear_ring(&poly, 0.005);
        assert_eq!(
            out.points.len(),
            3,
            "midpoint on straight edge must be dropped"
        );
        assert!(!out.points.contains(&Point2 { x: 500, y: 0 }));
    }

    #[test]
    fn dedup_ring_removes_closing_duplicate() {
        let poly = Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 100, y: 0 },
                Point2 { x: 100, y: 100 },
                Point2 { x: 0, y: 0 },
            ],
        };
        let out = dedup_ring(&poly);
        assert_eq!(out.points.len(), 3);
    }
}
