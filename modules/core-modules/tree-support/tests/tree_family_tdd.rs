#![allow(missing_docs)]

use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, Polygon, SupportPlanDeclineReason, SupportPlanIR, SupportPlanRole,
    SupportPlanRoleRegion,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;
use slicer_wasm_host::marshal::convert_native_support_output_with_plan;
use tree_support::TreeSupport;

fn square(size: f32) -> ExPolygon {
    ExPolygon {
        contour: Polygon {
            points: vec![
                slicer_ir::Point2::from_mm(0.0, 0.0),
                slicer_ir::Point2::from_mm(size, 0.0),
                slicer_ir::Point2::from_mm(size, size),
                slicer_ir::Point2::from_mm(0.0, size),
            ],
        },
        holes: vec![],
    }
}

fn fixture(
    family: &str,
    decline_reason: Option<SupportPlanDeclineReason>,
) -> (ConfigView, SliceRegionView, PaintRegionLayerView) {
    let config = ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_speed", 50.0)
        .float("line_width", 0.4)
        .int("tree_support_wall_count", 2)
        .build();
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    let entry = slicer_ir::SupportPlanEntry {
        global_layer_index: 0,
        object_id: "obj1".into(),
        region_id: 1,
        family_id: family.into(),
        demand_ids: vec!["demand-1".into()],
        body_ids: vec!["body-1".into()],
        anchor_layer_index: 0,
        anchor_z: 300,
        // Deliberately *overlapping* roles: the interface square sits inside
        // the body square. F-37 wired canonical `generate_interface_layers`'
        // regularization into the renderer, so the roof is `closing`-expanded
        // by the minimum island radius and then subtracted from the base
        // (`intermediate_layer.polygons = diff(intermediate_layer.polygons,
        // interface)`). The body square is 6 mm (not 4 mm) so the remaining
        // base ring is still wide enough for two walls plus fill.
        roles: vec![
            SupportPlanRoleRegion {
                role: SupportPlanRole::SupportBody,
                regions: vec![square(6.0)],
            },
            SupportPlanRoleRegion {
                role: SupportPlanRole::TopInterface,
                regions: vec![square(2.0)],
            },
        ],
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["tree-planner".into()],
        decline_reason,
    };
    let region = SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .z(0.3)
        .add_polygon(square_polygon(0.0, 0.0, 4.0))
        .build();
    let paint = PaintRegionLayerView::new(0).with_support_plan(Arc::new(SupportPlanIR {
        entries: vec![entry],
        ..Default::default()
    }));
    (config, region, paint)
}

#[test]
fn polygon_renderer_identity() {
    let (config, region, paint) = fixture("tree", None);
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &mut slicer_sdk::LayerCollectionBuilder::new(), &config)
        .unwrap();
    assert!(
        output.support_paths().len() > 2,
        "body must contain walls and fill paths"
    );
    assert!(
        !output.interface_paths().is_empty(),
        "top-interface geometry must be emitted"
    );
    assert!(output
        .support_paths()
        .iter()
        .all(|path| path.points.len() > 2 || path.points[0].width < 4.0));

    let support = convert_native_support_output_with_plan(
        &output,
        0,
        paint.support_plan().expect("fixture plan"),
    )
    .expect("native host join");
    assert!(support.entries.iter().any(|entry| {
        entry.family_id == "tree"
            && entry.body_id == "body-1"
            && entry.demand_ids == ["demand-1"]
            && entry.role == slicer_ir::SupportRole::SupportBody
            && entry.object_id == "obj1"
            && entry.region_id == 1
            && entry.paths.len() > 2
    }));
    assert!(support.entries.iter().any(|entry| {
        entry.family_id == "tree"
            && entry.body_id == "body-1"
            && entry.demand_ids == ["demand-1"]
            && entry.role == slicer_ir::SupportRole::TopInterface
            && entry.object_id == "obj1"
            && entry.region_id == 1
            && !entry.paths.is_empty()
    }));
}

#[test]
fn mismatched_family_rejected() {
    let (config, region, paint) = fixture("traditional", None);
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    let result = module.run_support(0, &[region], &paint, &mut output, &mut slicer_sdk::LayerCollectionBuilder::new(), &config);
    assert!(result.is_err());
    assert!(output.support_paths().is_empty());
}

#[test]
fn declined_or_empty_plan_has_no_fallback_paths() {
    let (config, region, paint) = fixture("tree", Some(SupportPlanDeclineReason::Blocked));
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region.clone()], &paint, &mut output, &mut slicer_sdk::LayerCollectionBuilder::new(), &config)
        .unwrap();
    assert!(output.support_paths().is_empty());
    assert!(output.interface_paths().is_empty());

    let empty_paint = PaintRegionLayerView::new(0).with_support_plan(Arc::new(SupportPlanIR {
        entries: Vec::new(),
        ..Default::default()
    }));
    let mut empty_output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &empty_paint, &mut empty_output, &mut slicer_sdk::LayerCollectionBuilder::new(), &config)
        .unwrap();
    assert!(empty_output.support_paths().is_empty());
    assert!(empty_output.interface_paths().is_empty());
}
