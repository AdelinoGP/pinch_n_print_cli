#![allow(missing_docs)]

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::PrepassRunnerError;
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, ExPolygon, GlobalLayer, LayerPlanIR, MeshIR,
    ModuleInvocation, ObjectLayerRef, ObjectMesh, ObjectSurfaceData, Point2, Point3, RegionKey,
    RegionMapIR, RegionPlan, SemVer, SurfaceClassificationIR, Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass, Blackboard, BlackboardError, BlackboardPrepassSlot,
    CompiledModule, CompiledModuleBuilder, CompiledModuleLive, CompiledStage,
    ExecutionModuleBinding, ExecutionPlan, IrAccessMask, LoadedModuleBuilder,
    PrepassExecutionError, PrepassStageInput, PrepassStageOutput, PrepassStageRunner,
    WasmArtifactMetadata,
};

#[test]
fn prepass_executor_locks_down_stage_order_full_commit_set_and_shared_mesh_input() {
    let mesh = Arc::new(mesh_fixture());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
    let plan = execution_plan_fixture(vec![
        compiled_stage("PrePass::MeshAnalysis", &["com.example.mesh-analysis"]),
        compiled_stage("PrePass::LayerPlanning", &["com.example.layer-planning"]),
        compiled_stage("PrePass::RegionMapping", &["com.example.region-mapping"]),
    ]);

    let runner = ScriptedRunner::new(
        &[
            "com.example.mesh-analysis",
            "com.example.layer-planning",
            "com.example.region-mapping",
        ],
        vec![
            (
                String::from("com.example.mesh-analysis"),
                Ok(PrepassStageOutput::SurfaceClassification(Arc::new(
                    surface_fixture(),
                ))),
            ),
            (
                String::from("com.example.layer-planning"),
                Ok(PrepassStageOutput::LayerPlan(
                    Arc::new(layer_plan_fixture()),
                )),
            ),
            (
                String::from("com.example.region-mapping"),
                Ok(PrepassStageOutput::RegionMap(
                    Arc::new(region_map_fixture()),
                )),
            ),
        ],
        Arc::as_ptr(&mesh) as usize,
    );

    let _audits = execute_prepass(&plan, &mut blackboard, &runner, &Default::default())
        .expect("prepass executor should run fixed stage order and commit each output once");

    assert_eq!(
        runner.observed_module_ids(),
        vec![
            String::from("com.example.mesh-analysis"),
            String::from("com.example.layer-planning"),
            String::from("com.example.region-mapping"),
        ]
    );
    assert!(Arc::ptr_eq(blackboard.mesh(), &mesh));
    assert!(blackboard.surface_classification().is_some());
    assert!(blackboard.layer_plan().is_some());
    assert!(blackboard.region_map().is_some());
}

#[test]
fn prepass_executor_surfaces_duplicate_commit_as_a_deterministic_blackboard_error() {
    let mesh = Arc::new(mesh_fixture());
    let mut blackboard = Blackboard::new(mesh, 0);
    let plan = execution_plan_fixture(vec![compiled_stage(
        "PrePass::MeshAnalysis",
        &["com.example.mesh-analysis.a", "com.example.mesh-analysis.b"],
    )]);

    let runner = ScriptedRunner::new(
        &["com.example.mesh-analysis.a", "com.example.mesh-analysis.b"],
        vec![
            (
                String::from("com.example.mesh-analysis.a"),
                Ok(PrepassStageOutput::SurfaceClassification(Arc::new(
                    surface_fixture(),
                ))),
            ),
            (
                String::from("com.example.mesh-analysis.b"),
                Ok(PrepassStageOutput::SurfaceClassification(Arc::new(
                    surface_fixture(),
                ))),
            ),
        ],
        0,
    );

    assert_eq!(
        execute_prepass(&plan, &mut blackboard, &runner, &Default::default()),
        Err(PrepassExecutionError::Blackboard {
            stage_id: String::from("PrePass::MeshAnalysis"),
            module_id: String::from("com.example.mesh-analysis.b"),
            source: BlackboardError::DuplicatePrepassCommit {
                slot: BlackboardPrepassSlot::SurfaceClassification,
            },
        })
    );
}

#[test]
fn prepass_executor_rejects_missing_required_prepass_before_running_dependent_stage() {
    let mesh = Arc::new(mesh_fixture());
    let mut blackboard = Blackboard::new(mesh, 0);
    // PrePass::LayerPlanning requires SurfaceClassification.
    // Attempt to run it without SurfaceClassification should surface MissingRequiredPrepass.
    let plan = execution_plan_fixture(vec![compiled_stage(
        "PrePass::LayerPlanning",
        &["com.example.layer-planning"],
    )]);

    let runner = ScriptedRunner::new(
        &["com.example.layer-planning"],
        vec![(
            String::from("com.example.layer-planning"),
            Ok(PrepassStageOutput::LayerPlan(
                Arc::new(layer_plan_fixture()),
            )),
        )],
        0,
    );

    assert_eq!(
        execute_prepass(&plan, &mut blackboard, &runner, &Default::default()),
        Err(PrepassExecutionError::MissingRequiredPrepass {
            stage_id: String::from("PrePass::LayerPlanning"),
            slot: BlackboardPrepassSlot::SurfaceClassification,
        })
    );
    assert!(runner.observed_module_ids().is_empty());
}

#[test]
fn prepass_executor_aborts_on_fatal_module_failure_without_running_later_stages() {
    let mesh = Arc::new(mesh_fixture());
    let mut blackboard = Blackboard::new(mesh, 0);
    let plan = execution_plan_fixture(vec![
        compiled_stage("PrePass::MeshAnalysis", &["com.example.mesh-analysis"]),
        compiled_stage("PrePass::LayerPlanning", &["com.example.layer-planning"]),
    ]);

    let runner = ScriptedRunner::new(
        &["com.example.mesh-analysis", "com.example.layer-planning"],
        vec![
            (
                String::from("com.example.mesh-analysis"),
                Err(PrepassRunnerError::FatalModule {
                    stage_id: String::from("PrePass::MeshAnalysis"),
                    module_id: String::from("com.example.mesh-analysis"),
                    message: String::from("fatal contract failure"),
                }),
            ),
            (
                String::from("com.example.layer-planning"),
                Ok(PrepassStageOutput::LayerPlan(
                    Arc::new(layer_plan_fixture()),
                )),
            ),
        ],
        0,
    );

    assert_eq!(
        execute_prepass(&plan, &mut blackboard, &runner, &Default::default()),
        Err(PrepassExecutionError::FatalModule {
            stage_id: String::from("PrePass::MeshAnalysis"),
            module_id: String::from("com.example.mesh-analysis"),
            message: String::from("fatal contract failure"),
        })
    );
    assert_eq!(
        runner.observed_module_ids(),
        vec![String::from("com.example.mesh-analysis")]
    );
    assert!(blackboard.surface_classification().is_none());
    assert!(blackboard.layer_plan().is_none());
}

#[derive(Debug)]
struct ScriptedRunner {
    expected_mesh_ptr: usize,
    scripted: HashMap<String, Result<PrepassStageOutput, PrepassRunnerError>>,
    observed: RefCell<Vec<String>>,
    expected_order: Vec<String>,
}

impl ScriptedRunner {
    fn new(
        expected_order: &[&str],
        scripted: Vec<(String, Result<PrepassStageOutput, PrepassRunnerError>)>,
        expected_mesh_ptr: usize,
    ) -> Self {
        Self {
            expected_mesh_ptr,
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

impl PrepassStageRunner for ScriptedRunner {
    fn run_stage(
        &self,
        _stage_id: &slicer_ir::StageId,
        module: &CompiledModuleLive<'_>,
        input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        let observed_mesh_ptr = Arc::as_ptr(&input.mesh) as usize;
        if self.expected_mesh_ptr != 0 {
            assert_eq!(observed_mesh_ptr, self.expected_mesh_ptr);
        }

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

fn execution_plan_fixture(prepass_stages: Vec<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages,
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: true,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
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
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );

    let binding = ExecutionModuleBinding {
        module: loaded_module,
        config_view: Arc::new(ConfigView::from_map(HashMap::from([(
            String::from("fixture.enabled"),
            ConfigValue::Bool(true),
        )]))),
    };

    CompiledModuleBuilder::new(binding.module.id().to_string())
        .ir_read_mask(IrAccessMask {
            paths: binding.module.ir_reads().to_vec(),
        })
        .ir_write_mask(IrAccessMask {
            paths: binding.module.ir_writes().to_vec(),
        })
        .config_view(Arc::clone(&binding.config_view))
        .build()
}

fn loaded_module(id: &str, stage: &str) -> slicer_runtime::LoadedModule {
    let ir_reads = match stage {
        "PrePass::MeshAnalysis" => vec![String::from("MeshIR.objects")],
        "PrePass::LayerPlanning" => vec![
            String::from("MeshIR.objects"),
            String::from("SurfaceClassificationIR.per_object"),
        ],
        "PrePass::RegionMapping" => vec![
            String::from("LayerPlanIR.global_layers"),
            String::from("ResolvedConfig.global"),
        ],
        _ => Vec::new(),
    };
    let ir_writes = match stage {
        "PrePass::MeshAnalysis" => vec![String::from("SurfaceClassificationIR.per_object")],
        "PrePass::LayerPlanning" => vec![String::from("LayerPlanIR.global_layers")],
        "PrePass::RegionMapping" => vec![String::from("RegionMapIR.entries")],
        _ => Vec::new(),
    };
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        slicer_schema::WORLD_PREPASS,
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .ir_reads(ir_reads)
    .ir_writes(ir_writes)
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build()
}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("cube"),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        x: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        y: 1.0,
                        ..Default::default()
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: identity4(),
            },
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3::default(),
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    }
}

fn surface_fixture() -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        per_object: HashMap::from([(
            String::from("cube"),
            ObjectSurfaceData {
                facet_classes: vec![slicer_ir::FacetClass::TopSurface],
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

fn layer_plan_fixture() -> LayerPlanIR {
    LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: true,
        }],
        object_participation: HashMap::from([(
            String::from("cube"),
            vec![ObjectLayerRef {
                local_layer_index: 0,
                global_layer_index: 0,
                effective_layer_height: 0.2,
            }],
        )]),
        ..Default::default()
    }
}

fn region_map_fixture() -> RegionMapIR {
    RegionMapIR {
        entries: HashMap::from([(
            RegionKey {
                global_layer_index: 0,
                object_id: String::from("cube"),
                region_id: 7,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                stage_modules: HashMap::from([(
                    String::from("Layer::Perimeters"),
                    vec![ModuleInvocation {
                        module_id: String::from("com.example.perimeters"),
                        ..Default::default()
                    }],
                )]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

#[allow(dead_code)]
fn square_polygon() -> ExPolygon {
    ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 10_000, y: 0 },
                Point2 {
                    x: 10_000,
                    y: 10_000,
                },
                Point2 { x: 0, y: 10_000 },
            ],
        },
        holes: Vec::new(),
    }
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}
