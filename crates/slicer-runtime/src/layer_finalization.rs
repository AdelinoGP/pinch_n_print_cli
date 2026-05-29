//! PostPass::LayerFinalization stage executor (TASK-032).
//!
//! This module defines the executor for the finalization stage that runs
//! after all per-layer parallel execution has completed. Modules in this stage
//! receive mutable access to the entire vector of LayerCollectionIR objects,
//! allowing them to insert synthetic layers or modify existing ones.

use std::fmt;

use slicer_ir::{LayerCollectionIR, ModuleId, StageId};

use crate::{Blackboard, CompiledModule, ExecutionPlan};

/// Output produced by a single layer finalization module invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum FinalizationOutput {
    /// Module completed successfully.
    Success,
    /// Module encountered a non-fatal error, continue with next module.
    NonFatalError {
        /// Stable human-readable detail.
        message: String,
    },
}

/// Fatal error from a layer finalization module or executor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FinalizationError {
    /// Fatal error, abort finalization.
    FatalModule {
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// Validation error (e.g. non-monotonic layer indices).
    Validation {
        /// Stable human-readable detail.
        message: String,
    },
}

impl fmt::Display for FinalizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal finalization module failure in {stage_id} for {module_id}: {message}"
            ),
            Self::Validation { message } => write!(f, "finalization validation failed: {message}"),
        }
    }
}

impl std::error::Error for FinalizationError {}

/// Callback surface used by tests and future runtime bindings.
pub trait FinalizationStageRunner {
    /// Execute one compiled layer finalization module.
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
        layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError>;
}

/// Builder for finalization output.
#[derive(Debug)]
pub struct FinalizationOutputBuilder;

/// Executes the PostPass::LayerFinalization pipeline.
///
/// Modules run sequentially with a forced pool size of 1.
/// Output layers are validated to maintain monotonic indices.
pub fn execute_layer_finalization(
    plan: &ExecutionPlan,
    blackboard: &Blackboard,
    runner: &dyn FinalizationStageRunner,
    layers: &mut Vec<LayerCollectionIR>,
) -> Result<(), FinalizationError> {
    if let Some(stage) = &plan.layer_finalization_stage {
        for module in &stage.modules {
            runner.run_stage(&stage.stage_id, module, blackboard, layers)?;

            // Validate that the layer indices remain strictly monotonic
            for window in layers.windows(2) {
                if window[0].global_layer_index >= window[1].global_layer_index {
                    return Err(FinalizationError::Validation {
                        message: format!(
                            "layer indices must be strictly monotonic, found {} followed by {}",
                            window[0].global_layer_index, window[1].global_layer_index
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}
