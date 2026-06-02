#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, MeshIR, Point3,
    Point3WithWidth, PrintEntity, SemVer, ToolChange, ZHop,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModule, CompiledModuleBuilder, FinalizationStageRunner, LoadedModule,
    LoadedModuleBuilder, WasmEngine, WasmRuntimeDispatcher,
};
use witness::{SdkFinalizationLayerWitness, SdkFinalizationLayerWitness1};

use crate::common::{finalization_input, wasm_cache};

const FINALIZATION_GUEST_COMPONENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/sdk-finalization-guest.component.wasm"
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

fn load_guest(engine: &WasmEngine) -> Arc<slicer_runtime::WasmComponent> {
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

fn make_module(id: &str, component: Arc<slicer_runtime::WasmComponent>) -> CompiledModule {
    let loaded = make_loaded_module(id);
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
    let engine = wasm_cache::shared_engine();
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

    FinalizationStageRunner::run_stage(
        &dispatcher,
        &stage,
        &module.as_live(),
        finalization_input(&blackboard),
        &mut layers,
    )
    .expect("finalization deep-copy run must succeed");

    assert_eq!(layers[0].ordered_entities.len(), layer0_original_len + 1);
    assert_eq!(layers[1].ordered_entities.len(), layer1_original_len + 1);

    let witness0 = witness_entity(&layers[0]);
    let w0 = SdkFinalizationLayerWitness::decode(&witness0.path.points);
    let w01 = SdkFinalizationLayerWitness1::decode(&witness0.path.points);
    assert_eq!(w0.layer_index, 0.0);
    assert!((w0.layer_z - 0.2).abs() < 1e-6);
    assert_eq!(w0.entity_count, 1.0);
    assert_eq!(w0.tool_changes_len, 1.0);
    assert_eq!(w0.z_hops_len, 1.0);
    assert_eq!(w01.first_entity_topo, 4.0);
    assert_eq!(w01.first_entity_point_count, 2.0);
    assert!((w01.first_entity_speed_factor - 1.25).abs() < 1e-6);
    assert_eq!(w01.first_zhop_after_entity, 3.0);
    assert!((w01.first_zhop_height - 0.45).abs() < 1e-6);

    let witness1 = witness_entity(&layers[1]);
    let w1 = SdkFinalizationLayerWitness::decode(&witness1.path.points);
    let w11 = SdkFinalizationLayerWitness1::decode(&witness1.path.points);
    assert_eq!(w1.layer_index, 1.0);
    assert!((w1.layer_z - 0.4).abs() < 1e-6);
    assert_eq!(w1.entity_count, 0.0);
    assert_eq!(w1.tool_changes_len, 0.0);
    assert_eq!(w1.z_hops_len, 0.0);
    assert_eq!(w11.first_entity_topo, -1.0);
    assert_eq!(w11.first_entity_point_count, -1.0);
    assert_eq!(w11.first_entity_speed_factor, -1.0);
    assert_eq!(w11.first_zhop_after_entity, -1.0);
    assert_eq!(w11.first_zhop_height, -1.0);
}
