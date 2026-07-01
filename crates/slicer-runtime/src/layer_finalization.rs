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
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
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
    wasm_handles: &HashMap<ModuleId, (Arc<WasmInstancePool>, Option<Arc<WasmComponent>>)>,
) -> Result<(), FinalizationError> {
    if let Some(stage) = &plan.layer_finalization_stage {
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
            let input = FinalizationStageInput {
                mesh: std::sync::Arc::clone(blackboard.mesh()),
                _phantom: std::marker::PhantomData,
            };
            let res = runner.run_stage(&stage.stage_id, &live_module, input, layers);
            // Drain module log messages (already forwarded to the log facade
            // inside the dispatcher; this clears the thread-local stash).
            let _log_messages = runner.last_log_messages();
            instrumentation.on_module_end(&stage.stage_id, None, module.module_id(), 0, 0);
            res?;

            // Validate that the layer indices remain strictly monotonic
            for window in layers.windows(2) {
                if window[0].global_layer_index >= window[1].global_layer_index {
                    return Err(FinalizationError::Validation {
                        message: format!(
                            "layer indices must be strictly monotonic, found {} followed by {}",
                            window[0].global_layer_index, window[1].global_layer_index
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}
