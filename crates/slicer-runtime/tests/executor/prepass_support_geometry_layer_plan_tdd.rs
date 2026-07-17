//! Integration TDD tests: `PrePass::SupportGeometry` contract with
//! proper `LayerPlanIR` + `RegionMapIR` fixtures.
//!
//! Verifies AC-7 (variable-layer-height walk), AC-8 (multi-region entry
//! emission), negative ACs (missing RegionMap, empty region map), and
//! determinism of the host-side projector functions.
//!
//! Tests marked "WILL FAIL" require the Step 9 planner implementation
//! and Step 11 WASM rebuild before they pass.

#![allow(missing_docs)]

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectLayerRef, ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan, SemVer, SupportPlanIR,
    Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass_with_builtins, instance_pool::WasmArtifactMetadata,
    Blackboard, CompiledModule, CompiledModuleBuilder, CompiledStage, ExecutionPlan, LoadedModule,
    LoadedModuleBuilder, PrepassExecutionError, WasmEngine, WasmRuntimeDispatcher,
};

use crate::common::{wasm_cache, TestModuleBundle};

// â”€â”€ Fixtures â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn support_planner_wasm() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/support-planner/support-planner.wasm")
}

/// Overhang plate mesh with configurable object ID.
fn overhang_mesh(object_id: &str) -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: object_id.to_string(),
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

/// Variable-height LayerPlanIR with 4 layers at z = 0.4, 0.8, 1.2, 2.0.
fn variable_height_layer_plan() -> LayerPlanIR {
    LayerPlanIR {
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: 0.4,
                active_regions: vec![],
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 1,
                z: 0.8,
                active_regions: vec![],
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 2,
                z: 1.2,
                active_regions: vec![],
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 3,
                z: 2.0,
                active_regions: vec![],
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ],
        object_participation: {
            let mut m = HashMap::new();
            m.insert(
                "plate".to_string(),
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
                    ObjectLayerRef {
                        local_layer_index: 3,
                        global_layer_index: 3,
                        effective_layer_height: 0.8,
                    },
                ],
            );
            m
        },
        ..Default::default()
    }
}

/// LayerPlanIR with a single layer 5 for the multi-region fixture.
/// Z must be >= 1.8 so the overhang contact (centroid at zâ‰ˆ1.8) lands on it.
fn multi_region_layer_plan() -> LayerPlanIR {
    LayerPlanIR {
        global_layers: vec![GlobalLayer {
            index: 5,
            z: 2.0,
            active_regions: vec![],
            has_nonplanar: false,
            is_sync_layer: false,
        }],
        object_participation: {
            let mut m = HashMap::new();
            m.insert(
                "obj-multi".to_string(),
                vec![ObjectLayerRef {
                    local_layer_index: 0,
                    global_layer_index: 5,
                    effective_layer_height: 0.2,
                }],
            );
            m
        },
        ..Default::default()
    }
}

/// Single-region RegionMapIR for the given object.
fn simple_region_map(object_id: &str, num_layers: u32) -> RegionMapIR {
    let mut entries = HashMap::new();
    for layer_idx in 0..num_layers {
        entries.insert(
            RegionKey {
                global_layer_index: layer_idx,
                object_id: object_id.to_string(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            RegionPlan::default(),
        );
    }
    RegionMapIR {
        entries,
        ..Default::default()
    }
}

/// Multi-region RegionMapIR: two regions (7, 42) for "obj-multi" on layer 5.
fn multi_region_map() -> RegionMapIR {
    let mut entries = HashMap::new();
    entries.insert(
        RegionKey {
            global_layer_index: 5,
            object_id: "obj-multi".to_string(),
            region_id: 7,
            variant_chain: Vec::new(),
        },
        RegionPlan::default(),
    );
    entries.insert(
        RegionKey {
            global_layer_index: 5,
            object_id: "obj-multi".to_string(),
            region_id: 42,
            variant_chain: Vec::new(),
        },
        RegionPlan::default(),
    );
    RegionMapIR {
        entries,
        ..Default::default()
    }
}

fn default_planner_config_map() -> HashMap<String, ConfigValue> {
    let mut map = HashMap::new();
    map.insert("support_enabled".to_string(), ConfigValue::Bool(true));
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

fn loaded_support_planner_module(id: &str, wasm_path: std::path::PathBuf) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
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
    .build()
}

fn compile_support_planner(engine: &Arc<WasmEngine>) -> TestModuleBundle {
    let wasm_path = support_planner_wasm();
    let bytes = std::fs::read(&wasm_path).unwrap_or_else(|_| {
        panic!(
            "support-planner.wasm not found at {}. Build with: \
             `cargo xtask build-guests`",
            wasm_path.display()
        )
    });
    let component = Arc::new(
        engine
            .compile_component(&bytes)
            .expect("support-planner.wasm must compile"),
    );
    let loaded = loaded_support_planner_module("com.core.support-planner", wasm_path);
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
        .config_view(Arc::new(ConfigView::from_map(default_planner_config_map())))
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

/// Build a Blackboard with mesh, LayerPlanIR, and RegionMapIR committed.
fn blackboard_with_layer_plan_and_region_map(
    mesh: MeshIR,
    layer_plan: LayerPlanIR,
    region_map: RegionMapIR,
) -> Blackboard {
    let mesh_arc = Arc::new(mesh);
    let mut bb = Blackboard::new(mesh_arc, 0);
    bb.commit_layer_plan(Arc::new(layer_plan))
        .expect("commit_layer_plan must succeed");
    bb.commit_region_map(Arc::new(region_map))
        .expect("commit_region_map must succeed");
    bb
}

/// Build a Blackboard with mesh and LayerPlanIR only (no RegionMapIR).
fn blackboard_with_layer_plan_no_region_map(mesh: MeshIR, layer_plan: LayerPlanIR) -> Blackboard {
    let mesh_arc = Arc::new(mesh);
    let mut bb = Blackboard::new(mesh_arc, 0);
    bb.commit_layer_plan(Arc::new(layer_plan))
        .expect("commit_layer_plan must succeed");
    bb
}

/// Run the full prepass pipeline and return the committed SupportPlanIR.
fn run_prepass(
    mesh: MeshIR,
    layer_plan: LayerPlanIR,
    region_map: RegionMapIR,
) -> Arc<SupportPlanIR> {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let bundle = compile_support_planner(&engine);
    let (module, wasm_handles) = bundle.into_module_and_handles();
    let plan = execution_plan_with_support_geometry(module);

    let mut blackboard = blackboard_with_layer_plan_and_region_map(mesh, layer_plan, region_map);
    execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher, &wasm_handles)
        .expect("execute_prepass_with_builtins must succeed");

    Arc::clone(
        blackboard
            .support_plan()
            .expect("SupportPlanIR must be committed after live dispatch"),
    )
}

/// Run the full prepass pipeline and return the result (or error).
/// Note: with the two-phase execution (packet 31a), when LayerPlanIR exists in
/// the blackboard, RegionMapping runs in phase-1 and commits RegionMap before
/// execute_prepass. So SupportGeometry succeeds even without an explicit
/// RegionMap in the test setup.
fn run_prepass_for_layer_plan_only(
    mesh: MeshIR,
    layer_plan: LayerPlanIR,
) -> Result<SupportPlanIR, PrepassExecutionError> {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let bundle = compile_support_planner(&engine);
    let (module, wasm_handles) = bundle.into_module_and_handles();
    let plan = execution_plan_with_support_geometry(module);

    let mut blackboard = blackboard_with_layer_plan_no_region_map(mesh, layer_plan);
    execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher, &wasm_handles).map(
        |_audits| {
            // The support planner should have committed SupportPlanIR.
            let support_plan = blackboard
                .support_plan()
                .expect("SupportPlanIR must be committed after successful run");
            // Clone the Arc contents to satisfy the return type.
            (**support_plan).clone()
        },
    )
}

// â”€â”€ AC-7: variable-layer-height walk (positive, WILL FAIL) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn planner_walks_real_layer_plan_with_variable_layer_heights() {
    let mesh = overhang_mesh("plate");
    let layer_plan = variable_height_layer_plan();
    let region_map = simple_region_map("plate", 4);

    let plan_ir = run_prepass(mesh, layer_plan, region_map);

    // All entries must carry global_layer_index in {0, 1, 2, 3}.
    for entry in &plan_ir.entries {
        assert!(
            entry.global_layer_index <= 3,
            "entry has global_layer_index={}, expected <= 3",
            entry.global_layer_index
        );
    }

    // The highest entry's branch_segments[*][0].z must be within 1e-4 of 2.0.
    let highest = plan_ir
        .entries
        .iter()
        .max_by_key(|e| e.global_layer_index)
        .expect("SupportPlanIR must have at least one entry");
    for seg in &highest.branch_segments {
        let first_z = seg.points[0].z;
        assert!(
            (first_z - 2.0).abs() < 1e-4,
            "highest entry first point z={} expected ~2.0",
            first_z
        );
    }
}

// â”€â”€ AC-8: multi-region entry emission (positive, WILL FAIL) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn planner_emits_one_entry_per_region_in_region_map() {
    let mesh = overhang_mesh("obj-multi");
    let layer_plan = multi_region_layer_plan();
    let region_map = multi_region_map();

    let plan_ir = run_prepass(mesh, layer_plan, region_map);

    // Must have exactly 2 entries for (layer=5, object="obj-multi").
    let matching: Vec<_> = plan_ir
        .entries
        .iter()
        .filter(|e| e.global_layer_index == 5 && e.object_id == "obj-multi")
        .collect();
    assert_eq!(
        matching.len(),
        2,
        "expected 2 entries for (layer=5, object=obj-multi), got {}",
        matching.len()
    );

    // One must have region_id=7, the other region_id=42.
    let region_ids: Vec<u64> = matching.iter().map(|e| e.region_id).collect();
    assert!(
        region_ids.contains(&7),
        "expected region_id=7, got {:?}",
        region_ids
    );
    assert!(
        region_ids.contains(&42),
        "expected region_id=42, got {:?}",
        region_ids
    );

    // Byte-identical branch_segments between the two entries.
    let entry_7 = matching.iter().find(|e| e.region_id == 7).unwrap();
    let entry_42 = matching.iter().find(|e| e.region_id == 42).unwrap();
    assert_eq!(
        entry_7.branch_segments.len(),
        entry_42.branch_segments.len(),
        "branch_segments length mismatch between region 7 and 42"
    );
    for (seg_7, seg_42) in entry_7
        .branch_segments
        .iter()
        .zip(entry_42.branch_segments.iter())
    {
        assert_eq!(seg_7.points.len(), seg_42.points.len());
        for (p7, p42) in seg_7.points.iter().zip(seg_42.points.iter()) {
            assert_eq!(p7.x.to_bits(), p42.x.to_bits());
            assert_eq!(p7.y.to_bits(), p42.y.to_bits());
            assert_eq!(p7.z.to_bits(), p42.z.to_bits());
            assert_eq!(p7.width.to_bits(), p42.width.to_bits());
        }
    }
}

// â”€â”€ Positive: RegionMap provided by built-in RegionMapping (phase-1) â”€â”€â”€â”€â”€â”€â”€â”€
// With two-phase execution (packet 31a), when LayerPlanIR exists in the
// blackboard, RegionMapping runs in phase-1 and commits RegionMap before
// execute_prepass. So SupportGeometry succeeds even without an explicit
// RegionMap in the test setup. This verifies the built-in RegionMapping path.

#[test]
fn prepass_support_generation_succeeds_with_builtin_region_mapping() {
    let mesh = overhang_mesh("plate");
    let layer_plan = variable_height_layer_plan();

    // RegionMapping runs in phase-1 (LayerPlanIR exists) and commits RegionMap.
    // SupportGeometry then runs in phase-2 and succeeds.
    let result = run_prepass_for_layer_plan_only(mesh, layer_plan);
    // The result is Ok(SupportPlanIR) â€” no error should occur.
    assert!(
        result.is_ok(),
        "execute_prepass_with_builtins must succeed when LayerPlanIR is present \
         (RegionMapping runs in phase-1, committing RegionMap before SupportGeometry)"
    );
}

// â”€â”€ Negative: empty region map (WILL FAIL) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn planner_skips_object_with_empty_region_map() {
    let mesh = overhang_mesh("plate");
    let layer_plan = variable_height_layer_plan();
    let empty_region_map = RegionMapIR::default();

    let plan_ir = run_prepass(mesh, layer_plan, empty_region_map);

    // With an empty region map the planner must produce zero entries.
    assert!(
        plan_ir.entries.is_empty(),
        "expected zero SupportPlanIR entries for empty region map, got {}",
        plan_ir.entries.len()
    );
}

// â”€â”€ Determinism: projector output ordering (SHOULD PASS now) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn host_projector_orders_region_segmentation_deterministically() {
    // Build a RegionMapIR with several entries in insertion order.
    let mut entries = HashMap::new();
    entries.insert(
        RegionKey {
            global_layer_index: 2,
            object_id: "z-obj".to_string(),
            region_id: 99,
            variant_chain: Vec::new(),
        },
        RegionPlan::default(),
    );
    entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "a-obj".to_string(),
            region_id: 1,
            variant_chain: Vec::new(),
        },
        RegionPlan::default(),
    );
    entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "a-obj".to_string(),
            region_id: 5,
            variant_chain: Vec::new(),
        },
        RegionPlan::default(),
    );
    entries.insert(
        RegionKey {
            global_layer_index: 1,
            object_id: "m-obj".to_string(),
            region_id: 3,
            variant_chain: Vec::new(),
        },
        RegionPlan::default(),
    );
    let region_map = RegionMapIR {
        entries,
        ..Default::default()
    };

    // Project twice and compare (WIT-generated types lack PartialEq,
    // so compare entry-by-entry).
    let view_1 = slicer_runtime::wit_host::project_region_segmentation_view(&region_map);
    let view_2 = slicer_runtime::wit_host::project_region_segmentation_view(&region_map);

    assert_eq!(
        view_1.entries.len(),
        view_2.entries.len(),
        "projector must be deterministic (length mismatch)"
    );
    for (a, b) in view_1.entries.iter().zip(view_2.entries.iter()) {
        assert_eq!(a.layer_index, b.layer_index);
        assert_eq!(a.object_id, b.object_id);
        assert_eq!(a.region_ids, b.region_ids);
    }

    // Verify sort order: (layer_index ASC, object_id ASC).
    for w in view_1.entries.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        assert!(
            (a.layer_index, &a.object_id) <= (b.layer_index, &b.object_id),
            "entries not sorted: ({}, {}) > ({}, {})",
            a.layer_index,
            a.object_id,
            b.layer_index,
            b.object_id
        );
    }

    // Verify region_ids within each entry are sorted ASC.
    for entry in &view_1.entries {
        for w in entry.region_ids.windows(2) {
            assert!(
                w[0] <= w[1],
                "region_ids not sorted in entry (layer={}, object={}): {:?}",
                entry.layer_index,
                entry.object_id,
                entry.region_ids
            );
        }
    }
}

#[test]
fn host_projector_orders_layer_plan_deterministically() {
    let layer_plan = variable_height_layer_plan();

    let view_1 = slicer_runtime::wit_host::project_layer_plan_view(&layer_plan);
    let view_2 = slicer_runtime::wit_host::project_layer_plan_view(&layer_plan);

    assert_eq!(
        view_1.layers.len(),
        view_2.layers.len(),
        "layer plan projector must be deterministic (length mismatch)"
    );
    for (a, b) in view_1.layers.iter().zip(view_2.layers.iter()) {
        assert_eq!(a.global_layer_index, b.global_layer_index);
        assert!((a.z - b.z).abs() < 1e-6, "z mismatch");
        assert!(
            (a.effective_layer_height - b.effective_layer_height).abs() < 1e-6,
            "effective_layer_height mismatch"
        );
    }

    // Verify sort order: global_layer_index ASC.
    for w in view_1.layers.windows(2) {
        assert!(
            w[0].global_layer_index <= w[1].global_layer_index,
            "layers not sorted: {} > {}",
            w[0].global_layer_index,
            w[1].global_layer_index
        );
    }
}
