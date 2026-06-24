//! AC-6: per-color outer-wall fragmentation (Model A / OrcaSlicer MMU parity, packet 105).
//!
//! Each per-color `SlicedRegion` must trace its own independent outer wall.
//! Before this change arachne grouped regions by object and traced the shared
//! `external_contour` once (union-trace), producing a monochrome outer wall
//! for painted objects. After the fix every region emits `emit_outer=true`,
//! so N per-color cells → N distinct outer-wall loops.
//!
//! Tests use classic-perimeters (deterministic, both modules are Model A equivalent
//! post-fix; classic is stable for these fixture sizes).

use classic_perimeters::ClassicPerimeters;
use slicer_ir::LoopType;
use slicer_sdk::builders::PerimeterOutputBuilder;
use slicer_sdk::test_prelude::*;
use slicer_sdk::traits::{LayerModule, PaintRegionLayerView};
use slicer_sdk::views::SliceRegionView;

/// Build a config appropriate for fragmentation tests: 1 wall so the
/// outer-wall count is simple to reason about.
fn fragmentation_config() -> slicer_ir::ConfigView {
    ConfigViewBuilder::new()
        .int("wall_count", 1)
        .float("outer_wall_line_width", 0.4_f64)
        .float("inner_wall_line_width", 0.4_f64)
        .build()
}

/// Build the left half-cell: a 5×10 mm rectangle centered at (-2.5, 0).
/// Together with the right half-cell these form a 10×10 mm square split at X=0.
fn left_cell_region(z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-mmu")
        .region_id(1)
        .z(z)
        .add_polygon(rect_polygon(-2.5, 0.0, 5.0, 10.0))
        .build()
}

/// Build the right half-cell: a 5×10 mm rectangle centered at (2.5, 0).
fn right_cell_region(z: f32) -> SliceRegionView {
    SliceRegionViewBuilder::new()
        .object_id("obj-mmu")
        .region_id(2)
        .z(z)
        .add_polygon(rect_polygon(2.5, 0.0, 5.0, 10.0))
        .build()
}

/// AC-6 — fragmentation: two adjacent per-color regions of the same object
/// must each produce their own outer wall loop (total 2 outer loops, not 1).
///
/// This is the OrcaSlicer Model A contract: near a bisector both colors trace
/// their own wall from opposite sides, ~one line-width apart. If the union-trace
/// were still present we would see only 1 outer loop across both regions.
#[test]
fn per_color_regions_each_trace_own_outer_wall() {
    let config = fragmentation_config();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    let regions = vec![left_cell_region(0.2), right_cell_region(0.2)];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();

    // Count outer loops: wall_count=1 so every loop is outer (perimeter_index==0).
    let outer_loops: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer)
        .collect();

    assert_eq!(
        outer_loops.len(),
        2,
        "Expected 2 outer-wall loops (one per color cell), got {}. \
         If this is 1 the union-trace regression has re-appeared.",
        outer_loops.len()
    );

    // Each outer loop must be a closed loop around its own cell.
    // The left cell's outer wall must have all X ≤ 0 mm (right side of left rect
    // is at X=0, inset by half outer width → a little negative, so max_x < 0.05).
    // The right cell's outer wall must have all X ≥ 0 mm (left side of right rect
    // is at X=0, inset → a little positive, so min_x > -0.05).
    // We identify the cells by their centroid X.
    let left_loop = outer_loops
        .iter()
        .min_by(|a, b| {
            let ax: f32 =
                a.path.points.iter().map(|p| p.x).sum::<f32>() / a.path.points.len() as f32;
            let bx: f32 =
                b.path.points.iter().map(|p| p.x).sum::<f32>() / b.path.points.len() as f32;
            ax.partial_cmp(&bx).unwrap()
        })
        .unwrap();
    let right_loop = outer_loops
        .iter()
        .max_by(|a, b| {
            let ax: f32 =
                a.path.points.iter().map(|p| p.x).sum::<f32>() / a.path.points.len() as f32;
            let bx: f32 =
                b.path.points.iter().map(|p| p.x).sum::<f32>() / b.path.points.len() as f32;
            ax.partial_cmp(&bx).unwrap()
        })
        .unwrap();

    let left_max_x = left_loop
        .path
        .points
        .iter()
        .map(|p| p.x)
        .fold(f32::MIN, f32::max);
    let right_min_x = right_loop
        .path
        .points
        .iter()
        .map(|p| p.x)
        .fold(f32::MAX, f32::min);

    // Left cell outer-wall right edge must be at or to the left of X=0.
    // (Inset of 0.2 mm from the bisector means max_x ≈ -0.2 mm)
    assert!(
        left_max_x <= 0.05,
        "Left-cell outer wall right edge X={} should be ≤ 0.05 mm (i.e. inside left half)",
        left_max_x
    );

    // Right cell outer-wall left edge must be at or to the right of X=0.
    assert!(
        right_min_x >= -0.05,
        "Right-cell outer wall left edge X={} should be ≥ -0.05 mm (i.e. inside right half)",
        right_min_x
    );
}

/// Model-A single-color guard: a single unpainted region must produce exactly
/// one outer-wall loop. Fragmentation logic must not over-emit.
///
/// Replaces the dropped AC-N3 guard.
#[test]
fn single_color_region_traces_one_outer_wall() {
    let config = fragmentation_config();
    let module = ClassicPerimeters::on_print_start(&config).unwrap();

    // Single 10×10 mm square, one region, one object.
    let regions = vec![SliceRegionViewBuilder::new()
        .object_id("obj-single")
        .region_id(1)
        .z(0.2)
        .add_polygon(square_polygon(0.0, 0.0, 10.0))
        .build()];
    let paint = PaintRegionLayerView::new(0);
    let mut output = PerimeterOutputBuilder::new();

    module
        .run_perimeters(0, &regions, &paint, &mut output, &config)
        .unwrap();

    let walls = output.wall_loops();
    let outer_loops: Vec<_> = walls
        .iter()
        .filter(|w| w.loop_type == LoopType::Outer)
        .collect();

    assert_eq!(
        outer_loops.len(),
        1,
        "Single-color region must produce exactly 1 outer-wall loop, got {}",
        outer_loops.len()
    );
}
