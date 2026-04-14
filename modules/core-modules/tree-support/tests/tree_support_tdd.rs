use std::collections::HashMap;

use slicer_ir::{ConfigValue, ConfigView, ExPolygon, ExtrusionRole, Point2, Polygon};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use tree_support::TreeSupport;

fn make_config(enabled: bool, density: f64, angle: f64, speed: f64, line_width: f64) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("support_enabled".to_string(), ConfigValue::Bool(enabled));
    fields.insert("support_density".to_string(), ConfigValue::Float(density));
    fields.insert("support_angle".to_string(), ConfigValue::Float(angle));
    fields.insert("support_speed".to_string(), ConfigValue::Float(speed));
    fields.insert("line_width".to_string(), ConfigValue::Float(line_width));
    ConfigView { fields }
}

fn make_square_region(size_mm: f32, z: f32) -> SliceRegionView {
    let half = size_mm / 2.0;
    let square = ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(-half, -half),
                Point2::from_mm(half, -half),
                Point2::from_mm(half, half),
                Point2::from_mm(-half, half),
            ],
        },
        holes: vec![],
    };
    SliceRegionView::new(
        "obj1".to_string(),
        1,
        vec![square.clone()],
        vec![square], // infill_areas
        0.2,
        z,
        false,
    )
}

/// Test 1: on_print_start with empty config uses defaults.
#[test]
fn on_print_start_defaults() {
    let config = ConfigView {
        fields: HashMap::new(),
    };
    let module = TreeSupport::on_print_start(&config).unwrap();
    assert!(!module.enabled());
    assert!((module.density() - 0.2).abs() < 0.001);
    assert!((module.line_width() - 0.4).abs() < 0.001);
}

/// Test 2: on_print_start reads custom config values.
#[test]
fn on_print_start_custom() {
    let config = make_config(true, 0.5, 15.0, 80.0, 0.6);
    let module = TreeSupport::on_print_start(&config).unwrap();
    assert!(module.enabled());
    assert!((module.density() - 0.5).abs() < 0.001);
    assert!((module.line_width() - 0.6).abs() < 0.001);
}

/// Test 3: A 10mm square region with support enabled produces non-empty paths.
#[test]
fn square_region_produces_paths() {
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TreeSupport::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
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
    let module = TreeSupport::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
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
    let module = TreeSupport::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
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
    let config = make_config(true, 0.0, 0.0, 50.0, 0.4);
    let module = TreeSupport::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
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
    let module = TreeSupport::on_print_start(&config).unwrap();

    let region = SliceRegionView::new(
        "obj1".to_string(),
        1,
        vec![], // empty polygons
        vec![],
        0.2,
        0.3,
        false,
    );

    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert_eq!(
        output.support_paths().len(),
        0,
        "empty regions should produce no paths"
    );
}

/// Test 8: All output points are at the correct z height.
#[test]
fn paths_at_correct_z() {
    let z = 1.5_f32;
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TreeSupport::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, z);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
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

/// Test 9: Branching pattern is present -- paths have varying directions
/// (not all parallel like traditional support).
#[test]
fn branching_pattern_present() {
    let config = make_config(true, 0.3, 0.0, 50.0, 0.4);
    let module = TreeSupport::on_print_start(&config).unwrap();

    // Use a large region to get many branches
    let region = make_square_region(20.0, 0.3);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let paths = output.support_paths();
    assert!(
        paths.len() >= 3,
        "need at least 3 paths to verify branching, got {}",
        paths.len()
    );

    // Compute angles of each path segment
    let mut angles: Vec<f64> = Vec::new();
    for path in paths {
        if path.points.len() >= 2 {
            let dx = (path.points.last().unwrap().x - path.points[0].x) as f64;
            let dy = (path.points.last().unwrap().y - path.points[0].y) as f64;
            if dx.abs() > 0.001 || dy.abs() > 0.001 {
                angles.push(dy.atan2(dx));
            }
        }
    }

    assert!(
        angles.len() >= 3,
        "need at least 3 non-degenerate path angles, got {}",
        angles.len()
    );

    // Verify that not all angles are the same -- at least two paths differ by
    // more than 10 degrees, indicating branching (not parallel lines).
    let mut has_different_angles = false;
    'outer: for i in 0..angles.len() {
        for j in (i + 1)..angles.len() {
            let diff = (angles[i] - angles[j]).abs();
            // Normalize to [0, PI]
            let diff = if diff > std::f64::consts::PI {
                2.0 * std::f64::consts::PI - diff
            } else {
                diff
            };
            if diff > 10.0_f64.to_radians() {
                has_different_angles = true;
                break 'outer;
            }
        }
    }

    assert!(
        has_different_angles,
        "tree support should have varying branch directions, but all angles are similar: {:?}",
        angles
    );
}

/// Test 10: Higher density produces more paths than lower density.
#[test]
fn density_affects_coverage() {
    let config_low = make_config(true, 0.1, 0.0, 50.0, 0.4);
    let config_high = make_config(true, 0.5, 0.0, 50.0, 0.4);

    let module_low = TreeSupport::on_print_start(&config_low).unwrap();
    let module_high = TreeSupport::on_print_start(&config_high).unwrap();

    let region_low = make_square_region(10.0, 0.3);
    let region_high = make_square_region(10.0, 0.3);

    let paint = PaintRegionLayerView::new(0);
    let mut output_low = SupportOutputBuilder::new();
    let mut output_high = SupportOutputBuilder::new();

    module_low
        .run_support(0, &[region_low], &paint, &mut output_low, &config_low)
        .unwrap();
    module_high
        .run_support(0, &[region_high], &paint, &mut output_high, &config_high)
        .unwrap();

    let count_low = output_low.support_paths().len();
    let count_high = output_high.support_paths().len();

    assert!(
        count_high > count_low,
        "higher density should produce more paths: low={}, high={}",
        count_low,
        count_high
    );
}

/// Test 11: All point widths match the configured line_width.
#[test]
fn width_matches_config() {
    let lw = 0.6_f32;
    let config = make_config(true, 0.2, 0.0, 50.0, lw as f64);
    let module = TreeSupport::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
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
