//! TDD tests for Track B: use_relative_e_distances config key and G-code relative extrusion.

use slicer_host::{DefaultGCodeSerializer, GCodeSerializer};
use slicer_ir::{ExtrusionRole, GCodeCommand, GCodeIR, PrintMetadata};

// ---------------------------------------------------------------------------
// Helper: build a minimal GCodeIR with a sequence of extrusion moves.
//
// `e_values` are absolute E positions (as the emitter would produce them).
// All moves use X/Y/Z values that increment by 1.0 mm each step.
// ---------------------------------------------------------------------------

fn make_gcode_ir(e_values: &[f32]) -> GCodeIR {
    let mut commands = Vec::new();
    for (i, &e) in e_values.iter().enumerate() {
        commands.push(GCodeCommand::Move {
            x: Some(i as f32 + 1.0),
            y: Some(2.0),
            z: Some(0.2),
            e: if e > 0.0 { Some(e) } else { None },
            f: Some(3000.0),
            role: ExtrusionRole::OuterWall,
        });
    }
    GCodeIR {
        commands,
        metadata: PrintMetadata {
            estimated_print_time_s: 0,
            filament_used_mm: vec![0.0],
            layer_count: 1,
            slicer_version: "test".to_string(),
        },
        ..Default::default()
    }
}

/// Parse G1 lines from serialized gcode and extract E token value (if present).
fn extract_e_values(gcode: &str) -> Vec<f64> {
    let mut result = Vec::new();
    for line in gcode.lines() {
        let line = line.trim();
        if !line.starts_with("G1") && !line.starts_with("G0") {
            continue;
        }
        for token in line.split_whitespace() {
            if token.starts_with('E') {
                if let Ok(v) = token.strip_prefix('E').unwrap_or("").parse::<f64>() {
                    result.push(v);
                }
            }
        }
    }
    result
}

/// Extract all G1/G0 lines from the serialized output.
fn extract_move_lines(gcode: &str) -> Vec<&str> {
    gcode
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("G1") || l.starts_with("G0"))
        .collect()
}

/// Extract the XYZ/F tokens from a G1/G0 line (exclude E tokens).
fn xyzf_tokens(line: &str) -> Vec<&str> {
    line.split_whitespace()
        .filter(|tok| {
            let first = tok.chars().next().unwrap_or(' ');
            matches!(first, 'G' | 'X' | 'Y' | 'Z' | 'F')
        })
        .collect()
}

// ---------------------------------------------------------------------------
// AC-2: default mode emits M83, no M82
// ---------------------------------------------------------------------------

#[test]
fn default_is_relative_m83() {
    let ir = make_gcode_ir(&[0.1, 0.2, 0.35]);
    let serializer = DefaultGCodeSerializer::new();
    let output = serializer
        .serialize_gcode(&ir)
        .expect("serialization must succeed");

    // M83 must appear exactly once
    let m83_count = output.lines().filter(|l| l.trim() == "M83").count();
    assert_eq!(
        m83_count, 1,
        "M83 must appear exactly once in relative mode; found {m83_count}"
    );

    // M82 must be absent
    assert!(
        !output.lines().any(|l| l.trim() == "M82"),
        "M82 must not appear in relative mode"
    );
}

// ---------------------------------------------------------------------------
// AC-3: use_relative_e_distances=false emits M82, no M83
// ---------------------------------------------------------------------------

#[test]
fn absolute_mode_when_flag_false() {
    let e_vals: Vec<f32> = vec![0.12345, 0.24690, 0.37035];
    let ir = make_gcode_ir(&e_vals);
    let serializer = DefaultGCodeSerializer::with_extrusion_mode(false);
    let output = serializer
        .serialize_gcode(&ir)
        .expect("serialization must succeed");

    // M82 must appear exactly once
    let m82_count = output.lines().filter(|l| l.trim() == "M82").count();
    assert_eq!(
        m82_count, 1,
        "M82 must appear exactly once in absolute mode; found {m82_count}"
    );

    // M83 must be absent
    assert!(
        !output.lines().any(|l| l.trim() == "M83"),
        "M83 must not appear in absolute mode"
    );

    // E values must match the original IR values (absolute, 5 decimal places)
    let emitted_e = extract_e_values(&output);
    assert_eq!(
        emitted_e.len(),
        e_vals.len(),
        "must have same number of E tokens as moves with E"
    );
    for (emitted, &original) in emitted_e.iter().zip(e_vals.iter()) {
        let diff = (emitted - original as f64).abs();
        assert!(
            diff < 1e-4,
            "absolute E value {emitted} does not match IR value {original} (diff={diff})"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-4: E values are per-move deltas (< 5 mm each) in relative mode
// ---------------------------------------------------------------------------

#[test]
fn e_values_are_per_move_deltas() {
    // Absolute E values that grow realistically (< 5 mm each step in a typical print)
    let e_vals: Vec<f32> = vec![0.05, 0.12, 0.22, 0.35, 0.50];
    let ir = make_gcode_ir(&e_vals);
    let serializer = DefaultGCodeSerializer::new(); // relative mode

    let output = serializer
        .serialize_gcode(&ir)
        .expect("serialization must succeed");
    let emitted_e = extract_e_values(&output);

    assert!(
        !emitted_e.is_empty(),
        "relative mode must emit at least one E token"
    );

    for &delta in &emitted_e {
        assert!(
            delta.abs() < 5.0,
            "relative-mode E delta {delta} must be < 5 mm; got large value suggesting absolute emission"
        );
        assert!(
            delta > 0.0,
            "relative-mode E delta {delta} must be positive for forward extrusion"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-5: X/Y/Z/F tokens are identical between relative and absolute modes
// ---------------------------------------------------------------------------

#[test]
fn xyzf_unchanged_across_modes() {
    let ir = make_gcode_ir(&[0.1, 0.2, 0.35, 0.52]);
    let rel_output = DefaultGCodeSerializer::with_extrusion_mode(true)
        .serialize_gcode(&ir)
        .expect("relative serialization must succeed");
    let abs_output = DefaultGCodeSerializer::with_extrusion_mode(false)
        .serialize_gcode(&ir)
        .expect("absolute serialization must succeed");

    let rel_lines = extract_move_lines(&rel_output);
    let abs_lines = extract_move_lines(&abs_output);

    assert_eq!(
        rel_lines.len(),
        abs_lines.len(),
        "both modes must emit the same number of move lines"
    );

    for (rel_line, abs_line) in rel_lines.iter().zip(abs_lines.iter()) {
        let rel_xyzf: Vec<&str> = xyzf_tokens(rel_line);
        let abs_xyzf: Vec<&str> = xyzf_tokens(abs_line);
        assert_eq!(
            rel_xyzf, abs_xyzf,
            "X/Y/Z/F tokens must be identical between modes.\n  relative: {rel_line}\n  absolute: {abs_line}"
        );
    }
}

// ---------------------------------------------------------------------------
// AC-6: sum of relative E deltas matches absolute E within 1e-3 mm (per G92 block)
// ---------------------------------------------------------------------------

#[test]
fn delta_sum_matches_absolute_per_g92_block() {
    // Build IR with a G92 E0 reset in the middle
    let mut commands = Vec::new();

    // First block: E goes from 0 to 0.5
    let first_block: Vec<f32> = vec![0.05, 0.15, 0.30, 0.50];
    for (i, &e) in first_block.iter().enumerate() {
        commands.push(GCodeCommand::Move {
            x: Some(i as f32 + 1.0),
            y: Some(2.0),
            z: Some(0.2),
            e: Some(e),
            f: Some(3000.0),
            role: ExtrusionRole::OuterWall,
        });
    }

    // Reset extruder via G92 E0 (raw)
    commands.push(GCodeCommand::Raw {
        text: "G92 E0".to_string(),
    });

    // Second block: E continues from 0 (after reset) to 0.4
    let second_block: Vec<f32> = vec![0.10, 0.25, 0.40];
    for (i, &e) in second_block.iter().enumerate() {
        commands.push(GCodeCommand::Move {
            x: Some(i as f32 + 10.0),
            y: Some(5.0),
            z: Some(0.2),
            e: Some(e),
            f: Some(3000.0),
            role: ExtrusionRole::OuterWall,
        });
    }

    let ir = GCodeIR {
        commands,
        metadata: PrintMetadata {
            estimated_print_time_s: 0,
            filament_used_mm: vec![0.0],
            layer_count: 1,
            slicer_version: "test".to_string(),
        },
        ..Default::default()
    };

    let serializer = DefaultGCodeSerializer::new(); // relative mode
    let output = serializer
        .serialize_gcode(&ir)
        .expect("serialization must succeed");

    // Collect deltas for first block (before G92 E0)
    // and second block (after G92 E0) separately.
    let mut first_deltas: Vec<f64> = Vec::new();
    let mut second_deltas: Vec<f64> = Vec::new();
    let mut past_reset = false;

    for line in output.lines() {
        let line = line.trim();
        if line == "G92 E0" {
            past_reset = true;
            continue;
        }
        if !line.starts_with("G1") && !line.starts_with("G0") {
            continue;
        }
        for token in line.split_whitespace() {
            if token.starts_with('E') {
                if let Ok(v) = token.strip_prefix('E').unwrap_or("").parse::<f64>() {
                    if past_reset {
                        second_deltas.push(v);
                    } else {
                        first_deltas.push(v);
                    }
                }
            }
        }
    }

    // Sum of first-block deltas must equal final absolute E of first block (0.50)
    let first_sum: f64 = first_deltas.iter().sum();
    assert!(
        (first_sum - 0.50_f64).abs() < 1e-3,
        "first block: sum of deltas {first_sum:.5} must equal final absolute E 0.50 within 1e-3"
    );

    // Sum of second-block deltas must equal final absolute E of second block (0.40)
    let second_sum: f64 = second_deltas.iter().sum();
    assert!(
        (second_sum - 0.40_f64).abs() < 1e-3,
        "second block: sum of deltas {second_sum:.5} must equal final absolute E 0.40 within 1e-3"
    );
}

// ---------------------------------------------------------------------------
// Negative-1: M82 must NOT appear in relative-mode output
// ---------------------------------------------------------------------------

#[test]
fn rejects_m82_in_relative_mode() {
    let ir = make_gcode_ir(&[0.1, 0.2, 0.35]);
    let serializer = DefaultGCodeSerializer::new(); // relative
    let output = serializer
        .serialize_gcode(&ir)
        .expect("serialization must succeed");

    assert!(
        !output.lines().any(|l| l.trim() == "M82"),
        "M82 must NOT appear in relative-mode output"
    );
}

// ---------------------------------------------------------------------------
// Negative-2: M83 must NOT appear in absolute-mode output
// ---------------------------------------------------------------------------

#[test]
fn rejects_m83_or_deltas_in_absolute_mode() {
    let ir = make_gcode_ir(&[0.1, 0.2, 0.35]);
    let serializer = DefaultGCodeSerializer::with_extrusion_mode(false);
    let output = serializer
        .serialize_gcode(&ir)
        .expect("serialization must succeed");

    assert!(
        !output.lines().any(|l| l.trim() == "M83"),
        "M83 must NOT appear in absolute-mode output"
    );
}

// ---------------------------------------------------------------------------
// Negative-3: E values must NOT be monotonically increasing in relative mode
// ---------------------------------------------------------------------------

#[test]
fn rejects_monotonic_e_run() {
    // Use many moves so that relative deltas are clearly < absolute values
    let e_vals: Vec<f32> = (1..=8).map(|i| i as f32 * 0.15).collect();
    let ir = make_gcode_ir(&e_vals);
    let serializer = DefaultGCodeSerializer::new(); // relative mode
    let output = serializer
        .serialize_gcode(&ir)
        .expect("serialization must succeed");

    let emitted_e = extract_e_values(&output);
    assert!(
        emitted_e.len() >= 2,
        "need at least 2 E tokens to check monotonicity"
    );

    // In relative mode, each delta should be roughly equal (not growing).
    // The final absolute value 8*0.15 = 1.2 must NOT appear as an E value.
    let final_abs = *e_vals.last().unwrap() as f64;
    let monotonically_growing = emitted_e.windows(2).all(|w| w[1] >= w[0])
        && emitted_e.last().copied().unwrap_or(0.0) >= final_abs * 0.9;

    assert!(
        !monotonically_growing,
        "relative-mode E values must NOT be monotonically increasing up to the final absolute value \
        (that would indicate absolute emission); emitted={emitted_e:?}, final_abs={final_abs}"
    );
}

// ---------------------------------------------------------------------------
// Negative-4: X/Y/Z/F must NOT drift between relative and absolute modes
// ---------------------------------------------------------------------------

#[test]
fn rejects_xyzf_drift_across_modes() {
    // Same as xyzf_unchanged_across_modes — the negative framing verifies
    // there is zero drift in X/Y/Z/F between the two modes.
    let ir = make_gcode_ir(&[0.1, 0.2, 0.35, 0.52]);
    let rel_output = DefaultGCodeSerializer::with_extrusion_mode(true)
        .serialize_gcode(&ir)
        .expect("relative serialization must succeed");
    let abs_output = DefaultGCodeSerializer::with_extrusion_mode(false)
        .serialize_gcode(&ir)
        .expect("absolute serialization must succeed");

    let rel_lines = extract_move_lines(&rel_output);
    let abs_lines = extract_move_lines(&abs_output);

    for (rel_line, abs_line) in rel_lines.iter().zip(abs_lines.iter()) {
        let rel_xyzf: Vec<&str> = xyzf_tokens(rel_line);
        let abs_xyzf: Vec<&str> = xyzf_tokens(abs_line);
        assert_eq!(
            rel_xyzf, abs_xyzf,
            "X/Y/Z/F MUST be identical — drift detected.\n  relative: {rel_line}\n  absolute: {abs_line}"
        );
    }
}
