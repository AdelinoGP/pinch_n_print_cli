//! PostPass executor (TASK-033).
//!
//! This module defines the executor for post-layer-finalization stages
//! (GCodeEmit, GCodePostProcess, TextPostProcess). All stages run sequentially
//! with immutable access to layer_irs.
//!
//! Pipeline order:
//! 1. emit_gcode(layer_irs, blackboard) -> GCodeIR (host-built-in, TASK-034)
//! 2. Run all PostPassGCodePostProcess modules sequentially
//! 3. Run all PostPassTextPostProcess modules sequentially OR serialize GCodeIR
//!
//! Reference: docs/04_host_scheduler.md lines 778-810

use std::collections::HashMap;
use std::sync::Arc;

use slicer_gcode::{GCodeEmitError, GCodeEmitter, GCodeSerializer};
use slicer_ir::{LayerCollectionIR, ModuleId, PostpassError, PostpassOutput};

/// Translate a `slicer_gcode::GCodeEmitError` into the matching
/// `slicer_ir::PostpassError` variant.
///
/// Defined here (rather than as a `From` impl) because of Rust's orphan rule:
/// `PostpassError` lives in `slicer-ir` and `GCodeEmitError` lives in
/// `slicer-gcode`; neither type is owned by `slicer-runtime`. The conversion
/// is the runtime-side seam between the two crates, so it lives next to the
/// only call sites (`emit_gcode` / `serialize_gcode` inside
/// `execute_postpass_with_instrumentation`).
fn gcode_emit_error_to_postpass(e: GCodeEmitError) -> PostpassError {
    match e {
        GCodeEmitError::MissingToolchangePurge {
            layer_index,
            tool_change_index,
        } => PostpassError::MissingToolchangePurge {
            layer_index,
            tool_change_index,
        },
        GCodeEmitError::Emit(message) => PostpassError::GCodeEmit { message },
        GCodeEmitError::Serialization(message) => PostpassError::GCodeSerialization { message },
        GCodeEmitError::ToolIndexOutOfRange { tool, max } => PostpassError::GCodeEmit {
            message: format!("tool index {tool} out of range (max plausible {max})"),
        },
    }
}
use slicer_wasm_host::{
    CompiledModuleLive, PostpassStageInput, PostpassStageRunner, WasmComponent, WasmInstancePool,
};

use crate::instrumentation::{NoopInstrumentation, PipelineInstrumentation};
use crate::{Blackboard, ExecutionPlan, ModuleAccessAudit};

// PostpassStageRunner trait is now defined in slicer-wasm-host::traits and re-exported
// from slicer_runtime via the transitional re-exports block in lib.rs (P83 Step 4c+4d).
// Signature changes:
//   run_gcode_postprocess: module: &CompiledModule → &CompiledModuleLive<'_>,
//                          blackboard: &Blackboard removed,
//                          gcode_ir: &mut GCodeIR → commands: &mut Vec<GCodeCommand>
//   run_text_postprocess:  module: &CompiledModule → &CompiledModuleLive<'_>,
//                          blackboard: &Blackboard removed (mesh projected into PostpassStageInput)
//
// `GCodeEmitter` and `GCodeSerializer` traits live in `slicer-gcode` as of
// packet 86 Step 3; they are re-imported above so the postpass executor
// continues to accept `&dyn GCodeEmitter` / `&dyn GCodeSerializer` callers.

/// Executes the PostPass pipeline.
///
/// Pipeline stages:
/// 1. Call `emitter.emit_gcode()` to produce initial GCodeIR from layers
/// 2. For each module in PostPass::GCodePostProcess, call `runner.run_gcode_postprocess()`
/// 3. Either:
///    - If PostPass::TextPostProcess modules exist, serialize GCodeIR and run them
///    - Otherwise, serialize GCodeIR directly to produce final text
///
/// # Arguments
///
/// * `plan` - The execution plan containing postpass stages
/// * `layer_irs` - Immutable reference to finalized layer collections
/// * `blackboard` - The blackboard for read-only access to prepass IRs
/// * `emitter` - GCode emission implementation (host-built-in)
/// * `serializer` - GCode serialization implementation (host-built-in)
/// * `runner` - The stage runner for module execution
///
/// # Returns
///
/// The final G-code string and collected runtime access audits, or an error if
/// any stage fails fatally.
pub fn execute_postpass(
    plan: &ExecutionPlan,
    layer_irs: &[LayerCollectionIR],
    blackboard: &Blackboard,
    emitter: &dyn GCodeEmitter,
    serializer: &dyn GCodeSerializer,
    runner: &mut dyn PostpassStageRunner,
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<(String, Vec<ModuleAccessAudit>), PostpassError> {
    execute_postpass_with_instrumentation(
        plan,
        layer_irs,
        blackboard,
        emitter,
        serializer,
        runner,
        &NoopInstrumentation,
        wasm_handles,
    )
}

/// Instrumented variant of [`execute_postpass`] that brackets each
/// postpass stage and module (including host built-ins) via
/// `instrumentation`.
pub fn execute_postpass_with_instrumentation(
    plan: &ExecutionPlan,
    layer_irs: &[LayerCollectionIR],
    blackboard: &Blackboard,
    emitter: &dyn GCodeEmitter,
    serializer: &dyn GCodeSerializer,
    runner: &mut dyn PostpassStageRunner,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<(String, Vec<ModuleAccessAudit>), PostpassError> {
    // Step 1a: Reconcile finalization-aware travel moves before emission.
    // This adjusts travel_moves to route through Skirt/Brim and WipeTower
    // geometry without modifying ordered_entities.
    let mut reconciled_layers: Vec<LayerCollectionIR> = layer_irs.to_vec();
    let travel_f = emitter.travel_feedrate_mm_per_min();
    for layer in &mut reconciled_layers {
        slicer_gcode::reconcile_finalization_travel(layer, travel_f);
    }

    // Step 1b: Emit initial GCodeIR from (reconciled) layers
    let emit_stage = "PostPass::GCodeEmit".to_string();
    let emit_module = "host:gcode_emit".to_string();
    instrumentation.on_stage_start(&emit_stage, None);
    instrumentation.on_module_start(&emit_stage, None, &emit_module);
    let mut gcode_ir = emitter
        .emit_gcode(&reconciled_layers)
        .map_err(gcode_emit_error_to_postpass)
        .inspect_err(|_| {
            instrumentation.on_stage_end(&emit_stage, None);
        })?;
    instrumentation.on_module_end(&emit_stage, None, &emit_module, 0, 0);
    instrumentation.on_stage_end(&emit_stage, None);
    let mut audits = Vec::new();

    // Step 2: Run all GCodePostProcess modules sequentially
    for stage in &plan.postpass_stages {
        if stage.stage_id.contains("GCodePostProcess") {
            instrumentation.on_stage_start(&stage.stage_id, None);
            for module in &stage.modules {
                instrumentation.on_module_start(&stage.stage_id, None, module.module_id());
                // Build IR-typed borrow structs for the new slicer-wasm-host trait boundary.
                let (instance_pool, wasm_component) = wasm_handles
                    .get(module.module_id().as_str())
                    .map(|(p, c)| (Arc::clone(p), c.clone()))
                    .unwrap_or_else(|| (WasmInstancePool::placeholder(), None));
                let live_module = CompiledModuleLive::new(
                    module.module_id(),
                    instance_pool,
                    wasm_component,
                    module.claims(),
                    Arc::clone(module.config_view()),
                );
                let input = PostpassStageInput {
                    mesh: std::sync::Arc::clone(blackboard.mesh()),
                    _phantom: std::marker::PhantomData,
                };
                let res = runner.run_gcode_postprocess(
                    &stage.stage_id,
                    &live_module,
                    input,
                    &mut gcode_ir.commands,
                );
                instrumentation.on_module_end(&stage.stage_id, None, module.module_id(), 0, 0);
                let result = match res {
                    Ok(r) => r,
                    Err(e) => {
                        instrumentation.on_stage_end(&stage.stage_id, None);
                        return Err(e);
                    }
                };
                match result {
                    PostpassOutput::GCodeSuccess => {
                        // Record runtime audit for GCodePostProcess modules.
                        // runtime_reads are drained via take_runtime_reads().
                        let runtime_reads = runner.take_runtime_reads();
                        let reads = runtime_reads.into_iter().flatten().collect();
                        audits.push(ModuleAccessAudit {
                            module_id: module.module_id().to_owned(),
                            runtime_reads: reads,
                            runtime_writes: vec![String::from("GCodeIR")],
                        });
                    }
                    PostpassOutput::NonFatalError { message: _ } => {
                        // Log warning but continue to next module
                    }
                    PostpassOutput::TextSuccess { text: _ } => {
                        // Unexpected from GCodePostProcess, but not fatal - continue
                    }
                }
            }
            instrumentation.on_stage_end(&stage.stage_id, None);
        }
    }

    // Step 3: Check if any TextPostProcess modules exist
    let text_postprocess_stages: Vec<_> = plan
        .postpass_stages
        .iter()
        .filter(|s| s.stage_id.contains("TextPostProcess"))
        .collect();

    if text_postprocess_stages.is_empty() {
        // No TextPostProcess modules - serialize directly
        let ser_stage = "PostPass::GCodeSerialize".to_string();
        let ser_module = "host:gcode_serialize".to_string();
        instrumentation.on_stage_start(&ser_stage, None);
        instrumentation.on_module_start(&ser_stage, None, &ser_module);
        let text = serializer
            .serialize_gcode(&gcode_ir)
            .map_err(gcode_emit_error_to_postpass)
            .inspect_err(|_| {
                instrumentation.on_stage_end(&ser_stage, None);
            })?;
        instrumentation.on_module_end(&ser_stage, None, &ser_module, 0, 0);
        instrumentation.on_stage_end(&ser_stage, None);
        return Ok((text, audits));
    }

    // Step 4: Serialize GCodeIR to text for TextPostProcess modules
    let ser_stage = "PostPass::GCodeSerialize".to_string();
    let ser_module = "host:gcode_serialize".to_string();
    instrumentation.on_stage_start(&ser_stage, None);
    instrumentation.on_module_start(&ser_stage, None, &ser_module);
    let mut text = serializer
        .serialize_gcode(&gcode_ir)
        .map_err(gcode_emit_error_to_postpass)
        .inspect_err(|_| {
            instrumentation.on_stage_end(&ser_stage, None);
        })?;
    instrumentation.on_module_end(&ser_stage, None, &ser_module, 0, 0);
    instrumentation.on_stage_end(&ser_stage, None);

    // Step 5: Run all TextPostProcess modules sequentially
    for stage in text_postprocess_stages {
        instrumentation.on_stage_start(&stage.stage_id, None);
        for module in &stage.modules {
            instrumentation.on_module_start(&stage.stage_id, None, module.module_id());
            let (instance_pool, wasm_component) = wasm_handles
                .get(module.module_id().as_str())
                .map(|(p, c)| (Arc::clone(p), c.clone()))
                .unwrap_or_else(|| (WasmInstancePool::placeholder(), None));
            let live_module = CompiledModuleLive::new(
                module.module_id(),
                instance_pool,
                wasm_component,
                module.claims(),
                Arc::clone(module.config_view()),
            );
            let input = PostpassStageInput {
                mesh: std::sync::Arc::clone(blackboard.mesh()),
                _phantom: std::marker::PhantomData,
            };
            let res = runner.run_text_postprocess(&stage.stage_id, &live_module, input, text);
            instrumentation.on_module_end(&stage.stage_id, None, module.module_id(), 0, 0);
            let result = match res {
                Ok(r) => r,
                Err(e) => {
                    instrumentation.on_stage_end(&stage.stage_id, None);
                    return Err(e);
                }
            };
            match result {
                PostpassOutput::TextSuccess { text: new_text } => {
                    // Record runtime audit for TextPostProcess modules.
                    // Extract runtime reads collected during this dispatch call.
                    // TextPostProcess modules produce final text output.
                    let runtime_reads = runner.take_runtime_reads();
                    let reads = runtime_reads.into_iter().flatten().collect();
                    audits.push(ModuleAccessAudit {
                        module_id: module.module_id().to_owned(),
                        runtime_reads: reads,
                        runtime_writes: vec![String::from("GCodeIR")],
                    });
                    text = new_text;
                }
                PostpassOutput::NonFatalError { message: _ } => {
                    // Log warning but continue - text remains unchanged from serialization
                    // Since we consumed `text` we need to re-serialize for the next module
                    let ser_stage = "PostPass::GCodeSerialize".to_string();
                    let ser_module = "host:gcode_serialize".to_string();
                    instrumentation.on_stage_start(&ser_stage, None);
                    instrumentation.on_module_start(&ser_stage, None, &ser_module);
                    text = serializer
                        .serialize_gcode(&gcode_ir)
                        .map_err(gcode_emit_error_to_postpass)
                        .inspect_err(|_| {
                            instrumentation.on_stage_end(&ser_stage, None);
                        })?;
                    instrumentation.on_module_end(&ser_stage, None, &ser_module, 0, 0);
                    instrumentation.on_stage_end(&ser_stage, None);
                }
                PostpassOutput::GCodeSuccess => {
                    // Unexpected from TextPostProcess, but not fatal
                    // Re-serialize since we consumed text
                    let ser_stage = "PostPass::GCodeSerialize".to_string();
                    let ser_module = "host:gcode_serialize".to_string();
                    instrumentation.on_stage_start(&ser_stage, None);
                    instrumentation.on_module_start(&ser_stage, None, &ser_module);
                    text = serializer
                        .serialize_gcode(&gcode_ir)
                        .map_err(gcode_emit_error_to_postpass)
                        .inspect_err(|_| {
                            instrumentation.on_stage_end(&ser_stage, None);
                        })?;
                    instrumentation.on_module_end(&ser_stage, None, &ser_module, 0, 0);
                    instrumentation.on_stage_end(&ser_stage, None);
                }
            }
        }
        instrumentation.on_stage_end(&stage.stage_id, None);
    }

    Ok((text, audits))
}
