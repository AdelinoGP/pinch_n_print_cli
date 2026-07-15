//! Host-side scheduler and manifest ingestion APIs.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod blackboard;
/// Builtin pipeline step producers.
pub mod builtins;
pub mod diagnose;
pub mod instrumentation;
pub mod layer_executor;
pub mod layer_finalization;
pub mod negative_part_subtract;
pub mod pipeline;
pub mod postpass;
pub mod prepass;
pub mod progress_events;
pub mod progress_instrumentation;
pub mod region_partition;
#[cfg(feature = "report")]
pub mod report;
pub mod run;
pub mod slice_postprocess;
pub mod slice_postprocess_prepass;
pub mod visual_debug_render;

// Modules moved to slicer-scheduler — re-exported here for backward compatibility.
// kept: transitional shim for downstream consumers (slicer-runtime tests, benches,
// and external callers) that previously imported these module paths from
// `slicer_runtime::*`. A follow-up packet may delete these once consumers migrate
// to `slicer_scheduler::*` directly. See ADR-0007 for the Static/Live split rationale.
pub use slicer_scheduler::config_resolution;
pub use slicer_scheduler::dag;
pub use slicer_scheduler::dag_cli;
pub use slicer_scheduler::execution_plan;
pub use slicer_scheduler::manifest;
pub use slicer_scheduler::module_search_path;
pub use slicer_scheduler::stage_order;
pub use slicer_scheduler::topology;
pub use slicer_scheduler::validation;

pub use blackboard::{Blackboard, DeferredRetract, DeferredTravelMove, LayerArena};
pub use config_resolution::{
    paint_semantic_namespace_key, resolve_global_config, resolve_per_object_configs,
    resolve_per_paint_semantic_configs, resolve_per_tool_configs, validate_support_layer_heights,
    BoundsDeclaration, ConfigBoundsIndex, ConfigResolutionError, UnknownSemanticWarning,
};
pub use dag::{
    build_global_dag, build_intra_stage_dag, BuiltinProducer, EdgeTo, GlobalEdge, ModuleNode,
    Producer,
};
pub use slicer_ir::{
    BlackboardError, BlackboardPrepassSlot, FinalizationError, FinalizationOutput, LayerArenaError,
    LayerArenaSlot, LayerStageError, LayerStageOutput, PostpassError, PostpassOutput,
    PrepassRunnerError,
};

/// Returns the 8 host built-in producers in their canonical pipeline order.
///
/// These producers represent built-in (non-WASM) pipeline steps that are
/// always present regardless of which WASM modules are loaded. They are used
/// by the DAG validator, `dag_cli`, and the startup validation request.
pub fn runtime_builtins() -> Vec<&'static dyn Producer> {
    use crate::builtins::gcode_emit_producer::GCODE_EMIT_PRODUCER;
    use crate::builtins::mesh_analysis_producer::{MESH_ANALYSIS_PRODUCER, MESH_PRODUCER};
    use crate::builtins::prepass_slice_producer::{SHELL_CLASSIFICATION_PRODUCER, SLICE_PRODUCER};
    use crate::builtins::region_mapping_producer::REGION_MAPPING_PRODUCER;
    use crate::builtins::support_geometry_producer::SUPPORT_GEOMETRY_PRODUCER;

    vec![
        &MESH_PRODUCER as &dyn Producer,
        &MESH_ANALYSIS_PRODUCER as &dyn Producer,
        &REGION_MAPPING_PRODUCER as &dyn Producer,
        &SLICE_PRODUCER as &dyn Producer,
        &SHELL_CLASSIFICATION_PRODUCER as &dyn Producer,
        &SUPPORT_GEOMETRY_PRODUCER as &dyn Producer,
        &GCODE_EMIT_PRODUCER as &dyn Producer,
    ]
}
// Transitional re-exports from slicer-wasm-host (P83 Step 4c+4d). External callers
// (tests, benches, downstream crates) that previously imported these from slicer_runtime
// keep working. Once external callers migrate to slicer_wasm_host::*, this block can shrink.
pub use slicer_wasm_host::{
    CompiledModuleLive, FinalizationStageInput, FinalizationStageRunner, LayerStageInput,
    LayerStageRunner, PostpassStageInput, PostpassStageRunner, PrepassStageInput,
    PrepassStageRunner, WasmComponent, WasmEngine, WasmInstance, WasmInstanceLease,
    WasmInstancePool,
};
// Additional slicer-wasm-host re-exports for test/bench backward compatibility.
pub use slicer_wasm_host::host::HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS;
pub use slicer_wasm_host::{
    build_wasm_instance_pool, HostState, InstancePoolError, InstancePoolMode, WasmArtifactMetadata,
    WasmCallError, WasmLoadError,
};
// Module-path compatibility aliases so tests that used
// `slicer_runtime::wit_host::*`, `slicer_runtime::instance_pool::*`, and
// `slicer_runtime::wasm_instance::*` continue to compile without per-file import rewrites.
pub use slicer_wasm_host::host as wit_host;
pub use slicer_wasm_host::instance as wasm_instance;
pub use slicer_wasm_host::pool as instance_pool;
// HostExecutionContext and HostExecutionContextBuilder are internal to slicer-wasm-host
// (not part of the public API surface); they are NOT re-exported here.

pub use builtins::overhang_annotation_producer::{
    commit_overhang_annotation_builtin, OverhangAnnotationBuiltinError,
};
pub use builtins::prepass_slice_producer::{
    commit_slice_builtin, execute_prepass_slice_all_layers,
};
pub use builtins::support_geometry_producer::commit_support_geometry_builtin;
pub use dag_cli::{
    run_dag_claims, run_dag_depends, run_dag_stage, run_dag_stages, ClaimOut, ClaimsOut,
    DependsOut, GlobalEdgeOut, ModuleOut, StageEdgeOut, StageOut, StageSummary, StagesOut,
};
pub use execution_plan::{
    bind_module_config_view, build_execution_plan, dedup_same_claim_modules_for_test,
    dedup_same_claim_modules_with_wall_generator, parse_cli_config_source, CompiledModuleBuilder,
    CompiledModuleStatic, CompiledStage, ConfigSourceParseError, ExecutionModuleBinding,
    ExecutionPlan, ExecutionPlanError, ExecutionPlanRequest, IrAccessMask, SortedStageModules,
    DEFAULT_REGION_MAP_CAP, DEFAULT_WALL_GENERATOR, MAX_LAYER_INDEX, STAGE_ORDER,
    WALL_GENERATOR_CONFIG_KEY,
};
// Live-path symbols moved to slicer-wasm-host (Step 3.5).
pub use slicer_wasm_host::{
    build_live_execution_plan, load_live_modules_for_plan, load_live_modules_for_plan_with_config,
    LiveModuleBinding, LiveModuleLoadError, LiveModuleLoadOutput,
};
// CompiledModule alias (transitional compat: was deleted by Step 3.5, use CompiledModuleStatic directly).
pub use crate::builtins::region_mapping_producer::{
    commit_region_mapping_builtin, RegionMappingBuiltinError,
};
pub use execution_plan::CompiledModuleStatic as CompiledModule;
pub use instrumentation::{
    compute_serial_edges_for_stage, compute_serial_edges_from_compiled, CompositeInstrumentation,
    EdgeReason, NoopInstrumentation, Phase, PipelineInstrumentation, SerialEdge, TierKind,
};
pub use layer_executor::{
    apply_entity_order_proposal, apply_for_test, project_ordered_entities, OrderedEntityView,
    StageApplyContext,
};
pub use layer_executor::{
    execute_per_layer, execute_per_layer_with_events, execute_per_layer_with_instrumentation,
    ir_path_for_layer_stage, LayerExecutionError, LayerProgressSink, NoopLayerProgressSink,
};
// Typed tap capture (packet 158): request-gated, post-commit IR capture at
// the executor boundary, consumed by `pnp-cli`'s visual-debug command.
pub use layer_executor::{
    execute_captured_stages, CaptureExecutionError, CaptureOutput, CaptureRequest, CapturedIr,
    LayerExpansion, StageCapture, SUPPORTED_TAP_STAGE_IDS,
};
pub use layer_finalization::{
    execute_layer_finalization, execute_layer_finalization_with_instrumentation,
    FinalizationOutputBuilder,
};
// Intermediate renderer (packet 159): pure typed-capture-in, PNG-bytes-out
// rendering, consumed by `pnp-cli`'s visual-debug command.
pub use manifest::{
    build_config_schema_json, load_module_from_paths, load_modules_from_roots, ConfigFieldEntry,
    ConfigSchema, DiagnosticLevel, LoadDiagnostic, LoadError, LoadErrorKind, LoadModulesReport,
    LoadedModule, LoadedModuleBuilder,
};
pub use module_search_path::{assemble_search_roots, SLICER_MODULE_PATH_ENV};
pub use postpass::execute_postpass;
pub use prepass::{
    execute_prepass, execute_prepass_with_builtins, execute_prepass_with_builtins_configured,
    execute_prepass_with_builtins_configured_instr, PrepassExecutionError,
};
pub use progress_instrumentation::ProgressPipelineInstrumentation;
pub use run::{
    prepare_prepass_context, run_slice, PrepassContext, SliceOutcome, SliceRunError,
    SliceRunOptions,
};
pub use slice_postprocess::{
    execute_slice_postprocess_paint_annotation, paint_annotation_warning_to_progress_event,
    paint_annotation_warnings_to_progress_events, SlicePostProcessPaintAnnotationError,
    SlicePostProcessPaintAnnotationRequest, SlicePostProcessPaintAnnotationResult,
    SlicePostProcessPaintAnnotationWarning, SlicePostProcessPaintAnnotationWarningReason,
};
pub use slice_postprocess_prepass::{
    commit_shell_classification_builtin, ShellClassificationError,
};
pub use visual_debug_render::{
    compute_viewport_bounds, render_stage_capture, GeometryView, RenderError, RenderView,
    RenderedImage, ViewportBoundsMm, BASE_DIMENSION_PX,
};
// kept: consumed by crates/slicer-runtime/tests/e2e/threemf_subtypes_synthetic_e2e_tdd.rs:602
pub use slicer_core::algos::region_mapping::execute_region_mapping;
// kept: consumed by crates/slicer-runtime/tests/e2e/threemf_fixture_e2e_tdd.rs, crates/slicer-runtime/tests/integration/region_mapping_tdd.rs
pub use slicer_core::algos::region_mapping::execute_region_mapping_with_cap;
// kept: consumed by crates/slicer-runtime/tests/integration/region_mapping_tdd.rs
pub use slicer_core::algos::region_mapping::RegionMappingError;
pub use slicer_core::{
    FacetAnnotationRecord, FacetClassRecord, MeshAnalysisAuxiliary, PrepassStageOutput,
    SurfaceGroupRecord,
};
// kept: DefaultGCodeEmitter — consumed by tests (dispatch_tdd, postpass_gcode_emit_contract_tdd, pnp-cli/tests/e2e_integration_tdd)
pub use slicer_gcode::DefaultGCodeEmitter;
// kept: DefaultGCodeSerializer — consumed by tests (postpass_gcode_emit_contract_tdd, gcode_skirt_brim_emission_tdd, pnp-cli/tests/e2e_integration_tdd)
pub use slicer_gcode::DefaultGCodeSerializer;
// kept: GCodeEmitError — consumed by tests (dispatch_tdd, postpass_executor_tdd, pipeline_tdd, runtime_wiring_tdd, run_pipeline_with_instrumentation_tdd, paint_annotation_integration_tdd, pnp-cli/tests/e2e_integration_tdd)
pub use slicer_gcode::GCodeEmitError;
// kept: GCodeEmitter — consumed by tests (dispatch_tdd, postpass_gcode_emit_contract_tdd, postpass_executor_tdd, pipeline_tdd, runtime_wiring_tdd, run_pipeline_with_instrumentation_tdd, paint_annotation_integration_tdd, gcode_skirt_brim_emission_tdd, pnp-cli/tests/e2e_integration_tdd)
pub use slicer_gcode::GCodeEmitter;
// kept: GCodeSerializer — consumed by tests (dispatch_tdd, postpass_gcode_emit_contract_tdd, postpass_executor_tdd, pipeline_tdd, runtime_wiring_tdd, run_pipeline_with_instrumentation_tdd, paint_annotation_integration_tdd, gcode_skirt_brim_emission_tdd, pnp-cli/tests/e2e_integration_tdd)
pub use slicer_gcode::GCodeSerializer;
// Re-exports from slicer_core::algos for backward compatibility.
pub use slicer_core::algos::mesh_analysis::{
    execute_mesh_analysis, execute_mesh_analysis_with, MeshAnalysisConfig, MeshAnalysisError,
};
pub use slicer_core::algos::prepass_slice::{execute_prepass_slice_single_layer, LayerSliceError};
pub use slicer_wasm_host::{DispatchError, DispatchPhase, WasmRuntimeDispatcher};
pub use topology::topological_sort;
pub use validation::{
    resolve_held_claims, validate_startup_dag, AccessKind, ClaimHolder, ConflictScope,
    DagValidationDiagnostic, DagValidationPass, DagValidationReport, DagValidationRequest,
    FillHolders, ModuleAccessAudit, SchedulerError, StageDag, FILL_CLAIM_IDS,
};
