#![allow(missing_docs)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigView, LayerCollectionIR, MeshIR, ObjectMesh, Point3, SemVer, StageId,
    Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_layer_finalization, Blackboard, CompiledModule,
    CompiledModuleBuilder, CompiledModuleLive, CompiledStage, ExecutionModuleBinding,
    ExecutionPlan, FinalizationError, FinalizationOutput, FinalizationStageInput,
    FinalizationStageRunner, LoadedModuleBuilder, WasmArtifactMetadata,
};

#[test]
fn finalization_executor_locks_down_stage_order_and_passes_mut_layers() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let plan = execution_plan_fixture(Some(compiled_stage(
        "PostPass::LayerFinalization",
        &["com.example.finalizer-a", "com.example.finalizer-b"],
    )));

    let runner = ScriptedRunner::new(
        &["com.example.finalizer-a", "com.example.finalizer-b"],
        vec![
            (
                String::from("com.example.finalizer-a"),
                Ok(FinalizationOutput::Success),
            ),
            (
                String::from("com.example.finalizer-b"),
                Ok(FinalizationOutput::Success),
            ),
        ],
    );

    let mut layers = vec![
        layer_collection_fixture(0, 0.2),
        layer_collection_fixture(1, 0.4),
    ];

    execute_layer_finalization(
        &plan,
        &blackboard,
        &runner,
        &mut layers,
        &Default::default(),
    )
    .expect("finalization should run and succeed");

    assert_eq!(
        runner.observed_module_ids(),
        vec![
            String::from("com.example.finalizer-a"),
            String::from("com.example.finalizer-b"),
        ]
    );
}

#[test]
fn finalization_executor_enforces_pool_size_of_1() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let plan = execution_plan_fixture(Some(compiled_stage(
        "PostPass::LayerFinalization",
        &["com.example.finalizer-a"],
    )));

    let runner = ScriptedRunner::new(
        &["com.example.finalizer-a"],
        vec![(
            String::from("com.example.finalizer-a"),
            Ok(FinalizationOutput::Success),
        )],
    );

    let mut layers = Vec::new();
    execute_layer_finalization(
        &plan,
        &blackboard,
        &runner,
        &mut layers,
        &Default::default(),
    )
    .expect("finalization should run and succeed");
}

#[test]
fn finalization_executor_rejects_non_monotonic_layers() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let plan = execution_plan_fixture(Some(compiled_stage(
        "PostPass::LayerFinalization",
        &["com.example.bad-finalizer"],
    )));

    struct BadRunner;
    impl FinalizationStageRunner for BadRunner {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModuleLive<'_>,
            _input: FinalizationStageInput<'_>,
            layers: &mut Vec<LayerCollectionIR>,
        ) -> Result<FinalizationOutput, FinalizationError> {
            layers.swap(0, 1);
            Ok(FinalizationOutput::Success)
        }
    }

    let mut layers = vec![
        layer_collection_fixture(0, 0.2),
        layer_collection_fixture(1, 0.4),
    ];

    let before = layers.clone();
    let recorder = ValidationErrorRecorder::default();
    let result =
        slicer_runtime::layer_finalization::execute_layer_finalization_with_instrumentation(
            &plan,
            &blackboard,
            &BadRunner,
            &mut layers,
            &recorder,
            &Default::default(),
        );
    assert_eq!(
        result,
        Err(FinalizationError::Validation {
            message: "layer indices must be monotonic, found 1 followed by 0".into(),
        })
    );
    assert_eq!(layers, before, "rejected candidate must not commit");
    assert_eq!(
        *recorder.0.lock().unwrap(),
        vec![(
            "PostPass::LayerFinalization".into(),
            None,
            "com.example.bad-finalizer".into(),
            "finalization validation failed: layer indices must be monotonic, found 1 followed by 0".into(),
            true,
        )],
        "validation failure must emit exactly one fatal module_error"
    );
}

type RecordedModuleError = (StageId, Option<u32>, slicer_ir::ModuleId, String, bool);

#[derive(Default)]
struct ValidationErrorRecorder(std::sync::Mutex<Vec<RecordedModuleError>>);

impl slicer_runtime::instrumentation::PipelineInstrumentation for ValidationErrorRecorder {
    fn on_phase_start(&self, _: slicer_runtime::instrumentation::Phase) {}
    fn on_phase_end(&self, _: slicer_runtime::instrumentation::Phase) {}
    fn on_stage_start(&self, _: &StageId, _: Option<u32>) {}
    fn on_stage_end(&self, _: &StageId, _: Option<u32>) {}
    fn on_module_start(&self, _: &StageId, _: Option<u32>, _: &slicer_ir::ModuleId) {}
    fn on_module_end(&self, _: &StageId, _: Option<u32>, _: &slicer_ir::ModuleId, _: u64, _: u64) {}
    fn on_layer_start(&self, _: u32, _: f32) {}
    fn on_layer_end(&self, _: u32) {}
    fn record_edges(
        &self,
        _: &StageId,
        _: slicer_runtime::instrumentation::TierKind,
        _: &[slicer_runtime::instrumentation::SerialEdge],
    ) {
    }
    fn on_module_error(
        &self,
        stage: &StageId,
        layer: Option<u32>,
        module: &slicer_ir::ModuleId,
        message: &str,
        fatal: bool,
    ) {
        self.0
            .lock()
            .unwrap()
            .push((stage.clone(), layer, module.clone(), message.into(), fatal));
    }
}

#[test]
fn finalization_executor_accepts_duplicate_layer_indices() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let plan = execution_plan_fixture(Some(compiled_stage(
        "PostPass::LayerFinalization",
        &["com.example.duplicate-finalizer"],
    )));

    struct DuplicateRunner;
    impl FinalizationStageRunner for DuplicateRunner {
        fn run_stage(
            &self,
            _stage_id: &StageId,
            _module: &CompiledModuleLive<'_>,
            _input: FinalizationStageInput<'_>,
            layers: &mut Vec<LayerCollectionIR>,
        ) -> Result<FinalizationOutput, FinalizationError> {
            layers.push(layer_collection_fixture(0, 0.2));
            Ok(FinalizationOutput::Success)
        }
    }

    let mut layers = vec![layer_collection_fixture(0, 0.2)];

    let result = execute_layer_finalization(
        &plan,
        &blackboard,
        &DuplicateRunner,
        &mut layers,
        &Default::default(),
    );
    // Packet 197 originally rejected duplicates. Since 2026-08-30, duplicates are
    // permits them because ADR-0059 requires a solo synthesized row to adopt
    // its upper model layer's index; the preceding test still rejects reversals.
    assert_eq!(result, Ok(()));
}

#[test]
fn finalization_executor_handles_fatal_error() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let plan = execution_plan_fixture(Some(compiled_stage(
        "PostPass::LayerFinalization",
        &["com.example.fatal-finalizer"],
    )));

    let runner = ScriptedRunner::new(
        &["com.example.fatal-finalizer"],
        vec![(
            String::from("com.example.fatal-finalizer"),
            Err(FinalizationError::FatalModule {
                stage_id: String::from("PostPass::LayerFinalization"),
                module_id: String::from("com.example.fatal-finalizer"),
                message: String::from("simulated failure"),
            }),
        )],
    );

    let mut layers = Vec::new();
    let result = execute_layer_finalization(
        &plan,
        &blackboard,
        &runner,
        &mut layers,
        &Default::default(),
    );
    assert_eq!(
        result,
        Err(FinalizationError::FatalModule {
            stage_id: String::from("PostPass::LayerFinalization"),
            module_id: String::from("com.example.fatal-finalizer"),
            message: String::from("simulated failure"),
        })
    );
}

#[test]
fn finalization_fatal_error_is_instrumented_and_mutating_runner_rolls_back() {
    struct MutatingFailure;
    impl FinalizationStageRunner for MutatingFailure {
        fn run_stage(
            &self,
            stage: &StageId,
            module: &CompiledModuleLive<'_>,
            _input: FinalizationStageInput<'_>,
            layers: &mut Vec<LayerCollectionIR>,
        ) -> Result<FinalizationOutput, FinalizationError> {
            layers.clear();
            Err(FinalizationError::FatalModule {
                stage_id: stage.clone(),
                module_id: module.module_id.clone(),
                message: "annotation denied".into(),
            })
        }
    }
    #[derive(Default)]
    struct Recorder(std::sync::Mutex<Vec<(String, String, bool)>>);
    impl slicer_runtime::instrumentation::PipelineInstrumentation for Recorder {
        fn on_phase_start(&self, _: slicer_runtime::instrumentation::Phase) {}
        fn on_phase_end(&self, _: slicer_runtime::instrumentation::Phase) {}
        fn on_stage_start(&self, _: &StageId, _: Option<u32>) {}
        fn on_stage_end(&self, _: &StageId, _: Option<u32>) {}
        fn on_module_start(&self, _: &StageId, _: Option<u32>, _: &slicer_ir::ModuleId) {}
        fn on_module_end(
            &self,
            _: &StageId,
            _: Option<u32>,
            _: &slicer_ir::ModuleId,
            _: u64,
            _: u64,
        ) {
        }
        fn on_layer_start(&self, _: u32, _: f32) {}
        fn on_layer_end(&self, _: u32) {}
        fn record_edges(
            &self,
            _: &StageId,
            _: slicer_runtime::instrumentation::TierKind,
            _: &[slicer_runtime::instrumentation::SerialEdge],
        ) {
        }
        fn on_module_error(
            &self,
            stage: &StageId,
            _layer: Option<u32>,
            module: &slicer_ir::ModuleId,
            message: &str,
            fatal: bool,
        ) {
            assert!(message.contains("annotation denied"));
            self.0
                .lock()
                .unwrap()
                .push((stage.clone(), module.clone(), fatal));
        }
    }
    let plan = execution_plan_fixture(Some(compiled_stage(
        "PostPass::LayerFinalization",
        &["failing-annotation"],
    )));
    let blackboard = Blackboard::new(Arc::new(mesh_fixture()), 0);
    let mut layers = vec![layer_collection_fixture(0, 0.2)];
    let before = layers.clone();
    let recorder = Recorder::default();
    let error =
        slicer_runtime::layer_finalization::execute_layer_finalization_with_instrumentation(
            &plan,
            &blackboard,
            &MutatingFailure,
            &mut layers,
            &recorder,
            &Default::default(),
        )
        .expect_err("runner fails after mutation");
    assert!(matches!(error, FinalizationError::FatalModule { .. }));
    assert_eq!(layers, before);
    assert_eq!(
        *recorder.0.lock().unwrap(),
        vec![(
            "PostPass::LayerFinalization".into(),
            "failing-annotation".into(),
            true
        )]
    );
}

#[test]
fn finalization_executor_handles_non_fatal_error() {
    let mesh = Arc::new(mesh_fixture());
    let blackboard = Blackboard::new(mesh, 0);
    let plan = execution_plan_fixture(Some(compiled_stage(
        "PostPass::LayerFinalization",
        &["com.example.non-fatal", "com.example.success"],
    )));

    let runner = ScriptedRunner::new(
        &["com.example.non-fatal", "com.example.success"],
        vec![
            (
                String::from("com.example.non-fatal"),
                Ok(FinalizationOutput::NonFatalError {
                    message: String::from("degraded state"),
                }),
            ),
            (
                String::from("com.example.success"),
                Ok(FinalizationOutput::Success),
            ),
        ],
    );

    let mut layers = vec![layer_collection_fixture(0, 0.2)];
    execute_layer_finalization(
        &plan,
        &blackboard,
        &runner,
        &mut layers,
        &Default::default(),
    )
    .expect("should continue after non-fatal error");

    assert_eq!(runner.observed_module_ids().len(), 2);
}

#[derive(Debug)]
struct ScriptedRunner {
    scripted: HashMap<String, Result<FinalizationOutput, FinalizationError>>,
    observed: RefCell<Vec<String>>,
    expected_order: Vec<String>,
}

impl ScriptedRunner {
    fn new(
        expected_order: &[&str],
        scripted: Vec<(String, Result<FinalizationOutput, FinalizationError>)>,
    ) -> Self {
        Self {
            scripted: scripted.into_iter().collect(),
            observed: RefCell::new(Vec::new()),
            expected_order: expected_order
                .iter()
                .map(|value| String::from(*value))
                .collect(),
        }
    }

    fn observed_module_ids(&self) -> Vec<String> {
        self.observed.borrow().clone()
    }
}

impl FinalizationStageRunner for ScriptedRunner {
    fn run_stage(
        &self,
        _stage_id: &StageId,
        module: &CompiledModuleLive<'_>,
        _input: FinalizationStageInput<'_>,
        _layers: &mut Vec<LayerCollectionIR>,
    ) -> Result<FinalizationOutput, FinalizationError> {
        assert_eq!(
            module.instance_pool.size(),
            1,
            "Pool size must be 1 for finalization modules"
        );

        let mut observed = self.observed.borrow_mut();
        let next_index = observed.len();
        if let Some(expected_module_id) = self.expected_order.get(next_index) {
            assert_eq!(module.module_id.as_str(), expected_module_id.as_str());
        }
        observed.push(module.module_id.to_string());
        drop(observed);

        self.scripted
            .get(module.module_id.as_str())
            .cloned()
            .expect("runner fixture should define every module outcome")
    }
}

fn execution_plan_fixture(layer_finalization_stage: Option<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        layer_finalization_stage,
        ..Default::default()
    }
}

fn compiled_stage(stage_id: &str, module_ids: &[&str]) -> CompiledStage {
    CompiledStage {
        stage_id: String::from(stage_id),
        modules: module_ids
            .iter()
            .map(|module_id| compiled_module(stage_id, module_id))
            .collect(),
    }
}

fn compiled_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded_module = loaded_module(module_id, stage_id);
    let _instance_pool = Arc::new(
        build_wasm_instance_pool(
            loaded_module.id(),
            loaded_module.stage(),
            loaded_module.layer_parallel_safe(),
            8, // Request 8, but pool mode should force 1 for finalization
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );

    let binding = ExecutionModuleBinding {
        module: loaded_module,
        config_view: Arc::new(ConfigView::new()),
    };

    CompiledModuleBuilder::new(binding.module.id().to_string()).build()
}

fn loaded_module(id: &str, stage: &str) -> slicer_runtime::LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        slicer_schema::TIER_FINALIZATION,
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build()
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("cube"),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![],
                indices: vec![],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: slicer_ir::ObjectConfig {
                data: HashMap::new(),
            },
            world_z_extent: None,
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

fn layer_collection_fixture(index: u32, z: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: index,
        z,
        ordered_entities: Vec::new(),
        ..Default::default()
    }
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}
