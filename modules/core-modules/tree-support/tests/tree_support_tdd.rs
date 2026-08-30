#![allow(missing_docs)]

use std::collections::HashMap;

use std::sync::Arc;

use slicer_ir::{
    ConfigView, ExtrusionRole, Point3, SupportPlanIR, SupportPlanRole, SupportPlanRoleRegion,
    SupportPlanSkeleton,
};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use tree_support::TreeSupport;

fn make_config(
    enabled: bool,
    _density: f64,
    angle: f64,
    speed: f64,
    line_width: f64,
) -> ConfigView {
    ConfigViewBuilder::new()
        .bool("enable_support", enabled)
        .float("support_base_pattern_spacing", 2.5)
        .float("support_angle", angle)
        .float("support_speed", speed)
        .float("support_line_width", line_width)
        .build()
}

fn make_square_region(size_mm: f32, z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj1")
        .region_id(1)
        .z(z)
        .add_polygon(square_polygon(0.0, 0.0, size_mm))
        .build()
}

fn paint_with_plan(family_id: &str) -> PaintRegionLayerView {
    paint_with_plan_at(family_id, 0)
}

fn paint_with_plan_at(family_id: &str, layer_index: i32) -> PaintRegionLayerView {
    // exhaustive: support-plan identity fixture; SupportPlanEntry has no Default impl and FRU would let a new plan field default silently
    let entry = slicer_ir::SupportPlanEntry {
        global_layer_index: layer_index,
        object_id: "obj1".into(),
        region_id: 1,
        family_id: family_id.into(),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square_polygon(0.0, 0.0, 10.0)],
        }],
        demand_ids: vec!["test-demand".into()],
        body_ids: vec!["test-body".into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["test".into()],
        decline_reason: None,
    };
    PaintRegionLayerView::new(layer_index as u32).with_support_plan(Arc::new(SupportPlanIR {
        entries: vec![entry],
        ..Default::default()
    }))
}

fn paint_with_interface_plan() -> PaintRegionLayerView {
    // exhaustive: interface-rendering fixture; SupportPlanEntry has no Default impl
    let entry = slicer_ir::SupportPlanEntry {
        global_layer_index: 0,
        object_id: "obj1".into(),
        region_id: 1,
        family_id: "tree".into(),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::TopInterface,
            regions: vec![square_polygon(0.0, 0.0, 10.0)],
        }],
        demand_ids: vec!["test-demand".into()],
        body_ids: vec!["test-body".into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        skeleton: None,
        capabilities: vec![],
        provenance: vec!["test".into()],
        decline_reason: None,
    };
    PaintRegionLayerView::new(0).with_support_plan(Arc::new(SupportPlanIR {
        entries: vec![entry],
        ..Default::default()
    }))
}

fn interface_paths(flow: f64) -> Vec<(slicer_ir::ExtrusionPath3D, bool)> {
    let config = ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_speed", 50.0)
        .float("support_line_width", 0.4)
        .float("support_interface_flow", flow)
        .build();
    let module = TreeSupport::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let paint = paint_with_interface_plan();
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
    output.interface_paths().to_vec()
}

/// Test 1: from_config with empty config uses defaults.
#[test]
fn from_config_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = TreeSupport::from_config(&config).unwrap();
    assert!(!module.enabled());
    assert!((module.line_width() - 0.45).abs() < 0.001);
}

/// Test 2: from_config reads custom config values.
#[test]
fn from_config_custom() {
    let config = make_config(true, 50.0, 15.0, 80.0, 0.6);
    let module = TreeSupport::from_config(&config).unwrap();
    assert!(module.enabled());
    assert!((module.line_width() - 0.6).abs() < 0.001);
}

/// Test 3: A 10mm square region with support enabled produces non-empty paths.
#[test]
fn square_region_produces_paths() {
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TreeSupport::from_config(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = paint_with_plan("tree");
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
        !output.support_paths().is_empty(),
        "enabled tree support on a 10mm square should produce paths"
    );
}

/// Test 4: All output paths have SupportMaterial role.
#[test]
fn paths_have_support_role() {
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TreeSupport::from_config(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = paint_with_plan("tree");
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

    assert!(!output.support_paths().is_empty());
    for path in output.support_paths() {
        assert_eq!(
            path.role,
            ExtrusionRole::SupportMaterial,
            "all tree support paths must be SupportMaterial"
        );
    }
}

/// Test 5: Disabled support produces no paths.
#[test]
fn disabled_no_paths() {
    let config = make_config(false, 0.2, 0.0, 50.0, 0.4);
    let module = TreeSupport::from_config(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = paint_with_plan("tree");
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

    assert_eq!(
        output.support_paths().len(),
        0,
        "disabled support should produce no paths"
    );
}

/// Test 6: Zero density produces no paths.
#[test]
fn zero_density_no_paths() {
    let config = make_config(false, 0.0, 0.0, 50.0, 0.4);
    let module = TreeSupport::from_config(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = paint_with_plan("tree");
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

    assert_eq!(
        output.support_paths().len(),
        0,
        "zero density should produce no paths"
    );
}

/// Test 7: Empty regions produce no output.
#[test]
fn empty_regions_no_output() {
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TreeSupport::from_config(&config).unwrap();

    let mut region = SliceRegionView::default();
    region.set_object_id("obj1".to_string());
    region.set_region_id(1);
    region.set_polygons(vec![]);
    // empty polygons

    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.3);
    region.set_has_nonplanar(false);

    let paint = PaintRegionLayerView::new(0);
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

    assert_eq!(
        output.support_paths().len(),
        0,
        "empty regions should produce no paths"
    );
}

#[test]
fn interface_pitch_derives_from_interface_flow_over_line_width() {
    let baseline = interface_paths(100.0);
    let doubled = interface_paths(200.0);
    let fallback_zero = interface_paths(0.0);
    let fallback_negative = interface_paths(-25.0);

    assert!(!baseline.is_empty());
    assert_eq!(baseline[0].0.points[0].width, 0.4);
    assert_eq!(doubled[0].0.points[0].width, 0.8);
    assert!(doubled.len() < baseline.len());
    assert_eq!(fallback_zero.len(), baseline.len());
    assert_eq!(fallback_negative.len(), baseline.len());
}

#[test]
fn nonpositive_interface_flow_falls_back_to_default_module_boundary() {
    let baseline = interface_paths(100.0);
    for flow in [0.0, -5.0] {
        let fallback = interface_paths(flow);
        assert_eq!(fallback.len(), baseline.len());
        for ((actual, _), (expected, _)) in fallback.iter().zip(&baseline) {
            assert_eq!(actual.points.len(), expected.points.len());
            for (point, default_point) in actual.points.iter().zip(&expected.points) {
                assert!(point.width > 0.0, "fallback must not emit zero-width paths");
                assert!(point.x.is_finite() && point.y.is_finite());
                assert_eq!(point.x, default_point.x);
                assert_eq!(point.y, default_point.y);
                assert_eq!(point.width, default_point.width);
            }
        }
    }
}

#[test]
fn zero_base_and_interface_spacing_clamp_to_solid_pitch() {
    let config = ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_base_pattern_spacing", 0.0)
        .float("support_interface_spacing", 0.0)
        .float("support_speed", 50.0)
        .float("support_line_width", 0.4)
        .build();
    let module = TreeSupport::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let paint = paint_with_plan("tree");
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
    let fill_paths: Vec<_> = output
        .support_paths()
        .iter()
        .filter(|path| path.points.len() == 2)
        .collect();
    assert!(
        fill_paths.len() >= 20,
        "zero base spacing must clamp to solid fill"
    );
    assert!(fill_paths
        .iter()
        .all(|path| path.points.iter().all(|point| point.width == 0.4)));
}

/// Test 8: All output points are at the correct z height.
#[test]
fn paths_at_correct_z() {
    let z = 1.5_f32;
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TreeSupport::from_config(&config).unwrap();

    let region = make_square_region(10.0, z);
    let paint = paint_with_plan("tree");
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

    assert!(!output.support_paths().is_empty());
    for path in output.support_paths() {
        for pt in &path.points {
            assert!(
                (pt.z - z).abs() < 0.001,
                "all points should be at z={}, got z={}",
                z,
                pt.z
            );
        }
    }
}

/// The renderer owns branch walls; density is a traditional-support concern.
#[test]
fn tree_support_wall_count() {
    let render = |wall_count: i64| {
        let config = ConfigViewBuilder::new()
            .bool("enable_support", true)
            .float("support_base_pattern_spacing", 2.5)
            .float("support_speed", 50.0)
            .float("line_width", 0.4)
            .int("tree_support_wall_count", wall_count)
            .build();
        let module = TreeSupport::from_config(&config).unwrap();
        let region = make_square_region(10.0, 0.3);
        let paint = paint_with_plan("tree");
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
        output
            .support_paths()
            .iter()
            .filter(|path| path.points.len() > 2)
            .count()
    };
    assert_eq!(render(1), 1);
    assert_eq!(render(3), 3);
}

#[test]
fn extra_wall_count_printed_from_skeleton() {
    let config = ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_base_pattern_spacing", 2.5)
        .float("support_speed", 50.0)
        .float("support_line_width", 0.4)
        .int("tree_support_wall_count", 1)
        .build();
    let module = TreeSupport::from_config(&config).unwrap();
    // exhaustive: skeleton wall-count fixture; SupportPlanEntry has no Default impl
    let entry = slicer_ir::SupportPlanEntry {
        global_layer_index: 0,
        object_id: "obj1".into(),
        region_id: 1,
        family_id: "tree".into(),
        roles: vec![SupportPlanRoleRegion {
            role: SupportPlanRole::SupportBody,
            regions: vec![square_polygon(0.0, 0.0, 10.0)],
        }],
        demand_ids: vec!["test-demand".into()],
        body_ids: vec!["test-body".into()],
        anchor_layer_index: 0,
        anchor_z: 0,
        skeleton: Some(SupportPlanSkeleton {
            points: vec![Point3 {
                x: 0.5,
                y: 0.5,
                z: 0.3,
            }],
            wall_counts: vec![2],
        }),
        capabilities: vec![],
        provenance: vec!["test".into()],
        decline_reason: None,
    };
    let paint = PaintRegionLayerView::new(0).with_support_plan(Arc::new(SupportPlanIR {
        entries: vec![entry],
        ..Default::default()
    }));
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(
            0,
            &[make_square_region(10.0, 0.3)],
            &paint,
            &mut output,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
        .unwrap();
    assert_eq!(
        output
            .support_paths()
            .iter()
            .filter(|path| path.points.len() > 2)
            .count(),
        3,
        "base wall plus two skeleton-requested extra walls"
    );
}

/// Test 11: All point widths match the configured line_width.
#[test]
fn width_matches_config() {
    let lw = 0.6_f32;
    let config = make_config(true, 0.2, 0.0, 50.0, lw as f64);
    let module = TreeSupport::from_config(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = paint_with_plan("tree");
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

    assert!(!output.support_paths().is_empty());
    for path in output.support_paths() {
        for pt in &path.points {
            assert!(
                (pt.width - lw).abs() < 0.001,
                "all point widths should be {}, got {}",
                lw,
                pt.width
            );
        }
    }
}

#[test]
fn opposite_family_plan_is_rejected() {
    let config = make_config(true, 20.0, 0.0, 50.0, 0.4);
    let module = TreeSupport::from_config(&config).unwrap();
    let region = make_square_region(10.0, 0.3);
    let paint = paint_with_plan("traditional");
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
fn tree_bodies_render_hollow_concentric_walls() {
    let config = ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_base_pattern_spacing", 2.5)
        .float("support_speed", 50.0)
        .float("support_line_width", 0.4)
        .int("tree_support_wall_count", 2)
        .build();
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(
            0,
            &[make_square_region(10.0, 0.3)],
            &paint_with_plan("tree"),
            &mut output,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
        .unwrap();
    let paths = output.support_paths();
    assert_eq!(paths.iter().filter(|p| p.points.len() > 2).count(), 2);
    assert!(
        paths.iter().any(|p| p.points.len() == 2),
        "interior must be filled"
    );
}

#[test]
fn body_fill_alternates_direction_across_layers() {
    let config = ConfigViewBuilder::new()
        .bool("enable_support", true)
        .float("support_base_pattern_spacing", 2.5)
        .float("support_speed", 50.0)
        .float("support_line_width", 0.4)
        .build();
    let module = TreeSupport::from_config(&config).unwrap();
    let mut horizontal = SupportOutputBuilder::new();
    module
        .run_support(
            0,
            &[make_square_region(10.0, 0.3)],
            &paint_with_plan("tree"),
            &mut horizontal,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
        .unwrap();
    let mut vertical = SupportOutputBuilder::new();
    let paint = paint_with_plan_at("tree", 1);
    module
        .run_support(
            1,
            &[make_square_region(10.0, 0.5)],
            &paint,
            &mut vertical,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
        .unwrap();
    let h = horizontal
        .support_paths()
        .iter()
        .find(|p| p.points.len() == 2)
        .unwrap();
    let v = vertical
        .support_paths()
        .iter()
        .find(|p| p.points.len() == 2)
        .unwrap();
    assert!((h.points[0].y - h.points[1].y).abs() < 0.001);
    assert!((v.points[0].x - v.points[1].x).abs() < 0.001);
}

#[test]
fn sub_pitch_tip_region_emits_solid_center_line() {
    let config = make_config(true, 0.0, 0.0, 50.0, 0.4);
    let module = TreeSupport::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support(
            0,
            &[make_square_region(0.5, 0.3)],
            &paint_with_plan("tree"),
            &mut output,
            &mut slicer_sdk::LayerCollectionBuilder::new(),
            &config,
        )
        .unwrap();
    assert!(output.support_paths().iter().any(|p| p.points.len() == 2));
}
