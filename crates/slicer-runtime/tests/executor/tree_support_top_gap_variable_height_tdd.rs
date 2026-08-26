//! Live-WASM regression for canonical tree-support top-gap placement.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet, LayerPlanIR, MeshIR,
    ObjectLayerRef, ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan, ResolvedConfig, SemVer,
    SupportPlanIR, SupportType, Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass_with_builtins_configured_collecting,
    instance_pool::WasmArtifactMetadata, Blackboard, CompiledModuleBuilder, CompiledStage,
    ConfigBoundsIndex, ExecutionPlan, LoadedModuleBuilder, WasmEngine, WasmRuntimeDispatcher,
};

use crate::common::{wasm_cache, TestModuleBundle};

const OBJECT_ID: &str = "variable-height-gap-plate";
const NOMINAL_LAYER_HEIGHT_MM: f32 = 0.2;
const TOP_GAP_MM: f32 = 0.2;

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn overhang_mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: OBJECT_ID.to_string(),
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
                // Downward-facing plate at z=1.8, contained by layer 7.
                indices: vec![1, 3, 2, 1, 4, 3],
            },
            transform: identity(),
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

fn variable_height_layer_plan() -> LayerPlanIR {
    // The final model layer is 0.6 mm high. An accumulated-Z walk for a
    // 0.2 mm gap would stop at layer 6; canonical count placement prints first
    // on layer 5 after propagating through the virtual node on layer 6.
    let layer_z = [0.2_f32, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4, 2.0];
    let heights = [0.2_f32, 0.2, 0.2, 0.2, 0.2, 0.2, 0.2, 0.6];
    LayerPlanIR {
        global_layers: layer_z
            .iter()
            .enumerate()
            .map(|(index, z)| GlobalLayer {
                index: index as u32,
                z: *z,
                ..Default::default()
            })
            .collect(),
        object_participation: HashMap::from([(
            OBJECT_ID.to_string(),
            heights
                .iter()
                .enumerate()
                .map(|(index, height)| ObjectLayerRef {
                    local_layer_index: index as u32,
                    global_layer_index: index as u32,
                    effective_layer_height: *height,
                })
                .collect(),
        )]),
        ..Default::default()
    }
}

fn region_map() -> RegionMapIR {
    let mut map = RegionMapIR::default();
    let config = map.intern_config(ResolvedConfig {
        support_type: SupportType::TreeAuto,
        ..ResolvedConfig::default()
    });
    map.entries = (0..8)
        .map(|global_layer_index| {
            (
                RegionKey {
                    global_layer_index,
                    object_id: OBJECT_ID.to_string(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                RegionPlan {
                    config,
                    ..RegionPlan::default()
                },
            )
        })
        .collect();
    map
}

fn planner_config() -> ConfigView {
    ConfigView::from_map(HashMap::from([
        ("enable_support".to_string(), ConfigValue::Bool(true)),
        (
            "tree_support_branch_angle".to_string(),
            ConfigValue::Float(45.0),
        ),
        (
            "support_branch_merge_distance_mm".to_string(),
            ConfigValue::Float(0.8),
        ),
        (
            "support_max_branches_per_layer".to_string(),
            ConfigValue::Int(1024),
        ),
        ("line_width".to_string(), ConfigValue::Float(0.4)),
        (
            "support_top_z_distance_mm".to_string(),
            ConfigValue::Float(TOP_GAP_MM as f64),
        ),
    ]))
}

fn compile_planner(engine: &Arc<WasmEngine>) -> TestModuleBundle {
    let wasm_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/tree-support-planner/tree-support-planner.wasm");
    let bytes = std::fs::read(&wasm_path).expect("tree-support-planner WASM must exist");
    let component = Arc::new(
        engine
            .compile_component(&bytes)
            .expect("tree-support-planner WASM must compile"),
    );
    let loaded = LoadedModuleBuilder::new(
        "com.core.tree-support-planner",
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
    TestModuleBundle {
        module: CompiledModuleBuilder::new(loaded.id().to_string())
            .config_view(Arc::new(planner_config()))
            .build(),
        pool,
        component: Some(component),
    }
}

fn run_planner() -> Arc<SupportPlanIR> {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let bundle = compile_planner(&engine);
    let (module, wasm_handles) = bundle.into_module_and_handles();
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::SupportGeometry".to_string(),
            modules: vec![module],
        }],
        ..Default::default()
    };
    let mut blackboard = Blackboard::new(Arc::new(overhang_mesh()), 0);
    blackboard
        .commit_layer_plan(Arc::new(variable_height_layer_plan()))
        .expect("layer plan commit must succeed");
    blackboard
        .commit_region_map(Arc::new(region_map()))
        .expect("region map commit must succeed");
    let enabled = ResolvedConfig {
        support_enabled: true,
        ..ResolvedConfig::default()
    };
    let (_, entries) = execute_prepass_with_builtins_configured_collecting(
        &plan,
        &mut blackboard,
        &dispatcher,
        &std::collections::BTreeMap::new(),
        &enabled,
        &HashMap::new(),
        &ConfigBoundsIndex::empty(),
        &wasm_handles,
    )
    .expect("live tree-support planner dispatch must succeed");
    Arc::new(SupportPlanIR {
        entries,
        ..SupportPlanIR::default()
    })
}

#[test]
fn canonical_top_gap_uses_nominal_layer_count_with_variable_heights() {
    let plan = run_planner();
    assert!(!plan.entries.is_empty(), "fixture must emit tree support");

    let gap_units = (TOP_GAP_MM * 10_000.0).round() as i64;
    let nominal_units = (NOMINAL_LAYER_HEIGHT_MM * 10_000.0).round() as i64;
    let canonical_offset = (gap_units.div_euclid(nominal_units)
        + i64::from(gap_units % nominal_units != 0)
        + 1) as i32;
    assert_eq!(canonical_offset, 2);

    let overhang_layer = 7_i32;
    let virtual_gap_layer = overhang_layer - 1;
    let expected_highest_extruded_layer = overhang_layer - canonical_offset;
    let highest = plan
        .entries
        .iter()
        .map(|entry| entry.global_layer_index)
        .max()
        .expect("support entries must have a highest layer");
    assert_eq!(
        highest, expected_highest_extruded_layer,
        "top support must use ceil(top_gap / nominal_height) + 1, not accumulated layer Z"
    );
    assert!(
        plan.entries
            .iter()
            .all(|entry| entry.global_layer_index != virtual_gap_layer),
        "layer {virtual_gap_layer} is the propagated distance_to_top=-1 virtual node and must not be extruded"
    );
    assert!(
        plan.entries
            .iter()
            .any(|entry| entry.global_layer_index == expected_highest_extruded_layer),
        "the virtual node must propagate to an extruded descendant"
    );

    // Walking down by actual Z from the 2.0 mm overhang layer crosses the
    // 0.2 mm gap immediately because the preceding layer is at 1.4 mm.
    let accumulated_z_walk_layer = 6_i32;
    assert_ne!(
        accumulated_z_walk_layer, expected_highest_extruded_layer,
        "fixture must discriminate the deleted accumulated-Z walk"
    );
}
