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
    ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, ObjectId, Point3WithWidth, PrintEntity,
    RegionKey,
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
