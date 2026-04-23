#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use slicer_host::{
    Blackboard, BlackboardError, BlackboardPrepassSlot, LayerArena, LayerArenaError, LayerArenaSlot,
};
use slicer_ir::{
    BoundingBox3, ExtrusionPath3D, ExtrusionRole, GlobalLayer, InfillIR, LayerCollectionIR,
    LayerPaintMap, LayerPlanIR, MeshIR, ModuleInvocation, ObjectMesh, ObjectSurfaceData,
    PaintRegionIR, PerimeterIR, Point3, Point3WithWidth, PrintEntity, RegionKey, RegionMapIR,
    RegionPlan, ResolvedConfig, SemVer, SliceIR, SupportIR, SurfaceClassificationIR, ToolChange,
    Transform3d, ZHop,
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
    let paint = Arc::new(paint_regions_fixture());
    let region_map = Arc::new(region_map_fixture());

    blackboard
        .commit_surface_classification(Arc::clone(&surface))
        .expect("surface classification should commit once");
    blackboard
        .commit_layer_plan(Arc::clone(&layer_plan))
        .expect("layer plan should commit once");
    blackboard
        .commit_paint_regions(Arc::clone(&paint))
        .expect("paint regions should commit once");
    blackboard
        .commit_region_map(Arc::clone(&region_map))
        .expect("region map should commit once");

    expect_arc_ref(blackboard.mesh());
    expect_optional_arc_ref(blackboard.surface_classification());
    expect_optional_arc_ref(blackboard.layer_plan());
    expect_optional_arc_ref(blackboard.paint_regions());
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
            .paint_regions()
            .expect("paint regions should be visible as shared state"),
        &paint,
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

fn expect_arc_ref<T>(_: &Arc<T>) {}

fn expect_optional_arc_ref<T>(_: Option<&Arc<T>>) {}

fn expect_option_ref<T>(_: Option<&T>) {}

fn mesh_fixture() -> MeshIR {
    MeshIR {
        schema_version: semver(1, 0, 0),
        objects: vec![ObjectMesh {
            id: String::from("cube"),
            mesh: slicer_ir::IndexedTriangleSet {
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
            transform: Transform3d {
                matrix: identity4(),
            },
            config: slicer_ir::ObjectConfig {
                data: HashMap::new(),
            },
            modifier_volumes: Vec::new(),
            paint_data: None,
            world_z_extent: None,
        }],
        build_volume: BoundingBox3 {
            min: Point3 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            max: Point3 {
                x: 200.0,
                y: 200.0,
                z: 200.0,
            },
        },
    }
}

fn surface_fixture() -> SurfaceClassificationIR {
    SurfaceClassificationIR {
        schema_version: semver(1, 0, 0),
        per_object: HashMap::from([(
            String::from("cube"),
            ObjectSurfaceData {
                facet_classes: vec![slicer_ir::FacetClass::TopSurface],
                surface_groups: Vec::new(),
                bridge_regions: Vec::new(),
                overhang_regions: Vec::new(),
            },
        )]),
    }
}

fn layer_plan_fixture() -> LayerPlanIR {
    LayerPlanIR {
        schema_version: semver(1, 0, 0),
        global_layers: vec![
            GlobalLayer {
                index: 0,
                z: 0.2,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: true,
            },
            GlobalLayer {
                index: 1,
                z: 0.4,
                active_regions: Vec::new(),
                has_nonplanar: false,
                is_sync_layer: false,
            },
        ],
        object_participation: HashMap::new(),
    }
}

fn paint_regions_fixture() -> PaintRegionIR {
    PaintRegionIR {
        schema_version: semver(1, 0, 0),
        per_layer: HashMap::from([(
            0,
            LayerPaintMap {
                global_layer_index: 0,
                semantic_regions: HashMap::new(),
            },
        )]),
    }
}

fn region_map_fixture() -> RegionMapIR {
    RegionMapIR {
        schema_version: semver(1, 0, 0),
        entries: HashMap::from([(
            RegionKey {
                global_layer_index: 0,
                object_id: String::from("cube"),
                region_id: 0,
            },
            RegionPlan {
                config: ResolvedConfig::default(),
                stage_modules: HashMap::from([(
                    String::from("Layer::Perimeters"),
                    vec![ModuleInvocation {
                        module_id: String::from("com.example.perimeters"),
                        config_view: slicer_ir::ConfigView::new(),
                    }],
                )]),
            },
        )]),
    }
}

fn slice_fixture() -> SliceIR {
    SliceIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        z: 0.2,
        regions: Vec::new(),
    }
}

fn perimeter_fixture() -> PerimeterIR {
    PerimeterIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        regions: Vec::new(),
    }
}

fn infill_fixture() -> InfillIR {
    InfillIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        regions: Vec::new(),
    }
}

fn support_fixture() -> SupportIR {
    SupportIR {
        schema_version: semver(1, 0, 0),
        global_layer_index: 0,
        support_paths: Vec::new(),
        interface_paths: Vec::new(),
        raft_paths: Vec::new(),
        ironing_paths: Vec::new(),
    }
}

fn layer_collection_fixture(global_layer_index: u32, z: f32) -> LayerCollectionIR {
    LayerCollectionIR {
        schema_version: semver(1, 0, 0),
        global_layer_index,
        z,
        ordered_entities: vec![PrintEntity {
            path: ExtrusionPath3D {
                points: vec![Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                }],
                role: ExtrusionRole::OuterWall,
                speed_factor: 1.0,
            },
            role: ExtrusionRole::OuterWall,
            region_key: RegionKey {
                global_layer_index,
                object_id: String::from("cube"),
                region_id: 0,
            },
            topo_order: 0,
        }],
        tool_changes: vec![ToolChange {
            after_entity_index: 0,
            from_tool: 0,
            to_tool: 1,
        }],
        z_hops: vec![ZHop {
            after_entity_index: 0,
            hop_height: 0.6,
        }],
        annotations: vec![],
    }
}

fn semver(major: u32, minor: u32, patch: u32) -> SemVer {
    SemVer {
        major,
        minor,
        patch,
    }
}

fn identity4() -> [f64; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ]
}

// ── SeamPlan blackboard slot tests (TASK-159) ─────────────────────────────

#[test]
fn seam_plan_blackboard_slot_is_write_once() {
    // The SeamPlan blackboard slot follows the standard prepass write-once contract:
    // exactly one commit is allowed; a second commit must fail with
    // BlackboardError::DuplicatePrepassCommit { slot: SeamPlan }.
    use slicer_host::{Blackboard, BlackboardError, BlackboardPrepassSlot};
    use slicer_ir::{RegionKey, SeamPlanEntry, SeamPlanIR, SeamPosition, SemVer};

    let mesh = Arc::new(mesh_fixture());
    let mut blackboard = Blackboard::new(Arc::clone(&mesh), 0);

    // Build a minimal valid SeamPosition for the chosen_candidate field.
    let dummy_position = slicer_ir::Point3WithWidth {
        x: 0.0, y: 0.0, z: 0.0, width: 0.4, flow_factor: 1.0,
    };
    let seam_position = SeamPosition {
        point: dummy_position,
        wall_index: 0,
    };

    // Commit an empty SeamPlanIR.
    let plan = SeamPlanIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        entries: vec![SeamPlanEntry {
            region_key: RegionKey {
                global_layer_index: 0,
                object_id: "cube".to_string(),
                region_id: 1,
            },
            chosen_candidate: seam_position,
            scored_candidates: vec![],
        }],
    };

    let first = blackboard.commit_seam_plan(Arc::new(plan));
    assert!(
        first.is_ok(),
        "first SeamPlan commit should succeed, got {:?}",
        first.err()
    );

    // Second commit to the same slot must be rejected.
    let duplicate = SeamPlanIR {
        schema_version: SemVer { major: 1, minor: 0, patch: 0 },
        entries: vec![],
    };
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
