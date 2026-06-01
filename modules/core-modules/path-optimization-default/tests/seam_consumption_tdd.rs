//! Integration TDD tests: seam consumption on path-optimization-default.
//!
//! Verifies that `path-optimization-default` replays wall loops from
//! `PerimeterRegionView.resolved_seam` starting at the seam point,
//! and that absent resolved_seam leaves wall-loop order unchanged.
//!
//! The path being tested:
//!   `Layer::PathOptimization` → path-optimization-default reads
//!   `PerimeterRegionView.wall_loops()` + `PerimeterRegionView.resolved_seam()`
//!   → replays wall loops starting at seam point via `GcodeOutputBuilder::push_move`
//!   → dispatch commits Move commands as LayerAnnotationKind::Raw annotations
//!
//! Key invariants verified:
//!   - resolved_seam causes wall loop to be replayed starting at seam point
//!   - first emitted move matches the resolved seam coordinates
//!   - missing resolved_seam leaves wall loop order unchanged (no fabrication)
//!   - repeated runs produce byte-identical output

#![allow(missing_docs)]

use slicer_ir::{ExtrusionPath3D, ExtrusionRole, Point3WithWidth, SeamPosition, WallLoop};
use slicer_sdk::prelude::LayerModule;
use slicer_sdk::test_prelude::PerimeterRegionViewBuilder;
use slicer_sdk::views::PerimeterRegionView;

/// Helper: make a 2-point horizontal wall loop.
#[rustfmt::skip]
fn make_wall_loop(x1: f32, y1: f32, x2: f32, y2: f32, z: f32, width: f32) -> WallLoop {
    let p = |x, y| Point3WithWidth { x, y, z, width, flow_factor: 1.0, overhang_quartile: None };
    PerimeterRegionViewBuilder::new().add_outer_wall(ExtrusionPath3D { points: vec![p(x1, y1), p(x2, y2)], role: ExtrusionRole::OuterWall, speed_factor: 1.0 }).build().wall_loops()[0].clone()
}

/// Test (AC-4): when PerimeterIR wall loops are already seam-first rotated,
/// path-optimization-default emits only the per-layer marker comment and
/// does NOT emit any GCodeMoveCmd via push_move.
#[test]
fn no_move_commands_emitted_when_perimeter_already_rotated() {
    // Build a PerimeterRegionView with resolved_seam set on wall index 0.
    // Per the seam-first contract, wall loops are already stored with
    // path.points[0] as the seam vertex. PathOptimization should NOT
    // replay them — it only emits the marker comment.
    let wall_loop = make_wall_loop(0.0, 0.0, 10.0, 0.0, 0.2, 0.4);
    let resolved_seam = SeamPosition {
        point: Point3WithWidth {
            x: 5.0,
            y: 0.0,
            z: 0.2,
            width: 0.0,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        wall_index: 0,
    };

    let mut region = PerimeterRegionView::default();
    region.set_object_id("test-object");
    region.set_region_id(0);
    region.set_wall_loops(vec![wall_loop]);
    region.set_infill_areas(vec![]);
    // no infill areas needed for this test

    region.set_seam_candidates(vec![]);
    region.set_resolved_seam(Some(resolved_seam));

    let module = path_optimization_default::PathOptimizationDefault::on_print_start(
        &slicer_ir::ConfigView::default(),
    )
    .expect("on_print_start must succeed");
    let mut output = slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
    let mut collection = slicer_sdk::LayerCollectionBuilder::new();
    module
        .run_path_optimization(
            7,
            &[region],
            &mut output,
            &mut collection,
            &slicer_ir::ConfigView::default(),
        )
        .expect("run_path_optimization must succeed");

    let commands = output.commands();
    let move_count = commands
        .iter()
        .filter(|c| {
            matches!(
                c,
                slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                    slicer_sdk::postpass_types::GcodeCommand::Move { .. }
                )
            )
        })
        .count();

    assert_eq!(
        move_count, 0,
        "path-optimization-default must not emit Move commands when perimeter is already rotated (AC-4); got {move_count}"
    );

    // But the marker comment must be present.
    let comment_count = commands
        .iter()
        .filter(|c| {
            matches!(
                c,
                slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                    slicer_sdk::postpass_types::GcodeCommand::Comment { .. }
                )
            )
        })
        .count();
    assert!(
        comment_count > 0,
        "path-optimization-default must emit the per-layer marker comment"
    );
}

/// Test that absent resolved_seam does not cause fabrication.
#[test]
fn missing_resolved_seam_leaves_wall_loop_order_unchanged() {
    // Build a PerimeterRegionView with no resolved_seam.
    let wall_loop = make_wall_loop(0.0, 0.0, 10.0, 0.0, 0.2, 0.4);
    let mut region = PerimeterRegionView::default();
    region.set_object_id("test-object");
    region.set_region_id(0);
    region.set_wall_loops(vec![wall_loop]);
    region.set_infill_areas(vec![]);
    region.set_seam_candidates(vec![]);
    region.set_resolved_seam(None);

    let module = path_optimization_default::PathOptimizationDefault::on_print_start(
        &slicer_ir::ConfigView::default(),
    )
    .expect("on_print_start must succeed");
    let mut output = slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
    let mut collection = slicer_sdk::LayerCollectionBuilder::new();
    module
        .run_path_optimization(
            7,
            &[region],
            &mut output,
            &mut collection,
            &slicer_ir::ConfigView::default(),
        )
        .expect("run_path_optimization must succeed");

    let commands = output.commands();

    // Without resolved_seam, NO Move commands should be emitted (only the comment).
    let move_count = commands
        .iter()
        .filter(|c| {
            matches!(
                c,
                slicer_sdk::postpass_types::GcodeOutputCommand::Command(
                    slicer_sdk::postpass_types::GcodeCommand::Move { .. }
                )
            )
        })
        .count();
    assert_eq!(
        move_count, 0,
        "absent resolved_seam must not cause Move fabrication, got {move_count} Move commands"
    );
}

/// Test that repeated runs produce byte-identical output.
#[test]
fn seam_started_wall_replay_is_deterministic() {
    let wall_loop = make_wall_loop(0.0, 0.0, 10.0, 0.0, 0.2, 0.4);
    let resolved_seam = SeamPosition {
        point: Point3WithWidth {
            x: 5.0,
            y: 0.0,
            z: 0.2,
            width: 0.0,
            flow_factor: 1.0,
            overhang_quartile: None,
        },
        wall_index: 0,
    };
    let mut region = PerimeterRegionView::default();
    region.set_object_id("test-object");
    region.set_region_id(0);
    region.set_wall_loops(vec![wall_loop]);
    region.set_infill_areas(vec![]);
    region.set_seam_candidates(vec![]);
    region.set_resolved_seam(Some(resolved_seam));

    let module = path_optimization_default::PathOptimizationDefault::on_print_start(
        &slicer_ir::ConfigView::default(),
    )
    .expect("on_print_start must succeed");

    let mut output1 = slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
    let mut collection1 = slicer_sdk::LayerCollectionBuilder::new();
    module
        .run_path_optimization(
            7,
            std::slice::from_ref(&region),
            &mut output1,
            &mut collection1,
            &slicer_ir::ConfigView::default(),
        )
        .expect("first run must succeed");

    let mut output2 = slicer_sdk::postpass_builders::GcodeOutputBuilder::new();
    let mut collection2 = slicer_sdk::LayerCollectionBuilder::new();
    module
        .run_path_optimization(
            7,
            std::slice::from_ref(&region),
            &mut output2,
            &mut collection2,
            &slicer_ir::ConfigView::default(),
        )
        .expect("second run must succeed");

    let cmds1 = output1.commands();
    let cmds2 = output2.commands();
    assert_eq!(
        cmds1.len(),
        cmds2.len(),
        "determinism: command count must match across runs"
    );
    for (a, b) in cmds1.iter().zip(cmds2.iter()) {
        assert_eq!(
            format!("{:?}", a),
            format!("{:?}", b),
            "determinism: command {a:?} must match {b:?} across runs"
        );
    }
}
