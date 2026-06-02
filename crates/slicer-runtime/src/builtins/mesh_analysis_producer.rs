//! BuiltinProducer wrapper for mesh analysis.

use std::sync::OnceLock;

use slicer_ir::SemVer;

use crate::dag::BuiltinProducer;

/// `BuiltinProducer` for the host-side mesh-commit pseudo-module (`host:mesh`).
pub static MESH_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:mesh",
    stage: "PrePass::MeshAnalysis",
    ir_writes: &["MeshIR"],
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

/// `BuiltinProducer` for the host-side `PrePass::MeshAnalysis` step.
pub static MESH_ANALYSIS_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:mesh_analysis",
    stage: "PrePass::MeshAnalysis",
    ir_writes: &["SurfaceClassificationIR"],
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
