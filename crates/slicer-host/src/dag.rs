//! Intra-stage DAG construction contracts.

use slicer_ir::{ModuleId, StageId};

use crate::manifest::LoadedModule;

/// One module node in an intra-stage dependency graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleNode {
    /// Deterministic node identity derived from `LoadedModule.id`.
    pub module_id: ModuleId,
    /// Declared IR read paths copied from the module manifest.
    pub ir_reads: Vec<String>,
    /// Declared IR write paths copied from the module manifest.
    pub ir_writes: Vec<String>,
    /// Outgoing edges to downstream modules in the same stage.
    pub edges_to: Vec<ModuleId>,
}

/// Scheduler error surfaced by DAG construction APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    /// Placeholder variant used until DAG construction is implemented.
    NotImplemented,
}

/// Builds the dependency graph for one scheduler stage.
pub fn build_intra_stage_dag(
    _stage: StageId,
    _modules: &[LoadedModule],
) -> Result<Vec<ModuleNode>, SchedulerError> {
    todo!("TASK-021: build intra-stage DAG from manifest reads, writes, and requires")
}
