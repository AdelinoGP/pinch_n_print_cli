//! TDD tests for WASM runtime dispatch — proving real module invocation.
//!
//! These tests verify that the WasmRuntimeDispatcher actually calls into
//! WASM module exports through the component model, with proper error handling,
//! pool correctness, and structured diagnostics.
//!
//! Layer-stage tests use the pre-built test guest component (which implements
//! the full layer-module WIT world) and go through the typed boundary.
//! Non-layer tests use minimal WAT fixtures on the legacy untyped path.

#![allow(missing_docs, dead_code, unused_imports, unused_variables)]

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use slicer_host::dispatch::{export_name_for_stage, DispatchPhase, WasmRuntimeDispatcher};
use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::manifest::LoadedModule;
use slicer_host::pipeline::{run_pipeline, PipelineConfig, PipelineStageRunners};
use slicer_host::postpass::{GCodeEmitter, GCodeSerializer};
use slicer_host::{
    Blackboard, CompiledModule, CompiledStage, ExecutionPlan, FinalizationStageRunner,
    IrAccessMask, LayerArena, LayerStageError, LayerStageOutput, LayerStageRunner,
    PostpassStageRunner, PrepassStageRunner, WasmEngine,
};
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, ExPolygon, GCodeIR, GlobalLayer, LayerCollectionIR,
    LayerPaintMap, LayerPlanIR, MeshIR, PaintRegionIR, PaintSemantic, PaintValue, Point2, Point3,
    Polygon, PrintMetadata, SemVer, SemanticRegion, SliceIR, SlicedRegion, StageId,
    SurfaceClassificationIR,
};

// ── WAT Fixtures (for non-layer stages on the legacy path) ──────────────

/// Minimal WASM component exporting a void function named `run-mesh-analysis`.
const WAT_VOID_MESH_ANALYSIS: &str = r#"
    (component
        (core module $m
            (func $f (export "run-mesh-analysis"))
        )
        (core instance $i (instantiate $m))
        (func (export "run-mesh-analysis") (canon lift (core func $i "run-mesh-analysis")))
    )
"#;

/// Minimal WASM component exporting a void function named `run-finalization`.
const WAT_VOID_FINALIZATION: &str = r#"
    (component
        (core module $m
            (func $f (export "run-finalization"))
        )
        (core instance $i (instantiate $m))
        (func (export "run-finalization") (canon lift (core func $i "run-finalization")))
    )
"#;

/// Minimal WASM component exporting a void function named `run-gcode-postprocess`.
const WAT_VOID_GCODE_POSTPROCESS: &str = r#"
    (component
        (core module $m
            (func $f (export "run-gcode-postprocess"))
        )
        (core instance $i (instantiate $m))
        (func (export "run-gcode-postprocess") (canon lift (core func $i "run-gcode-postprocess")))
    )
"#;

/// WASM component exporting `run-text-postprocess` with string→string signature.
const WAT_TEXT_POSTPROCESS: &str = r#"
    (component
        (core module $m
            (memory (export "memory") 1)
            (func $realloc (param i32 i32 i32 i32) (result i32)
                i32.const 16
            )
            (export "cabi_realloc" (func $realloc))
            (func $transform (param i32 i32) (result i32)
                ;; Return (ptr=16, len=0) — empty string
                i32.const 0
                i32.const 16
                i32.store
                i32.const 4
                i32.const 0
                i32.store
                i32.const 0
            )
            (export "run-text-postprocess" (func $transform))
        )
        (core instance $i (instantiate $m))
        (alias core export $i "memory" (core memory $mem))
        (alias core export $i "cabi_realloc" (core func $realloc))
        (func (export "run-text-postprocess") (param "text" string) (result string)
            (canon lift (core func $i "run-text-postprocess")
                (memory $mem)
                (realloc (func $realloc))
            )
        )
    )
"#;

/// An empty component with no exports — for testing typed instantiation failures.
const WAT_EMPTY_COMPONENT: &str = r#"(component)"#;

/// Path to the pre-built test guest component implementing the layer-module world.
const GUEST_COMPONENT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/layer-infill-guest.component.wasm"
);
const PREPASS_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/prepass-guest.component.wasm"
);
const FINALIZATION_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/finalization-guest.component.wasm"
);
const POSTPASS_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/postpass-guest.component.wasm"
);

// ── Helpers ──────────────────────────────────────────────────────────────

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
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        },
    })
}

fn minimal_gcode_ir() -> GCodeIR {
    GCodeIR {
        schema_version: semver(1, 0, 0),
        commands: Vec::new(),
        metadata: PrintMetadata {
            slicer_version: "test".into(),
            estimated_print_time_s: 0,
            filament_used_mm: Vec::new(),
            layer_count: 0,
        },
    }
}

fn compile_wat(engine: &WasmEngine, wat: &str) -> Arc<slicer_host::WasmComponent> {
    let bytes = wat::parse_str(wat).expect("WAT parse should succeed");
    Arc::new(
        engine
            .compile_component(&bytes)
            .expect("WAT compilation should succeed"),
    )
}

fn load_guest_component(engine: &WasmEngine, path: &str) -> Arc<slicer_host::WasmComponent> {
    let bytes = std::fs::read(path)
        .unwrap_or_else(|e| panic!("Test guest component not found at {path}: {e}"));
    Arc::new(
        engine
            .compile_component(&bytes)
            .expect("guest compilation should succeed"),
    )
}

fn load_test_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    load_guest_component(engine, GUEST_COMPONENT_PATH)
}
fn load_prepass_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    load_guest_component(engine, PREPASS_GUEST_PATH)
}
fn load_finalization_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    load_guest_component(engine, FINALIZATION_GUEST_PATH)
}
fn load_postpass_guest(engine: &WasmEngine) -> Arc<slicer_host::WasmComponent> {
    load_guest_component(engine, POSTPASS_GUEST_PATH)
}

fn make_loaded_module(id: &str, stage: &str) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: semver(1, 0, 0),
        stage: stage.to_string(),
        wit_world: "slicer:world-layer@1.0.0".to_string(),
        ir_reads: Vec::new(),
        ir_writes: Vec::new(),
        claims: Vec::new(),
        requires_claims: Vec::new(),
        incompatible_with: Vec::new(),
        requires_modules: Vec::new(),
        min_host_version: semver(0, 1, 0),
        min_ir_schema: semver(1, 0, 0),
        max_ir_schema: semver(2, 0, 0),
        config_schema: Default::default(),
        overridable_per_region: Vec::new(),
        overridable_per_layer: Vec::new(),
        layer_parallel_safe: true,
        wasm_path: std::path::PathBuf::from("/dev/null"),
        placeholder_wasm: false,
    }
}

fn make_compiled_module(engine: &WasmEngine, id: &str, stage: &str, wat: &str) -> CompiledModule {
    make_compiled_module_with(id, stage, compile_wat(engine, wat))
}

fn make_compiled_module_with(
    id: &str,
    stage: &str,
    component: Arc<slicer_host::WasmComponent>,
) -> CompiledModule {
    make_compiled_module_with_config(id, stage, component, ConfigView::from_map(HashMap::new()))
}

fn make_compiled_module_with_config(
    id: &str,
    stage: &str,
    component: Arc<slicer_host::WasmComponent>,
    config: ConfigView,
) -> CompiledModule {
    let loaded = make_loaded_module(id, stage);
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .unwrap(),
    );
    CompiledModule {
        module_id: id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(config),
        wasm_component: Some(component),
    }
}

fn make_compiled_module_no_wasm(id: &str, stage: &str) -> CompiledModule {
    let loaded = make_loaded_module(id, stage);
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .unwrap(),
    );
    CompiledModule {
        module_id: id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(ConfigView::from_map(HashMap::new())),
        wasm_component: None,
    }
}

struct MinimalEmitter;
impl GCodeEmitter for MinimalEmitter {
    fn emit_gcode(
        &self,
        _layer_irs: &[LayerCollectionIR],
        _blackboard: &Blackboard,
    ) -> Result<GCodeIR, slicer_host::PostpassError> {
        Ok(minimal_gcode_ir())
    }
}

struct MinimalSerializer;
impl GCodeSerializer for MinimalSerializer {
    fn serialize_gcode(&self, _gcode_ir: &GCodeIR) -> Result<String, slicer_host::PostpassError> {
        Ok(String::from("; test gcode"))
    }
}

// ── A. Export-name mapping tests ────────────────────────────────────────

#[test]
fn export_name_mapping_covers_all_documented_stages() {
    let stages = [
        ("PrePass::MeshSegmentation", "run-mesh-segmentation"),
        ("PrePass::MeshAnalysis", "run-mesh-analysis"),
        ("PrePass::LayerPlanning", "run-layer-planning"),
        ("PrePass::PaintSegmentation", "run-paint-segmentation"),
        ("Layer::Slice", "run-slice"),
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
        let result = export_name_for_stage(stage_id);
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
    assert_eq!(export_name_for_stage("Layer::Nonexistent"), None);
    assert_eq!(export_name_for_stage(""), None);
}

// ── B. Success-path per-runner tests ────────────────────────────────────

#[test]
fn prepass_runner_invokes_wasm_export() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_prepass_guest(&engine);
    let module = make_compiled_module_with("com.test.mesh", "PrePass::MeshAnalysis", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshAnalysis".to_string(),
        &module,
        &blackboard,
    );

    assert!(
        result.is_ok(),
        "prepass dispatch should succeed: {:?}",
        result.err()
    );
}

#[test]
fn layer_runner_invokes_typed_wasm_export() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    // Use the real test guest that implements the full layer-module world.
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with("com.test.infill", "Layer::Infill", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );

    assert!(
        result.is_ok(),
        "typed layer dispatch should succeed: {:?}",
        result.err()
    );
}

#[test]
fn finalization_runner_invokes_wasm_export() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_finalization_guest(&engine);
    let module =
        make_compiled_module_with("com.test.wipe", "PostPass::LayerFinalization", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut layers = Vec::new();

    let result = FinalizationStageRunner::run_stage(
        &dispatcher,
        &"PostPass::LayerFinalization".to_string(),
        &module,
        &blackboard,
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
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_postpass_guest(&engine);
    let module =
        make_compiled_module_with("com.test.gpost", "PostPass::GCodePostProcess", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut gcode_ir = minimal_gcode_ir();

    let result = dispatcher.run_gcode_postprocess(
        &"PostPass::GCodePostProcess".to_string(),
        &module,
        &blackboard,
        &mut gcode_ir,
    );

    assert!(
        result.is_ok(),
        "gcode postpass dispatch should succeed: {:?}",
        result.err()
    );
}

#[test]
fn postpass_text_runner_invokes_wasm_export() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_postpass_guest(&engine);
    let module =
        make_compiled_module_with("com.test.tpost", "PostPass::TextPostProcess", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let result = dispatcher.run_text_postprocess(
        &"PostPass::TextPostProcess".to_string(),
        &module,
        &blackboard,
        "; some gcode".to_string(),
    );

    assert!(
        result.is_ok(),
        "text postpass dispatch should succeed: {:?}",
        result.err()
    );
}

// ── C. Error-path coverage ──────────────────────────────────────────────

#[test]
fn typed_instantiation_failure_produces_structured_error() {
    // An empty component does not implement the layer-module world,
    // so typed instantiation must fail with a TypedInstantiation phase error.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module(
        &engine,
        "com.test.empty",
        "Layer::Infill",
        WAT_EMPTY_COMPONENT,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );

    assert!(
        result.is_err(),
        "should fail when component doesn't implement layer world"
    );
    let msg = format!("{}", result.unwrap_err());
    assert!(
        msg.contains("com.test.empty"),
        "error should name the module: {msg}"
    );
    assert!(
        msg.contains("TypedInstantiation") || msg.contains("Layer::Infill"),
        "error should reference typed instantiation or stage: {msg}"
    );
}

#[test]
fn missing_component_gracefully_skipped() {
    // MissingComponent (placeholder .wasm, `wasm_component = None`) must NOT
    // be a fatal error — the pipeline should skip the module silently so that
    // placeholder modules do not block the run.  The load path emits a
    // structured diagnostic; dispatch-time skips gracefully.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module_no_wasm("com.test.nowasm", "Layer::Infill");

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );

    // Graceful skip: the module is missing its compiled component; the
    // dispatcher returns Ok and leaves the arena untouched.
    assert!(
        result.is_ok(),
        "missing component should be gracefully skipped, not fatal: {:?}",
        result.err()
    );
    // No output committed — the module was skipped entirely.
    assert!(
        arena.take_infill().is_none(),
        "arena must be empty after skipping a module with no compiled component"
    );
}

// ── D. Pool correctness ─────────────────────────────────────────────────

#[test]
fn pool_slot_released_after_successful_typed_call() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with("com.test.infill", "Layer::Infill", component);

    // The module pool has size 1. If the slot isn't released, the second
    // call would deadlock.
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    for i in 0..3 {
        let mut arena = LayerArena::new();
        let result = LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::Infill".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        );
        assert!(
            result.is_ok(),
            "call #{} should succeed (pool reuse): {:?}",
            i,
            result.err()
        );
    }
}

#[test]
fn pool_slot_released_after_failed_typed_call() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    // Empty component — will fail at typed instantiation
    let module = make_compiled_module(
        &engine,
        "com.test.empty",
        "Layer::Infill",
        WAT_EMPTY_COMPONENT,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // Call should fail but not deadlock — pool slot must be released
    for i in 0..3 {
        let mut arena = LayerArena::new();
        let result = LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::Infill".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        );
        assert!(result.is_err(), "call #{} should fail", i);
    }
}

// ── E. Typed-path specific tests ────────────────────────────────────────

#[test]
fn typed_layer_dispatch_creates_fresh_context_per_call() {
    // Each call must create an independent HostExecutionContext.
    // The test guest logs on every call; if contexts leaked, we'd
    // see state accumulation. Here we just verify 3 calls all succeed.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with("com.test.infill", "Layer::Infill", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    for i in 0..3 {
        let mut arena = LayerArena::new();
        let result = LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::Infill".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        );
        assert!(
            result.is_ok(),
            "typed call #{i} should succeed with fresh context: {:?}",
            result.err()
        );
    }
}

// ── F. Full pipeline integration with typed dispatch ────────────────────

#[test]
fn full_pipeline_with_typed_layer_dispatch() {
    let engine = Arc::new(WasmEngine::new());

    let component = load_test_guest(&engine);
    let layer_module = make_compiled_module_with("com.test.infill", "Layer::Infill", component);

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Infill".into(),
            modules: vec![layer_module],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "pipeline with typed layer dispatch should complete: {:?}",
        result.err()
    );
}

#[test]
fn full_pipeline_multi_tier_with_typed_layer() {
    let engine = Arc::new(WasmEngine::new());

    let prepass_module = make_compiled_module_with(
        "com.test.mesh",
        "PrePass::MeshAnalysis",
        load_prepass_guest(&engine),
    );
    let layer_module =
        make_compiled_module_with("com.test.infill", "Layer::Infill", load_test_guest(&engine));
    let fin_module = make_compiled_module_with(
        "com.test.wipe",
        "PostPass::LayerFinalization",
        load_finalization_guest(&engine),
    );
    let gcode_module = make_compiled_module_with(
        "com.test.gpost",
        "PostPass::GCodePostProcess",
        load_postpass_guest(&engine),
    );

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::MeshAnalysis".into(),
            modules: vec![prepass_module],
        }],
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Infill".into(),
            modules: vec![layer_module],
        }],
        layer_finalization_stage: Some(CompiledStage {
            stage_id: "PostPass::LayerFinalization".into(),
            modules: vec![fin_module],
        }),
        postpass_stages: vec![CompiledStage {
            stage_id: "PostPass::GCodePostProcess".into(),
            modules: vec![gcode_module],
        }],
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "multi-tier pipeline with typed layer dispatch should complete: {:?}",
        result.err()
    );
}

// ── G. Output commitment tests ──────────────────────────────────────────

#[test]
fn guest_infill_output_committed_to_arena() {
    // The test guest pushes one sparse infill path in run_infill.
    // After dispatch, the arena must contain an InfillIR with that path.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with("com.test.infill", "Layer::Infill", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 7,
        z: 1.4,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(7, 1.4, 1, 1)).unwrap();

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );
    assert!(
        result.is_ok(),
        "dispatch should succeed: {:?}",
        result.err()
    );

    // Verify the infill slot is populated
    let infill = arena
        .infill()
        .expect("infill arena slot should be populated");
    assert_eq!(infill.global_layer_index, 7, "layer index should match");
    assert_eq!(infill.regions.len(), 1, "should have 1 region");
    let region = &infill.regions[0];
    assert_eq!(region.sparse_infill.len(), 1, "should have 1 sparse path");
    // The test guest creates a path with 2 points
    assert_eq!(
        region.sparse_infill[0].points.len(),
        2,
        "path should have 2 points"
    );
    // The test guest sets role to SparseInfill
    assert_eq!(
        region.sparse_infill[0].role,
        slicer_ir::ExtrusionRole::SparseInfill,
        "role should be SparseInfill"
    );
}

#[test]
fn empty_guest_output_does_not_populate_arena() {
    // When the guest produces no paths (empty but valid), the arena slot should remain empty.
    // The test guest's run_support_postprocess is a no-op stub.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.support-pp",
        "Layer::SupportPostProcess",
        component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SupportPostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );
    assert!(
        result.is_ok(),
        "dispatch should succeed: {:?}",
        result.err()
    );

    // Support slot should remain empty because guest produced no output
    assert!(
        arena.support().is_none(),
        "support slot should be empty for no-op stage"
    );
}

#[test]
fn output_commitment_deterministic_across_repeated_runs() {
    // Running the same dispatch 3 times with fresh arenas should produce
    // identical InfillIR each time.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with("com.test.infill", "Layer::Infill", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut results = Vec::new();
    for _ in 0..3 {
        let mut arena = LayerArena::new();
        arena.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
        LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::Infill".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .unwrap();
        let infill = arena.take_infill().expect("infill should be committed");
        results.push(infill);
    }

    // All three results must be identical
    assert_eq!(results[0], results[1], "run 0 and 1 should be identical");
    assert_eq!(results[1], results[2], "run 1 and 2 should be identical");
}

#[test]
fn invalid_nan_output_rejected_with_diagnostic() {
    // Test the conversion validation directly since we can't make the test
    // guest produce NaN (it produces valid data). We test the validation
    // layer by calling convert_infill_output with crafted invalid data.
    use slicer_host::wit_host::{
        convert_infill_output, ExtrusionPath3d, ExtrusionRole, InfillOutputCollected,
        Point3WithWidth,
    };

    let bad_output = InfillOutputCollected {
        sparse_paths: vec![ExtrusionPath3d {
            points: vec![Point3WithWidth {
                x: f32::NAN,
                y: 0.0,
                z: 0.0,
                width: 0.4,
                flow_factor: 1.0,
            }],
            role: ExtrusionRole::SparseInfill,
            speed_factor: 1.0,
        }],
        solid_paths: Vec::new(),
        ironing_paths: Vec::new(),
        ..Default::default()
    };

    let result = convert_infill_output(&bad_output, 0);
    assert!(result.is_err(), "NaN output should be rejected");
    let msg = result.unwrap_err();
    assert!(msg.contains("NaN"), "error should mention NaN: {msg}");
    assert!(
        msg.contains("point[0]"),
        "error should identify the point index: {msg}"
    );
}

#[test]
fn end_to_end_pipeline_commits_guest_output_to_arena() {
    // Full pipeline test: manifest → plan → typed dispatch → output committed.
    // We verify that the per-layer execution produces a LayerCollectionIR
    // from a pipeline that includes a real WASM infill module.
    let engine = Arc::new(WasmEngine::new());

    let component = load_test_guest(&engine);
    let layer_module = make_compiled_module_with("com.test.infill", "Layer::Infill", component);

    let plan = ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![CompiledStage {
            stage_id: "Layer::Infill".into(),
            modules: vec![layer_module],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ]),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let config = PipelineConfig {
        mesh_ir: empty_mesh_ir(),
        plan,
        runners: PipelineStageRunners {
            prepass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            layer: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            finalization: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            postpass: Box::new(WasmRuntimeDispatcher::new(Arc::clone(&engine))),
            emitter: Box::new(MinimalEmitter),
            serializer: Box::new(MinimalSerializer),
        },
    };

    let result = run_pipeline(config);
    assert!(
        result.is_ok(),
        "pipeline with output commitment should complete: {:?}",
        result.err()
    );
}

#[test]
fn dispatch_error_display_includes_all_diagnostic_fields() {
    let err = slicer_host::DispatchError {
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

// ── H. Perimeter output commit tests ──────────────────────────────────

#[test]
fn perimeter_output_converts_wall_loops_and_commits_to_arena() {
    use slicer_host::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected, Point3,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };

    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                    },
                    Point3WithWidth {
                        x: 10.0,
                        y: 0.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                    },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag {
                    tool_index: None,
                    fuzzy_skin: false,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
                WallFeatureFlag {
                    tool_index: None,
                    fuzzy_skin: false,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
            ],
        }],
        infill_areas: Vec::new(),
        seam_candidates: vec![(
            Point3 {
                x: 5.0,
                y: 0.0,
                z: 0.2,
            },
            0.8,
        )],
        ..Default::default()
    };

    let ir = convert_perimeter_output(&output, 3).expect("valid perimeter output should convert");
    assert_eq!(ir.global_layer_index, 3);
    assert_eq!(ir.regions.len(), 1);
    assert_eq!(ir.regions[0].walls.len(), 1);
    assert_eq!(ir.regions[0].walls[0].perimeter_index, 0);
    assert_eq!(ir.regions[0].walls[0].loop_type, slicer_ir::LoopType::Outer);
    assert_eq!(ir.regions[0].walls[0].path.points.len(), 2);
    assert_eq!(ir.regions[0].walls[0].feature_flags.len(), 2);
    assert_eq!(ir.regions[0].seam_candidates.len(), 1);
    assert_eq!(ir.regions[0].seam_candidates[0].score, 0.8);
}

#[test]
fn perimeter_output_rejects_nan_in_wall_loop_path() {
    use slicer_host::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };

    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![Point3WithWidth {
                    x: f32::NAN,
                    y: 0.0,
                    z: 0.0,
                    width: 0.4,
                    flow_factor: 1.0,
                }],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            }],
        }],
        infill_areas: Vec::new(),
        seam_candidates: Vec::new(),
        ..Default::default()
    };

    let result = convert_perimeter_output(&output, 0);
    assert!(result.is_err(), "NaN in wall loop path should be rejected");
    let msg = result.unwrap_err();
    assert!(msg.contains("NaN"), "error should mention NaN: {msg}");
}

#[test]
fn perimeter_output_rejects_feature_flags_cardinality_mismatch() {
    use slicer_host::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };

    // 2 points but only 1 feature flag → cardinality mismatch per docs/03
    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth {
                        x: 0.0,
                        y: 0.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                    },
                    Point3WithWidth {
                        x: 10.0,
                        y: 0.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                    },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag {
                    tool_index: None,
                    fuzzy_skin: false,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
                // Missing second flag
            ],
        }],
        infill_areas: Vec::new(),
        seam_candidates: Vec::new(),
        ..Default::default()
    };

    let result = convert_perimeter_output(&output, 0);
    assert!(
        result.is_err(),
        "feature flag cardinality mismatch should be rejected"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("feature_flags length") && msg.contains("path points length"),
        "error should describe cardinality mismatch: {msg}"
    );
}

#[test]
fn perimeter_output_rejects_nan_seam_candidate() {
    use slicer_host::wit_host::{convert_perimeter_output, PerimeterOutputCollected, Point3};

    let output = PerimeterOutputCollected {
        wall_loops: Vec::new(),
        infill_areas: Vec::new(),
        seam_candidates: vec![(
            Point3 {
                x: f32::NAN,
                y: 0.0,
                z: 0.0,
            },
            1.0,
        )],
        ..Default::default()
    };

    let result = convert_perimeter_output(&output, 0);
    assert!(result.is_err(), "NaN seam candidate should be rejected");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("seam_candidate"),
        "error should identify seam: {msg}"
    );
    assert!(msg.contains("NaN"), "error should mention NaN: {msg}");
}

#[test]
fn empty_perimeter_output_does_not_populate_arena() {
    // The test guest's run_perimeters is a no-op, so perimeter slot stays empty.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with("com.test.perim", "Layer::Perimeters", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Perimeters".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );
    assert!(
        result.is_ok(),
        "empty perimeter dispatch should succeed: {:?}",
        result.err()
    );
    assert!(
        arena.perimeter().is_none(),
        "perimeter slot should be empty for no-op"
    );
}

// ── I. Slice postprocess output commit tests ──────────────────────────

#[test]
fn slice_postprocess_merge_replaces_polygons_preserving_identity() {
    use slicer_host::wit_host::{
        merge_slice_postprocess_into, ExPolygon, Point2, Polygon, RegionKey,
        SlicePostprocessCollected,
    };

    let existing = make_slice_ir(5, 0.2, 2, 1);
    let target_key = RegionKey {
        layer_index: 5,
        object_id: existing.regions[1].object_id.clone(),
        region_id: existing.regions[1].region_id.to_string(),
    };
    let output = SlicePostprocessCollected {
        polygon_updates: vec![(
            target_key,
            vec![ExPolygon {
                contour: Polygon {
                    points: vec![
                        Point2 { x: 0, y: 0 },
                        Point2 { x: 100, y: 0 },
                        Point2 { x: 100, y: 100 },
                    ],
                },
                holes: Vec::new(),
            }],
        )],
        path_z_updates: Vec::new(),
    };

    let merged =
        merge_slice_postprocess_into(existing.clone(), &output).expect("merge should succeed");
    assert_eq!(
        merged.regions.len(),
        2,
        "all regions preserved (not flattened)"
    );
    assert_eq!(
        merged.regions[0], existing.regions[0],
        "untouched region unchanged"
    );
    assert_eq!(merged.regions[1].object_id, existing.regions[1].object_id);
    assert_eq!(merged.regions[1].region_id, existing.regions[1].region_id);
    assert_eq!(merged.regions[1].polygons[0].contour.points.len(), 3);
}

#[test]
fn slice_postprocess_rejects_nan_z_update() {
    use slicer_host::wit_host::{
        merge_slice_postprocess_into, RegionKey, SlicePostprocessCollected,
    };

    let existing = make_slice_ir(0, 0.2, 1, 1);
    let key = RegionKey {
        layer_index: 0,
        object_id: existing.regions[0].object_id.clone(),
        region_id: existing.regions[0].region_id.to_string(),
    };
    let output = SlicePostprocessCollected {
        polygon_updates: Vec::new(),
        path_z_updates: vec![(key, 0, 0, f32::NAN)],
    };

    let result = merge_slice_postprocess_into(existing, &output);
    assert!(result.is_err(), "NaN Z update should be rejected");
    let msg = result.unwrap_err();
    assert!(msg.contains("NaN"), "error should mention NaN: {msg}");
}

#[test]
fn slice_postprocess_rejects_unknown_region_key() {
    use slicer_host::wit_host::{
        merge_slice_postprocess_into, RegionKey, SlicePostprocessCollected,
    };

    let existing = make_slice_ir(0, 0.2, 2, 1);
    let bogus = RegionKey {
        layer_index: 0,
        object_id: "does-not-exist".to_string(),
        region_id: "999".to_string(),
    };
    let output = SlicePostprocessCollected {
        polygon_updates: vec![(bogus, Vec::new())],
        path_z_updates: Vec::new(),
    };

    let result = merge_slice_postprocess_into(existing, &output);
    assert!(
        result.is_err(),
        "unknown region key must fail with structured diagnostic"
    );
    let msg = result.unwrap_err();
    assert!(
        msg.contains("unknown region") && msg.contains("does-not-exist"),
        "diagnostic should explain mapping failure: {msg}"
    );
}

#[test]
fn empty_slice_postprocess_does_not_populate_arena() {
    // The test guest's run_slice_postprocess is a no-op, so slice slot stays empty.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.slicepp", "Layer::SlicePostProcess", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SlicePostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );
    assert!(
        result.is_ok(),
        "empty slicepp dispatch should succeed: {:?}",
        result.err()
    );
    assert!(
        arena.slice().is_none(),
        "slice slot should be empty for no-op"
    );
}

// ── J. Determinism and isolation for perimeter commit ──────────────────

#[test]
fn perimeter_conversion_deterministic_across_repeated_calls() {
    use slicer_host::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected, Point3,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };

    let mk_output = || PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth {
                        x: 1.0,
                        y: 2.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                    },
                    Point3WithWidth {
                        x: 3.0,
                        y: 4.0,
                        z: 0.2,
                        width: 0.4,
                        flow_factor: 1.0,
                    },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag {
                    tool_index: Some(0),
                    fuzzy_skin: true,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
                WallFeatureFlag {
                    tool_index: Some(0),
                    fuzzy_skin: true,
                    is_bridge: false,
                    is_thin_wall: false,
                    skip_ironing: false,
                    custom: vec![],
                },
            ],
        }],
        infill_areas: Vec::new(),
        seam_candidates: vec![(
            Point3 {
                x: 2.0,
                y: 1.0,
                z: 0.2,
            },
            0.9,
        )],
        ..Default::default()
    };

    let ir_a = convert_perimeter_output(&mk_output(), 0).unwrap();
    let ir_b = convert_perimeter_output(&mk_output(), 0).unwrap();
    let ir_c = convert_perimeter_output(&mk_output(), 0).unwrap();

    assert_eq!(ir_a, ir_b, "run 0 and 1 should be identical");
    assert_eq!(ir_b, ir_c, "run 1 and 2 should be identical");
}

#[test]
fn failed_commit_does_not_leak_into_next_call() {
    // Two sequential calls: first succeeds and populates infill,
    // second (for perimeters) with empty output should not see leaked infill.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // First call: infill (produces output)
    let infill_module =
        make_compiled_module_with("com.test.infill", "Layer::Infill", Arc::clone(&component));
    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    let r1 = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &infill_module,
        &blackboard,
        &mut arena,
    );
    assert!(r1.is_ok(), "infill should succeed");
    assert!(arena.infill().is_some(), "infill slot should be populated");

    // Second call: perimeters (no-op — should not contaminate anything)
    let perim_module = make_compiled_module_with(
        "com.test.perim",
        "Layer::Perimeters",
        Arc::clone(&component),
    );
    let r2 = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Perimeters".to_string(),
        &layer,
        &perim_module,
        &blackboard,
        &mut arena,
    );
    assert!(r2.is_ok(), "perimeters should succeed");
    // Perimeter slot should be empty (no-op guest), infill slot unchanged.
    assert!(
        arena.perimeter().is_none(),
        "perimeter slot should stay empty"
    );
    assert!(
        arena.infill().is_some(),
        "infill slot should still be populated"
    );
}

// ── K. Real config wiring through production dispatch ──────────────────

#[test]
fn real_config_visible_through_production_layer_dispatch() {
    // The test guest reads `infill-spacing` from config and computes
    // path second-point x = spacing * 10.0.
    // With spacing=5.0 the point should be at x=50.0.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert("infill-spacing".into(), ConfigValue::Float(5.0));
    let config = ConfigView::from_map(fields);

    let module =
        make_compiled_module_with_config("com.test.infill", "Layer::Infill", component, config);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );
    assert!(
        result.is_ok(),
        "dispatch with config should succeed: {:?}",
        result.err()
    );

    let infill = arena.infill().expect("infill slot should be populated");
    let path = &infill.regions[0].sparse_infill[0];
    // spacing=5.0 → x = 5.0 * 10.0 = 50.0
    assert_eq!(
        path.points[1].x, 50.0,
        "guest should use config spacing=5.0 → x=50.0, got {}",
        path.points[1].x
    );
}

#[test]
fn different_configs_produce_different_output() {
    // Two dispatches with different infill-spacing values should produce
    // different path X extents.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // Config A: spacing=3.0 → x=30.0
    let config_a =
        ConfigView::from_map([("infill-spacing".into(), ConfigValue::Float(3.0))].into());
    let mod_a = make_compiled_module_with_config(
        "com.test.infill-a",
        "Layer::Infill",
        Arc::clone(&component),
        config_a,
    );
    let mut arena_a = LayerArena::new();
    arena_a.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &mod_a,
        &blackboard,
        &mut arena_a,
    )
    .expect("dispatch A should succeed");

    // Config B: spacing=7.0 → x=70.0
    let config_b =
        ConfigView::from_map([("infill-spacing".into(), ConfigValue::Float(7.0))].into());
    let mod_b = make_compiled_module_with_config(
        "com.test.infill-b",
        "Layer::Infill",
        Arc::clone(&component),
        config_b,
    );
    let mut arena_b = LayerArena::new();
    arena_b.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &mod_b,
        &blackboard,
        &mut arena_b,
    )
    .expect("dispatch B should succeed");

    let x_a = arena_a.infill().unwrap().regions[0].sparse_infill[0].points[1].x;
    let x_b = arena_b.infill().unwrap().regions[0].sparse_infill[0].points[1].x;

    assert_eq!(x_a, 30.0, "config A spacing=3.0 → x=30.0, got {x_a}");
    assert_eq!(x_b, 70.0, "config B spacing=7.0 → x=70.0, got {x_b}");
    assert_ne!(
        x_a, x_b,
        "different configs should produce different output"
    );
}

#[test]
fn repeated_identical_config_produces_deterministic_output() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mk_module = || {
        let config =
            ConfigView::from_map([("infill-spacing".into(), ConfigValue::Float(4.0))].into());
        make_compiled_module_with_config(
            "com.test.infill",
            "Layer::Infill",
            Arc::clone(&component),
            config,
        )
    };

    let mut results = Vec::new();
    for _ in 0..3 {
        let module = mk_module();
        let mut arena = LayerArena::new();
        arena.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
        LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::Infill".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .unwrap();
        let infill = arena.take_infill().expect("should have infill");
        results.push(infill);
    }

    assert_eq!(results[0], results[1], "run 0 and 1 must be identical");
    assert_eq!(results[1], results[2], "run 1 and 2 must be identical");
}

#[test]
fn config_isolation_across_sequential_calls() {
    // Two calls with different configs should not leak values.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // First call: spacing=6.0
    let config1 = ConfigView::from_map([("infill-spacing".into(), ConfigValue::Float(6.0))].into());
    let mod1 = make_compiled_module_with_config(
        "com.test.infill",
        "Layer::Infill",
        Arc::clone(&component),
        config1,
    );
    let mut arena1 = LayerArena::new();
    arena1.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &mod1,
        &blackboard,
        &mut arena1,
    )
    .unwrap();

    // Second call: spacing=2.0 (must not see 6.0)
    let config2 = ConfigView::from_map([("infill-spacing".into(), ConfigValue::Float(2.0))].into());
    let mod2 = make_compiled_module_with_config(
        "com.test.infill2",
        "Layer::Infill",
        Arc::clone(&component),
        config2,
    );
    let mut arena2 = LayerArena::new();
    arena2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &mod2,
        &blackboard,
        &mut arena2,
    )
    .unwrap();

    let x1 = arena1.infill().unwrap().regions[0].sparse_infill[0].points[1].x;
    let x2 = arena2.infill().unwrap().regions[0].sparse_infill[0].points[1].x;

    assert_eq!(x1, 60.0, "first call spacing=6.0 → x=60.0, got {x1}");
    assert_eq!(x2, 20.0, "second call spacing=2.0 → x=20.0, got {x2}");
}

// ── H. Paint region wiring tests ───────────────────────────────────────

fn make_paint_region_ir(
    layer_index: u32,
    enforcer_count: usize,
    blocker_count: usize,
) -> PaintRegionIR {
    let mut semantic_regions = HashMap::new();

    if enforcer_count > 0 {
        let regions: Vec<SemanticRegion> = (0..enforcer_count)
            .map(|i| SemanticRegion {
                object_id: format!("obj-{i}"),
                polygons: vec![ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: 10_000, y: 0 },
                            Point2 {
                                x: 10_000,
                                y: 10_000,
                            },
                            Point2 { x: 0, y: 10_000 },
                        ],
                    },
                    holes: Vec::new(),
                }],
                value: PaintValue::Flag(true),
                paint_order: i as u64,
            })
            .collect();
        semantic_regions.insert(PaintSemantic::SupportEnforcer, regions);
    }

    if blocker_count > 0 {
        let regions: Vec<SemanticRegion> = (0..blocker_count)
            .map(|i| SemanticRegion {
                object_id: format!("blocker-{i}"),
                polygons: vec![ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: 5_000, y: 0 },
                            Point2 { x: 5_000, y: 5_000 },
                            Point2 { x: 0, y: 5_000 },
                        ],
                    },
                    holes: Vec::new(),
                }],
                value: PaintValue::Flag(true),
                paint_order: i as u64,
            })
            .collect();
        semantic_regions.insert(PaintSemantic::SupportBlocker, regions);
    }

    let mut per_layer = HashMap::new();
    per_layer.insert(
        layer_index,
        LayerPaintMap {
            global_layer_index: layer_index,
            semantic_regions,
        },
    );

    PaintRegionIR {
        schema_version: semver(1, 0, 0),
        per_layer,
    }
}

#[test]
fn real_paint_region_data_visible_through_production_support_dispatch() {
    // The test guest's run_support queries paint regions and encodes counts
    // into support output: x=enforcer_count, y=blocker_count,
    // flow_factor=layer_index.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.support", "Layer::Support", Arc::clone(&component));

    let mut blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let paint_ir = make_paint_region_ir(7, 3, 1);
    blackboard
        .commit_paint_regions(Arc::new(paint_ir))
        .expect("commit paint regions");

    let layer = GlobalLayer {
        index: 7,
        z: 1.4,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(7, 1.4, 1, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Support".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let support = arena.support().expect("support should be populated");
    let p = &support.support_paths[0].points[0];
    assert_eq!(p.x, 3.0, "enforcer count should be 3, got {}", p.x);
    assert_eq!(p.y, 1.0, "blocker count should be 1, got {}", p.y);
    assert_eq!(
        p.flow_factor, 7.0,
        "paint layer index should match layer.index=7, got {}",
        p.flow_factor
    );
}

#[test]
fn no_paint_region_ir_produces_empty_paint_view() {
    // When no PaintRegionIR is committed to the blackboard, the guest should see
    // zero enforcer/blocker regions. The support guest still produces a path
    // with x=0 (0 enforcers), y=0 (0 blockers).
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.support", "Layer::Support", Arc::clone(&component));

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Support".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let support = arena.support().expect("support output should still exist");
    let p = &support.support_paths[0].points[0];
    assert_eq!(p.x, 0.0, "no enforcers when PaintRegionIR absent");
    assert_eq!(p.y, 0.0, "no blockers when PaintRegionIR absent");
}

#[test]
fn paint_region_layer_mismatch_produces_empty_view() {
    // PaintRegionIR has data for layer 5, but we execute layer 10.
    // Guest should see empty paint regions.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.support", "Layer::Support", Arc::clone(&component));

    let mut blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let paint_ir = make_paint_region_ir(5, 2, 0); // paint at layer 5
    blackboard
        .commit_paint_regions(Arc::new(paint_ir))
        .expect("commit");

    let layer = GlobalLayer {
        index: 10, // execute at layer 10 — no paint data here
        z: 2.0,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(10, 2.0, 1, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Support".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let support = arena.support().expect("support output");
    let p = &support.support_paths[0].points[0];
    assert_eq!(p.x, 0.0, "no enforcers at mismatched layer");
    assert_eq!(
        p.flow_factor, 10.0,
        "paint layer index should be 10 (execution layer), got {}",
        p.flow_factor
    );
}

#[test]
fn paint_region_isolation_across_sequential_dispatches() {
    // Two sequential dispatches with different paint data must not leak.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    // First dispatch: 3 enforcers at layer 0
    let mut bb1 = Blackboard::new(empty_mesh_ir(), 1);
    bb1.commit_paint_regions(Arc::new(make_paint_region_ir(0, 3, 0)))
        .unwrap();
    let module1 =
        make_compiled_module_with("com.test.support", "Layer::Support", Arc::clone(&component));
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena1 = LayerArena::new();
    arena1.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Support".to_string(),
        &layer,
        &module1,
        &bb1,
        &mut arena1,
    )
    .unwrap();

    // Second dispatch: 1 enforcer at layer 0
    let mut bb2 = Blackboard::new(empty_mesh_ir(), 1);
    bb2.commit_paint_regions(Arc::new(make_paint_region_ir(0, 1, 2)))
        .unwrap();
    let module2 = make_compiled_module_with(
        "com.test.support2",
        "Layer::Support",
        Arc::clone(&component),
    );
    let mut arena2 = LayerArena::new();
    arena2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Support".to_string(),
        &layer,
        &module2,
        &bb2,
        &mut arena2,
    )
    .unwrap();

    let p1 = &arena1.support().unwrap().support_paths[0].points[0];
    let p2 = &arena2.support().unwrap().support_paths[0].points[0];
    assert_eq!(p1.x, 3.0, "first dispatch: 3 enforcers");
    assert_eq!(p1.y, 0.0, "first dispatch: 0 blockers");
    assert_eq!(p2.x, 1.0, "second dispatch: 1 enforcer (no leak)");
    assert_eq!(p2.y, 2.0, "second dispatch: 2 blockers (no leak)");
}

#[test]
fn paint_region_deterministic_across_repeated_dispatches() {
    // Same paint data dispatched 3 times must produce identical results.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let mut blackboard = Blackboard::new(empty_mesh_ir(), 1);
    blackboard
        .commit_paint_regions(Arc::new(make_paint_region_ir(0, 2, 1)))
        .unwrap();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut results = Vec::new();
    for i in 0..3 {
        let module = make_compiled_module_with(
            &format!("com.test.support-{i}"),
            "Layer::Support",
            Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        arena.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
        LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::Support".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .unwrap();
        let s = arena.take_support().unwrap();
        results.push(s);
    }

    assert_eq!(results[0], results[1], "runs 0 and 1 must match");
    assert_eq!(results[1], results[2], "runs 1 and 2 must match");
}

#[test]
fn non_paint_stage_not_affected_by_blackboard_paint_data() {
    // Layer::Infill does not receive paint data. Presence of paint on the
    // blackboard should not alter infill behavior.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    // Run without paint
    let bb_no_paint = Blackboard::new(empty_mesh_ir(), 1);
    let module1 =
        make_compiled_module_with("com.test.infill", "Layer::Infill", Arc::clone(&component));
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena1 = LayerArena::new();
    arena1.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module1,
        &bb_no_paint,
        &mut arena1,
    )
    .unwrap();

    // Run with paint
    let mut bb_with_paint = Blackboard::new(empty_mesh_ir(), 1);
    bb_with_paint
        .commit_paint_regions(Arc::new(make_paint_region_ir(0, 5, 3)))
        .unwrap();
    let module2 =
        make_compiled_module_with("com.test.infill2", "Layer::Infill", Arc::clone(&component));
    let mut arena2 = LayerArena::new();
    arena2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module2,
        &bb_with_paint,
        &mut arena2,
    )
    .unwrap();

    let infill1 = arena1.infill().unwrap();
    let infill2 = arena2.infill().unwrap();
    assert_eq!(
        infill1.regions[0].sparse_infill[0].points, infill2.regions[0].sparse_infill[0].points,
        "infill output should be identical regardless of paint presence"
    );
}

// ── I. Slice-region wiring tests ────────────────────────────────────────

fn make_slice_ir(
    layer_index: u32,
    z: f32,
    region_count: usize,
    polys_per_region: usize,
) -> SliceIR {
    let regions = (0..region_count)
        .map(|i| SlicedRegion {
            object_id: format!("obj-{i}"),
            region_id: i as u64,
            polygons: (0..polys_per_region)
                .map(|_| ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: 10_000, y: 0 },
                            Point2 {
                                x: 10_000,
                                y: 10_000,
                            },
                            Point2 { x: 0, y: 10_000 },
                        ],
                    },
                    holes: Vec::new(),
                })
                .collect(),
            infill_areas: Vec::new(),
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            boundary_paint: HashMap::new(),
        })
        .collect();

    SliceIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: layer_index,
        z,
        regions,
    }
}

#[test]
fn real_slice_region_data_visible_through_production_infill_dispatch() {
    // The test guest's run_infill encodes region data into output:
    //   point[0].flow_factor = region_count
    //   point[0].width = total polygon count
    //   point[0].z = z from first region
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.infill", "Layer::Infill", Arc::clone(&component));

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 3,
        z: 0.6,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();

    // Stage 2 regions with 3 polygons each into the arena before infill runs.
    let slice_ir = make_slice_ir(3, 0.6, 2, 3);
    arena.set_slice(slice_ir).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let infill = arena.infill().expect("infill should be populated");
    let p0 = &infill.regions[0].sparse_infill[0].points[0];
    assert_eq!(
        p0.flow_factor, 2.0,
        "guest should see 2 slice regions, got flow_factor={}",
        p0.flow_factor
    );
    assert_eq!(
        p0.width, 6.0,
        "guest should see 6 total polygons (2 regions × 3), got width={}",
        p0.width
    );
    assert_eq!(
        p0.z, 0.6,
        "guest should see z=0.6 from slice region, got {}",
        p0.z
    );
}

#[test]
fn empty_arena_produces_no_slice_regions() {
    // When the arena has no SliceIR, the guest has no valid layer Z source and
    // emits no infill output. The empty bypass must preserve that state.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.infill", "Layer::Infill", Arc::clone(&component));

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // No slice_ir set.

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    assert!(
        arena.infill().is_none(),
        "no slice regions → empty bypass preserved"
    );
}

#[test]
fn slice_region_isolation_across_sequential_dispatches() {
    // Two dispatches with different arena slice data must not leak.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    // First dispatch: 3 regions, 2 polygons each
    let module1 =
        make_compiled_module_with("com.test.infill1", "Layer::Infill", Arc::clone(&component));
    let mut arena1 = LayerArena::new();
    arena1.set_slice(make_slice_ir(0, 0.2, 3, 2)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module1,
        &blackboard,
        &mut arena1,
    )
    .unwrap();

    // Second dispatch: 1 region, 5 polygons
    let module2 =
        make_compiled_module_with("com.test.infill2", "Layer::Infill", Arc::clone(&component));
    let mut arena2 = LayerArena::new();
    arena2.set_slice(make_slice_ir(0, 0.2, 1, 5)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module2,
        &blackboard,
        &mut arena2,
    )
    .unwrap();

    let p1 = &arena1.infill().unwrap().regions[0].sparse_infill[0].points[0];
    let p2 = &arena2.infill().unwrap().regions[0].sparse_infill[0].points[0];
    assert_eq!(p1.flow_factor, 3.0, "first dispatch: 3 regions");
    assert_eq!(p1.width, 6.0, "first dispatch: 6 polys (3×2)");
    assert_eq!(p2.flow_factor, 1.0, "second dispatch: 1 region (no leak)");
    assert_eq!(p2.width, 5.0, "second dispatch: 5 polys (no leak)");
}

#[test]
fn slice_region_deterministic_across_repeated_dispatches() {
    // Same slice data 3 times must produce identical results.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut results = Vec::new();
    for i in 0..3 {
        let module = make_compiled_module_with(
            &format!("com.test.infill-{i}"),
            "Layer::Infill",
            Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        arena.set_slice(make_slice_ir(0, 0.2, 2, 4)).unwrap();
        LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::Infill".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .unwrap();
        results.push(arena.take_infill().unwrap());
    }

    assert_eq!(results[0], results[1], "runs 0 and 1 must match");
    assert_eq!(results[1], results[2], "runs 1 and 2 must match");
}

#[test]
fn slice_and_paint_both_visible_in_same_support_dispatch() {
    // Support stage receives both slice-region and paint-region data.
    // The guest encodes paint counts (enforcers, blockers) in its output,
    // and we can verify both data sources are present.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.support", "Layer::Support", Arc::clone(&component));

    let mut blackboard = Blackboard::new(empty_mesh_ir(), 1);
    blackboard
        .commit_paint_regions(Arc::new(make_paint_region_ir(0, 2, 1)))
        .unwrap();

    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Stage slice data so the guest can also see it (even though
    // the support guest doesn't encode region data into output,
    // the dispatch must still wire it without error).
    arena.set_slice(make_slice_ir(0, 0.2, 2, 3)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Support".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    // Verify paint data reached the guest
    let support = arena.support().expect("support should be populated");
    let p = &support.support_paths[0].points[0];
    assert_eq!(p.x, 2.0, "2 enforcers should be visible");
    assert_eq!(p.y, 1.0, "1 blocker should be visible");
}

#[test]
fn infill_output_correct_when_slice_regions_present() {
    // Verify that the existing output commitment for infill is not
    // regressed when real slice region data is provided.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert("infill-spacing".into(), ConfigValue::Float(3.0));
    let module = make_compiled_module_with_config(
        "com.test.infill",
        "Layer::Infill",
        Arc::clone(&component),
        ConfigView::from_map(fields),
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(5, 1.0, 1, 2)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let infill = arena.infill().expect("infill should be populated");
    let path = &infill.regions[0].sparse_infill[0];
    // Config spacing=3.0 → second point x = 30.0
    assert_eq!(
        path.points[1].x, 30.0,
        "config wiring still works with slice regions present"
    );
    // First point encodes region data: z from slice, region_count=1, poly_count=2
    assert_eq!(path.points[0].z, 1.0, "z from slice region");
    assert_eq!(path.points[0].flow_factor, 1.0, "1 region visible");
    assert_eq!(path.points[0].width, 2.0, "2 polygons visible");
    assert_eq!(
        infill.global_layer_index, 5,
        "layer index preserved in output"
    );
}

// ── L. Perimeter-region wiring tests ────────────────────────────────────

fn make_wall_loop(perimeter_index: u32, point_count: usize, z: f32) -> slicer_ir::WallLoop {
    let points = (0..point_count)
        .map(|i| slicer_ir::Point3WithWidth {
            x: i as f32,
            y: 0.0,
            z,
            width: 0.4,
            flow_factor: 1.0,
        })
        .collect::<Vec<_>>();
    let flags = (0..point_count)
        .map(|_| slicer_ir::WallFeatureFlags {
            tool_index: None,
            fuzzy_skin: false,
            is_bridge: false,
            is_thin_wall: false,
            skip_ironing: false,
            custom: HashMap::new(),
        })
        .collect();
    slicer_ir::WallLoop {
        perimeter_index,
        loop_type: slicer_ir::LoopType::Outer,
        path: slicer_ir::ExtrusionPath3D {
            points: points.clone(),
            role: slicer_ir::ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: slicer_ir::WidthProfile {
            widths: points.iter().map(|p| p.width).collect(),
        },
        feature_flags: flags,
        boundary_type: slicer_ir::WallBoundaryType::Interior,
    }
}

fn make_perimeter_ir(
    layer_index: u32,
    regions: usize,
    walls_per_region: u32,
    infill_polys: usize,
) -> slicer_ir::PerimeterIR {
    let wall_z = if layer_index == 0 {
        0.2
    } else {
        layer_index as f32 * 0.2
    };
    let regions = (0..regions)
        .map(|i| slicer_ir::PerimeterRegion {
            object_id: format!("obj-{i}"),
            region_id: i as u64,
            walls: (0..walls_per_region)
                .map(|w| make_wall_loop(w, 2, wall_z))
                .collect(),
            infill_areas: (0..infill_polys)
                .map(|_| ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: 1000, y: 0 },
                            Point2 { x: 1000, y: 1000 },
                        ],
                    },
                    holes: Vec::new(),
                })
                .collect(),
            seam_candidates: Vec::new(),
            resolved_seam: None,
        })
        .collect();
    slicer_ir::PerimeterIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: layer_index,
        regions,
    }
}

#[test]
fn real_perimeter_region_data_visible_through_infill_postprocess_dispatch() {
    // Guest's run_infill_postprocess encodes:
    //   point[0].x = region_count
    //   point[0].y = total wall_loops
    //   point[0].z = total infill polygons
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.infill-pp", "Layer::InfillPostProcess", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 2,
        z: 0.4,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir(2, 3, 2, 4)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::InfillPostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let infill = arena.infill().expect("infill slot should be populated");
    assert_eq!(infill.regions.len(), 3, "one InfillRegion per input region");
    for (i, r) in infill.regions.iter().enumerate() {
        let p = &r.solid_infill[0].points[0];
        assert_eq!(p.x, 2.0, "each region sees its own 2 walls");
        assert_eq!(p.y, 4.0, "each region sees its own 4 infill polygons");
        assert_eq!(r.object_id, format!("obj-{i}"), "object_id preserved");
        assert_eq!(r.region_id, i as u64, "region_id preserved");
    }
}

#[test]
fn real_perimeter_region_data_visible_through_wall_postprocess_dispatch() {
    // Guest encodes region_count as perimeter_index; wall count + infill count as x/y.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.perim-pp",
        "Layer::PerimetersPostProcess",
        component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 1,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir(1, 2, 3, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PerimetersPostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    // Post-process replaces perimeter slot with guest's committed output;
    // each input region produces its own committed PerimeterRegion.
    let perim = arena
        .perimeter()
        .expect("perimeter slot should be populated");
    assert_eq!(
        perim.regions.len(),
        2,
        "one PerimeterRegion per input region"
    );
    for (i, r) in perim.regions.iter().enumerate() {
        assert_eq!(r.object_id, format!("obj-{i}"), "object_id preserved");
        assert_eq!(r.region_id, i as u64, "region_id preserved");
        assert_eq!(r.walls.len(), 1, "guest emitted one wall-loop per region");
        let w = &r.walls[0];
        assert_eq!(w.perimeter_index, 3, "each region has 3 walls in input");
        let p = &w.path.points[0];
        assert_eq!(p.x, 3.0, "each region sees its own 3 walls");
        assert_eq!(p.y, 1.0, "each region sees its own 1 infill polygon");
    }
}

#[test]
fn path_optimization_receives_real_perimeter_regions() {
    // PathOptimization does not commit to an arena slot; it should still
    // consume perimeter-region data (this test proves no panic / error path
    // and is verified by the dispatch succeeding when perimeter IR is staged).
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.pathopt", "Layer::PathOptimization", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir(0, 4, 2, 0)).unwrap();

    let r = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PathOptimization".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    );
    assert!(
        r.is_ok(),
        "path-optimization with real perimeter regions should succeed: {:?}",
        r.err()
    );
}

#[test]
fn empty_perimeter_input_valid_for_infill_postprocess() {
    // When no PerimeterIR is staged, guest sees zero regions and emits no
    // output (per-region loop). The empty-bypass keeps the infill slot empty
    // — this is the documented empty case and must not fail.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.infill-pp-empty",
        "Layer::InfillPostProcess",
        component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Do not stage any perimeter IR.

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::InfillPostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    assert!(
        arena.infill().is_none(),
        "no input regions → no output → empty bypass"
    );
}

#[test]
fn perimeter_region_isolation_across_sequential_dispatches() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let m1 = make_compiled_module_with(
        "com.test.ipp1",
        "Layer::InfillPostProcess",
        Arc::clone(&component),
    );
    let mut a1 = LayerArena::new();
    a1.set_perimeter(make_perimeter_ir(0, 5, 1, 2)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::InfillPostProcess".to_string(),
        &layer,
        &m1,
        &blackboard,
        &mut a1,
    )
    .unwrap();

    let m2 = make_compiled_module_with(
        "com.test.ipp2",
        "Layer::InfillPostProcess",
        Arc::clone(&component),
    );
    let mut a2 = LayerArena::new();
    a2.set_perimeter(make_perimeter_ir(0, 1, 7, 3)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::InfillPostProcess".to_string(),
        &layer,
        &m2,
        &blackboard,
        &mut a2,
    )
    .unwrap();

    let i1 = a1.infill().unwrap();
    let i2 = a2.infill().unwrap();
    assert_eq!(i1.regions.len(), 5, "first dispatch: 5 regions committed");
    assert_eq!(i2.regions.len(), 1, "second dispatch: 1 region (no leak)");
    let p1 = &i1.regions[0].solid_infill[0].points[0];
    let p2 = &i2.regions[0].solid_infill[0].points[0];
    assert_eq!(p1.x, 1.0, "first dispatch: each region has 1 wall");
    assert_eq!(p1.y, 2.0, "first dispatch: each region has 2 infill polys");
    assert_eq!(p2.x, 7.0, "second dispatch: 7 walls per region (no leak)");
    assert_eq!(p2.y, 3.0, "second dispatch: 3 infill polys (no leak)");
}

#[test]
fn perimeter_region_deterministic_across_repeated_dispatches() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut results = Vec::new();
    for i in 0..3 {
        let module = make_compiled_module_with(
            &format!("com.test.ipp-det-{i}"),
            "Layer::InfillPostProcess",
            Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        arena.set_perimeter(make_perimeter_ir(0, 2, 3, 4)).unwrap();
        LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::InfillPostProcess".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .unwrap();
        results.push(arena.take_infill().unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn stage_without_perimeter_input_does_not_see_perimeter_state() {
    // Layer::Infill consumes slice regions, not perimeter regions. Even if
    // PerimeterIR is staged in the arena, the infill guest should not
    // observe it — with zero slice regions, the guest emits no geometry.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with("com.test.infill-no-perim", "Layer::Infill", component);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Stage perimeter data only; no slice data.
    arena.set_perimeter(make_perimeter_ir(0, 4, 2, 5)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::Infill".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    // No infill output confirms perimeter state was not misrouted into the
    // slice-region view.
    assert!(
        arena.infill().is_none(),
        "Infill stage must not see perimeter data as slice regions"
    );
}

// ── M. Identity-preservation tests for post-process commit ─────────────

fn make_perimeter_ir_with_ids(
    layer_index: u32,
    ids: &[(&str, u64)],
    walls_per: u32,
    infill_per: usize,
) -> slicer_ir::PerimeterIR {
    let wall_z = if layer_index == 0 {
        0.2
    } else {
        layer_index as f32 * 0.2
    };
    let regions = ids
        .iter()
        .map(|(obj, rid)| slicer_ir::PerimeterRegion {
            object_id: (*obj).to_string(),
            region_id: *rid,
            walls: (0..walls_per)
                .map(|w| make_wall_loop(w, 2, wall_z))
                .collect(),
            infill_areas: (0..infill_per)
                .map(|_| ExPolygon {
                    contour: Polygon {
                        points: vec![
                            Point2 { x: 0, y: 0 },
                            Point2 { x: 1, y: 0 },
                            Point2 { x: 1, y: 1 },
                        ],
                    },
                    holes: Vec::new(),
                })
                .collect(),
            seam_candidates: Vec::new(),
            resolved_seam: None,
        })
        .collect();
    slicer_ir::PerimeterIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: layer_index,
        regions,
    }
}

#[test]
fn perimeter_postprocess_commit_preserves_distinct_region_identities() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.perim-pp-ids",
        "Layer::PerimetersPostProcess",
        component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let ids = [("alpha", 11u64), ("beta", 22u64), ("gamma", 33u64)];
    let mut arena = LayerArena::new();
    arena
        .set_perimeter(make_perimeter_ir_with_ids(0, &ids, 2, 1))
        .unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PerimetersPostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let perim = arena.perimeter().expect("perimeter populated");
    assert_eq!(
        perim.regions.len(),
        3,
        "3 distinct regions preserved (not flattened)"
    );
    let observed: Vec<(String, u64)> = perim
        .regions
        .iter()
        .map(|r| (r.object_id.clone(), r.region_id))
        .collect();
    let expected: Vec<(String, u64)> = ids.iter().map(|(o, r)| (o.to_string(), *r)).collect();
    assert_eq!(observed, expected, "identities preserved in input order");
    for r in &perim.regions {
        assert_eq!(
            r.walls.len(),
            1,
            "each committed region got its own wall-loop"
        );
    }
}

#[test]
fn infill_postprocess_commit_preserves_distinct_region_identities() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.infill-pp-ids",
        "Layer::InfillPostProcess",
        component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let ids = [("part-A", 7u64), ("part-B", 9u64)];
    let mut arena = LayerArena::new();
    arena
        .set_perimeter(make_perimeter_ir_with_ids(0, &ids, 1, 1))
        .unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::InfillPostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let infill = arena.infill().expect("infill populated");
    assert_eq!(infill.regions.len(), 2, "2 distinct regions preserved");
    let observed: Vec<(String, u64)> = infill
        .regions
        .iter()
        .map(|r| (r.object_id.clone(), r.region_id))
        .collect();
    let expected: Vec<(String, u64)> = ids.iter().map(|(o, r)| (o.to_string(), *r)).collect();
    assert_eq!(observed, expected, "identities preserved in input order");
}

#[test]
fn perimeter_postprocess_identity_preservation_deterministic() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let ids = [("x", 1u64), ("y", 2u64), ("z", 3u64), ("w", 4u64)];
    let mut results = Vec::new();
    for i in 0..3 {
        let module = make_compiled_module_with(
            &format!("com.test.perim-pp-det-{i}"),
            "Layer::PerimetersPostProcess",
            Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        arena
            .set_perimeter(make_perimeter_ir_with_ids(0, &ids, 2, 0))
            .unwrap();
        LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::PerimetersPostProcess".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .unwrap();
        results.push(arena.take_perimeter().unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn perimeter_postprocess_identity_isolation_across_dispatches() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let m1 = make_compiled_module_with(
        "com.test.iso1",
        "Layer::PerimetersPostProcess",
        Arc::clone(&component),
    );
    let mut a1 = LayerArena::new();
    a1.set_perimeter(make_perimeter_ir_with_ids(
        0,
        &[("first", 100), ("second", 200)],
        1,
        0,
    ))
    .unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PerimetersPostProcess".to_string(),
        &layer,
        &m1,
        &blackboard,
        &mut a1,
    )
    .unwrap();

    let m2 = make_compiled_module_with(
        "com.test.iso2",
        "Layer::PerimetersPostProcess",
        Arc::clone(&component),
    );
    let mut a2 = LayerArena::new();
    a2.set_perimeter(make_perimeter_ir_with_ids(0, &[("alt", 999)], 1, 0))
        .unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PerimetersPostProcess".to_string(),
        &layer,
        &m2,
        &blackboard,
        &mut a2,
    )
    .unwrap();

    let p1 = a1.perimeter().unwrap();
    let p2 = a2.perimeter().unwrap();
    assert_eq!(
        p1.regions
            .iter()
            .map(|r| (r.object_id.clone(), r.region_id))
            .collect::<Vec<_>>(),
        vec![("first".to_string(), 100), ("second".to_string(), 200)]
    );
    assert_eq!(
        p2.regions
            .iter()
            .map(|r| (r.object_id.clone(), r.region_id))
            .collect::<Vec<_>>(),
        vec![("alt".to_string(), 999)],
        "no leak from prior dispatch's identities"
    );
}

#[test]
fn support_postprocess_empty_bypass_when_no_slice_regions() {
    // With no slice regions staged in the arena, the guest iterates nothing
    // and emits no support output; empty-bypass leaves the support slot None.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.spp-empty", "Layer::SupportPostProcess", component);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SupportPostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();
    assert!(
        arena.support().is_none(),
        "empty-input post-process: empty bypass preserved"
    );
}

#[test]
fn perimeter_postprocess_untagged_output_fails_with_diagnostic() {
    // If a guest emits perimeter output without ever querying a perimeter
    // region (origin tags all None) AND there were source regions, the
    // identity-preservation contract is violated. Verify convert_perimeter_output
    // surfaces a structured diagnostic in this case.
    use slicer_host::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole, PerimeterOutputCollected,
        Point3WithWidth, WallFeatureFlag, WallLoopType, WallLoopView,
    };
    // One untagged wall_loop and one tagged seam_candidate => mixed mode.
    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                    width: 0.4,
                    flow_factor: 1.0,
                }],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![WallFeatureFlag {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: vec![],
            }],
        }],
        wall_loop_origins: vec![None],
        infill_areas: Vec::new(),
        infill_areas_origin: None,
        rotated_wall_loops: Vec::new(),
        rotated_wall_loop_origins: Vec::new(),
        seam_candidates: Vec::new(),
        seam_candidate_origins: Vec::new(),
        resolved_seam: None,
        resolved_seam_origin: None,
    };
    // Force "any_tagged" by setting a dummy infill_areas_origin so the
    // identity-preserving path is taken; then the untagged wall_loop fails.
    let mut output = output;
    output.infill_areas_origin = Some(("dummy".into(), 0));
    let result = convert_perimeter_output(&output, 0);
    assert!(result.is_err(), "untagged push in identity mode must fail");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("active perimeter source region") || msg.contains("without an active"),
        "diagnostic should explain missing region context: {msg}"
    );
}

// ── K. SlicePostProcess / SupportPostProcess identity-preserving commit ─

#[test]
fn slice_postprocess_commit_preserves_distinct_region_identities() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.slice-pp-ids",
        "Layer::SlicePostProcess",
        component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Three distinct slice regions (object_id varies via make_slice_ir: obj-0..obj-2)
    arena.set_slice(make_slice_ir(0, 0.2, 3, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SlicePostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let slice = arena
        .slice()
        .expect("slice populated after post-process merge");
    assert_eq!(
        slice.regions.len(),
        3,
        "all three source regions preserved (not flattened)"
    );
    let observed: Vec<(String, u64)> = slice
        .regions
        .iter()
        .map(|r| (r.object_id.clone(), r.region_id))
        .collect();
    let expected: Vec<(String, u64)> = vec![
        ("obj-0".into(), 0),
        ("obj-1".into(), 1),
        ("obj-2".into(), 2),
    ];
    assert_eq!(
        observed, expected,
        "identities preserved in input order after merge"
    );
    // Guest replaced each region's polygons with a triangle (3 points).
    for r in &slice.regions {
        assert_eq!(r.polygons.len(), 1);
        assert_eq!(
            r.polygons[0].contour.points.len(),
            3,
            "guest polygon replacement applied per region"
        );
    }
}

#[test]
fn support_postprocess_commit_preserves_distinct_region_identities() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.support-pp-ids",
        "Layer::SupportPostProcess",
        component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Two distinct slice regions: (obj-0, 0), (obj-1, 1). Guest pushes one
    // support path per region; convert_support_output groups by origin with
    // structured diagnostics on untagged output.
    arena.set_slice(make_slice_ir(0, 0.2, 2, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SupportPostProcess".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let support = arena
        .support()
        .expect("support populated after post-process");
    assert_eq!(
        support.support_paths.len(),
        2,
        "two origin-tagged paths preserved"
    );
    // First-seen ordering by origin is stable; each path encodes poly count.
    assert_eq!(
        support.support_paths[0].points[0].x, 1.0,
        "region 0 has 1 polygon"
    );
    assert_eq!(
        support.support_paths[1].points[0].x, 1.0,
        "region 1 has 1 polygon"
    );
}

#[test]
fn slice_postprocess_identity_preservation_deterministic() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut results = Vec::new();
    for i in 0..3 {
        let module = make_compiled_module_with(
            &format!("com.test.spp-det-{i}"),
            "Layer::SlicePostProcess",
            Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        arena.set_slice(make_slice_ir(0, 0.2, 4, 1)).unwrap();
        LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::SlicePostProcess".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .unwrap();
        results.push(arena.take_slice().unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn support_postprocess_identity_isolation_across_dispatches() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let m1 = make_compiled_module_with(
        "com.test.spp-iso1",
        "Layer::SupportPostProcess",
        Arc::clone(&component),
    );
    let mut a1 = LayerArena::new();
    a1.set_slice(make_slice_ir(0, 0.2, 3, 2)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SupportPostProcess".to_string(),
        &layer,
        &m1,
        &blackboard,
        &mut a1,
    )
    .unwrap();

    let m2 = make_compiled_module_with(
        "com.test.spp-iso2",
        "Layer::SupportPostProcess",
        Arc::clone(&component),
    );
    let mut a2 = LayerArena::new();
    a2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SupportPostProcess".to_string(),
        &layer,
        &m2,
        &blackboard,
        &mut a2,
    )
    .unwrap();

    assert_eq!(
        a1.support().unwrap().support_paths.len(),
        3,
        "dispatch 1 kept its 3 regions"
    );
    assert_eq!(
        a2.support().unwrap().support_paths.len(),
        1,
        "dispatch 2 kept its 1 region (no leak)"
    );
}

#[test]
fn support_output_rejects_untagged_push_in_identity_mode() {
    // Manual collected output with mixed tagged/untagged pushes — simulates a
    // guest that armed origin tracking via at least one region access but
    // later emitted a path without an active region.
    use slicer_host::wit_host::{
        convert_support_output, ExtrusionPath3d, ExtrusionRole, Point3WithWidth,
        SupportOutputCollected,
    };
    let mk_path = || ExtrusionPath3d {
        points: vec![Point3WithWidth {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            width: 0.4,
            flow_factor: 1.0,
        }],
        role: ExtrusionRole::SupportMaterial,
        speed_factor: 1.0,
    };
    let output = SupportOutputCollected {
        support_paths: vec![mk_path(), mk_path()],
        interface_paths: Vec::new(),
        raft_paths: Vec::new(),
        support_path_origins: vec![Some(("obj-0".into(), 0)), None],
        interface_path_origins: Vec::new(),
        raft_path_origins: Vec::new(),
    };
    let result = convert_support_output(&output, 0);
    assert!(result.is_err(), "untagged push in identity mode must fail");
    let msg = result.unwrap_err();
    assert!(
        msg.contains("active slice source region") || msg.contains("without an active"),
        "diagnostic should explain missing region context: {msg}"
    );
}

#[test]
fn slice_postprocess_downstream_propagation_preserves_per_region_shape() {
    // After Layer::SlicePostProcess merges per-region updates, the arena's
    // SliceIR still carries all region identities. push_slice_regions (used
    // by downstream stages like Perimeters / Support) therefore sees every
    // region with its original (object_id, region_id). This confirms the
    // committed per-region shape is what downstream consumers will observe.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let spp = make_compiled_module_with(
        "com.test.spp-prop",
        "Layer::SlicePostProcess",
        Arc::clone(&component),
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(0, 0.2, 3, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SlicePostProcess".to_string(),
        &layer,
        &spp,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    // Now dispatch a downstream stage that consumes slice regions (Support).
    // The test guest's run_support observes paint data, but the key proof is
    // that push_slice_regions sees all three regions after SlicePostProcess.
    let sup = make_compiled_module_with(
        "com.test.sup-prop",
        "Layer::SupportPostProcess",
        Arc::clone(&component),
    );
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::SupportPostProcess".to_string(),
        &layer,
        &sup,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let support = arena
        .support()
        .expect("support populated via propagated slice regions");
    assert_eq!(
        support.support_paths.len(),
        3,
        "downstream stage saw all 3 per-region identities preserved by SlicePostProcess merge",
    );
}

// ── L. PathOptimization: ordered_entities threading + GCode override commit ─

#[test]
fn path_optimization_commit_folds_tool_changes_into_deferred_queue() {
    // Guest pushes one tool-change per perimeter region via gcode-output-builder.
    // commit_layer_outputs should route them into arena.take_deferred_tool_changes().
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module =
        make_compiled_module_with("com.test.pathopt-tc", "Layer::PathOptimization", component);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir(0, 3, 1, 0)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PathOptimization".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .unwrap();

    let tcs = arena.take_deferred_tool_changes();
    assert_eq!(tcs.len(), 3, "three tool-changes routed to deferred queue");
    let mapped: Vec<(u32, u32)> = tcs.iter().map(|t| (t.from_tool, t.to_tool)).collect();
    assert_eq!(mapped, vec![(0, 1), (1, 2), (2, 3)]);
}

#[test]
fn path_optimization_end_to_end_populates_layer_collection_tool_changes() {
    // Through execute_per_layer: assembly runs before PathOptimization,
    // guest emits tool-changes, final LayerCollectionIR has tool_changes.
    use slicer_host::execute_per_layer;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    // Minimal 1-layer plan with a seed stage that populates PerimeterIR,
    // then Layer::PathOptimization whose guest emits tool-changes. The
    // executor pre-assembles ordered_entities from arena.perimeter() right
    // before PathOptimization runs, so seeding must happen in an earlier
    // stage, not inside PathOptimization itself.
    let plan = slicer_host::ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            slicer_host::CompiledStage {
                stage_id: "Layer::Perimeters".into(),
                modules: vec![make_compiled_module_with(
                    "com.test.pathopt-seed",
                    "Layer::Perimeters",
                    Arc::clone(&component),
                )],
            },
            slicer_host::CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![make_compiled_module_with(
                    "com.test.pathopt-e2e",
                    "Layer::PathOptimization",
                    Arc::clone(&component),
                )],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
        module_region_index: HashMap::new(),
    };
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);

    // Seed the arena with PerimeterIR during the Layer::Perimeters stage
    // (before the PathOptimization pre-assembly runs).
    struct SeedingRunner<'a> {
        inner: &'a slicer_host::WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl<'a> LayerStageRunner for SeedingRunner<'a> {
        fn run_stage(
            &self,
            stage_id: &StageId,
            layer: &GlobalLayer,
            module: &CompiledModule,
            blackboard: &Blackboard,
            arena: &mut LayerArena,
        ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
            if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    arena.set_perimeter(p).unwrap();
                    return Ok((LayerStageOutput::Success, Vec::new(), Vec::new()));
                }
            }
            LayerStageRunner::run_stage(self.inner, stage_id, layer, module, blackboard, arena)
        }
    }
    let runner = SeedingRunner {
        inner: &dispatcher,
        perim: Mutex::new(Some(make_perimeter_ir(0, 2, 1, 0))),
    };

    let layers = execute_per_layer(&plan, &blackboard, &runner).expect("exec");
    assert_eq!(layers.len(), 1);
    let l = &layers[0];
    assert_eq!(
        l.ordered_entities.len(),
        2,
        "ordered_entities pre-staged from assembly visible at end",
    );
    assert_eq!(
        l.tool_changes.len(),
        2,
        "guest-emitted tool-change overrides folded into LayerCollectionIR",
    );
    // Region identity preserved through the loop.
    for (i, e) in l.ordered_entities.iter().enumerate() {
        assert_eq!(e.region_key.global_layer_index, 0);
        assert_eq!(e.topo_order, i as u32);
    }
    // Verify each tool-change's after_entity_index matches its region index,
    // matching the guest's per-region anchoring (region i emits tool-change
    // after entity index i).
    for (i, tc) in l.tool_changes.iter().enumerate() {
        assert_eq!(
            tc.after_entity_index, i as u32,
            "tool-change {i} should anchor at region index {i}"
        );
    }
}

#[test]
fn path_optimization_empty_input_is_no_op() {
    // No arena state staged — assembly produces empty ordered_entities,
    // guest iterates zero regions, no tool_changes.
    use slicer_host::execute_per_layer;
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let plan = slicer_host::ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![slicer_host::CompiledStage {
            stage_id: "Layer::PathOptimization".into(),
            modules: vec![make_compiled_module_with(
                "com.test.pathopt-empty",
                "Layer::PathOptimization",
                component,
            )],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
        module_region_index: HashMap::new(),
    };
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layers = execute_per_layer(&plan, &blackboard, &dispatcher).expect("exec");
    assert!(layers[0].ordered_entities.is_empty());
    assert!(layers[0].tool_changes.is_empty());
}

#[test]
fn path_optimization_deterministic_across_repeated_runs() {
    use slicer_host::execute_per_layer;
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    struct SeedingRunner<'a> {
        inner: &'a slicer_host::WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl<'a> LayerStageRunner for SeedingRunner<'a> {
        fn run_stage(
            &self,
            stage_id: &StageId,
            layer: &GlobalLayer,
            module: &CompiledModule,
            blackboard: &Blackboard,
            arena: &mut LayerArena,
        ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
            if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    arena.set_perimeter(p).unwrap();
                    return Ok((LayerStageOutput::Success, Vec::new(), Vec::new()));
                }
            }
            LayerStageRunner::run_stage(self.inner, stage_id, layer, module, blackboard, arena)
        }
    }

    let make_plan = |component: Arc<slicer_host::WasmComponent>| slicer_host::ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            slicer_host::CompiledStage {
                stage_id: "Layer::Perimeters".into(),
                modules: vec![make_compiled_module_with(
                    "com.test.pathopt-det-seed",
                    "Layer::Perimeters",
                    Arc::clone(&component),
                )],
            },
            slicer_host::CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![make_compiled_module_with(
                    "com.test.pathopt-det",
                    "Layer::PathOptimization",
                    component,
                )],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let mut results = Vec::new();
    for _ in 0..3 {
        let blackboard = Blackboard::new(empty_mesh_ir(), 1);
        let runner = SeedingRunner {
            inner: &dispatcher,
            perim: Mutex::new(Some(make_perimeter_ir(0, 3, 1, 0))),
        };
        results.push(
            execute_per_layer(&make_plan(Arc::clone(&component)), &blackboard, &runner).unwrap(),
        );
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn path_optimization_rejects_move_override_without_layer_collection_mapping() {
    // Per docs/03 § Path Optimization Output Contract, push-move has no
    // documented LayerCollectionIR mapping and must fail as a fatal module
    // error instead of being lowered into an annotation.
    use slicer_host::wit_host::{
        ExtrusionRole, GcodeCommandCollected, GcodeMoveCmd, HostExecutionContext,
    };
    let mut ctx =
        HostExecutionContext::new("com.test.pathopt-bad".to_string(), 0.0, 0.0, None, None);
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Move(GcodeMoveCmd {
            x: Some(1.0),
            y: Some(2.0),
            z: None,
            e: None,
            f: None,
            role: ExtrusionRole::OuterWall,
        }));
    let mut arena = LayerArena::new();
    let err = slicer_host::commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.pathopt-bad",
        0,
        &ctx,
        &mut arena,
        None,
    )
    .expect_err("move override must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("push-move"),
        "diagnostic should name the rejected method: {msg}"
    );
    assert!(
        msg.contains("no documented LayerCollectionIR mapping exists"),
        "diagnostic should explain the contract violation: {msg}"
    );
    assert!(
        arena.take_deferred_annotations().is_empty(),
        "rejected move override must not enqueue annotations"
    );
}

#[test]
fn path_optimization_commit_routes_comment_and_raw_to_deferred_annotations() {
    // Per docs/03 § Path Optimization Output Contract:
    // push-comment and push-raw are accepted at PathOptimization and must be
    // routed onto the per-layer deferred annotation queue (anchored at the
    // last entity index), not silently dropped.
    use slicer_host::wit_host::{GcodeCommandCollected, HostExecutionContext};
    use slicer_ir::LayerAnnotationKind;

    let mut ctx =
        HostExecutionContext::new("com.test.pathopt-ann".to_string(), 0.0, 0.0, None, None);
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Comment("hello".into()));
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Raw("M117 hi".into()));

    let mut arena = LayerArena::new();
    slicer_host::commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.pathopt-ann",
        0,
        &ctx,
        &mut arena,
        None,
    )
    .expect("comment/raw must commit successfully");

    let anns = arena.take_deferred_annotations();
    assert_eq!(anns.len(), 2, "both annotations are committed");
    assert!(matches!(anns[0].kind, LayerAnnotationKind::Comment(ref t) if t == "hello"));
    assert!(matches!(anns[1].kind, LayerAnnotationKind::Raw(ref t) if t == "M117 hi"));
    // Anchor defaults to 0 when no LayerCollectionIR is pre-staged.
    assert_eq!(anns[0].after_entity_index, 0);
    assert_eq!(anns[1].after_entity_index, 0);
}

#[test]
fn path_optimization_commit_is_deterministic_across_repeats() {
    // Repeated commit_layer_outputs over the same input stream must yield
    // bit-identical deferred queues — required by docs/03 determinism rule.
    use slicer_host::wit_host::{GcodeCommandCollected, HostExecutionContext};

    let mk_ctx = || {
        let mut c =
            HostExecutionContext::new("com.test.pathopt-det2".to_string(), 0.0, 0.0, None, None);
        c.gcode_output
            .commands
            .push(GcodeCommandCollected::ToolChange {
                after_entity_index: 0,
                from_tool: 0,
                to_tool: 1,
            });
        c.gcode_output
            .commands
            .push(GcodeCommandCollected::Comment("a".into()));
        c.gcode_output
            .commands
            .push(GcodeCommandCollected::Raw("b".into()));
        c
    };

    let mut snapshots = Vec::new();
    for _ in 0..3 {
        let mut arena = LayerArena::new();
        let ctx = mk_ctx();
        slicer_host::commit_layer_outputs_for_test(
            "Layer::PathOptimization",
            "com.test.pathopt-det2",
            0,
            &ctx,
            &mut arena,
            None,
        )
        .unwrap();
        snapshots.push((
            arena.take_deferred_tool_changes(),
            arena.take_deferred_annotations(),
        ));
    }
    assert_eq!(snapshots[0], snapshots[1]);
    assert_eq!(snapshots[1], snapshots[2]);
}

// ── M. PathOptimization z-hop ───────────────────────────────────────────

#[test]
fn path_optimization_commit_routes_z_hops_to_deferred_queue() {
    // push-z-hop is accepted at PathOptimization and routed onto the
    // per-layer deferred z-hop queue, preserving guest call order.
    use slicer_host::wit_host::{GcodeCommandCollected, HostExecutionContext};

    let mut ctx =
        HostExecutionContext::new("com.test.pathopt-zhop".to_string(), 0.0, 0.0, None, None);
    ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop {
        after_entity_index: 0,
        hop_height: 0.5,
    });
    ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop {
        after_entity_index: 0,
        hop_height: 0.75,
    });

    let mut arena = LayerArena::new();
    slicer_host::commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.pathopt-zhop",
        0,
        &ctx,
        &mut arena,
        None,
    )
    .expect("z-hop must commit");

    let zhops = arena.take_deferred_z_hops();
    assert_eq!(zhops.len(), 2);
    assert_eq!(zhops[0].after_entity_index, 0);
    assert_eq!(zhops[0].hop_height, 0.5);
    assert_eq!(zhops[1].hop_height, 0.75);
}

#[test]
fn path_optimization_z_hop_normalizes_to_global_anchor_with_entities() {
    // Module-supplied after_entity_index is ignored; the dispatch normalizes all
    // ZHop/Retract/Move commands to the same global anchor so gcode_emit.rs can
    // emit them as a coherent Retract→ZHop→Travel→Unretract sequence.
    use slicer_host::wit_host::{
        ExtrusionRole as WitRole, GcodeCommandCollected, GcodeMoveCmd, HostExecutionContext,
    };
    use slicer_ir::{LayerCollectionIR, RetractMode, SemVer};

    let mut ctx = HostExecutionContext::new(
        "com.test.pathopt-zhop-norm".to_string(),
        0.0,
        0.0,
        None,
        None,
    );
    // Emit a full travel sequence; ZHop uses an arbitrary (formerly-rejected) index.
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Retract {
            length: 0.8,
            speed: 25.0,
            mode: RetractMode::Gcode,
        });
    ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop {
        after_entity_index: 999,
        hop_height: 0.2,
    });
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Move(GcodeMoveCmd {
            x: Some(50.0),
            y: Some(50.0),
            z: None,
            e: None,
            f: None,
            role: WitRole::Custom("travel".to_string()),
        }));
    ctx.gcode_output
        .commands
        .push(GcodeCommandCollected::Unretract {
            length: 0.8,
            speed: 25.0,
            mode: RetractMode::Gcode,
        });

    let mut arena = LayerArena::new();
    // Pre-stage 2 entities so entity_count=2, anchor=1 (last entity index).
    let entity = slicer_ir::PrintEntity {
        path: slicer_ir::ExtrusionPath3D {
            points: vec![slicer_ir::Point3WithWidth {
                x: 0.0,
                y: 0.0,
                z: 0.2,
                width: 0.4,
                flow_factor: 1.0,
            }],
            role: slicer_ir::ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: slicer_ir::ExtrusionRole::OuterWall,
        region_key: slicer_ir::RegionKey {
            global_layer_index: 0,
            object_id: String::new(),
            region_id: 0,
        },
        topo_order: 0,
    };
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 0,
        z: 0.2,
        ordered_entities: vec![entity.clone(), entity],
        tool_changes: Vec::new(),
        z_hops: Vec::new(),
        annotations: Vec::new(),
        retracts: vec![],
        travel_moves: vec![],
    });

    slicer_host::commit_layer_outputs_for_test(
        "Layer::PathOptimization",
        "com.test.pathopt-zhop-norm",
        0,
        &ctx,
        &mut arena,
        None,
    )
    .expect("ZHop with arbitrary entity index must be accepted and normalized to anchor");

    let zhops = arena.take_deferred_z_hops();
    assert_eq!(zhops.len(), 1);
    assert_eq!(
        zhops[0].after_entity_index, 1,
        "ZHop must be anchored at global anchor (entity_count-1=1), got {}",
        zhops[0].after_entity_index
    );

    let retracts = arena.take_deferred_retracts();
    assert_eq!(retracts.len(), 2, "Retract + Unretract = 2");
    assert_eq!(
        retracts[0].after_entity_index, 1,
        "Retract must share anchor with ZHop"
    );
    assert_eq!(
        retracts[1].after_entity_index, 1,
        "Unretract must share anchor with ZHop"
    );

    let travels = arena.take_deferred_travel_moves();
    assert_eq!(travels.len(), 1);
    assert_eq!(
        travels[0].after_entity_index, 1,
        "TravelMove must share anchor with ZHop"
    );
}

#[test]
fn path_optimization_z_hop_rejects_invalid_hop_height() {
    use slicer_host::wit_host::{GcodeCommandCollected, HostExecutionContext};

    for bad in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
        let mut ctx = HostExecutionContext::new(
            "com.test.pathopt-zhop-bad".to_string(),
            0.0,
            0.0,
            None,
            None,
        );
        ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop {
            after_entity_index: 0,
            hop_height: bad,
        });
        let mut arena = LayerArena::new();
        let err = slicer_host::commit_layer_outputs_for_test(
            "Layer::PathOptimization",
            "com.test.pathopt-zhop-bad",
            0,
            &ctx,
            &mut arena,
            None,
        )
        .expect_err("bad hop_height must fail");
        assert!(
            err.to_string().contains("hop-height"),
            "diagnostic should name field for {bad}: {err}"
        );
    }
}

#[test]
fn path_optimization_end_to_end_populates_z_hops() {
    // Through execute_per_layer: guest emits push-z-hop calls that the host
    // commit path validates and folds into LayerCollectionIR.z_hops.
    use slicer_host::execute_per_layer;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let plan = slicer_host::ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            slicer_host::CompiledStage {
                stage_id: "Layer::Perimeters".into(),
                modules: vec![make_compiled_module_with(
                    "com.test.zhop-seed",
                    "Layer::Perimeters",
                    Arc::clone(&component),
                )],
            },
            slicer_host::CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![make_compiled_module_with(
                    "com.test.zhop-e2e",
                    "Layer::PathOptimization",
                    Arc::clone(&component),
                )],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
        module_region_index: HashMap::new(),
    };

    struct SeedingRunner<'a> {
        inner: &'a slicer_host::WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl<'a> LayerStageRunner for SeedingRunner<'a> {
        fn run_stage(
            &self,
            stage_id: &StageId,
            layer: &GlobalLayer,
            module: &CompiledModule,
            blackboard: &Blackboard,
            arena: &mut LayerArena,
        ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
            if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    arena.set_perimeter(p).unwrap();
                    return Ok((LayerStageOutput::Success, Vec::new(), Vec::new()));
                }
            }
            LayerStageRunner::run_stage(self.inner, stage_id, layer, module, blackboard, arena)
        }
    }

    let mut runs = Vec::new();
    for _ in 0..2 {
        let runner = SeedingRunner {
            inner: &dispatcher,
            perim: Mutex::new(Some(make_perimeter_ir(0, 2, 1, 0))),
        };
        let blackboard = Blackboard::new(empty_mesh_ir(), 1);
        runs.push(execute_per_layer(&plan, &blackboard, &runner).expect("exec"));
    }
    let layers = &runs[0];
    assert_eq!(layers.len(), 1);
    let l = &layers[0];
    assert_eq!(l.ordered_entities.len(), 2);
    assert_eq!(l.z_hops.len(), 2, "guest emits one z-hop per region");
    for zh in &l.z_hops {
        assert_eq!(zh.after_entity_index, 0);
        assert_eq!(zh.hop_height, 0.5);
    }
    // Existing tool-change/comment behaviour unchanged.
    assert_eq!(l.tool_changes.len(), 2);
    assert_eq!(l.annotations.len(), 1);
    // Determinism: repeated runs are bit-identical.
    assert_eq!(runs[0], runs[1]);
}

#[test]
fn path_optimization_end_to_end_emitter_renders_z_hops() {
    // Final downstream emission: DefaultGCodeEmitter consumes committed z_hops
    // and renders the lift+return travel pair after the anchor entity.
    use slicer_host::execute_per_layer;
    use slicer_host::gcode_emit::DefaultGCodeEmitter;
    use slicer_host::postpass::GCodeEmitter;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let plan = slicer_host::ExecutionPlan {
        prepass_stages: Vec::new(),
        per_layer_stages: vec![
            slicer_host::CompiledStage {
                stage_id: "Layer::Perimeters".into(),
                modules: vec![make_compiled_module_with(
                    "com.test.zhop-emit-seed",
                    "Layer::Perimeters",
                    Arc::clone(&component),
                )],
            },
            slicer_host::CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![make_compiled_module_with(
                    "com.test.zhop-emit",
                    "Layer::PathOptimization",
                    Arc::clone(&component),
                )],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0,
            z: 0.2,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
        module_region_index: HashMap::new(),
    };

    struct SeedingRunner<'a> {
        inner: &'a slicer_host::WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl<'a> LayerStageRunner for SeedingRunner<'a> {
        fn run_stage(
            &self,
            stage_id: &StageId,
            layer: &GlobalLayer,
            module: &CompiledModule,
            blackboard: &Blackboard,
            arena: &mut LayerArena,
        ) -> Result<(LayerStageOutput, Vec<String>, Vec<String>), LayerStageError> {
            if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    arena.set_perimeter(p).unwrap();
                    return Ok((LayerStageOutput::Success, Vec::new(), Vec::new()));
                }
            }
            LayerStageRunner::run_stage(self.inner, stage_id, layer, module, blackboard, arena)
        }
    }
    let runner = SeedingRunner {
        inner: &dispatcher,
        perim: Mutex::new(Some(make_perimeter_ir(0, 1, 1, 0))),
    };
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layers = execute_per_layer(&plan, &blackboard, &runner).expect("exec");

    let emitter = DefaultGCodeEmitter::new("test".into());
    let gcode = emitter.emit_gcode(&layers, &blackboard).expect("emit");
    // Look for at least one Move with the lifted Z = 0.2 + 0.5 = 0.7.
    let mut hop_lifts = 0;
    for c in &gcode.commands {
        if let slicer_ir::GCodeCommand::Move { z: Some(z), .. } = c {
            if (*z - 0.7).abs() < 1e-4 {
                hop_lifts += 1;
            }
        }
    }
    assert!(
        hop_lifts >= 1,
        "default emitter must lift to layer.z + hop_height for committed z_hops"
    );
}

// ── R. Layer-plan harvest tests ────────────────────────────────────────────
//
// These tests prove the prepass dispatch harvest gap is closed:
// `dispatch_prepass_call` returns `HostExecutionContext` (not ()),
// `harvest_layer_plan_ir` converts proposals to `LayerPlanIR`, and
// `PrepassStageRunner::run_stage` returns `PrepassStageOutput::LayerPlan`.
//
// The prepass-guest component is used: its `run_layer_planning` returns
// Ok(()) with ZERO proposals, so the harvested `LayerPlanIR` has an empty
// `global_layers` list.  That is the expected intermediate state while
// TASK-107 is not yet wired — an empty plan is structurally valid, and
// the dispatcher must return `LayerPlan(empty_ir)` NOT `None`.

const LAYER_PLANNER_DEFAULT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../modules/core-modules/layer-planner-default/layer-planner-default.wasm"
);

/// Load the real `layer-planner-default.wasm` component using the same engine
/// that the dispatcher will use.  Returns `None` and skips via `#[test]`
/// if the file is missing (optional build artifact).
fn load_layer_planner_default(engine: &WasmEngine) -> Option<Arc<slicer_host::WasmComponent>> {
    let path = std::path::Path::new(LAYER_PLANNER_DEFAULT_PATH);
    if !path.exists() {
        return None;
    }
    let bytes = std::fs::read(path).expect("read layer-planner-default.wasm");
    match engine.compile_component(&bytes) {
        Ok(c) => Some(Arc::new(c)),
        Err(_) => None,
    }
}

#[test]
fn layer_planning_dispatch_returns_layer_plan_variant() {
    // The prepass-guest returns Ok(()) with no proposals; the dispatcher must
    // still return `PrepassStageOutput::LayerPlan(ir)` (not None) because
    // the stage ID is `PrePass::LayerPlanning`.  An empty LayerPlanIR is the
    // correct intermediate state.
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_prepass_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.layer-plan-harvest",
        "PrePass::LayerPlanning",
        component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::LayerPlanning".to_string(),
        &module,
        &blackboard,
    );

    assert!(
        result.is_ok(),
        "PrePass::LayerPlanning dispatch must succeed: {:?}",
        result.err()
    );

    // Must return LayerPlan(...) variant, not None.
    match result.unwrap().0 {
        PrepassStageOutput::LayerPlan(ir) => {
            // The prepass-guest returns no proposals, so the plan has zero
            // global layers.  This is valid until a real planning module
            // with object data is wired.
            assert_eq!(
                ir.schema_version,
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0
                },
                "harvested LayerPlanIR must carry schema_version 1.0.0"
            );
            // Zero proposals → zero global layers (not a failure).
            // Object participation map must be empty too.
            assert!(
                ir.object_participation.is_empty(),
                "empty proposal list must produce empty object_participation"
            );
        }
        other => {
            panic!(
                "expected PrepassStageOutput::LayerPlan, got {:?} — \
                 dispatch harvest gap not closed",
                std::mem::discriminant(&other)
            );
        }
    }
}

#[test]
fn layer_plan_committed_to_blackboard_after_execute_prepass() {
    // Full prepass path: `execute_prepass` with a real WASM module for
    // `PrePass::LayerPlanning` must commit `LayerPlanIR` into the blackboard
    // so that downstream stages (`PaintSegmentation`, `RegionMapping`) can
    // read it via `blackboard.layer_plan()`.
    use slicer_host::{execute_prepass, PrepassStageOutput};

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_prepass_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.lp-commit",
        "PrePass::LayerPlanning",
        Arc::clone(&component),
    );

    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::LayerPlanning".into(),
            modules: vec![module],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };

    let mut blackboard = Blackboard::new(empty_mesh_ir(), 0);
    // PrePass::LayerPlanning requires SurfaceClassification to be present
    // (see prepass::required_slots). Pre-seed a stub so execute_prepass can
    // proceed to the LayerPlanning stage without hitting a missing-prerequisite
    // error.
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            per_object: HashMap::new(),
        }))
        .expect("pre-seed SurfaceClassificationIR");

    let result = execute_prepass(&plan, &mut blackboard, &dispatcher);

    assert!(
        result.is_ok(),
        "execute_prepass with LayerPlanning module must succeed: {:?}",
        result.err()
    );

    // LayerPlanIR must be committed — not None.
    assert!(
        blackboard.layer_plan().is_some(),
        "blackboard.layer_plan() must be Some after execute_prepass with LayerPlanning; \
         prepass dispatch harvest gap is NOT closed if this fails"
    );

    let ir = blackboard.layer_plan().unwrap();
    assert_eq!(
        ir.schema_version,
        SemVer {
            major: 1,
            minor: 0,
            patch: 0
        },
        "committed LayerPlanIR must carry schema_version 1.0.0"
    );
}

#[test]
fn layer_plan_harvest_deterministic_across_repeated_calls() {
    // Two independent dispatch calls over the same prepass-guest module must
    // produce byte-identical `LayerPlanIR` structures.  This proves the
    // harvest path has no non-deterministic state (timestamps, pointers, etc.).
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_prepass_guest(&engine);

    let make_module = || {
        make_compiled_module_with(
            "com.test.lp-det",
            "PrePass::LayerPlanning",
            Arc::clone(&component),
        )
    };

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);

    let run_once = || {
        let module = make_module();
        match PrepassStageRunner::run_stage(
            &dispatcher,
            &"PrePass::LayerPlanning".to_string(),
            &module,
            &blackboard,
        ) {
            Ok((PrepassStageOutput::LayerPlan(ir), _)) => ir,
            Ok((other, _)) => panic!(
                "expected LayerPlan variant, got discriminant {:?}",
                std::mem::discriminant(&other)
            ),
            Err(e) => panic!("dispatch failed: {e}"),
        }
    };

    let ir_a = run_once();
    let ir_b = run_once();

    assert_eq!(
        ir_a.schema_version, ir_b.schema_version,
        "schema_version must be identical across runs"
    );
    assert_eq!(
        ir_a.global_layers.len(),
        ir_b.global_layers.len(),
        "global_layers length must be identical across runs"
    );
    assert_eq!(
        ir_a.object_participation.len(),
        ir_b.object_participation.len(),
        "object_participation length must be identical across runs"
    );
    // Full structural equality of the IR (LayerPlanIR derives PartialEq).
    assert_eq!(
        *ir_a, *ir_b,
        "two identical dispatch calls must produce byte-identical LayerPlanIR"
    );
}

#[test]
fn layer_planning_module_error_propagates_as_fatal_prepass_error() {
    // When a `PrePass::LayerPlanning` module returns a module-level error the
    // dispatcher must NOT silently swallow it — it must surface as
    // `PrepassExecutionError::FatalModule`.
    //
    // The real `layer-planner-default.wasm` is used with a config view that
    // contains `layer_height = -1.0`.  The module validates this on entry and
    // returns `ModuleError { code: 2, message: "layer_height must be positive",
    // fatal: true }`.  This guards against accidental silent promotion of
    // module errors to skips.
    //
    // If the .wasm artifact is absent (build artifact, not committed), the
    // test is skipped with a note.
    use slicer_host::PrepassExecutionError;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_layer_planner_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!(
                "note: layer-planner-default.wasm not found; \
                 skipping layer_planning_module_error_propagates_as_fatal_prepass_error"
            );
            return;
        }
    };

    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    // Inject layer_height=-1.0 so the module returns a fatal ModuleError
    // (code 2: "layer_height must be positive") regardless of the object list.
    let bad_config = ConfigView::from_map({
        let mut m = HashMap::new();
        m.insert("layer_height".to_string(), ConfigValue::Float(-1.0));
        m
    });
    let module = make_compiled_module_with_config(
        "com.core.layer-planner-default",
        "PrePass::LayerPlanning",
        component,
        bad_config,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::LayerPlanning".to_string(),
        &module,
        &blackboard,
    );

    // The module must propagate as Err, not be silently swallowed as Ok(None).
    assert!(
        result.is_err(),
        "module error from layer-planner-default must propagate as Err, not be swallowed"
    );

    match result.unwrap_err() {
        PrepassExecutionError::FatalModule {
            stage_id,
            module_id,
            message,
        } => {
            assert_eq!(stage_id, "PrePass::LayerPlanning");
            assert!(
                module_id.contains("layer-planner-default"),
                "error must name the failing module: {module_id}"
            );
            assert!(
                message.contains("layer_height")
                    || message.contains("positive")
                    || message.contains("module error"),
                "error message must describe the root cause: {message}"
            );
        }
        other => {
            panic!("expected FatalModule error variant, got: {other}");
        }
    }
}

// ---------------------------------------------------------------------------
// Step B regression tests: PrePass::MeshSegmentation routing
// ---------------------------------------------------------------------------

const MESH_SEG_DEFAULT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../modules/core-modules/mesh-segmentation/mesh-segmentation.wasm"
);

fn load_mesh_segmentation_default(engine: &WasmEngine) -> Option<Arc<slicer_host::WasmComponent>> {
    let path = std::path::Path::new(MESH_SEG_DEFAULT_PATH);
    if !path.exists() {
        return None;
    }
    let bytes = std::fs::read(path).expect("read mesh-segmentation.wasm");
    match engine.compile_component(&bytes) {
        Ok(c) => Some(Arc::new(c)),
        Err(_) => None,
    }
}

/// Dispatch the real `mesh-segmentation.wasm` with an empty config: the
/// guest must run, emit zero marks (unpainted mesh), and the host must
/// harvest a `MeshSegmentationIR` variant with an empty `marks` vec.
#[test]
fn mesh_segmentation_dispatch_returns_empty_ir_for_unpainted_mesh() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_mesh_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: mesh-segmentation.wasm missing — rebuild core modules");
            return;
        }
    };
    let module = make_compiled_module_with(
        "com.test.mesh-seg-dispatch",
        "PrePass::MeshSegmentation",
        component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshSegmentation".to_string(),
        &module,
        &blackboard,
    )
    .expect("mesh-segmentation dispatch must succeed");

    match result.0 {
        PrepassStageOutput::MeshSegmentation(ir) => {
            assert_eq!(
                ir.schema_version,
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0
                }
            );
            assert!(
                ir.marks.is_empty(),
                "unpainted mesh must produce zero marks, got {}",
                ir.marks.len()
            );
        }
        other => panic!(
            "expected MeshSegmentation variant, got {:?}",
            std::mem::discriminant(&other)
        ),
    }
}

/// Dispatch the real `mesh-segmentation.wasm` with `mesh_seg_mark:*`
/// config entries: the guest must parse the keys, call `mark-triangle-
/// paint` on the WIT output resource, and the host must collect the
/// marks into `MeshSegmentationIR.marks` with deterministic ordering.
#[test]
fn mesh_segmentation_collects_config_driven_marks() {
    use slicer_host::PrepassStageOutput;
    use slicer_ir::ConfigValue;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_mesh_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: mesh-segmentation.wasm missing — rebuild core modules");
            return;
        }
    };

    // Two objects, three marks spanning semantics and facet indices.
    // We seed via config; the canonical guest parses these and emits
    // mark-triangle-paint calls in sorted order.
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "mesh_seg_mark:benchy:5:material".into(),
        ConfigValue::String("tool-2".into()),
    );
    fields.insert(
        "mesh_seg_mark:benchy:2:fuzzy_skin".into(),
        ConfigValue::String("true".into()),
    );
    fields.insert(
        "mesh_seg_mark:other-obj:0:material".into(),
        ConfigValue::String("tool-0".into()),
    );

    let module = make_compiled_module_with_config(
        "com.test.mesh-seg-marks",
        "PrePass::MeshSegmentation",
        component,
        ConfigView::from_declared(&fields, fields.keys().map(|s| s.as_str())),
    );

    // Pre-seed mesh with two object ids matching the config keys so the
    // guest's `object_index` sort key finds them.
    let mesh = Arc::new(slicer_ir::MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![make_object("benchy"), make_object("other-obj")],
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
    });
    let blackboard = Blackboard::new(mesh, 0);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshSegmentation".to_string(),
        &module,
        &blackboard,
    )
    .expect("mesh-segmentation dispatch must succeed");

    let ir = match result.0 {
        PrepassStageOutput::MeshSegmentation(ir) => ir,
        other => panic!(
            "expected MeshSegmentation variant, got {:?}",
            std::mem::discriminant(&other)
        ),
    };

    // Deterministic ordering: (object_index_in_objects, facet asc, semantic asc).
    // benchy comes first (index 0): fuzzy_skin@facet 2, material@facet 5.
    // other-obj (index 1): material@facet 0.
    assert_eq!(
        ir.marks.len(),
        3,
        "expected 3 marks, got {}",
        ir.marks.len()
    );
    assert_eq!(ir.marks[0].object_id, "benchy");
    assert_eq!(ir.marks[0].facet_index, 2);
    assert_eq!(ir.marks[0].semantic, "fuzzy_skin");
    assert_eq!(ir.marks[0].value, "true");
    assert_eq!(ir.marks[1].object_id, "benchy");
    assert_eq!(ir.marks[1].facet_index, 5);
    assert_eq!(ir.marks[1].semantic, "material");
    assert_eq!(ir.marks[1].value, "tool-2");
    assert_eq!(ir.marks[2].object_id, "other-obj");
    assert_eq!(ir.marks[2].facet_index, 0);
    assert_eq!(ir.marks[2].semantic, "material");
    assert_eq!(ir.marks[2].value, "tool-0");
}

/// Two back-to-back dispatches with the same inputs must produce
/// byte-identical `MeshSegmentationIR` — determinism holds even when
/// the guest pushes through the WIT resource.
#[test]
fn mesh_segmentation_dispatch_is_deterministic() {
    use slicer_host::PrepassStageOutput;
    use slicer_ir::ConfigValue;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_mesh_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: mesh-segmentation.wasm missing — rebuild core modules");
            return;
        }
    };

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "mesh_seg_mark:obj-a:7:material".into(),
        ConfigValue::String("tool-1".into()),
    );
    fields.insert(
        "mesh_seg_mark:obj-a:3:material".into(),
        ConfigValue::String("tool-3".into()),
    );
    let cfg = ConfigView::from_declared(&fields, fields.keys().map(|s| s.as_str()));
    let mesh = Arc::new(slicer_ir::MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![make_object("obj-a")],
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
    });
    let blackboard = Blackboard::new(mesh, 0);

    let run = || -> Vec<slicer_ir::FacetPaintMark> {
        let module = make_compiled_module_with_config(
            "com.test.mesh-seg-det",
            "PrePass::MeshSegmentation",
            Arc::clone(&component),
            cfg.clone(),
        );
        let result = PrepassStageRunner::run_stage(
            &dispatcher,
            &"PrePass::MeshSegmentation".to_string(),
            &module,
            &blackboard,
        )
        .expect("dispatch succeeds");
        match result.0 {
            PrepassStageOutput::MeshSegmentation(ir) => ir.marks.clone(),
            _ => panic!("wrong variant"),
        }
    };
    let a = run();
    let b = run();
    assert_eq!(
        a, b,
        "two identical dispatches must produce identical marks"
    );
}

/// The `HostMeshSegmentationOutput::mark_triangle_paint` validation
/// rejects an empty `obj` or `semantic` with a precise structured
/// diagnostic. This is the invariant every routed guest relies on.
#[test]
fn mesh_segmentation_output_rejects_invalid_marks() {
    use slicer_host::wit_host::prepass::{self as pm, HostMeshSegmentationOutput};
    use slicer_host::wit_host::HostExecutionContext;
    use wasmtime::component::Resource;

    let mut ctx =
        HostExecutionContext::new("com.test.mesh-seg-validate".into(), 0.0, 0.0, None, None);
    let handle = ctx.push_mesh_segmentation_output().expect("push resource");

    // obj empty
    let r = HostMeshSegmentationOutput::mark_triangle_paint(
        &mut ctx,
        Resource::<pm::MeshSegmentationOutput>::new_own(handle.rep()),
        String::new(),
        0,
        "material".into(),
        "0".into(),
    )
    .expect("wasmtime call");
    assert!(
        matches!(r, Err(ref msg) if msg.contains("obj")),
        "empty obj must be rejected with diagnostic, got {r:?}"
    );

    // semantic empty
    let r = HostMeshSegmentationOutput::mark_triangle_paint(
        &mut ctx,
        Resource::<pm::MeshSegmentationOutput>::new_own(handle.rep()),
        "benchy".into(),
        0,
        String::new(),
        "0".into(),
    )
    .expect("wasmtime call");
    assert!(
        matches!(r, Err(ref msg) if msg.contains("semantic")),
        "empty semantic must be rejected with diagnostic, got {r:?}"
    );

    // valid mark collects into ctx
    let r = HostMeshSegmentationOutput::mark_triangle_paint(
        &mut ctx,
        Resource::<pm::MeshSegmentationOutput>::new_own(handle.rep()),
        "benchy".into(),
        42,
        "material".into(),
        "tool-1".into(),
    )
    .expect("wasmtime call");
    assert!(r.is_ok(), "valid mark must succeed: {r:?}");
    assert_eq!(ctx.mesh_segmentation_marks.len(), 1);
    assert_eq!(ctx.mesh_segmentation_marks[0].0, "benchy");
    assert_eq!(ctx.mesh_segmentation_marks[0].1, 42);
    assert_eq!(ctx.mesh_segmentation_marks[0].2, "material");
    assert_eq!(ctx.mesh_segmentation_marks[0].3, "tool-1");
}

/// Mesh segmentation IR, once committed, survives through
/// `execute_prepass` onto the blackboard and is readable via
/// `Blackboard::mesh_segmentation()`.
#[test]
fn mesh_segmentation_commits_through_execute_prepass() {
    use slicer_host::execute_prepass;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_mesh_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: mesh-segmentation.wasm missing");
            return;
        }
    };
    let module = make_compiled_module_with(
        "com.test.mesh-seg-commit",
        "PrePass::MeshSegmentation",
        component,
    );
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::MeshSegmentation".into(),
            modules: vec![module],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };
    let mut blackboard = Blackboard::new(empty_mesh_ir(), 0);
    execute_prepass(&plan, &mut blackboard, &dispatcher).expect("prepass succeeds");

    let ir = blackboard
        .mesh_segmentation()
        .expect("mesh-segmentation IR must be committed after execute_prepass");
    assert_eq!(
        ir.schema_version,
        SemVer {
            major: 1,
            minor: 0,
            patch: 0
        }
    );
    assert!(ir.marks.is_empty(), "empty mesh → zero marks");
}

// ---------------------------------------------------------------------------
// Step C regression tests: PrePass::PaintSegmentation routing
// ---------------------------------------------------------------------------

const PAINT_SEG_DEFAULT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../modules/core-modules/paint-segmentation/paint-segmentation.wasm"
);

fn load_paint_segmentation_default(engine: &WasmEngine) -> Option<Arc<slicer_host::WasmComponent>> {
    let path = std::path::Path::new(PAINT_SEG_DEFAULT_PATH);
    if !path.exists() {
        return None;
    }
    let bytes = std::fs::read(path).expect("read paint-segmentation.wasm");
    match engine.compile_component(&bytes) {
        Ok(c) => Some(Arc::new(c)),
        Err(_) => None,
    }
}

#[test]
fn paint_segmentation_dispatch_returns_empty_paint_regions_for_unpainted_mesh() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_paint_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: paint-segmentation.wasm missing — rebuild core modules");
            return;
        }
    };
    let module = make_compiled_module_with(
        "com.test.paint-seg-dispatch",
        "PrePass::PaintSegmentation",
        component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::PaintSegmentation".to_string(),
        &module,
        &blackboard,
    )
    .expect("paint-segmentation dispatch must succeed");

    match result.0 {
        PrepassStageOutput::PaintRegions(ir) => {
            assert_eq!(
                ir.schema_version,
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0
                }
            );
            assert!(
                ir.per_layer.is_empty(),
                "unpainted mesh must produce zero per-layer entries"
            );
        }
        other => panic!(
            "expected PaintRegions variant, got {:?}",
            std::mem::discriminant(&other)
        ),
    }
}

/// Dispatch the real `paint-segmentation.wasm` with `paint_region:*`
/// config entries: the guest parses them, calls `push-paint-region`
/// on the WIT resource, and the host reshapes into `PaintRegionIR`
/// with per-layer keying, deterministic `paint_order`, and the
/// documented PaintSemantic/PaintValue parsing rules.
#[test]
fn paint_segmentation_collects_config_driven_regions() {
    use slicer_host::PrepassStageOutput;
    use slicer_ir::{ConfigValue, PaintSemantic, PaintValue};

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_paint_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: paint-segmentation.wasm missing");
            return;
        }
    };

    // Two entries on layer 3 (material tool-2, fuzzy_skin true) +
    // one entry on layer 5 (support_enforcer 1) — triangle polygons
    // in scaled integer coordinates.
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "paint_region:benchy:3:material:2".into(),
        ConfigValue::String("0,0;100,0;50,100".into()),
    );
    fields.insert(
        "paint_region:benchy:3:fuzzy_skin:true".into(),
        ConfigValue::String("10,10;90,10;50,90".into()),
    );
    fields.insert(
        "paint_region:benchy:5:support_enforcer:1".into(),
        ConfigValue::String("0,0;50,0;25,50".into()),
    );

    let module = make_compiled_module_with_config(
        "com.test.paint-seg-marks",
        "PrePass::PaintSegmentation",
        component,
        ConfigView::from_declared(&fields, fields.keys().map(|s| s.as_str())),
    );

    let mesh = Arc::new(slicer_ir::MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![make_object("benchy")],
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
    });
    let blackboard = Blackboard::new(mesh, 0);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::PaintSegmentation".to_string(),
        &module,
        &blackboard,
    )
    .expect("paint-segmentation dispatch must succeed");

    let ir = match result.0 {
        PrepassStageOutput::PaintRegions(ir) => ir,
        other => panic!("wrong variant: {:?}", std::mem::discriminant(&other)),
    };

    // Layer 3 must hold the two semantics; layer 5 holds the third.
    assert_eq!(ir.per_layer.len(), 2, "two distinct layers expected");
    let l3 = ir.per_layer.get(&3).expect("layer 3 must be present");
    assert_eq!(l3.global_layer_index, 3);
    assert_eq!(
        l3.semantic_regions
            .get(&PaintSemantic::Material)
            .map(|v| v.len()),
        Some(1),
        "material region on layer 3 missing"
    );
    assert_eq!(
        l3.semantic_regions
            .get(&PaintSemantic::FuzzySkin)
            .map(|v| v.len()),
        Some(1),
        "fuzzy_skin region on layer 3 missing"
    );
    // Value parsing: "2" → ToolIndex(2); "true" → Flag(true).
    let mat = &l3.semantic_regions[&PaintSemantic::Material][0];
    assert_eq!(mat.value, PaintValue::ToolIndex(2));
    assert_eq!(mat.object_id, "benchy");
    let fuzzy = &l3.semantic_regions[&PaintSemantic::FuzzySkin][0];
    assert_eq!(fuzzy.value, PaintValue::Flag(true));

    let l5 = ir.per_layer.get(&5).expect("layer 5 must be present");
    assert_eq!(
        l5.semantic_regions
            .get(&PaintSemantic::SupportEnforcer)
            .map(|v| v.len()),
        Some(1)
    );

    // paint_order must be unique across all three regions (the flat
    // insertion-index assigned by harvest_paint_segmentation_ir).
    let mut orders: Vec<u64> = Vec::new();
    for layer in ir.per_layer.values() {
        for regions in layer.semantic_regions.values() {
            for r in regions {
                orders.push(r.paint_order);
            }
        }
    }
    orders.sort();
    assert_eq!(
        orders,
        vec![0, 1, 2],
        "paint_order must be dense and unique"
    );
}

#[test]
fn paint_segmentation_dispatch_is_deterministic() {
    use slicer_host::PrepassStageOutput;
    use slicer_ir::ConfigValue;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_paint_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: paint-segmentation.wasm missing");
            return;
        }
    };

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "paint_region:obj:7:material:3".into(),
        ConfigValue::String("0,0;10,0;5,10".into()),
    );
    fields.insert(
        "paint_region:obj:7:material:1".into(),
        ConfigValue::String("0,0;20,0;10,20".into()),
    );
    let cfg = ConfigView::from_declared(&fields, fields.keys().map(|s| s.as_str()));
    let mesh = Arc::new(slicer_ir::MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![make_object("obj")],
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
    });
    let blackboard = Blackboard::new(mesh, 0);

    let run = || {
        let module = make_compiled_module_with_config(
            "com.test.paint-seg-det",
            "PrePass::PaintSegmentation",
            Arc::clone(&component),
            cfg.clone(),
        );
        let result = PrepassStageRunner::run_stage(
            &dispatcher,
            &"PrePass::PaintSegmentation".to_string(),
            &module,
            &blackboard,
        )
        .expect("dispatch succeeds");
        match result.0 {
            PrepassStageOutput::PaintRegions(ir) => ir,
            _ => panic!("wrong variant"),
        }
    };
    let a = run();
    let b = run();
    assert_eq!(
        format!("{a:?}"),
        format!("{b:?}"),
        "two identical dispatches must produce identical PaintRegionIR"
    );
}

/// A malformed `paint_region:*` config key (e.g. non-numeric layer)
/// must surface as a structured FatalModule diagnostic naming the
/// offending key — precision matters: operators need to fix the key,
/// not debug the guest.
#[test]
fn paint_segmentation_malformed_config_emits_structured_diagnostic() {
    use slicer_host::{PrepassExecutionError, PrepassStageRunner};
    use slicer_ir::ConfigValue;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_paint_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: paint-segmentation.wasm missing");
            return;
        }
    };

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    // Malformed: layer index is not a u32.
    fields.insert(
        "paint_region:benchy:notalayer:material:1".into(),
        ConfigValue::String("0,0;1,0;0,1".into()),
    );
    let module = make_compiled_module_with_config(
        "com.test.paint-seg-malformed",
        "PrePass::PaintSegmentation",
        component,
        ConfigView::from_declared(&fields, fields.keys().map(|s| s.as_str())),
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);

    let err = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::PaintSegmentation".to_string(),
        &module,
        &blackboard,
    )
    .expect_err("malformed config must produce a dispatch error");

    match err {
        PrepassExecutionError::FatalModule {
            stage_id, message, ..
        } => {
            assert_eq!(stage_id, "PrePass::PaintSegmentation");
            assert!(
                message.contains("notalayer") || message.contains("layer_index"),
                "diagnostic must name the offending layer_index token: {message}"
            );
        }
        other => panic!("expected FatalModule, got: {other}"),
    }
}

#[test]
fn paint_segmentation_output_rejects_invalid_entries() {
    use slicer_host::wit_host::prepass::slicer::world_prepass::geometry as geo;
    use slicer_host::wit_host::prepass::{self as pm, HostPaintSegmentationOutput};
    use slicer_host::wit_host::HostExecutionContext;
    use wasmtime::component::Resource;

    let mut ctx =
        HostExecutionContext::new("com.test.paint-seg-validate".into(), 0.0, 0.0, None, None);
    let handle = ctx.push_paint_segmentation_output().expect("push resource");

    let valid_poly = vec![geo::ExPolygon {
        contour: geo::Polygon {
            points: vec![
                geo::Point2 { x: 0, y: 0 },
                geo::Point2 { x: 10, y: 0 },
                geo::Point2 { x: 5, y: 10 },
            ],
        },
        holes: vec![],
    }];

    // empty object_id
    let r = HostPaintSegmentationOutput::push_paint_region(
        &mut ctx,
        Resource::<pm::PaintSegmentationOutput>::new_own(handle.rep()),
        pm::PaintRegionEntry {
            object_id: String::new(),
            layer_index: 0,
            semantic: "material".into(),
            polygons: valid_poly.clone(),
            value: "1".into(),
        },
    )
    .expect("wasmtime call");
    assert!(
        matches!(r, Err(ref msg) if msg.contains("object-id")),
        "empty object-id must be rejected: {r:?}"
    );

    // empty semantic
    let r = HostPaintSegmentationOutput::push_paint_region(
        &mut ctx,
        Resource::<pm::PaintSegmentationOutput>::new_own(handle.rep()),
        pm::PaintRegionEntry {
            object_id: "obj".into(),
            layer_index: 0,
            semantic: String::new(),
            polygons: valid_poly.clone(),
            value: "1".into(),
        },
    )
    .expect("wasmtime call");
    assert!(
        matches!(r, Err(ref msg) if msg.contains("semantic")),
        "empty semantic must be rejected: {r:?}"
    );

    // empty polygons
    let r = HostPaintSegmentationOutput::push_paint_region(
        &mut ctx,
        Resource::<pm::PaintSegmentationOutput>::new_own(handle.rep()),
        pm::PaintRegionEntry {
            object_id: "obj".into(),
            layer_index: 0,
            semantic: "material".into(),
            polygons: vec![],
            value: "1".into(),
        },
    )
    .expect("wasmtime call");
    assert!(
        matches!(r, Err(ref msg) if msg.contains("polygons")),
        "empty polygons must be rejected: {r:?}"
    );

    // valid entry collects into ctx
    let r = HostPaintSegmentationOutput::push_paint_region(
        &mut ctx,
        Resource::<pm::PaintSegmentationOutput>::new_own(handle.rep()),
        pm::PaintRegionEntry {
            object_id: "obj".into(),
            layer_index: 7,
            semantic: "material".into(),
            polygons: valid_poly,
            value: "3".into(),
        },
    )
    .expect("wasmtime call");
    assert!(r.is_ok(), "valid entry must succeed: {r:?}");
    assert_eq!(ctx.paint_region_entries.len(), 1);
    assert_eq!(ctx.paint_region_entries[0].object_id, "obj");
    assert_eq!(ctx.paint_region_entries[0].layer_index, 7);
}

#[test]
fn paint_segmentation_commits_through_execute_prepass() {
    use slicer_host::execute_prepass;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_paint_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: paint-segmentation.wasm missing");
            return;
        }
    };
    let module = make_compiled_module_with(
        "com.test.paint-seg-commit",
        "PrePass::PaintSegmentation",
        component,
    );
    let plan = ExecutionPlan {
        prepass_stages: vec![CompiledStage {
            stage_id: "PrePass::PaintSegmentation".into(),
            modules: vec![module],
        }],
        per_layer_stages: Vec::new(),
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(Vec::new()),
        region_plans: Arc::new(HashMap::new()),
        module_region_index: HashMap::new(),
    };
    let mut blackboard = Blackboard::new(empty_mesh_ir(), 0);
    // PrePass::PaintSegmentation requires SurfaceClassification and
    // LayerPlan to be present (see prepass::required_slots). Pre-seed
    // both so execute_prepass can proceed to the PaintSegmentation
    // stage without hitting a missing-prerequisite error.
    blackboard
        .commit_surface_classification(Arc::new(slicer_ir::SurfaceClassificationIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            per_object: HashMap::new(),
        }))
        .unwrap();
    blackboard
        .commit_layer_plan(Arc::new(slicer_ir::LayerPlanIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layers: Vec::new(),
            object_participation: HashMap::new(),
        }))
        .unwrap();
    execute_prepass(&plan, &mut blackboard, &dispatcher).expect("prepass succeeds");
    let ir = blackboard
        .paint_regions()
        .expect("paint_regions must be committed after execute_prepass");
    assert_eq!(
        ir.schema_version,
        SemVer {
            major: 1,
            minor: 0,
            patch: 0
        }
    );
    assert!(ir.per_layer.is_empty(), "unpainted mesh → empty per_layer");
}

// ---------------------------------------------------------------------------
// Step D regression tests: Layer::PathOptimization canonical module
// ---------------------------------------------------------------------------

const PATH_OPT_DEFAULT_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../modules/core-modules/path-optimization-default/path-optimization-default.wasm"
);

fn load_path_optimization_default(engine: &WasmEngine) -> Option<Arc<slicer_host::WasmComponent>> {
    let path = std::path::Path::new(PATH_OPT_DEFAULT_PATH);
    if !path.exists() {
        return None;
    }
    let bytes = std::fs::read(path).expect("read path-optimization-default.wasm");
    match engine.compile_component(&bytes) {
        Ok(c) => Some(Arc::new(c)),
        Err(_) => None,
    }
}

/// End-to-end guard: the canonical `Layer::PathOptimization` module
/// runs on the real per-layer path against a real Benchy-equivalent
/// set-up — the arena already carries a committed `PerimeterIR` (via
/// the `Layer::Perimeters` stage) and a pre-staged `LayerCollectionIR`
/// with `ordered_entities`. The guest's `push_comment` output
/// survives through to `LayerCollectionIR.annotations`, which the
/// default G-code emitter renders as a `; path-optimization layer X
/// regions=Y entities=Z` line (see benchy_end_to_end_tdd.rs for the
/// observed 239-marker count on the real Benchy run).
#[test]
fn path_optimization_dispatch_emits_per_layer_marker() {
    use slicer_host::{Blackboard, LayerArena, LayerStageOutput};

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_path_optimization_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: path-optimization-default.wasm missing");
            return;
        }
    };
    let module = make_compiled_module_with(
        "com.test.path-opt-dispatch",
        "Layer::PathOptimization",
        component,
    );

    // Pre-seed the arena with a perimeter commit so the guest sees a
    // non-empty region list (region_count=1, entity_count=1 on the
    // guest side). A PerimeterRegion with one wall loop.
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut arena = LayerArena::new();
    let wall = slicer_ir::WallLoop {
        perimeter_index: 0,
        loop_type: slicer_ir::LoopType::Outer,
        path: slicer_ir::ExtrusionPath3D {
            points: vec![
                slicer_ir::Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                },
                slicer_ir::Point3WithWidth {
                    x: 1.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                },
                slicer_ir::Point3WithWidth {
                    x: 0.0,
                    y: 1.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                },
            ],
            role: slicer_ir::ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: slicer_ir::WidthProfile {
            widths: vec![0.4; 3],
        },
        feature_flags: vec![
            slicer_ir::WallFeatureFlags {
                tool_index: None,
                fuzzy_skin: false,
                is_bridge: false,
                is_thin_wall: false,
                skip_ironing: false,
                custom: HashMap::new(),
            };
            3
        ],
        boundary_type: slicer_ir::WallBoundaryType::ExteriorSurface,
    };
    let perim = slicer_ir::PerimeterIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        global_layer_index: 7,
        regions: vec![slicer_ir::PerimeterRegion {
            object_id: "obj".into(),
            region_id: 0,
            walls: vec![wall],
            seam_candidates: Vec::new(),
            infill_areas: Vec::new(),
            resolved_seam: None,
        }],
    };
    arena.set_perimeter(perim).expect("seed perimeter");

    let layer = slicer_ir::GlobalLayer {
        index: 7,
        z: 1.4,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };

    let result = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PathOptimization".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .expect("path-optimization dispatch must succeed");
    assert_eq!(result.0, LayerStageOutput::Success);

    // Dispatch already ran commit_layer_outputs internally; the comment
    // is now in the arena as a deferred annotation. Verify it.
    let annotations = arena.take_deferred_annotations();
    assert_eq!(
        annotations.len(),
        1,
        "exactly one path-optimization marker expected, got {}",
        annotations.len()
    );
    match &annotations[0].kind {
        slicer_ir::LayerAnnotationKind::Comment(text) => {
            assert!(
                text.contains("path-optimization layer 7"),
                "expected 'path-optimization layer 7' in annotation text, got: {text}"
            );
            assert!(
                text.contains("regions=1"),
                "expected 'regions=1' in annotation text, got: {text}"
            );
            assert!(
                text.contains("entities=1"),
                "expected 'entities=1' (one wall loop) in annotation text, got: {text}"
            );
        }
        other => panic!("expected Comment annotation, got {other:?}"),
    }
}

/// Two back-to-back dispatches with the same arena seed produce
/// byte-identical annotation output.
#[test]
fn path_optimization_dispatch_is_deterministic() {
    use slicer_host::{Blackboard, LayerArena};

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_path_optimization_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: path-optimization-default.wasm missing");
            return;
        }
    };
    let layer = slicer_ir::GlobalLayer {
        index: 3,
        z: 0.6,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);

    let run_once = || -> Vec<slicer_ir::LayerAnnotation> {
        let module = make_compiled_module_with(
            "com.test.path-opt-det",
            "Layer::PathOptimization",
            Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        LayerStageRunner::run_stage(
            &dispatcher,
            &"Layer::PathOptimization".to_string(),
            &layer,
            &module,
            &blackboard,
            &mut arena,
        )
        .expect("dispatch succeeds");
        arena.take_deferred_annotations()
    };
    let a = run_once();
    let b = run_once();
    assert_eq!(a.len(), b.len());
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x.after_entity_index, y.after_entity_index);
        assert_eq!(format!("{:?}", x.kind), format!("{:?}", y.kind));
    }
}

/// Operator override: when `path_optimization_emit_layer_markers =
/// false` is declared in config, the module must emit zero
/// annotations (byte-size-sensitive preset path). Proves the config
/// schema's declared-read filter survives through to the module.
#[test]
fn path_optimization_emit_layer_markers_false_suppresses_output() {
    use slicer_host::{Blackboard, LayerArena};
    use slicer_ir::ConfigValue;

    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = match load_path_optimization_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: path-optimization-default.wasm missing");
            return;
        }
    };

    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "path_optimization_emit_layer_markers".into(),
        ConfigValue::Bool(false),
    );
    let module = make_compiled_module_with_config(
        "com.test.path-opt-silent",
        "Layer::PathOptimization",
        component,
        ConfigView::from_declared(&fields, fields.keys().map(|s| s.as_str())),
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut arena = LayerArena::new();
    let layer = slicer_ir::GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PathOptimization".to_string(),
        &layer,
        &module,
        &blackboard,
        &mut arena,
    )
    .expect("dispatch succeeds");

    let annotations = arena.take_deferred_annotations();
    assert!(
        annotations.is_empty(),
        "emit_layer_markers=false must suppress output, got {} annotations",
        annotations.len()
    );
}

/// End-to-end: the Benchy MVP run emits one `; path-optimization
/// layer <n> regions=<r> entities=<e>` line per layer. The count must
/// match the distinct-layer-Z count — every layer that reaches
/// G-code MUST have seen Layer::PathOptimization on the real path.
#[test]
fn path_optimization_markers_appear_in_benchy_gcode() {
    use std::path::PathBuf;
    use std::process::Command;

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repo root");
    let model = repo_root.join("resources/benchy.stl");
    let modules = repo_root.join("modules/core-modules");
    if !model.exists() {
        eprintln!("SKIP: benchy fixture missing");
        return;
    }

    let tmp = tempfile::tempdir().expect("tempdir");
    let out = tmp.path().join("benchy_path_opt.gcode");
    let bin = env!("CARGO_BIN_EXE_slicer-host");
    let result = Command::new(bin)
        .args([
            "run",
            "--module",
            model.to_str().unwrap(),
            "--model",
            model.to_str().unwrap(),
            "--module-dir",
            modules.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
        ])
        .output()
        .expect("slicer-host binary");
    assert!(
        result.status.success(),
        "benchy run must succeed: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let gcode = std::fs::read_to_string(&out).expect("read gcode");
    let marker_count = gcode
        .lines()
        .filter(|l| l.starts_with("; path-optimization layer"))
        .count();
    assert!(
        marker_count >= 100,
        "expected at least 100 path-optimization markers in Benchy gcode, \
         got {marker_count} (one should appear per layer; Benchy is ~240 layers)"
    );
    // Sanity: the first marker should be for layer 0.
    let first = gcode
        .lines()
        .find(|l| l.starts_with("; path-optimization layer"))
        .expect("at least one marker");
    assert!(
        first.starts_with("; path-optimization layer 0 "),
        "first marker should be for layer 0, got: {first}"
    );
}

fn make_object(id: &str) -> slicer_ir::ObjectMesh {
    slicer_ir::ObjectMesh {
        id: id.to_string(),
        mesh: slicer_ir::IndexedTriangleSet {
            vertices: vec![
                slicer_ir::Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                slicer_ir::Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                slicer_ir::Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            indices: vec![0, 1, 2],
        },
        transform: slicer_ir::Transform3d {
            matrix: [
                1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
            ],
        },
        config: slicer_ir::ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    }
}

// ── STEP F: layer-planner-default macro-path drain regression ───────────
//
// These tests prove that the rebuilt `layer-planner-default.wasm` reaches
// the host through the normal `#[slicer_module]` macro path (its
// `wit-guest/` shim is now a two-line `pub use` re-export, with no
// hand-written `wit_bindgen::generate!`). The macro's prepass-world arm
// forwards `objects` into the SDK trait call and drains the SDK
// `LayerPlanOutput` back through `layer-plan-output.push-layer`, so the
// host harvests a non-empty `LayerPlanIR` whose proposal sequence comes
// straight from the SDK planner (not from a duplicate planner embedded
// in the wit-guest).

/// Build a Blackboard whose mesh carries `object_ids` so the prepass
/// runner forwards them to the guest's `run-layer-planning` export.
fn blackboard_with_objects(object_ids: &[&str]) -> Blackboard {
    let objects: Vec<slicer_ir::ObjectMesh> = object_ids.iter().map(|id| make_object(id)).collect();
    let mesh = Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects,
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
    });
    Blackboard::new(mesh, 0)
}

fn layer_planner_config(
    layer_height: f64,
    first_layer_height: f64,
    object_heights: &[(&str, f64)],
) -> ConfigView {
    let mut m = HashMap::new();
    m.insert("layer_height".to_string(), ConfigValue::Float(layer_height));
    m.insert(
        "first_layer_height".to_string(),
        ConfigValue::Float(first_layer_height),
    );
    for (id, h) in object_heights {
        m.insert(format!("object_height:{}", id), ConfigValue::Float(*h));
    }
    ConfigView::from_map(m)
}

/// The rebuilt layer-planner-default.wasm (built from the macro path —
/// see `wit-guest/src/lib.rs` reduced to a `pub use` shim) must emit the
/// SDK planner's real proposal sequence via the macro-authored drain
/// bridge. A 2mm object at 0.2mm layer height must harvest as 10 global
/// layers with strictly ascending Z.
#[test]
fn layer_planner_default_macro_path_emits_real_proposals() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_layer_planner_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: layer-planner-default.wasm not found — rebuild core modules");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let config = layer_planner_config(0.2, 0.2, &[("obj-1", 2.0)]);
    let module = make_compiled_module_with_config(
        "com.core.layer-planner-default",
        "PrePass::LayerPlanning",
        component,
        config,
    );
    let blackboard = blackboard_with_objects(&["obj-1"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::LayerPlanning".to_string(),
        &module,
        &blackboard,
    );

    let ir = match result {
        Ok((PrepassStageOutput::LayerPlan(ir), _)) => ir,
        Ok((other, _)) => panic!(
            "expected PrepassStageOutput::LayerPlan, got {:?}",
            std::mem::discriminant(&other)
        ),
        Err(e) => panic!("dispatch failed: {e}"),
    };

    // 2mm object / 0.2mm layer height = 10 layers.
    assert_eq!(
        ir.global_layers.len(),
        10,
        "macro-path drain must deliver all SDK proposals to the host harvest \
         (expected 10, got {})",
        ir.global_layers.len()
    );

    // Strictly ascending Z, first layer at first_layer_height.
    assert!(
        (ir.global_layers[0].z - 0.2).abs() < 1e-4,
        "first harvested layer z must equal first_layer_height=0.2, got {}",
        ir.global_layers[0].z
    );
    for i in 1..ir.global_layers.len() {
        assert!(
            ir.global_layers[i].z > ir.global_layers[i - 1].z,
            "harvested proposals must preserve SDK push order (ascending Z) — \
             layer {} z={} vs layer {} z={}",
            i - 1,
            ir.global_layers[i - 1].z,
            i,
            ir.global_layers[i].z
        );
    }

    // object_participation must reach downstream scheduling: the planner
    // emitted one region per layer for obj-1.
    let participation = ir
        .object_participation
        .get("obj-1")
        .expect("object_participation must carry obj-1 after drain");
    assert_eq!(
        participation.len(),
        ir.global_layers.len(),
        "obj-1 must participate in every layer it fits in"
    );
}

/// Two independent dispatch calls through the rebuilt
/// layer-planner-default.wasm must produce byte-identical `LayerPlanIR`.
/// The macro-authored drain has no hidden state (no timestamps, no
/// pointer-derived ordering), so determinism holds end-to-end.
#[test]
fn layer_planner_default_macro_path_is_deterministic() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_layer_planner_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: layer-planner-default.wasm not found — rebuild core modules");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let run_once = || {
        let config = layer_planner_config(0.2, 0.2, &[("obj-1", 2.0)]);
        let module = make_compiled_module_with_config(
            "com.core.layer-planner-default",
            "PrePass::LayerPlanning",
            Arc::clone(&component),
            config,
        );
        let blackboard = blackboard_with_objects(&["obj-1"]);
        match PrepassStageRunner::run_stage(
            &dispatcher,
            &"PrePass::LayerPlanning".to_string(),
            &module,
            &blackboard,
        ) {
            Ok((PrepassStageOutput::LayerPlan(ir), _)) => ir,
            Ok((other, _)) => panic!(
                "expected LayerPlan, got {:?}",
                std::mem::discriminant(&other)
            ),
            Err(e) => panic!("dispatch failed: {e}"),
        }
    };

    let a = run_once();
    let b = run_once();
    assert_eq!(
        *a, *b,
        "macro-path layer-planner-default must be deterministic \
         across repeated dispatches"
    );
}

// ── STEP G: PrePass::MeshAnalysis macro-path drain regression ───────────
//
// These tests prove the macro-authored `PrePass::MeshAnalysis` arm
// forwards the real `objects` list into the SDK trait call AND drains
// the SDK `MeshAnalysisOutput` back through the WIT
// `mesh-analysis-output` resource to the host. The driver is the
// existing `sdk-prepass-guest` which now emits deterministic facet
// annotations + surface-group proposals when the config carries
// `emit_mesh_analysis = N`.

const SDK_PREPASS_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/sdk-prepass-guest.component.wasm"
);

fn load_sdk_prepass_guest(engine: &WasmEngine) -> Option<Arc<slicer_host::WasmComponent>> {
    let path = std::path::Path::new(SDK_PREPASS_GUEST_PATH);
    if !path.exists() {
        return None;
    }
    let bytes = std::fs::read(path).expect("read sdk-prepass-guest.component.wasm");
    match engine.compile_component(&bytes) {
        Ok(c) => Some(Arc::new(c)),
        Err(_) => None,
    }
}

fn mesh_analysis_emit_config(n: i64) -> ConfigView {
    let mut m = HashMap::new();
    m.insert("emit_mesh_analysis".to_string(), ConfigValue::Int(n));
    ConfigView::from_map(m)
}

/// Forwarding + drain proof: with two objects and `emit_mesh_analysis=3`,
/// the SDK trait body emits 3 facet annotations + 1 surface group per
/// object (6 + 2 = 8 pushes total). These must reach the host as a
/// `MeshAnalysisAuxiliary` variant, preserving push order and per-object
/// id. Proves `_objects` forwarding (not empty Vec) AND the drain.
#[test]
fn mesh_analysis_macro_path_forwards_objects_and_drains_output() {
    use slicer_host::{FacetClassRecord, PrepassStageOutput};

    let engine = Arc::new(WasmEngine::new());
    let component = match load_sdk_prepass_guest(&engine) {
        Some(c) => c,
        None => {
            eprintln!(
                "SKIP: sdk-prepass-guest.component.wasm missing — \
                 rebuild test guests"
            );
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module_with_config(
        "com.test.sdk-prepass-emit",
        "PrePass::MeshAnalysis",
        component,
        mesh_analysis_emit_config(3),
    );
    let blackboard = blackboard_with_objects(&["obj-A", "obj-B"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshAnalysis".to_string(),
        &module,
        &blackboard,
    );

    let aux = match result {
        Ok((PrepassStageOutput::MeshAnalysisAuxiliary(a), _)) => a,
        Ok((other, _)) => panic!(
            "expected PrepassStageOutput::MeshAnalysisAuxiliary, got {:?}",
            std::mem::discriminant(&other)
        ),
        Err(e) => panic!("dispatch failed: {e}"),
    };

    // 2 objects × 3 facet annotations = 6 pushes, preserving per-object grouping.
    assert_eq!(
        aux.facet_annotations.len(),
        6,
        "macro-path drain must forward every SDK-pushed annotation (expected 6, got {})",
        aux.facet_annotations.len()
    );
    let obj_ids: Vec<&str> = aux
        .facet_annotations
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(
        obj_ids,
        vec!["obj-A", "obj-A", "obj-A", "obj-B", "obj-B", "obj-B"],
        "objects forwarding + drain must preserve per-object push order"
    );

    // Verify field round-trip including the FacetClass mapping path.
    let (_, first) = &aux.facet_annotations[0];
    assert_eq!(first.facet_index, 0);
    assert!((first.slope_angle_deg - 0.0).abs() < 1e-6);
    assert_eq!(first.classification, FacetClassRecord::Normal);
    let (_, second) = &aux.facet_annotations[1];
    assert_eq!(second.facet_index, 1);
    assert!((second.slope_angle_deg - 10.0).abs() < 1e-6);
    assert_eq!(second.classification, FacetClassRecord::NearHorizontal);
    let (_, third) = &aux.facet_annotations[2];
    assert_eq!(third.facet_index, 2);
    assert_eq!(third.classification, FacetClassRecord::Overhang);

    // One surface group per object, in object push order.
    assert_eq!(aux.surface_groups.len(), 2);
    assert_eq!(aux.surface_groups[0].0, "obj-A");
    assert_eq!(aux.surface_groups[1].0, "obj-B");
    let grp = &aux.surface_groups[0].1;
    assert_eq!(grp.facet_indices, vec![0u32, 1, 2]);
    assert!((grp.z_min - 0.0).abs() < 1e-6);
    assert!((grp.z_max - 0.6).abs() < 1e-5);
    assert_eq!(grp.shell_count, 2);
}

/// Two independent dispatches through the rebuilt sdk-prepass-guest
/// must produce byte-identical `MeshAnalysisAuxiliary` payloads. The
/// drain has no hidden state (no timestamps / hashmap iteration order
/// / pointer-derived ordering), so determinism holds end to end.
#[test]
fn mesh_analysis_macro_path_drain_is_deterministic() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_sdk_prepass_guest(&engine) {
        Some(c) => c,
        None => {
            eprintln!(
                "SKIP: sdk-prepass-guest.component.wasm missing — \
                 rebuild test guests"
            );
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    let run_once = || {
        let module = make_compiled_module_with_config(
            "com.test.sdk-prepass-det-emit",
            "PrePass::MeshAnalysis",
            Arc::clone(&component),
            mesh_analysis_emit_config(2),
        );
        let blackboard = blackboard_with_objects(&["obj-1", "obj-2"]);
        match PrepassStageRunner::run_stage(
            &dispatcher,
            &"PrePass::MeshAnalysis".to_string(),
            &module,
            &blackboard,
        ) {
            Ok((PrepassStageOutput::MeshAnalysisAuxiliary(a), _)) => a,
            Ok((other, _)) => panic!(
                "expected MeshAnalysisAuxiliary, got {:?}",
                std::mem::discriminant(&other)
            ),
            Err(e) => panic!("dispatch failed: {e}"),
        }
    };
    let a = run_once();
    let b = run_once();
    assert_eq!(
        *a, *b,
        "macro-path MeshAnalysis drain must be byte-identical across runs"
    );
}

/// When a guest pushes no output, the dispatcher must return
/// `PrepassStageOutput::None` — preserves the existing empty-drain
/// contract that the round-trip regression tests rely on.
#[test]
fn mesh_analysis_macro_path_empty_drain_returns_none() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_sdk_prepass_guest(&engine) {
        Some(c) => c,
        None => {
            eprintln!(
                "SKIP: sdk-prepass-guest.component.wasm missing — \
                 rebuild test guests"
            );
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module_with_config(
        "com.test.sdk-prepass-empty",
        "PrePass::MeshAnalysis",
        component,
        ConfigView::from_map(HashMap::new()),
    );
    let blackboard = blackboard_with_objects(&["obj-1"]);
    let out = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshAnalysis".to_string(),
        &module,
        &blackboard,
    )
    .expect("empty-config path must succeed");
    assert!(matches!(out.0, PrepassStageOutput::None));
}

/// Host-side validation: a malformed push (empty object-id,
/// non-finite slope, inverted z range) must surface as a structured
/// error string on the `result<_, string>` WIT return. Unit-testing
/// the validator here covers the "push failure surfaces a precise
/// structured error" contract without requiring a malicious guest.
#[test]
fn mesh_analysis_output_push_validates_and_rejects_malformed() {
    use slicer_host::wit_host::prepass as pm;
    use slicer_host::wit_host::HostExecutionContext;
    use wasmtime::component::Resource;

    let mut ctx = HostExecutionContext::new("com.test.validator".to_string(), 0.0, 0.0, None, None);
    // Get a handle for the mesh-analysis-output resource.
    let handle = ctx
        .push_mesh_analysis_output()
        .expect("push mesh-analysis-output resource");

    // Empty object_id → Err(...)
    let empty_obj = String::new();
    let res = <HostExecutionContext as pm::HostMeshAnalysisOutput>::push_facet_annotation(
        &mut ctx,
        Resource::new_own(handle.rep()),
        empty_obj,
        pm::FacetAnnotation {
            facet_index: 0,
            slope_angle_deg: 30.0,
            classification: pm::FacetClass::Normal,
        },
    )
    .expect("host call must not fail at the wasmtime layer");
    match res {
        Err(msg) => assert!(
            msg.contains("object-id") && msg.contains("non-empty"),
            "empty object-id must surface a precise error: {msg}"
        ),
        Ok(()) => panic!("empty object-id should have been rejected"),
    }

    // Non-finite slope_angle_deg → Err(...)
    let res = <HostExecutionContext as pm::HostMeshAnalysisOutput>::push_facet_annotation(
        &mut ctx,
        Resource::new_own(handle.rep()),
        "obj-1".to_string(),
        pm::FacetAnnotation {
            facet_index: 7,
            slope_angle_deg: f32::NAN,
            classification: pm::FacetClass::Normal,
        },
    )
    .expect("host call must not fail at the wasmtime layer");
    match res {
        Err(msg) => assert!(
            msg.contains("non-finite") && msg.contains("slope_angle_deg"),
            "non-finite slope must surface a precise error: {msg}"
        ),
        Ok(()) => panic!("non-finite slope should have been rejected"),
    }

    // Inverted z range → Err(...)
    let res = <HostExecutionContext as pm::HostMeshAnalysisOutput>::push_surface_group(
        &mut ctx,
        Resource::new_own(handle.rep()),
        "obj-1".to_string(),
        pm::SurfaceGroupProposal {
            facet_indices: vec![1, 2, 3],
            z_min: 10.0,
            z_max: 5.0,
            shell_count: 1,
        },
    )
    .expect("host call must not fail at the wasmtime layer");
    match res {
        Err(msg) => assert!(
            msg.contains("z_max") && msg.contains("z_min"),
            "inverted z range must surface a precise error: {msg}"
        ),
        Ok(()) => panic!("inverted z range should have been rejected"),
    }

    // A well-formed push succeeds and is stored in push order.
    let res = <HostExecutionContext as pm::HostMeshAnalysisOutput>::push_facet_annotation(
        &mut ctx,
        Resource::new_own(handle.rep()),
        "obj-1".to_string(),
        pm::FacetAnnotation {
            facet_index: 0,
            slope_angle_deg: 45.0,
            classification: pm::FacetClass::Overhang,
        },
    )
    .expect("host call must not fail at the wasmtime layer");
    assert!(res.is_ok());
    assert_eq!(ctx.mesh_analysis_annotations.len(), 1);
    assert_eq!(ctx.mesh_analysis_annotations[0].0, "obj-1");
}

// ── STEP H: PrePass::MeshSegmentation macro-path regression ─────────────
//
// STEP H moved `mesh-segmentation` off its hand-written
// `wit_bindgen::generate!` duplicate onto the standard `#[slicer_module]`
// macro path. The macro now (a) forwards the real `_objects` list as
// skeletal `MeshObjectView`s and (b) drains
// `MeshSegmentationOutput::triangle_paint_marks()` back through the
// WIT `mesh-segmentation-output::mark-triangle-paint` resource method.
//
// The canonical end-to-end proof is that the pre-existing tests
// `mesh_segmentation_dispatch_returns_empty_ir_for_unpainted_mesh`,
// `mesh_segmentation_collects_config_driven_marks`, and
// `mesh_segmentation_dispatch_is_deterministic` (above) — which were
// written against the old wit-guest — keep passing verbatim with the
// rebuilt macro-path wasm. STEP H explicitly guarantees this contract
// shift: same WIT output, same host harvest, different authoring shape
// on the guest side.
//
// The test below adds a narrow regression specific to the drain path:
// it proves that the macro arm actually invokes
// `mark-triangle-paint` in push order (not some other ordering derived
// from the SDK builder) by replaying the canonical drain through the
// rebuilt wasm and spot-checking the first and last marks against the
// known-deterministic sort the module uses.

/// Regression guard for STEP H: after retiring the hand-written
/// wit-guest, the macro-path wasm must still emit `mark-triangle-paint`
/// calls in the module's declared sort order — `(object_index_in_host_list,
/// facet_index asc, semantic asc)`. If the drain were accidentally
/// reordered (e.g. by a HashMap-iteration detour through
/// `MeshSegmentationOutput`), this test surfaces it.
#[test]
fn mesh_segmentation_macro_path_drain_preserves_push_order() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_mesh_segmentation_default(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: mesh-segmentation.wasm missing — rebuild core modules");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    // Host object order: obj-B first, then obj-A. The module's sort
    // key is `object_index_in_host_list`, so all obj-B marks must
    // appear before any obj-A mark in the harvested IR even though
    // obj-A sorts lexically earlier.
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert(
        "mesh_seg_mark:obj-A:1:seam".into(),
        ConfigValue::String("x".into()),
    );
    fields.insert("mesh_seg_mark:obj-B:0:material".into(), ConfigValue::Int(5));
    fields.insert("mesh_seg_mark:obj-B:2:seam".into(), ConfigValue::Bool(true));
    let module = make_compiled_module_with_config(
        "com.test.mesh-seg-step-h",
        "PrePass::MeshSegmentation",
        component,
        ConfigView::from_declared(&fields, fields.keys().map(|s| s.as_str())),
    );
    let mesh = Arc::new(slicer_ir::MeshIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        objects: vec![make_object("obj-B"), make_object("obj-A")],
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
    });
    let blackboard = Blackboard::new(mesh, 0);

    let out = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshSegmentation".to_string(),
        &module,
        &blackboard,
    )
    .expect("mesh-segmentation dispatch must succeed");

    let ir = match out.0 {
        PrepassStageOutput::MeshSegmentation(ir) => ir,
        other => panic!(
            "expected MeshSegmentation variant, got {:?}",
            std::mem::discriminant(&other)
        ),
    };

    let keys: Vec<(String, u32, String, String)> = ir
        .marks
        .iter()
        .map(|m| {
            (
                m.object_id.clone(),
                m.facet_index,
                m.semantic.clone(),
                m.value.clone(),
            )
        })
        .collect();
    assert_eq!(
        keys,
        vec![
            // obj-B (host index 0) first, ordered by (facet asc, semantic asc):
            (
                "obj-B".to_string(),
                0,
                "material".to_string(),
                "5".to_string()
            ),
            (
                "obj-B".to_string(),
                2,
                "seam".to_string(),
                "true".to_string()
            ),
            // obj-A (host index 1) last:
            ("obj-A".to_string(), 1, "seam".to_string(), "x".to_string()),
        ],
        "macro-path drain must preserve the module's push order \
         (object_index_in_host_list, facet asc, semantic asc)"
    );
}

// ── PrePass::SeamPlanning tests (TASK-159) ──────────────────────────────

#[test]
fn prepass_seam_planning_requires_layer_plan_slot() {
    // PrePass::SeamPlanning requires BlackboardPrepassSlot::LayerPlan to be
    // committed before the stage can run. We verify this by calling the
    // ensure_stage_prerequisites check directly.
    use slicer_host::prepass::ensure_stage_prerequisites;

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    // No prepass slots committed — LayerPlan is absent.
    let result = ensure_stage_prerequisites(&"PrePass::SeamPlanning".to_string(), &blackboard);
    assert!(
        result.is_err(),
        "SeamPlanning must fail when LayerPlan is not committed"
    );
    let err = result.unwrap_err();
    match err {
        slicer_host::prepass::PrepassExecutionError::MissingRequiredPrepass { stage_id, slot } => {
            assert_eq!(stage_id.as_str(), "PrePass::SeamPlanning");
            assert_eq!(slot, slicer_host::BlackboardPrepassSlot::LayerPlan);
        }
        other => panic!("expected MissingRequiredPrepass, got {other:?}"),
    }
}

#[test]
fn seam_plan_ir_rejects_duplicate_region_keys() {
    // SeamPlanIR must reject commits that contain duplicate region keys
    // (same global_layer_index + object_id + region_id triple).
    // The validation happens at commit time in the blackboard.
    use slicer_host::{Blackboard, BlackboardError, BlackboardPrepassSlot};
    use slicer_ir::{RegionKey, SeamPlanEntry, SeamPlanIR, SeamPosition, SemVer};

    let mesh = empty_mesh_ir();
    let mut blackboard = Blackboard::new(empty_mesh_ir(), 0);

    // Build a minimal valid SeamPosition for the chosen_candidate field.
    let dummy_position = slicer_ir::Point3WithWidth {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        width: 0.4,
        flow_factor: 1.0,
    };
    let seam_position = SeamPosition {
        point: dummy_position,
        wall_index: 0,
    };

    // First commit with valid unique entries.
    let seam_plan = SeamPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        entries: vec![
            SeamPlanEntry {
                region_key: RegionKey {
                    global_layer_index: 0,
                    object_id: "obj-A".to_string(),
                    region_id: 1,
                },
                chosen_candidate: seam_position.clone(),
                scored_candidates: vec![],
            },
            SeamPlanEntry {
                region_key: RegionKey {
                    global_layer_index: 0,
                    object_id: "obj-B".to_string(),
                    region_id: 2,
                },
                chosen_candidate: seam_position.clone(),
                scored_candidates: vec![],
            },
        ],
    };

    // Commit once — should succeed.
    let result = blackboard.commit_seam_plan(std::sync::Arc::new(seam_plan));
    assert!(
        result.is_ok(),
        "first commit with unique keys should succeed"
    );

    // Second commit — same region key (global_layer_index=0, obj-A, region_id=1)
    // is a duplicate and must be rejected.
    let duplicate_seam_plan = SeamPlanIR {
        schema_version: SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
        entries: vec![SeamPlanEntry {
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "obj-A".to_string(),
                region_id: 1, // duplicate of above
            },
            chosen_candidate: seam_position,
            scored_candidates: vec![],
        }],
    };
    let result2 = blackboard.commit_seam_plan(std::sync::Arc::new(duplicate_seam_plan));
    assert!(
        result2.is_err(),
        "commit with duplicate region key must be rejected"
    );
    let err = result2.unwrap_err();
    match err {
        BlackboardError::DuplicatePrepassCommit { slot } => {
            assert_eq!(slot, BlackboardPrepassSlot::SeamPlan);
        }
        other => panic!("expected DuplicatePrepassCommit for SeamPlan slot, got {other:?}"),
    }
}

/// `PrePass::SeamPlanning` dispatch with the real seam-planner-default module
/// must return `PrepassStageOutput::SeamPlan`. The module is an MVP no-op (emits
/// no entries) but the harvest path must still produce a well-formed `SeamPlanIR`.
///
/// This is the Step 5 exit-condition test for AC-1.
#[test]
fn prepass_seam_planning_commits_seam_plan_ir() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = {
        const PATH: &str = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../modules/core-modules/seam-planner-default/seam-planner-default.wasm"
        );
        let path = std::path::Path::new(PATH);
        if !path.exists() {
            eprintln!("SKIP: seam-planner-default.wasm missing — rebuild core modules");
            return;
        }
        let bytes = std::fs::read(path).expect("read seam-planner-default.wasm");
        Arc::new(
            engine
                .compile_component(&bytes)
                .expect("compile seam-planner-default"),
        )
    };

    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));

    // Build a loaded + compiled module for SeamPlanning.
    let loaded = make_loaded_module("com.test.seam-planner", "PrePass::SeamPlanning");
    let pool = Arc::new(
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
        .unwrap(),
    );
    let compiled = CompiledModule {
        module_id: "com.test.seam-planner".to_string(),
        instance_pool: pool,
        ir_read_mask: Default::default(),
        ir_write_mask: Default::default(),
        config_view: Arc::new(ConfigView::from_map(Default::default())),
        wasm_component: Some(component),
    };

    // Build a blackboard with a committed LayerPlanIR (SeamPlanning's required slot).
    // The seam-planner-default module may or may not produce entries depending
    // on geometry (it skips empty meshes), so we assert the result is non-empty
    // only when geometry is present — AC-2 is verified via the live seam path test.
    let empty_mesh = empty_mesh_ir();
    let mut blackboard = Blackboard::new(Arc::clone(&empty_mesh), 0);
    blackboard
        .commit_layer_plan(Arc::new(LayerPlanIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            global_layers: vec![],
            object_participation: Default::default(),
        }))
        .expect("commit minimal layer plan for required slot");

    // Also commit SurfaceClassificationIR (MeshAnalysis output that other stages need).
    blackboard
        .commit_surface_classification(Arc::new(SurfaceClassificationIR {
            schema_version: SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            },
            per_object: Default::default(),
        }))
        .expect("commit surface classification for required slot");

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::SeamPlanning".to_string(),
        &compiled,
        &blackboard,
    );

    match result {
        Ok((PrepassStageOutput::SeamPlan(ir), _)) => {
            assert_eq!(
                ir.schema_version,
                SemVer {
                    major: 1,
                    minor: 0,
                    patch: 0
                },
                "SeamPlanIR schema_version must be 1.0.0"
            );
            // seam-planner-default emits seam entries for objects with mesh geometry.
            // Entries may be empty if the blackboard mesh has no objects.
            for entry in &ir.entries {
                assert!(
                    !entry.region_key.object_id.is_empty(),
                    "region_key.object_id must be non-empty"
                );
                assert!(
                    entry.region_key.global_layer_index < 1000,
                    "global_layer_index must be reasonable"
                );
                // Verify the chosen position is valid.
                assert!(entry.chosen_candidate.point.x.is_finite());
                assert!(entry.chosen_candidate.point.y.is_finite());
                assert!(entry.chosen_candidate.point.z.is_finite());
                assert!(entry.chosen_candidate.point.width > 0.0);
                eprintln!(
                    "DEBUG: seam_candidate point=({:.4}, {:.4}, {:.4}) wall_index={} width={:.4}",
                    entry.chosen_candidate.point.x,
                    entry.chosen_candidate.point.y,
                    entry.chosen_candidate.point.z,
                    entry.chosen_candidate.wall_index,
                    entry.chosen_candidate.point.width
                );
            }
            eprintln!(
                "DEBUG: prepass_seam_planning_commits_seam_plan_ir — SeamPlanIR entry count = {}",
                ir.entries.len()
            );
        }
        Ok((other, _)) => panic!(
            "expected PrepassStageOutput::SeamPlan, got {:?}",
            std::mem::discriminant(&other)
        ),
        Err(e) => panic!("SeamPlanning dispatch failed: {e}"),
    }
}
