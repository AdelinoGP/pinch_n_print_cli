//! Red-first contract tests for the anchored-events stage.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{
    AnchoredGeometryContract, ConfigValue, ConfigView, LayerStageCommit, MeshIR,
    OrderedEventCollection,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageError,
    LayerStageRunner, LoadedModuleBuilder, WasmEngine, WasmRuntimeDispatcher,
};

const ANCHORED_GUEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/anchored-events-roundtrip-guest.component.wasm"
);

fn semver(major: u32, minor: u32, patch: u32) -> slicer_ir::SemVer {
    slicer_ir::SemVer {
        major,
        minor,
        patch,
    }
}

fn load_guest(engine: &WasmEngine) -> Arc<slicer_runtime::WasmComponent> {
    let path = PathBuf::from(ANCHORED_GUEST);
    assert!(
        path.exists(),
        "anchored-events-roundtrip-guest missing at {}; run build-test-guests.sh first",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read anchored-events-roundtrip-guest");
    Arc::new(
        engine
            .compile_component(&bytes)
            .expect("compile anchored-events-roundtrip-guest"),
    )
}

fn dispatch(
    config: ConfigView,
) -> (
    Result<Option<LayerStageCommit>, LayerStageError>,
    LayerArena,
    Vec<u8>,
) {
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine);
    let loaded = LoadedModuleBuilder::new(
        "com.test.anchored-events-roundtrip",
        semver(1, 0, 0),
        "Layer::AnchoredEvents",
        slicer_schema::TIER_LAYER,
        PathBuf::from("/dev/null"),
    )
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
        .expect("build instance pool"),
    );
    let module = CompiledModuleBuilder::new(loaded.id())
        .config_view(Arc::new(config))
        .build();
    let live = CompiledModuleLive::new(
        module.module_id(),
        pool,
        Some(component),
        module.claims(),
        Arc::clone(module.config_view()),
    );
    let blackboard = Blackboard::new(Arc::new(MeshIR::default()), 0);
    let layer = slicer_ir::GlobalLayer {
        index: 7,
        z: 0.3,
        ..Default::default()
    };
    let arena = LayerArena::new();
    let before = format!("{arena:?}").into_bytes();
    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::AnchoredEvents".to_string(),
        &layer,
        &live,
        crate::common::layer_input(&blackboard, &arena),
    );
    (result, arena, before)
}

fn config(values: &[(&str, i64)]) -> ConfigView {
    let mut map = HashMap::new();
    for (key, value) in values {
        map.insert((*key).to_string(), ConfigValue::Int(*value));
    }
    ConfigView::from_map(map)
}

fn normal() -> (
    Result<Option<LayerStageCommit>, LayerStageError>,
    LayerArena,
    Vec<u8>,
) {
    dispatch(config(&[("anchored_event_count", 2)]))
}

fn anchored_commit(
    result: Result<Option<LayerStageCommit>, LayerStageError>,
) -> Vec<OrderedEventCollection> {
    match result.expect("anchored-events dispatch must succeed") {
        Some(LayerStageCommit::AnchoredEvents(value)) => value,
        other => panic!("expected anchored-events commit, got {other:?}"),
    }
}

#[test]
fn anchored_event_collection_round_trips_with_exact_canonical_z() {
    let collections = anchored_commit(normal().0);
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].anchor_global_layer_index, 7);
    assert_eq!(collections[0].events.len(), 2);
    assert_eq!(
        collections[0].events[0].geometry,
        AnchoredGeometryContract::Planar { z: 3000 }
    );
    assert_eq!(
        collections[0].events[1].geometry,
        AnchoredGeometryContract::ZSpanning {
            min_z: 3000,
            max_z: 5000,
        }
    );
}

#[test]
fn anchored_runtime_hooks_survive_the_boundary_unaltered() {
    let collections = anchored_commit(normal().0);
    assert_eq!(
        collections[0].runtime_hooks,
        slicer_ir::AnchoredEventRuntimeHooks {
            optimize_paths: false,
            account_cooling: true,
            account_time: false,
        }
    );
}

#[test]
fn anchored_provenance_and_capability_order_preserved() {
    let collections = anchored_commit(normal().0);
    let event = &collections[0].events[0];
    assert_eq!(event.provenance.requesting_feature, "same-z-support");
    assert_eq!(event.provenance.source_plan_entry, "plan-entry-4");
    assert_eq!(event.input_capabilities, vec!["support.plan"]);
    assert_eq!(
        event.output_capabilities,
        vec!["extrusion.paths", "cooling.account"]
    );
}

#[test]
fn malformed_anchored_geometry_is_rejected_as_fatal() {
    let result = dispatch(config(&[("emit_malformed_geometry", 1)])).0;
    match result {
        Err(LayerStageError::FatalModule { .. }) => {
            let message = format!("{result:?}");
            assert!(message.contains("anchored entity planar z mismatch"));
        }
        other => panic!("expected fatal malformed planar geometry error, got {other:?}"),
    }
}

#[test]
fn guest_emitting_no_anchored_events_produces_no_commit() {
    let (result, arena, before) = dispatch(config(&[("anchored_event_count", 0)]));
    assert!(matches!(result, Ok(None)));
    let after = format!("{arena:?}").into_bytes();
    assert_eq!(before, after);
}

#[test]
fn duplicate_anchored_proposal_is_rejected_and_commits_nothing() {
    let result = dispatch(config(&[("duplicate_proposal", 1)])).0;
    assert!(result.is_err(), "duplicate proposal must surface an error");
    assert!(format!("{result:?}").contains("anchored-events dispatch"));
}

#[test]
fn zspanning_anchored_geometry_out_of_range_is_rejected_as_fatal() {
    let result = dispatch(config(&[("emit_malformed_geometry", 2)])).0;
    match result {
        Err(LayerStageError::FatalModule { .. }) => {
            let message = format!("{result:?}");
            assert!(message.contains("anchored entity z-span violation"));
        }
        other => panic!("expected fatal malformed z-spanning geometry error, got {other:?}"),
    }
}
