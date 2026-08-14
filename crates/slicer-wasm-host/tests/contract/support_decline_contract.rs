/// Declined support candidates are structured and never filled by fallback.
use std::sync::Arc;

use slicer_ir::{
    MeshIR, SupportPlanDeclineReason as DeclineReason, SupportPlanEntry as DeclineEntry,
    SupportPlanIR as DeclinePlan,
};
use slicer_wasm_host::exact_z_query::ExactZQueryService;
use slicer_wasm_host::support_aggregation::aggregate_support_plan_irs_with_diagnostics;

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
    let (result, diagnostics) = aggregate_support_plan_irs_with_diagnostics(vec![plan], &exact_z);

    assert!(result.entries.is_empty());
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0].code, 1201);
    assert_eq!(
        diagnostics[0].message,
        "support demand 'demand-1' declined: NoRoute"
    );
}
