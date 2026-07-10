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
use slicer_ir::{mm_to_units, ConfigView, LoopType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_config(bridge_flow: f32, thick_bridges: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 2)
        .float("optimal_width", mm_to_units(0.4_f32) as f64)
        .float("preferred_bead_width_outer", mm_to_units(0.4_f32) as f64)
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

/// AC (D-104g branch, `thick_bridges = true`): bridge vertices' flow_factor
/// is `1.0` (no flow reduction) instead of the `bridge_flow` ratio.
#[test]
fn bridge_vertices_keep_full_flow_when_thick_bridges_on() {
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
                assert!(
                    (pt.flow_factor - 1.0).abs() < f32::EPSILON,
                    "wall loop_type={:?} perimeter_index={} vertex {} at ({}, {}) mm: \
                     is_bridge=true with thick_bridges=true, expected flow_factor == 1.0 \
                     (D-104g), got {}",
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
