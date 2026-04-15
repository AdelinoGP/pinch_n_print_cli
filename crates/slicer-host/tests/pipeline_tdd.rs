//! TDD tests for TASK-075: Main Entry Point / Pipeline Orchestration.
//!
//! These tests verify the `run_pipeline` orchestration function that ties together
//! all host scheduler stages (prepass → per-layer → finalization → postpass).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_host::pipeline::{run_pipeline, PipelineConfig, PipelineError, PipelineStageRunners};
use slicer_host::{
    build_wasm_instance_pool, Blackboard, CompiledModule, CompiledStage, ConfigSchema,
    ExecutionPlan, FinalizationError, FinalizationOutput, FinalizationStageRunner, GCodeEmitter,
    GCodeSerializer, IrAccessMask, LayerArena, LayerStageError, LayerStageOutput, LayerStageRunner,
    LoadedModule, PostpassError, PostpassOutput, PostpassStageRunner, PrepassExecutionError,
    PrepassStageOutput, PrepassStageRunner, WasmArtifactMetadata,
};
use slicer_ir::{
    BoundingBox3, ConfigView, GCodeIR, GlobalLayer, LayerCollectionIR, LayerPlanIR, MeshIR, Point3,
    PrintMetadata, SemVer, StageId,
};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects: Vec::new(),
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
    })
}

fn empty_execution_plan() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
    }
}

fn minimal_gcode_ir() -> GCodeIR {
    GCodeIR {
        schema_version: semver(1, 0, 0),
        commands: Vec::new(),
        metadata: PrintMetadata {
            slicer_version: "test".into(),
            estimated_print_time_s: 0,
            filament_used_mm: Vec::new(),
            layer_count: 0,
        },
    }
}

fn make_global_layer(index: u32, z: f32) -> GlobalLayer {
    GlobalLayer {
        index,
        z,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    }
}

/// A no-op stage runners that all succeed.
struct NoopPrepassRunner;
impl PrepassStageRunner for NoopPrepassRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
    ) -> Result<PrepassStageOutput, PrepassExecutionError> {
        Ok(PrepassStageOutput::None)
    }
}

struct NoopLayerRunner;
impl LayerStageRunner for NoopLayerRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _arena: &mut LayerArena,
    ) -> Result<LayerStageOutput, LayerStageError> {
        Ok(LayerStageOutput::Success)
    }
}

struct NoopFinalizationRunner;
impl FinalizationStageRunner for NoopFinalizationRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        Ok(FinalizationOutput::Success)
    }
}

struct NoopPostpassRunner;
impl PostpassStageRunner for NoopPostpassRunner {
    fn run_gcode_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        _gcode_ir: &mut GCodeIR,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::GCodeSuccess)
    }

    fn run_text_postprocess(
        &self,
        _stage_id: &StageId,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        text: String,
    ) -> Result<PostpassOutput, PostpassError> {
        Ok(PostpassOutput::TextSuccess { text })
    }
}

struct MinimalEmitter;
impl GCodeEmitter for MinimalEmitter {
    fn emit_gcode(
        &self,
        _layer_irs: &[LayerCollectionIR],
        _blackboard: &Blackboard,
    ) -> Result<GCodeIR, PostpassError> {
        Ok(minimal_gcode_ir())
    }
}

struct MinimalSerializer;
impl GCodeSerializer for MinimalSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
        Ok(String::new())
    }
}

fn noop_runners() -> PipelineStageRunners {
    PipelineStageRunners {
        prepass: Box::new(NoopPrepassRunner),
        layer: Box::new(NoopLayerRunner),
        finalization: Box::new(NoopFinalizationRunner),
        postpass: Box::new(NoopPostpassRunner),
        emitter: Box::new(MinimalEmitter),
        serializer: Box::new(MinimalSerializer),
    }
}

fn make_dummy_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModule {
        id: module_id.into(),
        version: semver(1, 0, 0),
        stage: stage_id.into(),
        wit_world: "slicer:world-prepass@1.0.0".into(),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: ConfigSchema::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: false,
        wasm_path: PathBuf::from(format!("fixtures/{module_id}.wasm")),
        placeholder_wasm: false,
    };
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );
    CompiledModule {
        module_id: module_id.into(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(ConfigView::new()),
        wasm_component: None,
    }
}

// ---------- Test 1: empty modules produces empty gcode ----------
#[test]
fn run_pipeline_empty_modules() {
    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan: empty_execution_plan(),
        runners: noop_runners(),
    };

    let result = run_pipeline(config);
    assert!(result.is_ok(), "empty pipeline should succeed");
    let output = result.unwrap();
    // With no layers and no modules, gcode should be empty or minimal
    assert!(
        output.gcode_text.is_empty() || output.gcode_text.len() < 100,
        "empty pipeline should produce minimal gcode"
    );
}

// ---------- Test 2: pipeline returns gcode string ----------
#[test]
fn run_pipeline_returns_gcode_string() {
    struct MarkerSerializer;
    impl GCodeSerializer for MarkerSerializer {
        fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
            Ok("G28 ; home\nG1 X10 Y10\n".into())
        }
    }

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan: empty_execution_plan(),
        runners: PipelineStageRunners {
            prepass: Box::new(NoopPrepassRunner),
            layer: Box::new(NoopLayerRunner),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MarkerSerializer),
        },
    };

    let output = run_pipeline(config).unwrap();
    assert_eq!(output.gcode_text, "G28 ; home\nG1 X10 Y10\n");
}

// ---------- Test 3: pipeline propagates prepass error ----------
#[test]
fn run_pipeline_propagates_prepass_error() {
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::MeshAnalysis".into(),
            modules: vec![make_dummy_module("PrePass::MeshAnalysis", "test-mod")],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
    };

    struct FailingPrepass;
    impl PrepassStageRunner for FailingPrepass {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModule,
            _blackboard: &Blackboard,
        ) -> Result<PrepassStageOutput, PrepassExecutionError> {
            Err(PrepassExecutionError::FatalModule {
                stage_id: "PrePass::MeshAnalysis".into(),
                module_id: "test-mod".into(),
                message: "prepass boom".into(),
            })
        }
    }

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(FailingPrepass),
            layer: Box::new(NoopLayerRunner),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(result.is_err());
    match result.unwrap_err() {
        PipelineError::Prepass(_) => {} // expected
        other => panic!("expected PipelineError::Prepass, got {:?}", other),
    }
}

// ---------- Test 4: pipeline propagates layer execution error ----------
#[test]
fn run_pipeline_propagates_layer_error() {
    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Slice".into(),
            modules: vec![make_dummy_module("Layer::Slice", "slice-mod")],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![make_global_layer(0, 0.2)]),
        region_plans: Arc::new(HashMap::new()),
    };

    struct FailingLayerRunner;
    impl LayerStageRunner for FailingLayerRunner {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _layer: &GlobalLayer,
            _module: &CompiledModule,
            _blackboard: &Blackboard,
            _arena: &mut LayerArena,
        ) -> Result<LayerStageOutput, LayerStageError> {
            Err(LayerStageError::FatalModule {
                stage_id: "Layer::Slice".into(),
                module_id: "slice-mod".into(),
                message: "layer boom".into(),
            })
        }
    }

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(NoopPrepassRunner),
            layer: Box::new(FailingLayerRunner),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(result.is_err());
    match result.unwrap_err() {
        PipelineError::LayerExecution(_) => {} // expected
        other => panic!("expected PipelineError::LayerExecution, got {:?}", other),
    }
}

// ---------- Test 5: pipeline propagates postpass error ----------
#[test]
fn run_pipeline_propagates_postpass_error() {
    struct FailingEmitter;
    impl GCodeEmitter for FailingEmitter {
        fn emit_gcode(
            &self,
            _layer_irs: &[LayerCollectionIR],
            _blackboard: &Blackboard,
        ) -> Result<GCodeIR, PostpassError> {
            Err(PostpassError::GCodeEmit {
                message: "emit boom".into(),
            })
        }
    }

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan: empty_execution_plan(),
        runners: PipelineStageRunners {
            prepass: Box::new(NoopPrepassRunner),
            layer: Box::new(NoopLayerRunner),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(FailingEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(result.is_err());
    match result.unwrap_err() {
        PipelineError::Postpass(_) => {} // expected
        other => panic!("expected PipelineError::Postpass, got {:?}", other),
    }
}

// ---------- Test 6: pipeline calls stages in order ----------
#[test]
fn run_pipeline_calls_stages_in_order() {
    let call_log: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    struct OrderTrackingPrepass(Arc<Mutex<Vec<String>>>);
    impl PrepassStageRunner for OrderTrackingPrepass {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModule,
            _blackboard: &Blackboard,
        ) -> Result<PrepassStageOutput, PrepassExecutionError> {
            self.0.lock().unwrap().push("prepass".into());
            Ok(PrepassStageOutput::None)
        }
    }

    struct OrderTrackingLayer(Arc<Mutex<Vec<String>>>);
    impl LayerStageRunner for OrderTrackingLayer {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _layer: &GlobalLayer,
            _module: &CompiledModule,
            _blackboard: &Blackboard,
            _arena: &mut LayerArena,
        ) -> Result<LayerStageOutput, LayerStageError> {
            self.0.lock().unwrap().push("per_layer".into());
            Ok(LayerStageOutput::Success)
        }
    }

    struct OrderTrackingFinalization(Arc<Mutex<Vec<String>>>);
    impl FinalizationStageRunner for OrderTrackingFinalization {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModule,
            _blackboard: &Blackboard,
            _layers: &mut Vec<LayerCollectionIR>,
        ) -> Result<FinalizationOutput, FinalizationError> {
            self.0.lock().unwrap().push("finalization".into());
            Ok(FinalizationOutput::Success)
        }
    }

    struct OrderTrackingEmitter(Arc<Mutex<Vec<String>>>);
    impl GCodeEmitter for OrderTrackingEmitter {
        fn emit_gcode(
            &self,
            _layer_irs: &[LayerCollectionIR],
            _blackboard: &Blackboard,
        ) -> Result<GCodeIR, PostpassError> {
            self.0.lock().unwrap().push("postpass".into());
            Ok(minimal_gcode_ir())
        }
    }

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::MeshAnalysis".into(),
            modules: vec![make_dummy_module("PrePass::MeshAnalysis", "prepass-mod")],
        }],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Slice".into(),
            modules: vec![make_dummy_module("Layer::Slice", "layer-mod")],
        }],
        layer_finalization_stage: Some(CompiledStage {
            stage_id: "PostPass::LayerFinalization".into(),
            modules: vec![make_dummy_module(
                "PostPass::LayerFinalization",
                "final-mod",
            )],
        }),
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![make_global_layer(0, 0.2)]),
        region_plans: Arc::new(HashMap::new()),
    };

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(OrderTrackingPrepass(call_log.clone())),
            layer: Box::new(OrderTrackingLayer(call_log.clone())),
            finalization: Box::new(OrderTrackingFinalization(call_log.clone())),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(OrderTrackingEmitter(call_log.clone())),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(result.is_ok(), "pipeline should succeed: {:?}", result);

    let log = call_log.lock().unwrap();
    assert_eq!(
        *log,
        vec!["prepass", "per_layer", "finalization", "postpass"],
        "stages must run in order: prepass → per_layer → finalization → postpass"
    );
}

// ---------- Test 7: pipeline propagates finalization error ----------
#[test]
fn run_pipeline_propagates_finalization_error() {
    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: Some(CompiledStage {
            stage_id: "PostPass::LayerFinalization".into(),
            modules: vec![make_dummy_module(
                "PostPass::LayerFinalization",
                "final-mod",
            )],
        }),
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
    };

    struct FailingFinalization;
    impl FinalizationStageRunner for FailingFinalization {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModule,
            _blackboard: &Blackboard,
            _layers: &mut Vec<LayerCollectionIR>,
        ) -> Result<FinalizationOutput, FinalizationError> {
            Err(FinalizationError::FatalModule {
                stage_id: "PostPass::LayerFinalization".into(),
                module_id: "final-mod".into(),
                message: "finalization boom".into(),
            })
        }
    }

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(NoopPrepassRunner),
            layer: Box::new(NoopLayerRunner),
            finalization: Box::new(FailingFinalization),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(result.is_err());
    match result.unwrap_err() {
        PipelineError::Finalization(_) => {} // expected
        other => panic!("expected PipelineError::Finalization, got {:?}", other),
    }
}

// ---------- Test 8: pipeline with layers produces layer output ----------
#[test]
fn run_pipeline_with_layers_produces_output() {
    struct CountingSerializer;
    impl GCodeSerializer for CountingSerializer {
        fn serialize_gcode(&self, gcode_ir: &GCodeIR) -> Result<String, PostpassError> {
            Ok(format!("layers:{}", gcode_ir.metadata.layer_count))
        }
    }

    struct LayerCountEmitter;
    impl GCodeEmitter for LayerCountEmitter {
        fn emit_gcode(
            &self,
            layer_irs: &[LayerCollectionIR],
            _blackboard: &Blackboard,
        ) -> Result<GCodeIR, PostpassError> {
            let mut ir = minimal_gcode_ir();
            ir.metadata.layer_count = layer_irs.len() as u32;
            Ok(ir)
        }
    }

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![
            make_global_layer(0, 0.2),
            make_global_layer(1, 0.4),
            make_global_layer(2, 0.6),
        ]),
        region_plans: Arc::new(HashMap::new()),
    };

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(NoopPrepassRunner),
            layer: Box::new(NoopLayerRunner),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(LayerCountEmitter),
            serializer: Box::new(CountingSerializer),
        },
    };

    let output = run_pipeline(config).unwrap();
    assert_eq!(output.gcode_text, "layers:3");
}

// ---------- Test 9: prepass LayerPlanIR is promoted into global_layers ----------
/// Regression guard: `run_pipeline_with_events` must promote the `LayerPlanIR`
/// committed by a prepass runner into `plan.global_layers` before the per-layer
/// loop runs. This is the real production scenario: the execution plan is built
/// before prepass runs (global_layers = []), so the pipeline must read the layer
/// schedule from the blackboard after prepass and update the plan accordingly.
#[test]
fn run_pipeline_prepass_layer_plan_promotes_global_layers() {
    struct LayerPlanPrepass;
    impl PrepassStageRunner for LayerPlanPrepass {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModule,
            _blackboard: &Blackboard,
        ) -> Result<PrepassStageOutput, PrepassExecutionError> {
            Ok(PrepassStageOutput::LayerPlan(Arc::new(LayerPlanIR {
                schema_version: SemVer { major: 1, minor: 0, patch: 0 },
                global_layers: vec![
                    make_global_layer(0, 0.2),
                    make_global_layer(1, 0.4),
                ],
                object_participation: HashMap::new(),
            })))
        }
    }

    let layer_call_count: Arc<Mutex<u32>> = Arc::new(Mutex::new(0));

    struct CountingLayerRunner(Arc<Mutex<u32>>);
    impl LayerStageRunner for CountingLayerRunner {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _layer: &GlobalLayer,
            _module: &CompiledModule,
            _blackboard: &Blackboard,
            _arena: &mut LayerArena,
        ) -> Result<LayerStageOutput, LayerStageError> {
            *self.0.lock().unwrap() += 1;
            Ok(LayerStageOutput::Success)
        }
    }

    // plan.global_layers starts empty — simulates the real production path where
    // main.rs builds the plan before prepass runs.
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::LayerPlanning".into(),
            modules: vec![make_dummy_module("PrePass::LayerPlanning", "layer-planner")],
        }],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Slice".into(),
            modules: vec![make_dummy_module("Layer::Slice", "slice-mod")],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()), // empty — must be filled by promotion
        region_plans: Arc::new(HashMap::new()),
    };

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(LayerPlanPrepass),
            layer: Box::new(CountingLayerRunner(layer_call_count.clone())),
            finalization: Box::new(NoopFinalizationRunner),
            postpass: Box::new(NoopPostpassRunner),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(result.is_ok(), "pipeline must succeed after LayerPlanIR promotion: {:?}", result);

    // 2 layers × 1 module in Layer::Slice = 2 runner calls
    assert_eq!(
        *layer_call_count.lock().unwrap(),
        2,
        "per-layer runner must be called once per promoted layer (2 layers × 1 module)"
    );
}
