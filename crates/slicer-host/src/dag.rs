//! Intra-stage DAG construction contracts.

use std::collections::BTreeMap;

use slicer_ir::{ModuleId, StageId};

use crate::instrumentation::EdgeReason;
use crate::manifest::LoadedModule;
use crate::validation::SchedulerError;

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

/// Builds the dependency graph for one scheduler stage.
pub fn build_intra_stage_dag(
    stage: StageId,
    modules: &[LoadedModule],
) -> Result<Vec<ModuleNode>, Box<SchedulerError>> {
    let stage_modules: Vec<&LoadedModule> = modules
        .iter()
        .filter(|module| module.stage == stage)
        .collect();

    let mut nodes = BTreeMap::new();
    for module in &stage_modules {
        nodes.insert(
            module.id.clone(),
            ModuleNode {
                module_id: module.id.clone(),
                ir_reads: module.ir_reads.clone(),
                ir_writes: module.ir_writes.clone(),
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

    for writer in &stage_modules {
        for reader in &stage_modules {
            if writer.id == reader.id {
                continue;
            }
            for written_path in &writer.ir_writes {
                if reader.ir_reads.contains(written_path) {
                    let reason = EdgeReason::IrWriteRead {
                        writer_path: written_path.clone(),
                    };
                    if let Some(by_dst) = edges_by_source.get_mut(&writer.id) {
                        let reasons = by_dst.entry(reader.id.clone()).or_default();
                        if !reasons.contains(&reason) {
                            reasons.push(reason);
                        }
                    }
                }
            }
        }
    }

    for module in &stage_modules {
        for required_module in &module.requires_modules {
            if let Some(by_dst) = edges_by_source.get_mut(required_module) {
                let reasons = by_dst.entry(module.id.clone()).or_default();
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

    use super::{build_intra_stage_dag, EdgeTo, ModuleNode};
    use crate::instrumentation::EdgeReason;
    use crate::manifest::{ConfigSchema, LoadedModule};

    #[test]
    fn deduplicates_and_sorts_edges_by_module_id() {
        let stage = String::from("Layer::Perimeters");
        let nodes = build_intra_stage_dag(
            stage.clone(),
            &[
                loaded_module(
                    "com.example.base",
                    &stage,
                    &[],
                    &["PerimeterIR.regions.walls"],
                    &[],
                ),
                loaded_module(
                    "com.example.alpha",
                    &stage,
                    &["PerimeterIR.regions.walls"],
                    &[],
                    &["com.example.base"],
                ),
                loaded_module(
                    "com.example.beta",
                    &stage,
                    &["PerimeterIR.regions.walls"],
                    &[],
                    &["com.example.base"],
                ),
            ],
        )
        .expect("intra-stage DAG construction should succeed");

        let base = nodes
            .iter()
            .find(|node| node.module_id == "com.example.base")
            .expect("base node should exist");

        assert_eq!(
            base,
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
        LoadedModule {
            id: String::from(id),
            version: semver(1, 0, 0),
            stage: String::from(stage),
            wit_world: String::from("slicer:world-layer@1.0.0"),
            ir_reads: strings(ir_reads),
            ir_writes: strings(ir_writes),
            claims: Vec::new(),
            requires_claims: Vec::new(),
            incompatible_with: Vec::new(),
            requires_modules: strings(requires_modules),
            min_host_version: semver(0, 1, 0),
            min_ir_schema: semver(1, 0, 0),
            max_ir_schema: semver(2, 0, 0),
            config_schema: ConfigSchema::default(),
            overridable_per_region: Vec::new(),
            overridable_per_layer: Vec::new(),
            layer_parallel_safe: true,
            wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
            placeholder_wasm: false,
        }
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
