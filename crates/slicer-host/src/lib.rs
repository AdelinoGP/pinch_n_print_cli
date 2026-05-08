//! Host-side scheduler and manifest ingestion APIs.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod blackboard;
pub mod cli;
pub mod config_resolution;
pub mod config_schema;
pub mod dag;
pub mod dispatch;
pub mod dispatch_helpers;
pub mod execution_plan;
pub mod gcode_emit;
pub mod instance_pool;
pub mod layer_executor;
pub mod layer_finalization;
pub mod layer_slice;
pub mod manifest;
pub mod mesh_analysis;
pub mod mesh_segmentation;
pub mod model_loader;
pub mod paint_segmentation;
pub mod pipeline;
pub mod postpass;
pub mod prepass;
pub mod progress_events;
pub mod python_bridge;
pub mod region_mapping;
pub mod slice_postprocess;
pub mod support_geometry;
pub mod topology;
pub mod validation;
pub mod wasm_instance;
pub mod wit_host;

pub use blackboard::{
    Blackboard, BlackboardError, BlackboardPrepassSlot, DeferredRetract, DeferredTravelMove,
    LayerArena, LayerArenaError, LayerArenaSlot,
};
pub use cli::{validate_run_options, CliError, HostCli, HostCommands, HostRunOptions};
pub use config_resolution::{
    resolve_global_config, resolve_per_object_configs, ConfigResolutionError,
};
pub use config_schema::{
    build_config_schema_json, get_advanced_fields, get_basic_fields, get_field_schema,
    group_fields_by_ui_group, parse_config_schema, query_config_schema, validate_config,
    validate_field_value, ConfigFieldSchema, ConfigFieldType, ConfigSchemaParseError,
    ConfigSchemaParseErrorKind, ConfigUnit, ConfigValidationError, ConfigValidationErrorKind,
    ConfigValue, CrossValidateRule, CrossValidateSeverity, FullConfigSchema,
};
pub use dag::{build_intra_stage_dag, ModuleNode};
pub use dispatch::{
    apply_entity_order_proposal, commit_layer_outputs_for_test, export_name_for_stage,
    project_ordered_entities, DispatchError, DispatchPhase, OrderedEntityView,
    WasmRuntimeDispatcher,
};
pub use execution_plan::{
    bind_module_config_view, build_execution_plan, build_live_execution_plan,
    dedup_same_claim_modules_for_test, load_live_modules_for_plan, parse_cli_config_source,
    CompiledModule, CompiledStage, ConfigSourceParseError, ExecutionModuleBinding, ExecutionPlan,
    ExecutionPlanError, ExecutionPlanRequest, IrAccessMask, LiveModuleBinding, LiveModuleLoadError,
    LiveModuleLoadOutput, SortedStageModules, DEFAULT_REGION_MAP_CAP, MAX_LAYER_INDEX, STAGE_ORDER,
};
pub use gcode_emit::{DefaultGCodeEmitter, DefaultGCodeSerializer};
pub use instance_pool::{
    build_wasm_instance_pool, InstancePoolError, InstancePoolMode, WasmArtifactMetadata,
    WasmInstanceLease, WasmInstancePool,
};
pub use layer_executor::{
    execute_per_layer, execute_per_layer_with_events, ir_path_for_layer_stage, LayerExecutionError,
    LayerProgressSink, LayerStageError, LayerStageOutput, LayerStageRunner, NoopLayerProgressSink,
};
pub use layer_finalization::{
    execute_layer_finalization, FinalizationError, FinalizationOutput, FinalizationOutputBuilder,
    FinalizationStageRunner,
};
pub use layer_slice::{execute_layer_slice, LayerSliceError};
pub use manifest::{
    load_module_from_paths, load_modules_from_roots, ConfigFieldEntry, ConfigSchema,
    DiagnosticLevel, LoadDiagnostic, LoadError, LoadErrorKind, LoadModulesReport, LoadedModule,
};
pub use mesh_analysis::{execute_mesh_analysis, MeshAnalysisConfig, MeshAnalysisError};
pub use mesh_segmentation::{
    execute_mesh_segmentation, DegenerateStrokeReason, MeshSegmentationError,
};
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
pub use python_bridge::{
    PythonBinding, PythonBridge, PythonBridgeError, PythonBridgePhase, PythonPostpassRunner,
};
pub use region_mapping::{
    commit_region_mapping_builtin, execute_region_mapping, execute_region_mapping_with_cap,
    RegionMappingBuiltinError, RegionMappingError,
};
pub use slice_postprocess::{
    execute_slice_postprocess_paint_annotation, paint_annotation_warning_to_progress_event,
    SlicePostProcessPaintAnnotationError, SlicePostProcessPaintAnnotationRequest,
    SlicePostProcessPaintAnnotationResult, SlicePostProcessPaintAnnotationWarning,
    SlicePostProcessPaintAnnotationWarningReason,
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
pub use wit_host::HOST_GET_ORDERED_ENTITIES_TOTAL_CALLS;
