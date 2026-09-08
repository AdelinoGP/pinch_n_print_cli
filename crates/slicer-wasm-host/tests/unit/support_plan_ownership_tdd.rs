//! Support-region ownership is a DECLARED property, not a geometric one.
//!
//! `union_same_family_entries` used to fold two entries together only when
//! they shared a body id or when their bounding-box centroids landed in the
//! same fixed grid cell. That made merge depend on where geometry happened to
//! sit, so one family's two contributions to a single declared region stayed
//! split whenever they were far apart. The merge key is the declared identity
//! `(family_id, global_layer_index, object_id, region_id, anchor_z)`.

use std::collections::BTreeMap;
use std::sync::Arc;

use slicer_ir::{
    ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3, Polygon, SupportAnalysisIR,
    SupportPlanEntry, SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion,
};
use slicer_wasm_host::{
    exact_z_query::ExactZQueryService,
    support_aggregation::{
        aggregate_support_plan_irs_with_policy_attributed, try_aggregate_support_plans_with_policy,
        FamilyConflictPolicy, OwnershipReason, SupportAggregationInput, SupportPlanProducer,
    },
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

fn entry(body_id: &str, body: ExPolygon) -> SupportPlanEntry {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index: 0,
        object_id: "object".into(),
        region_id: 7,
        family_id: "traditional".into(),
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
        provenance: vec!["traditional".into()],
        decline_reason: None,
    }
}

fn plan(entry: SupportPlanEntry) -> SupportPlanIR {
    SupportPlanIR {
        entries: vec![entry],
        ..SupportPlanIR::default()
    }
}

/// One coplanar triangle: no cross-section at any Z, so exact-Z occupancy is
/// empty and `validate_entry` cannot reject on occupancy grounds.
fn exact_z() -> ExactZQueryService {
    ExactZQueryService::new(Arc::new(MeshIR {
        objects: vec![ObjectMesh {
            id: "object".into(),
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
        }],
        ..MeshIR::default()
    }))
}

#[test]
fn union_merges_same_region_entries_regardless_of_distance() {
    let exact_z = exact_z();
    // 1 unit = 100 nm. The two bodies are 3_000_000 units (300 mm) apart in X,
    // far beyond the old routing-cell edge, but both are small enough to pass
    // `in_routing_cell`'s per-body extent bound.
    let near = entry("body-near", polygon(0, 0, 200_000, 200_000));
    let far = entry("body-far", polygon(3_000_000, 0, 3_200_000, 200_000));
    let analysis = SupportAnalysisIR {
        family_assignments: BTreeMap::from([(
            ("object".to_string(), 7),
            "traditional".to_string(),
        )]),
        ..SupportAnalysisIR::default()
    };

    let result = try_aggregate_support_plans_with_policy(
        SupportAggregationInput {
            plans: vec![plan(near), plan(far)],
            exact_z: &exact_z,
            territory: Some(&analysis),
            producers: vec![
                producer("com.core.traditional-support", &["support-family:traditional"]),
                producer("com.core.traditional-support", &["support-family:traditional"]),
            ],
        },
        FamilyConflictPolicy::Degrade,
    )
    .expect("structured aggregation");

    assert_eq!(
        result.retained.len(),
        1,
        "same declared region must yield one entry, got {:?}",
        result
            .retained
            .iter()
            .map(|e| e.body_ids.clone())
            .collect::<Vec<_>>()
    );
    let body_ids = &result.retained[0].body_ids;
    assert!(
        body_ids.iter().any(|id| id == "body-near") && body_ids.iter().any(|id| id == "body-far"),
        "merged entry must own both source bodies, got {body_ids:?}"
    );
}

// Ownership: a support region belongs to the family the host assigned it to in
// `SupportAnalysisIR::family_assignments`, and only a producer that declared
// the matching `support-family:<id>` claim may write it. Default-deny.

fn entry_in(family_id: &str, region_id: u64, body_id: &str, body: ExPolygon) -> SupportPlanEntry {
    SupportPlanEntry {
        region_id,
        family_id: family_id.into(),
        provenance: vec![family_id.into()],
        ..entry(body_id, body)
    }
}

fn producer(module_id: &str, claims: &[&str]) -> SupportPlanProducer {
    SupportPlanProducer {
        module_id: module_id.into(),
        claims: claims.iter().map(|claim| (*claim).to_string()).collect(),
    }
}

fn analysis(assignments: &[(u64, &str)]) -> SupportAnalysisIR {
    SupportAnalysisIR {
        family_assignments: assignments
            .iter()
            .map(|(region_id, family)| (("object".to_string(), *region_id), (*family).to_string()))
            .collect::<BTreeMap<_, _>>(),
        ..SupportAnalysisIR::default()
    }
}

#[test]
fn unassigned_region_entry_is_a_trespass() {
    let exact_z = exact_z();
    // Region 7 appears in no `family_assignments` row: nobody owns it, so any
    // write to it is a trespass no matter who asks.
    let unowned = analysis(&[(9, "traditional")]);
    let plans = vec![plan(entry_in(
        "traditional",
        7,
        "body",
        polygon(0, 0, 200_000, 200_000),
    ))];
    let producers = vec![producer(
        "com.core.traditional-support",
        &["support-family:traditional"],
    )];

    let degraded = try_aggregate_support_plans_with_policy(
        SupportAggregationInput {
            plans: plans.clone(),
            exact_z: &exact_z,
            territory: Some(&unowned),
            producers: producers.clone(),
        },
        FamilyConflictPolicy::Degrade,
    )
    .expect("degrade policy never errors");

    assert!(
        degraded.degraded,
        "an unowned write must degrade the aggregate"
    );
    assert!(
        degraded.retained.is_empty(),
        "the trespassing entry must not be published, got {:?}",
        degraded.retained
    );
    assert_eq!(
        degraded.ownership_violations.len(),
        1,
        "exactly one ownership violation expected, got {:?}",
        degraded.ownership_violations
    );
    let violation = &degraded.ownership_violations[0];
    assert_eq!(violation.reason, OwnershipReason::NoAssignment);
    assert_eq!(violation.region_id, 7);
    assert_eq!(violation.family_id, "traditional");

    let (_plan, diagnostics) = aggregate_support_plan_irs_with_policy_attributed(
        plans.clone(),
        producers.clone(),
        &exact_z,
        Some(&unowned),
        FamilyConflictPolicy::Degrade,
    )
    .expect("degrade policy never errors");
    let ownership = diagnostics
        .iter()
        .filter(|attributed| attributed.diagnostic.code == 1206)
        .collect::<Vec<_>>();
    assert_eq!(
        ownership.len(),
        1,
        "one code-1206 diagnostic expected, got {:?}",
        diagnostics
            .iter()
            .map(|a| (a.diagnostic.code, a.diagnostic.message.clone()))
            .collect::<Vec<_>>()
    );
    let message = &ownership[0].diagnostic.message;
    assert!(
        message.contains("traditional") && message.contains('7'),
        "the diagnostic must name the family and the region, got {message:?}"
    );

    let fatal = try_aggregate_support_plans_with_policy(
        SupportAggregationInput {
            plans,
            exact_z: &exact_z,
            territory: Some(&unowned),
            producers,
        },
        FamilyConflictPolicy::Fail,
    )
    .expect_err("fail policy must refuse to publish an unowned write");
    assert_eq!(fatal.reason, OwnershipReason::NoAssignment);
    assert_eq!(fatal.region_id, 7);
    assert_eq!(fatal.family_id, "traditional");
}

#[test]
fn producer_without_family_claim_is_a_trespass() {
    let exact_z = exact_z();
    // Region 9 IS assigned to `tree`, so the entry's declared family is right;
    // the module writing it simply never claimed that family.
    let assigned = analysis(&[(7, "traditional"), (9, "tree")]);
    let plans = vec![
        plan(entry_in(
            "traditional",
            7,
            "traditional-body",
            polygon(0, 0, 200_000, 200_000),
        )),
        plan(entry_in(
            "tree",
            9,
            "tree-body",
            polygon(400_000, 0, 600_000, 200_000),
        )),
    ];
    let producers = vec![
        producer(
            "com.core.traditional-support",
            &["support-family:traditional"],
        ),
        producer("com.core.tree-support", &["support-generator"]),
    ];

    let (_plan, diagnostics) = aggregate_support_plan_irs_with_policy_attributed(
        plans,
        producers,
        &exact_z,
        Some(&assigned),
        FamilyConflictPolicy::Degrade,
    )
    .expect("degrade policy never errors");

    let ownership = diagnostics
        .iter()
        .filter(|attributed| attributed.diagnostic.code == 1206)
        .collect::<Vec<_>>();
    assert_eq!(
        ownership.len(),
        1,
        "one code-1206 diagnostic expected, got {:?}",
        diagnostics
            .iter()
            .map(|a| (a.diagnostic.code, a.diagnostic.message.clone()))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        ownership[0].plan_index,
        Some(1),
        "the diagnostic must name the producing plan directly, not a family guess"
    );
    let message = &ownership[0].diagnostic.message;
    assert!(
        message.contains("tree") && message.contains('9'),
        "the diagnostic must name the family and the region, got {message:?}"
    );
}

#[test]
fn wrong_family_entry_is_a_trespass_in_both_plan_orders() {
    let exact_z = exact_z();
    // Region 7 belongs to `traditional`. `tree` writing it is a trespass
    // whichever plan arrives first: ownership is declared, never raced for.
    let assigned = analysis(&[(7, "traditional")]);
    let traditional = plan(entry_in(
        "traditional",
        7,
        "traditional-body",
        polygon(0, 0, 200_000, 200_000),
    ));
    let tree = plan(entry_in(
        "tree",
        7,
        "tree-body",
        polygon(0, 0, 200_000, 200_000),
    ));
    let traditional_producer = producer(
        "com.core.traditional-support",
        &["support-family:traditional"],
    );
    let tree_producer = producer("com.core.tree-support", &["support-family:tree"]);

    for (label, plans, producers) in [
        (
            "traditional first",
            vec![traditional.clone(), tree.clone()],
            vec![traditional_producer.clone(), tree_producer.clone()],
        ),
        (
            "tree first",
            vec![tree.clone(), traditional.clone()],
            vec![tree_producer.clone(), traditional_producer.clone()],
        ),
    ] {
        let result = try_aggregate_support_plans_with_policy(
            SupportAggregationInput {
                plans,
                exact_z: &exact_z,
                territory: Some(&assigned),
                producers,
            },
            FamilyConflictPolicy::Degrade,
        )
        .expect("degrade policy never errors");

        assert_eq!(
            result.retained.len(),
            1,
            "[{label}] only the owning family may be published, got {:?}",
            result
                .retained
                .iter()
                .map(|e| e.family_id.clone())
                .collect::<Vec<_>>()
        );
        assert_eq!(
            result.retained[0].family_id, "traditional",
            "[{label}] the assigned owner must be the survivor"
        );
        assert_eq!(
            result.ownership_violations.len(),
            1,
            "[{label}] exactly one trespass expected, got {:?}",
            result.ownership_violations
        );
        let violation = &result.ownership_violations[0];
        assert_eq!(
            violation.family_id, "tree",
            "[{label}] the trespasser must be named"
        );
        assert_eq!(
            violation.reason,
            OwnershipReason::WrongFamily {
                owner: "traditional".to_string()
            },
            "[{label}] wrong-family trespass"
        );
    }
}
