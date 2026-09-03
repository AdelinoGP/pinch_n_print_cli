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
        // 3000 units = the region's own 0.3 mm plane below; on-grid by the
        // COORDINATE_TOLERANCE_UNITS discriminator, so the on-grid route stays
        // byte-identical for this fixture (packet 239c Step 4).
        anchor_z: 3_000,
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
fn offgrid_plan_entry_renders_at_declared_anchor_z() {
    // Packet 239c AC-4: an entry whose `anchor_z` is off-grid (beyond
    // `AnchoredGeometryContract::COORDINATE_TOLERANCE_UNITS`) renders at the
    // declared plane — never at `region.z()` — and its paths leave via 239b's
    // anchored drain with the collection's declared plane equal to
    // `entry.anchor_z` and its anchor index equal to `entry.anchor_layer_index`.
    let (config, region, paint) = fixture("tree", None);
    // Rebuild the plan with the entry's anchor_z off-grid (2.9 mm against the
    // region's 0.3 mm plane): the renderer must emit at the declared plane.
    // exhaustive: AC-4 fixture pins the declared-plane semantics this test exists to prove
    let off_grid_entry = slicer_ir::SupportPlanEntry {
        global_layer_index: 0,
        object_id: "obj1".into(),
        region_id: 1,
        family_id: "tree".into(),
        demand_ids: vec!["demand-1".into()],
        body_ids: vec!["body-1".into()],
        anchor_layer_index: 3,
        anchor_z: slicer_ir::mm_to_units(2.9),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square(6.0)],
        }],
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["tree-planner".into()],
        decline_reason: None,
    };
    let off_grid_paint = PaintRegionLayerView::new(0).with_support_plan(Arc::new(SupportPlanIR {
        entries: vec![off_grid_entry],
        ..Default::default()
    }));
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    let mut collection = slicer_sdk::LayerCollectionBuilder::new();
    module
        .run_support(
            0,
            &[region.clone()],
            &off_grid_paint,
            &mut output,
            &mut collection,
            &config,
        )
        .unwrap();
    // Nothing may take the on-grid route at the region's 0.3 mm plane: every
    // emitted path point must sit at the declared 2.9 mm plane.
    let expected_z = slicer_ir::units_to_mm(slicer_ir::mm_to_units(2.9));
    assert!(
        output.support_paths().is_empty() && output.interface_paths().is_empty(),
        "an off-grid entry must not be emitted through the on-grid push route"
    );
    let proposal = collection
        .anchored_proposal()
        .expect("an off-grid plan entry must leave as an anchored event collection proposal");
    assert_eq!(proposal.anchor_global_layer_index, 3);
    assert!(
        !proposal.events.is_empty(),
        "the anchored proposal must carry the off-grid paths"
    );
    for event in &proposal.events {
        assert_eq!(
            event.geometry,
            slicer_ir::AnchoredGeometryContract::Planar {
                z: slicer_ir::mm_to_units(2.9)
            },
            "the collection's declared plane must equal entry.anchor_z"
        );
        assert_eq!(event.anchor_global_layer_index, 3);
        for point in &event.path_points {
            assert!(
                (point.z - expected_z).abs() <= 1e-3,
                "every emitted point's Z must equal the declared plane {expected_z}; got {}",
                point.z
            );
        }
    }
    // And the on-grid route still works unchanged: the original fixture runs
    // the entry at the region plane and must NOT propose anchored events.
    let mut on_grid_output = SupportOutputBuilder::new();
    let mut on_grid_collection = slicer_sdk::LayerCollectionBuilder::new();
    module
        .run_support(
            0,
            &[region],
            &paint,
            &mut on_grid_output,
            &mut on_grid_collection,
            &config,
        )
        .unwrap();
    assert!(on_grid_collection.anchored_proposal().is_none());
    assert!(!on_grid_output.support_paths().is_empty());
}

#[test]
fn polygon_renderer_identity() {
    let (config, region, paint) = fixture("tree", None);
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(
            0,
            &[region],
            &paint,
            &mut output,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
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
fn support_paths_are_unique_per_layer() {
    let (config, region, paint) = fixture("tree", None);
    let mut plan = paint.support_plan().expect("fixture plan").as_ref().clone();
    let body = plan.entries[0]
        .roles
        .iter_mut()
        .find(|role| role.role == SupportPlanRole::SupportBody)
        .expect("body role");
    body.regions.push(square(6.0));
    body.regions.push(square(3.0));
    let paint = PaintRegionLayerView::new(0).with_support_plan(Arc::new(plan));
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(
            0,
            &[region],
            &paint,
            &mut output,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
        .unwrap();

    let mut unique = std::collections::BTreeSet::new();
    for path in output.support_paths() {
        let signature: Vec<_> = path
            .points
            .iter()
            .map(|point| {
                (
                    point.x.to_bits(),
                    point.y.to_bits(),
                    point.z.to_bits(),
                    point.width.to_bits(),
                )
            })
            .collect();
        assert!(
            unique.insert(signature),
            "one object/layer/role must not render an identical path twice"
        );
    }
}

#[test]
fn mismatched_family_rejected() {
    let (config, region, paint) = fixture("traditional", None);
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    let result = module.run_support(
        0,
        &[region],
        &paint,
        &mut output,
        &mut slicer_sdk::LayerCollectionBuilder::new(),
        &config,
    );
    assert!(result.is_err());
    assert!(output.support_paths().is_empty());
}

#[test]
fn declined_or_empty_plan_has_no_fallback_paths() {
    let (config, region, paint) = fixture("tree", Some(SupportPlanDeclineReason::Blocked));
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(
            0,
            &[region.clone()],
            &paint,
            &mut output,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
        .unwrap();
    assert!(output.support_paths().is_empty());
    assert!(output.interface_paths().is_empty());

    let empty_paint = PaintRegionLayerView::new(0).with_support_plan(Arc::new(SupportPlanIR {
        entries: Vec::new(),
        ..Default::default()
    }));
    let mut empty_output = SupportOutputBuilder::new();
    module
        .run_support(
            0,
            &[region],
            &empty_paint,
            &mut empty_output,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
        .unwrap();
    assert!(empty_output.support_paths().is_empty());
    assert!(empty_output.interface_paths().is_empty());
}
