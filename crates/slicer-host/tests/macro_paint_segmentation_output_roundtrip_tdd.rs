//! TDD harness for PaintSegmentation macro-arm output drain round-trip (Packet-43).
//!
//! Tests in this file are RED today:
//! - Source-level grep tests fail because the macro arm not yet drained / comments not yet removed.
//! - Dispatch tests fail because sdk-prepass-guest does not yet emit paint fixtures.
//! - Docs tests fail because docs/07 and DEVIATION_LOG are not yet updated.
//!
//! After Step 2 (drain) + Step 3 (guest fixtures) + Step 7 (docs), all tests should be GREEN.
//!
//! Verification: cargo test -p slicer-host --test macro_paint_segmentation_output_roundtrip_tdd

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::dispatch::WasmRuntimeDispatcher;
use slicer_host::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_host::manifest::LoadedModule;
use slicer_host::{Blackboard, CompiledModule, IrAccessMask, PrepassStageRunner, WasmEngine};
use slicer_ir::{
    BoundingBox3, ConfigValue, ConfigView, IndexedTriangleSet, MeshIR, ObjectConfig, ObjectMesh,
    PaintSemantic, PaintValue, Point3, SemVer, Transform3d,
};

// ── Path to the sdk-prepass-guest component ───────────────────────────────────

const SDK_PREPASS_GUEST_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../test-guests/sdk-prepass-guest.component.wasm"
);

// ── Harness helpers ───────────────────────────────────────────────────────────

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity_transform() -> Transform3d {
    Transform3d {
        matrix: [
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ],
    }
}

fn minimal_object(id: &str) -> ObjectMesh {
    ObjectMesh {
        id: id.to_string(),
        mesh: IndexedTriangleSet {
            vertices: vec![
                Point3 {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3 {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                },
            ],
            indices: vec![0, 1, 2],
        },
        transform: identity_transform(),
        config: ObjectConfig {
            data: HashMap::new(),
        },
        modifier_volumes: Vec::new(),
        paint_data: None,
        world_z_extent: None,
    }
}

fn blackboard_with_objects(object_ids: &[&str]) -> Blackboard {
    let objects: Vec<ObjectMesh> = object_ids.iter().map(|id| minimal_object(id)).collect();
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

fn make_loaded_module(id: &str, stage: &str) -> LoadedModule {
    LoadedModule {
        id: id.to_string(),
        version: semver(1, 0, 0),
        stage: stage.to_string(),
        wit_world: "slicer:world-prepass@1.0.0".to_string(),
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
        claims: Vec::new(),
        wasm_component: Some(component),
    }
}

fn load_sdk_prepass_guest(engine: &WasmEngine) -> Option<Arc<slicer_host::WasmComponent>> {
    let path = std::path::Path::new(SDK_PREPASS_GUEST_PATH);
    if !path.exists() {
        return None;
    }
    let bytes = std::fs::read(path).expect("read sdk-prepass-guest.component.wasm");
    match engine.compile_component(&bytes) {
        Ok(c) => Some(Arc::new(c)),
        Err(e) => panic!("failed to compile sdk-prepass-guest: {e}"),
    }
}

/// Build a ConfigView with a `fixture_case` key for driving guest fixture branches.
/// Step 3 guest will dispatch on this key to emit the right fixture data.
fn fixture_config(case: &str) -> ConfigView {
    let mut m = HashMap::new();
    m.insert(
        "fixture_case".to_string(),
        ConfigValue::String(case.to_string()),
    );
    ConfigView::from_map(m)
}

// ── AC-1: source-level drain string grep ─────────────────────────────────────

/// AC-1: The PaintSegmentation macro arm body in slicer-macros/src/lib.rs must
/// contain the drain strings `sdk_output.regions()`, `_output.push_paint_region`,
/// and `ModuleError { code: 10, fatal: true }`.
///
/// RED today: the arm body does not yet have these strings (Step 2 will add them).
#[test]
fn macro_arm_drains_regions_to_wit() {
    let src = include_str!("../../slicer-macros/src/lib.rs");

    // Find the PaintSegmentation arm body.
    let sentinel = "PrePass::PaintSegmentation";
    let arm_start = src
        .find(sentinel)
        .expect("slicer-macros must contain PrePass::PaintSegmentation arm sentinel");

    // Bound the arm: take the next 4000 chars as a proxy for the arm body.
    let arm_body = &src[arm_start..arm_start + src[arm_start..].len().min(4000)];

    assert!(
        arm_body.contains("sdk_output.regions()"),
        "PaintSegmentation arm must call sdk_output.regions() to drain; arm snippet:\n{}",
        &arm_body[..arm_body.len().min(500)]
    );
    assert!(
        arm_body.contains("_output.push_paint_region"),
        "PaintSegmentation arm must call _output.push_paint_region; arm snippet:\n{}",
        &arm_body[..arm_body.len().min(500)]
    );
    assert!(
        arm_body.contains("ModuleError { code: 10, fatal: true }"),
        "PaintSegmentation arm must surface ModuleError{{ code: 10, fatal: true }} on push failure"
    );
}

// ── AC-2: hole-bearing typed value round-trip ─────────────────────────────────

/// AC-2: Dispatch PaintSegmentation with fixture_case="hole_bearing".
/// The guest (after Step 3) emits a region on layer 3, semantic=Material,
/// object_id="obj-a", ToolIndex(7), with a polygon that has 1 hole.
///
/// Config contract for Step 3: `fixture_case = "hole_bearing"` →
///   push_paint_region(layer=3, semantic="material", object_id="obj-a",
///     value=ToolIndex(7), polygon with holes.len()==1)
///
/// RED today: guest emits nothing → per_layer is empty.
#[test]
fn hole_bearing_typed_value_round_trips() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_sdk_prepass_guest(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: sdk-prepass-guest.component.wasm missing");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module_with_config(
        "com.test.paint-seg-hole",
        "PrePass::PaintSegmentation",
        component,
        fixture_config("hole_bearing"),
    );
    let blackboard = blackboard_with_objects(&["obj-a"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::PaintSegmentation".to_string(),
        &module,
        &blackboard,
    );

    let ir = match result {
        Ok((PrepassStageOutput::PaintRegions(ir), _)) => ir,
        Ok((PrepassStageOutput::None, _)) => {
            panic!("AC-2 FAIL: got None — guest did not emit any paint regions (Step 3 not done)");
        }
        Ok((other, _)) => panic!(
            "AC-2 FAIL: unexpected variant {:?}",
            std::mem::discriminant(&other)
        ),
        Err(e) => panic!("AC-2 FAIL: dispatch error: {e}"),
    };

    let layer_map = ir
        .per_layer
        .get(&3)
        .expect("AC-2: per_layer must contain layer index 3");
    let regions = layer_map
        .semantic_regions
        .get(&PaintSemantic::Material)
        .expect("AC-2: layer 3 must have Material semantic");
    assert!(
        !regions.is_empty(),
        "AC-2: Material regions must be non-empty"
    );
    let region = regions
        .iter()
        .find(|r| r.object_id == "obj-a")
        .expect("AC-2: must find region for obj-a");
    assert_eq!(
        region.polygons[0].holes.len(),
        1,
        "AC-2: polygon[0] must have 1 hole"
    );
    assert_eq!(
        region.value,
        PaintValue::ToolIndex(7),
        "AC-2: value must be ToolIndex(7)"
    );
    assert_eq!(region.object_id, "obj-a", "AC-2: object_id must be obj-a");
}

// ── AC-3: custom semantic + custom value round-trip ───────────────────────────

/// AC-3: Dispatch PaintSegmentation with fixture_case="custom_payload".
/// The guest emits a region with PaintSemantic::Custom("my_profile"),
/// value=PaintValue::Custom("profile_high"), object_id="obj-a".
///
/// Config contract for Step 3: `fixture_case = "custom_payload"` →
///   push_paint_region(semantic=Custom("my_profile"), value=Custom("profile_high"),
///     object_id="obj-a", layer=0)
///
/// RED today: guest emits nothing.
#[test]
fn custom_semantic_and_custom_value_round_trip() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_sdk_prepass_guest(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: sdk-prepass-guest.component.wasm missing");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module_with_config(
        "com.test.paint-seg-custom",
        "PrePass::PaintSegmentation",
        component,
        fixture_config("custom_payload"),
    );
    let blackboard = blackboard_with_objects(&["obj-a"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::PaintSegmentation".to_string(),
        &module,
        &blackboard,
    );

    let ir = match result {
        Ok((PrepassStageOutput::PaintRegions(ir), _)) => ir,
        Ok((PrepassStageOutput::None, _)) => {
            panic!("AC-3 FAIL: got None — guest did not emit (Step 3 not done)");
        }
        Ok((other, _)) => panic!(
            "AC-3 FAIL: unexpected variant {:?}",
            std::mem::discriminant(&other)
        ),
        Err(e) => panic!("AC-3 FAIL: dispatch error: {e}"),
    };

    let custom_key = PaintSemantic::Custom("my_profile".into());
    let layer_map = ir
        .per_layer
        .get(&0)
        .expect("AC-3: per_layer must contain layer 0");
    let regions = layer_map
        .semantic_regions
        .get(&custom_key)
        .expect("AC-3: must find Custom(my_profile) semantic");
    let region = regions
        .iter()
        .find(|r| r.object_id == "obj-a")
        .expect("AC-3: must find region for obj-a");

    // value must NOT be ToolIndex(0)
    assert_ne!(
        region.value,
        PaintValue::ToolIndex(0),
        "AC-3: value must not be ToolIndex(0)"
    );
    match &region.value {
        PaintValue::Custom(s) => assert_eq!(
            s, "profile_high",
            "AC-3: Custom payload must be 'profile_high', got '{s}'"
        ),
        other => panic!("AC-3: expected PaintValue::Custom, got {:?}", other),
    }
    // Verify key string preserved verbatim
    match &custom_key {
        PaintSemantic::Custom(k) => assert_eq!(k, "my_profile"),
        _ => unreachable!(),
    }
}

// ── AC-5: push failure surfaces as fatal module error ────────────────────────

/// AC-5: Dispatch PaintSegmentation with fixture_case="empty_polygons".
/// The guest emits a region with empty `polygons: vec![]`.
/// The host validator must reject and surface a fatal error.
///
/// Config contract for Step 3: `fixture_case = "empty_polygons"` →
///   push_paint_region(..., polygons=[]) — should fail validation.
///
/// RED today: guest emits nothing, so dispatch returns None (no Err yet).
#[test]
fn push_failure_surfaces_as_fatal_module_error() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_sdk_prepass_guest(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: sdk-prepass-guest.component.wasm missing");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module_with_config(
        "com.test.paint-seg-empty-poly",
        "PrePass::PaintSegmentation",
        component,
        fixture_config("empty_polygons"),
    );
    let blackboard = blackboard_with_objects(&["obj-a"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::PaintSegmentation".to_string(),
        &module,
        &blackboard,
    );

    // After Step 2+3: dispatch must return Err with FatalModule discriminant.
    // RED today: result is Ok(None) because guest emits nothing.
    match result {
        Err(e) => {
            let dbg = format!("{:?}", e);
            assert!(
                dbg.contains("FatalModule"),
                "AC-5: error must be FatalModule, got: {dbg}"
            );
        }
        Ok((PrepassStageOutput::None, _)) => {
            panic!(
                "AC-5 FAIL (RED): got Ok(None) — guest did not emit empty-polygon region yet \
                 (Step 3 must add empty_polygons fixture)"
            );
        }
        Ok((other, _)) => panic!(
            "AC-5 FAIL: expected Err(FatalModule) but got Ok({:?})",
            std::mem::discriminant(&other)
        ),
    }
}

// ── AC-6: legacy comment block removed ───────────────────────────────────────

/// AC-6: The legacy disconnect comment block must be removed from
/// crates/slicer-macros/src/lib.rs before packet closure.
///
/// RED today: the legacy comment is still present.
#[test]
fn legacy_comment_block_removed() {
    let src = include_str!("../../slicer-macros/src/lib.rs");
    assert!(
        !src.contains("Same disconnect as MeshSegmentation"),
        "AC-6 FAIL: legacy comment 'Same disconnect as MeshSegmentation' still present in \
         slicer-macros/src/lib.rs — remove it as part of the drain implementation"
    );
    assert!(
        !src.contains("the SDK PaintSegmentationOutput builder operates on an in-Rust tree"),
        "AC-6 FAIL: legacy comment about 'in-Rust tree disconnect' still present — remove it"
    );
}

// ── AC-7: docs/07 TASK-130 cluster marked done ───────────────────────────────

/// AC-7: docs/07_implementation_status.md must show TASK-130, TASK-130a, TASK-130b as [x].
/// The blocker list section must NOT reference TASK-130a or TASK-130b.
///
/// RED today: TASK-130 cluster not yet done.
#[test]
fn docs_07_marks_130_cluster_done() {
    let src = include_str!("../../../docs/07_implementation_status.md");

    // Each TASK-130 row must be [x].
    for task in &["TASK-130", "TASK-130a", "TASK-130b"] {
        // Find the line containing this task ID.
        let line = src
            .lines()
            .find(|l| l.contains(task))
            .unwrap_or_else(|| panic!("AC-7: {task} line not found in docs/07"));
        assert!(
            line.contains("[x]"),
            "AC-7 FAIL: {task} line not marked [x]: '{line}'"
        );
    }

    // Blocker section must not list TASK-130a or TASK-130b.
    let blocker_section_start = src
        .find("blocker")
        .or_else(|| src.find("Blocker"))
        .or_else(|| src.find("BLOCKER"));
    if let Some(start) = blocker_section_start {
        let blocker_slice = &src[start..start + src[start..].len().min(2000)];
        assert!(
            !blocker_slice.contains("TASK-130a"),
            "AC-7 FAIL: TASK-130a still referenced in blocker section"
        );
        assert!(
            !blocker_slice.contains("TASK-130b"),
            "AC-7 FAIL: TASK-130b still referenced in blocker section"
        );
    }
}

// ── AC-8: DEV-025 fully closed ───────────────────────────────────────────────

/// AC-8: docs/DEVIATION_LOG.md must show DEV-025 mismatch-3 closed-by-Packet-43
/// and the DEV-025 overall status must be `closed`.
///
/// RED today: not yet updated.
#[test]
fn dev_025_fully_closed() {
    let src = include_str!("../../../docs/DEVIATION_LOG.md");

    // Find DEV-025 section.
    let dev025_start = src
        .find("DEV-025")
        .expect("AC-8: DEV-025 must exist in DEVIATION_LOG.md");
    let dev025_slice = &src[dev025_start..dev025_start + src[dev025_start..].len().min(3000)];

    assert!(
        dev025_slice.contains("closed-by-Packet-43"),
        "AC-8 FAIL: DEV-025 mismatch-3 must show 'closed-by-Packet-43'"
    );
    assert!(
        dev025_slice.contains("status") && dev025_slice.contains("closed"),
        "AC-8 FAIL: DEV-025 status line must show 'closed'"
    );
}

// ── AC-9: audit history DEV-025 row complete ─────────────────────────────────

/// AC-9: docs/14_deviation_audit_history.md DEV-025 row must reference
/// TASK-128a, TASK-128b, TASK-130, TASK-130a, TASK-130b, TASK-130c.
///
/// RED today: not yet updated.
#[test]
fn dev_025_audit_history_complete() {
    let src = include_str!("../../../docs/14_deviation_audit_history.md");

    let dev025_start = src
        .find("DEV-025")
        .expect("AC-9: DEV-025 must exist in 14_deviation_audit_history.md");
    let dev025_slice = &src[dev025_start..dev025_start + src[dev025_start..].len().min(2000)];

    for task in &[
        "TASK-128a",
        "TASK-128b",
        "TASK-130",
        "TASK-130a",
        "TASK-130b",
        "TASK-130c",
    ] {
        assert!(
            dev025_slice.contains(task),
            "AC-9 FAIL: DEV-025 audit row missing {task}"
        );
    }
}

// ── Negative-1: empty polygons rejected at host validator ────────────────────

/// Negative-1: Same contract as AC-5 but framed as host-validator rejection.
/// A region with empty polygons vec must be rejected; dispatch must Err with FatalModule.
///
/// Config contract for Step 3: `fixture_case = "empty_polygons"` (same as AC-5).
///
/// RED today: guest emits nothing → Ok(None).
#[test]
fn empty_polygons_rejected_at_host_validator() {
    use slicer_host::PrepassStageOutput;

    let engine = Arc::new(WasmEngine::new());
    let component = match load_sdk_prepass_guest(&engine) {
        Some(c) => c,
        None => {
            eprintln!("SKIP: sdk-prepass-guest.component.wasm missing");
            return;
        }
    };
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let module = make_compiled_module_with_config(
        "com.test.paint-seg-neg1",
        "PrePass::PaintSegmentation",
        component,
        fixture_config("empty_polygons"),
    );
    let blackboard = blackboard_with_objects(&["obj-a"]);

    let result = PrepassStageRunner::run_stage(
        &dispatcher,
        &"PrePass::PaintSegmentation".to_string(),
        &module,
        &blackboard,
    );

    match result {
        Err(e) => {
            let dbg = format!("{:?}", e);
            assert!(
                dbg.contains("FatalModule"),
                "Neg-1: error must be FatalModule, got: {dbg}"
            );
        }
        Ok((PrepassStageOutput::None, _)) => {
            panic!(
                "Neg-1 FAIL (RED): got Ok(None) — empty_polygons fixture not yet emitted by guest"
            );
        }
        Ok((other, _)) => panic!(
            "Neg-1 FAIL: expected Err(FatalModule), got Ok({:?})",
            std::mem::discriminant(&other)
        ),
    }
}

// ── Negative-2: no early return bypasses drain ───────────────────────────────

/// Negative-2: Within the PaintSegmentation arm body in slicer-macros/src/lib.rs,
/// there must be zero occurrences of `return Ok(())` that appear BEFORE the
/// `for` loop over `sdk_output.regions()`.
///
/// RED today: there is no drain loop yet, so any `return Ok(())` before it counts as a bypass.
#[test]
fn no_early_return_bypasses_drain() {
    let src = include_str!("../../slicer-macros/src/lib.rs");

    let sentinel = "PrePass::PaintSegmentation";
    let arm_start = src
        .find(sentinel)
        .expect("must contain PrePass::PaintSegmentation arm sentinel");

    // Extract a bounded arm body (4000 chars max).
    let arm_end = arm_start + src[arm_start..].len().min(4000);
    let arm_body = &src[arm_start..arm_end];

    // Find position of the drain loop.
    let loop_pos = arm_body.find("for");

    // Count `return Ok(())` occurrences before the loop.
    let early_returns: usize = if let Some(loop_at) = loop_pos {
        let pre_loop = &arm_body[..loop_at];
        pre_loop.matches("return Ok(())").count()
    } else {
        // No loop found yet — any return Ok(()) in arm is a potential bypass.
        arm_body.matches("return Ok(())").count()
    };

    assert_eq!(
        early_returns, 0,
        "Neg-2 FAIL: found {early_returns} early `return Ok(())` before drain loop in \
         PaintSegmentation arm — these would bypass the drain"
    );
}
