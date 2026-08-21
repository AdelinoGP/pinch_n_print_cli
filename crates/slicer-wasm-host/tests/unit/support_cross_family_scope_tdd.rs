//! The cross-family overlap rejection must be scoped to one
//! `(global_layer_index, object_id)` identity.
//!
//! Two *different* print objects may legitimately choose different support
//! families (per-object `support_type`), and two entries on *different layers*
//! cannot physically collide at all. Neither case is a family-arbitration
//! conflict, so neither may annihilate the bodies involved.

use std::sync::Arc;

use slicer_ir::{
    ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3, Polygon, SupportPlanEntry,
    SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion,
};
use slicer_wasm_host::{
    exact_z_query::ExactZQueryService,
    support_aggregation::{try_aggregate_support_plans, SupportAggregationInput},
};

fn polygon(x0: i64, y0: i64, x1: i64, y1: i64) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: x0, y: y0 },
                Point2 { x: x1, y: y0 },
                Point2 { x: x1, y: y1 },
                Point2 { x: x0, y: y1 },
            ],
        },
        holes: Vec::new(),
    }
}

fn entry(
    family_id: &str,
    body_id: &str,
    object_id: &str,
    global_layer_index: i32,
    body: ExPolygon,
) -> SupportPlanEntry {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index,
        object_id: object_id.into(),
        region_id: 0,
        family_id: family_id.into(),
        demand_ids: vec![format!("{body_id}-demand")],
        body_ids: vec![body_id.into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![body],
        }],
        skeleton: None,
        capabilities: Vec::new(),
        provenance: vec![family_id.into()],
        decline_reason: None,
    }
}

fn plan(entry: SupportPlanEntry) -> SupportPlanIR {
    SupportPlanIR {
        entries: vec![entry],
        ..SupportPlanIR::default()
    }
}

/// Two flat triangles, one per object. A coplanar triangle has no cross-section
/// at any Z, so `occupancy` is empty and `validate_entry`'s exact-Z rejection
/// cannot fire: these tests isolate the cross-family overlap guard.
fn exact_z(object_ids: &[&str]) -> ExactZQueryService {
    ExactZQueryService::new(Arc::new(MeshIR {
        objects: object_ids
            .iter()
            .map(|id| ObjectMesh {
                id: (*id).into(),
                mesh: IndexedTriangleSet {
                    vertices: vec![
                        Point3 {
                            x: 0.0,
                            y: 0.0,
                            z: 100.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 0.0,
                            z: 100.0,
                        },
                        Point3 {
                            x: 10.0,
                            y: 10.0,
                            z: 100.0,
                        },
                    ],
                    indices: vec![0, 1, 2],
                },
                ..ObjectMesh::default()
            })
            .collect(),
        ..MeshIR::default()
    }))
}

#[test]
fn different_objects_choosing_different_families_both_survive_xy_overlap() {
    let exact_z = exact_z(&["object-a", "object-b"]);
    let result = try_aggregate_support_plans(SupportAggregationInput {
        plans: vec![
            plan(entry(
                "tree",
                "tree-body",
                "object-a",
                0,
                polygon(100, 100, 300, 300),
            )),
            plan(entry(
                "traditional",
                "normal-body",
                "object-b",
                0,
                polygon(200, 200, 400, 400),
            )),
        ],
        exact_z: &exact_z,
    })
    .expect("structured aggregation");

    let cross_family: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.reason == "body rejected: cross-family positive-area overlap")
        .collect();
    assert!(
        cross_family.is_empty(),
        "two distinct objects must not annihilate each other's support: {cross_family:?}"
    );
    assert_eq!(result.retained.len(), 2, "both bodies must survive");
    assert!(result.retained.iter().any(|e| e.family_id == "tree"));
    assert!(result.retained.iter().any(|e| e.family_id == "traditional"));
    assert!(result.unmet.is_empty());
    assert!(!result.degraded);
}

#[test]
fn different_layers_of_one_object_both_survive_xy_overlap() {
    let exact_z = exact_z(&["object"]);
    let result = try_aggregate_support_plans(SupportAggregationInput {
        plans: vec![
            plan(entry(
                "tree",
                "tree-body",
                "object",
                0,
                polygon(100, 100, 300, 300),
            )),
            plan(entry(
                "traditional",
                "normal-body",
                "object",
                7,
                polygon(200, 200, 400, 400),
            )),
        ],
        exact_z: &exact_z,
    })
    .expect("structured aggregation");

    let cross_family: Vec<_> = result
        .diagnostics
        .iter()
        .filter(|d| d.reason == "body rejected: cross-family positive-area overlap")
        .collect();
    assert!(
        cross_family.is_empty(),
        "entries on different layers cannot physically collide: {cross_family:?}"
    );
    assert_eq!(result.retained.len(), 2, "both bodies must survive");
    assert!(!result.degraded);
}

/// The guard must still fire for its actual purpose: one object, one layer, two
/// families both claiming positive area in the same place.
#[test]
fn same_object_same_layer_cross_family_overlap_still_rejected() {
    let exact_z = exact_z(&["object"]);
    let mut tree = entry("tree", "tree-body", "object", 0, polygon(100, 100, 300, 300));
    tree.region_id = 0;
    let mut traditional = entry(
        "traditional",
        "normal-body",
        "object",
        0,
        polygon(200, 200, 400, 400),
    );
    traditional.region_id = 1;
    let result = try_aggregate_support_plans(SupportAggregationInput {
        plans: vec![plan(tree), plan(traditional)],
        exact_z: &exact_z,
    })
    .expect("structured aggregation");

    assert!(result.retained.is_empty());
    assert_eq!(
        result
            .diagnostics
            .iter()
            .filter(|d| d.reason == "body rejected: cross-family positive-area overlap")
            .count(),
        2
    );
    assert!(result.degraded);
}
