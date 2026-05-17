//! Pipeline instrumentation surface.
//!
//! Provides a bracket-shaped trait that the scheduler calls at every phase,
//! stage, layer, and module boundary. The default `NoopInstrumentation` is
//! zero-cost; a real implementation (e.g. `slicer_report::Collector`) records
//! timing, memory, and DAG metadata for the HTML slicer report.
//!
//! This module also owns the report-domain types (`Phase`, `TierKind`,
//! `EdgeReason`, `SerialEdge`) and a helper that derives in-stage serial-edge
//! reasons from a stage's `LoadedModule` list without disturbing `dag.rs`.

use slicer_ir::{ModuleId, StageId};

use crate::execution_plan::CompiledModule;
use crate::manifest::LoadedModule;

/// Top-level execution phase. Layers are reported separately within
/// `Phase::PerLayer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Sequential pre-pass tier (MeshSegmentation, MeshAnalysis, …).
    PrePass,
    /// Per-layer tier executed in parallel across layers, sequential within.
    PerLayer,
    /// Sequential post-pass tier (LayerFinalization, GCodeEmit, …).
    PostPass,
}

/// Pipeline tier a stage belongs to. Used by the report to annotate stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierKind {
    /// Pre-pass: sequential by construction.
    PrePass,
    /// Per-layer: parallel across layers, sequential within each layer.
    PerLayer,
    /// Post-pass: sequential by construction.
    PostPass,
}

/// Reason a serial edge exists between two modules in the same stage.
///
/// Only the variants that the actual DAG builder emits are represented —
/// claim conflicts block plan validation entirely (they never appear in a
/// runnable plan), and `layer_parallel_safe = false` constrains parallelism
/// *across layers* rather than between modules in a stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EdgeReason {
    /// The `from` module writes an IR path that the `to` module reads.
    IrWriteRead {
        /// The shared IR path (e.g. `"PerimeterIR.regions.walls"`).
        writer_path: String,
    },
    /// The `to` module's manifest lists the `from` module in `requires_modules`.
    ExplicitRequires,
}

/// One serial edge between two modules in the same stage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SerialEdge {
    /// Upstream module — runs first.
    pub from: ModuleId,
    /// Downstream module — runs after `from`.
    pub to: ModuleId,
    /// Why the scheduler placed `from` before `to`.
    pub reason: EdgeReason,
}

/// Compute every in-stage serial edge for a single stage's modules.
///
/// Returns one `SerialEdge` per (writer, reader, reason) tuple — so a pair
/// connected by both an IR write/read overlap and an `ExplicitRequires` will
/// appear twice (once per reason). Callers that want a per-pair grouping can
/// fold by `(from, to)` themselves.
///
/// Order is deterministic: edges are sorted by `(from, to, reason_tag)`.
pub fn compute_serial_edges_for_stage(modules: &[LoadedModule]) -> Vec<SerialEdge> {
    let mut edges: Vec<SerialEdge> = Vec::new();

    for writer in modules {
        for reader in modules {
            if writer.id == reader.id {
                continue;
            }
            for written_path in &writer.ir_writes {
                if reader.ir_reads.contains(written_path) {
                    edges.push(SerialEdge {
                        from: writer.id.clone(),
                        to: reader.id.clone(),
                        reason: EdgeReason::IrWriteRead {
                            writer_path: written_path.clone(),
                        },
                    });
                }
            }
        }
    }

    for module in modules {
        for required in &module.requires_modules {
            if modules.iter().any(|m| &m.id == required) {
                edges.push(SerialEdge {
                    from: required.clone(),
                    to: module.id.clone(),
                    reason: EdgeReason::ExplicitRequires,
                });
            }
        }
    }

    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| reason_tag(&a.reason).cmp(&reason_tag(&b.reason)))
            .then_with(|| match (&a.reason, &b.reason) {
                (
                    EdgeReason::IrWriteRead { writer_path: ap },
                    EdgeReason::IrWriteRead { writer_path: bp },
                ) => ap.cmp(bp),
                _ => std::cmp::Ordering::Equal,
            })
    });
    edges
}

fn reason_tag(r: &EdgeReason) -> u8 {
    match r {
        EdgeReason::IrWriteRead { .. } => 0,
        EdgeReason::ExplicitRequires => 1,
    }
}

/// Runtime-equivalent of [`compute_serial_edges_for_stage`] that works on the
/// frozen [`CompiledModule`] view available inside `ExecutionPlan`.
///
/// `CompiledModule` does not carry `requires_modules`, so this variant only
/// emits `IrWriteRead` reasons. The topological order in `stage.modules`
/// already reflects any explicit-requires dependencies, so serial ordering is
/// still correct — the report just cannot label such pairs with
/// `EdgeReason::ExplicitRequires`. Plumbing `requires_modules` into
/// `CompiledModule` is a future enhancement.
pub fn compute_serial_edges_from_compiled(modules: &[CompiledModule]) -> Vec<SerialEdge> {
    let mut edges: Vec<SerialEdge> = Vec::new();
    for writer in modules {
        for reader in modules {
            if writer.module_id == reader.module_id {
                continue;
            }
            for written_path in &writer.ir_write_mask.paths {
                if reader.ir_read_mask.paths.contains(written_path) {
                    edges.push(SerialEdge {
                        from: writer.module_id.clone(),
                        to: reader.module_id.clone(),
                        reason: EdgeReason::IrWriteRead {
                            writer_path: written_path.clone(),
                        },
                    });
                }
            }
        }
    }
    edges.sort_by(|a, b| {
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| match (&a.reason, &b.reason) {
                (
                    EdgeReason::IrWriteRead { writer_path: ap },
                    EdgeReason::IrWriteRead { writer_path: bp },
                ) => ap.cmp(bp),
                _ => std::cmp::Ordering::Equal,
            })
    });
    edges
}

/// Bracket-shaped instrumentation surface called by the scheduler.
///
/// All methods take `&self` and must be safe to call from rayon worker
/// threads. Implementations should be lock-cheap on the hot path — the
/// scheduler invokes `on_module_start`/`on_module_end` once per module call
/// per layer.
pub trait PipelineInstrumentation: Send + Sync {
    /// Called when entering a top-level phase.
    fn on_phase_start(&self, phase: Phase);
    /// Called when leaving a top-level phase.
    fn on_phase_end(&self, phase: Phase);

    /// Called when a stage's module loop is about to begin. `layer` is `None`
    /// for prepass/postpass stages.
    fn on_stage_start(&self, stage: &StageId, layer: Option<u32>);
    /// Called after every module in a stage has run (or after an abort).
    fn on_stage_end(&self, stage: &StageId, layer: Option<u32>);

    /// Called immediately before a module dispatch.
    fn on_module_start(&self, stage: &StageId, layer: Option<u32>, module: &ModuleId);
    /// Called immediately after a module dispatch returns. `wasm_before` /
    /// `wasm_after` are the wasmtime `memory.data_size()` samples bracketing
    /// the call (0 / 0 if the module has no linear memory or if the
    /// dispatcher did not record a sample).
    fn on_module_end(
        &self,
        stage: &StageId,
        layer: Option<u32>,
        module: &ModuleId,
        wasm_before: u64,
        wasm_after: u64,
    );

    /// Called before a layer's stage loop begins. `z_mm` is the layer's
    /// nominal Z height in millimetres (matches `GlobalLayer.z: f32`).
    fn on_layer_start(&self, layer: u32, z_mm: f32);
    /// Called after a layer's stage loop completes (or aborts).
    fn on_layer_end(&self, layer: u32);

    /// Called once per stage during plan freeze, after the DAG has been
    /// built. Carries every in-stage serial edge so the report can show
    /// "module A → module B (reason)" rows even for stages where no modules
    /// happened to fire on a given run.
    fn record_edges(&self, stage: &StageId, tier: TierKind, edges: &[SerialEdge]);
}

/// Zero-overhead default. All methods are empty; the compiler inlines the
/// call sites to nothing.
pub struct NoopInstrumentation;

impl PipelineInstrumentation for NoopInstrumentation {
    fn on_phase_start(&self, _phase: Phase) {}
    fn on_phase_end(&self, _phase: Phase) {}
    fn on_stage_start(&self, _stage: &StageId, _layer: Option<u32>) {}
    fn on_stage_end(&self, _stage: &StageId, _layer: Option<u32>) {}
    fn on_module_start(&self, _stage: &StageId, _layer: Option<u32>, _module: &ModuleId) {}
    fn on_module_end(
        &self,
        _stage: &StageId,
        _layer: Option<u32>,
        _module: &ModuleId,
        _wasm_before: u64,
        _wasm_after: u64,
    ) {
    }
    fn on_layer_start(&self, _layer: u32, _z_mm: f32) {}
    fn on_layer_end(&self, _layer: u32) {}
    fn record_edges(&self, _stage: &StageId, _tier: TierKind, _edges: &[SerialEdge]) {}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::ConfigSchema;
    use slicer_ir::SemVer;
    use std::path::PathBuf;

    fn module(
        id: &str,
        ir_reads: &[&str],
        ir_writes: &[&str],
        requires_modules: &[&str],
    ) -> LoadedModule {
        LoadedModule {
            id: id.to_string(),
            version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            stage: "Layer::Perimeters".to_string(),
            wit_world: "slicer:world-layer@1.0.0".to_string(),
            ir_reads: ir_reads.iter().map(|s| s.to_string()).collect(),
            ir_writes: ir_writes.iter().map(|s| s.to_string()).collect(),
            claims: Vec::new(),
            requires_claims: Vec::new(),
            incompatible_with: Vec::new(),
            requires_modules: requires_modules.iter().map(|s| s.to_string()).collect(),
            min_host_version: SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            },
            min_ir_schema: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            max_ir_schema: SemVer {
                major: 2,
                minor: 0,
                patch: 0,
            },
            config_schema: ConfigSchema::default(),
            overridable_per_region: Vec::new(),
            overridable_per_layer: Vec::new(),
            layer_parallel_safe: true,
            wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
            placeholder_wasm: false,
        }
    }

    #[test]
    fn ir_write_read_overlap_emits_edge_with_path() {
        let modules = vec![
            module("a", &[], &["PerimeterIR.regions.walls"], &[]),
            module("b", &["PerimeterIR.regions.walls"], &[], &[]),
        ];
        let edges = compute_serial_edges_for_stage(&modules);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "a");
        assert_eq!(edges[0].to, "b");
        match &edges[0].reason {
            EdgeReason::IrWriteRead { writer_path } => {
                assert_eq!(writer_path, "PerimeterIR.regions.walls");
            }
            other => panic!("expected IrWriteRead, got {other:?}"),
        }
    }

    #[test]
    fn explicit_requires_emits_edge_with_explicit_reason() {
        let modules = vec![module("a", &[], &[], &[]), module("b", &[], &[], &["a"])];
        let edges = compute_serial_edges_for_stage(&modules);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].from, "a");
        assert_eq!(edges[0].to, "b");
        assert_eq!(edges[0].reason, EdgeReason::ExplicitRequires);
    }

    #[test]
    fn dual_reason_pair_emits_two_edges() {
        let modules = vec![
            module("a", &[], &["P"], &[]),
            module("b", &["P"], &[], &["a"]),
        ];
        let edges = compute_serial_edges_for_stage(&modules);
        assert_eq!(edges.len(), 2, "expected one edge per reason");
        assert!(edges.iter().any(|e| matches!(
            (&e.from[..], &e.to[..], &e.reason),
            ("a", "b", EdgeReason::IrWriteRead { .. })
        )));
        assert!(edges.iter().any(|e| matches!(
            (&e.from[..], &e.to[..], &e.reason),
            ("a", "b", EdgeReason::ExplicitRequires)
        )));
    }

    #[test]
    fn requires_pointing_at_missing_module_is_dropped() {
        let modules = vec![module("b", &[], &[], &["does-not-exist"])];
        let edges = compute_serial_edges_for_stage(&modules);
        assert!(edges.is_empty());
    }

    #[test]
    fn noop_instrumentation_compiles_and_runs() {
        let n = NoopInstrumentation;
        n.on_phase_start(Phase::PrePass);
        n.on_stage_start(&"s".to_string(), Some(0));
        n.on_module_start(&"s".to_string(), Some(0), &"m".to_string());
        n.on_module_end(&"s".to_string(), Some(0), &"m".to_string(), 0, 0);
        n.on_layer_start(0, 0.2f32);
        n.on_layer_end(0);
        n.on_stage_end(&"s".to_string(), Some(0));
        n.on_phase_end(Phase::PrePass);
        n.record_edges(&"s".to_string(), TierKind::PerLayer, &[]);
    }
}
