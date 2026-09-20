//! TDD test for arachne per-vertex parity packet 148, Step 6: `precise_outer_wall`.
//!
//! OrcaSlicer's `PerimeterGenerator.cpp:2146-2158` computes
//! `apply_precise_outer_wall = precise_outer_wall && wall_sequence ==
//! InnerOuter`, and when true, insets the outer wall's toolpath location by
//! `-(ext_perimeter_width/2 - ext_perimeter_spacing/2)` via
//! `OuterWallInsetBeadingStrategy`. This test drives
//! `ArachnePerimeters::run_perimeters` end-to-end and asserts the outer
//! wall's emitted path actually moves by that exact delta when the config
//! gate is satisfied, and does not move at all when it isn't (default-off,
//! and wall_sequence != InnerOuter).
//!
//! `preferred_bead_width_outer` stands in for `ext_perimeter_width` and
//! `optimal_width` stands in for `ext_perimeter_spacing` — the two width-like
//! quantities `arachne_params_from_config` already reads from the manifest
//! (see that function's doc comment); there is no separate "spacing" config
//! key in this module.

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, WallLoop};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

const OUTER_WIDTH_MM: f32 = 0.5;
const SPACING_WIDTH_MM: f32 = 0.4;
/// Fixture layer height (mm) — the manifest default for `layer_height`, set
/// explicitly in `make_config` (the read is now `require_*`; see below).
const LAYER_HEIGHT_MM: f64 = 0.2;
/// Expected outer-wall min-x shift (mm) when the gate is satisfied.
///
/// Spacing-correct Orca-parity inset; was `-0.05` (raw-width era, computed
/// from `-(ext_perimeter_width/2 - ext_perimeter_spacing/2)` using the raw
/// configured widths directly). Corrected in packet 150 (D-105): both
/// `optimal_width` and `preferred_bead_width_outer` are converted
/// width->spacing via `line_width_to_spacing` before the beading engine sees
/// them (AC-3), so the observed toolpath shift collapses to a width-independent
/// function of layer height alone: `-(layer_height * (1 - PI/4)) / 2`. Derived
/// in-test from `LAYER_HEIGHT_MM` (rot-proof, self-documenting) rather than
/// hardcoded, and matches the `arachne_parity.rs` lock for this same formula.
const EXPECTED_OFFSET_MM: f64 = -(LAYER_HEIGHT_MM * (1.0 - std::f64::consts::PI / 4.0)) / 2.0;
const TOLERANCE_MM: f32 = 1e-3;

/// Builds a config with distinct `optimal_width`/`preferred_bead_width_outer`
/// (so the gated offset is nonzero and observable), plus the wall-sequencing
/// keys under test. `precise_outer_wall`/`wall_sequence` default to their
/// manifest defaults (false / "InnerOuter") when `None` — item-11 migration
/// (packet 06): the classified reads are now `require_*`, so the view always
/// holds the key; "absent" is expressed as the manifest-default value.
fn make_config(precise_outer_wall: Option<bool>, wall_sequence: Option<&str>) -> ConfigView {
    let mut builder = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("inner_wall_line_width", SPACING_WIDTH_MM as f64)
        .float("outer_wall_line_width", OUTER_WIDTH_MM as f64)
        // Required-read baseline (packet 06 5c', item 11): the classified
        // reads in run_perimeters/arachne_params_from_config are now
        // require_*; the view holds every key those paths read, at
        // manifest-default values. line_width holds its post-expansion
        // default (1.125 x nozzle_diameter): the raw 0 is the auto sentinel
        // expanded at Phase B and cannot survive the D-162 spacing gate.
        .float("layer_height", LAYER_HEIGHT_MM)
        .float("nozzle_diameter", 0.4)
        .float("line_width", 0.45)
        .float("bridge_line_width", 0.0)
        .float("initial_layer_line_width", 0.0)
        .int("extra_perimeters", 0)
        .int("support_raft_layers", 0)
        .bool("only_one_wall_top", false)
        .string("wall_direction", "counter_clockwise")
        .bool("alternate_extra_wall", false)
        .bool("spiral_vase", false)
        .float("sparse_infill_density", 20.0)
        .bool("only_one_wall_first_layer", false)
        .bool("detect_overhang_wall", true)
        .bool("overhang_reverse", false)
        .bool("overhang_reverse_internal_only", false)
        .float("overhang_reverse_threshold", 0.0)
        .float("bridge_flow", 1.0)
        .bool("thick_bridges", false)
        .float("seam_candidate_angle_threshold_deg", 30.0);
    builder = builder.bool(
        "precise_outer_wall",
        precise_outer_wall.unwrap_or(false), // manifest default
    );
    builder = builder.string(
        "wall_sequence",
        wall_sequence.unwrap_or("InnerOuter"), // manifest default
    );
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

/// Runs the module end-to-end and returns the outer wall (`perimeter_index
/// == 0`) `WallLoop`.
fn run_and_get_outer_wall(config: &ConfigView) -> WallLoop {
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
        .find(|w| w.perimeter_index == 0)
        .expect("a wall loop with perimeter_index == 0 must be emitted")
        .clone()
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

/// AC-8 (positive): `precise_outer_wall=true` + `wall_sequence="InnerOuter"`
/// must shift the outer wall's toolpath by exactly `EXPECTED_OFFSET_MM`
/// relative to the same fixture with `precise_outer_wall=false`.
#[test]
fn precise_outer_wall_insets_outer_wall_by_expected_delta() {
    let config_off = make_config(Some(false), Some("InnerOuter"));
    let config_on = make_config(Some(true), Some("InnerOuter"));

    let outer_off = run_and_get_outer_wall(&config_off);
    let outer_on = run_and_get_outer_wall(&config_on);

    let observed_delta = (min_x(&outer_on) - min_x(&outer_off)) as f64;

    assert!(
        (observed_delta - EXPECTED_OFFSET_MM).abs() < TOLERANCE_MM as f64,
        "expected outer wall min-x to shift by {EXPECTED_OFFSET_MM} mm when \
         precise_outer_wall is gated on, observed shift {observed_delta} mm \
         (off min-x={}, on min-x={})",
        min_x(&outer_off),
        min_x(&outer_on)
    );
}

/// AC-N2 (negative, default_off): the manifest default of `precise_outer_wall`
/// (false — the migrated form of the former "key absent" path) must produce
/// the same outer wall placement as a non-gated control: `precise_outer_wall
/// =true` with `wall_sequence="OuterInner"` (the inner-outer gate can never
/// fire). Comparing against the distinct non-gated control keeps the claim
/// falsifiable now that the old "absent vs explicit false" distinction
/// collapsed into a single default-valued view (packet 06 item 11).
#[test]
fn precise_outer_wall_default_off_matches_explicit_off() {
    let config_default = make_config(None, Some("InnerOuter"));
    let config_ungated = make_config(Some(true), Some("OuterInner"));

    let outer_default = run_and_get_outer_wall(&config_default);
    let outer_ungated = run_and_get_outer_wall(&config_ungated);

    assert!(
        (min_x(&outer_default) - min_x(&outer_ungated)).abs() < TOLERANCE_MM,
        "default (manifest-value false) precise_outer_wall must match the \
         non-gated control: default min-x={}, ungated min-x={}",
        min_x(&outer_default),
        min_x(&outer_ungated)
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
