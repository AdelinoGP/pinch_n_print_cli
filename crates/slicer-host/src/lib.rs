//! Host-side scheduler and manifest ingestion APIs.

#![warn(missing_docs)]
#![warn(unused_imports)]
#![warn(unused_must_use)]

pub mod cli;
pub mod model_loader;
pub mod pipeline;
pub mod blackboard;
pub mod config_schema;
pub mod dag;
pub mod execution_plan;
pub mod gcode_emit;
pub mod instance_pool;
pub mod layer_executor;
pub mod layer_finalization;
pub mod manifest;
pub mod mesh_segmentation;
pub mod paint_segmentation;
pub mod postpass;
pub mod prepass;
pub mod progress_events;
pub mod slice_postprocess;
pub mod topology;
pub mod validation;
pub mod wasm_instance;

pub use cli::{validate_run_options, CliError, HostCli, HostCommands, HostRunOptions};
pub use blackboard::{
    Blackboard, BlackboardError, BlackboardPrepassSlot, LayerArena, LayerArenaError, LayerArenaSlot,
};
pub use config_schema::{
    get_advanced_fields, get_basic_fields, get_field_schema, group_fields_by_ui_group,
    parse_config_schema, query_config_schema, validate_config, validate_field_value,
    ConfigFieldSchema, ConfigFieldType, ConfigSchemaParseError, ConfigSchemaParseErrorKind,
    ConfigUnit, ConfigValidationError, ConfigValidationErrorKind, ConfigValue, CrossValidateRule,
    CrossValidateSeverity, FullConfigSchema,
};
pub use dag::{build_intra_stage_dag, ModuleNode};
pub use execution_plan::{
    build_execution_plan, CompiledModule, CompiledStage, ExecutionModuleBinding, ExecutionPlan,
    ExecutionPlanError, ExecutionPlanRequest, IrAccessMask, SortedStageModules,
};
pub use gcode_emit::{DefaultGCodeEmitter, DefaultGCodeSerializer};
pub use instance_pool::{
    build_wasm_instance_pool, InstancePoolError, InstancePoolMode, WasmArtifactMetadata,
    WasmInstanceLease, WasmInstancePool,
};
pub use layer_executor::{
    execute_per_layer, LayerExecutionError, LayerStageError, LayerStageOutput, LayerStageRunner,
};
pub use layer_finalization::{
    execute_layer_finalization, FinalizationError, FinalizationOutput, FinalizationOutputBuilder,
    FinalizationStageRunner,
};
pub use manifest::{
    load_module_from_paths, load_modules_from_roots, ConfigSchema, DiagnosticLevel, LoadDiagnostic,
    LoadError, LoadErrorKind, LoadModulesReport, LoadedModule,
};
pub use mesh_segmentation::{
    execute_mesh_segmentation, DegenerateStrokeReason, MeshSegmentationError,
};
pub use paint_segmentation::{execute_paint_segmentation, PaintSegmentationError};
pub use postpass::{
    execute_postpass, GCodeEmitter, GCodeSerializer, PostpassError, PostpassOutput,
    PostpassStageRunner,
};
pub use prepass::{execute_prepass, PrepassExecutionError, PrepassStageOutput, PrepassStageRunner};
pub use slice_postprocess::{
    execute_slice_postprocess_paint_annotation, SlicePostProcessPaintAnnotationError,
    SlicePostProcessPaintAnnotationRequest, SlicePostProcessPaintAnnotationResult,
    SlicePostProcessPaintAnnotationWarning, SlicePostProcessPaintAnnotationWarningReason,
};
pub use topology::topological_sort;
pub use validation::{
    validate_startup_dag, AccessKind, ClaimHolder, ConflictScope, DagValidationDiagnostic,
    DagValidationPass, DagValidationReport, DagValidationRequest, ModuleAccessAudit,
    SchedulerError, StageDag,
};
