//! Golden serialization test for `slicer-gcode` (AC-8 / AC-N5).
//!
//! Constructs a tiny `GCodeIR` containing the documented OrcaSlicer-parity
//! sentinels (`;LAYER_CHANGE`, `;Z:...`, `;TYPE:Outer wall`, `;TYPE:Sparse
//! infill`) plus a wall move, an infill move, and a travel move, then asserts
//! the serialized output contains the expected substrings.
//!
//! This test exercises `DefaultGCodeSerializer` end-to-end **without** any
//! `slicer-runtime`, `slicer-scheduler`, or `slicer-wasm-host` imports — it
//! relies solely on `slicer_gcode` and `slicer_ir`.

use slicer_gcode::{DefaultGCodeSerializer, GCodeSerializer};
use slicer_ir::{ExtrusionRole, GCodeCommand, GCodeIR, PrintMetadata, RetractMode};

/// Build a minimal `GCodeIR` representing one layer with:
///   * a `;LAYER_CHANGE` marker and `;Z:` height comment (Raw)
///   * a wall `;TYPE:Outer wall` comment + extrusion `Move`
///   * a `;TYPE:Sparse infill` comment + extrusion `Move`
///   * a travel `Move` (role = Custom("Travel"), no E)
///   * a `Retract` to exercise non-Move serialization
fn build_minimal_gcode_ir() -> GCodeIR {
    let commands = vec![
        // Layer-change header (OrcaSlicer parity, see emit.rs:272/275).
        GCodeCommand::Raw {
            text: ";LAYER_CHANGE".to_string(),
        },
        GCodeCommand::Raw {
            text: ";Z:0.200".to_string(),
        },
        GCodeCommand::Raw {
            text: ";HEIGHT:0.200".to_string(),
        },
        // Outer-wall entry: ;TYPE:Outer wall comment + one extrusion move.
        GCodeCommand::Raw {
            text: ";TYPE:Outer wall".to_string(),
        },
        GCodeCommand::Move {
            x: Some(10.0),
            y: Some(10.0),
            z: Some(0.2),
            e: Some(0.5),
            f: Some(1800.0),
            role: ExtrusionRole::OuterWall,
        },
        // Sparse-infill entry: ;TYPE:Sparse infill comment + one extrusion move.
        GCodeCommand::Raw {
            text: ";TYPE:Sparse infill".to_string(),
        },
        GCodeCommand::Move {
            x: Some(20.0),
            y: Some(20.0),
            z: Some(0.2),
            e: Some(1.5),
            f: Some(3600.0),
            role: ExtrusionRole::SparseInfill,
        },
        // Travel move (role = Custom("Travel") → serializer emits G0).
        GCodeCommand::Move {
            x: Some(30.0),
            y: Some(30.0),
            z: Some(0.2),
            e: None,
            f: Some(9000.0),
            role: ExtrusionRole::Custom("Travel".to_string()),
        },
        // Explicit retract (exercises Retract → "G1 E-..." path).
        GCodeCommand::Retract {
            length: 0.8,
            speed: 2400.0,
            mode: RetractMode::Gcode,
        },
    ];

    GCodeIR {
        commands,
        metadata: PrintMetadata {
            layer_count: 1,
            filament_used_mm: vec![3.0],
            ..PrintMetadata::default()
        },
        ..GCodeIR::default()
    }
}

#[test]
fn serialize_gcode_emits_documented_sentinels() {
    let gcode_ir = build_minimal_gcode_ir();
    let serializer = DefaultGCodeSerializer::default();

    let output = serializer
        .serialize_gcode(&gcode_ir)
        .expect("DefaultGCodeSerializer::serialize_gcode must succeed on a well-formed GCodeIR");

    // --- Header sentinels (AC-3..AC-6 / OrcaSlicer GCode.cpp:2644-2704) ---
    assert!(
        output.contains("; HEADER_BLOCK_START"),
        "expected `; HEADER_BLOCK_START` in serialized output, got:\n{output}"
    );
    assert!(
        output.contains("; HEADER_BLOCK_END"),
        "expected `; HEADER_BLOCK_END` in serialized output, got:\n{output}"
    );
    assert!(
        output.contains("; total layer number: 1"),
        "expected `; total layer number: 1` from PrintMetadata, got:\n{output}"
    );

    // --- Extrusion-width sentinel (AC-7, packet 55 Step 4) ---
    assert!(
        output.contains("; outer_wall_line_width = "),
        "expected `; outer_wall_line_width = ...` width-comment line, got:\n{output}"
    );

    // --- Layer/role sentinels (OrcaSlicer parity, Raw passthrough) ---
    assert!(
        output.contains(";LAYER_CHANGE"),
        "expected `;LAYER_CHANGE` marker in output, got:\n{output}"
    );
    assert!(
        output.contains(";Z:0.200"),
        "expected `;Z:0.200` layer-height comment in output, got:\n{output}"
    );
    assert!(
        output.contains(";TYPE:Outer wall"),
        "expected `;TYPE:Outer wall` role sentinel in output, got:\n{output}"
    );
    assert!(
        output.contains(";TYPE:Sparse infill"),
        "expected `;TYPE:Sparse infill` role sentinel in output, got:\n{output}"
    );

    // --- Move-line emission (G1 for extrusion, G0 for travel) ---
    // `format_xyz` strips trailing zeros, so 10.0 → "10", 0.2 → "0.2".
    assert!(
        output.contains("G1 X10 Y10 Z0.2 E0.50000 F1800"),
        "expected `G1` outer-wall move with stripped-zero XYZ + E + F, got:\n{output}"
    );
    assert!(
        output.contains("G0 X30 Y30 Z0.2 F9000"),
        "expected `G0` travel move (role=Custom(\"Travel\"), no E), got:\n{output}"
    );

    // --- Retract serialization (relative-E default → "G1 E-...") ---
    assert!(
        output.contains("G1 E-0.80000"),
        "expected `G1 E-0.80000` retract line (relative-E default), got:\n{output}"
    );
}
