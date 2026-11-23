//! Opt-in property tests for this module's private rectilinear kernels.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_13_pbt-infill`):
//!
//! * `M-RECTILINEAR-SCAN` — complete axis-aligned scan-interval coverage over
//!   rectangles, L/U notches and rectangular holes, checked segment by segment.
//! * `M-RECTILINEAR-SPACING` — the canonical `Fill::_adjust_solid_spacing`
//!   interval/cap model, including the 1.5 mm / 1.0 mm → 1.2 mm over-cap case.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `scan_expolygon` and `adjust_solid_spacing` are private, so the properties
//! live here as a crate-internal unit-test module. Production visibility is
//! deliberately unchanged.
//!
//! # Independent references (no SUT calls)
//!
//! * `canonical_adjusted_spacing` is the `Fill::_adjust_solid_spacing`
//!   (`src/libslic3r/Fill/FillBase.cpp`) interval/cap rule expressed on the
//!   100 nm grid. `EPSILON = 1e-4` mm is observable only as an exact-multiple
//!   tie-break on this grid: for any `0 < ε_units < 1`, integer truncation of
//!   `(width - ε) / d` and `(width - ε) / count` equals truncation of
//!   `(width - 1) / d` and `(width - 1) / count` for every integer pair
//!   (exact multiples drop by one interval; every other remainder is
//!   unchanged). `5 * new > 6 * distance` is the exact integer form of the
//!   source's `factor > 1.2` comparison; `(12 * distance + 5) / 10` is the
//!   exact integer form of `floor(distance * 1.2 + 0.5)`.
//! * `expected_row_intervals` derives each row's material intervals from the
//!   fixture's own vertex X coordinates and #2's `point_in_region` applied to a
//!   doubled-coordinate probe at `y + 1/2`. That probe is never on a horizontal
//!   or vertical edge, so it classifies each open cell exactly. The SUT's
//!   scanner, rotation helper and spacing adjustment are never consulted.
//! * Rows are the charter's half-open grid (`min_y + k * spacing < max_y`).
//!   No assertion claims that the last row touches the far boundary.
//!
//! # Literal review (AC-6)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. Every watched-type literal is
//! reviewed by hand here and in `docs/testing/proptest-infill-evidence.md`.
//!
//! Watched literal sites:
//!
//! * `ScanLimits::geometry_bounds` — `GeometryBounds` (`crates/slicer-sdk/src/
//!   test_support/property_testing/domains/mod.rs`), all five fields explicit
//!   with an `// exhaustive:` waiver.
//! * `geometry_domain` — `GeometryDomain`, all five fields explicit with an
//!   `// exhaustive:` waiver.
//! * `mutated_path` — `ExtrusionPath3D` uses FRU over a cloned real path.
//!   `Point3WithWidth` values are copied from real output and mutated
//!   field-by-field; there is no point literal.
//! * `ScanParams`, `ScanCase`, `SpacingParams`, `SpacingPair`, `ScanLimits`,
//!   `SpacingLimits`, `RowInterval` and `ScanStats` are private module-local
//!   types, not watched crate-public structs.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::BTreeMap;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, Strategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    mm_to_units, units_to_mm, ExPolygon, ExtrusionPath3D, ExtrusionRole, Point2, Point3WithWidth,
    Polygon,
};
use slicer_sdk::test_support::property_testing::domains::{
    BoundaryRule, DomainError, GeometryBounds, GeometryCase, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, segment_in_region, validate_geometry, PointLocation,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

const SCAN_ID: &str = "M-RECTILINEAR-SCAN";
const SCAN_TEST: &str = "property_tests::proptest_rectilinear_scan_intervals";
const SPACING_ID: &str = "M-RECTILINEAR-SPACING";
const SPACING_TEST: &str = "property_tests::proptest_rectilinear_solid_spacing";

/// Complete, required profile bound keys (missing or zero is an error).
const SCAN_BOUND_KEYS: [&str; 7] = [
    "max_abs_units",
    "max_vertices",
    "max_holes",
    "max_components",
    "max_shrink_candidates",
    "max_spacing_units",
    "max_rows",
];
const SPACING_BOUND_KEYS: [&str; 2] = ["max_width_units", "max_spacing_units"];

/// Smallest generated fixture span. Four units is the smallest width that
/// admits the one-unit U-notch and hole insets below.
const MIN_SPAN_UNITS: i64 = 4;
/// Largest scan profile bound accepted here. `validate_geometry` also caps
/// `max_abs_units` at 1,000,000; the other two caps keep every checked
/// multiplication below four million.
const MAX_ABS_UNITS: i64 = 1_000_000;
const MAX_ROWS_BUDGET: usize = 1_000_000;
/// Largest spacing profile bound accepted here; 100 mm at 100 nm per unit.
const MAX_SPACING_UNITS: i64 = 1_000_000;
/// The spacing property needs an over-cap band `(6d/5, 2d]` with `d >= 5`.
const MIN_CAP_DISTANCE_UNITS: i64 = 5;
const MIN_WIDTH_UNITS: i64 = 10;
const SHRINK_STEPS: usize = 64;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

fn required_value(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

fn fail_scan_bound(message: impl Into<String>) -> DomainError {
    DomainError::InvalidBounds(message.into())
}

// ---------------------------------------------------------------------------
// Checked profile bounds
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct ScanLimits {
    max_abs_units: i64,
    max_vertices: usize,
    max_holes: usize,
    max_components: usize,
    max_shrink_candidates: u32,
    max_spacing_units: i64,
    max_rows: usize,
}

impl ScanLimits {
    fn span_min(&self) -> i64 {
        (self.max_abs_units / 200).max(MIN_SPAN_UNITS)
    }

    fn span_max(&self) -> i64 {
        (self.max_abs_units / 2).max(self.span_min())
    }

    fn geometry_bounds(&self) -> GeometryBounds {
        // exhaustive: every field is derived from independently checked profile
        // limits; a newly added field must be reviewed here because
        // `check-literals` does not scan module sources.
        GeometryBounds {
            max_abs_units: self.max_abs_units,
            max_vertices: self.max_vertices,
            max_holes: self.max_holes,
            max_components: self.max_components,
            max_shrink_candidates: self.max_shrink_candidates,
        }
    }
}

fn checked_scan_limits(bounds: &BTreeMap<String, u64>) -> Result<ScanLimits, DomainError> {
    for key in SCAN_BOUND_KEYS {
        required_value(bounds, key)?;
    }
    let max_abs_units = i64::try_from(required_value(bounds, "max_abs_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_vertices = usize::try_from(required_value(bounds, "max_vertices")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_holes = usize::try_from(required_value(bounds, "max_holes")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_components = usize::try_from(required_value(bounds, "max_components")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_shrink_candidates = u32::try_from(required_value(bounds, "max_shrink_candidates")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_spacing_units = i64::try_from(required_value(bounds, "max_spacing_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_rows = usize::try_from(required_value(bounds, "max_rows")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;

    if u32::try_from(max_vertices).is_err()
        || u32::try_from(max_holes).is_err()
        || u32::try_from(max_components).is_err()
    {
        return Err(DomainError::ArithmeticOverflow);
    }
    if !(MIN_SPAN_UNITS..=MAX_ABS_UNITS).contains(&max_abs_units) {
        return Err(fail_scan_bound(format!(
            "max_abs_units must be in {MIN_SPAN_UNITS}..={MAX_ABS_UNITS}"
        )));
    }
    if max_vertices < 8 {
        return Err(fail_scan_bound(
            "max_vertices must admit the eight-vertex U-notch fixture",
        ));
    }
    if max_holes < 1 {
        return Err(fail_scan_bound(
            "max_holes must admit the rectangular-hole fixture",
        ));
    }
    if max_components < 1 {
        return Err(fail_scan_bound("max_components must be positive"));
    }
    if max_shrink_candidates < 1 {
        return Err(fail_scan_bound("max_shrink_candidates must be positive"));
    }
    if !(2..=MAX_SPACING_UNITS).contains(&max_spacing_units) {
        return Err(fail_scan_bound(format!(
            "max_spacing_units must be in 2..={MAX_SPACING_UNITS}"
        )));
    }
    if max_rows < 1 {
        return Err(fail_scan_bound("max_rows must be positive"));
    }
    // Checked coverage proof: the largest generated height must fit the largest
    // spacing inside the row budget. The multiplication is also the portable
    // `usize` overflow arm (a `u64::MAX` row bound overflows here).
    let span_max = (max_abs_units / 2).max((max_abs_units / 200).max(MIN_SPAN_UNITS));
    let span_max = usize::try_from(span_max).map_err(|_| DomainError::ArithmeticOverflow)?;
    let spacing =
        usize::try_from(max_spacing_units).map_err(|_| DomainError::ArithmeticOverflow)?;
    match spacing.checked_mul(max_rows) {
        None => return Err(DomainError::ArithmeticOverflow),
        Some(covered) if covered < span_max => {
            return Err(fail_scan_bound(
                "max_spacing_units * max_rows cannot cover the largest generated span",
            ))
        }
        Some(_) => {}
    }
    if max_rows > MAX_ROWS_BUDGET {
        return Err(fail_scan_bound(format!(
            "max_rows must not exceed {MAX_ROWS_BUDGET}"
        )));
    }
    Ok(ScanLimits {
        max_abs_units,
        max_vertices,
        max_holes,
        max_components,
        max_shrink_candidates,
        max_spacing_units,
        max_rows,
    })
}

#[derive(Debug, Clone, Copy)]
struct SpacingLimits {
    max_width_units: i64,
    max_spacing_units: i64,
}

fn checked_spacing_limits(bounds: &BTreeMap<String, u64>) -> Result<SpacingLimits, DomainError> {
    for key in SPACING_BOUND_KEYS {
        required_value(bounds, key)?;
    }
    let max_width_units = i64::try_from(required_value(bounds, "max_width_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    let max_spacing_units = i64::try_from(required_value(bounds, "max_spacing_units")?)
        .map_err(|_| DomainError::ArithmeticOverflow)?;
    if !(MIN_WIDTH_UNITS..=MAX_ABS_UNITS).contains(&max_width_units) {
        return Err(fail_scan_bound(format!(
            "max_width_units must be in {MIN_WIDTH_UNITS}..={MAX_ABS_UNITS}"
        )));
    }
    if !(MIN_CAP_DISTANCE_UNITS..=MAX_SPACING_UNITS).contains(&max_spacing_units) {
        return Err(fail_scan_bound(format!(
            "max_spacing_units must be in {MIN_CAP_DISTANCE_UNITS}..={MAX_SPACING_UNITS}"
        )));
    }
    Ok(SpacingLimits {
        max_width_units,
        max_spacing_units,
    })
}

fn probe_scan_bound_map() -> BTreeMap<String, u64> {
    // 20 mm of XY room, 2 mm maximum spacing and 64 rows admit every fixture
    // class while keeping the shrink probe bounded.
    BTreeMap::from([
        ("max_abs_units".to_owned(), 200_000),
        ("max_vertices".to_owned(), 8),
        ("max_holes".to_owned(), 1),
        ("max_components".to_owned(), 1),
        ("max_shrink_candidates".to_owned(), 16),
        ("max_spacing_units".to_owned(), 20_000),
        ("max_rows".to_owned(), 64),
    ])
}

fn probe_spacing_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_width_units".to_owned(), 200_000),
        ("max_spacing_units".to_owned(), 20_000),
    ])
}

fn live_scan_limits(run: &PropertyRun) -> ScanLimits {
    let mut bounds = BTreeMap::new();
    for key in SCAN_BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("scan bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_scan_limits(&bounds).expect("scan bound consistency")
}

fn live_spacing_limits(run: &PropertyRun) -> SpacingLimits {
    let mut bounds = BTreeMap::new();
    for key in SPACING_BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("spacing bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_spacing_limits(&bounds).expect("spacing bound consistency")
}

// ---------------------------------------------------------------------------
// Canonical `Fill::_adjust_solid_spacing` reference
// ---------------------------------------------------------------------------

/// Canonical `Fill::_adjust_solid_spacing` on the 100 nm grid.
///
/// `width - 1` represents `width - EPSILON` for any `0 < EPSILON < 1` unit
/// (see the module header derivation). The interval count is the source's
/// truncated first division; the over-cap branch is the source's
/// `floor(distance * 1.2 + 0.5)` in exact integer form.
fn canonical_adjusted_spacing(width: i64, distance: i64) -> Option<i64> {
    canonical_spacing_with_branch(width, distance).map(|(value, _)| value)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpacingBranch {
    /// `(width - EPSILON) / distance` truncated to zero intervals.
    BelowDistance,
    /// The `(width - EPSILON) / count` interval result.
    Interval,
    /// `factor > 1.2`, capped at `floor(distance * 1.2 + 0.5)`.
    OverCap,
}

fn canonical_spacing_with_branch(width: i64, distance: i64) -> Option<(i64, SpacingBranch)> {
    if distance <= 0 || width < 0 {
        return None;
    }
    let adjusted_width = width.checked_sub(1)?;
    let number_of_intervals = adjusted_width / distance;
    if number_of_intervals == 0 {
        return Some((distance, SpacingBranch::BelowDistance));
    }
    let distance_new = adjusted_width / number_of_intervals;
    if distance_new.checked_mul(5)? > distance.checked_mul(6)? {
        let cap = distance.checked_mul(12)?.checked_add(5)? / 10;
        return Some((cap, SpacingBranch::OverCap));
    }
    Some((distance_new, SpacingBranch::Interval))
}

fn check_spacing_with<F>(width: i64, distance: i64, produce: F) -> Result<i64, String>
where
    F: Fn(i64, i64) -> i64,
{
    let (expected, _) = canonical_spacing_with_branch(width, distance)
        .ok_or_else(|| format!("spacing reference rejected width={width} distance={distance}"))?;
    let actual = produce(width, distance);
    if actual != expected {
        return Err(format!(
            "width={width} distance={distance}: produced {actual}, canonical \
             Fill::_adjust_solid_spacing gives {expected}"
        ));
    }
    Ok(actual)
}

#[derive(Debug, Clone, Copy)]
struct SpacingPair {
    label: &'static str,
    width: i64,
    distance: i64,
}

#[derive(Debug, Clone)]
struct SpacingParams {
    below: SpacingPair,
    exact: SpacingPair,
    general: SpacingPair,
    over_cap: SpacingPair,
}

impl SpacingParams {
    fn pairs(&self) -> [SpacingPair; 4] {
        [self.below, self.exact, self.general, self.over_cap]
    }
}

fn check_spacing_pair(
    pair: &SpacingPair,
    limits: &SpacingLimits,
) -> Result<(i64, SpacingBranch), String> {
    if pair.distance <= 0 {
        return Err(format!("{}: distance must be positive", pair.label));
    }
    if pair.width < 0 || pair.width > limits.max_width_units {
        return Err(format!(
            "{}: width {} is outside 0..={}",
            pair.label, pair.width, limits.max_width_units
        ));
    }
    if pair.distance > limits.max_spacing_units {
        return Err(format!(
            "{}: distance {} exceeds the checked profile bound {}",
            pair.label, pair.distance, limits.max_spacing_units
        ));
    }
    let (expected, branch) = canonical_spacing_with_branch(pair.width, pair.distance)
        .ok_or_else(|| format!("{}: spacing reference arithmetic", pair.label))?;
    let actual = super::adjust_solid_spacing(pair.width, pair.distance);
    if actual != expected {
        return Err(format!(
            "{}: width={} distance={} produced {} but canonical \
             Fill::_adjust_solid_spacing gives {} ({branch:?})",
            pair.label, pair.width, pair.distance, actual, expected
        ));
    }
    Ok((actual, branch))
}

fn spacing_params_strategy(limits: &SpacingLimits) -> BoxedStrategy<SpacingParams> {
    let max_width = limits.max_width_units;
    let max_spacing = limits.max_spacing_units;
    let exact_distance_max = max_spacing.min(max_width);
    let cap_distance_max = max_spacing.min(max_width / 2);
    assert!(
        cap_distance_max >= MIN_CAP_DISTANCE_UNITS,
        "checked spacing bounds admit the over-cap band"
    );
    (
        1_i64..=max_spacing,
        any::<u64>(),
        1_i64..=exact_distance_max,
        any::<u64>(),
        0_i64..=max_width,
        1_i64..=max_spacing,
        MIN_CAP_DISTANCE_UNITS..=cap_distance_max,
        any::<u32>(),
    )
        .prop_map(
            move |(
                below_distance,
                below_selector,
                exact_distance,
                k_selector,
                general_width,
                general_distance,
                cap_distance,
                cap_selector,
            )| {
                let below_width = i64::try_from(
                    below_selector % u64::try_from(below_distance).expect("positive distance"),
                )
                .expect("below-distance width fits i64");
                let max_k = max_width / exact_distance;
                let k = 1 + i64::try_from(
                    k_selector % u64::try_from(max_k).expect("exact distance admits an interval"),
                )
                .expect("exact k fits i64");
                let exact_width = k * exact_distance;
                let cap_low = (6 * cap_distance) / 5 + 1;
                let cap_high = 2 * cap_distance - 1;
                let cap_span = u64::try_from(cap_high - cap_low + 1).expect("positive cap band");
                let cap_width =
                    cap_low + i64::try_from(u64::from(cap_selector) % cap_span).expect("cap width");
                SpacingParams {
                    below: SpacingPair {
                        label: "width-below-distance",
                        width: below_width,
                        distance: below_distance,
                    },
                    exact: SpacingPair {
                        label: "exact-interval-boundary",
                        width: exact_width,
                        distance: exact_distance,
                    },
                    general: SpacingPair {
                        label: "interval-truncation",
                        width: general_width,
                        distance: general_distance,
                    },
                    over_cap: SpacingPair {
                        label: "over-cap",
                        width: cap_width,
                        distance: cap_distance,
                    },
                }
            },
        )
        .boxed()
}

// ---------------------------------------------------------------------------
// Scan fixtures and the independent row-interval model
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeClass {
    Rectangle,
    SubSpacingRectangle,
    LNotch,
    UNotch,
    HoledRectangle,
}

impl ShapeClass {
    const ALL: [Self; 5] = [
        Self::Rectangle,
        Self::SubSpacingRectangle,
        Self::LNotch,
        Self::UNotch,
        Self::HoledRectangle,
    ];

    fn class(self) -> TopologyClass {
        match self {
            Self::Rectangle | Self::SubSpacingRectangle => TopologyClass::Convex,
            Self::LNotch | Self::UNotch => TopologyClass::Concave,
            Self::HoledRectangle => TopologyClass::Holed,
        }
    }
}

#[derive(Debug, Clone)]
struct ScanParams {
    origin_x: i64,
    origin_y: i64,
    width: i64,
    height: i64,
    spacing: i64,
    shift: i64,
    z_centi: u32,
    speed_per_mille: u32,
    flow_per_mille: u32,
    line_width_milli: u32,
}

impl ScanParams {
    fn z(&self) -> f32 {
        self.z_centi as f32 / 100.0
    }

    fn speed_factor(&self) -> f32 {
        self.speed_per_mille as f32 / 1000.0
    }

    fn flow_factor(&self) -> f32 {
        self.flow_per_mille as f32 / 1000.0
    }

    fn line_width(&self) -> f32 {
        self.line_width_milli as f32 / 1000.0
    }
}

#[derive(Debug, Clone)]
struct ScanCase {
    label: String,
    shape: ShapeClass,
    expoly: ExPolygon,
    domain: GeometryDomain,
    spacing: i64,
    adjust_for_solid: bool,
    x_shift: i64,
    z: f32,
    speed_factor: f32,
    flow_factor: f32,
    line_width: f32,
    role: ExtrusionRole,
    max_rows: usize,
}

fn point(x: i64, y: i64) -> Point2 {
    Point2 { x, y }
}

fn ring(points: Vec<Point2>) -> Polygon {
    Polygon { points }
}

fn rect_expoly(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
    ExPolygon {
        contour: ring(vec![
            point(x0, y0),
            point(x1, y0),
            point(x1, y1),
            point(x0, y1),
        ]),
        holes: Vec::new(),
    }
}

fn shape_expoly(shape: ShapeClass, params: &ScanParams) -> ExPolygon {
    let x0 = params.origin_x;
    let y0 = params.origin_y;
    let x1 = x0 + params.width;
    let y1 = y0 + params.height;
    match shape {
        ShapeClass::Rectangle => rect_expoly(x0, y0, x1, y1),
        ShapeClass::SubSpacingRectangle => {
            // Positive height strictly below the passed spacing: exactly one
            // half-open row at the minimum boundary.
            let sub_height = (params.spacing / 2).max(1);
            rect_expoly(x0, y0, x1, y0 + sub_height)
        }
        ShapeClass::LNotch => {
            let notch_width = (params.width / 4).max(1);
            let notch_height = (params.height / 4).max(1);
            ExPolygon {
                contour: ring(vec![
                    point(x0, y0),
                    point(x1, y0),
                    point(x1, y1 - notch_height),
                    point(x1 - notch_width, y1 - notch_height),
                    point(x1 - notch_width, y1),
                    point(x0, y1),
                ]),
                holes: Vec::new(),
            }
        }
        ShapeClass::UNotch => {
            let notch_width = (params.width / 4).max(1);
            let notch_height = (params.height / 4).max(1);
            let left = x0 + params.width / 2 - notch_width / 2;
            let right = left + notch_width;
            ExPolygon {
                contour: ring(vec![
                    point(x0, y0),
                    point(x1, y0),
                    point(x1, y1),
                    point(right, y1),
                    point(right, y1 - notch_height),
                    point(left, y1 - notch_height),
                    point(left, y1),
                    point(x0, y1),
                ]),
                holes: Vec::new(),
            }
        }
        ShapeClass::HoledRectangle => {
            let inset = (params.width.min(params.height) / 8).max(1);
            let hole_width = (params.width / 4).max(1);
            let hole_height = (params.height / 4).max(1);
            let hx0 = x0 + inset;
            let hy0 = y0 + inset;
            let hx1 = hx0 + hole_width;
            let hy1 = hy0 + hole_height;
            ExPolygon {
                contour: ring(vec![
                    point(x0, y0),
                    point(x1, y0),
                    point(x1, y1),
                    point(x0, y1),
                ]),
                // Clockwise hole ring (negative signed area).
                holes: vec![ring(vec![
                    point(hx0, hy0),
                    point(hx0, hy1),
                    point(hx1, hy1),
                    point(hx1, hy0),
                ])],
            }
        }
    }
}

fn geometry_domain(class: TopologyClass, limits: &ScanLimits) -> GeometryDomain {
    let (min_holes, min_components) = match class {
        TopologyClass::Holed => (1, 1),
        _ => (0, 1),
    };
    // exhaustive: class, minima and bounds are selected per fixture class; a
    // newly added field must be reviewed here because `check-literals` does not
    // scan module sources.
    GeometryDomain {
        class,
        min_holes,
        min_components,
        boundary: BoundaryRule::Strict,
        bounds: limits.geometry_bounds(),
    }
}

fn spacing_floor(height: i64, max_rows: usize) -> i64 {
    let rows = i64::try_from(max_rows).expect("checked max_rows");
    ((height + rows - 1) / rows).max(2)
}

fn scan_params_strategy(limits: &ScanLimits) -> BoxedStrategy<ScanParams> {
    let limits = limits.clone();
    let span_min = limits.span_min();
    let span_max = limits.span_max();
    let max_abs = limits.max_abs_units;
    (
        -max_abs..=-1_i64,
        -max_abs..=-1_i64,
        span_min..=span_max,
        span_min..=span_max,
    )
        .prop_ind_flat_map(move |(origin_x, origin_y, width, height)| {
            let floor = spacing_floor(height, limits.max_rows);
            assert!(
                floor <= limits.max_spacing_units,
                "checked scan bounds cover every generated height"
            );
            let shift_max = (limits.max_abs_units / 10).max(1);
            (
                floor..=limits.max_spacing_units,
                1_i64..=shift_max,
                any::<bool>(),
                0_u32..=1000,
                10_u32..=1000,
                10_u32..=200,
                50_u32..=2000,
            )
                .prop_map(
                    move |(spacing, shift_magnitude, negative, z, speed, flow, line_width)| {
                        ScanParams {
                            origin_x,
                            origin_y,
                            width,
                            height,
                            spacing,
                            shift: if negative {
                                -shift_magnitude
                            } else {
                                shift_magnitude
                            },
                            z_centi: z,
                            speed_per_mille: speed,
                            flow_per_mille: flow,
                            line_width_milli: line_width,
                        }
                    },
                )
        })
        .boxed()
}

fn build_suite(params: &ScanParams, limits: &ScanLimits) -> Vec<ScanCase> {
    let mut cases = Vec::with_capacity(ShapeClass::ALL.len() * 4);
    for shape in ShapeClass::ALL {
        let expoly = shape_expoly(shape, params);
        let domain = geometry_domain(shape.class(), limits);
        for adjust_for_solid in [false, true] {
            let role = if adjust_for_solid {
                ExtrusionRole::TopSolidInfill
            } else {
                ExtrusionRole::SparseInfill
            };
            let role_name = if adjust_for_solid { "solid" } else { "sparse" };
            for shift in [0, params.shift] {
                let shift_name = if shift == 0 { "unshifted" } else { "shifted" };
                cases.push(ScanCase {
                    label: format!("{shape:?}-{role_name}-{shift_name}"),
                    shape,
                    expoly: expoly.clone(),
                    domain: domain.clone(),
                    spacing: params.spacing,
                    adjust_for_solid,
                    x_shift: shift,
                    z: params.z(),
                    speed_factor: params.speed_factor(),
                    flow_factor: params.flow_factor(),
                    line_width: params.line_width(),
                    role: role.clone(),
                    max_rows: limits.max_rows,
                });
            }
        }
    }
    cases
}

fn run_scan(case: &ScanCase) -> Vec<ExtrusionPath3D> {
    super::scan_expolygon(
        &case.expoly,
        case.spacing,
        1.0,
        0.0,
        case.z,
        case.speed_factor,
        case.flow_factor,
        &case.role,
        case.line_width,
        case.adjust_for_solid,
        case.x_shift,
    )
}

fn bbox(expoly: &ExPolygon) -> (i64, i64, i64, i64) {
    let (mut min_x, mut min_y) = (i64::MAX, i64::MAX);
    let (mut max_x, mut max_y) = (i64::MIN, i64::MIN);
    for ring in std::iter::once(&expoly.contour).chain(expoly.holes.iter()) {
        for p in &ring.points {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }
    }
    (min_x, min_y, max_x, max_y)
}

fn doubled_ring(ring: &Polygon) -> Polygon {
    Polygon {
        points: ring
            .points
            .iter()
            .map(|p| point(p.x * 2, p.y * 2))
            .collect(),
    }
}

fn doubled_expoly(expoly: &ExPolygon) -> ExPolygon {
    ExPolygon {
        contour: doubled_ring(&expoly.contour),
        holes: expoly.holes.iter().map(doubled_ring).collect(),
    }
}

fn translated_expoly(expoly: &ExPolygon, dx: i64) -> ExPolygon {
    let translate = |ring: &Polygon| Polygon {
        points: ring.points.iter().map(|p| point(p.x + dx, p.y)).collect(),
    };
    ExPolygon {
        contour: translate(&expoly.contour),
        holes: expoly.holes.iter().map(translate).collect(),
    }
}

fn vertex_xs(expoly: &ExPolygon) -> Vec<i64> {
    let mut xs = Vec::new();
    for ring in std::iter::once(&expoly.contour).chain(expoly.holes.iter()) {
        xs.extend(ring.points.iter().map(|p| p.x));
    }
    xs.sort_unstable();
    xs.dedup();
    xs
}

fn expected_row_intervals(
    doubled: &ExPolygon,
    xs: &[i64],
    y: i64,
) -> Result<Vec<(i64, i64)>, DomainError> {
    // Probe the open cell at `(mid_x, y + 1/2)` in doubled coordinates. Every
    // ring edge is on an even coordinate, so the probe cannot lie on an edge
    // and `point_in_region` returns Inside or Outside, never Boundary.
    let probe_y = y * 2 + 1;
    let mut intervals: Vec<(i64, i64)> = Vec::new();
    let mut open: Option<(i64, i64)> = None;
    for window in xs.windows(2) {
        let (left, right) = (window[0], window[1]);
        let probe = point(left + right, probe_y);
        let inside =
            point_in_region(probe, std::slice::from_ref(doubled))? == PointLocation::Inside;
        if inside {
            match &mut open {
                Some((_, end)) => *end = right,
                None => open = Some((left, right)),
            }
        } else if let Some(interval) = open.take() {
            intervals.push(interval);
        }
    }
    if let Some(interval) = open {
        intervals.push(interval);
    }
    Ok(intervals)
}

fn enumerate_rows(
    min_y: i64,
    max_y: i64,
    spacing: i64,
    max_rows: usize,
) -> Result<Vec<i64>, String> {
    if spacing <= 0 {
        return Err(format!("row spacing {spacing} must be positive"));
    }
    let expected_rows =
        (i128::from(max_y) - i128::from(min_y) + i128::from(spacing) - 1) / i128::from(spacing);
    if expected_rows > i128::try_from(max_rows).expect("checked row budget") {
        return Err(format!(
            "row budget {max_rows} cannot hold {expected_rows} rows"
        ));
    }
    let mut rows = Vec::with_capacity(usize::try_from(expected_rows).unwrap_or(0));
    let mut y = min_y;
    while y < max_y {
        rows.push(y);
        y = y.checked_add(spacing).ok_or("row enumeration overflow")?;
    }
    if i128::try_from(rows.len()).expect("bounded rows") != expected_rows {
        return Err(format!(
            "half-open enumeration found {} rows, arithmetic says {expected_rows}",
            rows.len()
        ));
    }
    Ok(rows)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct RowInterval {
    y: i64,
    x0: i64,
    x1: i64,
}

#[derive(Debug, Clone, Copy, Default)]
struct ScanStats {
    rows: usize,
    segments: usize,
}

fn endpoint_units(point: &Point3WithWidth) -> Result<(i64, i64), String> {
    let x = mm_to_units(point.x);
    let y = mm_to_units(point.y);
    if units_to_mm(x).to_bits() != point.x.to_bits()
        || units_to_mm(y).to_bits() != point.y.to_bits()
    {
        return Err(format!(
            "endpoint ({}, {}) is not an exact canonical-grid point",
            point.x, point.y
        ));
    }
    Ok((x, y))
}

fn multiset_difference(expected: &[RowInterval], actual: &[RowInterval]) -> String {
    for (index, (want, got)) in expected.iter().zip(actual).enumerate() {
        if want != got {
            return format!(
                "segment multiset differs at index {index}: expected {want:?}, emitted {got:?}"
            );
        }
    }
    format!(
        "segment multiset length differs: expected {} {:?}, emitted {} {:?}",
        expected.len(),
        expected,
        actual.len(),
        actual
    )
}

fn check_scan(case: &ScanCase, paths: &[ExtrusionPath3D]) -> Result<ScanStats, String> {
    let (_, min_y, _, max_y) = bbox(&case.expoly);
    let height = max_y - min_y;
    let spacing = if case.adjust_for_solid {
        canonical_adjusted_spacing(height, case.spacing).ok_or_else(|| {
            format!(
                "solid spacing reference arithmetic for height={height} spacing={}",
                case.spacing
            )
        })?
    } else {
        case.spacing
    };
    let rows = enumerate_rows(min_y, max_y, spacing, case.max_rows)?;
    let doubled = doubled_expoly(&case.expoly);
    let xs = vertex_xs(&case.expoly);
    let mut expected: Vec<RowInterval> = Vec::new();
    for &y in &rows {
        for (x0, x1) in expected_row_intervals(&doubled, &xs, y)
            .map_err(|error| format!("row {y} interval model: {error}"))?
        {
            if x0 >= x1 {
                return Err(format!(
                    "row {y} produced nonpositive interval [{x0}, {x1}]"
                ));
            }
            let x0 = x0
                .checked_add(case.x_shift)
                .ok_or("expected interval shift overflow")?;
            let x1 = x1
                .checked_add(case.x_shift)
                .ok_or("expected interval shift overflow")?;
            expected.push(RowInterval { y, x0, x1 });
        }
    }
    expected.sort_unstable();

    // Shifted output is checked against the correspondingly translated region,
    // never against the unshifted polygon.
    let region = translated_expoly(&case.expoly, case.x_shift);
    let mut actual: Vec<RowInterval> = Vec::with_capacity(paths.len());
    for (index, path) in paths.iter().enumerate() {
        if path.points.len() != 2 {
            return Err(format!(
                "path {index} has {} points, expected one two-point segment",
                path.points.len()
            ));
        }
        if path.role != case.role {
            return Err(format!(
                "path {index} role {:?}, expected {:?}",
                path.role, case.role
            ));
        }
        if path.speed_factor.to_bits() != case.speed_factor.to_bits() {
            return Err(format!(
                "path {index} speed_factor {}, expected {}",
                path.speed_factor, case.speed_factor
            ));
        }
        if path.tool_index.is_some() || path.order_lock.is_some() {
            return Err(format!(
                "path {index} must carry None tool and lock, got tool={:?} lock={:?}",
                path.tool_index, path.order_lock
            ));
        }
        let start = &path.points[0];
        let end = &path.points[1];
        for endpoint in [start, end] {
            if endpoint.z.to_bits() != case.z.to_bits() {
                return Err(format!(
                    "path {index} Z {}, expected {}",
                    endpoint.z, case.z
                ));
            }
            if endpoint.width.to_bits() != case.line_width.to_bits() {
                return Err(format!(
                    "path {index} width {}, expected {}",
                    endpoint.width, case.line_width
                ));
            }
            if endpoint.flow_factor.to_bits() != case.flow_factor.to_bits() {
                return Err(format!(
                    "path {index} flow {}, expected {}",
                    endpoint.flow_factor, case.flow_factor
                ));
            }
        }
        let (x_start, y_start) = endpoint_units(start)?;
        let (x_end, y_end) = endpoint_units(end)?;
        if y_start != y_end {
            return Err(format!(
                "path {index} spans rows {y_start}..{y_end}; model rows are horizontal"
            ));
        }
        if x_start >= x_end {
            return Err(format!(
                "path {index} is not emitted left-to-right: {x_start}..{x_end}"
            ));
        }
        let segment = [point(x_start, y_start), point(x_end, y_end)];
        let contained = segment_in_region(
            segment,
            std::slice::from_ref(&region),
            BoundaryRule::Inclusive,
        )
        .map_err(|error| format!("path {index} segment oracle: {error}"))?;
        if !contained {
            return Err(format!(
                "path {index} segment ({x_start},{y_start})..({x_end},{y_end}) leaves the region"
            ));
        }
        actual.push(RowInterval {
            y: y_start,
            x0: x_start,
            x1: x_end,
        });
    }
    actual.sort_unstable();
    if actual != expected {
        return Err(multiset_difference(&expected, &actual));
    }
    Ok(ScanStats {
        rows: rows.len(),
        segments: actual.len(),
    })
}

// ---------------------------------------------------------------------------
// Per-property nonempty counters
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
struct ScanCounters {
    rectangle_segments: Cell<u64>,
    sub_spacing_segments: Cell<u64>,
    notch_segments: Cell<u64>,
    hole_segments: Cell<u64>,
    shifted_segments: Cell<u64>,
    solid_segments: Cell<u64>,
    negative_xy_segments: Cell<u64>,
}

impl ScanCounters {
    fn record(&self, case: &ScanCase, stats: ScanStats) {
        let count = stats.segments as u64;
        match case.shape {
            ShapeClass::Rectangle => {
                self.rectangle_segments
                    .set(self.rectangle_segments.get() + count);
            }
            ShapeClass::SubSpacingRectangle => {
                self.sub_spacing_segments
                    .set(self.sub_spacing_segments.get() + count);
            }
            ShapeClass::LNotch | ShapeClass::UNotch => {
                self.notch_segments.set(self.notch_segments.get() + count);
            }
            ShapeClass::HoledRectangle => {
                self.hole_segments.set(self.hole_segments.get() + count);
            }
        }
        if case.x_shift != 0 {
            self.shifted_segments
                .set(self.shifted_segments.get() + count);
        }
        if case.adjust_for_solid {
            self.solid_segments.set(self.solid_segments.get() + count);
        }
        if case
            .expoly
            .contour
            .points
            .iter()
            .any(|p| p.x < 0 || p.y < 0)
        {
            self.negative_xy_segments
                .set(self.negative_xy_segments.get() + count);
        }
    }

    fn assert_all_positive(&self) {
        assert!(self.rectangle_segments.get() > 0, "no rectangle segment");
        assert!(
            self.sub_spacing_segments.get() > 0,
            "no positive sub-spacing segment"
        );
        assert!(self.notch_segments.get() > 0, "no notch segment");
        assert!(self.hole_segments.get() > 0, "no holed-rectangle segment");
        assert!(self.shifted_segments.get() > 0, "no shifted segment");
        assert!(self.solid_segments.get() > 0, "no solid segment");
        assert!(
            self.negative_xy_segments.get() > 0,
            "no fixture with negative XY"
        );
    }

    fn report(&self) -> String {
        format!(
            "rect={} sub_spacing={} notch={} hole={} shifted={} solid={} negative_xy={}",
            self.rectangle_segments.get(),
            self.sub_spacing_segments.get(),
            self.notch_segments.get(),
            self.hole_segments.get(),
            self.shifted_segments.get(),
            self.solid_segments.get(),
            self.negative_xy_segments.get()
        )
    }
}

fn check_suite(cases: &[ScanCase], counters: &ScanCounters) -> TestCaseResult {
    for case in cases {
        let geometry = GeometryCase {
            polygons: vec![case.expoly.clone()],
        };
        if let Err(error) = validate_geometry(&geometry, &case.domain) {
            return fail(format!("{}: input validation failed: {error}", case.label));
        }
        let paths = run_scan(case);
        match check_scan(case, &paths) {
            Ok(stats) => counters.record(case, stats),
            Err(message) => return fail(format!("{}: {message}", case.label)),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// M-RECTILINEAR-SCAN
// ---------------------------------------------------------------------------

#[test]
fn proptest_rectilinear_scan_intervals() {
    let run = PropertyRun::new(SCAN_ID, SCAN_TEST).expect("M-RECTILINEAR-SCAN registry context");
    let limits = live_scan_limits(&run);
    let strategy = scan_params_strategy(&limits);
    let calls = Cell::new(0_u64);
    let counters = ScanCounters::default();
    run.check(&strategy, |params, _guard| {
        calls.set(calls.get() + 1);
        let suite = build_suite(&params, &limits);
        check_suite(&suite, &counters)
    })
    .expect("M-RECTILINEAR-SCAN complete interval coverage");
    counters.assert_all_positive();
    println!(
        "M-RECTILINEAR-SCAN accepted_calls={} {}",
        calls.get(),
        counters.report()
    );
}

// ---------------------------------------------------------------------------
// M-RECTILINEAR-SPACING
// ---------------------------------------------------------------------------

#[test]
fn proptest_rectilinear_solid_spacing() {
    let run =
        PropertyRun::new(SPACING_ID, SPACING_TEST).expect("M-RECTILINEAR-SPACING registry context");
    let limits = live_spacing_limits(&run);
    let strategy = spacing_params_strategy(&limits);
    let calls = Cell::new(0_u64);
    let below = Cell::new(0_u64);
    let exact = Cell::new(0_u64);
    let interval = Cell::new(0_u64);
    let over_cap = Cell::new(0_u64);
    run.check(&strategy, |params, _guard| {
        calls.set(calls.get() + 1);
        for pair in params.pairs() {
            let (value, branch) = match check_spacing_pair(&pair, &limits) {
                Ok(result) => result,
                Err(message) => return fail(message),
            };
            if value < pair.distance {
                return fail(format!(
                    "{}: canonical spacing {} shrinks distance {}",
                    pair.label, value, pair.distance
                ));
            }
            if pair.width < pair.distance {
                below.set(below.get() + 1);
            }
            if pair.width > 0 && pair.width % pair.distance == 0 {
                exact.set(exact.get() + 1);
            }
            match branch {
                SpacingBranch::OverCap => over_cap.set(over_cap.get() + 1),
                SpacingBranch::Interval => interval.set(interval.get() + 1),
                SpacingBranch::BelowDistance => {}
            }
        }
        Ok(())
    })
    .expect("M-RECTILINEAR-SPACING canonical intervals and cap");
    assert!(below.get() > 0, "no width-below-distance case");
    assert!(exact.get() > 0, "no exact-interval-boundary case");
    assert!(interval.get() > 0, "no truncation-interval case");
    assert!(over_cap.get() > 0, "no over-cap case");
    println!(
        "M-RECTILINEAR-SPACING accepted_calls={} width_below_distance={} \
         exact_multiple={} interval={} over_cap={}",
        calls.get(),
        below.get(),
        exact.get(),
        interval.get(),
        over_cap.get()
    );
}

// ---------------------------------------------------------------------------
// Deterministic controls (same checkers, explicit wrong outputs)
// ---------------------------------------------------------------------------

/// Build a deterministic case with the control metadata used by the mutants.
fn control_case(
    shape: ShapeClass,
    expoly: ExPolygon,
    spacing: i64,
    adjust_for_solid: bool,
    x_shift: i64,
) -> ScanCase {
    let limits = checked_scan_limits(&probe_scan_bound_map()).expect("probe scan bounds");
    ScanCase {
        label: format!("{shape:?}"),
        shape,
        expoly,
        domain: geometry_domain(shape.class(), &limits),
        spacing,
        adjust_for_solid,
        x_shift,
        z: 0.2,
        speed_factor: 1.0,
        flow_factor: 1.0,
        line_width: 0.4,
        role: if adjust_for_solid {
            ExtrusionRole::TopSolidInfill
        } else {
            ExtrusionRole::SparseInfill
        },
        max_rows: limits.max_rows,
    }
}

fn validate_control(case: &ScanCase) {
    let geometry = GeometryCase {
        polygons: vec![case.expoly.clone()],
    };
    validate_geometry(&geometry, &case.domain).expect("control fixture is valid geometry");
}

fn mutated_path(base: &ExtrusionPath3D, start_mm: Point2, end_mm: Point2) -> ExtrusionPath3D {
    let (sx, sy) = start_mm.to_mm();
    let (ex, ey) = end_mm.to_mm();
    let mut start = base.points[0];
    start.x = sx;
    start.y = sy;
    let mut end = base.points[1];
    end.x = ex;
    end.y = ey;
    ExtrusionPath3D {
        points: vec![start, end],
        ..base.clone()
    }
}

fn assert_points_in_material(expoly: &ExPolygon, points: &[Point2]) {
    for p in points {
        let location = point_in_region(*p, std::slice::from_ref(expoly)).expect("point oracle");
        assert_eq!(
            location,
            PointLocation::Inside,
            "control sample ({}, {}) must be inside material",
            p.x,
            p.y
        );
    }
}

#[test]
fn rect_controls() {
    // Exact hole control: outer [0,10] x [0,10] mm, hole [2,3] x [4,6] mm.
    // Units: 10 mm = 100_000, 1 mm = 10_000.
    let hole_outer_x1 = mm_to_units(10.0);
    let hole_x0 = mm_to_units(2.0);
    let hole_x1 = mm_to_units(3.0);
    let hole_y0 = mm_to_units(4.0);
    let hole_y1 = mm_to_units(6.0);
    let hole_expoly = ExPolygon {
        contour: ring(vec![
            point(0, 0),
            point(hole_outer_x1, 0),
            point(hole_outer_x1, hole_outer_x1),
            point(0, hole_outer_x1),
        ]),
        holes: vec![ring(vec![
            // Clockwise hole ring.
            point(hole_x0, hole_y0),
            point(hole_x0, hole_y1),
            point(hole_x1, hole_y1),
            point(hole_x1, hole_y0),
        ])],
    };
    let hole_case = control_case(
        ShapeClass::HoledRectangle,
        hole_expoly.clone(),
        mm_to_units(1.0),
        false,
        0,
    );
    validate_control(&hole_case);
    let hole_paths = run_scan(&hole_case);
    let hole_stats = check_scan(&hole_case, &hole_paths).expect("real hole baseline accepted");
    assert_eq!(hole_stats.rows, 10, "1 mm rows across a 10 mm span");
    assert_eq!(
        hole_stats.segments, 12,
        "rows 4 and 5 split around the hole: 8 + 2*2"
    );
    assert!(
        hole_stats.segments > hole_stats.rows,
        "a hole-crossing row is two segments, not one row"
    );

    // The charter's exact mutant: both endpoints and the midpoint lie in
    // material, but the open interval x in (2,3) crosses the hole.
    let mutant_start = point(hole_x0 - mm_to_units(1.0), mm_to_units(5.0));
    let mutant_end = point(hole_x1 + mm_to_units(6.0), mm_to_units(5.0));
    let mutant_mid = point(
        (mutant_start.x + mutant_end.x) / 2,
        (mutant_start.y + mutant_end.y) / 2,
    );
    assert_eq!(mutant_mid, point(mm_to_units(5.0), mm_to_units(5.0)));
    assert_points_in_material(&hole_expoly, &[mutant_start, mutant_end, mutant_mid]);
    assert!(
        !segment_in_region(
            [mutant_start, mutant_end],
            std::slice::from_ref(&hole_expoly),
            BoundaryRule::Inclusive,
        )
        .expect("hole segment oracle"),
        "the full-segment oracle must reject the hole-crossing chord"
    );

    let row_five = |paths: &[ExtrusionPath3D]| -> Vec<ExtrusionPath3D> {
        paths
            .iter()
            .filter(|path| mm_to_units(path.points[0].y) == mm_to_units(5.0))
            .cloned()
            .collect()
    };
    let split_row = row_five(&hole_paths);
    assert_eq!(split_row.len(), 2, "row y=5 is split in two");

    // A shortcut whose endpoints and midpoint hide the outside subinterval.
    let mut shortcut = hole_paths
        .iter()
        .filter(|path| mm_to_units(path.points[0].y) != mm_to_units(5.0))
        .cloned()
        .collect::<Vec<_>>();
    shortcut.push(mutated_path(&split_row[0], mutant_start, mutant_end));
    assert!(
        check_scan(&hole_case, &shortcut).is_err(),
        "hole-crossing shortcut accepted"
    );

    // Removing one interval of a split row must fail the complete multiset.
    let mut removed = hole_paths.clone();
    let removed_index = removed
        .iter()
        .position(|path| mm_to_units(path.points[0].y) == mm_to_units(5.0))
        .expect("split row present");
    removed.remove(removed_index);
    assert!(
        check_scan(&hole_case, &removed).is_err(),
        "removed interval accepted"
    );

    // Filling the hole (one merged segment across the row) must fail both the
    // multiset comparison and the boundary-inclusive whole-segment oracle.
    let merged = {
        let mut first = split_row[0].clone();
        first.points[1] = split_row[1].points[1];
        first
    };
    let mut filled = hole_paths
        .iter()
        .filter(|path| mm_to_units(path.points[0].y) != mm_to_units(5.0))
        .cloned()
        .collect::<Vec<_>>();
    filled.push(merged);
    assert!(
        !segment_in_region(
            [
                point(mm_to_units(0.0), mm_to_units(5.0)),
                point(mm_to_units(10.0), mm_to_units(5.0)),
            ],
            std::slice::from_ref(&hole_expoly),
            BoundaryRule::Inclusive,
        )
        .expect("filled-hole oracle"),
        "a filled-hole chord must leave the material"
    );
    assert!(
        check_scan(&hole_case, &filled).is_err(),
        "filled hole accepted"
    );

    // Concavity chord: outer [0,10] x [0,10] mm with the top notch
    // [4,5] x [6,10] mm cut out. At y=8 mm the material is [0,4] U [5,10].
    // Segment (3,8) -> (8,8): endpoints and midpoint (5.5,8) are in material,
    // the open interval x in (4,5) is outside.
    let notch_x0 = mm_to_units(4.0);
    let notch_x1 = mm_to_units(5.0);
    let notch_y0 = mm_to_units(6.0);
    let notch_outer = mm_to_units(10.0);
    let notch_expoly = ExPolygon {
        contour: ring(vec![
            point(0, 0),
            point(notch_outer, 0),
            point(notch_outer, notch_outer),
            point(notch_x1, notch_outer),
            point(notch_x1, notch_y0),
            point(notch_x0, notch_y0),
            point(notch_x0, notch_outer),
            point(0, notch_outer),
        ]),
        holes: Vec::new(),
    };
    let notch_case = control_case(
        ShapeClass::UNotch,
        notch_expoly.clone(),
        mm_to_units(1.0),
        false,
        0,
    );
    validate_control(&notch_case);
    let notch_paths = run_scan(&notch_case);
    let notch_stats = check_scan(&notch_case, &notch_paths).expect("real U-notch baseline");
    assert_eq!(notch_stats.rows, 10);
    assert_eq!(
        notch_stats.segments, 14,
        "rows 6..9 split into two arms: 6 + 4*2"
    );
    let chord_start = point(mm_to_units(3.0), mm_to_units(8.0));
    let chord_end = point(mm_to_units(8.0), mm_to_units(8.0));
    let chord_mid = point(mm_to_units(5.5), mm_to_units(8.0));
    assert_points_in_material(&notch_expoly, &[chord_start, chord_end, chord_mid]);
    assert!(
        !segment_in_region(
            [chord_start, chord_end],
            std::slice::from_ref(&notch_expoly),
            BoundaryRule::Inclusive,
        )
        .expect("notch chord oracle"),
        "the notch chord must leave the material"
    );
    let mut chord = notch_paths
        .iter()
        .filter(|path| mm_to_units(path.points[0].y) != mm_to_units(8.0))
        .cloned()
        .collect::<Vec<_>>();
    let row_eight = notch_paths
        .iter()
        .find(|path| mm_to_units(path.points[0].y) == mm_to_units(8.0))
        .expect("row y=8 present");
    chord.push(mutated_path(row_eight, chord_start, chord_end));
    assert!(
        check_scan(&notch_case, &chord).is_err(),
        "concavity chord accepted"
    );

    // Shifted output: compare against the translated region, never against the
    // unshifted polygon.
    let shift = mm_to_units(10.0);
    let shift_expoly = rect_expoly(0, 0, mm_to_units(10.0), mm_to_units(10.0));
    let shift_case = control_case(
        ShapeClass::Rectangle,
        shift_expoly.clone(),
        mm_to_units(2.0),
        false,
        shift,
    );
    validate_control(&shift_case);
    let shifted_paths = run_scan(&shift_case);
    let shifted_stats = check_scan(&shift_case, &shifted_paths).expect("shifted output accepted");
    assert_eq!(shifted_stats.rows, 5);
    assert_eq!(shifted_stats.segments, 5);
    let shifted_segment = [
        point(mm_to_units(10.0), mm_to_units(0.0)),
        point(mm_to_units(20.0), mm_to_units(0.0)),
    ];
    assert!(
        !segment_in_region(
            shifted_segment,
            std::slice::from_ref(&shift_expoly),
            BoundaryRule::Inclusive,
        )
        .expect("unshifted oracle"),
        "an x-shifted segment leaves the unshifted polygon"
    );

    // A positive sub-spacing rectangle has exactly one row at its minimum
    // boundary, not a blanket zero or blanket nonempty rule.
    let sub_expoly = rect_expoly(0, 0, mm_to_units(10.0), mm_to_units(0.5));
    let sub_case = control_case(
        ShapeClass::SubSpacingRectangle,
        sub_expoly,
        mm_to_units(1.0),
        false,
        0,
    );
    validate_control(&sub_case);
    let sub_stats = check_scan(&sub_case, &run_scan(&sub_case)).expect("sub-spacing rectangle");
    assert_eq!(sub_stats.rows, 1);
    assert_eq!(sub_stats.segments, 1);

    // Empty, degenerate and nonpositive-spacing modes are separate from the
    // generated exact model.
    let empty = ExPolygon {
        contour: ring(Vec::new()),
        holes: Vec::new(),
    };
    assert!(run_scan(&control_case(
        ShapeClass::Rectangle,
        empty,
        10_000,
        false,
        0
    ))
    .is_empty());
    let one_point = ExPolygon {
        contour: ring(vec![point(0, 0)]),
        holes: Vec::new(),
    };
    assert!(run_scan(&control_case(
        ShapeClass::Rectangle,
        one_point,
        10_000,
        false,
        0
    ))
    .is_empty());
    let zero_height = ExPolygon {
        contour: ring(vec![
            point(0, 0),
            point(10_000, 0),
            point(10_000, 0),
            point(0, 0),
        ]),
        holes: Vec::new(),
    };
    assert!(run_scan(&control_case(
        ShapeClass::Rectangle,
        zero_height,
        10_000,
        false,
        0
    ))
    .is_empty());
    let zero_width = ExPolygon {
        contour: ring(vec![
            point(0, 0),
            point(0, 10_000),
            point(0, 10_000),
            point(0, 0),
        ]),
        holes: Vec::new(),
    };
    assert!(run_scan(&control_case(
        ShapeClass::Rectangle,
        zero_width,
        10_000,
        false,
        0
    ))
    .is_empty());
    let square = rect_expoly(0, 0, 10_000, 10_000);
    for spacing in [0_i64, -10_000] {
        assert!(
            run_scan(&control_case(
                ShapeClass::Rectangle,
                square.clone(),
                spacing,
                false,
                0
            ))
            .is_empty(),
            "nonpositive spacing {spacing} must emit nothing"
        );
    }

    // Tangential zero-length row: the triangle base at y=0 is touched only at
    // a vertex, so the duplicate X intersection is skipped. This mode is
    // outside the axis-aligned exact model and is observed separately.
    let triangle = ExPolygon {
        contour: ring(vec![
            point(0, 0),
            point(100_000, 100_000),
            point(0, 200_000),
        ]),
        holes: Vec::new(),
    };
    let triangle_paths = run_scan(&control_case(
        ShapeClass::Rectangle,
        triangle,
        20_000,
        false,
        0,
    ));
    assert!(!triangle_paths.is_empty());
    assert!(
        triangle_paths
            .iter()
            .all(|path| mm_to_units(path.points[0].y) != 0),
        "the tangential base vertex must emit no zero-length row"
    );

    // Spacing controls: the 1.5 mm / 1.0 mm over-cap witness and the exact
    // interval / rounding boundaries, compared with the independent canonical
    // reference through the same checker the property uses.
    let witness = (mm_to_units(1.5), mm_to_units(1.0));
    assert_eq!(
        canonical_adjusted_spacing(witness.0, witness.1),
        Some(12_000)
    );
    assert_eq!(
        super::adjust_solid_spacing(witness.0, witness.1),
        12_000,
        "width=1.5mm distance=1.0mm must cap at 1.2mm, not reset to 1.0mm"
    );
    for (width, distance, expected) in [
        // Exact canonical cases, computed independently from
        // `Fill::_adjust_solid_spacing`: the 1.5/1.0 over-cap witness, the
        // 1.2x boundary from both sides, the epsilon zero-interval tie, a width
        // below distance, and the two existing solid-role spans.
        (15_000_i64, 10_000_i64, 12_000_i64),
        (12_000, 10_000, 11_999), // just under the cap
        (12_001, 10_000, 12_000), // factor exactly 1.2: no cap
        (12_002, 10_000, 12_000), // just over the cap
        (10_000, 10_000, 10_000), // epsilon tie: zero intervals
        (5_000, 10_000, 10_000),  // width below distance
        (100_000, 4_200, 4_347),  // 0.42 mm solid span
        (100_000, 5_000, 5_263),  // 0.50 mm solid span
        // Additional interval and rounding boundaries.
        (9_000, 10_000, 10_000),
        (10_001, 10_000, 10_000),
        (20_000, 10_000, 12_000),
        (50_000, 10_000, 12_000),
        (60_000, 10_000, 11_999),
        (70_000, 10_000, 11_666),
        (7, 5, 6),
        (8, 5, 6),
        (0, 10_000, 10_000),
    ] {
        assert_eq!(
            check_spacing_with(width, distance, super::adjust_solid_spacing),
            Ok(expected),
            "checked spacing case width={width} distance={distance}"
        );
        assert_eq!(canonical_adjusted_spacing(width, distance), Some(expected));
    }

    // The same checker rejects the pre-fix owner behavior and the former
    // rounded-interval rule on the formerly failing inputs.
    let pre_fix = |width: i64, distance: i64| {
        let count = width / distance;
        if count < 1 {
            return distance;
        }
        let new_distance = ((width as f64) / (count as f64)).round() as i64;
        if (new_distance as f64) > (distance as f64) * 1.2 {
            distance
        } else {
            new_distance
        }
    };
    assert!(check_spacing_with(witness.0, witness.1, pre_fix).is_err());
    assert!(check_spacing_with(60_000, 10_000, pre_fix).is_err());
    assert!(check_spacing_with(9_000, 10_000, pre_fix).is_ok());
}

// ---------------------------------------------------------------------------
// Shrink probes and bound-construction matrix
// ---------------------------------------------------------------------------

fn scan_bits(params: &ScanParams) -> Vec<u64> {
    vec![
        params.origin_x as u64,
        params.origin_y as u64,
        params.width as u64,
        params.height as u64,
        params.spacing as u64,
        params.shift as u64,
        u64::from(params.z_centi),
        u64::from(params.speed_per_mille),
        u64::from(params.flow_per_mille),
        u64::from(params.line_width_milli),
    ]
}

fn spacing_bits(params: &SpacingParams) -> Vec<u64> {
    params
        .pairs()
        .iter()
        .flat_map(|pair| [pair.width as u64, pair.distance as u64])
        .collect()
}

fn probe_scan_state(
    params: &ScanParams,
    limits: &ScanLimits,
    counters: &ScanCounters,
) -> TestCaseResult {
    let suite = build_suite(params, limits);
    check_suite(&suite, counters)
}

fn probe_spacing_state(params: &SpacingParams, limits: &SpacingLimits) -> TestCaseResult {
    for pair in params.pairs() {
        if let Err(message) = check_spacing_pair(&pair, limits) {
            return fail(message);
        }
    }
    Ok(())
}

#[test]
fn rect_shrink_bounds() {
    // Bound-construction rejection matrix. Missing and zero bounds must be
    // rejected, never defaulted.
    fn invalid_scan(label: &str, bounds: BTreeMap<String, u64>) {
        match checked_scan_limits(&bounds) {
            Err(DomainError::InvalidBounds(message)) => assert!(!message.is_empty(), "{label}"),
            other => panic!("{label}: expected InvalidBounds, got {other:?}"),
        }
    }
    fn overflow_scan(label: &str, bounds: BTreeMap<String, u64>) {
        assert!(
            matches!(
                checked_scan_limits(&bounds),
                Err(DomainError::ArithmeticOverflow)
            ),
            "{label}: expected ArithmeticOverflow"
        );
    }
    let valid_scan = probe_scan_bound_map();
    checked_scan_limits(&valid_scan).expect("valid scan bounds");
    for key in SCAN_BOUND_KEYS {
        let mut bounds = valid_scan.clone();
        bounds.remove(key);
        invalid_scan("missing scan bound", bounds);
    }
    for (key, value) in [
        ("max_abs_units", 0_u64),
        ("max_abs_units", 3),
        ("max_vertices", 0),
        ("max_vertices", 7),
        ("max_holes", 0),
        ("max_components", 0),
        ("max_shrink_candidates", 0),
        ("max_spacing_units", 0),
        ("max_spacing_units", 1),
        ("max_rows", 0),
        ("max_rows", 1),
    ] {
        let mut bounds = valid_scan.clone();
        bounds.insert(key.to_owned(), value);
        invalid_scan("invalid scan minimum", bounds);
    }
    for key in ["max_abs_units", "max_spacing_units"] {
        let mut bounds = valid_scan.clone();
        bounds.insert(key.to_owned(), i64::MAX as u64 + 1);
        overflow_scan("narrowing scan bound", bounds);
    }
    for key in [
        "max_vertices",
        "max_holes",
        "max_components",
        "max_shrink_candidates",
    ] {
        let mut bounds = valid_scan.clone();
        bounds.insert(key.to_owned(), u32::MAX as u64 + 1);
        overflow_scan("oversized count bound", bounds);
    }
    {
        // `u64::MAX` rows converts to `usize` on 64-bit but overflows the
        // checked `max_rows * max_spacing_units` coverage product; on 32-bit it
        // fails the `usize` conversion. Either way the arm is checked.
        let mut bounds = valid_scan.clone();
        bounds.insert("max_rows".to_owned(), u64::MAX);
        overflow_scan("usize coverage overflow", bounds);
    }
    {
        let mut bounds = valid_scan.clone();
        bounds.insert("max_rows".to_owned(), 1_000_000_000);
        invalid_scan("row budget", bounds);
    }

    let valid_spacing = probe_spacing_bound_map();
    checked_spacing_limits(&valid_spacing).expect("valid spacing bounds");
    for key in SPACING_BOUND_KEYS {
        let mut bounds = valid_spacing.clone();
        bounds.remove(key);
        assert!(matches!(
            checked_spacing_limits(&bounds),
            Err(DomainError::InvalidBounds(_))
        ));
    }
    for (key, value) in [
        ("max_width_units", 0_u64),
        ("max_width_units", 9),
        ("max_spacing_units", 0),
        ("max_spacing_units", 4),
    ] {
        let mut bounds = valid_spacing.clone();
        bounds.insert(key.to_owned(), value);
        assert!(matches!(
            checked_spacing_limits(&bounds),
            Err(DomainError::InvalidBounds(_))
        ));
    }
    for key in SPACING_BOUND_KEYS {
        let mut bounds = valid_spacing.clone();
        bounds.insert(key.to_owned(), i64::MAX as u64 + 1);
        assert!(matches!(
            checked_spacing_limits(&bounds),
            Err(DomainError::ArithmeticOverflow)
        ));
    }

    // Scan shrink probe over every generated fixture class and metadata mode.
    // Each simplified/complicated state is revalidated and rechecked.
    let scan_limits = checked_scan_limits(&probe_scan_bound_map()).expect("probe scan bounds");
    let mut runner = TestRunner::deterministic();
    let scan_strategy = scan_params_strategy(&scan_limits);
    let mut scan_tree = scan_strategy
        .new_tree(&mut runner)
        .expect("scan value tree");
    let initial = scan_tree.current();
    let counters = ScanCounters::default();
    probe_scan_state(&initial, &scan_limits, &counters).expect("initial scan probe");
    let initial_bits = scan_bits(&initial);
    let mut simplified = 0_usize;
    let mut changed = false;
    let mut repeated_complicate = 0_usize;
    loop {
        let before = scan_tree.current();
        if !scan_tree.simplify() {
            // A false simplify must not move the value.
            assert_eq!(
                scan_bits(&scan_tree.current()),
                scan_bits(&before),
                "a false scan simplify changed the case"
            );
            break;
        }
        simplified += 1;
        assert!(simplified <= SHRINK_STEPS, "scan shrink did not terminate");
        let current = scan_tree.current();
        probe_scan_state(&current, &scan_limits, &counters).expect("simplified scan probe");
        changed |= scan_bits(&current) != initial_bits;
        let mut restored = 0_usize;
        while scan_tree.complicate() {
            restored += 1;
            assert!(restored <= SHRINK_STEPS, "complicate did not terminate");
            probe_scan_state(&scan_tree.current(), &scan_limits, &counters)
                .expect("complicated scan probe");
        }
        repeated_complicate += restored;
    }
    assert!(simplified > 0, "false-only scan simplify probe");
    assert!(
        changed,
        "no scan parameter was reduced from a nonminimal start"
    );
    assert!(
        repeated_complicate >= simplified,
        "complicate never restored a scan simplification"
    );
    assert!(
        repeated_complicate > 0,
        "repeated complicate was never exercised"
    );
    // Repeated complicate calls past exhaustion must keep returning false and
    // must not move the value.
    let settled = scan_bits(&scan_tree.current());
    assert!(
        !scan_tree.complicate(),
        "complicate after scan exhaustion must be false"
    );
    assert_eq!(scan_bits(&scan_tree.current()), settled);
    counters.assert_all_positive();
    println!(
        "rect_shrink_bounds scan simplify={simplified} complicate={repeated_complicate} {}",
        counters.report()
    );

    // Spacing shrink probe: every pair is revalidated through the same checker.
    let spacing_limits =
        checked_spacing_limits(&probe_spacing_bound_map()).expect("probe spacing bounds");
    let spacing_strategy = spacing_params_strategy(&spacing_limits);
    let mut spacing_tree = spacing_strategy
        .new_tree(&mut runner)
        .expect("spacing value tree");
    let spacing_initial = spacing_tree.current();
    probe_spacing_state(&spacing_initial, &spacing_limits).expect("initial spacing probe");
    let spacing_initial_bits = spacing_bits(&spacing_initial);
    let mut spacing_simplified = 0_usize;
    let mut spacing_changed = false;
    let mut spacing_restores = 0_usize;
    loop {
        let before = spacing_tree.current();
        if !spacing_tree.simplify() {
            assert_eq!(
                spacing_bits(&spacing_tree.current()),
                spacing_bits(&before),
                "a false spacing simplify changed the pair set"
            );
            break;
        }
        spacing_simplified += 1;
        assert!(
            spacing_simplified <= SHRINK_STEPS,
            "spacing shrink did not terminate"
        );
        let current = spacing_tree.current();
        probe_spacing_state(&current, &spacing_limits).expect("simplified spacing probe");
        spacing_changed |= spacing_bits(&current) != spacing_initial_bits;
        let mut restored = 0_usize;
        while spacing_tree.complicate() {
            restored += 1;
            assert!(
                restored <= SHRINK_STEPS,
                "spacing complicate did not terminate"
            );
            probe_spacing_state(&spacing_tree.current(), &spacing_limits)
                .expect("complicated spacing probe");
        }
        spacing_restores += restored;
    }
    assert!(spacing_simplified > 0, "false-only spacing simplify probe");
    assert!(
        spacing_changed,
        "no spacing parameter was reduced from a nonminimal start"
    );
    assert!(
        spacing_restores > 0,
        "repeated spacing complicate was never exercised"
    );
    let spacing_settled = spacing_bits(&spacing_tree.current());
    assert!(
        !spacing_tree.complicate(),
        "complicate after spacing exhaustion must be false"
    );
    assert_eq!(spacing_bits(&spacing_tree.current()), spacing_settled);
    println!("rect_shrink_bounds spacing simplify={spacing_simplified}");

    // A fresh spacing tree must reject a pair outside its checked profile
    // instead of silently coercing it.
    let invalid_pair = SpacingPair {
        label: "outside-profile",
        width: spacing_limits.max_width_units + 1,
        distance: spacing_limits.max_spacing_units,
    };
    assert!(check_spacing_pair(&invalid_pair, &spacing_limits).is_err());
    let zero_distance = SpacingPair {
        label: "zero-distance",
        width: 1,
        distance: 0,
    };
    assert!(check_spacing_pair(&zero_distance, &spacing_limits).is_err());
}
