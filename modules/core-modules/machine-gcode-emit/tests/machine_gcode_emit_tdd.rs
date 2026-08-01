//! Behavioral tests for the machine-gcode-emit `PostpassModule`.
//!
//! Exercises `run_gcode_postprocess` through the public trait API: start/end
//! G-code prepend/append, single-pass `[key]` placeholder substitution (known
//! keys, verbatim pass-through of unresolved keys, unclosed bracket, multiline,
//! non-ASCII template text, legacy aliases), and verbatim pass-through of every
//! `GCodeCommand` variant.

#![allow(missing_docs)]

use machine_gcode_emit::MachineGcodeEmit;
use slicer_ir::{ConfigValue, ExtrusionRole, GCodeCommand, RetractMode};
use slicer_sdk::error::ModuleError;
use slicer_sdk::host::test_support::{install_log_capture, take_log_messages};
use slicer_sdk::host::LogLevel;
use slicer_sdk::postpass_builders::GcodeOutputBuilder;
use slicer_sdk::postpass_types::GcodeOutputCommand;
use slicer_sdk::test_prelude::config_with;
use slicer_sdk::traits::PostpassModule;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn run(config_pairs: &[(&str, ConfigValue)], commands: &[GCodeCommand]) -> GcodeOutputBuilder {
    let cfg = config_with(config_pairs);
    let module = MachineGcodeEmit::from_config(&cfg).expect("from_config must succeed");
    let mut output = GcodeOutputBuilder::new();
    module
        .run_gcode_postprocess(commands, &mut output, &cfg)
        .expect("run_gcode_postprocess must succeed");
    output
}

/// Like [`run`], but surfaces the module's `Result` instead of unwrapping it.
/// Returns the (possibly partially filled) output builder alongside it so a
/// test can assert nothing was emitted on the error path.
fn try_run(
    config_pairs: &[(&str, ConfigValue)],
    commands: &[GCodeCommand],
) -> (Result<(), ModuleError>, GcodeOutputBuilder) {
    let cfg = config_with(config_pairs);
    let module = MachineGcodeEmit::from_config(&cfg).expect("from_config must succeed");
    let mut output = GcodeOutputBuilder::new();
    let result = module.run_gcode_postprocess(commands, &mut output, &cfg);
    (result, output)
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
    install_log_capture();
    let output = run(&[], &[GCodeCommand::FanSpeed { value: 128 }]);
    let logs = take_log_messages();
    assert!(
        raw_texts(&output).is_empty(),
        "absent start/end gcode must emit no Raw wrapper commands"
    );
    assert_eq!(
        output.commands().len(),
        1,
        "only the single input command should pass through"
    );
    assert!(
        logs.is_empty(),
        "absent templates must not log any unresolved-placeholder warning: {logs:?}"
    );
}

#[test]
fn whitespace_only_template_is_skipped() {
    install_log_capture();
    let output = run(
        &[("machine_start_gcode", ConfigValue::String("   \n  ".into()))],
        &[GCodeCommand::FanSpeed { value: 128 }],
    );
    let logs = take_log_messages();
    assert!(
        raw_texts(&output).is_empty(),
        "a whitespace-only template must not emit a Raw command"
    );
    assert!(
        logs.is_empty(),
        "a whitespace-only template must not log any unresolved-placeholder warning: {logs:?}"
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

/// An unresolved `[key]` is **not** a slice error. A module's `ConfigView` is
/// scoped to its own manifest, so a template may legitimately name a key owned
/// by a module that is not loaded. The key passes through verbatim (the module
/// warns once) and the slice proceeds.
#[test]
fn unknown_placeholder_passes_through_verbatim() {
    let (result, output) = try_run(
        &[(
            "machine_start_gcode",
            ConfigValue::String("X[unknown_key]Y".into()),
        )],
        &[],
    );

    result.expect("an unresolved [key] must not fail the slice");
    assert!(
        raw_texts(&output).contains(&"X[unknown_key]Y".to_string()),
        "unresolved [key] must be emitted verbatim, brackets included: {:?}",
        raw_texts(&output)
    );
}

#[test]
fn non_ascii_template_text_survives_substitution() {
    let output = run(
        &[
            (
                "machine_start_gcode",
                ConfigValue::String(
                    "; café ☕ M140 S[bed_temperature_initial_layer_single]".into(),
                ),
            ),
            ("bed_temperature_initial_layer_single", ConfigValue::Int(60)),
        ],
        &[],
    );

    let want = "; café ☕ M140 S60".to_string();
    let got = raw_texts(&output);
    assert!(
        got.contains(&want),
        "non-ASCII literal text must survive byte-identically; want {:?}, got {:?}",
        want,
        got
    );
}

/// Every unresolved key across *both* injection points survives verbatim into
/// the emitted text, and the module logs exactly **one** aggregated warning:
/// keys sorted and deduplicated across sites (the same key in both templates
/// appears once), and both contributing injection points named.
#[test]
fn every_unresolved_placeholder_passes_through_verbatim() {
    install_log_capture();
    let (result, output) = try_run(
        &[
            (
                "machine_start_gcode",
                ConfigValue::String("A[zzz_two]B[zzz_one]C".into()),
            ),
            (
                "machine_end_gcode",
                ConfigValue::String("D[zzz_three]E[zzz_one]F".into()),
            ),
        ],
        &[],
    );
    let logs = take_log_messages();

    result.expect("unresolved placeholders must not fail the slice");

    let emitted = raw_texts(&output);
    assert!(
        emitted.contains(&"A[zzz_two]B[zzz_one]C".to_string()),
        "both start-template placeholders must survive verbatim: {emitted:?}"
    );
    assert!(
        emitted.contains(&"D[zzz_three]E[zzz_one]F".to_string()),
        "the end-template placeholders must survive verbatim: {emitted:?}"
    );

    let warnings: Vec<&str> = logs
        .iter()
        .filter(|(level, _)| *level == LogLevel::Warn)
        .map(|(_, msg)| msg.as_str())
        .collect();
    assert_eq!(
        warnings.len(),
        1,
        "unresolved keys across both templates must produce exactly one aggregated warning: {logs:?}"
    );
    let warning = warnings[0];
    let occurrences = |needle: &str| warning.matches(needle).count();
    assert_eq!(
        occurrences("[zzz_one]"),
        1,
        "a key present in both templates must appear exactly once (sorted, deduplicated): {warning}"
    );
    assert_eq!(
        occurrences("[zzz_two]"),
        1,
        "key must appear once: {warning}"
    );
    assert_eq!(
        occurrences("[zzz_three]"),
        1,
        "key must appear once: {warning}"
    );
    let (one_at, three_at, two_at) = (
        warning.find("[zzz_one]").expect("[zzz_one] must be named"),
        warning
            .find("[zzz_three]")
            .expect("[zzz_three] must be named"),
        warning.find("[zzz_two]").expect("[zzz_two] must be named"),
    );
    assert!(
        one_at < three_at && three_at < two_at,
        "keys must be listed in sorted order (zzz_one < zzz_three < zzz_two): {warning}"
    );
    assert_eq!(
        occurrences("machine_start_gcode"),
        1,
        "the start injection point must be named exactly once: {warning}"
    );
    assert_eq!(
        occurrences("machine_end_gcode"),
        1,
        "the end injection point must be named exactly once: {warning}"
    );
}

#[test]
fn first_layer_temperature_alias_resolves_to_nozzle_temperature_initial_layer() {
    let (result, output) = try_run(
        &[
            (
                "machine_start_gcode",
                ConfigValue::String("M109 S[first_layer_temperature]".into()),
            ),
            ("nozzle_temperature_initial_layer", ConfigValue::Int(215)),
        ],
        &[],
    );
    result.expect("the legacy first_layer_temperature alias must resolve");
    assert!(
        raw_texts(&output).contains(&"M109 S215".to_string()),
        "[first_layer_temperature] must alias nozzle_temperature_initial_layer: {:?}",
        raw_texts(&output)
    );
}

/// Regression pin (packet 186 / F2): real 3MF input supplies per-extruder
/// settings as vectors, so `nozzle_diameter` reaches the module as
/// `ConfigValue::List(['0.4'])`, never as a scalar. The `config.keys()` sweep
/// used to drop every `List` on its catch-all arm, which made the packet's
/// headline `[nozzle_diameter]` placeholder inert for every real slice while
/// the scalar-schema-default unit test stayed green.
#[test]
fn list_valued_config_key_resolves_from_first_element() {
    let (result, output) = try_run(
        &[
            (
                "machine_start_gcode",
                ConfigValue::String("; nozzle is [nozzle_diameter]".into()),
            ),
            (
                "nozzle_diameter",
                ConfigValue::List(vec![ConfigValue::Float(0.4)]),
            ),
        ],
        &[],
    );
    result.expect("a list-valued config key must resolve from its first element");
    assert!(
        raw_texts(&output).contains(&"; nozzle is 0.4".to_string()),
        "[nozzle_diameter] must render element 0 of the per-extruder vector: {:?}",
        raw_texts(&output)
    );
}

/// An *empty* list must stay unresolved rather than substitute an empty string:
/// silently emitting `M104 S` is the failure mode `design.md` §Rejected
/// alternatives rules out.
#[test]
fn empty_list_config_key_passes_through_verbatim() {
    let (result, output) = try_run(
        &[
            (
                "machine_start_gcode",
                ConfigValue::String("M104 S[nozzle_temperature]".into()),
            ),
            ("nozzle_temperature", ConfigValue::List(vec![])),
        ],
        &[],
    );

    result.expect("an unresolved [key] must not fail the slice");
    let emitted = raw_texts(&output);
    assert!(
        emitted.contains(&"M104 S[nozzle_temperature]".to_string()),
        "an empty-list key must stay unresolved and be emitted verbatim: {emitted:?}"
    );
    assert!(
        !emitted.contains(&"M104 S".to_string()),
        "an empty list must never substitute an empty string: {emitted:?}"
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
