/// Host-owned support plan aggregation and complete-body validation contract.
use std::sync::Arc;

use slicer_ir::{
    ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3, SupportAnalysisIR,
    SupportPlanEntry, SupportPlanIR, Transform3d,
};
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::{
    aggregate_support_plans, OwnershipReason, SupportAggregationInput, SupportPlanProducer,
};

/// Ownership at the merge point is default-deny: a region with no
/// `family_assignments` row has no owner, so a fixture that expects its entries
/// to be RETAINED has to say who owns what. This grants every entry's
/// `(object_id, region_id)` to that entry's own family, which is the "no
/// contested region" baseline the pre-ownership fixtures implicitly assumed.
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
            .collect(),
        ..SupportAnalysisIR::default()
    }
}

/// A producer per plan, holding the `support-family:<id>` claim for every
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

/// All entries across `plans`, in plan order.
fn all_entries(plans: &[SupportPlanIR]) -> Vec<SupportPlanEntry> {
    plans
        .iter()
        .flat_map(|plan| plan.entries.iter().cloned())
        .collect()
}

fn square(x: i64, y: i64, size: i64) -> ExPolygon {
    ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                Point2 { x, y },
                Point2 { x: x + size, y },
                Point2 {
                    x: x + size,
                    y: y + size,
                },
                Point2 { x, y: y + size },
            ],
        },
        holes: Vec::new(),
    }
}

fn mesh() -> MeshIR {
    MeshIR {
        objects: vec![ObjectMesh {
            id: "object-a".into(),
            mesh: IndexedTriangleSet {
                vertices: vec![
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 10.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 10.0,
                        z: 0.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 0.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 10.0,
                        y: 10.0,
                        z: 10.0,
                    },
                    Point3 {
                        x: 0.0,
                        y: 10.0,
                        z: 10.0,
                    },
                ],
                indices: vec![
                    0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6,
                    7, 2, 7, 3, 3, 7, 4, 3, 4, 0,
                ],
            },
            transform: Transform3d {
                matrix: [
                    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
                ],
            },
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn entry(body: &str, x: i64) -> slicer_ir::SupportPlanEntry {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    slicer_ir::SupportPlanEntry {
        global_layer_index: 0,
        object_id: "object-a".into(),
        region_id: 7,
        family_id: "tree".into(),
        demand_ids: vec![body.into()],
        body_ids: vec![body.into()],
        anchor_layer_index: 0,
        anchor_z: 4_321,
        roles: vec![slicer_ir::SupportPlanRoleRegion {
            role: slicer_ir::SupportPlanRole::SupportBody,
            regions: vec![square(x, 5_000, 1_000)],
        }],
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["test".into()],
        decline_reason: None,
    }
}

#[test]
pub fn support_plan_validation() {
    let service = ExactZQueryService::new(Arc::new(mesh()));
    let mut colliding = entry("colliding", 0);
    colliding.region_id = 8;
    // Genuinely oversized: one unit wider than MAX_BODY_EXTENT_UNITS (1 << 20), so
    // it fits in no cell-sized territory. (Before packet 224 this fixture was a
    // 1_000-unit body parked across the x = 1 << 20 grid line, which pinned the
    // absolute-grid defect rather than the size contract.)
    let mut spans_cell = entry("spans_cell", 30_000_000);
    spans_cell.region_id = 9;
    spans_cell.roles[0].regions = vec![square(30_000_000, 5_000, (1 << 20) + 1)];
    let plans = vec![slicer_ir::SupportPlanIR {
        entries: vec![entry("valid", 20_000_000), colliding, spans_cell],
        ..Default::default()
    }];
    let owned = family_assignments_for(&all_entries(&plans));
    let result = aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &service,
        territory: Some(&owned),
    });
    assert_eq!(result.retained.len(), 1);
    assert_eq!(result.retained[0].body_ids, vec!["valid"]);
    assert!(result.degraded);
    assert!(result
        .unmet
        .iter()
        .any(|d| d.demand_id == "colliding" && d.reason.contains("occupancy")));
    assert!(result
        .unmet
        .iter()
        .any(|d| d.demand_id == "spans_cell" && d.reason.contains("max-body-extent")));
}

#[test]
fn support_plan_aggregation_preserves_distinct_families() {
    let service = ExactZQueryService::new(Arc::new(mesh()));
    let mut tree = entry("tree-body", 20_000_000);
    tree.family_id = "tree".into();
    let mut traditional = entry("traditional-body", 30_000_000);
    traditional.family_id = "traditional".into();
    traditional.region_id = 8;

    let plans = vec![
        slicer_ir::SupportPlanIR {
            entries: vec![tree],
            ..Default::default()
        },
        slicer_ir::SupportPlanIR {
            entries: vec![traditional],
            ..Default::default()
        },
    ];
    let owned = family_assignments_for(&all_entries(&plans));
    let result = aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &service,
        territory: Some(&owned),
    });

    assert_eq!(result.retained.len(), 2);
    assert_eq!(result.retained[0].family_id, "tree");
    assert_eq!(result.retained[1].family_id, "traditional");
    assert!(result.duplicates.is_empty());
}

/// Ownership is declared, never raced for (packet 241b). When a `tree` entry
/// and a `traditional` entry name one `(layer, object, region)` identity, the
/// survivor is the family the host ASSIGNED the region to -- not whichever plan
/// happened to arrive first. So the outcome must be identical in both plan
/// orders, and the loser must be named as the trespasser.
#[test]
fn support_plan_aggregation_diagnoses_duplicate_identity() {
    let service = ExactZQueryService::new(Arc::new(mesh()));
    let mut tree = entry("tree-body", 20_000_000);
    tree.family_id = "tree".into();
    let mut traditional = entry("traditional-body", 30_000_000);
    traditional.family_id = "traditional".into();
    // `entry` pins layer 0 / object-a / region 7 for both: one contested identity.
    assert_eq!(tree.object_id, traditional.object_id);
    assert_eq!(tree.region_id, traditional.region_id);

    // The host assigned the contested region to `traditional`.
    let owned = SupportAnalysisIR {
        family_assignments: [((tree.object_id.clone(), tree.region_id), "traditional".to_string())]
            .into_iter()
            .collect(),
        ..SupportAnalysisIR::default()
    };

    let orders = [
        ("[tree, traditional]", vec![tree.clone(), traditional.clone()]),
        ("[traditional, tree]", vec![traditional.clone(), tree.clone()]),
    ];
    for (order, entries) in orders {
        let plans = entries
            .into_iter()
            .map(|entry| SupportPlanIR {
                entries: vec![entry],
                ..Default::default()
            })
            .collect::<Vec<_>>();
        let result = aggregate_support_plans(SupportAggregationInput {
            producers: producers_for(&plans),
            plans,
            exact_z: &service,
            territory: Some(&owned),
        });

        assert_eq!(
            result.retained.len(),
            1,
            "order {order}: exactly the owning family survives, got {:?}",
            result
                .retained
                .iter()
                .map(|e| e.family_id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            result.retained[0].family_id, "traditional",
            "order {order}: the assigned owner keeps the region"
        );
        assert!(result.degraded, "order {order}: a trespass degrades");
        assert_eq!(
            result.ownership_violations.len(),
            1,
            "order {order}: one violation expected, got {:?}",
            result.ownership_violations
        );
        assert_eq!(
            result.ownership_violations[0].family_id, "tree",
            "order {order}: the diagnostic must name the trespasser"
        );
        assert_eq!(
            result.ownership_violations[0].reason,
            OwnershipReason::WrongFamily {
                owner: "traditional".to_string()
            },
            "order {order}"
        );
    }
}

/// Regression (packet 224): the host bounds how much territory one body may
/// claim by extent, not by forbidding particular world coordinates.
/// A small, perfectly printable body positioned across y = 0 must be retained.
/// Historical defect (packet 224, RC-14): tree contact tips at the model edge
/// extend to y = -0.4 mm, and the since-deleted bbox-centroid routing-cell grid
/// rejected every straddling layer. Aggregation no longer partitions by position
/// at all - `in_routing_cell` is a pure max-body-extent bound - so this test now
/// guards against reintroducing any position-dependent rejection.
#[test]
fn support_body_straddling_absolute_cell_boundary_is_retained() {
    let service = ExactZQueryService::new(Arc::new(mesh()));
    // 4 mm square well clear of the 0..10 mm mesh footprint in x, spanning
    // y = -0.4 mm .. +3.6 mm so it crosses the y = 0 cell boundary.
    let mut straddling = entry("straddles_boundary", 200_000);
    straddling.region_id = 11;
    straddling.roles[0].regions = vec![square(200_000, -4_000, 40_000)];

    let plans = vec![slicer_ir::SupportPlanIR {
        entries: vec![straddling],
        ..Default::default()
    }];
    let owned = family_assignments_for(&all_entries(&plans));
    let result = aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &service,
        territory: Some(&owned),
    });

    assert!(
        result.unmet.is_empty(),
        "straddling body must not be rejected, got: {:?}",
        result.unmet
    );
    assert_eq!(result.retained.len(), 1);
    assert_eq!(result.retained[0].body_ids, vec!["straddles_boundary"]);
}

/// Regression (packet 224): a repeated `(layer, object, region)` identity from
/// ONE family is not a family conflict. It is two entries for one region from a
/// single writer (body plus interface candidate, or two candidates at the same
/// layer), and must be combined by same-family union rather than reported as a
/// duplicate. Before this fix a wedge slice emitted a flood of code-1202
/// "families 'traditional' and 'traditional'" diagnostics, and every
/// non-`traditional` family additionally had the second entry silently dropped.
#[test]
fn same_family_duplicate_identity_unions_without_a_duplicate_diagnostic() {
    let service = ExactZQueryService::new(Arc::new(mesh()));
    // Same family, same identity (`entry` pins layer 0 / object-a / region 7),
    // same body id so same-family union merges them into one entry.
    let mut first = entry("tree-body", 20_000_000);
    first.demand_ids = vec!["demand-a".into()];
    let mut second = entry("tree-body", 20_100_000);
    second.demand_ids = vec!["demand-b".into()];

    let plans = vec![slicer_ir::SupportPlanIR {
        entries: vec![first, second],
        ..Default::default()
    }];
    let owned = family_assignments_for(&all_entries(&plans));
    let result = aggregate_support_plans(SupportAggregationInput {
        producers: producers_for(&plans),
        plans,
        exact_z: &service,
        territory: Some(&owned),
    });

    assert!(
        result.duplicates.is_empty(),
        "same-family repeats are not duplicates: {:?}",
        result.duplicates
    );
    assert!(!result.degraded, "same-family union must not degrade");
    assert_eq!(result.retained.len(), 1);
    assert_eq!(
        result.retained[0].demand_ids,
        vec!["demand-a", "demand-b"],
        "both demands must survive the union"
    );
    assert_eq!(
        result.retained[0].roles[0].regions.len(),
        2,
        "both bodies' geometry must survive the union"
    );
}
