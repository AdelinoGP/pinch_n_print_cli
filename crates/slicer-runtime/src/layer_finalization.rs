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
///
/// Borrowing compatibility wrapper over
/// [`execute_layer_finalization_owned`]: the vector is moved into the owning
/// entry point and moved back on success, so the two calls together are a
/// pointer swap, not a copy. **One semantic delta:** after a mid-stage module
/// or validation failure the working collection is consumed by the abort and
/// `layers` is left empty instead of holding the pre-stage snapshot. Every
/// production caller aborts the slice on that error (`fin_result?`) and never
/// reads the vector again, so the state was already unobservable there; the
/// owning entry point is the one the pipeline uses.
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
    let owned = std::mem::take(layers);
    *layers = execute_layer_finalization_owned(
        plan,
        blackboard,
        runner,
        owned,
        instrumentation,
        wasm_handles,
    )?;
    Ok(())
}

/// Owning variant of [`execute_layer_finalization_with_instrumentation`]: the
/// caller hands over the per-layer collection and gets the committed collection
/// back.
///
/// Ownership is the point (`PostPass::LayerFinalization` perf finding, t90): the
/// stage used to deep-clone the whole collection for a rollback snapshot that
/// production never reads and then drop the replaced vector at commit, paying
/// two whole-print operations per slice on supports-on jobs (host copy/drop, not
/// guest work — see the t88 attribution). Modules now run on the caller's own
/// vector, and the single remaining drop is the vector itself.
///
/// The previous per-module `layers.clone()` shape paid eight whole-print
/// copy/drop operations per slice (2026-10-09 refresh); this keeps that fix and
/// removes the last whole-print clone. Module failure and validation abort the
/// stage: the working collection is dropped with the error and is no longer
/// available to the caller — unobservable in production, which aborts the slice
/// on that error.
pub fn execute_layer_finalization_owned(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &dyn FinalizationStageRunner,
    mut layers: Vec<LayerCollectionIR>,
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<slicer_sdk::native::NativeStageEntry>,
        ),
    >,
) -> Result<Vec<LayerCollectionIR>, FinalizationError> {
    let Some(stage) = &plan.layer_finalization_stage else {
        return Ok(layers);
    };
    if stage.modules.is_empty() {
        return Ok(layers);
    }

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
        let res = runner.run_stage(&stage.stage_id, &live_module, input, &mut layers);
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
        for window in layers.windows(2) {
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

    Ok(layers)
}
