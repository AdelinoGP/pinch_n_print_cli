// -----------------------------------------------------------------------------
// Portions of this file are derived from OrcaSlicer, Bambu Studio, PrusaSlicer,
// and Slic3r, which are licensed under the GNU Affero General Public License,
// version 3 (AGPLv3).
//
// Original C++ source path: src/libslic3r/Fill/WaveOverhangs.cpp
//   (from the `dennisklappe/OrcaSlicer-WaveOverhangs` fork)
//
// This file is an LLM-generated Rust port of the original C++ implementation,
// adapted for the Pinch 'n Print architecture.
//
// Wave-overhang algorithm credit: Andersons, Sanchez, Vaneker, McCulloch,
// and Klappe.
// -----------------------------------------------------------------------------
//! Wave-overhang contour generation.
//!
//! Port of `WaveOverhangs.cpp::generate` and its helpers. The pipeline builds
//! an anchored "wave cover" over the unsupported part of a fill area, seeds a
//! polyline on the anchor boundary, then repeatedly offsets that seed inwards
//! by one wave spacing, clipping each level to the piece being filled. The
//! resulting levels are assembled into printable fronts by one of three
//! ordering strategies (`smart`, `monotonic`, `zigzag`).
//!
//! # Coordinate hazard
//!
//! OrcaSlicer uses 1 unit = 1 nm; this repository uses **1 unit = 100 nm**
//! (`slicer_ir::UNITS_PER_MM` = 10 000). Every canonical constant is therefore
//! expressed here in **millimetres** and converted exactly once, at the point
//! of use, through [`units`] (linear) or [`units_sq`] (areas — the unit factor
//! must be **squared**, not applied linearly). Canonical `max(1, …)` guards are
//! one-unit epsilons whose physical size differs 100x between the two
//! codebases; they are ported as the named [`EPSILON_MM`] constant, never as a
//! raw `1`.
//!
//! # Inert canonical settings
//!
//! The canonical fork reads `spacing_mode` and `seam_mode` into its parameter
//! block but never acts on them. They are deliberately not ported. (There is
//! no `min_angle` field in canonical `CommonParams`; the similarly named
//! `corner_angle_threshold` and `min_length_mm` are both genuinely used.)

use slicer_core::polygon_ops::{
    clip_polylines, difference, difference_ex, intersection, offset, union, union_ex,
    OffsetJoinType,
};
use slicer_ir::{ExPolygon, Point2, Polygon, UNITS_PER_MM};

use crate::WavePattern;

/// An open or closed chain of points in scaled integer units.
pub(crate) type Polyline = Vec<Point2>;

/// One scaled unit expressed in millimetres.
///
/// This is the port of canonical's `max(1, …)` / `+1` one-nanometre guards.
/// Canonical's literal `1` is 1 nm; the same *code* here would mean 100 nm, so
/// the guard is re-expressed in millimetres and converted like every other
/// constant.
pub(crate) const EPSILON_MM: f32 = 1.0e-4;

/// Canonical `EXTERNAL_INFILL_MARGIN`, in millimetres.
const EXTERNAL_INFILL_MARGIN_MM: f32 = 3.0;

/// Canonical minimum wave line spacing, in millimetres.
const MIN_LINE_SPACING_MM: f32 = 0.01;

/// Canonical default output resolution used when simplifying wave fronts, mm.
const DEFAULT_RESOLUTION_MM: f32 = 0.0125;

/// Arc tolerance handed to every round-join offset, in millimetres.
///
/// Zero selects the Clipper2 default (a fraction of the offset delta), which is
/// what canonical relies on.
const ARC_TOLERANCE_MM: f32 = 0.0;

/// Convert millimetres to scaled units as a float (linear quantity).
#[inline]
pub(crate) fn units(mm: f32) -> f64 {
    f64::from(mm) * UNITS_PER_MM
}

/// Convert square millimetres to squared scaled units (**area** quantity).
///
/// The unit factor is squared here. Applying [`units`] to an area silently
/// under-scales it by 10 000 and breaks the growth predicate.
#[inline]
pub(crate) fn units_sq(mm2: f32) -> f64 {
    f64::from(mm2) * UNITS_PER_MM * UNITS_PER_MM
}

/// Resolved per-invocation parameters for the wave-overhang generator.
///
/// All lengths are in millimetres; `min_new_area` is in square millimetres.
#[derive(Clone, Copy, Debug)]
pub(crate) struct WaveParams {
    /// Spacing between successive wave contours, in millimetres.
    pub(crate) line_spacing: f32,
    /// Overlap of the first wave contour into the adjacent perimeter, in mm.
    pub(crate) perimeter_overlap: f32,
    /// Minimum bridge-area width that still receives wave fill, in mm.
    pub(crate) minimum_width: f32,
    /// Minimum newly-covered area required to keep iterating, in mm^2.
    pub(crate) min_new_area: f32,
    /// Minimum emitted contour length, in mm.
    pub(crate) min_length: f32,
    /// Iteration cap; `0` means unbounded.
    pub(crate) max_iterations: u32,
    /// Contour ordering strategy.
    pub(crate) pattern: WavePattern,
    /// Wave extrusion width, in mm (`overhang_flow.with_width(line_width)`).
    ///
    /// This is the **nominal** width stamped on every emitted point, not the
    /// width of the bead that lands on the plate; see [`WaveParams::flow_ratio`].
    pub(crate) flow_width: f32,
    /// Ratio of deposited material to the nominal `flow_width` bead.
    ///
    /// The caller stamps `flow_width` on each point and pairs it with a
    /// `flow_factor` (`wave_flow_mm3_per_mm / (width * layer_height)`); the
    /// emitter's volumetric-E path multiplies the two, so the bead physically
    /// laid down is `flow_width * flow_ratio` wide. This field carries exactly
    /// that `flow_factor`, so the two cannot drift.
    pub(crate) flow_ratio: f32,
    /// **Flow-derived** spacing, in mm.
    ///
    /// Canonical `base_spacing = overhang_flow.scaled_spacing()`. This is NOT
    /// the `line_spacing` config key; seed expansion, anchor size, filled-area
    /// regularization and the anchoring expansion all key off this value.
    pub(crate) base_spacing: f32,
    /// Perimeter count of the enclosing region (anchor sizing).
    pub(crate) wall_count: u32,
}

/// Why the generator could not produce waves for a component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FallbackReason {
    /// No unsupported area at all inside the fill area.
    NoOverhang,
    /// A component was dropped by the `min_length` filter.
    MinLengthFiltered,
    /// Neither the inset anchors nor the raw anchors touched the wave cover.
    MissingAnchors,
    /// Seed generation produced nothing to propagate from.
    EmptySeeds,
    /// The iteration cap stopped propagation with area still unfilled.
    IterationResidual,
    /// Assembly produced no printable path.
    EmptyOutput,
}

/// Result of one [`generate`] invocation.
#[derive(Clone, Debug, Default)]
pub(crate) struct WaveOutput {
    /// Printable wave fronts, first-emitted first (anchor end of the wave).
    pub(crate) paths: Vec<Polyline>,
    /// Union of every region the waves covered.
    pub(crate) filled: Vec<ExPolygon>,
    /// Non-empty when the caller must fall back to conventional bridge fill.
    pub(crate) fallbacks: Vec<FallbackReason>,
}

// ---------------------------------------------------------------------------
// Small geometry helpers
// ---------------------------------------------------------------------------

#[inline]
fn dist_sq(a: Point2, b: Point2) -> f64 {
    let dx = (a.x - b.x) as f64;
    let dy = (a.y - b.y) as f64;
    dx * dx + dy * dy
}

/// Signed doubled area of a closed contour, in squared scaled units.
fn contour_area2(poly: &Polygon) -> f64 {
    let n = poly.points.len();
    if n < 3 {
        return 0.0;
    }
    let mut acc = 0.0_f64;
    for i in 0..n {
        let a = poly.points[i];
        let b = poly.points[(i + 1) % n];
        acc += (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64);
    }
    acc
}

/// Absolute area of an `ExPolygon` (contour minus holes), in squared units.
fn expolygon_area(exp: &ExPolygon) -> f64 {
    let mut area = contour_area2(&exp.contour).abs();
    for hole in &exp.holes {
        area -= contour_area2(hole).abs();
    }
    (area / 2.0).max(0.0)
}

/// Total absolute area of a polygon set, in squared scaled units.
fn total_area(polys: &[ExPolygon]) -> f64 {
    polys.iter().map(expolygon_area).sum()
}

/// Perimeter length of a closed contour, in scaled units.
fn contour_length(poly: &Polygon) -> f64 {
    let n = poly.points.len();
    if n < 2 {
        return 0.0;
    }
    let mut acc = 0.0_f64;
    for i in 0..n {
        acc += dist_sq(poly.points[i], poly.points[(i + 1) % n]).sqrt();
    }
    acc
}

/// Length of an open polyline, in scaled units.
fn polyline_length(pl: &[Point2]) -> f64 {
    pl.windows(2).map(|w| dist_sq(w[0], w[1]).sqrt()).sum()
}

/// Number of holes across a polygon set.
fn hole_count(polys: &[ExPolygon]) -> usize {
    polys.iter().map(|p| p.holes.len()).sum()
}

/// Explode a polygon set into closed polylines (contour and every hole).
///
/// Canonical `to_polylines`. The first point is repeated at the end so the
/// chain is geometrically closed.
fn to_polylines(polys: &[ExPolygon]) -> Vec<Polyline> {
    let mut out = Vec::new();
    for exp in polys {
        for ring in std::iter::once(&exp.contour).chain(exp.holes.iter()) {
            if ring.points.len() < 2 {
                continue;
            }
            let mut pl = ring.points.clone();
            pl.push(ring.points[0]);
            out.push(pl);
        }
    }
    out
}

/// Axis-aligned bounding box of a polygon set, in scaled units.
fn bbox(polys: &[ExPolygon]) -> Option<(i64, i64, i64, i64)> {
    let mut acc: Option<(i64, i64, i64, i64)> = None;
    for exp in polys {
        for p in &exp.contour.points {
            acc = Some(match acc {
                None => (p.x, p.y, p.x, p.y),
                Some((min_x, min_y, max_x, max_y)) => (
                    min_x.min(p.x),
                    min_y.min(p.y),
                    max_x.max(p.x),
                    max_y.max(p.y),
                ),
            });
        }
    }
    acc
}

/// Rectangle `ExPolygon` from scaled-unit bounds.
fn rect(min_x: i64, min_y: i64, max_x: i64, max_y: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: min_x, y: min_y },
                Point2 { x: max_x, y: min_y },
                Point2 { x: max_x, y: max_y },
                Point2 { x: min_x, y: max_y },
            ],
        },
        holes: Vec::new(),
    }
}

/// Even-odd containment test against a single ring.
fn point_in_ring(p: Point2, ring: &Polygon) -> bool {
    let pts = &ring.points;
    let n = pts.len();
    if n < 3 {
        return false;
    }
    let (px, py) = (p.x as f64, p.y as f64);
    let mut inside = false;
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = (pts[i].x as f64, pts[i].y as f64);
        let (xj, yj) = (pts[j].x as f64, pts[j].y as f64);
        if (yi > py) != (yj > py) {
            let t = (py - yi) / (yj - yi);
            if px < xi + t * (xj - xi) {
                inside = !inside;
            }
        }
        j = i;
    }
    inside
}

/// Containment test against a polygon set (contour minus holes).
fn point_in_polys(p: Point2, polys: &[ExPolygon]) -> bool {
    polys.iter().any(|exp| {
        point_in_ring(p, &exp.contour) && !exp.holes.iter().any(|h| point_in_ring(p, h))
    })
}

/// Minimum distance from `p` to `pl`, and whether the closest projection foot
/// falls strictly inside a segment (canonical's `interior_projection`).
fn point_to_polyline(p: Point2, pl: &[Point2]) -> (f64, bool) {
    let mut best = f64::MAX;
    let mut interior = false;
    for w in pl.windows(2) {
        let (ax, ay) = (w[0].x as f64, w[0].y as f64);
        let (bx, by) = (w[1].x as f64, w[1].y as f64);
        let (dx, dy) = (bx - ax, by - ay);
        let len_sq = dx * dx + dy * dy;
        let (t, is_interior) = if len_sq <= f64::EPSILON {
            (0.0, false)
        } else {
            let raw = ((p.x as f64 - ax) * dx + (p.y as f64 - ay) * dy) / len_sq;
            (raw.clamp(0.0, 1.0), raw > 0.0 && raw < 1.0)
        };
        let fx = ax + t * dx;
        let fy = ay + t * dy;
        let d = ((p.x as f64 - fx).powi(2) + (p.y as f64 - fy).powi(2)).sqrt();
        if d < best {
            best = d;
            interior = is_interior;
        }
    }
    (best, interior)
}

/// Point at arc-length `target` along `pl`.
fn point_at_length(pl: &[Point2], target: f64) -> Point2 {
    let Some(first) = pl.first().copied() else {
        return Point2 { x: 0, y: 0 };
    };
    if target <= 0.0 {
        return first;
    }
    let mut acc = 0.0_f64;
    for w in pl.windows(2) {
        let seg = dist_sq(w[0], w[1]).sqrt();
        if acc + seg >= target {
            let t = if seg <= f64::EPSILON {
                0.0
            } else {
                (target - acc) / seg
            };
            return Point2 {
                x: (w[0].x as f64 + t * (w[1].x - w[0].x) as f64).round() as i64,
                y: (w[0].y as f64 + t * (w[1].y - w[0].y) as f64).round() as i64,
            };
        }
        acc += seg;
    }
    pl.last().copied().unwrap_or(first)
}

/// Douglas-Peucker simplification with an explicit stack (no recursion).
fn simplify_polyline(pl: &[Point2], tolerance: f64) -> Polyline {
    if pl.len() < 3 || tolerance <= 0.0 {
        return pl.to_vec();
    }
    let mut keep = vec![false; pl.len()];
    keep[0] = true;
    let last_idx = pl.len() - 1;
    keep[last_idx] = true;
    let mut stack = vec![(0usize, last_idx)];
    while let Some((start, end)) = stack.pop() {
        if end <= start + 1 {
            continue;
        }
        let mut worst = 0.0_f64;
        let mut worst_idx = start;
        let seg = [pl[start], pl[end]];
        for (offset_idx, p) in pl[start + 1..end].iter().enumerate() {
            let (d, _) = point_to_polyline(*p, &seg);
            if d > worst {
                worst = d;
                worst_idx = start + 1 + offset_idx;
            }
        }
        if worst > tolerance && worst_idx > start {
            keep[worst_idx] = true;
            stack.push((start, worst_idx));
            stack.push((worst_idx, end));
        }
    }
    pl.iter()
        .zip(keep)
        .filter_map(|(p, k)| if k { Some(*p) } else { None })
        .collect()
}

/// Buffer open polylines into polygons with round joins and round ends.
///
/// Canonical calls Clipper's `offset(..., jtRound, etOpenRound)`. `polygon_ops`
/// exposes no open-path inflate, so the buffer is built explicitly from one
/// regular polygon ("circle") per vertex plus one quad per segment, then
/// unioned. The result is the same round-capped buffer, and it is fully
/// deterministic.
fn buffer_open(polylines: &[Polyline], delta_mm: f32) -> Vec<ExPolygon> {
    let radius = units(delta_mm);
    if radius <= 0.0 {
        return Vec::new();
    }
    /// Segments per full circle for the round caps and joins.
    const CIRCLE_SEGMENTS: usize = 16;
    let mut parts: Vec<ExPolygon> = Vec::new();
    for pl in polylines {
        for p in pl {
            let pts = (0..CIRCLE_SEGMENTS)
                .map(|i| {
                    let a = std::f64::consts::TAU * (i as f64) / (CIRCLE_SEGMENTS as f64);
                    Point2 {
                        x: (p.x as f64 + radius * a.cos()).round() as i64,
                        y: (p.y as f64 + radius * a.sin()).round() as i64,
                    }
                })
                .collect::<Vec<_>>();
            parts.push(ExPolygon {
                contour: Polygon { points: pts },
                holes: Vec::new(),
            });
        }
        for w in pl.windows(2) {
            let (ax, ay) = (w[0].x as f64, w[0].y as f64);
            let (bx, by) = (w[1].x as f64, w[1].y as f64);
            let (dx, dy) = (bx - ax, by - ay);
            let len = (dx * dx + dy * dy).sqrt();
            if len <= f64::EPSILON {
                continue;
            }
            let (nx, ny) = (-dy / len * radius, dx / len * radius);
            parts.push(ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 {
                            x: (ax + nx).round() as i64,
                            y: (ay + ny).round() as i64,
                        },
                        Point2 {
                            x: (bx + nx).round() as i64,
                            y: (by + ny).round() as i64,
                        },
                        Point2 {
                            x: (bx - nx).round() as i64,
                            y: (by - ny).round() as i64,
                        },
                        Point2 {
                            x: (ax - nx).round() as i64,
                            y: (ay - ny).round() as i64,
                        },
                    ],
                },
                holes: Vec::new(),
            });
        }
    }
    union_ex(&parts)
}

// ---------------------------------------------------------------------------
// Front-merge closing pass (deviation from canonical)
// ---------------------------------------------------------------------------

/// Minimum residual width, as a fraction of the flow width, that earns a
/// closing path. Anything narrower is already bridged by the two neighbouring
/// beads and printing into it would only stack material.
const CLOSING_MIN_WIDTH_FACTOR: f32 = 0.5;
/// Arc-length spacing used when resampling a residual strip's two side chains
/// into a medial polyline, in millimetres.
const CLOSING_SAMPLE_STEP_MM: f32 = 0.2;
/// Upper bound on medial resampling, so the closing pass stays bounded on a
/// pathologically long residual.
const CLOSING_MAX_SAMPLES: usize = 512;

/// Approximate medial polyline of a thin residual strip.
///
/// The strip's contour runs up one side and back down the other, so splitting
/// the ring at its diameter pair — the two ends of the strip — yields two
/// chains that face each other. Resampling both by arc length and averaging
/// corresponding samples gives a centred polyline. This is deliberately not a
/// true medial axis; it does not need to be, because the closing pass only
/// ever sees the sliver left between two wave fronts that merged.
fn strip_centerline(exp: &ExPolygon, tolerance: f64, step: f64) -> Polyline {
    let ring = simplify_polyline(&exp.contour.points, tolerance);
    let n = ring.len();
    if n < 3 {
        return Vec::new();
    }
    let mut bi = 0usize;
    let mut bj = 0usize;
    let mut best = -1.0f64;
    for i in 0..n {
        for j in (i + 1)..n {
            let d = dist_sq(ring[i], ring[j]);
            if d > best {
                best = d;
                bi = i;
                bj = j;
            }
        }
    }
    if best <= 0.0 {
        return Vec::new();
    }
    let side_a: Polyline = ring[bi..=bj].to_vec();
    // `ring[bj..] ++ ring[..=bi]` walks bj -> bi the long way round; reversing
    // it makes both chains run bi -> bj.
    let mut side_b: Polyline = ring[bj..].to_vec();
    side_b.extend_from_slice(&ring[..=bi]);
    side_b.reverse();
    if side_a.len() < 2 || side_b.len() < 2 {
        return Vec::new();
    }
    let len_a = polyline_length(&side_a);
    let len_b = polyline_length(&side_b);
    let samples = ((best.sqrt() / step).ceil() as usize + 1).clamp(2, CLOSING_MAX_SAMPLES);
    let mut out: Polyline = Vec::with_capacity(samples);
    for k in 0..samples {
        let t = (k as f64) / ((samples - 1) as f64);
        let a = point_at_length(&side_a, len_a * t);
        let b = point_at_length(&side_b, len_b * t);
        let mid = Point2 {
            x: ((a.x as f64 + b.x as f64) / 2.0).round() as i64,
            y: ((a.y as f64 + b.y as f64) / 2.0).round() as i64,
        };
        if out.last() != Some(&mid) {
            out.push(mid);
        }
    }
    out
}

/// Close the seam left behind when two opposing wave fronts merge.
///
/// **Deviation from canonical.** Each propagation level is emitted as the
/// *contour* of the growing accumulated region. While two fronts advance
/// towards each other that contour runs along both of their leading edges, but
/// the moment they touch, the merged region's contour no longer passes between
/// them — so the last strip of material between the two fronts never receives a
/// centreline. Whether that shows as a visible void depends on how much of the
/// residual the two adjacent beads happen to cover, which is why the defect is
/// layer-dependent and leaves the path count unchanged.
///
/// The pass is residual-driven rather than merge-detecting, so it is robust to
/// three-way and multi-component merges: it subtracts the **actual swept
/// footprints** of everything already emitted (per-point flow width, never the
/// centrelines) from the trimmed piece and emits one medial polyline per
/// leftover component wider than [`CLOSING_MIN_WIDTH_FACTOR`] flow widths.
/// Components are visited in bounding-box order so the output does not depend
/// on the clipper's component ordering.
fn closing_pass(
    levels: &[Vec<Polyline>],
    trim_boundary: &[ExPolygon],
    flow_width_mm: f32,
) -> Vec<Polyline> {
    if flow_width_mm <= 0.0 || trim_boundary.is_empty() {
        return Vec::new();
    }
    let emitted: Vec<Polyline> = levels.iter().flatten().cloned().collect();
    if emitted.is_empty() {
        return Vec::new();
    }
    let swept = buffer_open(&emitted, flow_width_mm / 2.0);
    if swept.is_empty() {
        return Vec::new();
    }
    let mut residual = union_ex(&difference(trim_boundary, &swept));
    if residual.is_empty() {
        return Vec::new();
    }
    residual.sort_by_key(|exp| bbox(std::slice::from_ref(exp)).unwrap_or((0, 0, 0, 0)));

    let min_half = CLOSING_MIN_WIDTH_FACTOR * flow_width_mm / 2.0;
    let tolerance = units(0.05 * flow_width_mm);
    let step = units(CLOSING_SAMPLE_STEP_MM).max(1.0);
    let min_length = units(flow_width_mm);
    let mut out: Vec<Polyline> = Vec::new();
    for comp in &residual {
        let comp_slice = std::slice::from_ref(comp);
        // Width test: a component survives erosion by half the threshold only
        // if it is wider than the threshold somewhere.
        if offset(comp_slice, -min_half, OffsetJoinType::Round, ARC_TOLERANCE_MM).is_empty() {
            continue;
        }
        let center = strip_centerline(comp, tolerance, step);
        if center.len() < 2 {
            continue;
        }
        for pl in clip_polylines(std::slice::from_ref(&center), comp_slice) {
            let simplified = simplify_polyline(&pl, tolerance);
            if simplified.len() >= 2 && polyline_length(&simplified) >= min_length {
                out.push(simplified);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Canonical helpers
// ---------------------------------------------------------------------------

/// Canonical `reconnect_polylines`.
///
/// Greedy O(n^2) merge: for every surviving base `a`, scan forwards for a `b`
/// whose endpoints come within `limit_distance`, join it on, and erase it. The
/// four endpoint pairs are tested in canonical order — `base.last/next.first`,
/// `base.last/next.last`, `base.first/next.last`, `base.first/next.first`.
fn reconnect_polylines(polylines: &mut Vec<Polyline>, limit_distance: f64) {
    let limit_sq = limit_distance * limit_distance;
    let mut a = 0usize;
    while a < polylines.len() {
        let mut b = a + 1;
        while b < polylines.len() {
            if polylines[a].len() < 2 || polylines[b].len() < 2 {
                b += 1;
                continue;
            }
            let base_first = polylines[a][0];
            let base_last = *polylines[a].last().expect("len >= 2");
            let next_first = polylines[b][0];
            let next_last = *polylines[b].last().expect("len >= 2");

            let joined = if dist_sq(base_last, next_first) < limit_sq {
                let mut next = polylines[b].clone();
                next.remove(0);
                polylines[a].extend(next);
                true
            } else if dist_sq(base_last, next_last) < limit_sq {
                let mut next = polylines[b].clone();
                next.reverse();
                next.remove(0);
                polylines[a].extend(next);
                true
            } else if dist_sq(base_first, next_last) < limit_sq {
                let mut next = polylines[b].clone();
                next.pop();
                next.extend(polylines[a].iter().copied());
                polylines[a] = next;
                true
            } else if dist_sq(base_first, next_first) < limit_sq {
                let mut next = polylines[b].clone();
                next.reverse();
                next.pop();
                next.extend(polylines[a].iter().copied());
                polylines[a] = next;
                true
            } else {
                false
            };

            if joined {
                polylines.remove(b);
                // Restart the scan for this base, as canonical does.
                b = a + 1;
            } else {
                b += 1;
            }
        }
        a += 1;
    }
}

/// Canonical `generate_wave_overhang_seeds`.
///
/// Seeds are the part of the wave cover's own boundary that lies on the
/// anchoring material. The primary pass keeps seeds from boundary index 0
/// only; when that keeps nothing, canonical retries against the whole boundary
/// clipped by an expanded anchoring.
fn generate_wave_overhang_seeds(
    boundary: &[ExPolygon],
    anchoring: &[ExPolygon],
    seed_expansion_mm: f32,
) -> Vec<Polyline> {
    if anchoring.is_empty() || boundary.is_empty() {
        return Vec::new();
    }
    let primary: Vec<Polyline> = clip_polylines(&to_polylines(&boundary[..1]), anchoring)
        .into_iter()
        .filter(|p| p.len() >= 2)
        .collect();
    if !primary.is_empty() {
        return primary;
    }
    let expanded = offset(
        anchoring,
        seed_expansion_mm,
        OffsetJoinType::Round,
        ARC_TOLERANCE_MM,
    );
    clip_polylines(&to_polylines(boundary), &expanded)
        .into_iter()
        .filter(|p| p.len() >= 2)
        .collect()
}

/// Rectangle straddling `a`-`b`, of the given half width, extended past both
/// endpoints by `extension`.
fn slit_rect(a: Point2, b: Point2, half_width: f64, extension: f64) -> ExPolygon {
    let (ax, ay) = (a.x as f64, a.y as f64);
    let (bx, by) = (b.x as f64, b.y as f64);
    let (mut dx, mut dy) = (bx - ax, by - ay);
    let len = (dx * dx + dy * dy).sqrt();
    if len <= f64::EPSILON {
        // Degenerate pair: orient the slit along +X so the rectangle is still
        // well formed.
        dx = 1.0;
        dy = 0.0;
    } else {
        dx /= len;
        dy /= len;
    }
    let (nx, ny) = (-dy * half_width, dx * half_width);
    let (ex, ey) = (dx * extension, dy * extension);
    let start = (ax - ex, ay - ey);
    let end = (bx + ex, by + ey);
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 {
                    x: (start.0 + nx).round() as i64,
                    y: (start.1 + ny).round() as i64,
                },
                Point2 {
                    x: (end.0 + nx).round() as i64,
                    y: (end.1 + ny).round() as i64,
                },
                Point2 {
                    x: (end.0 - nx).round() as i64,
                    y: (end.1 - ny).round() as i64,
                },
                Point2 {
                    x: (start.0 - nx).round() as i64,
                    y: (start.1 - ny).round() as i64,
                },
            ],
        },
        holes: Vec::new(),
    }
}

/// Closest vertex pair between two rings, as `(distance^2, a, b)`.
fn closest_pair(u: &Polygon, v: &Polygon) -> Option<(f64, Point2, Point2)> {
    let mut best: Option<(f64, Point2, Point2)> = None;
    for a in &u.points {
        for b in &v.points {
            let d = dist_sq(*a, *b);
            if best.is_none_or(|(bd, _, _)| d < bd) {
                best = Some((d, *a, *b));
            }
        }
    }
    best
}

/// Canonical `generate_narrow_split_slits`.
///
/// Narrow necks in the wave cover make the propagation front pinch off. The
/// canonical fix erodes the cover at four fractions of a wave spacing; when the
/// erosion changes the topology (more components, or a different hole count)
/// the neck is real, and a thin rectangular "slit" is cut across it so the
/// cover splits into pieces that each propagate cleanly.
fn generate_narrow_split_slits(
    cover: &[ExPolygon],
    wave_spacing_mm: f32,
    min_width_mm: f32,
) -> Vec<ExPolygon> {
    if min_width_mm <= 0.0 || cover.is_empty() {
        return Vec::new();
    }
    let eps = units(EPSILON_MM);
    let spacing = units(wave_spacing_mm);
    let min_width = units(min_width_mm);
    let max_gap_sq = min_width * min_width;
    let slit_half_width = (spacing / 20.0).max(eps);
    let slit_extension = slit_half_width.max(min_width);
    let dup_radius = 0.5 * spacing;
    let dup_radius_sq = dup_radius * dup_radius;

    let base = union_ex(cover);
    let base_components = base.len();
    let base_holes = hole_count(&base);

    let mut candidates: Vec<(f64, Point2, Point2)> = Vec::new();
    for frac in [0.25_f32, 0.5, 0.75, 1.0] {
        let eroded = union_ex(&offset(
            &base,
            -(frac * wave_spacing_mm),
            OffsetJoinType::Round,
            ARC_TOLERANCE_MM,
        ));
        if !(eroded.len() > base_components.max(1) || hole_count(&eroded) != base_holes) {
            continue;
        }
        // Component-to-component necks.
        for i in 0..eroded.len() {
            for j in (i + 1)..eroded.len() {
                if let Some(c) = closest_pair(&eroded[i].contour, &eroded[j].contour) {
                    candidates.push(c);
                }
            }
        }
        // Outer-to-hole and hole-to-hole necks.
        for exp in &eroded {
            for (hi, hole) in exp.holes.iter().enumerate() {
                if let Some(c) = closest_pair(&exp.contour, hole) {
                    candidates.push(c);
                }
                for other in exp.holes.iter().skip(hi + 1) {
                    if let Some(c) = closest_pair(hole, other) {
                        candidates.push(c);
                    }
                }
            }
        }
    }

    candidates.retain(|(d, a, b)| {
        if *d > max_gap_sq {
            return false;
        }
        let mid = Point2 {
            x: (a.x + b.x) / 2,
            y: (a.y + b.y) / 2,
        };
        point_in_polys(mid, &base)
    });

    // Ascending by gap width; coordinates break ties so the order is stable.
    candidates.sort_by(|l, r| {
        l.0.total_cmp(&r.0)
            .then_with(|| (l.1.x, l.1.y, l.2.x, l.2.y).cmp(&(r.1.x, r.1.y, r.2.x, r.2.y)))
    });

    let mut kept_mids: Vec<Point2> = Vec::new();
    let mut slits: Vec<ExPolygon> = Vec::new();
    for (_, a, b) in candidates {
        let mid = Point2 {
            x: (a.x + b.x) / 2,
            y: (a.y + b.y) / 2,
        };
        if kept_mids.iter().any(|m| dist_sq(*m, mid) < dup_radius_sq) {
            continue;
        }
        let mut half = slit_half_width.max(eps).max(spacing / 2.0 + eps);
        let extension = spacing + slit_extension;
        let mut accepted = None;
        for _ in 0..6 {
            let candidate = slit_rect(a, b, half, extension);
            let cut = difference_ex(&base, std::slice::from_ref(&candidate));
            if cut.len() > base_components || hole_count(&cut) != base_holes {
                accepted = Some(candidate);
                break;
            }
            half *= 2.0;
        }
        if let Some(slit) = accepted {
            kept_mids.push(mid);
            slits.push(slit);
        }
    }
    union_ex(&slits)
}

/// Canonical `should_generate_waves_for_region`.
///
/// The full canonical predicate also decides bridgeability when waves are only
/// an *option*. In this repository the module is selected as the
/// `claim:bridge-fill` holder, which is exactly canonical's
/// `use_instead_of_bridges == true`, so the predicate short-circuits to `true`
/// whenever there is any real overhang. Ported for fidelity; the bridgeability
/// branch is unreachable under holder selection.
fn should_generate_waves_for_region(
    real_overhang: &[ExPolygon],
    use_instead_of_bridges: bool,
) -> bool {
    if real_overhang.is_empty() {
        return false;
    }
    if use_instead_of_bridges {
        return true;
    }
    // Unreachable here; canonical evaluates bridgeability of `real_overhang`.
    true
}

/// Score a candidate front against already-emitted paths (canonical
/// `support_score`).
fn support_score(candidate: &[Point2], support: &[Polyline], reach: f64, prefix_length: f64) -> f64 {
    if support.is_empty() || candidate.len() < 2 {
        return -1.0;
    }
    let sample_length = polyline_length(candidate).min(prefix_length.max(units(EPSILON_MM)));
    let samples = [
        (0.0_f64, 3.0_f64),
        (0.5 * sample_length, 2.0),
        (sample_length, 1.0),
    ];
    let mut best = -1.0_f64;
    // Newest support first, as canonical does.
    for path in support.iter().rev() {
        if path.len() < 2 {
            continue;
        }
        let mut score = 0.0_f64;
        for (at, weight) in samples {
            let p = point_at_length(candidate, at);
            let (dist, interior) = point_to_polyline(p, path);
            let normalized = (1.0 - dist / reach.max(units(EPSILON_MM))).max(0.0);
            score += weight * (3.0 * normalized + if interior { 1.5 } else { 0.2 });
        }
        if score > best {
            best = score;
        }
    }
    best
}

/// Canonical `append_wave_fronts`.
///
/// # Deliberate canonical asymmetry
///
/// The ZigZag branch here compares **raw (linear)** endpoint distances against
/// `connector_limit`, while [`append_zig_zag_front_levels`] compares
/// **squared** distances against the same limit. That difference is present in
/// canonical and is preserved intentionally — do not "fix" either side.
fn append_wave_fronts(
    fronts: &[Polyline],
    flow_width_mm: f32,
    connector_limit: f64,
    pattern: WavePattern,
) -> Vec<Polyline> {
    match pattern {
        WavePattern::Monotonic => fronts.iter().filter(|f| f.len() >= 2).cloned().collect(),
        WavePattern::Zigzag => {
            let mut out: Vec<Polyline> = Vec::new();
            let mut current: Polyline = Vec::new();
            for front in fronts.iter().filter(|f| f.len() >= 2) {
                let mut next = front.clone();
                if current.is_empty() {
                    current = next;
                    continue;
                }
                let last = *current.last().expect("non-empty");
                // LINEAR distances (see the asymmetry note above).
                let d_keep = dist_sq(last, next[0]).sqrt();
                let d_flip = dist_sq(last, *next.last().expect("len >= 2")).sqrt();
                if d_keep.min(d_flip) > connector_limit {
                    out.push(std::mem::take(&mut current));
                    current = next;
                    continue;
                }
                if d_flip < d_keep {
                    next.reverse();
                }
                if last == next[0] {
                    next.remove(0);
                }
                current.extend(next);
            }
            if !current.is_empty() {
                out.push(current);
            }
            out
        }
        WavePattern::Smart => {
            let reach = units(flow_width_mm).max(connector_limit);
            let prefix_length = units(flow_width_mm).max(connector_limit / 2.0);
            let mut emitted: Vec<Polyline> = Vec::new();
            for front in fronts.iter().filter(|f| f.len() >= 2) {
                let forward = front.clone();
                let mut reversed = front.clone();
                reversed.reverse();
                let s_fwd = support_score(&forward, &emitted, reach, prefix_length);
                let s_rev = support_score(&reversed, &emitted, reach, prefix_length);
                emitted.push(if s_rev > s_fwd { reversed } else { forward });
            }
            emitted
        }
    }
}

/// Canonical `append_zig_zag_front_levels`, with the depth-first recursion
/// rewritten as an explicit worklist loop (the canonical recursion is unbounded
/// and would overflow the stack on large layers).
///
/// Distances here are compared **squared** against `connector_limit^2`; see the
/// asymmetry note on [`append_wave_fronts`].
fn append_zig_zag_front_levels(levels: &[Vec<Polyline>], connector_limit: f64) -> Vec<Polyline> {
    let limit_sq = connector_limit * connector_limit;
    let mut used: Vec<Vec<bool>> = levels.iter().map(|l| vec![false; l.len()]).collect();
    let mut out: Vec<Polyline> = Vec::new();
    let mut current: Polyline = Vec::new();

    for seed_level in 0..levels.len() {
        for seed_idx in 0..levels[seed_level].len() {
            if used[seed_level][seed_idx] || levels[seed_level][seed_idx].len() < 2 {
                continue;
            }
            let mut cursor = Some((seed_level, seed_idx, false));
            while let Some((cl, cf, reverse)) = cursor {
                used[cl][cf] = true;
                let mut front = levels[cl][cf].clone();
                if reverse {
                    front.reverse();
                }
                if current.is_empty() {
                    current = front;
                } else {
                    let last = *current.last().expect("non-empty");
                    let d_keep = dist_sq(last, front[0]);
                    let d_flip = dist_sq(last, *front.last().expect("len >= 2"));
                    if d_keep.min(d_flip) > limit_sq {
                        out.push(std::mem::take(&mut current));
                        current = front;
                    } else {
                        if d_flip < d_keep {
                            front.reverse();
                        }
                        if last == front[0] {
                            front.remove(0);
                        }
                        current.extend(front);
                    }
                }

                // Search FORWARD levels only for the nearest unused front.
                let tail = *current.last().expect("non-empty");
                let mut best: Option<(f64, (usize, usize, bool))> = None;
                for (nl, level) in levels.iter().enumerate().skip(cl + 1) {
                    for (nf, pl) in level.iter().enumerate() {
                        if used[nl][nf] || pl.len() < 2 {
                            continue;
                        }
                        let d_first = dist_sq(tail, pl[0]);
                        // `<=` so that a tie flips the candidate, as canonical.
                        if best.is_none_or(|(bd, _)| d_first <= bd) {
                            best = Some((d_first, (nl, nf, false)));
                        }
                        let d_last = dist_sq(tail, *pl.last().expect("len >= 2"));
                        if best.is_none_or(|(bd, _)| d_last <= bd) {
                            best = Some((d_last, (nl, nf, true)));
                        }
                    }
                }
                cursor = best.map(|(_, next)| next);
            }
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    out
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Port of `WaveOverhangs.cpp::generate`.
///
/// `infill_area` is the region to fill; `lower` is the supporting material
/// beneath it. All returned geometry is in scaled units.
pub(crate) fn generate(
    infill_area: &[ExPolygon],
    lower: &[ExPolygon],
    params: &WaveParams,
) -> WaveOutput {
    let mut out = WaveOutput::default();
    if infill_area.is_empty() {
        out.fallbacks.push(FallbackReason::NoOverhang);
        return out;
    }

    // 1. Minimum-length filter (skipped entirely when min_length == 0).
    let filtered: Vec<ExPolygon> = if params.min_length > 0.0 {
        let min_len = units(params.min_length);
        let kept: Vec<ExPolygon> = infill_area
            .iter()
            .filter(|e| contour_length(&e.contour) >= min_len)
            .cloned()
            .collect();
        if kept.len() != infill_area.len() {
            out.fallbacks.push(FallbackReason::MinLengthFiltered);
        }
        kept
    } else {
        infill_area.to_vec()
    };
    if filtered.is_empty() {
        return out;
    }

    // 2. Scaled constants. `wave_flow = overhang_flow.with_width(line_width)`.
    let wave_spacing_mm = params.line_spacing.max(MIN_LINE_SPACING_MM);
    let wave_spacing = units(wave_spacing_mm);
    let flow_width_mm = if params.flow_width > 0.0 {
        params.flow_width
    } else {
        params.base_spacing
    };
    // Canonical shrinks the piece by half its own extrusion WIDTH before
    // clipping fronts to it. Canonical's `wave_flow` width *is* the deposited
    // width; here the caller stamps a nominal width plus a flow factor, so the
    // canonical quantity is their product. Using the nominal width alone leaves
    // `(deposited - nominal) / 2` of bead hanging outside the fillable region,
    // which lands on the adjacent wall.
    let deposited_width_mm = if params.flow_ratio > 0.0 {
        flow_width_mm * params.flow_ratio
    } else {
        flow_width_mm
    };
    let base_spacing_mm = params.base_spacing.max(EPSILON_MM);
    let seed_expansion_mm = (base_spacing_mm / 10.0).max(EPSILON_MM);
    let anchors_size_mm =
        EXTERNAL_INFILL_MARGIN_MM.min(base_spacing_mm * (params.wall_count as f32 + 1.0));
    let regularization_mm = (base_spacing_mm / 2.0).max(EPSILON_MM);
    let zig_zag_connector_limit =
        units(wave_spacing_mm.max(flow_width_mm) + params.perimeter_overlap);
    // AREA quantity: the unit factor is squared, never applied linearly.
    let min_area_growth = if params.min_new_area > 0.0 {
        units_sq(params.min_new_area)
    } else {
        0.05 * wave_spacing * wave_spacing
    };

    // 3. Clip the lower slices to the fill-area bbox (inflated by one epsilon)
    //    and derive the unsupported area.
    let lower_clipped = match bbox(&filtered) {
        Some((min_x, min_y, max_x, max_y)) => {
            let e = units(EPSILON_MM).round() as i64;
            intersection(lower, &[rect(min_x - e, min_y - e, max_x + e, max_y + e)])
        }
        None => Vec::new(),
    };
    let overhangs = difference(&filtered, &lower_clipped);
    if overhangs.is_empty() {
        out.fallbacks.push(FallbackReason::NoOverhang);
        return out;
    }

    // 4. Anchors, inset anchors, and the inset overhang area.
    let anchors = intersection(&filtered, &lower_clipped);
    let inset_anchors = difference(
        &anchors,
        &offset(
            &overhangs,
            anchors_size_mm + 0.1 * flow_width_mm,
            OffsetJoinType::Square,
            ARC_TOLERANCE_MM,
        ),
    );
    let inset_overhang_area = difference(&filtered, &inset_anchors);

    // 5. Per-component wave covers.
    let mut covers: Vec<ExPolygon> = Vec::new();
    for component in union_ex(&inset_overhang_area) {
        let wave_cover_area = offset(
            std::slice::from_ref(&component),
            params.perimeter_overlap,
            OffsetJoinType::Round,
            ARC_TOLERANCE_MM,
        );
        let real_overhang = intersection(&wave_cover_area, &overhangs);
        // Holder selection == canonical `use_instead_of_bridges == true`.
        if !should_generate_waves_for_region(&real_overhang, true) {
            continue;
        }
        covers.extend(union_ex(&wave_cover_area));
    }
    if covers.is_empty() {
        out.fallbacks.push(FallbackReason::NoOverhang);
        return out;
    }

    let mut levels_all: Vec<Vec<Polyline>> = Vec::new();
    let mut filled: Vec<ExPolygon> = Vec::new();
    let resolution = units(DEFAULT_RESOLUTION_MM);

    for cover in &covers {
        let cover_slice = std::slice::from_ref(cover);

        // 6. Split narrow necks.
        let slits = generate_narrow_split_slits(cover_slice, wave_spacing_mm, params.minimum_width);
        let pieces = if slits.is_empty() {
            vec![cover.clone()]
        } else {
            difference_ex(cover_slice, &slits)
        };

        // 7. Anchoring and seeds (thin-wall retry against the un-inset anchors).
        let expanded_cover = offset(
            cover_slice,
            1.1 * base_spacing_mm,
            OffsetJoinType::Round,
            ARC_TOLERANCE_MM,
        );
        let mut full_anchoring = intersection(&expanded_cover, &inset_anchors);
        if full_anchoring.is_empty() {
            full_anchoring = intersection(&expanded_cover, &anchors);
        }
        if full_anchoring.is_empty() {
            out.fallbacks.push(FallbackReason::MissingAnchors);
            continue;
        }
        let base_seeds =
            generate_wave_overhang_seeds(cover_slice, &full_anchoring, seed_expansion_mm);
        if base_seeds.is_empty() {
            out.fallbacks.push(FallbackReason::EmptySeeds);
            continue;
        }

        for piece in &pieces {
            let piece_slice = std::slice::from_ref(piece);
            let seeds = clip_polylines(&base_seeds, piece_slice);
            if seeds.is_empty() {
                continue;
            }

            // 8. Trim boundary, with the two canonical fallbacks. The inset is
            //    half the DEPOSITED bead width (see `deposited_width_mm`), not
            //    the nominal flow width.
            let mut trim_boundary = offset(
                piece_slice,
                -(EPSILON_MM.max(deposited_width_mm / 2.0)),
                OffsetJoinType::Round,
                ARC_TOLERANCE_MM,
            );
            if trim_boundary.is_empty() {
                trim_boundary = offset(
                    piece_slice,
                    -(0.1 * base_spacing_mm),
                    OffsetJoinType::Round,
                    ARC_TOLERANCE_MM,
                );
            }
            if trim_boundary.is_empty() {
                trim_boundary = piece_slice.to_vec();
            }

            // 9. Seed the accumulated region.
            let seed_region = buffer_open(&seeds, seed_expansion_mm);
            let mut accumulated = intersection(&seed_region, piece_slice);
            if accumulated.is_empty() {
                continue;
            }

            // 10. Propagation loop. Predicate order matches canonical.
            let mut levels: Vec<Vec<Polyline>> = Vec::new();
            let mut iteration = 0u32;
            let mut hit_cap = false;
            loop {
                if params.max_iterations > 0 && iteration >= params.max_iterations {
                    hit_cap = true;
                    break;
                }
                let next_region = intersection(
                    &offset(
                        &accumulated,
                        wave_spacing_mm,
                        OffsetJoinType::Round,
                        ARC_TOLERANCE_MM,
                    ),
                    piece_slice,
                );
                if next_region.is_empty() {
                    break;
                }
                if total_area(&next_region) <= total_area(&accumulated) + min_area_growth {
                    break;
                }
                let tolerance = (0.05 * wave_spacing).min(resolution);
                let mut fronts: Vec<Polyline> =
                    clip_polylines(&to_polylines(&next_region), &trim_boundary)
                        .into_iter()
                        .map(|pl| simplify_polyline(&pl, tolerance))
                        .filter(|pl| pl.len() >= 2)
                        .collect();
                reconnect_polylines(&mut fronts, wave_spacing);
                if !fronts.is_empty() {
                    levels.push(fronts);
                }
                accumulated = next_region;
                iteration += 1;
            }

            // 10b. Close the seam left where opposing fronts merged.
            //      NOTE: deliberately the NOMINAL `flow_width_mm`, not
            //      `deposited_width_mm`. Step 8 asks "how far in must the
            //      centreline stay so the bead lands inside the region?" --- a
            //      deposited-width question. This asks "is the residual strip
            //      between two merged fronts wide enough to deserve another
            //      line?", and its threshold is `CLOSING_MIN_WIDTH_FACTOR` of
            //      the width. At the deposited 0.75 mm the threshold becomes
            //      0.375 mm and skips the real ~0.29 mm seam entirely. Two
            //      different questions; do not harmonise them.
            let closing = closing_pass(&levels, &trim_boundary, flow_width_mm);
            if !closing.is_empty() {
                levels.push(closing);
            }

            if hit_cap {
                out.fallbacks.push(FallbackReason::IterationResidual);
            }
            // Canonical regularizes the filled region before recording it.
            let regularized = offset(
                &offset(
                    &accumulated,
                    regularization_mm,
                    OffsetJoinType::Round,
                    ARC_TOLERANCE_MM,
                ),
                -regularization_mm,
                OffsetJoinType::Round,
                ARC_TOLERANCE_MM,
            );
            filled = union(&filled, &regularized);
            levels_all.extend(levels);
        }
    }

    // 11. Assembly.
    let assembled = if params.pattern == WavePattern::Zigzag {
        append_zig_zag_front_levels(&levels_all, zig_zag_connector_limit)
    } else {
        let flat: Vec<Polyline> = levels_all.into_iter().flatten().collect();
        append_wave_fronts(&flat, flow_width_mm, zig_zag_connector_limit, params.pattern)
    };

    // 12. Drop empty paths; keep the unioned filled region.
    out.paths = assembled.into_iter().filter(|p| p.len() >= 2).collect();
    out.filled = union_ex(&filled);
    if out.paths.is_empty() {
        out.fallbacks.push(FallbackReason::EmptyOutput);
    }
    out
}
