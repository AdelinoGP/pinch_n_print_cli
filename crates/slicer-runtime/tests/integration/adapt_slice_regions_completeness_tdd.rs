//! Regression: the macro-emitted `__slicer_adapt_slice_regions` helper must
//! copy every field on the WIT `SliceRegionView` resource onto the SDK
//! `slicer_sdk::views::SliceRegionView` value type.
//!
//! Pre-fix the adapter (`crates/slicer-macros/src/lib.rs` —
//! `__slicer_adapt_slice_regions`) silently dropped `sparse_infill_area`,
//! `bridge_areas`, `bridge_orientation_deg`, and `held_claims`. Guest
//! modules then saw empty / default values for those fields and emitted no
//! sparse infill. The bug class is "field present on both sides of the
//! boundary but the adapter forgot it", so this test must check **every**
//! field that has both a host-side WIT accessor and an SDK setter — that way
//! a future field addition recreates the bug only if this test is also
//! updated.
//!
//! Test strategy: build a `SliceIR` with one region whose 17 covered fields
//! all hold distinguishable, non-default values; dispatch
//! `sdk-layer-infill-guest` (which is authored via `#[slicer_module]` so
//! it exercises the macro-emitted adapter); have the guest encode all 17
//! field values into a second sparse path via `SliceRegionFieldsWitness`;
//! assert each field round-tripped intact.

#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    ConfigValue, ConfigView, ExPolygon, GlobalLayer, ObjectId, PaintSemantic, PaintValue, Point2,
    Polygon, RegionId, SemVer, SliceIR, SlicedRegion, StageId,
};
use slicer_runtime::instance_pool::{build_wasm_instance_pool, WasmArtifactMetadata};
use slicer_runtime::{
    Blackboard, CompiledModuleBuilder, LayerArena, LoadedModule, LoadedModuleBuilder,
    WasmRuntimeDispatcher,
};
use witness::SliceRegionFieldsWitness;

use crate::common::wasm_cache;
use crate::common::{layer_input, TestModuleBundle};

// ── fixture builders ─────────────────────────────────────────────────────────

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn empty_mesh_ir() -> Arc<slicer_ir::MeshIR> {
    Arc::new(slicer_ir::MeshIR {
        schema_version: semver(1, 0, 0),
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

/// Axis-aligned square ExPolygon at given mm corners. Distinct polygons for
/// each field slot (slice, infill, top, bottom, bridge, sparse) so any
/// accidental cross-wiring on the adapter would change a length count.
fn square(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(min_x, min_y),
                Point2::from_mm(max_x, min_y),
                Point2::from_mm(max_x, max_y),
                Point2::from_mm(min_x, max_y),
            ],
        },
        holes: Vec::new(),
    }
}

/// Build the test's slice IR. One region whose 17 covered fields each hold
/// a distinguishable non-default value. Vector-typed fields use vectors of
/// distinct lengths so each per-field length assertion is independent
/// (no length collisions could mask a swapped accessor).
fn build_fixture_slice_ir(layer_index: u32, region_z: f32) -> SliceIR {
    let mut segment_annotations: HashMap<PaintSemantic, Vec<Vec<Option<PaintValue>>>> =
        HashMap::new();
    segment_annotations.insert(
        PaintSemantic::FuzzySkin,
        vec![vec![Some(PaintValue::Flag(true))]],
    );

    let region = SlicedRegion {
        // 17-byte object id keeps len/byte-sum distinct from any vector length.
        object_id: ObjectId::from("test-obj-uuid-001"),
        region_id: 42u64,
        polygons: vec![square(0.0, 0.0, 10.0, 10.0)], // 1 polygon
        infill_areas: vec![square(1.0, 1.0, 9.0, 9.0), square(2.0, 2.0, 8.0, 8.0)], // 2 polygons
        nonplanar_surface: Some(slicer_ir::SurfaceGroupId::default()), // → has_nonplanar = true
        effective_layer_height: 0.35,
        segment_annotations,
        variant_chain: Vec::new(),
        top_shell_index: Some(3),
        bottom_shell_index: Some(5),
        top_solid_fill: vec![
            square(0.5, 0.5, 9.5, 9.5),
            square(0.5, 0.5, 9.5, 9.5),
            square(0.5, 0.5, 9.5, 9.5),
        ], // 3 polygons
        bottom_solid_fill: vec![
            square(0.5, 0.5, 9.5, 9.5),
            square(0.5, 0.5, 9.5, 9.5),
            square(0.5, 0.5, 9.5, 9.5),
            square(0.5, 0.5, 9.5, 9.5),
        ], // 4 polygons
        is_bridge: true,
        // 5 polygons
        bridge_areas: (0..5).map(|_| square(0.5, 0.5, 9.5, 9.5)).collect(),
        bridge_orientation_deg: 42.0,
        // 6 polygons — distinct from every other vec length above.
        sparse_infill_area: (0..6).map(|_| square(0.5, 0.5, 9.5, 9.5)).collect(),
    };

    SliceIR {
        schema_version: semver(4, 1, 0),
        global_layer_index: layer_index,
        z: region_z,
        regions: vec![region],
    }
}

fn make_loaded_module(id: &str, stage: &str, wit_world: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        wit_world,
        std::path::PathBuf::from("/dev/null"),
    )
    .min_host_version(semver(0, 1, 0))
    .min_ir_schema(semver(1, 0, 0))
    .max_ir_schema(semver(2, 0, 0))
    .layer_parallel_safe(true)
    .build()
}

fn make_module_bundle(
    module_id: &str,
    stage_id: &str,
    wit_world: &str,
    component: Arc<slicer_runtime::WasmComponent>,
    config: ConfigView,
    claims: Vec<String>,
) -> TestModuleBundle {
    let loaded = make_loaded_module(module_id, stage_id, wit_world);
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
    let module = CompiledModuleBuilder::new(module_id)
        .config_view(Arc::new(config))
        .claims(claims)
        .build();
    TestModuleBundle {
        module,
        pool,
        component: Some(component),
    }
}

fn emit_field_witness_config() -> ConfigView {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    // Gates the guest's per-field encode path. See the
    // `if config.get_int("emit_field_witness") == Some(1)` arm in
    // `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs`.
    fields.insert("emit_field_witness".to_string(), ConfigValue::Int(1));
    ConfigView::from_map(fields)
}

// ── the regression test ──────────────────────────────────────────────────────

/// One assertion per SDK `SliceRegionView` field that has both a host-side
/// WIT accessor and an SDK setter. Detects the pre-fix bug class
/// ("adapter forgot field X") for every covered field at once.
#[test]
fn macro_adapter_round_trips_every_slice_region_view_field() {
    use slicer_runtime::LayerStageRunner;

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = wasm_cache::compiled_guest("sdk-layer-infill-guest");

    // Module ID must be a value the dispatcher's holder-matcher will accept
    // as equal to the region's `*_fill_holder` config. The matcher accepts
    // either the full module ID or — for `com.core.*` IDs — the short tail.
    // Using `rectilinear-infill` as the module ID (matching the holder
    // string in the region map) keeps the test independent of the
    // `com.core.` prefix policy.
    let module_id = "rectilinear-infill";
    let module = make_module_bundle(
        module_id,
        "Layer::Infill",
        slicer_schema::WORLD_LAYER,
        component,
        emit_field_witness_config(),
        // The dispatcher filters `claims` by the FILL_CLAIM_IDS allow-list
        // then by the per-role holder match. Giving the module exactly one
        // fill claim makes `held_claims = ["claim:sparse-fill"]` an
        // unambiguous distinguishable value to assert on.
        vec!["claim:sparse-fill".to_string()],
    );

    let layer_index = 7u32;
    let region_z = 7.7f32;
    let layer = GlobalLayer {
        index: layer_index,
        z: region_z,
        active_regions: Vec::new(),
        has_nonplanar: true,
        is_sync_layer: false,
    };

    let mut arena = LayerArena::new();
    let slice = build_fixture_slice_ir(layer_index, region_z);
    // Read out fixture values BEFORE handing the SliceIR to the arena, so the
    // assertion side has stable references to the populated inputs.
    let fixture_region_id: RegionId = slice.regions[0].region_id;
    let fixture_object_id = slice.regions[0].object_id.clone();
    arena.set_slice(slice).expect("commit slice ir");

    // No region_map committed: the dispatcher's `held_claims` resolver then
    // takes the `ResolvedConfig::default()` branch (`unwrap_or_default()` in
    // `slicer-wasm-host/src/dispatch.rs::run_stage`), whose
    // `*_fill_holder` defaults are all `"rectilinear-infill"`. Combined with
    // `module_id = "rectilinear-infill"` and `module.claims =
    // ["claim:sparse-fill"]`, the per-region holder filter matches and
    // emits `held_claims = ["claim:sparse-fill"]` — the value the SDK
    // accessor must witness. Skipping the region_map also avoids
    // `ConfigView::from_declared`'s key-filter pass dropping the test's
    // `emit_field_witness` sentinel key.
    let bb = Blackboard::new(empty_mesh_ir(), 1);

    let stage: StageId = "Layer::Infill".to_string();
    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &module.as_live(),
        layer_input(&bb, &arena),
    )
    .expect("dispatch must succeed under the field-witness config")
    .expect("dispatch must produce a commit for the field-witness config");
    slicer_runtime::apply_for_test(
        &mut arena,
        commit,
        &slicer_runtime::StageApplyContext {
            stage_id: &stage,
            module_id: module.module.module_id(),
            layer_index,
            seam_plan: None,
        },
    )
    .expect("commit must succeed");

    let infill = arena
        .infill()
        .expect("drain-back must commit InfillIR into the arena");
    assert!(
        !infill.regions.is_empty(),
        "infill IR must contain at least one region (got {} regions)",
        infill.regions.len()
    );
    let sparse_paths = &infill.regions[0].sparse_infill;
    // Guest emits exactly TWO paths under this config: the legacy
    // `SdkInfillWitness` content witness (path 0) and the new
    // `SliceRegionFieldsWitness` field witness (path 1).
    assert_eq!(
        sparse_paths.len(),
        2,
        "guest must emit two sparse paths under emit_field_witness=1; got {}",
        sparse_paths.len()
    );

    // Decode the field witness. The decoder is unconditional on `points` len
    // so we check the marker explicitly to guard against the wrong path
    // being decoded under the wrong layout.
    let witness = SliceRegionFieldsWitness::decode(&sparse_paths[1].points);
    assert_eq!(
        witness.marker,
        SliceRegionFieldsWitness::MARKER,
        "decoded path is not the field-witness path (marker mismatch)"
    );

    // ── one assertion per SDK SliceRegionView field ───────────────────────
    // Compute expected digests on the host side (same byte-sum formula the
    // guest uses), then compare each accessor's witness slot independently.
    let expected_object_id_byte_sum: u32 =
        fixture_object_id.as_bytes().iter().map(|b| *b as u32).sum();
    let expected_first_held_claim_byte_sum: u32 = "claim:sparse-fill"
        .as_bytes()
        .iter()
        .map(|b| *b as u32)
        .sum();

    // 1. object_id — String → (len, byte-sum digest)
    assert_eq!(
        witness.object_id_len,
        fixture_object_id.len() as f32,
        "field `object_id`: adapter dropped or truncated the string"
    );
    assert_eq!(
        witness.object_id_byte_sum, expected_object_id_byte_sum as f32,
        "field `object_id`: adapter dropped string content (byte-sum mismatch)"
    );
    // 2. region_id — u64
    assert_eq!(
        witness.region_id, fixture_region_id as f32,
        "field `region_id`: adapter dropped or zeroed the value"
    );
    // 3. polygons — Vec<ExPolygon>
    assert_eq!(
        witness.polygons_len, 1.0,
        "field `polygons`: adapter dropped slice polygons"
    );
    // 4. infill_areas — Vec<ExPolygon>
    assert_eq!(
        witness.infill_areas_len, 2.0,
        "field `infill_areas`: adapter dropped infill polygons"
    );
    // 5. effective_layer_height — f32
    assert!(
        (witness.effective_layer_height - 0.35).abs() < 1e-4,
        "field `effective_layer_height`: expected 0.35, got {}",
        witness.effective_layer_height
    );
    // 6. z — f32
    assert!(
        (witness.z - 7.7).abs() < 1e-4,
        "field `z`: expected 7.7, got {}",
        witness.z
    );
    // 7. has_nonplanar — bool (encoded as 0/1)
    assert_eq!(
        witness.has_nonplanar, 1.0,
        "field `has_nonplanar`: adapter dropped non-planar flag (expected true)"
    );
    // 8. segment_annotations — HashMap<PaintSemantic, _>
    assert_eq!(
        witness.segment_annotations_len, 1.0,
        "field `segment_annotations`: adapter dropped per-segment paint map"
    );
    // 9. top_shell_index — Option<u8>
    assert_eq!(
        witness.top_shell_index, 3.0,
        "field `top_shell_index`: adapter dropped Some(3) value"
    );
    // 10. bottom_shell_index — Option<u8>
    assert_eq!(
        witness.bottom_shell_index, 5.0,
        "field `bottom_shell_index`: adapter dropped Some(5) value"
    );
    // 11. top_solid_fill — Vec<ExPolygon>
    assert_eq!(
        witness.top_solid_fill_len, 3.0,
        "field `top_solid_fill`: adapter dropped top-solid-fill polygons"
    );
    // 12. bottom_solid_fill — Vec<ExPolygon>
    assert_eq!(
        witness.bottom_solid_fill_len, 4.0,
        "field `bottom_solid_fill`: adapter dropped bottom-solid-fill polygons"
    );
    // 13. is_bridge — bool
    assert_eq!(
        witness.is_bridge, 1.0,
        "field `is_bridge`: adapter dropped bridge classification flag"
    );
    // 14. bridge_areas — Vec<ExPolygon>  (was dropped pre-fix)
    assert_eq!(
        witness.bridge_areas_len, 5.0,
        "field `bridge_areas`: adapter dropped bridge polygons (pre-fix regression)"
    );
    // 15. bridge_orientation_deg — f32  (was dropped pre-fix)
    assert!(
        (witness.bridge_orientation_deg - 42.0).abs() < 1e-3,
        "field `bridge_orientation_deg`: adapter dropped bridge angle (pre-fix regression); expected 42.0, got {}",
        witness.bridge_orientation_deg
    );
    // 16. sparse_infill_area — Vec<ExPolygon>  (was dropped pre-fix)
    assert_eq!(
        witness.sparse_infill_area_len, 6.0,
        "field `sparse_infill_area`: adapter dropped sparse-only infill polygon (pre-fix regression)"
    );
    // 17. held_claims — Vec<String>  (was dropped pre-fix)
    assert_eq!(
        witness.held_claims_len, 1.0,
        "field `held_claims`: adapter dropped the resolved claim list (pre-fix regression)"
    );
    assert_eq!(
        witness.first_held_claim_byte_sum, expected_first_held_claim_byte_sum as f32,
        "field `held_claims[0]`: adapter dropped string content (byte-sum mismatch)"
    );
}
