//! Real WASM-boundary coverage for the SDK host-service bridge arms.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::common::{wasm_cache, TestModuleBundle};
use slicer_ir::{
    ActiveRegion, BoundingBox3, ConfigValue, ConfigView, GlobalLayer, IndexedTriangleSet,
    LayerPlanIR, MeshIR, ObjectMesh, Point3, RegionKey, RegionMapIR, RegionPlan, SemVer,
    Transform3d,
};
use slicer_runtime::{
    build_wasm_instance_pool, execute_prepass_with_builtins, instance_pool::WasmArtifactMetadata,
    Blackboard, CompiledModule, CompiledModuleBuilder, CompiledStage, ExecutionPlan,
    LoadedModuleBuilder, WasmRuntimeDispatcher,
};

fn semver() -> SemVer {
    SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    }
}

fn prism_mesh() -> MeshIR {
    let vertices = vec![
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        },
        Point3 {
            x: 10.0,
            y: 10.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 10.0,
            z: 0.0,
        },
        Point3 {
            x: 0.0,
            y: 0.0,
            z: 10.0,
        },
        Point3 {
            x: 10.0,
            y: 0.0,
            z: 10.0,
        },
        Point3 {
            x: 10.0,
            y: 10.0,
            z: 10.0,
        },
        Point3 {
            x: 0.0,
            y: 10.0,
            z: 10.0,
        },
    ];
    MeshIR {
        objects: vec![ObjectMesh {
            id: "cube".into(),
            mesh: IndexedTriangleSet {
                vertices,
                indices: vec![
                    0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3,
                    7, 2, 7, 6, 3, 0, 4, 3, 4, 7,
                ],
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

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

fn blackboard(mesh: MeshIR) -> Blackboard {
    let ids: Vec<String> = mesh.objects.iter().map(|o| o.id.clone()).collect();
    let regions = || {
        ids.iter()
            // exhaustive: host bridge roundtrip pins every field explicitly
            .map(|id| ActiveRegion {
                object_id: id.clone(),
                region_id: 0,
                resolved_config: slicer_ir::ResolvedConfig::default(),
                effective_layer_height: 0.2,
                nonplanar_shell: None,
                is_catchup_layer: false,
                catchup_z_bottom: 0.0,
                tool_index: 0,
            })
            .collect::<Vec<_>>()
    };
    let global_layers = (0..5)
        // exhaustive: host bridge roundtrip pins every field explicitly
        .map(|i| GlobalLayer {
            index: i,
            z: (i + 1) as f32 * 0.2,
            active_regions: regions(),
            has_nonplanar: false,
            is_sync_layer: false,
        })
        .collect();
    let mut participation = HashMap::new();
    let mut entries = HashMap::new();
    for id in &ids {
        participation.insert(
            id.clone(),
            (0..5)
                .map(|i| slicer_ir::ObjectLayerRef {
                    local_layer_index: i,
                    global_layer_index: i,
                    effective_layer_height: 0.2,
                })
                .collect(),
        );
        for i in 0..5 {
            entries.insert(
                RegionKey {
                    global_layer_index: i,
                    object_id: id.clone(),
                    region_id: 0,
                    variant_chain: Vec::new(),
                },
                RegionPlan::default(),
            );
        }
    }
    let mut bb = Blackboard::new(Arc::new(mesh), 0);
    bb.commit_layer_plan(Arc::new(LayerPlanIR {
        global_layers,
        object_participation: participation,
        ..Default::default()
    }))
    .unwrap();
    bb.commit_region_map(Arc::new(RegionMapIR {
        entries,
        ..Default::default()
    }))
    .unwrap();
    bb
}

fn bundle(config: HashMap<String, ConfigValue>) -> TestModuleBundle {
    let component = wasm_cache::compiled_guest("sdk-host-bridge-guest");
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("slicer-wasm-host")
        .join("test-guests")
        .join("sdk-host-bridge-guest.component.wasm");
    let object = config
        .get("bridge_probe_object")
        .and_then(|v| match v {
            ConfigValue::String(s) => Some(s.as_str()),
            _ => None,
        })
        .unwrap_or("cube");
    let loaded = LoadedModuleBuilder::new(
        format!("com.test.sdk-host-bridge-{object}"),
        semver(),
        "PrePass::SupportGeometry",
        slicer_schema::TIER_PREPASS,
        path,
    )
    .ir_reads(Vec::<String>::new())
    .ir_writes(vec!["SupportPlanIR.entries".into()])
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
        .config_view(Arc::new(ConfigView::from_map(config)))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn plan(module: CompiledModule) -> ExecutionPlan {
    ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::SupportGeometry".into(),
            modules: vec![module],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
        ..Default::default()
    }
}

fn config(object: &str) -> HashMap<String, ConfigValue> {
    let mut c = HashMap::new();
    c.insert(
        "bridge_probe_object".into(),
        ConfigValue::String(object.into()),
    );
    c
}

#[test]
fn host_bridge_roundtrip() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let b = bundle(config("cube"));
    let (module, handles) = b.into_module_and_handles();
    let audits = execute_prepass_with_builtins(
        &plan(module),
        &mut blackboard(prism_mesh()),
        &dispatcher,
        &handles,
    )
    .expect("WASM bridge dispatch must succeed");
    let message = &audits[0].diagnostics[0].message;
    let hit = message
        .split("raycast=Some(")
        .nth(1)
        .unwrap()
        .split(')')
        .next()
        .unwrap()
        .parse::<f32>()
        .unwrap();
    assert!(
        (hit - 10.0).abs() < 1e-4,
        "expected top hit at z=10: {message}"
    );
    let offset_width = message
        .split("offset_width=")
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .parse::<f32>()
        .unwrap();
    assert!(
        (offset_width - 12.0).abs() < 0.05,
        "expected 12 mm offset square: {message}"
    );
    assert!(
        message.contains("clip_count=1"),
        "clip wrapper did not return square: {message}"
    );
    assert!(
        message.contains("simplify_points=4"),
        "simplify wrapper result missing: {message}"
    );
    let a = message
        .split("now_a=")
        .nth(1)
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .parse::<u64>()
        .unwrap();
    let b = message
        .split("now_b=")
        .nth(1)
        .unwrap()
        .parse::<u64>()
        .unwrap();
    assert!(b >= a, "timestamps must be monotonic: {a}, {b}");
}

#[test]
fn host_bridge_unknown_object_errs() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let b = bundle(config("missing-object"));
    let (module, handles) = b.into_module_and_handles();
    let error = execute_prepass_with_builtins(
        &plan(module),
        &mut blackboard(prism_mesh()),
        &dispatcher,
        &handles,
    )
    .expect_err("unknown object must fail loudly");
    let text = format!("{error:?}");
    assert!(
        text.contains("missing-object"),
        "error must name unknown object: {text}"
    );
}
