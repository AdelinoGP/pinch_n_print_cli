#![allow(missing_docs)]

use std::collections::HashMap;

use slicer_ir::{ConfigView, ExtrusionRole};
use slicer_sdk::builders::InfillOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::LayerModule;
use slicer_sdk::views::SliceRegionView;

use rectilinear_infill::RectilinearInfill;

fn empty_paint_view() -> slicer_sdk::traits::PaintRegionLayerView {
    slicer_sdk::traits::PaintRegionLayerView::new(0)
}

#[rustfmt::skip]
fn make_config(density: f64, angle: f64, speed: f64, line_width: f64) -> ConfigView {
    ConfigViewBuilder::new().float("infill_density", density).float("infill_angle", angle).float("infill_speed", speed).float("line_width", line_width).build()
}

// Post-host-partition fixture: `sparse_infill_area` carries the square so
// SparseInfill emission has its canonical polygon (see
// `crates/slicer-runtime/src/region_partition.rs`).
#[rustfmt::skip]
fn make_square_region(size_mm: f32, z: f32) -> SliceRegionView {
    let sq = square_polygon(0.0, 0.0, size_mm);
    let mut region = SliceRegionViewBuilder::new().object_id("obj1").region_id(1).add_polygon(sq.clone()).add_infill_area(sq.clone()).sparse_infill_area(vec![sq]).effective_layer_height(0.2).z(z).has_nonplanar(false).build();
    // Rectilinear manifest declares all four fill claims.
    region.set_held_claims(vec![
        "claim:top-fill".into(),
        "claim:bottom-fill".into(),
        "claim:bridge-fill".into(),
        "claim:sparse-fill".into(),
    ]);
    region
}

/// Test 1: 10mm square, density=0.2, line_width=0.4, angle=0.
/// Spacing = 0.4/0.2 = 2mm. Square is 10mm, expect ~4 lines.
/// All lines should be horizontal (start.y == end.y within tolerance).
#[test]
fn single_square_sparse_fill() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();
    // spacing=2mm over 10mm range -> expect 6 lines
    assert!(
        paths.len() >= 5 && paths.len() <= 7,
        "expected 5-7 lines, got {}",
        paths.len()
    );

    // All lines should be horizontal: start.y == end.y
    for path in paths {
        assert_eq!(path.points.len(), 2);
        let dy = (path.points[0].y - path.points[1].y).abs();
        assert!(dy < 0.01, "expected horizontal line, got dy={}", dy);
    }
}

/// Test 2: Higher density produces more lines.
#[test]
fn density_affects_line_count() {
    let config_low = make_config(0.2, 0.0, 50.0, 0.4);
    let config_high = make_config(0.5, 0.0, 50.0, 0.4);

    let module_low = RectilinearInfill::on_print_start(&config_low).unwrap();
    let module_high = RectilinearInfill::on_print_start(&config_high).unwrap();

    let region_low = make_square_region(10.0, 0.3);
    let region_high = make_square_region(10.0, 0.3);

    let mut output_low = InfillOutputBuilder::new();
    let mut output_high = InfillOutputBuilder::new();

    module_low
        .run_infill(
            0,
            &[region_low],
            &empty_paint_view(),
            &mut output_low,
            &config_low,
        )
        .unwrap();
    module_high
        .run_infill(
            0,
            &[region_high],
            &empty_paint_view(),
            &mut output_high,
            &config_high,
        )
        .unwrap();

    let count_low = output_low.sparse_paths().len();
    let count_high = output_high.sparse_paths().len();

    assert!(
        count_high > count_low,
        "higher density should produce more lines: low={}, high={}",
        count_low,
        count_high
    );
}

/// Test 3: 45-degree angle produces diagonal lines.
#[test]
fn angle_rotation_45() {
    let config = make_config(0.2, 45.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    let paths = output.sparse_paths();
    assert!(!paths.is_empty(), "should produce some infill lines");

    // For 45-degree lines, start.x != end.x AND start.y != end.y
    let diagonal_count = paths
        .iter()
        .filter(|p| {
            let dx = (p.points[0].x - p.points[1].x).abs();
            let dy = (p.points[0].y - p.points[1].y).abs();
            dx > 0.1 && dy > 0.1
        })
        .count();

    assert!(
        diagonal_count > paths.len() / 2,
        "most lines should be diagonal at 45 degrees, got {}/{}",
        diagonal_count,
        paths.len()
    );
}

/// Test 4: Layer alternation — layer 0 vs layer 1 should have different orientations.
#[test]
fn layer_alternation() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    let region0 = make_square_region(10.0, 0.3);
    let region1 = make_square_region(10.0, 0.5);

    let mut output0 = InfillOutputBuilder::new();
    let mut output1 = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region0], &empty_paint_view(), &mut output0, &config)
        .unwrap();
    module
        .run_infill(1, &[region1], &empty_paint_view(), &mut output1, &config)
        .unwrap();

    let paths0 = output0.sparse_paths();
    let paths1 = output1.sparse_paths();

    assert!(!paths0.is_empty(), "layer 0 should have lines");
    assert!(!paths1.is_empty(), "layer 1 should have lines");

    // Layer 0 (angle=0): horizontal lines (dy ~ 0)
    // Layer 1 (angle=90): vertical lines (dx ~ 0)
    let avg_dy_0: f32 = paths0
        .iter()
        .map(|p| (p.points[0].y - p.points[1].y).abs())
        .sum::<f32>()
        / paths0.len() as f32;

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

/// Test 5: Empty infill areas produce no output.
#[test]
fn empty_infill_areas() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    // Region with empty infill_areas
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
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    assert_eq!(
        output.sparse_paths().len(),
        0,
        "empty infill areas should produce no paths"
    );
}

/// Test 6: Zero density produces no output.
#[test]
fn zero_density_no_output() {
    let config = make_config(0.0, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    assert_eq!(
        output.sparse_paths().len(),
        0,
        "zero density should produce no paths"
    );
}

/// Test 7: All output paths have role SparseInfill.
#[test]
fn extrusion_role_is_sparse() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    assert!(!output.sparse_paths().is_empty());
    for path in output.sparse_paths() {
        assert_eq!(
            path.role,
            ExtrusionRole::SparseInfill,
            "all paths must be SparseInfill"
        );
    }
}

/// Test 8: Speed factor derived from config infill_speed / BASE_SPEED.
#[test]
fn speed_factor_from_config() {
    let config = make_config(0.2, 0.0, 100.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    let region = make_square_region(10.0, 0.3);
    let mut output = InfillOutputBuilder::new();

    module
        .run_infill(0, &[region], &empty_paint_view(), &mut output, &config)
        .unwrap();

    assert!(!output.sparse_paths().is_empty());
    for path in output.sparse_paths() {
        assert!(
            (path.speed_factor - 2.0).abs() < 0.001,
            "speed_factor should be 100/50=2.0, got {}",
            path.speed_factor
        );
    }
}

/// Test 9: per-region `infill_density` override (packet 131 / TASK-256) is
/// read through `slicer_sdk::config_resolution` and overrides the
/// module-global default set in `on_print_start`.
///
/// Module-global density 0.2 (line_width=0.4 → spacing 2mm → ~5 lines on a
/// 10mm square). Per-region density 0.4 doubles the density → spacing 1mm →
/// ~10 lines on the same square. The line count is the direct observable.
#[test]
fn per_region_density_overrides_module_global() {
    let config = make_config(0.2, 0.0, 50.0, 0.4);
    let module = RectilinearInfill::on_print_start(&config).unwrap();

    // Region A: no per-region config. Module-global density 0.2 → spacing 2mm.
    let region_a = make_square_region(10.0, 0.3);
    let mut output_a = InfillOutputBuilder::new();
    module
        .run_infill(
            0,
            std::slice::from_ref(&region_a),
            &empty_paint_view(),
            &mut output_a,
            &config,
        )
        .unwrap();
    let count_a = output_a.sparse_paths().len();
    assert!(
        (5..=7).contains(&count_a),
        "density 0.2, line_width 0.4 → spacing 2mm → expect 5-7 lines on a 10mm square, got {}",
        count_a
    );

    // Region B: per-region infill_density = 0.4. Spacing = 0.4/0.4 = 1mm.
    let mut region_b = make_square_region(10.0, 0.3);
    let mut fields = HashMap::new();
    fields.insert("infill_density".into(), slicer_ir::ConfigValue::Float(0.4));
    region_b.set_config(ConfigView::from_map(fields));

    let mut output_b = InfillOutputBuilder::new();
    module
        .run_infill(
            0,
            std::slice::from_ref(&region_b),
            &empty_paint_view(),
            &mut output_b,
            &config,
        )
        .unwrap();
    let count_b = output_b.sparse_paths().len();
    assert!(
        (10..=12).contains(&count_b),
        "per-region density 0.4 → spacing 1mm → expect 10-12 lines on a 10mm square, got {}",
        count_b
    );

    // The per-region override produces ~2× the line count of the
    // module-global default.
    let ratio = count_b as f64 / count_a as f64;
    assert!(
        ratio > 1.5 && ratio < 2.5,
        "per-region density 0.4 should produce ~2× the line count of density 0.2; \
         got count_a={} count_b={} ratio={}",
        count_a,
        count_b,
        ratio
    );
}
