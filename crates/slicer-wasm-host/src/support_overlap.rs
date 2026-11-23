//! Exact positive-area overlap test for host support validation.
//!
//! The support gate rejects a body when `intersection(body, occupancy)` (Clipper2,
//! `FillRule::NonZero`) has positive area. That Clipper call dominated the
//! post-planner host tail even though almost every tested pair is disjoint.
//! [`PreparedOverlap::overlaps`] returns exactly what the Clipper test returns,
//! but proves the disjoint cases without running it:
//!
//! 1. **Strict bounding-box separation.** Every vertex Clipper2 can emit for an
//!    intersection lies inside both operands' bounding boxes. Clipper2 rounds an
//!    edge's scanline position (`TopX`) to an integer inside that edge's own
//!    x-range, so strictly separated boxes can never be ordered as crossing.
//!    The result is empty, and no Clipper call is needed.
//! 2. **Exact disjointness certificate** ([`certify_disjoint`]). All
//!    arithmetic is exact `i128` on integer coordinates. The certificate requires:
//!    - every edge of one operand stays at least [`BOUNDARY_MARGIN_UNITS`] from
//!      every edge of the other;
//!    - one vertex of every ring of each operand has winding number zero
//!      against the other operand's rings.
//!
//!    A ring that never meets the other boundary sees a constant winding
//!    number along its whole length, so the vertex test covers the ring. With
//!    both boundaries in each other's zero-winding region, the two NonZero fill
//!    sets are disjoint: a component of one set's closure avoids the other
//!    boundary and contains points of its own boundary, where the other winding
//!    is zero. The margin keeps Clipper2's integer `TopX` rounding (at most half
//!    a unit per edge) from ever ordering two such edges as crossing, so Clipper2
//!    sees no intersection and every winding it accumulates is the exact
//!    topological one. Its output is therefore empty too.
//! 3. Anything not proven falls back to the original Clipper2 test.
//!
//! Coordinates beyond [`MAX_CERTIFIED_COORD`] and rings with fewer than three
//! points are never certified; they take the Clipper2 path.

use std::cell::OnceCell;

use slicer_ir::{ExPolygon, Point2};

/// Minimum boundary separation, in workspace units (1 unit = 100 nm), below
/// which a pair is left to Clipper2. Production bodies keep the support gap
/// (hundreds of microns) from occupancy, so this only routes near-contacts to
/// the exact Clipper2 test.
pub(crate) const BOUNDARY_MARGIN_UNITS: i64 = 4;

/// Largest absolute coordinate the certificate accepts. With `|c| <= 2^30`,
/// coordinate differences fit in 31 bits, cross products in 63, and squared
/// cross products in 126 — all exact in `i128`.
pub(crate) const MAX_CERTIFIED_COORD: i64 = 1 << 30;

/// Edges per pruning chunk.
const CHUNK_EDGES: usize = 16;

/// Axis-aligned bounds `(min_x, max_x, min_y, max_y)`.
type Bounds = (i64, i64, i64, i64);

/// Overlap area, in square units, above which two support operands collide.
/// Zero: any positive area rejects. The certificate proves the intersection
/// empty, so it agrees with the Clipper2 test for every non-negative value.
pub(crate) const SUPPORT_OVERLAP_TOLERANCE: i64 = 0;

/// The original per-pair test: positive area of the Clipper2 intersection.
pub(crate) fn clipper_positive_overlap(a: &ExPolygon, b: &ExPolygon) -> bool {
    slicer_core::polygon_ops::intersection(std::slice::from_ref(a), std::slice::from_ref(b))
        .iter()
        .map(expolygon_area)
        .sum::<f64>()
        > SUPPORT_OVERLAP_TOLERANCE as f64
}

/// Signed-area magnitude of an `ExPolygon` (contour minus holes), as the
/// support gate has always measured overlap.
pub(crate) fn expolygon_area(poly: &ExPolygon) -> f64 {
    fn ring_area(points: &[Point2]) -> f64 {
        if points.len() < 3 {
            return 0.0;
        }
        points
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let b = &points[(i + 1) % points.len()];
                (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64)
            })
            .sum::<f64>()
            .abs()
            * 0.5
    }
    ring_area(&poly.contour.points) - poly.holes.iter().map(|h| ring_area(&h.points)).sum::<f64>()
}

/// Bounds over every point of every ring, degenerate rings included: a
/// superset of anything Clipper2 can fill from these paths.
fn full_bounds(poly: &ExPolygon) -> Option<Bounds> {
    std::iter::once(&poly.contour)
        .chain(poly.holes.iter())
        .flat_map(|ring| ring.points.iter())
        .fold(None, |acc: Option<Bounds>, p| {
            Some(match acc {
                None => (p.x, p.x, p.y, p.y),
                Some((x0, x1, y0, y1)) => (x0.min(p.x), x1.max(p.x), y0.min(p.y), y1.max(p.y)),
            })
        })
}

fn strictly_separated(a: Bounds, b: Bounds) -> bool {
    a.1 < b.0 || b.1 < a.0 || a.3 < b.2 || b.3 < a.2
}

/// One operand of the overlap test, with its bounds computed once and its
/// certificate structure built on first use.
pub(crate) struct PreparedOverlap<'a> {
    poly: &'a ExPolygon,
    bounds: Option<Bounds>,
    chunked: OnceCell<Option<ChunkedPoly>>,
}

impl<'a> PreparedOverlap<'a> {
    pub(crate) fn new(poly: &'a ExPolygon) -> Self {
        Self {
            poly,
            bounds: full_bounds(poly),
            chunked: OnceCell::new(),
        }
    }

    fn chunked(&self) -> Option<&ChunkedPoly> {
        self.chunked
            .get_or_init(|| ChunkedPoly::build(self.poly))
            .as_ref()
    }

    /// Exactly [`clipper_positive_overlap`]`(self, other)`.
    pub(crate) fn overlaps(&self, other: &PreparedOverlap<'_>) -> bool {
        if let (Some(a), Some(b)) = (self.bounds, other.bounds) {
            if strictly_separated(a, b) {
                return false;
            }
        }
        if let (Some(a), Some(b)) = (self.chunked(), other.chunked()) {
            if certify_disjoint(a, b) {
                return false;
            }
        }
        clipper_positive_overlap(self.poly, other.poly)
    }
}

#[derive(Clone, Copy)]
struct Seg {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    bounds: Bounds,
}

struct Chunk {
    bounds: Bounds,
    start: usize,
    end: usize,
}

/// Edges of every ring, in path order and orientation as Clipper2 receives
/// them, grouped into bounded chunks.
struct ChunkedPoly {
    bounds: Bounds,
    segs: Vec<Seg>,
    chunks: Vec<Chunk>,
    /// First vertex of every ring.
    ring_vertices: Vec<Point2>,
}

impl ChunkedPoly {
    fn build(poly: &ExPolygon) -> Option<Self> {
        let rings: Vec<&[Point2]> = std::iter::once(&poly.contour)
            .chain(poly.holes.iter())
            .map(|ring| ring.points.as_slice())
            .collect();
        if rings.iter().any(|ring| ring.len() < 3) {
            return None;
        }
        if rings
            .iter()
            .flat_map(|ring| ring.iter())
            .any(|p| p.x.abs() > MAX_CERTIFIED_COORD || p.y.abs() > MAX_CERTIFIED_COORD)
        {
            return None;
        }
        let mut segs = Vec::with_capacity(rings.iter().map(|ring| ring.len()).sum());
        let mut chunks = Vec::new();
        let mut ring_vertices = Vec::with_capacity(rings.len());
        for ring in rings {
            ring_vertices.push(ring[0]);
            let ring_start = segs.len();
            for (i, p0) in ring.iter().enumerate() {
                let p1 = ring[(i + 1) % ring.len()];
                segs.push(Seg {
                    x0: p0.x,
                    y0: p0.y,
                    x1: p1.x,
                    y1: p1.y,
                    bounds: (p0.x.min(p1.x), p0.x.max(p1.x), p0.y.min(p1.y), p0.y.max(p1.y)),
                });
            }
            let mut start = ring_start;
            while start < segs.len() {
                let end = (start + CHUNK_EDGES).min(segs.len());
                let bounds = union_bounds(segs[start..end].iter().map(|seg| seg.bounds))
                    .expect("chunk is non-empty");
                chunks.push(Chunk { bounds, start, end });
                start = end;
            }
        }
        let bounds = union_bounds(chunks.iter().map(|chunk| chunk.bounds))?;
        Some(Self {
            bounds,
            segs,
            chunks,
            ring_vertices,
        })
    }
}

fn union_bounds(mut items: impl Iterator<Item = Bounds>) -> Option<Bounds> {
    let first = items.next()?;
    Some(items.fold(first, |a, b| {
        (a.0.min(b.0), a.1.max(b.1), a.2.min(b.2), a.3.max(b.3))
    }))
}

/// `a` grown by `m` on every side.
fn inflate(a: Bounds, m: i64) -> Bounds {
    (a.0 - m, a.1 + m, a.2 - m, a.3 + m)
}

fn bounds_overlap(a: Bounds, b: Bounds) -> bool {
    a.0 <= b.1 && b.0 <= a.1 && a.2 <= b.3 && b.2 <= a.3
}

/// Exact: `true` only when the NonZero fill sets of `a` and `b` are disjoint
/// and their boundaries keep at least [`BOUNDARY_MARGIN_UNITS`] apart. See the
/// module documentation for the proof.
fn certify_disjoint(a: &ChunkedPoly, b: &ChunkedPoly) -> bool {
    let m = BOUNDARY_MARGIN_UNITS;
    let a_reach = inflate(a.bounds, m);
    let b_reach = inflate(b.bounds, m);
    // 1. Boundary separation.
    let b_chunks: Vec<&Chunk> = b
        .chunks
        .iter()
        .filter(|chunk| bounds_overlap(chunk.bounds, a_reach))
        .collect();
    if !b_chunks.is_empty() {
        for a_chunk in a.chunks.iter().filter(|chunk| bounds_overlap(chunk.bounds, b_reach)) {
            let a_chunk_reach = inflate(a_chunk.bounds, m);
            for b_chunk in &b_chunks {
                if !bounds_overlap(a_chunk_reach, b_chunk.bounds) {
                    continue;
                }
                for sa in &a.segs[a_chunk.start..a_chunk.end] {
                    let sa_reach = inflate(sa.bounds, m);
                    if !bounds_overlap(sa_reach, b_chunk.bounds) {
                        continue;
                    }
                    for sb in &b.segs[b_chunk.start..b_chunk.end] {
                        if bounds_overlap(sa_reach, sb.bounds) && segments_within(sa, sb, m) {
                            return false;
                        }
                    }
                }
            }
        }
    }
    // 2. Every ring of each operand lies where the other's winding is zero.
    a.ring_vertices.iter().all(|&p| winding_number(p, b) == 0)
        && b.ring_vertices.iter().all(|&p| winding_number(p, a) == 0)
}

#[inline]
fn cross(ax: i64, ay: i64, bx: i64, by: i64, cx: i64, cy: i64) -> i128 {
    (bx as i128 - ax as i128) * (cy as i128 - ay as i128)
        - (by as i128 - ay as i128) * (cx as i128 - ax as i128)
}

#[inline]
fn in_bounds(px: i64, py: i64, b: Bounds) -> bool {
    px >= b.0 && px <= b.1 && py >= b.2 && py <= b.3
}

/// Exact: segments intersect, touch, or come closer than `m` units.
fn segments_within(a: &Seg, b: &Seg, m: i64) -> bool {
    let d1 = cross(a.x0, a.y0, a.x1, a.y1, b.x0, b.y0);
    let d2 = cross(a.x0, a.y0, a.x1, a.y1, b.x1, b.y1);
    let d3 = cross(b.x0, b.y0, b.x1, b.y1, a.x0, a.y0);
    let d4 = cross(b.x0, b.y0, b.x1, b.y1, a.x1, a.y1);
    let proper = ((d1 > 0 && d2 < 0) || (d1 < 0 && d2 > 0))
        && ((d3 > 0 && d4 < 0) || (d3 < 0 && d4 > 0));
    if proper
        || (d1 == 0 && in_bounds(b.x0, b.y0, a.bounds))
        || (d2 == 0 && in_bounds(b.x1, b.y1, a.bounds))
        || (d3 == 0 && in_bounds(a.x0, a.y0, b.bounds))
        || (d4 == 0 && in_bounds(a.x1, a.y1, b.bounds))
    {
        return true;
    }
    // Non-intersecting segments: the minimum distance is attained at an
    // endpoint of one of them.
    let m2 = (m as i128) * (m as i128);
    point_segment_closer_than(b.x0, b.y0, a, m2)
        || point_segment_closer_than(b.x1, b.y1, a, m2)
        || point_segment_closer_than(a.x0, a.y0, b, m2)
        || point_segment_closer_than(a.x1, a.y1, b, m2)
}

/// Exact: squared distance from `(px, py)` to segment `s` is below `m2`.
fn point_segment_closer_than(px: i64, py: i64, s: &Seg, m2: i128) -> bool {
    let dx = s.x1 as i128 - s.x0 as i128;
    let dy = s.y1 as i128 - s.y0 as i128;
    let ux = px as i128 - s.x0 as i128;
    let uy = py as i128 - s.y0 as i128;
    let len2 = dx * dx + dy * dy;
    let dot = ux * dx + uy * dy;
    if len2 == 0 || dot <= 0 {
        return ux * ux + uy * uy < m2;
    }
    if dot >= len2 {
        let vx = px as i128 - s.x1 as i128;
        let vy = py as i128 - s.y1 as i128;
        return vx * vx + vy * vy < m2;
    }
    let c = dx * uy - dy * ux;
    c * c < m2 * len2
}

/// Winding number of `poly`'s rings around `p`, for `p` off every edge
/// (guaranteed by the separation check that precedes every call).
fn winding_number(p: Point2, poly: &ChunkedPoly) -> i64 {
    if !in_bounds(p.x, p.y, poly.bounds) {
        return 0;
    }
    let mut winding = 0i64;
    for chunk in &poly.chunks {
        let (_, max_x, min_y, max_y) = chunk.bounds;
        if max_x < p.x || max_y < p.y || min_y > p.y {
            continue;
        }
        for seg in &poly.segs[chunk.start..chunk.end] {
            let (_, s_max_x, s_min_y, s_max_y) = seg.bounds;
            if s_max_x < p.x || s_max_y < p.y || s_min_y > p.y {
                continue;
            }
            if seg.y0 <= p.y {
                if seg.y1 > p.y && cross(seg.x0, seg.y0, seg.x1, seg.y1, p.x, p.y) > 0 {
                    winding += 1;
                }
            } else if seg.y1 <= p.y && cross(seg.x0, seg.y0, seg.x1, seg.y1, p.x, p.y) < 0 {
                winding -= 1;
            }
        }
    }
    winding
}

#[cfg(test)]
mod tests {
    use super::*;
    use slicer_ir::Polygon;

    fn ring(points: &[(i64, i64)]) -> Polygon {
        Polygon {
            points: points.iter().map(|&(x, y)| Point2 { x, y }).collect(),
        }
    }

    fn poly(contour: &[(i64, i64)], holes: &[&[(i64, i64)]]) -> ExPolygon {
        ExPolygon {
            contour: ring(contour),
            holes: holes.iter().map(|h| ring(h)).collect(),
        }
    }

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
        poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)], &[])
    }

    /// CW hole ring.
    fn rect_hole(x0: i64, y0: i64, x1: i64, y1: i64) -> Vec<(i64, i64)> {
        vec![(x0, y0), (x0, y1), (x1, y1), (x1, y0)]
    }

    fn shifted(p: &ExPolygon, dx: i64, dy: i64) -> ExPolygon {
        let move_ring = |r: &Polygon| Polygon {
            points: r.points.iter().map(|q| Point2 { x: q.x + dx, y: q.y + dy }).collect(),
        };
        ExPolygon {
            contour: move_ring(&p.contour),
            holes: p.holes.iter().map(move_ring).collect(),
        }
    }

    /// The prepared test must agree with the raw Clipper2 test; returns how
    /// the answer was reached so callers can prove coverage.
    fn check(a: &ExPolygon, b: &ExPolygon) -> (&'static str, bool) {
        let want = clipper_positive_overlap(a, b);
        let (pa, pb) = (PreparedOverlap::new(a), PreparedOverlap::new(b));
        assert_eq!(pa.overlaps(&pb), want, "prepared vs Clipper2 for {a:?} / {b:?}");
        assert_eq!(pb.overlaps(&pa), want, "symmetric prepared vs Clipper2");
        let path = match (pa.bounds, pb.bounds) {
            (Some(x), Some(y)) if strictly_separated(x, y) => "bbox",
            _ => match (pa.chunked(), pb.chunked()) {
                (Some(x), Some(y)) if certify_disjoint(x, y) => "certificate",
                _ => "clipper",
            },
        };
        (path, want)
    }

    /// Boundary and topology cases at the 1-unit (100 nm) scale, each pinned to
    /// the path that must decide it.
    #[test]
    fn adversarial_cases_match_clipper_and_take_the_expected_path() {
        let big = rect(0, 0, 1_000, 1_000);
        let holed = poly(&[(0, 0), (1_000, 0), (1_000, 1_000), (0, 1_000)], &[&rect_hole(300, 300, 700, 700)]);
        let u_bay = poly(
            &[(0, 0), (900, 0), (900, 900), (600, 900), (600, 300), (300, 300), (300, 900), (0, 900)],
            &[],
        );
        let cw_big = poly(&[(0, 0), (0, 1_000), (1_000, 1_000), (1_000, 0)], &[]);
        let cases: Vec<(&str, ExPolygon, ExPolygon, &str, bool)> = vec![
            ("1-unit edge overlap", big.clone(), rect(999, 0, 2_000, 1_000), "clipper", true),
            ("1-unit^2 corner overlap", big.clone(), rect(999, 999, 2_000, 2_000), "clipper", true),
            ("shared edge, zero area", big.clone(), rect(1_000, 0, 2_000, 1_000), "clipper", false),
            ("single vertex touch", big.clone(), rect(1_000, 1_000, 2_000, 2_000), "clipper", false),
            ("1-unit gap, below margin", big.clone(), rect(1_001, 0, 2_000, 1_000), "bbox", false),
            ("A strictly inside B", rect(400, 400, 600, 600), big.clone(), "clipper", true),
            ("B strictly inside A", big.clone(), rect(400, 400, 600, 600), "clipper", true),
            ("A inside B's hole", rect(400, 400, 600, 600), holed.clone(), "certificate", false),
            ("A touching B's hole edge", rect(400, 400, 700, 600), holed.clone(), "clipper", false),
            ("A crossing B's hole edge by 1 unit", rect(400, 400, 701, 600), holed.clone(), "clipper", true),
            ("A 2 units from B's hole edge", rect(400, 400, 698, 600), holed.clone(), "clipper", false),
            ("A 5 units from B's hole edge", rect(400, 400, 695, 600), holed.clone(), "certificate", false),
            ("A surrounding B's hole", rect(200, 200, 800, 800), holed.clone(), "clipper", true),
            ("CW contour, inside", rect(400, 400, 600, 600), cw_big.clone(), "clipper", true),
            ("U-bay, clear", rect(350, 400, 550, 950), u_bay.clone(), "certificate", false),
            ("U-bay, 1-unit graze", rect(299, 400, 550, 950), u_bay.clone(), "clipper", true),
            ("beyond coordinate guard", shifted(&big, 1 << 31, 0), shifted(&rect(400, 400, 600, 600), 1 << 31, 0), "clipper", true),
        ];
        for (name, a, b, path, overlap) in cases {
            let (got_path, got_overlap) = check(&a, &b);
            assert_eq!(got_overlap, overlap, "{name}: overlap");
            assert_eq!(got_path, path, "{name}: decided by");
        }
    }

    /// Shifted real-shaped operands across the touching / near-miss scale,
    /// including holes: the prepared test never disagrees with Clipper2, and
    /// each decision path is exercised.
    #[test]
    fn shifted_pairs_match_clipper_on_every_path() {
        let comb = poly(
            &[
                (0, 0), (3_000, 0), (3_000, 400), (2_600, 400), (2_600, 2_000), (2_200, 2_000),
                (2_200, 400), (1_800, 400), (1_800, 2_000), (1_400, 2_000), (1_400, 400),
                (1_000, 400), (1_000, 2_000), (600, 2_000), (600, 400), (0, 400),
            ],
            &[],
        );
        let ring_with_hole = poly(
            &[(0, 0), (2_000, 0), (2_000, 2_000), (0, 2_000)],
            &[&rect_hole(500, 500, 1_500, 1_500)],
        );
        // Inside the comb gap x 1000..1400, 100 units from either tooth.
        let tooth = rect(1_100, 600, 1_300, 1_800);
        let mut paths = std::collections::HashMap::<&str, usize>::new();
        let mut overlaps = 0;
        for (a, b) in [(&comb, &tooth), (&ring_with_hole, &tooth), (&comb, &ring_with_hole)] {
            for dx in [-500, -60, -10, -5, -4, -3, -1, 0, 1, 3, 4, 5, 10, 60, 500, 5_000] {
                for dy in [-700, -1, 0, 1, 700] {
                    let (path, overlap) = check(a, &shifted(b, dx, dy));
                    *paths.entry(path).or_default() += 1;
                    overlaps += usize::from(overlap);
                }
            }
        }
        for path in ["bbox", "certificate", "clipper"] {
            assert!(paths.get(path).copied().unwrap_or(0) > 0, "path {path} never taken: {paths:?}");
        }
        assert!(overlaps > 0, "no overlapping pair, so the fallback answer went untested");
    }
}
