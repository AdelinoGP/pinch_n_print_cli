//! Opt-in property tests for this module's private traditional support fill kernel.
//!
//! Charter (`docs/spec_packets/proptest-openrscad_14_pbt-support-modules`):
//!
//! * `M-TRADITIONAL-SCAN` — `TraditionalSupport::fill_expolygon` on
//!   independently valid rectangles, an open-to-top U-notch and a rectangular
//!   hole. The checker compares the **complete** half-open row and material
//!   interval set, checks **every** consecutive segment of every returned
//!   `ExtrusionPath3D` for whole-segment containment, and inspects the supplied
//!   Z, speed factor, path width and the unset tool/order fields. A narrow
//!   translated tip, a chain-less interface fallback and a set of degenerate
//!   inputs are separate deterministic modes whose *actual* kernel outcome is
//!   asserted.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `fill_expolygon` is private, so the properties live here as a crate-internal
//! unit-test module; production visibility is not widened. The module is
//! constructed only through `TraditionalSupport::from_config`.
//!
//! # Independent expected model
//!
//! Fixtures are axis-aligned and describe themselves as rectangles: an outer
//! `[x0,x1] x [y0,y1]` and an optional cut `[cx0,cx1] x [cy0,cy1]`. At angle
//! zero the kernel scans rows at `y0 + n * spacing` while `< y1` (the outer top
//! edge is excluded) and pairs crossings with a half-open interval
//! `[cy0, cy1)` for the cut, so the expected intervals are pure rectangle
//! arithmetic over the fixture's own fields. Nothing in this model calls the
//! kernel twice as its own oracle.
//!
//! Rotated modes use exact quarter turns only (`0/90/180/270` degrees), so
//! `rotate_point`'s two `f64` roundings are exactly representable and the
//! derived rounding envelope is **zero** rather than an observed epsilon. Every
//! emitted point is mapped back into the kernel's bbox-centre-relative scan
//! frame and compared against the same complete row-and-interval model there,
//! which quarter turns keep axis-aligned — an exact integer comparison, so a
//! split row that loses one arm fails. Cut-carrying rotated cases are the
//! 180-degree turn; the odd turns carry no cut and exercise the axis swap alone.
//!
//! # Confirmed defect and focused fix
//!
//! `fill_expolygon` rotates every edge into bbox-centre-relative coordinates
//! once, scans there, and adds `refpt_x`/`refpt_y` back when it converts a
//! crossing to millimetres. The centroid fallback — the branch reached when the
//! scan loop emits no pair — did the same rotation but omitted that
//! translation, so on any polygon whose bbox centre is not the origin it
//! emitted a band around the *relative* centroid, i.e. outside the polygon. It
//! also ran before the interface-contour branch, so its bogus output suppressed
//! a correct printable fallback. On the downward-pointing triangle
//! `(5,4), (6,4.5), (4,4.5)` mm at angle zero with a 2 mm pitch, the required
//! band is `(4.5, 4.25) -> (5.5, 4.25)` mm and the pre-fix kernel emitted
//! `(-0.5, 0) -> (0.5, 0)` mm. The fallback now adds the same translation the
//! scan-line branch adds, which restores the documented behaviour rather than
//! changing it. The no-RNG regression is
//! `narrow_translated_tip_fallback_is_placed_at_the_bbox_centre_regression` in
//! this crate's `#[cfg(test)] mod tests`.
//!
//! The packet's sub-pitch diamond `(5,4.5), (5.5,5), (5,5.5), (4.5,5)` mm is
//! the separate `Interface` mode below: both of its single row's crossings
//! coincide at the tip, and no edge strictly straddles the centroid row, so
//! both the scan and the centroid fallback emit nothing and the interface
//! contour fallback is what keeps the layer printable. That empty-by-design
//! outcome is asserted as such and is not a defect.
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
//!   one literal each in `TsCase::geometry_domain`, both carrying an
//!   `// exhaustive:` note because every field is derived from the live limits
//!   or the fixture's topology class.
//! * `TsCase`, `TsLimits`, `FillStats`, `CheckFailure` and `ShapeKind` are
//!   private module-local types, not watched crate-public structs.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! controls are mutation controls over the SAME `check_fill` the property uses,
//! each asserting the specific `CheckFailure` class, so none is a decorative or
//! success-only assertion; the property requires positive per-subcase counters
//! from deterministic witnesses built inside its callback.

#![allow(missing_docs)]
// The pinned toolchain has `is_multiple_of` only for unsigned integers and no
// `div_ceil` for `i64` (E0658 `int_roundings`), so the remainder forms stay and
// the suggested rewrites are suppressed rather than silently disabled.
#![allow(clippy::manual_is_multiple_of, clippy::manual_div_ceil)]

use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt;

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    mm_to_units, units_to_mm, ConfigValue, ConfigView, ExPolygon, ExtrusionPath3D, ExtrusionRole,
    Point3WithWidth, Polygon,
};
use slicer_sdk::test_support::property_testing::domains::{
    BoundaryRule, DomainError, GeometryBounds, GeometryCase, GeometryDomain, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::{
    point_in_region, segment_in_region, validate_geometry, PointLocation,
};
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_sdk::traits::LayerModule;

use super::TraditionalSupport;

const PROPERTY_ID: &str = "M-TRADITIONAL-SCAN";
const PROPERTY_TEST: &str = "property_tests::proptest_traditional_support_fill";
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
/// Smallest generated width, in grid steps (1 mm).
const MIN_WIDTH_STEPS: i64 = 4;
/// Smallest row count a generated shape may have.
const MIN_ROWS: i64 = 1;
/// The kernel drops a pair whose endpoints coincide.
const MIN_INTERVAL_UNITS: i64 = 1;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

// Every key is mandatory rather than silently optional: `checked_limits`
// rejects a missing key, so a profile that forgot one fails instead of running
// with a default.
#[derive(Debug, Clone, Copy)]
struct TsLimits {
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

fn checked_limits(bounds: &BTreeMap<String, u64>) -> Result<TsLimits, DomainError> {
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

    if max_abs_units < 8 * GRID_UNITS || max_abs_units % GRID_UNITS != 0 {
        return Err(DomainError::InvalidBounds(
            "max_abs_units must be a positive multiple of the 0.25 mm fixture grid and at \
             least 2 mm"
                .to_owned(),
        ));
    }
    if max_spacing_units < GRID_UNITS || max_spacing_units % GRID_UNITS != 0 {
        return Err(DomainError::InvalidBounds(
            "max_spacing_units must be a positive multiple of the 0.25 mm fixture grid".to_owned(),
        ));
    }
    if !(4..=64).contains(&max_rows) {
        return Err(DomainError::InvalidBounds(
            "max_rows must be in 4..=64: at least four rows are needed to place a cut that a \
             scan row really crosses, and the derived per-row model assumes at most 64"
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
    Ok(TsLimits {
        max_abs_units,
        max_vertices,
        max_holes,
        max_components,
        max_shrink_candidates,
        max_spacing_units,
        max_rows,
    })
}

fn live_limits(run: &PropertyRun) -> TsLimits {
    let mut bounds = BTreeMap::new();
    for key in BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("traditional scan bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_limits(&bounds).expect("traditional scan bound consistency")
}

fn probe_bound_map() -> BTreeMap<String, u64> {
    BTreeMap::from([
        ("max_abs_units".to_owned(), 200_000),
        ("max_vertices".to_owned(), 16),
        ("max_holes".to_owned(), 2),
        ("max_components".to_owned(), 1),
        ("max_shrink_candidates".to_owned(), 1),
        ("max_spacing_units".to_owned(), 10_000),
        ("max_rows".to_owned(), 16),
    ])
}

// ─────────────────────────────────────────────────────────────────────────────
// Modes and fixtures
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeKind {
    Rect,
    Notch,
    Hole,
    Rotated,
    Narrow,
    Interface,
    Degenerate,
}

impl ShapeKind {
    fn label(self) -> &'static str {
        match self {
            ShapeKind::Rect => "rect",
            ShapeKind::Notch => "notch",
            ShapeKind::Hole => "hole",
            ShapeKind::Rotated => "rotated",
            ShapeKind::Narrow => "narrow",
            ShapeKind::Interface => "interface",
            ShapeKind::Degenerate => "degenerate",
        }
    }

    /// `true` when the fixture is described by `x0..y1` and an optional cut.
    fn is_rectangle(self) -> bool {
        matches!(
            self,
            ShapeKind::Rect | ShapeKind::Notch | ShapeKind::Hole | ShapeKind::Rotated
        )
    }
}

/// A degenerate input class with its specified no-work outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DegenerateKind {
    ZeroHeight,
    ZeroWidth,
    ShortRing,
    EmptyContour,
    ZeroSpacing,
    NegativeSpacing,
}

#[derive(Debug, Clone, Copy)]
struct TsCase {
    shape: ShapeKind,
    /// Outer rectangle, canonical units.
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    /// Cut rectangle `(cx0, cy0, cx1, cy1)`, canonical units. A notch is open
    /// through the top edge, so its `cy1` equals `y1`.
    cut: Option<(i64, i64, i64, i64)>,
    /// Quarter turns for the rotated mode, `0..=3`.
    quarter_turns: u8,
    spacing_units: i64,
    z: f32,
    speed_factor: f32,
    path_width_mm: f32,
    interface: bool,
    degenerate: Option<DegenerateKind>,
}

impl TsCase {
    fn polygon(&self) -> ExPolygon {
        match self.shape {
            ShapeKind::Narrow => triangle(5.0, 4.0, 6.0, 4.5, 4.0, 4.5),
            ShapeKind::Interface => diamond(),
            ShapeKind::Degenerate => match self.degenerate {
                Some(DegenerateKind::ShortRing) => ExPolygon {
                    contour: Polygon {
                        points: vec![
                            slicer_ir::Point2::from_mm(0.0, 0.0),
                            slicer_ir::Point2::from_mm(1.0, 1.0),
                        ],
                    },
                    holes: vec![],
                },
                Some(DegenerateKind::EmptyContour) => ExPolygon {
                    contour: Polygon { points: vec![] },
                    holes: vec![],
                },
                _ => rect_polygon(self.x0, self.y0, self.x1, self.y1, None, false),
            },
            _ => rect_polygon(
                self.x0,
                self.y0,
                self.x1,
                self.y1,
                self.cut,
                self.shape == ShapeKind::Notch,
            ),
        }
    }

    /// Independent input validation: the fixture must be a valid geometry case
    /// inside the live bounds before the kernel runs.
    fn geometry_domain(&self, limits: &TsLimits) -> GeometryDomain {
        let (class, holes) = match self.shape {
            ShapeKind::Rect => (TopologyClass::Convex, 0),
            // Odd quarter turns carry no cut, so the class follows the fixture.
            ShapeKind::Rotated => {
                if self.cut.is_some() {
                    (TopologyClass::Holed, 1)
                } else {
                    (TopologyClass::Convex, 0)
                }
            }
            ShapeKind::Notch => (TopologyClass::Concave, 0),
            ShapeKind::Hole => (TopologyClass::Holed, 1),
            _ => (TopologyClass::Convex, 0),
        };
        // exhaustive: every bound is the live profile limit and every class
        // field is derived from the fixture's topology, so there is nothing to
        // default.
        GeometryDomain {
            class,
            min_holes: holes,
            min_components: 1,
            boundary: BoundaryRule::Inclusive,
            // exhaustive: all five fields come straight from the live profile.
            bounds: GeometryBounds {
                max_abs_units: limits.max_abs_units,
                max_vertices: limits.max_vertices as usize,
                max_holes: limits.max_holes as usize,
                max_components: limits.max_components as usize,
                max_shrink_candidates: limits.max_shrink_candidates,
            },
        }
    }

    fn validate(&self, limits: &TsLimits) -> Result<(), String> {
        for v in [self.x0, self.y0, self.x1, self.y1] {
            if v.abs() > limits.max_abs_units {
                return Err(format!("coordinate {v} outside the profile bound"));
            }
        }
        if !self.z.is_finite() || self.z <= 0.0 {
            return Err("z must be positive and finite".to_owned());
        }
        if !self.speed_factor.is_finite() || self.speed_factor <= 0.0 {
            return Err("speed factor must be positive and finite".to_owned());
        }
        if !self.path_width_mm.is_finite() || self.path_width_mm <= 0.0 {
            return Err("path width must be positive and finite".to_owned());
        }
        if self.shape.is_rectangle() {
            if self.x1 - self.x0 < MIN_INTERVAL_UNITS || self.y1 - self.y0 < MIN_INTERVAL_UNITS {
                return Err("rectangle must have positive extent".to_owned());
            }
            if self.spacing_units < GRID_UNITS || self.spacing_units.rem_euclid(GRID_UNITS) != 0 {
                return Err("spacing must be a positive grid multiple".to_owned());
            }
            if self.spacing_units > limits.max_spacing_units {
                return Err("spacing outside the profile bound".to_owned());
            }
            let mut rows = 0_i64;
            let mut y = self.y0;
            while y < self.y1 {
                rows += 1;
                y = y
                    .checked_add(self.spacing_units)
                    .ok_or_else(|| "row arithmetic overflowed".to_owned())?;
            }
            if rows < MIN_ROWS || rows > i64::from(limits.max_rows) {
                return Err(format!("row count {rows} outside the profile bound"));
            }
            if self.shape == ShapeKind::Rotated {
                // A quarter turn swaps the extents, so the scan-frame row count
                // is what the profile has to admit, not the unrotated one.
                let scanned = expected_rotated_rows(self).len();
                if scanned < MIN_ROWS as usize || scanned > limits.max_rows as usize {
                    return Err(format!(
                        "rotated scan-frame row count {scanned} outside the profile bound"
                    ));
                }
            }
            if let Some((cx0, cy0, cx1, cy1)) = self.cut {
                if !(self.x0 < cx0 && cx0 < cx1 && cx1 < self.x1) {
                    return Err("the cut must sit strictly inside the outer x extent".to_owned());
                }
                if !(self.y0 < cy0 && cy0 < cy1) {
                    return Err("the cut must sit above the outer bottom edge".to_owned());
                }
                if self.shape == ShapeKind::Notch {
                    if cy1 != self.y1 {
                        return Err("the notch must open through the top edge".to_owned());
                    }
                } else if cy1 >= self.y1 {
                    return Err("the hole must stay below the top edge".to_owned());
                }
                if self.shape != ShapeKind::Rotated && self.split_rows() == 0 {
                    return Err("witness lost: no scan row crosses the cut".to_owned());
                }
            }
        }
        if self.shape == ShapeKind::Degenerate {
            // These fixtures are deliberately not valid geometry; their class
            // documents why, and the assertion is the kernel's no-work outcome.
            return if self.degenerate.is_none() {
                Err("a degenerate case must name its class".to_owned())
            } else {
                Ok(())
            };
        }
        // The fixture itself must be independently valid geometry.
        let case = GeometryCase {
            polygons: vec![self.polygon()],
        };
        validate_geometry(&case, &self.geometry_domain(limits))
            .map_err(|e| format!("fixture is not valid geometry: {e}"))?;
        Ok(())
    }

    /// Number of half-open scan rows that cross the cut.
    fn split_rows(&self) -> usize {
        let Some((_, cy0, _, cy1)) = self.cut else {
            return 0;
        };
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

    fn run_angle(&self) -> (f64, f64) {
        match self.quarter_turns % 4 {
            0 => (1.0, 0.0),
            1 => (0.0, 1.0),
            2 => (-1.0, 0.0),
            _ => (0.0, -1.0),
        }
    }
}

fn mm(v: f32) -> i64 {
    mm_to_units(v)
}

fn rect_polygon(
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
    cut: Option<(i64, i64, i64, i64)>,
    notch: bool,
) -> ExPolygon {
    let p = |x: i64, y: i64| slicer_ir::Point2 { x, y };
    let Some((cx0, cy0, cx1, cy1)) = cut else {
        return ExPolygon {
            contour: Polygon {
                points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)],
            },
            holes: vec![],
        };
    };
    if notch {
        // Counter-clockwise U: the cut is a bay cut down from the top edge.
        ExPolygon {
            contour: Polygon {
                points: vec![
                    p(x0, y0),
                    p(x1, y0),
                    p(x1, y1),
                    p(cx1, y1),
                    p(cx1, cy0),
                    p(cx0, cy0),
                    p(cx0, y1),
                    p(x0, y1),
                ],
            },
            holes: vec![],
        }
    } else {
        ExPolygon {
            contour: Polygon {
                points: vec![p(x0, y0), p(x1, y0), p(x1, y1), p(x0, y1)],
            },
            holes: vec![Polygon {
                // Clockwise hole, as the IR convention expects.
                points: vec![p(cx0, cy0), p(cx0, cy1), p(cx1, cy1), p(cx1, cy0)],
            }],
        }
    }
}

fn triangle(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                slicer_ir::Point2::from_mm(ax, ay),
                slicer_ir::Point2::from_mm(bx, by),
                slicer_ir::Point2::from_mm(cx, cy),
            ],
        },
        holes: vec![],
    }
}

fn diamond() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                slicer_ir::Point2::from_mm(5.0, 4.5),
                slicer_ir::Point2::from_mm(5.5, 5.0),
                slicer_ir::Point2::from_mm(5.0, 5.5),
                slicer_ir::Point2::from_mm(4.5, 5.0),
            ],
        },
        holes: vec![],
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Kernel driver
// ─────────────────────────────────────────────────────────────────────────────

fn module() -> TraditionalSupport {
    let mut map = std::collections::HashMap::new();
    map.insert("nozzle_diameter".to_string(), ConfigValue::Float(0.4));
    map.insert("layer_height".to_string(), ConfigValue::Float(0.2));
    map.insert(
        "support_base_pattern_spacing".to_string(),
        ConfigValue::Float(2.5),
    );
    map.insert("enable_support".to_string(), ConfigValue::Bool(false));
    map.insert("support_angle".to_string(), ConfigValue::Float(60.0));
    map.insert("support_speed".to_string(), ConfigValue::Float(50.0));
    map.insert("line_width".to_string(), ConfigValue::Float(0.4));
    map.insert(
        "support_interface_flow".to_string(),
        ConfigValue::Percent(100.0),
    );
    map.insert(
        "support_interface_spacing".to_string(),
        ConfigValue::Float(0.4),
    );
    map.insert(
        "support_bottom_interface_spacing".to_string(),
        ConfigValue::Float(0.4),
    );
    map.insert(
        "support_style".to_string(),
        ConfigValue::String("default".to_string()),
    );
    TraditionalSupport::from_config(&ConfigView::from_map(map)).expect("seeded traditional config")
}

fn run_kernel(case: &TsCase) -> Vec<ExtrusionPath3D> {
    let (cos_a, sin_a) = case.run_angle();
    module().fill_expolygon(
        &case.polygon(),
        case.spacing_units,
        cos_a,
        sin_a,
        case.z,
        case.speed_factor,
        case.interface,
        case.path_width_mm,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// The checker
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum CheckFailure {
    WrongRole,
    WrongSpeed,
    WrongWidth,
    WrongFlow,
    WrongZ,
    StrayToolOrLock,
    StrayOverhang,
    PointCount {
        path: usize,
        points: usize,
    },
    NotContained {
        path: usize,
        from: (i64, i64),
        to: (i64, i64),
    },
    Rows {
        expected: usize,
        actual: usize,
    },
    Intervals {
        row: i64,
        expected: Vec<(i64, i64)>,
        actual: Vec<(i64, i64)>,
    },
    MissingSplitRow,
    ExpectedEmpty {
        strokes: usize,
    },
    ExpectedNonempty,
    Fallback {
        expected: Vec<(i64, i64, i64)>,
        actual: Vec<(i64, i64, i64)>,
    },
    InterfaceContour,
    CoordinateNotOnGrid {
        path: usize,
    },
    FixtureInvalid(String),
}

impl fmt::Display for CheckFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckFailure::WrongRole => write!(f, "a path does not carry SupportMaterial"),
            CheckFailure::WrongSpeed => {
                write!(f, "a path does not carry the supplied speed factor")
            }
            CheckFailure::WrongWidth => write!(f, "a path does not carry the supplied width"),
            CheckFailure::WrongFlow => write!(f, "a path does not carry flow_factor 1.0"),
            CheckFailure::WrongZ => write!(f, "a point does not carry the supplied Z"),
            CheckFailure::StrayToolOrLock => write!(f, "a path sets tool_index or order_lock"),
            CheckFailure::StrayOverhang => write!(f, "a point sets an overhang field"),
            CheckFailure::PointCount { path, points } => {
                write!(f, "path {path} has {points} points, expected 2")
            }
            CheckFailure::NotContained { path, from, to } => write!(
                f,
                "path {path} segment {from:?} -> {to:?} leaves the material"
            ),
            CheckFailure::Rows { expected, actual } => {
                write!(f, "emitted {actual} scan rows, expected {expected}")
            }
            CheckFailure::Intervals {
                row,
                expected,
                actual,
            } => write!(
                f,
                "row {row} intervals {actual:?} differ from the expected {expected:?}"
            ),
            CheckFailure::MissingSplitRow => write!(f, "no scan row was split by the cut"),
            CheckFailure::ExpectedEmpty { strokes } => {
                write!(f, "expected empty output, got {strokes} strokes")
            }
            CheckFailure::ExpectedNonempty => write!(f, "expected a nonempty fill"),
            CheckFailure::Fallback { expected, actual } => write!(
                f,
                "fallback bands {actual:?} differ from the expected {expected:?}"
            ),
            CheckFailure::InterfaceContour => {
                write!(f, "the interface fallback is not the source contour")
            }
            CheckFailure::CoordinateNotOnGrid { path } => write!(
                f,
                "path {path} has a coordinate that does not recover to a whole canonical unit"
            ),
            CheckFailure::FixtureInvalid(message) => {
                write!(f, "the fixture itself is invalid: {message}")
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
struct FillStats {
    strokes: usize,
    segments: usize,
    rows: usize,
    split_rows: usize,
    left: usize,
    right: usize,
}

/// Every emitted point, recovered to canonical units. Rejects a value that does
/// not round-trip, so the comparison below is exact rather than tolerance-based.
fn to_units(paths: &[ExtrusionPath3D]) -> Result<Vec<Vec<(i64, i64)>>, CheckFailure> {
    let mut out = Vec::with_capacity(paths.len());
    for (index, path) in paths.iter().enumerate() {
        let mut points = Vec::with_capacity(path.points.len());
        for point in &path.points {
            let x = mm(point.x);
            let y = mm(point.y);
            if units_to_mm(x) != point.x || units_to_mm(y) != point.y {
                return Err(CheckFailure::CoordinateNotOnGrid { path: index });
            }
            points.push((x, y));
        }
        out.push(points);
    }
    Ok(out)
}

/// Exact expected rows and intervals for an angle-zero rectangle fixture.
fn expected_rows(case: &TsCase) -> Vec<(i64, Vec<(i64, i64)>)> {
    let mut rows = Vec::new();
    let mut y = case.y0;
    while y < case.y1 {
        let intervals = match case.cut {
            None => vec![(case.x0, case.x1)],
            Some((cx0, cy0, cx1, _)) => {
                let active = if case.shape == ShapeKind::Notch {
                    y >= cy0
                } else {
                    let (_, _, _, cy1) = case.cut.expect("cut");
                    y >= cy0 && y < cy1
                };
                if active {
                    vec![(case.x0, cx0), (cx1, case.x1)]
                } else {
                    vec![(case.x0, case.x1)]
                }
            }
        };
        rows.push((y, intervals));
        y += case.spacing_units;
    }
    rows
}

/// Mirror of the kernel's `rotate_point`: `f64` arithmetic, round to nearest.
fn rotate_units(x: i64, y: i64, cos_a: f64, sin_a: f64) -> (i64, i64) {
    let xf = x as f64;
    let yf = y as f64;
    (
        (xf * cos_a - yf * sin_a).round() as i64,
        (xf * sin_a + yf * cos_a).round() as i64,
    )
}

/// Unrotated bbox centre, the kernel's rotation pivot.
fn pivot(case: &TsCase) -> (i64, i64) {
    (
        case.x0 + (case.x1 - case.x0) / 2,
        case.y0 + (case.y1 - case.y0) / 2,
    )
}

/// Map an absolute point into the kernel's bbox-centre-relative scan frame.
///
/// The kernel rotates forward by `-angle` and back by `+angle`; the inverse of
/// the forward rotation is another `-angle` turn.
fn scan_frame(case: &TsCase, x: i64, y: i64) -> (i64, i64) {
    let (px, py) = pivot(case);
    let (cos_a, sin_a) = case.run_angle();
    rotate_units(x - px, y - py, cos_a, -sin_a)
}

/// Transform an axis-aligned rect into the scan frame (still axis-aligned for
/// quarter turns) and return its `(min_x, min_y, max_x, max_y)`.
fn rotated_rect(case: &TsCase, rect: (i64, i64, i64, i64)) -> (i64, i64, i64, i64) {
    let (x0, y0, x1, y1) = rect;
    let mut min = (i64::MAX, i64::MAX);
    let mut max = (i64::MIN, i64::MIN);
    for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
        let p = scan_frame(case, x, y);
        min = (min.0.min(p.0), min.1.min(p.1));
        max = (max.0.max(p.0), max.1.max(p.1));
    }
    (min.0, min.1, max.0, max.1)
}

/// Expected rows and material intervals for a rotated fixture, in scan-frame
/// coordinates. Quarter turns keep the fixture rectangles axis-aligned there, so
/// this is the same rectangle arithmetic the angle-zero model uses.
fn expected_rotated_rows(case: &TsCase) -> Vec<(i64, Vec<(i64, i64)>)> {
    let outer = rotated_rect(case, (case.x0, case.y0, case.x1, case.y1));
    let cut = case.cut.map(|c| rotated_rect(case, c));
    let mut rows = Vec::new();
    let mut y = outer.1;
    while y < outer.3 {
        let intervals = match cut {
            None => vec![(outer.0, outer.2)],
            Some(c) if y >= c.1 && y < c.3 => vec![(outer.0, c.0), (c.2, outer.2)],
            Some(_) => vec![(outer.0, outer.2)],
        };
        rows.push((y, intervals));
        y += case.spacing_units;
    }
    rows
}

fn check_fill(
    case: &TsCase,
    limits: &TsLimits,
    paths: &[ExtrusionPath3D],
) -> Result<FillStats, CheckFailure> {
    case.validate(limits)
        .map_err(CheckFailure::FixtureInvalid)?;
    let polygon = case.polygon();
    let mut stats = FillStats::default();

    for (index, path) in paths.iter().enumerate() {
        if path.role != ExtrusionRole::SupportMaterial {
            return Err(CheckFailure::WrongRole);
        }
        if path.speed_factor != case.speed_factor {
            return Err(CheckFailure::WrongSpeed);
        }
        if path.tool_index.is_some() || path.order_lock.is_some() {
            return Err(CheckFailure::StrayToolOrLock);
        }
        for point in &path.points {
            if point.width != case.path_width_mm {
                return Err(CheckFailure::WrongWidth);
            }
            if point.flow_factor != 1.0 {
                return Err(CheckFailure::WrongFlow);
            }
            if point.z != case.z {
                return Err(CheckFailure::WrongZ);
            }
            if point.overhang_quartile.is_some() || point.overhang_distance_mm.is_some() {
                return Err(CheckFailure::StrayOverhang);
            }
            if point.dist_to_top_mm != 0.0 {
                return Err(CheckFailure::StrayOverhang);
            }
        }
        // Every consecutive pair, including the closing edge of a closed ring.
        for window in path.points.windows(2) {
            let segment = [
                slicer_ir::Point2 {
                    x: mm(window[0].x),
                    y: mm(window[0].y),
                },
                slicer_ir::Point2 {
                    x: mm(window[1].x),
                    y: mm(window[1].y),
                },
            ];
            stats.segments += 1;
            if !segment_in_region(
                segment,
                std::slice::from_ref(&polygon),
                BoundaryRule::Inclusive,
            )
            .map_err(|_| CheckFailure::NotContained {
                path: index,
                from: (segment[0].x, segment[0].y),
                to: (segment[1].x, segment[1].y),
            })? {
                return Err(CheckFailure::NotContained {
                    path: index,
                    from: (segment[0].x, segment[0].y),
                    to: (segment[1].x, segment[1].y),
                });
            }
        }
    }

    if case.shape == ShapeKind::Degenerate {
        if !paths.is_empty() {
            return Err(CheckFailure::ExpectedEmpty {
                strokes: paths.len(),
            });
        }
        return Ok(stats);
    }

    if paths.is_empty() {
        return Err(CheckFailure::ExpectedNonempty);
    }

    match case.shape {
        ShapeKind::Narrow => {
            // The fallback band is derived from the bbox-centre-relative
            // rotation: the triangle (5,4) (6,4.5) (4,4.5) mm has its centre at
            // (5, 4.25) mm and the centroid row crosses its two slanted edges at
            // x = 4.5 and x = 5.5 mm.
            let actual = to_units(paths)?;
            let expected = vec![vec![(mm(4.5), mm(4.25)), (mm(5.5), mm(4.25))]];
            if actual != expected {
                return Err(CheckFailure::Fallback {
                    expected: vec![(mm(4.25), mm(4.5), mm(5.5))],
                    actual: actual.iter().flatten().map(|&(x, y)| (y, x, x)).collect(),
                });
            }
            stats.strokes = paths.len();
            stats.rows = 1;
            return Ok(stats);
        }
        ShapeKind::Interface => {
            let contour = case.polygon().contour.points;
            if paths.len() != 1 {
                return Err(CheckFailure::InterfaceContour);
            }
            let actual = to_units(paths)?;
            let mut expected: Vec<(i64, i64)> = contour.iter().map(|p| (p.x, p.y)).collect();
            expected.push(expected[0]);
            if actual[0] != expected {
                return Err(CheckFailure::InterfaceContour);
            }
            stats.strokes = 1;
            stats.rows = 1;
            return Ok(stats);
        }
        ShapeKind::Rotated => {
            // Exact quarter turns, so `rotate_point`'s two `f64` roundings are
            // exactly representable and the whole expected model can be built
            // in the kernel's own scan frame with integer arithmetic. The scan
            // frame is bbox-centre-relative and rotated by -angle; mapping an
            // emitted point back through the inverse quarter turn recovers the
            // crossing the kernel paired, exactly.
            let rows = expected_rotated_rows(case);
            let actual = to_units(paths)?;
            let mut by_row: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
            for (index, points) in actual.iter().enumerate() {
                if points.len() != 2 {
                    return Err(CheckFailure::PointCount {
                        path: index,
                        points: points.len(),
                    });
                }
                let start = scan_frame(case, points[0].0, points[0].1);
                let end = scan_frame(case, points[1].0, points[1].1);
                if start.1 != end.1 {
                    return Err(CheckFailure::Intervals {
                        row: start.1,
                        expected: Vec::new(),
                        actual: vec![start, end],
                    });
                }
                by_row
                    .entry(start.1)
                    .or_default()
                    .push((start.0.min(end.0), start.0.max(end.0)));
            }
            stats.strokes = paths.len();
            stats.rows = by_row.len();
            if by_row.len() != rows.len() {
                return Err(CheckFailure::Rows {
                    expected: rows.len(),
                    actual: by_row.len(),
                });
            }
            for (y, expected) in &rows {
                let mut actual_intervals = by_row.remove(y).unwrap_or_default();
                let mut expected_intervals = expected.clone();
                actual_intervals.sort_unstable();
                expected_intervals.sort_unstable();
                if actual_intervals != expected_intervals {
                    return Err(CheckFailure::Intervals {
                        row: *y,
                        expected: expected_intervals,
                        actual: actual_intervals,
                    });
                }
                if expected_intervals.len() == 2 {
                    stats.split_rows += 1;
                }
            }
            if case.cut.is_some() && stats.split_rows == 0 {
                return Err(CheckFailure::MissingSplitRow);
            }
            return Ok(stats);
        }
        _ => {}
    }

    // Angle-zero rectangle modes: exact row and interval set.
    let rows = expected_rows(case);
    let actual = to_units(paths)?;
    let mut by_row: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
    for (index, points) in actual.iter().enumerate() {
        if points.len() != 2 {
            return Err(CheckFailure::PointCount {
                path: index,
                points: points.len(),
            });
        }
        if points[0].1 != points[1].1 {
            return Err(CheckFailure::Intervals {
                row: points[0].1,
                expected: Vec::new(),
                actual: points.clone(),
            });
        }
        by_row
            .entry(points[0].1)
            .or_default()
            .push((points[0].0, points[1].0));
    }
    stats.strokes = paths.len();
    stats.rows = by_row.len();
    if by_row.len() != rows.len() {
        return Err(CheckFailure::Rows {
            expected: rows.len(),
            actual: by_row.len(),
        });
    }
    for (y, expected) in &rows {
        let mut actual_intervals = by_row.remove(y).unwrap_or_default();
        let mut expected_intervals = expected.clone();
        actual_intervals.sort_unstable();
        expected_intervals.sort_unstable();
        if actual_intervals != expected_intervals {
            return Err(CheckFailure::Intervals {
                row: *y,
                expected: expected_intervals,
                actual: actual_intervals,
            });
        }
        if let Some((cx0, _, cx1, _)) = case.cut {
            if expected_intervals.len() == 2 {
                stats.split_rows += 1;
                if actual_intervals.iter().any(|&(_, b)| b <= cx0) {
                    stats.left += 1;
                }
                if actual_intervals.iter().any(|&(a, _)| a >= cx1) {
                    stats.right += 1;
                }
            }
        }
    }
    if case.cut.is_some() && stats.split_rows == 0 {
        return Err(CheckFailure::MissingSplitRow);
    }
    // Every emitted interval must be non-degenerate.
    for points in &actual {
        if points[1].0 - points[0].0 < MIN_INTERVAL_UNITS {
            return Err(CheckFailure::Intervals {
                row: points[0].1,
                expected: Vec::new(),
                actual: points.clone(),
            });
        }
    }
    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategy
// ─────────────────────────────────────────────────────────────────────────────

const MODES: [ShapeKind; 5] = [
    ShapeKind::Rect,
    ShapeKind::Notch,
    ShapeKind::Hole,
    ShapeKind::Rotated,
    ShapeKind::Degenerate,
];

fn build_case(shape: ShapeKind, raw: [u32; 10], limits: &TsLimits) -> TsCase {
    let grid = GRID_UNITS;
    let abs_steps = limits.max_abs_units / grid;
    // Keep generated fixtures inside a 10 mm working square so the profile
    // bound always holds without a rejection.
    let span_steps = abs_steps.clamp(MIN_WIDTH_STEPS + 2, 40);
    let span_units = span_steps * grid;
    let spacing_steps = (limits.max_spacing_units / grid).max(1);
    // Keep at least four rows available so a Notch (rows >= 2) or a Hole
    // (rows >= 3) always has somewhere to put its cut, at every shrink state.
    let spacing_units = (grid * (1 + (raw[0] as i64 % spacing_steps))).clamp(grid, span_units / 4);

    let max_rows_by_space = (span_units / spacing_units).max(1);
    let allowed_rows = i64::from(limits.max_rows).min(max_rows_by_space).max(1);
    // A cut needs real height above it, so its row floor is part of the
    // generator rather than a post-hoc rejection.
    let row_floor = match shape {
        ShapeKind::Hole | ShapeKind::Rotated => 3,
        ShapeKind::Notch => 2,
        _ => 1,
    }
    .min(allowed_rows);
    let rows = row_floor + (raw[1] as i64 % (allowed_rows - row_floor + 1).max(1));
    let height_steps = rows * (spacing_units / grid);
    let mut width_steps =
        MIN_WIDTH_STEPS + (raw[2] as i64 % (span_steps - MIN_WIDTH_STEPS - 1).max(1));
    if shape == ShapeKind::Rotated {
        // A quarter turn swaps the extents, so the width becomes the scan-axis
        // height. Cap it at the row ceiling so the scan-frame row count always
        // fits the profile instead of being rejected after generation.
        width_steps = width_steps
            .min(i64::from(limits.max_rows))
            .max(MIN_WIDTH_STEPS);
    }
    let width = width_steps * grid;
    let height = height_steps * grid;

    // Centre the fixture on the origin and jitter it inside the remaining
    // slack, so every coordinate stays within `span / 2` without a rejection.
    let x_slack = (span_steps - width_steps).max(0);
    let y_slack = (span_steps - height_steps).max(0);
    let x_jitter = (raw[3] as i64 % (x_slack + 1)) - x_slack / 2;
    let y_jitter = (raw[4] as i64 % (y_slack + 1)) - y_slack / 2;
    let x0 = (x_jitter - width_steps / 2) * grid;
    let y0 = (y_jitter - height_steps / 2) * grid;
    let x1 = x0 + width;
    let y1 = y0 + height;

    let carries_cut = matches!(shape, ShapeKind::Notch | ShapeKind::Hole)
        // A quarter turn maps the cut's x extent onto the scan axis, so a
        // y-aligned cut only lands on scan-row boundaries at 180 degrees. Odd
        // turns carry no cut and exercise the axis swap alone.
        || (shape == ShapeKind::Rotated && raw[9] % 3 + 1 == 2);
    let cut = if carries_cut {
        // `k` keeps the cut strictly between the bottom edge and the top edge,
        // so a hole's one-row height still fits below `y1`.
        let k_span = if shape == ShapeKind::Notch {
            (rows - 1).max(1)
        } else {
            (rows - 2).max(1)
        };
        let k = 1 + (raw[5] as i64 % k_span);
        let cy0 = y0 + k * spacing_units;
        let inner_steps = (width_steps - 2).max(1);
        let cut_off = 1 + (raw[6] as i64 % inner_steps);
        let cut_len = 1 + (raw[7] as i64 % (width_steps - cut_off - 1).max(1));
        let cx0 = x0 + cut_off * grid;
        let cx1 = cx0 + cut_len * grid;
        if shape == ShapeKind::Notch {
            Some((cx0, cy0, cx1, y1))
        } else {
            Some((cx0, cy0, cx1, cy0 + spacing_units))
        }
    } else {
        None
    };

    let degenerate = if shape == ShapeKind::Degenerate {
        Some(match raw[8] % 6 {
            0 => DegenerateKind::ZeroHeight,
            1 => DegenerateKind::ZeroWidth,
            2 => DegenerateKind::ShortRing,
            3 => DegenerateKind::EmptyContour,
            4 => DegenerateKind::ZeroSpacing,
            _ => DegenerateKind::NegativeSpacing,
        })
    } else {
        None
    };

    let mut case = TsCase {
        shape,
        x0,
        y0,
        x1,
        y1,
        cut,
        quarter_turns: if shape == ShapeKind::Rotated {
            (raw[9] % 3 + 1) as u8
        } else {
            0
        },
        spacing_units: match degenerate {
            Some(DegenerateKind::ZeroSpacing) => 0,
            Some(DegenerateKind::NegativeSpacing) => -spacing_units,
            _ => spacing_units,
        },
        z: 0.2 + (raw[9] % 8) as f32 * 0.05,
        speed_factor: 0.25 + (raw[8] % 8) as f32 * 0.25,
        path_width_mm: 0.2 + (raw[7] % 6) as f32 * 0.1,
        interface: false,
        degenerate,
    };
    if degenerate == Some(DegenerateKind::ZeroHeight) {
        case.y1 = case.y0;
    }
    if degenerate == Some(DegenerateKind::ZeroWidth) {
        case.x1 = case.x0;
    }
    if degenerate.is_some() {
        case.cut = None;
    }
    case
}

fn case_strategy(limits: &TsLimits, shape: ShapeKind) -> BoxedStrategy<TsCase> {
    let limits = *limits;
    (
        prop::sample::select(vec![shape]).no_shrink(),
        any::<[u32; 10]>(),
    )
        .prop_map(move |(shape, raw)| build_case(shape, raw, &limits))
        .boxed()
}

fn fill_case_strategy(limits: &TsLimits) -> BoxedStrategy<TsCase> {
    let limits = *limits;
    (
        prop::sample::select(MODES.to_vec()).no_shrink(),
        any::<[u32; 10]>(),
    )
        .prop_map(move |(shape, raw)| build_case(shape, raw, &limits))
        .boxed()
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic witnesses
// ─────────────────────────────────────────────────────────────────────────────

fn witness(shape: ShapeKind) -> TsCase {
    let base = TsCase {
        shape,
        x0: 0,
        y0: 0,
        x1: mm(10.0),
        y1: mm(10.0),
        cut: None,
        quarter_turns: 0,
        spacing_units: mm(1.0),
        z: 1.0,
        speed_factor: 1.0,
        path_width_mm: 0.4,
        interface: shape == ShapeKind::Interface,
        degenerate: None,
    };
    match shape {
        ShapeKind::Rect => base,
        ShapeKind::Hole => TsCase {
            cut: Some((mm(2.0), mm(4.0), mm(3.0), mm(6.0))),
            ..base
        },
        ShapeKind::Notch => TsCase {
            cut: Some((mm(4.0), mm(4.0), mm(6.0), mm(10.0))),
            ..base
        },
        ShapeKind::Rotated => TsCase {
            quarter_turns: 2,
            ..base
        },
        ShapeKind::Narrow => TsCase {
            x0: 0,
            y0: 0,
            x1: 0,
            y1: 0,
            spacing_units: mm(2.0),
            ..base
        },
        ShapeKind::Interface => TsCase {
            x0: 0,
            y0: 0,
            x1: 0,
            y1: 0,
            spacing_units: mm(2.0),
            ..base
        },
        ShapeKind::Degenerate => TsCase {
            degenerate: Some(DegenerateKind::ZeroHeight),
            y1: 0,
            ..base
        },
    }
}

/// The charter's holed `[0,10]^2` mm fixture with hole `[4,6]^2` mm at 1 mm
/// spacing, whose interior hole row y = 5 mm carries `(0,4)` and `(6,10)` mm.
fn charter_hole() -> TsCase {
    TsCase {
        cut: Some((mm(4.0), mm(4.0), mm(6.0), mm(6.0))),
        ..witness(ShapeKind::Hole)
    }
}

/// The packet's sub-pitch diamond at 2 mm pitch: the scan row's two crossings
/// coincide at the tip and no edge straddles the centroid row, so both the scan
/// and the centroid fallback emit nothing and only the interface contour
/// fallback keeps the layer printable.
fn sub_pitch_diamond() -> TsCase {
    TsCase {
        interface: true,
        spacing_units: mm(2.0),
        ..witness(ShapeKind::Interface)
    }
}

struct WitnessStats {
    charter_hole_both_sides: usize,
    asym_split: usize,
    notch_split: usize,
    rotated_split: usize,
    square_rows: usize,
    empty_modes: usize,
}

fn witness_observations(limits: &TsLimits) -> Result<WitnessStats, String> {
    let mut stats = WitnessStats {
        charter_hole_both_sides: 0,
        asym_split: 0,
        notch_split: 0,
        rotated_split: 0,
        square_rows: 0,
        empty_modes: 0,
    };
    let hole = charter_hole();
    let paths = run_kernel(&hole);
    let observed = check_fill(&hole, limits, &paths).map_err(|e| format!("charter hole: {e}"))?;
    if observed.left == 0 || observed.right == 0 || observed.split_rows == 0 {
        return Err("the charter hole must split an interior row on both sides".to_owned());
    }
    let y5 = mm(5.0);
    let mut row: Vec<(i64, i64)> = to_units(&paths)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|p| p[0].1 == y5)
        .map(|p| (p[0].0, p[1].0))
        .collect();
    row.sort_unstable();
    if row != vec![(mm(0.0), mm(4.0)), (mm(6.0), mm(10.0))] {
        return Err(format!("charter hole row y=5 intervals were {row:?}"));
    }
    stats.charter_hole_both_sides = 1;

    let asym = TsCase {
        cut: Some((mm(2.0), mm(4.0), mm(3.0), mm(6.0))),
        ..witness(ShapeKind::Hole)
    };
    let asym_paths = run_kernel(&asym);
    let asym_stats = check_fill(&asym, limits, &asym_paths).map_err(|e| format!("asym: {e}"))?;
    if asym_stats.split_rows == 0 {
        return Err("the asymmetric hole must split at least one row".to_owned());
    }
    stats.asym_split = asym_stats.split_rows;

    let notch = witness(ShapeKind::Notch);
    let notch_paths = run_kernel(&notch);
    let notch_stats =
        check_fill(&notch, limits, &notch_paths).map_err(|e| format!("notch: {e}"))?;
    if notch_stats.split_rows == 0 {
        return Err("the U-notch must split at least one row".to_owned());
    }
    stats.notch_split = notch_stats.split_rows;

    // The 180-degree rotated case is the only rotated shape that carries a cut,
    // so nothing in the sampler guarantees one. Check it here, every call.
    let rotated = TsCase {
        cut: Some((mm(3.0), mm(3.0), mm(6.0), mm(6.0))),
        ..witness(ShapeKind::Rotated)
    };
    let rotated_paths = run_kernel(&rotated);
    let rotated_stats =
        check_fill(&rotated, limits, &rotated_paths).map_err(|e| format!("rotated: {e}"))?;
    if rotated_stats.split_rows == 0 {
        return Err("the 180-degree rotated hole must split a scan row".to_owned());
    }
    stats.rotated_split = rotated_stats.split_rows;

    let square = witness(ShapeKind::Rect);
    let square_paths = run_kernel(&square);
    let square_stats =
        check_fill(&square, limits, &square_paths).map_err(|e| format!("square: {e}"))?;
    if square_stats.rows != 10 {
        return Err(format!(
            "the 10 mm square at 1 mm pitch must emit 10 rows, got {}",
            square_stats.rows
        ));
    }
    stats.square_rows = square_stats.rows;

    // The narrow translated tip and the sub-pitch diamond are the two
    // fallback modes: one must produce the bbox-centred band, the other must
    // fall through to the interface contour.
    let narrow = witness(ShapeKind::Narrow);
    let narrow_paths = run_kernel(&narrow);
    check_fill(&narrow, limits, &narrow_paths).map_err(|e| format!("narrow tip: {e}"))?;
    let diamond_paths = run_kernel(&sub_pitch_diamond());
    check_fill(&sub_pitch_diamond(), limits, &diamond_paths)
        .map_err(|e| format!("sub-pitch diamond: {e}"))?;

    for kind in [
        DegenerateKind::ZeroHeight,
        DegenerateKind::ZeroWidth,
        DegenerateKind::ShortRing,
        DegenerateKind::EmptyContour,
        DegenerateKind::ZeroSpacing,
        DegenerateKind::NegativeSpacing,
    ] {
        let case = TsCase {
            degenerate: Some(kind),
            spacing_units: match kind {
                DegenerateKind::ZeroSpacing => 0,
                DegenerateKind::NegativeSpacing => -mm(2.0),
                _ => mm(2.0),
            },
            y1: if kind == DegenerateKind::ZeroHeight {
                0
            } else {
                mm(10.0)
            },
            x1: if kind == DegenerateKind::ZeroWidth {
                0
            } else {
                mm(10.0)
            },
            ..witness(ShapeKind::Degenerate)
        };
        let paths = run_kernel(&case);
        check_fill(&case, limits, &paths).map_err(|e| format!("degenerate {kind:?}: {e}"))?;
        stats.empty_modes += 1;
    }
    Ok(stats)
}

// ─────────────────────────────────────────────────────────────────────────────
// Property
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn proptest_traditional_support_fill() {
    let run = PropertyRun::new(PROPERTY_ID, PROPERTY_TEST).expect("M-TRADITIONAL-SCAN context");
    let limits = live_limits(&run);
    let strategy = fill_case_strategy(&limits);

    let calls = Cell::new(0_u64);
    let witness_charter = Cell::new(0_u64);
    let witness_asym = Cell::new(0_u64);
    let witness_notch = Cell::new(0_u64);
    let witness_rotated = Cell::new(0_u64);
    let witness_square = Cell::new(0_u64);
    let witness_empty = Cell::new(0_u64);
    let strokes = Cell::new(0_u64);
    let segments = Cell::new(0_u64);
    let rows = Cell::new(0_u64);
    let split_rows = Cell::new(0_u64);
    let per_mode = [
        Cell::new(0_u64),
        Cell::new(0_u64),
        Cell::new(0_u64),
        Cell::new(0_u64),
        Cell::new(0_u64),
    ];

    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        if let Err(message) = case.validate(&limits) {
            return fail(format!(
                "{} case failed input validation: {message}",
                case.shape.label()
            ));
        }
        let paths = run_kernel(&case);
        let stats = match check_fill(&case, &limits, &paths) {
            Ok(stats) => stats,
            Err(error) => {
                return fail(format!("{} case: {error}", case.shape.label()));
            }
        };
        strokes.set(strokes.get() + stats.strokes as u64);
        segments.set(segments.get() + stats.segments as u64);
        rows.set(rows.get() + stats.rows as u64);
        split_rows.set(split_rows.get() + stats.split_rows as u64);
        if let Some(slot) = MODES.iter().position(|m| *m == case.shape) {
            per_mode[slot].set(per_mode[slot].get() + 1);
        }
        // The deterministic positive witnesses are re-checked inside every
        // accepted call, so a rare topology is never the only evidence.
        match witness_observations(&limits) {
            Ok(witness) => {
                witness_charter.set(witness_charter.get() + witness.charter_hole_both_sides as u64);
                witness_asym.set(witness_asym.get() + witness.asym_split as u64);
                witness_notch.set(witness_notch.get() + witness.notch_split as u64);
                witness_rotated.set(witness_rotated.get() + witness.rotated_split as u64);
                witness_square.set(witness_square.get() + witness.square_rows as u64);
                witness_empty.set(witness_empty.get() + witness.empty_modes as u64);
                TestCaseResult::Ok(())
            }
            Err(message) => fail(format!("deterministic witness: {message}")),
        }
    })
    .expect("M-TRADITIONAL-SCAN property");

    assert!(calls.get() > 0, "the property callback never ran");
    assert!(witness_charter.get() > 0, "charter hole witness unobserved");
    assert!(witness_asym.get() > 0, "asymmetric hole split unobserved");
    assert!(witness_notch.get() > 0, "U-notch split unobserved");
    assert!(
        witness_rotated.get() > 0,
        "180-degree rotated hole split unobserved"
    );
    assert!(witness_square.get() > 0, "square rows unobserved");
    assert!(witness_empty.get() > 0, "degenerate empty modes unobserved");
    assert!(strokes.get() > 0, "no stroke was observed");
    assert!(segments.get() > 0, "no whole segment was observed");
    assert!(rows.get() > 0, "no scan row was observed");
    assert!(
        split_rows.get() > 0,
        "no cut ever split a scan row across the run"
    );
    for (slot, mode) in MODES.iter().enumerate() {
        assert!(
            per_mode[slot].get() > 0,
            "mode {} was never generated",
            mode.label()
        );
    }
    println!(
        "M-TRADITIONAL-SCAN accepted_calls={} rect={} notch={} hole={} rotated={} degenerate={} \
         strokes={} segments={} rows={} split_rows={} witness_charter={} witness_asym_split={}          witness_notch_split={} witness_rotated_split={} witness_square_rows={} witness_empty={}",
        calls.get(),
        per_mode[0].get(),
        per_mode[1].get(),
        per_mode[2].get(),
        per_mode[3].get(),
        per_mode[4].get(),
        strokes.get(),
        segments.get(),
        rows.get(),
        split_rows.get(),
        witness_charter.get(),
        witness_asym.get(),
        witness_notch.get(),
        witness_rotated.get(),
        witness_square.get(),
        witness_empty.get()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic mutation controls
// ─────────────────────────────────────────────────────────────────────────────

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
    case: &TsCase,
    limits: &TsLimits,
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

#[test]
fn traditional_renderer_controls() {
    let limits = checked_limits(&probe_bound_map()).expect("probe bounds");
    let witnesses = witness_observations(&limits).expect("all witnesses are observable");
    assert_eq!(witnesses.charter_hole_both_sides, 1);
    assert!(witnesses.asym_split > 0);
    assert!(witnesses.notch_split > 0);
    assert_eq!(witnesses.square_rows, 10);
    assert_eq!(witnesses.empty_modes, 6);

    let is_not_contained = |e: &CheckFailure| matches!(e, CheckFailure::NotContained { .. });
    let is_row_set = |e: &CheckFailure| {
        matches!(
            e,
            CheckFailure::Rows { .. }
                | CheckFailure::Intervals { .. }
                | CheckFailure::ExpectedNonempty
                | CheckFailure::MissingSplitRow
        )
    };
    let is_empty = |e: &CheckFailure| matches!(e, CheckFailure::ExpectedEmpty { .. });
    let is_fallback = |e: &CheckFailure| matches!(e, CheckFailure::Fallback { .. });

    // ── The asymmetric hole: the exact hole-crossing segment ────────────────
    let asym = TsCase {
        cut: Some((mm(2.0), mm(4.0), mm(3.0), mm(6.0))),
        ..witness(ShapeKind::Hole)
    };
    let asym_paths = run_kernel(&asym);
    check_fill(&asym, &limits, &asym_paths).expect("asym witness");
    // Both endpoints and the midpoint are material, so only a whole-segment
    // check catches this chord.
    let polygon = asym.polygon();
    for point in [(1.0, 5.0), (9.0, 5.0), (5.0, 5.0)] {
        assert_eq!(
            point_in_region(
                slicer_ir::Point2::from_mm(point.0, point.1),
                std::slice::from_ref(&polygon)
            )
            .unwrap(),
            PointLocation::Inside,
            "the control points must all be material"
        );
    }
    let crossing = stroke_like(&asym_paths[0], 1.0, 9.0, 5.0);
    expect_failure(
        &asym,
        &limits,
        &[crossing.clone()],
        "unsplit hole-crossing segment",
        is_not_contained,
    );
    let mut appended = asym_paths.clone();
    appended.push(crossing);
    expect_failure(
        &asym,
        &limits,
        &appended,
        "hole-crossing segment appended",
        is_not_contained,
    );

    // ── The U-notch chord ──────────────────────────────────────────────────
    let notch = witness(ShapeKind::Notch);
    let notch_paths = run_kernel(&notch);
    check_fill(&notch, &limits, &notch_paths).expect("notch witness");
    expect_failure(
        &notch,
        &limits,
        &[stroke_like(&notch_paths[0], 2.0, 8.0, 5.0)],
        "notch chord",
        is_not_contained,
    );

    // ── Rotated split row: dropping one side must be caught ───────────────
    // The rotated oracle compares complete intervals in the scan frame, so a
    // split row that loses one arm is a failure rather than a shorter row list.
    let rotated = TsCase {
        cut: Some((mm(3.0), mm(3.0), mm(6.0), mm(6.0))),
        ..witness(ShapeKind::Rotated)
    };
    let rotated_paths = run_kernel(&rotated);
    let rotated_stats = check_fill(&rotated, &limits, &rotated_paths).expect("rotated witness");
    assert!(
        rotated_stats.split_rows > 0,
        "the rotated witness must split at least one scan row"
    );
    // Group by the scan-frame row and drop one arm of the first split pair.
    let mut by_row: BTreeMap<i64, Vec<usize>> = BTreeMap::new();
    for (index, path) in rotated_paths.iter().enumerate() {
        let row = scan_frame(&rotated, mm(path.points[0].x), mm(path.points[0].y)).1;
        by_row.entry(row).or_default().push(index);
    }
    let split_row = by_row
        .values()
        .find(|indices| indices.len() == 2)
        .cloned()
        .expect("a split row in the rotated witness");
    let dropped = split_row[1];
    let mut missing_arm: Vec<ExtrusionPath3D> = rotated_paths
        .iter()
        .enumerate()
        .filter(|(index, _)| *index != dropped)
        .map(|(_, path)| path.clone())
        .collect();
    expect_failure(
        &rotated,
        &limits,
        &missing_arm,
        "rotated split row with one arm dropped",
        is_row_set,
    );
    // Sanity: the untouched set is still accepted, so the control isolates the
    // dropped arm rather than the grouping.
    missing_arm.push(rotated_paths[dropped].clone());
    check_fill(&rotated, &limits, &missing_arm).expect("restored rotated witness");

    // ── Dropped row and false all-empty on the 10 mm square ────────────────
    let square = witness(ShapeKind::Rect);
    let square_paths = run_kernel(&square);
    check_fill(&square, &limits, &square_paths).expect("square witness");
    expect_failure(
        &square,
        &limits,
        &without_row(&square_paths, 1.0),
        "dropped interior y=1 row",
        is_row_set,
    );
    // The outer top edge y = 10 mm is excluded, so adding it is also a failure.
    let mut extra = square_paths.clone();
    extra.push(stroke_like(&square_paths[0], 0.0, 10.0, 10.0));
    expect_failure(
        &square,
        &limits,
        &extra,
        "row on the excluded top edge",
        is_row_set,
    );
    expect_failure(&square, &limits, &[], "false all-empty", is_row_set);

    // ── Fallback modes ─────────────────────────────────────────────────────
    let narrow = witness(ShapeKind::Narrow);
    let narrow_paths = run_kernel(&narrow);
    check_fill(&narrow, &limits, &narrow_paths).expect("narrow witness");
    // The pre-fix bug emitted the band around the relative origin.
    expect_failure(
        &narrow,
        &limits,
        &[stroke_like(&narrow_paths[0], -0.5, 0.5, 0.0)],
        "untranslated centroid fallback",
        // The untranslated band sits outside the polygon, so containment
        // catches it before the fallback-specific comparison does.
        |e: &CheckFailure| is_fallback(e) || is_not_contained(e),
    );
    let diamond = sub_pitch_diamond();
    let diamond_paths = run_kernel(&diamond);
    check_fill(&diamond, &limits, &diamond_paths).expect("diamond witness");
    expect_failure(
        &diamond,
        &limits,
        &[],
        "sub-pitch diamond with no interface contour",
        is_intervals_or_interface,
    );

    // ── Degenerate inputs must produce nothing ─────────────────────────────
    for kind in [
        DegenerateKind::ZeroHeight,
        DegenerateKind::ZeroWidth,
        DegenerateKind::ShortRing,
        DegenerateKind::EmptyContour,
        DegenerateKind::ZeroSpacing,
        DegenerateKind::NegativeSpacing,
    ] {
        let case = TsCase {
            degenerate: Some(kind),
            spacing_units: match kind {
                DegenerateKind::ZeroSpacing => 0,
                DegenerateKind::NegativeSpacing => -mm(2.0),
                _ => mm(2.0),
            },
            y1: if kind == DegenerateKind::ZeroHeight {
                0
            } else {
                mm(10.0)
            },
            x1: if kind == DegenerateKind::ZeroWidth {
                0
            } else {
                mm(10.0)
            },
            ..witness(ShapeKind::Degenerate)
        };
        expect_failure(
            &case,
            &limits,
            &[stroke_like(&square_paths[0], 0.0, 10.0, 0.0)],
            &format!("degenerate {kind:?} with output"),
            // Either the empty-output contract or containment catches injected
            // material on a degenerate fixture.
            |e: &CheckFailure| is_empty(e) || is_not_contained(e),
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
        ("max_abs_units", 19_999),
        ("max_abs_units", 80_100),
        ("max_spacing_units", 2_400),
        ("max_spacing_units", 10_100),
        ("max_rows", 3),
        ("max_rows", 65),
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
    println!("traditional_renderer_controls: all mutations rejected by the shared checker");
}

fn is_intervals_or_interface(e: &CheckFailure) -> bool {
    matches!(
        e,
        CheckFailure::Intervals { .. }
            | CheckFailure::InterfaceContour
            | CheckFailure::ExpectedNonempty
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink probe
// ─────────────────────────────────────────────────────────────────────────────

fn check_shrink_state(case: &TsCase, shape: ShapeKind, limits: &TsLimits) {
    assert_eq!(
        case.shape, shape,
        "the selected shape must survive shrinking"
    );
    assert_eq!(
        case.degenerate.is_some(),
        shape == ShapeKind::Degenerate,
        "only the degenerate mode carries a degenerate class"
    );
    assert!(
        case.z > 0.0 && case.z.is_finite(),
        "z must stay positive and finite"
    );
    assert!(
        case.speed_factor > 0.0 && case.speed_factor.is_finite(),
        "speed factor must stay positive and finite"
    );
    assert!(
        case.path_width_mm > 0.0 && case.path_width_mm <= 1.0,
        "path width outside its generated range"
    );
    if case.shape.is_rectangle() {
        assert!(case.x0 < case.x1 && case.y0 < case.y1);
        assert_eq!(case.spacing_units % GRID_UNITS, 0);
        assert!((GRID_UNITS..=limits.max_spacing_units).contains(&case.spacing_units));
        let mut rows = 0_i64;
        let mut y = case.y0;
        while y < case.y1 {
            rows += 1;
            y += case.spacing_units;
        }
        assert!(
            (MIN_ROWS..=i64::from(limits.max_rows)).contains(&rows),
            "row count {rows} outside the profile bound"
        );
    }
    case.validate(limits)
        .unwrap_or_else(|e| panic!("shrunk {} fixture: {e}", shape.label()));
    let paths = run_kernel(case);
    check_fill(case, limits, &paths)
        .unwrap_or_else(|e| panic!("shrunk {} output: {e}", shape.label()));
    if shape == ShapeKind::Degenerate {
        assert!(paths.is_empty(), "a degenerate mode must stay empty");
    } else if shape.is_rectangle() {
        assert!(!paths.is_empty(), "a rectangle mode must stay nonempty");
    }
}

fn case_bits(case: &TsCase) -> Vec<i64> {
    vec![
        case.x0,
        case.y0,
        case.x1,
        case.y1,
        case.spacing_units,
        mm(case.z),
        mm(case.speed_factor),
        mm(case.path_width_mm),
        i64::from(case.quarter_turns),
        case.cut.map(|c| c.0).unwrap_or(i64::MIN),
    ]
}

#[test]
fn traditional_renderer_shrink_bounds() {
    let limits = checked_limits(&probe_bound_map()).expect("shrink bounds");
    for shape in MODES {
        let strategy = case_strategy(&limits, shape);
        let mut runner = TestRunner::deterministic();
        let mut tree = strategy.new_tree(&mut runner).expect("scan shrink tree");
        let initial = tree.current();
        check_shrink_state(&initial, shape, &limits);
        // Begin nonminimally: the probe bounds admit more than one row and a
        // nonzero speed factor, so a reduction is available.
        let initial_bits = case_bits(&initial);

        let mut simplified = 0_usize;
        let mut changed = false;
        while tree.simplify() {
            simplified += 1;
            let current = tree.current();
            check_shrink_state(&current, shape, &limits);
            changed |= case_bits(&current) != initial_bits;
            assert!(
                tree.complicate(),
                "complicate must restore the last simplification"
            );
        }
        let mut stable_false = 0;
        for _ in 0..4 {
            if !tree.simplify() {
                stable_false += 1;
            }
        }
        assert!(
            simplified > 0,
            "{} never simplified from its nonminimal start",
            shape.label()
        );
        assert!(
            changed,
            "{} shrinking never changed an exposed value",
            shape.label()
        );
        assert!(
            stable_false > 0,
            "{} never reached a stable false simplify state",
            shape.label()
        );
    }
    // The selector itself must refuse to shrink: a shrink that changed the mode
    // would let a required topology class disappear.
    let selector = prop::sample::select(MODES.to_vec()).no_shrink();
    let mut runner = TestRunner::deterministic();
    let mut tree = selector.new_tree(&mut runner).expect("mode selector tree");
    let chosen = tree.current();
    assert!(!tree.simplify(), "the mode selector must not simplify");
    assert!(!tree.complicate(), "the mode selector must not complicate");
    assert_eq!(tree.current(), chosen, "the selected mode must be stable");
    println!(
        "traditional_renderer_shrink_bounds: {} modes probed with real simplify/complicate",
        MODES.len()
    );
}
