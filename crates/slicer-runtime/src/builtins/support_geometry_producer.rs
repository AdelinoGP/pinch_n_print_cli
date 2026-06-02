//! BuiltinProducer wrapper for support geometry.

use std::sync::{Arc, OnceLock};

use slicer_ir::SemVer;

use crate::blackboard::Blackboard;
use crate::dag::BuiltinProducer;

/// `BuiltinProducer` for the host-side `PrePass::SupportGeometry` step.
pub static SUPPORT_GEOMETRY_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:support_geometry",
    stage: "PrePass::SupportGeometry",
    ir_writes: &["SupportGeometryIR"],
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
        major: 4,
        minor: 0,
        patch: 0,
    },
    _cache_ir_writes: OnceLock::new(),
    _cache_ir_reads: OnceLock::new(),
    _cache_claims_holds: OnceLock::new(),
    _cache_claims_requires: OnceLock::new(),
    _cache_requires_modules: OnceLock::new(),
};

/// Commit `SupportGeometryIR` to the blackboard using default parameters.
pub fn commit_support_geometry_builtin(
    blackboard: &mut Blackboard,
) -> Result<(), slicer_core::algos::support_geometry::SupportGeometryBuiltinError> {
    let layer_plan = blackboard
        .layer_plan()
        .ok_or(slicer_core::algos::support_geometry::SupportGeometryBuiltinError::NoLayerPlan)?;
    let slice_vec = blackboard
        .slice_ir()
        .ok_or(slicer_core::algos::support_geometry::SupportGeometryBuiltinError::MissingSliceIR)?;

    let ir = slicer_core::algos::support_geometry::execute_support_geometry(
        layer_plan.as_ref(),
        slice_vec.as_ref(),
    )?;
    blackboard
        .commit_support_geometry(Arc::new(ir))
        .map_err(|_| slicer_core::algos::support_geometry::SupportGeometryBuiltinError::NoLayerPlan)
}
