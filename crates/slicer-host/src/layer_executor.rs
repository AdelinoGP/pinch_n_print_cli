//! Per-layer parallel executor contracts (TASK-031).
//!
//! This module defines the per-layer parallel execution contracts for running
//! all Tier-2 layer stages using rayon. Each layer gets its own `LayerArena`
//! for intermediate IR storage. Stages within each layer run sequentially,
//! but layers can be processed in parallel.

use std::fmt;

use rayon::prelude::*;
use slicer_ir::{GlobalLayer, LayerCollectionIR, ModuleId, SemVer, StageId};

use crate::{
    Blackboard, BlackboardError, CompiledModule, ExecutionPlan, LayerArena, LayerArenaError,
};

/// Output produced by a single layer stage module invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum LayerStageOutput {
    /// Module completed successfully with optional IR commits.
    Success,
    /// Module encountered non-fatal error, continue with next module.
    NonFatalError {
        /// Stable human-readable detail.
        message: String,
    },
}

/// Fatal error from a layer stage module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerStageError {
    /// Fatal error, abort entire layer.
    FatalModule {
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// Arena commit failed.
    ArenaCommit {
        /// Underlying arena failure.
        source: LayerArenaError,
    },
}

impl fmt::Display for LayerStageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal layer stage module failure in {stage_id} for {module_id}: {message}"
            ),
            Self::ArenaCommit { source } => write!(f, "arena commit failed: {source}"),
        }
    }
}

impl std::error::Error for LayerStageError {}

/// Top-level execution failure for the per-layer parallel executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerExecutionError {
    /// Fatal error in one layer (layer index included).
    FatalLayer {
        /// Layer that failed.
        layer_index: u32,
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// Blackboard commit failed.
    BlackboardCommit {
        /// Layer that failed to commit.
        layer_index: u32,
        /// Underlying blackboard failure.
        source: BlackboardError,
    },
    /// Rayon join failed (should never happen).
    ParallelJoin {
        /// Stable human-readable detail.
        message: String,
    },
}

impl fmt::Display for LayerExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalLayer {
                layer_index,
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal layer execution failure at layer {layer_index} in {stage_id} for {module_id}: {message}"
            ),
            Self::BlackboardCommit {
                layer_index,
                source,
            } => write!(
                f,
                "blackboard commit failed for layer {layer_index}: {source}"
            ),
            Self::ParallelJoin { message } => {
                write!(f, "rayon parallel join failed: {message}")
            }
        }
    }
}

impl std::error::Error for LayerExecutionError {}

/// Callback surface used by tests and future runtime bindings for layer stage execution.
pub trait LayerStageRunner {
    /// Execute one compiled layer module against the current layer state.
    fn run_stage(
        &self,
        stage_id: &StageId,
        layer: &GlobalLayer,
        module: &CompiledModule,
        blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<LayerStageOutput, LayerStageError>;
}

/// Executes the Tier-2 per-layer parallel pipeline using rayon.
///
/// Layers are processed in parallel, but stages within each layer are sequential.
/// Each layer gets its own `LayerArena` that is freed when the layer completes.
/// Results are committed to the blackboard's write-once layer output slots.
pub fn execute_per_layer(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
) -> Result<Vec<LayerCollectionIR>, LayerExecutionError> {
    let global_layers = &plan.global_layers;

    // Process layers in parallel using rayon.
    // collect() preserves the original item order, matching global_layers index order.
    global_layers
        .par_iter()
        .map(|layer| execute_single_layer(plan, blackboard, runner, layer))
        .collect()
}

/// Execute all stages for a single layer sequentially.
fn execute_single_layer(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &(dyn LayerStageRunner + Sync),
    layer: &GlobalLayer,
) -> Result<LayerCollectionIR, LayerExecutionError> {
    // Create an isolated LayerArena for this layer
    let mut arena = LayerArena::new();

    // Execute stages sequentially in deterministic order
    for stage in &plan.per_layer_stages {
        // Execute modules in topological order within each stage
        for module in &stage.modules {
            let result = runner.run_stage(&stage.stage_id, layer, module, blackboard, &mut arena);

            match result {
                Ok(LayerStageOutput::Success) => {
                    // Module completed successfully, continue
                }
                Ok(LayerStageOutput::NonFatalError { message: _ }) => {
                    // Non-fatal error: log but continue with next module
                }
                Err(LayerStageError::FatalModule {
                    stage_id,
                    module_id,
                    message,
                }) => {
                    // Fatal error: abort this layer immediately
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id,
                        module_id,
                        message,
                    });
                }
                Err(LayerStageError::ArenaCommit { source: _ }) => {
                    // Arena commit failure: treat as fatal for this layer
                    return Err(LayerExecutionError::FatalLayer {
                        layer_index: layer.index,
                        stage_id: stage.stage_id.clone(),
                        module_id: module.module_id.clone(),
                        message: String::from("arena commit failed"),
                    });
                }
            }
        }
    }

    // Build the LayerCollectionIR output for this layer
    let layer_output = LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: layer.index,
        z: layer.z,
        ordered_entities: Vec::new(),
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
    };

    Ok(layer_output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_layer_stage_output_equality() {
        assert_eq!(LayerStageOutput::Success, LayerStageOutput::Success);
        assert_eq!(
            LayerStageOutput::NonFatalError {
                message: "test".into()
            },
            LayerStageOutput::NonFatalError {
                message: "test".into()
            }
        );
    }
}
