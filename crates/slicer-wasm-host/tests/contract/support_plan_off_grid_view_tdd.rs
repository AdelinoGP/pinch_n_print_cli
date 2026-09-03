//! Regression: grid and off-grid support planes sharing an anchor layer coexist.

#![allow(missing_docs)]

use std::sync::Arc;

use slicer_ir::{
    AnchoredEntity, AnchoredEntityProvenance, AnchoredEventRuntimeHooks, AnchoredGeometryContract,
    ExtrusionRole as IrExtrusionRole, OrderedEventCollection, Point3WithWidth as IrPoint3WithWidth,
    SupportPlanEntry, SupportPlanIR,
};
use slicer_sdk::PaintRegionLayerView;
use slicer_wasm_host::dispatch::{build_support_plan_layer_data_for_test, deconstruct_layer_ctx};
use slicer_wasm_host::host::{
    ExtrusionPath3d, ExtrusionRole, HostExecutionContextBuilder, Point3WithWidth,
};

fn entry(global_layer_index: i32, anchor_z: i64) -> SupportPlanEntry {
    // exhaustive: this identity fixture must pin every support-plan field
    SupportPlanEntry {
        global_layer_index,
        object_id: "object-a".into(),
        region_id: 7,
        family_id: "tree-support-family".into(),
        demand_ids: vec![format!("demand-{anchor_z}")],
        body_ids: vec![format!("body-{anchor_z}")],
        anchor_layer_index: 1,
        anchor_z,
        roles: Vec::new(),
        skeleton: None,
        capabilities: Vec::new(),
        provenance: Vec::new(),
        decline_reason: None,
    }
}

#[test]
fn support_plan_view_retains_grid_and_off_grid_planes_on_one_anchor_layer() {
    let plan = SupportPlanIR {
        entries: vec![entry(1, 4_000), entry(i32::MIN, 5_000)],
        ..SupportPlanIR::default()
    };

    let data = build_support_plan_layer_data_for_test(1, &plan);
    let bucket = data
        .support_plan_entries
        .get(&("object-a".into(), "7".into()))
        .expect("support-plan dispatch bucket");
    assert_eq!(
        bucket
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![4_000, 5_000]
    );

    let view = PaintRegionLayerView::new(1).with_support_plan(Arc::new(plan));
    assert_eq!(
        view.support_plan_entries_for("object-a", 7)
            .iter()
            .map(|entry| entry.anchor_z)
            .collect::<Vec<_>>(),
        vec![4_000, 5_000]
    );
}

#[test]
fn support_commit_retains_ordinary_paths_and_anchored_proposal() {
    let mut ctx = HostExecutionContextBuilder::new("support-both-payloads", 0.2, 0.2).build();
    ctx.support_output_mut()
        .support_paths
        .push(ExtrusionPath3d {
            points: vec![
                // Fixture pins every transported Point3WithWidth field so a
                // future transport widening (missing width/flow) fails this
                // test instead of silently defaulting (E=0 class, DEV-161).
                // exhaustive: all 9 fields intentional, reason above.
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
                // Second point: same fixture-pinning rationale as above.
                // exhaustive: all 9 fields intentional, reason above.
                Point3WithWidth {
                    x: 1.0,
                    y: 0.0,
                    z: 0.2,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                    overhang_distance_mm: None,
                },
            ],
            role: ExtrusionRole::SupportMaterial,
            speed_factor: 1.0,
            tool_index: None,
            order_lock: None,
        });
    ctx.anchored_events_mut().collection = Some(OrderedEventCollection {
        anchor_global_layer_index: 0,
        events: vec![
            // exhaustive: this fixture pins every anchored-entity field
            AnchoredEntity {
                local_id: 239,
                anchor_global_layer_index: 0,
                geometry: AnchoredGeometryContract::Planar { z: 2_100 },
                input_capabilities: vec!["support-plan".into()],
                output_capabilities: vec!["support-paths".into()],
                provenance: AnchoredEntityProvenance {
                    requesting_feature: "off-grid-regression".into(),
                    source_plan_entry: "off-grid-2100".into(),
                },
                path_points: vec![IrPoint3WithWidth {
                    x: 0.0,
                    y: 1.0,
                    z: 0.21,
                    width: 0.4,
                    flow_factor: 1.0,
                    ..Default::default()
                }],
                role: IrExtrusionRole::SupportMaterial,
            },
        ],
        runtime_hooks: AnchoredEventRuntimeHooks::default(),
    });

    let commit = deconstruct_layer_ctx(
        "Layer::Support",
        "support-both-payloads",
        0,
        None,
        ctx,
        None,
    )
    .expect("valid support output")
    .expect("support stage commit");
    let slicer_ir::LayerStageCommit::SupportWithAnchoredEvents {
        support,
        anchored_events,
    } = commit
    else {
        panic!("one support-stage commit must retain both payloads: {commit:?}");
    };
    assert_eq!(support.entries.len(), 1);
    assert_eq!(anchored_events.len(), 1);
    assert_eq!(
        anchored_events[0].events[0].provenance.source_plan_entry,
        "off-grid-2100"
    );
}
