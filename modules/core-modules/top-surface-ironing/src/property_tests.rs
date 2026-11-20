//! Opt-in property tests for this module's private ironing zigzag kernel.
//!
//! Charter (`docs/spec_packets/proptest-openrscad_13_pbt-infill`):
//!
//! * `M-IRONING-ZIGZAG` — the complete expected scan-interval set plus **every**
//!   consecutive segment of every returned `ExtrusionPath3D`, so the joins
//!   between intervals and between rows are checked too. A checker that only
//!   evaluated `chunks(2)` would miss exactly the defect this property found.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `generate_zigzag_strokes_for_polygon` is private, so the properties live
//! here as a crate-internal unit-test module; production visibility is
//! deliberately not widened for tests.
//!
//! # Confirmed defect and focused fix
//!
//! The pre-fix kernel concatenated every row's intervals into ONE point vector
//! per polygon, so the implicit move between two disjoint intervals of one row
//! became an extruded segment straight through the gap. On the deterministic
//! 8 mm square with hole `[3,5] x [2,6]` mm at 1 mm spacing the formerly
//! failing segment was `(5,3) -> (3,3)` mm (both endpoints on material, the
//! open segment inside the hole). The kernel now starts a new path whenever the
//! next interval does not start at the x the previous interval ended on, so
//! every implicit join is a vertical move at a shared x on the outer boundary.
//! The no-RNG regression lives in `tests/top_surface_ironing_emission_tdd.rs`
//! (`hole_crossing_regression_no_ironing_segment_enters_the_hole`) and fails on
//! the old kernel.
//!
//! # Exact fixture grid
//!
//! Every generated coordinate and spacing is a multiple of 0.25 mm (2500
//! canonical units). `units_to_mm` then produces exactly representable `f32`
//! millimetres, and `bb.y_min + i * spacing` stays exact because all operands
//! are binary fractions of small magnitude. The independent model can therefore
//! be derived in `f64` arithmetic and compared with a small derived envelope
//! instead of an observed epsilon.
//!
//! # Literal review (AC-6)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. Watched-type sites here:
//!
//! * `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`) — every literal in
//!   `iron_point` states `x`/`y` and uses `..Default::default()` for the six
//!   remaining fields (FRU).
//! * `ExtrusionPath3D` (`crates/slicer-ir/src/slice_ir.rs`) — the synthetic
//!   mutant paths in `ironing_controls` set `points` and use
//!   `..extrusion_path3d_base(ExtrusionRole::Ironing)` (FRU).
//! * `GeometryBounds` (`crates/slicer-sdk/src/test_support/property_testing/
//!   domains/mod.rs`) — two literals, each carrying an `// exhaustive:` note
//!   because all five measured bounds are explicit.
//! * `IronCase`, `IronLimits`, `ExpectedRow` and the match fixtures are private
//!   module-local types, not watched crate-public structs.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! controls below are mutation controls over the SAME checker the property
//! uses; none of them is a decorative assertion.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    units_to_mm, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth, Polygon,
};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::test_support::property_testing::domains::{
    BoundaryRule, DomainError, GeometryBounds, GeometryCase, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{segment_in_region, validate_geometry};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::{bbox_of_expoly, generate_zigzag_strokes_for_polygon, BBox2D, IRONING_LINE_WIDTH};

const PROPERTY_ID: &str = "M-IRONING-ZIGZAG";
const PROPERTY_TEST: &str = "property_tests::proptest_ironing_full_paths";
const BOUND_KEYS: [&str; 7] = [
    "max_abs_units",
    "max_vertices",
    "max_holes",
    "max_components",
    "max_shrink_candidates",
    "max_spacing_units",
    "max_rows",
];

/// Fixture grid: 0.25 mm is exact in `f32`, so every generated coordinate and
/// spacing converts to millimetres without rounding.
const GRID_UNITS: i64 = 2_500;
/// Minimum positive interval the kernel emits (`(b - a) <= 1e-4` is dropped).
const MIN_INTERVAL_MM: f64 = 1e-4;
/// Derived `f32` comparison envelope coefficient (see `coordinate_tolerance`).
const TOLERANCE_K: f64 = 64.0;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

// The count fields are validated for the profile contract even though the
// generator only needs the spatial ceilings; keeping them in one struct makes
// every key mandatory instead of silently optional.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
struct IronLimits {
    max_abs_units: i64,
    max_vertices: u32,
    max_holes: u32,
    max_components: u32,
    max_shrink_candidates: u32,
    max_spacing_units: i64,
    max_rows: u32,
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

fn checked_limits(bounds: &BTreeMap<String, u64>) -> Result<IronLimits, DomainError> {
    let max_abs_units = i64::try_from(required(bounds, "max_abs_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_vertices = u32::try_from(required(bounds, "max_vertices")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_holes = u32::try_from(required(bounds, "max_holes")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_components = u32::try_from(required(bounds, "max_components")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_shrink_candidates = u32::try_from(required(bounds, "max_shrink_candidates")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_spacing_units = i64::try_from(required(bounds, "max_spacing_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_rows = u32::try_from(required(bounds, "max_rows")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;

    if max_abs_units < 2 * GRID_UNITS || max_abs_units % GRID_UNITS != 0 {
        return Err(DomainError::InvalidBounds(
            "max_abs_units must be a positive multiple of the 0.25 mm fixture grid".to_owned(),
        ));
    }
    if max_spacing_units < GRID_UNITS || max_spacing_units % GRID_UNITS != 0 {
        return Err(DomainError::InvalidBounds(
            "max_spacing_units must be a positive multiple of the 0.25 mm fixture grid".to_owned(),
        ));
    }
    if !(4..=64).contains(&max_rows) {
        return Err(DomainError::InvalidBounds(
            "max_rows must be in 4..=64: the derived f32 envelope assumes at most 64 rows, \
             and at least four rows are needed to place a gap that a scan row really crosses"
                .to_owned(),
        ));
    }
    if max_vertices < 8 || max_holes < 1 || max_components < 1 || max_shrink_candidates < 1 {
        return Err(DomainError::InvalidBounds(
            "max_vertices >= 8, max_holes >= 1, max_components >= 1 and \
             max_shrink_candidates >= 1 are required by the generated classes"
                .to_owned(),
        ));
    }
    let room =
        u32::try_from(max_abs_units / GRID_UNITS).map_err(|_| DomainError::ArithmeticOverflow)?;
    if room < 16 {
        return Err(DomainError::InvalidBounds(
            "max_abs_units must allow at least a 4 mm fixture".to_owned(),
        ));
    }
    Ok(IronLimits {
        max_abs_units,
        max_vertices,
        max_holes,
        max_components,
        max_shrink_candidates,
        max_spacing_units,
        max_rows,
    })
}

fn live_limits(run: &PropertyRun) -> IronLimits {
    let mut bounds = BTreeMap::new();
    for key in BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("ironing bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_limits(&bounds).expect("ironing bound consistency")
}

fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 80_000),
        ("max_vertices".to_owned(), 16),
        ("max_holes".to_owned(), 2),
        ("max_components".to_owned(), 1),
        ("max_shrink_candidates".to_owned(), 16),
        ("max_spacing_units".to_owned(), 15_000),
        ("max_rows".to_owned(), 32),
    ])
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures and the independent expected model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeKind {
    Rect,
    Notch,
    Hole,
}

const SHAPE_MODES: [ShapeKind; 3] = [ShapeKind::Rect, ShapeKind::Notch, ShapeKind::Hole];

impl ShapeKind {
    fn label(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Notch => "notch",
            Self::Hole => "hole",
        }
    }

    fn topology(self) -> TopologyClass {
        match self {
            Self::Rect => TopologyClass::Convex,
            Self::Notch => TopologyClass::Concave,
            Self::Hole => TopologyClass::Holed,
        }
    }

    fn min_holes(self) -> usize {
        usize::from(self == Self::Hole)
    }
}

/// One bounded axis-aligned ironing fixture, all lengths in canonical units.
#[derive(Debug, Clone, PartialEq)]
struct IronCase {
    shape: ShapeKind,
    x0: i64,
    y0: i64,
    width: i64,
    height: i64,
    /// Notch/hole rectangle in absolute units: `(x0, x1, y0, y1)`.
    cut: Option<(i64, i64, i64, i64)>,
    spacing: i64,
    z: f32,
    flow_factor: f32,
    speed_factor: f32,
}

impl IronCase {
    fn x1(&self) -> i64 {
        self.x0 + self.width
    }

    fn y1(&self) -> i64 {
        self.y0 + self.height
    }

    fn rect_mm(&self) -> (f64, f64, f64, f64) {
        (
            f64::from(units_to_mm(self.x0)),
            f64::from(units_to_mm(self.y0)),
            f64::from(units_to_mm(self.x1())),
            f64::from(units_to_mm(self.y1())),
        )
    }

    fn spacing_mm(&self) -> f64 {
        f64::from(units_to_mm(self.spacing))
    }

    fn expoly(&self) -> ExPolygon {
        let pt = |x: i64, y: i64| Point2 { x, y };
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1(), self.y1());
        match self.shape {
            ShapeKind::Rect => ExPolygon {
                contour: Polygon {
                    points: vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)],
                },
                holes: Vec::new(),
            },
            ShapeKind::Notch => {
                let (nx0, nx1, ny0, _) = self.cut.expect("notch case carries a cut");
                ExPolygon {
                    contour: Polygon {
                        points: vec![
                            pt(x0, y0),
                            pt(x1, y0),
                            pt(x1, y1),
                            pt(nx1, y1),
                            pt(nx1, ny0),
                            pt(nx0, ny0),
                            pt(nx0, y1),
                            pt(x0, y1),
                        ],
                    },
                    holes: Vec::new(),
                }
            }
            ShapeKind::Hole => {
                let (hx0, hx1, hy0, hy1) = self.cut.expect("hole case carries a cut");
                ExPolygon {
                    contour: Polygon {
                        points: vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)],
                    },
                    holes: vec![Polygon {
                        points: vec![pt(hx0, hy0), pt(hx0, hy1), pt(hx1, hy1), pt(hx1, hy0)],
                    }],
                }
            }
        }
    }

    fn geometry_domain(&self) -> GeometryDomain {
        GeometryDomain {
            class: self.shape.topology(),
            min_holes: self.shape.min_holes(),
            min_components: 1,
            boundary: BoundaryRule::Inclusive,
            // exhaustive: all five measured bounds are supplied; the default is only a helper base.
            bounds: GeometryBounds {
                max_abs_units: self
                    .x0
                    .abs()
                    .max(self.y0.abs())
                    .max(self.x1().abs())
                    .max(self.y1().abs())
                    + GRID_UNITS,
                max_vertices: 16,
                max_holes: 2,
                max_components: 1,
                max_shrink_candidates: 16,
            },
        }
    }

    /// Half-open interior X intervals of the row at `y_mm`, from fixture edges.
    ///
    /// The top edge is excluded (`y >= y_max`), matching the half-open vertex
    /// rule the module documents; a hole's top edge is excluded and its bottom
    /// edge included.
    fn intervals_at(&self, y_mm: f64) -> Vec<(f64, f64)> {
        let (x0, y0, x1, y1) = self.rect_mm();
        if !(y_mm >= y0 && y_mm < y1) {
            return Vec::new();
        }
        let x = |units: i64| f64::from(units_to_mm(units));
        match self.shape {
            ShapeKind::Rect => vec![(x0, x1)],
            ShapeKind::Notch => {
                let (nx0, nx1, ny0, _) = self.cut.expect("notch case carries a cut");
                if y_mm >= x(ny0) {
                    vec![(x0, x(nx0)), (x(nx1), x1)]
                } else {
                    vec![(x0, x1)]
                }
            }
            ShapeKind::Hole => {
                let (hx0, hx1, hy0, hy1) = self.cut.expect("hole case carries a cut");
                if y_mm >= x(hy0) && y_mm < x(hy1) {
                    vec![(x0, x(hx0)), (x(hx1), x1)]
                } else {
                    vec![(x0, x1)]
                }
            }
        }
    }

    /// The complete expected point sequence, in emission order.
    fn expected_points(&self) -> Result<Vec<(f64, f64)>, String> {
        let (_, y0, _, y1) = self.rect_mm();
        let spacing = self.spacing_mm();
        if !(spacing.is_finite() && spacing > 0.0) {
            return Err(format!("nonpositive spacing {:?}", self.spacing));
        }
        let span = y1 - y0;
        if span <= 0.0 {
            return Ok(Vec::new());
        }
        let n_rows = (span / spacing).floor() as usize + 1;
        let mut points = Vec::new();
        for i in 0..n_rows {
            let y = (y0 + i as f64 * spacing).min(y1);
            let mut intervals = self.intervals_at(y);
            if i % 2 == 1 {
                intervals.reverse();
            }
            for (a, b) in intervals {
                if b - a <= MIN_INTERVAL_MM {
                    continue;
                }
                let (xa, xb) = if i % 2 == 1 { (b, a) } else { (a, b) };
                points.push((xa, y));
                points.push((xb, y));
            }
        }
        Ok(points)
    }
}

/// Derived absolute comparison envelope for exactly representable fixtures.
///
/// Operations on the path: canonical-unit -> `f32` mm (one rounding), row
/// `y = y_min + i * spacing` (at most 64 exact-grid additions, each with
/// relative error <= u), and the independent model's `f64` conversion (exact
/// for these values). With `u = f32::EPSILON / 2`, `K * f32::EPSILON` gives
/// `2K` unit roundoffs of headroom; `K = 64` is twice the largest allowed row
/// count.
fn coordinate_tolerance(max_abs_mm: f64, span_mm: f64) -> f64 {
    TOLERANCE_K * f64::from(f32::EPSILON) * (max_abs_mm + span_mm + 1.0)
}

fn iron_point(x_mm: f32, y_mm: f32, case: &IronCase) -> Point3WithWidth {
    Point3WithWidth {
        x: x_mm,
        y: y_mm,
        z: case.z,
        width: IRONING_LINE_WIDTH,
        flow_factor: case.flow_factor,
        // exhaustive: the three remaining fields (`overhang_quartile`, `dist_to_top_mm`,
        // `overhang_distance_mm`) are the documented default shape.
        ..Default::default()
    }
}

fn run_kernel(case: &IronCase) -> Vec<ExtrusionPath3D> {
    let poly = case.expoly();
    let bb: BBox2D = bbox_of_expoly(&poly).expect("nonempty contour");
    generate_zigzag_strokes_for_polygon(
        &poly,
        &bb,
        case.z,
        case.spacing_mm(),
        case.flow_factor,
        case.speed_factor,
    )
}

struct CheckStats {
    points: usize,
    segments: usize,
    left: usize,
    right: usize,
    rows_with_gap: usize,
}

/// The one checker the property and every control use.
fn check_ironing_output(
    case: &IronCase,
    paths: &[ExtrusionPath3D],
    require_nonempty: bool,
) -> Result<CheckStats, String> {
    let poly = case.expoly();
    validate_geometry(
        &GeometryCase {
            polygons: vec![poly.clone()],
        },
        &case.geometry_domain(),
    )
    .map_err(|error| format!("fixture is not valid geometry: {error}"))?;

    let expected = case.expected_points()?;
    let mut actual: Vec<(f32, f32)> = Vec::new();
    let mut stats = CheckStats {
        points: 0,
        segments: 0,
        left: 0,
        right: 0,
        rows_with_gap: 0,
    };
    let (_, _, _, y1) = case.rect_mm();
    let (max_abs_mm, span_mm) = {
        let (x0, y0, x1, y1m) = case.rect_mm();
        (
            x0.abs().max(x1.abs()).max(y0.abs()).max(y1m.abs()),
            y1m - y0,
        )
    };
    let tolerance = coordinate_tolerance(max_abs_mm, span_mm);
    let gap_low = case
        .cut
        .map_or(f64::INFINITY, |(cx0, _, _, _)| f64::from(units_to_mm(cx0)));
    let gap_high = case.cut.map_or(f64::NEG_INFINITY, |(_, cx1, _, _)| {
        f64::from(units_to_mm(cx1))
    });

    for path in paths {
        if path.role != ExtrusionRole::Ironing {
            return Err(format!("path role is {:?}, expected Ironing", path.role));
        }
        if path.speed_factor.to_bits() != case.speed_factor.to_bits() {
            return Err(format!(
                "path speed_factor {} differs from the input {}",
                path.speed_factor, case.speed_factor
            ));
        }
        if path.tool_index.is_some() {
            return Err("path carries a tool index the input did not request".to_owned());
        }
        if path.order_lock.is_some() {
            return Err("path carries an order lock the input did not request".to_owned());
        }
        if path.points.len() < 2 {
            return Err(format!(
                "path has {} point(s); every emitted path needs at least two",
                path.points.len()
            ));
        }
        for point in &path.points {
            if point.z.to_bits() != case.z.to_bits() {
                return Err(format!(
                    "point z {} differs from the input z {}",
                    point.z, case.z
                ));
            }
            if point.width.to_bits() != IRONING_LINE_WIDTH.to_bits() {
                return Err(format!(
                    "point width {} differs from the module width {IRONING_LINE_WIDTH}",
                    point.width
                ));
            }
            if point.flow_factor.to_bits() != case.flow_factor.to_bits() {
                return Err(format!(
                    "point flow_factor {} differs from the input {}",
                    point.flow_factor, case.flow_factor
                ));
            }
            if point.overhang_quartile.is_some()
                || point.dist_to_top_mm != 0.0
                || point.overhang_distance_mm.is_some()
            {
                return Err(
                    "point carries annotation fields the ironing input does not set".to_owned(),
                );
            }
            if !(point.x.is_finite() && point.y.is_finite()) {
                return Err(format!("nonfinite point ({}, {})", point.x, point.y));
            }
            actual.push((point.x, point.y));
            stats.points += 1;
        }
        for window in path.points.windows(2) {
            let a = Point2::from_mm(window[0].x, window[0].y);
            let b = Point2::from_mm(window[1].x, window[1].y);
            if !segment_in_region([a, b], std::slice::from_ref(&poly), BoundaryRule::Inclusive)
                .map_err(|error| format!("containment predicate failed: {error}"))?
            {
                return Err(format!(
                    "segment ({}, {}) -> ({}, {}) mm leaves the top-fill region",
                    window[0].x, window[0].y, window[1].x, window[1].y
                ));
            }
            stats.segments += 1;
            let mid_x = (f64::from(window[0].x) + f64::from(window[1].x)) / 2.0;
            if case.cut.is_some() {
                if mid_x < gap_low {
                    stats.left += 1;
                } else if mid_x > gap_high {
                    stats.right += 1;
                }
            }
        }
    }

    if actual.len() != expected.len() {
        return Err(format!(
            "emitted {} point(s), expected {} from the independent interval model",
            actual.len(),
            expected.len()
        ));
    }
    for (index, ((ax, ay), (ex, ey))) in actual.iter().zip(&expected).enumerate() {
        if (f64::from(*ax) - ex).abs() > tolerance || (f64::from(*ay) - ey).abs() > tolerance {
            return Err(format!(
                "point {index} is ({ax}, {ay}) mm, independent model expects ({ex}, {ey}) mm \
                 (envelope {tolerance:.3e}); top edge {y1}"
            ));
        }
    }

    // Count rows that really contain a gap between two intervals, so the
    // "both sides observed" counters below cannot be satisfied by a shape whose
    // cut never intersects a scan row.
    if case.cut.is_some() {
        let mut per_row: BTreeMap<i64, usize> = BTreeMap::new();
        for (x, y) in &actual {
            let key = slicer_ir::mm_to_units(*y);
            if f64::from(*x) <= gap_low || f64::from(*x) >= gap_high {
                *per_row.entry(key).or_insert(0) += 1;
            }
        }
        stats.rows_with_gap = per_row.values().filter(|count| **count >= 4).count();
    }

    if require_nonempty && (stats.points == 0 || stats.segments == 0) {
        return Err("a nonempty fixture produced no ironing geometry".to_owned());
    }
    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy
// ─────────────────────────────────────────────────────────────────────────────

fn build_case(shape: ShapeKind, raw: [u32; 10], limits: &IronLimits) -> IronCase {
    let max_abs_q = (limits.max_abs_units / GRID_UNITS).max(8);
    // At least two quarters (0.5 mm) of spacing, so the placed cut's bottom
    // edge always lands on an interior scan row with material above it.
    let max_spacing_q = (limits.max_spacing_units / GRID_UNITS).clamp(2, 6);
    let spacing_q = 2 + i64::from(raw[0]) % (max_spacing_q - 1);
    // 4..=40 quarters is 1..=10 mm and leaves room for a cut with one-quarter
    // margins on both sides.
    let width_q = 4 + i64::from(raw[1]) % 37;
    let mut height_q = 4 + i64::from(raw[2]) % 37;
    // Keep at least two rows' worth of material so the placed cut always has an
    // interior row above its bottom edge, and stay inside the coordinate bound.
    let height_upper = (spacing_q * i64::from(limits.max_rows)).min(max_abs_q);
    height_q = height_q.clamp(2 * spacing_q, height_upper);
    let x_room = (2 * max_abs_q - width_q).max(0);
    let y_room = (2 * max_abs_q - height_q).max(0);
    let x0_q = -max_abs_q + i64::from(raw[3]) % (x_room + 1);
    let y0_q = -max_abs_q + i64::from(raw[4]) % (y_room + 1);
    let x0 = x0_q * GRID_UNITS;
    let y0 = y0_q * GRID_UNITS;
    let width = width_q * GRID_UNITS;
    let height = height_q * GRID_UNITS;

    let cut = match shape {
        ShapeKind::Rect => None,
        _ => {
            // Horizontal cut with at least one quarter of material on both sides.
            let available = width_q - 1;
            let left_q = 1 + i64::from(raw[5]) % (available - 2).max(1);
            let max_cut_q = (available - left_q - 1).max(1);
            let cut_w_q = 1 + i64::from(raw[6]) % max_cut_q;
            // Place the cut's bottom edge exactly on an interior scan row
            // (`y0 + k * spacing`). That row is included by the half-open
            // rule, so every generated hole or notch case really splits a row.
            // A hole keeps at least one quarter of material above it; a notch
            // reaches the top edge.
            let rows = height_q / spacing_q;
            let row_index = 1 + i64::from(raw[7]) % (rows - 1).max(1);
            let bottom_units = row_index * spacing_q * GRID_UNITS;
            let max_cut_h = ((height - 1 - bottom_units) / GRID_UNITS).max(1);
            let cut_h_q = 1 + i64::from(raw[8]) % max_cut_h;
            let cx0 = x0 + left_q * GRID_UNITS;
            let cx1 = cx0 + cut_w_q * GRID_UNITS;
            let cy0 = y0 + bottom_units;
            let cy1 = match shape {
                ShapeKind::Hole => cy0 + cut_h_q * GRID_UNITS,
                ShapeKind::Notch => y0 + height,
                ShapeKind::Rect => unreachable!(),
            };
            Some((cx0, cx1, cy0, cy1))
        }
    };

    IronCase {
        shape,
        x0,
        y0,
        width,
        height,
        cut,
        spacing: spacing_q * GRID_UNITS,
        z: 0.05 + (raw[9] % 200) as f32 * 0.05,
        flow_factor: 0.05 + (raw[0] % 19) as f32 * 0.05,
        speed_factor: 0.1 + (raw[1] % 19) as f32 * 0.1,
    }
}

fn iron_case_strategy(limits: &IronLimits, shape: ShapeKind) -> BoxedStrategy<IronCase> {
    let limits = *limits;
    (
        prop::sample::select(vec![shape]).no_shrink(),
        any::<[u32; 10]>(),
    )
        .prop_map(move |(shape, raw)| build_case(shape, raw, &limits))
        .boxed()
}

fn case_bits(case: &IronCase) -> Vec<u64> {
    let mut bits = vec![
        case.x0 as u64,
        case.y0 as u64,
        case.width as u64,
        case.height as u64,
        case.spacing as u64,
        u64::from(case.z.to_bits()),
        u64::from(case.flow_factor.to_bits()),
        u64::from(case.speed_factor.to_bits()),
    ];
    if let Some((a, b, c, d)) = case.cut {
        bits.extend([a as u64, b as u64, c as u64, d as u64]);
    }
    bits
}

// ─────────────────────────────────────────────────────────────────────────────
// M-IRONING-ZIGZAG
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_ironing_full_paths() {
    let run = PropertyRun::new(PROPERTY_ID, PROPERTY_TEST).expect("M-IRONING-ZIGZAG context");
    let limits = live_limits(&run);
    let strategy = (
        prop::sample::select(SHAPE_MODES.to_vec()).no_shrink(),
        any::<[u32; 10]>(),
    )
        .prop_map(move |(shape, raw)| build_case(shape, raw, &limits))
        .boxed();

    let calls = Cell::new(0_u64);
    let points = Cell::new(0_u64);
    let segments = Cell::new(0_u64);
    let rect_cases = Cell::new(0_u64);
    let notch_cases = Cell::new(0_u64);
    let hole_cases = Cell::new(0_u64);
    let gap_rows = Cell::new(0_u64);
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        let paths = run_kernel(&case);
        match check_ironing_output(&case, &paths, true) {
            Ok(stats) => {
                points.set(points.get() + stats.points as u64);
                segments.set(segments.get() + stats.segments as u64);
                match case.shape {
                    ShapeKind::Rect => rect_cases.set(rect_cases.get() + 1),
                    ShapeKind::Notch => {
                        notch_cases.set(notch_cases.get() + 1);
                        if stats.left == 0 || stats.right == 0 {
                            return fail(format!(
                                "notch case produced no {} arm",
                                if stats.left == 0 { "left" } else { "right" }
                            ));
                        }
                        if stats.rows_with_gap == 0 {
                            return fail("notch case never split a scan row");
                        }
                    }
                    ShapeKind::Hole => {
                        hole_cases.set(hole_cases.get() + 1);
                        if stats.left == 0 || stats.right == 0 {
                            return fail(format!(
                                "hole case produced no {} side",
                                if stats.left == 0 { "left" } else { "right" }
                            ));
                        }
                        if stats.rows_with_gap == 0 {
                            return fail("hole case never split a scan row");
                        }
                    }
                }
                gap_rows.set(gap_rows.get() + stats.rows_with_gap as u64);
                Ok(())
            }
            Err(message) => fail(format!("{} case failed: {message}", case.shape.label())),
        }
    })
    .expect("M-IRONING-ZIGZAG full-path containment and interval coverage");

    assert!(rect_cases.get() > 0 && notch_cases.get() > 0 && hole_cases.get() > 0);
    assert!(
        gap_rows.get() > 0,
        "no generated case ever split a scan row"
    );
    println!(
        "M-IRONING-ZIGZAG accepted_calls={} points={} segments={} rect={} notch={} hole={} gap_rows={}",
        calls.get(),
        points.get(),
        segments.get(),
        rect_cases.get(),
        notch_cases.get(),
        hole_cases.get(),
        gap_rows.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses, modes and mutations
// ─────────────────────────────────────────────────────────────────────────────

fn rect_case(x0: i64, y0: i64, width: i64, height: i64, spacing: i64) -> IronCase {
    IronCase {
        shape: ShapeKind::Rect,
        x0,
        y0,
        width,
        height,
        cut: None,
        spacing,
        z: 0.2,
        flow_factor: 0.1,
        speed_factor: 0.4,
    }
}

fn hole_case() -> IronCase {
    // 8 mm square, hole [3,5] x [2,6] mm, 1 mm rows.
    IronCase {
        shape: ShapeKind::Hole,
        x0: 0,
        y0: 0,
        width: 80_000,
        height: 80_000,
        cut: Some((30_000, 50_000, 20_000, 60_000)),
        spacing: 10_000,
        z: 1.0,
        flow_factor: 0.1,
        speed_factor: 0.4,
    }
}

fn notch_case() -> IronCase {
    // 10 x 10 mm square with a 4 x 6 mm notch in the top edge, 0.5 mm rows.
    IronCase {
        shape: ShapeKind::Notch,
        x0: -50_000,
        y0: -50_000,
        width: 100_000,
        height: 100_000,
        cut: Some((-20_000, 20_000, -10_000, 50_000)),
        spacing: 5_000,
        z: 0.3,
        flow_factor: 0.1,
        speed_factor: 0.4,
    }
}

fn point(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn ring(points: Vec<Point2>) -> Polygon {
    Polygon { points }
}

fn run_poly(
    poly: &ExPolygon,
    spacing_units: i64,
    z: f32,
    flow_factor: f32,
    speed_factor: f32,
) -> Vec<ExtrusionPath3D> {
    let bb = bbox_of_expoly(poly).expect("nonempty contour");
    generate_zigzag_strokes_for_polygon(
        poly,
        &bb,
        z,
        f64::from(units_to_mm(spacing_units)),
        flow_factor,
        speed_factor,
    )
}

fn assert_paths_contained(poly: &ExPolygon, paths: &[ExtrusionPath3D], what: &str) {
    assert!(!paths.is_empty(), "{what}: no path emitted");
    for path in paths {
        assert!(
            path.points.len() >= 2,
            "{what}: a path with fewer than two points was emitted"
        );
        for segment in path.points.windows(2) {
            let a = Point2::from_mm(segment[0].x, segment[0].y);
            let b = Point2::from_mm(segment[1].x, segment[1].y);
            let inside =
                segment_in_region([a, b], std::slice::from_ref(poly), BoundaryRule::Inclusive)
                    .expect("exact containment predicate");
            assert!(
                inside,
                "{what}: segment ({}, {}) -> ({}, {}) mm leaves the region",
                segment[0].x, segment[0].y, segment[1].x, segment[1].y
            );
        }
    }
}

fn merged_join_mutant(paths: &[ExtrusionPath3D]) -> Option<Vec<ExtrusionPath3D>> {
    let mut join_at = None;
    for (index, path) in paths.iter().enumerate() {
        let first = path.points.first()?;
        for (other_index, other) in paths.iter().enumerate() {
            if other_index == index {
                continue;
            }
            let last = other.points.last()?;
            if last.y == first.y && last.x < first.x {
                join_at = Some((other_index, index));
            }
        }
    }
    let (left, right) = join_at?;
    let mut mutant: Vec<ExtrusionPath3D> = Vec::new();
    for (index, path) in paths.iter().enumerate() {
        if index == left {
            let mut merged = path.clone();
            merged.points.extend(paths[right].points.iter().copied());
            mutant.push(merged);
        } else if index != right {
            mutant.push(path.clone());
        }
    }
    Some(mutant)
}

#[test]
fn ironing_controls() {
    // ── Deterministic positive witnesses ────────────────────────────────────
    let hole = hole_case();
    let hole_paths = run_kernel(&hole);
    let hole_stats = check_ironing_output(&hole, &hole_paths, true).expect("hole witness");
    assert!(hole_stats.left > 0 && hole_stats.right > 0);
    assert!(hole_stats.rows_with_gap > 0);
    assert!(
        hole_paths.len() > 1,
        "the holed witness must split into multiple paths, got {}",
        hole_paths.len()
    );
    // The formerly failing move must be absent from the real output.
    for path in &hole_paths {
        for window in path.points.windows(2) {
            assert!(
                !(window[0].x == 5.0
                    && window[1].x == 3.0
                    && window[0].y == 3.0
                    && window[1].y == 3.0),
                "the formerly failing hole-crossing join is still emitted"
            );
        }
    }

    let notch = notch_case();
    let notch_paths = run_kernel(&notch);
    let notch_stats = check_ironing_output(&notch, &notch_paths, true).expect("notch witness");
    assert!(notch_stats.left > 0 && notch_stats.right > 0);

    let rect = rect_case(-20_000, -20_000, 40_000, 40_000, 10_000);
    let rect_paths = run_kernel(&rect);
    let rect_stats = check_ironing_output(&rect, &rect_paths, true).expect("rect witness");
    assert!(rect_stats.rows_with_gap == 0);
    assert_eq!(rect_paths.len(), 1, "a convex rectangle stays one snake");

    // ── Slanted boundaries: the join rule is containment, not "same x" ──────
    // A convex slanted fill keeps a single snake: every row-to-row join lies on
    // the hypotenuse, so the rule must not split it per row.
    let triangle = ExPolygon {
        contour: ring(vec![point(0, 0), point(100_000, 0), point(0, 100_000)]),
        holes: Vec::new(),
    };
    let triangle_paths = run_poly(&triangle, 5_000, 0.2, 0.1, 0.4);
    assert_eq!(
        triangle_paths.len(),
        1,
        "a convex slanted fill must stay one snake, got {} paths",
        triangle_paths.len()
    );
    assert_paths_contained(&triangle, &triangle_paths, "triangle");

    // A concave slanted boundary whose kink sits inside one row spacing: the
    // join from the wide row to the narrow row cuts outside the material, so it
    // must start a new path.
    // Vertices are mm*10_000: (0,0) (10,0) (10,3) (7,3.2) (7,3.4) (10,4) (10,10) (0,10).
    let kinked = ExPolygon {
        contour: ring(vec![
            point(0, 0),
            point(100_000, 0),
            point(100_000, 30_000),
            point(70_000, 32_000),
            point(70_000, 34_000),
            point(100_000, 40_000),
            point(100_000, 100_000),
            point(0, 100_000),
        ]),
        holes: Vec::new(),
    };
    let kinked_paths = run_poly(&kinked, 5_000, 0.2, 0.1, 0.4);
    assert!(
        kinked_paths.len() > 1,
        "a join that leaves the material must start a new path, got {} path(s)",
        kinked_paths.len()
    );
    assert_paths_contained(&kinked, &kinked_paths, "kinked notch");

    // ── Mutations rejected by the SAME checker ─────────────────────────────
    let mutant = merged_join_mutant(&hole_paths).expect("a same-row left/right path pair");
    assert!(
        check_ironing_output(&hole, &mutant, true).is_err(),
        "a gap-connector mutation was accepted"
    );

    let dropped_row = hole_paths
        .iter()
        .map(|path| {
            let mut path = path.clone();
            path.points.retain(|point| point.y != 4.0);
            path
        })
        .filter(|path| path.points.len() >= 2)
        .collect::<Vec<_>>();
    assert!(
        !dropped_row.is_empty(),
        "the missing-row control needs at least one surviving path"
    );
    match check_ironing_output(&hole, &dropped_row, true) {
        Ok(_) => panic!("a missing-row mutation was accepted"),
        Err(message) => assert!(
            message.contains("independent interval model"),
            "the missing row was rejected for an unrelated reason: {message}"
        ),
    }

    assert!(
        check_ironing_output(&hole, &[], true).is_err(),
        "a false-empty output was accepted"
    );

    let mut wrong_flow = hole_paths.clone();
    wrong_flow[0].points[0].flow_factor = 0.5;
    assert!(
        check_ironing_output(&hole, &wrong_flow, true).is_err(),
        "a mutated flow factor was accepted"
    );

    // A single-row synthetic shortcut across the hole: endpoints and midpoint
    // are on material, but the open interval crosses the hole.
    let synthetic = ExtrusionPath3D {
        points: vec![
            iron_point(0.0, 3.0, &hole),
            iron_point(3.0, 3.0, &hole),
            iron_point(5.0, 3.0, &hole),
            iron_point(8.0, 3.0, &hole),
        ],
        // exhaustive: FRU over the shared extrusion-path base supplies role, speed, tool and lock.
        ..extrusion_path3d_base(ExtrusionRole::Ironing)
    };
    assert!(
        check_ironing_output(&hole, std::slice::from_ref(&synthetic), true).is_err(),
        "a shortcut whose endpoints/midpoint hide an outside subinterval was accepted"
    );

    // ── Legitimate empty / degenerate modes ────────────────────────────────
    let empty_poly = ExPolygon {
        contour: Polygon { points: Vec::new() },
        holes: Vec::new(),
    };
    assert!(bbox_of_expoly(&empty_poly).is_none());
    let zero_span = rect_case(0, 0, 40_000, 0, 10_000);
    assert!(run_kernel(&zero_span).is_empty());
    let mut zero_spacing = rect_case(0, 0, 40_000, 40_000, 10_000);
    zero_spacing.spacing = 0;
    assert!(run_kernel(&zero_spacing).is_empty());
    let tiny_span = rect_case(0, 0, 40_000, 2_500, 10_000);
    let tiny = run_kernel(&tiny_span);
    assert_eq!(
        tiny.len(),
        1,
        "a sub-spacing positive span has exactly one row"
    );
    assert_eq!(tiny[0].points.len(), 2);

    // ── Bound-construction rejection matrix ────────────────────────────────
    fn invalid(label: &str, bounds: BTreeMap<String, u64>) {
        match checked_limits(&bounds) {
            Err(DomainError::InvalidBounds(message)) => assert!(!message.is_empty(), "{label}"),
            other => panic!("{label}: expected InvalidBounds, got {other:?}"),
        }
    }
    let valid = probe_bound_map();
    checked_limits(&valid).expect("valid ironing bounds");
    for key in BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        invalid(&format!("missing {key}"), bounds);
    }
    for (key, value) in [
        ("max_abs_units", 0),
        ("max_abs_units", 3_000),
        ("max_spacing_units", 0),
        ("max_spacing_units", 1_000),
        ("max_rows", 0),
        ("max_rows", 65),
        ("max_vertices", 4),
        ("max_holes", 0),
        ("max_components", 0),
        ("max_shrink_candidates", 0),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        invalid(&format!("invalid {key}={value}"), bounds);
    }
    for key in ["max_abs_units", "max_spacing_units"] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), i64::MAX as u64 + 1);
        assert!(matches!(
            checked_limits(&bounds),
            Err(DomainError::ArithmeticOverflow)
        ));
    }
    let mut oversized_rows = valid;
    oversized_rows.insert("max_rows".to_owned(), u32::MAX as u64 + 1);
    assert!(matches!(
        checked_limits(&oversized_rows),
        Err(DomainError::ArithmeticOverflow)
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probe
// ─────────────────────────────────────────────────────────────────────────────

fn check_shrink_state(case: &IronCase, shape: ShapeKind) {
    assert_eq!(case.shape, shape, "the shape mode must survive shrinking");
    assert_eq!(case.width % GRID_UNITS, 0);
    assert_eq!(case.height % GRID_UNITS, 0);
    assert_eq!(case.spacing % GRID_UNITS, 0);
    let (_, y0, _, y1) = case.rect_mm();
    assert!(y1 > y0, "a shrink candidate collapsed the vertical span");
    let span = y1 - y0;
    let n_rows = (span / case.spacing_mm()).floor() as usize + 1;
    assert!(n_rows <= 64, "shrink candidate exceeded the row envelope");
    match case.shape {
        ShapeKind::Rect => assert!(case.cut.is_none()),
        ShapeKind::Notch | ShapeKind::Hole => {
            let (cx0, cx1, cy0, cy1) = case.cut.expect("cut present");
            assert!(
                cx0 > case.x0 && cx1 < case.x1(),
                "cut must stay strictly inside in x"
            );
            assert!(
                cy0 >= case.y0 && cy1 <= case.y1(),
                "cut must stay inside in y"
            );
        }
    }
    let paths = run_kernel(case);
    check_ironing_output(case, &paths, true).expect("shrunk fixture still checks");
}

#[test]
fn ironing_shrink_bounds() {
    let limits = checked_limits(&probe_bound_map()).expect("shrink bounds");
    for shape in SHAPE_MODES {
        let strategy = iron_case_strategy(&limits, shape);
        let mut runner = TestRunner::deterministic();
        let mut tree = strategy.new_tree(&mut runner).expect("ironing shrink tree");
        let initial = tree.current();
        check_shrink_state(&initial, shape);
        let initial_bits = case_bits(&initial);

        let mut simplified = 0_usize;
        let mut changed = false;
        while tree.simplify() {
            simplified += 1;
            let current = tree.current();
            check_shrink_state(&current, shape);
            changed |= case_bits(&current) != initial_bits;
            assert!(
                tree.complicate(),
                "complicate must restore the last simplification"
            );
            check_shrink_state(&tree.current(), shape);
        }
        assert!(simplified > 0, "false-only shrink probe for {shape:?}");
        assert!(
            changed,
            "no allowed reduction changed a {shape:?} fixture from its nonminimal start"
        );

        // A false simplify must leave the current value untouched, and repeated
        // complicate calls must converge to a stable false-returning state.
        let at_minimum = format!("{:?}", tree.current());
        assert!(
            !tree.simplify(),
            "an exhausted {shape:?} tree simplified again"
        );
        assert_eq!(format!("{:?}", tree.current()), at_minimum);
        let mut restores = 0_usize;
        while tree.complicate() {
            restores += 1;
            check_shrink_state(&tree.current(), shape);
        }
        // Repeated complicate calls past exhaustion must keep returning false
        // and must not move the value.
        let settled = format!("{:?}", tree.current());
        assert!(
            !tree.complicate(),
            "complicate after exhaustion must be false"
        );
        assert_eq!(format!("{:?}", tree.current()), settled);
        println!("shape={shape:?} simplify={simplified} restores={restores} changed={changed}");
    }

    // The mode selector itself must be non-shrinkable: a shrinking mode tree
    // could switch a witness's class mid-search.
    let mut runner = TestRunner::deterministic();
    let modes = prop::sample::select(SHAPE_MODES.to_vec()).no_shrink();
    let mut tree = modes.new_tree(&mut runner).expect("mode tree");
    let before = format!("{:?}", tree.current());
    assert!(!tree.simplify(), "the shape mode selector simplified");
    assert!(!tree.complicate(), "the shape mode selector complicated");
    assert_eq!(format!("{:?}", tree.current()), before);
}
