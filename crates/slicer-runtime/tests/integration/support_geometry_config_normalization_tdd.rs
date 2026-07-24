//! Regression tests for packet 73 — support-geometry config normalization.
//!
//! AC-2:  `support_raft_layers` and raft-plan config keys reach the guest
//!         planner without producing raft geometry entries.
//! AC-N1: `enable_support = false` produces zero plan entries.
//! AC-N2: an empty layer-plan-view makes the planner return a fatal
//!         `ModuleError`, which the host surfaces as `PrepassExecutionError`.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan, SemVer,
    Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass_with_builtins, instance_pool::WasmArtifactMetadata,
    Blackboard, CompiledModule, CompiledModuleBuilder, CompiledStage, ExecutionPlan,
    LoadedModuleBuilder, PrepassExecutionError, WasmEngine, WasmRuntimeDispatcher,
};

use crate::common::{wasm_cache, TestModuleBundle};

// ── helpers ──────────────────────────────────────────────────────────────────

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn support_planner_wasm() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/support-planner/support-planner.wasm")
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

/// A mesh with a downward-facing overhang plate floating at z≈1.8 mm.
/// Same geometry as `overhang_plate_mesh` in prepass_support_geometry_tdd.rs
/// so we know it produces at least one support entry.
fn overhang_plate_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "plate".to_string(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        z: 1.8,
                        ..Default::default()
                    },
                    Point3 {
                        x: 4.0,
                        z: 1.8,
                        ..Default::default()
                    },
                    Point3 {
                        x: 4.0,
                        y: 4.0,
                        z: 1.8,
                    },
                    Point3 {
                        y: 4.0,
                        z: 1.8,
                        ..Default::default()
                    },
                ],
                indices: vec![1, 3, 2, 1, 4, 3],
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

/// Build a Blackboard with `LayerPlanIR` pre-committed (10 × 0.2 mm layers).
fn blackboard_with_layer_plan(mesh: MeshIR) -> Blackboard {
    let num_layers = 10u32;
    let layer_height = 0.2f32;
    let object_ids: Vec<String> = mesh.objects.iter().map(|o| o.id.clone()).collect();
    let global_layers: Vec<GlobalLayer> = (0..num_layers)
        .map(|i| {
            let regions = object_ids
                .iter()
                .map(|oid| ActiveRegion {
                    object_id: oid.clone(),
                    region_id: 0,
                    resolved_config: slicer_ir::ResolvedConfig::default(),
                    effective_layer_height: layer_height,
                    nonplanar_shell: None,
                    is_catchup_layer: false,
                    catchup_z_bottom: 0.0,
                    tool_index: 0,
                })
                .collect();
            GlobalLayer {
                index: i,
                z: (i + 1) as f32 * layer_height,
                active_regions: regions,
                has_nonplanar: false,
                is_sync_layer: false,
            }
        })
        .collect();
    let mut object_participation = HashMap::new();
    for obj in &mesh.objects {
        object_participation.insert(
            obj.id.clone(),
            (0..num_layers)
                .map(|i| slicer_ir::ObjectLayerRef {
                    local_layer_index: i,
                    global_layer_index: i,
                    effective_layer_height: layer_height,
                })
                .collect(),
        );
    }
    let mut region_entries = HashMap::new();
    for obj in &mesh.objects {
        for i in 0..num_layers {
            region_entries.insert(
                RegionKey {
                    global_layer_index: i,
                    object_id: obj.id.clone(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                RegionPlan::default(),
            );
        }
    }
    let mesh_arc = Arc::new(mesh);
    let mut bb = Blackboard::new(mesh_arc, 0);
    bb.commit_layer_plan(Arc::new(LayerPlanIR {
        global_layers,
        object_participation,
        ..Default::default()
    }))
    .expect("commit_layer_plan must succeed");
    bb.commit_region_map(Arc::new(RegionMapIR {
        entries: region_entries,
        ..Default::default()
    }))
    .expect("commit_region_map must succeed");
    bb
}

fn compile_support_planner_with_config(
    _engine: &Arc<WasmEngine>,
    config: HashMap<String, ConfigValue>,
) -> TestModuleBundle {
    let wasm_path = support_planner_wasm();
    let component = wasm_cache::compiled_component_at(&wasm_path);
    let loaded = LoadedModuleBuilder::new(
        "com.core.support-planner",
        semver(0, 1, 0),
        "PrePass::SupportGeometry",
        slicer_schema::WORLD_PREPASS,
        wasm_path,
    )
    .ir_reads(vec![
        "MeshIR.objects".into(),
        "SurfaceClassificationIR.per_object".into(),
        "LayerPlanIR.global_layers".into(),
        "PaintRegionIR.per_layer".into(),
    ])
    .ir_writes(vec!["SupportPlanIR.entries".into()])
    .claims(vec!["support-planner".into()])
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build();
    let pool = Arc::new(
        build_wasm_instance_pool(
            loaded.id(),
            loaded.stage(),
            loaded.layer_parallel_safe(),
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("instance pool must build"),
    );
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(ConfigView::from_map(config)))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn execution_plan_with_support_geometry(module: CompiledModule) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::SupportGeometry".to_string(),
            modules: vec![module],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::<GlobalLayer>::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        aggregated_region_split: BTreeMap::new(),
    }
}

fn base_config(enabled: bool) -> HashMap<String, ConfigValue> {
    let mut map = HashMap::new();
    map.insert("enable_support".to_string(), ConfigValue::Bool(enabled));
    map.insert(
        "support_branch_angle_deg".to_string(),
        ConfigValue::Float(45.0),
    );
    map.insert(
        "support_branch_merge_distance_mm".to_string(),
        ConfigValue::Float(0.8),
    );
    map.insert(
        "support_max_branches_per_layer".to_string(),
        ConfigValue::Int(1024),
    );
    map.insert("line_width".to_string(), ConfigValue::Float(0.4));
    map
}

// ── AC-2: raft layers config is honored ──────────────────────────────────────

/// AC-2: a slice configured with `support_raft_layers = 2` must produce one
/// configuration-only raft plan, proving the config keys reach the guest.
#[test]
fn raft_layers_config_is_honored() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let mut config = base_config(true);
    config.insert("support_raft_layers".to_string(), ConfigValue::Int(2));

    let bundle = compile_support_planner_with_config(&engine, config);
    let (module, wasm_handles) = bundle.into_module_and_handles();
    let plan = execution_plan_with_support_geometry(module);
    let mut blackboard = blackboard_with_layer_plan(overhang_plate_mesh());

    execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher, &wasm_handles)
        .expect("execute_prepass_with_builtins must succeed");

    let support_plan = blackboard
        .support_plan()
        .expect("SupportPlanIR must be committed after dispatch");

    let raft_plan = support_plan
        .raft_plan
        .as_ref()
        .expect("SupportPlanIR must contain a raft plan");
    assert_eq!(raft_plan.raft_layers, 2);
    assert!((raft_plan.raft_first_layer_density - 0.4).abs() < f32::EPSILON);
    assert_eq!(raft_plan.base_raft_layers, 1);
    assert_eq!(raft_plan.interface_raft_layers, 0);
    assert!(
        support_plan
            .entries
            .iter()
            .all(|entry| entry.global_layer_index >= 0),
        "raft planning must not emit raft geometry entries"
    );
}

// ── AC-N1: support disabled emits no plan ────────────────────────────────────

/// AC-N1: `enable_support = false` must produce zero plan entries.
/// Previously the empty `ConfigView` default forced enabled = true and the
/// planner always ran; now the config flows so the guest respects the flag.
#[test]
fn support_disabled_emits_no_plan() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let config = base_config(false); // enable_support = false

    let bundle = compile_support_planner_with_config(&engine, config);
    let (module, wasm_handles) = bundle.into_module_and_handles();
    let plan = execution_plan_with_support_geometry(module);
    let mut blackboard = blackboard_with_layer_plan(overhang_plate_mesh());

    execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher, &wasm_handles)
        .expect("execute_prepass_with_builtins must succeed");

    let support_plan = blackboard
        .support_plan()
        .expect("SupportPlanIR must be committed even when disabled");

    assert!(
        support_plan.entries.is_empty(),
        "enable_support=false must produce zero SupportPlanIR entries; \
         got {} entries",
        support_plan.entries.len()
    );
}

// ── AC-N2: planner fatal surfaces as dispatch error ──────────────────────────

/// AC-N2: when the layer-plan view is empty the planner returns
/// `Err(ModuleError::fatal(1, "empty layer-plan-view"))`.
/// The host must surface this as `PrepassExecutionError::FatalModule` (not
/// swallow it), proving that guest `Err` becomes a `DispatchError` that
/// propagates up.
#[test]
fn planner_fatal_surfaces_as_dispatch_error() {
    use slicer_ir::{ObjectLayerRef, RegionMapIR};

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let config = base_config(true);
    let bundle = compile_support_planner_with_config(&engine, config);
    let (module, wasm_handles) = bundle.into_module_and_handles();
    let plan = execution_plan_with_support_geometry(module);

    // Build a blackboard whose LayerPlanIR has zero global_layers — the
    // planner will see an empty LayerPlanView and return a fatal error.
    let mesh = overhang_plate_mesh();
    let mesh_arc = Arc::new(mesh.clone());
    let mut bb = Blackboard::new(Arc::clone(&mesh_arc), 0);

    // Commit an empty LayerPlanIR (zero global_layers) so the prerequisite
    // check passes but the guest sees an empty view.
    let mut object_participation = HashMap::new();
    for obj in &mesh.objects {
        object_participation.insert(obj.id.clone(), Vec::<ObjectLayerRef>::new());
    }
    bb.commit_layer_plan(Arc::new(LayerPlanIR {
        global_layers: Vec::new(), // ← empty: triggers guest fatal
        object_participation,
        ..Default::default()
    }))
    .expect("commit_layer_plan must succeed");
    bb.commit_region_map(Arc::new(RegionMapIR::default()))
        .expect("commit_region_map must succeed");

    let result = execute_prepass_with_builtins(&plan, &mut bb, &dispatcher, &wasm_handles);

    match result {
        Err(PrepassExecutionError::FatalModule { module_id, .. }) => {
            assert!(
                module_id.contains("support-planner"),
                "FatalModule error must identify the support-planner module; got: {module_id}"
            );
        }
        Err(other) => panic!(
            "expected PrepassExecutionError::FatalModule for empty layer-plan-view; \
             got: {other:?}"
        ),
        Ok(_) => {
            panic!("expected an error for empty layer-plan-view; dispatch succeeded unexpectedly")
        }
    }
}
