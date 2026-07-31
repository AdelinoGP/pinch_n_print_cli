//! TDD tests for the support-surface-ironing module.
//!
//! These tests were written BEFORE the implementation per TDD methodology.
//!
//! The module's declared stage is `Layer::SupportPostProcess` (see
//! `support-surface-ironing.toml` and `slicer_module_binding_tdd.rs`), so these
//! tests drive `LayerModule::run_support_postprocess` over `SliceRegionView`
//! and assert on `SupportOutputBuilder::support_paths`.

use std::collections::HashMap;

use slicer_ir::{ConfigValue, ConfigView, ExtrusionRole};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::test_prelude::square_polygon;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;
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

/// Build a SliceRegionView with a 10mm square slice polygon at the given z.
fn region_with_square_at_z(z: f32) -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-0".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![square_polygon(5.0, 5.0, 10.0)]);
    region.set_z(z);
    region
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn from_config_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    assert!(!module.enabled());
    assert!((module.ironing_speed() - 15.0).abs() < 0.001);
    assert!((module.ironing_flow_rate() - 0.1).abs() < 0.001);
    assert!((module.ironing_spacing() - 0.1).abs() < 0.001);
}

#[test]
fn from_config_custom() {
    let config = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_speed", ConfigValue::Float(20.0)),
        ("ironing_flow_rate", ConfigValue::Float(0.2)),
        ("ironing_spacing", ConfigValue::Float(0.15)),
        ("line_width", ConfigValue::Float(0.5)),
    ]);
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    assert!(module.enabled());
    assert!((module.ironing_speed() - 20.0).abs() < 0.001);
    assert!((module.ironing_flow_rate() - 0.2).abs() < 0.001);
    assert!((module.ironing_spacing() - 0.15).abs() < 0.001);
    assert!((module.line_width() - 0.5).abs() < 0.001);
}

#[test]
fn disabled_no_paths() {
    let config = ConfigView::from_map(HashMap::new());
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[region], &mut output, &config)
        .unwrap();
    assert!(output.support_paths().is_empty());
}

#[test]
fn square_region_produces_paths() {
    let config = enabled_config();
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[region], &mut output, &config)
        .unwrap();
    assert!(
        !output.support_paths().is_empty(),
        "expected ironing paths for a 10mm square region"
    );
}

#[test]
fn paths_have_ironing_role() {
    let config = enabled_config();
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[region], &mut output, &config)
        .unwrap();
    for path in output.support_paths() {
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
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[], &mut output, &config)
        .unwrap();
    assert!(output.support_paths().is_empty());
}

#[test]
fn paths_at_correct_z() {
    let z = 1.5_f32;
    let config = enabled_config();
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let region = region_with_square_at_z(z);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[region], &mut output, &config)
        .unwrap();
    assert!(!output.support_paths().is_empty());
    for path in output.support_paths() {
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
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[region], &mut output, &config)
        .unwrap();
    assert!(!output.support_paths().is_empty());
    for path in output.support_paths() {
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
    let module_narrow = SupportSurfaceIroning::from_config(&config_narrow).unwrap();
    let region_narrow = region_with_square_at_z(1.0);
    let mut output_narrow = SupportOutputBuilder::new();
    module_narrow
        .run_support_postprocess(0, &[region_narrow], &mut output_narrow, &config_narrow)
        .unwrap();

    // Wide spacing => fewer paths
    let config_wide = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("ironing_spacing", ConfigValue::Float(0.4)),
    ]);
    let module_wide = SupportSurfaceIroning::from_config(&config_wide).unwrap();
    let region_wide = region_with_square_at_z(1.0);
    let mut output_wide = SupportOutputBuilder::new();
    module_wide
        .run_support_postprocess(0, &[region_wide], &mut output_wide, &config_wide)
        .unwrap();

    assert!(
        output_narrow.support_paths().len() > output_wide.support_paths().len(),
        "narrow spacing ({}) should produce more paths than wide spacing ({})",
        output_narrow.support_paths().len(),
        output_wide.support_paths().len()
    );
}

#[test]
fn width_matches_config() {
    let config = config_with(vec![
        ("ironing_enabled", ConfigValue::Bool(true)),
        ("line_width", ConfigValue::Float(0.4)),
    ]);
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[region], &mut output, &config)
        .unwrap();
    assert!(!output.support_paths().is_empty());
    for path in output.support_paths() {
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
    let module = SupportSurfaceIroning::from_config(&config).unwrap();
    let region = region_with_square_at_z(1.0);
    let mut output = SupportOutputBuilder::new();
    module
        .run_support_postprocess(0, &[region], &mut output, &config)
        .unwrap();

    let paths = output.support_paths();
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
