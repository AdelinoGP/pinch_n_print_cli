// only_one_wall_first_layer_tdd.rs — AC-5 TDD tests.
//
// AC-5: When layer_index == 0 and only_one_wall_first_layer = true, run_perimeters
// must clamp wall count to 1 regardless of the configured wall_count.
// At layer_index > 0 the configured wall_count (4) must be respected.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ConfigView, ExPolygon, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a ConfigView with wall_count=4, line_width=0.4, only_one_wall_first_layer=<flag>.
fn config_4_walls(only_one_wall_first_layer: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 4)
        .float("line_width", 0.4)
        .bool("only_one_wall_first_layer", only_one_wall_first_layer)
        .build()
}

/// Build a 10×10 mm square polygon (100_000 units per side, 1 unit = 100 nm).
fn outer_square() -> ExPolygon {
    let size = 100_000_i64;
    ExPolygon {
        contour: Polygon {
            points: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: size, y: 0 },
                Point2 { x: size, y: size },
                Point2 { x: 0, y: size },
            ],
        },
        holes: Vec::new(),
    }
}

/// Build a plain region (no top/bottom shell overrides needed for first-layer tests).
fn make_region() -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-0".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![outer_square()]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    region.set_bridge_areas(vec![]);
    region
}

/// AC-5: layer_index == 0, only_one_wall_first_layer = true → exactly 1 wall.
#[test]
fn first_layer_clamped_to_one_wall() {
    let config = config_4_walls(true);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_region();

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    assert_eq!(
        walls.len(),
        1,
        "AC-5: layer_index=0 with only_one_wall_first_layer=true must emit 1 wall; got {}",
        walls.len()
    );
}

/// AC-5 negative: layer_index == 5, only_one_wall_first_layer = true → 4 walls.
#[test]
fn non_first_layer_respects_wall_count() {
    let config = config_4_walls(true);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_region();

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    assert_eq!(
        walls.len(),
        4,
        "AC-5 negative: layer_index=5 must not be clamped; expected 4 walls, got {}",
        walls.len()
    );
}
