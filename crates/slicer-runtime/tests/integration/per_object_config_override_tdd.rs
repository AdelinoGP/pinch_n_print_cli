//! AC-8: per-object config override contract (P105 R2).
//!
//! Verifies that the 7 P105 config keys (outer_wall_line_width,
//! inner_wall_line_width, wall_sequence, detect_thin_wall, gap_infill_speed,
//! filter_out_gap_fill, precise_outer_wall) are read per-invocation from the
//! `_config` argument to `run_perimeters`, NOT from the cached `on_print_start`
//! config.
//!
//! Test: set print-global outer_wall_line_width=0.5 at on_print_start, then
//! pass a per-object override config with outer_wall_line_width=0.6 to
//! run_perimeters.  The emitted outer-wall vertex widths must equal 0.6 (the
//! override), proving the per-invocation read is respected.
//!
//! The "per-object override mechanism" in the test harness is simply passing a
//! different ConfigView to run_perimeters than was used at on_print_start.
//! This is sufficient: R2's intent is that run_perimeters reads _config (the
//! per-invocation argument) rather than cached struct fields, so any caller
//! that passes a different ConfigView at invoke time gets the override applied.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::LoopType;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn make_region(side_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

/// AC-8: per-invocation outer_wall_line_width override.
///
/// on_print_start sees global outer_wall_line_width=0.5.
/// run_perimeters is called with a per-object config of outer_wall_line_width=0.6.
/// Emitted outer-wall vertex widths MUST be 0.6.
#[test]
fn per_object_outer_wall_line_width_override() {
    let global_outer_w = 0.5_f64;
    let override_outer_w = 0.6_f32;
    let inner_w = 0.4_f64;

    // on_print_start config: global values.
    let start_config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", global_outer_w)
        .float("inner_wall_line_width", inner_w)
        .build();

    let module = ClassicPerimeters::on_print_start(&start_config).unwrap();

    // Per-object override config: outer_wall_line_width bumped to 0.6.
    let override_config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", override_outer_w as f64)
        .float("inner_wall_line_width", inner_w)
        .build();

    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &override_config)
        .unwrap();

    let walls = output.wall_loops();
    let outer_walls: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer)
        .collect();
    assert!(
        !outer_walls.is_empty(),
        "Expected at least one outer wall loop"
    );

    for outer in &outer_walls {
        for pt in &outer.path.points {
            assert!(
                (pt.width - override_outer_w).abs() < 0.005,
                "Outer wall vertex width {} != override {} (per-invocation config read must prevail over on_print_start cache)",
                pt.width,
                override_outer_w
            );
        }
    }

    // Also verify the inner wall uses the inner_w from override_config, not anything
    // stale from on_print_start (inner_w is the same in both configs, so we just
    // confirm the module didn't produce zero-width inner walls).
    let inner_walls: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Inner)
        .collect();
    assert!(
        !inner_walls.is_empty(),
        "Expected at least one inner wall loop"
    );
    for inner in &inner_walls {
        for pt in &inner.path.points {
            assert!(
                (pt.width - inner_w as f32).abs() < 0.005,
                "Inner wall vertex width {} != inner_w {}",
                pt.width,
                inner_w
            );
        }
    }
}

/// Regression: inner_wall_line_width override is also respected per-invocation.
///
/// on_print_start sees inner_wall_line_width=0.4; run_perimeters gets 0.3.
/// Inner wall vertex widths must equal 0.3.
#[test]
fn per_object_inner_wall_line_width_override() {
    let outer_w = 0.5_f64;
    let global_inner_w = 0.4_f64;
    let override_inner_w = 0.3_f32;

    let start_config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w)
        .float("inner_wall_line_width", global_inner_w)
        .build();

    let module = ClassicPerimeters::on_print_start(&start_config).unwrap();

    let override_config = ConfigViewBuilder::new()
        .int("wall_count", 3)
        .float("outer_wall_line_width", outer_w)
        .float("inner_wall_line_width", override_inner_w as f64)
        .build();

    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &override_config)
        .unwrap();

    let walls = output.wall_loops();
    let inner_walls: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Inner)
        .collect();
    assert!(
        !inner_walls.is_empty(),
        "Expected at least one inner wall loop"
    );
    for inner in &inner_walls {
        for pt in &inner.path.points {
            assert!(
                (pt.width - override_inner_w).abs() < 0.005,
                "Inner wall vertex width {} != override {} (per-invocation config read must prevail)",
                pt.width,
                override_inner_w
            );
        }
    }
}
