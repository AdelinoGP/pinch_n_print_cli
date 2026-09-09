/// Declined support candidates are structured and never filled by fallback.
use std::sync::Arc;

use slicer_ir::{
    MeshIR, SupportAnalysisIR, SupportPlanDeclineReason as DeclineReason,
    SupportPlanEntry as DeclineEntry, SupportPlanIR as DeclinePlan,
};
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::{
    aggregate_support_plan_irs_degrading_with_attributed_diagnostics,
    aggregate_support_plan_irs_with_policy_attributed, FamilyConflictPolicy, SupportPlanProducer,
};

/// Ownership at the merge point is default-deny, and a DECLINE is only reported
/// for an entry that owns the region it names -- a trespassing entry is refused
/// before its decline is ever read. So a decline fixture must grant each
/// entry's `(object_id, region_id)` to that entry's own family.
fn family_assignments_for(plans: &[DeclinePlan]) -> SupportAnalysisIR {
    SupportAnalysisIR {
        family_assignments: plans
            .iter()
            .flat_map(|plan| plan.entries.iter())
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

/// One producer per plan, holding the `support-family:<id>` claim for every
/// family that plan writes. Index-parallel to `plans` by construction.
fn producers_for(plans: &[DeclinePlan]) -> Vec<SupportPlanProducer> {
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

#[test]
pub fn support_decline_contract() {
    let decline_reason = DeclineReason::NoRoute;
    assert!(matches!(decline_reason, DeclineReason::NoRoute));
    let plan = DeclinePlan {
        entries: vec![DeclineEntry {
            global_layer_index: 0,
            object_id: "object-a".into(),
            region_id: 7,
            family_id: "tree".into(),
            demand_ids: vec!["demand-1".into()],
            body_ids: vec![],
            anchor_layer_index: 0,
            anchor_z: 0,
            roles: vec![],
            skeleton: None,
            capabilities: vec![],
            provenance: vec![],
            decline_reason: Some(decline_reason),
        }],
        ..Default::default()
    };
    let exact_z = ExactZQueryService::new(Arc::new(MeshIR::default()));
    let plans = vec![plan];
    let owned = family_assignments_for(&plans);
    let (result, attributed) = aggregate_support_plan_irs_with_policy_attributed(
        plans.clone(),
        producers_for(&plans),
        &exact_z,
        Some(&owned),
        FamilyConflictPolicy::Fail,
    )
    .expect("owned decline must not be a routing failure");
    let diagnostics = attributed
        .into_iter()
        .map(|entry| entry.diagnostic)
        .collect::<Vec<_>>();

    assert!(result.entries.is_empty());
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, 1201);
    assert_eq!(
        diagnostics[0].message,
        "support demand 'demand-1' declined by family 'tree': NoRoute"
    );
}

/// A decline must be attributed to the family that produced it, never to
/// whichever writer happened to emit the stage output last.
///
/// Regression: aggregation returned a flat `Vec<Diagnostic>` and the prepass
/// attached the whole vector to the LAST support-plan writer's audit.
/// `emit_host_support_diagnostics` (slicer-runtime `run.rs`) then names that
/// audit's `module_id`, so a `traditional` family `NoRoute` decline was
/// reported against `com.core.tree-support-planner` -- a module whose sources
/// contain no `NoRoute` at all.
#[test]
pub fn decline_is_attributed_to_producing_family_not_last_writer() {
    fn declining_plan(family_id: &str, demand_id: &str) -> DeclinePlan {
        DeclinePlan {
            entries: vec![DeclineEntry {
                global_layer_index: 0,
                object_id: "object-a".into(),
                region_id: 7,
                family_id: family_id.into(),
                demand_ids: vec![demand_id.into()],
                body_ids: vec![],
                anchor_layer_index: 0,
                anchor_z: 0,
                roles: vec![],
                skeleton: None,
                capabilities: vec![],
                provenance: vec![],
                decline_reason: Some(DeclineReason::NoRoute),
            }],
            ..Default::default()
        }
    }

    // Plan 0 declines; plan 1 (a different family) is the LAST writer and
    // declines nothing. The old code blamed plan 1.
    let plans = vec![
        declining_plan("traditional", "demand-1"),
        DeclinePlan {
            entries: vec![DeclineEntry {
                global_layer_index: 0,
                object_id: "object-b".into(),
                region_id: 9,
                family_id: "tree".into(),
                demand_ids: vec!["demand-2".into()],
                body_ids: vec!["tree-body".into()],
                anchor_layer_index: 0,
                anchor_z: 0,
                roles: vec![],
                skeleton: None,
                capabilities: vec![],
                provenance: vec![],
                decline_reason: None,
            }],
            ..Default::default()
        },
    ];

    let exact_z = ExactZQueryService::new(Arc::new(MeshIR::default()));
    let owned = family_assignments_for(&plans);
    let producers = producers_for(&plans);
    let (_plan, diagnostics) = aggregate_support_plan_irs_degrading_with_attributed_diagnostics(
        plans,
        producers,
        &exact_z,
        Some(&owned),
    );

    let declines: Vec<_> = diagnostics
        .iter()
        .filter(|entry| entry.diagnostic.code == 1201)
        .collect();
    assert_eq!(declines.len(), 1, "exactly one decline expected");
    assert_eq!(
        declines[0].plan_index,
        Some(0),
        "decline must point at the plan that produced it (index 0, family 'traditional'),          not the last writer"
    );
    assert!(
        declines[0]
            .diagnostic
            .message
            .contains("family 'traditional'"),
        "message must name the producing family, got: {}",
        declines[0].diagnostic.message
    );
    assert!(
        !declines[0].diagnostic.message.contains("tree"),
        "message must not name the unrelated last writer, got: {}",
        declines[0].diagnostic.message
    );
}
