//! Behavioral contracts for host-owned mixed support-family routing.

use std::sync::Arc;

use slicer_ir::{
    ExPolygon, ExtrusionPath3D, ExtrusionRole, LayerStageCommit, Point2, Point3WithWidth, Polygon,
    SupportEntry, SupportIR, SupportPlanDeclineReason, SupportPlanEntry, SupportPlanIR,
    SupportPlanRole, SupportPlanRoleRegion, SupportRole,
};
use slicer_runtime::{
    layer_executor::{apply_for_test, StageApplyContext},
    LayerArena,
};
use slicer_wasm_host::{
    exact_z_query::ExactZQueryService,
    support_aggregation::{
        aggregate_support_plan_irs_with_diagnostics,
        try_aggregate_support_plan_irs_with_diagnostics, try_aggregate_support_plans,
        SupportAggregationInput,
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

fn entry(
    family_id: &str,
    body_id: &str,
    demand_id: &str,
    object_id: &str,
    region_id: u64,
    body: Option<ExPolygon>,
) -> SupportPlanEntry {
    SupportPlanEntry {
        global_layer_index: 0,
        object_id: object_id.into(),
        region_id,
        family_id: family_id.into(),
        demand_ids: vec![demand_id.into()],
        body_ids: vec![body_id.into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        roles: body
            .into_iter()
            .map(|region| SupportPlanRoleRegion {
                role: SupportPlanRole::SupportBody,
                regions: vec![region],
            })
            .collect(),
        skeleton: None,
        capabilities: Vec::new(),
        provenance: vec![family_id.into()],
        decline_reason: None,
    }
}

fn plan(entries: Vec<SupportPlanEntry>) -> SupportPlanIR {
    SupportPlanIR {
        entries,
        ..SupportPlanIR::default()
    }
}

fn exact_z() -> ExactZQueryService {
    ExactZQueryService::new(Arc::new(slicer_ir::MeshIR {
        objects: vec![slicer_ir::ObjectMesh {
            id: "object".into(),
            mesh: slicer_ir::IndexedTriangleSet {
                vertices: vec![
                    slicer_ir::Point3 {
                        x: 0.0,
                        y: 0.0,
                        z: 100.0,
                    },
                    slicer_ir::Point3 {
                        x: 1.0,
                        y: 0.0,
                        z: 100.0,
                    },
                    slicer_ir::Point3 {
                        x: 0.0,
                        y: 1.0,
                        z: 100.0,
                    },
                ],
                indices: vec![0, 1, 2],
            },
            ..slicer_ir::ObjectMesh::default()
        }],
        ..slicer_ir::MeshIR::default()
    }))
}

fn aggregate(plans: Vec<SupportPlanIR>) -> (SupportPlanIR, Vec<slicer_ir::Diagnostic>) {
    let exact_z = exact_z();
    aggregate_support_plan_irs_with_diagnostics(plans, &exact_z)
}

fn support_entry(
    family: &str,
    body: &str,
    demand: &str,
    x0: f32,
    y0: f32,
    x1: f32,
    y1: f32,
) -> SupportEntry {
    SupportEntry {
        family_id: family.into(),
        body_id: body.into(),
        demand_ids: vec![demand.into()],
        object_id: "object".into(),
        region_id: 0,
        role: SupportRole::SupportBody,
        paths: vec![ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: x0,
                    y: y0,
                    z: 0.2,
                    width: 1.0,
                    ..Default::default()
                },
                Point3WithWidth {
                    x: x1,
                    y: y1,
                    z: 0.2,
                    width: 1.0,
                    ..Default::default()
                },
            ],
            role: ExtrusionRole::SupportMaterial,
            speed_factor: 1.0,
        }],
    }
}

#[test]
fn routing_cells() {
    let plans = vec![
        plan(vec![entry(
            "tree",
            "tree-body",
            "tree-demand",
            "object",
            0,
            Some(polygon(100, 100, 200, 200)),
        )]),
        plan(vec![entry(
            "traditional",
            "normal-body",
            "normal-demand",
            "object",
            1,
            Some(polygon(1_100_000, 100, 1_100_200, 200)),
        )]),
    ];
    let (forward, diagnostics) = aggregate(plans.clone());
    let (reverse, reverse_diagnostics) = aggregate(plans.into_iter().rev().collect());
    assert!(diagnostics.is_empty());
    assert!(reverse_diagnostics.is_empty());
    let mut forward_entries = forward.entries.clone();
    let mut reverse_entries = reverse.entries.clone();
    forward_entries.sort_by_key(|entry| entry.region_id);
    reverse_entries.sort_by_key(|entry| entry.region_id);
    assert_eq!(
        forward_entries, reverse_entries,
        "routing output must be input-order independent"
    );
    assert_eq!(forward.entries.len(), 2);
    assert!(forward
        .entries
        .iter()
        .any(|e| e.family_id == "tree" && e.demand_ids == ["tree-demand"]));
    assert!(forward
        .entries
        .iter()
        .any(|e| e.family_id == "traditional" && e.demand_ids == ["normal-demand"]));

    let collision = vec![
        plan(vec![entry(
            "traditional",
            "normal-body",
            "normal-demand",
            "object",
            1,
            Some(polygon(200, 100, 400, 300)),
        )]),
        plan(vec![entry(
            "tree",
            "tree-body",
            "tree-demand",
            "object",
            2,
            Some(polygon(100, 100, 300, 300)),
        )]),
    ];
    let (collision_forward, diagnostics_forward) = aggregate(collision.clone());
    let (collision_reverse, diagnostics_reverse) = aggregate(collision.into_iter().rev().collect());
    assert_eq!(collision_forward.entries, collision_reverse.entries);
    assert_eq!(diagnostics_forward, diagnostics_reverse);
    assert!(collision_forward.entries.is_empty());
    assert!(diagnostics_forward
        .iter()
        .all(|d| d.code == 1200 || d.code == 1203));
}

#[test]
fn family_attribution() {
    let (output, diagnostics) = aggregate(vec![
        plan(vec![entry(
            "tree",
            "tree-body",
            "tree-demand",
            "object",
            0,
            Some(polygon(100, 100, 200, 200)),
        )]),
        plan(vec![entry(
            "traditional",
            "normal-body",
            "normal-demand",
            "object",
            1,
            Some(polygon(300, 100, 400, 200)),
        )]),
    ]);
    assert!(diagnostics.is_empty());
    assert_eq!(output.entries.len(), 2);
    for retained in &output.entries {
        assert_eq!(retained.object_id, "object");
        assert_eq!(retained.roles[0].role, SupportPlanRole::SupportBody);
        assert_eq!(retained.demand_ids.len(), 1);
        assert_eq!(retained.body_ids.len(), 1);
    }
    assert!(output
        .entries
        .iter()
        .any(|e| e.family_id == "tree" && e.region_id == 0));
    assert!(output
        .entries
        .iter()
        .any(|e| e.family_id == "traditional" && e.region_id == 1));
}

#[test]
fn same_family_union() {
    let (output, diagnostics) = aggregate(vec![plan(vec![
        entry(
            "tree",
            "shared-body",
            "demand-a",
            "object",
            0,
            Some(polygon(100, 100, 200, 200)),
        ),
        entry(
            "tree",
            "shared-body",
            "demand-b",
            "object",
            1,
            Some(polygon(300, 100, 400, 200)),
        ),
    ])]);
    assert!(diagnostics.is_empty());

    let demands: Vec<_> = output
        .entries
        .iter()
        .flat_map(|e| e.demand_ids.iter())
        .collect();
    assert_eq!(demands, [&"demand-a".to_string(), &"demand-b".to_string()]);
    assert_eq!(
        demands
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len(),
        2
    );

    let (output, diagnostics) = aggregate(vec![plan(vec![
        entry(
            "tree",
            "cross-cell-body",
            "demand-a",
            "object",
            0,
            Some(polygon(100, 100, 200, 200)),
        ),
        entry(
            "tree",
            "cross-cell-body",
            "demand-b",
            "object",
            1,
            Some(polygon(1_100_000, 100, 1_100_200, 200)),
        ),
    ])]);
    assert!(output.entries.is_empty());
    assert_eq!(diagnostics.iter().filter(|d| d.code == 1200).count(), 2);

    let (output, diagnostics) = aggregate(vec![
        plan(vec![entry(
            "tree",
            "tree-body",
            "tree-demand",
            "object",
            0,
            Some(polygon(100, 100, 200, 200)),
        )]),
        plan(vec![entry(
            "traditional",
            "normal-body",
            "normal-demand",
            "object",
            1,
            Some(polygon(1_100_000, 100, 1_200_000, 200)),
        )]),
    ]);
    assert_eq!(output.entries.len(), 2);
    assert!(diagnostics.is_empty());

    let exact_z = exact_z();
    let structured = try_aggregate_support_plans(SupportAggregationInput {
        plans: vec![
            plan(vec![entry(
                "tree",
                "tree-body",
                "tree-demand",
                "object",
                0,
                Some(polygon(100, 100, 1_100_000, 200)),
            )]),
            plan(vec![entry(
                "traditional",
                "normal-body",
                "normal-demand",
                "object",
                1,
                Some(polygon(1_100_000, 100, 1_100_200, 200)),
            )]),
        ],
        exact_z: &exact_z,
    })
    .expect("structured aggregation");
    assert_eq!(structured.retained.len(), 1);
    assert_eq!(structured.retained[0].family_id, "traditional");
    assert!(structured.retained.iter().all(|e| e.family_id != "tree"));
    let cell_crossing: Vec<_> = structured
        .unmet
        .iter()
        .filter(|d| d.reason == "body rejected: routing-cell collision")
        .collect();
    assert_eq!(cell_crossing.len(), 1);
    assert_eq!(cell_crossing[0].demand_id, "tree-demand");
    assert_eq!(cell_crossing[0].body_id, "tree-body");
    let routing: Vec<_> = structured
        .diagnostics
        .iter()
        .filter(|d| d.reason == "body rejected: routing-cell collision")
        .collect();
    assert_eq!(routing.len(), 1);
    assert_eq!(routing[0].family_id, "tree");
    assert_eq!(routing[0].body_id, "tree-body");
    assert_eq!(routing[0].demand_id, "tree-demand");
}

#[test]
fn cross_family_body_overlap() {
    let (overlap, diagnostics) = aggregate(vec![
        plan(vec![entry(
            "tree",
            "tree-body",
            "tree-demand",
            "object",
            0,
            Some(polygon(100, 100, 300, 300)),
        )]),
        plan(vec![entry(
            "traditional",
            "normal-body",
            "normal-demand",
            "object",
            1,
            Some(polygon(200, 200, 400, 400)),
        )]),
    ]);
    assert!(overlap.entries.is_empty());
    assert!(diagnostics.iter().any(|d| d.code == 1200));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == 1200 && d.message.contains("tree-body")));

    let exact_z = exact_z();
    let structured = try_aggregate_support_plans(SupportAggregationInput {
        plans: vec![
            plan(vec![entry(
                "tree",
                "tree-body",
                "tree-demand",
                "object",
                0,
                Some(polygon(100, 100, 300, 300)),
            )]),
            plan(vec![entry(
                "traditional",
                "normal-body",
                "normal-demand",
                "object",
                1,
                Some(polygon(200, 200, 400, 400)),
            )]),
        ],
        exact_z: &exact_z,
    })
    .expect("structured aggregation");
    let cross_family: Vec<_> = structured
        .diagnostics
        .iter()
        .filter(|d| d.reason == "body rejected: cross-family positive-area overlap")
        .collect();
    assert_eq!(cross_family.len(), 2);
    assert!(cross_family.iter().any(|d| d.family_id == "tree"
        && d.body_id == "tree-body"
        && d.demand_id == "tree-demand"));
    assert!(cross_family.iter().any(|d| d.family_id == "traditional"
        && d.body_id == "normal-body"
        && d.demand_id == "normal-demand"));

    let (touching, diagnostics) = aggregate(vec![
        plan(vec![entry(
            "tree",
            "tree-body",
            "tree-demand",
            "object",
            0,
            Some(polygon(100, 100, 300, 300)),
        )]),
        plan(vec![entry(
            "traditional",
            "normal-body",
            "normal-demand",
            "object",
            1,
            Some(polygon(300, 100, 500, 300)),
        )]),
    ]);
    assert_eq!(touching.entries.len(), 2);
    assert!(diagnostics.is_empty());
}

#[test]
fn swept_path_overlap() {
    let mut arena = LayerArena::new();
    apply_for_test(
        &mut arena,
        LayerStageCommit::Support(SupportIR {
            entries: vec![support_entry(
                "tree",
                "tree-body",
                "tree-demand",
                0.0,
                0.0,
                10.0,
                10.0,
            )],
            ..Default::default()
        }),
        &StageApplyContext {
            stage_id: "Layer::Support",
            module_id: "tree",
            layer_index: 0,
            seam_plan: None,
        },
    )
    .unwrap();
    apply_for_test(
        &mut arena,
        LayerStageCommit::Support(SupportIR {
            entries: vec![support_entry(
                "traditional",
                "normal-body",
                "normal-demand",
                0.0,
                10.0,
                10.0,
                0.0,
            )],
            ..Default::default()
        }),
        &StageApplyContext {
            stage_id: "Layer::Support",
            module_id: "traditional",
            layer_index: 0,
            seam_plan: None,
        },
    )
    .unwrap();
    assert!(arena
        .support()
        .expect("support arena slot")
        .entries
        .is_empty());
    let diagnostics = arena.support_routing_diagnostics();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics
        .iter()
        .all(|d| d.reason == "cross-family swept-path overlap"));
    assert!(diagnostics.iter().any(|d| d.family_id == "tree"
        && d.body_id == "tree-body"
        && d.demand_id == "tree-demand"));
    assert!(diagnostics.iter().any(|d| d.family_id == "traditional"
        && d.body_id == "normal-body"
        && d.demand_id == "normal-demand"));
}

#[test]
fn degraded_diagnostics() {
    for reason in [
        SupportPlanDeclineReason::DeclinedPolicy,
        SupportPlanDeclineReason::NoRoute,
        SupportPlanDeclineReason::Blocked,
        SupportPlanDeclineReason::UnsupportedMode,
    ] {
        let mut declined = entry(
            "tree",
            "declined-body",
            "declined-demand",
            "object",
            0,
            None,
        );
        declined.decline_reason = Some(reason);
        let (output, diagnostics) = aggregate(vec![plan(vec![declined])]);
        assert!(output.entries.is_empty());
        assert!(diagnostics
            .iter()
            .any(|d| d.code == 1201 && d.message.contains(&format!("{:?}", reason))));
    }
}

#[test]
fn mismatched_family_fatal() {
    let exact_z = exact_z();
    let result = try_aggregate_support_plan_irs_with_diagnostics(
        vec![
            plan(vec![entry(
                "tree",
                "tree-body",
                "tree-demand",
                "object",
                0,
                Some(polygon(100, 100, 200, 200)),
            )]),
            plan(vec![entry(
                "traditional",
                "normal-body",
                "normal-demand",
                "object",
                0,
                Some(polygon(300, 100, 400, 200)),
            )]),
        ],
        &exact_z,
    );
    let error = result.expect_err("two families cannot claim one source region");
    assert_eq!(error.global_layer_index, 0);
    assert_eq!(error.object_id, "object");
    assert_eq!(error.region_id, 0);
    assert_eq!(error.expected_family_id, "traditional");
    assert_eq!(error.conflicting_family_id, "tree");
}

#[test]
fn invalid_body_degraded() {
    let (output, diagnostics) = aggregate(vec![plan(vec![entry(
        "tree",
        "invalid-body",
        "unmet-demand",
        "object",
        0,
        Some(polygon(-600_000, 0, 600_000, 10)),
    )])]);
    assert!(output.entries.is_empty());
    assert!(diagnostics
        .iter()
        .any(|d| d.code == 1200 && d.message.contains("unmet-demand")));
    assert!(diagnostics
        .iter()
        .any(|d| d.code == 1203 && d.message.contains("invalid-body")));

    let mut occupied = entry(
        "tree",
        "occupied-body",
        "occupied-demand",
        "object",
        1,
        Some(polygon(-1, -1, 2, 2)),
    );
    occupied.anchor_z = 1_000_000;
    let (output, diagnostics) = aggregate(vec![plan(vec![occupied])]);
    assert!(output.entries.is_empty());
    assert!(diagnostics
        .iter()
        .any(|d| d.code == 1200 && d.message.contains("occupied-demand")));
}
