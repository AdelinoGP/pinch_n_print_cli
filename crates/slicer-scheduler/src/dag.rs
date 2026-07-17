//! Intra-stage DAG construction contracts.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use slicer_ir::{ModuleId, SemVer, StageId};

use crate::instrumentation::EdgeReason;
use crate::manifest::LoadedModule;
use crate::validation::SchedulerError;

// ---------------------------------------------------------------------------
// Producer trait — smallest projection all DAG validators and CLI commands need
// ---------------------------------------------------------------------------

/// Uniform projection over anything that behaves like a scheduled module:
/// either a WASM [`LoadedModule`] or a compile-time [`BuiltinProducer`].
///
/// The 9-method surface is locked; do not add methods without a spec change.
pub trait Producer {
    /// Reverse-domain module identifier.
    fn id(&self) -> &str;
    /// Canonical scheduler stage identifier.
    fn stage(&self) -> &str;
    /// Declared IR write paths.
    fn ir_writes(&self) -> &[String];
    /// Declared IR read paths.
    fn ir_reads(&self) -> &[String];
    /// Claims held by this producer.
    fn claims_holds(&self) -> &[String];
    /// Claims required from peer producers.
    fn claims_requires(&self) -> &[String];
    /// Explicit peer-module dependencies.
    fn requires_modules(&self) -> &[String];
    /// Inclusive minimum IR schema version accepted.
    fn min_ir_schema(&self) -> SemVer;
    /// Exclusive maximum IR schema version accepted.
    fn max_ir_schema(&self) -> SemVer;
}

// LoadedModule already stores Vec<String> for all list fields — zero allocation.
// Implemented on the value type so `&LoadedModule` coerces to `&dyn Producer`.
impl Producer for LoadedModule {
    fn id(&self) -> &str {
        &self.id
    }
    fn stage(&self) -> &str {
        &self.stage
    }
    fn ir_writes(&self) -> &[String] {
        &self.ir_writes
    }
    fn ir_reads(&self) -> &[String] {
        &self.ir_reads
    }
    fn claims_holds(&self) -> &[String] {
        &self.claims
    }
    fn claims_requires(&self) -> &[String] {
        &self.requires_claims
    }
    fn requires_modules(&self) -> &[String] {
        &self.requires_modules
    }
    fn min_ir_schema(&self) -> SemVer {
        self.min_ir_schema
    }
    fn max_ir_schema(&self) -> SemVer {
        self.max_ir_schema
    }
}

// ---------------------------------------------------------------------------
// BuiltinProducer — compile-time-declarable record for built-in pipeline steps
// ---------------------------------------------------------------------------

/// Compile-time-declarable record representing a built-in (non-WASM) pipeline
/// step. Fields use `&'static str` / `&'static [&'static str]` so instances
/// can be expressed as `const`.
///
/// `Producer` methods that return `&[String]` are backed by a per-instance
/// `OnceLock<Vec<String>>` cache (allocated once on first call; BuiltinProducer
/// instances are per-process singletons so this is acceptable).
pub struct BuiltinProducer {
    /// Reverse-domain module identifier.
    pub id: &'static str,
    /// Canonical scheduler stage identifier.
    pub stage: &'static str,
    /// Declared IR write paths.
    pub ir_writes: &'static [&'static str],
    /// Declared IR read paths.
    pub ir_reads: &'static [&'static str],
    /// Claims held by this producer.
    pub claims_holds: &'static [&'static str],
    /// Claims required from peer producers.
    pub claims_requires: &'static [&'static str],
    /// Explicit peer-module dependencies.
    pub requires_modules: &'static [&'static str],
    /// Inclusive minimum IR schema version.
    pub min_ir_schema: SemVer,
    /// Exclusive maximum IR schema version.
    pub max_ir_schema: SemVer,
    // OnceLock caches — one per list field that the trait returns as &[String].
    #[doc(hidden)]
    pub _cache_ir_writes: OnceLock<Vec<String>>,
    #[doc(hidden)]
    pub _cache_ir_reads: OnceLock<Vec<String>>,
    #[doc(hidden)]
    pub _cache_claims_holds: OnceLock<Vec<String>>,
    #[doc(hidden)]
    pub _cache_claims_requires: OnceLock<Vec<String>>,
    #[doc(hidden)]
    pub _cache_requires_modules: OnceLock<Vec<String>>,
}

// Implemented on the value type so `&BuiltinProducer` coerces to `&dyn Producer`.
// The OnceLock caches are initialised lazily on first call and are safe to
// share across threads because `OnceLock::get_or_init` is thread-safe.
impl Producer for BuiltinProducer {
    fn id(&self) -> &str {
        self.id
    }
    fn stage(&self) -> &str {
        self.stage
    }
    fn ir_writes(&self) -> &[String] {
        self._cache_ir_writes
            .get_or_init(|| self.ir_writes.iter().map(|s| s.to_string()).collect())
    }
    fn ir_reads(&self) -> &[String] {
        self._cache_ir_reads
            .get_or_init(|| self.ir_reads.iter().map(|s| s.to_string()).collect())
    }
    fn claims_holds(&self) -> &[String] {
        self._cache_claims_holds
            .get_or_init(|| self.claims_holds.iter().map(|s| s.to_string()).collect())
    }
    fn claims_requires(&self) -> &[String] {
        self._cache_claims_requires
            .get_or_init(|| self.claims_requires.iter().map(|s| s.to_string()).collect())
    }
    fn requires_modules(&self) -> &[String] {
        self._cache_requires_modules.get_or_init(|| {
            self.requires_modules
                .iter()
                .map(|s| s.to_string())
                .collect()
        })
    }
    fn min_ir_schema(&self) -> SemVer {
        self.min_ir_schema
    }
    fn max_ir_schema(&self) -> SemVer {
        self.max_ir_schema
    }
}

/// One outgoing edge from a [`ModuleNode`]: the downstream module plus
/// every reason the scheduler had to place it after the source.
///
/// Each `(from, to)` pair appears at most once in [`ModuleNode::edges_to`];
/// multiple reasons (e.g. an IR write/read overlap plus an explicit
/// `requires_modules`) accumulate into `reasons`. Consumers that only need
/// the topology (in-degree, reachability) read `edge.to`; consumers that
/// need to explain ordering (the slicer report) read both fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeTo {
    /// Downstream module that must run after the source.
    pub to: ModuleId,
    /// Non-empty list of distinct reasons this edge exists.
    pub reasons: Vec<EdgeReason>,
}

/// One module node in an intra-stage dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleNode {
    /// Deterministic node identity derived from `LoadedModule.id`.
    pub module_id: ModuleId,
    /// Declared IR read paths copied from the module manifest.
    pub ir_reads: Vec<String>,
    /// Declared IR write paths copied from the module manifest.
    pub ir_writes: Vec<String>,
    /// Outgoing edges to downstream modules in the same stage, each carrying
    /// the reasons it exists. Sorted by `to` for deterministic traversal.
    pub edges_to: Vec<EdgeTo>,
}

/// One cross-stage serial edge between two modules anywhere in the
/// discovered module set.
///
/// Returned by [`build_global_dag`] so `dag depends` can show edges that
/// cross stage boundaries (e.g. `Layer::Infill` → `PostPass::GCodeEmit` via
/// an `InfillIR.regions[].paths` write/read overlap).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobalEdge {
    /// Upstream module — runs first.
    pub from: ModuleId,
    /// Stage that `from` belongs to.
    pub from_stage: StageId,
    /// Downstream module — runs after `from`.
    pub to: ModuleId,
    /// Stage that `to` belongs to.
    pub to_stage: StageId,
    /// Why the scheduler placed `from` before `to`.
    pub reason: EdgeReason,
}

/// Apply the same `IrWriteRead` + `ExplicitRequires` edge rules as
/// [`build_intra_stage_dag`] across all producers without a stage filter.
/// Returns every edge with `(from_stage, to_stage)` populated so callers can
/// identify stage-boundary crossings.
///
/// Sort order is deterministic: by `(from, to, reason_tag, writer_path)`,
/// matching `compute_serial_edges_for_stage`.
pub fn build_global_dag(producers: &[&dyn Producer]) -> Vec<GlobalEdge> {
    let mut edges: Vec<GlobalEdge> = Vec::new();
    for writer in producers {
        for reader in producers {
            if writer.id() == reader.id() {
                continue;
            }
            for written_path in writer.ir_writes() {
                if reader.ir_reads().contains(written_path) {
                    edges.push(GlobalEdge {
                        from: writer.id().to_string(),
                        from_stage: writer.stage().to_string(),
                        to: reader.id().to_string(),
                        to_stage: reader.stage().to_string(),
                        reason: EdgeReason::IrWriteRead {
                            writer_path: written_path.clone(),
                        },
                    });
                }
            }
        }
    }
    for producer in producers {
        for required in producer.requires_modules() {
            if let Some(req) = producers.iter().find(|p| p.id() == required.as_str()) {
                edges.push(GlobalEdge {
                    from: req.id().to_string(),
                    from_stage: req.stage().to_string(),
                    to: producer.id().to_string(),
                    to_stage: producer.stage().to_string(),
                    reason: EdgeReason::ExplicitRequires,
                });
            }
        }
    }
    edges.sort_by(|a, b| {
        let a_tag = match &a.reason {
            EdgeReason::IrWriteRead { .. } => 0u8,
            EdgeReason::ExplicitRequires => 1u8,
        };
        let b_tag = match &b.reason {
            EdgeReason::IrWriteRead { .. } => 0u8,
            EdgeReason::ExplicitRequires => 1u8,
        };
        a.from
            .cmp(&b.from)
            .then_with(|| a.to.cmp(&b.to))
            .then_with(|| a_tag.cmp(&b_tag))
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

/// Builds the dependency graph for one scheduler stage.
pub fn build_intra_stage_dag(
    stage: StageId,
    producers: &[&dyn Producer],
) -> Result<Vec<ModuleNode>, Box<SchedulerError>> {
    let stage_producers: Vec<&dyn Producer> = producers
        .iter()
        .copied()
        .filter(|p| p.stage() == stage)
        .collect();

    let mut nodes = BTreeMap::new();
    for producer in &stage_producers {
        nodes.insert(
            producer.id().to_string(),
            ModuleNode {
                module_id: producer.id().to_string(),
                ir_reads: producer.ir_reads().to_vec(),
                ir_writes: producer.ir_writes().to_vec(),
                edges_to: Vec::new(),
            },
        );
    }

    // Per-source adjacency keyed by downstream module, with deduped reasons.
    let stage_ids: Vec<ModuleId> = nodes.keys().cloned().collect();
    let mut edges_by_source: BTreeMap<ModuleId, BTreeMap<ModuleId, Vec<EdgeReason>>> = stage_ids
        .iter()
        .cloned()
        .map(|module_id| (module_id, BTreeMap::new()))
        .collect();

    for writer in &stage_producers {
        for reader in &stage_producers {
            if writer.id() == reader.id() {
                continue;
            }
            for written_path in writer.ir_writes() {
                if reader.ir_reads().contains(written_path) {
                    let reason = EdgeReason::IrWriteRead {
                        writer_path: written_path.clone(),
                    };
                    if let Some(by_dst) = edges_by_source.get_mut(writer.id()) {
                        let reasons = by_dst.entry(reader.id().to_string()).or_default();
                        if !reasons.contains(&reason) {
                            reasons.push(reason);
                        }
                    }
                }
            }
        }
    }

    for producer in &stage_producers {
        for required_module in producer.requires_modules() {
            if let Some(by_dst) = edges_by_source.get_mut(required_module.as_str()) {
                let reasons = by_dst.entry(producer.id().to_string()).or_default();
                if !reasons.contains(&EdgeReason::ExplicitRequires) {
                    reasons.push(EdgeReason::ExplicitRequires);
                }
            }
        }
    }

    for (module_id, by_dst) in edges_by_source {
        if let Some(node) = nodes.get_mut(&module_id) {
            node.edges_to = by_dst
                .into_iter()
                .map(|(to, reasons)| EdgeTo { to, reasons })
                .collect();
        }
    }

    Ok(nodes.into_values().collect())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use slicer_ir::SemVer;

    use super::{build_intra_stage_dag, EdgeTo, ModuleNode, Producer};
    use crate::instrumentation::EdgeReason;
    use crate::manifest::{LoadedModule, LoadedModuleBuilder};

    #[test]
    fn deduplicates_and_sorts_edges_by_module_id() {
        let stage = String::from("Layer::Perimeters");
        let base = loaded_module(
            "com.example.base",
            &stage,
            &[],
            &["PerimeterIR.regions.walls"],
            &[],
        );
        let alpha = loaded_module(
            "com.example.alpha",
            &stage,
            &["PerimeterIR.regions.walls"],
            &[],
            &["com.example.base"],
        );
        let beta = loaded_module(
            "com.example.beta",
            &stage,
            &["PerimeterIR.regions.walls"],
            &[],
            &["com.example.base"],
        );
        let producers: Vec<&dyn Producer> = vec![&base, &alpha, &beta];
        let nodes = build_intra_stage_dag(stage.clone(), &producers)
            .expect("intra-stage DAG construction should succeed");

        let base_node = nodes
            .iter()
            .find(|node| node.module_id == "com.example.base")
            .expect("base node should exist");

        assert_eq!(
            base_node,
            &ModuleNode {
                module_id: String::from("com.example.base"),
                ir_reads: Vec::new(),
                ir_writes: vec![String::from("PerimeterIR.regions.walls")],
                edges_to: vec![
                    EdgeTo {
                        to: String::from("com.example.alpha"),
                        reasons: vec![
                            EdgeReason::IrWriteRead {
                                writer_path: String::from("PerimeterIR.regions.walls"),
                            },
                            EdgeReason::ExplicitRequires,
                        ],
                    },
                    EdgeTo {
                        to: String::from("com.example.beta"),
                        reasons: vec![
                            EdgeReason::IrWriteRead {
                                writer_path: String::from("PerimeterIR.regions.walls"),
                            },
                            EdgeReason::ExplicitRequires,
                        ],
                    },
                ],
            }
        );
    }

    fn loaded_module(
        id: &str,
        stage: &str,
        ir_reads: &[&str],
        ir_writes: &[&str],
        requires_modules: &[&str],
    ) -> LoadedModule {
        LoadedModuleBuilder::new(
            id,
            semver(1, 0, 0),
            stage,
            slicer_schema::WORLD_LAYER,
            PathBuf::from(format!("fixtures/{id}.wasm")),
        )
        .ir_reads(strings(ir_reads))
        .ir_writes(strings(ir_writes))
        .requires_modules(strings(requires_modules))
        .min_host_version(semver(0, 1, 0))
        .min_ir_schema(semver(1, 0, 0))
        .max_ir_schema(semver(2, 0, 0))
        .layer_parallel_safe(true)
        .build()
    }

    fn strings(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| String::from(*value)).collect()
    }

    fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
        SemVer {
            major,
            minor,
            patch,
        }
    }
}
