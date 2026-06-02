//! Host-side scheduler and manifest ingestion APIs.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod blackboard;
pub mod config_resolution;
pub mod dag;
pub mod dag_cli;
pub mod diagnose;
pub mod execution_plan;
pub mod gcode_emit;
pub mod instrumentation;
pub mod layer_executor;
pub mod layer_finalization;
pub mod manifest;
pub mod mesh_analysis;
pub mod mesh_segmentation;
pub mod module_search_path;
pub mod negative_part_subtract;
pub mod overhang_classifier;
pub mod paint_segmentation;
pub mod pipeline;
pub mod postpass;
pub mod prepass;
pub mod prepass_slice;
pub mod progress_events;
pub mod progress_instrumentation;
pub mod region_mapping;
#[cfg(feature = "report")]
pub mod report;
pub mod run;
pub mod slice_postprocess;
pub mod slice_postprocess_prepass;
pub mod stage_order;
pub mod support_geometry;
pub mod topology;
pub mod validation;

pub use blackboard::{Blackboard, DeferredRetract, DeferredTravelMove, LayerArena};
pub use config_resolution::{
    paint_semantic_namespace_key, resolve_global_config, resolve_per_object_configs,
    resolve_per_paint_semantic_configs, validate_support_layer_heights, BoundsDeclaration,
    ConfigBoundsIndex, ConfigResolutionError, UnknownSemanticWarning,
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
    use crate::gcode_emit::GCODE_EMIT_PRODUCER;
    use crate::mesh_analysis::{MESH_ANALYSIS_PRODUCER, MESH_PRODUCER};
    use crate::paint_segmentation::PAINT_SEGMENTATION_PRODUCER;
    use crate::prepass_slice::{SHELL_CLASSIFICATION_PRODUCER, SLICE_PRODUCER};
    use crate::region_mapping::REGION_MAPPING_PRODUCER;
    use crate::support_geometry::SUPPORT_GEOMETRY_PRODUCER;

    vec![
        &MESH_PRODUCER as &dyn Producer,
        &MESH_ANALYSIS_PRODUCER as &dyn Producer,
        &REGION_MAPPING_PRODUCER as &dyn Producer,
        &SLICE_PRODUCER as &dyn Producer,
        &SHELL_CLASSIFICATION_PRODUCER as &dyn Producer,
        &SUPPORT_GEOMETRY_PRODUCER as &dyn Producer,
        &PAINT_SEGMENTATION_PRODUCER as &dyn Producer,
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

pub use dag_cli::{
    run_dag_claims, run_dag_depends, run_dag_stage, run_dag_stages, ClaimOut, ClaimsOut,
    DependsOut, GlobalEdgeOut, ModuleOut, StageEdgeOut, StageOut, StageSummary, StagesOut,
};
pub use execution_plan::{
    bind_module_config_view, build_execution_plan, build_live_execution_plan,
    dedup_same_claim_modules_for_test, load_live_modules_for_plan, parse_cli_config_source,
    CompiledModule, CompiledModuleBuilder, CompiledModuleStatic, CompiledStage,
    ConfigSourceParseError, ExecutionModuleBinding, ExecutionPlan, ExecutionPlanError,
    ExecutionPlanRequest, IrAccessMask, LiveModuleBinding, LiveModuleLoadError,
    LiveModuleLoadOutput, SortedStageModules, DEFAULT_REGION_MAP_CAP, MAX_LAYER_INDEX, STAGE_ORDER,
};
pub use gcode_emit::{
    serialize_thumbnail_block, tolerance_for_role, DefaultGCodeEmitter, DefaultGCodeSerializer,
    ThumbnailAwareSerializer,
};
pub use instrumentation::{
    compute_serial_edges_for_stage, compute_serial_edges_from_compiled, CompositeInstrumentation,
    EdgeReason, NoopInstrumentation, Phase, PipelineInstrumentation, SerialEdge, TierKind,
};
pub use layer_executor::{
    apply_entity_order_proposal, commit_layer_outputs_for_test, project_ordered_entities,
    OrderedEntityView,
};
pub use layer_executor::{
    execute_per_layer, execute_per_layer_with_events, execute_per_layer_with_instrumentation,
    ir_path_for_layer_stage, LayerExecutionError, LayerProgressSink, NoopLayerProgressSink,
};
pub use layer_finalization::{execute_layer_finalization, FinalizationOutputBuilder};
pub use manifest::{
    build_config_schema_json, load_module_from_paths, load_modules_from_roots, ConfigFieldEntry,
    ConfigSchema, DiagnosticLevel, LoadDiagnostic, LoadError, LoadErrorKind, LoadModulesReport,
    LoadedModule, LoadedModuleBuilder,
};
pub use mesh_analysis::{execute_mesh_analysis, MeshAnalysisConfig, MeshAnalysisError};
pub use mesh_segmentation::{
    execute_mesh_segmentation, DegenerateStrokeReason, MeshSegmentationError,
};
pub use module_search_path::{assemble_search_roots, SLICER_MODULE_PATH_ENV};
pub use paint_segmentation::{execute_paint_segmentation, PaintSegmentationError};
pub use postpass::{execute_postpass, GCodeEmitter, GCodeSerializer};
pub use prepass::{
    execute_prepass, execute_prepass_with_builtins, execute_prepass_with_builtins_configured,
    PrepassExecutionError,
};
pub use prepass_slice::{
    commit_slice_builtin, execute_prepass_slice_all_layers, execute_prepass_slice_single_layer,
    LayerSliceError,
};
pub use progress_instrumentation::ProgressPipelineInstrumentation;
pub use region_mapping::{
    commit_region_mapping_builtin, execute_region_mapping, execute_region_mapping_with_cap,
    RegionMappingBuiltinError, RegionMappingError, TopContributor,
};
pub use run::{run_slice, SliceOutcome, SliceRunError, SliceRunOptions};
pub use slice_postprocess::{
    execute_slice_postprocess_paint_annotation, paint_annotation_warning_to_progress_event,
    paint_annotation_warnings_to_progress_events, SlicePostProcessPaintAnnotationError,
    SlicePostProcessPaintAnnotationRequest, SlicePostProcessPaintAnnotationResult,
    SlicePostProcessPaintAnnotationWarning, SlicePostProcessPaintAnnotationWarningReason,
};
pub use slice_postprocess_prepass::{
    commit_shell_classification_builtin, ShellClassificationError,
};
pub use slicer_core::{
    FacetAnnotationRecord, FacetClassRecord, MeshAnalysisAuxiliary, PrepassStageOutput,
    SurfaceGroupRecord,
};
pub use slicer_wasm_host::{DispatchError, DispatchPhase, WasmRuntimeDispatcher};
pub use support_geometry::{
    commit_support_geometry_builtin, execute_support_geometry, SupportGeometryBuiltinError,
};
pub use topology::topological_sort;
pub use validation::{
    resolve_held_claims, validate_startup_dag, AccessKind, ClaimHolder, ConflictScope,
    DagValidationDiagnostic, DagValidationPass, DagValidationReport, DagValidationRequest,
    FillHolders, ModuleAccessAudit, SchedulerError, StageDag, FILL_CLAIM_IDS,
};
