//! Opt-in property tests for the wipe-tower purge/prime path generator.
//!
//! Charter: `docs/spec_packets/proptest-openrscad_16_pbt-io-gcode/packet.spec.md`
//! § H-WIPE-TOWER and `## Deviations` item 2; registry record `H-WIPE-TOWER`
//! in `test-properties.toml`.
//!
//! * `H-WIPE-TOWER` — the real private
//!   `WipeTower::generate_purge_paths` against an independent
//!   travel/scan/prime model over the tower rectangle, plus a convex bed
//!   polygon used as an independent scene witness.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! This module is a crate-internal unit-test child, so the private
//! `generate_purge_paths` is reachable through `super::WipeTower` without
//! widening production visibility. The tower is built through the real
//! `WipeTower::from_config` with a complete `ConfigView`; `layer_height` is a
//! direct argument of the seam (not a config key).
//!
//! # Independent expected model
//!
//! The checker never calls the generator to obtain an expected value.
//!
//! ```text
//! cross_section = line_width * layer_height * tower_width        (f32 storage)
//! if cross_section <= 0                       -> no pairs at all
//! purge_depth   = purge_volume / cross_section
//! y_min, y_max   = tower_y, tower_y + purge_depth
//! x_min, x_max   = tower_x, tower_x + tower_width
//!
//! travel pair : (tower_x, tower_y) twice, flow 0.0 both, width line_width
//! scan rows   : y = tower_y + line_width/2 + k*line_width while y < y_max,
//!               alternating x_min -> x_max / x_max -> x_min, flow 1.0 both
//! prime pair  : (tower_x, tower_y) flow 0.0 ->
//!               (tower_x + min(purge_volume / (line_width * layer_height),
//!                             tower_width), tower_y) flow 1.0
//! ```
//!
//! The f32 evaluation order of `cross_section` and of the prime denominator is
//! mirrored explicitly: the model computes each product in `f64` and narrows
//! once, which is the same result as the f32 operation because `f64` carries
//! `53 >= 2*24+2` bits (double rounding is innocuous for one round-trip of
//! `+`, `-`, `*`, `/`). That is float-storage semantics, not a copy of the
//! generator's algorithm: the row positions, the row count, the prime cap and
//! every key/role/flow/width expectation come from the formula above.
//!
//! ## Scan-row count: exact class versus classified class
//!
//! The row count has two independent derivations that must agree:
//!
//! 1. a bounded loop that starts at `tower_y + line_width/2` and steps
//!    `y += line_width` (the model's own arithmetic on the f32 grid), and
//! 2. the closed form `ceil((y_max - (tower_y + line_width/2)) / line_width)`,
//!    computed in `f64` from the f32 endpoints.
//!
//! Before the loop the model computes
//! `n_upper = ceil(purge_depth / line_width) + 1` in checked `f64` arithmetic
//! and requires `n_upper <= max_scan_lines`, `line_width > 0`, and finite
//! `purge_depth`/`y_min`/`y_max`; inside the loop every `y += line_width` must
//! be finite and strictly increasing and the loop may not exceed `n_upper`.
//! Any failure is a domain/exhaustion diagnostic — the model never enters an
//! unbounded loop.
//!
//! The generator's volume is constructed from the profile bounds so that the
//! scan budget can never be exceeded: `purge_depth = units * line_width/2`
//! where `units` is capped by both `max_purge_volume_mm3` and
//! `2*(max_scan_lines-1) - 2`.
//!
//! **Exact class.** When the chosen f32 inputs make every addition exact —
//! `line_width` a power of two in `[1/8, 2]`, `tower_x`/`tower_y` on the
//! 1/16 mm grid, and `purge_depth` an exact multiple of `line_width/2` (so
//! `tower_y + purge_depth` is exact) — the property requires *exactly*
//! `ceil((y_max - (tower_y + line_width/2)) / line_width)` rows (zero when that
//! difference is not positive). On this domain every `y += line_width` step is
//! exact, so the closed form is the true count, including the near-boundary
//! witness where `y_last == y_max` and the strict `<` drops the last row.
//!
//! **Classified class.** When `purge_depth` is not an exact multiple of
//! `line_width/2` (or the cross-section product itself rounded), the property
//! requires the observed count to be within one of the rational `ceil` and
//! counts the case in the classified population. A ±1 observation is never
//! presented as an exact count.
//!
//! # Containment envelope
//!
//! Every generated point must lie inside the closed tower rectangle within a
//! derived envelope. For `a`, `b` with `|a|, |b| <= M`, one f32 rounding of
//! `a + b` satisfies `|fl(a+b) - (a+b)| <= 2^-24 * |fl(a+b)| <= 2^-23 * M`.
//! With `M = max_abs_mm + max(max_scan_lines * max_line_width_mm,
//! tower-width cap)` the envelope is `M * 2^-23` (≈ 2.3e-5 mm on the measured
//! profile), which covers the single rounding of `tower_x + tower_width`,
//! `tower_y + purge_depth` and `tower_x + prime_length`. Only centre points are
//! asserted, so no half-stroke term is added.
//!
//! # Bed containment is a scene witness, not a production check (deviation 2)
//!
//! `generate_purge_paths` never reads `bed_shape`; the production bed-polygon
//! check (`point_in_polygon`) lives in `WipeTower::run_finalization`. The
//! property therefore builds a **convex** bed polygon (a rectangle or a right
//! triangle, both on the 1/16 mm grid, padded by at least one grid cell around
//! the grid-rounded tower footprint) and checks every generated point with its
//! own half-plane containment. The implication "every generated point is inside
//! the bed" holds because the tower rectangle itself is inside the bed, which
//! the checker asserts separately for the four envelope-inflated rectangle
//! corners.
//!
//! Convexity is what makes four-corner containment sufficient. A concave bed is
//! explicitly **not** supported and is never claimed: the deterministic concave
//! control below shows that a concave bed can contain all four tower corners
//! while missing a tower-interior point, i.e. corner-only validation is
//! insufficient. That control is a statement about the *test oracle*, not a
//! claim that `generate_purge_paths` or `run_finalization` rejects the shape.
//!
//! # Zero modes
//!
//! `line_width = 0`, `tower_width = 0` and `layer_height = 0` make
//! `cross_section <= 0`, which must return an empty vector. `purge_volume = 0`
//! with a positive cross-section gives `purge_depth = 0`: no scan rows, one
//! zero-length travel pair and one zero-length prime pair. Those modes assert
//! exact counts and closure only — no positive-length and no extra enclosure
//! claim.
//!
//! # Literal review (manual; the scanners do not cover this file)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`.
//!
//! Watched-type literal sites here: **none**. A watched type is a `pub` struct
//! with at least five named fields defined under `crates/*/src`
//! (`docs/21_data_defaults_and_fixtures.md` §3). This file constructs:
//!
//! * `ToolChange` — three named fields (`crates/slicer-ir/src/slice_ir.rs`), below
//!   the five-field threshold; built with a `..ToolChange::default()` FRU rest
//!   anyway, matching the requested fixture shape.
//! * `ConfigView` — via `ConfigView::from_map`, not a literal.
//! * `WipeCase`, `BedPolygon`, `TowerRect`, `RowModel`, `WipeObservation`,
//!   `Counters`, `WipeLimits`, `OracleModel` and the mode enums — module-private
//!   test types, not `pub` in `crates/*/src`, so not watched. Their literals are
//!   exhaustive by construction because the fields are the fixture.
//! * `Point3WithWidth`, `ExtrusionPath3D`, `RegionKey` are only *read* from the
//!   generator's output, never constructed.
//!
//! No FRU waiver is required and none is claimed: this is a manual review, not
//! a scanner pass.
//!
//! # Test-quality review (manual)
//!
//! `cargo xtask check-test-quality` has the same module-source scope gap. The
//! controls below are mutations of the **same** `check_purge_paths` /
//! `check_bed_containment` the property uses: each first accepts the honest
//! model on a deterministic witness, then rejects exactly one mutated
//! expectation. The property requires non-vacuous positive populations (counters
//! asserted after the run), so a generator regression that stopped producing a
//! scan row, an exact-class case, a classified case, a zero mode or either bed
//! shape fails instead of passing silently. There is no hand-maintained roster
//! and no source grep. The bed-independence observation is a determinism pin
//! (two constructions of the same generator with different `bed_shape` values
//! must produce identical output) and is marked as such. The shrink probe walks
//! a nonminimal witness per mode, asserts the retained mode at every step and
//! never claims global minimality or a shrink-step count; the `no_shrink` probe
//! is a determinism pin on the mode selectors.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};

use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, RegionKey, ToolChange,
};
use slicer_sdk::test_support::property_testing::PropertyRun;

use super::WipeTower;

const PROPERTY_ID: &str = "H-WIPE-TOWER";
const PROPERTY_TEST: &str = "property_tests::proptest_generate_purge_paths";
/// Exact `PropertyRun::bound` keys of the registry record.
const BOUND_KEYS: [&str; 4] = [
    "max_abs_mm",
    "max_scan_lines",
    "max_line_width_mm",
    "max_purge_volume_mm3",
];

/// Exact 1/16 mm grid step used by every generated coordinate and size.
const GRID_MM: f32 = 1.0 / 16.0;
/// Documented maximum generated tower width. 16 mm keeps `tower_x +
/// tower_width` inside `max_abs_mm + 16` and keeps the sum exact on the grid.
const TOWER_WIDTH_CAP_MM: f32 = 16.0;
/// Positive generated layer heights are 1..=8 sixteenths of a millimetre.
const LAYER_HEIGHT_UNITS_MAX: u32 = 8;
/// The module's documented default layer height (`DEFAULT_LAYER_HEIGHT` in
/// `src/lib.rs`). 0.2 mm is **not** a 1/16 multiple (0.2 × 16 = 3.2), so it is a
/// control-only witness that exercises the classified row-count branch; the
/// generated domain stays on the exact 1/16 mm grid.
const PRODUCTION_DEFAULT_LAYER_HEIGHT_MM: f32 = 0.2;
/// Bed padding choices, in 1/16 mm units: 1/16, 1/4 and 4 mm. Every choice is
/// far above the derived containment envelope (≈ 2.3e-5 mm).
const BED_PAD_UNIT_CHOICES: [i64; 3] = [1, 4, 64];

/// Deterministic control values: the registry routine and extended profiles
/// share `max_abs_mm = 64`, `max_scan_lines = 64`, `max_line_width_mm = 2`,
/// `max_purge_volume_mm3 = 64`.
const CONTROL_MAX_ABS_MM: f32 = 64.0;
const CONTROL_MAX_SCAN_LINES: usize = 64;
const CONTROL_MAX_LINE_WIDTH_MM: f32 = 2.0;
const CONTROL_MAX_PURGE_VOLUME_MM3: f32 = 64.0;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// Bounds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
struct WipeLimits {
    max_abs_mm: f32,
    max_scan_lines: usize,
    max_line_width_mm: f32,
    max_purge_volume_mm3: f32,
}

fn live_limits(run: &PropertyRun) -> WipeLimits {
    let mut bounds = BTreeMap::new();
    for key in BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("H-WIPE-TOWER bound `{key}`: {error}"));
        bounds.insert(key, value);
    }
    WipeLimits {
        max_abs_mm: *bounds.get("max_abs_mm").expect("bound present") as f32,
        max_scan_lines: usize::try_from(*bounds.get("max_scan_lines").expect("bound present"))
            .expect("scan-line bound fits in usize"),
        max_line_width_mm: *bounds.get("max_line_width_mm").expect("bound present") as f32,
        max_purge_volume_mm3: *bounds.get("max_purge_volume_mm3").expect("bound present") as f32,
    }
}

fn control_limits() -> WipeLimits {
    WipeLimits {
        max_abs_mm: CONTROL_MAX_ABS_MM,
        max_scan_lines: CONTROL_MAX_SCAN_LINES,
        max_line_width_mm: CONTROL_MAX_LINE_WIDTH_MM,
        max_purge_volume_mm3: CONTROL_MAX_PURGE_VOLUME_MM3,
    }
}

fn tower_width_cap(limits: &WipeLimits) -> f32 {
    TOWER_WIDTH_CAP_MM.min(limits.max_abs_mm)
}

fn max_abs_units(limits: &WipeLimits) -> i64 {
    (f64::from(limits.max_abs_mm) / f64::from(GRID_MM)).round() as i64
}

fn tower_width_units_cap(limits: &WipeLimits) -> u32 {
    let units = (f64::from(tower_width_cap(limits)) / f64::from(GRID_MM)).round() as u32;
    units.max(1)
}

/// Powers of two in `[1/8, min(2 mm, max_line_width_mm)]`.
fn line_width_values(limits: &WipeLimits) -> Vec<f32> {
    let mut values = Vec::new();
    let mut value = 1.0f32 / 8.0;
    while value <= limits.max_line_width_mm && value <= 2.0 {
        values.push(value);
        value *= 2.0;
    }
    values
}

// ─────────────────────────────────────────────────────────────────────────────
// Domain
// ─────────────────────────────────────────────────────────────────────────────

/// Zero/positive mode, selected with `prop::sample::select(...).no_shrink()`.
/// Every zero mode is a separately labelled witness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ScaleMode {
    Positive,
    ZeroLineWidth,
    ZeroTowerWidth,
    ZeroVolume,
    ZeroLayerHeight,
}

/// `Positive` is repeated so the measured 64-case profile exercises the
/// positive domain more often than the degenerate modes.
const SCALE_MODES: [ScaleMode; 9] = [
    ScaleMode::Positive,
    ScaleMode::Positive,
    ScaleMode::Positive,
    ScaleMode::Positive,
    ScaleMode::Positive,
    ScaleMode::ZeroLineWidth,
    ScaleMode::ZeroTowerWidth,
    ScaleMode::ZeroVolume,
    ScaleMode::ZeroLayerHeight,
];

/// How `purge_volume` is constructed inside the positive mode. Selected with
/// `prop::sample::select(...).no_shrink()` so shrinking cannot move a witness
/// between the exact-class and classified-class populations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VolumeMode {
    /// `purge_depth` is an exact multiple of `line_width/2` (exact class).
    Grid,
    /// Off-grid quotient (classified class).
    Arbitrary,
    /// Push `purge_volume` to `max_purge_volume_mm3` exactly.
    Ceiling,
}

const VOLUME_MODES: [VolumeMode; 3] =
    [VolumeMode::Grid, VolumeMode::Arbitrary, VolumeMode::Ceiling];

/// Structured scan-row witnesses for the exact (`Grid`) volume mode, selected
/// with `prop::sample::select(...).no_shrink()` so the zero / one / many /
/// boundary row populations are structural rather than luck-based.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowEdge {
    Zero,
    One,
    Many,
    /// The largest odd `units`, so `n_real = units/2 - 1/2` is an exact integer
    /// and the last row lands exactly on `y_max` (dropped by the strict `<`).
    Boundary,
    Random,
}

/// `One`/`Many`/`Boundary` are repeated so the 64-case routine profile
/// exercises each row-count class several times.
const ROW_EDGES: [RowEdge; 9] = [
    RowEdge::Zero,
    RowEdge::One,
    RowEdge::One,
    RowEdge::One,
    RowEdge::Many,
    RowEdge::Many,
    RowEdge::Boundary,
    RowEdge::Boundary,
    RowEdge::Random,
];

/// Exact-class `units` of `line_width/2` for a row edge. `rows =
/// ceil(units / 2)` for an even `units`, so `One` is two units.
fn edge_units(edge: RowEdge, units_cap: u32, seed: [u32; 6]) -> f64 {
    let units = match edge {
        RowEdge::Zero => 0,
        RowEdge::One => 2,
        RowEdge::Many => 34,
        RowEdge::Boundary => {
            if units_cap.is_multiple_of(2) {
                units_cap.saturating_sub(1)
            } else {
                units_cap
            }
        }
        RowEdge::Random => (seed[5] >> 16) % (units_cap + 1),
    };
    f64::from(units.min(units_cap))
}

/// Convex bed witness shape, selected with
/// `prop::sample::select(...).no_shrink()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BedShape {
    Rectangle,
    RightTriangle,
}

const BED_SHAPES: [BedShape; 2] = [BedShape::Rectangle, BedShape::RightTriangle];

#[derive(Debug, Clone)]
struct WipeCase {
    line_width: f32,
    layer_height: f32,
    tower_x: f32,
    tower_y: f32,
    tower_width: f32,
    purge_volume: f32,
    layer_z: f32,
    global_layer_index: u32,
    tool_change: ToolChange,
    bed: BedPolygon,
    scale_mode: ScaleMode,
    volume_mode: VolumeMode,
    row_edge: RowEdge,
    bed_shape: BedShape,
}

/// Tower rectangle used by the containment and bed witnesses.
#[derive(Debug, Clone, Copy)]
struct TowerRect {
    x_min: f32,
    x_max: f32,
    y_min: f32,
    y_max: f32,
}

// ─────────────────────────────────────────────────────────────────────────────
// Grid and float helpers
// ─────────────────────────────────────────────────────────────────────────────

fn grid_mm(units: i64) -> f32 {
    units as f32 * GRID_MM
}

fn signed_units(seed: u32, bound_units: i64) -> i64 {
    let span = 2 * bound_units + 1;
    (i64::from(seed) % span) - bound_units
}

/// Mirror one f32 addition: the exact sum in `f64`, narrowed once. `f64` has
/// `53 >= 2*24+2` significand bits, so this equals the f32 operation.
fn add(a: f32, b: f32) -> f32 {
    (f64::from(a) + f64::from(b)) as f32
}

fn is_power_of_two(value: f32) -> bool {
    value.is_finite()
        && value > 0.0
        && value.to_bits() & 0x007f_ffff == 0
        && value.to_bits() >> 23 & 0xff != 0
        && value.to_bits() & 0x8000_0000 == 0
}

fn on_grid(value: f32) -> bool {
    value.is_finite() && (value * 16.0).fract() == 0.0
}

/// The documented cross-section formula with its two f32 storage points.
fn cross_section_f32(line_width: f32, layer_height: f32, tower_width: f32) -> f32 {
    let partial = (f64::from(line_width) * f64::from(layer_height)) as f32;
    (f64::from(partial) * f64::from(tower_width)) as f32
}

/// True when the cross-section product carried no rounding at all.
fn cross_section_is_exact(line_width: f32, layer_height: f32, tower_width: f32) -> bool {
    let cross = cross_section_f32(line_width, layer_height, tower_width);
    let real = f64::from(line_width) * f64::from(layer_height) * f64::from(tower_width);
    f64::from(cross) == real
}

fn purge_depth_mm(line_width: f32, layer_height: f32, tower_width: f32, purge_volume: f32) -> f32 {
    let cross = cross_section_f32(line_width, layer_height, tower_width);
    if !(cross > 0.0) {
        // The generator returns before computing a depth; no scan model exists.
        return 0.0;
    }
    (f64::from(purge_volume) / f64::from(cross)) as f32
}

fn purge_depth(case: &WipeCase) -> f32 {
    purge_depth_mm(
        case.line_width,
        case.layer_height,
        case.tower_width,
        case.purge_volume,
    )
}

fn tower_rect(case: &WipeCase) -> TowerRect {
    TowerRect {
        x_min: case.tower_x,
        x_max: add(case.tower_x, case.tower_width),
        y_min: case.tower_y,
        y_max: add(case.tower_y, purge_depth(case)),
    }
}

fn point_in_rect(rect: &TowerRect, x: f32, y: f32, envelope: f64) -> bool {
    let (x, y) = (f64::from(x), f64::from(y));
    x >= f64::from(rect.x_min) - envelope
        && x <= f64::from(rect.x_max) + envelope
        && y >= f64::from(rect.y_min) - envelope
        && y <= f64::from(rect.y_max) + envelope
}

/// Derived one-rounding envelope (see the module docs).
fn envelope_mm(limits: &WipeLimits) -> f64 {
    let depth_cap = limits.max_scan_lines as f64 * f64::from(limits.max_line_width_mm);
    let size_cap = depth_cap.max(f64::from(tower_width_cap(limits)));
    let operand_cap = f64::from(limits.max_abs_mm) + size_cap;
    operand_cap * 2f64.powi(-23)
}

// ─────────────────────────────────────────────────────────────────────────────
// Beds
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct BedPolygon {
    /// Closed polygon in declaration order.
    vertices: Vec<(f32, f32)>,
    /// `true` for a counter-clockwise convex polygon (half-plane containment);
    /// `false` for a simple concave polygon (even-odd containment).
    convex_ccw: bool,
}

impl BedPolygon {
    fn contains(&self, x: f32, y: f32, tolerance: f64) -> bool {
        if self.convex_ccw {
            self.contains_convex(x, y, tolerance)
        } else {
            self.contains_even_odd(x, y)
        }
    }

    /// Half-plane containment for a convex, counter-clockwise polygon.
    fn contains_convex(&self, x: f32, y: f32, tolerance: f64) -> bool {
        let n = self.vertices.len();
        if n < 3 {
            return false;
        }
        for index in 0..n {
            let a = self.vertices[index];
            let b = self.vertices[(index + 1) % n];
            let cross = (f64::from(b.0) - f64::from(a.0)) * (f64::from(y) - f64::from(a.1))
                - (f64::from(b.1) - f64::from(a.1)) * (f64::from(x) - f64::from(a.0));
            if cross < -tolerance {
                return false;
            }
        }
        true
    }

    /// Even-odd ray casting for the deterministic concave control.
    fn contains_even_odd(&self, x: f32, y: f32) -> bool {
        let n = self.vertices.len();
        if n < 3 {
            return false;
        }
        let (px, py) = (f64::from(x), f64::from(y));
        let mut inside = false;
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi) = (f64::from(self.vertices[i].0), f64::from(self.vertices[i].1));
            let (xj, yj) = (f64::from(self.vertices[j].0), f64::from(self.vertices[j].1));
            if ((yi > py) != (yj > py)) && (px < (xj - xi) * (py - yi) / (yj - yi) + xi) {
                inside = !inside;
            }
            j = i;
        }
        inside
    }

    /// Independent convexity/CCW witness for the generated bed polygons.
    fn is_convex_ccw(&self) -> bool {
        let n = self.vertices.len();
        if n < 3 {
            return false;
        }
        for index in 0..n {
            let a = self.vertices[index];
            let b = self.vertices[(index + 1) % n];
            let c = self.vertices[(index + 2) % n];
            let cross = (f64::from(b.0) - f64::from(a.0)) * (f64::from(c.1) - f64::from(b.1))
                - (f64::from(b.1) - f64::from(a.1)) * (f64::from(c.0) - f64::from(b.0));
            if cross <= 0.0 {
                return false;
            }
        }
        true
    }
}

fn grid_floor_units(value: f32) -> i64 {
    (f64::from(value) / f64::from(GRID_MM)).floor() as i64
}

fn grid_ceil_units(value: f32) -> i64 {
    (f64::from(value) / f64::from(GRID_MM)).ceil() as i64
}

/// Convex bed containing the tower rectangle, padded by `pad_units` grid cells
/// around the grid-rounded footprint.
fn build_bed(
    tower_x: f32,
    tower_width: f32,
    tower_y: f32,
    purge_depth: f32,
    shape: BedShape,
    pad_units: i64,
) -> BedPolygon {
    let x_max = add(tower_x, tower_width);
    let y_max = add(tower_y, purge_depth);
    let x0 = grid_mm(grid_floor_units(tower_x) - pad_units);
    let x1 = grid_mm(grid_ceil_units(x_max) + pad_units);
    let y0 = grid_mm(grid_floor_units(tower_y) - pad_units);
    let y1 = grid_mm(grid_ceil_units(y_max) + pad_units);
    let vertices = match shape {
        BedShape::Rectangle => vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)],
        BedShape::RightTriangle => {
            // Legs are 4x the padded tower extents, so the far tower corner
            // sits at (x1-x0)/(4(x1-x0)) + (y1-y0)/(4(y1-y0)) = 1/2 of the
            // hypotenuse: strictly inside, with the whole rectangle.
            let leg_x = 4.0 * (x1 - x0);
            let leg_y = 4.0 * (y1 - y0);
            vec![(x0, y0), (x0 + leg_x, y0), (x0, y0 + leg_y)]
        }
    };
    BedPolygon {
        vertices,
        convex_ccw: true,
    }
}

fn check_bed_containment(
    points: &[(f32, f32)],
    bed: &BedPolygon,
    corner_only: bool,
    tolerance: f64,
) -> Result<(), String> {
    if points.is_empty() {
        return Err("no generated points to validate against the bed".to_string());
    }
    if corner_only {
        // The rejected policy: validate only the four corners of the point
        // set's axis-aligned bounding box.
        let mut x0 = f32::INFINITY;
        let mut x1 = f32::NEG_INFINITY;
        let mut y0 = f32::INFINITY;
        let mut y1 = f32::NEG_INFINITY;
        for (x, y) in points {
            x0 = x0.min(*x);
            x1 = x1.max(*x);
            y0 = y0.min(*y);
            y1 = y1.max(*y);
        }
        for (x, y) in [(x0, y0), (x1, y0), (x1, y1), (x0, y1)] {
            if !bed.contains(x, y, tolerance) {
                return Err(format!(
                    "corner-only bed validation rejected corner ({x}, {y})"
                ));
            }
        }
        return Ok(());
    }
    for (x, y) in points {
        if !bed.contains(*x, *y, tolerance) {
            return Err(format!(
                "bed polygon does not contain generated point ({x}, {y})"
            ));
        }
    }
    Ok(())
}

/// The rectangle-inside-bed part of the witness (see deviation 2 in the module
/// docs): the four envelope-inflated rectangle corners must be inside the bed.
fn check_bed_contains_tower_rectangle(
    bed: &BedPolygon,
    rect: &TowerRect,
    envelope: f64,
) -> Result<(), String> {
    let e = envelope as f32;
    let corners = [
        (rect.x_min - e, rect.y_min - e),
        (rect.x_max + e, rect.y_min - e),
        (rect.x_max + e, rect.y_max + e),
        (rect.x_min - e, rect.y_max + e),
    ];
    for (x, y) in corners {
        if !bed.contains(x, y, envelope) {
            return Err(format!(
                "bed polygon does not contain the tower rectangle corner ({x}, {y})"
            ));
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Case construction (construction-first; no rejection)
// ─────────────────────────────────────────────────────────────────────────────

fn ceiling_line_widths(limits: &WipeLimits) -> Vec<f32> {
    [0.5f32, 1.0, 2.0]
        .into_iter()
        .filter(|value| *value <= limits.max_line_width_mm)
        .collect()
}

/// `(line_width, layer_height, tower_width)` whose volume budget reaches
/// `max_purge_volume_mm3` exactly.
fn ceiling_shape(limits: &WipeLimits, seed: u32) -> Option<(f32, f32, f32)> {
    let widths = ceiling_line_widths(limits);
    if widths.is_empty() {
        return None;
    }
    let line_width = widths[seed as usize % widths.len()];
    Some((line_width, 0.5, tower_width_cap(limits)))
}

/// Volume for the positive mode, constructed from the profile bounds so the
/// scan budget can never be exceeded.
fn volume_mm3(
    line_width: f32,
    layer_height: f32,
    tower_width: f32,
    volume_mode: VolumeMode,
    row_edge: RowEdge,
    seed: [u32; 6],
    limits: &WipeLimits,
) -> f32 {
    let cross = cross_section_f32(line_width, layer_height, tower_width);
    if !(cross > 0.0) {
        return 0.0;
    }
    let half = line_width * 0.5;
    let per_unit = f64::from(half) * f64::from(cross);
    if !(per_unit > 0.0) {
        return 0.0;
    }
    let volume_budget = f64::from(limits.max_purge_volume_mm3) / per_unit;
    // `purge_depth = units * line_width/2`, so `purge_depth / line_width =
    // units / 2` and `n_upper = ceil(units / 2) + 1`; the scan budget keeps
    // `n_upper <= max_scan_lines` with two units of margin for the arbitrary
    // fractional mode.
    let scan_budget = 2.0 * (limits.max_scan_lines as f64 - 1.0) - 2.0;
    let units_cap = volume_budget.floor().min(scan_budget).max(0.0);
    let units_cap = if units_cap.is_finite() {
        units_cap as u32
    } else {
        0
    };
    let raw_units = match volume_mode {
        VolumeMode::Grid => edge_units(row_edge, units_cap, seed),
        VolumeMode::Arbitrary => {
            let units = if units_cap > 0 {
                (u64::from(seed[5] >> 8) % u64::from(units_cap)) as f64
            } else {
                0.0
            };
            let fraction = (f64::from((seed[3] >> 16) % 64) + 1.0) / 64.0;
            units + fraction
        }
        VolumeMode::Ceiling => volume_budget.floor().min(scan_budget).max(0.0),
    };
    let volume = raw_units * per_unit;
    (volume as f32).min(limits.max_purge_volume_mm3)
}

fn tool_change_fixture(seed: u32) -> ToolChange {
    ToolChange {
        after_entity_index: seed % 8,
        to_tool: (seed % 3) + 1,
        ..ToolChange::default()
    }
}

fn build_case(
    scale_mode: ScaleMode,
    volume_mode: VolumeMode,
    row_edge: RowEdge,
    bed_shape: BedShape,
    seed: [u32; 6],
    limits: &WipeLimits,
) -> WipeCase {
    let widths = line_width_values(limits);
    let mut line_width = widths[seed[0] as usize % widths.len()];
    let mut layer_height = grid_mm(1 + i64::from(seed[4] % LAYER_HEIGHT_UNITS_MAX));
    let mut tower_width = grid_mm(1 + i64::from(seed[5] % tower_width_units_cap(limits)));
    if scale_mode == ScaleMode::Positive && volume_mode == VolumeMode::Ceiling {
        if let Some((width, height, tower)) = ceiling_shape(limits, seed[0]) {
            line_width = width;
            layer_height = height;
            tower_width = tower;
        }
    }
    let volume = volume_mm3(
        line_width,
        layer_height,
        tower_width,
        volume_mode,
        row_edge,
        seed,
        limits,
    );
    let (line_width, tower_width, layer_height, purge_volume) = match scale_mode {
        ScaleMode::Positive => (line_width, tower_width, layer_height, volume),
        ScaleMode::ZeroLineWidth => (0.0, tower_width, layer_height, volume),
        ScaleMode::ZeroTowerWidth => (line_width, 0.0, layer_height, volume),
        ScaleMode::ZeroLayerHeight => (line_width, tower_width, 0.0, volume),
        ScaleMode::ZeroVolume => (line_width, tower_width, layer_height, 0.0),
    };
    let tower_x = grid_mm(signed_units(seed[1], max_abs_units(limits)));
    let tower_y = grid_mm(signed_units(seed[2], max_abs_units(limits)));
    let layer_z = grid_mm(signed_units(seed[3], max_abs_units(limits)));
    let global_layer_index = seed[0] % (limits.max_abs_mm.round() as u32 + 1);
    let pad_units = BED_PAD_UNIT_CHOICES[(seed[3] as usize) % BED_PAD_UNIT_CHOICES.len()];
    let bed = build_bed(
        tower_x,
        tower_width,
        tower_y,
        purge_depth_mm(line_width, layer_height, tower_width, purge_volume),
        bed_shape,
        pad_units,
    );

    WipeCase {
        line_width,
        layer_height,
        tower_x,
        tower_y,
        tower_width,
        purge_volume,
        layer_z,
        global_layer_index,
        tool_change: tool_change_fixture(seed[1]),
        bed,
        scale_mode,
        volume_mode,
        row_edge,
        bed_shape,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures through the real constructor
// ─────────────────────────────────────────────────────────────────────────────

fn config_for(case: &WipeCase) -> ConfigView {
    let bed_values: Vec<ConfigValue> = case
        .bed
        .vertices
        .iter()
        .flat_map(|(x, y)| {
            [
                ConfigValue::Float(f64::from(*x)),
                ConfigValue::Float(f64::from(*y)),
            ]
        })
        .collect();
    ConfigView::from_map(HashMap::from([
        (
            "wipe_tower_x".to_string(),
            ConfigValue::Float(f64::from(case.tower_x)),
        ),
        (
            "wipe_tower_y".to_string(),
            ConfigValue::Float(f64::from(case.tower_y)),
        ),
        (
            "prime_tower_width".to_string(),
            ConfigValue::Float(f64::from(case.tower_width)),
        ),
        (
            "prime_volume".to_string(),
            ConfigValue::Float(f64::from(case.purge_volume)),
        ),
        (
            "line_width".to_string(),
            ConfigValue::Float(f64::from(case.line_width)),
        ),
        ("retract_length".to_string(), ConfigValue::Float(2.0)),
        ("enable_prime_tower".to_string(), ConfigValue::Bool(true)),
        ("printable_area".to_string(), ConfigValue::List(bed_values)),
    ]))
}

fn build_tower(case: &WipeCase) -> Result<WipeTower, String> {
    let config = config_for(case);
    WipeTower::from_config(&config).map_err(|error| format!("from_config: {}", error.message))
}

fn generate(case: &WipeCase, tower: &WipeTower) -> Vec<(ExtrusionPath3D, RegionKey)> {
    tower.generate_purge_paths(
        case.layer_z,
        case.layer_height,
        case.global_layer_index,
        &case.tool_change,
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Domain validation (guards against generator regressions)
// ─────────────────────────────────────────────────────────────────────────────

fn validate_case_domain(case: &WipeCase, limits: &WipeLimits) -> Result<(), String> {
    if !case.line_width.is_finite() || case.line_width < 0.0 {
        return Err(format!(
            "line_width {} is not a finite nonnegative size",
            case.line_width
        ));
    }
    if case.line_width != 0.0
        && !(is_power_of_two(case.line_width)
            && case.line_width >= 1.0 / 8.0
            && case.line_width <= limits.max_line_width_mm)
    {
        return Err(format!(
            "line_width {} is not a power of two in [1/8, {}]",
            case.line_width, limits.max_line_width_mm
        ));
    }
    if !case.tower_width.is_finite() || case.tower_width < 0.0 {
        return Err(format!(
            "tower_width {} is not a finite nonnegative size",
            case.tower_width
        ));
    }
    if case.tower_width != 0.0
        && !(on_grid(case.tower_width)
            && case.tower_width >= GRID_MM
            && case.tower_width <= tower_width_cap(limits))
    {
        return Err(format!(
            "tower_width {} is not a positive 1/16 mm multiple below {}",
            case.tower_width,
            tower_width_cap(limits)
        ));
    }
    // 0.2 mm is the module's documented default and is deliberately allowed as
    // a labelled control witness even though it is off the 1/16 mm grid.
    let layer_height_ok = case.layer_height == 0.0
        || (case.layer_height >= GRID_MM
            && case.layer_height <= GRID_MM * LAYER_HEIGHT_UNITS_MAX as f32
            && on_grid(case.layer_height))
        || case.layer_height == PRODUCTION_DEFAULT_LAYER_HEIGHT_MM;
    if !layer_height_ok {
        return Err(format!(
            "layer_height {} is not a 1/16 mm multiple in [1/16, 1/2] (or the labelled 0.2 mm witness)",
            case.layer_height
        ));
    }
    for (name, value) in [
        ("tower_x", case.tower_x),
        ("tower_y", case.tower_y),
        ("layer_z", case.layer_z),
    ] {
        if !on_grid(value) || value.abs() > limits.max_abs_mm {
            return Err(format!(
                "{name} = {value} is not a 1/16 mm multiple inside +/-{}",
                limits.max_abs_mm
            ));
        }
    }
    if !case.purge_volume.is_finite()
        || case.purge_volume < 0.0
        || case.purge_volume > limits.max_purge_volume_mm3
    {
        return Err(format!(
            "purge_volume {} is not inside [0, {}]",
            case.purge_volume, limits.max_purge_volume_mm3
        ));
    }
    if case.global_layer_index > limits.max_abs_mm.round() as u32 {
        return Err(format!(
            "global_layer_index {} exceeds the {}-index bound",
            case.global_layer_index, limits.max_abs_mm
        ));
    }
    if case.bed.vertices.len() < 3 {
        return Err(format!(
            "bed polygon has {} vertices, needs at least 3",
            case.bed.vertices.len()
        ));
    }
    for (x, y) in &case.bed.vertices {
        if !on_grid(*x) || !on_grid(*y) {
            return Err(format!("bed vertex ({x}, {y}) is not on the 1/16 mm grid"));
        }
    }
    if case.bed.convex_ccw && !case.bed.is_convex_ccw() {
        return Err("declared convex bed is not convex and counter-clockwise".to_string());
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Independent scan-row model
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct RowModel {
    /// Model row positions, in emission order.
    ys: Vec<f32>,
    /// Rational quotient `(y_max - y_first) / line_width`.
    n_real: f64,
    /// Every addition on the f32 grid was exact: the closed form is the count.
    exact: bool,
    /// `n_real` is within 1e-3 of an integer (counted for the report).
    near_integer: bool,
    /// `purge_depth / line_width` is exactly an integer.
    integer_quotient: bool,
    /// `n_real` is exactly an integer: the last scan row lands exactly on
    /// `y_max` and the strict `<` drops it.
    boundary_rows: bool,
    y_max: f32,
    x_min: f32,
    x_max: f32,
}

fn row_model(case: &WipeCase, cross: f32, limits: &WipeLimits) -> Result<RowModel, String> {
    if !(cross > 0.0) {
        return Err(format!(
            "domain: cross-section {cross} is not positive in the row model"
        ));
    }
    let recomputed_cross = cross_section_f32(case.line_width, case.layer_height, case.tower_width);
    if recomputed_cross != cross {
        return Err(format!(
            "model self-check: cross-section {cross} disagrees with the recomputed {recomputed_cross}"
        ));
    }
    let line_width = case.line_width;
    if !(line_width > 0.0) {
        return Err(format!(
            "domain: line_width {line_width} is not positive in the positive-cross-section branch"
        ));
    }
    let half = line_width * 0.5;
    if !(half >= GRID_MM) {
        return Err(format!(
            "domain: line_width/2 = {half} is below the 1/16 mm grid"
        ));
    }
    let depth = purge_depth(case);
    if !depth.is_finite() || depth < 0.0 {
        return Err(format!(
            "domain: purge_depth {depth} is not finite and nonnegative"
        ));
    }
    let quotient = f64::from(depth) / f64::from(line_width);
    if !quotient.is_finite() {
        return Err(format!(
            "domain: purge_depth / line_width = {quotient} is not finite"
        ));
    }
    let n_upper_value = quotient.ceil() + 1.0;
    if !(n_upper_value <= limits.max_scan_lines as f64) {
        return Err(format!(
            "domain/exhaustion: n_upper = ceil(purge_depth/line_width)+1 = {n_upper_value} exceeds max_scan_lines = {}",
            limits.max_scan_lines
        ));
    }
    let n_upper = n_upper_value as usize;

    let y_min = case.tower_y;
    let y_max = add(y_min, depth);
    if !y_max.is_finite() {
        return Err(format!("domain: y_max {y_max} is not finite"));
    }
    let y_first = add(y_min, half);

    let mut ys = Vec::new();
    let mut y = y_first;
    while y < y_max {
        if ys.len() >= n_upper {
            return Err(format!(
                "domain/exhaustion: model row loop exceeded n_upper = {n_upper} at y = {y}"
            ));
        }
        if !y.is_finite() {
            return Err(format!("domain: scan row y = {y} is not finite"));
        }
        ys.push(y);
        let next = add(y, line_width);
        if !(next > y) {
            return Err(format!(
                "domain: y += line_width did not increase (y = {y}, next = {next})"
            ));
        }
        y = next;
    }

    let n_real = (f64::from(y_max) - f64::from(y_first)) / f64::from(line_width);
    let closed = if n_real > 0.0 {
        n_real.ceil() as usize
    } else {
        0
    };
    if closed != ys.len() {
        return Err(format!(
            "model self-check: closed-form ceil {closed} disagrees with the {}-row step loop",
            ys.len()
        ));
    }
    let depth_on_half_grid = {
        let units = f64::from(depth) / f64::from(half);
        units.fract() == 0.0
    };
    let y_max_exact = f64::from(y_max) == f64::from(y_min) + f64::from(depth);
    let cross_exact = cross_section_is_exact(case.line_width, case.layer_height, case.tower_width);
    let exact = depth_on_half_grid && y_max_exact && cross_exact;
    let rounded = quotient.round();

    Ok(RowModel {
        ys,
        n_real,
        exact,
        near_integer: (quotient - rounded).abs() <= 1e-3,
        integer_quotient: quotient == rounded,
        boundary_rows: n_real == n_real.round(),
        y_max,
        x_min: case.tower_x,
        x_max: add(case.tower_x, case.tower_width),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Checker (shared by the property and the controls)
// ─────────────────────────────────────────────────────────────────────────────

/// Mutation knobs for the deterministic controls. The property always uses
/// [`OracleModel::default`].
#[derive(Debug, Clone, Copy, Default)]
struct OracleModel {
    /// Predict one scan row one line width above the tower rectangle.
    outside_scan_row: bool,
    /// Predict one scan row too many.
    scan_count_off_by_one: bool,
    /// Predict the prime endpoint one `tower_width` past the cap.
    prime_beyond_width: bool,
    /// Claim a positive prime length even for the zero-volume witness.
    require_positive_prime: bool,
    /// Validate only the four corners of the point set's bounding box.
    corner_only_bed: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct WipeObservation {
    cross_positive: bool,
    purge_positive: bool,
    observed_rows: usize,
    expected_rows: usize,
    exact_rows: bool,
    near_integer: bool,
    integer_quotient: bool,
    boundary_rows: bool,
    ceiling_volume: bool,
}

fn check_path_shape(
    case: &WipeCase,
    path: &ExtrusionPath3D,
    key: &RegionKey,
    index: usize,
) -> Result<(), String> {
    if path.role != ExtrusionRole::WipeTower {
        return Err(format!(
            "pair {index} has role {:?}, expected WipeTower",
            path.role
        ));
    }
    if path.speed_factor != 1.0 {
        return Err(format!(
            "pair {index} has speed_factor {}, expected 1.0",
            path.speed_factor
        ));
    }
    if path.tool_index.is_some() {
        return Err(format!("pair {index} carries a tool_index"));
    }
    if path.order_lock.is_some() {
        return Err(format!("pair {index} carries an order_lock"));
    }
    if key.object_id != "__wipe_tower__" {
        return Err(format!(
            "pair {index} has object_id {:?}, expected \"__wipe_tower__\"",
            key.object_id
        ));
    }
    if key.region_id != 0 {
        return Err(format!(
            "pair {index} has region_id {}, expected 0",
            key.region_id
        ));
    }
    if !key.variant_chain.is_empty() {
        return Err(format!(
            "pair {index} has a nonempty variant_chain of {} axes",
            key.variant_chain.len()
        ));
    }
    if key.global_layer_index != case.global_layer_index {
        return Err(format!(
            "pair {index} has global_layer_index {}, expected {}",
            key.global_layer_index, case.global_layer_index
        ));
    }
    if path.points.len() != 2 {
        return Err(format!(
            "pair {index} has {} points, expected 2",
            path.points.len()
        ));
    }
    for (point_index, point) in path.points.iter().enumerate() {
        check_point_shape(case, point, index, point_index)?;
    }
    Ok(())
}

fn check_point_shape(
    case: &WipeCase,
    point: &Point3WithWidth,
    pair: usize,
    index: usize,
) -> Result<(), String> {
    if point.z != case.layer_z {
        return Err(format!(
            "pair {pair} point {index} has z {}, expected the z argument {}",
            point.z, case.layer_z
        ));
    }
    if point.width != case.line_width {
        return Err(format!(
            "pair {pair} point {index} has width {}, expected line_width {}",
            point.width, case.line_width
        ));
    }
    if point.overhang_quartile.is_some() {
        return Err(format!(
            "pair {pair} point {index} carries an overhang_quartile"
        ));
    }
    if point.dist_to_top_mm != 0.0 {
        return Err(format!(
            "pair {pair} point {index} has dist_to_top_mm {}",
            point.dist_to_top_mm
        ));
    }
    if point.overhang_distance_mm.is_some() {
        return Err(format!(
            "pair {pair} point {index} carries an overhang_distance_mm"
        ));
    }
    if !point.x.is_finite() || !point.y.is_finite() {
        return Err(format!(
            "pair {pair} point {index} is not finite: ({}, {})",
            point.x, point.y
        ));
    }
    Ok(())
}

fn check_travel(case: &WipeCase, path: &ExtrusionPath3D) -> Result<(), String> {
    let start = &path.points[0];
    let end = &path.points[1];
    if start.x != case.tower_x || start.y != case.tower_y {
        return Err(format!(
            "travel pair starts at ({}, {}), expected ({}, {})",
            start.x, start.y, case.tower_x, case.tower_y
        ));
    }
    if end.x != case.tower_x || end.y != case.tower_y {
        return Err(format!(
            "travel pair ends at ({}, {}), expected ({}, {})",
            end.x, end.y, case.tower_x, case.tower_y
        ));
    }
    if start.flow_factor != 0.0 || end.flow_factor != 0.0 {
        return Err(format!(
            "travel pair carries flow ({}, {}), expected zero flow",
            start.flow_factor, end.flow_factor
        ));
    }
    Ok(())
}

fn check_prime(case: &WipeCase, path: &ExtrusionPath3D, model: &OracleModel) -> Result<(), String> {
    let denominator = (f64::from(case.line_width) * f64::from(case.layer_height)) as f32;
    let full = if case.layer_height > 0.0 && denominator > 0.0 {
        (f64::from(case.purge_volume) / f64::from(denominator)) as f32
    } else {
        0.0
    };
    let capped = full.min(case.tower_width);
    let expected_length = if model.prime_beyond_width {
        capped + case.tower_width
    } else {
        capped
    };
    let expected_x = add(case.tower_x, expected_length);

    let start = &path.points[0];
    let end = &path.points[1];
    if start.x != case.tower_x || start.y != case.tower_y {
        return Err(format!(
            "prime start ({}, {}), expected ({}, {})",
            start.x, start.y, case.tower_x, case.tower_y
        ));
    }
    if end.x != expected_x {
        return Err(format!(
            "prime end x {} does not equal the capped expectation {} (prime_length {}, capped {})",
            end.x, expected_x, expected_length, capped
        ));
    }
    if end.y != case.tower_y {
        return Err(format!(
            "prime end y {} does not equal tower_y {}",
            end.y, case.tower_y
        ));
    }
    if start.flow_factor != 0.0 {
        return Err(format!(
            "prime first point carries flow {}, expected 0.0",
            start.flow_factor
        ));
    }
    if end.flow_factor != 1.0 {
        return Err(format!(
            "prime second point carries flow {}, expected 1.0",
            end.flow_factor
        ));
    }
    let length = end.x - start.x;
    let require_positive = model.require_positive_prime || case.purge_volume > 0.0;
    if require_positive && !(length > 0.0) {
        return Err(format!(
            "prime segment length {length} is claimed positive but is not (purge_volume {})",
            case.purge_volume
        ));
    }
    if case.purge_volume == 0.0 && length != 0.0 {
        return Err(format!(
            "zero-volume prime must be zero-length, observed {length}"
        ));
    }
    if length > case.tower_width {
        return Err(format!(
            "prime segment length {length} exceeds tower_width {}",
            case.tower_width
        ));
    }
    Ok(())
}

fn check_scan_row(
    case: &WipeCase,
    path: &ExtrusionPath3D,
    index: usize,
    expected_y: f32,
    rows: &RowModel,
) -> Result<(), String> {
    let start = &path.points[0];
    let end = &path.points[1];
    if start.y != expected_y || end.y != expected_y {
        return Err(format!(
            "scan row {index} has y ({}, {}), expected {expected_y}",
            start.y, end.y
        ));
    }
    let forward = index.is_multiple_of(2);
    let (expected_start, expected_end) = if forward {
        (rows.x_min, rows.x_max)
    } else {
        (rows.x_max, rows.x_min)
    };
    if start.x != expected_start || end.x != expected_end {
        return Err(format!(
            "scan row {index} runs ({}, {}), expected ({expected_start}, {expected_end}) for {} order",
            start.x,
            end.x,
            if forward { "forward" } else { "reverse" }
        ));
    }
    if start.flow_factor != 1.0 || end.flow_factor != 1.0 {
        return Err(format!(
            "scan row {index} carries flow ({}, {}), expected 1.0 at both points",
            start.flow_factor, end.flow_factor
        ));
    }
    // The geometric (unsigned) stroke length: reverse rows run x_max -> x_min.
    let length = (end.x - start.x).abs();
    if !(length > 0.0) {
        return Err(format!(
            "scan row {index} has non-positive extrusion length {length}"
        ));
    }
    if length != case.tower_width {
        return Err(format!(
            "scan row {index} length {length} does not equal tower_width {}",
            case.tower_width
        ));
    }
    Ok(())
}

fn check_purge_paths(
    case: &WipeCase,
    pairs: &[(ExtrusionPath3D, RegionKey)],
    model: &OracleModel,
    limits: &WipeLimits,
) -> Result<WipeObservation, String> {
    validate_case_domain(case, limits)?;

    let cross = cross_section_f32(case.line_width, case.layer_height, case.tower_width);
    let mut observation = WipeObservation {
        cross_positive: cross > 0.0,
        ..WipeObservation::default()
    };

    if !(cross > 0.0) {
        if !pairs.is_empty() {
            return Err(format!(
                "cross-section {cross} must yield no purge pairs, got {}",
                pairs.len()
            ));
        }
        return Ok(observation);
    }

    observation.purge_positive = case.purge_volume > 0.0;
    observation.ceiling_volume = case.purge_volume == limits.max_purge_volume_mm3;

    let rect = tower_rect(case);
    let envelope = envelope_mm(limits);

    if pairs.len() < 2 {
        return Err(format!(
            "expected at least one travel pair and one prime pair, got {}",
            pairs.len()
        ));
    }
    for (index, (path, key)) in pairs.iter().enumerate() {
        check_path_shape(case, path, key, index)?;
    }
    check_travel(case, &pairs[0].0)?;
    check_prime(case, &pairs[pairs.len() - 1].0, model)?;

    let rows = row_model(case, cross, limits)?;
    let observed_rows = pairs.len() - 2;

    // Every generated point stays inside the closed tower rectangle.
    for (index, (path, _)) in pairs.iter().enumerate() {
        for point in &path.points {
            if !point_in_rect(&rect, point.x, point.y, envelope) {
                return Err(format!(
                    "pair {index} point ({}, {}) lies outside the tower rectangle x[{}..{}] y[{}..{}]",
                    point.x, point.y, rect.x_min, rect.x_max, rect.y_min, rect.y_max
                ));
            }
        }
    }

    // Every predicted scan row must also stay inside the rectangle; the
    // `outside_scan_row` mutation appends a row one line width above `y_max`,
    // which this same predicate rejects.
    let mut predicted_rows = rows.ys.clone();
    if model.outside_scan_row {
        predicted_rows.push(add(rows.y_max, case.line_width));
    }
    for (index, y) in predicted_rows.iter().enumerate() {
        if !point_in_rect(&rect, case.tower_x, *y, envelope) {
            return Err(format!(
                "predicted scan row {index} at y = {y} lies outside the tower rectangle y[{}..{}]",
                rect.y_min, rect.y_max
            ));
        }
    }

    let expected_count = rows.ys.len() + usize::from(model.scan_count_off_by_one);
    if rows.exact {
        if observed_rows != expected_count {
            return Err(format!(
                "exact scan-row count: rational ceil = {expected_count}, observed {observed_rows} (n_real = {})",
                rows.n_real
            ));
        }
    } else if observed_rows.abs_diff(expected_count) > 1 {
        return Err(format!(
            "classified scan-row count: observed {observed_rows} is more than one away from the rational ceil {expected_count} (n_real = {})",
            rows.n_real
        ));
    }
    let common = observed_rows.min(rows.ys.len());
    for index in 0..common {
        check_scan_row(case, &pairs[1 + index].0, index, rows.ys[index], &rows)?;
    }

    // test-quality: scene witness (Deviation 2) — the convex bed is built
    // around the tower rectangle, so this proves internal consistency, not the
    // production `run_finalization` bed rejection; that production path has its
    // own deterministic tests. The witness runs only in the positive-volume
    // mode; the zero-volume mode claims exact counts and tower-rectangle
    // closure, not enclosure beyond the rectangle.
    if observation.purge_positive {
        check_bed_contains_tower_rectangle(&case.bed, &rect, envelope)?;
        let points: Vec<(f32, f32)> = pairs
            .iter()
            .flat_map(|(path, _)| path.points.iter().map(|point| (point.x, point.y)))
            .collect();
        check_bed_containment(&points, &case.bed, model.corner_only_bed, envelope)?;
    }

    observation.observed_rows = observed_rows;
    observation.expected_rows = expected_count;
    observation.exact_rows = rows.exact;
    observation.near_integer = rows.near_integer;
    observation.integer_quotient = rows.integer_quotient;
    observation.boundary_rows = rows.boundary_rows;
    Ok(observation)
}

// ─────────────────────────────────────────────────────────────────────────────
// Property
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, Default)]
struct Counters {
    cases: u64,
    positive_cross: u64,
    empty_cross: u64,
    zero_line_width: u64,
    zero_tower_width: u64,
    zero_layer_height: u64,
    zero_volume: u64,
    scan_rows_positive: u64,
    rows_zero: u64,
    rows_one: u64,
    rows_many: u64,
    exact_rows: u64,
    classified_rows: u64,
    near_integer: u64,
    integer_quotient: u64,
    boundary_rows: u64,
    ceiling_volume: u64,
    rectangle_bed: u64,
    triangle_bed: u64,
}

#[test]
fn proptest_generate_purge_paths() {
    let run = PropertyRun::new(PROPERTY_ID, PROPERTY_TEST).expect("H-WIPE-TOWER context");
    let limits = live_limits(&run);
    assert!(
        !line_width_values(&limits).is_empty(),
        "profile max_line_width_mm {} is below 1/8 mm",
        limits.max_line_width_mm
    );
    assert!(
        tower_width_cap(&limits) >= 1.0,
        "profile max_abs_mm {} is too small for the tower-width cap",
        limits.max_abs_mm
    );
    assert!(
        ceiling_witness_reaches_bound(&limits),
        "profile bounds cannot form the volume-ceiling witness"
    );

    let strategy = (
        prop::sample::select(SCALE_MODES.to_vec()).no_shrink(),
        prop::sample::select(VOLUME_MODES.to_vec()).no_shrink(),
        prop::sample::select(ROW_EDGES.to_vec()).no_shrink(),
        prop::sample::select(BED_SHAPES.to_vec()).no_shrink(),
        any::<[u32; 6]>(),
    )
        .prop_map(
            move |(scale_mode, volume_mode, row_edge, bed_shape, seed)| {
                build_case(scale_mode, volume_mode, row_edge, bed_shape, seed, &limits)
            },
        );

    let counters = Cell::new(Counters::default());

    run.check(&strategy, |case, _guard| {
        let tower = match build_tower(&case) {
            Ok(tower) => tower,
            Err(message) => return fail(format!("{message} on {case:?}")),
        };
        let pairs = generate(&case, &tower);
        let observation = match check_purge_paths(&case, &pairs, &OracleModel::default(), &limits) {
            Ok(observation) => observation,
            Err(message) => return fail(format!("{message} on {case:?}")),
        };

        let mut next = counters.get();
        next.cases += 1;
        if observation.cross_positive {
            next.positive_cross += 1;
        } else {
            next.empty_cross += 1;
        }
        match case.scale_mode {
            ScaleMode::ZeroLineWidth => next.zero_line_width += 1,
            ScaleMode::ZeroTowerWidth => next.zero_tower_width += 1,
            ScaleMode::ZeroLayerHeight => next.zero_layer_height += 1,
            ScaleMode::ZeroVolume => next.zero_volume += 1,
            ScaleMode::Positive => {}
        }
        if observation.observed_rows > 0 {
            next.scan_rows_positive += 1;
        }
        if observation.cross_positive {
            if observation.observed_rows == 0 {
                next.rows_zero += 1;
            } else if observation.observed_rows == 1 {
                next.rows_one += 1;
            }
            if observation.observed_rows >= 8 {
                next.rows_many += 1;
            }
            if observation.exact_rows {
                next.exact_rows += 1;
            } else {
                next.classified_rows += 1;
            }
        }
        if observation.near_integer {
            next.near_integer += 1;
        }
        if observation.integer_quotient {
            next.integer_quotient += 1;
        }
        if observation.boundary_rows {
            next.boundary_rows += 1;
        }
        if observation.ceiling_volume {
            next.ceiling_volume += 1;
        }
        match case.bed_shape {
            BedShape::Rectangle => next.rectangle_bed += 1,
            BedShape::RightTriangle => next.triangle_bed += 1,
        }
        counters.set(next);
        Ok(())
    })
    .expect("H-WIPE-TOWER purge/prime path model");

    let totals = counters.get();
    println!("H-WIPE-TOWER populations: {totals:?}");
    assert!(totals.cases > 0, "no wipe-tower case was generated");
    assert!(
        totals.positive_cross > 0,
        "no case exercised a positive cross-section"
    );
    assert!(
        totals.empty_cross > 0,
        "no case exercised the empty zero-cross-section branch"
    );
    assert!(
        totals.zero_line_width > 0,
        "no zero-line-width mode was generated"
    );
    assert!(
        totals.zero_tower_width > 0,
        "no zero-tower-width mode was generated"
    );
    assert!(
        totals.zero_layer_height > 0,
        "no zero-layer-height mode was generated"
    );
    assert!(
        totals.zero_volume > 0,
        "no zero-purge-volume mode was generated"
    );
    assert!(totals.scan_rows_positive > 0, "no case produced a scan row");
    assert!(
        totals.rows_zero > 0,
        "no positive-cross-section case produced zero scan rows"
    );
    assert!(totals.rows_one > 0, "no case produced exactly one scan row");
    assert!(
        totals.rows_many > 0,
        "no case produced a deep scan-row stack"
    );
    assert!(
        totals.exact_rows > 0,
        "no case landed in the exact scan-row class"
    );
    assert!(
        totals.classified_rows > 0,
        "no case landed in the classified scan-row class"
    );
    assert!(
        totals.near_integer > 0,
        "no case was near an integer scan-row quotient"
    );
    assert!(
        totals.integer_quotient > 0,
        "no case had an exactly integer purge_depth/line_width quotient"
    );
    assert!(
        totals.boundary_rows > 0,
        "no case hit the strict-`<` scan-row boundary"
    );
    assert!(
        totals.ceiling_volume > 0,
        "no case reached max_purge_volume_mm3"
    );
    assert!(totals.rectangle_bed > 0, "no rectangle bed witness");
    assert!(totals.triangle_bed > 0, "no right-triangle bed witness");
}

fn ceiling_witness_reaches_bound(limits: &WipeLimits) -> bool {
    let widths = ceiling_line_widths(limits);
    widths.iter().any(|line_width| {
        let tower_width = tower_width_cap(limits);
        let cross = cross_section_f32(*line_width, 0.5, tower_width);
        let per_unit = f64::from(*line_width * 0.5) * f64::from(cross);
        if !(per_unit > 0.0) {
            return false;
        }
        let units = (f64::from(limits.max_purge_volume_mm3) / per_unit).floor();
        let volume = (units * per_unit) as f32;
        volume == limits.max_purge_volume_mm3
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic controls and shrink probe
// ─────────────────────────────────────────────────────────────────────────────

/// Deterministic control witness. Module-private test type, so exhaustive
/// literals are the fixture (not a watched type).
#[allow(clippy::too_many_arguments)]
fn control_case(
    line_width: f32,
    layer_height: f32,
    tower_x: f32,
    tower_y: f32,
    tower_width: f32,
    purge_volume: f32,
    bed_shape: BedShape,
    pad_units: i64,
) -> WipeCase {
    let bed = build_bed(
        tower_x,
        tower_width,
        tower_y,
        purge_depth_mm(line_width, layer_height, tower_width, purge_volume),
        bed_shape,
        pad_units,
    );
    WipeCase {
        line_width,
        layer_height,
        tower_x,
        tower_y,
        tower_width,
        purge_volume,
        layer_z: 0.25,
        global_layer_index: 0,
        tool_change: ToolChange {
            to_tool: 1,
            ..ToolChange::default()
        },
        bed,
        scale_mode: ScaleMode::Positive,
        volume_mode: VolumeMode::Grid,
        row_edge: RowEdge::Zero,
        bed_shape,
    }
}

fn run_check(
    case: &WipeCase,
    model: &OracleModel,
    limits: &WipeLimits,
) -> Result<WipeObservation, String> {
    let tower = build_tower(case)?;
    let pairs = generate(case, &tower);
    check_purge_paths(case, &pairs, model, limits)
}

#[test]
fn wipe_tower_controls() {
    let limits = control_limits();
    let honest = OracleModel::default();

    // Witness: 0.5 mm line, 0.25 mm layer, 4 mm tower, 8 mm^3 purge ->
    // cross-section 0.5, depth 16 mm, exactly 32 half-line-width units, so the
    // row count is in the exact class (n_real = 31.5, ceil = 32).
    let positive = control_case(0.5, 0.25, 10.0, 10.0, 4.0, 8.0, BedShape::Rectangle, 1);
    let observation = run_check(&positive, &honest, &limits)
        .expect("the deterministic positive witness is accepted");
    assert!(
        observation.exact_rows,
        "the control witness must be exact-class"
    );
    assert_eq!(
        observation.observed_rows, 32,
        "unexpected witness row count"
    );

    // 1. A predicted scan row outside the rectangle is rejected by the same
    // containment predicate the property uses.
    let error = run_check(
        &positive,
        &OracleModel {
            outside_scan_row: true,
            ..OracleModel::default()
        },
        &limits,
    )
    .expect_err("a scan row outside the tower rectangle must be rejected");
    assert!(error.contains("outside the tower rectangle"), "{error}");

    // 2. An off-by-one scan count is rejected.
    let error = run_check(
        &positive,
        &OracleModel {
            scan_count_off_by_one: true,
            ..OracleModel::default()
        },
        &limits,
    )
    .expect_err("an off-by-one scan count must be rejected");
    assert!(error.contains("scan-row count"), "{error}");

    // 3. A prime segment beyond tower_width is rejected. The witness caps the
    // prime at tower_width (8 / (0.5 * 0.25) = 64 > 4).
    let error = run_check(
        &positive,
        &OracleModel {
            prime_beyond_width: true,
            ..OracleModel::default()
        },
        &limits,
    )
    .expect_err("a prime segment beyond tower_width must be rejected");
    assert!(error.contains("prime end x"), "{error}");
    assert!(observation.purge_positive);

    // 4. A negative/zero extrusion length claimed positive is rejected: the
    // zero-volume witness has one zero-length prime pair.
    let zero_volume = control_case(0.5, 0.25, 10.0, 10.0, 4.0, 0.0, BedShape::Rectangle, 1);
    let zero_observation =
        run_check(&zero_volume, &honest, &limits).expect("the zero-volume witness is accepted");
    assert_eq!(
        zero_observation.observed_rows, 0,
        "the zero-volume witness must have no scan rows"
    );
    let error = run_check(
        &zero_volume,
        &OracleModel {
            require_positive_prime: true,
            ..OracleModel::default()
        },
        &limits,
    )
    .expect_err("a zero-length prime claimed positive must be rejected");
    assert!(error.contains("claimed positive"), "{error}");

    // 5. A concave-bed containment assertion is rejected by the same bed
    // checker: the four tower corners are inside, the tower interior is not.
    // This is a control about the test oracle, never a production claim.
    concave_bed_control(&limits);

    // Deterministic row-count ladder on the exact grid (cross-section 0.5,
    // line_width/2 = 0.25 mm): zero rows, one row, the strict-`<` boundary where
    // the last row lands exactly on `y_max`, and a deep stack.
    for (purge_volume, expected_rows, boundary) in [
        (0.0, 0, false),   // depth 0 -> n_real = -0.5
        (0.125, 0, true),  // units 1 -> n_real = 0.0
        (0.25, 1, false),  // units 2 -> n_real = 0.5
        (0.375, 1, true),  // units 3 -> n_real = 1.0
        (0.5, 2, false),   // units 4 -> n_real = 1.5
        (4.25, 17, false), // units 34 -> n_real = 16.5
        (15.5, 62, false), // units 124 -> n_real = 61.5
    ] {
        let case = control_case(
            0.5,
            0.25,
            10.0,
            10.0,
            4.0,
            purge_volume,
            BedShape::Rectangle,
            1,
        );
        let observation = run_check(&case, &honest, &limits).expect("row-count ladder witness");
        assert!(
            observation.exact_rows,
            "purge_volume {purge_volume} must be exact-class"
        );
        assert_eq!(
            observation.observed_rows, expected_rows,
            "purge_volume {purge_volume} row count"
        );
        assert_eq!(
            observation.boundary_rows, boundary,
            "purge_volume {purge_volume} strict-`<` boundary flag"
        );
        println!(
            "row ladder: purge_volume={purge_volume} rows={} boundary={boundary}",
            observation.observed_rows
        );
    }

    // The documented production layer height (0.2 mm) is off the 1/16 mm grid;
    // it must be classified, never claimed as an exact count. The 3 mm tower
    // width makes `purge_depth / (line_width/2)` non-integral
    // (8 / 0.30000000447034836 / 0.25 = 106.66...).
    let off_grid = control_case(
        0.5,
        PRODUCTION_DEFAULT_LAYER_HEIGHT_MM,
        10.0,
        10.0,
        3.0,
        8.0,
        BedShape::RightTriangle,
        4,
    );
    let off_grid_observation =
        run_check(&off_grid, &honest, &limits).expect("the 0.2 mm witness is accepted");
    assert!(
        !off_grid_observation.exact_rows,
        "the 0.2 mm layer height is off the 1/16 mm grid and must be classified"
    );
    let observed = off_grid_observation.observed_rows as i64;
    let expected = off_grid_observation.expected_rows as i64;
    assert!(
        (observed - expected).abs() <= 1,
        "0.2 mm witness: observed {observed} rows, rational ceil {expected}"
    );

    // Deviation 2 determinism pin: the generator never reads `bed_shape`, so two
    // constructions that differ only in the bed produce identical pairs.
    // test-quality: determinism pin — same generator, two bed configs, no
    // correctness claim about bed validation (that lives in run_finalization).
    let other_bed = control_case(0.5, 0.25, 10.0, 10.0, 4.0, 8.0, BedShape::RightTriangle, 64);
    let first = generate(&positive, &build_tower(&positive).expect("tower"));
    let second = generate(&other_bed, &build_tower(&other_bed).expect("tower"));
    assert_eq!(
        first, second,
        "generate_purge_paths must not depend on bed_shape (deviation 2)"
    );

    // The seam's `_tc` argument is unused by the generator: two different
    // `ToolChange` values must produce identical pairs.
    // test-quality: determinism pin — the seam ignores its tool-change argument
    // on the observed domain.
    let mut other_tool_change = positive.clone();
    other_tool_change.tool_change = ToolChange {
        after_entity_index: 7,
        from_tool: 2,
        to_tool: 3,
    };
    let with_other_change = generate(
        &other_tool_change,
        &build_tower(&other_tool_change).expect("tower"),
    );
    assert_eq!(
        first, with_other_change,
        "generate_purge_paths must not depend on the tool-change argument"
    );

    wipe_witness_probe();
}

fn concave_bed_control(limits: &WipeLimits) {
    // Concave polygon with a notch cut into its top edge between x = 4 and 6,
    // down to y = 4. Every tower corner below is inside; (5, 5) is in the notch.
    let bed = BedPolygon {
        vertices: vec![
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (6.0, 10.0),
            (6.0, 4.0),
            (4.0, 4.0),
            (4.0, 10.0),
            (0.0, 10.0),
        ],
        convex_ccw: false,
    };
    let corners = [(3.0f32, 3.0f32), (7.0, 3.0), (7.0, 5.0), (3.0, 5.0)];
    let mut points = corners.to_vec();
    points.push((5.0, 5.0));
    let envelope = envelope_mm(limits);

    // Corner-only validation accepts the witness...
    check_bed_containment(&corners, &bed, true, envelope)
        .expect("corner-only validation accepts the four tower corners");
    // ...and so does the corner-only policy over the full point set, because the
    // bounding-box corners are the tower corners.
    check_bed_containment(&points, &bed, true, envelope)
        .expect("corner-only validation accepts the concave bed");
    // The honest checker rejects the interior point, for the corners and for
    // the full set. The mutation is therefore caught by the same checker.
    let error = check_bed_containment(&points, &bed, false, envelope)
        .expect_err("a concave bed with a tower-interior point outside must be rejected");
    assert!(
        error.contains("does not contain generated point"),
        "{error}"
    );
    assert!(
        !bed.contains(5.0, 5.0, 0.0),
        "the concave witness must exclude its notch point"
    );
    assert!(
        corners.iter().all(|(x, y)| bed.contains(*x, *y, 0.0)),
        "the concave witness must contain all four tower corners"
    );
}

fn wipe_witness_probe() {
    let limits = control_limits();

    for scale_mode in [ScaleMode::Positive, ScaleMode::ZeroVolume] {
        for bed_shape in BED_SHAPES {
            let volume_mode = VolumeMode::Grid;
            let strategy = any::<[u32; 6]>().prop_map(move |seed| {
                build_case(
                    scale_mode,
                    volume_mode,
                    RowEdge::One,
                    bed_shape,
                    seed,
                    &limits,
                )
            });
            let mut runner = TestRunner::deterministic();
            let mut tree = strategy.new_tree(&mut runner).expect("wipe witness tree");

            check_witness(&tree.current(), scale_mode, bed_shape, &limits);
            let initial = format!("{:?}", tree.current());
            let mut simplified = 0usize;
            let mut changed = false;
            while tree.simplify() {
                simplified += 1;
                let current = tree.current();
                check_witness(&current, scale_mode, bed_shape, &limits);
                changed |= format!("{current:?}") != initial;
                assert!(
                    tree.complicate(),
                    "complicate must restore the last simplification"
                );
                check_witness(&tree.current(), scale_mode, bed_shape, &limits);
            }
            assert!(
                simplified > 0,
                "false-only shrink probe for {scale_mode:?}/{bed_shape:?}"
            );
            assert!(
                changed,
                "no allowed reduction changed the {scale_mode:?}/{bed_shape:?} witness"
            );
            println!("{scale_mode:?}/{bed_shape:?} simplify={simplified} changed={changed}");
        }
    }
}

fn check_witness(case: &WipeCase, scale_mode: ScaleMode, bed_shape: BedShape, limits: &WipeLimits) {
    assert_eq!(case.scale_mode, scale_mode, "the scale mode changed");
    assert_eq!(case.bed_shape, bed_shape, "the bed shape changed");
    assert_eq!(
        case.volume_mode,
        VolumeMode::Grid,
        "the volume mode changed"
    );
    assert_eq!(case.row_edge, RowEdge::One, "the row edge changed");
    let observation =
        run_check(case, &OracleModel::default(), limits).expect("the witness case checks");
    if scale_mode == ScaleMode::ZeroVolume {
        assert_eq!(
            observation.observed_rows, 0,
            "the zero-volume mode gained rows"
        );
        assert!(
            !observation.purge_positive,
            "the zero-volume mode gained volume"
        );
    }
}
