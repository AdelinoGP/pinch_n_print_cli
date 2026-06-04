//! Deterministic in-stage topological ordering contracts.

use std::collections::{BTreeMap, BTreeSet};

use slicer_ir::ModuleId;

use crate::dag::ModuleNode;

/// Produces a deterministic module order for a validated intra-stage DAG.
///
/// Returns the remaining unsorted module IDs when a cycle prevents full
/// ordering.
pub fn topological_sort(nodes: &[ModuleNode]) -> Result<Vec<ModuleId>, Vec<ModuleId>> {
    let mut in_degree: BTreeMap<ModuleId, usize> = nodes
        .iter()
        .map(|node| (node.module_id.clone(), 0usize))
        .collect();

    let adjacency: BTreeMap<ModuleId, BTreeSet<ModuleId>> = nodes
        .iter()
        .map(|node| {
            (
                node.module_id.clone(),
                node.edges_to
                    .iter()
                    .map(|e| e.to.clone())
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect();

    for downstream_ids in adjacency.values() {
        for downstream_id in downstream_ids {
            if let Some(degree) = in_degree.get_mut(downstream_id) {
                *degree += 1;
            }
        }
    }

    let mut ready: BTreeSet<ModuleId> = in_degree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(module_id, _)| module_id.clone())
        .collect();
    let mut sorted = Vec::with_capacity(nodes.len());

    while let Some(module_id) = ready.pop_first() {
        sorted.push(module_id.clone());

        if let Some(downstream_ids) = adjacency.get(&module_id) {
            for downstream_id in downstream_ids {
                if let Some(degree) = in_degree.get_mut(downstream_id) {
                    *degree -= 1;
                    if *degree == 0 {
                        ready.insert(downstream_id.clone());
                    }
                }
            }
        }
    }

    if sorted.len() == nodes.len() {
        Ok(sorted)
    } else {
        Err(in_degree
            .into_iter()
            .filter_map(|(module_id, degree)| (degree > 0).then_some(module_id))
            .collect())
    }
}
