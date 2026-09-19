//! TDD test for arachne per-vertex parity packet 148, Step 6: `precise_outer_wall`.
//!
//! Canonical `process_arachne` (`PerimeterGenerator.cpp`) moves TWO quantities
//! together under the gate:
//!
//! ```text
//! last         = offset_ex(surface.expolygon,
//!                  apply_precise_outer_wall ? -(ext_perimeter_width - ext_perimeter_spacing)
//!                                           : -(ext_perimeter_width/2 - ext_perimeter_spacing/2))
//! wall_0_inset = apply_precise_outer_wall
//!                  ? -(ext_perimeter_width/2 - ext_perimeter_spacing/2) : 0
//! ```
//!
//! where `apply_precise_outer_wall = precise_outer_wall && wall_sequence ==
//! InnerOuter`. The outer bead's centreline lands at `ext_perimeter_width / 2`
//! from the model boundary under EITHER branch — the deeper outline shrink is
//! exactly cancelled by `OuterWallInsetBeadingStrategy`'s negative offset. The
//! gate's observable is therefore NOT an outer-wall move: it is the rest of
//! the shell, whose beads are placed from the more deeply shrunk outline, so
//! every non-outer bead moves inward by `bump/2` and the wall-to-wall spacing
//! becomes the true canonical spacing instead of a bump-compressed one.
//!
//! This matches the option's own description (`PrintConfig.cpp`): "Improve
//! shell precision by adjusting outer wall spacing."
//!
//! **Oracle-measured** (OrcaSlicer 2.4.1, 10 mm square, outer 0.5 / inner 0.4,
//! 2 walls, 0.2 mm layers, `inner outer`; method and raw numbers in
//! `tmp/ahull/gate_oracle/FINDINGS.md`): outer wall min-x 95.250 -> 95.250
//! (delta 0.000), inner wall 95.657 -> 95.679 (delta +0.0220 = +bump/2). A
//! second configuration (0.3 mm layers, outer 0.6 / inner 0.42) reproduced
//! both: outer 95.300 -> 95.300, inner 95.746 -> 95.778 (+0.0322 = +bump/2).
//!
//! This test drives `ArachnePerimeters::run_perimeters` end-to-end and asserts
//! those three canonical observables: the outer wall is unchanged by the gate
//! AND sits at `ext_perimeter_width / 2` from the boundary, the inner wall
//! moves inward by exactly `bump/2`, and neither effect appears when the gate
//! is not satisfied (default-off, and wall_sequence != InnerOuter).
//!
//! `preferred_bead_width_outer` stands in for `ext_perimeter_width` and
//! `optimal_width` for `ext_perimeter_spacing` — the two width-like quantities
//! `arachne_params_from_config` already reads from the manifest (see that
//! function's doc comment); there is no separate "spacing" config key in this
//! module.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, WallLoop};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

const OUTER_WIDTH_MM: f32 = 0.5;
const SPACING_WIDTH_MM: f32 = 0.4;
/// Fixture layer height (mm) — module fallback default, matches
/// `make_region`'s implicit layer used by `run_perimeters` (no `layer_height`
/// key set in `make_config`, so the 0.2mm module default applies).
const LAYER_HEIGHT_MM: f64 = 0.2;
/// The model's half-side (mm): `square_polygon(0.0, 0.0, 10.0)` spans ±5 mm.
const HALF_SIDE_MM: f64 = 5.0;
/// Canonical outline bump: `width - spacing = layer_height * (1 - PI/4)`.
const BUMP_MM: f64 = LAYER_HEIGHT_MM * (1.0 - std::f64::consts::PI / 4.0);
/// Distance from the model boundary to the outer bead's centreline. Canonical
/// places it at `ext_perimeter_width / 2` under EITHER gate branch, because the
/// extra outline shrink the gate adds is exactly cancelled by
/// `wall_0_inset`. Oracle-confirmed: 95.250 from a 95.0 boundary at 0.5 mm
/// outer width, on both gates and both oracle configurations.
const EXPECTED_OUTER_FROM_BOUNDARY_MM: f64 = OUTER_WIDTH_MM as f64 / 2.0;
/// Expected INNER wall min-x shift (mm) when the gate is satisfied: the inner
/// beads are placed from the deeper outline, so they move INWARD, i.e. min-x
/// increases toward the interior. Oracle-confirmed: inner min-x 95.657 ->
/// 95.679 from a 95.0 boundary at 0.2 mm layers (= +bump/2 = +0.02146), and
/// 95.746 -> 95.778 at 0.3 mm layers (= +bump/2 = +0.03219), in both cases to
/// within the printed gcode's 3-decimal rounding.
const EXPECTED_INNER_DELTA_MM: f64 = BUMP_MM / 2.0;
const TOLERANCE_MM: f32 = 1e-3;

/// Builds a config with distinct `optimal_width`/`preferred_bead_width_outer`
/// (so the gated offset is nonzero and observable), plus the wall-sequencing
/// keys under test. `precise_outer_wall`/`wall_sequence` are only set when
/// `Some`, so callers can exercise the "key absent" default path.
fn make_config(precise_outer_wall: Option<bool>, wall_sequence: Option<&str>) -> ConfigView {
    let mut builder = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("inner_wall_line_width", SPACING_WIDTH_MM as f64)
        .float("outer_wall_line_width", OUTER_WIDTH_MM as f64);
    if let Some(p) = precise_outer_wall {
        builder = builder.bool("precise_outer_wall", p);
    }
    if let Some(ws) = wall_sequence {
        builder = builder.string("wall_sequence", ws);
    }
    builder.build()
}

/// 10mm square region, per the packet's AC-8 fixture description.
fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

/// Runs the module end-to-end and returns the wall loop with the given
/// `perimeter_index` (0 == outer wall, 1 == first inner wall).
fn run_and_get_wall(config: &ConfigView, perimeter_index: u32) -> WallLoop {
    let module = ArachnePerimeters::from_config(config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, config)
        .unwrap();

    output
        .wall_loops()
        .iter()
        .find(|w| w.perimeter_index == perimeter_index)
        .unwrap_or_else(|| {
            panic!("a wall loop with perimeter_index == {perimeter_index} must be emitted")
        })
        .clone()
}

/// Runs the module end-to-end and returns the outer wall (`perimeter_index
/// == 0`) `WallLoop`.
fn run_and_get_outer_wall(config: &ConfigView) -> WallLoop {
    run_and_get_wall(config, 0)
}

/// Minimum X coordinate across a wall loop's path points — a single scalar
/// summary of the toolpath's placement, robust to point-ordering
/// differences, sufficient to detect a uniform inward/outward shift.
fn min_x(wall: &WallLoop) -> f32 {
    wall.path
        .points
        .iter()
        .map(|p| p.x)
        .fold(f32::INFINITY, f32::min)
}

/// AC-8 (positive, oracle-derived): `precise_outer_wall=true` +
/// `wall_sequence="InnerOuter"` must leave the outer wall exactly at
/// `OUTER_WIDTH_MM / 2` from the model boundary (the outline shrink and
/// `wall_0_inset` cancel on the outermost bead), and must move the INNER wall
/// inward by exactly `BUMP_MM / 2` relative to the same fixture with
/// `precise_outer_wall=false` — the deeper outline every non-outer bead is
/// placed from.
///
/// Both observables are oracle-measured against OrcaSlicer 2.4.1 (see this
/// file's module doc); a port that only applies the beading-strategy inset
/// without the outline shrink moves the outer wall by `-bump/2` instead and
/// fails the first assertion.
#[test]
fn precise_outer_wall_moves_inner_walls_and_holds_outer_wall_at_half_width() {
    let config_off = make_config(Some(false), Some("InnerOuter"));
    let config_on = make_config(Some(true), Some("InnerOuter"));

    let outer_off = run_and_get_outer_wall(&config_off);
    let outer_on = run_and_get_outer_wall(&config_on);
    let inner_off = run_and_get_wall(&config_off, 1);
    let inner_on = run_and_get_wall(&config_on, 1);

    // Observable 1: the outer wall does not move when the gate is satisfied.
    let outer_delta = (min_x(&outer_on) - min_x(&outer_off)) as f64;
    assert!(
        outer_delta.abs() < TOLERANCE_MM as f64,
        "the precise gate must NOT move the outer wall (canonical outline shrink \
         is cancelled by wall_0_inset for the outermost bead, oracle delta 0.000); \
         observed shift {outer_delta} mm (off min-x={}, on min-x={})",
        min_x(&outer_off),
        min_x(&outer_on)
    );

    // Observable 2: the outer wall sits at ext_perimeter_width / 2 from the
    // model boundary under BOTH gates (oracle: 0.250 from a 0.5 mm width).
    let outer_on_from_boundary = (-HALF_SIDE_MM - min_x(&outer_on) as f64).abs();
    assert!(
        (outer_on_from_boundary - EXPECTED_OUTER_FROM_BOUNDARY_MM).abs() < TOLERANCE_MM as f64,
        "outer wall must sit {EXPECTED_OUTER_FROM_BOUNDARY_MM} mm from the model \
         boundary with the gate on; measured {outer_on_from_boundary} mm \
         (min-x={})",
        min_x(&outer_on)
    );

    // Observable 3: the inner wall moves inward by bump/2.
    let inner_delta = (min_x(&inner_on) - min_x(&inner_off)) as f64;
    assert!(
        (inner_delta - EXPECTED_INNER_DELTA_MM).abs() < TOLERANCE_MM as f64,
        "expected inner wall min-x to shift by {EXPECTED_INNER_DELTA_MM} mm when \
         precise_outer_wall is gated on, observed shift {inner_delta} mm \
         (off min-x={}, on min-x={})",
        min_x(&inner_off),
        min_x(&inner_on)
    );
}

/// AC-N2 (negative, default_off): `precise_outer_wall` unset (default false)
/// with `wall_sequence="InnerOuter"` must produce the same outer wall
/// placement as an explicit `precise_outer_wall=false` — i.e. no offset.
#[test]
fn precise_outer_wall_default_off_matches_explicit_off() {
    let config_default = make_config(None, Some("InnerOuter"));
    let config_explicit_off = make_config(Some(false), Some("InnerOuter"));

    let outer_default = run_and_get_outer_wall(&config_default);
    let outer_explicit_off = run_and_get_outer_wall(&config_explicit_off);

    assert!(
        (min_x(&outer_default) - min_x(&outer_explicit_off)).abs() < TOLERANCE_MM,
        "default (key absent) precise_outer_wall must match explicit false: \
         default min-x={}, explicit-off min-x={}",
        min_x(&outer_default),
        min_x(&outer_explicit_off)
    );
}

/// AC-N2 (negative, wall_sequence gate): `precise_outer_wall=true` combined
/// with `wall_sequence="OuterInner"` (not `InnerOuter`) must NOT apply any
/// offset — the outer wall placement must match the `precise_outer_wall=false`
/// baseline.
#[test]
fn precise_outer_wall_gated_off_when_wall_sequence_not_inner_outer() {
    let config_off = make_config(Some(false), Some("InnerOuter"));
    let config_wrong_sequence = make_config(Some(true), Some("OuterInner"));

    let outer_off = run_and_get_outer_wall(&config_off);
    let outer_wrong_sequence = run_and_get_outer_wall(&config_wrong_sequence);

    assert!(
        (min_x(&outer_wrong_sequence) - min_x(&outer_off)).abs() < TOLERANCE_MM,
        "precise_outer_wall=true with wall_sequence=OuterInner must not \
         apply any offset: off baseline min-x={}, wrong-sequence min-x={}",
        min_x(&outer_off),
        min_x(&outer_wrong_sequence)
    );
}
