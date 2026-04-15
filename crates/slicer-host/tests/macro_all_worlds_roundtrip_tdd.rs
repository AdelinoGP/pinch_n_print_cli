//! TASK-109 round-trip witnesses for every supported WIT world.
//!
//! Each guest under `test-guests/sdk-*-guest/` is authored purely via
//! `#[slicer_module]` — no hand-rolled `wit_bindgen::generate!` or
//! `export!(Component)` is written in the guest. Each test loads the
//! corresponding `.component.wasm`, builds a `CompiledModule`, runs it
//! through `WasmRuntimeDispatcher`, and asserts that:
//!
//! 1. The dispatcher successfully invokes the macro-emitted typed export.
//! 2. The typed `ConfigView` arrives at the user's trait body (we prove
//!    this by declaring an `intentional_error_code` key that the trait
//!    body converts back into a typed `ModuleError` with a specific
//!    non-zero code; the test verifies the code round-trips).
//! 3. The `Result<_, ModuleError>` return value marshals back through
//!    the component boundary with its typed fields intact.
//! 4. Repeated calls are deterministic.
//!
//! Covers worlds: postpass, finalization, prepass, layer. If any world
//! still emits the placeholder `-> i32 { 0 }` shim instead of real
//! typed glue, the corresponding test fails (the constant return value
//! collides with or ignores the typed contract).

#![allow(missing_docs)]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use slicer_ir::{ConfigValue, ConfigView, GlobalLayer, StageId};
use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::{
    Blackboard, CompiledModule, FinalizationError, FinalizationOutput, FinalizationStageRunner,
    IrAccessMask, LoadedModule, PrepassExecutionError, PrepassStageOutput, PrepassStageRunner,
    WasmEngine, WasmRuntimeDispatcher,
};

fn semver(major: u32, minor: u32, patch: u32) -> slicer_ir::SemVer {
    slicer_ir::SemVer { major, minor, patch }
}

fn empty_mesh_ir() -> Arc<slicer_ir::MeshIR> {
    Arc::new(slicer_ir::MeshIR {
        schema_version: slicer_ir::SemVer { major: 1, minor: 0, patch: 0 },
        objects: Vec::new(),
        build_volume: slicer_ir::BoundingBox3 {
            min: slicer_ir::Point3 { x: 0.0, y: 0.0, z: 0.0 },
            max: slicer_ir::Point3 { x: 1.0, y: 1.0, z: 1.0 },
        },
    })
}

fn make_loaded_module(id: &str, stage: &str, wit_world: &str) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: semver(1, 0, 0),
        stage: stage.to_string(),
        wit_world: wit_world.to_string(),
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

fn guest_component_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-guests")
        .join(format!("{name}.component.wasm"))
}

fn load_guest(engine: &WasmEngine, name: &str) -> Arc<slicer_host::WasmComponent> {
    let path = guest_component_path(name);
    assert!(
        path.exists(),
        "guest component {name} missing at {}: rebuild via \
         test-guests/build-test-guests.sh",
        path.display()
    );
    let bytes = std::fs::read(&path).expect("read .component.wasm");
    Arc::new(engine.compile_component(&bytes).expect("compile component"))
}

fn make_module(
    module_id: &str,
    stage_id: &str,
    wit_world: &str,
    component: Arc<slicer_host::WasmComponent>,
    config: ConfigView,
) -> CompiledModule {
    let loaded = make_loaded_module(module_id, stage_id, wit_world);
    let pool = Arc::new(
        build_wasm_instance_pool(&loaded, 1, WasmArtifactMetadata { uses_shared_memory: false })
            .expect("build instance pool"),
    );
    CompiledModule {
        module_id: module_id.to_string(),
        instance_pool: pool,
        ir_read_mask: IrAccessMask { paths: Vec::new() },
        ir_write_mask: IrAccessMask { paths: Vec::new() },
        config_view: Arc::new(config),
        wasm_component: Some(component),
    }
}

fn intentional_error_config(code: i64) -> ConfigView {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert("intentional_error_code".to_string(), ConfigValue::Int(code));
    ConfigView::from_map(fields)
}

// ── Finalization world ────────────────────────────────────────────────

#[test]
fn finalization_world_macro_guest_round_trips_typed_config_and_result() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-finalization-guest");

    let module = make_module(
        "com.test.sdk-finalization",
        "PostPass::LayerFinalization",
        "slicer:world-finalization@1.0.0",
        component,
        intentional_error_config(0xF1),
    );

    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage: StageId = "PostPass::LayerFinalization".to_string();

    let err = FinalizationStageRunner::run_stage(
        &dispatcher,
        &stage,
        &module,
        &bb,
        &mut Vec::new(),
    )
    .expect_err("guest must surface typed ModuleError driven by config");
    // The typed error code set in config must reach the trait body and
    // marshal back through the component boundary.
    let msg = format!("{err}");
    assert!(
        msg.contains("241") || msg.contains("0xF1") || msg.contains("f1"),
        "error must carry the intentional code 0xF1 (241): {msg}"
    );
    assert!(msg.contains("sdk-finalization-guest"), "trait body must run: {msg}");
}

#[test]
fn finalization_world_macro_guest_succeeds_without_error_config() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-finalization-guest");
    let module = make_module(
        "com.test.sdk-finalization-ok",
        "PostPass::LayerFinalization",
        "slicer:world-finalization@1.0.0",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage: StageId = "PostPass::LayerFinalization".to_string();
    let out = FinalizationStageRunner::run_stage(
        &dispatcher,
        &stage,
        &module,
        &bb,
        &mut Vec::new(),
    )
    .expect("empty config path must succeed through real typed glue");
    assert!(matches!(out, FinalizationOutput::Success));
}

#[test]
fn finalization_world_macro_guest_is_deterministic() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-finalization-guest");
    let module = make_module(
        "com.test.sdk-finalization-det",
        "PostPass::LayerFinalization",
        "slicer:world-finalization@1.0.0",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage: StageId = "PostPass::LayerFinalization".to_string();
    for _ in 0..3 {
        let out = FinalizationStageRunner::run_stage(&dispatcher, &stage, &module, &bb, &mut Vec::new())
            .expect("deterministic success across repeated calls");
        assert!(matches!(out, FinalizationOutput::Success));
    }
}

// ── Prepass world (MeshAnalysis) ──────────────────────────────────────

#[test]
fn prepass_world_macro_guest_round_trips_typed_config_and_result() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-prepass-guest");

    let module = make_module(
        "com.test.sdk-prepass",
        "PrePass::MeshAnalysis",
        "slicer:world-prepass@1.0.0",
        component,
        intentional_error_config(0xE7),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage: StageId = "PrePass::MeshAnalysis".to_string();
    let err = PrepassStageRunner::run_stage(&dispatcher, &stage, &module, &bb)
        .expect_err("guest must surface typed ModuleError driven by config");
    let msg = format!("{err}");
    assert!(msg.contains("231") || msg.contains("0xE7") || msg.contains("e7"),
        "error must carry the intentional code 0xE7 (231): {msg}");
    assert!(msg.contains("sdk-prepass-guest"), "trait body must run: {msg}");
}

#[test]
fn prepass_world_macro_guest_succeeds_without_error_config() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-prepass-guest");
    let module = make_module(
        "com.test.sdk-prepass-ok",
        "PrePass::MeshAnalysis",
        "slicer:world-prepass@1.0.0",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage: StageId = "PrePass::MeshAnalysis".to_string();
    let out = PrepassStageRunner::run_stage(&dispatcher, &stage, &module, &bb)
        .expect("empty config path must succeed through real typed glue");
    assert!(matches!(out, PrepassStageOutput::None));
}

#[test]
fn prepass_world_macro_guest_is_deterministic() {
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-prepass-guest");
    let module = make_module(
        "com.test.sdk-prepass-det",
        "PrePass::MeshAnalysis",
        "slicer:world-prepass@1.0.0",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 0);
    let stage: StageId = "PrePass::MeshAnalysis".to_string();
    for _ in 0..3 {
        let out = PrepassStageRunner::run_stage(&dispatcher, &stage, &module, &bb)
            .expect("deterministic success across repeated calls");
        assert!(matches!(out, PrepassStageOutput::None));
    }
}

// ── Layer world (Infill) ──────────────────────────────────────────────

fn one_layer_arena() -> (slicer_host::LayerArena, GlobalLayer) {
    let layer = GlobalLayer {
        index: 5,
        z: 1.0,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    (slicer_host::LayerArena::new(), layer)
}

#[test]
fn layer_world_macro_guest_round_trips_typed_config_and_result() {
    use slicer_host::{LayerStageRunner, LayerStageError};
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-layer-infill-guest");

    let module = make_module(
        "com.test.sdk-layer-infill",
        "Layer::Infill",
        "slicer:world-layer@1.0.0",
        component,
        intentional_error_config(0xD5),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 1);
    let (mut arena, layer) = one_layer_arena();
    let stage: StageId = "Layer::Infill".to_string();

    let err = LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &module, &bb, &mut arena)
        .expect_err("guest must surface typed ModuleError driven by config");
    match err {
        LayerStageError::FatalModule { stage_id, module_id, message } => {
            assert_eq!(stage_id, "Layer::Infill");
            assert!(module_id.contains("sdk-layer-infill"), "module id preserved: {module_id}");
            assert!(
                message.contains("213") || message.contains("0xD5") || message.contains("d5"),
                "error carries intentional code 0xD5 (213): {message}"
            );
            assert!(
                message.contains("sdk-layer-infill-guest"),
                "trait body ran: {message}"
            );
            assert!(
                message.contains("layer 5"),
                "typed layer_index reached trait body: {message}"
            );
        }
        other => panic!("expected FatalModule, got {other:?}"),
    }
}

#[test]
fn layer_world_macro_guest_succeeds_without_error_config() {
    use slicer_host::LayerStageRunner;
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-layer-infill-guest");
    let module = make_module(
        "com.test.sdk-layer-infill-ok",
        "Layer::Infill",
        "slicer:world-layer@1.0.0",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 1);
    let (mut arena, layer) = one_layer_arena();
    let stage: StageId = "Layer::Infill".to_string();
    let _out = LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &module, &bb, &mut arena)
        .expect("empty config path must succeed through real typed glue");
}

#[test]
fn layer_world_macro_guest_is_deterministic() {
    use slicer_host::LayerStageRunner;
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-layer-infill-guest");
    let module = make_module(
        "com.test.sdk-layer-infill-det",
        "Layer::Infill",
        "slicer:world-layer@1.0.0",
        component,
        ConfigView::new(),
    );
    let bb = Blackboard::new(empty_mesh_ir(), 1);
    let stage: StageId = "Layer::Infill".to_string();
    for _ in 0..3 {
        let (mut arena, layer) = one_layer_arena();
        LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &module, &bb, &mut arena)
            .expect("deterministic success across repeated calls");
    }
}

// ── Blocker-#2 content witnesses ─────────────────────────────────────
// These prove the macro-emitted layer-world glue does real resource-level
// deep copy of wit-bindgen resources into SDK views *and* drains SDK
// builder contents back through the wit-bindgen builder boundary.
// Before TASK-layer-world-deep-copy these assertions would fail because
// the macro handed the trait an empty `Vec<SliceRegionView>` and
// dropped the SDK builder on the floor.

use slicer_ir::{ExPolygon, Point2, Polygon, SliceIR, SlicedRegion};

fn slice_ir_with_regions(
    layer_index: u32,
    z: f32,
    region_count: usize,
    polys_per_region: usize,
) -> SliceIR {
    let regions = (0..region_count)
        .map(|i| SlicedRegion {
            object_id: format!("sdk-obj-{i}"),
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
            infill_areas: (0..i + 1)
                .map(|_| ExPolygon {
                    contour: Polygon { points: vec![Point2 { x: 0, y: 0 }] },
                    holes: Vec::new(),
                })
                .collect(),
            nonplanar_surface: None,
            effective_layer_height: 0.2,
            boundary_paint: HashMap::new(),
        })
        .collect();
    SliceIR { schema_version: semver(1, 0, 0), global_layer_index: layer_index, z, regions }
}

#[test]
fn layer_world_macro_guest_sees_real_slice_region_content() {
    use slicer_host::{LayerArena, LayerStageRunner};
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-layer-infill-guest");

    // Drive the intentional-error path so the guest's error message
    // carries the observed region + polygon counts straight back
    // through the typed `ModuleError` boundary. If deep-copy IN works,
    // the message will say "regions=2, total_polygons=6".
    let module = make_module(
        "com.test.sdk-layer-infill-witness-in",
        "Layer::Infill",
        "slicer:world-layer@1.0.0",
        component,
        intentional_error_config(0xD6),
    );

    let bb = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 7,
        z: 1.4,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    let slice = slice_ir_with_regions(7, 1.4, 2, 3);
    arena.set_slice(slice).expect("commit slice ir");

    let stage: StageId = "Layer::Infill".to_string();
    let err = LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &module, &bb, &mut arena)
        .expect_err("intentional error must surface");
    let msg = format!("{err}");

    assert!(
        msg.contains("regions=2"),
        "typed deep copy IN must forward 2 slice regions to the SDK trait body: {msg}"
    );
    assert!(
        msg.contains("total_polygons=6"),
        "typed deep copy IN must forward all 6 polygons (2 regions × 3): {msg}"
    );
    assert!(
        msg.contains("layer 7"),
        "typed layer_index=7 must arrive at the trait body: {msg}"
    );
}

#[test]
fn layer_world_macro_guest_drain_back_reaches_arena_infill() {
    use slicer_host::{LayerArena, LayerStageRunner};
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-layer-infill-guest");

    let module = make_module(
        "com.test.sdk-layer-infill-witness-out",
        "Layer::Infill",
        "slicer:world-layer@1.0.0",
        component,
        ConfigView::new(),
    );

    let bb = Blackboard::new(empty_mesh_ir(), 1);
    let layer = GlobalLayer {
        index: 9,
        z: 1.8,
        active_regions: Vec::new(),
        has_nonplanar: false,
        is_sync_layer: false,
    };
    let mut arena = LayerArena::new();
    // 3 regions, 4 polygons each → total 12 polygons.
    let slice = slice_ir_with_regions(9, 1.8, 3, 4);
    arena.set_slice(slice).expect("commit slice ir");

    let stage: StageId = "Layer::Infill".to_string();
    LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &module, &bb, &mut arena)
        .expect("success path must succeed");

    // Drain-back proof: the SDK-side `push_sparse_path` the guest issued
    // must have been replayed through the wit-bindgen infill-output-builder
    // and committed onto the arena's InfillIR.
    let infill = arena
        .infill()
        .expect("drain-back must commit InfillIR into the arena");
    assert!(
        !infill.regions.is_empty(),
        "drain-back must produce at least one infill region; got: {:?}",
        infill.regions.len()
    );
    let path0 = &infill.regions[0].sparse_infill[0];
    let p0 = &path0.points[0];
    // Matches the guest's encoding:
    //   point[0].x = region_count = 3
    //   point[0].y = total_polys  = 12
    //   point[0].z = first region's z = 1.8
    //   point[0].width = first region's effective_layer_height = 0.2
    //   point[0].flow_factor = first region's infill_areas().len() = 1
    assert_eq!(p0.x, 3.0, "deep-copy witnessed 3 regions (got x={})", p0.x);
    assert_eq!(p0.y, 12.0, "deep-copy witnessed 12 polygons (got y={})", p0.y);
    assert!(
        (p0.z - 1.8).abs() < 1e-4,
        "deep-copy witnessed z=1.8 from SliceRegionView::z(): {}",
        p0.z
    );
    assert!(
        (p0.width - 0.2).abs() < 1e-4,
        "deep-copy witnessed effective_layer_height=0.2: {}",
        p0.width
    );
    assert!(
        (p0.flow_factor - 1.0).abs() < 1e-4,
        "deep-copy witnessed first region's infill_areas().len()=1: {}",
        p0.flow_factor
    );
    // Second point encodes the forwarded layer_index.
    let p1 = &path0.points[1];
    assert_eq!(p1.x, 9.0, "typed layer_index=9 forwarded to trait body: x={}", p1.x);
}

#[test]
fn layer_world_macro_guest_deep_copy_is_deterministic() {
    use slicer_host::{LayerArena, LayerStageRunner};
    let engine = Arc::new(WasmEngine::new());
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = load_guest(&engine, "sdk-layer-infill-guest");

    let module = make_module(
        "com.test.sdk-layer-infill-det-content",
        "Layer::Infill",
        "slicer:world-layer@1.0.0",
        component,
        ConfigView::new(),
    );
    let stage: StageId = "Layer::Infill".to_string();
    let bb = Blackboard::new(empty_mesh_ir(), 1);

    let mut snapshots: Vec<(f32, f32, f32, f32, f32)> = Vec::new();
    for _ in 0..3 {
        let layer = GlobalLayer {
            index: 2,
            z: 0.4,
            active_regions: Vec::new(),
            has_nonplanar: false,
            is_sync_layer: false,
        };
        let mut arena = LayerArena::new();
        arena.set_slice(slice_ir_with_regions(2, 0.4, 2, 5)).unwrap();
        LayerStageRunner::run_stage(&dispatcher, &stage, &layer, &module, &bb, &mut arena).unwrap();
        let p = &arena.infill().unwrap().regions[0].sparse_infill[0].points[0];
        snapshots.push((p.x, p.y, p.z, p.width, p.flow_factor));
    }
    assert!(
        snapshots.windows(2).all(|w| w[0] == w[1]),
        "deep-copy IN + drain-back OUT must be deterministic: {:?}",
        snapshots
    );
}
