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

        ..Default::default()
    };

    SliceIR {
        schema_version: semver(4, 1, 0),
        global_layer_index: layer_index,
        z: region_z,
        regions: vec![region],
    }
}

fn make_loaded_module(id: &str, stage: &str) -> LoadedModule {
    LoadedModuleBuilder::new(
        id,
        semver(1, 0, 0),
        stage,
        String::new(),
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
    component: Arc<slicer_runtime::WasmComponent>,
    config: ConfigView,
    claims: Vec<String>,
) -> TestModuleBundle {
    let loaded = make_loaded_module(module_id, stage_id);
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
        has_nonplanar: true,
        is_sync_layer: false,

        ..Default::default()
    };

    let mut arena = LayerArena::new();
    let slice = build_fixture_slice_ir(layer_index, region_z);
    // Read out fixture values BEFORE handing the SliceIR to the arena, so the
    // assertion side has stable references to the populated inputs.
    let fixture_region_id: RegionId = slice.regions[0].region_id;
    let fixture_object_id = slice.regions[0].object_id.clone();
    let mut bb = Blackboard::new(empty_mesh_ir(), 1);
    let mut map = slicer_ir::RegionMapIR::default();
    let config = map.intern_config(slicer_ir::ResolvedConfig {
        extensions: std::collections::BTreeMap::from([(
            "emit_field_witness".to_owned(),
            ConfigValue::Int(1),
        )]),
        ..Default::default()
    });
    map.entries.insert(
        slicer_ir::RegionKey {
            global_layer_index: layer_index,
            object_id: fixture_object_id.clone(),
            region_id: fixture_region_id,
            variant_chain: slice.regions[0].variant_chain.clone(),
        },
        slicer_ir::RegionPlan {
            config,
            ..Default::default()
        },
    );
    bb.commit_region_map(Arc::new(map))
        .expect("commit exact holder and witness config");
    bb.commit_slice_ir(Arc::new(vec![slice.clone()]))
        .expect("commit selection slice");
    arena.set_slice(slice).expect("commit slice ir");

    // Exact config retains the rectilinear sparse holder and declared witness
    // extension; no missing-map default authority is used.

    let stage: StageId = "Layer::Infill".to_string();
    let index =
        crate::common::selection_for_module(&stage, &layer, &module.module, &bb, None, None)
            .expect("build fixture selection");
    let selection = index
        .selection_for(layer.index, module.module.module_id())
        .expect("fixture selection");
    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &module.as_live(),
        layer_input(&bb, &arena, selection),
    )
    .expect("dispatch must succeed under the field-witness config")
    .expect("dispatch must produce a commit for the field-witness config");
    slicer_runtime::apply_for_test(
        &mut arena,
        commit,
        // exhaustive: adapter fixture explicitly pins all apply context metadata.
        &slicer_runtime::StageApplyContext {
            stage_id: &stage,
            module_id: module.module.module_id(),
            layer_index,
            seam_plan: None,
            config_view: None,
            committed_slices: None,
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

// ── per-region config + invocation config witness (AC-R5) ────────────────────
//
// The guest emits one witness path per incoming region under the
// `emit_config_witness = 1` gate. The producer layout is specified in
// `crates/slicer-wasm-host/test-guests/sdk-layer-infill-guest/src/lib.rs`; the
// decoder below is authored independently against literal expectations rather
// than sharing an adapter resolver.

/// Marker for the config-witness paths. Decoded from the header
/// `point[0].flow_factor` and asserted before any payload slot is read.
const CONFIG_WITNESS_MARKER: f32 = 9.0;

/// Independently authored decoder for the guest's config-witness path.
#[derive(Debug)]
struct ConfigWitnessObservation {
    region_width: f32,
    invocation_width: f32,
    region_private_probe: f32,
    invocation_private_probe: f32,
    has_region_config: f32,
    region_id: f32,
    marker: f32,
}

/// Decode three `Point3WithWidth` slots: header, payload, identity.
fn decode_config_witness(points: &[slicer_ir::Point3WithWidth]) -> ConfigWitnessObservation {
    assert!(
        points.len() >= 3,
        "config witness path must carry 3 points, got {}",
        points.len()
    );
    ConfigWitnessObservation {
        region_width: points[1].x,
        invocation_width: points[1].y,
        region_private_probe: points[1].z,
        invocation_private_probe: points[1].width,
        has_region_config: points[1].flow_factor,
        region_id: points[2].x,
        marker: points[0].flow_factor,
    }
}

/// Invocation config declaring exactly `line_width` and the witness gate.
/// Declared keys are the module's read set: the per-region projection keeps
/// only these keys, so a region's undeclared `private_probe` extension must be
/// rejected from its SDK config view.
fn invocation_config_witness() -> ConfigView {
    let mut fields: HashMap<String, ConfigValue> = HashMap::new();
    fields.insert("emit_config_witness".to_string(), ConfigValue::Int(1));
    fields.insert("line_width".to_string(), ConfigValue::Float(0.8));
    ConfigView::from_map(fields)
}

/// Two source regions whose resolved configs carry distinct `line_width`
/// values (0.4 / 0.6) plus an undeclared `private_probe` extension.
fn build_config_witness_slice_ir(layer_index: u32, region_z: f32) -> SliceIR {
    let region = |region_id: u64, x: f32| SlicedRegion {
        object_id: ObjectId::from("config-witness-obj-01"),
        region_id,
        polygons: vec![square(x, x, x + 1.0, x + 1.0)],
        effective_layer_height: 0.35,
        ..Default::default()
    };
    SliceIR {
        schema_version: semver(4, 1, 0),
        global_layer_index: layer_index,
        z: region_z,
        regions: vec![region(42, 0.0), region(43, 1.0)],
    }
}

/// AC-R5: the real SDK guest observes the invocation-wide config, the
/// per-region slice config reached through `SliceRegionView::config()`, and
/// the rejection of the undeclared `private_probe` key. The invocation value
/// (0.8) must stay distinguishable from both per-region values (0.4 / 0.6).
#[test]
fn per_region_config_and_invocation_config_are_observed() {
    use slicer_runtime::LayerStageRunner;

    let guest_artifact = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("slicer-wasm-host")
        .join("test-guests")
        .join("sdk-layer-infill-guest.component.wasm");
    assert!(
        guest_artifact.exists(),
        "missing real SDK guest artifact (never skip): {}",
        guest_artifact.display()
    );

    let engine = wasm_cache::shared_engine();
    let dispatcher = WasmRuntimeDispatcher::new(Arc::clone(&engine));
    let component = wasm_cache::compiled_guest("sdk-layer-infill-guest");

    let module = make_module_bundle(
        "rectilinear-infill",
        "Layer::Infill",
        component,
        invocation_config_witness(),
        vec!["claim:sparse-fill".to_string()],
    );

    let layer_index = 7u32;
    let region_z = 7.7f32;
    let layer = GlobalLayer {
        index: layer_index,
        z: region_z,
        has_nonplanar: true,
        is_sync_layer: false,
        ..Default::default()
    };

    let slice = build_config_witness_slice_ir(layer_index, region_z);
    assert_eq!(
        slice.regions.len(),
        2,
        "fixture must carry two source regions"
    );
    let fixture_region_ids: Vec<u64> = slice
        .regions
        .iter()
        .map(|region| region.region_id)
        .collect();
    assert_eq!(
        fixture_region_ids,
        vec![42, 43],
        "fixture identity guard before dispatching the witness"
    );

    let mut arena = LayerArena::new();
    let mut bb = Blackboard::new(empty_mesh_ir(), 1);
    let mut map = slicer_ir::RegionMapIR::default();
    let mut cfg_a = slicer_ir::ResolvedConfig {
        line_width: 0.4,
        ..Default::default()
    };
    cfg_a
        .extensions
        .insert("private_probe".to_string(), ConfigValue::Int(99));
    let mut cfg_b = slicer_ir::ResolvedConfig {
        line_width: 0.6,
        ..Default::default()
    };
    cfg_b
        .extensions
        .insert("private_probe".to_string(), ConfigValue::Int(99));
    let id_a = map.intern_config(cfg_a);
    let id_b = map.intern_config(cfg_b);
    assert_ne!(
        id_a, id_b,
        "distinct per-region widths must intern distinct configs"
    );
    for (region_index, config_id) in [(0usize, id_a), (1usize, id_b)] {
        let region = &slice.regions[region_index];
        map.entries.insert(
            slicer_ir::RegionKey {
                global_layer_index: layer_index,
                object_id: region.object_id.clone(),
                region_id: region.region_id,
                variant_chain: region.variant_chain.clone(),
            },
            slicer_ir::RegionPlan {
                config: config_id,
                ..Default::default()
            },
        );
    }
    bb.commit_region_map(Arc::new(map))
        .expect("commit per-region configs carrying undeclared private_probe");
    bb.commit_slice_ir(Arc::new(vec![slice.clone()]))
        .expect("commit selection slice");
    arena.set_slice(slice).expect("commit slice ir");

    let stage: StageId = "Layer::Infill".to_string();
    let index =
        crate::common::selection_for_module(&stage, &layer, &module.module, &bb, None, None)
            .expect("build fixture selection");
    let selection = index
        .selection_for(layer.index, module.module.module_id())
        .expect("fixture selection");
    let commit = LayerStageRunner::run_stage(
        &dispatcher,
        &stage,
        &layer,
        &module.as_live(),
        layer_input(&bb, &arena, selection),
    )
    .expect("dispatch must succeed under the config-witness gate")
    .expect("dispatch must produce a commit for the config-witness gate");
    slicer_runtime::apply_for_test(
        &mut arena,
        commit,
        // exhaustive: adapter fixture explicitly pins all apply context metadata.
        &slicer_runtime::StageApplyContext {
            stage_id: &stage,
            module_id: module.module.module_id(),
            layer_index,
            seam_plan: None,
            config_view: None,
            committed_slices: None,
        },
    )
    .expect("commit must succeed");

    let infill = arena
        .infill()
        .expect("drain-back must commit InfillIR into the arena");
    // Untagged paths are bucketed together, so gather across regions; the
    // witness payload itself carries the source region id.
    let all_paths: Vec<&slicer_ir::ExtrusionPath3D> = infill
        .regions
        .iter()
        .flat_map(|region| region.sparse_infill.iter())
        .collect();
    assert!(
        !all_paths.is_empty(),
        "guest emitted no sparse paths; output guard failed"
    );

    let witness_paths: Vec<&slicer_ir::ExtrusionPath3D> = all_paths
        .iter()
        .copied()
        .filter(|path| {
            path.points
                .first()
                .map(|point| point.flow_factor == CONFIG_WITNESS_MARKER)
                .unwrap_or(false)
        })
        .collect();
    assert_eq!(
        witness_paths.len(),
        2,
        "guest must emit exactly one config-witness path per incoming region; got {}",
        witness_paths.len()
    );

    let observations: Vec<ConfigWitnessObservation> = witness_paths
        .iter()
        .map(|path| decode_config_witness(&path.points))
        .collect();

    for observation in &observations {
        assert_eq!(
            observation.marker, CONFIG_WITNESS_MARKER,
            "decoded path is not a config-witness path (marker mismatch)"
        );
        assert_eq!(
            observation.has_region_config, 1.0,
            "region {}: SDK SliceRegionView carried no per-region config (adapter did not forward it)",
            observation.region_id
        );
        assert_eq!(
            observation.region_private_probe, 0.0,
            "region {}: undeclared private_probe leaked into the SDK per-region config",
            observation.region_id
        );
        assert_eq!(
            observation.invocation_private_probe, 0.0,
            "invocation config leaked undeclared private_probe"
        );
        assert!(
            (observation.invocation_width - 0.8).abs() < 1e-4,
            "invocation line_width: expected 0.8, got {}",
            observation.invocation_width
        );
    }

    let region_a = observations
        .iter()
        .find(|observation| observation.region_id == 42.0)
        .expect("no config witness for source region 42");
    let region_b = observations
        .iter()
        .find(|observation| observation.region_id == 43.0)
        .expect("no config witness for source region 43");
    assert!(
        (region_a.region_width - 0.4).abs() < 1e-4,
        "region 42 per-region line_width: expected 0.4, got {}",
        region_a.region_width
    );
    assert!(
        (region_b.region_width - 0.6).abs() < 1e-4,
        "region 43 per-region line_width: expected 0.6, got {}",
        region_b.region_width
    );
    // The invocation value must be distinguishable from both per-region
    // reads, so a resolver that collapsed the two channels would fail.
    assert!(
        (region_a.invocation_width - region_a.region_width).abs() > 0.1,
        "invocation width 0.8 collapsed onto region 42 width {}",
        region_a.region_width
    );
    assert!(
        (region_b.invocation_width - region_b.region_width).abs() > 0.1,
        "invocation width 0.8 collapsed onto region 43 width {}",
        region_b.region_width
    );
    assert!(
        (region_a.region_width - region_b.region_width).abs() > 0.1,
        "per-region widths collapsed to the same value"
    );
}
