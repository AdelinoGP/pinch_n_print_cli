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

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectLayerRef, ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SemVer,
    SupportPlanIR, SupportType, Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass_with_builtins,
    execute_prepass_with_builtins_configured, instance_pool::WasmArtifactMetadata, Blackboard,
    CompiledModule, CompiledModuleBuilder, CompiledStage, ConfigBoundsIndex, ExecutionPlan,
    LoadedModule, LoadedModuleBuilder, PrepassExecutionError, WasmEngine, WasmRuntimeDispatcher,
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
        .join("modules/core-modules/tree-support-planner/tree-support-planner.wasm")
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
                ..Default::default()
            },
            GlobalLayer {
                index: 1,
                z: 0.8,
                ..Default::default()
            },
            GlobalLayer {
                index: 2,
                z: 1.2,
                ..Default::default()
            },
            GlobalLayer {
                index: 3,
                z: 2.0,
                ..Default::default()
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

/// LayerPlanIR for the multi-region fixture: global layers 2..=5, with layer 5
/// at z = 2.0 so the overhang (facet centroid z = 1.8) is CONTAINED by it.
///
/// CANONICAL JUSTIFICATION for the layers below 5, which this fixture did not
/// have before packet 224 (it was a single layer, index 5, z = 2.0):
/// canonical `generate_contact_points` (`TreeSupport.cpp`) is the only contact
/// source, it iterates `layer_nr` from 1, and its `insert_point` lambda calls
/// `create_node(pt, -gap_layers, layer_nr - 1, ...)` — the contact ALWAYS
/// lands one layer below the overhang, as the virtual top-Z-gap node that
/// `draw_circles` diverts into `roof_gap_areas` and never extrudes. The same
/// function returns early when
/// `m_object->layers().size() <= z_distance_top_layers + 1` ("fix bug of
/// generating support for very thin objects"). A one-layer plan therefore
/// carries no tree support under canonical rules at all, and the old fixture
/// only produced entries because this module used to seed the contact ON the
/// overhang layer — i.e. inside the model. AC-8 ("one entry per region in the
/// region map") is unchanged and asserted below; only the layer stack it runs
/// on is now canonically viable.
fn multi_region_layer_plan() -> LayerPlanIR {
    LayerPlanIR {
        global_layers: (2..=5)
            .map(|index| GlobalLayer {
                index,
                z: index as f32 * 0.4,
                ..Default::default()
            })
            .collect(),
        object_participation: {
            let mut m = HashMap::new();
            m.insert(
                "obj-multi".to_string(),
                (2..=5)
                    .map(|index| ObjectLayerRef {
                        local_layer_index: index - 2,
                        global_layer_index: index,
                        effective_layer_height: 0.4,
                    })
                    .collect(),
            );
            m
        },
        ..Default::default()
    }
}

/// Single-region RegionMapIR for the given object.
fn simple_region_map(object_id: &str, num_layers: u32) -> RegionMapIR {
    let mut entries = HashMap::new();
    let mut region_map = RegionMapIR::default();
    let tree_config = region_map.intern_config(ResolvedConfig {
        support_type: SupportType::TreeAuto,
        ..ResolvedConfig::default()
    });
    for layer_idx in 0..num_layers {
        entries.insert(
            RegionKey {
                global_layer_index: layer_idx,
                object_id: object_id.to_string(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                config: tree_config,
                ..RegionPlan::default()
            },
        );
    }
    region_map.entries = entries;
    region_map
}

/// Multi-region RegionMapIR: two regions (7, 42) for "obj-multi" on every
/// layer of `multi_region_layer_plan`. Both regions must exist on every layer
/// the planner can emit on, because the emit pass looks the region set up by
/// (object, global layer) — and canonical never emits on the overhang layer
/// itself (see `multi_region_layer_plan`).
fn multi_region_map() -> RegionMapIR {
    let mut entries = HashMap::new();
    let mut region_map = RegionMapIR::default();
    let tree_config = region_map.intern_config(ResolvedConfig {
        support_type: SupportType::TreeAuto,
        ..ResolvedConfig::default()
    });
    for global_layer_index in 2..=5 {
        for region_id in [7, 42] {
            entries.insert(
                RegionKey {
                    global_layer_index,
                    object_id: "obj-multi".to_string(),
                    region_id,
                    variant_chain: Vec::new(),
                },
                RegionPlan {
                    config: tree_config,
                    ..RegionPlan::default()
                },
            );
        }
    }
    region_map.entries = entries;
    region_map
}

fn default_planner_config_map() -> HashMap<String, ConfigValue> {
    let mut map = HashMap::new();
    map.insert("enable_support".to_string(), ConfigValue::Bool(true));
    map.insert(
        "tree_support_branch_angle".to_string(),
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
        slicer_schema::TIER_PREPASS,
        wasm_path,
    )
    .ir_reads(vec![
        "MeshIR.objects".into(),
        "SurfaceClassificationIR.per_object".into(),
        "LayerPlanIR.global_layers".into(),
        "RegionMapIR.entries".into(),
        "SupportGeometryIR.entries".into(),
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
    let loaded = loaded_support_planner_module("com.core.tree-support-planner", wasm_path);
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
        ..Default::default()
    }
}

/// Build a Blackboard with mesh, LayerPlanIR, and RegionMapIR committed.
fn blackboard_with_layer_plan_and_region_map(
    mesh: MeshIR,
    layer_plan: LayerPlanIR,
    region_map: RegionMapIR,
    // exhaustive: Blackboard explicit test fixture preserves boundary data
) -> Blackboard {
    let mesh_arc = Arc::new(mesh);
    let mut bb = Blackboard::new(mesh_arc, 0);
    bb.commit_layer_plan(Arc::new(layer_plan))
        .expect("commit_layer_plan must succeed");
    bb.commit_region_map(Arc::new(region_map))
        .expect("commit_region_map must succeed");
    bb
    // exhaustive: Blackboard explicit test fixture preserves boundary data
}

/// Build a Blackboard with mesh and LayerPlanIR only (no RegionMapIR).
// exhaustive: Blackboard explicit test fixture preserves boundary data
fn blackboard_with_layer_plan_no_region_map(mesh: MeshIR, layer_plan: LayerPlanIR) -> Blackboard {
    let mesh_arc = Arc::new(mesh);
    let mut bb = Blackboard::new(mesh_arc, 0);
    bb.commit_layer_plan(Arc::new(layer_plan))
        .expect("commit_layer_plan must succeed");
    bb
    // exhaustive: Blackboard explicit test fixture preserves boundary data
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
    let enabled_config = ResolvedConfig {
        support_enabled: true,
        ..ResolvedConfig::default()
    };
    execute_prepass_with_builtins_configured(
        &plan,
        &mut blackboard,
        &dispatcher,
        &std::collections::BTreeMap::new(),
        &enabled_config,
        &HashMap::new(),
        &ConfigBoundsIndex::empty(),
        &wasm_handles,
    )
    .expect("execute_prepass_with_builtins_configured must succeed");
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

    // Every entry's skeleton must sit at the LayerPlanIR Z of its own layer.
    // That is the AC-7 property: the planner walks the committed plan instead
    // of assuming a uniform `index * layer_height` stack. Layer 3 is the only
    // one whose plan Z (2.0) differs from the uniform-height guess (1.6), and
    // layer 1's plan Z (0.8) differs from a first-layer-relative guess, so the
    // check still discriminates against a planner that ignores the plan.
    let plan_z = |global_layer_index: i32| -> f32 {
        variable_height_layer_plan()
            .global_layers
            .iter()
            .find(|layer| layer.index as i32 == global_layer_index)
            .expect("entry layer must exist in the plan")
            .z
    };
    for entry in &plan_ir.entries {
        let expected_z = plan_z(entry.global_layer_index);
        for point in &entry.skeleton.as_ref().expect("skeleton").points {
            assert!(
                (point.z - expected_z).abs() < 1e-4,
                "entry at layer {} has skeleton z={}, expected the plan's z={}",
                entry.global_layer_index,
                point.z,
                expected_z
            );
        }
    }

    // CANONICAL: the topmost PRINTED support layer is TWO layers below the
    // overhang layer, not the overhang layer itself.
    //
    // The plate's downward facets sit at z = 1.8, which the plan contains in
    // layer 3 (z = 2.0, bottom_z = 1.2). Canonical `generate_contact_points`'
    // `insert_point` lambda (`TreeSupport.cpp`) has exactly one seeding rule —
    // `create_node(pt, -gap_layers, layer_nr - 1, ..., bottom_z,
    // z_distance_top, 0, radius)` — so the contact node lands on layer 2 with
    // `distance_to_top = -gap_layers`. That node is VIRTUAL: `draw_circles`
    // sends `distance_to_top < 0 && !is_sharp_tail` into `roof_gap_areas`,
    // which `generate_toolpaths` never fills. The first extruded cross-section
    // is therefore its descendant on layer 1, z = 0.8.
    //
    // This assertion used to require z = 2.0 — support printed on the same
    // layer as the overhang, i.e. inside the model, with no top Z gap at all.
    // That was the module's pre-packet-224 behaviour (contacts were seeded on
    // the overhang layer); it has no canonical counterpart.
    let highest = plan_ir
        .entries
        .iter()
        .max_by_key(|e| e.global_layer_index)
        .expect("SupportPlanIR must have at least one entry");
    assert_eq!(
        highest.global_layer_index, 1,
        "topmost printed support layer must be two below the overhang layer 3          (layer 2 is canonical's virtual top-Z-gap node, never extruded); got {}",
        highest.global_layer_index
    );
    for point in &highest.skeleton.as_ref().expect("skeleton").points {
        assert!(
            (point.z - 0.8).abs() < 1e-4,
            "highest entry point z={} expected the plan z of layer 1 (0.8)",
            point.z
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

    // AC-8 unchanged: ONE entry per region in the region map, for every layer
    // the planner emits on, with byte-identical structural skeletons.
    //
    // KNOWN RED (worker wV, packet 224 follow-up), and deliberately NOT
    // weakened. Two independent causes were separated here:
    //
    //   1. FIXED — the fixture was a one-layer plan, on which canonical
    //      `generate_contact_points` can seed no contact at all (see
    //      `multi_region_layer_plan`). It produced ZERO entries, which masked
    //      cause 2 entirely.
    //   2. OPEN — with a viable layer stack the planner emits ONE entry
    //      (region 7), not two. `SupportAnalysisIR.family_assignments`
    //      (`crates/slicer-runtime/src/builtins/support_analysis_producer.rs`)
    //      is minted per CANDIDATE, and candidates are derived from `SliceIR`
    //      regions — this fixture's plate yields a single sliced region. The
    //      planner then declines region 42 by design: `candidate_family`
    //      ("No self-default: a region the host did not assign to this family
    //      is not this planner's to plan"). So "one entry per RegionMap
    //      region" and "one entry per host-ASSIGNED region" have diverged.
    //      Which of the two AC-8 means is a spec decision, not a code fix, and
    //      is left to the planner owner rather than resolved by relaxing the
    //      count below.
    //
    // The layer is derived from the plan rather than pinned at 5 because
    // canonical never emits support on the overhang layer itself — see
    // `multi_region_layer_plan` for the `create_node(pt, -gap_layers,
    // layer_nr - 1, ...)` citation. Pinning layer 5 would assert the
    // pre-packet-224 behaviour of seeding contacts inside the model.
    let mine: Vec<_> = plan_ir
        .entries
        .iter()
        .filter(|e| e.object_id == "obj-multi")
        .collect();
    assert!(
        !mine.is_empty(),
        "planner emitted no entries for obj-multi; AC-8 cannot be observed"
    );

    let mut layers: Vec<i32> = mine.iter().map(|e| e.global_layer_index).collect();
    layers.sort_unstable();
    layers.dedup();
    for layer in layers {
        let matching: Vec<_> = mine
            .iter()
            .filter(|e| e.global_layer_index == layer)
            .collect();
        assert_eq!(
            matching.len(),
            2,
            "expected 2 entries for (layer={layer}, object=obj-multi), got {}",
            matching.len()
        );

        // One must have region_id=7, the other region_id=42.
        let region_ids: Vec<u64> = matching.iter().map(|e| e.region_id).collect();
        assert!(
            region_ids.contains(&7),
            "expected region_id=7 at layer {layer}, got {region_ids:?}"
        );
        assert!(
            region_ids.contains(&42),
            "expected region_id=42 at layer {layer}, got {region_ids:?}"
        );

        // Byte-identical structural skeletons between the two entries.
        let entry_7 = matching.iter().find(|e| e.region_id == 7).unwrap();
        let entry_42 = matching.iter().find(|e| e.region_id == 42).unwrap();
        assert_eq!(
            entry_7.skeleton.as_ref().unwrap().points.len(),
            entry_42.skeleton.as_ref().unwrap().points.len(),
            "skeleton length mismatch between region 7 and 42 at layer {layer}"
        );
        for (seg_7, seg_42) in entry_7
            .skeleton
            .as_ref()
            .unwrap()
            .points
            .iter()
            .zip(entry_42.skeleton.as_ref().unwrap().points.iter())
        {
            assert_eq!(seg_7.x.to_bits(), seg_42.x.to_bits());
            assert_eq!(seg_7.y.to_bits(), seg_42.y.to_bits());
            assert_eq!(seg_7.z.to_bits(), seg_42.z.to_bits());
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
