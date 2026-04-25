//! PrePass execution contracts.

use std::fmt;
use std::sync::Arc;

use slicer_ir::{
    LayerPlanIR, MeshSegmentationIR, ModuleId, PaintRegionIR, RegionMapIR, SeamPlanIR,
    StageId, SupportPlanIR, SurfaceClassificationIR,
};

use crate::mesh_analysis::{execute_mesh_analysis, MeshAnalysisError};
use crate::region_mapping::{commit_region_mapping_builtin, RegionMappingBuiltinError};
use crate::validation::ModuleAccessAudit;
use crate::{Blackboard, BlackboardError, BlackboardPrepassSlot, CompiledModule, ExecutionPlan};

/// One committed output produced by a prepass stage invocation.
#[derive(Debug, Clone, PartialEq)]
pub enum PrepassStageOutput {
    /// Stage produced no blackboard commit.
    None,
    /// Stage produced `SurfaceClassificationIR`.
    SurfaceClassification(Arc<SurfaceClassificationIR>),
    /// Stage produced `MeshSegmentationIR`.
    MeshSegmentation(Arc<MeshSegmentationIR>),
    /// Stage produced `LayerPlanIR`.
    LayerPlan(Arc<LayerPlanIR>),
    /// Stage produced `SeamPlanIR`.
    SeamPlan(Arc<SeamPlanIR>),
    /// Stage produced `SupportPlanIR`.
    SupportPlan(Arc<SupportPlanIR>),
    /// Stage produced `PaintRegionIR`.
    PaintRegions(Arc<PaintRegionIR>),
    /// Stage produced `RegionMapIR`.
    RegionMap(Arc<RegionMapIR>),
    /// Guest-emitted mesh-analysis pushes collected via the
    /// `mesh-analysis-output` WIT resource. This variant carries the raw
    /// `(object_id, FacetAnnotation)` / `(object_id, SurfaceGroupProposal)`
    /// pairs the macro-path drain forwarded from the SDK builder; it does
    /// **not** commit to the blackboard because
    /// `SurfaceClassificationIR` is still owned by the host built-in
    /// (`mesh_analysis::execute_mesh_analysis`). The variant exists to
    /// let the prepass dispatcher surface the drained output so tests and
    /// future consumers can observe what reached the host.
    MeshAnalysisAuxiliary(Arc<MeshAnalysisAuxiliary>),
}

/// Raw mesh-analysis output drained from a guest's
/// `mesh-analysis-output` WIT resource. Insertion order is preserved
/// exactly as the guest pushed, so downstream consumers can rely on
/// deterministic sequencing.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshAnalysisAuxiliary {
    /// Per-object facet annotations in push order.
    pub facet_annotations: Vec<(String, FacetAnnotationRecord)>,
    /// Per-object surface-group proposals in push order.
    pub surface_groups: Vec<(String, SurfaceGroupRecord)>,
}

/// Host-side mirror of the WIT `facet-annotation` record, decoupled
/// from the wit-bindgen-generated types so the `PrepassStageOutput`
/// enum does not depend on the generated module.
#[derive(Debug, Clone, PartialEq)]
pub struct FacetAnnotationRecord {
    /// Triangle index in the object's mesh.
    pub facet_index: u32,
    /// Slope angle of the facet normal in degrees.
    pub slope_angle_deg: f32,
    /// Classification label.
    pub classification: FacetClassRecord,
}

/// Host-side mirror of the WIT `facet-class` enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacetClassRecord {
    /// No special classification.
    Normal,
    /// Nearly-horizontal surface (top/bottom candidate).
    NearHorizontal,
    /// Facet that overhangs printed material below.
    Overhang,
    /// Bridge-suitable facet (horizontal span).
    Bridge,
    /// Top-facing surface.
    TopSurface,
    /// Bottom-facing surface.
    BottomSurface,
}

/// Host-side mirror of the WIT `surface-group-proposal` record.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceGroupRecord {
    /// Facet indices belonging to the group.
    pub facet_indices: Vec<u32>,
    /// Minimum Z coordinate of the group in world space (mm).
    pub z_min: f32,
    /// Maximum Z coordinate of the group in world space (mm).
    pub z_max: f32,
    /// Number of shells (perimeter loops) to emit around the group.
    pub shell_count: u32,
}

/// Callback surface used by tests and future runtime bindings.
pub trait PrepassStageRunner {
    /// Execute one compiled prepass module against the current blackboard state.
    ///
    /// Returns both the stage output and the runtime IR read paths collected
    /// by the WIT view methods during this call. The returned `runtime_reads`
    /// is used to populate `ModuleAccessAudit.runtime_reads` for audit
    /// construction in `execute_prepass`.
    fn run_stage(
        &self,
        stage_id: &StageId,
        module: &CompiledModule,
        blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError>;
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
///
/// Returns collected runtime access audits for all user modules that executed.
/// Host built-ins (MeshAnalysis, RegionMapping) are not audited as they are
/// not subject to the module access contract.
pub fn execute_prepass(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
) -> Result<Vec<ModuleAccessAudit>, PrepassExecutionError> {
    let mut audits = Vec::new();

    for stage in &plan.prepass_stages {
        ensure_stage_prerequisites(&stage.stage_id, blackboard)?;

        for module in &stage.modules {
            let (output, runtime_reads) = runner.run_stage(&stage.stage_id, module, blackboard)?;

            // Determine IR path before committing (output is moved into commit).
            let ir_path = ir_path_for_prepass_output(&output);

            commit_stage_output(&stage.stage_id, &module.module_id, blackboard, output)?;

            // Record runtime audit if the module produced output.
            // Always record the audit when there is a runtime_reads vector,
            // even if the output is None (read-performing modules that produce
            // no IR output still have their reads audited).
            if let Some(ir_path) = ir_path {
                audits.push(ModuleAccessAudit {
                    module_id: module.module_id.clone(),
                    runtime_reads,
                    runtime_writes: vec![ir_path],
                });
            } else if !runtime_reads.is_empty() {
                // Module performed reads but produced no output — still record audit.
                audits.push(ModuleAccessAudit {
                    module_id: module.module_id.clone(),
                    runtime_reads,
                    runtime_writes: Vec::new(),
                });
            }
        }
    }

    Ok(audits)
}

/// Maps a prepass stage output variant to the canonical IR field path written.
fn ir_path_for_prepass_output(output: &PrepassStageOutput) -> Option<String> {
    match output {
        PrepassStageOutput::None => None,
        PrepassStageOutput::SurfaceClassification(_) => {
            Some(String::from("SurfaceClassificationIR"))
        }
        PrepassStageOutput::MeshSegmentation(_) => Some(String::from("MeshSegmentationIR")),
        PrepassStageOutput::LayerPlan(_) => Some(String::from("LayerPlanIR")),
        PrepassStageOutput::SeamPlan(_) => Some(String::from("SeamPlanIR")),
        PrepassStageOutput::SupportPlan(_) => Some(String::from("SupportPlanIR")),
        PrepassStageOutput::PaintRegions(_) => Some(String::from("PaintRegionIR")),
        PrepassStageOutput::RegionMap(_) => Some(String::from("RegionMapIR")),
        // MeshAnalysisAuxiliary is auxiliary data, not a primary IR commit.
        PrepassStageOutput::MeshAnalysisAuxiliary(_) => None,
    }
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
///
/// Returns collected runtime access audits from user prepass modules.
/// Host built-ins (MeshAnalysis, RegionMapping) are not audited as they are
/// not subject to the module access contract.
pub fn execute_prepass_with_builtins(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    runner: &dyn PrepassStageRunner,
) -> Result<Vec<ModuleAccessAudit>, PrepassExecutionError> {
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
    let audits = execute_prepass(plan, blackboard, runner)?;

    // Host-built-in PrePass::RegionMapping runs last (docs/04 §Full
    // Lifecycle), after any user PrePass::LayerPlanning module has
    // committed the layer plan. Skipped if a LayerPlanIR was never
    // committed (e.g. empty prepass in unit tests) or if region_map is
    // already present.
    if blackboard.layer_plan().is_some() && blackboard.region_map().is_none() {
        commit_region_mapping_builtin(plan, blackboard)
            .map_err(|source| PrepassExecutionError::RegionMapping { source })?;
    }
    Ok(audits)
}

/// Ensures all prerequisite IR artifacts are present on the blackboard
/// before a prepass stage is executed.
pub fn ensure_stage_prerequisites(
    stage_id: &StageId,
    blackboard: &Blackboard,
) -> Result<(), PrepassExecutionError> {
    for &slot in required_slots(stage_id) {
        let present = match slot {
            BlackboardPrepassSlot::SurfaceClassification => {
                blackboard.surface_classification().is_some()
            }
            BlackboardPrepassSlot::MeshSegmentation => {
                blackboard.mesh_segmentation().is_some()
            }
            BlackboardPrepassSlot::LayerPlan => blackboard.layer_plan().is_some(),
            BlackboardPrepassSlot::PaintRegions => blackboard.paint_regions().is_some(),
            BlackboardPrepassSlot::RegionMap => blackboard.region_map().is_some(),
            BlackboardPrepassSlot::SeamPlan => blackboard.seam_plan().is_some(),
            BlackboardPrepassSlot::SupportPlan => blackboard.support_plan().is_some(),
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
        "PrePass::SeamPlanning" => &[BlackboardPrepassSlot::LayerPlan],
        "PrePass::SupportGeneration" => &[
            BlackboardPrepassSlot::SurfaceClassification,
            BlackboardPrepassSlot::LayerPlan,
        ],
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
        PrepassStageOutput::MeshSegmentation(ir) => blackboard.commit_mesh_segmentation(ir),
        PrepassStageOutput::LayerPlan(ir) => blackboard.commit_layer_plan(ir),
        PrepassStageOutput::SeamPlan(ir) => blackboard.commit_seam_plan(ir),
        PrepassStageOutput::SupportPlan(ir) => blackboard.commit_support_plan(ir),
        PrepassStageOutput::PaintRegions(ir) => blackboard.commit_paint_regions(ir),
        PrepassStageOutput::RegionMap(ir) => blackboard.commit_region_map(ir),
        // Mesh-analysis auxiliary pushes are surfaced for observability
        // but do not commit to the blackboard. The production
        // SurfaceClassificationIR slot is still owned by the host
        // built-in (`mesh_analysis::execute_mesh_analysis`); letting a
        // guest overwrite it here would require a conflict-resolution
        // design that is deliberately out of scope for STEP G.
        PrepassStageOutput::MeshAnalysisAuxiliary(_) => Ok(()),
    };

    result.map_err(|source| PrepassExecutionError::Blackboard {
        stage_id: stage_id.clone(),
        module_id: module_id.clone(),
        source,
    })
}
