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
//! (`crates/slicer-core/src/arachne/pipeline.rs`). Every literal of those five
//! uses a `..` rest; `ExtrusionLine` (4 fields), `ExtrusionJunction` (2) and
//! `WidthProfile` (1) are below the watch threshold.
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
    Point2, Point3WithWidth, Polygon, QuartileBand, WallBoundaryType, WallFeatureFlags, WallLoop,
    WidthProfile,
};
use slicer_sdk::host::{ArachneParams, WallSequence};
use slicer_sdk::test_support::fixtures::{wall_loop_base, SliceRegionViewBuilder};
use slicer_sdk::test_support::property_testing::oracles::{point_in_region, PointLocation};
use slicer_sdk::test_support::property_testing::PropertyRun;
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
        // Zero stays zero; a positive inset shrinks toward 1 without ever
        // losing positivity, so the `InnerWall` row survives every shrink path.
        prop_oneof![Just(0u32), 1u32..=max_inset_idx],
        any::<bool>(),
        any::<bool>(),
        0u32..=3,
    )
        .prop_map(
            |(is_odd, inset_idx, print_thin_walls, is_closed, junction_count)| ClassifyCase {
                is_odd,
                inset_idx,
                print_thin_walls,
                is_closed,
                junction_count,
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

/// Region half-extent: the declared region is the 4 mm square
/// `[-REGION_HALF_UNITS, +REGION_HALF_UNITS]^2`, in slicer units.
const REGION_HALF_UNITS: i64 = 20_000;

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

/// Junctions the generated roster places across its five nonempty lines.
const ROSTER_JUNCTIONS: u32 = 23;
/// Lower end of the generated layer-height range, in slicer units.
const MIN_GENERATED_HEIGHT_UNITS: u32 = 1_000;

/// Closed, explicitly repeated outer-wall rectangle on the region boundary,
/// wound clockwise.
const OUTER_RECT: [(i64, i64); 5] = [
    (REGION_HALF_UNITS, REGION_HALF_UNITS),
    (REGION_HALF_UNITS, -REGION_HALF_UNITS),
    (-REGION_HALF_UNITS, -REGION_HALF_UNITS),
    (-REGION_HALF_UNITS, REGION_HALF_UNITS),
    (REGION_HALF_UNITS, REGION_HALF_UNITS),
];

/// Closed, explicitly repeated inner-wall rectangle at half the region extent,
/// wound clockwise.
const INNER_RECT: [(i64, i64); 5] = [
    (10_000, 10_000),
    (10_000, -10_000),
    (-10_000, -10_000),
    (-10_000, 10_000),
    (10_000, 10_000),
];

/// Open inner-wall chain in the lower-left quadrant, wound counter-clockwise,
/// with distinct endpoints.
const OPEN_CCW: [(i64, i64); 4] = [
    (-15_000, -15_000),
    (-10_000, -15_000),
    (-10_000, -10_000),
    (-15_000, -10_000),
];

/// The exact point-order reverse of [`OPEN_CCW`]: an open clockwise chain.
const OPEN_CW: [(i64, i64); 4] = [
    (-15_000, -10_000),
    (-10_000, -10_000),
    (-10_000, -15_000),
    (-15_000, -15_000),
];

/// Closed, explicitly repeated small rectangle wound counter-clockwise. Its
/// first corner (0.5 mm, 0.5 mm) lies strictly inside the mixed bridge area, so
/// the `ThinWall` exclusion has a non-vacuous positive witness.
const THIN_RECT: [(i64, i64); 5] = [
    (5_000, 5_000),
    (-5_000, 5_000),
    (-5_000, -5_000),
    (5_000, -5_000),
    (5_000, 5_000),
];

/// The mixed bridge area: strictly inside the region, covering the
/// `(1 mm, 1 mm)` and `(1 mm, -1 mm)` corners of [`INNER_RECT`] but neither
/// negative-x corner. Asymmetric on purpose — a points-only reversal changes
/// which points the flag vector describes.
const BRIDGE_MIN_UNITS: i64 = 0;
const BRIDGE_MAX_UNITS: i64 = 15_000;

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

fn region_rect() -> ExPolygon {
    rect_units(
        -REGION_HALF_UNITS,
        -REGION_HALF_UNITS,
        REGION_HALF_UNITS,
        REGION_HALF_UNITS,
    )
}

fn mixed_bridge_rect() -> ExPolygon {
    rect_units(
        BRIDGE_MIN_UNITS,
        -BRIDGE_MAX_UNITS,
        BRIDGE_MAX_UNITS,
        BRIDGE_MAX_UNITS,
    )
}

/// Overhang band covering the region's lower half (`y < 0`). Every generated
/// point's y coordinate is a nonzero multiple of 0.5 mm, so no point sits on the
/// band boundary.
fn lower_half_band() -> QuartileBand {
    QuartileBand {
        quartile: 2,
        polygons: vec![rect_units(
            -REGION_HALF_UNITS,
            -REGION_HALF_UNITS,
            REGION_HALF_UNITS,
            0,
        )],
    }
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

/// Written ceiling for `max_junctions`; `MIN_JUNCTIONS` is the smallest bound
/// that still admits the repeated-endpoint rectangle witness.
const MIN_JUNCTIONS: u64 = 5;
const MAX_JUNCTIONS_CEILING: u64 = 64;
/// `max_abs_mm_units` is the absolute coordinate bound of the generated geometry
/// in slicer units (1 unit = 1e-4 mm). `MIN_ABS_MM_UNITS` admits the interior
/// rectangle witness; the ceiling is 100 mm.
const MIN_ABS_MM_UNITS: u64 = 20_000;
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
    /// Maximum junctions in one generated line.
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
            "max_junctions must be in {MIN_JUNCTIONS}..={MAX_JUNCTIONS_CEILING} \
             (the repeated-endpoint rectangle witness needs {MIN_JUNCTIONS} junctions), \
             got {max_junctions}"
        ));
    }
    if !(MIN_ABS_MM_UNITS..=MAX_ABS_MM_UNITS_CEILING).contains(&max_abs_mm_units) {
        return Err(format!(
            "max_abs_mm_units must be in {MIN_ABS_MM_UNITS}..={MAX_ABS_MM_UNITS_CEILING} \
             (the {REGION_HALF_UNITS}-unit interior rectangle witness must fit), \
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
    witness: BuildWitness,
}

/// Build the roster. Its structure is derived entirely from the `no_shrink` mode
/// word; only the layer height and the bead-spacing base are shrinkable, and
/// neither changes the witness.
fn build_scene(
    mode: u32,
    height_units: u32,
    spacing_base_units: u32,
    limits: &BuildLimits,
) -> BuildScene {
    let contour_should_be_ccw = mode & 1 != 0;
    let g7_reverse = mode & 2 != 0;
    let print_thin_walls = mode & 4 != 0;
    let bridge_present = mode & 8 != 0;

    let step_units = (limits.max_spacing_units / 64).max(1);
    let mut spacings = Spacings {
        next_units: spacing_base_units.min(limits.max_spacing_units),
        step_units,
    };

    let lines = vec![
        // Explicitly closed outer wall on the region boundary, clockwise.
        make_line(&OUTER_RECT, 0, false, true, &mut spacings),
        // Explicitly closed inner wall, clockwise; two of its five points sit
        // strictly inside the mixed bridge area.
        make_line(&INNER_RECT, 1, false, true, &mut spacings),
        // Open inner chain, counter-clockwise, distinct endpoints.
        make_line(&OPEN_CCW, 2, false, false, &mut spacings),
        // Open inner chain, the point-order reverse of the previous one.
        make_line(&OPEN_CW, 3, false, false, &mut spacings),
        // Odd inset-0 line: `ThinWall` when thin walls are enabled, otherwise an
        // ordinary `OuterWall`. Its first corner is inside the bridge area.
        make_line(&THIN_RECT, 0, true, true, &mut spacings),
        // Empty line: a legitimate Arachne input that must produce no wall.
        ExtrusionLine {
            junctions: Vec::new(),
            inset_idx: 4,
            is_odd: false,
            is_closed: false,
        },
    ];

    BuildScene {
        lines,
        bridge_areas: if bridge_present {
            vec![mixed_bridge_rect()]
        } else {
            Vec::new()
        },
        contour_should_be_ccw,
        g7_reverse,
        print_thin_walls,
        layer_height_mm: mm(i64::from(height_units.min(limits.max_height_units))),
        witness: BuildWitness {
            bridge_present,
            thin_enabled: print_thin_walls,
            contour_ccw: contour_should_be_ccw,
            g7_reverse,
        },
    }
}

fn build_strategy(limits: &BuildLimits) -> BoxedStrategy<BuildScene> {
    let limits = *limits;
    let step_units = (limits.max_spacing_units / 64).max(1);
    let widest_base = limits
        .max_spacing_units
        .saturating_sub(step_units.saturating_mul(ROSTER_JUNCTIONS + 1))
        .max(step_units);
    let lowest_height = MIN_GENERATED_HEIGHT_UNITS.min(limits.max_height_units);
    (
        any::<u32>().no_shrink(),
        lowest_height..=limits.max_height_units,
        step_units..=widest_base,
    )
        .prop_map(move |(mode, height_units, spacing_base_units)| {
            build_scene(mode, height_units, spacing_base_units, &limits)
        })
        .boxed()
}

fn region_for(scene: &BuildScene) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-arachne-property")
        .region_id(1)
        .z(Z_MM)
        .add_polygon(region_rect())
        .bridge_areas(scene.bridge_areas.clone())
        .previous_layer_boundary(vec![region_rect()])
        .overhang_quartile_polygons(vec![lower_half_band()])
        .build()
}

fn build_context(region: &SliceRegionView) -> PerimeterSpatialContext {
    PerimeterSpatialContext::new(
        region.prev_layer_boundary(),
        region.overhang_quartile_polygons(),
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

/// The winding the emitted loop must end up with, from the documented contract:
/// contours follow `wall_direction` (counter-clockwise here), holes wind
/// opposite, and `overhang_reverse` flips both on the layers it applies to. This
/// is an independent statement of the G1 contract, not the production
/// expression.
fn expected_want_ccw(inset_idx: u32, contour_should_be_ccw: bool, g7_reverse: bool) -> bool {
    let base_want_ccw = if inset_idx == 0 {
        contour_should_be_ccw
    } else {
        !contour_should_be_ccw
    };
    if g7_reverse {
        !base_want_ccw
    } else {
        base_want_ccw
    }
}

/// Boundary type when the region carries no paint annotations: the documented
/// `build_wall_flags` fallback keys off `is_outer` alone.
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

/// Exact signed distance to the axis-aligned region boundary for a point inside
/// it, derived analytically from the square's geometry.
fn expected_signed_distance_mm(x_mm: f32, y_mm: f32) -> f64 {
    let half_mm = f64::from(units_to_mm(REGION_HALF_UNITS));
    let nearest_edge = f64::from(x_mm.abs()).max(f64::from(y_mm.abs()));
    -(half_mm - nearest_edge)
}

/// Verify every nonempty input line against an independent model of the
/// conversion, and confirm the empty line produced nothing.
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

        // Which way the loop ends up, from the documented winding contract.
        let input_ccw = signed_area(
            &line
                .junctions
                .iter()
                .map(|junction| (f64::from(junction.p.x), f64::from(junction.p.y)))
                .collect::<Vec<_>>(),
        ) > 0.0;
        let want_ccw = expected_want_ccw(
            line.inset_idx,
            scene.contour_should_be_ccw,
            scene.g7_reverse,
        );
        let reversed = count >= 3 && input_ccw != want_ccw;

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
            let expected_quartile = if mm_to_units(point.y) < 0 {
                Some(2)
            } else {
                None
            };
            if point.overhang_quartile != expected_quartile {
                return Err(format!(
                    "{where_}: overhang_quartile {:?} disagrees with this point's own \
                     lower-half position ({expected_quartile:?})",
                    point.overhang_quartile
                ));
            }
            let Some(distance) = point.overhang_distance_mm else {
                return Err(format!(
                    "{where_}: a nonempty previous-layer boundary must stamp a distance"
                ));
            };
            let expected_distance =
                expected_signed_distance_mm(point.x, point.y) + 0.5 * f64::from(point.width);
            if (f64::from(distance) - expected_distance).abs() > 1.0e-4 {
                return Err(format!(
                    "{where_}: overhang distance {distance} does not equal the analytical \
                     {expected_distance} for this point"
                ));
            }
        }
    }

    if scene.bridge_areas.is_empty() {
        if saw_bridge_true {
            return Err(String::from(
                "an empty bridge view must leave every is_bridge flag false",
            ));
        }
    } else if !(saw_bridge_true && saw_bridge_false) {
        return Err(format!(
            "the mixed bridge area must produce both populations, observed positive={} \
             negative={}",
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
// Deterministic regression: the reverse-trigger case with unequal widths.
// ─────────────────────────────────────────────────────────────────────────────

/// Root cause: `build_walls` snapshotted `widths` from `path.points` and built
/// `feature_flags` on the pre-normalization point order, then reversed
/// `path.points` for the winding contract without reversing either parallel
/// vector. On this concrete unequal-width, asymmetric-bridge, closed rectangle
/// the winding branch fires and every index-aligned width/flag was attached to
/// the wrong geometric point.
#[test]
fn build_walls_reversed_variable_width_keeps_profiles_and_bridge_flags_attached() {
    // Clockwise closed inner wall at half the region extent, with an explicit
    // repeated endpoint and four unequal bead spacings on the distinct corners.
    let spacings = [0.50_f32, 0.60, 0.70, 0.80, 0.50];
    let junctions: Vec<ExtrusionJunction> = INNER_RECT
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

    let scene = BuildScene {
        lines: vec![line],
        // The bridge covers the (1 mm, 1 mm) and (1 mm, -1 mm) corners only.
        bridge_areas: vec![mixed_bridge_rect()],
        // `wall_direction = clockwise`: the contour winds clockwise and an inner
        // wall is a hole, so it must wind counter-clockwise. This clockwise input
        // therefore has to be reversed.
        contour_should_be_ccw: false,
        g7_reverse: false,
        print_thin_walls: false,
        layer_height_mm: REGRESSION_HEIGHT_MM,
        witness: BuildWitness {
            bridge_present: true,
            thin_enabled: false,
            contour_ccw: false,
            g7_reverse: false,
        },
    };
    let limits = BuildLimits {
        max_junctions: 8,
        max_abs_mm_units: REGION_HALF_UNITS,
        max_spacing_units: 10_000,
        max_height_units: 3_000,
    };

    let region = region_for(&scene);
    let walls =
        run_build_walls(&scene, &region).expect("the concrete reverse-trigger case is valid");
    assert_eq!(walls.len(), 1, "one nonempty line yields exactly one wall");
    let wall = &walls[0];
    assert_eq!(
        wall.path.points.len(),
        INNER_RECT.len(),
        "the wall must preserve every input junction, including the repeated endpoint"
    );

    // The winding branch really fired: the emitted rectangle is counter-clockwise
    // while the input was clockwise.
    let emitted_area = signed_area(&points_as_f64(&wall.path.points));
    assert!(
        emitted_area > 0.0,
        "the clockwise input must be reversed into a counter-clockwise inner wall, got signed \
         area {emitted_area}"
    );

    let envelope = width_envelope(&limits);
    let mut outside_count = 0usize;
    let mut inside_count = 0usize;
    for (index, point) in wall.path.points.iter().enumerate() {
        // Independent prediction of the point order: the reversal applied to a
        // five-junction closed rectangle maps output index `i` to input index
        // `INNER_RECT.len() - 1 - i`.
        let source = &scene.lines[0].junctions[INNER_RECT.len() - 1 - index];

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
    BuildLimits {
        max_junctions: 8,
        max_abs_mm_units: REGION_HALF_UNITS,
        max_spacing_units: 10_000,
        max_height_units: 3_000,
    }
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
}

fn build_controls() {
    // Mode 0b1101: counter-clockwise contour, thin walls enabled, bridge present.
    let scene = build_scene(0b1101, 2_000, 4_000, &valid_build_limits());
    let region = region_for(&scene);
    let walls = run_build_walls(&scene, &region).expect("the control scene must be accepted");
    let limits = valid_build_limits();
    verify_build_walls(&scene, &limits, &walls)
        .expect("the comparator must accept the real, unmutated result");

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
    let valid = (8u64, REGION_HALF_UNITS as u64, 10_000u64, 3_000u64);
    assert!(
        checked_build_limits(valid.0, valid.1, valid.2, valid.3).is_ok(),
        "the measured profile must be accepted"
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

    // Impossible witness bounds: the repeated-endpoint rectangle needs five
    // junctions, and the interior rectangle needs its own coordinate extent.
    assert!(
        checked_build_limits(MIN_JUNCTIONS - 1, valid.1, valid.2, valid.3).is_err(),
        "fewer than {MIN_JUNCTIONS} junctions cannot carry the closed-rectangle witness"
    );
    assert!(
        checked_build_limits(valid.0, MIN_ABS_MM_UNITS - 1, valid.2, valid.3).is_err(),
        "a coordinate bound below {MIN_ABS_MM_UNITS} units cannot hold the interior witness"
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
fn shrink_retains_geometry_and_mode_witness() {
    let limits = valid_build_limits();
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
