//! Immutable execution-plan contracts for the host scheduler.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigView, GlobalLayer, ModuleId, RegionKey, RegionPlan, StageId};

use crate::instance_pool::WasmInstancePool;
use crate::manifest::LoadedModule;

/// Frozen runtime scheduling state shared read-only across worker threads.
#[derive(Debug, Clone)]
pub struct ExecutionPlan {
    /// Topologically sorted prepass stages excluding host-built-ins.
    pub prepass_stages: Vec<CompiledStage>,
    /// Topologically sorted per-layer stages excluding host-built-ins.
    pub per_layer_stages: Vec<CompiledStage>,
    /// Dedicated sequential finalization bucket.
    pub layer_finalization_stage: Option<CompiledStage>,
    /// Topologically sorted postpass stages excluding host-built-ins and finalization.
    pub postpass_stages: Vec<CompiledStage>,
    /// Frozen global layer schedule.
    pub global_layers: Arc<Vec<GlobalLayer>>,
    /// Frozen per-region execution plans.
    pub region_plans: Arc<HashMap<RegionKey, RegionPlan>>,
}

/// One compiled scheduler stage ready for direct runtime iteration.
#[derive(Debug, Clone)]
pub struct CompiledStage {
    /// Canonical scheduler stage identifier.
    pub stage_id: StageId,
    /// Topologically sorted module invocations for this stage.
    pub modules: Vec<CompiledModule>,
}

/// One loaded module bound to immutable runtime execution metadata.
#[derive(Debug, Clone)]
pub struct CompiledModule {
    /// Reverse-domain module identifier.
    pub module_id: ModuleId,
    /// Bound instance pool selected during startup planning.
    pub instance_pool: Arc<WasmInstancePool>,
    /// Frozen IR read access mask derived from the manifest.
    pub ir_read_mask: IrAccessMask,
    /// Frozen IR write access mask derived from the manifest.
    pub ir_write_mask: IrAccessMask,
    /// Frozen module-specific config view.
    pub config_view: Arc<ConfigView>,
}

/// Minimal immutable IR access-mask representation for runtime planning.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IrAccessMask {
    /// Declared manifest access paths.
    pub paths: Vec<String>,
}

/// One already-sorted stage bucket supplied by validation/topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortedStageModules {
    /// Canonical scheduler stage identifier.
    pub stage_id: StageId,
    /// Topologically sorted module identifiers for the stage.
    pub module_ids: Vec<ModuleId>,
}

/// One loaded module plus its runtime bindings.
#[derive(Debug, Clone)]
pub struct ExecutionModuleBinding {
    /// Loaded manifest/module metadata.
    pub module: LoadedModule,
    /// Planned WASM instance pool for the module.
    pub instance_pool: Arc<WasmInstancePool>,
    /// Frozen config view bound for runtime execution.
    pub config_view: Arc<ConfigView>,
}

/// Immutable planning input assembled after validation and module loading.
#[derive(Debug, Clone)]
pub struct ExecutionPlanRequest {
    /// Already topologically sorted scheduler stages.
    pub sorted_stages: Vec<SortedStageModules>,
    /// Loaded modules and their runtime bindings.
    pub module_bindings: Vec<ExecutionModuleBinding>,
    /// Frozen global layer schedule.
    pub global_layers: Arc<Vec<GlobalLayer>>,
    /// Frozen per-region execution plans.
    pub region_plans: Arc<HashMap<RegionKey, RegionPlan>>,
}

/// Structured planning failure for immutable execution-plan assembly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionPlanError {
    /// A sorted stage referenced a module with no runtime binding.
    MissingModuleBinding {
        /// Stage that referenced the missing binding.
        stage_id: StageId,
        /// Missing module identifier.
        module_id: ModuleId,
    },
    /// A runtime binding declared a stage inconsistent with the sorted stage input.
    StageMismatch {
        /// Bound module identifier.
        module_id: ModuleId,
        /// Stage implied by the sorted input.
        expected_stage: StageId,
        /// Stage declared by the loaded module.
        actual_stage: StageId,
    },
    /// Multiple runtime bindings targeted the same module id.
    DuplicateModuleBinding {
        /// Duplicate module identifier.
        module_id: ModuleId,
    },
}

/// Builds the immutable runtime execution plan for TASK-025.
pub fn build_execution_plan(
    request: &ExecutionPlanRequest,
) -> Result<ExecutionPlan, ExecutionPlanError> {
    let mut bindings_by_module_id = HashMap::with_capacity(request.module_bindings.len());
    for binding in &request.module_bindings {
        let module_id = binding.module.id.clone();
        if bindings_by_module_id
            .insert(module_id.clone(), binding)
            .is_some()
        {
            return Err(ExecutionPlanError::DuplicateModuleBinding { module_id });
        }
    }

    let mut prepass_stages = Vec::new();
    let mut per_layer_stages = Vec::new();
    let mut layer_finalization_stage = None;
    let mut postpass_stages = Vec::new();

    for sorted_stage in &request.sorted_stages {
        let mut modules = Vec::with_capacity(sorted_stage.module_ids.len());

        for module_id in &sorted_stage.module_ids {
            let binding = bindings_by_module_id.get(module_id).ok_or_else(|| {
                ExecutionPlanError::MissingModuleBinding {
                    stage_id: sorted_stage.stage_id.clone(),
                    module_id: module_id.clone(),
                }
            })?;

            if binding.module.stage != sorted_stage.stage_id {
                return Err(ExecutionPlanError::StageMismatch {
                    module_id: binding.module.id.clone(),
                    expected_stage: sorted_stage.stage_id.clone(),
                    actual_stage: binding.module.stage.clone(),
                });
            }

            modules.push(CompiledModule {
                module_id: binding.module.id.clone(),
                instance_pool: Arc::clone(&binding.instance_pool),
                ir_read_mask: IrAccessMask {
                    paths: binding.module.ir_reads.clone(),
                },
                ir_write_mask: IrAccessMask {
                    paths: binding.module.ir_writes.clone(),
                },
                config_view: Arc::clone(&binding.config_view),
            });
        }

        if modules.is_empty() {
            continue;
        }

        let compiled_stage = CompiledStage {
            stage_id: sorted_stage.stage_id.clone(),
            modules,
        };

        if sorted_stage.stage_id.starts_with("PrePass::") {
            prepass_stages.push(compiled_stage);
        } else if sorted_stage.stage_id.starts_with("Layer::") {
            per_layer_stages.push(compiled_stage);
        } else if sorted_stage.stage_id == "PostPass::LayerFinalization" {
            layer_finalization_stage = Some(compiled_stage);
        } else if sorted_stage.stage_id.starts_with("PostPass::") {
            postpass_stages.push(compiled_stage);
        }
    }

    Ok(ExecutionPlan {
        prepass_stages,
        per_layer_stages,
        layer_finalization_stage,
        postpass_stages,
        global_layers: Arc::clone(&request.global_layers),
        region_plans: Arc::clone(&request.region_plans),
    })
}
