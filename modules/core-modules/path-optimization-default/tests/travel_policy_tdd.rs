//! TDD: travel retract/no-retract policy on path-optimization-default.
//!
//! Verifies that path-optimization-default emits matched Retract/Move/Unretract
//! for external inter-region travel and suppresses them for intra-region travel.

#![allow(missing_docs)]

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, Point3WithWidth, WallLoop,
};
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::postpass_types::{GcodeCommand, GcodeOutputCommand};
use slicer_sdk::test_prelude::PerimeterRegionViewBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;
use std::collections::HashMap;

#[rustfmt::skip]
fn make_wall_loop(x1: f32, y1: f32, x2: f32, y2: f32, z: f32) -> WallLoop {
    let p = |x, y| Point3WithWidth { x, y, z, width: 0.4, flow_factor: 1.0, overhang_quartile: None, dist_to_top_mm: 0.0};
    PerimeterRegionViewBuilder::new().add_outer_wall(ExtrusionPath3D { points: vec![p(x1, y1), p(x2, y2)], role: ExtrusionRole::OuterWall, speed_factor: 1.0 }).build().wall_loops()[0].clone()
}

fn config_with_retract(retract_length: f64) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert(
        "retract_length".to_string(),
        ConfigValue::Float(retract_length),
    );
    // Disable markers so they don't interfere with command-sequence assertions.
    fields.insert(
        "path_optimization_emit_layer_markers".to_string(),
        ConfigValue::Bool(false),
    );
    ConfigView::from_map(fields)
}

fn config_with_retract_and_z_hop(retract_length: f64, z_hop: f64) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert(
        "retract_length".to_string(),
        ConfigValue::Float(retract_length),
    );
    fields.insert("travel_z_hop".to_string(), ConfigValue::Float(z_hop));
    fields.insert(
        "path_optimization_emit_layer_markers".to_string(),
        ConfigValue::Bool(false),
    );
    ConfigView::from_map(fields)
}

/// AC-ext: two separate regions emit one matched Retract → Move(e=None) → Unretract.
#[test]
fn external_travel_emits_matched_retract_and_unretract() {
    let config = config_with_retract(0.8);

    let mut region_a = PerimeterRegionView::default();
    region_a.set_object_id("obj-a");
    region_a.set_region_id(0);
    region_a.set_wall_loops(vec![make_wall_loop(0.0, 0.0, 10.0, 0.0, 0.2)]);
    region_a.set_infill_areas(vec![]);
    region_a.set_seam_candidates(vec![]);
    region_a.set_resolved_seam(None);
    let mut region_b = PerimeterRegionView::default();
    region_b.set_object_id("obj-a");
    region_b.set_region_id(1);
    region_b.set_wall_loops(vec![make_wall_loop(50.0, 50.0, 60.0, 50.0, 0.2)]);
    region_b.set_infill_areas(vec![]);
    region_b.set_seam_candidates(vec![]);
    region_b.set_resolved_seam(None);

    let module = path_optimization_default::PathOptimizationDefault::on_print_start(&config)
        .expect("on_print_start must succeed");
    let mut output = GcodeOutputBuilder::new();
    let mut collection = LayerCollectionBuilder::new();
    module
        .run_path_optimization(
            0,
            &[region_a, region_b],
            &mut output,
            &mut collection,
            &config,
        )
        .expect("run_path_optimization must succeed");

    let commands = output.commands();

    let retract_pos = commands.iter().position(|c| {
        matches!(
            c,
            GcodeOutputCommand::Command(GcodeCommand::Retract { length, .. })
                if (*length - 0.8_f32).abs() < 1e-4
        )
    });
    let move_pos = commands.iter().position(|c| {
        matches!(
            c,
            GcodeOutputCommand::Command(GcodeCommand::Move { e: None, .. })
        )
    });
    let unretract_pos = commands.iter().position(|c| {
        matches!(
            c,
            GcodeOutputCommand::Command(GcodeCommand::Unretract { length, .. })
                if (*length - 0.8_f32).abs() < 1e-4
        )
    });

    assert!(
        retract_pos.is_some(),
        "external travel must emit Retract {{length:0.8}}, got: {commands:#?}"
    );
    assert!(
        move_pos.is_some(),
        "external travel must emit Move {{e:None}}, got: {commands:#?}"
    );
    assert!(
        unretract_pos.is_some(),
        "external travel must emit Unretract {{length:0.8}}, got: {commands:#?}"
    );

    let (ri, mi, ui) = (
        retract_pos.unwrap(),
        move_pos.unwrap(),
        unretract_pos.unwrap(),
    );
    assert!(
        ri < mi && mi < ui,
        "order must be Retract({ri}) < Move({mi}) < Unretract({ui}), commands: {commands:#?}"
    );
}

/// AC-int: single region with multiple wall loops suppresses retraction.
#[test]
fn internal_travel_suppresses_retraction() {
    let config = config_with_retract(0.8);

    let mut region = PerimeterRegionView::default();
    region.set_object_id("obj-a");
    region.set_region_id(0);
    region.set_wall_loops(vec![
        make_wall_loop(0.0, 0.0, 10.0, 0.0, 0.2),
        make_wall_loop(1.0, 1.0, 9.0, 1.0, 0.2),
    ]);
    region.set_infill_areas(vec![]);
    region.set_seam_candidates(vec![]);
    region.set_resolved_seam(None);

    let module = path_optimization_default::PathOptimizationDefault::on_print_start(&config)
        .expect("on_print_start must succeed");
    let mut output = GcodeOutputBuilder::new();
    let mut collection = LayerCollectionBuilder::new();
    module
        .run_path_optimization(0, &[region], &mut output, &mut collection, &config)
        .expect("run_path_optimization must succeed");

    let commands = output.commands();

    let retract_count = commands
        .iter()
        .filter(|c| matches!(c, GcodeOutputCommand::Command(GcodeCommand::Retract { .. })))
        .count();
    let unretract_count = commands
        .iter()
        .filter(|c| {
            matches!(
                c,
                GcodeOutputCommand::Command(GcodeCommand::Unretract { .. })
            )
        })
        .count();

    assert_eq!(
        retract_count, 0,
        "internal travel must emit NO Retract, got {retract_count}: {commands:#?}"
    );
    assert_eq!(
        unretract_count, 0,
        "internal travel must emit NO Unretract, got {unretract_count}: {commands:#?}"
    );
}

/// AC-z-hop (module level): external travel with travel_z_hop=0.2 emits a ZHop entry
/// alongside the retract/unretract pair.
#[test]
fn external_travel_with_z_hop_emits_z_hop_and_retract_pair() {
    let config = config_with_retract_and_z_hop(0.8, 0.2);

    let mut region_a = PerimeterRegionView::default();
    region_a.set_object_id("obj-a");
    region_a.set_region_id(0);
    region_a.set_wall_loops(vec![make_wall_loop(0.0, 0.0, 10.0, 0.0, 0.2)]);
    region_a.set_infill_areas(vec![]);
    region_a.set_seam_candidates(vec![]);
    region_a.set_resolved_seam(None);
    let mut region_b = PerimeterRegionView::default();
    region_b.set_object_id("obj-a");
    region_b.set_region_id(1);
    region_b.set_wall_loops(vec![make_wall_loop(50.0, 50.0, 60.0, 50.0, 0.2)]);
    region_b.set_infill_areas(vec![]);
    region_b.set_seam_candidates(vec![]);
    region_b.set_resolved_seam(None);

    let module = path_optimization_default::PathOptimizationDefault::on_print_start(&config)
        .expect("on_print_start must succeed");
    let mut output = GcodeOutputBuilder::new();
    let mut collection = LayerCollectionBuilder::new();
    module
        .run_path_optimization(
            0,
            &[region_a, region_b],
            &mut output,
            &mut collection,
            &config,
        )
        .expect("run_path_optimization must succeed");

    let commands = output.commands();

    let z_hop = commands.iter().find(|c| {
        matches!(c, GcodeOutputCommand::ZHop { hop_height, .. } if (*hop_height - 0.2_f32).abs() < 1e-4)
    });
    assert!(
        z_hop.is_some(),
        "travel_z_hop=0.2 must emit ZHop{{hop_height:0.2}}, got: {commands:#?}"
    );

    let retract = commands
        .iter()
        .filter(|c| matches!(c, GcodeOutputCommand::Command(GcodeCommand::Retract { .. })))
        .count();
    let unretract = commands
        .iter()
        .filter(|c| {
            matches!(
                c,
                GcodeOutputCommand::Command(GcodeCommand::Unretract { .. })
            )
        })
        .count();
    assert_eq!(retract, 1, "must have exactly one Retract with z_hop");
    assert_eq!(unretract, 1, "must have exactly one Unretract with z_hop");
}

/// Determinism: repeated runs on the same two-region fixture produce byte-identical output.
#[test]
fn travel_policy_output_is_deterministic() {
    let config = config_with_retract(0.8);

    let make_regions = || {
        vec![
            {
                let mut tmp = PerimeterRegionView::default();
                tmp.set_object_id("obj-a");
                tmp.set_region_id(0);
                tmp.set_wall_loops(vec![make_wall_loop(0.0, 0.0, 10.0, 0.0, 0.2)]);
                tmp.set_infill_areas(vec![]);
                tmp.set_seam_candidates(vec![]);
                tmp.set_resolved_seam(None);
                tmp
            },
            {
                let mut tmp = PerimeterRegionView::default();
                tmp.set_object_id("obj-a");
                tmp.set_region_id(1);
                tmp.set_wall_loops(vec![make_wall_loop(50.0, 50.0, 60.0, 50.0, 0.2)]);
                tmp.set_infill_areas(vec![]);
                tmp.set_seam_candidates(vec![]);
                tmp.set_resolved_seam(None);
                tmp
            },
        ]
    };

    let module = path_optimization_default::PathOptimizationDefault::on_print_start(&config)
        .expect("on_print_start must succeed");

    let mut out1 = GcodeOutputBuilder::new();
    let mut collection1 = LayerCollectionBuilder::new();
    module
        .run_path_optimization(0, &make_regions(), &mut out1, &mut collection1, &config)
        .expect("first run must succeed");

    let mut out2 = GcodeOutputBuilder::new();
    let mut collection2 = LayerCollectionBuilder::new();
    module
        .run_path_optimization(0, &make_regions(), &mut out2, &mut collection2, &config)
        .expect("second run must succeed");

    let cmds1 = out1.commands();
    let cmds2 = out2.commands();

    assert_eq!(cmds1.len(), cmds2.len(), "command count must match");
    for (a, b) in cmds1.iter().zip(cmds2.iter()) {
        assert_eq!(
            format!("{a:?}"),
            format!("{b:?}"),
            "commands must be byte-identical across runs"
        );
    }
}
