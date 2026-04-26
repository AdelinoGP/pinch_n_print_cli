//! TDD: host entity ordering before path-optimization (packet-18).
//!
//! Covers TASK-152 / TASK-152a / TASK-152d / TASK-152e.
//! All tests exercise `order_entities_by_nearest_neighbor` and the live
//! host pre-staging path in `execute_per_layer`.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use slicer_host::instance_pool::build_wasm_instance_pool;
use slicer_host::manifest::LoadedModule;
use slicer_host::{
    execute_per_layer, order_entities_by_nearest_neighbor, Blackboard, CompiledModule,
    CompiledStage, ConfigSchema, ExecutionModuleBinding, ExecutionPlan, IrAccessMask, LayerArena,
    LayerStageError, LayerStageOutput, LayerStageRunner, WasmArtifactMetadata,
};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigView, ExtrusionPath3D, ExtrusionRole, GlobalLayer,
    IndexedTriangleSet, InfillIR, InfillRegion, MeshIR, ObjectConfig, ObjectMesh, Point3,
    Point3WithWidth, PrintEntity, RegionKey, ResolvedConfig, SemVer, StageId, Transform3d,
};

// ── Fixtures ─────────────────────────────────────────────────────────────────

fn semver() -> SemVer {
    SemVer { major: 1, minor: 0, patch: 0 }
}

fn pt(x: f32, y: f32) -> Point3WithWidth {
    Point3WithWidth { x, y, z: 0.2, width: 0.4, flow_factor: 1.0 }
}

fn entity_at(x: f32, y: f32, role: ExtrusionRole, object_id: &str, original_idx: u32) -> PrintEntity {
    PrintEntity {
        path: ExtrusionPath3D {
            points: vec![pt(x, y)],
            role: role.clone(),
            speed_factor: 1.0,
        },
        role,
        region_key: RegionKey {
            global_layer_index: 0,
            object_id: object_id.to_string(),
            region_id: 0,
        },
        topo_order: original_idx,
    }
}

fn sparse(x: f32, y: f32, object_id: &str, idx: u32) -> PrintEntity {
    entity_at(x, y, ExtrusionRole::SparseInfill, object_id, idx)
}

fn bridge(x: f32, y: f32, object_id: &str, idx: u32) -> PrintEntity {
    entity_at(x, y, ExtrusionRole::BridgeInfill, object_id, idx)
}

// ── AC-1: same-object nearest-neighbor ordering ───────────────────────────────

/// AC-1: Given three same-object entities at (0,0), (30,0), (10,0), the
/// ordering helper produces the sequence 0.0, 10.0, 30.0 with topo_order 0,1,2.
#[test]
fn same_object_nearest_neighbor_ordering_is_applied_before_path_optimization() {
    let entities = vec![
        sparse(0.0, 0.0, "obj", 0),
        sparse(30.0, 0.0, "obj", 1),
        sparse(10.0, 0.0, "obj", 2),
    ];

    let result = order_entities_by_nearest_neighbor(entities);

    let xs: Vec<f32> = result.iter().map(|e| e.path.points[0].x).collect();
    assert_eq!(
        xs,
        vec![0.0_f32, 10.0, 30.0],
        "expected NN-ordered x sequence [0.0, 10.0, 30.0], got {xs:?}"
    );
    let topos: Vec<u32> = result.iter().map(|e| e.topo_order).collect();
    assert_eq!(
        topos,
        vec![0u32, 1, 2],
        "expected topo_order [0, 1, 2], got {topos:?}"
    );
}

// ── AC-2: cross-object ordering ───────────────────────────────────────────────

/// AC-2: Given a mixed-object layer whose raw order is [A1, A2, B1, B2] but
/// nearest-travel interleaves objects, the result is object_id sequence A,B,B,A.
#[test]
fn cross_object_ordering_resequences_entities_by_travel_cost() {
    // A1(0,0) A2(0,100) B1(1,0) B2(1,1) — raw order is all A then all B.
    // NN from (0,0): A1(d=0)→B1(d=1 from 0,0)→B2(d=1 from 1,0)→A2(last).
    let entities = vec![
        sparse(0.0, 0.0, "A", 0),
        sparse(0.0, 100.0, "A", 1),
        sparse(1.0, 0.0, "B", 2),
        sparse(1.0, 1.0, "B", 3),
    ];

    let result = order_entities_by_nearest_neighbor(entities);

    let ids: Vec<&str> = result.iter().map(|e| e.region_key.object_id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["A", "B", "B", "A"],
        "expected cross-object ordering [A,B,B,A], got {ids:?}"
    );
}

// ── AC-3: bridge-sensitive priority ──────────────────────────────────────────

/// AC-3: When a BridgeInfill and a SparseInfill entity are equidistant (within
/// 0.001 mm) from the current position, BridgeInfill appears first.
#[test]
fn bridge_sensitive_entities_are_prioritized_ahead_of_generic_infill() {
    // Both at exactly (5.0, 0.0) — equidistant from start (0,0). Bridge wins.
    let entities = vec![
        sparse(5.0, 0.0, "obj", 0),
        bridge(5.0, 0.0, "obj", 1),
    ];

    let result = order_entities_by_nearest_neighbor(entities);

    assert_eq!(
        result[0].role,
        ExtrusionRole::BridgeInfill,
        "BridgeInfill must appear before SparseInfill when equidistant"
    );
    assert_eq!(
        result[1].role,
        ExtrusionRole::SparseInfill,
        "SparseInfill must appear after BridgeInfill when equidistant"
    );
}

// ── AC-4: determinism across repeated runs ────────────────────────────────────

/// AC-4: Running the ordering helper twice on the same input produces a
/// byte-identical ordered entity sequence.
#[test]
fn path_ordering_is_deterministic_across_repeated_runs() {
    let make = || {
        vec![
            sparse(30.0, 0.0, "obj", 0),
            bridge(5.0, 5.0, "obj", 1),
            sparse(0.0, 0.0, "obj", 2),
            sparse(15.0, 0.0, "obj", 3),
        ]
    };

    let run1 = order_entities_by_nearest_neighbor(make());
    let run2 = order_entities_by_nearest_neighbor(make());

    let xs1: Vec<f32> = run1.iter().map(|e| e.path.points[0].x).collect();
    let xs2: Vec<f32> = run2.iter().map(|e| e.path.points[0].x).collect();
    assert_eq!(
        xs1, xs2,
        "ordering must be deterministic: run1={xs1:?} run2={xs2:?}"
    );
    let topos1: Vec<u32> = run1.iter().map(|e| e.topo_order).collect();
    let topos2: Vec<u32> = run2.iter().map(|e| e.topo_order).collect();
    assert_eq!(
        topos1, topos2,
        "topo_order must be identical across runs: {topos1:?} vs {topos2:?}"
    );
}

// ── AC-5: live-stage integration ──────────────────────────────────────────────

/// AC-5: The path-optimization module receives entities in the reordered
/// sequence (pre-staged by the host ordering helper), not the raw assembled
/// order. Verified by a mock LayerStageRunner that captures the arena's
/// pre-staged LayerCollectionIR when Layer::PathOptimization is invoked.
#[test]
fn reordered_sequence_is_consumed_by_path_optimization_stage() {
    // Raw infill order: (30,0), (10,0), (0,0) — expected NN order: (0,0),(10,0),(30,0).
    let infill = InfillIR {
        schema_version: semver(),
        global_layer_index: 0,
        regions: vec![InfillRegion {
            object_id: "test-object".to_string(),
            region_id: 0,
            sparse_infill: vec![
                path_at(30.0, 0.0),
                path_at(10.0, 0.0),
                path_at(0.0, 0.0),
            ],
            solid_infill: vec![],
            ironing: vec![],
        }],
    };

    let captured: Arc<Mutex<Option<f32>>> = Arc::new(Mutex::new(None));
    let runner = LiveStageCapture {
        infill: infill.clone(),
        captured_first_x: Arc::clone(&captured),
    };

    let mesh = minimal_mesh("test-object");
    let blackboard = Blackboard::new(Arc::clone(&mesh), 1);
    let plan = plan_with_stages(
        vec![
            stage("Layer::Infill", "com.test.infill"),
            stage("Layer::PathOptimization", "com.test.path-opt"),
        ],
        1,
    );

    execute_per_layer(&plan, &blackboard, &runner).expect("per-layer execution must succeed");

    let first_x = captured
        .lock()
        .unwrap()
        .take()
        .expect("PathOptimization mock must have captured ordered_entities");

    assert!(
        (first_x - 0.0_f32).abs() < 1e-4,
        "first entity after ordering must start at x=0.0 (nearest to origin), got x={first_x}"
    );
}

// ── AC-NEG: single / already-optimal sequence unchanged ─────────────────────

/// Negative: A single-entity input or an already-optimal sequence is returned
/// in the original order (topo_order may be reassigned to 0).
#[test]
fn single_or_already_optimal_sequence_is_left_unchanged() {
    // Single entity — must come back as-is.
    let single = vec![sparse(42.0, 7.0, "obj", 5)];
    let result = order_entities_by_nearest_neighbor(single);
    assert_eq!(result.len(), 1, "single-entity output must have length 1");
    assert!(
        (result[0].path.points[0].x - 42.0_f32).abs() < 1e-4,
        "single entity must be preserved"
    );
    assert_eq!(result[0].topo_order, 0, "topo_order for single entity must be 0");

    // Already-optimal: (0,0),(10,0),(30,0) — NN order from origin is this order.
    let optimal = vec![
        sparse(0.0, 0.0, "obj", 0),
        sparse(10.0, 0.0, "obj", 1),
        sparse(30.0, 0.0, "obj", 2),
    ];
    let result = order_entities_by_nearest_neighbor(optimal);
    let xs: Vec<f32> = result.iter().map(|e| e.path.points[0].x).collect();
    assert_eq!(
        xs,
        vec![0.0_f32, 10.0, 30.0],
        "already-optimal sequence must be unchanged: {xs:?}"
    );
}

// ── Live-stage runner ─────────────────────────────────────────────────────────

struct LiveStageCapture {
    infill: InfillIR,
    captured_first_x: Arc<Mutex<Option<f32>>>,
}

impl LayerStageRunner for LiveStageCapture {
    fn run_stage(
        &self,
        stage_id: &StageId,
        _layer: &GlobalLayer,
        _module: &CompiledModule,
        _blackboard: &Blackboard,
        arena: &mut LayerArena,
    ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
        if stage_id == "Layer::Infill" {
            arena.set_infill(self.infill.clone()).expect("infill slot must be free");
        } else if stage_id == "Layer::PathOptimization" {
            if let Some(lc) = arena.layer_collection() {
                if let Some(first) = lc.ordered_entities.first() {
                    if let Some(pt) = first.path.points.first() {
                        *self.captured_first_x.lock().unwrap() = Some(pt.x);
                    }
                }
            }
        }
        Ok((LayerStageOutput::Success, vec![], vec![]))
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn path_at(x: f32, y: f32) -> ExtrusionPath3D {
    ExtrusionPath3D {
        points: vec![pt(x, y)],
        role: ExtrusionRole::SparseInfill,
        speed_factor: 1.0,
    }
}

fn minimal_mesh(object_id: &str) -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 { x: 0.0, y: 0.0, z: 0.1 },
                    Point3 { x: 10.0, y: 0.0, z: 0.1 },
                    Point3 { x: 0.0, y: 10.0, z: 0.1 },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: [1.0,0.0,0.0,0.0, 0.0,1.0,0.0,0.0, 0.0,0.0,1.0,0.0, 0.0,0.0,0.0,1.0],
            },
            config: ObjectConfig { data: HashMap::new() },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 200.0, y: 200.0, z: 200.0 },
        },
    })
}

fn plan_with_stages(per_layer_stages: Vec<CompiledStage>, layer_count: usize) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![],
        per_layer_stages,
        layer_finalization_stage: None,
        postpass_stages: vec![],
        global_layers: Arc::new(
            (0..layer_count)
                .map(|i| GlobalLayer {
                    index: i as u32,
                    z: 0.2 * (i as f32 + 1.0),
                    active_regions: vec![ActiveRegion {
                        object_id: "test-object".to_string(),
                        region_id: 0,
                        resolved_config: ResolvedConfig::default(),
                        effective_layer_height: 0.2,
                        nonplanar_shell: None,
                        is_catchup_layer: false,
                        catchup_z_bottom: 0.0,
                        tool_index: 0,
                    }],
                    has_nonplanar: false,
                    is_sync_layer: i == 0,
                })
                .collect(),
        ),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    }
}

fn stage(stage_id: &str, module_id: &str) -> CompiledStage {
    CompiledStage {
        stage_id: stage_id.to_string(),
        modules: vec![compiled_module(stage_id, module_id)],
    }
}

fn compiled_module(stage_id: &str, module_id: &str) -> CompiledModule {
    let loaded = LoadedModule {
        id: module_id.to_string(),
        version: SemVer { major: 1, minor: 0, patch: 0 },
        stage: stage_id.to_string(),
        wit_world: String::new(),
        ir_reads: vec![],
        ir_writes: vec![],
        claims: vec![],
        requires_claims: vec![],
        incompatible_with: vec![],
        requires_modules: vec![],
        min_host_version: SemVer { major: 0, minor: 1, patch: 0 },
        min_ir_schema: SemVer { major: 1, minor: 0, patch: 0 },
        max_ir_schema: SemVer { major: 2, minor: 0, patch: 0 },
        config_schema: ConfigSchema::default(),
        overridable_per_region: vec![],
        overridable_per_layer: vec![],
        layer_parallel_safe: true,
        wasm_path: PathBuf::from(format!("fixtures/{module_id}.wasm")),
        placeholder_wasm: false,
    };
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata { uses_shared_memory: false },
        )
        .expect("fixture pool"),
    );
    let binding = ExecutionModuleBinding {
        module: loaded,
        instance_pool: Arc::clone(&pool),
        config_view: Arc::new(ConfigView::from_map(HashMap::new())),
        wasm_component: None,
    };
    CompiledModule {
        module_id: binding.module.id.clone(),
        instance_pool: Arc::clone(&pool),
        ir_read_mask: IrAccessMask { paths: vec![] },
        ir_write_mask: IrAccessMask { paths: vec![] },
        config_view: Arc::clone(&binding.config_view),
        wasm_component: None,
    }
}
