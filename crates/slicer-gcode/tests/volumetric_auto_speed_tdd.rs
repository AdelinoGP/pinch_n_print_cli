#![allow(missing_docs)]

//! TDD tests for TASK-571 / Step 3 of
//! `config-scope-resolution_10_remaining-automatic-values`: the Phase-C
//! volumetric automatic speed (`role base speed == 0` resolves from the active
//! tool's `filament_max_volumetric_speed` and the move's live geometry).
//!
//! These tests drive the real production path (`DefaultGCodeEmitter::emit_gcode`
//! over a real `LayerCollectionIR`) and assert literal feedrate tokens. Every
//! expected `F` value is derived analytically here — never by calling the
//! emitter or re-implementing its resolver:
//!
//! - `mm3_per_mm = width × height_delta × flow_factor` (mm³/mm)
//! - automatic speed (mm/s) = `filament_max_volumetric_speed / mm3_per_mm`
//! - emitted `F` = speed × 60, then `resolve_feedrate`'s `clamp(0.05, 5.0)`
//!   factor application and 3-decimal rounding (ADR-0052 seam).
//!
//! Worked examples (factor 1.0 unless noted):
//! - 8.0 mm³/s over 0.4 × 0.2 × 1.0 = 0.08 mm³/mm → 100 mm/s → F6000 (AC-1)
//! - 12.0 mm³/s over the same geometry → 150 mm/s → F9000 (AC-2, tool 1)
//! - explicit `outer_wall_speed = 30.0` → F1800, never the auto branch (AC-2)
//! - with a 1.0 mm³/s maximum, that explicit speed is above the 12.5 mm/s
//!   volumetric ceiling (which would emit F750 if applied), but must stay F1800
//! - 8.0 mm³/s over width 0.5 → 80 mm/s → F4800; factor 10 clamps to 5 →
//!   F24000; factor 0.01 clamps to 0.05 → F240 (variant coverage)
//!
//! `invalid_volumetric_auto_inputs_fail_closed` (AC-N1) is the negative control:
//! every invalid single input and each derived-speed f32 narrowing failure must
//! return `GCodeEmitError::Emit` naming the input or conversion, and must never
//! yield a zero, NaN, or infinite `Move.f`.
//!
//! Literal-precision note: `9000.0` and `6000.0` are the packet's mandated
//! literals. Canonical `GCode::_extrude` computes this quotient in doubles, so
//! `12.0 / (0.4 × 0.2) = 150.0` exactly; a naive f32 `width*height*flow`
//! product order yields 8999.999, which this test intentionally rejects.

use std::collections::BTreeMap;

use slicer_gcode::{DefaultGCodeEmitter, GCodeEmitError, GCodeEmitter};
use slicer_ir::{
    ConfigValue, ExtrusionPath3D, ExtrusionRole, FeedrateConfig, GCodeCommand, GCodeIR,
    LayerCollectionIR, Point3WithWidth, PrintEntity, RegionKey, ResolvedConfig, ToolChange,
};
use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use slicer_sdk::test_support::fixtures::print_entity_base;

/// First-layer Z used by every fixture; with `first_layer_height = 0.2` this is
/// the effective `height_delta` seeded for layer 0.
const Z_FIRST_LAYER: f32 = 0.2;
/// Declared-and-typed volumetric maximum used by most fixtures (mm³/s).
const AUTO_MAX_MM3_S: f32 = 8.0;
/// Outer-wall width of the AC-1 fixture (mm).
const WIDTH_MM: f32 = 0.4;
/// First-layer height / AC-1 layer height (mm).
const HEIGHT_MM: f64 = 0.2;
/// AC-1 flow factor.
const FLOW: f32 = 1.0;

/// Store the registered volumetric maximum in the resolved-config extension
/// carrier, matching the public config-resolution path.
fn config_with_volumetric_max(first_layer_height: f64, max: f32) -> ResolvedConfig {
    let mut config = ResolvedConfig {
        first_layer_height,
        ..Default::default()
    };
    config.extensions.insert(
        "filament_max_volumetric_speed".to_string(),
        ConfigValue::Float(max as f64),
    );
    config
}

/// A point on the fixture's first-layer plane.
fn auto_point(x: f32, z: f32, width: f32, flow: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y: 0.0,
        z,
        width,
        flow_factor: flow,
        ..Default::default()
    }
}

/// Two points 10 mm apart, so each move is a real extrusion move.
fn two_point_path(z: f32, width: f32, flow: f32) -> Vec<Point3WithWidth> {
    vec![
        auto_point(0.0, z, width, flow),
        auto_point(10.0, z, width, flow),
    ]
}

/// An `OuterWall` entity assigned to `tool`, from explicit points.
fn outer_wall_entity(
    entity_id: u64,
    tool: u32,
    points: Vec<Point3WithWidth>,
    speed_factor: f32,
) -> PrintEntity {
    PrintEntity {
        entity_id,
        tool_index: tool,
        path: ExtrusionPath3D {
            points,
            speed_factor,
            ..extrusion_path3d_base(ExtrusionRole::OuterWall)
        },
        region_key: RegionKey {
            region_id: entity_id,
            global_layer_index: 0,
            object_id: "obj".to_string(),
            variant_chain: Vec::new(),
        },
        ..print_entity_base(ExtrusionRole::OuterWall)
    }
}

/// Every `Move.f` carrying the `OuterWall` role, in emission order.
fn outer_wall_f_values(gcode_ir: &GCodeIR) -> Vec<f32> {
    gcode_ir
        .commands
        .iter()
        .filter_map(|cmd| match cmd {
            GCodeCommand::Move {
                f: Some(f_val),
                role,
                ..
            } if *role == ExtrusionRole::OuterWall => Some(*f_val),
            _ => None,
        })
        .collect()
}

/// AC-1: a zero role speed is replaced by the volumetric limit resolved against
/// the live move geometry (width, first-layer `height_delta`, flow factor).
#[test]
fn zero_role_speed_uses_move_geometry_and_volumetric_limit() {
    // 8.0 mm³/s ÷ (0.4 × 0.2 × 1.0) = 100 mm/s × 60 = F6000.
    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z: Z_FIRST_LAYER,
        ordered_entities: vec![outer_wall_entity(
            1,
            0,
            two_point_path(Z_FIRST_LAYER, WIDTH_MM, FLOW),
            1.0,
        )],
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config(
        "1.0".to_string(),
        FeedrateConfig {
            outer_wall_speed: 0.0,
            ..Default::default()
        },
    )
    .with_resolved_config(config_with_volumetric_max(HEIGHT_MM, AUTO_MAX_MM3_S));

    let gcode_ir = emitter
        .emit_gcode(&[layer])
        .expect("valid automatic-speed inputs must emit");

    let moves: Vec<(f32, Option<f32>)> = gcode_ir
        .commands
        .iter()
        .filter_map(|cmd| match cmd {
            GCodeCommand::Move {
                f: Some(f_val),
                e,
                role,
                ..
            } if *role == ExtrusionRole::OuterWall => Some((*f_val, *e)),
            _ => None,
        })
        .collect();

    assert_eq!(moves.len(), 2, "two points must emit two outer-wall moves");
    assert_eq!(
        moves[0].0, 6000.0,
        "first point must carry the literal F6000 (auto speed), got {:?}",
        moves[0].0
    );
    assert_eq!(
        moves[1].0, 6000.0,
        "the second (extruding) point's move feedrate must be exactly F6000"
    );
    assert!(
        moves[1].1.is_some_and(|e| e > 0.0),
        "second point must be the extruding move, got e={:?}",
        moves[1].1
    );
    assert!(
        moves.iter().all(|(f, _)| f.is_finite() && *f > 0.0),
        "auto-speed feedrates must be finite and positive, got {moves:?}"
    );
}

/// AC-2: the volumetric limit is selected per tool (tool config over global),
/// while an explicit positive role speed is untouched by the auto branch.
#[test]
fn per_tool_volumetric_auto_and_explicit_speed_are_distinct() {
    // Part A — per-tool selection. Global 20.0 is a decoy: if tool selection
    // were lost, every F would resolve from 20.0 and the literals below fail.
    let mut tool_configs: BTreeMap<u32, ResolvedConfig> = BTreeMap::new();
    tool_configs.insert(0, config_with_volumetric_max(HEIGHT_MM, 8.0));
    tool_configs.insert(1, config_with_volumetric_max(HEIGHT_MM, 12.0));
    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z: Z_FIRST_LAYER,
        ordered_entities: vec![
            outer_wall_entity(1, 0, two_point_path(Z_FIRST_LAYER, WIDTH_MM, FLOW), 1.0),
            outer_wall_entity(2, 1, two_point_path(Z_FIRST_LAYER, WIDTH_MM, FLOW), 1.0),
        ],
        tool_changes: vec![ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config(
        "1.0".to_string(),
        FeedrateConfig {
            outer_wall_speed: 0.0,
            ..Default::default()
        },
    )
    .with_resolved_config(config_with_volumetric_max(HEIGHT_MM, 20.0))
    .with_tool_configs(tool_configs);

    let gcode_ir = emitter
        .emit_gcode(&[layer])
        .expect("valid per-tool automatic-speed inputs must emit");
    assert_eq!(
        outer_wall_f_values(&gcode_ir),
        vec![6000.0, 6000.0, 9000.0, 9000.0],
        "tool 0 → 8.0 mm³/s → F6000; tool 1 → 12.0 mm³/s → F9000"
    );

    // Part B — an explicit positive role speed never enters the auto branch.
    // `max = 0.0` proves the maximum is only consulted when a zero speed needs
    // it. `max = 1.0` is a positive ceiling below the explicit 30 mm/s speed:
    // applying it would emit F750, so the literal F1800 falsifies capping.
    // `max = 8.0` also proves the explicit value is not replaced by auto speed.
    for max in [0.0f32, 1.0f32, 8.0f32] {
        let layer = LayerCollectionIR {
            global_layer_index: 0,
            z: Z_FIRST_LAYER,
            ordered_entities: vec![outer_wall_entity(
                1,
                0,
                two_point_path(Z_FIRST_LAYER, WIDTH_MM, FLOW),
                1.0,
            )],
            ..Default::default()
        };
        let emitter = DefaultGCodeEmitter::new_with_config(
            "1.0".to_string(),
            FeedrateConfig {
                outer_wall_speed: 30.0,
                ..Default::default()
            },
        )
        .with_resolved_config(config_with_volumetric_max(HEIGHT_MM, max));

        let gcode_ir = emitter
            .emit_gcode(&[layer])
            .unwrap_or_else(|err| panic!("explicit speed with max={max} must emit: {err:?}"));
        assert_eq!(
            outer_wall_f_values(&gcode_ir),
            vec![1800.0, 1800.0],
            "explicit outer_wall_speed=30.0 mm/s must stay exactly F1800 (max={max})"
        );
    }
}

/// A missing config for the entity's tool falls back to the global volumetric
/// maximum, not tool 0's distinct per-tool value.
#[test]
fn missing_tool_volumetric_maximum_uses_global_value() {
    // Tool 1 has no entry; the tool-0 value is a decoy that catches a lost or
    // defaulted entity tool index (8.0 would emit F6000 instead of F15000).
    let mut tool_configs: BTreeMap<u32, ResolvedConfig> = BTreeMap::new();
    tool_configs.insert(0, config_with_volumetric_max(HEIGHT_MM, 8.0));
    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z: Z_FIRST_LAYER,
        ordered_entities: vec![outer_wall_entity(
            1,
            1,
            two_point_path(Z_FIRST_LAYER, WIDTH_MM, FLOW),
            1.0,
        )],
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config(
        "1.0".to_string(),
        FeedrateConfig {
            outer_wall_speed: 0.0,
            ..Default::default()
        },
    )
    .with_resolved_config(config_with_volumetric_max(HEIGHT_MM, 20.0))
    .with_tool_configs(tool_configs);

    let gcode_ir = emitter
        .emit_gcode(&[layer])
        .expect("valid global fallback automatic-speed inputs must emit");
    assert_eq!(
        outer_wall_f_values(&gcode_ir),
        vec![15000.0, 15000.0],
        "tool 1 has no override: global 20.0 mm³/s over 0.4 × 0.2 × 1.0 must emit literal F15000"
    );
}

/// Emit one zero-speed OuterWall entity from the given automatic-speed inputs;
/// returns the raw result so the negative-control test can inspect the failure.
fn try_auto_emit(
    max: f32,
    width: f32,
    first_layer_height: f64,
    flow: f32,
) -> Result<GCodeIR, GCodeEmitError> {
    try_auto_emit_with_factor(max, width, first_layer_height, flow, 1.0)
}

/// Emit one zero-speed OuterWall entity with the requested speed factor.
fn try_auto_emit_with_factor(
    max: f32,
    width: f32,
    first_layer_height: f64,
    flow: f32,
    speed_factor: f32,
) -> Result<GCodeIR, GCodeEmitError> {
    try_auto_emit_with_configs(
        config_with_volumetric_max(first_layer_height, max),
        BTreeMap::new(),
        0,
        width,
        flow,
        speed_factor,
    )
}

/// Run the real public emitter against explicit global and per-tool configs.
fn try_auto_emit_with_configs(
    global_config: ResolvedConfig,
    tool_configs: BTreeMap<u32, ResolvedConfig>,
    tool: u32,
    width: f32,
    flow: f32,
    speed_factor: f32,
) -> Result<GCodeIR, GCodeEmitError> {
    let layer = LayerCollectionIR {
        global_layer_index: 0,
        z: Z_FIRST_LAYER,
        ordered_entities: vec![outer_wall_entity(
            1,
            tool,
            two_point_path(Z_FIRST_LAYER, width, flow),
            speed_factor,
        )],
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config(
        "1.0".to_string(),
        FeedrateConfig {
            outer_wall_speed: 0.0,
            ..Default::default()
        },
    )
    .with_resolved_config(global_config)
    .with_tool_configs(tool_configs);
    emitter.emit_gcode(&[layer])
}

/// Asserts a fail-closed `GCodeEmitError::Emit` whose message names one of the
/// expected invalid inputs; an `Ok` carrying any `Move.f` is a failure.
fn assert_emit_error_naming(result: Result<GCodeIR, GCodeEmitError>, label: &str, tokens: &[&str]) {
    match result {
        Err(GCodeEmitError::Emit(message)) => {
            let lowered = message.to_lowercase();
            assert!(
                tokens.iter().any(|token| lowered.contains(token)),
                "{label}: GCodeEmitError::Emit must name one of {tokens:?}, got {message:?}"
            );
        }
        Err(other) => panic!("{label}: expected GCodeEmitError::Emit, got {other:?}"),
        Ok(gcode_ir) => {
            let f_values = outer_wall_f_values(&gcode_ir);
            panic!(
                "{label}: invalid automatic-speed inputs must fail closed, but emit_gcode \
                 returned Ok with Move.f = {f_values:?}"
            );
        }
    }
}

/// AC-N1: zero, negative, NaN, and infinite inputs, plus derived-speed f32
/// narrowing underflow and overflow, fail closed with `GCodeEmitError::Emit`
/// naming the input or conversion, and never reach an unsafe `Move.f`.
#[test]
fn invalid_volumetric_auto_inputs_fail_closed() {
    let max_token: &[&str] = &["filament_max_volumetric_speed"];
    let width_token: &[&str] = &["width"];
    let height_token: &[&str] = &["height"];
    let flow_token: &[&str] = &["flow"];

    let direct: [(&str, f32, f32, f64, f32, &[&str]); 16] = [
        ("max zero", 0.0, 0.4, 0.2, 1.0, max_token),
        ("max negative", -8.0, 0.4, 0.2, 1.0, max_token),
        ("max NaN", f32::NAN, 0.4, 0.2, 1.0, max_token),
        ("max infinite", f32::INFINITY, 0.4, 0.2, 1.0, max_token),
        ("width zero", 8.0, 0.0, 0.2, 1.0, width_token),
        ("width negative", 8.0, -0.4, 0.2, 1.0, width_token),
        ("width NaN", 8.0, f32::NAN, 0.2, 1.0, width_token),
        ("width infinite", 8.0, f32::INFINITY, 0.2, 1.0, width_token),
        ("height zero", 8.0, 0.4, 0.0, 1.0, height_token),
        ("height negative", 8.0, 0.4, -0.2, 1.0, height_token),
        ("height NaN", 8.0, 0.4, f64::NAN, 1.0, height_token),
        (
            "height infinite",
            8.0,
            0.4,
            f64::INFINITY,
            1.0,
            height_token,
        ),
        ("flow zero", 8.0, 0.4, 0.2, 0.0, flow_token),
        ("flow negative", 8.0, 0.4, 0.2, -1.0, flow_token),
        ("flow NaN", 8.0, 0.4, 0.2, f32::NAN, flow_token),
        ("flow infinite", 8.0, 0.4, 0.2, f32::INFINITY, flow_token),
    ];
    for (label, max, width, height, flow, tokens) in direct {
        assert_emit_error_naming(try_auto_emit(max, width, height, flow), label, tokens);
    }

    // The f64 product and quotient are finite, but this tiny positive quotient
    // narrows to f32 zero. Pin the existing conversion diagnostic specifically;
    // despite saying "overflows", it is also the current error for underflow.
    assert_emit_error_naming(
        try_auto_emit(8.0, 3.0e38, 3.0e38, 1.0),
        "derived speed underflow/narrowing to zero",
        &["derived speed overflows f32"],
    );

    // Unlike the narrowing-underflow case above, this finite quotient is larger
    // than f32::MAX and genuinely overflows when narrowed to the base speed.
    assert_emit_error_naming(
        try_auto_emit(3.0e38, 0.4, 0.2, 1.0),
        "automatic base speed overflow",
        &["derived speed overflows f32"],
    );
}

/// Automatic speed rejects non-finite factors rather than letting clamp
/// semantics turn infinities into valid-looking feedrates or propagate NaN.
#[test]
fn automatic_nonfinite_speed_factors_fail_closed() {
    for (label, factor) in [
        ("NaN speed factor", f32::NAN),
        ("positive-infinite speed factor", f32::INFINITY),
        ("negative-infinite speed factor", f32::NEG_INFINITY),
    ] {
        assert_emit_error_naming(
            try_auto_emit_with_factor(AUTO_MAX_MM3_S, WIDTH_MM, HEIGHT_MM, FLOW, factor),
            label,
            &["speed factor"],
        );
    }
}

/// Finite factors retain ADR-0052's clamp, including negative and oversized
/// values, on the automatic-speed path.
#[test]
fn automatic_finite_speed_factors_keep_clamp() {
    let negative = try_auto_emit_with_factor(AUTO_MAX_MM3_S, WIDTH_MM, HEIGHT_MM, FLOW, -1.0)
        .expect("finite negative speed factor must clamp and emit");
    assert_eq!(
        outer_wall_f_values(&negative),
        vec![300.0, 300.0],
        "factor -1 clamps to 0.05 and base 100 mm/s emits literal F300"
    );

    let oversized = try_auto_emit_with_factor(AUTO_MAX_MM3_S, WIDTH_MM, HEIGHT_MM, FLOW, 10.0)
        .expect("finite oversized speed factor must clamp and emit");
    assert_eq!(
        outer_wall_f_values(&oversized),
        vec![30000.0, 30000.0],
        "factor 10 clamps to 5.0 and base 100 mm/s emits literal F30000"
    );
}

/// A finite positive quotient/base is not enough: the final rounded F must
/// also remain finite and positive after f32 narrowing.
#[test]
fn automatic_final_feedrate_overflow_and_zero_fail_closed() {
    // max / 0.08 remains below f32::MAX, but multiplying the resulting base by
    // 60 and the clamped factor 5 exceeds f32::MAX at the final-F conversion.
    assert_emit_error_naming(
        try_auto_emit_with_factor(f32::MAX / 100.0, WIDTH_MM, HEIGHT_MM, FLOW, 5.0),
        "final feedrate overflow",
        &["derived feedrate"],
    );

    // 8e-12 / 0.08 = 1e-10 mm/s (a positive finite f32 base), whose F value
    // rounds to zero at the emitter's three-decimal precision.
    assert_emit_error_naming(
        try_auto_emit_with_factor(8.0e-12, WIDTH_MM, HEIGHT_MM, FLOW, 1.0),
        "final feedrate rounds to zero",
        &["derived feedrate"],
    );
}

/// A wrong-typed extension must fail with an error naming its registered key.
#[test]
fn automatic_malformed_volumetric_extension_fails_closed() {
    let mut global_config = ResolvedConfig {
        first_layer_height: HEIGHT_MM,
        ..Default::default()
    };
    global_config.extensions.insert(
        "filament_max_volumetric_speed".to_string(),
        ConfigValue::Bool(true),
    );

    assert_emit_error_naming(
        try_auto_emit_with_configs(global_config, BTreeMap::new(), 0, WIDTH_MM, FLOW, 1.0),
        "boolean volumetric maximum extension",
        &["filament_max_volumetric_speed"],
    );
}

/// A present tool override is authoritative even when invalid; it must not
/// widen to the valid global maximum of 20 mm³/s.
#[test]
fn automatic_invalid_tool_override_does_not_fall_back_to_global() {
    let mut malformed_tool = ResolvedConfig {
        first_layer_height: HEIGHT_MM,
        ..Default::default()
    };
    malformed_tool.extensions.insert(
        "filament_max_volumetric_speed".to_string(),
        ConfigValue::Bool(true),
    );

    for (label, tool_config) in [
        (
            "explicit zero tool maximum",
            config_with_volumetric_max(HEIGHT_MM, 0.0),
        ),
        ("malformed tool maximum", malformed_tool),
    ] {
        assert_emit_error_naming(
            try_auto_emit_with_configs(
                config_with_volumetric_max(HEIGHT_MM, 20.0),
                BTreeMap::from([(0, tool_config)]),
                0,
                WIDTH_MM,
                FLOW,
                1.0,
            ),
            label,
            &["filament_max_volumetric_speed"],
        );
    }
}

/// The automatic base must track each independent move input and flow through
/// the ADR-0052 factor clamp rather than bypassing it.
#[test]
fn zero_role_speed_auto_base_tracks_width_flow_factor_clamp_and_layer_height() {
    let first_layer = LayerCollectionIR {
        global_layer_index: 0,
        z: Z_FIRST_LAYER,
        ordered_entities: vec![
            // Per-point width on the destination point: 0.5 → 80 mm/s → F4800,
            // then 0.4 → 100 mm/s → F6000.
            outer_wall_entity(
                1,
                0,
                vec![
                    auto_point(0.0, Z_FIRST_LAYER, 0.5, 1.0),
                    auto_point(10.0, Z_FIRST_LAYER, 0.4, 1.0),
                ],
                1.0,
            ),
            // flow 2.0 doubles mm3_per_mm and halves speed → 50 mm/s → F3000.
            outer_wall_entity(2, 0, two_point_path(Z_FIRST_LAYER, 0.4, 2.0), 1.0),
            // factor 10 clamps to 5 → 80 mm/s × 5 = 400 mm/s → F24000.
            outer_wall_entity(3, 0, two_point_path(Z_FIRST_LAYER, 0.5, 1.0), 10.0),
            // factor 0.01 clamps to 0.05 → 80 mm/s × 0.05 = 4 mm/s → F240.
            outer_wall_entity(4, 0, two_point_path(Z_FIRST_LAYER, 0.5, 1.0), 0.01),
        ],
        ..Default::default()
    };
    let second_layer = LayerCollectionIR {
        global_layer_index: 1,
        // 0.5 − 0.2 = 0.3 mm height delta → 8.0 ÷ (0.4 × 0.3) = 66.667 mm/s
        // → F4000, proving the live layer delta (not the first-layer seed).
        z: 0.5,
        ordered_entities: vec![outer_wall_entity(5, 0, two_point_path(0.5, 0.4, 1.0), 1.0)],
        ..Default::default()
    };
    let emitter = DefaultGCodeEmitter::new_with_config(
        "1.0".to_string(),
        FeedrateConfig {
            outer_wall_speed: 0.0,
            ..Default::default()
        },
    )
    .with_resolved_config(config_with_volumetric_max(HEIGHT_MM, AUTO_MAX_MM3_S));

    let gcode_ir = emitter
        .emit_gcode(&[first_layer, second_layer])
        .expect("valid automatic-speed inputs must emit");
    assert_eq!(
        outer_wall_f_values(&gcode_ir),
        vec![
            4800.0, 6000.0, // per-point width (destination point)
            3000.0, 3000.0, // per-point flow 2.0
            24000.0, 24000.0, // factor 10 → clamp 5.0
            240.0, 240.0, // factor 0.01 → clamp 0.05
            4000.0, 4000.0, // layer height delta 0.3
        ],
        "each automatic feedrate must track its own move geometry and factor clamp"
    );
}
