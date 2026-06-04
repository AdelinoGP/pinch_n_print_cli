//! Planning-side instrumentation types for the scheduler.
//!
//! Contains `EdgeReason`, `SerialEdge`, and `compute_serial_edges_for_stage` —
//! the planning-time helpers that derive in-stage serial-edge reasons from a
//! stage's `LoadedModule` list.
//!
//! Runtime-side types (`PipelineInstrumentation`, `Phase`, `TierKind`,
//! `NoopInstrumentation`, `StageInstrumentationGuard`, `CompositeInstrumentation`)
//! live in `slicer_runtime::instrumentation`.

use serde::Serialize;
use slicer_ir::ModuleId;

use crate::manifest::LoadedModule;

/// Reason a serial edge exists between two modules in the same stage.
///
/// Only the variants that the actual DAG builder emits are represented —
/// claim conflicts block plan validation entirely (they never appear in a
/// runnable plan), and `layer_parallel_safe = false` constrains parallelism
/// *across layers* rather than between modules in a stage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::LoadedModuleBuilder;
    use slicer_ir::SemVer;
    use std::path::PathBuf;

    fn module(
        id: &str,
        ir_reads: &[&str],
        ir_writes: &[&str],
        requires_modules: &[&str],
    ) -> LoadedModule {
        LoadedModuleBuilder::new(
            id,
            SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            "Layer::Perimeters",
            "slicer:world-layer@1.0.0",
            PathBuf::from(format!("fixtures/{id}.wasm")),
        )
        .ir_reads(ir_reads.iter().map(|s| s.to_string()).collect())
        .ir_writes(ir_writes.iter().map(|s| s.to_string()).collect())
        .requires_modules(requires_modules.iter().map(|s| s.to_string()).collect())
        .min_host_version(SemVer {
            major: 0,
            minor: 1,
            patch: 0,
        })
        .min_ir_schema(SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        })
        .max_ir_schema(SemVer {
            major: 2,
            minor: 0,
            patch: 0,
        })
        .layer_parallel_safe(true)
        .build()
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
}
