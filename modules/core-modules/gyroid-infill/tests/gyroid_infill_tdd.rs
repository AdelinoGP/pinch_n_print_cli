//! TDD tests for the gyroid-infill module.

use std::collections::HashMap;

use slicer_ir::{ConfigValue, ConfigView, ExPolygon, ExtrusionRole, Point2, Polygon};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

use gyroid_infill::GyroidInfill;

fn make_config(density: f64, angle: f64, speed: f64, line_width: f64) -> ConfigView {
    let mut fields = HashMap::new();
    fields.insert("infill_density".to_string(), ConfigValue::Float(density));
    fields.insert("infill_angle".to_string(), ConfigValue::Float(angle));
    fields.insert("infill_speed".to_string(), ConfigValue::Float(speed));
    fields.insert("line_width".to_string(), ConfigValue::Float(line_width));
    ConfigView::from_map(fields)
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

/// Test 1: Default config values when no fields provided.
#[test]
fn on_print_start_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = GyroidInfill::on_print_start(&config).unwrap();
    assert!((module.density() - 0.2).abs() < 0.001);
    assert!((module.line_width() - 0.4).abs() < 0.001);
}

/// Test 2: Custom config values are read correctly.
#[test]
fn on_print_start_custom() {
    let config = make_config(0.3, 30.0, 80.0, 0.5);
    let module = GyroidInfill::on_print_start(&config).unwrap();
    assert!((module.density() - 0.3).abs() < 0.001);
    assert!((module.line_width() - 0.5).abs() < 0.001);
}

/// Test 3: 10mm square at density=0.2 produces non-empty sparse paths.
#[test]
fn square_region_produces_paths() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(
        !output.sparse_paths().is_empty(),
        "gyroid should produce sparse infill paths for a 10mm square"
    );
}

/// Test 4: All paths have SparseInfill extrusion role.
#[test]
fn paths_have_sparse_infill_role() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(!output.sparse_paths().is_empty());
    for path in output.sparse_paths() {
        assert_eq!(
            path.role,
            ExtrusionRole::SparseInfill,
            "all gyroid paths must have SparseInfill role"
        );
    }
}

/// Test 5: Zero density produces no paths.
#[test]
fn zero_density_no_paths() {
    let config = make_config(0.0, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert_eq!(
        output.sparse_paths().len(),
        0,
        "zero density should produce no paths"
    );
}

/// Test 6: Empty regions produce no output.
#[test]
fn empty_regions_no_output() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = SliceRegionView::new(
        "obj1".to_string(),
        1,
        vec![],
        vec![], // empty infill_areas
        0.2,
        0.3,
        false,
    );

    let mut output = InfillOutputBuilder::new();
    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert_eq!(
        output.sparse_paths().len(),
        0,
        "empty infill areas should produce no paths"
    );
}

/// Test 7: All output points have the correct z value.
#[test]
fn paths_at_correct_z() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let z = 1.5;
    let region = make_square_region(10.0, z);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(!output.sparse_paths().is_empty());
    for path in output.sparse_paths() {
        for pt in &path.points {
            assert!(
                (pt.z - z).abs() < 0.001,
                "all points should have z={}, got z={}",
                z,
                pt.z
            );
        }
    }
}

/// Test 8: Different z values produce different path geometries.
#[test]
fn wave_pattern_varies_by_layer() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region1 = make_square_region(10.0, 0.3);
    let region2 = make_square_region(10.0, 1.5);

    let mut output1 = InfillOutputBuilder::new();
    let mut output2 = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region1], &mut output1, &config)
        .unwrap();
    module
        .run_infill(0, &[region2], &mut output2, &config)
        .unwrap();

    let paths1 = output1.sparse_paths();
    let paths2 = output2.sparse_paths();

    assert!(!paths1.is_empty());
    assert!(!paths2.is_empty());

    // Different z should produce different wave shapes.
    // Compare first path's first point y coordinates — they should differ.
    let y1 = paths1[0].points[0].y;
    let y2 = paths2[0].points[0].y;
    let differs = (y1 - y2).abs() > 0.01 || paths1.len() != paths2.len();
    assert!(
        differs,
        "different z heights should produce different wave patterns"
    );
}

/// Test 9: Higher density produces more/denser paths than lower density.
#[test]
fn density_affects_spacing() {
    let config_low = make_config(0.1, 0.0, 50.0, 0.4);
    let config_high = make_config(0.5, 0.0, 50.0, 0.4);

    let module_low = GyroidInfill::on_print_start(&config_low).unwrap();
    let module_high = GyroidInfill::on_print_start(&config_high).unwrap();

    let region_low = make_square_region(10.0, 0.3);
    let region_high = make_square_region(10.0, 0.3);

    let mut output_low = InfillOutputBuilder::new();
    let mut output_high = InfillOutputBuilder::new();

    module_low
        .run_infill(0, &[region_low], &mut output_low, &config_low)
        .unwrap();
    module_high
        .run_infill(0, &[region_high], &mut output_high, &config_high)
        .unwrap();

    let count_low = output_low.sparse_paths().len();
    let count_high = output_high.sparse_paths().len();

    assert!(
        count_high > count_low,
        "higher density should produce more paths: low={}, high={}",
        count_low,
        count_high
    );
}

/// Test 10: All point widths match configured line_width.
#[test]
fn width_matches_config() {
    let lw = 0.6;
    let config = make_config(0.2, 0.0, 50.0, lw);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(!output.sparse_paths().is_empty());
    for path in output.sparse_paths() {
        for pt in &path.points {
            assert!(
                (pt.width - lw as f32).abs() < 0.001,
                "all point widths should be {}, got {}",
                lw,
                pt.width
            );
        }
    }
}

/// Test 11: No NaN values in output points even at extreme z values.
#[test]
fn asin_nan_protection() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = GyroidInfill::on_print_start(&config).unwrap();

    // Test at z values where sin(z) or cos(z) are at extremes
    for z in [
        0.0_f32,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        100.0,
        0.001,
    ] {
        let region = make_square_region(10.0, z);
        let mut output = InfillOutputBuilder::new();

        module
            .run_infill(0, &[region], &mut output, &config)
            .unwrap();

        for path in output.sparse_paths() {
            for pt in &path.points {
                assert!(!pt.x.is_nan(), "x is NaN at z={}", z);
                assert!(!pt.y.is_nan(), "y is NaN at z={}", z);
                assert!(!pt.z.is_nan(), "z is NaN at z={}", z);
                assert!(!pt.width.is_nan(), "width is NaN at z={}", z);
            }
        }
    }
}
