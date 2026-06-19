// only_one_wall_top_tdd.rs — AC-4 and AC-N2 TDD tests.
//
// AC-4: When a SliceRegionView has top_shell_index == Some(0) and config
// only_one_wall_top = true, run_perimeters must emit exactly 1 outer wall
// (loop_type = Outer) and zero inner walls regardless of wall_count.
// With only_one_wall_top = false the configured wall_count (4) is respected.
//
// AC-N2 (non_top_layer_case): When top_shell_index == None and
// only_one_wall_top = true, wall_count must remain at the configured base (4).

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ConfigView, ExPolygon, LoopType, Point2, Polygon};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a ConfigView with wall_count=4, line_width=0.4, only_one_wall_top=<flag>.
fn config_4_walls(only_one_wall_top: bool) -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 4)
        .float("line_width", 0.4)
        .bool("only_one_wall_top", only_one_wall_top)
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

/// Build a region with the given top_shell_index.
fn make_region(top_shell_index: Option<u8>) -> SliceRegionView {
    let mut region = SliceRegionView::default();
    region.set_object_id("obj-0".to_string());
    region.set_region_id(0);
    region.set_polygons(vec![outer_square()]);
    region.set_infill_areas(vec![]);
    region.set_effective_layer_height(0.2);
    region.set_z(0.2);
    region.set_has_nonplanar(false);
    region.set_bridge_areas(vec![]);
    region.set_top_shell_index(top_shell_index);
    region
}

/// Build the right half (5mm × 10mm) of the 10mm square as top_solid_fill.
/// Coordinates: x from 50_000 to 100_000, y from 0 to 100_000.
fn right_half_fill() -> ExPolygon {
    let half = 50_000_i64;
    let full = 100_000_i64;
    ExPolygon {
        contour: slicer_ir::Polygon {
            points: vec![
                Point2 { x: half, y: 0 },
                Point2 { x: full, y: 0 },
                Point2 { x: full, y: full },
                Point2 { x: half, y: full },
            ],
        },
        holes: Vec::new(),
    }
}

/// Build a region with top_shell_index = Some(1) and top_solid_fill covering
/// the right half of the 10mm square.
fn make_sub_top_region() -> SliceRegionView {
    let mut region = make_region(Some(1));
    region.set_top_solid_fill(vec![right_half_fill()]);
    region
}

/// AC-4: top_shell_index == Some(0), only_one_wall_top = true → exactly 1 Outer wall, 0 Inner.
#[test]
fn top_layer_one_wall_when_flag_enabled() {
    let config = config_4_walls(true);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_region(Some(0));

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    let outer_count = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer)
        .count();
    let inner_count = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Inner)
        .count();

    assert_eq!(
        outer_count, 1,
        "AC-4: top surface with only_one_wall_top=true must emit exactly 1 outer wall; got {outer_count}"
    );
    assert_eq!(
        inner_count, 0,
        "AC-4: top surface with only_one_wall_top=true must emit 0 inner walls; got {inner_count}"
    );
}

/// AC-4 negative: top_shell_index == Some(0), only_one_wall_top = false → 4 walls emitted.
#[test]
fn top_layer_respects_wall_count_when_flag_disabled() {
    let config = config_4_walls(false);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_region(Some(0));

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    assert_eq!(
        walls.len(),
        4,
        "AC-4 negative: only_one_wall_top=false must not clamp; expected 4 walls, got {}",
        walls.len()
    );
}

/// AC-N2: top_shell_index == None, only_one_wall_top = true → wall_count unchanged at 4.
#[test]
fn non_top_layer_case() {
    let config = config_4_walls(true);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_region(None);

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    assert_eq!(
        walls.len(),
        4,
        "AC-N2: non-top region (top_shell_index=None) must not be clamped; expected 4 walls, got {}",
        walls.len()
    );
}

/// AC-4 sub_top_layer_carve_case: top_shell_index == Some(1), top_solid_fill covers right half,
/// only_one_wall_top = true, base wall_count = 4.
///
/// Expected: the split produces TWO outer walls (one per portion). The top portion
/// (right half) emits exactly 1 outer wall and 0 inner walls. The non-top portion
/// (left half) emits 1 outer wall and 3 inner walls (4-wall count for that portion).
/// Total: outer_count == 2 and inner_count == 3.
#[test]
fn sub_top_layer_carve_case() {
    let config = config_4_walls(true);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_sub_top_region();

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    let outer_count = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer)
        .count();
    let inner_count = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Inner)
        .count();

    assert_eq!(
        outer_count, 2,
        "sub-top carve: expected 2 outer walls (one per portion); got {outer_count}"
    );
    assert_eq!(
        inner_count, 3,
        "sub-top carve: non-top portion must emit 3 inner walls (wall_count=4, 1 outer + 3 inner); got {inner_count}"
    );
}

/// AC-4 sub_top_layer_noop_when_flag_disabled: same Some(1) + top_solid_fill fixture
/// but only_one_wall_top = false → full 4 walls over the WHOLE region, no carve.
#[test]
fn sub_top_layer_noop_when_flag_disabled() {
    let config = config_4_walls(false);
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    let region = make_sub_top_region();

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .expect("run_perimeters must not panic");

    let walls = output.wall_loops();
    assert_eq!(
        walls.len(),
        4,
        "sub-top noop: only_one_wall_top=false must produce 4 walls total (no carve); got {}",
        walls.len()
    );
}
