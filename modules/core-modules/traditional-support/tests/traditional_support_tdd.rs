use std::collections::HashMap;

use slicer_ir::{ConfigValue, ConfigView, ExPolygon, ExtrusionRole, Point2, Polygon};
use slicer_sdk::builders::SupportOutputBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

use traditional_support::TraditionalSupport;

fn make_config(
    enabled: bool,
    density: f64,
    angle: f64,
    speed: f64,
    line_width: f64,
) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert(
        "support_enabled".to_string(),
        ConfigValue::Bool(enabled),
    );
    fields.insert(
        "support_density".to_string(),
        ConfigValue::Float(density),
    );
    fields.insert(
        "support_angle".to_string(),
        ConfigValue::Float(angle),
    );
    fields.insert(
        "support_speed".to_string(),
        ConfigValue::Float(speed),
    );
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

/// Test 1: support_enabled=false produces no output.
#[test]
fn support_disabled_no_output() {
    let config = make_config(false, 0.2, 0.0, 50.0, 0.4);
    let module = TraditionalSupport::on_print_start(&config).unwrap();

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

/// Test 2: Enabled support with a 10mm square region produces paths.
#[test]
fn single_region_generates_support() {
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TraditionalSupport::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    let paths = output.support_paths();
    // spacing=2mm over 10mm range -> expect ~4 lines
    assert!(
        paths.len() >= 3 && paths.len() <= 5,
        "expected 3-5 support lines, got {}",
        paths.len()
    );
}

/// Test 3: All output paths have role SupportMaterial.
#[test]
fn extrusion_role_is_support_material() {
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TraditionalSupport::on_print_start(&config).unwrap();

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
            "all support paths must be SupportMaterial"
        );
    }
}

/// Test 4: Speed factor derived from config support_speed / BASE_SPEED.
#[test]
fn speed_factor_from_config() {
    let config = make_config(true, 0.2, 0.0, 80.0, 0.4);
    let module = TraditionalSupport::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let paint = PaintRegionLayerView::new(0);
    let mut output = SupportOutputBuilder::new();

    module
        .run_support(0, &[region], &paint, &mut output, &config)
        .unwrap();

    assert!(!output.support_paths().is_empty());
    for path in output.support_paths() {
        assert!(
            (path.speed_factor - 1.6).abs() < 0.001,
            "speed_factor should be 80/50=1.6, got {}",
            path.speed_factor
        );
    }
}

/// Test 5: Higher density produces more lines.
#[test]
fn density_affects_line_count() {
    let config_low = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let config_high = make_config(true, 0.5, 0.0, 50.0, 0.4);

    let module_low = TraditionalSupport::on_print_start(&config_low).unwrap();
    let module_high = TraditionalSupport::on_print_start(&config_high).unwrap();

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
        "higher density should produce more lines: low={}, high={}",
        count_low,
        count_high
    );
}

/// Test 6: Alternating angle — layer 0 vs layer 1 rotated by 90 degrees.
#[test]
fn alternating_angle() {
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TraditionalSupport::on_print_start(&config).unwrap();

    let region0 = make_square_region(10.0, 0.3);
    let region1 = make_square_region(10.0, 0.5);

    let paint = PaintRegionLayerView::new(0);
    let mut output0 = SupportOutputBuilder::new();
    let mut output1 = SupportOutputBuilder::new();

    module
        .run_support(0, &[region0], &paint, &mut output0, &config)
        .unwrap();
    module
        .run_support(1, &[region1], &paint, &mut output1, &config)
        .unwrap();

    let paths0 = output0.support_paths();
    let paths1 = output1.support_paths();

    assert!(!paths0.is_empty(), "layer 0 should have lines");
    assert!(!paths1.is_empty(), "layer 1 should have lines");

    // Layer 0 (angle=0): horizontal lines (dy ~ 0)
    let avg_dy_0: f32 = paths0
        .iter()
        .map(|p| (p.points[0].y - p.points[1].y).abs())
        .sum::<f32>()
        / paths0.len() as f32;

    // Layer 1 (angle=90): vertical lines (dx ~ 0)
    let avg_dx_1: f32 = paths1
        .iter()
        .map(|p| (p.points[0].x - p.points[1].x).abs())
        .sum::<f32>()
        / paths1.len() as f32;

    assert!(
        avg_dy_0 < 0.01,
        "layer 0 lines should be horizontal, avg dy={}",
        avg_dy_0
    );
    assert!(
        avg_dx_1 < 0.01,
        "layer 1 lines should be vertical, avg dx={}",
        avg_dx_1
    );
}

/// Test 7: Empty regions produce no output.
#[test]
fn empty_regions_no_output() {
    let config = make_config(true, 0.2, 0.0, 50.0, 0.4);
    let module = TraditionalSupport::on_print_start(&config).unwrap();

    // Region with empty polygons
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

/// Test 8: Zero density produces no output.
#[test]
fn zero_density_no_output() {
    let config = make_config(true, 0.0, 0.0, 50.0, 0.4);
    let module = TraditionalSupport::on_print_start(&config).unwrap();

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
