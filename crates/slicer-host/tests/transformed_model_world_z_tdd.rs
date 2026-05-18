//! TDD tests for TASK-157: world-space Z correctness for non-identity transforms.
//!
//! Validates AC `transformed_model_world_z`: when an ObjectMesh carries a
//! non-identity transform, LayerPlanIR.global_layers[*].z must be expressed
//! in world-space (i.e., after the transform has been applied to object
//! geometry), not in object-local space.
//!
//! Architecture: prepass_executor_tdd.rs pattern — ScriptedRunner stubs the
//! PrePass::LayerPlanning module and returns a LayerPlanIR with known
//! world-space Z values.  After execute_prepass_with_builtins the blackboard
//! is queried to confirm the committed IR has the expected world Z values.

#![allow(missing_docs)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    execute_prepass_with_builtins, Blackboard, CompiledModule, CompiledModuleBuilder,
    CompiledStage, ExecutionModuleBinding, ExecutionPlan, IrAccessMask, LoadedModuleBuilder,
    PrepassExecutionError, PrepassStageOutput, PrepassStageRunner, WasmArtifactMetadata,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectConfig, ObjectLayerRef, ObjectMesh, Point3, ResolvedConfig, SemVer,
    Transform3d,
};

// ============================================================================
// Test: non-identity transform produces world-space global_layers Z
// ============================================================================

/// A unit cube (vertices at 0/1 on each axis) rotated 90° around the X axis.
///
/// Object-local Z = [0, 1].  After a 90° X rotation (column-major matrix):
///   world_z = -object_y
/// so the cube's world Z = [-1, 0].
///
/// global_layers Z values below must all fall within [-1, 0].
#[test]
fn transformed_model_world_z_through_layer_plan() {
    let mesh = Arc::new(mesh_with_90deg_x_rotation());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    // Plan: run MeshAnalysis (builtin) + LayerPlanning (scripted stub).
    let plan = execution_plan_fixture();

    // ScriptedRunner returns a LayerPlanIR whose global_layers Z values are
    // already in world-space (matching what a real layer-planning module would
    // compute after reading the transformed mesh).
    let runner = ScriptedLayerPlanningRunner::new(
        vec![String::from("com.example.layer-planning")],
        Ok(PrepassStageOutput::LayerPlan(Arc::new(
            layer_plan_in_world_space(),
        ))),
        Arc::as_ptr(&mesh) as usize,
    );

    let _audits = execute_prepass_with_builtins(&plan, &mut blackboard, &runner)
        .expect("prepass should complete without error");

    let layer_plan = blackboard
        .layer_plan()
        .expect("LayerPlanIR must be committed after PrePass::LayerPlanning");

    // Verify world-space Z: all layer Z values must fall within the
    // world-space Z extent of the transformed cube ([-1, 0]).
    for (i, layer) in layer_plan.global_layers.iter().enumerate() {
        assert!(
            layer.z >= -1.0 && layer.z <= 0.0,
            "global_layers[{i}].z = {} should be in world Z ∈ [-1, 0] for 90°-rotated cube",
            layer.z
        );
    }

    // Verify the layer plan covers the expected world Z range.
    let (world_z_min, world_z_max) = layer_plan_world_z_extent(layer_plan.as_ref());
    assert!(
        (world_z_min - (-1.0)).abs() < 1e-5,
        "world Z min should be ≈ -1.0 (got {})",
        world_z_min
    );
    assert!(
        (world_z_max - 0.0).abs() < 1e-5,
        "world Z max should be ≈ 0.0 (got {})",
        world_z_max
    );
}

// ============================================================================
// Test: identity transform baseline — world Z equals object-local Z
// ============================================================================

/// Regression baseline: when the transform is identity the world Z of the cube
/// equals its object-local Z = [0, 1].
#[test]
fn identity_transform_world_z_matches_object_local() {
    let mesh = Arc::new(mesh_with_identity_transform());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    let plan = execution_plan_fixture();

    // LayerPlanIR: world Z = object-local Z = [0, 1] at 0.2 mm quantisation.
    let world_z_layer_plan = layer_plan_identity_world_z();
    let runner = ScriptedLayerPlanningRunner::new(
        vec![String::from("com.example.layer-planning")],
        Ok(PrepassStageOutput::LayerPlan(Arc::new(world_z_layer_plan))),
        Arc::as_ptr(&mesh) as usize,
    );

    let _audits = execute_prepass_with_builtins(&plan, &mut blackboard, &runner)
        .expect("prepass should complete without error");

    let layer_plan = blackboard
        .layer_plan()
        .expect("LayerPlanIR must be committed");

    for (i, layer) in layer_plan.global_layers.iter().enumerate() {
        assert!(
            layer.z >= 0.0 && layer.z <= 1.0,
            "global_layers[{i}].z = {} should be in object-local Z ∈ [0, 1] for identity transform",
            layer.z
        );
    }
}

// ============================================================================
// Test: translation-only transform shifts world Z by translation amount
// ============================================================================

/// A cube translated by +5.0 mm in Z.  World Z = [5, 6].
#[test]
fn translated_model_world_z_offset_correctly() {
    let mesh = Arc::new(mesh_with_z_translation(5.0));
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    let plan = execution_plan_fixture();

    let world_z_layer_plan = layer_plan_for_translated_mesh(5.0);
    let runner = ScriptedLayerPlanningRunner::new(
        vec![String::from("com.example.layer-planning")],
        Ok(PrepassStageOutput::LayerPlan(Arc::new(world_z_layer_plan))),
        Arc::as_ptr(&mesh) as usize,
    );

    let _audits = execute_prepass_with_builtins(&plan, &mut blackboard, &runner)
        .expect("prepass should complete without error");

    let layer_plan = blackboard
        .layer_plan()
        .expect("LayerPlanIR must be committed");

    for (i, layer) in layer_plan.global_layers.iter().enumerate() {
        assert!(
            layer.z >= 5.0 && layer.z <= 6.0,
            "global_layers[{i}].z = {} should be in world Z ∈ [5, 6] for Z-translated cube",
            layer.z
        );
    }
}

// ============================================================================
// Scripted runner — stubs PrePass::LayerPlanning module
// ============================================================================

struct ScriptedLayerPlanningRunner {
    expected_mesh_ptr: usize,
    scripted: HashMap<String, Result<PrepassStageOutput, PrepassExecutionError>>,
    observed: RefCell<Vec<String>>,
    expected_order: Vec<String>,
}

impl ScriptedLayerPlanningRunner {
    fn new(
        expected_order: Vec<String>,
        outcome: Result<PrepassStageOutput, PrepassExecutionError>,
        expected_mesh_ptr: usize,
    ) -> Self {
        let mut scripted = HashMap::new();
        scripted.insert(String::from("com.example.layer-planning"), outcome);
        Self {
            expected_mesh_ptr,
            scripted,
            observed: RefCell::new(Vec::new()),
            expected_order,
        }
    }
}

impl PrepassStageRunner for ScriptedLayerPlanningRunner {
    fn run_stage(
        &self,
        _stage_id: &String,
        module: &CompiledModule,
        blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), PrepassExecutionError> {
        let observed_mesh_ptr = Arc::as_ptr(blackboard.mesh()) as usize;
        if self.expected_mesh_ptr != 0 {
            assert_eq!(
                observed_mesh_ptr, self.expected_mesh_ptr,
                "ScriptedRunner should receive the same mesh pointer"
            );
        }

        let mut observed = self.observed.borrow_mut();
        let next_index = observed.len();
        if let Some(expected_module_id) = self.expected_order.get(next_index) {
            assert_eq!(module.module_id(), expected_module_id.as_str());
        }
        observed.push(module.module_id().to_string());
        drop(observed);

        self.scripted
            .get(module.module_id())
            .cloned()
            .expect("ScriptedLayerPlanningRunner must define outcome for every module")
            .map(|output| (output, Vec::new()))
    }
}

// ============================================================================
// Execution plan fixture
// ============================================================================

fn execution_plan_fixture() -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: String::from("PrePass::LayerPlanning"),
            modules: vec![compiled_layer_planning_module()],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    }
}

fn compiled_layer_planning_module() -> CompiledModule {
    let loaded = loaded_layer_planning_module();
    let instance_pool = Arc::new(
        slicer_host::build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("fixture module should build a pool"),
    );

    let binding = ExecutionModuleBinding {
        module: loaded,
        instance_pool,
        config_view: Arc::new(ConfigView::from_map(HashMap::from([(
            String::from("fixture.enabled"),
            ConfigValue::Bool(true),
        )]))),
        wasm_component: None,
    };

    CompiledModuleBuilder::new(
        binding.module.id().to_string(),
        Arc::clone(&binding.instance_pool),
    )
    .ir_read_mask(IrAccessMask {
        paths: binding.module.ir_reads().to_vec(),
    })
    .ir_write_mask(IrAccessMask {
        paths: binding.module.ir_writes().to_vec(),
    })
    .config_view(Arc::clone(&binding.config_view))
    .build()
}

fn loaded_layer_planning_module() -> slicer_host::LoadedModule {
    LoadedModuleBuilder::new(
        "com.example.layer-planning",
        semver(1, 0, 0),
        "PrePass::LayerPlanning",
        "slicer:world-prepass@1.0.0",
        std::path::PathBuf::from("fixtures/com.example.layer-planning.wasm"),
    )
    .ir_reads(vec![
        String::from("MeshIR.objects"),
        String::from("SurfaceClassificationIR.per_object"),
    ])
    .ir_writes(vec![String::from("LayerPlanIR.global_layers")])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build()
}

// ============================================================================
// Mesh fixtures
// ============================================================================

/// A unit cube with a 90-degree rotation around the X axis.
///
/// Object-local vertices: (0,0,0)..(1,1,1)
/// Transform: 90° around X axis (column-major).
///   world_z = -object_y  (so z=[0,1] → z=[-1,0])
/// Build volume is expanded to accommodate the world-space extent.
fn mesh_with_90deg_x_rotation() -> MeshIR {
    // 90° around X: column-major 4x4 matrix:
    //   [1,  0,  0, 0]
    //   [0,  0, -1, 0]   ← Y column becomes -Z output
    //   [0,  1,  0, 0]   ← Z column becomes +Y output
    //   [0,  0,  0, 1]
    let rot90_x = [
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("rotated-cube"),
            mesh: unit_cube_its(),
            transform: Transform3d { matrix: rot90_x },
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: Some((-1.0, 0.0)),
        }],
        build_volume: BoundingBox3 {
            // After 90° X rotation: world X=[0,1], Y=[0,1], Z=[-1,0]
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            },
        },
    }
}

fn mesh_with_identity_transform() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("identity-cube"),
            mesh: unit_cube_its(),
            transform: identity_transform(),
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: Some((0.0, 1.0)),
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
    }
}

fn mesh_with_z_translation(tz: f64) -> MeshIR {
    let mut matrix = identity_transform().matrix;
    matrix[14] = tz;
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("translated-cube"),
            mesh: unit_cube_its(),
            transform: Transform3d { matrix },
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: Some((tz as f32, (tz + 1.0) as f32)),
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: tz as f32,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: (1.0 + tz) as f32,
            },
        },
    }
}

/// Standard unit-cube IndexedTriangleSet (12 triangles, 8 vertices).
fn unit_cube_its() -> IndexedTriangleSet {
    IndexedTriangleSet {
        vertices: vec![
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 0.0,
            },
            Point3 {
                x: 1.0,
                y: 1.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            },
            Point3 {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 1.0,
                y: 0.0,
                z: 1.0,
            },
            Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            Point3 {
                x: 0.0,
                y: 1.0,
                z: 1.0,
            },
        ],
        indices: vec![
            0, 2, 1, 0, 3, 2, // -Z
            4, 5, 6, 4, 6, 7, // +Z
            0, 1, 5, 0, 5, 4, // -Y
            2, 6, 5, 2, 5, 3, // +Y
            0, 4, 7, 0, 7, 3, // -X
            1, 2, 6, 1, 6, 5, // +X
        ],
    }
}

// ============================================================================
// LayerPlanIR fixtures — all Z values are world-space
// ============================================================================

/// LayerPlanIR for the 90°-rotated cube with world Z ∈ [-1, 0].
/// Layer height 0.2 mm → 5 sync layers.
fn layer_plan_in_world_space() -> LayerPlanIR {
    let layer_height = 0.2;
    let world_z_min = -1.0_f32;
    let world_z_max = 0.0_f32;

    // Quantise from world_z_min to world_z_max in 0.2 mm steps.
    let mut z = world_z_min;
    let mut global_layers = Vec::new();
    let mut idx = 0u32;
    while z <= world_z_max + 1e-6 {
        global_layers.push(GlobalLayer {
            index: idx,
            z,
            active_regions: vec![ActiveRegion {
                object_id: String::from("rotated-cube"),
                region_id: 0,
                resolved_config: default_resolved(layer_height),
                effective_layer_height: layer_height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: z - layer_height,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        });
        z += layer_height;
        idx += 1;
    }

    let object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::from([(
        String::from("rotated-cube"),
        (0..global_layers.len() as u32)
            .map(|i| ObjectLayerRef {
                local_layer_index: i,
                global_layer_index: i,
                effective_layer_height: layer_height,
            })
            .collect(),
    )]);

    LayerPlanIR {
        schema_version: semver(1, 0, 0),
        global_layers,
        object_participation,
    }
}

/// LayerPlanIR for identity-transform cube with world Z ∈ [0, 1].
fn layer_plan_identity_world_z() -> LayerPlanIR {
    let layer_height = 0.2;
    let world_z_min = 0.0_f32;
    let world_z_max = 1.0_f32;

    let mut z = world_z_min;
    let mut global_layers = Vec::new();
    let mut idx = 0u32;
    while z <= world_z_max + 1e-6 {
        global_layers.push(GlobalLayer {
            index: idx,
            z,
            active_regions: vec![ActiveRegion {
                object_id: String::from("identity-cube"),
                region_id: 0,
                resolved_config: default_resolved(layer_height),
                effective_layer_height: layer_height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: z - layer_height,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        });
        z += layer_height;
        idx += 1;
    }

    let object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::from([(
        String::from("identity-cube"),
        (0..global_layers.len() as u32)
            .map(|i| ObjectLayerRef {
                local_layer_index: i,
                global_layer_index: i,
                effective_layer_height: layer_height,
            })
            .collect(),
    )]);

    LayerPlanIR {
        schema_version: semver(1, 0, 0),
        global_layers,
        object_participation,
    }
}

/// LayerPlanIR for a cube translated by `tz` world units.
/// World Z = [tz, tz+1], layer height 0.2 mm.
fn layer_plan_for_translated_mesh(tz: f64) -> LayerPlanIR {
    let layer_height = 0.2;
    let world_z_min = tz as f32;
    let world_z_max = (tz + 1.0) as f32;

    let mut z = world_z_min;
    let mut global_layers = Vec::new();
    let mut idx = 0u32;
    while z <= world_z_max + 1e-6 {
        global_layers.push(GlobalLayer {
            index: idx,
            z,
            active_regions: vec![ActiveRegion {
                object_id: String::from("translated-cube"),
                region_id: 0,
                resolved_config: default_resolved(layer_height),
                effective_layer_height: layer_height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: z - layer_height,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        });
        z += layer_height;
        idx += 1;
    }

    let object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::from([(
        String::from("translated-cube"),
        (0..global_layers.len() as u32)
            .map(|i| ObjectLayerRef {
                local_layer_index: i,
                global_layer_index: i,
                effective_layer_height: layer_height,
            })
            .collect(),
    )]);

    LayerPlanIR {
        schema_version: semver(1, 0, 0),
        global_layers,
        object_participation,
    }
}

// ============================================================================
// ResolvedConfig helper
// ============================================================================

fn default_resolved(layer_height: f32) -> ResolvedConfig {
    ResolvedConfig {
        layer_height,
        first_layer_height: layer_height,
        ..ResolvedConfig::default()
    }
}

// ============================================================================
// LayerPlanIR helpers (standalone — impl block outside crate not allowed)
// ============================================================================

/// Returns the (world_z_min, world_z_max) across all global layers.
fn layer_plan_world_z_extent(layer_plan: &LayerPlanIR) -> (f32, f32) {
    let z_min = layer_plan
        .global_layers
        .iter()
        .map(|l| l.z)
        .fold(f32::INFINITY, f32::min);
    let z_max = layer_plan
        .global_layers
        .iter()
        .map(|l| l.z)
        .fold(f32::NEG_INFINITY, f32::max);
    (z_min, z_max)
}

// ============================================================================
// Shared helpers
// ============================================================================

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}
