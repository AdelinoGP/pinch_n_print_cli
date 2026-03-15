//! Startup DAG validation contracts for the host scheduler.

use slicer_ir::{ModuleId, SemVer, StageId};

use crate::dag::ModuleNode;
use crate::manifest::{DiagnosticLevel, LoadedModule};

/// Structured scheduler error surfaced by DAG validation and DAG construction APIs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    /// Placeholder variant used until validation logic is implemented.
    NotImplemented,
    /// A manifest declared an unknown scheduler stage.
    UnknownStage {
        /// Module that declared the invalid stage.
        module: ModuleId,
        /// Stage string read from the manifest.
        declared_stage: StageId,
    },
    /// Two modules claim the same capability in the same effective scope.
    ClaimConflict {
        /// Claim identifier in conflict.
        claim: String,
        /// First conflicting module.
        module_a: ModuleId,
        /// Second conflicting module.
        module_b: ModuleId,
        /// Scope in which the conflict was observed.
        scope: ConflictScope,
    },
    /// Two modules were declared incompatible.
    IncompatibleModules {
        /// Module that declared the incompatibility.
        declared_by: ModuleId,
        /// Module matched by the incompatibility rule.
        conflicting: ModuleId,
        /// Human-readable explanation for the incompatibility.
        reason: String,
    },
    /// A required module or required capability could not be satisfied.
    MissingDependency {
        /// Module with the missing dependency.
        module: ModuleId,
        /// Missing module id or capability identifier.
        requires: ModuleId,
    },
    /// One stage DAG contains a cycle.
    CyclicDependency {
        /// Module ids that remain in the cycle.
        cycle: Vec<ModuleId>,
    },
    /// A module reads an IR field with no available upstream producer.
    UnfulfilledRead {
        /// Module performing the read.
        module: ModuleId,
        /// Missing IR field path.
        field: String,
        /// Optional remediation hint.
        suggestion: Option<String>,
    },
    /// A module requires an incompatible IR schema range.
    IrVersionIncompatible {
        /// Module with the incompatible requirement.
        module: ModuleId,
        /// IR contract or type name being checked.
        ir_type: String,
        /// Minimum required version.
        required: SemVer,
        /// Host-provided version.
        available: SemVer,
    },
    /// The exported runtime entrypoint does not match the declared stage.
    StageMismatch {
        /// Module with the mismatch.
        module: ModuleId,
        /// Stage declared in the manifest.
        declared_stage: StageId,
        /// Exported function that disagreed.
        exported_fn: String,
    },
    /// Two modules in the same stage both write the same field without ordering.
    WriteConflict {
        /// IR field path written by both modules.
        field: String,
        /// First conflicting module.
        module_a: ModuleId,
        /// Second conflicting module.
        module_b: ModuleId,
        /// Stage containing the conflict.
        stage: StageId,
        /// Whether a semantic read-after-write chain could order the pair.
        orderable: bool,
    },
    /// A module wrote an IR field that no downstream module consumes.
    DeadWrite {
        /// Module that performed the write.
        module: ModuleId,
        /// Dead IR field path.
        field: String,
    },
    /// Runtime access exceeded the module's declared manifest access mask.
    UndeclaredAccess {
        /// Module performing the undeclared access.
        module: ModuleId,
        /// Whether the access was a read or write.
        access: AccessKind,
        /// Access path that was not declared.
        path: String,
    },
    /// A module directly requires a module from a later scheduler stage.
    CrossStageDependency {
        /// Requesting module.
        module: ModuleId,
        /// Required module.
        requires: ModuleId,
        /// Requesting module stage.
        module_stage: StageId,
        /// Required module stage.
        required_stage: StageId,
    },
    /// A module transitively depends on a later scheduler stage.
    TransitiveStageDependency {
        /// Requesting module.
        module: ModuleId,
        /// Discovered dependency chain.
        path: Vec<ModuleId>,
        /// Requesting module stage.
        module_stage: StageId,
        /// Later stage found in the transitive closure.
        later_stage: StageId,
    },
}

/// Scope used when reporting claim conflicts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictScope {
    /// Conflict applies globally to the entire print.
    Global,
    /// Conflict applies after region-level filtering.
    Region {
        /// Object identifier participating in the conflict.
        object_id: String,
        /// Region identifier participating in the conflict.
        region_id: String,
        /// Optional global layer index used to pin the conflict.
        global_layer_index: Option<u32>,
    },
}

/// Runtime access classification used by undeclared-access diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    /// Runtime IR read.
    Read,
    /// Runtime IR write.
    Write,
}

/// One stage-local DAG supplied to the startup validator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageDag {
    /// Stage represented by these nodes.
    pub stage: StageId,
    /// Nodes for that stage.
    pub nodes: Vec<ModuleNode>,
}

/// One effective claim holder observation used by validation passes 2 and 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimHolder {
    /// Claim identifier being resolved.
    pub claim: String,
    /// Module selected as a holder in the given scope.
    pub module_id: ModuleId,
    /// Scope in which this holder is active.
    pub scope: ConflictScope,
}

/// Static or recorded runtime access summary for validation pass 11.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleAccessAudit {
    /// Module being audited.
    pub module_id: ModuleId,
    /// Runtime read paths requested by the module.
    pub runtime_reads: Vec<String>,
    /// Runtime write paths committed by the module.
    pub runtime_writes: Vec<String>,
}

/// Startup validation input spanning all 13 documented passes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagValidationRequest {
    /// Loaded modules to validate.
    pub modules: Vec<LoadedModule>,
    /// Per-stage DAGs produced during phase 2.
    pub stage_dags: Vec<StageDag>,
    /// Host IR schema version available to loaded modules.
    pub host_ir_schema_version: SemVer,
    /// Effective claim holder snapshots for global and region scopes.
    pub claim_holders: Vec<ClaimHolder>,
    /// Optional runtime/static access audits used by pass 11.
    pub access_audits: Vec<ModuleAccessAudit>,
}

/// One of the 13 startup DAG validation passes from the scheduler contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DagValidationPass {
    /// Pass 1.
    StageIdValidation,
    /// Pass 2.
    GlobalClaimConflicts,
    /// Pass 3.
    PerRegionClaimConflicts,
    /// Pass 4.
    IncompatibilityDeclarations,
    /// Pass 5.
    MissingDependencies,
    /// Pass 6.
    IrVersionCompatibility,
    /// Pass 7.
    CycleDetection,
    /// Pass 8.
    WriteConflicts,
    /// Pass 9.
    UnfulfilledReads,
    /// Pass 10.
    DeadWrites,
    /// Pass 11.
    UndeclaredAccess,
    /// Pass 12.
    CrossStageDependencyLegality,
    /// Pass 13.
    TransitiveDependencyLegality,
}

/// One structured startup validation diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DagValidationDiagnostic {
    /// Validation pass that emitted the diagnostic.
    pub pass: DagValidationPass,
    /// Whether the diagnostic blocks execution.
    pub level: DiagnosticLevel,
    /// Structured scheduler error or warning payload.
    pub detail: SchedulerError,
}

/// Aggregated DAG validation output collected before surfacing diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DagValidationReport {
    /// Fatal diagnostics that block execution.
    pub errors: Vec<DagValidationDiagnostic>,
    /// Warning diagnostics that do not block execution.
    pub warnings: Vec<DagValidationDiagnostic>,
}

impl DagValidationReport {
    /// Appends one fatal diagnostic to the report.
    pub fn push_error(&mut self, pass: DagValidationPass, detail: SchedulerError) {
        self.errors.push(DagValidationDiagnostic {
            pass,
            level: DiagnosticLevel::Error,
            detail,
        });
    }

    /// Appends one warning diagnostic to the report.
    pub fn push_warning(&mut self, pass: DagValidationPass, detail: SchedulerError) {
        self.warnings.push(DagValidationDiagnostic {
            pass,
            level: DiagnosticLevel::Warning,
            detail,
        });
    }

    /// Returns true when no fatal diagnostics were recorded.
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Runs all 13 startup DAG validation passes and aggregates diagnostics.
pub fn validate_startup_dag(_request: &DagValidationRequest) -> DagValidationReport {
    todo!("TASK-022 red pass: startup DAG validation is not implemented yet")
}

#[cfg(test)]
mod tests {
    use super::{DagValidationPass, DagValidationReport, SchedulerError};

    #[test]
    fn report_groups_errors_and_warnings_by_severity() {
        let mut report = DagValidationReport::default();
        report.push_error(DagValidationPass::CycleDetection, SchedulerError::NotImplemented);
        report.push_warning(DagValidationPass::DeadWrites, SchedulerError::NotImplemented);

        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.warnings.len(), 1);
        assert!(!report.is_valid());
    }
}
