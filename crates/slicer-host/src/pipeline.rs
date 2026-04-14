//! Pipeline orchestration for the slicer-host binary (TASK-075).
//!
//! This module provides the `run_pipeline` function that orchestrates the full
//! slicing pipeline: prepass → per-layer → finalization → postpass → gcode output.
//! All stage runners are injectable via traits for testability.

use std::fmt;
use std::sync::Arc;

use slicer_ir::MeshIR;

use crate::{
    execute_layer_finalization, execute_per_layer, execute_postpass, execute_prepass_with_builtins,
    Blackboard, ExecutionPlan, FinalizationError, FinalizationStageRunner, GCodeEmitter,
    GCodeSerializer, LayerExecutionError, LayerStageRunner, PostpassError, PostpassStageRunner,
    PrepassExecutionError, PrepassStageRunner,
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
}

/// Output produced by a successful pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineOutput {
    /// The final G-code text.
    pub gcode_text: String,
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
    let PipelineConfig {
        mesh_ir,
        plan,
        runners,
    } = config;

    // Step 1: Create blackboard with loaded mesh and layer count
    let layer_count = plan.global_layers.len();
    let mut blackboard = Blackboard::new(mesh_ir, layer_count);

    // Step 2: Execute prepass stages sequentially
    execute_prepass_with_builtins(&plan, &mut blackboard, runners.prepass.as_ref())?;

    // Step 3: Execute per-layer stages in parallel via rayon
    let mut layer_irs = execute_per_layer(&plan, &blackboard, runners.layer.as_ref())?;

    // Step 4: Execute layer finalization (if present)
    execute_layer_finalization(
        &plan,
        &blackboard,
        runners.finalization.as_ref(),
        &mut layer_irs,
    )?;

    // Step 5: Execute postpass (emit + serialize gcode)
    let gcode_text = execute_postpass(
        &plan,
        &layer_irs,
        &blackboard,
        runners.emitter.as_ref(),
        runners.serializer.as_ref(),
        runners.postpass.as_ref(),
    )?;

    Ok(PipelineOutput { gcode_text })
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
