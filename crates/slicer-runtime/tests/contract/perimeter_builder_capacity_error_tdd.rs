// perimeter_builder_capacity_error_tdd.rs — negative-path TDD:
// capacity-rejecting PerimeterOutputBuilder causes run_perimeters to
// return Err(ModuleError), not silently Ok(()).

use std::collections::HashMap;

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ConfigValue, ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn square_region(z: f32) -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_z(z);
    region.set_polygons(vec![ExPolygon {
        contour: Polygon {
            points: vec![
                Point2::from_mm(0.0, 0.0),
                Point2::from_mm(10.0, 0.0),
                Point2::from_mm(10.0, 10.0),
                Point2::from_mm(0.0, 10.0),
            ],
        },
        holes: vec![],
    }]);
    region
}

fn config_with_wall_count(n: i64) -> ConfigView {
    ConfigView::from_map([("wall_count".to_string(), ConfigValue::Int(n))].into())
}

#[test]
fn capacity_zero_wall_loops_rejects_push() {
    let module = ClassicPerimeters::on_print_start(&ConfigView::from_map(HashMap::new()))
        .expect("on_print_start should succeed");

    let region = square_region(0.2);
    let config = config_with_wall_count(2);

    // Builder with max_wall_loops=0 rejects the first push_wall_loop.
    let mut output = PerimeterOutputBuilder::with_capacity(Some(0), None, None, None);
    let result = module.run_perimeters(
        0,
        &[region],
        &PaintRegionLayerView::new(0),
        &mut output,
        &config,
    );

    assert!(
        result.is_err(),
        "capacity-zero builder should cause run_perimeters to return Err"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("builder at capacity"),
        "error message should contain 'builder at capacity', got: {}",
        err.message
    );
}

#[test]
fn capacity_one_wall_loop_accepts_one_rejects_second() {
    let module = ClassicPerimeters::on_print_start(&ConfigView::from_map(HashMap::new()))
        .expect("on_print_start should succeed");

    let region = square_region(0.2);

    // wall_count=1: one wall fits within capacity=1
    let config1 = config_with_wall_count(1);
    let mut output1 = PerimeterOutputBuilder::with_capacity(Some(1), None, None, None);
    let result1 = module.run_perimeters(
        0,
        &[region.clone()],
        &PaintRegionLayerView::new(0),
        &mut output1,
        &config1,
    );
    assert!(
        result1.is_ok(),
        "capacity=1 with wall_count=1 should succeed"
    );

    // wall_count=2: second wall exceeds capacity=1
    let config2 = config_with_wall_count(2);
    let mut output2 = PerimeterOutputBuilder::with_capacity(Some(1), None, None, None);
    let result2 = module.run_perimeters(
        0,
        &[region],
        &PaintRegionLayerView::new(0),
        &mut output2,
        &config2,
    );
    assert!(
        result2.is_err(),
        "capacity=1 with wall_count=2 should reject the second wall"
    );
    let err = result2.unwrap_err();
    assert!(
        err.message.contains("builder at capacity"),
        "error message should contain 'builder at capacity', got: {}",
        err.message
    );
}

#[test]
fn unbounded_builder_never_rejects() {
    let module = ClassicPerimeters::on_print_start(&ConfigView::from_map(HashMap::new()))
        .expect("on_print_start should succeed");

    let region = square_region(0.2);
    let config = config_with_wall_count(5);

    // Default builder (new()) has no capacity limits.
    let mut output = PerimeterOutputBuilder::new();
    let result = module.run_perimeters(
        0,
        &[region],
        &PaintRegionLayerView::new(0),
        &mut output,
        &config,
    );

    assert!(
        result.is_ok(),
        "unbounded builder should never reject, got: {:?}",
        result.err()
    );
    assert_eq!(
        output.wall_loops().len(),
        5,
        "unbounded builder should accept all 5 walls"
    );
}

#[test]
fn capacity_zero_seam_candidates_rejects_push() {
    let module = ClassicPerimeters::on_print_start(&ConfigView::from_map(HashMap::new()))
        .expect("on_print_start should succeed");

    let region = square_region(0.2);
    let config = config_with_wall_count(1);

    // wall_count=1 succeeds, but seam_candidates capacity=0 rejects.
    let mut output = PerimeterOutputBuilder::with_capacity(None, None, Some(0), None);
    let result = module.run_perimeters(
        0,
        &[region],
        &PaintRegionLayerView::new(0),
        &mut output,
        &config,
    );

    assert!(
        result.is_err(),
        "capacity-zero seam_candidates builder should cause run_perimeters to return Err"
    );
    let err = result.unwrap_err();
    assert!(
        err.message.contains("builder at capacity"),
        "error message should contain 'builder at capacity', got: {}",
        err.message
    );
}
