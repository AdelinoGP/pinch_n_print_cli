#![allow(missing_docs)]

//! TDD tests for WI-4 / AC-N1 — emitter-side tool-index bound check.
//!
//! Packet 125 (voronoi-oom-hardening), Step 4 "Guard".
//!
//! Acceptance criterion AC-N1:
//!   Given a synthetic `LayerCollectionIR` whose first entity has a
//!   `region_id` equal to the garbage value 2,664,076,552 (the one that
//!   caused the real OOM), `emit_gcode` must return
//!   `Err(GCodeEmitError::ToolIndexOutOfRange { .. })` — no multi-GB
//!   allocation, no panic, no OOM.
//!
//! RED safety note:
//!   Without the guard, calling `emit_gcode` with this input would attempt
//!   `vec![0.0f32; 2_664_076_553]` (~9.9 GiB), which would OOM the test
//!   runner.  To keep the RED run safe the guard was added atomically with
//!   this test file; the logical RED state is expressed by the fact that
//!   `emit_tool_guard_tdd.rs` did not exist before this packet step, and
//!   `GCodeEmitError::ToolIndexOutOfRange` did not exist before this step
//!   either — a checkout of the pre-step tree would fail to compile the test,
//!   confirming RED without executing the OOM path.

use slicer_gcode::{DefaultGCodeEmitter, GCodeEmitError, GCodeEmitter};
use slicer_ir::{
    ExtrusionPath3D, ExtrusionRole, GCodeCommand, LayerCollectionIR, ObjectId, Point3WithWidth,
    PrintEntity, RegionKey,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn out_of_range_region_id() -> u64 {
    // The actual garbage id observed in the field (Step 2 bug report).
    2_664_076_552_u64
}

fn point3(x: f32, y: f32, z: f32) -> Point3WithWidth {
    Point3WithWidth {
        x,
        y,
        z,
        width: 0.4,
        flow_factor: 1.0,
        overhang_quartile: None,
        dist_to_top_mm: 0.0,
    }
}

fn entity_with_region_id(region_id: u64) -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![point3(0.0, 0.0, 0.2), point3(1.0, 0.0, 0.2)],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        tool_index: region_id as u32,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: ObjectId::from("test-obj"),
            region_id,
            variant_chain: vec![],
        },
        topo_order: 0,
    }
}

fn layer_with_entity(entity: PrintEntity) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity],
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// AC-N1  — out-of-range tool id must be rejected, not allocated
// ---------------------------------------------------------------------------

/// Verifies that `emit_gcode` returns `Err(ToolIndexOutOfRange)` when a
/// layer entity carries the garbage `region_id` 2,664,076,552.
///
/// Without the WI-4 guard this call would attempt to allocate a ~9.9 GiB
/// `Vec<f32>` and crash the process.  With the guard it returns the typed
/// error immediately.
#[test]
fn emit_rejects_out_of_range_tool_id() {
    let emitter = DefaultGCodeEmitter::new("test-1.0".to_string());

    let layer = layer_with_entity(entity_with_region_id(out_of_range_region_id()));
    let result = emitter.emit_gcode(&[layer]);

    match result {
        Err(GCodeEmitError::ToolIndexOutOfRange { tool, max }) => {
            // The garbage value truncates to u32 as the emitter does: region_id as u32.
            let expected_tool = out_of_range_region_id() as u32;
            assert_eq!(
                tool, expected_tool,
                "expected tool={expected_tool} in error, got tool={tool}"
            );
            assert!(
                max < expected_tool,
                "max ({max}) should be less than the garbage tool id ({expected_tool})"
            );
        }
        Err(other) => panic!("expected ToolIndexOutOfRange, got a different error: {other:?}"),
        Ok(_) => panic!("emit_gcode should have rejected the out-of-range tool id but returned Ok"),
    }
}

/// Sanity check: a normal single-tool print (region_id = 0) still succeeds.
#[test]
fn emit_accepts_normal_tool_id() {
    let emitter = DefaultGCodeEmitter::new("test-1.0".to_string());

    let layer = layer_with_entity(entity_with_region_id(0));
    let result = emitter.emit_gcode(&[layer]);

    assert!(
        result.is_ok(),
        "normal tool id 0 should not be rejected: {result:?}"
    );
}

// ---------------------------------------------------------------------------
// Leading-T-change contract (packet 126 Bug #1 follow-up)
// ---------------------------------------------------------------------------
//
// The emit path at `emit.rs:319-343` is supposed to emit a `ToolChange`
// command BEFORE the first extrusion of any layer whose
// `ordered_entities[0].tool_index` differs from `current_tool` (initially 0).
// This regression test pins the consumer-side contract so that any future
// reordering or shortcut that drops the leading T-change surfaces as a
// test failure with a precise pointer to the contract being violated.
//
// The sentinel `after_entity_index == u32::MAX` is the established
// "before entity 0" marker that the entity-loop consumer at `emit.rs:348`
// uses to schedule a pre-extrusion tool change. See packet 58 / DEV-054 (i).

fn entity_with_tool(tool: u32, role: ExtrusionRole) -> PrintEntity {
    PrintEntity {
        entity_id: 1,
        path: ExtrusionPath3D {
            points: vec![point3(0.0, 0.0, 0.2), point3(1.0, 0.0, 0.2)],
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        tool_index: tool,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: ObjectId::from("test-obj"),
            region_id: 0,
            variant_chain: vec![],
        },
        topo_order: 0,
    }
}

/// When `ordered_entities[0].tool_index != 0`, `emit_gcode` MUST emit a
/// `ToolChange { after_entity_index: u32::MAX, from: 0, to: <first tool> }`
/// command BEFORE the first `Move` of the layer. Without this, the first
/// cluster of walls would silently emit under the header default tool
/// (T0) even though their entity carries `tool_index = 2` — the symptom
/// observed in `target/p126.gcode` from packet 126 / Bug #1.
#[test]
fn emit_emits_leading_tool_change_when_first_entity_differs_from_current_tool() {
    let emitter = DefaultGCodeEmitter::new("test-1.0".to_string());

    // Layer 0's first entity is on tool 2 (e.g. blue back-face wall).
    let layer = layer_with_entity(entity_with_tool(2, ExtrusionRole::OuterWall));
    let gcode = emitter
        .emit_gcode(&[layer])
        .expect("emit_gcode must succeed for in-range tool index");

    // Find the first Move command (extrusion). The leading T-change (if
    // emitted correctly) must appear BEFORE it.
    let first_move_idx = gcode
        .commands
        .iter()
        .position(|c| matches!(c, GCodeCommand::Move { .. }))
        .expect("at least one Move command must be emitted");

    // Look for a ToolChange whose `after_entity_index == u32::MAX` (sentinel
    // for "before entity 0") and whose `from == 0, to == 2`.
    let leading = gcode.commands[..first_move_idx]
        .iter()
        .find_map(|c| match c {
            GCodeCommand::ToolChange {
                after_entity_index,
                from,
                to,
            } if *after_entity_index == u32::MAX && *from == 0 && *to == 2 => Some(*to),
            _ => None,
        });

    assert_eq!(
        leading,
        Some(2),
        "expected leading ToolChange {{ after_entity_index: u32::MAX, from: 0, to: 2 }} \
         BEFORE the first Move. \n\
         First-Move index = {first_move_idx}. \n\
         Commands before first move: {:#?}",
        &gcode.commands[..first_move_idx]
    );
}

/// When `ordered_entities[0].tool_index == 0`, no leading T-change must be
/// emitted — `current_tool` already matches. This locks the consumer's
/// branch (`required_tool != current_tool`) from being weakened to "always
/// emit".
#[test]
fn emit_omits_leading_tool_change_when_first_entity_already_matches_default() {
    let emitter = DefaultGCodeEmitter::new("test-1.0".to_string());

    let layer = layer_with_entity(entity_with_tool(0, ExtrusionRole::OuterWall));
    let gcode = emitter
        .emit_gcode(&[layer])
        .expect("emit_gcode must succeed for default tool index");

    let has_leading = gcode.commands.iter().any(|c| {
        matches!(
            c,
            GCodeCommand::ToolChange {
                after_entity_index: u32::MAX,
                ..
            }
        )
    });

    assert!(
        !has_leading,
        "leading T-change must be suppressed when first_entity.tool_index == 0; \
         commands = {:#?}",
        gcode.commands
    );
}
