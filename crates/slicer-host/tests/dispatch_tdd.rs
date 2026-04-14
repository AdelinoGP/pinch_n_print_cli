//! TDD tests for WASM runtime dispatch — proving real module invocation.
//!
//! These tests verify that the WasmRuntimeDispatcher actually calls into
//! WASM module exports through the component model, with proper error handling,
//! pool correctness, and structured diagnostics.
//!
//! Layer-stage tests use the pre-built test guest component (which implements
//! the full layer-module WIT world) and go through the typed boundary.
//! Non-layer tests use minimal WAT fixtures on the legacy untyped path.

#![allow(missing_docs)]

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
    LayerPaintMap, MeshIR, PaintRegionIR, PaintSemantic, PaintValue, Point2, Point3, Polygon,
    PrintMetadata, SemVer, SemanticRegion, SliceIR, SlicedRegion, StageId,
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
const GUEST_COMPONENT_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-guests/layer-infill-guest.component.wasm");
const PREPASS_GUEST_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-guests/prepass-guest.component.wasm");
const FINALIZATION_GUEST_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-guests/finalization-guest.component.wasm");
const POSTPASS_GUEST_PATH: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/../../test-guests/postpass-guest.component.wasm");

// ── Helpers ──────────────────────────────────────────────────────────────

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer { major, minor, patch }
}

fn empty_mesh_ir() -> Arc<MeshIR> {
    Arc::new(MeshIR {
        schema_version: semver(1, 0, 0),
        objects: Vec::new(),
        build_volume: BoundingBox3 {
            min: Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: Point3 { x: 0.0, y: 0.0, z: 0.0 },
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
    Arc::new(engine.compile_component(&bytes).expect("WAT compilation should succeed"))
}

fn load_guest_component(engine: &WasmEngine, path: &str) -> Arc<slicer_host::WasmComponent> {
    let bytes = std::fs::read(path).unwrap_or_else(|e| {
        panic!("Test guest component not found at {path}: {e}")
    });
    Arc::new(engine.compile_component(&bytes).expect("guest compilation should succeed"))
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

fn make_compiled_module(
    engine: &WasmEngine,
    id: &str,
    stage: &str,
    wat: &str,
) -> CompiledModule {
    make_compiled_module_with(id, stage, compile_wat(engine, wat))
}

fn make_compiled_module_with(
    id: &str,
    stage: &str,
    component: Arc<slicer_host::WasmComponent>,
) -> CompiledModule {
    make_compiled_module_with_config(id, stage, component, ConfigView { fields: HashMap::new() })
}

fn make_compiled_module_with_config(
    id: &str,
    stage: &str,
    component: Arc<slicer_host::WasmComponent>,
    config: ConfigView,
) -> CompiledModule {
    let loaded = make_loaded_module(id, stage);
    let pool = Arc::new(
        build_wasm_instance_pool(&loaded, 1, WasmArtifactMetadata { uses_shared_memory: false })
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
        build_wasm_instance_pool(&loaded, 1, WasmArtifactMetadata { uses_shared_memory: false })
            .unwrap(),
    );
    CompiledModule {
        module_id: id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(ConfigView { fields: HashMap::new() }),
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
    let module = make_compiled_module_with(
        "com.test.mesh", "PrePass::MeshAnalysis", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::MeshAnalysis".to_string(),
        &module,
        &blackboard,
    );

    assert!(result.is_ok(), "prepass dispatch should succeed: {:?}", result.err());
}

#[test]
fn layer_runner_invokes_typed_wasm_export() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    // Use the real test guest that implements the full layer-module world.
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", component,
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

    assert!(result.is_ok(), "typed layer dispatch should succeed: {:?}", result.err());
}

#[test]
fn finalization_runner_invokes_wasm_export() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_finalization_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.wipe", "PostPass::LayerFinalization", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut layers = Vec::new();

    let result = FinalizationStageRunner::run_stage(
        &dispatcher,
        &"PostPass::LayerFinalization".to_string(),
        &module,
        &blackboard,
        &mut layers,
    );

    assert!(result.is_ok(), "finalization dispatch should succeed: {:?}", result.err());
}

#[test]
fn postpass_gcode_runner_invokes_wasm_export() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_postpass_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.gpost", "PostPass::GCodePostProcess", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let mut gcode_ir = minimal_gcode_ir();

    let result = dispatcher.run_gcode_postprocess(
        &"PostPass::GCodePostProcess".to_string(),
        &module,
        &blackboard,
        &mut gcode_ir,
    );

    assert!(result.is_ok(), "gcode postpass dispatch should succeed: {:?}", result.err());
}

#[test]
fn postpass_text_runner_invokes_wasm_export() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_postpass_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.tpost", "PostPass::TextPostProcess", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 0);
    let result = dispatcher.run_text_postprocess(
        &"PostPass::TextPostProcess".to_string(),
        &module,
        &blackboard,
        "; some gcode".to_string(),
    );

    assert!(result.is_ok(), "text postpass dispatch should succeed: {:?}", result.err());
}

// ── C. Error-path coverage ──────────────────────────────────────────────

#[test]
fn typed_instantiation_failure_produces_structured_error() {
    // An empty component does not implement the layer-module world,
    // so typed instantiation must fail with a TypedInstantiation phase error.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module(
        &engine, "com.test.empty", "Layer::Infill", WAT_EMPTY_COMPONENT,
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

    assert!(result.is_err(), "should fail when component doesn't implement layer world");
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("com.test.empty"), "error should name the module: {msg}");
    assert!(
        msg.contains("TypedInstantiation") || msg.contains("Layer::Infill"),
        "error should reference typed instantiation or stage: {msg}"
    );
}

#[test]
fn missing_component_produces_structured_error() {
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

    assert!(result.is_err(), "should fail when no WASM component is available");
    let msg = format!("{}", result.unwrap_err());
    assert!(msg.contains("com.test.nowasm"), "error should name the module: {msg}");
    assert!(
        msg.contains("no compiled WASM component") || msg.contains("MissingComponent"),
        "error should indicate missing component: {msg}"
    );
}

// ── D. Pool correctness ─────────────────────────────────────────────────

#[test]
fn pool_slot_released_after_successful_typed_call() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", component,
    );

    // The module pool has size 1. If the slot isn't released, the second
    // call would deadlock.
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
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
        assert!(result.is_ok(), "call #{} should succeed (pool reuse): {:?}", i, result.err());
    }
}

#[test]
fn pool_slot_released_after_failed_typed_call() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    // Empty component — will fail at typed instantiation
    let module = make_compiled_module(
        &engine, "com.test.empty", "Layer::Infill", WAT_EMPTY_COMPONENT,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
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
    let module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
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
    let layer_module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", component,
    );

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
        "com.test.mesh", "PrePass::MeshAnalysis", load_prepass_guest(&engine),
    );
    let layer_module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", load_test_guest(&engine),
    );
    let fin_module = make_compiled_module_with(
        "com.test.wipe", "PostPass::LayerFinalization", load_finalization_guest(&engine),
    );
    let gcode_module = make_compiled_module_with(
        "com.test.gpost", "PostPass::GCodePostProcess", load_postpass_guest(&engine),
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
            index: 0, z: 0.2, active_regions: Vec::new(),
            has_nonplanar: false, is_sync_layer: false,
        }]),
        region_plans: Arc::new(HashMap::new()),
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
    let module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 7,
        z: 1.4,
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
    assert!(result.is_ok(), "dispatch should succeed: {:?}", result.err());

    // Verify the infill slot is populated
    let infill = arena.infill().expect("infill arena slot should be populated");
    assert_eq!(infill.global_layer_index, 7, "layer index should match");
    assert_eq!(infill.regions.len(), 1, "should have 1 region");
    let region = &infill.regions[0];
    assert_eq!(region.sparse_infill.len(), 1, "should have 1 sparse path");
    // The test guest creates a path with 2 points
    assert_eq!(region.sparse_infill[0].points.len(), 2, "path should have 2 points");
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
        "com.test.support-pp", "Layer::SupportPostProcess", component,
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
    assert!(result.is_ok(), "dispatch should succeed: {:?}", result.err());

    // Support slot should remain empty because guest produced no output
    assert!(arena.support().is_none(), "support slot should be empty for no-op stage");
}

#[test]
fn output_commitment_deterministic_across_repeated_runs() {
    // Running the same dispatch 3 times with fresh arenas should produce
    // identical InfillIR each time.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };

    let mut results = Vec::new();
    for _ in 0..3 {
        let mut arena = LayerArena::new();
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
    use slicer_host::wit_host::{convert_infill_output, InfillOutputCollected, ExtrusionPath3d, Point3WithWidth, ExtrusionRole};

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
    assert!(msg.contains("point[0]"), "error should identify the point index: {msg}");
}

#[test]
fn end_to_end_pipeline_commits_guest_output_to_arena() {
    // Full pipeline test: manifest → plan → typed dispatch → output committed.
    // We verify that the per-layer execution produces a LayerCollectionIR
    // from a pipeline that includes a real WASM infill module.
    let engine = Arc::new(WasmEngine::new());

    let component = load_test_guest(&engine);
    let layer_module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", component,
    );

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
                index: 0, z: 0.2, active_regions: Vec::new(),
                has_nonplanar: false, is_sync_layer: false,
            },
            GlobalLayer {
                index: 1, z: 0.4, active_regions: Vec::new(),
                has_nonplanar: false, is_sync_layer: false,
            },
        ]),
        region_plans: Arc::new(HashMap::new()),
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
    assert!(display.contains("com.test.mod"), "should include module_id: {display}");
    assert!(display.contains("Layer::Infill"), "should include stage_id: {display}");
    assert!(display.contains("run-infill"), "should include export_name: {display}");
    assert!(display.contains("function not found"), "should include reason: {display}");
}

// ── H. Perimeter output commit tests ──────────────────────────────────

#[test]
fn perimeter_output_converts_wall_loops_and_commits_to_arena() {
    use slicer_host::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole,
        PerimeterOutputCollected, Point3, Point3WithWidth, WallFeatureFlag,
        WallLoopType, WallLoopView,
    };

    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth { x: 0.0, y: 0.0, z: 0.2, width: 0.4, flow_factor: 1.0 },
                    Point3WithWidth { x: 10.0, y: 0.0, z: 0.2, width: 0.4, flow_factor: 1.0 },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag { tool_index: None, fuzzy_skin: false, is_bridge: false, is_thin_wall: false, skip_ironing: false },
                WallFeatureFlag { tool_index: None, fuzzy_skin: false, is_bridge: false, is_thin_wall: false, skip_ironing: false },
            ],
        }],
        infill_areas: Vec::new(),
        seam_candidates: vec![(Point3 { x: 5.0, y: 0.0, z: 0.2 }, 0.8)],
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
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole,
        PerimeterOutputCollected, Point3WithWidth, WallFeatureFlag,
        WallLoopType, WallLoopView,
    };

    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![Point3WithWidth {
                    x: f32::NAN, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0,
                }],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag { tool_index: None, fuzzy_skin: false, is_bridge: false, is_thin_wall: false, skip_ironing: false },
            ],
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
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole,
        PerimeterOutputCollected, Point3WithWidth, WallFeatureFlag,
        WallLoopType, WallLoopView,
    };

    // 2 points but only 1 feature flag → cardinality mismatch per docs/03
    let output = PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth { x: 0.0, y: 0.0, z: 0.2, width: 0.4, flow_factor: 1.0 },
                    Point3WithWidth { x: 10.0, y: 0.0, z: 0.2, width: 0.4, flow_factor: 1.0 },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag { tool_index: None, fuzzy_skin: false, is_bridge: false, is_thin_wall: false, skip_ironing: false },
                // Missing second flag
            ],
        }],
        infill_areas: Vec::new(),
        seam_candidates: Vec::new(),
        ..Default::default()
    };

    let result = convert_perimeter_output(&output, 0);
    assert!(result.is_err(), "feature flag cardinality mismatch should be rejected");
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
        seam_candidates: vec![(Point3 { x: f32::NAN, y: 0.0, z: 0.0 }, 1.0)],
        ..Default::default()
    };

    let result = convert_perimeter_output(&output, 0);
    assert!(result.is_err(), "NaN seam candidate should be rejected");
    let msg = result.unwrap_err();
    assert!(msg.contains("seam_candidate"), "error should identify seam: {msg}");
    assert!(msg.contains("NaN"), "error should mention NaN: {msg}");
}

#[test]
fn empty_perimeter_output_does_not_populate_arena() {
    // The test guest's run_perimeters is a no-op, so perimeter slot stays empty.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.perim", "Layer::Perimeters", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
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
    assert!(result.is_ok(), "empty perimeter dispatch should succeed: {:?}", result.err());
    assert!(arena.perimeter().is_none(), "perimeter slot should be empty for no-op");
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

    let merged = merge_slice_postprocess_into(existing.clone(), &output).expect("merge should succeed");
    assert_eq!(merged.regions.len(), 2, "all regions preserved (not flattened)");
    assert_eq!(merged.regions[0], existing.regions[0], "untouched region unchanged");
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
    assert!(result.is_err(), "unknown region key must fail with structured diagnostic");
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
    let module = make_compiled_module_with(
        "com.test.slicepp", "Layer::SlicePostProcess", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
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
    assert!(result.is_ok(), "empty slicepp dispatch should succeed: {:?}", result.err());
    assert!(arena.slice().is_none(), "slice slot should be empty for no-op");
}

// ── J. Determinism and isolation for perimeter commit ──────────────────

#[test]
fn perimeter_conversion_deterministic_across_repeated_calls() {
    use slicer_host::wit_host::{
        convert_perimeter_output, ExtrusionPath3d, ExtrusionRole,
        PerimeterOutputCollected, Point3, Point3WithWidth, WallFeatureFlag,
        WallLoopType, WallLoopView,
    };

    let mk_output = || PerimeterOutputCollected {
        wall_loops: vec![WallLoopView {
            perimeter_index: 0,
            loop_type: WallLoopType::Outer,
            path: ExtrusionPath3d {
                points: vec![
                    Point3WithWidth { x: 1.0, y: 2.0, z: 0.2, width: 0.4, flow_factor: 1.0 },
                    Point3WithWidth { x: 3.0, y: 4.0, z: 0.2, width: 0.4, flow_factor: 1.0 },
                ],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![
                WallFeatureFlag { tool_index: Some(0), fuzzy_skin: true, is_bridge: false, is_thin_wall: false, skip_ironing: false },
                WallFeatureFlag { tool_index: Some(0), fuzzy_skin: true, is_bridge: false, is_thin_wall: false, skip_ironing: false },
            ],
        }],
        infill_areas: Vec::new(),
        seam_candidates: vec![(Point3 { x: 2.0, y: 1.0, z: 0.2 }, 0.9)],
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
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };

    // First call: infill (produces output)
    let infill_module = make_compiled_module_with(
        "com.test.infill", "Layer::Infill", Arc::clone(&component),
    );
    let mut arena = LayerArena::new();
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
        "com.test.perim", "Layer::Perimeters", Arc::clone(&component),
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
    assert!(arena.perimeter().is_none(), "perimeter slot should stay empty");
    assert!(arena.infill().is_some(), "infill slot should still be populated");
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

    let mut fields = HashMap::new();
    fields.insert("infill-spacing".into(), ConfigValue::Float(5.0));
    let config = ConfigView { fields };

    let module = make_compiled_module_with_config(
        "com.test.infill", "Layer::Infill", component, config,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
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
    assert!(result.is_ok(), "dispatch with config should succeed: {:?}", result.err());

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
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };

    // Config A: spacing=3.0 → x=30.0
    let config_a = ConfigView {
        fields: [("infill-spacing".into(), ConfigValue::Float(3.0))].into(),
    };
    let mod_a = make_compiled_module_with_config(
        "com.test.infill-a", "Layer::Infill", Arc::clone(&component), config_a,
    );
    let mut arena_a = LayerArena::new();
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
    let config_b = ConfigView {
        fields: [("infill-spacing".into(), ConfigValue::Float(7.0))].into(),
    };
    let mod_b = make_compiled_module_with_config(
        "com.test.infill-b", "Layer::Infill", Arc::clone(&component), config_b,
    );
    let mut arena_b = LayerArena::new();
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
    assert_ne!(x_a, x_b, "different configs should produce different output");
}

#[test]
fn repeated_identical_config_produces_deterministic_output() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };

    let mk_module = || {
        let config = ConfigView {
            fields: [("infill-spacing".into(), ConfigValue::Float(4.0))].into(),
        };
        make_compiled_module_with_config(
            "com.test.infill", "Layer::Infill", Arc::clone(&component), config,
        )
    };

    let mut results = Vec::new();
    for _ in 0..3 {
        let module = mk_module();
        let mut arena = LayerArena::new();
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
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };

    // First call: spacing=6.0
    let config1 = ConfigView {
        fields: [("infill-spacing".into(), ConfigValue::Float(6.0))].into(),
    };
    let mod1 = make_compiled_module_with_config(
        "com.test.infill", "Layer::Infill", Arc::clone(&component), config1,
    );
    let mut arena1 = LayerArena::new();
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
    let config2 = ConfigView {
        fields: [("infill-spacing".into(), ConfigValue::Float(2.0))].into(),
    };
    let mod2 = make_compiled_module_with_config(
        "com.test.infill2", "Layer::Infill", Arc::clone(&component), config2,
    );
    let mut arena2 = LayerArena::new();
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
                            Point2 { x: 10_000, y: 10_000 },
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
    // into support output: x=enforcer_count, y=blocker_count, z=layer_index.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.support",
        "Layer::Support",
        Arc::clone(&component),
    );

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
    assert_eq!(
        p.x, 3.0,
        "enforcer count should be 3, got {}",
        p.x
    );
    assert_eq!(
        p.y, 1.0,
        "blocker count should be 1, got {}",
        p.y
    );
    assert_eq!(
        p.z, 7.0,
        "paint layer index should match layer.index=7, got {}",
        p.z
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
    let module = make_compiled_module_with(
        "com.test.support",
        "Layer::Support",
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
    let module = make_compiled_module_with(
        "com.test.support",
        "Layer::Support",
        Arc::clone(&component),
    );

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
        p.z, 10.0,
        "paint layer index should be 10 (execution layer), got {}",
        p.z
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
    let module1 = make_compiled_module_with(
        "com.test.support",
        "Layer::Support",
        Arc::clone(&component),
    );
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena1 = LayerArena::new();
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
    let module1 = make_compiled_module_with(
        "com.test.infill",
        "Layer::Infill",
        Arc::clone(&component),
    );
    let layer = GlobalLayer {
        index: 0,
        z: 0.2,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena1 = LayerArena::new();
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
    let module2 = make_compiled_module_with(
        "com.test.infill2",
        "Layer::Infill",
        Arc::clone(&component),
    );
    let mut arena2 = LayerArena::new();
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
        infill1.regions[0].sparse_infill[0].points,
        infill2.regions[0].sparse_infill[0].points,
        "infill output should be identical regardless of paint presence"
    );
}

// ── I. Slice-region wiring tests ────────────────────────────────────────

fn make_slice_ir(layer_index: u32, z: f32, region_count: usize, polys_per_region: usize) -> SliceIR {
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
                            Point2 { x: 10_000, y: 10_000 },
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
    let module = make_compiled_module_with(
        "com.test.infill",
        "Layer::Infill",
        Arc::clone(&component),
    );

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
    // When the arena has no SliceIR, the guest should see 0 regions.
    // The guest encodes region_count=0 into flow_factor of point[0].
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.infill",
        "Layer::Infill",
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

    let infill = arena.infill().unwrap();
    let p0 = &infill.regions[0].sparse_infill[0].points[0];
    assert_eq!(p0.flow_factor, 0.0, "no slice regions → region count 0");
    assert_eq!(p0.width, 0.0, "no slice regions → polygon count 0");
    assert_eq!(p0.z, 0.0, "no slice regions → z default 0");
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
    let module1 = make_compiled_module_with(
        "com.test.infill1",
        "Layer::Infill",
        Arc::clone(&component),
    );
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
    let module2 = make_compiled_module_with(
        "com.test.infill2",
        "Layer::Infill",
        Arc::clone(&component),
    );
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
    let module = make_compiled_module_with(
        "com.test.support",
        "Layer::Support",
        Arc::clone(&component),
    );

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

    let mut fields = HashMap::new();
    fields.insert("infill-spacing".into(), ConfigValue::Float(3.0));
    let module = make_compiled_module_with_config(
        "com.test.infill",
        "Layer::Infill",
        Arc::clone(&component),
        ConfigView { fields },
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
    assert_eq!(path.points[1].x, 30.0, "config wiring still works with slice regions present");
    // First point encodes region data: z from slice, region_count=1, poly_count=2
    assert_eq!(path.points[0].z, 1.0, "z from slice region");
    assert_eq!(path.points[0].flow_factor, 1.0, "1 region visible");
    assert_eq!(path.points[0].width, 2.0, "2 polygons visible");
    assert_eq!(infill.global_layer_index, 5, "layer index preserved in output");
}

// ── L. Perimeter-region wiring tests ────────────────────────────────────

fn make_wall_loop(perimeter_index: u32, point_count: usize) -> slicer_ir::WallLoop {
    let points = (0..point_count)
        .map(|i| slicer_ir::Point3WithWidth {
            x: i as f32, y: 0.0, z: 0.2,
            width: 0.4, flow_factor: 1.0,
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

fn make_perimeter_ir(layer_index: u32, regions: usize, walls_per_region: u32, infill_polys: usize) -> slicer_ir::PerimeterIR {
    let regions = (0..regions)
        .map(|i| slicer_ir::PerimeterRegion {
            object_id: format!("obj-{i}"),
            region_id: i as u64,
            walls: (0..walls_per_region).map(|w| make_wall_loop(w, 2)).collect(),
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
    let module = make_compiled_module_with(
        "com.test.infill-pp", "Layer::InfillPostProcess", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 2, z: 0.4, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir(2, 3, 2, 4)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::InfillPostProcess".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

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
        "com.test.perim-pp", "Layer::PerimetersPostProcess", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 1, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir(1, 2, 3, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PerimetersPostProcess".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

    // Post-process replaces perimeter slot with guest's committed output;
    // each input region produces its own committed PerimeterRegion.
    let perim = arena.perimeter().expect("perimeter slot should be populated");
    assert_eq!(perim.regions.len(), 2, "one PerimeterRegion per input region");
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
    let module = make_compiled_module_with(
        "com.test.pathopt", "Layer::PathOptimization", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir(0, 4, 2, 0)).unwrap();

    let r = LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::PathOptimization".to_string(),
        &layer, &module, &blackboard, &mut arena,
    );
    assert!(r.is_ok(), "path-optimization with real perimeter regions should succeed: {:?}", r.err());
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
        "com.test.infill-pp-empty", "Layer::InfillPostProcess", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Do not stage any perimeter IR.

    LayerStageRunner::run_stage(
        &dispatcher,
        &"Layer::InfillPostProcess".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

    assert!(arena.infill().is_none(), "no input regions → no output → empty bypass");
}

#[test]
fn perimeter_region_isolation_across_sequential_dispatches() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };

    let m1 = make_compiled_module_with("com.test.ipp1", "Layer::InfillPostProcess", Arc::clone(&component));
    let mut a1 = LayerArena::new();
    a1.set_perimeter(make_perimeter_ir(0, 5, 1, 2)).unwrap();
    LayerStageRunner::run_stage(&dispatcher, &"Layer::InfillPostProcess".to_string(), &layer, &m1, &blackboard, &mut a1).unwrap();

    let m2 = make_compiled_module_with("com.test.ipp2", "Layer::InfillPostProcess", Arc::clone(&component));
    let mut a2 = LayerArena::new();
    a2.set_perimeter(make_perimeter_ir(0, 1, 7, 3)).unwrap();
    LayerStageRunner::run_stage(&dispatcher, &"Layer::InfillPostProcess".to_string(), &layer, &m2, &blackboard, &mut a2).unwrap();

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
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
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
            &dispatcher, &"Layer::InfillPostProcess".to_string(),
            &layer, &module, &blackboard, &mut arena,
        ).unwrap();
        results.push(arena.take_infill().unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn stage_without_perimeter_input_does_not_see_perimeter_state() {
    // Layer::Infill consumes slice regions, not perimeter regions. Even if
    // PerimeterIR is staged in the arena, the infill guest should not
    // observe it — it should only see slice regions (zero, here).
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.infill-no-perim", "Layer::Infill", component,
    );

    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Stage perimeter data only; no slice data.
    arena.set_perimeter(make_perimeter_ir(0, 4, 2, 5)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher, &"Layer::Infill".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

    // Guest sees zero slice regions (confirming perimeter state is NOT
    // misrouted to the slice-region view).
    let infill = arena.infill().unwrap();
    let p0 = &infill.regions[0].sparse_infill[0].points[0];
    assert_eq!(p0.flow_factor, 0.0, "Infill stage must not see perimeter data as slice regions");
    assert_eq!(p0.width, 0.0, "no polygons visible via slice view");
}

// ── M. Identity-preservation tests for post-process commit ─────────────

fn make_perimeter_ir_with_ids(layer_index: u32, ids: &[(&str, u64)], walls_per: u32, infill_per: usize) -> slicer_ir::PerimeterIR {
    let regions = ids
        .iter()
        .map(|(obj, rid)| slicer_ir::PerimeterRegion {
            object_id: (*obj).to_string(),
            region_id: *rid,
            walls: (0..walls_per).map(|w| make_wall_loop(w, 2)).collect(),
            infill_areas: (0..infill_per)
                .map(|_| ExPolygon {
                    contour: Polygon {
                        points: vec![Point2 { x: 0, y: 0 }, Point2 { x: 1, y: 0 }, Point2 { x: 1, y: 1 }],
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
        "com.test.perim-pp-ids", "Layer::PerimetersPostProcess", component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer { index: 0, z: 0.2, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false };
    let ids = [("alpha", 11u64), ("beta", 22u64), ("gamma", 33u64)];
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir_with_ids(0, &ids, 2, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher, &"Layer::PerimetersPostProcess".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

    let perim = arena.perimeter().expect("perimeter populated");
    assert_eq!(perim.regions.len(), 3, "3 distinct regions preserved (not flattened)");
    let observed: Vec<(String, u64)> = perim.regions.iter()
        .map(|r| (r.object_id.clone(), r.region_id)).collect();
    let expected: Vec<(String, u64)> = ids.iter().map(|(o, r)| (o.to_string(), *r)).collect();
    assert_eq!(observed, expected, "identities preserved in input order");
    for r in &perim.regions {
        assert_eq!(r.walls.len(), 1, "each committed region got its own wall-loop");
    }
}

#[test]
fn infill_postprocess_commit_preserves_distinct_region_identities() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.infill-pp-ids", "Layer::InfillPostProcess", component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer { index: 0, z: 0.2, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false };
    let ids = [("part-A", 7u64), ("part-B", 9u64)];
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir_with_ids(0, &ids, 1, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher, &"Layer::InfillPostProcess".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

    let infill = arena.infill().expect("infill populated");
    assert_eq!(infill.regions.len(), 2, "2 distinct regions preserved");
    let observed: Vec<(String, u64)> = infill.regions.iter()
        .map(|r| (r.object_id.clone(), r.region_id)).collect();
    let expected: Vec<(String, u64)> = ids.iter().map(|(o, r)| (o.to_string(), *r)).collect();
    assert_eq!(observed, expected, "identities preserved in input order");
}

#[test]
fn perimeter_postprocess_identity_preservation_deterministic() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer { index: 0, z: 0.2, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false };
    let ids = [("x", 1u64), ("y", 2u64), ("z", 3u64), ("w", 4u64)];
    let mut results = Vec::new();
    for i in 0..3 {
        let module = make_compiled_module_with(
            &format!("com.test.perim-pp-det-{i}"),
            "Layer::PerimetersPostProcess", Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        arena.set_perimeter(make_perimeter_ir_with_ids(0, &ids, 2, 0)).unwrap();
        LayerStageRunner::run_stage(
            &dispatcher, &"Layer::PerimetersPostProcess".to_string(),
            &layer, &module, &blackboard, &mut arena,
        ).unwrap();
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
    let layer = GlobalLayer { index: 0, z: 0.2, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false };

    let m1 = make_compiled_module_with("com.test.iso1", "Layer::PerimetersPostProcess", Arc::clone(&component));
    let mut a1 = LayerArena::new();
    a1.set_perimeter(make_perimeter_ir_with_ids(0, &[("first", 100), ("second", 200)], 1, 0)).unwrap();
    LayerStageRunner::run_stage(&dispatcher, &"Layer::PerimetersPostProcess".to_string(), &layer, &m1, &blackboard, &mut a1).unwrap();

    let m2 = make_compiled_module_with("com.test.iso2", "Layer::PerimetersPostProcess", Arc::clone(&component));
    let mut a2 = LayerArena::new();
    a2.set_perimeter(make_perimeter_ir_with_ids(0, &[("alt", 999)], 1, 0)).unwrap();
    LayerStageRunner::run_stage(&dispatcher, &"Layer::PerimetersPostProcess".to_string(), &layer, &m2, &blackboard, &mut a2).unwrap();

    let p1 = a1.perimeter().unwrap();
    let p2 = a2.perimeter().unwrap();
    assert_eq!(p1.regions.iter().map(|r| (r.object_id.clone(), r.region_id)).collect::<Vec<_>>(),
               vec![("first".to_string(), 100), ("second".to_string(), 200)]);
    assert_eq!(p2.regions.iter().map(|r| (r.object_id.clone(), r.region_id)).collect::<Vec<_>>(),
               vec![("alt".to_string(), 999)],
               "no leak from prior dispatch's identities");
}

#[test]
fn support_postprocess_empty_bypass_when_no_slice_regions() {
    // With no slice regions staged in the arena, the guest iterates nothing
    // and emits no support output; empty-bypass leaves the support slot None.
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with("com.test.spp-empty", "Layer::SupportPostProcess", component);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer { index: 0, z: 0.2, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false };
    let mut arena = LayerArena::new();
    LayerStageRunner::run_stage(&dispatcher, &"Layer::SupportPostProcess".to_string(), &layer, &module, &blackboard, &mut arena).unwrap();
    assert!(arena.support().is_none(), "empty-input post-process: empty bypass preserved");
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
                points: vec![Point3WithWidth { x: 0.0, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0 }],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            feature_flags: vec![WallFeatureFlag {
                tool_index: None, fuzzy_skin: false, is_bridge: false, is_thin_wall: false, skip_ironing: false,
            }],
        }],
        wall_loop_origins: vec![None],
        infill_areas: Vec::new(),
        infill_areas_origin: None,
        seam_candidates: Vec::new(),
        seam_candidate_origins: Vec::new(),
    };
    // Force "any_tagged" by setting a dummy infill_areas_origin so the
    // identity-preserving path is taken; then the untagged wall_loop fails.
    let mut output = output;
    output.infill_areas_origin = Some(("dummy".into(), 0));
    let result = convert_perimeter_output(&output, 0);
    assert!(result.is_err(), "untagged push in identity mode must fail");
    let msg = result.unwrap_err();
    assert!(msg.contains("active perimeter source region") || msg.contains("without an active"),
            "diagnostic should explain missing region context: {msg}");
}

// ── K. SlicePostProcess / SupportPostProcess identity-preserving commit ─

#[test]
fn slice_postprocess_commit_preserves_distinct_region_identities() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.slice-pp-ids", "Layer::SlicePostProcess", component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Three distinct slice regions (object_id varies via make_slice_ir: obj-0..obj-2)
    arena.set_slice(make_slice_ir(0, 0.2, 3, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher, &"Layer::SlicePostProcess".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

    let slice = arena.slice().expect("slice populated after post-process merge");
    assert_eq!(slice.regions.len(), 3, "all three source regions preserved (not flattened)");
    let observed: Vec<(String, u64)> = slice.regions.iter()
        .map(|r| (r.object_id.clone(), r.region_id)).collect();
    let expected: Vec<(String, u64)> = vec![
        ("obj-0".into(), 0), ("obj-1".into(), 1), ("obj-2".into(), 2),
    ];
    assert_eq!(observed, expected, "identities preserved in input order after merge");
    // Guest replaced each region's polygons with a triangle (3 points).
    for r in &slice.regions {
        assert_eq!(r.polygons.len(), 1);
        assert_eq!(r.polygons[0].contour.points.len(), 3, "guest polygon replacement applied per region");
    }
}

#[test]
fn support_postprocess_commit_preserves_distinct_region_identities() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let module = make_compiled_module_with(
        "com.test.support-pp-ids", "Layer::SupportPostProcess", component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // Two distinct slice regions: (obj-0, 0), (obj-1, 1). Guest pushes one
    // support path per region; convert_support_output groups by origin with
    // structured diagnostics on untagged output.
    arena.set_slice(make_slice_ir(0, 0.2, 2, 1)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher, &"Layer::SupportPostProcess".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

    let support = arena.support().expect("support populated after post-process");
    assert_eq!(support.support_paths.len(), 2, "two origin-tagged paths preserved");
    // First-seen ordering by origin is stable; each path encodes poly count.
    assert_eq!(support.support_paths[0].points[0].x, 1.0, "region 0 has 1 polygon");
    assert_eq!(support.support_paths[1].points[0].x, 1.0, "region 1 has 1 polygon");
}

#[test]
fn slice_postprocess_identity_preservation_deterministic() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_test_guest(&engine);
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut results = Vec::new();
    for i in 0..3 {
        let module = make_compiled_module_with(
            &format!("com.test.spp-det-{i}"),
            "Layer::SlicePostProcess", Arc::clone(&component),
        );
        let mut arena = LayerArena::new();
        arena.set_slice(make_slice_ir(0, 0.2, 4, 1)).unwrap();
        LayerStageRunner::run_stage(
            &dispatcher, &"Layer::SlicePostProcess".to_string(),
            &layer, &module, &blackboard, &mut arena,
        ).unwrap();
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
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };

    let m1 = make_compiled_module_with("com.test.spp-iso1", "Layer::SupportPostProcess", Arc::clone(&component));
    let mut a1 = LayerArena::new();
    a1.set_slice(make_slice_ir(0, 0.2, 3, 2)).unwrap();
    LayerStageRunner::run_stage(&dispatcher, &"Layer::SupportPostProcess".to_string(), &layer, &m1, &blackboard, &mut a1).unwrap();

    let m2 = make_compiled_module_with("com.test.spp-iso2", "Layer::SupportPostProcess", Arc::clone(&component));
    let mut a2 = LayerArena::new();
    a2.set_slice(make_slice_ir(0, 0.2, 1, 1)).unwrap();
    LayerStageRunner::run_stage(&dispatcher, &"Layer::SupportPostProcess".to_string(), &layer, &m2, &blackboard, &mut a2).unwrap();

    assert_eq!(a1.support().unwrap().support_paths.len(), 3, "dispatch 1 kept its 3 regions");
    assert_eq!(a2.support().unwrap().support_paths.len(), 1, "dispatch 2 kept its 1 region (no leak)");
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
        points: vec![Point3WithWidth { x: 0.0, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0 }],
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
    assert!(msg.contains("active slice source region") || msg.contains("without an active"),
            "diagnostic should explain missing region context: {msg}");
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
    let spp = make_compiled_module_with("com.test.spp-prop", "Layer::SlicePostProcess", Arc::clone(&component));
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer { index: 0, z: 0.2, active_regions: Vec::new(), has_nonplanar: false, is_sync_layer: false };

    let mut arena = LayerArena::new();
    arena.set_slice(make_slice_ir(0, 0.2, 3, 1)).unwrap();
    LayerStageRunner::run_stage(
        &dispatcher, &"Layer::SlicePostProcess".to_string(),
        &layer, &spp, &blackboard, &mut arena,
    ).unwrap();

    // Now dispatch a downstream stage that consumes slice regions (Support).
    // The test guest's run_support observes paint data, but the key proof is
    // that push_slice_regions sees all three regions after SlicePostProcess.
    let sup = make_compiled_module_with("com.test.sup-prop", "Layer::SupportPostProcess", Arc::clone(&component));
    LayerStageRunner::run_stage(
        &dispatcher, &"Layer::SupportPostProcess".to_string(),
        &layer, &sup, &blackboard, &mut arena,
    ).unwrap();

    let support = arena.support().expect("support populated via propagated slice regions");
    assert_eq!(
        support.support_paths.len(), 3,
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
    let module = make_compiled_module_with(
        "com.test.pathopt-tc", "Layer::PathOptimization", component,
    );
    let blackboard = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 0, z: 0.2, active_regions: Vec::new(),
        has_nonplanar: false, is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    arena.set_perimeter(make_perimeter_ir(0, 3, 1, 0)).unwrap();

    LayerStageRunner::run_stage(
        &dispatcher, &"Layer::PathOptimization".to_string(),
        &layer, &module, &blackboard, &mut arena,
    ).unwrap();

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
                modules: vec![make_compiled_module_with("com.test.pathopt-seed", "Layer::Perimeters", Arc::clone(&component))],
            },
            slicer_host::CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![make_compiled_module_with("com.test.pathopt-e2e", "Layer::PathOptimization", Arc::clone(&component))],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0, z: 0.2, active_regions: Vec::new(),
            has_nonplanar: false, is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
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
        ) -> Result<LayerStageOutput, LayerStageError> {
            if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    arena.set_perimeter(p).unwrap();
                    return Ok(LayerStageOutput::Success);
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
        l.ordered_entities.len(), 2,
        "ordered_entities pre-staged from assembly visible at end",
    );
    assert_eq!(
        l.tool_changes.len(), 2,
        "guest-emitted tool-change overrides folded into LayerCollectionIR",
    );
    // Region identity preserved through the loop.
    for (i, e) in l.ordered_entities.iter().enumerate() {
        assert_eq!(e.region_key.global_layer_index, 0);
        assert_eq!(e.topo_order, i as u32);
    }
    // after_entity_index anchored at the last pre-assembled entity.
    let anchor = (l.ordered_entities.len() - 1) as u32;
    for tc in &l.tool_changes {
        assert_eq!(tc.after_entity_index, anchor);
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
            modules: vec![make_compiled_module_with("com.test.pathopt-empty", "Layer::PathOptimization", component)],
        }],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0, z: 0.2, active_regions: Vec::new(),
            has_nonplanar: false, is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
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
        ) -> Result<LayerStageOutput, LayerStageError> {
            if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    arena.set_perimeter(p).unwrap();
                    return Ok(LayerStageOutput::Success);
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
                modules: vec![make_compiled_module_with("com.test.pathopt-det-seed", "Layer::Perimeters", Arc::clone(&component))],
            },
            slicer_host::CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![make_compiled_module_with("com.test.pathopt-det", "Layer::PathOptimization", component)],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0, z: 0.2, active_regions: Vec::new(),
            has_nonplanar: false, is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
    };

    let mut results = Vec::new();
    for _ in 0..3 {
        let blackboard = Blackboard::new(empty_mesh_ir(), 1);
        let runner = SeedingRunner {
            inner: &dispatcher,
            perim: Mutex::new(Some(make_perimeter_ir(0, 3, 1, 0))),
        };
        results.push(execute_per_layer(&make_plan(Arc::clone(&component)), &blackboard, &runner).unwrap());
    }
    assert_eq!(results[0], results[1]);
    assert_eq!(results[1], results[2]);
}

#[test]
fn path_optimization_rejects_unsupported_gcode_override() {
    // Guest emits a Move via gcode-output-builder — no documented mapping
    // into LayerCollectionIR → commit path must surface a structured error.
    // Build a tiny WAT guest that exports only run-path-optimization and
    // emits a move is non-trivial, so exercise commit path directly.
    use slicer_host::wit_host::{GcodeCommandCollected, GcodeMoveCmd, ExtrusionRole, HostExecutionContext};
    let mut ctx = HostExecutionContext::new("com.test.pathopt-bad".to_string());
    ctx.gcode_output.commands.push(GcodeCommandCollected::Move(GcodeMoveCmd {
        x: Some(1.0), y: Some(2.0), z: None, e: None, f: None,
        role: ExtrusionRole::OuterWall,
    }));
    let mut arena = LayerArena::new();
    let err = slicer_host::commit_layer_outputs_for_test(
        "Layer::PathOptimization", "com.test.pathopt-bad", 0, &ctx, &mut arena,
    ).expect_err("Move override must fail with structured diagnostic");
    let msg = err.to_string();
    assert!(
        msg.contains("unsupported GCode command") || msg.contains("Layer::PathOptimization"),
        "diagnostic should identify the rejection cause: {msg}",
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

    let mut ctx = HostExecutionContext::new("com.test.pathopt-ann".to_string());
    ctx.gcode_output.commands.push(GcodeCommandCollected::Comment("hello".into()));
    ctx.gcode_output.commands.push(GcodeCommandCollected::Raw("M117 hi".into()));

    let mut arena = LayerArena::new();
    slicer_host::commit_layer_outputs_for_test(
        "Layer::PathOptimization", "com.test.pathopt-ann", 0, &ctx, &mut arena,
    ).expect("comment/raw must commit successfully");

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
        let mut c = HostExecutionContext::new("com.test.pathopt-det2".to_string());
        c.gcode_output.commands.push(GcodeCommandCollected::ToolChange { from_tool: 0, to_tool: 1 });
        c.gcode_output.commands.push(GcodeCommandCollected::Comment("a".into()));
        c.gcode_output.commands.push(GcodeCommandCollected::Raw("b".into()));
        c
    };

    let mut snapshots = Vec::new();
    for _ in 0..3 {
        let mut arena = LayerArena::new();
        let ctx = mk_ctx();
        slicer_host::commit_layer_outputs_for_test(
            "Layer::PathOptimization", "com.test.pathopt-det2", 0, &ctx, &mut arena,
        ).unwrap();
        snapshots.push((arena.take_deferred_tool_changes(), arena.take_deferred_annotations()));
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

    let mut ctx = HostExecutionContext::new("com.test.pathopt-zhop".to_string());
    ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop { after_entity_index: 0, hop_height: 0.5 });
    ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop { after_entity_index: 0, hop_height: 0.75 });

    let mut arena = LayerArena::new();
    slicer_host::commit_layer_outputs_for_test(
        "Layer::PathOptimization", "com.test.pathopt-zhop", 0, &ctx, &mut arena,
    ).expect("z-hop must commit");

    let zhops = arena.take_deferred_z_hops();
    assert_eq!(zhops.len(), 2);
    assert_eq!(zhops[0].after_entity_index, 0);
    assert_eq!(zhops[0].hop_height, 0.5);
    assert_eq!(zhops[1].hop_height, 0.75);
}

#[test]
fn path_optimization_z_hop_rejects_out_of_bounds_index() {
    use slicer_host::wit_host::{GcodeCommandCollected, HostExecutionContext};
    use slicer_ir::{LayerCollectionIR, SemVer};

    let mut ctx = HostExecutionContext::new("com.test.pathopt-zhop-oob".to_string());
    ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop { after_entity_index: 5, hop_height: 0.5 });

    let mut arena = LayerArena::new();
    // Pre-stage 2 entities directly into the LayerCollectionIR so the
    // commit path sees a non-empty ordered_entities (entity_count=2).
    let entity = slicer_ir::PrintEntity {
        path: slicer_ir::ExtrusionPath3D {
            points: vec![slicer_ir::Point3WithWidth { x: 0.0, y: 0.0, z: 0.2, width: 0.4, flow_factor: 1.0 }],
            role: slicer_ir::ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        role: slicer_ir::ExtrusionRole::OuterWall,
        region_key: slicer_ir::RegionKey { global_layer_index: 0, object_id: String::new(), region_id: 0 },
        topo_order: 0,
    };
    arena.set_layer_collection(LayerCollectionIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        global_layer_index: 0, z: 0.2,
        ordered_entities: vec![entity.clone(), entity],
        tool_changes: Vec::new(), z_hops: Vec::new(), annotations: Vec::new(),
    });

    let err = slicer_host::commit_layer_outputs_for_test(
        "Layer::PathOptimization", "com.test.pathopt-zhop-oob", 0, &ctx, &mut arena,
    ).expect_err("out-of-bounds z-hop must fail");
    let msg = err.to_string();
    assert!(msg.contains("after-entity-index=5"), "diagnostic should name field: {msg}");
    assert!(msg.contains("out of bounds"), "diagnostic should explain: {msg}");
}

#[test]
fn path_optimization_z_hop_rejects_invalid_hop_height() {
    use slicer_host::wit_host::{GcodeCommandCollected, HostExecutionContext};

    for bad in [0.0_f32, -1.0, f32::NAN, f32::INFINITY] {
        let mut ctx = HostExecutionContext::new("com.test.pathopt-zhop-bad".to_string());
        ctx.gcode_output.commands.push(GcodeCommandCollected::ZHop { after_entity_index: 0, hop_height: bad });
        let mut arena = LayerArena::new();
        let err = slicer_host::commit_layer_outputs_for_test(
            "Layer::PathOptimization", "com.test.pathopt-zhop-bad", 0, &ctx, &mut arena,
        ).expect_err("bad hop_height must fail");
        assert!(err.to_string().contains("hop-height"), "diagnostic should name field for {bad}: {err}");
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
                modules: vec![make_compiled_module_with("com.test.zhop-seed", "Layer::Perimeters", Arc::clone(&component))],
            },
            slicer_host::CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![make_compiled_module_with("com.test.zhop-e2e", "Layer::PathOptimization", Arc::clone(&component))],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0, z: 0.2, active_regions: Vec::new(),
            has_nonplanar: false, is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
    };

    struct SeedingRunner<'a> {
        inner: &'a slicer_host::WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl<'a> LayerStageRunner for SeedingRunner<'a> {
        fn run_stage(&self, stage_id: &StageId, layer: &GlobalLayer, module: &CompiledModule,
                     blackboard: &Blackboard, arena: &mut LayerArena)
                     -> Result<LayerStageOutput, LayerStageError> {
            if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    arena.set_perimeter(p).unwrap();
                    return Ok(LayerStageOutput::Success);
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
                modules: vec![make_compiled_module_with("com.test.zhop-emit-seed", "Layer::Perimeters", Arc::clone(&component))],
            },
            slicer_host::CompiledStage {
                stage_id: "Layer::PathOptimization".into(),
                modules: vec![make_compiled_module_with("com.test.zhop-emit", "Layer::PathOptimization", Arc::clone(&component))],
            },
        ],
        layer_finalization_stage: None,
        postpass_stages: Vec::new(),
        global_layers: Arc::new(vec![GlobalLayer {
            index: 0, z: 0.2, active_regions: Vec::new(),
            has_nonplanar: false, is_sync_layer: false,
        }]),
        region_plans: Arc::new(std::collections::HashMap::new()),
    };

    struct SeedingRunner<'a> {
        inner: &'a slicer_host::WasmRuntimeDispatcher,
        perim: Mutex<Option<slicer_ir::PerimeterIR>>,
    }
    impl<'a> LayerStageRunner for SeedingRunner<'a> {
        fn run_stage(&self, stage_id: &StageId, layer: &GlobalLayer, module: &CompiledModule,
                     blackboard: &Blackboard, arena: &mut LayerArena)
                     -> Result<LayerStageOutput, LayerStageError> {
            if stage_id == "Layer::Perimeters" && arena.perimeter().is_none() {
                if let Some(p) = self.perim.lock().unwrap().take() {
                    arena.set_perimeter(p).unwrap();
                    return Ok(LayerStageOutput::Success);
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
            if (*z - 0.7).abs() < 1e-4 { hop_lifts += 1; }
        }
    }
    assert!(hop_lifts >= 1, "default emitter must lift to layer.z + hop_height for committed z_hops");
}
