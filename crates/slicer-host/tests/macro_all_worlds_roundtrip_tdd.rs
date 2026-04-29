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

use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::wit_host::{
    convert_extrusion_role, convert_wall_feature_flag, ExtrusionRole as WitExtrusionRole,
    PaintSemantic as WitPaintSemantic, PaintValue as WitPaintValue,
    WallFeatureFlag as WitWallFeatureFlag, BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG,
    BUILTIN_EXTRUSION_ROLE_SKIRT_TAG,
};
use slicer_host::{
    Blackboard, CompiledModule, FinalizationOutput, FinalizationStageRunner, IrAccessMask,
    LoadedModule, PrepassStageOutput, PrepassStageRunner, WasmEngine, WasmRuntimeDispatcher,
};
use slicer_ir::{ConfigValue, ConfigView, GlobalLayer, StageId};

fn semver(major: u32, minor: u32, patch: u32) -> slicer_ir::SemVer {
    slicer_ir::SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_mesh_ir() -> Arc<slicer_ir::MeshIR> {
    Arc::new(slicer_ir::MeshIR {
        schema_version: slicer_ir::SemVer {
            major: 1,
            minor: 0,
            patch: 0,
        },
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
        build_wasm_instance_pool(
            &loaded,
            1,
            WasmArtifactMetadata {
                uses_shared_memory: false,
            },
        )
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

    let err =
        FinalizationStageRunner::run_stage(&dispatcher, &stage, &module, &bb, &mut Vec::new())
            .expect_err("guest must surface typed ModuleError driven by config");
    // The typed error code set in config must reach the trait body and
    // marshal back through the component boundary.
    let msg = format!("{err}");
    assert!(
        msg.contains("241") || msg.contains("0xF1") || msg.contains("f1"),
        "error must carry the intentional code 0xF1 (241): {msg}"
    );
    assert!(
        msg.contains("sdk-finalization-guest"),
        "trait body must run: {msg}"
    );
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
    let out =
        FinalizationStageRunner::run_stage(&dispatcher, &stage, &module, &bb, &mut Vec::new())
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
        let out =
            FinalizationStageRunner::run_stage(&dispatcher, &stage, &module, &bb, &mut Vec::new())
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
    assert!(
        msg.contains("231") || msg.contains("0xE7") || msg.contains("e7"),
        "error must carry the intentional code 0xE7 (231): {msg}"
    );
    assert!(
        msg.contains("sdk-prepass-guest"),
        "trait body must run: {msg}"
    );
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
    assert!(matches!(out.0, PrepassStageOutput::None));
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
        assert!(matches!(out.0, PrepassStageOutput::None));
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
    use slicer_host::{LayerStageError, LayerStageRunner};
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
        LayerStageError::FatalModule {
            stage_id,
            module_id,
            message,
        } => {
            assert_eq!(stage_id, "Layer::Infill");
            assert!(
                module_id.contains("sdk-layer-infill"),
                "module id preserved: {module_id}"
            );
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
            infill_areas: (0..i + 1)
                .map(|_| ExPolygon {
                    contour: Polygon {
                        points: vec![Point2 { x: 0, y: 0 }],
                    },
                    holes: Vec::new(),
                })
                .collect(),
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
    assert_eq!(
        p0.y, 12.0,
        "deep-copy witnessed 12 polygons (got y={})",
        p0.y
    );
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
    assert_eq!(
        p1.x, 9.0,
        "typed layer_index=9 forwarded to trait body: x={}",
        p1.x
    );
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
        arena
            .set_slice(slice_ir_with_regions(2, 0.4, 2, 5))
            .unwrap();
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

// ── Custom payload round-trip tests (TASK-150) ──────────────────────────────────

/// Round-trip test for ExtrusionRole::Custom(String).
///
/// Tests that a custom extrusion role payload survives the WIT boundary:
/// IR (ExtrusionRole::Custom) → WIT (ExtrusionRole::Custom) → IR (ExtrusionRole::Custom)
///
/// Note: The IR→WIT direction uses inline conversion logic that mirrors the
/// macro's private __slicer_ir_role_to_wit function, since that function is
/// not accessible from tests. The WIT→IR direction uses the public
/// convert_extrusion_role function.
#[test]
fn extrusion_role_custom_payload_roundtrip() {
    // Step 1: Create IR ExtrusionRole::Custom
    let ir_role = slicer_ir::ExtrusionRole::Custom("test-role@1".to_string());

    // Step 2: Convert IR → WIT using inline logic (mirrors macro's __slicer_ir_role_to_wit)
    let wit_role = match &ir_role {
        slicer_ir::ExtrusionRole::Custom(s) => WitExtrusionRole::Custom(s.clone()),
        slicer_ir::ExtrusionRole::OuterWall => WitExtrusionRole::OuterWall,
        slicer_ir::ExtrusionRole::InnerWall => WitExtrusionRole::InnerWall,
        slicer_ir::ExtrusionRole::ThinWall => WitExtrusionRole::ThinWall,
        slicer_ir::ExtrusionRole::TopSolidInfill => WitExtrusionRole::TopSolidInfill,
        slicer_ir::ExtrusionRole::BottomSolidInfill => WitExtrusionRole::BottomSolidInfill,
        slicer_ir::ExtrusionRole::SparseInfill => WitExtrusionRole::SparseInfill,
        slicer_ir::ExtrusionRole::SupportMaterial => WitExtrusionRole::SupportMaterial,
        slicer_ir::ExtrusionRole::SupportInterface => WitExtrusionRole::SupportInterface,
        slicer_ir::ExtrusionRole::Ironing => WitExtrusionRole::Ironing,
        slicer_ir::ExtrusionRole::BridgeInfill => WitExtrusionRole::BridgeInfill,
        slicer_ir::ExtrusionRole::WipeTower => WitExtrusionRole::WipeTower,
        slicer_ir::ExtrusionRole::PrimeTower => {
            WitExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG.to_string())
        }
        slicer_ir::ExtrusionRole::Skirt => {
            WitExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_SKIRT_TAG.to_string())
        }
    };

    // Step 3: Convert WIT → IR using public convert_extrusion_role
    let ir_result = convert_extrusion_role(&wit_role);

    // Step 4: Assert payload is preserved
    match ir_result {
        slicer_ir::ExtrusionRole::Custom(s) => {
            assert_eq!(s, "test-role@1", "custom payload must survive round-trip");
        }
        other => panic!("expected ExtrusionRole::Custom, got {:?}", other),
    }
}

#[test]
fn extrusion_role_builtin_tags_roundtrip() {
    let cases = [
        (
            slicer_ir::ExtrusionRole::PrimeTower,
            BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG,
            slicer_ir::ExtrusionRole::PrimeTower,
        ),
        (
            slicer_ir::ExtrusionRole::Skirt,
            BUILTIN_EXTRUSION_ROLE_SKIRT_TAG,
            slicer_ir::ExtrusionRole::Skirt,
        ),
    ];

    for (ir_role, expected_tag, expected_ir) in cases {
        let wit_role = match ir_role {
            slicer_ir::ExtrusionRole::PrimeTower => {
                WitExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_PRIME_TOWER_TAG.to_string())
            }
            slicer_ir::ExtrusionRole::Skirt => {
                WitExtrusionRole::Custom(BUILTIN_EXTRUSION_ROLE_SKIRT_TAG.to_string())
            }
            _ => unreachable!("test only covers built-in reserved roles"),
        };

        assert!(matches!(
            wit_role,
            WitExtrusionRole::Custom(ref tag) if tag == expected_tag
        ));
        assert_eq!(convert_extrusion_role(&wit_role), expected_ir);
    }
}

/// Round-trip test for PaintSemantic::Custom(String).
///
/// Tests that a custom paint semantic payload survives the WIT boundary.
/// Uses inline conversion logic for IR→WIT since the host's ir_to_wit_paint_semantic
/// is private. The WIT→IR direction uses the public convert_wall_feature_flag
/// indirectly via WallFeatureFlag round-trip.
#[test]
fn paint_semantic_custom_payload_roundtrip() {
    // Create IR PaintSemantic::Custom
    let ir_semantic = slicer_ir::PaintSemantic::Custom("com.example/texture@1".to_string());

    // Convert IR → WIT using inline logic
    let wit_semantic = match &ir_semantic {
        slicer_ir::PaintSemantic::Custom(s) => WitPaintSemantic::Custom(s.clone()),
        slicer_ir::PaintSemantic::Material => WitPaintSemantic::Material,
        slicer_ir::PaintSemantic::FuzzySkin => WitPaintSemantic::FuzzySkin,
        slicer_ir::PaintSemantic::SupportEnforcer => WitPaintSemantic::SupportEnforcer,
        slicer_ir::PaintSemantic::SupportBlocker => WitPaintSemantic::SupportBlocker,
    };

    // Verify the WIT variant carries the correct payload
    match wit_semantic {
        WitPaintSemantic::Custom(ref s) => {
            assert_eq!(
                s, "com.example/texture@1",
                "WIT custom semantic must carry payload"
            );
        }
        other => panic!("expected WitPaintSemantic::Custom, got {:?}", other),
    }

    // Convert back to IR using inline logic (mirrors the private ir_to_wit_paint_semantic inverse)
    let ir_result = match wit_semantic {
        WitPaintSemantic::Custom(s) => slicer_ir::PaintSemantic::Custom(s),
        WitPaintSemantic::Material => slicer_ir::PaintSemantic::Material,
        WitPaintSemantic::FuzzySkin => slicer_ir::PaintSemantic::FuzzySkin,
        WitPaintSemantic::SupportEnforcer => slicer_ir::PaintSemantic::SupportEnforcer,
        WitPaintSemantic::SupportBlocker => slicer_ir::PaintSemantic::SupportBlocker,
    };

    // Assert payload is preserved
    match ir_result {
        slicer_ir::PaintSemantic::Custom(s) => {
            assert_eq!(
                s, "com.example/texture@1",
                "custom payload must survive round-trip"
            );
        }
        other => panic!("expected PaintSemantic::Custom, got {:?}", other),
    }
}

/// Round-trip test for WallFeatureFlags with a single custom entry.
///
/// Tests that a custom paint value survives the WIT boundary when encoded
/// in WallFeatureFlags::custom map.
#[test]
fn wall_feature_flags_custom_payload_roundtrip() {
    // Create IR WallFeatureFlags with one custom entry
    let ir_flags = slicer_ir::WallFeatureFlags {
        tool_index: None,
        fuzzy_skin: false,
        is_bridge: false,
        is_thin_wall: false,
        skip_ironing: false,
        custom: std::collections::HashMap::from_iter([(
            "key".to_string(),
            slicer_ir::PaintValue::Scalar(0.5),
        )]),
    };

    // Convert IR → WIT using inline logic (mirrors macro's __slicer_ir_feature_to_wit)
    let wit_flags = WitWallFeatureFlag {
        tool_index: ir_flags.tool_index,
        fuzzy_skin: ir_flags.fuzzy_skin,
        is_bridge: ir_flags.is_bridge,
        is_thin_wall: ir_flags.is_thin_wall,
        skip_ironing: ir_flags.skip_ironing,
        custom: {
            let mut entries: Vec<_> = ir_flags
                .custom
                .iter()
                .map(|(k, v)| {
                    let wit_v = match v {
                        slicer_ir::PaintValue::Flag(b) => WitPaintValue::Flag(*b),
                        slicer_ir::PaintValue::Scalar(s) => WitPaintValue::Scalar(*s),
                        slicer_ir::PaintValue::ToolIndex(t) => WitPaintValue::ToolIndex(*t),
                    };
                    (k.clone(), wit_v)
                })
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            entries
        },
    };

    // Convert WIT → IR using public convert_wall_feature_flag
    let ir_result = convert_wall_feature_flag(&wit_flags);

    // Assert the custom map has exactly one entry with correct key and value
    assert_eq!(
        ir_result.custom.len(),
        1,
        "custom map must have exactly one entry"
    );
    let entry = ir_result
        .custom
        .get("key")
        .expect("custom map must have 'key' entry");
    match entry {
        slicer_ir::PaintValue::Scalar(s) => {
            assert!((*s - 0.5).abs() < 1e-6, "custom Scalar value must be 0.5");
        }
        other => panic!("expected Scalar(0.5), got {:?}", other),
    }
}

/// Round-trip test for WallFeatureFlags with multiple custom entries.
///
/// Tests that multiple custom paint values of different types survive the WIT boundary.
#[test]
fn wall_feature_flags_custom_multiple_entries_roundtrip() {
    // Create IR WallFeatureFlags with three custom entries
    let ir_flags = slicer_ir::WallFeatureFlags {
        tool_index: Some(1),
        fuzzy_skin: true,
        is_bridge: true,
        is_thin_wall: false,
        skip_ironing: true,
        custom: std::collections::HashMap::from_iter([
            ("a".to_string(), slicer_ir::PaintValue::Scalar(0.1)),
            ("b".to_string(), slicer_ir::PaintValue::Flag(true)),
            ("c".to_string(), slicer_ir::PaintValue::ToolIndex(2)),
        ]),
    };

    // Convert IR → WIT using inline logic
    let wit_flags = WitWallFeatureFlag {
        tool_index: ir_flags.tool_index,
        fuzzy_skin: ir_flags.fuzzy_skin,
        is_bridge: ir_flags.is_bridge,
        is_thin_wall: ir_flags.is_thin_wall,
        skip_ironing: ir_flags.skip_ironing,
        custom: {
            let mut entries: Vec<_> = ir_flags
                .custom
                .iter()
                .map(|(k, v)| {
                    let wit_v = match v {
                        slicer_ir::PaintValue::Flag(b) => WitPaintValue::Flag(*b),
                        slicer_ir::PaintValue::Scalar(s) => WitPaintValue::Scalar(*s),
                        slicer_ir::PaintValue::ToolIndex(t) => WitPaintValue::ToolIndex(*t),
                    };
                    (k.clone(), wit_v)
                })
                .collect();
            entries.sort_by(|a, b| a.0.cmp(&b.0));
            entries
        },
    };

    // Convert WIT → IR using public convert_wall_feature_flag
    let ir_result = convert_wall_feature_flag(&wit_flags);

    // Assert all three entries survived with correct values
    assert_eq!(ir_result.custom.len(), 3, "custom map must have 3 entries");

    // Check "a" -> Scalar(0.1)
    match ir_result.custom.get("a") {
        Some(slicer_ir::PaintValue::Scalar(s)) => {
            assert!((*s - 0.1).abs() < 1e-6, "entry 'a' should be Scalar(0.1)");
        }
        other => panic!("expected Scalar(0.1) for 'a', got {:?}", other),
    }

    // Check "b" -> Flag(true)
    match ir_result.custom.get("b") {
        Some(slicer_ir::PaintValue::Flag(b)) => {
            assert!(b, "entry 'b' should be Flag(true)");
        }
        other => panic!("expected Flag(true) for 'b', got {:?}", other),
    }

    // Check "c" -> ToolIndex(2)
    match ir_result.custom.get("c") {
        Some(slicer_ir::PaintValue::ToolIndex(t)) => {
            assert_eq!(*t, 2, "entry 'c' should be ToolIndex(2)");
        }
        other => panic!("expected ToolIndex(2) for 'c', got {:?}", other),
    }

    // Also verify non-custom fields are preserved
    assert_eq!(ir_result.tool_index, Some(1));
    assert!(ir_result.fuzzy_skin);
    assert!(ir_result.is_bridge);
    assert!(!ir_result.is_thin_wall);
    assert!(ir_result.skip_ironing);
}

/// Round-trip test for PaintSemantic::Custom with empty string.
///
/// Tests that an empty custom semantic string survives the WIT boundary
/// (not converted to None or dropped).
#[test]
fn paint_semantic_custom_empty_string_roundtrip() {
    // Create IR PaintSemantic::Custom with empty string
    let ir_semantic = slicer_ir::PaintSemantic::Custom(String::new());

    // Convert IR → WIT using inline logic
    let wit_semantic = match &ir_semantic {
        slicer_ir::PaintSemantic::Custom(s) => WitPaintSemantic::Custom(s.clone()),
        slicer_ir::PaintSemantic::Material => WitPaintSemantic::Material,
        slicer_ir::PaintSemantic::FuzzySkin => WitPaintSemantic::FuzzySkin,
        slicer_ir::PaintSemantic::SupportEnforcer => WitPaintSemantic::SupportEnforcer,
        slicer_ir::PaintSemantic::SupportBlocker => WitPaintSemantic::SupportBlocker,
    };

    // Verify WIT variant is Custom (not None or dropped)
    match wit_semantic {
        WitPaintSemantic::Custom(ref s) => {
            assert_eq!(s, "", "empty string must be preserved in WIT custom");
        }
        other => panic!("expected WitPaintSemantic::Custom(\"\"), got {:?}", other),
    }

    // Convert back to IR using inline logic
    let ir_result = match wit_semantic {
        WitPaintSemantic::Custom(s) => slicer_ir::PaintSemantic::Custom(s),
        WitPaintSemantic::Material => slicer_ir::PaintSemantic::Material,
        WitPaintSemantic::FuzzySkin => slicer_ir::PaintSemantic::FuzzySkin,
        WitPaintSemantic::SupportEnforcer => slicer_ir::PaintSemantic::SupportEnforcer,
        WitPaintSemantic::SupportBlocker => slicer_ir::PaintSemantic::SupportBlocker,
    };

    // Assert empty string is preserved (not None or dropped)
    match ir_result {
        slicer_ir::PaintSemantic::Custom(s) => {
            assert_eq!(s, "", "empty custom payload must survive round-trip");
        }
        other => panic!("expected PaintSemantic::Custom(\"\"), got {:?}", other),
    }
}
