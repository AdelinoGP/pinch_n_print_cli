//! PostPass::LayerFinalization stage executor (TASK-032).
//!
//! This module defines the executor for the finalization stage that runs
//! after all per-layer parallel execution has completed. Modules in this stage
//! receive mutable access to the entire vector of LayerCollectionIR objects,
//! allowing them to insert synthetic layers or modify existing ones.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{FinalizationError, LayerCollectionIR, ModuleId};
use slicer_wasm_host::{
    CompiledModuleLive, FinalizationStageInput, FinalizationStageRunner, WasmComponent,
    WasmInstancePool,
};

use crate::instrumentation::{NoopInstrumentation, PipelineInstrumentation};
use crate::{Blackboard, ExecutionPlan};

// FinalizationStageRunner trait is now defined in slicer-wasm-host::traits and re-exported
// from slicer_runtime via the transitional re-exports block in lib.rs (P83 Step 4c+4d).
// Signature changes:
//   run_stage: module: &CompiledModule → &CompiledModuleLive<'_>,
//              blackboard: &Blackboard removed (mesh projected into FinalizationStageInput)

/// Builder for finalization output.
#[derive(Debug)]
pub struct FinalizationOutputBuilder;

/// Executes the PostPass::LayerFinalization pipeline.
///
/// Modules run sequentially with a forced pool size of 1.
/// Output layers are validated to maintain monotonic indices.
pub fn execute_layer_finalization(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &dyn FinalizationStageRunner,
    layers: &mut Vec<LayerCollectionIR>,
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    >,
) -> Result<(), FinalizationError> {
    execute_layer_finalization_with_instrumentation(
        plan,
        blackboard,
        runner,
        layers,
        &NoopInstrumentation,
        wasm_handles,
    )
}

/// Instrumented variant of [`execute_layer_finalization`] that brackets
/// each finalization module via `instrumentation`.
pub fn execute_layer_finalization_with_instrumentation(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &dyn FinalizationStageRunner,
    layers: &mut Vec<LayerCollectionIR>,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    >,
) -> Result<(), FinalizationError> {
    if let Some(stage) = &plan.layer_finalization_stage {
        if !stage.modules.is_empty() {
            // One whole-print working copy per stage run (`PostPass::LayerFinalization`
            // perf finding, 2026-10-09 refresh): every module runs on this copy and the
            // stage commits it back once, after the last module succeeds and validates.
            // The previous per-module `layers.clone()` + replaced-vector drop paid eight
            // whole-print copy/drop operations per slice and dominated base supports-on
            // (~100–121 s of host copy time per run; guest fuel ≤0.01% of the cell).
            // Module failure and validation still discard the working copy without
            // touching `layers`; the one semantic delta is that `layers` holds the
            // pre-stage snapshot after a mid-stage abort instead of the last committed
            // module's output — unobservable in production, where `fin_result?` aborts
            // the slice and drops `layers`.
            let mut candidate = layers.clone();
            for module in &stage.modules {
                instrumentation.on_module_start(&stage.stage_id, None, module.module_id());
                // Build IR-typed borrow structs for the new slicer-wasm-host trait boundary.
                let (instance_pool, wasm_component, native_entry) = wasm_handles
                    .get(module.module_id().as_str())
                    .map(|(p, c, e)| (Arc::clone(p), c.clone(), *e))
                    .unwrap_or_else(|| (WasmInstancePool::placeholder(), None, None));
                let mut live_module = CompiledModuleLive::new(
                    module.module_id(),
                    instance_pool,
                    wasm_component,
                    module.claims(),
                    Arc::clone(module.config_view()),
                )
                .with_ir_writes(&module.ir_write_mask().paths);
                if let Some(entry) = native_entry {
                    live_module = live_module.with_native_entry(entry);
                }
                let input = FinalizationStageInput {
                    mesh: std::sync::Arc::clone(blackboard.mesh()),
                    _phantom: std::marker::PhantomData,
                };
                let res = runner.run_stage(&stage.stage_id, &live_module, input, &mut candidate);
                // Drain module log messages (already forwarded to the human `log`
                // facade inside the dispatcher; this clears the thread-local stash)
                // and forward each onto the structured stream as a `module_log`
                // event.
                for (level, message) in runner.last_log_messages() {
                    instrumentation.on_module_log(
                        &stage.stage_id,
                        None,
                        module.module_id(),
                        &level,
                        &message,
                    );
                }
                // Must precede `on_module_end`, which is what emits the
                // `module_complete` the verbose scope detail attaches to. Both
                // accessors return empty/zero when profiling is off.
                instrumentation.on_module_profile(
                    &stage.stage_id,
                    None,
                    module.module_id(),
                    &runner.last_profile_marks(),
                    runner.last_call_fuel(),
                );
                instrumentation.on_module_end(&stage.stage_id, None, module.module_id(), 0, 0);
                if let Err(error) = res {
                    instrumentation.on_module_error(
                        &stage.stage_id,
                        None,
                        module.module_id(),
                        &error.to_string(),
                        true,
                    );
                    return Err(error);
                }

                // Synthesized anchored rows reuse their upper model layer's index,
                // so equal adjacent indices are valid while reversals are not.
                for window in candidate.windows(2) {
                    if window[0].global_layer_index > window[1].global_layer_index {
                        let error = FinalizationError::Validation {
                            message: format!(
                                "layer indices must be monotonic, found {} followed by {}",
                                window[0].global_layer_index, window[1].global_layer_index
                            ),
                        };
                        instrumentation.on_module_error(
                            &stage.stage_id,
                            None,
                            module.module_id(),
                            &error.to_string(),
                            true,
                        );
                        return Err(error);
                    }
                }
            }
            // The stage commits exactly once: every module ran on the shared working
            // copy, so the final assignment both publishes the result and drops the
            // pre-stage vector that this clone replaced.
            *layers = candidate;
        }
    }

    Ok(())
}
