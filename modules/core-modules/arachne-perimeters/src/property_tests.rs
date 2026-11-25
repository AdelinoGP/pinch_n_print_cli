//! Opt-in property tests and deterministic controls for this module's three
//! private Arachne kernels.
//!
//! Charters (`docs/spec_packets/proptest-openrscad_10_pbt-arachne`):
//!
//! * `M-ARACHNE-CLASSIFY` — `classify_line`'s role/loop-type table.
//! * `M-ARACHNE-BUILD` — `ArachnePerimeters::build_walls`' point, width, flag
//!   and topology correspondence.
//! * `M-ARACHNE-SEQUENCE` — `commit_wall_sequence`'s region-local, lossless wall
//!   commitment.
//!
//! Registered from `src/lib.rs` with the required stacked, separate gates:
//!
//! ```text
//! #[cfg(test)]
//! #[cfg(feature = "property_testing")]
//! mod property_tests;
//! ```
//!
//! These kernels are private (`fn classify_line`, `fn commit_wall_sequence`,
//! `ArachnePerimeters::build_walls`), so the properties live here as a
//! crate-internal unit-test module. Relocating them under `tests/**` would make
//! them integration tests, which cannot see the private kernels; production
//! visibility is deliberately not widened for tests.
//!
//! # Literal review (AC-8)
//!
//! `cargo xtask check-literals` is **not** evidence for this file:
//! `collect_enforced_files` (`xtask/src/check_literals.rs`) never collects
//! `modules/core-modules/*/src`, and `LiteralVisitor::visit_item_mod` does not
//! follow an out-of-line `mod property_tests;`. The watched types this file can
//! name (re-derived from `crates/*/src/**` `pub` structs with >= 5 named fields)
//! are `Point3WithWidth`, `ExtrusionPath3D`, `WallFeatureFlags`, `WallLoop`
//! (`crates/slicer-ir/src/slice_ir.rs`) and `ArachneParams`
//! (`crates/slicer-sdk/src/host.rs`). The last is the SDK's host-side *mirror*
//! struct, not `slicer_core::arachne::pipeline::ArachneParams` (which is
//! `host-algos`-gated and so not nameable from this wasm guest crate). Every
//! literal of those five uses a `..` rest; `ExtrusionLine` (4 fields),
//! `ExtrusionJunction` (2) and `WidthProfile` (1) are below the watch
//! threshold.
//!
//! # Registry identity
//!
//! The three `proptest_`-named bodies require the foundation runner's
//! `PNP_PROPERTY_CONTEXT` and are registered by a later step. Run plain they
//! fail with `missing_property_context`, which is the expected and required
//! behaviour — they are not standalone executions.

#![allow(missing_docs)]

use proptest::prelude::*;
use proptest::strategy::BoxedStrategy;
use proptest::test_runner::{TestCaseError, TestCaseResult};

use slicer_core::perimeter_spatial::PerimeterSpatialContext;
use slicer_ir::{
    mm_to_units, units_to_mm, ExPolygon, ExtrusionJunction, ExtrusionLine, ExtrusionRole, LoopType,
    Point2, Point3WithWidth, Polygon, WallBoundaryType, WallFeatureFlags, WallLoop, WidthProfile,
};
use slicer_sdk::host::{ArachneParams, WallSequence};
use slicer_sdk::test_support::fixtures::{wall_loop_base, SliceRegionViewBuilder};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::{CaseGuard, PropertyRun};
use slicer_sdk::views::SliceRegionView;

use super::{classify_line, commit_wall_sequence, ArachnePerimeters};

// ─────────────────────────────────────────────────────────────────────────────
// Shared plumbing.
// ─────────────────────────────────────────────────────────────────────────────

const CLASSIFY_PROPERTY_ID: &str = "M-ARACHNE-CLASSIFY";
const CLASSIFY_TEST_NAME: &str = "property_tests::proptest_module_arachne_classify";
const BUILD_PROPERTY_ID: &str = "M-ARACHNE-BUILD";
const BUILD_TEST_NAME: &str = "property_tests::proptest_module_arachne_build_walls";
const SEQUENCE_PROPERTY_ID: &str = "M-ARACHNE-SEQUENCE";
const SEQUENCE_TEST_NAME: &str = "property_tests::proptest_module_arachne_sequence";

fn fail(message: impl Into<String>) -> TestCaseResult {
    Err(TestCaseError::fail(message.into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// M-ARACHNE-CLASSIFY — the real role table.
// ─────────────────────────────────────────────────────────────────────────────

/// Producer-resolved overhang band width (mm), as
/// `SurfaceClassificationIR::overhang_band_width_mm` would carry it. Since
/// ticket 142 the consumer buckets the signed distance to the previous-layer
/// boundary with this width instead of reading producer-side band polygons.
const OVERHANG_BAND_WIDTH_MM: f32 = 0.4;
/// Signed-distance tolerance (mm) below which a point counts as lying on the
/// previous-layer boundary. Restated here from the documented contract so the
/// oracle does not call the production bucket function.
const OVERHANG_BOUNDARY_EPSILON_MM: f32 = 1.0e-4;

/// Written ceiling for `max_inset_idx`. The table is exhaustive well before
/// this; the ceiling only bounds the generated index's narrowing target.
const MAX_INSET_IDX_CEILING: u64 = 64;

/// Validate `max_inset_idx` before any strategy allocates.
///
/// Zero is rejected: the charter requires a positive-inset witness, and an
/// inset bound of zero would make the `InnerWall` row unreachable.
fn checked_classify_limits(max_inset_idx: u64) -> Result<u32, String> {
    if max_inset_idx == 0 {
        return Err(String::from(
            "max_inset_idx must be positive: the classify charter needs a positive-inset witness",
        ));
    }
    if max_inset_idx > MAX_INSET_IDX_CEILING {
        return Err(format!(
            "max_inset_idx {max_inset_idx} exceeds the written ceiling {MAX_INSET_IDX_CEILING}"
        ));
    }
    // Checked narrowing after the range check, never before it.
    u32::try_from(max_inset_idx).map_err(|_| String::from("max_inset_idx does not fit u32"))
}

/// Independent role/loop-type table for `classify_line`.
///
/// Derived from the documented contract, never from the function under test:
/// canonical `traverse_extrusions` (`PerimeterGenerator.cpp`) sets the role from
/// `inset_idx` alone (inset 0 => outermost, else perimeter), and this module's
/// single documented refinement relabels the widened centre bead of a thin
/// region — an odd line at inset 0 with thin walls enabled — as `ThinWall`.
/// `is_closed` is not an input.
fn expected_classification(
    is_odd: bool,
    inset_idx: u32,
    print_thin_walls: bool,
) -> (ExtrusionRole, LoopType) {
    if inset_idx == 0 {
        if is_odd && print_thin_walls {
            (ExtrusionRole::ThinWall, LoopType::ThinWall)
        } else {
            (ExtrusionRole::OuterWall, LoopType::Outer)
        }
    } else {
        (ExtrusionRole::InnerWall, LoopType::Inner)
    }
}

/// The rejected mutation: a deeper odd wall labelled `GapFill`.
///
/// Built explicitly here so no control can derive the wrong answer from the
/// production function. `is_odd` is structural (the centre bead of an odd bead
/// count, or an open line) and must never turn a real wall into a gap fill.
fn deeper_odd_as_gap_fill(
    is_odd: bool,
    inset_idx: u32,
    print_thin_walls: bool,
) -> (ExtrusionRole, LoopType) {
    if is_odd && inset_idx > 0 {
        (ExtrusionRole::GapFill, LoopType::GapFill)
    } else {
        expected_classification(is_odd, inset_idx, print_thin_walls)
    }
}

fn point3(x_mm: f32, y_mm: f32, width_mm: f32) -> Point3WithWidth {
    Point3WithWidth {
        x: x_mm,
        y: y_mm,
        width: width_mm,
        flow_factor: 1.0,
        ..Point3WithWidth::default()
    }
}

/// One generated classification case.
#[derive(Debug, Clone, Copy)]
struct ClassifyCase {
    is_odd: bool,
    inset_idx: u32,
    print_thin_walls: bool,
    is_closed: bool,
    junction_count: u32,
}

fn classify_strategy(max_inset_idx: u32) -> BoxedStrategy<ClassifyCase> {
    (
        any::<bool>(),
        // Zero/positive selection without a union alternative. An independent
        // boolean picks the zero row (`true`) or the positive row (`false`);
        // the positive component is drawn from `1..=max_inset_idx`, a range
        // that never contains zero.
        //
        // Shrink behaviour: proptest simplifies `bool` only in the `true ->
        // false` direction, and `false` is the positive row with no further
        // simplification available. So a positive draw's selector is already
        // at its shrink floor and its inset can only shrink *inside*
        // `1..=max_inset_idx`; it can never cross to the zero row. (A
        // `prop_oneof![Just(0), 1..=max]` union is the opposite: its `simplify`
        // may switch an accepted positive case to the `Just(0)` alternative.)
        // A zero draw can shrink into the positive row, which is a legal
        // classification case in its own right.
        //
        // Every expected value is re-derived from the case that is actually
        // checked (`check_classify_case` rebuilds the line from the shrunk
        // fields and recomputes `expected_classification`), so neither shrink
        // direction can leave a stale expectation behind: there is no false
        // green either way.
        any::<bool>(),
        1u32..=max_inset_idx,
        any::<bool>(),
        any::<bool>(),
        0u32..=3,
    )
        .prop_map(
            |(is_odd, zero_row, positive_inset, print_thin_walls, is_closed, junction_count)| {
                ClassifyCase {
                    is_odd,
                    inset_idx: if zero_row { 0 } else { positive_inset },
                    print_thin_walls,
                    is_closed,
                    junction_count,
                }
            },
        )
        .boxed()
}

fn classify_case_line(case: &ClassifyCase) -> ExtrusionLine {
    ExtrusionLine {
        junctions: (0..case.junction_count)
            .map(|index| ExtrusionJunction {
                p: point3(index as f32 * 0.5, index as f32 * 0.25, 0.4),
                perimeter_index: case.inset_idx,
            })
            .collect(),
        inset_idx: case.inset_idx,
        is_odd: case.is_odd,
        is_closed: case.is_closed,
    }
}

fn check_classify_case(case: &ClassifyCase) -> TestCaseResult {
    let line = classify_case_line(case);
    let observed = classify_line(&line, case.print_thin_walls);
    let expected = expected_classification(case.is_odd, case.inset_idx, case.print_thin_walls);
    if observed != expected {
        return fail(format!(
            "classify_line(is_odd={}, inset_idx={}, print_thin_walls={}) = {observed:?}, \
             the independent table says {expected:?}",
            case.is_odd, case.inset_idx, case.print_thin_walls
        ));
    }

    // `is_closed` alone changes no classification.
    let mut toggled = line.clone();
    toggled.is_closed = !case.is_closed;
    if classify_line(&toggled, case.print_thin_walls) != observed {
        return fail(format!(
            "toggling is_closed changed classify_line's result for \
             (is_odd={}, inset_idx={}, print_thin_walls={})",
            case.is_odd, case.inset_idx, case.print_thin_walls
        ));
    }
    Ok(())
}

#[test]
fn proptest_module_arachne_classify() {
    let run = PropertyRun::new(CLASSIFY_PROPERTY_ID, CLASSIFY_TEST_NAME)
        .expect("M-ARACHNE-CLASSIFY registry context");
    let max_inset_idx =
        checked_classify_limits(run.bound("max_inset_idx").expect("max_inset_idx bound"))
            .expect("M-ARACHNE-CLASSIFY safety ceilings");
    let strategy = classify_strategy(max_inset_idx);
    run.check(&strategy, |case, _guard| check_classify_case(&case))
        .expect("M-ARACHNE-CLASSIFY role table contract");
}

// ─────────────────────────────────────────────────────────────────────────────
// M-ARACHNE-BUILD — point, width, flag and topology correspondence.
// ─────────────────────────────────────────────────────────────────────────────

/// Fixed half-extent of the deterministic witness and regression scenes, in
/// slicer units: a 4 mm square. The generated `M-ARACHNE-BUILD` property does
/// not use this constant; it samples its own half-extent from the
/// `max_abs_mm_units` bound.
const WITNESS_HALF_UNITS: i64 = 20_000;

/// Layer height used by the reverse-trigger regression, in mm.
const REGRESSION_HEIGHT_MM: f32 = 0.2;
/// Fixed nozzle diameter. Only the round-cross-section branch reads it, and the
/// properties deliberately keep `thick_bridges` false.
const NOZZLE_MM: f32 = 0.4;
/// Bridging flow ratio. Deliberately not 1.0, so a bridge point's `flow_factor`
/// is distinguishable from an untouched point's.
const BRIDGE_FLOW_RATIO: f32 = 0.85;
/// Region and point Z height, in mm.
const Z_MM: f32 = 0.2;

/// Lower end of the generated layer-height range, in slicer units.
const MIN_GENERATED_HEIGHT_UNITS: u32 = 1_000;

/// Junctions in one closed, explicitly repeated rectangle: four corners plus the
/// repeated first corner.
const CLOSED_RECT_JUNCTIONS: usize = 5;
/// The five-line roster's required junction count: three closed rectangles
/// (outer, inner, thin-wall) at [`CLOSED_RECT_JUNCTIONS`] each, plus the two
/// open four-junction chains.
const ROSTER_JUNCTIONS: usize = 3 * CLOSED_RECT_JUNCTIONS + 2 * 4;
/// The two-line roster's junction count: the closed outer and inner rectangles
/// only (the empty line contributes none).
const TWO_LINE_ROSTER_JUNCTIONS: usize = 2 * CLOSED_RECT_JUNCTIONS;

/// The generated roster, expressed as exact integer-unit fractions of the
/// sampled region half-extent. Every shape is either on the region boundary or
/// strictly inside it, and the fractions are strictly ordered, so every accepted
/// `max_abs_mm_units` yields geometrically valid geometry:
///
/// * outer closed rectangle: `±half_units` (the region boundary);
/// * closed inner rectangle: `±half_units / 2`;
/// * mixed bridge band: `x` in `[-3 * half_units / 8, +5 * half_units / 8]`,
///   `y` in `±5 * half_units / 8`; it contains the thin-wall rectangle and both
///   open chains whole, and covers the inner rectangle's two positive-x
///   corners but neither negative-x corner, so a closed line's bridge flags
///   are not all identical;
/// * both open chains: within `±half_units / 4`, inside the bridge band;
/// * thin-wall rectangle: `±half_units / 10`, inside the bridge band and clear
///   of the open chains.
///
/// No generated point has `y == 0`, so no point lies on the overhang band's
/// `y == 0` boundary (an extra outer point may have `x == 0`, which is an
/// interior coordinate of that band, not a boundary). Every roster coordinate
/// is strictly off the bridge band's edges, and every inner/outer rectangle
/// corner lies on the same side of the band that its own coordinates put it.
#[derive(Debug, Clone)]
struct RosterGeometry {
    outer: Vec<(i64, i64)>,
    inner: Vec<(i64, i64)>,
    inner_ccw: Vec<(i64, i64)>,
    open_ccw: Vec<(i64, i64)>,
    open_cw: Vec<(i64, i64)>,
    thin: Vec<(i64, i64)>,
}

fn roster_geometry(half_units: i64, extra_outer_junctions: usize) -> RosterGeometry {
    debug_assert!(half_units >= MIN_ABS_MM_UNITS as i64);
    let inner = half_units / 2;
    let open = half_units / 4;
    let half_open = open / 2;
    let thin = half_units / 10;
    let band_left = -3 * half_units / 8;
    let band_right = 5 * half_units / 8;
    let band_top = 5 * half_units / 8;
    // Strict ordering over the whole accepted half-extent domain: the thin
    // rectangle stays clear of the open chains, both stay inside the bridge
    // band, the band splits the inner rectangle (its left edge falls between
    // the negative and positive corners, its right edge and top lie beyond the
    // positive corners), and the inner rectangle stays inside the region.
    debug_assert!(0 < thin && thin < half_open && open < inner);
    debug_assert!(band_left < -open && band_right > open && band_top > open);
    debug_assert!(band_left < -thin && band_right > thin && band_top > thin);
    debug_assert!(-inner < band_left && band_right > inner && band_top > inner);
    debug_assert!(inner < half_units);

    let inner_rect = vec![
        (inner, inner),
        (inner, -inner),
        (-inner, -inner),
        (-inner, inner),
        (inner, inner),
    ];
    // The exact point-order reverse: the same closed, explicitly repeated
    // rectangle wound counter-clockwise. Under canonical
    // `ExtrusionLine::is_contour` a closed counter-clockwise line is a hole.
    let inner_ccw: Vec<(i64, i64)> = inner_rect.iter().rev().copied().collect();
    // Open inner-wall chain, wound counter-clockwise, with distinct endpoints.
    // It straddles `y == 0` without ever touching it.
    let open_ccw = vec![
        (-open, -half_open),
        (-half_open, -half_open),
        (-half_open, half_open),
        (-open, half_open),
    ];
    let open_cw: Vec<(i64, i64)> = open_ccw.iter().rev().copied().collect();
    // Closed, explicitly repeated small rectangle wound counter-clockwise. It
    // lies strictly inside the bridge band, so the `ThinWall` bridge exclusion
    // has a non-vacuous positive witness.
    let thin_rect = vec![
        (thin, thin),
        (-thin, thin),
        (-thin, -thin),
        (thin, -thin),
        (thin, thin),
    ];
    RosterGeometry {
        outer: outer_ring(half_units, extra_outer_junctions),
        inner: inner_rect,
        inner_ccw,
        open_ccw,
        open_cw,
        thin: thin_rect,
    }
}

/// Closed, explicitly repeated outer-wall rectangle on the region boundary,
/// wound clockwise, with `extra` additional collinear junctions spread over its
/// bottom and top edges. Subdividing two edges keeps the ring simple and
/// clockwise, makes it exactly `5 + extra` junctions long, and gives every
/// added point `y == ±half`, so none lands on the overhang band boundary. The
/// integer interpolation never reproduces an existing corner: the accepted
/// half-extent floor is 1 mm and the extra count is below the junction ceiling.
fn outer_ring(half: i64, extra: usize) -> Vec<(i64, i64)> {
    let bottom = extra / 2;
    let top = extra - bottom;
    let mut ring = Vec::with_capacity(CLOSED_RECT_JUNCTIONS + extra);
    ring.push((half, half));
    ring.push((half, -half));
    // Bottom edge, from `(+half, -half)` toward `(-half, -half)`.
    for index in 1..=bottom {
        let x = half - 2 * half * index as i64 / (bottom as i64 + 1);
        ring.push((x, -half));
    }
    ring.push((-half, -half));
    ring.push((-half, half));
    // Top edge, from `(-half, half)` toward `(+half, half)`.
    for index in 1..=top {
        let x = -half + 2 * half * index as i64 / (top as i64 + 1);
        ring.push((x, half));
    }
    ring.push((half, half));
    ring
}

/// An axis-aligned rectangle in canonical units, wound counter-clockwise.
fn rect_units(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: x0, y: y0 },
                Point2 { x: x1, y: y0 },
                Point2 { x: x1, y: y1 },
                Point2 { x: x0, y: y1 },
            ],
        },
        holes: Vec::new(),
    }
}

/// The region: the square `[-half_units, +half_units]^2` derived from the
/// scene's own sampled half-extent.
fn region_rect(half_units: i64) -> ExPolygon {
    rect_units(-half_units, -half_units, half_units, half_units)
}

/// The mixed bridge band: `x` from `-3 * half / 8` to `+5 * half / 8`, `y`
/// within `±5 * half / 8`. Deliberately asymmetric: it contains the thin-wall
/// rectangle and both open chains whole, but of the closed inner rectangle it
/// covers only the two positive-x corners. A closed line therefore carries both
/// `is_bridge` values, and reversing its points without also reversing its
/// flags is observable at an index whose coordinate says the other value.
fn bridge_band_rect(half_units: i64) -> ExPolygon {
    let left = -3 * half_units / 8;
    let right = 5 * half_units / 8;
    let vertical = 5 * half_units / 8;
    rect_units(left, -vertical, right, vertical)
}

/// The right half of the region. Used only by the deterministic
/// reverse-trigger regression: it contains the closed inner rectangle's two
/// positive-x corners and neither negative-x corner, so that fixture exercises
/// asymmetric bridge flags.
fn right_half_rect(half_units: i64) -> ExPolygon {
    rect_units(0, -half_units, half_units, half_units)
}

/// The previous-layer footprint: the region's upper half
/// `[-half, half] x [0, half]`. The lower half is unsupported, so every point
/// with `y < 0` lies outside this boundary at distance `-y`; no generated point
/// has `y == 0`, so the predicted quartile never depends on how the
/// point-in-polygon test resolves the shared edge.
fn previous_layer_rect(half_units: i64) -> ExPolygon {
    rect_units(-half_units, 0, half_units, half_units)
}

/// Exact canonical-unit -> millimetre conversion for generated coordinates. The
/// unit values used here round-trip back through `mm_to_units` unchanged.
fn mm(units: i64) -> f32 {
    units_to_mm(units)
}

/// Hands out a strictly increasing, therefore pairwise distinct, positive bead
/// spacing per junction.
struct Spacings {
    next_units: u32,
    step_units: u32,
}

impl Spacings {
    fn take(&mut self) -> f32 {
        let spacing = mm(i64::from(self.next_units));
        self.next_units = self.next_units.saturating_add(self.step_units);
        spacing
    }
}

fn make_line(
    coords: &[(i64, i64)],
    inset_idx: u32,
    is_odd: bool,
    is_closed: bool,
    spacings: &mut Spacings,
) -> ExtrusionLine {
    ExtrusionLine {
        junctions: coords
            .iter()
            .map(|(x, y)| ExtrusionJunction {
                p: point3(mm(*x), mm(*y), spacings.take()),
                perimeter_index: inset_idx,
            })
            .collect(),
        inset_idx,
        is_odd,
        is_closed,
    }
}

/// The smallest `max_junctions` bound that can admit the five-line roster; a
/// lower bound is rejected before sampling. A higher bound is sampled up to and
/// the difference becomes extra collinear junctions on the outer rectangle.
const MIN_JUNCTIONS: u64 = ROSTER_JUNCTIONS as u64;
/// Written ceiling for `max_junctions`.
const MAX_JUNCTIONS_CEILING: u64 = 64;
/// `max_abs_mm_units` is the absolute coordinate bound of the generated geometry
/// in slicer units (1 unit = 1e-4 mm). `MIN_ABS_MM_UNITS` is a 1 mm half-extent,
/// the smallest region the roster's fractions stay nondegenerate in; the
/// ceiling is 100 mm.
const MIN_ABS_MM_UNITS: u64 = 10_000;
const MAX_ABS_MM_UNITS_CEILING: u64 = 1_000_000;
/// `max_spacing_units` bounds one junction's bead spacing in slicer units.
const MIN_SPACING_UNITS: u64 = 1;
const MAX_SPACING_UNITS_CEILING: u64 = 100_000;
/// `max_height_units` bounds the layer height in slicer units.
const MIN_HEIGHT_UNITS: u64 = 1;
const MAX_HEIGHT_UNITS_CEILING: u64 = 100_000;
/// Bead-domain ceiling, in millimetres: one bead's spacing plus the layer height
/// must stay below this. Beyond it the derived width is no longer a physically
/// meaningful extrusion bead, and the f32 rounding envelope stops discriminating
/// a missed conversion from a correct one.
const WIDTH_DOMAIN_MM_CEILING: f64 = 10.0;

/// Checked generation bounds for `M-ARACHNE-BUILD`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BuildLimits {
    /// Maximum junctions across the whole generated roster.
    max_junctions: usize,
    /// Absolute coordinate bound, in slicer units (1 unit = 1e-4 mm).
    max_abs_mm_units: i64,
    /// Maximum per-junction bead spacing, in slicer units.
    max_spacing_units: u32,
    /// Maximum layer height, in slicer units.
    max_height_units: u32,
}

/// Validate every bound in `u64` before any narrowing, arithmetic or allocation.
/// Out-of-ceiling, zero, overflowing and witness-impossible profiles fail closed.
fn checked_build_limits(
    max_junctions: u64,
    max_abs_mm_units: u64,
    max_spacing_units: u64,
    max_height_units: u64,
) -> Result<BuildLimits, String> {
    if !(MIN_JUNCTIONS..=MAX_JUNCTIONS_CEILING).contains(&max_junctions) {
        return Err(format!(
            "max_junctions must be in {MIN_JUNCTIONS}..={MAX_JUNCTIONS_CEILING} (the five-line \
             roster needs {MIN_JUNCTIONS} junctions: three closed lines at \
             {CLOSED_RECT_JUNCTIONS} each plus two open chains at four each), got {max_junctions}"
        ));
    }
    if !(MIN_ABS_MM_UNITS..=MAX_ABS_MM_UNITS_CEILING).contains(&max_abs_mm_units) {
        return Err(format!(
            "max_abs_mm_units must be in {MIN_ABS_MM_UNITS}..={MAX_ABS_MM_UNITS_CEILING} \
             (a {MIN_ABS_MM_UNITS}-unit, 1 mm half-extent must fit the roster), \
             got {max_abs_mm_units}"
        ));
    }
    if !(MIN_SPACING_UNITS..=MAX_SPACING_UNITS_CEILING).contains(&max_spacing_units) {
        return Err(format!(
            "max_spacing_units must be in {MIN_SPACING_UNITS}..={MAX_SPACING_UNITS_CEILING}, \
             got {max_spacing_units}"
        ));
    }
    if !(MIN_HEIGHT_UNITS..=MAX_HEIGHT_UNITS_CEILING).contains(&max_height_units) {
        return Err(format!(
            "max_height_units must be in {MIN_HEIGHT_UNITS}..={MAX_HEIGHT_UNITS_CEILING}, \
             got {max_height_units}"
        ));
    }
    // The roster hands out one strictly increasing integer spacing per junction,
    // and every one must stay within the declared spacing bound. This holds for
    // every sampled total at or below `max_junctions` exactly when the bound
    // itself is at least one unit per junction.
    if max_spacing_units < max_junctions {
        return Err(format!(
            "max_spacing_units {max_spacing_units} is smaller than max_junctions \
             {max_junctions}: the roster needs one strictly increasing unit spacing per junction \
             inside the declared spacing bound"
        ));
    }
    // Bead/geometry domain: a bead and a layer must each fit twice inside the
    // declared coordinate box. Checked arithmetic only; the ceilings above
    // already bound both values far below u64::MAX.
    let doubled_spacing = max_spacing_units
        .checked_mul(2)
        .ok_or_else(|| String::from("max_spacing_units * 2 overflows u64"))?;
    if doubled_spacing > max_abs_mm_units {
        return Err(format!(
            "max_spacing_units {max_spacing_units} does not fit twice inside \
             max_abs_mm_units {max_abs_mm_units}"
        ));
    }
    let doubled_height = max_height_units
        .checked_mul(2)
        .ok_or_else(|| String::from("max_height_units * 2 overflows u64"))?;
    if doubled_height > max_abs_mm_units {
        return Err(format!(
            "max_height_units {max_height_units} does not fit twice inside \
             max_abs_mm_units {max_abs_mm_units}"
        ));
    }

    // Checked conversion precedes the arithmetic below.
    let max_junctions = usize::try_from(max_junctions)
        .map_err(|_| String::from("max_junctions does not fit usize"))?;
    let max_abs_mm_units = i64::try_from(max_abs_mm_units)
        .map_err(|_| String::from("max_abs_mm_units does not fit i64"))?;
    let max_spacing_units = u32::try_from(max_spacing_units)
        .map_err(|_| String::from("max_spacing_units does not fit u32"))?;
    let max_height_units = u32::try_from(max_height_units)
        .map_err(|_| String::from("max_height_units does not fit u32"))?;

    let widest_spacing_mm = f64::from(units_to_mm(i64::from(max_spacing_units)));
    let tallest_height_mm = f64::from(units_to_mm(i64::from(max_height_units)));
    if widest_spacing_mm + tallest_height_mm > WIDTH_DOMAIN_MM_CEILING {
        return Err(format!(
            "bead domain {widest_spacing_mm} + {tallest_height_mm} mm exceeds the \
             {WIDTH_DOMAIN_MM_CEILING} mm width-domain ceiling"
        ));
    }

    Ok(BuildLimits {
        max_junctions,
        max_abs_mm_units,
        max_spacing_units,
        max_height_units,
    })
}

/// The mode witness a shrink must never change: which branch of the contract
/// each property case is in. Derived from `no_shrink` draws only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BuildWitness {
    bridge_present: bool,
    thin_enabled: bool,
    contour_ccw: bool,
    g7_reverse: bool,
    /// The roster is the two-line region (outer + closed inner), so canonical
    /// `traverse_extrusions`' `pg_extrusions.size() == 2` term applies.
    two_line_region: bool,
    /// The closed inner rectangle is wound counter-clockwise (a hole under
    /// `ExtrusionLine::is_contour`) instead of clockwise (a contour).
    inner_ccw: bool,
}

/// One generated `build_walls` case.
#[derive(Debug, Clone)]
struct BuildScene {
    lines: Vec<ExtrusionLine>,
    bridge_areas: Vec<ExPolygon>,
    contour_should_be_ccw: bool,
    g7_reverse: bool,
    print_thin_walls: bool,
    layer_height_mm: f32,
    /// The sampled region half-extent, in slicer units. The region, every roster
    /// shape and the mixed bridge band are fractions of it.
    half_units: i64,
    /// The junction total this case asked for, in `MIN_JUNCTIONS..=max_junctions`.
    /// The comparator asserts the emitted roster matches it exactly.
    junction_total: usize,
    witness: BuildWitness,
}

/// One draw of the generated `M-ARACHNE-BUILD` domain.
///
/// `mode` is `no_shrink` because it selects the contract branch; the layer
/// height and bead-spacing base are shrinkable without changing that branch.
/// `half_units` and `junction_total` are the bound-driven draws: they come from
/// `max_abs_mm_units` and `max_junctions` respectively, not from fixed
/// constants, and shrinking them changes only geometry and roster length.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BuildDraw {
    mode: u32,
    height_units: u32,
    spacing_base_units: u32,
    half_units: i64,
    junction_total: usize,
}

/// The bead-spacing base range shared by the strategy and the deterministic
/// controls: `Spacings` hands out one strictly increasing integer unit spacing
/// per junction, and the widest base still leaves room for every junction the
/// bound can generate inside `max_spacing_units`.
fn spacing_base_band(limits: &BuildLimits) -> (u32, u32) {
    let headroom = limits.max_junctions as u32 + 1;
    let step = (limits.max_spacing_units / (2 * headroom)).max(1);
    let widest = limits
        .max_spacing_units
        .saturating_sub(step.saturating_mul(headroom))
        .max(step);
    (step, widest)
}

/// Build the roster. Its structure is derived entirely from the `no_shrink` mode
/// word, while the region half-extent and the junction total are derived from
/// the accepted bounds: every shape is a fraction of the sampled half-extent
/// (see [`roster_geometry`]), and the shortfall between the mode's roster count
/// and the sampled total becomes extra collinear junctions on the outer
/// rectangle, so the emitted total is exactly the sampled one.
fn build_scene(draw: BuildDraw, limits: &BuildLimits) -> BuildScene {
    let BuildDraw {
        mode,
        height_units,
        spacing_base_units,
        half_units,
        junction_total,
    } = draw;
    let contour_should_be_ccw = mode & 1 != 0;
    let g7_reverse = mode & 2 != 0;
    let print_thin_walls = mode & 4 != 0;
    let bridge_present = mode & 8 != 0;
    let two_line_region = mode & 16 != 0;
    let inner_ccw = mode & 32 != 0;

    let (step_units, widest_base_units) = spacing_base_band(limits);
    let mut spacings = Spacings {
        next_units: spacing_base_units.min(widest_base_units),
        step_units,
    };

    let roster_base = if two_line_region {
        TWO_LINE_ROSTER_JUNCTIONS
    } else {
        ROSTER_JUNCTIONS
    };
    let geometry = roster_geometry(half_units, junction_total.saturating_sub(roster_base));
    let inner_rect: &[(i64, i64)] = if inner_ccw {
        &geometry.inner_ccw
    } else {
        &geometry.inner
    };
    let mut lines = vec![
        // Explicitly closed outer wall on the region boundary, clockwise, with
        // the bound-derived extra junctions on its horizontal edges.
        make_line(&geometry.outer, 0, false, true, &mut spacings),
        // Explicitly closed inner wall, clockwise or counter-clockwise per the
        // mode word.
        make_line(inner_rect, 1, false, true, &mut spacings),
    ];
    if !two_line_region {
        lines.extend([
            // Open inner chain, counter-clockwise, distinct endpoints, strictly
            // inside the bridge band.
            make_line(&geometry.open_ccw, 2, false, false, &mut spacings),
            // Open inner chain, the point-order reverse of the previous one.
            make_line(&geometry.open_cw, 3, false, false, &mut spacings),
            // Odd inset-0 line: `ThinWall` when thin walls are enabled,
            // otherwise an ordinary `OuterWall`. It lies inside the bridge band,
            // so the ThinWall bridge exclusion has a non-vacuous witness.
            make_line(&geometry.thin, 0, true, true, &mut spacings),
        ]);
    }
    // Empty line: a legitimate Arachne input that must produce no wall and does
    // not count toward the region's line total.
    lines.push(ExtrusionLine {
        junctions: Vec::new(),
        inset_idx: 4,
        is_odd: false,
        is_closed: false,
    });

    BuildScene {
        lines,
        bridge_areas: if bridge_present {
            vec![bridge_band_rect(half_units)]
        } else {
            Vec::new()
        },
        contour_should_be_ccw,
        g7_reverse,
        print_thin_walls,
        layer_height_mm: mm(i64::from(height_units.min(limits.max_height_units))),
        half_units,
        junction_total,
        witness: BuildWitness {
            bridge_present,
            thin_enabled: print_thin_walls,
            contour_ccw: contour_should_be_ccw,
            g7_reverse,
            two_line_region,
            inner_ccw,
        },
    }
}

fn build_strategy(limits: &BuildLimits) -> BoxedStrategy<BuildScene> {
    let limits = *limits;
    let (step_units, widest_base) = spacing_base_band(&limits);
    let lowest_height = MIN_GENERATED_HEIGHT_UNITS.min(limits.max_height_units);
    (
        any::<u32>().no_shrink(),
        lowest_height..=limits.max_height_units,
        step_units..=widest_base,
        // The region half-extent comes from the coordinate bound instead of a
        // fixed constant, so every accepted profile samples its own coordinate
        // domain: 1 mm up to `max_abs_mm_units`.
        (MIN_ABS_MM_UNITS as i64)..=limits.max_abs_mm_units,
        // The roster's total junction count likewise comes from the bound: the
        // smallest total that admits the five-line roster up to `max_junctions`.
        (MIN_JUNCTIONS as usize)..=limits.max_junctions,
    )
        .prop_map(
            move |(mode, height_units, spacing_base_units, half_units, junction_total)| {
                build_scene(
                    BuildDraw {
                        mode,
                        height_units,
                        spacing_base_units,
                        half_units,
                        junction_total,
                    },
                    &limits,
                )
            },
        )
        .boxed()
}

fn region_for(scene: &BuildScene) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-arachne-property")
        .region_id(1)
        .z(Z_MM)
        .add_polygon(region_rect(scene.half_units))
        .bridge_areas(scene.bridge_areas.clone())
        .previous_layer_boundary(vec![previous_layer_rect(scene.half_units)])
        .overhang_band_width_mm(OVERHANG_BAND_WIDTH_MM)
        .build()
}

fn build_context(region: &SliceRegionView) -> PerimeterSpatialContext {
    PerimeterSpatialContext::new(
        region.prev_layer_boundary(),
        region.overhang_band_width_mm(),
        region.bridge_areas(),
    )
}

fn build_params(scene: &BuildScene) -> ArachneParams {
    ArachneParams {
        layer_height: f64::from(scene.layer_height_mm),
        print_thin_walls: scene.print_thin_walls,
        outer_wall_offset: 0.0,
        ..ArachneParams::default()
    }
}

/// Run the real kernel exactly the way `run_perimeters` does for one region.
fn run_build_walls(scene: &BuildScene, region: &SliceRegionView) -> Result<Vec<WallLoop>, String> {
    let context = build_context(region);
    let params = build_params(scene);
    let module = ArachnePerimeters;
    module
        .build_walls(
            &scene.lines,
            region,
            region.z(),
            region.polygons(),
            &params,
            scene.contour_should_be_ccw,
            scene.g7_reverse,
            scene.layer_height_mm,
            NOZZLE_MM,
            BRIDGE_FLOW_RATIO,
            false,
            &context,
            true,
        )
        .map_err(|error| error.to_string())
}

/// The winding a CLOSED emitted loop must end up with, restated from canonical
/// `traverse_extrusions` (`PerimeterGenerator.cpp`):
///
/// ```text
/// if ((wall_direction == CounterClockwise) ==
///     (pg_extrusion.is_contour || pg_extrusions.size() == 2))
///     make_counter_clockwise(); else make_clockwise();
/// ```
///
/// where `is_contour` is `ExtrusionLine::is_contour` (`ExtrusionLine.cpp`):
/// `is_closed && polygon(junctions).is_clockwise()`. Contour/hole therefore
/// comes from the line's own input winding, never from `inset_idx`, and
/// `region_line_count` is the region's nonempty line total. `g7_reverse`
/// (`overhang_reverse` on the layers it applies to) flips the result. Open
/// lines are never reoriented by canonical, so callers consult this only for
/// closed lines. This is an independent statement, not the production
/// expression.
fn expected_want_ccw(
    is_closed: bool,
    input_clockwise: bool,
    region_line_count: usize,
    contour_should_be_ccw: bool,
    g7_reverse: bool,
) -> bool {
    let is_contour = is_closed && input_clockwise;
    let base_want_ccw = contour_should_be_ccw == (is_contour || region_line_count == 2);
    base_want_ccw != g7_reverse
}

/// Boundary type when the region carries no paint annotations: the documented
/// `build_wall_flags` fallback keys off `is_outer` alone. This is the PnP paint
/// classification only; it plays no part in the winding model.
fn expected_boundary_type(inset_idx: u32) -> WallBoundaryType {
    if inset_idx == 0 {
        WallBoundaryType::ExteriorSurface
    } else {
        WallBoundaryType::Interior
    }
}

/// Independent shoelace signed area over the input points, in f64. Only the sign
/// is used.
fn signed_area(points: &[(f64, f64)]) -> f64 {
    let mut area = 0.0;
    for index in 0..points.len() {
        let (ax, ay) = points[index];
        let (bx, by) = points[(index + 1) % points.len()];
        area += ax * by - bx * ay;
    }
    area / 2.0
}

fn points_as_f64(points: &[Point3WithWidth]) -> Vec<(f64, f64)> {
    points
        .iter()
        .map(|point| (f64::from(point.x), f64::from(point.y)))
        .collect()
}

/// `spacing + height * (1 - pi/4)`, the documented canonical spacing-to-width
/// conversion, evaluated in f64 and deliberately not through `flow_to_width`.
fn expected_width_mm(spacing_mm: f32, layer_height_mm: f32) -> f64 {
    f64::from(spacing_mm) + f64::from(layer_height_mm) * (1.0 - std::f64::consts::FRAC_PI_4)
}

/// Absolute f32 rounding envelope for [`expected_width_mm`], derived from the
/// registry bounds.
///
/// Production evaluates, in f32: `q_hat = 1 - PI_f32/4`, then `p_hat = h * q_hat`,
/// then `w_hat = s + p_hat`. With `q = 1 - pi/4`:
///
/// * `|q_hat - q| <= 2^-24` — the `PI_f32` half-ulp, an exact division by four,
///   and a half-ulp of the result near 0.2146 all sum below 2^-24;
/// * `|fl(h * q_hat) - h*q| <= h*q_hat*2^-24 + h*|q_hat - q| <= h_max*(q_ub + 1)*2^-24`;
/// * `|fl(s + p_hat) - (s + h*q)| <= (s + h*q)*2^-24 + the line above`.
///
/// Summed: `E = (s_max + (1 + 2*q_ub) * h_max) * 2^-24`.
fn width_envelope(limits: &BuildLimits) -> f64 {
    /// Upper bound for `1 - pi/4 = 0.21460183660255172`.
    const Q_UB: f64 = 0.214_601_84;
    let spacing_max = f64::from(units_to_mm(i64::from(limits.max_spacing_units)));
    let height_max = f64::from(units_to_mm(i64::from(limits.max_height_units)));
    (spacing_max + (1.0 + 2.0 * Q_UB) * height_max) * (1.0 / 16_777_216.0)
}

/// Whether `(x_mm, y_mm)` is inside the mixed bridge area, predicted
/// independently by the shared exact region oracle. Never consults
/// `PerimeterSpatialContext::is_bridge` or `build_wall_flags`.
fn predicted_bridge_membership(
    x_mm: f32,
    y_mm: f32,
    bridge_areas: &[ExPolygon],
) -> Result<bool, String> {
    if bridge_areas.is_empty() {
        return Ok(false);
    }
    let units = Point2 {
        x: mm_to_units(x_mm),
        y: mm_to_units(y_mm),
    };
    match point_in_region(units, bridge_areas) {
        Ok(PointLocation::Inside) => Ok(true),
        Ok(PointLocation::Outside) => Ok(false),
        Ok(PointLocation::Boundary) => Err(format!(
            "generated point ({x_mm}, {y_mm}) lies on a bridge boundary; the fixture must keep \
             every point strictly off it"
        )),
        Err(error) => Err(format!("point_in_region rejected the query: {error}")),
    }
}

/// Independent signed distance (mm) from a generated point to
/// [`previous_layer_rect`]. Points below `y = 0` are outside the previous
/// boundary and report `-y`; points above are inside and report the negative
/// distance to the nearest edge of that upper-half rectangle (either side, the
/// top, or the bottom edge at `y = 0`).
fn expected_signed_distance_mm(x_mm: f32, y_mm: f32, half_units: i64) -> f64 {
    let half_mm = f64::from(units_to_mm(half_units));
    let x = f64::from(x_mm);
    let y = f64::from(y_mm);
    if y <= 0.0 {
        -y
    } else {
        -(half_mm - x.abs()).min(y).min(half_mm - y)
    }
}

/// Independent distance-band oracle for the consumer-side quartile.
///
/// The documented thresholds are `0.5 / 1.0 / 1.5 x band_width`; band 4 is the
/// remainder (never capped). A point on the boundary within
/// [`OVERHANG_BOUNDARY_EPSILON_MM`] is not overhanging. The bucket arithmetic is
/// `f32`, matching the consumer's distance type, so a distance that lands
/// exactly on a threshold buckets the same way.
fn expected_overhang_quartile(raw_signed_distance_mm: f64) -> Option<u8> {
    let raw = raw_signed_distance_mm as f32;
    if raw <= OVERHANG_BOUNDARY_EPSILON_MM {
        return None;
    }
    let width = OVERHANG_BAND_WIDTH_MM;
    if raw <= width * 0.5 {
        Some(1)
    } else if raw <= width {
        Some(2)
    } else if raw <= width * 1.5 {
        Some(3)
    } else {
        Some(4)
    }
}

/// Verify every nonempty input line against an independent model of the
/// conversion, confirm the empty line produced nothing, and confirm the
/// bound-driven parts of the draw actually reached the roster.
fn verify_build_walls(
    scene: &BuildScene,
    limits: &BuildLimits,
    walls: &[WallLoop],
) -> Result<(), String> {
    let nonempty: Vec<&ExtrusionLine> = scene
        .lines
        .iter()
        .filter(|line| !line.junctions.is_empty())
        .collect();
    if walls.len() != nonempty.len() {
        return Err(format!(
            "expected one wall per nonempty input line ({} walls), observed {}",
            nonempty.len(),
            walls.len()
        ));
    }

    // Bound-driven roster length: the observed input total must equal the
    // sampled total and stay inside the declared junction bound.
    let input_total: usize = scene.lines.iter().map(|line| line.junctions.len()).sum();
    if input_total != scene.junction_total {
        return Err(format!(
            "the generated roster carries {input_total} junctions but the draw asked for {}",
            scene.junction_total
        ));
    }
    if input_total > limits.max_junctions {
        return Err(format!(
            "the generated roster carries {input_total} junctions, above the max_junctions \
             bound {}",
            limits.max_junctions
        ));
    }

    // Bound-driven coordinate extent: the outer rectangle must reach exactly
    // ±half_units and nothing may exceed it.
    let observed_half = scene
        .lines
        .iter()
        .flat_map(|line| line.junctions.iter())
        .map(|junction| {
            mm_to_units(junction.p.x)
                .abs()
                .max(mm_to_units(junction.p.y).abs())
        })
        .max()
        .unwrap_or(0);
    if observed_half != scene.half_units {
        return Err(format!(
            "the generated geometry reaches {observed_half} units but the draw sampled a \
             {} unit half-extent",
            scene.half_units
        ));
    }

    let envelope = width_envelope(limits);
    let mut saw_bridge_true = false;
    let mut saw_bridge_false = false;

    for (line, wall) in nonempty.iter().zip(walls) {
        let label = format!(
            "inset_idx={} is_odd={} is_closed={}",
            line.inset_idx, line.is_odd, line.is_closed
        );

        // Fixture self-check: closure is the explicit-repeat convention, never
        // synthesized from `is_odd`/`inset_idx`.
        let repeated = line.junctions.len() >= 2
            && line.junctions.first().map(|j| (j.p.x, j.p.y))
                == line.junctions.last().map(|j| (j.p.x, j.p.y));
        if repeated != line.is_closed {
            return Err(format!(
                "{label}: fixture declares is_closed={} but its endpoints are {}",
                line.is_closed,
                if repeated { "coincident" } else { "distinct" }
            ));
        }

        let (role, loop_type) =
            expected_classification(line.is_odd, line.inset_idx, scene.print_thin_walls);
        if wall.perimeter_index != line.inset_idx {
            return Err(format!(
                "{label}: perimeter_index {} does not preserve inset_idx",
                wall.perimeter_index
            ));
        }
        if wall.loop_type != loop_type || wall.path.role != role {
            return Err(format!(
                "{label}: expected ({role:?}, {loop_type:?}), observed ({:?}, {:?})",
                wall.path.role, wall.loop_type
            ));
        }
        if wall.boundary_type != expected_boundary_type(line.inset_idx) {
            return Err(format!(
                "{label}: expected boundary type {:?}, observed {:?}",
                expected_boundary_type(line.inset_idx),
                wall.boundary_type
            ));
        }
        if wall.path.is_closed() != line.is_closed {
            return Err(format!(
                "{label}: output path closure {} disagrees with the input's {}",
                wall.path.is_closed(),
                line.is_closed
            ));
        }

        let count = line.junctions.len();
        if wall.path.points.len() != count
            || wall.width_profile.widths.len() != count
            || wall.feature_flags.len() != count
        {
            return Err(format!(
                "{label}: {count} input junctions but points={} widths={} flags={}",
                wall.path.points.len(),
                wall.width_profile.widths.len(),
                wall.feature_flags.len()
            ));
        }

        // Output order, from the canonical winding contract: an open line keeps
        // its input order; a closed line is reversed iff its input winding
        // disagrees with the canonical target.
        let input_area = signed_area(
            &line
                .junctions
                .iter()
                .map(|junction| (f64::from(junction.p.x), f64::from(junction.p.y)))
                .collect::<Vec<_>>(),
        );
        let input_ccw = input_area > 0.0;
        let want_ccw = expected_want_ccw(
            line.is_closed,
            input_area < 0.0,
            nonempty.len(),
            scene.contour_should_be_ccw,
            scene.g7_reverse,
        );
        let reversed = line.is_closed && count >= 3 && input_ccw != want_ccw;

        for index in 0..count {
            let source_index = if reversed { count - 1 - index } else { index };
            let source = &line.junctions[source_index];
            let point = &wall.path.points[index];
            let where_ = format!("{label} point {index} (source junction {source_index})");

            if point.x != source.p.x || point.y != source.p.y {
                return Err(format!(
                    "{where_}: expected ({}, {}), observed ({}, {}) — the point-to-junction \
                     association moved",
                    source.p.x, source.p.y, point.x, point.y
                ));
            }
            if point.z != Z_MM {
                return Err(format!("{where_}: expected z {Z_MM}, observed {}", point.z));
            }

            let expected_width = expected_width_mm(source.p.width, scene.layer_height_mm);
            let observed_width = f64::from(point.width);
            if (observed_width - expected_width).abs() > envelope {
                return Err(format!(
                    "{where_}: spacing {} mm at layer height {} mm must convert to \
                     {expected_width} mm (envelope {envelope}), observed {observed_width} mm",
                    source.p.width, scene.layer_height_mm
                ));
            }
            if wall.width_profile.widths[index] != point.width {
                return Err(format!(
                    "{where_}: width_profile.widths[{index}] = {} does not equal \
                     path.points[{index}].width = {}",
                    wall.width_profile.widths[index], point.width
                ));
            }

            let expected_bridge = matches!(loop_type, LoopType::Outer | LoopType::Inner)
                && predicted_bridge_membership(point.x, point.y, &scene.bridge_areas)?;
            if wall.feature_flags[index].is_bridge != expected_bridge {
                return Err(format!(
                    "{where_}: is_bridge = {}, but this point's own coordinates say {}",
                    wall.feature_flags[index].is_bridge, expected_bridge
                ));
            }
            if expected_bridge {
                saw_bridge_true = true;
            } else {
                saw_bridge_false = true;
            }

            let expected_flow = if expected_bridge {
                BRIDGE_FLOW_RATIO
            } else {
                1.0
            };
            if point.flow_factor != expected_flow {
                return Err(format!(
                    "{where_}: flow_factor {} does not match the documented bridging-flow result \
                     {expected_flow} for this point's own bridge state",
                    point.flow_factor
                ));
            }

            let is_thin = loop_type == LoopType::ThinWall;
            if wall.feature_flags[index].is_thin_wall != is_thin {
                return Err(format!(
                    "{where_}: is_thin_wall = {}, expected {is_thin} for {loop_type:?}",
                    wall.feature_flags[index].is_thin_wall
                ));
            }
            if wall.feature_flags[index].tool_index.is_some()
                || wall.feature_flags[index].fuzzy_skin
            {
                return Err(format!(
                    "{where_}: an annotation-free region must not set paint flags"
                ));
            }

            // Per-point overhang metadata authored from the selected region.
            let raw_distance = expected_signed_distance_mm(point.x, point.y, scene.half_units);
            let expected_quartile = expected_overhang_quartile(raw_distance);
            if point.overhang_quartile != expected_quartile {
                return Err(format!(
                    "{where_}: overhang_quartile {:?} disagrees with the distance-band model \
                     ({expected_quartile:?}) for raw signed distance {raw_distance}",
                    point.overhang_quartile
                ));
            }
            let Some(distance) = point.overhang_distance_mm else {
                return Err(format!(
                    "{where_}: a nonempty previous-layer boundary must stamp a distance"
                ));
            };
            let expected_distance = raw_distance + 0.5 * f64::from(point.width);
            if (f64::from(distance) - expected_distance).abs() > 1.0e-4 {
                return Err(format!(
                    "{where_}: overhang distance {distance} does not equal the analytical \
                     {expected_distance} for this point (x={}, y={}, half_units={}, width={}, \
                     raw={raw_distance})",
                    point.x, point.y, scene.half_units, point.width
                ));
            }
        }
    }

    // Total output points equal the roster's junction total: no junction is
    // dropped or duplicated across the batch.
    let output_total: usize = walls.iter().map(|wall| wall.path.points.len()).sum();
    if output_total != input_total {
        return Err(format!(
            "the batch emitted {output_total} points for {input_total} input junctions"
        ));
    }

    // Bridge population, predicted from the roster's own coordinates before any
    // flag is read. A non-empty band must produce both populations exactly when
    // it contains an Outer/Inner roster junction. The asymmetric band splits the
    // closed inner rectangle, so both the five-line and the two-line rosters
    // contain such junctions; only a band that misses every Outer/Inner point
    // may leave all flags false.
    let mut expected_positive = false;
    for line in &nonempty {
        let (_, loop_type) =
            expected_classification(line.is_odd, line.inset_idx, scene.print_thin_walls);
        if !matches!(loop_type, LoopType::Outer | LoopType::Inner) {
            continue;
        }
        for junction in &line.junctions {
            if predicted_bridge_membership(junction.p.x, junction.p.y, &scene.bridge_areas)? {
                expected_positive = true;
            }
        }
    }
    if !expected_positive {
        if saw_bridge_true {
            return Err(String::from(
                "no Outer/Inner roster junction lies inside a bridge area, so every is_bridge \
                 flag must be false",
            ));
        }
    } else if !(saw_bridge_true && saw_bridge_false) {
        return Err(format!(
            "the bridge band must produce both populations, observed positive={} negative={}",
            saw_bridge_true, saw_bridge_false
        ));
    }

    Ok(())
}

fn check_build_scene(scene: &BuildScene, limits: &BuildLimits) -> TestCaseResult {
    let region = region_for(scene);
    let walls = match run_build_walls(scene, &region) {
        Ok(walls) => walls,
        Err(error) => return fail(format!("build_walls rejected a valid scene: {error}")),
    };
    match verify_build_walls(scene, limits, &walls) {
        Ok(()) => Ok(()),
        Err(message) => fail(message),
    }
}

#[test]
fn proptest_module_arachne_build_walls() {
    let run = PropertyRun::new(BUILD_PROPERTY_ID, BUILD_TEST_NAME)
        .expect("M-ARACHNE-BUILD registry context");
    let limits = checked_build_limits(
        run.bound("max_junctions").expect("max_junctions bound"),
        run.bound("max_abs_mm_units")
            .expect("max_abs_mm_units bound"),
        run.bound("max_spacing_units")
            .expect("max_spacing_units bound"),
        run.bound("max_height_units")
            .expect("max_height_units bound"),
    )
    .expect("M-ARACHNE-BUILD safety ceilings");
    // Deterministic prelude: the canonical-winding witnesses and the
    // bounds-reach-generation control run before the generated campaign, each
    // under its own fresh `CaseGuard`.
    build_winding_witnesses();
    build_bounds_drive_geometry_and_junctions();
    let strategy = build_strategy(&limits);
    run.check(&strategy, |scene, _guard| {
        check_build_scene(&scene, &limits)
    })
    .expect("M-ARACHNE-BUILD wall conversion contract");
}

// ─────────────────────────────────────────────────────────────────────────────
// M-ARACHNE-SEQUENCE — region-local, lossless commitment.
// ─────────────────────────────────────────────────────────────────────────────

/// Written ceiling for `max_walls`. `MIN_WALLS` is the smallest bound that still
/// admits the sandwich witness (`InnerOuterInner` with three walls).
const MIN_WALLS: u64 = 3;
const MAX_WALLS_CEILING: u64 = 16;

fn checked_sequence_limits(max_walls: u64) -> Result<usize, String> {
    if !(MIN_WALLS..=MAX_WALLS_CEILING).contains(&max_walls) {
        return Err(format!(
            "max_walls must be in {MIN_WALLS}..={MAX_WALLS_CEILING} (the sandwich charter needs \
             {MIN_WALLS} walls), got {max_walls}"
        ));
    }
    usize::try_from(max_walls).map_err(|_| String::from("max_walls does not fit usize"))
}

/// One generated commitment case.
#[derive(Debug, Clone)]
struct SequenceCase {
    mode: WallSequence,
    walls: Vec<WallLoop>,
    /// `Some(order)` in the controlled single-outer domain; `None` when the
    /// batch has no unique endpoint outer, where only losslessness is claimed.
    expected: Option<Vec<usize>>,
}

fn sequence_mode(index: u8) -> WallSequence {
    match index % 3 {
        0 => WallSequence::InnerOuter,
        1 => WallSequence::OuterInner,
        _ => WallSequence::InnerOuterInner,
    }
}

/// Independent mode table for the controlled single-outer batch.
fn expected_sequence_order(loops: &[LoopType], mode: WallSequence) -> Vec<usize> {
    let count = loops.len();
    let mut order: Vec<usize> = (0..count).collect();
    if count <= 1 {
        return order;
    }
    let outer_at_front = loops[0] == LoopType::Outer;
    let outer_at_back = loops[count - 1] == LoopType::Outer;
    match mode {
        WallSequence::InnerOuter => {
            if outer_at_back && !outer_at_front {
                order.reverse();
            }
        }
        WallSequence::OuterInner => {
            if outer_at_front && !outer_at_back {
                order.reverse();
            }
        }
        WallSequence::InnerOuterInner => {
            // Normalize to outer-first, then sandwich: first inner, outer, the
            // remaining inners.
            if outer_at_back && !outer_at_front {
                order.reverse();
            }
            if count == 2 {
                order.swap(0, 1);
            } else {
                let mut sandwich = Vec::with_capacity(count);
                sandwich.push(order[1]);
                sandwich.push(order[0]);
                sandwich.extend_from_slice(&order[2..]);
                order = sandwich;
            }
        }
    }
    order
}

fn sequence_wall(index: usize, loop_type: LoopType) -> WallLoop {
    let base = index as f32;
    // FRU from the shared fixture base: the identity lives in
    // perimeter_index/coordinates/widths/flags, so a permutation that drops any
    // one of them is visible.
    let mut wall = wall_loop_base(loop_type, WallBoundaryType::Interior);
    wall.perimeter_index = index as u32;
    wall.path.points = vec![
        point3(base, 0.0, 0.4 + base * 0.01),
        point3(base, 1.0, 0.5 + base * 0.01),
        point3(base + 0.5, 1.5, 0.6 + base * 0.01),
    ];
    wall.width_profile = WidthProfile {
        widths: vec![0.4 + base * 0.01, 0.5 + base * 0.01, 0.6 + base * 0.01],
    };
    wall.feature_flags = (0..3)
        .map(|point| WallFeatureFlags {
            tool_index: Some(index as u32),
            is_bridge: index % 2 == 1 && point == 1,
            ..WallFeatureFlags::default()
        })
        .collect();
    wall
}

fn build_sequence_case(count: usize, mode_word: u32, mode_index: u8) -> SequenceCase {
    let mode = sequence_mode(mode_index);
    let kind = mode_word % 4;
    let mut walls = Vec::with_capacity(count);
    for index in 0..count {
        let loop_type = match kind {
            // Exactly one outer wall at the front.
            0 => {
                if index == 0 {
                    LoopType::Outer
                } else {
                    LoopType::Inner
                }
            }
            // Exactly one outer wall at the back.
            1 => {
                if index + 1 == count {
                    LoopType::Outer
                } else {
                    LoopType::Inner
                }
            }
            // An outer wall at both endpoints: no unique endpoint outer.
            2 => {
                if index == 0 || index + 1 == count {
                    LoopType::Outer
                } else {
                    LoopType::Inner
                }
            }
            // No outer wall at all.
            _ => LoopType::Inner,
        };
        walls.push(sequence_wall(index, loop_type));
    }

    let loops: Vec<LoopType> = walls.iter().map(|wall| wall.loop_type).collect();
    let unique_endpoint_outer = count > 1
        && matches!(kind, 0 | 1)
        && loops
            .iter()
            .filter(|loop_type| **loop_type == LoopType::Outer)
            .count()
            == 1;
    let expected = if unique_endpoint_outer {
        Some(expected_sequence_order(&loops, mode))
    } else {
        None
    };

    SequenceCase {
        mode,
        walls,
        expected,
    }
}

fn sequence_strategy(max_walls: usize) -> BoxedStrategy<SequenceCase> {
    (
        // Shrinking the wall count is safe: the expected order is re-derived
        // from the shrunk batch itself.
        0usize..=max_walls,
        any::<u32>().no_shrink(),
        0u8..=2,
    )
        .prop_map(move |(count, mode_word, mode_index)| {
            build_sequence_case(count, mode_word, mode_index)
        })
        .boxed()
}

/// Compare the committed batch against the independently derived order, or —
/// when no unique endpoint outer exists — against a lossless permutation.
fn verify_sequence(case: &SequenceCase, observed: &[WallLoop]) -> Result<(), String> {
    if observed.len() != case.walls.len() {
        return Err(format!(
            "expected {} committed walls, observed {}",
            case.walls.len(),
            observed.len()
        ));
    }
    match &case.expected {
        Some(order) => {
            if order.len() != case.walls.len() {
                return Err(String::from("the independent order is not a permutation"));
            }
            for (slot, source) in order.iter().enumerate() {
                if observed[slot] != case.walls[*source] {
                    return Err(format!(
                        "{mode:?} slot {slot}: expected the wall at input index {source} \
                         (perimeter_index {expected_index}), observed perimeter_index \
                         {observed_index}",
                        mode = case.mode,
                        expected_index = case.walls[*source].perimeter_index,
                        observed_index = observed[slot].perimeter_index,
                    ));
                }
            }
        }
        None => {
            let mut used = vec![false; case.walls.len()];
            for (slot, wall) in observed.iter().enumerate() {
                let found = case
                    .walls
                    .iter()
                    .enumerate()
                    .position(|(index, candidate)| !used[index] && candidate == wall);
                match found {
                    Some(index) => used[index] = true,
                    None => {
                        return Err(format!(
                            "{:?} slot {slot}: the committed wall is not an unused input wall",
                            case.mode
                        ))
                    }
                }
            }
            if used.iter().any(|used| !used) {
                return Err(format!(
                    "{:?}: the committed batch lost input walls",
                    case.mode
                ));
            }
        }
    }
    Ok(())
}

fn check_sequence_case(case: &SequenceCase) -> TestCaseResult {
    let mut walls = case.walls.clone();
    commit_wall_sequence(&mut walls, case.mode);
    match verify_sequence(case, &walls) {
        Ok(()) => Ok(()),
        Err(message) => fail(message),
    }
}

#[test]
fn proptest_module_arachne_sequence() {
    let run = PropertyRun::new(SEQUENCE_PROPERTY_ID, SEQUENCE_TEST_NAME)
        .expect("M-ARACHNE-SEQUENCE registry context");
    let max_walls = checked_sequence_limits(run.bound("max_walls").expect("max_walls bound"))
        .expect("M-ARACHNE-SEQUENCE safety ceilings");
    let strategy = sequence_strategy(max_walls);
    run.check(&strategy, |case, _guard| check_sequence_case(&case))
        .expect("M-ARACHNE-SEQUENCE commitment contract");
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic winding witnesses: canonical `traverse_extrusions` orientation.
// ─────────────────────────────────────────────────────────────────────────────
//
// Canonical `traverse_extrusions` (`PerimeterGenerator.cpp`) reorients only
// closed lines, toward `(wall_direction == CCW) == (is_contour ||
// pg_extrusions.size() == 2)`, with `is_contour` from `ExtrusionLine::is_contour`
// (`ExtrusionLine.cpp`): closed and clockwise. Each witness below is a case the
// former inset-based rule (inset 0 = contour, inset > 0 = hole, open lines
// reoriented too) got wrong by reversing a line canonical leaves alone.

/// Fixed, strictly increasing witness spacings (0.40 mm upward, 0.01 mm steps).
fn witness_spacings() -> Spacings {
    Spacings {
        next_units: 4_000,
        step_units: 100,
    }
}

/// The fixed witness roster: the same fractional shapes the generated property
/// uses, evaluated at [`WITNESS_HALF_UNITS`] with no extra outer junctions.
fn witness_geometry() -> RosterGeometry {
    roster_geometry(WITNESS_HALF_UNITS, 0)
}

/// A bridge-free, thin-wall-off, `g7_reverse`-off scene over `lines`.
fn winding_scene(
    lines: Vec<ExtrusionLine>,
    contour_should_be_ccw: bool,
    inner_ccw: bool,
) -> BuildScene {
    let two_line_region = lines
        .iter()
        .filter(|line| !line.junctions.is_empty())
        .count()
        == 2;
    let junction_total = lines.iter().map(|line| line.junctions.len()).sum();
    BuildScene {
        lines,
        bridge_areas: Vec::new(),
        contour_should_be_ccw,
        g7_reverse: false,
        print_thin_walls: false,
        layer_height_mm: REGRESSION_HEIGHT_MM,
        half_units: WITNESS_HALF_UNITS,
        junction_total,
        witness: BuildWitness {
            bridge_present: false,
            thin_enabled: false,
            contour_ccw: contour_should_be_ccw,
            g7_reverse: false,
            two_line_region,
            inner_ccw,
        },
    }
}

/// Run `build_walls` on `scene`, assert the `inset_idx` wall kept its input
/// point order exactly, then run the full comparator. Returns the emitted
/// wall's signed area.
fn expect_input_order_kept(scene: &BuildScene, inset_idx: u32, witness: &str) -> f64 {
    let region = region_for(scene);
    let walls = run_build_walls(scene, &region)
        .unwrap_or_else(|error| panic!("{witness}: build_walls rejected the scene: {error}"));
    let line = scene
        .lines
        .iter()
        .find(|line| line.inset_idx == inset_idx)
        .unwrap_or_else(|| panic!("{witness}: fixture has no inset {inset_idx} line"));
    let wall = walls
        .iter()
        .find(|wall| wall.perimeter_index == inset_idx)
        .unwrap_or_else(|| panic!("{witness}: no wall for inset {inset_idx}"));
    let input: Vec<(f32, f32)> = line.junctions.iter().map(|j| (j.p.x, j.p.y)).collect();
    let observed: Vec<(f32, f32)> = wall.path.points.iter().map(|p| (p.x, p.y)).collect();
    assert_eq!(
        observed, input,
        "{witness}: canonical traverse_extrusions keeps this line's input order, but build_walls \
         emitted a different order"
    );
    verify_build_walls(scene, &valid_build_limits(), &walls)
        .unwrap_or_else(|error| panic!("{witness}: full comparator rejected the result: {error}"));
    signed_area(&points_as_f64(&wall.path.points))
}

/// Witness (a): an open counter-clockwise chain in a three-line region under
/// `wall_direction = counter_clockwise`. Canonical never reorients an open line.
fn winding_witness_open_path_keeps_order() {
    let _guard = CaseGuard::new();
    let geometry = witness_geometry();
    let mut spacings = witness_spacings();
    let scene = winding_scene(
        vec![
            make_line(&geometry.outer, 0, false, true, &mut spacings),
            make_line(&geometry.open_ccw, 1, false, false, &mut spacings),
            make_line(&geometry.thin, 2, false, true, &mut spacings),
        ],
        true,
        false,
    );
    expect_input_order_kept(
        &scene,
        1,
        "winding witness (a) open CCW path, 3-line region, wall_direction=CCW",
    );
}

/// Witness (b): a closed clockwise inner line in a three-line region under
/// `wall_direction = clockwise`. It is a contour by `ExtrusionLine::is_contour`,
/// so canonical wants it clockwise and leaves it alone.
fn winding_witness_clockwise_inner_is_contour() {
    let _guard = CaseGuard::new();
    let geometry = witness_geometry();
    let mut spacings = witness_spacings();
    let scene = winding_scene(
        vec![
            make_line(&geometry.outer, 0, false, true, &mut spacings),
            make_line(&geometry.inner, 1, false, true, &mut spacings),
            make_line(&geometry.open_ccw, 2, false, false, &mut spacings),
        ],
        false,
        false,
    );
    let witness = "winding witness (b) closed CW inner line is a contour, 3-line region, \
                   wall_direction=CW";
    let area = expect_input_order_kept(&scene, 1, witness);
    assert!(
        area < 0.0,
        "{witness}: the wall must stay clockwise, signed area {area}"
    );
}

/// Witness (c): a two-line region whose closed inner line is counter-clockwise
/// (a hole by `ExtrusionLine::is_contour`) under `wall_direction =
/// counter_clockwise`. `pg_extrusions.size() == 2` makes it contour-like, so
/// canonical wants it counter-clockwise and leaves it alone.
fn winding_witness_two_line_region_inner_is_contour_like() {
    let _guard = CaseGuard::new();
    let geometry = witness_geometry();
    let mut spacings = witness_spacings();
    let scene = winding_scene(
        vec![
            make_line(&geometry.outer, 0, false, true, &mut spacings),
            make_line(&geometry.inner_ccw, 1, false, true, &mut spacings),
        ],
        true,
        true,
    );
    let witness = "winding witness (c) closed CCW inner line in a 2-line region, \
                   wall_direction=CCW";
    let area = expect_input_order_kept(&scene, 1, witness);
    assert!(
        area > 0.0,
        "{witness}: the wall must stay counter-clockwise, signed area {area}"
    );
}

/// The `M-ARACHNE-BUILD` winding prelude.
fn build_winding_witnesses() {
    winding_witness_open_path_keeps_order();
    winding_witness_clockwise_inner_is_contour();
    winding_witness_two_line_region_inner_is_contour_like();
}

/// Deterministic proof that the bounds reach generation rather than decorating
/// a fixed fixture.
///
/// Two accepted bound pairs are turned into scenes with the same mode and the
/// same bead-spacing base. Only the bound-driven draw components differ:
/// `max_abs_mm_units` sets `half_units` and `max_junctions` sets the junction
/// total. Both scenes must run the real kernel and pass the full comparator,
/// and their observed coordinate extents and junction counts must differ. A
/// junction bound below the roster minimum must still fail closed before any
/// sampling.
fn build_bounds_drive_geometry_and_junctions() {
    let _guard = CaseGuard::new();

    let small_limits = checked_build_limits(MIN_JUNCTIONS, MIN_ABS_MM_UNITS, 4_000, 500)
        .expect("the smallest accepted profile must be a valid bound pair");
    let large_limits = checked_build_limits(MIN_JUNCTIONS + 9, 6 * MIN_ABS_MM_UNITS, 4_000, 500)
        .expect("a larger accepted profile must be a valid bound pair");

    let mode = 0b1000; // five-line roster with the bridge band present
    let draw_for = |limits: &BuildLimits, junction_total: usize| BuildDraw {
        mode,
        height_units: 500,
        spacing_base_units: spacing_base_band(limits).1,
        half_units: limits.max_abs_mm_units,
        junction_total,
    };
    let small = build_scene(
        draw_for(&small_limits, MIN_JUNCTIONS as usize),
        &small_limits,
    );
    let large = build_scene(
        draw_for(&large_limits, (MIN_JUNCTIONS + 9) as usize),
        &large_limits,
    );

    let small_junctions: usize = small.lines.iter().map(|line| line.junctions.len()).sum();
    let large_junctions: usize = large.lines.iter().map(|line| line.junctions.len()).sum();
    assert_eq!(
        small_junctions, MIN_JUNCTIONS as usize,
        "the small profile's roster must carry exactly its sampled junction total"
    );
    assert_eq!(
        large_junctions,
        (MIN_JUNCTIONS + 9) as usize,
        "the large profile's roster must carry exactly its sampled junction total"
    );
    assert_ne!(
        small_junctions, large_junctions,
        "two different max_junctions bounds must reach generation"
    );

    let small_extent = scene_extent_units(&small);
    let large_extent = scene_extent_units(&large);
    assert_eq!(
        small_extent, MIN_ABS_MM_UNITS as i64,
        "the small profile's extent must be its own max_abs_mm_units"
    );
    assert_eq!(
        large_extent,
        (6 * MIN_ABS_MM_UNITS) as i64,
        "the large profile's extent must be its own max_abs_mm_units"
    );
    assert_ne!(
        small_extent, large_extent,
        "two different max_abs_mm_units bounds must reach generation"
    );

    // Both scenes are real kernel inputs, not just fixture arithmetic: the
    // emitted walls must report the same extent and the same junction total.
    for (scene, limits) in [(&small, &small_limits), (&large, &large_limits)] {
        let region = region_for(scene);
        let walls = run_build_walls(scene, &region)
            .expect("the bounds control scene must be accepted by build_walls");
        verify_build_walls(scene, limits, &walls)
            .expect("the bounds control scene must satisfy the full comparator");
        let emitted_extent = walls
            .iter()
            .flat_map(|wall| wall.path.points.iter())
            .map(|point| mm_to_units(point.x).abs().max(mm_to_units(point.y).abs()))
            .max()
            .unwrap_or(0);
        assert_eq!(
            emitted_extent, scene.half_units,
            "the emitted walls must reach the bound-derived half-extent"
        );
        let emitted_junctions: usize = walls.iter().map(|wall| wall.path.points.len()).sum();
        assert_eq!(
            emitted_junctions, scene.junction_total,
            "the emitted walls must carry the bound-derived junction total"
        );
    }

    // Representative accepted profiles across the whole declared domain, with a
    // sample of mode words and both ends of each profile's junction range. This
    // is the shape of what the registry campaign will draw once the profile
    // bounds are raised to the new roster minimum.
    for (junctions, abs_units, spacing_units, height_units) in [
        (MIN_JUNCTIONS, MIN_ABS_MM_UNITS, 4_000, 500),
        (MIN_JUNCTIONS + 9, 6 * MIN_ABS_MM_UNITS, 4_000, 500),
        (
            MAX_JUNCTIONS_CEILING,
            MAX_ABS_MM_UNITS_CEILING,
            8_000,
            3_000,
        ),
    ] {
        let limits = checked_build_limits(junctions, abs_units, spacing_units, height_units)
            .expect("the domain-sweep profile must be a valid bound pair");
        for total in [MIN_JUNCTIONS as usize, limits.max_junctions] {
            for mode in [
                0b000000u32,
                0b000001,
                0b001000,
                0b010000,
                0b101010,
                0b111111,
            ] {
                let scene = build_scene(
                    BuildDraw {
                        mode,
                        height_units: height_units as u32,
                        spacing_base_units: spacing_base_band(&limits).1,
                        half_units: limits.max_abs_mm_units,
                        junction_total: total,
                    },
                    &limits,
                );
                let region = region_for(&scene);
                let walls = run_build_walls(&scene, &region).unwrap_or_else(|error| {
                    panic!(
                        "profile {junctions}/{abs_units}, mode {mode:#08b}, {total} junctions: \
                         build_walls rejected: {error}"
                    )
                });
                verify_build_walls(&scene, &limits, &walls).unwrap_or_else(|error| {
                    panic!(
                        "profile {junctions}/{abs_units}, mode {mode:#08b}, {total} junctions: \
                         comparator rejected: {error}"
                    )
                });
            }
        }
    }

    // A bound below the roster minimum still fails closed before sampling.
    let error = checked_build_limits(MIN_JUNCTIONS - 1, MIN_ABS_MM_UNITS, 4_000, 500)
        .expect_err("a junction bound below the roster minimum must be rejected");
    assert!(
        error.contains("max_junctions"),
        "the rejection must name the junction bound, got {error}"
    );
    let error = checked_build_limits(MIN_JUNCTIONS, MIN_ABS_MM_UNITS - 1, 4_000, 500)
        .expect_err("a coordinate bound below the half-extent floor must be rejected");
    assert!(
        error.contains("max_abs_mm_units"),
        "the rejection must name the coordinate bound, got {error}"
    );
}

/// The observed coordinate extent of a generated scene: the largest absolute
/// junction coordinate, in slicer units.
fn scene_extent_units(scene: &BuildScene) -> i64 {
    scene
        .lines
        .iter()
        .flat_map(|line| line.junctions.iter())
        .map(|junction| {
            mm_to_units(junction.p.x)
                .abs()
                .max(mm_to_units(junction.p.y).abs())
        })
        .max()
        .unwrap_or(0)
}

#[test]
fn build_walls_winding_witness_open_path_keeps_order() {
    winding_witness_open_path_keeps_order();
}

#[test]
fn build_walls_winding_witness_clockwise_inner_is_contour() {
    winding_witness_clockwise_inner_is_contour();
}

#[test]
fn build_walls_winding_witness_two_line_region_inner_is_contour_like() {
    winding_witness_two_line_region_inner_is_contour_like();
}

#[test]
fn build_walls_bounds_drive_geometry_and_junctions() {
    build_bounds_drive_geometry_and_junctions();
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic regression: the reverse-trigger case with unequal widths.
// ─────────────────────────────────────────────────────────────────────────────

/// Root cause: `build_walls` snapshotted `widths` from `path.points` and built
/// `feature_flags` on the pre-normalization point order, then reversed
/// `path.points` for the winding contract without reversing either parallel
/// vector. On this concrete unequal-width, asymmetric-bridge, closed rectangle
/// the winding branch fires and every index-aligned width/flag was attached to
/// the wrong geometric point.
///
/// The fixture is a closed counter-clockwise inner rectangle in a three-line
/// region under `wall_direction = counter_clockwise`, which canonical
/// `traverse_extrusions` reverses (see the scene comment). It also reversed
/// under the former inset-based winding rule, so it is not a falsifier of the
/// winding correction; the three `build_walls_winding_witness_*` tests are.
#[test]
fn build_walls_reversed_variable_width_keeps_profiles_and_bridge_flags_attached() {
    let geometry = witness_geometry();
    // Counter-clockwise closed inner wall at half the witness region extent,
    // with an explicit repeated endpoint and four unequal bead spacings on the
    // distinct corners.
    let spacings = [0.50_f32, 0.60, 0.70, 0.80, 0.50];
    let junctions: Vec<ExtrusionJunction> = geometry
        .inner_ccw
        .iter()
        .zip(spacings)
        .map(|((x, y), spacing)| ExtrusionJunction {
            p: point3(mm(*x), mm(*y), spacing),
            perimeter_index: 1,
        })
        .collect();
    let line = ExtrusionLine {
        junctions,
        inset_idx: 1,
        is_odd: false,
        is_closed: true,
    };
    // Two companion lines make this a three-line region, so canonical's
    // `pg_extrusions.size() == 2` term does not apply.
    let mut companion_spacings = witness_spacings();
    let lines = vec![
        make_line(&geometry.outer, 0, false, true, &mut companion_spacings),
        line,
        make_line(&geometry.open_cw, 2, false, false, &mut companion_spacings),
    ];
    let junction_total: usize = lines.iter().map(|line| line.junctions.len()).sum();

    let scene = BuildScene {
        lines,
        // The bridge band is the region's right half for this fixture: it
        // covers the two positive-x corners of the closed inner rectangle and
        // neither negative-x corner.
        bridge_areas: vec![right_half_rect(WITNESS_HALF_UNITS)],
        // `wall_direction = counter_clockwise`. A closed input wound CCW is a
        // hole under canonical `ExtrusionLine::is_contour`, and with three lines
        // in the region `traverse_extrusions` makes it clockwise. This
        // counter-clockwise input therefore has to be reversed.
        contour_should_be_ccw: true,
        g7_reverse: false,
        print_thin_walls: false,
        layer_height_mm: REGRESSION_HEIGHT_MM,
        half_units: WITNESS_HALF_UNITS,
        junction_total,
        witness: BuildWitness {
            bridge_present: true,
            thin_enabled: false,
            contour_ccw: true,
            g7_reverse: false,
            two_line_region: false,
            inner_ccw: true,
        },
    };
    let limits = valid_build_limits();

    let region = region_for(&scene);
    let walls =
        run_build_walls(&scene, &region).expect("the concrete reverse-trigger case is valid");
    assert_eq!(
        walls.len(),
        3,
        "three nonempty lines yield exactly three walls"
    );
    let wall = walls
        .iter()
        .find(|wall| wall.perimeter_index == 1)
        .expect("the inset-1 target line must have produced a wall");
    assert_eq!(
        wall.path.points.len(),
        geometry.inner_ccw.len(),
        "the wall must preserve every input junction, including the repeated endpoint"
    );

    // The winding branch really fired: the emitted rectangle is clockwise while
    // the input was counter-clockwise.
    let emitted_area = signed_area(&points_as_f64(&wall.path.points));
    assert!(
        emitted_area < 0.0,
        "the counter-clockwise (hole) input must be reversed into a clockwise wall, got signed \
         area {emitted_area}"
    );

    let envelope = width_envelope(&limits);
    let mut outside_count = 0usize;
    let mut inside_count = 0usize;
    for (index, point) in wall.path.points.iter().enumerate() {
        // Independent prediction of the point order: the reversal applied to a
        // five-junction closed rectangle maps output index `i` to input index
        // `geometry.inner_ccw.len() - 1 - i`.
        let source = &scene.lines[1].junctions[geometry.inner_ccw.len() - 1 - index];

        assert_eq!(
            (point.x, point.y),
            (source.p.x, source.p.y),
            "index {index}: the emitted point must be the reversed source junction"
        );

        let expected_width = expected_width_mm(source.p.width, REGRESSION_HEIGHT_MM);
        assert!(
            (f64::from(point.width) - expected_width).abs() <= envelope,
            "index {index}: spacing {} mm must convert to {expected_width} mm (envelope \
             {envelope}), observed {} mm",
            source.p.width,
            point.width
        );
        assert_eq!(
            wall.width_profile.widths[index], point.width,
            "index {index}: width_profile.widths[{index}] = {} must equal path.points[{index}].width \
             = {}",
            wall.width_profile.widths[index], point.width
        );

        let bridge = predicted_bridge_membership(point.x, point.y, &scene.bridge_areas)
            .expect("the concrete points stay strictly off the bridge boundary");
        assert_eq!(
            wall.feature_flags[index].is_bridge, bridge,
            "index {index}: is_bridge must describe the point at ({}, {})",
            point.x, point.y
        );
        if bridge {
            inside_count += 1;
        } else {
            outside_count += 1;
        }
        assert_eq!(
            point.flow_factor,
            if bridge { BRIDGE_FLOW_RATIO } else { 1.0 },
            "index {index}: flow_factor must follow this point's own bridge state"
        );
    }
    assert_eq!(
        (outside_count, inside_count),
        (2, 3),
        "the fixture must produce an asymmetric bridge population: two of the four distinct \
         corners are inside, and the repeated endpoint repeats the first corner's flag"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic controls: the module's independent oracles reject known defects.
// ─────────────────────────────────────────────────────────────────────────────

fn valid_build_limits() -> BuildLimits {
    checked_build_limits(MIN_JUNCTIONS, WITNESS_HALF_UNITS as u64, 10_000, 3_000)
        .expect("the deterministic witness profile is a valid bound pair")
}

fn classify_controls() {
    // Independently tabulate all Boolean modes and both inset classes.
    for is_odd in [false, true] {
        for print_thin_walls in [false, true] {
            for is_closed in [false, true] {
                let line = ExtrusionLine {
                    junctions: vec![ExtrusionJunction {
                        p: point3(0.0, 0.0, 0.4),
                        perimeter_index: 0,
                    }],
                    inset_idx: 0,
                    is_odd,
                    is_closed,
                };
                assert_eq!(
                    classify_line(&line, print_thin_walls),
                    expected_classification(is_odd, 0, print_thin_walls),
                    "inset-0 row: is_odd={is_odd} thin={print_thin_walls} closed={is_closed}"
                );
                for inset_idx in [1u32, 2, 7] {
                    let mut inner = line.clone();
                    inner.inset_idx = inset_idx;
                    assert_eq!(
                        classify_line(&inner, print_thin_walls),
                        (ExtrusionRole::InnerWall, LoopType::Inner),
                        "positive-inset row: is_odd={is_odd} inset={inset_idx}"
                    );
                }
            }
        }
    }

    // The rejected mutation: a deeper odd wall mapped to GapFill.
    let witness = ExtrusionLine {
        junctions: vec![ExtrusionJunction {
            p: point3(0.0, 0.0, 0.4),
            perimeter_index: 2,
        }],
        inset_idx: 2,
        is_odd: true,
        is_closed: false,
    };
    let real = classify_line(&witness, false);
    let mutated = deeper_odd_as_gap_fill(true, 2, false);
    assert_eq!(
        mutated,
        (ExtrusionRole::GapFill, LoopType::GapFill),
        "the mutation must actually produce GapFill"
    );
    assert_ne!(
        real, mutated,
        "the deeper-odd-as-GapFill mutation must be rejected by the real kernel"
    );
    assert_eq!(
        real,
        (ExtrusionRole::InnerWall, LoopType::Inner),
        "a deeper odd line is an inner wall, never a gap fill"
    );

    classify_shrink_retains_positivity();
}

/// Shrink control for the zero/positive inset selector.
///
/// The strategy's zero row is a `bool` (`true` = zero) beside a positive range
/// `1..=max_inset_idx`. Proptest simplifies `bool` only `true -> false`, and
/// `false` selects the positive component, so a case that starts positive can
/// only ever shrink its inset inside `1..=max_inset_idx`. This walks real
/// strategy trees to show that no simplification crosses from a positive inset
/// to zero, and that a zero start is allowed to become positive (a legal case
/// whose expectation is re-derived).
fn classify_shrink_retains_positivity() {
    let max_inset_idx = 8u32;
    let strategy = classify_strategy(max_inset_idx);
    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 1,
        max_local_rejects: 0,
        max_global_rejects: 0,
        max_flat_map_regens: 0,
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x5eed_c1a5),
        failure_persistence: None,
        ..proptest::test_runner::Config::default()
    });
    let mut saw_positive_start = false;
    let mut saw_simplification = false;
    for _ in 0..16 {
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("a classify shrink-audit tree must be generated");
        let start = tree.current().inset_idx;
        saw_positive_start |= start > 0;
        let mut previous = start;
        for _ in 0..16 {
            if !tree.simplify() {
                break;
            }
            saw_simplification = true;
            let current = tree.current().inset_idx;
            assert!(
                current <= max_inset_idx,
                "shrinking left the declared inset range: {current} > {max_inset_idx}"
            );
            if previous > 0 {
                assert!(
                    current > 0,
                    "a positive inset case shrank to zero: {previous} -> {current}"
                );
            }
            previous = current;
        }
        if start > 0 {
            assert!(
                tree.current().inset_idx > 0,
                "the final simplified tree lost the positive inset"
            );
        }
    }
    assert!(
        saw_positive_start,
        "the shrink audit never drew a positive-inset case, so it certifies nothing"
    );
    assert!(
        saw_simplification,
        "the shrink audit simplified nothing, so it certifies nothing"
    );
}

fn build_controls() {
    let limits = valid_build_limits();
    let draw_for = |mode: u32| BuildDraw {
        mode,
        height_units: 2_000,
        spacing_base_units: spacing_base_band(&limits).1,
        half_units: limits.max_abs_mm_units,
        junction_total: MIN_JUNCTIONS as usize,
    };
    // Mode 0b1101: counter-clockwise contour, thin walls enabled, bridge present.
    let scene = build_scene(draw_for(0b1101), &limits);
    let region = region_for(&scene);
    let walls = run_build_walls(&scene, &region).expect("the control scene must be accepted");
    verify_build_walls(&scene, &limits, &walls)
        .expect("the comparator must accept the real, unmutated result");

    // Every mode word the roster distinguishes (both wall directions, both
    // `g7_reverse` values, thin walls, bridge, the two-line region and both
    // closed-inner windings) is accepted by the comparator on the real kernel,
    // so no branch depends on the sampler happening to draw it.
    for mode in 0..64u32 {
        let swept = build_scene(draw_for(mode), &limits);
        let swept_region = region_for(&swept);
        let swept_walls = run_build_walls(&swept, &swept_region)
            .unwrap_or_else(|error| panic!("mode {mode:#08b}: build_walls rejected: {error}"));
        verify_build_walls(&swept, &limits, &swept_walls)
            .unwrap_or_else(|error| panic!("mode {mode:#08b}: comparator rejected: {error}"));
    }

    // Mutation 1: points-only reversal — reverse the coordinates and leave the
    // parallel width/flag arrays alone. Every per-point check in the comparator
    // is an association check, and any of them may fire first depending on the
    // geometry; what matters is that one of them does.
    let mut points_only = walls.clone();
    for wall in &mut points_only {
        wall.path.points.reverse();
    }
    let error = verify_build_walls(&scene, &limits, &points_only)
        .expect_err("a points-only reversal must be rejected");
    assert!(
        error.contains("association moved")
            || error.contains("must convert to")
            || error.contains("is_bridge"),
        "the points-only reversal must break the point/width/flag association, got {error}"
    );

    // Mutation 2: a missed spacing -> width conversion.
    let mut unconverted = walls.clone();
    for (wall, line) in unconverted
        .iter_mut()
        .zip(scene.lines.iter().filter(|line| !line.junctions.is_empty()))
    {
        for (point, junction) in wall.path.points.iter_mut().zip(&line.junctions) {
            point.width = junction.p.width;
        }
    }
    let error = verify_build_walls(&scene, &limits, &unconverted)
        .expect_err("an unconverted spacing must be rejected");
    assert!(
        error.contains("must convert to"),
        "the missed conversion must be caught by the width formula, got {error}"
    );

    // Mutation 3: fabricated closure on an open path.
    let mut closed_open_path = walls.clone();
    let open_wall = closed_open_path
        .iter_mut()
        .find(|wall| wall.perimeter_index == 2)
        .expect("the open inner chain must have produced a wall");
    let first = open_wall.path.points[0];
    open_wall.path.points.push(first);
    open_wall
        .width_profile
        .widths
        .push(open_wall.width_profile.widths[0]);
    open_wall
        .feature_flags
        .push(open_wall.feature_flags[0].clone());
    let error = verify_build_walls(&scene, &limits, &closed_open_path)
        .expect_err("fabricated closure on an open path must be rejected");
    assert!(
        error.contains("closure"),
        "the fabricated closure must be caught by the open/closed agreement, got {error}"
    );
}

fn sequence_controls() {
    // A controlled outer-first batch under InnerOuter must commit unchanged, so
    // a naive `reverse()` mutant is rejected.
    let case = build_sequence_case(3, 0, 0);
    let mut walls = case.walls.clone();
    commit_wall_sequence(&mut walls, case.mode);
    verify_sequence(&case, &walls).expect("the real commitment must match the mode table");
    let mut naive_reverse = walls.clone();
    naive_reverse.reverse();
    let error = verify_sequence(&case, &naive_reverse)
        .expect_err("a naive full reversal of an outer-first InnerOuter batch must be rejected");
    assert!(
        error.contains("slot 0"),
        "the order comparator must fail at the first mismatched slot, got {error}"
    );

    // Dropping parallel metadata must be rejected even when the order is right.
    let mut metadata_lost = walls.clone();
    metadata_lost[1].feature_flags.clear();
    let error =
        verify_sequence(&case, &metadata_lost).expect_err("a lost feature-flag vector must fail");
    assert!(
        error.contains("slot 1"),
        "the identity comparator must catch dropped per-point metadata, got {error}"
    );

    // `OuterInner` on the same outer-first batch is a full reverse.
    let reversed_case = build_sequence_case(3, 0, 1);
    let mut reversed_walls = reversed_case.walls.clone();
    commit_wall_sequence(&mut reversed_walls, reversed_case.mode);
    let identifiers: Vec<u32> = reversed_walls
        .iter()
        .map(|wall| wall.perimeter_index)
        .collect();
    assert_eq!(
        identifiers,
        vec![2, 1, 0],
        "OuterInner must emit the innermost wall first on an outer-first batch"
    );
    verify_sequence(&reversed_case, &reversed_walls)
        .expect("the reversed commitment must match the mode table");
}

#[test]
fn arachne_module_oracle_controls() {
    classify_controls();
    build_controls();
    sequence_controls();
}

// ─────────────────────────────────────────────────────────────────────────────
// Deterministic controls: the domains fail closed before sampling.
// ─────────────────────────────────────────────────────────────────────────────

fn classify_domain_rejections() {
    for rejected in [0u64, MAX_INSET_IDX_CEILING + 1, u64::MAX] {
        assert!(
            checked_classify_limits(rejected).is_err(),
            "max_inset_idx {rejected} must be rejected before sampling"
        );
    }
    for accepted in [1u64, MAX_INSET_IDX_CEILING] {
        assert!(
            checked_classify_limits(accepted).is_ok(),
            "max_inset_idx {accepted} must be accepted"
        );
    }
}

fn build_domain_rejections() {
    let valid = (MIN_JUNCTIONS, MIN_ABS_MM_UNITS, 4_000u64, 500u64);
    assert!(
        checked_build_limits(valid.0, valid.1, valid.2, valid.3).is_ok(),
        "the smallest accepted profile must be accepted"
    );

    // Overflow / narrowing and zero bounds.
    for bound in [
        (0u64, valid.1, valid.2, valid.3),
        (u64::MAX, valid.1, valid.2, valid.3),
        (valid.0, 0, valid.2, valid.3),
        (valid.0, u64::MAX, valid.2, valid.3),
        (valid.0, valid.1, 0, valid.3),
        (valid.0, valid.1, u64::MAX, valid.3),
        (valid.0, valid.1, valid.2, 0),
        (valid.0, valid.1, valid.2, u64::MAX),
    ] {
        assert!(
            checked_build_limits(bound.0, bound.1, bound.2, bound.3).is_err(),
            "bound tuple {bound:?} must be rejected before sampling"
        );
    }

    // Impossible witness bounds: the five-line roster needs its own junction
    // count, and the sampled half-extent needs its own coordinate floor.
    assert!(
        checked_build_limits(MIN_JUNCTIONS - 1, valid.1, valid.2, valid.3).is_err(),
        "fewer than {MIN_JUNCTIONS} junctions cannot carry the five-line roster"
    );
    assert!(
        checked_build_limits(valid.0, MIN_ABS_MM_UNITS - 1, valid.2, valid.3).is_err(),
        "a coordinate bound below {MIN_ABS_MM_UNITS} units cannot hold a 1 mm half-extent"
    );
    for ceiling in [
        (MAX_JUNCTIONS_CEILING + 1, valid.1, valid.2, valid.3),
        (valid.0, MAX_ABS_MM_UNITS_CEILING + 1, valid.2, valid.3),
        (valid.0, valid.1, MAX_SPACING_UNITS_CEILING + 1, valid.3),
        (valid.0, valid.1, valid.2, MAX_HEIGHT_UNITS_CEILING + 1),
    ] {
        assert!(
            checked_build_limits(ceiling.0, ceiling.1, ceiling.2, ceiling.3).is_err(),
            "out-of-ceiling tuple {ceiling:?} must be rejected"
        );
    }

    // A spacing bound must still fit one increasing integer unit per junction.
    assert!(
        checked_build_limits(valid.0, valid.1, valid.0 - 1, valid.3).is_err(),
        "a spacing bound below one unit per junction cannot hold the roster's spacings"
    );

    // Invalid bead/geometry domains: a bead or a layer that cannot fit twice
    // inside the declared coordinate box, and a width domain that could leave
    // f32's normal range.
    assert!(
        checked_build_limits(valid.0, 20_000, 20_000, valid.3).is_err(),
        "a spacing that does not fit twice inside the coordinate box is invalid"
    );
    assert!(
        checked_build_limits(valid.0, 20_000, valid.2, 20_000).is_err(),
        "a layer height that does not fit twice inside the coordinate box is invalid"
    );
    assert!(
        checked_build_limits(
            valid.0,
            MAX_ABS_MM_UNITS_CEILING,
            MAX_SPACING_UNITS_CEILING,
            MAX_HEIGHT_UNITS_CEILING,
        )
        .is_err(),
        "a bead domain above the {WIDTH_DOMAIN_MM_CEILING} mm width ceiling is invalid"
    );
}

fn sequence_domain_rejections() {
    for rejected in [0u64, MIN_WALLS - 1, MAX_WALLS_CEILING + 1, u64::MAX] {
        assert!(
            checked_sequence_limits(rejected).is_err(),
            "max_walls {rejected} must be rejected before sampling"
        );
    }
    for accepted in [MIN_WALLS, MAX_WALLS_CEILING] {
        assert!(
            checked_sequence_limits(accepted).is_ok(),
            "max_walls {accepted} must be accepted"
        );
    }
}

/// Every shrink must keep the case's geometry and mode witness, and the finally
/// reported simplified tree must still satisfy the contract.
///
/// The profile deliberately has a junction range (`23..=32`) and a coordinate
/// range (`10_000..=60_000` units) so shrinking exercises the bound-driven draws
/// as well as the layer height and spacing base.
fn shrink_retains_geometry_and_mode_witness() {
    let limits = checked_build_limits(MIN_JUNCTIONS + 9, 6 * MIN_ABS_MM_UNITS, 4_000, 500)
        .expect("the shrink-audit profile is a valid bound pair");
    let strategy = build_strategy(&limits);
    let mut runner = proptest::test_runner::TestRunner::new(proptest::test_runner::Config {
        cases: 1,
        max_local_rejects: 0,
        max_global_rejects: 0,
        max_flat_map_regens: 0,
        rng_algorithm: proptest::test_runner::RngAlgorithm::ChaCha,
        rng_seed: proptest::test_runner::RngSeed::Fixed(0x5eed_0a7c),
        failure_persistence: None,
        ..proptest::test_runner::Config::default()
    });
    let mut simplified_anything = false;
    for _ in 0..8 {
        let mut tree = strategy
            .new_tree(&mut runner)
            .expect("a shrink-audit tree must be generated");
        let witness = tree.current().witness;
        for _ in 0..8 {
            if !tree.simplify() {
                break;
            }
            simplified_anything = true;
            assert_eq!(
                tree.current().witness,
                witness,
                "shrinking changed the case's mode witness"
            );
        }
        let shrunk = tree.current();
        assert_eq!(
            shrunk.witness, witness,
            "the final tree lost its mode witness"
        );
        check_build_scene(&shrunk, &limits).unwrap_or_else(|error| {
            panic!("the final simplified tree no longer satisfies the contract: {error:?}")
        });
        assert!(
            shrunk.lines.iter().any(|line| !line.junctions.is_empty()),
            "the shrunk case must retain a nonempty line"
        );
    }
    assert!(
        simplified_anything,
        "the shrink audit simplified nothing, so it certifies nothing"
    );
}

#[test]
fn arachne_module_domain_rejections() {
    classify_domain_rejections();
    build_domain_rejections();
    sequence_domain_rejections();
    shrink_retains_geometry_and_mode_witness();
}
