#![allow(missing_docs)]

use slicer_sdk::test_support::fixtures::extrusion_path3d_base;
use std::collections::HashMap;
use std::sync::Arc;

use slicer_ir::{
    BoundingBox3, ExPolygon, ExtrusionPath3D, ExtrusionRole, GlobalLayer, InfillIR,
    LayerCollectionIR, LayerPlanIR, MeshIR, ModuleInvocation, ObjectMesh, ObjectSurfaceData,
    OverhangRegion, PerimeterIR, Point2, Point3, Point3WithWidth, Polygon, PrintEntity,
    QuartileBand, RegionKey, RegionMapIR, RegionPlan, SliceIR, SlicedRegion, SupportIR,
    SurfaceClassificationIR, SurfaceGroup, ToolChange, Transform3d, ZHop,
};
use slicer_runtime::{
    Blackboard, BlackboardError, BlackboardPrepassSlot, LayerArena, LayerArenaError, LayerArenaSlot,
};

// Contract notes:
// - Docs require host-owned immutable Blackboard IRs and write-once per-layer slots.
// - OrcaSlicer only provides loose context: ordered per-layer result handoff in GCode.cpp and
//   host thread-count constraints in Thread.cpp, not a direct API template.

#[test]
fn blackboard_contract_exposes_arc_backed_prepass_reads_and_exactly_once_layer_drain() {
    let mesh = Arc::new(mesh_fixture());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 2);

    let surface = Arc::new(surface_fixture());
    let layer_plan = Arc::new(layer_plan_fixture());
    let region_map = Arc::new(region_map_fixture());

    blackboard
        .commit_surface_classification(Arc::clone(&surface))
        .expect("surface classification should commit once");
    blackboard
        .commit_layer_plan(Arc::clone(&layer_plan))
        .expect("layer plan should commit once");
    // Note: PaintRegionIR blackboard slot removed in packet 95 sub-step 16.
    // Paint annotations now live in SliceIR segment_annotations (AC-16).
    blackboard
        .commit_region_map(Arc::clone(&region_map))
        .expect("region map should commit once");

    expect_arc_ref(blackboard.mesh());
    expect_optional_arc_ref(blackboard.surface_classification());
    expect_optional_arc_ref(blackboard.layer_plan());
    expect_optional_arc_ref(blackboard.region_map());

    assert!(Arc::ptr_eq(blackboard.mesh(), &mesh));
    assert!(Arc::ptr_eq(
        blackboard
            .surface_classification()
            .expect("surface classification should be visible as shared state"),
        &surface,
    ));
    assert!(Arc::ptr_eq(
        blackboard
            .layer_plan()
            .expect("layer plan should be visible as shared state"),
        &layer_plan,
    ));
    assert!(Arc::ptr_eq(
        blackboard
            .region_map()
            .expect("region map should be visible as shared state"),
        &region_map,
    ));

    blackboard
        .commit_layer_output(0, layer_collection_fixture(0, 0.2))
        .expect("layer 0 output should commit once");
    blackboard
        .commit_layer_output(1, layer_collection_fixture(1, 0.4))
        .expect("layer 1 output should commit once");

    let drained = blackboard
        .drain_layer_outputs()
        .expect("all layer outputs should drain into Vec after the layer loop");

    assert_eq!(drained.len(), 2);
    assert_eq!(drained[0].global_layer_index, 0);
    assert_eq!(drained[1].global_layer_index, 1);
}

#[test]
fn blackboard_contract_rejects_duplicate_prepass_and_layer_commits_plus_incomplete_or_double_drain()
{
    let mut blackboard = Blackboard::new(Arc::new(mesh_fixture()), 2);

    blackboard
        .commit_surface_classification(Arc::new(surface_fixture()))
        .expect("first prepass commit should succeed");
    assert_eq!(
        blackboard.commit_surface_classification(Arc::new(surface_fixture())),
        Err(BlackboardError::DuplicatePrepassCommit {
            slot: BlackboardPrepassSlot::SurfaceClassification,
        })
    );

    blackboard
        .commit_layer_output(0, layer_collection_fixture(0, 0.2))
        .expect("first layer output commit should succeed");
    assert_eq!(
        blackboard.commit_layer_output(0, layer_collection_fixture(0, 0.2)),
        Err(BlackboardError::DuplicateLayerCommit { layer_index: 0 })
    );
    assert_eq!(
        blackboard.drain_layer_outputs(),
        Err(BlackboardError::IncompleteLayerDrain {
            missing_indices: vec![1],
        })
    );

    blackboard
        .commit_layer_output(1, layer_collection_fixture(1, 0.4))
        .expect("second layer output commit should succeed");
    blackboard
        .drain_layer_outputs()
        .expect("complete layer output set should drain once");
    assert_eq!(
        blackboard.drain_layer_outputs(),
        Err(BlackboardError::LayerOutputsAlreadyDrained)
    );
}

#[test]
fn layer_arena_contract_stages_ephemeral_intermediates_with_shared_borrows_take_and_reset() {
    let mut arena = LayerArena::new();

    assert!(arena.slice().is_none());
    assert!(arena.perimeter().is_none());
    assert!(arena.infill().is_none());
    assert!(arena.support().is_none());

    arena
        .set_slice(slice_fixture())
        .expect("slice should stage into an empty arena slot");
    assert_eq!(
        arena.set_slice(slice_fixture()),
        Err(LayerArenaError::SlotAlreadyOccupied {
            slot: LayerArenaSlot::Slice,
        })
    );
    arena
        .set_perimeter(perimeter_fixture())
        .expect("perimeter should stage into an empty arena slot");
    arena
        .set_infill(infill_fixture())
        .expect("infill should stage into an empty arena slot");
    arena
        .set_support(support_fixture())
        .expect("support should stage into an empty arena slot");

    expect_option_ref(arena.slice());
    expect_option_ref(arena.perimeter());
    expect_option_ref(arena.infill());
    expect_option_ref(arena.support());

    assert_eq!(
        arena
            .slice()
            .expect("slice should be borrowed immutably while staged")
            .global_layer_index,
        0
    );
    assert_eq!(
        arena
            .take_slice()
            .expect("slice should move out exactly once")
            .global_layer_index,
        0
    );
    assert!(arena.slice().is_none());

    arena.reset();

    assert!(arena.slice().is_none());
    assert!(arena.perimeter().is_none());
    assert!(arena.infill().is_none());
    assert!(arena.support().is_none());
}

#[test]
fn layer_arena_prepares_both_region_views_once_and_reuses_them() {
    let classification = prepared_surface_fixture();
    let mut arena = LayerArena::new();
    arena
        .set_slice(prepared_slice_fixture())
        .expect("slice should stage");

    let ordinary_ptr = {
        let prepared = arena
            .ensure_prepared_regions(Some(&classification))
            .expect("ordinary regions should prepare");
        assert_prepared_region_fields(&prepared[0]);
        prepared.as_ptr()
    };
    assert_eq!(
        ordinary_ptr,
        arena
            .ensure_prepared_regions(None)
            .expect("prepared data should be reused without re-derivation")
            .as_ptr()
    );

    let perimeter_ptr = arena
        .ensure_prepared_perimeter_source_regions(Some(&classification))
        .expect("perimeter-source regions should prepare")
        .as_ptr();
    assert_eq!(
        perimeter_ptr,
        arena
            .ensure_prepared_perimeter_source_regions(None)
            .expect("prepared perimeter data should be reused")
            .as_ptr()
    );
    assert_prepared_region_fields(
        &arena
            .prepared_perimeter_source_regions()
            .expect("perimeter getter should expose prepared data")[0],
    );
}

#[test]
fn layer_arena_take_slice_invalidates_prepared_region_data() {
    let mut arena = prepared_arena();

    assert!(arena.take_slice().is_some());

    assert!(arena.prepared_regions().is_none());
    assert!(arena.prepared_perimeter_source_regions().is_none());
}

#[test]
fn layer_arena_successful_set_slice_starts_unprepared() {
    let mut arena = LayerArena::new();

    arena
        .set_slice(prepared_slice_fixture())
        .expect("empty slice slot should accept a new slice");

    assert!(arena.prepared_regions().is_none());
    assert!(arena.prepared_perimeter_source_regions().is_none());
}

#[test]
fn layer_arena_reset_clears_prepared_region_data() {
    let mut arena = prepared_arena();

    arena.reset();

    assert!(arena.prepared_regions().is_none());
    assert!(arena.prepared_perimeter_source_regions().is_none());
}

#[test]
fn layer_arena_failed_set_slice_preserves_existing_prepared_pairing() {
    let mut arena = prepared_arena();
    let ordinary_ptr = arena
        .prepared_regions()
        .expect("fixture prepares ordinary regions")
        .as_ptr();
    let perimeter_ptr = arena
        .prepared_perimeter_source_regions()
        .expect("fixture prepares perimeter regions")
        .as_ptr();

    assert_eq!(
        arena.set_slice(SliceIR {
            global_layer_index: 99,
            ..SliceIR::default()
        }),
        Err(LayerArenaError::SlotAlreadyOccupied {
            slot: LayerArenaSlot::Slice,
        })
    );

    assert_eq!(
        ordinary_ptr,
        arena
            .prepared_regions()
            .expect("failed set must preserve ordinary preparation")
            .as_ptr()
    );
    assert_eq!(
        perimeter_ptr,
        arena
            .prepared_perimeter_source_regions()
            .expect("failed set must preserve perimeter preparation")
            .as_ptr()
    );
    assert_eq!(
        arena
            .slice()
            .expect("failed set must preserve existing slice")
            .global_layer_index,
        7
    );
}

fn prepared_arena() -> LayerArena {
    let classification = prepared_surface_fixture();
    let mut arena = LayerArena::new();
    arena
        .set_slice(prepared_slice_fixture())
        .expect("fixture slice should stage");
    arena
        .ensure_prepared_regions(Some(&classification))
        .expect("fixture ordinary regions should prepare");
    arena
        .ensure_prepared_perimeter_source_regions(Some(&classification))
        .expect("fixture perimeter regions should prepare");
    arena
}

fn assert_prepared_region_fields(prepared: &slicer_ir::PreparedRegionData) {
    assert!(prepared.needs_support);
    assert_eq!(
        prepared.surface_group.as_ref().map(|group| group.id),
        Some(5)
    );
    assert_eq!(prepared.overhang_quartile_polygons.len(), 1);
    assert_eq!(prepared.overhang_quartile_polygons[0].quartile, 2);
    assert_eq!(prepared.overhang_areas.len(), 1);
    assert_eq!(prepared.prev_layer_boundary.len(), 1);
}

fn prepared_slice_fixture() -> SliceIR {
    SliceIR {
        global_layer_index: 7,
        z: 1.6,
        regions: vec![SlicedRegion {
            object_id: "cube".into(),
            region_id: 3,
            polygons: vec![rect(0.0, 0.0, 10.0, 10.0)],
            nonplanar_surface: Some(5),
            ..SlicedRegion::default()
        }],
        ..SliceIR::default()
    }
}

fn prepared_surface_fixture() -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        per_object: HashMap::from([(
            "cube".into(),
            ObjectSurfaceData {
                surface_groups: vec![SurfaceGroup {
                    id: 5,
                    printable: true,
                    ..SurfaceGroup::default()
                }],
                overhang_regions: vec![OverhangRegion {
                    needs_support: true,
                    xy_footprint: vec![rect(1.0, 1.0, 2.0, 2.0)],
                    ..OverhangRegion::default()
                }],
                ..ObjectSurfaceData::default()
            },
        )]),
        overhang_quartile_polygons: HashMap::from([(
            "cube".into(),
            HashMap::from([(
                7,
                vec![QuartileBand {
                    quartile: 2,
                    polygons: vec![rect(5.0, 5.0, 15.0, 15.0)],
                }],
            )]),
        )]),
        prev_layer_boundaries: HashMap::from([(
            "cube".into(),
            HashMap::from([(7, vec![rect(0.0, 0.0, 9.0, 9.0)])]),
        )]),
        ..SurfaceClassificationIR::default()
    }
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(x0, y0),
                Point2::from_mm(x1, y0),
                Point2::from_mm(x1, y1),
                Point2::from_mm(x0, y1),
            ],
        },
        holes: Vec::new(),
    }
}

fn expect_arc_ref<T>(_: &Arc<T>) {}

fn expect_optional_arc_ref<T>(_: Option<&Arc<T>>) {}

fn expect_option_ref<T>(_: Option<&T>) {}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: String::from("cube"),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![
                    Point3::default(),
                    Point3 {
                        x: 1.0,
                        ..Default::default()
                    },
                    Point3 {
                        y: 1.0,
                        ..Default::default()
                    },
                ],
                indices: vec![0, 1, 2],
            },
            transform: Transform3d {
                matrix: identity4(),
            },
            ..Default::default()
        }],
        build_volume: BoundingBox3 {
            min: Point3::default(),
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
        ..Default::default()
    }
}

fn surface_fixture() -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        per_object: HashMap::from([(
            String::from("cube"),
            ObjectSurfaceData {
                facet_classes: vec![slicer_ir::FacetClass::TopSurface],
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

fn layer_plan_fixture() -> LayerPlanIR {
    LayerPlanIR {
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                is_sync_layer: true,
                // active_regions and has_nonplanar retain their Default values.
                ..Default::default()
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

fn region_map_fixture() -> RegionMapIR {
    RegionMapIR {
        entries: HashMap::from([(
            RegionKey {
                global_layer_index: 0,
                object_id: String::from("cube"),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            RegionPlan {
                stage_modules: HashMap::from([(
                    String::from("Layer::Perimeters"),
                    vec![ModuleInvocation {
                        module_id: String::from("com.example.perimeters"),
                        ..Default::default()
                    }],
                )]),
                ..Default::default()
            },
        )]),
        ..Default::default()
    }
}

fn slice_fixture() -> SliceIR {
    SliceIR {
        z: 0.2,
        ..Default::default()
    }
}

fn perimeter_fixture() -> PerimeterIR {
    PerimeterIR::default()
}

fn infill_fixture() -> InfillIR {
    InfillIR::default()
}

fn support_fixture() -> SupportIR {
    SupportIR::default()
}

fn layer_collection_fixture(global_layer_index: u32, z: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        global_layer_index,
        z,
        // exhaustive: fixture specifies the complete PrintEntity boundary.
        ordered_entities: vec![PrintEntity {
            entity_id: 1,
            path: ExtrusionPath3D {
                points: vec![Point3WithWidth {
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                    ..Default::default()
                }],
                ..extrusion_path3d_base(ExtrusionRole::OuterWall)
            },
            role: ExtrusionRole::OuterWall,
            tool_index: 0,
            region_key: RegionKey {
                global_layer_index,
                object_id: String::from("cube"),
                region_id: 0,
                variant_chain: Vec::new(),
            },
            topo_order: 0,
        }],
        tool_changes: vec![ToolChange {
            to_tool: 1,
            ..Default::default()
        }],
        z_hops: vec![ZHop {
            after_entity_index: 0,
            hop_height: 0.6,
        }],
        ..Default::default()
    }
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

// â”€â”€ SeamPlan blackboard slot tests (TASK-159) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[test]
fn seam_plan_blackboard_slot_is_write_once() {
    // The SeamPlan blackboard slot follows the standard prepass write-once contract:
    // exactly one commit is allowed; a second commit must fail with
    // BlackboardError::DuplicatePrepassCommit { slot: SeamPlan }.
    use slicer_ir::{RegionKey, SeamPlanEntry, SeamPlanIR, SeamPosition};
    use slicer_runtime::{Blackboard, BlackboardError, BlackboardPrepassSlot};

    let mesh = Arc::new(mesh_fixture());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    // Build a minimal valid SeamPosition for the chosen_candidate field.
    let dummy_position = slicer_ir::Point3WithWidth {
        width: 0.4,
        flow_factor: 1.0,
        dist_to_top_mm: 0.0,
        overhang_distance_mm: None,
        ..Default::default()
    };
    let seam_position = SeamPosition {
        point: dummy_position,
        ..Default::default()
    };

    // Commit an empty SeamPlanIR.
    let plan = SeamPlanIR {
        entries: vec![SeamPlanEntry {
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "cube".to_string(),
                region_id: 1,
                variant_chain: Vec::new(),
            },
            chosen_candidate: seam_position,
            ..Default::default()
        }],
        ..Default::default()
    };

    let first = blackboard.commit_seam_plan(Arc::new(plan));
    assert!(
        first.is_ok(),
        "first SeamPlan commit should succeed, got {:?}",
        first.err()
    );

    // Second commit to the same slot must be rejected.
    let duplicate = SeamPlanIR::default();
    let second = blackboard.commit_seam_plan(Arc::new(duplicate));
    assert!(
        second.is_err(),
        "duplicate SeamPlan commit must be rejected"
    );
    match second.unwrap_err() {
        BlackboardError::DuplicatePrepassCommit { slot } => {
            assert_eq!(slot, BlackboardPrepassSlot::SeamPlan);
        }
        other => panic!("expected DuplicatePrepassCommit {{ slot: SeamPlan }}, got {other:?}"),
    }

    // The slot should be readable after the first commit.
    assert!(
        blackboard.seam_plan().is_some(),
        "seam_plan() should return Some after first commit"
    );
}
