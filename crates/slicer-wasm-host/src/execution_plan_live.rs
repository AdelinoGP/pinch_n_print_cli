//! Live-path (wasmtime-backed) execution plan building and module loading.
//!
//! These functions live here — in slicer-wasm-host — because they directly
//! construct `WasmEngine`, `WasmComponent`, and `WasmInstancePool` handles.
//! The pure-scheduling types (`CompiledModuleStatic`, `ExecutionPlan`, etc.)
//! live in `slicer-scheduler`, which has no wasmtime dependency.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{ConfigKey, ConfigValue, GlobalLayer, ModuleId, RegionKey, RegionPlan, StageId};

use slicer_scheduler::dag::{build_intra_stage_dag, Producer};
use slicer_scheduler::execution_plan::{
    bind_module_config_view, build_execution_plan, dedup_same_claim_modules_for_test,
    ExecutionModuleBinding, ExecutionPlan, ExecutionPlanError, ExecutionPlanRequest,
    SortedStageModules, STAGE_ORDER,
};
use slicer_scheduler::manifest::{
    load_modules_from_roots, DiagnosticLevel, LoadDiagnostic, LoadError, LoadedModule,
};
use slicer_scheduler::topology::topological_sort;
use slicer_scheduler::validation::SchedulerError;

use crate::instance::{WasmComponent, WasmEngine};
use crate::pool::{
    build_wasm_instance_pool, InstancePoolError, WasmArtifactMetadata, WasmInstancePool,
};

/// Runtime bindings for one loaded module, minus its `ConfigView`.
///
/// Used by [`build_live_execution_plan`] to build per-module bindings
/// whose `Arc<ConfigView>` is ALWAYS synthesised through
/// [`slicer_scheduler::execution_plan::bind_module_config_view`] — modules can't supply
/// a hand-rolled `ConfigView` on this path, so the declared-read invariant is upheld
/// by construction.
#[derive(Debug, Clone)]
pub struct LiveModuleBinding {
    /// Loaded manifest/module metadata.
    pub module: LoadedModule,
    /// Planned WASM instance pool for the module.
    pub instance_pool: Arc<WasmInstancePool>,
    /// Compiled WASM component for runtime instantiation (optional for
    /// fixtures that don't exercise dispatch).
    pub wasm_component: Option<Arc<WasmComponent>>,
}

/// Build the immutable `ExecutionPlan` used by the live host/runtime path.
///
/// For every `LiveModuleBinding`, the per-module `Arc<ConfigView>` is
/// synthesised via [`bind_module_config_view`] against `config_source`.
pub fn build_live_execution_plan(
    sorted_stages: Vec<SortedStageModules>,
    modules: Vec<LiveModuleBinding>,
    config_source: &HashMap<ConfigKey, ConfigValue>,
    global_layers: Arc<Vec<GlobalLayer>>,
    region_plans: Arc<HashMap<RegionKey, RegionPlan>>,
) -> Result<ExecutionPlan, ExecutionPlanError> {
    let module_bindings: Vec<ExecutionModuleBinding> = modules
        .into_iter()
        .map(|b| {
            let config_view = bind_module_config_view(&b.module, config_source);
            ExecutionModuleBinding {
                module: b.module,
                config_view,
            }
        })
        .collect();

    build_execution_plan(&ExecutionPlanRequest {
        sorted_stages,
        module_bindings,
        global_layers,
        region_plans,
    })
}

/// Aggregated output of [`load_live_modules_for_plan`] ready to feed into
/// [`build_live_execution_plan`].
#[derive(Debug)]
pub struct LiveModuleLoadOutput {
    /// Per-module runtime bindings (one per discovered module, in the
    /// deterministic order produced by manifest discovery).
    pub bindings: Vec<LiveModuleBinding>,
    /// Canonical per-stage module order (topologically sorted within
    /// each stage, stages emitted in `STAGE_ORDER`).
    pub sorted_stages: Vec<SortedStageModules>,
    /// Non-fatal discovery diagnostics surfaced by `load_modules_from_roots`.
    pub diagnostics: Vec<LoadDiagnostic>,
    /// The shared [`WasmEngine`] used to compile all module components.
    ///
    /// Callers that need to instantiate compiled components at runtime
    /// (e.g. [`WasmRuntimeDispatcher`]) must use this same engine; creating
    /// a second engine would produce a different `wasmtime::Engine` instance
    /// and `wasmtime::Store::new` would reject components compiled by a
    /// different engine.
    pub engine: Arc<WasmEngine>,
}

/// Structured failure for live module loading on the production path.
#[derive(Debug)]
pub enum LiveModuleLoadError {
    /// Manifest discovery/ingestion failed fatally.
    Load(LoadError),
    /// A stage's intra-stage DAG could not be built.
    Dag(SchedulerError),
    /// A stage's module set could not be topologically sorted (cycle).
    Cycle {
        /// Stage that carried the unresolved cycle.
        stage_id: StageId,
        /// Remaining module IDs that could not be ordered.
        unsorted: Vec<ModuleId>,
    },
    /// WASM instance pool planning rejected a module.
    InstancePool(InstancePoolError),
}

impl std::fmt::Display for LiveModuleLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Load(e) => write!(f, "module discovery failed: {e:?}"),
            Self::Dag(e) => write!(f, "intra-stage DAG construction failed: {e:?}"),
            Self::Cycle { stage_id, unsorted } => write!(
                f,
                "stage '{stage_id}' contains a dependency cycle; unsorted modules: {unsorted:?}"
            ),
            Self::InstancePool(e) => write!(f, "instance pool planning failed: {e:?}"),
        }
    }
}

impl std::error::Error for LiveModuleLoadError {}

impl From<LoadError> for LiveModuleLoadError {
    fn from(e: LoadError) -> Self {
        Self::Load(e)
    }
}
impl From<SchedulerError> for LiveModuleLoadError {
    fn from(e: SchedulerError) -> Self {
        Self::Dag(e)
    }
}
impl From<InstancePoolError> for LiveModuleLoadError {
    fn from(e: InstancePoolError) -> Self {
        Self::InstancePool(e)
    }
}
impl From<SchedulerError> for Box<LiveModuleLoadError> {
    fn from(e: SchedulerError) -> Self {
        Box::new(LiveModuleLoadError::Dag(e))
    }
}
impl From<LoadError> for Box<LiveModuleLoadError> {
    fn from(e: LoadError) -> Self {
        Box::new(LiveModuleLoadError::Load(e))
    }
}

/// Discover all modules under `search_roots`, plan their WASM instance
/// pools, and produce canonical `STAGE_ORDER`-sorted bindings ready to
/// feed [`build_live_execution_plan`].
///
/// `host_parallelism` controls the pool size for `layer-parallel-safe`
/// modules; other modules use a serialised pool of size 1 per
/// `build_wasm_instance_pool`.
pub fn load_live_modules_for_plan(
    search_roots: &[PathBuf],
    host_parallelism: usize,
) -> Result<LiveModuleLoadOutput, Box<LiveModuleLoadError>> {
    let mut report = load_modules_from_roots(search_roots)?;

    // Claim-uniqueness enforcement via the public test shim (same dedup logic).
    let filtered_modules =
        dedup_same_claim_modules_for_test(&mut report.modules, &mut report.diagnostics);
    report.modules = filtered_modules;

    // Build per-stage topological orderings in canonical STAGE_ORDER.
    let module_producers: Vec<&dyn Producer> =
        report.modules.iter().map(|m| m as &dyn Producer).collect();
    let mut sorted_stages = Vec::new();
    for stage in STAGE_ORDER {
        let stage_id = (*stage).to_string();
        let nodes = build_intra_stage_dag(stage_id.clone(), &module_producers)
            .map_err(|e| -> Box<LiveModuleLoadError> { Box::new(LiveModuleLoadError::Dag(*e)) })?;
        if nodes.is_empty() {
            continue;
        }
        let module_ids =
            topological_sort(&nodes).map_err(|unsorted| LiveModuleLoadError::Cycle {
                stage_id: stage_id.clone(),
                unsorted,
            })?;
        sorted_stages.push(SortedStageModules {
            stage_id,
            module_ids,
        });
    }

    // Build per-module runtime bindings, compiling each module's .wasm
    // into a reusable `WasmComponent` via a single shared engine.
    let engine = Arc::new(WasmEngine::new());
    let mut diagnostics = report.diagnostics;
    let mut bindings = Vec::with_capacity(report.modules.len());
    for module in report.modules {
        let pool = build_wasm_instance_pool(
            module.id(),
            module.stage(),
            module.layer_parallel_safe(),
            host_parallelism,
            WasmArtifactMetadata::default(),
        )
        .map_err(|e| -> Box<LiveModuleLoadError> {
            Box::new(LiveModuleLoadError::InstancePool(e))
        })?;
        let wasm_component = compile_module_component(engine.as_ref(), &module, &mut diagnostics);
        bindings.push(LiveModuleBinding {
            module,
            instance_pool: Arc::new(pool),
            wasm_component,
        });
    }

    Ok(LiveModuleLoadOutput {
        bindings,
        sorted_stages,
        diagnostics,
        engine,
    })
}

/// Compile one module's `.wasm` into a `WasmComponent`, or push a
/// structured `LoadDiagnostic` and return `None` for the well-defined
/// skip cases (placeholder binary, read failure, or non-component
/// compile failure). Dispatch-time will surface a typed error if a
/// `None` component is actually needed.
fn compile_module_component(
    engine: &WasmEngine,
    module: &LoadedModule,
    diagnostics: &mut Vec<LoadDiagnostic>,
) -> Option<Arc<WasmComponent>> {
    if module.placeholder_wasm() {
        diagnostics.push(LoadDiagnostic {
            level: DiagnosticLevel::Warning,
            path: module.wasm_path().to_owned(),
            field: Some(String::from("wasm_path")),
            message: format!(
                "module '{id}' uses a placeholder .wasm binary; \
                 skipping component compilation (dispatch of this module will fail fatally)",
                id = module.id()
            ),
        });
        return None;
    }

    let bytes = match std::fs::read(module.wasm_path()) {
        Ok(b) => b,
        Err(e) => {
            diagnostics.push(LoadDiagnostic {
                level: DiagnosticLevel::Warning,
                path: module.wasm_path().to_owned(),
                field: Some(String::from("wasm_path")),
                message: format!(
                    "failed to read .wasm for module '{id}': {e}",
                    id = module.id()
                ),
            });
            return None;
        }
    };

    match engine.compile_component(&bytes) {
        Ok(component) => Some(Arc::new(component)),
        Err(e) => {
            diagnostics.push(LoadDiagnostic {
                level: DiagnosticLevel::Warning,
                path: module.wasm_path().to_owned(),
                field: Some(String::from("wasm_path")),
                message: format!(
                    "failed to compile component for module '{id}': {e}",
                    id = module.id()
                ),
            });
            None
        }
    }
}
