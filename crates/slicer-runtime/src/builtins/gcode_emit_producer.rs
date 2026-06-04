//! BuiltinProducer wrapper for G-code emission.
//!
//! Per ADR-0001 (P86), the pure `LayerCollectionIR` → `GCodeIR` transformation
//! lives in `slicer-gcode` (`DefaultGCodeEmitter` + `GCodeEmitter` trait). This
//! thin wrapper retains the scheduler-visible descriptor in `slicer-runtime`
//! because `BuiltinProducer` references the runtime's DAG/Producer surface.
//!
//! The descriptor fields (`id`, `stage`, `ir_writes`, `claims_holds`, schema
//! window) are copied verbatim from the pre-P86 `gcode_emit.rs` definition so
//! scheduler topology is bit-for-bit unchanged.

use std::sync::OnceLock;

use slicer_ir::SemVer;

use crate::dag::BuiltinProducer;

/// `BuiltinProducer` for the host-side `PostPass::GCodeEmit` step.
pub static GCODE_EMIT_PRODUCER: BuiltinProducer = BuiltinProducer {
    id: "host:gcode_emit",
    stage: "PostPass::GCodeEmit",
    ir_writes: &["GCodeIR"],
    ir_reads: &[],
    claims_holds: &["host:gcode_emit"],
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
