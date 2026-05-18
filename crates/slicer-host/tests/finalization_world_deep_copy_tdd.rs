#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::{
    Blackboard, CompiledModule, CompiledModuleBuilder, FinalizationStageRunner, LoadedModule,
    LoadedModuleBuilder, WasmEngine, WasmRuntimeDispatcher,
};
use slicer_ir::{
    BoundingBox3, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, MeshIR, Point3,
    Point3WithWidth, PrintEntity, SemVer, ToolChange, ZHop,
};

const FINALIZATION_GUEST_COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/sdk-finalization-guest.component.wasm"
);

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects: Vec::new(),
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
    })
}

fn load_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    let path = PathBuf::from(FINALIZATION_GUEST_COMPONENT);
    assert!(
        path.exists(),
        "finalization guest component missing at {}",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read finalization guest component");
    Arc::new(
        engine
            .compile_component(&bytes)
            .expect("compile finalization guest component"),
    )
}

fn make_loaded_module(id: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        "PostPass::LayerFinalization",
        "slicer:world-finalization@1.0.0",
        PathBuf::from("/dev/null"),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build()
}

fn make_module(id: &str, component: Arc<slicer_host::WasmComponent>) -> CompiledModule {
    let loaded = make_loaded_module(id);
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .expect("build instance pool"),
    );
    CompiledModuleBuilder::new(id, pool)
        .wasm_component(Some(component))
        .build()
}

fn witness_entity(layer: &LayerCollectionIR) -> &PrintEntity {
    // Finalization entity pushes are batch-prepended so the witness appears first.
    layer
        .ordered_entities
        .first()
        .expect("witness entity prepended")
}

fn make_entity(
    layer_index: u32,
    topo_order: u32,
    point_count: usize,
    speed_factor: f32,
) -> PrintEntity {
    PrintEntity {
        entity_id: (topo_order as u64) + 1,
        path: ExtrusionPath3D {
            points: (0..point_count)
                .map(|index| Point3WithWidth {
                    x: index as f32,
                    y: (index * 2) as f32,
                    z: layer_index as f32,
                    width: 0.4 + index as f32,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                })
                .collect(),
            role: ExtrusionRole::OuterWall,
            speed_factor,
        },
        role: ExtrusionRole::OuterWall,
        region_key: slicer_ir::RegionKey {
            global_layer_index: layer_index,
            object_id: format!("obj-{layer_index}"),
            region_id: layer_index as u64 + 10,
        },
        topo_order,
    }
}

fn make_layer(
    global_layer_index: u32,
    z: f32,
    ordered_entities: Vec<PrintEntity>,
    tool_changes: Vec<ToolChange>,
    z_hops: Vec<ZHop>,
) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(1, 0, 0),
        global_layer_index,
        z,
        ordered_entities,
        tool_changes,
        z_hops,
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    }
}

#[test]
fn finalization_world_deep_copy_preserves_entities_and_z_hops() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine);
    let module = make_module("com.test.finalization-world-deep-copy", component);
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let stage = "PostPass::LayerFinalization".to_string();

    let layer0_original_len = 1usize;
    let layer1_original_len = 0usize;
    let mut layers = vec![
        make_layer(
            0,
            0.2,
            vec![make_entity(0, 4, 2, 1.25)],
            vec![ToolChange {
                after_entity_index: 0,
                from_tool: 0,
                to_tool: 1,
            }],
            vec![ZHop {
                after_entity_index: 3,
                hop_height: 0.45,
            }],
        ),
        make_layer(1, 0.4, Vec::new(), Vec::new(), Vec::new()),
    ];

    FinalizationStageRunner::run_stage(&dispatcher, &stage, &module, &blackboard, &mut layers)
        .expect("finalization deep-copy run must succeed");

    assert_eq!(layers[0].ordered_entities.len(), layer0_original_len + 1);
    assert_eq!(layers[1].ordered_entities.len(), layer1_original_len + 1);

    let witness0 = witness_entity(&layers[0]);
    let point0 = &witness0.path.points[0];
    let point1 = &witness0.path.points[1];
    assert_eq!(point0.x, 0.0);
    assert!((point0.y - 0.2).abs() < 1e-6);
    assert_eq!(point0.z, 1.0);
    assert_eq!(point0.width, 1.0);
    assert_eq!(point0.flow_factor, 1.0);
    assert_eq!(point1.x, 4.0);
    assert_eq!(point1.y, 2.0);
    assert!((point1.z - 1.25).abs() < 1e-6);
    assert_eq!(point1.width, 3.0);
    assert!((point1.flow_factor - 0.45).abs() < 1e-6);

    let witness1 = witness_entity(&layers[1]);
    let empty_point0 = &witness1.path.points[0];
    let empty_point1 = &witness1.path.points[1];
    assert_eq!(empty_point0.x, 1.0);
    assert!((empty_point0.y - 0.4).abs() < 1e-6);
    assert_eq!(empty_point0.z, 0.0);
    assert_eq!(empty_point0.width, 0.0);
    assert_eq!(empty_point0.flow_factor, 0.0);
    assert_eq!(empty_point1.x, -1.0);
    assert_eq!(empty_point1.y, -1.0);
    assert_eq!(empty_point1.z, -1.0);
    assert_eq!(empty_point1.width, -1.0);
    assert_eq!(empty_point1.flow_factor, -1.0);
}
