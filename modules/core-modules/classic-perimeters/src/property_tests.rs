//! Opt-in property tests for this module's private classic perimeter kernel.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_12_pbt-walls`):
//!
//! * `M-CLASSIC-WALL-STRUCTURE` — `ClassicPerimeters::emit_walls` role,
//!   closure and metadata correspondence.
//! * `M-CLASSIC-BAND-SELECTION` — emit gates, the allowed-prefix band
//!   invariant, capacity and the documented degenerate/fatal paths.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! `emit_walls` is private, so the properties live here as a crate-internal
//! unit-test module. Relocating them under `tests/**` would make them
//! integration tests, which cannot see the private kernel; production
//! visibility is deliberately not widened for tests.
//!
//! # Step status
//!
//! Step 3 implements the `M-CLASSIC-WALL-STRUCTURE` oracle, its generated
//! property, the required deterministic witnesses, the oracle-mutation control
//! and the shrink protocol probe. Step 4 implements `M-CLASSIC-BAND-SELECTION`:
//! `classic_band_oracle` (which runs the Step 3 structure oracle first), its
//! generated property, the three-mode real-output gate mutations, the 1 mm
//! capacity witness, the 2-point/collinear/empty degenerates, the fatal
//! negative-spacing control and the owner-local bound/domain rejection matrix.
//! No registry record is added: `test-properties.toml` gains either charter only
//! after Step 7 measures its profile, so the `proptest_`-named bodies below
//! require the foundation runner's `PNP_PROPERTY_CONTEXT` and fail bare with
//! `missing_property_context` by design. The six deterministic controls do not
//! need that context.
//!
//! # Domain scale
//!
//! The shared `geometry_cases` catalog bounds a generated component to a small
//! integer size (its width/height room is capped at 32 canonical units above
//! the class minimum), so the generated wall width/layer height are drawn from
//! the registry's `max_width_units` / `max_layer_height_units` at the same
//! scale, with a checked margin above the `line_width_to_spacing` threshold.
//! The deterministic 20 mm witness is a fixed literal, not a generated case.
//!
//! # Literal review (AC-6)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. This file's watched-type sites,
//! re-derived from `crates/*/src/**` `pub` structs with >= 5 named fields:
//!
//! * `WallFeatureFlags` (`crates/slicer-ir/src/slice_ir.rs`) — one literal in
//!   `classic_structure_oracle`, FRU over `Default::default()` with only
//!   `fuzzy_skin` pinned.
//! * `GeometryBounds` (`crates/slicer-sdk/src/test_support/property_testing/
//!   domains/mod.rs`) — two literals (`checked_limits`, `square_bounds`), both
//!   carrying `// exhaustive:` because all five measured bounds are supplied
//!   from the profile and defaulting any of them is exactly what the charter
//!   forbids.
//! * `GeometryDomain` (same file) — one literal in `ClassTag::domain`,
//!   `// exhaustive:` because the class, both minima, the boundary rule and the
//!   bounds are all derived above the literal.
//!
//! `Point3WithWidth`, `WallLoop` and `ExtrusionPath3D` appear only as types
//! (the oracles read them, the mutation controls clone real SUT output); no
//! literal of those three is written here. `ClassicOutput` stores
//! `Vec<Vec<ExPolygon>>` and `Vec<(Point3, f32)>` from the real builder, which
//! adds no literal. `Polygon` (1 field), `ExPolygon` (2), `Point3` (3),
//! `WidthProfile` (1) and `GeometryCase` (1) are below the watch threshold;
//! `ClassicCase`, `ClassicLimits`, `ClassicOutput` and `FixedModes` are private
//! test fixtures, not watched.

#![allow(missing_docs)]

use std::cell::Cell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

use proptest::prelude::*;
use proptest::strategy::{BoxedStrategy, ValueTree};
use proptest::test_runner::{TestCaseError, TestCaseResult, TestRunner};

use slicer_core::flow::line_width_to_spacing;
use slicer_core::perimeter_spatial::{PathAnnotationMode, PerimeterSpatialContext};
use slicer_core::perimeter_utils::{WallSequence, BASE_SPEED};
use slicer_ir::{
    mm_to_units, units_to_mm, ConfigView, ExPolygon, ExtrusionRole, LoopType, PaintSemantic,
    PaintValue, Point2, Point3, Point3WithWidth, Polygon, WallFeatureFlags, WallLoop,
};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::error::ModuleError;
use slicer_sdk::test_support::fixtures::ConfigViewBuilder;
use slicer_sdk::test_support::property_testing::domains::{
    geometry_cases, BoundaryRule, DomainError, GeometryBounds, GeometryCase, GeometryDomain,
    GeometryStrategy, TopologyClass,
};
use slicer_sdk::test_support::property_testing::oracles::validate_geometry;
use slicer_sdk::test_support::property_testing::PropertyRun;
use slicer_sdk::traits::LayerModule;

use super::ClassicPerimeters;

// ─────────────────────────────────────────────────────────────────────────────
// Shared plumbing.
// ─────────────────────────────────────────────────────────────────────────────

const STRUCTURE_PROPERTY_ID: &str = "M-CLASSIC-WALL-STRUCTURE";
const STRUCTURE_TEST_NAME: &str = "property_tests::proptest_classic_wall_structure";

/// Every bound this family reads from the selected profile. A missing key must
/// fail before sampling, never fall back to a helper default.
const CLASSIC_BOUND_KEYS: [&str; 8] = [
    "max_abs_units",
    "max_vertices",
    "max_holes",
    "max_components",
    "max_shrink_candidates",
    "max_wall_count",
    "max_width_units",
    "max_layer_height_units",
];

/// Shared `geometry_cases` cap on `max_abs_units` (`docs/10_*`, #2 export).
const SHARED_MAX_ABS_UNITS: i64 = 1_000_000;

/// Smallest coordinate bound that still holds the required `Holed` and
/// `Disconnected` templates with x/y placement room.
const MIN_ABS_UNITS: i64 = 16;
/// `Holed` needs 4 contour + 4 hole vertices; `Disconnected` needs 2 x 4.
const MIN_VERTICES: usize = 8;
/// The `Holed` class requires at least one hole.
const MIN_HOLES: usize = 1;
/// The `Disconnected` class requires at least two components.
const MIN_COMPONENTS: usize = 2;
/// A zero candidate budget would make every shrink probe vacuous.
const MIN_SHRINK_CANDIDATES: u32 = 1;
/// A zero wall count emits nothing and is outside the charter.
const MIN_WALL_COUNT: u32 = 1;

/// `from_config` keys this test supplies. The module's `require_*` reads are
/// fail-closed, so all five declared keys must be present.
const OUTER_WALL_SPEED_MM_S: f32 = 25.0;
const INNER_WALL_SPEED_MM_S: f32 = 50.0;
const PERIMETER_ARC_TOLERANCE_MM: f32 = 0.0125;
const RESOLUTION_MM: f32 = 0.01;

/// Independent re-derivation of the documented speed-factor rule
/// (`BASE_SPEED` is `slicer_core::perimeter_utils::BASE_SPEED`; a configured
/// `0.0` speed is the AUTO sentinel and means neutral 1.0). The oracle compares
/// `emit_walls` output against these, so a swapped or unscaled mapping in
/// `from_config` is visible.
const EXPECTED_OUTER_SPEED_FACTOR: f32 = OUTER_WALL_SPEED_MM_S / BASE_SPEED;
const EXPECTED_INNER_SPEED_FACTOR: f32 = INNER_WALL_SPEED_MM_S / BASE_SPEED;

/// Deterministic 20 mm witness, fixed at the canonical-unit scale.
const WITNESS_SIDE_MM: f32 = 20.0;
const WITNESS_WALL_COUNT: u32 = 3;
const WITNESS_WALL_WIDTH_MM: f32 = 0.4;
const WITNESS_LAYER_HEIGHT_MM: f32 = 0.2;
const WITNESS_Z_MM: f32 = 0.2;

/// Scalar values `emit_walls` accepts but the structure oracle does not vary.
const BRIDGE_FLOW_RATIO: f32 = 1.0;
const INFILL_WALL_OVERLAP_MM: f32 = 0.15;
const NOZZLE_DIAMETER_MM: f32 = 0.4;
const GAP_INFILL_SPEED_MM_S: f32 = 30.0;
const FILTER_OUT_GAP_FILL_MM: f32 = 0.5;
const SEAM_CANDIDATE_ANGLE_DEG: f32 = 30.0;
const REGION_ID: u64 = 0;

/// Ordinary, shrinkable z range for generated cases. Distinct from the
/// generated layer heights so a dropped z is visible; the oracle compares the
/// emitted z against the case value bit for bit.
const Z_MIN_MM: f32 = 0.05;
const Z_MAX_MM: f32 = 0.35;

/// Shrink-protocol step budget per direction. Bounded so a pathological tree
/// cannot spin; the probe fails if nothing changed inside the budget.
const SHRINK_STEPS: usize = 128;

// ─────────────────────────────────────────────────────────────────────────────
// M-CLASSIC-BAND-SELECTION fixtures.
// ─────────────────────────────────────────────────────────────────────────────

/// Band-selection charter identity. No registry record exists yet: Step 7
/// measures the profile, and the pilot below feeds this exact ID through a
/// hand-built context.
const BAND_PROPERTY_ID: &str = "M-CLASSIC-BAND-SELECTION";
const BAND_TEST_NAME: &str = "property_tests::proptest_classic_band_selection";

/// Capacity witness: a 1 mm square asked for ten 0.4 mm bands. Source analysis
/// predicts a single band, but the control asserts the measured range
/// `1..10` and records the actual count instead of pinning a universal rule.
const CAPACITY_SIDE_MM: f32 = 1.0;
const CAPACITY_WALL_COUNT: u32 = 10;

/// A width below `layer_height * (1 - PI/4)` for the witness layer height, i.e.
/// inside the documented fatal `ERR_NEGATIVE_SPACING` path.
const NEGATIVE_SPACING_WIDTH_MM: f32 = 0.02;

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

fn bits_eq(left: f32, right: f32) -> bool {
    left.to_bits() == right.to_bits()
}

// ─────────────────────────────────────────────────────────────────────────────
// Fixtures.
// ─────────────────────────────────────────────────────────────────────────────

/// Requested topology family of a generated classic case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClassTag {
    Convex,
    Concave,
    Holed,
    Disconnected,
}

/// All four class tags in the order the property declares them.
const CLASS_TAGS: [ClassTag; 4] = [
    ClassTag::Convex,
    ClassTag::Concave,
    ClassTag::Holed,
    ClassTag::Disconnected,
];

impl ClassTag {
    fn index(self) -> usize {
        match self {
            Self::Convex => 0,
            Self::Concave => 1,
            Self::Holed => 2,
            Self::Disconnected => 3,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Convex => "convex",
            Self::Concave => "concave",
            Self::Holed => "holed",
            Self::Disconnected => "disconnected",
        }
    }

    /// Requested #2 domain for this class.
    ///
    /// `Convex`/`Concave` require one component and no hole; `Holed` requires a
    /// hole; `Disconnected` requires two components. `BoundaryRule::Strict` and
    /// the registry bounds are supplied by the caller.
    fn domain(self, bounds: &GeometryBounds) -> GeometryDomain {
        let (class, min_holes, min_components) = match self {
            Self::Convex => (TopologyClass::Convex, 0, 1),
            Self::Concave => (TopologyClass::Concave, 0, 1),
            Self::Holed => (TopologyClass::Holed, 1, 1),
            Self::Disconnected => (TopologyClass::Disconnected, 0, 2),
        };
        // exhaustive: class, both topology minima, the boundary rule and the
        // bounds are all derived immediately above; defaulting any of them
        // would silently weaken the declared topology.
        GeometryDomain {
            class,
            min_holes,
            min_components,
            boundary: BoundaryRule::Strict,
            bounds: bounds.clone(),
        }
    }
}

/// Which of the two ordinary wall bands the case asks `emit_walls` to emit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EmitMode {
    Both,
    OuterOnly,
    InnerOnly,
}

/// Every emit mode, in counter order. The strategy and the coverage assertions
/// share this one list.
const EMIT_MODES: [EmitMode; 3] = [EmitMode::Both, EmitMode::OuterOnly, EmitMode::InnerOnly];

impl EmitMode {
    /// `(emit_outer, emit_inner)` for the real call.
    fn gates(self) -> (bool, bool) {
        match self {
            Self::Both => (true, true),
            Self::OuterOnly => (true, false),
            Self::InnerOnly => (false, true),
        }
    }

    /// Position in `EMIT_MODES`, used by the per-mode coverage counters.
    fn index(self) -> usize {
        match self {
            Self::Both => 0,
            Self::OuterOnly => 1,
            Self::InnerOnly => 2,
        }
    }

    /// Stable label for failure messages and the pilot log.
    fn name(self) -> &'static str {
        match self {
            Self::Both => "both",
            Self::OuterOnly => "outer-only",
            Self::InnerOnly => "inner-only",
        }
    }
}

/// `true` when the band's gate is enabled: band 0 is the outer wall, every band
/// above 0 is an inner wall.
fn band_allowed(emit: EmitMode, band: u32) -> bool {
    let (emit_outer, emit_inner) = emit.gates();
    if band == 0 {
        emit_outer
    } else {
        emit_inner
    }
}

/// One generated classic case: bounded geometry, the requested bands and the
/// ordinary scalar inputs of a direct private `emit_walls` call.
#[derive(Debug, Clone)]
struct ClassicCase {
    class: ClassTag,
    domain: GeometryDomain,
    geometry: GeometryCase,
    wall_count: u32,
    outer_width_mm: f32,
    inner_width_mm: f32,
    layer_height_mm: f32,
    z_mm: f32,
    emit: EmitMode,
    variant_fuzzy: bool,
    outer_speed_factor: f32,
    inner_speed_factor: f32,
}

/// The walls, infill areas and seam candidates the real private kernel returned
/// for one case. The side outputs are captured from the same fresh builder so
/// the band oracle's gate implications inspect real output, not a projection.
#[derive(Debug, Clone, PartialEq)]
struct ClassicOutput {
    walls: Vec<WallLoop>,
    infill_areas: Vec<Vec<ExPolygon>>,
    seam_candidates: Vec<(Point3, f32)>,
}

impl ClassicOutput {
    fn is_empty(&self) -> bool {
        self.walls.is_empty()
    }
}

/// The no-shrink part of a case: the selected class/topology and the two mode
/// selectors. `classic_shrink_preserves_emit_mode_and_topology` asserts these
/// survive every `simplify`/`complicate` step.
#[derive(Debug, Clone, PartialEq)]
struct FixedModes {
    class: ClassTag,
    domain: GeometryDomain,
    emit: EmitMode,
    variant_fuzzy: bool,
}

impl FixedModes {
    fn of(case: &ClassicCase) -> Self {
        Self {
            class: case.class,
            domain: case.domain.clone(),
            emit: case.emit,
            variant_fuzzy: case.variant_fuzzy,
        }
    }
}

/// The ordinary scalar inputs of a case. Bitwise comparison keeps `-0.0` and
/// two equal-looking widths distinguishable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OrdinaryValues {
    wall_count: u32,
    outer_width_bits: u32,
    inner_width_bits: u32,
    layer_height_bits: u32,
    z_bits: u32,
}

impl OrdinaryValues {
    fn of(case: &ClassicCase) -> Self {
        Self {
            wall_count: case.wall_count,
            outer_width_bits: case.outer_width_mm.to_bits(),
            inner_width_bits: case.inner_width_mm.to_bits(),
            layer_height_bits: case.layer_height_mm.to_bits(),
            z_bits: case.z_mm.to_bits(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Checked bounds and the four prebuilt class strategies.
// ─────────────────────────────────────────────────────────────────────────────

/// Validated, converted classic property limits plus the four prebuilt #2
/// geometry strategies. Construction never narrows a bound silently.
#[derive(Debug, Clone)]
struct ClassicLimits {
    bounds: GeometryBounds,
    max_wall_count: u32,
    width_units: std::ops::RangeInclusive<i64>,
    layer_height_units: std::ops::RangeInclusive<i64>,
    geometry: Vec<(ClassTag, GeometryStrategy)>,
}

impl ClassicLimits {
    fn domain(&self, tag: ClassTag) -> GeometryDomain {
        tag.domain(&self.bounds)
    }

    /// The prebuilt strategy for one class. `checked_limits` builds all four,
    /// so the generation closure cannot fail or fall back.
    fn class_strategy(&self, tag: ClassTag) -> GeometryStrategy {
        self.geometry
            .iter()
            .find(|(candidate, _)| *candidate == tag)
            .map(|(_, strategy)| strategy.clone())
            .expect("all four class strategies are prebuilt by checked_limits")
    }
}

fn required(bounds: &BTreeMap<String, u64>, key: &str) -> Result<u64, DomainError> {
    bounds
        .get(key)
        .copied()
        .ok_or_else(|| DomainError::InvalidBounds(format!("missing bound key `{key}`")))
}

fn checked_i64(value: u64) -> Result<i64, DomainError> {
    i64::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)
}

fn checked_u32(value: u64) -> Result<u32, DomainError> {
    u32::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)
}

fn checked_usize(value: u64) -> Result<usize, DomainError> {
    usize::try_from(value).map_err(|_| DomainError::ArithmeticOverflow)
}

/// Smallest width (canonical units) that clears `line_width_to_spacing` for
/// every admitted layer height, plus a one-unit safety margin.
///
/// The production rule is `width - layer_height * (1 - PI/4) > 0` in f32
/// (`line_width_to_spacing`). This bound is derived in f64 from
/// `1 - PI/4` and rounded up, then lifted by one whole canonical unit
/// (0.0001 mm); the unit margin dominates the f32/f64 difference at these
/// magnitudes, so every generated pair has strictly positive spacing.
fn width_floor(max_layer_height_units: i64) -> i64 {
    let rounded_cross_section = 1.0 - std::f64::consts::FRAC_PI_4;
    (max_layer_height_units as f64 * rounded_cross_section).ceil() as i64 + 1
}

/// Validate, convert and prebuild every bound this family needs.
///
/// Rejects with `DomainError::ArithmeticOverflow` when a key cannot narrow to
/// its Rust type and with `DomainError::InvalidBounds` when a key is missing or
/// the combination is unusable, before any strategy allocates.
fn checked_limits(bounds: &BTreeMap<String, u64>) -> Result<ClassicLimits, DomainError> {
    // Checked narrowing first: an unrepresentable key is an arithmetic
    // overflow, never a silently clamped bound.
    let max_abs_units = checked_i64(required(bounds, "max_abs_units")?)?;
    let max_vertices = checked_usize(required(bounds, "max_vertices")?)?;
    let max_holes = checked_usize(required(bounds, "max_holes")?)?;
    let max_components = checked_usize(required(bounds, "max_components")?)?;
    let max_shrink_candidates = checked_u32(required(bounds, "max_shrink_candidates")?)?;
    let max_wall_count = checked_u32(required(bounds, "max_wall_count")?)?;
    let max_width_units = checked_i64(required(bounds, "max_width_units")?)?;
    let max_layer_height_units = checked_i64(required(bounds, "max_layer_height_units")?)?;

    if !(1..=SHARED_MAX_ABS_UNITS).contains(&max_abs_units) {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units {max_abs_units} must be in 1..={SHARED_MAX_ABS_UNITS}"
        )));
    }
    if max_abs_units < MIN_ABS_UNITS {
        return Err(DomainError::InvalidBounds(format!(
            "max_abs_units {max_abs_units} cannot hold the required Holed/Disconnected \
             templates (needs at least {MIN_ABS_UNITS})"
        )));
    }
    if max_vertices < MIN_VERTICES {
        return Err(DomainError::InvalidBounds(format!(
            "max_vertices {max_vertices} cannot hold a hole or a second component \
             (needs at least {MIN_VERTICES})"
        )));
    }
    if max_holes < MIN_HOLES {
        return Err(DomainError::InvalidBounds(format!(
            "max_holes {max_holes} cannot satisfy the Holed minimum {MIN_HOLES}"
        )));
    }
    if max_components < MIN_COMPONENTS {
        return Err(DomainError::InvalidBounds(format!(
            "max_components {max_components} cannot satisfy the Disconnected minimum \
             {MIN_COMPONENTS}"
        )));
    }
    if max_shrink_candidates < MIN_SHRINK_CANDIDATES {
        return Err(DomainError::InvalidBounds(format!(
            "max_shrink_candidates {max_shrink_candidates} must be at least \
             {MIN_SHRINK_CANDIDATES} so the shrink probe is not vacuous"
        )));
    }
    if max_wall_count < MIN_WALL_COUNT {
        return Err(DomainError::InvalidBounds(format!(
            "max_wall_count {max_wall_count} must be at least {MIN_WALL_COUNT}"
        )));
    }
    if max_width_units < 1 || max_layer_height_units < 1 {
        return Err(DomainError::InvalidBounds(format!(
            "max_width_units {max_width_units} and max_layer_height_units \
             {max_layer_height_units} must both be positive"
        )));
    }
    let floor = width_floor(max_layer_height_units);
    if max_width_units < floor {
        return Err(DomainError::InvalidBounds(format!(
            "max_width_units {max_width_units} leaves no line_width_to_spacing margin for \
             max_layer_height_units {max_layer_height_units} (needs at least {floor})"
        )));
    }

    // exhaustive: live profiles must supply every measured bound, never the
    // helper default.
    let geometry_bounds = GeometryBounds {
        max_abs_units,
        max_vertices,
        max_holes,
        max_components,
        max_shrink_candidates,
    };

    let mut geometry = Vec::with_capacity(CLASS_TAGS.len());
    for tag in CLASS_TAGS {
        let strategy = geometry_cases(tag.domain(&geometry_bounds))?;
        geometry.push((tag, strategy));
    }

    Ok(ClassicLimits {
        bounds: geometry_bounds,
        max_wall_count,
        width_units: floor..=max_width_units,
        layer_height_units: 1..=max_layer_height_units,
        geometry,
    })
}

/// Read the eight live bounds through `PropertyRun::bound`.
///
/// A missing key panics with the foundation's actionable `PropertyRunError`;
/// the registry profile is required to declare every key this family reads.
fn live_limits(run: &PropertyRun) -> ClassicLimits {
    let mut bounds = BTreeMap::new();
    for key in CLASSIC_BOUND_KEYS {
        let value = run
            .bound(key)
            .unwrap_or_else(|error| panic!("M-CLASSIC-WALL-STRUCTURE bound `{key}`: {error}"));
        bounds.insert(key.to_owned(), value);
    }
    checked_limits(&bounds).expect("M-CLASSIC-WALL-STRUCTURE bound consistency")
}

/// Explicit bound set for the deterministic controls and the shrink probe. It
/// does not read a property context, so those controls run bare; every value is
/// a live-style bound (not a helper default).
fn probe_bound_map() -> BTreeMap<String, u64> {
    let mut bounds = BTreeMap::new();
    bounds.insert("max_abs_units".to_owned(), 1_024);
    bounds.insert("max_vertices".to_owned(), 16);
    bounds.insert("max_holes".to_owned(), 2);
    bounds.insert("max_components".to_owned(), 3);
    bounds.insert("max_shrink_candidates".to_owned(), 32);
    bounds.insert("max_wall_count".to_owned(), 4);
    bounds.insert("max_width_units".to_owned(), 8);
    bounds.insert("max_layer_height_units".to_owned(), 4);
    bounds
}

fn shrink_probe_limits() -> ClassicLimits {
    checked_limits(&probe_bound_map()).expect("the shrink probe bound set is valid")
}

// ─────────────────────────────────────────────────────────────────────────────
// Strategies.
// ─────────────────────────────────────────────────────────────────────────────

/// One `prop::sample::select(...).no_shrink()` emit-gate selector.
///
/// A `prop_oneof!` of `Just` arms is forbidden here: a union may simplify into
/// an earlier arm, which would flip the gate during shrinking.
fn emit_mode_strategy() -> BoxedStrategy<EmitMode> {
    prop::sample::select(EMIT_MODES.to_vec())
        .no_shrink()
        .boxed()
}

/// One no-shrink `variant_fuzzy` selector.
fn variant_fuzzy_strategy() -> BoxedStrategy<bool> {
    prop::sample::select(vec![false, true]).no_shrink().boxed()
}

/// Build the classic case strategy over `classes`.
///
/// Only the no-shrink `ClassTag` selector lives inside `prop_ind_flat_map`, so
/// the derived per-class `geometry_cases` strategy is fixed for a tree and the
/// topology cannot switch while shrinking. Wall count, widths, layer height and
/// z stay in an ordinary tuple outside the selector and therefore shrink; the
/// emit and fuzzy modes are no-shrink selectors and cannot flip.
///
/// `classes` is a parameter only so `classic_shrink_preserves_emit_mode_and_
/// topology` can exercise each tag deterministically through this exact
/// construction path; the live property passes all four.
fn classic_strategy(limits: &ClassicLimits, classes: &[ClassTag]) -> BoxedStrategy<ClassicCase> {
    let selected: Vec<(ClassTag, GeometryStrategy)> = CLASS_TAGS
        .iter()
        .filter(|tag| classes.contains(tag))
        .map(|tag| (*tag, limits.class_strategy(*tag)))
        .collect();
    assert!(
        !selected.is_empty(),
        "classic_strategy needs at least one class tag"
    );

    let ordinary = (
        (1..=limits.max_wall_count).boxed(),
        limits.width_units.clone().boxed(),
        limits.width_units.clone().boxed(),
        limits.layer_height_units.clone().boxed(),
        (Z_MIN_MM..Z_MAX_MM).boxed(),
        emit_mode_strategy(),
        variant_fuzzy_strategy(),
    );
    let domain_source = limits.clone();

    prop::sample::select(selected)
        .no_shrink()
        .prop_ind_flat_map(move |(class, geometry_strategy)| {
            let domain = domain_source.domain(class);
            let scalars = ordinary.clone();
            (geometry_strategy, scalars).prop_map(
                move |(
                    geometry,
                    (wall_count, outer_units, inner_units, layer_units, z_mm, emit, variant_fuzzy),
                )| {
                    ClassicCase {
                        class,
                        domain: domain.clone(),
                        geometry,
                        wall_count,
                        outer_width_mm: units_to_mm(outer_units),
                        inner_width_mm: units_to_mm(inner_units),
                        layer_height_mm: units_to_mm(layer_units),
                        z_mm,
                        emit,
                        variant_fuzzy,
                        outer_speed_factor: EXPECTED_OUTER_SPEED_FACTOR,
                        inner_speed_factor: EXPECTED_INNER_SPEED_FACTOR,
                    }
                },
            )
        })
        .boxed()
}

// ─────────────────────────────────────────────────────────────────────────────
// The real private kernel.
// ─────────────────────────────────────────────────────────────────────────────

/// Build the module from the five declared configuration keys.
fn classic_module() -> ClassicPerimeters {
    let config: ConfigView = ConfigViewBuilder::new()
        .int("wall_loops", i64::from(WITNESS_WALL_COUNT))
        .float("outer_wall_speed", f64::from(OUTER_WALL_SPEED_MM_S))
        .float("inner_wall_speed", f64::from(INNER_WALL_SPEED_MM_S))
        .float(
            "perimeter_arc_tolerance",
            f64::from(PERIMETER_ARC_TOLERANCE_MM),
        )
        .float("resolution", f64::from(RESOLUTION_MM))
        .build();
    ClassicPerimeters::from_config(&config)
        .expect("the five declared keys configure classic-perimeters")
}

/// Call the real private `emit_walls` for one case.
///
/// The domain is deliberately the documented no-paint fixture: no boundary,
/// overhang or bridge inputs, no segment annotations and
/// `PathAnnotationMode::Unconsumed`, so only ordinary `Outer`/`Inner` loops with
/// default annotations can be emitted. A fresh `PerimeterOutputBuilder` is
/// created per invocation, never shared across cases or shrink steps.
fn run_classic(
    module: &ClassicPerimeters,
    case: &ClassicCase,
) -> Result<ClassicOutput, ModuleError> {
    let mut output = PerimeterOutputBuilder::new();
    let context = PerimeterSpatialContext::new(&[], 0.0, &[]);
    let annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> = HashMap::new();
    let (emit_outer, emit_inner) = case.emit.gates();
    module.emit_walls(
        &case.geometry.polygons,
        // Original, un-simplified region polygons: the same bounded geometry, so
        // annotation attribution has a source even though no annotation exists.
        &case.geometry.polygons,
        case.z_mm,
        &annotations,
        case.variant_fuzzy,
        emit_outer,
        emit_inner,
        &mut output,
        case.wall_count,
        module.outer_speed_factor,
        module.inner_speed_factor,
        BRIDGE_FLOW_RATIO,
        false,
        case.outer_width_mm,
        case.inner_width_mm,
        INFILL_WALL_OVERLAP_MM,
        WallSequence::InnerOuter,
        false,
        false,
        NOZZLE_DIAMETER_MM,
        case.layer_height_mm,
        GAP_INFILL_SPEED_MM_S,
        FILTER_OUT_GAP_FILL_MM,
        REGION_ID,
        false,
        SEAM_CANDIDATE_ANGLE_DEG,
        &context,
        PathAnnotationMode::Unconsumed,
    )?;
    Ok(ClassicOutput {
        walls: output.wall_loops().to_vec(),
        infill_areas: output.infill_areas().to_vec(),
        seam_candidates: output.seam_candidates().to_vec(),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// M-CLASSIC-WALL-STRUCTURE — the independent structure oracle.
// ─────────────────────────────────────────────────────────────────────────────

/// Bit-identical point comparison, including optional annotation fields.
fn points_bit_identical(left: &Point3WithWidth, right: &Point3WithWidth) -> bool {
    bits_eq(left.x, right.x)
        && bits_eq(left.y, right.y)
        && bits_eq(left.z, right.z)
        && bits_eq(left.width, right.width)
        && bits_eq(left.flow_factor, right.flow_factor)
        && left.overhang_quartile == right.overhang_quartile
        && bits_eq(left.dist_to_top_mm, right.dist_to_top_mm)
        && left.overhang_distance_mm.map(f32::to_bits)
            == right.overhang_distance_mm.map(f32::to_bits)
}

fn require_bits(
    field: &str,
    index: usize,
    point: usize,
    actual: f32,
    expected: f32,
) -> Result<(), String> {
    if bits_eq(actual, expected) {
        Ok(())
    } else {
        Err(format!(
            "wall {index} point {point}: {field} {actual:?} ({:#010x}) is not the expected \
             {expected:?} ({:#010x})",
            actual.to_bits(),
            expected.to_bits()
        ))
    }
}

/// Independent structure contract for ordinary classic walls.
///
/// Derived from the documented `emit_walls` contract, not from a production
/// validator: band 0 is the outer wall and the module's own `line_width_for`
/// rule gives it the outer width and speed factor, every band above 0 is inner;
/// `expolygon_to_path3d_indexed` emits one point per ring vertex plus an
/// explicit closing repeat, and the builder mirrors that repeat's flag; the
/// no-paint/`Unconsumed` fixture pins the annotation defaults. Empty output is
/// legitimate and returns `Ok`: the count of emitted walls is geometry
/// dependent and is never compared with `wall_count`.
fn classic_structure_oracle(case: &ClassicCase, output: &ClassicOutput) -> Result<(), String> {
    // exhaustive-free FRU: only `fuzzy_skin` varies in this fixture; every other
    // flag field keeps its documented default.
    let expected_flag = WallFeatureFlags {
        fuzzy_skin: case.variant_fuzzy,
        ..Default::default()
    };

    for (index, wall) in output.walls.iter().enumerate() {
        let band = wall.perimeter_index;
        if band >= case.wall_count {
            return Err(format!(
                "wall {index}: perimeter_index {band} is outside the requested 0..{}",
                case.wall_count
            ));
        }
        let is_outer = band == 0;
        let expected_loop = if is_outer {
            LoopType::Outer
        } else {
            LoopType::Inner
        };
        let expected_role = if is_outer {
            ExtrusionRole::OuterWall
        } else {
            ExtrusionRole::InnerWall
        };
        let expected_speed = if is_outer {
            case.outer_speed_factor
        } else {
            case.inner_speed_factor
        };
        let expected_width = if is_outer {
            case.outer_width_mm
        } else {
            case.inner_width_mm
        };

        if wall.loop_type != expected_loop {
            return Err(format!(
                "wall {index} (band {band}): loop_type {:?} is not {expected_loop:?}",
                wall.loop_type
            ));
        }
        if wall.path.role != expected_role {
            return Err(format!(
                "wall {index} (band {band}): role {:?} is not {expected_role:?}",
                wall.path.role
            ));
        }
        if !bits_eq(wall.path.speed_factor, expected_speed) {
            return Err(format!(
                "wall {index} (band {band}): speed_factor {:?} is not the band's {expected_speed:?}",
                wall.path.speed_factor
            ));
        }
        if wall.path.tool_index.is_some() {
            return Err(format!(
                "wall {index} (band {band}): tool_index {:?} must be None in the no-paint fixture",
                wall.path.tool_index
            ));
        }
        if wall.path.order_lock.is_some() {
            return Err(format!(
                "wall {index} (band {band}): order_lock {:?} must be None",
                wall.path.order_lock
            ));
        }

        let points = &wall.path.points;
        if points.len() < 4 {
            return Err(format!(
                "wall {index} (band {band}): {} points is below the 3-vertex plus closing \
                 repeat minimum of 4",
                points.len()
            ));
        }
        if points.len() != wall.width_profile.widths.len() {
            return Err(format!(
                "wall {index} (band {band}): {} points but {} width-profile entries",
                points.len(),
                wall.width_profile.widths.len()
            ));
        }
        if points.len() != wall.feature_flags.len() {
            return Err(format!(
                "wall {index} (band {band}): {} points but {} feature flags",
                points.len(),
                wall.feature_flags.len()
            ));
        }

        let first = points.first().expect("checked nonempty points");
        let last = points.last().expect("checked nonempty points");
        if !wall.path.is_closed() {
            return Err(format!(
                "wall {index} (band {band}): path is not closed (first {first:?}, last {last:?})"
            ));
        }
        if !points_bit_identical(first, last) {
            return Err(format!(
                "wall {index} (band {band}): the closing repeat {last:?} is not bit-identical \
                 to the first point {first:?}"
            ));
        }

        for (point_index, pair) in points.windows(2).enumerate() {
            if bits_eq(pair[0].x, pair[1].x) && bits_eq(pair[0].y, pair[1].y) {
                return Err(format!(
                    "wall {index} (band {band}): points {point_index} and {} repeat XY ({:?})",
                    point_index + 1,
                    (pair[0].x, pair[0].y)
                ));
            }
        }

        for (point_index, point) in points.iter().enumerate() {
            for (field, value) in [
                ("x", point.x),
                ("y", point.y),
                ("z", point.z),
                ("width", point.width),
                ("flow_factor", point.flow_factor),
            ] {
                if !value.is_finite() {
                    return Err(format!(
                        "wall {index} (band {band}) point {point_index}: {field} {value:?} is not \
                         finite"
                    ));
                }
            }
            require_bits("z", index, point_index, point.z, case.z_mm)?;
            require_bits("width", index, point_index, point.width, expected_width)?;
            require_bits("flow_factor", index, point_index, point.flow_factor, 1.0)?;
            require_bits(
                "dist_to_top_mm",
                index,
                point_index,
                point.dist_to_top_mm,
                0.0,
            )?;
            if point.overhang_quartile.is_some() {
                return Err(format!(
                    "wall {index} (band {band}) point {point_index}: overhang_quartile {:?} must \
                     be None under PathAnnotationMode::Unconsumed",
                    point.overhang_quartile
                ));
            }
            if point.overhang_distance_mm.is_some() {
                return Err(format!(
                    "wall {index} (band {band}) point {point_index}: overhang_distance_mm {:?} \
                     must be None under PathAnnotationMode::Unconsumed",
                    point.overhang_distance_mm
                ));
            }
        }

        for (width_index, width) in wall.width_profile.widths.iter().enumerate() {
            require_bits(
                "width_profile.widths",
                index,
                width_index,
                *width,
                expected_width,
            )?;
        }

        for (flag_index, flag) in wall.feature_flags.iter().enumerate() {
            if *flag != expected_flag {
                return Err(format!(
                    "wall {index} (band {band}) flag {flag_index}: {flag:?} is not the \
                     no-paint fixture flag {expected_flag:?}"
                ));
            }
        }

        let first_width = wall.width_profile.widths.first().expect("checked nonempty");
        let last_width = wall.width_profile.widths.last().expect("checked nonempty");
        if !bits_eq(*first_width, *last_width) {
            return Err(format!(
                "wall {index} (band {band}): the closing width {last_width:?} does not mirror \
                 the first {first_width:?}"
            ));
        }
        let first_flag = wall.feature_flags.first().expect("checked nonempty");
        let last_flag = wall.feature_flags.last().expect("checked nonempty");
        if first_flag != last_flag {
            return Err(format!(
                "wall {index} (band {band}): the closing flag {last_flag:?} does not mirror the \
                 first {first_flag:?}"
            ));
        }
    }
    Ok(())
}

/// Certify a generated case independently, call the real kernel and apply the
/// oracle. `nonempty` counts emitted wall populations for the local in-process
/// nonempty witness.
fn check_classic_case(
    module: &ClassicPerimeters,
    case: &ClassicCase,
    nonempty: &Cell<u64>,
    class_nonempty: &Cell<[u64; CLASS_TAGS.len()]>,
    inner_walls: &Cell<u64>,
) -> TestCaseResult {
    if let Err(error) = validate_geometry(&case.geometry, &case.domain) {
        return fail(format!(
            "generator produced a case its own {:?} domain rejects: {error}",
            case.class
        ));
    }
    let output = match run_classic(module, case) {
        Ok(output) => output,
        Err(error) => {
            return fail(format!(
                "emit_walls rejected a valid {:?} case (wall_count={}, widths={:?}/{:?}, \
                 layer_height={:?}): {error:?}",
                case.class,
                case.wall_count,
                case.outer_width_mm,
                case.inner_width_mm,
                case.layer_height_mm
            ))
        }
    };
    if !output.is_empty() {
        nonempty.set(nonempty.get() + 1);
        let mut per_class = class_nonempty.get();
        per_class[case.class.index()] += 1;
        class_nonempty.set(per_class);
        let inner = output
            .walls
            .iter()
            .filter(|wall| wall.perimeter_index > 0)
            .count();
        inner_walls.set(inner_walls.get() + inner as u64);
    }
    match classic_structure_oracle(case, &output) {
        Ok(()) => Ok(()),
        Err(message) => fail(format!(
            "structure contract violated for {:?} case (wall_count={}, emit={:?}, \
             variant_fuzzy={}, widths={:?}/{:?}, layer_height={:?}, z={:?}): {message}",
            case.class,
            case.wall_count,
            case.emit,
            case.variant_fuzzy,
            case.outer_width_mm,
            case.inner_width_mm,
            case.layer_height_mm,
            case.z_mm
        )),
    }
}

#[test]
fn proptest_classic_wall_structure() {
    let run = PropertyRun::new(STRUCTURE_PROPERTY_ID, STRUCTURE_TEST_NAME)
        .expect("M-CLASSIC-WALL-STRUCTURE registry context");
    let limits = live_limits(&run);
    let module = classic_module();
    let strategy = classic_strategy(&limits, &CLASS_TAGS);
    let nonempty = Cell::new(0_u64);
    let class_nonempty = Cell::new([0_u64; CLASS_TAGS.len()]);
    let inner_walls = Cell::new(0_u64);
    let calls = Cell::new(0_u64);
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        check_classic_case(&module, &case, &nonempty, &class_nonempty, &inner_walls)
    })
    .expect("M-CLASSIC-WALL-STRUCTURE wall structure contract");
    let nonempty = nonempty.get();
    let class_nonempty = class_nonempty.get();
    let inner_walls = inner_walls.get();
    let calls = calls.get();
    assert!(
        nonempty > 0,
        "the generated population emitted no wall for any of {calls} cases, so the structure \
         oracle certified nothing"
    );
    for class in CLASS_TAGS {
        assert!(
            class_nonempty[class.index()] > 0,
            "the generated population emitted no nonempty {} case across {calls} cases \
             (counts {class_nonempty:?})",
            class.name()
        );
    }
    // The band>0 half of the role/width/speed mapping must be exercised by the
    // generated population, not only by the deterministic witness; otherwise
    // that half of the oracle is vacuous for this profile.
    assert!(
        inner_walls > 0,
        "the generated population emitted no band>0 wall across {nonempty} nonempty cases, so \
         the inner-band mapping was never checked"
    );
    println!(
        "M-CLASSIC-WALL-STRUCTURE accepted_calls={calls} nonempty_cases={nonempty} \
         convex={} concave={} holed={} disconnected={} inner_walls={inner_walls}",
        class_nonempty[ClassTag::Convex.index()],
        class_nonempty[ClassTag::Concave.index()],
        class_nonempty[ClassTag::Holed.index()],
        class_nonempty[ClassTag::Disconnected.index()],
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Required deterministic witnesses and the oracle mutation control.
// ─────────────────────────────────────────────────────────────────────────────

/// Coordinate bound for a fixed square witness of `side_mm`.
fn square_bounds(side_mm: f32) -> GeometryBounds {
    // exhaustive: the witness bound is explicit; defaulting a bound here would
    // hide the fixed square's extent from the fixture review.
    GeometryBounds {
        max_abs_units: mm_to_units(side_mm),
        max_vertices: 4,
        max_holes: 1,
        max_components: 1,
        max_shrink_candidates: 1,
    }
}

/// A fixed axis-aligned square case. Every non-geometry input is the shared
/// witness fixture: 0.4/0.4 mm walls, 0.2 mm layer height and the canonical z.
fn square_case(side_mm: f32, wall_count: u32, emit: EmitMode, variant_fuzzy: bool) -> ClassicCase {
    let side = mm_to_units(side_mm);
    let square = Polygon {
        points: vec![
            Point2 { x: 0, y: 0 },
            Point2 { x: side, y: 0 },
            Point2 { x: side, y: side },
            Point2 { x: 0, y: side },
        ],
    };
    ClassicCase {
        class: ClassTag::Convex,
        domain: ClassTag::Convex.domain(&square_bounds(side_mm)),
        geometry: GeometryCase {
            polygons: vec![ExPolygon {
                contour: square,
                holes: Vec::new(),
            }],
        },
        wall_count,
        outer_width_mm: WITNESS_WALL_WIDTH_MM,
        inner_width_mm: WITNESS_WALL_WIDTH_MM,
        layer_height_mm: WITNESS_LAYER_HEIGHT_MM,
        z_mm: WITNESS_Z_MM,
        emit,
        variant_fuzzy,
        outer_speed_factor: EXPECTED_OUTER_SPEED_FACTOR,
        inner_speed_factor: EXPECTED_INNER_SPEED_FACTOR,
    }
}

/// The fixed 20 mm square, 3 bands, both emit gates and 0.4/0.4/0.2 mm.
fn witness_case(variant_fuzzy: bool) -> ClassicCase {
    square_case(
        WITNESS_SIDE_MM,
        WITNESS_WALL_COUNT,
        EmitMode::Both,
        variant_fuzzy,
    )
}

/// The 20 mm witness inputs with a different polygon list. Only the deliberate
/// degeneracy controls use this.
fn case_with_polygons(polygons: Vec<ExPolygon>) -> ClassicCase {
    let mut case = square_case(WITNESS_SIDE_MM, WITNESS_WALL_COUNT, EmitMode::Both, false);
    case.geometry = GeometryCase { polygons };
    case
}

#[test]
fn classic_required_witnesses_are_nonempty() {
    let module = classic_module();
    // The fixture's speed-factor expectation must match `from_config`'s mapping;
    // otherwise the generated property would certify the wrong expectation.
    assert!(bits_eq(
        module.outer_speed_factor,
        EXPECTED_OUTER_SPEED_FACTOR
    ));
    assert!(bits_eq(
        module.inner_speed_factor,
        EXPECTED_INNER_SPEED_FACTOR
    ));

    for variant_fuzzy in [false, true] {
        let case = witness_case(variant_fuzzy);
        let output = run_classic(&module, &case).expect("the 20 mm witness is a valid input");
        assert!(
            !output.is_empty(),
            "the 20 mm/3-band witness emitted no wall (variant_fuzzy={variant_fuzzy})"
        );

        let bands: BTreeSet<u32> = output
            .walls
            .iter()
            .map(|wall| wall.perimeter_index)
            .collect();
        assert_eq!(
            bands,
            BTreeSet::from([0_u32, 1, 2]),
            "the 20 mm/3-band witness must emit exactly bands {{0,1,2}} \
             (variant_fuzzy={variant_fuzzy}); walls={}",
            output.walls.len()
        );

        classic_structure_oracle(&case, &output).unwrap_or_else(|message| {
            panic!("the deterministic witness violates the oracle (variant_fuzzy={variant_fuzzy}): {message}")
        });

        for wall in &output.walls {
            let expected_loop = if wall.perimeter_index == 0 {
                LoopType::Outer
            } else {
                LoopType::Inner
            };
            assert_eq!(wall.loop_type, expected_loop);
            for flag in &wall.feature_flags {
                assert_eq!(
                    flag.fuzzy_skin, variant_fuzzy,
                    "the variant_fuzzy mode must be stamped on every flag"
                );
            }
        }
        assert!(
            output
                .walls
                .iter()
                .any(|wall| wall.loop_type == LoopType::Outer),
            "the witness must contain an outer wall"
        );
        assert!(
            output
                .walls
                .iter()
                .any(|wall| wall.loop_type == LoopType::Inner),
            "the witness must contain inner walls"
        );
    }
}

fn outer_wall_index(output: &ClassicOutput) -> usize {
    output
        .walls
        .iter()
        .position(|wall| wall.perimeter_index == 0)
        .expect("the witness emits a band-0 outer wall")
}

fn inner_wall_index(output: &ClassicOutput) -> usize {
    output
        .walls
        .iter()
        .position(|wall| wall.perimeter_index > 0)
        .expect("the witness emits an inner wall")
}

#[test]
fn classic_structure_oracle_rejects_mutations() {
    let module = classic_module();
    let case = witness_case(true);
    let baseline = run_classic(&module, &case).expect("the 20 mm witness is a valid input");
    assert!(
        !baseline.is_empty(),
        "the mutation control needs a real output"
    );
    classic_structure_oracle(&case, &baseline)
        .expect("the unmutated witness output must pass its own oracle");

    let expect_rejected = |label: &str, mutate: &dyn Fn(&mut ClassicOutput)| {
        let mut mutant = baseline.clone();
        mutate(&mut mutant);
        assert_ne!(
            mutant, baseline,
            "mutation `{label}` did not change the witness output, so it proves nothing"
        );
        let outcome = classic_structure_oracle(&case, &mutant);
        assert!(
            outcome.is_err(),
            "the structure oracle accepted mutation `{label}`"
        );
    };
    let expect_rejected_naming =
        |label: &str, expected: &str, mutate: &dyn Fn(&mut ClassicOutput)| {
            let mut mutant = baseline.clone();
            mutate(&mut mutant);
            assert_ne!(
                mutant, baseline,
                "mutation `{label}` did not change the witness output, so it proves nothing"
            );
            let message = classic_structure_oracle(&case, &mutant)
                .expect_err(&format!("the structure oracle accepted mutation `{label}`"));
            assert!(
                message.contains(expected),
                "mutation `{label}` was rejected but not for `{expected}`: {message}"
            );
        };

    expect_rejected("missing closing repeat", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        wall.path.points.pop();
        wall.width_profile.widths.pop();
        wall.feature_flags.pop();
    });
    expect_rejected("shifted closing repeat", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let last = wall.path.points.last_mut().expect("witness points");
        last.x += 0.01;
    });
    expect_rejected("repeated consecutive XY", &|mutant| {
        let index = inner_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let first = wall.path.points[0];
        wall.path.points[1] = first;
    });
    expect_rejected("too few points", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        wall.path.points.truncate(3);
        wall.width_profile.widths.truncate(3);
        wall.feature_flags.truncate(3);
    });
    expect_rejected("band outside the request", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].perimeter_index = case.wall_count;
    });
    expect_rejected("band mislabeled", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].perimeter_index = 1;
    });
    expect_rejected("loop type", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].loop_type = LoopType::Inner;
    });
    expect_rejected("role", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].path.role = ExtrusionRole::InnerWall;
    });
    expect_rejected("speed factor", &|mutant| {
        let index = inner_wall_index(mutant);
        mutant.walls[index].path.speed_factor += 0.25;
    });
    expect_rejected_naming("point width", "width", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let point_index = wall.path.points.len() - 2;
        wall.path.points[point_index].width += 0.05;
    });
    expect_rejected("width profile entry", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].width_profile.widths[1] += 0.05;
    });
    expect_rejected_naming("z", "z", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let point_index = wall.path.points.len() - 2;
        wall.path.points[point_index].z += 0.1;
    });
    expect_rejected_naming("flow factor", "flow_factor", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let point_index = wall.path.points.len() - 2;
        wall.path.points[point_index].flow_factor = 1.5;
    });
    expect_rejected_naming("nonfinite coordinate", "is not finite", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let point_index = wall.path.points.len() - 2;
        wall.path.points[point_index].x = f32::NAN;
    });
    expect_rejected_naming("nonfinite width", "is not finite", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let point_index = wall.path.points.len() - 2;
        wall.path.points[point_index].width = f32::INFINITY;
    });
    expect_rejected("fuzzy skin flag", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].feature_flags[0].fuzzy_skin = !case.variant_fuzzy;
    });
    expect_rejected("bridge flag", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].feature_flags[0].is_bridge = true;
    });
    expect_rejected("closing width", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let last = wall
            .width_profile
            .widths
            .last_mut()
            .expect("witness widths");
        *last += 0.05;
    });
    expect_rejected("closing flag", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let last = wall.feature_flags.last_mut().expect("witness flags");
        last.is_bridge = true;
    });
    expect_rejected("parallel widths", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].width_profile.widths.pop();
    });
    expect_rejected("parallel flags", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].feature_flags.pop();
    });
    expect_rejected("tool index", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].path.tool_index = Some(1);
    });
    expect_rejected("order lock", &|mutant| {
        let index = outer_wall_index(mutant);
        mutant.walls[index].path.order_lock = Some(1);
    });
    expect_rejected_naming("overhang quartile", "overhang_quartile", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let point_index = wall.path.points.len() - 2;
        wall.path.points[point_index].overhang_quartile = Some(2);
    });
    expect_rejected_naming("dist to top", "dist_to_top_mm", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let point_index = wall.path.points.len() - 2;
        wall.path.points[point_index].dist_to_top_mm = 0.5;
    });
    expect_rejected_naming("overhang distance", "overhang_distance_mm", &|mutant| {
        let index = outer_wall_index(mutant);
        let wall = &mut mutant.walls[index];
        let point_index = wall.path.points.len() - 2;
        wall.path.points[point_index].overhang_distance_mm = Some(0.25);
    });
}

// ─────────────────────────────────────────────────────────────────────────────
// Shrink protocol probe.
// ─────────────────────────────────────────────────────────────────────────────

/// Re-assert every no-shrink invariant and the live structure contract on the
/// tree's current value.
fn check_shrunk_case(
    module: &ClassicPerimeters,
    case: &ClassicCase,
    fixed: &FixedModes,
    step: &str,
) {
    assert_eq!(
        case.class, fixed.class,
        "{step}: shrinking switched the selected topology class"
    );
    assert_eq!(
        case.domain, fixed.domain,
        "{step}: shrinking changed the declared topology domain"
    );
    assert_eq!(
        case.emit, fixed.emit,
        "{step}: shrinking flipped the emit-gate mode"
    );
    assert_eq!(
        case.variant_fuzzy, fixed.variant_fuzzy,
        "{step}: shrinking flipped the variant_fuzzy mode"
    );
    validate_geometry(&case.geometry, &case.domain).unwrap_or_else(|error| {
        panic!("{step}: the shrunk geometry is no longer valid for its domain: {error}")
    });
    let output = run_classic(module, case).unwrap_or_else(|error| {
        panic!("{step}: the shrunk case was rejected by emit_walls: {error:?}")
    });
    classic_structure_oracle(case, &output)
        .unwrap_or_else(|message| panic!("{step}: the shrunk case violates the oracle: {message}"));
}

/// The no-shrink selectors must report both directions as unavailable.
fn assert_no_shrink_tree<S>(runner: &mut TestRunner, strategy: &S, label: &str)
where
    S: Strategy,
    S::Value: PartialEq + std::fmt::Debug,
{
    let mut tree = strategy
        .new_tree(runner)
        .unwrap_or_else(|reason| panic!("{label}: no-shrink tree construction failed: {reason}"));
    let before = tree.current();
    assert!(!tree.simplify(), "{label}: a no-shrink tree simplified");
    assert!(!tree.complicate(), "{label}: a no-shrink tree complicated");
    assert_eq!(
        tree.current(),
        before,
        "{label}: a no-shrink tree changed value"
    );
}

#[test]
fn classic_shrink_preserves_emit_mode_and_topology() {
    let limits = shrink_probe_limits();
    let module = classic_module();
    let mut runner = TestRunner::deterministic();

    // 1. The class, emit and variant_fuzzy selectors are single no-shrink
    //    selector strategies: neither direction may move.
    assert_no_shrink_tree(
        &mut runner,
        &prop::sample::select(vec![
            ClassTag::Convex,
            ClassTag::Concave,
            ClassTag::Holed,
            ClassTag::Disconnected,
        ])
        .no_shrink(),
        "class selector",
    );
    assert_no_shrink_tree(
        &mut runner,
        &prop::sample::select(vec![
            EmitMode::Both,
            EmitMode::OuterOnly,
            EmitMode::InnerOnly,
        ])
        .no_shrink(),
        "emit mode selector",
    );
    assert_no_shrink_tree(
        &mut runner,
        &prop::sample::select(vec![false, true]).no_shrink(),
        "variant_fuzzy selector",
    );

    // 2. Per class: drive the real strategy through simplify and complicate.
    //    Each step is a simplify immediately followed by its complicate, so the
    //    element proptest most recently simplified is the one asked to restore;
    //    that is the only direction `TupleValueTree` will accept, and it keeps
    //    both directions genuinely exercised instead of a simplify-only walk.
    //    The class, its topology witness, both mode selectors and the band/role
    //    invariants must survive; at least one ordinary scalar must move.
    for class in CLASS_TAGS {
        let strategy = classic_strategy(&limits, &[class]);
        let mut tree = strategy.new_tree(&mut runner).unwrap_or_else(|reason| {
            panic!("{class:?}: shrink-probe tree construction failed: {reason}")
        });
        let mut previous = tree.current();
        assert_eq!(
            previous.class, class,
            "the single-class strategy generated another class"
        );
        let fixed = FixedModes::of(&previous);
        let mut changed = false;
        let mut simplified = 0_usize;
        let mut complicated = 0_usize;

        for _ in 0..SHRINK_STEPS {
            if !tree.simplify() {
                break;
            }
            simplified += 1;
            let current = tree.current();
            check_shrunk_case(&module, &current, &fixed, "simplify");
            changed |= OrdinaryValues::of(&current) != OrdinaryValues::of(&previous);
            previous = current;

            if !tree.complicate() {
                break;
            }
            complicated += 1;
            let current = tree.current();
            check_shrunk_case(&module, &current, &fixed, "complicate");
            changed |= OrdinaryValues::of(&current) != OrdinaryValues::of(&previous);
            previous = current;
        }

        assert!(
            simplified > 0 && complicated > 0,
            "{class:?}: the shrink probe moved simplify={simplified} complicate={complicated}; \
             both directions must be exercised or the probe certifies nothing"
        );
        assert!(
            changed,
            "{class:?}: no ordinary scalar changed during simplify/complicate, so the probe \
             cannot show that shrinkable inputs still shrink"
        );
        println!(
            "{class:?}: simplify_steps={simplified} complicate_steps={complicated} \
             final_wall_count={} final_z={:?}",
            previous.wall_count, previous.z_mm
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// M-CLASSIC-BAND-SELECTION — gates, allowed-prefix bands and degeneracy.
// ─────────────────────────────────────────────────────────────────────────────

/// Independent band-selection contract for ordinary classic walls.
///
/// Runs the full Step 3 structure oracle first, so a band that reaches this
/// second layer is already known to be a well-formed wall. Then:
///
/// * every emitted band must satisfy its own gate (band 0 is the outer wall,
///   band > 0 is an inner wall);
/// * for nonempty output the emitted set must be the computed allowed prefix
///   `{ i in 0..=max : allowed(i) }` implied by the gates — a missing allowed
///   index below the maximum is a gap, not a smaller request;
/// * `emit_outer == false` admits no seam candidates and `emit_inner == false`
///   admits no infill areas.
///
/// The emitted wall/band count is never compared with `wall_count`. That value
/// only bounds the computed prefix (`0..wall_count`), and a shape too small to
/// hold the request legitimately emits fewer bands.
fn classic_band_oracle(case: &ClassicCase, output: &ClassicOutput) -> Result<(), String> {
    classic_structure_oracle(case, output)?;

    let mut bands: BTreeSet<u32> = BTreeSet::new();
    for wall in &output.walls {
        bands.insert(wall.perimeter_index);
    }

    for band in &bands {
        if !band_allowed(case.emit, *band) {
            return Err(format!(
                "band {band} was emitted while the {:?} gate disables {} walls",
                case.emit,
                if *band == 0 { "outer" } else { "inner" }
            ));
        }
    }

    if let Some(&max) = bands.iter().next_back() {
        for band in 0..=max {
            if band_allowed(case.emit, band) && !bands.contains(&band) {
                return Err(format!(
                    "band {band} is allowed by the {:?} gate but missing below the emitted \
                     maximum {max}; the emitted set {bands:?} is not the allowed prefix",
                    case.emit
                ));
            }
        }
    }

    let (emit_outer, emit_inner) = case.emit.gates();
    if !emit_outer && !output.seam_candidates.is_empty() {
        return Err(format!(
            "emit_outer is false but {} seam candidate(s) were emitted",
            output.seam_candidates.len()
        ));
    }
    if !emit_inner && !output.infill_areas.is_empty() {
        return Err(format!(
            "emit_inner is false but {} infill area set(s) were emitted",
            output.infill_areas.len()
        ));
    }
    Ok(())
}

/// Certify, call the kernel and apply the band oracle, recording the band
/// charter's coverage counters.
fn check_band_case(
    module: &ClassicPerimeters,
    case: &ClassicCase,
    nonempty: &Cell<u64>,
    class_nonempty: &Cell<[u64; CLASS_TAGS.len()]>,
    mode_counts: &Cell<[u64; 3]>,
    single_band_cases: &Cell<u64>,
) -> TestCaseResult {
    if let Err(error) = validate_geometry(&case.geometry, &case.domain) {
        return fail(format!(
            "generator produced a case its own {:?} domain rejects: {error}",
            case.class
        ));
    }
    let output = match run_classic(module, case) {
        Ok(output) => output,
        Err(error) => {
            return fail(format!(
            "emit_walls rejected a valid {:?} case (wall_count={}, emit={:?}, widths={:?}/{:?}, \
                 layer_height={:?}): {error:?}",
            case.class,
            case.wall_count,
            case.emit,
            case.outer_width_mm,
            case.inner_width_mm,
            case.layer_height_mm
        ))
        }
    };
    if !output.is_empty() {
        nonempty.set(nonempty.get() + 1);
        let mut per_class = class_nonempty.get();
        per_class[case.class.index()] += 1;
        class_nonempty.set(per_class);
    }
    let mut counts = mode_counts.get();
    counts[case.emit.index()] += 1;
    mode_counts.set(counts);
    if case.wall_count == MIN_WALL_COUNT {
        single_band_cases.set(single_band_cases.get() + 1);
    }
    match classic_band_oracle(case, &output) {
        Ok(()) => Ok(()),
        Err(message) => fail(format!(
            "band contract violated for {:?} case (wall_count={}, emit={}, variant_fuzzy={}, \
             widths={:?}/{:?}, layer_height={:?}): {message}",
            case.class,
            case.wall_count,
            case.emit.name(),
            case.variant_fuzzy,
            case.outer_width_mm,
            case.inner_width_mm,
            case.layer_height_mm
        )),
    }
}

#[test]
fn proptest_classic_band_selection() {
    let run = PropertyRun::new(BAND_PROPERTY_ID, BAND_TEST_NAME)
        .expect("M-CLASSIC-BAND-SELECTION registry context");
    let limits = live_limits(&run);
    let module = classic_module();
    let strategy = classic_strategy(&limits, &CLASS_TAGS);
    let nonempty = Cell::new(0_u64);
    let class_nonempty = Cell::new([0_u64; CLASS_TAGS.len()]);
    let mode_counts = Cell::new([0_u64; EMIT_MODES.len()]);
    let single_band_cases = Cell::new(0_u64);
    let calls = Cell::new(0_u64);
    run.check(&strategy, |case, _guard| {
        calls.set(calls.get() + 1);
        check_band_case(
            &module,
            &case,
            &nonempty,
            &class_nonempty,
            &mode_counts,
            &single_band_cases,
        )
    })
    .expect("M-CLASSIC-BAND-SELECTION band contract");

    let nonempty = nonempty.get();
    let class_nonempty = class_nonempty.get();
    let mode_counts = mode_counts.get();
    let single_band_cases = single_band_cases.get();
    let calls = calls.get();
    assert!(
        nonempty > 0,
        "the generated population emitted no wall for any of {calls} cases, so the band oracle \
         certified nothing"
    );
    for class in CLASS_TAGS {
        assert!(
            class_nonempty[class.index()] > 0,
            "the generated population emitted no nonempty {} case across {calls} cases \
             (counts {class_nonempty:?})",
            class.name()
        );
    }
    for mode in EMIT_MODES {
        assert!(
            mode_counts[mode.index()] > 0,
            "the generated population never exercised the {} emit mode (counts {mode_counts:?} \
             over {calls} cases)",
            mode.name()
        );
    }
    assert!(
        single_band_cases > 0,
        "the generated population never exercised wall_count == {MIN_WALL_COUNT} (over {calls} \
         cases)"
    );
    println!(
        "M-CLASSIC-BAND-SELECTION accepted_calls={calls} nonempty_cases={nonempty} \
         convex={} concave={} holed={} disconnected={} both={} outer_only={} inner_only={} \
         single_band_cases={single_band_cases}",
        class_nonempty[ClassTag::Convex.index()],
        class_nonempty[ClassTag::Concave.index()],
        class_nonempty[ClassTag::Holed.index()],
        class_nonempty[ClassTag::Disconnected.index()],
        mode_counts[EmitMode::Both.index()],
        mode_counts[EmitMode::OuterOnly.index()],
        mode_counts[EmitMode::InnerOnly.index()],
    );
}

/// Band indices of a real output, for the deterministic controls' assertions.
fn band_set(output: &ClassicOutput) -> BTreeSet<u32> {
    output
        .walls
        .iter()
        .map(|wall| wall.perimeter_index)
        .collect()
}

/// Assert a mutated output differs from its baseline and is rejected by the
/// band oracle.
fn expect_band_rejected(
    label: &str,
    case: &ClassicCase,
    baseline: &ClassicOutput,
    mutant: &ClassicOutput,
) {
    assert_ne!(
        mutant, baseline,
        "mutation `{label}` did not change the real output, so it proves nothing"
    );
    let outcome = classic_band_oracle(case, mutant);
    assert!(
        outcome.is_err(),
        "the band oracle accepted mutation `{label}`"
    );
}

#[test]
fn classic_band_oracle_and_degenerate_cases() {
    let module = classic_module();

    // Real, nonempty witness outputs: all three emit modes, so every gate is
    // exercised against output the kernel actually produced.
    let both = witness_case(true);
    let both_output = run_classic(&module, &both).expect("the 20 mm witness is a valid input");
    assert!(
        !both_output.is_empty(),
        "the both-gates witness must emit walls"
    );
    classic_band_oracle(&both, &both_output)
        .expect("the both-gates witness must satisfy the band oracle");
    assert!(
        !both_output.seam_candidates.is_empty(),
        "the witness outer wall must produce seam candidates, otherwise the seam-gate mutation \
         would be vacuous"
    );
    assert!(
        !both_output.infill_areas.is_empty(),
        "the witness must produce an infill area, otherwise the infill-gate mutation would be \
         vacuous"
    );

    let outer_only = square_case(
        WITNESS_SIDE_MM,
        WITNESS_WALL_COUNT,
        EmitMode::OuterOnly,
        true,
    );
    let outer_output = run_classic(&module, &outer_only).expect("outer-only witness");
    assert!(
        !outer_output.is_empty(),
        "outer-only witness emitted nothing"
    );
    classic_band_oracle(&outer_only, &outer_output).expect("outer-only band contract");
    assert_eq!(
        band_set(&outer_output),
        BTreeSet::from([0]),
        "emit_inner=false must emit only band 0"
    );
    assert!(
        !outer_output.seam_candidates.is_empty(),
        "the outer-only witness must emit seam candidates, otherwise the seam-gate mutation \
         would be vacuous"
    );
    assert!(
        outer_output.infill_areas.is_empty(),
        "emit_inner=false must emit no infill areas"
    );

    let inner_only = square_case(
        WITNESS_SIDE_MM,
        WITNESS_WALL_COUNT,
        EmitMode::InnerOnly,
        true,
    );
    let inner_output = run_classic(&module, &inner_only).expect("inner-only witness");
    assert!(!inner_output.is_empty());
    classic_band_oracle(&inner_only, &inner_output).expect("inner-only band contract");
    assert_eq!(band_set(&inner_output), BTreeSet::from([1, 2]));
    assert!(inner_output.seam_candidates.is_empty());
    assert!(!inner_output.infill_areas.is_empty());

    let single = square_case(WITNESS_SIDE_MM, MIN_WALL_COUNT, EmitMode::Both, false);
    let single_output = run_classic(&module, &single).expect("single-band witness");
    assert!(
        !single_output.is_empty(),
        "the wall_count == 1 witness must emit the outer band"
    );
    classic_band_oracle(&single, &single_output).expect("single-band contract");
    assert_eq!(
        band_set(&single_output),
        BTreeSet::from([0]),
        "wall_count == 1 can only emit band 0"
    );

    let capacity = square_case(CAPACITY_SIDE_MM, CAPACITY_WALL_COUNT, EmitMode::Both, false);
    let capacity_output = run_classic(&module, &capacity).expect("capacity witness");
    classic_band_oracle(&capacity, &capacity_output).expect("capacity band contract");
    let capacity_bands = band_set(&capacity_output);
    assert!(
        !capacity_bands.is_empty(),
        "capacity witness emitted no band"
    );
    assert!(
        capacity_bands.len() < CAPACITY_WALL_COUNT as usize,
        "capacity witness emitted {} bands, expected fewer than {CAPACITY_WALL_COUNT}",
        capacity_bands.len()
    );
    println!(
        "classic capacity witness: requested={CAPACITY_WALL_COUNT} actual_bands={capacity_bands:?}"
    );

    // Removing band 1 leaves real, otherwise-valid bands 0 and 2. The structure
    // oracle alone accepts that output; the band oracle must reject the gap.
    let mut gap = both_output.clone();
    gap.walls.retain(|wall| wall.perimeter_index != 1);
    assert!(
        classic_structure_oracle(&both, &gap).is_ok(),
        "the gap mutant must stay structurally valid so the allowed-prefix rule is what rejects it"
    );
    expect_band_rejected("allowed band gap", &both, &both_output, &gap);

    // These mutants begin from real output for the *opposite* mode and add one
    // real element whose gate is wrong for the case being checked. The structure
    // oracle must still accept each mutant, so only the gate layer rejects it.
    let inner_wall = both_output
        .walls
        .iter()
        .find(|wall| wall.perimeter_index > 0)
        .expect("the both-gates witness emits an inner wall")
        .clone();
    let mut inner_leak = outer_output.clone();
    inner_leak.walls.push(inner_wall);
    assert!(
        classic_structure_oracle(&outer_only, &inner_leak).is_ok(),
        "the inner-wall leak must stay structurally valid so the emit_inner gate rejects it"
    );
    expect_band_rejected(
        "inner wall under emit_inner=false",
        &outer_only,
        &outer_output,
        &inner_leak,
    );

    let outer_wall = both_output
        .walls
        .iter()
        .find(|wall| wall.perimeter_index == 0)
        .expect("the both-gates witness emits an outer wall")
        .clone();
    let mut outer_leak = inner_output.clone();
    outer_leak.walls.push(outer_wall);
    assert!(
        classic_structure_oracle(&inner_only, &outer_leak).is_ok(),
        "the outer-wall leak must stay structurally valid so the emit_outer gate rejects it"
    );
    expect_band_rejected(
        "outer wall under emit_outer=false",
        &inner_only,
        &inner_output,
        &outer_leak,
    );

    let mut seam_violation = inner_output.clone();
    seam_violation
        .seam_candidates
        .push(both_output.seam_candidates[0]);
    expect_band_rejected(
        "seams under emit_outer=false",
        &inner_only,
        &inner_output,
        &seam_violation,
    );

    let mut infill_violation = outer_output.clone();
    infill_violation.infill_areas = both_output.infill_areas.clone();
    expect_band_rejected(
        "infill under emit_inner=false",
        &outer_only,
        &outer_output,
        &infill_violation,
    );

    let two_point = ExPolygon {
        contour: Polygon {
            points: vec![Point2 { x: 0, y: 0 }, Point2 { x: 100, y: 0 }],
        },
        holes: Vec::new(),
    };
    let collinear = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 100, y: 0 },
                Point2 { x: 200, y: 0 },
            ],
        },
        holes: Vec::new(),
    };
    for (label, polygons) in [
        ("two-point contour", vec![two_point]),
        ("three-point collinear contour", vec![collinear]),
        ("empty polygon list", Vec::new()),
    ] {
        let case = case_with_polygons(polygons);
        let output = run_classic(&module, &case).expect(label);
        assert!(output.walls.is_empty(), "{label}: emitted walls");
        assert!(output.infill_areas.is_empty(), "{label}: emitted infill");
        assert!(
            output.seam_candidates.is_empty(),
            "{label}: emitted seam candidates"
        );
        classic_band_oracle(&case, &output).expect(label);
    }
}

#[test]
fn classic_negative_spacing_is_fatal() {
    let module = classic_module();
    let mut case = witness_case(false);
    case.outer_width_mm = NEGATIVE_SPACING_WIDTH_MM;
    assert!(
        line_width_to_spacing(case.outer_width_mm, case.layer_height_mm).is_err(),
        "the control must enter the documented nonpositive-spacing path"
    );
    assert!(
        line_width_to_spacing(case.inner_width_mm, case.layer_height_mm).is_ok(),
        "the inner width must stay valid so the outer width is the sole cause"
    );
    let error = run_classic(&module, &case)
        .expect_err("a width below the positive-spacing threshold must be fatal");
    assert_eq!(
        error.code,
        super::ERR_NEGATIVE_SPACING,
        "the fatal code must be the module's ERR_NEGATIVE_SPACING: {error}"
    );
    assert!(error.fatal, "negative spacing must be fatal: {error:?}");

    // The valid property domain never enters this path: every width/layer-height
    // pair the live strategy can draw clears the threshold with a margin.
    let limits = shrink_probe_limits();
    let mut pairs = 0_usize;
    for width_units in limits.width_units.clone() {
        for layer_units in limits.layer_height_units.clone() {
            let width = units_to_mm(width_units);
            let layer = units_to_mm(layer_units);
            assert!(
                line_width_to_spacing(width, layer).is_ok(),
                "the property domain admits width {width_units} units with layer height \
                 {layer_units} units, whose spacing collapses to nonpositive"
            );
            pairs += 1;
        }
    }
    assert!(pairs > 0, "the spacing cross-check covered no pair");
    println!(
        "M-CLASSIC-BAND-SELECTION negative_spacing_code={} valid_width_height_pairs={pairs}",
        error.code
    );
}

#[test]
fn classic_property_domains_reject_unsafe_bounds() {
    fn expect_invalid(label: &str, bounds: BTreeMap<String, u64>) {
        match checked_limits(&bounds) {
            Err(DomainError::InvalidBounds(message)) => {
                assert!(!message.is_empty(), "{label}: empty InvalidBounds payload")
            }
            other => panic!("{label}: expected DomainError::InvalidBounds, got {other:?}"),
        }
    }

    fn expect_overflow(label: &str, bounds: BTreeMap<String, u64>) {
        match checked_limits(&bounds) {
            Err(DomainError::ArithmeticOverflow) => {}
            other => panic!("{label}: expected DomainError::ArithmeticOverflow, got {other:?}"),
        }
    }

    let valid = probe_bound_map();
    let limits = checked_limits(&valid).expect("the valid control bounds must be accepted");
    assert_eq!(
        limits.geometry.len(),
        CLASS_TAGS.len(),
        "the checked builder must prebuild every class strategy"
    );
    for tag in CLASS_TAGS {
        assert!(
            limits
                .geometry
                .iter()
                .any(|(candidate, _)| *candidate == tag),
            "missing prebuilt strategy for {tag:?}"
        );
    }

    // No declared key may silently fall back to a helper default.
    for key in CLASSIC_BOUND_KEYS {
        let mut missing = valid.clone();
        missing.remove(key);
        expect_invalid(&format!("missing bound `{key}`"), missing);
    }

    for (label, key, value) in [
        ("zero wall count", "max_wall_count", 0),
        ("too few vertices", "max_vertices", 7),
        ("impossible Holed minimum", "max_holes", 0),
        ("impossible Disconnected minimum", "max_components", 1),
        ("too little coordinate room", "max_abs_units", 15),
        ("zero coordinate bound", "max_abs_units", 0),
        (
            "coordinate bound above the shared cap",
            "max_abs_units",
            1_000_001,
        ),
        ("zero shrink candidates", "max_shrink_candidates", 0),
        ("nonpositive width", "max_width_units", 0),
        ("nonpositive layer height", "max_layer_height_units", 0),
    ] {
        let mut bounds = valid.clone();
        bounds.insert(key.to_owned(), value);
        expect_invalid(label, bounds);
    }

    let mut no_spacing_margin = valid.clone();
    no_spacing_margin.insert("max_width_units".to_owned(), 1);
    no_spacing_margin.insert("max_layer_height_units".to_owned(), 100);
    match checked_limits(&no_spacing_margin) {
        Err(DomainError::InvalidBounds(message)) => assert!(
            message.contains("spacing"),
            "a nonpositive line-width spacing margin must be reported as a spacing failure, \
             got: {message}"
        ),
        other => panic!(
            "nonpositive line-width spacing margin: expected DomainError::InvalidBounds, got \
             {other:?}"
        ),
    }

    // Unrepresentable keys are checked conversions, never clamped bounds.
    let mut oversized_abs = valid.clone();
    oversized_abs.insert("max_abs_units".to_owned(), i64::MAX as u64 + 1);
    expect_overflow("max_abs_units above i64::MAX", oversized_abs);

    let mut oversized_walls = valid.clone();
    oversized_walls.insert("max_wall_count".to_owned(), u32::MAX as u64 + 1);
    expect_overflow("max_wall_count above u32::MAX", oversized_walls);

    let mut oversized_shrink = valid;
    oversized_shrink.insert("max_shrink_candidates".to_owned(), u32::MAX as u64 + 1);
    expect_overflow("max_shrink_candidates above u32::MAX", oversized_shrink);
}
