//! TDD tests for PostpassModule trait and WIT bindings.
//!
//! These tests verify the API defined in docs/05_module_sdk.md and docs/03_wit_and_manifest.md.
//! Tests lock down trait signatures, postpass types, and output builders.

use slicer_sdk::prelude::*;
use std::collections::HashMap;

// =============================================================================
// Test 1: PostpassModule trait exists with lifecycle methods
// =============================================================================

/// A test module that implements PostpassModule.
struct TestPostpassModule {
    initialized: bool,
}

impl PostpassModule for TestPostpassModule {
    fn on_print_start(config: &ConfigView) -> Result<Self, ModuleError> {
        // Verify ConfigView is accessible
        let _ = config.len();
        Ok(Self { initialized: true })
    }

    fn on_print_end(&self) -> Result<(), ModuleError> {
        Ok(())
    }
}

#[test]
fn test_01_postpass_module_trait_exists_with_lifecycle() {
    // Test that PostpassModule trait can be implemented with on_print_start/on_print_end
    let config = ConfigView::from_map(HashMap::new());

    let module =
        TestPostpassModule::on_print_start(&config).expect("on_print_start should succeed");
    assert!(module.initialized, "module should be initialized");

    module.on_print_end().expect("on_print_end should succeed");
}

// =============================================================================
// Test 2: GcodeCommand enum exposes all payload-bearing variants
// =============================================================================

#[test]
fn test_02_gcode_command_enum_has_all_variants() {
    let move_cmd = GcodeCommand::Move {
        x: Some(10.0),
        y: Some(20.0),
        z: Some(0.3),
        e: Some(1.5),
        f: Some(1200.0),
        role: ExtrusionRole::OuterWall,
    };
    let retract = GcodeCommand::Retract {
        length: 1.0,
        speed: 30.0,
    };
    let unretract = GcodeCommand::Unretract {
        length: 1.0,
        speed: 30.0,
    };
    let fan_speed = GcodeCommand::FanSpeed { value: 255 };
    let temperature = GcodeCommand::Temperature {
        tool: 0,
        celsius: 210.0,
        wait: true,
    };
    let tool_change = GcodeCommand::ToolChange { from: 0, to: 1 };
    let comment = GcodeCommand::Comment {
        text: "layer change".to_string(),
    };
    let raw = GcodeCommand::Raw {
        text: "G28".to_string(),
    };

    assert!(matches!(move_cmd, GcodeCommand::Move { .. }));
    assert!(matches!(retract, GcodeCommand::Retract { .. }));
    assert!(matches!(unretract, GcodeCommand::Unretract { .. }));
    assert!(matches!(fan_speed, GcodeCommand::FanSpeed { .. }));
    assert!(matches!(temperature, GcodeCommand::Temperature { .. }));
    assert!(matches!(tool_change, GcodeCommand::ToolChange { .. }));
    assert!(matches!(comment, GcodeCommand::Comment { .. }));
    assert!(matches!(raw, GcodeCommand::Raw { .. }));
}

// =============================================================================
// Test 3: GcodeCommand preserves payload fields
// =============================================================================

#[test]
fn test_03_gcode_command_preserves_payload_fields() {
    let command = GcodeCommand::Move {
        x: Some(42.0),
        y: None,
        z: Some(0.2),
        e: Some(0.8),
        f: Some(1800.0),
        role: ExtrusionRole::InnerWall,
    };

    match command {
        GcodeCommand::Move {
            x,
            y,
            z,
            e,
            f,
            role,
        } => {
            assert_eq!(x, Some(42.0));
            assert_eq!(y, None);
            assert_eq!(z, Some(0.2));
            assert_eq!(e, Some(0.8));
            assert_eq!(f, Some(1800.0));
            assert_eq!(role, ExtrusionRole::InnerWall);
        }
        other => panic!("expected Move command, got {other:?}"),
    }
}

// =============================================================================
// Test 4: GcodeMoveCmd has required fields
// =============================================================================

#[test]
fn test_04_gcode_move_cmd_has_required_fields() {
    // Per docs/03_wit_and_manifest.md (ir-types.wit):
    // record gcode-move-cmd {
    //     x: option<f32>, y: option<f32>, z: option<f32>,
    //     e: option<f32>, f: option<f32>,
    //     role: extrusion-role,
    // }

    let cmd = GcodeMoveCmd {
        x: Some(10.0),
        y: Some(20.0),
        z: Some(0.3),
        e: Some(1.5),
        f: Some(1200.0),
        role: ExtrusionRole::OuterWall,
    };

    assert_eq!(cmd.x, Some(10.0));
    assert_eq!(cmd.y, Some(20.0));
    assert_eq!(cmd.z, Some(0.3));
    assert_eq!(cmd.e, Some(1.5));
    assert_eq!(cmd.f, Some(1200.0));
    assert_eq!(cmd.role, ExtrusionRole::OuterWall);

    // Test with None fields
    let cmd2 = GcodeMoveCmd::new(None, None, None, None, None, ExtrusionRole::InnerWall);
    assert_eq!(cmd2.x, None);
    assert_eq!(cmd2.role, ExtrusionRole::InnerWall);
}

// =============================================================================
// Test 5: GcodeOutputBuilder push_move
// =============================================================================

#[test]
fn test_05_gcode_output_builder_push_move() {
    let mut builder = GcodeOutputBuilder::new();
    let cmd = GcodeMoveCmd::new(
        Some(10.0),
        Some(20.0),
        Some(0.3),
        Some(1.5),
        Some(1200.0),
        ExtrusionRole::OuterWall,
    );

    let result = builder.push_move(cmd);
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

// =============================================================================
// Test 6: GcodeOutputBuilder push_retract
// =============================================================================

#[test]
fn test_06_gcode_output_builder_push_retract() {
    let mut builder = GcodeOutputBuilder::new();

    let result = builder.push_retract(1.0, 30.0);
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

#[test]
fn test_06b_gcode_output_builder_push_unretract() {
    let mut builder = GcodeOutputBuilder::new();

    let result = builder.push_unretract(1.0, 30.0);
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

// =============================================================================
// Test 7: GcodeOutputBuilder push_fan_speed
// =============================================================================

#[test]
fn test_07_gcode_output_builder_push_fan_speed() {
    let mut builder = GcodeOutputBuilder::new();

    let result = builder.push_fan_speed(255);
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

// =============================================================================
// Test 8: GcodeOutputBuilder push_temperature
// =============================================================================

#[test]
fn test_08_gcode_output_builder_push_temperature() {
    let mut builder = GcodeOutputBuilder::new();

    let result = builder.push_temperature(0, 210.0, true);
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

// =============================================================================
// Test 9: GcodeOutputBuilder push_tool_change
// =============================================================================

#[test]
fn test_09_gcode_output_builder_push_tool_change() {
    let mut builder = GcodeOutputBuilder::new();

    let result = builder.push_tool_change(0, 1);
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

// =============================================================================
// Test 10: GcodeOutputBuilder push_comment
// =============================================================================

#[test]
fn test_10_gcode_output_builder_push_comment() {
    let mut builder = GcodeOutputBuilder::new();

    let result = builder.push_comment("layer change".to_string());
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

// =============================================================================
// Test 11: GcodeOutputBuilder push_raw
// =============================================================================

#[test]
fn test_11_gcode_output_builder_push_raw() {
    let mut builder = GcodeOutputBuilder::new();

    let result = builder.push_raw("G28 ; home all axes".to_string());
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

#[test]
fn test_11b_gcode_output_builder_push_z_hop() {
    let mut builder = GcodeOutputBuilder::new();

    let result = builder.push_z_hop(3, 0.4);
    assert!(result.is_ok());
    assert_eq!(builder.commands().len(), 1);
}

// =============================================================================
// Test 12: run_gcode_postprocess signature matches WIT
// =============================================================================

struct GcodePostprocessTestModule;

impl PostpassModule for GcodePostprocessTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_gcode_postprocess(
        &self,
        commands: &[GcodeCommand],
        output: &mut GcodeOutputBuilder,
        config: &ConfigView,
    ) -> Result<(), ModuleError> {
        // This tests that the signature compiles correctly
        let _ = commands.len();
        let _ = output;
        let _ = config.len();
        Ok(())
    }
}

#[test]
fn test_12_run_gcode_postprocess_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = GcodePostprocessTestModule::on_print_start(&config).unwrap();
    let commands = vec![
        GcodeCommand::Move {
            x: Some(10.0),
            y: Some(20.0),
            z: Some(0.3),
            e: Some(1.5),
            f: Some(1200.0),
            role: ExtrusionRole::OuterWall,
        },
        GcodeCommand::Comment {
            text: "layer change".to_string(),
        },
    ];
    let mut output = GcodeOutputBuilder::new();

    let result = module.run_gcode_postprocess(&commands, &mut output, &config);
    assert!(result.is_ok());
}

// =============================================================================
// Test 13: run_text_postprocess signature matches WIT
// =============================================================================

struct TextPostprocessTestModule;

impl PostpassModule for TextPostprocessTestModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }

    fn run_text_postprocess(
        &self,
        gcode_text: &str,
        config: &ConfigView,
    ) -> Result<String, ModuleError> {
        // This tests that the signature compiles correctly
        let _ = config.len();
        Ok(format!("; postprocessed\n{}", gcode_text))
    }
}

#[test]
fn test_13_run_text_postprocess_signature_matches_wit() {
    let config = ConfigView::from_map(HashMap::new());
    let module = TextPostprocessTestModule::on_print_start(&config).unwrap();
    let input = "G28\nG1 X10 Y20\n";

    let result = module.run_text_postprocess(input, &config);
    assert!(result.is_ok());
    let output = result.unwrap();
    assert!(output.contains("; postprocessed"));
    assert!(output.contains("G28"));
}

// =============================================================================
// Test 14: Default implementations exist for both run methods
// =============================================================================

struct MinimalPostpassModule;

impl PostpassModule for MinimalPostpassModule {
    fn on_print_start(_config: &ConfigView) -> Result<Self, ModuleError> {
        Ok(Self)
    }
    // Both run_gcode_postprocess and run_text_postprocess have default implementations
}

#[test]
fn test_14_default_implementations_exist() {
    let config = ConfigView::from_map(HashMap::new());
    let module = MinimalPostpassModule::on_print_start(&config).unwrap();
    let commands = vec![GcodeCommand::Raw {
        text: "G28".to_string(),
    }];
    let mut gcode_output = GcodeOutputBuilder::new();

    // Default gcode postprocess should succeed (no-op)
    let gcode_result = module.run_gcode_postprocess(&commands, &mut gcode_output, &config);
    assert!(gcode_result.is_ok());

    // Default text postprocess should return input unchanged
    let text_result = module.run_text_postprocess("G28\n", &config);
    assert!(text_result.is_ok());
    assert_eq!(text_result.unwrap(), "G28\n");
}

// =============================================================================
// Test 15: All postpass types are accessible via slicer_sdk::prelude::*
// =============================================================================

#[test]
fn test_15_prelude_exports_all_postpass_types() {
    // Verify all postpass types are accessible via prelude
    fn _check_types() {
        // PostpassModule is a trait, so we check it via a function signature
        fn _takes_postpass_module<T: PostpassModule>(_: T) {}

        let _: GcodeCommand;
        let _: GcodeOutputCommand;
        let _: GcodeOutputBuilder;
        let _: GcodeMoveCmd;
    }
}

#[test]
fn test_15b_prelude_types_are_constructible() {
    // Verify types can be constructed via prelude imports
    let _command = GcodeCommand::Retract {
        length: 1.0,
        speed: 30.0,
    };
    let _output_command = GcodeOutputCommand::ZHop {
        after_entity_index: 0,
        hop_height: 0.2,
    };
    let _builder = GcodeOutputBuilder::new();
    let _cmd = GcodeMoveCmd::new(None, None, None, None, None, ExtrusionRole::OuterWall);
}
