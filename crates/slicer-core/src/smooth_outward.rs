// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/ExPolygon.cpp / Polygon.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------
//! Outward-only corner smoothing (`smooth_outward`).
//!
//! Port of canonical `smooth_outward` (`ExPolygon.cpp` / `Polygon.cpp`), the
//! regularization pass OrcaSlicer applies inside
//! `SupportMaterial.cpp::generate_interface_layers` (paired there with a
//! `closing` — see [`crate::polygon_ops::closing_ex`]).
//!
//! **Contract.** Every vertex where the contour turns *inward* (a reflex /
//! concave corner, measured through the material) is replaced by an outward
//! arc, so the emitted region is a **superset** of the input: the boundary
//! only ever moves into void, never into material. That one-sidedness is the
//! whole point of the canonical helper — interface regions may be regularized
//! but must never lose coverage under the part they support.
//!
//! **Not gated behind `host-algos`:** guest WASM modules call this.
//!
//! # Deviations from canonical
//!
//! There is no OrcaSlicer checkout on this machine; this is a port from the
//! documented behaviour of `smooth_outward`, not a line-by-line translation.
//! Known intentional differences:
//!
//! - Distances are taken in **millimetres** (`f64`), matching this crate's
//!   sibling helpers ([`crate::polygon_ops::opening`],
//!   [`crate::polygon_ops::closing_ex`]), rather than canonical's scaled
//!   `coord_t clip_dist_scaled`. Conversion uses [`slicer_ir::UNITS_PER_MM`]
//!   (1 unit = 100 nm, **not** OrcaSlicer's 1 nm — see
//!   `docs/08_coordinate_system.md`).
//! - The per-corner cut is capped at half of each adjacent edge, so two
//!   neighbouring concave corners can never eat past each other.
//! - Emitted integer points are validated against the two original edge
//!   half-planes with exact `i128` cross products, so coordinate rounding can
//!   never push the boundary back into the material. A corner whose cut
//!   endpoints cannot be placed on the lattice outward-safely is left
//!   unsmoothed.

use slicer_ir::{ExPolygon, Point2, Polygon};

/// Corners flatter than this deviation from straight (radians) are left alone:
/// smoothing them would add vertices without changing the region measurably,
/// and the bisector used to place the arc centre degenerates as the corner
/// approaches straight.
const MIN_CONCAVE_TURN_RAD: f64 = 0.05;

/// Cuts shorter than this (in scaled units, 1 unit = 100 nm) are dropped: the
/// resulting arc would be sub-lattice noise.
const MIN_CUT_UNITS: f64 = 2.0;

/// Arc resolution used when a caller has no reason to pick its own.
pub const DEFAULT_SMOOTHING_SEGMENTS: u32 = 4;

/// Smooths every concave corner of a set of [`ExPolygon`]s outward.
///
/// `smoothing_distance` is the maximum distance (mm) a cut may travel back
/// along either edge adjacent to a corner. `segments` is the arc resolution:
/// `0` or `1` produces a straight chamfer, `n >= 2` produces an `n`-segment
/// tessellated fillet arc tangent to both edges.
///
/// Each returned region contains its corresponding input region.
pub fn smooth_outward(
    subject: &[ExPolygon],
    smoothing_distance: f64,
    segments: u32,
) -> Vec<ExPolygon> {
    subject
        .iter()
        .map(|e| smooth_outward_ex(e, smoothing_distance, segments))
        .collect()
}

/// Smooths every concave corner of a single [`ExPolygon`] outward.
///
/// Both the outer contour and every hole are processed: a hole's corners that
/// are concave *through the material* (convex when viewed from inside the
/// hole) are cut away, which shrinks the hole and therefore still grows the
/// region. See [`smooth_outward`] for the parameters.
pub fn smooth_outward_ex(subject: &ExPolygon, smoothing_distance: f64, segments: u32) -> ExPolygon {
    ExPolygon {
        contour: smooth_ring(&subject.contour, smoothing_distance, segments, true),
        holes: subject
            .holes
            .iter()
            .map(|h| smooth_ring(h, smoothing_distance, segments, false))
            .collect(),
    }
}

/// Smooths every concave corner of a bare [`Polygon`] outward, treating it as
/// a solid contour (material inside). See [`smooth_outward`] for the
/// parameters.
pub fn smooth_outward_polygon(subject: &Polygon, smoothing_distance: f64, segments: u32) -> Polygon {
    smooth_ring(subject, smoothing_distance, segments, true)
}

/// Signed shoelace area of a ring; positive for CCW winding.
fn signed_area(points: &[Point2]) -> f64 {
    if points.len() < 3 {
        return 0.0;
    }
    let mut acc: i128 = 0;
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        acc += points[i].x as i128 * points[j].y as i128 - points[j].x as i128 * points[i].y as i128;
    }
    acc as f64 * 0.5
}

/// Exact 2D cross product `(b - a) x (p - a)` in `i128`.
fn cross_i128(a: Point2, b: Point2, p: Point2) -> i128 {
    (b.x as i128 - a.x as i128) * (p.y as i128 - a.y as i128)
        - (b.y as i128 - a.y as i128) * (p.x as i128 - a.x as i128)
}

/// Core per-ring pass.
///
/// `is_contour` selects the ring's semantic role, which together with the
/// ring's actual winding fixes which side the material is on. Under the
/// Slic3r convention (contour CCW, holes CW) the material is on the left of
/// travel for either role, and a concave corner is a right turn; the `sense`
/// factor below restores that invariant for rings that arrive wound the other
/// way instead of silently shrinking them.
fn smooth_ring(ring: &Polygon, smoothing_distance: f64, segments: u32, is_contour: bool) -> Polygon {
    let pts = &ring.points;
    let n = pts.len();
    if n < 3 || smoothing_distance <= 0.0 {
        return ring.clone();
    }
    let area = signed_area(pts);
    if area == 0.0 {
        return ring.clone();
    }
    // Material on the left of travel <=> CCW contour, or CW hole.
    let material_on_left = if is_contour { area > 0.0 } else { area < 0.0 };
    // `sense * cross <= 0` <=> the point is on the void side of a directed edge.
    let sense: f64 = if material_on_left { 1.0 } else { -1.0 };

    let max_cut_units = smoothing_distance * slicer_ir::UNITS_PER_MM;
    let mut out: Vec<Point2> = Vec::with_capacity(n);

    for i in 0..n {
        let u = pts[(i + n - 1) % n];
        let v = pts[i];
        let w = pts[(i + 1) % n];
        match smooth_corner(u, v, w, sense, max_cut_units, segments) {
            Some(replacement) => out.extend(replacement),
            None => out.push(v),
        }
    }

    dedup_ring(&mut out);
    if out.len() < 3 {
        return ring.clone();
    }
    Polygon { points: out }
}

/// Replaces a single concave corner `u -> v -> w` with an outward arc.
///
/// Returns `None` when the corner is convex, degenerate, too flat, too small
/// to cut, or when no lattice placement of the cut endpoints is provably
/// outward-safe — in every one of those cases the caller keeps `v` verbatim.
fn smooth_corner(
    u: Point2,
    v: Point2,
    w: Point2,
    sense: f64,
    max_cut_units: f64,
    segments: u32,
) -> Option<Vec<Point2>> {
    let ein = ((v.x - u.x) as f64, (v.y - u.y) as f64);
    let eout = ((w.x - v.x) as f64, (w.y - v.y) as f64);
    let li = (ein.0 * ein.0 + ein.1 * ein.1).sqrt();
    let lo = (eout.0 * eout.0 + eout.1 * eout.1).sqrt();
    if li <= 0.0 || lo <= 0.0 {
        return None;
    }

    let cross = ein.0 * eout.1 - ein.1 * eout.0;
    let dot = ein.0 * eout.0 + ein.1 * eout.1;
    // Concave (turning into the material) only: a right turn under the
    // material-on-left convention.
    if sense * cross >= 0.0 {
        return None;
    }
    // Deviation from straight; also the arc's central angle.
    let deviation = cross.abs().atan2(dot);
    if deviation < MIN_CONCAVE_TURN_RAD {
        return None;
    }

    // Never eat more than half of either adjacent edge, so neighbouring
    // concave corners cannot cut past one another.
    let t = max_cut_units.min(li * 0.5).min(lo * 0.5);
    if t < MIN_CUT_UNITS {
        return None;
    }

    // Unit directions from `v` back to `u` and forward to `w`. The cut
    // endpoints sit `t` along each.
    let du = (-ein.0 / li, -ein.1 / li);
    let dw = (eout.0 / lo, eout.1 / lo);
    let a_ideal = (v.x as f64 + du.0 * t, v.y as f64 + du.1 * t);
    let b_ideal = (v.x as f64 + dw.0 * t, v.y as f64 + dw.1 * t);

    let a = place_outward(a_ideal, u, v, w, sense)?;
    let b = place_outward(b_ideal, u, v, w, sense)?;

    let mut result = Vec::with_capacity(segments as usize + 1);
    result.push(a);

    if segments >= 2 {
        // Fillet tangent to both edges at `a` and `b`. Half the void wedge
        // angle at `v` is `(PI - deviation) / 2`; the centre lies on the
        // bisector at `t / cos(half)`, with radius `t * tan(half)`.
        let half = (std::f64::consts::PI - deviation) * 0.5;
        let cos_half = half.cos();
        let bis = (du.0 + dw.0, du.1 + dw.1);
        let bis_len = (bis.0 * bis.0 + bis.1 * bis.1).sqrt();
        if cos_half.abs() > 1e-9 && bis_len > 1e-9 {
            let bis = (bis.0 / bis_len, bis.1 / bis_len);
            let centre_dist = t / cos_half;
            let c = (
                v.x as f64 + bis.0 * centre_dist,
                v.y as f64 + bis.1 * centre_dist,
            );
            let r = t * half.tan();
            let ang_a = (a_ideal.1 - c.1).atan2(a_ideal.0 - c.0);
            // Sweep the short way (through the bisector) by `deviation`. The
            // travel direction a -> b is the turn direction of the corner.
            let sweep = if cross < 0.0 { -deviation } else { deviation };
            for k in 1..segments {
                let ang = ang_a + sweep * (k as f64) / (segments as f64);
                let ideal = (c.0 + r * ang.cos(), c.1 + r * ang.sin());
                // An arc point that cannot be placed outward-safely is simply
                // dropped: the void wedge is convex, so the chord between its
                // surviving neighbours is still outward-safe.
                if let Some(p) = place_outward(ideal, u, v, w, sense) {
                    result.push(p);
                }
            }
        }
    }

    result.push(b);
    Some(result)
}

/// Rounds `ideal` to a lattice point that provably lies on the void side of
/// both directed edges `u->v` and `v->w`.
///
/// Nearest-integer rounding alone can push a point across an edge line, which
/// would move the boundary *into* the material and break the outward-only
/// contract. All four rounding combinations are tried in order of distance
/// from `ideal`; `None` means none of them is safe.
fn place_outward(ideal: (f64, f64), u: Point2, v: Point2, w: Point2, sense: f64) -> Option<Point2> {
    let (fx, fy) = (ideal.0.floor(), ideal.1.floor());
    let mut candidates = [
        (fx, fy),
        (fx + 1.0, fy),
        (fx, fy + 1.0),
        (fx + 1.0, fy + 1.0),
    ];
    candidates.sort_by(|p, q| {
        let dp = (p.0 - ideal.0).powi(2) + (p.1 - ideal.1).powi(2);
        let dq = (q.0 - ideal.0).powi(2) + (q.1 - ideal.1).powi(2);
        dp.partial_cmp(&dq).unwrap_or(std::cmp::Ordering::Equal)
    });
    for (cx, cy) in candidates {
        let p = Point2 {
            x: cx as i64,
            y: cy as i64,
        };
        let in_side = cross_i128(u, v, p) as f64 * sense;
        let out_side = cross_i128(v, w, p) as f64 * sense;
        if in_side <= 0.0 && out_side <= 0.0 {
            return Some(p);
        }
    }
    None
}

/// Drops consecutive duplicate points, including the wrap-around pair.
fn dedup_ring(points: &mut Vec<Point2>) {
    points.dedup();
    while points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::polygon_ops::difference_ex;

    /// 1 mm in scaled units (1 unit = 100 nm).
    const MM: i64 = 10_000;

    fn poly(pts: &[(i64, i64)]) -> Polygon {
        Polygon {
            points: pts.iter().map(|&(x, y)| Point2 { x, y }).collect(),
        }
    }

    fn ex(contour: &[(i64, i64)], holes: &[&[(i64, i64)]]) -> ExPolygon {
        ExPolygon {
            contour: poly(contour),
            holes: holes.iter().map(|h| poly(h)).collect(),
        }
    }

    /// 10 mm x 10 mm L-shape, CCW, with exactly one reflex vertex at
    /// (5 mm, 5 mm).
    fn l_shape() -> ExPolygon {
        ex(
            &[
                (0, 0),
                (10 * MM, 0),
                (10 * MM, 5 * MM),
                (5 * MM, 5 * MM),
                (5 * MM, 10 * MM),
                (0, 10 * MM),
            ],
            &[],
        )
    }

    fn total_area(polys: &[ExPolygon]) -> f64 {
        polys
            .iter()
            .map(|e| {
                signed_area(&e.contour.points).abs()
                    - e.holes
                        .iter()
                        .map(|h| signed_area(&h.points).abs())
                        .sum::<f64>()
            })
            .sum()
    }

    /// The contract that makes `smooth_outward` safe for interface
    /// regularization: the boundary may move into void but never into
    /// material, so `input - output` must be empty. A single inward-moving
    /// vertex — including one produced only by coordinate rounding — leaves a
    /// residue here.
    #[test]
    fn output_strictly_contains_input() {
        // Concave (L-shape and a hole with material-side-concave corners),
        // convex, and a many-notch star, at several smoothing distances.
        let star = ex(
            &[
                (0, 0),
                (4 * MM, 0),
                (5 * MM, 2 * MM),
                (6 * MM, 0),
                (10 * MM, 0),
                (10 * MM, 10 * MM),
                (6 * MM, 10 * MM),
                (5 * MM, 8 * MM),
                (4 * MM, 10 * MM),
                (0, 10 * MM),
            ],
            &[],
        );
        // 20 mm square with a CW diamond hole: every hole corner is concave
        // through the material.
        let holed = ex(
            &[(0, 0), (20 * MM, 0), (20 * MM, 20 * MM), (0, 20 * MM)],
            &[&[
                (10 * MM, 5 * MM),
                (5 * MM, 10 * MM),
                (10 * MM, 15 * MM),
                (15 * MM, 10 * MM),
            ]],
        );
        let square = ex(
            &[(0, 0), (10 * MM, 0), (10 * MM, 10 * MM), (0, 10 * MM)],
            &[],
        );

        for input in [l_shape(), star, holed, square] {
            for distance in [0.05_f64, 0.3, 1.0, 4.0] {
                for segments in [0_u32, 1, 2, 5, 16] {
                    let out = smooth_outward_ex(&input, distance, segments);
                    let lost = difference_ex(std::slice::from_ref(&input), &[out.clone()]);
                    assert!(
                        total_area(&lost) == 0.0,
                        "smooth_outward moved the boundary INWARD \
                         (d={distance} mm, segments={segments}): lost {} units^2 \
                         across {} residue polygon(s)",
                        total_area(&lost),
                        lost.len()
                    );
                    assert!(
                        total_area(&[out]) >= total_area(&[input.clone()]),
                        "region shrank (d={distance} mm, segments={segments})"
                    );
                }
            }
        }
    }

    /// The reflex corner of an L-shape is replaced by an outward arc whose
    /// every point lies within `smoothing_distance` of the original corner,
    /// and the corner vertex itself is gone.
    #[test]
    fn sharp_concave_notch_is_smoothed_within_smoothing_distance() {
        let distance_mm = 0.5_f64;
        let corner = Point2 {
            x: 5 * MM,
            y: 5 * MM,
        };
        let out = smooth_outward_ex(&l_shape(), distance_mm, 8);
        let pts = &out.contour.points;

        assert!(
            !pts.contains(&corner),
            "the reflex vertex at (5mm, 5mm) must be replaced, got {pts:?}"
        );
        // 8-segment arc => 9 replacement points for the one reflex corner;
        // the 5 convex corners are untouched.
        assert!(
            pts.len() > l_shape().contour.points.len(),
            "smoothing must add vertices, got {} vs {}",
            pts.len(),
            l_shape().contour.points.len()
        );

        // Budget: `smoothing_distance` along each edge, plus one unit of
        // lattice rounding. Nothing may move further from the corner.
        let budget = distance_mm * slicer_ir::UNITS_PER_MM + 1.0;
        let moved: Vec<&Point2> = pts
            .iter()
            .filter(|p| !l_shape().contour.points.contains(p))
            .collect();
        assert!(
            !moved.is_empty(),
            "expected new arc vertices near the reflex corner"
        );
        for p in &moved {
            let d = (((p.x - corner.x) as f64).powi(2) + ((p.y - corner.y) as f64).powi(2)).sqrt();
            assert!(
                d <= budget,
                "arc vertex {p:?} is {d} units from the corner, over the \
                 {budget}-unit smoothing budget"
            );
        }
    }

    /// A purely convex ring has no concave corners, so it must come back
    /// byte-for-byte identical: `smooth_outward` regularizes notches, it does
    /// not resample contours.
    #[test]
    fn convex_ring_is_untouched() {
        let square = poly(&[(0, 0), (10 * MM, 0), (10 * MM, 10 * MM), (0, 10 * MM)]);
        assert_eq!(smooth_outward_polygon(&square, 1.0, 8), square);
    }

    /// Degenerate inputs pass through instead of collapsing.
    #[test]
    fn degenerate_rings_pass_through() {
        let two_pt = poly(&[(0, 0), (MM, 0)]);
        assert_eq!(smooth_outward_polygon(&two_pt, 1.0, 4), two_pt);
        let zero_area = poly(&[(0, 0), (MM, 0), (2 * MM, 0)]);
        assert_eq!(smooth_outward_polygon(&zero_area, 1.0, 4), zero_area);
        // Non-positive distance is a no-op.
        assert_eq!(
            smooth_outward_polygon(&l_shape().contour, 0.0, 4),
            l_shape().contour
        );
    }

    /// A ring wound the wrong way round for its role must still grow the
    /// region, not shrink it: the material side is derived from role plus
    /// actual winding, never assumed.
    #[test]
    fn reversed_winding_still_grows_the_region() {
        let mut reversed = l_shape();
        reversed.contour.points.reverse();
        let out = smooth_outward_ex(&reversed, 0.5, 6);
        assert!(
            total_area(&[out]) > total_area(&[reversed]),
            "a CW contour must still be smoothed outward"
        );
    }

    /// `segments <= 1` is the straight-chamfer path; `segments >= 2`
    /// tessellates an arc and must therefore emit strictly more vertices.
    #[test]
    fn segments_controls_arc_resolution() {
        let chamfer = smooth_outward_ex(&l_shape(), 0.5, 1).contour.points.len();
        let arc = smooth_outward_ex(&l_shape(), 0.5, 8).contour.points.len();
        assert!(
            arc > chamfer,
            "8-segment arc ({arc} pts) must be finer than the chamfer ({chamfer} pts)"
        );
    }
}
