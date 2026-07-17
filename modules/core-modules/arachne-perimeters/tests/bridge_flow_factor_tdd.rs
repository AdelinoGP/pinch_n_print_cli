//! TDD test for packet 149, Step 4: `bridge_flow` / `thick_bridges` wiring.
//!
//! `region.bridge_areas()` marks polygons that are bridge spans (packet 148,
//! AC-4 — `feature_flags[i].is_bridge` is already set per-vertex from this).
//! This packet wires the flow-rate side: every Outer/Inner wall vertex whose
//! `feature_flags[i].is_bridge == true` must get
//! `path.points[i].flow_factor == bridging_flow(bridge_flow, thick_bridges)`
//! (`slicer_core::flow::bridging_flow`); non-bridge vertices keep the
//! canonical default `flow_factor == 1.0`.
//!
//! Mirrors the harness in `arachne_parity_is_bridge_flag_tdd.rs`
//! (`make_config`/`make_region` shape) and `alternate_extra_wall_tdd.rs`
//! (native `run_perimeters` drive).
//!
//! OrcaSlicer ref: `LayerRegion.cpp:135` (`bridging_flow(frPerimeter,
//! thick_bridges)`).

use arachne_perimeters::ArachnePerimeters;
use slicer_core::flow::{bridging_flow, flow_to_width};
use slicer_ir::{ConfigView, LoopType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_config(bridge_flow: f32, thick_bridges: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("inner_wall_line_width", 0.4)
        .float("outer_wall_line_width", 0.4)
        .float("bridge_flow", bridge_flow as f64)
        .bool("thick_bridges", thick_bridges)
        .build()
}

/// 10mm square region with a 4mm x 4mm centered bridge area, overlapping at
/// least one wall segment (mirrors `arachne_parity_is_bridge_flag_tdd.rs`'s
/// own `make_region`).
fn make_region(side_mm: f32, bridge_side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .bridge_areas(vec![square_polygon(0.0, 0.0, bridge_side_mm)])
        .build()
}

/// AC (positive, `thick_bridges = false`): every bridge vertex gets
/// `flow_factor == bridge_flow` (0.7); every non-bridge vertex keeps the
/// canonical default `flow_factor == 1.0`.
#[test]
fn bridge_vertices_get_bridge_flow_ratio_when_thin() {
    let config = make_config(0.7, false);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 4.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    assert!(
        !output.wall_loops().is_empty(),
        "expected at least one wall loop to be emitted"
    );

    let mut found_bridge_vertex = false;
    for wall in output.wall_loops() {
        // is_bridge is only ever set on Outer/Inner walls (packet 148 AC-4).
        if !matches!(wall.loop_type, LoopType::Outer | LoopType::Inner) {
            continue;
        }
        for (j, flag) in wall.feature_flags.iter().enumerate() {
            let pt = &wall.path.points[j];
            if flag.is_bridge {
                found_bridge_vertex = true;
                assert!(
                    (pt.flow_factor - 0.7).abs() < f32::EPSILON,
                    "wall loop_type={:?} perimeter_index={} vertex {} at ({}, {}) mm: \
                     is_bridge=true, expected flow_factor == 0.7 (bridge_flow), got {}",
                    wall.loop_type,
                    wall.perimeter_index,
                    j,
                    pt.x,
                    pt.y,
                    pt.flow_factor
                );
            } else {
                assert!(
                    (pt.flow_factor - 1.0).abs() < f32::EPSILON,
                    "wall loop_type={:?} perimeter_index={} vertex {} at ({}, {}) mm: \
                     is_bridge=false, expected flow_factor == 1.0, got {}",
                    wall.loop_type,
                    wall.perimeter_index,
                    j,
                    pt.x,
                    pt.y,
                    pt.flow_factor
                );
            }
        }
    }

    assert!(
        found_bridge_vertex,
        "expected at least one is_bridge==true vertex to verify flow_factor against \
         (fixture must produce bridge vertices, or this test can never fail)"
    );
}

/// AC (D-104g CLOSED by packet 150, `thick_bridges = true`): bridge vertices'
/// flow_factor is the OrcaSlicer round-cross-section factor
/// `PI * dmr^2 / (4 * bead_width * layer_height)` (`dmr = nozzle_diameter *
/// sqrt(bridge_flow_ratio)`), not a stubbed constant `1.0`. Per-vertex bead
/// width varies along this fixture's medial axis (normal ~0.357mm beads and a
/// wide medial-axis closing bead ~7.14mm), so bridge vertices legitimately
/// get DIFFERENT factors — each vertex's expected factor is derived from its
/// own `pt.width` via `flow_to_width` (inverse of `line_width_to_spacing`) to
/// recover the raw mm bead width, mirroring the module's own call site.
#[test]
fn bridge_vertices_get_round_section_factor_when_thick_bridges_on() {
    const NOZZLE_DIAMETER_MM: f32 = 0.4;
    const LAYER_HEIGHT_MM: f32 = 0.2;

    let config = make_config(0.7, true);
    let module = ArachnePerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 4.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let mut found_bridge_vertex = false;
    for wall in output.wall_loops() {
        if !matches!(wall.loop_type, LoopType::Outer | LoopType::Inner) {
            continue;
        }
        for (j, flag) in wall.feature_flags.iter().enumerate() {
            let pt = &wall.path.points[j];
            if flag.is_bridge {
                found_bridge_vertex = true;
                let bead_flow_width_mm = flow_to_width(pt.width, LAYER_HEIGHT_MM);
                let expected = bridging_flow(
                    0.7,
                    true,
                    NOZZLE_DIAMETER_MM,
                    bead_flow_width_mm,
                    LAYER_HEIGHT_MM,
                );
                assert!(
                    (pt.flow_factor - expected).abs() < 1e-3,
                    "wall loop_type={:?} perimeter_index={} vertex {} at ({}, {}) mm: \
                     is_bridge=true with thick_bridges=true, expected flow_factor == {} \
                     (round-cross-section factor for bead width {}mm), got {}",
                    wall.loop_type,
                    wall.perimeter_index,
                    j,
                    pt.x,
                    pt.y,
                    expected,
                    bead_flow_width_mm,
                    pt.flow_factor
                );
            }
        }
    }

    assert!(
        found_bridge_vertex,
        "expected at least one is_bridge==true vertex to verify flow_factor against \
         (fixture must produce bridge vertices, or this test can never fail)"
    );

    // Strengthening assertion: lock the formula's physical values for a
    // uniform-width bead (independent of this fixture's per-vertex widths),
    // so a future regression can't silently pass by making all per-vertex
    // expectations self-consistent without matching the physical formula.
    // Sanity values per bridging_flow's own doc comment / OrcaSlicer's
    // Flow::bridging_flow (Flow.hpp/Flow.cpp).
    let uniform_bead_07 = bridging_flow(0.7, true, NOZZLE_DIAMETER_MM, 0.4, LAYER_HEIGHT_MM);
    assert!(
        (uniform_bead_07 - 1.0996).abs() < 1e-3,
        "bridging_flow(0.7, true, 0.4, 0.4, 0.2) expected ~1.0996, got {uniform_bead_07}"
    );
    let uniform_bead_10 = bridging_flow(1.0, true, NOZZLE_DIAMETER_MM, 0.4, LAYER_HEIGHT_MM);
    // Physical value is exactly pi/2 here (round-cross-section factor at
    // bead width == nozzle diameter); use the precise constant rather than
    // a truncated literal so clippy::approx_constant doesn't flag it.
    assert!(
        (uniform_bead_10 - std::f32::consts::FRAC_PI_2).abs() < 1e-3,
        "bridging_flow(1.0, true, 0.4, 0.4, 0.2) expected ~{}, got {uniform_bead_10}",
        std::f32::consts::FRAC_PI_2
    );
}
