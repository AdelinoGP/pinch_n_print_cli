//! TDD: `retract_mode` config propagation into emitted `Retract` / `Unretract`
//! `GCodeCommand`s on path-optimization-default (packet 34, Step 3).
//!
//! Verifies that `self.retract_mode`, resolved in `on_print_start` from the
//! `retract_mode` config field, is carried verbatim into every
//! `GCodeCommand::Retract` and `GCodeCommand::Unretract` written to the
//! `GcodeOutputBuilder` during `run_path_optimization`.

#![allow(missing_docs)]

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LoopType, Point3WithWidth,
    RetractMode, WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_sdk::layer_collection_builder::LayerCollectionBuilder;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::postpass_types::{GcodeCommand, GcodeOutputCommand};
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;
use std::collections::HashMap;

fn make_wall_loop(x1: f32, y1: f32, x2: f32, y2: f32, z: f32) -> WallLoop {
    WallLoop {
        perimeter_index: 0,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: x1,
                    y: y1,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: x2,
                    y: y2,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: vec![0.4, 0.4],
        },
        feature_flags: vec![],
        boundary_type: WallBoundaryType::Interior,
    }
}

/// Build a config that exercises external inter-region travel and disables the
/// layer marker comment so the output stream contains only the policy commands.
/// `retract_mode` is set via the optional `mode_override` argument; passing
/// `None` exercises the default path (no `retract_mode` field present).
fn config_for_external_travel(mode_override: Option<&str>) -> ConfigView {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert("retract_length".to_string(), ConfigValue::Float(0.8));
    fields.insert(
        "path_optimization_emit_layer_markers".to_string(),
        ConfigValue::Bool(false),
    );
    if let Some(value) = mode_override {
        fields.insert(
            "retract_mode".to_string(),
            ConfigValue::String(value.to_string()),
        );
    }
    ConfigView::from_map(fields)
}

/// Two non-adjacent regions force exactly one external inter-region travel,
/// which must produce one `Retract` and one `Unretract`.
fn two_separate_regions() -> Vec<PerimeterRegionView> {
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
}

/// Run the module against the synthetic two-region fixture and return the
/// emitted command stream.
fn run_with_config(config: &ConfigView) -> Vec<GcodeOutputCommand> {
    let module = path_optimization_default::PathOptimizationDefault::on_print_start(config)
        .expect("on_print_start must succeed");
    let mut output = GcodeOutputBuilder::new();
    let mut collection = LayerCollectionBuilder::new();
    let regions = two_separate_regions();
    module
        .run_path_optimization(0, &regions, &mut output, &mut collection, config)
        .expect("run_path_optimization must succeed");
    output.commands().to_vec()
}

/// Assert that EVERY emitted `Retract` and `Unretract` carries `expected_mode`.
/// Also asserts that at least one of each was emitted (otherwise the test
/// degenerates into vacuous truth).
fn assert_all_retracts_carry_mode(commands: &[GcodeOutputCommand], expected_mode: RetractMode) {
    let mut retract_count = 0usize;
    let mut unretract_count = 0usize;
    for cmd in commands {
        match cmd {
            GcodeOutputCommand::Command(GcodeCommand::Retract { mode, .. }) => {
                assert_eq!(
                    *mode, expected_mode,
                    "Retract emitted with mode {:?}, expected {:?}; commands: {:#?}",
                    mode, expected_mode, commands
                );
                retract_count += 1;
            }
            GcodeOutputCommand::Command(GcodeCommand::Unretract { mode, .. }) => {
                assert_eq!(
                    *mode, expected_mode,
                    "Unretract emitted with mode {:?}, expected {:?}; commands: {:#?}",
                    mode, expected_mode, commands
                );
                unretract_count += 1;
            }
            _ => {}
        }
    }
    assert!(
        retract_count >= 1,
        "fixture must produce at least one Retract; commands: {commands:#?}"
    );
    assert!(
        unretract_count >= 1,
        "fixture must produce at least one Unretract; commands: {commands:#?}"
    );
}

/// AC-3: With default config (no `retract_mode` field), every emitted
/// Retract/Unretract carries `RetractMode::Gcode`. With `retract_mode =
/// "firmware"`, every emitted Retract/Unretract carries
/// `RetractMode::Firmware`.
#[test]
fn retract_mode_propagates_into_ir_commands() {
    // Direction 1 — default (no override): expect RetractMode::Gcode.
    let default_config = config_for_external_travel(None);
    let default_commands = run_with_config(&default_config);
    assert_all_retracts_carry_mode(&default_commands, RetractMode::Gcode);

    // Direction 2 — explicit `retract_mode = "firmware"`: expect RetractMode::Firmware.
    let firmware_config = config_for_external_travel(Some("firmware"));
    let firmware_commands = run_with_config(&firmware_config);
    assert_all_retracts_carry_mode(&firmware_commands, RetractMode::Firmware);
}
