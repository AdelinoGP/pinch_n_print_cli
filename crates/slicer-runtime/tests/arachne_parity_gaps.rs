//! # Arachne parity audit 2026-07-09 — red tests against `parity/arachne`.
//!
//! Second-round audit deliverable, sibling to `arachne_parity.rs` (the
//! 2026-07-09-earlier audit whose gaps were largely closed by packets 148/149
//! and whose tests were rewritten as green regression locks). This file
//! contains ONLY the still-open gaps: **every test in this file fails on
//! purpose** on the current `parity/arachne` tree (`34ce576e`), panicking with
//! a message of the form:
//!
//! `PARITY GAP: <feature> | expected: <orcaslicer behavior> | got: <current
//! behavior> | ref: <OrcaSlicer path:line>`
//!
//! The failure message *is* the deliverable. Do not `#[ignore]`, weaken, or
//! delete these tests to get a green build — each one is closed by
//! implementing the named OrcaSlicer behavior, at which point the test body
//! already asserts the correct end state.
//!
//! **Framing:** PnP is a modular pipeline; `Layer::Perimeters` is filled by a
//! claim holder selected via the `wall_generator` config key
//! (`crates/slicer-scheduler/src/execution_plan.rs:182-260`). Gap categories
//! follow `arachne_parity.rs`:
//!
//! - **GAP_ARACHNE_PATH** — implemented in `classic-perimeters`/downstream but
//!   not in the `arachne-perimeters` path.
//! - **GAP_PIPELINE** — absent from the PnP pipeline as a whole.
//!
//! Canonical OrcaSlicer reference tree: `OrcaSlicerDocumented/src/libslic3r/`.
//! Full gap inventory: `docs/DEVIATION_LOG.md` (authoritative; the open Arachne
//! rows are the `D-104*` / `D-105*` / `D-112*` families).
//!
//! Coordinate convention (`docs/08_coordinate_system.md`): 1 unit = 100 nm =
//! 10⁻⁴ mm; OrcaSlicer scaled constants are divided by 100 when ported. All
//! config keys are snake_case.

#![allow(dead_code)]

/// The Arachne module manifest TOML (relative to this test file).
const ARACHNE_MANIFEST_TEXT: &str =
    include_str!("../../../modules/core-modules/arachne-perimeters/arachne-perimeters.toml");

/// Scheduler wall-generator selection source, for the spiral-vase dispatch gap.
const SCHEDULER_EXECUTION_PLAN_SRC: &str =
    include_str!("../../slicer-scheduler/src/execution_plan.rs");

/// Runtime run-loop source, for the spiral-vase dispatch gap.
const RUNTIME_RUN_SRC: &str = include_str!("../src/run.rs");

#[path = "fixtures/arachne_parity/mod.rs"]
mod fixtures;

use std::collections::BTreeSet;

use arachne_perimeters::ArachnePerimeters;
use slicer_ir::{ConfigView, LoopType, Point3WithWidth, WallLoop};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

// ---------------------------------------------------------------------------
// Shared helpers (mirroring arachne_parity.rs).
// ---------------------------------------------------------------------------

fn manifest() -> toml::Value {
    toml::from_str(ARACHNE_MANIFEST_TEXT).expect("arachne-perimeters.toml must parse as valid TOML")
}

fn manifest_has_config_key(key: &str) -> bool {
    manifest()
        .get("config")
        .and_then(|c| c.get("schema"))
        .and_then(|s| s.as_table())
        .map(|t| t.contains_key(key))
        .unwrap_or(false)
}

fn manifest_config_key_type(key: &str) -> String {
    manifest()
        .get("config")
        .and_then(|c| c.get("schema"))
        .and_then(|s| s.get(key))
        .and_then(|k| k.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("<key absent>")
        .to_string()
}

/// Drive `ArachnePerimeters::run_perimeters` natively via the `LayerModule`
/// trait and return the emitted wall loops.
fn run_walls(config: &ConfigView, regions: &[SliceRegionView], layer_index: u32) -> Vec<WallLoop> {
    let module = ArachnePerimeters::on_print_start(config).unwrap();
    let paint = PaintRegionLayerView::new(layer_index);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(layer_index, regions, &paint, &mut output, config)
        .unwrap();
    output.wall_loops().to_vec()
}

/// Baseline config: 0.4 mm beads, `wall_count` walls.
fn base_config(wall_count: i64) -> ConfigViewBuilder {
    ConfigViewBuilder::new()
        .int("wall_count", wall_count)
        .float("inner_wall_line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
}

/// A plain square region built from the shared parity fixture.
fn square_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(fixtures::square_mm(side_mm))
        .build()
}

/// Shoelace signed area (mm²) over a wall path; sign encodes winding.
fn signed_area_mm2(points: &[Point3WithWidth]) -> f64 {
    let mut acc = 0.0f64;
    for i in 0..points.len() {
        let p = &points[i];
        let q = &points[(i + 1) % points.len()];
        acc += (p.x as f64) * (q.y as f64) - (q.x as f64) * (p.y as f64);
    }
    acc / 2.0
}

/// The outermost (perimeter_index == 0) Outer wall of a run.
fn outer_wall(walls: &[WallLoop]) -> &WallLoop {
    walls
        .iter()
        .find(|w| w.perimeter_index == 0 && w.loop_type == LoopType::Outer)
        .expect("a perimeter_index == 0 Outer wall loop must be emitted")
}

/// Distinct perimeter indices among Outer/Inner walls.
fn wall_index_set(walls: &[WallLoop]) -> BTreeSet<u32> {
    walls
        .iter()
        .filter(|w| matches!(w.loop_type, LoopType::Outer | LoopType::Inner))
        .map(|w| w.perimeter_index)
        .collect()
}

fn min_x_mm(wall: &WallLoop) -> f32 {
    wall.path
        .points
        .iter()
        .map(|p| p.x)
        .fold(f32::INFINITY, f32::min)
}

// ===========================================================================
// GAP_PIPELINE: wall_direction (CCW/CW loop winding control)
// ===========================================================================

/// GAP_PIPELINE: OrcaSlicer exposes `wall_direction`
/// (`PrintConfig.cpp:2188-2198`, enum CounterClockwise/Clockwise, default
/// CCW) and applies it in the Arachne branch via
/// `ExtrusionLoop::make_counter_clockwise/make_clockwise`
/// (`PerimeterGenerator.cpp:527-545`), with holes always wound opposite the
/// contour. PnP has ZERO readers of `wall_direction` anywhere in `crates/` or
/// `modules/` and the key is not registered in `arachne-perimeters.toml`, so
/// contour winding cannot be controlled.
#[test]
fn arachne_parity_pipeline_wall_direction_controls_winding() {
    let key_registered = manifest_has_config_key("wall_direction");

    let regions = vec![square_region(10.0, 0.2)];
    let ccw_config = base_config(2)
        .string("wall_direction", "counter_clockwise")
        .build();
    let cw_config = base_config(2).string("wall_direction", "clockwise").build();

    let ccw_walls = run_walls(&ccw_config, &regions, 1);
    let cw_walls = run_walls(&cw_config, &regions, 1);
    let ccw_area = signed_area_mm2(&outer_wall(&ccw_walls).path.points);
    let cw_area = signed_area_mm2(&outer_wall(&cw_walls).path.points);
    let winding_flipped = ccw_area * cw_area < 0.0;

    assert!(
        key_registered && winding_flipped,
        "PARITY GAP: wall_direction winding control | expected: wall_direction \
         config key registered (PrintConfig.cpp:2188-2198, default \
         CounterClockwise) and flipping it reverses the outer contour winding \
         via make_counter_clockwise/make_clockwise \
         (PerimeterGenerator.cpp:527-545) | got: key registered in \
         arachne-perimeters.toml: {key_registered}; outer-wall signed area \
         under counter_clockwise = {ccw_area:.4} mm² vs clockwise = \
         {cw_area:.4} mm² (winding flipped: {winding_flipped}) — the key has \
         zero readers anywhere in crates/ or modules/ | ref: \
         PerimeterGenerator.cpp:527-545"
    );
}

// ===========================================================================
// GAP_PIPELINE: only_one_wall_first_layer
// ===========================================================================

/// GAP_PIPELINE: OrcaSlicer exposes `only_one_wall_first_layer`
/// (`PrintConfig.cpp:1513-1517`, coBool default false) and, in the Arachne
/// branch, forces `loop_number = 0` on the first printed layer
/// (`PerimeterGenerator.cpp:2137-2139`). PnP does not register the key in
/// `arachne-perimeters.toml` and the module never reduces wall count on
/// layer 0.
#[test]
fn arachne_parity_pipeline_only_one_wall_first_layer_forces_single_wall() {
    let key_registered = manifest_has_config_key("only_one_wall_first_layer");

    let config = base_config(3)
        .bool("only_one_wall_first_layer", true)
        .build();
    let regions = vec![square_region(10.0, 0.2)];
    let walls = run_walls(&config, &regions, 0);
    let indices = wall_index_set(&walls);

    assert!(
        key_registered && indices.len() == 1,
        "PARITY GAP: only_one_wall_first_layer | expected: with \
         only_one_wall_first_layer=true and wall_count=3, layer 0 emits \
         exactly one wall (loop_number forced to 0, \
         PerimeterGenerator.cpp:2137-2139; key defined at \
         PrintConfig.cpp:1513-1517) | got: key registered in \
         arachne-perimeters.toml: {key_registered}; layer-0 distinct \
         perimeter indices: {indices:?} (expected exactly {{0}}) | ref: \
         PerimeterGenerator.cpp:2137-2139"
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH: only_one_wall_top read but behaviorally inert (D-104d)
// ===========================================================================

/// GAP_ARACHNE_PATH: OrcaSlicer's Arachne branch forces a single wall on the
/// topmost layer when `only_one_wall_top` is set
/// (`PerimeterGenerator.cpp:2140-2144`, gate `upper_slices == nullptr`), and
/// for non-topmost top surfaces runs a SECOND `Arachne::WallToolPaths` pass
/// over the non-top region with `inner_loop_number + 2` walls, merging with
/// `inset_idx` renumbering (`PerimeterGenerator.cpp:2160-2246`,
/// `:2242`). `arachne-perimeters` reads the key but explicitly discards it
/// (`modules/core-modules/arachne-perimeters/src/lib.rs:305-306`,
/// `let _ = only_one_wall_top;` — deferred under
/// D-104d-MIN-WIDTH-TOP-SURFACE-NONE), so a top region still gets the full
/// wall count.
#[test]
fn arachne_parity_arachne_path_only_one_wall_top_forces_single_wall_on_top() {
    let config = base_config(3).bool("only_one_wall_top", true).build();
    let mut region = square_region(10.0, 1.0);
    // Mark the region as the topmost shell the way the PnP IR expresses it
    // (SliceRegionView top-shell metadata; Orca's equivalent gate is
    // upper_slices == nullptr).
    region.set_top_shell_index(Some(0));
    region.set_top_solid_fill(vec![fixtures::square_mm(10.0)]);

    let walls = run_walls(&config, &[region], 5);
    let indices = wall_index_set(&walls);

    assert!(
        indices.len() == 1,
        "PARITY GAP: only_one_wall_top | expected: with only_one_wall_top=true \
         and wall_count=3, a topmost region (top_shell_index == Some(0)) emits \
         exactly one wall — Orca forces loop_number = 0 on the topmost layer \
         (PerimeterGenerator.cpp:2140-2144) and re-runs Arachne for remaining \
         inner walls on non-top area only (PerimeterGenerator.cpp:2160-2246) | \
         got: distinct perimeter indices {indices:?} — the module reads \
         only_one_wall_top and discards it (arachne-perimeters/src/lib.rs:\
         305-306, D-104d deferred) | ref: PerimeterGenerator.cpp:2140-2246"
    );
}

// ===========================================================================
// GAP_PIPELINE: wall gap uses Flow spacing, not raw width (D-105)
// ===========================================================================

/// GAP_PIPELINE: OrcaSlicer feeds Flow **spacing** values into Arachne, not
/// raw widths: `bead_width_0 = ext_perimeter_spacing`
/// (`PerimeterGenerator.cpp:2129`) and the `WallToolPaths` constructor
/// receives `perimeter_spacing` (`PerimeterGenerator.cpp:2172-2173`), where
/// spacing = layer-height-dependent `Flow::spacing()`
/// (`Flow.hpp:67`, computed as width − layer_height·(1 − π/4) for a rounded
/// rectangle cross-section; `perimeter_spacing =
/// perimeter_flow.scaled_spacing()` at `PerimeterGenerator.cpp:578`). PnP
/// passes raw `optimal_width` (0.4 mm) with no `layer_height` awareness
/// (zero readers in `arachne-perimeters/src/lib.rs`;
/// `slicer_core::flow::line_width_to_spacing` exists but is unwired —
/// deviation D-105-FLOW-NOT-WIRED), so adjacent wall centerlines sit one
/// full width apart instead of one spacing apart, over-spacing every wall
/// pair by layer_height·(1 − π/4) ≈ 0.0429 mm at 0.2 mm layers.
#[test]
fn arachne_parity_pipeline_wall_gap_uses_flow_spacing_not_width() {
    const LAYER_HEIGHT_MM: f64 = 0.2;
    const WIDTH_MM: f64 = 0.4;
    // Flow::rounded_rectangle_extrusion_spacing: w - h * (1 - PI/4).
    let expected_gap_mm = WIDTH_MM - LAYER_HEIGHT_MM * (1.0 - std::f64::consts::PI / 4.0);

    let config = base_config(2)
        .float("layer_height", LAYER_HEIGHT_MM)
        .build();
    let regions = vec![square_region(10.0, 0.2)];
    let walls = run_walls(&config, &regions, 1);

    let wall0 = walls
        .iter()
        .find(|w| w.perimeter_index == 0)
        .expect("perimeter_index 0 wall must exist");
    let wall1 = walls
        .iter()
        .find(|w| w.perimeter_index == 1)
        .expect("perimeter_index 1 wall must exist");
    let observed_gap_mm = (min_x_mm(wall1) - min_x_mm(wall0)) as f64;

    assert!(
        (observed_gap_mm - expected_gap_mm).abs() < 0.02,
        "PARITY GAP: wall gap uses Flow spacing | expected: adjacent wall \
         centerlines one Flow spacing apart, ≈{expected_gap_mm:.4} mm for \
         0.4 mm width at 0.2 mm layer height (bead_width_0 = \
         ext_perimeter_spacing, PerimeterGenerator.cpp:2129; WallToolPaths \
         receives perimeter_spacing = perimeter_flow.scaled_spacing(), \
         PerimeterGenerator.cpp:578,2172-2173; Flow.hpp:67) | got: centerline \
         gap {observed_gap_mm:.4} mm — raw optimal_width is used, \
         layer_height is never read, and \
         slicer_core::flow::line_width_to_spacing is unwired \
         (D-105-FLOW-NOT-WIRED) | ref: PerimeterGenerator.cpp:2129"
    );
}

// ===========================================================================
// GAP_PIPELINE: thick_bridges bridging flow is a 1.0 stub (D-104g)
// ===========================================================================

/// GAP_PIPELINE: OrcaSlicer's Arachne branch extrudes overhang/bridge
/// perimeters with `overhang_flow = bridging_flow(frPerimeter,
/// thick_bridges)` (`LayerRegion.cpp:135`); with `thick_bridges` on this is a
/// round-cross-section flow of thread diameter `dmr`
/// (`LayerRegion.cpp:31-50`; `Flow::bridging_flow(dmr, nozzle) → width =
/// height = dmr`, `Flow.hpp:106`), whose volume-per-mm exceeds the flat
/// 0.4×0.2 mm bead by ≈π·d²/4 ÷ (w·h) ≈ 1.57×. PnP's
/// `slicer_core::flow::bridging_flow(bridge_flow, thick_bridges)` returns a
/// hardcoded `1.0` for the `thick_bridges == true` branch
/// (`crates/slicer-core/src/flow.rs:85-92`, deviation
/// D-104g-FLOW-FACTOR-PERVERTEX-DIVERGENCE), so bridge vertices get no flow
/// adjustment at all.
#[test]
fn arachne_parity_pipeline_thick_bridges_flow_factor_not_stubbed_to_one() {
    let config = base_config(2)
        .float("bridge_flow", 1.0)
        .bool("thick_bridges", true)
        .build();
    // Packet-151 fixture correction: under corrected max_bead_count = 2*wall_count
    // (= 4 for wall_count=2), the emitted Outer/Inner walls sit at half-widths
    // ~5.0 / ~4.6 mm, well outside a 4×4 centred bridge area. The 4×4 area
    // therefore intersected no wall vertices, so no is_bridge flag was ever set
    // and the fixture guard tripped. Enlarging to a 12×12 centred area (mirrors
    // the `native_bridge_region` 15-lock fixture fix) guarantees wall vertices
    // fall strictly inside the bridge region so the real flow_factor assertion
    // is exercised.
    let mut region = square_region(10.0, 0.2);
    region.set_bridge_areas(vec![fixtures::square_mm(12.0)]);

    let walls = run_walls(&config, &[region], 1);
    let bridge_flow_factors: Vec<f32> = walls
        .iter()
        .filter(|w| matches!(w.loop_type, LoopType::Outer | LoopType::Inner))
        .flat_map(|w| {
            w.feature_flags
                .iter()
                .zip(&w.path.points)
                .filter(|(f, _)| f.is_bridge)
                .map(|(_, p)| p.flow_factor)
        })
        .collect();

    assert!(
        !bridge_flow_factors.is_empty(),
        "fixture must produce is_bridge vertices (it does under packet 148 AC-4), \
         or this test can never fail"
    );
    assert!(
        bridge_flow_factors.iter().any(|ff| (ff - 1.0).abs() > 0.05),
        "PARITY GAP: thick_bridges bridging flow | expected: bridge vertices \
         carry a real round-cross-section bridging flow factor (≈1.57 for a \
         0.4 mm thread over a 0.4×0.2 mm flat bead; Flow::bridging_flow, \
         Flow.hpp:106; LayerRegion.cpp:31-50,135) when thick_bridges=true | \
         got: every is_bridge vertex has flow_factor == 1.0 because \
         slicer_core::flow::bridging_flow's thick_bridges branch is a \
         hardcoded 1.0 stub (crates/slicer-core/src/flow.rs:85-92, \
         D-104g) — observed factors: {bridge_flow_factors:?} | ref: \
         LayerRegion.cpp:135"
    );
}

// ===========================================================================
// GAP_PIPELINE: no percent / float-or-percent config type (D-104h)
// ===========================================================================

/// GAP_PIPELINE: OrcaSlicer's Arachne keys are percent-typed relative to
/// nozzle diameter or wall width — `min_width_top_surface` is
/// `coFloatOrPercent` default 300% of inner wall width
/// (`PrintConfig.cpp:1498-1511`), `min_feature_size` is `coPercent` default
/// 25% of nozzle diameter (`PrintConfig.cpp:7217-7226`),
/// `wall_transition_length` is `coPercent` default 100%
/// (`PrintConfig.cpp:7169-7178`). PnP has no percent config type at all
/// (deviation D-104h-NO-PERCENT-CONFIG-TYPE): these keys are declared
/// `type = "float"` with pre-resolved absolute defaults
/// (`arachne-perimeters.toml:38-42,68-72,257-261`), so changing the nozzle
/// diameter silently leaves them stale instead of rescaling.
#[test]
fn arachne_parity_pipeline_percent_config_type_for_arachne_keys() {
    let offending: Vec<(&str, String)> = [
        "min_width_top_surface",
        "min_feature_size",
        "wall_transition_length",
    ]
    .iter()
    .map(|k| (*k, manifest_config_key_type(k)))
    .filter(|(_, t)| t != "percent" && t != "float_or_percent")
    .collect();

    assert!(
        offending.is_empty(),
        "PARITY GAP: percent config type | expected: min_width_top_surface \
         (coFloatOrPercent, 300%, PrintConfig.cpp:1498-1511), min_feature_size \
         (coPercent, 25%, PrintConfig.cpp:7217-7226) and wall_transition_length \
         (coPercent, 100%, PrintConfig.cpp:7169-7178) declared with a \
         percent-relative config type so they rescale with nozzle diameter / \
         wall width | got: no percent config type exists (D-104h); keys are \
         pre-resolved absolute floats: {offending:?} | ref: \
         PrintConfig.cpp:1498-1511"
    );
}

// ===========================================================================
// GAP_PIPELINE: overhang_reverse registered but behaviorally inert (D-104c)
// ===========================================================================

/// GAP_PIPELINE: with `detect_overhang_wall` off and `overhang_reverse` on,
/// OrcaSlicer unconditionally marks contour and holes "steep" and reverses
/// wall loop direction on odd layers (`PerimeterGenerator.cpp:422-429`,
/// steep-overhang detection at `:58-98`, reversal applied in
/// `traverse_extrusions` `:370-523`); `overhang_reverse_threshold`
/// (coFloatOrPercent) tunes detection. In PnP the keys `overhang_reverse` /
/// `overhang_reverse_internal_only` / `detect_overhang_wall` are registered
/// in `arachne-perimeters.toml` but have ZERO readers in
/// `arachne-perimeters/src/lib.rs` (D-104c registration-only, behavior
/// deferred), and `overhang_reverse_threshold` is not registered at all —
/// toggling `overhang_reverse` changes nothing.
#[test]
fn arachne_parity_pipeline_overhang_reverse_flips_odd_layer_walls() {
    let threshold_registered = manifest_has_config_key("overhang_reverse_threshold");

    let regions = vec![square_region(10.0, 0.4)];
    let reversed_config = base_config(2)
        .bool("detect_overhang_wall", false)
        .bool("overhang_reverse", true)
        .build();
    let normal_config = base_config(2)
        .bool("detect_overhang_wall", false)
        .bool("overhang_reverse", false)
        .build();

    // Odd layer: Orca only reverses on odd layers.
    let reversed_walls = run_walls(&reversed_config, &regions, 1);
    let normal_walls = run_walls(&normal_config, &regions, 1);
    let reversed_area = signed_area_mm2(&outer_wall(&reversed_walls).path.points);
    let normal_area = signed_area_mm2(&outer_wall(&normal_walls).path.points);
    let direction_flipped = reversed_area * normal_area < 0.0;

    assert!(
        threshold_registered && direction_flipped,
        "PARITY GAP: overhang_reverse | expected: with detect_overhang_wall=\
         false and overhang_reverse=true, odd-layer walls print in reversed \
         direction (contour/holes unconditionally marked steep, \
         PerimeterGenerator.cpp:422-429; detect_steep_overhang :58-98) and \
         overhang_reverse_threshold is a registered config key | got: \
         overhang_reverse_threshold registered: {threshold_registered}; \
         outer-wall signed area with overhang_reverse=true = \
         {reversed_area:.4} mm² vs false = {normal_area:.4} mm² (flipped: \
         {direction_flipped}) — the registered keys have zero readers in the \
         module (D-104c registration-only) | ref: \
         PerimeterGenerator.cpp:422-429"
    );
}

// ===========================================================================
// GAP_PIPELINE: spiral vase does not force the classic generator
// ===========================================================================

/// GAP_PIPELINE: OrcaSlicer dispatches Arachne only when `wall_generator ==
/// Arachne && !spiral_mode` (`LayerRegion.cpp:138-141`) — spiral vase always
/// falls back to the classic generator because a continuously-rising
/// single-wall spiral cannot consume variable-width wall stacks. PnP's
/// generator selection (`dedup_same_claim_modules_with_wall_generator`,
/// `crates/slicer-scheduler/src/execution_plan.rs:256-261`, called from
/// `crates/slicer-runtime/src/run.rs:342`) keys ONLY off `wall_generator`
/// and has no spiral-vase input anywhere in the selection path, so
/// `wall_generator=arachne` + `spiral_vase=true` still selects
/// `arachne-perimeters`.
#[test]
fn arachne_parity_pipeline_spiral_vase_forces_classic_generator() {
    let scheduler_spiral_aware = SCHEDULER_EXECUTION_PLAN_SRC
        .to_lowercase()
        .contains("spiral");
    let runtime_spiral_aware = RUNTIME_RUN_SRC.to_lowercase().contains("spiral");

    assert!(
        scheduler_spiral_aware || runtime_spiral_aware,
        "PARITY GAP: spiral vase forces classic | expected: generator \
         selection falls back to the classic perimeter generator whenever \
         spiral vase mode is active, regardless of wall_generator \
         (LayerRegion.cpp:138-141: `wall_generator == Arachne && \
         !spiral_mode`) | got: neither \
         slicer-scheduler/src/execution_plan.rs (dedup_same_claim_modules_\
         with_wall_generator, :256-261) nor slicer-runtime/src/run.rs (:342) \
         mentions spiral at all — wall_generator=arachne + spiral_vase=true \
         still selects arachne-perimeters | ref: LayerRegion.cpp:138-141"
    );
}

// ===========================================================================
// GAP_PIPELINE: wall_maximum_resolution / wall_maximum_deviation unregistered
// ===========================================================================

/// GAP_PIPELINE: OrcaSlicer exposes `wall_maximum_resolution` (default
/// 0.5 mm) and `wall_maximum_deviation` (default 0.025 mm) as user config
/// (`PrintConfig.cpp:7242-7263`), feeding outline prep
/// (`WallToolPaths.cpp:487-503`) and `simplifyToolPaths`
/// (`WallToolPaths.cpp:702-719`). PnP's pipeline has the internal equivalents
/// (`ArachneParams.smallest_line_segment_squared` /
/// `allowed_error_distance_squared`,
/// `crates/slicer-core/src/arachne/pipeline.rs:149-154`) but neither key is
/// registered in `arachne-perimeters.toml` nor read by the module, so the
/// simplification tolerances are compile-time constants instead of user
/// config.
#[test]
fn arachne_parity_pipeline_wall_max_resolution_deviation_registered() {
    let missing: Vec<&str> = ["wall_maximum_resolution", "wall_maximum_deviation"]
        .iter()
        .copied()
        .filter(|k| !manifest_has_config_key(k))
        .collect();

    assert!(
        missing.is_empty(),
        "PARITY GAP: wall_maximum_resolution/deviation config | expected: both \
         keys registered and wired into the simplification tolerances \
         (PrintConfig.cpp:7242-7263; consumed by WallToolPaths.cpp:487-503 and \
         :702-719) | got: missing from arachne-perimeters.toml: {missing:?} — \
         ArachneParams.smallest_line_segment_squared / \
         allowed_error_distance_squared exist (pipeline.rs:149-154) but are \
         never config-driven | ref: PrintConfig.cpp:7242-7263"
    );
}

// ===========================================================================
// GAP_ARACHNE_PATH: removeSmallLines top/bottom-layer exception missing
// ===========================================================================

/// GAP_ARACHNE_PATH: OrcaSlicer's `removeSmallLines` drops odd unclosed
/// walls shorter than `min_width * min_length_factor`, EXCEPT on top/bottom
/// layers where a fixed `min_width / 2` is used instead so surface-visible
/// thin walls survive (`WallToolPaths.cpp:684-700`; the
/// `is_top_or_bottom_layer` flag covers BOTH the bottom layer and the
/// topmost layer, set at `PerimeterGenerator.cpp:2153-2154` from
/// `is_bottom_layer || is_topmost_layer`). PnP's port
/// (`crates/slicer-core/src/arachne/remove_small.rs:44-80`) keys the lenient
/// threshold on `is_initial_layer` (layer 0) only — the function has no
/// top-layer input at all, and its caller
/// (`run_arachne_pipeline(polygons, &params, is_initial_layer)`) can only
/// express "layer 0". A short odd center-line on the TOPMOST layer is
/// therefore dropped where Orca keeps it, leaving visible top-surface gaps.
#[test]
fn arachne_parity_arachne_path_remove_small_lines_top_layer_exception() {
    use slicer_ir::{ExtrusionJunction, ExtrusionLine};
    use slicer_sdk::host::{generate_arachne_walls, ArachneParams};

    // One odd, unclosed 3 mm center line of uniform 0.4 mm width.
    // Topmost-layer threshold per Orca = min_width/2 = 0.2 mm → KEEP.
    // Normal-layer threshold = min_width·min_length_factor = 8 mm → drop.
    let junction = |x: f32, width: f32| ExtrusionJunction {
        p: Point3WithWidth {
            x,
            y: 0.0,
            z: 0.0,
            width,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        perimeter_index: 0,
    };
    let center_line = ExtrusionLine {
        junctions: vec![junction(0.0, 0.4), junction(3.0, 0.4)],
        inset_idx: 0,
        is_odd: true,
        is_closed: false,
    };

    // This is what the pipeline passes for the TOPMOST layer today: the only
    // layer-type input is `is_initial_layer`, which is false for any layer
    // above 0 (pipeline.rs:144, module sets it to `layer_index == 0`).
    let surviving = slicer_core::arachne::remove_small_lines(
        vec![center_line],
        20.0,  // min_length_factor
        0.4,   // nominal min_width (unused by the per-line threshold)
        false, // is_initial_layer — a topmost layer can never set this
        true,  // is_top_or_bottom_layer — the TOPMOST layer sets this
    );

    assert!(
        !surviving.is_empty(),
        "PARITY GAP: removeSmallLines top/bottom-layer exception | expected: \
         a 3 mm odd unclosed center line survives small-line removal on the \
         TOPMOST layer — Orca switches the threshold from \
         min_width*min_length_factor (8 mm here) to min_width/2 (0.2 mm) \
         whenever is_top_or_bottom_layer, i.e. bottom OR topmost layer \
         (WallToolPaths.cpp:684-700; flag set from is_bottom_layer || \
         is_topmost_layer at PerimeterGenerator.cpp:2153-2154) | got: line \
         dropped — remove_small_lines keys the lenient threshold on \
         is_initial_layer (layer 0) only \
         (crates/slicer-core/src/arachne/remove_small.rs:44-80) and neither \
         it nor run_arachne_pipeline has any topmost-layer input | ref: \
          WallToolPaths.cpp:684-700"
    );

    // End-to-end wiring probe (packet 152 fix): the module's `params` now
    // carries `is_topmost_layer` into the SDK mirror and the native bridge
    // forwards it into `run_arachne_pipeline`, which derives
    // `is_top_or_bottom_layer` (pipeline.rs:398 = is_topmost_layer ||
    // is_bottom_layer) → `remove_small_lines` leniency. Before this fix the
    // bridge hardcoded both flags false, so this would not even compile
    // against the old SDK `ArachneParams`; here the topmost flag must flow
    // through the exact `generate_arachne_walls` bridge the module calls.
    let topmost_params = ArachneParams {
        is_topmost_layer: true,
        ..Default::default()
    };
    let probe_poly = slicer_ir::ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                slicer_ir::Point2::from_mm(0.0, 0.0),
                slicer_ir::Point2::from_mm(10.0, 0.0),
                slicer_ir::Point2::from_mm(10.0, 10.0),
                slicer_ir::Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![],
    };
    let probed = generate_arachne_walls(&[probe_poly], &topmost_params)
        .expect("topmost-flag must forward through the SDK bridge into the core pipeline");
    assert!(
        !probed.0.is_empty(),
        "G10 end-to-end wiring broken: topmost-layer params did not reach the \
         core Arachne pipeline via the module's generate_arachne_walls bridge"
    );
}

/// AC-1 (packet 151): wall_count → max_bead_count = 2 × wall_count wiring.
/// The toml registers both keys; the module reads wall_count when
/// max_bead_count is absent (get_int → None, since ConfigView never merges
/// schema defaults). On a 10 mm square with wall_count=3, the distinct
/// Outer/Inner perimeter_index set must be {0,1,2} (3 walls), NOT
/// {0,1,2,3,4} (the legacy max_bead_count=9 collapse).
#[test]
fn arachne_parity_wall_count_wires_max_bead_count() {
    let regions = vec![square_region(10.0, 0.2)];
    let config = base_config(3).build();
    let walls = run_walls(&config, &regions, 1);
    let indices = wall_index_set(&walls);
    assert_eq!(
        indices,
        BTreeSet::from([0u32, 1, 2]),
        "wall_count=3 must yield exactly {{0,1,2}} walls; got {:?} — wall_count \
         is not being read by arachne_params_from_config, so the module is still \
         falling back to defaults.max_bead_count=9 (Orca WallToolPaths.cpp:525: \
         max_bead_count = 2 * inset_count)",
        indices
    );
}
