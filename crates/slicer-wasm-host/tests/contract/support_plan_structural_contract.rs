// Structural SupportPlanIR contract: plans carry analysis geometry and metadata,
// never printable nozzle-width paths.

use slicer_wasm_host::host::prepass_support_geometry::SupportPlanEntry;
use slicer_wasm_host::host::prepass_support_geometry::slicer::prepass_support_geometry::support_geometry_types::{
    SupportPlanRole, SupportPlanRoleRegion, SupportPlanSkeleton,
};
use slicer_wasm_host::host::{prepass_support_geometry, HostExecutionContextBuilder};

fn structural_entry() -> SupportPlanEntry {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    SupportPlanEntry {
        global_layer_index: 3,
        object_id: "object-a".into(),
        region_id: "7".into(),
        family_id: "tree".into(),
        demand_ids: vec!["demand-1".into()],
        body_ids: vec!["body-1".into()],
        anchor_layer_index: 3,
        anchor_z: 42_000,
        roles: vec![
            SupportPlanRoleRegion {
                role: SupportPlanRole::SupportBody,
                regions: vec![],
            },
            SupportPlanRoleRegion {
                role: SupportPlanRole::TopInterface,
                regions: vec![],
            },
            SupportPlanRoleRegion {
                role: SupportPlanRole::BottomInterface,
                regions: vec![],
            },
        ],
        skeleton: Some(SupportPlanSkeleton {
            points: vec![],
            wall_counts: vec![],
        }),
        capabilities: vec!["anchored-entity".into()],
        provenance: vec!["support-analysis".into()],
        decline_reason: None,
    }
}

#[test]
pub fn support_plan_structural_contract() {
    let mut ctx = HostExecutionContextBuilder::new("test.prepass", 0.2, 0.2).build();
    let handle = ctx
        .push_support_geometry_output()
        .expect("support output handle");
    let entry = structural_entry();
    prepass_support_geometry::HostSupportGeometryOutput::push_support_plan_entry(
        &mut ctx,
        handle,
        entry.clone(),
    )
    .expect("wasmtime call")
    .expect("valid structural entry");

    let entries = ctx.support_plan_entries();
    assert_eq!(entries.len(), 1);
    let harvested = &entries[0];
    assert_eq!(harvested.family_id, entry.family_id);
    assert_eq!(harvested.demand_ids, entry.demand_ids);
    assert_eq!(harvested.body_ids, entry.body_ids);
    assert_eq!(harvested.anchor_layer_index, entry.anchor_layer_index);
    assert_eq!(harvested.anchor_z, entry.anchor_z);
    assert_eq!(harvested.roles.len(), 3);
    assert!(harvested.skeleton.is_some());
    assert_eq!(harvested.capabilities, entry.capabilities);
    assert_eq!(harvested.provenance, entry.provenance);
    assert!(harvested.decline_reason.is_none());
    // The structural WIT record has no ExtrusionPath3D/nozzle-width field.
}
