//! Charter `H-OFFSET-MITER`: the production two-pass offset
//! `slicer_core::polygon_ops::offset2_ex` (primary seam) and the single-pass
//! `slicer_core::polygon_ops::offset` (secondary seam), both with
//! `OffsetJoinType::Miter`, against an independent algebraic rectangle model.
//!
//! # Domain
//!
//! Axis-aligned rectangles with whole-millimetre sides `1..=max_side_mm`, and
//! rectangular annuli (an outer rectangle with one rectangular hole, every
//! band and the hole at least 1 mm wide). Whole-mm values become canonical
//! units only through the checked [`whole_mm_to_units`]. Both deltas are whole
//! millimetres in `[-max_offset_mm, max_offset_mm]`; positive, negative,
//! zero and unequal pairs are all generated.
//!
//! # Reference (independent of Clipper)
//!
//! Offsetting an axis-aligned rectangle by `c` units with a miter join moves
//! every edge by `c` along its outward normal: `[x0 - c, x1 + c] x [y0 - c,
//! y1 + c]`. An annulus hole is a clockwise ring, so its edges move the
//! opposite way: `[hx0 + c, hx1 - c] x [hy0 + c, hy1 - c]`. The two-pass model
//! applies this **stage by stage**: a stage whose rectangle has no positive
//! width or height collapses, and a collapsed first stage returns empty
//! (`offset2_ex`'s documented early return). When both stages survive, the
//! result depends only on `d1 + d2`, so surviving rectangles do not by
//! themselves detect a swapped pass order; the deterministic one-mm-square
//! witness in [`deterministic_witnesses`] does (`-1` then `+2` collapses on
//! the first pass; `+2` then `-1` survives as a 3 mm square).
//!
//! # Clearance precondition (checked before the SUT is called)
//!
//! [`require_clearance`] proves that after **each** pass (cumulative `d1`,
//! then `d1 + d2`) every rectangle side plus twice the cumulative delta is
//! positive, every annulus band plus twice the cumulative delta is positive,
//! and every hole side minus twice the cumulative delta is positive. A
//! disappearing shape is therefore a generator/precondition error, never a
//! silently empty comparison.
//!
//! # Exactness (no tolerance in this family)
//!
//! * `d_mm` is an integer with `|d_mm| <= 2 * MAX_OFFSET_MM_CEILING`, exact in
//!   both `f32` (`< 2^24`) and `f64`; production multiplies by
//!   `UNITS_PER_MM = 10_000.0` in `f64`, giving the exact integer
//!   `10_000 * d_mm` (`< 2^53`).
//! * Every edge is axis-aligned, so its unit normal is exactly `(±1, 0)` or
//!   `(0, ±1)`. At a 90° corner the miter vertex is `p + (n1 + n2) * delta`
//!   with `delta / (1 + cos 90°) = delta` exactly; the 90° miter ratio
//!   `sqrt(2)` is below the explicit limit `2.0`, so no corner is squared.
//! * Contracted corners and every cleanup crossing are intersections of one
//!   horizontal and one vertical integer segment: the exact crossing is an
//!   integer point, and any `f64` evaluation of it at magnitudes below
//!   `2 * 10^6` units is within far less than half a unit of it, so the final
//!   round-to-nearest returns the exact integer.
//!
//! Hence the expected region is compared **exactly**: canonicalised output
//! rings must equal the expected rectangles and the checked `i128` double
//! area must equal the expected one. Output rings are validated with #2
//! `validate_ring` and `signed_area2` winding (contours CCW, holes CW).
//!
//! No universal two-pass identity, arbitrary-topology monotonicity or
//! byte-for-byte canonical equivalence is claimed.

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Union};
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::polygon_ops::{offset, offset2_ex, OffsetJoinType};
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::boolean::validate_output_rings;
use super::bounds::{at_least, in_range, max_abs_units, read, to_i64, BoundsError};
use super::envelopes::{region_double_area, whole_mm_to_units, UNITS_PER_MM};
use super::rect::Rect;

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-OFFSET-MITER";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_offset_rectangular_clearance";

/// Explicit Clipper miter limit used for every `offset2_ex` call. The plain
/// `offset` wrapper hard-codes the same value.
pub const MITER_LIMIT: f64 = 2.0;
/// Arc tolerance passed to the single-pass `offset`; a miter join emits no
/// arcs.
pub const ARC_TOLERANCE_MM: f32 = 0.0;
/// Smallest annulus side: band + hole + band, each at least 1 mm.
pub const MIN_ANNULUS_SIDE_MM: u64 = 3;
/// Safety ceiling on `max_side_mm`.
pub const MAX_SIDE_MM_CEILING: u64 = 1_000;
/// Safety ceiling on `max_offset_mm`: keeps `2 * max_offset_mm` exact in
/// `f32` and the cumulative offset far inside the `i64` unit range.
pub const MAX_OFFSET_MM_CEILING: u64 = 1_000;
/// The order witness applies a `+2` mm pass, so the profile must admit it.
pub const MIN_OFFSET_MM: u64 = 2;

// ─────────────────────────────────────────────────────────────────────────────
// Profile validation.
// ─────────────────────────────────────────────────────────────────────────────

/// Raw `H-OFFSET-MITER` registry bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawOffsetBounds {
    /// `max_abs_units`.
    pub max_abs_units: u64,
    /// `max_side_mm`.
    pub max_side_mm: u64,
    /// `max_offset_mm`.
    pub max_offset_mm: u64,
}

impl RawOffsetBounds {
    /// Read every key from the selected registry profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        Ok(Self {
            max_abs_units: read(run, "max_abs_units")?,
            max_side_mm: read(run, "max_side_mm")?,
            max_offset_mm: read(run, "max_offset_mm")?,
        })
    }
}

/// Validated `H-OFFSET-MITER` bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetLimits {
    /// Absolute coordinate bound on every input, intermediate and output.
    pub max_abs_units: i64,
    /// Largest rectangle side, whole mm.
    pub max_side_mm: i64,
    /// Largest per-pass delta magnitude, whole mm.
    pub max_offset_mm: i64,
    /// Whole-mm half span usable for placement: `max_abs_units / 10_000`.
    pub half_span_mm: i64,
}

/// A valid profile used by the controls (not a measured registry profile).
pub fn reference_raw_bounds() -> RawOffsetBounds {
    RawOffsetBounds {
        max_abs_units: 120_000,
        max_side_mm: 8,
        max_offset_mm: 3,
    }
}

/// Validate the `H-OFFSET-MITER` profile before any strategy exists.
///
/// Placement proof: a rectangle with side `<= max_side_mm` and left edge
/// `x0` grows by at most `reach = 2 * max_offset_mm` (cumulative) on each
/// side. Choosing `x0 in [-H + reach, H - reach - max_side_mm]` with
/// `H = max_abs_units / 10_000` keeps every input, intermediate and output
/// coordinate within `±H` mm `<= max_abs_units` units; the range is nonempty
/// iff `2H >= 2 * reach + max_side_mm`, which is required here.
pub fn validate_limits(raw: RawOffsetBounds) -> Result<OffsetLimits, BoundsError> {
    let max_abs = max_abs_units(raw.max_abs_units)?;
    in_range("max_side_mm", raw.max_side_mm, 1..=MAX_SIDE_MM_CEILING)?;
    at_least(
        "max_side_mm",
        raw.max_side_mm,
        "rectangular annulus (band + hole + band)",
        MIN_ANNULUS_SIDE_MM,
    )?;
    in_range(
        "max_offset_mm",
        raw.max_offset_mm,
        1..=MAX_OFFSET_MM_CEILING,
    )?;
    at_least(
        "max_offset_mm",
        raw.max_offset_mm,
        "one-mm-square pass-order witness (+2 mm pass)",
        MIN_OFFSET_MM,
    )?;
    let max_side_mm = to_i64("max_side_mm", raw.max_side_mm)?;
    let max_offset_mm = to_i64("max_offset_mm", raw.max_offset_mm)?;
    let half_span_mm = max_abs / UNITS_PER_MM;
    let needed = max_offset_mm
        .checked_mul(4)
        .and_then(|v| v.checked_add(max_side_mm))
        .ok_or_else(|| BoundsError::Arithmetic {
            what: "placement span".into(),
        })?;
    if half_span_mm.checked_mul(2).is_none_or(|span| span < needed) {
        // Smallest max_abs_units with 2 * floor(max_abs_units / 10_000) >= needed.
        let minimum = whole_mm_to_units((needed + 1) / 2)?;
        return Err(BoundsError::BelowWitness {
            name: "max_abs_units".into(),
            value: raw.max_abs_units,
            witness: "placement of a max_side_mm rectangle grown by 2 * max_offset_mm per side",
            minimum: u64::try_from(minimum).map_err(|_| BoundsError::Arithmetic {
                what: "placement minimum".into(),
            })?,
        });
    }
    let witness_extent =
        deterministic_witness_extent_units().map_err(|what| BoundsError::Arithmetic { what })?;
    at_least(
        "max_abs_units",
        raw.max_abs_units,
        "deterministic offset witnesses",
        witness_extent,
    )?;
    Ok(OffsetLimits {
        max_abs_units: max_abs,
        max_side_mm,
        max_offset_mm,
        half_span_mm,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Shapes and the analytic model.
// ─────────────────────────────────────────────────────────────────────────────

/// An exact-family shape in canonical units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    /// A plain rectangle.
    Rect(Rect),
    /// An outer rectangle with one strictly interior rectangular hole.
    Annulus {
        /// Outer contour.
        outer: Rect,
        /// Hole.
        hole: Rect,
    },
}

impl Shape {
    /// The shape as a well-wound `ExPolygon` (contour CCW, hole CW).
    pub fn to_expolygon(self) -> ExPolygon {
        match self {
            Self::Rect(rect) => rect.to_expolygon(),
            Self::Annulus { outer, hole } => ExPolygon {
                contour: outer.ring(),
                holes: vec![hole.hole_ring()],
            },
        }
    }

    /// Checked double area of the material.
    pub fn double_area(self) -> Result<i128, String> {
        match self {
            Self::Rect(rect) => rect.double_area(),
            Self::Annulus { outer, hole } => outer
                .double_area()?
                .checked_sub(hole.double_area()?)
                .ok_or_else(|| String::from("annulus area overflow")),
        }
    }

    fn max_abs_coordinate(self) -> i64 {
        let rect_max = |r: Rect| {
            [r.x0, r.y0, r.x1, r.y1]
                .into_iter()
                .map(i64::unsigned_abs)
                .max()
                .unwrap_or(0)
        };
        let value = match self {
            Self::Rect(rect) => rect_max(rect),
            Self::Annulus { outer, hole } => rect_max(outer).max(rect_max(hole)),
        };
        i64::try_from(value).unwrap_or(i64::MAX)
    }
}

/// Outcome of one offset stage in the analytic model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expected {
    /// The region vanished.
    Empty,
    /// The region is exactly this shape.
    Shape(Shape),
}

fn grow(rect: Rect, c: i64) -> Result<Option<Rect>, String> {
    let lo = |v: i64| {
        v.checked_sub(c)
            .ok_or_else(|| String::from("offset overflow"))
    };
    let hi = |v: i64| {
        v.checked_add(c)
            .ok_or_else(|| String::from("offset overflow"))
    };
    let (x0, y0, x1, y1) = (lo(rect.x0)?, lo(rect.y0)?, hi(rect.x1)?, hi(rect.y1)?);
    Ok(Rect::new(x0, y0, x1, y1).ok())
}

/// One analytic offset stage by `c` canonical units.
///
/// A rectangle grows by `c` per side (collapsing when its width or height is
/// no longer positive). An annulus' outer contour grows by `c` and its hole
/// shrinks by `c`; a hole that closes leaves the grown outer rectangle. A
/// band that would close (only possible for `c < 0`) leaves a non-rectangular
/// region outside this exact family and is reported as an error.
pub fn stage(shape: Shape, c: i64) -> Result<Expected, String> {
    match shape {
        Shape::Rect(rect) => Ok(match grow(rect, c)? {
            Some(rect) => Expected::Shape(Shape::Rect(rect)),
            None => Expected::Empty,
        }),
        Shape::Annulus { outer, hole } => {
            let neg = c.checked_neg().ok_or("offset overflow")?;
            let grown = grow(outer, c)?.ok_or_else(|| {
                format!("annulus outer {outer:?} collapsed at {c} units: outside the exact family")
            })?;
            match grow(hole, neg)? {
                None => Ok(Expected::Shape(Shape::Rect(grown))),
                Some(shrunk) => {
                    let bands_positive = grown.x0 < shrunk.x0
                        && shrunk.x1 < grown.x1
                        && grown.y0 < shrunk.y0
                        && shrunk.y1 < grown.y1;
                    if !bands_positive {
                        return Err(format!(
                            "annulus band closes at {c} units (outer {grown:?}, hole {shrunk:?}): \
                             outside the exact family"
                        ));
                    }
                    Ok(Expected::Shape(Shape::Annulus {
                        outer: grown,
                        hole: shrunk,
                    }))
                }
            }
        }
    }
}

/// The analytic two-pass result: stage `d1`, then stage `d2` on the
/// intermediate. An empty first stage stays empty.
pub fn expected_two_pass(shape: Shape, d1_mm: i64, d2_mm: i64) -> Result<Expected, String> {
    let d1 = whole_mm_to_units(d1_mm).map_err(|e| e.to_string())?;
    let d2 = whole_mm_to_units(d2_mm).map_err(|e| e.to_string())?;
    match stage(shape, d1)? {
        Expected::Empty => Ok(Expected::Empty),
        Expected::Shape(intermediate) => stage(intermediate, d2),
    }
}

/// The analytic single-pass result.
pub fn expected_single_pass(shape: Shape, d_mm: i64) -> Result<Expected, String> {
    stage(shape, whole_mm_to_units(d_mm).map_err(|e| e.to_string())?)
}

/// The clearance precondition after **each** pass, checked before the SUT is
/// called: every rectangle side plus twice the cumulative delta, every band
/// plus twice the cumulative delta and every hole side minus twice the
/// cumulative delta are strictly positive.
pub fn require_clearance(shape: Shape, d1_mm: i64, d2_mm: i64) -> Result<(), String> {
    let cumulative = [
        d1_mm,
        d1_mm
            .checked_add(d2_mm)
            .ok_or("cumulative delta overflow")?,
    ];
    for (pass, c_mm) in cumulative.into_iter().enumerate() {
        let c = whole_mm_to_units(c_mm).map_err(|e| e.to_string())?;
        let twice = c.checked_mul(2).ok_or("clearance overflow")?;
        let ok = match shape {
            Shape::Rect(r) => r.x1 - r.x0 + twice > 0 && r.y1 - r.y0 + twice > 0,
            Shape::Annulus { outer, hole } => {
                let bands = [
                    hole.x0 - outer.x0,
                    outer.x1 - hole.x1,
                    hole.y0 - outer.y0,
                    outer.y1 - hole.y1,
                ];
                bands.iter().all(|band| band + twice > 0)
                    && hole.x1 - hole.x0 - twice > 0
                    && hole.y1 - hole.y0 - twice > 0
            }
        };
        if !ok {
            return Err(format!(
                "precondition: {shape:?} loses clearance after pass {} (cumulative {c_mm} mm)",
                pass + 1
            ));
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent output checker.
// ─────────────────────────────────────────────────────────────────────────────

fn cross(a: Point2, b: Point2, c: Point2) -> i128 {
    let (abx, aby) = (
        i128::from(b.x) - i128::from(a.x),
        i128::from(b.y) - i128::from(a.y),
    );
    let (acx, acy) = (
        i128::from(c.x) - i128::from(a.x),
        i128::from(c.y) - i128::from(a.y),
    );
    abx * acy - aby * acx
}

/// Reduce a ring to its corner vertices (drop exact repeats and exactly
/// collinear vertices) and require an axis-aligned rectangle.
///
/// Rings may legitimately start anywhere and carry collinear vertices, so the
/// comparison is on the rectangle the corners define, not on ring order.
pub fn canonical_rect(ring: &Polygon) -> Result<Rect, String> {
    let mut points: Vec<Point2> = ring.points.clone();
    points.dedup();
    while points.len() > 1 && points.first() == points.last() {
        points.pop();
    }
    loop {
        let n = points.len();
        if n < 3 {
            return Err(format!("ring {ring:?} has fewer than three corners"));
        }
        let Some(index) =
            (0..n).find(|&i| cross(points[(i + n - 1) % n], points[i], points[(i + 1) % n]) == 0)
        else {
            break;
        };
        points.remove(index);
    }
    if points.len() != 4 {
        return Err(format!(
            "ring has {} corners, not a rectangle: {ring:?}",
            points.len()
        ));
    }
    let n = points.len();
    for i in 0..n {
        let (a, b) = (points[i], points[(i + 1) % n]);
        if a.x != b.x && a.y != b.y {
            return Err(format!("edge {a:?}->{b:?} is not axis-aligned: {ring:?}"));
        }
    }
    let x0 = points.iter().map(|p| p.x).min().ok_or("empty ring")?;
    let x1 = points.iter().map(|p| p.x).max().ok_or("empty ring")?;
    let y0 = points.iter().map(|p| p.y).min().ok_or("empty ring")?;
    let y1 = points.iter().map(|p| p.y).max().ok_or("empty ring")?;
    let rect = Rect::new(x0, y0, x1, y1)?;
    for corner in rect.ring().points {
        if !points.contains(&corner) {
            return Err(format!(
                "corner {corner:?} of {rect:?} is missing from {ring:?}"
            ));
        }
    }
    Ok(rect)
}

/// Observed populations of one checked offset result.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct OffsetStats {
    /// Output components.
    pub components: usize,
    /// Output holes.
    pub holes: usize,
    /// Output material double area (squared units).
    pub double_area: i128,
}

/// Compare one production offset result with the analytic expectation.
pub fn check_offset(
    label: &str,
    expected: Expected,
    output: &[ExPolygon],
) -> Result<OffsetStats, String> {
    validate_output_rings(label, output)?;
    let shape = match expected {
        Expected::Empty => {
            if output.is_empty() {
                return Ok(OffsetStats::default());
            }
            return Err(format!(
                "{label}: expected empty (the region collapses) but the output has {} \
                 component(s): {output:?}",
                output.len()
            ));
        }
        Expected::Shape(shape) => shape,
    };
    if output.len() != 1 {
        return Err(format!(
            "{label}: component count {} != expected 1 for {shape:?}: {output:?}",
            output.len()
        ));
    }
    let got = &output[0];
    let (want_outer, want_hole) = match shape {
        Shape::Rect(rect) => (rect, None),
        Shape::Annulus { outer, hole } => (outer, Some(hole)),
    };
    let outer = canonical_rect(&got.contour).map_err(|e| format!("{label}: contour: {e}"))?;
    if outer != want_outer {
        return Err(format!(
            "{label}: contour rectangle {outer:?} != expected {want_outer:?}"
        ));
    }
    let want_holes = usize::from(want_hole.is_some());
    if got.holes.len() != want_holes {
        return Err(format!(
            "{label}: hole count {} != expected {want_holes} (expected hole {want_hole:?}): {got:?}",
            got.holes.len()
        ));
    }
    if let Some(want_hole) = want_hole {
        let hole = canonical_rect(&got.holes[0]).map_err(|e| format!("{label}: hole: {e}"))?;
        if hole != want_hole {
            return Err(format!(
                "{label}: hole rectangle {hole:?} != expected {want_hole:?}"
            ));
        }
    }
    let want_area = shape.double_area()?;
    let got_area = region_double_area(output)?;
    if got_area != want_area {
        return Err(format!(
            "{label}: double area {got_area} != expected {want_area}"
        ));
    }
    Ok(OffsetStats {
        components: 1,
        holes: want_holes,
        double_area: got_area,
    })
}

/// Run the real two-pass SUT and check it against the analytic model.
pub fn run_two_pass(shape: Shape, d1_mm: i64, d2_mm: i64) -> Result<OffsetStats, String> {
    let expected = expected_two_pass(shape, d1_mm, d2_mm)?;
    let output = offset2_ex(
        &[shape.to_expolygon()],
        d1_mm as f64,
        d2_mm as f64,
        OffsetJoinType::Miter,
        MITER_LIMIT,
    );
    check_offset(
        &format!("offset2_ex({d1_mm} mm, {d2_mm} mm) of {shape:?}"),
        expected,
        &output,
    )
}

/// Run the real single-pass SUT and check it against the analytic model.
pub fn run_single_pass(shape: Shape, d_mm: i64) -> Result<OffsetStats, String> {
    let expected = expected_single_pass(shape, d_mm)?;
    let output = offset(
        &[shape.to_expolygon()],
        d_mm as f32,
        OffsetJoinType::Miter,
        ARC_TOLERANCE_MM,
    );
    check_offset(
        &format!("offset({d_mm} mm) of {shape:?}"),
        expected,
        &output,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy.
// ─────────────────────────────────────────────────────────────────────────────

/// One generated input, in whole millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OffsetInput {
    /// Left edge, mm.
    pub x0: i64,
    /// Bottom edge, mm.
    pub y0: i64,
    /// Width, mm.
    pub w: i64,
    /// Height, mm.
    pub h: i64,
    /// Hole `(left margin, bottom margin, width, height)`, mm, if an annulus.
    pub hole: Option<(i64, i64, i64, i64)>,
    /// First-pass delta, mm.
    pub d1: i64,
    /// Second-pass delta, mm.
    pub d2: i64,
}

impl OffsetInput {
    /// The exact-family shape in canonical units (checked conversion).
    pub fn shape(&self) -> Result<Shape, String> {
        let outer = Rect::from_mm(self.x0, self.y0, self.x0 + self.w, self.y0 + self.h)
            .map_err(|e| e.to_string())?;
        Ok(match self.hole {
            None => Shape::Rect(outer),
            Some((a, b, hw, hh)) => {
                let hole =
                    Rect::from_mm(self.x0 + a, self.y0 + b, self.x0 + a + hw, self.y0 + b + hh)
                        .map_err(|e| e.to_string())?;
                Shape::Annulus { outer, hole }
            }
        })
    }
}

/// The admissible cumulative-delta interval `[lo, hi]` (mm) for a shape.
///
/// Features that shrink as `c` decreases (rectangle sides, annulus bands) need
/// `f + 2c > 0`, i.e. `c >= ceil((1 - f) / 2)`; hole sides need `g - 2c > 0`,
/// i.e. `c <= floor((g - 1) / 2)`. Every feature is at least 1 mm, so `0` is
/// always admissible.
fn admissible(w: i64, h: i64, hole: Option<(i64, i64, i64, i64)>) -> (i64, i64) {
    let shrinking_min = match hole {
        None => w.min(h),
        Some((a, b, hw, hh)) => a.min(b).min(w - a - hw).min(h - b - hh),
    };
    // ceil((1 - f) / 2) for f >= 1 equals -((f - 1) / 2) with truncating division.
    let lo = -((shrinking_min - 1) / 2);
    let hi = match hole {
        None => i64::MAX,
        Some((_, _, hw, hh)) => (hw.min(hh) - 1) / 2,
    };
    (lo, hi)
}

/// Clamp both cumulative deltas into the admissible interval.
///
/// `c1 = clamp(d1)`, `c2 = clamp(c1 + d2)`, `d2' = c2 - c1`. Clamping moves
/// toward `c1`, which is already admissible, so `|d2'| <= |d2| <= max_offset`.
fn clamp_deltas(d1: i64, d2: i64, lo: i64, hi: i64) -> (i64, i64) {
    let c1 = d1.clamp(lo, hi);
    let c2 = (c1 + d2).clamp(lo, hi);
    (c1, c2 - c1)
}

/// Build the `H-OFFSET-MITER` strategy from validated limits.
///
/// Every structural invariant is restored by a deterministic `prop_map` on
/// each generated **and shrunk** value (rectangle deltas are clamped into the
/// admissible interval; annuli are built delta-first by [`annulus_input`]),
/// so shrinking never leaves the exact family and needs no rejection.
pub fn strategy(limits: &OffsetLimits) -> BoxedStrategy<OffsetInput> {
    let m = limits.max_offset_mm;
    let side = limits.max_side_mm;
    let reach = 2 * m;
    let place = (-limits.half_span_mm + reach)..=(limits.half_span_mm - reach - side);
    let rect = (
        (place.clone(), place.clone()),
        (1..=side, 1..=side),
        (-m..=m, -m..=m),
    )
        .prop_map(|((x0, y0), (w, h), (d1, d2))| {
            let (lo, hi) = admissible(w, h, None);
            let (d1, d2) = clamp_deltas(d1, d2, lo, hi);
            OffsetInput {
                x0,
                y0,
                w,
                h,
                hole: None,
                d1,
                d2,
            }
        })
        .boxed();
    let extra = 0..=side;
    let annulus = (
        (place.clone(), place),
        (-m..=m, -m..=m),
        (extra.clone(), extra.clone(), extra.clone()),
        (extra.clone(), extra.clone(), extra),
    )
        .prop_map(move |((x0, y0), (d1, d2), ex, ey)| {
            annulus_input(side, (x0, y0), (d1, d2), ex, ey)
        })
        .boxed();
    Union::new(vec![rect, annulus]).boxed()
}

/// Smallest annulus span (mm) that keeps clearance for every cumulative delta
/// in `[lo, hi]` (`lo <= 0 <= hi`): two bands of `2(-lo) + 1` and a hole of
/// `2 hi + 1`.
fn annulus_span(lo: i64, hi: i64) -> i64 {
    2 * (2 * -lo + 1) + (2 * hi + 1)
}

/// Delta-first annulus construction.
///
/// Drawing the shape first and clamping the deltas afterwards almost always
/// leaves a 1–2 mm band or hole, which admits only `c = 0` (a pilot run of
/// that design produced eight annuli, all with `d1 = d2 = 0`). Here the
/// cumulative deltas `c1 = d1`, `c2 = d1 + d2` come first: the interval
/// `[min(c1, c2, 0), max(c1, c2, 0)]` is narrowed toward 0 (larger magnitude
/// first) until [`annulus_span`] fits `max_side_mm`, both cumulative deltas
/// are clamped into it (clamping into a common interval is 1-Lipschitz, so
/// `|c2 - c1| <= max_offset_mm` still holds), and each band/hole gets its
/// minimum plus a generated extra while the span stays `<= max_side_mm`.
fn annulus_input(
    side: i64,
    origin: (i64, i64),
    raw: (i64, i64),
    ex: (i64, i64, i64),
    ey: (i64, i64, i64),
) -> OffsetInput {
    let (c1, c2) = (raw.0, raw.0 + raw.1);
    let mut lo = c1.min(c2).min(0);
    let mut hi = c1.max(c2).max(0);
    // side >= 3 = annulus_span(0, 0), so this terminates with lo <= 0 <= hi.
    while annulus_span(lo, hi) > side {
        if hi > -lo {
            hi -= 1;
        } else {
            lo += 1;
        }
    }
    let (c1, c2) = (c1.clamp(lo, hi), c2.clamp(lo, hi));
    let band_min = 2 * -lo + 1;
    let hole_min = 2 * hi + 1;
    let axis = |(e_low, e_hole, e_high): (i64, i64, i64)| {
        let mut room = side - annulus_span(lo, hi);
        let mut take = |e: i64| {
            let t = e.min(room);
            room -= t;
            t
        };
        let low = band_min + take(e_low);
        let hole = hole_min + take(e_hole);
        let high = band_min + take(e_high);
        (low, hole, low + hole + high)
    };
    let (a, hw, w) = axis(ex);
    let (b, hh, h) = axis(ey);
    OffsetInput {
        x0: origin.0,
        y0: origin.1,
        w,
        h,
        hole: Some((a, b, hw, hh)),
        d1: c1,
        d2: c2 - c1,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Property body and deterministic witnesses.
// ─────────────────────────────────────────────────────────────────────────────

fn within(limits: &OffsetLimits, shape: Shape, label: &str) -> Result<(), String> {
    if shape.max_abs_coordinate() > limits.max_abs_units {
        return Err(format!(
            "generator invariant: {label} {shape:?} exceeds max_abs_units {}",
            limits.max_abs_units
        ));
    }
    Ok(())
}

/// Check one generated input: precondition, both passes, and both single
/// passes (`d1` and `d1 + d2`, each of which the precondition covers).
pub fn check_input(input: &OffsetInput, limits: &OffsetLimits) -> Result<(), String> {
    if input.d1.abs() > limits.max_offset_mm || input.d2.abs() > limits.max_offset_mm {
        return Err(format!(
            "generator invariant: deltas out of range in {input:?}"
        ));
    }
    let shape = input.shape()?;
    require_clearance(shape, input.d1, input.d2)
        .map_err(|e| format!("generator invariant: {e}"))?;
    within(limits, shape, "input")?;
    for c in [input.d1, input.d1 + input.d2] {
        if let Expected::Shape(s) = expected_single_pass(shape, c)? {
            within(limits, s, "offset result")?;
        } else {
            return Err(format!(
                "generator invariant: {shape:?} collapses at {c} mm"
            ));
        }
    }
    let stats = run_two_pass(shape, input.d1, input.d2)?;
    let want_holes = usize::from(input.hole.is_some());
    if stats.components != 1 || stats.holes != want_holes {
        return Err(format!("unexpected population {stats:?} for {input:?}"));
    }
    run_single_pass(shape, input.d1)?;
    run_single_pass(shape, input.d1 + input.d2)?;
    Ok(())
}

fn mm(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
    Rect::from_mm(x0, y0, x1, y1).expect("whole-mm witness rectangle")
}

/// The one-mm square `[0, 1]²` mm of the pass-order witness.
pub fn order_square() -> Shape {
    Shape::Rect(mm(0, 0, 1, 1))
}

/// Annulus `[0, 12]²` mm with hole `[5, 7]²` mm (5 mm bands, 2 mm hole).
pub fn eroding_annulus() -> Shape {
    Shape::Annulus {
        outer: mm(0, 0, 12, 12),
        hole: mm(5, 5, 7, 7),
    }
}

/// Annulus `[0, 8]²` mm with hole `[2, 6]²` mm (2 mm bands, 4 mm hole).
pub fn growing_annulus() -> Shape {
    Shape::Annulus {
        outer: mm(0, 0, 8, 8),
        hole: mm(2, 2, 6, 6),
    }
}

/// Every deterministic two-pass witness: `(label, shape, d1, d2, expected)`.
/// The expectations are written out by hand, independently of [`stage`].
pub fn two_pass_witnesses() -> Vec<(&'static str, Shape, i64, i64, Expected)> {
    vec![
        (
            "order: -1 then +2 collapses on the first pass",
            order_square(),
            -1,
            2,
            Expected::Empty,
        ),
        (
            "order: +2 then -1 grows first and survives",
            order_square(),
            2,
            -1,
            Expected::Shape(Shape::Rect(mm(-1, -1, 2, 2))),
        ),
        (
            "rect +2 then -1",
            Shape::Rect(mm(0, 0, 4, 3)),
            2,
            -1,
            Expected::Shape(Shape::Rect(mm(-1, -1, 5, 4))),
        ),
        (
            "rect -2 then +1",
            Shape::Rect(mm(0, 0, 6, 5)),
            -2,
            1,
            Expected::Shape(Shape::Rect(mm(1, 1, 5, 4))),
        ),
        (
            "rect zero two-pass",
            Shape::Rect(mm(0, 0, 4, 3)),
            0,
            0,
            Expected::Shape(Shape::Rect(mm(0, 0, 4, 3))),
        ),
        (
            "annulus -2 then +1 (hole grows)",
            eroding_annulus(),
            -2,
            1,
            Expected::Shape(Shape::Annulus {
                outer: mm(1, 1, 11, 11),
                hole: mm(4, 4, 8, 8),
            }),
        ),
        (
            "annulus +1 then -1 (hole shrinks then regrows)",
            growing_annulus(),
            1,
            -1,
            Expected::Shape(growing_annulus()),
        ),
        (
            "annulus +1 then 0 (hole shrinks)",
            growing_annulus(),
            1,
            0,
            Expected::Shape(Shape::Annulus {
                outer: mm(-1, -1, 9, 9),
                hole: mm(3, 3, 5, 5),
            }),
        ),
        (
            "annulus zero two-pass",
            eroding_annulus(),
            0,
            0,
            Expected::Shape(eroding_annulus()),
        ),
    ]
}

/// Every deterministic single-pass witness.
pub fn single_pass_witnesses() -> Vec<(&'static str, Shape, i64, Expected)> {
    vec![
        (
            "rect -1",
            Shape::Rect(mm(0, 0, 4, 4)),
            -1,
            Expected::Shape(Shape::Rect(mm(1, 1, 3, 3))),
        ),
        (
            "rect +1",
            Shape::Rect(mm(0, 0, 4, 4)),
            1,
            Expected::Shape(Shape::Rect(mm(-1, -1, 5, 5))),
        ),
        (
            "rect zero",
            Shape::Rect(mm(0, 0, 4, 4)),
            0,
            Expected::Shape(Shape::Rect(mm(0, 0, 4, 4))),
        ),
        (
            "annulus zero",
            growing_annulus(),
            0,
            Expected::Shape(growing_annulus()),
        ),
        (
            "annulus -1 (hole grows)",
            eroding_annulus(),
            -1,
            Expected::Shape(Shape::Annulus {
                outer: mm(1, 1, 11, 11),
                hole: mm(4, 4, 8, 8),
            }),
        ),
    ]
}

fn expected_extent(expected: Expected) -> i64 {
    match expected {
        Expected::Empty => 0,
        Expected::Shape(shape) => shape.max_abs_coordinate(),
    }
}

/// Largest absolute coordinate, in units, of any deterministic witness input
/// or expected output, derived from the witness list itself.
pub fn deterministic_witness_extent_units() -> Result<u64, String> {
    let two = two_pass_witnesses()
        .into_iter()
        .map(|(_, shape, _, _, want)| shape.max_abs_coordinate().max(expected_extent(want)));
    let one = single_pass_witnesses()
        .into_iter()
        .map(|(_, shape, _, want)| shape.max_abs_coordinate().max(expected_extent(want)));
    let max = two.chain(one).max().unwrap_or(0);
    u64::try_from(max).map_err(|_| String::from("negative witness extent"))
}

/// Deterministic, RNG-independent witnesses run before the generated cases.
pub fn deterministic_witnesses() {
    // Empty input: both seams return empty for nonzero and zero deltas.
    for (d1, d2) in [(1.0, -1.0), (0.0, 0.0)] {
        assert!(
            offset2_ex(&[], d1, d2, OffsetJoinType::Miter, MITER_LIMIT).is_empty(),
            "offset2_ex of the empty set ({d1}, {d2}) must be empty"
        );
    }
    for d in [1.0_f32, -1.0, 0.0] {
        assert!(
            offset(&[], d, OffsetJoinType::Miter, ARC_TOLERANCE_MM).is_empty(),
            "offset of the empty set ({d}) must be empty"
        );
    }
    for (label, shape, d1, d2, want) in two_pass_witnesses() {
        assert_eq!(
            expected_two_pass(shape, d1, d2),
            Ok(want),
            "{label}: the analytic stage model disagrees with the written expectation"
        );
        run_two_pass(shape, d1, d2).unwrap_or_else(|e| panic!("two-pass witness {label}: {e}"));
    }
    for (label, shape, d, want) in single_pass_witnesses() {
        assert_eq!(
            expected_single_pass(shape, d),
            Ok(want),
            "{label}: the analytic stage model disagrees with the written expectation"
        );
        run_single_pass(shape, d).unwrap_or_else(|e| panic!("single-pass witness {label}: {e}"));
    }
}

/// Registry entry point for `proptest_offset_rectangular_clearance`.
pub fn run_property() {
    deterministic_witnesses();
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("use the registry-backed property command");
    let raw = RawOffsetBounds::read(&run).expect("H-OFFSET-MITER bounds");
    let limits = validate_limits(raw).expect("H-OFFSET-MITER profile");
    let strategy = strategy(&limits);
    run.check(&strategy, |input, _guard| -> TestCaseResult {
        check_input(&input, &limits).map_err(TestCaseError::fail)
    })
    .expect("H-OFFSET-MITER charter");
}
