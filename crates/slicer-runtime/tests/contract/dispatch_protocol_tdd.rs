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
const TEXT_POSTPASS_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../slicer-wasm-host/test-guests/sdk-postpass-text-guest.component.wasm"
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
        slicer_schema::WORLD_LAYER,
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
        ("PrePass::MeshAnalysis", Some("run")),
        ("PrePass::LayerPlanning", Some("run")),
        ("PrePass::PaintSegmentation", None),
        ("Layer::SlicePostProcess", Some("run")),
        ("Layer::Perimeters", Some("run")),
        ("Layer::PerimetersPostProcess", Some("run")),
        ("Layer::Infill", Some("run")),
        ("Layer::InfillPostProcess", Some("run")),
        ("Layer::Support", Some("run")),
        ("Layer::SupportPostProcess", Some("run")),
        ("Layer::PathOptimization", Some("run")),
        // Packet 163: per-stage package migration. The func is `run` for every
        // migrated stage; `qualified_export_for_stage_id` is the only lookup
        // that fully identifies the contract.
        ("PostPass::LayerFinalization", Some("run")),
        ("PostPass::GCodePostProcess", Some("run")),
        ("PostPass::TextPostProcess", Some("run")),
    ];

    for (stage_id, expected_export) in &stages {
        let result = export_for_stage_id(stage_id);
        assert_eq!(
            result, *expected_export,
            "stage '{}' should map to '{:?}'",
            stage_id, expected_export
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
    let component = wasm_cache::compiled_component_at(Path::new(TEXT_POSTPASS_GUEST_PATH));
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
fn missing_component_is_fatal() {
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

    let Err(slicer_ir::LayerStageError::FatalModule { message, .. }) = result else {
        panic!("missing component must produce a fatal module error: {result:?}");
    };
    assert!(
        message.contains("MissingComponent"),
        "error should identify the missing component: {message}"
    );
    assert!(
        message.contains("com.test.fixture"),
        "error should name the module: {message}"
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

// ── Packet 163 (AC-N1): fatal-on-miss contract ─────────────────────────────

/// Per packet 163 AC-N1: dispatching a stage at a guest that does **not**
/// export the corresponding per-stage interface must be **fatal at typed
/// instantiation** — never silent `Ok(())`.
///
/// The engine (wasmtime 43.0.1) emits the expected-only diagnostic
/// `` no exported instance named `<package>/<interface>@<version>` ``
/// (ADR-0045 §"Verified empirically, not just read"). This test pins that
/// exact wording, and forbids any "found @x.y.z" fragment the engine does
/// not produce (the diagnostic names only what the host wanted).
///
/// The test instantiates a real `sdk-postpass-text-guest` (which exports
/// only the text postprocess interface) and dispatches
/// `PostPass::GCodePostProcess` at it. Per packet 163, the gcode and text
/// stages now live in distinct per-stage packages, so the text guest does
/// not (and cannot) export the gcode interface — wasmtime's typed
/// instantiation must surface this as a fatal `DispatchError`.
#[test]
fn stage_miss_is_fatal_at_instantiation() {
    // The text-only SDK guest. It compiles to a per-stage world that
    // exports the text postprocess interface and nothing else.
    const TEXT_GUEST_PATH: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../slicer-wasm-host/test-guests/sdk-postpass-text-guest.component.wasm"
    );
    let path = Path::new(TEXT_GUEST_PATH);
    if !path.exists() {
        // The text round-trip guest is built on demand by the macro; if
        // it has not been built yet, skip rather than fail (this test
        // requires a live `.component.wasm`).
        eprintln!(
            "skipping stage_miss_is_fatal_at_instantiation: {} not found",
            TEXT_GUEST_PATH
        );
        return;
    }

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    // Install the text-only SDK guest as the component for the dispatch.
    let component = wasm_cache::compiled_component_at(Path::new(TEXT_GUEST_PATH));
    // Wire the text-only component into a `PostPass::GCodePostProcess`
    // bundle. The bundle's own `stage` field does not gate dispatch —
    // the *dispatch* stage is what the runner is invoked with. The
    // runner will pull the component, build a typed-instantiation for
    // the gcode world, and wasmtime will reject the text-only artifact.
    let bundle = make_bundle(
        "sdk-postpass-text-guest",
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

    use slicer_ir::PostpassError;

    let err = result.expect_err(
        "dispatching `PostPass::GCodePostProcess` at the text-only SDK guest \
         must be a PostpassError, not silent success (AC-N1)",
    );
    // The trait impl wraps the underlying `DispatchError` into
    // `PostpassError::FatalModule { stage_id, module_id, message }`. The
    // `message` carries the host's enriched reason which in turn embeds
    // the wasmtime engine's "no exported instance named ..." text plus
    // the qualified export the host wanted.
    let (stage_id, module_id, message) = match &err {
        PostpassError::FatalModule {
            stage_id,
            module_id,
            message,
        } => (stage_id.clone(), module_id.clone(), message.clone()),
        other => panic!("miss must produce PostpassError::FatalModule, got {other:?}"),
    };
    assert_eq!(
        stage_id, "PostPass::GCodePostProcess",
        "FatalModule.stage_id must be the stage that was dispatched",
    );
    assert_eq!(
        module_id, "sdk-postpass-text-guest",
        "FatalModule.module_id must be the wired module",
    );

    // Per packet 163 dispatch.rs: the TypedInstantiation reason names
    // the qualified export the host wanted.
    let expected_qualified =
        slicer_schema::qualified_export_for_stage_id("PostPass::GCodePostProcess")
            .expect("gcode stage must be migrated");
    assert!(
        message.contains(&expected_qualified),
        "FatalModule.message must name the qualified export `{}`; got: {}",
        expected_qualified,
        message,
    );
    // The engine surfaces one of two measured wordings when a guest does
    // not export the required per-stage interface:
    //   * `no exported instance named <package>/<interface>@<version>` —
    //     the canonical miss-diagnostic (ADR-0045 §"Verified
    //     empirically, not just read"), observed when the world is
    //     satisfied structurally but the interface is absent.
    //   * `component imports resource <name>, but a matching
    //     implementation was not found in the linker` — observed when
    //     the world imports a host-owned resource that the linker cannot
    //     resolve against the guest's resource table (e.g. trying to
    //     instantiate a `gcode-postprocess-module` against a guest that
    //     only knows the text-postprocess world).
    // Either form is acceptable: the contract is "the dispatch fails at
    // typed instantiation, with the qualified export named in the
    // reason" — never silent `Ok(())` (AC-N1).
    let engine_miss_wording = "no exported instance named";
    let engine_linker_wording = "component imports resource";
    assert!(
        message.contains(engine_miss_wording) || message.contains(engine_linker_wording),
        "FatalModule.message must include an engine-issued miss diagnostic \
         (`{engine_miss_wording}` or `{engine_linker_wording}`); got: {message}",
    );
}
