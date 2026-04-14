//! PrePass execution contracts.

use std::fmt;
use std::sync::Arc;

use slicer_ir::{
    LayerPlanIR, ModuleId, PaintRegionIR, RegionMapIR, StageId, SurfaceClassificationIR,
};

use crate::mesh_analysis::{execute_mesh_analysis, MeshAnalysisError};
use crate::region_mapping::{commit_region_mapping_builtin, RegionMappingBuiltinError};
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
    /// The host-built-in `PrePass::MeshAnalysis` stage failed.
    MeshAnalysis {
        /// Underlying mesh-analysis failure.
        source: MeshAnalysisError,
    },
    /// The host-built-in `PrePass::RegionMapping` stage failed.
    RegionMapping {
        /// Underlying region-mapping failure.
        source: RegionMappingBuiltinError,
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
            Self::MeshAnalysis { source } => {
                write!(f, "built-in PrePass::MeshAnalysis failed: {source}")
            }
            Self::RegionMapping { source } => {
                write!(f, "built-in PrePass::RegionMapping failed: {source}")
            }
        }
    }
}

impl std::error::Error for PrepassExecutionError {}

/// Executes the sequential Tier 1 prepass pipeline.
pub fn execute_prepass(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
) -> Result<(), PrepassExecutionError> {
    for stage in &plan.prepass_stages {
        ensure_stage_prerequisites(&stage.stage_id, blackboard)?;

        for module in &stage.modules {
            let output = runner.run_stage(&stage.stage_id, module, blackboard)?;
            commit_stage_output(&stage.stage_id, &module.module_id, blackboard, output)?;
        }
    }

    Ok(())
}

/// Run the host-built-in [`PrePass::MeshAnalysis`](execute_mesh_analysis)
/// stage and then [`execute_prepass`].
///
/// This is the prepass entry-point used by the real pipeline (docs/04
/// §Full Lifecycle — prepass block): the built-in commits
/// `SurfaceClassificationIR` into the blackboard before any user prepass
/// module runs. If a caller has already committed a surface
/// classification (e.g. an earlier integration test pre-seeded one) the
/// built-in step is skipped so commits remain exactly-once.
pub fn execute_prepass_with_builtins(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
) -> Result<(), PrepassExecutionError> {
    if blackboard.surface_classification().is_none() {
        let ir = execute_mesh_analysis(blackboard.mesh().as_ref())
            .map_err(|source| PrepassExecutionError::MeshAnalysis { source })?;
        blackboard
            .commit_surface_classification(std::sync::Arc::new(ir))
            .map_err(|source| PrepassExecutionError::Blackboard {
                stage_id: "PrePass::MeshAnalysis".to_string(),
                module_id: "<host-built-in>".to_string(),
                source,
            })?;
    }
    execute_prepass(plan, blackboard, runner)?;

    // Host-built-in PrePass::RegionMapping runs last (docs/04 §Full
    // Lifecycle), after any user PrePass::LayerPlanning module has
    // committed the layer plan. Skipped if a LayerPlanIR was never
    // committed (e.g. empty prepass in unit tests) or if region_map is
    // already present.
    if blackboard.layer_plan().is_some() && blackboard.region_map().is_none() {
        commit_region_mapping_builtin(plan, blackboard)
            .map_err(|source| PrepassExecutionError::RegionMapping { source })?;
    }
    Ok(())
}

fn ensure_stage_prerequisites(
    stage_id: &StageId,
    blackboard: &Blackboard,
) -> Result<(), PrepassExecutionError> {
    for &slot in required_slots(stage_id) {
        let present = match slot {
            BlackboardPrepassSlot::SurfaceClassification => {
                blackboard.surface_classification().is_some()
            }
            BlackboardPrepassSlot::LayerPlan => blackboard.layer_plan().is_some(),
            BlackboardPrepassSlot::PaintRegions => blackboard.paint_regions().is_some(),
            BlackboardPrepassSlot::RegionMap => blackboard.region_map().is_some(),
        };

        if !present {
            return Err(PrepassExecutionError::MissingRequiredPrepass {
                stage_id: stage_id.clone(),
                slot,
            });
        }
    }

    Ok(())
}

fn required_slots(stage_id: &StageId) -> &'static [BlackboardPrepassSlot] {
    match stage_id.as_str() {
        "PrePass::MeshAnalysis" => &[],
        "PrePass::LayerPlanning" => &[BlackboardPrepassSlot::SurfaceClassification],
        "PrePass::PaintSegmentation" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
        ],
        "PrePass::RegionMapping" => &[BlackboardPrepassSlot::LayerPlan],
        _ => &[],
    }
}

fn commit_stage_output(
    stage_id: &StageId,
    module_id: &ModuleId,
    blackboard: &mut Blackboard,
    output: PrepassStageOutput,
) -> Result<(), PrepassExecutionError> {
    let result = match output {
        PrepassStageOutput::None => Ok(()),
        PrepassStageOutput::SurfaceClassification(ir) => {
            blackboard.commit_surface_classification(ir)
        }
        PrepassStageOutput::LayerPlan(ir) => blackboard.commit_layer_plan(ir),
        PrepassStageOutput::PaintRegions(ir) => blackboard.commit_paint_regions(ir),
        PrepassStageOutput::RegionMap(ir) => blackboard.commit_region_map(ir),
    };

    result.map_err(|source| PrepassExecutionError::Blackboard {
        stage_id: stage_id.clone(),
        module_id: module_id.clone(),
        source,
    })
}
