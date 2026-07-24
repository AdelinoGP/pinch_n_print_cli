//! Behavioral tests for the machine-gcode-emit `PostpassModule`.
//!
//! Exercises `run_gcode_postprocess` through the public trait API: start/end
//! G-code prepend/append, single-pass `[key]` placeholder substitution (known
//! keys, unknown-key passthrough, unclosed bracket, multiline), and verbatim
//! pass-through of every `GCodeCommand` variant.

#![allow(missing_docs)]

use machine_gcode_emit::MachineGcodeEmit;
use slicer_ir::{ConfigValue, ExtrusionRole, GCodeCommand, RetractMode};
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::postpass_types::GcodeOutputCommand;
use slicer_sdk::test_prelude::config_with;
use slicer_sdk::traits::PostpassModule;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run(config_pairs: &[(&str, ConfigValue)], commands: &[GCodeCommand]) -> GcodeOutputBuilder {
    let cfg = config_with(config_pairs);
    let module = MachineGcodeEmit::on_print_start(&cfg).expect("on_print_start must succeed");
    let mut output = GcodeOutputBuilder::new();
    module
        .run_gcode_postprocess(commands, &mut output, &cfg)
        .expect("run_gcode_postprocess must succeed");
    output
}

/// Raw command bodies in emission order.
fn raw_texts(output: &GcodeOutputBuilder) -> Vec<String> {
    output
        .commands()
        .iter()
        .filter_map(|c| match c {
            GcodeOutputCommand::Command(GCodeCommand::Raw { text }) => Some(text.clone()),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Start / end gcode framing
// ---------------------------------------------------------------------------

#[test]
fn start_and_end_gcode_frame_the_command_stream() {
    let output = run(
        &[
            ("machine_start_gcode", ConfigValue::String("START".into())),
            ("machine_end_gcode", ConfigValue::String("END".into())),
        ],
        &[GCodeCommand::FanSpeed { value: 255 }],
    );

    let cmds = output.commands();
    // Position of START raw, the FanSpeed command, and END raw.
    let start_idx = cmds.iter().position(
        |c| matches!(c, GcodeOutputCommand::Command(GCodeCommand::Raw { text }) if text == "START"),
    );
    let fan_idx = cmds.iter().position(|c| {
        matches!(
            c,
            GcodeOutputCommand::Command(GCodeCommand::FanSpeed { value: 255 })
        )
    });
    let end_idx = cmds.iter().position(
        |c| matches!(c, GcodeOutputCommand::Command(GCodeCommand::Raw { text }) if text == "END"),
    );

    let (s, f, e) = (
        start_idx.expect("START must be emitted"),
        fan_idx.expect("FanSpeed must be re-emitted"),
        end_idx.expect("END must be emitted"),
    );
    assert!(
        s < f && f < e,
        "order must be START({s}) < FanSpeed({f}) < END({e})"
    );
}

#[test]
fn empty_templates_emit_no_raw_wrappers() {
    let output = run(&[], &[GCodeCommand::FanSpeed { value: 128 }]);
    assert!(
        raw_texts(&output).is_empty(),
        "absent start/end gcode must emit no Raw wrapper commands"
    );
    assert_eq!(
        output.commands().len(),
        1,
        "only the single input command should pass through"
    );
}

#[test]
fn whitespace_only_template_is_skipped() {
    let output = run(
        &[("machine_start_gcode", ConfigValue::String("   \n  ".into()))],
        &[GCodeCommand::FanSpeed { value: 128 }],
    );
    assert!(
        raw_texts(&output).is_empty(),
        "a whitespace-only template must not emit a Raw command"
    );
}

// ---------------------------------------------------------------------------
// Placeholder substitution
// ---------------------------------------------------------------------------

#[test]
fn known_placeholder_is_substituted() {
    let output = run(
        &[
            (
                "machine_start_gcode",
                ConfigValue::String("M140 S[bed_temperature_initial_layer_single]".into()),
            ),
            ("bed_temperature_initial_layer_single", ConfigValue::Int(60)),
        ],
        &[],
    );
    assert!(
        raw_texts(&output).contains(&"M140 S60".to_string()),
        "known [key] must be substituted: {:?}",
        raw_texts(&output)
    );
}

#[test]
fn unknown_placeholder_passes_through_verbatim() {
    let output = run(
        &[(
            "machine_start_gcode",
            ConfigValue::String("X[unknown_key]Y".into()),
        )],
        &[],
    );
    assert!(
        raw_texts(&output).contains(&"X[unknown_key]Y".to_string()),
        "unknown [key] (incl. brackets) must pass through: {:?}",
        raw_texts(&output)
    );
}

#[test]
fn unclosed_bracket_is_literal() {
    let output = run(
        &[(
            "machine_start_gcode",
            ConfigValue::String("hello [world".into()),
        )],
        &[],
    );
    assert!(
        raw_texts(&output).contains(&"hello [world".to_string()),
        "an unclosed '[' must be treated as literal text: {:?}",
        raw_texts(&output)
    );
}

#[test]
fn multiline_template_substitutes_per_line() {
    let output = run(
        &[
            (
                "machine_start_gcode",
                ConfigValue::String("L1\n[bed_temperature_initial_layer_single]\nL3".into()),
            ),
            ("bed_temperature_initial_layer_single", ConfigValue::Int(60)),
        ],
        &[],
    );
    assert!(
        raw_texts(&output).contains(&"L1\n60\nL3".to_string()),
        "multiline template must substitute inline and keep newlines: {:?}",
        raw_texts(&output)
    );
}

// ---------------------------------------------------------------------------
// Command pass-through
// ---------------------------------------------------------------------------

#[test]
fn all_command_variants_pass_through_in_order() {
    let commands = vec![
        GCodeCommand::Move {
            x: Some(1.0),
            y: Some(2.0),
            z: None,
            e: Some(0.5),
            f: Some(1200.0),
            role: ExtrusionRole::OuterWall,
        },
        GCodeCommand::Retract {
            length: 1.0,
            speed: 30.0,
            mode: RetractMode::Gcode,
        },
        GCodeCommand::Unretract {
            length: 1.0,
            speed: 30.0,
            mode: RetractMode::Gcode,
        },
        GCodeCommand::FanSpeed { value: 200 },
        GCodeCommand::Temperature {
            tool: 0,
            celsius: 210.0,
            wait: false,
        },
        GCodeCommand::ToolChange {
            after_entity_index: 0,
            from: 0,
            to: 1,
        },
        GCodeCommand::Comment {
            text: "hello".into(),
        },
        GCodeCommand::Raw { text: "G28".into() },
    ];
    let output = run(&[], &commands);

    let emitted: Vec<&GCodeCommand> = output
        .commands()
        .iter()
        .filter_map(|c| match c {
            GcodeOutputCommand::Command(inner) => Some(inner),
            _ => None,
        })
        .collect();

    assert_eq!(
        emitted.len(),
        commands.len(),
        "every input command must be re-emitted exactly once"
    );
    for (got, want) in emitted.iter().zip(commands.iter()) {
        assert_eq!(
            *got, want,
            "command pass-through must preserve order and content"
        );
    }
}

/// Regression pin: `machine_start_gcode` must precede BOTH the M73 progress
/// pair and the `ExtrusionMode` declaration.
///
/// `DefaultGCodeEmitter::emit_gcode` builds its stream with `ExtrusionMode`
/// first and then calls `inject_m73`, which *prepends* an `M73 P0 R<n>` /
/// `M73 Q0 S<n>` pair — so by the time this module sees the stream,
/// `ExtrusionMode` sits at index 2, not index 0. `emit.rs` still documents an
/// "ExtrusionMode at index 0 so the postpass can prepend machine_start_gcode
/// before it" rationale; that index is now wrong even though the *ordering* it
/// was protecting still holds, because this module rebuilds the stream (start
/// template, then every input command in order) rather than splicing at an
/// index.
///
/// The ordering holds by construction rather than by the index the comment
/// named, which is exactly the kind of accident worth pinning: a future change
/// that reintroduced an `insert(0, ..)` would put the start block *after* the
/// M73 pair and emit progress reporting before the printer is homed.
#[test]
fn machine_start_gcode_precedes_m73_and_extrusion_mode() {
    // Shaped like a real post-`inject_m73` emitter stream.
    let commands = vec![
        GCodeCommand::Raw {
            text: "M73 P0 R10".into(),
        },
        GCodeCommand::Raw {
            text: "M73 Q0 S10".into(),
        },
        GCodeCommand::ExtrusionMode { absolute: true },
        GCodeCommand::Raw {
            text: ";LAYER_CHANGE".into(),
        },
    ];

    let output = run(
        &[(
            "machine_start_gcode",
            ConfigValue::String("G28 ; home".into()),
        )],
        &commands,
    );

    let cmds = output.commands();
    let position_of = |pred: &dyn Fn(&GcodeOutputCommand) -> bool| {
        cmds.iter()
            .position(|c| pred(c))
            .unwrap_or_else(|| panic!("command not found in {cmds:#?}"))
    };

    let start_at = position_of(
        &|c| matches!(c, GcodeOutputCommand::Command(GCodeCommand::Raw { text }) if text == "G28 ; home"),
    );
    let first_m73_at = position_of(
        &|c| matches!(c, GcodeOutputCommand::Command(GCodeCommand::Raw { text }) if text.starts_with("M73 ")),
    );
    // The module lowers `ExtrusionMode` to its `M82`/`M83` raw form on
    // re-emit, so accept either shape.
    let extrusion_mode_at = position_of(&|c| match c {
        GcodeOutputCommand::Command(GCodeCommand::ExtrusionMode { .. }) => true,
        GcodeOutputCommand::Command(GCodeCommand::Raw { text }) => text == "M82" || text == "M83",
        _ => false,
    });

    assert!(
        start_at < first_m73_at,
        "machine_start_gcode must precede the M73 progress pair; got start at \
         {start_at}, first M73 at {first_m73_at} in {cmds:#?}"
    );
    assert!(
        start_at < extrusion_mode_at,
        "machine_start_gcode must precede the ExtrusionMode declaration; got \
         start at {start_at}, ExtrusionMode at {extrusion_mode_at} in {cmds:#?}"
    );
}
