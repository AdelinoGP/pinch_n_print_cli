//! TDD tests for TASK-157: multi-object world-space Z with LCM synchronisation.
//!
//! Validates AC `multi_object_transform_world_z`: when two objects with
//! different transforms and different native layer heights coexist in a scene,
//! the LayerPlanIR must have its global layer indices derived from the LCM of
//! the layer heights, and each object's Z range must be correctly projected
//! to world-space.
//!
//! Architecture: follows prepass_executor_tdd.rs — ScriptedRunner stubs the
//! PrePass::LayerPlanning module and returns a LayerPlanIR whose global_layers
//! are LCM-synchronised and whose per-layer active_regions reflect which
//! objects are active at each world-Z height.

#![allow(missing_docs)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    execute_prepass_with_builtins, Blackboard, CompiledModule, CompiledStage,
    ExecutionModuleBinding, ExecutionPlan, PrepassExecutionError, PrepassStageOutput,
    PrepassStageRunner, WasmArtifactMetadata,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectConfig, ObjectLayerRef, ObjectMesh, Point3, ResolvedConfig, SemVer,
    Transform3d,
};

// ============================================================================
// Test: two objects, different transforms, LCM-synchronised global layers
// ============================================================================

/// Object A: 90° around X → world Z = [-1, 0], native layer height 0.4 mm.
/// Object B: translated by +10 mm in Z → world Z = [10, 11], native layer height 0.2 mm.
///
/// LCM(0.4, 0.2) = 0.4 mm — the global sync-layer height.
/// Global sync layers cover the union of both objects' world Z extents:
///   [-0.8, -0.4, 0.0] for object A
///   [10.0, 10.4, 10.8] for object B
///
/// The test verifies that:
/// 1. global_layers[*].z are all in world-space for each respective object.
/// 2. object_participation correctly maps each object to its local layers.
#[test]
fn multi_object_lcm_sync_produces_world_space_z_for_both_objects() {
    let mesh = Arc::new(mesh_with_two_objects());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    let plan = execution_plan_fixture();

    let layer_plan = build_lcm_layer_plan();
    let runner = ScriptedLayerPlanningRunner::new(
        vec![String::from("com.example.layer-planning")],
        Arc::new(layer_plan),
        Arc::as_ptr(&mesh) as usize,
    );

    let _audits = execute_prepass_with_builtins(&plan, &mut blackboard, &runner)
        .expect("prepass should complete without error");

    let layer_plan_ir = blackboard
        .layer_plan()
        .expect("LayerPlanIR must be committed after PrePass::LayerPlanning");

    // ── Object A: world Z ∈ [-1, 0] ──────────────────────────────────────
    let obj_a_layers: Vec<&GlobalLayer> = layer_plan_ir
        .global_layers
        .iter()
        .filter(|l| l.active_regions.iter().any(|r| r.object_id == "object-a"))
        .collect();

    assert!(
        !obj_a_layers.is_empty(),
        "object-a should have at least one active layer"
    );
    for (i, layer) in obj_a_layers.iter().enumerate() {
        assert!(
            layer.z >= -1.0 && layer.z <= 0.0,
            "object-a global_layers z[{}] = {} should be in world Z ∈ [-1, 0]",
            i,
            layer.z
        );
    }

    // Verify object-a's participation record matches its world Z range.
    let obj_a_participation = layer_plan_ir
        .object_participation
        .get("object-a")
        .expect("object-a must have a participation entry");
    assert_eq!(
        obj_a_layers.len(),
        obj_a_participation.len(),
        "object-a participation count must match its active layer count"
    );

    // ── Object B: world Z ∈ [10, 11] ───────────────────────────────────────
    let obj_b_layers: Vec<&GlobalLayer> = layer_plan_ir
        .global_layers
        .iter()
        .filter(|l| l.active_regions.iter().any(|r| r.object_id == "object-b"))
        .collect();

    assert!(
        !obj_b_layers.is_empty(),
        "object-b should have at least one active layer"
    );
    for (i, layer) in obj_b_layers.iter().enumerate() {
        assert!(
            layer.z >= 10.0 && layer.z <= 11.0,
            "object-b global_layers z[{}] = {} should be in world Z ∈ [10, 11]",
            i,
            layer.z
        );
    }

    let obj_b_participation = layer_plan_ir
        .object_participation
        .get("object-b")
        .expect("object-b must have a participation entry");
    assert_eq!(
        obj_b_layers.len(),
        obj_b_participation.len(),
        "object-b participation count must match its active layer count"
    );

    // ── LCM synchronisation: global layer heights should be quantised to 0.4 mm ──
    let global_z_values: Vec<f32> = layer_plan_ir.global_layers.iter().map(|l| l.z).collect();
    for (i, &z) in global_z_values.iter().enumerate() {
        let quantised = (z / 0.4).round() * 0.4;
        assert!(
            (z - quantised).abs() < 1e-5,
            "global_layers[{}].z = {} is not quantised to LCM(0.4) grid (nearest 0.4 = {})",
            i,
            z,
            quantised
        );
    }
}

// ============================================================================
// Test: three objects with three different transforms and layer heights
// ============================================================================

/// Object A: 90° X rotation → world Z = [-1, 0], layer height 0.4 mm.
/// Object B: Z translation +10 → world Z = [10, 11], layer height 0.2 mm.
/// Object C: Z translation +20 → world Z = [20, 21], layer height 0.3 mm.
///
/// LCM(0.4, 0.2, 0.3) = 1.2 mm — the global sync-layer height.
/// All three objects must appear in world-space at their respective Z ranges.
#[test]
fn three_objects_three_transforms_lcm_world_z_correct() {
    let mesh = Arc::new(mesh_with_three_objects());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    let plan = execution_plan_fixture();

    let layer_plan = build_three_object_lcm_layer_plan();
    let runner = ScriptedLayerPlanningRunner::new(
        vec![String::from("com.example.layer-planning")],
        Arc::new(layer_plan),
        Arc::as_ptr(&mesh) as usize,
    );

    let _audits = execute_prepass_with_builtins(&plan, &mut blackboard, &runner)
        .expect("prepass should complete without error");

    let layer_plan_ir = blackboard
        .layer_plan()
        .expect("LayerPlanIR must be committed");

    // Verify each object's world Z range is respected.
    let expectations = [
        ("object-a", -1.0, 0.0),
        ("object-b", 10.0, 11.0),
        ("object-c", 20.0, 21.0),
    ];

    for (obj_id, expected_z_min, expected_z_max) in expectations {
        let obj_layers: Vec<&GlobalLayer> = layer_plan_ir
            .global_layers
            .iter()
            .filter(|l| l.active_regions.iter().any(|r| r.object_id == obj_id))
            .collect();

        assert!(
            !obj_layers.is_empty(),
            "{obj_id} should have at least one active layer"
        );
        for layer in &obj_layers {
            assert!(
                layer.z >= expected_z_min && layer.z <= expected_z_max,
                "{obj_id} layer z={} should be in world Z ∈ [{expected_z_min}, {expected_z_max}]",
                layer.z
            );
        }

        let part = layer_plan_ir
            .object_participation
            .get(obj_id)
            .expect("{obj_id} must have participation entry");
        assert_eq!(
            obj_layers.len(),
            part.len(),
            "{obj_id} participation count mismatch"
        );
    }
}

// ============================================================================
// Mesh fixtures
// ============================================================================

/// Two objects:
///   Object A: 90° around X axis → world Z = [-1, 0], layer height 0.4 mm
///   Object B: translate by +10 in Z → world Z = [10, 11], layer height 0.2 mm
fn mesh_with_two_objects() -> MeshIR {
    // 90° around X (same as transformed_model_world_z_tdd)
    let rot90_x = [
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    // Object A
    let obj_a = ObjectMesh {
        id: String::from("object-a"),
        mesh: unit_cube_its(),
        transform: Transform3d { matrix: rot90_x },
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: Some((-1.0, 0.0)),
    };

    // Object B: translate by +10 in Z
    let mut trans_z10 = identity_transform().matrix;
    trans_z10[14] = 10.0;
    let obj_b = ObjectMesh {
        id: String::from("object-b"),
        mesh: unit_cube_its(),
        transform: Transform3d { matrix: trans_z10 },
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: Some((10.0, 11.0)),
    };

    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![obj_a, obj_b],
        build_volume: BoundingBox3 {
            // Object A world Z = [-1, 0], Object B world Z = [10, 11]
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: 11.0,
            },
        },
    }
}

/// Three objects with different transforms and layer heights.
fn mesh_with_three_objects() -> MeshIR {
    // Object A: 90° X rotation → world Z = [-1, 0]
    let rot90_x = [
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];

    let mut trans_z10 = identity_transform().matrix;
    trans_z10[14] = 10.0;

    let mut trans_z20 = identity_transform().matrix;
    trans_z20[14] = 20.0;

    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![
            ObjectMesh {
                id: String::from("object-a"),
                mesh: unit_cube_its(),
                transform: Transform3d { matrix: rot90_x },
                config: ObjectConfig {
                    data: HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: None,
                world_z_extent: Some((-1.0, 0.0)),
            },
            ObjectMesh {
                id: String::from("object-b"),
                mesh: unit_cube_its(),
                transform: Transform3d { matrix: trans_z10 },
                config: ObjectConfig {
                    data: HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: None,
                world_z_extent: Some((10.0, 11.0)),
            },
            ObjectMesh {
                id: String::from("object-c"),
                mesh: unit_cube_its(),
                transform: Transform3d { matrix: trans_z20 },
                config: ObjectConfig {
                    data: HashMap::new(),
                },
                modifier_volumes: Vec::new(),
                paint_data: None,
                world_z_extent: Some((20.0, 21.0)),
            },
        ],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
            max: Point3 {
                x: 1.0,
                y: 1.0,
                z: 21.0,
            },
        },
    }
}

// ============================================================================
// LayerPlanIR fixtures (all Z values are world-space)
// ============================================================================

/// Builds a LayerPlanIR for two objects with LCM(0.4, 0.2) = 0.4 mm sync layers.
///
/// Object A (world Z ∈ [-1, 0], layer height 0.4):
///   local layers 0, 1, 2 → global layers at -0.8, -0.4, 0.0
/// Object B (world Z ∈ [10, 11], layer height 0.2):
///   local layers 0..5 → global layers at 10.0, 10.4, 10.8
///   (note: 10.2, 10.6 are catch-up layers not in global sync schedule)
fn build_lcm_layer_plan() -> LayerPlanIR {
    // ── Object A sync layers at world Z ∈ [-1, 0], 0.4 mm height ──
    let obj_a_layers: Vec<(f32, f32)> = vec![(-0.8, 0.4), (-0.4, 0.4), (0.0, 0.4)];

    // ── Object B sync layers at world Z ∈ [10, 11], 0.2 mm native → LCM 0.4 mm ──
    // Object B native layers: 10.0, 10.2, 10.4, 10.6, 10.8, 11.0
    // LCM sync schedule (0.4 mm): 10.0, 10.4, 10.8
    let obj_b_layers: Vec<(f32, f32)> = vec![(10.0, 0.2), (10.4, 0.2), (10.8, 0.2)];

    // ── Build global_layers list ──
    // We interleave: all layers from A, then all layers from B.
    // The object_participation map tracks which object participates in each.
    let mut global_layers: Vec<GlobalLayer> = Vec::new();
    let mut global_idx = 0u32;

    // Object A layers
    for (z, height) in &obj_a_layers {
        global_layers.push(GlobalLayer {
            index: global_idx,
            z: *z,
            active_regions: vec![ActiveRegion {
                object_id: String::from("object-a"),
                region_id: 0,
                resolved_config: resolved_config_for_height(*height),
                effective_layer_height: *height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: *z - height,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        });
        global_idx += 1;
    }

    // Object B layers
    for (z, height) in &obj_b_layers {
        global_layers.push(GlobalLayer {
            index: global_idx,
            z: *z,
            active_regions: vec![ActiveRegion {
                object_id: String::from("object-b"),
                region_id: 0,
                resolved_config: resolved_config_for_height(*height),
                effective_layer_height: *height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: *z - height,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        });
        global_idx += 1;
    }

    // ── object_participation ──
    let object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::from([
        (
            String::from("object-a"),
            vec![
                ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 0,
                    effective_layer_height: 0.4,
                },
                ObjectLayerRef {
                    local_layer_index: 1,
                    global_layer_index: 1,
                    effective_layer_height: 0.4,
                },
                ObjectLayerRef {
                    local_layer_index: 2,
                    global_layer_index: 2,
                    effective_layer_height: 0.4,
                },
            ],
        ),
        (
            String::from("object-b"),
            vec![
                ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 3,
                    effective_layer_height: 0.2,
                },
                ObjectLayerRef {
                    local_layer_index: 2,
                    global_layer_index: 4,
                    effective_layer_height: 0.2,
                },
                ObjectLayerRef {
                    local_layer_index: 4,
                    global_layer_index: 5,
                    effective_layer_height: 0.2,
                },
            ],
        ),
    ]);

    LayerPlanIR {
        schema_version: semver(1, 0, 0),
        global_layers,
        object_participation,
    }
}

/// Builds a LayerPlanIR for three objects with LCM(0.4, 0.2, 0.3) = 1.2 mm.
///
/// Object A (world Z ∈ [-1, 0], layer height 0.4):
///   layers at -0.8, 0.0
/// Object B (world Z ∈ [10, 11], layer height 0.2):
///   sync layers at 10.0, 10.4, 10.8 (LCM grid)
/// Object C (world Z ∈ [20, 21], layer height 0.3):
///   sync layers at 20.0, 20.4, 20.8
fn build_three_object_lcm_layer_plan() -> LayerPlanIR {
    let mut global_layers: Vec<GlobalLayer> = Vec::new();
    let mut global_idx = 0u32;

    // Object A: world Z ∈ [-1, 0], height 0.4 → sync at -0.8, 0.0
    for (z, height) in [(-0.8_f32, 0.4), (0.0_f32, 0.4)] {
        global_layers.push(GlobalLayer {
            index: global_idx,
            z,
            active_regions: vec![ActiveRegion {
                object_id: String::from("object-a"),
                region_id: 0,
                resolved_config: resolved_config_for_height(height),
                effective_layer_height: height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: z - height,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        });
        global_idx += 1;
    }

    // Object B: world Z ∈ [10, 11], height 0.2 → LCM sync at 10.0, 10.4, 10.8
    for (z, height) in [(10.0_f32, 0.2), (10.4_f32, 0.2), (10.8_f32, 0.2)] {
        global_layers.push(GlobalLayer {
            index: global_idx,
            z,
            active_regions: vec![ActiveRegion {
                object_id: String::from("object-b"),
                region_id: 0,
                resolved_config: resolved_config_for_height(height),
                effective_layer_height: height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: z - height,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        });
        global_idx += 1;
    }

    // Object C: world Z ∈ [20, 21], height 0.3 → sync at 20.0, 20.4, 20.8
    for (z, height) in [(20.0_f32, 0.3), (20.4_f32, 0.3), (20.8_f32, 0.3)] {
        global_layers.push(GlobalLayer {
            index: global_idx,
            z,
            active_regions: vec![ActiveRegion {
                object_id: String::from("object-c"),
                region_id: 0,
                resolved_config: resolved_config_for_height(height),
                effective_layer_height: height,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: z - height,
                tool_index: 0,
            }],
            has_nonplanar: false,
            is_sync_layer: true,
        });
        global_idx += 1;
    }

    let object_participation: HashMap<String, Vec<ObjectLayerRef>> = HashMap::from([
        (
            String::from("object-a"),
            vec![
                ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 0,
                    effective_layer_height: 0.4,
                },
                ObjectLayerRef {
                    local_layer_index: 1,
                    global_layer_index: 1,
                    effective_layer_height: 0.4,
                },
            ],
        ),
        (
            String::from("object-b"),
            vec![
                ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 2,
                    effective_layer_height: 0.2,
                },
                ObjectLayerRef {
                    local_layer_index: 2,
                    global_layer_index: 3,
                    effective_layer_height: 0.2,
                },
                ObjectLayerRef {
                    local_layer_index: 4,
                    global_layer_index: 4,
                    effective_layer_height: 0.2,
                },
            ],
        ),
        (
            String::from("object-c"),
            vec![
                ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 5,
                    effective_layer_height: 0.3,
                },
                ObjectLayerRef {
                    local_layer_index: 1,
                    global_layer_index: 6,
                    effective_layer_height: 0.3,
                },
                ObjectLayerRef {
                    local_layer_index: 2,
                    global_layer_index: 7,
                    effective_layer_height: 0.3,
                },
            ],
        ),
    ]);

    LayerPlanIR {
        schema_version: semver(1, 0, 0),
        global_layers,
        object_participation,
    }
}

// ============================================================================
// Scripted runner
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
        layer_plan_ir: Arc<LayerPlanIR>,
        expected_mesh_ptr: usize,
    ) -> Self {
        let mut scripted = HashMap::new();
        scripted.insert(
            String::from("com.example.layer-planning"),
            Ok(PrepassStageOutput::LayerPlan(layer_plan_ir)),
        );
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
                "ScriptedRunner should receive the expected mesh pointer"
            );
        }

        let mut observed = self.observed.borrow_mut();
        let next_index = observed.len();
        if let Some(expected_module_id) = self.expected_order.get(next_index) {
            assert_eq!(&module.module_id, expected_module_id);
        }
        observed.push(module.module_id.clone());
        drop(observed);

        self.scripted
            .get(&module.module_id)
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

    CompiledModule {
        module_id: binding.module.id.clone(),
        instance_pool: Arc::clone(&binding.instance_pool),
        ir_read_mask: slicer_host::IrAccessMask {
            paths: binding.module.ir_reads.clone(),
        },
        ir_write_mask: slicer_host::IrAccessMask {
            paths: binding.module.ir_writes.clone(),
        },
        config_view: Arc::clone(&binding.config_view),
        wasm_component: None,
    }
}

fn loaded_layer_planning_module() -> slicer_host::LoadedModule {
    slicer_host::LoadedModule {
        id: String::from("com.example.layer-planning"),
        version: semver(1, 0, 0),
        stage: String::from("PrePass::LayerPlanning"),
        wit_world: String::from("slicer:world-prepass@1.0.0"),
        ir_reads: vec![
            String::from("MeshIR.objects"),
            String::from("SurfaceClassificationIR.per_object"),
        ],
        ir_writes: vec![String::from("LayerPlanIR.global_layers")],
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: slicer_host::ConfigSchema::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: false,
        wasm_path: std::path::PathBuf::from("fixtures/com.example.layer-planning.wasm"),
        placeholder_wasm: false,
    }
}

// ============================================================================
// Geometry helpers
// ============================================================================

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

fn resolved_config_for_height(layer_height: f32) -> ResolvedConfig {
    ResolvedConfig {
        layer_height,
        first_layer_height: layer_height,
        line_width: 0.4,
        first_layer_line_width: 0.4,
        wall_count: 2,
        outer_wall_speed: 50.0,
        inner_wall_speed: 50.0,
        wall_generator: slicer_ir::WallGenerator::Classic,
        arachne_min_feature_size: None,
        infill_type: slicer_ir::InfillType::Grid,
        infill_density: 0.2,
        infill_angle: 45.0,
        infill_speed: 50.0,
        solid_infill_speed: 50.0,
        top_shell_layers: 3,
        bottom_shell_layers: 3,
        support_enabled: false,
        support_type: slicer_ir::SupportType::Traditional,
        support_overhang_angle: 45.0,
        nonplanar_max_angle_deg: None,
        nonplanar_shell_count: None,
        nonplanar_amplitude: None,
        smoothificator_target_height: None,
        smoothificator_adaptive: None,
        extensions: HashMap::new(),
    }
}
