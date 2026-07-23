//! TDD tests for the support-surface-ironing module.
//!
//! These tests were written BEFORE the implementation per TDD methodology.

use std::collections::HashMap;

use slicer_ir::{
    ConfigValue, ConfigView, ExtrusionPath3D, ExtrusionRole, LoopType, Point3WithWidth,
    WallBoundaryType, WallLoop, WidthProfile,
};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::square_polygon;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::PerimeterRegionView;
use support_surface_ironing::SupportSurfaceIroning;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a ConfigView with the given key-value pairs.
fn config_with(entries: Vec<(&str, ConfigValue)>) -> ConfigView {
    let mut fields = HashMap::new();
    for (k, v) in entries {
        fields.insert(k.to_string(), v);
    }
    ConfigView::from_map(fields)
}

/// Create an enabled config with optional overrides.
fn enabled_config() -> ConfigView {
    config_with(vec![("ironing_enabled", ConfigValue::Bool(true))])
}

/// Create a WallLoop whose path points are at the given z height.
fn wall_loop_at_z(z: f32) -> WallLoop {
    WallLoop {
        perimeter_index: 0,
        loop_type: LoopType::Outer,
        path: ExtrusionPath3D {
            points: vec![
                Point3WithWidth {
                    x: 0.0,
                    y: 0.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
                Point3WithWidth {
                    x: 10.0,
                    y: 0.0,
                    z,
                    width: 0.4,
                    flow_factor: 1.0,
                    overhang_quartile: None,
                    dist_to_top_mm: 0.0,
                },
            ],
            role: ExtrusionRole::OuterWall,
            speed_factor: 1.0,
        },
        width_profile: WidthProfile {
            widths: vec![0.4, 0.4],
        },
        feature_flags: vec![],
        boundary_type: WallBoundaryType::ExteriorSurface,
    }
}

/// Build a PerimeterRegionView with a 10mm square at given z.
fn region_with_square_at_z(z: f32) -> PerimeterRegionView {
    {
        let mut tmp = PerimeterRegionView::default();
        tmp.set_object_id("obj-0".to_string());
        tmp.set_region_id(0);
        tmp.set_wall_loops(vec![wall_loop_at_z(z)]);
        tmp.set_infill_areas(vec![square_polygon(5.0, 5.0, 10.0)]);
        tmp.set_seam_candidates(vec![]);
        tmp.set_resolved_seam(None);
        tmp
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn on_print_start_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    assert!(!module.enabled());
    assert!((module.ironing_speed() - 15.0).abs() < 0.001);
    assert!((module.ironing_flow_rate() - 0.1).abs() < 0.001);
    assert!((module.ironing_spacing() - 0.1).abs() < 0.001);
}

#[test]
fn on_print_start_custom() {
    let config = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_speed", ConfigValue::Float(20.0)),
        ("ironing_flow_rate", ConfigValue::Float(0.2)),
        ("ironing_spacing", ConfigValue::Float(0.15)),
        ("line_width", ConfigValue::Float(0.5)),
    ]);
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    assert!(module.enabled());
    assert!((module.ironing_speed() - 20.0).abs() < 0.001);
    assert!((module.ironing_flow_rate() - 0.2).abs() < 0.001);
    assert!((module.ironing_spacing() - 0.15).abs() < 0.001);
    assert!((module.line_width() - 0.5).abs() < 0.001);
}

#[test]
fn disabled_no_paths() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, &[region], &[], &mut output, &config)
        .unwrap();
    assert!(output.ironing_paths().is_empty());
}

#[test]
fn square_region_produces_paths() {
    let config = enabled_config();
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, &[region], &[], &mut output, &config)
        .unwrap();
    assert!(
        !output.ironing_paths().is_empty(),
        "expected ironing paths for a 10mm square region"
    );
}

#[test]
fn paths_have_ironing_role() {
    let config = enabled_config();
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, &[region], &[], &mut output, &config)
        .unwrap();
    for path in output.ironing_paths() {
        assert_eq!(
            path.role,
            ExtrusionRole::Ironing,
            "all ironing paths must have ExtrusionRole::Ironing"
        );
    }
}

#[test]
fn empty_regions_no_output() {
    let config = enabled_config();
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, &[], &[], &mut output, &config)
        .unwrap();
    assert!(output.ironing_paths().is_empty());
}

#[test]
fn paths_at_correct_z() {
    let z = 1.5_f32;
    let config = enabled_config();
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    let region = region_with_square_at_z(z);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, &[region], &[], &mut output, &config)
        .unwrap();
    assert!(!output.ironing_paths().is_empty());
    for path in output.ironing_paths() {
        for pt in &path.points {
            assert!((pt.z - z).abs() < 0.001, "expected z={z}, got z={}", pt.z);
        }
    }
}

#[test]
fn flow_rate_applied() {
    let config = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_flow_rate", ConfigValue::Float(0.15)),
    ]);
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, &[region], &[], &mut output, &config)
        .unwrap();
    assert!(!output.ironing_paths().is_empty());
    for path in output.ironing_paths() {
        for pt in &path.points {
            assert!(
                (pt.flow_factor - 0.15).abs() < 0.001,
                "expected flow_factor=0.15, got {}",
                pt.flow_factor
            );
        }
    }
}

#[test]
fn spacing_affects_density() {
    // Narrow spacing => more paths
    let config_narrow = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_spacing", ConfigValue::Float(0.1)),
    ]);
    let module_narrow = SupportSurfaceIroning::on_print_start(&config_narrow).unwrap();
    let region_narrow = region_with_square_at_z(1.0);
    let mut output_narrow = InfillOutputBuilder::new();
    module_narrow
        .run_infill_postprocess(0, &[region_narrow], &[], &mut output_narrow, &config_narrow)
        .unwrap();

    // Wide spacing => fewer paths
    let config_wide = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_spacing", ConfigValue::Float(0.4)),
    ]);
    let module_wide = SupportSurfaceIroning::on_print_start(&config_wide).unwrap();
    let region_wide = region_with_square_at_z(1.0);
    let mut output_wide = InfillOutputBuilder::new();
    module_wide
        .run_infill_postprocess(0, &[region_wide], &[], &mut output_wide, &config_wide)
        .unwrap();

    assert!(
        output_narrow.ironing_paths().len() > output_wide.ironing_paths().len(),
        "narrow spacing ({}) should produce more paths than wide spacing ({})",
        output_narrow.ironing_paths().len(),
        output_wide.ironing_paths().len()
    );
}

#[test]
fn width_matches_config() {
    let config = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("line_width", ConfigValue::Float(0.4)),
    ]);
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, &[region], &[], &mut output, &config)
        .unwrap();
    assert!(!output.ironing_paths().is_empty());
    for path in output.ironing_paths() {
        for pt in &path.points {
            assert!(
                (pt.width - 0.4).abs() < 0.001,
                "expected width=0.4, got {}",
                pt.width
            );
        }
    }
}

#[test]
fn rectilinear_pattern() {
    // For a large region, paths should have parallel-line geometry:
    // each path should have exactly 2 points (start/end of a scan line),
    // and all scan lines should share the same Y direction (horizontal lines
    // means all points in a path have the same Y).
    let config = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_spacing", ConfigValue::Float(0.5)),
    ]);
    let module = SupportSurfaceIroning::on_print_start(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = InfillOutputBuilder::new();
    module
        .run_infill_postprocess(0, &[region], &[], &mut output, &config)
        .unwrap();

    let paths = output.ironing_paths();
    assert!(paths.len() >= 2, "expected multiple scan lines");

    for path in paths {
        // Each scan line segment should have exactly 2 points
        assert_eq!(
            path.points.len(),
            2,
            "each ironing scan line should be a 2-point segment"
        );

        // Both points in a scan line should have the same Y (horizontal lines)
        let y0 = path.points[0].y;
        let y1 = path.points[1].y;
        assert!(
            (y0 - y1).abs() < 0.001,
            "scan line points should have same Y: {} vs {}",
            y0,
            y1
        );
    }
}
