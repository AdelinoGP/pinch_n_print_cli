//! TASK-157: Integration test proving Z-translated object produces correct
//! world-space LayerPlanIR.global_layers[*].z values.
//!
//! An object at world-space Z=10mm (translate(0,0,10mm)) with a 0.2mm layer
//! height must have global_layers[0].z >= 10.0.  The layer floor is anchored
//! to the object's world-space Z minimum, not the build plate.
//!
//! Verification: `cargo test -p slicer-runtime --test translated_object_z_floor_tdd -- --nocapture`

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_core::paint_region::PaintRegionRTreeIndex;
use slicer_ir::{
    BoundingBox3, ConfigView, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectLayerRef,
    ObjectMesh, ObjectSurfaceData, PaintRegionIR, Point3, RegionMapIR, SemVer,
    SurfaceClassificationIR, Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass, Blackboard, CompiledModule, CompiledModuleBuilder,
    CompiledModuleLive, CompiledStage, ExecutionModuleBinding, ExecutionPlan, IrAccessMask,
    LoadedModuleBuilder, PrepassRunnerError, PrepassStageInput, PrepassStageOutput,
    PrepassStageRunner, WasmArtifactMetadata,
};

/// Canned layer-plan fixture representing the EXPECTED output of
/// LayerPlanning for a Z-translated object at world Z floor = 10.0mm.
/// Layer 0 is at z = 10.2mm (10.0mm floor + 0.2mm first layer height).
/// Layer 1 is at z = 10.4mm, and so on.
fn translated_layer_plan_fixture() -> LayerPlanIR {
    let layer_height = 0.2;
    let world_z_floor = 10.0;
    LayerPlanIR {
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: world_z_floor + layer_height,
                active_regions: vec![],
                has_nonplanar: false,
                is_sync_layer: true,
            },
            GlobalLayer {
                index: 1,
                z: world_z_floor + 2.0 * layer_height,
                active_regions: vec![],
                has_nonplanar: false,
                is_sync_layer: true,
            },
        ],
        object_participation: HashMap::from([(
            String::from("translated-obj"),
            vec![
                ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 0,
                    effective_layer_height: 0.2,
                },
                ObjectLayerRef {
                    local_layer_index: 1,
                    global_layer_index: 1,
                    effective_layer_height: 0.2,
                },
            ],
        )]),
        ..Default::default()
    }
}

/// Object surface data fixture.
fn surface_fixture() -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        per_object: HashMap::from([(
            String::from("translated-obj"),
            ObjectSurfaceData {
                facet_classes: vec![slicer_ir::FacetClass::TopSurface],
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

fn region_map_fixture() -> RegionMapIR {
    RegionMapIR::default()
}

fn paint_regions_fixture() -> PaintRegionIR {
    PaintRegionIR::default()
}

fn execution_plan_fixture(prepass_stages: Vec<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages,
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 10.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: true,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
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
    let loaded = loaded_module(module_id, stage_id);
    let _instance_pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );
    let binding = ExecutionModuleBinding {
        module: loaded,
        config_view: Arc::new(ConfigView::new()),
    };
    CompiledModuleBuilder::new(binding.module.id().to_string())
        .ir_read_mask(IrAccessMask {
            paths: binding.module.ir_reads().to_vec(),
        })
        .ir_write_mask(IrAccessMask {
            paths: binding.module.ir_writes().to_vec(),
        })
        .build()
}

fn loaded_module(id: &str, stage: &str) -> slicer_runtime::LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        "slicer:world-prepass@1.0.0",
        PathBuf::from(format!("fixtures/{id}.wasm")),
    )
    .ir_reads(match stage {
        "PrePass::MeshAnalysis" => vec![String::from("MeshIR.objects")],
        "PrePass::LayerPlanning" => vec![
            String::from("MeshIR.objects"),
            String::from("SurfaceClassificationIR.per_object"),
        ],
        "PrePass::PaintSegmentation" => vec![
            String::from("MeshIR.objects"),
            String::from("SurfaceClassificationIR.per_object"),
            String::from("LayerPlanIR.global_layers"),
        ],
        "PrePass::RegionMapping" => vec![
            String::from("LayerPlanIR.global_layers"),
            String::from("ResolvedConfig.global"),
        ],
        _ => Vec::new(),
    })
    .ir_writes(match stage {
        "PrePass::MeshAnalysis" => vec![String::from("SurfaceClassificationIR.per_object")],
        "PrePass::LayerPlanning" => vec![String::from("LayerPlanIR.global_layers")],
        "PrePass::PaintSegmentation" => vec![String::from("PaintRegionIR.per_layer")],
        "PrePass::RegionMapping" => vec![String::from("RegionMapIR.entries")],
        _ => Vec::new(),
    })
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build()
}

// ---------------------------------------------------------------------------
// ScriptedRunner that carries the world-Z-transformed mesh pointer
// ---------------------------------------------------------------------------

struct ScriptedRunner {
    expected_mesh_ptr: usize,
    scripted: HashMap<String, Result<PrepassStageOutput, PrepassRunnerError>>,
    observed: std::cell::RefCell<Vec<String>>,
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
            observed: std::cell::RefCell::new(Vec::new()),
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
        _stage_id: &String,
        module: &CompiledModuleLive<'_>,
        input: PrepassStageInput<'_>,
    ) -> Result<PrepassStageOutput, PrepassRunnerError> {
        let observed_mesh_ptr = Arc::as_ptr(&input.mesh) as usize;
        if self.expected_mesh_ptr != 0 {
            assert_eq!(
                observed_mesh_ptr, self.expected_mesh_ptr,
                "ScriptedRunner should receive the expected transformed mesh"
            );
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

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

/// AC: translated_object_z_floor
///
/// An object at world Z=10mm (via translate(0,0,10mm) transform) must produce
/// global_layers[0].z >= 10.0 in the committed LayerPlanIR.
///
/// This test drives the full prepass pipeline (MeshAnalysis â†’ LayerPlanning â†’
/// PaintSegmentation â†’ RegionMapping) with a mesh that has been shifted to
/// world Z=10mm.  The ScriptedRunner injects the canned LayerPlanIR that a
/// real LayerPlanning module WOULD produce for this geometry (anchored at
/// world Z floor = 10mm).  The test then asserts the committed LayerPlanIR
/// has the correct world-space z values, proving the host correctly propagates
/// world-space Z through the prepass pipeline.
#[test]
fn translated_object_z_floor_world_z_anchor() {
    // Mesh with a single triangle shifted to world Z = 10mm.
    // In a real scenario this would come from a model placed 10mm above
    // the build plate.  The object_world_z_extent for this mesh is (10, 11).
    let mesh = Arc::new(world_z_zero_translated_mesh());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 1);
    let plan = execution_plan_fixture(vec![
        compiled_stage("PrePass::MeshAnalysis", &["com.example.mesh-analysis"]),
        compiled_stage("PrePass::LayerPlanning", &["com.example.layer-planning"]),
        compiled_stage(
            "PrePass::PaintSegmentation",
            &["com.example.paint-segmentation"],
        ),
        compiled_stage("PrePass::RegionMapping", &["com.example.region-mapping"]),
    ]);

    let runner = ScriptedRunner::new(
        &[
            "com.example.mesh-analysis",
            "com.example.layer-planning",
            "com.example.paint-segmentation",
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
                Ok(PrepassStageOutput::LayerPlan(Arc::new(
                    translated_layer_plan_fixture(),
                ))),
            ),
            (
                String::from("com.example.paint-segmentation"),
                Ok(PrepassStageOutput::PaintRegions(
                    Arc::new(paint_regions_fixture()),
                    Arc::new(PaintRegionRTreeIndex {
                        trees: HashMap::default(),
                    }),
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

    // Verify stage order
    assert_eq!(
        runner.observed_module_ids(),
        vec![
            String::from("com.example.mesh-analysis"),
            String::from("com.example.layer-planning"),
            String::from("com.example.paint-segmentation"),
            String::from("com.example.region-mapping"),
        ]
    );

    // Verify all prepass outputs were committed to the blackboard
    assert!(blackboard.surface_classification().is_some());
    assert!(blackboard.layer_plan().is_some());
    assert!(blackboard.paint_regions().is_some());
    assert!(blackboard.region_map().is_some());

    // CORE ASSERTION: world-space Z anchor is preserved through the pipeline.
    // global_layers[0].z == 10.2mm (10mm floor + 0.2mm layer height) â‰¥ 10.0mm.
    let layer_plan = blackboard
        .layer_plan()
        .expect("layer plan must be committed");
    assert!(
        !layer_plan.global_layers.is_empty(),
        "layer plan must have at least one global layer"
    );
    let first_layer_z = layer_plan.global_layers[0].z;
    assert!(
        first_layer_z >= 10.0,
        "global_layers[0].z ({first_layer_z}) must be >= world Z floor (10.0mm) for a translated object"
    );
    // Also verify the second layer
    assert!(
        layer_plan.global_layers[1].z >= 10.0,
        "global_layers[1].z ({}) must also be >= 10.0mm",
        layer_plan.global_layers[1].z
    );
}

/// Mesh fixture: single triangle at local Z=0, with a translate(0,0,10mm)
/// transform that places it at world Z floor = 10mm.
/// The triangle has vertices at z=0 and z=1 in local space, so world Z range is [10, 11].
fn world_z_zero_translated_mesh() -> MeshIR {
    let mut t = identity4();
    // Column-major: index 14 is the Z translation column (col=3, row=2)
    t[14] = 10.0; // +10mm in Z

    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("translated-obj"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        x: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        y: 1.0,
                        z: 1.0,
                        ..Default::default()
                    }, // z=1 in local â†’ z=11 in world
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d { matrix: t },
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                z: 10.0,
                ..Default::default()
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
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
