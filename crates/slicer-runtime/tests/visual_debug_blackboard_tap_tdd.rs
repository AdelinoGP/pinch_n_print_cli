//! Contract coverage for the Blackboard-read visual-debug tap capture path
//! (packet 161, Step 3): `slicer_runtime::layer_executor::execute_blackboard_taps`.
//!
//! Distinct from the arena `execute_captured_stages` closure (packet 158) —
//! this reads the committed, whole-print `Vec<SliceIR>` straight off a
//! `Blackboard` (as it would appear on `PrepassContext::blackboard` after
//! `prepare_prepass_context`), with no `LayerArena`, no `LayerStageRunner`,
//! and no module dispatch involved at all.
//!
//! This test now covers all nine Blackboard-read taps assigned through
//! packet 161 Step 4: the four SliceIR-family taps (`Layer::Slice`,
//! `PrePass::PaintSegmentation`, `Layer::PaintRegionAnnotation`,
//! `Layer::SlicePostProcess`) plus five composite taps
//! (`PrePass::MeshAnalysis`, `PrePass::OverhangAnnotation`,
//! `PrePass::SeamPlanning`, `PrePass::SupportGeometry`,
//! `PrePass::RegionMapping`). A later step extends this same test with the
//! remaining two PostPass taps (`PostPass::LayerFinalization`,
//! `PostPass::GCodeEmit`) — do not assert on taps not yet implemented.

use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::slice_ir::QuartileBand;
use slicer_ir::{
    BridgeRegion, ExPolygon, ExtrusionPath3D, ExtrusionRole, MeshIR, ObjectSurfaceData,
    OverhangRegion, PaintSemantic, PaintValue, Point2, Point3WithWidth, Polygon, RegionKey,
    RegionMapIR, RegionPlan, ResolvedConfig, ScoredSeamCandidate, SeamPlanEntry, SeamPlanIR,
    SeamPosition, SeamReason, SliceIR, SlicedRegion, SupportGeometryIR, SupportGeometryKey,
    SupportPlanEntry, SupportPlanIR, SurfaceClassificationIR, CURRENT_REGION_MAP_IR_SCHEMA_VERSION,
    CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION, CURRENT_SLICE_IR_SCHEMA_VERSION,
    CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION, CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
    CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
};
use slicer_runtime::layer_executor::{
    execute_blackboard_taps, CapturedIr, BLACKBOARD_TAP_STAGE_IDS,
};
use slicer_runtime::{Blackboard, CaptureExecutionError, CaptureRequest};

/// One populated `ExPolygon`: a simple triangle, no holes. Values are
/// arbitrary but deterministic — only used to prove the captured payload is
/// byte-identical to what was committed, not to model real geometry.
fn triangle_expolygon() -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: Vec::new(),
    }
}

/// A `SliceIR` for global layer 0 with every field this step's taps
/// document as their source populated with a distinguishable, non-default
/// value: `regions[].polygons`, `regions[].infill_areas`,
/// `regions[].segment_annotations`, and the IR's own `global_layer_index`.
fn seeded_slice_ir() -> SliceIR {
    let mut segment_annotations = HashMap::new();
    segment_annotations.insert(
        PaintSemantic::Material,
        vec![vec![Some(PaintValue::ToolIndex(2))]],
    );
    SliceIR {
        schema_version: CURRENT_SLICE_IR_SCHEMA_VERSION,
        global_layer_index: 0,
        z: 0.5,
        regions: vec![SlicedRegion {
            object_id: "obj-0".to_string(),
            region_id: 7,
            polygons: vec![triangle_expolygon()],
            infill_areas: vec![triangle_expolygon()],
            segment_annotations,
            ..SlicedRegion::default()
        }],
    }
}

/// Build a `Blackboard` with `slice_ir` committed for one global layer,
/// mirroring the shape `Blackboard::slice_ir()` has after prepass
/// (`crate::run::prepare_prepass_context`) commits `PrePass::Slice`'s output —
/// without running any prepass, module, or arena machinery at all.
fn blackboard_with_committed_slice(slice_ir: SliceIR) -> Blackboard {
    let mut blackboard = Blackboard::new(Arc::new(MeshIR::default()), 1);
    blackboard
        .commit_slice_ir(Arc::new(vec![slice_ir]))
        .expect("commit_slice_ir on a fresh Blackboard must succeed");
    blackboard
}

/// A `SurfaceClassificationIR` with every field the `PrePass::MeshAnalysis`/
/// `PrePass::OverhangAnnotation` taps document as their source populated
/// with a distinguishable, non-default value: `per_object` (via
/// `bridge_regions[].xy_footprint` and `overhang_regions[].xy_footprint`)
/// and `overhang_quartile_polygons`.
fn seeded_surface_classification() -> SurfaceClassificationIR {
    let mut per_object = HashMap::new();
    per_object.insert(
        "obj-0".to_string(),
        ObjectSurfaceData {
            bridge_regions: vec![BridgeRegion {
                id: 11,
                xy_footprint: vec![triangle_expolygon()],
                ..BridgeRegion::default()
            }],
            overhang_regions: vec![OverhangRegion {
                id: 22,
                xy_footprint: vec![triangle_expolygon()],
                ..OverhangRegion::default()
            }],
            ..ObjectSurfaceData::default()
        },
    );
    let mut overhang_quartile_polygons = HashMap::new();
    overhang_quartile_polygons.insert(
        0u32,
        vec![QuartileBand {
            quartile: 3,
            polygons: vec![triangle_expolygon()],
        }],
    );
    SurfaceClassificationIR {
        schema_version: CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION,
        per_object,
        overhang_quartile_polygons,
    }
}

/// A `SeamPlanIR` with `entries[].{region_key, chosen_candidate.point,
/// scored_candidates[].reason}` populated — pinning the corrected field
/// names (`chosen_candidate.point`, NOT `seam_xy`) and millimeter units on
/// `Point3WithWidth`.
fn seeded_seam_plan() -> SeamPlanIR {
    let point = Point3WithWidth {
        x: 1.5,
        y: 2.5,
        z: 0.5,
        width: 0.4,
        ..Point3WithWidth::default()
    };
    SeamPlanIR {
        schema_version: CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION,
        entries: vec![SeamPlanEntry {
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "obj-0".to_string(),
                region_id: 7,
                variant_chain: Vec::new(),
            },
            chosen_candidate: SeamPosition {
                point,
                wall_index: 0,
            },
            scored_candidates: vec![ScoredSeamCandidate {
                position: point,
                score: 0.1,
                reason: SeamReason::Concave,
            }],
        }],
    }
}

/// A `SupportGeometryIR`/`SupportPlanIR` composite with `entries` and
/// `branch_segments` populated — pinning millimeter units on
/// `ExtrusionPath3D`'s `Point3WithWidth` points (NOT 100-nm scaled units).
fn seeded_support_geometry_and_plan() -> (SupportGeometryIR, SupportPlanIR) {
    let mut entries = HashMap::new();
    entries.insert(
        SupportGeometryKey {
            global_support_layer_index: 0,
            object_id: "obj-0".to_string(),
            region_id: 7,
        },
        vec![triangle_expolygon()],
    );
    let geometry = SupportGeometryIR {
        schema_version: CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION,
        support_layer_height_mm: 0.2,
        support_top_z_distance_mm: 0.1,
        entries,
    };
    let plan = SupportPlanIR {
        schema_version: CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION,
        entries: vec![SupportPlanEntry {
            global_layer_index: 0,
            object_id: "obj-0".to_string(),
            region_id: 7,
            branch_segments: vec![ExtrusionPath3D {
                points: vec![
                    Point3WithWidth {
                        x: 1.0,
                        y: 1.0,
                        z: 0.2,
                        width: 0.4,
                        ..Point3WithWidth::default()
                    },
                    Point3WithWidth {
                        x: 2.0,
                        y: 2.0,
                        z: 0.4,
                        width: 0.4,
                        ..Point3WithWidth::default()
                    },
                ],
                role: ExtrusionRole::SupportMaterial,
                speed_factor: 1.0,
            }],
        }],
    };
    (geometry, plan)
}

/// A `RegionMapIR` with one entry whose `RegionPlan.config` is a real
/// interned `ConfigId` (via `intern_config`), pinning that `config` is a
/// `ConfigId`, not a raw `ResolvedConfig`.
fn seeded_region_map() -> RegionMapIR {
    let mut region_map = RegionMapIR::default();
    let config_id = region_map.intern_config(ResolvedConfig::default());
    region_map.entries.insert(
        RegionKey {
            global_layer_index: 0,
            object_id: "obj-0".to_string(),
            region_id: 7,
            variant_chain: Vec::new(),
        },
        RegionPlan {
            config: config_id,
            ..RegionPlan::default()
        },
    );
    region_map
}

/// The four SliceIR-family taps assigned to packet 161 Step 3 — pinned as a
/// fixed sub-list so the composite-tap additions below are visibly
/// deliberate rather than a silent scope drift.
const SLICE_FAMILY_TAPS: &[&str] = &[
    "Layer::Slice",
    "PrePass::PaintSegmentation",
    "Layer::PaintRegionAnnotation",
    "Layer::SlicePostProcess",
];

/// Pins, for every SliceIR-family Blackboard-read tap implemented in this
/// step:
/// - the exact source fields (`polygons`, `infill_areas`,
///   `segment_annotations`, `global_layer_index`) survive the capture
///   unchanged;
/// - the captured `SliceIR`'s `schema_version` equals
///   `CURRENT_SLICE_IR_SCHEMA_VERSION`;
/// - tap id and layer identity (`layer_index`, `layer_z`) are correctly
///   attributed per capture;
/// - the capture ran prepass-only: no arena/per-layer stage closure ran
///   (`closure_stage_ids` stays empty; `expansions` stays empty), which is
///   also proven structurally by `execute_blackboard_taps`'s signature
///   never accepting a `LayerStageRunner` or wasm handles at all.
#[test]
fn blackboard_tap_capture_contracts() {
    let seeded = seeded_slice_ir();
    let mut blackboard = blackboard_with_committed_slice(seeded.clone());

    let surface_classification = seeded_surface_classification();
    blackboard
        .commit_surface_classification(Arc::new(surface_classification.clone()))
        .expect("commit_surface_classification on a fresh slot must succeed");
    let seam_plan = seeded_seam_plan();
    blackboard
        .commit_seam_plan(Arc::new(seam_plan.clone()))
        .expect("commit_seam_plan on a fresh slot must succeed");
    let (support_geometry, support_plan) = seeded_support_geometry_and_plan();
    blackboard
        .commit_support_geometry(Arc::new(support_geometry.clone()))
        .expect("commit_support_geometry on a fresh slot must succeed");
    blackboard
        .commit_support_plan(Arc::new(support_plan.clone()))
        .expect("commit_support_plan on a fresh slot must succeed");
    let region_map = seeded_region_map();
    blackboard
        .commit_region_map(Arc::new(region_map.clone()))
        .expect("commit_region_map on a fresh slot must succeed");

    // All nine Blackboard-read taps are in scope as of this step — assert
    // the fixed tap set is exactly what this step implements before
    // exercising it, so a future step's additions are visible as a
    // deliberate, reviewed change to this list rather than a silent scope
    // drift.
    assert_eq!(
        BLACKBOARD_TAP_STAGE_IDS,
        &[
            "Layer::Slice",
            "PrePass::PaintSegmentation",
            "Layer::PaintRegionAnnotation",
            "Layer::SlicePostProcess",
            "PrePass::MeshAnalysis",
            "PrePass::OverhangAnnotation",
            "PrePass::SeamPlanning",
            "PrePass::SupportGeometry",
            "PrePass::RegionMapping",
        ],
        "Blackboard-read tap set changed; update this test alongside it"
    );

    let request = CaptureRequest {
        stage_ids: BLACKBOARD_TAP_STAGE_IDS
            .iter()
            .map(|s| s.to_string())
            .collect(),
        layer_indices: vec![0],
    };

    let output =
        execute_blackboard_taps(&blackboard, &request).expect("all nine taps are documented");

    // One capture per requested tap, all for the single requested layer.
    assert_eq!(output.captures.len(), BLACKBOARD_TAP_STAGE_IDS.len());

    for tap in SLICE_FAMILY_TAPS {
        let capture = output
            .captures
            .iter()
            .find(|c| c.stage_id == *tap)
            .unwrap_or_else(|| panic!("no capture recorded for tap '{tap}'"));

        // Identity: tap id, layer index, layer z.
        assert_eq!(capture.stage_id, *tap);
        assert_eq!(capture.layer_index, 0);
        assert_eq!(capture.layer_z, seeded.z);

        // Payload: the captured SliceIR must carry every documented source
        // field unchanged from what was committed.
        let CapturedIr::Slice(captured_ir) = &capture.ir else {
            panic!(
                "tap '{tap}' must capture CapturedIr::Slice, got {:?}",
                capture.ir
            );
        };
        assert_eq!(captured_ir.global_layer_index, seeded.global_layer_index);
        assert_eq!(captured_ir.regions.len(), seeded.regions.len());
        assert_eq!(captured_ir.regions[0].polygons, seeded.regions[0].polygons);
        assert_eq!(
            captured_ir.regions[0].infill_areas,
            seeded.regions[0].infill_areas
        );
        assert_eq!(
            captured_ir.regions[0].segment_annotations,
            seeded.regions[0].segment_annotations
        );

        // Schema version pin: must equal the current SliceIR schema
        // version, formatted MAJOR.MINOR.PATCH.
        let v = CURRENT_SLICE_IR_SCHEMA_VERSION;
        assert_eq!(
            capture.ir.schema_version_string(),
            format!("{}.{}.{}", v.major, v.minor, v.patch)
        );
    }

    // `PrePass::MeshAnalysis` and `PrePass::OverhangAnnotation` both capture
    // the same committed `SurfaceClassificationIR`, unfiltered.
    for tap in ["PrePass::MeshAnalysis", "PrePass::OverhangAnnotation"] {
        let capture = output
            .captures
            .iter()
            .find(|c| c.stage_id == tap)
            .unwrap_or_else(|| panic!("no capture recorded for tap '{tap}'"));
        assert_eq!(capture.layer_index, 0);
        assert_eq!(capture.layer_z, seeded.z);

        let CapturedIr::SurfaceClassification(captured_ir) = &capture.ir else {
            panic!(
                "tap '{tap}' must capture CapturedIr::SurfaceClassification, got {:?}",
                capture.ir
            );
        };
        assert_eq!(captured_ir.per_object, surface_classification.per_object);
        assert_eq!(
            captured_ir.overhang_quartile_polygons,
            surface_classification.overhang_quartile_polygons
        );
        let v = CURRENT_SURFACE_CLASSIFICATION_SCHEMA_VERSION;
        assert_eq!(captured_ir.schema_version, v);
        assert_eq!(
            capture.ir.schema_version_string(),
            format!("{}.{}.{}", v.major, v.minor, v.patch)
        );
    }

    // `PrePass::SeamPlanning` captures the committed `SeamPlanIR` with the
    // corrected field names: `chosen_candidate.point` + `region_key` (NOT
    // `seam_xy`), and millimeter units on the seam point.
    {
        let tap = "PrePass::SeamPlanning";
        let capture = output
            .captures
            .iter()
            .find(|c| c.stage_id == tap)
            .unwrap_or_else(|| panic!("no capture recorded for tap '{tap}'"));
        assert_eq!(capture.layer_index, 0);
        assert_eq!(capture.layer_z, seeded.z);

        let CapturedIr::SeamPlan(captured_ir) = &capture.ir else {
            panic!(
                "tap '{tap}' must capture CapturedIr::SeamPlan, got {:?}",
                capture.ir
            );
        };
        assert_eq!(captured_ir.entries, seam_plan.entries);
        assert_eq!(
            captured_ir.entries[0].region_key,
            seam_plan.entries[0].region_key
        );
        let point = captured_ir.entries[0].chosen_candidate.point;
        let expected_point = seam_plan.entries[0].chosen_candidate.point;
        assert_eq!(point.x, expected_point.x);
        assert_eq!(point.y, expected_point.y);
        assert_eq!(point.z, expected_point.z);
        assert_eq!(point.width, expected_point.width);
        // Millimeter-scale values (single-digit mm), not 100-nm scaled
        // integers (which would read in the tens of thousands for the same
        // physical position).
        assert!(point.x.abs() < 1000.0 && point.y.abs() < 1000.0);
        assert_eq!(
            captured_ir.entries[0].scored_candidates[0].reason,
            SeamReason::Concave
        );

        let v = CURRENT_SEAM_PLAN_IR_SCHEMA_VERSION;
        assert_eq!(captured_ir.schema_version, v);
        assert_eq!(
            capture.ir.schema_version_string(),
            format!("{}.{}.{}", v.major, v.minor, v.patch)
        );
    }

    // `PrePass::SupportGeometry` captures the `SupportGeometryIR` +
    // `SupportPlanIR` composite, with millimeter units on `branch_segments`
    // points.
    {
        let tap = "PrePass::SupportGeometry";
        let capture = output
            .captures
            .iter()
            .find(|c| c.stage_id == tap)
            .unwrap_or_else(|| panic!("no capture recorded for tap '{tap}'"));
        assert_eq!(capture.layer_index, 0);
        assert_eq!(capture.layer_z, seeded.z);

        let CapturedIr::SupportGeometry { geometry, plan } = &capture.ir else {
            panic!(
                "tap '{tap}' must capture CapturedIr::SupportGeometry, got {:?}",
                capture.ir
            );
        };
        assert_eq!(geometry.entries, support_geometry.entries);
        assert_eq!(
            geometry.support_layer_height_mm,
            support_geometry.support_layer_height_mm
        );
        assert_eq!(
            geometry.support_top_z_distance_mm,
            support_geometry.support_top_z_distance_mm
        );
        assert_eq!(plan.entries, support_plan.entries);
        let branch_point = plan.entries[0].branch_segments[0].points[0];
        assert_eq!(branch_point.x, 1.0);
        assert_eq!(branch_point.y, 1.0);
        assert_eq!(branch_point.z, 0.2);
        // Millimeter-scale, not 100-nm scaled units.
        assert!(branch_point.x.abs() < 1000.0);

        let v = CURRENT_SUPPORT_GEOMETRY_IR_SCHEMA_VERSION;
        assert_eq!(geometry.schema_version, v);
        assert_eq!(plan.schema_version, CURRENT_SUPPORT_PLAN_IR_SCHEMA_VERSION);
        assert_eq!(
            capture.ir.schema_version_string(),
            format!("{}.{}.{}", v.major, v.minor, v.patch)
        );
    }

    // `PrePass::RegionMapping` captures the `RegionMapIR` retained alongside
    // the whole-print `Vec<SliceIR>` for a render-time join — no join
    // performed here. `RegionPlan.config` is pinned as a `ConfigId`.
    {
        let tap = "PrePass::RegionMapping";
        let capture = output
            .captures
            .iter()
            .find(|c| c.stage_id == tap)
            .unwrap_or_else(|| panic!("no capture recorded for tap '{tap}'"));
        assert_eq!(capture.layer_index, 0);
        assert_eq!(capture.layer_z, seeded.z);

        let CapturedIr::RegionMapping {
            region_map: captured_region_map,
            slice_ir: captured_slice_ir,
        } = &capture.ir
        else {
            panic!(
                "tap '{tap}' must capture CapturedIr::RegionMapping, got {:?}",
                capture.ir
            );
        };
        assert_eq!(captured_region_map.entries, region_map.entries);
        let key = RegionKey {
            global_layer_index: 0,
            object_id: "obj-0".to_string(),
            region_id: 7,
            variant_chain: Vec::new(),
        };
        let plan = captured_region_map
            .entries
            .get(&key)
            .expect("seeded RegionKey must survive capture");
        // `RegionPlan.config` is a `ConfigId` — resolvable via
        // `RegionMapIR::config_for`, not a raw `ResolvedConfig`.
        assert_eq!(
            captured_region_map.config_for(&key),
            region_map.config_for(&key)
        );
        let _: slicer_ir::ConfigId = plan.config;

        // Whole-print `Vec<SliceIR>` retained unfiltered, not joined here.
        assert_eq!(captured_slice_ir, &vec![seeded.clone()]);

        let v = CURRENT_REGION_MAP_IR_SCHEMA_VERSION;
        assert_eq!(captured_region_map.schema_version, v);
        assert_eq!(
            capture.ir.schema_version_string(),
            format!("{}.{}.{}", v.major, v.minor, v.patch)
        );
    }

    // Prepass-only proof: no arena per-layer stage sequence ran, and no
    // layer was executed-but-not-retained (both fields only exist to record
    // arena-closure activity, which this path never performs).
    assert!(
        output.closure_stage_ids.is_empty(),
        "Blackboard-read capture must not run any arena stage closure"
    );
    assert!(
        output.expansions.is_empty(),
        "Blackboard-read capture has no cross-layer dependency to expand"
    );
    assert_eq!(output.executed_layer_indices, vec![0]);
}

/// An unknown tap id is rejected before any Blackboard slot is touched,
/// mirroring the arena path's fail-closed contract.
#[test]
fn unknown_tap_is_rejected() {
    let blackboard = blackboard_with_committed_slice(seeded_slice_ir());
    let request = CaptureRequest {
        stage_ids: vec!["Layer::NotARealTap".to_string()],
        layer_indices: vec![0],
    };

    let err = execute_blackboard_taps(&blackboard, &request).unwrap_err();
    assert_eq!(
        err,
        CaptureExecutionError::UnknownTap {
            tap: "Layer::NotARealTap".to_string()
        }
    );
}

/// A requested layer index absent from the committed `Vec<SliceIR>` fails
/// closed with `NoApplicableLayer` rather than silently returning nothing.
#[test]
fn no_applicable_layer_is_rejected() {
    let blackboard = blackboard_with_committed_slice(seeded_slice_ir());
    let request = CaptureRequest {
        stage_ids: vec!["Layer::Slice".to_string()],
        layer_indices: vec![99],
    };

    let err = execute_blackboard_taps(&blackboard, &request).unwrap_err();
    assert_eq!(err, CaptureExecutionError::NoApplicableLayer);
}
