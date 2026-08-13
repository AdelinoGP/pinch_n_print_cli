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
use slicer_sdk::native::NativeStageEntry;

use slicer_scheduler::dag::{build_intra_stage_dag, Producer};
use slicer_scheduler::execution_plan::{
    bind_module_config_view, build_execution_plan, dedup_same_claim_modules_with_wall_generator,
    ExecutionModuleBinding, ExecutionPlan, ExecutionPlanError, ExecutionPlanRequest,
    SortedStageModules, SPIRAL_VASE_CONFIG_KEY, STAGE_ORDER, SUPPORT_GENERATOR_CONFIG_KEY,
    WALL_GENERATOR_CONFIG_KEY,
};
use slicer_scheduler::manifest::{
    load_modules_from_roots_with_integrated, LoadDiagnostic, LoadError, LoadedModule,
};
use slicer_scheduler::topology::topological_sort;
use slicer_scheduler::validation::SchedulerError;
use slicer_scheduler::{IntegratedModuleRegistration, ModuleProvenance};

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
    /// Native entry point for an integrated module, when registered.
    pub native_entry: Option<NativeStageEntry>,
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
    diagnostics: &mut Vec<LoadDiagnostic>,
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

    build_execution_plan(
        &ExecutionPlanRequest {
            sorted_stages,
            module_bindings,
            global_layers,
            region_plans,
        },
        diagnostics,
    )
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
    /// A module's WASM artifact could not be loaded for live dispatch.
    Component {
        /// Module ID whose artifact could not be loaded.
        module_id: String,
        /// Human-readable cause of the artifact load failure.
        cause: String,
    },
    /// An integrated module has no registered native entry for its stage.
    NativeEntry {
        /// Module ID whose native entry is missing.
        module_id: String,
        /// Stage family that requires the entry.
        stage_id: String,
    },
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
            Self::Component { module_id, cause } => {
                write!(
                    f,
                    "module '{module_id}' WASM component load failed: {cause}"
                )
            }
            Self::NativeEntry {
                module_id,
                stage_id,
            } => write!(
                f,
                "integrated module '{module_id}' has no native entry for stage '{stage_id}'"
            ),
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
///
/// Equivalent to [`load_live_modules_for_plan_with_config`] with an empty
/// config source — the `perimeter-generator` claim (contested by
/// `com.core.classic-perimeters` / `com.core.arachne-perimeters`) resolves to
/// `DEFAULT_WALL_GENERATOR` (`"classic"`) rather than alphabetical order.
/// Callers that need to honor a user's `wall_generator` config (i.e. the
/// production `run_slice` path) MUST use
/// [`load_live_modules_for_plan_with_config`] instead.
pub fn load_live_modules_for_plan(
    search_roots: &[PathBuf],
    host_parallelism: usize,
) -> Result<LiveModuleLoadOutput, Box<LiveModuleLoadError>> {
    load_live_modules_for_plan_with_config(search_roots, host_parallelism, &HashMap::new())
}

/// Same as [`load_live_modules_for_plan`], except the `perimeter-generator`
/// claim collision between `com.core.classic-perimeters` and
/// `com.core.arachne-perimeters` is resolved by the `wall_generator` key in
/// `config_source` (`"classic"` | `"arachne"`, default `"classic"` when
/// absent — see `slicer_scheduler::execution_plan::WALL_GENERATOR_CONFIG_KEY`
/// / `DEFAULT_WALL_GENERATOR`) instead of alphabetical module-id order.
///
/// This is the entry point the production `run_slice` path
/// (`crates/slicer-runtime/src/run.rs`) uses so a user's config can express
/// intent; see docs/04 §2 "Global claim conflicts" and packet 112 Step 10 for
/// the production defect this closes (the two-arg `load_live_modules_for_plan`
/// silently selected `arachne-perimeters` — alphabetically first — with no
/// config input, and `incompatible-with` never fired because dedup runs
/// before `validate_startup_dag`).
pub fn load_live_modules_for_plan_with_config(
    search_roots: &[PathBuf],
    host_parallelism: usize,
    config_source: &HashMap<ConfigKey, ConfigValue>,
) -> Result<LiveModuleLoadOutput, Box<LiveModuleLoadError>> {
    load_live_modules_for_plan_profiled(search_roots, host_parallelism, config_source, false)
}

/// Same as [`load_live_modules_for_plan_with_config`], plus control over
/// whether the shared [`WasmEngine`] meters fuel (ADR-0055).
///
/// `profile` is the single switch that turns fuel-based module profiling on for
/// a whole run. It reaches the guest by two routes, both derived from this one
/// engine: `wasmtime::Config::consume_fuel` (so `store.get_fuel()` returns a
/// real reading) and `WasmEngine::profiling_enabled`, which
/// `WasmRuntimeDispatcher`'s store constructor copies onto every
/// `HostExecutionContext` as the answer to the WIT `profile-enabled` query.
/// Because both come from the engine, there is no second place a caller could
/// forget to flip.
///
/// Kept as a separate entry point rather than a fourth parameter on
/// [`load_live_modules_for_plan_with_config`] so the dozens of existing call
/// sites — none of which profile — stay untouched.
pub fn load_live_modules_for_plan_profiled(
    search_roots: &[PathBuf],
    host_parallelism: usize,
    config_source: &HashMap<ConfigKey, ConfigValue>,
    profile: bool,
) -> Result<LiveModuleLoadOutput, Box<LiveModuleLoadError>> {
    load_live_modules_for_plan_with_integrated(
        search_roots,
        host_parallelism,
        config_source,
        profile,
        &[],
        &[],
    )
}

/// Same as [`load_live_modules_for_plan_profiled`], plus integrated-module
/// registrations (ADR-0056): embedded-manifest modules with no on-disk
/// `.wasm`, forming search tier 5 beneath the four search-path tiers.
///
/// Integrated modules flow through the identical ingestion/claims/DAG
/// pipeline; the only difference on this path is that a module whose
/// [`ModuleProvenance`] is `Integrated` skips component compilation — its
/// [`LiveModuleBinding`] gets `wasm_component: None` and
/// `compile_module_component` is never attempted for it (there is no `.wasm`
/// artifact to read).
///
/// Kept as a separate entry point rather than a fifth parameter on
/// [`load_live_modules_for_plan_profiled`] so existing call sites — none of
/// which register integrated modules — stay untouched.
pub fn load_live_modules_for_plan_with_integrated(
    search_roots: &[PathBuf],
    host_parallelism: usize,
    config_source: &HashMap<ConfigKey, ConfigValue>,
    profile: bool,
    integrated: &[IntegratedModuleRegistration],
    native_entries: &[(ModuleId, NativeStageEntry)],
) -> Result<LiveModuleLoadOutput, Box<LiveModuleLoadError>> {
    let mut report = load_modules_from_roots_with_integrated(search_roots, integrated)?;

    let wall_generator = config_source
        .get(WALL_GENERATOR_CONFIG_KEY)
        .and_then(|v| match v {
            ConfigValue::String(s) => Some(s.as_str()),
            _ => None,
        });

    let spiral_vase = config_source
        .get(SPIRAL_VASE_CONFIG_KEY)
        .and_then(|v| match v {
            ConfigValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap_or(false);

    let support_type = config_source
        .get(SUPPORT_GENERATOR_CONFIG_KEY)
        .and_then(|v| match v {
            ConfigValue::String(s) => Some(s.as_str()),
            _ => None,
        });

    // Claim-uniqueness enforcement, config-aware for `perimeter-generator`
    // and `support-generator`. `spiral_vase` forces the classic perimeter
    // generator (Arachne is incompatible with spiral-vase mode);
    // `support_type` selects the `support-generator` claim holder
    // (traditional by default).
    let filtered_modules = dedup_same_claim_modules_with_wall_generator(
        &mut report.modules,
        &mut report.diagnostics,
        wall_generator,
        spiral_vase,
        support_type,
    );
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
    let engine = Arc::new(WasmEngine::with_profiling(profile));
    let diagnostics = report.diagnostics;
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
        let instance_pool = Arc::new(pool);
        let native_entry = (module.provenance() == ModuleProvenance::Integrated)
            .then(|| {
                native_entries
                    .iter()
                    .find(|(id, _)| id == module.id())
                    .map(|(_, entry)| *entry)
            })
            .flatten();
        if module.provenance() == ModuleProvenance::Integrated && native_entry.is_none() {
            return Err(Box::new(LiveModuleLoadError::NativeEntry {
                module_id: module.id().to_string(),
                stage_id: module.stage().to_string(),
            }));
        }
        // ADR-0056: integrated modules carry no on-disk `.wasm` artifact;
        // dispatch for them is native, so component compilation is skipped.
        if module.provenance() == ModuleProvenance::Integrated {
            bindings.push(LiveModuleBinding {
                module,
                instance_pool,
                wasm_component: None,
                native_entry,
            });
            continue;
        }
        let wasm_component = compile_module_component(engine.as_ref(), &module)?;
        bindings.push(LiveModuleBinding {
            module,
            instance_pool,
            wasm_component: Some(wasm_component),
            native_entry,
        });
    }

    Ok(LiveModuleLoadOutput {
        bindings,
        sorted_stages,
        diagnostics,
        engine,
    })
}

/// Compile one module's `.wasm` into a `WasmComponent`.
fn compile_module_component(
    engine: &WasmEngine,
    module: &LoadedModule,
) -> Result<Arc<WasmComponent>, Box<LiveModuleLoadError>> {
    if module.placeholder_wasm() {
        return Err(Box::new(LiveModuleLoadError::Component {
            module_id: module.id().to_owned(),
            cause: String::from("placeholder .wasm binary"),
        }));
    }

    let bytes = std::fs::read(module.wasm_path()).map_err(|e| {
        Box::new(LiveModuleLoadError::Component {
            module_id: module.id().to_owned(),
            cause: format!("failed to read .wasm: {e}"),
        })
    })?;

    engine
        .compile_component(&bytes)
        .map(|component| Arc::new(component))
        .map_err(|e| {
            Box::new(LiveModuleLoadError::Component {
                module_id: module.id().to_owned(),
                cause: format!("failed to compile component: {e}"),
            })
        })
}
