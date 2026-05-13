//! Pipeline orchestration for the slicer-host binary (TASK-075).
//!
//! This module provides the `run_pipeline` function that orchestrates the full
//! slicing pipeline: prepass → per-layer → finalization → postpass → gcode output.
//! All stage runners are injectable via traits for testability.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use slicer_ir::{ConfigKey, ConfigValue, MeshIR, ResolvedConfig};

use crate::{
    execute_layer_finalization, execute_per_layer_with_events, execute_postpass,
    prepass::execute_prepass_with_builtins_configured, Blackboard, ExecutionPlan,
    FinalizationError, FinalizationStageRunner, GCodeEmitter, GCodeSerializer, LayerExecutionError,
    LayerProgressSink, LayerStageRunner, ModuleAccessAudit, NoopLayerProgressSink, PostpassError,
    PostpassStageRunner, PrepassExecutionError, PrepassStageRunner,
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
