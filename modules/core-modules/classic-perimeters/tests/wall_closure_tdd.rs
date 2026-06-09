//! Regression: wall loops produced by classic-perimeters carry an explicit
//! closing repeat (OrcaSlicer `ExtrusionPath::is_closed()` convention).
//!
//! Without the closing repeat, the G-code emitter drops the closing edge
//! (every wall on the 20mm cube was missing its bottom side) and fuzzy-skin's
//! per-segment loop never perturbs the closing edge (visible asymmetry
//! between three fuzzy edges and one straight one). See
//! `docs/specs/infill-fill-partition-plan.md` Phase A1 and the OrcaSlicer
//! reference `OrcaSlicerDocumented/src/libslic3r/ExtrusionEntity.hpp:269`.

use classic_perimeters::ClassicPerimeters;
use slicer_ir::{ConfigView, LoopType};
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

fn config_one_outer_wall() -> ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 1)
        .float("line_width", 0.4)
        .build()
}

fn make_square_region(side_mm: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-1")
        .region_id(0)
        .z(0.2)
        .effective_layer_height(0.2)
        .add_polygon(square_polygon(0.0, 0.0, side_mm))
        .build()
}

#[test]
fn outer_wall_path_carries_explicit_closing_repeat() {
    let module = ClassicPerimeters::on_print_start(&config_one_outer_wall()).unwrap();
    let region = make_square_region(10.0);
    let mut output = PerimeterOutputBuilder::new();
    let paint = PaintRegionLayerView::new(0);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config_one_outer_wall())
        .unwrap();

    let walls = output.wall_loops();
    assert!(!walls.is_empty(), "expected at least one wall loop");
    let outer = walls
        .iter()
        .find(|w| w.loop_type == LoopType::Outer)
        .expect("classic-perimeters must emit at least one outer wall");

    // A4-vertex square is now emitted as 5 points with last == first.
    assert!(
        outer.path.is_closed(),
        "outer wall path must be closed (ExtrusionPath3D::is_closed): \
         first={:?} last={:?}",
        outer.path.points.first().map(|p| (p.x, p.y)),
        outer.path.points.last().map(|p| (p.x, p.y)),
    );
    assert!(
        outer.path.points.len() >= 4,
        "expected >= 4 points (square + closing repeat); got {}",
        outer.path.points.len()
    );
    let first = outer.path.points.first().unwrap();
    let last = outer.path.points.last().unwrap();
    assert!(
        (first.x - last.x).abs() < f32::EPSILON && (first.y - last.y).abs() < f32::EPSILON,
        "closing-repeat XY must equal first-vertex XY: first={:?}, last={:?}",
        (first.x, first.y),
        (last.x, last.y),
    );
}

#[test]
fn outer_wall_parallel_arrays_match_points_length() {
    let module = ClassicPerimeters::on_print_start(&config_one_outer_wall()).unwrap();
    let region = make_square_region(10.0);
    let mut output = PerimeterOutputBuilder::new();
    let paint = PaintRegionLayerView::new(0);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config_one_outer_wall())
        .unwrap();

    let outer = output
        .wall_loops()
        .iter()
        .find(|w| w.loop_type == LoopType::Outer)
        .expect("at least one outer wall")
        .clone();

    assert_eq!(
        outer.path.points.len(),
        outer.feature_flags.len(),
        "feature_flags must be parallel to path.points (including closing repeat)"
    );
    assert_eq!(
        outer.path.points.len(),
        outer.width_profile.widths.len(),
        "width_profile.widths must be parallel to path.points (including closing repeat)"
    );
}

#[test]
fn closing_repeat_feature_flag_mirrors_first_vertex() {
    // The closing-repeat vertex is geometrically identical to the first
    // vertex and must carry the same paint/feature flag so segment-iterating
    // post-processors (fuzzy-skin, seam-placer) see consistent flags.
    let module = ClassicPerimeters::on_print_start(&config_one_outer_wall()).unwrap();
    let region = make_square_region(10.0);
    let mut output = PerimeterOutputBuilder::new();
    let paint = PaintRegionLayerView::new(0);

    module
        .run_perimeters(0, &[region], &paint, &mut output, &config_one_outer_wall())
        .unwrap();

    let outer = output
        .wall_loops()
        .iter()
        .find(|w| w.loop_type == LoopType::Outer)
        .expect("at least one outer wall")
        .clone();

    let first_flag = outer.feature_flags.first().expect("at least one flag");
    let last_flag = outer.feature_flags.last().expect("at least one flag");
    assert_eq!(
        first_flag, last_flag,
        "closing-repeat flag must mirror first-vertex flag"
    );

    let first_width = outer.width_profile.widths.first().copied().unwrap();
    let last_width = outer.width_profile.widths.last().copied().unwrap();
    assert!(
        (first_width - last_width).abs() < f32::EPSILON,
        "closing-repeat width must mirror first-vertex width"
    );
}
