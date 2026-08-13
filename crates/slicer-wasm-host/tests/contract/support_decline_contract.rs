/// Declined support candidates are structured and never filled by fallback.
use slicer_ir::{
    SupportPlanDeclineReason as DeclineReason, SupportPlanEntry as DeclineEntry,
    SupportPlanIR as DeclinePlan,
};
use slicer_wasm_host::support_aggregation::aggregate_declined_support_plans;

#[test]
pub fn support_decline_contract() {
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
            decline_reason: Some(DeclineReason::NoRoute),
        }],
        ..Default::default()
    };
    let result = aggregate_declined_support_plans(&[plan]);
    assert_eq!(result.declined.len(), 1);
    assert!(matches!(result.declined[0].reason, DeclineReason::NoRoute));
    assert!(result.support_paths.is_empty());
}
