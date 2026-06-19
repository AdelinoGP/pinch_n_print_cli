// per_layer_config_override_tdd.rs — LayerOverrides per-layer config plumbing
// (wall_count, outer_wall_speed, inner_wall_speed read per invocation)

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
fn per_layer_config_wall_count_override() {
    let base_module = ClassicPerimeters::on_print_start(&ConfigView::from_map(HashMap::new()))
        .expect("on_print_start should succeed");

    // Layer 0: base wall_count = 2 (default)
    let region0 = square_region(0.2);
    let mut output0 = PerimeterOutputBuilder::new();
    let config0 = config_with_wall_count(2);
    base_module
        .run_perimeters(
            0,
            &[region0],
            &PaintRegionLayerView::new(0),
            &mut output0,
            &config0,
        )
        .expect("run_perimeters for layer 0 should succeed");
    assert_eq!(
        output0.wall_loops().len(),
        2,
        "layer 0 with wall_count=2 should emit 2 walls"
    );

    // Layer 5: wall_count = 5 (override)
    let region5 = square_region(1.0);
    let mut output5 = PerimeterOutputBuilder::new();
    let config5 = config_with_wall_count(5);
    base_module
        .run_perimeters(
            5,
            &[region5],
            &PaintRegionLayerView::new(0),
            &mut output5,
            &config5,
        )
        .expect("run_perimeters for layer 5 should succeed");
    assert_eq!(
        output5.wall_loops().len(),
        5,
        "layer 5 with wall_count=5 should emit 5 walls"
    );
}

#[test]
fn per_layer_config_wall_count_zero_emits_only_infill() {
    let base_module = ClassicPerimeters::on_print_start(&ConfigView::from_map(HashMap::new()))
        .expect("on_print_start should succeed");

    let region = square_region(0.2);
    let mut output = PerimeterOutputBuilder::new();
    let config = config_with_wall_count(0);
    base_module
        .run_perimeters(
            0,
            &[region],
            &PaintRegionLayerView::new(0),
            &mut output,
            &config,
        )
        .expect("run_perimeters with wall_count=0 should succeed");

    assert!(
        output.wall_loops().is_empty(),
        "wall_count=0 should emit no walls"
    );
    assert!(
        !output.infill_areas().is_empty(),
        "wall_count=0 should emit infill areas"
    );
}

#[test]
fn per_layer_config_missing_wall_count_falls_back_to_on_print_start() {
    let config = ConfigView::from_map([("wall_count".to_string(), ConfigValue::Int(3))].into());
    let module = ClassicPerimeters::on_print_start(&config)
        .expect("on_print_start with wall_count=3 should succeed");

    let region = square_region(0.2);
    let mut output = PerimeterOutputBuilder::new();
    // No wall_count in per-layer config → falls back to on_print_start value (3)
    let empty_config = ConfigView::from_map(HashMap::new());
    module
        .run_perimeters(
            0,
            &[region],
            &PaintRegionLayerView::new(0),
            &mut output,
            &empty_config,
        )
        .expect("run_perimeters with empty per-layer config should succeed");

    assert_eq!(
        output.wall_loops().len(),
        3,
        "fallback to on_print_start wall_count=3 should emit 3 walls"
    );
}

#[test]
fn per_layer_config_speed_override() {
    let module = ClassicPerimeters::on_print_start(&ConfigView::from_map(HashMap::new()))
        .expect("on_print_start should succeed");

    let region = square_region(0.2);
    let mut output = PerimeterOutputBuilder::new();
    let config = ConfigView::from_map(
        [
            ("wall_count".to_string(), ConfigValue::Int(2)),
            ("outer_wall_speed".to_string(), ConfigValue::Float(30.0)),
            ("inner_wall_speed".to_string(), ConfigValue::Float(40.0)),
        ]
        .into(),
    );
    module
        .run_perimeters(
            0,
            &[region],
            &PaintRegionLayerView::new(0),
            &mut output,
            &config,
        )
        .expect("run_perimeters with speed overrides should succeed");

    let walls = output.wall_loops();
    assert_eq!(walls.len(), 2, "should emit 2 walls");
    // Outer wall (perimeter_index 0) should have speed_factor = 30.0/50.0 = 0.6
    let outer_wall = &walls[0];
    assert_eq!(outer_wall.perimeter_index, 0);
    assert!(
        (outer_wall.path.speed_factor - 0.6).abs() < 0.001,
        "outer wall speed_factor should be 0.6, got {}",
        outer_wall.path.speed_factor
    );
    // Inner wall (perimeter_index 1) should have speed_factor = 40.0/50.0 = 0.8
    let inner_wall = &walls[1];
    assert_eq!(inner_wall.perimeter_index, 1);
    assert!(
        (inner_wall.path.speed_factor - 0.8).abs() < 0.001,
        "inner wall speed_factor should be 0.8, got {}",
        inner_wall.path.speed_factor
    );
}
