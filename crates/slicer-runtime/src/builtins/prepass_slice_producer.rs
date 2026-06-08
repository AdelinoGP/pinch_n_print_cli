//! BuiltinProducer wrapper for pre-pass slicing.

use std::sync::{Arc, OnceLock};

use slicer_ir::SemVer;

use crate::blackboard::Blackboard;
use crate::dag::BuiltinProducer;

/// `BuiltinProducer` for the host-side `PrePass::Slice` step.
pub static SLICE_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:slice",
    stage: "PrePass::Slice",
    ir_writes: &["SliceIR"],
    ir_reads: &[],
    claims_holds: &["host:slice"],
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

/// `BuiltinProducer` for the host-side `PrePass::ShellClassification` step.
pub static SHELL_CLASSIFICATION_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:shell_classification",
    stage: "PrePass::ShellClassification",
    ir_writes: &["SliceIR"],
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

/// Whole-print wrapper: produces one `SliceIR` per `global_layer` in the
/// committed layer plan, in plan order. Reads `MeshIR`, `LayerPlanIR`,
/// `SurfaceClassificationIR`, and `RegionMapIR` (all immutable) from the
/// blackboard. Used by [`commit_slice_builtin`].
pub fn execute_prepass_slice_all_layers(
    blackboard: &Blackboard,
) -> Result<Vec<slicer_ir::SliceIR>, slicer_core::algos::prepass_slice::LayerSliceError> {
    let mesh = blackboard.mesh();
    let layer_plan = blackboard
        .layer_plan()
        .ok_or(slicer_core::algos::prepass_slice::LayerSliceError::MissingLayerPlan)?;
    let surface_class = blackboard.surface_classification().map(|a| a.as_ref());
    let region_map = blackboard.region_map().map(|a| a.as_ref());

    layer_plan
        .global_layers
        .iter()
        .map(|gl| {
            slicer_core::algos::prepass_slice::execute_prepass_slice_single_layer(
                mesh.as_ref(),
                gl,
                surface_class,
                region_map,
            )
        })
        .collect()
}

/// `PrePass::Slice` host built-in entry point. Computes the per-global-layer
/// `Vec<SliceIR>` from blackboard reads and commits it via
/// [`Blackboard::commit_slice_ir`].
pub fn commit_slice_builtin(
    blackboard: &mut Blackboard,
) -> Result<(), slicer_core::algos::prepass_slice::LayerSliceError> {
    let slices = execute_prepass_slice_all_layers(blackboard)?;
    blackboard.commit_slice_ir(Arc::new(slices))?;
    Ok(())
}
