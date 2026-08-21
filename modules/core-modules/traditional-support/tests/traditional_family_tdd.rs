#![allow(missing_docs)]

use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExPolygon, Polygon, SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;
use slicer_wasm_host::marshal::convert_native_support_output_with_plan;
use traditional_support::TraditionalSupport;

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
    roles: Vec<SupportPlanRoleRegion>,
) -> (ConfigView, SliceRegionView, PaintRegionLayerView) {
    let config = ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_density", 20.0)
        .float("support_speed", 50.0)
        .float("line_width", 0.4)
        .build();
    fixture_with_config(config, family, roles)
}

fn fixture_with_config(
    config: ConfigView,
    family: &str,
    roles: Vec<SupportPlanRoleRegion>,
) -> (ConfigView, SliceRegionView, PaintRegionLayerView) {
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
        roles,
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["traditional-planner".into()],
        decline_reason: None,
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

/// Deliberately *overlapping* roles: the interface square sits inside the body
/// square. F-37 wired canonical `generate_interface_layers`'
/// `intermediate_layer.polygons = diff(intermediate_layer.polygons, interface)`
/// into the renderer, so the body is now filled over the L-shaped remainder
/// rather than over the whole square. The body square is 6 mm (not 4 mm) so
/// that remainder still contains a scan line that is not collinear with the
/// notch edge at the 2 mm body pitch; the interface square must stay at 2 mm
/// for `interface_spacing_config_controls_scan_fill` to resolve two pitches.
fn body_and_interface_roles() -> Vec<SupportPlanRoleRegion> {
    vec![
        SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square(6.0)],
        },
        SupportPlanRoleRegion {
            role: SupportPlanRole::TopInterface,
            regions: vec![square(2.0)],
        },
    ]
}

#[test]
fn planned_polygon_renderer() {
    let (config, region, paint) = fixture("traditional", body_and_interface_roles());
    let module = TraditionalSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();
    assert!(
        !output.support_paths().is_empty(),
        "body polygons must be scan-filled"
    );
    assert!(
        !output.interface_paths().is_empty(),
        "top-interface geometry must be emitted"
    );

    let support = convert_native_support_output_with_plan(
        &output,
        0,
        paint.support_plan().expect("fixture plan"),
    )
    .expect("native host join");
    assert!(support.entries.iter().any(|entry| {
        entry.family_id == "traditional"
            && entry.body_id == "body-1"
            && entry.demand_ids == ["demand-1"]
            && entry.role == slicer_ir::SupportRole::SupportBody
            && entry.object_id == "obj1"
            && entry.region_id == 1
            && !entry.paths.is_empty()
    }));
    assert!(support.entries.iter().any(|entry| {
        entry.family_id == "traditional"
            && entry.body_id == "body-1"
            && entry.demand_ids == ["demand-1"]
            && entry.role == slicer_ir::SupportRole::TopInterface
            && entry.object_id == "obj1"
            && entry.region_id == 1
            && !entry.paths.is_empty()
    }));
}

#[test]
fn interface_spacing_config_controls_scan_fill() {
    let (config, region, paint) = fixture("traditional", body_and_interface_roles());
    let module = TraditionalSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();
    let default_count = output.interface_paths().len();

    let wide_config = ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_density", 20.0)
        .float("support_speed", 50.0)
        .float("line_width", 0.4)
        .float("support_interface_spacing", 0.8)
        .build();
    let (config, region, paint) =
        fixture_with_config(wide_config, "traditional", body_and_interface_roles());
    let module = TraditionalSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();
    let wide_count = output.interface_paths().len();

    assert!(wide_count > 0, "interface must still be scan-filled");
    assert!(
        wide_count < default_count,
        "wider interface spacing must yield fewer interface scan lines (default={default_count}, wide={wide_count})"
    );
}

#[test]
fn mismatched_or_missing_plan() {
    let (config, region, paint) = fixture("tree", body_and_interface_roles());
    let module = TraditionalSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    let result = module.run_support(0, &[region], &paint, &mut output, &config);
    assert!(result.is_err(), "non-traditional family must be rejected");
    assert!(output.support_paths().is_empty());
    assert!(output.interface_paths().is_empty());

    let (config, region, paint) = fixture("traditional", Vec::new());
    let mut output = SupportOutputBuilder::new();
    let result = module.run_support(0, &[region], &paint, &mut output, &config);
    assert!(result.is_err(), "plan without polygons must be rejected");
    assert!(output.support_paths().is_empty());
    assert!(output.interface_paths().is_empty());
}
