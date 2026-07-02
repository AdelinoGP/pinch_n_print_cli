//! AC-1 (packet 108, T-070/T-071): `extra_perimeters` per-region config bonus.
//!
//! OrcaSlicer PerimeterGenerator.cpp:1569 —
//! `int loop_number = this->config->wall_loops + surface.extra_perimeters - 1;
//! // 0-indexed loops`
//!
//! A region with base `wall_count=2` and `extra_perimeters=2` must emit exactly
//! 4 walls (loop_number = wall_count + extra_perimeters - 1, zero-indexed);
//! with `extra_perimeters=0` it must emit exactly 2 walls (bonus is a no-op).

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

/// Run perimeters with the given config and return emitted Outer/Inner wall loops.
fn run_with_config(config: slicer_ir::ConfigView) -> Vec<slicer_ir::WallLoop> {
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(10.0, 0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();
    output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer || w.loop_type == LoopType::Inner)
        .cloned()
        .collect()
}

/// AC-1 positive case: base wall_count=2, extra_perimeters=2 → 4 walls.
#[test]
fn extra_perimeters_bonus_adds_to_wall_count() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .int("extra_perimeters", 2)
        .build();

    let walls = run_with_config(config);
    assert_eq!(
        walls.len(),
        4,
        "Expected 4 wall loops (wall_count=2 + extra_perimeters=2); got {}",
        walls.len()
    );
}

/// AC-1 no-op case: base wall_count=2, extra_perimeters=0 → 2 walls (unchanged).
#[test]
fn extra_perimeters_zero_is_noop() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .int("extra_perimeters", 0)
        .build();

    let walls = run_with_config(config);
    assert_eq!(
        walls.len(),
        2,
        "Expected 2 wall loops (wall_count=2 + extra_perimeters=0); got {}",
        walls.len()
    );
}
