/// Host-owned support plan aggregation and complete-body validation contract.
use std::sync::Arc;

use slicer_ir::{ExPolygon, IndexedTriangleSet, MeshIR, ObjectMesh, Point2, Point3, Transform3d};
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::{aggregate_support_plans, SupportAggregationInput};

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
    // Genuinely oversized: one unit wider than ROUTING_CELL_SIZE (1 << 20), so
    // it fits in no cell-sized territory. (Before packet 224 this fixture was a
    // 1_000-unit body parked across the x = 1 << 20 grid line, which pinned the
    // absolute-grid defect rather than the size contract.)
    let mut spans_cell = entry("spans_cell", 30_000_000);
    spans_cell.region_id = 9;
    spans_cell.roles[0].regions = vec![square(30_000_000, 5_000, (1 << 20) + 1)];
    let result = aggregate_support_plans(SupportAggregationInput {
        plans: vec![slicer_ir::SupportPlanIR {
            entries: vec![entry("valid", 20_000_000), colliding, spans_cell],
            ..Default::default()
        }],
        exact_z: &service,
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
        .any(|d| d.demand_id == "spans_cell" && d.reason.contains("routing-cell")));
}

#[test]
fn support_plan_aggregation_preserves_distinct_families() {
    let service = ExactZQueryService::new(Arc::new(mesh()));
    let mut tree = entry("tree-body", 20_000_000);
    tree.family_id = "tree".into();
    let mut traditional = entry("traditional-body", 30_000_000);
    traditional.family_id = "traditional".into();
    traditional.region_id = 8;

    let result = aggregate_support_plans(SupportAggregationInput {
        plans: vec![
            slicer_ir::SupportPlanIR {
                entries: vec![tree],
                ..Default::default()
            },
            slicer_ir::SupportPlanIR {
                entries: vec![traditional],
                ..Default::default()
            },
        ],
        exact_z: &service,
    });

    assert_eq!(result.retained.len(), 2);
    assert_eq!(result.retained[0].family_id, "tree");
    assert_eq!(result.retained[1].family_id, "traditional");
    assert!(result.duplicates.is_empty());
}

#[test]
fn support_plan_aggregation_diagnoses_duplicate_identity() {
    let service = ExactZQueryService::new(Arc::new(mesh()));
    let mut first = entry("tree-body", 20_000_000);
    first.family_id = "tree".into();
    let mut duplicate = entry("traditional-body", 30_000_000);
    duplicate.family_id = "traditional".into();

    let result = aggregate_support_plans(SupportAggregationInput {
        plans: vec![
            slicer_ir::SupportPlanIR {
                entries: vec![first],
                ..Default::default()
            },
            slicer_ir::SupportPlanIR {
                entries: vec![duplicate],
                ..Default::default()
            },
        ],
        exact_z: &service,
    });

    assert_eq!(result.retained.len(), 1);
    assert_eq!(result.retained[0].family_id, "tree");
    assert!(result.degraded);
    assert_eq!(result.duplicates.len(), 1);
    assert_eq!(result.duplicates[0].first_family_id, "tree");
    assert_eq!(result.duplicates[0].duplicate_family_id, "traditional");
}

/// Regression (packet 224): routing cells partition space to bound how much
/// territory one body may claim, not to forbid particular world coordinates.
/// A small, perfectly printable body whose bbox straddles an absolute cell
/// boundary (here y = 0, a multiple of `ROUTING_CELL_SIZE`) must be retained.
/// Measured defect: tree contact tips at the model edge extend to y = -0.4 mm,
/// which rejected every straddling layer as "routing-cell collision".
#[test]
fn support_body_straddling_absolute_cell_boundary_is_retained() {
    let service = ExactZQueryService::new(Arc::new(mesh()));
    // 4 mm square well clear of the 0..10 mm mesh footprint in x, spanning
    // y = -0.4 mm .. +3.6 mm so it crosses the y = 0 cell boundary.
    let mut straddling = entry("straddles_boundary", 200_000);
    straddling.region_id = 11;
    straddling.roles[0].regions = vec![square(200_000, -4_000, 40_000)];

    let result = aggregate_support_plans(SupportAggregationInput {
        plans: vec![slicer_ir::SupportPlanIR {
            entries: vec![straddling],
            ..Default::default()
        }],
        exact_z: &service,
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

    let result = aggregate_support_plans(SupportAggregationInput {
        plans: vec![slicer_ir::SupportPlanIR {
            entries: vec![first, second],
            ..Default::default()
        }],
        exact_z: &service,
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
