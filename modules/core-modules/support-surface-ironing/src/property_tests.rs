//! Opt-in property tests for this module's private support-ironing scan kernel.
//!
//! Charter (`docs/spec_packets/proptest-openrscad_14_pbt-support-modules`):
//!
//! * `M-SUPPORT-IRONING` — `SupportSurfaceIroning::fill_expolygon` on
//!   independently valid rectangle, open-to-top notch and rectangular-hole
//!   shapes with a positive configured spacing. The checker compares the
//!   **complete** expected set of half-open horizontal rows and material
//!   intervals, checks **every** consecutive segment of every returned
//!   `ExtrusionPath3D` for whole-segment containment (hole exclusion), and
//!   inspects every point's supplied Z, the configured line width and the
//!   configured `ironing_flow_rate`, plus the caller-supplied speed factor.
//!   This is a metadata propagation contract, not a claim that the manifest
//!   default flow value is an independently verified physical percentage.
//!   Nonpositive spacing and degenerate geometry are a separate mode whose
//!   actual kernel outcome (empty) is checked.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `fill_expolygon` is private, so the properties live here as a
//! crate-internal unit-test module; production visibility is not widened. The
//! module is constructed only through `SupportSurfaceIroning::from_config`.
//!
//! # Independent expected model
//!
//! Every generated shape is axis-aligned with integer canonical-unit vertices,
//! so every material crossing of a scan row lies exactly on a vertical edge.
//! The expected rows are `y0 + i * spacing` while `< y1` (the outer top edge
//! is excluded); a cut `[cx0,cx1] × [cy0,cy1]` splits the rows with
//! `cy0 <= y < cy1` (its bottom edge included, its top edge excluded). This is
//! rectangle arithmetic over the fixture's own fields, never kernel output.
//! Output coordinates are recovered to canonical units and rejected unless the
//! recovery round-trips bit-exactly, so the comparison is exact, not
//! tolerance-based.
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
//!   one literal each in `FillCase::geometry_domain`, both carrying an
//!   `// exhaustive:` note because every field is derived from the live limits
//!   or the fixture's topology class.
//! * `FillCase`, `SiLimits`, `FillStats` and `CheckFailure` are private
//!   module-local types, not watched crate-public structs.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! controls are mutation controls over the SAME checker the property uses,
//! each asserting the specific `CheckFailure` class, so none is a decorative
//! or success-only assertion; the property requires positive per-subcase
//! counters from deterministic witnesses built inside its callback.

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

use super::{SupportSurfaceIroning, BASE_SPEED};

const PROPERTY_ID: &str = "M-SUPPORT-IRONING";
const PROPERTY_TEST: &str = "property_tests::proptest_support_ironing_fill";
const BOUND_KEYS: [&str; 7] = [
    "max_abs_units",
    "max_vertices",
    "max_holes",
    "max_components",
    "max_shrink_candidates",
    "max_spacing_units",
    "max_rows",
];

/// 0.01 mm, the manifest minimum `ironing_spacing`; generated spacings are
/// positive multiples of it.
const SPACING_QUANTUM_UNITS: i64 = 100;
/// 1.0 mm, the manifest maximum `ironing_spacing`.
const CONFIG_MAX_SPACING_UNITS: i64 = 10_000;
/// 1 mm pitch of every deterministic witness.
const WITNESS_SPACING_UNITS: i64 = 10_000;
/// 10 mm side of every deterministic `[0,10]²` mm witness.
const WITNESS_EXTENT_UNITS: i64 = 100_000;
/// Scan rows of every witness at 1 mm pitch: y = 0, 1, ..., 9 mm.
const WITNESS_ROWS: u32 = 10;
/// Coordinate ceiling `validate_geometry` accepts.
const VALIDATOR_ABS_CEILING: i64 = 1_000_000;
/// Fewest scan rows of a generated case: one below a cut, one through it and
/// one above it.
const MIN_ROWS: i64 = 3;
/// Runtime ceiling on the configured row bound.
const MAX_ROWS_CEILING: u32 = 1_024;
/// Narrowest generated width: one unit of material, cut, material.
const MIN_WIDTH_UNITS: i64 = 3;
/// Total vertices of the largest generated class (8-vertex notch contour, or
/// 4 + 4 for a rectangle with one hole).
const CLASS_VERTICES: u32 = 8;
/// Step ceiling for the shrink probe's transition loops.
const MAX_SHRINK_STEPS: usize = 20_000;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct SiLimits {
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

fn checked_limits(bounds: &BTreeMap<String, u64>) -> Result<SiLimits, DomainError> {
    let as_i64 = |key: &str| -> Result<i64, DomainError> {
        i64::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
    };
    let as_u32 = |key: &str| -> Result<u32, DomainError> {
        u32::try_from(required(bounds, key)?).map_err(|_| DomainError::ArithmeticOverflow)
    };
    let limits = SiLimits {
        max_abs_units: as_i64("max_abs_units")?,
        max_vertices: as_u32("max_vertices")?,
        max_holes: as_u32("max_holes")?,
        max_components: as_u32("max_components")?,
        max_shrink_candidates: as_u32("max_shrink_candidates")?,
        max_spacing_units: as_i64("max_spacing_units")?,
        max_rows: as_u32("max_rows")?,
    };

    if !(WITNESS_EXTENT_UNITS..=VALIDATOR_ABS_CEILING).contains(&limits.max_abs_units) {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units must be in {WITNESS_EXTENT_UNITS}..={VALIDATOR_ABS_CEILING}: the \
             [0,10]² mm witnesses need 100000 and validate_geometry caps coordinates"
        )));
    }
    if limits.max_spacing_units % SPACING_QUANTUM_UNITS != 0
        || limits.max_spacing_units < WITNESS_SPACING_UNITS
    {
        return Err(DomainError::InvalidBounds(format!(
            "max_spacing_units must be a multiple of {SPACING_QUANTUM_UNITS} and admit the \
             1 mm witness pitch ({WITNESS_SPACING_UNITS})"
        )));
    }
    if limits.max_spacing_units > CONFIG_MAX_SPACING_UNITS {
        return Err(DomainError::InvalidBounds(format!(
            "max_spacing_units must not exceed the manifest maximum ironing_spacing \
             ({CONFIG_MAX_SPACING_UNITS} units = 1.0 mm)"
        )));
    }
    if !(WITNESS_ROWS..=MAX_ROWS_CEILING).contains(&limits.max_rows) {
        return Err(DomainError::InvalidBounds(format!(
            "max_rows must be in {WITNESS_ROWS}..={MAX_ROWS_CEILING}: the witnesses scan \
             {WITNESS_ROWS} rows"
        )));
    }
    if limits.max_vertices < CLASS_VERTICES
        || limits.max_holes < 1
        || limits.max_components < 1
        || limits.max_shrink_candidates < 1
    {
        return Err(DomainError::InvalidBounds(format!(
            "max_vertices >= {CLASS_VERTICES}, max_holes >= 1, max_components >= 1 and \
             max_shrink_candidates >= 1 are required by the generated classes"
        )));
    }
    let span = limits
        .max_abs_units
        .checked_mul(2)
        .ok_or(DomainError::ArithmeticOverflow)?;
    if span / limits.max_spacing_units < MIN_ROWS {
        return Err(DomainError::InvalidBounds(
            "the coordinate span must admit three rows at the maximum spacing".to_owned(),
        ));
    }
    Ok(limits)
}

fn live_limits(run: &PropertyRun) -> SiLimits {
    let mut bounds = BTreeMap::new();
    for key in BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("support ironing bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_limits(&bounds).expect("support ironing bound consistency")
}

/// The smallest bounds that admit every deterministic witness and every
/// generated class; the controls and the shrink probe use them without a
/// property context.
fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 100_000),
        ("max_vertices".to_owned(), 8),
        ("max_holes".to_owned(), 1),
        ("max_components".to_owned(), 1),
        ("max_shrink_candidates".to_owned(), 1),
        ("max_spacing_units".to_owned(), 10_000),
        ("max_rows".to_owned(), 10),
    ])
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures and the independent expected model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Rect,
    Notch,
    Hole,
    Degenerate,
}

const MODES: [Mode; 4] = [Mode::Rect, Mode::Notch, Mode::Hole, Mode::Degenerate];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Degenerate {
    ZeroHeight,
    ZeroWidth,
    ShortRing,
    EmptyContour,
    ZeroSpacing,
    NegativeSpacing,
    /// A positive configured spacing that rounds to zero canonical units.
    SubUnitSpacing,
}

const DEGENERATES: [Degenerate; 7] = [
    Degenerate::ZeroHeight,
    Degenerate::ZeroWidth,
    Degenerate::ShortRing,
    Degenerate::EmptyContour,
    Degenerate::ZeroSpacing,
    Degenerate::NegativeSpacing,
    Degenerate::SubUnitSpacing,
];

impl Mode {
    fn label(self) -> &'static str {
        match self {
            Self::Rect => "rect",
            Self::Notch => "notch",
            Self::Hole => "hole",
            Self::Degenerate => "degenerate",
        }
    }
}

/// One bounded axis-aligned support-ironing fixture plus its explicit config.
#[derive(Debug, Clone, PartialEq)]
struct FillCase {
    mode: Mode,
    /// Present exactly when `mode == Mode::Degenerate`.
    degenerate: Option<Degenerate>,
    /// Outer rectangle `[x0,x1] × [y0,y1]`, canonical units.
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    /// Hole or open-to-top notch `[cx0,cx1] × [cy0,cy1]`, canonical units.
    cut: Option<(i64, i64, i64, i64)>,
    /// The intended canonical-unit pitch; `spacing_mm` must round-trip to it.
    spacing_units: i64,
    /// Configured `ironing_spacing`, mm.
    spacing_mm: f64,
    /// Configured `line_width`, mm.
    line_width_mm: f64,
    /// Configured `ironing_flow_rate`.
    flow_rate: f64,
    /// Configured `ironing_speed`, mm/s.
    ironing_speed: f64,
    /// Caller-supplied layer Z, mm.
    z: f32,
}

impl FillCase {
    /// The caller's speed factor, computed as `run_support_postprocess` does;
    /// the kernel only propagates it.
    fn speed_factor(&self) -> f32 {
        (self.ironing_speed as f32) / BASE_SPEED
    }

    fn expoly(&self) -> ExPolygon {
        let pt = |x: i64, y: i64| Point2 { x, y };
        let (x0, y0, x1, y1) = (self.x0, self.y0, self.x1, self.y1);
        let rect = || Polygon {
            points: vec![pt(x0, y0), pt(x1, y0), pt(x1, y1), pt(x0, y1)],
        };
        match (self.mode, self.degenerate) {
            (Mode::Degenerate, Some(Degenerate::ShortRing)) => ExPolygon {
                contour: Polygon {
                    points: vec![pt(x0, y0)],
                },
                holes: Vec::new(),
            },
            (Mode::Degenerate, Some(Degenerate::EmptyContour)) => ExPolygon {
                contour: Polygon { points: Vec::new() },
                holes: Vec::new(),
            },
            (Mode::Notch, _) => {
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
            (Mode::Hole, _) => {
                let (hx0, hx1, hy0, hy1) = self.cut.expect("hole case carries a cut");
                ExPolygon {
                    contour: rect(),
                    holes: vec![Polygon {
                        points: vec![pt(hx0, hy0), pt(hx0, hy1), pt(hx1, hy1), pt(hx1, hy0)],
                    }],
                }
            }
            _ => ExPolygon {
                contour: rect(),
                holes: Vec::new(),
            },
        }
    }

    /// The fixture's geometry is a valid region (anything but a geometric
    /// degenerate); spacing-degenerate cases still have valid rectangles.
    fn has_valid_geometry(&self) -> bool {
        !matches!(
            self.degenerate,
            Some(
                Degenerate::ZeroHeight
                    | Degenerate::ZeroWidth
                    | Degenerate::ShortRing
                    | Degenerate::EmptyContour
            )
        )
    }

    fn geometry_domain(&self, limits: &SiLimits) -> GeometryDomain {
        let (class, min_holes) = match self.mode {
            Mode::Rect | Mode::Degenerate => (TopologyClass::Convex, 0),
            Mode::Notch => (TopologyClass::Concave, 0),
            Mode::Hole => (TopologyClass::Holed, 1),
        };
        // exhaustive: every field is the fixture's topology or the boundary rule the checker uses.
        GeometryDomain {
            class,
            min_holes,
            min_components: 1,
            boundary: BoundaryRule::Inclusive,
            // exhaustive: all five fields come from the live profile bounds.
            bounds: GeometryBounds {
                max_abs_units: limits.max_abs_units,
                max_vertices: limits.max_vertices as usize,
                max_holes: limits.max_holes as usize,
                max_components: limits.max_components as usize,
                max_shrink_candidates: limits.max_shrink_candidates,
            },
        }
    }

    /// Material intervals of the row at `y` by rectangle arithmetic.
    fn intervals_at(&self, y: i64) -> Vec<(i64, i64)> {
        if !(self.y0 <= y && y < self.y1) {
            return Vec::new();
        }
        match (self.mode, self.cut) {
            (Mode::Hole, Some((hx0, hx1, hy0, hy1))) if hy0 <= y && y < hy1 => {
                vec![(self.x0, hx0), (hx1, self.x1)]
            }
            (Mode::Notch, Some((nx0, nx1, ny0, _))) if ny0 <= y => {
                vec![(self.x0, nx0), (nx1, self.x1)]
            }
            _ => vec![(self.x0, self.x1)],
        }
    }

    /// The complete expected stroke set `(y, x_start, x_end)`, row-major and
    /// left to right, all in canonical units.
    fn expected_strokes(&self) -> Result<Vec<(i64, i64, i64)>, CheckFailure> {
        // No positive pitch defines rows, and a zero-area region holds no
        // material, so both degenerate families have no expected strokes.
        if self.spacing_units <= 0 || !self.has_valid_geometry() {
            return Ok(Vec::new());
        }
        let mut strokes = Vec::new();
        let mut y = self.y0;
        while y < self.y1 {
            for (a, b) in self.intervals_at(y) {
                strokes.push((y, a, b));
            }
            y = y
                .checked_add(self.spacing_units)
                .ok_or_else(|| CheckFailure::Fixture("row arithmetic overflow".to_owned()))?;
        }
        Ok(strokes)
    }

    /// Rows `y0 + i * spacing` (below `y1`) that lie in the cut's `[cy0, cy1)`.
    fn split_rows(&self) -> usize {
        let Some((_, _, cy0, cy1)) = self.cut else {
            return 0;
        };
        if self.spacing_units <= 0 {
            return 0;
        }
        let mut count = 0;
        let mut y = self.y0;
        while y < self.y1 {
            if cy0 <= y && y < cy1 {
                count += 1;
            }
            y += self.spacing_units;
        }
        count
    }
}

/// Build the module through its real config path; fields stay private.
fn module_for(case: &FillCase) -> SupportSurfaceIroning {
    let mut fields = HashMap::new();
    fields.insert("ironing_enabled".to_owned(), ConfigValue::Bool(true));
    fields.insert(
        "ironing_speed".to_owned(),
        ConfigValue::Float(case.ironing_speed),
    );
    fields.insert(
        "ironing_flow_rate".to_owned(),
        ConfigValue::Float(case.flow_rate),
    );
    fields.insert(
        "ironing_spacing".to_owned(),
        ConfigValue::Float(case.spacing_mm),
    );
    fields.insert(
        "line_width".to_owned(),
        ConfigValue::Float(case.line_width_mm),
    );
    SupportSurfaceIroning::from_config(&ConfigView::from_map(fields))
        .expect("every key from_config reads is supplied")
}

fn run_kernel(case: &FillCase) -> Vec<ExtrusionPath3D> {
    module_for(case).fill_expolygon(&case.expoly(), case.z, case.speed_factor())
}

// ─────────────────────────────────────────────────────────────────────────────
// The one checker
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckFailure {
    /// The fixture itself is outside the declared domain (generator bug).
    Fixture(String),
    /// A path or point carries metadata the input did not specify.
    Metadata(String),
    /// An output coordinate is not exactly a canonical-unit value.
    OffGrid(String),
    /// A consecutive segment leaves the region (e.g. crosses a hole/notch).
    Containment(String),
    /// A fixture with expected material produced no output.
    FalseEmpty,
    /// The stroke set differs from the complete expected row/interval set.
    Intervals(String),
}

impl fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fixture(m) => write!(f, "fixture outside the domain: {m}"),
            Self::Metadata(m) => write!(f, "metadata: {m}"),
            Self::OffGrid(m) => write!(f, "off-grid coordinate: {m}"),
            Self::Containment(m) => write!(f, "containment: {m}"),
            Self::FalseEmpty => write!(f, "a fixture with material produced no ironing paths"),
            Self::Intervals(m) => write!(f, "intervals: {m}"),
        }
    }
}

#[derive(Debug, Default)]
struct FillStats {
    strokes: usize,
    segments: usize,
    rows: usize,
    split_rows: usize,
    left: usize,
    right: usize,
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

fn validate_fixture(case: &FillCase, limits: &SiLimits) -> Result<(), CheckFailure> {
    let fixture = |m: String| Err(CheckFailure::Fixture(m));
    if (case.mode == Mode::Degenerate) != case.degenerate.is_some() {
        return fixture("degenerate tag disagrees with the mode".to_owned());
    }
    if mm_to_units(case.spacing_mm as f32) != case.spacing_units {
        return fixture(format!(
            "spacing {} mm does not round to the intended {} units",
            case.spacing_mm, case.spacing_units
        ));
    }
    let spacing_degenerate = matches!(
        case.degenerate,
        Some(Degenerate::ZeroSpacing | Degenerate::NegativeSpacing | Degenerate::SubUnitSpacing)
    );
    if !spacing_degenerate
        && !(SPACING_QUANTUM_UNITS..=limits.max_spacing_units).contains(&case.spacing_units)
    {
        return fixture(format!("spacing {} units", case.spacing_units));
    }
    if !(case.line_width_mm > 0.0 && case.line_width_mm <= 2.0)
        || !(1.0..=200.0).contains(&case.flow_rate)
        || !(1.0..=300.0).contains(&case.ironing_speed)
        || !(case.z.is_finite() && case.z > 0.0)
    {
        return fixture("config value outside the manifest range".to_owned());
    }
    for v in [case.x0, case.y0, case.x1, case.y1] {
        if v.abs() > limits.max_abs_units {
            return fixture(format!("coordinate {v} exceeds max_abs_units"));
        }
    }
    if case.has_valid_geometry() {
        validate_geometry(
            &GeometryCase {
                polygons: vec![case.expoly()],
            },
            &case.geometry_domain(limits),
        )
        .map_err(|error| CheckFailure::Fixture(format!("invalid geometry: {error}")))?;
    }
    Ok(())
}

/// The one checker used by the property, the witnesses, the controls and the
/// shrink probe.
fn check_fill(
    case: &FillCase,
    limits: &SiLimits,
    paths: &[ExtrusionPath3D],
) -> Result<FillStats, CheckFailure> {
    validate_fixture(case, limits)?;
    let expected = case.expected_strokes()?;
    let expected_rows: BTreeSet<i64> = expected.iter().map(|s| s.0).collect();
    if expected_rows.len() > limits.max_rows as usize {
        return Err(CheckFailure::Fixture(format!(
            "{} expected rows exceed max_rows {}",
            expected_rows.len(),
            limits.max_rows
        )));
    }

    if expected.is_empty() && !paths.is_empty() {
        return Err(CheckFailure::Intervals(format!(
            "no material/rows are expected but {} path(s) were emitted",
            paths.len()
        )));
    }

    let width = case.line_width_mm as f32;
    let flow = case.flow_rate as f32;
    let speed = case.speed_factor();
    for path in paths {
        if path.role != ExtrusionRole::Ironing {
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
            if p.z.to_bits() != case.z.to_bits() {
                return Err(CheckFailure::Metadata(format!(
                    "z {} differs from the supplied {}",
                    p.z, case.z
                )));
            }
            if p.width.to_bits() != width.to_bits() {
                return Err(CheckFailure::Metadata(format!(
                    "width {} differs from the configured line_width {width}",
                    p.width
                )));
            }
            if p.flow_factor.to_bits() != flow.to_bits() {
                return Err(CheckFailure::Metadata(format!(
                    "flow_factor {} differs from the configured ironing_flow_rate {flow}",
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

    // Whole-segment containment of EVERY consecutive pair (ironing strokes are
    // open paths, so `windows(2)` is every edge; a closed path would repeat
    // its first point and the closing edge is a window too).
    let region = [case.expoly()];
    let mut stats = FillStats::default();
    for path in paths {
        for w in path.points.windows(2) {
            let a = Point2 {
                x: exact_units(w[0].x, "x")?,
                y: exact_units(w[0].y, "y")?,
            };
            let b = Point2 {
                x: exact_units(w[1].x, "x")?,
                y: exact_units(w[1].y, "y")?,
            };
            let inside = segment_in_region([a, b], &region, BoundaryRule::Inclusive)
                .map_err(|e| CheckFailure::Fixture(format!("containment predicate: {e}")))?;
            if !inside {
                return Err(CheckFailure::Containment(format!(
                    "segment ({}, {}) -> ({}, {}) mm leaves the region",
                    w[0].x, w[0].y, w[1].x, w[1].y
                )));
            }
            stats.segments += 1;
        }
    }

    if !expected.is_empty() && paths.is_empty() {
        return Err(CheckFailure::FalseEmpty);
    }

    let mut actual = Vec::with_capacity(paths.len());
    for path in paths {
        let [s, e] = path.points.as_slice() else {
            return Err(CheckFailure::Intervals(format!(
                "a path has {} points; each stroke is one two-point row interval",
                path.points.len()
            )));
        };
        let (sx, sy, ex, ey) = (
            exact_units(s.x, "x")?,
            exact_units(s.y, "y")?,
            exact_units(e.x, "x")?,
            exact_units(e.y, "y")?,
        );
        if sy != ey || sx >= ex {
            return Err(CheckFailure::Intervals(format!(
                "stroke ({sx},{sy}) -> ({ex},{ey}) units is not a positive left-to-right row"
            )));
        }
        actual.push((sy, sx, ex));
    }
    let mut sorted = actual.clone();
    sorted.sort_unstable();
    if sorted != expected {
        let group = |v: &[(i64, i64, i64)]| {
            let mut m: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
            for &(y, a, b) in v {
                m.entry(y).or_default().push((a, b));
            }
            m
        };
        let (want, got) = (group(&expected), group(&sorted));
        let rows: BTreeSet<i64> = want.keys().chain(got.keys()).copied().collect();
        let first = rows
            .into_iter()
            .find(|y| want.get(y) != got.get(y))
            .unwrap_or(i64::MIN);
        return Err(CheckFailure::Intervals(format!(
            "row y={first} units: expected {:?}, observed {:?} ({} expected / {} observed strokes)",
            want.get(&first),
            got.get(&first),
            expected.len(),
            sorted.len()
        )));
    }

    let per_row = {
        let mut m: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
        for &(y, a, b) in &sorted {
            m.entry(y).or_default().push((a, b));
        }
        m
    };
    stats.strokes = sorted.len();
    stats.rows = per_row.len();
    if let Some((cx0, cx1, _, _)) = case.cut {
        for intervals in per_row.values().filter(|v| v.len() >= 2) {
            stats.split_rows += 1;
            stats.left += intervals.iter().filter(|(_, b)| *b <= cx0).count();
            stats.right += intervals.iter().filter(|(a, _)| *a >= cx1).count();
        }
    }
    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy
// ─────────────────────────────────────────────────────────────────────────────

const RAW_LEN: usize = 17;

/// Inclusive `lo..=hi` pick; an empty range is a generator bug and panics
/// (a failure, never a rejection).
fn pick(raw: u32, lo: i64, hi: i64) -> i64 {
    assert!(lo <= hi, "generator range {lo}..={hi} is empty");
    lo + i64::from(raw) % (hi - lo + 1)
}

fn build_case(mode: Mode, degenerate: Degenerate, raw: [u32; RAW_LEN], l: &SiLimits) -> FillCase {
    let m = l.max_abs_units;
    let span = 2 * m;
    let spacing =
        SPACING_QUANTUM_UNITS * pick(raw[0], 1, l.max_spacing_units / SPACING_QUANTUM_UNITS);
    let row_cap = (span / spacing).min(i64::from(l.max_rows));
    let rows = pick(raw[1], MIN_ROWS, row_cap);
    // height in ((rows - 1) * spacing, rows * spacing], so exactly `rows`
    // half-open scan rows lie below the top edge.
    let height = (rows - 1) * spacing + pick(raw[2], 1, spacing);
    let width = pick(raw[3], MIN_WIDTH_UNITS, span);
    let x0 = -m + pick(raw[4], 0, span - width);
    let y0 = -m + pick(raw[5], 0, span - height);
    let (x1, mut y1) = (x0 + width, y0 + height);

    let cut = match mode {
        Mode::Rect | Mode::Degenerate => None,
        Mode::Notch | Mode::Hole => {
            let cx0 = pick(raw[6], x0 + 1, x1 - 2);
            let cx1 = pick(raw[7], cx0 + 1, x1 - 1);
            // Row index `k` in 1..=rows-2 always crosses the cut; its bottom
            // is either exactly on that row (inclusion edge) or strictly
            // between row k-1 and row k.
            let k = pick(raw[8], 1, rows - 2);
            let row_y = y0 + k * spacing;
            let cy0 = if raw[9].is_multiple_of(3) {
                row_y - pick(raw[10], 1, spacing - 1)
            } else {
                row_y
            };
            let cy1 = if mode == Mode::Notch {
                y1
            } else if raw[11].is_multiple_of(2) {
                // Top exactly on a later row: the half-open exclusion edge.
                y0 + pick(raw[12], k + 1, rows - 1) * spacing
            } else {
                pick(raw[12], row_y + 1, y1 - 1)
            };
            Some((cx0, cx1, cy0, cy1))
        }
    };

    let mut x1_final = x1;
    let mut spacing_units = spacing;
    let mut spacing_mm = spacing as f64 / 10_000.0;
    let degenerate = (mode == Mode::Degenerate).then_some(degenerate);
    match degenerate {
        Some(Degenerate::ZeroHeight) => y1 = y0,
        Some(Degenerate::ZeroWidth) => x1_final = x0,
        Some(Degenerate::ZeroSpacing) => {
            spacing_units = 0;
            spacing_mm = 0.0;
        }
        Some(Degenerate::NegativeSpacing) => {
            spacing_units = -spacing;
            spacing_mm = -spacing_mm;
        }
        Some(Degenerate::SubUnitSpacing) => {
            // 0.4 canonical units rounds to a zero kernel pitch.
            spacing_units = 0;
            spacing_mm = 0.000_04;
        }
        Some(Degenerate::ShortRing | Degenerate::EmptyContour) | None => {}
    }

    FillCase {
        mode,
        degenerate,
        x0,
        y0,
        x1: x1_final,
        y1,
        cut,
        spacing_units,
        spacing_mm,
        // (0, 2] mm in exact 1/64 mm steps.
        line_width_mm: pick(raw[13], 1, 128) as f64 / 64.0,
        // [1, 200] in exact 0.5 steps.
        flow_rate: pick(raw[14], 2, 400) as f64 / 2.0,
        ironing_speed: pick(raw[15], 1, 300) as f64,
        z: pick(raw[16], 1, 400) as f32 * 0.05,
    }
}

fn fill_case_strategy(
    limits: &SiLimits,
    modes: Vec<Mode>,
    degenerates: Vec<Degenerate>,
) -> BoxedStrategy<FillCase> {
    let limits = *limits;
    (
        prop::sample::select(modes).no_shrink(),
        prop::sample::select(degenerates).no_shrink(),
        any::<[u32; RAW_LEN]>(),
    )
        .prop_map(move |(mode, degenerate, raw)| build_case(mode, degenerate, raw, &limits))
        .boxed()
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses
// ─────────────────────────────────────────────────────────────────────────────

/// A `[0,10]²` mm witness at 1 mm pitch with explicit non-default config.
fn witness(mode: Mode, cut_mm: Option<(i64, i64, i64, i64)>) -> FillCase {
    let mm = |v: i64| v * 10_000;
    FillCase {
        mode,
        degenerate: None,
        x0: 0,
        y0: 0,
        x1: WITNESS_EXTENT_UNITS,
        y1: WITNESS_EXTENT_UNITS,
        cut: cut_mm.map(|(a, b, c, d)| (mm(a), mm(b), mm(c), mm(d))),
        spacing_units: WITNESS_SPACING_UNITS,
        spacing_mm: 1.0,
        line_width_mm: 0.4375,
        flow_rate: 87.5,
        ironing_speed: 45.0,
        z: 0.2,
    }
}

/// Charter witness: hole `[4,6]²` mm.
fn charter_hole() -> FillCase {
    witness(Mode::Hole, Some((4, 6, 4, 6)))
}

/// Asymmetric hole `[2,3] × [4,6]` mm.
fn asymmetric_hole() -> FillCase {
    witness(Mode::Hole, Some((2, 3, 4, 6)))
}

/// U shape: open-to-top notch `[4,6] × [4,10]` mm.
fn u_notch() -> FillCase {
    witness(Mode::Notch, Some((4, 6, 4, 10)))
}

fn square() -> FillCase {
    witness(Mode::Rect, None)
}

fn degenerate_witness(kind: Degenerate) -> FillCase {
    let mut case = square();
    case.mode = Mode::Degenerate;
    case.degenerate = Some(kind);
    match kind {
        Degenerate::ZeroHeight => case.y1 = case.y0,
        Degenerate::ZeroWidth => case.x1 = case.x0,
        Degenerate::ZeroSpacing => {
            case.spacing_units = 0;
            case.spacing_mm = 0.0;
        }
        Degenerate::NegativeSpacing => {
            case.spacing_units = -WITNESS_SPACING_UNITS;
            case.spacing_mm = -1.0;
        }
        Degenerate::SubUnitSpacing => {
            case.spacing_units = 0;
            case.spacing_mm = 0.000_04;
        }
        Degenerate::ShortRing | Degenerate::EmptyContour => {}
    }
    case
}

/// Strokes of the observed row `y_mm` as `(x_start, x_end)` mm.
fn row_mm(paths: &[ExtrusionPath3D], y_mm: f32) -> Vec<(f32, f32)> {
    let mut row: Vec<(f32, f32)> = paths
        .iter()
        .filter(|p| p.points.len() == 2 && p.points[0].y == y_mm)
        .map(|p| (p.points[0].x, p.points[1].x))
        .collect();
    row.sort_by(|a, b| a.0.total_cmp(&b.0));
    row
}

#[derive(Debug, Default)]
struct WitnessStats {
    charter_hole_both_sides: u64,
    asymmetric_split: u64,
    notch_split: u64,
    square_rows: u64,
    empty_modes: u64,
}

/// Every deterministic positive and empty witness, through the one checker.
fn witness_observations(limits: &SiLimits) -> Result<WitnessStats, String> {
    let mut out = WitnessStats::default();
    let checked = |case: &FillCase| -> Result<(Vec<ExtrusionPath3D>, FillStats), String> {
        let paths = run_kernel(case);
        let stats = check_fill(case, limits, &paths)
            .map_err(|e| format!("{} witness: {e}", case.mode.label()))?;
        Ok((paths, stats))
    };

    let (paths, stats) = checked(&charter_hole())?;
    if row_mm(&paths, 5.0) != vec![(0.0, 4.0), (6.0, 10.0)] || stats.left == 0 || stats.right == 0 {
        return Err(format!(
            "charter hole row y=5 mm lacks material on both sides: {:?}",
            row_mm(&paths, 5.0)
        ));
    }
    out.charter_hole_both_sides += 1;

    let (paths, stats) = checked(&asymmetric_hole())?;
    if row_mm(&paths, 5.0) != vec![(0.0, 2.0), (3.0, 10.0)] || stats.split_rows != 2 {
        return Err("asymmetric hole rows y=4,5 mm are not split".to_owned());
    }
    out.asymmetric_split += stats.split_rows as u64;

    let (paths, stats) = checked(&u_notch())?;
    if row_mm(&paths, 5.0) != vec![(0.0, 4.0), (6.0, 10.0)] || stats.split_rows != 6 {
        return Err("U notch rows y=4..9 mm are not split".to_owned());
    }
    out.notch_split += stats.split_rows as u64;

    let (_, stats) = checked(&square())?;
    if stats.rows != WITNESS_ROWS as usize || stats.strokes != WITNESS_ROWS as usize {
        return Err(format!("square witness scanned {} rows", stats.rows));
    }
    out.square_rows += stats.rows as u64;

    for kind in DEGENERATES {
        let (paths, _) = checked(&degenerate_witness(kind))?;
        if !paths.is_empty() {
            return Err(format!("{kind:?} emitted output"));
        }
        out.empty_modes += 1;
    }
    Ok(out)
}

// ─────────────────────────────────────────────────────────────────────────────
// M-SUPPORT-IRONING
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_support_ironing_fill() {
    let run = PropertyRun::new(PROPERTY_ID, PROPERTY_TEST).expect("M-SUPPORT-IRONING context");
    let limits = live_limits(&run);
    let strategy = fill_case_strategy(&limits, MODES.to_vec(), DEGENERATES.to_vec());

    let calls = Cell::new(0_u64);
    let w_charter = Cell::new(0_u64);
    let w_asym = Cell::new(0_u64);
    let w_notch = Cell::new(0_u64);
    let w_square = Cell::new(0_u64);
    let w_empty = Cell::new(0_u64);
    let strokes = Cell::new(0_u64);
    let segments = Cell::new(0_u64);
    let split_rows = Cell::new(0_u64);
    let per_mode: [Cell<u64>; 4] = Default::default();
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);

        // Deterministic positives, constructed inside the callback.
        let w = match witness_observations(&limits) {
            Ok(w) => w,
            Err(message) => return fail(message),
        };
        w_charter.set(w_charter.get() + w.charter_hole_both_sides);
        w_asym.set(w_asym.get() + w.asymmetric_split);
        w_notch.set(w_notch.get() + w.notch_split);
        w_square.set(w_square.get() + w.square_rows);
        w_empty.set(w_empty.get() + w.empty_modes);

        let paths = run_kernel(&case);
        let stats = match check_fill(&case, &limits, &paths) {
            Ok(stats) => stats,
            Err(e) => return fail(format!("{} case failed: {e}", case.mode.label())),
        };
        match case.mode {
            Mode::Notch | Mode::Hole => {
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
            Mode::Rect => {
                if stats.strokes == 0 {
                    return fail("rect case produced no strokes");
                }
            }
            Mode::Degenerate => {
                if !paths.is_empty() {
                    return fail("degenerate case produced output");
                }
            }
        }
        let slot = MODES.iter().position(|m| *m == case.mode).expect("mode");
        per_mode[slot].set(per_mode[slot].get() + 1);
        strokes.set(strokes.get() + stats.strokes as u64);
        segments.set(segments.get() + stats.segments as u64);
        split_rows.set(split_rows.get() + stats.split_rows as u64);
        Ok(())
    })
    .expect("M-SUPPORT-IRONING complete rows, whole-segment containment and metadata");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(w_charter.get() > 0, "charter hole witness unobserved");
    assert!(w_asym.get() > 0, "asymmetric hole split rows unobserved");
    assert!(w_notch.get() > 0, "notch split rows unobserved");
    assert!(w_square.get() > 0, "square rows unobserved");
    assert!(w_empty.get() > 0, "degenerate empty modes unobserved");
    println!(
        "M-SUPPORT-IRONING accepted_calls={} rect={} notch={} hole={} degenerate={} \
         strokes={} segments={} split_rows={} witness_charter={} witness_asym_split={} \
         witness_notch_split={} witness_square_rows={} witness_empty={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        strokes.get(),
        segments.get(),
        split_rows.get(),
        w_charter.get(),
        w_asym.get(),
        w_notch.get(),
        w_square.get(),
        w_empty.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic mutation controls
// ─────────────────────────────────────────────────────────────────────────────

/// A stroke at `y_mm` from `xs_mm` to `xe_mm` with the metadata of a real
/// emitted path, so the only defect a control introduces is geometric.
fn stroke_like(template: &ExtrusionPath3D, xs_mm: f32, xe_mm: f32, y_mm: f32) -> ExtrusionPath3D {
    let mut path = template.clone();
    path.points = vec![
        Point3WithWidth {
            x: xs_mm,
            y: y_mm,
            ..template.points[0]
        },
        Point3WithWidth {
            x: xe_mm,
            y: y_mm,
            ..template.points[1]
        },
    ];
    path
}

fn without_row(paths: &[ExtrusionPath3D], y_mm: f32) -> Vec<ExtrusionPath3D> {
    paths
        .iter()
        .filter(|p| p.points[0].y != y_mm)
        .cloned()
        .collect()
}

fn expect_failure(
    case: &FillCase,
    limits: &SiLimits,
    paths: &[ExtrusionPath3D],
    what: &str,
    matches_class: impl Fn(&CheckFailure) -> bool,
) {
    match check_fill(case, limits, paths) {
        Ok(_) => panic!("{what}: the mutation was accepted"),
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

#[test]
fn support_ironing_controls() {
    let limits = checked_limits(&probe_bound_map()).expect("probe bounds");

    // ── Exact asymmetric hole: (1,5) -> (9,5) mm ───────────────────────────
    let hole = asymmetric_hole();
    let hole_paths = run_kernel(&hole);
    let stats = check_fill(&hole, &limits, &hole_paths).expect("unmodified asymmetric hole");
    assert_eq!(stats.split_rows, 2, "rows y=4 and y=5 mm cross [4,6) mm");
    assert_eq!(row_mm(&hole_paths, 5.0), vec![(0.0, 2.0), (3.0, 10.0)]);
    // The naive point checks all pass for the shortcut, which is why the
    // whole-segment oracle is required.
    let region = [hole.expoly()];
    for (x, y) in [(1.0, 5.0), (9.0, 5.0), (5.0, 5.0)] {
        assert_eq!(
            point_in_region(Point2::from_mm(x, y), &region).expect("point oracle"),
            PointLocation::Inside,
            "({x},{y}) mm must be material"
        );
    }
    let template = hole_paths[0].clone();
    let shortcut = stroke_like(&template, 1.0, 9.0, 5.0);
    let mut replaced = without_row(&hole_paths, 5.0);
    replaced.push(shortcut.clone());
    expect_failure(
        &hole,
        &limits,
        &replaced,
        "unsplit (1,5)->(9,5) row",
        is_containment,
    );
    let mut appended = hole_paths.clone();
    appended.push(shortcut);
    expect_failure(
        &hole,
        &limits,
        &appended,
        "appended (1,5)->(9,5)",
        is_containment,
    );

    // ── U notch chord (2,5) -> (8,5) mm ────────────────────────────────────
    let notch = u_notch();
    let notch_paths = run_kernel(&notch);
    let stats = check_fill(&notch, &limits, &notch_paths).expect("unmodified U notch");
    assert!(stats.split_rows > 0 && stats.left > 0 && stats.right > 0);
    let chord = stroke_like(&notch_paths[0], 2.0, 8.0, 5.0);
    let mut appended = notch_paths.clone();
    appended.push(chord.clone());
    expect_failure(&notch, &limits, &appended, "notch chord", is_containment);
    let mut replaced = without_row(&notch_paths, 5.0);
    replaced.push(chord);
    expect_failure(
        &notch,
        &limits,
        &replaced,
        "row replaced by notch chord",
        is_containment,
    );

    // ── Plain 10 mm square, 1 mm pitch: dropped row and false empty ────────
    let sq = square();
    let sq_paths = run_kernel(&sq);
    let stats = check_fill(&sq, &limits, &sq_paths).expect("unmodified square");
    assert_eq!(stats.rows, 10);
    assert!(sq_paths.iter().any(|p| p.points[0].y == 1.0));
    let dropped = without_row(&sq_paths, 1.0);
    assert_eq!(dropped.len(), sq_paths.len() - 1, "exactly the y=1 row");
    expect_failure(&sq, &limits, &dropped, "dropped y=1 row", is_intervals);
    expect_failure(&sq, &limits, &[], "false all-empty output", |e| {
        *e == CheckFailure::FalseEmpty
    });
    // A row at the excluded top edge y=10 mm lies on the boundary, so only
    // the complete row set can reject it.
    let mut extra = sq_paths.clone();
    extra.push(stroke_like(&sq_paths[0], 0.0, 10.0, 10.0));
    expect_failure(
        &sq,
        &limits,
        &extra,
        "row on the open top edge",
        is_intervals,
    );

    // ── Charter hole [4,6]²: material on both sides at y = 5 mm ───────────
    let charter = charter_hole();
    let charter_paths = run_kernel(&charter);
    let stats = check_fill(&charter, &limits, &charter_paths).expect("charter hole");
    assert!(stats.left > 0 && stats.right > 0);
    assert_eq!(row_mm(&charter_paths, 5.0), vec![(0.0, 4.0), (6.0, 10.0)]);
    // The hole's top edge y=6 is excluded, its bottom edge y=4 included.
    assert_eq!(row_mm(&charter_paths, 4.0), vec![(0.0, 4.0), (6.0, 10.0)]);
    assert_eq!(row_mm(&charter_paths, 6.0), vec![(0.0, 10.0)]);

    // ── Metadata propagation mutations ─────────────────────────────────────
    type Mutate = fn(&mut ExtrusionPath3D);
    let mutations: [(&str, Mutate); 7] = [
        ("flow", |p| p.points[0].flow_factor += 1.0),
        ("width", |p| p.points[1].width += 0.25),
        ("z", |p| p.points[0].z += 0.5),
        ("speed", |p| p.speed_factor += 0.5),
        ("role", |p| p.role = ExtrusionRole::SupportMaterial),
        ("tool", |p| p.tool_index = Some(1)),
        ("overhang", |p| p.points[1].overhang_quartile = Some(2)),
    ];
    for (what, mutate) in mutations {
        let mut mutant = charter_paths.clone();
        mutate(&mut mutant[0]);
        expect_failure(&charter, &limits, &mutant, what, is_metadata);
    }
    // The configured values really are what the output carries.
    assert!(charter_paths.iter().all(|p| p
        .points
        .iter()
        .all(|q| q.flow_factor == 87.5 && q.width == 0.4375 && q.z == 0.2)
        && p.speed_factor == 45.0 / 50.0));

    // ── Degenerate modes: actual empty outcome; nonempty output rejected ───
    for kind in DEGENERATES {
        let case = degenerate_witness(kind);
        let paths = run_kernel(&case);
        assert!(paths.is_empty(), "{kind:?}: the kernel emitted output");
        check_fill(&case, &limits, &paths).expect("empty degenerate output is accepted");
        expect_failure(
            &case,
            &limits,
            &sq_paths[..1],
            &format!("{kind:?} with output"),
            is_intervals,
        );
    }

    // ── Bound-construction rejection matrix ────────────────────────────────
    fn invalid(label: &str, bounds: BTreeMap<String, u64>) {
        match checked_limits(&bounds) {
            Err(DomainError::InvalidBounds(message)) => assert!(!message.is_empty(), "{label}"),
            other => panic!("{label}: expected InvalidBounds, got {other:?}"),
        }
    }
    let valid = probe_bound_map();
    for key in BOUND_KEYS {
        let mut bounds = valid.clone();
        bounds.remove(key);
        invalid(&format!("missing {key}"), bounds);
    }
    for (key, value) in [
        ("max_abs_units", 99_999),
        ("max_abs_units", 1_000_001),
        ("max_spacing_units", 9_900),
        ("max_spacing_units", 10_050),
        ("max_spacing_units", 10_100),
        ("max_rows", 9),
        ("max_rows", 1_025),
        ("max_vertices", 7),
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
    let mut oversized = valid;
    oversized.insert("max_rows".to_owned(), u64::from(u32::MAX) + 1);
    assert!(matches!(
        checked_limits(&oversized),
        Err(DomainError::ArithmeticOverflow)
    ));

    // The witnesses themselves are inside the smallest bounds.
    witness_observations(&limits).expect("all witnesses inside the probe bounds");
    println!("support_ironing_controls: all mutations rejected by the shared checker");
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probe
// ─────────────────────────────────────────────────────────────────────────────

/// Independently validate every exposed input of a generated value and run it
/// through the same checker.
fn check_shrink_state(case: &FillCase, mode: Mode, limits: &SiLimits) {
    assert_eq!(case.mode, mode, "the mode must survive shrinking");
    assert_eq!(case.degenerate.is_some(), mode == Mode::Degenerate);
    for v in [case.x0, case.y0, case.x1, case.y1] {
        assert!(
            v.abs() <= limits.max_abs_units,
            "coordinate {v} out of bounds"
        );
    }
    assert!(case.line_width_mm > 0.0 && case.line_width_mm <= 2.0);
    assert!((1.0..=200.0).contains(&case.flow_rate));
    assert!((1.0..=300.0).contains(&case.ironing_speed));
    assert!(case.z > 0.0 && case.z.is_finite());
    if mode != Mode::Degenerate {
        assert!(case.x1 - case.x0 >= MIN_WIDTH_UNITS);
        assert_eq!(case.spacing_units % SPACING_QUANTUM_UNITS, 0);
        assert!((SPACING_QUANTUM_UNITS..=limits.max_spacing_units).contains(&case.spacing_units));
        // Count the half-open rows `y0 + i * spacing < y1` directly.
        let mut rows = 0_i64;
        let mut y = case.y0;
        while y < case.y1 {
            rows += 1;
            y += case.spacing_units;
        }
        assert!(
            (MIN_ROWS..=i64::from(limits.max_rows)).contains(&rows),
            "row count {rows} outside {MIN_ROWS}..={}",
            limits.max_rows
        );
    }
    match mode {
        Mode::Rect | Mode::Degenerate => assert!(case.cut.is_none()),
        Mode::Notch | Mode::Hole => {
            let (cx0, cx1, cy0, cy1) = case.cut.expect("cut present");
            assert!(case.x0 < cx0 && cx0 < cx1 && cx1 < case.x1, "cut x inside");
            assert!(case.y0 < cy0 && cy0 < cy1, "cut y ordered and above y0");
            if mode == Mode::Notch {
                assert_eq!(cy1, case.y1, "the notch opens through the top edge");
            } else {
                assert!(cy1 < case.y1, "the hole stays below the top edge");
            }
            assert!(
                case.split_rows() > 0,
                "witness lost: no scan row crosses the cut"
            );
        }
    }
    let paths = run_kernel(case);
    let stats = check_fill(case, limits, &paths)
        .unwrap_or_else(|e| panic!("shrunk {} fixture: {e}", mode.label()));
    match mode {
        Mode::Degenerate => assert!(paths.is_empty()),
        Mode::Rect => assert!(stats.strokes > 0),
        Mode::Notch | Mode::Hole => {
            assert!(stats.split_rows > 0 && stats.left > 0 && stats.right > 0);
        }
    }
}

fn complexity(case: &FillCase, limits: &SiLimits) -> i128 {
    let m = limits.max_abs_units;
    let cut = case.cut.map_or(0, |(a, b, c, d)| {
        (b - a) + (d - c) + (a - case.x0) + (c - case.y0)
    });
    i128::from(
        (case.x1 - case.x0)
            + (case.y1 - case.y0)
            + (case.x0 + m)
            + (case.y0 + m)
            + case.spacing_units.abs()
            + cut,
    ) + (case.line_width_mm * 64.0) as i128
        + (case.flow_rate * 2.0) as i128
        + case.ironing_speed as i128
        + (case.z * 20.0).round() as i128
}

#[test]
fn support_ironing_shrink_bounds() {
    let limits = checked_limits(&probe_bound_map()).expect("shrink bounds");
    for mode in MODES {
        // Degenerate kind is fixed per run so the probe is deterministic.
        let degenerates = vec![Degenerate::NegativeSpacing];
        let strategy = fill_case_strategy(&limits, vec![mode], degenerates);

        // Phase 1: accept every simplification (as if each still failed) and
        // demonstrate a real reduction from a nonminimal start.
        let mut runner = TestRunner::deterministic();
        let mut tree = strategy.new_tree(&mut runner).expect("shrink tree");
        let initial = tree.current();
        check_shrink_state(&initial, mode, &limits);
        let initial_complexity = complexity(&initial, &limits);
        let mut accepted = 0_usize;
        while tree.simplify() {
            accepted += 1;
            assert!(
                accepted < MAX_SHRINK_STEPS,
                "{mode:?}: simplify never exhausted"
            );
            check_shrink_state(&tree.current(), mode, &limits);
        }
        let minimal = tree.current();
        let minimal_complexity = complexity(&minimal, &limits);
        assert!(accepted > 0, "false-only shrink probe for {mode:?}");
        assert!(
            minimal_complexity < initial_complexity,
            "{mode:?}: no permitted reduction ({initial_complexity} -> {minimal_complexity})"
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
                "{mode:?}: alternation never ended"
            );
            let current = tree.current();
            check_shrink_state(&current, mode, &limits);
            changed |= current != initial;
            if tree.complicate() {
                complicated += 1;
            }
            check_shrink_state(&tree.current(), mode, &limits);
        }
        assert!(simplified > 0 && changed, "{mode:?}: no real transition");
        let exhausted = tree.current();
        assert!(
            !tree.simplify(),
            "an exhausted {mode:?} tree simplified again"
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
                "{mode:?}: complicate never ended"
            );
            check_shrink_state(&tree.current(), mode, &limits);
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
            "mode={mode:?} phase1_simplify={accepted} complexity {initial_complexity}->{minimal_complexity} \
             phase2_simplify={simplified} complicate={complicated} restores={restores}"
        );
    }

    // The mode and degenerate selectors themselves never shrink: a shrinking
    // mode tree could switch a witness's class mid-search.
    let mut runner = TestRunner::deterministic();
    let modes = prop::sample::select(MODES.to_vec()).no_shrink();
    let mut tree = modes.new_tree(&mut runner).expect("mode tree");
    let before = tree.current();
    assert!(!tree.simplify(), "the mode selector simplified");
    assert!(!tree.complicate(), "the mode selector complicated");
    assert_eq!(tree.current(), before);
    let kinds = prop::sample::select(DEGENERATES.to_vec()).no_shrink();
    let mut tree = kinds.new_tree(&mut runner).expect("degenerate tree");
    let before = tree.current();
    assert!(!tree.simplify(), "the degenerate selector simplified");
    assert!(!tree.complicate(), "the degenerate selector complicated");
    assert_eq!(tree.current(), before);
}
