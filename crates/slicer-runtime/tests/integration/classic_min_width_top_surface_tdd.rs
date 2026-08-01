//! D-152 / packet 184: `min_width_top_surface` must gate the
//! `only_one_wall_top` single-wall collapse in `classic-perimeters`.
//!
//! Canonical `PerimeterGenerator::split_top_surfaces` resolves
//! `min_width_top_surface` via `get_abs_value` against the perimeter width and
//! uses it as an EROSION threshold on the top area: a top sub-area narrower
//! than the threshold is dropped from the top portion and therefore keeps the
//! full configured wall count instead of collapsing to a single wall.
//!
//! `arachne-perimeters` already implements this (`emit_only_one_wall_top_second_pass`);
//! `classic-perimeters` read the key and discarded it, so EVERY top sub-area
//! collapsed to one wall. This test pins the gated behaviour:
//!
//! - a NARROW top sub-area (bbox min extent < `min_width_top_surface`) keeps
//!   all `wall_count` loops,
//! - a WIDE top sub-area (bbox min extent >= threshold) collapses to 1 loop.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::WallLoop;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};

/// Narrow island: 4 mm x 20 mm, centred well to the LEFT of the origin.
const NARROW_CX: f32 = -20.0;
const NARROW_W: f32 = 4.0;
const NARROW_H: f32 = 20.0;
/// Wide island: 20 mm square, centred well to the RIGHT of the origin.
const WIDE_CX: f32 = 20.0;
const WIDE_SIDE: f32 = 20.0;
/// Threshold sits strictly between the two islands' minimum bbox extents.
const MIN_WIDTH_TOP: f64 = 6.0;
const WALL_COUNT: i64 = 3;

fn islands() -> Vec<slicer_ir::ExPolygon> {
    vec![
        rect_polygon(NARROW_CX, 0.0, NARROW_W, NARROW_H),
        square_polygon(WIDE_CX, 0.0, WIDE_SIDE),
    ]
}

/// Loops whose vertices all sit left of the origin belong to the narrow island.
fn split_loops_by_island(walls: &[WallLoop]) -> (usize, usize) {
    let mut narrow = 0;
    let mut wide = 0;
    for w in walls {
        let max_x = w.path.points.iter().map(|p| p.x).fold(f32::MIN, f32::max);
        if max_x < 0.0 {
            narrow += 1;
        } else {
            wide += 1;
        }
    }
    (narrow, wide)
}

/// Site B: the whole-region collapse (`top_shell_index == Some(0)`) must
/// consult the same `min_width_top_surface` gate as the partial-top split
/// branch. A region narrower than the threshold keeps the full wall count.
#[test]
fn min_width_top_surface_gates_full_region_collapse() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", WALL_COUNT)
        .float("outer_wall_line_width", 0.5)
        .float("inner_wall_line_width", 0.4)
        .bool("only_one_wall_top", true)
        .float("min_width_top_surface", MIN_WIDTH_TOP)
        .build();

    let narrow = rect_polygon(NARROW_CX, 0.0, NARROW_W, NARROW_H);
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(1.0)
        .add_polygon(narrow.clone())
        .top_shell_index(Some(0))
        .top_solid_fill(vec![narrow])
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let paint = PaintRegionLayerView::new(5);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops().len();
    assert_eq!(
        walls, WALL_COUNT as usize,
        "a fully-top region narrower than min_width_top_surface (bbox min extent \
         {NARROW_W} mm < {MIN_WIDTH_TOP} mm) must keep the full wall count \
         {WALL_COUNT}, observed {walls} wall loop(s) — the whole-region \
         only_one_wall_top collapse is not gated by min_width_top_surface"
    );
}

#[test]
fn min_width_top_surface_gates_only_one_wall_top() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", WALL_COUNT)
        .float("outer_wall_line_width", 0.5)
        .float("inner_wall_line_width", 0.4)
        .bool("only_one_wall_top", true)
        .float("min_width_top_surface", MIN_WIDTH_TOP)
        .build();

    let polys = islands();
    let region = SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(1.0)
        .add_polygon(polys[0].clone())
        .add_polygon(polys[1].clone())
        .top_shell_index(Some(1))
        .top_solid_fill(polys.clone())
        .build();

    let module = ClassicPerimeters::from_config(&config).unwrap();
    let paint = PaintRegionLayerView::new(5);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(5, &[region], &paint, &mut output, &config)
        .unwrap();

    let (narrow_walls, wide_walls) = split_loops_by_island(output.wall_loops());

    assert_eq!(
        narrow_walls, WALL_COUNT as usize,
        "narrow top sub-area (bbox min extent {NARROW_W} mm < min_width_top_surface \
         {MIN_WIDTH_TOP} mm) must keep the full wall count {WALL_COUNT}, observed \
         {narrow_walls} wall loop(s) — the min_width_top_surface gate is not wired"
    );
    assert_eq!(
        wide_walls, 1,
        "wide top sub-area (bbox min extent {WIDE_SIDE} mm >= min_width_top_surface \
         {MIN_WIDTH_TOP} mm) must collapse to a single wall, observed {wide_walls}"
    );
}
