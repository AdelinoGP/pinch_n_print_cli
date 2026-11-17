//! Charter `H-OFFSET-SMOOTH`: `slicer_core::smooth_outward::smooth_outward_polygon`
//! (primary seam; `smooth_outward_ex` is exercised by one deterministic
//! witness) on two safe families with independently derived expectations,
//! plus a separate tiny clockwise deletion witness.
//!
//! `clip_dist_scaled` is **integer grid units** and is the **chord** of the
//! inserted clipping edge, not the per-arm setback. The production
//! near-duplicate prepass removes edges shorter than 100 units.
//!
//! # Convex family
//!
//! CCW axis-aligned rectangles whose every side is at least
//! `chord + EXTRA_MIN_UNITS` (so far above the 100-unit prepass threshold and
//! strictly longer than the chord). A convex corner is never a clipping
//! candidate, so the result must be **geometrically unchanged**: the same
//! vertices in the same cyclic order and orientation, no vertex added or
//! removed, identical signed double area. The ring is compared as a cyclic
//! sequence: the start index is not part of the contract (the input itself is
//! generated with every start rotation).
//!
//! # L family
//!
//! A CCW L with exactly one 90° reflex corner `R`, arms of length `la`, `lb`
//! and leg thicknesses `a`, `b`, every one at least `chord + EXTRA_MIN_UNITS`,
//! in all four quarter-turn orientations and every ring start. Independent
//! derivation: the chamfer replaces `R` by `A = R + k u1` and `B = R + k u2`
//! (`u1`, `u2` the unit arm directions toward the previous and next vertex),
//! the inserted edge `A -> B` is the chord, so the real setback is
//! `s* = chord / sqrt(2)`.
//!
//! **Setback bound.** Production evaluates `s*` in `f64` with at most five
//! rounded operations after exact steps (one division, one square root, one
//! product on the shorter arm; two more products/quotients on the longer
//! arm), so the computed `s^` satisfies `|s^ - s*| <= delta = gamma(5) * s*`,
//! `gamma(5) = 5u / (1 - 5u)`, `u = 2^-53` (≈ `5.55e-16`). It then truncates
//! toward zero, so the integer setback `k` lies in
//! `(s* - 1 - delta, s* + delta]`. Because `chord^2 = 2 k^2` has no positive
//! integer solution, `s*` is irrational and
//! `|s* - k| = |chord^2 - 2k^2| / (sqrt(2) (chord + sqrt(2) k)) >= 1 / (2 sqrt(2) chord)`.
//! At the ceiling `chord <= CLIP_CEILING = 10^6` this is `>= 3.5e-7` units
//! while `delta <= 3.9e-10` units, so the interval holds exactly one integer,
//! `k = floor(chord / sqrt(2))`, which is checked **exactly** in integers:
//! `2 k^2 < chord^2 < 2 (k + 1)^2`. The perpendicular coordinate of each
//! chamfer point is `0 * t = 0` exactly. The checker also requires the
//! reflex vertex gone, the other five corners present, exactly one new
//! vertex per arm, a valid CCW ring, the vertex count grown by exactly one,
//! the exact cyclic ring `[.., prev, A, B, next, ..]` and the exact double
//! area `input + k^2`.
//!
//! # Tiny clockwise deletion
//!
//! A separate concrete witness: a 2000-unit CW square with chord 5000 is a
//! hole to be filled; `clip_narrow_corner` reduces it below three points and
//! the ring is deleted. This is asserted as the documented outcome, not
//! generalised. The disputed `DEV-141` far/blocked branch is never used as a
//! byte oracle, and no blanket superset or arbitrary-topology claim is made.

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Union};
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::smooth_outward::{smooth_outward_ex, smooth_outward_polygon};
use slicer_ir::{ExPolygon, Point2, Polygon};
use slicer_sdk::test_support::property_testing::oracles::{signed_area2, validate_ring};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::bounds::{at_least, in_range, max_abs_units, read, to_i64, BoundsError};

/// Registry identity of this charter.
pub const PROPERTY_ID: &str = "H-OFFSET-SMOOTH";
/// Exact registry test name.
pub const TEST_NAME: &str = "proptest_smooth_outward_safe_families";

/// Production's near-duplicate prepass epsilon, canonical units (0.01 mm).
pub const PREPASS_EPS_UNITS: i64 = 100;
/// Every generated edge/feature is at least `chord + EXTRA_MIN_UNITS` long:
/// ten times the prepass epsilon and strictly longer than the chord.
pub const EXTRA_MIN_UNITS: i64 = 1_000;
/// Smallest generated chord. For a 90° corner with arms longer than the
/// chord the obtuse branch needs `la^2 + lb^2 > (chord + 1)^2` (true for
/// `chord >= 3`) and `min(la, lb)^2 / 2 >= chord^2 / 4 + 1` (true for
/// `chord >= 2`); 16 leaves margin.
pub const MIN_CHORD_UNITS: u64 = 16;
/// Safety ceiling on `max_clip_units`, where the setback proof above holds.
pub const CLIP_CEILING_UNITS: u64 = 1_000_000;
/// Chord of the deterministic L and tiny-deletion witnesses.
pub const WITNESS_CHORD: i64 = 5_000;

// ─────────────────────────────────────────────────────────────────────────────
// Profile validation.
// ─────────────────────────────────────────────────────────────────────────────

/// Raw `H-OFFSET-SMOOTH` registry bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawSmoothBounds {
    /// `max_abs_units`.
    pub max_abs_units: u64,
    /// `max_side_units`.
    pub max_side_units: u64,
    /// `max_clip_units`.
    pub max_clip_units: u64,
}

impl RawSmoothBounds {
    /// Read every key from the selected registry profile.
    pub fn read(run: &PropertyRun) -> Result<Self, BoundsError> {
        Ok(Self {
            max_abs_units: read(run, "max_abs_units")?,
            max_side_units: read(run, "max_side_units")?,
            max_clip_units: read(run, "max_clip_units")?,
        })
    }
}

/// Validated `H-OFFSET-SMOOTH` bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmoothLimits {
    /// Absolute coordinate bound on every input and output.
    pub max_abs_units: i64,
    /// Largest local extent of a generated ring.
    pub max_side_units: i64,
    /// Largest chord.
    pub max_clip_units: i64,
}

/// A valid profile used by the controls (not a measured registry profile).
pub fn reference_raw_bounds() -> RawSmoothBounds {
    RawSmoothBounds {
        max_abs_units: 120_000,
        max_side_units: 80_000,
        max_clip_units: 8_000,
    }
}

/// Validate the `H-OFFSET-SMOOTH` profile before any strategy exists.
///
/// `max_side_units >= 2 * (max_clip_units + EXTRA_MIN_UNITS)` leaves room for
/// an L whose two legs and two arms all exceed the chord;
/// `max_side_units <= max_abs_units` lets every quarter-turned ring (local
/// extent `±max_side_units`) be placed within `±max_abs_units`. The bounds
/// must also admit every deterministic witness.
pub fn validate_limits(raw: RawSmoothBounds) -> Result<SmoothLimits, BoundsError> {
    let max_abs = max_abs_units(raw.max_abs_units)?;
    in_range(
        "max_clip_units",
        raw.max_clip_units,
        MIN_CHORD_UNITS..=CLIP_CEILING_UNITS,
    )?;
    let max_clip = to_i64("max_clip_units", raw.max_clip_units)?;
    in_range("max_side_units", raw.max_side_units, 1..=raw.max_abs_units)?;
    let l_room = max_clip
        .checked_add(EXTRA_MIN_UNITS)
        .and_then(|v| v.checked_mul(2))
        .ok_or_else(|| BoundsError::Arithmetic {
            what: "L family room".into(),
        })?;
    at_least(
        "max_side_units",
        raw.max_side_units,
        "L with legs and arms longer than max_clip_units",
        u64::try_from(l_room).map_err(|_| BoundsError::Arithmetic {
            what: "L family room".into(),
        })?,
    )?;
    let max_side = to_i64("max_side_units", raw.max_side_units)?;
    let (witness_side, witness_abs) =
        witness_extents().map_err(|what| BoundsError::Arithmetic { what })?;
    at_least(
        "max_side_units",
        raw.max_side_units,
        "deterministic smoothing witnesses",
        witness_side,
    )?;
    at_least(
        "max_abs_units",
        raw.max_abs_units,
        "deterministic smoothing witnesses",
        witness_abs,
    )?;
    at_least(
        "max_clip_units",
        raw.max_clip_units,
        "deterministic smoothing witness chord",
        WITNESS_CHORD as u64,
    )?;
    Ok(SmoothLimits {
        max_abs_units: max_abs,
        max_side_units: max_side,
        max_clip_units: max_clip,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Ring helpers.
// ─────────────────────────────────────────────────────────────────────────────

fn pt(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn ring(points: Vec<Point2>) -> Polygon {
    Polygon { points }
}

/// Quarter-turn `(x, y) -> (-y, x)` applied `turns` times.
fn rotate(p: Point2, turns: u8) -> Point2 {
    (0..turns % 4).fold(p, |q, _| pt(-q.y, q.x))
}

fn place(points: &[Point2], turns: u8, tx: i64, ty: i64, start: usize) -> Polygon {
    let mut out: Vec<Point2> = points
        .iter()
        .map(|&p| {
            let r = rotate(p, turns);
            pt(r.x + tx, r.y + ty)
        })
        .collect();
    let len = out.len();
    out.rotate_left(start % len);
    ring(out)
}

/// Whether `b` is a cyclic rotation of `a` (same orientation).
pub fn is_cyclic_rotation(a: &[Point2], b: &[Point2]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    if a.is_empty() {
        return true;
    }
    (0..b.len()).any(|shift| (0..a.len()).all(|i| a[i] == b[(i + shift) % b.len()]))
}

/// Largest `k` with `2 k^2 < chord^2`, by exact integer search.
///
/// Since `chord^2 = 2 k^2` has no positive solution, this is
/// `floor(chord / sqrt(2))`.
pub fn floor_setback(chord: i64) -> Result<i64, String> {
    let c2 = i128::from(chord)
        .checked_mul(i128::from(chord))
        .ok_or("chord square overflow")?;
    let (mut lo, mut hi) = (0_i128, i128::from(chord));
    // Invariant: 2 lo^2 < c2 (or lo = 0), 2 hi^2 >= c2.
    while hi - lo > 1 {
        let mid = (lo + hi) / 2;
        if 2 * mid * mid < c2 {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    i64::try_from(lo).map_err(|_| String::from("setback overflow"))
}

/// The exact integer form of the setback bound: `2k^2 < chord^2 < 2(k+1)^2`.
pub fn setback_within_bound(k: i64, chord: i64) -> bool {
    let (k, c) = (i128::from(k), i128::from(chord));
    k >= 0 && 2 * k * k < c * c && c * c < 2 * (k + 1) * (k + 1)
}

fn validate_ccw(label: &str, output: &Polygon) -> Result<i128, String> {
    validate_ring(output).map_err(|e| format!("{label}: output ring invalid: {e}"))?;
    let area = signed_area2(output).map_err(|e| format!("{label}: {e}"))?;
    if area <= 0 {
        return Err(format!(
            "{label}: output is not counter-clockwise (double area {area})"
        ));
    }
    Ok(area)
}

// ─────────────────────────────────────────────────────────────────────────────
// Convex family.
// ─────────────────────────────────────────────────────────────────────────────

/// The convex precondition: CCW, every edge at least `chord + 1` and above the
/// prepass threshold.
pub fn require_convex(input: &Polygon, chord: i64) -> Result<(), String> {
    validate_ccw("convex input", input)?;
    let n = input.points.len();
    for i in 0..n {
        let (p, q) = (input.points[i], input.points[(i + 1) % n]);
        let len = (q.x - p.x).abs() + (q.y - p.y).abs();
        if p.x != q.x && p.y != q.y {
            return Err(format!(
                "precondition: edge {p:?}->{q:?} is not axis-aligned"
            ));
        }
        if len <= chord || len < PREPASS_EPS_UNITS {
            return Err(format!(
                "precondition: edge {p:?}->{q:?} of length {len} is not longer than chord {chord} \
                 and the {PREPASS_EPS_UNITS}-unit prepass threshold"
            ));
        }
    }
    Ok(())
}

/// The convex contract: geometrically unchanged, compared as a cyclic ring.
pub fn check_convex_unchanged(input: &Polygon, output: &Polygon) -> Result<(), String> {
    let area = validate_ccw("convex", output)?;
    if output.points.len() != input.points.len() {
        return Err(format!(
            "convex: vertex count {} != input {}: {output:?}",
            output.points.len(),
            input.points.len()
        ));
    }
    if !is_cyclic_rotation(&input.points, &output.points) {
        return Err(format!(
            "convex: output is not a cyclic rotation of the input: {input:?} -> {output:?}"
        ));
    }
    let want = signed_area2(input).map_err(|e| e.to_string())?;
    if area != want {
        return Err(format!("convex: double area {area} != input {want}"));
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// L family.
// ─────────────────────────────────────────────────────────────────────────────

/// A placed L ring with its single reflex corner.
#[derive(Debug, Clone, PartialEq)]
pub struct LCase {
    /// The CCW input ring.
    pub ring: Polygon,
    /// Index of the reflex corner in `ring`.
    pub reflex: usize,
    /// Chord, canonical units.
    pub chord: i64,
}

/// Build an L in its local frame (leg thicknesses `a`, `b`, arms `la`, `lb`),
/// rotate it by `turns` quarter turns, translate it and rotate its start.
///
/// Local CCW ring: `(0,0) (a+la,0) (a+la,b) R=(a,b) (a,b+lb) (0,b+lb)`.
pub fn l_case(
    dims: (i64, i64, i64, i64),
    chord: i64,
    turns: u8,
    offset: (i64, i64),
    start: usize,
) -> LCase {
    let (a, b, la, lb) = dims;
    let local = [
        pt(0, 0),
        pt(a + la, 0),
        pt(a + la, b),
        pt(a, b),
        pt(a, b + lb),
        pt(0, b + lb),
    ];
    let start = start % local.len();
    LCase {
        ring: place(&local, turns, offset.0, offset.1, start),
        reflex: (3 + local.len() - start) % local.len(),
        chord,
    }
}

fn arm(corner: Point2, end: Point2) -> Result<((i64, i64), i64), String> {
    let (dx, dy) = (end.x - corner.x, end.y - corner.y);
    match (dx == 0, dy == 0) {
        (true, false) => Ok(((0, dy.signum()), dy.abs())),
        (false, true) => Ok(((dx.signum(), 0), dx.abs())),
        _ => Err(format!("arm {corner:?}->{end:?} is not axis-aligned")),
    }
}

/// The L precondition: CCW, a 90° reflex corner, arms and legs strictly longer
/// than the chord, every edge above the prepass threshold.
pub fn require_l(case: &LCase) -> Result<(), String> {
    validate_ccw("L input", &case.ring)?;
    let pts = &case.ring.points;
    let n = pts.len();
    if n != 6 {
        return Err(format!("precondition: L has {n} vertices, not 6"));
    }
    let (prev, corner, next) = (
        pts[(case.reflex + n - 1) % n],
        pts[case.reflex],
        pts[(case.reflex + 1) % n],
    );
    let ((ux, uy), la) = arm(corner, prev)?;
    let ((vx, vy), lb) = arm(corner, next)?;
    // Reflex for a CCW ring: cross(prev - R, next - R) > 0, at 90°.
    if ux * vy - uy * vx <= 0 || ux * vx + uy * vy != 0 {
        return Err(format!(
            "precondition: {corner:?} is not a 90° reflex corner"
        ));
    }
    if case.chord < MIN_CHORD_UNITS as i64 || la <= case.chord || lb <= case.chord {
        return Err(format!(
            "precondition: arms {la}, {lb} must exceed chord {} >= {MIN_CHORD_UNITS}",
            case.chord
        ));
    }
    for i in 0..n {
        let (p, q) = (pts[i], pts[(i + 1) % n]);
        let len = (q.x - p.x).abs() + (q.y - p.y).abs();
        if len <= case.chord || len < PREPASS_EPS_UNITS {
            return Err(format!(
                "precondition: edge {p:?}->{q:?} ({len}) must exceed the chord and the prepass"
            ));
        }
    }
    Ok(())
}

/// The independently expected chamfered ring.
pub fn expected_l(case: &LCase) -> Result<Polygon, String> {
    let pts = &case.ring.points;
    let n = pts.len();
    let (prev, corner, next) = (
        pts[(case.reflex + n - 1) % n],
        pts[case.reflex],
        pts[(case.reflex + 1) % n],
    );
    let ((ux, uy), _) = arm(corner, prev)?;
    let ((vx, vy), _) = arm(corner, next)?;
    let k = floor_setback(case.chord)?;
    let a = pt(corner.x + k * ux, corner.y + k * uy);
    let b = pt(corner.x + k * vx, corner.y + k * vy);
    let mut out = Vec::with_capacity(n + 1);
    for (i, &p) in pts.iter().enumerate() {
        if i == case.reflex {
            out.push(a);
            out.push(b);
        } else {
            out.push(p);
        }
    }
    Ok(ring(out))
}

/// Observed chamfer of one checked L result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LStats {
    /// Setback on the arm toward the previous vertex.
    pub setback_prev: i64,
    /// Setback on the arm toward the next vertex.
    pub setback_next: i64,
}

fn strictly_on_arm(corner: Point2, end: Point2, p: Point2) -> Option<i64> {
    let between = |c: i64, e: i64, v: i64| c.min(e) < v && v < c.max(e);
    if corner.x == end.x && p.x == corner.x && between(corner.y, end.y, p.y) {
        Some((p.y - corner.y).abs())
    } else if corner.y == end.y && p.y == corner.y && between(corner.x, end.x, p.x) {
        Some((p.x - corner.x).abs())
    } else {
        None
    }
}

/// The L contract checker (see the module doc).
pub fn check_l_chamfer(case: &LCase, output: &Polygon) -> Result<LStats, String> {
    let area = validate_ccw("L", output)?;
    let pts = &case.ring.points;
    let n = pts.len();
    if output.points.len() != n + 1 {
        return Err(format!(
            "L: vertex count {} != input {n} + 1 (missing or extra chamfer vertex): {output:?}",
            output.points.len()
        ));
    }
    let (prev, corner, next) = (
        pts[(case.reflex + n - 1) % n],
        pts[case.reflex],
        pts[(case.reflex + 1) % n],
    );
    if output.points.contains(&corner) {
        return Err(format!(
            "L: reflex vertex {corner:?} is still present: {output:?}"
        ));
    }
    for (i, p) in pts.iter().enumerate() {
        if i != case.reflex && !output.points.contains(p) {
            return Err(format!("L: source corner {p:?} is missing: {output:?}"));
        }
    }
    let mut setbacks = [0_i64; 2];
    for (slot, (label, end)) in [("previous", prev), ("next", next)].into_iter().enumerate() {
        let found: Vec<i64> = output
            .points
            .iter()
            .filter_map(|&p| strictly_on_arm(corner, end, p))
            .collect();
        let [k] = found.as_slice() else {
            return Err(format!(
                "L: expected exactly one chamfer point on the {label} arm {corner:?}->{end:?}, \
                 found setbacks {found:?}"
            ));
        };
        if !setback_within_bound(*k, case.chord) {
            return Err(format!(
                "L: chamfer setback {k} on the {label} arm violates the floor(chord/sqrt(2)) bound \
                 2k^2 < chord^2 < 2(k+1)^2 for chord {}",
                case.chord
            ));
        }
        setbacks[slot] = *k;
    }
    let expected = expected_l(case)?;
    if !is_cyclic_rotation(&expected.points, &output.points) {
        return Err(format!(
            "L: output ring is not the expected chamfered ring {expected:?}: {output:?}"
        ));
    }
    let k = i128::from(setbacks[0]);
    let want = signed_area2(&case.ring)
        .map_err(|e| e.to_string())?
        .checked_add(k * k)
        .ok_or("area overflow")?;
    if area != want {
        return Err(format!("L: double area {area} != input + k^2 = {want}"));
    }
    Ok(LStats {
        setback_prev: setbacks[0],
        setback_next: setbacks[1],
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Tiny CW deletion witness.
// ─────────────────────────────────────────────────────────────────────────────

/// A 2000-unit CW square (a tiny hole shape).
pub fn tiny_cw_square() -> Polygon {
    ring(vec![pt(0, 0), pt(0, 2_000), pt(2_000, 2_000), pt(2_000, 0)])
}

/// The documented outcome for [`tiny_cw_square`] at chord [`WITNESS_CHORD`]:
/// the first concave corner sends it through `clip_narrow_corner`, whose
/// backward step leaves two points, so the ring is deleted.
pub fn check_tiny_cw_deleted(output: &Polygon) -> Result<(), String> {
    if output.points.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "tiny CW ring: expected deletion (empty ring), got {output:?}"
        ))
    }
}

/// `[0, 40000]²` contour with the tiny CW square, translated to its centre,
/// as a hole.
pub fn holed_with_tiny_hole() -> ExPolygon {
    let hole = tiny_cw_square()
        .points
        .into_iter()
        .map(|p| pt(p.x + 19_000, p.y + 19_000))
        .collect();
    ExPolygon {
        contour: ring(vec![
            pt(0, 0),
            pt(40_000, 0),
            pt(40_000, 40_000),
            pt(0, 40_000),
        ]),
        holes: vec![ring(hole)],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy.
// ─────────────────────────────────────────────────────────────────────────────

/// One generated family member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    /// Rectangle with sides `w`, `h` and a ring start.
    Convex {
        /// Width.
        w: i64,
        /// Height.
        h: i64,
        /// Ring start rotation.
        start: usize,
    },
    /// L with leg thicknesses `a`, `b` and arms `la`, `lb`.
    L {
        /// `(a, b, la, lb)`.
        dims: (i64, i64, i64, i64),
        /// Quarter turns.
        turns: u8,
        /// Ring start rotation.
        start: usize,
    },
}

/// One generated `H-OFFSET-SMOOTH` input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmoothInput {
    /// The family member.
    pub family: Family,
    /// Chord, canonical units.
    pub chord: i64,
    /// Translation.
    pub offset: (i64, i64),
}

/// Build the `H-OFFSET-SMOOTH` strategy from validated limits.
///
/// Every dimension is `chord + EXTRA_MIN_UNITS + e` with a generated `e >= 0`
/// whose upper bound depends only on the validated limits, so every generated
/// **and shrunk** value keeps every feature longer than the chord and the
/// prepass threshold without rejection. Translations stay within
/// `±(max_abs_units - max_side_units)`, so quarter-turned rings (local extent
/// `±max_side_units`) stay within `±max_abs_units`.
pub fn strategy(limits: &SmoothLimits) -> BoxedStrategy<SmoothInput> {
    let chord = MIN_CHORD_UNITS as i64..=limits.max_clip_units;
    let reach = limits.max_abs_units - limits.max_side_units;
    let shift = (-reach..=reach, -reach..=reach);
    let rect_extra = 0..=(limits.max_side_units - limits.max_clip_units - EXTRA_MIN_UNITS);
    let l_extra = 0..=(limits.max_side_units / 2 - limits.max_clip_units - EXTRA_MIN_UNITS);
    let convex = (
        chord.clone(),
        rect_extra.clone(),
        rect_extra,
        0..4_usize,
        shift.clone(),
    )
        .prop_map(|(chord, ew, eh, start, offset)| SmoothInput {
            family: Family::Convex {
                w: chord + EXTRA_MIN_UNITS + ew,
                h: chord + EXTRA_MIN_UNITS + eh,
                start,
            },
            chord,
            offset,
        })
        .boxed();
    let l = (
        chord,
        (l_extra.clone(), l_extra.clone(), l_extra.clone(), l_extra),
        0..4_u8,
        0..6_usize,
        shift,
    )
        .prop_map(|(chord, (ea, eb, ela, elb), turns, start, offset)| {
            let dim = |e: i64| chord + EXTRA_MIN_UNITS + e;
            SmoothInput {
                family: Family::L {
                    dims: (dim(ea), dim(eb), dim(ela), dim(elb)),
                    turns,
                    start,
                },
                chord,
                offset,
            }
        })
        .boxed();
    Union::new(vec![convex, l]).boxed()
}

/// Convex ring for a generated rectangle.
pub fn convex_ring(w: i64, h: i64, offset: (i64, i64), start: usize) -> Polygon {
    place(
        &[pt(0, 0), pt(w, 0), pt(w, h), pt(0, h)],
        0,
        offset.0,
        offset.1,
        start,
    )
}

fn within(limits: &SmoothLimits, ring: &Polygon) -> Result<(), String> {
    if ring
        .points
        .iter()
        .any(|p| p.x.abs() > limits.max_abs_units || p.y.abs() > limits.max_abs_units)
    {
        return Err(format!(
            "generator invariant: {ring:?} exceeds max_abs_units {}",
            limits.max_abs_units
        ));
    }
    Ok(())
}

/// Check one generated input against its family contract.
pub fn check_input(input: &SmoothInput, limits: &SmoothLimits) -> Result<(), String> {
    if input.chord > limits.max_clip_units {
        return Err(format!(
            "generator invariant: chord above max_clip_units in {input:?}"
        ));
    }
    match input.family {
        Family::Convex { w, h, start } => {
            let ring = convex_ring(w, h, input.offset, start);
            within(limits, &ring)?;
            require_convex(&ring, input.chord).map_err(|e| format!("generator invariant: {e}"))?;
            check_convex_unchanged(&ring, &smooth_outward_polygon(&ring, input.chord))
        }
        Family::L { dims, turns, start } => {
            let case = l_case(dims, input.chord, turns, input.offset, start);
            within(limits, &case.ring)?;
            require_l(&case).map_err(|e| format!("generator invariant: {e}"))?;
            let output = smooth_outward_polygon(&case.ring, case.chord);
            within(limits, &output)?;
            check_l_chamfer(&case, &output).map(|_| ())
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses.
// ─────────────────────────────────────────────────────────────────────────────

/// The deterministic L witness: legs and arms 20000 units, chord 5000.
/// `floor(5000 / sqrt(2)) = 3535` (`2 * 3535^2 = 24_991_450 < 25_000_000 <
/// 2 * 3536^2 = 25_005_632`).
pub fn l_witness(turns: u8, start: usize) -> LCase {
    l_case(
        (20_000, 20_000, 20_000, 20_000),
        WITNESS_CHORD,
        turns,
        (0, 0),
        start,
    )
}

/// The deterministic convex witness: `[0, 30000] x [0, 20000]`.
pub fn convex_witness() -> Polygon {
    convex_ring(30_000, 20_000, (0, 0), 0)
}

/// `(largest local side, largest absolute coordinate)` over the witnesses.
pub fn witness_extents() -> Result<(u64, u64), String> {
    let mut rings: Vec<Polygon> = (0..4).map(|t| l_witness(t, 0).ring).collect();
    rings.push(convex_witness());
    rings.push(tiny_cw_square());
    rings.push(holed_with_tiny_hole().contour);
    let mut side = 0_i64;
    let mut abs = 0_i64;
    let span = |values: &[i64]| -> i64 {
        values.iter().max().copied().unwrap_or(0) - values.iter().min().copied().unwrap_or(0)
    };
    for ring in &rings {
        let xs: Vec<i64> = ring.points.iter().map(|p| p.x).collect();
        let ys: Vec<i64> = ring.points.iter().map(|p| p.y).collect();
        side = side.max(span(&xs)).max(span(&ys));
        abs = abs.max(
            xs.iter()
                .chain(ys.iter())
                .map(|v| v.abs())
                .max()
                .unwrap_or(0),
        );
    }
    Ok((
        u64::try_from(side).map_err(|_| "negative side")?,
        u64::try_from(abs).map_err(|_| "negative extent")?,
    ))
}

/// Deterministic, RNG-independent witnesses run before the generated cases.
pub fn deterministic_witnesses() {
    assert_eq!(floor_setback(WITNESS_CHORD), Ok(3_535));
    assert!(setback_within_bound(3_535, WITNESS_CHORD));
    let convex = convex_witness();
    require_convex(&convex, WITNESS_CHORD).expect("convex witness precondition");
    check_convex_unchanged(&convex, &smooth_outward_polygon(&convex, WITNESS_CHORD))
        .unwrap_or_else(|e| panic!("convex witness: {e}"));
    for turns in 0..4 {
        for start in 0..6 {
            let case = l_witness(turns, start);
            require_l(&case).expect("L witness precondition");
            let stats = check_l_chamfer(&case, &smooth_outward_polygon(&case.ring, case.chord))
                .unwrap_or_else(|e| panic!("L witness turns={turns} start={start}: {e}"));
            assert_eq!(
                stats,
                LStats {
                    setback_prev: 3_535,
                    setback_next: 3_535,
                },
                "L witness turns={turns} start={start}"
            );
        }
    }
    check_tiny_cw_deleted(&smooth_outward_polygon(&tiny_cw_square(), WITNESS_CHORD))
        .unwrap_or_else(|e| panic!("tiny CW witness: {e}"));
    // Secondary seam: the emptied hole is dropped and the convex contour stays.
    let holed = holed_with_tiny_hole();
    let out = smooth_outward_ex(&holed, WITNESS_CHORD);
    assert!(
        out.holes.is_empty(),
        "the tiny hole must be filled: {out:?}"
    );
    check_convex_unchanged(&holed.contour, &out.contour)
        .unwrap_or_else(|e| panic!("holed witness contour: {e}"));
}

/// Registry entry point for `proptest_smooth_outward_safe_families`.
pub fn run_property() {
    deterministic_witnesses();
    let run =
        PropertyRun::new(PROPERTY_ID, TEST_NAME).expect("use the registry-backed property command");
    let raw = RawSmoothBounds::read(&run).expect("H-OFFSET-SMOOTH bounds");
    let limits = validate_limits(raw).expect("H-OFFSET-SMOOTH profile");
    let strategy = strategy(&limits);
    run.check(&strategy, |input, _guard| -> TestCaseResult {
        check_input(&input, &limits).map_err(TestCaseError::fail)
    })
    .expect("H-OFFSET-SMOOTH charter");
}
