//! Immutable execution-plan contracts for the host scheduler.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{ConfigView, GlobalLayer, ModuleId, RegionKey, RegionPlan, StageId};

use crate::instance_pool::WasmInstancePool;
use crate::manifest::LoadedModule;
use crate::wasm_instance::WasmComponent;

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
    /// Compiled WASM component for runtime instantiation.
    /// `None` only during test fixtures that don't exercise real WASM dispatch.
    pub wasm_component: Option<Arc<WasmComponent>>,
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
    /// Compiled WASM component for runtime instantiation.
    pub wasm_component: Option<Arc<WasmComponent>>,
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

/// Maximum allowed `GlobalLayer.index` value. Plans with layers at or above
/// this index are rejected per docs/02_ir_schemas.md and docs/12_architecture_gate_metrics.md.
pub const MAX_LAYER_INDEX: u32 = 100_000;

/// Default cap on `RegionMapIR` entry count per docs/04_host_scheduler.md.
pub const DEFAULT_REGION_MAP_CAP: usize = 1_000;

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
    /// A `GlobalLayer.index` exceeds the documented budget (>= 100_000).
    LayerIndexBudgetExceeded {
        /// The offending layer index.
        layer_index: u32,
        /// The configured budget cap.
        budget: u32,
    },
    /// The `RegionMapIR` entry count exceeds the configured cap.
    RegionMapCapExceeded {
        /// Computed entry count.
        entry_count: usize,
        /// Configured cap.
        cap: usize,
    },
}

impl std::fmt::Display for ExecutionPlanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingModuleBinding {
                stage_id,
                module_id,
            } => {
                write!(
                    f,
                    "stage '{stage_id}' references unknown module '{module_id}'"
                )
            }
            Self::StageMismatch {
                module_id,
                expected_stage,
                actual_stage,
            } => {
                write!(f, "module '{module_id}' declared stage '{actual_stage}' but was placed in '{expected_stage}'")
            }
            Self::DuplicateModuleBinding { module_id } => {
                write!(f, "duplicate runtime binding for module '{module_id}'")
            }
            Self::LayerIndexBudgetExceeded {
                layer_index,
                budget,
            } => {
                write!(
                    f,
                    "layer index {layer_index} exceeds budget (must be < {budget}); \
                     reduce layer count or increase layer height"
                )
            }
            Self::RegionMapCapExceeded { entry_count, cap } => {
                write!(
                    f,
                    "region map has {entry_count} entries, exceeding cap of {cap}; \
                     reduce region granularity, raise cap, or split job"
                )
            }
        }
    }
}

impl std::error::Error for ExecutionPlanError {}

/// Builds the immutable runtime execution plan.
///
/// Validates documented resource-bound contracts before assembling the plan:
/// - Every `GlobalLayer.index` must be `< 100_000` (docs/02_ir_schemas.md).
/// - `RegionMapIR` entry count must not exceed `DEFAULT_REGION_MAP_CAP` (docs/04_host_scheduler.md).
pub fn build_execution_plan(
    request: &ExecutionPlanRequest,
) -> Result<ExecutionPlan, ExecutionPlanError> {
    // ── Layer budget check ──────────────────────────────────────────
    for layer in request.global_layers.iter() {
        if layer.index >= MAX_LAYER_INDEX {
            return Err(ExecutionPlanError::LayerIndexBudgetExceeded {
                layer_index: layer.index,
                budget: MAX_LAYER_INDEX,
            });
        }
    }

    // ── Region map cap check ────────────────────────────────────────
    let region_count = request.region_plans.len();
    if region_count > DEFAULT_REGION_MAP_CAP {
        return Err(ExecutionPlanError::RegionMapCapExceeded {
            entry_count: region_count,
            cap: DEFAULT_REGION_MAP_CAP,
        });
    }

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
                wasm_component: binding.wasm_component.clone(),
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
