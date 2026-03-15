//! Host-side scheduler and manifest ingestion APIs.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod dag;
pub mod execution_plan;
pub mod instance_pool;
pub mod manifest;
pub mod topology;
pub mod validation;

pub use dag::{build_intra_stage_dag, ModuleNode};
pub use execution_plan::{
    build_execution_plan, CompiledModule, CompiledStage, ExecutionModuleBinding, ExecutionPlan,
    ExecutionPlanError, ExecutionPlanRequest, IrAccessMask, SortedStageModules,
};
pub use instance_pool::{
    build_wasm_instance_pool, InstancePoolError, InstancePoolMode, WasmArtifactMetadata,
    WasmInstanceLease, WasmInstancePool,
};
pub use manifest::{
    load_module_from_paths, load_modules_from_roots, ConfigSchema, DiagnosticLevel, LoadDiagnostic,
    LoadError, LoadErrorKind, LoadModulesReport, LoadedModule,
};
pub use topology::topological_sort;
pub use validation::{
    validate_startup_dag, AccessKind, ClaimHolder, ConflictScope, DagValidationDiagnostic,
    DagValidationPass, DagValidationReport, DagValidationRequest, ModuleAccessAudit,
    SchedulerError, StageDag,
};
