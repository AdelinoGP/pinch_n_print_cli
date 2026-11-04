// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/ClipperUtils.cpp,
// src/libslic3r/ClipperUtils.hpp, src/libslic3r/Polygon.cpp,
// src/libslic3r/ExPolygon.cpp
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
// -----------------------------------------------------------------------------

//! Portable polygon primitives for the isolated Lightning feasibility kernel.
//!
//! Unit discipline: every length is an IR integer unit where **1 unit = 100 nm**
//! (`10_000` units per millimetre). OrcaSlicer works in 1 nm units, so every
//! canonical length constant crosses this module's boundary divided by `100`
//! (see `CANONICAL_NM_PER_UNIT`). Ported canonical constants are named with a
//! `_UNITS` suffix so the conversion is visible at every use site.
//!
//! Ring normalization follows the recorder's documented convention (the
//! `ring_normalization` string in the recorded provenance bundle):
//! contours counter-clockwise (positive shoelace area), holes clockwise; rings
//! rotated so the lexicographically smallest `(x, y)` point is first; duplicated
//! closing and consecutive duplicate points dropped; `ring_id` 0 = contour and
//! `1..n` = holes; `segment_id` `0..n-1` in normalized order.
//!
//! Boolean/offset helpers wrap `clipper2-rust` and mirror the canonical defaults
//! of `ClipperUtils`: `jtMiter` with miter limit `3.0`, fill rule `NonZero`, and
//! the `ClipperSafetyOffset` of `10` canonical nm (= `0.1` IR units) used by
//! `union_safety_offset`.

use clipper2_rust::{
    boolean_op_tree_64, inflate_paths_64, ClipType, EndType, FillRule, JoinType, Path64, Paths64,
    Point64, PolyTree64,
};
use slicer_ir::{ExPolygon, Point2, Polygon};

use crate::model::Rational;

/// IR units per millimetre (`1 unit = 100 nm`).
pub const UNITS_PER_MM: f64 = 10_000.0;

/// Canonical nm represented by one IR unit. Canonical constants in nm are
/// divided by this value at the module boundary.
pub const CANONICAL_NM_PER_UNIT: f64 = 100.0;

/// `ClipperUtils::ClipperSafetyOffset` (canonical 10 nm) in IR units.
pub const CLIPPER_SAFETY_OFFSET_UNITS: f64 = 10.0 / CANONICAL_NM_PER_UNIT;

/// `SCALED_EPSILON` (`scale_(EPSILON)` with `EPSILON = 1e-4` mm) in IR units.
pub const SCALED_EPSILON_UNITS: f64 = 1.0;

/// Canonical `DefaultMiterLimit` (`ClipperUtils`).
pub const DEFAULT_MITER_LIMIT: f64 = 3.0;

/// Convert millimetres to IR integer units (`coord_t` truncation).
#[inline]
pub fn mm_to_units(mm: f64) -> i64 {
    (mm * UNITS_PER_MM) as i64
}

/// Convert IR integer units to millimetres.
#[inline]
pub fn units_to_mm(units: i64) -> f64 {
    units as f64 / UNITS_PER_MM
}

/// Shoelace signed area of a closed ring, in units². Positive means CCW.
pub fn signed_area(ring: &[Point2]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }
    let mut acc: i128 = 0;
    let n = ring.len();
    for i in 0..n {
        let a = ring[i];
        let b = ring[(i + 1) % n];
        acc += a.x as i128 * b.y as i128 - b.x as i128 * a.y as i128;
    }
    acc as f64 / 2.0
}

/// True when the ring winds counter-clockwise (positive shoelace area).
pub fn is_ccw(ring: &[Point2]) -> bool {
    signed_area(ring) > 0.0
}

/// Area of one expolygon (contour minus holes) in units².
pub fn expolygon_area(poly: &ExPolygon) -> f64 {
    let mut acc = signed_area(&poly.contour.points).abs();
    for hole in &poly.holes {
        acc -= signed_area(&hole.points).abs();
    }
    acc.max(0.0)
}

/// Total area of a set of disjoint expolygons in units².
pub fn area(polys: &[ExPolygon]) -> f64 {
    polys.iter().map(expolygon_area).sum()
}

/// Drop duplicated closing point and consecutive duplicates, preserving order.
pub fn dedupe_ring(ring: &[Point2]) -> Vec<Point2> {
    let mut out: Vec<Point2> = Vec::with_capacity(ring.len());
    for &p in ring {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    while out.len() > 1 && out.first() == out.last() {
        out.pop();
    }
    out
}

/// Lexicographically smallest `(x, y)` vertex index, or `None` for an empty ring.
pub fn lexicographic_min_index(ring: &[Point2]) -> Option<usize> {
    ring.iter()
        .enumerate()
        .min_by_key(|(_, p)| (p.x, p.y))
        .map(|(i, _)| i)
}

/// Orient, rotate and dedupe a ring into canonical form.
///
/// `want_ccw` selects the winding (contour = `true`, hole = `false`). The first
/// vertex is the lexicographically smallest `(x, y)` point of the oriented ring.
pub fn normalize_ring(ring: &[Point2], want_ccw: bool) -> Vec<Point2> {
    let mut points = dedupe_ring(ring);
    if points.len() < 3 {
        return points;
    }
    if is_ccw(&points) != want_ccw {
        points.reverse();
    }
    let start = lexicographic_min_index(&points).unwrap_or(0);
    points.rotate_left(start);
    points
}

/// Canonicalize one expolygon: CCW contour, CW holes, deterministic hole order.
pub fn normalize_expolygon(poly: &ExPolygon) -> ExPolygon {
    let contour = normalize_ring(&poly.contour.points, true);
    let mut holes: Vec<Vec<Point2>> = poly
        .holes
        .iter()
        .map(|h| normalize_ring(&h.points, false))
        .collect();
    holes.sort();
    ExPolygon {
        contour: Polygon { points: contour },
        holes: holes.into_iter().map(|points| Polygon { points }).collect(),
    }
}

/// Canonicalize a set of expolygons and put them in a deterministic total order.
///
/// The order key is the bounding box of the whole expolygon followed by the
/// normalized rings, so equal geometry always serializes identically.
pub fn normalize_expolygons(polys: &[ExPolygon]) -> Vec<ExPolygon> {
    let mut normalized: Vec<ExPolygon> = polys.iter().map(normalize_expolygon).collect();
    normalized.sort_by_key(expolygon_order_key);
    normalized
}

fn expolygon_order_key(poly: &ExPolygon) -> (i64, i64, i64, i64, usize, Vec<Point2>) {
    let bbox =
        ring_bbox(&poly.contour.points).unwrap_or((Point2 { x: 0, y: 0 }, Point2 { x: 0, y: 0 }));
    (
        bbox.0.x,
        bbox.0.y,
        bbox.1.x,
        bbox.1.y,
        poly.holes.len(),
        poly.contour.points.clone(),
    )
}

/// Bounding box `(min, max)` of a ring.
pub fn ring_bbox(ring: &[Point2]) -> Option<(Point2, Point2)> {
    let first = *ring.first()?;
    let mut min = first;
    let mut max = first;
    for &p in ring.iter().skip(1) {
        min.x = min.x.min(p.x);
        min.y = min.y.min(p.y);
        max.x = max.x.max(p.x);
        max.y = max.y.max(p.y);
    }
    Some((min, max))
}

/// Bounding box `(min, max)` over a whole expolygon set.
pub fn bounding_box(polys: &[ExPolygon]) -> Option<(Point2, Point2)> {
    let mut acc: Option<(Point2, Point2)> = None;
    for poly in polys {
        for ring in std::iter::once(&poly.contour).chain(poly.holes.iter()) {
            if let Some((min, max)) = ring_bbox(&ring.points) {
                acc = Some(match acc {
                    None => (min, max),
                    Some((amin, amax)) => (
                        Point2 {
                            x: amin.x.min(min.x),
                            y: amin.y.min(min.y),
                        },
                        Point2 {
                            x: amax.x.max(max.x),
                            y: amax.y.max(max.y),
                        },
                    ),
                });
            }
        }
    }
    acc
}

/// Even-odd ray-cast point-in-ring test, exact in integer coordinates.
///
/// Points exactly on an edge are reported as inside only when the ray cast
/// happens to keep them so; callers must not depend on boundary classification.
pub fn point_in_ring(pt: Point2, ring: &[Point2]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let a = ring[i];
        let b = ring[j];
        if (a.y > pt.y) != (b.y > pt.y) {
            let dy = (b.y - a.y) as f64;
            if dy != 0.0 {
                let x_cross = a.x as f64 + (pt.y - a.y) as f64 * (b.x - a.x) as f64 / dy;
                if (pt.x as f64) < x_cross {
                    inside = !inside;
                }
            }
        }
        j = i;
    }
    inside
}

/// Point-in-expolygon: inside the contour and outside every hole.
pub fn point_in_expolygon(pt: Point2, poly: &ExPolygon) -> bool {
    if !point_in_ring(pt, &poly.contour.points) {
        return false;
    }
    for hole in &poly.holes {
        if point_in_ring(pt, &hole.points) {
            return false;
        }
    }
    true
}

/// True when `inner` lies inside `outer` (first vertex + a mid-edge probe).
pub fn ring_contains_ring(outer: &[Point2], inner: &[Point2]) -> bool {
    let probe = match (inner.first(), inner.get(1)) {
        (Some(a), Some(b)) => Point2 {
            x: a.x + (b.x - a.x) / 2,
            y: a.y + (b.y - a.y) / 2,
        },
        (Some(a), None) => *a,
        _ => return false,
    };
    point_in_ring(probe, outer)
}

/// Exact rational interval `[start, end]` (inclusive, ascending) covered by
/// `edge` when projected onto `source`'s parametrization.
///
/// The parameter is expressed along the source segment, so a complete
/// coincidence yields `[0/1, 1/1]` regardless of the two directions. Returns
/// `None` when the two segments are not collinear or do not overlap with
/// positive length.
pub fn segment_overlap(edge: [Point2; 2], source: [Point2; 2]) -> Option<(Rational, Rational)> {
    let dx = source[1].x - source[0].x;
    let dy = source[1].y - source[0].y;
    let len_sq = dx as i128 * dx as i128 + dy as i128 * dy as i128;
    if len_sq == 0 {
        return None;
    }
    let cross = |p: Point2| -> i128 {
        let px = (p.x - source[0].x) as i128;
        let py = (p.y - source[0].y) as i128;
        dx as i128 * py - dy as i128 * px
    };
    if cross(edge[0]) != 0 || cross(edge[1]) != 0 {
        return None;
    }
    let project = |p: Point2| -> i128 {
        let px = (p.x - source[0].x) as i128;
        let py = (p.y - source[0].y) as i128;
        px * dx as i128 + py * dy as i128
    };
    let (t0, t1) = (project(edge[0]), project(edge[1]));
    let lo = t0.min(t1).max(0);
    let hi = t0.max(t1).min(len_sq);
    if hi <= lo {
        return None;
    }
    Some((
        reduce_interval_bound(lo, len_sq),
        reduce_interval_bound(hi, len_sq),
    ))
}

/// Reduce `numerator / denominator` to lowest terms with a positive denominator.
fn reduce_interval_bound(numerator: i128, denominator: i128) -> Rational {
    let mut num = numerator;
    let mut den = denominator;
    let mut a = num.unsigned_abs();
    let mut b = den.unsigned_abs();
    while b != 0 {
        let t = a % b;
        a = b;
        b = t;
    }
    let g = a;
    if g > 1 {
        num /= g as i128;
        den /= g as i128;
    }
    if den < 0 {
        num = -num;
        den = -den;
    }
    Rational {
        numerator: num as i64,
        denominator: den as u64,
    }
}

// -----------------------------------------------------------------------------
// Clipper2-backed boolean and offset helpers
// -----------------------------------------------------------------------------

fn to_paths(polys: &[ExPolygon]) -> Paths64 {
    let mut paths: Paths64 = Vec::new();
    for poly in polys {
        for ring in std::iter::once(&poly.contour).chain(poly.holes.iter()) {
            if ring.points.len() < 3 {
                continue;
            }
            let path: Path64 = ring.points.iter().map(|p| Point64::new(p.x, p.y)).collect();
            paths.push(path);
        }
    }
    paths
}

fn from_path(path: &Path64) -> Vec<Point2> {
    path.iter().map(|p| Point2 { x: p.x, y: p.y }).collect()
}

/// Collapse a Clipper2 `PolyTree64` into expolygons, keeping hole nesting.
fn from_poly_tree(tree: &PolyTree64) -> Vec<ExPolygon> {
    fn ring(path: &Path64) -> Polygon {
        Polygon {
            points: from_path(path),
        }
    }
    fn walk(tree: &PolyTree64, node: usize, out: &mut Vec<ExPolygon>) {
        for &child in tree.nodes[node].children() {
            let contour = ring(tree.nodes[child].polygon());
            let mut holes = Vec::new();
            for &hole in tree.nodes[child].children() {
                holes.push(ring(tree.nodes[hole].polygon()));
                // An island inside a hole is a new outer contour.
                walk(tree, hole, out);
            }
            out.push(ExPolygon { contour, holes });
        }
    }
    let mut out = Vec::new();
    walk(tree, 0, &mut out);
    out
}

fn boolean(kind: ClipType, subjects: &Paths64, clips: &Paths64) -> Vec<ExPolygon> {
    if subjects.is_empty() {
        return Vec::new();
    }
    let mut tree = PolyTree64::new();
    boolean_op_tree_64(kind, FillRule::NonZero, subjects, clips, &mut tree);
    normalize_expolygons(&from_poly_tree(&tree))
}

/// Union of a set of expolygons.
pub fn union(polys: &[ExPolygon]) -> Vec<ExPolygon> {
    let paths = to_paths(polys);
    boolean(ClipType::Union, &paths, &Paths64::new())
}

/// Intersection of two expolygon sets.
pub fn intersection(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    let subjects = to_paths(subject);
    let clips = to_paths(clip);
    boolean(ClipType::Intersection, &subjects, &clips)
}

/// Difference `subject - clip`.
pub fn difference(subject: &[ExPolygon], clip: &[ExPolygon]) -> Vec<ExPolygon> {
    let subjects = to_paths(subject);
    let clips = to_paths(clip);
    boolean(ClipType::Difference, &subjects, &clips)
}

/// Offset (canonical `ClipperUtils::offset`) with the canonical default join.
///
/// A positive `delta_units` dilates the shape and closes holes; a negative one
/// erodes it. Input rings are normalized first so Clipper2 sees canonical
/// winding (CCW contour, CW holes).
pub fn offset(polys: &[ExPolygon], delta_units: f64) -> Vec<ExPolygon> {
    if polys.is_empty() {
        return Vec::new();
    }
    let normalized = normalize_expolygons(polys);
    let paths = to_paths(&normalized);
    if paths.is_empty() {
        return Vec::new();
    }
    let inflated = inflate_paths_64(
        &paths,
        delta_units,
        JoinType::Miter,
        EndType::Polygon,
        DEFAULT_MITER_LIMIT,
        0.0,
    );
    boolean(ClipType::Union, &inflated, &Paths64::new())
}

/// Canonical `expand`: positive offset, asserted positive by the caller.
pub fn expand(polys: &[ExPolygon], delta_units: f64) -> Vec<ExPolygon> {
    if delta_units <= 0.0 {
        return normalize_expolygons(polys);
    }
    offset(polys, delta_units)
}

/// Canonical `shrink`: negative offset.
pub fn shrink(polys: &[ExPolygon], delta_units: f64) -> Vec<ExPolygon> {
    if delta_units <= 0.0 {
        return normalize_expolygons(polys);
    }
    offset(polys, -delta_units)
}

/// Canonical `closing_ex(polys, delta)`: dilate then erode by `delta`.
pub fn closing(polys: &[ExPolygon], delta_units: f64) -> Vec<ExPolygon> {
    if delta_units <= 0.0 {
        return normalize_expolygons(polys);
    }
    let dilated = offset(polys, delta_units);
    offset(&dilated, -delta_units)
}

/// Canonical `union_safety_offset_ex`: union plus the Clipper safety offset.
pub fn union_safety_offset(polys: &[ExPolygon]) -> Vec<ExPolygon> {
    offset(polys, CLIPPER_SAFETY_OFFSET_UNITS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
        ExPolygon {
            contour: Polygon {
                points: vec![
                    Point2 { x: x0, y: y0 },
                    Point2 { x: x1, y: y0 },
                    Point2 { x: x1, y: y1 },
                    Point2 { x: x0, y: y1 },
                ],
            },
            holes: Vec::new(),
        }
    }

    #[test]
    fn signed_area_is_positive_ccw_and_negative_cw() {
        let ccw = vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: 4000, y: 0 },
            Point2 { x: 4000, y: 3000 },
            Point2 { x: 0, y: 3000 },
        ];
        assert_eq!(signed_area(&ccw), 12_000_000.0);
        let mut cw = ccw.clone();
        cw.reverse();
        assert_eq!(signed_area(&cw), -12_000_000.0);
        assert!(is_ccw(&ccw));
        assert!(!is_ccw(&cw));
    }

    #[test]
    fn normalize_rotates_to_lexicographic_min_and_fixes_winding() {
        // Same square, entered from a different vertex, wound clockwise.
        let raw = vec![
            Point2 { x: 4000, y: 3000 },
            Point2 { x: 4000, y: 0 },
            Point2 { x: 0, y: 0 },
            Point2 { x: 0, y: 3000 },
            Point2 { x: 4000, y: 3000 },
        ];
        let norm = normalize_ring(&raw, true);
        assert_eq!(norm.len(), 4, "closing duplicate must be dropped");
        assert_eq!(norm[0], Point2 { x: 0, y: 0 });
        assert!(is_ccw(&norm));
        // Idempotent, and rotation invariance: every cyclic rotation of the
        // same ring normalizes to the same sequence.
        assert_eq!(normalize_ring(&norm, true), norm);
        for shift in 0..4 {
            let mut rotated = norm.clone();
            rotated.rotate_left(shift);
            assert_eq!(normalize_ring(&rotated, true), norm);
        }
    }

    #[test]
    fn normalize_preserves_holes_and_assigns_them_clockwise() {
        let mut poly = square(0, 0, 10_000, 10_000);
        poly.holes.push(Polygon {
            points: vec![
                Point2 { x: 2000, y: 2000 },
                Point2 { x: 8000, y: 2000 },
                Point2 { x: 8000, y: 8000 },
                Point2 { x: 2000, y: 8000 },
            ],
        });
        let norm = normalize_expolygon(&poly);
        assert_eq!(norm.holes.len(), 1);
        assert!(is_ccw(&norm.contour.points));
        assert!(!is_ccw(&norm.holes[0].points), "holes must wind CW");
        // Area is the square minus its hole, so the hole really was preserved.
        assert_eq!(expolygon_area(&norm), 100_000_000.0 - 36_000_000.0);
        assert_eq!(expolygon_area(&norm), 64_000_000.0);
    }

    #[test]
    fn point_in_expolygon_rejects_points_inside_holes() {
        let mut poly = square(0, 0, 10_000, 10_000);
        poly.holes.push(Polygon {
            points: vec![
                Point2 { x: 2000, y: 2000 },
                Point2 { x: 8000, y: 2000 },
                Point2 { x: 8000, y: 8000 },
                Point2 { x: 2000, y: 8000 },
            ],
        });
        assert!(point_in_expolygon(Point2 { x: 1000, y: 1000 }, &poly));
        assert!(!point_in_expolygon(Point2 { x: 5000, y: 5000 }, &poly));
        assert!(!point_in_expolygon(Point2 { x: 20_000, y: 5000 }, &poly));
    }

    #[test]
    fn expand_grows_area_and_closes_holes_shrink_erodes() {
        let mut poly = square(0, 0, 10_000, 10_000);
        poly.holes.push(Polygon {
            points: vec![
                Point2 { x: 2000, y: 2000 },
                Point2 { x: 8000, y: 2000 },
                Point2 { x: 8000, y: 8000 },
                Point2 { x: 2000, y: 8000 },
            ],
        });
        let base = expolygon_area(&poly);
        let grown = expand(std::slice::from_ref(&poly), 1000.0);
        assert!(
            area(&grown) > base,
            "expand must increase area: {} vs {}",
            area(&grown),
            base
        );
        let shrunk = shrink(std::slice::from_ref(&poly), 1000.0);
        assert!(
            area(&shrunk) < base,
            "shrink must decrease area: {} vs {}",
            area(&shrunk),
            base
        );
        // Eroding by more than the hole's half-width removes the hole.
        let sealed = expand(std::slice::from_ref(&poly), 5000.0);
        assert_eq!(sealed.len(), 1);
        assert!(
            sealed[0].holes.is_empty(),
            "dilation past the hole closes it"
        );
    }

    #[test]
    fn difference_cuts_a_hole_and_offset_respects_hole_winding() {
        let outer = square(0, 0, 10_000, 10_000);
        let cutter = square(3000, 3000, 7000, 7000);
        let cut = difference(std::slice::from_ref(&outer), std::slice::from_ref(&cutter));
        assert_eq!(cut.len(), 1);
        assert_eq!(cut[0].holes.len(), 1, "the cutter leaves a hole");
        assert_eq!(expolygon_area(&cut[0]), 100_000_000.0 - 16_000_000.0);
        // Shrinking the holed shape grows the hole (negative offset on a CW hole).
        let shrunk = shrink(&cut, 500.0);
        assert!(expolygon_area(&shrunk[0]) < expolygon_area(&cut[0]));
    }

    #[test]
    fn overlapping_union_merges_and_keeps_disjoint_parts() {
        let a = square(0, 0, 5000, 5000);
        let b = square(4000, 4000, 9000, 9000);
        let c = square(20_000, 20_000, 25_000, 25_000);
        let merged = union(&[a, b, c]);
        assert_eq!(merged.len(), 2);
        assert_eq!(
            area(&merged),
            25_000_000.0 + 25_000_000.0 - 1_000_000.0 + 25_000_000.0
        );
    }

    #[test]
    fn segment_overlap_reports_full_and_partial_intervals() {
        let source = [Point2 { x: 0, y: 0 }, Point2 { x: 4000, y: 0 }];
        // Same segment, reversed, still the full interval.
        let (start, end) = segment_overlap([source[1], source[0]], source).unwrap();
        assert_eq!((start.numerator, start.denominator), (0, 1));
        assert_eq!((end.numerator, end.denominator), (1, 1));
        // Half of it.
        let (start, end) =
            segment_overlap([Point2 { x: 2000, y: 0 }, Point2 { x: 4000, y: 0 }], source).unwrap();
        assert_eq!((start.numerator, start.denominator), (1, 2));
        assert_eq!((end.numerator, end.denominator), (1, 1));
        // Not collinear / not overlapping.
        assert!(
            segment_overlap([Point2 { x: 0, y: 1 }, Point2 { x: 4000, y: 1 }], source).is_none()
        );
        assert!(segment_overlap(
            [Point2 { x: 9000, y: 0 }, Point2 { x: 12_000, y: 0 }],
            source
        )
        .is_none());
    }

    #[test]
    fn mm_unit_boundary_matches_the_documented_ratio() {
        assert_eq!(mm_to_units(0.45), 4500);
        assert_eq!(units_to_mm(4500), 0.45);
        // Canonical 3 mm expansion constant becomes 30_000 IR units.
        assert_eq!(mm_to_units(3.0), 30_000);
        // The canonical 10 nm Clipper safety offset is a tenth of one IR unit.
        assert!((CLIPPER_SAFETY_OFFSET_UNITS - 0.1).abs() < 1e-12);
    }
}
