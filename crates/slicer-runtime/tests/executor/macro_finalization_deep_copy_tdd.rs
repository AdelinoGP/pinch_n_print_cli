//! TASK-109 narrow follow-on: world-finalization must deep-copy
//! `LayerCollectionView` inputs AND drain `FinalizationOutputBuilder`
//! pushes back through the component boundary. Exercises the macro-
//! authored `sdk-finalization-guest` end-to-end so any regression in
//! the host resource plumbing or the macro-emitted Guest body fails
//! here.
//!
//! Deep-copy IN witness: the guest's trait body emits one synthetic
//! `push_entity_to_layer` per observed layer encoding the observed
//! `(layer_index, z, entity_count, tool_changes.len())` as point
//! coordinates. If the host forwarded real per-layer metadata, those
//! numbers round-trip. If it passed empty shells, they'd all be zero.
//!
//! Drain-back witness: pushes emitted by the guest arrive back in
//! `FinalizationStageRunner::run_stage`'s `&mut Vec<LayerCollectionIR>`
//! as appended entities / synthetic layers. We assert both the push
//! counts and the semantic field values.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionRole, LayerCollectionIR, PrintEntity, SemVer, ToolChange,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModule, CompiledModuleBuilder, FinalizationStageRunner, LoadedModule,
    LoadedModuleBuilder, WasmEngine, WasmRuntimeDispatcher,
};
use witness::SdkFinalizationLayerWitness;

use crate::common::{finalization_input, wasm_cache};

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_mesh_ir() -> Arc<slicer_ir::MeshIR> {
    Arc::new(slicer_ir::MeshIR {
        schema_version: semver(1, 0, 0),
        objects: Vec::new(),
        build_volume: slicer_ir::BoundingBox3 {
            min: slicer_ir::Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: slicer_ir::Point3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        },
    })
}

fn guest_component_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test-guests")
        .join(format!("{name}.component.wasm"))
}

fn load_guest(engine: &WasmEngine, name: &str) -> Arc<slicer_runtime::WasmComponent> {
    let path = guest_component_path(name);
    assert!(
        path.exists(),
        "guest component {name} missing; run test-guests/build-test-guests.sh"
    );
    let bytes = std::fs::read(&path).expect("read .component.wasm");
    Arc::new(engine.compile_component(&bytes).expect("compile component"))
}

fn make_loaded(id: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        "PostPass::LayerFinalization",
        "slicer:world-finalization@1.0.0",
        std::path::PathBuf::from("/dev/null"),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .build()
}

fn make_module(
    id: &str,
    component: Arc<slicer_runtime::WasmComponent>,
    config: ConfigView,
) -> CompiledModule {
    let loaded = make_loaded(id);
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
        .config_view(Arc::new(config))
        .wasm_component(Some(component))
        .build()
}

/// Build a small layer fixture with known `entity_count` and tool changes.
fn layer_ir(
    global_layer_index: u32,
    z: f32,
    entity_count: usize,
    tool_changes: &[(u32, u32, u32)],
) -> LayerCollectionIR {
    let mut ordered_entities = Vec::with_capacity(entity_count);
    for i in 0..entity_count {
        ordered_entities.push(PrintEntity {
            entity_id: (i as u64) + 1,
            path: slicer_ir::ExtrusionPath3D {
                points: Vec::new(),
                role: ExtrusionRole::Custom(String::new()),
                speed_factor: 1.0,
            },
            role: ExtrusionRole::Custom(String::new()),
            region_key: slicer_ir::RegionKey {
                global_layer_index,
                object_id: "obj".into(),
                region_id: 0,
            },
            topo_order: i as u32,
        });
    }
    LayerCollectionIR {
        schema_version: semver(1, 0, 0),
        global_layer_index,
        z,
        ordered_entities,
        tool_changes: tool_changes
            .iter()
            .map(|(after, from, to)| ToolChange {
                after_entity_index: *after,
                from_tool: *from,
                to_tool: *to,
            })
            .collect(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: Vec::new(),
        travel_moves: Vec::new(),
    }
}

#[test]
fn finalization_deep_copy_in_and_drain_back_out_round_trip() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-finalization-guest");

    let module = make_module(
        "com.test.sdk-finalization-deep",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage = "PostPass::LayerFinalization".to_string();

    // Three layers with distinctive metadata. If deep-copy IN works,
    // the guest will receive all of these values and echo them back
    // through one synthetic entity per layer.
    let mut layers = vec![
        layer_ir(0, 0.2, 4, &[(1, 0, 1)]),
        layer_ir(1, 0.4, 7, &[]),
        layer_ir(2, 0.6, 0, &[(0, 1, 0), (3, 0, 2)]),
    ];
    let original_lengths: Vec<usize> = layers.iter().map(|l| l.ordered_entities.len()).collect();

    FinalizationStageRunner::run_stage(
        &dispatcher,
        &stage,
        &module.as_live(),
        finalization_input(&bb),
        &mut layers,
    )
    .expect("finalization must succeed");

    // Every layer must have gained exactly one synthetic entity with
    // the witness coordinates carrying the deep-copied metadata.
    let expected = [(0u32, 0.2f32, 4u32, 1u32), (1, 0.4, 7, 0), (2, 0.6, 0, 2)];
    for (i, (exp_idx, exp_z, exp_entity_count, exp_tc_len)) in expected.iter().enumerate() {
        let new_len = layers[i].ordered_entities.len();
        assert_eq!(
            new_len,
            original_lengths[i] + 1,
            "layer {i} must have gained exactly one witness entity via drain-back"
        );
        let witness = layers[i].ordered_entities.first().unwrap();
        assert_eq!(witness.region_key.object_id, "__task109_fin_witness__");
        assert_eq!(witness.region_key.region_id, 109);
        let fw = SdkFinalizationLayerWitness::decode(&witness.path.points);
        assert_eq!(
            fw.layer_index as u32, *exp_idx,
            "deep-copy IN: layer_index for layer {i}"
        );
        assert!(
            (fw.layer_z - *exp_z).abs() < 1e-6,
            "deep-copy IN: z for layer {i}"
        );
        assert_eq!(
            fw.entity_count as u32, *exp_entity_count,
            "deep-copy IN: entity_count for layer {i}"
        );
        assert_eq!(
            fw.tool_changes_len as u32, *exp_tc_len,
            "deep-copy IN: tool_changes.len() for layer {i}"
        );
    }
}

#[test]
fn finalization_drain_back_creates_synthetic_layer_when_config_requests() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-finalization-guest");

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert("synthetic_layer_z".into(), ConfigValue::Float(7.5));
    let module = make_module(
        "com.test.sdk-finalization-synth",
        component,
        ConfigView::from_map(fields),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage = "PostPass::LayerFinalization".to_string();

    let mut layers: Vec<LayerCollectionIR> = Vec::new();
    FinalizationStageRunner::run_stage(
        &dispatcher,
        &stage,
        &module.as_live(),
        finalization_input(&bb),
        &mut layers,
    )
    .expect("finalization must succeed");

    assert_eq!(
        layers.len(),
        1,
        "synthetic layer must be appended via drain-back"
    );
    assert_eq!(layers[0].global_layer_index, 0);
    assert!(
        (layers[0].z - 7.5).abs() < 1e-6,
        "synthetic layer Z must match config-driven value"
    );
    assert_eq!(
        layers[0].ordered_entities.len(),
        1,
        "synthetic layer must carry exactly one extrusion path"
    );
}

#[test]
fn finalization_deep_copy_round_trip_is_deterministic_across_repeated_runs() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-finalization-guest");
    let module = make_module(
        "com.test.sdk-finalization-det",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage = "PostPass::LayerFinalization".to_string();

    let run_once = || -> Vec<(u32, f32, u32, u32, String, u64)> {
        let mut layers = vec![layer_ir(0, 0.2, 2, &[]), layer_ir(1, 0.4, 5, &[(2, 0, 1)])];
        FinalizationStageRunner::run_stage(
            &dispatcher,
            &stage,
            &module.as_live(),
            finalization_input(&bb),
            &mut layers,
        )
        .unwrap();
        layers
            .iter()
            .flat_map(|l| {
                l.ordered_entities
                    .iter()
                    .filter(|e| e.region_key.region_id == 109)
                    .map(|e| {
                        let fw = SdkFinalizationLayerWitness::decode(&e.path.points);
                        (
                            fw.layer_index as u32,
                            fw.layer_z,
                            fw.entity_count as u32,
                            fw.tool_changes_len as u32,
                            e.region_key.object_id.clone(),
                            e.region_key.region_id,
                        )
                    })
            })
            .collect()
    };
    let a = run_once();
    let b = run_once();
    let c = run_once();
    assert_eq!(a, b, "deep-copy + drain-back must be byte-deterministic");
    assert_eq!(b, c);
    assert_eq!(a.len(), 2, "one witness entity per layer");
}
