#![allow(missing_docs)]

use slicer_ir::{AnchoredEntity, AnchoredEntityProvenance, AnchoredGeometryContract};
use slicer_scheduler::execution_plan::ExecutionPlan;

pub fn capability_derived_anchor_closure() {
    let plan = ExecutionPlan::default();
    // exhaustive: no Default impl for AnchoredEntity; anchored-contract fixture pins every field
    let path_optimization = AnchoredEntity {
        local_id: 1,
        anchor_global_layer_index: 7,
        geometry: AnchoredGeometryContract::Planar { z: 2 },
        input_capabilities: vec![String::from("Layer::PathOptimization")],
        output_capabilities: vec![String::from("ordered-events")],
        provenance: AnchoredEntityProvenance {
            requesting_feature: String::from("test-feature"),
            source_plan_entry: String::from("test-plan-entry"),
        },
        path_points: Vec::new(),
    };
    let unrelated = AnchoredEntity {
        input_capabilities: vec![String::from("ordered-events")],
        ..path_optimization.clone()
    };

    let invocation = plan.anchored_invocation(&path_optimization, true);
    assert_eq!(invocation.anchor_global_layer_index, 7);
    assert!(invocation
        .closure
        .derived_capabilities
        .contains(&String::from("Layer::PathOptimization")));
    assert_eq!(invocation.provenance, path_optimization.provenance);
    assert!(invocation.layer_parallel_safe);

    let unrelated_invocation = plan.anchored_invocation(&unrelated, false);
    assert!(!unrelated_invocation
        .closure
        .derived_capabilities
        .contains(&String::from("Layer::PathOptimization")));
    assert!(!unrelated_invocation.layer_parallel_safe);
}
