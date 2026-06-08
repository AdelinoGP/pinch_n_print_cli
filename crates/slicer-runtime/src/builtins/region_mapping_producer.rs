//! BuiltinProducer wrapper for the host-side `PrePass::RegionMapping` step.
//!
//! The pure kernel lives in `slicer_core::algos::region_mapping`. This thin
//! wrapper holds the scheduler-visible `BuiltinProducer` descriptor and the
//! `commit_region_mapping_builtin` function that bridges from `ExecutionPlan`
//! + `Blackboard` to the IR-only kernel.

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock};

use slicer_ir::{PaintSemantic, ResolvedConfig, SemVer};

use crate::dag::BuiltinProducer;
use crate::{Blackboard, BlackboardError, ExecutionPlan};

pub use slicer_core::algos::region_mapping::RegionMappingError;
pub use slicer_core::algos::region_mapping::DEFAULT_REGION_MAP_CAP;

/// `BuiltinProducer` for the host-side `PrePass::RegionMapping` step.
pub static REGION_MAPPING_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:region_mapping",
    stage: "PrePass::RegionMapping",
    ir_writes: &["RegionMapIR"],
    ir_reads: &[],
    claims_holds: &[],
    claims_requires: &[],
    requires_modules: &[],
    min_ir_schema: SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    },
    max_ir_schema: SemVer {
        major: 5,
        minor: 0,
        patch: 0,
    },
    _cache_ir_writes: OnceLock::new(),
    _cache_ir_reads: OnceLock::new(),
    _cache_claims_holds: OnceLock::new(),
    _cache_claims_requires: OnceLock::new(),
    _cache_requires_modules: OnceLock::new(),
};

/// Wrapper error used when the built-in runs on the real prepass path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionMappingBuiltinError {
    /// No `LayerPlanIR` committed to the blackboard yet.
    MissingLayerPlan,
    /// Region mapping itself failed.
    Mapping(RegionMappingError),
    /// Blackboard commit failed (e.g. duplicate commit).
    Blackboard {
        /// Underlying blackboard failure.
        source: BlackboardError,
    },
}

impl std::fmt::Display for RegionMappingBuiltinError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingLayerPlan => write!(
                f,
                "built-in PrePass::RegionMapping requires a committed LayerPlanIR"
            ),
            Self::Mapping(e) => write!(f, "built-in PrePass::RegionMapping failed: {e}"),
            Self::Blackboard { source } => {
                write!(f, "built-in PrePass::RegionMapping commit failed: {source}")
            }
        }
    }
}

impl std::error::Error for RegionMappingBuiltinError {}

/// Commit the built-in region map into the blackboard (idempotent).
pub fn commit_region_mapping_builtin(
    plan: &ExecutionPlan,
    blackboard: &mut Blackboard,
    resolved_configs: &BTreeMap<String, ResolvedConfig>,
    default_resolved_config: &ResolvedConfig,
    paint_semantic_configs: &BTreeMap<PaintSemantic, ResolvedConfig>,
) -> Result<(), RegionMappingBuiltinError> {
    if blackboard.region_map().is_some() {
        return Ok(());
    }
    let Some(layer_plan) = blackboard.layer_plan().cloned() else {
        return Err(RegionMappingBuiltinError::MissingLayerPlan);
    };
    let mesh_arc = Arc::clone(blackboard.mesh());
    let paint_regions = blackboard.paint_regions().map(|arc| arc.as_ref());

    // Precompute stage invocations from the scheduler plan.
    let stage_invocations: Vec<(slicer_ir::StageId, Vec<slicer_ir::ModuleInvocation>)> = plan
        .per_layer_stages
        .iter()
        .chain(plan.postpass_stages.iter())
        .map(|stage| {
            let invocations = stage
                .modules
                .iter()
                .map(|m| slicer_ir::ModuleInvocation {
                    module_id: m.module_id().to_owned(),
                    config_view: m.config_view().as_ref().clone(),
                })
                .collect::<Vec<_>>();
            (stage.stage_id.clone(), invocations)
        })
        .collect();
    let projection = slicer_core::algos::region_mapping::RegionMappingPlanProjection {
        stage_invocations: &stage_invocations,
    };

    let ir = slicer_core::algos::region_mapping::execute_region_mapping_inner(
        layer_plan.as_ref(),
        &projection,
        paint_regions,
        paint_semantic_configs,
        &mesh_arc.objects,
        Some((resolved_configs, default_resolved_config)),
        DEFAULT_REGION_MAP_CAP,
    )
    .map_err(RegionMappingBuiltinError::Mapping)?;

    blackboard
        .commit_region_map(Arc::new(ir))
        .map_err(|source| RegionMappingBuiltinError::Blackboard { source })?;
    Ok(())
}
