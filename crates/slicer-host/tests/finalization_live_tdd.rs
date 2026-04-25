//! TDD: live host finalization dispatch merges entity pushes before model entities.
//!
//! Verifies that `WasmRuntimeDispatcher::run_stage` for
//! `PostPass::LayerFinalization` batch-prepends finalization entity pushes
//! from `push-entity-to-layer` before the original model entities in each
//! target layer — matching the legacy `SkirtBrim::process()` ordering where
//! skirt/brim entities precede model paths.
//!
//! Tests use the pre-built `sdk-finalization-guest.component.wasm` which
//! emits one witness entity per observed layer via `push_entity_to_layer`.
//! With the batch-prepend dispatch fix, the witness entity is the FIRST
//! entity in each layer, preceding any original model entities.

#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::{
    Blackboard, CompiledModule, FinalizationStageRunner, IrAccessMask, LoadedModule, WasmEngine,
    WasmRuntimeDispatcher,
};
use slicer_ir::{
    BoundingBox3, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR, MeshIR, ObjectMesh,
    Point3, Point3WithWidth, PrintEntity, RegionKey, SemVer, Transform3d,
};

const SDK_FINALIZATION_GUEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/sdk-finalization-guest.component.wasm"
);

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer { major, minor, patch }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: slicer_ir::IndexedTriangleSet { vertices: vec![], indices: vec![] },
            transform: Transform3d {
                matrix: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0],
            },
            config: slicer_ir::ObjectConfig { data: std::collections::HashMap::new() },
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

fn load_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    let path = PathBuf::from(SDK_FINALIZATION_GUEST);
    assert!(path.exists(), "sdk-finalization-guest missing at {}", path.display());
    let bytes = std::fs::read(&path).expect("read sdk-finalization-guest");
    Arc::new(engine.compile_component(&bytes).expect("compile sdk-finalization-guest"))
}

fn make_loaded_module(id: &str) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: semver(1, 0, 0),
        stage: "PostPass::LayerFinalization".to_string(),
        wit_world: "slicer:world-finalization@1.0.0".to_string(),
        ir_reads: vec![],
        ir_writes: vec![],
        claims: vec![],
        requires_claims: vec![],
        incompatible_with: vec![],
        requires_modules: vec![],
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: Default::default(),
        overridable_per_region: vec![],
        overridable_per_layer: vec![],
        layer_parallel_safe: false,
        wasm_path: PathBuf::from("/dev/null"),
        placeholder_wasm: false,
    }
}

fn make_module(id: &str, component: Arc<slicer_host::WasmComponent>) -> CompiledModule {
    let loaded = make_loaded_module(id);
    let pool = Arc::new(
        build_wasm_instance_pool(&loaded, 1, WasmArtifactMetadata { uses_shared_memory: false })
            .expect("build instance pool"),
    );
    CompiledModule {
        module_id: id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: vec![] },
        ir_write_mask: IrAccessMask { paths: vec![] },
        config_view: Arc::new(ConfigView::new()),
        wasm_component: Some(component),
    }
}

fn model_entity(layer_index: u32, z: f32) -> PrintEntity {
    PrintEntity {
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth { x: 10.0, y: 10.0, z, width: 0.4, flow_factor: 1.0 },
                Point3WithWidth { x: 20.0, y: 20.0, z, width: 0.4, flow_factor: 1.0 },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            global_layer_index: layer_index,
            object_id: "obj1".to_string(),
            region_id: 1,
        },
        topo_order: 0,
    }
}

fn make_layer(index: u32, z: f32, entities: Vec<PrintEntity>) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: index,
        z,
        ordered_entities: entities,
        tool_changes: vec![],
        z_hops: vec![],
        annotations: vec![],
        retracts: vec![],
        travel_moves: vec![],
    }
}

/// AC-4: The live host finalization dispatch batch-prepends entity pushes from
/// the WASM guest so that finalization entities (e.g. skirt/brim) appear
/// BEFORE the original model entities in each target layer.
///
/// Uses `sdk-finalization-guest.component.wasm` which emits one witness entity
/// per observed layer via `push_entity_to_layer`. After the dispatch, each
/// layer's first entity must be the witness (role = Custom("task-109...")),
/// not the original model entity (OuterWall).
#[test]
fn live_finalization_dispatch_merges_skirt_brim_entity_pushes() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine);
    let module = make_module("com.test.finalization-prepend-witness", component);
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let stage = "PostPass::LayerFinalization".to_string();

    // Layer 0 has one OuterWall model entity.
    let mut layers = vec![make_layer(0, 0.2, vec![model_entity(0, 0.2)])];

    assert_eq!(
        layers[0].ordered_entities.len(), 1,
        "layer 0 must have exactly 1 model entity before finalization"
    );
    assert_eq!(
        layers[0].ordered_entities[0].role,
        ExtrusionRole::OuterWall,
        "the pre-finalization entity must be OuterWall"
    );

    // Run finalization dispatch. The sdk-finalization-guest emits one
    // witness entity to layer 0 via push_entity_to_layer.
    dispatcher
        .run_stage(&stage, &module, &blackboard, &mut layers)
        .expect("finalization dispatch must succeed");

    // The batch-prepend fix places the witness entity FIRST.
    assert_eq!(
        layers[0].ordered_entities.len(), 2,
        "layer 0 must have 2 entities after finalization (witness + model)"
    );

    // Finalization entity (witness) must appear BEFORE the model entity.
    let finalization_entity = &layers[0].ordered_entities[0];
    let model_entity_after = &layers[0].ordered_entities[1];

    assert_ne!(
        finalization_entity.role,
        ExtrusionRole::OuterWall,
        "first entity must be the finalization witness, not the model OuterWall"
    );
    assert_eq!(
        model_entity_after.role,
        ExtrusionRole::OuterWall,
        "second entity must be the original OuterWall model entity"
    );
    assert_ne!(
        finalization_entity.region_key.object_id, "obj1",
        "finalization entity must not have the model object_id"
    );
}
