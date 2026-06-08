//! Static planning subsystem extracted from `slicer-runtime` (packet 85).
//!
//! Wasmtime-free: this crate hosts manifest ingestion, config resolution,
//! DAG construction + validation, execution-plan compilation, and DAG-CLI
//! introspection. Live-loading (anything that owns wasmtime artifacts) lives
//! in `slicer-wasm-host`.

#![warn(missing_docs)]

pub mod config_resolution;
pub mod dag;
pub mod dag_cli;
pub mod execution_plan;
pub mod instrumentation;
pub mod manifest;
pub mod module_search_path;
pub mod region_split;
pub mod stage_order;
pub mod topology;
pub mod validation;

// Flat re-exports mirroring the pre-P85 `slicer_runtime::*` shape so external
// callers can write `slicer_scheduler::ExecutionPlan` etc. without per-module
// qualifications.

pub use config_resolution::{
    paint_semantic_namespace_key, resolve_global_config, resolve_per_object_configs,
    resolve_per_paint_semantic_configs, validate_support_layer_heights, BoundsDeclaration,
    ConfigBoundsIndex, ConfigResolutionError, UnknownSemanticWarning,
};
pub use dag::{
    build_global_dag, build_intra_stage_dag, BuiltinProducer, EdgeTo, GlobalEdge, ModuleNode,
    Producer,
};
pub use dag_cli::{
    run_dag_claims, run_dag_depends, run_dag_stage, run_dag_stages, ClaimOut, ClaimsOut,
    DependsOut, GlobalEdgeOut, ModuleOut, StageEdgeOut, StageOut, StageSummary, StagesOut,
};
pub use execution_plan::{
    bind_module_config_view, build_execution_plan, dedup_same_claim_modules_for_test,
    parse_cli_config_source, CompiledModuleBuilder, CompiledModuleStatic, CompiledStage,
    ConfigSourceParseError, ExecutionModuleBinding, ExecutionPlan, ExecutionPlanError,
    ExecutionPlanRequest, IrAccessMask, SortedStageModules, DEFAULT_REGION_MAP_CAP,
    MAX_LAYER_INDEX, STAGE_ORDER,
};
pub use instrumentation::{compute_serial_edges_for_stage, EdgeReason, SerialEdge};
pub use manifest::{
    build_config_schema_json, load_module_from_paths, load_modules_from_roots, ConfigFieldEntry,
    ConfigSchema, DiagnosticLevel, LoadDiagnostic, LoadError, LoadErrorKind, LoadModulesReport,
    LoadedModule, LoadedModuleBuilder, RegionSplitDeclaration, RegionSplitValueType,
};
pub use module_search_path::{assemble_search_roots, SLICER_MODULE_PATH_ENV};
pub use topology::topological_sort;
pub use validation::{
    resolve_held_claims, validate_startup_dag, AccessKind, ClaimHolder, ConflictScope,
    DagValidationDiagnostic, DagValidationPass, DagValidationReport, DagValidationRequest,
    FillHolders, ModuleAccessAudit, SchedulerError, StageDag, FILL_CLAIM_IDS,
};
