//! Charter `H-BOOL-CLIP`: `slicer_core::polygon_ops::clip_polylines` against
//! an independent full-path interval model.
//!
//! Production pre-inflates the clip universe with
//! `inflate_paths_64(raw_clip_paths, 1.0, JoinType::Miter, EndType::Polygon,
//! 2.0, 0.0)` and documents every reported boundary coordinate within ±2 units
//! of the exact clip boundary. For the rectilinear clips used here that
//! inflation is analytic: every contour edge moves out by [`INFLATE`] and every
//! hole edge moves into the hole by the same amount (miter corners, ratio
//! `sqrt 2 < 2`). The model `R'` is built from the clip with
//! [`offset_rectilinear`], never from Clipper.
//!
//! * **Axis-aligned inputs (exact 1D partition).** Each horizontal/vertical
//!   line carrying an input is partitioned at every input and emitted endpoint,
//!   every crossing of a perpendicular `∂R'` edge, and that crossing ±[`BAND`].
//!   On every elementary interval the number of emitted spans `e` is compared
//!   with the number of covering inputs `k`: `e == k` where the interval is
//!   interior to `R'` outside the band (keep), `e == 0` where it is exterior
//!   outside the band (drop), `e <= k` inside the band or along `∂R'` itself
//!   (side-dependent collinear inclusion). This covers every input span in
//!   full, so a dropped interior span, a shortcut, a duplicated span or an
//!   endpoint more than ±2 units past a crossing is a failure.
//! * **Other inputs (bounded samples).** Every emitted piece must lie within
//!   the documented ±[`BAND`] units of some input line, within its span ±2, and
//!   entirely inside `offset(R, INFLATE + BAND)` by #2 `segment_in_region`
//!   (every boundary meeting of the piece is checked, not one midpoint). Each
//!   input is sampled at `i / max_queries`; samples strictly inside
//!   `offset(R, INFLATE - BAND)` (#2 `point_in_region`) must be covered by an
//!   emitted piece attributed to that input.
//!
//! Emitted pieces are matched to inputs semantically; result order across
//! paths is unspecified and never compared. A non-axis piece is credited to
//! every input whose line lies within `BAND` of both its endpoints (not only
//! the nearest), so the piece lies within `BAND` of that input along its whole
//! length. Limitation: the strategy does not separate non-axis inputs, so two
//! distinct inputs running within `2 * BAND` units of each other along a span
//! (e.g. exact duplicates from the `line_segments` witness set) can share one
//! emitted piece; multiplicity is exact only for axis-aligned lines.

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::polygon_ops::clip_polylines;
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::domains::{
    line_segments, BoundaryRule, GeometryBounds,
};
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, segment_in_region, signed_area2, validate_ring, PointLocation,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::bounds::{
    at_least, in_range, read, to_usize, validate_geometry_limits, BoundsError, GeometryLimits,
    RawGeometryBounds,
};
use super::rect::{edges, is_rectilinear, offset_rectilinear, rings, scale_polygons, Rect};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-BOOL-CLIP";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_clip_polylines_full_segments";

/// Production's clip pre-inflation, canonical units.
pub const INFLATE: i64 = 1;
/// Documented boundary-coordinate allowance, canonical units.
pub const BAND: i64 = 2;
/// Scene grid: `grid = max_abs_units / GRID_DIVISOR`; scenes use `±16 grid`.
pub const GRID_DIVISOR: i64 = 32;
/// Smallest grid step: keeps every feature wider than `2 * (INFLATE + BAND)`.
pub const MIN_GRID: i64 = 16;
/// Smallest accepted `max_abs_units` (`GRID_DIVISOR * MIN_GRID`).
pub const MIN_ABS_UNITS: u64 = (GRID_DIVISOR * MIN_GRID) as u64;
/// Holed scenes need four contour plus four hole vertices.
pub const MIN_VERTICES: u64 = 8;
/// Fewest samples per non-axis input that still put interior samples inside.
pub const MIN_QUERIES: u64 = 8;
/// Deterministic witness paths included in every generated batch.
pub const WITNESS_PATHS: usize = 9;
/// Safety ceiling on paths per call.
pub const MAX_POLYLINES_CEILING: u64 = 256;

// ─────────────────────────────────────────────────────────────────────────────
// Profile validation.
// ─────────────────────────────────────────────────────────────────────────────

/// Validated `H-BOOL-CLIP` bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipLimits {
    /// Shared geometry bounds. `max_shrink_candidates`, `max_components` and
    /// `max_holes` are validated but only `max_abs_units` reaches
    /// `line_segments` (its documented behaviour); the scenes need one
    /// component and at most one hole.
    pub geometry: GeometryLimits,
    /// Paths per `clip_polylines` call, witnesses included.
    pub max_polylines: usize,
}

/// Validate the `H-BOOL-CLIP` profile before any strategy exists.
pub fn validate_limits(
    raw: RawGeometryBounds,
    max_polylines: u64,
) -> Result<ClipLimits, BoundsError> {
    let geometry = validate_geometry_limits(
        raw,
        MIN_ABS_UNITS,
        MIN_VERTICES,
        1,
        1,
        MIN_QUERIES,
        "non-axis span sample",
    )?;
    // Sample coordinates are scaled by max_queries; keep that product exact.
    geometry
        .max_abs_units
        .checked_mul(
            i64::try_from(geometry.max_queries).map_err(|_| BoundsError::Arithmetic {
                what: "max_queries as i64".into(),
            })?,
        )
        .and_then(|v| v.checked_mul(4))
        .ok_or_else(|| BoundsError::Arithmetic {
            what: "scaled sample coordinates".into(),
        })?;
    in_range("max_polylines", max_polylines, 1..=MAX_POLYLINES_CEILING)?;
    at_least(
        "max_polylines",
        max_polylines,
        "deterministic witness paths plus one generated segment",
        WITNESS_PATHS as u64 + 1,
    )?;
    Ok(ClipLimits {
        geometry,
        max_polylines: to_usize("max_polylines", max_polylines)?,
    })
}

/// A valid profile used by the controls (not a measured registry profile).
pub fn reference_raw_bounds() -> (RawGeometryBounds, u64) {
    (
        RawGeometryBounds {
            max_abs_units: 100_000,
            max_vertices: 8,
            max_holes: 1,
            max_components: 1,
            max_shrink_candidates: 64,
            max_queries: 32,
        },
        16,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Scenes and strategy.
// ─────────────────────────────────────────────────────────────────────────────

/// Rectilinear clip family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipShape {
    /// Plain rectangle.
    Rect,
    /// Rectangle minus its upper-right quadrant `[0, x1] x [0, y1]` (concave).
    L,
    /// Rectangle with a rectangular hole `[x0+3g, x1-3g] x [-2g, 2g]`.
    Holed,
}

/// One clip scene on a grid of `grid` units, edges at grid indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClipScene {
    /// Clip family.
    pub shape: ClipShape,
    /// Left edge index, in `-14..=-6`.
    pub x0: i64,
    /// Right edge index, in `6..=14`.
    pub x1: i64,
    /// Bottom edge index, in `-14..=-6`.
    pub y0: i64,
    /// Top edge index, in `6..=14`.
    pub y1: i64,
    /// Grid step in canonical units, at least [`MIN_GRID`].
    pub grid: i64,
}

fn pt(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

impl ClipScene {
    fn u(&self, index: i64) -> i64 {
        index * self.grid
    }

    /// The clip polygons, in canonical units. All coordinates are at most
    /// `14 * grid <= max_abs_units`.
    pub fn clip(&self) -> Vec<ExPolygon> {
        let (x0, x1, y0, y1) = (
            self.u(self.x0),
            self.u(self.x1),
            self.u(self.y0),
            self.u(self.y1),
        );
        match self.shape {
            ClipShape::Rect => vec![Rect::new(x0, y0, x1, y1)
                .expect("scene rect")
                .to_expolygon()],
            ClipShape::L => vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        pt(x0, y0),
                        pt(x1, y0),
                        pt(x1, 0),
                        pt(0, 0),
                        pt(0, y1),
                        pt(x0, y1),
                    ],
                },
                holes: Vec::new(),
            }],
            ClipShape::Holed => {
                let hole = Rect::new(
                    self.u(self.x0 + 3),
                    self.u(-2),
                    self.u(self.x1 - 3),
                    self.u(2),
                )
                .expect("scene hole");
                vec![ExPolygon {
                    contour: Rect::new(x0, y0, x1, y1).expect("scene rect").ring(),
                    holes: vec![hole.hole_ring()],
                }]
            }
        }
    }

    /// The nine deterministic witness paths, independent of the RNG. Every
    /// coordinate is within `18 * grid <= max_abs_units`.
    pub fn witness_paths(&self) -> Vec<Vec<Point2>> {
        let g = self.grid;
        let (x0, x1, y0, y1) = (
            self.u(self.x0),
            self.u(self.x1),
            self.u(self.y0),
            self.u(self.y1),
        );
        let through = match self.shape {
            ClipShape::Holed => vec![pt(x0 + g, 0), pt(x1 - g, 0)],
            ClipShape::L => vec![pt(x1 - g, y0 + g), pt(x1 - g, y1 - g)],
            ClipShape::Rect => vec![pt(x0 - 2 * g, 0), pt(x1 + 2 * g, 0)],
        };
        vec![
            // Fully inside: the bottom strip below the hole and the L notch.
            vec![pt(x0 + g, y0 + g), pt(x1 - g, y0 + g)],
            // Fully outside, far beyond the band.
            vec![pt(x0, y1 + 4 * g), pt(x1, y1 + 4 * g)],
            // Crossing the bottom edge once.
            vec![pt(x0 + g, y0 - 3 * g), pt(x0 + g, y0 + 2 * g)],
            // Riding the original bottom boundary (inside R').
            vec![pt(x0, y0), pt(x1, y0)],
            // Through the hole / notch / both sides.
            through,
            // Multi-vertex polyline: up the left arm, across the top.
            vec![pt(x0 + g, y0 + g), pt(x0 + g, y1 - g), pt(x1 - g, y1 - g)],
            // Non-axis-aligned crossing.
            vec![pt(x0 - 2 * g, y0 - 2 * g), pt(x1 + 2 * g, y1 + 2 * g)],
            // Short path: skipped by production.
            vec![pt(x0 + 2 * g, y0 + 2 * g)],
            // Empty path: skipped by production.
            Vec::new(),
        ]
    }

    /// A point strictly inside the clip material.
    pub fn inside_point(&self) -> Point2 {
        pt(
            self.u(self.x0) + 2 * self.grid,
            self.u(self.y0) + 2 * self.grid,
        )
    }
}

/// One generated `H-BOOL-CLIP` input.
#[derive(Debug, Clone)]
pub struct ClipInput {
    /// Clip scene.
    pub scene: ClipScene,
    /// Generated nonzero-length segments.
    pub segments: Vec<[Point2; 2]>,
}

fn axis_segment(m: i64) -> BoxedStrategy<[Point2; 2]> {
    (any::<bool>(), -m..=m, -m..m, 1..=2 * m)
        .prop_map(move |(horizontal, c, a, len)| {
            // a < m and len >= 1, so b > a: nonzero for every shrunk value.
            let b = (a + len).min(m);
            if horizontal {
                [pt(a, c), pt(b, c)]
            } else {
                [pt(c, a), pt(c, b)]
            }
        })
        .boxed()
}

/// #2 `line_segments` wrapped so it never emits a zero-length segment.
///
/// The map runs on every value the tree exposes, including shrunk ones, so
/// the nonzero invariant cannot be lost by shrinking; `|x| <= m` is kept.
pub fn nonzero_line_segments(
    bounds: GeometryBounds,
) -> Result<BoxedStrategy<[Point2; 2]>, BoundsError> {
    let m = bounds.max_abs_units;
    let inner = line_segments(bounds).map_err(|error| BoundsError::Arithmetic {
        what: format!("line_segments: {error}"),
    })?;
    Ok(inner
        .prop_map(move |[p, q]| {
            if p == q {
                let x = if p.x < m { p.x + 1 } else { p.x - 1 };
                [p, pt(x, p.y)]
            } else {
                [p, q]
            }
        })
        .boxed())
}

/// Build the `H-BOOL-CLIP` strategy. Bound failures surface before sampling.
pub fn strategy(limits: &ClipLimits) -> Result<BoxedStrategy<ClipInput>, BoundsError> {
    let geometry = limits.geometry;
    let grid = geometry.max_abs_units / GRID_DIVISOR;
    if grid < MIN_GRID {
        return Err(BoundsError::Arithmetic {
            what: format!("scene grid {grid} below {MIN_GRID}"),
        });
    }
    // exhaustive: every measured bound is passed through, none defaulted
    let bounds = GeometryBounds {
        max_abs_units: geometry.max_abs_units,
        max_vertices: geometry.max_vertices,
        max_holes: geometry.max_holes,
        max_components: geometry.max_components,
        max_shrink_candidates: geometry.max_shrink_candidates,
    };
    let segment = prop_oneof![
        axis_segment(geometry.max_abs_units),
        nonzero_line_segments(bounds)?,
    ];
    let generated = limits.max_polylines - WITNESS_PATHS;
    let scene = (0_u8..3, -14_i64..=-6, 6_i64..=14, -14_i64..=-6, 6_i64..=14).prop_map(
        move |(shape, x0, x1, y0, y1)| ClipScene {
            shape: match shape {
                0 => ClipShape::Rect,
                1 => ClipShape::L,
                _ => ClipShape::Holed,
            },
            x0,
            x1,
            y0,
            y1,
            grid,
        },
    );
    Ok((scene, proptest::collection::vec(segment, 0..=generated))
        .prop_map(|(scene, segments)| ClipInput { scene, segments })
        .boxed())
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent model and checker.
// ─────────────────────────────────────────────────────────────────────────────

/// Analytic regions derived from the clip, never from Clipper.
#[derive(Debug, Clone)]
pub struct ClipModel {
    /// `R' = offset(R, INFLATE)`: production's inflated universe.
    pub expanded: Vec<ExPolygon>,
    /// `R'` with doubled coordinates, for half-integer interval midpoints.
    expanded_x2: Vec<ExPolygon>,
    /// `offset(R, INFLATE - BAND)`: certainly retained.
    pub keep: Vec<ExPolygon>,
    /// `offset(R, INFLATE + BAND)`: never exceeded.
    pub allowed: Vec<ExPolygon>,
}

fn check_wound(label: &str, polygons: &[ExPolygon]) -> Result<(), String> {
    for polygon in polygons {
        validate_ring(&polygon.contour).map_err(|e| format!("{label} contour: {e}"))?;
        if signed_area2(&polygon.contour).map_err(|e| e.to_string())? <= 0 {
            return Err(format!("{label} contour is not counter-clockwise"));
        }
        for hole in &polygon.holes {
            validate_ring(hole).map_err(|e| format!("{label} hole: {e}"))?;
            if signed_area2(hole).map_err(|e| e.to_string())? >= 0 {
                return Err(format!("{label} hole is not clockwise"));
            }
        }
    }
    Ok(())
}

/// Build the analytic model for a rectilinear, well-wound clip.
pub fn clip_model(clip: &[ExPolygon]) -> Result<ClipModel, String> {
    if !is_rectilinear(clip) {
        return Err(String::from("the interval model needs a rectilinear clip"));
    }
    check_wound("clip", clip)?;
    let expanded = offset_rectilinear(clip, INFLATE)?;
    let keep = offset_rectilinear(clip, INFLATE - BAND)?;
    let allowed = offset_rectilinear(clip, INFLATE + BAND)?;
    check_wound("expanded", &expanded)?;
    check_wound("keep", &keep)?;
    check_wound("allowed", &allowed)?;
    Ok(ClipModel {
        expanded_x2: scale_polygons(&expanded, 2)?,
        expanded,
        keep,
        allowed,
    })
}

/// Observed populations of one clip check.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ClipStats {
    /// Input length (times multiplicity) in certainly-retained intervals.
    pub keep_len: i128,
    /// Input length (times multiplicity) in certainly-omitted intervals.
    pub drop_len: i128,
    /// Input length in the boundary band.
    pub band_len: i128,
    /// Non-axis samples that had to be covered.
    pub diag_keep_samples: usize,
    /// Non-axis samples certainly outside.
    pub diag_drop_samples: usize,
    /// Nonzero emitted pieces matched to an input.
    pub matched_pieces: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Line {
    /// `y = c`, along-coordinate `x`.
    Horizontal(i64),
    /// `x = c`, along-coordinate `y`.
    Vertical(i64),
}

fn axis_line(s: [Point2; 2]) -> Option<(Line, i64, i64)> {
    let [p, q] = s;
    if p == q {
        None
    } else if p.y == q.y {
        Some((Line::Horizontal(p.y), p.x.min(q.x), p.x.max(q.x)))
    } else if p.x == q.x {
        Some((Line::Vertical(p.x), p.y.min(q.y), p.y.max(q.y)))
    } else {
        None
    }
}

fn path_segments(path: &[Point2]) -> Vec<[Point2; 2]> {
    path.windows(2).map(|w| [w[0], w[1]]).collect()
}

fn loc(point: Point2, polygons: &[ExPolygon]) -> Result<PointLocation, String> {
    point_in_region(point, polygons).map_err(|e| format!("point_in_region: {e}"))
}

fn checked(value: Option<i128>) -> Result<i128, String> {
    value.ok_or_else(|| String::from("clip model arithmetic overflow"))
}

/// `(cross, dot, len²)` of `q` relative to segment `p0 -> p1`.
fn relate(p0: Point2, p1: Point2, q: Point2) -> Result<(i128, i128, i128), String> {
    let (dx, dy) = (
        i128::from(p1.x) - i128::from(p0.x),
        i128::from(p1.y) - i128::from(p0.y),
    );
    let (qx, qy) = (
        i128::from(q.x) - i128::from(p0.x),
        i128::from(q.y) - i128::from(p0.y),
    );
    let cross = checked(
        qx.checked_mul(dy)
            .and_then(|l| qy.checked_mul(dx).and_then(|r| l.checked_sub(r))),
    )?;
    let dot = checked(
        qx.checked_mul(dx)
            .and_then(|l| qy.checked_mul(dy).and_then(|r| l.checked_add(r))),
    )?;
    let len_sq = checked(
        dx.checked_mul(dx)
            .and_then(|l| dy.checked_mul(dy).and_then(|r| l.checked_add(r))),
    )?;
    Ok((cross, dot, len_sq))
}

/// Whether `q` is within `BAND` units of the input line and its projection is
/// within the input span extended by `BAND` units.
///
/// Justification: `clip_polylines` documents that, after its one-unit
/// pre-inflation, every reported boundary coordinate is within ±2 units
/// (`BAND`) of the exact clip boundary; it promises nothing tighter about how
/// far a computed crossing may sit from the input line. All three tests are
/// exact `i128` comparisons without square roots: perpendicular distance
/// `|cross| / |D| <= BAND` is `cross² <= BAND² · |D|²`.
pub fn near_input(p0: Point2, p1: Point2, q: Point2) -> Result<bool, String> {
    let (cross, dot, len_sq) = relate(p0, p1, q)?;
    let band_sq = i128::from(BAND * BAND);
    if checked(cross.checked_mul(cross))? > checked(band_sq.checked_mul(len_sq))? {
        return Ok(false);
    }
    if dot < 0 {
        return Ok(checked(dot.checked_mul(dot))? <= checked(band_sq.checked_mul(len_sq))?);
    }
    if dot > len_sq {
        let over = dot - len_sq;
        return Ok(checked(over.checked_mul(over))? <= checked(band_sq.checked_mul(len_sq))?);
    }
    Ok(true)
}

/// The independent full-path check of one `clip_polylines` call.
pub fn check_clip(
    inputs: &[Vec<Point2>],
    clip: &[ExPolygon],
    emitted: &[Vec<Point2>],
    max_queries: usize,
) -> Result<ClipStats, String> {
    let model = clip_model(clip)?;
    let mut stats = ClipStats::default();

    let input_segments: Vec<[Point2; 2]> = inputs
        .iter()
        .filter(|path| path.len() >= 2)
        .flat_map(|path| path_segments(path))
        .collect();
    if input_segments.iter().any(|[p, q]| p == q) {
        return Err(String::from(
            "model precondition: zero-length input segment",
        ));
    }
    let axis_inputs: Vec<(Line, i64, i64)> = input_segments
        .iter()
        .filter_map(|s| axis_line(*s))
        .collect();
    let diag_inputs: Vec<[Point2; 2]> = input_segments
        .iter()
        .copied()
        .filter(|s| axis_line(*s).is_none())
        .collect();

    // Attribute every emitted piece semantically, never by position.
    let mut axis_emitted: Vec<(Line, i64, i64)> = Vec::new();
    let mut diag_emitted: Vec<Vec<[Point2; 2]>> = vec![Vec::new(); diag_inputs.len()];
    for (index, path) in emitted.iter().enumerate() {
        if path.len() < 2 {
            return Err(format!(
                "emitted path {index} has fewer than two points: {path:?}"
            ));
        }
        for &vertex in path {
            if loc(vertex, &model.allowed)? == PointLocation::Outside {
                return Err(format!(
                    "emitted vertex {vertex:?} lies beyond the ±{BAND}-unit allowance of R'"
                ));
            }
        }
        for piece in path_segments(path) {
            if piece[0] == piece[1] {
                continue;
            }
            if let Some((line, lo, hi)) = axis_line(piece) {
                if axis_inputs
                    .iter()
                    .any(|&(l, ilo, ihi)| l == line && ilo <= lo && hi <= ihi)
                {
                    axis_emitted.push((line, lo, hi));
                    stats.matched_pieces += 1;
                    continue;
                }
            }
            let mut matched = false;
            for (slot, &[p0, p1]) in diag_inputs.iter().enumerate() {
                if near_input(p0, p1, piece[0])? && near_input(p0, p1, piece[1])? {
                    diag_emitted[slot].push(piece);
                    matched = true;
                }
            }
            if !matched {
                return Err(format!(
                    "emitted piece {piece:?} lies on no input segment: a shortcut or invented span"
                ));
            }
            if !segment_in_region(piece, &model.allowed, BoundaryRule::Inclusive)
                .map_err(|e| e.to_string())?
            {
                return Err(format!(
                    "emitted piece {piece:?} leaves R' by more than the ±{BAND}-unit allowance"
                ));
            }
            stats.matched_pieces += 1;
        }
    }

    // Exact 1D partition of every axis line.
    let mut lines: Vec<Line> = axis_inputs.iter().map(|&(l, _, _)| l).collect();
    lines.sort_unstable();
    lines.dedup();
    let r_edges = edges(&model.expanded);
    for line in lines {
        let ins: Vec<(i64, i64)> = axis_inputs
            .iter()
            .filter(|&&(l, _, _)| l == line)
            .map(|&(_, lo, hi)| (lo, hi))
            .collect();
        let outs: Vec<(i64, i64)> = axis_emitted
            .iter()
            .filter(|&&(l, _, _)| l == line)
            .map(|&(_, lo, hi)| (lo, hi))
            .collect();
        let crossings: Vec<i64> = r_edges
            .iter()
            .filter_map(|&(a, b)| match line {
                Line::Horizontal(c) if a.x == b.x && a.y.min(b.y) <= c && c <= a.y.max(b.y) => {
                    Some(a.x)
                }
                Line::Vertical(c) if a.y == b.y && a.x.min(b.x) <= c && c <= a.x.max(b.x) => {
                    Some(a.y)
                }
                _ => None,
            })
            .collect();
        let mut breaks: Vec<i64> = Vec::new();
        for &(lo, hi) in ins.iter().chain(outs.iter()) {
            breaks.extend([lo, hi]);
        }
        for &c in &crossings {
            breaks.extend([c - BAND, c, c + BAND]);
        }
        breaks.sort_unstable();
        breaks.dedup();
        for pair in breaks.windows(2) {
            let (p, q) = (pair[0], pair[1]);
            let k = ins.iter().filter(|&&(lo, hi)| lo <= p && q <= hi).count();
            let e = outs.iter().filter(|&&(lo, hi)| lo <= p && q <= hi).count();
            if k == 0 && e == 0 {
                continue;
            }
            let mid2 = p + q;
            let length = i128::from(q - p) * k as i128;
            let in_band = crossings.iter().any(|&c| (mid2 - 2 * c).abs() < 2 * BAND);
            let probe = match line {
                Line::Horizontal(c) => pt(mid2, 2 * c),
                Line::Vertical(c) => pt(2 * c, mid2),
            };
            let zone = if in_band {
                PointLocation::Boundary
            } else {
                loc(probe, &model.expanded_x2)?
            };
            let ok = match zone {
                PointLocation::Inside => {
                    stats.keep_len += length;
                    e == k
                }
                PointLocation::Outside => {
                    stats.drop_len += length;
                    e == 0
                }
                PointLocation::Boundary => {
                    stats.band_len += length;
                    e <= k
                }
            };
            if !ok {
                return Err(format!(
                    "{line:?} interval [{p}, {q}] is {zone:?} for R' (band {in_band}): \
                     {k} input span(s) but {e} emitted span(s); inputs {ins:?}, emitted {outs:?}"
                ));
            }
        }
    }

    // Bounded samples of every non-axis input.
    let n = i64::try_from(max_queries).map_err(|_| String::from("max_queries as i64"))?;
    let keep_n = scale_polygons(&model.keep, n)?;
    let allowed_n = scale_polygons(&model.allowed, n)?;
    for (slot, &[p0, p1]) in diag_inputs.iter().enumerate() {
        let (_, _, len_sq) = relate(p0, p1, p1)?;
        let pieces: Vec<(i128, i128)> = diag_emitted[slot]
            .iter()
            .map(|&[a, b]| {
                let (_, da, _) = relate(p0, p1, a)?;
                let (_, db, _) = relate(p0, p1, b)?;
                Ok((da.min(db), da.max(db)))
            })
            .collect::<Result<_, String>>()?;
        for i in 0..=n {
            let coordinate = |start: i64, end: i64| {
                start
                    .checked_mul(n)
                    .and_then(|s| (end - start).checked_mul(i).and_then(|d| s.checked_add(d)))
                    .ok_or_else(|| String::from("sample coordinate overflow"))
            };
            let sample = pt(coordinate(p0.x, p1.x)?, coordinate(p0.y, p1.y)?);
            if loc(sample, &keep_n)? == PointLocation::Inside {
                stats.diag_keep_samples += 1;
                let target = checked(i128::from(i).checked_mul(len_sq))?;
                let covered = pieces.iter().any(|&(lo, hi)| {
                    lo.checked_mul(i128::from(n)).is_some_and(|l| l <= target)
                        && hi.checked_mul(i128::from(n)).is_some_and(|h| target <= h)
                });
                if !covered {
                    return Err(format!(
                        "input {:?} sample {i}/{n} is certainly inside R' but no emitted piece \
                         covers it (dropped span); pieces {:?}",
                        [p0, p1],
                        diag_emitted[slot]
                    ));
                }
            } else if loc(sample, &allowed_n)? == PointLocation::Outside {
                stats.diag_drop_samples += 1;
            }
        }
    }
    Ok(stats)
}

/// An empty path and a one-point path inside the clip produce no output
/// (production skips paths with fewer than two points).
pub fn check_short_paths_skipped(scene: &ClipScene) -> Result<(), String> {
    let clip = scene.clip();
    let result = clip_polylines(&[Vec::new(), vec![scene.inside_point()]], &clip);
    if result.is_empty() {
        Ok(())
    } else {
        Err(format!("short/empty paths produced output {result:?}"))
    }
}

fn check_input(input: &ClipInput, limits: &ClipLimits) -> Result<ClipStats, String> {
    let clip = input.scene.clip();
    let mut paths = input.scene.witness_paths();
    paths.extend(input.segments.iter().map(|s| s.to_vec()));
    if paths.len() > limits.max_polylines {
        return Err(format!("{} paths exceed max_polylines", paths.len()));
    }
    for ring in rings(&clip) {
        for p in &ring.points {
            if p.x.abs() > limits.geometry.max_abs_units
                || p.y.abs() > limits.geometry.max_abs_units
            {
                return Err(format!(
                    "generator invariant: clip vertex {p:?} out of bounds"
                ));
            }
        }
    }
    let emitted = clip_polylines(&paths, &clip);
    let stats = check_clip(&paths, &clip, &emitted, limits.geometry.max_queries)?;
    if stats.keep_len == 0
        || stats.drop_len == 0
        || stats.diag_keep_samples == 0
        || stats.diag_drop_samples == 0
        || stats.matched_pieces == 0
    {
        return Err(format!("vacuous clip populations {stats:?}"));
    }
    check_short_paths_skipped(&input.scene)?;
    Ok(stats)
}

/// Fixed scenes (1 mm grid) for every clip family.
pub fn fixed_scenes() -> [ClipScene; 3] {
    let base = |shape| ClipScene {
        shape,
        x0: -10,
        x1: 10,
        y0: -10,
        y1: 10,
        grid: 10_000,
    };
    [
        base(ClipShape::Rect),
        base(ClipShape::L),
        base(ClipShape::Holed),
    ]
}

/// Deterministic, RNG-independent witnesses run before the generated cases.
pub fn deterministic_witnesses() {
    let limits = ClipLimits {
        geometry: GeometryLimits {
            max_abs_units: 1_000_000,
            max_vertices: 8,
            max_holes: 1,
            max_components: 1,
            max_shrink_candidates: 1,
            max_queries: 64,
        },
        max_polylines: WITNESS_PATHS,
    };
    for scene in fixed_scenes() {
        let input = ClipInput {
            scene,
            segments: Vec::new(),
        };
        let stats = check_input(&input, &limits)
            .unwrap_or_else(|error| panic!("clip witness {:?}: {error}", scene.shape));
        assert!(stats.keep_len > 0 && stats.drop_len > 0, "{stats:?}");
        // The through-feature path splits around a hole, exits a notch once,
        // and is clipped on both sides of a plain rectangle.
        let through = scene.witness_paths()[4].clone();
        let pieces = clip_polylines(&[through], &scene.clip());
        let expected = match scene.shape {
            ClipShape::Holed => 2,
            ClipShape::L | ClipShape::Rect => 1,
        };
        assert_eq!(
            pieces.len(),
            expected,
            "{:?} through-feature pieces {pieces:?}",
            scene.shape
        );
    }
}

/// Registry entry point for `proptest_clip_polylines_full_segments`.
pub fn run_property() {
    deterministic_witnesses();
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("use the registry-backed property command");
    let raw = RawGeometryBounds::read(&run).expect("H-BOOL-CLIP bounds");
    let max_polylines = read(&run, "max_polylines").expect("max_polylines bound");
    let limits = validate_limits(raw, max_polylines).expect("H-BOOL-CLIP profile");
    let strategy = strategy(&limits).expect("H-BOOL-CLIP strategy");
    run.check(&strategy, |input, _guard| -> TestCaseResult {
        check_input(&input, &limits)
            .map(|_| ())
            .map_err(TestCaseError::fail)
    })
    .expect("H-BOOL-CLIP charter");
}
