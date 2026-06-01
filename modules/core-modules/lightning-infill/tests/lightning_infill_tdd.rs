//! TDD tests for the lightning-infill module.

use std::collections::HashMap;

use slicer_ir::{ConfigView, ExtrusionRole};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

use lightning_infill::LightningInfill;

fn make_config(density: f64, speed: f64, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new()
        .float("infill_density", density)
        .float("infill_speed", speed)
        .float("line_width", line_width)
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

/// Test 1: Default config values when no fields provided.
#[test]
fn on_print_start_defaults() {
    let config = ConfigView::from_map(HashMap::new());
    let module = LightningInfill::on_print_start(&config).unwrap();
    assert!((module.density() - 0.2).abs() < 0.001);
    assert!((module.line_width() - 0.4).abs() < 0.001);
}

/// Test 2: Custom config values are read correctly.
#[test]
fn on_print_start_custom() {
    let config = make_config(0.3, 80.0, 0.5);
    let module = LightningInfill::on_print_start(&config).unwrap();
    assert!((module.density() - 0.3).abs() < 0.001);
    assert!((module.line_width() - 0.5).abs() < 0.001);
}

/// Test 3: 10mm square at density=0.2 produces non-empty sparse paths.
#[test]
fn square_region_produces_paths() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    assert!(
        !output.sparse_paths().is_empty(),
        "lightning should produce sparse infill paths for a 10mm square"
    );
}

/// Test 4: All paths have SparseInfill extrusion role.
#[test]
fn paths_have_sparse_infill_role() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();

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
            "all lightning paths must have SparseInfill role"
        );
    }
}

/// Test 5: Zero density produces no paths.
#[test]
fn zero_density_no_paths() {
    let config = make_config(0.0, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();

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
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();

    let mut region = SliceRegionView::default();
    region.set_object_id("obj1".to_string());
    region.set_region_id(1);
    region.set_polygons(vec![]);
    region.set_infill_areas(vec![]);
    // empty infill_areas

    region.set_effective_layer_height(0.2);
    region.set_z(0.3);
    region.set_has_nonplanar(false);

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
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();

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

/// Test 8: Branches have non-parallel geometry (unlike rectilinear scan lines).
/// Lightning infill should produce paths that connect to different boundary
/// points, not all parallel.
#[test]
fn branching_pattern_present() {
    let config = make_config(0.3, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();

    let region = make_square_region(20.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();
    assert!(
        paths.len() >= 3,
        "large region at 0.3 density should produce multiple branches, got {}",
        paths.len()
    );

    // Verify branches have different directions (not all parallel)
    // by checking that endpoint directions vary
    let mut angles: Vec<f64> = Vec::new();
    for path in paths {
        if path.points.len() >= 2 {
            let p0 = &path.points[0];
            let p1 = &path.points[path.points.len() - 1];
            let dx = (p1.x - p0.x) as f64;
            let dy = (p1.y - p0.y) as f64;
            let angle = dy.atan2(dx);
            angles.push(angle);
        }
    }

    // At least some branches should point in different directions
    if angles.len() >= 2 {
        let mut has_different = false;
        for i in 1..angles.len() {
            if (angles[i] - angles[0]).abs() > 0.1 {
                has_different = true;
                break;
            }
        }
        assert!(
            has_different,
            "lightning branches should have varying directions, not all parallel"
        );
    }
}

/// Test 9: Higher density produces more/denser paths than lower density.
#[test]
fn density_affects_coverage() {
    let config_low = make_config(0.1, 50.0, 0.4);
    let config_high = make_config(0.5, 50.0, 0.4);

    let module_low = LightningInfill::on_print_start(&config_low).unwrap();
    let module_high = LightningInfill::on_print_start(&config_high).unwrap();

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
    let config = make_config(0.2, 50.0, lw);
    let module = LightningInfill::on_print_start(&config).unwrap();

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

/// Test 11: Branches reach interior points (not just boundary-adjacent).
#[test]
fn interior_first_growth() {
    let config = make_config(0.2, 50.0, 0.4);
    let module = LightningInfill::on_print_start(&config).unwrap();

    let region = make_square_region(20.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();
    assert!(!paths.is_empty());

    // Check that some path start points are in the interior
    // (distance from center > 2mm, i.e., not right at the boundary)
    let mut has_interior_start = false;
    for path in paths {
        if !path.points.is_empty() {
            let p = &path.points[0];
            let dist_from_center = ((p.x * p.x) + (p.y * p.y)).sqrt();
            // Interior means far from boundary (closer to center for a centered square)
            // For a 20mm square centered at origin, boundary is at 10mm
            if dist_from_center < 8.0 {
                has_interior_start = true;
                break;
            }
        }
    }

    assert!(
        has_interior_start,
        "some branches should start from interior points (not just near boundary)"
    );
}
