//! Opt-in property tests for this module's private tree-support render kernels.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_14_pbt-support-modules`):
//!
//! * `M-TREE-SCAN` — `TreeSupport::scan_fill_region` on axis-aligned
//!   rectangle, open notch and rectangular-hole domains in horizontal and
//!   vertical modes. The checker derives the **complete** stroke set from the
//!   fixture's own rectangle fields: rows at `scan_min + spacing/2 + n*spacing`
//!   while `< scan_max`, and a separate mid-scan fallback row only when no
//!   ordinary stroke exists. It compares whole strokes, checks every
//!   consecutive segment for whole-segment containment (hole/notch
//!   exclusion), and inspects role, width, flow, Z and speed. Nonpositive
//!   spacing and degenerate geometry are a separate empty mode. The exact
//!   vertical witness `[0,10] × [0,20]` mm at 2.5 mm must emit four strokes at
//!   x = 1.25, 3.75, 6.25, 8.75 spanning y `[0,20]`; this is the corrected
//!   behaviour of the defect fixed in `1515f8ce` (the existing
//!   `tests::vertical_scan_fill_spans_the_full_y_extent_regression` stays the
//!   concrete regression; this file adds none). The 0.5 mm square at 2 mm
//!   pitch exercises the fallback branch only, not a universal tiny-tip
//!   guarantee.
//! * `M-TREE-RENDER` — `TreeSupport::render_polygon_with_wall_count` on roomy
//!   axis-aligned rectangles and holed rectangles. Walls are checked against
//!   analytic center bands at inset `line_width * (i + 0.5)` (a hole grows
//!   outward by the same distance) and fill against the analytic region inset
//!   by `line_width * wall_count`, never against `offset_polygons` output.
//!   Modes: a production-supported positive wall count (through
//!   `from_config`, whose `.max(1)` clamp means zero never reaches the
//!   kernel from config), a private-kernel zero-wall mode, and an
//!   offset-collapse mode whose possibly-empty outcome is predicted exactly.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! # Exactness and the conversion allowance
//!
//! Both kernels work in f32/f64 millimetres. Every scan fixture coordinate is
//! a multiple of 1/8 mm (1250 units) and every scan spacing a multiple of
//! 1/4 mm, so `units_to_mm`, the f64 row accumulation, the mid-scan fallback
//! and `mm_to_units` are all exact dyadic arithmetic and `M-TREE-SCAN` is
//! compared **exactly**, including rows that land exactly on a hole edge
//! (the kernel's crossing test `(a > scan) != (b > scan)` makes a ring's low
//! edge inclusive and its high edge exclusive, which the model reproduces).
//!
//! `M-TREE-RENDER` routes through Clipper offsetting, so a proven allowance
//! `CLIPPER_ALLOW = 2` canonical units (200 nm) applies to every
//! Clipper-derived coordinate: the offset delta is a dyadic multiple of
//! 1/4 mm, so `delta_mm as f64 * 10_000` is exact; Clipper2 places each
//! offset vertex in f64 and rounds it to the nearest integer (≤ 0.5 unit);
//! the scan kernel then re-derives rows/crossings from those integers through
//! one f32 round trip and one `mm_to_units` rounding (≤ 0.5 unit plus an f32
//! relative error of 2^-24, below 0.06 unit for |coordinate| ≤ 10^6 units).
//! The total is ≤ 1.06 unit, so 2 units is a factor-of-two margin. The
//! generator keeps every fill row at least 625 units (1/16 mm) away from
//! every analytic fill edge, far outside the allowance, so the row set and
//! each row's interval count are unambiguous; the checker re-verifies this
//! separation and reports a violation as a fixture (generator) failure. The
//! zero-wall mode never calls Clipper and is compared with allowance 0.
//!
//! # Literal review
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. Watched-type sites here (a
//! watched type is a `pub` struct with at least five named fields under
//! `crates/*/src`):
//!
//! * `Point3WithWidth` (`crates/slicer-ir/src/slice_ir.rs`) — the only literal
//!   is in `stroke_like`, which sets `x`/`y` and uses FRU over a real emitted
//!   point (`..template.points[0]`).
//! * `ExtrusionPath3D` (`crates/slicer-ir/src/slice_ir.rs`) — no literal;
//!   mutants clone a real emitted path and replace its points.
//! * `GeometryBounds` and `GeometryDomain`
//!   (`crates/slicer-sdk/src/test_support/property_testing/domains/mod.rs`) —
//!   one literal each in `geometry_domain`, both carrying an `// exhaustive:`
//!   note because every field is derived from the live limits or the
//!   fixture's topology class.
//! * `Rect`, `ScanCase`, `RenderCase`, `Band`, `FillModel`, the limits, stats
//!   and failure types are private module-local types, not watched.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! controls are mutation controls over the SAME checkers the properties use,
//! each first accepting the unmodified observation and then asserting the
//! specific `CheckFailure` class of the mutant, so none is a decorative or
//! success-only assertion; both properties require positive per-subcase
//! counters from deterministic witnesses built inside their callbacks.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    mm_to_units, units_to_mm, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole,
    Point2, Point3WithWidth, Polygon,
};
use slicer_sdk::test_support::property_testing::domains::{
    BoundaryRule, DomainError, GeometryBounds, GeometryCase, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, segment_in_region, validate_geometry, PointLocation,
};
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_sdk::traits::LayerModule;

use super::{TreeSupport, BASE_SPEED};

/// 1/16 mm in canonical units.
const TICK: i64 = 625;
/// 1/8 mm: the coordinate grid of every generated fixture.
const GRID: i64 = 2 * TICK;
/// 1/4 mm: the scan-spacing quantum (so `spacing / 2` stays on `GRID`).
const SCAN_PITCH_QUANTUM: i64 = 2 * GRID;
/// 1/2 mm: the render line-width and fill-spacing quantum.
const RENDER_QUANTUM: i64 = 4 * GRID;
/// Coordinate ceiling `validate_geometry` accepts.
const VALIDATOR_ABS_CEILING: i64 = 1_000_000;
/// Runtime ceiling on any configured row bound.
const MAX_ROWS_CEILING: u32 = 1_024;
/// Step ceiling for the shrink probe's transition loops.
const MAX_SHRINK_STEPS: usize = 20_000;
/// Proven Clipper/f32 conversion allowance, canonical units (module docs).
const CLIPPER_ALLOW: i64 = 2;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

/// Inclusive `lo..=hi` pick; an empty range is a generator bug and panics
/// (a failure, never a rejection).
fn pick(raw: u32, lo: i64, hi: i64) -> i64 {
    assert!(lo <= hi, "generator range {lo}..={hi} is empty");
    lo + i64::from(raw) % (hi - lo + 1)
}

fn ceil_div(a: i64, b: i64) -> i64 {
    (a + b - 1) / b
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

fn bound_i64(bounds: &BTreeMap<String, u64>, key: &str) -> Result<i64, DomainError> {
    i64::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
}

fn bound_u32(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u32, DomainError> {
    u32::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
}

/// `2 * floor(max_abs / GRID) * GRID`, checked.
fn quantized_span(max_abs_units: i64) -> Result<i64, DomainError> {
    ((max_abs_units / GRID) * GRID)
        .checked_mul(2)
        .ok_or(DomainError::ArithmeticOverflow)
}

fn read_live_bounds(run: &PropertyRun, keys: &[&str]) -> BTreeMap<String, u64> {
    let mut bounds = BTreeMap::new();
    for key in keys {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("tree-support bound `{key}`: {error}"));
        bounds.insert((*key).to_owned(), value);
    }
    bounds
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared geometry frame
// ─────────────────────────────────────────────────────────────────────────────

/// An axis-aligned rectangle in the scan frame: `u` is the stroke (segment)
/// axis and `v` the scan axis. Horizontal mode maps `(u, v) = (x, y)`,
/// vertical mode `(u, v) = (y, x)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Rect {
    u0: i64,
    u1: i64,
    v0: i64,
    v1: i64,
}

impl Rect {
    /// An XY rectangle `[x0,x1] × [y0,y1]` expressed in the frame.
    fn from_xy(vertical: bool, x0: i64, x1: i64, y0: i64, y1: i64) -> Self {
        if vertical {
            Self {
                u0: y0,
                u1: y1,
                v0: x0,
                v1: x1,
            }
        } else {
            Self {
                u0: x0,
                u1: x1,
                v0: y0,
                v1: y1,
            }
        }
    }

    /// Grow outward by `by` (shrink when negative).
    fn grow(self, by: i64) -> Self {
        Self {
            u0: self.u0 - by,
            u1: self.u1 + by,
            v0: self.v0 - by,
            v1: self.v1 + by,
        }
    }

    fn is_proper(self) -> bool {
        self.u0 < self.u1 && self.v0 < self.v1
    }

    /// `self` lies strictly inside `outer` with at least `margin` clearance.
    fn strictly_inside(self, outer: Rect, margin: i64) -> bool {
        outer.u0 + margin < self.u0
            && self.u1 < outer.u1 - margin
            && outer.v0 + margin < self.v0
            && self.v1 < outer.v1 - margin
    }

    fn ccw(self, vertical: bool) -> Polygon {
        uv_ring(
            vertical,
            &[
                (self.u0, self.v0),
                (self.u1, self.v0),
                (self.u1, self.v1),
                (self.u0, self.v1),
            ],
        )
    }

    fn cw(self, vertical: bool) -> Polygon {
        uv_ring(
            vertical,
            &[
                (self.u0, self.v0),
                (self.u0, self.v1),
                (self.u1, self.v1),
                (self.u1, self.v0),
            ],
        )
    }
}

fn uv_point(vertical: bool, u: i64, v: i64) -> Point2 {
    if vertical {
        Point2 { x: v, y: u }
    } else {
        Point2 { x: u, y: v }
    }
}

/// Map a frame ring to XY preserving its winding: the vertical `(u,v) →
/// (y,x)` swap is a reflection, so its point order is reversed.
fn uv_ring(vertical: bool, points: &[(i64, i64)]) -> Polygon {
    let mut points: Vec<Point2> = points
        .iter()
        .map(|&(u, v)| uv_point(vertical, u, v))
        .collect();
    if vertical {
        points.reverse();
    }
    Polygon { points }
}

fn geometry_domain(
    class: TopologyClass,
    min_holes: usize,
    max_abs_units: i64,
    max_vertices: u32,
    max_holes: u32,
    max_components: u32,
    max_shrink_candidates: u32,
) -> GeometryDomain {
    // exhaustive: every field is the fixture's topology or the boundary rule the checker uses.
    GeometryDomain {
        class,
        min_holes,
        min_components: 1,
        boundary: BoundaryRule::Inclusive,
        // exhaustive: all five fields come from the live profile bounds.
        bounds: GeometryBounds {
            max_abs_units,
            max_vertices: max_vertices as usize,
            max_holes: max_holes as usize,
            max_components: max_components as usize,
            max_shrink_candidates,
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Shared checker pieces
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckFailure {
    /// The fixture itself is outside the declared domain (generator bug).
    Fixture(String),
    /// A path or point carries metadata the input did not specify.
    Metadata(String),
    /// An output coordinate is not exactly a canonical-unit value.
    OffGrid(String),
    /// A consecutive segment leaves its analytic region.
    Containment(String),
    /// A fixture with expected material produced no output.
    FalseEmpty,
    /// The stroke set differs from the complete expected row/interval set.
    Intervals(String),
    /// The wall loops differ from the analytic wall-center bands.
    WallBands(String),
}

impl fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(m) => write!(f, "fixture outside the domain: {m}"),
            Self::Metadata(m) => write!(f, "metadata: {m}"),
            Self::OffGrid(m) => write!(f, "off-grid coordinate: {m}"),
            Self::Containment(m) => write!(f, "containment: {m}"),
            Self::FalseEmpty => write!(f, "a fixture with material produced no support paths"),
            Self::Intervals(m) => write!(f, "intervals: {m}"),
            Self::WallBands(m) => write!(f, "wall bands: {m}"),
        }
    }
}

fn exact_units(value_mm: f32, what: &str) -> Result<i64, CheckFailure> {
    if !value_mm.is_finite() {
        return Err(CheckFailure::OffGrid(format!("{what} is nonfinite")));
    }
    let units = mm_to_units(value_mm);
    if units_to_mm(units).to_bits() != value_mm.to_bits() {
        return Err(CheckFailure::OffGrid(format!(
            "{what} = {value_mm} mm is not exactly a canonical unit"
        )));
    }
    Ok(units)
}

fn point_xy(p: &Point3WithWidth) -> Result<Point2, CheckFailure> {
    Ok(Point2 {
        x: exact_units(p.x, "x")?,
        y: exact_units(p.y, "y")?,
    })
}

/// `(u, v)` frame coordinates of an emitted point.
fn point_uv(vertical: bool, p: &Point3WithWidth) -> Result<(i64, i64), CheckFailure> {
    let q = point_xy(p)?;
    Ok(if vertical { (q.y, q.x) } else { (q.x, q.y) })
}

/// Every path must carry exactly the metadata the kernel was given:
/// `SupportMaterial`, the supplied speed factor and Z, the module line width,
/// flow 1.0 and no tool/order/overhang annotations.
fn check_metadata(
    paths: &[ExtrusionPath3D],
    z: f32,
    width: f32,
    speed: f32,
) -> Result<(), CheckFailure> {
    for path in paths {
        if path.role != ExtrusionRole::SupportMaterial {
            return Err(CheckFailure::Metadata(format!("role {:?}", path.role)));
        }
        if path.speed_factor.to_bits() != speed.to_bits() {
            return Err(CheckFailure::Metadata(format!(
                "speed_factor {} differs from the supplied {speed}",
                path.speed_factor
            )));
        }
        if path.tool_index.is_some() || path.order_lock.is_some() {
            return Err(CheckFailure::Metadata(
                "tool_index/order_lock set without input".to_owned(),
            ));
        }
        for p in &path.points {
            if p.z.to_bits() != z.to_bits() {
                return Err(CheckFailure::Metadata(format!(
                    "z {} differs from the supplied {z}",
                    p.z
                )));
            }
            if p.width.to_bits() != width.to_bits() {
                return Err(CheckFailure::Metadata(format!(
                    "width {} differs from the module line width {width}",
                    p.width
                )));
            }
            if p.flow_factor.to_bits() != 1.0_f32.to_bits() {
                return Err(CheckFailure::Metadata(format!(
                    "flow_factor {} is not 1.0",
                    p.flow_factor
                )));
            }
            if p.overhang_quartile.is_some()
                || p.dist_to_top_mm.to_bits() != 0.0_f32.to_bits()
                || p.overhang_distance_mm.is_some()
            {
                return Err(CheckFailure::Metadata(
                    "overhang/top-distance annotations set without input".to_owned(),
                ));
            }
        }
    }
    Ok(())
}

/// Whole-segment containment of EVERY consecutive pair (a closed wall repeats
/// its first point, so its closing edge is a window too).
fn contained_segments<'a>(
    paths: impl IntoIterator<Item = &'a ExtrusionPath3D>,
    region: &[ExPolygon],
    what: &str,
) -> Result<usize, CheckFailure> {
    let mut segments = 0;
    for path in paths {
        for w in path.points.windows(2) {
            let (a, b) = (point_xy(&w[0])?, point_xy(&w[1])?);
            let inside = segment_in_region([a, b], region, BoundaryRule::Inclusive)
                .map_err(|e| CheckFailure::Fixture(format!("containment predicate: {e}")))?;
            if !inside {
                return Err(CheckFailure::Containment(format!(
                    "{what} segment ({}, {}) -> ({}, {}) mm leaves its analytic region",
                    w[0].x, w[0].y, w[1].x, w[1].y
                )));
            }
            segments += 1;
        }
    }
    Ok(segments)
}

/// Normalize two-point strokes to sorted `(v, u_lo, u_hi)` frame triples.
fn stroke_triples<'a>(
    paths: impl IntoIterator<Item = &'a ExtrusionPath3D>,
    vertical: bool,
) -> Result<Vec<(i64, i64, i64)>, CheckFailure> {
    let mut out = Vec::new();
    for path in paths {
        let [s, e] = path.points.as_slice() else {
            return Err(CheckFailure::Intervals(format!(
                "a path has {} points; each stroke is one two-point interval",
                path.points.len()
            )));
        };
        let ((su, sv), (eu, ev)) = (point_uv(vertical, s)?, point_uv(vertical, e)?);
        if sv != ev || su == eu {
            return Err(CheckFailure::Intervals(format!(
                "stroke ({su},{sv}) -> ({eu},{ev}) frame units is not a positive-length scan stroke"
            )));
        }
        out.push((sv, su.min(eu), su.max(eu)));
    }
    out.sort_unstable();
    Ok(out)
}

fn split_row_count(strokes: &[(i64, i64, i64)]) -> usize {
    let mut per_row: BTreeMap<i64, usize> = BTreeMap::new();
    for &(v, _, _) in strokes {
        *per_row.entry(v).or_default() += 1;
    }
    per_row.values().filter(|n| **n >= 2).count()
}

fn first_row_diff(expected: &[(i64, i64, i64)], actual: &[(i64, i64, i64)]) -> String {
    let group = |s: &[(i64, i64, i64)]| {
        let mut m: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
        for &(v, a, b) in s {
            m.entry(v).or_default().push((a, b));
        }
        m
    };
    let (want, got) = (group(expected), group(actual));
    let rows: BTreeSet<i64> = want.keys().chain(got.keys()).copied().collect();
    let first = rows
        .into_iter()
        .find(|v| want.get(v) != got.get(v))
        .unwrap_or(i64::MIN);
    format!(
        "row v={first} units: expected {:?}, observed {:?} ({} expected / {} observed strokes)",
        want.get(&first),
        got.get(&first),
        expected.len(),
        actual.len()
    )
}

/// Build the module through its real config path; fields stay private.
fn module_for(line_width_mm: f64, support_speed: f64, wall_count: i64) -> TreeSupport {
    let mut fields = HashMap::new();
    fields.insert("enable_support".to_owned(), ConfigValue::Bool(true));
    fields.insert(
        "support_speed".to_owned(),
        ConfigValue::Float(support_speed),
    );
    fields.insert("nozzle_diameter".to_owned(), ConfigValue::Float(0.4));
    fields.insert("layer_height".to_owned(), ConfigValue::Float(0.2));
    fields.insert(
        "support_line_width".to_owned(),
        ConfigValue::Float(line_width_mm),
    );
    fields.insert(
        "support_interface_flow".to_owned(),
        ConfigValue::Percent(100.0),
    );
    fields.insert(
        "tree_support_wall_count".to_owned(),
        ConfigValue::Int(wall_count),
    );
    fields.insert(
        "support_interface_spacing".to_owned(),
        ConfigValue::Float(0.4),
    );
    fields.insert(
        "support_bottom_interface_spacing".to_owned(),
        ConfigValue::Float(0.4),
    );
    fields.insert(
        "support_base_pattern_spacing".to_owned(),
        ConfigValue::Float(2.5),
    );
    TreeSupport::from_config(&ConfigView::from_map(fields))
        .expect("every key from_config reads is supplied")
}

/// The caller's speed factor, computed as `TreeSupport::run_support` does;
/// the kernels only propagate it.
fn speed_factor_for(support_speed: f64) -> f32 {
    (support_speed as f32) / BASE_SPEED
}

// ═════════════════════════════════════════════════════════════════════════════
// M-TREE-SCAN
// ═════════════════════════════════════════════════════════════════════════════

const SCAN_ID: &str = "M-TREE-SCAN";
const SCAN_TEST: &str = "property_tests::proptest_tree_support_scan";
const SCAN_BOUND_KEYS: [&str; 7] = [
    "max_abs_units",
    "max_vertices",
    "max_holes",
    "max_components",
    "max_shrink_candidates",
    "max_spacing_units",
    "max_rows",
];

/// The `[0,10] × [0,20]` mm vertical witness needs 20 mm.
const SCAN_WITNESS_ABS_UNITS: i64 = 200_000;
/// The vertical witness pitch, 2.5 mm.
const SCAN_WITNESS_SPACING_UNITS: i64 = 25_000;
/// The plain `[0,10]²` mm witness at 1 mm pitch scans 10 rows.
const SCAN_WITNESS_ROWS: u32 = 10;
/// 10 mm pitch ceiling.
const SCAN_MAX_SPACING_CEILING: i64 = 100_000;
/// One row below a cut, one through it and one above it.
const SCAN_MIN_ROWS: i64 = 3;
/// 8-vertex notch contour, or 4 + 4 for a rectangle with one hole.
const SCAN_CLASS_VERTICES: u32 = 8;
const SCAN_RAW_LEN: usize = 16;

#[derive(Debug, Clone, Copy)]
struct ScanLimits {
    max_abs_units: i64,
    max_vertices: u32,
    max_holes: u32,
    max_components: u32,
    max_shrink_candidates: u32,
    max_spacing_units: i64,
    max_rows: u32,
}

fn checked_scan_limits(bounds: &BTreeMap<String, u64>) -> Result<ScanLimits, DomainError> {
    let limits = ScanLimits {
        max_abs_units: bound_i64(bounds, "max_abs_units")?,
        max_vertices: bound_u32(bounds, "max_vertices")?,
        max_holes: bound_u32(bounds, "max_holes")?,
        max_components: bound_u32(bounds, "max_components")?,
        max_shrink_candidates: bound_u32(bounds, "max_shrink_candidates")?,
        max_spacing_units: bound_i64(bounds, "max_spacing_units")?,
        max_rows: bound_u32(bounds, "max_rows")?,
    };
    if !(SCAN_WITNESS_ABS_UNITS..=VALIDATOR_ABS_CEILING).contains(&limits.max_abs_units) {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units must be in {SCAN_WITNESS_ABS_UNITS}..={VALIDATOR_ABS_CEILING}: the \
             [0,10] x [0,20] mm witness needs 200000 and validate_geometry caps coordinates"
        )));
    }
    if limits.max_spacing_units % SCAN_PITCH_QUANTUM != 0
        || !(SCAN_WITNESS_SPACING_UNITS..=SCAN_MAX_SPACING_CEILING)
            .contains(&limits.max_spacing_units)
    {
        return Err(DomainError::InvalidBounds(format!(
            "max_spacing_units must be a multiple of {SCAN_PITCH_QUANTUM} in \
             {SCAN_WITNESS_SPACING_UNITS}..={SCAN_MAX_SPACING_CEILING} (the 2.5 mm witness pitch)"
        )));
    }
    if !(SCAN_WITNESS_ROWS..=MAX_ROWS_CEILING).contains(&limits.max_rows) {
        return Err(DomainError::InvalidBounds(format!(
            "max_rows must be in {SCAN_WITNESS_ROWS}..={MAX_ROWS_CEILING}"
        )));
    }
    if limits.max_vertices < SCAN_CLASS_VERTICES
        || limits.max_holes < 1
        || limits.max_components < 1
        || limits.max_shrink_candidates < 1
    {
        return Err(DomainError::InvalidBounds(format!(
            "max_vertices >= {SCAN_CLASS_VERTICES}, max_holes >= 1, max_components >= 1 and \
             max_shrink_candidates >= 1 are required by the generated classes"
        )));
    }
    // Every spacing up to the maximum must admit three rows inside the span
    // (`half + rows * spacing` is monotone in the spacing). With the ranges
    // above this always holds (10 mm pitch needs 35 mm, the span is >= 40 mm);
    // it is re-proved here so a future range edit cannot silently break it.
    let span = quantized_span(limits.max_abs_units)?;
    let s = limits.max_spacing_units;
    if s / 2 + SCAN_MIN_ROWS * s > span {
        return Err(DomainError::InvalidBounds(
            "the coordinate span must admit three rows at the maximum spacing".to_owned(),
        ));
    }
    Ok(limits)
}

fn scan_probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 200_000),
        ("max_vertices".to_owned(), 8),
        ("max_holes".to_owned(), 1),
        ("max_components".to_owned(), 1),
        ("max_shrink_candidates".to_owned(), 1),
        ("max_spacing_units".to_owned(), 25_000),
        ("max_rows".to_owned(), 10),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanMode {
    Rect,
    Notch,
    Hole,
    /// A rectangle no wider (along the scan axis) than half a pitch: no
    /// ordinary row exists, so only the mid-scan fallback can emit.
    Fallback,
    Degenerate,
}

const SCAN_MODES: [ScanMode; 5] = [
    ScanMode::Rect,
    ScanMode::Notch,
    ScanMode::Hole,
    ScanMode::Fallback,
    ScanMode::Degenerate,
];

impl ScanMode {
    fn label(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Notch => "notch",
            Self::Hole => "hole",
            Self::Fallback => "fallback",
            Self::Degenerate => "degenerate",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScanDegenerate {
    ZeroScanExtent,
    ZeroSegmentExtent,
    ShortRing,
    EmptyContour,
    ZeroSpacing,
    NegativeSpacing,
}

const SCAN_DEGENERATES: [ScanDegenerate; 6] = [
    ScanDegenerate::ZeroScanExtent,
    ScanDegenerate::ZeroSegmentExtent,
    ScanDegenerate::ShortRing,
    ScanDegenerate::EmptyContour,
    ScanDegenerate::ZeroSpacing,
    ScanDegenerate::NegativeSpacing,
];

/// One bounded axis-aligned scan-fill fixture.
#[derive(Debug, Clone, PartialEq)]
struct ScanCase {
    mode: ScanMode,
    /// Present exactly when `mode == ScanMode::Degenerate`.
    degenerate: Option<ScanDegenerate>,
    vertical: bool,
    /// Outer rectangle in the frame, canonical units.
    outer: Rect,
    /// Hole, or open-toward-`+v` notch `[u0,u1] × [v0, outer.v1]`.
    cut: Option<Rect>,
    /// Intended pitch in canonical units; `spacing_mm` must equal it exactly.
    spacing_units: i64,
    /// The kernel's `spacing` argument, mm.
    spacing_mm: f64,
    /// Configured `support_line_width`, mm.
    line_width_mm: f64,
    /// Configured `support_speed`, mm/s.
    support_speed: f64,
    /// Caller-supplied layer Z, mm.
    z: f32,
}

impl ScanCase {
    fn speed_factor(&self) -> f32 {
        speed_factor_for(self.support_speed)
    }

    fn expoly(&self) -> ExPolygon {
        let vertical = self.vertical;
        let o = self.outer;
        match (self.mode, self.degenerate) {
            (ScanMode::Degenerate, Some(ScanDegenerate::ShortRing)) => ExPolygon {
                contour: uv_ring(vertical, &[(o.u0, o.v0), (o.u1, o.v1)]),
                holes: Vec::new(),
            },
            (ScanMode::Degenerate, Some(ScanDegenerate::EmptyContour)) => ExPolygon {
                contour: Polygon { points: Vec::new() },
                holes: Vec::new(),
            },
            (ScanMode::Notch, _) => {
                let n = self.cut.expect("notch case carries a cut");
                ExPolygon {
                    contour: uv_ring(
                        vertical,
                        &[
                            (o.u0, o.v0),
                            (o.u1, o.v0),
                            (o.u1, o.v1),
                            (n.u1, o.v1),
                            (n.u1, n.v0),
                            (n.u0, n.v0),
                            (n.u0, o.v1),
                            (o.u0, o.v1),
                        ],
                    ),
                    holes: Vec::new(),
                }
            }
            (ScanMode::Hole, _) => ExPolygon {
                contour: o.ccw(vertical),
                holes: vec![self.cut.expect("hole case carries a cut").cw(vertical)],
            },
            _ => ExPolygon {
                contour: o.ccw(vertical),
                holes: Vec::new(),
            },
        }
    }

    fn has_valid_geometry(&self) -> bool {
        !matches!(
            self.degenerate,
            Some(
                ScanDegenerate::ZeroScanExtent
                    | ScanDegenerate::ZeroSegmentExtent
                    | ScanDegenerate::ShortRing
                    | ScanDegenerate::EmptyContour
            )
        )
    }

    /// Material intervals along `u` of the scan line at `v`, by rectangle
    /// arithmetic with the kernel's half-open crossing rule (a ring's low
    /// edge is inclusive, its high edge exclusive).
    fn intervals_at(&self, v: i64) -> Vec<(i64, i64)> {
        let o = self.outer;
        if !(o.v0 <= v && v < o.v1) {
            return Vec::new();
        }
        match (self.mode, self.cut) {
            (ScanMode::Hole, Some(c)) if c.v0 <= v && v < c.v1 => vec![(o.u0, c.u0), (c.u1, o.u1)],
            (ScanMode::Notch, Some(c)) if c.v0 <= v => vec![(o.u0, c.u0), (c.u1, o.u1)],
            _ => vec![(o.u0, o.u1)],
        }
    }

    /// Ordinary rows `v0 + spacing/2 + n*spacing` while `< v1`.
    fn ordinary_rows(&self) -> Result<Vec<i64>, CheckFailure> {
        let s = self.spacing_units;
        if s <= 0 {
            return Ok(Vec::new());
        }
        let mut rows = Vec::new();
        let mut v = self.outer.v0 + s / 2;
        while v < self.outer.v1 {
            rows.push(v);
            v = v
                .checked_add(s)
                .ok_or_else(|| CheckFailure::Fixture("row arithmetic overflow".to_owned()))?;
        }
        Ok(rows)
    }

    /// The complete expected stroke set `(v, u_lo, u_hi)`, sorted, plus
    /// whether it came from the mid-scan fallback.
    fn expected_strokes(&self) -> Result<(Vec<(i64, i64, i64)>, bool), CheckFailure> {
        if self.spacing_units <= 0 || !self.has_valid_geometry() {
            return Ok((Vec::new(), false));
        }
        let mut strokes = Vec::new();
        for v in self.ordinary_rows()? {
            for (a, b) in self.intervals_at(v) {
                strokes.push((v, a, b));
            }
        }
        if !strokes.is_empty() {
            strokes.sort_unstable();
            return Ok((strokes, false));
        }
        // Separate fallback: one scan line at the scan-axis midpoint, emitted
        // only because no ordinary line produced a stroke.
        let sum = self.outer.v0 + self.outer.v1;
        if sum % 2 != 0 {
            return Err(CheckFailure::Fixture(
                "the fallback midline is not on the unit grid".to_owned(),
            ));
        }
        let mid = sum / 2;
        for (a, b) in self.intervals_at(mid) {
            strokes.push((mid, a, b));
        }
        strokes.sort_unstable();
        Ok((strokes, true))
    }

    /// Ordinary rows inside the cut's scan range.
    fn split_rows(&self) -> usize {
        let Some(c) = self.cut else {
            return 0;
        };
        self.ordinary_rows()
            .unwrap_or_default()
            .into_iter()
            .filter(|v| c.v0 <= *v && (self.mode == ScanMode::Notch || *v < c.v1))
            .count()
    }
}

fn run_scan(case: &ScanCase) -> Vec<ExtrusionPath3D> {
    module_for(case.line_width_mm, case.support_speed, 1).scan_fill_region(
        &case.expoly(),
        case.spacing_mm,
        case.z,
        case.speed_factor(),
        case.vertical,
    )
}

#[derive(Debug, Default)]
struct ScanStats {
    strokes: usize,
    segments: usize,
    rows: usize,
    split_rows: usize,
    left: usize,
    right: usize,
    fallback: bool,
}

fn validate_scan_fixture(case: &ScanCase, limits: &ScanLimits) -> Result<(), CheckFailure> {
    let fixture = |m: String| Err(CheckFailure::Fixture(m));
    if (case.mode == ScanMode::Degenerate) != case.degenerate.is_some() {
        return fixture("degenerate tag disagrees with the mode".to_owned());
    }
    if case.spacing_mm * 10_000.0 != case.spacing_units as f64 {
        return fixture(format!(
            "spacing {} mm is not exactly {} units",
            case.spacing_mm, case.spacing_units
        ));
    }
    let spacing_degenerate = matches!(
        case.degenerate,
        Some(ScanDegenerate::ZeroSpacing | ScanDegenerate::NegativeSpacing)
    );
    if !spacing_degenerate
        && (case.spacing_units % SCAN_PITCH_QUANTUM != 0
            || !(SCAN_PITCH_QUANTUM..=limits.max_spacing_units).contains(&case.spacing_units))
    {
        return fixture(format!("spacing {} units", case.spacing_units));
    }
    if !(case.line_width_mm > 0.0 && case.line_width_mm <= 2.0)
        || !(1.0..=300.0).contains(&case.support_speed)
        || !(case.z.is_finite() && case.z > 0.0)
    {
        return fixture("config value outside the manifest range".to_owned());
    }
    let o = case.outer;
    let cut = case
        .cut
        .map_or(Vec::new(), |c| vec![c.u0, c.u1, c.v0, c.v1]);
    for v in [o.u0, o.u1, o.v0, o.v1].into_iter().chain(cut) {
        if v.abs() > limits.max_abs_units || v % GRID != 0 {
            return fixture(format!(
                "coordinate {v} exceeds max_abs_units or is off the 1/8 mm grid"
            ));
        }
    }
    if case.has_valid_geometry() {
        let (class, min_holes) = match case.mode {
            ScanMode::Notch => (TopologyClass::Concave, 0),
            ScanMode::Hole => (TopologyClass::Holed, 1),
            _ => (TopologyClass::Convex, 0),
        };
        validate_geometry(
            &GeometryCase {
                polygons: vec![case.expoly()],
            },
            &geometry_domain(
                class,
                min_holes,
                limits.max_abs_units,
                limits.max_vertices,
                limits.max_holes,
                limits.max_components,
                limits.max_shrink_candidates,
            ),
        )
        .map_err(|error| CheckFailure::Fixture(format!("invalid geometry: {error}")))?;
    }
    Ok(())
}

/// The one scan checker used by the property, the witnesses, the controls and
/// the shrink probe.
fn check_scan(
    case: &ScanCase,
    limits: &ScanLimits,
    paths: &[ExtrusionPath3D],
) -> Result<ScanStats, CheckFailure> {
    validate_scan_fixture(case, limits)?;
    let (expected, fallback) = case.expected_strokes()?;
    let rows: BTreeSet<i64> = expected.iter().map(|s| s.0).collect();
    if rows.len() > limits.max_rows as usize {
        return Err(CheckFailure::Fixture(format!(
            "{} expected rows exceed max_rows {}",
            rows.len(),
            limits.max_rows
        )));
    }
    if expected.is_empty() && !paths.is_empty() {
        return Err(CheckFailure::Intervals(format!(
            "no material/rows are expected but {} path(s) were emitted",
            paths.len()
        )));
    }
    check_metadata(
        paths,
        case.z,
        case.line_width_mm as f32,
        case.speed_factor(),
    )?;
    let region = [case.expoly()];
    let mut stats = ScanStats {
        segments: contained_segments(paths, &region, "scan")?,
        ..ScanStats::default()
    };
    if !expected.is_empty() && paths.is_empty() {
        return Err(CheckFailure::FalseEmpty);
    }
    let actual = stroke_triples(paths, case.vertical)?;
    if actual != expected {
        return Err(CheckFailure::Intervals(first_row_diff(&expected, &actual)));
    }
    stats.strokes = actual.len();
    stats.rows = rows.len();
    stats.split_rows = split_row_count(&actual);
    stats.fallback = fallback;
    if let Some(c) = case.cut {
        let mut per_row: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
        for &(v, a, b) in &actual {
            per_row.entry(v).or_default().push((a, b));
        }
        for intervals in per_row.values().filter(|v| v.len() >= 2) {
            stats.left += intervals.iter().filter(|(_, b)| *b <= c.u0).count();
            stats.right += intervals.iter().filter(|(a, _)| *a >= c.u1).count();
        }
    }
    Ok(stats)
}

fn build_scan_case(
    mode: ScanMode,
    degenerate: ScanDegenerate,
    vertical: bool,
    raw: [u32; SCAN_RAW_LEN],
    l: &ScanLimits,
) -> ScanCase {
    let mq = (l.max_abs_units / GRID) * GRID;
    let span = 2 * mq;
    let s = SCAN_PITCH_QUANTUM * pick(raw[0], 1, l.max_spacing_units / SCAN_PITCH_QUANTUM);
    let half = s / 2;
    let (h, rows) = if mode == ScanMode::Fallback {
        // h <= spacing/2, so `v0 + spacing/2 < v1` never holds.
        (GRID * pick(raw[1], 1, half / GRID), 0)
    } else {
        let row_cap = ((span - half) / s).min(i64::from(l.max_rows));
        let rows = pick(raw[1], SCAN_MIN_ROWS, row_cap);
        // h in (half + (rows-1)*s, half + rows*s]: exactly `rows` rows.
        (
            half + (rows - 1) * s + GRID * pick(raw[2], 1, s / GRID),
            rows,
        )
    };
    let w = GRID * pick(raw[3], 3, span / GRID);
    let u0 = -mq + GRID * pick(raw[4], 0, (span - w) / GRID);
    let v0 = -mq + GRID * pick(raw[5], 0, (span - h) / GRID);
    let mut outer = Rect {
        u0,
        u1: u0 + w,
        v0,
        v1: v0 + h,
    };
    let cut = match mode {
        ScanMode::Notch | ScanMode::Hole => {
            let cu0 = u0 + GRID * pick(raw[6], 1, w / GRID - 2);
            let cu1 = cu0 + GRID * pick(raw[7], 1, (outer.u1 - GRID - cu0) / GRID);
            // Row index k in 1..=rows-2 always crosses the cut; its low edge
            // is exactly on that row (inclusion edge) or strictly below it.
            let k = pick(raw[8], 1, rows - 2);
            let row_k = v0 + half + k * s;
            let cv0 = if raw[9].is_multiple_of(3) {
                row_k - GRID * pick(raw[10], 1, s / GRID - 1)
            } else {
                row_k
            };
            let cv1 = if mode == ScanMode::Notch {
                outer.v1
            } else if raw[11].is_multiple_of(2) {
                // High edge exactly on a later row: the exclusion edge.
                v0 + half + pick(raw[12], k + 1, rows - 1) * s
            } else {
                row_k + GRID * pick(raw[12], 1, (outer.v1 - GRID - row_k) / GRID)
            };
            Some(Rect {
                u0: cu0,
                u1: cu1,
                v0: cv0,
                v1: cv1,
            })
        }
        _ => None,
    };
    let degenerate = (mode == ScanMode::Degenerate).then_some(degenerate);
    let mut spacing_units = s;
    match degenerate {
        Some(ScanDegenerate::ZeroScanExtent) => outer.v1 = outer.v0,
        Some(ScanDegenerate::ZeroSegmentExtent) => outer.u1 = outer.u0,
        Some(ScanDegenerate::ZeroSpacing) => spacing_units = 0,
        Some(ScanDegenerate::NegativeSpacing) => spacing_units = -s,
        Some(ScanDegenerate::ShortRing | ScanDegenerate::EmptyContour) | None => {}
    }
    ScanCase {
        mode,
        degenerate,
        vertical,
        outer,
        cut,
        spacing_units,
        spacing_mm: spacing_units as f64 / 10_000.0,
        // (0, 2] mm in exact 1/64 mm steps.
        line_width_mm: pick(raw[13], 1, 128) as f64 / 64.0,
        support_speed: pick(raw[14], 1, 300) as f64,
        z: pick(raw[15], 1, 400) as f32 * 0.05,
    }
}

fn scan_case_strategy(
    limits: &ScanLimits,
    modes: Vec<ScanMode>,
    degenerates: Vec<ScanDegenerate>,
    orientations: Vec<bool>,
) -> BoxedStrategy<ScanCase> {
    let limits = *limits;
    (
        prop::sample::select(modes).no_shrink(),
        prop::sample::select(degenerates).no_shrink(),
        prop::sample::select(orientations).no_shrink(),
        any::<[u32; SCAN_RAW_LEN]>(),
    )
        .prop_map(move |(mode, degenerate, vertical, raw)| {
            build_scan_case(mode, degenerate, vertical, raw, &limits)
        })
        .boxed()
}

// ── Deterministic scan witnesses ────────────────────────────────────────────

/// A witness given in XY canonical units; `cut_xy` is `[x0,x1] × [y0,y1]`.
fn scan_witness(
    mode: ScanMode,
    vertical: bool,
    outer_xy: (i64, i64, i64, i64),
    cut_xy: Option<(i64, i64, i64, i64)>,
    spacing_units: i64,
) -> ScanCase {
    let rect = |(x0, x1, y0, y1): (i64, i64, i64, i64)| Rect::from_xy(vertical, x0, x1, y0, y1);
    ScanCase {
        mode,
        degenerate: None,
        vertical,
        outer: rect(outer_xy),
        cut: cut_xy.map(rect),
        spacing_units,
        spacing_mm: spacing_units as f64 / 10_000.0,
        line_width_mm: 0.4375,
        support_speed: 45.0,
        z: 0.2,
    }
}

/// `[0,10]²` mm.
const SQUARE_10: (i64, i64, i64, i64) = (0, 100_000, 0, 100_000);
/// 1 mm.
const PITCH_1MM: i64 = 10_000;

/// Charter witness: `[0,10] × [0,20]` mm, vertical, 2.5 mm pitch.
fn vertical_witness() -> ScanCase {
    scan_witness(
        ScanMode::Rect,
        true,
        (0, 100_000, 0, 200_000),
        None,
        SCAN_WITNESS_SPACING_UNITS,
    )
}

/// 0.5 mm square at 2 mm pitch: only the mid-scan fallback can emit.
fn fallback_witness(vertical: bool) -> ScanCase {
    scan_witness(
        ScanMode::Fallback,
        vertical,
        (0, 5_000, 0, 5_000),
        None,
        20_000,
    )
}

/// Asymmetric hole `[2,3] × [4,6]` mm in `[0,10]²`, 1 mm pitch.
fn asymmetric_hole_scan(vertical: bool) -> ScanCase {
    scan_witness(
        ScanMode::Hole,
        vertical,
        SQUARE_10,
        Some((20_000, 30_000, 40_000, 60_000)),
        PITCH_1MM,
    )
}

/// U shape: open-to-top notch `[4,6] × [4,10]` mm, horizontal, 1 mm pitch.
fn u_notch_scan() -> ScanCase {
    scan_witness(
        ScanMode::Notch,
        false,
        SQUARE_10,
        Some((40_000, 60_000, 40_000, 100_000)),
        PITCH_1MM,
    )
}

fn square_scan() -> ScanCase {
    scan_witness(ScanMode::Rect, false, SQUARE_10, None, PITCH_1MM)
}

fn degenerate_scan_witness(kind: ScanDegenerate) -> ScanCase {
    let mut case = square_scan();
    case.mode = ScanMode::Degenerate;
    case.degenerate = Some(kind);
    match kind {
        ScanDegenerate::ZeroScanExtent => case.outer.v1 = case.outer.v0,
        ScanDegenerate::ZeroSegmentExtent => case.outer.u1 = case.outer.u0,
        ScanDegenerate::ZeroSpacing => {
            case.spacing_units = 0;
            case.spacing_mm = 0.0;
        }
        ScanDegenerate::NegativeSpacing => {
            case.spacing_units = -PITCH_1MM;
            case.spacing_mm = -1.0;
        }
        ScanDegenerate::ShortRing | ScanDegenerate::EmptyContour => {}
    }
    case
}

/// Observed strokes on the scan line `v_mm`, as sorted `(u_lo, u_hi)` mm.
fn row_mm(paths: &[ExtrusionPath3D], vertical: bool, v_mm: f32) -> Vec<(f32, f32)> {
    let uv = |p: &Point3WithWidth| if vertical { (p.y, p.x) } else { (p.x, p.y) };
    let mut row: Vec<(f32, f32)> = paths
        .iter()
        .filter(|p| p.points.len() == 2 && uv(&p.points[0]).1 == v_mm)
        .map(|p| {
            let (a, b) = (uv(&p.points[0]).0, uv(&p.points[1]).0);
            (a.min(b), a.max(b))
        })
        .collect();
    row.sort_by(|a, b| a.0.total_cmp(&b.0));
    row
}

#[derive(Debug, Default)]
struct ScanWitnessStats {
    vertical_strokes: u64,
    fallback: u64,
    hole_split: u64,
    notch_split: u64,
    square_rows: u64,
    empty_modes: u64,
}

/// Every deterministic positive and empty scan witness, through the checker.
fn scan_witness_observations(limits: &ScanLimits) -> Result<ScanWitnessStats, String> {
    let mut out = ScanWitnessStats::default();
    let checked = |case: &ScanCase| -> Result<(Vec<ExtrusionPath3D>, ScanStats), String> {
        let paths = run_scan(case);
        let stats = check_scan(case, limits, &paths)
            .map_err(|e| format!("{} scan witness: {e}", case.mode.label()))?;
        Ok((paths, stats))
    };

    // Charter vertical witness: four strokes at the listed x, each y [0,20].
    let (paths, stats) = checked(&vertical_witness())?;
    let mut strokes: Vec<(f32, f32, f32)> = paths
        .iter()
        .filter(|p| p.points[0].x == p.points[1].x)
        .map(|p| {
            let (a, b) = (p.points[0].y, p.points[1].y);
            (p.points[0].x, a.min(b), a.max(b))
        })
        .collect();
    strokes.sort_by(|a, b| a.0.total_cmp(&b.0));
    let want = vec![
        (1.25, 0.0, 20.0),
        (3.75, 0.0, 20.0),
        (6.25, 0.0, 20.0),
        (8.75, 0.0, 20.0),
    ];
    if paths.len() != 4 || strokes != want || stats.fallback {
        return Err(format!(
            "vertical [0,10]x[0,20] mm witness: expected {want:?}, observed {strokes:?}"
        ));
    }
    out.vertical_strokes += stats.strokes as u64;

    for vertical in [false, true] {
        let (paths, stats) = checked(&fallback_witness(vertical))?;
        if !stats.fallback || row_mm(&paths, vertical, 0.25) != vec![(0.0, 0.5)] {
            return Err(format!(
                "fallback witness (vertical={vertical}) lacks the 0.25 mm center line"
            ));
        }
        out.fallback += 1;
    }

    let (paths, stats) = checked(&asymmetric_hole_scan(false))?;
    if row_mm(&paths, false, 4.5) != vec![(0.0, 2.0), (3.0, 10.0)] || stats.split_rows != 2 {
        return Err("asymmetric hole rows y=4.5,5.5 mm are not split".to_owned());
    }
    out.hole_split += stats.split_rows as u64;
    let (paths, stats) = checked(&asymmetric_hole_scan(true))?;
    if row_mm(&paths, true, 2.5) != vec![(0.0, 4.0), (6.0, 10.0)] || stats.split_rows != 1 {
        return Err("vertical asymmetric hole row x=2.5 mm is not split".to_owned());
    }
    out.hole_split += stats.split_rows as u64;

    let (paths, stats) = checked(&u_notch_scan())?;
    if row_mm(&paths, false, 5.5) != vec![(0.0, 4.0), (6.0, 10.0)] || stats.split_rows != 6 {
        return Err("U notch rows y=4.5..9.5 mm are not split".to_owned());
    }
    out.notch_split += stats.split_rows as u64;

    let (_, stats) = checked(&square_scan())?;
    if stats.rows != SCAN_WITNESS_ROWS as usize || stats.strokes != SCAN_WITNESS_ROWS as usize {
        return Err(format!("square witness scanned {} rows", stats.rows));
    }
    out.square_rows += stats.rows as u64;

    for kind in SCAN_DEGENERATES {
        let (paths, _) = checked(&degenerate_scan_witness(kind))?;
        if !paths.is_empty() {
            return Err(format!("{kind:?} emitted output"));
        }
        out.empty_modes += 1;
    }
    Ok(out)
}

#[test]
fn proptest_tree_support_scan() {
    let run = PropertyRun::new(SCAN_ID, SCAN_TEST).expect("M-TREE-SCAN context");
    let limits = checked_scan_limits(&read_live_bounds(&run, &SCAN_BOUND_KEYS))
        .expect("M-TREE-SCAN bound consistency");
    let strategy = scan_case_strategy(
        &limits,
        SCAN_MODES.to_vec(),
        SCAN_DEGENERATES.to_vec(),
        vec![false, true],
    );

    let calls = Cell::new(0_u64);
    let w_vertical = Cell::new(0_u64);
    let w_fallback = Cell::new(0_u64);
    let w_hole = Cell::new(0_u64);
    let w_notch = Cell::new(0_u64);
    let w_square = Cell::new(0_u64);
    let w_empty = Cell::new(0_u64);
    let strokes = Cell::new(0_u64);
    let segments = Cell::new(0_u64);
    let split_rows = Cell::new(0_u64);
    let vertical_cases = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 5] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        // Deterministic positives, constructed inside the callback.
        let w = match scan_witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        w_vertical.set(w_vertical.get() + w.vertical_strokes);
        w_fallback.set(w_fallback.get() + w.fallback);
        w_hole.set(w_hole.get() + w.hole_split);
        w_notch.set(w_notch.get() + w.notch_split);
        w_square.set(w_square.get() + w.square_rows);
        w_empty.set(w_empty.get() + w.empty_modes);

        let paths = run_scan(&case);
        let stats = match check_scan(&case, &limits, &paths) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{} case failed: {e}", case.mode.label())),
        };
        match case.mode {
            ScanMode::Notch | ScanMode::Hole => {
                if stats.split_rows == 0 || stats.left == 0 || stats.right == 0 {
                    return fail(format!(
                        "{} case lost its split row (split={}, left={}, right={})",
                        case.mode.label(),
                        stats.split_rows,
                        stats.left,
                        stats.right
                    ));
                }
            }
            ScanMode::Rect => {
                if stats.strokes == 0 || stats.fallback {
                    return fail("rect case produced no ordinary strokes");
                }
            }
            ScanMode::Fallback => {
                if !stats.fallback || stats.strokes != 1 {
                    return fail("fallback case did not emit exactly its center line");
                }
            }
            ScanMode::Degenerate => {
                if !paths.is_empty() {
                    return fail("degenerate case produced output");
                }
            }
        }
        let slot = SCAN_MODES
            .iter()
            .position(|m| *m == case.mode)
            .expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        if case.vertical {
            vertical_cases.set(vertical_cases.get() + 1);
        }
        strokes.set(strokes.get() + stats.strokes as u64);
        segments.set(segments.get() + stats.segments as u64);
        split_rows.set(split_rows.get() + stats.split_rows as u64);
        Ok(())
    })
    .expect("M-TREE-SCAN complete rows, whole-segment containment and metadata");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(w_vertical.get() > 0, "vertical 10x20 mm witness unobserved");
    assert!(w_fallback.get() > 0, "mid-scan fallback witness unobserved");
    assert!(w_hole.get() > 0, "hole split rows unobserved");
    assert!(w_notch.get() > 0, "notch split rows unobserved");
    assert!(w_square.get() > 0, "square rows unobserved");
    assert!(w_empty.get() > 0, "degenerate empty modes unobserved");
    println!(
        "M-TREE-SCAN accepted_calls={} rect={} notch={} hole={} fallback={} degenerate={} \
         vertical_cases={} strokes={} segments={} split_rows={} witness_vertical_strokes={} \
         witness_fallback={} witness_hole_split={} witness_notch_split={} \
         witness_square_rows={} witness_empty={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        per_mode[4].get(),
        vertical_cases.get(),
        strokes.get(),
        segments.get(),
        split_rows.get(),
        w_vertical.get(),
        w_fallback.get(),
        w_hole.get(),
        w_notch.get(),
        w_square.get(),
        w_empty.get()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// M-TREE-RENDER
// ═════════════════════════════════════════════════════════════════════════════

const RENDER_ID: &str = "M-TREE-RENDER";
const RENDER_TEST: &str = "property_tests::proptest_tree_support_walls_and_fill";
const RENDER_BOUND_KEYS: [&str; 8] = [
    "max_abs_units",
    "max_vertices",
    "max_holes",
    "max_components",
    "max_shrink_candidates",
    "max_width_units",
    "max_walls",
    "max_rows",
];

/// The `[0,10]²` mm witnesses need 10 mm.
const RENDER_WITNESS_ABS_UNITS: i64 = 100_000;
/// The charter line width, 0.5 mm.
const RENDER_WITNESS_WIDTH_UNITS: i64 = 5_000;
/// 2 mm line-width ceiling.
const RENDER_MAX_WIDTH_CEILING: i64 = 20_000;
/// The witnesses use two walls.
const RENDER_WITNESS_WALLS: u32 = 2;
const RENDER_MAX_WALLS_CEILING: u32 = 8;
/// The zero-wall `[0,10]²` witness at 1 mm pitch scans 10 rows.
const RENDER_WITNESS_ROWS: u32 = 10;
/// Fill-spacing steps of `RENDER_QUANTUM`: (0, 4] mm.
const RENDER_FS_STEPS: i64 = 8;
const RENDER_RAW_LEN: usize = 16;

#[derive(Debug, Clone, Copy)]
struct RenderLimits {
    max_abs_units: i64,
    max_vertices: u32,
    max_holes: u32,
    max_components: u32,
    max_shrink_candidates: u32,
    max_width_units: i64,
    max_walls: u32,
    max_rows: u32,
}

/// Largest fill row count whose rows fit in `span` around `2 * d_total` of
/// walls: `e_v <= half + rows*s <= span - 2*d_total`.
fn render_row_cap(span: i64, d_total: i64, s: i64, max_rows: u32) -> i64 {
    let room = span - 2 * d_total - s / 2 - s;
    if room < 0 {
        return 0;
    }
    (room / s + 1).min(i64::from(max_rows))
}

/// Smallest row count a holed roomy scene needs for a split row.
fn render_holed_min_rows(d_total: i64, s: i64) -> i64 {
    1 + ceil_div(2 * d_total + 2 * TICK, s)
}

fn checked_render_limits(bounds: &BTreeMap<String, u64>) -> Result<RenderLimits, DomainError> {
    let limits = RenderLimits {
        max_abs_units: bound_i64(bounds, "max_abs_units")?,
        max_vertices: bound_u32(bounds, "max_vertices")?,
        max_holes: bound_u32(bounds, "max_holes")?,
        max_components: bound_u32(bounds, "max_components")?,
        max_shrink_candidates: bound_u32(bounds, "max_shrink_candidates")?,
        max_width_units: bound_i64(bounds, "max_width_units")?,
        max_walls: bound_u32(bounds, "max_walls")?,
        max_rows: bound_u32(bounds, "max_rows")?,
    };
    if !(RENDER_WITNESS_ABS_UNITS..=VALIDATOR_ABS_CEILING).contains(&limits.max_abs_units) {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units must be in {RENDER_WITNESS_ABS_UNITS}..={VALIDATOR_ABS_CEILING}"
        )));
    }
    if limits.max_width_units % RENDER_QUANTUM != 0
        || !(RENDER_WITNESS_WIDTH_UNITS..=RENDER_MAX_WIDTH_CEILING)
            .contains(&limits.max_width_units)
    {
        return Err(DomainError::InvalidBounds(format!(
            "max_width_units must be a multiple of {RENDER_QUANTUM} in \
             {RENDER_WITNESS_WIDTH_UNITS}..={RENDER_MAX_WIDTH_CEILING}"
        )));
    }
    if !(RENDER_WITNESS_WALLS..=RENDER_MAX_WALLS_CEILING).contains(&limits.max_walls) {
        return Err(DomainError::InvalidBounds(format!(
            "max_walls must be in {RENDER_WITNESS_WALLS}..={RENDER_MAX_WALLS_CEILING}"
        )));
    }
    if !(RENDER_WITNESS_ROWS..=MAX_ROWS_CEILING).contains(&limits.max_rows) {
        return Err(DomainError::InvalidBounds(format!(
            "max_rows must be in {RENDER_WITNESS_ROWS}..={MAX_ROWS_CEILING}"
        )));
    }
    if limits.max_vertices < SCAN_CLASS_VERTICES
        || limits.max_holes < 1
        || limits.max_components < 1
        || limits.max_shrink_candidates < 1
    {
        return Err(DomainError::InvalidBounds(format!(
            "max_vertices >= {SCAN_CLASS_VERTICES}, max_holes >= 1, max_components >= 1 and \
             max_shrink_candidates >= 1 are required by the generated classes"
        )));
    }
    // Exhaustively prove every generator parameter combination feasible
    // before any allocation: rows fit, a holed scene's split row fits, and
    // every side fits inside the coordinate span.
    let span = quantized_span(limits.max_abs_units)?;
    for lw_step in 1..=limits.max_width_units / RENDER_QUANTUM {
        let lw = RENDER_QUANTUM * lw_step;
        for walls in 0..=i64::from(limits.max_walls) {
            let d_total = lw * walls;
            for fs_step in 1..=RENDER_FS_STEPS {
                let s = (RENDER_QUANTUM * fs_step).max(lw);
                let cap = render_row_cap(span, d_total, s, limits.max_rows);
                let holed_u = 4 * d_total + 34 * TICK;
                let plain_u = 2 * d_total + 64 * TICK;
                let collapse = 2 * d_total + 2 * s + GRID;
                if cap < render_holed_min_rows(d_total, s)
                    || holed_u > span
                    || plain_u > span
                    || collapse > span
                {
                    return Err(DomainError::InvalidBounds(format!(
                        "line width {lw}, {walls} walls and pitch {s} units do not fit the \
                         coordinate span / max_rows"
                    )));
                }
            }
        }
    }
    Ok(limits)
}

fn render_probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 100_000),
        ("max_vertices".to_owned(), 8),
        ("max_holes".to_owned(), 1),
        ("max_components".to_owned(), 1),
        ("max_shrink_candidates".to_owned(), 1),
        ("max_width_units".to_owned(), 5_000),
        ("max_walls".to_owned(), 2),
        ("max_rows".to_owned(), 10),
    ])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderMode {
    /// Roomy rectangle, production-supported positive wall count.
    Walls,
    /// Roomy holed rectangle, production-supported positive wall count.
    HoledWalls,
    /// Private-kernel zero walls: fill covers the exact source.
    ZeroWall,
    /// Private-kernel zero walls on a holed rectangle.
    ZeroWallHoled,
    /// Small rectangle whose walls/fill may collapse; possibly empty.
    Collapse,
}

const RENDER_MODES: [RenderMode; 5] = [
    RenderMode::Walls,
    RenderMode::HoledWalls,
    RenderMode::ZeroWall,
    RenderMode::ZeroWallHoled,
    RenderMode::Collapse,
];

impl RenderMode {
    fn label(self) -> &'static str {
        match self {
            Self::Walls => "walls",
            Self::HoledWalls => "holed-walls",
            Self::ZeroWall => "zero-wall",
            Self::ZeroWallHoled => "zero-wall-holed",
            Self::Collapse => "collapse",
        }
    }

    fn holed(self) -> bool {
        matches!(self, Self::HoledWalls | Self::ZeroWallHoled)
    }

    fn zero_wall(self) -> bool {
        matches!(self, Self::ZeroWall | Self::ZeroWallHoled)
    }
}

/// One bounded axis-aligned render fixture.
#[derive(Debug, Clone, PartialEq)]
struct RenderCase {
    mode: RenderMode,
    vertical: bool,
    outer: Rect,
    hole: Option<Rect>,
    walls: usize,
    line_width_units: i64,
    fill_spacing_units: i64,
    support_speed: f64,
    z: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BandKind {
    Outer,
    Hole,
}

/// One analytic wall-center loop: the outer contour inset, or the hole grown,
/// by `line_width * (index + 0.5)`.
#[derive(Debug, Clone, Copy)]
struct Band {
    kind: BandKind,
    index: usize,
    rect: Rect,
}

impl Band {
    /// The closed allowance annulus around the band's center rectangle.
    fn annulus(&self, vertical: bool, a: i64) -> ExPolygon {
        let inner = self.rect.grow(-a);
        ExPolygon {
            contour: self.rect.grow(a).ccw(vertical),
            holes: if inner.is_proper() {
                vec![inner.cw(vertical)]
            } else {
                Vec::new()
            },
        }
    }
}

/// The analytic fill region (outer inset and hole grown by the total wall
/// width) and its complete expected stroke set.
#[derive(Debug, Clone)]
struct FillModel {
    outer: Rect,
    hole: Option<Rect>,
    strokes: Vec<(i64, i64, i64)>,
    fallback: bool,
}

impl FillModel {
    fn region(&self, vertical: bool, a: i64) -> ExPolygon {
        ExPolygon {
            contour: self.outer.grow(a).ccw(vertical),
            holes: self
                .hole
                .map(|h| h.grow(-a))
                .filter(|h| h.is_proper())
                .map(|h| vec![h.cw(vertical)])
                .unwrap_or_default(),
        }
    }
}

impl RenderCase {
    fn speed_factor(&self) -> f32 {
        speed_factor_for(self.support_speed)
    }

    fn line_width_mm(&self) -> f32 {
        units_to_mm(self.line_width_units)
    }

    fn fill_spacing_mm(&self) -> f32 {
        units_to_mm(self.fill_spacing_units)
    }

    /// Clipper-derived coordinates carry `CLIPPER_ALLOW`; the zero-wall
    /// kernel never offsets and is exact.
    fn allowance(&self) -> i64 {
        if self.walls == 0 {
            0
        } else {
            CLIPPER_ALLOW
        }
    }

    fn expoly(&self) -> ExPolygon {
        ExPolygon {
            contour: self.outer.ccw(self.vertical),
            holes: self
                .hole
                .map(|h| vec![h.cw(self.vertical)])
                .unwrap_or_default(),
        }
    }

    /// Analytic wall bands from `-line_width * (i + 0.5)`; collapse is
    /// predicted from the inset width, never observed from Clipper.
    fn expected_bands(&self) -> Result<Vec<Band>, CheckFailure> {
        let a = self.allowance();
        let mut bands = Vec::new();
        for index in 0..self.walls {
            let d = self.line_width_units * (2 * index as i64 + 1) / 2;
            let inset = self.outer.grow(-d);
            let (w, h) = (inset.u1 - inset.u0, inset.v1 - inset.v0);
            if w.abs() <= 2 * a || h.abs() <= 2 * a {
                return Err(CheckFailure::Fixture(format!(
                    "wall {index} inset {w} x {h} units is within the allowance of collapse"
                )));
            }
            if w <= 0 || h <= 0 {
                continue;
            }
            bands.push(Band {
                kind: BandKind::Outer,
                index,
                rect: inset,
            });
            if let Some(hole) = self.hole {
                let grown = hole.grow(d);
                if !grown.strictly_inside(inset, 2 * a) {
                    return Err(CheckFailure::Fixture(format!(
                        "hole band {index} is not strictly inside its outer band"
                    )));
                }
                bands.push(Band {
                    kind: BandKind::Hole,
                    index,
                    rect: grown,
                });
            }
        }
        Ok(bands)
    }

    /// Analytic fill model from `-line_width * wall_count`, with the
    /// kernel's `spacing.max(line_width)` pitch and the scan rows/fallback.
    fn fill_model(&self) -> Result<Option<FillModel>, CheckFailure> {
        let a = self.allowance();
        let d_total = self.line_width_units * self.walls as i64;
        let outer = self.outer.grow(-d_total);
        let (w, h) = (outer.u1 - outer.u0, outer.v1 - outer.v0);
        if self.walls > 0 && (w.abs() <= 2 * a || h.abs() <= 2 * a) {
            return Err(CheckFailure::Fixture(
                "the fill region is within the allowance of collapse".to_owned(),
            ));
        }
        if w <= 0 || h <= 0 {
            return Ok(None);
        }
        let hole = self.hole.map(|h| h.grow(d_total));
        if let Some(fh) = hole {
            if !fh.strictly_inside(outer, 2 * a) {
                return Err(CheckFailure::Fixture(
                    "the grown hole is not strictly inside the fill region".to_owned(),
                ));
            }
        }
        let s = self.fill_spacing_units.max(self.line_width_units);
        let near = |x: i64, y: i64| (x - y).abs() <= 2 * a;
        let intervals = |v: i64| -> Result<Vec<(i64, i64)>, CheckFailure> {
            match hole {
                Some(fh) if near(v, fh.v0) || near(v, fh.v1) => Err(CheckFailure::Fixture(
                    format!("fill row {v} is within the allowance of a hole edge"),
                )),
                Some(fh) if fh.v0 <= v && v < fh.v1 => {
                    Ok(vec![(outer.u0, fh.u0), (fh.u1, outer.u1)])
                }
                _ => Ok(vec![(outer.u0, outer.u1)]),
            }
        };
        let mut strokes = Vec::new();
        let mut v = outer.v0 + s / 2;
        loop {
            if near(v, outer.v1) {
                return Err(CheckFailure::Fixture(format!(
                    "fill row {v} is within the allowance of the fill edge {}",
                    outer.v1
                )));
            }
            if v >= outer.v1 {
                break;
            }
            for (lo, hi) in intervals(v)? {
                strokes.push((v, lo, hi));
            }
            v += s;
        }
        let fallback = strokes.is_empty();
        if fallback {
            let sum = outer.v0 + outer.v1;
            if a == 0 && sum % 2 != 0 {
                return Err(CheckFailure::Fixture(
                    "the exact fallback midline is not on the unit grid".to_owned(),
                ));
            }
            let mid = sum.div_euclid(2);
            for (lo, hi) in intervals(mid)? {
                strokes.push((mid, lo, hi));
            }
        }
        strokes.sort_unstable();
        Ok(Some(FillModel {
            outer,
            hole,
            strokes,
            fallback,
        }))
    }
}

/// Build the module and call the kernel the way each mode is defined: a
/// positive count goes through `from_config` and `self.wall_count` (the
/// production path; extra skeleton walls are zero here), zero is passed to
/// the private kernel directly because `from_config` clamps it to one.
fn run_render(case: &RenderCase) -> Result<Vec<ExtrusionPath3D>, String> {
    let configured = if case.walls == 0 {
        1
    } else {
        case.walls as i64
    };
    let module = module_for(
        f64::from(case.line_width_mm()),
        case.support_speed,
        configured,
    );
    let wall_count = if case.walls == 0 {
        0
    } else {
        if module.wall_count != case.walls {
            return Err(format!(
                "from_config produced wall_count {} for a configured {}",
                module.wall_count, case.walls
            ));
        }
        module.wall_count
    };
    Ok(module.render_polygon_with_wall_count(
        &case.expoly(),
        case.z,
        case.speed_factor(),
        case.fill_spacing_mm(),
        case.vertical,
        wall_count,
    ))
}

#[derive(Debug, Default)]
struct RenderStats {
    outer_walls: usize,
    hole_walls: usize,
    fill_strokes: usize,
    split_rows: usize,
    segments: usize,
    fallback: bool,
}

fn validate_render_fixture(case: &RenderCase, limits: &RenderLimits) -> Result<(), CheckFailure> {
    let fixture = |m: String| Err(CheckFailure::Fixture(m));
    if case.mode.zero_wall() != (case.walls == 0) || case.walls > limits.max_walls as usize {
        return fixture(format!("{} walls for mode {:?}", case.walls, case.mode));
    }
    if case.mode.holed() != case.hole.is_some() {
        return fixture("hole presence disagrees with the mode".to_owned());
    }
    if case.line_width_units % RENDER_QUANTUM != 0
        || !(RENDER_QUANTUM..=limits.max_width_units).contains(&case.line_width_units)
        || case.fill_spacing_units % RENDER_QUANTUM != 0
        || case.fill_spacing_units <= 0
    {
        return fixture(format!(
            "line width {} / fill spacing {} units off the 1/2 mm quantum",
            case.line_width_units, case.fill_spacing_units
        ));
    }
    if !(1.0..=300.0).contains(&case.support_speed) || !(case.z.is_finite() && case.z > 0.0) {
        return fixture("config value outside the manifest range".to_owned());
    }
    let o = case.outer;
    let hole = case
        .hole
        .map_or(Vec::new(), |h| vec![h.u0, h.u1, h.v0, h.v1]);
    for v in [o.u0, o.u1, o.v0, o.v1].into_iter().chain(hole) {
        if v.abs() > limits.max_abs_units || v % TICK != 0 {
            return fixture(format!(
                "coordinate {v} exceeds max_abs_units or is off the 1/16 mm grid"
            ));
        }
    }
    let (class, min_holes) = if case.hole.is_some() {
        (TopologyClass::Holed, 1)
    } else {
        (TopologyClass::Convex, 0)
    };
    validate_geometry(
        &GeometryCase {
            polygons: vec![case.expoly()],
        },
        &geometry_domain(
            class,
            min_holes,
            limits.max_abs_units,
            limits.max_vertices,
            limits.max_holes,
            limits.max_components,
            limits.max_shrink_candidates,
        ),
    )
    .map_err(|error| CheckFailure::Fixture(format!("invalid geometry: {error}")))
}

/// The one render checker used by the property, the witnesses, the controls
/// and the shrink probe.
fn check_render(
    case: &RenderCase,
    limits: &RenderLimits,
    paths: &[ExtrusionPath3D],
) -> Result<RenderStats, CheckFailure> {
    validate_render_fixture(case, limits)?;
    let a = case.allowance();
    let vertical = case.vertical;
    let bands = case.expected_bands()?;
    let fill = case.fill_model()?;
    if case.mode != RenderMode::Collapse {
        let per_wall = if case.hole.is_some() { 2 } else { 1 };
        if bands.len() != case.walls * per_wall || fill.is_none() {
            return Err(CheckFailure::Fixture(format!(
                "{} scene is not roomy: {} bands, fill {}",
                case.mode.label(),
                bands.len(),
                fill.is_some()
            )));
        }
    }
    if let Some(f) = &fill {
        let rows: BTreeSet<i64> = f.strokes.iter().map(|s| s.0).collect();
        if rows.len() > limits.max_rows as usize {
            return Err(CheckFailure::Fixture(format!(
                "{} fill rows exceed max_rows {}",
                rows.len(),
                limits.max_rows
            )));
        }
    }
    check_metadata(paths, case.z, case.line_width_mm(), case.speed_factor())?;

    let mut walls = Vec::new();
    let mut fills = Vec::new();
    for path in paths {
        let n = path.points.len();
        let closed = n >= 4
            && path.points[0].x.to_bits() == path.points[n - 1].x.to_bits()
            && path.points[0].y.to_bits() == path.points[n - 1].y.to_bits();
        if closed {
            walls.push(path);
        } else if n == 2 {
            fills.push(path);
        } else {
            return Err(CheckFailure::WallBands(format!(
                "a {n}-point path is neither a closed wall loop nor a two-point fill stroke"
            )));
        }
    }

    let mut stats = RenderStats::default();
    match &fill {
        None if !fills.is_empty() => {
            return Err(CheckFailure::Containment(format!(
                "{} fill stroke(s) emitted but the analytic fill region is empty",
                fills.len()
            )));
        }
        None => {}
        Some(f) => {
            let region = [f.region(vertical, a)];
            stats.segments += contained_segments(fills.iter().copied(), &region, "fill")?;
        }
    }
    if (!bands.is_empty() || fill.is_some()) && paths.is_empty() {
        return Err(CheckFailure::FalseEmpty);
    }

    let mut hits = vec![0_usize; bands.len()];
    for wall in &walls {
        let pts = wall
            .points
            .iter()
            .map(point_xy)
            .collect::<Result<Vec<_>, _>>()?;
        let uv = wall
            .points
            .iter()
            .map(|p| point_uv(vertical, p))
            .collect::<Result<Vec<_>, _>>()?;
        let bbox = Rect {
            u0: uv.iter().map(|p| p.0).min().expect("nonempty loop"),
            u1: uv.iter().map(|p| p.0).max().expect("nonempty loop"),
            v0: uv.iter().map(|p| p.1).min().expect("nonempty loop"),
            v1: uv.iter().map(|p| p.1).max().expect("nonempty loop"),
        };
        let mut matched = None;
        for (i, band) in bands.iter().enumerate() {
            let r = band.rect;
            let close = (bbox.u0 - r.u0).abs() <= a
                && (bbox.u1 - r.u1).abs() <= a
                && (bbox.v0 - r.v0).abs() <= a
                && (bbox.v1 - r.v1).abs() <= a;
            if !close {
                continue;
            }
            let annulus = [band.annulus(vertical, a)];
            let mut inside = true;
            for w in pts.windows(2) {
                inside &= segment_in_region([w[0], w[1]], &annulus, BoundaryRule::Inclusive)
                    .map_err(|e| CheckFailure::Fixture(format!("containment predicate: {e}")))?;
            }
            if inside {
                matched = Some(i);
                break;
            }
        }
        let Some(i) = matched else {
            return Err(CheckFailure::WallBands(format!(
                "wall loop with frame bbox {bbox:?} matches no analytic band"
            )));
        };
        hits[i] += 1;
        stats.segments += pts.len() - 1;
    }
    for (band, &hit) in bands.iter().zip(&hits) {
        if hit != 1 {
            return Err(CheckFailure::WallBands(format!(
                "{:?} band {} ({:?}) observed {hit} times, expected once",
                band.kind, band.index, band.rect
            )));
        }
    }
    stats.outer_walls = bands.iter().filter(|b| b.kind == BandKind::Outer).count();
    stats.hole_walls = bands.iter().filter(|b| b.kind == BandKind::Hole).count();

    let actual = stroke_triples(fills.iter().copied(), vertical)?;
    let expected = fill.as_ref().map(|f| f.strokes.clone()).unwrap_or_default();
    if actual.len() != expected.len() {
        return Err(CheckFailure::Intervals(format!(
            "{} fill strokes observed, {} expected ({})",
            actual.len(),
            expected.len(),
            first_row_diff(&expected, &actual)
        )));
    }
    for (o, e) in actual.iter().zip(&expected) {
        if (o.0 - e.0).abs() > a || (o.1 - e.1).abs() > a || (o.2 - e.2).abs() > a {
            return Err(CheckFailure::Intervals(format!(
                "fill stroke {o:?} differs from the analytic {e:?} by more than {a} units"
            )));
        }
    }
    stats.fill_strokes = actual.len();
    stats.split_rows = split_row_count(&actual);
    stats.fallback = fill.as_ref().is_some_and(|f| f.fallback);
    Ok(stats)
}

fn build_render_case(
    mode: RenderMode,
    vertical: bool,
    raw: [u32; RENDER_RAW_LEN],
    l: &RenderLimits,
) -> RenderCase {
    let mq = (l.max_abs_units / GRID) * GRID;
    let span = 2 * mq;
    let lw = RENDER_QUANTUM * pick(raw[0], 1, l.max_width_units / RENDER_QUANTUM);
    let walls = if mode.zero_wall() {
        0
    } else {
        pick(raw[1], 1, i64::from(l.max_walls))
    };
    let d_total = lw * walls;
    let fs = RENDER_QUANTUM * pick(raw[2], 1, RENDER_FS_STEPS);
    let s = fs.max(lw);
    let half = s / 2;

    // Sides and the hole are first built relative to the fill region's
    // low corner `(fu0, fv0)`, then translated.
    let (side_u, side_v, hole_rel) = if mode == RenderMode::Collapse {
        // Odd multiples of 1/8 mm never equal a 1/2 mm-quantum inset, so
        // every collapse decision and row count is unambiguous.
        let cap = (2 * d_total + 2 * s) / (2 * GRID);
        let side_u = GRID * (2 * pick(raw[3], 0, cap) + 1);
        let side_v = GRID * (2 * pick(raw[4], 0, cap) + 1);
        (side_u, side_v, None)
    } else {
        let r_min = if mode.holed() {
            render_holed_min_rows(d_total, s)
        } else {
            1
        };
        let rows = pick(raw[3], r_min, render_row_cap(span, d_total, s, l.max_rows));
        // Odd multiple of 1/8 mm in (0, s): the fill edge stays >= 1/8 mm
        // from every row.
        let d = GRID * (2 * pick(raw[4], 0, s / (2 * GRID) - 1) + 1);
        let e_v = half + (rows - 1) * s + d;
        if mode.holed() {
            // Fill-hole low edge one or three ticks below a chosen row (odd
            // 1/16 mm, so never on the 1/8 mm row grid).
            let k_max = (rows - 1).min((e_v - half - 2 * d_total - 2 * TICK) / s);
            let r_k = half + pick(raw[8], 0, k_max) * s;
            let fhv0 = r_k - TICK * (1 + 2 * pick(raw[9], 0, 1));
            let lo = (r_k + TICK).max(fhv0 + 2 * d_total + 2 * TICK);
            let fhv1 = lo + 2 * TICK * pick(raw[10], 0, (e_v - TICK - lo) / (2 * TICK));
            let gl = TICK * pick(raw[5], 1, 8);
            let core = 2 * d_total + 2 * TICK + TICK * pick(raw[6], 0, 16);
            let gr = TICK * pick(raw[7], 1, 8);
            let e_u = gl + core + gr;
            // Hole = fill-hole shrunk back by the wall width.
            let hole = Rect {
                u0: gl + d_total,
                u1: gl + core - d_total,
                v0: fhv0 + d_total,
                v1: fhv1 - d_total,
            };
            (e_u + 2 * d_total, e_v + 2 * d_total, Some(hole))
        } else {
            let e_u = TICK * pick(raw[5], 2, 64);
            (e_u + 2 * d_total, e_v + 2 * d_total, None)
        }
    };
    let u0 = -mq + GRID * pick(raw[11], 0, (span - side_u) / GRID);
    let v0 = -mq + GRID * pick(raw[12], 0, (span - side_v) / GRID);
    let outer = Rect {
        u0,
        u1: u0 + side_u,
        v0,
        v1: v0 + side_v,
    };
    // `(fu0, fv0)` = outer low corner + the wall width.
    let (fu0, fv0) = (u0 + d_total, v0 + d_total);
    let hole = hole_rel.map(|h| Rect {
        u0: fu0 + h.u0,
        u1: fu0 + h.u1,
        v0: fv0 + h.v0,
        v1: fv0 + h.v1,
    });
    RenderCase {
        mode,
        vertical,
        outer,
        hole,
        walls: walls as usize,
        line_width_units: lw,
        fill_spacing_units: fs,
        support_speed: pick(raw[13], 1, 300) as f64,
        z: pick(raw[14], 1, 400) as f32 * 0.05,
    }
}

fn render_case_strategy(
    limits: &RenderLimits,
    modes: Vec<RenderMode>,
    orientations: Vec<bool>,
) -> BoxedStrategy<RenderCase> {
    let limits = *limits;
    (
        prop::sample::select(modes).no_shrink(),
        prop::sample::select(orientations).no_shrink(),
        any::<[u32; RENDER_RAW_LEN]>(),
    )
        .prop_map(move |(mode, vertical, raw)| build_render_case(mode, vertical, raw, &limits))
        .boxed()
}

// ── Deterministic render witnesses ──────────────────────────────────────────

fn render_witness(
    mode: RenderMode,
    vertical: bool,
    outer_xy: (i64, i64, i64, i64),
    hole_xy: Option<(i64, i64, i64, i64)>,
    walls: usize,
    fill_spacing_units: i64,
) -> RenderCase {
    let rect = |(x0, x1, y0, y1): (i64, i64, i64, i64)| Rect::from_xy(vertical, x0, x1, y0, y1);
    RenderCase {
        mode,
        vertical,
        outer: rect(outer_xy),
        hole: hole_xy.map(rect),
        walls,
        line_width_units: RENDER_WITNESS_WIDTH_UNITS,
        fill_spacing_units,
        support_speed: 45.0,
        z: 0.2,
    }
}

/// `[0,10]²` mm, width 0.5, two walls: bands `[0.25,9.75]`, `[0.75,9.25]`,
/// fill in `[1,9]²`.
fn two_wall_square(vertical: bool) -> RenderCase {
    render_witness(RenderMode::Walls, vertical, SQUARE_10, None, 2, PITCH_1MM)
}

/// `[0,10]²` mm minus `[4,6]²`, width 0.5, two walls.
fn two_wall_holed() -> RenderCase {
    render_witness(
        RenderMode::HoledWalls,
        false,
        SQUARE_10,
        Some((40_000, 60_000, 40_000, 60_000)),
        2,
        PITCH_1MM,
    )
}

/// Private-kernel zero walls on the exact asymmetric-hole control witness.
fn zero_wall_asymmetric_hole() -> RenderCase {
    render_witness(
        RenderMode::ZeroWallHoled,
        false,
        SQUARE_10,
        Some((20_000, 30_000, 40_000, 60_000)),
        0,
        PITCH_1MM,
    )
}

/// 1.125 mm square, two walls: wall 0 (inset 0.25) survives, wall 1 (inset
/// 0.75) and the fill (inset 1.0) collapse.
fn collapse_partial() -> RenderCase {
    render_witness(
        RenderMode::Collapse,
        false,
        (0, 11_250, 0, 11_250),
        None,
        2,
        PITCH_1MM,
    )
}

/// 0.375 mm square, one wall: everything collapses; empty is correct.
fn collapse_empty() -> RenderCase {
    render_witness(
        RenderMode::Collapse,
        false,
        (0, 3_750, 0, 3_750),
        None,
        1,
        PITCH_1MM,
    )
}

/// 2.125 mm square, one wall, 4 mm pitch: the 1.125 mm fill region has no
/// ordinary row, so the sub-pitch mid-scan fallback emits one stroke.
fn collapse_fallback() -> RenderCase {
    render_witness(
        RenderMode::Collapse,
        false,
        (0, 21_250, 0, 21_250),
        None,
        1,
        40_000,
    )
}

fn wall_loops(paths: &[ExtrusionPath3D]) -> Vec<&ExtrusionPath3D> {
    paths.iter().filter(|p| p.points.len() > 2).collect()
}

/// `(min x, max x)` mm of a loop.
fn x_extent(path: &ExtrusionPath3D) -> (f32, f32) {
    path.points
        .iter()
        .fold((f32::MAX, f32::MIN), |(lo, hi), p| {
            (lo.min(p.x), hi.max(p.x))
        })
}

#[derive(Debug, Default)]
struct RenderWitnessStats {
    square_bands: u64,
    square_fill: u64,
    vertical_fill: u64,
    hole_bands: u64,
    hole_split: u64,
    zero_wall_strokes: u64,
    collapse_partial: u64,
    collapse_empty: u64,
    collapse_fallback: u64,
}

/// Every deterministic render witness, through the checker.
fn render_witness_observations(limits: &RenderLimits) -> Result<RenderWitnessStats, String> {
    let mut out = RenderWitnessStats::default();
    let checked = |case: &RenderCase| -> Result<(Vec<ExtrusionPath3D>, RenderStats), String> {
        let paths = run_render(case)?;
        let stats = check_render(case, limits, &paths)
            .map_err(|e| format!("{} render witness: {e}", case.mode.label()))?;
        Ok((paths, stats))
    };
    let tol = units_to_mm(CLIPPER_ALLOW);
    let near = |x: f32, y: f32| (x - y).abs() <= tol;

    let (paths, stats) = checked(&two_wall_square(false))?;
    let mut extents: Vec<(f32, f32)> = wall_loops(&paths).into_iter().map(x_extent).collect();
    extents.sort_by(|a, b| a.0.total_cmp(&b.0));
    let distinct = extents.len() == 2
        && near(extents[0].0, 0.25)
        && near(extents[0].1, 9.75)
        && near(extents[1].0, 0.75)
        && near(extents[1].1, 9.25);
    if !distinct || stats.outer_walls != 2 || stats.fill_strokes != 8 {
        return Err(format!(
            "two-wall square: wall x-extents {extents:?}, {} fill strokes",
            stats.fill_strokes
        ));
    }
    out.square_bands += stats.outer_walls as u64;
    out.square_fill += stats.fill_strokes as u64;

    let (paths, stats) = checked(&two_wall_square(true))?;
    let vertical_strokes = paths
        .iter()
        .filter(|p| p.points.len() == 2 && p.points[0].x == p.points[1].x)
        .count();
    if stats.fill_strokes != 8 || vertical_strokes != 8 {
        return Err("vertical two-wall square lacks its 8 vertical fill strokes".to_owned());
    }
    out.vertical_fill += stats.fill_strokes as u64;

    let (_, stats) = checked(&two_wall_holed())?;
    if stats.hole_walls != 2 || stats.split_rows != 4 {
        return Err(format!(
            "two-wall holed square: {} hole walls, {} split rows",
            stats.hole_walls, stats.split_rows
        ));
    }
    out.hole_bands += stats.hole_walls as u64;
    out.hole_split += stats.split_rows as u64;

    let (paths, stats) = checked(&zero_wall_asymmetric_hole())?;
    if !wall_loops(&paths).is_empty() || stats.fill_strokes != 12 || stats.split_rows != 2 {
        return Err("zero-wall asymmetric hole: expected 12 exact strokes, 2 split".to_owned());
    }
    out.zero_wall_strokes += stats.fill_strokes as u64;

    let (paths, stats) = checked(&collapse_partial())?;
    if wall_loops(&paths).len() != 1 || stats.fill_strokes != 0 {
        return Err("collapse witness: expected exactly the surviving outer wall".to_owned());
    }
    out.collapse_partial += 1;

    let (paths, _) = checked(&collapse_empty())?;
    if !paths.is_empty() {
        return Err("fully collapsed witness emitted output".to_owned());
    }
    out.collapse_empty += 1;

    let (_, stats) = checked(&collapse_fallback())?;
    if !stats.fallback || stats.fill_strokes != 1 || stats.outer_walls != 1 {
        return Err("sub-pitch render witness lacks its fallback stroke".to_owned());
    }
    out.collapse_fallback += 1;
    Ok(out)
}

#[test]
fn proptest_tree_support_walls_and_fill() {
    let run = PropertyRun::new(RENDER_ID, RENDER_TEST).expect("M-TREE-RENDER context");
    let limits = checked_render_limits(&read_live_bounds(&run, &RENDER_BOUND_KEYS))
        .expect("M-TREE-RENDER bound consistency");
    let strategy = render_case_strategy(&limits, RENDER_MODES.to_vec(), vec![false, true]);

    let calls = Cell::new(0_u64);
    let w_square_bands = Cell::new(0_u64);
    let w_square_fill = Cell::new(0_u64);
    let w_vertical = Cell::new(0_u64);
    let w_hole_bands = Cell::new(0_u64);
    let w_hole_split = Cell::new(0_u64);
    let w_zero = Cell::new(0_u64);
    let w_collapse = Cell::new(0_u64);
    let w_empty = Cell::new(0_u64);
    let w_fallback = Cell::new(0_u64);
    let walls_seen = Cell::new(0_u64);
    let fill_seen = Cell::new(0_u64);
    let split_seen = Cell::new(0_u64);
    let empty_collapse = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 5] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        let w = match render_witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        w_square_bands.set(w_square_bands.get() + w.square_bands);
        w_square_fill.set(w_square_fill.get() + w.square_fill);
        w_vertical.set(w_vertical.get() + w.vertical_fill);
        w_hole_bands.set(w_hole_bands.get() + w.hole_bands);
        w_hole_split.set(w_hole_split.get() + w.hole_split);
        w_zero.set(w_zero.get() + w.zero_wall_strokes);
        w_collapse.set(w_collapse.get() + w.collapse_partial);
        w_empty.set(w_empty.get() + w.collapse_empty);
        w_fallback.set(w_fallback.get() + w.collapse_fallback);

        let paths = match run_render(&case) {
            Ok(paths) => paths,
            Err(message) => return fail(message),
        };
        let stats = match check_render(&case, &limits, &paths) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{} case failed: {e}", case.mode.label())),
        };
        let ok = match case.mode {
            RenderMode::Walls => stats.outer_walls == case.walls && stats.fill_strokes > 0,
            RenderMode::HoledWalls => {
                stats.outer_walls == case.walls
                    && stats.hole_walls == case.walls
                    && stats.split_rows > 0
            }
            RenderMode::ZeroWall => paths.len() == stats.fill_strokes && stats.fill_strokes > 0,
            RenderMode::ZeroWallHoled => paths.len() == stats.fill_strokes && stats.split_rows > 0,
            RenderMode::Collapse => true,
        };
        if !ok {
            return fail(format!(
                "{} case lost its positive witness: {stats:?}",
                case.mode.label()
            ));
        }
        if case.mode == RenderMode::Collapse && paths.is_empty() {
            empty_collapse.set(empty_collapse.get() + 1);
        }
        let slot = RENDER_MODES
            .iter()
            .position(|m| *m == case.mode)
            .expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        walls_seen.set(walls_seen.get() + (stats.outer_walls + stats.hole_walls) as u64);
        fill_seen.set(fill_seen.get() + stats.fill_strokes as u64);
        split_seen.set(split_seen.get() + stats.split_rows as u64);
        Ok(())
    })
    .expect("M-TREE-RENDER analytic wall bands, fill region and metadata");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(w_square_bands.get() > 0, "two-wall square bands unobserved");
    assert!(w_square_fill.get() > 0, "two-wall square fill unobserved");
    assert!(w_vertical.get() > 0, "vertical fill unobserved");
    assert!(w_hole_bands.get() > 0, "hole wall bands unobserved");
    assert!(w_hole_split.get() > 0, "holed fill split rows unobserved");
    assert!(w_zero.get() > 0, "zero-wall exact fill unobserved");
    assert!(w_collapse.get() > 0, "partial collapse unobserved");
    assert!(w_empty.get() > 0, "full collapse unobserved");
    assert!(w_fallback.get() > 0, "sub-pitch fallback unobserved");
    println!(
        "M-TREE-RENDER accepted_calls={} walls={} holed_walls={} zero_wall={} \
         zero_wall_holed={} collapse={} collapse_empty={} wall_loops={} fill_strokes={} \
         split_rows={} witness_square_bands={} witness_square_fill={} witness_vertical_fill={} \
         witness_hole_bands={} witness_hole_split={} witness_zero_wall={} \
         witness_collapse_partial={} witness_collapse_empty={} witness_fallback={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        per_mode[4].get(),
        empty_collapse.get(),
        walls_seen.get(),
        fill_seen.get(),
        split_seen.get(),
        w_square_bands.get(),
        w_square_fill.get(),
        w_vertical.get(),
        w_hole_bands.get(),
        w_hole_split.get(),
        w_zero.get(),
        w_collapse.get(),
        w_empty.get(),
        w_fallback.get()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// Deterministic mutation controls
// ═════════════════════════════════════════════════════════════════════════════

/// A stroke `a → b` (XY mm) with the metadata of a real emitted path, so the
/// only defect a control introduces is geometric.
fn stroke_like(template: &ExtrusionPath3D, a: (f32, f32), b: (f32, f32)) -> ExtrusionPath3D {
    let mut path = template.clone();
    path.points = vec![
        Point3WithWidth {
            x: a.0,
            y: a.1,
            ..template.points[0]
        },
        Point3WithWidth {
            x: b.0,
            y: b.1,
            ..template.points[0]
        },
    ];
    path
}

fn expect_class(
    result: Result<impl fmt::Debug, CheckFailure>,
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    match result {
        Ok(stats) => panic!("{what}: the mutation was accepted ({stats:?})"),
        Err(e) => assert!(
            matches_class(&e),
            "{what}: rejected for the wrong reason: {e}"
        ),
    }
}

fn is_containment(e: &CheckFailure) -> bool {
    matches!(e, CheckFailure::Containment(_))
}

fn is_intervals(e: &CheckFailure) -> bool {
    matches!(e, CheckFailure::Intervals(_))
}

fn is_metadata(e: &CheckFailure) -> bool {
    matches!(e, CheckFailure::Metadata(_))
}

fn is_wall_bands(e: &CheckFailure) -> bool {
    matches!(e, CheckFailure::WallBands(_))
}

fn is_false_empty(e: &CheckFailure) -> bool {
    *e == CheckFailure::FalseEmpty
}

fn assert_invalid<T: fmt::Debug>(label: &str, result: Result<T, DomainError>) {
    match result {
        Err(DomainError::InvalidBounds(message)) => assert!(!message.is_empty(), "{label}"),
        other => panic!("{label}: expected InvalidBounds, got {other:?}"),
    }
}

/// The charter hole-crossing segment `(1,5) → (9,5)` mm: both endpoints and
/// the midpoint are material, so only a whole-segment oracle rejects it.
fn assert_shortcut_points_are_material(region: &ExPolygon) {
    for (x, y) in [(1.0, 5.0), (9.0, 5.0), (5.0, 5.0)] {
        assert_eq!(
            point_in_region(Point2::from_mm(x, y), std::slice::from_ref(region))
                .expect("point oracle"),
            PointLocation::Inside,
            "({x},{y}) mm must be material"
        );
    }
}

#[test]
fn tree_renderer_controls() {
    // ── M-TREE-SCAN checker ────────────────────────────────────────────────
    let limits = checked_scan_limits(&scan_probe_bound_map()).expect("scan probe bounds");

    // Exact asymmetric hole, both orientations: reject (1,5) -> (9,5).
    for vertical in [false, true] {
        let hole = asymmetric_hole_scan(vertical);
        let paths = run_scan(&hole);
        let stats = check_scan(&hole, &limits, &paths).expect("unmodified asymmetric hole");
        assert!(stats.split_rows > 0 && stats.left > 0 && stats.right > 0);
        assert_shortcut_points_are_material(&hole.expoly());
        let shortcut = stroke_like(&paths[0], (1.0, 5.0), (9.0, 5.0));
        let mut appended = paths.clone();
        appended.push(shortcut.clone());
        expect_class(
            check_scan(&hole, &limits, &appended),
            &format!("appended (1,5)->(9,5), vertical={vertical}"),
            is_containment,
        );
        if !vertical {
            // Replace the split rows y=4.5 and 5.5 by the single unsplit stroke.
            let mut replaced: Vec<ExtrusionPath3D> = paths
                .iter()
                .filter(|p| p.points[0].y != 4.5 && p.points[0].y != 5.5)
                .cloned()
                .collect();
            replaced.push(shortcut);
            expect_class(
                check_scan(&hole, &limits, &replaced),
                "split rows replaced by unsplit (1,5)->(9,5)",
                is_containment,
            );
        }
    }

    // U notch chord (2,5) -> (8,5).
    let notch = u_notch_scan();
    let notch_paths = run_scan(&notch);
    let stats = check_scan(&notch, &limits, &notch_paths).expect("unmodified U notch");
    assert!(stats.split_rows > 0 && stats.left > 0 && stats.right > 0);
    let mut appended = notch_paths.clone();
    appended.push(stroke_like(&notch_paths[0], (2.0, 5.0), (8.0, 5.0)));
    expect_class(
        check_scan(&notch, &limits, &appended),
        "notch chord",
        is_containment,
    );

    // Plain 10 mm square, 1 mm pitch. This kernel's rows sit at
    // `y0 + spacing/2 + n*spacing` = 0.5, 1.5, ..., 9.5 mm, so there is no
    // y = 1 mm row; the charter's "interior y=1 row" is row index 1, y = 1.5.
    let sq = square_scan();
    let sq_paths = run_scan(&sq);
    let stats = check_scan(&sq, &limits, &sq_paths).expect("unmodified square");
    assert_eq!(stats.rows, 10);
    let dropped: Vec<ExtrusionPath3D> = sq_paths
        .iter()
        .filter(|p| p.points[0].y != 1.5)
        .cloned()
        .collect();
    assert_eq!(dropped.len(), sq_paths.len() - 1, "exactly the y=1.5 row");
    expect_class(
        check_scan(&sq, &limits, &dropped),
        "dropped interior row",
        is_intervals,
    );
    expect_class(
        check_scan(&sq, &limits, &[]),
        "false all-empty output",
        is_false_empty,
    );
    // A stroke on the excluded top edge lies on the boundary: only the
    // complete row set rejects it.
    let mut extra = sq_paths.clone();
    extra.push(stroke_like(&sq_paths[0], (0.0, 10.0), (10.0, 10.0)));
    expect_class(
        check_scan(&sq, &limits, &extra),
        "row on the open top edge",
        is_intervals,
    );

    // Vertical charter witness: drop a stroke, truncate a stroke.
    let vw = vertical_witness();
    let vw_paths = run_scan(&vw);
    check_scan(&vw, &limits, &vw_paths).expect("unmodified vertical witness");
    expect_class(
        check_scan(&vw, &limits, &vw_paths[1..]),
        "vertical witness missing a stroke",
        is_intervals,
    );
    let mut truncated = vw_paths.clone();
    let x = truncated[0].points[0].x;
    truncated[0] = stroke_like(&vw_paths[0], (x, 0.0), (x, 10.0));
    expect_class(
        check_scan(&vw, &limits, &truncated),
        "vertical stroke truncated to y [0,10]",
        is_intervals,
    );

    // Fallback witness: the center line is required, not optional.
    let fb = fallback_witness(false);
    let fb_paths = run_scan(&fb);
    let stats = check_scan(&fb, &limits, &fb_paths).expect("unmodified fallback witness");
    assert!(stats.fallback);
    expect_class(
        check_scan(&fb, &limits, &[]),
        "missing fallback line",
        is_false_empty,
    );

    // Metadata propagation mutations.
    type Mutate = fn(&mut ExtrusionPath3D);
    let mutations: [(&str, Mutate); 7] = [
        ("flow", |p| p.points[0].flow_factor += 1.0),
        ("width", |p| p.points[1].width += 0.25),
        ("z", |p| p.points[0].z += 0.5),
        ("speed", |p| p.speed_factor += 0.5),
        ("role", |p| p.role = ExtrusionRole::SupportInterface),
        ("tool", |p| p.tool_index = Some(1)),
        ("overhang", |p| p.points[1].overhang_quartile = Some(2)),
    ];
    for (what, mutate) in mutations {
        let mut mutant = sq_paths.clone();
        mutate(&mut mutant[0]);
        expect_class(check_scan(&sq, &limits, &mutant), what, is_metadata);
    }
    assert!(sq_paths.iter().all(|p| p
        .points
        .iter()
        .all(|q| q.width == 0.4375 && q.z == 0.2 && q.flow_factor == 1.0)
        && p.speed_factor == 45.0 / 50.0));

    // Degenerate modes: actual empty outcome; nonempty output rejected.
    for kind in SCAN_DEGENERATES {
        let case = degenerate_scan_witness(kind);
        let paths = run_scan(&case);
        assert!(paths.is_empty(), "{kind:?}: the kernel emitted output");
        check_scan(&case, &limits, &paths).expect("empty degenerate output is accepted");
        expect_class(
            check_scan(&case, &limits, &sq_paths[..1]),
            &format!("{kind:?} with output"),
            is_intervals,
        );
    }

    // Scan bound-construction rejection matrix.
    let valid = scan_probe_bound_map();
    for key in SCAN_BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        assert_invalid(&format!("scan missing {key}"), checked_scan_limits(&bounds));
    }
    for (key, value) in [
        ("max_abs_units", 199_999),
        ("max_abs_units", 1_000_001),
        ("max_spacing_units", 22_500),
        ("max_spacing_units", 26_000),
        ("max_spacing_units", 102_500),
        ("max_rows", 9),
        ("max_rows", 1_025),
        ("max_vertices", 7),
        ("max_holes", 0),
        ("max_components", 0),
        ("max_shrink_candidates", 0),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        assert_invalid(
            &format!("scan invalid {key}={value}"),
            checked_scan_limits(&bounds),
        );
    }
    let mut huge = valid.clone();
    huge.insert("max_abs_units".to_owned(), i64::MAX as u64 + 1);
    assert!(matches!(
        checked_scan_limits(&huge),
        Err(DomainError::ArithmeticOverflow)
    ));
    let mut oversized = valid;
    oversized.insert("max_rows".to_owned(), u64::from(u32::MAX) + 1);
    assert!(matches!(
        checked_scan_limits(&oversized),
        Err(DomainError::ArithmeticOverflow)
    ));
    scan_witness_observations(&limits).expect("all scan witnesses inside the probe bounds");

    // ── M-TREE-RENDER checker ──────────────────────────────────────────────
    let limits = checked_render_limits(&render_probe_bound_map()).expect("render probe bounds");

    // Two-wall square: coincident wall, un-inset fill, dropped row, empty.
    let square = two_wall_square(false);
    let paths = run_render(&square).expect("two-wall square");
    let stats = check_render(&square, &limits, &paths).expect("unmodified two-wall square");
    assert_eq!((stats.outer_walls, stats.fill_strokes), (2, 8));
    let tol = units_to_mm(CLIPPER_ALLOW);
    let outer_idx = paths
        .iter()
        .position(|p| p.points.len() > 2 && (x_extent(p).0 - 0.25).abs() <= tol)
        .expect("outer [0.25,9.75] loop");
    let inner_idx = paths
        .iter()
        .position(|p| p.points.len() > 2 && (x_extent(p).0 - 0.75).abs() <= tol)
        .expect("inner [0.75,9.25] loop");
    let mut coincident = paths.clone();
    coincident[inner_idx] = paths[outer_idx].clone();
    expect_class(
        check_render(&square, &limits, &coincident),
        "inner wall replaced by a duplicate of the outer wall",
        is_wall_bands,
    );
    let fill_template = paths
        .iter()
        .find(|p| p.points.len() == 2)
        .expect("a fill stroke")
        .clone();
    let mut uninset = paths.clone();
    uninset.push(stroke_like(&fill_template, (0.5, 5.0), (9.0, 5.0)));
    expect_class(
        check_render(&square, &limits, &uninset),
        "fill at (0.5,5) outside [1,9]^2",
        is_containment,
    );
    let dropped: Vec<ExtrusionPath3D> = paths
        .iter()
        .filter(|p| !(p.points.len() == 2 && p.points[0].y == 1.5))
        .cloned()
        .collect();
    assert_eq!(dropped.len(), paths.len() - 1, "exactly the y=1.5 fill row");
    expect_class(
        check_render(&square, &limits, &dropped),
        "dropped fill row",
        is_intervals,
    );
    expect_class(
        check_render(&square, &limits, &[]),
        "false all-empty render",
        is_false_empty,
    );
    let mut wide = paths.clone();
    wide[outer_idx].points[0].width += 0.25;
    expect_class(
        check_render(&square, &limits, &wide),
        "wall width",
        is_metadata,
    );

    // Holed two-wall square: removing only one hole wall fails.
    let holed = two_wall_holed();
    let hpaths = run_render(&holed).expect("two-wall holed square");
    let stats = check_render(&holed, &limits, &hpaths).expect("unmodified holed square");
    assert_eq!((stats.outer_walls, stats.hole_walls), (2, 2));
    let hole_walls: Vec<usize> = hpaths
        .iter()
        .enumerate()
        .filter(|(_, p)| p.points.len() > 2 && x_extent(p).0 > 2.0)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(hole_walls.len(), 2, "two hole wall loops");
    for idx in hole_walls {
        let mut removed = hpaths.clone();
        removed.remove(idx);
        expect_class(
            check_render(&holed, &limits, &removed),
            &format!("hole wall {idx} removed"),
            is_wall_bands,
        );
    }
    let mut crossing = hpaths.clone();
    crossing.push(stroke_like(&fill_template, (1.0, 5.0), (9.0, 5.0)));
    expect_class(
        check_render(&holed, &limits, &crossing),
        "fill across the grown hole",
        is_containment,
    );

    // Zero walls on the exact control witness: reject (1,5) -> (9,5).
    let zero = zero_wall_asymmetric_hole();
    let zpaths = run_render(&zero).expect("zero-wall render");
    check_render(&zero, &limits, &zpaths).expect("unmodified zero-wall render");
    assert_shortcut_points_are_material(&zero.expoly());
    let mut appended = zpaths.clone();
    appended.push(stroke_like(&zpaths[0], (1.0, 5.0), (9.0, 5.0)));
    expect_class(
        check_render(&zero, &limits, &appended),
        "zero-wall (1,5)->(9,5)",
        is_containment,
    );

    // Collapse: the predicted empty outcome is accepted; spurious output and
    // a missing surviving wall are not.
    let empty = collapse_empty();
    let epaths = run_render(&empty).expect("collapsed render");
    assert!(epaths.is_empty());
    check_render(&empty, &limits, &epaths).expect("predicted empty collapse");
    expect_class(
        check_render(&empty, &limits, &paths[..1]),
        "output on a fully collapsed scene",
        is_wall_bands,
    );
    let partial = collapse_partial();
    let ppaths = run_render(&partial).expect("partial collapse");
    check_render(&partial, &limits, &ppaths).expect("predicted partial collapse");
    expect_class(
        check_render(&partial, &limits, &[]),
        "missing surviving wall",
        is_false_empty,
    );

    // `from_config` clamps the configured wall count: zero never reaches the
    // kernel from config, which is why the zero-wall mode calls it directly.
    assert_eq!(module_for(0.5, 45.0, 0).wall_count, 1);
    assert_eq!(module_for(0.5, 45.0, 3).wall_count, 3);

    // Render bound-construction rejection matrix.
    let valid = render_probe_bound_map();
    for key in RENDER_BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        assert_invalid(
            &format!("render missing {key}"),
            checked_render_limits(&bounds),
        );
    }
    for (key, value) in [
        ("max_abs_units", 99_999),
        ("max_abs_units", 1_000_001),
        ("max_width_units", 2_500),
        ("max_width_units", 7_500),
        ("max_width_units", 25_000),
        ("max_walls", 1),
        ("max_walls", 9),
        ("max_rows", 9),
        ("max_rows", 1_025),
        ("max_vertices", 7),
        ("max_holes", 0),
        ("max_components", 0),
        ("max_shrink_candidates", 0),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        assert_invalid(
            &format!("render invalid {key}={value}"),
            checked_render_limits(&bounds),
        );
    }
    // Eight 2 mm walls cannot fit a 20 mm span.
    let mut cramped = valid.clone();
    cramped.insert("max_width_units".to_owned(), 20_000);
    cramped.insert("max_walls".to_owned(), 8);
    assert_invalid("render cramped span", checked_render_limits(&cramped));
    let mut huge = valid;
    huge.insert("max_width_units".to_owned(), i64::MAX as u64 + 1);
    assert!(matches!(
        checked_render_limits(&huge),
        Err(DomainError::ArithmeticOverflow)
    ));
    render_witness_observations(&limits).expect("all render witnesses inside the probe bounds");
    println!("tree_renderer_controls: all mutations rejected by the shared checkers");
}

// ═════════════════════════════════════════════════════════════════════════════
// Shrink probe
// ═════════════════════════════════════════════════════════════════════════════

/// Drive a deterministic value tree through real simplify/complicate
/// transitions, validating every exposed value with `check`.
fn probe_shrinking<T>(
    label: &str,
    strategy: &BoxedStrategy<T>,
    check: &dyn Fn(&T),
    complexity: &dyn Fn(&T) -> i128,
) where
    T: fmt::Debug + Clone + PartialEq,
{
    // Phase 1: accept every simplification (as if each still failed) and
    // demonstrate a real reduction from a nonminimal start.
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    let initial = tree.current();
    check(&initial);
    let initial_complexity = complexity(&initial);
    let mut accepted = 0_usize;
    while tree.simplify() {
        accepted += 1;
        assert!(
            accepted < MAX_SHRINK_STEPS,
            "{label}: simplify never exhausted"
        );
        check(&tree.current());
    }
    let minimal_complexity = complexity(&tree.current());
    assert!(accepted > 0, "false-only shrink probe for {label}");
    assert!(
        minimal_complexity < initial_complexity,
        "{label}: no permitted reduction ({initial_complexity} -> {minimal_complexity})"
    );

    // Phase 2: alternate simplify/complicate (as if each simplification
    // passed), including failed transitions and repeated complicate.
    let mut runner = TestRunner::deterministic();
    let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
    assert_eq!(tree.current(), initial, "the deterministic start is stable");
    let mut simplified = 0_usize;
    let mut complicated = 0_usize;
    let mut changed = false;
    while tree.simplify() {
        simplified += 1;
        assert!(
            simplified < MAX_SHRINK_STEPS,
            "{label}: alternation never ended"
        );
        let current = tree.current();
        check(&current);
        changed |= current != initial;
        if tree.complicate() {
            complicated += 1;
        }
        check(&tree.current());
    }
    assert!(simplified > 0 && changed, "{label}: no real transition");
    let exhausted = tree.current();
    assert!(
        !tree.simplify(),
        "an exhausted {label} tree simplified again"
    );
    assert_eq!(
        tree.current(),
        exhausted,
        "a false simplify moved the value"
    );
    let mut restores = 0_usize;
    while tree.complicate() {
        restores += 1;
        assert!(
            restores < MAX_SHRINK_STEPS,
            "{label}: complicate never ended"
        );
        check(&tree.current());
    }
    let settled = tree.current();
    assert!(
        !tree.complicate(),
        "complicate after exhaustion must be false"
    );
    assert_eq!(
        tree.current(),
        settled,
        "a false complicate moved the value"
    );
    println!(
        "{label} phase1_simplify={accepted} complexity {initial_complexity}->{minimal_complexity} \
         phase2_simplify={simplified} complicate={complicated} restores={restores}"
    );
}

/// Independently validate every exposed scan input and run it through the
/// same checker.
fn check_scan_shrink_state(case: &ScanCase, mode: ScanMode, vertical: bool, l: &ScanLimits) {
    assert_eq!(case.mode, mode, "the scan mode must survive shrinking");
    assert_eq!(
        case.vertical, vertical,
        "the orientation must survive shrinking"
    );
    assert_eq!(case.degenerate.is_some(), mode == ScanMode::Degenerate);
    let o = case.outer;
    for v in [o.u0, o.u1, o.v0, o.v1] {
        assert!(
            v.abs() <= l.max_abs_units && v % GRID == 0,
            "coordinate {v}"
        );
    }
    assert!(case.line_width_mm > 0.0 && case.line_width_mm <= 2.0);
    assert!((1.0..=300.0).contains(&case.support_speed));
    assert!(case.z > 0.0 && case.z.is_finite());
    if mode != ScanMode::Degenerate {
        assert!(o.u1 - o.u0 >= 3 * GRID, "segment extent");
        assert_eq!(case.spacing_units % SCAN_PITCH_QUANTUM, 0);
        assert!((SCAN_PITCH_QUANTUM..=l.max_spacing_units).contains(&case.spacing_units));
        // Count `v0 + s/2 + n*s < v1` directly.
        let mut rows = 0_i64;
        let mut v = o.v0 + case.spacing_units / 2;
        while v < o.v1 {
            rows += 1;
            v += case.spacing_units;
        }
        if mode == ScanMode::Fallback {
            assert_eq!(rows, 0, "a fallback fixture must have no ordinary row");
        } else {
            assert!(
                (SCAN_MIN_ROWS..=i64::from(l.max_rows)).contains(&rows),
                "row count {rows}"
            );
        }
    }
    match mode {
        ScanMode::Notch | ScanMode::Hole => {
            let c = case.cut.expect("cut present");
            assert!(o.u0 < c.u0 && c.u0 < c.u1 && c.u1 < o.u1, "cut u inside");
            assert!(o.v0 < c.v0 && c.v0 < c.v1, "cut v ordered and above v0");
            if mode == ScanMode::Notch {
                assert_eq!(c.v1, o.v1, "the notch opens through the +v edge");
            } else {
                assert!(c.v1 < o.v1, "the hole stays below the +v edge");
            }
            assert!(
                case.split_rows() > 0,
                "witness lost: no row crosses the cut"
            );
        }
        _ => assert!(case.cut.is_none()),
    }
    let paths = run_scan(case);
    let stats = check_scan(case, l, &paths)
        .unwrap_or_else(|e| panic!("shrunk {} scan fixture: {e}", mode.label()));
    match mode {
        ScanMode::Degenerate => assert!(paths.is_empty()),
        ScanMode::Rect => assert!(stats.strokes > 0 && !stats.fallback),
        ScanMode::Fallback => assert!(stats.fallback && stats.strokes == 1),
        ScanMode::Notch | ScanMode::Hole => {
            assert!(stats.split_rows > 0 && stats.left > 0 && stats.right > 0);
        }
    }
}

fn scan_complexity(case: &ScanCase, l: &ScanLimits) -> i128 {
    let m = l.max_abs_units;
    let o = case.outer;
    let cut = case.cut.map_or(0, |c| {
        (c.u1 - c.u0) + (c.v1 - c.v0) + (c.u0 - o.u0) + (c.v0 - o.v0)
    });
    i128::from(
        (o.u1 - o.u0) + (o.v1 - o.v0) + (o.u0 + m) + (o.v0 + m) + case.spacing_units.abs() + cut,
    ) + (case.line_width_mm * 64.0) as i128
        + case.support_speed as i128
        + (case.z * 20.0).round() as i128
}

/// Independently validate every exposed render input and run it through the
/// same checker.
fn check_render_shrink_state(
    case: &RenderCase,
    mode: RenderMode,
    vertical: bool,
    l: &RenderLimits,
) {
    assert_eq!(case.mode, mode, "the render mode must survive shrinking");
    assert_eq!(
        case.vertical, vertical,
        "the orientation must survive shrinking"
    );
    let o = case.outer;
    for v in [o.u0, o.u1, o.v0, o.v1] {
        assert!(v.abs() <= l.max_abs_units, "coordinate {v}");
    }
    assert!(case.line_width_units % RENDER_QUANTUM == 0);
    assert!((RENDER_QUANTUM..=l.max_width_units).contains(&case.line_width_units));
    assert!(case.fill_spacing_units % RENDER_QUANTUM == 0 && case.fill_spacing_units > 0);
    assert!((1.0..=300.0).contains(&case.support_speed));
    assert!(case.z > 0.0 && case.z.is_finite());
    if mode.zero_wall() {
        assert_eq!(case.walls, 0);
    } else {
        assert!((1..=l.max_walls as usize).contains(&case.walls));
    }
    let d_total = case.line_width_units * case.walls as i64;
    match case.hole {
        Some(h) => {
            assert!(mode.holed());
            assert!(h.is_proper(), "the hole has positive size");
            assert!(
                h.grow(d_total).strictly_inside(o.grow(-d_total), 0),
                "the grown hole stays inside the fill region"
            );
        }
        None => assert!(!mode.holed(), "a holed mode lost its hole"),
    }
    if mode != RenderMode::Collapse {
        let fill = o.grow(-d_total);
        assert!(fill.is_proper(), "a roomy scene keeps a fill region");
    }
    let paths = run_render(case).unwrap_or_else(|e| panic!("shrunk render: {e}"));
    let stats = check_render(case, l, &paths)
        .unwrap_or_else(|e| panic!("shrunk {} render fixture: {e}", mode.label()));
    match mode {
        RenderMode::Walls => assert!(stats.outer_walls == case.walls && stats.fill_strokes > 0),
        RenderMode::HoledWalls => assert!(stats.hole_walls == case.walls && stats.split_rows > 0),
        RenderMode::ZeroWall => assert!(stats.fill_strokes > 0),
        RenderMode::ZeroWallHoled => assert!(stats.split_rows > 0),
        RenderMode::Collapse => {}
    }
}

fn render_complexity(case: &RenderCase, l: &RenderLimits) -> i128 {
    let m = l.max_abs_units;
    let o = case.outer;
    let hole = case.hole.map_or(0, |h| {
        (h.u1 - h.u0) + (h.v1 - h.v0) + (h.u0 - o.u0) + (h.v0 - o.v0)
    });
    i128::from(
        (o.u1 - o.u0)
            + (o.v1 - o.v0)
            + (o.u0 + m)
            + (o.v0 + m)
            + hole
            + case.line_width_units
            + case.fill_spacing_units
            + case.walls as i64 * 10_000,
    ) + case.support_speed as i128
        + (case.z * 20.0).round() as i128
}

#[test]
fn tree_renderer_shrink_bounds() {
    let scan_limits = checked_scan_limits(&scan_probe_bound_map()).expect("scan shrink bounds");
    for vertical in [false, true] {
        for mode in SCAN_MODES {
            // The degenerate kind is fixed per run so the probe is deterministic.
            let strategy = scan_case_strategy(
                &scan_limits,
                vec![mode],
                vec![ScanDegenerate::NegativeSpacing],
                vec![vertical],
            );
            probe_shrinking(
                &format!("scan mode={mode:?} vertical={vertical}"),
                &strategy,
                &|case| check_scan_shrink_state(case, mode, vertical, &scan_limits),
                &|case| scan_complexity(case, &scan_limits),
            );
        }
    }
    let render_limits =
        checked_render_limits(&render_probe_bound_map()).expect("render shrink bounds");
    // Render probes call Clipper several times per transition; every mode is
    // probed horizontally and the orientation's survival once vertically
    // (the scan probes above cover every mode in both orientations).
    let render_probes = RENDER_MODES
        .iter()
        .map(|mode| (*mode, false))
        .chain(std::iter::once((RenderMode::HoledWalls, true)));
    for (mode, vertical) in render_probes {
        {
            let strategy = render_case_strategy(&render_limits, vec![mode], vec![vertical]);
            probe_shrinking(
                &format!("render mode={mode:?} vertical={vertical}"),
                &strategy,
                &|case| check_render_shrink_state(case, mode, vertical, &render_limits),
                &|case| render_complexity(case, &render_limits),
            );
        }
    }

    // The mode, degenerate and orientation selectors never shrink: a
    // shrinking selector could switch a witness's class mid-search.
    let mut runner = TestRunner::deterministic();
    let modes = prop::sample::select(SCAN_MODES.to_vec()).no_shrink();
    let mut tree = modes.new_tree(&mut runner).expect("scan mode tree");
    let before = tree.current();
    assert!(!tree.simplify(), "the scan mode selector simplified");
    assert!(!tree.complicate(), "the scan mode selector complicated");
    assert_eq!(tree.current(), before);
    let kinds = prop::sample::select(SCAN_DEGENERATES.to_vec()).no_shrink();
    let mut tree = kinds.new_tree(&mut runner).expect("degenerate tree");
    let before = tree.current();
    assert!(!tree.simplify(), "the degenerate selector simplified");
    assert!(!tree.complicate(), "the degenerate selector complicated");
    assert_eq!(tree.current(), before);
    let render_modes = prop::sample::select(RENDER_MODES.to_vec()).no_shrink();
    let mut tree = render_modes
        .new_tree(&mut runner)
        .expect("render mode tree");
    let before = tree.current();
    assert!(!tree.simplify(), "the render mode selector simplified");
    assert!(!tree.complicate(), "the render mode selector complicated");
    assert_eq!(tree.current(), before);
    let orientations = prop::sample::select(vec![false, true]).no_shrink();
    let mut tree = orientations
        .new_tree(&mut runner)
        .expect("orientation tree");
    let before = tree.current();
    assert!(!tree.simplify(), "the orientation selector simplified");
    assert!(!tree.complicate(), "the orientation selector complicated");
    assert_eq!(tree.current(), before);
}
