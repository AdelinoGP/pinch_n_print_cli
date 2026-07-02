//! AC-6 (packet 108, T-077): `extra_perimeters_on_overhangs` per-region overhang
//! wall bonus.
//!
//! When `extra_perimeters_on_overhangs = true` and `region.overhang_areas()` is
//! non-empty, exactly ONE extra perimeter is added inside the overhang
//! polygons; wall count outside those polygons stays at the base count N.
//! With the config false (or overhang_areas empty), wall count is N
//! everywhere regardless of overhang membership.
//!
//! Non-planar branch precedence and the plain `extra_perimeters` bonus are
//! covered by `nonplanar_shell_emission_tdd` and `extra_perimeters_config_tdd`
//! respectively; this test only exercises the overhang-extra branch.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{LoopType, WallLoop};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Two disjoint 10mm squares in one region: a "left" square fully inside the
/// overhang footprint, and a "right" square fully outside it. Keeping them
/// disjoint avoids any ambiguity in the intersection/difference split.
fn make_region(overhang_areas: Vec<slicer_ir::ExPolygon>) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(1)
        .z(0.2)
        .add_polygon(square_polygon(-10.0, 0.0, 10.0)) // x in [-15,-5]
        .add_polygon(square_polygon(10.0, 0.0, 10.0)) // x in [5,15]
        .overhang_areas(overhang_areas)
        .build()
}

fn run_with_config(
    config: slicer_ir::ConfigView,
    overhang_areas: Vec<slicer_ir::ExPolygon>,
) -> Vec<WallLoop> {
    let module = ClassicPerimeters::on_print_start(&config).unwrap();
    let regions = vec![make_region(overhang_areas)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();
    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();
    output
        .wall_loops()
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer || w.loop_type == LoopType::Inner)
        .cloned()
        .collect()
}

/// Mean X (mm) of a wall loop's path points, used to classify a loop as
/// belonging to the "left" (overhang, x<0) or "right" (non-overhang, x>0)
/// square.
fn mean_x_mm(w: &WallLoop) -> f32 {
    let pts = &w.path.points;
    let sum: f32 = pts.iter().map(|p| p.x).sum();
    sum / pts.len() as f32
}

/// AC-6 positive case: base wall_count=2, extra_perimeters_on_overhangs=true,
/// overhang_areas covers the left square → left gets 3 walls, right stays 2.
#[test]
fn overhang_extra_adds_one_wall_inside_overhang_only() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .bool("extra_perimeters_on_overhangs", true)
        .build();
    // Overhang footprint covers (and slightly overshoots) the left square only.
    let overhang = rect_polygon(-10.0, 0.0, 12.0, 12.0);

    let walls = run_with_config(config, vec![overhang]);

    let left_count = walls.iter().filter(|w| mean_x_mm(w) < 0.0).count();
    let right_count = walls.iter().filter(|w| mean_x_mm(w) > 0.0).count();

    assert_eq!(
        left_count, 3,
        "overhang-covered square should get wall_count+1=3 walls; got {left_count}"
    );
    assert_eq!(
        right_count, 2,
        "non-overhang square should stay at base wall_count=2; got {right_count}"
    );
}

/// AC-6 negative case: same overhang footprint, but extra_perimeters_on_overhangs
/// is false → both squares emit exactly the base wall_count everywhere.
#[test]
fn overhang_extra_disabled_leaves_wall_count_uniform() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .bool("extra_perimeters_on_overhangs", false)
        .build();
    let overhang = rect_polygon(-10.0, 0.0, 12.0, 12.0);

    let walls = run_with_config(config, vec![overhang]);

    let left_count = walls.iter().filter(|w| mean_x_mm(w) < 0.0).count();
    let right_count = walls.iter().filter(|w| mean_x_mm(w) > 0.0).count();

    assert_eq!(left_count, 2, "config disabled: left square stays at N=2");
    assert_eq!(right_count, 2, "config disabled: right square stays at N=2");
}

/// Empty-input path: extra_perimeters_on_overhangs=true but overhang_areas is
/// empty → no extras anywhere, no panic.
#[test]
fn overhang_extra_with_empty_overhang_areas_is_noop() {
    let config = ConfigViewBuilder::new()
        .int("wall_count", 2)
        .bool("extra_perimeters_on_overhangs", true)
        .build();

    let walls = run_with_config(config, Vec::new());

    let left_count = walls.iter().filter(|w| mean_x_mm(w) < 0.0).count();
    let right_count = walls.iter().filter(|w| mean_x_mm(w) > 0.0).count();

    assert_eq!(
        left_count, 2,
        "empty overhang_areas: left square stays at N=2"
    );
    assert_eq!(
        right_count, 2,
        "empty overhang_areas: right square stays at N=2"
    );
}
