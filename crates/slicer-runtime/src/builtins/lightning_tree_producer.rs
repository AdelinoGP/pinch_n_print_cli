//! BuiltinProducer wrapper for lightning tree generation.

use std::sync::{Arc, OnceLock};

use slicer_ir::SemVer;

use crate::blackboard::Blackboard;
use crate::dag::BuiltinProducer;
use slicer_ir::BlackboardError;

/// `BuiltinProducer` for the host-side `PrePass::LightningTreeGen` step.
///
/// The producer is **skipped** (no commit) when no region's
/// `sparse_fill_holder` resolves to `lightning-infill` (see ADR-0029). The
/// skip check is performed by the prepass wiring at
/// `crates/slicer-runtime/src/prepass.rs` before this producer's commit fn
/// is ever called.
pub static LIGHTNING_TREE_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:lightning_tree",
    stage: "PrePass::LightningTreeGen",
    ir_writes: &["LightningTreeIR"],
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

/// Commit `LightningTreeIR` to the blackboard.
///
/// The 137 contract returns the empty-but-valid `LightningTreeIR` produced
/// by the algorithm skeleton (`generate_lightning_trees` in
/// `crates/slicer-core/src/algos/lightning/mod.rs`); 139 wires the real
/// generator. The skip-when-no-lightning-holder guard is enforced upstream
/// in the prepass wiring; this fn assumes the predicate already passed.
pub fn commit_lightning_tree_ir_builtin(
    blackboard: &mut Blackboard,
) -> Result<(), BlackboardError> {
    let ir = slicer_core::algos::lightning::generate_lightning_trees()
        .unwrap_or_else(|_| Arc::new(slicer_ir::LightningTreeIR::default()));
    blackboard.commit_lightning_tree_ir(ir)
}
