//! Shared helper that seeds `Blackboard::slice_ir` from the execution plan.
//!
//! Tests that bypass the prepass executor and call `execute_per_layer` directly
//! must commit `slice_ir` before the call; the layer executor enforces this with
//! a hard `FatalLayer` error after commit `0fec45c` was reverted.
//!
//! Import with:
//!   ```ignore
//!   mod common;
//!   use common::seed::seed_slice_ir;
//!   ```

#![allow(dead_code)]

use std::sync::Arc;

use slicer_ir::SliceIR;
use slicer_runtime::{execute_prepass_slice_single_layer, Blackboard, ExecutionPlan};

/// Seed `blackboard.slice_ir` from the global-layer list in `plan`.
///
/// Calls `execute_prepass_slice_single_layer` for every global layer and
/// commits a `Vec<SliceIR>` to the blackboard. The Vec is sized to hold
/// `max(layer.index) + 1` entries so that `slice_vec[layer.index]` is valid
/// for all layers in the plan â€” matching the invariant assumed by
/// `execute_single_layer` (`slice_vec.get(layer.index as usize)`).
///
/// Panics on failure â€” callers are unit tests, so a panic is the right signal.
pub fn seed_slice_ir(blackboard: &mut Blackboard, plan: &ExecutionPlan) {
    let mesh = blackboard.mesh().clone();

    // Determine the required Vec capacity: max layer index + 1.
    let max_index = plan
        .global_layers
        .iter()
        .map(|gl| gl.index as usize)
        .max()
        .unwrap_or(0);
    let capacity = max_index + 1;

    // Build a default-filled Vec<SliceIR> of the right length, then overwrite
    // the entries that actually have global layers.
    let mut slices: Vec<SliceIR> = (0..capacity)
        .map(|i| SliceIR {
            global_layer_index: i as u32,
            ..Default::default()
        })
        .collect();

    for gl in plan.global_layers.iter() {
        slices[gl.index as usize] =
            execute_prepass_slice_single_layer(mesh.as_ref(), gl, None, None).unwrap();
    }

    blackboard
        .commit_slice_ir(Arc::new(slices))
        .expect("commit_slice_ir");
}
