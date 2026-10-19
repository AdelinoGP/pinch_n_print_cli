//! Pipeline orchestration for the `slicer-runtime` library (TASK-075).
//!
//! This module provides the `run_pipeline` function that orchestrates the full
//! slicing pipeline: prepass → per-layer → finalization → postpass → gcode output.
//! All stage runners are injectable via traits for testability.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use slicer_ir::{
    ConfigKey, ConfigValue, LayerCollectionIR, MeshIR, ModuleId, ResolvedConfig, StageId,
};
use slicer_sdk::native::NativeStageEntry;
use slicer_wasm_host::{WasmComponent, WasmInstancePool};

/// Default for the `thumbnail_path` host config key when the user does not set
/// it. The empty string is the "no embedded thumbnail" sentinel (an absent or
/// empty value yields no `THUMBNAIL_BLOCK`). Mirrored in
/// `docs/config/host-keys.toml` (`[host_runtime]`) and locked by
/// `gcode_emit::host_keys_doc_lock`.
pub const DEFAULT_THUMBNAIL_PATH: &str = "";

use slicer_gcode::{
    parse_thumbnails_key, render_thumbnail_entries, resolved_config_to_map, RenderedThumbnail,
    ThumbnailAwareSerializer, ThumbnailFormat, ThumbnailSpec,
};

use crate::layer_executor::{
    build_and_commit_layer_selection_index, execute_blackboard_taps,
    execute_captured_stages_with_support_tools,
    execute_per_layer_with_committed_anchored_events_instrumented, promote_global_layers,
    CaptureExecutionError, CaptureRequest, StageCapture, SupportToolSelection,
};
use crate::{
    compute_serial_edges_from_compiled, execute_layer_finalization_with_instrumentation,
    execute_postpass, prepass::ConfigExpansionAuthority, Blackboard, ConfigBoundsIndex,
    ExecutionPlan, FinalizationError, FinalizationStageRunner, GCodeEmitter, GCodeSerializer,
    LayerExecutionError, LayerProgressSink, LayerStageRunner, ModuleAccessAudit,
    NoopInstrumentation, NoopLayerProgressSink, Phase, PipelineInstrumentation, PostpassError,
    PostpassStageRunner, PrepassExecutionError, PrepassStageRunner, TierKind,
};

/// Injectable stage runners for the pipeline.
pub struct PipelineStageRunners {
    /// PrePass stage runner.
    pub prepass: Box<dyn PrepassStageRunner>,
    /// Per-layer stage runner.
    pub layer: Box<dyn LayerStageRunner + Sync>,
    /// Layer finalization stage runner.
    pub finalization: Box<dyn FinalizationStageRunner>,
    /// PostPass stage runner.
    pub postpass: Box<dyn PostpassStageRunner>,
    /// GCode emitter (host-built-in).
    pub emitter: Box<dyn GCodeEmitter>,
    /// GCode serializer (host-built-in).
    pub serializer: Box<dyn GCodeSerializer>,
}

/// Configuration for the pipeline orchestration function.
pub struct PipelineConfig {
    /// Optional cooperative cancellation flag shared with the slice caller.
    pub cancel_flag: Option<std::sync::Arc<std::sync::atomic::AtomicBool>>,
    /// Loaded mesh to slice.
    pub mesh_ir: Arc<MeshIR>,
    /// Frozen execution plan from the scheduler.
    pub plan: ExecutionPlan,
    /// Injectable stage runners.
    pub runners: PipelineStageRunners,
    /// Filament indices selected for support and interface paths.
    pub support_tools: SupportToolSelection,
    /// Per-object resolved configs, keyed by `ObjectMesh.id`.
    ///
    /// Produced by [`crate::resolve_per_object_configs`] from the user-supplied CLI
    /// config source.  An empty map is valid (all objects fall back to
    /// `default_resolved_config`).
    pub resolved_configs: Arc<BTreeMap<String, ResolvedConfig>>,
    /// Global fallback [`ResolvedConfig`] used for objects not present in
    /// `resolved_configs` and passed as the default to the RegionMapping built-in.
    pub default_resolved_config: Arc<ResolvedConfig>,
    /// Numeric `[min, max]` bounds aggregated from every loaded module's
    /// manifest `[config.schema]`. Threaded into the prepass-time
    /// `resolve_per_paint_semantic_configs` call so `paint_config:<sem>:<key>`
    /// overrides are rejected on out-of-range values, same as global keys.
    /// Defaults to [`ConfigBoundsIndex::empty`] for pipeline call sites that
    /// don't load modules (in-process tests).
    pub bounds: Arc<ConfigBoundsIndex>,
    /// Per-module wasmtime handles sourced from `LiveModuleLoadOutput.bindings`.
    ///
    /// Keyed by `ModuleId`. Executors look up this map to construct a
    /// `CompiledModuleLive` for each module they dispatch. An empty map is valid
    /// for in-process test pipelines that don't exercise real WASM dispatch.
    pub wasm_handles: HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<NativeStageEntry>,
        ),
    >,
    /// Host-side anchored work, lowered into rows at finalization.
    ///
    /// Off-grid entities produced outside the per-layer grid that must survive
    /// into G-code. An empty vec is the behaviour-preserving default.
    pub anchored_entities: Vec<slicer_ir::AnchoredEntity>,
}

/// Output produced by a successful pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineOutput {
    /// The final G-code text.
    pub gcode_text: String,
    /// Runtime access audits collected during prepass execution.
    pub prepass_audits: Vec<ModuleAccessAudit>,
    /// Runtime access audits collected during per-layer execution (TASK-123b).
    pub layer_audits: Vec<ModuleAccessAudit>,
    /// Runtime access audits collected during postpass execution (TASK-123c).
    pub postpass_audits: Vec<ModuleAccessAudit>,
    /// The `RegionMapIR` the prepass committed, when one was committed.
    ///
    /// Exposed because it is the authoritative per-region resolved config —
    /// including any layer-range values — and the returned G-code does not
    /// carry per-region config (the emitter writes one config block per print).
    /// Callers that must observe what a layer actually resolved against read it
    /// here rather than re-deriving it.
    pub region_map: Option<Arc<slicer_ir::RegionMapIR>>,
}

/// Structured pipeline orchestration failures.
#[derive(Debug)]
pub enum PipelineError {
    /// Model loading failed.
    ModelLoad(String),
    /// PrePass stage execution failed.
    Prepass(PrepassExecutionError),
    /// A required whole-print preparation dependency failed.
    Preparation {
        /// Selected module whose dependency failed.
        module_id: slicer_ir::ModuleId,
        /// Original typed failure, including module fatality and code.
        failure: Arc<slicer_wasm_host::preparation::PreparationFailure>,
    },
    /// Per-layer execution failed.
    LayerExecution(LayerExecutionError),
    /// Layer finalization failed.
    Finalization(FinalizationError),
    /// PostPass execution failed.
    Postpass(PostpassError),
}

impl fmt::Display for PipelineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ModelLoad(msg) => write!(f, "model load failed: {msg}"),
            Self::Prepass(e) => write!(f, "prepass failed: {e}"),
            Self::Preparation { module_id, failure } => {
                write!(f, "preparation failed for {module_id}: {failure}")
            }
            Self::LayerExecution(e) => write!(f, "layer execution failed: {e}"),
            Self::Finalization(e) => write!(f, "finalization failed: {e}"),
            Self::Postpass(e) => write!(f, "postpass failed: {e}"),
        }
    }
}

impl std::error::Error for PipelineError {}

impl From<PrepassExecutionError> for PipelineError {
    fn from(e: PrepassExecutionError) -> Self {
        Self::Prepass(e)
    }
}

impl From<LayerExecutionError> for PipelineError {
    fn from(e: LayerExecutionError) -> Self {
        Self::LayerExecution(e)
    }
}

impl From<FinalizationError> for PipelineError {
    fn from(e: FinalizationError) -> Self {
        Self::Finalization(e)
    }
}

impl From<PostpassError> for PipelineError {
    fn from(e: PostpassError) -> Self {
        Self::Postpass(e)
    }
}

/// Execute the full slicing pipeline.
///
/// Orchestration sequence:
/// 1. Create blackboard with loaded mesh and layer count from the execution plan
/// 2. Execute prepass stages sequentially
/// 3. Execute per-layer stages in parallel via rayon
/// 4. Execute layer finalization (if present)
/// 5. Execute postpass (emit + serialize gcode)
///
/// # Errors
///
/// Returns [`PipelineError`] if any stage fails fatally.
pub fn run_pipeline(config: PipelineConfig) -> Result<PipelineOutput, PipelineError> {
    run_pipeline_with_events(config, &NoopLayerProgressSink)
}

/// Execute the full slicing pipeline, routing per-layer progress events
/// (including host-built-in paint-annotation fallback warnings) to `sink`.
pub fn run_pipeline_with_events(
    config: PipelineConfig,
    sink: &(dyn LayerProgressSink + Sync),
) -> Result<PipelineOutput, PipelineError> {
    // Compatibility callers still receive a bare body, not a CONFIG_BLOCK.
    run_pipeline_core(
        config,
        &HashMap::new(),
        sink,
        &NoopInstrumentation,
        None,
        None,
        None,
        false,
    )
}

/// Execute the full slicing pipeline with paint-semantic config overrides.
///
/// Identical to [`run_pipeline_with_events`] except `raw_config_source` is
/// forwarded to the RegionMapping built-in so that `paint_config:<semantic>:*`
/// keys in the user-supplied config are applied as per-semantic overlays
/// (AC-4 / compatibility path for callers supplying already-expanded configs).
pub fn run_pipeline_with_raw_config(
    config: PipelineConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    sink: &(dyn LayerProgressSink + Sync),
) -> Result<PipelineOutput, PipelineError> {
    run_pipeline_core(
        config,
        raw_config_source,
        sink,
        &NoopInstrumentation,
        None,
        None,
        None,
        true,
    )
}

pub(crate) fn run_pipeline_with_raw_config_authority(
    config: PipelineConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    sink: &(dyn LayerProgressSink + Sync),
    expansion_authority: ConfigExpansionAuthority<'_>,
    config_block: Option<&BTreeMap<String, ConfigValue>>,
    lifecycle: Option<&crate::preparation_lifecycle::PrintPreparationScope>,
) -> Result<PipelineOutput, PipelineError> {
    run_pipeline_core(
        config,
        raw_config_source,
        sink,
        &NoopInstrumentation,
        Some(expansion_authority),
        config_block,
        lifecycle,
        true,
    )
}

/// Execute the full slicing pipeline with bracket-shaped instrumentation
/// surfaced through `instrumentation`. Behaviour-identical to
/// [`run_pipeline_with_raw_config`] when `&NoopInstrumentation` is passed — both
/// forward to [`run_pipeline_core`].
///
/// The instrumentation receives:
/// - `on_phase_start/end` for `Phase::PrePass`, `Phase::PerLayer`, `Phase::PostPass`
/// - `record_edges` once per stage at plan freeze, for every prepass /
///   per-layer / postpass stage, carrying the serial-edge reasons derived
///   from the compiled IR access masks (see
///   [`compute_serial_edges_from_compiled`]).
/// - Per-stage / per-module brackets in all three tiers (including host
///   built-ins). Per-layer additionally brackets `on_layer_start/end`
///   around each layer.
pub fn run_pipeline_with_instrumentation(
    config: PipelineConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    sink: &(dyn LayerProgressSink + Sync),
    instrumentation: &(dyn PipelineInstrumentation + Sync),
) -> Result<PipelineOutput, PipelineError> {
    run_pipeline_core(
        config,
        raw_config_source,
        sink,
        instrumentation,
        None,
        None,
        None,
        true,
    )
}

pub(crate) fn run_pipeline_with_instrumentation_authority(
    config: PipelineConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    sink: &(dyn LayerProgressSink + Sync),
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    expansion_authority: ConfigExpansionAuthority<'_>,
    config_block: Option<&BTreeMap<String, ConfigValue>>,
    lifecycle: Option<&crate::preparation_lifecycle::PrintPreparationScope>,
) -> Result<PipelineOutput, PipelineError> {
    run_pipeline_core(
        config,
        raw_config_source,
        sink,
        instrumentation,
        Some(expansion_authority),
        config_block,
        lifecycle,
        true,
    )
}

/// Shared pipeline body for the public compatibility and internal production
/// entry points (packet 76, 1b).
///
/// Runs prepass → per-layer → finalization → postpass with phase brackets and
/// thumbnail-aware serialization. The public `run_pipeline*` functions are thin
/// forwarders supplying the raw config source and instrumentation; passing
/// `&NoopInstrumentation` reproduces the non-instrumented behaviour exactly,
/// because `execute_per_layer_with_instrumentation` / the postpass helper treat
/// the noop brackets as zero-cost no-ops.
fn run_pipeline_core(
    config: PipelineConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    sink: &(dyn LayerProgressSink + Sync),
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    expansion_authority: Option<ConfigExpansionAuthority<'_>>,
    config_block: Option<&BTreeMap<String, ConfigValue>>,
    lifecycle: Option<&crate::preparation_lifecycle::PrintPreparationScope>,
    wrap_serializer: bool,
) -> Result<PipelineOutput, PipelineError> {
    run_pipeline_until_preparation(
        config,
        raw_config_source,
        sink,
        instrumentation,
        expansion_authority,
        config_block,
        lifecycle,
        wrap_serializer,
        false,
        None,
    )
    .map(|output| output.expect("full pipeline completes consumers"))
}

/// Private tap-completion state for one preparation visual run.
///
/// The public facade is `run_slice_preparation_visual_capture` in
/// `crate::run`, which also validates the request shape. This carrier exists
/// so the shared pipeline can report typed layer/owner resolution and real
/// truncated selected-layer capture evidence without widening the public
/// `PipelineError` (which stays a string-preserving ordinary surface).
pub(crate) struct PreparationVisualCompletion {
    /// Requested global layer indices, already deduplicated by the facade.
    pub(crate) layers: Vec<u32>,
    /// Requested stage taps, already classified by the facade.
    pub(crate) stage_taps: Vec<StageId>,
    /// Real captures and execution evidence, filled after preparation.
    pub(crate) parts: Option<PreparationVisualParts>,
    /// Typed visual failure; the pipeline returns a sentinel error so the
    /// facade can prefer this typed value over formatted text.
    pub(crate) error: Option<crate::preparation_lifecycle::PreparationVisualCaptureError>,
}

/// Captures and executed-closure evidence produced by the visual completion.
#[derive(Default)]
pub(crate) struct PreparationVisualParts {
    /// Ordinary stage captures, sorted by the executors' own tap/layer order.
    pub(crate) captures: Vec<StageCapture>,
    /// Per-layer stage ids the truncated closure actually ran.
    pub(crate) executed_stage_ids: Vec<StageId>,
    /// Global layer indices actually executed (never the rendered subset when
    /// the closure and the request differ).
    pub(crate) executed_layer_indices: Vec<u32>,
}

/// Resolve requested layers and eligible owner participation after the
/// committed selection and before any preparation invocation.
///
/// Layer membership is read from the promoted committed schedule; owner
/// participation uses exactly the same selection predicate as
/// `capture_committed_selection` (a layer whose selection both invokes the
/// owner and carries targets). An owner that never participates has no Ready
/// plan to publish a requested view, so the request fails closed as inactive.
fn resolve_preparation_visual_request(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    lifecycle: &crate::preparation_lifecycle::PrintPreparationScope,
    completion: &PreparationVisualCompletion,
) -> Result<(), crate::preparation_lifecycle::PreparationVisualCaptureError> {
    use crate::preparation_lifecycle::PreparationVisualCaptureError as VisualError;

    let schedule_layers: std::collections::HashSet<u32> =
        plan.global_layers.iter().map(|layer| layer.index).collect();
    for &global_layer_index in &completion.layers {
        if !schedule_layers.contains(&global_layer_index) {
            return Err(VisualError::UnknownLayer { global_layer_index });
        }
    }

    let selection = blackboard.layer_selection_index().ok_or_else(|| {
        VisualError::Slice(crate::run::SliceRunError(
            "preparation visual request: committed layer selection index missing".into(),
        ))
    })?;
    for request in lifecycle.preparation_views() {
        let participates = plan.global_layers.iter().any(|layer| {
            selection
                .selection_for(layer.index, &request.module_id)
                .is_some_and(|selected| selected.invoke && !selected.targets.is_empty())
        });
        if !participates {
            return Err(VisualError::InactiveOwner {
                module_id: request.module_id.clone(),
            });
        }
    }
    Ok(())
}

/// Run the real truncated selected-layer capture executor for per-layer taps
/// and the committed-slot reader for prepass taps.
///
/// The lifecycle-aware `PreparedLayerStageRunner` is passed unchanged, so
/// consumer calls consume the same Ready plans preparation just published.
///
/// Requested taps are partitioned into ADR-0040's three classes. Per-layer
/// taps run the truncated selected-layer closure and blackboard taps read
/// already-committed slots. A whole-print postpass tap is refused before any
/// capture runs: ADR-0040's furthest-stage rule makes that tap a full
/// prepass -> all per-layer stages over all layers -> layer finalization ->
/// postpass closure, while this visual path disposes the print scope right
/// after preparation (`run_pipeline_until_preparation` returns `Ok(None)`
/// below). Executing only the truncated closure and presenting the result as
/// a postpass capture would fabricate execution evidence, so the path fails
/// closed with a named error instead of `UnknownTap` or a silent success.
fn execute_preparation_visual_taps(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<NativeStageEntry>,
        ),
    >,
    support_tools: SupportToolSelection,
    completion: &PreparationVisualCompletion,
) -> Result<PreparationVisualParts, crate::preparation_lifecycle::PreparationVisualCaptureError> {
    use crate::preparation_lifecycle::preparation_visual_capture::{
        classify_preparation_visual_taps, postpass_tap_closure_not_forwarded,
    };
    use crate::preparation_lifecycle::PreparationVisualCaptureError as VisualError;

    let capture_failure = |error: CaptureExecutionError| {
        VisualError::Slice(crate::run::SliceRunError(error.to_string()))
    };

    let classes =
        classify_preparation_visual_taps(&completion.stage_taps).map_err(capture_failure)?;
    if let Some(error) = postpass_tap_closure_not_forwarded(&classes.postpass) {
        return Err(error);
    }

    let mut parts = PreparationVisualParts::default();
    if !classes.per_layer.is_empty() {
        let output = execute_captured_stages_with_support_tools(
            plan,
            blackboard,
            runner,
            wasm_handles,
            support_tools,
            &CaptureRequest {
                stage_ids: classes.per_layer,
                layer_indices: completion.layers.clone(),
            },
        )
        .map_err(capture_failure)?;
        parts.captures.extend(output.captures);
        parts.executed_stage_ids.extend(output.closure_stage_ids);
        parts
            .executed_layer_indices
            .extend(output.executed_layer_indices);
    }
    if !classes.blackboard.is_empty() {
        // Prepass taps read already-committed slots; they execute no layer and
        // therefore add nothing to either execution axis.
        let output = execute_blackboard_taps(
            blackboard,
            &CaptureRequest {
                stage_ids: classes.blackboard,
                layer_indices: completion.layers.clone(),
            },
        )
        .map_err(capture_failure)?;
        parts.captures.extend(output.captures);
    }
    Ok(parts)
}

/// Both full slicing and metadata capture enter this body; capture exits only
/// after the final committed preparation closure, before any consumer work.
pub(crate) fn run_pipeline_until_preparation(
    config: PipelineConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    sink: &(dyn LayerProgressSink + Sync),
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    expansion_authority: Option<ConfigExpansionAuthority<'_>>,
    config_block: Option<&BTreeMap<String, ConfigValue>>,
    lifecycle: Option<&crate::preparation_lifecycle::PrintPreparationScope>,
    wrap_serializer: bool,
    preparation_only: bool,
    mut visual: Option<&mut PreparationVisualCompletion>,
) -> Result<Option<PipelineOutput>, PipelineError> {
    let PipelineConfig {
        mesh_ir,
        mut plan,
        mut runners,
        resolved_configs,
        default_resolved_config,
        bounds,
        wasm_handles,
        support_tools,
        cancel_flag,
        anchored_entities,
    } = config;

    // Plan-freeze: emit one `record_edges` call per stage so the report has
    // the full serial-edge explainer up front, even for stages where no
    // modules fire on this run.
    for stage in &plan.prepass_stages {
        let edges = compute_serial_edges_from_compiled(&stage.modules);
        instrumentation.record_edges(&stage.stage_id, TierKind::PrePass, &edges);
    }
    for stage in &plan.per_layer_stages {
        let edges = compute_serial_edges_from_compiled(&stage.modules);
        instrumentation.record_edges(&stage.stage_id, TierKind::PerLayer, &edges);
    }
    if let Some(stage) = plan.layer_finalization_stage.as_ref() {
        let edges = compute_serial_edges_from_compiled(&stage.modules);
        instrumentation.record_edges(&stage.stage_id, TierKind::PostPass, &edges);
    }
    for stage in &plan.postpass_stages {
        let edges = compute_serial_edges_from_compiled(&stage.modules);
        instrumentation.record_edges(&stage.stage_id, TierKind::PostPass, &edges);
    }

    let mut blackboard = Blackboard::new(mesh_ir, 0);

    if cancel_flag
        .as_ref()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    {
        return Err(PipelineError::LayerExecution(
            LayerExecutionError::Cancelled,
        ));
    }
    instrumentation.on_phase_start(Phase::PrePass);
    let prepass_audits = crate::prepass::execute_prepass_with_builtins_configured_instr_authority(
        &plan,
        &mut blackboard,
        runners.prepass.as_ref(),
        &resolved_configs,
        &default_resolved_config,
        raw_config_source,
        &bounds,
        instrumentation,
        &wasm_handles,
        expansion_authority,
    );
    let prepass_audits = match prepass_audits {
        Ok(audits) => audits,
        Err(error) => {
            instrumentation.on_phase_end(Phase::PrePass);
            return Err(error.into());
        }
    };

    dump_prepass_ir_if_requested(&blackboard);

    let preparation_result = (|| -> Result<(), PipelineError> {
        promote_global_layers(&mut plan, &blackboard);
        build_and_commit_layer_selection_index(&plan, &mut blackboard).map_err(|error| {
            let slicer_ir::LayerStageError::FatalModule {
                stage_id,
                module_id,
                message,
            } = error
            else {
                unreachable!("selection construction returns only FatalModule errors")
            };
            LayerExecutionError::FatalLayer {
                // Construction fails before the first layer is invoked.
                layer_index: plan.global_layers.first().map_or(0, |layer| layer.index),
                stage_id,
                module_id,
                message,
            }
        })?;

        if let Some(lifecycle) = lifecycle {
            lifecycle
                .capture_committed_selection(&plan, &blackboard)
                .map_err(|message| LayerExecutionError::FatalLayer {
                    layer_index: plan.global_layers.first().map_or(0, |layer| layer.index),
                    stage_id: "Layer::Selection".into(),
                    module_id: "host".into(),
                    message,
                })?;
            if let Some(completion) = visual.as_deref() {
                let resolution =
                    resolve_preparation_visual_request(&plan, &blackboard, lifecycle, completion);
                if let Err(error) = resolution {
                    // The typed value is the authoritative failure; the
                    // pipeline error only carries the run out of the shared
                    // body. The facade prefers `completion.error`.
                    if let Some(completion) = visual.as_deref_mut() {
                        completion.error = Some(error);
                    }
                    return Err(PipelineError::ModelLoad(
                        "preparation visual request rejected before preparation".into(),
                    ));
                }
            }
            lifecycle.prepare_committed(cancel_flag.clone())?;
        }
        Ok(())
    })();
    instrumentation.on_phase_end(Phase::PrePass);
    preparation_result?;

    if let Some(completion) = visual {
        let taps = execute_preparation_visual_taps(
            &plan,
            &blackboard,
            runners.layer.as_ref(),
            &wasm_handles,
            support_tools,
            completion,
        );
        match taps {
            Ok(parts) => completion.parts = Some(parts),
            Err(error) => {
                // The typed value is the authoritative failure (including the
                // named postpass-closure rejection); the pipeline error only
                // carries the run out of the shared body, and the facade
                // prefers `completion.error`.
                completion.error = Some(error);
                return Err(PipelineError::ModelLoad(
                    "preparation visual tap capture failed".into(),
                ));
            }
        }
        // Visual completion never enters the per-layer, finalization or
        // postpass closure; callers dispose the print scope immediately.
        return Ok(None);
    }

    if preparation_only {
        return Ok(None);
    }

    if cancel_flag
        .as_ref()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    {
        return Err(PipelineError::LayerExecution(
            LayerExecutionError::Cancelled,
        ));
    }
    instrumentation
        .on_phase_start_with_layer_count(Phase::PerLayer, Some(plan.global_layers.len() as u32));
    let per_layer_result = execute_per_layer_with_committed_anchored_events_instrumented(
        &plan,
        &blackboard,
        runners.layer.as_ref(),
        sink,
        instrumentation,
        &wasm_handles,
        support_tools,
        cancel_flag.as_deref(),
        &anchored_entities,
        lifecycle.and_then(|scope| scope.observer()),
    );
    instrumentation.on_phase_end(Phase::PerLayer);
    let (committed_events, layer_audits) = per_layer_result?;
    let mut layer_irs = crate::anchored_rows::synthesize_anchored_rows(committed_events)?;

    if cancel_flag
        .as_ref()
        .is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
    {
        return Err(PipelineError::LayerExecution(
            LayerExecutionError::Cancelled,
        ));
    }
    instrumentation.on_phase_start(Phase::PostPass);
    let post_result = (|| -> Result<(String, Vec<ModuleAccessAudit>), PipelineError> {
        // Bracket layer finalization as a synthetic stage so the report's
        // per-stage table accounts for time spent in the finalization tier.
        // Individual finalization modules are now bracketed via the
        // instrumented variant (matching the postpass pattern).
        if let Some(fin_stage) = plan.layer_finalization_stage.as_ref() {
            instrumentation.on_stage_start(&fin_stage.stage_id, None);
        }
        let fin_result = execute_layer_finalization_with_instrumentation(
            &plan,
            &blackboard,
            runners.finalization.as_ref(),
            &mut layer_irs,
            instrumentation,
            &wasm_handles,
        );
        if let Some(fin_stage) = plan.layer_finalization_stage.as_ref() {
            instrumentation.on_stage_end(&fin_stage.stage_id, None);
        }
        fin_result?;

        if !wrap_serializer {
            return execute_postpass(
                &plan,
                &layer_irs,
                &blackboard,
                runners.emitter.as_ref(),
                runners.serializer.as_ref(),
                runners.postpass.as_mut(),
                &wasm_handles,
            )
            .map_err(PipelineError::from);
        }
        run_postpass_with_thumbnail(
            &plan,
            &blackboard,
            &mut runners,
            raw_config_source,
            &default_resolved_config,
            config_block,
            &layer_irs,
            instrumentation,
            &wasm_handles,
        )
    })();
    instrumentation.on_phase_end(Phase::PostPass);
    let (gcode_text, postpass_audits) = post_result?;

    Ok(Some(PipelineOutput {
        gcode_text,
        prepass_audits,
        layer_audits,
        postpass_audits,
        region_map: blackboard.region_map().cloned(),
    }))
}

/// Dump the committed prepass IR artifacts when `PNP_DUMP_PREPASS_IR` names a
/// directory. Off unless the variable is set, so the normal slice path pays
/// only one environment read.
///
/// Exists so performance harnesses can replay a *real* model's prepass output
/// (benchy scale: hundreds of layers) instead of hand-built fixtures, which do
/// not reproduce the cost distribution being measured. Written on demand rather
/// than committed, per the repo's "re-record, don't vendor large fixtures" rule.
///
/// Failures are reported but never fatal: this is a diagnostic side-channel and
/// must not be able to fail a slice.
fn dump_prepass_ir_if_requested(blackboard: &Blackboard) {
    let Ok(dir) = std::env::var("PNP_DUMP_PREPASS_IR") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        eprintln!("PNP_DUMP_PREPASS_IR: cannot create {}: {e}", dir.display());
        return;
    }

    // Postcard, not JSON: `RegionMapIR` and `SupportGeometryIR` are maps keyed
    // by `RegionKey` / `SupportGeometryKey` structs, and JSON object keys must
    // be strings — `serde_json` rejects both with "key must be a string".
    // Postcard has no such restriction, is already the workspace's IR codec,
    // and keeps the mesh dump ~10x smaller.
    fn write<T: serde::Serialize>(dir: &std::path::Path, name: &str, value: &T) {
        let path = dir.join(format!("{name}.postcard"));
        match postcard::to_allocvec(value).map(|bytes| std::fs::write(&path, bytes)) {
            Ok(Ok(())) => eprintln!("PNP_DUMP_PREPASS_IR: wrote {}", path.display()),
            Ok(Err(e)) => eprintln!("PNP_DUMP_PREPASS_IR: write {} failed: {e}", path.display()),
            Err(e) => eprintln!("PNP_DUMP_PREPASS_IR: encode {name} failed: {e}"),
        }
    }

    write(&dir, "mesh", blackboard.mesh().as_ref());
    if let Some(ir) = blackboard.layer_plan() {
        write(&dir, "layer_plan", ir.as_ref());
    }
    if let Some(ir) = blackboard.region_map() {
        write(&dir, "region_map", ir.as_ref());
    }
    if let Some(ir) = blackboard.support_geometry() {
        write(&dir, "support_geometry", ir.as_ref());
    }
}

/// PNG file signature — first 8 bytes of every valid PNG.
const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Shared post-finalization helper: validate the optional thumbnail file, build
/// the effective config map, wrap the serializer, and run the instrumented
/// postpass. Pass `&NoopInstrumentation` for the non-report path; the noop
/// methods are empty, so callers that don't need a report pay no overhead.
fn run_postpass_with_thumbnail(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runners: &mut PipelineStageRunners,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    default_resolved_config: &ResolvedConfig,
    config_block: Option<&BTreeMap<String, ConfigValue>>,
    layer_irs: &[LayerCollectionIR],
    instrumentation: &(dyn PipelineInstrumentation + Sync),
    wasm_handles: &HashMap<
        ModuleId,
        (
            Arc<WasmInstancePool>,
            Option<Arc<WasmComponent>>,
            Option<NativeStageEntry>,
        ),
    >,
) -> Result<(String, Vec<ModuleAccessAudit>), PipelineError> {
    // Extract and validate thumbnail bytes from raw_config before serialization.
    // If thumbnail_path is non-empty, read the file and check PNG magic; fail fast on error.
    let thumbnail_bytes: Option<Vec<u8>> = match raw_config_source.get("thumbnail_path") {
        Some(ConfigValue::String(path)) if path != DEFAULT_THUMBNAIL_PATH => {
            let bytes = std::fs::read(path).map_err(|_| PostpassError::GCodeSerialization {
                message: format!("thumbnail_path: file not found: {path}"),
            })?;
            if bytes.len() < 8 || bytes[..8] != PNG_MAGIC {
                return Err(PipelineError::Postpass(PostpassError::GCodeSerialization {
                    message: format!("thumbnail_path: invalid PNG magic in file: {path}"),
                }));
            }
            Some(bytes)
        }
        _ => None,
    };

    // Build the effective config map.
    //
    // When the caller supplies a pre-resolved `config_block`, it is used
    // verbatim: the upstream resolver has already performed defaults resolution,
    // overlay, and removal of invocation-time routing keys such as
    // `thumbnail_path`.
    //
    // Otherwise (legacy path) resolved defaults form the baseline, the
    // user-supplied raw config is overlaid (raw values take precedence), and
    // `thumbnail_path` is stripped here so this invocation-time routing key does
    // not appear in CONFIG_BLOCK. The baseline keeps CONFIG_BLOCK non-empty even
    // when raw_config_source is empty (AC-9 / NEG-4) while still including all
    // user-passed keys (AC-8).
    let effective_config: HashMap<String, ConfigValue> = match config_block {
        Some(block) => block.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
        None => {
            let mut legacy_config = resolved_config_to_map(default_resolved_config);
            for (k, v) in raw_config_source {
                legacy_config.insert(k.clone(), v.clone());
            }
            legacy_config.remove("thumbnail_path");
            legacy_config
        }
    };

    // Wrap the serializer with thumbnail support when bytes are present.
    let inner_serializer = std::mem::replace(
        &mut runners.serializer,
        Box::new(slicer_gcode::DefaultGCodeSerializer::new()),
    );
    // Build the rendered thumbnail entries. When thumbnail bytes are present,
    // read the `thumbnails` config key (if any) to determine which
    // resolutions/formats to render. Absent the key, default to a single PNG
    // entry at the source PNG's native dimensions (IHDR bytes 16-23), passed
    // through verbatim by the renderer's same-size PNG fast path.
    let thumbnails: Option<Vec<RenderedThumbnail>> = if let Some(bytes) = thumbnail_bytes.as_ref() {
        let (src_w, src_h) = {
            let mut reader = std::io::Cursor::new(bytes);
            let decoder = png::Decoder::new(&mut reader);
            let info = decoder
                .read_info()
                .expect("validated PNG header must decode");
            (info.info().width, info.info().height)
        };
        let specs: Vec<ThumbnailSpec> = match raw_config_source.get("thumbnails") {
            Some(ConfigValue::String(s)) => {
                parse_thumbnails_key(s).map_err(|e| PostpassError::GCodeSerialization {
                    message: format!("invalid `thumbnails` config key: {e}"),
                })?
            }
            _ => vec![ThumbnailSpec::new(src_w, src_h, ThumbnailFormat::Png)],
        };
        Some(render_thumbnail_entries(bytes, &specs).map_err(|e| {
            PostpassError::GCodeSerialization {
                message: format!("{e}"),
            }
        })?)
    } else {
        None
    };
    runners.serializer = Box::new(ThumbnailAwareSerializer::new(
        inner_serializer,
        thumbnails,
        effective_config,
    ));

    crate::postpass::execute_postpass_with_instrumentation(
        plan,
        layer_irs,
        blackboard,
        runners.emitter.as_ref(),
        runners.serializer.as_ref(),
        runners.postpass.as_mut(),
        instrumentation,
        wasm_handles,
    )
    .map_err(PipelineError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_error_display() {
        let err = PipelineError::Postpass(PostpassError::GCodeEmit {
            message: "test".into(),
        });
        assert!(err.to_string().contains("postpass failed"));
    }
}
