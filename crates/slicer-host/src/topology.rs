//! Deterministic in-stage topological ordering contracts.

use slicer_ir::ModuleId;

use crate::dag::ModuleNode;

/// Produces a deterministic module order for a validated intra-stage DAG.
///
/// Returns the remaining unsorted module IDs when a cycle prevents full
/// ordering.
pub fn topological_sort(nodes: &[ModuleNode]) -> Result<Vec<ModuleId>, Vec<ModuleId>> {
    let _ = nodes;
    todo!("TASK-023: implement Kahn topological sort")
}
