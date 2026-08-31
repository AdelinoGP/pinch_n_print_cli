//! Live guest-dispatch tests for the tree-support-planner config surface.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectLayerRef, ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan,
    SemVer, SupportPlanIR, Transform3d,
};
use slicer_runtime::{
    bind_module_config_view, build_wasm_instance_pool, execute_prepass_with_builtins, Blackboard,
    CompiledModuleBuilder, CompiledStage, ExecutionPlan, LoadedModuleBuilder, WasmArtifactMetadata,
    WasmEngine, WasmRuntimeDispatcher,
};

use crate::common::{wasm_cache, TestModuleBundle};

fn semver() -> SemVer {
    SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

fn mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "plate".into(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        z: 1.8,
                        ..Default::default()
                    },
                    Point3 {
                        x: 20.0,
                        z: 1.8,
                        ..Default::default()
                    },
                    Point3 {
                        x: 20.0,
                        y: 20.0,
                        z: 1.8,
                    },
                    Point3 {
                        y: 20.0,
                        z: 1.8,
                        ..Default::default()
                    },
                ],
                indices: vec![1, 3, 2, 1, 4, 3],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
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

fn inputs() -> (LayerPlanIR, RegionMapIR) {
    let layers = (0..10u32)
        .map(|index| GlobalLayer {
            index,
            z: (index + 1) as f32 * 0.2,
            active_regions: vec![ActiveRegion {
                object_id: "plate".into(),
                region_id: 0,
                effective_layer_height: 0.2,
                ..Default::default()
            }],
            ..Default::default()
        })
        .collect();
    let mut participation = HashMap::new();
    participation.insert(
        "plate".into(),
        (0..10u32)
            .map(|index| ObjectLayerRef {
                local_layer_index: index,
                global_layer_index: index,
                effective_layer_height: 0.2,
            })
            .collect(),
    );
    let mut entries = HashMap::new();
    for index in 0..10u32 {
        entries.insert(
            RegionKey {
                global_layer_index: index,
                object_id: "plate".into(),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            RegionPlan::default(),
        );
    }
    (
        LayerPlanIR {
            global_layers: layers,
            object_participation: participation,
            ..Default::default()
        },
        RegionMapIR {
            entries,
            ..Default::default()
        },
    )
}

fn config(extra: &[(&str, ConfigValue)]) -> HashMap<String, ConfigValue> {
    let mut values = HashMap::from([
        ("enable_support".into(), ConfigValue::Bool(true)),
        ("tree_support_branch_angle".into(), ConfigValue::Float(45.0)),
        ("line_width".into(), ConfigValue::Float(0.4)),
    ]);
    values.extend(
        extra
            .iter()
            .map(|(key, value)| ((*key).into(), value.clone())),
    );
    values
}

fn bundle(engine: &Arc<WasmEngine>, values: HashMap<String, ConfigValue>) -> TestModuleBundle {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("modules/core-modules/tree-support-planner/tree-support-planner.wasm");
    let component = Arc::new(
        engine
            .compile_component(&std::fs::read(&path).unwrap())
            .unwrap(),
    );
    let loaded = LoadedModuleBuilder::new(
        "com.core.tree-support-planner",
        semver(),
        "PrePass::SupportGeometry",
        slicer_schema::TIER_PREPASS,
        path,
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
    .min_host_version(semver())
    .min_ir_schema(semver())
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
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
        .unwrap(),
    );
    let module = CompiledModuleBuilder::new(loaded.id().to_string())
        .config_view(Arc::new(ConfigView::from_map(values)))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn run(values: HashMap<String, ConfigValue>) -> SupportPlanIR {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let (layer_plan, region_map) = inputs();
    let mut blackboard = Blackboard::new(Arc::new(mesh()), 0);
    blackboard.commit_layer_plan(Arc::new(layer_plan)).unwrap();
    blackboard.commit_region_map(Arc::new(region_map)).unwrap();
    let (module, handles) = bundle(&engine, values).into_module_and_handles();
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::SupportGeometry".into(),
            modules: vec![module],
        }],
        ..Default::default()
    };
    execute_prepass_with_builtins(&plan, &mut blackboard, &dispatcher, &handles).unwrap();
    (**blackboard.support_plan().unwrap()).clone()
}

fn point_count(plan: &SupportPlanIR) -> usize {
    plan.entries
        .iter()
        .filter_map(|entry| entry.skeleton.as_ref())
        .map(|skeleton| skeleton.points.len())
        .sum()
}

#[test]
fn max_bridge_length_config_reaches_tree_planner() {
    let default_count = point_count(&run(config(&[])));
    let short_bridge_count = point_count(&run(config(&[(
        "max_bridge_length",
        ConfigValue::Float(2.0),
    )])));
    assert!(short_bridge_count > default_count,
        "max_bridge_length=2.0 must densify contact samples: default={default_count}, short={short_bridge_count}");
}

#[test]
fn support_branch_merge_distance_config_reaches_tree_planner() {
    let default_count = point_count(&run(config(&[(
        "support_branch_merge_distance_mm",
        ConfigValue::Float(0.8),
    )])));
    let merged_count = point_count(&run(config(&[(
        "support_branch_merge_distance_mm",
        ConfigValue::Float(10.0),
    )])));
    assert_eq!(merged_count, default_count,
        "merge-distance remains a declare-only key in this slice and must retain the current default behavior");
}

#[test]
fn support_max_branches_per_layer_config_reaches_tree_planner() {
    let default_count = point_count(&run(config(&[(
        "support_max_branches_per_layer",
        ConfigValue::Int(1024),
    )])));
    let capped_count = point_count(&run(config(&[(
        "support_max_branches_per_layer",
        ConfigValue::Int(1),
    )])));
    assert!(capped_count < default_count,
        "branch cap must affect the dispatched plan: default={default_count}, capped={capped_count}");
}

/// Packet 239c AC-6: `independent_support_layer_height` is declared in BOTH
/// shipped `*-support-planner` manifests as `type = "bool"`, `default = true`,
/// and a global config carrying the key binds through `bind_module_config_view`
/// so each planner's `ConfigView` sees `Some(true)`.
#[test]
fn independent_support_layer_height_is_declared_and_bound_on_both_planners() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let source = HashMap::from([(
        "independent_support_layer_height".to_string(),
        ConfigValue::Bool(true),
    )]);
    for stem in ["tree-support-planner", "traditional-support-planner"] {
        let dir = repo_root.join("modules/core-modules").join(stem);
        let module = slicer_scheduler::manifest::load_module_from_paths(
            &dir.join(format!("{stem}.toml")),
            &dir.join(format!("{stem}.wasm")),
        )
        .unwrap_or_else(|error| panic!("load {stem} manifest: {error:?}"));
        let entry = module
            .config_schema()
            .entries
            .get("independent_support_layer_height")
            .unwrap_or_else(|| {
                panic!("{stem} must declare [config.schema.independent_support_layer_height]")
            });
        assert_eq!(entry.field_type, "bool", "{stem} field_type");
        assert_eq!(entry.default.as_deref(), Some("true"), "{stem} default");
        let view = bind_module_config_view(&module, &source);
        assert_eq!(
            view.get_bool("independent_support_layer_height"),
            Some(true),
            "{stem} bound ConfigView must expose independent_support_layer_height"
        );
    }
}
