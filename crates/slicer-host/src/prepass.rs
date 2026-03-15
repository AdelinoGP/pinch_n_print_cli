//! PrePass execution contracts.

use std::fmt;
use std::sync::Arc;

use slicer_ir::{
    LayerPlanIR, ModuleId, PaintRegionIR, RegionMapIR, StageId, SurfaceClassificationIR,
};

use crate::{Blackboard, BlackboardError, BlackboardPrepassSlot, CompiledModule, ExecutionPlan};

/// One committed output produced by a prepass stage invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum PrepassStageOutput {
    /// Stage produced no blackboard commit.
    None,
    /// Stage produced `SurfaceClassificationIR`.
    SurfaceClassification(Arc<SurfaceClassificationIR>),
    /// Stage produced `LayerPlanIR`.
    LayerPlan(Arc<LayerPlanIR>),
    /// Stage produced `PaintRegionIR`.
    PaintRegions(Arc<PaintRegionIR>),
    /// Stage produced `RegionMapIR`.
    RegionMap(Arc<RegionMapIR>),
}

/// Callback surface used by tests and future runtime bindings.
pub trait PrepassStageRunner {
    /// Execute one compiled prepass module against the current blackboard state.
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
    ) -> Result<PrepassStageOutput, PrepassExecutionError>;
}

/// Structured prepass executor failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepassExecutionError {
    /// A stage started before one of its required prepass inputs existed.
    MissingRequiredPrepass {
        /// Stage that required the missing input.
        stage_id: StageId,
        /// Missing blackboard slot.
        slot: BlackboardPrepassSlot,
    },
    /// A module returned a fatal error.
    FatalModule {
        /// Stage being executed.
        stage_id: StageId,
        /// Module that failed.
        module_id: ModuleId,
        /// Stable human-readable detail.
        message: String,
    },
    /// A blackboard commit contract failed.
    Blackboard {
        /// Stage being committed.
        stage_id: StageId,
        /// Module whose commit failed.
        module_id: ModuleId,
        /// Underlying blackboard failure.
        source: BlackboardError,
    },
}

impl fmt::Display for PrepassExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRequiredPrepass { stage_id, slot } => {
                write!(f, "stage {stage_id} requires committed prepass slot {slot}")
            }
            Self::FatalModule {
                stage_id,
                module_id,
                message,
            } => write!(
                f,
                "fatal prepass module failure in {stage_id} for {module_id}: {message}"
            ),
            Self::Blackboard {
                stage_id,
                module_id,
                source,
            } => write!(
                f,
                "blackboard commit failed in {stage_id} for {module_id}: {source}"
            ),
        }
    }
}

impl std::error::Error for PrepassExecutionError {}

/// Executes the sequential Tier 1 prepass pipeline.
pub fn execute_prepass(
    _plan: &ExecutionPlan,
    _blackboard: &mut Blackboard,
    _runner: &dyn PrepassStageRunner,
) -> Result<(), PrepassExecutionError> {
    todo!("TASK-027: implement deterministic prepass executor")
}
