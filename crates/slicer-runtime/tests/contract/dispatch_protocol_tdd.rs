// dispatch_protocol_tdd.rs — Cross-runner protocol tests
// (export-name lookup, per-runner success/error/pool, MissingComponent contract)

use std::path::Path;
use std::sync::Arc;

use slicer_ir::{GCodeIR, GlobalLayer, LayerCollectionIR, MeshIR, PrintMetadata};
use slicer_runtime::{
    Blackboard, FinalizationStageRunner, LayerArena, LayerStageRunner, PostpassStageRunner,
    PrepassStageRunner,
};
use slicer_schema::export_for_stage_id;
use slicer_wasm_host::{DispatchPhase, WasmRuntimeDispatcher};

use crate::common::dispatch_fixture;
use crate::common::wasm_cache;
use crate::common::{finalization_input, layer_input, postpass_input, prepass_input};

// ── WAT Fixtures ──────────────────────────────────────────────────────────────

/// An empty component with no exports — for testing typed instantiation failures.
const WAT_EMPTY_COMPONENT: &str = r#"(component)"#;

// ── Helpers ───────────────────────────────────────────────────────────────────

const PREPASS_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/prepass-guest.component.wasm"
);
const FINALIZATION_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/finalization-guest.component.wasm"
);
const POSTPASS_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/postpass-guest.component.wasm"
);

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR::default())
}

fn make_bundle(
    id: &str,
    stage: &str,
    component: Option<Arc<slicer_runtime::WasmComponent>>,
) -> crate::common::TestModuleBundle {
    use slicer_ir::{ConfigView, SemVer};
    use slicer_runtime::manifest::LoadedModuleBuilder;
    use slicer_runtime::{build_wasm_instance_pool, CompiledModuleBuilder, WasmArtifactMetadata};
    use std::collections::HashMap;

    let loaded = LoadedModuleBuilder::new(
        id,
        SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        stage,
        "slicer:world-layer@1.0.0",
        std::path::PathBuf::from("/dev/null"),
    )
    .min_host_version(SemVer {
        major: 0,
        minor: 1,
        patch: 0,
    })
    .min_ir_schema(SemVer {
        major: 1,
        minor: 0,
        patch: 0,
    })
    .max_ir_schema(SemVer {
        major: 2,
        minor: 0,
        patch: 0,
    })
    .layer_parallel_safe(true)
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

    let module = CompiledModuleBuilder::new(id)
        .config_view(Arc::new(ConfigView::from_map(HashMap::new())))
        .build();

    crate::common::TestModuleBundle {
        module,
        pool,
        component,
    }
}

fn minimal_gcode_ir() -> GCodeIR {
    GCodeIR {
        metadata: PrintMetadata {
            slicer_version: "test".into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ── A. Export-name mapping tests ──────────────────────────────────────────────

#[test]
fn export_name_mapping_covers_all_documented_stages() {
    let stages = [
        ("PrePass::MeshAnalysis", "run-mesh-analysis"),
        ("PrePass::LayerPlanning", "run-layer-planning"),
        ("PrePass::PaintSegmentation", "run-paint-segmentation"),
        ("Layer::SlicePostProcess", "run-slice-postprocess"),
        ("Layer::Perimeters", "run-perimeters"),
        ("Layer::PerimetersPostProcess", "run-wall-postprocess"),
        ("Layer::Infill", "run-infill"),
        ("Layer::InfillPostProcess", "run-infill-postprocess"),
        ("Layer::Support", "run-support"),
        ("Layer::SupportPostProcess", "run-support-postprocess"),
        ("Layer::PathOptimization", "run-path-optimization"),
        ("PostPass::LayerFinalization", "run-finalization"),
        ("PostPass::GCodePostProcess", "run-gcode-postprocess"),
        ("PostPass::TextPostProcess", "run-text-postprocess"),
    ];

    for (stage_id, expected_export) in &stages {
        let result = export_for_stage_id(stage_id);
        assert_eq!(
            result,
            Some(*expected_export),
            "stage '{}' should map to '{}'",
            stage_id,
            expected_export
        );
    }
}

#[test]
fn unknown_stage_returns_none() {
    assert_eq!(export_for_stage_id("Layer::Nonexistent"), None);
    assert_eq!(export_for_stage_id(""), None);
}

// ── B. Success-path per-runner tests ──────────────────────────────────────────

#[test]
fn prepass_runner_invokes_wasm_export() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = wasm_cache::compiled_component_at(Path::new(PREPASS_GUEST_PATH));
    let bundle = make_bundle("com.test.mesh", "PrePass::MeshAnalysis", Some(component));

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshAnalysis".to_string(),
        &bundle.as_live(),
        prepass_input(&blackboard),
    );

    assert!(
        result.is_ok(),
        "prepass dispatch should succeed: {:?}",
        result.err()
    );
}

#[test]
fn layer_runner_invokes_typed_wasm_export() {
    let mut fx = dispatch_fixture::for_stage("Layer::Infill").build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    fx.run_layer(&layer)
        .expect("Layer::Infill dispatch+commit should succeed");
}

#[test]
fn finalization_runner_invokes_wasm_export() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = wasm_cache::compiled_component_at(Path::new(FINALIZATION_GUEST_PATH));
    let bundle = make_bundle(
        "com.test.wipe",
        "PostPass::LayerFinalization",
        Some(component),
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut layers: Vec<LayerCollectionIR> = Vec::new();

    let result = FinalizationStageRunner::run_stage(
        &dispatcher,
        &"PostPass::LayerFinalization".to_string(),
        &bundle.as_live(),
        finalization_input(&blackboard),
        &mut layers,
    );

    assert!(
        result.is_ok(),
        "finalization dispatch should succeed: {:?}",
        result.err()
    );
}

#[test]
fn postpass_gcode_runner_invokes_wasm_export() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = wasm_cache::compiled_component_at(Path::new(POSTPASS_GUEST_PATH));
    let bundle = make_bundle(
        "com.test.gpost",
        "PostPass::GCodePostProcess",
        Some(component),
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut gcode_ir = minimal_gcode_ir();

    let result = dispatcher.run_gcode_postprocess(
        &"PostPass::GCodePostProcess".to_string(),
        &bundle.as_live(),
        postpass_input(&blackboard),
        &mut gcode_ir.commands,
    );

    assert!(
        result.is_ok(),
        "gcode postpass dispatch should succeed: {:?}",
        result.err()
    );
}

#[test]
fn postpass_text_runner_invokes_wasm_export() {
    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = wasm_cache::compiled_component_at(Path::new(POSTPASS_GUEST_PATH));
    let bundle = make_bundle(
        "com.test.tpost",
        "PostPass::TextPostProcess",
        Some(component),
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let result = dispatcher.run_text_postprocess(
        &"PostPass::TextPostProcess".to_string(),
        &bundle.as_live(),
        postpass_input(&blackboard),
        "; some gcode".to_string(),
    );

    assert!(
        result.is_ok(),
        "text postpass dispatch should succeed: {:?}",
        result.err()
    );
}

// ── C. Error-path coverage ────────────────────────────────────────────────────

#[test]
fn typed_instantiation_failure_produces_structured_error() {
    let fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_wat(WAT_EMPTY_COMPONENT)
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let arena = LayerArena::new();

    let live = fx.bundle.as_live();
    let result = LayerStageRunner::run_stage(
        &fx.dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &live,
        layer_input(&fx.blackboard, &arena),
    );

    assert!(
        result.is_err(),
        "should fail when component doesn't implement layer world"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("com.test.fixture"),
        "error should name the module: {msg}"
    );
    assert!(
        msg.contains("TypedInstantiation") || msg.contains("Layer::Infill"),
        "error should reference typed instantiation or stage: {msg}"
    );
}

#[test]
fn missing_component_gracefully_skipped() {
    let mut fx = dispatch_fixture::for_stage("Layer::Infill")
        .no_wasm()
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let result = fx.run_layer(&layer);

    assert!(
        result.is_ok(),
        "missing component should be gracefully skipped, not fatal: {:?}",
        result.err()
    );
    assert!(
        fx.arena.take_infill().is_none(),
        "arena must be empty after skipping a module with no compiled component"
    );
}

// ── D. Pool correctness ───────────────────────────────────────────────────────

#[test]
fn pool_slot_released_after_successful_typed_call() {
    let fx = dispatch_fixture::for_stage("Layer::Infill").build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    for _i in 0..3 {
        let arena = LayerArena::new();
        let live = fx.bundle.as_live();
        let result = LayerStageRunner::run_stage(
            &fx.dispatcher,
            &"Layer::Infill".to_string(),
            &layer,
            &live,
            layer_input(&fx.blackboard, &arena),
        );
        result.expect("Layer::Infill dispatch should succeed");
    }
}

#[test]
fn pool_slot_released_after_failed_typed_call() {
    let fx = dispatch_fixture::for_stage("Layer::Infill")
        .with_wat(WAT_EMPTY_COMPONENT)
        .build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    for i in 0..3 {
        let arena = LayerArena::new();
        let live = fx.bundle.as_live();
        let result = LayerStageRunner::run_stage(
            &fx.dispatcher,
            &"Layer::Infill".to_string(),
            &layer,
            &live,
            layer_input(&fx.blackboard, &arena),
        );
        assert!(result.is_err(), "call #{} should fail", i);
    }
}

// ── E. Typed-path specific tests ──────────────────────────────────────────────

#[test]
fn typed_layer_dispatch_creates_fresh_context_per_call() {
    let mut fx = dispatch_fixture::for_stage("Layer::Infill").build();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    for _i in 0..3 {
        fx.run_layer(&layer)
            .expect("Layer::Infill dispatch+commit should succeed");
    }
}

// ── DispatchError Display ─────────────────────────────────────────────────────

#[test]
fn dispatch_error_display_includes_all_diagnostic_fields() {
    let err = slicer_runtime::DispatchError {
        module_id: "com.test.mod".to_string(),
        stage_id: "Layer::Infill".to_string(),
        export_name: "run-infill".to_string(),
        phase: DispatchPhase::TypedExportCall,
        reason: "function not found".to_string(),
    };
    let display = format!("{err}");
    assert!(
        display.contains("com.test.mod"),
        "should include module_id: {display}"
    );
    assert!(
        display.contains("Layer::Infill"),
        "should include stage_id: {display}"
    );
    assert!(
        display.contains("run-infill"),
        "should include export_name: {display}"
    );
    assert!(
        display.contains("function not found"),
        "should include reason: {display}"
    );
}
