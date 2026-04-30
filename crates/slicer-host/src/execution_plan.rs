//! Immutable execution-plan contracts for the host scheduler.

use std::collections::HashMap;
use std::sync::Arc;

use std::path::PathBuf;

use slicer_ir::{
    ActiveRegion, ConfigKey, ConfigValue, ConfigView, GlobalLayer, ModuleId, RegionKey, RegionPlan,
    StageId,
};

use crate::dag::build_intra_stage_dag;
use crate::instance_pool::{
    build_wasm_instance_pool, InstancePoolError, WasmArtifactMetadata, WasmInstancePool,
};
use crate::manifest::DiagnosticLevel;
use crate::manifest::{load_modules_from_roots, LoadDiagnostic, LoadError, LoadedModule};
use crate::topology::topological_sort;
use crate::validation::SchedulerError;
use crate::wasm_instance::{WasmComponent, WasmEngine};

/// Canonical scheduler stage ordering for the live host path
/// (docs/04 §Fixed Stage Order). Modules discovered by
/// [`load_live_modules_for_plan`] are grouped and sorted in this
/// order; stages not present among the loaded modules are skipped.
pub const STAGE_ORDER: &[&str] = &[
    "PrePass::MeshSegmentation",
    "PrePass::MeshAnalysis",
    "PrePass::LayerPlanning",
    "PrePass::SeamPlanning",
    "PrePass::SupportGeometry",
    "PrePass::PaintSegmentation",
    "PrePass::RegionMapping",
    "Layer::Slice",
    "Layer::SlicePostProcess",
    "Layer::Perimeters",
    "Layer::PerimetersPostProcess",
    "Layer::Infill",
    "Layer::InfillPostProcess",
    "Layer::Support",
    "Layer::SupportPostProcess",
    "Layer::PathOptimization",
    "PostPass::LayerFinalization",
    "PostPass::GCodeEmit",
    "PostPass::GCodePostProcess",
    "PostPass::TextPostProcess",
];

/// Build the `Arc<ConfigView>` bound for one `LoadedModule` on the live
/// host/runtime path.
///
/// Pre-filters `source` to the module's declared `config_schema.entries`
/// keys (the canonical declared-read set per docs/03 §host-boundary
/// enforcement and docs/02 §pre-filtered config), then freezes the result
/// behind an `Arc` so downstream consumers cannot mutate the view they see.
///
/// This is the ONLY supported construction path for live-runtime config
/// views; test fixtures may still use `ConfigView::from_map`, but
/// production planning (main.rs / runtime binding) must route through this
/// helper to stay contract-compliant.
#[must_use]
pub fn bind_module_config_view(
    module: &LoadedModule,
    source: &HashMap<ConfigKey, ConfigValue>,
) -> Arc<ConfigView> {
    // Support `prefix:*` wildcard entries in the module's declared
    // config schema so per-object keys (e.g. `object_height:<uuid>`)
    // can be consumed by planners that only know a static schema.
    // A declared key of the form `<prefix>:*` expands to every source
    // key that begins with `<prefix>:`. Static declared keys continue
    // to require exact match (docs/03 §host-boundary enforcement;
    // docs/02 §pre-filtered config).
    let mut effective: Vec<String> = Vec::new();
    for declared_key in module.config_schema.entries.keys() {
        if let Some(prefix) = declared_key.strip_suffix(":*") {
            let needle = format!("{prefix}:");
            for src_key in source.keys() {
                if src_key.starts_with(&needle) {
                    effective.push(src_key.clone());
                }
            }
        } else {
            effective.push(declared_key.clone());
        }
    }
    Arc::new(ConfigView::from_declared(
        source,
        effective.iter().map(String::as_str),
    ))
}

/// Runtime bindings for one loaded module, minus its `ConfigView`.
///
/// Used by [`build_live_execution_plan`] to build per-module bindings
/// whose `Arc<ConfigView>` is ALWAYS synthesised through
/// [`bind_module_config_view`] — modules can't supply a hand-rolled
/// `ConfigView` on this path, so the declared-read invariant is upheld
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
/// This is the ONLY public helper allowed to assemble live bindings; any
/// caller that bypasses it and hand-rolls a `ConfigView` still has to go
/// through [`build_execution_plan`], where the declared-read guardrail
/// (`ExecutionPlanError::UndeclaredConfigKey`) fails closed.
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
                instance_pool: b.instance_pool,
                config_view,
                wasm_component: b.wasm_component,
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

/// Structured failure parsing a user-facing JSON config source.
#[derive(Debug, Clone)]
pub enum ConfigSourceParseError {
    /// The input was not valid JSON.
    InvalidJson {
        /// Human-readable serde error.
        message: String,
    },
    /// The top-level JSON value was not an object.
    NotAnObject,
    /// A value under `key` could not be mapped to any `ConfigValue` variant.
    UnsupportedValue {
        /// Key carrying the unsupported value.
        key: String,
    },
}

impl std::fmt::Display for ConfigSourceParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidJson { message } => write!(f, "invalid JSON config: {message}"),
            Self::NotAnObject => {
                write!(
                    f,
                    "top-level JSON config must be an object of key→value pairs"
                )
            }
            Self::UnsupportedValue { key } => {
                write!(f, "config key '{key}' has an unsupported JSON value (only bool, number, string, and homogeneous arrays are allowed)")
            }
        }
    }
}

impl std::error::Error for ConfigSourceParseError {}

/// Parse a user-facing JSON config source into a raw
/// `HashMap<ConfigKey, ConfigValue>` ready to be fed to
/// [`bind_module_config_view`].
///
/// JSON types map as: `bool → Bool`, integer number → `Int`, non-integer
/// number → `Float` (subnormals normalised to `0.0`, matching the WIT
/// boundary), string → `String`, and array → `List` (recursed element-wise).
/// `null` and nested object values are rejected with `UnsupportedValue`,
/// because `ConfigValue` has no `null`/record representation and silent
/// coercion would contradict docs/03 §host-boundary enforcement.
pub fn parse_cli_config_source(
    json: &str,
) -> Result<HashMap<ConfigKey, ConfigValue>, ConfigSourceParseError> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| ConfigSourceParseError::InvalidJson {
            message: e.to_string(),
        })?;
    let object = match value {
        serde_json::Value::Object(m) => m,
        _ => return Err(ConfigSourceParseError::NotAnObject),
    };
    let mut out = HashMap::with_capacity(object.len());
    for (key, raw) in object {
        let value = json_to_config_value(&raw)
            .ok_or_else(|| ConfigSourceParseError::UnsupportedValue { key: key.clone() })?;
        out.insert(key, value);
    }
    Ok(out)
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
/// This is the live production plan/build entry point. It deliberately
/// does NOT build per-module `ConfigView`s here — that happens inside
/// `build_live_execution_plan`, which routes every view through
/// [`bind_module_config_view`], so this loader cannot accidentally leak
/// undeclared config keys.
///
/// `host_parallelism` controls the pool size for `layer-parallel-safe`
/// modules; other modules use a serialised pool of size 1 per
/// `build_wasm_instance_pool`.
///
/// Each discovered module's `.wasm` file is compiled via a shared
/// [`WasmEngine`] and attached to `LiveModuleBinding.wasm_component`.
/// Modules flagged by manifest ingestion as `placeholder_wasm = true`,
/// or whose binary fails to read/compile as a component-model artifact,
/// get `wasm_component = None` plus a structured `LoadDiagnostic` on
/// the returned output. The loader never aborts on a single bad module
/// binary — that matches the docs/04 recoverability contract where
/// dispatch-time handles missing components with a typed error.
pub fn load_live_modules_for_plan(
    search_roots: &[PathBuf],
    host_parallelism: usize,
) -> Result<LiveModuleLoadOutput, Box<LiveModuleLoadError>> {
    let mut report = load_modules_from_roots(search_roots)?;

    // Claim-uniqueness enforcement (docs/04 §Global claim conflicts;
    // docs/10 §Glossary: "Exactly one holder per (layer, object, region,
    // claim)"). When two modules in the same stage declare the same
    // `claims.holds` entry (e.g. classic-perimeters + arachne-perimeters
    // both holding `perimeter-generator`), both would attempt to
    // `arena.set_perimeter` and the second fails with
    // `LayerArenaError::SlotAlreadyOccupied`. Here we keep the single
    // alphabetically-first module per (stage, claim) and drop the rest
    // with an Info diagnostic. Tests that intentionally load multiple
    // same-claim modules should either pick one via file layout or use
    // synthetic modules that declare no `holds`.
    let filtered_modules = dedup_same_claim_modules(&mut report.modules, &mut report.diagnostics);
    report.modules = filtered_modules;

    // Build per-stage topological orderings in canonical STAGE_ORDER.
    let mut sorted_stages = Vec::new();
    for stage in STAGE_ORDER {
        let stage_id = (*stage).to_string();
        let nodes = build_intra_stage_dag(stage_id.clone(), &report.modules)
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
        let pool =
            build_wasm_instance_pool(&module, host_parallelism, WasmArtifactMetadata::default())
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

/// Returns true when `key` is satisfied by some entry in `declared`,
/// either as an exact match or via a `prefix:*` wildcard pattern that
/// [`bind_module_config_view`] also accepts. See that helper for the
/// full rationale (docs/03 §host-boundary enforcement).
fn config_key_declared(
    declared: &std::collections::BTreeMap<String, crate::manifest::ConfigFieldEntry>,
    key: &str,
) -> bool {
    if declared.contains_key(key) {
        return true;
    }
    for declared_key in declared.keys() {
        if let Some(prefix) = declared_key.strip_suffix(":*") {
            let needle = format!("{prefix}:");
            if key.starts_with(&needle) {
                return true;
            }
        }
    }
    false
}

/// Enforce claim uniqueness across modules in the same stage.
///
/// For each `(stage, claim)` pair, keeps the alphabetically-first
/// `module_id` and drops the rest. Emits one `LoadDiagnostic` per
/// dropped module so operators can see which module "won" each claim.
/// Modules with no `claims.holds` entries are kept unchanged.
///
/// Matches docs/04 §2 "Global claim conflicts" (exactly one holder
/// globally per claim) and docs/10 §Glossary ("Exactly one holder per
/// (layer, object, region, claim) at execution"). Per-region scoping
/// is deferred to the region-mapping pass; at live-load time we only
/// enforce the global/stage constraint.
/// Test-only wrapper around [`dedup_same_claim_modules`] so integration
/// tests can exercise the claim dedup path without building a full
/// `LoadModulesReport`. Behaviour is identical to the private helper.
#[doc(hidden)]
pub fn dedup_same_claim_modules_for_test(
    modules: &mut Vec<LoadedModule>,
    diagnostics: &mut Vec<LoadDiagnostic>,
) -> Vec<LoadedModule> {
    dedup_same_claim_modules(modules, diagnostics)
}

fn dedup_same_claim_modules(
    modules: &mut Vec<LoadedModule>,
    diagnostics: &mut Vec<LoadDiagnostic>,
) -> Vec<LoadedModule> {
    use std::collections::BTreeMap;

    let mut winner_for: BTreeMap<(StageId, String), ModuleId> = BTreeMap::new();
    let mut sorted: Vec<LoadedModule> = std::mem::take(modules);
    sorted.sort_by(|a, b| a.id.cmp(&b.id));

    let mut kept: Vec<LoadedModule> = Vec::with_capacity(sorted.len());
    for module in sorted {
        let mut losing_claim: Option<(String, ModuleId)> = None;
        for claim in &module.claims {
            let key = (module.stage.clone(), claim.clone());
            if let Some(winner) = winner_for.get(&key) {
                losing_claim = Some((claim.clone(), winner.clone()));
                break;
            }
        }
        if let Some((claim, winner)) = losing_claim {
            diagnostics.push(LoadDiagnostic {
                level: DiagnosticLevel::Info,
                path: module.wasm_path.clone(),
                field: Some(String::from("claims.holds")),
                message: format!(
                    "module '{id}' in stage '{stage}' dropped: claim '{claim}' \
                     already held by '{winner}' (first-winner dedup; docs/04 §2)",
                    id = module.id,
                    stage = module.stage,
                    claim = claim,
                    winner = winner,
                ),
            });
            continue;
        }
        for claim in &module.claims {
            winner_for.insert((module.stage.clone(), claim.clone()), module.id.clone());
        }
        kept.push(module);
    }

    kept
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
    if module.placeholder_wasm {
        diagnostics.push(LoadDiagnostic {
            level: DiagnosticLevel::Warning,
            path: module.wasm_path.clone(),
            field: Some(String::from("wasm_path")),
            message: format!(
                "module '{id}' uses a placeholder .wasm binary; \
                 skipping component compilation (dispatch of this module will fail fatally)",
                id = module.id
            ),
        });
        return None;
    }

    let bytes = match std::fs::read(&module.wasm_path) {
        Ok(b) => b,
        Err(e) => {
            diagnostics.push(LoadDiagnostic {
                level: DiagnosticLevel::Warning,
                path: module.wasm_path.clone(),
                field: Some(String::from("wasm_path")),
                message: format!(
                    "failed to read .wasm for module '{id}': {e}",
                    id = module.id
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
                path: module.wasm_path.clone(),
                field: Some(String::from("wasm_path")),
                message: format!(
                    "failed to compile component for module '{id}': {e}",
                    id = module.id
                ),
            });
            None
        }
    }
}

fn json_to_config_value(raw: &serde_json::Value) -> Option<ConfigValue> {
    match raw {
        serde_json::Value::Bool(b) => Some(ConfigValue::Bool(*b)),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Some(ConfigValue::Int(i))
            } else if let Some(f) = n.as_f64() {
                let f = if f.is_subnormal() { 0.0 } else { f };
                Some(ConfigValue::Float(f))
            } else {
                None
            }
        }
        serde_json::Value::String(s) => Some(ConfigValue::String(s.clone())),
        serde_json::Value::Array(items) => {
            let mut converted = Vec::with_capacity(items.len());
            for item in items {
                converted.push(json_to_config_value(item)?);
            }
            Some(ConfigValue::List(converted))
        }
        serde_json::Value::Null | serde_json::Value::Object(_) => None,
    }
}

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
    /// Precomputed index for O(1) lookup of active regions per (layer, module).
    /// Key: (global_layer_index, module_id) → Value: slice of ActiveRegion.
    pub module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>>,
}

impl Default for ExecutionPlan {
    fn default() -> Self {
        Self {
            prepass_stages: Vec::new(),
            per_layer_stages: Vec::new(),
            layer_finalization_stage: None,
            postpass_stages: Vec::new(),
            global_layers: Arc::new(Vec::new()),
            region_plans: Arc::new(HashMap::new()),
            module_region_index: HashMap::new(),
        }
    }
}

impl ExecutionPlan {
    /// Build an ExecutionPlan with a precomputed module_region_index.
    #[cfg(test)]
    #[allow(dead_code)]
    pub(crate) fn build_with_index(
        prepass_stages: Vec<CompiledStage>,
        per_layer_stages: Vec<CompiledStage>,
        layer_finalization_stage: Option<CompiledStage>,
        postpass_stages: Vec<CompiledStage>,
        global_layers: Arc<Vec<GlobalLayer>>,
        region_plans: Arc<HashMap<RegionKey, RegionPlan>>,
    ) -> Self {
        // Build index for all Layer:: stages
        let mut module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>> = HashMap::new();
        for layer in global_layers.iter() {
            for stage in &per_layer_stages {
                for module in &stage.modules {
                    let key = (layer.index, module.module_id.clone());
                    let entry = module_region_index.entry(key).or_default();
                    entry.extend(layer.active_regions.iter().cloned());
                }
            }
        }

        ExecutionPlan {
            prepass_stages,
            per_layer_stages,
            layer_finalization_stage,
            postpass_stages,
            global_layers,
            region_plans,
            module_region_index,
        }
    }
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
    /// A module binding's `ConfigView` exposes a key that is not in the
    /// module's declared `[config.schema]` — a contract violation per
    /// docs/03 §host-boundary enforcement and docs/02 §pre-filtered config.
    /// Callers MUST route every per-module `ConfigView` through
    /// [`bind_module_config_view`] to avoid this error.
    UndeclaredConfigKey {
        /// Module whose `ConfigView` leaked an undeclared key.
        module_id: ModuleId,
        /// The offending undeclared key.
        key: String,
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
            Self::UndeclaredConfigKey { module_id, key } => {
                write!(
                    f,
                    "module '{module_id}' config view exposes undeclared key '{key}'; \
                     bind per-module ConfigView via bind_module_config_view() \
                     (see docs/03 §host-boundary enforcement)"
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
        // ── Declared-read guard (docs/03 §host-boundary enforcement) ──
        // Every key visible through the bound ConfigView must appear in
        // the module's declared `[config.schema]`. This is the invariant
        // upheld by `bind_module_config_view`; enforce it at plan-build
        // time so any caller bypassing the helper still fails closed.
        for key in binding.config_view.keys() {
            if !config_key_declared(&binding.module.config_schema.entries, &key) {
                return Err(ExecutionPlanError::UndeclaredConfigKey { module_id, key });
            }
        }
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

    // ── Precompute module_region_index for O(1) resolve_active_regions ──
    let mut module_region_index: HashMap<(u32, ModuleId), Vec<ActiveRegion>> = HashMap::new();
    for layer in request.global_layers.iter() {
        for stage in &request.sorted_stages {
            if !stage.stage_id.starts_with("Layer::") {
                continue;
            }
            for module_id in &stage.module_ids {
                // Only index for modules that are actually bound
                if bindings_by_module_id.contains_key(module_id) {
                    let entry = module_region_index
                        .entry((layer.index, module_id.clone()))
                        .or_default();
                    entry.extend(layer.active_regions.iter().cloned());
                }
            }
        }
    }

    Ok(ExecutionPlan {
        prepass_stages,
        per_layer_stages,
        layer_finalization_stage,
        postpass_stages,
        global_layers: Arc::clone(&request.global_layers),
        region_plans: Arc::clone(&request.region_plans),
        module_region_index,
    })
}

impl ExecutionPlan {
    /// O(1) lookup of active regions for a (layer, module) pair via precomputed index.
    pub fn resolve_active_regions(
        &self,
        layer: &GlobalLayer,
        module: &CompiledModule,
    ) -> &[ActiveRegion] {
        self.module_region_index
            .get(&(layer.index, module.module_id.clone()))
            .map(|v: &Vec<ActiveRegion>| v.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod dedup_tests {
    use std::path::PathBuf;

    use slicer_ir::SemVer;

    use super::dedup_same_claim_modules;
    use crate::manifest::{ConfigFieldEntry, ConfigSchema, LoadDiagnostic, LoadedModule};

    fn loaded(id: &str, stage: &str, holds: &[&str]) -> LoadedModule {
        LoadedModule {
            id: id.into(),
            version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            stage: stage.into(),
            wit_world: "slicer:world-layer@1.0.0".into(),
            ir_reads: Vec::new(),
            ir_writes: Vec::new(),
            claims: holds.iter().map(|s| (*s).to_string()).collect(),
            requires_claims: Vec::new(),
            incompatible_with: Vec::new(),
            requires_modules: Vec::new(),
            min_host_version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            min_ir_schema: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            max_ir_schema: SemVer {
                major: 2,
                minor: 0,
                patch: 0,
            },
            config_schema: ConfigSchema::default(),
            overridable_per_region: Vec::new(),
            overridable_per_layer: Vec::new(),
            layer_parallel_safe: true,
            wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
            placeholder_wasm: false,
        }
    }

    #[test]
    fn same_claim_same_stage_keeps_alphabetically_first_and_emits_diagnostic() {
        // Regression guard for the pre-2026-04 Benchy MVP failure mode:
        // classic-perimeters and arachne-perimeters both held
        // `perimeter-generator` in `Layer::Perimeters` and both committed
        // to the arena, producing a `LayerArenaError::SlotAlreadyOccupied`
        // masked as the generic string "arena commit failed".
        let mut modules = vec![
            loaded(
                "com.core.classic-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
            loaded(
                "com.core.arachne-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
        ];
        let mut diagnostics: Vec<LoadDiagnostic> = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics);

        assert_eq!(kept.len(), 1, "exactly one holder survives per claim");
        assert_eq!(kept[0].id, "com.core.arachne-perimeters");
        assert_eq!(diagnostics.len(), 1);
        assert!(diagnostics[0].message.contains("perimeter-generator"));
        assert!(diagnostics[0]
            .message
            .contains("com.core.classic-perimeters"));
        assert!(diagnostics[0]
            .message
            .contains("com.core.arachne-perimeters"));
    }

    #[test]
    fn different_stages_same_claim_name_do_not_collide() {
        // Claims are scoped by stage: two modules can legitimately both
        // declare the same claim name across different stages.
        let mut modules = vec![
            loaded("mod.a", "Layer::Perimeters", &["x"]),
            loaded("mod.b", "Layer::Infill", &["x"]),
        ];
        let mut diagnostics = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics);
        assert_eq!(kept.len(), 2);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn modules_with_no_claims_are_always_kept() {
        let mut modules = vec![
            loaded("mod.a", "Layer::Perimeters", &[]),
            loaded("mod.b", "Layer::Perimeters", &[]),
        ];
        let mut diagnostics = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics);
        assert_eq!(kept.len(), 2);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn bind_config_view_expands_prefix_wildcard_entries() {
        // Regression guard for planner-specific per-object config keys.
        // `layer-planner-default.toml` declares `"object_height:*"`, and
        // the bound ConfigView must preserve every matching source key
        // that was explicitly provided to the host/runtime plan builder.
        use slicer_ir::ConfigValue;
        use std::collections::HashMap;

        let mut module = loaded("planner", "PrePass::LayerPlanning", &[]);
        module.config_schema.entries.insert(
            "object_height:*".to_string(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );
        module.config_schema.entries.insert(
            "layer_height".to_string(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );

        let mut source: HashMap<String, ConfigValue> = HashMap::new();
        source.insert("object_height:abc".into(), ConfigValue::Float(48.0));
        source.insert("object_height:xyz".into(), ConfigValue::Float(12.5));
        source.insert("layer_height".into(), ConfigValue::Float(0.2));
        source.insert("unrelated_key".into(), ConfigValue::Float(1.0));

        let view = super::bind_module_config_view(&module, &source);
        let mut keys: Vec<String> = view.keys().to_vec();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "layer_height".to_string(),
                "object_height:abc".to_string(),
                "object_height:xyz".to_string(),
            ],
            "declared `object_height:*` must expand to every matching source key",
        );
    }

    #[test]
    fn config_key_declared_accepts_exact_and_wildcard() {
        use std::collections::BTreeMap;
        let mut declared: BTreeMap<String, ConfigFieldEntry> = BTreeMap::new();
        declared.insert(
            "layer_height".into(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );
        declared.insert(
            "object_height:*".into(),
            ConfigFieldEntry {
                field_type: "float".to_string(),
                ..Default::default()
            },
        );

        assert!(super::config_key_declared(&declared, "layer_height"));
        assert!(super::config_key_declared(&declared, "object_height:a"));
        assert!(super::config_key_declared(
            &declared,
            "object_height:long-uuid"
        ));
        assert!(!super::config_key_declared(&declared, "object_height"));
        assert!(!super::config_key_declared(&declared, "random_key"));
    }

    #[test]
    fn canonical_benchy_core_modules_collapse_to_one_holder_per_stage() {
        // Mirrors what live module discovery finds under modules/core-modules/
        // and documents the canonical winner for each claim after dedup.
        let mut modules = vec![
            loaded(
                "com.core.arachne-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
            loaded(
                "com.core.classic-perimeters",
                "Layer::Perimeters",
                &["perimeter-generator"],
            ),
            loaded(
                "com.core.gyroid-infill",
                "Layer::Infill",
                &["infill-generator"],
            ),
            loaded(
                "com.core.lightning-infill",
                "Layer::Infill",
                &["infill-generator"],
            ),
            loaded(
                "com.core.rectilinear-infill",
                "Layer::Infill",
                &["infill-generator"],
            ),
            loaded(
                "com.core.traditional-support",
                "Layer::Support",
                &["support-generator"],
            ),
            loaded(
                "com.core.tree-support",
                "Layer::Support",
                &["support-generator"],
            ),
        ];
        let mut diagnostics = Vec::new();
        let kept = dedup_same_claim_modules(&mut modules, &mut diagnostics);

        let ids: Vec<&str> = kept.iter().map(|m| m.id.as_str()).collect();
        // One holder per stage; alphabetically-first module id wins.
        assert_eq!(
            ids,
            [
                "com.core.arachne-perimeters",
                "com.core.gyroid-infill",
                "com.core.traditional-support",
            ]
        );
        // Four modules dropped: one infill runner-up does not emit a
        // diagnostic for itself (the second lightning-infill is already
        // losing to gyroid, then rectilinear also loses) — three drops
        // for infill, one for perimeters, one for support = 4 total.
        assert_eq!(diagnostics.len(), 4);
    }
}
