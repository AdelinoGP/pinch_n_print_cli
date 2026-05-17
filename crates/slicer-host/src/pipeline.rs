//! Pipeline orchestration for the slicer-host binary (TASK-075).
//!
//! This module provides the `run_pipeline` function that orchestrates the full
//! slicing pipeline: prepass → per-layer → finalization → postpass → gcode output.
//! All stage runners are injectable via traits for testability.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use slicer_ir::{ConfigKey, ConfigValue, LayerCollectionIR, MeshIR, ResolvedConfig};

use crate::{
    compute_serial_edges_from_compiled, execute_layer_finalization, execute_per_layer_with_events,
    execute_per_layer_with_instrumentation, execute_postpass,
    gcode_emit::{resolved_config_to_map, ThumbnailAwareSerializer},
    prepass::execute_prepass_with_builtins_configured,
    Blackboard, ExecutionPlan, FinalizationError, FinalizationStageRunner, GCodeEmitter,
    GCodeSerializer, LayerExecutionError, LayerProgressSink, LayerStageRunner, ModuleAccessAudit,
    NoopLayerProgressSink, Phase, PipelineInstrumentation, PostpassError, PostpassStageRunner,
    PrepassExecutionError, PrepassStageRunner, TierKind,
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
    /// Loaded mesh to slice.
    pub mesh_ir: Arc<MeshIR>,
    /// Frozen execution plan from the scheduler.
    pub plan: ExecutionPlan,
    /// Injectable stage runners.
    pub runners: PipelineStageRunners,
    /// Per-object resolved configs, keyed by `ObjectMesh.id`.
    ///
    /// Produced by [`crate::resolve_per_object_configs`] from the user-supplied CLI
    /// config source.  An empty map is valid (all objects fall back to
    /// `default_resolved_config`).
    pub resolved_configs: Arc<BTreeMap<String, ResolvedConfig>>,
    /// Global fallback [`ResolvedConfig`] used for objects not present in
    /// `resolved_configs` and passed as the default to the RegionMapping built-in.
    pub default_resolved_config: Arc<ResolvedConfig>,
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
}

/// Structured pipeline orchestration failures.
#[derive(Debug)]
pub enum PipelineError {
    /// Model loading failed.
    ModelLoad(String),
    /// PrePass stage execution failed.
    Prepass(PrepassExecutionError),
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
    let PipelineConfig {
        mesh_ir,
        mut plan,
        mut runners,
        resolved_configs,
        default_resolved_config,
    } = config;

    // Step 1: Create blackboard with the loaded mesh. Layer count is not known
    // yet — the execution plan is built before prepass runs, so global_layers
    // is always empty at this point. We pass 0 here; the blackboard's
    // layer_outputs slot-vec is not in the per-layer critical path (the layer
    // loop returns a Vec<LayerCollectionIR> directly), so this is safe.
    let mut blackboard = Blackboard::new(mesh_ir, 0);

    // Step 2: Execute prepass stages sequentially, collecting runtime audits.
    // Pass the resolved configs so the RegionMapping built-in can use them.
    // raw_config_source is empty here for backward compat (no paint overrides);
    // use run_pipeline_with_raw_config for production paint-override support.
    let empty_raw: HashMap<ConfigKey, ConfigValue> = HashMap::new();
    let prepass_audits = execute_prepass_with_builtins_configured(
        &plan,
        &mut blackboard,
        runners.prepass.as_ref(),
        &resolved_configs,
        &default_resolved_config,
        &empty_raw,
    )?;

    // Step 2b: Promote the LayerPlanIR committed by prepass into the execution
    // plan so that the per-layer loop iterates real layers. The plan is built
    // before prepass runs (global_layers = []) because the layer schedule is
    // determined by modules such as layer-planner-default during prepass itself.
    if let Some(layer_plan) = blackboard.layer_plan() {
        plan.global_layers = Arc::new(layer_plan.global_layers.clone());
    }

    // Step 3: Execute per-layer stages in parallel via rayon
    let (mut layer_irs, layer_audits) =
        execute_per_layer_with_events(&plan, &blackboard, runners.layer.as_ref(), sink)?;

    // Step 4: Execute layer finalization (if present)
    execute_layer_finalization(
        &plan,
        &blackboard,
        runners.finalization.as_ref(),
        &mut layer_irs,
    )?;

    // Step 5: Execute postpass (emit + serialize gcode)
    let (gcode_text, postpass_audits) = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        runners.emitter.as_ref(),
        runners.serializer.as_ref(),
        runners.postpass.as_mut(),
    )?;

    Ok(PipelineOutput {
        gcode_text,
        prepass_audits,
        layer_audits,
        postpass_audits,
    })
}

/// Execute the full slicing pipeline with paint-semantic config overrides.
///
/// Identical to [`run_pipeline_with_events`] except `raw_config_source` is
/// forwarded to the RegionMapping built-in so that `paint_config:<semantic>:*`
/// keys in the user-supplied config are applied as per-semantic overlays
/// (AC-4 / production path for MMU paint overrides).
pub fn run_pipeline_with_raw_config(
    config: PipelineConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    sink: &(dyn LayerProgressSink + Sync),
) -> Result<PipelineOutput, PipelineError> {
    let PipelineConfig {
        mesh_ir,
        mut plan,
        mut runners,
        resolved_configs,
        default_resolved_config,
    } = config;

    let mut blackboard = Blackboard::new(mesh_ir, 0);

    let prepass_audits = execute_prepass_with_builtins_configured(
        &plan,
        &mut blackboard,
        runners.prepass.as_ref(),
        &resolved_configs,
        &default_resolved_config,
        raw_config_source,
    )?;

    if let Some(layer_plan) = blackboard.layer_plan() {
        plan.global_layers = Arc::new(layer_plan.global_layers.clone());
    }

    let (mut layer_irs, layer_audits) =
        execute_per_layer_with_events(&plan, &blackboard, runners.layer.as_ref(), sink)?;

    execute_layer_finalization(
        &plan,
        &blackboard,
        runners.finalization.as_ref(),
        &mut layer_irs,
    )?;

    let (gcode_text, postpass_audits) = run_postpass_with_thumbnail(
        &plan,
        &blackboard,
        &mut runners,
        raw_config_source,
        &default_resolved_config,
        &layer_irs,
    )?;

    Ok(PipelineOutput {
        gcode_text,
        prepass_audits,
        layer_audits,
        postpass_audits,
    })
}

/// Execute the full slicing pipeline with bracket-shaped instrumentation
/// surfaced through `instrumentation`. Identical semantics to
/// [`run_pipeline_with_raw_config`] when `&NoopInstrumentation` is passed.
///
/// The instrumentation receives:
/// - `on_phase_start/end` for `Phase::PrePass`, `Phase::PerLayer`, `Phase::PostPass`
/// - `record_edges` once per stage at plan freeze, for every prepass /
///   per-layer / postpass stage, carrying the serial-edge reasons derived
///   from the compiled IR access masks (see
///   [`compute_serial_edges_from_compiled`] for the v1 reason coverage).
/// - Per-layer / per-stage / per-module brackets *inside* the per-layer tier
///   only (from `execute_per_layer_with_instrumentation`). Prepass and
///   postpass currently surface only their phase brackets; finer-grained
///   hooks inside those phases are a follow-up.
pub fn run_pipeline_with_instrumentation(
    config: PipelineConfig,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    sink: &(dyn LayerProgressSink + Sync),
    instrumentation: &(dyn PipelineInstrumentation + Sync),
) -> Result<PipelineOutput, PipelineError> {
    let PipelineConfig {
        mesh_ir,
        mut plan,
        mut runners,
        resolved_configs,
        default_resolved_config,
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

    instrumentation.on_phase_start(Phase::PrePass);
    let prepass_audits = execute_prepass_with_builtins_configured(
        &plan,
        &mut blackboard,
        runners.prepass.as_ref(),
        &resolved_configs,
        &default_resolved_config,
        raw_config_source,
    );
    instrumentation.on_phase_end(Phase::PrePass);
    let prepass_audits = prepass_audits?;

    if let Some(layer_plan) = blackboard.layer_plan() {
        plan.global_layers = Arc::new(layer_plan.global_layers.clone());
    }

    instrumentation.on_phase_start(Phase::PerLayer);
    let per_layer_result = execute_per_layer_with_instrumentation(
        &plan,
        &blackboard,
        runners.layer.as_ref(),
        sink,
        instrumentation,
    );
    instrumentation.on_phase_end(Phase::PerLayer);
    let (mut layer_irs, layer_audits) = per_layer_result?;

    instrumentation.on_phase_start(Phase::PostPass);
    let post_result = (|| -> Result<(String, Vec<ModuleAccessAudit>), PipelineError> {
        execute_layer_finalization(
            &plan,
            &blackboard,
            runners.finalization.as_ref(),
            &mut layer_irs,
        )?;

        run_postpass_with_thumbnail(
            &plan,
            &blackboard,
            &mut runners,
            raw_config_source,
            &default_resolved_config,
            &layer_irs,
        )
    })();
    instrumentation.on_phase_end(Phase::PostPass);
    let (gcode_text, postpass_audits) = post_result?;

    Ok(PipelineOutput {
        gcode_text,
        prepass_audits,
        layer_audits,
        postpass_audits,
    })
}

/// PNG file signature — first 8 bytes of every valid PNG.
const PNG_MAGIC: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// Shared post-finalization helper: validate the optional thumbnail file, build
/// the effective config map, wrap the serializer, and run `execute_postpass`.
///
/// Extracted to eliminate the identical block that previously lived in both
/// [`run_pipeline_with_raw_config`] and [`run_pipeline_with_instrumentation`].
fn run_postpass_with_thumbnail(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runners: &mut PipelineStageRunners,
    raw_config_source: &HashMap<ConfigKey, ConfigValue>,
    default_resolved_config: &ResolvedConfig,
    layer_irs: &[LayerCollectionIR],
) -> Result<(String, Vec<ModuleAccessAudit>), PipelineError> {
    // Extract and validate thumbnail bytes from raw_config before serialization.
    // If thumbnail_path is non-empty, read the file and check PNG magic; fail fast on error.
    let thumbnail_bytes: Option<Vec<u8>> = match raw_config_source.get("thumbnail_path") {
        Some(ConfigValue::String(path)) if !path.is_empty() => {
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

    // Build the effective config map: resolved defaults as baseline, then overlay
    // the user-supplied raw config (raw values take precedence).
    // This ensures CONFIG_BLOCK is non-empty even when raw_config_source is empty
    // (AC-9 / NEG-4) while still including all user-passed keys (AC-8).
    // thumbnail_path is an invocation-time routing key consumed above; strip it
    // so it does not appear in CONFIG_BLOCK.
    let mut effective_config = resolved_config_to_map(default_resolved_config);
    for (k, v) in raw_config_source {
        effective_config.insert(k.clone(), v.clone());
    }
    effective_config.remove("thumbnail_path");

    // Wrap the serializer with thumbnail support when bytes are present.
    let inner_serializer = std::mem::replace(
        &mut runners.serializer,
        Box::new(crate::gcode_emit::DefaultGCodeSerializer::new()),
    );
    runners.serializer = Box::new(ThumbnailAwareSerializer::new(
        inner_serializer,
        thumbnail_bytes,
        effective_config,
    ));

    execute_postpass(
        plan,
        layer_irs,
        blackboard,
        runners.emitter.as_ref(),
        runners.serializer.as_ref(),
        runners.postpass.as_mut(),
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
