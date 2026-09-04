//! The cross-family overlap rejection must be scoped to one
//! `(global_layer_index, object_id)` identity.
//!
//! Two *different* print objects may legitimately choose different support
//! families (per-object `support_type`), and two entries on *different layers*
//! cannot physically collide at all. Neither case is a family-arbitration
//! conflict, so neither may annihilate the bodies involved.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use slicer_ir::{
    ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3, Polygon, SupportAnalysisIR,
    SupportGeometryKey, SupportPlanEntry, SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion,
};
use slicer_wasm_host::{
    exact_z_query::ExactZQueryService,
    support_aggregation::{
        try_aggregate_support_plans, SupportAggregationInput, SupportPlanProducer,
    },
    support_territory::SUPPORT_TERRITORY_CLEARANCE_KEY,
};

fn area(poly: &ExPolygon) -> f64 {
    fn ring(points: &[Point2]) -> f64 {
        points
            .iter()
            .enumerate()
            .map(|(i, a)| {
                let b = &points[(i + 1) % points.len()];
                (a.x as f64) * (b.y as f64) - (b.x as f64) * (a.y as f64)
            })
            .sum::<f64>()
            .abs()
            * 0.5
    }
    ring(&poly.contour.points) - poly.holes.iter().map(|h| ring(&h.points)).sum::<f64>()
}

fn overlap_area(a: &[ExPolygon], b: &[ExPolygon]) -> f64 {
    slicer_core::polygon_ops::intersection(a, b)
        .iter()
        .map(area)
        .sum()
}

fn body_regions(entry: &SupportPlanEntry) -> Vec<ExPolygon> {
    entry
        .roles
        .iter()
        .flat_map(|role| role.regions.iter().cloned())
        .collect()
}

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

/// Ownership at the merge point is default-deny: a region with no
/// `family_assignments` row has no owner, so a fixture that expects its entries
/// to be RETAINED must state who owns what. This grants every entry's
/// `(object_id, region_id)` to that entry's own family -- the "no contested
/// region" baseline these overlap fixtures implicitly assumed before ownership
/// was declared. It publishes no `support_territory`, so the territory clipper
/// stays disarmed and the legacy overlap guard is exercised unchanged.
fn family_assignments_for(entries: &[SupportPlanEntry]) -> SupportAnalysisIR {
    SupportAnalysisIR {
        family_assignments: entries
            .iter()
            .map(|entry| {
                (
                    (entry.object_id.clone(), entry.region_id),
                    entry.family_id.clone(),
                )
            })
            .collect::<BTreeMap<_, _>>(),
        ..SupportAnalysisIR::default()
    }
}

/// One producer per plan, holding the `support-family:<id>` claim for every
/// family that plan writes. Index-parallel to `plans` by construction.
fn producers_for(plans: &[SupportPlanIR]) -> Vec<SupportPlanProducer> {
    plans
        .iter()
        .map(|plan| SupportPlanProducer {
            module_id: "test.support-writer".into(),
            claims: plan
                .entries
                .iter()
                .map(|entry| format!("support-family:{}", entry.family_id))
                .collect(),
        })
        .collect()
}

/// Every entry across `plans`, in plan order.
fn all_entries(plans: &[SupportPlanIR]) -> Vec<SupportPlanEntry> {
    plans
        .iter()
        .flat_map(|plan| plan.entries.iter().cloned())
        .collect()
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
    let plans = vec![
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
    ];
    let owned = family_assignments_for(&all_entries(&plans));
    let result = try_aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &exact_z,
        territory: Some(&owned),
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
    let mut tree = entry(
        "tree",
        "tree-body",
        "object",
        0,
        polygon(100, 100, 300, 300),
    );
    tree.region_id = 0;
    let mut traditional = entry(
        "traditional",
        "normal-body",
        "object",
        7,
        polygon(200, 200, 400, 400),
    );
    // `family_assignments` is keyed `(object_id, region_id)` and is
    // layer-independent, so two families on one object must name two regions.
    // The overlap guard this test exercises is scoped to
    // `(global_layer_index, object_id)` and ignores `region_id`, so splitting
    // the regions leaves the property under test untouched.
    traditional.region_id = 1;
    let plans = vec![plan(tree), plan(traditional)];
    let owned = family_assignments_for(&all_entries(&plans));
    let result = try_aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &exact_z,
        territory: Some(&owned),
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

#[test]
fn different_anchor_planes_on_one_dispatch_layer_both_survive() {
    let exact_z = exact_z(&["object"]);
    let mut lower = entry(
        "tree",
        "lower-body",
        "object",
        0,
        polygon(100, 100, 300, 300),
    );
    lower.anchor_z = 20_000;
    let mut intermediate = entry(
        "tree",
        "intermediate-body",
        "object",
        0,
        polygon(100, 100, 300, 300),
    );
    intermediate.anchor_z = 21_000;

    let plans = vec![plan(lower), plan(intermediate)];
    let owned = family_assignments_for(&all_entries(&plans));
    let result = try_aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &exact_z,
        territory: Some(&owned),
    })
    .expect("structured aggregation");

    assert_eq!(result.retained.len(), 2);
    assert_eq!(
        result
            .retained
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<std::collections::BTreeSet<_>>(),
        [20_000, 21_000].into_iter().collect()
    );
}

/// The guard must still fire for its actual purpose: one object, one layer, two
/// families both claiming positive area in the same place — when the host
/// published no territory that could arbitrate between them.
#[test]
fn same_object_same_layer_cross_family_overlap_without_territory_still_rejected() {
    let exact_z = exact_z(&["object"]);
    let mut tree = entry(
        "tree",
        "tree-body",
        "object",
        0,
        polygon(100, 100, 300, 300),
    );
    tree.region_id = 0;
    let mut traditional = entry(
        "traditional",
        "normal-body",
        "object",
        0,
        polygon(200, 200, 400, 400),
    );
    traditional.region_id = 1;
    let plans = vec![plan(tree), plan(traditional)];
    // Both regions are legitimately owned by their writers; what is absent is
    // published `support_territory`, so nothing can arbitrate the overlap.
    let owned = family_assignments_for(&all_entries(&plans));
    let result = try_aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &exact_z,
        territory: Some(&owned),
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

/// Ticket 19: with territory published for the layer, the guard does not
/// annihilate. The traditional sub-region body keeps `∩ own`, the tree base
/// body keeps `- inflate(foreign, clearance)`, nothing is unmet, and the trim
/// is reported as a clip (Info 1205), not a rejection.
#[test]
fn same_object_same_layer_cross_family_overlap_with_territory_is_clipped() {
    let exact_z = exact_z(&["object"]);
    let mut tree = entry(
        "tree",
        "tree-body",
        "object",
        0,
        polygon(100, 100, 300, 300),
    );
    tree.region_id = 0;
    let mut traditional = entry(
        "traditional",
        "normal-body",
        "object",
        0,
        polygon(200, 200, 400, 400),
    );
    traditional.region_id = 1;
    let footprint = polygon(200, 200, 400, 400);
    let analysis = SupportAnalysisIR {
        family_assignments: BTreeMap::from([
            (("object".to_string(), 0), "tree".to_string()),
            (("object".to_string(), 1), "traditional".to_string()),
        ]),
        support_territory: HashMap::from([(
            SupportGeometryKey {
                global_support_layer_index: 0,
                object_id: "object".into(),
                region_id: 1,
            },
            vec![footprint.clone()],
        )]),
        // 0.001 mm = 10 units of base-side clearance.
        shared_settings: BTreeMap::from([(
            SUPPORT_TERRITORY_CLEARANCE_KEY.to_string(),
            "0.001".to_string(),
        )]),
        ..SupportAnalysisIR::default()
    };
    let plans = vec![plan(tree), plan(traditional)];
    let result = try_aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &exact_z,
        territory: Some(&analysis),
    })
    .expect("structured aggregation");

    assert!(result.unmet.is_empty(), "unmet={:?}", result.unmet);
    assert!(!result.degraded);
    assert!(
        !result
            .diagnostics
            .iter()
            .any(|d| d.reason == "body rejected: cross-family positive-area overlap"),
        "territory must arbitrate instead of the reject-both guard: {:?}",
        result.diagnostics
    );
    assert_eq!(result.retained.len(), 2, "both families survive");

    let tree = result
        .retained
        .iter()
        .find(|e| e.family_id == "tree")
        .expect("tree entry retained");
    let traditional = result
        .retained
        .iter()
        .find(|e| e.family_id == "traditional")
        .expect("traditional entry retained");
    assert_eq!(
        overlap_area(&body_regions(tree), &[footprint.clone()]),
        0.0,
        "tree geometry must not enter the modifier footprint: {:?}",
        tree.roles
    );
    let traditional_regions = body_regions(traditional);
    assert!(
        slicer_core::polygon_ops::difference(&traditional_regions, &[footprint.clone()]).is_empty(),
        "traditional geometry must stay inside its own footprint: {traditional_regions:?}"
    );
    assert!(
        overlap_area(&body_regions(tree), &traditional_regions) == 0.0,
        "the two families must not overlap after clipping"
    );
    // Tree: 200x200 body minus the 10-unit-inflated 200x200 footprint corner
    // (110x110 overlap) = 40_000 - 12_100. Traditional is unchanged, so it is
    // not reported.
    assert_eq!(result.clipped.len(), 1, "clipped={:?}", result.clipped);
    let clipped = &result.clipped[0];
    assert_eq!(clipped.family_id, "tree");
    assert_eq!(clipped.body_ids, vec!["tree-body".to_string()]);
    assert!(!clipped.dropped);
    assert!(
        (clipped.removed_area - 12_100.0).abs() < 1.0,
        "removed_area={}",
        clipped.removed_area
    );
    let tree_area: f64 = body_regions(tree).iter().map(area).sum();
    assert!((tree_area - 27_900.0).abs() < 1.0, "tree_area={tree_area}");
}

/// A layer that carries no territory keeps the legacy guard even when other
/// layers of the same object do carry it.
#[test]
fn territory_on_another_layer_does_not_disarm_the_guard() {
    let exact_z = exact_z(&["object"]);
    let mut tree = entry(
        "tree",
        "tree-body",
        "object",
        0,
        polygon(100, 100, 300, 300),
    );
    tree.region_id = 0;
    let mut traditional = entry(
        "traditional",
        "normal-body",
        "object",
        0,
        polygon(200, 200, 400, 400),
    );
    traditional.region_id = 1;
    let analysis = SupportAnalysisIR {
        family_assignments: BTreeMap::from([
            (("object".to_string(), 0), "tree".to_string()),
            (("object".to_string(), 1), "traditional".to_string()),
        ]),
        support_territory: HashMap::from([(
            SupportGeometryKey {
                global_support_layer_index: 5,
                object_id: "object".into(),
                region_id: 1,
            },
            vec![polygon(200, 200, 400, 400)],
        )]),
        ..SupportAnalysisIR::default()
    };
    let plans = vec![plan(tree), plan(traditional)];
    let result = try_aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &exact_z,
        territory: Some(&analysis),
    })
    .expect("structured aggregation");
    assert!(result.retained.is_empty());
    assert!(result.degraded);
    assert!(result.clipped.is_empty());
}
