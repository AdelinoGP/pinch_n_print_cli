//! Red-first contract test for anchored output from `Layer::Support`.

#![allow(missing_docs)]

use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{AnchoredGeometryContract, ConfigView, LayerStageCommit, MeshIR};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, CompiledModuleLive, LayerArena, LayerStageError,
    LayerStageRunner, LoadedModuleBuilder, WasmRuntimeDispatcher,
};

const SUPPORT_GUEST: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/support-anchored-reach-guest.component.wasm"
);

fn semver(major: u32, minor: u32, patch: u32) -> slicer_ir::SemVer {
    slicer_ir::SemVer {
        major,
        minor,
        patch,
    }
}

fn dispatch() -> Result<Option<LayerStageCommit>, LayerStageError> {
    let engine = crate::common::wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let path = PathBuf::from(SUPPORT_GUEST);
    assert!(
        path.exists(),
        "support-anchored-reach-guest missing at {}; run build-test-guests first",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read support-anchored-reach-guest");
    let component = Arc::new(
        engine
            .compile_component(&bytes)
            .expect("compile support-anchored-reach-guest"),
    );
    let loaded = LoadedModuleBuilder::new(
        "com.test.support-anchored-reach",
        semver(1, 0, 0),
        "Layer::Support",
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
        .config_view(Arc::new(ConfigView::new()))
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
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Support".to_string(),
        &layer,
        &live,
        crate::common::layer_input(&blackboard, &arena),
    )
}

#[test]
fn support_stage_guest_reaches_anchored_drain_with_exact_canonical_z() {
    let result = dispatch();
    let collections = match result.expect("support dispatch must not fail") {
        Some(LayerStageCommit::AnchoredEvents(value)) => value,
        other => panic!("expected anchored-events commit, got {other:?}"),
    };
    assert_eq!(collections.len(), 1);
    assert_eq!(collections[0].anchor_global_layer_index, 7);
    assert_eq!(collections[0].events.len(), 1);
    assert_eq!(
        collections[0].events[0].geometry,
        AnchoredGeometryContract::Planar { z: 1_234_567 }
    );
}
