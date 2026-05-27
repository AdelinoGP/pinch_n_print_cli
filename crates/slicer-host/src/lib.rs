//! Host-side scheduler and manifest ingestion APIs.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod blackboard;
pub mod cli;
pub mod config_resolution;
pub mod dag;
pub mod dag_cli;
pub mod dispatch;
pub mod execution_plan;
pub mod gcode_emit;
pub mod instance_pool;
pub mod instrumentation;
pub mod layer_executor;
pub mod layer_finalization;
pub mod manifest;
pub mod mesh_analysis;
pub mod mesh_segmentation;
pub mod model_loader;
pub mod model_loader_sidecar;
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
pub mod python_bridge;
pub mod region_mapping;
pub mod report;
pub mod slice_postprocess;
pub mod slice_postprocess_prepass;
pub mod support_geometry;
pub mod topology;
pub mod validation;
pub mod wasm_instance;
pub mod wit_host;

pub use blackboard::{
    Blackboard, BlackboardError, BlackboardPrepassSlot, DeferredRetract, DeferredTravelMove,
    LayerArena, LayerArenaError, LayerArenaSlot,
};
pub use cli::{write_with_parents, HostCli, HostCommands, HostRunOptions, OutputFormat};
pub use config_resolution::{
    paint_semantic_namespace_key, resolve_global_config, resolve_per_object_configs,
    resolve_per_paint_semantic_configs, validate_support_layer_heights, BoundsDeclaration,
    ConfigBoundsIndex, ConfigResolutionError, UnknownSemanticWarning,
};
pub use dag::{build_global_dag, build_intra_stage_dag, EdgeTo, GlobalEdge, ModuleNode};
pub use dag_cli::{
    run_dag_claims, run_dag_depends, run_dag_stage, run_dag_stages, ClaimOut, ClaimsOut,
    DependsOut, GlobalEdgeOut, ModuleOut, StageEdgeOut, StageOut, StageSummary, StagesOut,
};
pub use dispatch::{
    apply_entity_order_proposal, commit_layer_outputs_for_test, export_name_for_stage,
    project_ordered_entities, DispatchError, DispatchPhase, OrderedEntityView,
    WasmRuntimeDispatcher,
};
pub use execution_plan::{
    bind_module_config_view, build_execution_plan, build_live_execution_plan,
    dedup_same_claim_modules_for_test, load_live_modules_for_plan, parse_cli_config_source,
    CompiledModule, CompiledModuleBuilder, CompiledStage, ConfigSourceParseError,
    ExecutionModuleBinding, ExecutionPlan, ExecutionPlanError, ExecutionPlanRequest, IrAccessMask,
    LiveModuleBinding, LiveModuleLoadError, LiveModuleLoadOutput, SortedStageModules,
    DEFAULT_REGION_MAP_CAP, MAX_LAYER_INDEX, STAGE_ORDER,
};
pub use gcode_emit::{
    serialize_thumbnail_block, tolerance_for_role, DefaultGCodeEmitter, DefaultGCodeSerializer,
    ThumbnailAwareSerializer,
};
pub use instance_pool::{
    build_wasm_instance_pool, InstancePoolError, InstancePoolMode, WasmArtifactMetadata,
    WasmInstanceLease, WasmInstancePool,
};
pub use instrumentation::{
    compute_serial_edges_for_stage, compute_serial_edges_from_compiled, CompositeInstrumentation,
    EdgeReason, NoopInstrumentation, Phase, PipelineInstrumentation, SerialEdge, TierKind,
};
pub use layer_executor::{
    execute_per_layer, execute_per_layer_with_events, execute_per_layer_with_instrumentation,
    ir_path_for_layer_stage, LayerExecutionError, LayerProgressSink, LayerStageError,
    LayerStageOutput, LayerStageRunner, NoopLayerProgressSink,
};
pub use layer_finalization::{
    execute_layer_finalization, FinalizationError, FinalizationOutput, FinalizationOutputBuilder,
    FinalizationStageRunner,
};
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
pub use postpass::{
    execute_postpass, GCodeEmitter, GCodeSerializer, PostpassError, PostpassOutput,
    PostpassStageRunner,
};
pub use prepass::{
    execute_prepass, execute_prepass_with_builtins, FacetAnnotationRecord, FacetClassRecord,
    MeshAnalysisAuxiliary, PrepassExecutionError, PrepassStageOutput, PrepassStageRunner,
    SurfaceGroupRecord,
};
pub use prepass_slice::{
    commit_slice_builtin, execute_prepass_slice_all_layers, execute_prepass_slice_single_layer,
    LayerSliceError,
};
pub use progress_instrumentation::ProgressPipelineInstrumentation;
pub use python_bridge::{
    PythonBinding, PythonBridge, PythonBridgeError, PythonBridgePhase, PythonPostpassRunner,
};
pub use region_mapping::{
    commit_region_mapping_builtin, execute_region_mapping, execute_region_mapping_with_cap,
    RegionMappingBuiltinError, RegionMappingError, TopContributor,
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
pub use support_geometry::{
    commit_support_geometry_builtin, execute_support_geometry, SupportGeometryBuiltinError,
};
pub use topology::topological_sort;
pub use validation::{
    resolve_held_claims, validate_startup_dag, AccessKind, ClaimHolder, ConflictScope,
    DagValidationDiagnostic, DagValidationPass, DagValidationReport, DagValidationRequest,
    FillHolders, ModuleAccessAudit, SchedulerError, StageDag, FILL_CLAIM_IDS,
};
pub use wasm_instance::{WasmCallError, WasmComponent, WasmEngine, WasmInstance, WasmLoadError};
pub use wit_host::{
    HostExecutionContext, HostExecutionContextBuilder, HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS,
};
