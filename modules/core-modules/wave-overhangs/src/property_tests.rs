//! Opt-in property tests for this module's real wave generator.
//!
//! Charter (`docs/spec_packets/proptest-openrscad_13_pbt-infill`,
//! `packet.spec.md` § Property charters → `M-WAVE-TRIM`):
//!
//! * Call the actual `generate` on a controlled axis-aligned bridge rectangle
//!   with analytically specified supporting bands, zero perimeter overlap, no
//!   narrow split, finite positive spacing/width/flow, and enough trim
//!   clearance for the first deposited-half-width inset to survive.
//! * For identical inputs call `generate` twice and compare every path point,
//!   every `filled` contour/hole and every fallback in order. This is the
//!   `docs/22_test_quality.md` §2.1 determinism-pin exception, not an
//!   independent correctness oracle; the waiver is on the property itself.
//! * `P` is the original rectangular wave piece **before** the first trim, `r`
//!   the deposited radius, `T = P ⊖ disk(r)` the centerline trim. Every output
//!   segment's centerline endpoints must lie in `T`, and its whole deposited
//!   capsule (both endcaps included) must lie in `P`. The capsule is **never**
//!   required to fit inside `T` — that would apply the inset twice.
//! * Coverage is a bounded **sampled** statement over a declared finite
//!   interior lattice, not a proof of arbitrary continuous area. `filled` is
//!   regularized by the generator and is not a swept footprint, so it is never
//!   used as a coverage oracle.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates. The
//! `generator` module is crate-private, so the properties reach the same
//! `generate` entry point `run_infill` calls without widening production
//! visibility. The `proptest_` body requires the foundation runner's
//! `PNP_PROPERTY_CONTEXT`; the controls and the shrink probe are context-free
//! and run under a plain `cargo test --lib`.
//!
//! # Fixture, and the executed reason it deviates from the literal charter
//!
//! The charter's literal direct-kernel fixture — `infill_area = [rect(0,0,6,6)]`
//! with `lower = [rect(-6,-6,0,12), rect(6,-6,12,12)]` — was executed first and
//! does **not** produce the required witness: the support bands touch the fill
//! rectangle only along its edges, and `generate` intersects `lower` with the
//! fill-area bounding box inflated by one epsilon unit before deriving anchors.
//! The resulting anchor region has zero area, so the run returns `paths = []`
//! with `fallbacks = [MissingAnchors, EmptyOutput]`
//! (`target/proptest-infill-evidence/step6/probe-fixture.log`, variant
//! "A literal"). That is not a production defect: `run_infill` never calls
//! `generate` with a bare bridge — it builds the wave domain as
//! `external_bridge ∪ anchor_band`, so an anchor band is always part of the
//! fill area.
//!
//! The fixture below keeps the charter's rectangle, parameters and `P`/`T` and
//! changes only the support bands, each extended `ANCHOR_INTRUSION_MM = 0.5` mm
//! into the bridge. The unsupported span stays 5 mm wide, the bands are still
//! analytically specified axis-aligned rectangles, and the anchor region is now
//! `[0,0.5]×[0,6] ∪ [5.5,6]×[0,6]` — positive area, so seeds exist. `0.5 mm` is
//! safe rather than knife-edge: the generator erodes anchors by
//! `min(3.0, base_spacing·(wall_count+1)) + 0.1·flow_width = 1.80 + 0.04 =
//! 1.84 mm`, i.e. 1.34 mm deeper than the rim, so `inset_anchors` is empty with
//! wide margin and the wave piece is the raw bridge rectangle.
//!
//! # Derivation of `P`, `r` and `T`
//!
//! With `min_length = 0` the minimum-length filter is a no-op, so
//! `filtered = infill_area = [0,6]²`. The bands give
//! `anchors = [0,0.5]×[0,6] ∪ [5.5,6]×[0,6]` and
//! `overhangs = [0.5,5.5]×[0,6]`; the erosion above removes every anchor, so
//! `inset_overhang_area = filtered`. `perimeter_overlap = 0` leaves the cover
//! equal to the piece, and `minimum_width = 0` disables the narrow-neck split
//! outright, so the piece is exactly `P = [0,6]×[0,6]` mm.
//!
//! `r = (flow_width · flow_ratio)/2` is evaluated in the same `f32` arithmetic
//! the generator uses. For the seam fixture `r = (0.4 · 1.875)/2 = 0.375` mm
//! exactly (3 750 scaled units), so `T = [0.375, 5.625]²`. The property's
//! sampled domain keeps `P` fixed (it never varies `perimeter_overlap`,
//! `minimum_width`, `min_length` or `base_spacing`) and varies line spacing,
//! flow width, flow ratio, pattern and the iteration bound.
//!
//! # Allowance derivation (no observed-output epsilon)
//!
//! * `CLIP_BAND_UNITS = 2`: `PreparedPolylineClip::new`
//!   (`crates/slicer-core/src/polygon_ops.rs`) pre-inflates the clip universe by
//!   one unit and documents that every reported boundary coordinate lies within
//!   ±2 units of the exact clip boundary — one unit of inflation plus at most
//!   one unit of integer-coordinate rounding per side.
//! * `TRIM_QUANTIZATION_UNITS = 2`: the trim boundary is produced by an
//!   integer-coordinate Clipper offset of an integer-coordinate piece, so each
//!   resulting edge is the nearest integer to the analytic `P ⊖ disk(r)` edge:
//!   at most one unit of snap per side, two units in total.
//! * `RATIONAL_EPSILON_MM = 1e-9`: every compared coordinate is an exact
//!   integer number of units converted to `f64` mm; the representation error at
//!   |x| ≤ 64 mm is below 1.4e-14 mm, so 1e-9 mm is ~5 orders below one unit
//!   and ~70 000× the actual float error.
//!
//! Total boundary allowance: `4 units + 1e-9 mm = 0.0004 mm`. The capsule
//! allowance is that same 4-unit outward band: with a convex rectangular `P`
//! the radius-`r` capsule is inside `P` iff both centerline endpoints are at
//! least `r` from every edge (the inward distance is linear along a straight
//! segment, and each endcap is covered by the same half-plane test), so the
//! capsule may protrude at most 4 units past `P`. Both bands are 625× smaller
//! than the 0.25 mm gap the nominal-radius negative control relies on, so
//! neither can mask it. Radius `r` is evaluated with the same `f32` operations
//! as the generator, so no separate radius allowance is needed.
//!
//! # Coverage lattice
//!
//! `LATTICE_SIDE = 96` cells per axis over `P` (9 216 cell centres, pitch
//! 0.0625 mm), guarded by the `max_cells` profile bound. A cell is *empty* when
//! its centre is farther than `r + MAX_INTERIOR_VOID_HALF_WIDTH_MM` from every
//! emitted centerline, where `0.1 mm` is half of the existing seam fixture's
//! `MAX_INTERIOR_VOID_WIDTH_MM = 0.5 · nozzle = 0.2 mm`: a point at distance
//! `r + w/2` witnesses a void of width `w`, which mirrors that fixture's
//! erosion test. Empty cells 4-connected to the lattice border are
//! boundary-connected slivers and are excluded, exactly as the continuous seam
//! test excludes voids that touch the domain boundary; any remaining empty cell
//! is an interior gap. The pitch is chosen so the recorded pre-fix seam void
//! (0.29 mm × 4.76 mm in `waves_cover_domain_without_seam_gap`) has a central
//! locus `0.29 − 0.2 = 0.09 mm` wider than one pitch, i.e. guaranteed to
//! contain a sample column; the checker therefore guarantees detection of
//! axis-aligned voids at least `0.2 mm + pitch = 0.2625 mm` wide and one pitch
//! long, and *may* miss narrower ones. This is sampled coverage only.
//!
//! **Declared blind spot.** Inside the property's spacing domain
//! (`line_spacing <= r + MAX_INTERIOR_VOID_HALF_WIDTH_MM`) two adjacent fronts
//! overlap by more than the void threshold, so deleting a *single* front cannot
//! create a detectable interior gap by construction, and deleting a wider band
//! opens a void that reaches the piece boundary and is excluded as a sliver. The
//! removal controls therefore step outside the domain on purpose: the
//! seam-closing pair is deleted at the 0.35 mm seam spacing (a pair, whose
//! removal re-creates the pre-fix 0.29 mm seam void) and a middle segment is
//! deleted at 0.7 mm spacing (where `2 · 0.7 > 2r = 0.75` is no longer true).
//! The property's coverage counters measure that a positive interior population
//! exists; they do not claim single-front sensitivity.
//!
//! # Literal review (AC-6)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow this out-of-line `mod property_tests;`. Re-derived watchlist types
//! and every literal site in this file:
//!
//! * `WaveParams` (`crate::generator`, `pub(crate)`, 11 named fields) — **not**
//!   watched (the watchlist derives from `crates/*/src` pub structs only). Its
//!   sites are the single exhaustive literal in [`params_from`] (inline
//!   `// exhaustive:` waiver; every other case is built through that
//!   constructor) and one partial `..fixture_params()` override for the
//!   `min_length` fallback fixture in `wave_controls`.
//! * `WaveOutput` (`pub(crate)`, 3 fields) — uses `..WaveOutput::default()` at
//!   its synthetic-control site.
//! * `ExPolygon` (2 fields), `Polygon` (1), `Point2` (2) — below the five-field
//!   watchlist threshold and not watched. [`rect_mm`] is their only site.
//! * `RectMm`, `Lattice`, `WaveCase`, `WaveLimits`, `WaveMode`,
//!   `ContainmentReport`, `CoverageReport`, `WaveReport` — private
//!   module-local types, not watched.
//!
//! `docs/22_test_quality.md` is reviewed by hand for the same reason: the
//! `generate` double call is the marked determinism pin; the mutations in
//! `wave_controls` are explicit negative controls, not SUT regressions; the
//! coverage population is asserted positive so no coverage loop is vacuous; and
//! the fixture parameters are the only inputs, so no expectation is derived
//! from `generate`'s own output.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{units_to_mm, ExPolygon, Point2, Polygon, UNITS_PER_MM};
use slicer_sdk::test_support::property_testing::domains::DomainError;
use slicer_sdk::test_support::property_testing::PropertyRun;

use crate::generator::{generate, FallbackReason, WaveOutput, WaveParams, EPSILON_MM};
use crate::WavePattern;

// ---------------------------------------------------------------------------
// Registry identity and bounds
// ---------------------------------------------------------------------------

const WAVE_ID: &str = "M-WAVE-TRIM";
const WAVE_TEST: &str = "property_tests::proptest_wave_trim_and_coverage";
const WAVE_BOUND_KEYS: [&str; 5] = [
    "max_abs_units",
    "max_spacing_units",
    "max_width_units",
    "max_iterations",
    "max_cells",
];

// ---------------------------------------------------------------------------
// Seam fixture (adapted from `tests/wave_overhangs_tdd.rs`)
// ---------------------------------------------------------------------------

/// Edge length of the controlled bridge rectangle, in millimetres.
const SEAM_BRIDGE_MM: f32 = 6.0;
/// How far each support band reaches into the bridge, in millimetres.
///
/// `0.5` is well inside the generator's `1.84 mm` anchor erosion, so
/// `inset_anchors` stays empty and the wave piece is the raw rectangle. See the
/// module docs for the executed `MissingAnchors` result that rules out the
/// charter's zero-intrusion bands.
const ANCHOR_INTRUSION_MM: f32 = 0.5;
/// Manifest default `wave_overhang_line_spacing`, in millimetres.
const SEAM_LINE_SPACING_MM: f32 = 0.35;
/// `bridge_line_width` / nozzle width handed to the generator, in millimetres.
const SEAM_FLOW_WIDTH_MM: f32 = 0.4;
/// `wave_overhang_flow_mm3_per_mm / (width · layer_height)` from the seam
/// fixture, dimensionless.
const SEAM_FLOW_RATIO: f32 = 1.875;
/// Flow-derived spacing (`bridge_flow.scaled_spacing()`), in millimetres.
const SEAM_BASE_SPACING_MM: f32 = 0.45;
/// Enclosing region's perimeter count.
const SEAM_WALL_COUNT: u32 = 3;
/// Manifest default `wave_overhang_min_new_area`, in square millimetres.
const SEAM_MIN_NEW_AREA_MM2: f32 = 0.01;

/// The wave piece `P` before the first trim, in millimetres.
const PIECE: RectMm = RectMm {
    min_x: 0.0,
    min_y: 0.0,
    max_x: 6.0,
    max_y: 6.0,
};

// ---------------------------------------------------------------------------
// Derived numeric bounds
// ---------------------------------------------------------------------------

/// One scaled coordinate unit in millimetres (1 unit = 100 nm).
const UNIT_MM: f64 = 1.0 / UNITS_PER_MM;

/// Documented `PreparedPolylineClip` boundary band, in units.
const CLIP_BAND_UNITS: f64 = 2.0;
/// Integer snapping of the integer-unit trim offset, in units.
const TRIM_QUANTIZATION_UNITS: f64 = 2.0;
/// Total centerline/capsule boundary allowance, in units.
const BOUNDARY_ALLOWANCE_UNITS: f64 = CLIP_BAND_UNITS + TRIM_QUANTIZATION_UNITS;
/// `f64` rational comparison epsilon, in millimetres.
const RATIONAL_EPSILON_MM: f64 = 1.0e-9;

/// Half the narrowest interior void the existing seam fixture treats as a
/// defect, in millimetres (`0.5 · nozzle_diameter = 0.2` there).
const MAX_INTERIOR_VOID_HALF_WIDTH_MM: f64 = 0.1;

/// Cell centres per lattice axis: pitch `6/96 = 0.0625` mm.
const LATTICE_SIDE: usize = 96;
/// Lattice cells implied by [`LATTICE_SIDE`]; the `max_cells` floor.
const LATTICE_CELLS: u32 = (LATTICE_SIDE * LATTICE_SIDE) as u32;

/// Lowest sampled wave line spacing, in units (0.2 mm).
const MIN_LINE_SPACING_UNITS: i64 = 2_000;
/// Highest sampled wave line spacing, in units (0.6 mm). The effective upper
/// bound is `min(r + 0.1 mm, max_spacing_units, this)`.
const MAX_LINE_SPACING_UNITS: i64 = 6_000;
/// Seed-strip expansion the generator uses (`max(base_spacing/10, EPSILON_MM)`),
/// in units, with margin: `0.045 mm = 450` units, reduced by 50 units because
/// `buffer_open`'s 16-segment disc reaches only `450·cos(π/16) ≈ 441` units.
const SEED_EXPANSION_UNITS: i64 = 400;
/// Lowest sampled flow width, in units (0.36 mm).
const MIN_FLOW_WIDTH_UNITS: i64 = 3_600;
/// Highest sampled flow width, in units (0.50 mm).
const MAX_FLOW_WIDTH_UNITS: i64 = 5_000;
/// Lowest sampled flow ratio, thousandths (1.25).
const MIN_FLOW_RATIO_MILLIS: i64 = 1_250;
/// Highest sampled flow ratio, thousandths (2.50).
const MAX_FLOW_RATIO_MILLIS: i64 = 2_500;
/// Smallest `max_width_units` that still admits the sampled box:
/// `MIN_FLOW_WIDTH_UNITS · MAX_FLOW_RATIO_MILLIS / 1000`.
const MIN_DEPOSITED_WIDTH_UNITS: i64 = MIN_FLOW_WIDTH_UNITS * MAX_FLOW_RATIO_MILLIS / 1_000;
/// Largest iteration cap sampled for the capped mode.
///
/// Even at the widest sampled spacing the 6 mm piece still needs about five
/// propagation iterations to merge (`5.91 mm / (2 · 0.6 mm) ≈ 5`), so a cap of
/// 1..=3 always stops with area still unfilled and therefore always reports
/// `IterationResidual`.
const MAX_CAPPED_ITERATIONS: u32 = 3;
/// Shrink-probe step ceiling.
const SHRINK_STEPS: usize = 96;

/// The fixture's largest absolute coordinate, in units: the support bands reach
/// 12 mm.
const FIXTURE_MAX_ABS_UNITS: i64 = 120_000;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ---------------------------------------------------------------------------
// Independent geometry
// ---------------------------------------------------------------------------

/// Axis-aligned rectangle in millimetres with inclusive bounds.
#[derive(Debug, Clone, Copy, PartialEq)]
struct RectMm {
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl RectMm {
    fn width(self) -> f64 {
        self.max_x - self.min_x
    }

    fn height(self) -> f64 {
        self.max_y - self.min_y
    }

    /// `self ⊖ disk(radius)`, or an error when the erosion collapses.
    ///
    /// A collapsed erosion is exactly the generator's narrower-trim fallback
    /// branch, which this property's domain excludes: the full deposited-half-
    /// width inset must survive.
    fn inscribed_disc(self, radius: f64) -> Result<RectMm, String> {
        if self.width() <= 0.0 || self.height() <= 0.0 {
            return Err(format!("piece {self:?} is empty"));
        }
        let inset = RectMm {
            min_x: self.min_x + radius,
            min_y: self.min_y + radius,
            max_x: self.max_x - radius,
            max_y: self.max_y - radius,
        };
        if inset.width() <= 0.0 || inset.height() <= 0.0 {
            return Err(format!(
                "centerline trim {inset:?} is empty: radius {radius} does not survive the first \
                 trim (the narrower-trim fallback is outside this property's domain)"
            ));
        }
        Ok(inset)
    }

    /// Closed containment with an outward allowance.
    fn contains_with_allowance(self, x: f64, y: f64, allowance: f64) -> bool {
        x >= self.min_x - allowance
            && x <= self.max_x + allowance
            && y >= self.min_y - allowance
            && y <= self.max_y + allowance
    }

    /// Smallest signed distance from `(x, y)` to one of the four edges;
    /// negative when the point is outside.
    fn inward_margin(self, x: f64, y: f64) -> f64 {
        (x - self.min_x)
            .min(self.max_x - x)
            .min(y - self.min_y)
            .min(self.max_y - y)
    }
}

/// Boundary allowance for both the centerline and the capsule check, in mm.
fn boundary_allowance_mm() -> f64 {
    BOUNDARY_ALLOWANCE_UNITS * UNIT_MM + RATIONAL_EPSILON_MM
}

fn point_mm(point: Point2) -> (f64, f64) {
    (point.x as f64 * UNIT_MM, point.y as f64 * UNIT_MM)
}

fn rect_mm(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

/// Distance from a point to a segment, in millimetres, computed independently
/// of the generator (plain `f64` projection, no clipping).
fn point_segment_distance_mm(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq <= 0.0 {
        0.0
    } else {
        (((px - ax) * dx + (py - ay) * dy) / len_sq).clamp(0.0, 1.0)
    };
    let fx = ax + t * dx;
    let fy = ay + t * dy;
    ((px - fx) * (px - fx) + (py - fy) * (py - fy)).sqrt()
}

// ---------------------------------------------------------------------------
// The checker
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default)]
struct ContainmentReport {
    points: usize,
    segments: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct CoverageReport {
    cells: usize,
    interior_cells: usize,
    covered_interior_cells: usize,
    interior_empty_cells: usize,
    boundary_connected_empty_cells: usize,
}

#[derive(Debug, Clone, Copy, Default)]
struct WaveReport {
    paths: usize,
    containment: ContainmentReport,
    coverage: CoverageReport,
}

/// Finite square lattice over the declared wave piece.
#[derive(Debug, Clone, Copy)]
struct Lattice {
    piece: RectMm,
    side: usize,
    pitch_x: f64,
    pitch_y: f64,
}

impl Lattice {
    fn new(piece: RectMm, side: usize) -> Self {
        assert!(side >= 2, "lattice needs at least one interior cell");
        Self {
            piece,
            side,
            pitch_x: piece.width() / side as f64,
            pitch_y: piece.height() / side as f64,
        }
    }

    fn cells(&self) -> usize {
        self.side * self.side
    }

    fn center(&self, column: usize, row: usize) -> (f64, f64) {
        (
            self.piece.min_x + (column as f64 + 0.5) * self.pitch_x,
            self.piece.min_y + (row as f64 + 0.5) * self.pitch_y,
        )
    }
}

/// Check every emitted centerline against `T = P ⊖ disk(r)` and every whole
/// deposited capsule (both endcaps included) against `P`.
///
/// For a convex rectangular `P`, endpoint containment in `T` already proves the
/// entire straight centerline lies in `T`; the capsule half-plane test below is
/// the independent second statement the charter requires, and it is *not*
/// `capsule ⊆ T`.
fn check_containment(
    output: &WaveOutput,
    piece: RectMm,
    radius: f64,
) -> Result<ContainmentReport, String> {
    if !(radius >= f64::from(EPSILON_MM)) {
        return Err(format!(
            "deposited radius {radius} mm is below EPSILON_MM; this property is restricted to the \
             positive-flow domain where max(EPSILON_MM, r) is r"
        ));
    }
    let trim = piece.inscribed_disc(radius)?;
    let allowance = boundary_allowance_mm();
    let mut report = ContainmentReport::default();
    for (path_index, path) in output.paths.iter().enumerate() {
        if path.len() < 2 {
            return Err(format!(
                "path {path_index} has {} point(s); emitted waves must be polylines",
                path.len()
            ));
        }
        for point in path {
            let (x, y) = point_mm(*point);
            if !trim.contains_with_allowance(x, y, allowance) {
                return Err(format!(
                    "path {path_index} centerline point ({x:.6}, {y:.6}) mm leaves the \
                     centerline trim {trim:?} (allowance {allowance:.7} mm)"
                ));
            }
            report.points += 1;
        }
        for window in path.windows(2) {
            let (ax, ay) = point_mm(window[0]);
            let (bx, by) = point_mm(window[1]);
            let margin = piece.inward_margin(ax, ay).min(piece.inward_margin(bx, by));
            if margin + allowance < radius {
                return Err(format!(
                    "path {path_index} deposited capsule protrudes {:.6} mm past the piece \
                     boundary at ({ax:.6}, {ay:.6})->({bx:.6}, {by:.6}) mm (radius {radius}, \
                     allowance {allowance:.7} mm)",
                    radius - margin
                ));
            }
            report.segments += 1;
        }
    }
    Ok(report)
}

/// Sampled coverage over the declared finite lattice.
fn check_coverage(
    output: &WaveOutput,
    piece: RectMm,
    radius: f64,
    lattice: &Lattice,
) -> Result<CoverageReport, String> {
    if lattice.piece != piece {
        return Err(format!(
            "lattice {:?} does not cover the checked piece {piece:?}",
            lattice.piece
        ));
    }
    let reach = radius + MAX_INTERIOR_VOID_HALF_WIDTH_MM;
    let side = lattice.side;
    let mut empty = vec![false; side * side];
    for row in 0..side {
        for column in 0..side {
            let (x, y) = lattice.center(column, row);
            let mut nearest = f64::INFINITY;
            'paths: for path in &output.paths {
                for window in path.windows(2) {
                    let (ax, ay) = point_mm(window[0]);
                    let (bx, by) = point_mm(window[1]);
                    nearest = nearest.min(point_segment_distance_mm(x, y, ax, ay, bx, by));
                    if nearest <= reach {
                        break 'paths;
                    }
                }
            }
            empty[row * side + column] = nearest > reach;
        }
    }
    // 4-connected flood fill from every empty cell on the lattice border.
    let mut connected = vec![false; side * side];
    let mut stack: Vec<usize> = Vec::new();
    let seed = |column: usize, row: usize, connected: &mut Vec<bool>, stack: &mut Vec<usize>| {
        let index = row * side + column;
        if empty[index] && !connected[index] {
            connected[index] = true;
            stack.push(index);
        }
    };
    for column in 0..side {
        seed(column, 0, &mut connected, &mut stack);
        seed(column, side - 1, &mut connected, &mut stack);
    }
    for row in 0..side {
        seed(0, row, &mut connected, &mut stack);
        seed(side - 1, row, &mut connected, &mut stack);
    }
    while let Some(index) = stack.pop() {
        let row = index / side;
        let column = index % side;
        if column > 0 {
            seed(column - 1, row, &mut connected, &mut stack);
        }
        if column + 1 < side {
            seed(column + 1, row, &mut connected, &mut stack);
        }
        if row > 0 {
            seed(column, row - 1, &mut connected, &mut stack);
        }
        if row + 1 < side {
            seed(column, row + 1, &mut connected, &mut stack);
        }
    }

    let mut report = CoverageReport {
        cells: side * side,
        ..CoverageReport::default()
    };
    for row in 0..side {
        for column in 0..side {
            let index = row * side + column;
            if connected[index] {
                report.boundary_connected_empty_cells += 1;
                continue;
            }
            report.interior_cells += 1;
            if empty[index] {
                report.interior_empty_cells += 1;
            } else {
                report.covered_interior_cells += 1;
            }
        }
    }
    Ok(report)
}

/// The complete checker used by both the property and every control.
///
/// `require_full_fill` distinguishes the unbounded propagation class (no
/// fallbacks, nonempty paths, sampled coverage complete) from the deliberately
/// capped class, where `IterationResidual` is legal and coverage is *not*
/// asserted because the cap leaves real area unfilled.
fn check_wave_output(
    output: &WaveOutput,
    piece: RectMm,
    radius: f64,
    lattice: &Lattice,
    require_full_fill: bool,
) -> Result<WaveReport, String> {
    if require_full_fill {
        if !output.fallbacks.is_empty() {
            return Err(format!(
                "the unbounded-propagation class must not report fallbacks, got {:?}",
                output.fallbacks
            ));
        }
        if output.paths.is_empty() {
            return Err("the unbounded-propagation class emitted no wave path".to_owned());
        }
    }
    let containment = check_containment(output, piece, radius)?;
    let coverage = check_coverage(output, piece, radius, lattice)?;
    if require_full_fill && coverage.interior_empty_cells > 0 {
        return Err(format!(
            "sampled coverage left {} interior empty cell(s) of {} ({:.6} mm pitch, reach \
             {:.6} mm)",
            coverage.interior_empty_cells,
            coverage.interior_cells,
            lattice.pitch_x,
            radius + MAX_INTERIOR_VOID_HALF_WIDTH_MM
        ));
    }
    Ok(WaveReport {
        paths: output.paths.len(),
        containment,
        coverage,
    })
}

/// Bit-identical path/filled/fallback comparison — the determinism pin.
fn outputs_identical(left: &WaveOutput, right: &WaveOutput) -> bool {
    left.paths == right.paths && left.filled == right.filled && left.fallbacks == right.fallbacks
}

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The controlled wave piece's fill area: one axis-aligned 6×6 mm bridge.
fn bridge_infill() -> Vec<ExPolygon> {
    vec![rect_mm(0.0, 0.0, SEAM_BRIDGE_MM, SEAM_BRIDGE_MM)]
}

/// Support bands anchored on the bridge's left and right edges, each reaching
/// [`ANCHOR_INTRUSION_MM`] into the bridge (see the module docs for why the
/// charter's zero-intrusion bands produce `MissingAnchors`).
fn bridge_lower() -> Vec<ExPolygon> {
    vec![
        rect_mm(-6.0, -6.0, ANCHOR_INTRUSION_MM, 12.0),
        rect_mm(SEAM_BRIDGE_MM - ANCHOR_INTRUSION_MM, -6.0, 12.0, 12.0),
    ]
}

/// Build the generator parameters for one case.
///
/// This is the only `WaveParams` literal in the file; every other case is built
/// through it.
fn params_from(
    spacing_units: i64,
    width_units: i64,
    ratio_millis: i64,
    iterations: u32,
    pattern: WavePattern,
) -> WaveParams {
    WaveParams {
        line_spacing: units_to_mm(spacing_units),
        // exhaustive: crate-private 13-field `WaveParams` (the docs/21 watchlist
        // covers `crates/*/src` pub structs only); each field is a deliberate
        // fixture parameter. Perimeter overlap and minimum width stay zero (no
        // cover growth, no narrow split) so `P` stays the analytic rectangle.
        perimeter_overlap: 0.0,
        minimum_width: 0.0,
        min_new_area: SEAM_MIN_NEW_AREA_MM2,
        min_length: 0.0,
        max_iterations: iterations,
        pattern,
        flow_width: units_to_mm(width_units),
        flow_ratio: (ratio_millis as f32) / 1000.0,
        base_spacing: SEAM_BASE_SPACING_MM,
        wall_count: SEAM_WALL_COUNT,
    }
}

/// The seam fixture's integer factors: line spacing, flow width, ratio millis.
fn seam_factors() -> (i64, i64, i64) {
    (
        (SEAM_LINE_SPACING_MM * UNITS_PER_MM as f32).round() as i64,
        (SEAM_FLOW_WIDTH_MM * UNITS_PER_MM as f32).round() as i64,
        (SEAM_FLOW_RATIO * 1000.0).round() as i64,
    )
}

/// The seam fixture's own parameter set.
fn fixture_params() -> WaveParams {
    let (spacing_units, width_units, ratio_millis) = seam_factors();
    params_from(
        spacing_units,
        width_units,
        ratio_millis,
        0,
        WavePattern::Smart,
    )
}

/// Deposited radius `r = (flow_width · flow_ratio)/2` in the generator's own
/// `f32` arithmetic.
fn deposited_radius(params: &WaveParams) -> f64 {
    let deposited = params.flow_width * params.flow_ratio;
    f64::from(deposited / 2.0)
}

fn run_case(case: &WaveCase) -> WaveOutput {
    generate(&bridge_infill(), &bridge_lower(), &case.params)
}

// ---------------------------------------------------------------------------
// Bounds, cases and strategies
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct WaveLimits {
    /// Maximum absolute fixture coordinate, in units.
    max_abs_units: i64,
    /// Maximum sampled wave line spacing, in units.
    max_spacing_units: i64,
    /// Maximum sampled deposited bead width (`flow_width · flow_ratio`), units.
    max_width_units: i64,
    /// Maximum capped-mode iteration bound.
    max_iterations: u32,
    /// Maximum lattice cell count.
    max_cells: u32,
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

/// Checked bound construction. Every key is required and must be positive; no
/// key ever falls back to a default.
fn checked_limits(bounds: &BTreeMap<String, u64>) -> Result<WaveLimits, DomainError> {
    let max_abs_units = i64::try_from(required(bounds, "max_abs_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_spacing_units = i64::try_from(required(bounds, "max_spacing_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_width_units = i64::try_from(required(bounds, "max_width_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_iterations = u32::try_from(required(bounds, "max_iterations")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_cells = u32::try_from(required(bounds, "max_cells")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    if max_abs_units < FIXTURE_MAX_ABS_UNITS {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units must contain the fixture's support bands ({FIXTURE_MAX_ABS_UNITS} \
             units)"
        )));
    }
    if max_spacing_units < MAX_LINE_SPACING_UNITS {
        return Err(DomainError::InvalidBounds(format!(
            "max_spacing_units must admit the widest sampled spacing \
             ({MAX_LINE_SPACING_UNITS} units)"
        )));
    }
    // The deposited width bounds `2r`; a quarter of the piece extent keeps the
    // first trim from collapsing.
    if max_width_units <= 0 || max_width_units > max_abs_units / 4 {
        return Err(DomainError::InvalidBounds(
            "max_width_units must be positive and at most a quarter of max_abs_units".to_owned(),
        ));
    }
    if max_width_units < MIN_DEPOSITED_WIDTH_UNITS {
        return Err(DomainError::InvalidBounds(format!(
            "max_width_units must admit the sampled deposited width (>= \
             {MIN_DEPOSITED_WIDTH_UNITS} units)"
        )));
    }
    if max_iterations == 0 {
        return Err(DomainError::InvalidBounds(
            "max_iterations must be positive (the capped mode samples 1..=cap)".to_owned(),
        ));
    }
    if max_cells < LATTICE_CELLS {
        return Err(DomainError::InvalidBounds(format!(
            "max_cells must hold the declared {LATTICE_SIDE}x{LATTICE_SIDE} lattice \
             ({LATTICE_CELLS} cells)"
        )));
    }
    Ok(WaveLimits {
        max_abs_units,
        max_spacing_units,
        max_width_units,
        max_iterations,
        max_cells,
    })
}

fn live_limits(run: &PropertyRun) -> WaveLimits {
    let mut bounds = BTreeMap::new();
    for key in WAVE_BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("wave bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_limits(&bounds).expect("wave bound consistency")
}

fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), FIXTURE_MAX_ABS_UNITS as u64),
        (
            "max_spacing_units".to_owned(),
            MAX_LINE_SPACING_UNITS as u64,
        ),
        ("max_width_units".to_owned(), 12_500),
        (
            "max_iterations".to_owned(),
            u64::from(MAX_CAPPED_ITERATIONS),
        ),
        ("max_cells".to_owned(), u64::from(LATTICE_CELLS)),
    ])
}

fn capped_iteration_cap(limits: &WaveLimits) -> u32 {
    limits.max_iterations.clamp(1, MAX_CAPPED_ITERATIONS)
}

/// Which fallback expectation the case belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WaveMode {
    /// `max_iterations = 0`: propagation runs to completion, so the case must
    /// have no fallbacks and complete sampled coverage.
    Full,
    /// A small iteration cap: `IterationResidual` is expected and coverage is
    /// not asserted.
    Capped,
}

const ALL_PATTERNS: [WavePattern; 3] = [
    WavePattern::Smart,
    WavePattern::Monotonic,
    WavePattern::Zigzag,
];

#[derive(Debug, Clone, Copy)]
struct WaveCase {
    mode: WaveMode,
    spacing_units: i64,
    width_units: i64,
    ratio_millis: i64,
    iterations: u32,
    params: WaveParams,
}

/// Deposited radius for the integer factors, in units, rounded the way the
/// generator's `f32` product rounds. Used only for the sampled-domain coupling
/// bound so strategy and revalidation agree exactly.
fn radius_units_of(width_units: i64, ratio_millis: i64) -> i64 {
    (width_units * ratio_millis + 1_000) / 2_000
}

/// Smallest sampled line spacing for a deposited radius, in units.
///
/// The first emitted front is the contour of the seed strip grown by one
/// spacing, so its inner edge sits at `seed_expansion + line_spacing` from the
/// piece boundary. Below `line_spacing ≈ r − seed_expansion` the whole first
/// front falls outside the centerline trim and is clipped away, which would
/// leave a capped run with no path at all. [`SEED_EXPANSION_UNITS`] is that
/// expansion with its disc-chord margin.
fn spacing_lower_units(radius_units: i64) -> i64 {
    (radius_units - SEED_EXPANSION_UNITS).max(MIN_LINE_SPACING_UNITS)
}

/// Largest sampled line spacing for a deposited radius, in units.
///
/// Two independent front sequences each advance at most one spacing per
/// iteration, so the pre-merge gap between adjacent emitted fronts is at most
/// `2 · line_spacing`. Sampled coverage treats a point farther than `r + 0.1`
/// from every centerline as a void, so the domain is restricted to
/// `line_spacing <= r + 0.1` — then every gap point is within `r + 0.1` of a
/// front even at the merge.
fn spacing_upper_units(radius_units: i64, max_spacing_units: i64) -> i64 {
    (radius_units + (MAX_INTERIOR_VOID_HALF_WIDTH_MM * UNITS_PER_MM) as i64)
        .min(max_spacing_units)
        .min(MAX_LINE_SPACING_UNITS)
}

fn wave_strategy(
    limits: &WaveLimits,
    modes: &[WaveMode],
    patterns: &[WavePattern],
) -> BoxedStrategy<WaveCase> {
    assert!(!modes.is_empty(), "at least one mode must be selected");
    assert!(
        !patterns.is_empty(),
        "at least one pattern must be selected"
    );
    let width_upper = MAX_FLOW_WIDTH_UNITS
        .min(limits.max_width_units * 1_000 / MAX_FLOW_RATIO_MILLIS)
        .max(MIN_FLOW_WIDTH_UNITS);
    let iteration_cap = capped_iteration_cap(limits);
    let max_spacing = limits.max_spacing_units;
    (
        prop::sample::select(patterns.to_vec()).no_shrink(),
        prop::sample::select(modes.to_vec()).no_shrink(),
        MIN_FLOW_WIDTH_UNITS..=width_upper,
        MIN_FLOW_RATIO_MILLIS..=MAX_FLOW_RATIO_MILLIS,
        0_i64..=1_000,
        1_u32..=iteration_cap,
    )
        .prop_map(
            move |(pattern, mode, width_units, ratio_millis, fraction, iterations)| {
                let radius_units = radius_units_of(width_units, ratio_millis);
                let lower = spacing_lower_units(radius_units);
                let upper = spacing_upper_units(radius_units, max_spacing).max(lower);
                let span = (upper - lower).max(0);
                let spacing_units = lower + span * fraction / 1_000;
                let iterations = if mode == WaveMode::Full {
                    0
                } else {
                    iterations.max(1)
                };
                WaveCase {
                    mode,
                    spacing_units,
                    width_units,
                    ratio_millis,
                    iterations,
                    params: params_from(
                        spacing_units,
                        width_units,
                        ratio_millis,
                        iterations,
                        pattern,
                    ),
                }
            },
        )
        .boxed()
}

/// Re-validate every exposed input of a (possibly shrunk) case.
fn revalidate_case(case: &WaveCase, limits: &WaveLimits) -> Result<(RectMm, f64, Lattice), String> {
    let pattern = case.params.pattern;
    if !ALL_PATTERNS.contains(&pattern) {
        return Err(format!("unknown pattern {pattern:?}"));
    }
    match case.mode {
        WaveMode::Full => {
            if case.iterations != 0 || case.params.max_iterations != 0 {
                return Err("the Full class must not set an iteration cap".to_owned());
            }
        }
        WaveMode::Capped => {
            let cap = capped_iteration_cap(limits);
            if case.iterations == 0
                || case.iterations > cap
                || case.params.max_iterations != case.iterations
            {
                return Err(format!(
                    "the Capped class iteration bound {} is outside 1..={cap}",
                    case.iterations
                ));
            }
        }
    }
    if !(MIN_FLOW_WIDTH_UNITS..=MAX_FLOW_WIDTH_UNITS).contains(&case.width_units) {
        return Err(format!(
            "flow width {} units is out of range",
            case.width_units
        ));
    }
    if !(MIN_FLOW_RATIO_MILLIS..=MAX_FLOW_RATIO_MILLIS).contains(&case.ratio_millis) {
        return Err(format!(
            "flow ratio {} millis is out of range",
            case.ratio_millis
        ));
    }
    let deposited_units = case.width_units * case.ratio_millis / 1_000;
    if deposited_units > limits.max_width_units {
        return Err(format!(
            "deposited width {deposited_units} units exceeds max_width_units {}",
            limits.max_width_units
        ));
    }
    let radius = deposited_radius(&case.params);
    if !(radius >= f64::from(EPSILON_MM)) {
        return Err(format!("deposited radius {radius} is below EPSILON_MM"));
    }
    let radius_units = radius_units_of(case.width_units, case.ratio_millis);
    let lower = spacing_lower_units(radius_units);
    let upper = spacing_upper_units(radius_units, limits.max_spacing_units).max(lower);
    if !(lower..=upper).contains(&case.spacing_units) {
        return Err(format!(
            "line spacing {} units is outside {lower}..={upper}",
            case.spacing_units
        ));
    }
    if case.params.line_spacing != units_to_mm(case.spacing_units)
        || case.params.flow_width != units_to_mm(case.width_units)
        || case.params.flow_ratio != (case.ratio_millis as f32) / 1000.0
    {
        return Err("a param does not match its source integer factor".to_owned());
    }
    if case.params.perimeter_overlap != 0.0
        || case.params.minimum_width != 0.0
        || case.params.min_length != 0.0
        || case.params.min_new_area != SEAM_MIN_NEW_AREA_MM2
        || case.params.base_spacing != SEAM_BASE_SPACING_MM
        || case.params.wall_count != SEAM_WALL_COUNT
    {
        return Err("a fixed fixture parameter changed".to_owned());
    }
    let piece = PIECE;
    piece.inscribed_disc(radius)?;
    if piece.width().min(piece.height()) <= 2.0 * radius {
        return Err("the first trim inset does not survive the piece extent".to_owned());
    }
    let lattice = Lattice::new(piece, LATTICE_SIDE);
    if lattice.cells() as u64 > u64::from(limits.max_cells) {
        return Err(format!(
            "the declared lattice ({} cells) exceeds max_cells {}",
            lattice.cells(),
            limits.max_cells
        ));
    }
    Ok((piece, radius, lattice))
}

// ---------------------------------------------------------------------------
// M-WAVE-TRIM property
// ---------------------------------------------------------------------------

// test-quality: determinism pin — the two `generate` calls with identical
// inputs pin bitwise stability (docs/22_test_quality.md §2.1 exception); the
// independent statements in this callback are the trim/capsule containment and
// the sampled coverage lattice, which do not read `generate`'s own output back
// as an expectation.
#[test]
fn proptest_wave_trim_and_coverage() {
    let run = PropertyRun::new(WAVE_ID, WAVE_TEST).expect("M-WAVE-TRIM registry context");
    let limits = live_limits(&run);
    let strategy = wave_strategy(&limits, &[WaveMode::Full, WaveMode::Capped], &ALL_PATTERNS);
    let calls = Cell::new(0_u64);
    let nonempty_path_cases = Cell::new(0_u64);
    let full_cases = Cell::new(0_u64);
    let capped_cases = Cell::new(0_u64);
    let coverage_cases = Cell::new(0_u64);
    let segments = Cell::new(0_u64);
    let interior_samples = Cell::new(0_u64);
    let interior_empty = Cell::new(0_u64);
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let (piece, radius, lattice) = match revalidate_case(&case, &limits) {
            Ok(values) => values,
            Err(message) => return fail(format!("case {case:?}: {message}")),
        };
        let first = run_case(&case);
        let second = run_case(&case);
        if !outputs_identical(&first, &second) {
            return fail(format!(
                "identical inputs produced different output (determinism pin): case {case:?}"
            ));
        }
        match check_wave_output(&first, piece, radius, &lattice, case.mode == WaveMode::Full) {
            Ok(report) => {
                if report.paths == 0 {
                    return fail(format!("case {case:?} emitted no path"));
                }
                nonempty_path_cases.set(nonempty_path_cases.get() + 1);
                segments.set(segments.get() + report.containment.segments as u64);
                interior_samples
                    .set(interior_samples.get() + report.coverage.interior_cells as u64);
                match case.mode {
                    WaveMode::Full => {
                        full_cases.set(full_cases.get() + 1);
                        coverage_cases.set(coverage_cases.get() + 1);
                        // Only the full class asserts complete coverage; a
                        // capped run legitimately leaves area unfilled.
                        interior_empty.set(
                            interior_empty.get() + report.coverage.interior_empty_cells as u64,
                        );
                    }
                    WaveMode::Capped => {
                        if !first.fallbacks.contains(&FallbackReason::IterationResidual) {
                            return fail(format!(
                                "capped case {case:?} did not report IterationResidual, got {:?}",
                                first.fallbacks
                            ));
                        }
                        capped_cases.set(capped_cases.get() + 1);
                    }
                }
                Ok(())
            }
            Err(message) => fail(format!("case {case:?}: {message}")),
        }
    })
    .expect("M-WAVE-TRIM trim and sampled coverage");
    assert!(calls.get() > 0);
    assert!(
        nonempty_path_cases.get() > 0,
        "no generated case emitted a wave path"
    );
    assert!(full_cases.get() > 0, "the Full class never ran");
    assert!(capped_cases.get() > 0, "the Capped class never ran");
    assert!(
        coverage_cases.get() > 0 && interior_samples.get() > 0,
        "no case exercised the coverage lattice"
    );
    assert_eq!(
        interior_empty.get(),
        0,
        "a sampled interior gap was accepted"
    );
    assert!(segments.get() > 0, "no segment was checked");
    println!(
        "M-WAVE-TRIM accepted_calls={} nonempty_path_cases={} full_cases={} capped_cases={} \
         coverage_cases={} segments={} interior_samples={} interior_empty={}",
        calls.get(),
        nonempty_path_cases.get(),
        full_cases.get(),
        capped_cases.get(),
        coverage_cases.get(),
        segments.get(),
        interior_samples.get(),
        interior_empty.get()
    );
}

// ---------------------------------------------------------------------------
// M-WAVE-TRIM deterministic controls (no context, no RNG)
// ---------------------------------------------------------------------------

fn synthetic_output(points: &[(f64, f64)]) -> WaveOutput {
    WaveOutput {
        paths: vec![points
            .iter()
            .map(|&(x, y)| Point2::from_mm(x as f32, y as f32))
            .collect()],
        ..WaveOutput::default()
    }
}

fn without_paths(output: &WaveOutput, remove: std::ops::Range<usize>) -> WaveOutput {
    let mut mutant = output.clone();
    mutant.paths.drain(remove);
    mutant
}

/// Delete the middle of the wave front whose vertical run is nearest `x_target`:
/// the path is split in two at `y_low` and `y_high`, removing the span between.
fn with_removed_middle_segment(
    output: &WaveOutput,
    x_target: f64,
    y_low: f64,
    y_high: f64,
) -> Option<WaveOutput> {
    let mut chosen: Option<(usize, usize, f64)> = None;
    for (path_index, path) in output.paths.iter().enumerate() {
        for (segment, window) in path.windows(2).enumerate() {
            let (ax, _) = point_mm(window[0]);
            let (bx, _) = point_mm(window[1]);
            if (ax - bx).abs() > 1.0e-9 {
                continue;
            }
            let score = (ax - x_target).abs();
            if chosen.is_none_or(|(_, _, best)| score < best) {
                chosen = Some((path_index, segment, score));
            }
        }
    }
    let (path_index, segment, _) = chosen?;
    let path = &output.paths[path_index];
    let (ax, ay) = point_mm(path[segment]);
    let (_, by) = point_mm(path[segment + 1]);
    let (top, bottom) = if ay > by { (ay, by) } else { (by, ay) };
    if !(bottom < y_low && top > y_high) {
        return None;
    }
    // `path[..=segment]` ends at the top of the run and `path[segment + 1..]`
    // starts at its bottom, so the cut points attach in that order.
    let mut head: Vec<Point2> = path[..=segment].to_vec();
    let mut tail: Vec<Point2> = path[segment + 1..].to_vec();
    head.push(Point2::from_mm(ax as f32, y_high as f32));
    tail.insert(0, Point2::from_mm(ax as f32, y_low as f32));
    let mut mutant = output.clone();
    mutant.paths[path_index] = head;
    mutant.paths.insert(path_index + 1, tail);
    Some(mutant)
}

#[test]
fn wave_controls() {
    let (spacing_units, width_units, ratio_millis) = seam_factors();
    let case = WaveCase {
        mode: WaveMode::Full,
        spacing_units,
        width_units,
        ratio_millis,
        iterations: 0,
        params: fixture_params(),
    };
    let radius = deposited_radius(&case.params);
    assert!(
        (radius - 0.375).abs() < 1.0e-9,
        "the seam fixture's deposited radius must be 0.375 mm, got {radius}"
    );
    let lattice = Lattice::new(PIECE, LATTICE_SIDE);

    // ---- 1. The required positive witness, established by execution. ------
    let baseline = run_case(&case);
    assert!(
        !baseline.paths.is_empty(),
        "the fixture must emit waves; got fallbacks {:?}",
        baseline.fallbacks
    );
    assert!(
        baseline.fallbacks.is_empty(),
        "the fixture must not fall back; got {:?}",
        baseline.fallbacks
    );
    let report = check_wave_output(&baseline, PIECE, radius, &lattice, true)
        .expect("the executed positive witness must pass the checker");
    assert!(report.paths > 0);
    assert!(
        report.containment.segments > 0,
        "the positive witness must contain real segments"
    );
    assert!(
        report.coverage.interior_cells > 0,
        "the coverage lattice must have a positive interior population"
    );
    assert!(
        report.coverage.covered_interior_cells > 0,
        "the positive witness must cover interior samples"
    );
    assert_eq!(report.coverage.interior_empty_cells, 0);
    // The trim is the analytic `P ⊖ disk(r)`; pin it as the executed
    // derivation of the value stated in the module docs.
    let trim = PIECE.inscribed_disc(radius).expect("fixture trim");
    assert!((trim.min_x - 0.375).abs() < 1.0e-9);
    assert!((trim.max_x - 5.625).abs() < 1.0e-9);

    // ---- 2. A changed repeat result is detected. --------------------------
    let repeat = run_case(&case);
    assert!(outputs_identical(&baseline, &repeat));
    let mut moved = repeat.clone();
    moved.paths[0][0].x += 1;
    assert!(
        !outputs_identical(&baseline, &moved),
        "a one-unit centerline change must break the determinism pin"
    );
    let mut extra_fallback = repeat.clone();
    extra_fallback
        .fallbacks
        .push(FallbackReason::MinLengthFiltered);
    assert!(
        !outputs_identical(&baseline, &extra_fallback),
        "a changed fallback list must break the determinism pin"
    );
    let mut grown = repeat;
    grown.filled[0].contour.points[0].y += 1;
    assert!(
        !outputs_identical(&baseline, &grown),
        "a one-unit `filled` contour change must break the determinism pin"
    );

    // ---- 3. Exact double-inset false-rejection regression. ---------------
    // P = [0,10]², r = 1 mm, T = [1,9]², centerline (1,2)->(1,8). The capsule
    // touches P at x = 0 and is NOT contained in T; a checker that demanded
    // `capsule ⊆ T` would reject it (double inset).
    let wide = RectMm {
        min_x: 0.0,
        min_y: 0.0,
        max_x: 10.0,
        max_y: 10.0,
    };
    let tangent = synthetic_output(&[(1.0, 2.0), (1.0, 8.0)]);
    assert!(
        check_containment(&tangent, wide, 1.0).is_ok(),
        "the exact boundary-tangent capsule must be accepted"
    );
    let wide_trim = wide.inscribed_disc(1.0).expect("wide trim");
    assert!(
        (wide_trim.min_x - 1.0).abs() < 1.0e-12 && (wide_trim.max_x - 9.0).abs() < 1.0e-12,
        "the wide control's trim must be [1,9]², got {wide_trim:?}"
    );
    assert!(
        check_containment(&tangent, wide_trim, 1.0).is_err(),
        "the double-inset checker (capsule inside T) must reject the true positive, which is \
         exactly the false rejection this control guards"
    );

    // ---- 4. Nominal-width radius mutation is rejected. -------------------
    // Same P, nominal width 1 mm, flow_ratio 2 => r = 1 mm. Centerline at
    // x = 0.75 reaches x = -0.25 mm, so it must be rejected at r = 1 but would
    // be accepted by the nominal-radius mistake r = 0.5.
    let nominal = synthetic_output(&[(0.75, 2.0), (0.75, 8.0)]);
    assert!(
        check_containment(&nominal, wide, 1.0).is_err(),
        "a capsule reaching x = -0.25 mm must be rejected at the deposited radius"
    );
    assert!(
        check_containment(&nominal, wide, 0.5).is_ok(),
        "the same input is what the nominal-radius (0.5 mm) mistake would accept"
    );
    assert!(
        boundary_allowance_mm() * 4.0 < 0.25,
        "the derived allowance must stay far below the 0.25 mm negative-control gap"
    );

    // ---- 5. A removed central wave is detected. -------------------------
    // (a) At the seam fixture's 0.35 mm spacing, removing a *single* front is
    // not a coverage defect: adjacent deposited beads still overlap
    // (2·0.35 = 0.7 < 2r = 0.75), and a wider removal opens a void that
    // reaches the piece boundary, which the declared boundary-connected model
    // excludes as a sliver. The central seam-closing pair is enclosed by its
    // neighbouring fronts, so deleting it re-creates the pre-fix seam void
    // that `waves_cover_domain_without_seam_gap` guards.
    let path_count = baseline.paths.len();
    assert!(
        path_count >= 4,
        "the seam fixture must emit enough fronts for a central deletion"
    );
    let removed_closing = without_paths(&baseline, path_count - 2..path_count);
    assert!(
        check_containment(&removed_closing, PIECE, radius).is_ok(),
        "the deletion mutation must not incidentally break containment"
    );
    let closing_gap = check_coverage(&removed_closing, PIECE, radius, &lattice)
        .expect("coverage on the closing-pass deletion");
    assert!(
        closing_gap.interior_empty_cells > 0,
        "removing the central seam-closing pair left no sampled interior gap"
    );

    // (b) The literal middle-segment deletion. On a 0.7 mm-spaced fixture the
    // fronts are 1.4 mm apart, so cutting the middle out of one front leaves
    // an enclosed 0.45 mm × 1.05 mm void between its two halves and its
    // neighbours — the case a single 0.35 mm-spaced front removal cannot
    // create. 0.7 mm is outside the property's conservative
    // `line_spacing <= r + 0.1` coupling; it exists only to make a
    // single-front removal physically meaningful.
    let wide_spaced = WaveCase {
        params: params_from(7_000, 4_000, 1_875, 0, WavePattern::Smart),
        spacing_units: 7_000,
        ..case
    };
    let wide_spacing_baseline = run_case(&wide_spaced);
    assert!(
        wide_spacing_baseline.fallbacks.is_empty(),
        "the wide-spaced fixture must not fall back: {:?}",
        wide_spacing_baseline.fallbacks
    );
    let wide_report = check_wave_output(&wide_spacing_baseline, PIECE, radius, &lattice, true)
        .expect("the wide-spaced fixture must pass the full checker");
    assert!(wide_report.coverage.covered_interior_cells > 0);
    // Front vertical runs sit at 0.745 + k·0.700 mm; 1.445 is the second front.
    let cut = with_removed_middle_segment(&wide_spacing_baseline, 1.445, 2.0, 4.0)
        .expect("the wide-spaced fixture must expose a cuttable vertical front");
    assert!(
        check_containment(&cut, PIECE, radius).is_ok(),
        "the middle-segment deletion must not incidentally break containment"
    );
    let cut_gap = check_coverage(&cut, PIECE, radius, &lattice)
        .expect("coverage on the middle-segment deletion");
    assert!(
        cut_gap.interior_empty_cells > 0,
        "removing a middle segment of one front left no sampled interior gap"
    );

    // ---- 6. A false fallback is detected. --------------------------------
    let mut false_fallback = baseline.clone();
    false_fallback
        .fallbacks
        .push(FallbackReason::MinLengthFiltered);
    assert!(
        check_wave_output(&false_fallback, PIECE, radius, &lattice, true).is_err(),
        "a fallback on the unbounded class must fail the checker"
    );
    // ... and the same geometry passes once the spurious fallback is gone, so
    // the rejection is the fallback contract and not an incidental break.
    assert!(check_wave_output(&baseline, PIECE, radius, &lattice, true).is_ok());

    // ---- 7. The declared fallback fixtures. ------------------------------
    // (a) Empty fill area -> NoOverhang, no paths, no filled area.
    let empty_area = generate(&[], &bridge_lower(), &case.params);
    assert_eq!(empty_area.fallbacks, vec![FallbackReason::NoOverhang]);
    assert!(empty_area.paths.is_empty() && empty_area.filled.is_empty());

    // (b) A 1 mm square whose 4 mm perimeter is below `min_length` = 5 mm.
    let short = WaveParams {
        min_length: 5.0,
        ..fixture_params()
    };
    let short_area = generate(&[rect_mm(0.0, 0.0, 1.0, 1.0)], &bridge_lower(), &short);
    assert_eq!(
        short_area.fallbacks,
        vec![FallbackReason::MinLengthFiltered]
    );
    assert!(short_area.paths.is_empty() && short_area.filled.is_empty());

    // (c) No lower support -> MissingAnchors, no paths. The reasons vector is
    // repeatable: EmptyOutput is appended too because assembly produced
    // nothing, so `fallbacks` is not a single-valued flag.
    let unsupported = generate(&bridge_infill(), &[], &case.params);
    assert!(
        unsupported
            .fallbacks
            .contains(&FallbackReason::MissingAnchors),
        "no-lower-support run must report MissingAnchors, got {:?}",
        unsupported.fallbacks
    );
    assert!(unsupported.paths.is_empty());

    // (d) One-iteration cap on the positive scene -> IterationResidual while
    // real first-iteration progress exists (paths and filled are nonempty).
    let capped = WaveCase {
        params: params_from(
            spacing_units,
            width_units,
            ratio_millis,
            1,
            WavePattern::Smart,
        ),
        iterations: 1,
        ..case
    };
    let capped_area = run_case(&capped);
    assert_eq!(
        capped_area.fallbacks,
        vec![FallbackReason::IterationResidual]
    );
    assert!(
        !capped_area.paths.is_empty(),
        "the capped run must witness first-iteration progress"
    );
    assert!(
        !capped_area.filled.is_empty(),
        "the capped run must record the reached filled area"
    );
    // Paths, filled and fallbacks coexist; only the *full* class forbids them.
    let capped_report = check_wave_output(&capped_area, PIECE, radius, &lattice, false)
        .expect("the capped output's geometry must still be contained");
    assert!(capped_report.paths > 0);
    assert!(check_wave_output(&capped_area, PIECE, radius, &lattice, true).is_err());

    println!(
        "M-WAVE-TRIM controls: paths={} segments={} lattice_cells={} interior_cells={} \
         covered_interior={} boundary_empty={} closing_deleted_gap={} segment_deleted_gap={}",
        report.paths,
        report.containment.segments,
        report.coverage.cells,
        report.coverage.interior_cells,
        report.coverage.covered_interior_cells,
        report.coverage.boundary_connected_empty_cells,
        closing_gap.interior_empty_cells,
        cut_gap.interior_empty_cells
    );
}

// ---------------------------------------------------------------------------
// M-WAVE-TRIM shrink probe and bound matrix
// ---------------------------------------------------------------------------

/// Effective integer parameters of a case, for reduction measurement.
fn ordinary_size(case: &WaveCase) -> i64 {
    case.spacing_units + case.width_units + case.ratio_millis + i64::from(case.iterations)
}

fn assert_no_shrink<S: Strategy>(runner: &mut TestRunner, strategy: &S, label: &str)
where
    S::Value: std::fmt::Debug,
{
    let mut tree = strategy.new_tree(runner).expect("no-shrink tree");
    let before = format!("{:?}", tree.current());
    assert!(!tree.simplify(), "{label} simplified");
    assert!(!tree.complicate(), "{label} complicated");
    assert_eq!(format!("{:?}", tree.current()), before);
}

/// Revalidate a case and run the real kernel plus the containment check on it.
fn revalidate_and_check(case: &WaveCase, limits: &WaveLimits) -> Result<WaveReport, String> {
    let (piece, radius, _lattice) = revalidate_case(case, limits)?;
    let output = run_case(case);
    let containment = check_containment(&output, piece, radius)?;
    Ok(WaveReport {
        paths: output.paths.len(),
        containment,
        coverage: CoverageReport::default(),
    })
}

#[test]
fn wave_shrink_bounds() {
    // ---- Bound-construction rejection matrix. ----------------------------
    let valid = probe_bound_map();
    let limits = checked_limits(&valid).expect("valid wave bounds");
    assert_eq!(limits.max_abs_units, FIXTURE_MAX_ABS_UNITS);
    for key in WAVE_BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        match checked_limits(&bounds) {
            Err(DomainError::InvalidBounds(message)) => assert!(!message.is_empty(), "{key}"),
            other => panic!("missing `{key}` must be InvalidBounds, got {other:?}"),
        }
        let mut zero = valid.clone();
        zero.insert(key.to_owned(), 0);
        match checked_limits(&zero) {
            Err(DomainError::InvalidBounds(message)) => assert!(!message.is_empty(), "{key}"),
            other => panic!("zero `{key}` must be InvalidBounds, got {other:?}"),
        }
    }
    for (key, value) in [
        ("max_abs_units", (FIXTURE_MAX_ABS_UNITS - 1) as u64),
        ("max_spacing_units", (MAX_LINE_SPACING_UNITS - 1) as u64),
        ("max_width_units", (MIN_DEPOSITED_WIDTH_UNITS - 1) as u64),
        ("max_cells", (LATTICE_CELLS - 1) as u64),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        assert!(
            matches!(checked_limits(&bounds), Err(DomainError::InvalidBounds(_))),
            "`{key}` = {value} must be rejected"
        );
    }
    let mut too_wide = valid.clone();
    too_wide.insert(
        "max_width_units".to_owned(),
        (limits.max_abs_units / 4 + 1) as u64,
    );
    assert!(matches!(
        checked_limits(&too_wide),
        Err(DomainError::InvalidBounds(_))
    ));
    for key in ["max_abs_units", "max_spacing_units", "max_width_units"] {
        let mut overflow = valid.clone();
        overflow.insert(key.to_owned(), i64::MAX as u64 + 1);
        assert!(
            matches!(
                checked_limits(&overflow),
                Err(DomainError::ArithmeticOverflow)
            ),
            "`{key}` overflow must be ArithmeticOverflow"
        );
    }
    for key in ["max_iterations", "max_cells"] {
        let mut overflow = valid.clone();
        overflow.insert(key.to_owned(), u32::MAX as u64 + 1);
        assert!(
            matches!(
                checked_limits(&overflow),
                Err(DomainError::ArithmeticOverflow)
            ),
            "`{key}` overflow must be ArithmeticOverflow"
        );
    }

    // ---- Mode selectors are shrink-proof (false-return state). ----------
    let mut runner = TestRunner::deterministic();
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(ALL_PATTERNS.to_vec()).no_shrink(),
        "pattern",
    );
    assert_no_shrink(
        &mut runner,
        &prop::sample::select(vec![WaveMode::Full, WaveMode::Capped]).no_shrink(),
        "mode",
    );

    // ---- Per mode/pattern probe: current, simplify, complicate, reduce. --
    //
    // `generate` is the expensive part, so the SUT runs on the initial state,
    // on the first simplified state and on the first repeated-complicate state;
    // *every* exposed input at every step is revalidated by the pure
    // `revalidate_case`, which is what the charter's "revalidating every
    // exposed input" requires. This keeps the probe bounded while still
    // executing the kernel across `current`/`simplify`/`complicate`.
    for mode in [WaveMode::Full, WaveMode::Capped] {
        for pattern in ALL_PATTERNS {
            let strategy = wave_strategy(&limits, &[mode], &[pattern]);
            let mut tree = strategy.new_tree(&mut runner).expect("wave shrink tree");
            let initial = tree.current();
            assert_eq!(initial.mode, mode);
            assert_eq!(initial.params.pattern, pattern);
            let initial_report =
                revalidate_and_check(&initial, &limits).expect("initial case must pass");
            assert!(initial_report.paths > 0, "initial case emitted no path");
            assert!(
                initial_report.containment.segments > 0,
                "initial case emitted no segment"
            );
            let initial_size = ordinary_size(&initial);

            let mut simplified = 0usize;
            let mut reduced_runs = 0usize;
            let mut last_size = initial_size;
            let mut simplified_checked = false;
            let mut exhausted = false;
            for _ in 0..SHRINK_STEPS {
                if !tree.simplify() {
                    exhausted = true;
                    break;
                }
                simplified += 1;
                let current = tree.current();
                assert_eq!(current.mode, mode);
                assert_eq!(current.params.pattern, pattern);
                revalidate_case(&current, &limits)
                    .expect("every simplified case must revalidate its inputs");
                let size = ordinary_size(&current);
                if size < last_size {
                    reduced_runs += 1;
                }
                last_size = size;
                if !simplified_checked {
                    simplified_checked = true;
                    let report = revalidate_and_check(&current, &limits)
                        .expect("the simplified state must stay contained");
                    assert!(report.paths > 0);
                }
            }
            assert!(simplified > 0, "the shrink probe never simplified");
            assert!(
                reduced_runs > 0 && last_size < initial_size,
                "{mode:?}/{pattern:?}: shrinking produced no real reduction \
                 ({initial_size} -> {last_size})"
            );
            assert!(
                exhausted || simplified == SHRINK_STEPS,
                "the shrink probe neither exhausted nor reached its step ceiling"
            );

            // A fresh tree for the repeated simplify/complicate states: after
            // `simplify()` has reported exhaustion the sibling tree can no
            // longer complicate, so the alternation has to start over. Each
            // exposed input is revalidated, and the first simplified and first
            // restored states each run the real kernel.
            let mut probe = strategy
                .new_tree(&mut runner)
                .expect("wave alternation tree");
            let probe_initial = probe.current();
            revalidate_case(&probe_initial, &limits).expect("alternation initial case");
            let mut pairs_simplified = 0usize;
            let mut pairs_complicated = 0usize;
            let mut simplified_checked = false;
            let mut complicated_checked = false;
            for _ in 0..4 {
                if !probe.simplify() {
                    break;
                }
                pairs_simplified += 1;
                let state = probe.current();
                assert_eq!(state.mode, mode);
                assert_eq!(state.params.pattern, pattern);
                revalidate_case(&state, &limits)
                    .expect("a simplified alternation state must revalidate");
                if !simplified_checked {
                    simplified_checked = true;
                    let report = revalidate_and_check(&state, &limits)
                        .expect("the simplified alternation state must stay contained");
                    assert!(report.paths > 0);
                }
                if !probe.complicate() {
                    break;
                }
                pairs_complicated += 1;
                let restored = probe.current();
                assert_eq!(restored.mode, mode);
                assert_eq!(restored.params.pattern, pattern);
                revalidate_case(&restored, &limits)
                    .expect("a repeated-complicate state must revalidate");
                if !complicated_checked {
                    complicated_checked = true;
                    let report = revalidate_and_check(&restored, &limits)
                        .expect("the repeated-complicate state must stay contained");
                    assert!(report.paths > 0);
                }
            }
            assert!(
                pairs_simplified > 0 && pairs_complicated > 0,
                "{mode:?}/{pattern:?}: the alternation never exercised \
                 simplify/complicate (simplified={pairs_simplified}, \
                 complicated={pairs_complicated})"
            );
            println!(
                "{mode:?}/{pattern:?}: simplify={simplified} reductions={reduced_runs} \
                 exhausted={exhausted} alternation={pairs_simplified}/{pairs_complicated} \
                 size={initial_size}->{last_size} simplify_checked={simplified_checked} \
                 complicate_checked={complicated_checked}"
            );
        }
    }
}
