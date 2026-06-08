//! TDD scaffold for packet 41 â€" AC-5, NEG-3, AC-7.
//!
//! AC-5: A guest calling `modify_entity(layer_index, 1, SetSpeedFactor(0.5))`
//!       round-trips through WIT and mutates the host-side IR `speed_factor`
//!       from 1.0 to 0.5.
//!
//! NEG-3: When the guest targets an unknown entity_id (99), the host surfaces
//!        a diagnostic containing both the literal strings "entity_id" and "99",
//!        and the layer's entities remain unmodified.
//!
//! AC-7: Code-shape assertion â€" the macro drain-back iterates `merge_ops` at
//!       least once in `crates/slicer-macros/src/lib.rs`.
//!
//! All three tests are EXPECTED TO FAIL until Steps 2/4/5 land the real
//! `EntityMutation` types, WIT plumbing, and drain-back wiring.

#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LayerCollectionIR,
    MeshIR, ObjectMesh, Point3, Point3WithWidth, PrintEntity, RegionKey, SemVer, Transform3d,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, FinalizationStageRunner, LoadedModule,
    LoadedModuleBuilder, WasmEngine, WasmRuntimeDispatcher,
};

use crate::common::{finalization_input, wasm_cache, TestModuleBundle};

const MUTATION_GUEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/finalization-mutation-roundtrip-guest.component.wasm"
);

// â"€â"€ helpers â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€

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
        objects: vec![ObjectMesh {
            id: "cube".to_string(),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![],
                indices: vec![],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            config: slicer_ir::ObjectConfig {
                data: std::collections::HashMap::new(),
            },
            modifier_volumes: vec![],
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    })
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

fn make_module(id: &str, component: Arc<slicer_runtime::WasmComponent>) -> TestModuleBundle {
    make_module_with_config(id, component, ConfigView::new())
}

fn make_module_with_config(
    id: &str,
    component: Arc<slicer_runtime::WasmComponent>,
    config: ConfigView,
) -> TestModuleBundle {
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
    let module = CompiledModuleBuilder::new(id)
        .config_view(Arc::new(config))
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn load_guest(engine: &WasmEngine) -> Arc<slicer_runtime::WasmComponent> {
    let path = PathBuf::from(MUTATION_GUEST);
    assert!(
        path.exists(),
        "finalization-mutation-roundtrip-guest missing at {}; run build-test-guests.sh first",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read finalization-mutation-roundtrip-guest");
    Arc::new(
        engine
            .compile_component(&bytes)
            .expect("compile finalization-mutation-roundtrip-guest"),
    )
}

/// One entity with entity_id = 1 and speed_factor = 1.0.
fn entity_with_id(entity_id: u64, layer_index: u32, z: f32, speed_factor: f32) -> PrintEntity {
    PrintEntity {
        entity_id,
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 10.0,
                    y: 10.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
                Point3WithWidth {
                    x: 20.0,
                    y: 20.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor,
        },
        role: ExtrusionRole::OuterWall,
        region_key: RegionKey {
            global_layer_index: layer_index,
            object_id: "obj1".to_string(),
            region_id: 1,
            variant_chain: Vec::new(),
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

// â"€â"€ tests â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€â"€

/// AC-5: guest calls `modify_entity(layer_index, entity_id=1, SetSpeedFactor(0.5))`;
/// after dispatch the host IR must have speed_factor == 0.5 on entity_id 1.
///
/// EXPECTED FAIL until Steps 2/4/5 deliver: EntityMutation WIT type, host
/// drain-back wiring, and SDK trait method `modify_entity`.
#[test]
fn modify_entity_round_trips_through_wit() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine);
    let bundle = make_module("com.test.finalization-mutation-roundtrip", component);
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let stage = "PostPass::LayerFinalization".to_string();

    // One layer, one entity with entity_id=1, speed_factor=1.0.
    let mut layers = vec![make_layer(0, 0.2, vec![entity_with_id(1, 0, 0.2, 1.0)])];

    dispatcher
        .run_stage(
            &stage,
            &CompiledModuleLive::new(
                bundle.module.module_id(),
                Arc::clone(&bundle.pool),
                bundle.component.clone(),
                bundle.module.claims(),
                Arc::clone(bundle.module.config_view()),
            ),
            finalization_input(&blackboard),
            &mut layers,
        )
        .expect("finalization dispatch must succeed");

    // AC-5: The guest mutated entity_id=1 to speed_factor=0.5.
    let entity = layers[0]
        .ordered_entities
        .iter()
        .find(|e| e.entity_id == 1)
        .expect("entity_id=1 must still be present after mutation");

    assert_eq!(
        entity.path.speed_factor, 0.5,
        "AC-5: modify_entity(SetSpeedFactor(0.5)) must update host IR speed_factor from 1.0 to 0.5"
    );
}

/// NEG-3: guest targets entity_id=99 (via config) which does not exist in the fixture;
/// the host must surface a structured error whose message contains both "entity_id"
/// and "99", and the layer entities must be unmodified.
#[test]
fn modify_entity_unknown_id_round_trips_error() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine);

    // Strategy (a): pass target_entity_id=99 via config so the guest targets an
    // entity_id that does not exist in the fixture (only entity_id=1 is present).
    let config = {
        let mut m = std::collections::HashMap::new();
        m.insert("target_entity_id".to_string(), ConfigValue::Int(99));
        ConfigView::from_map(m)
    };
    let bundle =
        make_module_with_config("com.test.finalization-mutation-unknown", component, config);
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let stage = "PostPass::LayerFinalization".to_string();

    // Fixture has entity_id=1 only; guest targets entity_id=99 â€" not found.
    let mut layers = vec![make_layer(0, 0.2, vec![entity_with_id(1, 0, 0.2, 1.0)])];

    // Host's apply_to() must surface a FatalModule error containing "entity_id" and "99".
    let run_result = dispatcher.run_stage(
        &stage,
        &CompiledModuleLive::new(
            bundle.module.module_id(),
            Arc::clone(&bundle.pool),
            bundle.component.clone(),
            bundle.module.claims(),
            Arc::clone(bundle.module.config_view()),
        ),
        finalization_input(&blackboard),
        &mut layers,
    );

    match run_result {
        Err(e) => {
            let msg = format!("{e:?}");
            assert!(
                msg.contains("entity_id"),
                "NEG-3: error must mention 'entity_id', got: {msg}"
            );
            assert!(
                msg.contains("99"),
                "NEG-3: error must mention '99', got: {msg}"
            );
        }
        Ok(_) => {
            panic!("NEG-3 FAIL: host silently accepted modify_entity for unknown entity_id=99; expected a FatalModule error");
        }
    }
}

/// AC-7: Code-shape assertion.
///
/// The macro drain-back in `crates/slicer-macros/src/lib.rs` MUST contain an
/// iteration site over `merge_ops`. This test reads the file from disk and
/// applies a regex-like string pattern check.
///
/// Also asserts the file does NOT contain the legacy no-op markers
/// (`silently no-op`, `DEV-041`, or `merge_ops` appearing only in a TODO
/// comment without an actual iteration).
#[test]
fn drain_back_forwards_merge_ops() {
    // Navigate from CARGO_MANIFEST_DIR (crates/slicer-runtime) up two levels to
    // the workspace root, then into crates/slicer-macros/src/lib.rs.
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let macros_lib = PathBuf::from(manifest_dir)
        .join("..") // workspace/crates
        .join("..") // workspace root
        .join("crates")
        .join("slicer-macros")
        .join("src")
        .join("lib.rs");

    let source = std::fs::read_to_string(&macros_lib)
        .unwrap_or_else(|e| panic!("AC-7: could not read {}: {e}", macros_lib.display()));

    // AC-7: require at least one iteration site over merge_ops.
    // Acceptable patterns (any one suffices):
    //   for <ident> in <expr involving merge_ops>
    //   merge_ops().iter()
    //   .merge_ops().into_iter()
    let has_merge_ops_iteration =
        source.contains("merge_ops().iter()") || source.contains(".merge_ops().into_iter()") || {
            // Simple linear scan for "for " ... "in " ... "merge_ops" on the same logical line.
            source.lines().any(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("for ")
                    && trimmed.contains(" in ")
                    && trimmed.contains("merge_ops")
            })
        };

    assert!(
        has_merge_ops_iteration,
        "AC-7 FAIL: crates/slicer-macros/src/lib.rs must contain an iteration \
         site over merge_ops (for <x> in <...merge_ops...>, merge_ops().iter(), \
         or .merge_ops().into_iter()), but none was found. \
         This means the macro drain-back has NOT been wired yet (Step 5 pending)."
    );

    // AC-7: file must NOT contain legacy no-op markers.
    assert!(
        !source.contains("silently no-op"),
        "AC-7 FAIL: slicer-macros/src/lib.rs must not contain 'silently no-op'"
    );
    assert!(
        !source.contains("DEV-041"),
        "AC-7 FAIL: slicer-macros/src/lib.rs must not contain 'DEV-041' (placeholder reference)"
    );
}
