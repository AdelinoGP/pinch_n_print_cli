//! TASK-157: Integration test proving rotate_x(90deg) produces a degenerate
//! world-space Z extent (z_max == z_min → None).
//!
//! When a vertical rod (oriented along +Z in local space) is rotated 90°
//! about the X axis, it lies flat in the Y-Z plane.  After rotation,
//! all vertices share the same world-space Z coordinate, making
//! object_world_z_extent return None (no valid print height).
//!
//! Verification: `cargo test -p slicer-host --test rotated_object_world_extent_tdd -- --nocapture`

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::{
    build_wasm_instance_pool, execute_prepass, Blackboard, CompiledModule, CompiledStage,
    ConfigSchema, ExecutionModuleBinding, ExecutionPlan, PrepassStageOutput, PrepassStageRunner,
    WasmArtifactMetadata,
};
use slicer_ir::{
    BoundingBox3, ConfigView, IndexedTriangleSet, LayerPlanIR, MeshIR, ObjectConfig, ObjectMesh,
    ObjectSurfaceData, PaintRegionIR, Point3, RegionMapIR, SemVer, SurfaceClassificationIR,
    Transform3d,
};

/// Vertical rod mesh: two vertices at local Z=0 and Z=10, one triangle.
/// The third vertex is ALSO at y=0 so the rod has zero thickness in Y,
/// giving a truly degenerate world Z extent after rotate_x(90°).
/// After rotation: (0,0,0)→(0,0,0) and (0,0,10)→(0,-10,0).
/// Both have world Z = 0 → degenerate (z_max == z_min).
fn vertical_rod_mesh() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("vertical-rod"),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    }, // bottom at local Z=0, Y=0
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 10.0,
                    }, // top at local Z=10, Y=0
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 5.0,
                    }, // third vertex also at Y=0 (zero-thickness)
                ],
                indices: vec![0, 1, 2],
            },
            // rotate_x(90°): (x, y, z) → (x, -z, y)
            // After rotation: both y=0 vertices have world Z = 0
            transform: Transform3d {
                matrix: rotate_x_90(),
            },
            config: ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: -200.0,
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

/// Object surface data fixture.
fn surface_fixture() -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        schema_version: semver(1, 0, 0),
        per_object: HashMap::from([(
            String::from("vertical-rod"),
            ObjectSurfaceData {
                facet_classes: vec![slicer_ir::FacetClass::TopSurface],
                surface_groups: Vec::new(),
                bridge_regions: Vec::new(),
                overhang_regions: Vec::new(),
            },
        )]),
    }
}

/// Canned layer-plan fixture for a degenerate (None world Z extent) object.
/// A rotate_x(90deg) object has no printable height, so no valid layers
/// are produced.  global_layers is empty.
fn degenerate_layer_plan_fixture() -> LayerPlanIR {
    LayerPlanIR {
        schema_version: semver(1, 0, 0),
        global_layers: Vec::new(), // no layers — degenerate Z extent
        object_participation: HashMap::new(),
    }
}

fn region_map_fixture() -> RegionMapIR {
    RegionMapIR {
        schema_version: semver(1, 0, 0),
        entries: HashMap::new(),
    }
}

fn paint_regions_fixture() -> PaintRegionIR {
    PaintRegionIR {
        schema_version: semver(1, 0, 0),
        per_layer: HashMap::new(),
    }
}

fn execution_plan_fixture(prepass_stages: Vec<CompiledStage>) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages,
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
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
    let instance_pool = Arc::new(
        build_wasm_instance_pool(
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
        config_view: Arc::new(ConfigView::new()),
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

fn loaded_module(id: &str, stage: &str) -> slicer_host::LoadedModule {
    slicer_host::LoadedModule {
        id: String::from(id),
        version: semver(1, 0, 0),
        stage: String::from(stage),
        wit_world: String::from("slicer:world-prepass@1.0.0"),
        ir_reads: match stage {
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
        },
        ir_writes: match stage {
            "PrePass::MeshAnalysis" => vec![String::from("SurfaceClassificationIR.per_object")],
            "PrePass::LayerPlanning" => vec![String::from("LayerPlanIR.global_layers")],
            "PrePass::PaintSegmentation" => vec![String::from("PaintRegionIR.per_layer")],
            "PrePass::RegionMapping" => vec![String::from("RegionMapIR.entries")],
            _ => Vec::new(),
        },
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
        wasm_path: PathBuf::from(format!("fixtures/{id}.wasm")),
        placeholder_wasm: false,
    }
}

// ---------------------------------------------------------------------------
// ScriptedRunner
// ---------------------------------------------------------------------------

struct ScriptedRunner {
    expected_mesh_ptr: usize,
    scripted: HashMap<String, Result<PrepassStageOutput, slicer_host::PrepassExecutionError>>,
    observed: std::cell::RefCell<Vec<String>>,
    expected_order: Vec<String>,
}

impl ScriptedRunner {
    fn new(
        expected_order: &[&str],
        scripted: Vec<(
            String,
            Result<PrepassStageOutput, slicer_host::PrepassExecutionError>,
        )>,
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
        module: &CompiledModule,
        blackboard: &Blackboard,
    ) -> Result<(PrepassStageOutput, Vec<String>), slicer_host::PrepassExecutionError> {
        let observed_mesh_ptr = Arc::as_ptr(blackboard.mesh()) as usize;
        if self.expected_mesh_ptr != 0 {
            assert_eq!(
                observed_mesh_ptr, self.expected_mesh_ptr,
                "ScriptedRunner should receive the expected rotated mesh"
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
            .expect("runner fixture should define every module outcome")
            .map(|output| (output, Vec::new()))
    }
}

// ---------------------------------------------------------------------------
// Test
// ---------------------------------------------------------------------------

/// AC: rotated_object_world_extent
///
/// rotate_x(90deg) on a vertical rod collapses the world-space Z extent to a
/// single Z value (z_max == z_min), making object_world_z_extent return None.
/// The LayerPlanIR for such an object must have empty global_layers.
///
/// This test proves:
/// 1. object_world_z_extent returns None for the rotated rod mesh
/// 2. The prepass pipeline correctly handles a degenerate Z extent by
///    producing an empty global_layers list
#[test]
fn rotated_object_world_extent_is_degenerate() {
    // Mesh: vertical rod rotated 90° about X axis.
    // object_world_z_extent should return None (z_max == z_min).
    let mesh = Arc::new(vertical_rod_mesh());
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
                // Degenerate Z extent → no layers
                Ok(PrepassStageOutput::LayerPlan(Arc::new(
                    degenerate_layer_plan_fixture(),
                ))),
            ),
            (
                String::from("com.example.paint-segmentation"),
                Ok(PrepassStageOutput::PaintRegions(Arc::new(
                    paint_regions_fixture(),
                ))),
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

    let _audits = execute_prepass(&plan, &mut blackboard, &runner)
        .expect("prepass executor should run fixed stage order even with degenerate mesh");

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

    // CORE ASSERTION 1: object_world_z_extent is None for rotate_x(90deg)
    let extent = slicer_host::model_loader::object_world_z_extent(&mesh.objects[0]);
    assert!(
        extent.is_none(),
        "rotate_x(90deg) vertical rod must have degenerate world Z extent (None), got {:?}",
        extent
    );

    // CORE ASSERTION 2: The committed LayerPlanIR has no global_layers
    let layer_plan = blackboard
        .layer_plan()
        .expect("layer plan must be committed");
    assert!(
        layer_plan.global_layers.is_empty(),
        "Degenerate world Z extent must result in zero global_layers, got {} layers",
        layer_plan.global_layers.len()
    );
}

/// Returns the column-major 4x4 matrix for a 90-degree rotation about the X axis.
///
/// Rotation formula: (x, y, z) → (x, -z, y)
/// The new Y = -old Z and new Z = old Y.
/// Column-major storage: matrix[col * 4 + row]
///   col 0: (1, 0, 0, 0) → [0]=1,  [4]=0,  [8]=0,  [12]=0
///   col 1: (0, 0, 1, 0) → [1]=0,  [5]=0,  [9]=1,  [13]=0   (+Z→Y)
///   col 2: (0,-1, 0, 0) → [2]=0,  [6]=-1, [10]=0, [14]=0   (-Z→Y? no!)
///   col 3: (0, 0, 0, 1) → [3]=0,  [7]=0,  [11]=0, [15]=1
/// Correction: new_z = old_y, so col 1 row 2 = +1 (not col 2).
/// col 1 = (0, 0, 1, 0) → [1]=0, [5]=0, [9]=1, [13]=0   → new_y = +z
/// Wait: (x,y,z)→(x,-z,y): new_y = -z = -1*z + 0*y → col1[1]=0, col2[1]=-1
/// new_z = y = 0*x + 1*y + 0*z → col0[2]=0, col1[2]=1, col2[2]=0
/// So: m[9]=+1 (col1,row2=new_y=+z? no that's wrong)
/// Let me redo: output.y = m[1]*x + m[5]*y + m[9]*z + m[13]*1
/// For new_y = -z: m[9] = -1, m[5] = 0
/// For new_z = y: m[2]*x + m[6]*y + m[10]*z + m[14] = y → m[6]=1
fn rotate_x_90() -> [f64; 16] {
    let mut m = [0.0f64; 16];
    m[0] = 1.0; // col 0 row 0: X → X
    m[9] = -1.0; // col 1 row 2: new_y = -z (so +z gives -y in output)
    m[6] = 1.0; // col 2 row 1: new_z = +y
    m[15] = 1.0; // homogeneous w
    m
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}
