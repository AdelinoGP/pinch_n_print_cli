#![allow(missing_docs)]

//! P35 (ticket 42): static pressure-advance emission.
//!
//! Canonical `GCode.cpp` gates on `enable_pressure_advance.get_at(id)` and emits
//! `writer().set_pressure_advance(pressure_advance.get_at(id))` once at file
//! start plus once after every toolchange; `GCodeWriter::set_pressure_advance`
//! (`GCodeWriter.cpp`) returns empty for `pa < 0` and spells per flavor
//! (Klipper `SET_PRESSURE_ADVANCE`, RRF `M572`, Repetier `M233`,
//! Marlin/Marlin2 `M900`). The adaptive `resetPreviousPA` arm rides the
//! unimplemented AdaptivePAProcessor (ticket 42's returned keys), so no reset
//! exists here.
//!
//! OrcaSlicer citations by file + function, never line numbers:
//! `GCode.cpp` toolchange/start sites, `GCodeWriter.cpp::set_pressure_advance`,
//! `PrintConfig.cpp` defaults (`enable_pressure_advance` false,
//! `pressure_advance` 0.02 max 2).

use std::collections::BTreeMap;

use slicer_gcode::{DefaultGCodeEmitter, GCodeEmitter, GcodeFlavor};
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, GCodeCommand, LayerCollectionIR, ObjectId, Point3WithWidth,
    PrintEntity, RegionKey, ResolvedConfig,
};
use slicer_sdk::test_support::fixtures::{extrusion_path3d_base, print_entity_base};

fn point3_with_width(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        ..Default::default()
    }
}

fn region_key_fixture() -> RegionKey {
    RegionKey {
        global_layer_index: 0,
        object_id: ObjectId::from("pa-test-object"),
        region_id: 0u64,
        variant_chain: Vec::new(),
    }
}

fn entity_with_tool(
    points: Vec<Point3WithWidth>,
    role: ExtrusionRole,
    id: u64,
    tool: u32,
) -> PrintEntity {
    let mut entity = PrintEntity {
        entity_id: id,
        path: ExtrusionPath3D {
            points,
            ..extrusion_path3d_base(role.clone())
        },
        region_key: region_key_fixture(),
        ..print_entity_base(role)
    };
    entity.tool_index = tool;
    entity
}

fn single_entity_layer(tool: u32) -> LayerCollectionIR {
    let entity = entity_with_tool(
        vec![
            point3_with_width(0.0, 0.0, 0.2),
            point3_with_width(10.0, 0.0, 0.2),
        ],
        ExtrusionRole::OuterWall,
        1,
        tool,
    );
    LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity],
        ..Default::default()
    }
}

fn pa_raw_texts(commands: &[GCodeCommand]) -> Vec<&str> {
    commands
        .iter()
        .filter_map(|c| match c {
            GCodeCommand::Raw { text } => {
                if text.starts_with("M900")
                    || text.starts_with("SET_PRESSURE_ADVANCE")
                    || text.starts_with("M572")
                    || text.starts_with("M233")
                {
                    Some(text.as_str())
                } else {
                    None
                }
            }
            _ => None,
        })
        .collect()
}

fn resolved_with_pa(enabled: bool, value: f32) -> ResolvedConfig {
    ResolvedConfig {
        enable_pressure_advance: enabled,
        pressure_advance: value,
        ..Default::default()
    }
}

#[test]
fn default_config_emits_no_pressure_advance() {
    let emitter = DefaultGCodeEmitter::new("pa-test".to_string());
    let layer = single_entity_layer(0);
    let ir = emitter.emit_gcode(&[layer]).unwrap();
    assert!(
        pa_raw_texts(&ir.commands).is_empty(),
        "default (enable=false) must emit no PA line, got {:#?}",
        ir.commands
    );
}

#[test]
fn disabled_with_value_emits_nothing() {
    let emitter = DefaultGCodeEmitter::new("pa-test".to_string())
        .with_resolved_config(resolved_with_pa(false, 0.08));
    let layer = single_entity_layer(0);
    let ir = emitter.emit_gcode(&[layer]).unwrap();
    assert!(
        pa_raw_texts(&ir.commands).is_empty(),
        "enable=false with a value set must still emit nothing, got {:#?}",
        ir.commands
    );
}

#[test]
fn negative_value_emits_nothing() {
    let emitter = DefaultGCodeEmitter::new("pa-test".to_string())
        .with_resolved_config(resolved_with_pa(true, -0.01));
    let layer = single_entity_layer(0);
    let ir = emitter.emit_gcode(&[layer]).unwrap();
    assert!(
        pa_raw_texts(&ir.commands).is_empty(),
        "negative PA must emit nothing (canonical pa<0 guard), got {:#?}",
        ir.commands
    );
}

#[test]
fn enabled_emits_marlin_m900_right_after_extrusion_mode() {
    let emitter = DefaultGCodeEmitter::new("pa-test".to_string())
        .with_resolved_config(resolved_with_pa(true, 0.05));
    let layer = single_entity_layer(0);
    let ir = emitter.emit_gcode(&[layer]).unwrap();
    let texts = pa_raw_texts(&ir.commands);
    assert_eq!(
        texts,
        vec!["M900 K0.0500"],
        "Marlin PA 0.05 must emit exactly one M900 line, got {texts:?}"
    );
    assert!(
        !texts[0].ends_with('\n'),
        "Raw PA text must not carry a trailing newline (serializer adds it)"
    );
    let mode_at = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::ExtrusionMode { .. }))
        .expect("stream must contain ExtrusionMode");
    assert!(
        matches!(
            ir.commands.get(mode_at + 1),
            Some(GCodeCommand::Raw { text }) if text == "M900 K0.0500"
        ),
        "PA must sit immediately after ExtrusionMode (M73 prepends ahead of both), got {:#?}",
        &ir.commands[mode_at..(mode_at + 3).min(ir.commands.len())]
    );
}

#[test]
fn enabled_emits_per_flavor_forms() {
    let cases = [
        (GcodeFlavor::Marlin, "M900 K0.0500"),
        (GcodeFlavor::Marlin2, "M900 K0.0500"),
        (GcodeFlavor::Klipper, "SET_PRESSURE_ADVANCE ADVANCE=0.0500"),
        (GcodeFlavor::RepRapFirmware, "M572 D0 S0.0500"),
        (GcodeFlavor::Repetier, "M233 X0.0500 Y0.0500"),
    ];
    for (flavor, expected) in cases {
        let emitter = DefaultGCodeEmitter::new("pa-test".to_string())
            .with_resolved_config(resolved_with_pa(true, 0.05))
            .with_flavor(flavor);
        let layer = single_entity_layer(0);
        let ir = emitter.emit_gcode(&[layer]).unwrap();
        let texts = pa_raw_texts(&ir.commands);
        assert_eq!(
            texts,
            vec![expected],
            "flavor {flavor:?} must emit {expected:?}, got {texts:?}"
        );
    }
}

#[test]
fn first_layer_needing_tool_1_still_emits_initial_toolchange() {
    let emitter = DefaultGCodeEmitter::new("pa-test".to_string())
        .with_resolved_config(resolved_with_pa(true, 0.05));
    let layer = single_entity_layer(1);
    let ir = emitter.emit_gcode(&[layer]).unwrap();
    let tc_at = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::ToolChange { from: 0, to: 1, .. }));
    assert!(
        tc_at.is_some(),
        "starting on tool 1 must still emit the initial T0->T1 (physical start is T0), got {:#?}",
        ir.commands
    );
}

#[test]
fn toolchange_emits_new_tool_value_after_the_change() {
    let global = ResolvedConfig {
        enable_pressure_advance: true,
        pressure_advance: 0.05,
        ..Default::default()
    };
    let tool1 = ResolvedConfig {
        enable_pressure_advance: true,
        pressure_advance: 0.09,
        ..Default::default()
    };
    let mut overlays = BTreeMap::new();
    overlays.insert(1u32, tool1);
    let emitter = DefaultGCodeEmitter::new("pa-test".to_string())
        .with_resolved_config(global)
        .with_tool_configs(overlays);

    let first = entity_with_tool(
        vec![point3_with_width(0.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
        1,
        0,
    );
    let second = entity_with_tool(
        vec![point3_with_width(10.0, 0.0, 0.2)],
        ExtrusionRole::OuterWall,
        2,
        1,
    );
    let mut layer = LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![first, second],
        ..Default::default()
    };
    layer.tool_changes = vec![slicer_ir::ToolChange {
        after_entity_index: 0,
        from_tool: 0,
        to_tool: 1,
    }];

    let ir = emitter.emit_gcode(&[layer]).unwrap();
    let texts = pa_raw_texts(&ir.commands);
    assert_eq!(
        texts,
        vec!["M900 K0.0500", "M900 K0.0900"],
        "head PA (tool 0) plus post-change PA (tool 1) expected, got {texts:?}"
    );
    let tc_at = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::ToolChange { .. }))
        .expect("a ToolChange must be emitted");
    let second_pa_at = ir
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::Raw { text } if text == "M900 K0.0900"))
        .expect("post-change PA must be emitted");
    assert!(
        tc_at < second_pa_at,
        "post-change PA must follow the ToolChange (T<n> then PA, canonical order)"
    );
}
